# The punch-hit sound

This document records the boundary established by
issue #10: the native
development fight plays the light punch's **impact sound** (sound command
0x84) from the native hit event, through a Rust translation of the original
samples-CPU program and an MSM5205 ADPCM model. The original sound program
is not executed. This covers one effect and the program paths it uses; it
does not claim the game's sound is complete.

## The chosen sound and its trace

The light punch (ticket #9, [punch.md](punch.md)) produces three sound
commands: the swing 0x81, the impact 0x84, and the spark 0x01. The impact is
the hit sound. Its path from the hit to the chip:

| Step | Where | Timing in sf-punch-01 (hit 1) |
| --- | --- | --- |
| Hit judged; the reaction queues the reaction-table entry's sound byte | 68000 0x4451a → 0x445d6, entry 0x4473a + 3 = 0x84; queue 0x3b7c8 | frame 2716 (the hit frame H) |
| Vblank pops one command into the sound latch | 68000 0x3b12c, write to 0xc0001d | frame 2717, **183 µs** after the frame starts (the handler's 1406 cycles plus the interrupt latency; latch-timing.md) |
| The samples CPU's channel poll reads the latch (bit 7 set: a sample command), cuts the swing still playing on channel 0, and starts sample 0x04 | Z80 `audio2` 0x0046 → 0x008d | interrupt 362269 (45.28363 s; 0.25 ms after the latch write) |
| Chip held in reset one service, released (0x40) the next | 0x0111, 0x011e | ticks 362269, 362275 |
| 1248 bytes = 2496 nibbles, high nibble first, one per channel service | 0x012b (bank 0 at port 2 before each) | 4 kHz, 0.624 s, ending ~45.908 s |
| Each port write = one ADPCM decode | MSM5205 #0 (sf.cpp `msm_w`, VCK slave mode) | 15.6 µs after each write |

The spark command 0x01 has bit 7 clear: it belongs to the music CPU and its
YM2151 (ticket #11), not to this effect.

### The samples CPU (`audio2`: sfu-00.1h + sf-01.1k)

A Z80 with no RAM, interrupted at 8 kHz (sf.cpp: `set_periodic_int(irq0,
8000 Hz)`). Its program is a two-channel coroutine scheduler run inside the
interrupt (entry 0x0038): each interrupt swaps register sets (`ex af,af'`,
`exx`), so the channels alternate ticks (channel 0 odd, channel 1 even), and
the swapped-in channel counts down A and, at zero, jumps to its
continuation (IX / IY). Recovered behavior:

- **Poll (0x0046 / 0x0065)**: latch bit 7 clear → not for this CPU; 0xff →
  idle (I = 0xff, so a repeated command can restart); 0x80 → stop the
  channel; another command different from the last accepted one (I) starts
  if the channel table at **0x01b1** assigns it to this channel. A playing
  channel then outputs its next nibble at once; an idle one holds its chip
  in reset and polls again three services (6 ticks) later.
- **Start (0x008d..0x0128)**: bank bits from **0x01f1** (E bits 5-7); the
  sample pointer from the table at **0x0231** (commands < 0x10) or from a
  table at 0x8000 in the command range's bank (ranges 0x10-0x1b, 0x1c-0x1e,
  0x1f-0x28, 0x29-0x31, 0x32-0x39, 0x3a-0x3d, 0x3e-); the sample header is a
  big-endian byte length. Sample 0x04: pointer 0x0713 in bank 0, 1248 bytes.
- **Play (0x012b / 0x017b)**: re-select the channel's bank (port 2 is shared
  by both channels), write one nibble with bit 6 set, high nibble first.
- Power-on (0x0000): both chips reset, both channels idle, countdowns 3; the
  first interrupt runs channel 0.

Translated in `crates/sf-audio/src/samples.rs` (`SamplesCpu`), producing the
same port writes per interrupt tick.

## Sound-chip model

`crates/sf-audio/src/msm5205.rs`: the OKI MSM5205 4-bit ADPCM decoder,
following the pinned baseline's device model (MAME 0.289
`src/devices/sound/msm5205.cpp`, BSD-3-Clause, copyright Aaron Giles;
read at tag mame0289, not vendored): 49 step sizes `floor(16 * 1.1^n)`,
difference `±(s·b2 + s/2·b1 + s/4·b0 + s/8)`, index shifts
{-1,-1,-1,-1,2,4,6,8}, 12-bit signal clamped to ±2048, reset forcing signal
and step to 0, 10-bit DAC (two low bits dropped) scaled by 1/4096.
**Configuration** (sf.cpp): 384 kHz clock, `SEX_4B` (VCK slave mode, 4-bit
data), so every write decodes one nibble; two chips, mixed at gain 1.0
each. **Integration constraint**: the decode happens 15.6 µs after each
write in the device model; the native model decodes at the write's
interrupt tick (8 kHz resolution), which leaves the decoded sample values
unchanged.

## Native hit event → sound

- `sf_audio::LatchTimeline` carries the 68000 → sound boundary: one latch
  write per frame. Its time was a constant here (`LATCH_DELAY`, 183 µs,
  the observed position in the versus frames); since ticket #35 it is the
  frame start plus the vblank handler's cycles to the write, which the
  engine accounts per frame, plus a disclosed interrupt latency
  (latch-timing.md). The poll's read time was a 20 µs constant
  (`POLL_DELAY`) here; since ticket #32 it is the T-state time of the
  poll's `in`, with the pinned MAME's scheduling slice deciding what it sees
  (fight-samples.md).
- For a capture comparison the timeline is the reference's latch writes
  before the fixture frame 2450 (the samples CPU's state at the fixture —
  channel 0 is still finishing a sample — comes from replaying them from
  power-on) and the **native punch run's** per-frame latch from 2450 on.
- The development launcher (`cargo run -p sf-dev --release`) plays the
  translated samples CPU live (cpal 0.15.3, default output device), fed by
  the native engine's latch each frame. In this slice it started from
  power-on at the fixture, so the reference's FIGHT! voice tail was not
  heard there, and it did not play the YM2151. Since ticket #37 it plays
  both sound programs (`sf_audio::mix::GameSound`), pre-rolled from
  power-on to the fixture on the reference's latch writes, so the tail and
  the round music are heard (launcher.md, both-chips.md).

## Verification

Reference audio runs: `python tools/reference/run_audio_probe.py
tools/reference/scenarios/sf_punch_01.json --out
tools/reference/local/captures/hit-sound-run-a` (and `-run-b`; plus
`--no-wav --hook-window 2714 2760` for `hit-sound-hook`, the per-device
output streams). Two runs are byte-identical (tap logs and WAVs).

| Check | Result |
| --- | --- |
| Translated samples CPU from power-on with the reference's latch writes vs every reference samples-CPU port write over the whole 56.7 s run | MATCH, 198,753 writes (`sf-audio compare-writes --taps …/hit-sound-run-a/audio_taps.tsv`) |
| Native punch run's latch from 2450 vs the reference's writes, frames 2450..3399 | MATCH, 68,968 writes (`… --native-capture <native punch capture> --native-from 2450 --from-frame 2450 --to-frame 3399`) |
| Native MSM5205 DAC outputs vs the reference's hooked device streams, both chips, every channel tick, 45.22-45.99 s (swing tail + impact) | MATCH, 6238 channel ticks, 2389 non-silent (`sf-audio compare-dac --hook-dir …/hit-sound-hook …`) |
| Impact start / length | reset at tick 362269, release 6 ticks later, 2496 nibbles from 4 ticks after the release |

Acceptance test: `crates/sf-audio/tests/hit_sound.rs` (all four checks;
skips without local data). Unit tests cover the decoder.

### Listening comparison (repeatable)

```
sf-audio render --taps tools/reference/local/captures/hit-sound-run-a/audio_taps.tsv \
    --native-capture <native punch capture> --native-from 2450 \
    --from-frame 2713 --to-frame 2760 --out native_msm.wav
sf-audio hook-wav --hook-dir tools/reference/local/captures/hit-sound-hook --out reference_msm.wav
sf-audio measure --a native_msm.wav --a-start 0.003375 --b reference_msm.wav --seconds 0.75
```

| Measure (0.75 s from 45.22 s) | Native | Reference (MAME MSM5205 streams) |
| --- | --- | --- |
| Onset / active duration | 20 ms / 490 ms | 20 ms / 490 ms |
| RMS / peak | 0.0272 / 0.1679 | 0.0270 / 0.1679 |
| Zero crossings | 1427 /s | 1427 /s |
| Spectral centroid | 5713 Hz | 5049 Hz |
| Normalized cross-correlation | 0.996 at 0.08 ms lag | |

Pitch (zero-crossing rate), duration, onset, and level agree; the centroid
difference comes from the renderers, not the decoded samples (the native
WAV holds each 8 kHz tick for 6 output samples; the reference WAV averages
the 384 kHz device stream, which also places each decode 15.6 µs-140 µs
after its tick). MAME's full mixed WAV over the same window is dominated by
the stage music (RMS 0.150 vs the impact's 0.024), so a full-mix listening
comparison waits for the YM2151 (ticket #11).

The live launcher output was built and smoke-run (stream opened, no
errors) but not listened to in the recording session.

## Not covered (recorded for the remaining-work map)

- Channel 1: in this run it only idles (no command assigns to it); its code
  path is the same translation with port 1 / even ticks, unexercised.
- Banks 1-6 and the command ranges above 0x1b: translated, unexercised
  (this run plays samples 0x01, 0x04, 0x0f, 0x14, 0x17 and the 0x80 stop,
  all in bank 0, through both the fixed and the banked pointer tables).
- The YM2151 side (music CPU, ticket #11), including the spark command
  0x01.
- Sub-tick timing of writes and decodes (the 68000's and Z80's cycle
  timing), modelled by the disclosed constants above (the Z80's side is
  exact since ticket #32, fight-samples.md).
