# A bounded phrase of the fixed fight's music

This document records the boundary established by
issue #11: one identified
phrase of the fixed fight's round music is played by a Rust translation of
the original music-CPU program and a YM2151 model, and compared with the
pinned reference. The original sound program is not executed; ymfm
supplies the chip synthesis, as ADR-0001 allows. This is one phrase, not
the game's music.

## The phrase

Scenario `sf-music-01`: the fixed fight's boot path (1P Ryu vs CPU Retsu,
round 1, no fight inputs; the same as sf-fixed-fight-01 / sf-idle-scene-01).

- **Command**: the main CPU writes music command **0x28** (the round music)
  to the sound latch in frame 2280 (38.000102 s; the idle slice, ticket #5,
  reproduces that latch frame natively). Every other frame writes 0xff
  (idle); each write also pulses the music CPU's NMI.
- **Phrase**: frames **2280..=2359** (1.333 s): the new tune keys every
  channel off, plays a three-voice stab (channels 0-2, 2280-2297), a
  six-voice chord (channels 0-5, from 2307), and the first melody notes on
  channels 3-5. **Stopping point**: the end of frame 2359, one frame before
  the next chord (2360). The tune continues past it (not covered).
- **Initial sound state** (the fixture, end of frame 2279 = 38.0 s): the
  music CPU's RAM (0xc000-0xc7ff) from the reference checkpoint dump
  `ram_002279_music.bin` (the previous tune's channels, the last command
  0xff, both effect slots empty); the CPU idling, no interrupt in progress
  (the previous handler ended at 37.99969 s); the YM2151's state rebuilt by
  replaying the reference's register writes from power-on (see below); the
  idle loop's phase (below).

Evidence: reference captures `music-01-run-a` / `-run-b` (checkpoints
2279..=2360, byte-identical) and audio runs `music-audio-a` / `-b`
(`python tools/reference/run_audio_probe.py scenarios/sf_music_01.json
--out local/captures/music-audio-a --hook-window 2279 2362`: every YM2151
write with its machine time, MAME's YM2151 output stream over the window,
and the mixed WAV; the two runs are byte-identical). A MAME instruction
trace of the music CPU over 2278-2360 (`trace_music.log`, local-only)
located the executed code (1154 instructions in 49 ranges).

## The music CPU (`audiocpu`, sf-02.7k)

A Z80 at 3.579545 MHz (the YM2151's clock) with 2 KiB of RAM; its main loop
only idles (0x0009 `ei` / 0x000a `jr`). All work runs in the interrupt the
YM2151's timers raise (0x0038):

| Routine | Address | Role |
| --- | --- | --- |
| Interrupt dispatch | 0x0038 | status bit 0 (timer A) → 0x00ca, 0x0a9b, 0x0424, then bit 1; timer B → 0x00e2, 0x019d |
| Timer A re-arm | 0x00ca | 0x10 = 0xd6, 0x11 = 0 (856: 168 × 64 clocks = 3.0037 ms), 0x14 = 0x1f |
| Timer B re-arm | 0x00e2 | 0x12 = 0xf2 (14 × 1024 clocks = 4.004 ms), 0x14 = 0x2f |
| Chip write | 0x00fc | polls the busy flag (status bit 7), then address and data |
| Command intake | 0x019d | reads the latch (0xc800) directly; table 0x1011 → tune header; priority against the current / saved tune; 0x0228 loads eight channel stream pointers; 0x0202 mutes (0x009d) and keys off (0x00b9) every channel no effect owns |
| Effects | 0x0a9b | the two effect slots (0xc500 / 0xc580) — empty during the phrase |
| Sequencer | 0x0424 / 0x0453 | eight channels of 0x30 bytes at 0xc100: count down (0x048c), gate (0x04a7), events (0x04d2): notes and rests (0x0505 / 0x0529, folded into 0x00..0x60), ties (0x04ef), double speed (0x04ff), opcodes 0x0-0x9, 0xb-0xe (0x0861: length unit, gate %, transpose, patch 0x08c4, output bits 0x096c, repeat counters 0x0989...) |
| Durations | 0x0801 | length unit × table 0x0a2a / 0x0a32 entry; gate = duration × gate % / 255 (0x010e multiply, 0x0155 / 0x017d divide) |
| Key-on | 0x0551 | key code from 0x0a3a, operator levels (0x05ac / 0x0600 / 0x064b with tables 0x0675, 0x0701), KC, KF, key-on mask, LFO reset |
| Patch load | 0x08e4 / 0x0924 | patch 0x6b00 + 0x29 × n: LFO registers, RL/FB/CON, PMS/AMS, 20 operator registers |
| NMI | 0x0066 | copies the latch to 0xc000 |

Translated in `crates/sf-audio/src/music.rs` (`MusicCpu`).

### Timing is behavior

The interrupt handler runs with interrupts disabled. When the new tune
loads patches for several channels, a handler takes up to ~5.5 ms — longer
than timer A's period — so later expiries wait, and when timer A expires
twice during one handler its flag coalesces: **the reference processes 430
timer-A ticks in the phrase, not the 444 its period gives**. Fourteen
sequencer ticks are lost and the music runs correspondingly later. An
instant-handler translation reproduces the notes but not this timing (444
ticks, 45 extra writes in the window).

The translation therefore keeps the CPU's clock: each translated block
charges the T-states of the original instructions it stands for (standard
Z80 timings, with taken / not-taken costs on conditional branches; e.g.
one patch-register write is 280 T-states, the reference's measured 78.2 µs
spacing). Chip accesses happen at their instruction's memory cycle; the
write routine polls the chip's busy flag (ymfm: busy for 64 clocks after a
data write); the idle loop accepts the interrupt at the end of `jr` (13
T-states, IM 1), NMIs (72 T-states) are taken between blocks, and a flag
still pending after `reti` is accepted at once.

The **idle-loop phase at the fixture** (`MUSIC_FIXTURE_IDLE_PHASE` = 12:
the loop's `ei` starts 12 T-states after the fixture instant) is the only
CPU state the RAM dump cannot carry. The loop is 16 T-states long, so the
phase decides on which loop boundary each interrupt is accepted; it was
derived from the reference's write timing (the `music_phase` example).
With phase 12 acceptance lands on MAME's grid — all 2,283 writes on the
exact chip clock; with other phases every write of a handler is offset by
the grid difference, and phase 3 changes 9,476 output samples. (This slice
first recorded phase 11 with 1,828 exact writes: the indirect bus access
was then placed 5 T-states into its instruction instead of 4, a constant
the phase absorbed; ticket #33's windows showed the difference,
music-stops.md.)

## Sound-chip integration

- **Model**: ymfm (Aaron Giles, BSD-3-Clause) through the `ymfm-sys` 0.2.0
  crate (h1romas4/ymfm-sys, BSD-3-Clause; vendors the ymfm sources, built
  with the host's C++ compiler). The pinned baseline, MAME 0.289, emulates
  the YM2151 with the same library. `crates/sf-audio/src/opm.rs`.
- **Configuration** (sf.cpp): 3.579545 MHz clock; output at clock / 64 =
  **55930 Hz** (ymfm's integer rate, the rate of MAME's stream), two
  outputs; the IRQ line drives the music CPU; MAME mixes the chip at 0.6.
- **Host timers**: ymfm leaves timers to the host. Their deadlines are kept
  in chip clocks of real time (sample n is at n / 55930 s, clock
  floor(n × 3579545 / 55930)), as in MAME's glue; a re-armed timer
  continues from its previous deadline. Expiry is processed at sample
  boundaries, but its flag and the IRQ are placed at the exact deadline
  (status reads and interrupt acceptance use it). Writes made at known
  times are stamped with them.
- **Evaluation**: replaying the reference's YM2151 writes (38 s, from
  power-on) through the model reproduces MAME's YM2151 stream **bit for
  bit** over the whole hooked window (78,302 samples, frames 2279-2362).
- **Consistency with ticket #10**: both sound paths use a model of the
  reference's own device behavior — the MSM5205 decoder follows MAME's
  device model, and the YM2151 is MAME's ymfm core.

## Machine-checkable comparison

```
sf-audio compare-music --taps tools/reference/local/captures/music-audio-a/audio_taps.tsv \
    --capture tools/reference/local/captures/music-01-run-a \
    --hook-dir tools/reference/local/captures/music-audio-a
```

| Check | Result |
| --- | --- |
| Timer A ticks / timer B polls, 2280..=2359 | reference 430 / 321, native 430 / 321 |
| All 2283 YM2151 writes (timer re-arms included) | same values, same order; largest timing difference 1 output sample (18 µs); 1828 on the exact chip clock |
| The 352 sound writes (excluding timer registers) | same order, within 1 sample |
| Native output vs MAME's YM2151 stream, every sample of the phrase | **74,574 / 74,574 identical** |
| Untranslated branches reached | none |

Both reference runs give the same results. Acceptance test:
`crates/sf-audio/tests/music_phrase.rs` (repeatability, the chip-model
replay, and the native phrase; skips without local data).

## Listening comparison

```
sf-audio compare-music ... --wav native_phrase.wav     # (left+right)/2, 48 kHz
sf-audio measure --a native_phrase.wav --b tools/reference/local/captures/music-audio-a/audio.wav \
    --b-start 38.0 --seconds 1.3
```

Against MAME's own mixed recording (`-wavwrite`, 48 kHz stereo, YM2151 at
0.6, MAME's output resampler):

| Measure (1.3 s from 38.0 s) | Native | MAME WAV |
| --- | --- | --- |
| Onset / active span | 0 ms / 1300 ms | 0 ms / 1300 ms |
| RMS / peak | 0.0762 / 0.5369 | 0.0456 / 0.3102 (× 0.6 mixing gain: ratio 0.598) |
| Zero crossings | 1504 /s | 1418 /s |
| Spectral centroid | 5779 Hz | 4159 Hz |
| Normalized cross-correlation | 0.990 at 0.08 ms lag | |

Pitch and character are the chip's own output (identical samples); the
zero-crossing and centroid differences come from the export path (linear
interpolation to 48 kHz here, MAME's band-limiting resampler there), not
from the synthesis. The files are produced for listening; no listening
judgement was recorded in the session that made them — the measured
checks above stand in until one is.

## Demonstration

`cargo run -p sf-dev --release -- music` (from the crate directory) renders
the phrase from the fixture with the translated program and plays it in a
loop on the default output device (Enter stops). It was built and
smoke-run (rendered, stream opened, no errors) in the recording session.

## Disclosed limits (recorded for the remaining-work map)

- **Inputs**: the 68000 side is the reference's latch writes (the command
  at 38.000102 s and the per-frame NMIs). The native idle slice reproduces
  the command's frame; the sub-frame write time (102 µs after the frame
  start in this scene, 183 µs in the versus scenes) was not modelled when
  this was written — ticket #35 accounts it from the vblank handler's
  cycles (latch-timing.md).
- **Not translated** (reaching one is reported, never guessed): opcode
  0xa (jumps to 0x0000). The special commands (0x02a5) and opcode 0xf (the
  end of a tune, 0x09e7) are ticket #33's (music-stops.md); the effect
  start (0x0240), the effects player (0x0ab9..0x0faf) and the music's
  skipped write to an effect-owned channel (0x0a16) are ticket #34's
  (fm-effects.md); all were untranslated when this was written.
- **Translated but not exercised by this phrase**: the saved-tune paths
  (0x01e5 copy, 0x01d3, the silent tick of 0xc300 channels), commands that
  do not change the tune.
- **Loops, transitions, channels**: the tune after frame 2359 (its later
  phrases and its loop), transitions to other tunes, sound effects mixed
  with music, and channels 6 and 7 (silent in the phrase) are not
  compared (no channel 6 / 7 key-on occurs in the phrase; both are keyed
  off at its start).
- **Timing model resolution**: NMIs are taken at translated-block
  boundaries (their total cost is exact; some writes move by up to ~80
  T-states, still within the same output sample); timer expiry is
  processed at output-sample resolution with flags placed at the exact
  deadline.
- The ADPCM channel (ticket #10) is silent in this window; playing both
  chips together over a gameplay scene is not yet compared.
