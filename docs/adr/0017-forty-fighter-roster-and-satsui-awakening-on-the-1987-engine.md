# ADR-0017: Forty-Fighter Roster Bank, Center-Grid Select, and Mid-Match Satsui Awakening on the 1987 Engine

- **Status:** Accepted
- **Date:** 2026-11-09
- **Context:** [`CONTEXT.md`](../../CONTEXT.md), [`docs/design/2026-11-03-bonus-roster-by-lore-design.md`](../design/2026-11-03-bonus-roster-by-lore-design.md), [`docs/design/2026-11-08-bonus-challengers-wave-1-design.md`](../design/2026-11-08-bonus-challengers-wave-1-design.md), [`ADR-0004`](0004-reach-fights-by-driving-the-faithful-flow.md), [`ADR-0007`](0007-extended-sprite-bank.md), [`ADR-0008`](0008-bonus-roster-chosen-by-lore.md), [`ADR-0009`](0009-stages-and-themes-in-extended-banks.md), [`ADR-0014`](0014-bosses-become-1987-player-fighters.md), [`ADR-0015`](0015-era-styles-and-sub-hp-on-the-1987-engine.md), [`ADR-0016`](0016-dramatic-battle-and-composite-endings-on-the-1987-engine.md)

[ADR-0023](0023-character-themes-and-separate-native-score-storage.md) assigns Evil Ryu a separate command and specifies when a match selects its theme.

## Context

Per [`ADR-0008`](0008-bonus-roster-chosen-by-lore.md) and [`docs/design/2026-11-03-bonus-roster-by-lore-design.md`](../design/2026-11-03-bonus-roster-by-lore-design.md), the complete roster grows from the 12 original tournament fighters to **40 fighters** (`12 + 28` lore-admitted bonus challengers) across three ordered release waves:
- **Wave 1 (*Super Street Fighter: Rising*, 2 challengers):** Evil Ryu, M. Bison
- **Wave 2 (*Hyper Street Fighter: Bloodlines*, 10 challengers):** Gou Hibiki, Dan, Gouken, Akuma, Goutetsu, Chun-Li, Guile, Dhalsim, Oro, Dorai
- **Wave 3 (*Ultra Street Fighter: Final Round*, 16 challengers):** Guy, Cody, Rolento, Sodom, Rose, Nash, Haggar, Zangief, Vega, Balrog, Zeku, Belger, Poison, Hugo, Abigail, Damnd

We need a clean, zero-rework architecture on the translated 1987 engine for:
1. Addressing all 40 fighters (`0..40`) in the **Extended data bank** (`crates/sf-engine/src/data_bank.rs`), which previously ended at `0x1f_ffff` (only 16 slots of `0x1_0000`).
2. Replacing the 12-only Character Select layout (`crates/sf-engine/src/front/select.rs`) once with a 40-slot center-grid layout that accommodates Project 4's (`ADR-0015`) in-frame `ERA`/`REV`/`LOAD` Style card and the two-way `MenuInput::Filter` (`ALL` vs. `1987`).
3. Executing a mid-match **Final-Round Satsui Awakening** (`Ryu -> Evil Ryu`) when a 1P Arcade Thailand final (`opponent_index == 9`) between Ryu and Sagat (either `Human Ryu vs. CPU Sagat` or `Human Sagat vs. CPU Stand-In Ryu`) reaches a tied `1–1` deciding Round 3 (`0xffc00a == 2`).
4. Sequencing code and tests before the November 27 PixelLab art pass while strictly honoring the **"A wave ships whole"** rule (`RELEASED_WAVES = 0` until all Wave 1 sprites, busts, portraits, and stages are approved).

## Decision

1. **Unified 40-Fighter Identifier (`RosterFighter`) and `0x10_0000..=0x37_ffff` Extended Data Bank:**
   - Introduce `BonusChallenger` (`28` variants in wave order `1..=3`) and `RosterFighter` (`Tournament(TournamentFighter)` `0..12` | `Bonus(BonusChallenger)` `12..40`).
   - Widen `DATA_BANK_END` in [`crates/sf-engine/src/data_bank.rs`](../../crates/sf-engine/src/data_bank.rs) from `0x1f_ffff` to **`0x37_ffff`** (`40 × 0x1_0000 = 0x28_0000` bytes = `2.5 MB`). Because the 1987 main CPU bus maps nothing between the end of program ROM (`0x05_ffff`) and video RAM (`0x80_0000`), and `BackgroundBank::PALETTE_WINDOW` sits at `0x40_0000`, `0x10_0000..=0x37_ffff` gives every `RosterFighter` slot (`0..40`) a dedicated `64 KB` region (`EvilRyu` at `0x1c_0000`, `Bison` at `0x1d_0000`) with zero address collisions.
   - When a Wave 1 challenger's PixelLab `.sf1pack` is not yet on disk, `DataBank::build` synthesizes a deterministic ROM-backed stand-in pose catalogue so headless `Fight` and `GameSession` tests verify all moves, eras, intrusions, endings, Dramatic Battle presets, and World Tour nodes immediately, while `RELEASED_WAVES` remains `0` on default player menus until the November 27 PixelLab art tickets flip `RELEASED_WAVES = 1`.

2. **Unified 40-Slot Center-Grid Character Select (`framed_wide`, `64×108` Panels, `26×30` Tiles):**
   - `SelectScreen` renders inside `framed_wide` (`cols 1..=46`, inner area `x = 16..368`, `y = 24..200`, empty `FOOT_ROW`) across all modes, placing P1's tall panel at `(20, 32, 64, 108)` and P2/Style's tall panel at `(300, 32, 64, 108)` flanking a centered grid of `26×30` portrait tiles on a `27×32` pitch.
   - The grid uses the **"even rows"** rule (`6×2` for 12, `7×2` for 14 in Wave 1, `8×3` for 24 in Wave 2, `8×5` for 40 in Wave 3), toggles between `AllReleased` and `Tournament1987` on `MenuInput::Filter`, and flips a locked side's `64×108` panel to stacked `ERA` / `REV` / `LOAD` spinner rows (`SF1` and `SF1+` lit as native for all 40 fighters).

3. **Mid-Match Thailand Final Round 3 Transformation (`Flow::AwaitWipe` at `0x3ec58`):**
   - In 1P Arcade mode at ladder step 9 (`opponent_index == 9`) when `enable_bonus_challengers && enable_secret_arcade_intrusions && released_waves >= 1`, reaching a tied `1–1` deciding Round 3 (`bus.rb(0xffc00a) == 2`) in **Ryu vs. Sagat** or **Sagat vs. Stand-In Ryu** triggers **Final-Round Satsui Awakening** inside `Flow::AwaitWipe` (`0x3ec58`):
     - Rewrites the transforming side's move-table pointer at **`ps + 0x12`** (`P1_STRUCT + 0x12` or `CPU_STRUCT + 0x12`) to `0x1c_0000` (`EvilRyu`), sets their round max Sub-HP to `864` (`90%` of `960`, filling the 48-px HUD bar), grants a **Satsui Awakening meter surge** (`+1` Super/V-ISM stock, `+50%` Ultra/V-Gauge, or full `6`-stock Drive Gauge) on top of carried meter, switches `Pause -> MOVES` dynamically to Evil Ryu's move list, and calls `request_stage_palette` + `stage_reset` + `stage_music` for `"evil-ryu"` (*Ayutthaya Storm (Night)*, tune `0x51`).
     - If human Ryu wins Round 3 as awakened Evil Ryu, the ending sequence (`0x44b00..0x44b86`) routes to `ending:Evil Ryu` and his *Satsui no Hado* epilogue (whereas winning `2–0` without Round 3 preserves pure Ryu's classic `0x7700` ending).

## Consequences

- **Positive:** All 40 fighters share a uniform bank layout (`0x10_0000 + slot * 0x1_0000`) and a single Character Select screen; Wave 1's entire engine, moveset, stage, FM theme, intrusion, Dramatic Battle, and World Tour stack can be implemented and tested immediately before November 27 without violating the **"A wave ships whole"** invariant.
- **Negative / Trade-offs:** Maintaining the ROM-backed stand-in catalogue builder adds a small fallback helper in `data_bank.rs`, which remains useful permanently for unit tests that run with only the 1987 ROM set.
