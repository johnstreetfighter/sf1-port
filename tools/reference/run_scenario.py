"""Run a reference scenario on the pinned MAME baseline.

Usage:
    python run_scenario.py scenarios/<name>.json [--out DIR] [--mame EXE] [--rompath DIR]

The runner owns the baseline machine options (pinned in
docs/reference/mame-baseline.md); the scenario file owns reference identity,
frame-indexed logical inputs, duration, and observation settings.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

from lua_gen import generate_driver, verify_dip_readbacks, verify_readback, write_cfg
from scenario import ScenarioError, load_scenario

HERE = Path(__file__).resolve().parent

# Pinned baseline machine options: headless, unthrottled, deterministic.
BASELINE_OPTIONS = [
    "-video", "none",
    "-sound", "none",
    "-nothrottle",
    "-skip_gameinfo",
    "-noreadconfig",
    "-nowriteconfig",
]

SCREEN_WIDTH = 384
SCREEN_HEIGHT = 224


def rgba_to_png(rgba_path: Path) -> Path:
    from PIL import Image

    buf = rgba_path.read_bytes()
    expected = SCREEN_WIDTH * SCREEN_HEIGHT * 4
    if len(buf) != expected:
        raise ValueError(f"{rgba_path.name}: expected {expected} bytes, got {len(buf)}")
    # MAME emits one little-endian ARGB u32 per pixel, i.e. B,G,R,A byte order.
    img = Image.frombytes("RGBA", (SCREEN_WIDTH, SCREEN_HEIGHT), buf, "raw", "BGRA")
    png_path = rgba_path.with_suffix(".png")
    img.save(png_path)
    return png_path


def run(args: argparse.Namespace) -> int:
    s = load_scenario(args.scenario)
    out_dir = Path(args.out).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)

    driver_path = out_dir / "generated_driver.lua"
    driver_path.write_text(generate_driver(s, out_dir.as_posix()), encoding="utf-8")

    options = list(BASELINE_OPTIONS)
    if s.dipswitches_explicit or s.dip_changes:
        write_cfg(s, out_dir)
        options = [opt for opt in options if opt != "-noreadconfig"]

    cap_seconds = int(s.duration_frames / 60 * 3) + 30
    cmd = [
        str(args.mame),
        s.machine_set,
        "-rompath", str(args.rompath),
        *options,
        # Keep MAME's side-effect directories out of the source tree.
        "-cfg_directory", str(out_dir / "cfg"),
        "-snapshot_directory", str(out_dir / "snap"),
        "-autoboot_script", str(driver_path),
        "-autoboot_delay", "0",
        "-seconds_to_run", str(cap_seconds),
    ]
    print(f"[run] scenario {s.scenario_id}, {s.duration_frames} frames")
    print(f"[run] {' '.join(cmd)}")
    started = time.monotonic()
    proc = subprocess.run(cmd, capture_output=True, text=True)
    wall_seconds = time.monotonic() - started

    sys.stdout.write(proc.stdout)
    if proc.stderr.strip():
        sys.stderr.write(proc.stderr)
    if proc.returncode != 0:
        print(f"[run] MAME exited with code {proc.returncode}")
        return 1

    frames_tsv = out_dir / "frames.tsv"
    if not frames_tsv.exists():
        print("[run] frames.tsv missing: the driver never ran or exited early")
        return 1

    try:
        verify_readback(s, out_dir / "meta.tsv")
        if s.dip_changes:
            verify_dip_readbacks(s, out_dir / "dip_readbacks.tsv")
    except ScenarioError as exc:
        print(f"[run] DIP readback verification failed: {exc}")
        return 1

    pngs = []
    for rgba in sorted(out_dir.glob("pixels_*.rgba")):
        pngs.append(rgba_to_png(rgba).name)

    version = subprocess.run(
        [str(args.mame), "-version"], capture_output=True, text=True
    ).stdout.strip()

    files = {}
    for p in sorted(out_dir.iterdir()):
        if p.is_file() and p.suffix != ".png":
            files[p.name] = hashlib.sha256(p.read_bytes()).hexdigest()

    summary = {
        "scenario_id": s.scenario_id,
        "mame_version_reported": version,
        "mame_version_expected": s.mame_version,
        "wall_seconds": round(wall_seconds, 2),
        "completed_at_utc": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "png_previews": pngs,
        "sha256": files,
    }
    (out_dir / "summary.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(f"[run] done in {wall_seconds:.1f}s wall; capture in {out_dir}")
    if version != s.mame_version and not version.startswith(s.mame_version):
        print(f"[run] WARNING: MAME reports '{version}', scenario pins '{s.mame_version}'")
    return 0


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("scenario", help="path to the scenario JSON file")
    parser.add_argument("--out", required=True, help="capture output directory")
    parser.add_argument("--mame", default=HERE / "local" / "mame" / "mame.exe")
    parser.add_argument("--rompath", default=HERE / "local" / "roms")
    return run(parser.parse_args(argv))


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
