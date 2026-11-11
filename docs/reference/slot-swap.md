# The fighters' entity-slot swap

This document records the boundary established by
issue #16. The two
fighters' object regions (0xffe400 and 0xffea00, 0x600 bytes each) are not
fixed to a player. Certain move frames trade them, and the native engine now
follows the fighters through their player structures instead of assuming
P1 = 0x400.

## The swap (0x3b828..0x3b8fe)

Called on every move's first frame and after every frame load (in the
object runner, before the collision judge). Translated as
`fighter::Bus::effect_flag_spawn` / `swap_regions` / `copy_grid`.

- **Trigger.** A frame flag in entity +0xf: 0x10 swaps a fighter that is in
  region 0x400; 0x20 (or 0x30) swaps one that is in region 0xa00. Any other
  combination does nothing. The flags are frame data. Ryu's standing heavy
  kick (move 0x16) carries 0x10 on one frame and 0x20 on a later one.
  Retsu's moves set them as well: in the fixed fight's round 1 the regions
  end up swapped (P1 in 0xa00) and stay so into round 2.
- **Copy.**
  1. Clear the positions of the 24 entries of the buffer at 0xff8712
     (0x3b8ea).
  2. Clear the flags (+0xf &= 0xcf).
  3. Copy the swapping fighter's grid into the buffer.
  4. Copy the other fighter's grid into the swapping fighter's region.
  5. Copy the buffer into the other region.

  Each copy moves (rows × cols) entries of 0x40 bytes, or the first copy's
  count (word at 0xff8710) if that is larger (0x3b8bc).
- **Player structures.** After each copy into a region, the player structure
  named by that region's +0x38 has its entity words (+0xe: other / self)
  exchanged. Every later routine finds the fighters through these words.
- **Afterwards.** Clear the positions of objects 0x80 and 0xc0 (0xffe084,
  0xffe0c4).
- **Caller continues on the new slot.** The routine returns with D1 swapped,
  so the rest of the object runner's frame (separation, clamps, the
  collision judge) continues on the same fighter in its new region. The
  translation returns the swapped (me, other) pair for that reason.

The versus frame now runs the players' objects through the dispatcher
(0x429f2, pair table 0x42a3e, then 0x42a1c per player structure), as the
one-player scene already did, instead of fixed entity offsets.

## Verification

**sf-swap-01** (new scenario: versus fixture, 2450..=2950, checkpoints every
frame).

| Frame | Event |
| --- | --- |
| 2460 | Heavy kick pressed |
| 2473 | Move 0x16 starts in region 0x400 |
| 2489 | Ryu swapped into 0xa00 (P1 +0xe = 0400 0a00) |
| 2503 | Swapped back |
| 2560 | Second heavy kick; swaps at 2589 and 2603 |
| 2640, 2700, 2760 | Walk, crouch, jump |
| 2840 | Light punch; lands on Ken (his reaction from 2856) |

- The two reference runs are identical.
- The native run matches on every observable (501 frames, 501 checkpoints:
  pixels, hashes, latch).
- The owned RAM is byte-identical after every frame: both regions, both
  player structures, effects, queues, random state and camera
  (`run::compare_versus_ram`).
- The heavy kick also needed its whiff-sound stream call (0x454c6: while
  the attack frame is still active, +0xf bit 7, queue header +0x11), now
  translated.

**sf-round2-01** (new capture: the sf-cpu-01 boot path, P1 idle
throughout, checkpoints every frame over 3900..=4100).

- The two runs' observables and work dumps are identical.
- At 3900 round 2 is under way: the CPU has one round win, P1 (Ryu) is in
  region 0xa00 and Retsu in 0x400.
- `CpuMachine::from_fight_checkpoint` loads this fixture. The native run
  (`run::compare_cpu_ram`) matches the owned one-player RAM (object RAM,
  P1 and CPU structures, AI tables, sound queue, random state, camera,
  scroll copies) for 31 frames. At 3932 it reaches Retsu's follow-up
  decision 0x45630, which is not translated (#25).

**Random-task phase.** The fixture's random-task phase is taken from task
slot 7's sleep count (0xff8077). With an end-of-frame count s ≥ 2 the task
runs at frame + s; every pass in the recorded captures follows this. A
count of 1 is ambiguous and is refused. The scheduler itself is #14.

The acceptance tests are `crates/sf-engine/tests/swap_scene.rs`. All earlier
scenes still match.

## Not covered

- Swaps triggered by Retsu's moves: they occur in round 1 of the fixed
  fight, which needs P1's attacks and Retsu's follow-up decisions (#25, #27).
- The swapped layout during P1 movement: in sf-swap-01 Ryu is back in 0x400
  before he walks. Round 2's P1 is idle.
- The TIME HUD of fight-checkpoint fixtures: it is not advanced, because the
  timer task is #24. Those fixtures are compared on RAM.
