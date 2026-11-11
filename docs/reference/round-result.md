# The round result after a CPU win

This document records the boundary established by
issue #28: what follows a KO
when the CPU wins the round. The game-flow task's KO path and the
screen-event task's round-result event are translated in
`crates/sf-engine/src/tasks.rs`; see also task-scheduler.md.

## The KO path (game-flow task, 0x3eb22..0x3ec4e)

1. **The KO is seen.** At its next wake the fight loop sees a fighter with
   +8 == 8 and sleeps once (0x3b5aa).
2. **The fight ends** (0x3eb28): scene mode 0, the TIME HUD drawn once
   more, and TIME copied into its frozen copy 0xff8e3c. The round timer
   keeps counting; docs/reference/round-timer.md.
3. **The fighters finish their moves** (loop 0x3eb44: drift, dispatcher,
   effects, camera, sleep) until neither is busy (+9 bit 7).
4. **The winner** (0x3eb78): a fighter that is down leaves the win to its
   opponent (its +0x3c). Both down is a double KO (0x3eca2, ticket #30).
5. **The win** (0x3ebb0..0x3ebe8):
   - the winner's wins (+8) count up;
   - its round markers are drawn (0x3bed2: per win a 2 × 2 block of tiles
     0x1100.. at the HUD offset +0xc);
   - the round result is requested (scene bit 0x20, event 7, winner at
     0xff8e38);
   - the winner starts its victory pose (move 0x0c), and a human winner's
     input handler runs once.
6. **The victory pose plays out** (loop 0x3ebf0) until neither fighter is
   busy, then the task sleeps until the event bit clears (loop 0x3ec24).
7. **Then:**
   - a second win ends the match: 0xffc009 = 1 (P1), 2 (P2) or 4 (CPU),
     then 0x3ee26 (the CPU's match win, 0x3eefa, is still untranslated;
     a player's is ticket #31);
   - otherwise the round counter 0xffc00a counts up, and a third round
     played goes to 0x3ee04 (#30);
   - otherwise the next round is requested (event 6, 0x3ec4e, ticket #29).

## The round result (screen-event task, event 7: 0x3fcd4)

**CPU winner.** The text for the round (table 0x3fd7a by 0xffc00a; "ENEMY
WON …", 23 cells at attribute 0x5000) is typed by the text writer's
typewriter entry (0x3bb98, D4 = 3): each written cell is followed by a
3-frame sleep. The writer is now a resumable cursor (`TextWriter`), so the
task sleeps mid-text as the original does. Then:

1. the text is held for 120 frames (0x3b56a);
2. it is erased (0x3bd00);
3. the task waits 30 frames (0x3b552);
4. the event bit is cleared, and the task goes back to its poll.

**Human winner** (sound 0x35, the time and vitality bonus tallies): not
translated (ticket #31). Reaching it is reported.

## Verification

The acceptance test is `crates/sf-engine/tests/round_result.rs`. Both of the
fixed fight's rounds start from fixtures inside their fights, with P1 idle:

| Round | Capture, fixture | KO | Result event | Native match | Next frame |
| --- | --- | --- | --- | --- | --- |
| 1 | sf-round-end-01, 3300 | 3328 | 3362–3569 | 3301–3569 | 3570: the next round requested (0x3ec4e), as the reference starts event 6 |
| 2 | sf-round2-02, 3900 | 4568 | 4589–4796 | 3901–4796 | 4797: the CPU's match win (0x3eefa, the continue screen), as the reference sets 0xffc009 = 4 |

Over each window, both checks hold:

- **Observables:** no difference in per-frame screen hashes, checkpoint
  pixels and video/palette RAM on every frame (the typed text, the markers,
  the victory pose, the frozen HUD) or sound-latch timing.
- **Owned RAM:** byte-identical after every frame, including the scheduler
  and both tasks' memory.

The earlier round-2 tests (`tests/cpu_follow_up.rs`, `round2_scene.rs`) now
run to the same boundary.

## Not covered

- **The next round** (event 6, round initialisation, intro): ticket #29.
- **The CPU's match win, the quote and continue:** later work. A
  player's match win is verified by ticket #31 (match-win.md).
- **Time-over, draw and double KO:** verified by ticket #30
  (round-outcomes.md).
- **A human winner's result and bonus tallies:** verified by ticket #31
  (match-win.md).
- **The attract demo's KO** (0x3ce8c).
