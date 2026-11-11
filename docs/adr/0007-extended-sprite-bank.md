---
status: accepted
---

# New poses are tiles in an extended sprite bank, drawn by the game's own sprite code

New poses are reduced to the fighter's palette, cut into a rectangle of 16×16 tiles, and placed above the ROM's 14,336 sprite tiles in an extended sprite bank that only overhaul modes load. The unchanged renderer and pose-grid code draw them, and SF1+'s moves point at them. We chose this over an overlay layer painted on top of a placeholder ROM pose, because the game's own sprite path gives new poses the original mirroring, draw order and palette behaviour for free, while an overlay would duplicate all of it. The cost is that every pose must obey the engine's pose rules (a tile rectangle, at most 24 sprite entries, one palette), which the pipeline checks when it packs a pose.

## Consequences

Faithful 1987 never loads the bank and stays byte-identical. With a larger tile sheet, tile codes past the ROM no longer wrap around, so the builder must confirm the 1987 game never draws a code at or past 0x3800 before the bank is enabled.
