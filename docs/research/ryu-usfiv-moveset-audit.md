# Ryu moveset audit for Ultra Street Fighter IV

## Source and port

The reference is the PC Steam **Ultra Street Fighter IV v1.05** standard Ultra regulation `patch_ae2_tu3/battle/regulation/ae2_111`. Capcom identifies v1.05 for Steam in its April 21, 2015 announcement. Its standard Ultra changes list only Decapre. The archived Ryu BAC and BCM were extracted from the unmodified-files backup at Git commit `670d47e4311a8952e3d0b35039c4bc6eccdd746b`. The backup SHA-256 is `2cacd70aed9e54f17188b63577ae6cbccb4c9f7e3303911b962b4e9edfe92783`. `RYU.bac` is 230,606 bytes, SHA-256 `9b4cb7b0aa645df6f164916240c397ce3e22856ba80f235322d89b4600f3b6e2`. `RYU.bcm` is 11,652 bytes, SHA-256 `4a340fb3147e9a6f04ece66bd2c528fccb07cd3dea10843150f7b0c5e216cfdf`. UltraDiff's release map names this path Ultra v1.05. The hashes identify the extracted regulation files. The source executable, its region settings and a live Steam installation were outside this pass. [Capcom v1.05 announcement](https://news.capcomusa.com/lets/browse/usfiv-ver-1-05-patch-coming-tomorrow) · [Pinned unmodified backup](https://github.com/EternalStar/USF4-Bugfix/blob/670d47e4311a8952e3d0b35039c4bc6eccdd746b/Original%20Unmodded%20Files.zip) · [Pinned UltraDiff release mapping and decoder](https://github.com/dantarion/sf4tools/tree/e5c17132f3b6e63f66150c3a347a9e8f6e0d73a4/SFUltraDiff/src)

The port baseline is Git commit `0e336743760689ccdd192cbe4ebce5c4d638c9c4` with the working-tree #138 pacing expectations. The comparison setup is Ryu versus Ken in the USF4 Era Style with Era Default gauges and Classic controls. It includes Ultra 1 Metsu Hadoken, Ultra 2 Metsu Shoryuken and Ultra Combo Double. Standard Ultra has Focus, Red Focus and delayed standing. Omega and other Edition Select balances are outside this revision. [Port loadout rules](../../crates/sf-engine/src/era.rs#L508) · [Capcom Ultra manual, pp. 8–9](https://static.capcom.com/manuals/usfiv/USFIV_360_DMNL_EN.pdf) · [Spec #138](https://github.com/johnstreetfighter/sf1-port/issues/138)

## Command inventory

The BCM has 88 rows, 10 motion definitions and 20 cancel lists. The BAC has 230 named actions, 11 projectile or effect actions and 120 populated reactions. The rows below account for all 88. Numpad directions assume Ryu faces right.

| BCM rows | Inputs and variants |
|---|---|
| 0–13 | Close and far standing LP, MP, HP, LK, MK and HK, plus rapid close and far LP. Far MP uses `5MPF_NEW` and close MK `5MK_NEW`. |
| 14–21 | `2LP/MP/HP/LK/MK/HK`, rapid `2LP` and rapid `2LK`. |
| 22–33 | Six neutral jump and six directional jump normals. Directional MP uses `9MP_NEW`. |
| 34–35 | Collarbone Breaker `6MP`, Solar Plexus Strike `6HP`. |
| 36–37 | Forward dash `66`, backdash `44`. |
| 38–41 | `236LP/MP/HP` Hadoken and `236PP` EX Shakunetsu Hadoken. |
| 42–45 | `214LK/MK/HK/KK` grounded Tatsumaki. The EX row selects `TATSUMAKI_EX_NEW`. |
| 46–52 | `j.214LK/MK/HK` standard and early forward-jump air variants, and `j.214KK` EX. |
| 53–56 | `623LP/MP/HP/PP` Shoryuken. LP and HP select `SYOURYU_L_NEW` and `SYORYUKEN_H_NEW`. |
| 57–59 | `236236LP/MP/HP` Shinku Hadoken. |
| 60–61 | `236236PPP` Metsu Hadoken and `236236KKK` Metsu Shoryuken. The U2 row selects `METU_SHOURYUU_START` and has separate cinematic continuations. |
| 62–63 | `5/6+LP+LK` Seoi Nage and `4+LP+LK` Tomoe Nage at close range. |
| 64–68 | `MP+MK` Focus levels 1–3, ground and air EX Focus cancel. |
| 69 | Neutral `HP+HK` Appeal. |
| 70–73 | Alternate Shoryuken motion rows `SHORYUKEN_2`, each pointing to its ordinary strength's attack script. They appear in the `2MK_SHORYUKEN` cancel list. |
| 74–79 | Red Focus `LP+MP+MK`, full charge and two release levels. Four full-charge entries point to the same action and encode the same button chord. |
| 80–87 | EX Red Focus ground and air cancel rows, four each, all pointing to `SUPER_SAVING_EX`. |

All six standing close/far pairs use the 1.2 native-unit cutoff. Standard air Tatsumaki requires height 0.5 and EX 0.25. Forward jump selects the forward air Tatsumaki list over raw `[0,15)` and the standard list over `[15,35)`. Both lists contain EX. EX specials cost 250 Super units, EX Focus and neutral Red Focus 500, EX Red Focus 750, and Shinku 1000. Both Ultra rows encode a 200-unit Revenge requirement and a 1000-unit drain field. The published game system displays Ultra availability from 50% and drains the remaining gauge. [Capcom Ultra manual](https://static.capcom.com/manuals/usfiv/USFIV_360_DMNL_EN.pdf)

## Pacing and revision anchors

**Match on the agreed #138 endpoints.** The source forward walk script sets `0.045000001788` native units per tick, with zero horizontal acceleration. The port's 64 px cutoff divided by the source 1.2-unit cutoff gives 38.4 px per 16 ticks, rounded to 38 in the regression. Far LP has a raw `[8,11)` strike window and raw action boundary 22. Light Hadoken creates its projectile at raw `[25,26)` and has raw action boundary 58. The port expects far LP at offset 3, three active ticks and first retreat at offset 12, with light Hadoken first retreat 33 ticks after a live projectile. The regression covers U1, U2 and Double. The raw values use the BAC animation clock. Rate commands and source timing conventions account for the different port-facing endpoints. [Pacing comparison](2026-11-13-ryu-era-pacing.md) · [Port pacing regression](../../crates/sf-engine/tests/st_ryu_pacing.rs#L159)

| Property | Ultra v1.05 evidence | Earlier AE2012 source |
|---|---|---|
| LP/MP/HP Hadoken travel | VFX horizontal velocity 0.055 / 0.070 / 0.090 native units per tick. All three encode zero acceleration. | 0.060 / 0.070 / 0.080. The directions of change agree with Capcom's v1.04 notes. |
| Close HK | Raw action boundary 54, second hit duration field 20, `SUPER COMBO` cancel on `[24,28)` for the second hit. Capcom gives 17 recovery frames. | Boundary 52, standing second-hit duration field 17, no second-hit cancel group. Capcom gives 15 recovery frames before the Ultra v1.04 change. |
| Crouch MK | Raw `[10,15)`, reaction 32. Standing guard defender hitstop 11. Capcom gives -2 on block at the cited final Ultra change. | Same raw strike window, standing guard defender hitstop 10. |
| Directional jump MP | `[21,24)` and `[24,28)`, first hit `JMP_TATSU` cancel list. | Same two windows, no first-hit air Tatsumaki cancel. |
| EX Hadoken | Same raw `[25,26)` spawn as the ordinary move. Initial animation multiplier 2.7. Capcom's final change gives 11-frame startup. | Raw spawn `[25,26)`, initial multiplier 2.5. Capcom's final change gives 12 as its prior reference. |
| Air EX Tatsumaki | Second box height 0.22 and last three 0.23. Juggle-potential fields 7 through 11 across the five boxes. | All five box heights 0.20. Juggle-potential fields 0 through 4. |
| U2 opener | Raw `[84,85)` box height 0.75 and `[85,86)` 1.0. Speed changes to 0.5 at raw 84 and 1.0 at 85. Capcom's final Ultra list gives nine game frames of invincibility. | The audited arcade balance dispatches `UC2_NEW` with the widened box. The available regular `METU_SHOURYUU_START` script in that data has the 0.75 box throughout `[84,86)`. |

[Capcom Ultra final change list, Ryu](https://news.capcomusa.com/combofiend/blog/2014/05/22/usfiv-final-character-change-list) · [Capcom v1.04 adjustments](https://news.capcomusa.com/lets/browse/usfiv-version-1-04-adjustments) · [AE2012 direct source audit](ryu-sfiv-ae2012-moveset-audit.md)

The archived SRK maintainer post dates the Ultra frame-table update to v1.05. SuperCombo preserves the Ryu table, retrieved through the MediaWiki API as revision `344689`. Its far LP is 4 startup, 3 active and 6 recovery, with 12 total. It lists close HK recovery 17, crouch MK guard advantage −2, ordinary Hadoken 13 startup and 32 recovery, and travel speeds `0.055 / 0.070 / 0.090`. EX Hadoken lists 11 startup and 28 recovery. Use this for public-facing timing and outcome cross-checks and the BAC/BCM for raw animation bounds and cancel groups. The legacy AE page has its own balance and gives EX Hadoken startup 12. [Archived v1.05 table update](https://archive.supercombo.gg/t/ultra-street-fighter-4-srk-wiki-has-had-a-major-update/176249) · [Pinned Ultra Ryu table](https://wiki.supercombo.gg/api.php?action=parse&oldid=344689&format=json) · [Legacy AE Ryu table](https://wiki.supercombo.gg/api.php?action=parse&oldid=166985&format=json)

The SRK archive on Shib has an alternate Ultra index and Ryu URL. Both returned HTTP 403 during this check, so the captured SuperCombo revision `344689` remains the basis for the v1.05 table comparison. [SRK Ultra index](https://srk.shib.live/w/Ultra_Street_Fighter_IV) · [SRK Ultra Ryu](https://srk.shib.live/w/Ultra_Street_Fighter_IV/Ryu)

FAT's Ultra data is a spreadsheet-derived lookup with matching far LP, close HK and crouch MK values for those fields. Its older Ryu Hadoken `recovery` field holds 45, the total duration. Its EX Hadoken field holds 40, one tick above the pinned wiki total. Keep the column convention and this difference visible in any comparison. [Pinned Ultra FAT JSON](https://raw.githubusercontent.com/D4RKONION/FAT/dee6b85a7f8f166701ff3f8ab6cfa954949d99aa/src/js/constants/framedata/USF4FrameData.json) · [FAT app](https://fullmeter.com/fat/)

## Representative source contact and system values

The windows are half-open raw animation intervals. Damage and stun are health and stun-meter fields. Freeze is attacker/defender. Push is the reaction's native displacement parameter. The same record can be reused across windows. Live hit count, elapsed stun and displacement depend on the target, collision, gauge and cancel state.

| Move | Raw windows or reaction IDs | Standing damage, stun and chip | Freeze, push and reaction |
|---|---|---|---|
| Far LP | `[8,11)` 12, mid | 30, 50, 0 | 8/8 on hit and guard, 0.3 |
| Crouch LK | `[5,8)` 30, low | 20, 50, 0 | 8/8 on hit and guard, 0.3 |
| Crouch MK | `[10,15)` 32, low | 60, 100, 0 | 10/10 hit, 10/11 standing guard, 0.5 |
| Sweep | `[12,14)` 34, low | 90, 100, 0 | 12/12 hit and guard, 0.7, sweep reaction script |
| Close HK | `[14,18)` 10 and `[24,28)` 11, mid | 40+70, 125+75, 0 | 12/12. Hit push 0.3 both. Guard push 0.3 then 0.7. |
| Collarbone Breaker | `[19,21)` 80 and `[22,24)` 81, overhead | 30+50, 50+50, 0 | 10/10, 0.5 both, standing `DAMAGE_LOWER` |
| Solar Plexus Strike | `[19,21)` 82 and `[21,23)` 84, mid | 40+60, 50+50, 0 | 12/12 then 10/12, 0.2 both, standing `DAMAGE_MH` |
| Forward/back throw | Grab `[3,5)` 90, finish IDs 91/92 | 130 each, 140/120, n/a | Finisher 0/0, `BOUND_UPWARD_H` |
| Hadoken and EX Fire | VFX 100/101/102 ordinary, 103+104 EX | Ordinary 70, 100, 15. EX 50+50, 50+50, 12+12. | 8/8 both. Ordinary push 0.8, EX 0.5 then 0.8 with blowback. |
| Shoryuken LP/MP/HP/EX | `[10,12)` first and `[12,24)` later, pairs 117/118, 119/120, 125/126, 123/124 | First damage 100/80/160/80, later 70/50/60/60. HP chip 40/15. | First hit 14/14, late 12/12. Early LP/MP guard 15/14. Push 0.8. |
| Ground Tatsumaki LK/MK/HK | First `[19,21)`, IDs 105–107 | 100/110/120, 200, 25/27/30 | 14/14 hit and guard, 0.8, blowback. LK has two raw windows and MK/HK have five. |
| Air Tatsumaki LK/MK/HK | `[19,21)`, `[27,29)`, `[35,37)`, IDs 108–110 | 70/80/90, 100, 17/20/22 | 14/14, 0.8, blowback. Both air banks use the same reactions. |
| Ground/Air EX Tatsumaki | Five raw windows, IDs 65/113 ground and 114/116 air | Ground 30 × 4 + 40, 40 each, 7 × 4 + 10. Air 40, 50 and 10 per contact. | Hit 5/1 ground or 3/1 air. Guard 5/5. Early push -1.5 ground or -1.0 air, final 1.0 both. |
| Focus levels 1/2/3 | IDs 60/61/62 at `[14,16)`, `[14,16)` and `[74,76)` | 60/80/140, 100/150/200, 0 | Hit 16/16, 30/30, 30/30. Guard rows 12/16, 24/30, 22/30. L2/L3 crumple. |
| Red Focus levels 1/2/3 | IDs 67/68/69, same raw windows. EX Red uses 69. | 90/120/210, 100/150/200, 0 | Same encoded hit and guard stop as ordinary Focus. L2/L3 crumple. |
| Shinku Hadoken | Spawn `[54,55)`, VFX 128 × 4 and 129 | 50 × 4 + 100, 0, 12 × 4 + 25 | 8/8, push 0.5 then 0.8, blowback finish |
| Metsu Hadoken | Spawn `[166,167)`, VFX 130 × 7 and 131 | Base 28 × 7 + 50, 0, 7 × 7 + 12 | 8/8, hit push 0.3 then 0.8. Guard uses distinct reaction and initial 0.4. |
| Metsu Shoryuken | Opener 134, late 140/141, cinematic 136/135/137 and late 0 | Opener 0, late fields 25/33, principal cinematic 180/30/125 | Opener 0/0 on hit, 30/30 on guard. Late 4/4 on hit and 14/14 on guard. |

The other normal strengths have the same raw contact windows as in the AE2012 inventory. Their Ultra raw values can be reproduced by resolving BCM rows 0–33 into the source BAC hitboxes. Air normals are overhead. Crouch LK, MK and HK are low. Rapid variants reuse the light reaction IDs. Forward and backdash raw action boundaries are 11 and 30. Backdash starts with a full hit, throw and projectile invulnerability record over `[0,8)`. The source scripts contain several recovery actions including long down variants. Which knocks can be delayed and the first accepted button tick require live checks. Capcom describes two buttons on a hard knockdown adding 11 frames on the ground. [Capcom Ultra system explanation](https://news.capcomusa.com/combofiend/blog/2014/01/30/ultra-street-fighter-4-change-list)

## Port comparison

| Property | Status |
|---|---|
| Ground specials, Shinku and both Ultra inputs | **Match for mapped inputs and loadouts.** Port maps each normal and EX ground motion, `236236P` Super, and the `236236PPP` and `236236KKK` Ultras according to loadout. Contact timing and outcome remain **unverified** for each version and route. |
| Command normals, air Tatsumaki and Classic throw input | **Verified dispatch differences.** Ultra data exposes both forward punch command normals and all seven air Tatsumaki rows. The port canonical SFIV table omits those actions. The standard source ground throws use the light-pair chord. Port Classic canonical Ryu uses the shared directional heavy-button dispatch. |
| EX Fire and U1 projectile limit | **Verified configuration differences.** Source EX Fire encodes two contacts and U1 eight. Port `0x1a` and `0x2a` use one-contact limits. Source and port Shinku have a configured five-contact limit. Live count, damage, reaction and distance remain **unverified**. |
| U2 and Double | **Match for available commands and tested port scaling.** Port regression verifies both Ultras and shared Revenge spend in Double, and compares U2 damage at 75% of standalone. Capcom places Ryu in the 75% tier. The cinematic transition, air/late contact branches and per-hit scaling still need live comparison. |
| Focus, Red Focus and delayed standing | **Match for tested system anchors.** Port tests cover ordinary armor, Red multi-absorption and two-stock cost, three-stock EX Red, and the 11 extra lying frames on a tested knockdown. The source manual specifies the chords and stock costs. Exact armor intervals, crumple, damage and wakeup eligibility remain **unverified**. |
| HP Shoryuken Focus | **Verified difference in the configured cancel gate.** Source heavy DP has no cancel group. The port's native special gate admits Focus during slot `0x19` without a strength condition. Live accepted offsets remain **unverified**. |
| Close HK, directional jump MP, 2MK and projectile speed deltas | **Unverified** for matched live outcomes. The source close HK second hit goes to `SUPER COMBO` (Shinku, EX Focus, EX Red). First directional jump MP goes to `JMP_TATSU`, which contains all normal and EX air strengths. Crouch MK goes to `SUPER MOVE` over raw `[1,15)` and `[10,15)`. The alternative DP list exists in the BCM, while the audited crouch MK action references `SUPER MOVE`. |

[Port SFIV commands and Focus handler](../../crates/sf-engine/src/era_fight/sf4.rs) · [Port native system tests](../../crates/sf-engine/tests/sf4_native.rs) · [Port projectile limits](../../crates/sf-engine/src/era_fight.rs#L956) · [Port throw dispatch](../../crates/sf-engine/src/fighter.rs#L8737) · [Capcom Ultra manual](https://static.capcom.com/manuals/usfiv/USFIV_360_DMNL_EN.pdf) · [Capcom Double and delayed standing reference](https://news.capcomusa.com/combofiend/blog/2014/01/30/ultra-street-fighter-4-change-list)

The existing release test binary `sf4_native-f84c3b7aca3f0778.exe` ran from `crates/sf-engine` with local assets and passed all 15 tests. The existing pacing harness `st_ryu_pacing-7d71ddcb63221474.exe` passed the all-revision pacing test during the AE2012 audit, including the USF4 loadout matrix. No new port build was used for these checks.

## Reproduction and remaining live evidence

Extract the two `Unmodded/patch_ae2_tu3/battle/regulation/ae2_111/RYU/RYU.*` members from the pinned backup and verify the hashes. With the two files adjacent, load them using UltraDiff's `BACFile` and `BCMFile`. Resolve `BCMFile["Moves"][name]["Script"]` to the BAC action. Inspect its `HITBOX`, `CANCELS`, `SPEED`, `PHYSICS` and `INVINC` command lists. `HitboxData` resolves the reaction in `HitboxTable`. Standing, crouching and air hit rows are 0–2, and corresponding guard rows 3–5. Projectile `ETC` commands of type `VFX_SCRIPT` index the ordered `VFXScripts` table. As extraction checks, inspect close HK `IASAFrame=54` and reaction 11, EX Red action `SUPER_SAVING_EX` with reaction 69, and LP/HP Hadoken VFX velocities 0.055 and 0.090. [UltraDiff decoder](https://github.com/dantarion/sf4tools/tree/e5c17132f3b6e63f66150c3a347a9e8f6e0d73a4/SFUltraDiff/src)

Run Ryu versus Ken in Steam v1.05 using the standard Ultra balance and in port USF4 using Era Default gauges. Use U1, U2 and Double. Reset to neutral between attempts. Begin midscreen, then repeat at the corner and with sides exchanged. Log input, action start, first contact, meter and damage writes, per-side advancing/release ticks and both positions. Establish separation using the source and port close/far boundaries.

| Recipe | Remaining measurement |
|---|---|
| Far LP, crouch LK/MK/HK, close HK, `6MP`, `6HP` on stand, crouch, guard and air | Hitbox reach, hit and guard stop, stun, push, overhead dispatch, wakeup and close HK second-hit cancel. |
| `2LK→2LK`, `2MK→236LP/623P`, `6HP→623MP`, close HK second hit → Shinku or EX Red | Sweep the final input one tick at a time. Record chain/link/cancel and outcome on hit, guard and whiff. |
| First directional `j.MP→214K/KK`, early and late forward jump `214K/KK`, and anti-air juggle | Air variant, route acceptance, height threshold, landing and reaction, EX five-window juggle. |
| `236LP/MP/HP/PP` at two fixed gaps on hit, guard and whiff, then against a projectile | Spawn, speed, live hit count, clash, damage, push and recovery. |
| `623LP/MP/HP/PP→MP+MK→66`, repeat with `LP+MP+MK` | Separate early/late, grounded/air, hit/guard/whiff. Measure cancel windows and cost, HP restriction and blocked LP/MP advantage. |
| Shinku at full Super and each Ultra at first legal and full Revenge, alone and in Double | Flash, contact count, damage/chip and meter. Repeat U2 on point-blank ground, at tip, versus airborne Ken and after DP FADC to find cinematic and non-cinematic branches. |
| Both throws, sweep knockdown, ordinary/Red Focus at each charge, dash and appeal | Throw range and tech, ordinary or delayed standing eligibility, armor and crumple, cancel cost, dash displacement and recovery. |

This pass uses extracted shipped data, Capcom's documentation and port code/tests. Matched source and port combat captures remain open under #148. [Direct SFIV evidence #148](https://github.com/johnstreetfighter/sf1-port/issues/148)
