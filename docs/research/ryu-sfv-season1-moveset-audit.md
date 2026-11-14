# Ryu moveset audit for Street Fighter V Season 1

## Source and port

The source is WydD's rendered Ryu game-script extraction, snapshot `004` labeled `1 [cpt-version]`. Capcom's January 2017 update corroborates the preceding Season 1 values for LP Hadoken's 46-frame total, Mind's Eye's 29-frame whiff recovery and +25 V-Gauge reward, and HP Shoryuken's 140 damage and 200 stun. The executable, exact Season 1 patch, region and platform behind the extracted scripts are unconfirmed. The hashes below identify the captured HTML. [Snapshot index](https://sfvsim.com/diff/RYU/004/) · [Extraction documentation](https://sfvsim.com/diff/about.html) · [MoveTool parser](https://github.com/DoctorBearPhD/MoveTool) · [Capcom Season 2 Ryu change list](https://game.capcom.com/cfn/sfv/as/pdf/en/adjustments201701ver.pdf?20220329_2=&h=fa5671af393d0444a3061d628ab63c1b)

| Rendered artifact | SHA-256 |
|---|---|
| Snapshot index | `0f45706e3f62ecf321566a1db2398c5db03676bfa7948b5b02ae84de80cf5d3e` |
| `scripts/600_5LP.html` | `daec5b4f7b58b65719f3e27e0ee8103be9b03f7791ac1ff7b71d9118aebac1d6` |
| `scripts/900_FB_L.html` | `ab1c0ef91ccbbdcc09dc18c6e18372321977476557e111665b06dea2f3de4282` |
| `scripts/740_VT_01_RENSEI.html` | `78ac68e4ca54476c81379d4849af037614a5110b5989e09f1a010641aa124fc4` |
| `hiteffects/0.html` | `0db2fac68b5dfd394f209657f146bdf34d1ee8e8dcc7b033d71d21b9dcb5018c` |

The port baseline is Git commit `0e336743760689ccdd192cbe4ebce5c4d638c9c4` with the working-tree #138 pacing regression. Set up local Ryu versus Ken with the **SF5** Era Style, Era Default gauges, Classic controls and **Mind's Eye / Denjin Renki**, the sole Ryu loadout in this revision. Existing pacing evidence verifies the port target of 63 px of forward walk over 16 ticks, standing LP `[2,4)` and actionable at 11, and LP Hadoken spawn at 13 and actionable at 46. The source records forward walk velocity of `0.047` with a `0.5` pushbox width. The 63 px is the port conversion under the established #138 normalization. The tests establish those pacing targets independently of the static comparison below. [Pacing evidence](2026-11-13-ryu-era-pacing.md) · [Port pacing regression](../../crates/sf-engine/tests/st_ryu_pacing.rs) · [Port revision rules](../../crates/sf-engine/src/era.rs)

## Supplemental frame tables

SFVSim exposes separate script snapshots for Season 1 CPT (`004`), Arcade Edition launch v3.002 (`020`), Kage balance v4.000 (`030`), Champion Edition launch v5.001 (`040`) and Seth release v5.012 (`041`). Use the versioned timelines for active boxes, cancel lists and hit-effect records. [SFVSim version index](https://sfvsim.com/diff/RYU/004/) · [CE launch](https://sfvsim.com/diff/RYU/040/index.html) · [Seth release](https://sfvsim.com/diff/RYU/041/index.html)

The later snapshots establish when Ryu gains V-Trigger II, V-Skill II and V-Shift, as well as changes to parry, install and reversal timing. [Ryu V-System revision comparison](ryu-sfv-v-system-revisions.md)

FAT publishes a spreadsheet-derived roster table. Its pinned late Champion Edition JSON includes Luke and V-Trigger II rows. Ryu's ordinary jab agrees at 3 startup, 2 active and 7 recovery. Ordinary LP Hadoken is 14 startup and 45 total there, compared with 46 total in the Season 1 CPT scripts. The specific CE game patch is unconfirmed for this FAT file. Use its move rows as CE comparisons and retain the `004` scripts for this audit. [Pinned SFV JSON](https://raw.githubusercontent.com/D4RKONION/FAT/dee6b85a7f8f166701ff3f8ab6cfa954949d99aa/src/js/constants/framedata/SFVFrameData.json) · [FAT spreadsheet provenance](https://github.com/D4RKONION/FAT#frame-data--general-data-corrections) · [FAT app](https://fullmeter.com/fat/)

The standalone FAT and FATOnline export last changed on March 14, 2021 and has 42 roster entries. The later FAT snapshot has 46, adding Rose, Oro, Akira and Luke. The two exports agree on the standing LP and ordinary LP Hadoken timing above, including 31 recovery for Hadoken. Ordinary LP Hadoken damage is 60 in the standalone export and 70 in the later snapshot. The exact game patch for the standalone export is unconfirmed. [Standalone export provenance](https://github.com/D4RKONION/fatsfvframedatajson/tree/f02f85d57eb4bd2b27f9c41fdf2c07aafa00aa17) · [Pinned standalone SFV JSON](https://raw.githubusercontent.com/D4RKONION/fatsfvframedatajson/f02f85d57eb4bd2b27f9c41fdf2c07aafa00aa17/sfv.json) · [FATOnline](https://fullmeter.com/fatonline/)

## Command inventory

The source index contains 91 command rows. Directions use numpad notation relative to Ryu facing right. Each grounded button dispatches the same standing normal across distances. The source lists proximity hitboxes inside some actions. [Extracted command index](https://sfvsim.com/diff/RYU/004/)

| Rows | Inputs and variants | Source dispatch |
|---:|---|---|
| 18 | `5/2/8` × `LP/MP/HP/LK/MK/HK` | Standing, crouching and jumping normals |
| 9 | `5/2/8` × `LP/MP/HP` in Denjin Renki | `VT_01_5/2/8` punch overrides |
| 5 | `4HK` Axe Kick, `6MP` Collarbone Breaker, `6HP` Solar Plexus Strike, plus two powered overrides | `4HK`, `6MP`, `6HP`, `VT_01_6MP`, `VT_01_6HP` |
| 4 | Target routes from `5MP` or `5HP`, direction-dependent HK continuation, and a powered HP continuation | `TC_5HP`, `TC_5HK`, `TC_5HK_BACK`, `VT_01_TC_5HP` |
| 5 | Rapid `5LP`, `2LP`, `2LK` and two powered LP overrides | `RC_5LP`, `RC_2LP`, `RC_2LK`, powered `RC_5LP/2LP` |
| 8 | `236LP/MP/HP/PP` ordinary and powered | `FB_*` and chargeable `FBD_*` |
| 8 | `623LP/MP/HP/PP` ordinary and powered | `SHOURYU_*` and `VT_01_SHOURYU_*` |
| 4 | `214LK/MK/HK/KK` on ground | `TATSU_*` |
| 7 | `j.214LK/MK/HK` standard and early-forward, and `j.214KK` | Air Tatsumaki actions and forward variants |
| 12 | `236236P`, each punch and Shoryuken-cancel variant, ordinary and powered | Shinku Hadoken `SFB_*` and Denjin Hadoken `SFBD_*` |
| 4 | `MP+MK`, regular, special-input-lock and powered command variants | `V_SKILL` and `V_SKILL_CANCELBAN` share the Mind's Eye actions |
| 2 | `HP+HK` from neutral and cancel | `VT_RENKI` and `VT_RENKI_CANCEL` |
| 1 | `6PPP` during blockstun | Hashogeki V-Reversal |
| 3 | `LP+LK` neutral, forward and back | `THROW_N/F/B` |
| 1 | All six attack buttons | Appeal |

Capcom names the target routes Sanrengeki (`5MP → 5HP → 5HK`) and Nirengeki (`5HP → 5HK`). The input rows for target continuations also list forward HP and forward or back HK. The run-time buffer and overlap priority are open. Season 1 inventory excludes V-Shift, V-Skill II and V-Trigger II. [Capcom move names and inputs](https://game.capcom.com/cfn/sfv/character/ryu/movelist?lang=en) · [Capcom Season 2 Ryu change list](https://game.capcom.com/cfn/sfv/adjust/201701/ryu?lang=en)

## Script timing and reaction fields

Intervals are `[start,end)` in script ticks. The renderer positions action entries at 30 pixels per tick. `Without Interruption` gives the actionable boundary used by #138. Hit-effect data below uses an ordinary standing hit and standing guard unless specified. Freeze is attacker/defender. Reaction duration, horizontal push and guard white damage are raw fields. The freeze, duration and cancel fields still require live comparison to measure elapsed advantage and input leniency. [Action and hit-effect indexes](https://sfvsim.com/diff/RYU/004/) · [Reaction display example](https://sfvsim.com/diff/RYU/004/hiteffects/0.html)

| Contact | Source windows and effects | Standing hit | Standing guard |
|---|---|---|---|
| `5LP` | `[2,4)` · 0 | 30 damage, 70 stun, freeze `8/8`, duration 8, push `0.35` | 8 white, freeze `8/8`, duration 9, push `0.40` |
| `2LK` | `[3,5)` · 31 | 20 damage, 70 stun, freeze `8/8`, duration 6, push `0.33` | 5 white, freeze `8/8`, duration 8, push `0.35` |
| `2MK` | `[5,7)` · 33 | 50 damage, 100 stun, freeze `12/12`, duration 12, push `0.50` | 13 white, freeze `12/12`, duration 11, push `0.45` |
| `2HK` | `[6,8)` · 35 | 100 damage, 150 stun, freeze `15/15`, duration 12, push `0.25` | 25 white, freeze `15/15`, duration 19, push `0.20`. Guard branch `646`. |
| Axe Kick `4HK` | `[7,10)` · 13, `[13,17)` · 14 | 40+40 damage, 60+90 stun, freeze `12/12` then `15/15`. The `[13,14)` opening of the second strike is air only. | Freeze `12/12` for each strike |
| Collarbone Breaker `6MP` | `[19,20)` · 55, `[20,23)` · 56 | Early and late overhead effects have 30 damage each, 60/40 stun, freeze `12/12`, push `0.20/0.25`. The script gives them separate hit groups. Verify which groups connect in play. | Push `0.20` each |
| Solar Plexus Strike `6HP` | `[16,18)` · 57, `[19,21)` · 58 | 45+45 damage, 50+50 stun, freeze `15/13` then `15/15`, push `0.15/0.35` | Same freeze as hit, push `0.25` each |
| Target HP and HK | Each `[8,10)` · 61 and 63 | 60+70 damage, 70+80 stun, freeze `15/15` then `12/12`. HK uses an airborne blow and excludes crouching opponents. | HP push `0.30`. HK has guard continuation `676`. |
| Forward and back throw | Grab `[4,6)` · 90, attack `[55,56)` · 91 or 92 | Forward 120 damage, 170 stun. Back 140 damage, 200 stun. | — |
| LP/MP/HP Shoryuken | Early `[2,4)` / `[3,6)` / `[4,6)`. Late begins at the corresponding boundary and runs to 14. Effects 101–106. | Early 100/120/140 damage, 150/150/200 stun. Late 60 damage each, 100/100/150 stun. Freeze `14/14` early, `13/13` late. | Freeze `12/12` |
| EX Shoryuken | `[2,4)` · 107 and `[4,14)` · 108. Early confirmation enters `945` and its `[19,31)` effect 109. | Raw effects 80 damage each, 0/100/200 stun, freeze `16/18`, `16/16` and `15/15`. | Freeze `12/12`, duration 17 |
| Powered LP/MP/HP Shoryuken | Same early and late boundaries as ordinary, with effects 169–174 on the grounded boxes | Early 120/140/160 damage and 200/200/250 stun. Late 80 damage each and 150/150/200 stun. | Freeze `12/12`, duration 16 |
| Ground LK/MK/HK Tatsumaki | LK `[9,11)` · 116, MK `[11,13) [29,31)` · 117, HK `[13,15) [29,31) [45,47)` · 118. MK/HK have on-hit branch actions. Uninterrupted boundaries 41/59/75. | 80/100/120 damage, 200 stun and `14/14` freeze. All three ordinary strengths exclude crouching targets. | 20/25/30 white, `14/14` freeze and duration 20/18/18 |
| Ground EX Tatsumaki | `[9,11) [15,17) [22,24) [27,30) [33,35)` · 160–164. Uninterrupted boundary 61. | Five 28-damage, 40-stun entries. Early `5/1` freeze with pull, final `5/5` with push `2`. Crouching is permitted. | 7 white each, freeze `5/5` |
| Air LK Tatsumaki | Standard `[8,10) [16,18) [24,26)`, early forward `[8,10) [15,17) [23,25)` · 122 | 100 damage, 200 stun, freeze `14/14`, push `2`. Standard windows exclude some standing or crouching targets. | 25 white, freeze `14/14`, push `0.8` |
| Air EX Tatsumaki | `[7,9) [13,15) [17,19) [22,24) [26,28)` · 129 four times, then 130 | 30 damage on each of the first four hits, then 40. 40 stun per hit, freeze `5/1` early then `5/5`, push `-2` early then `3`. | Freeze `5/5`, push `0.8` |

The jab `600` has lists `32` and `77` executable at `[2,7)` on hit or guard. `615` crouching LK buffers rapid list `34` at `[5,11)` on hit, guard or whiff and executes at `[9,11)`. V-Trigger list `77` executes at `[3,7)` after contact. `616` crouching MK buffers lists `32` and `77` at `[1,10)` after contact and executes them at `[5,11)`. [Jab script](https://sfvsim.com/diff/RYU/004/scripts/600_5LP.html) · [Crouching LK](https://sfvsim.com/diff/RYU/004/scripts/615_2LK.html) · [Crouching MK](https://sfvsim.com/diff/RYU/004/scripts/616_2MK.html)

### Command normals and target-combo windows

These are script ticks. Each listed cancel buffers on hit or guard. The source index lists the actions admitted by each cancel list. [CPT cancel lists](https://sfvsim.com/diff/RYU/004/)

| Action | Active / uninterrupted boundary | Cancel destination | Buffer / execute |
|---|---|---|---|
| [Axe Kick `607`](https://sfvsim.com/diff/RYU/004/scripts/607_4HK.html) | `[7,10)` and `[13,17)` / 33. Second hit starts at 14 against grounded opponents. | — | — |
| [Collarbone Breaker `670`](https://sfvsim.com/diff/RYU/004/scripts/670_6MP.html) and [Denjin `758`](https://sfvsim.com/diff/RYU/004/scripts/758_VT_01_6MP.html) | `[19,20)` and `[20,23)` / 40 | — | — |
| [Solar Plexus Strike `671`](https://sfvsim.com/diff/RYU/004/scripts/671_6HP.html) and [Denjin `759`](https://sfvsim.com/diff/RYU/004/scripts/759_VT_01_6HP.html) | `[16,18)` and `[19,21)` / 39 | List `67`, ordinary, EX and Denjin Shoryuken | `[16,31)` / `[26,31)` |
| [Standing MP `601`](https://sfvsim.com/diff/RYU/004/scripts/601_5MP.html) and [Denjin `744`](https://sfvsim.com/diff/RYU/004/scripts/744_VT_01_5MP.html) | `[4,7)` / 17 | List `72`, target HP. List `32`, grounded specials or Critical Art. Ordinary `601` also has list `77`, Denjin activation. | `72` `[3,12)` / `[7,12)`, `32` `[1,10)` / `[5,10)`, `77` `[1,9)` / `[4,9)` |
| [Standing HP `602`](https://sfvsim.com/diff/RYU/004/scripts/602_5HP.html) and [Denjin `745`](https://sfvsim.com/diff/RYU/004/scripts/745_VT_01_5HP.html) | `[6,9)` / 29 | List `73`, target HK. List `33`, Critical Art. Ordinary `602` also has list `77`, Denjin activation. | `73` `[6,12)` / `[9,12)`, `33` `[1,12)` / `[7,12)`, `77` `[1,14)` / `[11,14)` |
| [Target HP `674`](https://sfvsim.com/diff/RYU/004/scripts/674_TC_5HP.html) and [Denjin `768`](https://sfvsim.com/diff/RYU/004/scripts/768_VT_01_TC_5HP.html) | `[8,10)` / 38 | List `73`, target HK | `[8,14)` / `[11,14)` |
| [Target HK `675`](https://sfvsim.com/diff/RYU/004/scripts/675_TC_5HK.html) | `[8,10)` / 36. A guard during the active window branches to [`676`](https://sfvsim.com/diff/RYU/004/scripts/676_TC_5HK_GRD.html), whose own uninterrupted boundary is 24. | List `77`, Denjin activation | `[1,10)` / `[8,10)` |

The three standalone command normals use exact relative directions and a positive button edge. The target HP command row uses forward HP with the script's `Similar` direction restriction. The two target HK rows use forward or back HK with the same restriction and share action `675`. Capcom shows the continuations with plain standing buttons. Check the actual accepted neutral and diagonal inputs, command overlap priority and the guard-branch activation boundary in a paired live run. [Target HP command](https://sfvsim.com/diff/RYU/004/moves/TC_5HP.html) · [Forward target HK command](https://sfvsim.com/diff/RYU/004/moves/TC_5HK.html) · [Back target HK command](https://sfvsim.com/diff/RYU/004/moves/TC_5HK_BACK.html) · [Capcom move list](https://game.capcom.com/cfn/sfv/character/ryu/movelist?lang=en)

## Fireballs and V-System

| Action | Source script and reaction fields |
|---|---|
| Ordinary `236LP/MP/HP` | Projectile effects 0/1/2. Each gives 60 damage, 100 stun, 10 guard white and `8/8` freeze. Raw velocities `0.055/0.066/0.076`. Standing hit duration `19/20/21`, guard `16/17/18`. Push `0.80` hit, `0.45` guard. |
| Ordinary `236PP` | Spawns at 10 and becomes actionable at 44. Velocity `0.085`. Effects 9/10 give 40+60 damage, 0+150 stun, 7+10 guard white, freeze `6/6` then `6/8` on hit, `4/5` then `6/6` on guard. |
| Powered fireball | Uncharged ordinary effects 21/23/25 each give 70 damage and 150 stun, 12 guard white and `6/8` freeze on hit. Ordinary charge effects 33/34 give 45+45 damage with 200 finisher stun. Fully charged effects 35/36 give two early and one final 40-damage contact with 300 finisher stun. Powered EX has three 40-damage entries via effects 30/31, or three 50-damage entries with full charge via effects 37/38. Both full-charge finishers have a guard-break reaction. The powered fireball command consumes 50 raw V timer units. |
| Powered fireball charge routes | LP script `1050` branches on release at `[7,20)` to the ordinary action and `[20,59)` to charged action `1054`. It automatically enters full-charge action `1055` at `[59,61)`. Powered EX `1053` branches to ordinary powered EX on release at `[6,40)` and automatically enters `1056` at `[40,42)`. Projectile spawn is 11 within each charged ordinary continuation and 10 within charged EX. The branch uses a nonzero target script offset. Confirm the elapsed hold and spawn clocks in a live run. |
| Shinku and Denjin Hadoken | Ordinary projectiles 40/41/42 use effects 40/41 for five 40-damage contacts and a 120-damage finisher for 320. Powered projectiles 46/47/48 use effects 52/53 for six 40-damage contacts and a 110-damage finisher for 350, with 300 finisher stun and a guard-break entry. Freeze is `4/8` early and `6/8` on the finisher. In neutral LP scripts `1200` and `1210`, super flash is at 125 and projectile spawn is at 128 on the cinematic script clock. Projectile velocities are `0.06` and `0.09`. Denjin Hadoken's command consumes 1000 raw V timer units. |
| Mind's Eye | Script `885` armor `[2,9)` branches to `886/887/888` for low, mid or high success. Each success grants 25 raw V-Gauge. Actionable at 38 on the whiff script and 22 on the success scripts. Capcom gives 29 frames for whiff recovery. High success removes 200 raw stun at script tick 1. |
| Denjin Renki | The snapshot index configures 800 V-Trigger timer units and two V bars. Neutral `740` and cancel `739` cost 600 raw V-Gauge with 300 per bar. Each script issues an execution-speed change at tick 0, with a duration of 50 or 47 respectively and the opponent set to 0% speed. Timer decay and activation freeze need a live trace. |
| Hashogeki V-Reversal | `6PPP` during blockstun costs 300 raw V-Gauge. First hit `[14,16)` uses effect 143 with 60 recoverable damage and zero stun, `15/15` freeze, standing hit and guard duration `16/22` and push `1.4/0.7`. |

The neutral and projectile scripts use separate hit-effect tables. Ordinary LP Hadoken action `900` spawns `PROJ_0`, which uses projectile effect `PROJ_0`. The charged fireball and CA projectile tables include their own multi-hit and finisher entries. [LP action](https://sfvsim.com/diff/RYU/004/scripts/900_FB_L.html) · [LP projectile effect](https://sfvsim.com/diff/RYU/004/hiteffects/PROJ_0.html) · [LP charge script](https://sfvsim.com/diff/RYU/004/scripts/1050_FBD_L_CHAGE.html) · [Full-charge projectile](https://sfvsim.com/diff/RYU/004/scripts/PROJ_34_FBD_GUARD_BREAK.html) · [Denjin projectile](https://sfvsim.com/diff/RYU/004/scripts/PROJ_46_DENJIN_L.html) · [Mind's Eye action](https://sfvsim.com/diff/RYU/004/scripts/885_V_SKILL.html) · [Neutral V-Trigger action](https://sfvsim.com/diff/RYU/004/scripts/740_VT_01_RENSEI.html)

## Port comparison and remaining measurements

- The source and port pace targets for walk, jab and LP Hadoken match in the #138 regression. The active port dispatch has ordinary Hadoken, Shoryuken and grounded Tatsumaki. Ryu's air Tatsumaki, Axe Kick, Collarbone Breaker, Solar Plexus Strike and named target routes have no corresponding Season 1 dispatch in the inspected path. Record command acceptance before comparing geometry or damage for these moves. [Port dispatch](../../crates/sf-engine/src/era_fight/sf5.rs) · [Port native moves](../../crates/sf-engine/src/data_bank/sf5.rs)
- The port's generic Season 1 projectile slots allow two ordinary EX Hadoken contacts and up to five Critical Art contacts. The extracted source streams specify two ordinary EX contacts and six or seven CA contacts. The port powers the generic special slots in Denjin Renki. The source adds chargeable projectile commands, distinct powered reactions and a separate Denjin CA stream. Test normal and powered `236PP` and `236236LP` at point blank and midscreen, logging every contact and meter change. [Port projectile construction](../../crates/sf-engine/src/era_fight.rs) · [Port V-Trigger and CA dispatch](../../crates/sf-engine/src/era_fight/sf5.rs)
- The port shares a 12-tick defensive counter window and 1000-tick Ryu Denjin duration with its broader SFV path. The source Mind's Eye script uses `[2,9)` and the extracted Denjin definition configures 800 raw units. The port also accepts `6KKK` for V-Reversal during blockstun, where the extracted Ryu row specifies `6PPP`. Measure freeze, phase clocks, activation/cancel timings and meter on the same actions before choosing changes. [Port V-System](../../crates/sf-engine/src/era_fight/sf5.rs)
- The port's shared ground throw initiation uses a directional heavy attack or an assigned throw macro. It already accepts the light chord for techs. The source has `LP+LK` neutral and directional throw initiation rows. Check `5/6/4+LP+LK` and matched tech input on both sides, including a whiff at max grab range, direction switch, ender damage, knockdown and wakeup. [Port fighter input](../../crates/sf-engine/src/fighter.rs)
- Run Ryu versus Ken from reset to control distance, facing, gauge and prior stun. Test neutral, stand guard, crouch guard and air contacts for the marked moves. Sample every advancing tick around `2LK → 2LK`, `2MK → 236LP` or V-Trigger, the two target routes, Mind's Eye against a low, mid and jump-in, and both air Tatsumaki jump arcs. Record first hit, freeze on each fighter, actionability, hit or block stun, displacement and cancellation. Preserve source asset identity and stage state for a live run. Matched grounded Tatsumaki timing and hit branches, exact standing and crouching reactions, powered-normal geometry, counter-hit and crush-counter behavior, command buffering and in-match meter scaling remain open. The source script extraction and the port audit were static apart from the existing pacing regression.

## Reproduce the extraction

1. Open snapshot `004` and enumerate the `moves/` links under the command table. Keep command rows separate when they point to the same action. Follow the `scripts/` link from each representative command and the projectile script linked from its spawn command. [Snapshot](https://sfvsim.com/diff/RYU/004/)
2. Use the timeline's left and width CSS values divided by 30 to recover a half-open script window. On the same page, inspect each attack-box row for its effect ID and allowed target states. Read cancel buffer and execute lanes independently, and follow confirm and guard branches. [Timeline example](https://sfvsim.com/diff/RYU/004/scripts/600_5LP.html)
3. Follow each effect ID into `hiteffects/`. For a parser, expand HTML `colspan` to preserve standing, crouching, off-the-ground and air, the four corresponding counter-hit columns, and the two guard columns. Read the projectile effect namespace for projectiles. The effect tables contain raw push, damage and reaction fields in addition to the action clock. [Normal effect example](https://sfvsim.com/diff/RYU/004/hiteffects/0.html) · [Projectile effect example](https://sfvsim.com/diff/RYU/004/hiteffects/PROJ_0.html)
