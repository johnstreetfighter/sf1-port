# Balrog in World Super Turbo

The source is Super Street Fighter II Turbo World 940223, parent set `ssf2t`, in MAME 0.289. The character select inputs pick New or Old Balrog (ID 10), with New Ken (ID 4) on P2. The trace checks the Old flag at fighter `+0x3b6` and records World Turbo 2 (`0xff83a9=2`, `0xff8cd3=0x60`). This is a source-side bonus challenger audit. [MAME set manifest](https://github.com/mamedev/mame/blob/mame0289/src/mame/capcom/cps2.cpp#L8152)

## Forward release

Timings start on the write entering special state 12. Hit and guard use a 90 px initial axis gap with both horizontal fractions zero. The high-target control uses a 190 px gap and holds Ken above the attack. Active intervals are inclusive at the start and exclusive at the end.

| Command | Contact / Balrog recovery, hit or standing block | High-target active interval / recovery | Standing hit damage, New / Old | Standing block chip, New / Old |
| --- | --- | --- | --- | --- |
| Forward + LP | +5 / +37 | [11,19) / +29 | 27–29 / 24–28 | 6 / 7 |
| Forward + MP | +5 / +37 | [12,20) / +30 | 29–31 / 26–30 | 6–7 / 7 |
| Forward + HP | +5 / +37 | [13,21) / +31 | 31–33 / 27–32 | 6–7 / 8 |
| Forward + LK | +7 / +35 | [11,19) / +25 | 22–26 / 25–26 | 4–5 / 6 |
| Forward + MK | +7 / +35 | [12,20) / +26 | 24–27 / 27 | 5–6 / 6–7 |
| Forward + HK | +7 / +35 | [12,20) / +26 | 27–31 / 30–31 | 5–6 / 7–8 |

Damage values are observed arcade HP ranges over two input phases. Forward + Punch selects Dash Straight. New HP and Old LP knock standing Ken down, reaching a peak 77 world px over floor and giving Ken his first reaction exit +134 after contact. Old LP also knocks down crouching Ken. The other standing hits keep Ken grounded.

New Dash Straight misses crouching Ken at 90 px in both the Down and Down-Back controls. Old Dash Straight hits crouching Ken at +5 and is blocked by Down-Back. Its Old standing and crouch-block reactions both last +36 from contact. The New Dash Straight first attack box has center 62 px forward and 74 px above Balrog’s axis with radii 49 × 7 px. Old retains 62 px forward, lowers its center to 61 px and extends the vertical radius to 16 px.

Forward + Kick selects Dash Upper. It is blocked standing and misses both crouching controls at 90 px in both loadouts. The attack box is the same across the tested strengths and loadouts, with center 56 px forward and 97 px high and radii 17 × 25 px. Crouching whiffs at this gap recover at +23 for Straight and +21 for Upper, demonstrating that proximity changes when the forward dash enters its attack pose.

## Down-Forward release

With charge held Down-Back and released at Down-Forward, New selects six additional special streams. In the 120 px control, Old enters an ordinary crouching attack and never enters special state 12 with any of the six buttons. The New low Straight beats standing guard and is crouch-blockable. The New knockdown Upper hits standing, is standing-blockable, and misses both crouching controls.

| New command | Standing contact / Balrog recovery | High-target first active / recovery | Standing hit damage | Guard chip |
| --- | --- | --- | --- | --- |
| Down-Forward + LP | +9 / +35 | +11 / +23 | 24–27 | 6 crouching |
| Down-Forward + MP | +10 / +37 | +16 / +29 | 26–27 | 6 crouching |
| Down-Forward + HP | +12 / +42 | +18 / +34 | 27 | 6 crouching |
| Down-Forward + LK | +6 / +32 | +8 / +21 | 24–27 | 6 standing |
| Down-Forward + MK | +7 / +34 | +13 / +26 | 26–29 | 7 standing |
| Down-Forward + HK | +8 / +36 | +14 / +28 | 27–31 | 6 standing |

The low Straight knocks Ken down on either standing or crouching hit. Its first attack box has center 56 px forward and 23 px high with radii 51 × 22 px. Its peak height is 55 px above floor and first reaction exit is contact +95. The knockdown Upper has center 65 px forward and 103 px high with radii 18 × 23 px. It lifts standing Ken 57 px above floor and exits reaction at contact +127. The 120 px fixture isolates the dash. At 90 px, the tested New diagonal LP input can land a crouching LP before the low Straight and alter its measured timing and reaction.

All contacts in the comparison load both freeze counters to 14. Balrog clears freeze at contact +14 and Ken at +15. Ken exits a grounded hit or block at contact +36 and travels 78 integer world px by that transition. These are whole contact-to-exit intervals, with freeze included. The 190 px high-target run measures free-motion attack pose timing with the defender above every sampled strike. It does not substitute for a grounded long-range contact measurement.

## Turn Punch

New and Old Balrog recognize all three punch buttons or all three kick buttons held together. Releasing any one button of an armed trio activates the move when Balrog can act. The two-button controls and the mixed LP+MK+HP control never arm. Holding all six for 80 displays, releasing Punches and then releasing Kicks after another 61 displays gives a level 1 followed by a level 2 in both loadouts. In the four replays, the second punch starts +77 through +79 from the first and hits for 34 HP.

The first sample of the complete trio initializes its counter to zero. Thirty further qualifying game updates arm it, so the first legal counter value is 30 and the previous value 29 fails. The observed input schedules needed 23–25 display samples to reach the threshold depending on loadout, button trio, and Turbo 2 input phase. The World New handler at `0x8F1E0` and `0x8F2E2` uses separate punch and kick counters at fighter `+0xb6/+0xb8`, capped at 32767. It selects the level with thresholds at `0x8F292` and stores its zero-based value at `+0xbc` on dispatch. The live Old replays reach the same levels from the same count ranges.

| Level | Qualifying updates after chord detection | Launch speed (px/tick) | Standing damage | Standing chip |
| --- | --- | --- | --- | --- |
| 1 | 30–120 | 8 | 24 New, 22–26 Old | 5–6 New, 6 Old |
| 2 | 121–240 | 9 | 34 | 8 |
| 3 | 241–480 | 10 | 40 | 10 |
| 4 | 481–960 | 11 | 47 | 11 |
| 5 | 961–1440 | 12 | 54 | 13 |
| 6 | 1441–1920 | 13 | 63 | 15 |
| 7 | 1921–2400 | 14 | 72 | 18 |
| Final | 2401 or more | 15 | 83 | 20 |

The World movement table at `0x8FF78` loads the listed speed and 0.375 px/tick deceleration in 8.8 units. Every level hits or blocks at +27 from state 12 in the 120 px standing controls. Both fighters’ freeze counters load 14. Ken’s counter clears at contact +15. Both fighters exit the grounded reaction or attack at +63. All eight levels keep standing Ken grounded, and the tested crouch and crouch-guard cases for levels 1, 4 and Final also stay grounded. The high-target whiff control gives attack interval [27,39) and neutral +49. The first attack box has center 59 px forward, 58 px above Balrog’s axis and radii 52×13 in both loadouts across the eight levels.

For the Turn Punch matrix, keep the same World Turbo 2 selection setup below. Let the charge start at display 2190, 2191 or 2192. Representative displayed holds for levels 1 through Final are 60, 140, 260, 460, 830, 1220, 1600, and 2010. Hold the full Punch or Kick trio through the display before release, then remove the trio. For a one-button release leave LP+HP or LK+HK held on the release display. Write timer byte `0xff8dce=0x99` on every display from 2071 through release +3 for the long controls. Reposition both fighters at release −1 with zero X fractions. Use X552 and X672 for contact, or a 190 px gap with Ken’s Y held at 250 from release −3 for whiff. Hold Ken’s Back or Down-Back from release −4 through +75 for block. Hold Down alone for the crouch control. Tap the count and level alongside the state, attack boxes, life, and both freeze counters using the game tick byte. The [Turn Punch capture summary](balrog-super-turbo-turn-punch-writes.tsv) records 322 distinct replays.

## Buffalo Headbutt

Both loadouts charge with Down and release with Up plus a punch edge. Down-Back also charges and Up-Forward or Up-Back also release. Holding Back alone fails. The command samples individual direction bits. New `$08edbe` and Old `$0a1c06` load `$41` into fighter `+$c1` on the first qualifying Down poll and arm after 65 further uninterrupted polls. The short-charge controls use 50–55 displayed samples, with first activation at 51 in phase 0 and 52 in phase 1 in both loadouts. The previous displayed duration fails in each phase.

After Down clears, New `$08edf0` and Old `$0a1c38` load a timer from the shared 32-entry RNG mapping at `$08ee0e/$0a1c56`. The possible loads are 8–15 ticks and Up is checked after the decrement. New `$08ee42` and Old `$0a1c8a` enter a separate punch window on Up. Press and release are both checked via `$0044d0/$0044de`. The window starts with counters `8,6` at `+$c3/+$c4`. With no punch edge on the Up poll, the parser checks punch before each decrement and lowers the second counter after subsequent polls +8, +10 and +12. These are the latest parser polls for HP, MP and LP respectively, subject to the separate action eligibility check at `$004542`. An unsuccessful edge on the Up poll can spend the first count immediately. The command dispatches as special 6 through `$08eede/$0a1d26`. Simultaneous Up+LP after the neutral gap enters special from the idle state on the sampled punch edge. An uninterrupted Down-to-Up input can enter a normal jump first and activate Headbutt later on punch release. These later activations are outside the clean timing comparison.

Both loadouts share the following whiff timelines in the high-target control. All ticks are relative to the state 12 entry write. The attack box changes once, with both intervals beginning on the stated tick and the last one ending before the inactive tick.

| Strength | First attack / second attack / inactive | Air flag / first rise / falling / landing | Neutral | Peak height / forward X |
| --- | --- | --- | ---: | --- |
| LP | +11 / +15 / +22 | +11 / +12 / +22 / +38 | +43 | 58 / 48 px |
| MP | +13 / +17 / +22 | +13 / +14 / +22 / +37 | +42 | 47 / 72 px |
| HP | +15 / +20 / +26 | +15 / +16 / +26 / +42 | +47 | 58 / 115 px |

The first attack has center `(53,53)` with half-width 8 and half-height 27 across strengths. The second attacks use `(41,60,22,24)` for LP, `(19,54,43,18)` for MP and `(19,49,43,23)` for HP, relative to Balrog’s axis and facing. New `$08fcc2` and Old `$0a25e0` store identical 8.8 initial motion entries `(vx,vy,ax,ay)` of `(0x0680,0x0c00,0x0058,0x0120)`, `(0x0800,0x0c00,0,0x0158)` and `(0x0a80,0x0c00,0,0x0120)`. Descent switches vertical acceleration to `0x0070` at New `$08fd1a`. [Whiff trajectory](balrog-super-turbo-headbutt-whiff-trajectory.tsv)

| Strength | Standing hit damage, New / Old | Crouching hit damage, New / Old | Standing / crouching chip | Balrog neutral on contact |
| --- | --- | --- | --- | ---: |
| LP | 13–16 / 16–17 | 16 / 19 | 4 / 4 | +57 |
| MP | 17–18 / 18 | 17–21 / 19 | 4 / 4 | +56 |
| HP | 20 / 19–20 | 19–21 / 18–19 | 4–5 / 4–5 | +61 |

Each standing or crouching hit or block occurs once at the first active tick and loads both fighters’ freeze to 14. Grounded clean hits knock Ken down. His first reaction exit is contact +134 across strengths. Standing and crouching guard keep him grounded and return him to guard-ready at contact +36. In the jump control LP contacts at +16 in both loadouts, and MP and HP contact at +20 New and +19 Old. New HP hits that jump with its second attack box and Old HP hits with the first. The jump timings describe those two arcade phases and their actual jump heights.

For the clean probe set Turbo 2, Balrog X552, and Ken X607 with zero X fractions at display `N−2`. Use `N=2200` or `2201`. Hold Down on `N−103…N−4`, then Up and the punch on `N…N+2`. For guard and crouch use Back, Down-Back, or Down on P2 from `N−4…N+110`. For jump use Up on P2 from `N−11…N−7`. The whiff control keeps Ken at Y250 from display 2195. The charge edge cases use the same three-display neutral gap, Ken X732 and the charge durations in the capture. Tap fighter `+$c0…+$c6`, state `+$03`, air flag `+$181`, animation pointer `+$1a`, freeze `+$47`, X/Y and defender life. The archived contact fields contain tick, HP lost, both freeze counters, Balrog X/height, Ken X/height and the active attack ID and geometry. [Buffalo Headbutt capture summary](balrog-super-turbo-headbutt-writes.tsv)

## Crazy Buffalo

New Balrog uses charge Back, Forward, Back, Forward and any punch or kick edge. Down-Back also charges. Down-Forward, Down-Back and Down-Forward satisfy the three release directions. Down alone fails the charge control, and a single Forward with the button gives an ordinary dash. All six buttons activate on a press or on release after a settled hold. The two-display pre-hold control starts another attack and fails the tested release setup.

New `$08ec30…$08ed9e` uses the facing-relative Back and Forward bits in fighter `+$3a6`. The initial Back poll loads `$46` into `+$d5`, then 70 uninterrupted Back polls arm the command at `$08ec66`. At Turbo 2, 55/56 displayed charge samples fail and 56/57 succeed in phases 0/1 respectively, before the three release directions. Once Back clears and after the accepted Forward and Back polls, `$08ec8e` loads separate 16–23 tick timers from the RNG-indexed table at `$08eca2` plus eight. The parser decrements each timer before testing its direction. The final Forward poll checks the edge immediately and seeds a 12-tick button timer. The later edge polls decrement that timer before the check, so the parser accepts the final Forward poll through +11 subject to action eligibility. The attack family goes to `+$c9`. Punch takes priority when both families appear on the same edge.

The guard at `$004038` checks the Old loadout flag before the super parser. At raw stock 48, New dispatches special 8 at `$08ed70` and spends the stock via `$027fae` on the same game tick. The first rush starts at +18 after the super pause. New stock 0, 47 and 49 controls give ordinary Dash Straight or Dash Upper. Old gives those ordinary dashes at all four stock values. The stock comparison at `$0045dc` requires exactly `$30`.

Without a held follow-up button, rushes two through four default to Straight after either opening. Holding LK from display `N+4` through `N+120` switches all three to Upper. Holding LK only on `N+35…N+54` and `N+65…N+85` gives Straight, Upper, Straight in those rushes. The ROM reads current buttons at `$08fc40` and uses the result with the rush index at `$08fb5c`. The final strike uses Turn Punch. The measured high-target whiff intervals are half-open.

| Input | First rush | Second rush | Third rush | Fourth rush | Finisher | Neutral |
| --- | --- | --- | --- | --- | --- | --- |
| Punch, no follow-up | Straight [23,31) | Straight [44,55) | Straight [60,69) | Straight [74,83) | [93,105) | +118 |
| Kick, no follow-up | Upper [25,33) | Straight [44,55) | Straight [60,69) | Straight [74,83) | [93,105) | +118 |
| Punch or kick, hold LK afterward | Same as initial button | Upper [45,55) | Upper [61,69) | Upper [75,83) | [93,105) | +118 |

Straight uses attack ID 60 and box `(62,61,49,16)`. Upper uses ID 61 and box `(56,97,17,25)`. The finisher uses ID 62 and box `(59,58,52,13)`. Each box lists forward center, height, horizontal radius and vertical radius. The phase-0 high-target sample travels 361.375 px on both openings. The grounded controls vary with distance as the engine can enter the strike early based on proximity during each rush.

| Opponent at requested gap 80 | Punch contacts | Kick contacts | Balrog neutral, Punch / Kick |
| --- | --- | --- | --- |
| Standing | +23, +47, +67, +87, +112 | +25, +47, +67, +87, +112 | +151 / +151 |
| Crouching | +23, +46–47, +66–67, +86–87, +111–112 | +40–41, +60–61, +80–81, +105–106 | +150–151 / +144–145 |
| Standing guard | +23, +50, +75, +97, +124 | +27, +50, +74, +96–97, +123–124 | +163 / +162–163 |
| Crouching guard | +23, +46–47, +68, +90, +117–118 | +40–41, +60–61, +80–81, +107–108 | +156–157 / +146–147 |

In the standing controls, Punch deals 80–81 HP and Kick 81–82 HP. Standing guard takes 20 chip on either route. Crouch guard takes 19–20 on Punch and 15–16 on Kick. The rushes load 6 attacker and 8 defender freeze. The finisher loads 14 each. The last clean grounded hit knocks Ken down for +133 through his first reaction exit. A guarded finisher leaves him grounded and guard-ready at contact +35. With actual entry gap 193.5 px the standing Punch route connects all five times and the Kick route connects four times. The controlled jumping cases vary by phase and appear in the capture.

Use the selection and Turbo 2 setup below. Set the requested axis gap at `N−17` with zero fractions and let `N=2200` or `2201`. Hold Back on `N−108…N−9`, Forward on `N−8…N−5`, Back on `N−4…N−1`, and Forward and the selected button on `N…N+3`. For release controls hold the button from `N−80` through `N−1` and finish with Forward alone. Write stock to `+$2b4` at display 2070. Hold P2 Back, Down-Back or Down on `N−15…N+160` for guard or crouch, Up on `N−11…N−7` for the jump, or hold Ken at Y250 from display 2195 for whiff. The recorded requested gap is the initial placement, which changes during the command. The archive retains the actual gap at state 12 entry, stock, attack-box changes, hits and first defender reaction exit. [Crazy Buffalo capture summary](balrog-super-turbo-crazy-buffalo-writes.tsv)

## Standing normals

New and Old Balrog share the measured whiff timing and attack boxes, with separate animation streams in the ROM. LP and LK each use the same stream across the tested gaps. MP and HK switch to far at 81 px, and HP and MK switch at 71 px. In both loadouts and both input phases, each move still uses close one 16.16 unit below its boundary. The guard and clean-hit outcomes were measured at requested gaps 38 and 96 against New Ken.

| Button | Close active / idle | Far active / idle | Close attack box | Far attack box |
| --- | --- | --- | --- | --- |
| LP | [4,8) / +13 | [4,8) / +13 | `1,49,84,39,8` | Same |
| MP | [6,10) / +17 | [7,11) / +18 | `4,65,100,20,23` | `11,92,99,30,24` |
| HP | [9,15) / +30 | [9,15) / +30 | `2,68,86,48,9` | `12,94,70,55,9` |
| LK | [4,8) / +13 | [4,8) / +13 | `6,63,87,26,10` | Same |
| MK | [5,9) / +16 | [6,10) / +17 | `5,41,53,29,10` | `13,64,53,31,13` |
| HK | [7,13) / +28 | [9,15) / +30 | `3,41,53,29,10` | `10,74,82,41,10` |

Offsets begin at the state 10 write at `$07b378`. Active intervals exclude their upper endpoint. Attack boxes give the ID, forward center, height, horizontal radius and vertical radius. All six hit standing Ken at first active in both spacings, and standing guard takes zero chip. Against settled crouch only MK at both gaps and HK at close connect. Crouch guard blocks those contacts. Every contact in the matrix loads 14 freeze for both fighters and delays Balrog’s idle return by 14 ticks. From contact, Ken’s hit or guard exit is +27 light, +32 medium and +36 heavy. His corresponding X displacement is 30, 54 and 78 px. Both LP and LK deal 4 HP in the sampled standing controls. Standing MP, HP, MK and HK give 21–23, 26–29, 22–25 and 27–31 HP across sampled loadouts and phases.

For reproduction use the selection below, then apply Turbo 2 and X positions at `N−2`. Use `N=2200` or `2201`, and hold the chosen attack on `N…N+2`. Set X552 for Balrog and X552 plus the desired gap for Ken. For a boundary probe write Ken’s fractional X word to 65535 and leave Balrog’s fraction at zero. For clean whiff timing hold Ken at Y250 starting at `N−2`. For standing guard hold P2 Back on `N+1…N+120`; for crouch guard and idle crouch hold Down-Back or Down on `N−12…N+120`. Tap attacker state and animation as below. Use the life write at `$0be64c` for hits and defender freeze `$0bed22` for zero-chip blocks. The contact fields retain tick, damage, both freezes, both X axes and the attack box. [Standing normals capture](balrog-super-turbo-standing-writes.tsv)

## Crouching normals

At the two tested gaps (38 and 96 px), New and Old share crouching attack selection, whiff timing and geometry. Timings start at the state 10 write. Active intervals exclude their upper endpoint. The attack box form is the same as in the standing table.

| Button | Whiff active / idle | New / Old first animation | Attack box |
| --- | --- | --- | --- |
| LP | [3,7) / +11 | `$161066 / $1f25ec` | `38,48,54,26,9` |
| MP | [5,9) / +16 | `$1610ca / $1f2650` | `20,51,53,40,9` |
| HP | [6,8) then [8,12) / +29 | `$16115e / $1f26e4` | `29,35,54,23,20` then `9,65,103,18,23` |
| LK | [3,7) / +11 | `$1611f2 / $1f2778` | `39,36,34,22,19` |
| MK | [6,12) / +16 | `$161256 / $1f27dc` | `40,50,22,30,19` |
| HK | [9,15) / +33 | `$1612d2 / $1f2858` | `41,56,23,51,22` |

All six connect once against either standing or crouching Ken at 38 px on the first active tick. At 96 px, LP and MK connect against settled crouch and miss standing. MP connects in all four grounded postures. HP connects standing at +8 with the second box and misses crouch. LK whiffs all four controls. HK connects in all four controls. MK and HK hit standing Back and are blocked by Down-Back where they reach. Both guards block the other normals where they reach and take no chip.

All contacts load 14 freeze on each side and add 14 ticks to Balrog’s idle return. Representative damage across the two phases and loadouts is LP/LK 4, MP 21–23, HP 26–29, MK 22–25 and HK 25–31. Grounded hit or block sends Ken 30, 54 or 78 px with reaction exit at contact +27, +32 or +36 by strength. HK clean knocks down, reaches 55 px above the floor and travels 80 px through first exit at +95. At 48 px against Ken’s fixed jump, HP hits at +8 with its second box for 26–29 HP. Ken’s sampled absolute peak height is 146–147 px after the air hit, his first reaction exit is +69, and his X advances 106 px. HK misses the same jump.

Use the standing fixture, hold P1 Down on displays `N−12…N+60`, and the attack on `N…N+2`. For the extra 48 px jump control hold Ken Up on `N−10…N−6`. The [crouching capture](balrog-super-turbo-crouching-writes.tsv) records 248 cases, including X and peak Y for the measured defender reaction. The peak column is floor-relative, and the defender exit tick is relative to Balrog’s state 10 entry.

## Aerial normals

Both loadouts lift at +4 from the jump state 4 write at `$07b482`. Neutral, forward and backward first land at +48, +47 and +49 and reach idle at +55, +54 and +56. Their measured peaks are 77, 74 and 80 px, with X displacements of 0, +112.6640625 and −103.88671875 px. New neutral HP uses a special air-attack route through state 10 at PC `$08f63a` and clears its air flag at +49, with idle still +55. Old neutral HP uses the ordinary jump attack substate and lands at +48 in the whiff control.

Each button selects a separate neutral and directional stream. Forward and backward share the directional attack stream. Both loadouts share dispatch-relative active timing and hitbox geometry. The fixed whiff schedule dispatches at jump +10 New and +11 Old. For the following intervals, zero is attack dispatch and the endpoint is excluded.

| Button | Neutral / directional active | Neutral box | Directional box |
| --- | --- | --- | --- |
| LP | [2,34) / [3,33) | `42,42,49,30,14` | `21,44,66,21,16` |
| MP | [6,20) / [4,19) | `43,43,60,32,14` | `22,46,53,23,16` |
| HP | [8,18) / [9,21) | `44,43,60,32,18` | `23,31,37,28,17` |
| LK | [2,34) / [2,34) | `45,42,49,30,14` | Same |
| MK | [4,18) / [4,18) | `46,42,49,30,14` | Same |
| HK | [8,19) / [9,21) | `47,31,37,28,17` | `50,31,37,28,17` |

In the 105 px late forward-jump probe, all six attacks hit standing Ken in both loadouts and are blocked by standing guard with zero chip. Contact relative to jump entry is LP +41, MP +38, LK/MK +37 and HP/HK +36 New or +37 Old. Clean standing hits give reaction exit +27 from contact for every strength. Standing guards give +27, +32 and +36 by strength. HP/HK connect at +40 against settled crouch and Down-Back, with 23–25 damage and reaction exit +36. LP/MP/LK/MK whiff against both crouch controls at this timing. The neutral LP/HP/HK hits at 42 px give exit +27 while guard exits take +27 for LP or +36 for HP/HK. The sampled backward LP/HP/HK pass at 42 px whiffs. All contacts load 14 freeze on each side. Net X in the contact capture includes movement from overlapping pushboxes as well as reaction recoil.

Set the starting positions at display `N−2` as in the standing fixture. Use `N=2200`. Hold P1 Up, Up-Forward or Up-Back on `N…N+2`. For whiff set Ken 200 px away and hold his Y at 250. Press the attack on `N+8…N+10`. For contact, use these requested gap and display delay pairs: forward 105/+21, forward 120/+25, neutral 42/+23, backward 42/+19. Hold the attack for three displays. For standing guard start P2 Back one display after the first attack display. For crouch and crouch guard start Down or Down-Back at `N−12`. Hold those inputs until `N+90`. The [aerial capture](balrog-super-turbo-air-normal-writes.tsv) records 140 cases with attack pointers, absolute jump-clock timings, and post-contact defender exit and net X.

## Headbutt hold

Forward or Back plus MP or HP enters the hold against grounded New Ken in both Balrog loadouts. LP, LK, MK and HK are negative controls at the same 36 px actual gap. Both directions capture at a sampled 80 px axis gap and fail at 81 px. The shared capture writes holder `+$63=1` and target `+$63=255` at `$004772/$004778`. The damage helper at `$0049b6` writes 28 HP at capture +15, then 4 HP per further damage write. With both controllers idle those writes recur every 36 ticks.

| Loadout | Idle damage writes / total | Hold releases and Balrog idle | Ken can act / neutral |
| --- | --- | ---: | --- |
| New | 9 / 60 HP | +318 | +364 / +370 |
| Old | 12 / 72 HP | +444 | +491 / +497 |

The terminal path clears the throw flags at `$004e44/$004e48` and puts Ken in reaction state 14 at `$004ce0`. The earliest subsequent action can start when that state clears to turning state 4. One isolated Ken Forward+MP pulse at each tested displayed offset in `N+{5,10,11,12,13}` gives the same early hold cadence as idle in both loadouts. Repeated direction and attack edges can break the hold. In the long mash schedule, New releases at +74/+84 across phases and Old at +112/+75, after two to three damage writes. The holder’s alternating directions and punches can shorten damage intervals in both loadouts. The recorded schedules do not establish the underlying mash count or all possible cadence bounds.

For the capture use the same setup and placement as the standing matrix, then send Forward or Back and MP or HP together on display `N` only. The requested close gap is 34 px and the pushboxes settle it to 36 px before capture. Ken mash alternates `r2 lp2` from `N+18` every eight displays with `l2 hp2` from `N+22` every eight displays, each pulse held for three displays. The long control stops scheduling new pulses before `N+183`, and the short control before `N+84`. Holder mash uses `r1 lp1` from `N+22` and `l1 hp1` from `N+26`, both every eight displays until `N+183`. In the archive response 0 is idle, 97 is short defender mash, 98 is holder mash and 99 is long defender mash. Other response values are offsets of the isolated Ken Forward+MP. Damage ticks extend across the game tick byte’s wrap. [Headbutt hold capture](balrog-super-turbo-hold-writes.tsv)

## Reproduction

Reuse the World `ssf2t` default EEPROM, `qsound_hle`, and the `trace.lua`/`events.txt` input format from local audit commit `9646b61`. Start with these selection rows:

```text
1250 1255 coin1
1270 1275 coin2
1300 1305 start1
1340 1345 start2
1400 1405 lp1 lp2
1444 1446 l1
1452 1454 l1
1470 1475 lp1 lp2
1540 1545 lp1 lp2
```

For Old Balrog insert:

```text
1478 1479 r1
1482 1483 l1
1486 1487 l1
1490 1491 r1 lp1
```

At display 2070, apply the Turbo 2 bytes and place Balrog at X552, Ken at X552 plus the specified gap, with both X fractions zeroed. Let `N` equal 2200 or 2201. Hold `d1 l1` on `N−129…N−1`, then `r1` plus the button on `N…N+2`. Add `d1` to the release rows for the Down-Forward controls. For standing guard or crouch guard hold P2 `r2` or `d2 r2` on `N+1…N+120`. For settled crouch hold `d2` on `N−10…N+120`. In the high-target whiff control set Ken’s Y to 250 at display 2190 and on each subsequent display. The stage floor is Y40.

Timestamp writes with the game tick at `0xff801d`. Tap each fighter’s state at `+0x03`, animation pointer at `+0x1a`, freeze at `+0x47`, world X at `+0x06`, Ken’s Y at `+0x0a`, and Ken’s life at `+0x2a`. Read the attack box from the animation’s attack ID at `+0x0c` and table at fighter `+0x34`. Match the first nonzero attack ID to its first write and measure until the first zero ID. Count contact from Ken’s freeze load and reaction exit from the first transition out of state 14. [Attack-box layout](https://github.com/peon2/fbneo-training-mode/blob/master/hitboxes/st-hitboxes.lua)

The dash matrix includes 120 Forward and 120 Down-Forward controls across two phases, six strengths and both loadouts. The [dash capture summary](balrog-super-turbo-dash-writes.tsv) retains the attack-box geometry and tick boundaries for every case. Shortest dash charge, actual dash input-to-action delay, further opposing hurtboxes, counter-hit properties, Turn Punch projectile avoidance and buffered release, Headbutt and Crazy Buffalo invulnerability, normal cancels, the hold’s mash algorithm, and the port comparison remain open.
