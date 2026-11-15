---
status: accepted
---

# New stages and themes are extended banks read by the game's own code

Every bonus challenger brings a stage and a theme. We decided that a new stage is tiles, maps and palettes in an extended background bank, drawn by the translated background code, and that a new theme is channel streams in an extended tune table, played by the translated FM driver. A Local stage choice reaches that code through an overhaul-only gate, never a RAM write, as ADR-0004 requires. We chose this so that scrolling, flip screen, stage drift, and the driver's cut-and-resume of the win jingle work on new content exactly as on the 1987 stages, the same reasoning ADR-0007 gave for poses. Faithful 1987 never loads either bank.

## Considered options

- Render themes to audio and play them through the Jukebox: much cheaper to compose, but the host must imitate the driver's stops, and the music only sounds like 1987.
- Draw a new backdrop over a 1987 stand-in stage with the overhaul compositor: fastest, but it duplicates scrolling, flip screen and drift.

## Consequences

The 1987 background format must be mapped far enough for a map to live past the ROM tilemap's 2048 columns, and the builder must confirm the 1987 game never scrolls or reads past it before the bank is enabled. Themes need a score-to-driver converter. The original decision kept arrangements of Capcom melodies in the local music pack. [ADR-0023's pack amendment](0023-character-themes-and-separate-native-score-storage.md#repository-music-pack-2026-11-13) records the later repository distribution decision. The design is [`docs/design/2026-11-03-stages-and-music-design.md`](../design/2026-11-03-stages-and-music-design.md).

## Amendment (2026-11-04): VS screens

The pre-fight VS screen of a two-player front-end fight is also a map in the extended background bank. The 1987 versus branch (`0x3fa00`) always scrolls to the RYU VS KEN screen, and the ROM's tilemap holds no screen for most pairs, so each pair's screen is put together from ROM screens. The left fighter's bust and name come from a screen where he stands on the left, and the right fighter's from one where he stands on the right. Ryu or Ken on his unusual side is mirrored with the map's per-cell flip bit, with the name left unflipped. Each palette is made of the ROM's own colour rows, and a bust that would share a row with the other is moved to a free one. The bank builds these maps from the ROM when the game loads, so nothing is committed or packed. An overhaul-only gate on that branch reads the roster's two fighters, as ADR-0004 requires. We chose this over drawing the busts on top in the front-end compositor for the same reason this ADR gave for stages. Tracked in #215.

## Amendment (2026-11-04): pages chosen by the gate

ST4 (`docs/reference/background-format.md`) found that the ROM tilemap fills its 2048 columns exactly, so the extended maps live on pages laid out like the ROM map: page p is another 0x40000 block, and its column c is read as column c of the ROM map is. We decided that the overhaul gate which picks a stage or a VS pair also sets the page the renderer reads. This is renderer state, not a RAM write, and the scroll registers keep their 1987 meaning. We did not use the scroll register's unused bit 15 as the page bit. That bit would let service mode's scroll test, which scrolls with no limit, reach the bank, and it would cap the bank at one page of 36 stage backgrounds (user, 2026-11-04).

## Amendment (2026-11-06): the extended tune table

ST6 (`docs/reference/extended-tunes.md`) settled what the design left to the builder. The extended tune table answers sound commands `0x40..=0x7f`, which the ROM's own table has no entries for and the 1987 game never sends (swept over every committed scenario). It sits at `0x8000`, past the ROM's 32 KB, in the music CPU's program image, which is the ROM alone under Faithful 1987 and longer, with the table behind it, in the overhaul presets. A theme is written as a readable score and converted to the driver's channel streams, with the driver's limits enforced and a message for anything it cannot play. We chose to extend the image over a lookup outside the CPU so that the priority test, the saved-tune slot, the win jingle and the stops run on a theme as on a 1987 tune, with no new code in them.

## Amendment (2026-11-06): the table is handed over per overhaul fight

ST7 (#199) found that the music CPU is one object for a whole session while the preset can change between fights, so the extended image is not set per preset at launch. It is handed to the music CPU for each fight under an overhaul preset and taken away after. Faithful 1987 fights run the ROM alone, and its music pack is never opened. The image can outlast the fight until its theme ends: a take-away waits while a current or saved tune has a stream in the extension, and leaving an overhaul fight for a menu stops the music, so a theme does not play on into the menus or the faithful cabinet. A stage whose theme the table lacks plays a 1987 stage theme and the log says so. Details are in `docs/reference/extended-tunes.md` and the design's section 12.

## Amendment (2026-11-06): the Jukebox in real fights

ST8 (#200) made the Jukebox the other source of a fight's music, which the first option above had said needs the host to imitate the driver's stops. The session tells the host when (`FrameOutput::track`: a track starts on the frame the fight latches its stage theme and stops where the driver would stop its music or the win jingle replaces it). While a track plays, the host leaves that theme's command out of the music CPU's input, on its frame and whenever it is held or sent again, so the 1987 stops, effects and the jingle still run on the real driver. A file the host has not decoded in time, cannot find or cannot play leaves the native theme; nothing is decoded on the game thread. Arcade's stage is identified from the ladder's counters in work RAM, read and never written. Faithful 1987 never uses it. Details are in the design's section 13.

## Amendment (2026-11-06): tracker modules through a vendored libopenmpt

ST9 (#201) added tracker modules to the Jukebox. libopenmpt is the player, as the design chose, and its C++ source is vendored in `crates/sf-openmpt` and compiled with the `cc` crate, so a fresh Windows clone builds with its existing MSVC toolchain and the executable carries no DLL. We chose this over the library's Windows development package (a DLL set that must sit beside every executable and a download step), over a system install found by `pkg-config` or vcpkg (neither is on a stock Windows setup), and over the existing crates.io bindings (they declare the C API and build nothing). A module is rendered once to the same decoded track as any other file, looping at the restart position libopenmpt itself reports. The Jukebox screen offers module files only in a build that has the player. A `tracker` feature of `sf-host` (default on) gates it for a machine with no C++ compiler. Details are in the design's section 14.

## Amendment (2026-11-10): fine pitch, timing and tone in extended scores

The Ryu arrangement reviewed for #267 needs fractional pitch, held-note pitch changes, independent note gates and per-voice FM tone controls. The score converter now offers an explicit `voice` form with those controls, encoded as an extension-only `0x0a` opcode family. The translated music CPU plays it through the same YM2151, priority, saved-tune, effect-ownership and stop paths. The controls and clocks live in each channel's copied record, so pauses and hand-backs retain the voice and held pitch. Durations count elapsed chip clocks, including time spent loading patches, instead of accumulating missed sequencer ticks.

This amends the earlier assumption that everything after the extended lookup uses existing ROM event code. ROM tunes and scores using the original notation retain that code and its charged timing; the new commands require an extended stream in a loaded image. Faithful 1987 still loads neither the extension nor the music pack. We chose these native stream controls over playing the listening WAV through the Jukebox, which would need separate handling of music interruption and effect ownership. At the time, the arrangement stayed in the local music pack and the repository held the converter, playback support, and original test phrases. Format and limits are in `docs/reference/extended-tunes.md`.

## Amendment (2026-11-10): individual character themes and larger score storage

The user approved [the character-theme design](../design/2026-11-10-character-themes-design.md), including individual themes for all bonus challengers and mode-dependent selection on shared stages. [ADR-0023](0023-character-themes-and-separate-native-score-storage.md) supersedes this ADR's fixed 16 KiB storage and extended-pointer range assumptions for the new library: larger separate logical score storage preserves the original sound CPU ROM/RAM map. Current and saved themes must retain their source, cursor, voice, loop, and timing state through interruptions and effect handback. The existing readable scores, Ryu's approved arrangement, native driver semantics, and Faithful isolation remain compatibility requirements. [ADR-0023's pack amendment](0023-character-themes-and-separate-native-score-storage.md#repository-music-pack-2026-11-13) also records where the approved playable scores are distributed.
