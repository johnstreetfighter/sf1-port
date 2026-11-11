# 0019. Utility AI, Control Schemes, and Replay Takeover on the 1987 Engine

- **Status:** Accepted
- **Date:** 2026-11-09
- **Context:** [`CONTEXT.md`](../../CONTEXT.md), [`docs/design/2026-11-09-training-drills-replay-takeover-and-control-schemes-design.md`](../design/2026-11-09-training-drills-replay-takeover-and-control-schemes-design.md), [`ADR-0004`](0004-reach-fights-by-driving-the-faithful-flow.md), [`ADR-0005`](0005-game-binary-pure-screens-shared-host.md), [`ADR-0012`](0012-the-coach-reads-the-engines-command-state.md), [`ADR-0013`](0013-training-rules-run-in-the-engine-under-every-preset.md), [`ADR-0015`](0015-era-styles-and-sub-hp-on-the-1987-engine.md), [`ADR-0018`](0018-rollback-netplay-and-virtual-net-bus-on-the-1987-engine.md)

## Context

Prior to [`ADR-0004`](0004-reach-fights-by-driving-the-faithful-flow.md), `crates/sf-engine/src/utility.rs` (`decide`, `DrillRun`, `ComboTrial`) and `crates/sf-engine/src/replay.rs` (`ReplayTakeoverSession` over the checkpoint-2450 `FightMachine`) were built as standalone prototypes outside [`front::GameSession`](../../crates/sf-engine/src/front/session.rs) and [`front::fight::Fight`](../../crates/sf-engine/src/front/fight.rs). Meanwhile, [`MatchConfig`](../../crates/sf-engine/src/config.rs) already persisted `p1_scheme: ControlScheme`, `p2_scheme: ControlScheme`, and `modern_control_damage_penalty: bool`, while `Tab::Controls` (`Row::Scheme`) remained hardcoded to `"CLASSIC"` and `Fight::step` did not yet translate `ControlScheme::Pneumatic` or `ControlScheme::Modern`.

We need to decide:
1. Where per-side `ControlScheme` state (`Pneumatic` hold counters, `Modern` auto-combo state, and `modern_scaled_move: [bool; 2]` for the 20% `SP` shortcut penalty) and `UtilityAi` multi-frame macro state (`UtilityState`) live so that `fighter.rs` (`0x3c034` drain and `CPU_STRUCT` `0xffc664` decision routing) can read them directly and `PowerOnMachine::save_snapshot()` snapshots them automatically for both Rollback Netplay (`ADR-0018`) and Replay Takeover.
2. How `TrainingBox` (`PauseBox -> TRAINING`) and the `Coach` strip (`ADR-0012`) host live **Training Drills** and **Combo Trials** without conflicting with `Moves -> Try`.
3. How `.sf1replay` (`MatchReplay`) recording, 60-frame keyframe scrubbing, and 10-second Replay Takeover operate on the real `front::fight::Fight` while keeping `sf-engine` 100% file-I/O-free (`ADR-0005`).

## Decisions

### 1. `SchemeState` and `UtilityState` Live Inside `CoreToggleState` (`crates/sf-engine/src/config.rs`)
- Both `SchemeState` (per-side `Pneumatic` hold/impulse counters, `Modern` auto-combo step, and `modern_scaled_move: [bool; 2]`) and `UtilityState` (the bounded `1..=12`-frame input macro queue and `cooldown` for `CPU_STRUCT`) are stored on `CoreToggleState` (`config.rs:1350`).
- Because `CoreToggleState` is owned by `PowerOnMachine`, passed into `Bus` during `fighter.rs` execution, and cloned in `PowerOnSnapshot`, this gives us three properties at once:
  1. `0x3c034` (`drain` in `fighter.rs`) reads `modern_scaled_move[attacker_side]` directly and scales `SP`-triggered specials/Supers to `80%` (`damage * 4 / 5`, min `1`) when `MatchConfig::modern_control_damage_penalty` is `true`, while leaving manual motion specials unscaled (`100%`).
  2. When `CPU_STRUCT` (`0xffc664`) uses an `SF1+` through `SF6` style (or when a `Drill` runs in `FightKind::TrainingCpu`), `fighter.rs` routes `CPU_STRUCT` through the `DataBank` player input/posture/recognizer pipeline driven by `UtilityState` (while keeping `EraFamily::Sf1987` on the 1987 ROM decision table `0x40bd4`).
  3. Both Rollback Netplay (`FightSnapshot`, `ADR-0018`) and Replay Takeover snapshot and restore `SchemeState` and `UtilityState` bit-for-bit with zero extra plumbing, while `MatchReplay` and `NetPacket::Input` continue to transmit compact 10-bit `PlayerInputs` masks.
- Under `RulesetPreset::Faithful1987`, `Fight::step` forces both sides to `ControlScheme::Classic` so MAME 0.289 parity is never altered.

### 2. Five-Row `TrainingBox` and `Coach` Strip Integration for Drills & Combo Trials
- `TrainingBox` (`PauseBox -> TRAINING` in `front/pause.rs`, widened to `SETTINGS_COL = 11`, `SETTINGS_COLS = 26`, `SETTINGS_ROW = 8`, `SETTINGS_SLOTS = 5`) adds **`DRILL`** (`OFF`, `ANTI-AIR`, `WHIFF PUNISH`, `THROW TECH`, `DEFENSE`) and **`TRIAL`** (`OFF`, `TRIAL 1`..`TRIAL 5`).
- `DRILL`, `TRIAL`, and `Moves -> Try/Show` are mutually exclusive: activating one clears the others. Unlike `Moves -> Try`, an active `DRILL` or `TRIAL` persists across `PauseBox` open/resume and `ResetPositions` (which resets the round to `0x3ea3e` and restarts the current attempt/step at `0`).
- Both `DrillController` and `ComboTrialSpec` render their progress badges through the existing `Coach` strip (`Strip::Input { steps, lit }`) and emit concise `RomFont`-compatible uppercase ASCII feedback via `CalloutKind::fix`.

### 3. Pure `.sf1replay` (`MatchReplay`) Keyframe Scrubbing and 10-Second Replay Takeover
- `MatchReplay` (`crates/sf-engine/src/replay.rs`, binary header `b"SF1R\x01"`) records `FightKind`, `MatchConfig`, `FightDips`, `StagePick`, `flag: Option<u8>`, `seed`, fighters/styles, and per-frame `(p1_bits, p2_bits)` after `DRIVE_BLACK_FRAMES` (with `start` in bit `10` of `p1_bits`).
- `GameSession` exchanges replays with `sf-host` via `with_replays` / `set_replays` and `take_saved_replay(&mut self) -> Option<MatchReplay>` (`ADR-0005`).
- Watching a replay from `ReplaysScreen` (`MenuItem::Replays` on the Main Menu) drives a real `front::fight::Fight`, caches a `FightSnapshot` keyframe every `60` frames (`1 s`) for instant `±1 s` (`Left`/`Right`) and `±5 s` (`TabLeft`/`TabRight`) scrubbing, and offers `TAKEOVER P1` / `TAKEOVER P2` (`600` frames = `10 s`) plus 1-button `MenuInput::TabLeft` (`REWIND TO BOOKMARK`) via `PauseKind::Replay`.

## Consequences

- **Positive:** `Classic`, `Pneumatic`, and `Modern` control schemes work identically across local play, saved `.sf1replay` files, and online Rollback Netplay (`ADR-0018`) without widening the wire/replay input format.
- **Positive:** `SF1+`–`SF6` CPU opponents use their Era's actual cancels and gauge mechanics, and players can practice scenarios and combo trials or take over any saved match replay directly on the real 1987 engine.
- **Trade-off:** Caching 60-frame `FightSnapshot` keyframes during a 3-minute replay uses `180 * 40 KB ≈ 7.2 MB` of RAM while watching that replay, freed immediately upon returning to `ReplaysScreen`.
