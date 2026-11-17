# Ken moveset audit in World Super Turbo 2

Source and initial port comparison for [issue #146](https://github.com/johnstreetfighter/sf1-port/issues/146). New Ryu is P1 and New or Old Ken is P2 in **Super Street Fighter II Turbo, World 940223** (`ssf2t`), with World Turbo 2 selected in MAME 0.289. The first table identifies implementation paths. The port probes below check selected contacts with both loadouts.

## Directly verified source differences

| Input and setup | New Ken | Old Ken | Port v1.5.8 baseline |
| --- | --- | --- | --- |
| Forward or back + MP/HP at 32 px | Shoulder throw, both directions. | Shoulder throw, both directions. | Direction + HP uses the shared shoulder throw. MP uses the normal dispatch. |
| Forward or back + MK at 32 px | Knee hold. The forward sample dealt five packets, `26, 4, 4, 4, 4` HP, without extra button mashing. | Same Hell Wheel stream as his HK throw. | MK uses the normal dispatch. No Ken-specific knee hold appears in this path. |
| Forward or back + HK at 32 px | Hell Wheel, distinct from the knee hold and shoulder throw. | Hell Wheel, distinct from the shoulder throw. | Direction + HK takes the same shoulder throw path as HP. |
| Jump toward airborne Ryu and press Forward or Back + MK or HK | Air Throw once the capture conditions are met. The initial early HK recipe gives an ordinary jump kick for 26 HP. A later press at close range captures and deals 32 HP in these fixtures. | The same recipes give ordinary jump attacks. | The dedicated air-throw handler is gated to Evil Ryu's Alpha 3 profile. |
| `236K`, `632K`, and `41236K` at 55 px | Each motion started a distinct special stream with LK, MK and HK. Holding HK changed the follow-up stream in all three. | These inputs started normal attacks or proximity throws in the tested fixture. | Ken has no corresponding SF2 extra-special recognizer. The named Inazuma Kakato Wari entry belongs to the Alpha move set. |
| `623LP`, `623MP`, `623HP` at 55 px | Three distinct Shoryuken streams. LP and MP each hit once. HP hit twice, with its first hit on the first sampled special frame. | Three distinct streams. LP and MP each hit once. HP hit twice, with its first hit two display samples after the first sampled special frame. | The port has a native Shoryuken and a Ken-specific two-hit ceiling in SF2 ST. The separate strength and reaction behavior needs a matched probe. |
| Air `214HK` | Air Tatsumaki activated from a forward jump at 60 px. | Air Tatsumaki activated in the same fixture. | SF2 shotos have an air Tatsumaki input path. These arcade trials verified activation. Neither dealt damage. |
| Ground `6+MP/HP/MK/HK` at 90 px | All four selected their corresponding neutral-direction far attack stream. | Same result. | No separate forward-normal entry appears in the SF2 ST move list. |

The later port comparison covers the v1.5.21 command kicks and the v1.5.22 walk and throws. [Command kick follow-up](ken-super-turbo-command-kicks.md) · [Throw follow-up](ken-super-turbo-throws.md) · [Walk follow-up](#ground-walk-follow-up)

Capcom’s arcade retrospective identifies the first hit of Ken’s heavy Shoryuken as forcing a crouched reaction in Super and Super Turbo. The later game-tick sweep measured three contacts for both loadouts at close range and on crouch guard at 55 px. It includes separate LP, MP and HP flight paths and a one-tick onset difference for the heavy opening strike. [Capcom’s SFII combo seminar](https://game.capcom.com/cfn/sfv/column/131578?lang=en) · [Shoryuken follow-up](ken-super-turbo-shoryuken.md)

### Representative trace rows

Each cell lists the first observed move stream pointer, then damage samples. All timestamps are MAME display samples. The game can advance more than one game tick between displays. A combo means the defender's counter increased during the second contact.

| Setup | New Ken | Old Ken |
| --- | --- | --- |
| `236LK` | `1321b0`, sample 2121. Damage at 2126 and 2139 (combo). | `1c63d2`, 2122. Damage at 2126. |
| `632LK` | `132398`, 2121. Damage at 2133 and 2146 (combo). | `1c6812`, 2122. Damage at 2123. |
| `41236LK` | `132598`, 2121. Damage at 2130. | `1c63d2`, 2122. Damage at 2126. |
| Held `236HK` | Held branch `1322bc` at 2139. Damage at 2126 and 2153 (combo). | Proximity throw `1c7c9a`, 2121. Damage at 2186. |
| Forward + MK | `131308`, 2121. Damage at 2132, 2158, 2184, 2211 and 2236. | `1c7c9a`, 2121. Damage at 2182. |
| Forward + HK | `133618`, 2121. Damage at 2181. | `1c7c9a`, 2121. Damage at 2182. |
| `623HP` | `132ad8`, 2121. Damage at 2121 and 2134 (combo). | `1c72a2`, 2121. Damage at 2123 and 2135 (combo). |

The Old Ken `236MK/HK` and `41236MK/HK` fallback started a proximity throw at the tested gap. The final input holds Forward and the preceding walk puts Ken in range. The corresponding Old LK cases start an ordinary standing kick. These traces identify which move the command selected under this setup.

### Normal inventory

Each cell is the first attack animation pointer in hex, given as New / Old. Each row lists the six button inputs even when the fixture whiffed. New standing HK selected `131494` from both 32 and 90 px. Every other corresponding close and far pair in this set had distinct pointers. The six neutral and six directional jump attacks selected distinct streams in both loadouts.

| Setup | LP | MP | HP | LK | MK | HK |
| --- | --- | --- | --- | --- | --- | --- |
| Standing 32 px | `130dec/1c5ff6` | `130ecc/1c60be` | `13100c/1c6216` | `131134/1c633e` | `13128c/1c644e` | `131494/1c655e` |
| Standing 90 px | `130e50/1c605a` | `130f78/1c6182` | `1310a0/1c62aa` | `1311c8/1c63d2` | `131384/1c64e2` | `131494/1c65f2` |
| Crouching 32 px | `131588/1c6686` | `131604/1c66ea` | `131680/1c677e` | `131714/1c6812` | `131790/1c6876` | `13180c/1c690a` |
| Neutral jump 32 px | `1318a0/1c699e` | `1318d4/1c69d2` | `131980/1c6a7e` | `131a2c/1c6b2a` | `131aa8/1c6ba6` | `131b24/1c6c22` |
| Forward jump 78 px | `131bd0/1c6cce` | `131c04/1c6d02` | `131cb0/1c6dae` | `131d5c/1c6e5a` | `131e6c/1c6ebe` | `131f48/1c6f6a` |

At 32 px, all standing and crouching probes made contact. In the 90-pixel trials, New far LK made contact while Old far LK whiffed. LP and MK whiffed in both loadouts. At the sampled jump timing, neutral LP and LK hit grounded Ryu, and all six directional jump trials missed him.

### Projectile and ground Tatsumaki controls

The 55-pixel LP/MP/HP Hadoken probes all activated, hit once, and did 16/17/19 HP respectively in the matching phase. All three New strengths first hit on display 2131 and the Old strengths hit on 2130. These display samples leave the exact game-tick startup and projectile speed open.

#### Game-tick Hadoken strength sweep

The follow-up writes pin all three strengths to the same per-loadout release clocks. From Ken's first state-12 write, New releases at +13 and first writes idle at +53. Old releases at +12 and writes idle at +52. These clocks agree in both input phases for standing targets, crouch guard, a neutral jump and a forced whiff. Ken's release-to-idle interval is 40 game ticks in both loadouts. On the 171-pixel whiff route, per-update projectile travel is 3/4/5 px for LP/MP/HP in both loadouts. The 104-pixel fixture gives an immediate release-tick hit for 16/17/19 HP or 4 chip. Its grounded hit and guard freeze New Ryu for 14 ticks while Ken's clock runs. Those damage amounts belong to the sampled setup and phases.

Set New Ryu to X552 and Ken to X656 or X723 at display 2070. At `N=2120` or `2121`, queue Ken Down on `N−8…N−5`, Down-Forward on `N−4…N−1`, then Forward and the tested punch on `N…N+2`. Hold Ryu Down-Back starting `N−2` or queue his neutral jump starting `N−8` for the respective controls. The forced whiff pins Ryu Y600 through the full shot. Tap the special-state transition, projectile object activation, life and freeze, and sample the projectile integer and fractional X while it advances. The archive stacks 48 recipes. [Hadoken writes](ken-super-turbo-hadoken-writes.tsv)

In all 24 grounded cases New Ryu's freeze reaches zero at contact +14. The first recoil step is +15, the last is +34, and the first neutral or guard-ready state write is +35. Hit and crouch block both displace him 78 px before that write. The per-tick magnitudes match the heavy ground reaction already used by the port for measured Super Turbo projectile contacts. [Grounded recoil writes](ken-super-turbo-hadoken-recoil.tsv)

The v1.5.23 Local Versus replay checks every strength for both loadouts. It verifies projectile activation and speed in the 171 px forced-whiff control, immediate standing hit and settled crouch block at the 104 px setup, damage, defender-only freeze, readiness and rollback during flight. Ken returns to stance at release +40 after every outcome. The crouching LK and MK follow-ups retain the earlier measured contact clocks. The port uses the sampled damage amounts and its shared knockout, meter and mixed-era defense rules. [Controller regressions](../../crates/sf-engine/tests/sf2_native.rs)

At 55 px, ground Tatsumaki hit twice with LK or MK and three times with HK in each loadout. All recorded hits dealt 11 HP. The MK second contact and the HK second contact increased the combo counter. The sampled LK second contact and HK third contact landed outside the preceding stun.

The follow-up measures all six ground strength/loadout combinations and compares air entry and contact from all three jump directions. [Tatsumaki advancing-tick audit](ken-super-turbo-tatsumaki.md)

### Shoryureppa activation

At 32 and 55 px, New Ken spent his full 48-point gauge on `236236+LP` and entered the Shoryureppa stream `0x132ea0`. The shorter `23623+LP` also activated it at 32 px. Both recipes yielded five contacts and a final combo counter of four. Old Ken ran the same inputs with the gauge byte set to 48 as a control. He retained all 48 points and entered the ordinary LP Shoryuken stream `0x1c7142`, which hit once for 33–36 HP.

At 32 px in phase zero, New first entered the Super on display 2121, game tick 215. Damage appeared on displays 2135, 2143, 2166, 2175 and 2183 with game ticks 234, 244, 15, 26 and 36. Ryu's HP went from 144 to 129, 113, 97, 80 and 64. The times describe frame-end observations. Exact game-tick boundaries across skipped display samples and cancel windows remain open.

The Super test adds one RAM write at sample 2100, setting `Ken + 0x2b4` to 48. For phase zero, hold `2` at 2110…2111, `3` at 2112…2113, `6` at 2114…2115, `2` at 2116…2117, `3` at 2118…2119, and `6+LP` at 2120…2122. For the shorter input, keep `3` through 2121 and press LP on 2120…2121. Shift the full input by one sample for the second phase. The raw cases are in `%TEMP%/ssf2-ryu-audit/ken-super-*`.

### Shoryureppa game-tick capture for #153

The same World Turbo 2 setup was repeated with write taps. New Ken used both motion forms and each punch in two input phases at 32 px. The full LP input was also repeated at 55 px in two phases. All 14 runs set state 12, wrote the first animation `0x132ea0`, spent the 48-point gauge and set Super flash on the same advancing tick. The flash byte cleared at +18 from that entry, and the first hit occurred at +19. The five contact ticks and Ken's return to stance at +143 held across these fixtures. The sampled total damage was 79–83 HP and the combo counter advanced 0 through 4. [Issue #153](https://github.com/johnstreetfighter/sf1-port/issues/153)

| Hit | Game tick from Super entry | Ken animation · attack ID | Facing-local center · radii | Phase-zero LP damage at 32 px | Ryu X/Y at contact |
| --- | ---: | --- | --- | ---: | --- |
| 1 | +19 | `0x132eb8` · 56 | `(30,45)` · `(28,22)` | 15 | 552/40 |
| 2 | +29 | `0x132ed0` · 57 | `(27,79)` · `(26,44)` | 16 | 544/40 |
| 3 | +56 | `0x132f1c` · 58 | `(24,49)` · `(25,13)` | 16 | 518/40 |
| 4 | +66 | `0x132f34` · 59 | `(30,45)` · `(28,22)` | 17 | 518/40 |
| 5 | +76 | `0x132f4c` · 60 | `(27,79)` · `(26,44)` | 16 | 518/56 |

Every contact writes 14 to each fighter's ordinary freeze byte. Ken's counter reaches zero at +8 from each contact. Ryu's active Super reaction advances his counter faster than ordinary hitstop. It has 1 remaining just before the second contact at entry +29, clears at +39, and clears at +66, +76 and +87 after contacts three through five. The writes at +66 and +76 clear the preceding counter before loading 14 again. Ryu first enters reaction at +20 and stays in state 14 through the last four hits. The fourth contact first raises him to Y 56 on +67. On +77 after the fifth contact, he first moves to Y 61 and the new launch raises him to Y 77. He peaks at Y 138 on +106…+110, lands at +135, rebounds to Y 50 and reaches the floor again at +157. Ryu returns to neutral at +209. Ken's final rising motion begins at +86, reaches Y 178 at +111 and lands at +141 before the neutral-state write at +143.

The source contact count varies with spacing and guard. These additional full-LP captures use the same phase-zero input and measure Ken's first return to stance. The tested runs agree with a 103-tick whiff duration and eight added ticks for each hit or block. Standing guard is held from display 2118 through 2275. The guard gaps are the setup values before Ryu can walk back.

| Setup | Contact ticks from Super entry | Damage or chip | Ken ready | Ryu ready |
| --- | --- | --- | ---: | ---: |
| Clean, 90 px | +22, +48, +58, +68 | 14, 17, 16, 16 | +135 | +201 |
| Clean, 170 px | +48 | 16 | +111 | +178 |
| Whiff, 232 px | — | — | +103 | — |
| Standing guard, 32 px | +19, +29, +58, +68 | 4, 4, 4, 4 chip | +135 | +99 guard-ready |
| Standing guard, 55 px | +19, +29, +58, +72 | 4, 3, 4, 4 chip | +135 | +103 guard-ready |

The 232 px whiff separates the strike animation from contact freeze. From the flash-clear tick, the five attack poses start at +1, +3, +22, +24 and +26. They change to the next pose at +3, +9, +24, +26 and +52. Ken starts the first forward hop at post-flash +2, rises 14 px above the floor and travels 68 px forward before the second series. He starts the last rise at post-flash +28. In the 32 px clean sequence, Ryu moves 4 px away on action +28 and +29. After the second hit he moves another 26 px over +38…+50. He stays at that X through the fifth contact while Ken maintains a gap near 32 px. The saved trace records each game tick, both integer axes and Ken's animation for the 232 px whiff and the 32 and 55 px clean hits. [World trajectory TSV](ken-super-turbo-shoryureppa-trajectory.tsv)

Old Ken kept the supplied meter and never set Super flash in 12 full and short 32 px trials. Both motions select Shoryuken at the corresponding strength. LP enters `0x1c7142`, hits at +2 for 33–36 HP and returns at +53. MP enters `0x1c71f2`, hits at +2 and +20 for 41–42 total and returns at +83. HP enters `0x1c72a2`, hits at +1, +16 and +32 for 42 total and returns at +106. These are normal Shoryuken timings from its own state-12 write. They distinguish the source fallback from the earlier port's LP Hadoken.

The New Ken recognizer loads motion stage 10 at `+$a0` and timer 12 at `+$a1` on the second Down-Forward. The following Forward leaves this timer running. The write callback PCs are `$085958` and `$08595c` for the stage and timer loads, `$085978` for decrement and `$085a2e` for successful action selection. ROM code checks both punch press and release edges on the load tick and on later ticks while at least 2 remain. With Forward held, final LP at second Down-Forward +10 enters Shoryureppa in both display phases. LP at +11 enters ordinary LP Shoryuken at `$085886`, first pose `0x132978`, and leaves the Super stock intact. With the short motion held in Down-Forward, simultaneous Down-Forward + LP and delayed LP at +10 both enter Shoryureppa. Old Ken's two verified inputs contain the regular `623` Shoryuken suffix. Their ordinary special selector is `$0986f6` and accepts either the held final Down-Forward or a following Forward with the punch.

To reproduce the exact traces, use the motion schedule above for each punch and input phase. Tap Ken `+$03` and `+$1a` for action selection and animation, `+$1fa` for flash, `+$2b4` for stock, both fighters' `+$47` freeze bytes and `+$06/+$0a` positions, and Ryu `+$2a` life and `+$1f4` combo. Timestamp writes with `0xff801d`. At contact, use the animation `+$0c` attack ID and the 16-byte entry from Ken's box table at `+$34`. Scratch cases `ken-shoryureppa-exact/<loadout>-<full|short>-<gap>-<phase>-<punch>-0`, `ken-shoryureppa-long` and `ken-shoryureppa-controls` preserve the write, geometry and input logs. Airborne spacing and port verification are open.

For the full-motion recognition boundary, use `N=2120` or `2121` as the first queued final Forward. Keep the same first five directions and hold Forward through the final punch. Queue LP for two display events starting at `N+6` for the two exact +10 acceptances. Use `N=2121` and `N+7` for the +11 rejection. Tap `Ken +$a0/+$a1` and `+$392` in addition to the action and flash writes. The scratch case suffix gives the delay from `N` in display samples.

For the short-motion edge, start the second Down-Forward at `N−2` and hold it through two display samples of LP. Queue LP at `N−2` for exact +0 or at `N+6` for exact +10. The scratch cases are in `ken-shoryureppa-df-edge`.

For a grounded cancel, use New Ken at 32 px with `N=2120`. Hold Down on `N−13…N−12`, Down-Forward on `N−11…N−10`, Forward on `N−9…N−8`, then Down on `N−7…N+1`. Add LK or MK on `N…N+1`. Queue Down-Forward on `N+D−1` and Down-Forward + LP on `N+D…N+D+1`. With `D=4` and `5`, the normal contacts at action +4 and the final LP selects the Super at action +5 and +7 respectively. With `D=6` the final input arrives at +8, after `+$2b0` reaches zero, and the Super does not start. Both normals load 7 to this gate at action entry and decrement it during contact freeze. Flash and stock spend wait until opener contact +15. The five Super damage writes come at contact +34, +44, +71, +81 and +91. The first Super hit occurs during Ryu's state-14 reaction with combo byte 0. The byte first increments at the second Super hit and reaches 4, so the opening normal is absent from the Super's displayed hit count. The scratch cases are in `ken-shoryureppa-early-normal`.

### v1.5.15 Shoryureppa in the port

New Ken now accepts either motion form and any punch through +10 from the second Down-Forward. The tested full-motion +11 input falls back to Shoryuken with stock intact. Both tested Old forms also select Shoryuken. The port's New Ken flashes for 18 ticks. Its crouching MK buffered route accepts contact +1 and +3, rejects +4, and spends meter on contact +15 after the opener pause.

The 32 px clean port replay lands at +19, +29, +56, +66 and +76 and ends at +143. The 232 px whiff verifies the five strike windows and +103 action exit. The attacks use source rectangles and a movement stream based on the measured whiff axes. The port uses SF1 art and a representative defender reaction. Exact reaction movement, guard chip, damage rolls and the 55, 90 and 170 px contact schedules remain open.

### v1.5.16 Shoryureppa follow-up

The clean Local Versus fixtures now match the source contact ticks and Ken's action exit at all four sampled gaps. At 32 and 55 px the contacts are +19, +29, +56, +66 and +76 with exit +143. At 90 px they are +22, +48, +58 and +68 with exit +135. At 170 px the only contact is +48 with exit +111. The 232 px whiff remains +103.

Standing guard at 32 and 55 px replays the source four-contact schedules and +135 Ken exit. New Ryu's guard recoil starts 11 ticks after contact and finishes before guard-ready at +31. The guard fixture includes the eight pixels of retreat before the first contact and places Ryu with room to recoil. It checks rollback from the second contact. The port uses the 32 px clean sample's per-strike HP and four HP per blocked strike, converted from the 144-point arcade life bar. The source also rolled 3 HP chip at 55 px and 14 HP for the first contact at 90 px. Exact clean defender launch, damage variation, and other defender profiles remain open. [Local Versus regression](../../crates/sf-engine/tests/sf2_native.rs)

## Reproduce the source captures

`mame ssf2t -verifyroms` returned `romset ssf2t is good`. The copied merged `ssf2t.zip` has SHA-256 `3fd1c7fb30069586cb9ac7f473745d5074a0c56c28ac583c38bfe5fe9f3a7167`. Its `qsound_hle.zip` dependency has SHA-256 `55feb9b70dacb7364f0310ed16cc9281b5b0a31318b9e2e0587ae06d15f95356`. Use a fresh working directory for each probe and run with `-video none -sound none -nothrottle -skip_gameinfo -autoboot_script trace.lua`. [Pinned MAME manifest](https://github.com/mamedev/mame/blob/mame0289/src/mame/capcom/cps2.cpp#L8152)

Queue controls during the frame-end callback and read the prior input's result on the following display sample. Hold six samples each for P1 coin at 1250, P2 coin at 1270, P1 start at 1300, P2 start at 1340, and both LP at 1400, 1470 and 1540. To select Old Ken, additionally queue P2 Left on 1478…1479, 1482…1483 and 1486…1487, then P2 Right + LP on 1490…1491. Verify character bytes `0` for Ryu and `4` for Ken, and Ken’s Old flag at `+0x3b6`.

At display 2070, write `2` to `0xff83a9` and `0x60` to `0xff8cd3` for World Turbo 2. Set Ryu's integer world X to 552 and Ken's to `552 + gap`, with both fractions zero. These are the only gameplay RAM writes. Ken faces left, so `6` means left and `4` means right in the following recipes. Ryu and Ken's object bases are `0xff844e` and `0xff884e`. Read X and fraction at `+0x06/+0x08`, Y at `+0x0a`, state/substate at `+0x03/+0x04`, animation pointer at `+0x1a`, life at `+0x2a`, freeze at `+0x47`, combo at `+0x1f4`, and the game tick at `0xff801d`. [World Turbo 2 timing procedure](ryu-st-turbo2-contact-evidence.md#replay-setup)

| Probe | Starting gap and queued input |
| --- | --- |
| Ground throws | 32 px. Hold the tested direction and MP, HP, MK or HK at 2120…2121. |
| Air Throw | 48 px initial gap. Ryu neutral jump and Ken forward jump at 2110…2112. New Ken Forward or Back + MK or HK at 2122…2124 or 2125…2127. The earlier 2119…2121 input produces an ordinary jump HK in both loadouts. |
| `236K` and `632K` | 55 px. Start directions at 2116. Hold each direction two samples and add the kick at 2120. The tap releases after 2121. The held HK trial releases after 2169. |
| `41236K` | 55 px. Start `4` at 2112. Hold each direction two samples and add kick to `6` at 2120. Same release times as `236K`. |
| Shoryuken | 55 px. `6` at 2116…2117, `2` at 2118…2119, `3` + the tested punch at 2120…2121. |
| Air Tatsumaki | 60 px. Ken forward jump at 2105…2108. `2` at 2116…2117, `1` at 2118…2119 and `4` + HK at 2120…2121. |
| Hadoken and ground Tatsumaki | 55 px. Hold each direction two samples starting 2116, add the tested attack button to the final direction at 2120…2121. |
| Standing and crouching buttons | 32 px close or crouching, 90 px far. Hold the tested button at 2120…2121. Add Down at 2118…2123 for crouching. |
| Jump normals | 32 px neutral or 78 px forward. Queue the jump direction at 2112…2114 and the tested button at 2120…2121. |
| Forward normal candidates | 90 px. Forward + MP, HP, MK or HK at 2120…2121. |

Raw traces and recipes are in `%TEMP%/ssf2-ryu-audit/ken-st-{new|old}-<case>` on the capture machine. The generator is `%TEMP%/ssf2-ryu-audit/ken-st.py`.

## Port comparison

The probes used port v1.5.8 built from the working tree based on `0e336743`, with the Ryu #139 changes present. The engine library SHA-256 was `e5ec883602cd36cc19ad2662c74de8cff381596a18a219aa7cd5bdb57205cf39`. The headless driver selected the actual styles in Local Versus with Definitive Overhaul, Classic controls, default gauges, and country 0. After 94 active-loop ticks, it walked Ken to each fixture and held both pads neutral for 16 ticks before replaying. Position used the camera-adjusted fighter anchor with the signed pose offset removed. The probe and its output are in `%TEMP%/ssf2-ken-port/ken.rs` and `results.txt`.

| Controller input | World Turbo 2 | Port v1.5.8 |
| --- | --- | --- |
| Forward + MK at 32 px | New knee hold, Old Hell Wheel. | Both start ordinary standing MK (`0x15`). It whiffs in the tested setup. |
| Forward + HK at 32 px | Both use their Hell Wheel streams. | Both start the shared throw (`0x27`), which deals 6 HP for New and 7 for Old at input +24 port ticks. |
| `236LK`, `632LK`, `41236LK` and held `236HK` | New selects three separate special streams at 55 px. Old uses ordinary moves or proximity throws in the tested recipes. | Both select ordinary kicks at 52 px. `632LK` lands crouching LK. The other LK samples whiff. Held `236HK` becomes standing HK and hits once. |
| Close LP at 32 px | Each deals 4 HP, starts with 14 freeze per fighter, and moves Ryu back 30 px. | Each deals 4 HP at input +3, with no ordinary freeze. Grounded stun is 9 ticks New and 13 Old. Ryu moves 16 px New and 24 Old. |
| Crouching LK at 32 px | Clean New / Old deal 4 / 6 HP. Clean and guard start with 14 freeze per fighter and move Ryu back 30 px. | Clean New / Old both deal 3 HP at +6 from kick press and move Ryu 24 px. Hitstun is 18 / 22 ticks. Guard has 12 blockstun ticks for both and no pushback. All port samples have zero ordinary freeze. |
| Crouching MK at 32 px | Both leave Ryu grounded and push him back 54 px. | Both deal 4 HP at +6 from kick press, launch Ryu in reaction `0x33`, and move him back 140 px over the reaction. |
| Far LK near 90 px | New hits at 90 px, Old whiffs. | Both whiff at the nearest sampled 92 px. The two-pixel spacing difference keeps this probe from identifying the exact reach difference. |
| `623HP` near 55 px | Both hit twice at 55 px. | Both start Shoryuken (`0x19`) and whiff at 52 px. The port two-hit ceiling needs an in-range contact test. |

### Crouching kick into LP Hadoken

In the arcade, both Ken loadouts connected crouching MK into LP Hadoken as a true combo. For the representative +5 display-sample recipe, New’s opener lands at sample 2124 and tick 221. Old’s opener lands at sample 2124 and tick 223. Both start the special at sample 2130. The second hits at samples 2146 and 2145, ticks 249 and 250 respectively, each with combo counter 1. For both loadouts, the sampled crouching LK routes with final LP queued at opener display +2, +5 and +9 landed Hadoken after Ryu had recovered. These are sampled inputs. They leave the exact limits and overlapping recipes open.

In the port, both loadouts start Hadoken with final LP at +2, +4, +6, +8 or +10 port ticks after crouching MK contact. At the first four offsets the projectile hits at opener +43. At the last it hits at +45. All arrive after the launcher reaction. Crouching LK into Hadoken combos at the tested +2, +4 and +6 offsets for New and at all five offsets for Old. New’s +8 and +10 hits arrive late. The port recipe holds Down for 25 ticks, taps the kick, then holds Down until one tick before Down-Forward. Forward + LP follows one tick later, followed by neutral.

To repeat the arcade crouching MK route, use the same 32-pixel setup and hold Ken’s Down at 2095–2127. Add MK at 2120–2121. Queue Down-Forward at 2128 and Forward + LP at 2129, then neutral. To repeat the crouching LK guard, hold Ken’s Down from 2095 through 2123 and Ryu’s Down-Back from 2118 through 2260, with Ken’s LK at 2120–2121. New’s guard contact appears at tick 221 and guard-ready at tick 249. Old’s appears at tick 221 and guard-ready at tick 248. Ken’s and Ryu’s ordinary freeze both read 14 at contact.

## World Turbo 2 write-tap follow-up for #152

The follow-up keeps the World 940223 source, Turbo 2, P1 New Ryu and P2 New or Old Ken. At the normal's first contact both origins remain X 552 and 584, floor Y 40, with zero X fractions. Each ordinary normal was captured on clean hit and settled crouch guard in two display phases for each loadout. The table reports advancing game ticks from the CPU state or freeze write. [Issue #152](https://github.com/johnstreetfighter/sf1-port/issues/152)

| Kick | Contact from attack entry, New / Old | Clean damage in the two phases, New / Old | Ryu first ready after contact, clean or guard | Ken leaves attack / returns to stand | Ryu recoil |
| --- | --- | --- | --- | --- | --- |
| 2LK | +4 / +3 | 4, 4 / 6, 4 | +27 | +23 / +28 | 30 px over +16 through +26 |
| 2MK | +4 / +4 | 19, 19 / 21, 19 | +32 | +29 / +34 | 54 px over +16 through +31 |

Both fighters load 14 ordinary freeze on the contact tick. Ken's byte reaches zero at +14 and Ryu's at +15. Ryu's first recoil step is +16, and his Y remains 40 through both hit and block reactions. The guard costs zero life. Ryu returns to state 0 on clean hit and guard-ready state 8 on block. For LK the per-tick recoil magnitudes are `4, 4, 4, 4, 3, 3, 3, 2, 1, 1, 1`. For MK they are `6, 6, 6, 5, 5, 5, 4, 4, 3, 3, 2, 1, 1, 1, 1, 1`. They apply away from Ken in the measured facing. The clean damage samples depend on phase. The earlier v1.5.8 port launched Ryu from MK, omitted the ordinary freeze in its LK trials, and gave zero guarded LK recoil. The source confirms grounded MK and full recoil in both guard and clean cases.

At a 170 px whiff gap, New 2LK is active at action +4…+7, leaves attack state at +13 and returns to stand at +18 with Down released. Old 2LK is active at +3…+6, leaves attack at +12 and returns to stand at +17. Both 2MK variants are active at +4…+9, leave attack at +19 and return to stand at +24. These timings repeat in both display phases. The first active 2LK poses are `0x13172c` and `0x1c682a` for New and Old, and the first active 2MK poses are `0x1317a8` and `0x1c68a6`.

The first active boxes were identical for New and Old Ken. With X positive in the direction each fighter faces and Y positive above the floor, Ken's 2LK strike is ID 22, center `(+56,+9)`, radii `(33,12)`. His 2MK strike is ID 23, center `(+58,+9)`, radii `(40,12)`. From left-facing Ken at X 584, their world rectangles are X 495–561 and 486–566 respectively, both Y 37–61. Right-facing New Ryu's neutral head, torso and foot boxes are ID 1 with `(center X, center Y, X radius, Y radius)` of `(4,79,12,8)`, `(−6,52,20,21)` and `(−6,14,20,16)`. Crouch guard uses head ID 14 `(14,51,12,9)`, torso ID 8 `(6,33,22,12)` and foot ID 8 `(6,11,22,12)`. Both kicks intersect the foot boxes at the measured gap. [ST hitbox table layout](https://github.com/peon2/fbneo-training-mode/blob/master/hitboxes/st-hitboxes.lua)

### Light Hadoken cancel and contact

The final Forward + LP input writes `18` with preceding Down-Forward `6` in the P2 input word. The cancel writes Ken state 12 on that same game tick, while his first Hadoken animation can wait for attacker freeze to finish. The projectile spawns and contacts on the same game tick at 32 px in the measured cancel routes. The combo byte counts 1 while Ryu remains in the MK reaction. The same early cancel from LK can spawn a shot and hit with combo count 0 after Ryu becomes ready.

| Opener and result | Final LP and special-state selection from contact | First Hadoken pose from contact | Projectile spawn and life write from contact |
| --- | ---: | ---: | --- |
| New 2MK early combo | +1 | +15 | +28, combo 1 |
| New 2MK last accepted | +16 | +16 | +29, combo 1 |
| Old 2MK early combo | +1 | +15 | +27, combo 1 |
| Old 2MK last accepted | +16 | +16 | +28, combo 1 |
| New and Old 2MK first late input | +17 | — | No cancel or projectile in the tested motion |
| New 2LK early cancel | +1 | +15 | +28, combo 0 |
| Old 2LK early cancel | +1 | +15 | +27, combo 0. Ryu's ready write occurs before contact on this tick. |
| New 2LK last accepted | +16 | +16 | +29, combo 0 |
| Old 2LK last accepted | +17 | +17 | +29, combo 0 |
| New / Old 2LK first rejected after countdown | +17 / +18 | — | No cancel or projectile in the tested motion |
| New 2MK after normal recovery | +31 | +31 | +44, combo 0 |
| Old 2MK after normal recovery | +32 | +32 | +44, combo 0 |

The first Hadoken animation pointer is `0x13284c` for New and `0x1c7016` for Old. The interval from that write to the spawn is 13 ticks for New and 12 for Old in the two successful MK boundary fixtures. Raw LP Hadoken at 232 px gives those same intervals. For MK and New LK the ordinary cancel byte `+$195` is 2 after impact, then 1 when input is processed on +16 and zero on +17. Old LK has 3 after impact, 1 on +17 and zero on +18. Several short motion timings left the cancel unused despite a valid button edge and positive countdown. The table records observed endpoints. The earliest confirmed normal-preserving input is +1. Earlier sampled inputs can replace the kick with a raw Hadoken.

Both Ken loadouts use the same LP projectile flight at 232 px. The first live shot is 80 px forward from Ken's axis, 56 px above the floor. Left-facing Ken at `(784,40)` spawns it at `(704,96)` with zero X fraction. It travels 3 px per advancing tick. At ages 0 and 1, animation `0x6ebd2` uses attack ID 4 with facing-local center `(−16,0)` and radii `(31,12)`. At age 2, animation `0x6ebea` uses ID 1 with center `(0,0)` and radii `(15,12)`. Later sampled flight poses retain ID 1. In the first phase the display callback samples age 1, and the second phase samples age 0. The allocation write supplies the exact age origin.

### Reproduce the follow-up

Use the same boot, loadout selection, MAME version and Turbo 2 setup above. Set `N` to 2120 or 2121 for isolated normals, then hold Ken's Down on `N−25…N+3` and LK or MK on `N…N+1`. For guard, also hold Ryu Down-Back on `N−2…N+100`. For whiff, move Ken to X 722 at setup. To sweep a one-display Hadoken motion at final-button delay `D`, hold Ken Down from `N−25` through `N+D−2`, queue Ken Down-Forward at `N+D−1`, and Forward + LP at `N+D`. Use `D ≥ 3` and keep the kick schedule. New MK early, last and rejected are `N=2120`, `D=4,16,17`. Old MK uses `N=2120`, `D=4,15,16` for those outcomes. New LK last and rejected use `D=16` at `N=2123` and `2120`, respectively. Old LK last and rejected use `N=2120`, `D=15,16`.

For the held-motion cases, put Down-Forward on `N+D−2…N+D−1` and Forward + LP on `N+D…N+D+1`, leaving the other inputs unchanged. Early LK uses `N=2120`, `D=4` for New and `D=3` for Old. The separate Hadoken after MK recovery uses `N=2120`, `D=28` for New and `N=2121`, `D=27` for Old.

At each write read `0xff801d` and timestamp the first state-10 and state-12 writes to `Ken +$03`, the first special animation pointer at `Ken +$1a`, and the ordinary cancel countdown at `Ken +$195`. Tap both `+$47` freeze bytes, Ryu's `+$06` and `+$0a` positions, `+$2a` life and `+$1f4` combo, and Ken's `+$392` input. Contact zero is the first incoming freeze value 14 on Ryu for the normal. Use the first life decrement and combo increment to classify the projectile. Find its first `0x0101` header in the slots starting `0xff97a2` at stride `0xc0`. Scratch captures use the `ken-st-contact-t2`, `ken-st-contact-t2-geometry` and `ken-st-contact-t2-held` folders under `%TEMP%/ssf2-ryu-audit`.

To reproduce raw projectile flight, place Ken at X 784 and use `N=2120` and `2121`. Hold Ken Down on `N−8…N−5`, Down-Forward on `N−4…N−1`, and Forward + LP on `N…N+4`. Leave Ryu neutral. Use the allocation tick from the header write, then sample shot positions `+$06/+$08/+$0a` and animation pointer `+$1a` on each display. Read attack ID at animation `+$0c` and the 16-byte attack record through the shot's box-table pointer `+$34`. The raw flight captures use `ken-st-contact-t2-projectile-live`.

### v1.5.14 grounded port comparison

The native Local Versus replay covers New and Old Ken against New Ryu. It starts with neutral or crouch guard at 32 px. Both loadouts now keep Ryu grounded through 14 ticks of contact freeze, followed by the measured recoil from contact +16. LK moves Ryu 30 px and leaves him ready at +27. MK moves him 54 px and leaves him ready at +32. The 170 px tests verify the attack intervals and exit clocks. The port can enter the next allowed action after its native attack stream ends. The source's separate transition from crouch to standing remains unverified in the port.

For the cancel tests, hold Down through the kick contact. For the early recipe, press Down-Forward at contact +1 and Forward + LP at +2. For a late recipe, queue Down and Down-Forward at two and one ticks before the chosen final LP. The latest MK input at +16 remains a combo in both loadouts. The latest LK inputs at +16 New and +17 Old start Hadoken, which lands outside hitstun. The next input is declined while the kick is busy. The replay also checks a separate Hadoken after MK recovery, snapshot replay and New Ken's crouching MK against Third Strike Ryu's low parry. The port retains its damage model and queues the early selection through the contact pause. [Native controller tests](../../crates/sf-engine/tests/sf2_native.rs)

### Raw Shoryureppa in the port

At 32 pixels, New Ken started Shoryureppa on the final LP of `236236+LP` and spent one stock. The first start sample showed 27 Super freeze ticks remaining. The hits arrived at +32, +36 and +53 port ticks relative to final LP and did 4, 5 and 5 HP. The last two registered into the preceding stun. Ken first returned to stance at +118. At 52 and 92 pixels the same input still starts Shoryureppa, with the contacts at +32/+53/+55 and +53/+55/+57 respectively. Only the second hit at 52 pixels registered into the preceding stun. Old Ken fired regular LP Hadoken. At 32 pixels it hit once at +11 for 10 HP. The arcade comparison established five Super contacts for New and an LP Shoryuken fallback for Old.

Use the same Local Versus fixture. Before New’s input, set only P2’s meter to 144 in the native fight snapshot. Old retains his native gauge rules. Hold each direction for two port ticks and add LP on the first tick of the second Forward. Leave P1 neutral. Record each health change and `LandedHit::into_hitstun` as well as the first Super and stance move IDs. The supplementary driver is `%TEMP%/ssf2-ken-port/super.rs`.

## Ground walk follow-up

Both Ken loadouts in World 940223 Turbo 2 step forward 3.1875 px (`$00033000`) and back 2.125 px (`$00022000`) per advancing tick when clear of the opponent and walls. Both move on the first sampled input tick, and the first neutral tick adds zero. At 59.637405 display Hz the first visible movement follows the queued input by one display sample. Each active display interval contains one or two position updates.

| Loadout | Forward, 16 display intervals | Back, 16 display intervals |
| --- | --- | --- |
| New | 70.125, 66.9375, 70.125 px | 44.625, 44.625, 42.5 px |
| Old | 70.125, 66.9375, 70.125 px | 46.75, 44.625, 44.625 px |

Use the World Turbo 2 setup above. Set New Ryu at X552 and Ken at X720 at display event 2160 and zero both fractions. Queue left-facing Ken's Left or Right at `N…N+28`, where `N` is 2200, 2201 or 2202. Read the sampled input word `+$392`, X and fraction `+$06/+$08`, and the advancing tick byte `$ff801d`. The 16-display measurements run from sample `N+1` through `N+17`. [Arcade walk samples](ken-super-turbo-walk.tsv)

For the port's 60 Hz clock, the fixed cadence uses 43 advancing-equivalent updates per 32 ticks going forward in both loadouts, 41 going back in New and 42 going back in Old. These counts come from the average of the 29 held, unclipped samples per phase, converted from 59.637405 display Hz to 60 Hz and rounded to the nearest update over 32 ticks. The per-update movement remains 51/16 px forward and 34/16 px back. Whole-pixel movement in each 32-tick port cycle is 137 px forward, 87 px back in New and 89 px back in Old. Port replays verify first-tick movement, immediate stop on release, a fresh start and an interrupting normal in both facing directions and both loadouts. They also verify rollback during the cadence. [Local Versus regression](../../crates/sf-engine/tests/st_ken_walk.rs)

## Base jump trajectories

Against neutral New Ryu, Ken's three jumps use identical airborne trajectories in both loadouts. Old lifts one advancing game tick earlier. The table gives first rising axis write, first return to floor, and Ken's first idle-state write. Tick zero is the state-4 write. The peak is above floor Y40 and travel is signed relative to facing.

| Jump | New lift / floor / ready | Old lift / floor / ready | Peak | Travel |
| --- | --- | --- | ---: | ---: |
| Neutral | 5 / 49 / 56 | 4 / 48 / 55 | 77 px | 0 px |
| Forward | 5 / 48 / 55 | 4 / 47 / 54 | 74 px | +134.6640625 px |
| Back | 5 / 50 / 57 | 4 / 49 / 56 | 80 px | −162.88671875 px |

Use the ground walk setup at X552/X720 for neutral and Forward with `N=2200`, `2201`, or `2202`. For Back use Ryu X445 and Ken X525 so the whole arc fits before the arena edge. The first Back sweep at X552/X720 grazed the wall and measured 135.88671875 px. Keep Ryu neutral. Queue Up, Up-Left or Up-Right for `N…N+2` for the left-facing Ken. Read Ken's state word `+$02`, X/fraction `+$06/+$08`, Y `+$0a`, and the advancing tick byte `$ff801d`. Forward begins at 3.48046875 px per tick and Back at 3.98046875 px. The magnitude decreases by 0.01953125 px each update, including the first floor update. The same direction and loadout reproduce all anchors in three independent input phases. The position table preserves signed world X, so negative travel is left on screen. [Arcade jump trajectories](ken-super-turbo-jump-trajectory.tsv)

The Local Versus replay checks all six free jump paths through both facings using the original action clock, exact floor-relative heights and the arcade's 16.16 X rounded to the nearest port pixel. Both loadouts can branch into measured jump normals and existing Air Tatsumaki. New Ken can enter Air Throw. The jump-normal replay covers each button in each jump direction and a late neutral MK against standing, standing guard and crouch guard New Ryu. These paths use the resident SF1 Ken sprites. Air-to-air reactions and wall or opponent collision behavior remain open. [Local Versus tests](../../crates/sf-engine/tests/st_ken_jump.rs) · [Arcade jump-normal captures](ken-super-turbo-jump-normals.md)

## Open verification

- Measure Hadoken air-hit outcomes and clashes across spacing and document damage RNG variance. Check exact close/far thresholds and pair jump-normal and Air Tatsumaki reactions at matched heights and distances.
- Extend the [throw follow-up](ken-super-turbo-throws.md) to exact mash parity, corner paths and additional defender profiles.
- Extend the [command kick follow-up](ken-super-turbo-command-kicks.md) to the spacing-dependent follow-up contacts and defender reactions.
- Extend the Shoryureppa tests to corner and airborne routes. Replay the measured Shoryuken paths and spacing-dependent contacts in the port. [Source sweep](ken-super-turbo-shoryuken.md)
