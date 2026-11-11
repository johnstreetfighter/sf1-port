# 21. Wave 3 (*Ultra Street Fighter: Final Round*): 360° Command Throws, Screen-Edge Wall Dives, Zeku's Form Shift, and 8 Shared Stage Owners

Date: 2026-11-09

## Status

Accepted (`Spec: Bonus Challengers — Wave 3 (Ultra Street Fighter: Final Round) on the real engine`)

## Context

`ADR-0008` and `ADR-0017` define the 40-fighter 1987 lore roster across three waves. This design assumes the planned implementation and release of Wave 1 (`Spec #292`, `slots 12..14`) and Wave 2 (`Spec #319`, `slots 14..24`, `ADR-0020`). Neither wave is implemented on the current SF1+ baseline.

Wave 3 (*Ultra Street Fighter: Final Round*, `slots 24..40`, `0x28_0000..=0x37_ffff`) completes the 40-fighter roster with the 16 fighters who existed in 1987 across *Final Fight* (*Street Fighter '89*) and *Street Fighter Alpha*: **Guy**, **Cody**, **Rolento**, **Sodom**, **Rose**, **Charlie Nash (`Nash`)**, **Mike Haggar**, **Zangief**, **Vega**, **Balrog**, **Zeku**, **Belger**, **Poison**, **Hugo**, **Abigail**, and **Damnd**.

Four architectural decisions are required to run all 16 Wave 3 archetypes cleanly on the translated 1987 state machine (`crates/sf-engine/src/fighter.rs`, `crates/sf-engine/src/data_bank.rs`, `crates/sf-engine/src/config.rs`) without breaking Rollback Netplay (`ADR-0018`), 2v1 Dramatic Battle (`ADR-0016`), or `StageScreen` (`ADR-0020`):
1. How 360° command throws (*Spinning Piledriver*, *Moonsault Press*, *Butsumetsu Buster*) and `22` (`Down, Down`) motions avoid being swallowed by the 1987 engine's unconditional jump dispatch on `d0 = 8 | 9 | 10` (`input_handler` `0x402f8`).
2. How Vega's wall dive (*Flying Barcelona Attack* / *Rolling Izuna Drop*) and removable claw state operate in the 1987 engine's screen-relative coordinate space (`0x60..=0x1a0` in `clamp_x`).
3. How Zeku's *Tenpo Kari* (`22 + P/K`) switches between *Old Zeku* and *Young Zeku* without `drive_roster_side` overwriting `ps + 0x12` back to the base table on the next frame or breaking `roster_fighter_of_table`.
4. How Wave 3's 16 fighters map to stages and original YM2151 FM `.score` themes (`0x49`–`0x50`).

## Decision

1. **`TRACKER_MOTION_360` (`0x1d`), `ROUTINE_COMMAND_THROW` (`8`), and `TRACKER_DOWN_DOWN` (`0x1e`) in `DataBank` & `input_handler` (`0x402f8`):**
   - `input_handler` (`0x402f8`) evaluates `TRACKER_MOTION_360` (3-of-4 cardinal directions within `16` frames + `Punch`/`Kick`, or `Back + SP` under `ControlScheme::Modern` per `ADR-0019`) **before** the `d0 = 8 | 9 | 10` jump branch clears `ew(me, 0x34)` (and allows cancelling pre-jump startup frames `mv = 9..=0x0b` while `!is_jump_airborne(me)`).
   - If the opponent is grounded, standing/crouching (`eb(other, 8) == 2`), not in hitstun/blockstun, not invulnerable (`eb(other, 0xf) & 0x08 == 0`), and within `throw_range_px` (stored in the low byte of `SpecialSlot::effect_template`), `ROUTINE_COMMAND_THROW` locks both fighters with invulnerability bit `0x08` through the airborne arc and slams at `GROUND = 0xe8` for unblocked damage. If out of range, `TRACKER_MOTION_360` does not fire or consume the button latch, falling through to the normal attack or jump.
2. **Screen-Space Wall Dive (`SPECIAL_WALL_DIVE`) and `claw_equipped` in `CoreToggleState`:**
   - `SPECIAL_WALL_DIVE` (`[4]8 + K`) leaps to the nearer screen-space clamp boundary (`x = 0x60` or `x = 0x1a0`), clings for `6` frames (`dx = 0, dy = 0`), flips facing toward `other`, and dives diagonally toward `center_x(other)`, polling `ps` (`P` -> *Rolling Izuna Drop* capture) and `ps + 1` (`K` -> *Flying Barcelona Attack* slash) on descent.
   - `CoreToggleState` tracks `claw_equipped: [bool; 3]` and `claw_heavy_hits: [u8; 3]` (indexed by `FIGHTER_SLOTS: [u32; 3] = [0x400, 0xa00, 0x1000]`). When `!claw_equipped[slot]` on a punch normal in `judge` (`0x43e82`), attack half-width `aw` is reduced by `3` px and `ax` shifted back by `3` px (`-6 px` tip reach), base damage is reduced by `2` (`40` Sub-HP, minimum `1`), and walk speed increases by `+1`.
3. **Zeku's Dual Move Tables (`0x32_0000` / `0x32_8000`) & `zeku_young_form` in `CoreToggleState`:**
   - Both **Old Zeku** (`0x32_0000`) and **Young Zeku** (`0x32_8000`) reside inside Zeku's `64 KB` `DataBank` slot (`0x32_0000..=0x32_ffff`), so `roster_fighter_of_table` always resolves to `BonusChallenger::Zeku`.
   - `CoreToggleState` stores `zeku_young_form: [bool; 3]` (reset to `false` at round init `0x3ea3e`), and `player_table_for_side` / `drive_roster_side` selects `0x32_8000` when `zeku_young_form[slot]` is `true`.
4. **Damnd's *Whistle Call* Screen-Edge Helper (`ROUTINE_PROJECTILE_AT_EDGE = 9`) and 8 Shared Wave 3 Stages (`0x49`–`0x50`):**
   - `ROUTINE_PROJECTILE_AT_EDGE = 9` spawns Damnd's Mad Gear thug helper at `x = 0x64` (facing right) or `x = 0x19c` (facing left) inside `effect_offscreen`'s `0x60..=0x1a0` window with `ROUTINE_EFFECT_HIT` + `ROUTINE_EFFECT_VS_ATTACK`.
   - Wave 3's 16 fighters consolidate onto **8 shared Wave 3 stages** (`"damnd"`, `"haggar"`, `"sodom"`, `"belger"`, `"zeku"`, `"rose"`, `"zangief"`, `"vega"`, with committed original YM2151 `.score` themes `0x49`–`0x50`), bringing the complete stage roster to **`26` stages + `RANDOM`** (`27` tiles across `4` rows of `8` on `StageScreen`).

## Consequences

- Completing `UW1`–`UW8` makes all 40 fighters (`Wave 0..=3`) playable in code with deterministic ROM stand-in catalogues across Local, Arcade, Training, Combo Trials, Replay Takeover, Rollback Netplay, 2v1 Dramatic Battle, and 1987 World Tour.
- Completing the PixelLab art pass (`UW9`–`UW13`) and flipping `RELEASED_WAVES = 3` unlocks the final **`8×5 = 40`** Character Select screen (`ADR-0008`, `ADR-0017`).
