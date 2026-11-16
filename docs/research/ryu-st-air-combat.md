# Ryu’s jumping MP and Air Tatsumaki in World Super Turbo

These runs replay the audited World `ssf2t` 940223 New and Old Ryu versus New Ken inputs in MAME 0.289. The new taps use game ticks and read current hitboxes at the exact damage write. Source Turbo 2 uses selector `2` and pacing threshold `0x60`. The existing Turbo 1 recipe remains the reference for the full three-jump route. [Jumping MP and Air Tatsumaki #140](https://github.com/johnstreetfighter/sf1-port/issues/140) · [Earlier audit and replay inputs at `9646b61`](https://github.com/johnstreetfighter/sf1-port/tree/9646b61/docs/research/evidence/ryu-super-turbo)

## Jumping MP move streams

Tick +0 is the first jump attack animation write. The phase table uses no-contact runs. The attack box centers are forward from Ryu’s facing-relative axis and up from his current world Y. Radii are horizontal × vertical.

| Loadout and jump | First animation | Active attack records | Following pose writes |
| --- | --- | --- | --- |
| New directional | `0x110f62` | +4: `0x110faa`, ID 54, center (+43,+81), radii 31×11. +9: `0x110fc2`, ID 55, center (+29,+97), radii 15×17. | +1, +3, +4, +9, +13, +17, +21, +25. The last pose is held through landing. |
| Old directional | `0x1a86fc` | +4: `0x1a872c`, ID 13, center (+30,+81), radii 31×11. | +2, +4, +24, +28, +32, +36. |
| New neutral | `0x110ada` | +4: `0x110b0a`, ID 12, center (+30,+81), radii 31×11. | +2, +4, +24, +28, +32, +36. |
| Old neutral | `0x1a83cc` | +4: `0x1a83fc`, ID 12, center (+30,+81), radii 31×11. | +2, +4, +24, +28, +32, +36. |

Forward and backward jumps use the same directional attack streams for a given loadout. In the 64 px forward-jump fixture against simultaneous neutral-jumping Ken, New hits twice for 21 + 4 HP and Old hits once for 19 HP. At 32 px, simultaneous neutral-jump MP hits once for 19 HP in both loadouts. These are sampled damage values. The backward-jump controls at 64 and 170 px whiffed in both loadouts.

## Continuous airborne reactions

The original `new-mp-2176-4-r1` sequence still produces 2+1 at Turbo 2. The first contact is the timing origin below. The defender state is read immediately before damage. Both fighters load 14 freeze on all three contacts. Ryu's counter reaches zero at +14 and Ken's at +15 after each contact in this sequence.

| Contact | From first hit | Damage | Ryu X/Y | Ken X/Y | Ken state before damage | Attack / relevant hurtbox |
| --- | ---: | ---: | --- | --- | --- | --- |
| First jump, first attack | 0 | 21 | 748/88 | 780/90 | Jump, state 4 | ID 54. Ken's jumping head ID 11 and torso ID 2. |
| First jump, second attack | +19 | 4 | 758/105 | 790/131 | Airborne reaction, state 14 | ID 55. Ken's remaining hurtbox ID 47, center (+19,+39), radii 23×26. |
| Fresh forward jump, first attack | +82 | 21 | 850/70 | 890/109 | Airborne reaction, state 14 | ID 54. Ken's remaining hurtbox ID 48, center (−8,+16), radii 50×15. |

The X positions are integer words, while Ryu also has a nonzero fraction. Ken’s combo byte increments at the same game tick as the second and third damage writes. Old with the same sequence records one 19 HP contact and no follow-up. At Turbo 1, the archived `new-l3s-2159-6-2213-8` recipe still produces three contacts at game ticks +0/+38/+104, using attack IDs 54, 55 and 54 on three distinct jumps. The same input recipe at Turbo 2 produces the first two contacts only. Those use IDs 54 and 55 at +0/+39, and Ken remains in airborne reaction for the second.

The first airborne reaction in `new-mp-2176-4-r1` raises Ken’s Y from 90 to 110 on initialization. The write at hit +1 includes 4 px of his remaining jump and the reaction adds 16 px. After Ken’s freeze clears, his next four advancing ticks move X by +3/+3/+2/+2 and Y by +5/+6/+5/+5. The reaction switches to ID 47 at the fourth advancing tick and ID 48 at the eighth. Following the second hit, Ken rises to Y 212 and descends to 109 by the fresh MP contact 48 advancing ticks later. The port uses the measured integer Y steps from that second reaction, the ID 47 and 48 rectangles and a floor check before the landing stream. It adapts the Y origin to the SF1 sprite grid and uses the native landing drawings.

A Local Versus 2+1 test reproduces New’s second strike at hit +19 and the fresh first strike at hit +81 with continuous airborne reaction and 14/14 freeze. The source comparison lands the fresh strike at +82. The Old control hits once. The pinned `new-one-start-52-2820-5` gives the World Turbo 1 1+2 reference at +0/+61/+81. A port controller fixture at 80 px produces the same strike order at +0/+53/+75. The pinned `new-l3s-2159-6-2213-8` gives the World Turbo 1 1+1+1 reference at +0/+38/+104, with the second strike of the second jump connecting. A port controller fixture at 96 px produces one first strike per jump at +0/+53/+107. Ken remains in the same airborne reaction in both port fixtures. The 1+1+1 source opener occurs late in the descending jump and the arcade lands 21 ticks after that MP begins. The port currently plays its full directional MP flight stream after a late input. Reproducing the source 1+1+1 strike order and re-jump timing remains open.

## Airborne Shinku follow-ups

Write taps in the World Turbo 1 replays at `9646b61` confirm the game-tick clocks below. The P1/P2 axes are arcade world X/Y at the health write. Hurtbox records give center X/Y offsets from Ken's axis in the recorded world orientation, then their radii. Ken is New in every row. [Pinned inputs](https://github.com/johnstreetfighter/sf1-port/blob/9646b61/docs/research/evidence/ryu-super-turbo/arcade-follow-up-inputs.tsv) · [Pinned display traces](https://github.com/johnstreetfighter/sf1-port/blob/9646b61/docs/research/evidence/ryu-super-turbo/arcade-follow-up-traces.tsv)

### Full-health 2+1+2

`new-hfull-20-2835-4-45` starts the jump sequence at display 2770 with Ryu X 807, Ken X 855, 144 Ken HP and full Super. Ryu queues forward jump and Ken queues neutral jump at 2770–2772. Ryu queues MP at 2777–2779, a fresh forward jump at 2835–2837 and MP at 2839–2841. The six `236236+LP` inputs occupy one display event each at 2880–2885. Ryu first reaches floor Y 40 on display 2884. The final input enters the Super at exact tick 175 and its projectile allocates at 203.

| Hit | Display · exact tick | Ryu X/Y · Ken X/Y | Ken state · HP after · combo after | Attack and reaction geometry |
| --- | --- | --- | --- | --- |
| First jump MP, first | 2781 · 41 | 830/89 · 862/90 | 4 · 122 · 0 | Attack ID 54. Ken jumping head 11, torso 2. |
| First jump MP, second | 2796 · 60 | 840/105 · 872/131 | 14 · 118 · 1 | Attack ID 55. Ken ID 47 `(19,39,23,26)`. |
| Fresh jump MP, first | 2843 · 120 | 935/75 · 967/130 | 14 · 97 · 2 | Attack ID 54. Ken ID 48 `(−8,16,50,15)`. |
| Shinku, first | 2910 · 203 | 952/40 · 984/78 | 14 · 83 · 3 | Shot X 1045, Y 94. Ken ID 48. |
| Shinku, second | 2919 · 213 | 952/40 · 984/87 | 14 · 69 · 4 | Shot X 1050, Y 94. Ken ID 47. |

MP ID 54 has center `(+43,+81)` and radii `(31,11)`. ID 55 has center `(+29,+97)` and radii `(15,17)`. Both Shinku contacts use shot pose `0x7434e`, ID 2 with center `(−16,0)` and radii `(31,13)`. The projectile loads freeze 10 and Ken loads 14 on each contact. Ken is against the right boundary and stays in airborne reaction through both hits. In the separate `new-juggle-super-4234` recipe Ken starts with 41 HP. At Turbo 1, MP hits at ticks 12 and 31, then Shinku hits at 133, 143 and 153, ending the round. The first and second Shinku contacts see Ken at X 984 and Y 58/64, with reaction IDs 48/47 and the same ID 2 projectile.

The fresh MP also enters its second strike pose at tick 139, display 2858. Its box overlaps Ken's ID 47 both when the pose is written and at the end of the game tick, when their world box centers are `(975,193)` and `(997,200)`. Ken is still in state 14 with zero freeze, combo byte 2 and airborne hit count 3. HP stays 97 through the rest of the strike.

At Turbo 2 the full-health recipe with the same display inputs gives MP hits at ticks 61, 80 and 142. Shinku starts at 198 and allocates at 226, with no further damage. This gives a failure control for the full-health sequence and shows that the cabinet speed changes the relationship between the recorded display inputs and the airborne trajectory.

The port's full-health Local Versus fixture starts 32 px apart. The three MP strikes occur at +0/+19/+73 and Shinku adds two contacts at +155/+167, with continuous Ken airborne reaction. The World Turbo 1 reference is +0/+19/+79/+162/+172. The port snapshot taken after the first Shinku hit replays the second hit at the same tick and reproduces the final checksum. In a separate port control, an earlier Super motion fails to enter Shinku. Another port input timing gives only one Shinku contact after the same three MP hits.

### One- and two-hit Shinku into fresh MP

The pinned `new-sff-35--17-2842-4` and `new-sff-35--18-2842-4` recipes begin at Ryu X 837 and Ken X 869 before the Super. Ryu queues the six `236236+LP` events in two-display holds from 2770–2781, then fresh forward jump at 2842–2844 and MP at 2846–2848. Ken queues Up at 2753–2755 in the first recipe and one display earlier in the second. In both recipes the Super starts at exact tick 38 and its projectile allocates at 66.

| Route | Exact damage ticks and Ken life | Fresh MP contact | Fresh MP second pose |
| --- | --- | --- | --- |
| One Shinku hit → MP | 68: 128. 119: 106. | Display 2850, Ryu 854/71, Ken 931/113 in state 14 with ID 48. | Display 2865, tick 138. Ryu 870/93, Ken 939/142 with ID 47. X center separation 59 px, combined X radii 38. |
| Two Shinku hits → MP | 67: 129. 77: 114. 120: 93. | Display 2850, Ryu 854/71, Ken 927/134 in state 14 with ID 48. | Display 2865, tick 139. Ryu 870/93, Ken 935/164 with ID 47. X center separation 55 px, combined X radii 38. |

The two reverse fixtures show the second MP attack pose horizontally short of Ken. The same two recipes at Turbo 2 each give three Shinku hits and no MP contact. The state and hurtbox sequence distinguishes the follow-up miss from a failed Super input.

In the port's 32 px one-hit fixture, Ken queues a neutral jump 27 ticks before the Super input. Shinku hits him on descent at entry +28 and the fresh forward MP hits at +82. World Turbo 1 gives entry +30/+81. The port replays the follow-up from a snapshot after the shot. The 60 px port two-hit fixture has Shinku contacts at entry +28/+38 and a fresh MP at +83. A later jump and MP input in that fixture misses.

The raw tap replay uses the full archived input schedules and default World Turbo 1 EEPROM. The Turbo 2 control changes selector and skip threshold at display 2070. Record Ken's life and combo writes, both fighters' animation pointers and axes, and the projectile header and box at each contact. Scratch runs are under `%TEMP%/ssf2-ryu-audit/st-air-shinku-taps`, with `arch1-` and `arch2-` prefixes on the archived fixture IDs.

### World ROM airborne hit gate

The World 940223 program calls the same gate at `0xBE708` for melee from `0xBE5C0` and projectiles from `0xBEB5E`, before checking rectangle overlap. The gate applies when the defender is in state 14 at `+0x03` with reaction `0x3e`, `0x44` or `0x48` at `+0x55`.

| Incoming attack type at attack record `+0x0b` | Defender count at `+0x2ac` required to pass |
| --- | --- |
| `0x3e`, `0x4a` | Below 3 |
| `0x42`, `0x44`, `0x46`, `0x48` | Below 5 |
| Other | Rejected during these reactions |

The opcode at `0xBF16A` increments the airborne hit count. The full-health route changes it from 0 through 3 on the three directional MP hits, then to 4 and 5 on the two Shinku hits. The rejected overlapping MP pose sees count 3 before Shinku. The two-hit Shinku opener increments to 2 and its fresh MP increments to 3. The one-hit Shinku opener increments to 1 and its fresh MP increments to 2. The reset at `0x7955C` clears the count as Ken exits the reaction in the two reverse fixtures. Reproduce the increments and clears with a write tap on Ken’s `+0x2ac`, recording the old and new byte, main CPU PC, display sample and game tick. [Capcom SFII air combo seminar](https://game.capcom.com/cfn/sfv/column/132437?lang=en)

In the three airborne recipes above, both directional MP boxes use incoming type `0x4a` and produce reaction `0x3e`. Each of the five sampled airborne Shinku contacts uses incoming type `0x42` and produces reaction `0x44`. The input type writes at `0xBEFBC` to defender `+0x4b`, then the reaction handler writes the resolved value there. The resolved value is copied to `+0x55` on the next game tick.

For both reverse fixtures, the first projectile contacts at Ken X869.75, Y57.0625 (floor Y40). One tick later residual jump motion brings Ken to Y50.75, then the reaction adds 16 px and holds him at Y66.75 through freeze. The second projectile in the two-hit fixture connects ten ticks after the first, while Ken is still at X869.75, Y66.75. Its reaction adds another 16 px at contact +1. The isolated first hit clears freeze at +15, and the second hit in the two-hit route clears it at +14. The first motion is the following tick. The reaction initializer at `0x7991E` loads 8.8 motion constants, confirmed in the working registers after the freeze-clear routine. Motion adds +2 px horizontally every tick, begins vertically at +6 px/tick, and subtracts 0.28125 px/tick after each advance. The observed integer Y peaks are 127 and 143 at their 19th advancing tick. The later MP interrupts both descents.

A separate World Turbo 2 control uses LP Shinku from 90 px against standing Ken. Its incoming type is `0x42` for the first three contacts, which resolve to grounded reaction `0x38`, then `0x14` for the fourth and `0x3c` for the fifth. The airborne hit count stays at zero in this grounded sequence.

### v1.5.15 airborne Shinku in the port

The port uses a separate three-hit airborne limit for New Ryu's directional MP and a five-hit limit for the tested Shinku reactions against New Ken. The directional MP can still hit twice per action. A qualifying hit starts or refreshes New Ken's airborne reaction, and the running count clears when he leaves it. A Shinku rehit leaves Ryu advancing and reloads Ken's ordinary freeze. The port uses the measured 8.8 Shinku trajectory with the 1987 art origin.

In the port's reverse controller fixture at 60 px, Ken queues a neutral jump at Super +2 and takes Shinku hits at +28 and +38. From that state Ryu's fresh forward jump at +70 and MP at +77 hit at +83 while Ken is still airborne. A later jump and attack miss. The replay restores a snapshot after the first projectile hit and reproduces the rehit checksum. In the forward controller fixture, the port's 2+1 jumping MP route leads to a fourth airborne hit from Shinku at +157 relative to the first MP. The source 2+1+2 route and the one-hit reverse fixture still need matched port results.

## Air Tatsumaki cancel

The archived `new-jmp-tatsu-2118-hk1` and `old-jmp-tatsu-2118-hk1` sequences reproduce a first jumping MP hit at Turbo 2 followed by Air Tatsumaki state 12 at hit +3. The first active kick pose is written at hit +15, which is +12 from the special transition. The kick misses in both runs. The first active rectangles are shared within each loadout's forward and backward kick phase except for the sign of the center. New attack IDs 52/53 center at (±54,+67) with radii 15×16. Old IDs 52/53 center at (±35,+67) with radii 34×16.

The original New grounded Ken LP and HP sequences also activate Air Tatsumaki after contact at Turbo 2. The first hits are 11 HP for j.LP and 22 HP for j.HP. State 12 starts at hit +2 and +3 respectively, and both kicks miss. The archived New grounded j.MP recipe still lands 22 HP at Turbo 2 but does not activate the special at this input phase. In Old’s two archived corner controls, the special itself hits for 22 HP after the earlier jumping MP misses. These controls verify an active Air Tatsumaki hit.

A controlled air-to-air check tested both loadouts, five starting gaps from 32 through 96 px, LK and HK, and three input delays. The three delays started Air Tatsumaki at first hit +1, +4 and +6 in New, and +1, +4 and +7 in Old. Every one of the 60 air-to-air runs had a first jumping MP hit and a subsequent special activation. The kick dealt no further damage in those runs. LK shares the measured HK box geometry in both loadouts and uses attack IDs 50/51. Against grounded Ken at 72 px, Old also cancels a 26 HP directional j.HP into Air HK at +2, with no second hit.

### Late input, guard and whiff

At 64 px against jumping Ken, both loadouts accept Back+HK at first MP hit +15 and reject +16. Three New phase repeats delivered +16 and kept the second j.MP strike at +19. The successful +1 and +15 New inputs both write the special state on the input tick. Ryu’s X and Y stay fixed through hit +14. The first active kick and the first post-freeze movement occur on hit +15 in both runs. At 128 px against grounded Ken, the matched New late j.MP samples hit or block with ordinary 14/14 freeze. Neither setup accepts Air HK at contact +2, +4, or +7. The tested height and pose conditions were the same in the clean and guard runs. New j.LP and j.HP and Old j.HP against grounded Ken also load 14/14, with Ryu’s counter clearing at +14 and Ken’s at +15.

At 232 px against jumping Ken, both MP no-kick controls whiff. A final Back+HK at first active MP pose +4 (New) or +3 (Old) starts Air Tatsumaki from jump-attack state `4`/substate `6`, with no earlier damage or contact freeze. New’s kick misses and Old’s hits for 22 HP as the first damaging contact. The tested later inputs at first active pose +8 and +15 for New and +7 and +13 for Old produce no special. The alternate New 190 px whiff control also rejects its early sampled +5 input. These source fixtures show activation during an early whiff and leave the exact phase boundary open.

### Standalone Air Tatsumaki phases

The 18 no-contact trials cover both loadouts, all three strengths, and forward, neutral and backward jumps. Every trial writes the first active kick pose on the same game tick as the transition to special state `12`. Use that write as +0 for the half-open intervals below.

| Kick | Forward attack windows | Rear attack windows | Forward / rear IDs | First spin recovery write |
| --- | --- | --- | --- | ---: |
| LK | [0,3) | [6,9) | 50 / 51 | +11 |
| MK | [0,3), [12,15) | [6,9), [18,21) | 48 / 49 | +23 |
| HK | [0,3), [10,13), [20,23) | [5,8), [15,18), [25,28) | 52 / 53 | +29 |

All strengths use center (±54,+67) and radii 15×16 in New, and (±35,+67) and radii 34×16 in Old. The forward jump controls land at +42/+51/+54 for LK/MK/HK in both loadouts, then write the second landing pose four ticks later. The floor Y write and neutral state write identify the exact endpoints even when an emulator frame samples two game ticks.

| Jump | New floor → neutral, LK/MK/HK | Old floor → neutral, LK/MK/HK |
| --- | --- | --- |
| Forward | +42→47 / +51→56 / +54→59 | +42→47 / +51→56 / +54→59 |
| Neutral | +43→48 / +52→57 / +55→60 | +42→47 / +50→55 / +54→59 |
| Back | +44→49 / +52→58 / +56→61 | +43→48 / +51→56 / +55→60 |

The fixed-point forward traces record the full 16.16 X and Y. At 190 px, both loadouts activate at X 589.2109375 and Y 96.375. The first step is 4.265625/4.765625/5.265625 px horizontally for LK/MK/HK and 3.4375 px vertically. The first active pose sees Y 99.8125. At spin recovery, X/Y is 639.109375/129.375 for LK, 698.1953125/144.375 for MK, and 738.68359375/145.125 for HK. Neutral begins at +47/+56/+59. The initial speed changes with the jump phase at activation. At 384 px the same inputs put New and Old at different jump phases. New activates at Y 92.8125 and steps up 3.75 px. Old activates at Y 99.625 and steps up 3.125 px. The horizontal step decreases by 0.01953125 per advancing tick while unobstructed. The vertical step decreases by 0.125 per tick during the spin, then by 0.3125 from +12/+24/+30 for LK/MK/HK until the ground clamps Y. Pushbox contacts and camera bounds can alter horizontal travel.

The directional traces also retain X fractions. Neutral-jump LK/MK/HK move forward by 1/1.5/2 px per advancing tick in the samples. New's backward-jump LK/MK/HK start at −4.765625/−5.265625/−5.765625 px per tick while Ryu still faces forward. The next step is 0.01953125 px closer to zero. Old's first displayed backward sample contains two game ticks. Its next visible step matches New's second step for each strength. The port rounds the source velocity to an integer step for each animation record, uses the forward fixture's initial vertical velocity and keeps its existing activation-height adjustment. Landing schedules are sampled fixture values. Exact fractional displacement and landing for arbitrary activation heights remain open, as do the special's hit count and reaction against a continuous juggle.

## Replay

The scratch runs are under `%TEMP%/ssf2-ryu-audit/st-jump-mp`. For an archived recipe, export its rows from `arcade-follow-up-inputs.tsv` at `9646b61` into `events.txt` without the recipe column. Preserve the default EEPROM for Turbo 1. For Turbo 2 write `0xff83a9 = 2` and `0xff8cd3 = 0x60` at frame-end sample 2070. Preserve the character positions produced by the archived inputs.

For controlled geometry, use the same setup and set Ryu X to 552 and Ken X to 552 plus the target gap, with zero horizontal fractions at sample 2070. Queue Up-Forward, Up-Back, or Up at `2110…2112` and Ryu MP at `2117…2119`. For the neutral-jumping Ken fixtures, queue his Up at `2110…2112`. Use 64 px for the forward air contact, 170 px for New forward whiff, 64 px for Old backward whiff, and 130 px for neutral whiffs. [Hitbox table layout](https://github.com/peon2/fbneo-training-mode/blob/master/hitboxes/st-hitboxes.lua)

For the air-to-air cancel checks, use forward jump and simultaneous Ken neutral jump. Start the quarter-circle back on `N = 2117`, `2119`, or `2121`. Hold Down on `N…N+1`, Down-Back on `N+2…N+3`, and Back+LK or Back+HK on `N+4…N+5`. The Old grounded j.HP case uses 72 px, Up-Forward on `2110…2112`, HP on `2113…2115`, no Ken input, and the same motion at `N = 2114` with HK.

For the connected +15 and first late checks, use gap 64 and HK with `N = 2128` and `2129`. With the unshifted inputs, `N = 2129` gives Old +16 and New +17. To reproduce New +16, shift both jumps, MP, and the `N = 2128` motion one display sample later. For the 232 px whiff success use `N = 2117`. For the matched 128 px grounded New controls, jump forward on `2110…2112`, hold MP on `2142…2144`, and use `N = 2142`, `2144` or `2146` with HK. Ken holds Back on `2141…2164` in the guard runs and stays neutral in the clean controls. For standalone strength traces, use gap 190, jump on `2110…2112`, omit the normal, and execute the quarter-circle-back motion at `N = 2118`.

Timestamp writes with the game tick byte `0xff801d`. Record attack and reaction state, first damage, both freeze counters, X and Y, animation pointer, and Ken’s combo byte `+0x1f4`. At each life decrement resolve the attack and hurtboxes from the active animation and the object’s hitbox table pointer `+0x34`. For the forward MP route, the positions at the life write precede any later attacker X update on the same game tick.
