# Every ADPCM sample on both channels and every bank

Evidence class: **SM** (controlled service-mode reference).

This document records the boundary established by
issue #64 (F24): every
samples-CPU command `0x81..0xbf` plays through its natural end, `0x80` stops
active playback on each MSM5205 channel, and both channels play together.
The translated samples CPU reproduces the complete reference run. No new
translation was required.

## Scenario and repeatable reference evidence

`tools/reference/scenarios/sf_sound_test_04.json` defines
`sf-sound-test-04`: Service Mode On from power-on, 209 inputs over 12,760
frames (through frame 12759), and 76 full checkpoints. There is a checkpoint
on the frame before every command latch and at the run's end. Both MSM5205
hooks cover frames 1376–12759, beginning one frame before the first command.

The local captures are `sound-test-04-audio-a` and `sound-test-04-audio-b`
under `tools/reference/local/captures/`. Their taps, both hook streams, WAV,
checkpoints, and sorted hook index are byte-identical. Both read back DSW1
`0x5fff` and DSW2 `0xffff`, confirming that only Service Mode changes from
the default DSW1 `0xdfff`.

All 72 planned latch writes match in command and frame: the 63 isolated
sample commands, the stop and overlap sequences, and the service-exit helper
sequence. Reproduce one capture with the pinned local MAME and ROMs:

```
python tools/reference/run_audio_probe.py \
  tools/reference/scenarios/sf_sound_test_04.json \
  --out tools/reference/local/captures/sound-test-04-audio-a \
  --hook-window 1376 12759 --hook-devices msm1,msm2
```

`python tools/reference/local/inspect_sound_test_04.py` checks the observed
latches and natural-end counts against the local decode and plan. The plan,
captures, MAME, and ROMs remain local-only.
`crates/sf-audio/tests/sound_test_04.rs` contains the acceptance checks and
skips when those local inputs are absent.

## Complete command and sample table

The local decoder reads the original channel, bank, pointer, and length
tables. `Bytes` is the length stored at the sample pointer; each byte produces
two ADPCM nibbles at 4,000 nibbles/s. The acceptance test observes exactly
`2 * Bytes` nibble writes plus the release write for every isolated command,
followed by at least ten frames before the next command.

| Command | Latch frame | Channel | Bank | Pointer | Bytes | Seconds |
| --- | ---: | ---: | ---: | --- | ---: | ---: |
| `81` | 1377 | 0 | 0 | `0251` | 1216 | 0.608 |
| `82` | 1454 | 0 | 0 | `0251` | 1216 | 0.608 |
| `83` | 1531 | 0 | 0 | `0713` | 1248 | 0.624 |
| `84` | 1609 | 0 | 0 | `0713` | 1248 | 0.624 |
| `85` | 1687 | 0 | 0 | `0bf5` | 2320 | 1.160 |
| `86` | 1797 | 0 | 0 | `1507` | 1024 | 0.512 |
| `87` | 1868 | 0 | 0 | `1909` | 3264 | 1.632 |
| `88` | 2006 | 0 | 0 | `25cb` | 2768 | 1.384 |
| `89` | 2130 | 0 | 0 | `309d` | 2048 | 1.024 |
| `8a` | 2232 | 0 | 0 | `389f` | 2304 | 1.152 |
| `8b` | 2342 | 0 | 0 | `41a1` | 2560 | 1.280 |
| `8c` | 2459 | 1 | 0 | `4ba3` | 2128 | 1.064 |
| `8d` | 2563 | 0 | 0 | `53f5` | 3257 | 1.629 |
| `8e` | 2701 | 0 | 0 | `0bf5` | 2320 | 1.160 |
| `8f` | 2811 | 0 | 0 | `60af` | 2400 | 1.200 |
| `90` | 2923 | 0 | 0 | `8018` | 3200 | 1.600 |
| `91` | 3059 | 0 | 0 | `8c9a` | 2288 | 1.144 |
| `92` | 3168 | 0 | 0 | `958c` | 1648 | 0.824 |
| `93` | 3258 | 0 | 0 | `9bfe` | 2496 | 1.248 |
| `94` | 3373 | 0 | 0 | `a5c0` | 1968 | 0.984 |
| `95` | 3473 | 0 | 0 | `ad72` | 3456 | 1.728 |
| `96` | 3617 | 0 | 0 | `baf4` | 1696 | 0.848 |
| `97` | 3708 | 0 | 0 | `c196` | 2224 | 1.112 |
| `98` | 3815 | 0 | 0 | `ca48` | 2649 | 1.325 |
| `99` | 3935 | 0 | 0 | `d4a2` | 2624 | 1.312 |
| `9a` | 4054 | 0 | 0 | `dee4` | 2672 | 1.336 |
| `9b` | 4175 | 0 | 0 | `e956` | 2640 | 1.320 |
| `9c` | 4295 | 0 | 1 | `8006` | 4272 | 2.136 |
| `9d` | 4464 | 0 | 1 | `90b8` | 14080 | 7.040 |
| `9e` | 4927 | 0 | 1 | `c7ba` | 8960 | 4.480 |
| `9f` | 5236 | 1 | 2 | `8014` | 4448 | 2.224 |
| `a0` | 5410 | 1 | 2 | `9176` | 6144 | 3.072 |
| `a1` | 5635 | 1 | 2 | `a978` | 1193 | 0.597 |
| `a2` | 5711 | 1 | 2 | `ae22` | 1936 | 0.968 |
| `a3` | 5810 | 1 | 2 | `b5b4` | 1536 | 0.768 |
| `a4` | 5897 | 1 | 2 | `bbb6` | 1696 | 0.848 |
| `a5` | 5988 | 1 | 2 | `c258` | 2976 | 1.488 |
| `a6` | 6118 | 1 | 2 | `cdfa` | 1977 | 0.989 |
| `a7` | 6218 | 1 | 2 | `d5b4` | 3584 | 1.792 |
| `a8` | 6366 | 1 | 2 | `e3b6` | 2816 | 1.408 |
| `a9` | 6491 | 1 | 3 | `8012` | 6144 | 3.072 |
| `aa` | 6716 | 1 | 3 | `9814` | 1792 | 0.896 |
| `ab` | 6810 | 1 | 3 | `9f16` | 2816 | 1.408 |
| `ac` | 6935 | 1 | 3 | `aa18` | 2304 | 1.152 |
| `ad` | 7045 | 1 | 3 | `b31a` | 5280 | 2.640 |
| `ae` | 7244 | 1 | 3 | `c7bc` | 1536 | 0.768 |
| `af` | 7331 | 1 | 3 | `cdbe` | 2976 | 1.488 |
| `b0` | 7461 | 1 | 3 | `d960` | 2233 | 1.117 |
| `b1` | 7568 | 1 | 3 | `e21a` | 5536 | 2.768 |
| `b2` | 7775 | 1 | 4 | `8010` | 2560 | 1.280 |
| `b3` | 7892 | 1 | 4 | `8a12` | 5376 | 2.688 |
| `b4` | 8094 | 1 | 4 | `9f14` | 1856 | 0.928 |
| `b5` | 8190 | 1 | 4 | `a656` | 5280 | 2.640 |
| `b6` | 8389 | 1 | 4 | `baf8` | 1792 | 0.896 |
| `b7` | 8483 | 1 | 4 | `c1fa` | 5120 | 2.560 |
| `b8` | 8677 | 1 | 4 | `d5fc` | 2304 | 1.152 |
| `b9` | 8787 | 1 | 4 | `defe` | 4864 | 2.432 |
| `ba` | 8973 | 0 | 5 | `8008` | 8448 | 4.224 |
| `bb` | 9267 | 0 | 5 | `a10a` | 4608 | 2.304 |
| `bc` | 9446 | 1 | 5 | `b30c` | 1536 | 0.768 |
| `bd` | 9533 | 0 | 5 | `b90e` | 9984 | 4.992 |
| `be` | 9873 | 0 | 6 | `8004` | 8704 | 4.352 |
| `bf` | 10175 | 0 | 6 | `8004` | 8704 | 4.352 |

There are 34 channel-0 commands and 29 channel-1 commands. Command counts
by bank are `{0: 27, 1: 3, 2: 10, 3: 9, 4: 8, 5: 4, 6: 2}`; thus the run
explicitly exercises the previously uncovered banks 1 and 3–6 as well as
banks 0 and 2.

The 63 command playbacks total **436,506 nibbles = 109.1265 seconds**.
Deduplicating identical `(bank, pointer, length)` samples gives **409,530
nibbles = 102.3825 seconds**, matching the ticket's approximately 102-second
measure. The duplicate commands are `82`, `84`, `8e`, and `bf`.

## Exact CPU, DAC, stop, and overlap comparisons

| Check | Verified result |
| --- | --- |
| Samples-CPU port writes | 1,415,151 identical in count, order, port, and data |
| Write timing | Every write compared; worst residual 0.001821 T-state |
| MSM5205 DAC output | Both chips identical at all 1,517,917 ticks in the hook window |
| Non-silent DAC coverage | 474,818 ticks |
| Untranslated samples-CPU branches | None; `samples.rs` has no report sites |
| Coverage ledger | All 223 executed `audio2` instruction PCs translated |

The active-stop and overlap paths are:

- **Channel 0:** `9d@10807 -> 80@11217`. More than 1,000 nibbles precede the
  stop, and the last active nibble is within 0.1 seconds of it. One
  already-scheduled nibble may occur after the latch and before the first
  `c0` reset; the test permits at most that one in-flight write and forbids
  every later nibble.
- **Overlap:** `9d@11567` starts channel 0 and `a0@11647` starts channel 1.
  The test finds 12,289 channel-1 nibbles with a channel-0 nibble within
  four T-states. `80@12087` then resets the shared command state.
- **Channel 1:** `a0@12527` restarts a 3.072-second sample. At frame 12580,
  Up plus heavy kick invokes the service screen's exit helper, which queues
  `27@12592`, `3f@12593`, and `80@12594`. Channel 1 is still active
  immediately before `80`; the intermediate commands are FM-only. Its first
  post-stop `c0` reset is subject to the same one-in-flight-nibble rule,
  followed by required silence. Navigating from `a0` to `80` instead would
  take 320 frames, longer than any channel-1 sample, so it cannot test an
  active stop.

## Negative control

The C-NEG test changes the high nibble of command `81`'s first ROM data byte
at `pointer + 2`. The unmodified comparison first succeeds. After the change,
`compare_dac` reports its first mismatch at the exact write for that nibble:
**tick 183615** (`t = 22.952130 s`), chip 0, reference `0.0009765625`, native
`0.001953125`. This demonstrates that the full-stream comparison detects a
single altered DAC nibble at its own tick rather than merely checking totals.

## Not covered

- Native translation of the service screen or its input handling: the
  samples CPU consumes the reference latch timeline in this scenario.
- New YM2151 acceptance for the exit helper's `27` and `3f` commands; they
  are the input-only mechanism used to reach the active channel-1 stop.
- MAME's final band-limited WAV resampling and mixer; acceptance compares the
  hooked native-rate MSM5205 DAC streams.
- Gameplay-triggered variants of these samples beyond the controlled service
  commands and the separate fight scenarios.
