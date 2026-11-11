# The music CPU from power-on over the whole fixed fight

This document records the boundary established by
issue #36: the translated
music CPU (music-phrase.md, music-stops.md, fm-effects.md) run from its
reset vector at clock 0 over the whole fixed fight — sf-fixed-fight-01,
5,000 frames, 83 s, from the power-on silence through the title, the
attract effects, the coin, both tunes, the stops and special commands, the
fight's effects and the end of round 2 — on the reference's latch writes,
with no RAM fixture and no fitted idle-loop phase. Every one of the run's
146,045 YM2151 writes matches the reference's in order on the exact chip
clock, and the chip output is sample-identical to the reference's hooked
stream over all 4,660,834 samples. Code: `MusicCpu::boot`,
`check::run_music_from_power_on` (and the `idle_loop` both runners share),
the `music_power_on` example, `sf-audio compare-music --power-on`.

## The boot (0x0000)

`di` 4, `im 1` 8, `ld sp,0xc800` 10, `jp 0x0070` 10; then 0x0070 clears
the RAM (`ld hl,0xc000; ld de,0xc001; ld bc,0x7ff; ld (hl),0; ldir` —
42,982 T-states for the copy), mutes every channel (0x0092: channel 7 down
to 0 through 0x009d, the four total levels to 0x7f each) and keys them off
(0x00ae through 0x00b9), stops the timers (0x00c2: register 0x14 = 0),
loads timer A (0x00ca), waits (0x00f3: three rounds of 256 `djnz`, 10,032
T-states with the call), loads timer B (0x00e2) and jumps into the idle
loop (`ei; jr`). From reset at clock 0 the first chip write — the address
of channel 7's first total level — lands at clock 43,213 and its data at
43,239, exactly where the reference logs them: the boot's T-states are
right, and MAME's Z80 starts at machine time 0.

The timers start at the boot's load writes: timer A's chain from its 0x14
= 0x1f write, timer B's from the 0x2f write with ymfm's first-period
adjustment, both then free-running — the grids the stop and effect windows
had measured (music-stops.md) are where the boot puts them. The idle
loop's phase is not fitted: the first `ei` starts at the clock the boot
code's T-states give.

## Two NMI timings settled on the way

Run from power-on, the fight showed 51 of its 146,045 writes off the exact
clock, in two patterns (every other write exact):

- **A handler 72 T-states early** (a chip write the reference made after
  the NMI the model made before it): the 68000's latch write reaches the
  Z80 at the start of MAME's 500-ns scheduler slice, up to two clocks
  before the write's own time (fight-samples.md), so an NMI arriving a
  clock before a write's instruction start is taken there in MAME; the
  model placed the NMI at `ceil(t × clock)`. It now places it at
  `floor(t × clock)` — the clock the write falls in — the best one-clock
  stand-in for the slice start without the slice anchors.
- **A handler 12 T-states late** (its first write after an NMI 250–290
  T-states before): the NMI was taken at the boundary after `ei`; the
  NMI handler's `retn` ends the `ei` shadow, so MAME accepts the pending
  timer interrupt at `jr`'s start, right after the NMI — the model only
  checked at `jr`'s end. The idle loop now checks the interrupt after an
  NMI taken behind `ei`.

With both, the fight's 146,045 writes and the stop and effect windows'
writes (music-stops.md, fm-effects.md) are all on the exact clock; the 38
output samples that had differed (behind one such handler at frame 2502)
are identical.

## Verification

`crates/sf-audio/tests/music_power_on.rs` (local data; skips without it):

| Check | Result |
| --- | --- |
| The run with the whole-fight YM2151 hook (`fight-audio-ym`, `run_audio_probe.py --hook-window 0 5000 --hook-devices ymsnd`) logs the same chip writes as `fight-audio-a` | identical taps |
| From reset at clock 0 through frame 4999 on the reference's latch: untranslated branches | none |
| The boot's first write | 0x67 = 0x7f, data at clock 43,239 |
| 146,045 YM2151 writes | equal in count, order and data; every one on the exact chip clock |
| The hooked YM2151 output, 4,660,834 samples | identical |

What the run exercises that the windows did not: the boot; the title's and
attract mode's effects 0x1c (channels 5–7, 822 ms) and the coin effect 0x0f
(channels 6–7: the slide-only event 0xff, a patch with an LFO block and the
LFO reset, a restart while it plays — all three branches fm-effects.md had
listed as reached by no window); tune 0x34 (the select screen) and 0x33
(the stage intro) in full; the stop 0x3f over 0x34 and the special command
0x27 over 0x33 (music-stops.md's windows A and B, here in sequence); the
round music 0x28 in both rounds with every effect of the fight (0x19,
0x02, 0x03, 0x01, 0x04, 0x09, 0x0b) and the KO; the round-end stop at 3669
and the round-2 restart at 3839; the effect self-ends with 0xc024 over a
live tune (0x19's and 0x02's over 0x28).

The music CPU's RAM is not compared in this run: the fixed fight's
checkpoints (14 frames) all fall inside a music-CPU handler by
music-stops.md's criterion, so none of their dumps is a state the
continuous run passes through at a frame end.

## Not covered

- **The native engine's latch** over the fight: this run uses the
  reference's latch writes; the native engine's writes (latch-timing.md,
  with its interrupt-latency band) drive the music CPU in the hit-spark
  window (fm-effects.md) and, over round 2 with both chips, in
  both-chips.md (ticket #37).
- **Opcode 0xa and effect opcode bytes beyond 5**: never reached.
- **The NMI's slice start** itself: `floor` stands in for the slice-anchored
  delivery; a 68000 write within a clock of a Z80 instruction boundary
  could still fall on the other side of it. None did in this run.
- **The fixture-based harness** (`run_music` with a RAM dump and the
  `music_phase` example's fitted phase) remains for the windowed tests;
  the power-on runner needs neither.
