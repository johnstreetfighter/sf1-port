# Ken moveset audit for Street Fighter IV and Arcade Edition 2012

Source slice for [#148](https://github.com/johnstreetfighter/sf1-port/issues/148). It compares original 2008 arcade Ken, AE2012 arcade Ken, and selected endpoints from the [standard Ultra audit](ken-usfiv-moveset-audit.md). The paired match is Ken versus Ryu with Classic controls and Era Default gauges. The port baseline is Git `0e336743760689ccdd192cbe4ebce5c4d638c9c4`. Both references use shipped data. Live source and port input comparisons remain open. [Port revision and loadout registry](../../crates/sf-engine/src/era.rs#L452)

## Source identity and extraction

| Source | Build identification and selected members |
|---|---|
| 2008 Taito Type X2 v1.0 STD | Preservation archive `Street Fighter IV (2008).zip`, SHA-1 `1120092ad2dcde6a381a2305dbbb843339585c44`. Original `launcher.exe` MD5 `e47d7c83e526dbb1986173f197801b99`, first 0x400 bytes CRC-32 `ddd19e50`, PE timestamp 2008-06-03 02:15:20 UTC. Extract `Battle/Chara/KEN/KEN.cmn.emz`. |
| 2011 Taito Type X2 AE2012 NESYS 1.3 | Preservation archive `Super Street Fighter IV Arcade Edition Ver. 2012 (1.3) (2011).zip`, SHA-1 `dd2d5cf64e9d35dd1b3da772c9d72b906b860026`. `game.exe` MD5 `15432af2c3ec96b7dec196477b0e3591`, normalized header CRC-32 `545b0d0c`, PE timestamp 2011-12-21 10:10:01 UTC. Use `patch/battle/regulation/latest/KEN/`. |

[Preservation metadata](https://archive.org/metadata/Yori-Arcade-Arcade_Games_-_TAiTO_Type_X2-RomVault) · [2008 executable fingerprint](https://github.com/teknogods/OpenParrot/blob/fc8ba632e1f67c5c340ff88f30efca743e8d38f5/OpenParrot/src/Utility/GameDetect.cpp#L322) · [NESYS normalization and fingerprint](https://github.com/teknogods/OpenParrot/blob/fc8ba632e1f67c5c340ff88f30efca743e8d38f5/OpenParrot/src/Utility/GameDetect.cpp#L449)

| Member | Bytes | SHA-256 |
|---|---:|---|
| Original `KEN.cmn.emz` | 5819025 | `4aabe350ad045b5bc94a13c60597884f02a5d884a28e4051b005d83aa253c534` |
| Original `KEN.bac` | 187296 | `3dd624fb426e0cba08e614b379e26e9860d1929ba3815de7bd32afed5886a00c` |
| Original `KEN.bcm` | 7524 | `39a636b74118ba2e3cc79629b19e9765f661f6299a59bdf6f1d610f09f174887` |
| AE2012 latest `KEN.bac` | 234824 | `3948fe8604bd84ea9eb02318da9c5ba4c155e8b2cab485ae1100bf76e525bbed` |
| AE2012 latest `KEN.bcm` | 9167 | `16066263ff133fa21c64e841135783342ed9b43dc2d281204d46ae02c3d4d4a5` |
| AE2012 archive `v101/KEN.bac` | 216665 | `294a5a3ff99a25e281f7d9054c8653ac75373f8918c6ecb1f5b56d4cf6d505f4` |
| AE2012 archive `resource/.../KEN.bac` | 216705 | `4413541bc678408f636c1d895117750cbd7aa4875f2881d98de2aae81b80282a` |
| AE2012 archive `v101` and `resource` Ken BCM | 8958 each | `f15ad9c9f84749a68cc7912ca3b9a18c9987c56dafa8d663bf4ed14dc6793780` |

The original BAC has 185 populated actions and 114 populated reaction records. Its BCM has 69 command rows, six motion definitions and 19 cancel lists. AE2012 latest has 214 action names, four projectile scripts, 132 populated reaction records, 70 command rows, eight motion definitions and 19 cancel lists. The two older regulation layers in the arcade archive provide internal comparisons. Their labels alone leave the exact original Super SFIV console version open. The PC extraction chain separately documents Super → AE → AE2012. [Pinned Ken PC script deltas](https://github.com/dantarion/sf4tools/tree/e07764262003f3aa1feff1a0a5a66fc3c14efb43/characters/KEN)

## Commands

The first 62 rows have the same command families and order. Directions are facing-relative. Inputs below use the usual player notation, including motion leniency accounted for by the BCM.

| Original / AE2012 rows | Command family |
|---|---|
| 0–14 | Close and far standing LP, LK, MP, MK, HP and HK. Separate rapid close LP, far LP and close LK. The proximity cutoff is 1.2 native units. |
| 15–22 | Six crouching normals plus rapid LP and LK. AE2012 remaps crouching MP, MK and HK to revised scripts. |
| 23–34 | Six neutral and six directional jump normals. |
| 35–39 | Inazuma Kick `4MK`, Forward Step Kick `6MK`, Thunder Kick `6HK` feint/hold start and release, close `MP → HP`. |
| 40–41 | Forward dash `66` and backdash `44`. |
| 42–45 | Hadoken `236LP/MP/HP` and EX `236PP`. |
| 46–49 | Ground Tatsumaki `214LK/MK/HK/KK`. |
| 50–53 | Air Tatsumaki `j.214LK/MK/HK/KK`, all height 0.5. One action per strength. AE2012 uses `AIR_TATSU_EX_NEW` for EX. Ultra later provides four additional early-forward rows. |
| 54–57 | Shoryuken `623LP/MP/HP/PP`. AE2012 selects `SYOURYU_L_NEW` for LP. |
| 58–60 | Shoryureppa `236236LP/MP/HP`. Three distinct strength actions. |
| 61 | Shinryuken (Ultra I), `236236PPP`. |
| — / 62 | Guren Senpukyaku (Ultra II), `236236KKK`. |
| 62–63 / 63–64 | Shoulder Throw `5/6+LP+LK` and Hell Wheel `4+LP+LK`. |
| 64–67 / 65–68 | Focus `MP+MK`, maximum charge, EX Focus and two release stages. |
| 68 / 69 | Neutral `HP+HK` appeal. |

Both encode EX specials at 250 Super units, EX Focus 500, Shoryureppa 1000, and the standard Ultra rows at 200 raw Revenge threshold and 1000 raw cost. Ultra II is inapplicable in original SFIV. Ultra Double, Red Focus and delayed standing arrive in Ultra. [Capcom original SFIV command FAQ](https://news.capcomusa.com/lets/browse/street-fighter-iv-faq) · [Capcom Ultra systems](https://news.capcomusa.com/lets/browse/ultra-street-fighter-4-change-list)

## Movement and contact anchors

The table uses half-open animation tick windows. Damage and stun are meter fields. Freeze is written as attacker/defender and push as the source displacement parameter. Rate commands, hitstun, blockstun, landing and runtime flow may change elapsed times. The source's animating boundary is listed where useful.

| Property | 2008 v1.0 STD | AE2012 latest and later checks |
|---|---|---|
| Walk | `FORWARD/BACKWARD` set +0.040/−0.030 native units per tick, zero acceleration. At 16 ticks forward travels 0.640 native units, 53.33% of the 1.2 cutoff. | Same velocities in all three arcade layers. Ultra standard sets +0.046/−0.0345, 61.33% for forward over 16 ticks. |
| Close/far LP | Close `[9,13)` boundary 26, far `[8,12)` boundary 22. Damage 30/20, 50 stun, 8/8 on hit and guard, push 0.3. Far uses 2× startup then 3× post-strike speed. | Same anchors in AE2012 latest and Ultra. The final Ultra published table lists 4 startup, 2 active and 5 recovery. The archive speed fields and raw offsets remain separate from the port's 60 Hz ticks. |
| Crouching kicks | Crouch LK `[5,9)`, effect 30, 20 damage/50 stun, 8/8 freeze on hit/guard, 0.3 push. Crouch MK `[10,14)`, 70/100, 10/10, 0.5. Sweep `[14,17)`, 110/200, 12/12, 0.7 and sweep reaction 146. | AE2012 MK and sweep retain these windows, with 60/100 and 100/100 respectively. The latest sweep boundary is 41 and its 2× rate command is at raw tick 2. Original boundary 42 and the 2× command is at tick 0. Capcom reports ordinary sweep startup 7→8 entering AE2012. |
| Far HK | `5HKF` `[13,16)`, boundary 56, 130/200, 12/12 freeze and 0.7 push. | AE2012 latest `5HKF_aer` `[12,15)`, boundary 53. Capcom gives 12→11 startup, +1 on hit and −3 on guard. |
| Inazuma and Step Kick | `4MK` overhead `[21,23)`, 80/100, 10/10 freeze, 0.3 push. `6MK` mid `[20,23)`, 60/100, 10/10 freeze, 0.7 push. No cancel entry in either action. | Same sampled windows and contact effects in AE2012 latest and Ultra. Explicit Kara throw input timing remains a live measurement. |
| Thunder Kick | Forward+HK starts `6HK_FEINT`. Source release flow is `[11,13)` into `6HK`; held feint ends at 39 with no strike. Released overhead strikes `[21,23)`, 120/150, 12/12 freeze on both hit and guard, push 0.6. | `6HK_FEINT_aer` keeps the release flow and ends at raw 35. Capcom gives an elapsed feint reduction 27→24. The source standing guard row remains 12/12 in AE2012 and becomes 12/16 in Ultra. |
| Close MP → HP | Close MP `[11,14)` feeds `TC_5HP` over `[1,14)` and `[11,14)` for original flags 259 and 1073741824. TC HP touches `[16,18)` and `[18,20)`, both use reaction 83, 30/50, 12/12 freeze, 0.5 push. | AE2012 latest uses `5MP_NEW`, gate `[11,35)` for conditions 6 and 1. First HP box is lower and taller (Y/height 0.15/0.20→0.05/0.30 relative to both 2008 and archive v101). Reaction 83 becomes 70/50. Its special/Focus/Super cancel condition 1 is `[16,18)` in the dedicated HP action. Far MP has no target combo list in either build. |
| Hadoken | All four spawn `[25,26)`; ordinary boundary 59, EX 56. VFX speeds 0.05/0.06/0.07/0.10 with zero acceleration. Ordinary 60/100, chip 15, stop 8/8 and push 0.8. EX reactions 103/149 give 50+40 damage, 50+50 stun, chip 12+10, stop 8/8 and push 0.5/0.8. | Same spawn/recovery boundaries, speeds and damage in AE2012 latest. Ultra damage becomes 70 and 50+50. Separate the projectile start from its first actual contact. |
| Shoryuken | L `[9,12)`, `[12,20)`; M `[8,11)`, `[11,20)`; H `[8,10)`, `[10,12)`, `[12,20)`. MP raw standing effects 80/80, first grounded. HP 70/40/30. EX `[8,10)`, `[10,12)`, `[12,14)`, `[14,19)`, damage fields 80/60/50/50. The last two use reaction 125. These fields do not establish a landed four-hit total. | Latest AE MP 90/60, first standing reaction remains grounded. HP remains 70/40/30. Latest AE EX uses 80/30/30/60 and the two middle boxes reuse reaction 124. The v101 and resource copies use 70/30/30/50. Capcom confirms this AE→AE2012 change. Ultra changes HP second to 50 and MP first to a launch. |
| Ground/air Tatsumaki | Ground L/M/H/EX get 3/6/8/8 authored attacks, first `[9,11)`. Air L/M/H get 4/5/7 attacks beginning `[19,21)`, EX gets seven beginning `[19,22)`. Original EX air boxes are overhead. | Same sampled group counts in AE2012. The latest `AIR_TATSU_EX_NEW` has mid settings for the seven boxes. The two older arcade layer scripts carry overhead. Ordinary jumping attacks still use the six directional/neutral normal variants. |
| Throws | Both grabs use `[3,5)`, reaction 90, width 0.464 at x 0.35, and separate on-contact actions. Shoulder Throw finisher `[62,64)`, reaction 91, 120 damage. Hell Wheel `[65,67)`, 92, 130. Both give 80 stun. | Same AE2012 raw windows, damage and grab size. Verify actual grab spacing, tech, orientation and floor recovery in both. |
| Focus | First two release levels strike `[17,19)`, effects 60/61, maximum `[77,79)`, reaction 62. The charge branches at `[13,21)` and `[21,59)`. Dash and backdash entries are in the Focus cancel lists. | Same main charge attack windows in AE2012. Both releases and dash paths need controller checks at the meter thresholds. |
| Shoryureppa | Six contact groups `[52,54)`, `[54,56)`, `[56,65)`, `[94,96)`, `[96,99)`, `[99,108)`, action boundary 151. Reaction 126–131, damage fields 50/50/50/50/100/100 across L/M/H. | Latest AE keeps the windows, uses 126–131 for L/M and 166–171 for H. Damage fields 40/40/40/50/100/100. Test which strikes land in the three strength trajectories. |
| Shinryuken and Guren | Shinryuken opens at `[66,68)` and transfers on hit over `[67,68)` to `UC_REPPA_2`. There are ten authored opener/alternate boxes and sixteen in the cinematic branch. Raw opener reaction 132 contains a signed −40 damage field. Its runtime interpretation is open. | AE2012 opener remains `[66,68)` and the same cinematic link, but effect 132 carries +40 and a standing blowback reaction. Guren first attacks `[124,126)`; the `[213,215)` group selects `UC_TATSUMAKI_2`, with cinematic finisher `[84,88)`, reaction 184 and 150 damage field. |

[Capcom AE2012 changes, Ken on page 2](https://static.capcom.com/streetfighter/downloads/SSFIV%20Arcade%20Edition%202012%20Final%20Change%20List%20-%20US.pdf#page=2) · [Capcom Ultra launch changes](https://news.capcomusa.com/combofiend/blog/2014/05/22/usfiv-final-character-change-list) · [Pinned Ultra frame data](https://wiki.supercombo.gg/api.php?action=parse&oldid=356186&format=json)

For original cancel groups, `SUPER_MOVE` contains the three grounded special families, EX specials, Super and EX Focus. `SUPER_COMBO` contains the three Shoryureppa strengths plus EX Focus. The original/AE2012 crouch MK `SUPER_MOVE` execution is `[10,14)`. HP and EX Shoryuken each encode a `[8,12)` `SUPER_COMBO` execution gate, with `[1,12)` in the other condition. These windows overlap several boxes. Exact accepted hit/guard/whiff inputs and interrupted actions need live traces. Capcom explicitly documents the second-hit Focus cancel behavior in Ultra; that behavior alone does not establish the original and AE2012 runtime gates. [Capcom final Ultra changes](https://news.capcomusa.com/combofiend/blog/2014/05/22/usfiv-final-character-change-list)

## Port comparison and reproduction

The port's original Ken card offers Shinryuken and the AE2012 card offers Shinryuken and Guren. Standard grounded special and Super command dispatch matches the listed inputs at that level. The canonical Ken SFIV table has no handlers for the three directional kicks, Thunder Kick feint, close MP target HP or air Tatsumaki. The shared projectile path has one contact by default where both arcade versions carry two EX effects. The Focus path admits an active special without a Ken-specific phase gate. These are verified configuration or dispatch differences. Combat outcomes remain unverified in a paired run. [Port Ken commands](../../crates/sf-engine/src/era_fight/sf4.rs#L293) · [Port projectile accounting](../../crates/sf-engine/src/era_fight.rs#L1592) · [Port Focus admission](../../crates/sf-engine/src/era_fight/sf4.rs#L651)

For 2008, use the authenticated archive member named above. Inflate the raw deflate stream after the EMZ's 16-byte header, read the nested EMB member table and extract the BAC and BCM. In original BAC, the script and name tables are at the pointers stored at 0x0c/0x10; action 16 is `FORWARD`. The original BCM move/name pointers sit at 0x20/0x24 and the 69-row stride is 68 bytes. The move action index is at +0x1c. The original reaction offset table is at the pointer at BAC 0x14, with 12×64-byte reaction entries. The Ryu report specifies the compatible timeline extraction and reaction offsets. For AE2012, load the adjacent regulation members through the pinned UltraDiff decoders and follow `Moves → Script` through `HITBOX`, `CANCELS`, `FLOW`, `SPEED` and `PHYSICS`. Direct validation anchors are original action 300 and group 12, latest `5MP_NEW` with gate end 35, and latest EX DP reaction 125 at 60 damage. [Original arcade extraction layout](ryu-sfiv-arcade-moveset-audit.md#reproduction) · [AE2012 extraction layout](ryu-sfiv-ae2012-moveset-audit.md#reproduction-and-remaining-evidence) · [Pinned UltraDiff decoder](https://github.com/dantarion/sf4tools/tree/e5c17132f3b6e63f66150c3a347a9e8f6e0d73a4/SFUltraDiff/src)

| Paired recipe | Needed result |
|---|---|
| Ken versus Ryu, each revision and permitted Ultra loadout, midscreen and sides swapped | Walk 16 unobstructed ticks, find close/far LP switch, and whiff ordinary LP Hadoken. Record both input and projectile clocks, action boundary, scaling reference and pose. |
| Close MP into HP against neutral, crouch and guard, then at far spacing | Sweep the HP press through both gate edges and record fallback. Repeat `4MK`, `6MK` and `6HK` release and hold. |
| Clock an isolated strike/contact from each LP/MP/HP/EX DP and Tatsu strength | Collect hitstop, hit and guard reactions, juggle and landing timing. Test DP → Focus → dash by each contact, and air Tatsu by jump direction. |
| Both throws, alternate spacing, then Super and Ultras at empty/eligible/full meter | Confirm throw tech and knockdown, landed contact count, cinematic trigger and early/late airborne outcomes. Use a failure control just outside each measured boundary. |

Original Super SFIV console data, authenticated Ken live captures, and the port trajectory and combat checks remain open under #148.
