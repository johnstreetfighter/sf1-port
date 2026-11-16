# Sagat’s aerial normals in World Super Turbo

The source is Super Street Fighter II Turbo World 940223 (`ssf2t`) in MAME 0.289. The 36 whiff cases cover six buttons on neutral, forward and backward jump for each Sagat loadout, with New Ken on P2. Every run checks Sagat ID 9, Ken ID 4, both loadout flags, and World Turbo 2 selector 2 and pacing threshold `$60`. [MAME ROM manifest](https://github.com/mamedev/mame/blob/mame0289/src/mame/capcom/cps2.cpp#L8152)

## Jump and attack timing

The jump clock starts on fighter state 4 at PC `$07b482`; that write shares the game tick of the sampled Up. The air flag at fighter `+$181` rises before the first positive floor-relative Y. Landing is the air flag clearing at `$078a7a`. State 0 at `$0784d4` marks return to idle in these whiff cases.

| Direction | New lift / landing / idle | Old lift / landing / idle | Peak height | X at idle from entry |
| --- | --- | --- | ---: | ---: |
| Neutral | +5 / +46 / +53 | +3 / +44 / +51 | 75 px | 0 |
| Forward | +5 / +45 / +52 | +3 / +43 / +50 | 70 px | +90.80859375 px |
| Backward | +5 / +48 / +55 | +3 / +46 / +53 | 80 px | −99.6640625 px |

The six phase-0 unarmed replays give Sagat pushbox ID 1 at jump entry and landing, with center `(0,42)` and radii `(23,43)`. The airborne pose uses ID 2, center `(0,82)` and radii `(23,23)`, loaded at +4 New and +2 Old. Standing New Ken in the same fixture uses ID 2, center `(0,57)` and radii `(16,33)`. Decode the pushbox through animation `+$0d`, table offset `+8`, stride 4. [Pushbox decoder layout](https://github.com/peon2/fbneo-training-mode/blob/master/hitboxes/st-hitboxes.lua)

The move loads its first air-attack animation at jump +11 in the fixed schedule. Old is two advancing ticks further into its jump at dispatch. Forward and backward jumps share each button’s attack stream in their respective loadout. Neutral jump selects separate streams. All attack intervals below are half-open and relative to jump entry. **Land** means the last live pose is replaced on landing. Geometry lists the attack ID, forward center, height above Sagat’s axis, horizontal radius and vertical radius.

| Button | Neutral interval | Directional interval | Neutral box | Directional box |
| --- | --- | --- | --- | --- |
| LP | [14, Land) | [14,38) | `40,56,96,27,11` | `48,56,89,27,18` |
| MP | [14,28) | [14,28) | `41,61,96,35,11` | `49,62,95,35,12` |
| HP | [16,26) | [16,24) | `42,55,75,23,20` | `43,65,70,25,9` |
| LK | [13, Land) | [13, Land) | `44,61,87,28,10` | `12,42,64,25,18` |
| MK | [14,34) | [14,34) | `10,65,65,35,8` | New `11,46,65,54,8`, Old `11,65,65,35,8` |
| HK | [16,24) | [16,24) | `28,70,58,41,15` | `30,67,60,38,13` |

The first active offset from attack dispatch is +2 for LK, +3 for LP/MP/MK and +5 for HP/HK in both loadouts. The first air-attack animation pointers are as follows.

| Button | New neutral / directional | Old neutral / directional |
| --- | --- | --- |
| LP | `$15bee6 / $15c05a` | `$1ed698 / $1ed80c` |
| MP | `$15bf32 / $15c0ee` | `$1ed6e4 / $1ed8a0` |
| HP | `$15bfc6 / $15c182` | `$1ed778 / $1ed934` |
| LK | `$15c27a / $15c216` | `$1eda2c / $1ed9c8` |
| MK | `$15c2de / $15c406` | `$1eda90 / $1edbb8` |
| HK | `$15c372 / $15c49a` | `$1edb24 / $1edc4c` |

## Forward jump contact

These 48 grounded-defender controls use requested gap 105 px and a later button, on displays `N+21…N+23`. All listed standing clean hits are blocked by standing guard at the same tick for zero chip. Damage numbers are the observed single-phase HP loss.

| Button | New standing tick / damage | Old standing tick / damage |
| --- | --- | --- |
| LP | +42 / 12 | +40 / 13 |
| MP | Whiff | +42 / 21 |
| HP | Whiff | +40 / 27 |
| LK | +38 / 14 | +36 / 14 |
| MK | +40 / 21 | +38 / 19 |
| HK | +38 / 21 | +36 / 26 |

New directional MK hits settled crouch and Down-Back at jump +44 for 19 and 18 HP, respectively, at Sagat height 3 px. The Old directional MK box misses both controls. The other five buttons whiff on crouch and Down-Back for both loadouts in this setup. All contacts take 14 freeze on each side. They move the forward-jump landing and idle writes to +59 and +66 for New, and +57 and +64 for Old. The two-tick difference in physical jump phase at dispatch changes which late attacks reach standing Ken. The active intervals and geometry above isolate the shared timing from the loadout-specific directional MK box.

Ken exits standing hit reaction at contact +27 in every recorded forward normal hit. Standing guard exits after +27 for LP/LK, +32 for MP/MK, and +36 for HP/HK. The New MK against crouch and Down-Back gives +32. The clean light recoil sequence starts at contact +16 and travels 30 px. Medium and heavy recoil travel 54 and 78 px. Total X also includes pose-dependent separation. For example, Old MP guard travels 55 px in the recorded fixture and New MK hits against crouch travel 66 px.

## Replay

Reuse the selection and EEPROM fixture in the [Sagat arcade audit](sagat-super-turbo-audit.md). At the end of display `N−2`, write Turbo 2 through `$ff83a9=2` and `$ff8cd3=$60`. Set Sagat X552 and Ken X752 with zero fractions, and keep Ken’s Y at 250 from that display for the whiff. Use `N=2200`. Hold P1 Up, Up-Forward or Up-Back on displays `N…N+2`, and the selected attack on `N+8…N+10`.

Use `$ff801d` to timestamp the state write, `+$181` air flag, attack-dispatch substate `+$04=6` at `$07899e`, animation pointer `+$1a`, 16.16 X at `+$06` and floor-relative Y from `+$0a` minus 40. Read the attack ID at animation `+$0c` and geometry through the fighter’s table at `+$34`, attack offset `+6`, stride 16. Facing stays right throughout these runs.

For the contact fixture set Ken X657 on `N−2` and leave his height unmodified. Hold P2 Back on `N+22…N+90` for standing guard. For neutral crouch or crouch guard, hold Down or Down-Back on `N−12…N+90`. Tap defender life at PC `$0be64c` for hits and the defender freeze load at `$0bed22` for zero-chip blocks. The 12 unarmed comparison jumps repeat the high-target fixture across both loadouts, directions and phases with no attack and P1 Back from `N+12…N+65`; the recorded neutral write is the return from state 4. The [capture TSV](sagat-super-turbo-air-normal-writes.tsv) retains all 96 replays. Each contact field has tick, damage, both freezes, Sagat X/height, Ken X/height, and the attack box.

## Port comparison in v1.5.20

The port keeps the measured jump clock across attack dispatch and hitstop. Its tests compare the integer position on every tick for both loadouts, all three jumps and all six attack buttons. The late forward MK comparison covers clean standing hit, standing guard, settled crouch and Down-Back. Damage uses representative source samples. Sagat uses resident sprites, and airborne and corner contacts outside these fixtures still need paired comparisons. [Per-tick unarmed flight](sagat-super-turbo-jump-trajectory.tsv) · [Local Versus checks](../../crates/sf-engine/tests/sf2_native_moves.rs)
