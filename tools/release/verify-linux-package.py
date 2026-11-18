"""Verify a Linux release tar.gz against its annotated tag and release build."""

from __future__ import annotations

import argparse
import hashlib
import io
import json
from pathlib import Path, PurePosixPath
import re
import runpy
import struct
import subprocess
import sys
import tarfile
import tempfile

from release_packs import RESERVED_NAME, git, require, tagged_pack_files

# The path scan, private names and tagged-file list are the Windows verifier's.
SHARED = runpy.run_path(str(Path(__file__).with_name("verify-windows-package.py")))
tagged_files = SHARED["tagged_files"]
private_prefixes = SHARED["private_prefixes"]
scan_paths = SHARED["scan_paths"]
PRIVATE_COMPONENTS = SHARED["PRIVATE_COMPONENTS"]
PACK_COMPONENTS = SHARED["PACK_COMPONENTS"]
PRIVATE_SUFFIXES = SHARED["PRIVATE_SUFFIXES"]

EXECUTABLE = "sf-game"
TARGET = "x86_64-unknown-linux-gnu"
RELEASE_FILES = {"BUILD.txt", "Cargo.lock", "QUICKSTART.txt", "README.md",
                 "THIRD_PARTY_NOTICES.md", EXECUTABLE, "licenses/README.md"}
# The runtime baseline: Ubuntu 22.04.
MAX_GLIBC = (2, 35)
MAX_GLIBCXX = (3, 4, 30)
INTERPRETER = "/lib64/ld-linux-x86-64.so.2"
# Libraries every supported desktop and SteamOS provide without extra
# packages. The window system, Vulkan and gamepad libraries are loaded at run
# time and are not listed in the executable.
ALLOWED_LIBRARIES = {
    "libc.so.6", "libm.so.6", "libdl.so.2", "libpthread.so.0", "librt.so.1", "ld-linux-x86-64.so.2",
    "libgcc_s.so.1", "libstdc++.so.6", "libasound.so.2", "libudev.so.1",
}


def elf_report(data: bytes) -> dict:
    """Architecture, interpreter, needed libraries and symbol-version ceilings of a 64-bit ELF."""
    require(data[:4] == b"\x7fELF", "executable lacks the ELF header")
    require(data[4] == 2 and data[5] == 1, "executable is not 64-bit little-endian ELF")
    kind, machine = struct.unpack_from("<HH", data, 16)
    require(machine == 62, "executable is not x86-64")
    require(kind in (2, 3), "ELF file is not an executable")
    phoff, = struct.unpack_from("<Q", data, 32)
    phentsize, phnum = struct.unpack_from("<HH", data, 54)
    segments = []
    for index in range(phnum):
        p_type, _, offset, vaddr, _, filesz, _, _ = struct.unpack_from("<IIQQQQQQ", data, phoff + index * phentsize)
        segments.append((p_type, offset, vaddr, filesz))

    def locate(address: int) -> int:
        for p_type, offset, vaddr, filesz in segments:
            if p_type == 1 and vaddr <= address < vaddr + filesz:
                return offset + (address - vaddr)
        raise ValueError("ELF address is outside file-backed segments")

    def text(offset: int) -> str:
        end = data.index(b"\0", offset)
        return data[offset:end].decode("utf-8")

    interpreter = next((text(offset) for p_type, offset, _, _ in segments if p_type == 3), None)
    dynamic = next(((offset, filesz) for p_type, offset, _, filesz in segments if p_type == 2), None)
    require(dynamic is not None, "ELF executable has no dynamic section")
    entries = []
    for offset in range(dynamic[0], dynamic[0] + dynamic[1], 16):
        tag, value = struct.unpack_from("<qQ", data, offset)
        if tag == 0:
            break
        entries.append((tag, value))
    strtab = next((value for tag, value in entries if tag == 5), None)
    require(strtab is not None, "ELF executable has no dynamic string table")
    strings = locate(strtab)
    needed = [text(strings + value) for tag, value in entries if tag == 1]
    search_paths = [text(strings + value) for tag, value in entries if tag in (15, 29)]

    def ceiling(pattern: bytes) -> tuple[int, ...]:
        found = {tuple(int(part) for part in match.group(1).split(b"."))
                 for match in re.finditer(pattern + rb"_([0-9]+(?:\.[0-9]+)+)\0", data)}
        return max(found, default=())

    return {"interpreter": interpreter, "needed": needed, "search_paths": search_paths,
            "glibc": ceiling(rb"GLIBC"), "glibcxx": ceiling(rb"GLIBCXX")}


def check_elf(data: bytes) -> dict:
    report = elf_report(data)
    require(report["interpreter"] == INTERPRETER, "executable does not use the standard x86-64 loader")
    require(not report["search_paths"], "executable carries a library search path")
    unexpected = sorted(set(report["needed"]) - ALLOWED_LIBRARIES)
    require(not unexpected, "executable needs libraries outside the baseline: " + ", ".join(unexpected))
    require(bool(report["glibc"]), "executable names no glibc symbol version")
    require(report["glibc"] <= MAX_GLIBC,
            f"executable needs glibc {'.'.join(map(str, report['glibc']))}, newer than the baseline")
    require(report["glibcxx"] <= MAX_GLIBCXX,
            f"executable needs libstdc++ {'.'.join(map(str, report['glibcxx']))}, newer than the baseline")
    return report


def member_path(member: tarfile.TarInfo, root: str) -> str:
    name = member.name
    parts = name.split("/")
    require((member.isreg() or member.isdir()) and parts[0] == root and not name.startswith("/")
            and all(part and part not in (".", "..") and not part.endswith((".", " "))
                    and not RESERVED_NAME.match(part)
                    and not re.search(r'[\\:<>"|?*\x00-\x1f\x7f]', part) for part in parts),
            "unsafe or unexpected archive member")
    require(member.uid == 0 and member.gid == 0 and member.uname == "root" and member.gname == "root",
            "archive member records a local owner")
    return "/".join(parts[1:])


def archive_contents(raw: bytes, root: str, tracked: dict[str, str], roms: dict[int, set[str]],
                     prefixes: set[bytes], packs: dict[str, bytes]) -> tuple[bytes, bytes, int]:
    expected = RELEASE_FILES | set(tracked) | set(packs)
    dirs = {str(parent) for name in expected for parent in PurePosixPath(name).parents}
    dirs.discard(".")
    pack_dirs = {str(parent) for name in packs for parent in PurePosixPath(name).parents}
    seen = set()
    files = set()
    notices = 0
    game = build = b""
    # No stored file name, comment or extra field, and no build time.
    require(raw[:4] == b"\x1f\x8b\x08\x00" and raw[4:8] == b"\0\0\0\0", "archive is not a plain gzip stream")
    with tarfile.open(fileobj=io.BytesIO(raw), mode="r:gz") as package:
        members = package.getmembers()
        require(bool(members), "archive is empty")
        for member in members:
            path = member_path(member, root)
            key = path.casefold()
            require(key not in seen, "duplicate archive member")
            seen.add(key)
            suffix = Path(path).suffix.casefold()
            dependency = path.startswith("licenses/rust-dependencies/")
            components = key.split("/")
            pack_member = path in packs or (member.isdir() and path in pack_dirs)
            require(not PRIVATE_COMPONENTS.intersection(components)
                    and (pack_member or not PACK_COMPONENTS.intersection(components))
                    and (suffix not in PRIVATE_SUFFIXES or (path in packs and suffix == ".vox")),
                    "archive contains local data or an unexpected binary")
            scan_paths(member.name.encode() + b"\0" + json.dumps(member.pax_headers).encode(),
                       prefixes, "archive member metadata")
            if member.isdir():
                require(not path or path in dirs or path in ("licenses", "licenses/rust-dependencies")
                        or dependency, "unexpected archive directory")
                require(member.mode == 0o755, "unexpected directory mode")
                continue
            require(path in expected or dependency, "unexpected archive file")
            require(member.mode == (0o755 if path == EXECUTABLE else 0o644), f"unexpected file mode: {path}")
            data = package.extractfile(member).read()
            require(data or path not in RELEASE_FILES, f"archive contains an empty {path}")
            require(len(data) not in roms or hashlib.sha1(data).hexdigest() not in roms[len(data)],
                    "archive contains an original game ROM")
            scan_paths(data, prefixes, f"archive member {path}")
            files.add(path)
            notices += dependency
            if path in packs:
                require(data == packs[path], f"tagged pack file differs: {path}")
            elif path.startswith("shaders/") or path == "docs/netplay.md":
                blob = b"blob " + str(len(data)).encode("ascii") + b"\0" + data
                require(hashlib.sha1(blob).hexdigest() == tracked[path], f"tagged file differs: {path}")
            elif path == EXECUTABLE:
                game = data
            elif path == "BUILD.txt":
                build = data
    require(expected <= files, "archive is missing expected package files")
    require(notices > 0, "archive is missing dependency notices")
    return game, build, len(files)


def run_extracted(raw: bytes, root: str) -> str:
    """Extract the whole archive and run the executable from another working folder."""
    with tempfile.TemporaryDirectory(prefix="sf1-port-verify-") as tmp:
        with tarfile.open(fileobj=io.BytesIO(raw), mode="r:gz") as package:
            package.extractall(tmp, filter="data")
        game = Path(tmp) / root / EXECUTABLE
        elsewhere = Path(tmp) / "elsewhere"
        elsewhere.mkdir()
        return subprocess.check_output([str(game), "--version"], cwd=elsewhere,
                                       text=True, encoding="utf-8", timeout=30).strip()


def verify(archive: Path, exe: Path, source: Path, tag: str, commit: str, extra: list[Path]) -> None:
    require(re.fullmatch(r"v[0-9]+\.[0-9]+\.[0-9]+", tag) is not None, "expected a vX.Y.Z release tag")
    require(re.fullmatch(r"[0-9a-f]{40}", commit) is not None, "expected the full Git commit SHA")
    root = f"sf1-port-{tag}-linux-x86_64"
    require(archive.name == f"{root}.tar.gz", "unexpected archive basename")
    sidecar = archive.with_name(archive.name + ".sha256")
    archive_bytes = archive.read_bytes()
    archive_digest = hashlib.sha256(archive_bytes).hexdigest()
    require(sidecar.read_bytes() == f"{archive_digest}  {archive.name}\n".encode("ascii"),
            "SHA-256 sidecar does not match the archive")
    tracked = tagged_files(source, tag, commit)
    packs = tagged_pack_files(source, commit)
    roms: dict[int, set[str]] = {}
    manifest = json.loads(git(source, "show", f"{commit}:tools/reference/rom-manifest.json"))
    for row in manifest["entries"]:
        roms.setdefault(row["size"], set()).add(row["sha1"])
    prefixes = private_prefixes(source, exe, extra)
    game, build, count = archive_contents(archive_bytes, root, tracked, roms, prefixes, packs)
    game_digest = hashlib.sha256(game).hexdigest()
    require(hashlib.sha256(exe.read_bytes()).hexdigest() == game_digest,
            "packaged executable differs from the release build")
    report = check_elf(game)
    banner = run_extracted(archive_bytes, root)
    require(banner.endswith(f" {tag} ({commit[:8]})"), "extracted executable version disagrees with the tag")
    expected_build = {f"Tag: {tag}", f"Source commit: {commit}", f"Target: {TARGET}",
                      f"Executable version: {banner}", f"{EXECUTABLE} SHA-256: {game_digest}"}
    require(expected_build <= set(build.decode("utf-8").splitlines()), "BUILD.txt disagrees with verified package")
    print(f"Version: {banner}")
    print(f"Archive SHA-256: {archive_digest}")
    print(f"Executable SHA-256: {game_digest}")
    print(f"Verified {count} archive files and {sum(p.startswith('shaders/') for p in tracked)} tagged shaders.")
    print(f"Verified {len(packs)} tagged art, voice, and music pack files.")
    print("x86-64 ELF needs: " + ", ".join(report["needed"]))
    print("Newest glibc symbol: " + ".".join(map(str, report["glibc"]))
          + "; newest libstdc++ symbol: " + (".".join(map(str, report["glibcxx"])) or "none"))
    print("Workstation path scan passed.")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archive", type=Path)
    parser.add_argument("exe", type=Path)
    parser.add_argument("source", type=Path)
    parser.add_argument("tag")
    parser.add_argument("commit")
    parser.add_argument("--private-root", action="append", type=Path, default=[],
                        help="additional absolute paths that must not be embedded")
    args = parser.parse_args()
    if any(not root.is_absolute() for root in args.private_root):
        parser.error("--private-root must be an absolute path")
    try:
        verify(args.archive.resolve(), args.exe.resolve(), args.source.resolve(),
               args.tag, args.commit, args.private_root)
    except (OSError, ValueError, RuntimeError, struct.error, subprocess.SubprocessError,
            tarfile.TarError) as error:
        print(f"Package verification failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
