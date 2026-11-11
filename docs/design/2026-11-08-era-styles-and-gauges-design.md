# Project 4 — Era Styles and Gauges on the Real Fighters

**Date:** 2026-11-08
**Status:** Approved
**ADR:** [`docs/adr/0015-era-styles-and-sub-hp-on-the-1987-engine.md`](../adr/0015-era-styles-and-sub-hp-on-the-1987-engine.md)
**Parent designs:**
- [`docs/design/2026-10-27-definitive-overhaul-design.md`](2026-10-27-definitive-overhaul-design.md) — original 12-char × 8-era overhaul scope
- [`docs/design/2026-11-02-art-pipeline-design.md`](2026-11-02-art-pipeline-design.md) — offline sprite pipeline (`sf1-port-art`) and runtime `SpritePack` injection
- [`docs/design/2026-11-04-era-hud-and-callouts-design.md`](2026-11-04-era-hud-and-callouts-design.md) — era meter HUD, action call-out banners, and 4 kHz announcer
- [`docs/design/2026-11-07-sf1plus-on-the-real-engine-design.md`](2026-11-07-sf1plus-on-the-real-engine-design.md) — Project 3 (`Spec #246`, `SF1+` on the real translated 1987 engine)
**Companion research:** [`docs/research/era-styles-existing-art.md`](../research/era-styles-existing-art.md)
**Visual mockup:** [`docs/design/mockups/2026-11-08-era-select-layout.html`](mockups/2026-11-08-era-select-layout.html)

---

## 1. Intent and the Why

Project 3 (`Spec #246: SF1+ on the real engine`) retires the early `crates/sf-engine/src/combo/` prototype for the `SF1+` ruleset and wires all 12 tournament fighters, their bespoke animations (`sf1-port-art`), and modern fighting-game fundamentals (negative edge, 2-in-1 special cancels, techable throws, true multi-hit combos, and stun decay) directly onto the translated 1987 state machine (`crates/sf-engine/src/fight.rs` and `crates/sf-engine/src/fighter.rs`).

Project 4 brings the remaining six Street Fighter eras—**`SF2`** (`WW`/`CE`/`HF`/`NC`/`ST`), **`SFZ` / `Alpha`** (`SFZ1`/`SFZ2`/`SFZ2A`/`SFZ3` `A-ISM`, `X-ISM`, `V-ISM`), **`SF3`** (`NG`/`2I`/`3S`), **`SF4`** (`SF4`/`SSF4`/`AE2012`/`USF4`), **`SF5`** (`S1`/`AE`/`CE`), and **`SF6`** (`Drive System`)—along with the 12-fighter **What-If Styles** onto that same real 1987 engine.

Four key realizations from our codebase and art audit shaped this project:

1. **The audiovisual presentation layer for all 8 eras already exists.** `crates/sf-video/src/era_hud.rs` (`Spec #216`) renders both players' bottom-corner 2-row gauges (`EraHud { p1, p2 }`, where each `EraGauge` holds a `main: GaugeRow` and optional `second: Option<GaugeRow>` covering `ST` Super, `Alpha` A/X/V-ISM + Guard, `SF3` Super Art I/II/III stocks, `SF4` Super + Revenge, `SF5` EX + V-Gauge, and `SF6` 6-stock Drive top row + 3-stock Super Art bottom row). `crates/sf-video/src/callouts.rs` renders side-margin call-out words (`CalloutKind`: `Super`, `Counter`, `PunishCounter`, `PerfectParry`, `Burnout`, `CrushCounter`, `AlphaCounter`, `CustomCombo`, `FirstAttack`, `Reversal`, `Perfect`, plus Training's `Move` and `Fix`), dims the stage to 25% brightness on `Super`, flashes the gauge on `AlphaCounter`, and desaturates a fighter's body palette to luminance grey on `Burnout`. `crates/sf-audio/src/announcer.rs` and `tools/reference/local/roms/sf1-port-voice` already carry the 34-clip 4 kHz announcer bank. Project 4 uses `era_hud.rs`, `callouts.rs`, and `announcer.rs` **as built in `#216` with zero HUD layout or `CalloutKind` enum changes**—it simply drives them from real fights.
2. **Existing 1987 ROM art + the 67 approved `sf1-port-art` packs cover ~92% of every era's animations today.** By combining our existing animation streams with classic CPS1/CPS2/CPS3 compositor techniques—palette flashes (EX gold, Focus ink-black/white, Drive neon-green/magenta, V-Trigger red/gold), afterimage shadow trails (`V-ISM`, `Speed`, `FADC`, `Drive Rush`), stage dim (`callouts.rs`), hit-stop, and re-timed multi-hit frame streams—all 12 fighters can play across all 8 eras immediately with what we already have on disk.
3. **Only two non-cosmetic sprite sets are missing, and both are scoped as PixelLab-renewal tickets inside Project 4.** Birdie's `Alpha`/`SF5` chain & food props (*Murderer Chain*, *Hanging Chain*, *Break Time* Can/Banana/Donut) and Gen's `Crane` (*Ki*) stance idle & unique normals (`Alpha`/`SF4`) are gameplay-Loadout/stance features rather than cosmetic costumes. They stay in Project 4 as dedicated art tickets gated on the November 27 PixelLab balance renewal, while the engine tickets ship immediately using existing ROM stand-in frames so nothing is blocked.
4. **Purely cosmetic Era Costumes belong in a post-game project.** Nostalgia/Classic costume swaps (`SFZ` Young Birdie/Alpha Retsu/Vest Eagle, `SF5` Nostalgia costumes, `SF6` Outfit 1) are purely visual and are deferred to a separate, lowest-priority post-game project.

---

## 2. Goals, Non-Goals, and Anti-Goals

### Goals
- **All 8 eras playable in real matches (`fight.rs` / `fighter.rs`) under `Definitive Overhaul`:**
  - `SF1` (1987 arcade rules, Ryu/Ken or selected fighter with 1987 single-hit knockback, no cancels, no throws, random startup).
  - `SF1+` (1987 Reimagined from Project 3—fixed 1f startup, 2-in-1 special cancels, throws, 12-fighter roster, no gauges).
  - `SF2` (`WW`, `CE`, `HF`, `NC`, `ST`—Super Combo gauge in `ST`, `Old` vs `New` loadout toggle in `ST`, reversal timing window, throw-softening in `ST`).
  - `SFZ` / `Alpha` (`SFZ1`, `SFZ2`, `SFZ2A`, `SFZ3`—`A-ISM` 3-level Super + Alpha Counter, `X-ISM` single Lv3 Super + `1.12×` damage boost + no air block, `V-ISM` Variable/Custom Combo with shadow trails + `0.50×` scaling, Air Block, Alpha Counter, Guard Power gauge in `SFZ3`).
  - `SF3` (`NG`, `2I`, `3S`—high/low/air Parry, Red Parry, Universal Overhead `MP+MK`, Super Art I / II / III selection with per-SA stock count and bar length, Gill-style Resurrection when applicable).
  - `SF4` (`SF4`, `SSF4`, `AE2012`, `USF4`—4-segment Super Combo gauge, Revenge gauge, Focus Attack L1/L2/L3 + Focus Attack Dash Cancel (`FADC`), Red Focus (`USF4`), Ultra I / Ultra II / Ultra Combo Double (`W` at `75%` damage in `USF4`), Delayed Wakeup (`USF4`)).
  - `SF5` (`S1`, `AE`, `CE`—3-stock EX gauge, V-Skill I / II, V-Trigger I / II (2-bar or 3-bar), V-Reversal, V-Shift + V-Shift Break (`CE`), Critical Art, Crush Counter on designated heavy counter-hits).
  - `SF6` (`Drive System`—6-bar Drive gauge `[60_000]`, Overdrive (`OD`, 2 bars), Drive Parry (`0.5` bar start + drain, refund on parry, Perfect Parry `2f` window with `50%` damage scaling), Drive Rush (raw from Parry `1` bar; cancel from special-cancelable normal `3` bars with `+4f` advantage), Drive Impact (`1` bar, 2 hits armor, wall splat / crumple), Drive Reversal (`2` bars), Burnout state (chip damage on block, `+4f` extra blockstun, wall-splat stun on DI), and Separate 3-stock Super Art gauge (`SA1`, `SA2`, `SA3` / Critical Art at `≤ 25%` HP)).
- **All 12 fighters across canonical and What-If eras:**
  - Every canonical appearance (Ryu, Ken, Sagat, Birdie, Adon, Gen, Eagle) is always selectable in `Definitive Overhaul`.
  - Every What-If era design (Retsu, Geki, Joe, Mike, Lee across `SF2`–`SF6`, plus Birdie/Adon/Gen/Eagle in `SF2`/`SF3`/`SF6` as applicable) is selectable whenever `WHAT-IF STYLES` is `ON` (default `ON` in `Definitive Overhaul`).
- **One unified `960` sub-HP health scale** (`48 × 20` sub-HP per 1987 health point) synchronized every frame to the 1987 work-RAM health byte (`+0x22`), giving `~0.1%` precision for multi-hit Supers, `V-ISM`, Burnout chip, damage scaling, and per-era vitality modifiers (`90%` Gen, `95%` Adon, `105%` Birdie/Sagat).
- **Character Select Option B Spinner Card (`ERA`, `REV`, `LOAD`)** inside the player's portrait panel under `Definitive Overhaul`, with flagship revisions and Loadout I pre-selected so locking a fighter's default style takes a single press.
- **Arcade Ladder CPU Era Matching:** Under `Definitive Overhaul`, CPU opponents match the player's chosen era (falling back to that era's flagship revision if the player chose an early sub-revision the opponent didn't appear in, or to `SF1+` if `WHAT-IF STYLES` is `OFF` and the opponent has no canonical style in that era).
- **Real-fight integration with Era HUD, Call-Out Banners, 4 kHz Announcer, Training `FILL METERS`, and era-filtered Move List / Coach.**
- **Retire the standalone `CombatSession` era prototype crates (`alpha.rs`, `sf3.rs`, `sf4.rs`, `sf5.rs`, `sf6.rs`, `what_if.rs`, `flagship.rs` wrappers)** once their data tables and state machines are migrated into the real engine and covered by real-session integration tests.

### Non-Goals
- **Era Costumes (`art/recipes/` cosmetic costume swaps per era):** Deferred to a separate, lowest-priority post-game project.
- **Unblocking Project 4 on PixelLab's monthly credit renewal:** Only the two dedicated art tickets (Birdie's chain/food props and Gen's Crane stance sprites) wait on the November 27 PixelLab renewal; every gameplay and engine ticket ships immediately using ROM stand-ins.
- **Adding new characters outside the 12 1987 fighters:** Mentioned cameo/mentor inspirations (e.g. Gouken, Rose, Dudley, Zeku, Kimberly, Balrog, Fei Long) exist only as move/style inspirations for the 12 1987 characters' What-If kits.

### Anti-Goals
- **Running `CombatSession` in parallel with `fight.rs` during real matches:** We will not maintain two combat engines or shadow-step `CombatSession` alongside the 1987 work-RAM fighter structs.
- **Showing the Era/Revision/Loadout selector in `Classic Competitive` or `Pure Footsies`:** Those two presets are dedicated `SF1+` tournament rulesets where confirming a portrait locks `SF1+` immediately with zero sub-menus.
- **Changing the 1987 HUD life bar's 48-pixel visual width or breaking `0x00`..`0x30` work-RAM invariants:** `+0x22` must always remain in `0..=48` (`1..=48` while `sub_hp > 0`, `0` on KO).

---

## 3. What Exists Today & Constraints

### 3.1 Real Fighter Engine after Project 3 (`Spec #246`)
- **`crates/sf-engine/src/fight.rs` & `fighter.rs`:** The translated 1987 state machine steps P1 (`work.player1` at `0x8100`) and P2 (`work.player2` at `0x8200`) at 60 Hz using `0x20`-byte animation frame records (`[flags_duration, sprite_id, dx, dy, ...]` plus hitbox/hurtbox IDs and damage).
- **`crates/sf-engine/src/data_bank.rs`:** Project 3 (`SP2` `#248`) relocates all 12 fighters' move tables into an extended data bank (`0x10_0000..=0x1f_ffff`), giving each fighter a full `0x1_0000` (64 KB) slot (`fighter_base(CharacterId)`). Within each slot:
  - `0x0000..0x007c` holds the 31 (`0x1f`) base `u32` move-header pointers (`0x00`..`0x1e`: walks, jumps, crouch, block, hit reactions, throws, standing/crouching/jumping normals, and the 3 classic specials `0x1c`..`0x1e`).
  - `APEX_POINTER` (`0x007c`) and `SPECIALS` (`0x0080`) follow the move pointer table.
  - Stream control bytes support `0xfc` (end of move), `0xfd` (flip), `0xfe` (jump to 4-byte bank address), and `0xff` (call bank routine such as `FIREBALL_ROUTINE`, `TATSU_ROUTINE`, `EAGLE_PROJ_ROUTINE`, `GEKI_SHURIKEN_ROUTINE`, `GEKI_TELEPORT_ROUTINE`).
- **`CoreToggleState` & `MatchConfig` (`crates/sf-engine/src/config.rs`):** Holds overhaul runtime state alongside `WorkRam`, including `preset: RulesetPreset`, `toggles: CoreToggles`, `roster: RosterSettings` (`what_if_styles: bool`), `gauge_mode: GaugeMode`, per-slot `(TournamentFighter, StylePick)` selections, and per-fighter overhaul state.

### 3.2 Standalone Prototype Era Modules (`crates/sf-engine/src/`)
- `era.rs` defines the canonical enums (`EraFamily`, `Revision`, `Loadout`, `Costume`, `GaugeMode`, `StylePick`) and the character × revision availability functions (`families(fighter)`, `revisions(fighter, family)`, `listed_families(fighter, what_if)`, `listed_revisions(fighter, family, what_if)`).
- `alpha.rs`, `sf3.rs`, `sf4.rs`, `sf5.rs`, `sf6.rs`, `what_if.rs`, and `flagship.rs` contain rich, unit-tested data tables (move names, inputs, startup/active/recovery, damage, meter gain/cost, vitality modifiers, and per-revision tweaks) plus standalone state-machine logic (`AlphaMeter`, `Sf3State`, `Sf4State`, `Sf5State`, `Sf6Drive`), currently wired only to the deprecated `CombatSession` prototype.

### 3.3 HUD, Call-Outs, Audio, and Sprite Packs (`Spec #216`)
- **`crates/sf-video/src/era_hud.rs`:** Draws `EraHud { p1: Option<EraGauge>, p2: Option<EraGauge> }` into the bottom corners of the `384×224` RGBA framebuffer (`y = 207..222`) using `GaugeArt` (`Ruled`, `Blocks`, or `Plate`). Each side's `EraGauge` holds `main: GaugeRow` and `second: Option<GaugeRow>`, so asymmetric cross-era matches (`p1` and `p2` in different eras) are already supported natively with zero layout changes.
- **`crates/sf-video/src/callouts.rs`:** Draws `CalloutBoard` / `CalloutFrame` side-column words for the 11 voiced `CalloutKind` moments (`Super { name }`, `Counter`, `PunishCounter`, `PerfectParry`, `Burnout`, `CrushCounter`, `AlphaCounter`, `CustomCombo`, `FirstAttack`, `Reversal`, `Perfect`) plus Training's unvoiced `Move { word }` and `Fix { text }`, dims the stage on `Super`, flashes the gauge on `AlphaCounter`, and desaturates burned-out fighters (`CalloutBoard::set_greyed`).
- **`crates/sf-audio/src/announcer.rs`:** Plays the 34-clip 4 kHz announcer bank (`VoiceClip`) on call-out and round events.
- **`crates/sf-engine/src/sprite_pack.rs`:** Loads `.sf1pack` files at runtime (`0x8000 | pack_idx` sprite IDs) and composites them identically to ROM sprites.

---

## 4. Approaches Considered

| Approach | Summary | Verdict |
|---|---|---|
| **Approach A — Unified Era Subsystem + Extended `data_bank` Move Slots on `fighter.rs` (Chosen)** | Extend each fighter's 64 KB `data_bank.rs` table with era move slots (`0x1f..0x3f`) for EX specials, extra specials, Supers/Ultras, Universal Overhead, Focus Attack, V-Skill/V-Shift/V-Reversal, and Drive Impact/Reversal. Step a single `[EraFighterState; 2]` subsystem inside `fight.rs` / `fighter.rs` that intercepts input priority, parry/armor/guard states, hit/block gauge hooks, and compositor visual effects, using a compact revision-delta table for per-revision frame/damage tweaks. Retire `CombatSession`. | **Chosen.** Single combat engine, full reuse of Project 3's `data_bank.rs` and `SpritePack` infrastructure, and zero duplication of movement/collision/throw code. |
| **Approach B — Code-Driven Procedural Action Interceptors** | Keep `data_bank.rs` at 31 base slots (`0x00..0x1e`) and implement every era move (`EX`, `Super`, `Focus`, `Drive Impact`, What-If moves) as a Rust state machine that overrides `+0x07` sprite IDs and velocity each frame. | **Rejected.** Splits move authoring across two incompatible formats (`0x20`-byte frame streams for base moves vs imperative Rust match arms for era moves) and makes frame-data inspection for the Move List/Coach brittle. |
| **Approach C — Bridge `CombatSession` to Drive `WorkRam` Sprites** | Step `CombatSession` when an era is `SF2`–`SF6` and copy its abstract coordinates and states back into `WorkRam` for rendering. | **Rejected.** Violates Project 3's core architectural principle (one real 1987 engine), creates seam bugs on cross-era matches (`SF1+` vs `SF6`), and breaks stage/scroll/camera parity. |

---

## 5. Decided Shape of the Work

### 5.1 Character × Era & What-If Availability Matrix

Project 4 preserves the exact `EraFamily` and `Revision` availability matrix already implemented in `crates/sf-engine/src/era.rs` (`families`, `revisions`, `listed_families`, `listed_revisions`):

1. **Canonical Styles (`era::families(fighter)` / `era::revisions(fighter, family)`, always available when era selection is active):**
   - **`SF1` (`Revision::Arcade1987`) & `SF1+` (`Revision::Sf1Plus`):** All 12 fighters (`Ryu`, `Ken`, `Retsu`, `Geki`, `Joe`, `Mike`, `Lee`, `Gen`, `Birdie`, `Eagle`, `Adon`, `Sagat`); under `Faithful 1987`, only `Ryu` and `Ken` in `SF1`.
   - **`SF2` (`Ww`, `Ce`, `Hf`, `Super`, `St`):** `Ryu`, `Ken`, `Sagat` across all 5 revisions (`Ww`..`St`); in `St`, `Loadout::Standard` (`New`, with Super Combo gauge) vs `Loadout::Old` (`O.Ryu`/`O.Ken`/`O.Sagat`, pre-`ST` properties and no Super gauge).
   - **`SFZ` / `Alpha` (`Sfz1`, `Sfz2`, `Sfz2A`, `Sfz3`):** `Ryu`, `Ken`, `Sagat`, `Birdie`, `Adon`, `Gen` (`Sfz1`..`Sfz3`), and `Eagle` (`Sfz3`). In `Sfz3`, `Loadout` selects `AIsm`, `XIsm`, or `VIsm`.
   - **`SF3` (`Sf3Ng`, `Sf3TwoI`, `Sf3ThreeS`):** `Ryu`, `Ken` (`Loadout::Art1` / `Art2` / `Art3`).
   - **`SF4` (`Sf4`, `Ssf4`, `Ae2012`, `Usf4`):** `Ryu`, `Ken`, `Sagat`, `Gen` (`Sf4`..`Usf4`), and `Adon` (`Ssf4`..`Usf4`). `Loadout` selects `Ultra1`, `Ultra2` (`Ssf4`+), or `UltraDouble` (`Usf4`).
   - **`SF5` (`Sf5`, `Sf5Ae`, `Sf5Ce`):** `Ryu`, `Ken`, `Birdie` (`Sf5`..`Sf5Ce`), and `Sagat` (`Sf5Ae`, `Sf5Ce`). `Loadout` selects `Skill1Trigger1`, `Skill1Trigger2`, `Skill2Trigger1`, or `Skill2Trigger2` (`Skill2*` in `Sf5Ce`).
   - **`SF6` (`Revision::Sf6`, Drive System):** `Ryu`, `Ken`, `Retsu` (per `era.rs` / `sf6.rs` World Tour appearance; all three Super Arts `SA1`/`SA2`/`SA3` active simultaneously).
2. **What-If Styles (`era::listed_families(fighter, true)` / `era::listed_revisions(fighter, family, true)`, available when `roster.what_if_styles` is `true`; default `true` in `Definitive Overhaul`):**
   - Unlocks the flagship revision(s) of every missing post-1987 era for the 12 tournament fighters using the designs cataloged in `what_if.rs` and `flagship.rs`:
     - **`SF2` (`Hf` & `St`):** `Retsu`, `Geki`, `Joe`, `Mike`, `Lee`, `Gen`, `Birdie`, `Eagle`, `Adon`.
     - **`SFZ` (`Sfz3` `AIsm` / `XIsm` / `VIsm`):** `Retsu`, `Geki`, `Joe`, `Mike`, `Lee`.
     - **`SF3` (`Sf3ThreeS` `Art1` / `Art2` / `Art3`):** `Retsu`, `Geki`, `Joe`, `Mike`, `Lee`, `Gen`, `Birdie`, `Eagle`, `Adon`, `Sagat`.
     - **`SF4` (`Usf4` `Ultra1` / `Ultra2` / `UltraDouble`):** `Retsu`, `Geki`, `Joe`, `Mike`, `Lee`, `Birdie`, `Eagle`.
     - **`SF5` (`Sf5Ce` `Skill1/2 × Trigger1/2`):** `Retsu`, `Geki`, `Joe`, `Mike`, `Lee`, `Gen`, `Eagle`, `Adon`.
     - **`SF6` (`Revision::Sf6`, Drive System):** `Geki`, `Joe`, `Mike`, `Lee`, `Gen`, `Birdie`, `Eagle`, `Adon`, `Sagat`.
   - When `WHAT-IF STYLES` (`roster.what_if_styles`) is toggled `OFF` on the Roster tab, Character Select filters each fighter's `ERA` and `REV` spinners down to `listed_families(fighter, false)` and `listed_revisions(fighter, family, false)`.

### 5.2 Ruleset Presets, Cross-Era Match Rules, and the `960` Sub-HP Scale

#### Ruleset Presets & `MatchConfig` Integration
| Preset | Roster | Era Selection on Character Select |
|---|---|---|
| **`Faithful 1987`** | `Ryu` / `Ken` (`SF1`) | None — `SF1` (`Arcade1987`) locked automatically. |
| **`Classic Competitive`** | All 12 (`SF1+`) | None — confirming a fighter tile locks `SF1+` in 1 press. |
| **`Pure Footsies (1987+)`** | All 12 (`SF1+`, strict specials, `GaugeMode::NoGauges`) | None — confirming a fighter tile locks `SF1+` in 1 press. |
| **`Definitive Overhaul`** | All 12 (`SF1`, `SF1+`, `SF2`–`SF6`) | Opens the **Option B Spinner Card** (`ERA`, `REV`, `LOAD`) on portrait confirm. |
| **`Custom`** (`(my call)`) | Follows `config.roster` | Opens the **Option B Spinner Card** whenever `config.toggles.balanced_roster_and_rebalance` is `true` (so tweaking a gameplay setting from `Definitive Overhaul` into `Custom` never locks the player out of era selection). |

In addition, `EraFighterState` honors the existing `MatchConfig` era fields in `crates/sf-engine/src/config.rs`:
- **`config.gauge_mode`:**
  - `GaugeMode::EraDefault`: Uses each fighter's chosen era gauge and system mechanics.
  - `GaugeMode::NoGauges`: Strips the entire Era Gauge & System-Mechanic package (including meterless mechanics such as `SF3` Parry, `SFZ` Air Block, `SF3` Universal Overhead, and `SF5` V-Skill), so the fighter plays purely their era's moveset and revision frame data on the `SF1+` base system rules.
  - `GaugeMode::Unified`: Gives both fighters a shared 3-stock bar (`GaugeRow::stock(3, ...)`). Era costs map onto those 3 stocks as:
    - **`1` stock:** `EX` / `Overdrive`, `Alpha Counter`, `V-Reversal`, `V-Shift`, `FADC`, `Drive Impact`, `Drive Rush`, `Lv1 Super` / `SA1`.
    - **`2` stocks:** `V-Trigger` activation, `V-ISM` Custom Combo activation, `Lv2 Super` / `SA2`.
    - **`3` stocks:** `SF2 ST` Super, `Lv3 Super`, `Ultra I / II / Double`, `Critical Art`, `SA3`.
- **Safety switches:** `config.disable_raw_drive_rush` disables raw Parry `66` Drive Rush in `SF6`; `config.disable_comeback_scaling` suppresses `SF4` Revenge / `SF5` V-Gauge / `SF6` Critical Art comeback bonuses; `config.sf1plus.untechable_sf2_throws` and `config.sf1plus.allow_corner_throw_loops` govern throw teching and post-throw proximity in `SF2`/`SF1+`.
- **Round-to-round carryover vs. round reset:**
  - **Carry over between rounds:** `SF2 ST` Super, `SFZ` Super/Custom, `SF3` Super Art stocks, `SF4` 4-segment Super, `SF5` 3-stock EX, `SF6` 3-stock Super Art, and `Unified Gauge` stocks.
  - **Reset every round:** `SFZ3` Guard Power (to full), `SF4` Revenge / Ultra (`0`), `SF5` V-Gauge (`0`) and active V-Trigger, `SF6` Drive (`6` full stocks, clearing any Burnout), active timed installs (`V-ISM`, `Genei-Jin`), and Gen's stance (always starts each round in `Mantis` / *Sou*).

#### Cross-Era Interaction Rules (Asymmetric Era Matches)
When P1 and P2 pick different eras (e.g. `SF3 3S` Ryu vs `SF6` Sagat, or `SF1` Ryu vs `SF4` Gen):
- **Attacker-driven offense:**
  - An attacker in `SF2`–`SF6` always has access to their era's offensive rules: fixed 1-frame startup (`fixed_startup`), negative edge, 2-in-1 special cancels (`special_cancels`), era-specific cancels (Super cancel, `FADC`, `V-Trigger` cancel, `Drive Rush` cancel), throws, and multi-hit true combos (`true_combos`) with their era's damage/stun scaling.
  - **Airborne multi-hit & juggle rule:** While `SF1` and `SF1+` attackers remain strictly ground-only per Project 3 (`SP17`), an `SF2`–`SF6` attacker uses a CPS2/SF3/SF4/SF6-style juggle counter (`defender.juggle_count: u8` vs the attacking move's `juggle_limit: u8`) so rising multi-hit specials (*Shoryuken*, *Tiger Uppercut*, *Shinryuken*), `SFZ3` `V-ISM` Custom Combos, `SF4` `FADC` into Ultra, `SF5` Crush Counter launches, and `SF6` Drive Impact wall-splat / Overdrive wall-bounce connect cleanly against an airborne defender until `juggle_count >= juggle_limit`.
  - **Multi-hit projectile clashes:** Each fighter's active projectile tracks `projectile_hits_remaining: u8` (`1` for normal projectiles, `2` for EX/OD projectiles, `3` for *Shakunetsu Hadoken*, `5` for Super beams/barrages). When opposing projectiles collide, both subtract `1` hit; a projectile with `projectile_hits_remaining > 0` continues moving and hits the opponent.
  - An attacker in `SF1+` uses the match's `MatchConfig` switches (all on by default in `Definitive Overhaul`, ground-only combos).
  - An attacker in `SF1` uses authentic 1987 offense: random 1–3f startup jitter, no special cancels, no throws, and single-hit invulnerable reaction knockback (the defender drops out of comboability after 1 hit just like 1987).
- **Defender-driven defense:**
  - Each defender uses their own era's defensive mechanics and gauges regardless of who attacks them:
    - `SFZ`: Air Block (except `X-ISM`), Alpha Counter (`B+F+P/K` during blockstun, `1` Super level or `50%` Custom gauge), Guard Power gauge (`SFZ3`).
    - `SF3`: High/Low/Air Parry (`6f` window on `Forward`/`Down`, `3f` vs projectiles) and Red Parry (`2f..3f` window during blockstun).
    - `SF4`: Focus Attack (`MP+MK`, 1 hit armor) + Dash Cancel, Backdash invulnerability, Delayed Wakeup (`USF4`, `11f` extra knockdown delay on 2 buttons).
    - `SF5`: V-Reversal (`Forward + 3P/3K` in blockstun, `1` V-bar) and V-Shift (`HP + MK`, `1` V-bar, slow-motion blowback or V-Shift Break follow-up).
    - `SF6`: Drive Parry (`MP + MK`, Perfect Parry `2f` window), Drive Impact (`HP + HK`, 2 hits armor), Drive Reversal (`Forward + HP + HK` in blockstun or Drive Parry, `2` Drive bars), and Burnout penalties (`+4f` extra blockstun, chip damage on special/Super block, wall-splat stun on blocked Drive Impact).
  - **Throw teching:** Defender-driven by default—any defender in `SF1+` or `SF2`–`SF6` can tech throws (`LP + LK` within `7f` of throw capture), unless `config.sf1plus.untechable_sf2_throws` is turned `ON` in Settings (in which case throws initiated by an `SF2` attacker cannot be teched, matching 1991 `SF2`). An `SF1` (`1987`) defender never techs throws.

#### Unified `960` Sub-HP Scale (`48 × 20`)
Every era in Street Fighter history uses a slightly different internal life bar (1987 uses `48`, `SF2`/`Alpha` use `144`, `SF3` uses `160`, `SF4`/`SF5` use `900`–`1100`, `SF6` uses `10,000`). To reconcile all 8 eras cleanly with the 1987 work-RAM health byte (`+0x22`, `0x00..=0x30` / `0..=48`):
- **Internal tracking:** `EraFighterState` tracks `sub_hp: u16` and `max_sub_hp: u16` with a standard baseline of **`960` sub-HP** (`48 × 20`).
- **Exact `× 20` mapping for `SF1` and `SF1+`:** Every `1` point of 1987 / Project 3 damage equals exactly `20` sub-HP (`14` HP Fierce = `280` sub-HP; `20` HP Hadoken = `400` sub-HP).
- **Per-era vitality modifiers:** Applied to `max_sub_hp` at round start for eras with character-specific stamina (`SF3`, `SF4`, `SF5`, `SF6`):
  - Standard fighters (`Ryu`, `Ken`, `Retsu`, `Joe`, `Lee`, `Eagle`): `100%` = **`960`** `max_sub_hp`.
  - High-vitality heavyweights (`Birdie`, `Sagat`, `Mike`): `105%` = **`1008`** `max_sub_hp`.
  - Low-vitality technical/speed fighters (`Adon`, `Geki`): `95%` = **`912`** `max_sub_hp`.
  - Glass-cannon masters (`Gen`): `90%` = **`864`** `max_sub_hp`.
- **Every-frame Work-RAM sync invariant:**
  - While `sub_hp > 0`, `+0x22` is updated every frame to:
    `work_hp = ((u32::from(sub_hp) * 48).div_ceil(u32::from(max_sub_hp))) as u8` (guaranteed in `1..=48` so a fighter at `1..19` sub-HP still shows 1 pixel of life and does not trigger premature 1987 KO).
  - When `sub_hp == 0`, `+0x22` is set to `0`, triggering the standard 1987 KO sequence, slow-motion knockdown, and round-end announcer flow.

### 5.3 Character Select (`Option B` Spinner Card) & Arcade CPU Era Matching

See the interactive mockup in [`docs/design/mockups/2026-11-08-era-select-layout.html`](mockups/2026-11-08-era-select-layout.html).

#### Option B — 3-Row Left/Right Spinner Card Inside the Portrait Box
1. **When it appears:** Whenever `config.era_select_enabled()` is `true` (`RulesetPreset::DefinitiveOverhaul`, or `RulesetPreset::Custom` with `balanced_roster_and_rebalance` enabled). Under `Classic Competitive` and `Pure Footsies (1987+)`, confirming a portrait on the 12-tile grid immediately locks `StylePick::default()` (`EraFamily::Sf1Plus`, `Revision::Sf1Plus`, `Loadout::Standard`) with no overlay.
2. **Layout inside the player's portrait panel (`102×102` in 2P/Training at `x=48..150` / `x=234..336`, `y=28..130`; `104×104` in 1P Arcade at `x=56..160`, `y=32..136`):**
   - Confirming a fighter tile dims that player's portrait box (`75%` black translucent tint over the bust portrait) and opens a 3-row spinner card:
     - **Row 0 (`ERA`):** Cycles `listed_families(fighter, config.roster.what_if_styles)` (`◄ SF1+ / SF2 / SFZ / SF3 / SF4 / SF5 / SF6 / SF1 ►`), remembering that player's last chosen era across matches and defaulting to `SF1+` on boot `(my call)`.
     - **Row 1 (`REV`):** Cycles `listed_revisions(fighter, family, config.roster.what_if_styles)`, **pre-filled with that era's flagship revision** whenever `ERA` changes:
       - `SF1` → `1987` (1 choice, row dimmed)
       - `SF1+` → `87+` (1 choice, row dimmed)
       - `SF2` → defaults to `ST` (cycles `WW`, `CE`, `HF`, `Super`, `ST` as available for that fighter)
       - `SFZ` → defaults to `SFZ3` (cycles `SFZ1`, `SFZ2`, `SFZ2A`, `SFZ3` as available)
       - `SF3` → defaults to `3S` (cycles `NG`, `2I`, `3S` as available)
       - `SF4` → defaults to `USF4` (cycles `SF4`, `SSF4`, `AE2012`, `USF4` as available)
       - `SF5` → defaults to `CE` (cycles `S1`, `AE`, `CE` as available)
       - `SF6` → `DRIVE` (1 choice, row dimmed)
     - **Row 2 (`LOAD`):** Cycles loadouts for the active `(ERA, REV)`, **pre-filled with Loadout I / default**:
       - `SF1`, `SF1+`, `SF2 WW..Super`, `SFZ1..SFZ2A`, `SF6` → 1 choice (row dimmed, e.g. `SA I-III ALL` for `SF6`)
       - `SF2 ST` → `Standard` (`NEW`, default) / `Old` (`OLD`)
       - `SFZ3` → `AIsm` (default) / `XIsm` / `VIsm`
       - `SF3` → `Art1` (default) / `Art2` / `Art3`
       - `SF4` → `Ultra1` (default) / `Ultra2` (`SSF4`+) / `UltraDouble` (`USF4`)
       - `SF5` → `Skill1Trigger1` (default) / `Skill1Trigger2` / `Skill2Trigger1` / `Skill2Trigger2` (`Skill2*` in `CE`)
   - **1-Line Summary Footer:** Displays the active loadout's move name or mechanic summary (e.g. `SHINKU HADOKEN (1 STOCK)`, `O.SAGAT (NO SUPER GAUGE)`, `6-BAR DRIVE · SA1/SA2/SA3`).
   - **Center VS Badge:** Updates live beneath the player's fighter name (`SF3 3S · SA I`).
3. **Controls:**
   - `Up` / `Down`: Moves highlight between active (non-dimmed) rows (`ERA`, `REV`, `LOAD`), skipping rows that have only 1 valid option `(my call)`.
   - `Left` / `Right`: Cycles the highlighted row's value immediately.
   - `Confirm` (`P1/P2 Light Punch` / `Confirm`) or `Start`: **Locks all three rows immediately in one press**, regardless of which row is currently highlighted.
   - `Back` (`Light Kick` / `Cancel`): Closes the spinner card and returns the cursor to the 12-fighter grid.
4. **Palettes & Costumes on the Style Card:** The Style Card stays strictly 3 rows (`ERA`, `REV`, `LOAD`); same-character matches automatically apply Project 3's P2 **Mirror palette** (`SP18`), and manual costume/palette pickers are deferred to the post-game cosmetic Era Costumes project.

#### Arcade Ladder CPU Era Matching & Era CPU AI
When starting an Arcade 1P run with era selection active:
1. Every CPU opponent on the 10-stage ladder matches the player's chosen **`EraFamily`**.
2. **Sub-revision resolution:**
   - If the CPU fighter is available in the exact `Revision` the player picked (via `listed_revisions(cpu, player_family, config.roster.what_if_styles)`), the CPU uses that exact `Revision`.
   - If the player picked an earlier sub-revision that the CPU fighter did not appear in (for example, the player picked `SF2 WW` Ryu and faces Retsu or Eagle), the CPU uses that era family's **flagship revision** (`Revision::St`, `Revision::Sfz3`, `Revision::Sf3ThreeS`, `Revision::Usf4`, `Revision::Sf5Ce`, `Revision::Sf6`).
   - If `WHAT-IF STYLES` is `OFF` and `listed_revisions(cpu, player_family, false)` is empty, the CPU falls back to **`Revision::Sf1Plus`**.
3. **CPU Loadout selection (`(my call)`):** CPU opponents default to Loadout I (`Standard`, `AIsm`, `Art1`, `Ultra1`, `Skill1Trigger1`) on stages 1–7, and deterministically vary across stages 8–10 (Adon/Sagat/rival) seeded from the stage index so boss fights showcase signature loadouts.
4. **Era-aware CPU AI (`ADR-0014` pad-feeding controller):**
   - Extends the overhaul's pad-feeding CPU controller with difficulty-scaled (`config.difficulty`) era tactics:
     - Hit-confirms or wakeup-reversals into available `Super` / `Ultra` / `Critical Art` at medium+ difficulty, and weaves `EX`/`OD` specials when holding `≥ 2` stocks.
     - Uses era defensive/system mechanics (`SFZ` Air Block / Alpha Counter, `SF3` Parry, `SF4` Focus Attack, `SF5` V-Skill / V-Trigger activation, `SF6` Drive Parry / Drive Impact) at difficulty-scaled probabilities, while keeping a `1.5`-bar Drive reserve at lower difficulties in `SF6` to avoid self-inflicted Burnout.

### 5.4 The Six Era Gauge & System Engines on `fighter.rs` + `data_bank.rs`

#### Extended `data_bank.rs` Move Slot Layout (`(my call)`)
Each fighter's 64 KB bank slot (`fighter_base(ch)`) expands its move pointer table from `0x1f` (31) entries to `0x40` (64) `u32` entries (`0x0000..0x0100`), shifting `APEX_POINTER` to `0x0100` and `SPECIALS` to `0x0104`:
- **`0x00..=0x1e` (Slots 0–30):** Existing 1987 + Project 3 base moves (idle, walks, jumps, crouch, block, hit reactions, throws, 18 normals, and 3 base specials `0x1c`..`0x1e`).
- **`0x1f..=0x23` (Slots 31–35):** Additional era specials & command normals (4th/5th specials such as *Shakunetsu Hadoken*, *Joudan Sokutou Geri*, *Tiger Knee*, *Jaguar Tooth*, *Bull Horn*, *Bull Revenger*, *Gekiro*, *Oga*, *Manchester Black*, plus stance toggle / command overheads).
- **`0x24..=0x28` (Slots 36–40):** **EX / Overdrive (`OD`)** versions of slots `0x1c..=0x20` (faster startup, multi-hit/invulnerability/wall-bounce properties, gold/neon palette flash).
- **`0x29..=0x2d` (Slots 41–45):** **Super Combos / Super Arts I–III / Critical Arts / Ultra I–II** (invulnerable startup freeze, stage dim via `CalloutKind::Super { name }`, multi-hit stream or high-damage cinematic sequence).
- **`0x2e..=0x33` (Slots 46–51):** **Universal System Mechanics:**
  - `0x2e`: Parry / Perfect Parry / Red Parry pose (`6f` flash).
  - `0x2f`: Universal Overhead (`SF3` `MP+MK`) / Focus Attack charge & strike (`SF4` `MP+MK`) / Drive Impact armor & strike (`SF6` `HP+HK`).
  - `0x30`: Alpha Counter (`SFZ`) / V-Reversal (`SF5`) / Drive Reversal (`SF6`) guard-cancel blowback.
  - `0x31`: V-Skill I / II (`SF5` `MP+MK`).
  - `0x32`: V-Trigger activation pose (`SF5` `HP+HK`) / Custom Combo activation freeze (`SFZ3` `V-ISM`).
  - `0x33`: V-Shift evade & V-Shift Break (`SF5 CE` `HP+MK`).
- **`0x34..=0x3f` (Slots 52–63):** Character-specific stance moves and follow-ups (Gen's *Crane* (`Ki`) stance normals/specials, Birdie's *Break Time* food items & *Hanging Chain* angles, Geki's follow-up teleports, Lee's *Zanku* feints).

#### Unified `[EraFighterState; 2]` in `CoreToggleState`
During `fight.rs`'s per-frame step (`step_fighting`), each player's `EraFighterState` hooks into four points of `fighter.rs`:
1. **Pre-input & System Mechanic Priority (`step_era_pre_input`):**
   - Evaluates era-specific defensive/system inputs before normal/special decoding:
     - `SF3` Parry (`Forward` / `Down` tapped within `6f` of incoming contact; Red Parry within `3f` of blockstun recovery).
     - `SF4` Focus Attack hold/release (`MP+MK`, L1 `1..19f`, L2 `20..49f` crumple, L3 `50f` unblockable crumple; `66`/`44` dash cancel for `0` meter on L2+ hit or `2` Super segments on L1/block).
     - `SF5` V-Trigger activation (`HP+HK` with full V-Gauge), V-Shift (`HP+MK`, `1` V-bar), V-Reversal (`6+PPP/KKK` in blockstun, `1` V-bar), V-Skill (`MP+MK`).
     - `SF6` Drive Parry (`MP+MK`, Perfect Parry on `1..2f`), Drive Rush (`66` from Parry unless `disable_raw_drive_rush`, or `66` cancel from special-cancelable normal), Drive Impact (`HP+HK`, 2 hits armor), Drive Reversal (`6+HP+HK` in blockstun/Parry).
     - `SFZ` Alpha Counter (`412+P/K` in blockstun) and `V-ISM` Custom Combo activation (`2P+2K` at `≥ 50%` gauge).
2. **Special, EX/OD, and Super/Ultra Decoder (`step_era_specials`):**
   - Evaluates high-priority motion inputs (double-QCF / double-QCB / Raging Demon / Charge-back-forward-back-forward for Supers, Ultra I/II/W, and Critical Arts) before single-motion specials.
   - Evaluates two-button strength presses (`LP+MP`, `MP+HP`, `LK+MK`, `MK+HK`) to route to EX (`SF3` `2I`/`3S` where applicable, `SF4`, `SF5`) or Overdrive (`SF6`) move slots (`0x24..=0x28`) when gauge is available.
3. **Hit, Block, Armor, Juggle, and Scaling Resolution (`step_era_hit_resolution`):**
   - Applies era damage scaling (combo hit count scaling `100% → 90% → 80% ...` down to era floor; `50%` on `SF6` Perfect Parry; `50%` on `V-ISM` Custom Combo; `75%` on `USF4` `UltraDouble`; `1.12×` damage / `1.15×` incoming damage on `SFZ3` `X-ISM`).
   - Resolves airborne juggle hits when `attacker_era >= EraFamily::Sf2` using `defender.juggle_count < move.juggle_limit`.
   - Resolves armor absorption (`SF4` Focus 1 hit, `SF5` V-Trigger armored specials, `SF6` Drive Impact 2 hits) with grey/recoverable sub-HP or armor-break interaction.
   - Updates `sub_hp`, syncs `+0x22`, feeds meter gains (`ST` Super, `Alpha` Super/Guard, `SF3` Super Art, `SF4` Super + Revenge on damage taken, `SF5` EX + V-Gauge on damage/White Life/V-Skill, `SF6` Drive + Super Art), and emits `CalloutEvent` + `VoiceClip` triggers.
4. **Revision Delta Table (`RevisionDelta`):**
   - Instead of duplicating move streams for sub-revisions (`Ww` vs `Ce` vs `Hf` vs `Super` vs `St`, or `Sf4` vs `Ssf4` vs `Ae2012` vs `Usf4`), a compact `RevisionDelta` lookup adjusts startup/recovery frames (`±1..3f`), projectile cooldown, damage/stun multipliers, invulnerability windows, and system toggles (e.g. `Ae2012` vs `Usf4` Red Focus & Delayed Wakeup) on top of the era's base move table.

### 5.5 Visual Recipes from Existing Art & the Two PixelLab Tickets

Full per-fighter and per-mechanic specifications are documented in [`docs/research/era-styles-existing-art.md`](../research/era-styles-existing-art.md).

#### Compositor Primitives (`crates/sf-video/` + `crates/sf-engine/`)
To make every era mechanic and What-If move read unmistakably at 60 Hz using our existing sprites, we add three lightweight per-fighter visual modifiers to the sprite renderer (`crates/sf-video/`):
1. **Per-Fighter Palette Tint / Flash Override:**
   - **EX / Overdrive:** Alternating 2-frame gold (`#FFD700` / `#FFF8DC`) body palette flash during active EX/OD moves.
   - **Focus / Red Focus (`SF4`):** Ink-black silhouette flash (`1f`) into white charge glow (crimson glow for Red Focus).
   - **V-Trigger (`SF5`):** Persistent warm red/gold rim tint while the V-Trigger timer is active; cyan flash on V-Shift.
   - **Drive System (`SF6`):** Character-colored neon paint splash (`P1` gold/orange, `P2` cyan/magenta) on Drive Impact/Drive Rush; green flash on Drive Parry; existing luminance-grey desaturation (`CalloutBoard::set_greyed`) on Burnout.
   - **Elemental Projectiles:** Palette swaps on the existing fireball/slash tiles (`0x66`/`0x67`, Eagle's slash, Geki's shuriken) for *Shakunetsu Hadoken* (crimson/orange), *Denjin Hadoken* (electric cyan/yellow), and *Ren-Hadoken* / *Shinku Hadoken* (gold/blue beam).
2. **Afterimage Shadow Trail Ring Buffer (`[ShadowFrame; 4]` per fighter):**
   - Records `(x, y, sprite_id, flip)` every `3` frames and renders up to 3 trailing silhouettes at 50% opacity using a monochrome tint:
     - Blue shadows for `SFZ3` `V-ISM` Custom Combo and `SF3` *Genei-Jin* / Speed-type Supers.
     - Green shadows for `SF6` Drive Rush.
     - White/gold shadows for `SF4` `FADC` and `SF5` V-Shift.
3. **Super Freeze & Stage Dim:**
   - Triggering any Super Combo / Super Art / Ultra / Critical Art pauses both fighters for `28f` (`~0.47s`), fires `CalloutKind::Super { name }` (which already dims the stage background in `callouts.rs`), and flashes a 4-frame expanding ring around the attacker's chest before releasing the multi-hit stream.

#### The Two PixelLab-Gated Tickets in Project 4 (Waiting on Nov 27 Balance Renewal)
Both tickets live inside Project 4, explicitly tagged as blocked on the November 27 PixelLab credit renewal, while the engine tickets use existing ROM stand-in frames immediately:
1. **Ticket A — Birdie's `Alpha` / `SF5` Chain & Food Props (`art/recipes/birdie-era-props.json`, ~18 frames):**
   - *Bull Revenger* / *Murderer Chain* horizontal & diagonal chain-reach frames (`6` frames).
   - *Hanging Chain* (`SF5` V-Skill / anti-air chain) diagonal & upward chain frames (`4` frames).
   - *Break Time* (`SF5` `V-Skill I`) Can, Banana peel, and Donut consumption + ground-hazard projectile frames (`8` frames).
   - **Immediate stand-in until Nov 27:** Uses Birdie's existing `birdie-throws` grab lunge + recolored horizontal chain/slash projectile tiles.
2. **Ticket B — Gen's `Crane` (*Ki*) Stance Idle & Unique Normals (`art/recipes/gen-crane-stance.json`, ~16 frames):**
   - Distinct one-legged `Crane` (*Ki*) idle loop (`4` frames), Crane crouch/walk (`4` frames), and signature Crane ground pokes + `Jasen` rolling attack (`8` frames).
   - **Immediate stand-in until Nov 27:** Uses Gen's airborne/kick frames with a distinct silver/cyan robe palette swap so `Mantis` (*Sou*) and `Crane` (*Ki*) stances are immediately distinguishable in play and tests.

### 5.6 Era HUD, Call-Outs, Announcer, Move List / Coach, and Training `FILL METERS`

1. **Real-Fight → `EraHud` Bridge (`crates/sf-host/` & `crates/sf-engine/`):**
   - Project `EraFighterState::era_gauge()` into `EraHud { p1, p2 }` every frame so `crates/sf-video/src/era_hud.rs` renders each player's exact `GaugeRow::Stock` / `GaugeRow::Level`, `GaugeState` (`Normal`, `Full`, `Draining`, `Greyed`, `Flashing`), and secondary bars in the bottom corners with zero changes to `era_hud.rs` geometry.
2. **Real-Fight → `CalloutBoard` & `Announcer` Bridge:**
   - Emit `CalloutEvent` (`CalloutKind`: `Super { name }`, `Counter`, `PunishCounter`, `PerfectParry`, `Burnout`, `CrushCounter`, `AlphaCounter`, `CustomCombo`, `FirstAttack`, `Reversal`, `Perfect`) and `CalloutBoard::set_greyed(side, burned_out)` directly from `EraFighterState` transitions, driving `crates/sf-audio/src/announcer.rs` automatically.
3. **Training Mode `FILL METERS` (`config.fill_meters`):**
   - When `FILL METERS` is enabled in Training Mode, `EraFighterState` pins all active offensive/system gauges (`Super`, `Revenge`, `V-Gauge`, `Drive`, `Guard`) to max at the start of every frame when the fighter returns to neutral/recovery, allowing infinite back-to-back Super/Ultra/Drive/V-Trigger practice.
4. **Pause Move List & Interactive Coach (`MOVES`, `SHOW`, `TRY`):**
   - The Pause Move List queries the fighter's active `(TournamentFighter, StylePick)` and lists:
     - That era's special moves (including EX/OD variants when applicable).
     - The selected Super Combo / Super Art / Ultra / Critical Art.
     - Active era system mechanics (e.g. `Parry`, `Focus Attack`, `V-Skill / V-Trigger`, `Drive Impact / Drive Parry`).
   - `SHOW` (CPU demonstration) and `TRY` (3-rep interactive trial) temporarily grant full gauge for metered moves so the player can practice any Super, Ultra, EX, or Drive move directly from the Pause menu.

---

## 6. What's Left to the Builder

- **Exact slot indices within `0x1f..=0x3f` per fighter:** The builder may organize each character's extra `data_bank.rs` slots in whatever order is cleanest, provided the Move List metadata and `step_era_specials` decoder read from one shared per-fighter era move table.
- **Order of retirement for `CombatSession` modules:** Keep the pure data tables (`RevisionSpec`, `WhatIfKit`, `FlagshipKit`, move frame/damage constants) or move them into `crates/sf-engine/src/era/`, and delete the duplicate `CombatSession` stepping code only after real-engine integration tests (`crates/sf-engine/tests/`) cover all 8 eras and all 12 fighters.
- **Invariants to preserve:**
  - `+0x22` work-RAM health must always be in `1..=48` while `sub_hp > 0` and `0` when `sub_hp == 0`.
  - `era_hud.rs`, `callouts.rs`, and `announcer.rs` (`Spec #216`) are used as built without changing their screen regions or `CalloutKind` variants.
  - `Faithful 1987`, `Classic Competitive`, and `Pure Footsies (1987+)` must remain frame-for-frame identical to Project 3 behavior and must never show the Character Select Era/Revision/Loadout spinner card.

---

## 7. Staged `CONTEXT.md` Glossary Additions

*(Staged here so we do not touch `CONTEXT.md` while Project 3 worktrees are actively merging; add to `CONTEXT.md` when Project 4 begins.)*

```markdown
**Style card**:
The three-row spinner (`ERA`, `REV`, `LOAD`) that dims a fighter's portrait box on Character Select under Definitive Overhaul (or Custom with Balanced roster on), pre-filled with that era's flagship revision and first loadout so one press of Confirm or Start locks all three.
_Avoid_: groove menu, edition popup, style drawer

**Sub-HP**:
The overhaul's 960-point internal health scale (twenty sub-points for each of the 1987 game's 48 health points, scaled by a fighter's era vitality), rounded up into the 1987 health byte every frame so the 48-pixel life bar and 1987 knockout rules work unchanged.
_Avoid_: float HP, 1000 HP, secondary life bar

**Revision delta**:
The small table of startup, recovery, damage, stun and system-switch adjustments that turns an era's base move table into one of its sub-revisions (`WW` vs `CE` vs `HF` vs `Super` vs `ST`, or `SF4` vs `SSF4` vs `AE2012` vs `USF4`) without duplicating move streams.
_Avoid_: balance patch, cloned move table

**Afterimage trail**:
The three monochrome trailing silhouettes of a fighter's recent poses drawn behind him during a Custom Combo, a Speed Super Art, a Focus Attack Dash Cancel or a Drive Rush.
_Avoid_: motion blur, ghost sprite, clone
```
