---
status: accepted
---

# Individual character themes use a larger separate native score store

The user approved the [character-theme design](../design/2026-11-10-character-themes-design.md) on 2026-11-10: all 28 bonus challengers have individual native themes, even on shared stages, and the thirty-arrangement library uses larger separate logical score storage while the original sound CPU ROM/RAM map stays intact. We chose this over small per-fight images because those retain a 16 KiB limit on each required theme set and complicate in-fight changes and saved music. Playback remains on the native driver and SF1 instruments; current and saved streams retain source identity, positions, voice controls, loops, and elapsed-clock state through priority changes, effects, stops, and library replacement, and Faithful 1987 opens no music pack or extended library.

Arcade uses the CPU opponent's character theme. Multiplayer and Training use the selected stage's associations: one associated participant supplies the theme, two alternate each match, and no associated participants means cycling the stage's associated themes. Rotations are deterministic session presentation state; Jukebox overrides and missing local scores do not change their phase. All Capcom arrangements and source event data remain local, with a source card and short-proof, complete-arrangement, and solo-part review before acceptance.

This amends [ADR-0009](0009-stages-and-themes-in-extended-banks.md) and the older shared-stage theme assumptions in the wave plans. It records the accepted target; implementation is still required. Ryu retains command `0x41`, the original test theme retains `0x40`, and the obsolete Evil Ryu reservation of `0x41` must be replaced explicitly.
