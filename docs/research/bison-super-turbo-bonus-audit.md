# M. Bison in World Super Turbo

New and Old M. Bison were selected through the character screen in Super Street Fighter II Turbo World 940223, using MAME 0.289. The trace verified fighter ID `8`, the requested Old flag at player `+0x3b6`, and New Ken (ID `4`) in both cases. MAME verified the local `ssf2t` ROM set. The port’s native M. Bison work is tracked by #30. [MAME manifest](https://github.com/mamedev/mame/blob/mame0289/src/mame/capcom/cps2.cpp#L8152) · [Wave 1 M. Bison](https://github.com/johnstreetfighter/sf1-port/issues/30)

## Exact contacts at 64 world pixels

Ticks in this section start with the transition to Bison’s special state `12`. The exact write taps cover both loadouts, all three kick strengths, and two consecutive input phases. Ken stands neutral, holds Back, holds Down-Back, or holds Down alone. Each clean two-contact run hits at +8 and +29. Ken’s combo counter increments on the game tick after the second hit for both loadouts. This resolves the apparent New versus Old difference in the first frame-end capture.

| Kick | Clean standing damage, New / Old | Standing guard chip, New / Old | Crouch guard chip, New / Old | Bison exits special on clean hit | Clean Bison X at exit |
| --- | --- | --- | --- | ---: | ---: |
| LK | 19–20 + 11 / 19 + 11 | 5 + 2 / 4 + 2 | 5 + 2 / 4 + 2 | +62 | 661 |
| MK | 21–22 + 11 / 21 + 11 | 5 + 2 / 5 + 2 | 5 + 2 / 5 + 2 | +63 | 682 |
| HK | 22–23 + 11 / 22 + 11 | 5 + 2 / 5 + 2 | 5 only / 5 only | +64 | 703 |

Damage is in arcade HP and the ranges are observed samples. Settled crouching Ken without guard also takes both hits and the knockdown for all strengths. Standing guard blocks both contacts at +8 and +29 and takes no knockdown. Crouch guard blocks both contacts at +8 and +29 against LK and MK. Against HK, only the +8 contact blocks at 64 px. Bison crosses Ken and exits at +51, world X 784, with Ken at X 686. The standing guard exits and two-block crouch guard exits use the same attacker timing as the clean column. At 64 px standing guard leaves Bison at X 685, 706, and 727 for LK, MK, and HK respectively.

Both fighters’ freeze counters load 14 on every hit or block in these runs. On the first contact, Bison’s counter clears at contact +14 and Ken’s at +15. On a second contact while Ken is in reaction, both counters clear at contact +14. Ken exits a two-block reaction +35 after the second contact. In the HK one-block crouching fixture, Ken exits the reaction +36 after the first contact. The second clean hit knocks Ken down. His peak is 77 world px above the floor, and he exits reaction +133 after the second contact. His horizontal landing position depends on spacing and the stage bounds.

The Forward+Kick press can enter a normal state before the special begins. At 64 px that state produces no earlier damage in the audited runs. At 48 px, Old LK hits for 11 HP before the special, so that case includes an extra contact. At 190 px, both loadouts’ LK and MK land only the second strike at special +23 and +19, respectively. That standalone second strike knocks down and Ken exits reaction +134 after its contact. HK at 190 px hits twice, at +14 and +32.

## Animation and attack boxes

The table gives the attack pose writes on a whiff, measured from the special state transition. The first attack box has center 27 px forward and 65 px above Bison’s axis, with radii 44×15. The second has center 23 px forward and 25 px above the axis, with radii 29×22. The geometry matches across all three strengths and both loadouts. The 64 px two-contact trials reach the second attack pose at +29 after first-hit freeze. [ST hitbox table layout](https://github.com/peon2/fbneo-training-mode/blob/master/hitboxes/st-hitboxes.lua)

| Kick | New first / second attack pose | Old first / second attack pose | Attack IDs |
| --- | --- | --- | --- |
| LK | `0x154e80` / `0x154e98` | `0x1e6cfe` / `0x1e6d16` | 4 / 5 |
| MK | `0x154f2c` / `0x154f44` | `0x1e6daa` / `0x1e6dc2` | 47 / 48 |
| HK | `0x154fd8` / `0x154ff0` | `0x1e6e56` / `0x1e6e6e` | 49 / 50 |

All six whiffs write poses at +0, +3, +5, +8, +16, and +26. The last recovery pose writes at +35, +36 and +37 for LK, MK and HK respectively, one tick before exit. In the whiff control, Ken remains at X 742 and is held above the strike. From +5 through +25, Bison takes 21 horizontal steps. The first is 6.0625, 8.0625, or 10.0625 world px for LK, MK, or HK, then each successive step increases by 0.0625 px. Total free travel is 140.4375, 182.4375, or 224.4375 world px. In the grounded 64 px fixture, HK against crouch guard crosses over and skips its second contact. The adjacent 65 px setup blocks both strikes at +8 and +29 for both loadouts. Both blocked strikes also recur at the sampled 66, 68, 70, 72, and 80 px gaps.

## Light charge input boundary

At 64 px, a 54–62 display-sample sweep across four input phases per loadout isolates the light charge boundary. The shortest sampled successful hold was 60 display samples for New (three of four phases) and 58 for Old (one of four). New succeeded in all four at 61 and Old in all four at 60. When measured using the game tick byte at the frame-end start and release events, every sampled hold of 78 or more elapsed game ticks produced the special and every hold of 77 or less failed. The display-phase differences therefore do not imply a New versus Old charge timing rule. This measurement describes the input checkpoints in this recipe. The internal charge counter was outside the capture.

## Reproduce the capture

Use the Ryu arcade fixture’s `trace.lua` and `events.txt` format from local audit commit `9646b61`. Use the default EEPROM, the World `ssf2t` parent, and its `qsound_hle` dependency. The `start`, `coin`, and `lp` rows below are shared with the existing fixture. The two cursor movements select M. Bison from the first player’s initial Ryu position. Ken remains on the second player’s initial cursor position.

```powershell
rtk proxy git show 9646b61:docs/research/evidence/ryu-super-turbo/trace.lua
```

```text
1250 1255 coin1
1270 1275 coin2
1300 1305 start1
1340 1345 start2
1400 1405 lp1 lp2
1444 1446 l1
1452 1454 d1
1470 1475 lp1 lp2
1540 1545 lp1 lp2
2071 2199 d1 l1
2200 2202 r1 lk1
```

For Old Bison add the following rows. The kick is `lk1`, `mk1`, or `hk1`. For exact guard runs hold Ken’s `r2` or `d2 r2` on `N+1…N+120`, where `N = 2200` or `2201` is the Forward+Kick event. For settled Down alone use `d2` on `N−10…N+120`. Queue charge on `N−129…N−1` and Forward+Kick on `N…N+2`.

```text
1478 1479 d1
1482 1483 u1
1486 1487 u1
1490 1491 d1 lp1
```

Add this block inside `emu.register_frame_done`, immediately after reading `screen:frame_number()`. It applies the established Turbo 2 bytes and sets the starting distance. All moves and guard inputs use the controller. Stop at display sample 2350 or later. [Existing Turbo 2 setup](ryu-st-turbo2-contact-evidence.md)

```lua
if n == 2070 then
  assert(u8(p1 + 0x391) == 8 and u8(p2 + 0x391) == 4)
  ram:write_u8(0xff83a9, 2)
  ram:write_u8(0xff8cd3, 0x60)
  ram:write_u16(p1 + 6, 552)
  ram:write_u16(p1 + 8, 0)
  ram:write_u16(p2 + 6, 616)
  ram:write_u16(p2 + 8, 0)
end
```

Confirm the Old flag, matchup, speed, and starting gap in the trace. For 190 px, set Ken X to 742. For the whiff control, also hold his world Y word at 250 on every frame-end sample starting at 2190. For the light charge sweep use `N = 2200…2203`, replace the 129-sample hold with each `C = 54…62`, and record the game ticks at `N−C` and `N`.

Timestamp writes with `0xff801d`. Tap the state at both fighters’ `+0x03`, their world X at `+0x06`, Ken’s Y at `+0x0a`, both freeze counters at `+0x47`, and Ken’s life at `+0x2a`. Record Ken’s current animation and combo counter `+0x1f4` at each relevant write. Use the first damage write for each contact and the later combo readback to verify the string. Read Bison’s animation pointer at `+0x1a` and hitbox table at `+0x34`. The per-case schedules and exact capture rows are under `%TEMP%/ssf2-ryu-audit/st-bison-scissor`. Exact trace assertions passed for 72 contact and whiff trials. The World `sf2ce` and `sf2hf` parent archives are also present in the local library. The 1987 and SF1+ styles in #30 are project designs.

## Psycho Crusher

Hold Back and press Forward+Punch. The write taps see dispatch at `$08c5fc` for New and `$09ff7c` for Old. Both load a 60-tick charge count at `+0x81`. In the New parser, Back release opens a random 8–15 tick transition window and the direction test at `$08c544` requires exactly Forward. The command can briefly enter a normal before dispatch. At the nominal 108 px setup that normal never damages Ken. Both loadouts' 64 px Light Punch trials include pre-special damage.

The whiff control holds Ken above the strike at 190 px. The first attack pose reaches LP/MP/HP at +18/+15/+11 and the same rect continues through changes in attack ID. Its center is 31 px forward and 55 px above Bison's axis, and its radii are 44 by 15 px. The first IDs are 12/13/14 and the later IDs are 64/65/66. Both loadouts share the pose clocks and whiff displacement.

| Punch | First attack | Whiff exit and neutral | Whiff travel | Clean hit exit and neutral | Standing guard contacts |
| --- | ---: | ---: | ---: | ---: | --- |
| LP | +18 | +66 | +210 px | +79 | +18, +55 |
| MP | +15 | +67 | +322 px | +80 | +15, +52 |
| HP | +11 | +63 | +398 px | +75 | +11 |

Each nominal 108 px neutral trial lands once and knocks Ken down. His reaction ends 127 ticks after impact. Standing damage is LP 22–24, MP 24–26, and HP 26–29 among the sampled rows. Against standing guard, the first two strengths inflict two chip contacts and exit at +92/+93. HP inflicts one chip contact and exits at +76. Against crouch guard, New MP chips twice (+15/+54) and Old MP chips once (+15) in the sampled matched space. LP chips twice against either guard and HP chips once. All contacts load 14 on each fighter. The counters clear 14 ticks after contact for Bison and 15 for Ken, and Ken exits a single or final block reaction at contact +36. Holding standing guard advances Ken away before dispatch in this fixture, yielding an actual entry gap of about 121 px. Crouch and neutral start at 108 px.

## Head Press and the punch follow-up

Hold Down and press Up+Kick. Both loadouts load 60 at `+0x92` before charging, with state-write taps at `$08c8d4` and `$0a0248`. At 190 px with elevated Ken, the three kick attack poses start at +30 and use IDs 10/51/52. Each has a center 8 px forward with radii 18 by 21 px. Its local height is 1 px for LK and 16 px for MK/HK. All three whiffs exit at +74 and return to neutral at +80 after the turn. They travel 340.5625 px in the control. At nominal 85 px both loadouts' LK contacts at +30. Standing Ken takes 17–18 HP or 4–5 chip, with special exit +95 and neutral +101. Down-Back loses to the press for 17–18 HP, with special exit +93 and neutral +99. The contact loads 14 on both counters. Bison's bounce clears his counter two ticks later, while Ken's clears at contact +15 and his reaction ends at +36. In the first input phase of the standing neutral control, New ends near X 740 and Old near X 625. In the second phase both end near X 625.

New Bison can press Punch for a second attack after the bounce. At nominal 85 px and the first sampled input phase, LP at display `N+42`, `N+54` or `N+65` leads to a second clean hit at special +90 for 22 HP in addition to the initial 17. That strike uses pointer `$1549e0` and ID 16, with center 33 px forward and 57 px up, and radii 21 by 19 px. It loads 14 on both counters and they clear on contact +14/+15. Bison's special exits at +109, and he becomes neutral at +115 after turning. Early LP at `N+18` or `N+30` produces only the stomp in that fixture. The same late LP schedule produces one hit against tested guards and in the matched Old route. Depending on the contact and subsequent position, a follow-up can animate and miss. The tested display inputs sample the branch's acceptance window.

## Devil Reverse and Old Up+Punch

Hold Down and press Up+Punch. The Down count loads 60 into `+0xad` in both loadouts. The New dispatch tap is `$08c48c`, and Old is `$09fe24`. New's initial ascent has no attacking box. At 190 px the no-follow-up LP route leaves the special at +78 and turns neutral at +84. At 85 px with a grounded Ken it can land earlier, exiting at +62 and reaching neutral at +68 in the standing neutral control. A later LP enables a separately animated dive. At nominal 190 px the `N+37` follow-up enters substates 8 and 10 at +48 and +54, with attack pose `$15534c` at +93. Its ID is 76. The center lies 42 px to one side and 39 px above the axis, with radii 47 by 11 px. The X offset is +42 when the direction byte is 0 and −42 when it is 1. LP/MP/HP starters give whiff exit +124/+126/+128 with the same LP follow-up. At 85 px the `N+30` follow-up enters the branch earlier at +39 and exits at +108 in the standing control. The `N+37` LP follow-up without steering hits at +98 and exits at +129 for 24 HP. With Right held through the same 85 px standing matchup, `N+30` hits at +92, deals 26 HP and exits +122. Both clean dives load 14/14 freeze, clear at contact +14/+15, and knock Ken down for 134 ticks after impact.

Direction during New's follow-up changes the landing point without changing its timing in the 190 px whiff control. With LP as starter and follow-up, holding Left, neutral or Right gives about −51.5, +57.5 or +202.5 px total from the entry axis. This check presses the direction from display `N+14` through `N+105` and starts facing right. It describes the sampled camera bounds and opponent position.

At nominal 85 px with Right and the `N+30` LP follow-up, standing guard chips for 6 HP at +91 and crouch guard chips for 6 at +93. With `N+37` the same guards chip for 6 and 5, both at +98. Holding one guard direction through the dive is side-dependent. At nominal 120 px, a `N+37` Right-steered dive crosses standing Ken and hits him clean for 25 HP despite his held guard direction.

Old performs the attack automatically. At nominal 190 px the attack pose starts +53 for all strengths. It uses IDs 73/74/75, a 26 px horizontal offset with the same direction convention as New, a 39 px vertical offset, and radii 53 by 11 px. LP/MP/HP whiffs exit +77/+79/+82. In the sampled LP controls at nominal 85 px, standing Ken takes 24 HP at +57 or blocks for 6 chip. Down-Back receives a clean hit for 24–27 HP at +63. All three exit +91, load 14/14 freeze, and clear the counters at contact +14/+15. Ken's clean knockdown lasts 134 ticks from the attack contact. His block reaction lasts 36. The sampled Old Left/Right or second Punch inputs preserve the automatic dive path.

## Knee Press Nightmare

Hold Back, then press Forward, Back, Forward+Kick. The New parser at `$08c1b2` checks the Old flag, then uses `+0xc5` to `+0xca` for this route. The observed first Back poll loads a 70-tick count at `+0xc6`. There are 8–15 tick random windows between directional checkpoints after release. The final Forward opens a 14-tick button count and can accept a Kick edge. Exact display-sweep charge and button limits remain unmeasured. In the raw stock controls, 48 at `+0x2b4` and Kick select the super (`$08c312`) and clear stock immediately (`$027fae`). The controls with 0, 47 or 49 and the same LK motion select Double Knee Press. An LP edge at 48 selects Psycho Crusher. Old's LK motion at a forced 48 selects Double Knee Press without spending stock.

Super startup pause transitions at +18. With elevated Ken, the four attack poses write at +27/+35/+51/+59, with alternating 44-by-15 and 29-by-22 radii shared by Double Knee Press. The corresponding New pointers are `$155084`, `$15509c`, `$1550e4`, and `$1550fc`, attack IDs 79 to 82, and the whiff exits at +80. At nominal 80 px, New hits at +27/+39/+59/+71 for 17+16+19+17 HP. At nominal 115 px the first clean hit shifts to +28. Standing guard at 115 px chips at +29/+39/+59/+71, and crouch guard chips at +27/+39/+59 and then Bison crosses Ken. The four-contact routes exit +96 and the three-contact route exits +92. All Nightmare contacts load 5 freeze on Bison and 8 on Ken. The first freeze clears +5/+9, and subsequent freezes clear +5/+8. Ken exits the final clean knockdown at contact +115, or the final block reaction at contact +29.

## Charge-special capture details

Use the earlier selection rows and the same RAM taps. `N` is display sample 2200 or 2201. Set the Turbo bytes and initial axes at `N−14` so Ken can settle into the requested guard before `N`. For Psycho Crusher hold Down-Back on `N−120…N−1`, then Forward+Punch on `N…N+3`. For Head Press and Devil Reverse hold Down on `N−123…N−4`, then Up+Kick or Up+Punch on `N…N+2`. For Nightmare use Down-Back on `N−128…N−9`, Forward on `N−8…N−5`, Back on `N−4…N−1`, and Forward+Kick on `N…N+3`. Set the desired super stock byte at display 2070. Optional LP follow-up is a three-display input starting at `N + follow_display`. Ken holds the guard or crouch from `N−4` through `N+130`. In the elevated controls, hold Ken's Y at 250 from display 2195 onward. Actual entry gaps sometimes differ from the placed gap because fighters move before dispatch.

The [charge-special write summary](bison-super-turbo-charge-special-writes.tsv) gives one row per dispatch. `route` encodes the input recipe (`pc`, `head`, `reverse`, `super`), and `entry_pc` identifies what the game executed. The Ken fixture values are `sn` standing neutral, `sg` standing guard, `cn` crouching neutral, `cg` crouch guard, and `whiff` elevated Ken. `steer`/`retreat` use elevated Ken with Right/Left and `followr`/`followl` use grounded Ken with Right/Left, plus an optional `sg` or `cg` guard suffix. `entry_tick` is the raw game counter. Ticks after entry are relative, including special exit and subsequent state-0 entry. `pre_special_ken_hp` exposes any contact before the special. Each `contact_details` entry gives tick, damage, both loaded freezes, both contact-relative freeze clears, contact-relative defender reaction end, attack pose, attack box, Bison fixed-point X and Y, Ken fixed-point X and Y, and both facings. The X values are world coordinates scaled by 65,536. Attack boxes on contacts use signed world-relative X. Whiff box X is reported as if the fighter direction byte were 1. Box values are ID, X, height, X radius, and Y radius. `whiff_attack_poses` records each attack pose write with its relative tick. `-` marks fields without an event or applicable value. Raw `events.txt`, `trace.lua`, `command.tsv`, `writes.tsv`, `geometry.tsv`, and `trace.tsv` are grouped by the first nine columns in `%TEMP%/ssf2-ryu-audit/st-bison-charge-specials`.

## Ground throw

Forward or Back + MP or HP selects the same throw with either Bison loadout at 38 world px. LP and all three Kick buttons select their ordinary attack routes in the same starting position. Both throw buttons capture at an integer axis gap of 81 and fall back to standing normals at 82–84 px, in either direction and loadout. The common lock writes `+0x63` at `$00476c` and `$004772`. The victim write tap reports PC `$004778`. Bison enters attack state 10 on the capture tick, with pose `$154158` for New or `$1e5fd6` for Old. The established range applies to grounded New Ken.

In the 38 px fixture, Bison stays at X 550. Both throw directions release at +40. Ken is at Y 87 and X 606 with Forward or X 494 with Back. The full throw peaks at world Y 107 and does 32 HP at the +75 landing. Bison returns to state 0 at +65 and Ken at +149. Forward ends with Ken at X 775.90625, and Back at X 324.09375. These clocks match for both buttons and both loadouts in the sampled controls. Both freeze counters stay zero on the capture, release and damage writes.

New Ken can soften by pressing Left+MP during the lock. For both loadouts and throw directions, a response polled at +13 softens the throw, and +14 gives full damage. The softened route applies 16 HP at the +40 release, peaks at Y 158, lands at +90, and returns Ken to state 0 at +97. Bison still returns to neutral at +65. At wake-up, Ken is at X 706 for Forward or X 394 for Back in the 38 px fixture. Old Ken stays on the full route in the sampled +12 and +13 responses against either Bison loadout.

To reproduce, use the selection recipes above. For Old Ken add P2 Left at display 1478–1479, 1482–1483, and 1486–1487, then P2 Right+LP at 1490–1491. Set the speed and axes at `N−2`; this fixture uses P1 X 550 and P2 X 550 plus the requested gap, with fractions zero. Press P1 direction and button for one display at `N = 2200 + phase`. At the nominal 34 px placement, the idle pushboxes settle to an actual 38 px gap before the input. To test softening, press P2 Left+MP for one display at `N + response_display` and use the live input write to read its actual response tick. Tap both throw bytes `+0x63`, state `+0x03`, input `+0x392`, and Ken's life `+0x2a` and world Y `+0x0a`. Use game clock `0xff801d` for writes. The [ground throw capture summary](bison-super-turbo-ground-throw-writes.tsv) retains absolute input and capture game counters, observed axis gaps, dispatch pose, release position, peak, landing, damage, and neutral clocks relative to capture. `-` marks unavailable or inapplicable values. Raw schedules and taps are under `%TEMP%/ssf2-ryu-audit/st-bison-ground-throws` and `st-bison-ground-throws-old-ken`.

## Walk and jump

New and Old walk forward 4 world px per advancing game tick and backward 3 px, beginning on the first effective input tick. Three display phases per direction and loadout give the same free steps. With axes at X 600 and 768 and Ken neutral, the first 32 forward steps are +4 px and subsequent steps are +2 px as the fighters push against each other. Back reaches 36 or 37 three-pixel steps in the 29-display hold, depending on the Turbo skip phase. [Movement writes](bison-super-turbo-movement-writes.tsv)

The neutral, forward and backward jump arcs match between loadouts. Tick 0 is the jump state write at `$07b482`. The airborne byte at `+0x181` sets at +2 and Bison first rises at +3. The table uses an elevated Ken to isolate contact and starts Bison at X 650. Peak height is above floor Y 40. The first floor write can precede the airborne clear by one tick.

| Jump | Peak height | First floor | Airborne clear | Neutral state | X delta at clear | X delta at neutral |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Neutral | 108 | +50 | +51 | +58 | 0 | 0 |
| Forward | 103 | +49 | +50 | +56 | +193.03125 | +193.03125 |
| Back | 112 | +51 | +52 | +59 | −221.09765625 | −218.09765625 |

The backward jump from X 552 stops earlier at the camera boundary and ends about 103.1 px behind its start. The forward jump from X 552 can take one additional tick to enter state 0 after clearing airborne. The per-tick table retains the X words and fractions together as a signed displacement scaled by 65,536.

## Aerial normal attacks

All six buttons were tested on neutral, forward and back jumps. Forward and back share the directional attack poses within a loadout. Neutral jumps use their own poses and hitboxes. The table measures box timing from attack dispatch and uses an exclusive end tick. For LP and LK the box lasts until airborne clear in these early-button whiffs. Geometry uses the attacker axis and lists box ID, forward offset, height, horizontal radius and vertical radius.

| Button | Neutral active interval | Neutral attack box | Directional interval | Directional attack box |
| --- | --- | --- | --- | --- |
| LP | +3 to landing | `26,50,60,22,18` | +2 to landing | `34,37,48,34,12` |
| MP | +3–23 | `28,50,60,22,18` | New +3–13, Old +3–23 | New `83,50,60,22,18`, Old `36,49,61,38,16` |
| HP | +6–16 | `30,51,67,33,19` | +9–21 | `38,48,65,30,17` |
| LK | +2 to landing | `8,46,43,35,15` | +2 to landing | `9,38,60,24,17` |
| MK | +3–23 | `31,51,62,36,15` | +3–23 | `39,16,39,24,20` |
| HK | +5–13 | `32,57,62,42,15` | +5–13 | `40,38,58,52,12` |

New directional MP enters state 10 at `$08cb96` and uses attack box ID 83. It clears airborne on the usual jump clock and can reach neutral a tick sooner after landing. Old directional MP stays in the ordinary jump attack substate and uses a wider box for ten additional ticks. In the 175 px forward jump at display delay 23, Old MP hits at jump +44 for 22 HP and can be blocked standing. The same New input whiffs. At 205 px and delay 29, New MP does 9 HP to standing Ken at +44 and Old does 19 HP. Both can be blocked standing. A held Down-Back at the same gap and delay takes a clean hit at +49 for 13 HP from New or +48 for 19 HP from Old.

In representative forward contacts at 175 px and display delay 23, standing Ken takes LP at jump +43, LK at +46 and MK at +40. At 185 px, HP hits at +44 for 22 HP New or 24 HP Old. At 175 px and delay 29, HK hits standing at +44 for 24 HP New or 29 HP Old. Each of these controls also produces a standing block without chip. Against crouch guard at 175 px, LP and MK hit clean at +48 and +44 with delay 23, and HK hits clean at +48 with delay 29 in both loadouts. All recorded hit or guard contacts load 14 on both fighters. A hit or block can delay landing by 14 ticks and change movement after the contact. [Aerial normal writes](bison-super-turbo-aerial-normal-writes.tsv)

For the walks, use the selection recipe above, set X 600 and 768 at display 2160, leave Ken neutral, then hold Right or Left from `N` through `N+28`, for `N = 2200…2202`. For jumps, set axes at `N−2`; use Bison X 650 for the isolated arcs and X 552 for normal comparisons. Hold Up, Up+Right or Up+Left from `N` through `N+2`. Press the listed button for three display samples beginning at `N + display_button_delay`. For elevated controls, set Ken's world Y to 250 continuously starting at `N−2`. For standing guard hold Ken's Right from one display after the button through `N+90`. For crouch or Down-Back, hold the input from `N−12` through `N+90`.

In the aerial TSV, `start_tick` is the raw jump game counter and subsequent ticks are offsets from it. `attack_box_windows` and `pose_steps` pair tick with new box and pointer respectively. The contact cell records jump tick, damage, both loaded freeze values, Ken's contact-relative reaction end, the attack pointer and box, Bison's X displacement and height, and Ken's X relative to Bison's starting axis. X values use world px scaled by 65,536. Contact box X is signed in world orientation. `-` marks a missing or inapplicable value. Raw schedules and traces are under `%TEMP%/ssf2-ryu-audit/bison-t2-walk`, `st-bison-free-jump` and `st-bison-air-normals`.

## Standing and crouching normals

These World Turbo 2 write taps cover New and Old Bison against New Ken. Tick 0 is entry to normal attack state 10. The active intervals use exclusive endpoints. Close and far rows name the spacing-selected animation, while the contacts were sampled at the specified axis gaps. The sampled whiff timings and hitbox geometry match between both loadouts.

| Button | Close standing active / exit | Far standing active / exit | Crouching active / exit |
| --- | --- | --- | --- |
| LP | +4–8 / +12 | +4–8 / +12 | +4–8 / +12 |
| MP | +3–11 / +19 | +7–11 / +19 | +5–9 / +17 |
| HP | +9–15 / +34 | +9–15 / +34 | +9–15 / +34 |
| LK | +3–5 / +6 | +5–9 / +12 | +5–9 / +14 |
| MK | +7–12 / +21 | +7–12 / +21 | +5–13 / +18 |
| HK | +7–13 / +31 | +7–13 / +31 | +7–36 / +53 |

Close MP has an initial box centered 21 px forward and 41 px above the axis with radii 16 by 11. At +7 it changes to the far MP's box, centered 47 px forward and 56 px above with radii 35 by 5. LP swaps animation with space and retains the same box and timing. The last close and first far integer gaps in the elevated-Ken fixture are LP 81/82, MP 68/69, LK 63/64 and MK 55/56, with the same boundaries for both loadouts. HP and HK use the same pose stream throughout the sampled 18–85 px gaps. Forward or Back on the button edge and diagonals while crouching were checked on all six buttons. The matching ground normal or the known MP/HP throw accounts for the sampled inputs.

At 38 px on standing Ken, the first clean hits are LP +4, MP +3, HP +9, LK +3, MK +7 and HK +7. At 75 px LP, HP, MK and HK keep those clocks, while MP first hits +7 and LK +5. The standing LP misses settled crouching Ken. At 54 px, crouching LK/MK/HK land clean against standing guard at +5/+5/+7 and block against Down-Back. On clean sweep, Ken exits reaction +95 after impact. On a blocked sweep he exits +36. All recorded contacts load both freeze counters to 14. The typical post-contact Ken reaction lengths for light, medium, and heavy normals are 27/32/36 ticks. Corresponding X recoil in the grounded midstage controls is 30/54/78 px. Blocked sweep uses 116 px in the 54 px fixture. The full [normal write summary](bison-super-turbo-normal-writes.tsv) retains pose transitions, attack boxes, hit or block, and both fighter positions as deltas.

### Charge special cancels

The source can leave a normal directly for a charge special during contact freeze. On crouching MK at nominal 54 px, New Bison's LP Psycho Crusher enters at +14 from normal entry and reaches Ken at +37, after the normal's hit reaction ends. In the same spacing, crouching MK into LK Double Knee Press can form a three-hit combo. A sampled New route uses the Forward+LK command at +16. The normal hits at +5, the special enters +19, and its hits follow at +27/+51. `+0x1f4` reaches 2, counting two continuations. A matched Old route with the command at +16 enters +20 and hits +28/+53, also yielding two continuations. Both loadouts can cancel crouching LK or MK on crouch block. In the first New phase, a +13 command cancels at +17 and causes two chip contacts at +27 and +48. In the sampled two-phase Double Knee trials, c.LK and c.MK sometimes produce a special only after a whiffed normal ends, and the c.HK sweep produces no special during its tested command windows.

Crouching MK also cancels into Head Press. At 54 px and a clean +5 normal hit, an Up+LK command polled at +16 dispatches on +16 in both loadouts. The Head Press first damages standing Ken at +62 New or +59 Old and the combo byte stays zero. Earlier +11 commands dispatch at +11. Old's press then hits at +54 while New's still hits +62 in the sampled fixture. The [cancel write summary](bison-super-turbo-normal-cancel-writes.tsv) marks direct state-10 to state-12 transitions as `cancel`, later special entry as `after`, and absence of special entry as `-`. Command timing, damage, and combo continuation are separate columns. The sampled display schedules do not establish a universal cancel acceptance tick.

Use the existing character and Turbo selection; set Bison X 552, Ken X plus the requested gap, and both fractions zero at `N−2`, where `N = 2200 + phase`. Hold Down from `N−10` for crouching normals and press the listed button on `N…N+2`. For directional candidates add Left/Right during those three displays. Ken holds Right from `N+1` for standing guard or Down / Down-Back from `N−12` for the crouching trials. Whiff controls continuously raise Ken to Y 250 starting at `N−2`. For the cancel controls hold Back or Down-Back from `N−125` until the display before the command, use a one-display normal at `N`, and apply Forward+LP or Forward+LK at the listed display delay for three displays. The Head Press controls charge Down and use Up+LK, with a three-display directional release before the command. All data timestamps come from the `0xff801d` game clock. Source raws are under `%TEMP%/ssf2-ryu-audit/st-bison-normal` and `st-bison-normal-cancel`.
