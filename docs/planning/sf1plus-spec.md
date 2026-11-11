## Problem Statement

The overhaul lets a player pick any of the twelve tournament fighters, but only Ryu and Ken are real fighters. A boss in a player's hands only fires his 1987 CPU attacks from the buttons: he can't block, most bosses can't crouch, Sagat can't jump, and nobody has command motions. The art pipeline has finished every fighter's SF1+ poses, yet none of them appears in play. Arcade still greys out the ten bosses. Nobody can throw, and the 1987 rules allow no combos. The maintainer wants the whole roster playable, at a 1987-style classic level and at a modern level, before netplay, so that he and a friend can play all twelve online.

## Solution

All twelve tournament fighters play the SF1+ era style in every overhaul fight. Each has Ryu's kind of kit: standing, crouching and jumping normals on all six buttons, high and low blocking, and his own specials on command motions, drawn with his approved poses.

- **Ruleset presets.**
  - *Classic Competitive* (renamed from SF1++ Competitive) and *Pure Footsies* play the classic rules: no combos, no cancels, no throws.
  - *Definitive Overhaul* plays the modern rules: true combos, normal-to-special cancels, and throws that can be teched.
- **Balanced roster.** A switch that evens out the twelve with numbers only. It is off in Classic Competitive and Pure Footsies, and on in Definitive Overhaul.
- **The CPU.** A new CPU uses the full kit and plays fair, like a person, at the Gameplay tab's difficulty.
- **Arcade** opens to all twelve. When the player is one of a country's pair, Ryu stands in for the player's slot and brings his own theme. A Sagat player's final battle is against Ryu.
- **Faithful 1987** is untouched.

## User Stories

### Playing the bosses
1. As a player, I want to pick any of the twelve fighters in Local, Arcade and Training under an overhaul preset, so that the whole 1987 roster is playable.
2. As a player of a boss, I want standing light, medium and heavy punches and kicks, so that he has a full set of normals like Ryu's.
3. As a player of a boss, I want crouching normals on all six buttons, so that I can fight low.
4. As a player of a boss, I want jumping normals on all six buttons, straight up and diagonal, so that I can attack from the air.
5. As a player of Sagat, I want him to jump, so that he isn't the one fighter stuck on the ground.
6. As a player of a boss, I want to block high by holding back and low by holding down-back, so that defence works the same for everyone.
7. As a player of Geki, I want a standing block, so that his teleport no longer takes the block's place.
8. As a player of a boss, I want my fighter to crouch, so that every boss can duck.
9. As a player, I want each boss's specials on command motions read by the same command trackers as Ryu's, so that inputs feel consistent across the roster.
10. As a Retsu player, I want the Kiai Shot.
11. As a Geki player, I want the Shuriken and the Teleport Strike.
12. As a Joe player, I want the Rolling Sobat and the Dynamite Straight.
13. As a Mike player, I want the Bison Rush and the Iron Upper.
14. As a Lee player, I want the Zesshouohken and the Double Kick.
15. As a Gen player, I want the Hyakurenko and the Gekiro, with no stance switch.
16. As a Birdie player, I want the Bull Head and the Bull Horn.
17. As an Eagle player, I want Manchester Black and St. Andrew's Green.
18. As an Adon player, I want the Jaguar Kick and the Rising Jaguar.
19. As a Sagat player, I want high and low Tiger Shots and the Tiger Knee.
20. As a Mike or Eagle player, I want the kick buttons to start his punch and stick moves, as the art pipeline decided, so that every button does something that suits him.
21. As a player, I want every move drawn with the fighter's own approved poses, never Ryu's, Ken's or another fighter's, so that each fighter looks like himself.
22. As a player, I want each boss's existing 1987 attacks to keep their 1987 timing, reach and damage, so that he keeps his 1987 character.
23. As a Joe player, I want to crouch without the game stopping, and to play Joe against Joe, so that Joe is no longer the exception.

### Classic and modern rules
24. As a classic player, I want Classic Competitive to play by 1987 rules with no combos, cancels or throws, so that the full roster feels like the 1987 game.
25. As a returning player, I want my saved SF1++ Competitive settings to load as Classic Competitive, so that the rename doesn't lose my choice.
26. As a classic player, I want Classic Competitive to keep its control improvements (positive-edge attacks, input buffering, lenient commands, interruptible walks, anytime jump attacks, standard high and low blocking), so that it plays responsively.
27. As a footsies player, I want Pure Footsies to play the classic rules with all twelve fighters and 1987-strict commands.
28. As a modern player, I want true combos in Definitive Overhaul, so that hits can link into further hits.
29. As a modern player, I want normals marked as cancelable to cancel into specials, so that I can combo into my specials.
30. As a modern player, I want close-range throws, so that a blocking opponent can be opened up.
31. As a player being thrown, I want to tech a throw by inputting a throw myself within a short window, so that throws aren't a free win.
32. As a modern player, I want a thrown fighter to use his thrown and get-up poses.
33. As a player, I want Ryu and Ken to follow the same modern rules as everyone else, so that no fighter plays by different rules.
34. As a player who makes custom presets, I want combos, cancels and throws as three separate switches, so that I can mix them.
35. As a purist, I want Faithful 1987 completely unchanged, Ryu and Ken only, so that it still matches the arcade machine.
36. As a player of the modern rules, I want the era HUD's combo count to show my combos, now that hitstun can be combo'd.

### Balanced roster
37. As a player who wants fair matches, I want a Balanced roster switch that evens out the twelve fighters, so that no boss is far stronger than Ryu or Ken.
38. As a player, I want Balanced roster to change numbers only (damage, speed, recovery, hitbox sizes and positions), never animations or moves, so that every fighter stays recognisable.
39. As a classic player, I want Balanced roster off in Classic Competitive and Pure Footsies, so that the 1987 character values stand.
40. As a modern player, I want Balanced roster on in Definitive Overhaul.
41. As a player, I want to flip Balanced roster in any preset, so that I can mix 1987 character with modern rules or the reverse.
42. As a player, I want the older damage and Shoryuken rebalances folded into Balanced roster, so that damage is never rebalanced twice.
43. As a classic player, I want every 1987 hitbox exactly as the ROM has it when Balanced roster is off, quirks included.
44. As the maintainer, I want a report of the ROM moves whose boxes or reach stand out before any box is adjusted, so that I can see which 1987 quirks will be smoothed.

### The CPU
45. As a player in Arcade, I want CPU opponents to use the full kit (blocking, crouching attacks, specials, and throws and combos under the modern rules), so that they fight like real players.
46. As a player, I want the CPU to react with a human's delay and never read my inputs, so that it feels fair.
47. As a player, I want the Gameplay tab's difficulty to set how well the CPU plays (reaction delay, choices, execution), so that Easy is easy and Very Hard is hard but fair.
48. As a Training player, I want Training's CPU opponent to be the new CPU under the overhaul presets.
49. As a purist, I want Faithful 1987 to keep Capcom's own CPU.
50. As a netplay player later, I want the CPU and every fight to replay identically from the same inputs.

### Arcade
51. As a player, I want all twelve selectable in Arcade under the overhaul presets.
52. As a boss player in Arcade, I want Ryu to take my own slot in my country's pair, fought on that slot's stage, so that I never fight myself.
53. As a Sagat player, I want my final battle to be against Ryu in Thailand.
54. As an Arcade player, I want Ryu's theme, an arrangement of his SF2 theme for the 1987 sound chip, whenever Ryu is my CPU opponent.
55. As a player without that arrangement in my music pack, I want the stage's own 1987 theme to play, with the log saying why.
56. As a boss player who clears Arcade, I want the staff roll and name entry, so that the run ends properly until each fighter gets his own ending.

### Mirrors and palettes
57. As a Local player, I want any fighter against himself, with P2 in a second palette, so that the two can be told apart.
58. As the maintainer, I want to approve each fighter's mirror palette on the review page, Ryu's and Ken's included.

### Training and the move list
59. As a Training player, I want every fighter's moves in the MOVES list, so that I can look up any boss's specials.
60. As a Training player, I want Show and Try for every fighter's moves, so that the coach can teach any of the twelve.

### Art and packs
61. As a player without the art pack, I want the game to start and play, with the bosses greyed out on every select and the log saying why, so that a missing pack never breaks the game.
62. As the maintainer, I want new effect art, such as the Kiai Shot's burst, made through the review page, with the credit cost of each round put to me first.
63. As a netplay player later, I want the handshake to check that both players have the same art pack, so that a fighter's kit is the same on both machines.

### Tuning
64. As the maintainer, I want Claude to tune every number from the research and the fighters' 1987 data, so that I can play-test the finished roster and ask for one-off fixes.
65. As a player, I want every hitbox this project writes to match what's drawn, so that attacks never hit where nothing is drawn and bodies can always be hit where they are drawn.

## Implementation Decisions

- **Approach (ADR-0014).** The bosses become 1987 player fighters. Each boss gets a player move table in Ryu's format, held in an extended data bank that the translated player code reads through an overhaul-only gate. All twelve run on the same 1987 movement, jump, block and command code. Faithful 1987 never sees the bank. Ryu's and Ken's ROM tables are unchanged.
- **Move data** is written in the 1987 formats: move streams and frame records with tiles from the fighter's pose catalogue, the pose anchor turned into the record's offsets, and hit, hurt and push boxes. Where a format limit is too small, it is extended through an overhaul-only gate (as ADR-0007 did for tiles) and the reason recorded.
- **Boxes.** A ROM move kept with its ROM tiles keeps its ROM boxes. Every box this project writes must match what is drawn. Any ROM move shown with a new pose gets new boxes.
- **ROM box audit.** Every ROM move players use, Ryu's and Ken's included, is measured against its drawing and against the research's reference numbers. The outliers go in a report to the maintainer before adjustments are tuned. Box adjustments live only in the Balanced roster layer.
- **The boss driving path is deleted.** The path that mapped buttons onto a boss's CPU attack streams goes once every boss runs on his table. Joe's untranslated crouch branch and his mirror ban go with it.
- **Modern rules as switches**, in the core-toggle style:
  - *True combos* make the hit reaction hittable, with hitstun and blockstun from move data.
  - *Cancels* open a window on normals marked cancelable, into specials.
  - *Throws* are close-range, with a tech window.

  Each is an overhaul-only gate in the 1987 player code. The match configuration carries all three, and so do the rollback hash and saved settings.
- **Presets.**
  - SF1++ Competitive is renamed Classic Competitive. An old settings file naming it still loads.
  - Classic Competitive keeps the control switches on and turns combos, cancels, throws and Balanced roster off. Pure Footsies is the same, with its 1987-strict commands.
  - Definitive Overhaul turns on combos, cancels, throws and Balanced roster.
  - Faithful 1987 turns nothing on.
- **Balanced roster** is a switch over a data layer: per fighter where that's enough, per move where it isn't. It covers damage, startup, recovery, hitbox sizes and positions, and the like. The existing `balanced_damage_and_chip` and `symmetric_shotos_and_blockable_dp` switches fold into it, and their behaviour becomes part of the layer. A move's 1987 character values are its ROM numbers wherever the ROM has the move.
- **The CPU** under the overhaul presets is new and plays through the pad. It decides from what is visible on screen, after a human-like reaction delay, and never reads the opponent's inputs. The Gameplay tab's difficulty sets its reaction delay, decision quality and execution. Any randomness is seeded from the fight state. Faithful 1987 keeps Capcom's CPU.
- **Arcade.**
  - All twelve are selectable outside Faithful 1987.
  - The stand-in is Ryu: an overhaul-only gate on the opponent-load path. When the player is one of a country's pair, Ryu takes the player's own slot on that slot's stage. A Sagat player meets Ryu last, in Thailand.
  - Whenever Ryu is the CPU opponent, his theme plays from the local music pack (an arrangement of his SF2 theme for the 1987 sound chip). It falls back to the stage's 1987 theme with a log line.
  - A boss's clear goes straight to the staff roll and name entry.
- **Mirror palettes.** Every fighter gets a hand-picked second palette for P2's side of a mirror match, approved on the review page.
- **Move registry.** Every fighter's moves are added, so MOVES, Show and Try cover all twelve.
- **Without the art pack**, the ten bosses are greyed out on every select outside Faithful 1987, with a log line. Netplay (next project) must compare art packs at the handshake.
- **New effect art** goes through the art pipeline's review page, with the maintainer asked the credit cost before each generation round.
- **Determinism.** Everything that decides a fight lives in the machine's state and snapshot, so rollback, replays and RESET POSITIONS agree with a fight played straight through.
- **Records.** The design is `docs/design/2026-11-07-sf1plus-on-the-real-engine-design.md`. ADR-0014 records the approach. `CONTEXT.md` carries *classic rules*, *modern rules*, *Balanced roster*, *1987 character values*, the SF1 and SF1+ era styles, and the new *stand-in*. The reference research is `docs/research/sf1plus-reference-numbers.md`.

## Testing Decisions

- **One seam: the game session** (the maintainer, 2026-11-07). Pad inputs go in; out come the presented frames, the fight reader (positions, health, each fighter's move and frame, boxes, hit and block events) and the log. There are no test-only entry points into the engine. Good tests check behaviour a player could see: that a move starts, hits and recovers on the frames its data says, that a block stops it, that a combo links. They never check how the code stores it.
- **What gets tested there:**
  - each fighter's normals, blocks, crouch, jumps and specials by command motion;
  - frame data and boxes against move data, and boxes against the drawing for new boxes;
  - the three modern switches, each on and off;
  - Balanced roster on and off;
  - the rename and old-file load;
  - the stand-in, Sagat's final and Ryu's theme with and without the music pack;
  - mirror palettes;
  - the move list's new entries with Show and Try;
  - the no-art-pack greying;
  - the CPU's determinism (same inputs, same fight) and its reaction delay (no response faster than its difficulty allows);
  - Faithful 1987 unchanged: its MAME checks and frame goldens still pass.
- **The boss ladder report** runs every fighter through a whole Arcade game from power-on.
- **Prior art:**
  - session-level fight tests such as the front-end layout, Training and move-list tests (`front_end_layout`, `training_round_furniture`, `try_coach`, `stage_select`);
  - the ladder probe and its report;
  - `art_at_launch` for pack-dependent behaviour;
  - the existing MAME scenario checks for Faithful 1987.

## Out of Scope

- **Other projects:**
  - Each fighter's own ending, Dramatic Battle and World Tour (project 5).
  - Era styles and gauges on the real fighters (project 4).
  - Netplay (next).
  - Bonus challengers (project 6).
- **SF1+ content not in this project:**
  - Supers and meters of any kind.
  - New poses beyond the approved catalogues (a missing pose is brought to the maintainer, not made silently).
  - Throws at the classic rules.
- **The old SF1+ practice model** is not built on. Whether it's retired is the builder's call.

## Further Notes

- **Tuning** is Claude's, from `docs/research/sf1plus-reference-numbers.md` and the 1987 data. The maintainer play-tests the finished roster and asks for one-off adjustments.
- **The ROM box audit's report** goes to the maintainer before box adjustments are tuned.
- **the maintainer's friend** needs a copy of the art pack (and the music pack, for Ryu's theme) to play online.
- **Project order:** this project, then netplay, then era styles (4), then endings, Dramatic Battle and World Tour (5), then bonus challengers (6).
