# The extended tune table, the score converter and the test theme

This document records the boundary established by
issue #195 (`ST6`, spec
#192, design `docs/design/2026-11-03-stages-and-music-design.md` section 6,
[ADR-0009](../adr/0009-stages-and-themes-in-extended-banks.md)): themes the
1987 game does not have, played by the translated FM driver. Code:
`crates/sf-audio/src/tunes.rs` (the table), `score.rs` (the converter),
`music.rs` (the lookup), `crates/sf-host/src/themes.rs` (who gets the table);
the committed score is `art/themes/test-theme.score`.

## The table

- **Commands `0x40..=0x7f`.** The ROM's command table (`0x1011`, two bytes a
  command) has entries for `0x00..=0x3f` only; past them its words are tune
  data read as addresses, so nothing in the 1987 game can mean those commands.
  The sweep in `crates/sf-engine/tests/extended_tunes.rs` reads the sound
  latch after every frame of the 137 committed scenarios (874,344 frames,
  129 distinct values) and of Local fights under Faithful 1987 and an overhaul
  preset, and finds none of them. Commands with bit 7 set are effects and are
  not touched.
- **Where it lives.** The music CPU's program image is the 32 KB ROM. An image
  that is longer carries the extension at `0x8000`, which the real board
  leaves unmapped; the driver reads `0x8000..0xc000` from the image and
  everything else as before. `ExtendedTunes::program` builds that image. A
  ROM-length image has no extension, so every command behaves exactly as it
  does today, and the lookup (`MusicCpu::commands`) takes the extended path
  only for a command `>= 0x40` on a longer image.
- **Layout.** 64 little-endian words from `0x8000` (one per command, 0 = no
  tune), then the tunes. A tune is the ROM's own shape: the header byte (its
  priority in the low nibble), eight little-endian stream pointers (0 = the
  channel is off), then the streams. An extended command that names no tune
  does nothing. The extension holds 16,384 bytes; the table is 128 of them.
- **Timing.** The lookup is charged the T-states of the ROM's lookup, so a
  1987 command costs what it did, and a run of 1987 commands with and without
  the table behind the ROM is identical in every write, every output sample
  and the final RAM.
- **Legacy scores use the ROM's event code after the lookup.** The priority comparison,
  the saved-tune slot (`0xc012`, `0xc300`), the win jingle's equal-priority
  replacement, the stops (`0x35`, `0x3d`, `0x3f`) and a higher tune's end
  popping the saved one back (`music-stops.md`) run on a theme exactly as on a
  1987 tune. Scores opting into the explicit `voice` form below add native
  stream controls for fine pitch, time and tone; they share these same
  priority and lifetime paths.
- **Who gets it** (ticket #199, `ST7`). The music CPU runs for the entire
  session, and the preset can change between fights. The session
  says `FrameOutput::tune_table` while an overhaul fight runs (its boxes
  included) or the local title theme is active. The faithful cabinet and
  Faithful 1987 use the original sound ROM. The launcher gives the CPU the
  table (`Audio::set_tune_table`, `GameSound::set_tune_table`) before the
  first frame that can send an extended command. The change is pushed in order
  with the frames, so the CPU has it for the command of the next frame
  whatever it is doing; it takes it where it reads a command (0x019d),
  between handlers (`MusicCpu::sync_extension`). A take-away waits while a
  current or saved tune has a stream in the extension (a stream pointer in
  `0x8000..0xc000`), so a theme that outlasts its fight is never cut by it. Leaving a fight whose stage plays an extended theme for a menu (Quit to Menu, the after-match box's other rows, an abandoned fight) queues the stop-all command, so the theme ends with the fight and the table can be taken away; the faithful cabinet is only reachable from a menu.
  A Faithful 1987 fight therefore runs the ROM alone, as a CPU that never had
  a table does (the 1987 commands play identically either way, tested).
  `sf_host::themes::load` builds the table, and gives none under Faithful
  1987.

## Character theme library

[ADR-0023](../adr/0023-character-themes-and-separate-native-score-storage.md)
adds a separate score library for character themes. The timed request carries
a theme identity from the match to the host. The host resolves it to a compiled
score before delivering the sound latch. The native YM2151 driver keeps the
score with its current and saved stream positions. It reads logical offsets
from the score while leaving the original sound ROM and RAM address map intact.

New scores can put a stable identity in the header, such as `theme ken`
with `command 0x52`. The registry in `sf_engine::theme` owns these pairs.
Existing scores without a `theme` header still load by command. Ryu keeps
`0x41`, the original test score keeps `0x40`, and Evil Ryu uses `0x51`.
A named score with a mismatched command or a duplicate command is reported.
The separate library can hold scores larger than the legacy 16 KiB window.

The legacy table remains available for the title arrangement, raw-command
callers and tooling that builds a ROM image. In matches the host uses the
selected theme identity. Missing character scores play the original stage
music, or `0x29` for a stage without a ROM theme. The logical choice and
rotation are independent of music files and Jukebox overrides.

## The music pack

Arrangements of Capcom melodies are scores in the converter's format in a
local folder, `sf1-port-music`, beside the ROM set (looked for beside each
ROM folder, as the art pack is), never in the public repository.
`sf_audio::music_pack` reads its `.score` files; `tunes::add_pack` converts
each against the player's sound ROM into the same table as the committed
themes. Each score names its own command (`0x41..=0x7f` in practice; `0x40`
is the test theme's). A score that does not convert, or whose command is
taken, is left out and logged with its file name (`music pack: file: why`);
the rest load. The pack is looked for and read only once the configured
preset is an overhaul one, so Faithful 1987 reads nothing from it.

**Menu music.** In the overhaul front end, the title screen and Main Menu
share the score on command `0x7e`. The SFII World Warrior title arrangement
uses that command in the local music pack. If the score is unavailable, the
title keeps its original sound and the menu keeps its sound effects.
Character Select plays the original SF1 country-select tune (`0x34`) from
the sound ROM and works with or without a music pack.

**A theme that is missing.** A stage whose record names an extended command
the table does not answer (no pack, a pack without the score, a score that
would not convert) plays the 1987 stage theme `0x29` (Geki's) instead, and
the log has a line naming the stage and the theme. The fight always starts.
`BackgroundBank::stage` gives the gates the substituted theme, so the
substitution is in the engine and holds without the launcher; a bank not yet
told what the table answers (`set_playable_themes`) falls back for every
extended theme, never sending a command no table answers.

## The score

A score is white-space-separated words; `;` starts a comment (`#` is a sharp).

```text
title    Test theme         ; optional
command  0x40               ; 0x40..=0x7f
priority 8                  ; 1..=15, optional (8)
unit     10                 ; 1..=255 sequencer ticks (3.0 ms) per half 1/32 note

channel 0                   ; the YM2151 channel, 0..=7
  patch 24                  ; one of the ROM's patches
  gate 210                  ; 0..=255, optional (200): how much of a note sounds
  pan left                  ; left, right or both, optional (both)
  C4:4 E4 G4:8. r:8         ; notes
  A3:4+8                    ; tied lengths: one key-on
  repeat 3 { D4:8 E4 }      ; a block played three times
  forever { C4:2 }          ; the rest of the channel, for ever
```

- **Notes** are a letter, an optional `#` or `b`, and an octave 0 to 8 (`C#0`
  to `C8`, `A4` = 440 Hz); `r` is a rest. A length (`1 2 4 8 16 32`, each with
  an optional `.`) follows a `:` and stays until the next one. `unit` sets the
  tempo for the whole tune: a 1/32 note lasts 2 units, a quarter 16, so a
  quarter lasts `16 x unit x 3.0 ms` (unit 10 is about 125 beats a minute).
  `a+b` ties lengths (up to 15) into one key-on; each part of a rest is a
  rest of its own. A dotted length inside a tie is written as two plain
  ones (`4.` as `4+8`), and `32.` cannot be tied (it would need a 1/64).
  The converter does this because a dotted event is opcode 6, which in the
  ROM (`0x08ba`) jumps past the tie counter's count-down (`0x04e2`): a tie
  leaves the counter at 1 for the next plain event to use up, and a dotted
  event that came first would be taken for a tied one and lose its key-on
  (a rest, its key-off). So a dotted event that can follow a tie (the next
  event, or the first of a block, which a jump back reaches after a tie)
  gets `0x20` before it, which clears the counter; it costs one byte.
- **A channel** sets its `patch` before its first note or block, and ends with
  `end` (the tune stops, as opcode `0xf` does) or with a `forever` block.
  Patches come from the ROM's table (`0x6b00`, `0x29` bytes each); the
  converter reads each patch's own transpose from the ROM and sets the
  channel's transpose (opcode 3) whenever a note falls outside the 31
  semitones a note byte reaches, so the score names real pitches.
- **Blocks** repeat from their first note. The driver keeps three repeat
  counters a channel (opcodes `0xc`, `0xd`, `0xe`), so repeats nest three
  deep; a `forever` block is an unconditional jump and is last. A block that
  changes the patch, gate or pan has them put back at its end, and its first
  note sets the transpose again, so every pass starts as the first did.
- **The legacy streams it writes** use only opcodes the 1987 tunes use: 0 (unit), 2
  (gate), 3 (transpose), 6 (dotted note), 7 (patch), 9 (pan), `0xc..0xe`
  (repeat, jump) and `0xf` (end), the `0x2n` tie prefix and note bytes. Every
  channel gets a pan: without opcode 9 the driver's output bits stay clear and
  the channel is silent.

### What the converter refuses

Each is an error with the line of the score and a message: a command outside
`0x40..=0x7f` or given twice; a unit outside 1..=255; a priority outside
1..=15 (0 is "no tune" to the driver's priority test); a channel outside
0..=7 or given twice; a patch past the ROM's 128 or blank in the ROM (it
would be silent); a note outside `C#0..C8`; a length the driver has no entry
for (64, 3, triplets); more than 15 tied lengths; a repeat count outside
2..=256; repeats nested four deep; a block with no notes (a pass would take
no time and the driver would never come back from it); an unclosed block, a
stray `}`, `end` or `forever` inside a block, statements after a channel's
end; a channel with neither `end` nor `forever` (the driver would read on
past its last byte); a note before a patch or before any length; a tune that
does not fit the extension's 16,384 bytes. Themes are FM only: nothing here
starts an ADPCM sample.

## Explicit voice, pitch and timing controls

An extended score can opt into `voice` timing instead of named-note lengths:

```text
command 0x41
unit 1                       ; still required; explicit spans use chip clocks
channel 0
  voice 16 4 0 native        ; ROM patch, carrier TL, modulator TL, feedback
  pan both
  forever {
    raw 0x4a 0 1400 5000     ; KC, KF, span, independent gate
    pitch 0x4a 32            ; change held pitch without another attack
    wait 4200               ; advance time, retaining any running gate
    off
    wait 1400
  }
```

- `voice PATCH CARRIER MODULATOR FEEDBACK` selects a nonblank ROM patch.
  Carrier and modulator values are additional total-level attenuation steps,
  each `0..127`; one step is 0.75 dB. Each computed operator level is clamped
  to 127. Feedback is `native`, or `0..7` overriding only the feedback bits.
  The patch's algorithm, operator ratios, envelopes and LFO settings remain
  its own. These are per-channel controls: two voices using the same ROM
  patch can have different tone and volume. Controls deliberately adjust the
  effective instrument; they do not modify the player's ROM bytes.
- `raw KC KF DURATION GATE` keys on the selected voice at raw YM2151 keycode
  `0..127` and key fraction `0..255`. `DURATION` is `1..65535` units of 256
  chip clocks, about 71.52 microseconds each. `GATE` uses the same units and
  may be `0..65535`; zero leaves the note held without an automatic key-off.
  A gate may outlast the span, allowing subsequent held-pitch changes.
- `pitch KC KF` updates pitch without retriggering. `wait DURATION` advances
  time while retaining the current note and its gate; long waits can be
  split. `off` keys off and clears the held-note state. `voice`, `pitch` and
  `off` do not advance time. A repeating block must still contain a positive
  span, so it cannot spin forever without giving the sequencer time back.
- An explicit channel chooses `voice` before its notes or blocks and uses
  `raw`/`wait` instead of the legacy note-length and `gate` notation. It can
  change patches through another `voice` statement. Existing repeat and
  `forever` blocks apply; control restoration at a block's end does not
  retrigger or reset a held note.
- Every attack reapplies the selected SF1 patch and its global LFO writes,
  then computes levels with the per-voice attenuation. Effects retain native
  ownership of their channels. A silently advancing saved theme updates its
  copied state without writing to the chip; a paused theme resumes its
  remaining spans rather than counting the interlude as elapsed music time.
- Explicit spans carry elapsed-clock credit through delayed native timer
  handlers, avoiding cumulative tempo drift. Actual chip writes still take
  CPU time, so this is not a promise of sample-identical replay. Loops must
  preserve their note/gate state; review-WAV fades are not loop data.

The converter writes these controls as `0x0a` subcommands inside the extended
table: voice `0`, raw note `1`, held pitch `2`, wait `3`, off `4`. Durations
are little-endian words. The driver recognizes these commands in
extended streams. The shared 16,384-byte limit applies to the mapped
table. Separately compiled character scores use their own address space.

`crates/sf-audio/tests/raw_tunes.rs` covers tone, fractional pitch, held
updates, gates, native saves, effect hand-back and timing under eight-voice
load. The real-front-end fixture in `crates/sf-game/tests/stage_themes.rs`
covers pack loading, live audio, invalid-score fallback and leaving a fight.
All committed phrases are original test content; Capcom arrangements stay
in the player's local pack.

## Separate character score storage

The host compiles character arrangements into a separate score library
before playback. `score::compile` accepts the same notes, explicit voice
controls and blocks as `score::convert`. The optional `theme <slug>` header
pins the source to the registry. `saved advance` allows the saved tune to
keep time silently while the new tune plays. The default is `saved pause`.

Each compiled stream uses 32-bit positions and channel-relative jumps.
The native driver keeps a reference to the exact compiled score in its
current and saved slots. Channel records retain the native voice, pitch,
gate and repeat state. The original mapped table still serves its existing
callers, including the original test and the title arrangement.

A cue supplies its compiled score with the timed latch write. The driver
can queue two requests for the same command even when the scores differ
or the latch never returns to idle. Replacing the library leaves already
queued, playing and saved scores readable. The score compiler rejects a
zero-time repeat before playback and rejects positions or jump targets that
cannot be represented in the stream format.

Audition a local score on the translated YM2151 path and measure the
compiled stream size:

```text
sf-audio render-score --score local.score --seconds 30 --out local.wav
```

The command produces a stereo 48 kHz WAV. For a solo part, pass a copy of
the score containing that channel. These renderings are local listening
artifacts. Verify the arrangement in a match after the listening review.

## The test theme

`art/themes/test-theme.score` is a short original piece with no Capcom or
other existing melody: eight bars in A minor on three channels (a tune, right
channel arpeggios, a left channel bass), looped for ever, command `0x40`, no
screen lists it. It is committed as a readable score and converted against
the player's sound ROM, so it needs no art pack and no music pack.

## Verification

- `crates/sf-audio/tests/extended_tunes.rs` (local ROM, from power-on on a
  latch timeline, as the music and sound-test suites run the driver):
  - the theme plays from the table: the key codes the driver writes, channel
    by channel, are the score's notes; bar lengths hold within 2% (the driver
    loses a sequencer tick when two come due in one handler, which runs it
    slow by about 1%); the second pass starts as the first did; the output is
    not silent;
  - the win jingle (`0x37`) replaces the theme at equal priority and ends
    with nothing handed back, and a higher tune (`0x36`) saves the theme in
    the saved slot (all three channel pointers in the extension), plays to its
    own end and pops it back, the held note of each channel keyed on again
    from the attack and the melody going on with the next note; `0x35`, `0x3d`
    and `0x3f` over a saved theme clear both slots, key off all eight
    channels and leave silence. Each is run beside the 1987 stage theme
    `0x29` (priority 8 too): the header pair `0xc011/0xc012` is equal at
    every sampled second and the driver's path counters
    (`MusicObservations`) are equal;
  - the table changes nothing for 1987 commands (writes, output and RAM
    identical with and without it); an extended command with no tune does
    nothing; the converter refuses bad scores against the real ROM's patches;
    the committed streams use 1987 opcodes only.
  - Negative control, in `reading_the_table_one_word_off_would_change_what_plays`:
    a program whose table is shifted one word plays nothing for `0x40` and
    the theme for `0x41`, so the lookup reading the word for its own
    command is what the other tests rely on.
  - `a_dotted_event_beside_a_tie_keeps_its_key_on_and_key_off` checks the
    driver's key-ons and key-offs for the tie and dot cases below.
- `crates/sf-audio/src/score.rs` and `tunes.rs` unit tests (no ROM): exact
  stream bytes for notes, dots, ties, rests, repeats, nesting and restored
  state; every refusal above with its line; the table's layout and limits.
- `crates/sf-engine/tests/extended_tunes.rs` (local ROM): the sweep above.
- `crates/sf-host/src/themes.rs` unit tests: Faithful 1987 gets the ROM
  alone, the overhaul presets the table behind it, and a ROM the themes
  cannot convert against keeps the ROM alone.

## Not covered

- ADPCM in a theme (design section 6, item 5).
- Triplets, opcodes 1, 4, 5, 8 and `0xb` (parameters the 1987 tunes use
  rarely or never) and double speed: the converter does not write them.
- Starting a theme from a stage record in a fight (`ST7`, #199) and the music
  pack for Capcom arrangements.
