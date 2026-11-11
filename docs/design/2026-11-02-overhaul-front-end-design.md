# Overhaul Front End Design

- **Date:** 2026-11-02
- **Status:** Approved (2026-11-02). Builder check done 2026-11-02; both open questions resolved (section 8).
- **Context:** [`CONTEXT.md`](../../CONTEXT.md), [`docs/design/2026-10-27-definitive-overhaul-design.md`](2026-10-27-definitive-overhaul-design.md), [`docs/adr/0002-two-tier-overhaul-architecture.md`](../adr/0002-two-tier-overhaul-architecture.md), [`docs/planning/overhaul-spec.md`](../planning/overhaul-spec.md), [`docs/session-handoff.md`](../session-handoff.md)

---

## 1. Intent and Why

The faithful **port** is complete and verified from power-on to the ending. The overhaul tickets `O1`–`O21` and `M1`–`M8` are closed, but only the core toggles (`O3`–`O6`), the diagnostic overlay and replay takeover (`O1`, `O2`), and the twelve-fighter binding (`O7`) act on the real 1987 fight. Everything from `O8` onward was built as a separate combat model that never touches the 1987 work RAM, and there is no title, menu, settings or pause flow anywhere: the only way to play is the development launcher's subcommands and function keys, which start quick fights from gitignored reference captures a player will not have.

On 2026-11-02 the user decided that overhaul mechanics must act on the real 1987 fighters in the real fight, that the missing boss animations will be generated from the ROM's own art, and that the work splits into seven projects in this order: front end; art pipeline; SF1+ on the real engine; era styles and gauges; endings, Dramatic Battle and World Tour; bonus challengers; netplay transport.

This document is the first project: **a player-facing game with a 1987-styled front end that reaches every feature that already works on the real engine**, and that gives later projects a place to plug in. Nothing here needs new art, and nothing here changes the engine.

### What the user asked for, in their words

- In overhaul play, no coins are needed.
- The game opens on the normal arcade attract screens. Pressing Start goes to a menu screen with the choices Single Player, Multi Player, Settings, 1987 (the faithful arcade game) and Exit.
- Single Player presents Arcade Mode, World Tour and so on. Multi Player presents local or online and so on.
- Settings has sections for all the overhaul toggles and other features, video, sound, controls and bindings, and anything else worth settling there.
- Gamepads are in scope.
- The main menu gets its own framed screen in the style of COUNTRY SELECT (not the title art). Settings sections run as tabs across the top (not a list on the left).
- The build is a new game binary with a pure, testable menu layer (not an extension of the dev launcher, and not menus inside the 1987 machine).
- Arcade is Ryu and Ken for now. Local and Training take all twelve. Arcade opens to the rest with the SF1+ project.

---

## 2. Goals, Non-Goals, and Anti-Goals

### Goals

1. A shipped executable that boots to the faithful attract, opens a menu on Start, and reaches Arcade, Local versus, Training, Settings and the faithful 1987 cabinet, with every fight running on the real 1987 engine under the active ruleset preset.
2. Any of the twelve tournament fighters playable in Local and Training, and Ryu or Ken in Arcade, selected on the portrait select screen the user already chose, with the four-flag country screen in overhaul presets.
3. Every existing core toggle, the ruleset presets, the roster and ban settings, and the difficulty and round-time DIP values reachable from a settings screen, persisted across launches.
4. Keyboard and gamepad play for two local players, with rebinding in-game.
5. Screens that are pure state machines, unit-tested without a window, so the era picker, World Tour and online lobby of later projects slot in as new screens rather than new plumbing.
6. The faithful path stays byte-identical: every existing comparison test passes unchanged, and the 1987 menu item boots the real cabinet with coins, a DIP screen and the service test mode.
7. A ladder report: every boss driven through the whole one-player game as the player, with the first untranslated branch per stage written down, so the SF1+ project starts from a map rather than a guess.

### Non-Goals

- New sprite or audio art of any kind. The twelve bosses keep their 1987 move sets in this project.
- Engine changes of any kind. The Arcade stand-in rule (the other fighter of a country stands in when the player is one of its two) is recorded here for the SF1+ project and not built now.
- Era styles, era gauges, SF1+ move sets, the twelve endings, World Tour, Dramatic Battle, online play, the jukebox and the asset pipeline. Their menu entries appear greyed where the user named them, so the tree is complete, but they do nothing until their own projects land.
- The Pneumatic and Modern control schemes. They need engine work (hold-strength under positive edge, and synthesised motions for one-button specials) that belongs with the SF1+ project. Their rows appear greyed under Controls.
- Infinite health or a frozen timer in Training. Those are engine gates for a later ticket.
- A CRT or scanline filter. Video settings are window scale and borderless fullscreen only.
- Changing or removing anything in the development launcher. It keeps its fixtures, recorders and listening kits.
- Mouse support. The front end is stick-and-buttons, like the cabinet.
- Persisting the 1987 ranking table across launches. A cabinet lost it at power-off; so does this.

### Anti-Goals (what would make this a failure even if it works)

- Any change to what the faithful machine does with the toggles off. The attract, the 1987 item and every comparison fixture must produce the same bytes as before.
- Menus that read as a PC launcher: system fonts, dialog boxes, dropdowns, a mouse cursor. Every screen is 384×224, drawn from the ROM's own tiles, in the header colours, gold frame and yellow cursor of COUNTRY SELECT and the portrait select.
- A shipped game that depends on anything under the gitignored reference folders: checkpoint captures, MAME output, crops.
- Writing 1987 work RAM by hand to "jump" into a fight or to fake a training mode. Every fight is reached by driving the translated flow with inputs, so nothing is guessed.
- A second copy of the window, audio or bindings code that drifts from the dev launcher's.

---

## 3. Constraints

- **Resolution and tiles.** 384×224 at an integer scale. Text comes from the ROM's 8×8 text-layer font and the header tiles the translated flow already draws; the frame, cursor and portrait grid come from the compositor's existing select renderer.
- **The 1987 flow is the only way into a fight.** The engine has no entry point for "start a fight as Sagat in Japan" and must not get one. The coin and Start inputs that reach country select are known from the verified full-game scenarios (coin at frame 1500 opens the start screen at 1514; Start at 1520 selects game mode 2 and opens country select at 1865; the fight begins at 2389).
- **Roster binding is read by the engine, not pushed.** `MatchConfig.roster` (`full_cast`, `four_country`, `p1`, `p2`, `ban`, `mode`) is read at the country-select branch and in the fight input dispatch. The front end sets it on the machine before driving the cabinet and never touches fighter state. The engine never reads `roster.country`; the country is chosen on the real four-flag screen.
- **No engine changes.** Everything in this project lives in the new crates, the new `front` module, the compositor, and the host glue. The engine crate gains screens and a driver that only call what it already exposes.
- **ROMs stay local.** The game loads the user's own ROM set from a folder it is told about, and shows a clear screen when the set is missing or fails the manifest.
- **Host libraries.** The window is `minifb`, audio is `cpal`, both already in the tree. Gamepads add one crate. Windows is the first target; nothing chosen may be Windows-only in principle.
- **Determinism.** Menus never run while a fight steps, and a fight's `MatchConfig` is fixed from the pre-fight until the match ends, except in Training.
- **Rendering.** Fights render through the faithful hardware path plus the diagnostic overlay when it is on. The era-gauge strip that the overlay module paints for non-faithful presets is not used; era gauges are a non-goal here.

---

## 4. Approaches Considered

### Approach 1: Extend the development launcher

Add a default game mode to `sf-dev` beside its fixtures, recorders and listening kits. Fastest start and one binary. Rejected because the shipped game would carry 2,900 lines of dev tooling with relative paths into the gitignored reference folders, the menus would live inside window-loop code with no test seam, and every dev subcommand would become a support surface.

### Approach 2: A new game binary and a pure menu layer (selected)

A new crate holds the executable: window, audio, keyboard and pad input, and a stack of screens. The menus, settings pages and select flow are pure state machines in the engine crate, next to the character-select screen that already works that way, rendered through the compositor and unit-tested without a window. The dev launcher's audio and bindings code moves into a shared host crate so both binaries use it. More files up front and one refactor of shared glue, but a clean executable and testable screens that later projects extend.

### Approach 3: Menus as tasks inside the 1987 machine

Hook the attract's start check and draw menus with the ROM's own text routines on the translated task scheduler. Most authentic in theory. Rejected because it puts overhaul UI on the byte-identical path, budgets menu state out of the 64 KB work RAM, and makes every screen 68000-shaped, against ADR-0002.

**Shared by every approach, and kept:** fights are reached by driving the faithful flow with synthetic cabinet inputs while the screen is hidden, never by writing fight state.

The user chose Approach 2 on 2026-11-02.

---

## 5. Detailed Design

### 5.1 Crates and modules

| Piece | Where | What it is |
| --- | --- | --- |
| Game executable | `crates/sf-game` (new) | `main.rs` opens the window and audio, loads settings and the ROM set, builds the screen stack, and runs the 60 Hz loop. No game logic. |
| Host glue | `crates/sf-host` (new) | The window helper, `audio.rs`, `bindings.rs` and the new gamepad reader, moved out of `sf-dev` and used by both binaries. `sf-dev` keeps its behaviour and drops its private copies. |
| Screens | `crates/sf-engine/src/front/` (new module) | Pure state machines: `attract`, `main_menu`, `single_player`, `multi_player`, `settings` (one submodule per tab), `select` (wrapping the existing `CharacterSelectScreen`), `cabinet_1987` (the DIP screen), `pause`, `after_match`, `rom_missing`. Each takes `MenuInput` events and returns a `Frame` plus a transition. No window types, no I/O. |
| Cabinet driver | `crates/sf-engine/src/front/cabinet.rs` | Turns a chosen mode and fighters into the exact cabinet-input sequence that takes a running `PowerOnMachine` from the attract to the country-select screen, steps it hidden, and reports where it stopped. |
| Text and frames | `crates/sf-video/src/compositor.rs` | A text helper that draws a string from the ROM's 8×8 font tiles at a tile position in a given palette, a header helper using the tiles the translated COUNTRY SELECT draws, and a frame helper for the gold border. The existing portrait renderer gains a second cursor and greyed tiles. |
| Settings file | `%APPDATA%\sf1-port\settings.toml` (new) | Video, sound, ROM folder, the 1987 item's DIP words, and the last-used menu positions. Gameplay and roster stay in `match_config.toml`; bindings stay in `bindings.toml`. |
| Ladder report | `docs/reference/boss-ladder-report.md` (new, generated) | What the ladder probe (section 5.8) found for each boss as the player. Input to the SF1+ project. |

The window title is `Street Fighter`. The edition on screen is the preset line at the bottom of every framed screen (`DEFINITIVE OVERHAUL`, `FAITHFUL 1987`, and so on), following the edition lineage in `CONTEXT.md`. *My call.* (Dropped on 2026-11-07; see section 11.)

### 5.2 The screen map

Solid items are the user's; the rest were proposed on 2026-11-02 and accepted.

```
Attract (faithful machine, toggles off)
  └─ START ─► Main Menu ─┬─ Single Player ─┬─ Arcade Mode ─► Select (Ryu, Ken) ─► Country (1987 screen) ─► Fight ─► ladder
                          │                 ├─ Training     ─► Select (all twelve) ─► Country ─► Fight (live preset, tools)
                          │                 ├─ World Tour      (greyed)
                          │                 └─ Dramatic Battle (greyed)
                          ├─ Multi Player ──┬─ Local ─► Select (all twelve, two cursors) ─► Country (P1 picks) ─► Fight
                          │                 ├─ Online          (greyed)
                          │                 └─ Local co-op Dramatic (greyed)
                          ├─ Settings ─ tabs: GAMEPLAY · ROSTER · VIDEO · SOUND · CONTROLS · (NETPLAY greyed)
                          ├─ 1987 ─► 1987 CABINET (DIP screen) ─► POWER ON ─► the faithful cabinet; held Esc returns
                          └─ Exit
Fight ─► Pause (Space or pad Start) ─► Resume · Overlays · Restart round · Quit to menu
Fight ends ─► After match: Local: Rematch · Reselect · Menu. Arcade: the ROM's continue, game over, name entry and ranking play; then Menu.
```

- **The attract runs the faithful machine.** The screen shows the real 1987 warning, title, demo fight and ranking, frame for frame, with every toggle off. Any Start press (keyboard, either pad) opens the Main Menu; the attract machine is paused behind it and resumes when the menu is left, so the demo picks up where it was.
- **No coins in overhaul play.** The front end never shows INSERT COIN as a prompt and never needs a coin key. The 1987 item is the only place credits exist.
- **Greyed items** are drawn in the dim blue of the preset line and skip the cursor. They are present so the tree the user described is complete and so later projects only turn an item on.
- **Faithful 1987 as the active preset** in the overhaul menus behaves as the 2026-10-27 design says: only Ryu and Ken are selectable (ten tiles greyed), Local is Ryu against Ken, and country select is the two-country screen the DIP sets. The preset line reads `FAITHFUL 1987`. Nothing is forced on.

### 5.3 Driving the cabinet into a fight

When a mode starts, the game:

1. Takes the attract's `PowerOnMachine`, sets its `MatchConfig` to the active preset with `roster` filled from the select (`mode`, `p1`, `p2`, `full_cast`, `four_country`, `ban`), and the DIP words for difficulty and round time from the Gameplay tab.
2. Feeds the sequence the cabinet driver produces: a coin pulse, then a Start pulse (`start1` for one-player modes; `start1` then the `start2` join for Local, matching the verified two-player scenario). The driver steps the machine without rendering until the country-select task is reached or a frame budget runs out.
3. Shows the real country-select screen and hands the stick to the player. P1 picks the flag in every mode, as the 1987 two-player game does; the stage follows the flag. From here the translated flow runs visibly through the pre-fight into the fight.

Local versus fights run as the 1987 two-player versus; Arcade and Training run as the 1987 one-player game. Training is a one-player game against the chosen CPU opponent, or a versus game against an idle dummy on the P2 side; the choice is on the Training screen. Training adds nothing to the engine: it is the fight, the overlays, the pause box, snapshot and rewind, and the live preset row.

If the machine hits an untranslated branch during the hidden frames or during a fight, the game shows a one-line notice in the ROM font, returns to the Main Menu, and writes the branch addresses to the log, as the dev launcher does. Nothing is guessed or skipped.

### 5.4 Arcade

- Arcade offers Ryu and Ken in this project. The other ten tiles are greyed with the caption `SF1+` so the player knows why. The 1987 ladder, country order, bonus stages, flights, continue and ending run exactly as translated, with the chosen fighter on the human side as the twelve-fighter binding already does for Ken.
- Continue is free: the ROM's countdown plays and any Start continues. Letting it run out plays the ROM's game over, ranking check, name entry and ranking screen as translated, and the front end returns to the Main Menu when that flow would have returned to the attract. The ranking table lives in the attract machine for the session, as it would on a cabinet left switched on.
- Ban lists from `RosterSettings.ban` grey out banned fighters on the select screen when the preset is Classic Competitive.
- **For the SF1+ project, recorded here and not built now:** the stand-in rule (when the player is one of a country's two fighters, the other stands in for the player's slot) as an overhaul-only gate on the opponent-load path in the task translation, tested like the four-country gate; and whatever the ladder report lists.
- **Mirror matches** in Local are allowed only if the ladder probe proves the same fighter on both sides runs with no untranslated branch; otherwise P2's cursor skips P1's fighter. *My call.*

### 5.5 The screens, in order of appearance

**Main Menu (framed, option B).** Header `MAIN MENU`, gold double frame, five rows with the yellow cursor, the preset line at the bottom. Up and down move, confirm enters, back returns to the attract. (Since 2026-11-07: no header, the frame unbroken, the 1987 title logo at the top and no preset line; see section 11.)

**Single Player / Multi Player.** Same frame, header is the parent item, rows are the children. Greyed rows as above.

**Select (portrait).** The screen the user chose: large bust left, twelve tiles in three columns right, yellow cursor, gold frame, the fighter's name under the bust. For Local, P2's cursor is the compositor's blue, both players move at once, and the bust panel shows whichever cursor moved last. For Arcade and Training, only P1 picks and the CPU side is implied. Busts are decoded at startup from the ROM's own VS-screen tiles, so no image files ship; a bust that fails to decode shows the name, as today. Tiles that the mode, preset or ban list excludes are greyed and skipped.

**Country.** The 1987 country-select screen as the translated flow draws it, four flags in overhaul presets and two under Faithful 1987. The player picks with the stick. It is not redrawn by the front end.

**Settings (tabs, option S1).** Header `SETTINGS`, the five tab names across the top under the frame, the current tab underlined in yellow, shoulder keys or Q/E switch tabs. Rows are `NAME ........ VALUE` with left and right changing the value, saved on leaving the screen.

| Tab | Rows |
| --- | --- |
| GAMEPLAY | `PRESET` (cycles the presets; any edit below flips it to CUSTOM, as the config already does); the eleven core toggles by their display names; the three frame counts (`RECOVERY BUFFER 4F`, `COMMAND WINDOW 28F`, `CPU REACTION 8F`); `DIFFICULTY` and `ROUND TIME` as the 1987 DIP values. |
| ROSTER | `FULL CAST`, `FOUR COUNTRIES`, `BAN LIST` (Unrestricted, Standard Tournament, Low Tier), `WHAT-IF STYLES`, `BONUS CHALLENGERS`, `SECRET RIVALS` (the last three greyed until their projects). |
| VIDEO | `SCALE` 1× to 4×, `FULLSCREEN` (borderless at the monitor size, integer-scaled and centred), `VSYNC` only if the window library exposes it. |
| SOUND | `MASTER`, `FM` (YM2151), `SAMPLES` (both MSM5205s), each 0 to 10. `JUKEBOX` opens the Jukebox screen (ST10; stages and music design, section 14). |
| CONTROLS | `P1 DEVICE` / `P2 DEVICE` (keyboard, pad 1, pad 2), `P1 SCHEME` / `P2 SCHEME` showing `CLASSIC` with Pneumatic and Modern greyed, then `REBIND P1`, `REBIND P2` and `REBIND MENU` rows that open the rebind flow: each logical input in turn, press the key or pad button, Esc keeps the old one. |
| NETPLAY | Greyed tab. |

Changing `PRESET` applies that preset's toggles and roster defaults (`full_cast`, `four_country`, `ban`) as the config does today. The fighters chosen on the select screen are not part of any preset: the front end stamps `p1` and `p2` onto the config after every preset change and before every drive, so a preset change never retargets a chosen fighter.

**Pause.** A small framed box over the dimmed fight: `RESUME`, `OVERLAYS` (cycles off, hitboxes, frame meter, both), `RESTART ROUND` (restores the round-start snapshot), `QUIT TO MENU`. In Training only, a `PRESET` row cycles the live preset among the overhaul presets; Faithful 1987 is excluded from the live cycle because it unbinds a boss on the human side mid-fight. Pausing stops the machine; no frames step while the box is up.

**After match.** Local: `REMATCH`, `RESELECT`, `MENU` over the final frame. Arcade: nothing of the front end's until the ROM's game-over flow has played out; then the Main Menu.

**1987.** Opens `1987 CABINET`, a framed screen listing the cabinet's DIP settings as rows (difficulty, round time, coinage, continuation, buy-in, flip screen, demo sounds, attract music, country count, test mode) with `POWER ON` at the bottom. Power on boots a fresh faithful `PowerOnMachine` with `MatchConfig::faithful()`, those DIP words, the cabinet keys live (coins, starts, service), and nothing from the overhaul settings except video, sound and bindings. Holding Esc for one second returns to the overhaul attract. This is the only way to leave it. The DIP words persist in `settings.toml`.

**ROM missing.** If the ROM set is not found or fails the manifest, the game opens on a framed screen in the compositor's ROM-independent fallback font stating the folder it looked in and what it expected, and offers `EXIT`. *My call:* the game looks in `roms\sf` beside the executable first, then `%APPDATA%\sf1-port\roms\sf`, then the folder named in `settings.toml`.

### 5.6 Input

- **Logical inputs** stay as today: four directions and six buttons per player, plus cabinet keys for the 1987 item. Menus add `MENU_CONFIRM`, `MENU_BACK`, `MENU_START` and the tab-switch pair, with defaults that reuse LP for confirm and LK for back on the fight bindings, Enter and Backspace on the keyboard, and the pad's face buttons.
- **Gamepads** come through `gilrs` 0.11 (pure Rust). Its default Windows backend is Windows Gaming Input, which enumerates HID controllers that have no XInput driver as raw game controllers and maps them through the SDL controller database; the XInput backend is an opt-in feature and is not used. The user's pads are DualShock 4s, which Windows Gaming Input reports as raw game controllers, so they are expected to work over USB without DS4Windows or Steam Input, with no rumble (the cabinet had none) and only while the game window has focus. The host smoke check in the first ticket confirms this on the user's own pads; if a pad fails to enumerate, the swap to SDL2 is confined to the host crate. Bluetooth pairing is untested. *My call.* Each pad maps to `P1` or `P2` by the Controls tab or, by default, by connection order. A pad disconnecting mid-fight pauses the fight.
- **Control schemes.** Classic is the only scheme wired in this project. The `ControlSession` code stays where it is for the SF1+ project to pick up.
- **Keyboard** remains the layout-independent physical US layout the dev launcher uses, with the same binding file.

### 5.7 Persistence

| File | Holds | Owner |
| --- | --- | --- |
| `match_config.toml` | preset, core toggles, roster, difficulty and round time | existing `MatchConfig` |
| `bindings.toml` | keyboard bindings per player and cabinet; gains pad bindings and menu inputs | existing `BindingsTable`, extended |
| `settings.toml` | video, sound, ROM folder, the 1987 item's DIP words, last menu positions | new |

All three live under `%APPDATA%\sf1-port\`. Any file that fails to parse is reported on the log and replaced by defaults without crashing.

### 5.8 Testing

- **Screens** are tested the way `CharacterSelectScreen` is: build the screen, feed `MenuInput` events, assert the rows, the cursor, the transition, and the frame's pixels at known tile positions. No window.
- **The cabinet driver** is tested on a real `PowerOnMachine` from power-on: for each mode, drive to the country-select task and assert the task position, the game mode byte, and that `unrecovered()` is empty. Then for Ryu and Ken in Arcade, and for each of the twelve in Local and Training, pick a flag and run to the pre-fight, asserting the bound move table on the human side and a clean first round.
- **The ladder probe** is a report, not a gate. For each of the ten bosses as the player, a scripted scenario drives the whole one-player game from power-on under the Definitive Overhaul preset and records the first untranslated branch, if any, by stage, into the ladder report. It also records whether each mirror match runs. It never fails the build.
- **Faithful non-regression:** every existing comparison test runs unchanged; the new attract wrapper is checked against the power-on scenario for a full attract loop with no toggles.
- **Host smoke** extends the existing check to the new binary: window, audio device, keyboard, pad enumeration, settings round trip, and ROM lookup in each of the three folders.
- **Determinism:** pausing and resuming a fight at frame N yields the same state as not pausing, using the snapshot comparison the replay tests already do.

---

## 6. What Is Left to the Builder

- The exact file split inside `front/` and whether the settings tabs share one row-editing type.
- Which ROM tile indices hold the 8×8 font, the header glyphs and the VS-screen busts, and the fallback when a header glyph is missing from the ROM's set.
- The precise cabinet-input frame offsets for each mode, taken from the verified scenarios, and the hidden-frame budget.
- The gamepad crate's event-to-logical mapping and the default pad layout.
- How the dimmed fight is drawn under the pause box (a palette darken is enough).
- Log file location and format, and the ladder report's layout.
- Whether `sf-host` also absorbs the dev launcher's `present` and window helpers in the same move or a later one.

---

## 7. Decisions recorded here that the user did not make

These are my calls, listed so they can be checked while reading:

1. Window title `Street Fighter`; edition shown as the preset line.
2. The country is picked on the real 1987 screen, by P1 in every mode, and the stage follows it. `roster.country` is not used.
3. ROM lookup order: beside the executable, then app data, then the settings file.
4. The attract machine pauses behind the menu rather than restarting.
5. Training is the fight plus the tools (overlays, pause, snapshot, rewind, live preset), against a chosen CPU or an idle versus dummy. No frozen health or timer.
6. Difficulty and round time are exposed as Gameplay rows and reach the engine as the 1987 DIP words.
7. `gilrs` with its Windows Gaming Input backend for pads, confirmed on the user's DualShock 4s by the first host smoke check; a disconnect pauses the fight.
8. Only the Classic control scheme is wired. Pneumatic and Modern wait for the SF1+ project.
9. A preset change keeps the chosen fighters: the front end re-stamps `p1` and `p2` after every preset change. Training's live cycle excludes Faithful 1987.
10. With Faithful 1987 active in the overhaul menus, only Ryu and Ken are selectable and the country screen is the two-country one. Nothing is forced on.
11. Arcade lets the ROM's game over, name entry and ranking screen play before returning to the menu. The ranking is not persisted across launches.
12. The 1987 item opens a DIP screen before power-on, with test mode among the rows.
13. Fights render through the faithful path plus the diagnostic overlay; the overlay module's era-gauge strip is not used.
14. Greyed items for the modes, fighters and schemes that do not exist yet, rather than hidden.
15. A held Esc is the only exit from the 1987 item.
16. Mirror matches in Local follow the ladder probe's finding.
17. The ladder probe is a report that never fails the build, and the stand-in rule is written down for the SF1+ project instead of built here.

---

## 8. Questions resolved with the user after the builder check

1. **Arcade with a boss as the player.** Decided 2026-11-02: Ryu and Ken in Arcade for this project; all twelve in Local and Training; Arcade opens to the rest with the SF1+ project. The ladder probe still runs as a report.
2. **Pads.** The user's only non-keyboard inputs are DualShock 4 pads. `gilrs` stands: its default Windows Gaming Input backend reports the DualShock 4 as a raw game controller and maps it through the SDL database. The first host smoke check confirms it on the real pads; SDL2 is the fallback, confined to the host crate.

---

## 9. Settled in the grilling (2026-11-02)

Three rounds of questions after approval. Every answer below is the user's; where it changes something above, this section wins.

### Round one

1. **Free play text.** The attract machine stays byte-identical; the front end composites `FREE PLAY` in the ROM font over the `INSERT COIN` position, in overhaul play only.
2. **Menu sounds.** Two of the ROM's verified FM effects, one for cursor movement and one for confirm, through the same mixer. Nothing else on menus; the attract's sound stops with its machine.
3. **Transitions.** Hard cuts everywhere.
4. **Esc.** Back on every overhaul screen; opens the pause box in a fight; never quits. The only quit is the Exit item. A held Esc exists only inside the faithful cabinet.
5. **Who drives menus.** Any player's inputs, merged into one cursor. Two cursors only on the Local select.
6. **Attract snapshot.** Taken before a drive. Local Rematch keeps fighters and country and replays the flag hidden; Reselect returns to the portrait select.
7. **Free continue.** The continuation DIP is forced to allowed in overhaul Arcade and a coin pulse is injected when the continue screen opens. The ROM's continue limit still applies.
8. **Shipping shape.** One executable with the manifest embedded; the player supplies `roms\sf` beside it. No installer.
9. **First-launch defaults.** Scale 3×, windowed, all volumes 10, Definitive Overhaul, keyboard for both players until a pad is seen.
10. **Training screen.** `OPPONENT`: `CPU` or `DUMMY`, then the portrait select for the player and, for `CPU`, the opponent.
11. **Pause in Local.** Either player pauses or resumes; `QUIT TO MENU` acts at once.
12. **Vocabulary.** Front end, Attract, Play mode, Local, Faithful cabinet, Cabinet driver, Screen, Ladder, Stand-in, Dummy, Preset line, Free play: in `CONTEXT.md`.
13. **ADR-0004.** Fights are reached by driving the faithful flow; the attract is the faithful machine.
14. **ADR-0005.** Separate game binary, pure screens, shared host crate.

### Round two

1. **Two cursors.** Both may rest on one tile; confirm locks a tile and stops that cursor; back un-confirms; both confirmed moves on at once.
2. **Arcade select.** The ten boss tiles greyed with the caption `SF1+`, cursor skips them. Banned or unavailable tiles greyed with no caption.
3. **Pad and keys.** DualShock 4: Cross confirms, Circle back, Options is Start, L1/R1 switch tabs. Keyboard: Enter, Backspace, Q and E. Fight bindings on menus: LP confirms, LK back.
4. **Settings wording.** `ON`/`OFF`; `4F`; `DIFFICULTY` 1 to 4; `ROUND TIME` by the DIP's values; a `RESET TAB` row ends every tab.
5. **Rebind flow.** Walks every logical input in order; a press binds; Esc keeps; a duplicate within the same player is refused with a notice. Menu inputs the same under `REBIND MENU`.
6. **DIP screen.** Coinage A, coinage B, continuation, buy-in, difficulty, round time, flip screen, demo sounds, attract music, country count, test mode; factory defaults; `POWER ON`; test mode on boots into the service screen.
7. **Host crate.** Audio, bindings, window open and present helpers, and the pad reader move to `sf-host`; the launcher imports them unchanged and gains pads.
8. **Start timing.** Honoured from the first frame after the RAM clear, at any point of the attract.
9. **Which machine each play mode keeps.** Arcade keeps the machine through the ROM's game-over flow, then the menu. Local and Training stop at the match end, before the winner-carries-on branch, show the after-match box, and restore the attract snapshot on leaving. This supersedes any reading of 5.2 that restores after Arcade.
10. **Ladder report.** An ignored-by-default integration test writes `docs/reference/boss-ladder-report.md`, committed.
11. **DIP rows reach every fight.** Difficulty and round time are stamped before every drive; the attract keeps factory values.

### Round three

1. **ROM-missing screen.** Fallback font; the set name from the manifest; the three folders tried; `RETRY` and `EXIT`.
2. **Log.** `%APPDATA%\sf1-port\sf-game.log`, overwritten per launch: settings in force, ROM folder and manifest result, pads seen or lost, settings files that failed to parse, untranslated-branch notices with frame and addresses.
3. **Esc hint.** `HOLD ESC ONE SECOND TO RETURN` on the DIP screen, and a small hold bar drawn over the cabinet while Esc is down, as a presentation overlay with the machine untouched.
4. **Hidden drive.** Black for the span, then a cut to country select.
5. **Close button.** Quits at once.
6. **Pad disconnect.** In Arcade, Local and Training the fight pauses with the notice `P2 PAD DISCONNECTED` (or `P1`) and that side idles until a pad is seen again; the keyboard does not take over. Online fights never pause on a disconnect; that rule is recorded here for the netplay project.

---

## 10. Settled while building

Found while implementing the tickets. Each refines a section above; none changes an ADR's rule.

### FE4 (Local into a real fight)

1. **Each fight powers on its own machine.** Boot decodes DSW2 once (`0x3c132..0x3c184`: difficulty, round time, continuation, buy-in, countries), so the Gameplay tab's difficulty and round time can reach the attract's running machine only by writing RAM, which ADR-0004 rules out. So every fight powers on a fresh faithful machine with the play mode's DIP words and configuration, and the cabinet driver carries that machine to the country select with coin and Start pulses. The attract's machine keeps the factory words and is never driven: it waits, paused and untouched, behind every fight, so leaving a fight finds it exactly as it was (the snapshot-equality test). This refines section 5.3 step 1 ("takes the attract's `PowerOnMachine`"). ADR-0004's rule, cabinet inputs only and no RAM writes, holds. The drive takes 32 hidden frames from the attract for one player and 62 for two. From power-on it takes 812, because a coin during the warning screen is only credited at the join (frame 794).
2. **The two-player start** is the verified `sf-start-2p-01` sequence (`coin1`, `coin2`, `start2`, `GAME_MODE = 1`), not `start1` followed by a `start2` join, which is the challenger path. The driver feeds exactly the coins it needs, then pulses Start once they are credited.
3. **The match-end stop** is the game flow waiting on the versus match wipe (or, with no winner, the game-over wipe), just before `0x3ee7a` / `0x3eeac` carry the winner on against the CPU. The front end reads it through a read-only `PowerOnMachine::versus_match_over`. Fights render through `PowerOnMachine::render_faithful_with_overlay` (the hardware frame plus the diagnostic overlay, never the era-gauge strip). Neither changes engine behaviour.
4. **Sound across the hidden drive.** The hidden frames cannot sound, so the black frames play a stop (`0x00`, then `0x80`), then the sound commands the drive latched once Start was pressed (the country select's tune `0x34`, say), one per frame.
5. **Win screens show the original side's art.** After a Local match the win screen shows the 1987 side's portrait and quote (Ryu's for P1, Ken's for P2), because the twelve-fighter binding binds move tables only. This is for the art pipeline and SF1+ projects, not the front end.
6. **Scripted players for the tests.** Sagat, bound on P1, walking forward with heavy kick never landed a hit on a standing Gen in 20,000 frames. Standing and cycling the six buttons did. Why was not investigated, so the tests' scripted players walk in, then stand and press buttons.

### FE5 (the portrait select finished)

1. **Where the busts are.** They are background-layer tiles (gfx1, tile ROM base 0) on the event-4 VS screen (`0x3f9ec`), not foreground tiles on the event-3 country screen as the 2026-11-02 handoff said. `sf_video::busts` decodes all twelve from the ROM's VS scroll and palette tables. They equal the native machine's VS screens and the MAME captures pixel for pixel (`crates/sf-engine/tests/vs_busts.rs`).
2. **The select's style (user, 2026-11-02).** The select is framed like every other front-end screen: the gold double frame, `PLAYER SELECT` in the ROM's header font, names and captions in the ROM's 8×8 font, ROM-palette borders (P1 yellow, P2 blue), and the preset line. This replaces the 3×5 fallback font of the earlier portrait renderer, which only the development launcher's `select` subcommand still uses.
3. **No resampling (user).** Pixel art is drawn at integer scale only, so the busts are drawn 1:1. The large frame holds a whole bust (each is at most 98 pixels square), and a tile shows a 1:1 window onto the bust's face. The window's own upscale stays integer.
4. **One facing direction (user).** The VS screen draws the player's bust on the left facing right and the opponent's on the right facing left. On the select, every bust faces left: Ryu's, the only left-side bust, is mirrored.
5. **Local shows both busts (user).** Local is laid out like the VS screen. P1's bust is on the left, mirrored to face P2. P2's bust is on the right. The names and `VS` sit between them, and the twelve tiles run in two rows of six below. Arcade keeps the mockup's arrangement: one large frame on the left, three columns of tiles on the right.
6. **Inputs.** Each side drives its own cursor on the Local select: P1's stick, LP and LK, plus the menu keys and `start1`; P2's stick, LP and LK, plus `start2`. Every other screen merges both players into one cursor.
7. **Faithful 1987.** Without the full cast the 1987 game binds Ryu to P1 and Ken to P2 whatever the select says. So under Faithful 1987, P1 may pick only Ryu and P2 only Ken; the select shows only what will be played.
8. **The mirror rule** is `MIRROR_NOT_CLEAN` in `front/select.rs`, taken from the ladder report: only Joe, whose crouch reaches `0x44506`. In Local, neither side may land on or lock the fighter the other side has locked if that fighter is on the list.

### FE7 (pause box and Training)

1. **Training against a CPU offers four opponents.** The 1987 one-player game chooses its CPU by the starting country, and the front end changes no engine behaviour, so Training can only offer the opponent each flag brings first. Those are the CPU move tables `0x16e6c`, `0x175bc`, `0x1734c` and `0x17a9c` for flags 0–3, the same on all four difficulties (`front_end_training.rs`). The driver picks that flag hidden, and the other tiles are greyed with `SF1+`. A free choice of CPU is for the SF1+ project. Training then stops at the match end like Local, whoever wins.
2. **Opening the pause box.** Keyboard Start (Space), Esc or Backspace, either cabinet start, or a pad's Start opens it. No attack button does. It does not open during the hidden drive, nor on Arcade's continue countdown, where Start continues. Arcade and Local get the same box without the `PRESET` row.
3. **The frame the box closes on** shows the fight without stepping it. That way a restarted round begins exactly at its snapshot, and a resumed fight equals one that never paused.
4. **`OVERLAYS`** cycles the diagnostic overlay's two halves: off, hitboxes (fighter and projectile boxes), frame meter (the input panels and the 60-cell meter), both. `render_diagnostic_overlay_parts` in `overlay.rs` draws them; a presentation change only.
5. **`PRESET`** cycles Definitive Overhaul, Classic Competitive and Pure Footsies, keeping the play mode, the two fighters and the full cast. It never offers Faithful 1987.

### FE8 (settings and persistence)

1. **Difficulty and round time** are two `MatchConfig` fields, saved under `[dips]` in `match_config.toml` as section 5.7 asks. The engine never reads them; the front end powers each fight on with them as DSW2 bits. A file written before `[dips]` loads with the factory values (2 and 100). No preset changes them.
2. **The Gameplay rows** are every core toggle the configuration has, by display name: fourteen `ON`/`OFF` rows and four frame counts (`RECOVERY WINDOW 4F`, `COMMAND WINDOW 28F`, `RELEASE GRACE 8F`, `CPU REACTION 8F`). Section 5.5's "eleven" and "three" undercounted them. Then `DIFFICULTY` and `ROUND TIME`. The tab scrolls, 16 rows at a time.
3. **The Settings frame spans the full screen width**, so the six tab names (`GAMEPLAY ROSTER VIDEO SOUND CONTROLS NETPLAY`, 44 cells) fit on one line.
4. **Volumes.** `GameSound::sample_at_gains` scales the YM2151 route (`FM`) and the two MSM5205s (`SAMPLES`) separately, with `MASTER` scaling both. At full volume the host uses the mixer's own `sample_at`, unchanged.
5. **Video.** `sf_host::window::GameWindow` draws at an integer scale only. `FULLSCREEN` is a borderless window at the monitor's size (`GetSystemMetrics`), using the largest integer scale that fits, centred on black. Scale and fullscreen apply when Settings is left (the window is reopened). Both files are written then too.
6. **Controls.** The device rows show `KEYBOARD` and the scheme rows `CLASSIC` until pads arrive (FE9).
7. **Last menu positions** (section 5.7, added after FE12) are saved in `settings.toml` as `main_menu`, `single_player_menu`, `multi_player_menu` and `training_menu`, so each of the four menus opens on the row it was left on. A saved row that is greyed or missing opens on the first live row instead. Moving the cursor does not write the file. The game saves the positions when it ends, and with any other save.

### FE10 (the faithful cabinet)

1. **The DIP rows** are the eleven settings `docs/reference/dip-switches.md` verifies, each with its own options and the factory one first. The factory words are `0xdfff` (DSW1) and `0xffff` (DSW2). They are saved in `settings.toml` as `cabinet_dsw1` and `cabinet_dsw2` on leaving the screen and on `POWER ON`. The rows are checked against boot's own decode (`0x3c132..`) in `front_end_cabinet.rs`.
2. **The cabinet** is a fresh `PowerOnMachine` with those words and `MatchConfig::faithful()`. Both players' sticks and buttons and the five cabinet keys (coin 1, coin 2, start 1, start 2, service 1) go to it as they are. The menu Esc key reaches nothing in the cabinet; it only counts towards leaving.
3. **The hold** is 60 frames of Esc. The bar is 64×6 pixels at (8, 8): a gold outline filling with yellow. It is drawn on the presented frame only. The test runs a 6,000-frame attract loop with Esc held 40 frames in every 100. Work RAM equals a plain power-on machine's on every frame, and every pixel outside the bar equals the plain machine's frame.
4. **Leaving** clears the menu stack and returns to the overhaul attract, which was paused behind the cabinet and resumes as it was (snapshot equality). The attract is held for two frames while the stop commands (`0x00`, `0x80`) play. A branch the translation lacks abandons the cabinet the same way a fight is abandoned.
5. **Sound at power-on.** The host's sound board is not reset when the cabinet powers on. The main menu has already played the stop commands, and `POWER ON` drops the confirm sound, so the cabinet starts in silence.

### FE11 (shipping shape)

1. **The manifest check.** A folder holds the set when every file the embedded manifest lists is there, at its size, with its CRC-32 (`sf_video::romset::check_set`; 44 files). The error names the first file that fails, and the ROM-missing screen shows it under that folder. Extra files in the folder are ignored.
2. **The ROM-missing screen** is `front::rom_missing`, drawn with the compositor's 3×5 fallback font. The font gained `:`, `\`, `_`, `(`, `)`, `,`, `'` and `?` for paths, and draws any other character as `?`. It shows the title at 2×, the set's name and description, each folder with its problem (a long path keeps its end), and `RETRY` and `EXIT` at 2×. Confirm or Start chooses; Back does nothing (there is no screen behind it). `RETRY` looks in every folder again and, once one matches, goes straight into the attract.
3. **The folders** are `roms\sf` beside the executable, `%APPDATA%\sf1-port\roms\sf`, and the settings file's `rom_folder`. When `rom_folder` is not set, the screen and the log say so on its line. `--roms <folder>` (a development aid) is looked in first.
4. **The log** is `%APPDATA%\sf1-port\sf-game.log`, created empty at every launch. Each line is the seconds since launch, a kind (`settings`, `roms`, `pads`, `parse`, `untranslated`) and the text, flushed at once, and also written to stderr. `settings` lines record the settings in force at launch and again on every save, and each file written with its defaults. `roms` lines record each folder that failed and the one that matched. `untranslated` lines are the session's notices.
5. **The three files.** At launch a missing file is written with its defaults, so a first launch writes all three. A file that fails to parse is logged as `parse`, and its defaults are used. It is left on disk untouched until the game next saves that file.

### FE9 (pads and rebinding)

1. **Pad numbers.** `sf_host::pads` numbers pads by connection order, `PAD 1` and `PAD 2`. A pad that connects after a loss takes the first empty number, so a cable pulled and plugged back in returns its side. A third pad is ignored and logged.
2. **`DEVICE` rows** are `AUTO` (the default), `KEYBOARD`, `PAD 1` and `PAD 2`, saved in `settings.toml` as `p1_device` and `p2_device`. `AUTO` is section 5.6's "by default, by connection order": P1 takes `PAD 1` and P2 takes `PAD 2`, and each side is on the keyboard until its pad is first seen. So a first launch with no pad is keyboard for both players, as FE11 asks. Once a side's pad has been seen, a lost pad idles that side until a pad takes the number again. The keyboard never takes over on its own. Devices apply when Settings is left, like the video rows.
3. **The Classic pad layout:** the d-pad; Square, Triangle and R1 for LP, MP and HP; Cross, Circle and R2 for LK, MK and HK. The left stick moves too, whatever the directions are bound to. The pads' menu buttons are Cross (confirm), Circle (back), Options (start), L1 and R1 (tabs). Cross is both confirm and LK, so a pad side's LP and LK never stand in for confirm and back as the keyboard's do (`FrameInputs::pads`).
4. **Pads on the cabinet.** A pad's Options is also its side's cabinet start, and Share is its coin. That lets the 1987 cabinet be played from the pads, and makes Options pause a fight like either cabinet start.
5. **Each side's pad drives its own cursor** on the Local select. The keyboard's menu keys go with P1, or with P2 when only P1 is on a pad.
6. **The disconnect pause.** A lost pad opens the pause box with `P1 PAD DISCONNECTED` or `P2 PAD DISCONNECTED`. During the hidden drive or Arcade's continue, where the box cannot open, it opens as soon as it can. A box already up takes the notice instead. Any device's confirm resumes.
7. **The rebind walk** (`front::rebind`) takes the frame's first press by name (`FrameInputs::press`), never the menu inputs. Esc keeps the old binding. A key is refused when anything else on the keyboard has it (players, menu or cabinet keys), because one keyboard serves both players. A pad button is refused only within the walk's own list, because each side's pad is its own device and the menu's pad buttons share the player's by design. The notice reads `H IS ALREADY P2 RIGHT`. The walk's end saves `bindings.toml`, and `RESET TAB` on Controls restores the default devices and bindings.
8. **The bindings file** gains `[menu]`, `[p1_pad]`, `[p2_pad]` and `[menu_pad]`. A file without them takes their defaults, so older files still load. The pads' names are `CROSS`, `CIRCLE`, `SQUARE`, `TRIANGLE`, `L1`, `R1`, `L2`, `R2`, `SHARE`, `OPTIONS`, `L3`, `R3`, `PS` and `DPAD RIGHT` / `LEFT` / `DOWN` / `UP`.
9. **The development launcher** adds `PAD 1` to P1 and `PAD 2` to P2 in every subcommand (`sf_host::input::LauncherPads`), with no device setting and no change to its subcommands. Pads seen or lost are printed.
10. **A real DualShock 4 (user, 2026-11-02).** A DualShock 4 v2 over USB (`VID_054C&PID_09CC`), with nothing else installed, drove the menus and a fight with this mapping. A pulled cable gave `PAD 1 lost`, and plugging it back in, `PAD 1 seen`.
    - **The fix it needed:** Windows Gaming Input reports pads already plugged in as connection events a moment after it starts (41 ms here), not at once. So the smoke check, which listed pads as the reader opened, printed `pads: none connected`, and the game logged "no pads connected" before seeing PAD 1 on its first frame.
    - `PadReader::open` now gives the backend up to 250 ms to report them before numbering. The smoke check prints `pads: PS4 Controller`, and the game logs `PAD 1 seen: PS4 Controller` at 0.27 s.
    - Two pads at once remain FE13's.

## 11. Changed after shipping

### The Main Menu's logo, and no preset line (the maintainer, 2026-11-07)

The maintainer asked for two changes, in his words:

> I think the Main Menu screen needs a change; I don't like that it says Main Menu at the top, I think I would prefer the board all the way around and have the street fighter logo from the title screen anchored to the top just below the border, and we can move the options lower to make room.

> Also, I don't think we need 'definitive overhaul' at the bottom of the screen anymore, it seems superfluous given all the features.

The maintainer approved a rendered mockup and chose to drop the preset line from every front-end screen, not only the Main Menu.

1. **The Main Menu.**
   - The gold frame runs unbroken all the way round, with no header.
   - The logo is the title screen's own text block, `0x14afc` (`front::attract::TitleLogo`), drawn with the boot text palette at the cell the ROM gives it: 32 × 12 cells from column 8, row 3. That is exactly where the 1987 title screen draws it, and just inside the frame's top edge (row 2). ADR-0006 holds: it is ROM art only.
   - The five rows move down to rows 16, 18, 20, 22 and 24. A notice (an untranslated branch, a failed look) goes on row 25, the one free row under them.
   - The Single Player, Multi Player and Training sub-menus keep their headers and rows.
2. **No preset line.**
   - The menus, player select, stage select, Settings and the Jukebox show nothing at their foot.
   - The active preset is still shown by Settings' `PRESET` row, and in Training by the pause box's `PRESET`.
   - The 1987 cabinet's hold hint and the rebind walk's `ESC KEEPS THE OLD ONE` stay on that row. They are hints, not the preset line.
3. **Tests.** In `front_end_menus.rs`:
   - The Main Menu's logo region equals frame 1500 of the attract (the title screen), pixel for pixel.
   - Every cell of its frame is the unbroken gold frame.
   - On each framed screen, row 25 is black inside the frame.
