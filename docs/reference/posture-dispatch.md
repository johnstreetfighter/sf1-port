# Walk, crouch and jump from the translated posture dispatch

This document records the boundary established by
issue #15. Ryu's walking,
crouching and jumping in the versus fixture are now produced by a
translation of the original input handler and his ROM move streams. They
are no longer produced by the per-frame tables that tickets #6–#8 built from
observed behavior; those tables are deleted. The observations recorded in
[movement.md](movement.md), [crouch.md](crouch.md) and [jump.md](jump.md)
remain the evidence those scenarios were captured with. The translation now
explains them.

This is a controlled-fixture result (2P versus, P2 = Ken never touched), not
complete gameplay.

## The input handler (0x402f8)

Each frame, for each human player structure (P1 0xff86b0, P2 0xff86da), the
handler runs as follows; translated as `fighter::Bus::input_handler`.

1. **Input byte.** It reads the player's input byte through the pointer at
   structure +2: active low, bits 0–3 = right, left, down, up.
2. **Knocked down.** A fighter that is down (entity +8 == 8) goes straight
   to the tail.
3. **Command trackers** (0x40360–0x403b2, tracker routine 0x40ae4). They run
   every frame unless the scene mode (0xffc008) is 0x40 or 0xc0.
   - The held direction is first mapped facing-relative through the 11-byte
     tables 0x403be (facing right) / 0x403c9 (facing left): 1 neutral,
     2 forward, 3 back, 5 down, 6 down-forward, 7 down-back, 9 up,
     0xa up-forward, 0xb up-back.
   - Directions 11–15 (impossible up+down combinations) read past the
     tables into the following bytes; the translation reads the same bytes.
   - Each tracker is three bytes (stage, frame count, done):

   | Tracker | Pattern | Command |
   | --- | --- | --- |
   | +0x16 | 0x4038e: 05 07 03 | D, DB, B (Tatsumaki) |
   | +0x19 | 0x403a2: 02 05 06 | F, D, DF (Shoryuken) |
   | +0x1c | 0x403ba: 05 06 02 | D, DF, F (Hadoken) |

   - Stage 0 waits for the first direction; stages 1–2 advance on the next
     ones (stage 3 sets done = 0xff); stage 3 holds while the last direction
     is held.
   - 0x12 frames after the first direction, or on leaving the last
     direction in stage 3, the tracker resets.
4. **Busy gate** (0x403d4). The fighter is free when +9 & 0xc9 is 0, 1, 8
   or 9, or its move (+0xa) is 0 or 0xc. It then keeps facing the opponent
   (0x40418; in scene modes 0x40 / 0xc0 the held direction turns it
   instead, 0x4046c).
   - +9 & 0xc9 == 0x80 continues to the dispatch, unless +9 is exactly
     0x80.
   - Any other value goes to the tail.
   - **This explains the crouch deferral observed during walks** (crouch.md):
     the walk stream alternates +9 between 0x92 (interruptible) and 0x91
     (gated). For example, the reference shows 0x92 at 2461, 0x91 from 2463
     to 2470, and 0x92 from 2471, so a down press during the 0x91 phase
     waits.
5. **Dispatch** on the direction through the table at 0x404cc:

| Direction | Handler | No button | Notes |
| --- | --- | --- | --- |
| neutral | 0x4052c | stance (move 1); a pending turn turns | standing attacks (#9) |
| right | 0x405c6 | facing right: walk forward (2); facing left: walk back (3), or block (4) against an attacking opponent | |
| left | 0x406ee | the mirror of right | the block resets the trackers and latches first, unlike right's |
| down | 0x40812 | crouch (5); an existing crouch applies a pending turn | |
| down-right / down-left | 0x40894 / 0x4098e | crouch forward (6) or back (7), crouch-block (8) against a low or jumping attack | the kick table is tried before the facing test |
| up | 0x40a86 | neutral jump (9) | +0x34 cleared; buttons ignored |
| up-right / up-left | 0x40a98 / 0x40abe | forward jump (0xa) / back jump (0xb) by facing | +0x34 cleared |
| 3, 7, 11–15 | — | nothing (tail) | both horizontal directions held do nothing |

The walk branches keep a walk going (busy, same move) instead of
restarting it; the crouch and block branches test the move id only.

6. **Tail** (0x4050c): it clears +0x37 bit 0 and drops the latched
   strengths while an attack is in progress (+9 & 0xc0 == 0xc0).

## The moves

The moves start through 0x441b0 from Ryu's move table (0x1605c) and run on
the translated object runner (0x42ad0) like every other move. Their streams
call four routines, and the jump streams call one more:

- **0x45a7a / 0x45ab2 / 0x45a96** set the posture bits of +9 (0x91 / 0x92
  in the walk).
- **0x454ee** queues the landing sound.
- **0x452e0 (Ryu) / 0x453b6 (Ken)** is the jump apex call. Without a
  buffered jump attack (+9 != 0xc1) it skips its entry; with one it starts
  the jump attack (ticket #19, docs/reference/jump-attacks.md).

**Diagonal jumps (sf-jump-02).** In the reference:

| Jump | Frames | x |
| --- | --- | --- |
| Back (up+left) | 2461–2512, 52 frames | 0x9e → 0x5a |
| Forward (up+right) | 2621–2672 | 0x5a → 0xda |

- **Up pressed during a walk** (right held from 2780, up at 2800) starts a
  forward jump at 2809, once the walk's gated phase ends. That jump carries
  Ryu past Ken, and he turns to face left at 2861.
- **Up held for 120 frames** (2950–3069) keeps re-entering the neutral jump
  (move 9 from 2951 until 3083).

The native translation matches every one of these.

## Attacks from a direction

Standing attacks with forward or back held are verified by ticket #17
(docs/reference/standing-attacks.md), crouching attacks by ticket #18
(docs/reference/crouch-attacks.md), blocks by ticket #20
(docs/reference/blocking.md), the Shoryuken and Tatsumaki by ticket #21
(docs/reference/specials.md), the Hadoken by ticket #22
(docs/reference/hadoken.md). Their strength-mapping tables are the same
(threshold, move) form as the neutral branch's:

| Table | Posture | Moves (light / medium / heavy) |
| --- | --- | --- |
| 0x406de / 0x40802 | forward / back punch | 0d / 0f / 10 |
| 0x406e6 / 0x4080a | forward / back kick | 14 / 15 / 16 |
| 0x40884 / 0x4088c | down punch / kick | 11, 1c, 1c / 17, 17, 18 |
| 0x40976 / 0x4097e | down-forward / down-back punch | 12, 1d, 1d / 13, 1e, 1e |
| 0x40986 / 0x40a7e | diagonal crouch kicks | 17 / 17 / 18 |

## Verification

The acceptance tests are in `crates/sf-engine/tests/`. Each runs the scene
natively from the checkpoint-2450 fixture and compares it with the pinned
reference capture. They also fail if any untranslated or unverified branch
is reached.

| Scenario | Observables (per frame, every checkpoint) | Owned RAM, every frame |
| --- | --- | --- |
| sf-move-01 (2450–3350) | MATCH, 901 frames | identical, 900 frames |
| sf-crouch-01 (2450–3399) | MATCH, 950 frames | identical, 949 frames |
| sf-jump-01 (2450–2999) | MATCH, 550 frames | identical, 549 frames |
| sf-jump-02 (2450–3200), new | MATCH, 751 frames; two reference runs identical | identical, 750 frames |
| sf-punch-01 (2450–3399) | MATCH, 950 frames | identical, 949 frames |

"Owned RAM" (`run::compare_versus_ram`, `VERSUS_RAM_REGIONS`) covers
Ryu's and Ken's whole entity regions, the effect slots, both player
structures (command trackers included), the sound queue, button latches,
random state, camera, top score and fg_scroll copy. Each is compared with
the reference's work-RAM dump after every frame.

The tables of #6–#8 only ever wrote Ryu's sprite words. This is the first
time his entity state has matched the reference during movement.

## Not covered

- **Unverified branches:** the Hadoken (above).
- **Demo-play input** (0x4030c).
- **Scene modes 0x40 / 0xc0:** translated (direction-driven turning, no
  trackers) but not exercised.
- **Ken's input handler:** it runs every frame with no input, which is
  exercised. His walk, crouch and jumps are not.
- **Still a stand-in in this slice:** the round-timer task's TIME ticks,
  translated since ticket #24 (docs/reference/round-timer.md); the stage
  drift and the random task run as translated tasks since ticket #14
  (docs/reference/task-scheduler.md).
- **The idle intro** (docs/reference/idle-scene.md) still used its observed
  tables in this slice; since ticket #29 it runs on the translated intro
  loop and the tables are deleted (docs/reference/round-transition.md).
