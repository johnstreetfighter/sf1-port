# Ken's Shoryuken in World Super Turbo 2

World `ssf2t` (940223) at Turbo 2, captured in MAME 0.289 against New Ryu. The 162 trials cover ordinary LP, MP and HP Shoryuken for New and Old Ken. The natural trials include standing, crouching, guarding, jumping and whiff controls. The placement controls reset both horizontal axes immediately before the last motion input to isolate spacing. [Spec #138](https://github.com/johnstreetfighter/sf1-port/issues/138) · [Ken and Sagat follow-up #153](https://github.com/johnstreetfighter/sf1-port/issues/153)

## Uninterrupted action

Tick zero is the write that enters special state 12 (decimal). The attack intervals include their first tick and exclude their end. Box values are centre toward the opponent, centre above Ken's axis, half-width and half-height, in arcade pixels. Start, contact and ready ticks use the advancing game tick. The two tested display phases produce the same uninterrupted trajectories.

| Strength | Active ticks and attack ID | First rise | First floor tick | First ready tick | Height above floor | Forward travel |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| LP, both | `[2,6)` ID 31 at `(30,45,28,22)` and `[6,20)` ID 32 at `(27,79,26,44)` | 8 | 36 | 39 | 35 px | 18 px |
| MP, both | `[2,6)` ID 41 at `(30,45,28,22)` and `[6,28)` ID 42 at `(27,79,26,44)` | 8 | 52 | 55 | 89 px | 68 px |
| HP, New | `[0,2)` ID 55 at `(11,49,21,13)`, `[2,4)` ID 43 at `(30,45,28,22)` and `[4,30)` ID 44 at `(27,79,26,44)` | 6 | 61 | 64 | 138 px | 126.5 px |
| HP, Old | `[1,2)` ID 55 at `(9,49,21,13)`, `[2,4)` ID 43 at `(30,45,28,22)` and `[4,30)` ID 44 at `(27,79,26,44)` | 6 | 61 | 64 | 138 px | 126.5 px |

## Spacing, guard and repeated contact

| Loadout and input | Defender and gap at move entry | Contact ticks | Damage or chip per contact | Ken ready |
| --- | --- | --- | --- | ---: |
| Both LP | Standing, 55 px | +2 | 30 | +53 |
| Both LP | Crouch guard, 55 px | +2/+20 | 7/5 or 8/4 | +67 |
| Both MP | Standing, 55 px | +2 | 29 | +69 |
| Both MP | Crouch guard, 55 px | +2/+20 | 7/3 | +83 |
| New HP | Standing, 32 px placement controls | +0/+16/+32 | 4/27–29/11 | +106 |
| Old HP | Standing, natural 39 px | +1/+16/+32 | 4/27/11 | +106 |
| New HP | Crouch guard, 55 px | +0/+16/+32 | 1/6–7/2 | +106 |
| Old HP | Crouch guard, 55 px | +1/+16/+32 | 1/6/2 | +106 |
| Both HP | Standing, 55 px | +2/+18 | 27/11 | +92 |
| Both HP | Standing, 72 px | +2 | 27 | +78 |
| Both HP | Standing, 95 px placement controls | +8 | 11 | +78 |

Every listed contact adds 14 game ticks to Ken's exit. The first HP strike can register a distinct hit before the main rising strikes. At a 55 px entry gap, it reaches crouch guard for both loadouts. The standing defender at the same gap first takes the lower rising strike, ID 43. The standing guard controls move the defender back before contact and enter at 57 or 59 px. Their HP contacts are +2/+18 for 6/2 chip. Damage variation in the table is observed across the two input phases.

The airborne target checks cover New Ryu jumping and an elevated whiff control. The natural jump at setup gap 55 produces first contacts at +6 for LP and MP and +4 for HP in the tested cases. In the New HP 32 px phase-zero control, the defender enters reaction state 14 one tick after the opener. The third hit sends Ryu airborne and he returns to idle at +158. In the New HP 55 px phase-zero crouch-guard control, Ryu returns to guard-ready state 8 at +67. The reaction file records state and displacement writes through recovery. Other opposing characters, corner recoil and cancel recipes remain open. Capcom's historical note documents the crouched hit reaction on the first heavy Shoryuken strike. [Capcom SFII combo seminar](https://game.capcom.com/cfn/sfv/column/131578?lang=en)

## Capture recipe and data

Use the [World Turbo 2 setup](ken-super-turbo-audit.md#reproduce-the-source-captures) and set speed selector `$ff83a9` to 2 and frame-skip byte `$ff8cd3` to `$60` at display event 2070. Place Ryu at X552 and Ken at X552 plus `setup_gap` with both X fractions cleared. Ken faces left. Let `N=2150+display_phase`. Hold Left from `N−8` through `N−5`, Down from `N−4` through `N−1`, then Down-Left and the strength button from `N` through `N+2`. For the placement controls, rewrite both X axes and their fractions at `N−1`. The natural trials allow the pre-input Ken walk to close the gap. The case table keeps both the configured distance and Ken's actual entry axis. For standing guard, hold Ryu's Back from `N−3` through `N+82`. For crouch and crouch guard, hold Down or Down-Back from `N−11` through `N+82`. For jumping, hold Up from `N−9` through `N−6`. For the elevated whiff control, pin Ryu at Y600 from display event 2100 through 2250.

- [Case setup, boundaries and contact totals](ken-super-turbo-shoryuken-cases.tsv)
- [Every advancing whiff tick, exact 16.16 horizontal travel and pose writes](ken-super-turbo-shoryuken-flight.tsv)
- [Defender state, damage and freeze writes](ken-super-turbo-shoryuken-contacts.tsv). The X columns for contact events are integer axes. The raw box has a negative X centre when Ken faces left.
- [Defender reaction state and integer-axis changes](ken-super-turbo-shoryuken-reactions.tsv). `ryustate` writes at fighter `+$02` put the state in the low byte. `ryux` and `ryuy` write the signed axis words at `+$06` and `+$0a`.

## Port replay

The workspace after v1.5.25 replaces the single 1987 Shoryuken stream that Super Turbo Ken used for every punch. Before this pass all three strengths rose 48 px, were ready at +56 for New and +58 for Old, landed one hit at 32 px and whiffed at 55 px. Both loadouts now have separate LP, MP and HP streams in every matchup. Champion Edition, Hyper Fighting and Faithful 1987 Ken keep their earlier Shoryuken.

### Matched against New Ryu

The ROM-backed Local Versus replay sets the gap before the last motion input, as the placement controls do. It covers both loadouts. [Controller regression](../../crates/sf-engine/tests/st_ken_shoryuken.rs)

| Check | Port result |
| --- | --- |
| Whiff, all strengths | Active ticks, first floor tick, ready tick and height match the uninterrupted table. Travel is rounded to the port pixel, so HP covers 127 px against the arcade's 126.5. |
| LP, standing, 55 px | Hits at +2 for 30. Ken ready +53. Ryu is knocked down, peaks 77 px above the floor and is idle at +136. |
| MP, standing, 55 px | Hits at +2 for 29. Ken ready +69. Ryu stays grounded, recoils 78 px and is idle at +38. |
| MP, standing, 32 px | Hits at +2 and +20 for 29 and 12. Ken ready +83. Ryu idle at +153. |
| HP, standing, 32 px | New hits at +0, +16 and +32. Old hits at +1, +16 and +32. Damage 4, 27 and 11. Ken ready +106. Ryu peaks 57 px and is idle at +158. |
| HP, standing, 39 px | New lands the first two strikes and is ready at +92. Old lands all three and is ready at +106. |
| HP, standing, 55 px | Hits at +2 and +18 for 27 and 11. Ken ready +92. Ryu idle at +144. |
| HP, standing, 72 px | Hits at +2 for 27. Ken ready +78. Ryu recoils 78 px and is idle at +38. |
| LP, crouch guard, 55 px | Chip at +2 and +20 for 8 and 4. Ken ready +67. Ryu guard-ready at +55 after 102 px. |
| MP, crouch guard, 55 px | Chip at +2 and +20 for 7 and 3. Ken ready +83. Ryu guard-ready at +55 after 102 px. |
| HP, crouch guard, 55 px | New chips at +0, +16 and +32. Old chips at +1, +16 and +32. Chip 1, 6 and 2. Ken ready +106. Ryu guard-ready at +67 after 102 px for New and 94 px for Old. |
| LP and MP, standing guard, 57 px | One chip at +2 for 8 or 7. Ken ready +53 or +69. Ryu guard-ready at +38. |
| HP, standing guard, 57 px | Chip at +2 and +18 for 6 and 2. Ken ready +92. Ryu guard-ready at +53. |
| Standing, 95 px | LP whiffs and is ready at +39. MP hits at +11 for 12, is ready at +69 and Ryu is idle at +145. HP hits at +8 for 11, is ready at +78 and Ryu is idle at +135. |
| Crouching, 39 px | LP hits once at +2. HP gives the same contacts as the standing 39 px row. |

Every contact loads 14 freeze on both fighters. A snapshot taken between the second and third heavy strikes replays to the same checksum.

### Rules the port applies

- **Strikes.** Each stream has a lower rising strike and an upper rising strike. HP adds the close opener, live from +0 for New and +1 for Old. Each strike can make one contact, so HP can make three. The source boxes above decide reach against New Ryu.
- **Reactions.** Both LP strikes and the upper strike of MP and HP knock a grounded New Ryu down. The MP lower strike, the HP opener and the HP lower strike leave him standing in the 78 px heavy recoil. A strike that lands during the previous reaction starts its recoil or knockdown one tick earlier. A knocked-down opponent cannot be hit again.
- **Knockdown.** The port replays the recorded heights from the [reaction file](ken-super-turbo-shoryuken-reactions.tsv). After a fresh hit Ryu first rises at contact +16. LP and MP send him 77 px up. He first reaches the floor 44 ticks after the first rise, lands for good at +65 and is idle at +118. HP sends him 57 px up, with the same three points at +37, +58 and +111.
- **Guard on the entry tick.** New Ken's opener is live on the tick the move starts. The port puts a New Ryu who is holding Down-Back into crouch guard on that tick, as the arcade state writes show.
- **Other matchups.** Flight, strike phases, the three-contact limit, the 14-tick hold on Ken, the damage table and the choice between a standing and a knockdown reaction apply against every opponent. Reach, recoil distance and knockdown timing for other defenders use the port's general rules. At 32 px a Faithful 1987 Ryu and an Old Super Turbo Ryu take one LP, two MP and three HP contacts.

### Fitted or still open

- **Damage.** The port uses one value per strike: LP 30 then 20, MP 29 then 12, HP 4, 27 and 11. Chip is LP 8 then 4, MP 7 then 3, HP 1, 6 and 2. The arcade rolls damage. The cases also show 33 and 35 for LP, 28 and 29 for the HP lower strike, and 13 for an upper strike.
- **Ryu's recoiling body.** No hurtbox was sampled for Ryu's Shoryuken recoil. The port bounds it from the contacts. The upper strike lands at a 63 px gap on the first recoil tick. From the second recoil tick it lands at 56 px and misses at 63 px. A direct hurtbox sample would replace this fit.
- **Knockdown drift.** The port moves Ryu back 2 px per tick to the first landing, rests four ticks, then 2.5 px per tick through the hop and a nine-tick slide. In the arcade his total displacement varies by case from 138 to 175 px, which suggests leftover recoil and contact with Ken's rising body. The port's landing spot is not matched.
- **Late guard shift.** After a crouch-guarded HP the arcade moves Ryu a further 1 px for New and 5 px for Old at +98. The port does not.
- **Ken's own boxes.** Ken's hurtboxes and invulnerability stay with the adapted 1987 poses. The source values were not measured.
- **Not yet replayed.** The jumping and elevated targets, the crouching MP whose second hit arrives at +20 from 35 px and +29 from 39 px, and the 59 px standing guard have source rows and no port test. An airborne Ryu takes the port's general knockdown. The tests put Ken on the right as player two.
- **Balance options.** The port's default roster balance still adds its one-tick startup invulnerability to Ken's Shoryuken. That rule predates this pass.
- **Not measured.** Other opposing characters, corner recoil and cancel recipes.
