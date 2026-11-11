# Crouch and return to standing

> **Superseded implementation (issue #15).** The native tables described below
> were built from observed behavior and have been deleted. The same scenario now
> runs through the translated input handler and Ryu's ROM move streams, and
> matches with his entity RAM byte-identical as well. See
> [posture-dispatch.md](posture-dispatch.md). The observations below remain the
> captured evidence.

This document records the boundary established by
issue #7: a logical down
input crouches Ryu and releases back to standing with the reference's state
transitions, animation, and timing, on the same versus fixture as
[movement](movement.md) (Ryu vs a standing, untouched Ken). Implemented in
`crates/sf-engine::versus` (`P1::Crouching`), matching the pinned reference
run frame by frame on the slice's declared observables.

## The bounded interval

**Frames 2450..=3399 of scenario `sf-crouch-01`** (950 frames). Same boot
path and checkpoint-2450 fixture as sf-move-01; the interval extends past
the movement slice's 3350 to cover the final settle.

Evidence: probe captures `crouch-run-a`/`crouch-run-b` (repeatability:
MATCH, 950 frames, 950 checkpoints) plus the `crouch-lat` probe for the
walk-phase deferral below (all local-only, produced by
`tools/reference/local/probe_move.lua`-style objectram blob dumps).

## Input sequence and observed transitions

| Input event | Hold | Behavior |
| --- | --- | --- |
| down 2460 | 70 | crouch at 2461 (neutral pose), stand at 2531 — one frame after release |
| right 2580 | 60 | walk baseline 2581-2645 |
| down 2700 | 120 | crouch at 2701; stand at 2821 |
| left 2740 | 60 | *while crouched*: back pose 2741-2800, then neutral again; no movement |
| right 2960 | 260 | walk 2961-3230 (pinned at sx 0x102) |
| down 3020 | 60 | *while walking*: crouch at 3023 (deferred, see below); walk restarts at 3081 |

All crouch entries and exits take effect one frame after the input event,
except the mid-walk entry (below). The pose tracks the held horizontal
direction one frame after any change.

## The crouch pose

The crouch replaces the 6-entry big-sprite stance/walk grid with **12 small
sprites** (attr 0x0000 — the large-sprite bit 0x400 is clear) over objectram
slots 0x200..0x360 (stride 0x20 words): code = base + row*0x10 + col, entry
(row, col) at (sx + col*0x10, sy + row*0x10), 3 columns x 4 rows. Anchor y
is 0xa8 (standing: 0x88); the anchor x shifts **+8** on entry and back **-8**
on exit (0x9e <-> 0xa6 free, 0x102 <-> 0x10a pinned).

Three pose bases, selected by the held horizontal direction (Ryu faces
right in this fixture):

| Direction held | Base | Observed |
| --- | --- | --- |
| none | 0x120 | 2461-2530, 2701-2740, 2801-2820 |
| right (toward) | 0x126 | 3023-3080, latency probe 2506/2581/2661/2741 |
| left (back) | 0x163 | 2741-2800, latency probe 3151 |

While crouched, horizontal inputs only switch the pose — there is no crawl
movement. Both-directions-held is outside the captured evidence (the model
prefers toward).

## Down during a walk: the cycle-phase deferral

A down press while walking is **not** honored immediately unless the walk
cycle is already in its 0063 (stance) phase:

- Press effective during the 0604/0608 phases (cycle idx 0..=10): the crouch
  is deferred until the cycle reaches idx 11 (the 060c-phase boundary).
  Observed twice: press effective at idx 6 -> crouch at idx 11 (frames
  2501->2506); press effective at idx 9 -> crouch at idx 11 (3021->3023).
- Press effective during the 0063 phase (idx 14..=16): crouch on the next
  frame (observed four times: 2581, 2661, 2741, 3151).
- Press effective during the 060c phase (idx 11..=13) is unobserved; the
  model honors it immediately, like 0063.

The walk simply continues while the deferral is pending (the input stays
held and re-checked each frame). A down tap released before idx 11 would be
swallowed entirely — unobserved, extrapolated from the same rule.

## Exits

- **Crouch -> stand** (down released, no direction held): stance grid
  rewritten, anchor -8, one frame after the release (2531, 2821).
- **Crouch -> walk** (down released with a direction held): a fresh walk
  cycle starts at idx 0 with its dx applied after the -8 anchor restore —
  no stance frame in between (observed 3081: 0063 never appears; left-walk
  exit at 3181 lands sx 0x6e = 0x86 - 8 - 16).
- On **every** crouch exit the six tail slots (0x2c0..0x360) are parked:
  sy = sx = 0 while codes/attr keep the last crouch pose's values
  (observed 2531, 2821, 3081, 3231).

## Unchanged machinery

bg_scroll drift (+1 every 5 frames, 2432+5k), TIME ticks (every 19 frames
from 2449), Ken's static camera-tracking entries, gfxctrl 0xe8, sound latch
0xff (no crouch sounds) — all as in [movement](movement.md); the interval's
checkpoint dumps confirm zero palette/videoram surprises.

## Development launcher

`cargo run -p sf-dev --release`: Down crouches, Left/Right walk (and switch
the crouch pose while crouched); the focus-loss clearing covers Down too.
Esc exits; the fixture reloads at frame 3350.

## Verification

- Reference repeatability: `crouch-run-a` vs `crouch-run-b` — MATCH, 950
  frames, 950 checkpoints.
- Native vs reference: `sf-engine capture-crouch` output vs `crouch-run-a` —
  MATCH, 950 frames, 950 checkpoints (checkpoint pixels and video/palette
  RAM byte-identical; hashes and latch timing equal).
- Acceptance test: `crates/sf-engine/tests/crouch_scene.rs`.
- The movement (#6) and idle (#5) acceptance tests still pass unchanged.

## Not covered by this slice (recorded for the remaining-work map)

Crouching attacks and every combat branch (their own tickets), the
both-directions tie-break, crouch entered directly with a diagonal from
standing (unobserved; the model derives the pose from the held direction at
entry), down during a *left* walk's early phases (the deferral is applied
symmetrically, observed only for right), crouch interaction with the
stage-edge camera script (the scenario never moves the camera), and Ken's
behavior.
