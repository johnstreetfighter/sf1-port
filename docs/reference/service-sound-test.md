# Every FM effect and special command from the service sound test

This document records the boundary established by
issue #62 (F22, label SM):
the reference service sound test drives every FM effect entry and special
command through the original sound programs, and the translated music CPU
reproduces the run from power-on. The service screen itself is not translated;
the reference's recorded latch writes are the input to the native music CPU.

## Scenario and reference evidence

`tools/reference/scenarios/sf_sound_test_01.json` defines
`sf-sound-test-01`: Service Mode On from power-on, 322 inputs over 8,700
frames (through frame 8699), and 118 full checkpoints. Directions are held
for ten frames per observed single step, and heavy punch plays on release.
The YM2151 hook runs from frame 106, before the first command, through the
end of the run.

The local captures are `sound-test-01-audio-a` and `sound-test-01-audio-b`
under `tools/reference/local/captures/`. Their taps, hook index and stream,
WAV, and checkpoints are byte-identical. Both read back DSW1 `0x5fff` and
DSW2 `0xffff`: only Service Mode changes from the default DSW1 `0xdfff`.
The probe applies explicit DIP settings and verifies those readbacks.

There are 108 planned music-command latch writes. Every planned code and
frame matches the actual log. The commands include every `0x00..0x27`, plus
`0x28`, `0x33`, `0x35`, `0x3d`, and `0x3f`: all 27 effect entries and the
specials over silence, all effects over tunes `0x33` and `0x28`, priority
replacement, non-empty effect stops, and `0x00` while an effect is running.
Sample commands are not part of this FM scenario.

Reproduce a capture with the pinned local MAME and ROMs:

```
python tools/reference/run_audio_probe.py \
  tools/reference/scenarios/sf_sound_test_01.json \
  --out tools/reference/local/captures/sound-test-01-audio-a \
  --hook-window 106 8699 --hook-devices ymsnd
```

`python tools/reference/local/validate_sound_plan.py` checks all 108 latch
writes against the locally generated plan and prints the three required RAM
transitions below. The plan, captures, MAME, and ROMs remain local-only.
`crates/sf-audio/tests/sound_test.rs` contains the acceptance checks and
skips when those local inputs are absent.

## Power-on equivalence

The run uses `check::run_music_from_power_on`: reset at clock 0, no RAM
fixture, and no fitted idle-loop phase. Through frame 8699:

| Check | Result |
| --- | --- |
| YM2151 writes | 255,734; equal in count, order, register, data, and exact chip clock |
| Hooked YM2151 output | 8,095,308 samples identical |
| Untranslated branches reached | none |
| Music writes/key-offs skipped through effect ownership `0xc023` | 80 |
| Zero-slide steps, including effect `0x22` | 126 |
| Full RAM `0xc000..0xc5cf` at idle checkpoints | 82 identical |
| Non-idle checkpoints | 36; full RAM not compared |

The separate replay check also reproduces every hooked sample from the
reference's logged YM2151 writes. Thus both the native CPU execution and the
chip-side write/sample scheduling are checked.

### Required effect paths

The reference checkpoints establish the required non-empty effect states;
the native masks are checked after each transition even when that checkpoint
is non-idle. Full native/reference RAM comparison remains restricted to idle
checkpoints, since a dump inside a handler need not be a state reached by the
native run's frame-end stop.

| Commands | Slot A mask `0xc500` | Slot B mask `0xc580` | Required behavior |
| --- | --- | --- | --- |
| `02@2468 -> 00@2480` | `03 -> 00` | `00 -> 00` | stop while an effect is running |
| `0f@2683 -> 10@2707` | `03 -> 01` | `00 -> 00` | replace an effect and free the changed-mask channel |
| `0f@2763 -> 0c@2807` | `81 -> 00` | `00 -> 00` | stop a non-empty effect slot |

The complete power-on comparison additionally reaches the music's
effect-owned-channel skips and effect `0x22`'s zero-slide path. The expected
unreached error paths `0x0e62` and `0x02a5` remain unreached; the run's
unrecovered-path set is empty.

### Commands without an idle checkpoint (F72)

The following complete list identifies each command with no idle checkpoint
before the next command. Their writes and hooked output are still compared;
only their full music-RAM state is not.

```
05@357, 08@507, 0a@607, 0c@707, 13@1057, 15@1157, 1a@1407,
1c@1507, 21@1757, 23@1857, 10@2707, 0f@2763, 0c@2807, 07@4387,
09@4487, 0d@4647, 10@4757, 12@4857, 19@5167, 1b@5227, 22@5457,
01@6487, 03@6587, 05@6687, 07@6787, 08@6837, 0a@6937, 0d@7047,
10@7157, 11@7207, 13@7307, 1c@7677, 20@7757, 3f@8187
```

## Stream scheduling fix

The service matrix exposed one shared replay/native scheduling error. MAME
updates a sound stream through its current sample before a device access;
the access affects the following sample. A video update at the same
quantized CPU clock can include one further sample when the video frame
boundary coincides with an FM sample boundary.

`check::mame_current_sample` now expresses that boundary with integer clock
arithmetic. `reference_opm_writes`, `replay_opm`, and the native
`TimedChip::sync` all use the same rule. This removes the output mismatch
without moving a write to a different chip clock: all 255,734 native writes
remain on the reference's exact clock.

## Negative control

The C-NEG test clones the real latch timeline, moves only the isolated
`01@157` write by 4 ms, and runs through frame 399. The exact-clock/write
comparison detects the perturbation: 4,132 of 11,969 reference writes remain
exact matches. The unmodified full-run assertions are unchanged.

## Not covered

- Native translation of the service screen or its input handling: the music
  CPU consumes the reference latch timeline in this scenario.
- MSM5205 sample commands and DAC output: this is the FM/effect matrix.
- Full music-RAM equality at the 36 non-idle checkpoints; only the required
  effect-mask transitions are checked there.
- Opcode `0x0a`, effect opcode bytes beyond 5, and the expected-unreached
  error paths `0x0e62` / `0x02a5`, none of which this run reaches.
- MAME's final band-limited WAV resampling; acceptance compares the hooked
  native-rate YM2151 stream, not a modelled WAV resampler.
