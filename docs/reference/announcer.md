# The announcer's chip and voice pack (EH8, #224)

ADR 0011: the overhaul announcer plays on a third emulated MSM5205 of the board's kind, mixed beside the two the samples CPU drives. Code: `crates/sf-audio/src/announcer.rs` (chip, queue, pack), `crates/sf-video/src/announcer.rs` (the lines and their order), `crates/sf-audio/src/mix.rs` (the mix), `crates/sf-host/src/voice.rs` (loading at launch). Tests: `crates/sf-audio/tests/announcer.rs`, `crates/sf-engine/tests/announcer_calls.rs`, `crates/sf-host/tests/voice_pack.rs`.

## The chip

- 4 kHz, the board's rate per chip (the samples CPU serves each chip on every other 8 kHz interrupt). One nibble per tick; a line's start resets the chip; idle, it is held in reset (output 0).
- Routed to both speaker channels at `MSM_ROUTE_GAIN` (1.0), like the board's two, scaled by `MASTER` × `ANNOUNCER`.
- The third chip is mixed independently of the board's two chips. The seam test runs the fixed fight's KO window (frames 4740..4840) twice from power-on on the reference latch, with lines called every four frames on one run. The port writes (with their times) and both original chips' levels match at every 8 kHz sample. With `ANNOUNCER` at 0 the mix is bit-identical, and at 10 it is the board's mix plus his chip.

## The queue

One line plays at a time. Classic calls, SUPER and PERFECT have the highest priority, followed by Perfect Parry, the counters, Reversal, First Attack, Burnout and Custom Combo, and the combo count.

- Nothing playing: the line starts on the first tick at or after its frame's start.
- Higher than the line playing: it cuts it off. The cut line is not resumed.
- Equal or lower: it waits. When the chip falls free, the highest waiting line starts, the oldest first among equals.
- A waiting line older than 30 frames (2000 ticks) is dropped. A line that has waited exactly 2000 ticks still plays, but one at 2001 is dropped.
- A line the pack lacks is not called. A combo count above the pack's highest `combo_N` is therefore silent, and he never says a wrong number.

## What he is asked for

`Fight::step` turns each call-out into a line and adds any finished combo of 5 hits or more. The host plays these lines from the voice selected in **Settings > Sound > Announcer Voice**. Cycling the setting plays the menu chirp and auditions "Fight!" at the current **Announcer** volume.

In an overhaul fight with Japanese selected, the host sends classic ROM commands `0x8f` through `0x9b` to the third chip when the Japanese pack has the corresponding clip. The host sends idle to the original chips for that command and delivers the classic call ahead of same-frame call-outs. A missing classic clip plays from the ROM. Ringside, Faithful 1987 and the attract loop keep the original ROM playback. The host stops the third chip on `STOP_SAMPLES` or when leaving an overhaul fight.

## The voice pack

The bundled Ringside pack lives at `assets/sf1-port-voice` in the repository and Windows ZIP. A `sf1-port-voice` folder beside the ROM set takes precedence for local overrides. Ringside loads its Settings audition from the ROM's `0x94` sample at startup.

The optional local Japanese pack lives in `sf1-port-voice/japanese/` beside the ROM set. It has its own manifest and contains the 22 overhaul call-outs plus 13 classic lines. Each voice's pack is loaded independently.

```
sf1-port-voice/
  manifest.json
  super.vox  perfect.vox  perfect_parry.vox  counter.vox  punish_counter.vox
  crush_counter.vox  alpha_counter.vox  reversal.vox  first_attack.vox
  burnout.vox  custom_combo.vox  combo_5.vox  combo_6.vox  ...
```

- `manifest.json`: `{"format": 1, "sample_rate": 4000, "lines": {"super": {"nibbles": 2412, "crc32": "1a2b3c4d"}, ...}}`. Keys are the file stems; `combo_<N>` is for N ≥ 5, with no leading zeros.
- `<stem>.vox`: 4-bit MSM5205 ADPCM, two nibbles a byte, high nibble first (the samples ROM's order). With an odd count, the last low nibble is 0 and is not played. The file's size is `ceil(nibbles / 2)`.
- Anything else in the folder (takes, prompts, git files) is ignored.
- The pack is refused whole if the format is wrong, the rate is not 4000 Hz (`validate_adpcm`), a nibble is wider than 4 bits, a file is missing or fails its size or CRC-32, or a key is not one of his lines. The launcher logs an unusable pack. Classic calls can still play from the ROM.

`VoiceStore::write` writes a pack in this format. It is the last step for the voice pipeline (encode each chosen take with `sf_video::asset_pack::encode_adpcm`, which takes PCM at 4 kHz). The pipeline is [`voice-pipeline.md`](voice-pipeline.md) (EH9, #225).
