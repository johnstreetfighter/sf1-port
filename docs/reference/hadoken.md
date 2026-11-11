# The Hadoken and its projectile

This document records the boundary established by
issue #22 and extended by
issue #53: the Hadoken
(move 0x1a) and its projectile — the throw, the projectile's flight, its
three collision tests and their handlers, the explosion, and the slow
motion that an unblocked hit requests (scene event 9), in the versus
fixture (`CF` at 2450) and against Retsu in one-player mode (`DF` at 2389,
with the live CPU). Code:
`crates/sf-engine/src/fighter.rs` (the projectile) and
`crates/sf-engine/src/tasks.rs` (event 9). See also specials.md for the
command tracker that fires the move.

## The throw

The Hadoken fires from a completed D, DF, F command on the forward-punch
branches (0x40670 facing right, 0x40798 facing left; specials.md). Its
stream calls 0x4548c at the throw frame:

1. sound 7 is queued;
2. the projectile template 0x186dc is spawned (0x42e1e) in the first free
   effect slot (0x4304c: 12 slots from 0xffe100, 0x40 apart), with the
   thrower's player structure at +0x20 and the target's at +0x24, the
   thrower's flip (attr bit 0x100: x offset and speed negated), and its
   position at the thrower's attack-box centre less half the sprite.

The template: code 0x1e8, attr 0x404, offsets (−0x10, +0x30), 4 px per
frame (+0xb), half-sizes 0xc / 0xc (+0xc / +0xd), all three collision tests
enabled (+0xe = 0xe0), sound 7, stream 0x19034 (two one-frame entries,
half-sizes 8 then 0xc, looping through a 0xfe goto), and the handlers
0x433ba (+0x14, against the fighter), 0x43490 (+0x18, against an attack
box) and 0x43476 (+0x1c, against another projectile).

The sibling stream call 0x454ae spawns template 0x1871c silently; its
only handler (0x434d6) ends the effect. It is translated but no scenario
reaches it.

**A note on the command's timing.** Pressing forward starts the walk, and
the walk accepts the punch only on its interruptible frames (+9 = 0x92:
two per ten-frame cycle in the captures, the other eight 0x91 are gated at
0x403d4). The scenarios hold forward through the button's release and the
next such frame; a forward released before it gives a standing punch.

## The flight and the collisions

The effect runner (0x42d46) moves the projectile by its stream, frees it
when its centre leaves 0x10..0xe8 vertically or 0x60..0x1a0 horizontally
(0x43268), and runs the three tests every frame, the first-frame path
included:

- **Against the target fighter** (0x430b6, +0xe bit 0x80): the
  projectile's centre (0x43234: position + 0x10 for a big sprite, else 8)
  against the target's loose box — the vertical distance to
  `centre_y − (+0x1e)` must be under `+0x1f + half-height`, the horizontal
  distance to `centre_x + (+0x20)` under `+0x21 + half-width` (the
  subtractions negated on borrow) — and the target must not be
  invulnerable (+0xf bit 3). A hit runs the handler at +0x14.
- **Against the target's attack box** (0x43144, bit 0x40, live effects
  only): the same test with +0x22..+0x25, no invulnerability check;
  handler +0x18.
- **Against another projectile** (0x431c6, bit 0x20, live effects only):
  the first later live slot with bit 0x20 (0x43072: the next 11 slots,
  wrapping 0x400 to 0x100); the centres' distances must be under the sum of
  the two half-widths, then of the two half-heights (summed as bytes);
  handler +0x1c.

## The handlers

**0x433ba — the hit.** The projectile becomes the explosion (0x42eb4:
template 0x186fc copied over it, the flip kept, the position moved so the
explosion is centred where the projectile was). The thrower's hold is cut
short (+0x33 and +0xe cleared). Then, by the target's state:

- **Blocking** (+9 bit 0x20): chip damage — entry 0x43466 (score index
  0x50, damage 8) in 2P versus, otherwise the stage's byte of 0x4340a
  (8, 8, 0x10, 0x10, 0, 0xff) added to 0x4345e, a zero byte giving no
  damage at all.
- **Otherwise**: entry 0x4345e (score index 0x50, damage 20), and the slow
  motion is requested: scene mode bit 0x20 and screen event 9.

Score, its redraw, the top-score check, the damage and the defender's
reaction then follow as for any landed hit (0x3b9f0, 0x3be6a, 0x3ba84,
0x3c034, 0x4451a).

**0x43476 — two projectiles meet.** This one becomes the explosion; the
other is freed. The throwers' holds are not cut short.

**0x43490 — a projectile meets an attack box.** Nothing in 2P versus;
otherwise the stage's byte of 0x434d0 (0, 0, 1, 1, 1, 1): a 1 makes the
projectile explode and cuts the thrower's hold short.

**The explosion** (template 0x186fc): code 0x1fcc, attr 0x404, sound 6 on
its first frame, stream 0x1904a — 0x1fce for 2 frames, 0x1fec for 3,
0x1fee for 5, then the end. No collision tests. Its palette is the attr
word's; event 9 writes no palette.

## Scene event 9: the slow motion

This resolves the conflict recorded in the next-batch proposal (the
combat survey read event 9 as the projectile explosion, the flow survey as
the KO slow motion): **event 9 is the slow motion that an unblocked
projectile hit requests**, and it uses the KO's mechanism. Its handler
(0x3ff2c) clears the request, sets 0xffc00b, sleeps 60 then 30 frames,
clears 0xffc00b, and then returns to the poll — clearing the scene bit
0x20 unless another event was requested meanwhile. While 0xffc00b is set
the fight loop's speed sleep (0x3b5aa) is 6 frames per step instead of 1.

## Verification

Three versus scenarios from the checkpoint-2450 fixture (Ryu P1 vs Ken
P2, Japan) and one one-player scenario from the checkpoint-2389 fixture
of the `sf-cpu-01` boot path (Ryu P1 vs CPU Retsu, stage 0), checkpoints
every frame, two reference runs each (`crates/sf-engine/tests/hadoken_scene.rs`,
`crates/sf-engine/tests/one_player_specials_scene.rs`):

| Scenario | Interval | What happens |
| --- | --- | --- |
| sf-hadoken-01 | 2450–2940 | LP Hadoken at 2460: thrown 2486, hits the standing Ken 2494 (11 → 7 cells), slow motion 2494–2583. HP Hadoken at 2700: thrown 2726, hits 2764 (7 → 2 cells), slow motion 2764–2853. |
| sf-hadoken-02 | 2450–2960 | Ken walks back; MP Hadoken at 2460: thrown 2486, hits the blocking Ken 2510 (chip: 11 → 10 cells, no slow motion). Ryu walks to the pin; HP Hadoken at 2760: thrown 2786, hits in its spawn frame (10 → 5 cells), slow motion 2786–2875. |
| sf-hadoken-03 | 2450–2940 | Both walk apart; Ryu's and Ken's LP Hadokens at 2530: both thrown 2556, meet 2566 (Ryu's explodes, Ken's is freed; neither hold cut short). Ken's LP Hadoken at 2700: thrown 2726, hits Ryu 2748 (11 → 7 cells), slow motion 2748–2837. |
| sf-1p-hadoken-01 | 2389–2729 | LP Hadoken at 2415: thrown 2428, flies under jumping Retsu and misses off the right edge at 2475. Ryu walks back 2468–2504; LP Hadoken at 2534: thrown 2547, overlaps Retsu's move-0x24 attack box at 2549 (reading stage 0 of `0x434d0`, byte 0) and hits Retsu unblocked at 2550 (48 → 28 HP, slow motion event 9 over 2550–2639). MP Hadoken at 2641: thrown 2654, blocked by Retsu at 2697 (reading stage 0 of `0x4340a`, byte 8 → entry `0x43466`, chip damage 8: 28 → 20 HP). |

Each matches the reference throughout:

- **Reference repeatability:** the two runs are identical (and `sf-1p-hadoken-01`
  equals `sf-cpu-01` on every field through 2389).
- **Observables:** every one matches — per-frame screen hashes, checkpoint
  pixels and video/palette RAM on every frame (the explosion's palette and
  sprites included), and sound-latch timing (sounds 7 and 6).
- **Owned RAM:** byte-identical after every frame. The compared regions now
  include the scene mode (0xffc008), the slow-motion byte (0xffc00b) and
  the screen-event request (0xff8e36).
- **Negative control (ticket #53):** mutating stage 0 of the one-player chip
  table `0x4340a` to stage 2's byte (`0x10`, damage 4 instead of 8) fails at
  frame 2697 (the first chip hit), and mutating stage 0 of the attack-box
  table `0x434d0` to stage 2's byte (`1`) fails at frame 2549 (the attack-box
  overlap).
- **Translated paths only:** no untranslated branch is reached. The
  Hadoken's unverified markers (0x40688, 0x407b0) are removed.

## Not covered

- **Stages 1–5 (`0xffc003 == 1..=5`) of the one-player chip table** (`0x4340a`)
  and **attack-box table** (`0x434d0`): stage 0 is exercised at frames 2697 and
  2549 of `sf-1p-hadoken-01`; the entries for stages 1–5 are translated but not
  reached.
- **Template 0x1871c** (stream call 0x454ae) and its handler 0x434d6:
  translated, not reached.
- **Template 0x1873c** (handler 0x43652): another projectile, not
  translated; reaching it is reported.
- **A projectile hit that knocks out**, and the forward punch's
  Hadoken-then-Tatsumaki fall-through (0x40798 → 0x407b4) firing a
  Tatsumaki: not exercised.
