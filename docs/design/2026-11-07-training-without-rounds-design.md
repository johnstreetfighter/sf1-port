# Training without rounds

## 1. Intent

Training should behave the way it does in later Street Fighter games. A player practising a combo on the CPU or the dummy should never have the match end under them. Health comes back by itself, and nobody is ever knocked out. There is no clock, and none of the round furniture shows: the timer, the scores and the round-win markers. The frame meter must not hide the HUD it sits over. A pause-menu switch keeps the super and other era meters full, so supers can be practised without building meter first. (the maintainer, 2026-11-07: his four Training requests.)

## 2. Goals, non-goals, anti-goals

### Goals
- **No KO, health refills** (the maintainer): in Training neither fighter can be knocked out. The player's own fighter is covered too, so the CPU can't KO them (my call, the maintainer agreed). A fighter's health comes back to full about a second after they were last hit, once they are out of their hit or block reaction. The delay is one constant, about 60 frames, tuned in play (the maintainer: "a second or so might be fine").
- **No clock** (the maintainer): the round timer stops, and with no KO and no time-out, a Training round never ends. So no round result, bonus tally, after-match box or next round ever comes. The player leaves with QUIT TO MENU.
- **No round furniture** (the maintainer): in Training the TIME digits, the P1 / HI / P2 scores and the round-win markers are hidden.
- **The same rules under every preset, Faithful 1987 included** (the maintainer). Faithful 1987 Training stops matching the arcade machine byte for byte. That is a deliberate exception, recorded as an ADR (section 7). Arcade, Local and the faithful cabinet stay exact.
- **A see-through frame meter, option B** (the maintainer picked it from mockups). The era gauges keep lifting above the meter when it's on, as today. The meter's dark panel becomes a faint tint, and its cells become partly see-through, so the floor and the fighters' feet show through.
- **FILL METERS** (the maintainer: "your rec"): a Training switch, ON or OFF. While it's on, every era meter stays full and fills straight back up after it is spent. The switch is built now. Real fights carry no era meters until project 4, so until then it has nothing to fill (section 6).
- **A TRAINING settings box, layout C** (the maintainer picked it from mockups). Training's pause box gains a `TRAINING` row that opens a small box holding `PRESET`, `HOLD COMBO` and `FILL METERS`. The main box gets shorter and has room for later Training options.
- **`RESTART ROUND` becomes `RESET POSITIONS` in Training** (my call, the maintainer agreed). The machine goes back to the round's start snapshot, both fighters at their starting places with full health. It keeps the Training settings as they are now (PRESET, HOLD COMBO, FILL METERS), not as they were at the round's start (the maintainer, grilling Q1). The snapshot holds the match configuration, so the reset puts the current one back into the machine after restoring. This also fixes a bug found in grilling: today, RESTART ROUND after a PRESET change leaves the machine on the old preset while the fight's own record says the new one. Local and Arcade keep `RESTART ROUND`.

### Non-goals
- Training options beyond FILL METERS: dummy recording, guard or stance settings, health or refill-delay choices in the menu, counter-hit settings.
- The era meters' real behaviour in fights. That is project 4's work; this design only sets the rule they follow in Training.
- Restyling the two telemetry boxes at the top (`P1 ACT:OPEN` …). The maintainer asked only about the bottom strip (my call to leave them).
- Any change to Arcade, Local or the faithful cabinet, apart from the frame meter's look when a player turns it on.

### Anti-goals (a failure even if it works)
- The front end writing health, the timer or any other fight state into the machine's RAM between frames (ADR-0004).
- A Training fight ending, or a KO, a KO's slow-motion or a round result appearing in Training under any preset.
- A combo count or damage readout in Training that is wrong because the health floor swallowed a hit.
- Arcade, Local or the faithful cabinet changing by a single byte, or by a single pixel with the overlays off. Faithful 1987 Arcade and Local must still pass their MAME checks. The only exception is the frame meter's new look, which shows wherever a player turns the meter on (5.3).
- RESET POSITIONS, rollback or replays disagreeing with a fight played straight through, because some of Training's state lives outside the machine's snapshot.
- A FILL METERS that only paints the gauges full while the meter underneath can't actually be spent.

## 3. What exists today (found while designing)

- **Training is real 1987 rounds.** `FightKind::TrainingCpu` / `TrainingDummy` (`crates/sf-engine/src/front/fight.rs`) run the translated engine. A KO or a time-out ends the round, and the match end reports `FightEvent::MatchOver` and opens the after-match box (`stops_at_match_end`).
- **The engine already knows it is in Training.** `MatchConfig.roster.mode` is stamped `PlayMode::Training` for both Training kinds under every preset (`stamped`, `session.rs`). Translated code reads it as `bus.roster.mode`: the stage gates `picked_stage` and `picked_vs` (`tasks.rs`) already gate on `Versus | Training`.
- **Health** is in the player structure: `+7` whole cells (`0xb` = full, 12 cells) and `+6` the remainder (4 points a cell; full is `+6 = 4`, `+7 = 0xb`, set by `round_hud`). Damage goes through `damage_with_chip` (`fighter.rs`, `0x3c034`), which drains it and redraws the bar's tiles. `react` picks the knock-down table when health is zero. The `chip_death` toggle already shows the pattern of a gate that stops damage at 1 point.
- **The round timer** ticks in `round_timer` (`tasks.rs`; `docs/reference/round-timer.md`). Time-over is `0x3ed26` → `time_over_hold` / `time_over_judge`.
- **Round-win markers** are drawn by `round_markers` (`0x3bed2`): one 2 × 2 block of tiles `0x1100..` per win (`+8`), from the HUD offset `+0xc`. P1's is `0x298` (text row 5, from column 12) and the CPU's `0x2e0` (row 5, from column 48) in 1P mode; `prefight.md` has every mode's offsets. Note: the Training golden `crates/sf-engine/tests/goldens/training_overlay/meter_off.png` shows two blocks in each top corner during round 1, before anyone has won. Find out why before hiding them (section 8).
- **The overhaul layout** (`Fight::layout`) already hides TIME and redraws it under the health bars, and hides the score row outside Arcade. It returns `None` under Faithful 1987, so Faithful Training today shows the plain 1987 screen.
- **The frame meter** (`overlay.rs`, `draw_frame_meter`) fills its panel (x 4, y `HEIGHT - 20`, 376 × 18) with `HUD_BG` at alpha 200/256. Its 60 cells per row are solid 4 × 6 boxes 5 px apart, so the cells themselves cover most of the strip. With the meter on, `gauge_lift` lifts the gauge corners above the panel. The `OVERLAYS` row, and so the meter, is in every fight's pause box, not only Training's.
- **The pause box** (`pause.rs`): Training's nine rows (RESUME, OVERLAYS, LOOK, GAUGE ART, RESTART ROUND, QUIT TO MENU, PRESET, HOLD COMBO, MOVES) already reach the screen's last text row, so there is no room for a tenth. `HOLD COMBO` is saved in `settings.toml` (`Settings.hold_combo`).
- **Era state in fights:** none yet. `Fight.era_hud` stays `EraHud::NONE` unless a test or tool stages it (`set_era_hud`). The six era modules (`era.rs`, `sf2_era` …) are standalone models.

## 4. Approaches

How the no-KO, the refill and the stopped clock get built was the real choice. The rest has one obvious way each (section 5).

1. **A Training gate inside the translated engine — chosen** (my recommendation; the maintainer: "your rec"). The translated routines check `bus.roster.mode == PlayMode::Training`, as the stage gates do. Damage leaves at least one point of health, so the KO path never runs. The round clock doesn't tick. The engine refills health. Everything happens inside the machine, so snapshots (RESET POSITIONS), replays and rollback see one consistent fight. ADR-0004 allows exactly this ("an overhaul-only gate on the translated path, never a RAM write"). Running it under Faithful 1987 is the new decision, recorded in its own ADR.
2. **The front end tops up health between frames.** Ruled out. ADR-0004 forbids it, and it can't work anyway: the engine decides the KO on the hit's own frame, before any top-up.
3. **Catch the KO and restore the round-start snapshot automatically.** Ruled out. It needs no engine change, but both fighters would snap back to their starting places every time the dummy "died," which ruins the combo practice this is for.

## 5. The design

### 5.1 The Training gate (engine)
- **Key:** `bus.roster.mode == PlayMode::Training`, under every preset. It is never set in Arcade, Local (`Versus`), the attract or the faithful cabinet, so they are untouched.
- **No KO:** every damage path into a player's health stops at one point left (health `+7 == 0, +6 == 1`) in Training. That includes normal hits, chip, specials, a boss's hits and anything else that drains `+6/+7`. A hit that would have finished a fighter lands as an ordinary hit: the alive reaction table, no knock-down-for-KO and no round end. Because no hit is ever lethal, nothing KO-only happens. A move that knocks down as a normal reaction still does. Ordinary 1987 behaviour that isn't about the KO is left alone. For example, under Faithful 1987 every clean projectile hit still starts the 90-frame slow motion (`SCENE_MODE |= 0x20`; the overhauls already limit it through `disable_fireball_slowdown`) (my call, found by the builder check).
- **Hits still count at the floor:** the fight reader sees a landed hit only when health goes down (`fight_reader.rs`). A hit on a fighter held at one point, or one the floor cut short, would otherwise be lost or under-counted in the combo count, HOLD COMBO's damage and FIRST ATTACK. So in Training the gate also records each landed hit and its full damage before the floor, inside the snapshot, and the reader takes that record in Training. The counts and damage then match a fight with unlimited health, as later games show them (my call: the maintainer wants it like later games, and combo practice is the point).
- **Refill:** a fighter whose health is below full, who hasn't been hit or blocked anything for `TRAINING_REFILL_FRAMES` (about 60), and who is out of their hit or block reaction gets full health. Any hit or block, even a blocked one that does no damage, starts the wait over, so health comes back only once the pressure stops (the maintainer, grilling Q2). Full health is `+6 = 4, +7 = 0xb`, with the bar's tiles redrawn as `round_hud` draws a full bar. A builder may make the bar fill up visibly over a few frames instead, if that reads better; snapping to full is fine.
- **FIRST ATTACK** calls once per Training session, on the first hit, and again after each RESET POSITIONS, which starts a new round's moments as RESTART ROUND does today. REVERSAL and the combo count are unchanged (the maintainer, grilling).
- **Clock:** `round_timer` doesn't count down in Training (the digits stay at the round's start value). So time-over never happens.
- **State:** anything the gate counts, such as frames since the last hit, must live in the machine's snapshot (`PowerOnSnapshot`), so that RESET POSITIONS and rollback restore it. Where it lives is the builder's call. It must not be a guess about unused 1987 work RAM unless it is recorded and checked.

### 5.2 What Training shows
- Training gets a layout pass under every preset, Faithful 1987 included. It hides TIME without redrawing it, the score row and both sides' round-marker cells, wherever the ROM draws them. Under the overhaul presets this is `Fight::layout` with TIME's redraw skipped in Training. Under Faithful 1987 it is a Training-only pass that hides those three things and nothing else, and keeps the 1987 PLAYER1 / ENEMY labels.
- The column wipe's rules still apply to the hidden cells.

### 5.3 The frame meter, option B
- The panel's fill drops from alpha 200 to a faint tint (the mockup used 40/256). The cells and the meter's text blend over the frame (the mockup used 170/256) instead of being drawn solid. The exact values are tuned by eye in the real game, and the Training overlay goldens are re-blessed after the maintainer looks.
- The gauge lift stays as it is. The top telemetry boxes stay as they are.
- **It applies wherever the frame meter is drawn.** The `OVERLAYS` row is in every fight's pause box, so Local and Arcade get the same see-through look when a player turns the meter on. One look for one tool (my call, found by the builder check). Every overlay golden or pixel test that includes the meter is re-blessed after the maintainer looks.

### 5.4 The pause box, layout C
- **Training's main box:** `RESUME`, `OVERLAYS`, `LOOK`, `GAUGE ART`, `RESET POSITIONS`, `QUIT TO MENU`, `TRAINING`, `MOVES`. `MOVES` stays the last row under every preset, as spec #226 set (my call; the mockup put `QUIT TO MENU` last). Eight rows fit the screen at the current spacing.
- **`TRAINING`** opens a smaller gold box over the main one, as in the mockup. It is titled `TRAINING` and holds `PRESET`, `HOLD COMBO` and `FILL METERS`. Up and down move, left, right and confirm cycle the values, and Back returns to the main box with the cursor on `TRAINING`. Start resumes the fight from either box (my call).
- **Other fights' boxes are unchanged:** Local and Arcade keep `RESTART ROUND` and have no `TRAINING` row.

### 5.5 FILL METERS
- `Settings.fill_meters`, saved in `settings.toml` like `hold_combo`, default OFF (my call). It is stamped into the fight's `MatchConfig`, so the machine carries it, and the row pushes a changed config into the live machine the way `PRESET` already does (`Fight::set_preset` → `set_match_config`). Project 4's era resources live in the machine, so they can read it there, and it is part of the snapshot. It means nothing outside Training (my call, found by the builder check).
- **The rule era meters follow:** in a Training fight with FILL METERS on, each era resource behind a gauge (super stocks, level gauges, Drive, V-Gauge and the like) is held at its maximum. It fills straight back up after it is spent, and spending it works normally. This acts on the real resource, never on the drawing alone.
- **Now vs project 4:** today no real fight has era resources, so the switch has nothing to act on. Project 4, which moves the eras onto the real fighters, must read the flag and follow the rule. That requirement is written into this design and the glossary. It is also added, as an acceptance condition, to spec #91, which tracks the multi-era suite that project 4 builds out, so it can't be lost.

## 6. Honest limits
- Until project 4, FILL METERS is a working switch with no visible effect in real fights. Its test today covers the row, the setting and the flag. The refill itself is tested when the first era reaches real fights.
- Faithful 1987 Training is no longer a byte-faithful replay of the arcade machine. That is the maintainer's call, recorded in an ADR.

## 7. Records
- **ADR-0013** (`docs/adr/0013-training-rules-run-in-the-engine-under-every-preset.md`), written in grilling: "Training's rules run in the engine under every preset." The Training gate (no KO, refill, stopped clock, hidden round furniture) applies under Faithful 1987 too, as a deliberate, Training-only exception to Faithful 1987's byte-faithful path. Arcade, Local and the faithful cabinet stay exact. It sits alongside ADR-0004, which this design follows.
- **`CONTEXT.md`**, written in grilling, in a new Training section: Training, Health floor, Refill, Reset positions, Training settings box, Fill meters.

## 8. Left to the builder
- Every path that drains health, so the floor holds on all of them (bosses' tables, chip, projectiles, throws as they arrive).
- Where the refill counter lives, within 5.1's snapshot rule; the exact refill delay (about 60 frames); whether the bar snaps or slides up.
- Why the round-1 Training frame shows two marker blocks per side. Wins carried in `+8`? Leftover tiles from the hidden drive? Something the ROM draws? If it turns out to be wins carried into Local fights too, report that as its own bug rather than fixing it here.
- The meter's exact tint and cell alpha (the maintainer looks before the goldens are blessed).
- Tests, modelled on `crates/sf-engine/tests/front_end_training.rs`, `training_overlay_clearance.rs` and `front_end_layout.rs`. At least:
  - a Training fight where the dummy (and the player) take more than full health in hits never reports `MatchOver`, never shows a KO, and is at full health a second after the last hit;
  - hits landed at the floor still count, and a hit the floor cut short reports its full damage;
  - TIME doesn't move in Training and does in Local;
  - Training frames under every preset show no TIME, scores or markers;
  - RESET POSITIONS equals a fresh round start, the refill counter included, except that it keeps a PRESET or FILL METERS change made after the start;
  - a blocked attack that does no damage starts the refill wait over;
  - the pause box's TRAINING box navigates and saves FILL METERS;
  - Arcade, Local and the faithful cabinet are unchanged, and the existing MAME-verified tests still pass.
