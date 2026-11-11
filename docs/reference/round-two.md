# Retsu wins round 2 natively, through the KO

This document records the boundary established by
issue #26: the fixed
fight's round 2, played natively by the one-player machine from a fixture
inside the round until Retsu knocks Ryu out. It is compared with the pinned
reference on every observable. P1 is idle, so this is the CPU opponent's
behavior against a standing player, not complete gameplay.

## Scenario and fixture

- **Capture:** sf-round2-02, the sf-cpu-01 boot path with P1 idle for the
  whole run. Checkpoints every frame over 3900–5600; two runs identical.
- **Fixture (3900):** round 2 under way.
  - The CPU has won round 1.
  - Ryu is in object slot 0xa00 and Retsu in 0x400 (the slots swapped in
    round 1; docs/reference/slot-swap.md).
  - The tasks are decoded from their stacks
    (docs/reference/task-scheduler.md).
- **Ticket's numbering:** the ticket named fight-run-a's frames
  (3949–4773). In this P1-idle boot path round 2 starts earlier, and Ryu
  is knocked out at 4568.

## What runs

Each frame runs the translated vblank and tasks:

- **The fight loop:** P1's input handler, the CPU decisions 0x40bd4 and
  0x45630, the drift, the dispatcher, effects, camera and the TIME
  digits.
- **The round timer, the random task, and the screen and watch polls.**

Retsu walks and throws his attacks 0x14, 0x18 and 0x0f, jumps forward, and
drains Ryu's health through his hit functions
(docs/reference/cpu-follow-up.md). Ryu takes his hit reactions until his
health runs out and he is knocked down (+8 = 8) at 4568.

**KO detection.** The fight loop tests both fighters' +8 when it wakes
(0x3eafa). It sees the KO at 4569 and takes the round-end path 0x3eb22.
That path is round flow (tickets #28 and #29), so the run reports it there
and stops rather than guessing.

## Verification

The acceptance test is `crates/sf-engine/tests/round2_scene.rs`.

- **Observables** (`run::run_cpu_fight_capture`): over 3901–4568, 668
  frames and 668 checkpoints, no observable difference:
  - per-frame screen hashes;
  - checkpoint pixels and video/palette RAM on every frame: sprites,
    health bars and the TIME HUD;
  - sound-latch timing: Retsu's attack sounds, the impacts and the KO.
- **KO:** a run through 4569 fails with the untranslated round-end
  address 0x3eb22, and the reference has Ryu knocked out at 4568.
- **Owned RAM:** byte-identical on every one of these frames
  (`tests/cpu_follow_up.rs`).

The fixture row (3900) is not compared: the reference takes its screen hash
from the previous frame's render.

## Not covered

- **The round result, the round transition and the match end**
  (tickets #28–#30).
- **A player who fights back** (tickets #27 and #31).
