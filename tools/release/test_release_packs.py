"""Release pack allowlists and ZIP validation."""

import io
import json
from pathlib import Path
import runpy
import struct
import tempfile
import unittest
from zipfile import ZipFile
import zlib

from release_packs import ART_PACK, VOICE_PACK, git, manifest_entries, stage, tagged_pack_files


VERIFY = runpy.run_path(str(Path(__file__).with_name("verify-windows-package.py")))
ROOT = "sf1-port-test-windows-x86_64"
POSE = f"{ART_PACK}/ryu/stance.png"
LINE = f"{VOICE_PACK}/super.vox"


def indexed_png() -> bytes:
    def chunk(kind: bytes, data: bytes) -> bytes:
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))

    header = struct.pack(">IIBBBBB", 1, 1, 8, 3, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"PLTE", b"\0\0\0") + \
        chunk(b"IDAT", zlib.compress(b"\0\0")) + chunk(b"IEND", b"")


def art_manifest(name: str, pose: bytes) -> bytes:
    return json.dumps({"format": 1, "files": {name: {
        "size": len(pose), "crc32": f"{zlib.crc32(pose):08x}",
    }}}).encode()


def zip_bytes(files: dict[str, bytes]) -> bytes:
    stream = io.BytesIO()
    with ZipFile(stream, "w") as archive:
        for name, data in files.items():
            archive.writestr(f"{ROOT}/{name}", data)
    return stream.getvalue()


class ReleasePacksTests(unittest.TestCase):
    def test_staging_copies_only_manifest_files_and_readmes_from_the_commit(self):
        pose = indexed_png()
        line = b"\0\x10"
        voice = {"format": 1, "sample_rate": 4000, "lines": {
            "super": {"nibbles": 3, "crc32": f"{zlib.crc32(line):08x}"},
        }}
        expected = {
            POSE: pose, f"{ART_PACK}/manifest.json": art_manifest("ryu/stance.png", pose),
            LINE: line, f"{VOICE_PACK}/manifest.json": json.dumps(voice).encode(),
            f"{ART_PACK}/README.md": b"art provenance", f"{VOICE_PACK}/README.md": b"voice provenance",
        }
        excluded = {
            f"{ART_PACK}/candidates/raw.png": pose,
            f"{VOICE_PACK}/archive/raw.vox": line,
        }
        with tempfile.TemporaryDirectory() as tmp:
            source = Path(tmp) / "source"
            source.mkdir()
            git(source, "init", "-q")
            for name, data in (expected | excluded).items():
                path = source / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(data)
            git(source, "add", ".")
            git(source, "-c", "user.name=Release Test", "-c", "user.email=release-test@example.com",
                "commit", "-qm", "Pack fixture")
            commit = git(source, "rev-parse", "HEAD").decode().strip()
            (source / LINE).write_bytes(b"dirty working copy")
            target = Path(tmp) / "package"
            stage(source, commit, target)
            staged = {p.relative_to(target).as_posix(): p.read_bytes() for p in target.rglob("*") if p.is_file()}
            self.assertEqual(staged, expected)

            git(source, "add", LINE)
            git(source, "-c", "user.name=Release Test", "-c", "user.email=release-test@example.com",
                "commit", "-qm", "Break the manifest")
            with self.assertRaisesRegex(ValueError, "differs from its manifest"):
                tagged_pack_files(source, "HEAD")

    def test_manifest_rejects_authoring_or_unsafe_paths(self):
        for name in ("candidates/raw.png", "archive/raw.png", "../pose.png", "ryu/CON.png"):
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, "invalid playable asset path"):
                manifest_entries(ART_PACK, art_manifest(name, indexed_png()))

    def test_zip_verifier_checks_exact_pack_bytes_and_rejects_extra_assets(self):
        packs = {POSE: indexed_png(), LINE: b"\0\x10"}
        files = {name: b"baseline" for name in VERIFY["RELEASE_FILES"]}
        files["licenses/rust-dependencies/example-1.0.0/LICENSE"] = b"example license"
        files.update(packs)

        def verify(files: dict[str, bytes]) -> None:
            VERIFY["archive_contents"](zip_bytes(files), ROOT, {}, {}, set(), packs)

        verify(files)
        for path, data in (
            (LINE, b"\x10\0"),
            (f"{ART_PACK}/candidates/raw.png", indexed_png()),
            (f"{VOICE_PACK}/archive/raw.vox", b"\0\x10"),
            ("roms/sf/capcom.rom", b"original ROM"),
            ("assets/sf1-port-music/example.wav", b"local music"),
        ):
            with self.subTest(path=path), self.assertRaises(ValueError):
                verify(files | {path: data})
        with self.assertRaisesRegex(ValueError, "missing expected package files"):
            verify({path: data for path, data in files.items() if path != LINE})


if __name__ == "__main__":
    unittest.main()
