# The round timer

This document records the boundary established by
issue #24: the round-timer
task (slot 0) is translated, replacing the stand-in that ticket #14 left.
It is `tasks::round_timer`, run by the scheduler
(docs/reference/task-scheduler.md).

## The task (0x3f4e2..0x3f7ac)

One pass per wake, then a one-frame sleep (0x3b51a). Its memory is at
0xff8e16–0xff8e35.

**Round timer.**

- **Reload.** With flag bit 0 (0xff8e16) set, the flag is cleared and the
  three TIME digits (0xff8e18–1a, one decimal digit per byte) are loaded
  from the reload table 0x3f544. The table is indexed by the round-time
  setting 0xff8e1c (from DSW2) and holds 100, 150, 200 and 250.
- **Tick.**
  - If TIME is zero, nothing happens. The test is the word at 0xff8e18
    with 0xff8e1a added to its low byte, the same test the fight loop
    uses.
  - Otherwise the frame counter 0xff8e1b counts 0 to 0x12. At 0x12 it
    clears and the digits count down by one (0x3bb74: decimal, borrowing
    9s leftwards). That is a tick every 19 passes.

**Training (overhaul gate, ticket #240).** In Training, under every preset,
Faithful 1987 included, the tick never runs: the reload still loads the
digits, which then stay at the round's start value, so time-over never
comes (ADR-0013). The gate keys on the play mode the engine reads
(`bus.roster.mode`); Arcade, Local, the attract and the faithful cabinet
never set it, so their timer is the ROM's.

**Screen and countdown timers.** Four counters run beside the timer, at
0xff8e1e, 0xff8e24, 0xff8e2a and 0xff8e30. Each holds two decimal words A
and B, a frame byte C and a done byte D. Flag bits 1, 2, 3 and 4–5 of
0xff8e16 start them, with C = 2 and D = 0:

- **Bit 1 (`0x02`, `0xff8e1e`):** the continue countdown (`0x0100_0000` =
  `10.00`, started at `0x3f070`; see [cpu-match-win](cpu-match-win.md)).
- **Bit 2 (`0x04`, `0xff8e24`):** the name-entry countdown (`0x0300_0000` =
  `30.00`, started at `0x424a6`).
- **Bit 3 (`0x08`, `0xff8e2a`):** the ranking display timer (`0x0003_0000` =
  `03.00`, started at `0x426f2`).
- **Bits 4–5 (`0x10` / `0x20`, `0xff8e30`):** country select (`0x0005_0000` =
  `05.00` when `0xff8e16 == 0x10`, started at `0x3e0ec` / `0x3e258`; verified
  in `sf-credit-select-01`, `sf-select-usa-01` and `sf-start-2p-01`, ticket
  #74) and the bonus stage (`0x0100_0000` = `10.00` on bit 5, started at
  `0x40dee` / `0x417de` / `0x41aae` / `0x420aa`).

Without its start flag, each pass:

- **While B is not zero:** every second pass takes 3 from B (0x3bb48).
- **Once B is zero:** every second pass moves one from A into B as 0x0909.
- **When A runs out:** it marks D = 1.

Outside their active screens, C keeps counting down every frame, as the
reference shows.

## What the fight shows

- **The HUD.** The TIME HUD is drawn by the fight loop (0x3beb4), not by
  this task.
- **After a KO.** Once a KO takes the game-flow task out of the fight loop,
  the HUD stops being redrawn but the task keeps counting the digits. The
  window below shows a tick every 19 frames straight through the round-1
  KO (3328) and the round result.
- **The "frozen copy".** The copy at 0xff8e3c that the round result counts
  from is written by the game-flow task's KO path (0x3eb22, ticket #28),
  not by this task.
- **The Freeze DIP.** The vblank spin runs no scheduler passes and does not
  count sleeping slots down, so TIME and the other task counters remain
  unchanged throughout the frozen window. Release finishes the interrupted
  vblank, runs one held vblank, and only then resumes the scheduler. See
  [freeze](freeze.md) (ticket #61).

**Tick period: 19 or about 23?** The code's period is 19 passes. Every
recorded scenario's ticks are exactly 19 frames apart: 98 ticks in
sf-blocks-01, 57 in sf-normals-04, 59 in sf-crouch-attacks-01, 35 in
sf-specials-03, 10 in sf-round2-01, and 21 in sf-round-end-01.

The roughly 23-frame figure came from a probe run whose RAM was changed to
force wins (CPU at one health cell, the timer cut short). Its cause is
inferred from the code, not verified in a capture:

- **Passes, not frames:** the task advances once per scheduler pass, not
  once per frame.
- **The overrun mechanism:** when the fight loop's work overruns a frame,
  the next vblank finds the timer task already ready (state 2). That vblank
  doesn't count it down, so the task runs once for two vblanks, and a tick
  takes longer in real frames.
- **Fixtures:** the native frame never overruns (docs/reference/task-scheduler.md,
  disclosed limits). The fixtures here run 19.

## Verification

**Inside every scene test.** The task runs under the scheduler, and its
whole memory (0xff8e16–0xff8e35) is now part of the owned RAM compared with
the reference after every frame, in the versus and one-player scenes.

- In the CPU scene this covers the reload at 2390: the fixture has the
  flag set and TIME 000, and the reference shows 100 at 2390.
- It covers the first tick at 2409, and all versus ticks from 2468 on.
- The fixed-fight DIP variants cover the other reloads at 2390: 150, 200 and
  250, in both reference and native RAM, with native play continuing through
  each match decision. See [fixed-fight-dips](fixed-fight-dips.md) (ticket
  #59).

**One frame at a time** (`crates/sf-engine/tests/round_timer.rs`). One pass
of the translated task over the reference's work RAM of frame f−1 predicts
the task's memory at frame f. Every frame of three captures matches:

| Capture | Frames | Notes |
| --- | --- | --- |
| sf-round-end-01 | 3300–3700, 400 | new: the fixed fight's round-1 KO window, checkpoints every frame; its two runs are identical |
| sf-blocks-01 | 2450–4320, 1870 | |
| sf-round2-01 | 3900–4100, 200 | |

This works because the task runs first in slot order: whatever the other
tasks wrote in frame f−1 is what it reads at f.

## Not covered

- **Countdown starts** (flag bits 1–5): bit 1 (`0xff8e1e`, continue
  countdown) is exercised in [cpu-match-win](cpu-match-win.md) (ticket #45),
  bit 3 (`0xff8e2a`, ranking display timer) in [attract](attract.md) (ticket
  #56), and bit 4 (`0x10` at `0xff8e30`, country select countdown) in
  [start-screen](start-screen.md) (ticket #74); bits 2 and 5 are exercised in
  their respective screen tickets (F29, F42).
