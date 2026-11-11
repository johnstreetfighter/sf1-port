## Problem Statement

Two players sitting down to Local can't choose where they fight. In the overhaul, P1 picks a flag on the 1987 country screen and the stage follows that flag. P2 never gets a say. Thailand's stages can't be reached at all, and of the two stages in each country, only the first can be.

The roster is also growing to 40. The bonus-roster design promises every challenger "a stage, a theme". But the game has no way to add a stage or a theme, so nothing a wave makes would have anywhere to go. The Jukebox exists in code, but no fight uses it, and its Sound settings row is greyed out, so players can't swap in their own music either.

## Solution

In overhaul presets, Local and Training get a **stage select** after fighter select. It lists every stage: the ten 1987 stages now, and each roster wave's stages when that wave ships. P1 picks the stage for the first match. After that, the loser of the last match picks. There's also a RANDOM tile. The fight goes straight from the stage screen to the VS screen and the fight, with no country intro. The chosen stage's theme plays, unless the player has set a Jukebox track for that stage or fighter.

Behind the scenes, a new stage lives in an **extended background bank** and a new theme lives in an **extended tune table**. The game's own background code and sound driver read both, so new content scrolls, flips, drifts, and gets cut and resumed by the win jingle exactly like 1987 content (ADR-0009). One test stage built from ROM tiles and one short original theme prove this path end to end before any wave depends on it.

The Jukebox gets wired into real fights, and its Sound settings row now opens a screen where every stage and fighter can be set to `NATIVE` or to a file from a `jukebox` folder. The folder takes OGG, WAV, FLAC and tracker modules.

Faithful 1987 doesn't change at all.

Design: `docs/design/2026-11-03-stages-and-music-design.md`. Decision: `docs/adr/0009-stages-and-themes-in-extended-banks.md`. Vocabulary: the "Stages and music" section of `CONTEXT.md`.

## User Stories

1. As a Local player, I want to choose the stage before a fight, so that we can fight where we like.
2. As P2 in Local, I want the stage cursor to start on my fighter's home stage, so that the first match feels like a visit to my opponent's home, as in *SF2* versus.
3. As P1 in Local, I want to be the one who picks the first stage, so that it's clear who chooses.
4. As the loser of a Local match, I want to pick the next stage, so that both players get a say over a session.
5. As the loser choosing on Rematch, I want the cursor to start on the stage we just played, so that one press keeps it.
6. As a player after a draw, I want whoever picked last to pick again, so that there's never a moment where nobody is in control.
7. As a player who goes back to the menu, I want the next Local session to start with P1 picking, so that a new session starts fresh.
8. As a player choosing Reselect, I want to pick fighters and then reach the stage screen under the same loser-picks rule, so that changing fighters doesn't reset who chooses the stage.
9. As a player who doesn't want to choose, I want a RANDOM tile, so that the game picks for me.
10. As a player who picks RANDOM, I want the stage revealed only when the fight starts, so that it feels like a surprise.
11. As a player picking Ryu or Ken as P2, I want the cursor to start on a stage from their home country (Retsu's stage for Ryu, Joe's for Ken), since they have no home stage, so that the default still makes sense.
12. As a Training player, I want the same stage select with me in control, so that I can practise on any backdrop.
13. As a Training player against the CPU, I want the cursor to start on the CPU opponent's home stage while still being free to pick another, so that my choice of opponent doesn't limit my choice of stage.
14. As a Training player against the dummy, I want to pick any stage, so that the dummy can stand anywhere.
15. As a player of any overhaul preset, I want to be able to reach all ten 1987 stages, Thailand's included, so that no stage is out of reach.
16. As a player, I want bonus stages left off the list, so that every tile is a real place to fight.
17. As a player with bonus challengers turned on, I want a released wave's stages to appear on the stage screen, so that new fighters bring new places.
18. As a player with bonus challengers turned off, I want only the 1987 stages listed, so that the screen matches the roster I chose.
19. As a player, I want no stage from an unreleased wave ever listed, and no "coming soon" tile, so that the screen never shows unfinished work.
20. As a player, I want the stage screen to look like the rest of the front end: framed, with tile previews, the owner's name, the picker's cursor colour and the preset line, so that it fits in.
21. As a player, I want the cursor shown in the picker's colour (P1 yellow, P2 blue), so that I can see whose turn it is.
22. As a player, I want the stage screen to still look right with 38 stages, so that it doesn't need a redesign at every wave.
23. As a player, I want the fight to go straight from the stage screen to the VS screen and the fight, with no country intro, so that I never see one country's scenery and then fight somewhere else.
24. As a player, I want the chosen stage's scrolling, palette and music in the fight, so that the choice actually takes effect.
25. As a player, I want the stage's own theme to play on it whoever is fighting, so that every place has its own sound.
26. As a player on a 1987 stage, I want the theme the 1987 game plays for that opponent, so that the original stages sound like themselves.
27. As a Faithful 1987 player, I want the flag screen, the country intro, the stages and the music exactly as in 1987, so that the faithful promise holds.
28. As an Arcade player, I want the ladder to choose the stage as it always has, so that Arcade doesn't change.
29. As a player, I want a new stage to scroll with the camera, flip with flip screen and drift like a 1987 stage, so that it doesn't feel bolted on.
30. As a player, I want the win jingle to cut a new theme and hand it back the way it does a 1987 tune, so that new music behaves like the old.
31. As a player, I want a new theme to sound like it comes from the 1987 sound board, so that challengers fit the game.
32. As a player whose music pack is missing a theme, I want the fight to start anyway with a 1987 stage theme, and a line in the log saying so, so that a missing file never blocks play.
33. As a player who keeps arranged music, I want Capcom-melody arrangements read from a local music pack beside my ROM set, so that they stay out of the public repository.
34. As a player, I want the JUKEBOX row in Sound settings to open a Jukebox screen, so that I can set tracks without editing files by hand.
35. As a player, I want every stage and every fighter, challengers included, listed on the Jukebox screen, so that I can set music for any of them.
36. As a player, I want each Jukebox entry set to `NATIVE` or to a file from the `jukebox` folder beside my ROM set, so that choosing a track is a simple pick from a list.
37. As a player, I want OGG, WAV and FLAC files accepted, so that common audio rips work as they are.
38. As a player, I want tracker modules accepted (MOD, XM, S3M, IT, MPTM, 669, MTM and the rest libopenmpt plays), so that tracker music plays directly.
39. As a player, I want OGG and FLAC tracks to loop at their `LOOPSTART`/`LOOPLENGTH` tags, WAV tracks at their loop chunk, and modules at their own restart position, so that tracks loop seamlessly.
40. As a player with a track that has no loop information, I want it to play to the end and start again, so that it still works.
41. As a player, I want a fighter's track to win over a stage's track, and a stage's track over the global track, so that the most specific choice plays.
42. As a player where both fighters have a character track, I want the stage picker's fighter's track to play, so that one consistent rule decides.
43. As a player, I want a Jukebox track to stop for the win jingle and round ends the way native music does, so that the fight's sound still makes sense.
44. As a player whose chosen track file has gone missing, I want the native theme to play, so that a missing file is harmless.
45. As a wave builder, I want to add a stage as one record (scroll positions into the extended map, palette, theme, owner, name, preview) plus its art in the art pack, so that a challenger's stage drops in without engine changes.
46. As a wave builder, I want a stage packed only if it obeys the 1987 background rules, so that a bad stage is rejected before it ever reaches a fight.
47. As a wave builder, I want to write a theme as a readable score and have a converter turn it into the driver's own channel streams, so that I never hand-write driver bytes.
48. As a wave builder, I want the converter to reject anything the 1987 driver can't play, so that every theme is genuinely 1987-playable.
49. As a wave builder, I want each challenger to own exactly one stage and each stage exactly one owner, so that the theme rule never has two owners.
50. As a maintainer, I want the stage choice to reach the engine through a roster setting and an overhaul-only gate, never a RAM write, so that ADR-0004 holds.
51. As a maintainer, I want both new banks never loaded in Faithful 1987, the attract or the faithful cabinet, so that those paths stay byte-identical.
52. As a maintainer, I want confirmation that the 1987 game never scrolls or reads past the ROM tilemap or sends the tune commands the extended table uses, so that the banks can't change 1987 behaviour.
53. As a maintainer, I want a test stage built from ROM tiles and a short original test theme committed and exercised in tests with only the ROM set, so that the path is proven without any pack.
54. As a maintainer, I want libopenmpt to build on Windows as part of this work, so that tracker support isn't left half-done.
55. As a player who pauses mid-fight, I want quitting to the menu to behave as before and reset the stage-pick rule, so that pausing works the same way it always has.

## Implementation Decisions

- **Stage records.** A stage is a record: foreground and background scroll positions, palette, theme tune command, owner, display name, and preview. The ten 1987 stages are records built from the ROM tables (one per opponent: Retsu, Geki, Joe, Mike, Lee, Gen, Birdie, Eagle, Adon, Sagat). New stages are records pointing into the extended background bank. The stage screen, the gate and the Jukebox all treat every stage alike. Every challenger owns exactly one stage, so there are 38 stages once all waves ship.
- **A 1987 stage's theme** is the tune the 1987 table gives for that opponent when the player started in the opponent's own country. Adon and Sagat use the Japan start's tune, because Thailand is never a starting country.
- **Stage-select screen.** A new pure front-end screen, in the shape of the existing ones: menu inputs in, a frame and a transition out. It runs between the Local/Training select and the hidden drive. It tracks who picks: P1 at the start of a session, then the loser of the last match, and the last picker again after a draw. That state lives as long as the Local or Training session and resets when players return to the menu. The cursor starts on P2's fighter's home stage (Retsu's for Ryu, Joe's for Ken). In Training against the CPU it starts on the CPU opponent's home stage, and on Rematch it starts on the stage just played. RANDOM picks among the listed stages, and the result is only revealed at the fight. The list holds the 1987 stages, plus each released wave's stages when bonus challengers are on. The exact layout is shown to the user as a mockup before it's built.
- **After-match flow.** Rematch now passes through the stage screen with the loser in control. Reselect goes through fighter select and then the stage screen.
- **Faithful 1987 preset.** No stage screen, the flag stays visible, and nothing new is gated in. Arcade is unchanged under every preset.
- **How the stage reaches the engine.** The chosen stage travels as a roster setting. The cabinet driver still picks a flag, hidden, so the 1987 flow runs as verified. Three overhaul-only gates in the translated path then read the chosen stage's record in place of the country tables: the stage's scroll set-up (`stage_reset`), the stage palette request at every call site that serves the chosen fight (the palette is chosen by country and ladder step only, and has five call sites), and the stage music request. With no stage chosen, every gate stays closed. Nothing writes work RAM (ADR-0004).
- **Country intro.** One more overhaul-only gate skips the country intro (events 2 and 3) in Local and Training under overhaul presets. Arcade and Faithful 1987 keep the intro.
- **Extended background bank** (ADR-0009). This is overhaul-only background tiles above the ROM's own, maps past the ROM tilemap's 2048 columns, and stage palettes. The translated background code and the renderer draw it. Before it is enabled, a sweep has to confirm the 1987 game never scrolls or reads past the ROM map. A packer checks each stage against the 1987 background rules. Stage art lives in the art pack, and only the records are committed. The background format and the scroll range have not been mapped yet, so the first ticket here carries the most risk.
- **Extended tune table** (ADR-0009). This is overhaul-only, looked up for tune commands the 1987 game never sends, and played by the translated FM driver. A score converter turns a readable score into the driver's channel streams and rejects anything the driver can't play. Themes are FM-only for now; ADPCM in themes waits until a wave needs it. Arrangements of Capcom melodies live in the local music pack. Original compositions may be committed. If a theme is missing, a 1987 stage theme plays instead and a line is logged.
- **Proof content.** The test stage is rebuilt from the ROM's own background tiles in a new arrangement. The test theme is a short original piece. Both are committed and run with only the ROM set.
- **Jukebox wiring.** The fight's music asks the Jukebox first. Its keys widen from the 1987 stage byte and the twelve fighters to every stage record and every fighter. The resolve order stays character, then stage, then global. When both fighters have a character track, the stage picker's track wins. The host plays external tracks and imitates the driver's stops for them, which was the Jukebox's accepted trade-off from the start.
- **Jukebox screen.** The greyed `JUKEBOX` row in Sound settings opens a screen listing every stage and every fighter. Each is set to `NATIVE` or to a file from a `jukebox` folder beside the ROM set. There's no file browser and no loop editing. The choices persist with the other settings.
- **Track decoding.** OGG Vorbis, WAV and FLAC play through Rust decoders. Tracker modules play through libopenmpt, across its full format list, and its C++ build on Windows is part of this work. Loop points come from `LOOPSTART`/`LOOPLENGTH` tags in OGG and FLAC, from the loop chunk in WAV, and from a module's own restart position. A track with no loop information plays through and restarts. A missing file falls back to the native theme.

## Testing Decisions

- **A good test** drives the game the way a player or the real program does and checks what comes out: the frame, the sound commands latched, the YM2151 writes, the screen transitions, and the stage actually in use. It never checks private state or the shape of internal structures. As in the existing front-end and fidelity tests, the ROM-dependent ones skip when the local ROM set isn't there.
- **Seam 1: the headless game session.** This is the one the existing front-end tests drive, with inputs in and frames and sound out. It covers:
  - the stage screen's rules: P1 first, loser picks, the draw rule, reset at the menu, the starting cursor in each case, Rematch keeping the stage in one press, RANDOM, the list filter for bonus challengers;
  - each of the ten 1987 stages reaching a real fight, with the right scroll positions, palette and stage tune command;
  - the country intro being skipped;
  - the test stage and test theme in a real fight;
  - the Jukebox screen, and the Jukebox's resolve order (character, then stage, then global; picker's character wins);
  - Faithful 1987 still showing the flag screen.
  - Prior art: the front-end Local, Training, select and settings tests.
- **Seam 2: the power-on scenario sweep.** With both new banks loaded, every committed scenario from power-on stays byte-identical. The 1987 game never reads past the ROM tilemap or sends an extended tune command. The attract, the faithful cabinet and Faithful 1987 fights never load either bank. Prior art: the extended sprite bank test.
- **Seam 3: the sound-CPU tests.** A converted test theme plays through the translated driver from the extended table. The win jingle and round stops cut it and hand it back exactly as they do a 1987 tune. The converter rejects a score the driver can't play. Prior art: the music-stops, tune-loops and sound-test tests.
- **Track decoding** is tested where the Jukebox resolves and loops a track: loop points read from each format's tags, the no-loop rule, and a missing file falling back to native. Prior art: the existing looped-sample test.

## Out of Scope

- Any challenger's actual stage art or theme. Each wave's spec makes those.
- New stages for the twelve tournament fighters.
- Any change to Arcade or its ladder, or to Faithful 1987.
- Stage choice online, in Dramatic Battle or in World Tour.
- ADPCM in themes.
- Editing loop points in the game, and any in-game file browser.

## Further Notes

- The riskiest unknown is how much of the 1987 background format, and the range of its scroll registers, has to be mapped before a map can sit past column 2048. Mapping it should be the first ticket on that path, and its findings go in a reference doc as for earlier reverse-engineering work.
- The stage select over just the 1987 stages, the country-intro gate and the Jukebox wiring don't depend on the extended banks. They can ship first, so Local gets its stage choice before any wave.
- What this supersedes: the front-end design's line that Local runs visibly from the flag pick into the fight (for overhaul presets only), and the 2026-10-27 design's "stage/BGM selector", which is this screen plus the Jukebox.
