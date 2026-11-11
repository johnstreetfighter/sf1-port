# The round transition into round 2

This document records the boundary established by
issue #29. After the round
result, the game-flow and screen-event tasks are translated through to the
next round's fight: the next-round wipe, the round initialisation with its
announcement, the walk-in intro, and FIGHT!. The observed intro tables of
ticket #5 are retired. Code: `crates/sf-engine/src/tasks.rs` (see also
task-scheduler.md and round-result.md).

## The sequence

**Game-flow task** (slot 1), after the round result (round-result.md):

1. **Next round.** The round counter 0xffc00a counts up. The next-round
   wipe (scene event 6) is requested (0x422d0) and the task sleeps until it
   is done.
2. **Clear the objects** (0x3b9bc / 0x3b9c8 / 0x3b9d4): the effect objects
   and both fighter regions.
3. **Reset the stage position** (0x437d8):
   - the scroll copies are cleared;
   - fg_scroll is set from the country's table (by opponent index) + 0xc0,
     and bg_scroll from its second table;
   - the camera is set to 0xc0 and the drift position to 0.
4. **Round announcement.** Scene event 5 is requested, and the task waits.
5. **The HUD and the fighters** (0x3e78c and its one-player twins
   0x3e83c / 0x3e8ce, by game mode):
   - the HUD text blocks;
   - the scores, the top score, and the two name plates (0x3be1e /
     0x3be26);
   - full health (4 / 0xb) for both fighters;
   - their starting moves through the placement entries of the move
     start: 0x44114 / 0x44128 keep the header's position, while 0x4413c /
     0x44158 place the CPU at 0x1c0 or 0x40 with a facing (`start_move_at`);
   - the empty health bars (0x3bf22) and both fighters' round markers.
6. **Stage music** (0x42332, by country and opponent), then the input
   handlers once.
7. **The intro loop** (0x3e9bc): stage drift and dispatcher each frame while
   either fighter is in a 0x80 state (walking in).
8. **FIGHT!** (text 0x14f44, voice 0x94) is shown for a 0x1e-count wait,
   then erased. The round timer's reload is requested (0xff8e16 bit 0),
   and the round start follows (task-scheduler.md).

**Screen-event task** (slot 2):

- **Event 6** (0x3fc8a): a column wipe with tile 0x4a, 49 columns of 28
  cells from 0x170 leftwards, a 2-frame sleep per column. Then sound 0x3f.
- **Event 5** (0x3fba2), the round announcement:
  1. the text and background layers go off through the gfxctrl copy
     0xff8682 (0x3b690), and the screen is cleared to tile 0x4a (0x422ba),
     with a 6-frame sleep;
  2. the round's "ROUND n" text, its voice and its bell (table 0x3fc62)
     are shown, held for 60 frames;
  3. a column wipe with blank tiles (0x3c);
  4. the layers come back on (0x3b644), then sounds 0x27 and 0x3f, each
     followed by a 2-frame sleep.

The bus now carries the gfxctrl register (0xc0001b). The machines take its
fixture value from the work-RAM copy 0xff8682, and the fixture's sound latch
from the capture's frame record (`machine::fixture_latch`). Both registers
are write-only and absent from work RAM.

## Verification

**sf-round-transition-01** is new: the fixed fight's inputs, checkpoints
every frame over 3300–3999, two runs identical
(`crates/sf-engine/tests/round_transition.rs`). From the round-1 fixture at
3300 (P1 idle from there), the native one-player run covers all of 3301–3999:

| Frames | What happens |
| --- | --- |
| 3301–3569 | the KO and the round result (round-result.md) |
| 3570–3668 | the wipe |
| 3669–3837 | ROUND 2 |
| 3838 on | the round reset and the walk-in intro |
| 3919 | FIGHT! |
| 3949 on | the fight |

It matches the reference throughout:

- **Observables:** every one matches. That means per-frame screen hashes,
  checkpoint pixels and video/palette RAM on every frame, and sound-latch
  timing.
- **Owned RAM:** byte-identical after every frame.

**The idle intro** (ticket #5, sf-idle-scene-01, 2280–2388) now runs on the
one-player machine from its fixture. The game-flow task resumes in the
translated intro loop (0x3ea30). The scene still matches every observable
(109 frames, 109 checkpoints), and its owned RAM matches on all 108 stepped
frames. The observed per-frame tables (`idle.rs`) are deleted.

## Not covered

- **Rounds after round 2:** the third round's flow, and the three-rounds
  rule (0x3ee04).
- **The match end, quote, continue and game over.**
- **Time-over, draw and double KO:** ticket #30.
- **Two-player and P2-versus-CPU HUD paths:** translated with their game
  modes, but only one-player play is exercised.
