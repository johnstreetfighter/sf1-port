"""Regenerate the ROM verification manifest from the pinned MAME source.

Fetches (or reads) the pinned sf.cpp and transcribes ROM_START(sf) into
tools/reference/rom-manifest.json: name, region, offset, size, CRC32, SHA1
per entry. The manifest records hashes only, never game data.

Usage:
    python gen_rom_manifest.py [path-to-sf.cpp]

Without an argument, downloads sf.cpp at the pinned verification commit.
"""

from __future__ import annotations

import json
import re
import sys
import urllib.request
from pathlib import Path

PINNED_COMMIT = "774a180df2b6ef9c53e48337792256817c50f768"
SOURCE_URL = (
    f"https://raw.githubusercontent.com/mamedev/mame/{PINNED_COMMIT}/src/mame/capcom/sf.cpp"
)
OUT = Path(__file__).resolve().parent / "rom-manifest.json"


def parse_rom_start(source: str) -> list[dict]:
    match = re.search(r"ROM_START\( sf \)(.*?)ROM_END", source, re.S)
    if not match:
        raise SystemExit("ROM_START(sf) block not found in the supplied source")
    region = None
    entries = []
    for line in match.group(1).splitlines():
        line = line.strip()
        region_match = re.match(r'ROM_REGION\(\s*(0x[0-9a-fA-F]+),\s*"([^"]+)"', line)
        if region_match:
            region = region_match.group(2)
            continue
        load_match = re.match(
            r'ROM_LOAD(?:16_BYTE)?\(\s*"([^"]+)",\s*(0x[0-9a-fA-F]+),\s*(0x[0-9a-fA-F]+),'
            r"\s*CRC\(([0-9a-fA-F]+)\)\s*SHA1\(([0-9a-fA-F]+)\)",
            line,
        )
        if load_match:
            name, offset, size, crc, sha1 = load_match.groups()
            entries.append(
                {
                    "name": name,
                    "region": region,
                    "offset": int(offset, 16),
                    "size": int(size, 16),
                    "crc32": crc.lower(),
                    "sha1": sha1.lower(),
                }
            )
    if len(entries) != 44:
        raise SystemExit(f"expected 44 ROM entries, parsed {len(entries)}")
    return entries


def main(argv: list[str]) -> int:
    if len(argv) > 2:
        print(__doc__)
        return 2
    if len(argv) == 2:
        source = Path(argv[1]).read_text(encoding="utf-8")
    else:
        with urllib.request.urlopen(SOURCE_URL) as response:
            source = response.read().decode("utf-8")
    manifest = {
        "rom_set": "sf",
        "description": "US set 1 (MAME set sf), six-button controls",
        "source": f"https://github.com/mamedev/mame/blob/{PINNED_COMMIT}/src/mame/capcom/sf.cpp ROM_START(sf)",
        "entries": parse_rom_start(source),
    }
    OUT.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {OUT} ({len(manifest['entries'])} entries)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
