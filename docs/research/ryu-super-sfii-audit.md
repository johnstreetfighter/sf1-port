# Super Street Fighter II Ryu moveset audit

The source is the arcade parent set **Super Street Fighter II: The New Challengers, World 931005** (`ssf2`) in MAME 0.289. `-verifyroms` returned `romset ssf2 is good`. Ryu faces Ken in a two-player match from the default EEPROM. Port comparisons use SF2 Super, Standard, on both sides, with Definitive Overhaul and the Classic controls. The [pacing measurements](2026-11-13-ryu-era-pacing.md#sfii-and-sfiii-arcade-runs) cover forward walk, far jab whiff and LP Hadoken whiff separately. [MAME ROM manifest](https://github.com/mamedev/mame/blob/mame0289/src/mame/capcom/cps2.cpp#L7621)

## Input inventory

Arcade animation pointers identify distinct move streams within this build. Shared pointers across two inputs show they selected the same stream in the tested setup. The port uses the 1987-derived standing, crouching and airborne dispatch with its SF2 rules and revision-specific adjustments. [Port normal and jump dispatch](../../crates/sf-engine/src/fighter.rs#L2194) · [Port revision adjustment](../../crates/sf-engine/src/era_fight.rs#L226)

| Input family | Arcade World 931005 | Port SF2 Super |
| --- | --- | --- |
| Close standing LP, MP, HP, LK, MK, HK | All six verified. At 32 px, each connected. HK connected twice, the second hit into the first hit’s stun. | All six button inputs are available. The 1987-derived dispatch and reaction data need comparison for each proximity band. |
| Far standing LP, MP, HP, LK, MK, HK | All six verified at 90 px. Each selected a different stream from the corresponding 32 px input. At 78 px, HP and HK still selected their 32 px stream, while LP, MP, LK and MK selected their 90 px stream. | All six button inputs are available. The far LP pacing check already has a revision-specific target. The other proximity thresholds and corresponding streams require their own checks. |
| Crouching LP, MP, HP, LK, MK, HK | All six verified at 32 px. HK was the only launcher in this grounded set. | All six are available through the shared crouching dispatch. |
| Neutral-jump LP, MP, HP, LK, MK, HK | Six distinct streams, all different from their directional-jump counterparts. LP and LK connected at the tested height against a grounded Ken. | Six inputs are available through the jump dispatch. Compare the move streams and hitboxes for each jump direction. |
| Forward- and backward-jump LP, MP, HP, LK, MK, HK | All twelve input combinations verified. The forward and backward version of each button entered the same attack stream. These samples establish activation and stream identity. | The six buttons can be used on directional jumps. The jump-normal dispatch and individual contact data require comparison. |
| Forward + MP, HP, MK, HK outside throw range | At 90 px each selected its corresponding neutral-direction far attack stream. The `6MP` Collarbone Breaker and `6HP` Solar Plexus Smasher inputs yielded no distinct move in this fixture. | The SF2 Super roster has no separate Ryu entry for either named command normal. |
| Ground throws | At 32 px, forward or back + MP/HP selected the shoulder throw. Forward or back + MK/HK selected a separate kick throw. Both forward throws dealt 32 of 144 HP. The tested escape attempt did not change the shoulder throw. | The native direction + HP or HK inputs share a shoulder throw path. Ordinary MP and MK remain normal inputs. Throw tech uses the SF2 ruleset switch. [Port throw selection](../../crates/sf-engine/src/fighter.rs#L8516) · [Port throw rules](../../crates/sf-engine/src/combo/throw.rs#L55) |
| Hadoken (`236P`), Red Hadoken (`41236P`) | LP Hadoken spawned and hit once. HP Red Hadoken used a different projectile stream, hit once at both distances, launched at 36 px and hit grounded at 180 px. | The base Hadoken and a Super-specific red projectile entry are present. The port’s crimson projectile budget is three hits. [Port move entry](../../crates/sf-engine/src/moves.rs#L1439) · [Port projectile budget](../../crates/sf-engine/src/era_fight.rs#L940) |
| Shoryuken (`623P`) | LP, MP and HP each selected a distinct stream and launched in the 32 px fixture. | Available through the base Shoryuken recognizer. [Port move list](../../crates/sf-engine/src/moves.rs#L1000) |
| Ground and air Tatsumaki (`214K`) | LK ground and air versions selected separate special streams. The ground version launched Ken from 55 px. The air version activated from a forward jump at 76 px and did no damage. | The ground version uses the base special. Air activation goes through the SF2 air-special input path. [Port air-special gate](../../crates/sf-engine/src/era_fight/sf3.rs#L564) |
| Super Combo, EX and ST-style throw softening | Super SFII has no Super Combo or EX meter. The double-quarter-circle LP trial produced an ordinary Shoryuken stream with these input timings. The throw-response trial is listed below. | No Super Combo or EX entry for this style. The SF2 throw-tech switch can permit escapes. [Port SF2 move list](../../crates/sf-engine/src/moves.rs#L1432) |

Capcom's original console documentation corroborates the special-move inputs. The timing and outcomes here come from the arcade parent. [Capcom Town manual](https://captown.capcom.com/en/classic_games/31)

## Arcade contact measurements

Use the first sample with a life decrement or a new defender freeze counter as contact +0. The one-byte counters at player `+0x47` load 14 on the sampled direct normal contacts. Ken's counter reaches zero at contact +16. His first grounded displacement is visible at +17. The clock at `0xff801d` advanced once per display sample in these trials. The display frequency is 59.637405 Hz.

| Ryu input | First contact sample | Freeze Ryu / Ken at contact | Ken reaction after freeze | Ken displacement |
| --- | ---: | ---: | --- | ---: |
| Close LP | 2904 | 14 / 14 | Grounded, neutral 12 ticks after freeze reaches zero | +30 px |
| Close MP | 2905 | 14 / 14 | Grounded, neutral after 17 ticks | +54 px |
| Close HP | 2905 | 14 / 14 | Grounded, neutral after 21 ticks | +78 px |
| Close LK | 2907 | 14 / 14 | Grounded, neutral after 12 ticks | +30 px |
| Close MK | 2905 | 14 / 14 | Grounded, neutral after 17 ticks | +54 px |
| Close HK | 2909 and 2931 | 14 / 14 on both | Two grounded hits, neutral 21 ticks after the second freeze reaches zero | +123 px across both hits |
| Crouching LP or LK | 2904 | 14 / 14 | Grounded, neutral after 12 ticks | +30 px |
| Crouching MP or MK | 2905 | 14 / 14 | Grounded, neutral after 17 ticks | +54 px |
| Crouching HP | 2905 | 14 / 14 | Grounded, neutral after 21 ticks | +78 px |
| Crouching HK | 2905 | 14 / 14 | Peak Y 95 from floor 40. Landed at 2962 and returned to neutral at 3001 | +80 px |
| Crouching LK guard | 2904 | 14 / 14 | Left guard reaction at 2932. A new LP on that sample activated | +30 px |
| Crouching MK guard | 2905 | 14 / 14 | Left guard reaction at 2938. The next sample accepted a new crouching LP | +54 px |
| Crouching HK guard | 2905 | 14 / 14 | Left guard reaction at 2942. The next sample accepted a new crouching LP | +78 px |
| Neutral-jump LP or LK | 2944 | 14 / 14 | Grounded, neutral after 12 ticks | +30 px |
| LP Hadoken at 128 px | 2915 | 0 / 14 | Grounded, neutral after 42 ticks | +78 px |
| HP Red Hadoken at 36 px | 2913 | 0 / 14 | One hit. Peak Y 97, landed at 3032 and neutral at 3071 | +126 px |
| HP Red Hadoken at 180 px | 2927 | 0 / 14 | One grounded hit, neutral after 42 ticks | +78 px |

For throws, the shoulder and kick captures both began at 2901. The forward shoulder released Ken at 2945, dealt damage at 2980, and returned him to neutral at 3062. The forward kick released at 2939, dealt damage at 2972, and returned him to neutral at 3053. In the backward cases, the release and damage samples matched their forward counterparts. Ken reached the left boundary and returned to neutral at 3050 for the shoulder and 3041 for the kick. `Forward+MP` from Ken six samples after the initial HP throw command had the same full-damage result as the unanswered shoulder throw.

### Two sampled normal-to-special routes

At 32 px, `2LK → LP Hadoken` and `2MK → LP Hadoken` both produced a second hit while Ken remained in hit reaction and the combo counter reached 1. For `2LK`, the tested LP command offsets +4 and +7…+17 worked. At +1…+3, Hadoken displaced the opener. At +5…+6 and +18…+20, the opener connected and the follow-up never spawned. For `2MK`, +7…+14 and +17…+19 worked. At +1…+4, the opener was displaced. At +5…+6, +15…+16 and +20, the follow-up never spawned. The exact command includes the one-sample diagonal and two-sample LP hold below. The gaps describe that recipe's input recognition, rather than a continuous cancel window.

## Reproduce the arcade measurements

Start a fresh MAME working directory with `ssf2 -video none -sound none -nothrottle -skip_gameinfo -autoboot_script <probe.lua>`. Run the callback from `emu.register_frame_done`, counting its first invocation as sample 1. Each listed input is held through its inclusive end sample and consumed starting on the next sample. Use active-low pad bits at `0x804000` (directions and punches), `0x804010` (kicks), and `0x804020` (coin, start, P2 HK). MAME's normal input fields also carry the coin and start commands. For character selection, apply coin P1 at 1250, coin P2 at 1270, start P1 at 1300, start P2 at 1340, coin P2 again at 1630 and 1870, start P2 again at 1660 and 1890, and both LP at 1400, 1470, 1540, 1800, 1950, 2030, 2100 and 2170. Hold each of these boot inputs for six samples. Verify character bytes 0 and 4 and both life words 144. [MAME CPS2 input map](https://github.com/mamedev/mame/blob/mame0289/src/mame/capcom/cps2.cpp#L1190)

At sample 2860, set Ryu's integer world X to 552 and Ken's to `552 + spacing`, and clear both X fractions. This is the only gameplay RAM write in the capture. Leave Y and the game clock alone. Ryu faces right. Player object bases are `0xff83ce` and `0xff87ce`. Read X and fraction at `+0x06/+0x08`, Y at `+0x0a`, life at `+0x2a`, state and substate at `+0x03/+0x04`, animation pointer at `+0x1a`, freeze at `+0x47` and combo count at `+0x1f4`. The projectile scan checks eight slots from `0xff96a2`, stride `0xc0`, with object signature `0x0101`. At 2900 the round state word `0xff8008` is `0x000a`. The same fixture sampled at 2500 was still in its round intro.

| Case | Spacing and inputs queued at frame end |
| --- | --- |
| Standing normals | 32, 78 and 90 px. Press one button at 2900. For forward command tests use 90 px and add Forward on 2900. |
| Crouching normals and guard | 32 px. Ryu Down 2898…2904 and the attack button at 2900. For guard, Ken Down-Back 2897…2930. For guard readiness, add a one-sample LP so its first consumed sample falls at the tested reaction boundary. |
| Neutral and directional jumping normals | 32 px for neutral and 90 px for directional. Jump direction 2900…2902 and attack button 2909. |
| Throws | 32 px. Ryu presses the tested throw direction plus button at 2900. In the response trial Ken presses forward + MP at 2906. |
| LP Hadoken | 128 px. `2` 2892…2895, `3` 2896…2899, `6+LP` 2900…2904. |
| HP Red Hadoken | 36 or 180 px. `4` 2884…2887, `1` 2888…2891, `2` 2892…2895, `3` 2896…2899, `6+HP` 2900…2904. |
| Shoryuken | 32 px. `6` 2884…2887, `2` 2888…2893, `3` 2894…2897, `3+P` 2898…2901. |
| Ground and air LK Tatsumaki | 55 and 76 px respectively. For the air version add `9` 2887…2889. `2` 2892…2895, `1` 2896…2899, `4+LK` 2900…2904. |
| Crouching LK or MK into LP Hadoken | 32 px. Begin holding `2` at 2874 through `max(N−2, 2899)`. Press `2+LK` or `2+MK` at 2900, `3` at `N−1`, and `6+LP` at `N…N+1`. Set `N = 2900 + offset` and sweep offsets 1…20. |

Raw traces and input files are in `%TEMP%/ssf2-ryu-audit` on the capture machine. The `live-*` directories are the valid live-round captures. The copied merged `ssf2.zip` archive has SHA-256 `2b5edb6781a121f67e2258d88681d9b18b0b0558ce812e14d33ed8b8ae77630a`. Its `qsound_hle.zip` dependency has SHA-256 `55feb9b70dacb7364f0310ed16cc9281b5b0a31318b9e2e0587ae06d15f95356`.

## Matched port probes

The native probe used version 1.5.7 from the working tree based on `0e336743760689ccdd192cbe4ebce5c4d638c9c4`, compiled before the Super Turbo contact changes from this audit. The `sf_engine` library had SHA-256 `35e1f59191a3fff92422d11ca35616c1da9b1b24f56c375b5aa677e70e83e5ea`. The executable `%TEMP%/ssf2-ryu-audit/port-close.exe` had SHA-256 `28f8500ac22cd813b7312dbf6fbba6155de403b01ceb71b68fd9f41c1c0ae08c`. The ROM-backed probe entered Local Versus through the Style Cards, stepped 94 neutral ticks in the fight loop, walked Ken left to a 32-pixel center gap, and settled for 16 neutral ticks. Each case cloned that starting state. The input schedule maps arcade sample 2900 to port tick 20. It reads both fighters with `fight_read()` and logs world X using camera X plus collision center minus the pose offset. Reaction length below is from first contact to the first unlocked tick. [Local Versus fixture](../../crates/sf-engine/tests/support/alpha.rs#L427) · [Fight telemetry](../../crates/sf-engine/src/fight_reader.rs#L79)

| Input | Arcade at 32 px | Port at 32 px |
| --- | --- | --- |
| Close LP | Grounded, +30 px | Grounded `0x30`, 9 ticks, +16 px |
| Close MP | Grounded, +54 px | Grounded `0x32`, 13 ticks, +32 px |
| Close HP | Grounded, +78 px | Airborne `0x33`, 43 ticks, +133 px |
| Close LK | Grounded, +30 px | Grounded `0x31`, 18 ticks, +24 px |
| Close MK | Grounded, +54 px | Airborne `0x33`, 44 ticks, +133 px |
| Close HK | Two grounded hits, +123 px | One airborne hit `0x34`, 27 ticks, +160 px |
| Crouching LP | Grounded, +30 px | Grounded `0x30`, 17 ticks, +24 px |
| Crouching MP | Grounded, +54 px | Grounded `0x31`, 15 ticks, +24 px |
| Crouching HP | Grounded, +78 px | Airborne `0x33`, 44 ticks, +133 px |
| Crouching LK | Grounded, +30 px | Grounded `0x30`, 18 ticks, +24 px |
| Crouching MK | Grounded, +54 px | Airborne `0x33`, 44 ticks, +133 px |
| Crouching HK | Launch, 96 ticks from contact to neutral, +80 px | Airborne `0x33`, 44 ticks, +127 px |
| Crouching LK guard | Guard reaction until contact +28, +30 px | Crouching guard, 12 ticks, 0 px |
| Crouching MK guard | Guard reaction until contact +33, +54 px | Crouching guard, 11 ticks, 0 px |
| Crouching HK guard | Guard reaction until contact +37, +78 px | Crouching guard, 7 ticks, 0 px |

In these clean normal port probes, Ken’s displacement started at contact or the next tick. The arcade started moving at contact +17 and gave the attacker and defender 14 ticks of freeze. The two clocks and reaction anchors are recorded above. Ken’s 32 px center gap is in each game’s native world units, with different hurtbox geometry.

The port also reproduced the HP/HK shared throw path and MP/MK normal dispatch directly. On the same neutral-jump MP and HP timing that missed in the arcade, both port buttons selected jump attack `0x20` and launched Ken. This establishes a contact difference for the recorded timing and spacing.

On `2LK → LP Hadoken`, port offsets +3…+12 produced a true combo. Offsets +1 and +2 left only the opener. At +12, Ken took the first hit on port tick 26, Hadoken began on tick 32, and the projectile hit into stun on tick 43. The next offset, +13, started Hadoken at 33 and hit on 44 after Ken had recovered. The same +12 and +13 command offsets both produced a combo in the arcade. Its latest tested success, +17, pressed LP after sample 2917 and landed the projectile at 2931 while Ken was still reacting. The next tested offset, +18, left only the opener. All offsets in this paragraph use the opener button input as origin.

On `2MK → LP Hadoken`, the port's opener launches while the arcade opener leaves Ken grounded. At port offsets +3…+7, the opener launched on tick 26, Hadoken started at 27 and the projectile spawned at 38. It passed under Ken during his airborne reaction. At tested port offsets +8 and +12…+20, the projectile hit on ticks 70…73, after the launch reaction. Those trials recorded no combo. The arcade combo succeeds at +7…+14 and +17…+19. Its latest tested success, +19, pressed LP after sample 2919 and landed the projectile at 2931. The +20 trial left only the arcade opener, while the port projectile hit late at tick 73. The traces record Hadoken activation separately from a successful second contact.

## Open measurements

- Exact close/far thresholds for each standing button, airborne contact timing at matched heights and spacing, jump-normal cancel routes, and guard and counterhit data outside the sampled fixtures.
- Every strength of regular and red Hadoken and both Tatsumaki versions, including the exact red knockdown threshold, collision with other projectiles, corner cases and air follow-ups. The Shoryuken activation tests establish three strength-specific streams and grounded launch. Their complete hitbox and invulnerability timelines are unmeasured.
- Full throw response inputs and timing, air-throw eligibility, corner recoil, matched midscreen sweep and knockdown distances, and wakeup action boundaries.
- More combo families, including chains, links, airborne follow-ups and late LP Hadoken inputs beyond the sampled command window. Damage differed between some exact replays that sampled different input phases. The conclusions use reaction, contact count and timing where these were stable.
