# M. Bison in Super Street Fighter II World 931005

These are source measurements for the SF2 Super style in [M. Bison #30](https://github.com/johnstreetfighter/sf1-port/issues/30). MAME 0.289 verifies the World 931005 parent `ssf2` with `qsound_hle`. Bison is P1 (ID 8) and Ken is P2 (ID 4), both initially at 144 HP. The comparison uses the independently measured New and Old Bison in World 940223 Super Turbo, Turbo 2. The sampled Super fight clock advances one tick per display. Turbo 2 sometimes advances two ticks. Tables compare the in-game ticks so a shared move clock still looks faster at Turbo 2 over display time. [MAME revision manifest](https://github.com/mamedev/mame/blob/mame0289/src/mame/capcom/cps2.cpp#L11075) · [World 940223 Bison captures](bison-super-turbo-bonus-audit.md)

## Walk and grounded normals

Both releases move Bison forward by 4 px per free game tick and backward by 3 px. From a 168 px gap with walk held for 29 displays, Super takes 29 steps and travels 116 px forward or 87 px backward in both phases. The comparable Turbo 2 schedule yields 36–37 walking steps, with its final forward steps slowed by contact with Ken. The step size for an unobstructed walk stays at +4/−3 in New and Old.

The 38 px close, 85 px far and 54 px crouch whiffs have the same first attack tick, active durations and sampled attack geometry as New and Old Turbo 2. Close MP still uses two attack boxes. Far MP activates four ticks after close MP. Super's recovery differs in the following normal families.

| Form | Super first active | Super exits attack | Turbo 2 exits attack |
| --- | ---: | ---: | ---: |
| Close / far LP | +4 | +13 | +12 |
| Close / far MP | +3 / +7 | +20 | +19 |
| Standing HP | +9 | +35 | +34 |
| Crouching MP | +5 | +18 | +17 |
| Crouching HP | +9 | +35 | +34 |

The standing kick whiffs exit at +6/+12 for close/far LK, +21 for MK, and +31 for HK. Crouching LP/LK/MK/HK exit at +12/+14/+18/+53, all as in Turbo 2. The first contact loads 14 freeze on both fighters. In the matched clean neutral samples Ken's light/medium/heavy grounded reactions end +28/+33/+37 after contact, with 30/54/78 px recoil. The comparable Turbo 2 cases end +27/+32/+36. Super's crouching sweep knocks down and ends Ken's first clean reaction at +96, one tick later than the analogous Turbo 2 case.

## Charge specials and their branches

The source accepts Back then Forward+Punch for Psycho Crusher, Back then Forward+Kick for Double Knee Press, Down then Up+Kick for Head Press, and Down then Up+Punch for the automatic dive. Motion, entry, exit and guard data are linked below. Special ticks start at entry to state 12.

| Input | Super dispatch tap PC | First active in a whiff | Special exit in a whiff | Sampled clean standing hit ticks |
| --- | --- | --- | --- | --- |
| Psycho Crusher LP / MP / HP | `$079d46` | +18 / +15 / +11 | +66 / +67 / +63 | +18 / +15 / +11 at 108 px |
| Double Knee LK / MK / HK | `$079eae` | +8 | +36 / +37 / +38 | +8 and +29 at 64 px |
| Head Press LK / MK / HK | `$07a012` | +30 | +74 | +30 at 85 px |
| Up+Punch LP / MP / HP | `$079bee` | +53 | +77 / +79 / +82 | +57 / +61 / +63 at 85 px |

Psycho Crusher matches Turbo 2 on whiff clocks, first attack boxes, and free travel of 210/322/398 px. At 108 px all three strengths land once and knock down. Super has the same two-contact LP/MP standing guard routes and one-contact HP guard route at the sampled spacing. Super MP also chips twice against crouch guard, as in New Turbo 2 in that fixture. The Super final block and clean knockdown clocks here are +37 and +128 from contact, one tick longer than the matched Turbo 2 examples.

Double Knee matches the Turbo 2 whiff geometry and first and second whiff attack ticks (+8 and +16). At 64 px two clean hits extend the second contact to +29 and the special exits at +62/+63/+64. LK and MK chip twice on crouch guard. HK chips just once in the same nominal 64 px fixture and crosses Ken. Each contact loads 14/14 freeze.

For light Head Press at 85 px the initial hit loads 14/14 freeze. Bison clears his counter two ticks later on bounce, as in both Turbo 2 loadouts. Super accepts a second LP after the bounce. In both sampled phases, a three-display follow-up starting at `N+52` produces one hit. Starting at `N+53`, `N+54` or `N+65` produces a second hit and exits at +109. The first phase gives 20+22 HP at +30/+93, and the second gives 17+22 at +30/+90. The follow-up pose uses attack ID 16, with center 33 px forward and 57 px up, and radii 21 by 19. Turbo 2 New can produce its additional hit at +90 in the matched standing fixture. The Turbo 2 Old samples use a single stomp at those display delays.

Super Up+Punch automatically dives and hits without a second Punch. This matches the Turbo 2 Old branch in the 85 px LP neutral fixture (+57 hit and +91 exit). The whiff geometry, attack IDs 73–75, and LP/MP/HP whiff exit ticks also match Turbo 2 Old. In Turbo 2 New the initial Up+Punch needs a follow-up Punch to attack, and directional steering can change the dive's path. Super's two-hit Head Press and automatic Up+Punch belong to separate branches within the same release.

The Turbo 2 Nightmare charge and forward-back-forward motion selects ordinary Double Knee with LK in Super, or Crusher with LP. Forcing the later game's stock byte location `Bison+0x2b4` to 48 leaves both Super dispatches unchanged. This is a comparison of that command and raw address. New Turbo 2 spends the stock and selects Knee Press Nightmare with the Kick. Old Turbo 2 falls back to the ordinary kick special.

## Ground throw

Forward or Back + MP/HP selects the same throw at 38 px. LP and all kicks keep their standing normals in the same fixture. MP throws at integer axis gap 81 px, and at 82 px it falls back to far MP, in either direction. The source writes Ken's lock at PC `$0043bc`. Bison releases at +40, returns to neutral +65, deals 32 HP at +75 and leaves Ken neutral at +149. Both freeze counters stay zero on these events. This matches the sampled full throw in both Turbo 2 loadouts. For Super Ken, Left+MP polled at +8 or +11 through +15 leaves the full throw in both directions. In Turbo 2, New Ken can soften with that input at +13. [World Turbo 2 throw summary](bison-super-turbo-ground-throw-writes.tsv)

## Reproduction

Use the fresh-EEPROM boot and join inputs from the [World 931005 Ryu fixture](ryu-super-sfii-audit.md#reproduce-the-arcade-measurements), including the second-player coin and Start retries. Between the 1400 and 1470 selection presses, move P1 Left at displays 1444–1446, Down at 1452–1454 and 1460–1462, and Right at 1468–1470. Leave Ken's cursor in place. Reset axes and fractions at display 2860. Use X 600 and 768 for walking and X 552 plus the listed gap for combat. Keep the cabinet's original timing. Confirm fighter IDs 8 and 4 and 144 HP before the command display, `N = 2900` or `2901`.

For walks hold Right or Left through `N+28`. Press a normal for three displays and hold Down from `N−10` for crouching normals. For charge special controls, hold Down-Back for 125 displays before Forward+button, or hold Down through `N−4` before Up+button. Use the directions and timing in the [Turbo 2 command recipe](bison-super-turbo-bonus-audit.md#charge-special-capture-details) for the Nightmare motion. Head Press follow-up uses a three-display LP hold starting at the listed delay. Standing guard is Ken's Right from `N+1` for normals and `N−4` for specials. Hold Down-Back from `N−12` for crouch guard. For the whiff controls keep Ken's Y at 250 from display 2880. Throws from 38–84 px use three displays of the specified direction and button and a one-display softening response where listed.

Tap fighter bases `0xff83ce` and `0xff87ce`, game clock `0xff801d`, state `+0x03`, animation `+0x1a`, freeze `+0x47`, life `+0x2a`, position `+0x06/+0x0a`, throw lock `+0x63` and input `+0x392`. Direct strike damage writes use PC `$0921f4`, and defender freeze loads use `$0921cc`. Resolve box ID from pose `+0x0c` and table `+0x34` with the Super 12-byte attack-box stride. The [Super write summary](bison-super-world-931005-writes.tsv) records the actual entry gap, pose, box timeline, state exit, attacker travel, and exact contact data. The contact sequence records tick, damage, freeze values, defender's contact-relative reaction end, defender X travel, and attacker pose and box. Fixed-point displacement is scaled by 65,536. `follow_display` is relative to the command display. `-` marks missing or inapplicable events. Raw traces are grouped in `%TEMP%/ssf2-ryu-audit/bison-ssf2-source-lddr`, `bison-ssf2-boxes-lddr` and `bison-ssf2-fullstock-lddr`. [Super hitbox profile](https://github.com/peon2/fbneo-training-mode/blob/master/hitboxes/sf2-hitboxes.lua#L95)
