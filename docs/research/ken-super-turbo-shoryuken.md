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

## Port follow-up

The SF2 ST Ken code currently uses a two-hit ceiling for ordinary Shoryuken. The three-strength, three-contact and defender-dependent source cases above provide targets for the native replay. [Engine hit limit](../../crates/sf-engine/src/era_fight.rs)
