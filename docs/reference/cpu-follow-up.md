# Retsu's follow-up decisions and attacks

This document records the boundary established by
issue #25. The CPU
opponent's follow-up decision runs at the end of every one of its move
streams. It is translated along with the stream calls and hit functions
Retsu's moves use, so the CPU opponent keeps fighting after its first
decision (ticket #12, docs/reference/cpu-decision.md).

## The follow-up decision (0x45630)

Each of the CPU's move streams ends with a call entry to 0x45630,
followed by `fd` (end the move) and, in looping streams, a `fe` goto back
to the stream's start. Translated as `fighter::Bus::cpu_follow_up`,
`cpu_react` and `cpu_attack`. The CPU structure is 0xffc664; "me" is the
CPU's fighter, "other" the player's. The decision continues the stream's
control loop at one of these places:

1. **Countdown.** While the CPU structure's countdown +0x19 runs, it counts
   down and the stream goes on 8 bytes on, at the goto that loops a walk.
2. **Player down or time out.** When the player is down (+8 == 8), or TIME
   is zero (the word at 0xff8e18 with 0xff8e1a *or*'ed into its low byte),
   it goes on 6 bytes on and ends the move.
3. **React** (0x4571a) needs three conditions:
   - the player is attacking (+9 bit 6);
   - the react roll (0xffc0d4 by random byte 3) is 0x20;
   - the player's attack reaches: the reach (word table 0x146aa by the
     player's move id − 0xd, or − 0x12 from 0x20) plus the CPU's +0x21 is
     at least the distance between the fighters' centres.

   Then a reaction record runs: from 0xffc0e4 against an airborne attack
   (player +9 & 3) or 0xffc164 against a ground attack, by random byte 4.
4. **Attack** (0x457fa): the distance is inside the CPU's range band
   (+0x16 .. +0x17). An attack record runs, from the range class's table
   (+0x18: 0xffc1e4 / 0xffc364 / 0xffc4e4), the player's posture row
   ((+9 & 7) / 2, 0x80 bytes) and random byte 5.
5. **Range shift** (0x458a2): with the range-shift roll (0xffc0c4 by random
   byte 6) at 0x40, it tries the next two range classes (their bands at
   0xffc068) for one that holds the distance, attacking with it if found.
   - **Otherwise,** or without the roll, the band is restored, the held
     range roll is requested (+0x1b = 0x80, read by 0x40bd4) and the
     move ends.
   - **The byte quirk:** the reach branch's and the roll-failure branch's
     band test compares only the distance's low byte (`cmp.b`); the
     attack branch compares the word. Both are kept.

**Records.** A record is 8 bytes: stream, +9, +0xa, and two parameter
bytes. Before one runs, the CPU turns to face the player (+0x37 = 4 if its
centre is at or right of the player's, else 2). The record's stream becomes
the CPU's stream, with +9 and +0xa as given. The two parameter bytes go to
+0x11 / +0x12; for a non-jump reaction they go to +0x12 and the countdown
+0x19 instead.

## Retsu's moves

Retsu's move table (0x16e6c) holds 13 moves. His AI tables
(docs/reference/ai-tables.md) start these streams:

| Stream | State / move | Use |
| --- | --- | --- |
| 0xc380 | 0x82 / 3 | walk, looping while the countdown runs |
| 0xc488, 0xc5a8, 0xc6e8 | 0x82 / 9, 0xa, 0xb | jumps |
| 0xc86a, 0xc8da | 0xa2 / 4, 0xa4 / 8 | standing and crouching blocks |
| 0xc9b6, 0xca46, 0xcb16, 0xcbe6, 0xccd6, 0xcdf8, 0xcf1a | 0xc2 / 0x0d, 0x0f, 0x14, 0x15, 0x24, 0x23, 0x18 | his seven attacks |

The streams call 0x45630, 0x455de (newly translated: queue the sound
command in the entry's second byte), and the posture and landing calls
translated earlier.

**His hit functions** (0x4664a, 0x466b8, 0x46726, 0x46794, 0x46802,
0x46870, 0x468de) are seven 0x6e-byte copies of one shape:

- **No score:** the CPU earns none.
- **Block test:** a player whose state +9 is 0x22 (the standing block)
  only gets the block sound 0x08.
- **Hit:** otherwise a damage entry by depth (depth 1 or not), the
  reaction, and a spark.

Their damage values run from 4 to 12. The last copy, 0x468de (stream
0xcf1a, move 0x18), has no block test at all: that attack cannot be
blocked. `fighter::hit_function` lists each with its spark template and
entries, read from the code.

## Verification

The acceptance test is `crates/sf-engine/tests/cpu_follow_up.rs`, on the new
capture **sf-round2-02**: the sf-cpu-01 boot path with P1 idle, checkpoints
every frame over 3900–5600. Its two runs are identical.

**The run.** From the round-2 fixture at 3900 (Ryu in object slot 0xa00,
Retsu in 0x400; docs/reference/slot-swap.md) the native run matches the
owned one-player RAM after every frame from 3901 to 4568: 668 frames.

- **What the RAM covers:** object RAM, both player structures, the AI
  tables, the round timer, the scheduler, the sound queue, the random
  state, the camera and scroll copies.
- **What happens:** Retsu walks, attacks with moves 0x14, 0x18 (the
  unblockable one, twice) and 0x0f, and jumps forward. Ryu's health drains
  through Retsu's hit functions, and Ryu is knocked down at 4568.
- **Where it stops:** at 4569 the fight loop's KO check reaches the
  round-end path (0x3eb22), the first untranslated branch, reported as
  such.

**Ticket's fixture frame.** The ticket named a checkpoint at 3948 from the
fight-run-a timeline. In this P1-idle boot path round 2 is already under
way by 3900, so the fixture is 3900.

**Earlier tests.** sf-round2-01's fixture (ticket #16) now matches all 200
frames of its capture, and the CPU-decision scene's follow-up call at 2434
is translated.

## Not covered

- **The reaction branch** and the reaction to an airborne attack: Ryu is
  idle in these scenarios; both are exercised by ticket #27
  (retsu-reactions.md).
- **Range-shift success:** not exercised in this run, though the hold and
  attack paths are.
- **The round end after the KO** (ticket #26).
- **Other opponents:** their extra stream routines are not translated.
  Geki uses 0x4594c, 0x4596e and 0x45a34; Adon 0x4558a; Sagat 0x45ace.
