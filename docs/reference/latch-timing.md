# The sound latch's write time within the frame

This document records the boundary established by
issue #35: where in the
frame the 68000 writes the sound latch (0xc0001c/d, the command byte the
samples CPU polls at 8 kHz). The native engine now places each frame's
write at

    frame start + (interrupt latency + handler cycles) / 8 MHz

where the **handler cycles** — from the vblank handler's first instruction
to the write's bus cycle — are accounted by the translation on every frame
from the standard 68000 instruction costs of the path the handler takes,
and reproduce the pinned MAME's write times exactly on every traced frame;
and the **interrupt latency** — from the frame start to the handler's
first instruction — is the one quantity the translation cannot give, and
is disclosed as the band observed in the reference (45–138 cycles). The
latency is a scope decision, recorded at the end. Code:
`sf_engine::tasks::{vblank_prefix, vblank_suffix}` (`LatchWrite`),
`CpuMachine::latch_writes`, `Bus::vblank_buttons`, `fighter::latch_cycles`,
`MachineState::latch_cycles`, the capture's `latch_cycles` column;
`sf_audio::latch_write_time`, `IRQ_LATENCY_CYCLES`, `check::latch_residuals`.

## What the reference showed

The write's offset into the frame, from the tap logs of the reference
audio runs (fight-audio-a, hit-sound-run-a, win-01-audio), by scene:

| Frames | Run | Offset into the frame |
| --- | --- | --- |
| Title / attract (no players) | the fixed fight, ~1000 | 91–103 µs |
| The fight's intro (scene mode 2, P1's latches) | the fixed fight, ~1600 | 125–137 µs |
| 1P fight, buttons idle | the fixed fight, 2500– | 143–155 µs |
| 2P versus fight, buttons idle | the punch run, 2450– | 181–193 µs |
| A frame with a palette load | the fixed fight, 795–796; sf-win-01 | 755–769 µs |

Within one scene the offset varies by 11.6 µs from frame to frame; between
scenes it differs by what the handler does before the write. The 102 µs
versus 183 µs that the ticket asked about are the title frame and the
versus-fight frame: the versus frame runs the four button-latch routines
(both players' two button groups, 682 cycles) and the game-mode chain and
in-game copy (42 cycles) that the title frame does not — 724 cycles, 90.5 µs.

## The handler's cycles

Two reference runs were traced at the instruction level (every
program-space read of 0x3b052–0x3b4ff with its machine time, over ten
windows of the fixed fight and the punch run: 52 frames, local-only). They
showed that the write happens at the handler's first fetch plus a cost that
depends only on the path, and that every path's cost is the sum of the
standard 68000 instruction costs (branch taken 10, not taken 12; the
byte-displacement branches 10 / 8) plus 14 cycles (the handler's first
fetch precedes its first instruction by 2; the write's bus cycle is 12 into
`move.w D1,($1c,A6)`). The translation sums the same costs along its own
branch decisions:

| Block | Instructions | Cycles |
| --- | --- | --- |
| 0x3b052 entry | `movem.l D0-D7/A0-A6,-(A7)`, `addq.b #1,($2,A5)` | 144 |
| 0x3b05a scene chain | `cmpi.b` 16 + branch per step: scene 2 → 26, 1 → 54, 0xc0 → 82, 0x80 → 112, other → 110 | |
| 0x3b082 game-mode chain (0x80 / 0xc0) | mode 1 → 26, 2 → 54, 4 → 82, other → 94 | |
| A latch routine | `bsr` 18; head (`lea`, `lea`, `move.b`, `move.b`, `not.w`, `andi.w`, `beq`) 62, or 82 (P1) / 72 (P2) with the heavy button; body (0x3b3ee: `not.w`, `andi.w`, `lsr.w #4` 14 or `#6` 18, `or`, `cmpi.b`, `beq`) 46 / 50 + the branch: idle 40, first press 76, held 56 + 26 (count saturated) or 44 + 34 (stronger) or 10, release 82 + 26 (saturated) or 74 + 40 | 166 / 170 idle |
| 0x3b0b4 / 0x3b0c0 | `bra $3b0cc` (the P2-only pair falls through) | 10 |
| A coin input | `lea`, `move.w #`, `jsr` 36; 0x3b31a idle 76, pressed 84 (saturated 70), release 116 (a held-down 0x64) / 144 (short) / 166 (counted) | 112 idle |
| 0x3b0e8 system port | `move.w`, `andi.w`, `bne` | 32 |
| 0x3b0f4 palette request | `move.b`, `beq`: none 22; a load 24 + `bsr` 18 + 5294 (one copy: 128 × `move.l (A0)+,(A1,D1.w)`) | |
| 0x3b100 freeze DIP | `move.w`, `andi.w`, `beq.s` | 28 |
| 0x3b10a IN1 copy | `move.w ($2,A6),$ff868e.l` 24, `cmpi.b #-1,($3,A5)` 16, `bne`: in a game 24 (the dead `movea.l`), else 10 | 50 / 64 |
| 0x3b122 to the write | `move.b`, three `nop`, `lea`, `move.w`, `move.w (A1,D0.w),D1` 58 + 14 | 72 |

The paths the traces exercised and their costs (all exact in the traces):
title / attract 682, a coin pressed 690, in-game frames without latches
696, the fight's intro 958, 1P fight idle 1098, 2P versus idle 1406, a
held button group 1476, a release frame 1522, a palette load on a title
frame 5996. The whole-run checks below reach more.

## The interrupt latency

The handler's first fetch comes 45–138 cycles after the frame start
(exactly, with the fractional thirds of a cycle that the 133,333⅓-cycle
frame period puts on the frame start). Between frames the 68000 spins in
the task scheduler's slot scan (0x3b1ae), which masks interrupts
(`move #$2700,SR` … `move #$2000,SR`, 82 cycles per slot step) and unmasks
them for 22; an interrupt arriving in the masked stretch waits for its end,
one arriving unmasked waits for the instruction in flight, and the
exception entry takes 44 cycles — a 45–138 band, as observed. (The
derivation is from the disassembly; the band itself is the observation,
over the 52 traced frames and the 4,855 frames of the whole-run checks.)

Where in that stretch the frame start falls depends on the cycle-exact
duration of everything the frame ran before — every task's instructions —
which the translation does not account (ADR-0001: the program is
translated, not emulated cycle by cycle). So the latency is a band, not a
value: the engine supplies the handler cycles; the audio side adds a
disclosed latency (`IRQ_LATENCY_MID`, 92, the band's middle) and the checks
run the band's ends.

**What the band costs.** The samples CPU reads the latch at its 8 kHz
poll (fight-samples.md); which poll sees a command depends on the write's
time against the poll's. The band is 93 cycles, 11.6 µs, against a 125-µs
poll period: it matters only when a poll falls inside a frame's band. The
punch-hit check runs the samples CPU on the native latch at both ends of
the band and gets the same 68,968 port writes either way; a run where the
ends differ would show the frame, and only then would the latency's value
matter for the sound.

## Verification

| Check | Result |
| --- | --- |
| Instruction-level traces, 52 frames, 9 paths (`run_audio_probe.py --m68k-trace`, `vblank_paths.py --blocks`; local) | every write at the first fetch + the path's summed cost, exactly |
| Native punch run (949 frames) against hit-sound-run-a: implied latency, values | all in 45.0–138.3, values equal (`hit_sound.rs`) |
| Native CPU-scene run (44 frames) against fight-audio-a | all in 45.0–135.0, values equal (`latch_timing` example) |
| Native sf-win-01 (3,862 frames: the 1P fight with heavy punches, KO and result screens, a palette load, the match end) against win-01-audio | all in 45.0–138.3, values equal, six path costs (`latch_timing.rs`) |
| Native sf-fixed-fight-01 from checkpoint 2389 through 5000 (2,611 frames: the 1P fight with a light punch and kick, two rounds) against fight-audio-a | all in 45.0–138.3, values equal, five path costs (`latch_timing.rs`) |
| Native power-on `sf-attract-01` from frame 0 through 1647 (`1,647` latch writes `1..=1647`: reset release at frame 1, four boot palette uploads `2..=5`, warning screen `8..=793`, attract join `794`, attract title `795..=1647`) against `attract-01-hook-a` (`F15` / `#55`) | frame 1 write at `186,957` cycles (`186,275` reset-to-vblank-1 + `682` handler = `0.023369625 s`, residual `0.000`); all `1,646` frames `2..=1647` in `45.0–138.3`, values equal (`both_chips.rs`) |
| Native `sf-freeze-01` release at frame 2561 against `freeze-audio-probe` (`F21` / `#61`) | two `0xffff` writes at `42.683355375` and `42.683650375` seconds; native write-to-write interval exactly 2,360 cycles (`freeze.rs`) |
| Samples-CPU writes from the native punch latch at latency 45 and 138 | the same 68,968 writes as the reference at both |

The paths the whole runs reach beyond the traces — a first press (1442,
1134), a heavy button's first press (1154) and hold (1188), a light
button's hold (1168), releases (1214, 1522), an in-game palette load
(6010) — have their costs from the instruction table alone; the
reference's write times confirm them. (The fixed-fight scenario names its
buttons `p1_button1`…`6`; the native input schedule now reads those as the
cabinet's punches 1–3 and kicks 4–6.)

The Freeze release interval is 2 + 94 + 970 + 196 + 1,098 cycles: the
write-instruction remainder, post-write work, this frame's slot loop,
`movem`/RTE/held-exception entry, and the complete held handler through
its write. See [freeze](freeze.md) for the instruction and slot-state
breakdown. This compares the two writes within the release; it does not
assign a new value to the initial interrupt-acceptance latency.

The former constant `LATCH_DELAY` (183 µs, hit-sound.md) is gone: a native
capture without the `latch_cycles` column is refused rather than placed by
a constant.

## Not covered

- **Paths not reached by any run:** scene modes 1 and 0xc0, game mode 4,
  the coin routine's release and saturated branches, a saturated button
  hold, the system port's skip (0x3b0f0: no write that frame) and the
  palette load's repeat (0x3b378). Their costs are from the instruction
  table, not confirmed. *(Follow-up 2026-10-31, ticket
  #80 / `F40`: retires
  "game mode 4" — `sf-versus-carry-02` exercises game mode 4 across
  `5503..=8510`, including `vblank_buttons` P2 button latches in mode 4,
  with all sound-latch and latch-change observations matching the
  reference; see `docs/reference/versus-carry.md`.)*
- **A frame whose work overruns into the vblank:** the interrupt would then
  land in a task, not the scan loop, and the latency would be that
  instruction's remainder plus 44 (up to about 170 for a 15-register
  `movem.l`). No run showed one.
- **A write within one scheduler quantum (500 ns) of a poll** at a band
  end (fight-samples.md): unresolvable within the band by construction.
- **The latency's value:** modelling it means accounting the 68000's cycles
  through every task — an ADR-0001 change, not a ticket.

## Scope decision

Presented on 2026-10-22 with the ticket's three options — accept the
disclosed band, amend ADR-0001 for cycle-exact 68000 timing, or leave the
question open — the band was accepted: the handler's cycles are modelled,
the latency stays the observed band with its middle in use and both ends
checked, and no ADR change is made.
