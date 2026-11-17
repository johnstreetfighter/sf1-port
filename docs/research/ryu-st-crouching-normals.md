# Ryu’s crouching punches and sweep in World Super Turbo 2

These captures use World `ssf2t` 940223 at Turbo 2 in MAME 0.289. New and Old Ryu face New Ken on Ken’s dock stage in a two-player match. The earlier LK and MK captures are the reference for their grounded reactions and LP Hadoken routes. This set covers the remaining crouching normals. [Spec #139](https://github.com/johnstreetfighter/sf1-port/issues/139) · [Crouching LK and MK evidence](ryu-st-turbo2-contact-evidence.md) · [MAME source manifest](https://github.com/mamedev/mame/blob/mame0289/src/mame/capcom/cps2.cpp#L8152)

## Startup, active boxes and recovery

Move ticks begin at Ryu’s write from crouch state `2` to attack state `10`. The 170 px whiff runs isolate the animation, and the 32 px contact runs start against settled standing Ken. Both loadouts use the same phase timing and attack rectangles. Rectangle centers are measured forward from Ryu’s axis and up from his floor, with horizontal and vertical radii in arcade world pixels.

| Move | New / Old first pose | Whiff pose writes | Active box, center X/Y and radii | Ryu exits attack, whiff / contact |
| --- | --- | --- | --- | --- |
| 2LP | `0x11078e` / `0x1a8080` | +1, +3, +7, +11 | ID 7, +39/+46, 31×7, on +3 through +6 | +12 / +26 |
| 2MP | `0x1107f2` / `0x1a80e4` | +1, +2, +4, +8, +11, +14 | ID 8, +44/+42, 36×11, on +4 through +7 | +15 / +29 |
| 2HP | `0x110886` / `0x1a8178` | +1, +4, +7, +15, +25, +37 | ID 9, +37/+68, 28×20, on +4 through +6. ID 10, +34/+85, 21×30, on +7 through +14. | +38 / +52 |
| 2HK | `0x110a12` / `0x1a8304` | +1, +4, +10, +16, +24, +34 | ID 24, +58/+9, 43×12, on +4 through +9 | +35 / +49 |

The contact values use the first transition out of attack state. With neutral input, the state machine can pass through the crouch exit state before standing neutral. [ST hitbox table layout](https://github.com/peon2/fbneo-training-mode/blob/master/hitboxes/st-hitboxes.lua)

## Hit and guard

At 32 px both loadouts contact neutral Ken at +3 for 2LP and +4 for the other three. They also connect at the same timings against Ken with settled Down alone. Each hit or block loads both freeze counters with `14`. Ryu’s counter reaches zero at contact +14, and Ken’s reaches zero at +15.

| Move | Back | Down-Back | Clean or block recoil | Ken exits reaction, from contact |
| --- | --- | --- | --- | --- |
| 2LP | Block | Block | Light, 30 px in 11 ticks | +27 |
| 2MP | Block | Block | Medium, 54 px in 16 ticks | +32 |
| 2HP | Block | Block | Heavy, 78 px in 20 ticks | +36 |
| 2HK | Low hit and sweep knockdown | Block | 80 px on hit. Heavy 78 px on block. | +95 on hit, +36 on block |

The ordinary clean-hit and block curves start at contact +16 and match the corresponding light, medium and heavy standing curves. On sweep hit, Ken enters hit reaction at +1 and begins airborne motion at +17, gaining 5 integer Y pixels on that first step. The peak is 55 px above the floor. Ken moves 2 px horizontally per game tick for 40 ticks from +17 through +56, touching down on +56. He exits the knockdown reaction at +95. These values also hold when the sweep hits standing guard and when it hits settled crouching Ken. [Standing contact curves](ryu-st-standing-proximity.md#grounded-contact-freeze-and-displacement)

The two neutral samples at 32 px started with 144 HP. Damage in the two sampled phases was New 3/4 and Old 6/4 for LP, 19/19 for either MP, New 24/24 and Old 24/27 for HP, and New 22/22 and Old 26/28 for sweep. These are observed rolls. The damage calculation and complete range remain to be verified.

## LP Hadoken cancel samples

At 32 px, the input, special-start and projectile writes distinguish a cancel from a late fireball and from a repeated jab. These measurements use the normal’s first contact as +0. A combo is recorded when the next projectile contact increments Ken’s combo counter. Several Old Ryu samples at the same final-input tick differed by display phase. The table gives the latest observed successes and earliest tested later failures, with mixed results shown at the affected boundary.

| Normal | New Ryu, clean hit | Old Ryu, clean hit | On crouch block |
| --- | --- | --- | --- |
| 2LP | +15 starts Hadoken and combos at +26. +16 starts Hadoken and hits at +27 after the combo window. | +16 starts Hadoken and combos at +26. +17 produced a late fireball in one replay and a repeated jab in another. | Both can produce fireball chip. The light-normal sequence can also produce a late shot. |
| 2MP | +16 starts Hadoken and combos at +27. +17 gave no shot. | +16 starts Hadoken and combos at +26 in one phase, with no shot in another. +17 gave no shot. | +16 starts and chips at +27 New / +26 Old. +17 gave no shot. |
| 2HP | +16 starts Hadoken and combos at +27. +17 gave no shot. | +15 starts Hadoken and combos at +25 in one phase, with no shot in another. +16 gave no shot in the clean samples. | +16 starts and chips at +27 New / +26 Old. +17 gave no shot. |
| 2HK | +16 starts Hadoken and the projectile goes live at +27. +17 gave no shot. | +16 starts Hadoken with a live projectile at +26 in one phase, with no shot in another. +17 gave no shot. | +16 starts and chips at +27 New / +26 Old. +17 gave no shot. |

On a clean sweep, the verified LP Hadoken activations in the sampled window produced a projectile without a second contact. The guard sequence confirms that the same sweep can cancel and the projectile can connect in a grounded block reaction. At 170 px, the tested HP and HK whiffs did not produce a shot. LP and MP whiff controls could show an intervening normal-state restart from the light input, so they do not establish a simple uninterrupted whiff cancel window.

### First-frame Hadoken collision

Six repeat captures at 32 px resolved the ordinary LP Hadoken’s first projectile box and Ken’s live hurtboxes at damage. Ryu stays at world X 552. The first active projectile is at X 645 with zero fraction and Y 94, or 54 px above the floor. Animation `0x72246` selects attack ID 4, with center offset X −16, Y 0 and radii 31×13. Its world rectangle is X 598–660, Y 81–107. Each shot below hits on its first active tick.

| Opener | New / Old contact tick | New / Old Ken axis X | Ken’s head, torso and feet at impact, relative to axis and floor |
| --- | --- | --- | --- |
| 2LP | +26 / +26 | 614 / 614 | Both head ID 27 at (+1,75), radius 14×9, torso ID 19 at (+10,47), 21×21, feet ID 18 at (+9,16), 21×17 |
| 2MP | +27 / +26 | 634 / 633 | Both head ID 28 at (+3,72), radius 14×9, torso ID 20 at (+14,46), 19×18, feet ID 19 at (+13,16), 21×17 |
| New 2HP | +27 | 654 | Head ID 31 at (+8,81), radius 14×9, torso ID 22 at (+9,51), 21×20, feet ID 21 at (+3,18), 23×18 |
| Old 2HP | +25 | 649 | Head ID 30 at (+14,57), radius 14×9, torso ID 21 at (+22,43), 19×15, feet ID 20 at (+19,16), 21×17 |

Ken faces left, and his floor is world Y 40. In New 2HP, the +25 and +26 recoil steps bring Ken to X 649 and 652 with the same hurtboxes as Old’s +25 sample. On New’s +27 tick, Ken moves to X 654 and changes to hurtbox IDs 31, 22 and 21 before projectile collision. The active torso spans X 642–684 and Y 71–111. In Old’s +25 sample, Ken moves to X 649 before collision and his head and torso both overlap the projectile. The raw six geometry repeats are under `%TEMP%/ssf2-ryu-audit/st-lp-hadoken-crouch-combo-box`. For each pose, the hitbox decoder reads animation offsets `+0x08/+0x09/+0x0a` for the defender and `+0x0c` for the projectile, then uses the fighter’s box table pointer at `+0x34`. [ST hitbox table layout](https://github.com/peon2/fbneo-training-mode/blob/master/hitboxes/st-hitboxes.lua)

Four neutral launches at 232 px give the same first-flight transition across both loadouts and two input phases. Attack ID 4 is active at ages 0, 1 and 2, with projectile X 645, 648 and 651. At age 3 and X 654, animation `0x7225e` switches to ID 7 with center offset (0,0) and radii 15×14. The next animation at age 6 (`0x72276`) retains ID 7. Y stays 94 and X advances 3 px per tick. Reproduce with Ken at X 784 and one held LP Hadoken motion at `N = 2120` or `2121`. Hold Down on `N−8…N−5`, Down-Forward on `N−4…N−1`, and Forward+LP on `N…N+4`. The geometry rows are under `st-lp-hadoken-idle-box`.

## Port comparison in v1.5.11

Both loadouts use the recorded whiff timings, mid or low guard class, 14-tick contact freeze, and grounded recoil profiles. Against New Ken, the port sweep covers 80 px, reaches 55 px above the floor, lands at contact +56, and returns Ken to neutral at +95. The port uses SF1 poses and the measured New Ken sweep trajectory. Other defenders use their ordinary knockdown path with the source freeze.

The ordinary LP Hadoken now starts 93 px ahead of Ryu's axis, travels 3 px per tick, and uses the measured initial and flight rectangles. The Ken collision test uses the sampled pose rectangles. The controller regression verifies source-matched projectile spawn and contact on the recorded crouching punch routes, and a sweep cancel that spawns a projectile without hitting the knocked-down Ken. The 60 Hz port uses the recorded successful boundary where arcade input phase can change the outcome. The full source damage rolls and exact phase-sensitive cancel rules remain open. [Port controller regressions](../../crates/sf-engine/tests/st_ryu_grounded.rs)

## Replay

Use the World Turbo 2 setup from the earlier contact captures. At frame-end sample 2070 use selector `0xff83a9 = 2`, skip threshold `0xff8cd3 = 0x60`, Ryu X 552 and Ken X `552 + gap` with zero X fractions. Run `N = 2120` and `2121`. Queue Down for Ryu from `N−6` through `N+2` and the attack button from `N` through `N+2`. Ken holds Back or Down-Back on `N+1…N+90` for the guard runs and Down alone on `N−8…N+90` for settled crouching. Leave both neutral after the inputs to observe each reaction’s endpoint. [Previous setup and character select](ryu-st-turbo2-contact-evidence.md#replay-setup)

The scratch captures under `%TEMP%/ssf2-ryu-audit/st-crouching-contact` retain `events.txt`, `trace.tsv`, `writes.tsv` and `boxes.tsv`. The 80 normal-only trials cover both loadouts, four buttons, two input phases, four defender states at 32 px and a 170 px whiff control. Read the game tick byte at `0xff801d` on each write. In addition to the previous freeze, state and X taps, record Ken’s Y word at `Ken + 0x0a` and the current animation pointer. The floor in this stage setup is world Y 40.

For the cancel samples, replay the normal from `N = 2120…2123`. At `N+3+D` begin one display sample each of Down, Down-Forward and Forward+LP. The clean boundary sweep samples `D = 8…12` with other delays as controls. On crouch block, use `N = 2120` and `D = 7, 10, 11, 12, 13`. Tap Ryu’s current input word `+0x392` and record the first `17` with previous input `5`. Record the move-state write to `12`, projectile creation at the first `0x0101` object-header write across slots `0xff97a2 + i × 0xc0`, and the defender’s first life decrement and combo byte. A single-slot probe would miss these runs because the projectile uses slot 7. Keep the display phase and full input schedule when comparing repetitions.

| Move | New clean success / late input, (N, D) | Old clean success / late input, (N, D) |
| --- | --- | --- |
| 2LP | (2120, 9) / (2120, 10) | (2121, 9) / (2120, 10) |
| 2MP | (2120, 11) / (2121, 11) | (2120, 11) / (2121, 11) |
| 2HP | (2120, 11) / (2120, 12) | (2120, 10) / (2121, 11) |
| 2HK | (2120, 11) / (2121, 11) | (2120, 10) / (2120, 11) |

For Old 2MP, 2HP and 2HK, the (2121, 10) clean controls match the successful input tick of their respective rows, and each produces no shot. “Success” for 2HK refers to canceling into Hadoken.
