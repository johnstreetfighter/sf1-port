# Adon moveset audit for Super Street Fighter IV, AE2012 and Ultra

Source evidence for [#148](https://github.com/johnstreetfighter/sf1-port/issues/148). The port offers Adon in Super SFIV, AE2012 and Ultra. The original 2008 Type X2 archive and Capcom's 2009 PC benchmark both lack an `ADN` character package. Capcom's Super SFIV development blog and console manual establish Adon's first SFIV-series roster appearance and the four special move families. The pinned Super→AE game-data diff supplies an additional versioned source for Super. Full standalone Super BAC/BCM and paired gameplay traces remain open. [Port revision registry](../../crates/sf-engine/src/era.rs#L486) · [Capcom Super SFIV Adon development blog](https://news.capcomusa.com/lets/browse/super-street-fighter-iv-dev-blog-adon) · [Capcom Super SFIV manual](https://static.capcom.com/manuals/ssfiv/SSFIV_360_DMNL_EN.pdf) · [Pinned Super→AE diff](https://github.com/dantarion/sf4tools/blob/e07764262003f3aa1feff1a0a5a66fc3c14efb43/characters/ADN/Super_TO_AE.html)

## Source builds

AE2012 uses Japanese Type X2 NESYS 1.3. The archive SHA-1 is `dd2d5cf64e9d35dd1b3da772c9d72b906b860026`, the `game.exe` MD5 is `15432af2c3ec96b7dec196477b0e3591`, and the normalized executable header CRC-32 is `545b0d0c`. Latest files are under `patch/battle/regulation/latest/ADN/`. The same archive's `resource/battle/chara/ADN/` is an earlier AE comparison with no corresponding `v101/ADN/` replacement. The original archive checked for roster availability is Type X2 v1.0 STD, SHA-1 `1120092ad2dcde6a381a2305dbbb843339585c44`, launcher MD5 `e47d7c83e526dbb1986173f197801b99`. [Preservation metadata](https://archive.org/metadata/Yori-Arcade-Arcade_Games_-_TAiTO_Type_X2-RomVault) · [AE2012 fingerprint](https://github.com/teknogods/OpenParrot/blob/fc8ba632e1f67c5c340ff88f30efca743e8d38f5/OpenParrot/src/Utility/GameDetect.cpp#L449)

Ultra uses Adon's standard PC v1.04 `ae2_110` balance files retained in v1.05. The unmodified-file backup has SHA-256 `2cacd70aed9e54f17188b63577ae6cbccb4c9f7e3303911b962b4e9edfe92783`. Within it read `Unmodded/patch_ae2_tu2/battle/regulation/ae2_110/ADN/`. Neither its later `ae2_111` directory nor the pinned Adon diff shows a standard v1.05 replacement. Capcom's v1.05 notes list Adon's Super change for Omega only. [Pinned backup](https://github.com/EternalStar/USF4-Bugfix/blob/670d47e4311a8952e3d0b35039c4bc6eccdd746b/Original%20Unmodded%20Files.zip) · [Pinned file decoder and release map](https://github.com/dantarion/sf4tools/tree/e5c17132f3b6e63f66150c3a347a9e8f6e0d73a4/SFUltraDiff/src) · [Pinned Adon diff directory](https://github.com/dantarion/sf4tools/tree/e07764262003f3aa1feff1a0a5a66fc3c14efb43/characters/ADN) · [Capcom v1.05 patch notes](https://news.capcomusa.com/lets/browse/usfiv-ver-1-05-patch-coming-tomorrow)

| Member | Bytes | SHA-256 |
|---|---:|---|
| AE2012 latest `ADN.bac` | 232935 | `c2017e8644b133095002bb0f3b8c435247035d397ff186159cd444a5806dbae7` |
| AE2012 latest `ADN.bcm` | 10826 | `5e12909b6e260e0d6e9224f7e1ce2934114c196ab345e38d37dc469bc64f4465` |
| AE resource `ADN.bac` | 226971 | `da1a7fd1cc8e251314078635764d98cf2a84c208ef40a68aabb8db9255de9919` |
| AE resource `ADN.bcm` | 10400 | `bb5fdfe03b4e4c4bf2718a0d0d3e80dac00aa962ae399eaea578ff7bc4d6ca59` |
| Ultra `ADN.bac` | 264124 | `f53ded6247d99f8b654278de0e0704690291fcc4bd1575d963fb390d9132b17b` |
| Ultra `ADN.bcm` | 12554 | `2ebf9163f9072092da57b4084a5b428113b459f30704acfbae72e50b682a3b62` |

Run `BACFile` and `BCMFile` with the two files of each pair under the same stem, then resolve `Moves[*].Script` and each non-proximity `HITBOX.HitboxData` into `HitboxTable`. AE2012 latest has 207 named actions, 135 reactions, 72 commands, 15 motion definitions and 18 cancel lists. Ultra has 221 actions, 156 reactions, 88 commands, 15 motions and 18 cancel lists. Both BCMs have zero charge definitions.

The source-derived Super→AE diff shows light Air Jaguar launch X declining from 0.05 to 0.04. Super heavy Rising Jaguar had a `[5,6)` Super execute window. AE in the PC diff ends it at 7, while the arcade resource→AE2012 diff dates the same end change in the arcade data to the 2012 upgrade. Use the pinned arcade and Ultra packages for the main timing rows. [Pinned Super→AE diff](https://github.com/dantarion/sf4tools/blob/e07764262003f3aa1feff1a0a5a66fc3c14efb43/characters/ADN/Super_TO_AE.html) · [Pinned AE→AE2012 diff](https://github.com/dantarion/sf4tools/blob/e07764262003f3aa1feff1a0a5a66fc3c14efb43/characters/ADN/AE_TO_AE2012.html)

## Input and branch inventory

Numpad directions are relative to facing. Source BCM indices are zero-based. AE2012 and Ultra share these 72 base rows.

| Rows | Actions |
|---|---|
| 0–15 | Six close/far standing pairs, including rapid LP/LK variants. Each pair uses a 1.1 native unit switch except HK, which uses 1.2. The earlier arcade resource uses 1.1 for HK as well. |
| 16–35 | Six crouching normals, rapid 2LP/2LK, six neutral-jump and six directional-jump normals. |
| 36 | Jaguar Crunch, strict `6MP`. Two overhead strike groups. There is no conventional grounded target-combo row. |
| 37–38 | Dash `66`, backdash `44`. |
| 39–42 | Jaguar Tooth `63214K`, L/M/H and KK. Wall contact selects a respective attack script. |
| 43–46 | Ground Jaguar Kick `421K`, L/M/H and KK. `GYAKU_SYORYU` has back/down/back samples with buffers 6/6/8. |
| 47–50 | Air Jaguar Kick `214K` in neutral or forward jump, L/M/H and KK. Source height restricts ordinary to 0.35, EX to 0.25, versus the AE resource's 0.40/0.30. `AIR_B` has no direct kick command. Neutral jump MK prepares the air-special cancel in `[1,13)` and executes in `[5,13)`. |
| 51–54 | Rising Jaguar `623K`, L/M/H and KK. `SYORYUKEN` has forward/down/forward with buffers 8/8/12. |
| 55–62 | Jaguar Varied Assault `236236P`, L/M/H. Kick chooses Jaguar Assassin, punch chooses Thousand Jaguars and repeated punches advance up to stages 2–4. |
| 63–64 | Jaguar Revolver Ultra I `214214KKK` and Jaguar Avalanche Ultra II `236236KKK`. Raw Revenge requirement/cost 200/1000. |
| 65–71 | Forward and backward LP+LK throws, MP+MK Focus charge/release and EX Focus, and neutral HP+HK appeal. |
| Ultra 72–87 | Two alternate Ultra cancel rows with raw 400/2000 requirement/cost, four HP+MK delayed wakeup rows, and ten Red Focus press/release rows. The alternate Ultra rows' raw presence does not establish normal entry or frame gates. |

EX specials cost 250 raw Super, EX Focus costs 500, Jaguar Varied Assault costs 1000. Ultra Double grants both normal Ultra commands at 75% damage for Adon. Capcom's notes put Adon in the 75% class by exclusion from the named 60% group. No Adon-specific live Ultra Double output was captured. [Capcom Ultra Double rules](https://news.capcomusa.com/lets/browse/ultra-street-fighter-4-change-list)

## Movement, normals and special contacts

Intervals are half-open local action ticks. Script rate, in-game freezes and conditional transitions govern elapsed timing. A target animation timer is a decoded reaction field. Actual advantage can also depend on the landing and overlay state. L/M/H order applies when rows provide three values.

| Action | AE2012 latest | Standard Ultra |
|---|---|---|
| Walk | +0.050 forward, −0.035 backward per advancing tick with zero acceleration. 16 forward ticks cover 0.80 native units. | Same. |
| Far standing LP | `[7,10)` with total 30, 30 damage/50 stun, stop 8/8, push 0.30 on hit and block. Special execute `[7,10)`, rapid-light execute `[22,30)`. | Same sampled fields. |
| Close standing MP | `[9,14)` with total 32, 70 damage/100 stun, stop 10/10, hit/guard timers 21/14, push 0.30/0.50. Special and Super execute `[9,14)`. | Same sampled fields. |
| Jaguar Crunch `6MP` | `[29,30)` and `[30,31)`, 30+50 damage, 50+50 stun, stop 10/10 per hit. Second hit grounded reaction timer 12, guard 11. Raw IASA 48, total 56. | Second standing-hit reaction timer 11. Guard remains 11, matching Capcom's reported one-frame loss on standing hit. |
| Throws and Focus | Both throws grab in `[3,5)`. Jaguar Carry damages at `[43,45)` of the capture. Jaguar Slam has a zero-damage marker at `[10,12)` and damage at `[32,34)`. Both damage fields are 130. Focus levels 1/2 strike `[10,12)`, level 3 `[70,72)`, for 60/80/140 damage. | Same sampled grab/strike windows and damage. |
| Ground Jaguar Kick L/M/H | Initial box `[35,36)`, second pose coverage through raw 41/42/43. Standing and airborne damage 140/140/130, chip 35/35/33, stop 12/12, hit/guard push 1.0. X launch 0.130/0.090/0.025. | Ground damage 130/130/120, airborne rows 90/90/100, chip 33/33/30. Heavy late box Y/height 0.15/0.60 → −0.10/0.35. Use these late values for v1.04-based balance. |
| Ground EX Jaguar Kick | Overhead coverage `[35,36)` and `[36,38)` for first effect 103, then `[38,40)` for second effect 104. Damage 80+70, stun 90+110, stop 8/8, hit/guard push 0.70. Launch X/Y at raw 4 is +0.15/+0.20. | Same sampled public command and contacts. The hidden `EX2/EX3` actions encode launch X 0.08/0.025. No inspected normal command or flow references them. |
| Air Jaguar Kick L/M/H | `[28,35/33/31)`, 80 damage/160 stun, stop 12/12, guard timer 13 and push 1.0. X speeds are 0.04/0.07/0.09, Y 0.10, vertical acceleration −0.014. On-land branch starts at raw 27 and the landing action has raw total 21. | Guard timers 13/12/11. The AE2012 and Ultra sampled ordinary hits do not become overhead. |
| Air EX Jaguar Kick | First effect 112 `[27,28)` and `[28,32)`, second effect 113 `[32,34)`. All mid, 90+70 damage, 110+90 stun, stop 8/8. Effect 113 guard reaction 22, push 1.0 and airborne target VY −0.30. | Effect 113 becomes overhead. Earlier coverage stays mid. |
| Jaguar Tooth L/M/H/EX | Opening wall branch `[36,46)`. Initial horizontal velocity −0.20 ordinary, −0.24 EX. In the wall-strike action ordinary contact is `[13,20)`, EX `[9,20)`. Ordinary 120 damage/150 stun and 12/12 stop. EX 120/200, stop 14/14, airborne target velocity +0.10/+0.08. | Same sampled values and wall branch. Wall distance affects elapsed arrival. |
| Rising Jaguar L/M/H | `[5,7)` then `[17,26)`. Damage 50+80 / 90+50 / 50+100. Stun 100+70 / 110+70 / 70+120. Stop 8/8, push 0.8. All three first hits have Super execute `[5,7)`. | Same sampled contacts. First-hit guard timer 20→17 for all three. Light's second hit raises grounded target VY 0.120→0.135. Its sampled already-airborne reaction remains 0.120. |
| EX Rising Jaguar | `[5,9)`, `[17,19)`, `[19,25)`, damage 80+40+40, stun 100+60+60. Stop 14/14 then 8/8 twice. Super execute `[5,7)`. | Same sampled values. |

Capcom's Ultra launch notes and the final standard files target different builds. The launch notes report heavy Jaguar Kick airborne damage 90, while retained v1.04/v1.05 row 102 uses 100. AE2012 patch notes describe the first Rising Jaguar strike's grounded forward knockdown increase. All three first-contact records 117/119/121 change standing and crouching target VX from +0.040 in the AE resource to +0.045 in AE2012. [Capcom AE2012 changes](https://static.capcom.com/streetfighter/downloads/SSFIV%20Arcade%20Edition%202012%20Final%20Change%20List%20-%20US.pdf) · [Capcom Ultra launch changes](https://news.capcomusa.com/combofiend/blog/2014/05/22/usfiv-final-character-change-list)

## Super and Ultras

- **Jaguar Varied Assault.** All three strength scripts buffer the follow-up `[119,129)` and allow execution `[127,129)`. The ordinary route's first four hitbox windows are `[71,74)`, `[83,85)`, `[96,98)`, `[109,111)` in the sampled light script. Each deals 50 damage and stop 4/4. Its default final two strikes are `[129,132)` for 50 and `[171,174)` for 70. The Ultra source adds additional numeric-condition rows alongside the same named follow-ups.
- **Jaguar Assassin.** Kick branches to raw `[1,4)`, `[21,25)`, `[31,34)` and `[42,47)`, with 30/30/30/60 damage, stop 12/12 for the first three and 24/24 for the last. Landing from `[65,70)` routes to `JAG_ASSAULT_GROUND`.
- **Thousand Jaguars.** The first action opens `[2,5)` for 30. Additional punch strikes carry 12 damage and 2/2 stop. The three extension commands each sample six lenient punches with buffers 12, 10 and 8. Prepare/execute gates are `[26,38)/[36,38)`, `[12,22)/[20,22)` and `[14,22)/[20,22)`. Ultra changes repeated contact reaction 155's ground hit push from 0.20 to 0.13.
- **Jaguar Revolver.** Raw opening strikes `[86,89)`, `[119,122)`, and `[152,155)`, with 40 damage per opening group in both extracted files. The last window has distinct grounded/airborne coverage and the cinematic transition `UC01_ATK`. The captured final strike at `[180,182)` changes 90→70 damage. The first leap VY changes 0.22→0.20 at raw 66 in Ultra.
- **Jaguar Avalanche.** Raw `[94,98)` and `[104,109)` deal 40 damage each with stop 8/8. The third window `[132,134)` uses the capture reaction and branches to `UC02_ATTACK`. That action strikes on `[0,2)`, `[73,75)`, `[87,89)`, `[101,103)`, `[115,117)`, `[141,143)` and `[204,206)`, with decoded damage 30, 15 five times and 100. No standalone-versus-Double total follows from summing raw fields without the Revenge scaling and capture logic.

## Port comparison

The port has the same four special families and both Ultra motions. Ground Jaguar Kick currently uses quarter-circle back, while the extracted AE2012 and Ultra command uses `421K`. The current port uses a shared family alias for several moves and a dedicated Jaguar Avalanche stream using SF1 poses. Jaguar Crunch and the two Jaguar Varied Assault follow-up actions have no named entry in the SFIV Adon dispatch. Compare private landing and wall/air transitions via inputs, ordinary and EX height boundaries, strike order and grounded/air/guard reactions. Recheck Ultra Double with both Adon commands and matched one-Ultra damage at equal gauge. Existing port tests exercise the Adon Super and selected Ultra II. The port's 75% general Ultra Double test uses Ryu. [Port commands](../../crates/sf-engine/src/era_fight/sf4.rs#L311) · [Port family aliases](../../crates/sf-engine/src/data_bank/sf4.rs#L38) · [Jaguar Avalanche authored stream](../../crates/sf-engine/src/data_bank/sf4.rs#L234) · [Port SFIV native tests](../../crates/sf-engine/tests/sf4_native.rs#L145)
