# Idle scene: a bounded native interval with matching timing

> **Superseded implementation (issue #29).** The observed tables this slice
> used are deleted: the scene now runs on the one-player machine from its
> checkpoint, the game-flow task resuming in the translated intro loop
> (docs/reference/round-transition.md). The scene still matches every
> observable and its owned RAM every frame. The observations below remain
> the captured evidence.

This document records the boundary established by
issue #5: a short controlled
scene — the round-1 intro idle of the fixed fight — is advanced natively in
Rust (`crates/sf-engine`) and matches the pinned reference run frame by frame
on the slice's declared observables.

Native state updates drive the output: counters and tables translated from
observed original behavior update the machine-visible RAM regions, and
sf-video renders the result. No captured frames are replayed and no original
CPU code is executed.

## The bounded interval

**Frames 2280..=2388 of scenario `sf-idle-scene-01`** (109 frames), the same
boot path as sf-fixed-fight-01 (coin at 1450, start at 1520, country confirm
at 1900), with no fight inputs at all.

- **Start**: the checkpoint-2280 fixture (the state validated by
  docs/reference/first-frame.md). The fight scene's first objectram write is
  at frame 2279 (per-frame objectram CRCs), so 2280 is the first fully
  established fight frame and the first the fixture covers.
- **End**: frame 2388 is the last fully static frame. At 2389 the reference
  run erases the FIGHT! text; at 2390 it writes the TIME HUD and the CPU
  opponent takes its first AI action. Both are outside this slice: opponent
  AI is a later ticket (#12), and the timer HUD tick (first at ~2409) is
  reported below as uncovered.

Evidence: `tools/reference/local/probe_idle.lua` / `probe_idle2.lua`
(local-only probes) recorded per-frame CRCs of objectram/videoram/palette,
per-frame objectram blobs, register taps, and the driver-identical screen
hash over frames 2270..2395; the probe run's screen hashes match the
`fight-run-a` capture exactly on every overlapping frame, and two
independent slice captures (`idle-run-a`, `idle-run-b`) match each other.

## Timeline inside the interval

All frame numbers are end-of-frame RAM state; scanout/hash visibility lags
as described below. `t` = intro timer = frame - 2279.

| Frames | t | Event |
| --- | --- | --- |
| 2279 | 0 | fight scene activates: Ryu walk frame 0 at x=0x56, Retsu stance at x=0x16a |
| 2280 | 1 | interval start; sound latch command 0x28 (round-start jingle) |
| 2283+5k | 4+5k | bg_scroll += 1, k=0..15, stopping at 0x1810 (last write 2358) |
| every 9 from 2288 | 9..63 | Ryu walk steps 1..7 (see tables) |
| 2288 | 9 | Retsu anticipation crouch (codes 0x112b grid), x -2 |
| 2292..2326 | 13..47 | Retsu jump arc: rise dy -9, apex dy -4, plateau (y=0x20 held t=29..31), fall dy +4 then +9; x -2 per active frame, lands x=0x128 |
| 2327 / 2328 | 48 / 49 | Retsu landing crouch, then stance; static for the rest of the interval |
| 2351 | 72 | Ryu stance transition: 6 big-sprite entries (attr 0x400), 12 slots parked at (0,0) |
| 2360 | 81 | FIGHT! written to text row 13 cols 29-34 (color 0xa) |
| 2361 | 82 | sound latch command 0x94 (FIGHT! voice); cleared next frame |
| 2361..2388 | | fully static: sprites, videoram, palette, scrolls all unchanged |
| 2389 | | (out of scope) FIGHT! erased |
| 2390 | | (out of scope) TIME HUD appears; CPU AI acts |

Ryu walk table (code base, dx) per step, base added to row*0x10+col over the
3x6 tile grid: (0x00, -) fixture, (0x03, +8), (0x06, +8), (0x09, +16),
(0x0c, +10), (0x60, +6), (0x00, +16), (0x03, +8) — x reaches 0x9e at the
stance transition. Retsu's code grids: stance {1002,1000,1022,1020,1042,1040},
anticipation {112b,...}, fast rise/fall {1246,...}, apex {128a,...}, attr
0x0503, 2x3 grid with +0x20 spacing.

## Pipeline timing of the observables

The two screen observables sit at different points of the emulated frame
pipeline, verified against the reference capture:

- **frames.tsv screen hash** (`screen:pixel`, the bitmap scanned out during
  frame N) reflects the **end-of-(N-1)** RAM state. Examples: objectram first
  written in S_2279 appears in the hash at 2280; the bg_scroll write of frame
  2283 appears at 2284; FIGHT! written in S_2360 appears at 2361.
- **checkpoint pixels** (`video:snapshot_pixels`) reflect the **end-of-N**
  state — the freshly rendered bitmap. The scroll write of 2283 is in
  pixels_002283; FIGHT! is in pixels_002360.
- **RAM dumps and the sound-latch read** are likewise end-of-N.

The hash also samples the raw 512x256 screen bitmap (`screen:pixel(x,y)` at
bitmap coordinates, visarea origin (64,16), opaque black border), so the
stride-4 FNV-1a maps sample (x,y) to visible (x-64, y-16) when in range.

## Fixture controls (disclosed)

The interval's initial state is the reference capture's checkpoint 2280:
videoram, paletteram, and the objectram tail of the work dump
(`ram_002280_{video,palette,work}.bin`). Values that RAM dumps cannot
provide are constants observed in the same run: bg_scroll 0x1800, fg_scroll
0x1d40, gfxctrl 0xe8 (write-only registers, from tap logs), sound latch
0x28 (audio space, from frames.tsv), and intro timer t=1.

The equivalent applied control on the reference side is the identical
scenario inputs; both runs share the boot path up to frame 2280.

## What is compared, and what is not

```
python tools/reference/run_scenario.py tools/reference/scenarios/sf_idle_scene_01.json \
    --out tools/reference/local/captures/idle-run-a   # pinned reference run

cargo run -p sf-engine -- capture-idle \
    --fixture-capture tools/reference/local/captures/idle-run-a \
    --out tools/reference/local/captures/idle-native-a

cargo run -p sf-compare -- compare \
    --scenario tools/reference/scenarios/sf_idle_scene_01.json \
    --reference tools/reference/local/captures/idle-run-a \
    --candidate tools/reference/local/captures/idle-native-a \
    --candidate-kind native --from 2280 --to 2388 \
    --fields screen_hash,latch,latch_changed
```

Validated per frame over 2280..=2388: screen hash, sound-latch value and
change timing; validated at all 109 checkpoints: full-screen pixels
byte-exact, videoram and paletteram dumps byte-exact. The acceptance test is
`crates/sf-engine/tests/idle_scene.rs` (skips on a clean checkout).

**Explicitly not covered by this slice** (reported, not silently absent):

- Work-RAM and audio-RAM CRC columns (`ram_crc`, `music_crc`): the native
  state models only video-relevant RAM plus the sound latch. The comparison
  scope excludes these columns; RAM dump regions `work`/`music` are absent
  from the native capture and skipped by the comparator.
- Opponent AI: the CPU's first action (2390) is outside the interval.
- The TIME HUD and round timer (first tick ~2409): outside the interval.
- Player inputs: none occur in the interval (first scripted fight input is
  at frame 2400).
- Audio output: latch command timing is translated; synthesis is #10/#11.
- The pre-interval intro (frames < 2280): the fixture init stands in for it.
- Ryu's idle stance is genuinely static in this interval (no breathing
  animation): per-frame objectram CRCs are unchanged across 2361..2388.

The animation tables in `crates/sf-engine/src/machine.rs` are translated
from the observed per-frame objectram sequence (provenance: the probes
above), retaining the original RAM addresses for every field. Locating the
corresponding script data in the original ROMs is future work (#13).
