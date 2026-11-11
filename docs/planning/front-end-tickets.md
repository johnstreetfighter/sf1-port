# Overhaul Front End tickets

Tracer-bullet tickets for Spec #150, the first of the seven overhaul projects decided on 2026-11-02. Each ticket cuts a complete path from screen to engine to test and is demoable on its own. Blocking edges are published as native GitHub dependencies; the `Blocked by` sections below are the same edges in text. Vocabulary follows the Front end terms in `CONTEXT.md`; the decisions follow ADR-0004 and ADR-0005.

Order of publication (blockers first): FE1, FE3, FE2, FE4, FE5, FE6, FE7, FE8, FE9, FE10, FE11, FE12, FE13.

---

## FE1: Shared host crate (prefactor)

### Parent

Spec #150

### What to build

Move the development launcher's audio output, bindings table and window open and present helpers into a new host crate that both the launcher and the coming game binary use, then make the launcher import them. Nothing a player sees changes. This is the "make the change easy" step for ADR-0005: without it the game binary would need a second copy of the host glue, and the two would drift.

### Acceptance criteria

- [ ] A host crate exists holding the audio output, the bindings table with its TOML round trip and key-name mapping, and the window open and present helpers; the development launcher depends on it and its own copies are gone.
- [ ] Every development-launcher subcommand (versus, cpu, power-on, select, smoke, kit, kit-sound-test, music, record, record-synthetic, convert, verify, replay, preset, rebind) runs as before from the crate directory with the same output.
- [ ] The host smoke check and the bindings round-trip test pass unchanged; the engine, video, audio and compare crates are untouched.
- [ ] The workspace builds clean in release with no new warnings.

### Blocked by

- None (can start immediately)

---

## FE2: The game boots to the attract and the Main Menu

### Parent

Spec #150

### What to build

A new game binary that opens on the faithful attract with every toggle off and `FREE PLAY` composited over `INSERT COIN`. Any keyboard Start, from the first frame after power-on's RAM clear, opens the framed Main Menu (Single Player, Multi Player, Settings, 1987, Exit) with the preset line at its foot; the attract is paused behind the menu and resumes from the same frame when Esc returns; Exit closes the window, and so does the OS close button. Single Player and Multi Player open their framed sub-menus with the items that do not exist yet greyed and skipped. Cursor movement and confirm each play one of the ROM's verified FM effects. Every transition is a hard cut. Underneath, a headless game session takes per-frame logical inputs from power-on and yields the presented frame, the sound latch, the current screen and its transition; the binary is a window loop over it. The ROM set is looked up beside the executable and then in the app-data folder; a missing set prints a console line for now (the screen comes in FE11).

### Acceptance criteria

- [ ] A headless game session exists: per-frame inputs in, presented 384×224 frame, sound latch, screen name and transition out; the game binary contains no logic beyond the window loop, device setup and the screen stack.
- [ ] Compositor helpers draw text from the ROM's 8×8 font, a header in the COUNTRY SELECT style, and the gold double frame; `FREE PLAY` replaces the `INSERT COIN` pixels in overhaul play only, and the attract machine's work RAM and task positions equal the faithful power-on scenario across a full attract loop.
- [ ] Start opens the Main Menu during the warning screen, the title, the demo fight and the ranking; Esc returns to an attract that continues from the frame it was paused on; Exit and the OS close button close the window at once.
- [ ] Single Player lists Arcade Mode, Training, World Tour and Dramatic Battle; Multi Player lists Local, Online and Local co-op Dramatic; items that do not exist yet are drawn in the dim blue of the preset line and skipped by the cursor; the preset line names the active preset.
- [ ] Cursor movement and confirm play two of the ROM's verified FM effects through the existing mixer; nothing else sounds on menus; the attract's sound stops while a menu is up.
- [ ] Headless acceptance tests cover every bullet above by asserting pixels at known tile positions, screen names and transitions, and an empty unrecovered set.

### Blocked by

- FE1

---

## FE3: Boss ladder report

### Parent

Spec #150

### What to build

An ignored-by-default integration test that drives each of the ten bosses as the player through the whole one-player game from power-on under the Definitive Overhaul preset and writes a committed report for the SF1+ project: for each boss, the stage reached and the first untranslated branch (pre-fight, round 2, each bonus stage, the flights, continue, the ending), and for each of the twelve fighters whether a Local-style mirror match reaches a clean first round. It never fails the build; it is a map, not a gate.

### Acceptance criteria

- [ ] Running the ignored test writes the report under the reference docs with one section per boss, naming the stage reached and the first untranslated branch (address and frame) or "complete".
- [ ] The report records, for each of the twelve fighters, whether a mirror match in a two-player fight reaches a clean first round with an empty unrecovered set.
- [ ] The test is ignored by default, passes whenever the report is written, and the generated report is committed.

### Blocked by

- None (can start immediately)

---

## FE4: Local into a real fight

### Parent

Spec #150

### What to build

Multi Player, Local, the existing portrait select with names (P1 picks, then P2, any of the twelve under an overhaul preset), then the cabinet driver: it snapshots the attract machine, sets the active preset's configuration with the chosen fighters, full cast, four countries, ban list and the Gameplay difficulty and round time, feeds a coin, `start1` and the `start2` join behind a black frame, and stops at the real country-select screen, where P1 picks the flag. The fight runs on the real engine. At the match end the session stops before the original's winner-carries-on branch and shows `REMATCH`, `RESELECT`, `MENU`: Rematch replays the same fighters and flag hidden; Reselect and Menu restore the attract snapshot. An untranslated branch during the drive or the fight shows a one-line ROM-font notice, returns to the Main Menu and logs the frame and addresses.

### Acceptance criteria

- [ ] The cabinet driver carries a running faithful machine from any attract phase to the country-select task using coin and Start pulses only, inside a frame budget; a driver test proves it for the Local path and the one-player path from several attract frames.
- [ ] From the country screen, each of the twelve fighters on each side reaches the pre-fight with its own move table bound to the human side and plays a clean first round (driver test on the real power-on machine, no untranslated branch).
- [ ] The fight runs under the active preset's toggles with the configured difficulty and round time; the chosen fighters are stamped onto the configuration after any preset change and before every drive.
- [ ] At the match end the session stops before the carry-on and shows `REMATCH` / `RESELECT` / `MENU`; Rematch reaches the pre-fight again with the same fighters and flag; Menu and Reselect restore an attract state equal to the snapshot (snapshot equality test).
- [ ] An untranslated branch produces the notice, the return to the Main Menu, and a log line with the frame and addresses.
- [ ] Headless acceptance walk: power-on, Start, Multi Player, Local, Sagat against Gen, country, first round, match end, Rematch, match end, Menu, attract resumed.

### Blocked by

- FE2

---

## FE5: The portrait select finished

### Parent

Spec #150

### What to build

The select screen as chosen: the twelve VS-screen busts decoded from the ROM at startup (name shown for any that fails), two simultaneous cursors in Local (P1 yellow, P2 blue) that may share a tile, lock on confirm, un-confirm on back, and move on the moment both are locked, with the bust panel following the cursor that moved last; greyed tiles the cursors skip (Arcade's ten bosses with the caption `SF1+`, banned fighters under the Classic Competitive ban list, the ten bosses under Faithful 1987); the mirror rule as the ladder report found it; and Faithful 1987 reducing Local to Ryu against Ken with the two-country screen.

### Acceptance criteria

- [ ] All twelve busts decode from the ROM's VS-screen tiles at startup and render at the mockup's positions; a decode failure shows the fighter's name instead, proven with a deliberately damaged tile set.
- [ ] In Local both cursors move at once, may rest on one tile, lock on confirm, un-confirm on back, and the screen moves on when both are locked; in Arcade and Training only P1's cursor exists.
- [ ] Greyed tiles are drawn dimmed, captioned where the spec says, and skipped by both cursors under each of the three conditions.
- [ ] A mirror match is offered only when the ladder report records it as clean; otherwise P2's cursor skips P1's locked fighter.
- [ ] With Faithful 1987 active the select offers only Ryu and Ken, Local is Ryu against Ken, and the driver reaches the two-country screen.
- [ ] Pixel-asserting screen tests with no window cover every rule above.

### Blocked by

- FE3
- FE4

---

## FE6: Arcade as Ryu or Ken

### Parent

Spec #150

### What to build

Single Player, Arcade, the select with Ryu and Ken (the bosses greyed), the driver's one-player path (coin, `start1`) to the four-flag screen, and the 1987 ladder from the chosen country. Continuing is free: the continuation DIP is forced to allowed for overhaul Arcade and a coin pulse is injected when the continue screen opens, so any Start continues, with the ROM's own continue limit still applying. Letting the countdown expire plays the ROM's game over, ranking check, name entry and ranking screen as translated, and the front end shows the Main Menu where the ROM would have returned to the attract; the new ranking entry stays on the attract's ranking screen for the session.

### Acceptance criteria

- [ ] Arcade reaches the first fight from each of the four countries as Ryu and as Ken, under the active preset with the configured difficulty and round time.
- [ ] Losing a match shows the ROM's continue countdown; any Start continues with no coin key pressed; the ROM's continue limit still ends the run.
- [ ] Letting the countdown expire plays the ROM's game over, ranking check, name entry and ranking screen; the front end returns to the Main Menu at the frame the ROM would have returned to the attract.
- [ ] The new ranking entry appears on the attract's ranking screen afterwards in the same session.
- [ ] Headless acceptance walk covering a continue, an expiry, name entry and the return; the ladder beyond the first fight is covered by the existing full one-player scenario, which still passes.

### Blocked by

- FE4

---

## FE7: Pause box and Training

### Parent

Spec #150

### What to build

Space or a pad Start opens a small framed pause box over the dimmed fight with `RESUME`, `OVERLAYS` (off, hitboxes, frame meter, both), `RESTART ROUND` (the round-start snapshot) and `QUIT TO MENU`; either player in Local; no frames step while it is up. The Training screen asks `OPPONENT: CPU` or `DUMMY`, then the select for the player and, for a CPU, the opponent; Training with a CPU is the one-player game against that opponent, Training with a dummy is a Local-type fight whose P2 side never moves; both stop at the match end like Local. In Training only, the pause box carries a `PRESET` row that cycles the overhaul presets, never Faithful 1987, re-stamping the chosen fighters.

### Acceptance criteria

- [ ] Pause opens from the keyboard and from either player's Start, dims the fight, steps no frames, and `RESUME` continues with a state equal to never having paused (snapshot comparison).
- [ ] `OVERLAYS` cycles the diagnostic overlay modes; `RESTART ROUND` restores the round-start snapshot bit for bit; `QUIT TO MENU` returns at once and restores the attract.
- [ ] Training with a CPU runs the one-player game against the chosen opponent and stops at the match end with the after-match box; Training with a dummy runs a Local-type fight whose P2 side stays idle.
- [ ] The `PRESET` row appears only in Training, cycles the overhaul presets without Faithful 1987, keeps the chosen fighters, and the fight continues with no untranslated branch.
- [ ] Headless acceptance walks for both Training kinds and for pause in Local.

### Blocked by

- FE4

---

## FE8: Settings and persistence

### Parent

Spec #150

### What to build

The Settings screen with its tabs across the top: GAMEPLAY (the preset row, every core toggle by display name as `ON`/`OFF`, the three frame counts as `4F`, `28F`, `8F`, `DIFFICULTY` 1 to 4, `ROUND TIME` by the DIP's values), ROSTER (full cast, four countries, ban list live; what-if styles, bonus challengers, secret rivals greyed), VIDEO (scale 1× to 4×, borderless fullscreen), SOUND (master, FM, samples 0 to 10; jukebox greyed), CONTROLS (device rows showing the keyboard, scheme rows showing `CLASSIC` with the others greyed; pads and rebinding arrive in FE9) and a greyed NETPLAY tab. Rows read `NAME ........ VALUE`, left and right change values, every tab ends with `RESET TAB`, and the screen saves on leaving. Editing a toggle flips the preset to `CUSTOM`; changing the preset keeps the chosen fighters. Difficulty and round time reach every fight. Scale and fullscreen apply to the window; volumes apply in the host mixer. Gameplay and roster stay in the match-config file; video, sound and the ROM folder go to a new settings file; a file that fails to parse is logged and replaced by defaults.

### Acceptance criteria

- [ ] Every row in the spec's tab table exists with its wording and value set; greyed rows and the greyed tab are skipped; L1/R1 and Q/E switch tabs; `RESET TAB` restores that tab's defaults.
- [ ] Changing a Gameplay toggle flips the preset to `CUSTOM`; changing the preset keeps the fighters chosen on the select screen.
- [ ] Difficulty and round time set here are observed in the next Local, Training and Arcade fight through the DIP words on the machine.
- [ ] Scale and borderless fullscreen apply on leaving the screen; the three volumes scale the mixer output, proven by a gain test.
- [ ] Settings persist across a relaunch through the match-config file and the new settings file; a corrupted file is logged and replaced by defaults with no crash.
- [ ] Headless tests change and read back every row; the host smoke check covers the settings round trip in a scratch app-data folder.

### Blocked by

- FE4

---

## FE9: Pads and rebinding

### Parent

Spec #150

### What to build

A pad reader in the host crate through the pure-Rust pad crate with its default Windows Gaming Input backend, mapping a DualShock 4 as Cross to confirm, Circle to go back, Options as Start and L1/R1 for tabs; `P1 DEVICE` and `P2 DEVICE` rows (keyboard, pad 1, pad 2; default by connection order); the `REBIND P1`, `REBIND P2` and `REBIND MENU` walks, where a key or pad press binds, Esc keeps the old one, and a duplicate within the same player is refused with a one-line notice; the bindings file extended with pad bindings and menu inputs. A pad disconnect during an Arcade, Local or Training fight opens the pause box with `P1 PAD DISCONNECTED` or `P2 PAD DISCONNECTED`, any device resumes, and that side idles until a pad is seen again; the keyboard never takes over on its own. The development launcher reads pads through the same host code.

### Acceptance criteria

- [ ] A DualShock 4 over USB enumerates with nothing else installed and drives menus and fights with the spec's mapping; the host smoke check lists enumerated pads.
- [ ] The rebind walk binds keys and pad buttons in order, Esc keeps the old binding, a duplicate within the same player is refused with a notice, menu inputs rebind the same way, and bindings persist across relaunch.
- [ ] Pulling a pad during an Arcade, Local or Training fight opens the pause box with the notice; any device resumes; that side stays idle until a pad returns; the keyboard does not take over.
- [ ] The development launcher reads pads through the shared host crate with no change to its subcommands.
- [ ] Headless tests drive menus and a fight with synthetic pad events, including a disconnect and a reconnect.

### Blocked by

- FE7
- FE8

---

## FE10: The faithful cabinet

### Parent

Spec #150

### What to build

The 1987 item opens `1987 CABINET`: a framed screen listing coinage A, coinage B, continuation, buy-in, difficulty, round time, flip screen, demo sounds, attract music, country count and test mode with the DIP's own options at factory defaults, `HOLD ESC ONE SECOND TO RETURN` where the preset line sits, and `POWER ON`. Power on boots a fresh faithful machine from power-on with those DIP words, the coin, start and service credit keys live, and nothing from the overhaul settings except video, sound and bindings; test mode on boots straight into the service screen. Holding Esc for one second, shown as a small hold-bar overlay that leaves the machine untouched, returns to the overhaul attract and is the only way out. The DIP words persist in the settings file.

### Acceptance criteria

- [ ] The DIP screen lists exactly the verified settings with the DIP's own options and factory defaults; `POWER ON` boots a faithful machine whose decoded DIP words match the rows, checked against the existing DIP decode tests.
- [ ] Coins, starts and the service credit key work in the cabinet; with test mode on it boots into the service screen; no overhaul toggle or roster setting reaches it.
- [ ] A full attract loop inside the cabinet equals the power-on scenario frame for frame, and the hold bar is a presentation overlay that leaves work RAM and the hardware frame untouched.
- [ ] Holding Esc for one second returns to the overhaul attract; a shorter press does nothing; the DIP words persist across relaunch.
- [ ] Headless walk: Main Menu, 1987, DIP screen, `POWER ON`, coin, start, the first fight, Esc hold, overhaul attract.

### Blocked by

- FE8

---

## FE11: Shipping shape

### Parent

Spec #150

### What to build

One executable with the ROM manifest embedded. The ROM set is looked up beside the executable, then in the app-data folder, then in the folder named in the settings file. With no set, a framed screen in the compositor's ROM-independent fallback font names the set, lists the three folders, and offers `RETRY`, which re-scans and continues into the attract, and `EXIT`. One log file in the app-data folder, overwritten at every launch, records the settings in force, the ROM folder and manifest result, pads seen or lost, settings files that failed to parse, and untranslated-branch notices with frame and addresses. First launch on a clean app-data folder yields scale 3×, windowed, volumes 10, Definitive Overhaul and keyboard for both players, and writes the three files.

### Acceptance criteria

- [ ] The release binary runs from an empty folder with only the ROM folder beside it; nothing under the reference folders is read, and the manifest is embedded.
- [ ] With no ROM set the ROM-missing screen names the set and the three folders; adding the files and choosing `RETRY` continues into the attract without a relaunch; `EXIT` closes.
- [ ] The log holds the five kinds of line and is overwritten each launch.
- [ ] First launch on a clean app-data folder yields the spec's defaults and writes the match-config, bindings and settings files.
- [ ] The host smoke check covers ROM lookup in each of the three folders and the clean-folder first launch.

### Blocked by

- FE8

---

## FE12: Completeness audit and README

### Parent

Spec #150

### What to build

The closing audit of the front end: one headless acceptance walk across every screen and play mode, the whole workspace test suite including every comparison fixture, every greyed item verified greyed and skipped, the development launcher's subcommands verified unchanged, and the README's "Playing the game" and "Controls" sections rewritten for the game binary, with the session handoff updated.

### Acceptance criteria

- [ ] One headless acceptance test walks power-on, attract, menu, Arcade (a continue and an expiry), Local (a rematch), Training (both kinds, every pause row, the live preset), Settings (every tab), the 1987 cabinet and back, and Exit, with no untranslated branch.
- [ ] The whole workspace test suite passes, including every comparison fixture with the toggles off and the development launcher's smoke check.
- [ ] An audit note under the planning folder lists each of the spec's user stories with the test that covers it or the reason it is deferred.
- [ ] README's "Playing the game" and "Controls" describe the game binary, the ROM folder, the pads and the settings; the development launcher's section stays as it is.

### Blocked by

- FE5
- FE6
- FE7
- FE9
- FE10
- FE11

---

## FE13: A person's session with two DualShock 4 pads (person)

### Parent

Spec #150

### What to build

A person plugs two DualShock 4 pads into the Windows machine over USB with nothing else installed, plays a Local match on them, swaps P1 and P2 on the Controls tab, rebinds a button and relaunches, pulls one cable mid-fight and plugs it back, and plays the faithful cabinet with the pads. They record what worked and what did not in a comment on this ticket, with the pad model, cable and Windows version.

### Acceptance criteria

- [ ] Both pads enumerate without DS4Windows or Steam Input; P1 and P2 follow connection order and can be swapped on the Controls tab.
- [ ] Cross, Circle, Options, L1 and R1 behave as specified in menus and settings; a rebind survives a relaunch.
- [ ] A disconnect pauses with the notice and the side resumes when the pad returns.
- [ ] Any failure is recorded with the pad, cable and Windows version; a failure to enumerate triggers the SDL2 fallback decision recorded in the spec.

### Blocked by

- FE9
