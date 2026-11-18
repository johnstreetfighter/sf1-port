"""Linux release archive, ELF baseline and dependency notice checks."""

import base64
import io
import json
from pathlib import Path
import runpy
import struct
import tarfile
import tempfile
import unittest

import release_notices
from release_packs import ART_PACK, MUSIC_PACK, VOICE_PACK

HERE = Path(__file__).parent
PACKAGE = runpy.run_path(str(HERE / "package-linux.py"))
VERIFY = runpy.run_path(str(HERE / "verify-linux-package.py"))
ROOT = "sf1-port-v9.9.9-linux-x86_64"
PACKS = {f"{ART_PACK}/ryu/stance.png": b"pose", f"{VOICE_PACK}/super.vox": b"\0\x10",
         f"{MUSIC_PACK}/ryu.score": b"title Test tune\n"}
LOADER = "/lib64/ld-linux-x86-64.so.2"


def elf(needed=("libc.so.6", "libasound.so.2"), versions=("GLIBC_2.34", "GLIBC_2.2.5", "GLIBCXX_3.4.30"),
        machine=62, interpreter=LOADER, runpath=None) -> bytes:
    """A minimal x86-64 ELF image with a loader, a dynamic section and its strings."""
    strings = b"\0"
    offsets = {}
    for name in (*needed, *versions, *([runpath] if runpath else [])):
        offsets[name] = len(strings)
        strings += name.encode() + b"\0"
    entries = [(1, offsets[name]) for name in needed]
    if runpath:
        entries.append((29, offsets[runpath]))
    headers = 64 + 3 * 56
    interp = interpreter.encode() + b"\0"
    interp_at = headers
    strings_at = interp_at + len(interp)
    dynamic_at = strings_at + len(strings)
    entries += [(5, strings_at), (0, 0)]
    dynamic = b"".join(struct.pack("<qQ", tag, value) for tag, value in entries)
    size = dynamic_at + len(dynamic)
    header = b"\x7fELF" + bytes([2, 1, 1, 0]) + bytes(8) + struct.pack(
        "<HHIQQQIHHHHHH", 3, machine, 1, 0, 64, 0, 0, 64, 56, 3, 64, 0, 0)

    def segment(kind: int, offset: int, length: int) -> bytes:
        return struct.pack("<IIQQQQQQ", kind, 4, offset, offset, offset, length, length, 1)

    return header + segment(1, 0, size) + segment(3, interp_at, len(interp)) + \
        segment(2, dynamic_at, len(dynamic)) + interp + strings + dynamic


def staged(folder: Path, extra: dict[str, bytes] | None = None) -> Path:
    package = folder / ROOT
    files = {name: b"baseline" for name in VERIFY["RELEASE_FILES"]}
    files["licenses/rust-dependencies/example-1.0.0/LICENSE"] = b"example license"
    files.update(PACKS)
    files.update(extra or {})
    for name, data in files.items():
        path = package / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
    return package


def archive_of(package: Path) -> bytes:
    archive = package.parent / f"{package.name}.tar.gz"
    PACKAGE["write_archive"](package, archive)
    data = archive.read_bytes()
    archive.unlink()
    return data


def check(raw: bytes) -> tuple[bytes, bytes, int]:
    return VERIFY["archive_contents"](raw, ROOT, {}, {}, set(), PACKS)


class LinuxArchiveTests(unittest.TestCase):
    def test_the_archive_holds_the_package_with_a_runnable_launcher_and_no_local_owner(self):
        with tempfile.TemporaryDirectory() as tmp:
            raw = archive_of(staged(Path(tmp)))
        game, build, count = check(raw)
        self.assertEqual((game, build), (b"baseline", b"baseline"))
        self.assertEqual(count, len(VERIFY["RELEASE_FILES"]) + len(PACKS) + 1)
        with tarfile.open(fileobj=io.BytesIO(raw), mode="r:gz") as package:
            members = {member.name: member for member in package.getmembers()}
        self.assertEqual(members[f"{ROOT}/sf-game"].mode, 0o755)
        self.assertEqual(members[f"{ROOT}/README.md"].mode, 0o644)
        self.assertEqual({(m.uid, m.gid, m.uname, m.gname) for m in members.values()}, {(0, 0, "root", "root")})

    def test_the_same_package_always_makes_the_same_archive(self):
        with tempfile.TemporaryDirectory() as tmp:
            package = staged(Path(tmp))
            self.assertEqual(archive_of(package), archive_of(package))

    def test_the_verifier_rejects_local_data_extra_assets_and_missing_files(self):
        for path, data in (
            ("roms/sf/capcom.rom", b"original ROM"),
            ("settings.toml", b"scale = 2\n"),
            ("sf-game.log", b"log"),
            (f"{MUSIC_PACK}/other.score", b"title Extra\n"),
            (f"{ART_PACK}/candidates/raw.png", b"pose"),
            ("notes.txt", b"stray"),
        ):
            with self.subTest(path=path), tempfile.TemporaryDirectory() as tmp:
                raw = archive_of(staged(Path(tmp), {path: data}))
                with self.assertRaises(ValueError):
                    check(raw)
        with tempfile.TemporaryDirectory() as tmp:
            package = staged(Path(tmp))
            (package / VOICE_PACK / "super.vox").unlink()
            with self.assertRaisesRegex(ValueError, "missing expected package files"):
                check(archive_of(package))

    def test_the_verifier_rejects_changed_pack_bytes_links_and_local_owners(self):
        with tempfile.TemporaryDirectory() as tmp:
            raw = archive_of(staged(Path(tmp), {f"{VOICE_PACK}/super.vox": b"\x10\0"}))
        with self.assertRaisesRegex(ValueError, "tagged pack file differs"):
            check(raw)

        def rewritten(change) -> bytes:
            with tempfile.TemporaryDirectory() as tmp:
                raw = archive_of(staged(Path(tmp)))
            out = io.BytesIO()
            with tarfile.open(fileobj=io.BytesIO(raw), mode="r:gz") as source, \
                    tarfile.open(fileobj=out, mode="w:gz") as target:
                for member in source.getmembers():
                    data = source.extractfile(member).read() if member.isreg() else None
                    change(member)
                    target.addfile(member, io.BytesIO(data) if data is not None else None)
            return out.getvalue()

        def link(member):
            if member.name.endswith("/README.md") and member.name.count("/") == 1:
                member.type, member.linkname, member.size = tarfile.SYMTYPE, "/etc/passwd", 0

        def owner(member):
            member.uid, member.uname = 1000, "builder"

        def not_runnable(member):
            if member.name == f"{ROOT}/sf-game":
                member.mode = 0o644

        for change, message in ((owner, "local owner"), (not_runnable, "unexpected file mode")):
            with self.subTest(change=change.__name__):
                raw = rewritten(change)
                # Rewriting stores a gzip time; this check is about members.
                raw = raw[:4] + bytes(4) + raw[8:]
                with self.assertRaisesRegex(ValueError, message):
                    check(raw)

        with tempfile.TemporaryDirectory() as tmp:
            raw = archive_of(staged(Path(tmp)))
        out = io.BytesIO()
        with tarfile.open(fileobj=io.BytesIO(raw), mode="r:gz") as source, tarfile.open(fileobj=out, mode="w:gz") as target:
            for member in source.getmembers():
                data = source.extractfile(member).read() if member.isreg() else None
                link(member)
                target.addfile(member, io.BytesIO(data) if member.isreg() else None)
        raw = out.getvalue()
        with self.assertRaisesRegex(ValueError, "unsafe or unexpected archive member"):
            check(raw[:4] + bytes(4) + raw[8:])


class ElfBaselineTests(unittest.TestCase):
    def test_a_baseline_executable_reports_its_libraries_and_newest_symbols(self):
        report = VERIFY["check_elf"](elf())
        self.assertEqual(report["needed"], ["libc.so.6", "libasound.so.2"])
        self.assertEqual(report["glibc"], (2, 34))
        self.assertEqual(report["glibcxx"], (3, 4, 30))
        self.assertEqual(report["interpreter"], LOADER)

    def test_an_executable_outside_the_baseline_is_rejected(self):
        for image, message in (
            (elf(versions=("GLIBC_2.38",)), "needs glibc 2.38"),
            (elf(versions=("GLIBC_2.35", "GLIBCXX_3.4.32")), "needs libstdc\\+\\+ 3.4.32"),
            (elf(needed=("libc.so.6", "libSDL2-2.0.so.0")), "libSDL2-2.0.so.0"),
            (elf(machine=183), "not x86-64"),
            (elf(interpreter="/nix/store/ld-linux-x86-64.so.2"), "standard x86-64 loader"),
            (elf(runpath="/build/lib"), "library search path"),
            (b"MZ" + bytes(200), "ELF header"),
        ):
            with self.subTest(message=message), self.assertRaisesRegex(ValueError, message):
                VERIFY["check_elf"](image)

    def test_glibc_2_35_itself_is_inside_the_baseline(self):
        self.assertEqual(VERIFY["check_elf"](elf(versions=("GLIBC_2.35",)))["glibc"], (2, 35))


class NoticeTests(unittest.TestCase):
    def crate(self, root: Path, name: str, files: dict[str, bytes], **fields) -> dict:
        folder = root / "registry" / f"{name}-1.0.0"
        for path, data in ({"Cargo.toml": b"[package]\n"} | files).items():
            (folder / path).parent.mkdir(parents=True, exist_ok=True)
            (folder / path).write_bytes(data)
        return {"name": name, "version": "1.0.0", "source": "registry+https://github.com/rust-lang/crates.io-index",
                "manifest_path": str(folder / "Cargo.toml"), "license": "MIT", "license_file": None,
                "repository": f"https://github.com/example/{name}"} | fields

    def test_notices_come_from_the_crate_or_its_pinned_upstream_revision(self):
        revision = "a" * 40
        calls = []

        def api(path: str):
            calls.append(path)
            if path == f"repos/example/bare/contents?ref={revision}":
                return [{"name": "LICENSE-MIT", "path": "LICENSE-MIT", "type": "file"},
                        {"name": "src", "path": "src", "type": "dir"}]
            if path == f"repos/example/bare/contents/LICENSE-MIT?ref={revision}":
                return {"content": base64.b64encode(b"upstream MIT text").decode()}
            raise AssertionError(path)

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            packages = [
                self.crate(root, "own", {"LICENSE": b"own text", "LICENSES/extra.txt": b"extra", "src/lib.rs": b""}),
                self.crate(root, "lower", {"license": b"lowercase name"}),
                self.crate(root, "bare", {".cargo_vcs_info.json": json.dumps({"git": {"sha1": revision}}).encode()}),
                {"name": "sf-game", "version": "9.9.9", "source": None, "manifest_path": str(root / "Cargo.toml")},
            ]
            package = root / "package"
            count = release_notices.collect(packages, package, release_notices.Upstream(api=api))
            licenses = package / "licenses" / "rust-dependencies"
            written = {p.relative_to(licenses).as_posix(): p.read_bytes() for p in licenses.rglob("*") if p.is_file()}
            index = (package / "licenses" / "README.md").read_text(encoding="utf-8")
        self.assertEqual(count, 3)
        self.assertEqual(written, {
            "own-1.0.0/LICENSE": b"own text",
            "lower-1.0.0/license": b"lowercase name",
            "own-1.0.0/LICENSES/extra.txt": b"extra",
            "bare-1.0.0/LICENSE-MIT": b"upstream MIT text",
            "bare-1.0.0/UPSTREAM-SOURCE.txt": f"https://github.com/example/bare (commit {revision})".encode(),
        })
        self.assertIn("- bare-1.0.0; license: MIT; source: https://crates.io/crates/bare/1.0.0", index)
        self.assertNotIn("sf-game-9.9.9", index)
        self.assertEqual(len(calls), 2)

    def test_a_crate_with_no_license_text_anywhere_stops_the_package(self):
        revision = "b" * 40
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            unpinned = self.crate(root, "unpinned", {})
            empty = self.crate(root, "empty", {".cargo_vcs_info.json": json.dumps({"git": {"sha1": revision}}).encode()})
            with self.assertRaisesRegex(ValueError, "No license text or pinned upstream revision for unpinned"):
                release_notices.collect([unpinned], root / "a", release_notices.Upstream(api=lambda path: []))
            with self.assertRaisesRegex(ValueError, "no root license files for empty"):
                release_notices.collect([empty], root / "b", release_notices.Upstream(api=lambda path: []))


if __name__ == "__main__":
    unittest.main()
