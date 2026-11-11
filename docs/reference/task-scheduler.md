# The task scheduler

This document records the boundary established by
issue #14: the native
frame now runs through a translation of the original cooperative scheduler
and the tasks it runs during the fight. The versus and one-player machines'
hard-coded fight-loop order and their observed random-task phase are
retired. Translated in `crates/sf-engine/src/tasks.rs`.

## The scheduler (0x3b19a, traps 0x3b216–0x3b2be, sleep helpers 0x3b51a–0x3b5aa)

- **Slots.** 16 task slots of 0x10 bytes at 0xff8006 (A5+6):
  - +0 state: 1 new, 2 ready, 4 running, 8 sleeping
  - +1 sleep count
  - +2 SR, +4 PC, +8 USP
- **Traps.**
  - #0 (0x3b20a) creates a task.
  - #1 (0x3b230) and #3 kill one.
  - #2 (0x3b24e) sleeps the running task for D0 frames: state 8, the count,
    and its USP / SR / PC saved. A task sleeps through the helpers
    0x3b51a–0x3b57a: 1, 2, 3, 6, 12, 18, 24, 30, 44, 60, 120, 180 or 240
    frames.
- **Vblank (0x3b052).** It does all of this:
  - counts the frame byte 0xff8002;
  - runs the button latches by scene and game mode;
  - debounces both coin inputs (0x3b31a);
  - services a palette-upload request (0x3b362);
  - copies IN1 (0x3b10a) and pops one sound command into the latch
    (0x3b12c);
  - clears the frame-done flag 0xff8632, sets the vblank flag 0xff8000 =
    0x88, and counts the frame word 0xff8004;
  - counts every sleeping slot down, making it ready at zero.
- **Main loop.** It clears the flag and scans the slots in order, running
  each ready task until it sleeps again. The current slot is kept at
  0xff8001.

Translated as `tasks::vblank` and `tasks::Scheduler::run`. The slots' state
and sleep bytes and the scheduler's variables are kept in work RAM exactly
as the original keeps them; the scene tests compare them with the reference
after every frame. Two things are not reproduced: each slot's saved
SR / PC / USP words, and the register images on the tasks' stacks. The
translation keeps its continuations as Rust state machines instead.

## Where a fixture's tasks are

A sleeping slot's PC is always 0x3b254, inside the sleep wrapper. The
position in the task body is on the task's stack: above the wrapper's 60
saved bytes sit the return into the sleep helper (0x3b5a8), then the return
into the body. `Scheduler::from_fixture` decodes it, and refuses any
occupied slot whose position is not translated.

| Slot | Task | Resume points seen | Fixtures | Translation |
| --- | --- | --- | --- | --- |
| 0 | Round timer and score tallies (loop 0x3f4e2) | 0x3f7ac | all | translated by ticket #24 (docs/reference/round-timer.md); a stand-in until then |
| 1 | Game flow | 0x3ea3e (round start), 0x3eafa (fight loop); 0x3ea30 (intro loop); 0x42552, 0x42584 (name entry); 0x40dd4..0x41296 (Japan bonus stage) | 2389; 2450, 3900; 2280; 9745, 9746; 9278 | translated: the round start, the fight loop, the KO path to the round result (ticket #28: resume points 0x3eb28, 0x3eb62, 0x3ec0e, 0x3ec2a), the next round's wipe, initialisation, intro loop (0x3ea30) and FIGHT! (ticket #29), ranking name entry (ticket #82: resume points 0x42552 and 0x42584), and the Japan bonus stage and stage step (ticket #69: resume points 0x40dd4..0x41296 and 0x3efc4) |
| 2 | Screen events (0x3f7b0) | 0x3f7d6 | all | translated: the poll, the round result for a CPU win (event 7, ticket #28), the next-round wipe and the round announcement (events 6 and 5, ticket #29); other events' handlers are reported as untranslated |
| 3 | Credit / challenger watch (0x3f374) | 0x3f3a2 | all | translated for attract (`0xff8f20` states `0x3f3be / 0x3f3d8 / 0x3f402`, ticket #43) and for the game (`0xff8f24` states `0x3f41e / 0x3f454 / 0x3f47e / 0x3f4c6` with D2 from the task's saved registers, including the challenger join `0x3f4a2..0x3f4da`, ticket #81) |
| 7 | Random state and coins (0x4813e) | 0x48270, 0x4819e | all | translated: the pass, a 1-frame sleep, the four coin slots, the lockout output copy 0xff8684, the service switch and attract-start checks, the coin counters, then a 6-frame sleep. A coin arriving, the service switch, the attract start and a counter pulse are reported |

**The fight loop** (0x3ea66–0x3eb1e), one iteration per wake:

1. **On waking**, the checks:
   - a TIME of zero ends the round by time (0x3ed20);
   - a knocked-out fighter (+8 == 8) ends it by KO (0x3eb22).

   Both are round flow, reported as untranslated.
2. **The frame body:**
   - the input handlers by game mode (versus: P1 and P2; one-player: the
     human player and the CPU decision);
   - the challenger scene check;
   - the stage drift 0x438ea, the dispatcher 0x429f2, effects 0x42d22 and
     camera requests 0x4392c;
   - the TIME digits, drawn from 0xff8e18 (0x3beb4).
3. **The sleep** (0x3b5aa): one frame, or six if the Speed DIP is set to
   slow or the KO slow motion (0xffc00b) is on.

**The round start** (0x3ea3e) runs once, before the first fight-loop
iteration:

- scene mode 0x80;
- the button-latch groups cleared (0x3b6c6), and both players' latches;
- the TIME label drawn by the text-block writer (0x3bba0, block 0x14f1c).

The writer is translated for its immediate form: cells or'ed with the
attribute, 0xffff fills of 0x103c, rows 0x80 bytes apart, and the
typewriter sound word cleared.

**What it replaces.**

- The CPU machine's observed TIME HUD write at 2390 comes from the round
  start.
- The versus machine's native bg_scroll drift comes from the translated
  stage drift.
- Both machines' observed random-task phases ("passes at 2452 + 7k",
  "2383 + 7k") come from the random task's decoded position and sleep count.
  For example, the versus fixture's slot 7 sleeps 2 more frames at 0x48270,
  so it passes at 2452.

## Disclosed limits

- **Frame overruns.** The original skips the fight loop's sleep when a
  vblank interrupted the frame's work (0xff8632 cleared mid-frame), which
  is how the game slows down. The native frame never overruns, so it always
  sleeps. Modelling overruns needs the 68000's cycle timing.
- **The round-timer task** was a stand-in in this ticket. Ticket #24 has
  since translated it (docs/reference/round-timer.md).
- **Not exercised:** the hardware inputs vblank reads beyond the
  directions, buttons and Freeze DIP (coins, service, SYSTEM bit 0x80).
  Reaching one of their paths is reported. Freeze is translated only on the
  one-player machine's verified fixed-fight path (docs/reference/freeze.md).
- **The idle intro** (ticket #5's machine) still used its observed tables
  here; its fixture's game-flow task sat in the then untranslated intro
  loop. Since ticket #29 it runs on the one-player machine through the
  translated intro loop 0x3ea30 and the tables are deleted
  (docs/reference/round-transition.md).

## Verification

- **Every versus and one-player scene still matches its reference:**
  movement, crouch, jumps, punch, swap, standing, crouching and jump
  attacks, blocks, specials, and the CPU decision. The owned-RAM comparison
  now also covers the scheduler's variables (0xff8000–0xff8005), every
  slot's state and sleep bytes, the frame-done and palette flags, the coin
  slots and credits, the coin-control copy, the text-sound word, the TIME
  digits and the watch continuation. All are byte-identical after every
  frame.
- **The round-2 fixture** (sf-round2-01, 3900) still runs to the same
  boundary: 31 frames, then Retsu's follow-up decision 0x45630 (#25).
- **The Speed Slow fixed fight** (sf-fixed-fight-slow-01, 2975) matches every
  native frame through the time-over result at 5652. The fight loop sleeps six
  frames per pass while TIME keeps its 19-frame tick; changing only Speed to
  Normal first diverges at 2982 (ticket #60, docs/reference/fixed-fight-slow.md).
- **The Freeze fixed fight** (sf-freeze-01) leaves slot states, sleep counts,
  frame counters and every accepted work/object-RAM byte unchanged on frozen
  frames. Release runs the interrupted suffix and one held vblank before the
  scheduler; the full native fight matches through decision 5175 (ticket #61,
  docs/reference/freeze.md).
- **`crates/sf-engine/tests/task_scheduler.rs`:**
  - the versus (2450), round-start (2389) and round-2 (3900) fixtures decode
    to the translated positions;
  - the idle fixture (2280) decodes to slot 1 at the translated intro loop
    0x3ea30 (since ticket #29);
  - the versus fixture with slot 1's resume address set to 0x3e000 is
    refused at slot 1, 0x3e000.
