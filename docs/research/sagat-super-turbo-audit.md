# Sagat in World Super Turbo at Turbo 2

## Reference and scope

The source is the World arcade set `ssf2t` (940223), measured in MAME 0.289 with speed byte `02` and skip-pattern byte `60`. The ROM archive SHA-256 is `3fd1c7fb30069586cb9ac7f473745d5074a0c56c28ac583c38bfe5fe9f3a7167`. The QSound HLE archive SHA-256 is `55feb9b70dacb7364f0310ed16cc9281b5b0a31318b9e2e0587ae06d15f95356`. P1 is New or Old Sagat and P2 is New Ken. [MAME set manifest](https://github.com/mamedev/mame/blob/mame0289/src/mame/capcom/cps2.cpp#L8152)

Counts use the arcade game tick at `$ff801d` modulo 256. Action entry is tick zero, at the Sagat state write to `0c` (special). Contact is the first defender life write. A display sample can contain two game ticks. The listed release and fighter state changes come from RAM write taps.

The companion aerial audit covers all six neutral and directional normals, both jump startup clocks, and late forward-jump contacts. [Aerial normal audit](sagat-super-turbo-air-normals.md)

## Release, movement and grounded recovery

All three punches perform High Tiger Shot and all three kicks perform Low Tiger Shot with `236`. Within each loadout and height, all three strengths share the same action timings. The numbers below are relative to special entry.

| Loadout | Shot | Projectile allocation | Special state ends | Neutral / first held walk | Allocation to special exit |
| --- | --- | ---: | ---: | ---: | ---: |
| New | High | +10 | +48 | +48 | 38 |
| Old | High | +10 | +48 | +48 | 38 |
| New | Low | +12 | +50 | +53 | 38 |
| Old | Low | +10 | +48 | +51 | 38 |

Low Shot exits special state into crouch (`02`) and takes three more game ticks to return to neutral with no further input or to displace horizontally with Back held. The extra two ticks for New Low Shot occur before release. First action animation pointers for High/Low are `15d12e`/`15d17a` for New and `1ee714`/`1ee760` for Old.

Both heights start at fighter X +104 in this facing. High starts 78 px above the floor and Low starts 38 px above it. Light, medium and heavy travel 3, 5 and 7 px per advancing game tick. Compute projectile age from its distance traveled when interpreting frame-end samples. A sample can capture allocation followed by the first travel step, or the reported game clock can advance through a skipped display interval.

| Projectile age | X offset | Y offset | Half-width | Half-height | High attack IDs | Low attack IDs |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| 0–1 | −25 | 0 | 34 | 5 | 4/5/6 | 10/11/12 |
| 2 onward through observed contact | 0 | 0 | 9 | 5 | 1/2/3 | 7/8/9 |

Against standing idle New Ken at initial axis gap 120, all six strengths contact at age zero for both loadouts. At 220, the game-tick intervals from allocation to contact are High LP 31, MP 19, HP 13 and Low LK 31, MK 19, HK 14. The corresponding contact projectile X values are High/Low light 749, medium 751, High heavy 747 and Low heavy 754.

## Grounded contact and guard

The posture fixtures use gap 120. All indicated contacts occur on the allocation tick. Results repeat for both loadouts and all three strengths.

| Defender input | High Tiger Shot | Low Tiger Shot |
| --- | --- | --- |
| Neutral standing | Clean hit | Clean hit |
| Standing Back | Standing block | Clean hit |
| Neutral Down | Whiff | Clean hit |
| Down+Back | Whiff | Crouching block |

The defender freeze write is 14 on clean hit and block, and Sagat's freeze remains zero. Ken's first recoil is contact +16 and the reaction ends at +36, giving 20 advancing recoil ticks. The exact integer X increments are `[8, 8, 8, 7, 7, 7, 6, 5, 5, 4, 3, 2, 1, 1, 1, 1, 1, 1, 1, 1]`, totaling 78 px. The held guard resumes in state `08`. Recorded clean damage varies across source phases and defender postures. High block chip is 4 except one New HP sample at 5, while Low LK/MK/HK chip is 3/4/4. [Case write summaries](sagat-super-turbo-tiger-shot-writes.tsv)

## Reproduction

Boot a fresh MAME instance from the same standard World EEPROM for each case. The 128-byte EEPROM has SHA-256 `91b7b54d0612c77ff8e0ac228590f132b8e5b107f5baacce588f8cc172eff904`. The display-frame input ranges below are inclusive and use the frame-done callback to set pad fields for the next sample. P1 starts at Ryu and P2 starts at Ken in this sequence.

```text
1250 1255 coin1
1270 1275 coin2
1300 1305 start1
1340 1345 start2
1400 1405 lp1 lp2
1444 1446 l1
1452 1454 d1
1460 1462 d1
1470 1475 lp1 lp2
1540 1545 lp1 lp2
```

For Old Sagat, add `1478 1479 u1`, `1482 1483 d1`, `1486 1487 d1`, and `1490 1491 u1 lp1`. Confirm `P1+$391 = 09`, `P2+$391 = 04` and `P1+$3b6` equals 0 for New or 1 for Old. `P2+$3b6` is 0.

At frame 2160 after the round intro, set `$ff83a9 = 02` and `$ff8cd3 = 60`. Set P1 X to 552 and P2 X to 552 + gap, with both X fractions zero. Use gap 120 or 220. Send Down at `N−8..N−5`, Down+Forward at `N−4..N−1`, and Forward+button at `N..N+4`, where `N` is 2200 or 2201. In the clean fixture leave Ken neutral. For standing guard, hold Ken's Back at `N+3..N+110`. For crouch guard or neutral crouch, hold Down+Back or Down at `N−10..N+110`. The movement probe holds Sagat Back at `N+5..N+100`.

Use P1 base `$ff844e` and P2 base `$ff884e`. Watch fighter state at `+$03`, life at `+$2a`, freeze at `+$47`, fixed-point X at `+$06`, world Y at `+$0a`, and the animation pointer at `+$1a`. Projectile slots start at `$ff97a2` and have stride `$c0`. The first header write with data `0101` and mask `ff00` gives allocation time. The attack-box table pointer is `+$34`, attack ID is animation byte `+$0c`, and the attack record is `table + signed16(table+6) + 16*ID`. Its first four bytes give X, Y, half-width, half-height. If record byte 5 is at least 128, use its negated unsigned value for local X. Apply facing after decoding the local X. The TSV gives absolute action and allocation clocks plus relative outcome values for 144 cases. Phase 0/1 denotes the two input offsets. A dash indicates an inapplicable field or an unheld movement input.

## Port comparison

At port base `0e336743760689ccdd192cbe4ebce5c4d638c9c4`, the common Sagat table uses the SF1 Tiger Shot template for High and copies it for Low with a Y adjustment. The player test expects all punches to release High on its ninth recorded action frame in a 36-tick stream and all kicks to release Low on its twelfth in a 40-tick stream. That test fixes a 51 px separation between the port shot heights. The source shows a 40 px separation and a Low recovery difference between loadouts. The port test counts action frames from one and the arcade table counts action ticks from zero. [Sagat data bank](../../crates/sf-engine/src/data_bank.rs#L6079) · [Sagat player test](../../crates/sf-engine/tests/sagat_player.rs#L909)

The port uses the SF1 Tiger Shot handler and damage scaling before era-specific reaction. A paired port replay with matching selections, strength, spacing and defender pose is still open. The first source-backed changes to evaluate are the ST-only release and recovery split, flight speed by strength, and height and guard behavior. Shot clashes, cancel-entry timing and airborne shot reactions are outside this capture. [Tiger Shot collision handler](../../crates/sf-engine/src/fighter.rs#L4683)

### v1.5.17 Tiger Shot comparison

The Local Versus regression tests 60 cases across both Sagat loadouts and all six strengths. The port matches the World Turbo 2 allocation and first held retreat ticks, spawn offset and height, travel speed, contact ticks at 120 and 220 px, and four grounded defender postures. At 120 px it also verifies 14 defender freeze ticks, no attacker freeze, 78 px of recoil, and contact-to-ready at +36. One Low Tiger Shot guard case replays from the contact snapshot. The port converts representative source damage from the 144 HP bar and keeps it fixed by strength. Exact damage rolls, other defenders, airborne contacts, shot clashes and cancel-entry timing remain open. [Local Versus checks](../../crates/sf-engine/tests/sf2_native_moves.rs)

## Tiger Uppercut and Tiger Knee

Use the same MAME set, EEPROM, selection inputs, game clock and two display phases as the Tiger Shot fixtures. At frame 2160 set the requested axis gap, with both characters standing at world Y 40. These moves can walk forward during the input and close separation can trigger pushbox correction. Each case records the actual axis gap at the special-state write. The primary contact matrix uses requested gap 40. The whiff control uses gap 220 and holds Sagat Back after completing the motion. All listed times start at the special-state write.

### Commands

Tiger Uppercut accepts Forward, Down, Down+Forward+Punch (`623P`) in both loadouts. Tiger Knee accepts Down, Forward, Up+Forward+Kick (`269K`) in both. For each successful fixture hold the first direction at `N−8..N−5`, the second at `N−4..N−1`, and the final direction and button at `N..N+3`. Use `N=2200` or `2201`. The World ROM Knee parsers at `$08db5e` (New) and `$0a0fc8` (Old) advance on exact normalized direction nibbles `4`, `2`, then `a` from fighter `+$3a6`. The `2369K` control with four display samples on each direction produced Low Tiger Shot in both loadouts. The `623K`, `239K` and `29K` controls produced ordinary kick states.

While waiting for Forward or Up+Forward, the parser tolerates other directions and neutral until the current timer expires. Entering Down or Forward independently draws an 8–15 tick timer from the same RNG-indexed ROM table at `$08dc6c` and `$0a10d6`. The parser decrements the timer before checking the next direction. After accepting Up+Forward it sets 12 and checks kick press and release edges without a further direction requirement. The LK/MK/HK remaining-timer thresholds are 0/2/4, giving latest accepted delays of +11/+10/+8 from Up+Forward before other action eligibility checks. These timer limits come from disassembly. The slow `2369K` control uses enough display samples for the first timer to expire, and an intervening diagonal inside the timer does not itself reset recognition.

Delaying LK while holding Up+Forward can start Knee during jump startup. New accepted the sampled +1, +2 and +3 or +4 cases across the two phases. Old accepted +1 in both and +2 in phase 0. In the later Old cases the jump continued. The port uses the same countdown rules with its existing deterministic RNG state. Exact synchronization with the arcade RNG sequence remains unverified.

### Whiff timing and movement

Both loadouts share the following action and physical timelines at requested gap 220. X advance is the change of integer fighter X from entry to the neutral write, before the first held Back movement. Peak height is above the Y 40 floor.

| Move | Strength | First active | First rise | Floor return | Neutral / first held Back | Peak Y | X advance |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Tiger Uppercut | LP | +3 | +12 | +43 | +51 | 45 | 14 |
| Tiger Uppercut | MP | +3 | +12 | +54 | +61 | 81 | 33 |
| Tiger Uppercut | HP | +3 | +12 | +59 | +71 | 103 | 60 |
| Tiger Knee | LK | +4 | +9 | +28 | +35 | 31 | 108 |
| Tiger Knee | MK | +4 | +9 | +30 | +37 | 38 | 127 |
| Tiger Knee | HK | +5 | +10 | +33 | +40 | 45 | 145 |

The per-tick trajectory file includes X fractions and floor-relative Y through the neutral write. Each trajectory matched across both loadouts and both phases. [Whiff trajectory](sagat-super-turbo-melee-whiff-trajectory.tsv)

The fighter's airborne flag and fall substate have their own boundaries. A separate 24-case tap captured fighter `+$181` setting and clearing, and the write of substate 6 to `+$04`. Both phases and loadouts matched for each strength. An integer Y sample can equal the floor before the airborne flag clears.

| Move | Strength | Airborne flag set | Fall substate | Airborne flag clears |
| --- | --- | ---: | ---: | ---: |
| Tiger Uppercut | LP | +11 | +21 | +44 |
| Tiger Uppercut | MP | +11 | +25 | +54 |
| Tiger Uppercut | HP | +11 | +29 | +60 |
| Tiger Knee | LK | +8 | +15 | +28 |
| Tiger Knee | MK | +8 | +16 | +30 |
| Tiger Knee | HK | +9 | +18 | +33 |

The exact active pose boundaries below combine the fighter animation pointer writes with the animation attack ID at `pointer+$0c`. Each range includes both endpoints.

| Move | Active game ticks and attack IDs |
| --- | --- |
| Uppercut LP | +3–6 ID 13, +7–9 ID 14, +10–23 ID 15 |
| Uppercut MP | +3–6 ID 16, +7–9 ID 17, +10–31 ID 18 |
| Uppercut HP, Old | +3–6 ID 19, +7–9 ID 20, +10–35 ID 21 |
| Uppercut HP, New | +3–6 ID 19, +7–9 ID 20, +10–13 ID 21, +14–17 ID 56, +18–35 ID 57 |
| Knee LK | +4–6 ID 45, +7–9 ID 31, +10–14 ID 32 |
| Knee MK | +4–6 ID 46, +7–9 ID 33, +10–15 ID 34 |
| Knee HK | +5–7 ID 47, +8–10 ID 35, +11–17 ID 36 |

Uppercut's first, second and remaining strike geometries are `(42,31,28,27)`, `(57,68,42,18)` and `(61,118,33,39)`. Knee's three are `(30,18,18,19)`, `(27,67,29,15)` and `(36,61,32,15)`. Each tuple lists local X and Y followed by half-width and half-height. Decode the box through the fighter's table at `+$34` using the same record layout as the projectile.

The Knee’s first two boxes use contact key 1 and the extended kick uses key 2. New LK write taps show the first hit storing 1 at Ken’s `+$4a`. The second box still overlaps the standing recoil pose and is rejected with the same key. The third box can hit and stores 2. The ROM compares this key at `$0be592` before its guard branch, so the first two boxes also share one guard contact.

Knee's first two strike shapes can share a contact. The melee check at `$0be592` compares the attack record's contact key to the defender's previous key at `+$4a`. It takes attack record byte 5 when the value is below 128 and otherwise uses byte 0. The attack stores its key before hit or guard handling at `$0be5fc`. The gap-40 New LK tap shows IDs 45 and 31 both have key 1 and ID 32 has key 2. The opener leaves Ken's key at 1, so the next shape cannot hit or block him again. The third shape changes the key and connects. At gap 120, the second shape can hit when the opener misses. The raw LK fixture is `sagat-knee-rehit-rom/raw.txt` under the existing temporary arcade capture folder.

### Close standing, crouching and jumping contact

For standing guard hold P2 Back at `N+1..N+110`. For crouch guard and neutral crouch, hold Down+Back or Down respectively at `N−12..N+110`. For the jump probe hold Up at `N−10..N−6`. Every recorded clean hit or guard contact sets both Sagat and Ken freeze to 14. Follow-up times in the table include earlier hitstop.

| Move and loadout | Standing or crouching idle | Standing guard | Crouching guard |
| --- | --- | --- | --- |
| Uppercut LP/MP, both | +3 launches | +3 blocks | +3 blocks |
| Uppercut HP, New | +3 grounded, +21 launches, +38/+56/+74 juggle | +3/+21/+38 block | +3/+21 block |
| Uppercut HP, Old | +3 launches | +3 blocks | +3 blocks |
| Knee LK/MK, both | +4 grounded, +24 launches | +4/+24 block | +4 blocks |
| Knee HK, both | +5 grounded, +25 launches | +5/+25 block | +5 blocks |

All three Uppercut strengths first hit the controlled jumping Ken at +10. New HP follows at +28 and +46 in both tested phases. The phase-0 Old HK Knee hits the jump at +17. All other Knee jump probes whiff. The fixture includes actual defender Y and axis positions at each recorded hit.

Each hit or block adds 14 game ticks to Sagat's neutral return in these cases. For standing idle, the New HP Uppercut returns at +141 and Old HP at +85. LP/MP return at +65/+75 for either loadout. Clean two-hit LK/MK/HK Knee returns at +63/+65/+68 for either loadout.

The extended New HP Uppercut close capture records Ken's reaction through neutral in two input phases. After the last contact at +74, he reaches the floor at +147, starts rebound movement at +152, lands again at +168 and returns to neutral at +220. [Heavy Uppercut reaction trace](sagat-super-turbo-hp-uppercut-reaction.tsv)

At requested gap 80, the actual entry gaps are 67.5 for Uppercut and 72.5 for Knee in phase 0. Uppercut keeps the close standing hit sequences. LK Knee has only its first hit and MK/HK still land both. At requested gap 120, the entry gaps are 107.5 and 112.5 respectively. All Uppercut strengths first connect at +7, and New HP adds +29/+46 for three total contacts. LK/MK Knee first connect at +9/+7 for a single hit. HK Knee still connects twice at +8/+25. These are sampled spacings.

The [melee case writes](sagat-super-turbo-melee-writes.tsv) retain the posture and measured entry gap for each run. `sn` denotes standing neutral, `sg` standing guard, `cn` crouching neutral, `cg` crouching guard, `jump` the fixed jump, and `move` the held-Back whiff control. Each contact entry lists relative tick, damage, Sagat and Ken freeze, Sagat X and Y above floor, Ken X and Y above floor, and the attack record ID and rectangle. Each defender state write lists relative tick and the new state byte. Contact is the defender life write from arcade routine `$0be64c`. Record the landing with Sagat's Y write and the return with his fighter-state write to distinguish the two.

### Port comparison

At the inspected port baseline `0e336743760689ccdd192cbe4ebce5c4d638c9c4`, Sagat's SF2 move list and live Knee recognizer use `2369K`. The native Knee slot authors one trajectory across strengths with six startup ticks, two six-tick strike poses and 22 recovery ticks. Its true-combo description specifies one hit after either strike connects. The SF2 Uppercut slot uses a common rising stream across strengths and loadouts. The measured source gives Knee three strength trajectories and a grounded opening hit into a launching second hit at the close gap. Uppercut needs separate strength trajectories, with New HP's grounded-to-air sequence and Old HP's single launching hit. This is a static comparison. [SF2 move list](../../crates/sf-engine/src/moves.rs#L1341) · [Live SF2 recognizer](../../crates/sf-engine/src/era_fight.rs#L40) · [Sagat Uppercut and Knee bank](../../crates/sf-engine/src/data_bank.rs#L6138)

### v1.5.17 Tiger Knee and Uppercut comparison

The Super Turbo card now lists and accepts `269K`. Short intervening directions can fit inside the measured countdowns. The whiff tests verify all six trajectories, active ticks and action exits for both loadouts. The port rounds arcade X to the nearest integer pixel and uses its resident Sagat poses.

The close Local Versus fixtures check standing hit, standing guard and crouching guard. Knee’s first two boxes share one contact opportunity and the extended kick can connect separately. New HP Uppercut hits at +3/+21/+38/+56/+74 and Old HP launches on its first hit at +3 in the tested standing fixture. The port reuses its authored Ken airborne reaction for New HP. Exact receiver motion, bounce and wakeup, the wider contact matrix, damage rolls and other defender profiles remain open. [Local Versus checks](../../crates/sf-engine/tests/sf2_native_moves.rs) · [Authored pose streams](../../crates/sf-engine/src/data_bank/st_sagat_melee.rs)

## Tiger Genocide

### Motion, stock and Old fallback

The World arcade New parser at `$08d9aa` tracks exact `23623`. On the tick that accepts the second Down+Forward it tests a kick press or release edge. If no kick edge is used, it sets a 12-tick timer and tests punch press or release edges on later ticks. Expiry occurs before input on the twelfth subsequent game tick. The punch edge can occur with any direction. The earlier directional stages use RNG-indexed timers from `$08db00` plus eight. The engine's super eligibility routine at `$0045dc` requires raw stock exactly 48.

With stock 48, `236236P` reliably starts New Tiger Genocide with LP, MP or HP. `23623K` starts it with LK, MK or HK if the button arrives on the exact second Down+Forward tick. Holding the diagonal and delaying the kick by one display event produced Low Tiger Shot in both phases. A punch on that diagonal works if it arrives after the recognition tick. If the punch arrives on the same game tick as the second Down+Forward, it can select Tiger Uppercut. Full `236236+LK` selected Low Tiger Shot.

Set `Sagat+$2b4` to 48 at display frame 2170. At `N−10`, `N−8`, `N−6`, `N−4` and `N−2`, hold each of `2, 3, 6, 2, 3` for two display events. At `N` hold `6` and press a punch. For the kick shortcut, start at `N−8` and put the kick on `3` at `N`. Use `N=2200` or `2201`. The full motion with Forward held and LP delayed six display events accepted at +10 and +11 after the second Down+Forward in the two phases. Delay seven produced an ordinary standing LP at +12 in both phases. Stock tests at 0, 1, 32, 45, 46, 47 and 49 produced Tiger Uppercut on the full LP motion.

Old Sagat produced Tiger Uppercut for the full and short LP motions and Low Tiger Shot for the tested LK motions, with stock 0 or the injected 48. None of those Old probes entered Tiger Genocide or spent the supplied stock.

### Flash, strikes and recovery

The first animation pointer is `$15c7ae`. On raw activation from neutral, state 12 entry, flash set and stock clear occur on the same tick. The flash byte clears at action +18. The six strike shapes below use the Knee geometries followed by the Uppercut geometries documented above. The whiff probe uses requested gap 300 and keeps Ken outside the hitboxes by writing his Y to 600 each display event from 2215 through 2325. At requested gaps 256, 300 and 350 with both fighters grounded, camera constraints bring Ken into range and the super still gets one hit. The lift fixture isolates Sagat's unimpeded action.

| Whiff phase | Action ticks | Attack ID |
| --- | --- | ---: |
| Opening knee | +24–26 | 50 |
| High knee | +27–29 | 51 |
| Air knee | +30–35 | 52 |
| Opening uppercut | +52–55 | 53 |
| Second uppercut | +56–58 | 54 |
| Rising uppercut | +59–85 | 55 |

Whiff returns Sagat to neutral at +118 and the held-Back probe moves on that same tick. X advances 131 integer px to the neutral write. On every measured Genocide contact, Sagat freeze is 6 and Ken freeze is 8. Each contact extends Sagat's return by six ticks. [Per-tick whiff trajectory](sagat-super-turbo-genocide-whiff-trajectory.tsv)

| Defender probe | Contact ticks from entry | IDs | Sagat neutral |
| --- | --- | --- | ---: |
| Standing or crouching idle at 40 | +24, +33, +42, +74, +83 | 50, 51, 52, 54, 55 | +148 |
| Standing guard at 40 | +24, +33, +42, +70, +80, +89 | 50, 51, 52, 53, 54, 55 | +154 |
| Crouch guard at 40 | +24, +33 | 50, 51 | +130 |
| Fixed jump at 40 | +52, +62, +71 | 53, 54, 55 | +136 |

In the close clean sequence the first two contacts are grounded, the third lifts Ken, and the last two arrive on the air-reaction sequence. The full-motion standing LP fixture dealt 71–72 damage across the two phases. Standing guard took 20 chip and crouching guard took 6. The defender posture schedules are the same as the other Sagat melee fixtures. Use Sagat `+$9e/+$9f` for the command stage and timer, `+$1fa` for flash, `+$2b4` for stock, and `+$2b0` for the grounded super-cancel timer.

### Grounded cancel

Use stock 48 and requested gap 32. Hold Down at `N−4..N−1` and press Down+LP at `N`. Use one display event each for Down+Forward at `N+1`, Forward at `N+2`, Down at `N+3`, Down+Forward at `N+4`, and Forward+LP at `N+5`. Both phases give a six-hit combo on standing neutral Ken. The same input pattern works from crouching LK, MP and MK in the clean fixture.

The isolated normal controls used one button event at `N = 2200` or `2201` with Down held from `N−4` through `N+45`. Leave Ken standing at a 140 px gap for whiff, or use requested gap 32 for contact. The pushboxes settle at 38 px before the hit. The guard replay holds Ken's Down+Back from `N−12`. Anchor the timings to Sagat's exact state 10 write at `$07b378`. The next animation-pointer write defines the end of the active phase even when the pre-update display sample still carries the previous box. New and Old share the following timings and attack rectangles.

| Move | Active | Whiff / contact return to crouch | Attack center X/Y and horizontal × vertical radii | Clean damage, New / Old |
| --- | --- | --- | --- | --- |
| 2LP | +5 through +7 | +12 / +26 | +57/+44, 29 × 11 | 11 / 11 |
| 2LK | +5 through +8 | +14 / +28 | +52/+10, 33 × 10 | 4 / 11 |
| 2MP | +5 through +8 | +15 / +29 | +62/+44, 34 × 11 | 19 / 19 |
| 2MK | +5 through +8 | +16 / +30 | +62/+10, 41 × 10 | 19 / 19 |

All four hit standing Ken and block on crouching Ken at +5. Clean and guard contacts load both freeze counters to 14, and a guard takes no HP. Ken recoils 30 px and exits hitstun or blockstun at contact +27 after a light normal. He recoils 54 px and exits at +32 after a medium normal. Both phases of each loadout gave the same results. The fixture names in `%TEMP%/ssf2-ryu-audit/sagat-t2-genocide-openers` follow `<loadout>-lp-<gap>-<phase>-<posture>-only<button>-48-0`. The New active animation pointers are `$15bc2e`, `$15bd72`, `$15bcaa`, and `$15bdee` in the table order. The Old pointers are `$1ed3e0`, `$1ed524`, `$1ed45c`, and `$1ed5a0`.

The 2LP hits at normal action +5 and deals 11 HP. Tiger Genocide replaces the normal at contact +2, while 13 ticks of Sagat's normal hitstop remain. Flash and stock spend occur at contact +15. The five super hits arrive at opener contact +39, +48, +57, +89 and +98, giving 83–85 total damage. The combo counter writes 1 through 5 for the five super contacts after the opener. Moving the final LP to `N+6` in phase 0 selects Tiger Uppercut at contact +3 with stock intact.

In the late replay `new-lp-32-0-sn-flp-48-6`, Sagat's raw Forward+LP press is sampled on game tick 26. The super gate reached zero on tick 25, then the ordinary Uppercut routine writes state 12 at `$08ddf0` on tick 26. The controller release follows on tick 27.

The generic grounded super check at `$0045ba` requires a nonzero `+$2b0` in normal attack state. In these four normal fixtures the timer starts at 7, continues to advance through freeze, and is still 1 when the successful +2 input is processed. For the guard control, hold Ken Down+Back at `N−12..N+110`. The opening normal blocks without HP loss, Genocide starts at contact +2, and the remaining five guard contacts deal 15 chip in the 2LP probe. The saved write log uses the first normal action write as tick zero for these cancels and the special write for raw activations. [Flash, contact and cancel writes](sagat-super-turbo-genocide-writes.tsv)

The inspected port presents Tiger Genocide as double quarter-circle Kick and supplies a generic rising arc with a three-hit limit. The arcade confirms both a simultaneous-diagonal kick edge and a later punch route, followed by distinct six-strike geometry. [SF2 move list](../../crates/sf-engine/src/moves.rs#L1465) · [Sagat super bank](../../crates/sf-engine/src/data_bank.rs#L6145) · [Move hit limits](../../crates/sf-engine/src/era_fight.rs#L898)

### v1.5.18 Tiger Genocide and crouching-normal comparison

The port accepts both World commands for New Sagat, with the short kick edge confined to the second Down+Forward tick. The exact standalone stock gate is 48. The matched Old controls give Tiger Uppercut or Low Tiger Shot. The Local Versus replay checks the 18-tick flash, six strike windows, integer whiff trajectory and +118 return. At the tested gap it reproduces all five clean hits, all six standing blocks and both crouching blocks with 6/8 contact freeze. The clean replay verifies rollback from the launch. The port uses resident Sagat poses, deterministic command timers and representative damage from the measured samples. The arcade RNG sequence, exact launch and wakeup trajectory, and contacts with other defenders remain open. [Local Versus checks](../../crates/sf-engine/tests/sf2_native_moves.rs) · [Authored pose streams](../../crates/sf-engine/src/data_bank/st_sagat_melee.rs)

The four crouching light and medium normals match the measured active windows, return, contact freeze, damage, recoil distance and reaction clocks against New Ken in both loadouts. The crouching LP cancel replay verifies Genocide entry at contact +2, flash and stock spend at +15, and all five follow-up hits. The +3 input selects Tiger Uppercut immediately and preserves stock. The full late-cancel boundary remains open. [Local Versus checks](../../crates/sf-engine/tests/sf2_native_moves.rs) · [Authored normal streams](../../crates/sf-engine/src/data_bank/st_sagat_normals.rs)

### Crouching HP and sweep

The World Turbo 2 heavy normal fixtures use the same input schedule and requested gaps as the isolated crouching controls, with zero stock. Standing guard holds Ken's Back from `N+1`, and crouching neutral holds Down from `N−12`. All 40 fixtures retain exact state, animation, position and life writes in `%TEMP%/ssf2-ryu-audit/sagat-t2-heavy-normals`. The case names follow `<loadout>-lp-<gap>-<phase>-<posture>-only<button>-0-0`.

| Move | Active ticks | Whiff / contact return to crouch | Attack center X/Y and radii | Active pointer, New / Old |
| --- | --- | --- | --- | --- |
| 2HP | +7 through +12 | +26 / +40 | ID 27, +65/+44, 40 × 11 | `$15bd0e` / `$1ed4c0` |
| 2HK | +7 through +12 | +30 / +44 | ID 24, +67/+10, 53 × 10 | `$15be82` / `$1ed634` |

Each tested contact loads Sagat's and Ken's freeze counters with 14. HP hits standing and crouching neutral Ken, and blocks on either guard. Sweep hits both neutral postures and standing guard, and blocks on crouching guard. HP hit or block and sweep block produce the same 20 recoil increments as Ryu's heavy normals, starting at contact +16 and totaling 78 px. Ken leaves grounded reaction at +36. Sweep hit lifts Ken from +17 through +55 with the same trajectory as Ryu's sweep. Ken advances 2 px each tick from +17 to +56, reaches a height of 55 px, lands at +56 after 80 px of travel and leaves knockdown at +95. [Ryu sweep trajectory](ryu-st-crouching-normals.md#hit-and-guard)

| Clean defender posture | New HP, phases 0 / 1 | Old HP, phases 0 / 1 | New sweep, phases 0 / 1 | Old sweep, phases 0 / 1 |
| --- | --- | --- | --- | --- |
| Standing neutral | 18 / 22 | 26 / 26 | 18 / 22 | 27 / 27 |
| Crouching neutral | 24 / 24 | 24 / 26 | 24 / 24 | 26 / 27 |
| Standing guard | Block | Block | 22 / 21 | 26 / 27 |

The authored bank uses phase 0 standing-neutral damage as a fixed value. The wider damage-roll distribution remains open. New Sagat's `+$2b0` super gate starts at 7 and reaches zero at action +7, the first active tick, without rearming on contact. Old Sagat does not initialize the gate. [Authored normal streams](../../crates/sf-engine/src/data_bank/st_sagat_normals.rs)

### Standing normal whiff measurements

In the same World Turbo 2 fixture, set the requested axis gap at display event 2160 to 40 or 140 px. At display event 2200 press one attack button for a single event with Sagat's stick neutral. Ken stays grounded through the state 10 write so the source chooses the normal using the grounded gap. On subsequent display events while Sagat is still in state 10, set Ken's Y to 600 to avoid contact and retain the unimpeded animation. All 24 runs had no life write from collision. The results are relative to Sagat's exact entry to state 10 and use one display phase. The two gaps distinguish the punch variants and yield the same kicks.

| Move and gap | Active ticks, New / Old | Whiff exit, New / Old | Active attack center X/Y and radii, in order |
| --- | --- | --- | --- |
| 5LP, 40 px | +4 through +7, both | +12, both | ID 37, +60/+63, 37 × 7 |
| 5LP, 140 px | +4 through +7, both | +13, both | ID 1, +69/+80, 35 × 12 |
| 5MP, 40 px | +4 through +9 / +5 through +10 | +17 / +18 | ID 38, +61/+59, 36 × 10 |
| 5MP, 140 px | +6 through +9, both | +19, both | ID 2, +76/+75, 43 × 12 |
| 5HP, 40 px | +4 through +11 / +7 through +14 | +23 / +26 | ID 39, +68/+82, 43 × 15 |
| 5HP, 140 px | +8 through +13, both | +37, both | ID 25, +86/+78, 53 × 9 |
| 5LK, 40 and 140 px | +4 through +5, then +6 through +7 | +15, both | ID 8, +44/+23, 21 × 21. ID 9, +77/+44, 44 × 13. |
| 5MK, 40 and 140 px | +4 through +5, then +6 through +7 | +21, both | ID 6, +38/+58, 27 × 15. ID 7, +80/+78, 49 × 12. |
| 5HK, 40 and 140 px | +4 through +6, then +7 through +9 | +33, both | ID 4, +39/+58, 28 × 17. ID 5, +96/+99, 31 × 17. |

The same recipe tested adjacent integer world gaps around each transition. For both loadouts the last close and first far gaps are 81 / 82 px for LP, 68 / 69 px for MP and 92 / 93 px for HP against New Ken. At `$078180`, the engine subtracts the defender's `+$1c2` from the absolute integer axis gap and clamps the result at zero in Sagat's `+$182`. Ken's value is 40. The New LP/MP/HP comparators at `$08e11a/$08e164/$08e1ae` and the Old comparators at `$0a156c/$0a15b6/$0a1600` select the far pose at effective gaps 42/29/53 or greater. Raw traces are in `%TEMP%/ssf2-ryu-audit/sagat-t2-standing-whiffs`, `%TEMP%/ssf2-ryu-audit/sagat-t2-standing-thresholds` and `%TEMP%/ssf2-ryu-audit/sagat-t2-standing-distance-register`, named `<loadout>-lp-<gap>-0-whiff-stand<button>-0-0`. The new standing bank uses resident Sagat drawings for the measured variants. [Resident Sagat bank](../../crates/sf-engine/src/data_bank.rs) · [Authored standing bank](../../crates/sf-engine/src/data_bank/st_sagat_standing.rs)

Separate neutral World Turbo 2 matches read the width field at display event 2160. New Ryu (ID 0) and both Kens (ID 4) use 40. Both Sagats (ID 9) use 44. Against Sagat, the same comparators therefore imply first-far gaps 86, 73 and 97 px for LP, MP and HP. This inference uses the measured width and the shared arcade routine.

### Standing normal contact measurements

Repeat the standing recipe with Ken on the floor throughout. Leave Ken neutral, hold Back from `N+1` for standing guard, or hold Down+Back from `N−12` for crouching guard. The punch controls use the 40 px close gap and the first far gap for each punch. Both loadouts hit neutral Ken and block on standing guard at first active. Close LP and MP also block on crouching guard. Close HP and all three far punches whiff on crouching guard. The measured punches load 14 ticks of freeze for each fighter. LP, MP and HP yield 30, 54 and 78 px of grounded recoil and reaction exits at contact +27, +32 and +36, respectively.

| Standing punch clean damage | New LP / MP / HP | Old LP / MP / HP |
| --- | --- | --- |
| Close | 11 / 19 / 23 | 11 / 19 / 24 |
| Far | 11 / 19 / 21 | 11 / 22 / 27 |

At 40 px, LK hits at action +4 and +20 on neutral and standing guard, and blocks twice on crouching guard. The clean damage is 11 + 11 in both loadouts. The low first strike puts standing Ken in hitstun through the second contact. MK hits at +4 and +20 on neutral and blocks twice on standing guard. Crouching guard blocks the first and the second whiffs. The clean damage is 12 + 16 New and 11 + 15 Old. HK hits or blocks once at +4 on the three tested postures and takes 14 HP on clean. These contacts load 14 ticks of freeze for each fighter. The second clean LK or MK reaches Ken just after his first recoil step of 4 or 6 px. At that instant Ken's head and torso are IDs 27 and 19, with facing-relative center/radii `(1,75,14,9)` and `(10,47,21,21)` relative to his axis and the floor.

At 100 px the first kick phase whiffs. The LK second phase hits at +6 for 11 HP in both loadouts and blocks on standing or crouching guard. MK's second phase hits at +6 for 16 New or 18 Old, and HK's second phase hits at +7 for 20 New or 19 Old. Both block standing and whiff on crouching guard. Each kick uses the same light, medium or heavy recoil as its punch counterpart. The measured clean figures are the phase 0 samples used as fixed values in the port. Raw controls are in `%TEMP%/ssf2-ryu-audit/sagat-t2-standing-punch-live` and `%TEMP%/ssf2-ryu-audit/sagat-t2-standing-kick-live`.

The [standing normal writes](sagat-super-turbo-standing-writes.tsv) retain the first pose, action exit, each contact, cumulative damage and Ken's reaction exit and displacement. The posture codes are `sn` for standing neutral, `sg` for standing guard, `cg` for crouching guard and `whiff` for the lifted control.

### v1.5.19 grounded-normal comparison

The port now uses separate close and far punch streams and phase-specific standing kick boxes in both Super Turbo loadouts. The tests compare startup and recovery on both sides of each punch proximity threshold. They also compare standing and crouching guard, close two-hit kicks, far second-strike contacts, and the HP and sweep reactions against New Ken. Standing punches use the source's opponent-width rule with the measured 40 px for Ryu and Ken and 44 px for Sagat in Super Turbo. Other opponents use 40 px pending comparison. The port uses resident art and fixed damage from the measured examples. Exact arcade damage rolls, broader matchup boxes and ordinary cancel windows remain open. [Local Versus tests](../../crates/sf-engine/tests/sf2_native_moves.rs)

## Ground walk

New and Old Sagat move forward by exactly `+$00028000` (+2.5 px) and backward by `−$00020000` (−2 px) per advancing World Turbo 2 game tick. Both directions move on the first tick that samples the input after idle. Back therefore first writes X598.0 from X600.0, and Forward first writes X602.5. The first neutral update after release adds no movement. New Ken's world axis stays fixed during these runs.

The display at 59.637405 Hz can include one or two advancing ticks per interval. The first visible movement is one display interval after the queued input, with a forward change of +2.5 or +5 px or a backward change of −2 or −4 px. In each 16-interval window below, measure from the first visible movement sample to the sample 16 display intervals later. The observed differences between loadouts track the number of advancing updates in the sampled windows. Both loadouts use identical 16.16 increments.

| Loadout | Forward across 16 display intervals, px | Back across 16 display intervals, px |
| --- | --- | --- |
| New | +52.5, +52.5, +52.5 | −40, −40, −42 |
| Old | +55, +52.5, +52.5 | −42, −40, −42 |

The six windows per direction average +52.92 and −41.00 px, or +197.24 and −152.82 px/s at the arcade display rate. Those elapsed speeds would average about +3.29 and −2.55 px per 60 Hz port tick. The sampled means depend on the chosen display phases. The exact advancing-tick increments and a fixed elapsed-time cadence are separate targets. [Ryu elapsed pacing measurement](2026-11-13-ryu-era-pacing.md#world-turbo-2-walk-and-light-hadoken-playtest-build)

Use World `ssf2t` 940223 in MAME 0.289 with the Sagat and Ken selection and Turbo 2 setup above. At display event 2160, set Sagat to X600 and Ken to X768 and clear both horizontal fractions. Queue only Forward or Back from event `N` through `N+28`, with `N = 2200, 2201, 2202`. Tap the input word `+$392`, integer X `+$06` and fractional X `+$08` and align each write to the game tick byte `$ff801d`. Recombine the two position words using the final writes in each game tick. Retain the ordinary display samples to measure elapsed travel. The 12 captures and exact event schedules are under `%TEMP%/ssf2-ryu-audit/sagat-t2-walk`, named `<loadout>-<direction>-<phase>`.

The v1.5.21 port fits both loadouts to +105 px forward and −82 px backward per 32 ticks at 60 Hz, or 196.875 and 153.75 px/s. Each tick distributes one or two of the source movement increments across a fixed cycle with integer position rounding. Neutral stops movement on the first sampled tick. The capture TSV preserves the source display samples and raw 16.16 displacement separately. [Arcade walk samples](sagat-super-turbo-walk.tsv) · [Local Versus checks](../../crates/sf-engine/tests/sf2_native_moves.rs)

## Ground throw

At World Turbo 2, New and Old Sagat use Forward or Back plus MP or HP for the same ground throw against New Ken. A one-display input of LP, LK, MK or HK does not capture at 38 px in either direction. The accepted punch buttons capture at a 77 px axis gap and fail at 78 px. Full and softened trajectories start from the same capture. The defender's throw word write to `$ffff` defines tick zero.

| Result | Release | Damage | First landing | Ken neutral |
| --- | ---: | --- | ---: | ---: |
| Forward, full | +39 | 32 HP at +83 | +82 | +146 |
| Back, full | +39 | 32 HP at +83 | +82 | +145 |
| Either direction, softened | +39 | 16 HP at +39 | +96 | +102 |

In the midstage trace, Sagat's axis stays at X650. The forward release places Ken at X777 and the backward release at X523, at 107 px above the floor in both directions. Ken ends at X949 or X350 after the full knockdown. Sagat returns to neutral at +59 in full and softened throws. Defender directional MP sampled at contact +10 through +13 softened the throw. The tested +14 through +22 responses gave full damage. Both Sagat loadouts allow New Ken to soften.

Use the selection and speed setup above, with Sagat at X650 and Ken at X688 at display event 2160. At `N = 2200` or `2201`, press Forward or Back and a button for one display event. For softening, New Ken presses Back+MP for one display event at `N+8` through `N+17`. Tap the throw words `+$62`, life `+$2a`, state `+$02`, and axis X/Y `+$06/+$0a`. The 200-case archive records sampled input and result ticks. The trajectory archive preserves the final integer axis and height writes of each game tick for both directions and outcomes in the New Sagat phase 0 fixture. [Throw write summaries](sagat-super-turbo-throw-writes.tsv) · [Per-tick throw trajectories](sagat-super-turbo-throw-trajectory.tsv)

In the phase 0 follow-up with New Sagat's forward MP throw, New Ken's Back+MP softens at measured ticks +0, +1, +2, +3, +5, +6, +7 and +9. At +13, Back with HP, MK or HK also softens. Back with LP or LK and neutral MP give full damage. Raw writes are in `%TEMP%/ssf2-ryu-audit/sagat-t2-throw-soft-controls-new-1-mp-0-*`.

The port uses the shared directional MP/HP/MK/HK response window through +13 against New Ken. The throw itself accepts directional MP/HP and applies the measured 77 px range, capture, release, damage and recovery clocks. The 16 and 32 HP arcade losses convert to 107 and 213 Sub-HP on the port's 960-point health scale. The trajectory test covers all 499 samples. Sagat uses the resident grab, knee and fling drawings at +0, +6 and +26. Arcade pose writes also occur at +32, +38 and +58. The port uses its resident Ken reaction drawings through release, bounce and recovery. [Port throw profile](../../crates/sf-engine/src/fighter/st_ryu_throw.rs) · [Local Versus checks](../../crates/sf-engine/tests/st_sagat_throw.rs)
