# Neutral jump

> **Superseded implementation (issue #15).** The native tables described below
> were built from observed behavior and have been deleted. The same scenario now
> runs through the translated input handler and Ryu's ROM move streams, and
> matches with his entity RAM byte-identical as well. See
> [posture-dispatch.md](posture-dispatch.md). The observations below remain the
> captured evidence.

This document records the boundary established by
issue #8: a logical up input
jumps Ryu straight up and back down with the reference's takeoff, rise,
fall, and landing timing, on the same versus fixture as
[movement](movement.md) (Ryu vs a standing, untouched Ken). Implemented in
`crates/sf-engine::versus` (`P1::Jumping`), matching the pinned reference
run frame by frame on the slice's declared observables.

## The bounded interval

**Frames 2450..=2999 of scenario `sf-jump-01`** (550 frames). Same boot
path and checkpoint-2450 fixture as sf-move-01 / sf-crouch-01. P1 taps up
for 12 frames at 2460 and again at 2700 — the second jump validates that
the state after landing is indistinguishable from the state before the
first jump.

Evidence: reference captures `jump-01-run-a`/`jump-01-run-b`
(repeatability: MATCH, 550 frames, 550 checkpoints).

## The observed jump script

Both jumps are identical (the second is the first shifted by 240 frames),
so the jump is a fixed 40-frame script indexed from the anticipation entry
(frame 2461 for jump 1). Entry latency is one frame after the input event,
as for walk and crouch.

| Script idx | Frames (jump 1) | Phase | Sprites | Anchor |
| --- | --- | --- | --- | --- |
| 0..=8 | 2461-2469 | anticipation crouch | 15 small, base 0x10d | sy 0x98, sx = stance anchor |
| 9..=17 | 2470-2478 | rise | 3 big, base 0xcb | sy 0x80 - 2·(idx-9), sx = anchor + 0x10 |
| 18..=30 | 2479-2491 | airborne | 12 small, base 0xcd | sy 0x70 - 2·(idx-18) to apex 0x60 at idx 26 (2487), then +4/frame to 0x70; sx = anchor + 8 |
| 31..=35 | 2492-2496 | descent | 3 big, base 0xcb | sy 0x70 + 4·(idx-31), sx = anchor + 0x10 |
| 36..=39 | 2497-2500 | landing crouch | 15 small, base 0x10d | sy 0x98, sx = stance anchor |

The stance returns at idx 40 (frame 2501). Releasing up mid-jump changes
nothing (the 12-frame tap ends at 2472, mid-rise) — the script always
plays out in full.

Sprite layouts, all on Ryu's objectram slots 0x200.. with stride 0x20
words:

- **Anticipation/landing** (attr 0x0000): 5 rows × 3 columns over slots
  0x200..0x3c0, code = 0x10d + row*0x10 + col, entry (row, col) at
  (sx + col*0x10, 0x98 + row*0x10). Unlike the crouch pose there is no
  anchor-x shift.
- **Rise/descent** (attr 0x0400): one column of 3 big sprites at slots
  0x200/0x220/0x240, code = 0xcb + row*0x20, entry row at (sx, sy +
  row*0x20).
- **Airborne** (attr 0x0000): 4 rows × 3 columns over slots 0x200..0x360,
  code = 0xcd + row*0x10 + col.

Each phase switch parks the slots the previous phase used beyond the new
one — sy = sx = 0, codes/attr keep their last values:

| Transition | Parked slots | Codes retained from |
| --- | --- | --- |
| anticipation -> rise (2470) | 0x260..0x3c0 | anticipation (011d..014f) |
| rise -> airborne (2479) | 0x380..0x3c0 | anticipation (014d..014f) |
| airborne -> descent (2492) | 0x260..0x3c0 | airborne/anticipation (00dd..00ff, 014d..014f) |
| landing -> stance (2501) | 0x2c0..0x3c0 | landing (012d..014f) |

## Sound commands

- `0x82` on the second anticipation frame (script idx 1: 2462/2702) —
  the jump takeoff command.
- `0x19` on the second landing-crouch frame (script idx 37: 2498/2738) —
  the landing thud.

Each lasts exactly one frame; the latch is 0xff at every other frame of
the interval.

## Unchanged machinery

bg_scroll drift (+1 every 5 frames, 2432+5k), TIME ticks (every 19 frames
from 2449), Ken's static entries, the camera, gfxctrl 0xe8, and the
palette — all as in [movement](movement.md); the jump touches nothing but
Ryu's sprite entries and the latch.

## Development launcher

`cargo run -p sf-dev --release`: Up jumps (from standing), Down crouches,
Left/Right walk. Esc exits; the fixture reloads at frame 3350.

## Verification

- Reference repeatability: `jump-01-run-a` vs `jump-01-run-b` — MATCH, 550
  frames, 550 checkpoints.
- Native vs reference: `sf-engine capture-jump` output vs `jump-01-run-a` —
  MATCH, 550 frames, 550 checkpoints (checkpoint pixels and video/palette
  RAM byte-identical; hashes and latch timing equal). Both jumps, including
  the post-landing state, are covered by the per-frame comparison.
- Acceptance test: `crates/sf-engine/tests/jump_scene.rs`.
- The movement (#6), crouch (#7), and idle (#5) acceptance tests still
  pass unchanged.

## Not covered by this slice (recorded for the remaining-work map)

Diagonal jumps (up-left/up-right; the scenario only taps neutral up),
jumping attacks and every combat branch (their own tickets), up pressed
while walking or crouching (unobserved; the model only honors up from
standing), up held continuously through landing (the model would re-jump
immediately — unobserved), simultaneous direction+up tie-breaks (the
model gives up priority from standing), and Ken's behavior.
