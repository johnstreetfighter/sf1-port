"""Verify a Windows release ZIP against its annotated tag and release build."""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import struct
import subprocess
import sys
import tempfile
import zipfile


from release_packs import RESERVED_NAME, git, require, tagged_pack_files


RELEASE_FILES = {"BUILD.txt", "Cargo.lock", "QUICKSTART.txt", "README.md",
                 "THIRD_PARTY_NOTICES.md", "sf-game.exe", "licenses/README.md"}
PRIVATE_COMPONENTS = {"roms", "captures", "settings.toml", "bindings.toml", "match.toml"}
PACK_COMPONENTS = {"sf1-port-art", "sf1-port-voice", "sf1-port-music"}
PRIVATE_SUFFIXES = {".7z", ".bin", ".exe", ".log", ".pdb", ".rom", ".vox", ".wav", ".zip"}
PRINTABLE = (
    (re.compile(rb"[\x20-\x7e]{6,}"), "ascii"),
    (re.compile(rb"(?:[\x20-\x7e]\x00){6,}"), "utf-16le"),
    (re.compile(rb"(?:\x00[\x20-\x7e]){6,}"), "utf-16be"),
)
WORKSTATION_PATH = re.compile(
    r"(?i)[a-z]:[\\/]+(?:users|documents and settings|a|_work|actions-runner|workspace|"
    r"program files(?: \(x86\))?|programdata)[\\/]+"
    r"|(?<![a-z0-9_./:])/(?:home|users|root|__w|workspaces)/"
    r"|\\\\[a-z0-9][a-z0-9_.-]+\\[a-z0-9_$.-]{2,}\\"
)
ROOT_VARIABLES = ("USERPROFILE", "HOME", "LOCALAPPDATA", "TEMP", "TMP", "GITHUB_WORKSPACE",
                  "RUNNER_WORKSPACE", "RUNNER_TEMP", "CARGO_HOME", "RUSTUP_HOME")


def tagged_files(source: Path, tag: str, commit: str) -> dict[str, str]:
    ref = f"refs/tags/{tag}"
    require(git(source, "cat-file", "-t", ref).strip() == b"tag", "release tag must be annotated")
    require(git(source, "rev-parse", f"{ref}^{{commit}}").strip().decode() == commit,
            "release tag does not point to the expected commit")
    entries = git(source, "ls-tree", "-rz", commit, "--", "shaders", "docs/netplay.md").split(b"\0")
    files = {}
    for entry in filter(None, entries):
        header, path = entry.split(b"\t", 1)
        mode, kind, digest = header.split()
        require(kind == b"blob" and mode in (b"100644", b"100755"), "tagged package file is not regular")
        files[path.decode("utf-8")] = digest.decode("ascii")
    require(any(path.startswith("shaders/") for path in files), "release tag has no shaders")
    return files


def private_prefixes(source: Path, exe: Path, extra: list[Path]) -> set[bytes]:
    roots = [source, exe.parent, *extra]
    if exe.parent.name.casefold() == "release":
        roots.append(exe.parent.parent)
    roots.extend(Path(value) for name in ROOT_VARIABLES if (value := os.environ.get(name)))
    prefixes = set()
    for root in roots:
        if not root.is_absolute() or root == Path(root.anchor):
            continue
        normalized = str(root).replace("\\", "/").rstrip("/")
        for form in (normalized, normalized.lower()):
            for spelling in (form, form.replace("/", "\\")):
                for encoding in ("utf-8", "utf-16le", "utf-16be"):
                    prefixes.add(spelling.encode(encoding))
    return prefixes


def scan_paths(data: bytes, prefixes: set[bytes], context: str) -> None:
    folded = data.lower()
    require(not any(prefix in data or prefix in folded for prefix in prefixes),
            f"{context} contains a private path")
    for pattern, encoding in PRINTABLE:
        require(not any(WORKSTATION_PATH.search(run[0].decode(encoding)) for run in pattern.finditer(data)),
                f"{context} contains a workstation path")


def member_path(member: zipfile.ZipInfo, root: str) -> str:
    name = member.filename
    parts = name.removesuffix("/").split("/")
    kind = stat.S_IFMT(member.external_attr >> 16) if member.create_system == 3 else 0
    expected_kind = stat.S_IFDIR if member.is_dir() else stat.S_IFREG
    require(name == member.orig_filename and parts[0] == root and kind in (0, expected_kind)
            and all(part and not part.endswith((".", " ")) and not RESERVED_NAME.match(part)
                    and not re.search(r'[\\:<>"|?*\x00-\x1f\x7f]', part) for part in parts),
            "unsafe or unexpected ZIP member")
    return "/".join(parts[1:])


def pe_imports(data: bytes) -> list[str]:
    def read(fmt: str, offset: int):
        return struct.unpack_from("<" + fmt, data, offset)

    require(data.startswith(b"MZ"), "executable lacks the DOS header")
    pe, = read("I", 0x3c)
    require(data[pe:pe + 4] == b"PE\0\0", "executable lacks the PE header")
    machine, count, _, _, _, optional_size, _ = read("HHIIIHH", pe + 4)
    optional = pe + 24
    require(machine == 0x8664 and optional_size >= 128 and read("H", optional)[0] == 0x20b,
            "executable is not Windows x64 PE32+")
    require(read("H", optional + 68)[0] == 2, "executable does not use the Windows GUI subsystem")
    table = optional + optional_size
    directories, = read("I", optional + 108)
    sections = [read("IIII", table + index * 40 + 8) for index in range(count)]

    def directory(index: int) -> tuple[int, int]:
        offset = optional + 112 + index * 8
        if index >= directories:
            return 0, 0
        require(offset + 8 <= table, "invalid PE data directory")
        return read("II", offset)

    def locate(rva: int, size: int) -> int:
        for _, address, raw_size, raw_offset in sections:
            if address <= rva and rva + size <= address + raw_size:
                offset = raw_offset + (rva - address)
                require(offset + size <= len(data), "PE directory extends beyond executable")
                return offset
        raise ValueError("PE directory is outside file-backed sections")

    import_rva, import_size = directory(1)
    require(import_rva != 0 and import_size >= 20, "PE executable has no import table")
    start = locate(import_rva, import_size)
    imports = []
    for offset in range(start, start + import_size - 19, 20):
        fields = read("IIIII", offset)
        if not any(fields):
            break
        name_offset = locate(fields[3], 1)
        end = data.find(b"\0", name_offset, name_offset + 256)
        name = data[name_offset:end] if end >= 0 else b""
        require(re.fullmatch(rb"[a-zA-Z0-9_.-]+\.dll", name, re.I) is not None,
                "PE import is not a DLL basename")
        imports.append(name.decode("ascii"))
    else:
        raise ValueError("PE import table is missing its terminator")
    require(bool(imports), "PE executable does not import any DLLs")

    debug_rva, debug_size = directory(6)
    if debug_size:
        require(debug_size % 28 == 0, "invalid PE debug directory")
        start = locate(debug_rva, debug_size)
        for offset in range(start, start + debug_size, 28):
            kind, length, _, pointer = read("IIII", offset + 12)
            if kind == 2:  # CodeView
                record = data[pointer:pointer + length]
                prefix = {b"RSDS": 24, b"NB10": 16}.get(record[:4])
                require(prefix is not None and len(record) == length, "invalid PE CodeView record")
                name, separator, _ = record[prefix:].partition(b"\0")
                require(bool(separator) and re.fullmatch(rb"[a-zA-Z0-9_.-]+\.pdb", name, re.I) is not None,
                        "PE CodeView path is not a PDB basename")
    return imports


def archive_contents(raw: bytes, root: str, tracked: dict[str, str], roms: dict[int, set[str]],
                     prefixes: set[bytes], packs: dict[str, bytes]) -> tuple[bytes, bytes, int]:
    expected = RELEASE_FILES | set(tracked) | set(packs)
    dirs = {str(parent) for name in expected for parent in PurePosixPath(name).parents}
    dirs.discard(".")
    pack_dirs = {str(parent) for name in packs for parent in PurePosixPath(name).parents}
    seen = set()
    files = set()
    notices = 0
    with zipfile.ZipFile(io.BytesIO(raw)) as package:
        end = len(raw) - len(package.comment) - 22
        require(raw[end:end + 4] == b"PK\x05\x06", "ZIP has unexpected trailing data")
        scan_paths(package.comment, prefixes, "ZIP comment")
        members = package.infolist()
        require(bool(members) and min(member.header_offset for member in members) == 0,
                "ZIP is empty or contains a prepended payload")
        for member in members:
            path = member_path(member, root)
            key = path.casefold()
            require(key not in seen, "duplicate ZIP member")
            seen.add(key)
            suffix = Path(path).suffix.casefold()
            dependency = path.startswith("licenses/rust-dependencies/")
            components = key.split("/")
            pack_member = path in packs or (member.is_dir() and path in pack_dirs)
            require(not PRIVATE_COMPONENTS.intersection(components)
                    and (pack_member or not PACK_COMPONENTS.intersection(components))
                    and (path == "sf-game.exe" or suffix not in PRIVATE_SUFFIXES
                         or (path in packs and suffix == ".vox")),
                    "ZIP contains local data or an unexpected binary")
            if member.is_dir():
                require(not path or path in dirs or path in ("licenses", "licenses/rust-dependencies")
                        or dependency, "unexpected ZIP directory")
            else:
                require(path in expected or dependency, "unexpected ZIP file")
            scan_paths(member.filename.encode() + b"\0" + member.comment + b"\0" + member.extra,
                       prefixes, "ZIP member metadata")
            offset = member.header_offset
            require(raw[offset:offset + 4] == b"PK\x03\x04", "invalid ZIP local file header")
            name_size, extra_size = struct.unpack_from("<HH", raw, offset + 26)
            header_end = offset + 30 + name_size + extra_size
            require(header_end <= len(raw), "truncated ZIP local file header")
            scan_paths(raw[offset + 30:header_end], prefixes, "ZIP local file header")
            # ZipFile checks the CRC as each member is read, including empty directories.
            data = package.read(member)
            if member.is_dir():
                require(not data, "ZIP directory carries a payload")
                continue
            require(data or path not in RELEASE_FILES, f"ZIP contains an empty {path}")
            require(len(data) not in roms or hashlib.sha1(data).hexdigest() not in roms[len(data)],
                    "ZIP contains an original game ROM")
            scan_paths(data, prefixes, f"ZIP member {path}")
            files.add(path)
            notices += dependency
            if path in packs:
                require(data == packs[path], f"tagged pack file differs: {path}")
            elif path.startswith("shaders/"):
                blob = b"blob " + str(len(data)).encode("ascii") + b"\0" + data
                require(hashlib.sha1(blob).hexdigest() == tracked[path], f"tagged shader differs: {path}")
            elif path == "sf-game.exe":
                game = data
            elif path == "BUILD.txt":
                build = data
    require(expected <= files, "ZIP is missing expected package files")
    require(notices > 0, "ZIP is missing dependency notices")
    return game, build, len(files)


def verify(archive: Path, exe: Path, source: Path, tag: str, commit: str, extra: list[Path]) -> None:
    require(re.fullmatch(r"v[0-9]+\.[0-9]+\.[0-9]+", tag) is not None, "expected a vX.Y.Z release tag")
    require(re.fullmatch(r"[0-9a-f]{40}", commit) is not None, "expected the full Git commit SHA")
    root = f"sf1-port-{tag}-windows-x86_64"
    require(archive.name == f"{root}.zip", "unexpected ZIP basename")
    sidecar = archive.with_suffix(".zip.sha256")
    archive_bytes = archive.read_bytes()
    zip_digest = hashlib.sha256(archive_bytes).hexdigest()
    require(sidecar.read_text(encoding="utf-8").strip() == f"{zip_digest}  {archive.name}",
            "ZIP SHA-256 sidecar does not match the archive")
    tracked = tagged_files(source, tag, commit)
    packs = tagged_pack_files(source, commit)
    roms: dict[int, set[str]] = {}
    manifest = json.loads(git(source, "show", f"{commit}:tools/reference/rom-manifest.json"))
    for row in manifest["entries"]:
        roms.setdefault(row["size"], set()).add(row["sha1"])
    prefixes = private_prefixes(source, exe, extra)
    game, build, count = archive_contents(archive_bytes, root, tracked, roms, prefixes, packs)
    game_digest = hashlib.sha256(game).hexdigest()
    with exe.open("rb") as stream:
        require(hashlib.file_digest(stream, "sha256").hexdigest() == game_digest,
                "packaged executable differs from the release build")
    imports = pe_imports(game)
    with tempfile.TemporaryDirectory(prefix="sf1-port-verify-") as tmp:
        extracted = Path(tmp) / "sf-game.exe"
        extracted.write_bytes(game)
        banner = subprocess.check_output([str(extracted), "--version"], cwd=tmp,
                                         text=True, encoding="utf-8", timeout=30).strip()
    require(banner.endswith(f" {tag} ({commit[:8]})"), "extracted executable version disagrees with the tag")
    expected_build = {f"Tag: {tag}", f"Source commit: {commit}", "Target: x86_64-pc-windows-msvc",
                      f"Executable version: {banner}", f"sf-game.exe SHA-256: {game_digest}"}
    require(expected_build <= set(build.decode("utf-8").splitlines()), "BUILD.txt disagrees with verified package")
    print(f"Version: {banner}")
    print(f"ZIP SHA-256: {zip_digest}")
    print(f"EXE SHA-256: {game_digest}")
    print(f"Verified {count} ZIP files and {sum(p.startswith('shaders/') for p in tracked)} tagged shaders.")
    print(f"Verified {len(packs)} tagged art, voice, and music pack files.")
    print("Windows x64 GUI imports: " + ", ".join(imports))
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
            zipfile.BadZipFile) as error:
        print(f"Package verification failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
