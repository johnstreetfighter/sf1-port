---
status: accepted
---

# Individual character themes use a larger separate native score store

The user approved the [character-theme design](../design/2026-11-10-character-themes-design.md) on 2026-11-10: all 28 bonus challengers have individual native themes, even on shared stages, and the thirty-arrangement library uses larger separate logical score storage while the original sound CPU ROM/RAM map stays intact. We chose this over small per-fight images because those retain a 16 KiB limit on each required theme set and complicate in-fight changes and saved music. Playback remains on the native driver and SF1 instruments; current and saved streams retain source identity, positions, voice controls, loops, and elapsed-clock state through priority changes, effects, stops, and library replacement, and Faithful 1987 opens no music pack or extended library.

Arcade uses the CPU opponent's character theme. Multiplayer and Training use the selected stage's associations: one associated participant supplies the theme, two alternate each match, and no associated participants means cycling the stage's associated themes. Rotations are deterministic session presentation state; Jukebox overrides and missing local scores do not change their phase. Each arrangement requires a source card and reviews of the short proof, complete mix, and solo parts before acceptance.

This amends [ADR-0009](0009-stages-and-themes-in-extended-banks.md) and the older shared-stage theme assumptions in the wave plans. Ryu retains command `0x41`, the original test theme retains `0x40`, and Evil Ryu uses `0x51`.

## Repository music pack (2026-11-13)

The original design kept all arrangements local. The user later chose to
track the thirty approved playable character scores in
`assets/sf1-port-music`. The launcher finds the pack under `assets` beside
the executable or in the current directory. Valid ROM-adjacent scores
override the tracked score with the same identity.

The Windows release ZIP includes the thirty approved character scores, the
tagged music manifest, and the music pack README alongside the art and voice
packs. The title arrangement, extracted event data, source captures, and
listening artifacts remain local. See the [extended tunes reference](../reference/extended-tunes.md#the-music-pack).
