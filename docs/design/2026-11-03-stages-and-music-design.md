# Stages and music: a stage select for Local, and room for every challenger's stage and theme

- **Date:** 2026-11-03
- **Status:** approved by the user, 2026-11-03
- **Project:** groundwork that serves the front end now and the bonus roster later (project 6 of the seven set on 2026-11-02)
- **Context:** [`CONTEXT.md`](../../CONTEXT.md), [`2026-11-02-overhaul-front-end-design.md`](2026-11-02-overhaul-front-end-design.md), [`2026-11-03-bonus-roster-by-lore-design.md`](2026-11-03-bonus-roster-by-lore-design.md), [`2026-11-02-art-pipeline-design.md`](2026-11-02-art-pipeline-design.md), [ADR-0004](../adr/0004-reach-fights-by-driving-the-faithful-flow.md), [ADR-0007](../adr/0007-extended-sprite-bank.md)

## 1. Intent

The user noticed that Local has no stage choice. Today P1 picks a flag on the real 1987 country screen and the stage follows it, so P2 never has a say and most stages cannot be reached at all. The user also sees the roster growing to 40, and wants stages and music to grow with it.

The new stages are for the incoming bonus challengers, one home stage each, with a theme of their own (user, 2026-11-03). The twelve tournament fighters keep their 1987 stages. The bonus-roster design already makes "a stage, a theme" part of a challenger being fully finished; this design builds what those stages and themes plug into, and gives Local its stage choice now, with the 1987 stages alone.

## 2. Goals, non-goals, anti-goals

### Goals

1. **A stage select in Local** (and Training) in overhaul presets, holding every 1987 fight stage today and each wave's stages when that wave ships.
2. **Fair picking rules**: P1 picks the first match, the loser picks after that, with a RANDOM choice (section 4).
3. **A way to add a stage** (backdrop and theme) that runs through the game's own background and sound code, with Faithful 1987 untouched (section 5).
4. **A way to add a theme** in the native 1987 sound: YM2151 FM, with ADPCM where it fits, played by the translated driver (section 6).
5. **A local music pack** for arrangements of Capcom melodies, kept out of the public repository like the art pack.
6. **The Jukebox wired into real fights**, covering every stage and fighter (section 6a).

### Non-goals

- **Any challenger's actual stage art or theme.** Each wave's spec makes those.
- **New stages for the twelve tournament fighters.** They keep the 1987 stages.
- **Changing Arcade.** The ladder decides the stage there.
- **Stage choice online, in Dramatic Battle or in World Tour.** Those projects decide it when they start.

### Anti-goals (failure even if it works)

- **Faithful 1987 changes in any way**, including its flag screen, its stages or its music.
- **A challenger's stage or theme that feels bolted on**: drawn over a stand-in, missing the 1987 scrolling, flip screen or stage drift, or music that ignores how the 1987 driver cuts and resumes tunes.
- **A front-end RAM write to choose the stage.** ADR-0004 allows inputs and overhaul-only gates on the translated path, never RAM writes.
- **An arrangement of a Capcom melody in the public repository.**

## 3. What the 1987 game does today (found while designing)

- **A stage is a pair of scroll positions** into the ROM's one long background tilemap (2048 columns of 16 rows, `sf-video/src/busts.rs`), one for the foreground layer and one for the background layer. `stage_reset` (`0x437d8`, `sf-engine/src/tasks.rs`) reads them from two tables chosen by the starting country (`0xffc006`), indexed by the opponent index: stage `0xffc003` × 2 + fight `0xffc004`. The stage palette is chosen differently: `request_stage_palette` (`0x43748`) picks it by country and by stage `0xffc003` only, so a country's two fight stages share one palette entry. It has five call sites, including the versus intro and the continue path.
- **The country intro runs on screen in every non-demo mode.** `country_intro_start` (events 2 and 3) scrolls to the starting country's scenery, by country and stage, and loads its palette, before the fight.
- **So there are ten fight stages, one per opponent**: Retsu, Geki, Joe, Mike, Lee, Gen, Birdie, Eagle, Adon and Sagat. Ryu and Ken have no stage of their own. The two bonus stages are not fight stages.
- **Stage music** (`stage_music`, `0x42332`) is a tune command from a table chosen by the starting country, by the same opponent index. The sound program reads the command at `0x019d` and looks the tune up in table `0x1011`. The ROM has 21 tunes (`docs/reference/tune-loops.md`); some are stingers, not stage themes.
- **The flag screen cannot reach every stage.** It offers four countries (Japan, USA, China, England), not Thailand, and the country's two fights decide which of its two stages appears.
- **The Jukebox exists in part** (`Jukebox` in `sf-engine/src/session_io.rs`): an external looping track per character, then per stage, then global, falling back to native music.

## 4. The stage select (Local and Training)

These rules are the user's (2026-11-03) unless marked as a call.

1. **Where it appears.** In overhaul presets, after both fighters lock on the Local select and before the fight. Training gets it too, with the player at the controls. *My call:* Training already runs Local-style fights, and someone practising wants to choose the backdrop.
2. **Faithful 1987 keeps the flag screen**, and no stage select appears under that preset. *My call*, from the preset's promise of the untouched 1987 experience.
3. **First match: P1 picks.** One shared stage screen, P1 at the controls. The cursor starts on P2's fighter's home stage, as *SF2* versus sent you to your opponent's home.
   - *My call:* Ryu and Ken have no stage, so for them the cursor starts on a stage of their home country: Retsu's (Japan) for Ryu, Joe's (USA) for Ken.
4. **After a match, the loser picks.** `REMATCH` opens the stage screen with the loser at the controls and the cursor on the stage just played, so one press keeps it. *My call* on the mechanism: the user's rule is "Rematch keeps the stage unless the loser changes it", and this costs one press. `RESELECT` goes back through fighter select, then the stage screen under the same loser-picks rule.
   - *My call:* after a draw, whoever picked last picks again.
   - The rule starts over when players leave Local or Training for the menu: the next session opens with P1 picking (user, 2026-11-03, grilling).
   - In Training against the CPU, the cursor starts on the CPU opponent's home stage, and any stage may be picked; opponent and stage are separate choices (user, 2026-11-03, grilling).
5. **RANDOM** is a tile on the screen. It picks among the stages shown, and the pick is revealed only when the fight starts.
6. **What is listed.** The ten 1987 fight stages, then each released wave's stages. A challenger's stage appears only with `enable_bonus_challengers` on and their wave released, the same filter the select screen uses. Bonus stages are never listed.
7. **Music follows the stage.** The stage's owner's theme plays there, whoever is fighting. A player's Jukebox track for that stage or character overrides it (section 6a).
8. **No country intro.** In Local and Training under overhaul presets, an overhaul-only gate skips the 1987 country intro (`country_intro_start`, events 2 and 3), so the stage screen leads straight to the VS screen and the fight (user, 2026-11-03). The intro would otherwise show the hidden flag's country, which never matches a challenger's stage. Arcade and Faithful 1987 keep the intro.
9. **How the choice reaches the engine.** The cabinet driver still picks a flag, hidden, so the 1987 flow runs as verified. The chosen stage reaches the engine as a roster setting, and an overhaul-only gate in `stage_reset`, `request_stage_palette` (at every call site that serves the chosen fight) and `stage_music` reads the chosen stage's scroll positions, palette and tune instead of the country tables. With no stage chosen, and always in Faithful 1987, the gate is closed and the 1987 tables are read. This is the "overhaul-only gate on the translated path" ADR-0004 allows; nothing writes work RAM.

## 5. Adding a stage: an extended background bank

A new stage is data the game's own background code draws, the same idea as ADR-0007's extended sprite bank (user chose this approach, 2026-11-03).

1. **Tiles, map and palettes above the ROM's.** A new stage's 16×16 background tiles sit above the ROM's background tiles. Its foreground and background maps sit past the ROM tilemap's 2048 columns. Its palettes go beside the stage palettes. Only overhaul modes load this bank; Faithful 1987 never does and stays byte-identical.
2. **One owner, one stage.** Every challenger owns exactly one stage and every stage has exactly one owner. Two challengers may be set in the same city, never the same place. That is 38 stages once all three waves ship (user, 2026-11-03, grilling).
3. **A stage is a record**: foreground and background scroll positions into the extended map, its palette, its theme's tune command, its owner, its name for the stage screen, and a small preview for the tile (section 7). The ten 1987 stages are records too, read from the ROM tables, so the stage screen treats all stages alike.
4. **The game's own code draws it.** Scrolling, the camera, flip screen and stage drift work on a new stage exactly as on a 1987 one, because the translated code does the work.
5. **Every stage obeys the 1987 background rules** (tile size, layer count, palette limits), checked when the stage is packed, as the art pipeline checks poses.
6. **Where stage art lives.** Generated or hand-drawn backdrops are art, so they go in the art pack, kept out of the public repository. Stage records that refer to them can be committed.

The main unknown is in this section: how much of the 1987 background format, the tilemap layout across `tilerom`'s two halves and the scroll registers' range, has to be mapped before a map can live past column 2048. As ADR-0007 required for sprite codes, the builder must confirm the 1987 game never scrolls or reads past the ROM map before the bank is enabled.

## 6. Adding a theme: an extended tune table

1. **New themes are tunes the translated FM driver plays.** They live in an extended tune table, looked up for tune commands the 1987 game never sends, and loaded only in overhaul modes. The win jingle, round stops and the driver's saved-tune slot (`docs/reference/music-stops.md`) then work on a new theme exactly as on a 1987 one.
2. **A converter** turns a written score into the driver's own channel streams, so a theme is composed or arranged in a readable form, not hand-written as driver bytes. The converter rejects anything the 1987 driver cannot play.
3. **Where a theme comes from** (user, 2026-11-03):
   - a challenger with a well-known theme from their own games gets that melody arranged for the 1987 chip;
   - a challenger with none, such as Dorai, gets new music in the same style, built from their home and lore.
4. **Where it lives** (user, 2026-11-03): arrangements of Capcom melodies go in a **local music pack** beside the ROM set, never in the public repository. New compositions with no Capcom melody may be committed. A stage whose theme is missing from the pack plays a 1987 stage theme and says so in the log; it never fails to start. *My call* on the fallback.
5. **ADPCM in themes waits until a wave needs it.** The translated driver plays FM only; samples are started separately by the main CPU. Timing samples to a tune means either an overhaul-only extension of the driver or the main CPU imitating its stops, and that choice is made when a real theme needs it. Themes are FM-only until then, including the first test theme. When it comes, samples use the 1987 format, as the 2026-10-27 design requires for new sounds. *My call.*

## 6a. The Jukebox in real fights

The `Jukebox` in `session_io.rs` exists but nothing in `sf-host` or `sf-game` calls it, so no fight uses it today. This project wires it in (user, 2026-11-03):

1. **The fight's music asks the Jukebox first.** When it resolves to an external track, the host plays that track; otherwise the native theme plays.
2. **Its keys widen** from the 1987 stage byte and the twelve fighters to every stage record and every fighter, challengers included.
3. **When both fighters have a character track, the stage picker's wins**: P1's on the first match, the loser's after that. *My call*, following the stage-pick rule.
4. **Players set tracks in the game** (user, 2026-11-03, grilling). The greyed `JUKEBOX` row in Sound settings opens a Jukebox screen listing every stage and every fighter. Each is set to `NATIVE` or to an audio file from a `jukebox` folder beside the ROM set. There is no file browser, and loop points aren't edited in the game.
5. **Track formats** (user, 2026-11-03, grilling): OGG Vorbis, WAV, FLAC, and tracker modules across the wider libopenmpt list (MOD, XM, S3M, IT, MPTM, 669, MTM and the other formats it plays), so libopenmpt is the tracker player and its C++ build on Windows is part of the work. Loop points come from `LOOPSTART`/`LOOPLENGTH` tags in OGG and FLAC and from the loop chunk in WAV. A tracker module loops the way the module itself says (its restart position). A file with no loop information plays through and restarts from the top. *My call* on the tag names and the no-loop rule.
6. The host imitating the driver's stops (win jingle, round end) for an external track is the Jukebox's existing trade-off, accepted by the 2026-10-27 design; native themes are unaffected.

## 7. The stage screen's look

A framed front-end screen in the style the front-end design set: a row or grid of stage tiles, each a small preview of the stage with its owner's name, the cursor in the picker's colour (P1 yellow, P2 the compositor's blue), the preset line at the foot, and a RANDOM tile. It must hold the ten 1987 stages now and 38 stages once all three waves ship without being redesigned at each wave, the same rule the bonus-roster design sets for the select screen. The exact layout is left to the builder, to be shown to the user as a mockup before it is built.

## 8. Approaches considered

1. **Extend the game's own banks (chosen).** New stage art and new themes are data the translated background code and FM driver read. New content gets the 1987 behaviour without re-implementing it, which matches the art pipeline's choice for poses. It costs more up front: mapping the background format and writing the score converter.
2. **Same stages, music as audio files.** Stages as above, but themes rendered to audio and played through the Jukebox path. Composing is much cheaper, but the host has to imitate the driver's stop-and-resume behaviour, and the music is only 1987-sounding rather than played by the 1987 driver.
3. **Paint over a stand-in.** Run the fight on a 1987 stage and draw the new backdrop over it with the overhaul compositor, music from the Jukebox. It's the fastest to a first result, but it duplicates scrolling, flip screen and drift, the same reason ADR-0007 rejected an overlay for poses.

The user chose 1 (2026-11-03).

## 9. What this changes elsewhere

- **Front-end design (2026-11-02):** in overhaul presets, Local no longer has P1 pick a visible flag; the flag is picked hidden, the stage screen decides the stage, and the country intro is skipped, superseding "the translated flow runs visibly through the pre-fight" for Local and Training. Rematch keeps the stage by default but now passes through the stage screen. Faithful 1987 is unchanged.
- **Bonus-roster design (2026-11-03):** each wave's "stage and theme" means a stage record in the extended background bank and a theme in the extended tune table, made under sections 5 and 6.
- **2026-10-27 design, 5.5.3:** the "stage/BGM selector" is this stage screen. The Jukebox stays the BGM override.

## 10. Left to the builder

- The stage screen's exact layout (section 7), shown to the user as a mockup first.
- How the extended background bank is laid out and loaded, and how the scroll registers address it (section 5).
- Which unused tune commands the extended table uses, and the converter's score format (section 6). Settled by ST6: commands `0x40..=0x7f` and a text score, in `docs/reference/extended-tunes.md`.
- How stage previews are made: cut from the stage itself or drawn.
- Proving one test stage and one test theme end to end before any wave depends on them (user, 2026-11-03, grilling). The test stage is rebuilt from the ROM's own background tiles in a new arrangement, the way a recipe rebuilds a pose. The test theme is a short original piece with no Capcom melody. Both are committed and run in tests with only the ROM set, with no art or music pack.
- Which tune is a 1987 stage's theme, if the 1987 tables give an opponent different tunes by starting country: use the tune the table gives when the player started in that opponent's own country. Thailand is never a starting country, so for Adon and Sagat use the Japan start's tune. *My call.*

## 11. As built

### ST2 (#196): the pick rules

Built as section 4 says. Calls the ticket left open:

- **The picker.** The session holds the side at the controls: Player 1 at the start, the loser after a match (read from the original's winner byte `0xffc009`, 4 for the CPU counting as Player 2), unchanged after a draw. It starts over whenever no fight is up and the screen is neither a select nor a stage screen, which is the menu or the attract, however the players got there (the after-match `MENU`, the pause box, backing out of the select).
- **Local and Training.** Only the picker's inputs move the cursor in Local. In Training one player is at the controls, so the stage screen keeps the merged cursor and Player 1 is the picker; the CPU and the dummy never pick, whatever the last match was.
- **Rematch** pushes the stage screen over the select with the cursor on the stage just played. After a `RANDOM` pick that is the stage `RANDOM` gave, not the tile.
- **Cursor start.** The listed stage whose owner is the home owner of Player 2's fighter (Retsu for Ryu, Joe for Ken, the fighter's own stage for anyone else). A fighter with no listed stage starts on the first. Reselect starts there, not on the stage last played.
- **`RANDOM`** is a tile one row under the last stage, reached by Down from the last stage and Up from the first. It is chosen when the player confirms, from the session's frame count, and the fight starts at once, so nothing shows the pick before the VS screen. A later wave that lists more stages widens the choice with no change.
- **Look.** Built by ST3 (see below); this ticket had only a `P1 PICKS` or `P2 PICKS` line above the list and the `RANDOM` text.

### ST3 (#197): the look

Built as layout A of `docs/design/mockups/2026-11-06-stage-screen-layout.html` (the maintainer's choice, 2026-11-06). Calls the ticket left open:

- **The frame** is the front end's wide gold frame (the Settings frame, columns 1..=46), because eight 40-pixel tiles need its 352 pixels. `STAGE SELECT` is in the 2×2 header and the preset line is at the foot.
- **The grid grows by rows and the large preview takes what is left.** Tiles are 40×22 on a 44×24 pitch, eight to a row, sitting on the screen's foot, so ten stages and `RANDOM` (two rows) leave a preview of about 168×98 pixels and 38 plus `RANDOM` (five rows) leave about 46×26. Nothing moves otherwise, and no wave redesigns the screen. *My call*: the mockup's fixed 160×75 preview cannot sit above five rows in 224 pixels.
- **Tiles** are a gold outline around a 38×14 window cut from the stage's backdrop (a band across it, a little above its middle) with the owner's name under it in the 3×5 font; a long name is cut with `..`. The cursor's tile has a second outline and a name in the picker's colour: P1's yellow, P2's the compositor's blue (the select screen's pens). In Training the cursor is always yellow and no `PICKS` line is drawn.
- **`RANDOM`** is the last tile, grey diagonal stripes under a `?`; with the cursor on it the large preview is the same pattern with a big `?` and the caption reads `RANDOM · ANY STAGE`.
- **The middle dot** is the one the move list (ML1) added to the compositor's made glyphs.
- **Previews are cut from the stage's own backdrop.** `BackgroundBank::stage_preview` draws both layers from the record's views as a fight starts on it (foreground at its word plus the camera's `0xc0`, background at its word, no sprites or text), halves it to 192×112, and the screen scales that into the tiles and the large preview. The session draws them once, the first time a stage screen opens (about 30 ms for the ten, in release), and keeps them. A stage whose preview is missing draws a dark swatch.
- **Who appears.** Nothing marked a wave as released, so a stage record now carries its `wave` (0 for the ten 1987 stages, 1 to 3 for a challenger's, listed with `list_wave_stage`) and `shown_stages` filters the list: wave 0 always, a wave's stages only with bonus challengers on and the wave among `RELEASED_WAVES`, which is 0 until a wave ships. Bonus stages have no listing and are never shown; there is no "coming soon" tile.
- **Moving the cursor** stays ST2's one tile at a time through the drawn order (Up and Down, wrapping through `RANDOM`), because ST2's tests and the key-art tool step with Down. Left and Right now do the same, back and forward. A row jump is left for a later ticket if wanted.

## 12. As built: stage themes in fights and the music pack (ST7, #199)

- **How the table reaches the music CPU** (the boundary ST6 left; *my call*, taking the first of the two options in the ticket). The CPU is one object per session and the preset can change between fights, so it holds the extended image only during a fight under an overhaul preset. `FrameOutput::tune_table` is true while such a fight runs; the launcher gives the CPU the table before that fight's first frame and takes it away after. The change travels in order with the latch frames, and a take-away waits while a tune (current or saved) still has a stream in the extension, so a theme that outlasts its fight is never cut. Faithful 1987 fights run the ROM alone, and `themes::load` returns no table, and opens no pack, under that preset. Details: `docs/reference/extended-tunes.md`.
- **The music pack** is a folder `sf1-port-music` beside the ROM set holding `.score` files, each naming its own command. It is read only once the configured preset is an overhaul one (*my call*: lazily, so a session that starts in Faithful 1987 and switches in Settings still gets it, and one that never switches never reads it).
- **The fallback** (*my call*, as section 6.4 says): a stage whose theme the table does not answer plays the 1987 theme `0x29` (Geki's) and the log names the stage and the theme. The substitution lives in the background bank's stage view, so the engine never sends a command no table answers.
- **The test stage's record** now names `0x40`, the test theme's command, so it plays the test theme in tests and with `--stage test-stage`.

## 13. As built: the Jukebox in real fights (ST8, #200)

Built as section 6a says (OGG Vorbis, WAV and FLAC; tracker modules are not part of this ticket). Code: `crates/sf-engine/src/jukebox.rs` (keys, resolve order, the cue machine), `front/session.rs` (`FrameOutput::track`), `crates/sf-host/src/track.rs` (decoding, loop points, playhead) and `jukebox.rs` (the folder, the deck, the mixer), `crates/sf-game/src/main.rs` (the wiring). Calls the ticket left open:

- **Keys are names.** A track is set for the whole game, for a stage by its record's name in the background bank (`retsu`, `test-stage`) or for a fighter by name (`Ken`, `Evil Ryu`); names compare without regard to case. The 1987 stage byte and the twelve-fighter array are gone, so a wave's stages and challengers need no change here. The old Red Book `Jukebox` of the O9 prototype (`session_io`) was replaced by this one; its loop points in the choice are gone, as they now come from the file.
- **Resolve order.** Character, then stage, then global. Of two character tracks the stage picker's wins (ST2's picker: Player 1 on the first match, the loser after; Training is Player 1); with one, that one. Arcade has only Player 1's fighter, as its opponent changes with the ladder (*my call*). Arcade's stage is the one the ladder is at, read (never written, ADR-0004) from work RAM: the start country `0xffc006` and the stage and fight counters `0xffc003` and `0xffc004` (stage x 2 + fight) name the opponent through the country's foreground-scroll table (`rom_stages::ladder_opponent`), well before his theme. It is not found from the theme: the test stage and Geki's stage can play the same tune, and the ladder tells them apart.
- **Cues, not latches.** The session leaves the latch as it was and adds `FrameOutput::track`: `Prepare` when the track the fight will have is announced (a fight's first frames, and again whenever it changes, as Arcade's next opponent does), `Play` on the frame a new command equals the fight's stage theme, `Stop` on a new stop (`0x00`, `0x35`, `0x3d`, `0x3f`), on the win jingle `0x37`, and when the fight is left. A command held for several frames is one command, as the driver takes it. The theme is the stage record's played theme, so a stage whose own theme is missing (the 1987 fallback) still gets its track. Faithful 1987, the attract, the cabinet and a fight with no track set give no cue.
- **The host decides the latch.** While a started track plays, the theme's command is left out of the music CPU's input on its frame and on every frame it is held or sent again (`Deck::latch`) until the stop; stops and the jingle still reach it, so effects and the jingle behave as natively. A file the host cannot find or play logs one line (once, not every round) and the command passes, so the native theme plays (*my call*: the fall-back is to the native theme of that stage, not to the next level of the order).
- **Nothing is decoded on the game thread.** `Prepare` starts a thread decoding the file (the hidden drive and VS screen take seconds); `Play` uses it if it is done. If it is not (the file was not announced, or is still decoding) the thread goes on and the native theme plays that round, with one log line. A panic in a decoder is caught in its thread and counts as a failed file. One track is kept, so the next round starts at once; announcing another stops the decode of the one replaced (a cancel flag, read each packet); a file whose modified time or size changed is decoded again, a failed one included.
- **Limits** (*my call*): a file over 256 MiB is refused before it is read, and one stated or found longer than ten minutes before or while it is decoded. A WAV's `smpl` chunk is read by seeking over the chunk headers, never reading the sound.
- **Where files are.** A `jukebox` folder beside the ROM set, looked for as the art pack is; an absolute path in the settings is used as it is.
- **Loop points** (*my call* on the details). `LOOPSTART` and `LOOPLENGTH` in sample frames, tag names without regard to case, in OGG and FLAC; the first forward loop of a WAV's `smpl` chunk, its end the last frame played. `LOOPSTART` alone loops to the end; a length past the end is cut; a length alone says nothing; loop points outside the file or not advancing are no loop. No loop information replays from the top. The intro before the loop start plays once.
- **Decoders: symphonia 0.6** for all three formats (MPL-2.0, like librashader): maintained, one dependency, tags for OGG and FLAC come back from it. `lewton` and `claxon` have had no release for years, and `hound` reads WAV only and exposes none of its chunks. The `smpl` chunk is read from the bytes by `wav_loop`. A track is held decoded as stereo `f32` (mono on both sides, the first two of more channels) and is refused past ten minutes (*my call*).
- **Level** (*my call*). A track is mixed over the board's sound at `MASTER` x `FM` x `TRACK_GAIN`, which is the chip route's gain (`YM_ROUTE_GAIN`, 0.6) times a reference of 1. The reference is measured: the ten 1987 stage themes come to 0.07 to 0.13 RMS at the output (0.09 on average; `crates/sf-game/tests/loudness.rs` repeats the measure), and a mastered track sits near 0.15 RMS, so a track at that level lands on the themes'. The sum of track and board goes through a soft limiter (unchanged up to 0.8, bent to 1) while a track plays. The track runs on the output's machine time, starting and stopping at the frame of its cue, and holds its last sample when the game stalls.
- **Settings keys** (set from the Jukebox screen since ST10, section 14): `[jukebox]` with `global = "file"`, `[jukebox.stages]` and `[jukebox.fighters]` with `"name" = "file"`. A file with no track has none of the tables. Tracks are local presentation, never in the synced `MatchConfig`. A bad line of these tables (a table name that is not one of the three, a key with no quoted file) costs only that line: the game loads the rest and logs it. A key set twice (names differing only in case are one key) logs a warning and the later line wins; a stage or fighter name that matches no record is logged at launch. Any settings file that fails to load or loads with a skipped line is copied to `settings.toml.rejected` before the game can save over it.
- **Not done here:** the Jukebox screen (built in ST10, section 14), tracker modules, and any track for the VS screen, menus or Arcade's continue tune.


## 14. As built: the Jukebox screen (ST10, #202)

The Sound tab's `JUKEBOX` row opens the Jukebox screen as section 6a, item 4 says. Code: `crates/sf-engine/src/front/jukebox.rs` (the screen), `jukebox.rs` (`Pick`, the resolve order, `TRACK_EXTENSIONS`), `front/session.rs` (the screen on the stack, `set_tracks`), `crates/sf-host/src/jukebox.rs` (`list_tracks`) and `crates/sf-game/src/main.rs` (the host lists the folder). Calls the ticket left open:

- **The list.** The global track first, then every stage the stage select shows (ST3's `shown_stages` with the Roster tab's bonus switch and `RELEASED_WAVES`, in the stage screen's order, so the test stage is never listed), then every fighter: the twelve of the tournament and, for a shown stage a challenger owns (wave 1 and up), that challenger (*my call*: no roster record of released challengers exists yet, and a challenger's stage ships with the challenger, so the stage listing is the one source both lists follow). Today no wave has shipped, so the list is the ten stages and the twelve fighters. A stage is keyed by its record's name and a fighter by name, as ST8 has it, and drawn as the stage select spells its owner. A list taller than the screen scrolls with the cursor and shows an arrow where rows are hidden.
- **The choices.** Left and right step a row's choices round a ring, confirm steps on, and a tab press steps ten (*my call*, for a long folder; there is no browser). A stage or fighter steps `NATIVE`, then the files; with a global track set, `GLOBAL` (no entry) comes first. The global row steps `NATIVE` (no track) and the files. `CLEAR ALL` on the last row puts every row back. `RESET TAB` on Sound still resets the volumes only (*my call*: the tracks have their own clear, and a tab reset should not drop a library of choices).
- **`NATIVE` overrides the global track** (the gap ST8's review noted). `NATIVE` on a stage or fighter is an entry of its own (`Pick::Native`), kept in the settings as `"retsu" = "NATIVE"` (no case, no padding; a file really named `NATIVE` is written `./NATIVE`). The resolve order is unchanged in shape, now over entries: a fighter's entry first (the stage picker's before the other's; of two, the picker's decides even when it is `NATIVE`), then the stage's, then the global track, and the first entry found decides, `NATIVE` meaning the native theme. `global = "NATIVE"` reads as no global track (nothing is under it to override). A stage or fighter with no entry follows the next level down; without a global track it shows `NATIVE`, which is what plays, and a `NATIVE` the player steps to is stored all the same, so a global track set afterwards does not take that row (*my call*: what the row showed is what it keeps).
- **Files.** The host lists the `jukebox` folder (`sf_host::jukebox::list_tracks`, beside each ROM folder as the art pack is looked for, files only and not the folders in it, hidden names left out, each name once, in name order without regard to case) as Settings and then the Jukebox screen open, so files dropped in since show. The engine never reads a folder: `GameSession::set_tracks` takes the list, like `set_looks`. One list of extensions, `jukebox::TRACK_EXTENSIONS` (OGG, WAV, FLAC today), is what the host lists by and what the screen filters by again, so ST9 adds the tracker formats in one place. The list is flat: a file in a subfolder is not offered (a hand-written `"cd/slug.wav"` in the settings still plays). Names are drawn upper case in the ROM font's characters, `?` for any it lacks; the settings keep the real name.
- **A file the folder lacks** (renamed, removed, or an absolute path): the row keeps showing it, dim, with a notice, and it is the last choice on its row until the row is stepped off it. The settings keep it as they were until the row changes.
- **An empty folder** shows `NATIVE` alone and a notice saying what goes in the folder.
- **Saving.** The screen edits a copy and hands it to the Settings screen under it on Back; leaving Settings saves with the other settings (`settings_changed`), as every Settings change does. The tracks are used from the next fight.
- **Tests.** `tests/jukebox_screen.rs` (the screen, no ROMs), `tests/jukebox_in_fights.rs` (the walk Settings, Sound, JUKEBOX through the session into fights, with ROMs), `sf-host/tests/track_listing.rs` (the folder).

## 15. As built: tracker modules (ST9, #201)

Built as section 6a.5 says. Code: `crates/sf-openmpt` (libopenmpt's vendored source, its `build.rs`, and the few C calls used), `crates/sf-host/src/tracker.rs` (rendering and the loop) and the dispatch in `track.rs`. The Jukebox's settings keys and the host's cue handling are ST8's, unchanged: a module is one more kind of file that decodes to a `Track`. The screen's list of formats is the one place ST10's work changed (below). Calls the ticket left open:

- **Building libopenmpt** (*my call*): its official 0.8.10 source release is vendored in `crates/sf-openmpt/vendor/libopenmpt`, an unmodified subset (6.6 MB of source, no binaries; BSD-3-Clause, with miniz, minimp3 and stb_vorbis beside it under their own licences, listed in `THIRD_PARTY_NOTICES.md`), and compiled to a static library by `build.rs` with the `cc` crate, with `NDEBUG` so a C assert cannot abort the game. The crates.io bindings only declare the C API; the Windows development package would put `libopenmpt.dll` and four helper DLLs beside every executable and a download step in a fresh clone; vcpkg is a toolchain to install. The vendored source needs only the C++ compiler Rust's MSVC target already comes with. Steps and the comparison are in the README and `docs/technical-overview.md`.
- **The gate** is `sf-host`'s `tracker` cargo feature, on by default (the build is reliable: the vendored source builds clean with MSVC), forwarded by `sf-game` and `sf-dev`. `--no-default-features` on those crates builds without libopenmpt and a C++ compiler; a module then fails to load with a message saying so, and the Jukebox's existing fallback logs it and plays the native theme. `--workspace --no-default-features` still builds `sf-openmpt`, a member of its own.
- **Which files are modules.** libopenmpt reads by content. A file whose extension is on its list (WAV, OGG and FLAC aside) goes to it first; if it cannot play it, symphonia gets the file. Any other file goes to symphonia first and, if that does not recognise it, to libopenmpt. So a wrong or missing extension plays either way; a file neither can read is one error naming both reasons.
- **The Jukebox screen offers modules only where they play** (review of this ticket; changes ST10's `TRACK_EXTENSIONS`). The engine's list is now `TrackFormats`, which the host supplies (`sf_host::track::supported_extensions()`: OGG, WAV and FLAC, and libopenmpt's extensions only with the `tracker` feature) to the folder listing and, through `Session::set_track_formats`, to the screen, whose empty-folder notice names modules when they are on the list. A build with no tracker player never offers one.
- **The loop is the module's own, through libopenmpt's restart position** (*my call*; the first version measured a second pass and was wrong when the tempo carries over the jump back). The module is played once (repeat count 0, `play.at_end` `stop`, so there is no fade-out tail); that is the track. libopenmpt reports the sub-song's restart order and row, which includes a jump back that makes the song loop (a last-pattern Bxx in S3M, IT and MTM) as well as the MOD, XM, 669 and MPTM restart positions; a fresh module is seeked there, and the time it reports is the loop's start frame. The loop runs from there to the end of the song; no restart position is a loop from frame 0. The test song has a speed change (`loop-tempo.mod`) that a length-by-second-pass method gets wrong.
- **The seam and the tempo.** The track is the first pass cut at the song's end, then restarted at the loop start, like every other track. libopenmpt's own repeat would carry notes still ringing at the end over the jump, and would keep the speed and tempo the song ended on; the Jukebox's `Playhead` does neither, so the loop plays as it sounds the first time through, and a module whose last notes ring may click or lose a tail at the seam. A tracker-aware streaming player would avoid it and would not fit the Jukebox's decode-once design; not done.
- **No sound is an error.** A module that loads and plays but renders nothing (a truncated file whose samples never came) is refused as "no sound in the module", with the same log line and native fallback as any other failure. The limit is a peak under 1e-4 over the whole song.
- **Limits.** Ten minutes, as every track: a song over `MAX_SECONDS` is refused, which also stops a module that never ends. A libopenmpt error mid-render is an error naming it, not a short track.
- **Not tested here:** MPTM and the formats beyond the six the tests build (MOD, XM, S3M, IT, 669, MTM): no small hand-made file exists for them and no copyrighted one is committed. libopenmpt reads them by the same route.
