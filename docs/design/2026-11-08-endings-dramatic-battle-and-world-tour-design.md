# Character Endings, 2v1 Dramatic Battle, and 1987 World Tour on the Real Engine

- **Date:** 2026-11-08
- **Status:** Approved
- **Roadmap slot:** Project 5 (follows Project 3 `SF1+ on the real engine` `Spec #246` and Project 4 `Era styles and gauges on the real fighters` [`Spec #5`](https://github.com/johnstreetfighter/sf1-port/issues/5))
- **Related ADRs & references:**
  - [`ADR-0004`](../adr/0004-reach-fights-by-driving-the-faithful-flow.md) — Menu-driven fights run on the faithful machine
  - [`ADR-0014`](../adr/0014-bosses-become-1987-player-fighters.md) — Extended data bank (`0x100000`)
  - [`ADR-0009`](../adr/0009-stages-and-themes-in-extended-banks.md) — Extended background bank (`BackgroundBank` pages, palettes, and `LayerView` gates)
  - [`ADR-0014`](../adr/0014-bosses-become-1987-player-fighters.md) — `SF1+` boss fighters and rules run on the 1987 engine
  - [`ADR-0015`](../adr/0015-era-styles-and-sub-hp-on-the-1987-engine.md) — Era styles and `960` Sub-HP on the 1987 engine
  - [`ADR-0016`](../adr/0016-dramatic-battle-and-composite-endings-on-the-1987-engine.md) — Three-fighter Dramatic Battle, ROM-composite endings, and World Tour on the 1987 engine
  - [`docs/reference/ending.md`](../reference/ending.md) — The 1987 ending sequence (`0x44a40..0x44c1c`)
  - [`docs/reference/match-win.md`](../reference/match-win.md) — Event `0xb` (`0x4001c`) match-won cards and Event `2` (`0x3f812`) world travel map

---

## 1. Intent and the Why

Project 3 (`Spec #246`, specifically `SP14` `#266`) opens the 1987 Arcade ladder to all twelve tournament fighters with Ryu standing in for a boss player's own ladder slot, and sends a boss player's Thailand clear straight to the 1987 staff roll (`0x44b88`) and name entry (`0x42486`) without an ending screen. Meanwhile, three major single-player and co-op pillars exist in [`crates/sf-engine/src/`](../../crates/sf-engine/src/) only as standalone 2D prototype structs from the pre-`ADR-0004` milestone (`arcade.rs`, `dramatic.rs`, and `world_tour.rs`), while their menu items in [`crates/sf-engine/src/front/menu.rs`](../../crates/sf-engine/src/front/menu.rs) (`MenuItem::WorldTour`, `MenuItem::DramaticBattle`, and `MenuItem::LocalCoopDramatic`) sit greyed out and unselectable.

Project 5 brings all three pillars onto the real 1987 engine (`FightMachine`, `Fight`, `GameSession`, `BackgroundBank`, and `OverhaulScene`) with zero new PixelLab sprite dependencies:

1. **12-Fighter Arcade Endings & Post-Match Win Quotes:** Give all ten bosses their own ROM-composite congratulatory ending screens (`BackgroundBank` left-bust + home-stage backdrop + bespoke 1987-style character epilogue typed by the real `TextWriter` at `0x3bb98`), swap the player's own boss slot in the 10-opponent defeated montage (`0x44a60..0x44aff`) and post-match win card (`0x4014c`) to show defeated stand-in Ryu (`0x0900`), and give all twelve fighters character-specific post-match win quotes on event `0xb` in overhaul Arcade runs.
2. **2v1 Dramatic Battle (`Single Player -> Dramatic Battle` & `Multi Player -> Local Co-Op Dramatic`):** Enable real 3-fighter `2v1` matches on the 1987 engine (`Ally 1` on `P1_STRUCT`, `Boss` on `CPU_STRUCT`, and `Ally 2` stepped via a 3rd fighter slot in `CoreToggleState` through the same `fighter.rs` move/command/collision routines and rendered in `OverhaulScene`), complete with 3-fighter camera clamping, 6 signature rivalry presets + `CUSTOM 2V1`, `Shared` vs `Independent` lifebars, a shared Team Era Gauge in `Shared` mode, and a `Friendly Fire` toggle.
3. **1987 World Tour (`Single Player -> World Tour`):** Enable an 18-node branching campaign across the five 1987 countries (`Japan`, `USA`, `China`, `England`, `Thailand`) rendered over the ROM's `0x3f812` world travel map background, featuring six real-fight mission types (including Survival Gauntlets, Rule-Modifier Trials, Era Style Mastery nodes, and 2v1 Dramatic Boss finales) and a single shared profile with toggleable RPG progression (`XP`, `ATTACK`/`DEFENSE`/`SPEED` stats, and 5 unlockable Perks) persisted to `%APPDATA%\sf1-port\world_tour.toml`.

---

## 2. Goals, Non-Goals, and Anti-Goals

### Goals
- **Complete 12-fighter Arcade endings without PixelLab bottlenecks:** Build the 10 bosses' congratulatory ending views (`ending:{Boss}`) in `BackgroundBank` at load time from existing ROM tiles and palettes (using the same zero-new-tile composite technique as `#215` in [`crates/sf-video/src/vs_screens.rs`](../../crates/sf-video/src/vs_screens.rs)), and type each boss's bespoke epilogue through the real 1987 `TextWriter` (`0x3bb98`, `D4 = 5`, `TEXT_SOUND = 0x000d`) before entering the 1987 staff roll (`0x44b88`) and name entry (`0x42486`).
- **Keep Ryu & Ken's classic 1987 endings intact:** When the player clears Arcade as Ryu or Ken (in either Faithful 1987 or an overhaul preset), `0x44b20..0x44b86` continues to display their original ROM victory scenes (`0x7700` for Ryu, `0x7880` for Ken) and classic `0x44d60` congratulatory text.
- **Stand-in Ryu consistency across win cards and the ending montage:** Whenever a boss player fights and defeats stand-in Ryu on the Arcade ladder (or views the 10-opponent defeated montage at `0x44a60..0x44aff`), that boss's own slot displays the ROM's defeated Ryu card (`bg_scroll = 0x0900`, palette `0x1ae68`) and a 2×2 `RYU` card instead of showing the player's own boss defeated.
- **Character-specific post-match win quotes (`event 0xb`, `0x4001c`):** In overhaul Arcade runs, after winning each country match, event `0xb` displays a character-specific win quote block for the winning fighter (all 12 fighters).
- **Light up `DramaticBattle` and `LocalCoopDramatic` in `menu.rs`:** Run 3-fighter `2v1` fights on the real engine with both human+CPU (`Single Player -> Dramatic Battle`) and two-human (`Multi Player -> Local Co-Op Dramatic`) control, 6 curated rivalry presets + `CUSTOM 2V1`, `Shared` (default) and `Independent` lifebar modes, a `Friendly Fire` (`OFF` default / `ON`) toggle, and full `Era Style` support.
- **Light up `WorldTour` in `menu.rs`:** Present an 18-node branching map over the 1987 world travel map backdrop with 6 real-fight mission types, free Fighter + Era Style switching between nodes, and toggleable RPG stats/perks saved to `%APPDATA%\sf1-port\world_tour.toml`.
- **Retire standalone prototype sessions (`arcade.rs`, `dramatic.rs`, `world_tour.rs`):** Once real-engine `GameSession` / `Fight` tests cover all three pillars, delete the standalone 2D prototype stepping code while keeping pure data tables (win quotes, epilogues, rivalry presets, World Tour node graph, and TOML persistence).

### Non-Goals
- **Bespoke PixelLab-illustrated ending panels:** Generating multi-panel pixel-art illustrations for the 10 bosses' endings is deferred to the lowest-priority post-game cosmetic art project alongside Era Costumes.
- **Bonus Challenger (`Wave 1–3`) endings and World Tour nodes:** Secret encounters and Arcade endings for the 14 Bonus Challengers belong to Project 6 (`Bonus Challengers`). World Tour's node structure and profile schema leave a clean hook (`unlocked_challengers`) for Project 6 without implementing Bonus Challengers here.
- **Online Co-Op Dramatic Battle:** `Multi Player -> Local Co-Op Dramatic` is local 2-player (`P1` + `P2` pads against the CPU Boss). Online rollback netplay (`MenuItem::Online`) is Project 7.

### Anti-Goals
- **Never alter `Faithful 1987` (`RulesetPreset::Faithful1987` or `1987 Cabinet`):** Every ending gate, win-quote gate, 3rd-fighter slot, and World Tour modifier is strictly gated behind `!config.is_faithful_1987()`. All 137 MAME reference scenarios (`sf-win-10`, `sf-full-1p-01`, `travel_map.rs`, `ending.rs`) must remain bit-for-bit identical.
- **Never leak World Tour RPG stats or perks outside World Tour:** `TourRules` (`attack`, `defense`, `speed`, `perk`) apply only inside a `FightKind::WorldTour` match when the player's `RPG: ON` toggle is active. Arcade, Local Versus, Training, Dramatic Battle, and Online never read `world_tour.toml`.
- **Never keep two parallel implementations of Dramatic Battle or World Tour:** Do not wire the front-end menu items to the old 1D/2D `DramaticBattle` or `WorldTour` prototype structs; every match runs through `Fight` on the real 1987 engine.

---

## 3. What Exists Today & Constraints

### 3.1 The 1987 Ending (`0x44a40..0x44c1c`) and Event `0xb` (`0x4001c`)
In [`crates/sf-engine/src/tasks.rs`](../../crates/sf-engine/src/tasks.rs) (documented in [`docs/reference/ending.md`](../reference/ending.md) and [`docs/reference/match-win.md`](../reference/match-win.md)):
1. **Post-match win screen (`event 0xb`, `0x4001c` / `Screen::WinText` at `0x4014c`):**
   - After a human match win in 1P mode (`GAME_MODE == 2` or `4`, `0xffc009 & 3 != 0`), `0x4014c` clears the objects and text layer, queues sound `0x9e` (`0xba` on opponent `9`, Sagat), draws the generic ROM win-quote text block `0x158e4` (`0x159b2` on opponent `9`), and scrolls `bg_scroll` to `*(0x4018a + 4 * country) + 2 * opponent` (showing the defeated opponent's background card at `0x1500 + 0x980 * opponent + 0x680` in that stage's palette) for `180 + 30` frames.
   - In Versus (`0x4006c`), `0x400d8` holds two defeated-player `bg_scroll` words with palette `0x1ae68`: `0x0a80` (P1 wins: defeated Ken) and **`0x0900` (P2 wins: defeated Ryu)**.
2. **Stage-4 Thailand ending dispatch (`0x3ef96 -> 0x44a40`):**
   - **Part 1 — Five country-ordered defeated scenes (`0x44a60..0x44aff`, `Flow::EndingStart` / `Flow::EndingSceneHold`):** Steps `d1 = 0..=4` and `d4 = 1..=0` (10 sub-steps of 120 frames each). For each sub-step, it loads `byte` from `*(0x44c1c + 4 * country) + 4 * d1 + (1 or 2)`: requests the country's stage palette (`0x44c7c`), scrolls `bg_scroll` to `*(0x44c90 + 2 * (byte >> 4))` (the same defeated-opponent background card used by `0x4018a`), and draws the opponent's 2×2-tile name card `*(0x44ca4 + 4 * (byte & 0x0f))` via `text_block_2x2` (`0x3bc3e`).
   - **Part 2 — Congratulatory portrait & typewriter message (`0x44b00..0x44b86`, `Flow::EndingMessageSetup` / `Flow::EndingMessageTyping` / `Flow::EndingMessageHold`):** Requests palette `0x1b868` at offset `0`, scrolls `bg_scroll` to `0x7700` (`0x44d5c`, Ryu's left-side victory portrait) or `0x7880` (`0x44d5e`, Ken's left-side victory portrait), sets `TEXT_SOUND = 0x000d`, and runs `TextWriter::new(bus, 0x44d60, 5)` (`0x3bb98`) to type the `13 × 33` congratulatory block (169 non-fill characters, 5 frames per character), holds 120 frames, and erases the block (`0x3bd00`).
   - **Part 3 — Staff roll & name entry (`0x44b88..0x44c1b`, `Flow::EndingStaffSetup`..):** Plays tune `0x36`, types the 8 staff blocks (`0x44f5c..0x44f7b`) under `STREET FIGHTER STAFF` (`0x44f80`), holds `THE END` (`0x452cc`), and jumps to `0x3efc4 -> 0x3f2d0 -> 0x42422 -> 0x42486` (ranking check & 3-initial name entry).
3. **`SP14` (`#266`) Handoff Boundary:**
   - In `SP14`, when the player is one of the 10 bosses in overhaul Arcade mode, Ryu stands in for the player's own boss slot on the ladder, and clearing Thailand skips Part 1/Part 2 or goes straight to the staff roll (`0x44b88`) and name entry (`0x42486`). Project 5 replaces that temporary bypass with full 12-fighter ending scenes.

### 3.2 `BackgroundBank` Composite Views (`ADR-0009` & `#215`)
- [`crates/sf-video/src/background_bank.rs`](../../crates/sf-video/src/background_bank.rs) and [`crates/sf-video/src/vs_screens.rs`](../../crates/sf-video/src/vs_screens.rs) already build composite 24-column background screens at load time from `rom: &Assets` (copying 16×16 ROM tiles and merging 16-color palette rows into a `0x200`-byte layer palette) and register each screen under a string key via `bank.set_view(key, LayerView { page, scroll, palette })`.
- Every boss `i` (`2..=11` in `FIGHTERS`) already has a 6×6-tile left-facing bust block in the ROM (`MODE_4_SCROLLS` at `0x3fad6`, `BLOCK_ROW = 3..=8`, `BLOCK_COL[LEFT] = 4..=9`, with nameplate at `NAME_ROWS = [10, 11]`, `NAME_COL[LEFT] = 6..=9`) and a 56-column home-stage background (`0x437fe` per-country pointer table) in their country stage palette (`0x43778`).

### 3.3 Two-Slot Fighter Execution vs 3-Fighter Dramatic Battle
- In [`crates/sf-engine/src/fighter.rs`](../../crates/sf-engine/src/fighter.rs) and [`crates/sf-engine/src/tasks.rs`](../../crates/sf-engine/src/tasks.rs), the 1987 fight loop steps two work-RAM player structs and two 24-entry (`0x600`-byte) object-RAM (`OBJ_BASE = 0xffe000`) sprite grid regions:
  - Slot 0: `P1_STRUCT` (`0xff86b0`, object-RAM region `0x400..0x9ff` = `0xffe400..0xffe9ff`)
  - Slot 1: `P2_STRUCT` (`0xff86da`) in Versus, or `CPU_STRUCT` (`0xffc664`) in 1-player mode (object-RAM region `0xa00..0xfff` = `0xffea00..0xffefff`).
  - Effect objects occupy `0x100..0x3ff` (`0xffe100..0xffe3ff`, 12 slots of `0x40`).
- **Planned prerequisite after Project 4:** Once `ADR-0015` (`Project 4`) is implemented on the existing `ADR-0014` (`Project 3`) engine, player-controlled and overhaul-CPU fighters will run through parameterized move/command/physics/collision routines in `fighter.rs`, reading move tables from the extended data bank (`0x100000`) and storing per-fighter overhaul state (`sub_hp: u16` on the `960` scale, `EraFighterState`, active overhaul projectile, and sprite override) in `CoreToggleState`. The current SF1+ `CoreToggleState` does not yet contain `sub_hp` or `EraFighterState`; these are planned Project 4 additions.
- The renderer (`OverhaulScene` in [`crates/sf-video/src/compositor.rs`](../../crates/sf-video/src/compositor.rs)) already composites arbitrary `IndexedSprite` entries into the sprite plane before CRT presentation.

### 3.4 Prototype Files & Greyed Menu Items
- [`crates/sf-engine/src/front/menu.rs`](../../crates/sf-engine/src/front/menu.rs) already places `MenuItem::WorldTour` and `MenuItem::DramaticBattle` in `SINGLE` (`Single Player`) and `MenuItem::LocalCoopDramatic` in `MULTI` (`Multi Player`), currently returning `false` in `MenuItem::enabled(self)`.
- [`crates/sf-engine/src/arcade.rs`](../../crates/sf-engine/src/arcade.rs), [`crates/sf-engine/src/dramatic.rs`](../../crates/sf-engine/src/dramatic.rs), and [`crates/sf-engine/src/world_tour.rs`](../../crates/sf-engine/src/world_tour.rs) hold standalone 1D/2D prototype sessions from the pre-`ADR-0004` milestone that are not connected to `GameSession` or `Fight`.

---

## 4. Approaches Considered

### 4.1 10-Boss Arcade Ending Art
- **Option A (Chosen): Zero-PixelLab ROM-Composite Endings in `BackgroundBank` + 1987 `TextWriter` epilogues.**
  - Build 10 composite ending screens (`ending:{Boss}`) in `BackgroundBank` at startup by combining each boss's 6×6 left bust (`0x3fad6`) and nameplate over a darkened/framed slice of their home stage backdrop in their stage palette, leaving columns `11..=22` clear for the text layer. Store 10 character-specific `13 × 33` text blocks in the extended data bank (`0x100000`) and type them through the real 1987 `TextWriter` (`0x3bb98`, `D4 = 5`, `TEXT_SOUND = 0x000d`). Swap the player's own boss slot in both `0x4014c` (event `0xb`) and `0x44a60..0x44aff` (the 10-opponent defeated montage) to show the ROM's defeated Ryu scene (`bg_scroll = 0x0900`, palette `0x1ae68`, 2×2 card `RYU`).
  - *Pros:* Ships immediately after Project 3 (`SP14`) with zero PixelLab credit dependency; 100% authentic 1987 visual and audio presentation; reuses `#215`'s battle-tested `BackgroundBank` composition pattern.
- **Option B: Wait for PixelLab credit renewal to generate 10 multi-panel ending illustrations.**
  - *Cons:* Blocks Project 5 on external PixelLab credit renewals after November 27; deferred to the post-game cosmetic art project.

### 4.2 3-Fighter Execution in 2v1 Dramatic Battle
- **Option A (Chosen): Keep Ally 1 on `P1_STRUCT` (`0x400..0x9ff`) and Boss on `CPU_STRUCT` (`0xa00..0xfff`), and step Ally 2 on `P2_STRUCT` with a 3rd object-RAM region (`0x1000..0x15ff`).**
  - Ally 1 (`P1`) occupies `P1_STRUCT` (`0xff86b0`, object-RAM region `0x400..0x9ff`) and the Boss occupies `CPU_STRUCT` (`0xffc664`, object-RAM region `0xa00..0xfff`), so all 1987 stage setup, HUD rendering, round timers, and KO/round-transition tasks (`tasks.rs`) work natively without rewriting the 1987 task scheduler.
  - Ally 2 (`P2` human pad in `LocalCoopDramatic`, or overhaul CPU pad-feeder in `DramaticBattle`) runs on `P2_STRUCT` (`0xff86da`) paired with a 3rd 24-entry object-RAM region (`0x1000..0x15ff` = `0xfff000..0xfff5ff`) and a 3rd slot in `CoreToggleState` (`EraFighterState` + projectile slot), stepped each fight frame (`0x3eafa`) through the exact same `fighter.rs` move/command/physics/collision routines against `CPU_STRUCT` (and against `P1_STRUCT` when `Friendly Fire` is `ON`), with Ally 2's body and projectile sprites injected into `OverhaulScene`.
- **Option B: Dynamically swap `P1_STRUCT` and `P2_STRUCT` depending on which ally is closer to the Boss.**
  - *Cons:* Breaks continuous charge/motion input buffers (`Down`/`Back` charge timers, `QCF`/`DP` history) and animation state whenever the two allies cross each other's X distance to the Boss.

### 4.3 World Tour Map Presentation & Profile Model
- **Option A (Chosen): 18-node 5-country branching map rendered over the 1987 `0x3f812` world travel map background, backed by one shared profile (`world_tour.toml`) with free Fighter/Era switching.**
  - Render `WorldTourScreen` using the ROM's world travel map foreground layer and palette (`fg_scroll = 0x1980`, palette `0x1a268` at offset `0x200` from `travel_map_start` at `0x3f812` in [`crates/sf-engine/src/tasks.rs`](../../crates/sf-engine/src/tasks.rs)) with connected node markers across `Japan`, `USA`, `China`, `England`, and `Thailand` and a framed bottom HUD bar (`Confirm`: Start Fight, `Tab Left`: Change Fighter & Era Style via `SelectScreen`, `Tab Right`: RPG Stats/Perks & `RPG: ON/OFF` toggle).
- **Option B: Plain framed text list of missions locked to a single fighter per save file.**
  - *Cons:* Loses the 1987 world-map identity and forces repetitive restarts to try different fighters or Era Styles on later nodes.

---

## 5. Decided Shape of the Work

### 5.1 Pillar 1 — 12-Fighter Arcade Endings, Stand-in Ryu Montage Swap, & Character Win Quotes

#### A. Stand-in Ryu Swap in Event `0xb` (`0x4014c`) and the 10-Opponent Montage (`0x44a60..0x44aff`)
In overhaul Arcade runs (`!config.is_faithful_1987()`), when the human player's chosen fighter is one of the 10 bosses (`player_fighter >= TournamentFighter::Retsu`, opponent index `player_opp_idx = (player_fighter as u8) - 2` in `0..=9`):
1. **Post-match win card (`Screen::WinText` at `0x4014c` in [`crates/sf-engine/src/tasks.rs`](../../crates/sf-engine/src/tasks.rs)):**
   - When `opponent_index(bus) as u8 == player_opp_idx` (the fight where `SP14` placed stand-in Ryu in the player's own boss slot), instead of scrolling `bg_scroll` to the player's own defeated boss card (`*(0x4018a + country) + 2 * opponent`), the overhaul gate requests palette `0x1ae68` at offset `0` and scrolls `bg_scroll` to **`0x0900`** (`0x400d8 + 2`, the ROM's defeated Ryu card).
2. **Five-country defeated montage (`Flow::EndingStart` / `ending_scene_outer` / `ending_scene_step` / `Flow::EndingSceneHold` at `0x44a60..0x44aff`):**
   - Each of the 10 sub-steps (`d1 = 0..=4`, `d4 = 1..=0`) encodes `slot = (byte & 0x0f)` (`0..=9`, matching `Retsu..=Sagat` in `FIGHTERS[2..=11]`).
   - When `slot == player_opp_idx`, `ending_scene_step` overrides the scene for that 120-frame sub-step:
     - Requests palette `0x1ae68` at offset `0`;
     - Scrolls `bg_scroll` to **`0x0900`** (defeated Ryu);
     - Draws the 2×2-tile name block **`RYU`** (stored in the extended data bank at `0x100000`) instead of `*(0x44ca4 + 4 * slot)`.
   - On waking in `Flow::EndingSceneHold`:
     - When the completed sub-step was `slot == player_opp_idx`, `erase_block_2x2` erases the `RYU` 2×2 block in `DataBank` (rather than `*(0x44ca4 + d5)`).
     - When transitioning from `d4 = 1` (first boss of country `d1`) to `d4 = 0` (second boss of country `d1`), if `d4 == 1` overrode palette offset `0` with `0x1ae68` for stand-in Ryu, `ending_scene_step` on `d4 == 0` explicitly re-requests the country's stage palette from `0x44c7c` (`*(0x44c7c + 4 * (entry[0] & 0x0f))`).

#### B. Zero-PixelLab ROM-Composite Ending Views (`ending:{Boss}`) & Epilogues (`0x44b00..0x44b86`)
1. **Composite Ending Views in `BackgroundBank` (`crates/sf-video/src/ending_screens.rs`):**
   - At startup (alongside `BackgroundBank::add_vs_screens`), `BackgroundBank::add_ending_screens(&mut self, rom: &Assets)` builds 10 views keyed by `ending_key(fighter.name())` (`"ending:Retsu"`, `"ending:Geki"`, …, `"ending:Sagat"`):
     - Each screen is 24 columns (`SCREEN_COLUMNS = 24`) placed on a `BackgroundBank` page, starting from the blank cell of that boss's ROM VS screen (`rom_vs.cell(0, 0)`, as in [`crates/sf-video/src/vs_screens.rs`](../../crates/sf-video/src/vs_screens.rs)).
     - **Left portrait & nameplate:** Places the boss's 6×6-tile left-facing bust from `MODE_4_SCROLLS` (`0x3fad6`, `BLOCK_ROW = 3..=8`, `BLOCK_COL[LEFT] = 4..=9`) and their 4×2 nameplate (`NAME_ROWS = [10, 11]`, `NAME_COL[LEFT] = 6..=9`, bracket at col `5`), with top/bottom stage border accents from their home stage's background (`0x437fe` country table), while keeping the right-hand text region (`cols 10..=22`) filled with the ROM VS screen's blank backdrop cell so the typewriter text is crisp and legible.
     - **Palette:** Uses the boss's home country stage `bg` palette (`0x43778`, `0x200` bytes).
2. **10 Boss Epilogue Text Blocks in the Extended Data Bank (`0x100000`):**
   - Store 10 `TextWriter`-formatted blocks in `DataBank` (`0x100000`) matching the header and coordinate layout of `0x44d60` (`D4 = 5`, `TEXT_SOUND = 0x000d`):
     - **Retsu:** *"HAVING TESTED EVERY MASTER OF THE WORLD TOURNAMENT, RETSU RETURNS TO THE SHORINJI TEMPLE. HIS KIAI STRIKES ECHO THROUGH THE COURTYARD AS NEW DISCIPLES GATHER AT THE GATE."*
     - **Geki:** *"THE TOURNAMENT ENDS IN SILENCE. BEFORE THE CROWD CAN SPEAK HIS NAME, GEKI VANISHES INTO THE NIGHT SHADOWS—HIS CONTRACT FULFILLED, HIS CLAW UNBLEMISHED."*
     - **Joe:** *"BELT SLUNG OVER HIS SHOULDER, JOE WALKS OUT OF THE THAILAND ARENA UNDEFEATED. FROM THE BACK ALLEYS TO THE WORLD STAGE, THE AMERICAN CHAMP STANDS ALONE."*
     - **Mike:** *"NONE COULD WITHSTAND THE IRON FISTS OF MIKE. HAVING CRUSHED EVERY CHALLENGER ON EARTH, THE HEAVYWEIGHT POWERHOUSE SETS HIS SIGHTS ON A NEW RING."*
     - **Lee:** *"WITH BAJIQUAN PERFECTED AGAINST THE WORLD'S GREATEST FIGHTERS, LEE BOWS TO THE FOUR WINDS AND RETURNS TO THE GREAT WALL TO GUARD CHINA'S MARTIAL LEGACY."*
     - **Gen:** *"NO ASSASSIN NOR EMPEROR COULD FELL THE MASTER OF MANTIS AND CRANE. GEN COUGHS SOFTLY AND WALKS AWAY INTO THE MIST, STILL SEEKING ONE FINAL WORTHY DEATH-MATCH."*
     - **Birdie:** *"THE CROWD ROARS AS BIRDIE RAISES A MASSIVE FIST IN TRIUMPH. ENGLAND'S BRAWLER OWNS THE STREETS AND THE WORLD TITLE—AND RECKONS A PROPER FEAST IS NEXT."*
     - **Eagle:** *"TWO KALI STICKS RETURN CLEANLY TO THEIR CASE. HAVING PROVED THE ART OF GENTLEMAN'S COMBAT AGAINST EVERY STYLE ON EARTH, EAGLE TIPS HIS HAT AND DEPARTS LONDON."*
     - **Adon:** *"HAVING SURPASSED EVEN HIS MASTER'S LEGEND IN THE TEMPLE COURTYARD, ADON STANDS ATOP MUAY THAI AS ITS NEW EMPEROR. THE JAGUAR REIGNS SUPREME."*
     - **Sagat:** *"THE YOUNG CHALLENGERS FELL ONE BY ONE BEFORE THE TIGER'S WRATH. UNSCARRED AND UNBROKEN, SAGAT STANDS ATOP THE WORLD AS THE UNDISPUTED EMPEROR OF MUAY THAI."*
3. **Gate in `Flow::EndingMessageSetup` & `Flow::EndingMessageHold` (`0x44b20..0x44b86`):**
   - When `!config.is_faithful_1987()` and the active human fighter is one of the 10 bosses:
     - Select the `BackgroundBank` view `ending:{Boss}` (`LayerView { page, scroll, palette }`), request its `palette` at offset `0`, set `bg` page and `scroll`, and launch `TextWriter::new(bus, boss_epilogue_addr, 5)` with `TEXT_SOUND = 0x000d`.
     - On completion in `Flow::EndingMessageHold`, erase `boss_epilogue_addr` (`erase_block(bus, boss_epilogue_addr)`) and transition straight into `Flow::EndingStaffSetup` (`0x44b88`, staff roll -> `THE END` -> ranking check & name entry `0x42486`).
   - When the human fighter is **Ryu** or **Ken**:
     - `Flow::EndingMessageSetup` uses `0x44d5c` (`0x7700`) for Ryu and `0x44d5e` (`0x7880`) for Ken (including when Ken is played in 1P `GAME_MODE == 2` under an overhaul preset) and types the classic `0x44d60` congratulatory text block.

#### C. Character-Specific Post-Match Win Quotes (`event 0xb`, `0x4014c`)
- In overhaul Arcade runs (`!config.is_faithful_1987()`), when a human winner enters `Screen::WinText` (`0x4014c` in [`crates/sf-engine/src/tasks.rs`](../../crates/sf-engine/src/tasks.rs)):
  - Instead of drawing the generic ROM text block `0x158e4` (`YOU WIN THIS GAME...`), the overhaul gate draws the active fighter's character-specific post-match win-quote block from the extended data bank (`0x100000`) at the same text-layer coordinates beneath the defeated opponent card, using the 12 fighter win quotes established in `arcade::win_quote(fighter)` (expanded to two crisp arcade lines per fighter, with a special rivalry line when defeating final opponent `#9`).

---

### 5.2 Pillar 2 — 2v1 Dramatic Battle on the Real Engine

#### A. Menu Entry Points & Matchup Setup Flow
1. **Menu Items Enabled in [`crates/sf-engine/src/front/menu.rs`](../../crates/sf-engine/src/front/menu.rs):**
   - `MenuItem::DramaticBattle` under `SINGLE` (`Single Player -> Dramatic Battle`): **1P Human (`Ally 1`) + CPU Teammate (`Ally 2`) vs CPU Boss**.
   - `MenuItem::LocalCoopDramatic` under `MULTI` (`Multi Player -> Local Co-Op Dramatic`): **1P Human (`Ally 1`) + 2P Human (`Ally 2`) vs CPU Boss**.
2. **`DramaticSetupScreen` (`crates/sf-engine/src/front/dramatic_select.rs`):**
   - Selecting `DramaticBattle` or `LocalCoopDramatic` opens a framed setup screen (`Screen::DramaticSetup`) with:
     - **Matchup Row (`Left`/`Right`):** Cycles through **6 Signature 2v1 Rivalry Presets** plus **`CUSTOM 2V1`**:
       1. **`TWIN DRAGONS`:** *Ryu & Ken vs Sagat* (Stage: Thailand — Sagat)
       2. **`TEMPLE SCHISM`:** *Retsu & Geki vs Gen* (Stage: China — Gen)
       3. **`AMERICAN HEAVYWEIGHTS`:** *Joe & Mike vs Birdie* (Stage: England — Birdie)
       4. **`ROYAL GUARD`:** *Birdie & Eagle vs Sagat* (Stage: England — Eagle)
       5. **`MUAY THAI COUP`:** *Adon & Ryu vs Sagat* (Stage: Thailand — Adon)
       6. **`FINAL TRIAL`:** *Ryu & Sagat vs Gen* (Stage: Japan — Retsu)
       7. **`CUSTOM 2V1`:** Opens `SelectScreen` / `StageScreen` to freely pick `Ally 1` (Fighter + Style), `Ally 2` (Fighter + Style), `Boss` (Fighter + Style), and `Stage`.
     - **Era Family Row (for the 6 presets, `Left`/`Right`):** Defaults to the active preset's default Era (`SF1+` or `SFZ3` for classic Alpha-style Dramatic Battle), or lets players cycle the preset's Era Style (`SF1+`, `SF2 ST`, `SFZ3`, `SF3 3S`, `USF4`, `SF5 CE`, `SF6`).
     - **`LIFEBAR` Row (`Left`/`Right`):** `SHARED` (default) / `INDEPENDENT`.
     - **`FRIENDLY FIRE` Row (`Left`/`Right`):** `OFF` (default) / `ON`.
     - **`START FIGHT` (`Confirm`):** Launches `Fight` with `FightKind::Dramatic { co_op: bool }` and `DramaticSetup` (using the `Local`-style round-start snapshot boot path in `front/fight.rs` with `GAME_MODE = 2`, the chosen stage view, and `match_over()` completion into `AfterMatchBox` `REMATCH` / `RESELECT` / `MENU`).

#### B. 3-Fighter Real-Engine Execution (`P1_STRUCT`, `CPU_STRUCT`, and 3rd Slot)
1. **Work-RAM, Object-RAM, & `CoreToggleState` 3-Slot Assignment:**
   - **Ally 1 (`slot 0`):** Runs on `P1_STRUCT` (`0xff86b0`, 24-entry object-RAM region `0x400..0x9ff` = `0xffe400..0xffe9ff`), driven by `P1` pad inputs.
   - **Boss (`slot 1`):** Runs on `CPU_STRUCT` (`0xffc664`, 24-entry object-RAM region `0xa00..0xfff` = `0xffea00..0xffefff`), driven by the overhaul pad-feeding CPU AI (`ADR-0014` / `ES11`), targeting whichever living ally is horizontally closer each decision tick.
   - **Ally 2 (`slot 2`):**
     - Runs on `P2_STRUCT` (`0xff86da`) paired with a dedicated 3rd 24-entry (`0x600`-byte) object-RAM sprite grid region at **`0x1000..0x15ff` (`0xfff000..0xfff5ff`)** (with `clear_objects` / `clear_fighters` in `tasks.rs` and per-fighter state arrays in `CoreToggleState` and `combo::slot` expanded to `3` slots: `0` = `P1_STRUCT`, `1` = `CPU_STRUCT` / Versus `P2_STRUCT`, `2` = Dramatic Ally 2).
     - Initialized at round start at `x = P1_x - 36` (clamped inside the left stage boundary, facing the Boss), with its own `sub_hp: u16` (`960`), its own `EraFighterState`, its own input/charge/motion history, and its own active overhaul projectile slot. Its text-RAM health-bar pointer (`ps + 0xa`) is pointed to an off-screen scratch offset (`0x0000`, suppressed in `damage_with_chip` when `slot == 2`) so Ally 2 never overwrites the Boss's right-side health bar at `0x3bc`.
     - In `LocalCoopDramatic` (`co_op == true`), `vblank_buttons` (`fighter.rs`) enables `do_p2 = true` even in `GAME_MODE == 2` so `P2_STRUCT` latches `inputs.p2` every frame.
     - In `DramaticBattle` (`co_op == false`), Ally 2's input latch is fed by a second instance of the **overhaul pad-feeding CPU controller** (`ADR-0014` / `ES11`, never the 1987 `0x40bd4` `cpu_decide` table) targeting the Boss (`CPU_STRUCT`).
     - During each active fight frame (`0x3eafa`, as well as `intro_frame`, `fighters_busy`, `settle_frame`, and `outcome_frame` in `tasks.rs`), Ally 2 is stepped alongside `P1_STRUCT` and `CPU_STRUCT` through the shared `fighter.rs` player movement, posture, special-cancel, and era-mechanic routines, and `OverhaulScene` composites Ally 2's body and projectile sprites from object-RAM region `0x1000..0x15ff` (applying the P2 mirror palette if Ally 2 shares a character and palette with Ally 1 or the Boss).
     - Frame-flag slot swapping (`effect_flag_spawn` / `swap_regions` on flags `0x10` / `0x20`) is skipped in `FightKind::Dramatic` so all three fighters keep stable object-RAM regions (`0x400`, `0xa00`, `0x1000`).
2. **3-Fighter Hit / Hurt / Throw / Projectile Resolution:**
   - **Target pointer binding (`me + 0xe`, `me + 0x3c`, projectile `s + 0x24`):** Before running `judge` / hit / projectile collision between any attacker `A` and defender `D`, `fighter.rs` binds `A`'s target entity (`A + 0xe` high word) and target player-struct pointer (`A + 0x3c`, and projectile `s + 0x24`) to `D` so `damage_with_chip` and hit reactions always apply to the struck fighter.
   - **Allies -> Boss:** Both Ally 1's and Ally 2's active melee hitboxes, throws, and projectiles test collision against the Boss (`CPU_STRUCT` / `0xa00`). If both allies connect on the Boss on the exact same frame, Ally 1 resolves first and Ally 2 chains into the Boss's resulting hitstun/juggle state under the overhaul combo rules.
   - **Boss -> Allies:** The Boss's melee hitboxes and projectiles (`effect_vs_fighter` / `effect_vs_attack`) test collision against **both** living allies (`0x400` and `0x1000`) on the same frame—a sweeping Tiger Knee or projectile can hit or be blocked/parried by Ally 1, Ally 2, or both independently. The Boss's `push_apart` and throws test against whichever living ally is closer.
   - **Ally <-> Ally (`Friendly Fire`):**
     - When `FRIENDLY FIRE: OFF` (default), Ally 1 and Ally 2 never hit, throw, or push (`push_apart`) each other's boxes (they can cross through each other cleanly to flank the Boss).
     - When `FRIENDLY FIRE: ON`, Ally 1's and Ally 2's melee hitboxes and projectiles also test collision against each other (normal damage, block, and hitstun; throws and `push_apart` between allies remain disabled so co-op players do not accidentally throw or body-block each other when stacked).
   - **Auto-Facing:** Each frame at neutral/walk/crouch recovery, Ally 1 and Ally 2 face the Boss (`x_boss`), and the Boss faces whichever living ally is horizontally closer.

#### C. 3-Fighter Camera Clamping (`384`-Pixel View)
- During each fight frame in `FightKind::Dramatic`, the camera step (`0x43a58` override in `fighter.rs` / `tasks.rs`) computes `x_min` and `x_max` across all **living** fighters (`Ally 1` if alive, `Ally 2` if alive, and `Boss`):
  - Centers the `384`-pixel viewport at `clamp((x_min + x_max) / 2 - 192, 0, 0x180)` and updates `fg_scroll` accordingly.
  - Clamps all living fighters' world `X` coordinates within `[camera_x + 16, camera_x + 368]` so the two outermost living fighters cannot walk past opposite screen edges (`max_span <= 352 px`).

#### D. Lifebars, Boss Scaling, and Team Era Gauge
1. **Boss Scaling:**
   - The Boss starts each round with **`1536` Sub-HP** (`1.6 × 960`, synced to the right 48-px HUD bar as `ceil(boss_sub_hp * 48 / 1536)`) and deals **`1.25×` (`5 / 4`) outgoing hit and chip damage**.
2. **`LifebarMode::Shared` (default):**
   - Both allies share a single **`960` Sub-HP** pool (`team_sub_hp: u16`) displayed on the standard left 48-px HUD bar (`ceil(team_sub_hp * 48 / 960)`). Damage taken by either Ally 1 or Ally 2 deducts from `team_sub_hp`.
   - When `team_sub_hp` reaches `0`, **both** Ally 1 and Ally 2 enter their KO knockdown animation simultaneously and the Boss wins the round. On Time Over (`time_over_judge`), `team_sub_hp * 1536` is compared against `boss_sub_hp * 960`.
   - **Shared Team Era Gauge:** Both allies build and spend from a single shared **Team Era Gauge** rendered in `EraHud.p1` (bottom-left corner) against the Boss's Era Gauge in `EraHud.p2` (bottom-right corner). If Ally 1 and Ally 2 picked different Era Families in `CUSTOM 2V1` under `Shared` lifebar mode, the Team Era Gauge uses `Unified Gauge` (`GaugeMode::Unified`, 3 shared stocks) so both allies can spend stocks seamlessly.
3. **`LifebarMode::Independent`:**
   - Ally 1 (`960` Sub-HP) drives the main left 48-px HUD bar; Ally 2 (`960` Sub-HP) is displayed on a compact **48-px sub-bar drawn directly beneath P1's life bar** via `OverhaulScene` / HUD overlay.
   - Ally 1 and Ally 2 maintain separate `EraFighterState` gauges (`EraHud.p1` displays Ally 1's gauge with a compact secondary bar above it for Ally 2).
   - **Mid-round single-ally KO:** If Ally 1 reaches `0` Sub-HP while Ally 2 is still alive, Ally 1 plays the KO knockdown animation and stays down (`invulnerable`, `inputs ignored`, excluded from camera clamping, while `P1_STRUCT` keeps `1` sentinel HP byte in work RAM until Ally 2 also falls so `tasks.rs` does not end the round early) while Ally 2 continues fighting the Boss `1v1` (and vice versa if Ally 2 reaches `0` Sub-HP first). The round only ends in a Boss win when **both** Ally 1 and Ally 2 have `0` Sub-HP (or on Time Over comparing `max(ally1_sub_hp, ally2_sub_hp) * 1536` against `boss_sub_hp * 960`).

#### E. Pause -> `MOVES` Attribution & 3-Fighter Overlays in Dramatic Battle
- **Pause -> `MOVES` Attribution:** In `LocalCoopDramatic`, opening `MOVES` from the pause box inspects `paused_by` (`0` = Player 1, `1` = Player 2) and displays the move list for **Ally 1** when Player 1 paused or **Ally 2** when Player 2 paused.
- **3-Fighter Overlays (`Hitboxes`, `Frame Meter`, `Input Display`):**
  - When `Hitboxes` is enabled in `FightKind::Dramatic`, the overlay renders pushbox, hurtbox, and hitbox rectangles for **all 3 fighters** (`0x400`, `0xa00`, and `0x1000`).
  - The 2-column `Frame Meter` and `Input Display` strips remain bound to `Ally 1` on the left (`slot 0`) and the `Boss` on the right (`slot 1`, or `Ally 2` on the right in `LocalCoopDramatic` when `Input Display` is active) so the 2-column HUD layout never overlaps.

---

### 5.3 Pillar 3 — 1987 World Tour (`Single Player -> World Tour`)

#### A. Menu Entry & `WorldTourScreen` over the `0x3f812` World Travel Map
1. **Menu Item Enabled:** `MenuItem::WorldTour` under `SINGLE` (`Single Player -> World Tour`) opens `Screen::WorldTour(Box<WorldTourScreen>)`.
2. **Backdrop & Visual Layout (`crates/sf-engine/src/front/world_tour.rs`):**
   - `WorldTourScreen` renders the 1987 world travel map from `rom: &Assets` using the exact foreground tilemap window and palette loaded by `travel_map_start` (`0x3f812` in [`crates/sf-engine/src/tasks.rs`](../../crates/sf-engine/src/tasks.rs): `fg` layer enabled with `fg_scroll = 0x1980` and palette `0x1a268` at offset `0x200`).
   - **Node Graph Overlay:** Draws connected route lines and node markers across the five countries on the world map (`★` gold star for cleared nodes, pulsing gold cursor for the active node, bright white for unlocked unvisited nodes, dim grey for locked nodes).
   - **Framed Bottom Status & Command Bar:**
     - **Line 1:** `[COUNTRY] — NODE TITLE` (e.g. `JAPAN 1 — SHORINJI DOJO`) and `[CLEAR ★]` or `[XP REWARD: +150]`.
     - **Line 2:** `OPPONENT:` (e.g. `JOE -> BIRDIE -> EAGLE (3 WAVES)`) and `RULE:` (e.g. `SURVIVAL: SINGLE LIFEBAR`).
     - **Line 3:** `FIGHTER: RYU [SFZ3 A-ISM]` | `RPG: ON (LV 3 — ATK +2 DEF +1 SPD +1 | PERK: METER SURGE)`.
     - **Line 4 (Prompt):** `CONFIRM: FIGHT   TAB L: FIGHTER/STYLE   TAB R: RPG STATS & PERKS   BACK: MENU`.
3. **Sub-Overlays on `WorldTourScreen`:**
   - **`Tab Left` (`Fighter / Style Select`):** Opens `SelectScreen` (with the `ES1` Style card) so the player can switch to any of the 12 fighters and any Era Style / Revision / Loadout before starting a node. (On `EraMastery` nodes that require a featured Era Family, the node lends/locks that Era Family for the match while keeping the player's chosen Fighter and Loadout.)
   - **`Tab Right` (`RPG Stats & Perks Modal`):** Opens a framed modal over the map where the player can:
     - Toggle **`RPG BOOSTS: ON / OFF`** at any time (for purists who want a pure skill run vs RPG progression);
     - Spend accumulated **`XP`** to upgrade **`ATTACK` (`0..=5`)**, **`DEFENSE` (`0..=5`)**, and **`SPEED` (`0..=5`)** (or refund points freely to experiment with builds);
     - Equip one of **5 Unlockable Perks** (unlocked by clearing designated Era Mastery or Survival nodes).

#### B. The 18-Node 5-Country Branching Map & 6 Real-Fight Mission Types

Each of the **18 nodes** specifies `(id, country, title, mission_kind, opponents, stage, xp_reward, clear_unlocks_nodes, clear_unlocks_perk)`:

| # | Node ID | Country | Mission Type | Matchup & Special Rules | Unlocks Next |
|---|---|---|---|---|---|
| 1 | `jp-1-dojo` | Japan | **Rival Bout** | vs **Retsu** (`SF1+`, 2/3 rounds) | `jp-2-shuriken`, `jp-3-era` |
| 2 | `jp-2-shuriken` | Japan | **Rule-Modifier Trial** (`Projectile Defense`) | vs **Geki** (1 round, `60` timer ticks; Geki has halved shuriken cooldown; win by KO or surviving until `TIME == 0`) | `jp-4-boss` |
| 3 | `jp-3-era` | Japan | **Era Style Mastery** (`SF2 ST`) | vs **Ryu** (`SF2 ST` lent; 2-in-1 cancel & Super Combo showcase; unlocks **`VITAL SPIRIT`** perk) | `jp-4-boss` |
| 4 | `jp-4-boss` | Japan | **Country Finale — 2v1 Dramatic Boss** | **Player + Retsu** vs **Boss Geki** (`2v1` Dramatic Battle on Japan — Geki stage) | `us-1-ring`, `cn-1-wall` |
| 5 | `us-1-ring` | USA | **Rival Bout** | vs **Joe** (`SF1+`, 2/3 rounds) | `us-2-rush`, `us-3-era` |
| 6 | `us-2-rush` | USA | **Rule-Modifier Trial** (`No Blocking`) | vs **Mike** (`No Blocking` active for P1—must use spacing, whiff punish, or invincible reversals) | `us-4-boss` |
| 7 | `us-3-era` | USA | **Era Style Mastery** (`SFZ3`) | vs **Ken** (`SFZ3` lent; `A-ISM`/`X-ISM`/`V-ISM` showcase; unlocks **`ISM PLUS`** perk) | `us-4-boss` |
| 8 | `us-4-boss` | USA | **Country Finale — 2v1 Dramatic Boss** | **Player + Joe** vs **Boss Mike** (`2v1` Dramatic Battle on USA — Mike stage) | `en-1-punk` |
| 9 | `cn-1-wall` | China | **Time Attack** | Defeat **Lee** in a single round within **`40` ROM timer ticks** (`TIME_DIGITS`) | `cn-2-combo`, `cn-3-era` |
| 10 | `cn-2-combo` | China | **Rule-Modifier Trial** (`Combo-Only`) | vs **Gen** (first hit of any P1 sequence deals `0` damage; only hits with `combo_count >= 2` deal damage) | `cn-4-boss` |
| 11 | `cn-3-era` | China | **Era Style Mastery** (`SF3 3S`) | vs **Lee** (`SF3 3S` lent; Parry & Super Art showcase; unlocks **`GUARD MASTER`** perk) | `cn-4-boss` |
| 12 | `cn-4-boss` | China | **Country Finale — 2v1 Dramatic Boss** | **Player + Lee** vs **Boss Gen** (`2v1` Dramatic Battle on China — Gen stage) | `en-1-punk` |
| 13 | `en-1-punk` | England | **Survival Gauntlet** (`3 Waves`) | Defeat **Joe -> Birdie -> Eagle** sequentially on a single lifebar (`sub_hp` carries across waves, `+20%` recovery per wave clear) | `en-2-footsies`, `en-3-era` |
| 14 | `en-2-footsies` | England | **Rule-Modifier Trial** (`Pure Footsies`) | vs **Eagle** (`No Specials / No Supers` for both fighters—normals and throws only) | `en-4-boss` |
| 15 | `en-3-era` | England | **Era Style Mastery** (`USF4` / `SF5 CE`) | vs **Birdie** (`USF4` lent; Focus/Ultra showcase; unlocks **`AIR GUARD`** perk) | `en-4-boss` |
| 16 | `en-4-boss` | England | **Country Finale — 2v1 Dramatic Boss** | **Player + Eagle** vs **Boss Birdie** (`2v1` Dramatic Battle on England — Birdie stage) | `th-1-gauntlet`, `th-2-era` |
| 17 | `th-1-gauntlet` | Thailand | **Survival Gauntlet** (`4 Waves`) | Defeat **Retsu -> Gen -> Eagle -> Adon** on a single lifebar (`+25%` recovery per wave clear; unlocks **`METER SURGE`** perk) | `th-2-era` |
| 18 | `th-2-era` | Thailand | **Grand Finale — Era Mastery & 2v1 Dramatic Boss** | **Player + Ken** (or **Ryu** when playing as Ken, `SF6` Drive System available) vs **Emperor Sagat** (`2v1` Dramatic Battle on Thailand — Sagat stage) | Campaign Clear ★ |

*(Clearing either `us-4-boss` or `cn-4-boss` opens England. On clearing `th-2-era`, the game plays the active fighter's congratulatory ending screen and the 1987 staff roll (`0x44b00..0x44c1a`), then returns to `WorldTourScreen` with all 18 nodes unlocked and replayable for any Fighter or Era Style.)*

#### C. Real-Engine Enforcement of Mission Types & RPG Stats/Perks
All World Tour nodes run as real `Fight` sessions (`FightKind::WorldTour` or `FightKind::Dramatic` for the Country Finale nodes) booted from the `Local`-style round-start snapshot in `front/fight.rs` with `TourRules` on `MatchConfig` / `CoreToggleState`:
1. **Six Mission Mechanics in `Fight` / `fighter.rs`:**
   - **`RivalBout`:** Standard 2-out-of-3 match on the node's stage.
   - **`SurvivalGauntlet { waves }`:** Single-round waves (`1..=N`). When the CPU opponent is KO'd on wave `w < N`, `Fight` records P1's remaining `sub_hp` (restoring `+20%` or `+25%` up to `960`), boots the next wave's `Fight` snapshot for opponent `w + 1`, and immediately writes the carried `sub_hp` into P1's `CoreToggleState` and syncs `P1_STRUCT + 6..+7` and the left text-RAM health bar (`0x8002bc`) after `round_hud` runs, continuing until all waves are cleared or P1 is KO'd.
   - **`TimeAttack { max_ticks }`:** Sets the ROM round timer (`TIME_DIGITS` at `0xff8e18`) to `max_ticks` at round start; Time Over is always a loss even if P1 has more health.
   - **`RuleModifierTrial`:**
     - `NoBlocking`: Disables standing/crouching/air guard state transition in `fighter.rs` for P1 (`P1_STRUCT`).
     - `ProjectileDefense`: Sets 1 round (`60` ROM timer ticks); CPU opponent's projectile cooldown is halved, and Time Over with `p1_sub_hp > 0` counts as a P1 win.
     - `ComboOnly`: In `fighter.rs` damage application for P1 (`P1_STRUCT` attacking `CPU_STRUCT`), when `combo_count < 2` (the opening hit of a string), P1's attack deals hitstun and builds meter normally but deducts `0` `sub_hp` from the CPU opponent; hits with `combo_count >= 2` deal `1.25×` damage (CPU attacks against P1 deal normal damage).
     - `PureFootsies`: Suppresses special-move and super-move command recognition in `fighter.rs` for both fighters (normals, command normals, and throws only).
   - **`EraStyleMastery { required_era }`:** Temporarily sets P1's `EraFamily` to `required_era` for the bout while preserving P1's fighter and loadout pick.
   - **`DramaticBoss`:** Runs a 3-fighter `2v1` Dramatic Battle (`Section 5.2`) with the node's designated CPU partner and Boss.
2. **RPG Stats & 5 Unlockable Perks (Applied Only to P1 in `WorldTour` When `rpg == true`):**
   - **`ATTACK` (`0..=5`):** `+4%` outgoing hit damage per level (`+20%` at Level 5).
   - **`DEFENSE` (`0..=5`):** `-4%` incoming hit/chip damage taken per level (`-20%` at Level 5).
   - **`SPEED` (`0..=5`):** `+3%` walk/dash horizontal velocity per level (`+15%` at Level 5).
   - **5 Unlockable Perks (1 equipped at a time):**
     1. **`VITAL SPIRIT`:** Regenerates `2` `sub_hp` every `30` frames while at neutral (`<= 960`).
     2. **`METER SURGE`:** Starts every round with `+1` bar/stock of Era meter and increases meter gain by `+25%`.
     3. **`GUARD MASTER`:** Negates all block chip damage (`chip = 0`) and reduces guard pushback/guard-crush damage by `50%`.
     4. **`AIR GUARD`:** Grants `SFZ`-style Air Block (`air_block = true`) regardless of the active Era Family.
     5. **`ISM PLUS`:** Reduces all metered era costs (`EX`/`OD`, `FADC`, `V-Trigger`, `Super`) by `25%`.
3. **Persistence (`%APPDATA%\sf1-port\world_tour.toml`):**
   - Saved automatically after every node clear or RPG stat/perk change (using atomic temp-file + rename, matching `settings.toml` in [`crates/sf-engine/src/front/settings.rs`](../../crates/sf-engine/src/front/settings.rs)):
     ```toml
     fighter = "Ryu"
     era = "SF1+"
     revision = "SF1+"
     loadout = "Default"
     current_node = "jp-1-dojo"
     rpg = true
     xp = 350
     attack = 2
     defense = 1
     speed = 1
     perk = "MeterSurge"
     cleared_nodes = ["jp-1-dojo", "jp-2-shuriken", "jp-3-era"]
     unlocked_perks = ["VitalSpirit", "MeterSurge"]
     ```

---

### 5.4 Retiring the Standalone Prototype Sessions (`arcade.rs`, `dramatic.rs`, `world_tour.rs`)
- Once the real-engine integration tests cover:
  1. All 12 fighters' Arcade endings, stand-in Ryu montage/win-card swaps, and post-match win quotes;
  2. 3-fighter `2v1` Dramatic Battle (`Single Player -> Dramatic Battle` and `Multi Player -> Local Co-Op Dramatic`, `Shared` and `Independent` lifebars, `Friendly Fire`, 3-fighter camera clamping, and Team Era Gauge);
  3. The 18-node `WorldTourScreen`, all 6 mission types on the real `Fight` engine, and `world_tour.toml` RPG persistence;
- Delete the standalone prototype stepping structs in [`crates/sf-engine/src/arcade.rs`](../../crates/sf-engine/src/arcade.rs) (`run_arcade`, `BonusStage`, `Ending`), [`crates/sf-engine/src/dramatic.rs`](../../crates/sf-engine/src/dramatic.rs) (`DramaticBattle` 1D body/shot simulation), and [`crates/sf-engine/src/world_tour.rs`](../../crates/sf-engine/src/world_tour.rs) (`WorldTour` wrapper over `dramatic.rs`), keeping only the pure data tables (`win_quote`, `ending_epilogue`, `DramaticPreset`, `WorldTourNode`, `WorldTourProfile`) that drive the real engine.

---

### 5.5 Domain Glossary Additions (`CONTEXT.md`, staged for `ED1`)
To avoid merge conflicts with active Project 3 worktrees, the first ticket (`ED1`) adds these four entries to `CONTEXT.md`:
- **Composite ending view** — A 24-column `LayerView` built in the **Extended background bank** (`BackgroundBank`) at startup (`ending:{Boss}`) from a boss's 6×6 ROM VS bust (`0x3fad6`), nameplate, and home-stage palette (`0x43778`), paired with a character-specific epilogue block in the **Extended data bank** (`0x100000`) typed by the 1987 `TextWriter` (`0x3bb98`) during the ending sequence (`0x44b00..0x44b86`). (`ADR-0016`.)
- **Dramatic Battle** — A 3-fighter `2v1` match on the real 1987 engine (`Single Player -> Dramatic Battle` with a CPU teammate or `Multi Player -> Local Co-Op Dramatic` with a human P2 teammate) where Ally 1 runs on `P1_STRUCT` (`0x400..0x9ff`), the Boss runs on `CPU_STRUCT` (`0xa00..0xfff`, `1536` Sub-HP, `1.25×` damage), and Ally 2 runs on `P2_STRUCT` paired with a third 24-entry object-RAM region (`0x1000..0x15ff`) and a third slot in `CoreToggleState`. (`ADR-0016`.)
- **Team Era Gauge** — The shared bottom-left **Era gauge** (`EraHud.p1`) built and spent jointly by both allies in **Dramatic Battle** when `LifebarMode::Shared` is active (`Unified Gauge` 3-stock bar when the allies pick different Era Families). (`ADR-0016`.)
- **World Tour profile** — The persistent single-player campaign state saved to `%APPDATA%\sf1-port\world_tour.toml`, tracking unlocked/cleared nodes across the 18-node 5-country map on `WorldTourScreen`, the `RPG: ON/OFF` toggle, `XP`, `ATTACK`/`DEFENSE`/`SPEED` (`0..=5`), and the 5 unlockable Perks (`VitalSpirit`, `MeterSurge`, `GuardMaster`, `AirGuard`, `IsmPlus`). (`ADR-0016`.)

---

## 6. What's Left to the Builder

The following implementation details are intentionally left to engineering judgment during ticket execution:
- Exact helper signature in [`crates/sf-engine/src/fighter.rs`](../../crates/sf-engine/src/fighter.rs) for stepping Ally 2 (`P2_STRUCT` + object-RAM region `0x1000..0x15ff`) through the shared fighter movement, command, and collision routines without duplicating 2-player code paths.
- Exact pixel coordinates `(x, y)` of the 18 node markers across the five countries on the `0x3f812` world travel map foreground in `WorldTourScreen`, and exact y-offset of Ally 2's compact sub-lifebar beneath P1's HUD bar in `LifebarMode::Independent`.
- Exact address offsets within the extended data bank (`0x100000`) for the 10 boss epilogue blocks, the 12 post-match win-quote blocks, and the 2×2 `RYU` montage card.
