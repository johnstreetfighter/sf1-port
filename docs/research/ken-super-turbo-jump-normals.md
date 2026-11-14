# Ken’s jump normals in World Super Turbo

The reference is **Super Street Fighter II Turbo, World 940223** (`ssf2t`), with World Turbo 2 selected in MAME 0.289. New Ryu is P1 and Ken is P2. The source captures cover all six buttons during New and Old Ken’s neutral, forward and backward jumps. The existing arcade audit pins the ROM hashes, cabinet settings and fighter selection. [Arcade setup](ken-super-turbo-audit.md#reproduce-the-source-captures) · [MAME ROM manifest](https://github.com/mamedev/mame/blob/mame0289/src/mame/capcom/cps2.cpp#L8152)

## Timing and geometry

Jump tick zero is the write of state 4 at PC `$07b482`. The air attack starts inside state 4 at the substate-6 write at `$07899e`. The 72 early whiffs used two input phases. The button queued eight displays after Up produced attack dispatch at jump +11 in phase 0. Phase 1 gives jump +10 for New neutral and forward, and +11 for the other cases. Each attack’s relative animation and box changes agree throughout the early replays. Forward and backward select identical attack animations within each loadout. Neutral chooses separate streams.

The half-open active intervals below count from attack dispatch. Floor means the landing transition replaces the last attack pose. Each shape is `(ID, forward X, Y above Ken’s axis, X radius, Y radius)`. Positive X in this table points in the attack’s facing direction. The TSV retains the original signed X, animation pointers and three Ken hurtboxes. [Whiff animation writes](ken-super-turbo-jump-normal-streams.tsv)

| Button | Neutral active | Neutral shape | Directional active | Directional shape |
| --- | --- | --- | --- | --- |
| LP | [2, Floor) | `45,29,77,21,19` | [2, Floor) | `11,29,74,21,16` |
| MP | [4, 24) | `12,30,81,31,11` | [4, 12) | `13,30,81,31,11` |
| HP | [4, 24) | `46,33,59,31,9` | [4, 12) | `47,33,59,31,9` |
| LK | [3, Floor) | `27,10,75,34,19` | [5, Floor) | New `35,15,50,19,15`, Old `35,13,56,26,15` |
| MK | [5, 18) | `28,16,75,39,19` | New [5, 10), [10, 18). Old [5, 18). | New `29,-2,51,25,18`, then `73,45,61,35,9`. Old `29,24,61,49,12`. |
| HK | [2, 6), [6, 10) | First `25,18,70,25,20`. Second New `26,30,45,42,14`, Old `26,34,56,46,25`. | [5, 12) | New `30,43,55,43,18`, Old `30,30,55,56,18` |

Across the 72 early and 36 late whiffs, every 16.16 X displacement and integer Y agrees with Ken’s corresponding unarmed jump at each advancing tick through idle. The first floor/idle writes are New neutral +49/+56, forward +48/+55, backward +50/+57. Old reaches both writes one tick earlier. [Unarmed jump trajectories](ken-super-turbo-jump-trajectory.tsv)

The late whiff schedule queues each button on displays `N+33…N+35`. New starts the air attack at jump +45 in all directions. Old starts at +45 in neutral and backward, and +44 in forward. Forward LP gets one active tick before floor in both loadouts. Forward MP, HP, LK, MK and HK start their animations and land before their first active frame. The late button leaves the original 7-tick landing recovery intact. [Late whiff cases](ken-super-turbo-jump-normal-cases.tsv)

## Grounded New Ryu contacts

Each cell gives standing clean hit / standing guard as two events. `45/22` means jump tick +45 and 22 arcade HP. `45g` is jump tick +45, standing block and zero chip. Damage is the result for the sampled input phase and spacing. The contacts table includes the hit rectangle, both world axes, defender hurtboxes and recoil per tick. [Grounded reactions](ken-super-turbo-jump-normal-contacts.tsv)

| Button | New neutral | Old neutral | New forward | Old forward | New backward | Old backward |
| --- | --- | --- | --- | --- | --- | --- |
| LP | 44/11 · 44g | 43/11 · 43g | 43/13 · 43g | 42/11 · 42g | 45/11 · 45g | 44/10 · 44g |
| MP | 46/19 · 46g | 45/19 · 45g | 45/20 · 45g | 44/17 · 44g | 47/22 · 47g | 46/22 · 46g |
| HP | 45/22 · 45g | 44/22 · 44g | 45/23 · 45g | 43/26 · 43g | 44/21 · 44g | 43/26 · 43g |
| LK | 44/12 · 44g | 43/12 · 43g | 46/11 · 46g | 44/9 · 44g | 42/11 · 42g* | 42/11 · 43g* |
| MK | 46/19 · 46g | 45/19 · 45g | 46/22 · 46g | 44/17 · 44g | 47/22 · 47g | 43/19 · 43g |
| HK | 43/25 · 43g | 42/27 · 42g | 46/22 · 46g | 44/24 · 44g | 42/22 · 42g | 42/26 · 42g |

The asterisk for backward LK denotes a positioning control. In the input-only sweep at configured gaps 24 to 48 px, LK either misses or hits before Back takes effect. Holding Back first and writing Ryu X630 with zero fraction at display `N+31` produces a standing block in both loadouts. X632, X636 and X640 give the same result. X626 whiffs. These fixture rows use `sgpin` and record the position override.

On the primary Down-Back recipes, LP and MP whiff in all directions and backward LK whiffs. Neutral and forward LK and all HP, MK and HK variants deal clean damage when they reach Ryu. For example, New forward MK hits standing guard and Down-Back on the same jump tick +46. The standing guard takes zero damage. The crouching target takes 22 HP. The New backward MK hit at jump +47 uses the second attack shape, ID 73.

Contact loads 14 freeze for both fighters. Ken’s decrement writes zero at contact +14 and Ryu’s at +15. Ken lands and exits 14 ticks later than the matching whiff. The scripted defender recoil begins at +16. Standing clean contacts select the light reaction and return to idle at +27 for all six buttons. Standing guard and the verified crouching clean hits use the reaction tier by strength. On guard, the exit write returns Ryu to guard-ready state 8.

| Reaction | Exit from contact | Scripted recoil steps | Scripted distance |
| --- | ---: | --- | ---: |
| Light | +27 | `4,4,4,4,3,3,3,2,1,1,1` | 30 px |
| Medium | +32 | `6,6,6,5,5,5,4,4,3,3,2,1,1,1,1,1` | 54 px |
| Heavy | +36 | `8,8,8,7,7,7,6,5,5,4,3,2,1,1,1,1,1,1,1,1` | 78 px |

Total axis displacement can also include auto-spacing writes at `$0bf7a0`. A paired test can check the scripted sequence independently from collision separation.

## Reproduce the captures

Use the selection and EEPROM setup in the [Ken arcade audit](ken-super-turbo-audit.md#reproduce-the-source-captures). At display 2160, write `$ff83a9=2` and `$ff8cd3=$60`, then write X and zero the X fractions for both fighters. Queue P2 Up, Up-Left, or Up-Right from display `N` through `N+2`, where `N=2200+phase`. Each case’s button interval begins at `N+button_display_delay` and lasts three displays. All other inputs are **screen relative**. For the early and late neutral/forward whiffs, use Ryu X520, Ken X740 and no Ryu inputs. For early and late backward whiffs, use Ryu X445 and Ken X525. The early delay is eight display samples and the late delay is 33.

The primary grounded cases use phase 0 and Ryu X510. At neutral gap 38 and forward gap 128, queue the button from `N+30` and Ryu Left for standing guard or Down-Left for crouch guard from `N+31`. For backward gap 31, queue the button from `N+28`. Ryu pursues Ken with screen Right from `N` for 47 samples in the standing clean recipe. In the basic guarded recipe, he pursues for 29 samples, then holds Left or Down-Left. The backward LP and MP standing guard successes in the table use 32 samples of approach. The backward LK positioning controls omit the approach, hold Left from `N+25` and set X at `N+31`. Hold the guard inputs through `N+130`. During split-side trials a nominal standing guard can become a clean hit, so use the written outcome and facing. The case file gives the configured gap and the actual integer X and facing at jump entry.

Read the advancing game clock at `$ff801d`. Tap state, substates, animations and air flag at fighter `+$02`, `+$04`, `+$1a` and `+$180` words. Record 16.16 X from words `+$06/+$08` and integer Y from `+$0a` with floor at Y40. Animation bytes `+$08…+$0a` index head, body and leg box tables at offsets 0, 2 and 4. Byte `+$0c` indexes the attack table at offset 6, stride 16. The table base is the longword at fighter `+$34`, with each subtable offset a signed word at the indicated position. The per-pose attack fields list decimal ID, raw signed X/Y, radii and the twelve hex bytes at offsets 4…15. X is in the arcade’s left-facing convention. Facing flag 1 reflects it. Read clean damage at Ryu’s `+$2a` write at `$0be64c` and both freeze loads at `$0bed1c/$0bed22`. The case and contact TSVs retain both the display and the absolute advancing tick to identify the tapped transition between display samples.

## Port comparison

Both World Super Turbo Ken loadouts keep the measured unarmed flight and landing clocks while playing the jump-normal animation at its own age. Local Versus verifies X, height and the attack rectangles for all 36 loadout, direction and button combinations in both facings, plus the forward LP input three ticks before floor. Against grounded New Ryu, both loadouts take 14 ticks of contact freeze and use the measured reaction. The late neutral MK test covers clean standing, standing guard and Down-Back for each loadout. The port uses resident SF1 Ken art, converts three source hurtboxes to the native two-box format, and uses representative recorded damage. Paired air-to-air reactions, corner collision, damage variance, normal-to-Air-Tatsumaki cancels and precise input behavior within the final two ticks remain open. [Ken jump bank](../../crates/sf-engine/src/data_bank/st_ken_jump.rs) · [Local Versus checks](../../crates/sf-engine/tests/st_ken_jump.rs)
