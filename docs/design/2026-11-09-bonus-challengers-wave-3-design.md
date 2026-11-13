# Bonus Challengers — Wave 3 (*Ultra Street Fighter: Final Round*) on the Real Engine

- **Date:** 2026-11-09
- **Status:** Approved
- **Project:** Project 10 (`Bonus Challengers — Wave 3: Ultra Street Fighter: Final Round on the real engine`)
- **Context:** [`CONTEXT.md`](../../CONTEXT.md), [`2026-10-27-definitive-overhaul-design.md`](2026-10-27-definitive-overhaul-design.md), [`2026-11-02-art-pipeline-design.md`](2026-11-02-art-pipeline-design.md), [`2026-11-03-bonus-roster-by-lore-design.md`](2026-11-03-bonus-roster-by-lore-design.md), [`2026-11-03-stages-and-music-design.md`](2026-11-03-stages-and-music-design.md), [`2026-11-08-era-styles-and-gauges-design.md`](2026-11-08-era-styles-and-gauges-design.md), [`2026-11-08-endings-dramatic-battle-and-world-tour-design.md`](2026-11-08-endings-dramatic-battle-and-world-tour-design.md), [`2026-11-08-bonus-challengers-wave-1-design.md`](2026-11-08-bonus-challengers-wave-1-design.md), [`2026-11-09-training-drills-replay-takeover-and-control-schemes-design.md`](2026-11-09-training-drills-replay-takeover-and-control-schemes-design.md), [`2026-11-09-bonus-challengers-wave-2-design.md`](2026-11-09-bonus-challengers-wave-2-design.md), [`ADR-0004`](../adr/0004-reach-fights-by-driving-the-faithful-flow.md), [`ADR-0006`](../adr/0006-rom-only-art-sources.md), [`ADR-0008`](../adr/0008-bonus-roster-chosen-by-lore.md), [`ADR-0009`](../adr/0009-stages-and-themes-in-extended-banks.md), [`ADR-0014`](../adr/0014-bosses-become-1987-player-fighters.md), [`ADR-0015`](../adr/0015-era-styles-and-sub-hp-on-the-1987-engine.md), [`ADR-0016`](../adr/0016-dramatic-battle-and-composite-endings-on-the-1987-engine.md), [`ADR-0017`](../adr/0017-forty-fighter-roster-and-satsui-awakening-on-the-1987-engine.md), [`ADR-0018`](../adr/0018-rollback-netplay-and-virtual-net-bus-on-the-1987-engine.md), [`ADR-0019`](../adr/0019-utility-ai-control-schemes-and-replay-takeover-on-the-1987-engine.md), [`ADR-0020`](../adr/0020-wave-2-bloodlines-airborne-specials-and-shared-stage-owners.md)

---

The stage-command columns below identify the primary owner's command. The [character-theme design](2026-11-10-character-themes-design.md) and [ADR-0023](../adr/0023-character-themes-and-separate-native-score-storage.md) assign individual themes and determine which one plays when fighters share a stage.

## 1. Intent

Once the planned **Wave 1 (*Super Street Fighter: Rising*, `Spec #292`)** and **Wave 2 (*Hyper Street Fighter: Bloodlines*, `Spec #319`)** work is implemented, it will expand the 12-fighter 1987 tournament roster to `24` (`slots 0..24`, `8×3` Select Grid).

This design specifies **Wave 3 (*Ultra Street Fighter: Final Round*)**—the final **16 challengers** (`slots 24..40`, `0x28_0000..=0x37_ffff`) who existed as active fighters in the shared *Street Fighter* / *Final Fight* (*Street Fighter '89*) / *Alpha* world of 1987 (`ADR-0008`, [`2026-11-03-bonus-roster-by-lore-design.md` §4](2026-11-03-bonus-roster-by-lore-design.md)), completing the **40-fighter roster** (`8×5` Select Grid):
1. **Guy** (`slot 24`, `0x28_0000`) — 39th successor of Bushinryu Ninjutsu and defender of Metro City (*Final Fight*, *Alpha 1–3*, *SF4*).
2. **Cody** (`slot 25`, `0x29_0000`) — 1987 Metro City street hero in white T-shirt and jeans, before his *Alpha 3* prison stripes and *SF5* mayoral term (*Final Fight*, *Alpha 3*, *SF4*, *SF5*).
3. **Rolento** (`slot 26`, `0x2a_0000`) — Ex-Red Beret & Mad Gear Industrial Zone commander (*Final Fight*, *Alpha 2–3*, *USF4*).
4. **Sodom** (`slot 27`, `0x2b_0000`) — Kabuki-obsessed Mad Gear Subway ring boss wielding dual jitte/katana (*Final Fight*, *Alpha 1–3*).
5. **Rose** (`slot 28`, `0x2c_0000`) — Genoa fortune-teller and master of Soul Power opposing M. Bison (*Alpha 1–3*, *SF4*, *SF5*).
6. **Charlie Nash** (`slot 29`, `BonusChallenger::Nash`, `0x2d_0000`) — First Lieutenant in the US Air Force investigating Shadaloo (*Alpha 1–3*, *SF5*).
7. **Mike Haggar** (`slot 30`, `BonusChallenger::Haggar`, `0x2e_0000`) — Former *Slam Masters* pro-wrestling champion and Mayor of Metro City (*Final Fight*).
8. **Zangief** (`slot 31`, `0x2f_0000`) — The "Red Cyclone," Soviet national hero and 360° grappling icon (*SF2*, *Alpha 2–3*, *SF4*, *SF5*, *SF6*).
9. **Vega** (`slot 32`, `0x30_0000`) — Spanish cage-fighting ninja and Shadaloo Heavenly King (*SF2*, *Alpha 3*, *SF4*, *SF5*).
10. **Balrog** (`slot 33`, `0x31_0000`) — Disgraced Las Vegas heavyweight boxer and Shadaloo Heavenly King (*SF2*, *Alpha 3*, *SF4*, *SF5*).
11. **Zeku** (`slot 34`, `0x32_0000`) — 38th Grandmaster of Bushinryu who trained Guy and founded the Striders (*Alpha 2*, *SF5*).
12. **Belger** (`slot 35`, `0x33_0000`) — Crime Lord of the Mad Gear Gang and final boss of *Final Fight* (*Final Fight*).
13. **Poison** (`slot 36`, `0x34_0000`) — Mad Gear acrobat and Hugo's future wrestling promoter (*Final Fight*, *USF4*, *SF5*).
14. **Hugo** (`slot 37`, `0x35_0000`) — Giant German powerhouse appearing in his 1987 *Final Fight* **Andore** leopard-singlet identity, with his *SF3*/*USF4* pink singlet in later Era Styles (*Final Fight*, *SF3*, *USF4*).
15. **Abigail** (`slot 38`, `0x36_0000`) — Towering 244 cm commander of Mad Gear's North Bay Area scrapyard (*Final Fight*, *SF5*).
16. **Damnd** (`slot 39`, `0x37_0000`) — Thrasher Damnd, Mad Gear's Slum boss whose kidnapping of Jessica Haggar ignites the Metro City war (*Final Fight*, *SF6 World Tour*).

Following the **Code-First + Deterministic ROM Stand-In Catalogue** architecture established in Waves 1 & 2, all 16 fighters' `DataBank` movesets, 360° command-throw recognition (`MOTION_360`), screen-edge wall dives (`SPECIAL_WALL_DIVE`), Zeku's *Tenpo Kari* Old/Young form toggle (`0x32_0000` / `0x32_8000`), Damnd's *Whistle Call* helper projectile (`ROUTINE_PROJECTILE_AT_EDGE`), 8 Wave 3 stages (`0x49`–`0x50`), `40 × 40 = 1,600` composite VS screens, 16 endings, Secret Arcade Intrusions, 8 Dramatic Battle presets (`Presets 15–22`), and 8 World Tour nodes (`Nodes #26–#33`) are implemented and verified in code first (`UW1`–`UW8`), while flipping `RELEASED_WAVES` from `2` to `3` (completing the **`8×5 = 40`** Select Screen) is gated on the PixelLab art tickets (`UW9`–`UW13`).

---

## 2. Goals, Non-Goals, Anti-Goals

### Goals
1. **Wave 3 Archetype Mechanics on the 1987 Engine (`crates/sf-engine/src/fighter.rs`, `crates/sf-engine/src/data_bank.rs`, `crates/sf-engine/src/config.rs`):**
   - **360° Command-Throw Recognition (`MOTION_360` & `ROUTINE_COMMAND_THROW`):** Recognize 3-of-4 cardinal directions (`Forward`, `Down`, `Back`, `Up` in clockwise or counter-clockwise order within `16` frames + `Punch`/`Kick`, plus `Back + SP` under `ControlScheme::Modern` per `ADR-0019`) inside `input_handler` (`0x402f8`) **before** the `d0 = 8 | 9 | 10` jump branch clears `ew(me, 0x34)`, when within `throw_range_px` (`<= 44 px`), powering Zangief's *Spinning Piledriver*, Mike Haggar's *Spinning Piledriver*, and Hugo's *Moonsault Press*.
   - **Screen-Edge Wall Dive (`SPECIAL_WALL_DIVE`) & Vega's Claw State (`CoreToggleState`):** Support `[4]8 + K` leaping to the nearer screen-space clamp boundary (`x = 0x60` or `x = 0x1a0` in `clamp_x`), clinging for `6` frames, and diving diagonally toward `center_x(other)` (`P` for *Rolling Izuna Drop* command grab, `K` for *Flying Barcelona Attack* slash), plus `claw_equipped: [bool; 3]` and `claw_heavy_hits: [u8; 3]` in `CoreToggleState` (`214 + P` to toggle or knocked off after `8` heavy hits, trading `-6 px` punch tip reach and `-2` damage in `judge` `0x43e82` for `+1` walk speed).
   - **Zeku's *Tenpo Kari* Form Shift (`22 + P` / `22 + K`, `COMMAND_DOWN_DOWN`):** Toggle `zeku_young_form: [bool; 3]` in `CoreToggleState` so `player_table_for_side` / `drive_roster_side` switches between **Old Zeku** (`0x32_0000`, long-range *Bushin Gram* slashes and *Bushin Flip*) and **Young Zeku** (`0x32_8000`, *Hozanto* shoulder ram, *Hayagake* run-cancel suite, and *Bushin Sho*).
   - **Belger & Damnd *Final Fight* Boss Kits (`ROUTINE_PROJECTILE_AT_EDGE`):** Standing second-phase Belger (*Crossbow Shot* `236+P` / `air 236+P`, *Executive Chair Ram* `[4]6+P`, *Penthouse Vault* `214+K`) and Thrasher Damnd with *Whistle Call* (`214+P`, spawning a Mad Gear thug rush projectile at the behind-Damnd screen edge `x = 0x64` / `0x19c` via `spawn_effect` `0x42e1e` with `ROUTINE_EFFECT_HIT` + `ROUTINE_EFFECT_VS_ATTACK`), *Thrasher Jump Kick* (`236+K`), and *Brooklyn Battering Ram* (`236+P`).
2. **Complete `DataBank` Movesets (`0x28_0000..=0x37_ffff`), Era Styles (`SF1`–`SF6`), & ROM Stand-In Catalogues for All 16 Wave 3 Challengers:**
   - Every Wave 3 challenger has **native `SF1` and `SF1+` styles**, **Canon** styles for every entry they appeared in, and **`[WHAT-IF]`** flagship styles (`SF2 ST`, `SFZ3`, `SF3 3S`, `USF4`, `SF5 CE`, `SF6`) for eras they missed (including all 6 later eras for `Mike Haggar`, `Belger`, and `Damnd`), plus entries in `moves::REGISTRY`, `front/moves.rs`, `utility.rs`, and `ComboTrialSpec` (`TRD4` `#316`).
3. **Eight Wave 3 Stages & Primary Character Themes (`0x49`–`0x50`):**
   - Register 8 Wave 3 stages (`wave = 3`) in `BackgroundBank` with eight primary character theme assignments, bringing the total stage roster to **`26` stages + `RANDOM`** (`27` tiles across `4` rows of `8` on `StageScreen`).
4. **Full 40-Fighter Composite VS Screens (`40 × 40 = 1,600`), 16 Endings (`0x44b00`), & Win Quotes (`0x4014c`):**
   - Extend `BackgroundBank::add_vs_screens` and `BackgroundBank::add_ending_screens` (`crates/sf-video/src/vs_screens.rs`) across all 40 `Wave 0..=3` fighters with a strict `<= 6` `Bg` colour-row budget per fighter bust+nameplate, character-specific `TextWriter` epilogues, and post-match win quotes.
5. **Wave 3 Secret Arcade Intrusions, 2v1 Dramatic Battle (`Presets 15–22`), & 1987 World Tour (`Nodes #26–#33`):**
   - Add 3 Wave 3 Secret Arcade Intrusion families (*Metro City Turf War*, *Bushinryu Succession*, and *Shadaloo Dragnet*) with deterministic priority rules in `tasks.rs`, 8 Dramatic Battle presets (`Presets 15–22`, including the canonical *Cody & Guy vs. Mike Haggar* and *Cody & Haggar vs. Belger* episodes), and 8 World Tour nodes (`Nodes #26–#33`).

### Non-Goals
- **Adding fighters beyond slot 39:** `RosterFighter::ALL` is capped at `40` (`ADR-0008`, `ADR-0017`).

### Anti-Goals
- **Never expose half-finished Wave 3 fighters on default menus:** Keep `RELEASED_WAVES = 2` until all 5 Wave 3 PixelLab art tickets (`UW9`–`UW13`) are complete and flip `RELEASED_WAVES = 3`.
- **Music pack.** The character arrangements live in the local `sf1-port-music` folder beside the ROMs.

---

## 3. Design

### 3.1 Pillar 1 — Wave 3 Fighters, `DataBank` Addresses (`0x28_0000..=0x37_ffff`), & Stage/Theme Mapping

| Slot | `BonusChallenger` | `name()` | `short_name()` (`<= 8` chars) | 3-Char Initials | `DataBank` Address | Home Stage (`StageView`) | Stage Owner (`<= 7` chars) | FM Theme | ROM Stand-In Base (Pre-Art Fallback) |
|---|---|---|---|---|---|---|---|---|---|
| `24` | **`Guy`** | `"Guy"` | `"GUY"` | `"GUY"` | `0x28_0000` | `"damnd"` (*Metro City Slums*) | `"Damnd"` | `0x49` | Geki/Lee ninja agile base + vermilion Bushinryu gi palette |
| `25` | **`Cody`** | `"Cody"` | `"CODY"` | `"CDY"` | `0x29_0000` | `"damnd"` (*Metro City Slums*) | `"Damnd"` | `0x49` | Joe/Ken brawler base + white T-shirt & blue jeans palette |
| `26` | **`Rolento`** | `"Rolento"` | `"ROLENTO"` | `"ROL"` | `0x2a_0000` | `"sodom"` (*Metro City Subway*) | `"Sodom"` | `0x4b` | Eagle baton base + olive fatigues & red beret palette |
| `27` | **`Sodom`** | `"Sodom"` | `"SODOM"` | `"SOD"` | `0x2b_0000` | `"sodom"` (*Metro City Subway*) | `"Sodom"` | `0x4b` | Birdie/Retsu heavy armor base + blue/red kabuki armor palette |
| `28` | **`Rose`** | `"Rose"` | `"ROSE"` | `"ROS"` | `0x2c_0000` | `"rose"` (*Palazzo Mistero*) | `"Rose"` | `0x4e` | Gen/Eagle scarf-strike base + burgundy gown & gold scarf palette |
| `29` | **`Nash`** | `"Charlie"` | `"CHARLIE"` | `"NSH"` | `0x2d_0000` | `"rose"` (*Palazzo Mistero*) | `"Rose"` | `0x4e` | Guile/Joe kick base + green vest & blonde forelock palette |
| `30` | **`Haggar`** | `"Mike Haggar"` | `"HAGGAR"` | `"HAG"` | `0x2e_0000` | `"haggar"` (*Metro City Ring*) | `"Haggar"` | `0x4a` | Birdie/Mike powerhouse base + olive wrestling trunks & strap palette |
| `31` | **`Zangief`** | `"Zangief"` | `"ZANGIEF"` | `"ZAN"` | `0x2f_0000` | `"zangief"` (*Bilsk Steelworks*) | `"Zangief"` | `0x4f` | Birdie/Mike grapple base + crimson trunks & gold boots palette |
| `32` | **`Vega`** | `"Vega"` | `"VEGA"` | `"VEG"` | `0x30_0000` | `"vega"` (*Shadaloo Golden Cage*) | `"Vega"` | `0x50` | Geki/Adon claw-leap base + purple/gold matador sash palette |
| `33` | **`Balrog`** | `"Balrog"` | `"BALROG"` | `"BAL"` | `0x31_0000` | `"vega"` (*Shadaloo Golden Cage*) | `"Vega"` | `0x50` | Mike boxer base + blue satin boxing trunks & red gloves palette |
| `34` | **`Zeku`** | `"Zeku"` | `"ZEKU"` | `"ZEK"` | `0x32_0000` | `"zeku"` (*Bushinryu Rooftop*) | `"Zeku"` | `0x4d` | Retsu/Geki ninja master base + crimson/charcoal shinobi shozoku |
| `35` | **`Belger`** | `"Belger"` | `"BELGER"` | `"BLG"` | `0x33_0000` | `"belger"` (*Belger's Penthouse*) | `"Belger"` | `0x4c` | Eagle/Sagat tall boss base + purple coat, gold hair & crossbow palette |
| `36` | **`Poison`** | `"Poison"` | `"POISON"` | `"POI"` | `0x34_0000` | `"haggar"` (*Metro City Ring*) | `"Haggar"` | `0x4a` | Lee/Adon acrobat base + magenta hair, black cap & denim palette |
| `37` | **`Hugo`** | `"Hugo"` | `"HUGO"` | `"HUG"` | `0x35_0000` | `"haggar"` (*Metro City Ring*) | `"Haggar"` | `0x4a` | Birdie/Sagat giant base + 1987 Andore leopard singlet (pink in SF3+) |
| `38` | **`Abigail`** | `"Abigail"` | `"ABIGAIL"` | `"ABI"` | `0x36_0000` | `"belger"` (*Belger's Penthouse*) | `"Belger"` | `0x4c` | Birdie/Sagat colossus base + white tank, tire pauldrons & crimson hair |
| `39` | **`Damnd`** | `"Damnd"` | `"DAMND"` | `"DMD"` | `0x37_0000` | `"damnd"` (*Metro City Slums*) | `"Damnd"` | `0x49` | Birdie/Joe punk brawler base + teal leather jacket & yellow mohawk |

---

### 3.2 Pillar 2 — Wave 3 Engine Mechanics (`UW1`)

#### A. 360° Command-Throw Recognition (`MOTION_360` & `ROUTINE_COMMAND_THROW`) and `COMMAND_DOWN_DOWN` (`22`)
1. **`SpecialSlot` Encoding (`crates/sf-engine/src/data_bank.rs`):**
   - Extend `SpecialSlot` and `validate_specials` beyond the 1987 `tracker_offset` set (`[0x16, 0x19, 0x1c]`) to accept two new command recognizers:
     - `TRACKER_MOTION_360 = 0x1d`: 360° command throw; stores `throw_range_px: u8` (`44` for Zangief's *Spinning Piledriver*, `42` for Haggar's *Spinning Piledriver*, `40` for Hugo's *Moonsault Press*, `38` for Sodom's *Butsumetsu Buster*) in the low byte of `effect_template` and uses `ROUTINE_COMMAND_THROW = 8`.
     - `TRACKER_DOWN_DOWN = 0x1e`: `22` (`Down -> Neutral -> Down` within `16` frames + `Punch`/`Kick`), used by Zeku's *Tenpo Kari*, Cody's *Knife Stance*, and Abigail's *Hungabee Tire Reflect*.
2. **Pre-Jump Priority & Input Recognition in `input_handler` (`0x402f8`, `crates/sf-engine/src/fighter.rs`):**
   - In the 1987 state machine (`fighter.rs:4029-4041`), when `dispatch_d0` is `8` (`Up`), `9` (`Up-Right`), or `10` (`Up-Left`), `input_handler` normally clears `ew(me, 0x34)` and enters jump startup (`mv = 9..=0x0b`) without checking specials.
   - Update `input_handler` (and allow cancelling pre-jump startup frames `mv = 9..=0x0b` while `!is_jump_airborne(me)`) to evaluate `TRACKER_MOTION_360` **before** the `8 | 9 | 10` jump branch clears `ew(me, 0x34)`:
     - Scans the 16-frame directional history ring buffer for at least **3 of the 4 cardinal directions** (`Forward`, `Down`, `Back`, `Up`, in clockwise or counter-clockwise progression) + `Punch`/`Kick`.
     - Under `ControlScheme::Modern` (`ADR-0019`), pressing `Back + SP` (`buttons[3]` / `LK` slot) also satisfies `TRACKER_MOTION_360` and sets `CoreToggleState::modern_scaled_move[side] = true` (`damage * 4 / 5` when `MatchConfig::modern_control_damage_penalty` is enabled).
3. **Proximity Gate, Whiff Fall-Through, & `ROUTINE_COMMAND_THROW` Object-RAM Contract:**
   - **Valid Grab Gate:** `center_x(me).abs_diff(center_x(other)) <= throw_range_px as u16`, defender grounded (`!is_jump_airborne(other)` and `eb(other, 9) & 3 != 1`), alive/standing (`eb(other, 8) == 2`), not in hitstun/blockstun (`!combo_reacting(other) && toggle_state.combo.blockstun[other_slot] == 0`), and not invulnerable (`eb(other, 0xf) & 0x08 == 0`).
   - **Whiff Behavior:** If the grab gate is not met, `TRACKER_MOTION_360` does **not** fire or consume the button latch; execution falls through cleanly to the normal `P`/`K` attack (or jump if `d0` is `8..=10`).
   - **Capture & Slam Synchronization:** When `ROUTINE_COMMAND_THROW` connects, both attacker and defender gain invulnerability bit `0x08` (`eb(e, 0xf) |= 0x08`) for the airborne arc (so neither projectiles nor a 2v1 Dramatic Battle partner can break the piledriver mid-flight), the defender's `(cx, cy)` tracks the attacker's mount offset each frame, and upon reaching `GROUND = 0xe8` the slam deals unblocked damage (`damage_with_chip` with `blocked = false`) and places the defender into knockdown (`0x34`/`0x40`).

#### B. Screen-Edge Wall Dive (`SPECIAL_WALL_DIVE`) & Vega's Claw State (`CoreToggleState`)
1. **Screen-Space Wall Dive (`[4]8 + K`):**
   - In `fighter.rs` (`clamp_x` at lines `1713-1744`), fighter X coordinates (`ew(e, 6)` / `center_x(e)`) are screen-relative and clamped to `0x60..=0x1a0`.
   - When Vega (or Guy/Zeku on their wall-jump command normals) triggers a `SPECIAL_WALL_DIVE` move (`0x1b`), the fighter leaps at high horizontal velocity toward the nearer screen-space clamp boundary (`x = 0x60` on the left or `x = 0x1a0` on the right, bypassing `push_apart` during the wall leap), clings to the boundary for `6` frames (`dx = 0, dy = 0`), flips facing toward `other`, and dives diagonally toward `center_x(other)` (`dy > 0`).
   - On each descent frame, `SPECIAL_WALL_DIVE` polls `ps` (punch latch `> 0x10`) and `ps + 1` (kick latch `> 0x10`): pressing `K` transitions the stream to the *Flying Barcelona Attack* diving claw slash, while pressing `P` within `|dx| <= 48 && |dy| <= 48` of a non-invulnerable `other` captures `other` into *Rolling Izuna Drop* (`ROUTINE_COMMAND_THROW`).
2. **Vega's Claw State (`CoreToggleState` in `crates/sf-engine/src/config.rs`):**
   - Add `claw_equipped: [bool; 3]` and `claw_heavy_hits: [u8; 3]` to `CoreToggleState` (indexed by `FIGHTER_SLOTS: [u32; 3] = [0x400, 0xa00, 0x1000]` and reset to `true` / `0` at round init `0x3ea3e`).
   - Vega's `214 + P` (*Switch Claw*) voluntarily toggles `claw_equipped[slot]`, and absorbing `8` heavy hits/blocks increments `claw_heavy_hits[slot]` and sets `claw_equipped[slot] = false` for the rest of the round.
   - In `judge` (`0x43e82`, `fighter.rs:1818`), when `!toggle_state.claw_equipped[slot]` on a punch normal (`mv` in `0x0d..=0x13` or `0x20..=0x22`), reduce attack half-width `aw` (`eb(me, 0x25)`) by `3` px and shift attack center `ax` back by `3` px (`-6 px` tip reach), and reduce base damage by `2` (`40` Sub-HP, minimum `1`), while granting `+1` walk speed.

#### C. Zeku's *Tenpo Kari* Form Shift (`zeku_young_form: [bool; 3]` in `CoreToggleState`)
- Because `drive_roster_side` (`fighter.rs:3858-3862`) re-evaluates `player_table_for_side` every frame and writes `ps + 0x12`, storing form state only in work RAM would be overwritten on the next frame.
- Store **Old Zeku's move table at `0x32_0000`** and **Young Zeku's move table at `0x32_8000`** inside Zeku's `64 KB` `DataBank` slot (`0x32_0000..=0x32_ffff`, so `roster_fighter_of_table` always maps `(table - 0x10_0000) / 0x1_0000` back to `BonusChallenger::Zeku`).
- Add `zeku_young_form: [bool; 3]` to `CoreToggleState` (reset to `false` / *Old Zeku* at round init `0x3ea3e`). Executing *Tenpo Kari* (`22 + P` or `22 + K`, `TRACKER_DOWN_DOWN`) flips `toggle_state.zeku_young_form[slot] = !toggle_state.zeku_young_form[slot]`, and `player_table_for_side` returns `0x32_8000` when `zeku_young_form[slot]` is `true` and `0x32_0000` when `false`.

#### D. Damnd's *Whistle Call* Screen-Edge Helper Projectile (`ROUTINE_PROJECTILE_AT_EDGE = 9`)
- In `fighter.rs`, `effect_offscreen` (`0x43268`, line `3113`) culls effects whose `(x, y)` leaves `x: 0x60..=0x1a0` or `y: 0x10..=0xe8`.
- Add `ROUTINE_PROJECTILE_AT_EDGE = 9` in `data_bank.rs` / `spawn_effect` (`0x42e1e`): when Damnd executes *Whistle Call* (`214 + P`), the Mad Gear thug helper spawns just inside the behind-Damnd screen boundary (`x = 0x64` when facing right, `x = 0x19c` when facing left, `y = 0xd8` / `center_y = 0xe8`) with `ROUTINE_EFFECT_HIT` (`+0x14`) and `ROUTINE_EFFECT_VS_ATTACK` (`+0x18`, allowing the defender to strike the charging thug out of the air).

#### E. Shared Stage Owner Resolution (`StageScreen::home_owner`) & `BanFormat::allows_roster`
- Extend `StageScreen::home_owner` (`crates/sf-engine/src/front/stage_select.rs`) so all 16 Wave 3 challengers resolve to their shared Wave 3 stage owner (`<= 7` chars so `TILE_NAME_CELLS = 9` never truncates):
  - `BonusChallenger::Guy | BonusChallenger::Cody | BonusChallenger::Damnd => "Damnd"`
  - `BonusChallenger::Haggar | BonusChallenger::Hugo | BonusChallenger::Poison => "Haggar"`
  - `BonusChallenger::Sodom | BonusChallenger::Rolento => "Sodom"`
  - `BonusChallenger::Belger | BonusChallenger::Abigail => "Belger"`
  - `BonusChallenger::Zeku => "Zeku"`
  - `BonusChallenger::Rose | BonusChallenger::Nash => "Rose"`
  - `BonusChallenger::Zangief => "Zangief"`
  - `BonusChallenger::Vega | BonusChallenger::Balrog => "Vega"`
- Extend `BanFormat::allows_roster` (`roster.rs`):
  - `BanFormat::Unrestricted` and `BanFormat::LowTierNoShoto` allow all 16 Wave 3 challengers when `enable_bonus_challengers` is `true` (none of the 16 are Ansatsuken/Saikyo shotos).
  - `BanFormat::StandardTournament` bans boss-tier `Belger` (`slot 35`) when `balanced` is `false` and allows all 16 when `balanced` is `true`.

---

### 3.3 Pillar 3 — Movesets & Era Styles (`SF1`–`SF6`) for All 16 Wave 3 Challengers (`UW2`–`UW5`)

#### Group 1 (`UW2`): Bushinryu & Metro City Heroes (`Guy`, `Cody`, `Mike Haggar`, `Zeku`)
1. **Guy (`slot 24`, `0x28_0000`):** `960` Sub-HP.
   - **Specials:** `236 + P` *Bushin Senpukyaku* (spinning hurricane kick), `236 + K` *Hayagake* (sprint with *Kyuffu* stop `LK`, *Kage Sukui* slide `MK`, *Kubikiri* overhead `HK`), `214 + P` *Hozanto* (spinning shoulder blow, projectile-invulnerable startup), `air 236 + P` (`SPECIAL_AIRBORNE_ONLY`) *Bushin Izuna Otoshi* (airborne trajectory into shoulder throw).
   - **Era Styles:** Native `SF1`, `SF1+`; Canon `SFZ` (`Alpha 1–3`: *Bushin Hassoken*, *Bushin Goraisenpujin*, *Tessenko*), `SF4` (`SSF4`–`USF4`: `Ultra I: Bushin Goraisenpujin`, `Ultra II: Bushin Muso Renge`); What-If `SF2 ST`, `SF3 3S`, `SF5 CE`, `SF6`.
2. **Cody (`slot 25`, `0x29_0000`):** `1000` Sub-HP (1987 Street Hero in `SF1`/`SF1+`, Prison Stripes in `SFZ3`/`SF4`, Mayor Suit in `SF5`).
   - **Specials:** `236 + P` *Bad Stone / Tornado Sweep* (throws stone projectile in `SF1`/`Alpha`/`SF4`, wind wave in `SF5`), `214 + P` *Criminal Upper* (rising tornado uppercut), `236 + K` *Ruffian Kick* (lunging dust kick), `22 + P` (`TRACKER_DOWN_DOWN`) *Knife Stance / Zonk Knuckle* (equips ground knife for boosted slash normals).
   - **Era Styles:** Native `SF1`, `SF1+`; Canon `SFZ` (`Alpha 3`: *Final Destruction*, *Dead End Irony*), `SF4` (`SSF4`–`USF4`: `Ultra I: Final Destruction`, `Ultra II: Last Dread Dust`), `SF5` (`CE`: *Side Arm* / *Dirty Coach* V-Triggers, *Criminal Punisher* CA); What-If `SF2 ST`, `SF3 3S`, `SF6`.
3. **Mike Haggar (`slot 30`, `BonusChallenger::Haggar`, `0x2e_0000`):** `1100` Sub-HP.
   - **Specials:** `P+K` / `623 + P` *Double Lariat* (full-body invulnerable spinning clothesline), `TRACKER_MOTION_360 + P` *Spinning Piledriver* (`42 px` command grab leaping into a spinning skull-crusher), `[4]6 + P` *Violent Axe* (advancing double-axe handle chop), `air 2 + P` (`SPECIAL_AIRBORNE_ONLY`) *Flying Body Press* (cross-up splash).
   - **Era Styles:** Native `SF1`, `SF1+`; What-If in all 6 later eras (`SF2 ST`, `SFZ3`, `SF3 3S`, `USF4`, `SF5 CE`, `SF6`; Supers: *Final Haggar Buster*, *Steel Pipe Rampage*, *Mayor's Verdict*).
4. **Zeku (`slot 34`, `0x32_0000` / `0x32_8000`):** `980` Sub-HP.
   - **Specials:** `22 + P/K` (`TRACKER_DOWN_DOWN`) *Tenpo Kari* (toggles `zeku_young_form` between *Old Zeku* at `0x32_0000` and *Young Zeku* at `0x32_8000`), *Old Zeku* `236 + K` *Bushin Gram — Koku*, `214 + K` *Bushin Gram — Teki*, `623 + K` *Bushin Gram — Ban*, `236 + P` *Bushin Flip*; *Young Zeku* `214 + P` *Hozanto*, `236 + K` *Hayagake*, `623 + P` *Bushin Sho*.
   - **Era Styles:** Native `SF1`, `SF1+`; Canon `SF5` (`CE`: *Bushinryu Shinge Kiko* / *Karura Tenzan* V-Triggers, *Batsuzan Gaisei* CA); What-If `SF2 ST`, `SFZ3`, `SF3 3S`, `USF4`, `SF6`.

#### Group 2 (`UW3`): Mad Gear Lieutenants & Enforcers (`Rolento`, `Sodom`, `Poison`, `Hugo`)
5. **Rolento (`slot 26`, `0x2a_0000`):** `960` Sub-HP.
   - **Specials:** `236 + P` *Patriot Circle* (3-part advancing baton flurry rekka), `214 + P` *Mekong Delta Attack* (backflip evade with rolling baton strike follow-up), `623 + K` *Stinger* (high-jump knife/grenade toss at peak), `air 2 + K` (`SPECIAL_AIRBORNE_ONLY`) *Rod Landing* (pogo-bounce off the baton).
   - **Era Styles:** Native `SF1`, `SF1+`; Canon `SFZ` (`Alpha 2`, `Alpha 3`: *Take No Prisoner*, *Mine Sweeper*, *Steel Rain*), `SF4` (`USF4`: `Ultra I: Patriot Sweeper`, `Ultra II: Take No Prisoner`); What-If `SF2 ST`, `SF3 3S`, `SF5 CE`, `SF6`.
6. **Sodom (`slot 27`, `0x2b_0000`):** `1050` Sub-HP.
   - **Specials:** `623 + P` *Jigoku Scrape* (charging double-jitte uppercut slash), `TRACKER_MOTION_360 + K` *Butsumetsu Buster* (`38 px` leaping suplex command grab), `623 + K` *Daikyo Burning* (running kimono-grip drag across the stage), `214 + K` *Shiraha Catch* (high/jumping anti-air weapon parry).
   - **Era Styles:** Native `SF1`, `SF1+`; Canon `SFZ` (`Alpha 1–3`: *Meido no Miyage*, *Tenchuu Satsu*, *Omega Slam*); What-If `SF2 ST`, `SF3 3S`, `USF4`, `SF5 CE`, `SF6`.
7. **Poison (`slot 36`, `0x34_0000`):** `960` Sub-HP.
   - **Specials:** `236 + P` *Aeolus Edge* (pink riding-crop energy wave), `214 + P` *Whip of Love* (3-part whip rekka), `623 + K` *Kissed By a Goddess* (invulnerable somersault flash kick), `214 + K` *Love Me Tender* (acrobatic flip kick into headscissor mount).
   - **Era Styles:** Native `SF1`, `SF1+`; Canon `SF4` (`USF4`: `Ultra I: Love Storm`, `Ultra II: Poison Kiss`), `SF5` (`CE`: *Heart Raid* whip swing, *Toxic Glamor* Molotov V-Trigger, *Love Hurricane* CA); What-If `SF2 ST`, `SFZ3`, `SF3 3S`, `SF6`.
8. **Hugo (`slot 37`, `0x35_0000`):** `1150` Sub-HP (1987 Andore leopard singlet in `SF1`/`SF1+`, pink singlet in `SF3`/`SF4`+).
   - **Specials:** `TRACKER_MOTION_360 + P` *Moonsault Press* (`40 px` command grab), `214 + P` *Giant Palm Bomber* (double thunderclap slap that destroys projectiles), `236 + K` *Monster Lariat* (1-hit armor clothesliner), `623 + K` *Shootdown Backbreaker* (leaping anti-air air-grab).
   - **Era Styles:** Native `SF1`, `SF1+`; Canon `SF3` (`2I`, `3S`: `SA I: Gigas Breaker`, `SA II: Megaton Press`, `SA III: Hammer Frenzy`), `SF4` (`USF4`: `Ultra I: Gigas Breaker`, `Ultra II: Megaton Press`); What-If `SF2 ST`, `SFZ3`, `SF5 CE`, `SF6`.

#### Group 3 (`UW4`): Mad Gear Bosses & Red Cyclone (`Zangief`, `Belger`, `Abigail`, `Damnd`)
9. **Zangief (`slot 31`, `0x2f_0000`):** `1150` Sub-HP.
   - **Specials:** `TRACKER_MOTION_360 + P` *Spinning Piledriver* (`44 px` range, massive damage), `P+K` / `623 + P` *Double Lariat* (projectile-invulnerable spinning arms), `623 + K` *Banishing Flat (Green Hand)* (advancing glowing palm strike that nullifies fireballs), `TRACKER_MOTION_360 + K` *Flying Power Bomb / Siberian Suplex* (running grab).
   - **Era Styles:** Native `SF1`, `SF1+`; Canon `SF2` (`WW`–`ST`: *Final Atomic Buster*), `SFZ` (`Alpha 2`, `Alpha 3`: *Final Atomic Buster*, *Aerial Russian Slam*), `SF4` (`SF4`–`USF4`: `Ultra I: Ultimate Atomic Buster`, `Ultra II: Siberian Blizzard`), `SF5` (`CE`: *Cossack Muscle* cyclone V-Trigger, *Bolshoi Russian Suplex* CA), `SF6` (*Cyclone Lariat*, *Bolshoi Storm Buster* CA); What-If `SF3 3S`.
10. **Belger (`slot 35`, `0x33_0000`):** `1050` Sub-HP.
    - **Specials:** `236 + P` *Crossbow Shot* (fires rapid iron bolts; `HP` fires a 2-bolt spread), `air 236 + P` (`SPECIAL_AIRBORNE_ONLY`) *Aerial Crossbow Barrage*, `[4]6 + P` *Executive Chair Ram* (summons his rocket-powered motorized executive chair for an armored full-screen charge), `214 + K` *Penthouse Vault* (agile leaping knee drop that hops over low attacks).
    - **Era Styles:** Native `SF1`, `SF1+`; What-If in all 6 later eras (`SF2 ST`, `SFZ3`, `SF3 3S`, `USF4`, `SF5 CE`, `SF6`; Supers: *Mad Gear Firing Squad*, *Executive Liquidation*, *Penthouse Window Fall*).
11. **Abigail (`slot 38`, `0x36_0000`):** `1150` Sub-HP.
    - **Specials:** `236 + P` *Abigail Punch* (rapid giant-fist flurry ending in an uppercut), `214 + P` *Nitro Charge (Run)* (armored tire-screech charge with `P` *Bay Area Sunrise* launcher, `K` *Hungabee* overhead kick, `LP+LK` command grab), `623 + P` *Giant Flip*, `22 + P` (`TRACKER_DOWN_DOWN`) *Hungabee Tire Reflect* (absorbs/parries one hit with crimson rage flash).
    - **Era Styles:** Native `SF1`, `SF1+`; Canon `SF5` (`CE`: *Max Power* armored heavy normals, *Hybrid Charge*, *Abigailizer* CA); What-If `SF2 ST`, `SFZ3`, `SF3 3S`, `USF4`, `SF6`.
12. **Damnd (`slot 39`, `0x37_0000`):** `1000` Sub-HP.
    - **Specials:** `214 + P` *Whistle Call* (`ROUTINE_PROJECTILE_AT_EDGE = 9`: blows his gang whistle, spawning a Mad Gear thug rush projectile at `x = 0x64` / `0x19c` with `ROUTINE_EFFECT_HIT` + `ROUTINE_EFFECT_VS_ATTACK`), `236 + K` *Thrasher Jump Kick* (long-arcing wall-to-center flying kick), `236 + P` *Brooklyn Battering Ram* (armored shoulder tackle), `623 + P` *Slum Laugh Upper* (anti-air hook with invulnerable startup).
    - **Era Styles:** Native `SF1`, `SF1+`; What-If in all 6 later eras (`SF2 ST`, `SFZ3`, `SF3 3S`, `USF4`, `SF5 CE`, `SF6`; Supers: *Mad Gear Gang Stampede*, *Slum King Execution*).

#### Group 4 (`UW5`): Shadaloo Dragnet & Soul Power (`Rose`, `Charlie Nash`, `Vega`, `Balrog`)
13. **Rose (`slot 28`, `0x2c_0000`):** `960` Sub-HP.
    - **Specials:** `236 + P` *Soul Spark* (scarf-ignited Soul Power projectile), `214 + P` *Soul Reflect* (absorbs/reflects incoming projectiles for Era meter or reflects diagonally), `236 + K` *Soul Spiral* (low-profiling scarf drill thrust), `623 + P` *Soul Throw* (leaping anti-air scarf grab).
    - **Era Styles:** Native `SF1`, `SF1+`; Canon `SFZ` (`Alpha 1–3`: *Aura Soul Spark*, *Aura Soul Throw*, *Soul Illusion* clone super), `SF4` (`SF4`–`USF4`: `Ultra I: Illusion Spark`, `Ultra II: Soul Satellite` dual orbiting orbs), `SF5` (`CE`: *Soul Dimension* teleport / *Soul Illusion* V-Triggers, *Fate Tarot* cards, *Soulaisen* CA); What-If `SF2 ST`, `SF3 3S`, `SF6`.
14. **Charlie Nash (`slot 29`, `BonusChallenger::Nash`, `0x2d_0000`):** `980` Sub-HP.
    - **Specials:** `[4]6 + P` (and `236 + P` in `SF5` style) *Sonic Boom* (one-handed crescent wave with faster recovery than Guile's), `[2]8 + K` (and `214 + K` in `SF5` style) *Somersault Shell (Flash Kick)* (vertical bicycle-kick flash kick), `236 + K` *Moonsault Slash* (hopping overhead axe kick), `214 + P` *Sonic Move / Tragedy Assault* (absorbs projectile or lunging life-drain grab).
    - **Era Styles:** Native `SF1`, `SF1+`; Canon `SFZ` (`Alpha 1–3`: *Sonic Break*, *Crossfire Blitz*, *Somersault Justice*), `SF5` (`CE`: *Sonic Move* teleport, *Judgement Saber* CA); What-If `SF2 ST`, `SF3 3S`, `USF4`, `SF6`.
15. **Vega (`slot 32`, `0x30_0000`):** `960` Sub-HP.
    - **Specials:** `[4]6 + P` *Rolling Crystal Flash* (multi-hit rolling claw thrust), `[4]8 + K` (`SPECIAL_WALL_DIVE`) *Flying Barcelona Attack / Rolling Izuna Drop* (screen-edge `0x60`/`0x1a0` wall bounce into dive slash `K` or slam `P`), `[2]8 + P` *Sky High Claw* (horizontal wall-to-wall glide), `214 + P` *Backslash & Switch Claw* (invulnerable backflip or toggles `claw_equipped`).
    - **Era Styles:** Native `SF1`, `SF1+`; Canon `SF2` (`WW`–`ST`: *Rolling Izuna Drop*), `SFZ` (`Alpha 3`: *Scarlet Mirage*, *Rolling Izuna Drop*, *Red Impact*), `SF4` (`SF4`–`USF4`: `Ultra I: Bloody High Claw`, `Ultra II: Splendid Claw`), `SF5` (`CE`: *Bloody Kiss* rose toss, *Bloody Rain* CA); What-If `SF3 3S`, `SF6`.
16. **Balrog (`slot 33`, `0x31_0000`):** `1050` Sub-HP.
    - **Specials:** `[4]6 + P` *Dash Straight* (lunging straight punch), `[4]6 + K` *Dash Low Smash / Dash Overhead* (low-hitting body hook on `LK`/`MK`, overhead smash on `HK`), `[2]8 + P` *Buffalo Headbutt* (invulnerable rising headbutt that passes through projectiles), `Hold P+K -> Release` *Turn Punch* (invulnerable spinning hook scaling in damage by charge duration).
    - **Era Styles:** Native `SF1`, `SF1+`; Canon `SF2` (`WW`–`ST`: *Crazy Buffalo* 5-hit rush super), `SFZ` (`Alpha 3`: *Crazy Buffalo*, *Gigaton Blow*), `SF4` (`SF4`–`USF4`: `Ultra I: Violent Buffalo`, `Ultra II: Dirty Bull`), `SF5` (`CE`: *KKB* spin dash, *Crazy Rush* V-Trigger, *Gigaton Blow* CA); What-If `SF3 3S`, `SF6`.

---

### 3.4 Pillar 4 — Eight Wave 3 Stages, Primary Character Themes (`0x49`–`0x50`), Composite VS/Endings (`40×40`), & Win Quotes (`UW6`)

#### A. Eight Wave 3 Stage Records (`wave = 3`) & YM2151 Themes (`0x49`–`0x50`)
| Stage ID | Stage Title & Setting | Listed Owner (`StageView.owner`) | Shared Home Stage For | Country | Theme Cmd | `.score` File |
|---|---|---|---|---|---|---|
| `"damnd"` | **Metro City Slums — Warehouse Alley (1987)** | `"Damnd"` | **Damnd**, **Cody**, **Guy** | `"USA"` | `0x49` | `damnd.score` |
| `"haggar"` | **Metro City — Mayor's Slum Gym & Underground Ring** | `"Haggar"` | **Mike Haggar**, **Hugo**, **Poison** | `"USA"` | `0x4a` | `haggar.score` |
| `"sodom"` | **Metro City Subway — Katana Ring** | `"Sodom"` | **Sodom**, **Rolento** | `"USA"` | `0x4b` | `sodom.score` |
| `"belger"` | **Metro City Bay Area — Belger's Penthouse Tower** | `"Belger"` | **Belger**, **Abigail** | `"USA"` | `0x4c` | `belger.score` |
| `"zeku"` | **Kyoto — Bushinryu Rooftop Moon Dojo** | `"Zeku"` | **Zeku** | `"Japan"` | `0x4d` | `zeku.score` |
| `"rose"` | **Genoa — Palazzo Mistero Tarot Harbor** | `"Rose"` | **Rose**, **Charlie Nash (`Nash`)** | `"Italy"` | `0x4e` | `rose.score` |
| `"zangief"` | **Bilsk Steelworks — Blast Furnace Ring** | `"Zangief"` | **Zangief** | `"USSR"` | `0x4f` | `zangief.score` |
| `"vega"` | **Barcelona & Las Vegas — Shadaloo Golden Cage** | `"Vega"` | **Vega**, **Balrog** | `"Spain"` | `0x50` | `vega.score` |

- With `released_waves == 3` and `enable_bonus_challengers == true`, `BackgroundBank::shown_stages` returns **`26` stages** (`10` 1987 + `2` Wave 1 + `6` Wave 2 + `8` Wave 3) + `RANDOM` (`27` tiles across `4` rows of `8` on `StageScreen`).
- `JukeboxScreen` lists all `26` stages and all **`40` fighters** (`RosterFighter::released(3, true)`).

#### B. `40 × 40 = 1,600` Composite VS Screens, 16 Endings (`0x44b00`), & Win Quotes (`0x4014c`) (`crates/sf-video/src/vs_screens.rs`)
- Extend `BackgroundBank::add_vs_screens` across all `40` fighters (`40 × 40 = 1,600` pre-built VS pairs) and `BackgroundBank::add_ending_screens` across all `40` fighters (`40` congratulatory ending views):
  1. **Pre-Art Fallback vs. `.sf1pack` Bust/Nameplate Tiles:** When a bonus challenger's `.sf1pack` bust is not yet present on disk, `vs_screens.rs` falls back to their deterministic ROM stand-in bust (`part(m, stand_in_opponent, side)` with their fighter palette tint) and synthesizes their `4 × 2` (`64 × 32` px) nameplate tiles from `short_name()`. Once `.sf1pack` assets exist (`UW9`–`UW12`), `vs_screens.rs` loads the packed `6 × 6` `Bg` bust and `4 × 2` nameplate tiles via `bank.add_tile(MapLayer::Bg, ...)`.
  2. **Palette Row Budget:** Each fighter's `6 × 6` bust + `4 × 2` nameplate is budgeted to **at most 6 `Bg` colour rows (`1..16`)**, guaranteeing `left_rows (<= 6) + right_rows (<= 6) + gold frame (row 2) <= 13 <= 15` usable colour rows so `add_vs_screens` never exhausts the 16-row `Bg` palette.
  3. Add 16 character-specific `TextWriter` epilogues (`0x44b00`) and post-match win quotes (`0x4014c`).

---

### 3.5 Pillar 5 — Wave 3 Secret Arcade Intrusions (`UW7`)

When `enable_bonus_challengers && enable_secret_arcade_intrusions && released_waves >= 3`, `tasks.rs` evaluates three new Wave 3 Secret Arcade Intrusions after Country 3 (`opponent_index == 5`) under the **Bonus Rival Exhibition Rule** (`0` credit loss on defeat):

1. ***Metro City Turf War (`Belger` / `Mike Haggar` / `Damnd`)*:**
   - Playing as **Cody**, **Guy**, or **Mike Haggar** with `continues_used == 0 && special_or_super_kos >= 2` triggers a mid-tournament grudge ambush by **Belger** on *Belger's Penthouse Tower* (`0x4c`).
   - Playing as any **Mad Gear** member (**Rolento**, **Sodom**, **Belger**, **Poison**, **Hugo**, **Abigail**, **Damnd**) with `continues_used == 0 && special_or_super_kos >= 2` triggers a mid-tournament bust by **Mayor Mike Haggar** on *Metro City Underground Ring* (`0x4a`).
   - Playing as **Joe** or **Mike** (the 1987 USA street fighters) with `continues_used == 0 && perfect_rounds >= 1` triggers a turf-war ambush by **Thrasher Damnd** on *Metro City Slums* (`0x49`).
2. ***Bushinryu Succession (`Zeku` / `Guy`)*:**
   - Playing as **Geki**, **Guy**, or **Zeku** with `continues_used == 0 && special_or_super_kos >= 2` triggers a rooftop shinobi trial against **Zeku** (or **Guy** when playing as Zeku) on *Bushinryu Rooftop Moon Dojo* (`0x4d`).
3. ***Shadaloo Dragnet (`Rose` / `Charlie Nash` / `Vega` / `Balrog`)*:**
   - Playing as **M. Bison**, **Vega**, or **Balrog** with `continues_used == 0 && special_or_super_kos >= 2` triggers an ambush by **Rose** (or **Charlie Nash** for Balrog) on *Palazzo Mistero* (`0x4e`).
   - Playing as **Guile**, **Charlie Nash (`Nash`)**, or **Eagle** with `continues_used == 0 && special_or_super_kos >= 2` triggers an assassin ambush by **Vega** on *Shadaloo Golden Cage* (`0x50`). (Note: `Chun-Li` and `Dorai` remain exclusively tied to Wave 2's *Bloodline Grudge* so their father/daughter exhibition is never shadowed.)
4. **Deterministic Mid-Tournament Priority Order (After Country 3 / `opponent_index == 5`):**
   1. Wave 2 *Bloodline Grudge* (`Gou Hibiki` for `Sagat`/`Adon`/`Dan`; `Dorai` for `Gen`/`Chun-Li`; `Gen` for `Dorai`)
   2. Wave 3 *Metro City Turf War* (`Belger` / `Haggar` / `Damnd`)
   3. Wave 3 *Bushinryu Succession* (`Zeku` / `Guy`)
   4. Wave 3 *Shadaloo Dragnet* (`Rose` / `Nash` / `Vega`)
   5. Wave 2 *Master's Trial* (`Gouken` / `Goutetsu`)

---

### 3.6 Pillar 6 — Wave 3 in 2v1 Dramatic Battle (`Presets 15–22`) & 1987 World Tour (`Nodes #26–#33`) (`UW8`)

#### A. Eight New Wave 3 Rivalry Presets in `DramaticSetupScreen` (`Presets 15–22`, gated on `released_waves >= 3`)
| # | Preset Title | Allies (Team of 2) | Boss (1536 Sub-HP, 1.25× Dmg) | Stage & Theme | Lore Tie |
|---|---|---|---|---|---|
| 15 | **Brawl in the Slums** | **Cody & Guy** | **Mike Haggar** | USA — *Metro City Underground Ring* (`0x4a`) | Preserved from [`2026-11-03-bonus-roster-by-lore-design.md` §8](2026-11-03-bonus-roster-by-lore-design.md): Metro City's two street heroes test their mettle against Mayor Haggar |
| 16 | **Final Fight: Top Floor** | **Cody & Mike Haggar** | **Belger** | USA — *Belger's Penthouse Tower* (`0x4c`) | Cody and Mayor Haggar storm Belger's penthouse to rescue Jessica |
| 17 | **Bushinryu Succession** | **Guy & Geki** | **Zeku** | Japan — *Bushinryu Rooftop Moon Dojo* (`0x4d`) | Two rival ninja clans confront the 38th Grandmaster of Bushinryu |
| 18 | **Operation Shadaloo Hunt** | **Guile & Charlie (`Nash`)** | **M. Bison** | USA — *US Air Force Tarmac* (`0x46`) | Best friends Guile and First Lt. Charlie Nash team up against Lord Bison |
| 19 | **Tarot & Twilight** | **Rose & Charlie (`Nash`)** | **M. Bison** | Italy — *Palazzo Mistero Tarot Harbor* (`0x4e`) | Soul Power mystic Rose and Lt. Nash unite to stop Bison's 1987 ascension |
| 20 | **Heavyweight Grudge** | **Mike & Joe** | **Balrog** | Spain/USA — *Shadaloo Golden Cage* (`0x50`) | 1987 USA heavyweights Mike and Joe take on Shadaloo's disgraced Vegas champion |
| 21 | **Giant Tag Team** | **Hugo & Poison** | **Zangief** | USSR — *Bilsk Steelworks* (`0x4f`) | Promoter Poison and Andore-era Hugo challenge the Red Cyclone of the USSR |
| 22 | **Scrapyard Demolition** | **Sodom & Rolento** | **Abigail** | USA — *Belger's Penthouse Tower* (`0x4c`) | Mad Gear lieutenants try to restrain a rampaging Abigail |

#### B. Eight Wave 3 Nodes in `1987 World Tour` (`Nodes #26–#33`, gated on `released_waves >= 3`)
| # | Node ID | Country Region | Prerequisite Node | Mission Type | Opponent(s) & Stage | Reward |
|---|---|---|---|---|---|---|
| 26 | `us-6-slums` | USA (Metro City) | `us-5-airbase` | **Rule-Modifier Bout (Street Brawler)** | **Damnd & Cody** (`Metro City Slums`, `0x49`) | `+350 XP` + *Metro City Vigilante* Perk |
| 27 | `us-7-subway` | USA (Metro City) | `us-6-slums` | **2v1 Dramatic Gauntlet** | **Sodom & Rolento** (`Metro City Subway`, `0x4b`) | `+400 XP` + *Katana & Baton* Badge |
| 28 | `us-8-ring` | USA (Metro City) | `us-7-subway` | **2v1 Grappling Trial** | **Hugo, Poison & Mike Haggar** (`Metro City Ring`, `0x4a`) | `+450 XP` + *Slam Master* Perk (`+15%` command throw damage) |
| 29 | `us-9-penthouse` | USA (Metro City) | `us-8-ring` | **2v1 Boss Raid** | **Abigail & Belger** (`Belger's Penthouse`, `0x4c`) | `+500 XP` + *Savior of Metro City* Title |
| 30 | `jp-6-bushinryu` | Japan | `jp-5-ansatsuken` | **2v1 Era Style Mastery Bout** | **Guy & Zeku** (`Bushinryu Rooftop`, `0x4d`) | `+450 XP` + *39th Bushinryu Successor* Perk |
| 31 | `cn-6-furnace` | China / USSR | `cn-5-kowloon` | **360° Iron Body Trial** | **Zangief** (`Bilsk Steelworks`, `0x4f`) | `+400 XP` + *Red Cyclone Iron Muscle* Perk |
| 32 | `en-6-palazzo` | England / Europe | `en-5-ashram` | **2v1 Soul Power Bout** | **Rose & Charlie (`Nash`)** (`Palazzo Mistero`, `0x4e`) | `+450 XP` + *Soul Tarot* Perk |
| 33 | `th-6-shadaloo` | Thailand | `th-5-eyepatch` | **Four Kings Final Round Raid** | **Balrog, Vega & M. Bison** (`Shadaloo Golden Cage`, `0x50`) | `+600 XP` + *Wave 3: Final Round* ★ Star |

---

### 3.7 Pillar 7 — PixelLab Art Pass (`UW9`–`UW13`) & Flipping `RELEASED_WAVES = 3` (`8×5 = 40` Select Grid)

To keep PixelLab credit usage disciplined and explicit (`ADR-0006`):
1. **`UW9` (Bushinryu & Metro City Heroes Sprites & Busts):** Generate 1987-style sprite sheets and 6×6 VS busts for **Guy**, **Cody**, **Mike Haggar**, and **Zeku** (both Old and Young stances).
2. **`UW10` (Mad Gear Lieutenants & Enforcers Sprites & Busts):** Generate 1987-style sprite sheets and 6×6 VS busts for **Rolento**, **Sodom**, **Poison**, and **Hugo (Andore)**.
3. **`UW11` (Mad Gear Bosses & Red Cyclone Sprites & Busts):** Generate 1987-style sprite sheets and 6×6 VS busts for **Zangief**, **Belger**, **Abigail**, and **Damnd**.
4. **`UW12` (Shadaloo Dragnet, Soul Power & 8 Wave 3 Stage Backdrops):** Generate 1987-style sprite sheets and 6×6 VS busts for **Rose**, **Charlie Nash (`Nash`)**, **Vega**, and **Balrog**, plus the 8 Wave 3 stage backdrops (`"damnd"`, `"haggar"`, `"sodom"`, `"belger"`, `"zeku"`, `"rose"`, `"zangief"`, `"vega"`).
5. **`UW13` (16 Drawn-to-Size `26×30` Select Portraits & `RELEASED_WAVES = 3` Gate Flip):** Generate the 16 drawn-to-size `26×30` (`24×28` inner) Character Select portraits for `slots 24..40` and **flip `RELEASED_WAVES` from `2` to `3`**, completing the **`8×5 = 40`** Character Select screen (`ADR-0008`, `ADR-0017`).

---

## 4. What's Left to the Builder

The following implementation details are intentionally left to engineering judgment during ticket execution:
- Exact byte offsets within `0x28_0000..=0x37_ffff` in `DataBank` for each Wave 3 challenger's move streams, projectile templates, `ROUTINE_COMMAND_THROW` capture offsets, `SPECIAL_WALL_DIVE` trajectories, and Zeku's `0x32_0000` / `0x32_8000` move tables.
- Local YM2151 scores and review evidence follow the approved individual character assignments.
- Exact `(x, y)` coordinates of the 8 `[WAVE 3]` nodes (`Nodes #26–#33`) on `WorldTourScreen`.
