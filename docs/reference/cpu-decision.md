# The fixed fight's first CPU decision

This document records the boundary established by
issue #12: from a
documented state of the fixed fight, the CPU opponent's first decision is
advanced natively — the translated AI routine picks the move from the
opponent's AI tables and the random state — and its visible outcome (a
forward jump and the landing) matches the pinned reference frame by frame.
This is one decision branch of a controlled scenario, **not a complete AI
opponent and not a playable fight**.

## The scenario and the interval

Scenario `sf-cpu-01`: the fixed fight's boot path (coin 1450, 1P start
1520, default country confirmed 1900 — the same as sf-fixed-fight-01 /
sf-idle-scene-01), **1P Ryu vs CPU Retsu**, round 1, with **no fight inputs
at all** (P1 idle). The opponent is fixed by the boot path: country Japan
puts Retsu first.

**Compared interval: frames 2390..=2433** (44 frames) from the
checkpoint-2389 fixture.

- 2389 (fixture): FIGHT! has just been erased; both fighters stand (Ryu x
  0x9e, Retsu x 0x128); the fight loop has not run yet.
- 2390: the fight loop starts (P1's input handler, the CPU decision, the
  object runners); **the decision**: Retsu starts move 0x0a, a forward jump.
- 2433: Retsu lands in his stance. At 2434 the jump's stream calls the
  follow-up decision routine (0x45630) — outside the slice.

The fixture row 2389 is not compared per frame: its screen hash is the
scanout of 2388 (FIGHT! still visible), which a post-erase fixture cannot
reproduce. Its checkpoint dumps are the fixture itself.

Evidence: reference captures `cpu-01-run-a` / `cpu-01-run-b`
(repeatability: MATCH, byte-identical); probe capture `cpu-probe-a`
(checkpoints 2380..=2699) and a MAME instruction trace of frames 2389-2390
(`tools/reference/local/trace_2390.log`, local-only), which gave the exact
code path below.

## Initial conditions (all recorded in the fixture's work RAM)

| Item | Address | Value at 2389 |
| --- | --- | --- |
| Game mode | 0xffc002 | 2 (1P vs CPU) |
| Random state | 0xff8e4a..0xff8e51 | `a9 eb 5f 66 63 fb 45 be` |
| CPU player structure | 0xffc664 | entity 0x0a00 (Retsu), other 0x0400 (Ryu), move table 0x16e6c, health 4/11, range roll +0x16..+0x1b all 0 |
| Range classes (by random nibble) | 0xffc058 | `00 01 00 00 01 00 02 00 01 00 00 00 00 01 00 02` |
| Preferred ranges (low byte used) | 0xffc068 | words `084c 8296 bedc` → 0x4c / 0x96 / 0xdc |
| Distance limits | 0xffc06e / 0xffc070 | 0x0047 (near), 0x0080 (middle) |
| Near moves | 0xffc074 | `02 02 02 02 02 02 02 02 02 02 02 0a 02 02 02 02` |
| Middle moves | 0xffc084 | `02 02 02 02 02 02 0a 0a 02 0a 02 0a 0a 02 0a 02` |
| Far moves | 0xffc094 | `0a 02 0a 02 0a 02 0a 02 0a 0a 02 0a 0a 02 0a 0a` |
| Stunned moves (pairs) | 0xffc0a4 | `04 00 04 02 04 04 03 00 0a 00 …` |
| Fighter centres | entities +6 / +0xd | Ryu 0x9e + 0x20 = 0xbe; Retsu 0x128 + 0x20 = 0x148 |
| Random-task loop | — | passes at 2383 + 7k (the 2390 pass runs after the decision) |

The AI tables are copied into work RAM before the round by the setup
routine at 0x44880 from per-opponent ROM data, selected through pointer
tables at 0xffc020.. by stage and round parameters (0xffc003/4/6/7); their
contents appear in the program ROM around 0x1f056..0x1f284 (range classes
at 0x1f056, preferred ranges 0x1f0d6, limits 0x1f0dc, move tables 0x1f0e4 /
0x1f0f4 / 0x1f104, stunned pairs 0x1f264). That copy is **not translated**
in this slice: the native run reads the fixture's RAM copy.

## The decision (0x40bd4, translated as `Bus::cpu_decide`)

Run once per frame after P1's input handler (fight loop 0x3ea98):

1. A busy fighter (+9 bit 7) or one still waiting (entity +0x1a nonzero,
   decremented) does nothing.
2. It turns to face the opponent if needed (a turn takes the frame). Retsu
   already faces left (+0x37 = 4) with Ryu to his left.
3. A knocked-down fighter (+8 == 8) does nothing; a stunned one (+9 bit 3)
   picks from the stunned pairs with random byte 2.
4. Unless the range roll is held (+0x1b == 0x80), roll a preferred range:
   class = 0xffc058[random byte 0 & 0xf], range = 0xffc068[class].
   **0xa9 & 0xf = 9 → class 0 → range 0x4c.**
5. Distance from the preferred range: |0x148 − 0xbe| − 0x4c = 0x8a − 0x4c =
   **0x3e** (no borrow, so not "closer than preferred"). 0x3e ≤ 0x47 →
   **near table**.
6. Move = table[random byte 1 & 0xf] (+1 when closer than preferred):
   **0xeb & 0xf = 0xb → near[0xb] = 0x0a**, started through 0x441b0 from
   Retsu's move table (header at 0x16e6c + 0x0a·0x30 = 0x1704c).

## The outcome: move 0x0a, the forward jump

Header 0x1704c: anticipation pose 0x1129 (shown x-flipped as 0x112b), state
0x81, sound command **0xa6** (queued on 2390, on the latch at 2391),
landing sound 0x19 (+0x12), stream 0xc5c8. The stream (decoded from ROM;
steps are facing-relative, negated for the left-facing Retsu):

| Stream | Effect |
| --- | --- |
| 0xc5c8 | call 0x45a7a: posture bits of +9 → rising (1) |
| 0xc5ce | 0x1244 ×8 repeats, step (dy −9, dx −5): 2394-2401 |
| 0xc5ee | 0x1288 ×8, (−4, −5): 2402-2409, apex y 0x20 |
| 0xc60e | 0x1288 held 3 frames: 2410-2412 |
| 0xc62e | 0x1288 ×8, (+4, −4): 2413-2420 |
| 0xc64e | 0x1244 ×8, (+9, −4): 2421-2428 — x held at 0xb6 by the separation push against Ryu (0x43d7c) |
| 0xc66e | call 0x45ab2: posture bits → 2 |
| 0xc674 | landing crouch 0x1129: 2429 |
| 0xc694 | call 0x454ee: landing sound 0x19 queued (latch 2431) |
| 0xc69a | 0x1129 held 3 frames |
| 0xc6ba | stance 0x1000: 2433 |
| 0xc6da | call 0x45630: follow-up decision (2434, not recovered) |

Repeats use the runner's movement path (frame hold count +0xb, 0x42cd6);
the jump state 0x81 routes the runner through the air-attack buffering
branch (0x42b34; the CPU has no latched strength, so nothing is buffered).

Meanwhile Ryu's input handler (P1 idle, neutral) restarts his stance (move
1) at 2390 with no visible change, the stage drift (0x438ea) moves
bg_scroll +1 every fifth frame from 2392, and the camera has no requests.

## Native implementation

- `crates/sf-engine/src/cpu.rs`: `CpuMachine` — the checkpoint-2389
  fixture plus the program data; per frame the vblank latches / IN1 copy /
  sound pop, then (from 2390) P1's input handler, the CPU decision, the
  stage drift, both object runners in the dispatcher's order (pair table
  0x42a88 for 1P vs CPU), effects, camera, and the random task.
- `crates/sf-engine/src/fighter.rs`: the decision (`cpu_decide`), the stage
  drift (`drift_stage`), the mode-dependent dispatch order and camera
  player structures, the vblank latch dispatch (0x3b05a), the jump's
  stream calls (0x45a7a / 0x45ab2 / 0x45a96 / 0x454ee), and the air-strength
  buffering (0x42b34), on top of the ticket #9 routines.
- Disclosed observed events (round-flow and timer tasks not translated):
  the TIME HUD written at 2390 (row 28, "TIME 100") and its digit ticks
  every 19 frames from 2409.

## Development launcher

`cargo run -p sf-dev --release -- cpu` (from the crate directory) plays the
native decision scene in a loop; P1 is idle, as in the scenario.

## Verification

- Reference repeatability: `cpu-01-run-a` vs `cpu-01-run-b` — MATCH.
- Native vs reference: `sf-engine capture-cpu` vs `cpu-01-run-a`, scope
  2390..=2433 — MATCH, 44 frames, 44 checkpoints (pixels, video/palette
  RAM, screen hashes, latch timing), with no untranslated branch reached.
- Machine state beyond the screen (example `compare_ram_cpu`): the whole
  object RAM (both fighters, effect slots), the CPU player structure, P1's
  structure (except the untranslated command trackers), the sound queue,
  the random state, camera and drift variables, and both scroll copies are
  byte-identical to the reference on every frame through 2433. Run one
  frame further, the native run stops at 0x45630 — the next decision — and
  reports it instead of continuing.
- Acceptance test: `crates/sf-engine/tests/cpu_scene.rs` (repeatability,
  native match, and the decision's inputs and outcome: random bytes, range
  class and range, near limit, the chosen entry, move 0x0a / state 0x81,
  the landing stance at x 0xb6, and the stop at 0x45630).

## Adjacent branches not yet recovered

> **Update (ticket #25):** the follow-up decision 0x45630 and Retsu's
> attacks are translated since; see docs/reference/cpu-follow-up.md. The
> list below is as this slice recorded it.

- The follow-up decision called from the jump stream (0x45630: CPU +0x19
  countdown, the opponent's state, random byte 3 and the table at
  0xffc0d4, special-move choices) — the decision at 2434.
- The other outcomes of 0x40bd4 itself are translated but unexercised:
  turning (0x40c3c / 0x40c24), waiting on +0x1a, the stunned table, the
  held range roll (+0x1b == 0x80, set elsewhere), the middle and far
  tables, and the "closer than preferred" move-id +1 variants (e.g. move
  0x03 next to 0x02). The moves those tables name (0x02 is the most common
  entry) are not verified against the reference.
- The AI table setup (0x44880) and its per-opponent ROM data.
- The round-flow and timer tasks (TIME HUD, timer countdown, round end).
- Any P1 input in this scene: P1's direction branches are not translated
  for the 1P fixture (the versus walk tables are a different fixture), so
  the launcher's CPU mode ignores keys.
