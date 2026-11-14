# Vega’s wall specials in World Super Turbo

The source is Super Street Fighter II Turbo World 940223 (`ssf2t`) in MAME 0.289. P1 is New or Old Vega (ID 11) and P2 is New Ken (ID 4). The trace verifies the fighter IDs, Old flag at fighter `+0x3b6`, and World Turbo 2 bytes `0xff83a9=2` and `0xff8cd3=0x60`. The stage is Ken’s dock. Timings below count advancing game ticks from Vega’s entry into special state 12. [MAME set manifest](https://github.com/mamedev/mame/blob/mame0289/src/mame/capcom/cps2.cpp#L8152)

## Flying Barcelona Attack

With Vega at X552 and Ken at X642, both loadouts enter the launch phase at +4 after releasing a Down charge with Up+Kick. Holding no horizontal direction sends Vega to the left wall for all three launch kicks. Up-Right+LK with Right held through display `N+4` still picks the left wall. Keeping Right through `N+5`, the display interval containing the launch-phase transition, sends Vega to the right wall in both loadouts and phases. In either direction he stays attached for four game ticks, then rebounds. Without a follow-up, the route has no attack box or hit.

| LK launch control | Latch / rebound | First landing / neutral | Vega’s latch X,Y |
| --- | --- | --- | --- |
| Up or Up-Left with Left held | +46 / +50 | +91 / +110 | 336,76 |
| Up-Right with Right held | +27 / +31 | +72 / +91 | 853,100 |
| Up with Ken initially at X462 | +37 / +41 | +82 / +101 | 287,96 |

Latch coordinates are sampled just before the wall placement write. The baseline left and right rebound positions are (339,76) and (845,100), respectively. The swapped-position control moves the camera and still chooses the left wall when Vega is facing left. Both trajectories continue through a small landing hop before neutral.

With the baseline left-wall flight, a punch input at display `N+62` starts the strike at game tick +75 through +77 depending on phase. The first attack box follows three advancing ticks later. Its raw center offsets are (13,40) with radii (86,18), shared by the three punch strengths. All three punch follow-ups hit standing Ken at +83 and restore Vega to neutral at +125. Ken is knocked down and exits his reaction at +217, or 134 ticks after contact. Late standing guard blocks LP at +83 for 6 chip, ends its block reaction at +119, and leaves Ken 78 integer world px from the contact position. LP misses late crouching guard and settled crouch in this spacing and timing, and Vega returns to neutral at +117. The guard comparison starts late to keep the first wall latch unchanged.

| Follow-up | Standing damage, New | Standing damage, Old |
| --- | --- | --- |
| LP | 28–29 | 27 |
| MP | 30–31 | 28–29 |
| HP | 32–33 | 30–31 |

## Izuna Drop

At the same 90 px initial gap, Up+LK followed by Right+LP on displays `N+71…N+72` captures in both loadouts and both phases. The same input at `N+70` produces a mix of grabs and ordinary strikes. At that display the first game tick sampled and the fighter coordinates vary by phase. For example, a New Vega input sampled at +87 captured with 38 px horizontal and 35 px vertical separation. Old Vega at the same coordinates and tick also captured. In the Old phase 0 `N+70` control, the input was sampled at +85 at (589,90) versus Ken at (642,40). That attempt selected the strike and hit at +88.

| Matched capture +87 | Game tick | World positions |
| --- | --- | --- |
| Ken throw flag set to 255 | +87 | Vega (604,75), Ken (642,40) |
| Slam, 34 arcade HP, Ken throw flag cleared | +125 | Vega (604,40), Ken (654,10) at the slam write |
| Vega returns to neutral | +151 | Vega (604,40) |
| Ken exits throw knockdown | +227 | Ken (758,40) |

The capture and slam load neither ordinary freeze counter. Tested captures at +86 through +88 share the +125 slam and +151 Vega recovery. At +89 the sampled pair (613,59) and (648,40) slams at +133 and Vega recovers at +159. At +90 the sampled pair (617,50) and (652,40) slams at +135 and Vega recovers at +161. New phase 0 also captures with Left+LP, Right+MP, or Right+HP at `N+70`, with the same +87 grab, 34 damage, and +151 recovery. A punch without a horizontal direction at that timing can strike. A second LK at `N+62` leaves the ordinary rebound unchanged in the sampled phase.

## Sky High Claw

Down charge followed by Up+Punch selects Sky High Claw in both loadouts. In the same 90 px fixture, all three strengths latch the left wall at +46, rebound at +50, and first expose an attack box at +51. The first box has raw center offsets (−62,62) and radii (33,10). The LP version travels across at a different height and misses neutral Ken in this fixture.

| Release | Standing contact | Vega’s first landing / neutral | Standing hit damage |
| --- | --- | --- | --- |
| Up+LP | Whiff | +104 / +116 | — |
| Up+MP | +66 | +127 / +139 | 16 New, 16–17 Old |
| Up+HP | +66 | +125 / +137 | 16 New, 16–17 Old |

Standing Ken exits the grounded reaction +36 after contact. HP also hits crouching Ken at +65 for 14–15 New and 15 Old. Standing guard blocks HP at +66, and late crouching guard blocks at +65. Both guards take 3 chip and the grounded block reaction exits +36 after contact. In the tested guarded fixtures Ken moves 78 integer world px in that reaction. All measured strike hits and blocks load both freeze counters to 14. Vega’s counter clears at contact +14 and Ken’s at contact +15.

## Rolling Izuna Drop

New Vega can charge Down-Back, move to Down-Forward then Down-Back, and release with Up-Forward+Kick. The opening flight keeps the 48-point meter full. A successful horizontal direction plus punch capture spends all 48 on the next game tick and starts the 18-tick Super flash. A strike or an empty rebound keeps the meter full. Old Vega with the same forced full meter and New Vega with 47 meter both produce ordinary Izuna Drops in the captured controls. The charge motion also worked with Up or Up-Back on release. Omitting the second Down-Back selected ordinary Izuna in the matched successful capture control.

In the baseline 90 px left-wall route, a Right+LP follow-up at displays `N+64…N+67` gives an ordinary strike, `N+68…N+72` captures, and `N+73` or `N+74` leaves an empty rebound. Each offset refers to the first display of a separate two-display press. These are tested bounds for this placement and input phase. All three kicks launch the same measured Super at the `N+69` capture schedule. Right+MP, Right+HP and Left+LP also capture in the `N+70` and `N+71` controls. LP alone at those late offsets starts a strike that whiffs. The `N+69` Right+LP route captures late standing guard, late crouching guard, and settled crouch.

| Left-wall Right+LP input | Grab | Slam 1 / 2 / 3 | Vega neutral / Ken reaction exit |
| --- | --- | --- | --- |
| `N+68` or `N+69` | +86…+88 | +144 / +205 / +266 | +292 / +369 |
| `N+70` | +89 | +152 / +213 / +274 | +300 / +377 |
| `N+71` or `N+72` | +90…+91 | +154 / +215 / +276 | +302 / +379 |

In the baseline `N+69` neutral-Ken replays, both first slams deal 20 arcade HP each and the third deals 29 or 30. The first two slams write Vega’s ordinary freeze counter to 12 and overwrite it to 6 on the same tick. Ken remains held by the throw flag, with no ordinary contact freeze. The third slam clears that flag and starts Ken’s state 20. Vega returns to neutral 26 ticks after release. The `N+69` phase 1 capture starts at Vega (604,75), Ken (642,40). At the three life writes the respective Vega/Ken axes are (604,40)/(654,10), (691,40)/(740,14), and (763,40)/(813,14).

For the right-wall control, keep Right through `N+12` after the same launch. Vega latches at +27 and rebounds at +31. Left+LP at `N+54` grabs at +69, slams at +125/+186/+247 and returns Vega to neutral +273 in both phases. Left+LP at `N+56` also captures in both phases, with different heights and slam times.

To reproduce, use the same fighter selection and X placement below. Fill Vega’s meter at fighter `+0x2b4` on display 2070. For motion `a`, hold `d1 l1` on `N−129…N−9`, `d1 r1` on `N−8…N−5`, `d1 l1` on `N−4…N−1`, then Up, Up-Left or Up-Right plus the specified Kick for two displays starting at `N`. Motion `b` uses Right, Down-Right, Down, Down-Left and Left for 2/2/1/2/1 displays after charge, followed by the same release. Motion `c` holds Down-Right on `N−8…N−1` after charge. Motion `hold` repeats `a` and holds Right through `N+12`. The follow-up is a two-display hold. Use the Barcelona guard rows below for late guard and settled crouch.

Tap the Super flag `+0x1fa`, meter `+0x2b4`, Ken’s throw flag `+0x63` and life `+0x2a`, and both ordinary freeze counters `+0x47`. Unwrap the 8-bit game clock through the third slam and both recovery writes. The [Super capture summary](vega-super-turbo-super-writes.tsv) records 180 runs, including 46 Super captures.

## Claw loss and recovery

The World 940223 hit and block reaction handler calls the claw routine at `0x7AE8C`. While equipped, the first eight qualifying reactions increment Vega’s byte `+0x162` to 8. Every later qualifying reaction rolls the shared random routine `0x2E96` and drops the claw when the low nibble is 5 or 13. The `0xDFDF` bit test admits the other values without dropping. The first eight qualifying contacts are safe, and the count then stays at 8 until reset. LP, MP, HP, LK and HK each incremented this count in matched clean-hit and block fixtures for New and Old Vega. The earliest observed drop was contact 9. Several controls reached 20 contacts with the claw still equipped.

Loss clears `+0x14c`, spawns actor `0x28` at Vega’s axis plus 80 world px vertically, and changes fighter hitbox table pointer `+0x34`. New switches `0x19CD08` to `0x19CD12` and Old `0x220ED0` to `0x220EDA`. The New object can settle on the floor. The pickup routine at `0x4AB7C` checks once every four game ticks, while the owner is active and neutral, for inclusive world-axis separations of at most 24 px horizontally and 16 px vertically. It clears the counter and restores the equipped flag and original table. The Old actor uses a separate timed bouncing path and bypasses that pickup routine.

In the New HK guard fixture, contact 14 drops the claw at display 2833, tick 65. The object spawns at (527,120) and settles at (366,39) by tick 125. Hold Left beginning at display 2880. The game picks up the claw at display 2896, tick 146, with Vega near X387. The same replay without the walk remains unequipped through the observed interval. In the Old MP guard fixture the claw drops on contact 9 at display 2589, tick 5, and the actor travels offscreen and disappears.

### Mask and incoming damage

Vega retains his mask after a natural claw drop in World 940223. The World Turbo 2 captures show the face mask on both New and Old Vega after the loss and reaction. The match reaction call at `0x7AD8A` reaches the equipment routine at `0x7AE8C`. When the claw flag is clear, the routine returns immediately. Its only equipment change clears that flag, spawns claw actor `0x28`, and swaps the hitbox table. The sprite overlay loader at `0x7805A` uses the same claw flag and animation byte `+0x15`. The mask stays in Vega’s regular pose after the separate claw overlay is suppressed. The audited match paths provide no second equipment state or mask loss.

Matched hits on full-life New and Old Vega give identical incoming damage with and without the claw in two input phases. For Ken’s standing LP and HP, setting RNG word `0xff82d0` to `0x1987` on each display from 2190 through 2225 isolates the normal random damage variation. LP deals 4 HP and HP deals 27 HP in every paired control at 32 px. At the same gap, Ken’s Forward+MP shoulder throw deals 32 HP in both claw states with the normal random stream. HP Hadoken at 90 px deals 19 HP in both claw states, also with the normal random stream.

The melee and projectile life writes follow attack-record damage lookup, reaction checks, random damage adjustment, and low-life scaling. Their damage paths reach `0xBE646` and `0xBEBA6` without an equipment multiplier. Ken’s shoulder subtracts the pending throw damage at `0x7AB6E`. These paths and the paired probes provide no basis for a mask-loss damage multiplier in this arcade revision.

To replay the damage pairs, use the armed and unequipped table setup from the claw replay below with Ken as attacker. At `N = 2200` or 2201, hold `lp2`, `hp2` or `l2 mp2` for two displays at 32 px. For HP Hadoken at 90 px, hold `d2` on `N−4…N−3`, `d2 l2` on `N−2…N−1`, and `l2 hp2` on `N…N+2`. Tap Vega’s life at `+0x2a`, equipment flag at `+0x14c`, and hitbox table at `+0x34`. To check the mask after a natural drop, replay the New HK and Old MP block fixtures without the New pickup walk, then capture the rendered pose after reaction recovery. [MAME screen snapshot API](https://docs.mamedev.org/luascript/ref-devices.html)

### Attack and movement effects

The matched attack controls use the precise hitbox table selected by the loss routine. Damage ranges are the observed arcade HP in two input phases across both loadouts. Box centers below are raw table offsets, followed by horizontal and vertical radii.

| Move and initial gap | Equipped first box → unequipped | Equipped damage → unequipped | Contact / neutral |
| --- | --- | --- | --- |
| Far standing MP, 100 px | (−96,67; 59×6) → (−80,67; 51×6) | 17 → 16 | +8 / +38 |
| Crouching MP, 80 px | (−100,54; 58×7) → (−90,53; 42×4) | 15–16 → 14 | +4 / +32 |
| Far standing HP, 100 px | (−46,57; 31×10) in both states | 22–24 → 21–22 | +12 / +45 |
| Flying Barcelona LP, 90 px | (13,40; 86×18) → (−1,40; 73×18) | 27–29 → 19–22 | +83 / +125 |
| Sky High Claw HP, 90 px | (−62,62; 33×10) in both states | 16–17 → 13–14 | +66 / +137 |
| Rolling Crystal Flash HP, 90 px | First roll (−15,32; 17×18) in both states | Final hit 29–32 → 21–22 | +20/+50/+81 / +108 |

Standing LK at 48 px keeps its attack box and contact and recovery timing. The walking controls give 4.125 px forward and 3.25 px backward per advancing game tick in both claw states and loadouts. The tested quarter-circle-back LP motion starts an ordinary standing LP in both states and loadouts. These results describe the arcade behavior. [Claw capture summary](vega-super-turbo-claw-writes.tsv)

### Claw replay

Use the same selection and Turbo 2 bytes as below. For the repeated-contact matrix, place Vega at X552 and Ken at X584 with both X fractions zero at display 2070. At displays `2198 + 48i`, restore those axes, Vega’s life words `+0x2a/+0x2c` to 144 and stun word `+0x5c` to zero. Set `N = 2200 + 48i` and press Ken’s chosen normal on `N…N+1`. The block controls hold Vega’s Left on `N−1…N+22`. For natural New pickup, use HK blocks up to and including `N = 2824`, stop the placement/HP writes after drop display 2833, then hold Left on 2880–2963. The untouched control omits that walk. For Old’s actor control use MP blocks up to `N = 2584`.

For isolated attack controls, use Vega X552 and Ken X552 plus the listed gap. At display 2070, the unequipped arm clears `+0x14c` and writes the measured unequipped pointer to `+0x34`. Use `N = 2200` or 2201. Hold the normal for two displays beginning at `N`, with crouch controls holding Down from `N−10`. Rolling Crystal Flash holds Down-Back on `N−129…N−1` and Forward+HP on `N…N+2`. Barcelona and Sky High Claw use their reproduction rows below. For walk hold Forward or Back on `N…N+7`. The quarter-circle-back control holds Down, Down-Back, and Back+LP on consecutive two-display intervals ending at `N+1`. Tap equip/count, attack records, life and movement, and timestamp with `0xff801d`.

## Reproduction

Use World `ssf2t`, its default EEPROM, and `qsound_hle` with the `trace.lua` and `events.txt` input format from local audit commit `9646b61`. Start with these selection rows:

```text
1250 1255 coin1
1270 1275 coin2
1300 1305 start1
1340 1345 start2
1400 1405 lp1 lp2
1444 1446 l1
1452 1454 l1
1460 1462 d1
1470 1475 lp1 lp2
1540 1545 lp1 lp2
```

For Old Vega, insert these rows:

```text
1478 1479 l1
1482 1483 r1
1486 1487 r1
1490 1491 l1 lp1
```

At display 2070 set the Turbo 2 bytes and write Vega’s X to 552 and Ken’s X to 642 (462 in the swapped control), with both X fractions zero. Let `N` be display 2200 for phase 0 and 2201 for phase 1. Hold `d1` on `N−129…N−1`, then `u1` plus the tested button on `N…N+1`. For a held wall direction add `l1` or `r1` on `N…N+12`. The `rightto2` through `rightto7` controls hold Right through the corresponding display offset. Hold each follow-up for two displays beginning at the indicated offset. For Barcelona’s late guard controls hold P2 `r2` or `d2 r2` from `N+61` through `N+130`. For Sky High Claw use the same guard inputs from `N+49` through `N+130`. Settled crouch holds `d2` from `N+1` through `N+130`.

Timestamp writes with the wrapping game tick at `0xff801d`. Tap each fighter’s state at `+0x03`, substate at `+0x04`, integer world X and Y at `+0x06` and `+0x0a`, animation pointer at `+0x1a`, freeze at `+0x47`, throw flag at `+0x63`, and controller word at `+0x392`. Tap Ken’s life at `+0x2a`. Read Vega’s attack ID from animation `+0x0c` and box data from fighter table `+0x34`. The [capture summary](vega-super-turbo-wall-writes.tsv) records 128 selected runs and exact boundaries. [Attack-box layout](https://github.com/peon2/fbneo-training-mode/blob/master/hitboxes/st-hitboxes.lua)

The shortest charge, broader grab ranges, wall routes from Vega’s stage, further opponent hurtboxes, other attacks affected by claw loss, and the port comparison remain open.
