# Roguelite World Tour, hybrid builds, active tag and prop minigames: design

- **Date:** 2026-11-18
- **Size:** Project (replacing the static 18-node World Tour with a seeded roguelite run, between-round 3-card drafts, escalating loss burns, meta-progression starting unlocks, slot-shared `TourBuild` modifiers/statuses/cross-era move grafting/CPU specialists, 3rd-slot active tag swaps, and 2,048-column side-scrolling prop minigames).
- **Marking:** `"(user)"` is the maintainer's decision in brainstorming; `"(user accepted)"` is a proposal the maintainer confirmed in playback; `"My call"` is the designer's choice, which the maintainer has not been asked about one by one.

## Intent and the Why

Today's World Tour (`crates/sf-engine/src/world_tour/native.rs`, `crates/sf-engine/src/front/world_tour.rs`, `crates/sf-engine/src/front/session/world_tour.rs`, `crates/sf-engine/src/era_fight/world_tour.rs`, `ADR-0016`) is a fixed 18-node checklist across the five 1987 countries backed by one shared profile, three 0–5 stats, five perks, no story, and XP paid only on a node's first clear. Because `docs/roadmap.md` keeps every fighter, era style, costume, and palette unlocked from the start across Arcade, Local, Training, and Online, World Tour cannot rely on gating the core roster elsewhere; once a player finishes those 18 fights in under an hour, there is little reason to return.

This revamp turns World Tour into a **single-player roguelite run** (`(user)`): a dedicated home for creative fight variations, engine-stretching builds, silliness, *Street Fighter II: Rainbow Edition* / *Koryu*-style chaos, and intense retro-homage minigames that would not belong in balanced multiplayer (`(user)`). Every run begins barebones and lets the player assemble an outrageous hybrid fighter round by round, while clearing runs and milestones unlocks new starting options for future runs (`(user)`).

This design supersedes the World Tour portion of `ADR-0016` while leaving `ADR-0016`'s `2v1` Dramatic Battle and 12-fighter ROM-composite Arcade endings untouched. It also updates how open Bonus Challenger and Fair CPU World Tour tickets (`#33`, `#51`, `#64`, `#215`) fit in: instead of appending static nodes `19..33` to a fixed graph, released Bonus Challengers and the stronger Fair CPU feed directly into World Tour's run node pools, specialist templates, partner bench, and meta-unlock tracks. It will require a new ADR and `CONTEXT.md` glossary updates in the next workflow phase (`grill-with-docs`).

## Goals

1. **From vanilla `SF1` to wild late-run hybrids (`(user)`):** A fresh World Tour save starts in vanilla `SF1` with no era styles available inside the mode (`(user)`). Over a single run, the player unlocks and stacks Era Styles, cross-era grafted moves from that fighter's other eras (for example, an `SFZ3` Ryu with Ice-infused attacks and `SF6` command normals), elemental effects, *Rainbow Edition* mutators, and a tag partner (`(user)`).
2. **Per-round and per-match 3-card upgrade drafts (`(user)`):** Winning each round immediately presents three upgrade cards before the next round begins; winning the match locks in that match's round picks, grants one bonus 3-card draft (three total upgrades per won match), and restores the player to full health (`(user)`).
3. **Upgrades as your lifeline (`(user)` + `(user accepted)`):** Losing a match discards any unbanked round picks from that match and costs banked upgrades (`(user)`): continuing costs `1`, `2`, `3`… banked upgrades of the player's choice to step back onto the map, or triggers **Run Over** when the player cannot pay (`(user accepted)`).
4. **Build-specialized opponents (`(user)`):** Runs feature CPU opponents constructed around extreme build archetypes—such as all-fire specialists, status specialists who slow the player down, and speed specialists whose animations run up to 4× normal speed (`(user)`).
5. **Retro-homage minigames with extensible props (`(user)`):** Runs mix in intense minigames—highlighted by a left-to-right *Super Mario Bros.*-style scrolling course with breakable and dodgeable props—launching immediately with 1987 ROM props and expanding through a data-driven prop registry as PixelLab credits renew (`(user)` + `(user accepted)`).
6. **Between-run starting unlocks (`(user)` + `(user accepted)`):** Finishing a run (or hitting milestone clears before a Run Over) unlocks new starting choices—fighters, era styles, and starting relics—that were not available at the start of prior runs (`(user)` + `(user accepted)`), while leaving Arcade, Local, Dramatic Battle, Training, Online, and Matchmaking completely unlocked from day one (`(user)`).

## Non-goals

- Gating any fighter, era style, costume, stage, or palette outside World Tour (`(user)`).
- Grafting moves from a *different* fighter whose sprite poses the active fighter does not have (`(user accepted)` — grafted command normals and specials stay within the active fighter's own built eras).
- Grafting metered or era-system donor moves (`EX`/`OD` specials, `Drive` Impact/Parry/Rush/Reversal, `Focus` Attack, `V-Skill`/`V-Trigger`, `Super Arts`), which depend on era gauges that `SF1`, `SF1+`, and `V-ISM` lack (`(user)`).
- Blocking launch on external PixelLab prop art generation (`(user accepted)` — launch with ROM props first, then grow the prop library incrementally).
- Auto-saving `.sf1replay` recordings for World Tour bouts or minigames (`*My call:*` keeps today's `leave_world_tour_fight` call to `finish_replay_recording(false)`, avoiding `.sf1replay` format churn for mid-match draft pauses and uncapped scrolling courses).
- Online or local 2-player co-op World Tour runs.

## Anti-goals

- **Zero leakage into `Faithful 1987` or competitive modes:** No World Tour build modifier, status effect, animation-speed multiplier, uncapped camera scroll, or grafted move ever fires outside World Tour fights (`FightKind::WorldTour` and World Tour's Dramatic/Tag encounters). All 137 MAME reference scenarios remain bit-for-bit identical.
- **No parallel combat engine:** Do not bring back standalone 2D combat structs outside `Fight` (`ADR-0004`, `ADR-0016`). Every bout, tag swap, and side-scrolling prop minigame runs on the translated 1987 machine.
- **No 25-loss slogs or random build destruction:** Escalating burn costs prevent dragging out a doomed late run across dozens of losses, and the player always chooses which banked upgrade(s) to burn.

## Constraints

- **3-slot fighter and object-RAM layout (`crates/sf-engine/src/fighter.rs`, `tasks.rs`, `ADR-0016`):** Keep the engine's 3-slot limit (`(user)`) with no 4th fighter slot: Slot `0` is `P1_STRUCT` (`0xff86b0`, object RAM `0x400..0x9ff`), Slot `1` is `CPU_STRUCT` (`0xffc664`, object RAM `0xa00..0xfff`), Slot `2` is `P2_STRUCT` (`0xff86da`, object RAM `0x1000..0x15ff`), and `0xffe100..0xffe3ff` provides twelve `0x40`-byte projectile/effect object slots.
- **Art-pack requirement for non-shoto fighters (`crates/sf-engine/src/front/session/world_tour.rs`):** Only Ryu and Ken have native 1987 ROM player move tables; the ten 1987 bosses require the local `SF1+` art pack (`player_tables()`) to play overhaul kits, `TourBuild` moves, or tag/playable roles. `*My call:*` Entering World Tour checks that all 12 tournament fighters are present in `player_tables()` and surfaces the existing `NEEDS THE SF1+ ART PACK` notice if missing. Bonus challengers enter World Tour pools automatically whenever their roster wave is released and `config.roster.bonus_challengers` is enabled.
- **2,048-column `BackgroundBank` pages (`crates/sf-video/src/background_bank.rs`, `ADR-0009`):** Each extended background map page holds `PAGE_COLS = 2048` columns (`32,768` pixels across `16` rows) on both `Bg` (`gfx1`) and `Fg` (`gfx2`) layers. Standard fights clamp `CAMERA` (`0xffc680`) to `0..=0x180`; uncapping `CAMERA` up to a course's pixel width under a World Tour minigame rule enables multi-screen left-to-right scrolling on the existing video renderer.
- **Indexed move-effect strips (`crates/sf-video/src/move_effects.rs`, `signature.rs`):** Embedded 16-color indexed strips (`KenFire`, `KenBurn`, `DenjinHands`, `DenjinProjectile`, `DenjinImpact`, `Satsui`, `Zanei`, `LandingDust`, `Parry`) carry explicit `palette` arrays (`[u32; 16]`) on `IndexedSprite`, allowing Ice/Frost, Fire, Shock, and Venom overlays immediately via palette swaps without waiting on new art.
- **Six-button cabinet input, scheme compatibility, and pad-only UI (`crates/sf-engine/src/control.rs`, `front/rebind.rs`, `#129`):** Every World Tour screen, draft modal, and burn picker must navigate purely via `MenuInput` on a controller/Steam Deck. No new keybinds are added (`bindings.toml` and `RebindScreen` stay untouched) (`(user)`). Because `ControlScheme::Modern` lacks `HK` (`[light, medium, heavy, special, assist, macro_button]`) and `ControlScheme::Pneumatic` fires light attacks on release and heavy on press (and 2-button setups omit `HK`), Active Tag and Tag Cancel are detected on **raw pre-translation inputs** before `translate_player_controls` runs (`(user)`): **`LP+HK`** under Classic, **`Special+Macro`** under Modern, and **`Down, Down + Punch+Kick`** under Pneumatic, suppressing the normal/macro output on the trigger frame.

## Approaches

1. **Continuous `Fight` overlay + runtime `TourBuild` dispatch + `Fight`-driven minigames — Chosen (`(user)`):**
   - Matches run inside a single continuous `Fight` (`crates/sf-engine/src/front/fight.rs`). When `P1` wins a round, `GameSession` pauses immediately after the round-result tally behind a 3-card draft overlay, writes the chosen upgrade into `TourBuild` inside `CoreToggleState`, and resumes so Round 2 starts natively with gauges and round markers intact.
   - All three fighter slots (`0..3`) carry a `TourBuild` in `CoreToggleState`. Grafted command normals and specials redirect at input-recognition time to the donor era's existing `DataBank` move slot/stream for that fighter (alongside newly authored signature `SF6` command normals) rather than generating combinatorial move tables. Active Tag uses slot `2` (`P2_STRUCT` / `0x1000..0x15ff`) for a leap-in entry strike when slot `2` is free and swaps in-place when slot `2` is occupied (`1v2` Ambushes / `2v1` Finales). Side-scrolling homage minigames run inside `Fight` over a `2,048`-column `BackgroundBank` page with uncapped `CAMERA` and hittable/hazardous prop objects.
   - *Trade-off:* Keeps native round flow, real fighter physics and upgrades in minigames, and zero `DataBank` combinatorial bloat, at the cost of disciplined `world_tour_rules()` gating in `fighter.rs` and clean donor-move state resets when grafted attacks return to neutral.
2. **Per-round `Fight` reboots + synthesized `DataBank` tables + standalone `MinigameScreen` — Rejected:**
   - Reboots a fresh single-round `Fight` after each round draft (extending today's `SurvivalGauntlet` wave loop in `front/session/world_tour.rs`), synthesizes a custom 64-slot move table in scratch `DataBank` memory for each hybrid kit, and runs scrolling minigames on a separate `Screen` outside `FightMachine`. Rejected because rebooting `Fight` mid-match breaks native round flow unless win icons and era meters are manually patched back into RAM, complicates mid-round partner swaps, and forces minigames to duplicate simplified fighter physics instead of letting your real upgraded build interact with props.
3. **Dual-live structs (`P1_STRUCT` + `P2_STRUCT`) for Active Tag — Rejected:**
   - Uses Approach 1's continuous `Fight` draft overlay, runtime `TourBuild` dispatch, and `Fight`-driven minigames, except lead and partner stay permanently resident in `P1_STRUCT` and `P2_STRUCT` all round and Tag routes `P1`'s pad inputs between them while parking the inactive partner off-screen. Rejected because although rapid tag-cancels avoid migrating charge buffers, dozens of checks across `tasks.rs`, `era_fight/world_tour.rs`, HUD bars, and KO handling assume the human player is always `P1_STRUCT`, widening engine risk unnecessarily.

## What was decided

### 1. Fresh start and between-run meta-progression

- **Barebones initial state (`(user)` + `(user accepted)`):** A brand-new World Tour profile starts with **Ryu** and **Ken** unlocked in vanilla **`SF1`** (`EraFamily::Sf1987`), zero Era Styles unlocked as starting options, and no starting relics (`(user)` + `(user accepted)`).
- **Pre-run setup and three unlock tracks (`(user)` + `(user accepted)`):** Ending a run unlocks new starting options on a pre-run **Run Setup** screen across three tracks (`(user)` + `(user accepted)`):
  1. **Starting Fighters:** Unlocks the ten 1987 tournament bosses (plus released Bonus Challengers when `config.roster.bonus_challengers` is enabled). Because only Ryu and Ken have native `SF1` (`Arcade1987`) tables, recruited bosses and bonus challengers default to `SF1+` as their starting base style (`(user accepted)`).
  2. **Starting Era Styles:** Unlocks `SF1+`, `SF2`, `SFZ`, `SF3`, `SF4`, `SF5`, `SF6`, and `SF KESSEN` (where built for the chosen fighter) as starting styles on Run Setup instead of always starting in `SF1`. `*My call:*` Unlocking an Era Family unlocks its flagship revision for Run Setup and unlocks that family's revision/loadout cards in mid-run drafts; inside World Tour, built what-if styles are available regardless of the multiplayer `what_if_styles` toggle so every fighter has full hybrid variety.
  3. **Starting Relics:** Unlocks one Fight-1 keepsake slot for the run (such as starting with an Elemental Core, a pre-signed Partner contract, or a *Rainbow* bootleg chip).
- **How unlocks are earned (`*My call:*`):** First-time clears of a Country Boss (`Japan`, `USA`, `China`, `England`) in a run bank `1` meta-unlock pick for the end of that run, and clearing the `Thailand` Grand Finale awards `2` meta-unlock picks (with repeat clears still granting `1` pick if locked items remain). At **Run Clear** or **Run Over**, any earned unlock picks present a 3-card choice of locked items across the three tracks so every meaningful run expands future runs' starting possibilities.
- **Pure-run challenge switch (`*My call:*`):** Run Setup includes a `PURE RUN: OFF / ON` toggle (`OFF` by default) for purists who want to play a seeded run's crazy enemy archetypes and minigames without mid-run upgrade cards.

### 2. Run map, pacing, round/match drafts, and loss burns

- **World travel map backdrop (`crates/sf-engine/src/front/world_tour.rs`, `ADR-0016`):** Keep the 1987 `0x3f812` world travel map (`fg_scroll = 0x1980`, palette `0x1a268`) as the run map backdrop, drawing connected route nodes, clear stars, and specialist/minigame badges over the map.
- **Seeded 5-country branching run (`*My call:*`):** Starting a run generates a seeded branching tour across `Japan -> USA / China -> England -> Thailand` (`10–12` stops total: `2–3` nodes per country mixing Specialist Bouts, *Rainbow* Twist / `1v2` Ambush Bouts, at least `1` Homage Minigame stop per region, and a Country Finale Boss; ~20–30 minutes per run). Every map node previews the opponent fighter(s), specialist archetype badge, or minigame title.
- **Round and match 3-card drafts (`(user)` + `(user accepted)`):**
  - Winning **any round** of a bout immediately opens a 3-card upgrade draft overlay inside `GameSession` before the next round starts; the chosen card applies to `P1`'s `TourBuild` right away for the following round (`(user)`).
  - Winning **the match** locks those round picks into the run's banked upgrade list, opens one bonus 3-card draft (yielding **three total upgrades** per won 2-of-3 match), and restores `P1` to full `960` Sub-HP before returning to the map (`(user)` + `(user accepted)`).
  - Defeating a build-specialized opponent guarantees at least one card matching their specialty (e.g., Fire, Chill/Status, 4× Speed) in the match-win draft (`(user accepted)`).
- **Escalating loss burns, pause rules, and Run Over (`(user)` + `(user accepted)`):**
  - Losing a match discards any unbanked round picks from that match and increments the run's defeat counter `L` (`1` on first loss, `2` on second loss, `3` on third, and so on) (`(user accepted)`).
  - If the player holds at least `L` banked upgrades, a **Burn `L` Upgrades** modal lets the player choose exactly which `L` banked card(s) to sacrifice, strips their effects from `TourBuild`, and steps the player back onto the map (allowing a rematch or taking an alternate branch/minigame) (`(user accepted)`).
  - If the player has fewer than `L` banked upgrades (including losing Fight 1 with `0` banked cards), the run ends in **Run Over** (`(user accepted)`), still awarding any meta-unlock picks earned from country milestones already cleared in that run (`(user accepted)`).
  - World Tour combat bouts use a dedicated pause menu (`PauseKind::WorldTour`) that removes `RESTART ROUND` and replaces `QUIT TO MENU` with **`FORFEIT MATCH`** (`(user)`). Choosing `FORFEIT MATCH` mid-bout—or force-closing the game mid-bout—counts as a match defeat so players cannot dodge Loss Burns (`(user)`). Quitting from the world map saves cleanly with no penalty (`(user)`).
- **Card tiers (`*My call:*`):** To keep ~25–30 upgrades across a full run clean and exciting without cluttering inputs, cards divide into **Major Cards** (base Era Style shifts, Grafted Moves, Partner Contracts, *Rainbow* Keystones) and **Rank / Synergy Cards** (leveling owned elemental cores, mutators, grafted slots, or partners from Rank 1 to Rank 3, + projectile count, + status potency, or meter/vitality bonuses). Early rounds weight Era Style shifts and first elemental/move unlocks heavily so players rapidly escape `SF1`'s barebones start.

### 3. Shared `TourBuild` engine (player, CPU specialists, and partners)

- **Unified `[TourBuild; 3]` state (`(user accepted)`):** Today's `era_fight/world_tour.rs` hard-codes World Tour bonuses to `P1_STRUCT`. Instead, `CoreToggleState` stores `TourBuild` for all three fighter slots (`0 = P1_STRUCT`, `1 = CPU_STRUCT`, `2 = Partner / 2nd CPU`) inside `WorldTourRules` so every elemental effect, status ailment, *Rainbow* mutator, grafted move, and speed multiplier runs uniformly for the player, enemy specialists, and tag partners (`(user accepted)`).
- **Base Era Style unlocks mid-run (`(user)`):** Drafting an Era Style Shift card switches the fighter's active `StylePick` (for example from `SF1` into `SFZ3` or `SF6`), initializing that era's gauge and base move list on the next round.
- **Cross-era move grafting (`(user)` + `(user accepted)`):**
  - Drafting a **Grafted Move** card attaches a meterless special or command normal from another built era of the **same fighter** (e.g., `SFZ3` Ryu grafting `SF6` Ryu's command normals or Hashogeki) (`(user)` + `(user accepted)`).
  - Because `EraFamily::Sf6` in `crates/sf-engine/src/era_fight/sf6.rs` currently lacks `MoveKind::Normal` command normals, this project authors missing signature `SF6` command normals in `DataBank` and `era_fight/sf6.rs` (such as Ryu's `6+MP` Solar Plexus Strike / `6+HP` and peer command normals for built `SF6` fighters) so cross-era command-normal hybrids work natively (`(user)`).
  - **Metered and era-system donor moves** (`EX`/`OD` specials, `Drive` Impact/Parry/Rush/Reversal, `Focus` Attack, `V-Skill`/`V-Trigger`, `Super Arts`) are excluded from Move Grafting so grafted moves never need gauge conversion or donor mini-gauges (`(user)`).
  - `*My call:*` A fighter can hold up to `3` grafted move slots (offering a swap modal if all 3 are full); Pause -> `MOVES` appends active grafted moves and their inputs below the base era's move list.
  - At input recognition time in `fighter.rs`, a grafted move dispatches straight to the donor era's move slot/stream in `DataBank` and clears any transient donor-era flags when returning to neutral, requiring no generated combinatorial move tables in `DataBank`.
- **Elemental effects and statuses (`(user)` + `(user accepted)`):** Attacks carrying an elemental tag emit recolored indexed overlays (`KenFire`/`KenBurn`, cyan Frost palettes, `Denjin` arcs, `Satsui` dark flames) and apply status effects tracked per slot:
  - **Fire:** Ignites the victim on hit or heavy block, ticking periodic burn Sub-HP damage (capped so burn alone leaves at least `1` Sub-HP unless the final blow hits).
  - **Ice / Chill (`(user)`):** Chills the victim on hit, reducing horizontal walk/dash speed and stretching animation frame holds (`0x33`) by `1.5×–2×` (at max chill stacks, triggers a brief frost-stun window before clearing).
  - **Shock:** Electrifies attacks, adding bonus hitstun/blockstun frames and extra dizzy/stun build-up.
  - **Venom / Satsui:** Dark-flame hits dealing heightened chip damage and leeching a sliver of Sub-HP back to the attacker on specials.
- ***Rainbow Edition* / *Koryu* mutators (`(user)`):**
  - **Hyper-Speed Animations (`2×` to `4×`, `(user)`):** Steps a fighter's frame-hold countdown (`eb(me, 0x33)`) and movement delta multiple times per tick (`*My call:*` capped at `4×` with input polling kept at 60 Hz so player controls stay readable while movement and attacks fly).
  - **Airborne Specials:** Allows grounded special commands to fire mid-jump using existing `DataBank` / ROM streams.
  - **Multi-Projectile Volley & Homing Projectiles:** Spawns `1–2` bonus projectiles at staggered speeds or angles across the 12 `0xffe100` effect slots, or gently homes projectile `Y` toward the opponent's center.
  - ***Koryu* Uppercut Fireball Wall:** Spawns extra projectile streams on rising uppercut/Shoryuken frames.
  - **Instant Charge & Turbo Chain Cancels:** Bypasses directional charge hold minimums and enables normal -> special -> special chain cancels.
  - **Extra Juggle / Air Float:** Extends airborne juggle windows so absurd *Rainbow* juggles connect cleanly.

### 4. Active Tag Partner within the 3-fighter-slot engine

- **Recruiting and sharing (`(user)` + `(user accepted)`):** Partner Contract cards recruit a second tournament fighter (with their own `StylePick` and optional grafted moves) onto `P1`'s bench, displayed as a compact reserve portrait/icon beneath `P1`'s life bar. Both partners share `P1`'s `960` Sub-HP bar and all passive, elemental, and *Rainbow* upgrades (`(user accepted)`).
- **Scheme-specific pre-translation Tag inputs (`(user)`):** Active Tag and Tag Cancel are read from raw pre-translation player inputs with no new keybinds added to `bindings.toml` or `RebindScreen`, suppressing normal/macro attack output on trigger:
  - `ControlScheme::Classic`: **`LP+HK`** (`buttons[0] && buttons[5]`).
  - `ControlScheme::Modern`: **`Special+Macro`** (`raw[3] && raw[5]`).
  - `ControlScheme::Pneumatic`: **`Down, Down + Punch+Kick`** (`Down, Neutral, Down` followed by simultaneous punch-pad and kick-pad press).
- **3-slot execution across encounter types (`(user)` + `(user accepted)`):**
  - **Standard `1v1` bouts & side-scrolling minigames:** Slot `2` (`P2_STRUCT`, object RAM `0x1000..0x15ff`) is free. Triggering Active Tag (`*My call:*` `360`-frame / 6-second cooldown) spawns the partner in slot `2` leaping in from `P1`'s screen edge with an entry strike while the outgoing fighter leaps back off-screen; once the entry strike completes (or immediately on landing/control recovery), `P1_STRUCT` switches to the incoming partner's fighter table and `StylePick`, resetting temporary move/charge state cleanly (`(user accepted)`).
  - **`2v1` Country Finales (`(user)`):** When the player has a recruited Tag Partner, that partner **replaces** the scripted Country Finale AI ally in slot `2` (fighting alongside `P1`, while pressing Tag swaps which of the two `P1_STRUCT` controls in-place; if the player has no recruited partner, the scripted country AI ally fills slot `2`).
  - **`1v2` Ambushes (`(user)`):** Slot `2` holds the second CPU enemy; while slot `2` is occupied, pressing Tag swaps the lead and partner **in-place on `P1_STRUCT`** (with a smoke/flash cue) without a slot-2 leap-in strike.
- **Tag Cancel (`*My call:*`):** Pressing the Tag input during a connected hit's cancel window triggers a **Tag Cancel**, spending half a Super/Drive stock (or starting the tag cooldown slightly longer) to extend combos across both fighters just like a crossover team game.

### 5. Build-specialized opponents and encounters

- **Archetype bosses and specialists (`(user)`):** Every normal or elite bout on the run map assigns the CPU a `TourBuild` crafted around a named archetype (using the Fair CPU and scaling complexity across countries):
  - **Pyromancers (`ALL-FIRE`):** Attacks ignite on contact and fireballs arrive as multi-projectile spreads (`(user)`).
  - **Cryomancers / Hexers (`CHILL / STATUS`):** Normals and projectiles stack Chill to slow `P1`'s walk speed and frame cadence (`(user)`).
  - **Speedsters (`4× TURBO`):** Animations and dashes run at `2×` early and up to `4×` in late countries (`(user)`).
  - **Bootleg Legends (`RAINBOW / KORYU`):** Air specials, *Koryu* fireball uppercuts, and homing projectiles (`(user)`).
  - **`1v2` Ambushes (`(user)` + `*My call:*`):** Elite nodes that place a second themed CPU enemy in slot `2` (`P2_STRUCT`, extending `dramatic_hit_targets` / CPU target selection so `CPU_STRUCT` and `P2_STRUCT` both target `P1_STRUCT`), while Tag switches to in-place `P1_STRUCT` swaps (`(user)`).
  - **Country Finales (`(user)` + `*My call:*`):** Multi-trait country bosses (`2v1` encounters where the player's recruited Tag Partner replaces the scripted AI ally in slot `2` (`(user)`), or dual-archetype empowered bosses) that award a meta-unlock pick on first defeat in the run. Clearing `Thailand`'s final boss plays the active lead fighter's congratulatory ending and 1987 staff roll (`ADR-0016` / `0x44b00`) before presenting the Run Clear summary and between-run unlock picks.

### 6. Retro-homage minigames and extensible prop registry

- **Flagship side-scrolling homage course (`(user)` + `(user accepted)`):** Inspired by *Super Mario Bros.* and classic beat-'em-up bonus courses (`(user)`). Runs directly inside `Fight` over a `BackgroundBank` course (up to `2,048` columns wide) with `CAMERA` (`0xffc680`) uncapped beyond `0x180` to follow `P1` left-to-right toward a finish flag before `TIME` expires (`(user accepted)`).
  - Because the player controls their real World Tour fighter inside `Fight`, all drafted upgrades—elemental attacks, 4× speed, air specials, *Rainbow* fireballs, and tag swaps—interact directly with the course (`(user)` + `(user accepted)`).
  - **Breakable Props (`(user)`):** Crates, brick/tile stacks, overhead blocks, and barrels placed along the course with hurtboxes that shatter under `P1`'s normals, specials, or jump heads for bonus points or `+TIME` pickups.
  - **Dodgeable Props & Hazards (`(user)`):** Rolling barrels, sweeping shuriken volleys, timed flame jets, and spike/pit hazards that knock `P1` back or shave seconds off the course timer, rewarding jumps, dashes, parries, and air specials.
- **ROM props at launch, PixelLab-extensible via `PropSpec` (`(user)` + `(user accepted)`):**
  - Launches with zero new art required by reusing the 1987 ROM's tile stacks (`0x40d30`), wooden crates/boards/tables (`0x41740`, `0x41fe6`), shuriken/fireball templates, and palette-shifted effects (`(user accepted)`).
  - Defines a data-driven `PropSpec` table (sprite dimensions, hit/hurt boxes, breakable HP vs hazard behavior, scroll velocity, and optional art-pack sprite key) so newly generated PixelLab props (custom crates, arcade hazards, enemies/homage props) slot in automatically when PixelLab credits renew without rewriting engine logic (`(user)` + `(user accepted)`).
- **Minigame rewards (`*My call:*`):** Failing to reach the goal before `TIME == 0` in a minigame never burns upgrades; clearing Bronze, Silver, or Gold course thresholds awards `1`, `2`, or `3` 3-card upgrade drafts respectively. Hyper-remixed versions of the 1987 tile-break and 10-second barrel-smash bonus stages (`tasks.rs`) also appear in the minigame rotation alongside the side-scrolling course.

### 7. Persistence (`world_tour.toml` v2)

- **Profile and active-run save (`crates/sf-engine/src/world_tour/native.rs`, `sf-host/src/files.rs`):** Replaces the `v1` static-node fields in `%APPDATA%\sf1-port\world_tour.toml` (`~/.local/share/sf1-port/world_tour.toml` on Linux) with a versioned `v2` schema (`schema = 2`) storing:
  - **Meta-progression:** `unlocked_fighters`, `unlocked_eras`, `unlocked_relics`, `runs_played`, `runs_cleared`, and earned unlock counts.
  - **Optional `active_run` and anti-dodge lock (`(user)`):** Run seed, current map node and cleared nodes, defeat burn count `L`, `in_bout: bool`, pending/banked upgrade cards (`TourBuild` recipe), active lead fighter/style, recruited tag partner, and pending meta-unlock picks. Quitting cleanly from the world map (`BACK` / `QUIT & SAVE`) preserves `active_run` with `in_bout = false` and zero penalty (`(user)`). Starting a non-minigame combat bout sets and flushes `in_bout = true` to `world_tour.toml`, clearing `in_bout = false` only when the match finishes (`(user)`). Choosing `FORFEIT MATCH` from `PauseKind::WorldTour` or force-closing and relaunching while `in_bout == true` treats that bout as a loss: unbanked round picks from that match are discarded, `L` increments, and the player must immediately resolve **Burn `L` Upgrades** (or **Run Over** if banked upgrades `< L`) (`(user)`). Minigame stops never set `in_bout` or burn upgrades on exit (`*My call:*`).
- **Legacy migration (`*My call:*`):** Reading an old `v1` `world_tour.toml` cleanly initializes a fresh `v2` profile (`Ryu` + `Ken`, `SF1`, empty unlock sets) with a log note rather than failing startup.

## What's left to the builder

- Exact card catalog, rarity weights, synergy prerequisites, and numerical tuning (burn ticks, chill percentages, meter boosts, card rank curves).
- Exact layout and coordinates of the front-end screens and in-fight modals (Run Setup, seeded world map with specialist/minigame badges, mid-fight 3-card draft overlay, Burn `L` Upgrades modal, Pause active-build view, Run Clear / Run Over meta-unlock picker), mocked up for approval in the existing `RomFont` framed style.
- Exact `BackgroundBank` course tilemaps, prop placement scripts, and Bronze/Silver/Gold thresholds for the side-scrolling minigame and remixed 1987 bonus stages.
- Exact per-fighter entry-strike move IDs and frame timing for Active Tag handoffs.
- Exact interaction rules when opposing elemental statuses overlap on the same target (e.g., whether Fire thaws Ice or both tick concurrently).
