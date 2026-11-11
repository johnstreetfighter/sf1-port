# Versus movement: native horizontal movement from logical direction inputs

> **Superseded implementation (issue #15).** The native tables described below
> were built from observed behavior and have been deleted. The same scenario now
> runs through the translated input handler and Ryu's ROM move streams, and
> matches with his entity RAM byte-identical as well. See
> [posture-dispatch.md](posture-dispatch.md). The observations below remain the
> captured evidence.

This document records the boundary established by
issue #6: keyboard-mappable
logical direction inputs drive Ryu's horizontal movement in a bounded 2P
versus scene, advanced natively in Rust (`crates/sf-engine::versus`) and
matching the pinned reference run frame by frame on the slice's declared
observables (per-frame screen hashes, full-pixel checkpoints every frame,
video/palette RAM dumps, sound-latch timing).

Native state updates drive the output: per-frame walk/edge/camera tables
translated from observed original behavior update the machine-visible RAM
regions, and sf-video renders the result. No captured frames are replayed
and no original CPU code is executed.

## The bounded interval

**Frames 2450..=3350 of scenario `sf-move-01`** (901 frames), a 2P versus
match, Ryu (P1) vs Ken (P2), both defaulting to Japan. P2 is never touched:
versus mode has no CPU AI, so Ken is a standing, controlled opponent for the
whole interval.

- **Boot path** (reference only; the native interval starts after it):
  coin1 1450, coin2 1480, 2P start (start2) 1560, P1 country confirm
  (start1) 1900, P2 country confirm (start2) 2000. Fighters reach their
  stances at 2391; FIGHT! at 2400 (erased 2429); TIME HUD written at 2430
  showing 100; first timer tick to 099 at 2449.
- **Start**: the checkpoint-2450 fixture (fighters standing, TIME showing
  099, fg_scroll 0x1d40 i.e. camera 0x140, bg_scroll 0x1814).
- **End**: frame 3350, 25 frames of standing after the third input's stop
  (3325); the probe data ends there.

Evidence: `tools/reference/local/probe_move.lua` / `probe_detail.lua`
(local-only probes) recorded per-frame objectram blobs, write taps on the
scroll/gfxctrl registers and the text layer, and sound-latch readbacks over
the interval; two independent slice captures (`move-run-a`, `move-run-b`)
match each other exactly (901 frames, 901 checkpoints).

## Inputs and latency

P1 logical direction inputs (scenario `p1_right` / `p1_left`, IN1 bits 0/1):

| Input event | Hold | Movement observed |
| --- | --- | --- |
| right 2460 | 70 (released 2530) | walk right 2461-2543; pinned against Ken from 2503; stands at 2546 |
| left 2600 | 450 (released 3050) | walk left 2601-3057; camera follows from 2660, clamps at 2743; stage-edge oscillation; stands at 3060 |
| right 3120 | 200 (released 3320) | walk right 3121-3324 with the camera clamped; pinned at the same world position; stands at 3325 |

An input event at frame `f` first appears in the end-of-(`f`+1) state (the
reference driver applies inputs at `frame_done(f)`); the native runner and
the sf-dev launcher keep the same convention. Releasing a direction does not
stop the walk immediately: the current 17-frame cycle finishes through its
0063 (stance) phase, then the fighter stands.

## Walk cycles

Each walk is a 17-frame cycle of big-sprite grid states; per frame the
sprite-code base, anchor sy, and anchor dx come from tables translated out
of the probe objectram blobs (right walk observed 2461-2543 and 3121-3324,
left walk 2601-3057):

- **Right**: 0604 x6 (dx +8,0,0,+2,+2,+2; sy 88,88,87,86,85,84) → 0608 x5
  (+2,+2,+2,+2,0; sy 85,86,87,88,88) → 060c x3 (+10,0,0) → 0063 x3 (+8,0,0).
- **Left**: 0608 x6 (-16,0,-2,-2,-2,-2; sy 88,88,87,86,85,84) → 0604 x5
  (-4,-2,-2,-2,0; sy 85,86,87,88,88) → 060c x3 (-4,0,0) → 0063 x3 (-6,0,0).

The 0063 phase is the standing pose (code 0x063, the fixture stance), so a
releasing walk blends into standing.

Sprite grid: six big-sprite entries (attr 0x400) at objectram words
0x200+0x20k (original addresses 0xffe400+0x40k), entry (row, col) at
code = base + row*0x20 + col*2, position (sx + col*0x20, sy + row*0x20).
Ken's six entries sit at words 0x500+0x20k (0xffea00+), attr 0x0501,
code = 0x865 + row*0x20 - col*2 — the mirrored column order is P2's
left-facing orientation (the facing path this slice translates).

## Boundary conditions observed in the fixture

- **Forward proximity stop (right)**: Ryu's world x stops at 0x242 = Ken's
  world x (0x262) - 0x20. The screen-x pin shifts with the camera: 0x102
  while cam = 0x140, 0x146 while cam = 0xfc. The walk animation keeps
  cycling while pinned.
- **Stage-left edge (left)**: once the anchor sx is <= 0x60 at frame start
  (observed switch between frames 2659/2660), the walk switches to an
  absolute-sx oscillation table per cycle frame (0x54..=0x60), independent
  of further input: 0608-phase 5c,5c,5a,58,56,54; 0604-phase 60 x5;
  060c-phase 5c x3; 0063-phase 5a x3.
- **Camera follow**: while the edge script runs, the camera moves -2 on
  cycle frames 6..=9 and -6 on frame 14, clamped at cam 0xfc
  (fg_scroll = 0x1c00 + cam, so 0x1d40 down to 0x1cfc; last camera write at
  2743). Ken's screen x tracks the camera (sx = 0x262 - cam).

## Background, HUD, and latches

- bg_scroll drifts +1 every 5 frames (frames 2432+5k) for the whole
  interval — an autonomous stage-layer drift, not the camera: 0x1814 at
  2450, last write 0x18c8 at 3347. (Register 0xc00018, write-only.)
- fg_scroll (0xc00014) only changes with the camera, above.
- TIME HUD: value 100 written at 2430 (text row 28, cols 9-16); the three
  digit cells (row 28, cols 14-16; word = 0x3000 | digit) tick down every 19
  frames from 2449 (099 at 2450, 052 at 3347). The original rewrites the
  cells every frame with unchanged values; only ticks change machine state,
  so only ticks are translated.
- gfxctrl low byte (0xc0001b) stays 0xe8; the sound latch stays 0xff (no
  sound commands for walking); palette RAM is untouched all interval.

## Development launcher

`cargo run -p sf-dev --release` opens the versus window on this fixture:
Left/Right arrows feed the same logical direction inputs the capture runner
replays from the scenario (`VersusMachine::set_p1_direction`), 60 fps. A key
counts as held only while the window is focused, so an alt-tab or input
reset releases everything — held keys cannot stick. Esc exits; at frame 3350
the fixture reloads, keeping the demo inside the validated interval.

## Verification

- Reference repeatability: `move-run-a` vs `move-run-b` — MATCH, 901 frames,
  901 checkpoints (`sf-compare`, scope 2450..=3350, fields
  screen_hash,latch,latch_changed).
- Native vs reference: `sf-engine capture-versus` output vs `move-run-a` —
  MATCH, 901 frames, 901 checkpoints (checkpoint pixels and video/palette
  RAM dumps byte-identical; per-frame screen hashes and latch timing equal).
- Acceptance test: `crates/sf-engine/tests/versus_scene.rs` runs both
  comparisons (skips when the local-only captures/ROMs are absent).
- The idle-scene acceptance test (#5) still passes unchanged.

## Not covered by this slice

Collision and push-out between the fighters (the proximity stop is the
observed world-x pin only), the simultaneous left+right tie-break (the
original's behavior is unobserved), keyboard remapping (sf-dev arrows are
fixed dev bindings), Ken's behavior as a controlled or AI opponent (#12),
and anything past frame 3350. This is controlled-fixture behavior, not
complete gameplay.
