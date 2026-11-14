# Ryu’s standing normals in World Super Turbo 2

These captures use World `ssf2t` 940223 in MAME 0.289. New and Old Ryu face standing New Ken in a two-player match. Gap is the difference between the fighters’ integer arcade world X positions when Ryu enters the attack state. The far animation can be selected beyond its hit range. [MAME driver and ROM manifest](https://github.com/mamedev/mame/blob/mame0289/src/mame/capcom/cps2.cpp#L8152)

## Close and far selection

Each adjacent pair below was observed directly. Across 342 distinct trials, the recorded gap equaled the requested gap and each loadout and button selected one close and one far animation family. The addresses identify the first attack animation record written by the 68000 after Ryu enters state `10`.

| Standing input | New last close / first far | Old last close / first far | New close / far animation | Old close / far animation |
| --- | ---: | ---: | --- | --- |
| LP | 64 / 65 px | 64 / 65 px | `0x1100fe` / `0x110162` | `0x1a79f0` / `0x1a7a54` |
| MP | 70 / 71 px | 70 / 71 px | `0x1101c6` / `0x11028a` | `0x1a7ab8` / `0x1a7b7c` |
| HP | 80 / 81 px | 80 / 81 px | `0x11031e` / `0x1103b2` | `0x1a7c10` / `0x1a7ca4` |
| LK | 40 / 41 px | 40 / 41 px | `0x110446` / `0x1104da` | `0x1a7d38` / `0x1a7dcc` |
| MK | 40 / 41 px | 40 / 41 px | `0x110556` / `0x1105ea` | `0x1a7e48` / `0x1a7edc` |
| HK | 95 / 96 px | 79 / 80 px | `0x110666` / `0x1106fa` | `0x1a7f58` / `0x1a7fec` |

Both loadouts kept the same selection across all four sampled input phases and when facing left. A fraction of `65535` on the right-side fighter’s X kept each last-close sample close. For LK, additional fractions `1` and `32768` changed neither the 40-pixel close selection nor the 41-pixel far selection in either facing direction. The port uses its integer fighter axis, computed by subtracting the signed pose X offset from the sprite-grid center. The standing Ryu and Ken poses have zero X offset, so the two quantities agree in the standing test setup. [Port fighter geometry](../../crates/sf-engine/src/fighter.rs)

## Contact and timing samples

Ticks below are arcade game ticks relative to the state-10 attack start write. They include contact freeze where present. The separate pacing comparison reports observed display frames for far LP. [Era pacing report](2026-11-13-ryu-era-pacing.md)

| Move and setup | New Ryu | Old Ryu |
| --- | --- | --- |
| Close LP at 64 px | First contact `+3`, neutral `+26` | First contact `+3`, neutral `+26` |
| Far LP whiff at 80 px | Pose writes `+1, +3, +7, +11`, neutral `+12` | Same |
| Close MP at 70 px | First contact `+4`, neutral `+37` | Same |
| Close MP whiff at 70 px | Pose writes `+1, +2, +4, +6, +12, +16, +19, +22`, neutral `+23` | Same |
| Far MP whiff at 112 px | Pose writes `+1, +2, +4, +8, +11, +14`, neutral `+15` | Same |
| Close HP at 80 px | First contact `+4`, neutral `+49` | Same |
| Close HP whiff at 80 px | Pose writes `+1, +4, +6, +12, +22, +34`, neutral `+35` | Same |
| Far HP whiff at 112 px | Pose writes `+1, +4, +6, +12, +22, +34`, neutral `+35` | Same |
| Close MK at 40 px | First contact `+4`, neutral `+33` | Same |
| Far MK whiff at 72 px | Pose writes `+1, +4, +8, +16, +22`, neutral `+23` | Pose writes `+1, +6, +12, +24, +30`, neutral `+31` |
| Far MK contact at 41–64 px | First contact `+8` | First contact `+12` |
| Close LK at 40 px | First contact `+6`, neutral `+30` | Same |
| Far LK whiff at 72 px | Pose writes `+1, +3, +5, +13, +17`, neutral `+18` | Pose writes `+1, +4, +7, +15, +19`, neutral `+20` |
| Close HK in tested setups | Both hits connect at 31 px, at `+8` and `+30`. One contact at `+8` at the sampled 32–80 px gaps. Only the second strike hit at each integer gap from 82 to 95 px, at `+16`. It dealt 4–6 arcade HP across repeated inputs at 95 px. | Both hits connect at 31 px, at `+8` and `+30`. One contact at `+8` at the sampled 32–79 px gaps. |
| Close HK whiff at 74 or 79 px | Pose writes `+1, +4, +8, +16, +20, +30`, neutral `+31` | Same |
| Far HK at first far distance | Contact `+7` at 96 px | Contact `+7` at 80 px |
| Far HK whiff at 112 px | Pose writes `+1, +3, +7, +15, +25, +31`, neutral `+32` | Same |

The earlier World 940223 audit directly observed close LK defeating standing guard at 32 px for both loadouts and far LK being guarded at 57–61 px. Those guard runs used World Turbo 1. A separate 32-trial Turbo 2 guard sweep used both loadouts and two input phases. At 32 and 40 px, close LK hit standing guard and crouching guard blocked it. At 41 and 60 px, standing guard blocked far LK and far LK missed crouching Ken in the recorded setup, with no damage or freeze. The port’s authored far LK box reproduces those results with Ken at the four measured gaps. The published Old LK diagram lists standing guard for the close kick, while both World settings recorded a standing-guard state followed by damage. [World trace audit at `9646b61`](https://github.com/johnstreetfighter/sf1-port/issues/139) · [Published Old Ryu hitbox capture](https://doctorguile.github.io/ssf2st/old-chars/ryu/#close-lk)

## Grounded contact freeze and displacement

An additional 180 Turbo 2 trials covered both loadouts, two input phases, and standing neutral, Back, and Down-Back. The 180 contacts across those trials loaded `14` in both fighters’ freeze counters. Ryu’s counter reached zero at contact `+14`. On the first or only contact Ken’s reached zero at `+15`, and his first recoil step occurred at `+16`. Ticks are relative to the contact write in this section unless a move-start tick is specified.

| Buttons, either standing distance | Ken’s displacement on consecutive game ticks | Total | Ken exits reaction |
| --- | --- | ---: | ---: |
| LP, LK | 4, 4, 4, 4, 3, 3, 3, 2, 1, 1, 1 | 30 px | +27 |
| MP, MK | 6, 6, 6, 5, 5, 5, 4, 4, 3, 3, 2, 1, 1, 1, 1, 1 | 54 px | +32 |
| HP, HK | 8, 8, 8, 7, 7, 7, 6, 5, 5, 4, 3, 2, 1, 1, 1, 1, 1, 1, 1, 1 | 78 px | +36 |

Clean hits and blocks used the same curves. The light and medium curves match the earlier crouching LK and MK captures. Ryu’s X stayed fixed during the sampled reactions. When close LK hit standing Back at 32 px, Ken moved an additional 2 px on the exit tick as he resumed walking. His reaction itself still used the 30 px light curve. [Crouching kick displacement](ryu-st-turbo2-contact-evidence.md)

| Move | Sampled gap | Standing neutral | Back | Down-Back |
| --- | ---: | --- | --- | --- |
| Close LP | 32 px | Hit | Block | Whiff |
| Close MP, HP, MK | 32 px | Hit | Block | Block |
| Close LK | 32 px | Hit | Hit | Block |
| Close HK | 31 px | Two hits | Two blocks | Two blocks |
| Close HK | 32, 40 px | First strike only | Two blocks | Two blocks |
| Close HK | 95 px New | Second strike only | Second strike blocks | Second strike blocks |
| Close HK | 79 px Old | First strike only | First strike blocks | First strike blocks |
| Far LP, MP, HP | 65, 71, 81 px respectively | Hit | Block | Whiff |
| Far LK, MK | 41 px | Hit | Block | Whiff |
| Far HK | 96 px New, 80 px Old | Hit | Block | Whiff |

Settled crouching Ken with Down alone also avoided close LP at 31, 32, 40, and 64 px and far LP at 65 and 66 px in both loadouts and input phases. Far LP hit at 65 px when Ken began crouching with Down alone during Ryu’s attack startup. Crouch entry and the settled crouching pose have different hurtboxes.

### Ken’s standing and crouching geometry

At the first active pose of close and far LP, neutral standing New Ken and active standing guard use the same rectangles in the sampled setups. Settled crouching Ken and active crouch guard also share their rectangles. X offsets below are for Ken facing left, toward Ryu. Y is measured from the floor, and both radii are in arcade pixels.

| Ken pose | Head center and radii | Torso center and radii | Feet center and radii |
| --- | --- | --- | --- |
| Neutral standing, standing guard | (−4, 79), 12×8 | (+6, 52), 20×21 | (+6, 14), 20×16 |
| Settled crouching, crouching guard | (−14, 51), 12×9 | (−6, 33), 22×12 | (−6, 11), 22×12 |

| Ryu pose | Attack ID | Center forward and above the floor | Radii |
| --- | ---: | --- | --- |
| Close LP | 1 | (33, 85) | 19×8 |
| New far LP | 2 | (35, 69) | 25×3 |
| Old far LP | 2 | (35, 75) | 25×9 |
| Close LK | 14 | (52, 21) | 33×21 |
| Far LK | 15 | (16, 83) | 33×14 |
| Close MK | 17 | (53, 57) | 20×21 |
| Far MK | 16 | (16, 83) | 33×14 |

The two far LP boxes share a lower edge at 66 px. Ken’s settled crouch head tops out at 60 px. The standing head and torso reach 87 and 73 px respectively. These first-active geometry captures are in `st-standing-lp-lk-mk-geometry`. [ST hitbox table layout](https://github.com/peon2/fbneo-training-mode/blob/master/hitboxes/st-hitboxes.lua)

### Close HK follow-up

For both loadouts, the first strike occurs at move start `+8` in the 31, 32, and 40 px setups. Strike two begins at move start `+30`, 22 ticks after the first contact. On that tick Ken takes the seventh recoil step (`+6`) before the collision check, bringing his first-reaction displacement to 51 px. A second contact reloads both counters with `14`. Since Ken is already in reaction, both counters reach zero at `+14` after the second contact, Ken starts a fresh heavy curve at `+15`, and he exits reaction at `+35`. A two-contact sequence therefore displaces Ken 129 px before any recovered movement. The second clean strike deals less damage and still uses the full heavy curve. The source combo counter changes from `0` to `1` on the tick after a second clean hit and stays `0` on two blocks.

Ryu’s axis is X 552 in these samples. Strike two uses attack box `21`, with X 581–647 and Y 89–125 for New and Old Ryu. The rectangles below use the same world coordinate system and Ken’s X after his seventh recoil step.

| First strike setup | Ken’s axis at strike two | Ken’s relevant box | Result |
| --- | ---: | --- | --- |
| Clean at 31 px | 634 | Torso `17`, X 647–687, Y 64–110 | Inclusive edge touch at X 647 |
| Clean at 32 px | 635 | Torso `17`, X 648–688, Y 64–110 | One-pixel horizontal separation |
| Standing block at 40 px | 643 | Torso `1`, X 629–669, Y 71–113 | Overlap and second block |
| Crouching block at 40 px | 643 | Head `14`, X 617–641, Y 82–100 | Overlap and second block |
| Standing block at 60 px | 663 | Head `1`, X 647–671, Y 111–127 | Inclusive edge touch at X 647 |
| Standing block at 61 px | 664 | Head `1`, X 648–672, Y 111–127 | One-pixel horizontal separation |
| Crouching block at 70 px | 673 | Head `14`, X 647–671, Y 82–100 | Inclusive edge touch at X 647 |
| Crouching block at 71 px | 674 | Head `14`, X 648–672, Y 82–100 | One-pixel horizontal separation |

At clean gap 31 the other Ken rectangles are head `24` (X 666–686, Y 106–126) and feet `16` (X 627–667, Y 38–76). The head is beyond the kick and the feet are below it. At clean gap 32 both shift one pixel farther right and retain the same heights. The measured rehit boundaries at the second strike are 82 px for clean hit reaction, 111 px for standing block reaction, and 121 px for crouching block reaction. The standing-neutral second-strike-only case at New Ryu’s 95 px close threshold uses Ken’s neutral pose.

Both Ryu loadouts use pushbox `1` in neutral and all six close HK poses. Standing New Ken also uses pushbox `1`. Both rectangles have zero horizontal offset, horizontal radius 16, vertical offset 40 and vertical radius 39 in arcade coordinates. Across 24 trials with both facing directions, injected gaps 28 and 30 settled to 32, while 29 and 31 settled to 31. Gaps 32 and 33 stayed unchanged. The fighters kept their settled X and zero X fractions through the first strike. A settled gap of 31 therefore allows one pixel of nominal pushbox overlap, stays fixed through windup, and gives two clean hits. A gap of 32 gives one clean hit. The `st-hk-pushboxes` replay uses the neutral HK recipe and records pushbox ID at animation `+0x0d`, with table offset `+0x08` and four-byte entries. [ST hitbox table layout](https://github.com/peon2/fbneo-training-mode/blob/master/hitboxes/st-hitboxes.lua)

## Replay

Start from the character-select input recipe and default EEPROM described in the World Turbo 2 contact capture. At frame-end sample 2070 set speed selector `0xff83a9` to `2` and skip threshold `0xff8cd3` to `0x60`. Set Ryu’s world X to `552` and Ken’s X to `552 + gap`; clear both horizontal fractions. For a mirrored case, put Ken at `552` and Ryu at `552 + gap`. For a fractional case, set the fraction of the right-side fighter. Leave both fighters neutral until the attack. [Turbo 2 capture setup](ryu-st-turbo2-contact-evidence.md)

Queue the neutral standing button on frame-end samples `N…N+2`, with `N` from `2120` through `2123`. At the attack state write, record both fighters’ world X and fractions, loadouts, and Ryu’s facing. Capture both words of the animation pointer at `Ryu + 0x1a` and `Ryu + 0x1c`, the state and substate at `+0x03/+0x04`, and Ken’s life at `Ken + 0x2a`. Timestamp writes with the game tick at `0xff801d`. The state-10 write is the origin. Deduplicate repeated animation writes and repeated life values. The sweep used 32, 40, 48, 56, 64, 72, 80, 96, and 112 px as initial gaps and tested each integer in the resulting transition brackets.

The local captures are under `%TEMP%/ssf2-ryu-audit/st-standing-proximity`. The tables `coarse.tsv`, `lk-refine.tsv`, `other-refine.tsv`, and `last-refine.tsv` retain the threshold sweeps. The `*-controls.tsv` tables record the repeats. Per-case `events.txt`, `writes.tsv`, and `trace.tsv` retain the inputs and observations. The guard sweep under `st-standing-guard` holds Ken’s Back or Down-Back on `N+1…N+30` and uses `N=2120` or `2121`.

For the close whiff timing controls, the `st-standing-whiff` capture set held Ken at arcade Y 160 or 250 from sample 2115. Each setup selected the same close move as the grounded control at the same X, with no contact. The `st-standing-boxes` cases additionally resolved attack IDs and rectangles from each pose’s animation pointer and the live hitbox table. [ST hitbox table layout](https://github.com/peon2/fbneo-training-mode/blob/master/hitboxes/st-hitboxes.lua)

The first and second close HK strike records use attack IDs `20` and `21`. New and Old use the same active rectangles for close and far MP, HP, and HK. Close HP and far HK change rectangles during their active phases. The port uses SF1 poses and adapts the second close HK and far HK horizontal radii for its smaller Ken hurtbox.

The grounded contact sweep is under `st-standing-contact` and holds Ken’s Back or Down-Back on `N+1…N+75`, with `N=2120` or `2121`. Add write taps for both freeze counters at `fighter + 0x47` and both integer X words at `fighter + 0x06`. Record Ken’s previous state, combo byte at `+0x1f4`, and both fighters’ old X and freeze values in each tap. Count a contact when the defender counter reloads to `14`. Preserve write order when an X update and contact share a tick. To get the second-strike boxes, the `st-standing-contact-boxes` cases also record the live rectangles from Ken’s current animation and the second-strike record at `0x1106ae` (New) or `0x1a7fa0` (Old), at the attack pose write, X writes, and freeze writes.

The additional `st-standing-guard-edges` runs use the same recipe for close HK against guards near the two-contact boundaries and LP against Down alone during crouch entry. The `st-standing-settled-crouch` controls hold Down on `N−8…N+75` and confirm Ken is already in state `2` before the jab.

## Port comparison in v1.5.10

New and Old Ryu's close and far standing attacks now use the light, medium, or heavy contact freeze and recoil above. Local Versus controller tests exercise clean hits, the standing and crouching guard matrix, and the 31/32 px clean, 60/61 px standing guard, and 70/71 px crouching guard close HK edges. The clean two-hit sequence connects at move ticks 8 and 30, with 51 px of recoil before the second contact and 129 px in total. The rollback test restores the first reaction and replays into the same state.

The port still draws its SF1 poses. For the measured New Ken matchup, standing LP, LK, and MK use the source strike and Ken hurtbox rectangles for contact. Close HK uses the measured axis-centered separation and the Ken reaction reaches above. The contact profile uses the existing mixed-era defense paths. Exact standing-normal cancel boundaries, damage rolls, and the corresponding hitbox and reaction comparisons for other opponents remain open. [Local Versus regressions](../../crates/sf-engine/tests/sf2_native.rs)
