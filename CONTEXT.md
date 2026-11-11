# Street Fighter PC Port

This glossary defines the terms used to discuss the original arcade Street Fighter and its PC port.

## Language

**Original game**:
The original arcade Street Fighter, the game this project aims to bring to PC.
_Avoid_: Street Fighter II, the series

**Port**:
The PC version of the original game developed by this project, preserving its gameplay and quirks as the initial fidelity target.
_Avoid_: remake, remaster

**Optional improvement**:
A deliberately selectable change to original gameplay, separate from the faithful baseline.
_Avoid_: fidelity fix

**Overhaul**:
The expanded edition of the original game—featuring all twelve playable tournament fighters, optional bonus challengers, era styles (including optional what-if styles), new modes, and training tools—that becomes the default experience once the faithful port is complete.
_Avoid_: remake, remaster, mod

**The World Tournament**:
The release name of the faithful port, *Street Fighter: The World Tournament*: the base edition, carrying the twelve tournament fighters and nothing more.
_Avoid_: sf1-port (the repository, not the game), Arcade Edition

**Edition lineage**:
Capcom's prefix-and-suffix naming applied to this port: the faithful edition takes the plain name, a prime or plus (Street Fighter') marks core toggles, *Anniversary Edition* adds era styles (after *Hyper Street Fighter II: The Anniversary Edition*, which let each fighter use any revision), and the three roster waves are *Super Street Fighter: Rising*, *Hyper Street Fighter: Bloodlines* and *Ultra Street Fighter: Final Round*.
_Avoid_: version number as a name, mod pack

**Tournament fighter**:
One of the twelve fighters from the original 1987 arcade tournament (Ryu, Ken, Retsu, Geki, Joe, Mike, Lee, Gen, Birdie, Eagle, Adon, and Sagat).
_Avoid_: base character, vanilla fighter

**Bonus challenger**:
An optional fighter from wider official Street Fighter or Final Fight lore who could have fought in 1987 (alive and adult then, or already dead), such as Evil Ryu, M. Bison, Gou Hibiki, Dan, Gouken, Dorai, Guy, or Belger, selectable when bonus roster options are enabled. SF1 and SF1+ are their native styles.
_Avoid_: guest character, MUGEN character, DLC fighter

**Roster wave**:
One of three releases of bonus challengers, each shipped only when every fighter in it is finished: wave 1 is tied to the 1987 tournament itself, wave 2 to one of the twelve tournament fighters, and wave 3 to having existed in 1987.
_Avoid_: phase (the overhaul's roadmap uses phases), DLC pack, season

**Core toggle**:
An individually selectable mechanical change applied directly to the original game's routines (such as positive-edge attacks, lenient command trackers, or standard blocking).
_Avoid_: fidelity fix, hack

**Era style**:
A per-fighter selectable package representing a specific Street Fighter entry or revision (`SF1`, `SF1+`, `SF2 WW`, `SF2 ST`, `SFZ3`, `SF3 3S`, `USF4`, `SF5 CE`, `SF6`, etc.), bundling that entry's moveset and era gauge. `SF1` is the 1987 game's own Ryu and Ken, as Faithful 1987 plays them; `SF1+` is the style all twelve tournament fighters play under the overhaul presets, each with Ryu's kind of kit.
_Avoid_: groove, ISM, edition, variation

**Classic rules**:
How an SF1+ fight plays by 1987's rules: a fighter can't be hit during his hit reaction, so there are no combos, and there are no cancels and no throws.
_Avoid_: classic level, 1987 mode

**Modern rules**:
Three rules that can each be switched on over the classic rules: true combos, cancels from normals into specials, and throws that can be teched.
_Avoid_: modern level, SF2 rules

**Balanced roster**:
A switch that retunes the twelve fighters' numbers, such as damage, speed, recovery and hitbox sizes, so they come out roughly even; it never changes animations or which moves a fighter has.
_Avoid_: rebalance, balance patch

**1987 character values**:
A move's numbers before Balanced roster touches them: the ROM's own where the ROM has the move, and tuned to match the fighter's 1987 personality where it doesn't.
_Avoid_: raw values, default values

**What-if style**:
An optional era style adapting a fighter to a Street Fighter entry they did not officially appear in (such as `SF3 3S` Lee or `SF2` Dorai), selectable when what-if style options are enabled.
_Avoid_: fan edit, custom groove, non-canon mode

**Era gauge**:
The resource and system-mechanic package associated with an era style (such as Parry and Super Art or Drive and Super Art), which can be decoupled from the fighter's moveset via match toggles.
_Avoid_: meter system, bar

**Era costume**:
An optional 1987-pixel-art outfit variant representing a fighter's appearance from a later Street Fighter entry, distinct from a color palette swap.
_Avoid_: HD skin, sprite rip

**Ruleset preset**:
A named bundle of core toggles and era-style rules (such as Faithful 1987, Classic Competitive, Pure Footsies, or Definitive Overhaul). Classic Competitive and Pure Footsies play the classic rules; Definitive Overhaul plays the modern rules with Balanced roster on. Classic Competitive was called SF1++ Competitive until 2026-11-07.
_Avoid_: profile, game mode

**Complete arcade experience**:
The full original game, including its progression and two-player modes.

**Reference version**:
US set 1 of the original game with six-button controls, against which the port's fidelity is judged.
_Avoid_: any arcade version

**Gameplay fidelity**:
Agreement with the reference version's gameplay outcomes and timing under the same inputs and starting state.
_Avoid_: looks close enough

**Comparison baseline**:
The pinned MAME version running verified reference ROMs used to compare the port with the original game. Known emulation discrepancies require investigation.
_Avoid_: unquestionable hardware truth

**Visual fidelity**:
Exact agreement with the original game pixels before display scaling.

**Look**:
A named way of showing the game picture as a monitor would, such as an arcade cabinet's tube or a consumer TV; it covers every screen, the faithful cabinet included. A look is built in, saved by the player from a tuned look, or dropped in as a RetroArch preset file.
_Avoid_: preset (that is a ruleset preset), filter, shader (bare)

**True 4:3**:
The picture at the shape the 1987 cabinet's monitor gave it, each game pixel 7/9 as wide as it is tall; the one place the picture is scaled by other than a whole number.
_Avoid_: stretched, widescreen

**Audio fidelity**:
Agreement with the original sound events, timing, pitch, and character, without requiring byte-identical output audio.

## Front end

**Front end**:
The overhaul's screens outside a fight: the attract wrapper, menus, settings, select, pause and after-match screens.
_Avoid_: launcher, shell, UI layer

**Attract**:
The faithful attract loop (warning, title, demo fight, ranking) running under the front end with every toggle off.
_Avoid_: demo mode, idle screen

**Play mode**:
What the player chose to play: Arcade, Local or Training.
_Avoid_: game mode (the original program's own mode byte), ruleset preset

**Local**:
The two-player fight on one machine, the original game's two-player mode under the front end.
_Avoid_: versus (bare), multiplayer (bare), couch play

**Faithful cabinet**:
The 1987 menu item: the complete original cabinet booted from power-on with coins, DIP switches and service mode, untouched by overhaul settings.
_Avoid_: faithful mode, arcade mode

**Cabinet driver**:
The front-end component that feeds coin and Start inputs to the running faithful machine, hidden, to carry it from the attract to the country-select screen.
_Avoid_: fight launcher, entry point, warp

**Screen**:
One front-end state machine that takes menu inputs and yields a frame and a transition, with no window or I/O of its own.
_Avoid_: page, view, menu (as a type)

**Ladder**:
The original one-player opponent sequence through the five countries, bonus stages, flights and ending.
_Avoid_: campaign, story mode, tower

**Stand-in**:
Ryu taking the player's own slot in the Arcade ladder when the player is one of a country's two fighters, fought on the stage of the slot he takes (the maintainer, 2026-11-07; before, the country's other fighter stood in). A Sagat player's final battle is against Ryu, in Thailand.
_Avoid_: substitute, swap, mirror fix

**Dummy**:
The idle human side of a Local-type fight used as the opponent in Training.
_Avoid_: bot, AI off, sparring partner

**Fair CPU**:
The overhaul's CPU opponent, which plays like a person through Player 2's pad: it decides from what the screen shows, seen after a human reaction delay, presses the same stick and buttons a player would, and never reads the opponent's inputs. The Gameplay tab's difficulty sets its delay, how often it chooses well and how cleanly it enters specials. Training's CPU under the overhaul presets; Faithful 1987 keeps Capcom's CPU.
_Avoid_: AI, bot, Capcom's CPU (the 1987 routines), utility CPU (the old practice model's)

**Preset line**:
The edition name that every framed front-end screen showed at its foot, naming the active ruleset preset. The maintainer dropped it on 2026-11-07. Settings' `PRESET` row shows the active preset, and so does Training's pause box.
_Avoid_: status bar, footer, version line

**Free play**:
The overhaul rule that no coin is ever needed: Start alone opens the front end, and the attract shows FREE PLAY where the cabinet blinked INSERT COIN.
_Avoid_: unlimited credits, coin hack

## Art pipeline

**Pose**:
One drawn picture of a fighter: a rectangle of sprite tiles in his palette.
_Avoid_: sprite, frame (the engine's word for one step of a move stream)

**Recipe**:
A pose rebuilt at launch from a fighter's own ROM tiles, reused or pieced together.
_Avoid_: kitbash, edit

**Art pack**:
The local folder of generated poses, kept beside the ROM set and never in the public repository.
_Avoid_: asset pack, mod

**Derived pose**:
A pose in the art pack made from another fighter's approved generated pose, recoloured and fixed by hand, with no new generation, as Ken's throw is made from Ryu's.
_Avoid_: palette swap (his differences are redrawn), copy

**Pose catalogue**:
The per-fighter list of moves and their poses that the SF1+ project builds its moves on.
_Avoid_: sprite sheet, atlas

**Extended sprite bank**:
The tiles above the ROM's own sprite tiles, loaded only in overhaul modes, where recipes and the art pack's poses live.
_Avoid_: expansion ROM

**Extended data bank**:
The bosses' move tables in Ryu's format, with their streams and frame records, held above the ROM's program region and read by the translated 1987 player code only in overhaul fights.
_Avoid_: mod data, patch table

**Mirror palette**:
A fighter's second palette, worn by the second side of a fight of a fighter against himself (Local, and both Training kinds; never Faithful 1987) so the two can be told apart. Hand-picked from his own colours, committed under `art/palettes/`, and approved by the maintainer on the palettes review page (`sf-art palettes`). It recolours the body only: projectiles, bursts and hit sparks keep their colours (the maintainer, 2026-11-08). The engine draws it through an overhaul-only gate that points the second side's sprites at a free sprite palette.
_Avoid_: palette swap (a Derived pose's recolour is redrawn by hand), costume, alt colours (a what-if style's outfit is another picture)

## Stages and music

**Stage**:
The place a fight happens: its backdrop and associated fighters, with one primary stage owner. The ten 1987 opponent stages each belong to one fighter; several bonus challengers can share a stage.
_Avoid_: background (one of the 1987 layers), arena, level, stage for a ladder step

**Primary stage owner**:
The fighter whose name identifies a stage when several fighters share it. The primary owner is also one of that stage's associated fighters.
_Avoid_: sole owner, theme owner

**Associated fighter**:
A fighter who belongs to a stage for native-theme selection. A cursor's home-country fallback does not create an association: Ryu and Ken do not belong to Retsu's and Joe's stages merely because the cursor starts there.
_Avoid_: stage picker, cursor default

**Ladder step**:
One country of the ladder, what the original program counts as its stage number.
_Avoid_: stage

**Bonus stage**:
The tile, table or board breaking interlude of the ladder, never a place a fight can be picked for.
_Avoid_: stage (bare), minigame

**Home stage**:
The stage a fighter belongs to, which may be shared with other fighters. Ryu and Ken have none.
_Avoid_: home country

**Stage select**:
The overhaul screen in Local and Training where one player picks the stage: P1 first, then the loser of the last match. A `RANDOM` tile picks among the listed stages as the fight starts. It leads straight to the VS screen and the fight, because under the overhaul presets the country intro is skipped there; Arcade and Faithful 1987 keep the intro.
_Avoid_: country select (the 1987 flag screen), level select

**Country intro**:
The 1987 scenery and country name shown before a fight (events 2 and 3), about 90 frames. The cabinet driver's hidden flag decides which country it would show.
_Avoid_: flag screen (the select that picks the flag)

**Theme**:
Native fight music played by the game's sound driver: the 1987 tunes and new arrangements in the same sound.
_Avoid_: BGM, song, track

**Character theme**:
The native theme assigned to a fighter, including a deliberately borrowed theme from a connected fighter. Sharing a stage does not make its fighters share one character theme.
_Avoid_: character track, stage theme (when the assignment is to the fighter)

**Theme cycle**:
The match-to-match sequence of associated fighters' themes used when neither participant belongs to the selected stage. It is distinct from the musical loop within one theme.
_Avoid_: loop, shuffle

**Theme library**:
The collection of native themes available to the overhaul, including the arrangements supplied by the local music pack.
_Avoid_: soundtrack, Jukebox, music pack (the local folder)

**Jukebox**:
The settings that name a track for the whole game, a stage or a fighter, set on the Jukebox screen, and the rule that picks one for a fight: character, then stage, then global. A stage or fighter set to `NATIVE` plays the native theme even where the global track is set. Without a track for a fight the native theme plays.
_Avoid_: soundtrack, music player

**Track**:
An external audio file the Jukebox plays in place of a theme.
_Avoid_: theme, song

**Music pack**:
The local folder of theme arrangements built on Capcom melodies, kept beside the ROM set and never in the public repository.
_Avoid_: soundtrack, audio pack

**Source card**:
The local provenance record identifying an arrangement's exact source game version, music cue, reference evidence, and musical loop.
_Avoid_: soundtrack listing, transcription

**Extended background bank**:
The background tiles, maps and palettes above the ROM's own, loaded only in overhaul modes, where new stages live.
_Avoid_: expansion ROM, overlay

**Map page**:
One block of both scroll layers' maps laid out as the ROM tilemap is, 2048 columns each. Page 0 is the ROM's; the extended background bank's pages follow, and an overhaul gate chooses which page a layer shows.
_Avoid_: page (bare, beside a front-end screen), bank (the whole)

**Stage record**:
The description of a stage: its name, primary owner, associated fighters and fallback theme, where its maps sit, and how its art is rebuilt from the ROM or found in the art pack. It holds no pixels. A new stage's is committed; the ten 1987 stages' are read from the ROM's tables when the game loads, on the ROM's own map page.
_Avoid_: stage file, level data

**VS screen**:
The background screen shown before a fight with both fighters' busts and the nameplate baked into the map. The ROM holds Ryu against each boss, each boss against Ken, and Ryu against Ken; every other two-player pair's is built in the extended background bank from those ROM screens when the game loads.
_Avoid_: versus card, portrait screen

**Test stage**:
The stage rebuilt from the ROM's own background tiles in a new arrangement that proves the extended background bank. It has no owner and is never listed for players.
_Avoid_: debug stage, dummy stage

**Extended tune table**:
The table answering overhaul-only music commands that the 1987 game never sends. It identifies native themes independently of the larger storage holding their scores and is never loaded in Faithful 1987.
_Avoid_: expansion ROM, custom BGM

**Score**:
The readable text a theme is written in, which the converter turns into the sound driver's channel streams and refuses when the driver cannot play it.
_Avoid_: MML, tracker file, sheet music

**Test theme**:
The short original theme, with no Capcom melody, that proves the extended tune table. It has no owner and is never listed for players.
_Avoid_: debug music, placeholder theme

## Fight HUD

**Era HUD**:
The overhaul's layer over a fight: the era gauges in the bottom corners, the combo counter and call-outs, TIME under the health bars, and the announcer's voice. The Faithful 1987 preset and the faithful cabinet show the plain 1987 screen instead, with one exception: while a show or a try the player asked for in Training runs, the strip under the health block, a Try's call-out for a move that came out, and after a miss the strip's second line (the player's last inputs) and the coach's fix words.
_Avoid_: overlay (that is the Training overlay), UI layer

**Gauge art**:
The drawing of the era gauges, one of three that every era shares: Ruled, Blocks or Plate.
_Avoid_: gauge style (an era style is a fighter's era package), gauge look (a look is the monitor picture), skin, theme

**Call-out**:
A word the era HUD shows on the side of the fighter who earned a moment, such as COUNTER, REVERSAL or PERFECT PARRY.
_Avoid_: banner, popup, notification

**Combo**:
Hits that each land while the opponent is still in hitstun from the one before. It takes two or more.
_Avoid_: string, chain (an Alpha chain combo is one kind)

**Combo timing window**:
A consecutive range of frames in which the follow-up’s activation input produces a combo. The frames are measured from the opening hit for a given fight setup and command sequence.
_Avoid_: cancel window

**Hitstun**:
The frames a hit fighter can't act. Under the classic rules it is his whole 1987 hit reaction, during which he can't be hit; under true combos it is the attacking move's, and a ground reaction can be hit.
_Avoid_: stagger, hit reaction (that is the animation)

**Blockstun**:
The frames a guarding fighter is held in his guard after blocking a hit. The 1987 game has none; true combos give each move its own.
_Avoid_: guard stun

**Cancel**:
Cutting a normal short with a special: under the modern rules, a special's command made in a cancelable normal's cancel window starts the special at once. The classic rules have none.
_Avoid_: interrupt, chain (a chain goes from one normal into another)

**Cancel window**:
The frames after a cancelable normal connects, hit or guarded, in which a special can cancel it. A whiff opens none.
_Avoid_: cancel frames

**Announcer**:
The one voice that calls the round and the call-outs in a fight: either the 1987 announcer with new lines designed to sound like him, or the alternate Japanese-accented announcer calling the whole set.
_Avoid_: narrator, commentator, MC

**Announcer voice**:
The player's choice of announcer on the Sound tab: Ringside (the default, pairing the 1987 lines with call-outs in the same voice) or Japanese (an alternate Japanese-accented tournament announcer who calls both the 1987 announcements and the overhaul call-outs).
_Avoid_: voice pack (that is the folder of generated lines), language

**Voice pack**:
The local folder of the announcer's generated lines, kept beside the ROM set and never in the public repository.
_Avoid_: audio pack, sound pack

## Netplay

**Rollback engine (`RollbackEngine`)**:
The pure, socket-free state machine that wraps a real fight, predicts missing remote inputs from the previous frame, and restores and replays the fight when a late input differs. It keeps pre-step fight snapshots for rollback up to eight frames, prevents repeated audio during replay, and compares per-frame state checksums.
_Avoid_: network driver (the host's socket transport), prototype peer

**Virtual net bus (`VirtualNetBus`)**:
An in-memory packet router connecting two or more online sessions in headless tests under scripted latency, jitter, packet loss and reordering, without opening OS sockets.
_Avoid_: mock server, socket driver

## Training

**Training**:
The play mode for practice, one player against a CPU opponent or the dummy, with no rounds: nobody is knocked out, health refills, there is no clock, and no round result, score or round-win marker shows. The same under every preset, Faithful 1987 included.
_Avoid_: practice mode, sandbox, versus CPU

**Health floor**:
The sliver of health that no fighter in Training drops below, so a hit that would have knocked them out lands as an ordinary hit. Hits on a fighter at the floor still count, at their full damage, in the combo count and its damage.
_Avoid_: infinite health, god mode, 1 HP

**Refill**:
A fighter's health coming back to full in Training once about a second has passed since they were last hit or blocked.
_Avoid_: regen, recovery (a fighter's recovery is the end of their move), heal

**Reset positions**:
Training's pause-box action putting both fighters back where the fight began, with full health, keeping the Training settings as they are now.
_Avoid_: restart round (Local and Arcade only), reload, rewind

**Training settings box**:
The small box the pause box's TRAINING row opens, holding the settings that only Training has: PRESET, HOLD COMBO and FILL METERS.
_Avoid_: training menu, options, sub-menu

**Fill meters**:
The Training setting that holds every era gauge's resource at its maximum, refilled straight after it is spent; spending it works as it normally does.
_Avoid_: infinite meter, max meter, free supers

## Move list and coach

**Move list**:
A fighter's moves in the current fight, each with its input, as the pause box's MOVES panel shows them; only moves that fighter can do under this preset and era style.
_Avoid_: command list, movelist, moveset (that is the fighter's whole kit, art included)

**Input notation**:
How the move list writes an input: drawn arrows, numpad numbers or motion words.
_Avoid_: input style, command format

**Show**:
Training performing a chosen move with the player's own fighter, its input drawn as it goes.
_Avoid_: demo, playback, replay

**Try**:
Training watching the player attempt a chosen move, with the coach.
_Avoid_: trial, challenge, practice mode

**Coach**:
The part of Training that reads the engine during a try and says whether the move came out, and if not, why.
_Avoid_: tutor, input checker, trainer

**Attempt**:
During a try, the tried move's own command tracker getting past its first step; it ends in the move coming out, a miss, or neither when the fighter is hit or blocks.
_Avoid_: input, try (a try holds many attempts)

**Fix**:
The coach's short line after a miss, such as TOO SLOW or MISSED ↘: the engine decides that the attempt failed and where, and the player's inputs choose the words.
_Avoid_: tip, hint, error

**Move registry**:
The table of every move real fights can do, each with its name, kind, input, recognizer, the engine move it produces and when it is available; the move list is the registry filtered by the fight.
_Avoid_: move table (the ROM's per-fighter table of move headers), move data

## Promotion

**Key art**:
A promotional picture of the port, made in the style of the 1987 game's advertising rather than its screen, for the README and posts.
_Avoid_: poster, banner (a banner is one of its formats), cover

**Illustrated style**:
The key art's line-and-wash treatment: pencil and ink lines with watercolour and marker washes on light paper, after late-1980s Japanese character illustration.
_Avoid_: painted style (the earlier, dropped treatment), sketch look (a look is the monitor picture)

**Cartoon style**:
The key art's flat cartoon figures on hot stripes, after the 1987 US cabinet art.
_Avoid_: flat look, cabinet look (a look is the monitor picture)
