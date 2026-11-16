# M. Bison in Hyper Fighting World 921209

This is the reference for the Hyper Fighting Bison style in [#30](https://github.com/johnstreetfighter/sf1-port/issues/30). The arcade parent `sf2hf`, Hyper Fighting World 921209, passes `-verifyroms` in MAME 0.289. Bison (P1 ID 8) and Ken (P2 ID 4) were selected through the default cabinet, with full 144 HP. The game clock is indexed by the once-per-update P1 input store at `$003ec2`. The same display can have two input stores. Unless labeled as displays, all clocks below are relative to the first dispatch write. Compare these with the matched [Super World 931005](bison-super-world-931005-audit.md) and [Super Turbo World 940223](bison-super-turbo-bonus-audit.md) captures. [MAME CPS1 driver and set inventory](https://github.com/mamedev/mame/blob/mame0289/src/mame/capcom/cps1.cpp)

## Movement and normals

The first free horizontal walk step is 4 world px forward or 3 backward, as in Super and Super Turbo. In the first phase of the 29-display walk from X 600 with Ken at X 768, Hyper Fighting advances 40 updates. Bison moves 120 px back or 144 px forward, with the last forward steps reduced by push. In Super, the same 29-display hold advances 29 updates and gives 87 px back or 116 forward. The three idle jump arcs first lift at +3 and peak 108/103/112 px above floor in neutral/forward/back. They first return to world Y 40 at +50/+49/+51. The movement rows retain the measured X displacement of each test and its camera and opponent conditions.

| Button | 38 px standing active / exit | 85 px standing active / exit | 54 px crouching active / exit |
| --- | --- | --- | --- |
| LP | +4–8 / +13 | +4–8 / +13 | +4–8 / +12 |
| MP | +7–11 / +20 | +7–11 / +20 | +5–9 / +18 |
| HP | +9–15 / +35 | +9–15 / +35 | +9–15 / +35 |
| LK | +5–9 / +14 | +5–9 / +14 | +5–9 / +14 |
| MK | +7–12 / +21 | +7–12 / +21 | +5–13 / +18 |
| HK | +9–15 / +33 | +9–15 / +33 | +7–36 / +53 |

Active interval endpoints are exclusive. The 38 and 85 px labels specify the fixture, since some Hyper Fighting close and far inputs dispatch the same attack. At 38 px MP lacks the +3 close attack phase measured in Super and both Super Turbo loadouts. LK at 38 px uses ID 6, also used at 85 px. Its hitbox center is 57 px forward and 61 px up, with radii 36 by 11. Super and Super Turbo use a separate, faster close LK at 38 px. Hyper Fighting's standing HK starts and exits two ticks later than in Super or Super Turbo, with box ID 67 and center 72 px forward, 64 up, radii 51 by 8. Its standing HP box extends 6 px farther forward and has 6 px more horizontal radius than the later versions. The crouching phase clocks agree with Super. Crouching LP has 11 px more forward offset than in Super.

Across the neutral standing and crouching matchups, the first direct strike damage or block loads 14 freeze in both fighters. Light/medium/heavy grounded reactions are generally +28/+33/+37 after contact, with 30/54/78 px horizontal recoil, as in the matched Super fixtures. Crouching sweep first hits at +7 and the sampled clean knockdown ends +96 from contact. The table of captures retains standing guard, crouch guard, and elevated tests. Aerial trials cover all six buttons in neutral, forward and back jumps, plus representative forward approach contacts. The aerial timing rows use jump entry as the origin, and their `delay` is the display offset at which the normal button was held.

## Charge specials

The horizontal parsers keep separate charge counts. Psycho Crusher loads 60 polls into `+0x81`; Double Knee loads 108 into `+0x89` at `$072566`. The release windows read from their tables at `$072458` and `$0725a2` are 8–15 polls. A two-display Forward lead after a pure Back hold generated reliable dispatches at matched distance. The tested Down-Back holds, including the 195-display control, selected ordinary attacks with the same final buttons. The two proofs are input recipes for this source. Bison actually begins the measured attack 3 px closer in the neutral pure-Back fixture because the walk shift precedes dispatch.

| Move | Dispatch write PC | First active whiff, light / medium / heavy | Special exits on whiff | Clean first contact |
| --- | --- | --- | --- | --- |
| Psycho Crusher | `$072526` | +13 / +13 / +13 | +61 / +65 / +63–64 | +13 at actual 105 px |
| Double Knee Press | `$07267c` | +8 / +9 / +10 | +34 / +36 / +38 | +8 / +9 / +10 at actual 61 px |
| Head Press | `$0727cc` | +30 / +30 / +30 | +74 / +74 / +74 | +30 at 85 px |

Hyper Fighting Crusher has an attack box centered on Bison's horizontal axis, 55 px up, with radii 81 by 15. Super and Super Turbo use 31 px forward, 55 up, and 44 by 15. All Hyper Fighting strengths start active at +13. Super and Super Turbo instead use +18/+15/+11. The Hyper Fighting neutral contact knocks down and ends Ken's reaction at +128. Each of the three strengths chips twice in the sampled stand-block and crouch-block cases. In Super and the sampled Super Turbo HP matches, heavy Crusher chips once.

Double Knee keeps the later releases' attack geometry and 14/14 contact freeze, but its two whiff attack poses use +8/+16, +9/+17 or +10/+18 by strength. The Super and Super Turbo comparison has +8/+16 for every strength. At actual 61 px the Hyper Fighting standing contacts are +8/+29, +9/+30 or +10/+31. Ken returns from the second clean hit in a grounded reaction. Super's and Super Turbo's second clean hit at nominal 64 px knocks down. HK reaches both hits against the sampled standing guard and one against crouch guard.

Head Press has the same first active tick and whiff exit as Super when Ken is held well above the attack. An elevated control at Y 250 can be contacted by Hyper Fighting Head Press and aerial normals, so the clean whiff uses Y 600. At 85 px LK hits at +30, with 14 freeze on both fighters. The attacker clears freeze two ticks later on the bounce. An LP follow-up starting at display `N+36` gives one hit in both phases, while `N+37` gives two. The second hit deals 19 HP at +93 in the first phase or +90 in the second. The special then exits at +109. The Up+Punch controls in Hyper Fighting yielded ordinary punches and no state-12 dive for any tested strength. Super and Old Super Turbo instead have the automatic Up+Punch attack. New Super Turbo requires a follow-up Punch on that separate route. The sampled later Nightmare command selects ordinary Double Knee in Hyper Fighting.

## Ground throw

Forward or Back + MP/HP captures at 38 px, while LP and all kicks select ordinary attacks in the same fixture. In both directions MP captures at 81 px and selects a normal at 82 px. The lock write reports PC `$0072a2`. Full throw releases at +40, Bison is neutral at +65, Ken takes 32 HP at +75 and is neutral at +149. The first Forward control shows the events after 28, 46, 53 and 106 displays, respectively. The corresponding Super control uses 40, 65, 75 and 149 displays. Neither freeze counter loads on throw capture, release or damage. The sampled Ken Left+MP responses include the +13 poll and leave damage and recovery on the full route. Super also gives full damage under this test. New Super Turbo Ken softens at the same +13 poll.

## Reproduction

Start MAME with the default `sf2hf` settings. Hold P1 Coin on displays 800–810, 840–850 and 880–890. Hold P1 Start on 920–930 and P2 Start on 975–985. Hold both LPs at 1040–1050, 1130–1140, 1220–1230 and 1300–1310. Move P1 Left at 1060–1063 and Down at 1072–1075 to select Bison, leaving Ken's second-player cursor in place. The controller addresses for directions/punches and system controls are `0x800000` and `0x800018`. Kick inputs use the MAME `:IN2` fields. Use combat input display `N=2200` or `2201`, with the same button hold lengths as the Super audit. Place Bison at X 552 and Ken at the fixture gap, fractions zero. For walks use X 600 instead, and for unarmed jumps use X 650. Tap Fighter 1 base `0xff83be` and Fighter 2 `0xff86be`, input `+0x292`, state `+0x03`, axis `+0x06/+0x0a`, animation `+0x1a`, freeze `+0x47`, life `+0x2a`, and throw `+0x63`.

The `pure_back` suite places the fighters at `N−4` for horizontal specials, holds Back from `N−125` to `N−3`, then holds Forward two displays before the button. `charged` is the longer Down-Back control, which places fighters at `N−2`. `control` uses the standard schedules, with horizontal charge sweep delays adding the named number of Forward-only displays before the button. `air` contains the separate aerial controls, where `n`, `f` and `b` in the button column encode the jump direction. `high` uses Y 600 to isolate the air whiffs. Raw captures are under `%TEMP%/ssf2-ryu-audit/sf2hf-bison-*`. The [write summary](bison-hyper-fighting-world-921209-writes.tsv) gives each measured game and display clock, actual gap, pose, box transitions and displacement. Contact fields are game tick, display, damage, both freezes, defender reaction ticks, defender X displacement, pose, and attack box. Fixed-point deltas are scaled by 65,536. The recorded input schedule distinguishes a failed special command from a missed attack. [Prior Hyper Fighting pacing fixture](2026-11-13-ryu-era-pacing.md#sfii-and-sfiii-arcade-runs)
