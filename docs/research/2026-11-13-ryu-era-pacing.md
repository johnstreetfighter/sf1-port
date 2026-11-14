# Ryu’s walk, jab, and Hadoken across the Era Styles

This comparison looks at how quickly Ryu can change spacing, threaten with a jab, and move again after throwing a light Hadoken. The port baseline is `cfa43bd49309cdc55ede47277a031ca5a5197305` (`1.5.0`). The arcade runs use MAME `0.289`. Each figure describes a named setup, and the sources use different units for 2D and 3D movement.

## Spec #138

Spec #138 already calls for recovery, displacement, and timing differences to be recorded per fighter and revision, even when the same combo works. It pins World Super Turbo `940223` as the first source and asks audits to record the clock and coordinate system. Its initial acceptance criteria focused on moves, hit reactions, and combo routes. The earlier Super Turbo report left regular Hadoken recovery open (`9646b61`, `docs/research/ryu-super-turbo-audit.md` on `codex/ryu-era-audit`). The three probes here extend that evidence. [Spec #138](https://github.com/johnstreetfighter/sf1-port/issues/138) · [Super Turbo normals #139](https://github.com/johnstreetfighter/sf1-port/issues/139) · [Super SFII #145](https://github.com/johnstreetfighter/sf1-port/issues/145) · [Published modern evidence #147](https://github.com/johnstreetfighter/sf1-port/issues/147)

The port’s clock targets 60 Hz for every Era Style. SFII revision deltas adjust selected move startup, recovery, and stun. Capcom’s Super Turbo arcade manual also documents a speed control for the whole fight, and its Alpha 3 manual lists Normal, Turbo 1, and Turbo 2. The clock, ordinary movement, and move data each have a bearing on how an Era Style feels. [Port pacing](../../crates/sf-host/src/pacing.rs#L15) · [Revision deltas](../../crates/sf-engine/src/era_fight.rs#L234) · [Super Turbo operator manual](https://arcarc.xmission.com/PDF_Arcade_Manuals_and_Schematics/Street%20Fighter%20II%20Turbo.pdf) · [Alpha 3 operator manual](https://manualzz.com/doc/4196656/capcom-street-fighter-alpha-3-arcade-game-operators-manual)

## Timing endpoints

- **Walk** uses sustained forward world displacement after movement begins, with both fighters clear of the wall and each other. Port and arcade movement is in world pixels per second. The original IV, V, and 6 use their own world scales. Their rates require a common spacing reference, such as time to cover one Ryu standing pushbox width, before a numerical comparison with the port.
- **Jab** is a whiffed standing LP. Use the far version where the source selects by distance. The numbers give elapsed time from the sampled button input to the first live hitbox and to the end of the move. SFII arcade uses return to the neutral state. SFIII uses the first accepted walk input. The charts and game data for Alpha and IV–6 start at move onset, so they exclude any input processing before that onset. A chart’s frame 4 is three elapsed intervals from the first move frame.
- **Hadoken** uses the first active LP projectile sample through neutral or accepted walk, with no contact. When a chart supplies the number, it uses that chart’s published recovery convention. An animation endpoint, the end of the simulation’s action lock, and the first observed walk can differ by a frame. Port and arcade timing use their respective display intervals. Published values are rounded when expressed in milliseconds.

## Source measurements

| Reference build and setting | Forward walk | Jab active / end | LP Hadoken recovery |
| --- | ---: | ---: | ---: |
| SFII World Warrior, World `910522`, `sf2` | 179 px/s | 67 / 218 ms | 671 ms |
| SFII Champion Edition, World `920513`, `sf2ce` | 179 px/s | 67 / 218 ms | 671 ms |
| SFII Hyper Fighting, World `921209`, `sf2hf` | 246–257 px/s | 50 / 151–168 ms | 470–587 ms |
| Super SFII, World `931005`, `ssf2` | 179 px/s | 67 / 218 ms | 671 ms |
| Super Turbo New, World `940223`, default Turbo 1, `ssf2t` | 224 px/s | 50–67 / 168–184 ms | 537–553 ms |
| Super Turbo Old, same World setup | 224–235 px/s | 50–67 / 168–184 ms | 520–537 ms |
| Super Turbo New, Japan `940311`, Turbo 3, `ssf2xj` | 224–246 px/s | 50 / 168 ms | 503–537 ms |
| Alpha 3, A-ISM (Zero 3 Z-ISM chart) | 3 px/game tick | 3 / 14 game ticks | 45 game ticks |
| SFIII New Generation, Asia `970204`, `sfiiina` | ~195 px/s | 50 / 185 ms | 722 ms |
| SFIII 2nd Impact, Asia `970930`, `sfiii2n` | ~195 px/s | 67 / 185 ms | 722 ms |
| SFIII 3rd Strike, Japan `990512`, `sfiii3nr1` | 194 px/s | 67 / 185 ms | 621 ms |
| Ultra SFIV, guide’s 2014 pre-release build | 0.045 source units/frame | ~50 / 200 ms | ~550 ms (33f listed) |
| SFV, earlier Prima guide | 0.047 source units/frame | ~33 / 183 ms | ~533 ms (32f listed) |
| SFV Champion Edition, normal state, published data | 0.047 source units/frame | ~33 / 183 ms | ~517 ms (31f listed) |
| SF6, Classic, December 2024 game data | 4.7 source units/frame | ~50 / 217 ms | ~520–533 ms (31f listed, 32 intervals from zero-based spawn to action margin) |

### SFII and SFIII arcade runs

The SFII sets run at 59.637405 display Hz, with a 384-pixel viewport. Each row uses Ryu against Ken, three steady walks, three far jab whiffs, and five clean LP Hadoken whiffs. The walk spans 16 display intervals, starting at the first world X change. Ken jumps over each fireball. His HP and both fighters’ horizontal positions stay fixed through the whiffs. The stage selector read 0 for World Warrior. The original World ST stage label is unverified. The controlled Turbo 1 and Turbo 2 runs below use Ken’s dock, verified visually at both speeds. The original World ST run uses a fresh default EEPROM (`0x70` at `FF8CD3`). The Japan run sets the ROM’s Turbo 3 threshold (`0x60`) once after the round starts and leaves the pacing code untouched. The numbers describe these emulator setups. An open MAME report documents a difference from a physical Hyper Fighting cabinet. [World ST baseline recipe](https://github.com/johnstreetfighter/sf1-port/issues/138) · [SFII object and hitbox map](https://github.com/peon2/fbneo-training-mode/blob/master/hitboxes/sf2-hitboxes.lua) · [ST object and hitbox map](https://github.com/peon2/fbneo-training-mode/blob/master/hitboxes/st-hitboxes.lua) · [Japan ST pacing code](https://github.com/jotego/jtcores/issues/1584) · [MAME timing report](https://mametesters.org/view.php?id=00408)

| Build | Walk delta per 16 intervals | Jab intervals to active / neutral | Fireball intervals to neutral |
| --- | --- | --- | --- |
| WW | 48, 48, 48 px | 4/13 × 3 | 40 × 5 |
| CE | 48, 48, 48 px | 4/13 × 3 | 40 × 5 |
| HF | 66, 69, 69 px | 3/9, 3/10, 3/10 | 29, 29, 28, 28, 35 |
| Super | 48, 48, 48 px | 4/13 × 3 | 40 × 5 |
| ST World default | 60, 60, 60 px | 4/11, 3/11, 3/10 | 33, 32, 32, 32, 32 |
| ST World default, Old Ryu | 60, 60, 63 px | 4/11, 3/10, 3/10 | 32, 32, 32, 31, 31 |
| ST Japan Turbo 3 | 63, 66, 60 px | 3/10 × 3 | 31, 30, 30, 32, 31 |

For WW, CE, HF, and ST, the three forward holds start at frame-end events 2135, 2220, and 2300 and run through start +18. Back is held from start +26 through start +50. Far jab pulses begin at 2415, 2492, and 2568 and last four samples. Fireball motions begin at 2650, 2800, 2950, 3100, and 3250. Each uses four samples Down, four Down-Forward, and five Forward+LP. Ken holds Up from start +11 through start +16. The ST World start sequences use `new_baseline` and `old_baseline` from the prior audit. The Old flag was set for Ryu and clear for Ken. Super uses the same walk and jab holds with starts 2630/2715/2795 and 2910/2987/3063. Its fireball starts are 3145/3295/3445/3595/3745 and Ken jumps at +14…+19. The first sample with an attack box on the live projectile and the first Ryu neutral sample bound recovery.

The SFIII sets run at 59.599491 display Hz. NG and 2I use Ryu P1 against Ken. 3S uses Ryu P2 against Alex. Each uses default settings and an opponent who jumps over the projectile. Recovery ends when held retreat selects the walk animation. NG/2I jab input appears at sample 1901, the active box at 1904/1905, and walk at 1912. The Hadoken spawns at 2120 and walk starts at 2163. Forward walk spans samples 2301–2316. In 3S the jab samples are 1801/1805/1812 and the Hadoken samples 2016/2053. The 3S walk uses the 16.16 position at steady speed (3.25 px per interval). The same 3S replay reproduced its jab and Hadoken timestamps. [SFIII set labels](https://github.com/mamedev/mame/blob/mame0289/src/mame/capcom/cps3.cpp) · [CPS3 hitbox map](https://github.com/peon2/fbneo-training-mode/blob/master/hitboxes/cps3-hitboxes.lua)

| SFIII input, inclusive frame-end events | NG and 2I | 3S |
| --- | --- | --- |
| Standing LP | 1900–1901 | 1800–1801 |
| Hold retreat after jab | 1908–1920 | 1809–1817 |
| Down, Forward, LP for Hadoken | 2104–2107, 2106–2111, 2110–2111 | 2000–2003, 2002–2007, 2006–2007 |
| Opponent holds Up | 2130–2133 | 2012–2014 |
| Hold retreat after Hadoken | 2130–2170 | 2040–2057 |
| Hold Forward for walk | 2300–2315 | 2200–2215 |

### Published Alpha and modern source values

- **Alpha 3.** The Japanese Zero 3 hitbox archive’s Ryu Z-ISM action chart gives +48 px over 16 game ticks for forward walk (action 8), first active at tick 3 and end at tick 14 for far LP (48, shared across ISMs), and spawn at tick 12 and end at tick 57 for LP Hadoken (87, shared by Z and V). The data describe game ticks. The archive leaves the ROM revision and Turbo playback setting unspecified. A direct Normal/Turbo run would establish elapsed wall time. [Zero 3 hitbox and action archive](https://cf710705.cloudfree.jp/)
- **Ultra SFIV.** Prima’s Ryu section lists forward walk at 0.045 world units per frame, close jab at 3/3/6 and far jab at 4/3/6 (startup, active, recovery). Its Hadoken lists 13 startup and 33 recovery. The guide specifies a 60 Hz pre-release build. Its far jab ends after 12 occupied frames. The source build and recovery convention both matter for a one-frame parity claim. [Prima Ultra Street Fighter IV Bible, pp. 475–479](https://www.scribd.com/document/358157078/prima-ultra-street-fighter-iv-bible-eguide-pdf)
- **SFV.** Prima lists the earlier jab at 3/2/7 and LP Hadoken recovery at 32 frames. It gives forward walk as 4.7 on its scale. Capcom’s Arcade Edition balance notes cut Hadoken’s overall duration from 47 to 45 frames. The Champion Edition published data lists normal standing LP at 3/2/7 and normal LP Hadoken at 14 startup, 45 total, and 31 recovery. The exact build used by the earlier guide is unspecified. [Prima walk table](https://primagames.com/eguides/street-fighter-v-eguide/glossary/k-o) · [Prima SFV jab](https://primagames.com/eguides/street-fighter-v-eguide/fighters/ryu/standing-normal-moves) · [Prima SFV Hadoken](https://primagames.com/eguides/street-fighter-v-eguide/fighters/ryu/special-moves) · [Capcom’s Arcade Edition balance notes](https://news.capcomusa.com/lets/browse/street-fighter-v-arcade-edition-balance-updates-and-developer-comments-for-december-16) · [Champion Edition frame data snapshot](https://github.com/D4RKONION/FAT/blob/dee6b85a7f/src/js/constants/framedata/SFVFrameData.json)
- **SF6.** The December 2024 move dump gives the forward-walk loop (`BAS_FORWARD_Loop`, action 10) a fixed value of 4.6999969. Standing LP (600) has a strike box on zero-based ticks 3–5 and an action margin of 13. Light Hadoken (900) spawns on tick 15 and has an action margin of 47. FAT's December 9 Winter 2024 update gives a 4/3/7 jab and 16/31 fireball under its published convention. [MMDK Ryu move dump, December 2024](https://github.com/alphazolam/MMDK/blob/831564f1993926a9fff834108f24767748d40c06/MMDK/reframework/data/MMDK/PlayerData/Ryu/Ryu%20moves_dict.json) · [FAT December 2024 frame data snapshot](https://raw.githubusercontent.com/D4RKONION/FAT/17eea0064c376e68883f96bb260499ed25dc4095/src/js/constants/framedata/SF6FrameData.json)

The expanded moveset audits record the exact build or extraction used for each timing claim.

- **Ultra SFIV.** Use the pinned SuperCombo v1.05 table and FAT JSON for published frame checks. The audit records FAT's total-duration convention for Ryu's Hadoken and the corresponding raw script timings. The Shib archive is an alternate index. [Ultra SFIV moveset audit](ryu-usfiv-moveset-audit.md) · [Shib Ultra index](https://srk.shib.live/w/Ultra_Street_Fighter_IV)
- **SFV.** SFVSim exposes versioned game scripts for hitboxes, cancels and contact effects. The standalone FAT JSON last changed in March 2021. The later FAT app export adds Rose, Oro, Akira and Luke. The audit pins both exports and tracks the Season 1 and Champion Edition differences. [SFV moveset audit](ryu-sfv-season1-moveset-audit.md) · [SFVSim script revisions](https://sfvsim.com/diff/RYU/) · [Standalone FAT export](https://github.com/D4RKONION/fatsfvframedatajson/tree/f02f85d57eb4bd2b27f9c41fdf2c07aafa00aa17)
- **SF6.** Cross-check actions with Capcom's tables and the Training Mode Frame Meter on the corresponding balance. The current audit uses December 2024 game scripts and the corresponding FAT export. The checked SF6 Sensei snapshot derives from SuperCombo and identifies its balance as August 3, 2026. [December 2024 SF6 moveset audit](ryu-sf6-december-2024-moveset-audit.md) · [Capcom Ryu frame table](https://www.streetfighter.com/6/en-us/character/ryu/frame) · [SuperCombo SF6 index](https://wiki.supercombo.gg/w/Street_Fighter_6) · [SF6 Sensei version pin](https://github.com/RyoSogawa/sf6-sensei/blob/a0b4105fb368c308699ec0757ee6e4182e5a9d1c/packages/core/src/version.ts)

## Port v1.5.0 compared with the sources

The native probe exercised all 22 port revisions, including both Super Turbo loadouts and one selected loadout per other revision. Every row walked **40 px in 17 ticks**, or **141 px/s**. Their corrected world X followed the same sequence, including uneven steps within the walk cycle. Every tested jab had two active ticks. In the table, the port values use the first active hitbox and first neutral state with the input gate clear. The source columns use the endpoints documented above.

| Port style and source | Walk port / source | Jab active / end, port → source | LP Hadoken recovery, port → source |
| --- | --- | --- | --- |
| Faithful SF1 | 141 px/s | 67 / 133 ms | 950 ms |
| SF1+ | 141 px/s | 50 / 117 ms | 950 ms |
| SF2 WW | 141 / 179 px/s | 67 / 150 → 67 / 218 ms | 967 → 671 ms |
| SF2 CE | 141 / 179 px/s | 50 / 133 → 67 / 218 ms | 967 → 671 ms |
| SF2 HF | 141 / 246–257 px/s | 33 / 100 → 50 / 151–168 ms | 917 → 470–587 ms |
| SF2 Super | 141 / 179 px/s | 50 / 117 → 67 / 218 ms | 950 → 671 ms |
| SF2 ST New, World default | 141 / 224 px/s | 50 / 117 → 50–67 / 168–184 ms | 933 → 537–553 ms |
| SF2 ST Old, World default | 141 / 224–235 px/s | 50 / 133 → 50–67 / 168–184 ms | 967 → 520–537 ms |
| SFZ3 A-ISM | 2.35 px/port tick / 3 px/source tick | 3 / 7 port ticks → 3 / 14 source ticks | 57 port ticks → 45 source ticks |
| SF3 NG, Super Art I | 141 / ~195 px/s | 50 / 117 → 50 / 185 ms | 950 → 722 ms |
| SF3 2I, Super Art I | 141 / ~195 px/s | 50 / 117 → 67 / 185 ms | 950 → 722 ms |
| SF3 3S, Super Art I | 141 / 194 px/s | 50 / 117 → 67 / 185 ms | 950 → 621 ms |
| USF4, Ultra I | 141 px/s / 0.045 source units/frame | 50 / 117 → ~50 / 200 ms | 950 → ~550 ms |
| SF5 CE, V-Skill I and V-Trigger I | 141 px/s / 0.047 source units/frame | 50 / 117 → ~33 / 183 ms | 950 → ~517 ms |
| SF6, Classic | 141 px/s / 4.7 source units/frame | 50 / 117 → ~50 / 217 ms | 950 → ~520–533 ms |

The SFZ1, SFZ2, SFZ2A, SF4, SSF4, AE2012, SF5, and SF5 AE port rows each measure 141 px/s, 50/117 ms for jab, and 950 ms for Hadoken, just like their family’s later port style. Source coverage for those earlier revisions is partial. The Alpha 3 chart establishes game-tick timings. Direct captures at a pinned Normal or Turbo setting would establish how they translate to elapsed time. IV–6 have published timing data and game-native walk values. Comparable movement needs a shared distance calibration. [Modern direct evidence #148](https://github.com/johnstreetfighter/sf1-port/issues/148) · [#149](https://github.com/johnstreetfighter/sf1-port/issues/149) · [#150](https://github.com/johnstreetfighter/sf1-port/issues/150)

The probe used the port’s native Local Versus fight with Ryu P1 and Ken P2 in country 0. Faithful used Normal Speed DIPs `dfff/ffff`. The other rows selected Ryu’s revision and loadout in Definitive Overhaul with the era’s default gauge, Classic controls, and SF1+ Ken. Each probe cloned the fight 96 ticks into the active round. For walk, hold Forward and compare zero-based ticks 16 and 33. World X adds camera X to the collision center and subtracts the signed pose offset. The pushboxes stay disjoint, and the center gap drops from 92 to 52 px. For the jab, press LP on tick 0 and release on tick 1, with the opponent out of range. Faithful starts on release. The other rows start on press. Before the Hadoken trial, both fighters hold Back for 42 ticks and then wait 18. In overhaul, use Down, Down-Forward, and Forward+LP on consecutive ticks. In Faithful, use Down on ticks 0–1, Down-Forward on tick 2, Forward on 3–21, and LP on 6–9. Ken jumps forward on ticks 15–26 of the motion trial. The probes verify no damage for 160 ticks and register a hit when the jump is omitted. Holding retreat also confirmed the same first actionable Hadoken tick for SF1+, ST New, and 3S. [Fight entry point](../../crates/sf-engine/src/front/fight.rs#L1837) · [Faithful movement reference](../reference/movement.md#walk-cycles) · [Faithful move stream audit](../reference/rom-box-audit.md#ryu)

## Implications from the v1.5.0 baseline

The port’s HF Ryu covers about 55–57% of arcade HF Ryu’s ground per second in these fixtures. Yet his jab is back at neutral about 51–68 ms sooner, and he stays committed to a whiffed Hadoken about 330–447 ms longer. World Super Turbo New and Third Strike show the same directions of difference. A shared speed increase would shorten the fireball wait and also shorten the already quicker jab. The movement cycle, jab duration, and fireball recovery each have a source-specific target.

These measurements gave #138 three repeatable checks to add alongside the combo and reaction tests. The initial comparison raised the question of whether a source revision’s faster cadence would still need to be represented after ordinary movement and attack timing agreed. The World Turbo 2 measurements and playtest below informed the final decision. [Spec #138](https://github.com/johnstreetfighter/sf1-port/issues/138)

## World Turbo 2 versus v1.5.1

The tournament reference is World Super Turbo `940223` at Turbo 2. First Attack 2026 specified USA Turbo 2 or Japan Turbo 3 for its MiSTer CPS2 event. The measurements here use the World build in MAME 0.289 at 59.637405 display Hz. The port runs at 60 Hz. [First Attack 2026 rules](https://www.start.gg/tournament/first-attack-2026-2/event/super-street-fighter-ii-x-grand-master-challenge-st) · [Super Turbo operator manual](https://arcarc.xmission.com/PDF_Arcade_Manuals_and_Schematics/Street%20Fighter%20II%20Turbo.pdf)

The v1.5.1 executable identifies commit `af1104ab`, the tagged source used by the ROM-backed Local Versus test. New and Old Ryu both moved 60 world pixels in 16 ticks, or 225 px/s. Their far jabs went live at tick 4 and accepted retreat at tick 11, or 67 and 183 ms. A whiffed light Hadoken allowed retreat 33 ticks after the live projectile for New Ryu and 32 for Old, or 550 and 533 ms. These results held against New Super Turbo Ken and against SF1+ Ken. [v1.5.1 pacing test](../../crates/sf-engine/tests/st_ryu_pacing.rs#L120)

To reproduce the cabinet setting, start with the default World EEPROM. In MAME’s Test Menu, open System Configuration, move to Game Speed, and select Turbo 2. Exit with P1 Start to save. The headless service sequence holds each press for four display samples starting at the following frame-end events.

| Frame-end events | Input |
| --- | --- |
| 1200 | Service Mode |
| 1320, 1340 | P1 Down |
| 1360 | P1 LP to open System Configuration |
| 1430, 1450, 1470, 1490, 1510, 1530, 1550, 1570 | P1 Down to Game Speed |
| 1590 | P1 Right to select Turbo 2 |
| 1610, 1630 | P1 Down to Exit |
| 1650 | P1 Start to save |

Saving through the menu changed the two mirrored speed bytes at EEPROM offsets `0x1f` and `0x4f` from `01` to `02`. The resulting local 128-byte EEPROM has SHA-256 `93616ee35398f10821786a3c1237bc665e9ba9d53e77db9d54b21223119ed2d1`. [Capcom’s cabinet configuration instructions](https://arcarc.xmission.com/PDF_Arcade_Manuals_and_Schematics/Street%20Fighter%20II%20Turbo.pdf)

The comparison runs booted with the saved EEPROM at each speed and used controller inputs only. Every sampled row from frame 2135 through 3380 confirmed an active round, stage ID 4, the selected Ryu loadout, and New Ken. A screenshot of the replay verifies that stage ID 4 is Ken’s dock stage. The live speed selector and pacing threshold were `1 / 0x70` for Turbo 1 and `2 / 0x60` for Turbo 2. The same walk, far LP, and light Hadoken input recipe above was used at both speeds. The Old Ryu runs also held Right at 1478–1479, 1482–1483, and 1486–1487, then Left+LP at 1490–1491 to select Old Ryu.

For each jab and Hadoken, replay the sequence up to that trial. Hold Back from jab +4 to +18 or from the Hadoken motion start +13 to +58. This preserves spacing and cadence through the start of each trial. Record the first live box, the first live projectile, and the first decrease in Ryu’s world X. The port test identifies recovery when the engine accepts held Back and selects backward walk. Verify that Ken takes no damage and neither fighter enters contact freeze. The summary starts jab timing at the first frame sampled after LP is applied, the same origin used by the port test. The raw arcade table retains the capture convention of starting one display interval earlier, at the frame-end event that queues LP.

Replaying the arcade jabs with one LP sample and immediate held Back, the same input duration as the port test, reproduced the jab timings at both cabinet speeds.

| Setup | Forward walk in world px/s | Far jab input to active / retreat | Light Hadoken live projectile to retreat |
| --- | ---: | ---: | ---: |
| v1.5.1 New Ryu | 225 | 67 / 183 ms | 550 ms |
| World Turbo 2 New Ryu | 235 | 34 / 151 ms | 520–537 ms |
| v1.5.1 Old Ryu | 225 | 67 / 183 ms | 533 ms |
| World Turbo 2 Old Ryu | 224–235 | 34 / 151 ms | 503–520 ms |

| Arcade sample | Walk in 16 display intervals, px | Jab intervals from input event to active / retreat | Projectile to retreat, intervals |
| --- | --- | --- | --- |
| World Turbo 1, New | 63, 60, 60 | 3/11, 3/10, 3/10 | 33, 33, 33, 33, 32 |
| World Turbo 1, Old | 63, 60, 60 | 3/10, 3/10, 4/11 | 32, 31, 32, 32, 31 |
| World Turbo 2, New | 63, 63, 63 | 3/10, 3/10, 3/10 | 31, 32, 32, 31, 32 |
| World Turbo 2, Old | 63, 60, 63 | 3/10, 3/10, 3/10 | 31, 31, 31, 30, 31 |

Turbo 2 New Ryu covered about 4% more ground per second than v1.5.1. Old Ryu’s sampled walk range includes the port rate. Both arcade jabs went live and accepted retreat about 33 ms sooner from the first LP sample. The arcade jab had three live display samples in each Turbo 2 trial, and the port had four live ticks. Light Hadoken allowed retreat 13–30 ms sooner in the arcade for either loadout. The same-stage Turbo 1 controls show that some of the jab gap is already present at Turbo 1. Changing only the cabinet setting moved jab retreat by at most one display interval across these paired trials. Most Turbo 2 fireball trials finished one or two display intervals earlier. These are emulated elapsed timings for three actions in one matchup and stage.

## World Turbo 2 far jab playtest build

A wider cadence sweep on Ken’s stage used the same cabinet-saved Turbo 2 EEPROM. From frame-end event 2415, tap LP for one display sample every 41 intervals for 16 trials per loadout. Hold Back from each tap +1 through +12. Confirm the far jab animation and full health for both fighters on each trial. Read the results below as elapsed ticks from the first LP sample to the live hitbox, the number of active display samples, and elapsed ticks from LP to retreat.

| Arcade timing tuple | New Ryu trials | Old Ryu trials |
| --- | ---: | ---: |
| 2 / 3 / 9 | 13 | 14 |
| 2 / 4 / 9 | 2 | 1 |
| 3 / 3 / 9 | 1 | 0 |
| 3 / 3 / 10 | 0 | 1 |

v1.5.2 uses the most frequent arcade timing, `2 / 3 / 9`, for both Super Turbo Ryu loadouts at far range. The port uses a fixed 60 Hz move stream. [Far jab stream](../../crates/sf-engine/src/data_bank.rs#L1627) · [ROM-backed pacing regression](../../crates/sf-engine/tests/st_ryu_pacing.rs#L123)

## World Turbo 2 walk and light Hadoken playtest build

v1.5.3 uses a shared forward walk of 63 world pixels per 16 port ticks for New and Old Ryu. It distributes five 6-pixel steps among eleven 3-pixel steps, using the two step sizes observed in MAME. At 60 Hz, that is 236.25 px/s. The 63-pixel arcade samples are 234.82 px/s at 59.637405 Hz. The port’s fixed cadence is within about 0.6% of that representative arcade rate.

For the wider walk sweep, begin a forward hold at frame-end event `2060 + 83i` for `i = 0…25` and continue through +22. Hold Back from +30 through +74 before the next trial. Measure from first forward displacement to 16 display intervals later. Both loadouts averaged 61.73 pixels per interval window (230.09 px/s) and had a median and mode of 63. The port’s fixed pace is about 2.7% faster than the sweep mean.

| Arcade pixels in 16 display intervals | New Ryu trials | Old Ryu trials |
| --- | ---: | ---: |
| 57 | 1 | 1 |
| 60 | 9 | 9 |
| 63 | 16 | 15 |
| 66 | 0 | 1 |

Light Hadoken allows held retreat at 32 ticks (533 ms) after the live projectile for New Ryu and 31 ticks (517 ms) for Old Ryu. Those are the most frequent observed arcade recovery counts for each loadout and land within their measured elapsed ranges. The native fight test also checks the World Turbo 2 far jab timing and walk release. [ROM-backed pacing regression](../../crates/sf-engine/tests/st_ryu_pacing.rs#L123)

The maintainer accepted all three checks for both loadouts after playtesting v1.5.3 and made the permanent decision to omit match-wide Turbo from Era Style fights. The shared simulation runs at 60 Hz, with source pacing tuned per fighter, revision, and loadout. [Spec #138](https://github.com/johnstreetfighter/sf1-port/issues/138) · [Era Styles architecture decision](../adr/0015-era-styles-and-sub-hp-on-the-1987-engine.md#match-pacing)

## Full revision comparison against v1.5.3

The port capture uses `0e336743760689ccdd192cbe4ebce5c4d638c9c4` and the normal Local fight path. It covers all **22 revisions and 39 selectable Ryu loadouts**. The 39 cases include all three Alpha 3 ISMs, all three Super Arts per SFIII revision, every available Ultra selection, and every available V-Skill and V-Trigger combination. Ryu fights Ken with matching revision and loadout, with New Ken for both Super Turbo cases. The port uses country 0 (Japan), Classic controls, default gauge, and normal unpowered state. Faithful uses the Normal Speed DIPs `dfff/ffff`. [Ryu selection matrix](../../crates/sf-engine/src/era.rs#L462) · [Local Versus fixture](../../crates/sf-engine/tests/support/alpha.rs#L79)

Both fighters held Back for 42 ticks, then waited 18, yielding a corrected axis gap of 268 port px. Each measurement began from a copy of that state. The forward walks use 16 elapsed intervals at offsets 4–20, 8–24, and 12–28 after Forward begins. Ryu stays clear of Ken's pushbox and the walls. For jab, tap LP for one sample and hold Back until the retreat move takes over. For Hadoken, press Down, Down-Forward, then Forward+LP on consecutive ticks and hold Back. Faithful uses the longer motion described above. Ken jumps forward on ticks 15–26. A colliding projectile owned by Ryu marks the recovery origin. Ken's life and the hit flags confirm a whiff. The existing Super Turbo integration test also passed on the pinned baseline. [ROM-backed test and position calculation](../../crates/sf-engine/tests/st_ryu_pacing.rs)

The tables read **port → source**. A jab triple gives elapsed intervals to the first live strike box, the count of live samples, and elapsed intervals to the first retreat or the named data boundary. Hadoken gives elapsed intervals from the first colliding projectile sample to the first retreat or the named data boundary. One port tick is 16.667 ms. The arcade intervals use each board's measured display rate. The 3D sources use 60 Hz game data without a live opponent or stage. Their results begin at action onset and end at the charted action boundary. The results establish the direction of the larger timing differences. Exact input-to-movement parity in the 3D games still requires a live run.

### 1987 and 2D arcade revisions

| Port revision and source | Walk, world px per 16 display intervals | Jab, first / active / retreat | LP Hadoken, projectile to retreat |
| --- | ---: | ---: | ---: |
| Faithful SF1, US set 1 `sf`, Japan stage, Normal | 38–40 → 32–40 sampled. **Match** at 40 px per 17 intervals in both. | 4 / 2 / 8 → 4 / 2 / 8. **Match**. | 57 → 57. **Match**. |
| SF1+, Standard | 38–40. **Historical parity inapplicable**. | 3 / 2 / 7. **Historical parity inapplicable**. | 57. **Historical parity inapplicable**. |
| SF2 WW, World `sf2` 910522 | 38–40 → 48. **Difference**. | 4 / 2 / 9 → 3 / 4 / 12. **Difference**. | 58 → 40. **Difference**. |
| SF2 CE, World `sf2ce` 920513 | 38–40 → 48. **Difference**. | 3 / 2 / 8 → 3 / 4 / 12. **Difference**. | 58 → 40. **Difference**. |
| SF2 HF, World `sf2hf` 921209 | 38–40 → 66–69. **Difference**. | 2 / 2 / 6 → 2 / 3 / 8–9. **Difference**. | 55 → 29–30. **Difference**. |
| SF2 Super, World `ssf2` 931005 | 38–40 → 48. **Difference**. | 3 / 2 / 7 → 3 / 4 / 12. **Difference**. | 57 → 40. **Difference**. |
| SF2 ST, New, World `ssf2t` 940223 Turbo 2 | 63 → 57–63, most often 63. **Accepted match**. | 2 / 3 / 9 → 2 / 3 / 9, modal. **Accepted match**. | 32 → 31–32. **Accepted match**. |
| SF2 ST, Old, World `ssf2t` 940223 Turbo 2 | 63 → 57–66, most often 63. **Accepted match**. | 2 / 3 / 9 → 2 / 3 / 9, modal. **Accepted match**. | 31 → 30–31. **Accepted match**. |
| SFZ1, Japan `sfzj` 950727, Turbo On | 38–40 → 57–60. **Difference**. | 3 / 2 / 7 → 2–3 / 3–4 / 10–11. **Difference**. | 57 → 34. **Difference**. |
| SFZ2, Japan `sfz2j` 960430, Turbo On | 38–40 → 63. **Difference**. | 3 / 2 / 7 → 2 / 3 / 9–10. **Difference**. | 57 → 31. **Difference**. |
| SFZ2 Alpha, Japan `sfz2alj` 960805, Turbo On | 38–40 → 63. **Difference**. | 3 / 2 / 7 → 2 / 3 / 9–10. **Difference**. | 57 → 32. **Difference**. |
| SFZ3, Japan `sfz3j` 980904, Turbo 1, A-ISM (Z-ISM) | 38–40 → 57–60. **Difference**. | 3 / 2 / 7 → 2–3 / 3–4 / 11–12. **Difference**. | 57 → 36–37. **Difference**. |
| SFZ3, same, X-ISM | 38–40 → 58.1875–61.25. **Difference**. | 3 / 2 / 7 → 2–3 / 3–4 / 11–12. **Difference**. | 57 → 36–37. **Difference**. |
| SFZ3, same, V-ISM | 38–40 → 59.375–62.5. **Difference**. | 3 / 2 / 7 → 2–3 / 3–4 / 11–12. **Difference**. | 57 → 36–37. **Difference**. |
| SF3 NG, Asia `sfiiina` 970204, SA I–III in port | 38–40 → 52. **Difference**. | 3 / 2 / 7 → 3 / 3 / 11. **Difference**. | 57 → 43. **Difference**. |
| SF3 2I, Asia `sfiii2n` 970930, SA I–III in port | 38–40 → 52. **Difference**. | 3 / 2 / 7 → 4 / 2 / 11. **Difference**. | 57 → 43. **Difference**. |
| SF3 3S, Japan `sfiii3nr1` 990512, SA I–III in port | 38–40 → 52. **Difference**. | 3 / 2 / 7 → 4 / 3 / 11. **Difference**. | 57 → 37. **Difference**. |

SF1+ follows the port's overhaul rules. The one-sample jab begins on press under the overhaul input rules, one tick sooner than Faithful's release-driven jab. Its complete-cycle walk and projectile-to-retreat interval equal the Faithful baseline. [SF1+ domain definition](../../CONTEXT.md#era-style) · [Overhaul input rules](../planning/overhaul-spec.md)

#### Arcade capture recipes

The 2D arcade rows come from ROM-audited MAME 0.289 runs. Inputs below are queued at the named frame-end event and consumed at the following sample. The compact recipes give starting phases. Health and contact flags were checked for whiffs. SF1 runs at 60 Hz. The SFII and Alpha runs use the CPS world axes, 384×224 visible area, and 59.637405 display Hz. SFIII uses 16.16 position at 59.599491 display Hz. The HF results describe MAME, which has a reported timing difference from a physical cabinet. [SF1 driver](https://github.com/mamedev/mame/blob/mame0289/src/mame/capcom/sf.cpp) · [CPS1 driver](https://github.com/mamedev/mame/blob/mame0289/src/mame/capcom/cps1.cpp) · [CPS2 driver](https://github.com/mamedev/mame/blob/mame0289/src/mame/capcom/cps2.cpp) · [CPS3 driver](https://github.com/mamedev/mame/blob/mame0289/src/mame/capcom/cps3.cpp) · [HF timing report](https://mametesters.org/view.php?id=00408)

- **Faithful.** Ryu P1 versus Ken P2 in Japan, Normal Speed and Difficulty. From a fresh boot for each trial, queue P1 Coin at 1450–1461, P2 Coin at 1480–1491, P2 Start at 1560–1571, P1 Start at 1900–1911, and P2 Start at 2000–2011. Hold Right on events 2460–2490 for walk. Samples 2461 and 2478 enclose the 40 px cycle. Queue LP at 2460 and Back at 2461–2495. The LP sample is 2461, live samples are 2465–2466, and retreat starts at 2469. For Hadoken, back both fighters up at 2450–2491. Queue Down at 2510–2511, Down-Forward at 2512, Forward at 2513–2531, LP at 2516–2519, and Back at 2532–2620. Ken holds Up+Left at 2525–2536. The projectile goes live at 2536 and retreat starts at 2593. Six jump phases gave the same result and the standing control took damage.
- **SFII.** Ryu P1 versus Ken P2 in every run. World Warrior uses Blanka's Brazil dock, Champion Edition and Hyper Fighting use Honda's Japanese bathhouse, and Super uses Ken's dock. The CPS1 runs use default Normal difficulty and no speed DIP. Super uses fresh default EEPROM and all emulated DIPs off. Use the walk and jab starts in the earlier recipe, tapping LP for one event and holding Back starting at the next event. Relative to the LP queue event, WW, CE and Super go live at +4…+7 and retreat at +13. HF goes live at +3…+5 and retreats at +9 or +10. For Hadoken, use the earlier motion and jump schedule and hold Back from motion +13 through +80. In the first clean phase, spawn and retreat are +22/+62 (WW), +18/+58 (CE), +16/+45 (HF), and +19/+59 (Super). HF uses 12 independent runs starting the motion at events 2650–2661 and yields eight 29s and four 30s, with health and hit freeze clear. The ST recipe above uses the saved World Turbo 2 setting. [SFII collision read map](https://github.com/peon2/fbneo-training-mode/blob/master/hitboxes/sf2-hitboxes.lua)
- **Alpha.** Ryu P1 versus Ken P2. SFZ1 uses the brick waterfront stage, SFZ2 and SFZ2 Alpha use the snowy Japanese stage, and SFZ3 uses the sunset Japanese stage with Ken in Z-ISM. For the two-player selection, queue coins at 1250/1270, P1/P2 Start at 1300/1340, and both LP at 1400/1470/1540/1610. In SFZ3 select X/V with P1 Up/Down at 1460. Hold Forward for 20 events at 2235/2320/2400, using 2244 for the first SFZ2 Alpha run. For jab, queue one LP at 2515/2592/2668 and hold Back starting on the next event. For Hadoken, start at 2750/2900/3050/3200/3350 on separate runs, queue four Down, four Down-Forward, and three Forward+LP events. Ken jumps on +11…+17 and Ryu holds Back from +11. The table ends at the first backward-walk state. Visible retreat can begin one display later. SFZ3's paired walks were 60/57/60 px (A), 61.25/58.1875/61.25 (X), and 62.5/59.375/62.5 (V). Its paired far LP trials were 2/3/11, 2/4/11, and 3/3/12 in every ISM. [CPS2 collision read map](https://github.com/peon2/fbneo-training-mode/blob/master/hitboxes/cps2-hitboxes.lua)
- **SFIII.** Default board settings. NG is Ryu P1 versus Ken on Ryu's Japanese waterfall stage. 2I uses the same fighters on Ken's dock. 3S is Ryu P2 versus Alex P1 on Ryu's Japanese rooftop. On both sides pulse Coin for 10 events at 30 + 300n and Start for 10 events at 60 + 300n, through event 1599. Confirm with both LP for six events starting at 392 and every 30 events thereafter through event 1499. Use the earlier motion, jump and walk starts, with one LP event for the jab and Back held starting at the next event. For Hadoken, hold Back from event 2112 (NG/2I) or 2008 (3S). Walk runs for 26 events and gives exactly 52 px from 2301 to 2317 or 2201 to 2217. The source runs use Super Art I, Shinku Hadoken, in the unpowered state. The port repeats the three checks with each available Art. [CPS3 collision read map](https://github.com/peon2/fbneo-training-mode/blob/master/hitboxes/cps3-hitboxes.lua)

The local raw captures are under `%TEMP%/sf-ryu-source-check`, `%TEMP%/sfii-ryu-source-check`, `%TEMP%/sf1-alpha-revisions`, `%TEMP%/sfiii-ryu-source-check`, and `%TEMP%/sf1-ryu-all-eras-port`. The port table preserves the three individual walk windows and the exact projectile sample per loadout. The ROMs and emulators are local inputs.

### 3D revisions and spacing calibration

The port's standing Ryu pushbox is 42 px wide in these runs. The 38–40 px walk samples cover 0.905–0.952 pushbox widths in 16 ticks, or 3.39–3.57 widths/s. A source pushbox provides a common spacing measure when its width and walk speed use the same axes. These ratios describe collision-relative movement. Camera framing and the stage dimensions are separate properties.

| Port revision and source evidence | Walk | Jab, first / active / action boundary | LP Hadoken, projectile to action boundary |
| --- | --- | ---: | ---: |
| SF4, Ultra I, original arcade guide | 3.39–3.57 port widths/s. Source value **unverified**. | 3 / 2 / 7 → 3 / 3 / 12. **Difference**. | 57 → ≈33. **Difference**, guide convention. |
| SSF4, Ultras I–II, Super release data | 3.39–3.57 port widths/s. Source ≈0.045 units/tick inferred from Super→AE. Normalized result **unverified**. | 3 / 2 / 7 → 3 / 3 / 12. **Difference**. | 57 → ≈33. **Difference**, frame-table boundary. |
| AE2012, Ultras I–II, 2012 patch | 3.39–3.57 port widths/s. Source 0.045 units/tick. Normalized result **unverified**. | 3 / 2 / 7 → 3 / 3 / 12. **Difference**. | 57 → ≈33. **Difference**, frame-table boundary. |
| USF4, Ultras I–II and Double | 3.39–3.57 port widths/s. Source 0.045 units/tick. Normalized result **unverified**. | 3 / 2 / 7 → 3 / 3 / 12. **Difference**. | 57 → ≈33. **Difference**, frame-table boundary. |
| SF5, V-Skill I / V-Trigger I, Season 1 CPT snapshot | 3.39–3.57 → 5.64 widths/s. **Difference**. | 3 / 2 / 7 → 2 / 2 / 11. **Difference**. | 57 → 33. **Difference**. |
| SF5 AE, V-Skill I / both V-Triggers, v3.002 | 3.39–3.57 → 5.64 widths/s. **Difference**. | 3 / 2 / 7 → 2 / 2 / 11. **Difference**. | 57 → 34. **Difference**. December 2018 AE is 32. |
| SF5 CE, both V-Skills and both V-Triggers, v5.012 | 3.39–3.57 → 5.64 widths/s. **Difference**. | 3 / 2 / 7 → 2 / 2 / 11. **Difference**. | 57 → 32. **Difference**. Also 32 in v5.001 and v7.001. |
| SF6, Classic, December 2024 balance snapshot | 3.39–3.57 → 4.03 widths/s. **Difference**. | 3 / 2 / 7 → 3 / 3 / 13. **Difference**. | 57 → 32. **Difference**. |

- **SFIV source endpoints.** The original arcade guide gives far LP at 4 startup / 3 active / 6 recovery in its inclusive convention. First active is offset 3 and the move occupies 12 ticks. The original guide gives light Hadoken 13 execution and 45 duration, consistent with projectile on offset 12 and 33 elapsed intervals to the action boundary. The Super and AE tables give the same timings. The extracted Super→AE→AE2012→Ultra data show hit and presentation changes to far LP without a timing change, and the USF4 guide lists 33 recovery for normal Hadoken. Capcom's Ultra arcade patch notes document a separate projectile travel speed change. The AE2012 walk value was measured in game coordinates. Exact IV launch walk and equivalent standing pushbox measurements are still needed for walk parity. [SFIV arcade guide, pp. 186–187](https://f.openpdfs.org/315v8meKgYy.pdf#page=186) · [Super and AE reference tables](https://archive.supercombo.gg/t/ryu-general-discussion-frame-data-strategy-combo-etc/129798) · [SFIV revision script differences](https://dantarion.github.io/sf4tools/) · [AE2012 walk extraction](https://archive.supercombo.gg/t/character-walk-speed-breakdown/153892) · [Capcom Ultra v1.04 changes](https://usf4.nesica.net/sozai/2014Ver104_battle_balance.pdf) · [Prima USF4 guide, pp. 475–479](https://www.scribd.com/document/358157078/prima-ultra-street-fighter-iv-bible-eguide-pdf)
- **SFV source endpoints.** SFVSim dumps `004` (Season 1 CPT), `020` (AE launch v3.002), `030` (Kage balance v4.000), and `041` (CE Seth release v5.012) use Ryu actions `16_FORWARD`, `600_5LP`, and `900_FB_L`. Walking moves 0.047 native units/tick. The viewer and box renderer both use 100 px per native unit, giving a 50 px pushbox or 0.50 native units. The jab strikes on offsets 2–3 and has an uninterrupted boundary of 11. Light Hadoken creates its projectile at offset 13 and has boundaries 46, 47, 45, and 45, respectively. Capcom's notes confirm the 46→47 change entering Season 2 and 47→45 change in December 2018. These are ordinary, unpowered moves. [Ryu versioned game scripts](https://sfvsim.com/diff/RYU/) · [SFVSim CE Seth snapshot](https://sfvsim.com/diff/RYU/041/index.html) · [Viewer position scale](https://sfvsim.com/diff/webapp.js) · [Pushbox renderer scale](https://gitlab.com/loic.petit/sfvdiff/-/blob/master/view/render/box.py#L67) · [Capcom Season 2 changes](https://game.capcom.com/cfn/sfv/adjust/201701/ryu?lang=en) · [Capcom December 2018 changes](https://game.capcom.com/cfn/sfv/adjust/201812/ryu?lang=en) · [Capcom CE v5.012 release](https://game.capcom.com/cfn/sfv/systemfault/132931?lang=en)
- **SF6 source endpoints.** December 2024 Ryu actions 10, 600, and 900 give 4.6999969 native units/tick, strike offsets 3–5 with margin 13, and projectile creation at 15 with margin 47. The companion extraction resolves standing and walking pushbox 1 to common rectangle group 05, index 001. Its horizontal radius is 35 native units, so the walk covers 4.70 × 60 ÷ 70 ≈ 4.03 widths/s. These are game-data boundaries for Classic, unpowered Ryu. [MMDK December 2024 snapshot](https://github.com/alphazolam/MMDK/blob/831564f1993926a9fff834108f24767748d40c06/MMDK/reframework/data/MMDK/PlayerData/Ryu/Ryu%20moves_dict.json) · [December balance move data](https://github.com/PsychoDrive-Dictator/psychodrive/blob/4bf6e9e7d67ebebaba126e3f1ad2c1c62449f27c/data/chars/ryu/ryu25_moves.json) · [Common rectangles](https://github.com/PsychoDrive-Dictator/psychodrive/blob/4bf6e9e7d67ebebaba126e3f1ad2c1c62449f27c/data/chars/common/common24_rects.json) · [Rectangle lookup and radius](https://github.com/PsychoDrive-Dictator/psychodrive/blob/4bf6e9e7d67ebebaba126e3f1ad2c1c62449f27c/chara.cpp#L283)

### Pacing findings for #138

Across every historical style outside Faithful and the accepted Super Turbo builds, the port returns control sooner after jab and later after light Hadoken. For example, HF retreat comes 6 ticks after LP in the port and 8–9 displays in the arcade. Its light Hadoken leaves Ryu committed for 55 port ticks and 29–30 arcade displays. The two actions require timing changes in opposite directions. The three direct Zero 3 captures also establish separate walk targets for A-, X-, and V-ISM even though their two whiff timings agree at Turbo 1. [Spec #138](https://github.com/johnstreetfighter/sf1-port/issues/138)

The captures establish movement and timing targets at the recorded arcade settings. The four IV walk rows remain unverified in a shared distance scale. The IV–6 action data establish timing differences and remain subject to the direct live-game checks tracked by the spec. The audit adds evidence for subsequent revision-level corrections. [SFIV direct evidence #148](https://github.com/johnstreetfighter/sf1-port/issues/148) · [SFV direct evidence #149](https://github.com/johnstreetfighter/sf1-port/issues/149) · [SF6 direct evidence #150](https://github.com/johnstreetfighter/sf1-port/issues/150)

### Targets applied in v1.5.5

Ryu's forward walk uses whole port pixels over 16 ticks. The arcade targets use representative captured phases at the recorded speeds, and the SFV and SF6 targets use the calibrated pushbox ratios. Alpha 3 uses 60, 61, and 62 px for A-, X-, and V-ISM. SFV uses 63 px and SF6 uses 45 px. All four SFIV styles retain the original port walk pending comparable source distances. [Forward walk targets](../../crates/sf-engine/src/fighter/ryu_walk_pacing.rs)

The jab timing uses representative arcade samples or the named 3D move boundary. A near-range check at 40 source px repeated the Japanese Alpha jab input. SFZ1, SFZ2, and SFZ2 Alpha selected the same animation as the 150–160 px trials, while Alpha 3 selected its close jab. The port applies the standing jab timings at all spacing in those first three Alpha styles and in V and 6. The proximity-normal styles use the far timing beyond the port's 64 px cutoff. Light Hadoken targets the tested unpowered whiff. The port's existing hit and block transitions still apply when the projectile connects. [Jab move streams](../../crates/sf-engine/src/data_bank.rs) · [Local Versus checks](../../crates/sf-engine/tests/st_ryu_pacing.rs)

### SFIV forward walk in v1.5.6

Ryu's close/far standing LP transition provides a common distance. The SFIV game data uses a 1.2-unit position restriction for both the close `5LP` and far `5LPF`. The port compares the fighters' corrected horizontal axes at its 64 px jab cutoff. In all four SFIV revisions the `FORWARD` script sets velocity to 0.045 native units per tick, equivalent to 0.72 units per 16 ticks. Scaling by `64 / 1.2` gives 38.4 port px per 16 ticks, or 144 px/s at 60 Hz. The port rounds to 38 px per 16 ticks (142.5 px/s) and distributes it across the walk cycle. This measures how quickly Ryu covers a distance that matters for his jab. SFIV standing collision width was not established. [Port jab cutoff](../../crates/sf-engine/src/fighter.rs#L6371) · [Walk target](../../crates/sf-engine/src/fighter/ryu_walk_pacing.rs)

| Revision | Reference and verification | Native units/tick | Jab cutoff, native units | Port px/16 ticks |
| --- | --- | ---: | ---: | ---: |
| SFIV | 2008 Taito Type X2 arcade v1.0 STD and Capcom's June 2009 PC benchmark v1.0.0.1, extracted Ryu BAC and BCM | 0.045 | 1.2 | 38 |
| SSFIV | Super ruleset from the PC installation, checked through the extracted move and script diffs through Ultra | 0.045 | 1.2 | 38 |
| AE2012 | PC AE2012 regulation, published first-hand walk extraction and extracted move and script diffs through Ultra | 0.045 | 1.2 | 38 |
| USFIV | PC v1.05 `ae2_111` regulation, extracted unmodified Ryu BAC and BCM | 0.045 | 1.2 | 38 |

For the original PC data, extract Capcom's benchmark installer as an archive and read `Battle/Chara/RYU/RYU.cmn.emz`. Inflate raw DEFLATE from byte 16, then extract `RYU.bac` and `RYU.bcm` from the EMB directory. In the BAC, action 16 sets horizontal velocity in its frame 0–1 physics command. The float is at `0x3060`. In the BCM, read the restriction floats at `0x5b4` (`5LP`, close) and `0x64c` (`5LPF`, far). The two files' SHA-256 values are `63565b9437c970332dcc256e2c274ea9a7a4d36e731357d5957898f5abd78e10` and `e1611b85c199575ab13ceeefd92d98455359b5c6e787574c95933aed74e05e24`. [Capcom benchmark distribution](https://www.4gamer.net/games/049/G004954/20090612078/) · [Installer archive extractor](https://github.com/pawstas80/Unpacker)

For Ultra, read `patch_ae2_tu3/battle/regulation/ae2_111/RYU/RYU.bac` and the adjacent BCM from the unmodified backup. UltraDiff's decoder yields `FORWARD` → `PHYSICS` → `VelocityX` and the `5LP` and `5LPF` `PositionRestrictionDistance` fields. Both endpoint builds give binary32 values `0.04500000178813934` and `1.2000000476837158`. The eight Ryu diffs from Super through Ultra v1.05 have no `FORWARD` script change or close/far LP command restriction change. The original Super and AE2012 values are supported by that extracted chain, with an independent first-hand walk extraction for AE2012. This pass did not run a separate live fight in each of those builds. [Unmodified Ultra files](https://github.com/EternalStar/USF4-Bugfix/tree/670d47e4311a8952e3d0b35039c4bc6eccdd746b) · [UltraDiff decoder and revision mapping](https://github.com/dantarion/sf4tools/tree/e5c17132f3b6e63f66150c3a347a9e8f6e0d73a4/SFUltraDiff/src) · [Pinned Ryu script diffs](https://github.com/dantarion/sf4tools/tree/e07764262003f3aa1feff1a0a5a66fc3c14efb43/characters/RYU) · [AE2012 walk extraction](https://archive.supercombo.gg/t/character-walk-speed-breakdown/153892)

#### 2008 arcade versus the PC benchmark

The Taito Type X2 v1.0 STD arcade source is the `Street Fighter IV (2008).zip` preservation set. Its `launcher.exe` has MD5 `e47d7c83e526dbb1986173f197801b99`, the documented v1.0 STD executable fingerprint. CRC-32 of its first `0x400` bytes is `ddd19e50`, matching OpenParrot's original Street Fighter IV detector. Its PE timestamp is 2008-06-03 02:15:20 UTC and its embedded debug path ends in `SF4_Std\Bin\TypeX\game.pdb`. Read the executable for identification and extract `Battle/Chara/RYU/RYU.cmn.emz` from the same archive. The arcade BAC and BCM SHA-256 values are `c1364dd7be172188cb5b04dcad10563c2877d1ffbdcb7d29d416b5c8c88710c2` and `4c0263e97948f22359b3f5af672963ff32909d7ff984dd54bf9e4502cd10fa4d`. [Preservation set metadata](https://archive.org/metadata/Yori-Arcade-Arcade_Games_-_TAiTO_Type_X2-RomVault) · [v1.0 STD identification](https://jconfig-universe.fandom.com/wiki/Street_Fighter_IV) · [OpenParrot detection source](https://github.com/teknogods/OpenParrot/blob/fc8ba632e1f67c5c340ff88f30efca743e8d38f5/OpenParrot/src/Utility/GameDetect.cpp#L322)

| Field | 2008 arcade v1.0 STD | 2009 PC benchmark v1.0.0.1 |
| --- | ---: | ---: |
| `FORWARD` action 16 byte range, end exclusive | `0x2ef0`–`0x30b0` | `0x2fa0`–`0x3160` |
| Horizontal velocity field | `0x2fb0` | `0x3060` |
| Horizontal velocity, native units/tick | 0.04500000178813934 | 0.04500000178813934 |
| Close/far `5LP` / `5LPF` restriction fields | `0x5b4` / `0x63c` | `0x5b4` / `0x64c` |
| Jab spacing, native units | 1.2000000476837158 | 1.2000000476837158 |

Read the BAC's absolute script and name tables using the pointers at `0x0c` and `0x10`. Entry 16 is named `FORWARD`. At script offset `+0x16`, the little-endian command group count precedes the group table offset. Physics group 5 assigns horizontal velocity over `[0, 1)` with zero acceleration. Compare action 16 through the start of action 17. All 448 bytes match and hash to `59387d6f91b69b596841759e6ff86ce105678a7e627ccc8ba65b079af660b275`. The BCM close and far mode values are 2 and 1 in each build. Their identical spacing and velocity retain the 38 px target in the original SFIV Era Style. This comparison reads game data. A live cabinet input-to-movement trace remains outside its coverage.
