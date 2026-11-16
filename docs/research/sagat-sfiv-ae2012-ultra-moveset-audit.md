# Sagat special moves and Focus in AE2012 and Ultra Street Fighter IV

Source evidence for [#148](https://github.com/johnstreetfighter/sf1-port/issues/148). The [arcade audit](sagat-sfiv-arcade-moveset-audit.md) covers the original SFIV baseline. This companion uses the shipped AE2012 and late standard Ultra scripts for all four strengths in each special family, Tiger Genocide, both Ultras, and the ordinary and Red Focus release and cancel paths.

## Source identity

AE2012 uses the verified Taito Type X2 NESYS 1.3 archive and its `patch/battle/regulation/latest/SGT/` members. Standard Ultra uses `Unmodded/patch_ae2_tu2/battle/regulation/ae2_110/SGT/` in the unmodified PC backup, archive SHA-256 `2cacd70aed9e54f17188b63577ae6cbccb4c9f7e3303911b962b4e9edfe92783`. UltraDiff identifies `ae2_110` as v1.04. The backup has an `ae2_111` directory with no Sagat member, and Capcom's v1.05 notes list Sagat's Super change under Omega. The standard audit therefore uses the retained v1.04 files for v1.05. [Arcade source and hashes](sagat-sfiv-arcade-moveset-audit.md#source-builds) · [Pinned Ultra backup](https://github.com/EternalStar/USF4-Bugfix/blob/670d47e4311a8952e3d0b35039c4bc6eccdd746b/Original%20Unmodded%20Files.zip) · [Pinned decoder and version map](https://github.com/dantarion/sf4tools/tree/e5c17132f3b6e63f66150c3a347a9e8f6e0d73a4/SFUltraDiff/src) · [Capcom v1.05](https://news.capcomusa.com/lets/browse/usfiv-ver-1-05-patch-coming-tomorrow)

| Ultra member | Bytes | SHA-256 |
|---|---:|---|
| `SGT.bac` | 262609 | `e0dba028d2f1ac9e153be62f2882744e5d51c5b077228789b668ee9711864702` |
| `SGT.bcm` | 12286 | `87f24698172ab0f4ef8e60d59b33691a3272e5080c83d3bb62cdc7d8683fbd76` |

## Commands and cancels

Both builds route high Tiger Shot `236P`, Grand Tiger Shot `236K`, Tiger Knee `623K`, Tiger Uppercut `623P`, Angry Charge `214214K`, Tiger Genocide `236236K`, Tiger Destruction `236236KKK`, and Tiger Cannon `236236PPP`. Each special family has L/M/H and two-button EX rows. Angry Charge and EX specials cost 250 raw Super, Tiger Genocide 1000. The ordinary Ultras have raw Revenge requirement 200 and cost 1000. All moves are non-charge motions in the input table. Ultra keeps these command rows and adds Red Focus, delayed wakeup and alternate Ultra dash-entry rows. [Capcom Super SFIV development interview](https://news.capcomusa.com/lets/browse/super-street-fighter-iv-dev-blog-changes-to-guile-dhalsim-balrog-vega-and-m-bison) · [Capcom Ultra system notes](https://news.capcomusa.com/lets/browse/ultra-street-fighter-4-change-list)

| Script family | Source buffer and execute intervals |
|---|---|
| All Tiger Shot strengths, high and low | `SUPER COMBO` `[14,30)` and `[28,30)`, after the projectile spawn for the execute row. |
| All Tiger Knees | `SUPER COMBO` `[2,10)` and `[7,10)`. |
| Ordinary and powered Tiger Uppercut, L/M/H/EX | `SUPER COMBO` `[1,14)` and `[10,14)`. Ordinary L/M/H also have `SAVING_EX_AIR` in both windows. Ordinary EX and all powered strengths lack the separate air list. |
| Close standing LK | AE2012 `SUPER MOVE` execute `[13,14)`. Late Ultra extends it to `[13,17)`, covering the second hit. Far `5LKF_NEW` retains `[13,14)`; rapid LK uses a different action. |
| Step Low and Step High | Both retain their startup special-cancel gates. Late Ultra reduces Step Low guard push from 0.50 to 0.25. Step High still strikes over `[24,26)` and its damage goes from 100 to 80. |
| Ultra cancel destinations | `SUPER MOVE` and `SUPER COMBO` add grounded EX Red Focus. `SAVING_EX_AIR` adds EX Red Focus air entries after ordinary EX Focus. |

`SUPER COMBO` includes all three Super strengths and EX Focus. The encoded conditions identify candidate timing. Hit, guard, whiff, input buffer and first advancing-tick acceptance require live input traces.

## Specials and Super

Intervals are half-open local BAC times. The source advances some actions at variable rates. Reaction timers and push values are authored fields for the defender.

| Family | AE2012 and late Ultra contact evidence |
|---|---|
| Ordinary high Tiger Shot | Spawn `[24,25)` to slots 0–2. L/M/H VX is +0.06/+0.08/+0.10. Each uses 65 damage, 100 stun, 16 chip, 8/8 stop and timer 18 in both builds. Ultra raises ordinary actor IASA from 61 to 64 and adds a 1.0 rate write at 61. EX stays at 61. Capcom's launch notes give 39→42 for recovery. |
| Ordinary Grand Tiger Shot | Spawn `[25,26)` to slots 3–5. Same L/M/H velocity sequence, 60 damage, 100 stun, 15 chip, 8/8 stop, timer 18. IASA 68 persists. High projectiles have paired boxes in each of `[0,2)` and `[2,61)`; low projectiles have one in each interval. Each phase uses one reaction per strength. |
| EX high/low Tiger Shot | Slots 6/7 independently use reactions 200/201 and 205/206. Both sequences encode 70+50 damage, 50+50 stun, 18+13 chip, stop 8/8 and hit/guard timer 12. High velocity changes 0.11→0.10 at projectile tick 21 and low stays 0.11. High has two overlapping coverage sets with different juggle eligibility. Low has one set. Both revisions preserve two distinct contact effects. |
| Tiger Knee L/M/H | Opening `[7,9)`, late `[9,18)` L or `[9,19)` M/H. Damage 90+30, 100+40, 110+50. Chip 22+7, 25+10, 27+12. Stop 14/14, hit timer 20/20, guard 20/17. Ultra changes grounded opening reactions 108/110/112 from `HIT/DAMAGE_ML` to `SCRIPT/DAMAGE_UPPER` and preserves distinct airborne reactions. The publisher describes the ordinary first hit forcing stand. |
| EX Tiger Knee | `[7,10)` effect 114 for 100 damage/100 stun, then `[10,15)` and `[15,20)` reuse effect 115 for the 40 damage/50 stun group. The latter windows do not alone assert two late contacts. Chip 25+10. Ultra's opening also changes to `SCRIPT/DAMAGE_UPPER`, with stop 8/8→14/14 on hit and guard. Ultra late guard changes 8/8→7/8 and guard timer 17→15. All four versions may branch to their matching landing action during `[21,28)`, each with authored landing total 9. |
| Ordinary Tiger Uppercut L/M/H | Early `[10,12)` for 120 damage/30 chip. Late `[12,14)` and continuation through 21/23/24 reuse effects 117/119/121, damage 130/140/160, chip 33/35/40. Stop 14/14, push 0.9 early and 0.8 late. Ultra changes all three first standing-guard timers 20→16. Capcom specifies −5 after EX Focus into forward dash on block. |
| Angry Charge and powered Uppercut | Charge takes one stock and sets the flag at raw 9, releases the superflash at 27 and has raw total 40. The powered action clears the flag at raw 4 for L/M/H or 3 for EX. L/M/H damage is 150/170/180 whichever early or late standing row qualifies. All three share Ultra's first guard timer 20→16. |
| EX Uppercut, normal and powered | Early `[10,15)` gives 120 or 170 damage with 14/14 stop. Four subsequent 2-tick boxes through 23 use the corresponding 20-damage, 5-chip effect, 6/6 stop. Both EX families retain first-guard timer 20 and use the usual `SUPER COMBO` timing. |
| Tiger Genocide L/M/H | Superflash release event at 67. Seven strike windows `[67,70)`, `[71,74)`, `[76,79)`, `[100,102)`, `[103,105)`, `[106,108)`, `[109,111)`. L uses 129–131, M 132–134, H 135–137. Damage 50 and chip 12 each. Hit stop 4/4 throughout. Guard stop 4/4 in the first three, 8/8 on the four later rows. Ultra adds condition rows `[134,149)` and `[148,149)` with raw list `0xffffffff`. The sentinel leaves the destination unresolved. |

[Capcom AE2012 changes](https://static.capcom.com/streetfighter/downloads/SSFIV%20Arcade%20Edition%202012%20Final%20Change%20List%20-%20US.pdf#page=4) · [Capcom Ultra launch changes](https://news.capcomusa.com/combofiend/blog/2014/05/22/usfiv-final-character-change-list)

## Focus and Red Focus

| Route | Source release and reaction |
|---|---|
| Normal Focus charge | `SAVING_MAX` releases to `SAVING2` at `[13,21)` or `SAVING3` at `[21,60)`. It has a raw absorb-related record over `[0,74)`, charge dash execute `[4,60)`, then full-charge contact and dash execute `[74,76)`. |
| Normal Level 1 | `[14,16)`, effect 60, 60 damage/100 stun. Hit stop 16/16, guard 12/16. Hit/guard timer 10 and push 0.7. Grounded ordinary hit uses `DAMAGE_MM`. |
| Normal Level 2 | `[14,16)`, effect 61, 90/150. Grounded crumple, 30/30 stop and push 0.5. Guard 24/30 and push 0.7. Timer 14. |
| Normal Level 3 | `[74,76)`, effect 62, 150/200. Ground crumple, hit stop 30/30 and push 0.5. Authored guard row 22/30 and push 0.7. Hit/guard timers 20/14. Full-charge guardability needs the system state. |
| EX Focus | Ground and air rows use `EX_SAVING`, cost 500 and reference the same normal release stages and full-charge contact. The EX entry has no absorb-related record in its own action. Both level-release scripts execute follow-up dash over `[14,16)`. |
| Ultra Red Focus | Input LP+MP+MK, cost 500, script `SUPER_FOCUS_MAX`. Same release windows and attack clocks as ordinary Focus. Level 1/2/3 effects 75/76/77 give 90/135/225 damage at the same 100/150/200 stun and corresponding stop/timers as ordinary Focus. The full charge moves the absorb-related end from raw 74 to 75. |
| Ultra EX Red Focus | The ground and air commands cost 750. `SUPER_FOCUS_EX` has the same release windows, with EX L1/2 continuations. EX L1 uses effect 78, 90 damage/100 stun, grounded crumple, stop 30/30 hit and 24/30 guard, timer 14. EX L2 and full charge reuse 76/77. |

[Capcom Red Focus and EX Red Focus rules](https://news.capcomusa.com/lets/browse/ultra-street-fighter-4-change-list)

## Ultras and Ultra Double

**Tiger Destruction.** The ordinary opener in both builds has six strike windows, starting at raw 151, 156, 161, 166, 192 and 197, each 23 damage and 4/4 stop. Its short bridge executes at `[200,201)`. The bridge's `[9,10)` hit condition selects the cinematic action, whose six groups from raw 51 through 64 deal 28 five times then 30. The initial cinematic contact has 30/30 stop and the rest 4/4. Late Ultra adds a check at `[170,171)` and air fallback on whiff. Its `LOWDMG1` bridge checks for the wall over `[0,2)`, and the following `HIT_LD` and `HIT2_LD` change the last two raw damage values to 10 and 11. This accounts for a distinct low-damage route in the source. Cabinet contact counts, spacing and late Ultra's published airborne damage of 340 need live confirmation.

**Tiger Cannon.** The actor releases superflash at 156, emits slot 10 at `[166,167)` and has raw IASA 260. The projectile uses VX 0.12 and duplicates the 3+4+1 contact groups across `[0,2)` and `[2,50)`. AE2012 uses reactions 150/150/151 with damage 32 per contact, 8 chip, stop 6/6 and timer 20. Late Ultra uses 149/150/151 with juggle potential 127, standing and crouching damage 26/32/32 and airborne 20/28/30. At the 1.5× multiplier implied by Capcom's published full-gauge totals, 8×32 gives the AE2012 384, and late Ultra's ground 238 and airborne 202 give 357 and 303. Capcom's Ultra launch notes gave 255 on air hit. The v1.04 notes confirm 303. Scaling interpolation and individual hit rounding still need live checks. [Capcom Ultra launch changes](https://news.capcomusa.com/combofiend/blog/2014/05/22/usfiv-final-character-change-list) · [Capcom v1.04 balance changes](https://news.capcomusa.com/lets/browse/usfiv-version-1-04-adjustments)

**Ultra Double.** Capcom's 75% class includes Sagat. Its system notes make both ordinary Ultra commands available with the chosen loadout. Ultra also ships `UC01_CANCEL_DASH` and `UC02_CANCEL_DASH`, with raw requirement/cost 400/2000. Each separate dash has wall and hit branches. The Ultra II cancel branch spawns separate slot 12, reactions 190/191 at 21 damage per contact. The source connection between the alternate commands and the Super's unresolved sentinel remains unverified. [Capcom Ultra Double rules](https://news.capcomusa.com/lets/browse/ultra-street-fighter-4-change-list)

## Extraction and live checks

The [contact matrix](sagat-sfiv-ae2012-ultra-contacts.tsv) has 634 data records. AE2012 supplies 53 fighter actions, nine projectile slots, 146 contacts and 63 distinct reactions. Ultra supplies 73 fighter actions, ten projectile slots, 200 contacts and 80 reactions. The action selection includes the four Tiger Shot, Knee and Uppercut strengths, both projectile heights, four charged Uppercuts, Focus families, activated and alternate finishers, proximity LK and Step Kick anchors, plus named `FLOW` destinations. Legacy action rows are included for byte-context and only the listed `input_rows` or a reachable `FLOW` edge establishes a candidate route.

1. Extract the hashed BAC/BCM pairs side by side. Use the pinned decoder's `BACFile`, `BCMFile` and `BACFile.readScript`. Preserve BAC main slot count/pointers from header `0x0c/0x14/0x1c`, VFX slot count/pointers from `0x0e/0x18/0x20`, and reaction pointer table from `0x24`. Counts are little-endian u16, pointers u32. Read each selected projectile by slot from the VFX pointer table after resolving `ETC.VFX_SCRIPT.RawParams[0]`. Restore the decoder's `HitBoxUsage` field to `defaultdict(set)` before calling `readScript` again on the existing decoded instance. Slots 8 and 9 are empty in both builds, and Ultra's 11 is empty. The emitted U2 and alternate U2 are 10 and 12.
2. Preserve each action's raw IASA/total, BCM inputs, and `STATE/SPEED/PHYSICS/INVINC/CANCELS/FLOW/ETC/TARGETLOCK` groups. The table stores action data and decoded input definitions as JSON cells. `input_rows` entries are `[BCM index, name, fields]`. Only control events are serialized from ETC. For each non-proximity attack box preserve the listed contact fields and join `HitboxData` to its revision's reaction record.
3. The twelve reaction cell arrays preserve the decoder's order. Rows 0–2 are standing/crouching/air hit, 3–5 standing/crouching/air guard, 6–8 counter-hit, and 9–11 unresolved. Each array is `[Damage, Stun, Effect, OnHit, SelfHitstop, TargetHitstop, TargetAnimationTime, PushbackDistance, VelocityX, VelocityY, VelocityZ, AccelerationX, AccelerationY, AccelerationZ, JuggleStart, MiscFlag, SelfMeter, TargetMeter]`. Retain the version and raw timestamps when reusing targets. [Pinned BAC decoder](https://github.com/dantarion/sf4tools/blob/e5c17132f3b6e63f66150c3a347a9e8f6e0d73a4/SFUltraDiff/src/bac.py)

| Live control | Evidence still needed |
|---|---|
| Tiger Shot, Grand and EX at different target heights and through clashes | First live box, clean/guard hit count, confirmation of the physical low projectile and actor recovery. |
| Tiger Knee and Uppercut with each strength, normal/charged where applicable | Landed links and launch, first advancing tick on hit or guard, contact-specific EX Focus and EX Red Focus entry, dash advantage, meter debits and Angry Charge through whiff. |
| All Focus levels, EX variants, dash/backdash, and Red Focus against multihit and armor breakers | Hit absorption and release boundaries, held-input latching, crumple, guardability, dash spending and health/meter effects. |
| Both Ultras at matched Revenge, stand/crouch and several airborne heights, midscreen and corner | Group selection, raw-to-live damage, full-gauge totals, hit count, cinematic selection, Double damage ratio and alternate dash-entry eligibility. |
