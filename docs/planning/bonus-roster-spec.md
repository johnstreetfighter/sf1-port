## Problem Statement

The overhaul's first bonus roster (13 **bonus challengers**, 2026-10-28) was chosen largely by which characters M.U.G.E.N creators had already drawn in a 1987 style. That reason has gone. The **art pipeline** can now make 1987 frames from the ROM's own art, so community sprites no longer decide who can be added. The old list has fighters who could not have fought in 1987, or who are not real characters at all:

- **Sheng Long** began as a magazine April Fools' joke.
- **Haggar's tie to Birdie** and **Guy's tie to Geki** were invented "natural rivals", not canon.
- The list as a whole was never checked against the rule that matters to the user: could this person have been there in 1987?

The user wants a roster chosen on purpose. Every added fighter must belong in 1987, the year of the first tournament, and arrive fully finished. The roster must not crowd a select screen that is already full at twelve.

## Solution

28 **bonus challengers**, chosen by 1987 lore and capped at 40 fighters in all, released in three **roster waves** after the game is feature complete:

| Roster wave | Release name | Fighters |
|---|---|---|
| 1, tied to the 1987 tournament itself | *Super Street Fighter: Rising* | Evil Ryu, M. Bison |
| 2, tied to one of the twelve **tournament fighters** | *Hyper Street Fighter: Bloodlines* | Gou Hibiki, Dan, Gouken, Akuma, Goutetsu, young Chun-Li, Guile, Dhalsim, Oro, Dorai |
| 3, existed in 1987 | *Ultra Street Fighter: Final Round* | Guy, Cody, Haggar, Rolento, Sodom, Rose, Charlie Nash, Zangief, Vega, Balrog, Zeku, Belger, Poison, Hugo, Abigail, Damnd |

A roster wave ships whole. No fighter reaches any select screen until every fighter in that wave is fully finished:

- art and portraits;
- SF1 and SF1+;
- every **era style** the fighter would have, canon and **what-if**;
- a stage, a theme, voice and sound effects;
- an Arcade ending and rival dialogue;
- a place in secret-challenger intrusions, Dramatic Battle and World Tour.

The select screen is redesigned once, for 40, with portraits drawn for their tile size, and that redesign ships with wave 1. Everything stays behind the existing `enable_bonus_challengers` switch. The default Arcade and Faithful 1987 never change.

Full design: [`docs/design/2026-11-03-bonus-roster-by-lore-design.md`](https://github.com/johnstreetfighter/sf1-port/blob/main/docs/design/2026-11-03-bonus-roster-by-lore-design.md). Selection rules: [ADR-0008](https://github.com/johnstreetfighter/sf1-port/blob/main/docs/adr/0008-bonus-roster-chosen-by-lore.md).

## User Stories

### Who is on the roster

1. As a player, I want every bonus challenger to be someone who could have fought in 1987, so that the roster feels like part of the first tournament's world.
2. As a player, I want no fan-made or joke characters, so that every fighter is official Street Fighter or Final Fight.
3. As a lore fan, I want dead characters such as Gou Hibiki and Goutetsu allowed, so that the series' spirits and legends have a place, as Gou does in *Super Gem Fighter Mini Mix*.
4. As a lore fan, I want no one who was a child or unborn in 1987 (Alex, Sakura, Cammy, Karin, Maki), so that the timeline holds.
5. As a lore fan, I want each fighter's tie backed by Capcom or UDON material, not by the EX series or a film alone, so that the roster rests on main-series lore.
6. As a lore fan, I want Evil Ryu to be the Ryu who scarred Sagat with the Metsu Shoryuken, so that the tournament's most famous moment becomes playable.
7. As a lore fan, I want M. Bison to be the man who recruited the beaten Sagat, so that wave 1 shows what the tournament set in motion.
8. As a lore fan, I want Gou and Dan tied to Sagat, so that Sagat's eye and Dan's revenge both have their source on the roster.
9. As a lore fan, I want Gouken, Akuma and Goutetsu, so that Ryu and Ken's school is complete back to its grandmaster.
10. As a lore fan, I want young Chun-Li and her father Dorai, so that Gen's ties to her family are playable.
11. As a lore fan, I want Guile (Ken's brother-in-law and Eagle's opponent), Dhalsim (Retsu's friend) and Oro (whom Retsu knew), so that the twelve's wider circle is present.
12. As a Final Fight fan, I want Guy, Cody, Haggar and Mad Gear (Belger, Poison, Hugo as Andore, Abigail, Damnd, Rolento, Sodom), so that Final Fight, which began as *Street Fighter '89*, gets the representation later games never gave it.
13. As an Alpha fan, I want Rose, Charlie, Zangief, Vega and Balrog, so that the years between the tournament and SF2 are represented.
14. As a player, I want Zeku, so that Guy's Bushin master stands beside him.
15. As a maintainer, I want an ordered list of alternates (Edi E., then Blanka, then the rest), so that a fighter who fails a later lore check is replaced without reopening the roster.
16. As a maintainer, I want a wave 1 or wave 2 fighter who fails a lore check to leave their wave smaller, with wave 3 taking the next alternate, so that waves keep their meaning and the roster stays at 40.
17. As a maintainer, I want the cap of 40 to hold, so that the select screen never has to be redesigned again.

### Releasing roster waves

18. As a player, I want the waves to arrive in order (1, 2, 3), so that the roster grows outward from the tournament.
19. As a player, I want each wave released only when every fighter in it is fully finished, so that I never pick a half-made fighter.
20. As a player, I want no "coming soon" tiles for unreleased waves, so that the select screen only shows who I can play.
21. As a player, I want each wave release to have its own name (*Rising*, *Bloodlines*, *Final Round*), so that each feels like the next step in Capcom's lineage.
22. As a maintainer, I want this work to start only after the game is feature complete, so that it never delays the port, the overhaul or the other six projects.
23. As a maintainer, I want each wave to get its own spec and tickets when its work starts, so that detail is not written years before it's needed.
24. As a maintainer, I want a wave completeness check that fails a wave missing any part of "fully finished", so that "a wave ships whole" is enforced and not left to memory.

### What a finished bonus challenger has

25. As a player, I want every bonus challenger drawn in 1987 arcade pixel art by the art pipeline, so that they stand next to the twelve without looking pasted in.
26. As a player, I want SF1 and SF1+ as every bonus challenger's native styles, so that they all play as 1987 fighters even with what-if styles off.
27. As a player, I want each bonus challenger's canon later era styles, so that, say, Guile plays as he does in SF2 when I choose that era.
28. As a player, I want what-if era styles for the eras a bonus challenger missed, with `enable_what_if_styles` on, so that every fighter can join every era.
29. As a player, I want Dorai's movesets designed from canon, starting with the Genden kick Gen taught him, so that a never-playable character still feels official.
30. As a player, I want each bonus challenger to have a stage, a theme, voice and sound effects, so that they feel as complete as the twelve.
31. As a player, I want an Arcade ending and rival dialogue for each bonus challenger, so that playing through as them tells their story.
32. As a player, I want bonus challengers to appear in Dramatic Battle and World Tour, with lore episodes rewritten for the new roster, so that those modes use the whole cast.
33. As a player, I want Hugo's SF3 look available as an era costume, so that his later appearance is there without changing his 1989 entry.

### Arcade

34. As a player, I want Evil Ryu and M. Bison to appear in Arcade as secret challengers only when `enable_bonus_challengers` and `enable_secret_arcade_intrusions` are on, so that the default Arcade stays as it was.
35. As a player, I want losing to an intruding bonus challenger to cost me nothing, under the Bonus Rival Exhibition Rule, so that secret challengers are a reward, not a trap.
36. As a player, I want the ten country opponents never to be replaced, so that the 1987 ladder stays intact.

### The select screen

37. As a player, I want one select screen that holds all 40 and still looks good, so that the full roster fits without paging.
38. As a player, I want portraits drawn for their tile size, so that small tiles read as faces, not crops.
39. As a player, I want the select screen to keep the front end's 1987 look (the gold frame, ROM fonts, P1 yellow, P2 blue), so that it matches every other screen.
40. As a player, I want the layout redesigned once, with wave 1, so that later waves only fill in tiles.
41. As a player, I want a two-way filter (the twelve, or everyone released so far), so that I can play a pure 1987 tournament or the full cast.
42. As a player, I want one `enable_bonus_challengers` switch for all waves, so that there's one setting to understand.
43. As a player, I want Faithful 1987 never to show a bonus challenger, so that the faithful port stays faithful.
44. As a netplay player, I want both sides to agree on the switch and the released waves, so that both select screens show the same roster.

## Implementation Decisions

- **This spec is the parent for the bonus challengers project** (project 6 of seven). It replaces the roster in section 5.4 of the 2026-10-28 design. Wave 1, wave 2 and wave 3 each get their own spec and tickets, as sub-issues of this one, when their work starts.
- **A roster catalogue** is the single source of who the 40 fighters are: each bonus challenger's roster wave and whether that wave has been released. The front end, Arcade intrusions, Dramatic Battle, World Tour and netplay read it. None of them hold their own list.
- **Release is per wave, never per fighter.** A wave is either released or not. Releasing it is the step that the wave completeness check guards.
- **The wave completeness check** is a deep module with one question, "is this wave complete?". It answers from what the game actually has for each fighter in the wave:
  - approved art and drawn portraits;
  - SF1 and SF1+;
  - every era style the fighter is owed;
  - a stage, a theme, voice and sound effects;
  - an ending and rival dialogue;
  - a place in intrusions, Dramatic Battle and World Tour.

  Its answer names each missing piece.
- **Era styles the fighter is owed** are their canon later styles plus a what-if for each era they missed. SF1 and SF1+ are native for every bonus challenger, never what-ifs, which replaces the 2026-10-28 design's `[WHAT-IF] SF1` marking. For Dorai, every later era style is a what-if.
- **Era styles, gauges, what-if styles for the twelve, and the `enable_what_if_styles` switch** come from project 4. They exist before wave 1 starts, and this project builds on them.
- **The switches stay as the 2026-10-28 design set them:** `enable_bonus_challengers` and `enable_secret_arcade_intrusions`, with their preset defaults, both part of the netplay `MatchConfig`. There is no switch per wave.
- **Wave 1's Arcade content is an option.** Evil Ryu and Bison are secret challengers under the Bonus Rival Exhibition Rule, and the default ladder does not change.
- **The select screen redesign** ships with wave 1. It is laid out for 40, with portraits drawn for their tile size by the art pipeline. Wave 1's spec decides the layout and how unreleased slots are absent (no "coming soon"). The two-way filter is the twelve or everyone released.
- **The standalone combat model's bonus code** (`bonus*`, the M4–M8 work) stays reference material. Nothing here builds on it, and Sheng Long is not carried forward.
- **Edition lineage** (in `CONTEXT.md`): *Anniversary Edition* now marks era styles, and the three waves are *Super Street Fighter: Rising*, *Hyper Street Fighter: Bloodlines* and *Ultra Street Fighter: Final Round*.

## Testing Decisions

- **Good tests here check what a player or a release step can observe**: which tiles the select screen offers, and whether a wave may ship. They never check how the catalogue or the check is built inside.
- **Seam 1, the select screen.** This seam already exists: build the screen, feed `MenuInput` events, assert the tiles, the cursor and the frame's pixels. Cases:
  - switch off, so only the twelve;
  - switch on with no wave released, so only the twelve;
  - wave 1 released, so 14;
  - the filter set to the twelve;
  - Faithful 1987, where no bonus challenger ever appears;
  - an unreleased wave, which has no tile and no "coming soon";
  - the full 40 laid out and fully reachable by the cursor.

  Prior art: the front end's select-screen tests.
- **Seam 2, the wave completeness check (new).** Given a wave, it reports complete or names what's missing. Cases:
  - a wave where every fighter has every part passes;
  - remove any one part from any one fighter, and it fails, naming that fighter and part;
  - a fighter owed a what-if style who lacks it fails.

  The release step refuses an incomplete wave. Prior art: the art pack and review tests, which check the art pack's contents in a similar way.
- **Each wave's spec** adds the fight tests its fighters need, on the scenario seam the fight tests already use. That work belongs there, not here.

## Out of Scope

- Each fighter's moveset, frame data, stage, theme, ending and dialogue. Each wave's spec settles these.
- The select screen's exact layout. Wave 1's spec settles it within this spec's constraints.
- Era styles, gauges and what-if styles for the twelve (project 4).
- Any schedule. Work starts after feature complete, with no deadline.
- Any change to the twelve tournament fighters, the default Arcade, or Faithful 1987.
- Fighters past the cap of 40, and any switch per wave.

## Further Notes

- Lore was checked on 2026-11-03 against the Street Fighter Fandom wiki and the Capcom and UDON sources it cites. Each wave's spec re-checks its fighters before work starts.
- Calls made by the designer rather than the user are marked in the design doc: films don't count, Nakahira's Alpha manga does, rival-battle lists aren't ties, Dan is about Ryu and Ken's age, Hugo enters as Andore, and the order of the alternates.
- The old bonus spec, #113, stays as history for the switches and the Bonus Rival Exhibition Rule.
