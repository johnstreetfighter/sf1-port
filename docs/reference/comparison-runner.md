# Comparison runner: scenario boundary and invocation

This document records the boundary established by
issue #3 for subsequent
implementation slices: what a scenario is, what a capture is, how the Rust
comparison runner is invoked, and the evidence that it detects divergence.
No native game behavior exists yet; every result below is a
reference-versus-reference self-check and is labelled as such by the runner.

## The boundary

**Scenario** (committed, `tools/reference/scenarios/*.json`,
`format_version: 1`): reference identity (ROM set, pinned MAME version),
machine configuration, frame-indexed logical inputs, bounded duration, and
observation settings (checkpoint frames, CRC cadences). The runner parses
the same JSON contract as the Python reference tooling and validates it
(unknown controls, overlapping presses, missing fields, unsupported format
version are errors). The boundary names no translated function, module, or
internal representation.

**Capture** (local-only, `tools/reference/local/captures/<run>/`): the
observation set one adapter run produced — `frames.tsv` (per-frame screen
hash, sound latch, latch-changed flag, decimated RAM CRCs),
`checkpoints.tsv` + `pixels_*.rgba` (full checkpoint screens),
`ram_<frame>_<region>.bin` (raw RAM dumps), `meta.tsv` (machine readback),
`summary.json` (runner identity: scenario id, reported MAME version).
`summary.json` is tooling used to establish identity; it is never compared
as an observation. A future native adapter must write the same identity
fields so baseline identity checks keep working.

**Comparison**: a frame-ordered walk over both captures. The earliest
frame at which any observable differs is reported, whether the divergence
shows up in the per-frame hash/CRC stream, a checkpoint screen (with the
first differing pixel located), or a raw RAM dump (with region and offset).
Compatibility is checked first: both captures must be of the scenario being
compared, the reference capture must come from the pinned MAME version,
and both must agree on screen geometry.

## Input-data validation

`tools/reference/rom-manifest.json` records the 44 expected ROM entries
(name, region, size, CRC32, SHA1) transcribed from the pinned MAME source
(`ROM_START(sf)` at commit `774a180d…`; regenerate with
`python tools/reference/gen_rom_manifest.py`). The runner verifies the
local ROM set against it before comparing when `--roms` is passed; any
missing, resized, or corrupted entry fails with the entry name and the
actual checksum. The manifest contains hashes only; ROM files stay local.

## Documented invocation

From the repository root:

```
cargo run -p sf-compare -- compare \
  --scenario tools/reference/scenarios/sf_fixed_fight_01.json \
  --reference tools/reference/local/captures/<reference-run> \
  --candidate tools/reference/local/captures/<candidate-run> \
  --candidate-kind reference \
  --roms tools/reference/local/roms/sf
```

- Exit 0: observations match (a `--candidate-kind reference` run is
  explicitly labelled a reference-versus-reference self-check).
- Exit 1: first mismatch reported (frame, observable, expected vs actual,
  locating detail, scenario, baseline, reproduce command).
- Exit 2: input/validation failure — invalid ROMs, absent or malformed
  capture, incompatible baseline or scenario identity.

`cargo run -p sf-compare -- verify-roms` runs the ROM check alone.

## Evidence (recorded 2026-10-15, MAME 0.289 baseline)

- **Control comparison passes**: `fight-run-a` vs `fight-run-b` (two fresh
  captures of the fixed fight) → `MATCH: 5000 frames, 13 checkpoints`,
  exit 0, labelled a self-check.
- **Shifted gameplay event fails at the expected frame**: a capture of the
  same scenario with the light punch shifted +10 frames (2590 → 2600) →
  `MISMATCH at frame 2595`, observable `ram dump 'work'` offset `0x068f`
  (the input-state byte the game read from the pressed button), exit 1.
  This precedes the per-frame RAM-CRC divergence at frame 2600 and any
  pixel change; the frame-ordered walk reports the true first divergence.
- **Changed game pixel fails at the expected frame**: one pixel flipped in
  the frame-2280 checkpoint of a copied capture → `MISMATCH at frame 2280`,
  `checkpoint screen pixels`, `first differing pixel at (100, 0)`, exit 1.
- **Incompatible baseline is rejected**: comparing against the earlier
  `sf-probe-shifted` probe capture (different scenario id) → exit 2 naming
  both scenario ids; synthetic tests pin the MAME-version mismatch error.
- **Invalid ROMs are rejected**: synthetic corrupt/missing entries fail
  with the entry name and actual checksum; the real 44-file set verifies.

## Tests

```
cargo test -p sf-compare
```

21 tests at the CLI and library seams (synthetic fixtures only; no test
depends on local-only ROMs or captures). The real-capture checks above are
the documented manual verification for this slice, repeatable with the
commands quoted.

## Explicitly not claimed

- No native game behavior exists; nothing here demonstrates port fidelity.
- Sound observations remain latch commands and music-driver RAM CRCs;
  synthesized audio is not compared.
- The per-frame latch poll samples once per frame (see the baseline doc).
