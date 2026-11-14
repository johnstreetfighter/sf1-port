# Ryu Red Hadoken in World Super Turbo 2

Arcade evidence for [issue #142](https://github.com/johnstreetfighter/sf1-port/issues/142), using World `ssf2t` 940223 and Turbo 2 in MAME 0.289. New and Old Ryu face grounded, standing New Ken. Source-world distances use the fighter and projectile object coordinates. [ROM and cabinet setup](ryu-st-turbo2-contact-evidence.md#replay-setup)

## Heavy Red Hadoken

Both loadouts deal one hit. In the fixed-position sweep, HP Red launches at 162 px and gives a grounded reaction at 163 px. Both measured outcomes held across the sampled input phases. The 162 and 163 px gaps are measured at special start, projectile spawn, and contact. In these static trials the three readings agree. The held back portion of the input moved Ryu away from Ken before activation.

The launch follows the projectile’s travel or animation phase. Teleporting only Ryu after the projectile had spawned crossed the measured spacing boundary without changing the reaction. Moving only Ken changed when the projectile reached him and changed the reaction.

| Change after spawn | Fighter gap at spawn → contact | Projectile travel at contact | New and Old result |
| --- | ---: | ---: | --- |
| None | 162 → 162 px | 40 px, 8 advances | Launch |
| None | 163 → 163 px | 45 px, 9 advances | Grounded |
| Ken moves 16 px toward Ryu | 167 → 151 px | 30 px, 6 advances | Launch |
| Ryu moves 16 px toward Ken | 167 → 151 px | 45 px, 9 advances | Grounded |
| Ken moves 16 px away from Ryu | 158 → 174 px | 50 px, 10 advances | Grounded |
| Ryu moves 16 px away from Ken | 158 → 174 px | 40 px, 8 advances | Launch |

In the phase-zero bracket replays, New's projectile spawned on game tick 226 and Old's on 231. At 162 px, the first life writes are ticks 234 and 239 respectively. At 163 px, the writes are ticks 235 and 240. All four contacts remove 19 of Ken's 144 HP in that phase. Ken's floor Y is 40 and his launch peaks at Y 97. His first neutral sample is 127 game ticks after the 162 px contact and 36 game ticks after the 163 px contact in the phase-zero bracket. Other HP cases in the sweep removed 18–21 HP.

### Projectile geometry

The HP projectile spawns 93 px forward of Ryu and 54 px above Ryu's ground position. The raw rightward spawn was X 635, Y 94 with Ryu at X 542, Y 40. Its X advances 5 px per projectile update and Y stays 94. Count projectile advances by displacement from the first position. The game clock can advance twice per display or advance without changing position after contact.

| Projectile advances after spawn | Attack box ID | Box center offset from projectile axis for a rightward shot | Horizontal / vertical radius |
| --- | ---: | ---: | ---: |
| 0–2 | 18 | −16 X, 0 Y | 31 / 13 px |
| 3–8 | 15 | 0 X, 0 Y | 15 / 14 px |
| 9 onward, through advance 25 | 12 | −3 X, 0 Y | 21 / 12 px |

The near projectile’s raw axis can be ahead of Ken. At the 42 px New Ryu contact, the first box spans X 588–650 and Ken's torso spans X 570–610. The two intersect despite the projectile axis being X 635. At the 162 px boundary, the first damage uses attack ID 15. Its right edge and Ken’s torso left edge both equal X 690, establishing inclusive edge contact in this fixture. The 163 px contact uses ID 12. The attack table entry derives from the projectile's animation byte `+0x0c` and its hitbox table pointer at `+0x34`. [ST hitbox object and record layout](https://github.com/peon2/fbneo-training-mode/blob/master/hitboxes/st-hitboxes.lua#L39)

At first HP Red contact, neutral standing Ken and standing Ken holding Back both used a torso centered 6 px behind his world axis and 52 px above the floor, with radii 20 and 21 px. The equivalent crouching and crouch-guard torso was 6 px forward and 33 px above the floor, with radii 22 and 12 px. Ken faces left in this fixture. Both guard postures took 4–5 chip damage in the 66 and 162 px trials and stayed grounded. At the same 162 px starting gap, crouching Ken could be reached at six projectile advances, compared with eight when standing.

A mirrored replay put Ryu on the right. Ken used the same raw torso data with the opposite facing byte. His world torso X offset changed from +6 to −6 when standing and from −6 to +6 when crouching. HP Red spawned 93 px to Ryu’s left, and its attack box offsets reversed in world X. At 162 px in the mirrored standing guard probe, the projectile box and Ken’s torso touched exactly at X 592. The mirrored crouching hit reached him at six advances and launched.

### Clash and punch-strength controls

In both loadouts, HP Red and Ken's ordinary LP Hadoken trade at the tested 190 and 230 px spawn gaps. Both projectile objects start with `0x0100` life and receive `0xffff` on clash. Both disappear, with both fighters at 144 HP. This verifies one HP Red versus one ordinary Hadoken collision independently of the single fighter contact.

LP and MP Red also made one fighter contact in the tested near and far cases. All three strengths share the same launch-phase endpoint at eight projectile advances. Initial X/Y and attack geometry at advances 0–8 match across strengths. The last column gives the late box's center X offset for a rightward projectile and both radii. Its center Y offset is zero.

| Strength | X per advance | Last launching / first grounded standing Ken gap | First grounded travel | Damage at first grounded gap | Late box offset X, radii X/Y |
| --- | ---: | ---: | ---: | ---: | --- |
| LP | 3 px | 146 / 147 px | 27 px | 16 HP | `0, 15/12` |
| MP | 4 px | 154 / 155 px | 36 px | 17 HP | `−1, 17/12` |
| HP | 5 px | 162 / 163 px | 45 px | 19 HP | `−3, 21/12` |

For all three strengths, the phase-zero first-grounded trials first show the projectile at game tick 226 for New and 231 for Old. The defender's first health write occurs nine ticks later, at 235 and 240. The first observed defender freeze counter is 14 in all six. The projectile’s three box IDs are 16/13/10 for LP, 17/14/11 for MP, and 18/15/12 for HP.

Ryu's freeze counter stays zero on the sampled clean contacts. On grounded hits, Ken's freeze counter first reaches zero at contact +15. His first horizontal move is +8 px at +16. He stays in his grounded reaction through +35, then returns to neutral at +36. One Old LP display first observed neutral at +37. The individual movement increments are `8, 8, 8, 7, 7, 7, 6, 5, 5, 4, 3, 2, 1, 1, 1, 1, 1, 1, 1, 1` for 78 px total.

### HP Red guard reaction

Standing guard and crouching guard both load 14 ticks of freeze. Ken's freeze counter reaches zero at contact +15. He takes the first 8 px push at +16, advances through the same 20-step sequence as the grounded clean hit, and finishes the 78 px push at +35. His block reaction leaves state `14` for state `8` at +36. The state `8` substate can differ with the held direction. The two loadouts and both replay phases agree at 66–68 px and 162–165 px contact gaps. The guard chip samples remove 4–5 HP.

For a first-action control at 66 px, release Ken's guard after display 2142. A one-display LP queued at 2155 fails for both loadouts and guard postures. The same LP queued at 2156 starts the normal on display 2157, observed at contact +36 for New and +37 for Old. The Old failure sample advances across +35 and +36, so its first displayed tick alone does not locate input consumption.

### HP Red clean launch, rebound, and wakeup

The source write taps agree across both loadouts at 66–68 px and in the launching 162 px trials. The floor is Y 40. Neither of these fixtures reached a wall. Contact +0 is the first life decrement.

| Time from contact | Ken's reaction |
| --- | --- |
| +1 | Enters state `14`, substate `2`. Y changes from 40 to 56 during freeze. |
| +15 | Freeze reaches zero and substate `4` begins. |
| +16…+53 | Advances 2 px away from Ryu each tick, for 76 px. The first moving sample is Y 60. Y reaches 97 at +30 and returns to the floor at +53. |
| +54…+57 | Holds X and floor Y. Substate changes from `6` to `8` at +57. |
| +58…+75 | Rebounds with alternating 2 and 3 px horizontal steps, for another 45 px. Y reaches 50 at +64. The first integer-floor sample is +74. At +75 the Y write falls below the floor, clamps back to 40, and enters landing substate `10`. |
| +77, +78, +79, +81, +83 | Slides 1 px per listed tick. Total displacement is 126 px. |
| +88, +98, +127 | Enters the down state, starts the wakeup pose, and returns to neutral, respectively. |

The integer Y trace at +16…+53 is `60, 65, 69, 73, 76, 80, 83, 85, 88, 90, 92, 94, 95, 96, 97, 97, 97, 97, 97, 96, 96, 94, 93, 91, 89, 87, 84, 81, 78, 75, 71, 67, 63, 58, 53, 48, 43, 40`. The rebound at +58…+75 is `42, 44, 46, 47, 48, 49, 50, 50, 50, 50, 49, 48, 47, 46, 44, 42, 40, 40`.

Phase-zero LP and MP Red at 66 px follow the same X and Y paths and reaction state changes as HP for both loadouts. The contacts deal 16, 17 and 19 HP respectively. In a close HP hit at 42–44 px, the projectile axis has already passed Ken, and he moves with the projectile’s travel direction. A separate position write moves Ryu across Ken after spawning a rightward shot. Ken also launches right in that case. The overlap causes pushbox correction and changes its measured travel distance, while the direction of each advancing step remains rightward.

### HP Red whiff recovery and first movement

Write taps pin the HP Red special-state write to 11 game ticks before the projectile's first X write. The first neutral write is 41 ticks after spawn, for 52 ticks from the start of the special. All four staggered whiff phases agreed for each loadout. Holding Back took the first 2 px step on the neutral tick in all eight whiffs and 16 near and far contact trials. The hit replays showed the same 41 game-counter ticks from spawn to the first step. The defender's contact freeze delayed the display sample reaching that same counter value by 12–15 callbacks.

The phase-zero New whiff writes start at tick 215, spawn at 226, and return to neutral at tick 11 after wraparound. Old writes the equivalent events at 220, 231 and 16. A one-display Back input queued after sample 2158 produces no displacement. Queuing it after 2159 moves Ryu back 2 px on sample 2160 for both loadouts. The same input after 2160 takes effect on sample 2161.

## Replay

Use the New or Old Ryu versus New Ken setup linked above. At display 2070, leave Ryu at X 552, put Ken at `552 + initial gap`, and clear both fractions. Use `N = 2120` for phase zero, and `2121…2123` for staggered replays. Hold each listed input through its inclusive end.

| Input | Frame-end samples |
| --- | --- |
| Back (`4`) | `N−16…N−13` |
| Down-back (`1`) | `N−12…N−9` |
| Down (`2`) | `N−8…N−5` |
| Down-forward (`3`) | `N−4…N−1` |
| Forward + punch (`6+P`) | `N…N+4` |

- Use initial gaps 136/137 for LP, 144/145 for MP, and 152/153 for HP in the phase-zero stationary boundary. For motion-origin controls, use 157 or 148 and change the chosen fighter's integer X by 16 px on sample 2131 after the projectile spawns. These two replays intentionally write one extra position. To check guard and crouch, use initial gaps 56 and 152, holding Ken Back, Down or Down-Back from 2125 through 2160. Ken stays idle in the stationary clean-hit probes.
- For the clash, use initial gaps 180 and 220. Queue Ken Down at 2104…2107, Down-Forward at 2108…2111, and Forward + LP at 2112…2116. Ryu uses HP Red with `N = 2120`.
- To reproduce the whiff, use initial gap 152 and queue Ken neutral jump from 2121 through 2155. Verify no life change. Queue Ryu Back from 2140 through 2185 for the held test, or on one sample at 2158, 2159 or 2160 for the phase-zero movement boundary. Keep Ken idle for the matching hit controls.
- Read all eight projectile slots starting at `0xff97a2`, with stride `0xc0`. A live slot begins with `0x0101`. Read X/Y at `+0x06/+0x0a`, animation at `+0x1a`, life at `+0x2a`, and hitbox data pointer at `+0x34`. Use a write tap on Ken's `+0x2a` and read `0xff801d` inside the tap to anchor the first health decrement to its exact game tick.

The raw files and runner scripts are in `%TEMP%/ssf2-ryu-audit` on the capture machine. `st-t2-red-*` folders contain individual `events.txt`, `trace.tsv`, and `writes.tsv` captures. The geometry, two-projectile, and reaction runs use `st-t2-red-frames.py`, `st-t2-red-clash-slots.py`, and `st-t2-red-reaction.py`. The latter adds write taps for Ken's X, Y, state, substate and freeze, in addition to the life word. Guard replays for exact blockstun hold Back or Down-Back through sample 2155. The same runner keeps Ken idle for the launch replays at initial gaps 56 and 152.
