# Ryu moveset audit for 2008 Street Fighter IV

## Scope and evidence

The source is the Taito Type X2 **v1.0 STD** build in the `Street Fighter IV (2008).zip` preservation set. The `launcher.exe` MD5 is `e47d7c83e526dbb1986173f197801b99`. The CRC-32 of its first `0x400` bytes is `ddd19e50`, the original SFIV fingerprint in OpenParrot. Its PE timestamp is 2008-06-03 02:15:20 UTC. The extracted Ryu BAC SHA-256 is `c1364dd7be172188cb5b04dcad10563c2877d1ffbdcb7d29d416b5c8c88710c2` and the BCM SHA-256 is `4c0263e97948f22359b3f5af672963ff32909d7ff984dd54bf9e4502cd10fa4d`. The 2009 PC benchmark v1.0.0.1 supplies the existing walk cross-check. The moveset values below come from the arcade files. [Preservation metadata](https://archive.org/metadata/Yori-Arcade-Arcade_Games_-_TAiTO_Type_X2-RomVault) · [OpenParrot detection source](https://github.com/teknogods/OpenParrot/blob/fc8ba632e1f67c5c340ff88f30efca743e8d38f5/OpenParrot/src/Utility/GameDetect.cpp#L322) · [Arcade extraction and benchmark comparison](2026-11-13-ryu-era-pacing.md#2008-arcade-versus-the-pc-benchmark)

The port baseline is Git commit `0e336743760689ccdd192cbe4ebce5c4d638c9c4` with the working-tree #138 pacing changes inspected for the already accepted checks. The tested port setup from #138 is local Ryu versus Ken, both in the SFIV ruleset with Era Default gauges. Ryu's only selectable loadout in `Revision::Sf4` is Ultra 1, Metsu Hadoken. Ultra 2 and Ultra Double are inapplicable. Static port comparisons below identify the command dispatch and bank definitions. The source data contain no opposing-character state, and a live arcade Ryu versus Ken run remains open. [Port loadout test](../../crates/sf-engine/tests/sf4_era.rs) · [Port pacing test](../../crates/sf-engine/tests/st_ryu_pacing.rs) · [Audit issue #147](https://github.com/johnstreetfighter/sf1-port/issues/147)

The evidence covers the **2008 SFIV** style and its one loadout. SSFIV, AE2012 and USFIV have their own moves and balance data and remain separate audit subjects.

## Arcade command inventory

The BCM contains 65 named move rows and 15 cancel lists. The BAC contains 189 populated actions across 666 slots and 99 populated reaction entries across 132 slots. The table accounts for every BCM move row. Numpad directions are relative to Ryu facing right, with `5` neutral. The Capcom-hosted FAQ by Torrence Lang corroborates the arcade command names and inputs. [Arcade command reference](https://news.capcomusa.com/lets/browse/street-fighter-iv-faq)

| BCM rows | Inputs and variants | Arcade BAC actions |
|---|---|---|
| 0–13 | Close and far `5LP/MP/HP/LK/MK/HK` plus rapid close and far LP | 256–261 close, 262–267 far, 300–301 rapid LP |
| 14–21 | `2LP/MP/HP/LK/MK/HK` plus rapid `2LP` and `2LK` | 268–273, 302–303 rapid |
| 22–33 | Neutral jump `8+P/K` and directional jump `7/9+P/K`, all six buttons | 274–279 neutral, 280–285 directional |
| 34–35 | Collarbone Breaker `6MP` and Solar Plexus Strike `6HP` | 320–321 |
| 36–37 | Forward dash `66`, back dash `44` | 18–19 |
| 38–41 | Hadoken `236LP/MP/HP` and EX Shakunetsu Hadoken `236PP` | 384–387, projectiles 512–515 |
| 42–45 | Ground Tatsumaki `214LK/MK/HK/KK` | 390–392 and 404 |
| 46–49 | Air Tatsumaki `j.214LK/MK/HK/KK` | 393–395 and 405 |
| 50–53 | Shoryuken `623LP/MP/HP/PP` | 396–399, landings 400–403 |
| 54–56 | Shinku Hadoken `236236LP/MP/HP` | 420–422, projectiles 518–520 |
| 57 | Metsu Hadoken `236236PPP` | 436, projectile 521 |
| 58–59 | Seoi Nage `5/6+LP+LK`, Tomoe Nage `4+LP+LK`, both close | 288–289, attack continuations 292 and 294 |
| 60–63 | Focus Attack `MP+MK`, three charge levels and the metered special cancel | 322 full charge, 323 level 1, 324 level 2, 325 EX cancel |
| 64 | Appeal `HP+HK` | 333 and alternate appeal scripts |

Standing normal proximity uses the same 1.2 native-unit axis separation in all six button pairs. The close and far records have comparison modes 2 and 1 respectively. Movement, crouch, jump, landing and guard also have common BAC actions. The two additional `FIRE_HADOKEN` actors at 388–389 point to projectile actors 516–517 and share the same two reaction entries as 387. The BCM exposes a single `236PP` EX Fire row. Its required and consumed Super meter are each 250 internal units. The metered Focus cancel uses 500 and Shinku Hadoken uses 1000. Metsu uses the separate Revenge gauge.

## Pacing comparisons

| Property | Arcade evidence | Port evidence and status |
|---|---|---|
| Forward walk | BAC 16 sets 0.045 native units per tick, 0.72 over 16 ticks. The PC benchmark has the same action bytes and 1.2-unit close/far cutoff. | **Match on the agreed proximity scale.** The port cutoff is 64 px, yielding a 38.4 px calibrated target and a 38 px/16-tick regression. Standing pushbox width and live movement remain unverified. |
| Far LP pacing | Arcade guide lists 4 startup, 3 active, 6 recovery in its inclusive convention. BAC 262 has a raw strike window `[8,11)` and writes animation-rate changes at offsets 0, 6, 11 and 19. | **Match on the agreed timing endpoints.** The port test checks first active at offset 3, 3 active ticks and first retreat at offset 12. Contact properties remain open. |
| Light Hadoken whiff recovery | Arcade guide lists 13 execution and 45 duration. BAC 384 creates projectile 512 at raw offset 25, with raw action-boundary field 58 and rate changes at offsets 0 and 23. | **Match on the agreed recovery endpoint.** The port test checks 33 elapsed ticks from the first live projectile to first retreat. The projectile and contact fields below need matched port traces. |

The BAC windows use the animation script clock and half-open intervals. The pacing test uses port simulation ticks. The far LP window and the Hadoken spawn offset above preserve both sets of anchors. [Pacing source and port conventions](2026-11-13-ryu-era-pacing.md) · [Port pacing test](../../crates/sf-engine/tests/st_ryu_pacing.rs#L158)

## Arcade contact records

Each line gives raw attacker script windows and the reaction IDs attached to the contact boxes. Damage, stun-meter damage, chip and hitstop are fields from the ordinary standing hit and guard rows. Hitstop is attacker/defender. The BAC also holds a reaction duration field and a push-distance parameter. For example, the `+0x1e` duration field and `+0x2c` float are 8 and 0.3 for far LP, 12 and 0.5 for crouching MK, and 18 and 0.8 for ordinary Hadoken. Elapsed hitstun, blockstun, displacement and first actionable tick require live verification.

| Move | Raw window and reaction IDs | Damage / stun / chip | Hitstop hit; guard | Source reaction and guard level |
|---|---|---|---|---|
| Close LP, far LP, crouching LP | `[9,12)` 0, `[8,11)` 12, `[3,5)` 24 | 30 / 50 / 0 each | 8/8; 8/8 | Mid, generic light standing/crouching reaction |
| Crouching LK | `[5,8)` 30 | 20 / 50 / 0 | 8/8; 8/8 | Low, generic light standing/crouching reaction |
| Crouching MK | `[10,15)` 32 | 70 / 100 / 0 | 10/10; 10/10 | Low, generic medium standing/crouching reaction |
| Sweep | `[12,14)` 34 | 90 / 100 / 0 | 12/12; 12/12 | Low, explicit sweep reaction script 146 |
| Collarbone Breaker | `[19,21)` 80, `[22,24)` 81 | 30+50 / 50+50 / 0 | 10/10; 10/10 each | Two overhead boxes. Standing hit selects script 145, crouching hit selects generic medium reaction. |
| Solar Plexus Strike | `[19,21)` 82, `[21,23)` 83 | 40+80 / 50+50 / 0 | 12/12; 12/12, then 10/12; 10/12 | Two mid boxes. Standing hit selects script 133, crouching hit selects generic heavy reaction. |
| Forward and back throw | Grab `[3,5)` 90, finish `[62,64)` 91 forward and `[45,47)` 92 back | 130 each / 140 forward, 120 back / n/a | Finish 0/0 | Grab box invokes separate throw and victim scripts. |
| Focus levels 1, 2, 3 | `[14,16)` 60/61, `[74,76)` 62 | 60/100/0, 80/150/0, 140/200/0 | 16/16; 12/16, 30/30; 24/30, 30/30; guard row 22/30 | Level 1 generic medium on ordinary grounded hit. Levels 2 and 3 point to standing/crouching crumple scripts 147/148. Full-charge guardability depends on the system state. |
| Hadoken LP/MP/HP | Spawn `[25,26)`, projectile reactions 100/101/102 | 70 / 100 / 17 each | 8/8; 8/8 | One contact per shot. Generic grounded hit, separate air reaction. |
| EX Fire Hadoken | Spawn `[25,26)`, projectile reactions 103 and 104 | 50+50 / 50+50 / 12+12 | 8/8; 8/8 each | Two contact records. Both use airborne-type hit effect even for grounded targets. |
| Ground Tatsumaki LK/MK/HK | First `[19,21)`, IDs 105/106/107 | 100/110/120 / 200 / 25/27/30 | 14/14; 14/14 | Mid, airborne-type hit effect. Later windows repeat the same reaction ID. |
| Air Tatsumaki LK/MK/HK | First `[19,21)`, IDs 108/109/110 | 70/80/90 / 100 / 17/20/22 | 14/14; 14/14 | Mid, airborne-type hit effect. Start height and landing determine available windows. |
| Ground/air EX Tatsumaki | `[19,21)`, `[27,29)`, `[35,37)`, `[43,45)`, `[51,53)`, IDs 111/113 ground and 114/116 air | 40 / 50 / 10 per scripted contact | 5/1 ground, 3/1 air; 5/5 | Final window switches to reaction selector 200. |
| Shoryuken LP/MP/HP/EX | `[10,12)` first, `[12,24)` late, ID pairs 117–124 | Early 130/140/160/80, late 70/70/70/60 | 14/14 early, 12/12 late on hit or guard | All use airborne-type hit effect. EX early/late records can be tested separately. |
| Shinku Hadoken | Spawn `[54,55)`, projectile 518/519/520, IDs 128 and 129 | 50 × 4 + 100 / 0 / 12 × 4 + 25 | 8/8; 8/8 each | Scripted repeat counts 4+1, with an airborne-type finish. |
| Metsu Hadoken | Spawn `[166,167)`, projectile 521, IDs 130 and 131 | Base fields 30 × 7 + 55 / 0 / 7 × 7 + 13 | 8/8; 8/8 each | Scripted repeat counts 7+1, with an airborne-type finish. Live Revenge scaling remains unverified. |

These are per-record values. Repeated windows can refer to the same contact, and live collision, scaling, juggle eligibility and range determine the resulting sequence. The EX Focus cancel script 325 carries the same full-charge contact record as 322. Its dash-cancel branches are separate from its attack.

The BCM `SUPER MOVE` cancel list 6 contains regular and EX Hadoken, regular Tatsumaki, regular Shoryuken, Shinku and EX Focus. Far LP, crouching LP, crouching LK and crouching MK each reference this list around their active windows. Solar Plexus references it later at `[33,49)`. Collarbone and sweep have no cancel group in their actions. Hadoken and Shoryuken reference list 7, which contains Shinku and EX Focus. Focus lists 8 and 9 both resolve to forward and back dash. The raw conditions and windows identify candidate routes. Which frames accept a buffered input on hit, block or whiff remains open.

## Port comparisons beyond pacing

| Property | Status and evidence |
|---|---|
| Ground Hadoken, Shoryuken, Tatsumaki and EX inputs | **Match for mapped commands.** The port's canonical SFIV Ryu table maps `236P/PP`, `623P/PP` and `214K/KK`. Strength-specific contact, movement, recovery, chip and reactions remain **unverified**. |
| Air Tatsumaki, including EX | **Verified difference in dispatch.** Arcade BCM exposes four airborne rows. The canonical port SFIV Ryu command array has no airborne entry. The air Tatsumaki entry in that file belongs to the Evil Ryu array, and the shared SFII air-special route is gated to SFII styles. |
| Collarbone Breaker and Solar Plexus Strike | **Verified difference in dispatch.** The arcade has `6MP` and `6HP` actions. Port command-normal paths for the named attacks are Alpha or New Ryu Super Turbo. The canonical SFIV Ryu table has no forward-normal entries. |
| EX Fire Hadoken | **Match for the EX input, verified difference in projectile limit.** The arcade command spends one stock on `236PP` and its Fire projectile has two scripted contacts. Port `236PP` selects EX Hadoken at slot `0x1a` with 120% projectile scaling and a one-contact limit. Live contact, launch, movement and recovery remain **unverified**. |
| Shinku and Metsu | **Match for mapped commands and loadout.** Port has `236236P` Super and `236236PPP` Ultra 1. The native port test exercises original-SFIV Ryu starting Shinku at full Super. The Ryu data bank binds both to the same bank stream. Port slot `0x29` (Shinku) gets the five-contact Super limit, and slot `0x2a` (Metsu) gets the one-contact default. Metsu's eight scripted source contacts are a **verified difference in configured limit**. Source and port live hit counts, damage, spacing and freeze need matched traces. |
| Focus and dash | **Match for available inputs.** Native Focus and dash handlers exist. The original arcade Focus levels, armor timing, crumple, contact and cancel costs need matched traces. Existing deeper Focus tests exercise the shared system on USFIV. Red Focus is **inapplicable** to original SFIV. |
| Ground throws | **Verified difference in Classic input dispatch.** Arcade uses the light-pair chord. Port canonical Ryu's shared path reads directional HP/HK, with a separate control-scheme throw macro. The light-pair branch for SFIV Evil Ryu is distinct. Throw timing, range, tech and wakeup remain **unverified**. |
| Other normals, repeat chains, appeal and wakeup | **Unverified** for matched SFIV behavior. The arcade command inventory and selected reaction records above provide exact anchors. |

[Port command table and dispatch](../../crates/sf-engine/src/era_fight/sf4.rs) · [Port SFII air Tatsumaki scope](../../crates/sf-engine/src/era_fight/sf3.rs#L290) · [Port Alpha command normals](../../crates/sf-engine/src/era_fight.rs#L72) · [Port ST command normals](../../crates/sf-engine/src/fighter/st_ryu_normals.rs) · [Port SFIV move bank](../../crates/sf-engine/src/data_bank/sf4.rs) · [Port projectile limits](../../crates/sf-engine/src/era_fight.rs#L956) · [Port ground throw dispatch](../../crates/sf-engine/src/fighter.rs#L8737) · [Native SFIV-family regression tests](../../crates/sf-engine/tests/sf4_native.rs)

## Reproduction

Extract `Battle/Chara/RYU/RYU.cmn.emz` as in the pacing recipe. Read the extracted BAC and BCM little-endian. For BAC, the `u16` counts at `0x08` and `0x0a` give action and reaction slots. Absolute `u32` pointers at `0x0c`, `0x10` and `0x14` address the action, name and reaction pointer tables. Use the action's `u16` group count at `+0x16` and relative group table at `+0x18`. Group descriptors are `<HHII>` for group type, record count, timeline offset and payload offset. Resolve timeline and payload relative to the descriptor. Each timeline item is `<HH>` for start and exclusive end. Type 4 has a 4-byte animation-rate float, type 5 a 32-byte physics payload, type 6 an 8-byte condition/list pair, type 7 a 44-byte hitbox and type 10 a 32-byte event. In a type-7 payload, the bytes at `+0x1a`, `+0x1b`, `+0x20` and the `u32` at `+0x28` give box type, guard level, contact count and reaction ID. A type-10 `<HH>` event prefix of `(0,2)` points to a projectile child action via its `u32` at `+4`. [Pacing extraction recipe](2026-11-13-ryu-era-pacing.md#2008-arcade-versus-the-pc-benchmark)

The arcade reaction entry has twelve 64-byte variants. Rows 0–2 are ordinary standing, crouching and airborne hit. Rows 3–5 are the corresponding guard variants. In each record, use `i16` values at `+0x00`, `+0x02`, `+0x04`, `+0x06`, `+0x08` and `+0x0c` for damage, stun-meter damage, effect, reaction selector, attacker hitstop and defender hitstop. A guard row's damage field gives chip. A useful cross-check is sweep ID 34, whose hit row contains `90, 100, 1, 146` and points to the named BAC action `DAMAGE_ASHIBARAI`. Later parser source gives the field vocabulary. The offsets and record sizes above come from the original arcade files. [Later BAC tooling](https://github.com/dantarion/ssf4ae-tools)

For BCM, the `u16` counts at `0x0c` and `0x0e` give move and cancel-list slots. The `u32` pointers at `0x20`, `0x24`, `0x28` and `0x2c` give the move table, move-name pointer table, cancel-list descriptors and cancel-list name pointers. Use the `0x44` move stride and the action ID at `+0x1c`. Each 8-byte cancel-list descriptor contains a `u16` item count and a `u32` relative list pointer at `+4`. List items are BCM move indices stored as `u16`.

## Live evidence remaining

Run arcade v1.0 STD Ryu versus Ken in a local match and port SFIV Ryu Ultra 1 versus Ken with Era Default gauges. Record each build and control mapping. Reset the same neutral state between attempts. Use corrected axis separation, test midscreen first, then repeat against a cornered Ken and with sides exchanged. Use the arcade's 1.2-unit jab cutoff and the port's 64 px cutoff for close/far setup. Hold guard before the attack when measuring block. Record attacker action, Ken action, first contact and damage write, each side's first advancing sample after freeze, attacker and defender first accepted reversal, and both positions until release. [Direct arcade evidence #148](https://github.com/johnstreetfighter/sf1-port/issues/148)

| Recipe | Field to measure |
|---|---|
| At close and far, `5LP`, `2LP`, `2LK`, `2MK`, `2HK`, then the remaining normal strengths on stand, crouch, guard and jump | Contact range and height, hit and block freeze, hitstun and blockstun, push, sweep wakeup and air reactions. |
| `2LP→2LP`, `2LK→2LK`, `2LK/2MK→236LP`, `6HP→623P` | Sweep the final button one tick at a time. Check linked or chained opener, first and last accepted cancel, whether the follow-up combos and whether it also works on guard or whiff. |
| At close, `6MP` and `6HP` against standing guard, crouching guard and neutral Ken | Dispatch, number and spacing of contacts, overhead classification, forward travel and post-contact advantage. |
| Ground and air `214K/KK` with each strength, repeat at multiple jump heights and ranges | Air dispatch and EX cost, clean and guard hit count, float/juggle, air travel, landing and recovery. |
| Close and midrange `236P/PP`, each strength, on hit, guard and whiff, then projectile versus projectile | Spawn, travel, recovery, clash behavior, contact counts, push, launch and EX meter. |
| `623P/PP` at point-blank and anti-air distances, with separate early and late contact | Invulnerability, hit selection, guard push, freeze, knockdown, landing and FADC window. |
| `236236P` at full Super, `236236PPP` at first legal Revenge and at full, close and midrange, grounded and airborne Ken | Freeze and spawn, active contact count, damage/guard scaling, juggle, finish and meter consumption. |
| Throw both directions, `HP+HK`, both dashes, tap/hold Focus through all three levels and `236P→MP+MK→66` | Throw range/tech and wakeup, appeal availability, dash distance/recovery, armor interval, crumple, block reaction and FADC spend. |

Store the input and timing excerpts with the next evidence pass. Record the complete setup for every measured difference, including cases where both games permit the same follow-up.
