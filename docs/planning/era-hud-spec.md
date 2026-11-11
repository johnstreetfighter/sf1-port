# Era HUD: shared 1987-style gauges, call-outs, combo counter and announcer
Published as #216.

## Problem Statement

The overhaul brings in the resource systems of later Street Fighter entries: the ST Super stock, the Alpha gauge and Custom Combo, SF3 Super Arts, SF4 Super and Revenge, SF5 EX and V-Gauge, and SF6 Drive and Super Art. None of them has a proper face. The era modules report their gauge state, but all that draws it is a placeholder strip: a black band over the bottom of the screen, flat bars, pips and a tiny "SF" stamp. It covers TIME and the fighters' feet, and the front end keeps it out of fights on purpose.

Overhaul fights also say nothing when something big happens. There is no combo count, no word for a counter hit or a reversal, and no announcer beyond the 1987 "ROUND 1" and "FIGHT!". The Local and Training screens still carry the arcade's P1 / HI / P2 scores, which mean nothing in a two-player or practice fight.

## Solution

Overhaul fights get an **era HUD**: one invented 1987 style that every era shares, so a Drive fighter against an Alpha fighter still looks like one game.

- **Layout:**
  - Era gauges sit in the bottom corners, P1's in the bottom-left and P2's mirrored in the bottom-right.
  - TIME moves to the centre, under the PLAYER1 / PLAYER2 health bars.
  - Local and Training drop the scores, and Arcade keeps them.
- **Gauge art.** The gauges are drawn from the 1987 HUD's own pens in one of three kinds of gauge art, chosen in Settings and in the pause box: Ruled (the default), Blocks or Plate.
- **Call-outs.** Each big moment gets a call-out on the earner's side, in 1987 lettering. A combo counter runs in every overhaul fight.
- **The announcer.** He calls the moments in a generated voice designed to sound like the 1987 announcer. It is encoded as the cabinet's own 4-bit ADPCM and played on a third emulated voice chip, so he never cuts off a 1987 sound.
- **No overlaps.** Nothing ever draws over anything else, however many moments land at once.

The Faithful 1987 preset and the faithful cabinet keep the plain 1987 screen.

**Where it shows first.** The combo counter, FIRST ATTACK, REVERSAL, PERFECT and the new layout read the real 1987 fight, so they show in real fights at once. The gauges and the era-specific call-outs are built and proven against the existing era models in tests and a dev preview. They first appear in a real fight when project 4 moves each era onto the real fighters.

Design: `docs/design/2026-11-04-era-hud-and-callouts-design.md`. ADR 0011 (the announcer's third voice chip). Glossary: the Fight HUD section of `CONTEXT.md`.

## User Stories

### Layout

1. As a player in an overhaul fight, I want the era gauges in the bottom corners, P1 left and P2 mirrored right, so that each fighter's resources sit on their side of the screen.
2. As a player, I want TIME centred just under the PLAYER1 / PLAYER2 health bars, so that the bottom corners are free for gauges and everything I watch is near the health bars.
3. As a player in Local or Training, I want no P1 / HI / P2 scores, so that the top of the screen isn't cluttered with numbers that mean nothing there.
4. As a player in Arcade, I want my score shown as in 1987, so that the arcade run still keeps score.
5. As a player of any overhaul preset, Pure Footsies included, I want TIME in the same place, so that the screen layout never shifts between presets.
6. As a purist playing the Faithful 1987 preset in Local or Training, I want the exact 1987 screen (TIME bottom-left, scores shown, no era HUD), so that the faithful baseline stays untouched.
7. As a purist, I want the faithful cabinet completely unchanged, so that the 1987 machine is still the 1987 machine.
8. As a player, I want a side with no gauge (an SF1 or SF1+ fighter, or Pure Footsies) to show nothing in its corner, so that an empty frame doesn't clutter the floor.
9. As a player in Unified Gauge mode, I want both corners to show the same kind of gauge, so that the display matches the rules.

### Gauges and gauge art

10. As a player, I want every era's gauge drawn in one shared 1987 style, so that mixed-era fights look like one game.
11. As a player, I want stock gauges drawn as a bar with one cell per stock (ST 1, SF5 EX 3, SF4 Super 4, SF6 Drive 6), so that I can count stocks at a glance.
12. As a player, I want level gauges drawn as one long bar with the level number beside it (Alpha, SF3 Super Art, SF6 Super Art), so that I can read the level without counting.
13. As an SF3 player, I want the Super Art bar's length to follow the Super Art I chose, so that the gauge reflects my loadout.
14. As a player, I want a second gauge (SF4 Revenge, SF5 V-Gauge, SF6 Super Art under Drive) as a thinner row under the main one, so that both resources are visible without crowding.
15. As a player, I want a corner never to hold more than two gauge rows, so that the corner stays readable.
16. As a player, I want fills to grow from the screen edge inward, so that P1's and P2's gauges mirror each other.
17. As a player, I want the gauges drawn only from the 1987 HUD's own pens and the ROM's own lettering, so that they look as if the 1987 artists drew them.
18. As a player, I want to choose the gauge art (Ruled, Blocks or Plate) on the Settings VIDEO tab, so that I can pick the look I like.
19. As a new player, I want Ruled as the default gauge art, so that the gauges first look closest to the 1987 health bars.
20. As a player, I want a GAUGE ART row in the pause box beside LOOK, so that I can compare the three against a live fight, saved as the Settings value.
21. As a player in netplay, I want my gauge art to stay my own, so that my choice never changes my opponent's screen.
22. As a player, I want a gauge to show partial fill wherever its era tracks it (SF6 Drive half bars, SF4 Revenge, SF5 V-meter), so that I can see how close the next stock is.

### Call-outs and screen effects

23. As a player, I want a call-out word at mid-height on the side of the fighter who earned the moment, so that I know at once who did what.
24. As a player, I want call-outs in the ROM's own large lettering in PLAYER1 gold, so that they look like 1987.
25. As a player, I want a super to dim the stage behind the fighter and show SUPER with the move's name below it, so that the super lands with weight.
26. As a player, I want no screen freeze or dim ever to eat my directional or button inputs, so that a super flash never robs me of a reaction.
27. As a player, I want COUNTER and PUNISH COUNTER call-outs, so that I see when my hit was a counter.
28. As an SF5 player, I want a CRUSH COUNTER call-out, so that SF5's crush counters read as they do in their own game.
29. As an SF6 player, I want PERFECT PARRY when I parry in the perfect window, so that I know I hit it.
30. As an SF6 player, I want BURNOUT to grey the fighter and their gauge until it ends, so that I can see who is burnt out without reading the gauge.
31. As an Alpha player, I want a CUSTOM COMBO call-out, with the gauge draining as the timer runs, so that I can see how long the combo has left.
32. As an Alpha player, I want an ALPHA COUNTER call-out with a gauge flash, so that the counter reads clearly.
33. As a player in any overhaul fight, I want FIRST ATTACK on the round's first clean hit, so that the opening exchange is marked.
34. As a player in any overhaul fight, I want REVERSAL when an attack comes out on the first possible frame after blockstun or a get-up, so that I know my timing was tight.
35. As a player in any overhaul fight, I want PERFECT after a round won without taking damage, so that a flawless round is marked, with the 1987 round-end screens unchanged.
36. As a player, I want CPU sides to earn call-outs exactly as human sides do, so that Arcade fights read the same.

### Combo counter

37. As a player in any overhaul fight, SF1 and SF1+ included, I want a combo counter, so that I can see how many hits my combo landed.
38. As a player, I want the count to appear at the second hit and clear shortly after the combo ends, so that single hits don't flash a count.
39. As a player, I want the count as a large white number over HITS in gold at mid-height on the attacker's side, so that it reads at a glance.
40. As a player, I want a combo to mean every hit that lands while the opponent is still in hitstun from the one before, so that the count is honest.
41. As a learner in Training, I want the combo's damage shown under the count, so that I can compare routes.
42. As a learner in Training, I want a setting that keeps the last count on screen until the next combo starts, so that I can read it after practising.
43. As a player outside Training, I want the count without damage, so that the fight screen stays clean.

### No overlaps

44. As a player, I want no call-out, gauge, counter, TIME or overlay ever drawn over another, so that the screen stays readable when several moments land at once.
45. As a player, I want each side's call-outs to stack in one fixed column, with the combo count on top and words below in arrival order, so that a busy moment still reads top to bottom.
46. As a player, I want a full column to drop its oldest word and never the combo count, so that the most important number stays up.
47. As a player, I want P1's and P2's call-outs each in their own half of the screen, so that the two sides never collide.
48. As a player, I want two supers at once to show each move's name on its own side, so that both read.
49. As a player, I want the super dim to stay one level however many supers are running, and a burnt-out fighter to stay grey under it, so that effects combine without stacking.
50. As a learner in Training with the frame meter on, I want the gauge corners to move up and sit just above the meter, so that neither covers the other.
51. As a learner, I want the per-fighter telemetry boxes and the hitbox viewer kept clear of the era HUD's regions, so that the training tools stay readable.

### The announcer

52. As a player, I want an announcer to call the moments (super, the counters, PERFECT PARRY, BURNOUT, CUSTOM COMBO, ALPHA COUNTER, FIRST ATTACK, REVERSAL, PERFECT), so that the fight sounds alive.
53. As a player, I want him to call finished combos of five hits and up ("Five hit combo!"), so that big combos get their moment without every hit being shouted.
54. As a player, I want him silent during a combo, so that he never shouts over every hit.
55. As a player, I want the announcer to sound like the 1987 voice that says "ROUND 1" and "FIGHT!", so that a fight sounds like one announcer.
56. As a player, I want the 1987 voice lines left exactly as they are, so that the original announcer is still heard.
57. As a player, I want the new lines to sound like the cabinet (4-bit ADPCM at the hardware's 4 kHz), so that they don't sound cleaner than the 1987 board.
58. As a player, I want the announcer never to cut off a 1987 punch, hit sound or shout, so that the fight's own sounds are never lost to him.
59. As a player, I want him never to talk over himself, so that a busy moment doesn't become noise.
60. As a player, I want a higher-priority line to cut off a lower one, and a line that has waited more than about half a second to be dropped, so that he always calls what's happening now.
61. As a player, I want an ANNOUNCER volume from 0 to 10 on the SOUND tab, with 0 as off, so that I can set him like the other sounds.
62. As a player, I want him never to say move names, so that move names stay with the fighters' own voices.
63. As a player, I want CPU sides' moments called as a human side's are, so that Arcade sounds as alive as Local.

### Making the voice

64. As the developer, I want the announcer designed from a written description, never cloned from a person, so that the voice is original.
65. As the developer, I want every take encoded to the cabinet's ADPCM before it is judged, so that the pick is made on what the player will hear.
66. As the developer, I want two or three takes per line, judged by ear right after the ROM's "FIGHT!", so that the pick fits the 1987 voice.
67. As the developer, I want the line list, prompts and voice pipeline in the repository and the generated audio in a local voice pack beside the ROM set, backed up like the art and music packs, so that the public repository holds no generated audio.
68. As the developer, I want the ElevenLabs key read from the Windows user environment and never printed, logged or committed, so that the key stays secret.
69. As the developer, I want a re-run of the pipeline to voice only new or changed lines, so that regenerating costs nothing for lines already chosen.

### Developer and tests

70. As the developer, I want a dev preview that runs two era models under the new HUD, so that I can see every gauge and call-out before project 4 wires the eras into real fights.
71. As the developer, I want the HUD to read one small "what to show" description per side, so that each era only fills it in and project 4 can do the same for the real fighters.
72. As the developer, I want COUNTER and PUNISH COUNTER built and tested on fixtures, so that they are ready when project 4 emits counter hits.
73. As the developer, I want gauges to draw only what the era models already hold, so that no meter rules are invented here.
74. As the developer, I want the HUD to be presentation only, never touching the 1987 work RAM, text layer or sound program, so that gameplay and the fidelity checks are unaffected.

## Implementation Decisions

- **The kit is one shared presentation layer.**
  - It is drawn by the compositor in the overhaul present path, from a per-side description of what to show, plus a list of call-out events. This replaces the placeholder strip and grows the existing `EraHud`, which carried only cells, pips and stocks.
  - The faithful present path never reads it.
- **The description.** Per side it holds:
  - a main row, either stock cells (n of m) or a level bar (level and fraction);
  - an optional second row;
  - states: full, draining, greyed and flashing;
  - nothing at all when the side has no gauge.

  Call-out events last one frame and carry the earner's side and the moment's kind.
- **Who fills it:**
  - The six era models (SF2 ST, Alpha, SF3, SF4, SF5, SF6) fill the gauge rows and their era events, from state they already hold: SF5 `CrushCounter`; SF6 parry-perfect and Burnout; Alpha `AlphaCounter` and Custom Combo.
  - A new fight reader fills the combo counter, FIRST ATTACK, REVERSAL and PERFECT from the real 1987 fight's work RAM (hitstun, landed hits, the first actionable frame, health). It only reads.
- **No hit logic is added.** Nothing emits COUNTER or PUNISH COUNTER yet. Their drawing and announcer lines are exercised by fixture events until project 4 emits them.
- **Gauge art.** Ruled, Blocks and Plate are three drawings of the same description:
  - **Ruled:** lavender rules, gold fill and a dark shadow, after the PLAYER1 bar.
  - **Blocks:** bevelled boxes after the round markers, and a thin ruled line with a boxed level number.
  - **Plate:** a gold frame with a dark-gold shadow, and LV in PLAYER1 gold.

  All three use exact ROM HUD pens and ROM lettering (the 8×8 font, header tiles, or the large round-call lettering where it can spell the word), per ADR 0006. Serviceable-but-not-ideal pieces are listed in the art upgrades list.
- **The layout:**
  - **TIME.** The presenter hides the ROM's TIME tiles in the presented frame and draws TIME under the health bars from the ROM's timer value.
  - **Scores.** In Local and Training, the score tiles are hidden in the presented frame.
  - **Untouched.** The ROM's RAM and text layer are never written.
  - **Faithful 1987 preset.** It uses the plain 1987 presentation in every play mode.
- **Fixed screen regions.** These are the health block with TIME, each side's gauge corner, each side's call-out column, and each side's super move-name line. They are laid out not to intersect.
  - Each call-out column has a fixed number of slots. The combo count always takes the top slot, words fill in arrival order below it, and a full column drops its oldest word.
  - While the Training frame meter is on, the gauge corners move up to sit directly above it.
  - The super dim is a single level. Burnout grey applies to the fighter's palette and the gauge, under the dim.
- **Settings:**
  - VIDEO gains `GAUGE ART` (`RULED` / `BLOCKS` / `PLATE`, default `RULED`).
  - SOUND gains `ANNOUNCER` (0–10, default 10, 0 is off).
  - The pause box gains `GAUGE ART` beside `LOOK`, saving to Settings.
  - Training gains a row that holds the last combo count.
  - All of these live in `settings.toml`, never in the synced `MatchConfig`. An old `settings.toml` without the keys loads the defaults.
- **The announcer's chip (ADR 0011).** A third emulated MSM5205, mixed beside the board's two, plays the announcer's lines at 4 kHz. The samples CPU, its latch and its two chips run exactly as today.
  - **Queue.** The announcer plays one line at a time.
  - **Priority order, highest first:** super and PERFECT; Perfect Parry; the counters (COUNTER, PUNISH COUNTER, CRUSH COUNTER, ALPHA COUNTER); reversal; first attack; Burnout and Custom Combo; the combo count.
  - **Cut-off.** A higher-priority line cuts off a lower one. An equal or lower one waits, and a waiting line older than about half a second is dropped.
  - **Volume.** The ANNOUNCER volume scales his chip under MASTER.
- **ADPCM.** The existing asset-pack encoder and validator assume 8 kHz. They change to the hardware's 4 kHz, and existing callers are checked against the change.
- **Lines.** The announcer's lines are loaded from the local voice pack into an extended sample store used only in overhaul modes. A missing voice pack means no announcer, with a log line and nothing else.
- **The voice pipeline** follows the one used for Highfall:
  - an original ElevenLabs voice designed from the description in the design doc, tuned toward the 1987 announcer's character, never cloned;
  - takes archived with their prompts and settings;
  - a status command that costs nothing and a run that voices only new or changed lines;
  - an encode-to-ADPCM step before any listening.

  Spoken lines: "Super!", "Counter!", "Punish counter!", "Crush counter!", "Perfect parry!", "Burnout!", "Custom combo!", "Alpha counter!", "First attack!", "Reversal!", "Perfect!", and "<N> hit combo!" for N from five up to a builder-chosen cap.
- **Dev preview.** A development launcher view runs two era sessions under the new HUD, for any pair of eras and any gauge art.

## Testing Decisions

- **What a good test is.** Good tests check what a player sees and hears at the highest seam: frames out of the fight screen, frames out of the compositor, samples out of the mix. They don't check the kit's internal types. Golden frames are compared pixel for pixel. Tests that need the ROM set or local captures skip without them, as the existing front-end and fight tests do.
- **Seam 1: the front end's fight screen.** Real Local, Training and Arcade fights are driven by inputs, as the front-end Local and Training tests already do. They check:
  - TIME under the health bars and the scores hidden in Local and Training;
  - the score kept in Arcade;
  - the Faithful 1987 preset frame-identical to today's in every mode;
  - the faithful cabinet untouched;
  - the combo counter appearing at the second hit and clearing;
  - FIRST ATTACK, REVERSAL and PERFECT on scripted rounds;
  - Training's combo damage and hold-count row;
  - the gauge corners moving above the frame meter;
  - the GAUGE ART and ANNOUNCER rows round-tripping through `settings.toml` and the pause box;
  - old settings files loading with the defaults.
- **Seam 2: the compositor's present, fed an era-HUD scene.**
  - Golden frames cover every era's gauge shapes in all three gauge arts, mirrored for P2, plus the empty side.
  - Every call-out is checked in its column slot.
  - The super dim and the Burnout grey are checked.
  - The no-overlap stress test fires every event on both sides on one frame and over a burst of frames, in all three arts, with the Training overlay on and off. It asserts that no two drawn elements' boxes intersect.
  - Each era's existing session tests (`sf2_era`, `sfz_era`, `sf3_era`, `sf4_era`, `sf5_era`, `sf6_era`) grow checks that the era reports the right description and events.
- **Seam 3: the audio mix.**
  - The announcer's lines play on his own chip.
  - The samples CPU's port writes and the two original chips' output are identical with the announcer on and off. The existing fight-samples test is the prior art.
  - Priority cut-off, waiting and the half-second drop are checked.
  - Volume 0 is silent.
  - A missing voice pack is quiet.
  - Every voice-pack line validates as 4 kHz 4-bit MSM5205 ADPCM.
- **Prior art:**
  - the overhaul compositor tests (faithful present ignores the HUD; the HUD paints only its region);
  - the front-end Local, Training and Settings tests;
  - the frame meter and takeover tests;
  - the era session tests;
  - the fight-samples audio test.

## Out of Scope

- New era mechanics, the era styles acting on the real 1987 fighters, and counter-hit detection. These are project 4's.
- The character select screen's era display, the two-step era selector (O12).
- The 2v1 Dramatic Battle HUD (project 5), though it should reuse this kit.
- Fighters' own voices and move sounds for later-era moves.
- Unified Gauge mode's rules, beyond showing the same gauge in both corners.
- Any change to the 1987 voice lines, round-end screens, scoring or the faithful cabinet.
- Languages other than English for the announcer.

## Further Notes

- **Order.** This is the screen side of project 4 (era styles and gauges) and can run first within it. The combo counter and its round call-outs ship in real fights as soon as they land.
- **Mockups.** Gauge placement, the three gauge arts with every era, call-outs and the combo counter are local mockups in the brainstorm folder. The design doc names them.
- **Art compromises.** Any lettering that has to fall back from the ROM's large type to its 8×8 font goes on the art upgrades list.
- **The voice pack** follows the art pack's and music pack's rules: local and backed up.
