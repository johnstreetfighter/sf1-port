# Ryu moveset audit for Street Fighter IV Arcade Edition 2012

## Source and port

The source is the Japanese Taito Type X2 Super Street Fighter IV Arcade Edition Ver. 2012 **NESYS 1.3** preservation set, archive SHA-1 `dd2d5cf64e9d35dd1b3da772c9d72b906b860026`. Its `game.exe` has SHA-256 `02710ccbd140d06c49aefb250ad1c0d58eeea16518ac258495b9d40353aca4c4`, MD5 `15432af2c3ec96b7dec196477b0e3591` and PE timestamp 2011-12-21 10:10:01 UTC. The first `0x400` bytes produce CRC-32 `545b0d0c` after the OpenParrot normalization, which identifies NESYS 1.3. The arcade regulation used for this report is `patch/battle/regulation/latest/RYU`. [Preservation metadata](https://archive.org/metadata/Yori-Arcade-Arcade_Games_-_TAiTO_Type_X2-RomVault) · [OpenParrot identification and normalization](https://github.com/teknogods/OpenParrot/blob/fc8ba632e1f67c5c340ff88f30efca743e8d38f5/OpenParrot/src/Utility/GameDetect.cpp#L449)

| Arcade Ryu files | BAC SHA-256 | BCM SHA-256 |
|---|---|---|
| `patch/battle/regulation/latest/RYU` | `3163b03159589db993ff6a023c8237ec762a1b978b013e2b76d6fd5c1fdf874d` | `383dc64834da4adba42914f20de4b7484bf06b949bd5739b0680e49ee7f77a99` |
| `patch/battle/regulation/v101/RYU` | `3e74d0b4a08a7f0de31eba4152e5fd78c9171f81bff632f8460dff0167e423af` | `791deb96fac9d5e51be4d09d471dede7b1a35d9aba57b13a677c1849d3a40309` |
| `resource/battle/chara/RYU` | `adc4ab698b3a3c7264c4579e45e64d2d41980d84a65083411308fd03b608e2bb` | `791deb96fac9d5e51be4d09d471dede7b1a35d9aba57b13a677c1849d3a40309` |

Capcom's final Ver. 2012 notes identify the balance data. Only `latest` matches the published 70 damage for ordinary Hadoken, 40+60 Solar Plexus Strike, the 160/60 early and late heavy Shoryuken, and the enlarged Metsu Shoryuken opening box. Both alternative files in the archive give 60, 40+80, 100/50, and the shorter Ultra box, respectively. The executable's BAC and BCM loader branches select `battle\regulation\latest\` when the regulation field is 4. The alternate branch for value 1 can select `battle\regulation\v101\` subject to a fighter lookup. The balance data and the loader path are verified by file inspection. A live arcade asset-load trace remains open. [Capcom's final US change list, Ryu](https://static.capcom.com/streetfighter/downloads/SSFIV%20Arcade%20Edition%202012%20Final%20Change%20List%20-%20US.pdf)

The port baseline is Git commit `0e336743760689ccdd192cbe4ebce5c4d638c9c4`, with the working-tree #138 pacing regression inspected for its accepted values. The matched setup is local Ryu versus Ken with the **SF4 AE2012** Era Style, Era Default gauges and Classic controls. Ryu has two selectable loadouts, **Ultra 1 Metsu Hadoken** and **Ultra 2 Metsu Shoryuken**. Ultra Double, Red Focus and Ultra's delayed wakeup are inapplicable. Static port checks establish input and configuration. Live contact and route comparisons require the recordings below. [Port loadout rules](../../crates/sf-engine/src/era.rs#L508) · [Pacing regression](../../crates/sf-engine/tests/st_ryu_pacing.rs#L159) · [Audit #154](https://github.com/johnstreetfighter/sf1-port/issues/154)

## Command inventory

The identified BCM has 70 move rows, nine motion definitions and 18 cancel lists. Its BAC has 212 named actions, 10 projectile or effect actions and 110 populated reaction IDs. Directions use numpad notation relative to Ryu facing right.

| BCM rows | Inputs and variants | Arcade dispatch |
|---|---|---|
| 0–13 | Close and far standing LP/MP/HP/LK/MK/HK, plus rapid close and far LP | `5LP…5HK`, `5LPF…5HKF`, `R5LP`, `R5LPF`. Far MP and close MK use `5MPF_NEW` and `5MK_NEW`. |
| 14–21 | `2LP/MP/HP/LK/MK/HK` and rapid `2LP`, `2LK` | `2LP…2HK`, `R2LP`, `R2LK` |
| 22–33 | Neutral jump and directional jump LP/MP/HP/LK/MK/HK | `8LP…8HK`, `9LP…9HK`. Directional jump MP uses the two-window `9MP_NEW`. |
| 34–35 | `6MP` Collarbone Breaker and `6HP` Solar Plexus Strike | `6MP`, `6HP` |
| 36–37 | `66` dash, `44` backdash | `DASH`, `BACKDASH` |
| 38–41 | `236LP/MP/HP` Hadoken, `236PP` EX Shakunetsu Hadoken | `HADOKEN_L/M/H`, `FIRE_HADOKEN_L` |
| 42–45 | `214LK/MK/HK` grounded Tatsumaki, `214KK` EX | `TATSUMAKI_L/M/H`, `TATSUMAKI_EX_NEW` |
| 46–52 | `j.214LK/MK/HK` in standard air states and early forward jump, plus `j.214KK` EX | `AIR_TATSUMAKI_L/M/H`, `A_TATSU_FL_NEW/FM_NEW/FH_NEW`, `AIR_TATSUMAKI_EX` |
| 53–56 | `623LP/MP/HP` Shoryuken and `623PP` EX | `SYOURYU_L_NEW`, `SYORYUKEN_M`, `SYORYUKEN_H_NEW`, `SYORYUKEN_EX` |
| 57–59 | `236236LP/MP/HP` Shinku Hadoken | `SHINKU_HADOLEN_L/M/H` |
| 60–61 | `236236PPP` Metsu Hadoken (U1) and `236236KKK` Metsu Shoryuken (U2) | `METU_HADOUKEN` and `UC2_NEW`, with separate cinematic continuations for U2 |
| 62–63 | `5/6+LP+LK` Seoi Nage, `4+LP+LK` Tomoe Nage | `THROW_F`, `THROW_B` |
| 64–68 | `MP+MK` Focus levels 1–3 and ground/air EX Focus cancel routes | `SAVING2`, `SAVING3`, `SAVING_MAX`, `EX_SAVING` |
| 69 | `HP+HK` Appeal | `APPEAL_01` and alternate appeal scripts |

All six close/far standing button pairs use the 1.2 native-unit axis cutoff. The two air Tatsumaki banks use different forward trajectory data, with the forward-jump action selecting the early bank over raw ticks `[0,15)` and the standard bank over `[15,35)`. Early forward LK uses a horizontal acceleration field of -0.0001 native units/tick² and standard air LK uses +0.001. The height restrictions are 0.5 units for the regular variants and 0.25 for EX. Ordinary EX specials cost 250 Super units, EX Focus costs 500, and Shinku costs 1000. Both Ultras require 200 Revenge units and list a 1000-unit drain field. A live threshold and meter trace remain open.

## Pacing and arcade contact evidence

**Match on the agreed #138 pacing endpoints.** The source `FORWARD` record gives 0.045000001788 native units/tick with zero acceleration. The 1.2-unit cutoff and port 64 px cutoff normalize the movement to 38.4 px/16 ticks, rounded to the 38 px regression. Far LP's raw window is `[8,11)` with its action boundary at raw 22. Light Hadoken's raw spawn is `[25,26)`, and its action boundary is at raw 58. Animation-rate commands translate these scripts to the frame-table convention. The port regression expects first far LP active at offset 3, three active ticks and first retreat at 12, and 33 ticks from first live LP Hadoken to first retreat. Those expectations cover both Ultra loadouts. [Source endpoint conventions](2026-11-13-ryu-era-pacing.md#full-revision-comparison-against-v153) · [Port regression](../../crates/sf-engine/tests/st_ryu_pacing.rs#L159)

The windows below are half-open raw script intervals. Stun is stun-meter damage. Freeze is encoded attacker/defender hitstop. Push is the source reaction displacement parameter in native units. A matched elapsed hitstun, blockstun or position trace remains open.

| Move | Raw contact windows and IDs | Standing damage / stun / chip | Freeze and push parameter |
|---|---|---|---|
| Far LP | `[8,11)` 12, mid | 30 / 50 / 0 | 8/8 hit and guard, 0.3 |
| Crouch LK | `[5,8)` 30, low | 20 / 50 / 0 | 8/8 hit and guard, 0.3 |
| Crouch MK | `[10,15)` 32, low | 60 / 100 / 0 | 10/10 hit and guard, 0.5, grounded generic medium hit |
| Sweep | `[12,14)` 34, low | 90 / 100 / 0 | 12/12 hit and guard, 0.7, hit script `DAMAGE_ASHIBARAI` |
| Close HK | `[14,18)` 10 and `[24,28)` 11, mid | 40+70 / 125+75 / 0 | 12/12, hit push 0.3 each, guard push 0.3 then 0.7 |
| Collarbone Breaker | `[19,21)` 80 and `[22,24)` 81, overhead | 30+50 / 50+50 / 0 | 10/10, 0.5 each, standing hit script `DAMAGE_LOWER` |
| Solar Plexus Strike | `[19,21)` 82 and `[21,23)` 84, mid | 40+60 / 50+50 / 0 | 12/12 then 10/12, 0.2 each, standing hit script `DAMAGE_MH` |
| Forward/back throw | Grab `[3,5)` 90. Finish `[62,64)` 91 forward, `[45,47)` 92 back. | 130 each / 140 forward, 120 back / n/a | Finish 0/0, bound script `BOUND_UPWARD_H` |
| Hadoken LP/MP/HP | Spawn `[25,26)`, VFX IDs 100/101/102 | 70 / 100 / 15 each | 8/8, 0.8, grounded medium hit |
| EX Fire Hadoken | Spawn `[25,26)`, VFX IDs 103+104 | 50+50 / 50+50 / 12+12 | 8/8, 0.5 then 0.8, blowback rows |
| Shoryuken LP/MP/HP/EX | Early `[10,12)` and late `[12,24)`, pairs 117/118, 119/120, 125/126 and 123/124 | Early 100/80/160/80, late 70/50/60/60. HP stun 200/50, HP chip 40/15. | 14/14 early and 12/12 late on hit and guard, 0.8, blowback rows |
| Ground Tatsumaki LK/MK/HK | First `[19,21)`, IDs 105/106/107. LK has two raw windows, MK and HK five. | 100/110/120 / 200 / 25/27/30 | 14/14 hit and guard, 0.8, blowback rows |
| Air Tatsumaki LK/MK/HK | `[19,21)`, `[27,29)`, `[35,37)`, IDs 108/109/110 in both air banks | 70/80/90 / 100 / 17/20/22 | 14/14 hit and guard, 0.8, blowback rows |
| Ground EX Tatsumaki | `[19,21)`, `[27,29)`, `[35,37)`, `[43,45)`, `[51,53)`, IDs 65 × 4 and 113 | 30 × 4 + 40 / 40 each / 7 × 4 + 10 | Hit 5/1, guard 5/5. Hit push -1.5 first four, 1.0 final. |
| Air EX Tatsumaki | Same five raw windows, IDs 114 × 4 and 116 | 40 each / 50 each / 10 each | Hit 3/1, guard 5/5. Hit push -1.0 first four, 1.0 final. |
| Focus levels 1, 2, 3 | `[14,16)` 60/61, `[74,76)` 62 | 60/100/0, 80/150/0, 140/200/0 | Hit 16/16, 30/30, 30/30. L2 and L3 standing/crouching hit scripts crumple. |
| Shinku Hadoken | Spawn `[54,55)`, VFX IDs 128 and 129 | 50 × 4 + 100 / 0 / 12 × 4 + 25 | 8/8 each. Hit push 0.5, final 0.8. |
| Metsu Hadoken, U1 | Spawn `[166,167)`, VFX IDs 130 and 131 | 28 × 7 + 50 / 0 / 7 × 7 + 12 base fields | 8/8 each. Hit push 0.3, final 0.8. Revenge scaling and actual contacts remain open. |
| Metsu Shoryuken, U2 | Opener `[84,86)` 134, later `[86,98)` 140 and `[98,107)` 141. Separate cinematic attack has IDs 136, 135, 137 and a late 0. | Opener 0, late fields 25 and 33. Cinematic principal records 180, 30, 125. | Opener 0/0 hit and 30/30 guard, late 4/4 hit and 14/14 guard. Raw opener height rises from 0.75 to 1.0 at 85. |

The four Shoryuken variants encode full hit/throw/projectile protection over raw `[0,10)` LP, `[0,13)` MP, `[0,12)` HP and `[0,24)` EX. The ordinary strengths have additional partial-body or throw records. The heavy variant has an empty cancel group. Capcom specifies one live heavy DP hit with invulnerability through the fourth game frame and no mid-air follow-up or Super/EX Focus cancel. Its early and late reaction windows are alternatives. Other raw contact windows can also share reaction records or have conditional/juggle prerequisites. The U2 cinematic transition, distinct air and late contacts, and Revenge scaling need live traces before summing a result. [Capcom's final US change list, Ryu](https://static.capcom.com/streetfighter/downloads/SSFIV%20Arcade%20Edition%202012%20Final%20Change%20List%20-%20US.pdf)

For the other normal strengths, the source's raw windows and standing damage are below. Every jump normal is marked overhead. Standing and crouching normals are mid except the low crouching kicks. A slash separates consecutive raw windows or distinct contact records.

| Buttons | Close standing | Far standing | Crouching | Neutral jump | Directional jump |
|---|---|---|---|---|---|
| LP | `[9,12)` 30 | `[8,11)` 30 | `[3,5)` 30 | `[21,28)` 50 | `[19,26)` 50 |
| MP | `[3,6)` 70 | `[5,9)` 80 | `[3,7)` 60 | `[23,27)` 80 | `[21,24)/[24,28)` 50/30 |
| HP | `[15,16)/[16,20)` 100/80 | `[23,26)` 120 | `[12,14)/[14,16)` 90 | `[25,29)` 100 | `[23,27)` 100 |
| LK | `[10,15)` 30 | `[8,11)` 40 | `[5,8)` 20 | `[12,21)` 40 | `[16,24)` 40 |
| MK | `[12,15)` 80 | `[16,18)` 80 | `[10,15)` 60 | `[14,24)` 80 | `[19,25)` 70 |
| HK | `[14,18)/[24,28)` 40/70 | `[16,20)` 110 | `[12,14)` 90 | `[18,21)` 100 | `[21,34)` 100 |

## Arcade cancel routes

- `5LPF`, `2LK` and `2MK` reference `SUPER MOVE` through raw 11, 8 and 15 respectively. This list includes the normal and EX ground specials, Shinku and EX Focus. Rapid LP/LK have their own lists. Far LP's rapid windows are `[11,22)` and `[15,22)`. Crouch LK's are `[8,25)` and `[16,25)`.
- `6HP` references the same list at `[33,49)` and `[46,49)`. `6MP` and `2HK` have no cancel group.
- Ordinary and EX Hadoken expose `SUPER COMBO` at `[14,30)` and `[28,30)`. LP and MP Shoryuken reference it through raw 12, EX through 13. LP also has `SAVING_AIR`, which resolves to the air EX Focus row. The grounded EX Focus route and Shinku are in `SUPER COMBO`.
- `SAVING` and `SAVING_CANCEL` list forward and backdash. The BCM contains conditions for ground and air inputs. The accepted command ticks on hit, block or whiff need live trials.

## Port comparison

| Property | Evidence and status |
|---|---|
| Ground specials, EX inputs, Shinku and both selected Ultra commands | **Match for the mapped inputs and loadouts.** Canonical port Ryu uses `236P/PP`, `623P/PP`, `214K/KK`, `236236P`, and the Ultra selection routes `236236PPP` or `236236KKK`. Shared native tests exercise the Ultra commands under USFIV. Their AE2012 contact and guard behavior is **unverified**. |
| Collarbone Breaker and Solar Plexus Strike | **Verified difference in dispatch.** Canonical port SFIV Ryu has no `6MP` or `6HP` route. The existing Alpha and Super Turbo command-normal branches are scoped to those styles. |
| Air Tatsumaki, early forward jump variants and EX | **Verified difference in dispatch.** The canonical port SFIV Ryu array has no airborne entries. The port's SFII air-special branch is scoped to SFII and the SFIV-family air entry belongs to Evil Ryu. |
| Classic throws | **Verified difference in dispatch.** Arcade uses the light-pair chord. The Classic canonical port uses the shared directional heavy-button throw path. Throw timing, range, tech and wakeup are **unverified**. |
| EX Hadoken and U1 contact configuration | **Verified difference in configured limit.** Arcade EX Fire has two scripted contacts and U1 has eight. Port slots `0x1a` and `0x2a` receive the one-contact limit. The port's Shinku slot `0x29` shares the five-contact limit of the arcade data. The arcade U1 damage and reactions, port live contact counts and scaling remain **unverified**. |
| U2 cinematic, late hit and juggle branches | **Unverified.** Port U2 maps to the native bank stream at slot `0x2a` via the SFIV alias at `0x2b`. The arcade's distinct opener, follow-up contact sequence, guard and geometry are identified above. |
| Focus and dash | **Match for available basic inputs.** Port has Focus, dash and metered cancel handlers. **Verified difference in the configured HP Shoryuken Focus gate.** Arcade HP Shoryuken has no cancel group, and the port's shared Focus handler accepts the chord during an active native special, including Shoryuken at slot `0x19`, without an AE2012 strength condition. Air Focus, exact armor, crumple, startup and accepted cancel ticks remain **unverified**. |
| Other normals, special strengths, appeal and movement outside #138 | **Unverified** for matched behavior. The arcade inventory and extracted contact records give the expected moves and source-side anchors. |

[Port command table](../../crates/sf-engine/src/era_fight/sf4.rs#L274) · [Port move bank](../../crates/sf-engine/src/data_bank/sf4.rs) · [Port projectile limits](../../crates/sf-engine/src/era_fight.rs#L956) · [Port throw dispatch](../../crates/sf-engine/src/fighter.rs#L8737) · [Port shared SFIV tests](../../crates/sf-engine/tests/sf4_native.rs) · [2008 SFIV audit](ryu-sfiv-arcade-moveset-audit.md)

The existing release test binary `st_ryu_pacing-7d71ddcb63221474.exe` passed `ryu_matches_the_other_source_revision_pacing_targets` with local ROM assets from the `crates/sf-engine` working directory. The existing release test binary `sf4_native-f84c3b7aca3f0778.exe` passed `ultra_double_offers_both_ryu_ultras_and_spends_revenge` from the same directory. The second test uses USFIV Ultra Double. No new port build was used for these two regression rechecks.

## Reproduction and remaining evidence

Extract the exact member paths above. To reproduce the executable fingerprint, read the PE pointer at file offset `0x3c`, copy the first `0x400` bytes, zero eight bytes at PE `+0x18` and four bytes at PE `+54` (decimal) and compute CRC-32. The ASCII latest path is at file offset `0x405990`, PE virtual address `0x806d90`. In the x86 loader, `0x6c8993` and `0x6c8b37` compare the regulation value to 4. Their path arguments at `0x6c89c8` and `0x6c8b6c` point to that string and the adjacent `.bac` and `.bcm` path formats. The v101 path is at `0x806db8`, referenced by `0x6c8a2f` and `0x6c8bd3`. This is static executable evidence. [OpenParrot normalization](https://github.com/teknogods/OpenParrot/blob/fc8ba632e1f67c5c340ff88f30efca743e8d38f5/OpenParrot/src/Utility/GameDetect.cpp#L449)

Use `BACFile` and `BCMFile` from the pinned UltraDiff Python decoder, with the exact `RYU.bac` and `RYU.bcm` files adjacent. Enumerate `BCMFile["Moves"]`, resolve `Script` into `BACFile["Scripts"]`, and read the `HITBOX`, `CANCELS`, `PHYSICS`, `SPEED` and `INVINC` groups. For projectile actions follow the `ETC` event with `ControlType = "VFX_SCRIPT"` into the separate `VFXScripts` table. The `HitboxData` value selects a reaction record. The standing, crouching and air hit rows are 0–2, and guard rows are 3–5. Cross-check `6HP` on reaction IDs 82 and 84 and heavy Shoryuken on 125 and 126 before using the rest. This regulation's BAC/BCM container structure is decoded by the later parser. [Pinned UltraDiff decoder](https://github.com/dantarion/sf4tools/tree/e5c17132f3b6e63f66150c3a347a9e8f6e0d73a4/SFUltraDiff/src)

Run both Ultra loadouts versus Ken in the pinned arcade balance and in port AE2012. Restore neutral between attempts. Test midscreen, corner and opposite facing. For timing, log input, action start, contact, per-side freeze, first advancing tick, first accepted defensive or movement input, damage, meter and both axes. Use the 1.2 native-unit and 64 port-pixel boundaries for close/far normal setups.

| Recipe | Remaining live observation |
|---|---|
| `5LP`, `2LK`, `2MK`, `2HK`, `6MP`, `6HP` against standing/crouching Ken, both guard heights and airborne Ken | Dispatch, hitbox reach, hit and guard freeze, hitstun/blockstun, push, sweep wakeup, overhead and follow-up window. Repeat at the close/far cutoff and both sides. |
| `2LK→2LK`, `2MK→236LP`, `6HP→623MP`, `623LP/MP/HP→MP+MK→66` on hit, block and whiff | Sweep the final input tick. Record chain, link and true cancel, HP Shoryuken restriction, air Focus route and meter consumption. |
| `j.214K/KK` on neutral, backward, and early/late forward jump, then a single directional `j.MP` press | Variant selection, apex and landing, both scripted MP contacts, motion height threshold, air juggle and displacement. |
| `236P/PP` against neutral and guarding Ken, then one opposing projectile | Spawn and recovery, clash, damage/chip, launch, range and hit count. |
| `623HP` point blank and late anti-air, plus EX and MP trials | Early/late damage, launch, invulnerability, landing, FADC and first actionable tick. |
| `236236P` at full Super, then U1 `236236PPP` and U2 `236236KKK` at first legal and full Revenge | Contact counts, meter and damage scaling, flash and recovery. For U2 compare grounded close cinematic, tip contact, an airborne Ken and a juggle after DP FADC. |
| Throws both directions, `HP+HK`, dashes, and three `MP+MK` charge levels | Throw input and tech, appeal, dash movement, Focus armor, crumple and guard behavior. |

The current evidence is a data and code comparison. Record live timing, displacement and same-input route outcomes under the direct evidence issue as the captures become available. [Direct SFIV evidence #148](https://github.com/johnstreetfighter/sf1-port/issues/148)
