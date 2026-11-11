"""Determinism check: byte-compare two capture directories.

Reference scenarios must be reproducible from a fresh initialization; this
compares every artifact two runs produced and reports the first difference.
Ticket #3 (comparison runner) will own gameplay-divergence reporting; this
tool only answers "did the same scenario produce the same observations?".
"""

from __future__ import annotations

import sys
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class CompareResult:
    identical: bool
    first_difference: tuple[str, int | None] | None
    files_compared: int


def _first_byte_offset(a: bytes, b: bytes) -> int:
    for i, (x, y) in enumerate(zip(a, b)):
        if x != y:
            return i
    return min(len(a), len(b))


# Files the runner produces that are tooling, not observations: the generated
# driver embeds the run's output path, the summary records wall-clock time,
# and PNGs are human-viewable conversions of the authoritative .rgba pixels.
TOOLING_ARTIFACTS = {"generated_driver.lua", "summary.json"}


def _is_observation(p: Path) -> bool:
    return p.is_file() and p.name not in TOOLING_ARTIFACTS and p.suffix != ".png"


def compare_capture_dirs(dir_a: str | Path, dir_b: str | Path) -> CompareResult:
    dir_a, dir_b = Path(dir_a), Path(dir_b)
    files_a = {p.name: p for p in sorted(dir_a.iterdir()) if _is_observation(p)}
    files_b = {p.name: p for p in sorted(dir_b.iterdir()) if _is_observation(p)}

    compared = 0
    for name in sorted(files_a.keys() | files_b.keys()):
        if name not in files_a or name not in files_b:
            return CompareResult(False, (name, None), compared)
        content_a = files_a[name].read_bytes()
        content_b = files_b[name].read_bytes()
        if content_a != content_b:
            return CompareResult(
                False, (name, _first_byte_offset(content_a, content_b)), compared
            )
        compared += 1
    return CompareResult(True, None, compared)


def main(argv: list[str]) -> int:
    if len(argv) != 3:
        print("usage: compare_captures.py <capture-dir-a> <capture-dir-b>")
        return 2
    result = compare_capture_dirs(argv[1], argv[2])
    if result.identical:
        print(f"IDENTICAL: {result.files_compared} files match byte-for-byte")
        return 0
    name, offset = result.first_difference
    if offset is None:
        print(f"DIFFERENT: file {name} exists in only one capture")
    else:
        print(f"DIFFERENT: {name} first differs at byte offset {offset}")
    return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
