# M. Bison in Champion Edition World 920513

These are arcade measurements for the Champion Edition Bison style in [#30](https://github.com/johnstreetfighter/sf1-port/issues/30). MAME 0.289 verifies the `sf2ce` parent, Street Fighter II' Champion Edition (World 920513). Bison is P1 (ID 8) and Ken is P2 (ID 4), with 144 HP each at the start of every case. The P1 input write at `$0020f0` advances once per display in the sampled combat interval. Clocks below count those writes from the first dispatch, with the jump input as the origin for aerial normals. [Hyper Fighting World 921209](bison-hyper-fighting-world-921209-audit.md) · [Super World 931005](bison-super-world-931005-audit.md) · [Super Turbo World 940223](bison-super-turbo-bonus-audit.md) · [MAME set manifest](https://github.com/mamedev/mame/blob/mame0289/src/mame/capcom/cps1.cpp)

## Movement and normals

Bison walks +4 px forward or −3 px backward per free game tick, as in Hyper Fighting and Super. Both Champion Edition phases produce 29 steps over 29 displays, for +116/−87 px. Super gives the same travel. Hyper Fighting advances 40 updates during the first matched 29-display hold, with later forward steps limited by Ken. The three unarmed jump arcs lift at +3. Neutral, forward and back peak at 108/103/112 px above floor, first return to world Y 40 at +50/+49/+51 and return to neutral at +58/+56/+59.

| Button | 38 and 85 px standing active / exit | 54 px crouching active / exit |
| --- | --- | --- |
| LP | +4–8 / +13 | +4–8 / +12 |
| MP | +7–11 / +17 | +5–9 / +15 |
| HP | +9–15 / +35 | +9–15 / +35 |
| LK | +5–9 / +14 | +5–9 / +14 |
| MK | +5–9 / +17 | +5–13 / +18 |
| HK | +6–12 / +33 | +7–36 / +53 |

Active endpoints are exclusive. Both standing gaps use identical observed attack phases. Hyper Fighting and Super exit far MP at +20 and crouching MP at +18. Champion Edition standing MK activates two ticks earlier and exits four ticks earlier than both. Standing HK activates three ticks earlier than Hyper Fighting and one tick earlier than Super. Champion Edition standing LK uses box 6 centered 67 px forward and 64 up with radii 46 by 8. Hyper Fighting uses box 6 centered 57 px forward and 61 up with radii 36 by 11. Super adds a separate fast close LK and MP. Champion Edition standing HK uses box 20 with center 79 px forward and 64 up and radii 58 by 8. Hyper Fighting uses box 67 with center 72 px forward and 64 up and radii 51 by 8. Champion Edition crouching MK reaches 5 px farther forward and has 5 px more horizontal radius than the corresponding Hyper Fighting and Super box.

Clean grounded contacts load 14 freeze on both fighters. From nominal 54 px, crouching LP/LK, MP/MK and HP leave Ken reacting for +28/+33/+37 with 30/54/78 px recoil. Crouching HK contacts at +7 and Ken's first clean knockdown ends +96 from contact. The measured +28/+33/+37 and +96 clocks agree with the comparable Hyper Fighting and Super cases.

All six aerial buttons were captured in neutral, forward and back jumps. A button held from display `N+16` first activates at +18 for LP/LK, +19 for MP/MK, or +21 for HP/HK in all three directions. Medium attack boxes end at +39 and heavy attack boxes at +29. Light attack boxes persist through the last airborne sample. Neutral and diagonal jumps select distinct boxes. In the nominal 155 px forward-jump contact fixture, standing guard blocks all six buttons at some sampled schedule and crouch guard registers no contact. Clean neutral Ken is hit by LK, MK and HK in that fixture. Those contacts gave 14/14 freeze and +28 reaction. At the +36 button schedule, all three contacted at +44 for 8/15/22 HP and 12 px net Ken recoil. Guarded LK/MK/HK at the same schedule produces +28/+33/+37 reaction and 30/54/78 px recoil. The table retains the other gaps, guards and whiffs independently of the normal names.

## Charge specials

The charge-load instructions at `$0434d4/$043628/$043788` use table bytes indexed by `$31c(a5)`. At index 0 the values are 60 polls for Psycho Crusher, 93 for Double Knee and 60 for Head Press, at `$0434dc/$043630/$043790` respectively. The separate release-window tables at `$043514/$043668/$0437c8` contain 8–15 polls. Hyper Fighting's Double Knee loads 108. Champion Edition accepts both pure Back with a two-display Forward lead and the tested direct Down-Back to Forward+button controls for horizontal specials. The matched direct Down-Back Hyper Fighting controls fell through to normals.

| Move | Entry write PC | First active on whiff, light / medium / heavy | Whiff special exit | Clean contacts at listed gap |
| --- | --- | --- | --- | --- |
| Psycho Crusher | `$0435e2` | +13 / +13 / +13 | +57 / +61 / +61 | +13 at 108 px |
| Double Knee Press | `$043742` | +8 / +8 / +8 | +30 / +31 / +32 | +8 and +29 at 64 px |
| Head Press | `$04389c` | +30 / +30 / +30 | +74 / +74 / +74 | +30 at 85 px |

Psycho Crusher shares Hyper Fighting's startup and centered attack box, with horizontal center 0, vertical center 55 and radii 81 by 15. Super and Super Turbo use +18/+15/+11 and a box 31 px forward with 44 px horizontal radius. Champion Edition neutral Ken is hit once and recovers from knockdown at +128. At the tested 108 px crouch and 118.625 px standing guard entry gaps, LP/MP chip at +13/+38/+63 and HP also chips at +88, with 5–7 HP per contact across both phases. Hyper Fighting yielded two chips for each strength in its sampled guard fixtures. Super yielded two for LP and MP and one for HP against the sampled standing guard. The Champion Edition crusher cycles its primary and secondary boxes. The four-hit HP case against standing guard did not yield a defender recovery marker in the extracted data.

Double Knee shows both whiff boxes at +8/+16 for all strengths. Both clean phases hit +8/+29 with a second hit of 5–6 HP and a +36 grounded reaction from that contact. Hyper Fighting also leaves Ken grounded, with strength-dependent contact clocks and higher second-hit damage. Super and Super Turbo knock down on the second clean strike at the matched gap. In Champion Edition all three strengths chip twice against the measured standing guard, while only LK does so against crouch guard. The first chip is 4–5 HP and the second 1.

LK Head Press lands for 15 HP with 14/14 freeze in both phases. A three-display LP follow-up at `N+52` leaves one hit and the special exits at +95. Starting LP at `N+53`, `N+54` or `N+65` adds a 19 HP second hit at +93 and exits the special at +109. Super has the same measured display threshold and a later-strengthened follow-up. Hyper Fighting's +36/+37 display boundary tracks its faster clock. The separate Up+Punch dive command selects ordinary punches in Champion Edition and Hyper Fighting, while Super activates the automatic dive. The later Nightmare Kick command selects ordinary Double Knee in both Champion Edition phases.

## Ground throw

Forward or Back + MP/HP captures at 38 px. LP and the three kicks dispatch their ordinary normals. In both directions MP captures at 81 integer axis px and falls through to standing MP at 82. Ken's lock write reports PC `$003460`. Bison releases at +40 and is neutral at +65. Ken loses 32 HP at +75 and is neutral at +149. Both freeze counters stay zero. These are also the matched Hyper Fighting and Super game ticks. The sampled one-display Left+MP responses at +6, +8 and +11–15 preserve the full throw in both directions. Super also preserves full damage. New Super Turbo Ken softens at +13.

## Reproduction

Boot `sf2ce` with original timing. Hold P1 Coin at displays 800–810, 840–850 and 880–890, P1 Start at 920–930, and P2 Start at 975–985. Move P1 Left at 1050–1053 and Down at 1062–1065 to select Bison. Leave Ken's cursor in place. Confirm both fighters with LP at 1160–1170, 1220–1230 and 1300–1310. Directions and punches use `$800000`, system controls `$800018`, and kicks `:IN2`. Verify the IDs and full life at `N=2200` or `2201`.

Place Bison at X 552 and Ken at the nominal gap, both with zero fractional X, at display 2060. Reset X at `N−4` for Crusher and Knee. Use X 600/768 for walks and X 650 plus the nominal gap for the unarmed jump control. Move Ken to Y 600 from initial placement on elevated whiff trials. Walk for 29 displays. Normal, jump and throw inputs start at `N` and hold through `N+2`. Crouching normals keep Down from `N−10` through `N+90`. In the aerial grid press the measured button for three displays starting at the listed delay. Standing guard uses Ken's Right from `N+1` for grounded normals or `N−4` for specials and aerial cases. Crouching guard uses Down+Right from `N−12`. Throw softening controls press Ken's Left+MP for one display beginning at the listed delay.

For the pure Back charge cases, hold Back from `N−152` through `N−3`, Forward from `N−2`, and Forward+button from `N` through `N+3`. The `diag` horizontal charge cases hold Down+Back through `N−1` and replace it directly with Forward+button. For Head Press and the Up+Punch probe hold Down from `N−125` through `N−4` and press Up+button from `N` through `N+2`. The follow-up LP is held three displays from the listed delay. The Nightmare control retains Down+Back until `N−9`, Forward at `N−8…N−5`, Back at `N−4…N−1` and Forward+LK from `N` through `N+3`.

Tap fighters `$ff83be/$ff86be`, input `+0x292`, state `+0x03`, axis `+0x06/+0x0a`, animation `+0x1a`, freeze `+0x47`, life `+0x2a` and throw lock `+0x63`. The defender freeze load at `$095004` also identifies zero-chip guard contacts. Strike and chip life writes report `$095030`. Resolve the attack ID from pose `+0x0c` and the position/radii from the fighter's box table at `+0x34` using the signed attack offset at table `+0x08` and 12-byte records.

The [507-case write summary](bison-champion-edition-world-920513-writes.tsv) retains actual entry gaps, separate game and display clocks, box changes, terminal states, walk and aerial displacement, and contact sequences. Box values are ID, facing-relative X, Y, horizontal radius and vertical radius. Contact entries give tick, display, HP change, attacker freeze, defender freeze, contact-relative defender recovery, Ken X travel, attacker pose and attack box. Fixed-point deltas are scaled by 65,536. A dash means the event was absent, inapplicable or outside the capture. The `air` suite is the forward contact grid; its `control` whiffs cover all three jump directions. The `bound` rows are extra command replicates. Raw write taps and per-display inputs are grouped under `%TEMP%/ssf2-ryu-audit/sf2ce-bison-*` locally.
