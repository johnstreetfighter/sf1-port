# Era HUD, call-outs and announcer: design

- **Date:** 2026-11-04
- **Project:** the screen side of project 4 (era styles and gauges) of the seven the user set on 2026-11-02 (front end; art pipeline; SF1+ on the real engine; **era styles and gauges**; endings, Dramatic Battle and World Tour; bonus challengers; netplay transport). It may run first within project 4 or just before it.
- **Mockups:** `.superpowers/brainstorm/19485-1793799909/content/` (local, gitignored): `placement-v2.html`, `bar-style.html`, `callouts.html`, `combo.html`.

## Intent

The overhaul brings in the resource systems of later entries: the ST Super stock, the Alpha gauge and Custom Combo, SF3 Super Arts, SF4 Super and Revenge, SF5 EX and V-Gauge, and SF6 Drive and Super Art. Today they have no proper face. The six era modules report their state as an `EraHud`, and the compositor paints it as a placeholder strip: a black band over the bottom 16 rows, flat red cells, yellow pips and a 3×5 "SF" stamp (`crates/sf-video/src/compositor.rs`, `blit_hud`). The front end keeps that strip out of fights on purpose (front-end spec story 79).

This project gives those systems an on-screen language that looks as if Capcom's 1987 artists had drawn one more element for the cabinet. That covers the gauges, the call-outs for the big moments, and an announcer who shouts them. The kit is shared by every era, so a fight between two different eras still looks like one game.

## Goals

1. **One invented 1987 style for every era** (user). Nothing in the fight names an era; players learn who is running what on the character select screen (user).
2. **A new fight layout for overhaul fights:**
   - era gauges in the bottom corners, P1 bottom-left and P2 mirrored bottom-right (user);
   - TIME moved from bottom-left to the centre, just under the PLAYER1 / PLAYER2 health bars (user);
   - no P1 / HI / P2 scores in Local and Training (user). Arcade keeps its score.
3. **Three kinds of gauge art, all available in Settings, A (Ruled) by default** (user):
   - **A, health-bar sibling:** the PLAYER1 bar's lavender rules and dark shadow, filled gold. Stocks are cells split by lavender rules, and second gauges are the same bar, thinner and in lavender.
   - **B, round-marker blocks:** stocks are bevelled boxes like the round-win markers, lit gold when full. Level gauges are a thin ruled line with the level in a box.
   - **C, gold plate:** the bar sits in a gold frame with a dark-gold shadow, with LV and the number in PLAYER1 gold.
4. **A combo counter, with FIRST ATTACK and REVERSAL call-outs, in every overhaul fight** (user). Nothing may ever overlap, however many moments land at once (user).
5. **Call-outs for every later-era moment, in 1987 style** (user). The set:
   - the super flash, COUNTER and PUNISH COUNTER;
   - PERFECT PARRY and BURNOUT;
   - the Custom Combo and ALPHA COUNTER.
6. **An announcer voices the call-outs, using a generated voice** (user): an original ElevenLabs voice designed from a text description, never cloned from a person.
7. **The six existing era modules drive the kit in this project**, through golden-frame tests and a dev preview that runs two era sessions under the new HUD. They are standalone combat models today: their `hud()` is read only by tests, and front-end fights carry no era state. The user decided on 2026-11-02 that overhaul mechanics must act on the real 1987 fighters, so this project does not hook those models into live fights. **Gauges and call-outs first appear in a real fight when project 4 moves each era onto the real fighters.** The new layout (TIME moved, scores hidden) ships in real fights at once.

## Non-goals

- **New era mechanics, or moving the era styles onto the real 1987 fighters.** That is project 4's work. This project draws whatever the era modules report.
- **The character select screen's era display.** The two-step era selector belongs to project 4 (overhaul ticket O12). This design only relies on players learning the era there.
- **The 2v1 Dramatic Battle HUD** (three fighters, a shared lifebar). That is project 5's, though it should be able to reuse this kit.
- **Fighters' own voices and move sounds for later-era moves.** The announcer calls the moments, and fighters shouting their move names is separate work.
- **Unified Gauge mode's rules.** Here it only means both corners show the same gauge kind.

## Anti-goals

- **Any change to the faithful cabinet**, or to an overhaul fight's gameplay, timing or RAM. The HUD is presentation only.
- **A gauge that looks like a later game's HUD dropped onto 1987.**
- **A screen freeze that eats directional or button inputs.** The overhaul design already rules this out, and the super flash must honour it.
- **A voice modelled on any real announcer or on a later Street Fighter announcer.**
- **A layout that shifts between overhaul presets.** TIME moves in every overhaul fight, even when Pure Footsies shows no gauges. The one exception is the **Faithful 1987 preset**: it keeps the 1987 frame exactly (TIME bottom-left, scores shown) in Local and Training too, because it is the untouched baseline (user, 2026-11-04). The faithful cabinet is untouched as always.

## Constraints

- **Resolution and colours:**
  - 384×224, with no scaling inside the picture.
  - HUD art is built from the 1987 HUD's own colours. Sampled from a Local fight frame (`docs/media/local-fight.png`), these are:
    - health bar: lavender rule `#9999ff`, red fill `#aa0000`, shadow `#444444`;
    - round-marker bevel: `#ccdddd` / `#99aaaa` / `#889999`;
    - PLAYER1 gold: `#cc9900`, dark `#885500`, light `#ffcc77`.
  - The builder should take exact pens from the ROM palette rather than from the screenshot.
- **ROM-only art (ADR 0006).** HUD lettering comes from the ROM's own type: its 8×8 font, its header tiles, or its large round-call lettering if it holds suitable glyphs. Bars and boxes are drawn from ROM HUD pens. No outside fonts or HUD art. Any serviceable-but-not-ideal piece goes on `docs/planning/art-upgrades.md`.
- **Fidelity.** TIME and the scores are drawn by the ROM onto its text layer. Moving TIME and dropping scores means the overhaul presenter hides those tiles and draws its own TIME from the same timer value. The ROM's RAM and text layer are left alone; only the presented frame changes.
- **Sound hardware.**
  - **The 1987 voices.** The ROM's voices play through the board's two MSM5205s, driven by the translated samples CPU (`crates/sf-audio/src/samples.rs`), at 4 kHz per channel. A new command cuts off whatever its channel is playing.
  - **The announcer's own chip.** The announcer plays on a third emulated MSM5205 of the same kind, mixed beside the original two (user, 2026-11-04). He never cuts off a 1987 hit or shout, and the 1987 sound program and its latch run exactly as today.
  - **Format.** Announcer lines are 4-bit MSM5205 ADPCM at the hardware's 4 kHz. *My call:* `crates/sf-video/src/asset_pack.rs`'s `encode_adpcm` / `validate_adpcm` assume 8 kHz, as do the overhaul tickets. They follow the hardware instead, and `sf6.rs`, which already calls `encode_adpcm`, is checked against the change.
- **Netplay.** Bar style and announcer on/off are local, per-player presentation. They must not enter the synced `MatchConfig`.

## Approaches

1. **One HUD kit now, filled in by era later (chosen).** One project builds the shared pieces:
   - the layout;
   - the three bar styles and their Settings row;
   - the call-out lettering, placement and screen effects;
   - the announcer, from designed voice to encoded line.

   The kit reads one small description of what to show, and the six existing era modules fill it in now. Project 4 keeps them filled in as the styles move onto the real fighters. *Trade-off:* it is built ahead of the full mechanics, so a few effects may need a touch-up once their era is real.
2. **Each era brings its own HUD as project 4 reaches it.** Nothing is built before its mechanic exists. *Trade-off:* the shared style gets designed piecemeal across many tickets, and the layout and announcer have no single owner. That is how one invented style drifts apart.
3. **Repaint the placeholder strip only.** Cheapest. *Trade-off:* no gauges in real fights, no call-outs and no announcer.

The user took the recommendation, approach 1, because one shared style needs one owner.

## What was decided

### The layout (overhaul fights only)

| Element | Where | Notes |
|---|---|---|
| Health bars, round markers | where 1987 puts them | unchanged |
| PLAYER1 / PLAYER2 / ENEMY labels | where 1987 puts them | each is the name of the fighter on that bar, in the label's lettering and colours (the maintainer, 2026-11-06); a name longer than seven letters ends where the label ends |
| The round result's RYU / KEN / ENEMY WON THE … ROUND | where 1987 types it, centred on the screen | the winner's own name with the rest of the line as the ROM types it, centred again, in its lettering and colours (the maintainer, 2026-11-07, found in the showcase recording) |
| TIME | centred, just under the health bars | every overhaul fight; drawn by the presenter from the ROM's timer |
| Scores | Arcade: as 1987. Local, Training: hidden | the ROM's tiles are hidden in the presented frame |
| Era gauges | P1 bottom-left, P2 bottom-right, mirrored | fills grow from the screen edge inward. Each side can run a different era |
| Call-outs | mid-height, on the side of the fighter who earned the moment | clear quickly |

### Gauge shapes

Two shapes cover every era (*my proposal, agreed*):

- **Stock gauges** are a segmented bar, one cell per stock: ST Super 1, SF5 EX 3, SF4 Super 4, SF6 Drive 6.
- **Level gauges** are one long bar with the level number beside it: Alpha, SF3 Super Art and SF6 Super Art. The SF3 bar's length follows the chosen Super Art.
- **Second gauges sit in a thinner row underneath:** SF4 Revenge, SF5 V-Gauge and SF6 Super Art under Drive. A corner never holds more than two rows.
- **Unified Gauge mode** shows the same gauge in both corners.

The `bar-style.html` mockup draws every era in all three styles.

### Call-outs and screen effects (*my proposal, agreed*)

| Moment | Word(s) | Screen | Announcer |
|---|---|---|---|
| Super starts | SUPER, and the move's name below it | the stage dims behind the fighter, with no input lost | "Super!" |
| Counter hit | COUNTER | none | "Counter!" |
| Punish counter | PUNISH COUNTER | none | "Punish counter!" |
| Perfect Parry | PERFECT PARRY | none | "Perfect parry!" |
| Burnout | BURNOUT | the fighter and their gauge grey out until it ends | "Burnout!" |
| Custom Combo | CUSTOM COMBO | the gauge drains as the timer runs | "Custom combo!" |
| Alpha Counter | ALPHA COUNTER | the gauge flashes | "Alpha counter!" |

| Crush Counter (SF5) | CRUSH COUNTER | none | "Crush counter!" |

*My calls:*

- **The words.** The spoken words follow the on-screen word. The announcer does not say move names: those belong to the fighters' own voices.
- **CRUSH COUNTER** is added for SF5, from its existing `CrushCounter` event.
- **Which eras' modules emit what today:**
  - SF5: `CrushCounter`;
  - SF6: `Parry { perfect }` and `burnout()`;
  - Alpha: `AlphaCounter` and `CustomCombo`.
- **Words with no event yet.** No module emits a counter hit or a punish counter. COUNTER and PUNISH COUNTER are built and tested on fixtures until project 4 emits them. This project adds no hit logic.
- **No invented meter rules.** Gauges draw only what the modules already hold. Alpha, SF3 and SF6 Super Art hold whole levels only, so they fill in whole steps for now. SF6 Drive (half bars), SF4 Revenge (0-200) and the SF5 V-meter already show partial fill.

### The combo counter

- **Every overhaul fight gets a combo counter** (user, 2026-11-04), SF1 and SF1+ included. The Faithful 1987 preset keeps the plain 1987 screen, as for the layout.
- **It reads the real fight and changes nothing.** The real 1987 fight already holds the fighters' hitstun state, so this works today, before project 4. It adds no hit logic and makes no change to RAM.
- **What counts** (*my proposal, agreed*): a combo is every hit that lands while the opponent is still in hitstun from the one before. The counter appears at the second hit and clears shortly after the combo ends.
- **Where it sits:** mid-height on the attacker's side, a large white number over HITS in gold. The other call-outs stack under it.
- **Training** gets a row that keeps the last count on screen until the next combo starts.
- 1987 fighters are invulnerable for their whole hit reaction, so a real SF1 fight has no true combos; the counter shows once a style gives combo-able hitstun (user, 2026-11-04).

**Two related call-outs** run in every overhaul fight, read from the real fight:

| Moment | Word | Announcer |
|---|---|---|
| The round's first clean hit | FIRST ATTACK | "First attack!" |
| An attack on the first possible frame after blockstun or a get-up | REVERSAL | "Reversal!" |
| A combo of five hits or more ends | the count stays up a moment | "Five hit combo!", with the number spoken (*my call* on the threshold) |

REVERSAL is literal: only out of true blockstun or a get-up; the 1987 fight has neither (normal blocks have no blockstun and every knockdown is a KO), so it shows once a style adds them (user, 2026-11-04).

The announcer stays quiet during a combo, because shouting every hit would be noise.

The mockup is `combo.html`.

### Nothing overlaps

Several of these moments can land on the same frame (user, 2026-11-04): a counter-hit reversal that starts a combo and ends in a super, while the other fighter burns out. No text or graphic may ever draw over another.

- **Fixed regions.** Each element has a screen region of its own: the health block with TIME under it, each side's gauge corner, each side's call-out column, and the super's move-name line. Regions are laid out so they never intersect.
- **One call-out column per side** (*my call*):
  - It has a fixed number of slots, top down. The combo counter always takes the top slot, and words fill the slots below in arrival order.
  - A new word that finds the column full replaces the oldest word, never the counter.
  - Each side's column lives in its own half of the screen, so P1's and P2's never meet.
- **The super's move name** has its own line under the earner's column. If both fighters super at once, each name stays on its own side.
- **Screen effects combine without stacking.** The super dim is one level, however many supers are running. A greyed Burnout fighter stays grey under the dim.
- **The Training overlay counts too.**
  - The frame meter panel fills the bottom 20 rows (`overlay.rs`, `draw_frame_meter`). *My call:* while it is on, the gauge corners move up to sit directly above it.
  - The per-fighter telemetry boxes and the hitbox viewer are checked against the same regions.
- **The announcer never overlaps himself.** He has one chip, so lines play one at a time:
  - A higher-priority line cuts off a lower one. The order, highest first: super, Perfect Parry, the counters, reversal, first attack, Burnout, the combo count.
  - An equal or lower-priority line waits.
  - A waiting line older than about half a second is dropped, so he never calls something long past. *My call* on the order and the timeout.
- **A stress test proves it.** A fixture fires every event on both sides on the same frame and over a burst of frames, in all three bar styles and with the Training overlay on and off. The test checks that no two drawn elements' boxes intersect and that the announcer's output never has two lines at once.

### The announcer

- **Description** the voice is designed from (*my proposal, agreed*). Grilling later tuned it toward the 1987 announcer's own voice, below:

  > A ringside announcer at a 1980s world fighting tournament. Adult man, strong mid-range voice, a little gravel, loud and bright. He barks single words and short phrases with hard consonants, as if he's calling the action over a crowd through a cheap PA. No breathiness, no long vowels, no commentary.

- **Generate, encode, then judge.** Each line gets two or three takes. Every take is downsampled and encoded to the cabinet's ADPCM before anyone judges it, because the voice chips cut everything above about 2 kHz. The user picks by ear from the encoded takes.
- **The 1987 game's own voice lines are unchanged.**
- **Where the lines live.** *My call:* the line list, prompts and pipeline are in the repository. The generated audio lives in a local voice pack beside the ROM set, like the art pack and the music pack, and is backed up the same way. `ELEVENLABS_API_KEY` is read from the Windows user environment and never printed, logged or committed.

### Settings

*My calls:*

- **`GAUGE ART`** goes on the VIDEO tab: `RULED` (A, the default), `BLOCKS` (B) or `PLATE` (C) (user, 2026-11-04, grilling). It is not "gauge style", because an era style is a fighter's era package.
- **`ANNOUNCER`** goes on the SOUND tab, a volume from 0 to 10 like `MASTER`, `FM` and `SAMPLES`. 0 is off and the default is 10 (user, grilling).
- Both are stored in `settings.toml`, not `match_config.toml`, so netplay never syncs them.

### Also settled in grilling (2026-11-04)

- **A side with no gauge shows nothing.** An SF1 or SF1+ fighter, or a Pure Footsies match, leaves that corner clear.
- **CPU sides are treated like human ones.** They earn combo counts, call-outs and announcer lines exactly as a human side does.
- **One announcer.** The 1987 ROM already says "ROUND 1" and "FIGHT!" in its own voice (`docs/reference/mame-baseline.md`). The new lines are designed to sit close to that voice's character, from a description and never cloned, so a fight sounds like one announcer. Every take is judged by ear right after the ROM's "FIGHT!". The 1987 lines stay.
- **The third voice chip** is ADR 0011.
- **The pause box gets a `GAUGE ART` row** beside `LOOK`. It saves as the Settings value, as `LOOK` does.
- **Training shows the combo's damage** under the hit count. Other fights show the count alone.
- **The announcer calls finished combos of five hits and up.**
- **While the Training frame meter is on, the gauge corners move up** to sit just above it.
- **PERFECT** shows on the winner's side after the knockout of a round won without taking damage, and the announcer calls it. It reads the real fight's health, so it works in every overhaul fight. The 1987 round-end screens are unchanged. In the announcer's priority order it sits with the super, at the top.

## Left to the builder

- **Where the code lives.** Suggested:
  - the kit's drawing in `sf-video` beside the compositor;
  - the description type and the call-out event list beside `EraHud`, which they replace or grow;
  - the era modules in `sf-engine` filling the description in.
- **The description's exact shape.** It needs to carry:
  - per side, a main row (stock cells n of m, or a level bar with level and fraction) and an optional second row;
  - states: full, draining, greyed and flashing;
  - one-frame call-out events with the side that earned them.
- **Call-out timing:** how long each word stays up, and how the dim fades in and out, matched to 1987's own text timings where one exists.
- **The ADPCM encoder.** Check whether `sf-audio` has one; the repository may only decode today. Also how announcer lines reach the samples CPU, likely an extended sample bank loaded only in overhaul modes, as the sprite and background banks are.
- **Reading combos, first attacks and reversals from the real fight:**
  - which work-RAM state marks hitstun, a landed hit and the first actionable frame after blockstun or a get-up;
  - the exact number of call-out slots per side;
  - the region coordinates.
- **Exact ROM pens and glyphs**, and whether the ROM's large lettering can spell every call-out word.
- **Tests:**
  - the faithful frame is untouched;
  - every era's description draws in all three styles (golden frames);
  - TIME and the scores in each play mode;
  - Settings round trips;
  - every encoded line meets the MSM5205 format.
