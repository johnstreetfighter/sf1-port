"""Stage and verify the playable assets declared by the tagged pack manifests."""

from __future__ import annotations

import argparse
import io
import json
from collections.abc import Iterable
from pathlib import Path
import re
import subprocess
import sys
import zlib


ART_PACK = "assets/sf1-port-art"
VOICE_PACK = "assets/sf1-port-voice"
PACKS = (ART_PACK, VOICE_PACK)
RESERVED_NAME = re.compile(r"(?i)(?:CON|PRN|AUX|NUL|COM[1-9¹²³]|LPT[1-9¹²³])(?:\.|$)")
EXCLUDED_FOLDERS = {"archive", "candidates", "captures", "roms", "sf1-port-music"}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def git(source: Path, *args: str, input: bytes | None = None) -> bytes:
    return subprocess.run(
        ["git", *args], cwd=source, input=input, stdout=subprocess.PIPE,
        stderr=subprocess.PIPE, timeout=30, check=True,
    ).stdout


def unique_fields(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result = dict(pairs)
    require(len(result) == len(pairs), "pack manifest contains a duplicate field")
    return result


def manifest_entries(pack: str, raw: bytes) -> dict[str, tuple[int, str]]:
    manifest = json.loads(raw.decode("utf-8"), object_pairs_hook=unique_fields)
    art = pack == ART_PACK
    field = "files" if art else "lines"
    fields = {"format", field} if art else {"format", field, "sample_rate"}
    require(isinstance(manifest, dict) and set(manifest) == fields
            and type(manifest["format"]) is int and manifest["format"] == 1,
            f"invalid format in {pack}/manifest.json")
    if not art:
        rate = manifest["sample_rate"]
        require(type(rate) is int and rate == 4000, f"invalid sample rate in {pack}/manifest.json")

    entries = manifest[field]
    require(isinstance(entries, dict) and bool(entries), f"empty or invalid entries in {pack}/manifest.json")
    files = {}
    for key, entry in entries.items():
        path = key if art else f"{key}.vox"
        parts = path.split("/")
        suffix = ".png" if art else ".vox"
        size_field = "size" if art else "nibbles"
        require(len(parts) == (2 if art else 1)
                and re.fullmatch(r"[a-z0-9_-]+" + re.escape(suffix), parts[-1]) is not None
                and all(not RESERVED_NAME.match(part) for part in parts)
                and (not art or (re.fullmatch(r"[a-z0-9_-]+", parts[0]) is not None
                                and parts[0] not in EXCLUDED_FOLDERS)),
                f"invalid playable asset path: {pack}/{path}")
        require(isinstance(entry, dict) and set(entry) == {size_field, "crc32"}
                and type(entry[size_field]) is int and entry[size_field] > 0
                and isinstance(entry["crc32"], str) and re.fullmatch(r"[0-9a-f]{8}", entry["crc32"]) is not None,
                f"invalid manifest entry for {pack}/{path}")
        size = entry[size_field]
        files[f"{pack}/{path}"] = (size if art else (size + 1) // 2, entry["crc32"])
    return files


def read_blobs(source: Path, tree: dict[str, tuple[bytes, bytes, bytes]],
               paths: Iterable[str]) -> dict[str, bytes]:
    blobs = {}
    for path in paths:
        mode, kind, digest = tree.get(path, (b"", b"", b""))
        require(kind == b"blob" and mode in (b"100644", b"100755"),
                f"tagged pack file is missing or is not regular: {path}")
        blobs[path] = digest
    if not blobs:
        return {}
    stream = io.BytesIO(git(source, "cat-file", "--batch", input=b"\n".join(blobs.values()) + b"\n"))
    files = {}
    for path, digest in blobs.items():
        header = stream.readline().split()
        require(len(header) == 3 and header[:2] == [digest, b"blob"], f"could not read tagged blob: {path}")
        size = int(header[2])
        data = stream.read(size)
        require(len(data) == size and stream.read(1) == b"\n", f"truncated tagged blob: {path}")
        files[path] = data
    return files


def tagged_pack_files(source: Path, commit: str) -> dict[str, bytes]:
    tree = {}
    for entry in filter(None, git(source, "ls-tree", "-rz", commit, "--", *PACKS).split(b"\0")):
        header, path = entry.split(b"\t", 1)
        mode, kind, digest = header.split()
        tree[path.decode("utf-8")] = (mode, kind, digest)
    files = read_blobs(source, tree, [f"{pack}/{name}" for pack in PACKS for name in ("manifest.json", "README.md")])
    expected = {}
    for pack in PACKS:
        require(bool(files[f"{pack}/README.md"]), f"empty pack README: {pack}")
        expected.update(manifest_entries(pack, files[f"{pack}/manifest.json"]))
    assets = read_blobs(source, tree, expected)
    for path, data in assets.items():
        size, crc = expected[path]
        require(len(data) == size and f"{zlib.crc32(data):08x}" == crc, f"tagged pack file differs from its manifest: {path}")
        if path.startswith(f"{ART_PACK}/"):
            require(data.startswith(b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR")
                    and len(data) >= 33 and data[24] in (1, 2, 4, 8) and data[25] == 3,
                    f"tagged art file is not an indexed PNG: {path}")
    return files | assets


def stage(source: Path, commit: str, package: Path) -> None:
    for name, data in tagged_pack_files(source, commit).items():
        destination = package / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(data)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("commit")
    parser.add_argument("package", type=Path)
    args = parser.parse_args()
    try:
        stage(args.source, args.commit, args.package)
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"Could not stage release packs: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
