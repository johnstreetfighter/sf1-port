---
status: accepted
---

# The announcer plays on a third emulated MSM5205 beside the cabinet's two

The 1987 board has two MSM5205 voice chips. The ROM's sound program gives every sample a fixed one, and a new sample cuts off whatever that chip is playing. The overhaul's announcer lines need to sound like the board, as 4-bit MSM5205 ADPCM at 4 kHz. We decided that they play on a third emulated MSM5205 of the same kind, mixed beside the original two, and never on the ROM's own chips. That way the announcer never cuts off a 1987 hit or shout, and the 1987 sound program and its latch run exactly as they do today, so the faithful audio checks still hold in overhaul fights.

## Considered options

- Sharing the two original chips through the ROM's sound program: literally on the board's hardware, but each line would cut off a punch or a shout, and the samples CPU's state would drift from the faithful run.

## Consequences

The mixer carries a channel the cabinet never had. The announcer never overlaps himself, because he has one chip: a priority order decides which line plays. The design is [`docs/design/2026-11-04-era-hud-and-callouts-design.md`](../design/2026-11-04-era-hud-and-callouts-design.md).

## First amendment (2026-11-09): the alternate Japanese announcer's classic calls

When the player chooses the alternate `JAPANESE` announcer voice (`#348`), one voice calls both the twenty-two overhaul call-outs and the thirteen 1987 tournament announcements (`JAPAN`, `U.S.A.`, `ENGLAND`, `CHINA`, `THAILAND`, `FIGHT!`, `GAME OVER`, `TIME UP!`, and `ROUND 1` through `ROUND 5`, sample commands `0x8f`–`0x9b`). In an overhaul fight under `JAPANESE`, when the Japanese voice pack holds the clip for one of those thirteen commands, the host masks that frame's output latch to idle (`0xff`) before the samples CPU sees it (the same boundary the Jukebox uses for stage themes) and plays the line on the third MSM5205 instead. The 68000's sound queue in work RAM is untouched, `RINGSIDE` and `Faithful 1987` still play the 1987 ROM samples on the first MSM5205, and any missing stem falls back to the 1987 ROM sample. The design is [`docs/design/2026-11-09-japanese-announcer-design.md`](../design/2026-11-09-japanese-announcer-design.md).
