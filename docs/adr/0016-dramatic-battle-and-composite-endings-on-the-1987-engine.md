---
status: accepted
---

# Three-fighter Dramatic Battle, ROM-composite endings, and World Tour run on the 1987 engine

Twelve-fighter Arcade endings (`0x44a40`), `2v1` Dramatic Battle (`Single Player -> Dramatic Battle` and `Multi Player -> Local Co-Op Dramatic`), and `1987 World Tour` (`Single Player -> World Tour`) run on the translated 1987 machine (`crates/sf-engine/src/tasks.rs`, `fighter.rs`, and `front/fight.rs`) rather than through the standalone `arcade.rs`, `dramatic.rs`, and `world_tour.rs` prototype sessions. In `2v1` Dramatic Battle, Ally 1 runs on `P1_STRUCT` (`0xff86b0`, object-RAM region `0x400..0x9ff`) and the Boss runs on `CPU_STRUCT` (`0xffc664`, object-RAM region `0xa00..0xfff`), while Ally 2 runs on `P2_STRUCT` (`0xff86da`) paired with a third 24-entry (`0x600`-byte) object-RAM sprite grid region (`0x1000..0x15ff` = `0xfff000..0xfff5ff`) and a third slot in `CoreToggleState` / `combo::slot`, stepped each frame through the shared `fighter.rs` routines and composited into `OverhaulScene`. The ten bosses' congratulatory ending screens (`0x44b00..0x44b86`) are built at startup in `BackgroundBank` (`ending:{Boss}`) from their 6×6 ROM VS busts (`0x3fad6`) and home-stage palettes (`0x43778`) with bespoke epilogues typed by the 1987 `TextWriter` (`0x3bb98`), while `WorldTourScreen` renders an 18-node 5-country branching campaign over the ROM's `0x3f812` world travel map (`fg_scroll = 0x1980`, palette `0x1a268`) and launches real `Fight` bouts with `TourRules`. (the maintainer, 2026-11-08, [`docs/design/2026-11-08-endings-dramatic-battle-and-world-tour-design.md`](../design/2026-11-08-endings-dramatic-battle-and-world-tour-design.md).)

## Considered options

- Wiring `DramaticBattle` and `WorldTour` in `menu.rs` to the standalone 1D/2D `DramaticBattle` (`dramatic.rs`) and `WorldTour` (`world_tour.rs`) prototype structs: violates ADR-0004, bypasses the real 1987 fighter physics, stages, audio, and Era styles (`ADR-0014`, `ADR-0015`), and maintains two parallel combat implementations.
- Dynamically swapping `P1_STRUCT` and `P2_STRUCT` in Dramatic Battle based on which ally is closer to the Boss: corrupts continuous charge/motion input buffers and animation state whenever the two allies cross each other's horizontal distance to the Boss.
- Blocking the 10 bosses' Arcade endings on external PixelLab multi-panel illustration generation: delays Project 5 on external credit renewals when the ROM's 6×6 VS busts, stage backdrops/palettes, defeated Ryu card (`0x0900`), and `TextWriter` (`0x3bb98`) already assemble authentic 1987 congratulatory screens in `BackgroundBank` with zero new tile generation.

## Consequences

The standalone prototype stepping code in `arcade.rs`, `dramatic.rs`, and `world_tour.rs` is retired once real-engine `GameSession` / `Fight` integration tests cover all three modes, leaving only pure data tables (epilogues, win quotes, Dramatic rivalry presets, the 18-node World Tour map, and `world_tour.toml` persistence). `CoreToggleState`, `combo::slot`, and `FIGHTER_SLOTS` widen from 2 to 3 slots (`0x400`, `0xa00`, `0x1000`) for `FightKind::Dramatic`, while `Faithful 1987` remains completely unchanged.
