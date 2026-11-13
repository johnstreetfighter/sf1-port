---
status: accepted
---

# Era styles and gauges run on the 1987 player engine with a 960 sub-HP scale

Each fighter's post-1987 era styles (`SF2`, `SFZ`, `SF3`, `SF4`, `SF5`, `SF6`, and What-If styles) run on the same translated 1987 player engine (`crates/sf-engine/src/fighter.rs` and `fight.rs`) established in ADR-0014. Each fighter's 64 KB slot in the extended data bank widens its move-header table from `0x1f` (31) to `0x40` (64) entries (`0x00..0x3f`) to hold extra specials, EX/Overdrive moves, Supers/Ultras/Critical Arts, stances, and universal system actions as standard `0x20`-byte 1987 frame streams, with a revision-delta table applying per-revision frame and damage adjustments. Overhaul fights track a baseline of `960` sub-HP (`48 × 20`). Each frame projects `(sub_hp * 48).div_ceil(max_sub_hp)` to the original player's work-RAM life remainder (`+0x06`) and four-point cells (`+0x07`). The translated life bar and KO routines read those two fields. (the maintainer, 2026-11-08, [`docs/design/2026-11-08-era-styles-and-gauges-design.md`](../design/2026-11-08-era-styles-and-gauges-design.md).)

## Considered options

- Running the standalone `CombatSession` era prototype (`alpha.rs`, `sf3.rs`, `sf4.rs`, `sf5.rs`, `sf6.rs`) for `SF2`–`SF6` fights and copying its coordinates into `WorkRam` for rendering: creates a two-engine split, breaks cross-era matches (`SF1+` vs `SF6`), and loses stage/camera/collision parity with the 1987 engine.
- Keeping the extended data bank at 31 move slots and driving all era moves procedurally from Rust match arms: splits move authoring across two incompatible representations and prevents the Move List and Coach from inspecting era moves uniformly.
- Using a `1000` or `10,000` HP internal scale: `1000` does not divide cleanly by the 48-pixel 1987 health bar (`1000 / 48 = 20.833...`), whereas `960` (`48 × 20`) preserves an exact `× 20` integer mapping for every 1987 and `SF1+` damage value while still providing `~0.1%` granularity for multi-hit Supers, `V-ISM` Custom Combos, Burnout chip, and vitality modifiers (`90%` = `864`, `95%` = `912`, `105%` = `1008`).

## Consequences

The standalone `CombatSession` stepping wrappers are retired once real-fight era integration tests replace them; pure era data tables (`era.rs`, revision deltas, What-If and Flagship kits) remain. Every era move and stance uses existing 1987 ROM tiles and approved `sf1-port-art` sprite packs, differentiated at runtime by palette flashes/tints, afterimage shadow trails, and `#216`'s `EraHud` and `CalloutBoard` stage dim / burnout grey effects.

## Move effects

The SF1 era style uses its arcade move effects, including in Definitive Overhaul. A new move effect follows the style of the fighter who produces it. In mixed-era fights, an eligible opponent can still show their own move effects, including effects on contact with the SF1 fighter. SF1+ is a separate, eligible era style.

Move Effects Preview uses original light, medium, and heavy hit sparks for eligible styles. Drive Impact has larger contact effects with a paint burst on a hit and a cool arc on guard. Throws and hard knockdowns use a wider dust burst when the defender hits the floor.
