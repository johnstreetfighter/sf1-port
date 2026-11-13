# Character theme music pack

These 30 readable `.score` files are arrangements of music from Capcom?s arcade games. The source games are *Street Fighter II: The World Warrior* (World 910522), *Street Fighter Alpha 2*, *Street Fighter Alpha 3*, *Street Fighter III: 2nd Impact*, *Final Fight*, and *Final Fight Revenge*. Original music is credited to Capcom and its composers. Some fighters use a theme borrowed from another character or a stage. The [character theme design](../../docs/design/2026-11-10-character-themes-design.md) lists the source cue for each fighter.

The game loads the pack from `assets/sf1-port-music`, under the same asset root as the art and voice packs. It compiles the scores against your own *Street Fighter* sound ROM and plays them through the native YM2151 driver with SF1 instruments.

To override a bundled score locally, put a `.score` file with the same theme identity and sound command in `sf1-port-music` beside your ROM set. See the [native score reference](../../docs/reference/extended-tunes.md) for the file format.

Source ROMs, raw sequence and register dumps, arcade recordings, and listening renders stay local.
