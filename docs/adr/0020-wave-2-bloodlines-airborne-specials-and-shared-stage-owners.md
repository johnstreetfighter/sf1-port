# 0020. Wave 2 Bloodlines Airborne Specials, Shared Stage Owners, and Intrusion Priority

- **Status:** Accepted
- **Date:** 2026-11-09
- **Context:** [`CONTEXT.md`](../../CONTEXT.md), [`docs/design/2026-11-09-bonus-challengers-wave-2-design.md`](../design/2026-11-09-bonus-challengers-wave-2-design.md), [`docs/design/2026-11-03-bonus-roster-by-lore-design.md`](../design/2026-11-03-bonus-roster-by-lore-design.md), [`docs/design/2026-11-03-stages-and-music-design.md`](../design/2026-11-03-stages-and-music-design.md), [`ADR-0008`](0008-bonus-roster-chosen-by-lore.md), [`ADR-0009`](0009-stages-and-themes-in-extended-banks.md), [`ADR-0014`](0014-bosses-become-1987-player-fighters.md), [`ADR-0017`](0017-forty-fighter-roster-and-satsui-awakening-on-the-1987-engine.md)

## Context

Wave 2 (*Hyper Street Fighter: Bloodlines*) adds 10 lore-admitted challengers (`slots 14..24` in `RosterFighter`, `DataBank` addresses `0x1e_0000..=0x27_ffff`): **Gou Hibiki**, **Dan**, **Gouken**, **Akuma**, **Goutetsu**, **Chun-Li**, **Guile**, **Dhalsim**, **Oro**, and **Dorai**.

Implementing Wave 2 on the real 1987 engine requires three architectural decisions:
1. Several Wave 2 fighters possess signature **airborne special moves** (Akuma's *Zanku Hadoken*, Dhalsim's *Yoga Drill* / *Yoga Mummy* / airborne *Yoga Teleport*, and Oro's airborne *Jinchu Watari*), whereas the 1987 engine only calls `validate_specials` (`0x16ca2`) from grounded states (`ryu_ground`).
2. Wave 2 introduces **6 Bloodlines stages** (`"gouken"`, `"gou-hibiki"`, `"dorai"`, `"guile"`, `"dhalsim"`, `"oro"`) shared across 10 fighters (`Gouken`/`Goutetsu`/`Akuma`, `Gou Hibiki`/`Dan`, `Dorai`/`Chun-Li`), whereas `BackgroundBank::list_wave_stage` takes one primary `owner` per stage and `JukeboxScreen` previously derived its fighter list from listed stage owners.
3. Wave 2 adds mid-tournament (after Country 3 / ladder step 5) and pre-final (before Thailand final / ladder step 9) Secret Arcade Intrusions that can overlap with each other or with Wave 1's `Evil Ryu` intrusion.

## Decisions

### 1. Airborne Special Dispatch (`SPECIAL_AIRBORNE_ONLY`) in `ryu_jump` (`crates/sf-engine/src/fighter.rs`)
- In `DataBank`, extended special slots (`0x1b`, `0x1f..=0x23`) may set the `SPECIAL_AIRBORNE_ONLY` (or `SPECIAL_GROUND_AND_AIR`) flag.
- During jump frames (`ryu_jump` in `fighter.rs`), before checking standard air normal attacks, the engine runs an airborne special check against any special slot on the fighter's active `DataBank` table marked `SPECIAL_AIRBORNE_ONLY` / `SPECIAL_GROUND_AND_AIR`.
- When an airborne special triggers, the fighter transitions into that special's animation/velocity stream while airborne and enters standard landing recovery (`ryu_land`) upon reaching `GROUND_Y`.

### 2. Primary Stage Owner Aliasing (`StageScreen::home_owner`) and 24-Fighter `JukeboxScreen` Listing
- `BackgroundBank::list_wave_stage` lists each of the 6 Wave 2 stages **once** under its primary `<= 8`-character owner (`"Gouken"`, `"G.Hibiki"`, `"Dorai"`, `"Guile"`, `"Dhalsim"`, `"Oro"`), producing `10 + 2 + 6 = 18` stage tiles + `RANDOM` (`19` tiles across 3 rows on `StageScreen`).
- `StageScreen::home_owner(fighter: RosterFighter)` aliases fighters who share a Wave 2 stage to that stage's primary listed owner (`Akuma | Goutetsu | Gouken => "Gouken"`, `Dan | GouHibiki => "G.Hibiki"`, `ChunLi | Dorai => "Dorai"`), matching how 1987 aliases `Ryu -> "Retsu"` and `Ken -> "Joe"`.
- `JukeboxScreen` (`crates/sf-engine/src/front/jukebox.rs`) populates its `FIGHTERS` section from `RosterFighter::released(released_waves, bonus_challengers)` rather than `shown_stages` owners, ensuring every released challenger (`Dan`, `Akuma`, `Goutetsu`, `Chun-Li`, etc.) gets their own Jukebox fighter row.

### 3. Deterministic Secret Arcade Intrusion Priority (`crates/sf-engine/src/tasks.rs`)
- **After Country 3 (Ladder Step 5):** At most one mid-tournament intrusion runs per Arcade run, evaluated in priority order:
  1. *Bloodline Grudge* (`Gou Hibiki` for `Sagat`/`Adon`/`Dan`, or `Dorai` for `Gen`/`Chun-Li` when `continues_used == 0 && perfect_rounds >= 1`).
  2. *Master's Trial* (`Gouken` for `Ryu`/`Ken`/`Dan`, or `Goutetsu` for `Gouken`/`Akuma`/`Retsu` when `continues_used == 0 && special_or_super_kos >= 3`).
- **Before Thailand Final (Between Ladder Steps 8 and 9):** When `released_waves >= 2` and the player is `Ryu`, `Ken`, `Gen`, `Gouken`, `Goutetsu`, or `Evil Ryu` with `continues_used == 0 && perfect_rounds >= 2 && special_or_super_kos >= 3`, **Akuma's *Shun Goku Satsu* Ambush** takes priority over Wave 1's `Evil Ryu` intrusion. All intrusions obey the **Bonus Rival Exhibition Rule** (`0` credit penalty on loss, advancing to the canonical ladder step afterward).

## Consequences

- **Positive:** Airborne specials (*Zanku Hadoken*, *Yoga Drill*, *Yoga Mummy*, *Jinchu Watari*) run natively through the translated 1987 jump/stream state machine (`ryu_jump`) and are 100% snapshot- and rollback-deterministic.
- **Positive:** `StageScreen` avoids duplicate stage tiles for shared bloodline stages while `JukeboxScreen` still exposes a dedicated per-fighter track override for all 24 fighters.
