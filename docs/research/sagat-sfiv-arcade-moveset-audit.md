# Sagat moveset audit for Street Fighter IV and Arcade Edition 2012

Source evidence for [#148](https://github.com/johnstreetfighter/sf1-port/issues/148). The comparison uses original SFIV and AE2012 arcade data. Use Sagat versus Ryu with Classic inputs and Era Default gauges in the port, with the original Ultra I and both AE2012 Ultra loadouts. The port source baseline is Git `0e336743760689ccdd192cbe4ebce5c4d638c9c4`. Paired live traces remain open. [Port card registry](../../crates/sf-engine/src/era.rs#L452)

## Source builds

The original reference is Taito Type X2 v1.0 STD, from `Street Fighter IV (2008).zip`, preservation SHA-1 `1120092ad2dcde6a381a2305dbbb843339585c44`. Its `launcher.exe` MD5 is `e47d7c83e526dbb1986173f197801b99`, header CRC-32 `ddd19e50`. Extract `Battle/Chara/SGT/SGT.cmn.emz`. The AE2012 reference is Japanese Taito Type X2 NESYS 1.3, from `Super Street Fighter IV Arcade Edition Ver. 2012 (1.3) (2011).zip`, preservation SHA-1 `dd2d5cf64e9d35dd1b3da772c9d72b906b860026`. Its `game.exe` MD5 is `15432af2c3ec96b7dec196477b0e3591`, normalized header CRC-32 `545b0d0c`. Use `patch/battle/regulation/latest/SGT/`. [Preservation metadata](https://archive.org/metadata/Yori-Arcade-Arcade_Games_-_TAiTO_Type_X2-RomVault) · [Original fingerprint](https://github.com/teknogods/OpenParrot/blob/fc8ba632e1f67c5c340ff88f30efca743e8d38f5/OpenParrot/src/Utility/GameDetect.cpp#L322) · [AE2012 normalization and fingerprint](https://github.com/teknogods/OpenParrot/blob/fc8ba632e1f67c5c340ff88f30efca743e8d38f5/OpenParrot/src/Utility/GameDetect.cpp#L449)

| Member | Bytes | SHA-256 |
|---|---:|---|
| Original `SGT.cmn.emz` | 5416126 | `c948bb0cb94c79a67df02e486e62a10ee1bea15ceb348191dbd65eb6a70e3141` |
| Original `SGT.bac` | 177216 | `a5ec18e10ac54454b401d7a52d7783bd437f398a2404095987fd89f1d61771ab` |
| Original `SGT.bcm` | 7589 | `012fcc12b190b51758671e96fa4bc2d83e33cb6fc76009de6413c7ce42927671` |
| AE2012 latest `SGT.bac` | 227325 | `8f7b8e0b6651a0ff4576dacd8125d50d789079639db9322d3582529e653abbad` |
| AE2012 latest `SGT.bcm` | 10095 | `8704aabed3bc39eb8e2e7ae78fc5305c720dd8e158586cd9e9299034dc176da1` |
| Arcade `v101/SGT/SGT.bac` | 212501 | `97db2cee1761eb2dd738c8ea3957d2d1599182502dcfa15c158513dc0361719d` |
| Arcade `resource/.../SGT.bac` | 212501 | `e7305b69e5ad8ec970f396c599da7e562a3fe125cda31808ce004e8e73fb983c` |
| Arcade `v101` and `resource` Sagat BCM | 9677 each | `662450aff0d08bf11dac1c2f35d1dd436578351ff8d010b54a736c73396da209` |

The original BAC has 195 populated actions and 104 populated reactions. Its BCM has 69 commands, seven motion definitions and 16 cancel lists. AE2012 latest has 213 decoded main actions, nine named projectile actions, 131 populated reactions, 76 commands, ten motion definitions and 17 cancel lists. The older `v101` files in the AE archive supply a balance comparison. Original Super SFIV console data remain open. [Pinned PC Sagat change files](https://github.com/dantarion/sf4tools/tree/e07764262003f3aa1feff1a0a5a66fc3c14efb43/characters/SGT)

## Command inventory

BCM indices are zero-based. Directions use facing-relative numpad notation. The first 54 rows have the same inputs in both arcade references.

| Original / AE2012 rows | Commands |
|---|---|
| 0–15 | Six close/far standing pairs at 1.3 native units, including rapid LP and LK for both ranges. AE2012 routes ordinary LK to `5LK_NEW/5LKF_NEW`. Rapid LK keeps the older actions. |
| 16–23 | Six crouching normals plus rapid `2LP/2LK`. |
| 24–35 | Six neutral and six directional jump normals. |
| 36–39 | Step Low Kick `6LK`, Step High Kick `6HK`, Heavy Tiger Elbow `6HP`, and Fake Kick through another HK during close or far standing HK startup. |
| 40–41 | Forward dash `66`, backdash `44`. |
| 42–49 | High Tiger Shot `236LP/MP/HP`, EX `236PP`; Grand Tiger Shot `236LK/MK/HK`, EX `236KK`. |
| 50–53 | Tiger Knee Crush `623LK/MK/HK/KK`. |
| 54–57 / 54–61 | Tiger Uppercut `623LP/MP/HP/PP`. AE2012 has four stance-gated charged rows for the same inputs. |
| — / 62 | Angry Charge `214214K`. Uses the BAC `ANGRY_SCAR` state and costs one Super stock. |
| 58–60 / 63–65 | Tiger Genocide `236236LK/MK/HK`. |
| 61 / 66 | Tiger Destruction Ultra I, `236236KKK`. |
| — / 67 | Tiger Cannon Ultra II, `236236PPP`. |
| 62–63 / 68–69 | Tiger Rage `5/6+LP+LK` and Tiger Carry `4+LP+LK`. |
| 64–67 / 70–74 | Focus `MP+MK`, charge, two release stages and EX Focus. AE2012 adds an airborne EX Focus row. |
| 68 / 75 | Neutral `HP+HK` appeal. |

EX specials are 250 raw Super units, EX Focus 500 and Tiger Genocide 1000. The Ultras encode 200 raw Revenge threshold and 1000 cost. No distinct target combo row exists beyond Fake Kick and rapid lights. Double, Red Focus and delayed standing are Ultra-era systems. [Capcom Ultra systems](https://news.capcomusa.com/lets/browse/ultra-street-fighter-4-change-list)

## Movement, normals and contact

Intervals below are half-open local BAC animation ticks. Freeze is attacker/defender. Damage and stun are meter fields; push is the source reaction parameter. Actual input and elapsed endpoints require the rate commands and live target state.

| Property | 2008 v1.0 STD | AE2012 latest |
|---|---|---|
| Forward/backward walk | +0.024/−0.020 native units per advancing tick with zero acceleration. Sixteen forward ticks move 0.384 units, or 29.54% of the 1.3 cutoff. | Same. |
| Standing far LP | `[13,16)`, boundary 24, reaction 12. 40 damage/50 stun, hit and guard stop 8/8, push 0.3. | Same sampled fields. |
| Close/far LK | Both `[14,17)`, boundary 37 and stop 8/8. Reactions 6/18 deal 40/50 damage. Both list a `[14,17)` special gate. | Both use `[13,14)` then `[14,17)`, reactions 68/69 close and 70/71 far. Damage 20/30, stun 30/20, stop 8/8. Only the first hit has the `SUPER MOVE` execute row. Rapid LK still uses 6/18. |
| Standing HK and Fake Kick | Close HK `[12,14)` then `[14,17)`, damage fields 40/110. Close and far HK provide feint buffer `[1,12)`, execute `[10,12)`. The feint has no strike and ends at raw 12. | Feint and contact clocks persist. Close HK damage fields 80/40. Both `SUPER COMBO` and `SUPER MOVE` execute from its first hit at `[12,14)`. |
| Crouching MK and sweep | MK `[24,27)` boundary 52, 90/100 damage/stun, stop 10/10, push 0.5. Sweep `[14,17)` boundary 50, 130/200, stop 12/12, push 0.7 and sweep reaction 146. | Same sampled clocks and stop with 65 damage for MK and 100/100 for sweep. `2MP` has 70/100 and `[8,11)` execution for `SUPER MOVE`. |
| Step Low, Step High, Elbow | `6LK [23,26)`, low, 50/50, 10/10, 0.5 push. `6HK [24,26)`, mid, 140/200, 12/12, 0.7 push. `6HP [25,30)`, overhead, 100/150, 12/12, 0.7 push. | Same boxes' intervals and sampled push. Step High damage drops to 100. Both have startup special gates, `[20,23)` for Step Low and `[1,23)` for Step High under condition 1. Elbow has no cancel list. |
| Tiger Rage and Tiger Carry | Both grab at `[3,5)`, reaction 90, branch `[3,6)`. Rage impacts `[63,65)`, `[94,96)`, `[124,126)` in the capture action for 40/40/60, final stun 160. Carry impacts `[51,53)` for 140/120. | Same sampled windows and damage. Throw tech, spacing and side switch remain live checks. |
| Focus | Level 1/2 `[14,16)`, level 3 `[74,76)`, effects 60/61/62. Damage 60/90/150 and stun 100/150/200. Charge release branches `[13,21)` and `[21,60)`. | Same sampled data. Separate ground and air EX Focus command rows cost 500. |

### Complete normal and command-normal source records

The [Sagat normal matrix](sagat-sfiv-arcade-normals.tsv) enumerates BCM rows 0–39 for the original board, the AE archive's resource and v101 copies, and AE2012 latest. The 186 records cover all ordinary, rapid, neutral-jump, directional-jump and command normals, including Fake Kick as one action with no strike per build. Each strike records its BAC action, authored timing, four attack-rectangle fields, non-proximity box ID, guard level, cancel rows, animation-rate writes and six ordinary reaction variants.

Each reaction cell gives damage, stun-meter damage, effect, reaction selector, attacker stop, defender stop, target animation timer, X/Y/Z velocities and push distance, in that order. Guard damage is chip. Original effects and selectors retain raw numeric IDs. The later files use the decoder's named effects and selectors. A source rectangle preserves the coordinates as encoded by its revision. The data leave coordinate conversion, hurtbox intersection, counter hit, simultaneous box selection and elapsed playable outcomes to the paired traces.

- Close and far ordinary LK each select new AE2012 action IDs 349/350 with damage 20 then 30. Their rapid versions stay on 308/309, each with one `[14,17)` box. The rapid far record drops from 50 in 2008 to 30 in the AE resource/v101 and 20 in latest. Only the first ordinary LK group has `SUPER MOVE` execution.
- Close and far standing HK both change from 40/110 to 80/40 damage across their two reaction groups. AE's close HK gains `SUPER MOVE` execute over `[12,14)` alongside `SUPER COMBO`, while far HK retains `SUPER COMBO` only there. Both preserve the separate pre-contact Fake Kick execution window `[10,12)`.
- Directional jumping MP extends its active window from `[23,31)` in 2008 to `[23,33)` in all three AE copies. Directional HP preserves `[22,27)` and has 140 damage in 2008, 120 in each AE copy. The resource's raw total is 54 and its rate-0 write is at 53. The distinct v101 and latest copies move that write to 58 and the total to 60.
- The rapid light groups `CANCEL_L` and `CANCEL_2L` resolve to rapid close/far LP and LK and rapid crouching LP/LK. Far MP, far MK, close and far HP and every jump normal have no listed BAC cancel rows in these source samples.

## Specials and finishers

| Family | Extracted results |
|---|---|
| High and low Tiger Shot clock | High spawn `[24,25)`, boundary 61. Low spawn `[25,26)`, boundary 67 original and 68 latest AE2012. Each family's ordinary and EX strengths share its emitter clock. L/M/H velocity settings are 0.06/0.08/0.10. High EX sets 0.11 on spawn and 0.10 at projectile tick 21. Low EX stays 0.11. |
| Shot contact | Original ordinary high/low deal 70 damage, 100 stun, chip 17, stop 8/8 and 0.8 push. AE2012 gives high 65 and chip 16, low 60 and chip 15. Original EX effects 103/107 give 80+70 damage, 100+100 stun and 20+17 chip. AE2012 EX effects 200/201 high and 205/206 low give 70+50, 50+50 and 18+13. Both EX paths encode stop 8/8 and 0.5/0.8 push, with separate hit effects and overlapping coverage boxes. |
| Tiger Knee Crush | Ordinary `[7,9)` then `[9,18)` L or `[9,19)` M/H, effects 108–113. Original damage splits 30+90, 40+100, 50+110. AE2012 flips the splits to 90+30, 100+40, 110+50. Normal effects give 100 stun each, stop 14/14 and 0.3/0.8 push. EX has `[7,10)`, `[10,15)`, `[15,20)` for 100 with reaction 114 and two boxes using reaction 115 at 40. EX freeze is 8/8. All strengths use the `[7,10)` execute row in `SUPER COMBO`. |
| Ordinary Tiger Uppercut | L/M/H first `[10,12)`, second `[12,14)` and continuation from 14. Original first records 140/150/170 and late 80. AE2012 first records 120 for the three, late 130/140/160. Original continuation ends 17/20/22, AE2012 21/23/24. Both use stop 14/14, early/late push 0.9/0.8. EX retains `[10,15)` at 120 and four 20-damage groups from 15 through 23, stop 6/6 on the late groups. The Super and EX Focus execution gate is `[10,14)`. AE2012 normal strengths also list the air EX Focus route there. |
| Angry Charge and powered Uppercut | AE2012 has a Superflash start at raw 0 and release at 27, flag set at 9, and authored length 40. Powered DP clears the flag at 4 for L/M/H and 3 for EX. Latest L/M/H give 150/170/180 per first or later qualifying hit. EX gives 170 plus four 20 fields. Archive `v101` has 140/160/180 and 160+4×20. Capcom corroborates the change in light, medium and EX totals. The powered variants use the same authored clocks and omit the additional ordinary air EX Focus group. |
| Tiger Genocide | Three versions encode seven strike groups at `[67,70)`, `[71,74)`, `[76,79)`, `[100,102)`, `[103,105)`, `[106,108)`, `[109,111)`. Effects 129/130/131 use 50 damage each, zero stun, hit freeze 4/4, and initial/late guard freeze 4/4 and 8/8. This raw sequence is shared by both builds. |
| Tiger Destruction | Original `UC_GENOCIDE` has six groups across `[151,171)`, `[192,195)` and `[197,201)`, damage 30 each. At `[200,201)` it branches to the next strike stage. Latest AE2012 starts `UC_GENOCIDE_NEW`, with the same first six windows at 23 each. The follow-up strike over `[0,10)` can branch on hit at `[9,10)` to `UC_GENOCIDE_HIT_NEW`. The cinematic stage has six groups from 51 to 64. At `[54,55)` a missed group can select the alternate cinematic. |
| Tiger Cannon | AE2012 Ultra II has flash release raw 156, spawns projectile slot 10 at `[166,167)`, and actor boundary 260. Projectile velocity 0.12. Two early groups encode 3 and 4 contacts with effect 150. A final group allows one with effect 151. Both effects have raw damage 32, no stun, hit stop 6/6, chip 8. Actual landed group counts remain open. |

[Capcom AE2012 changes, Sagat on page 4](https://static.capcom.com/streetfighter/downloads/SSFIV%20Arcade%20Edition%202012%20Final%20Change%20List%20-%20US.pdf#page=4)

The `SUPER MOVE` list allows grounded specials, Super and EX Focus. In AE2012 it also includes Angry Charge and the powered Uppercut command rows. The `SUPER COMBO` group contains the three Tiger Genocide strengths and EX Focus. The `[28,30)` shot gate begins after projectile spawn. A raw gate gives the candidate interval. Contact and live input eligibility still need controller tests.

## Port and reproduction

The port maps Sagat's grounded special families and Super/Ultra inputs. Angry Scar is available outside the original card for one stock. Step Low, Step High, Heavy Tiger Elbow and Fake Kick have no Sagat SFIV command entries. The port projectile path defaults to one contact for SFIV EX shots, and Focus admission uses the active-special gate. These are static differences. [Port Sagat command dispatch](../../crates/sf-engine/src/era_fight/sf4.rs#L301) · [Port Focus admission](../../crates/sf-engine/src/era_fight/sf4.rs#L651) · [Port projectile defaults](../../crates/sf-engine/src/era_fight.rs#L1592)

Extract the original EMZ, inflate raw deflate after the 16-byte header and extract the BAC/BCM from the expanded EMB. Original BAC script, name and reaction tables are at the offsets stored at 0x0c, 0x10 and 0x14. Original BCM move stride is 68 bytes and its action index is at row +0x1c. Check original action 300 for Step Low and VFX action 518 for EX high effects 103/107. Decode AE2012's adjacent members with UltraDiff and follow each BCM action through BAC `HITBOX`, `CANCELS`, `FLOW`, `ETC` and `PHYSICS`. Read emitter indices from the raw VFX tables. Slots 8 and 9 are empty and omitted by the decoder's name dictionary. Slots 6 and 7 share the name `VFX_TIGRE_EX`, so the dictionary aliases their separate scripts. Resolving by emitter slot preserves the distinct high and low EX effects and collision boxes. Check `UPPER_L_AG` reaction 160 for 150 and `ANGRYCHARGE` for tick-nine flag set. Use reaction rows 0–2 for standing, crouching and air hits, 3–5 for guard. [Original data layout](ryu-sfiv-arcade-moveset-audit.md#reproduction) · [AE2012 extraction](ryu-sfiv-ae2012-moveset-audit.md#reproduction-and-remaining-evidence) · [Pinned decoder](https://github.com/dantarion/sf4tools/tree/e5c17132f3b6e63f66150c3a347a9e8f6e0d73a4/SFUltraDiff/src)

| Paired test | Evidence to capture |
|---|---|
| Free walk for 16 ticks, close/far LP near boundary and L/M/H/EX shots | Starting and final axes, 1.3 cutoff, first active projectile and first accepted retreat. Repeat both sides at equal measured gaps. |
| Step Low and Step High into a special, neutral HK twice, and Elbow | Distinguish a startup cancel from a contact cancel. Preserve first valid, last valid and first rejected input. |
| Every Knee/Uppercut strength on hit, guard and airborne target | Contact count, chip, stop, launch and landing. Sweep the Focus input one tick at a time. In AE2012 repeat after Angry Charge, including a whiff. |
| Both throws, all Super strengths and both legal Ultra loadouts | Grab/tech, cinematic admission, corner and air branches, meter writes, hit count and defended control. |

An authenticated original Super SFIV console build and paired live source/port checks remain open under #148.
