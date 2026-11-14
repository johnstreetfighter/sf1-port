# Ken throws in World Super Turbo 2

Source captures for [#153](https://github.com/johnstreetfighter/sf1-port/issues/153) use **Super Street Fighter II Turbo, World 940223** (`ssf2t`) in MAME 0.289. New Ryu is P1 and New or Old Ken is P2. Both are grounded and idle at X650 and X688, floor Y40, in the first ground fixture. These timings describe the source setups.

## Full throws

Hold Forward or Back and press the indicated button for one display sample. Both loadouts use Shoulder Throw for MP or HP and Hell Wheel for HK. New Ken uses Knee Bash for MK. Old Ken uses Hell Wheel for MK. Each close input was repeated at two consecutive display phases, with matching game-tick results in the table.

| Ken input | Release | Damage on New Ryu | Ken ready | Ryu ready |
| --- | ---: | --- | ---: | ---: |
| Either loadout, Forward + MP or HP | +44 | 32 at +79 | +74 | +153 |
| Either loadout, Back + MP or HP | +44 | 32 at +85 | +74 | +159 |
| Either loadout, Forward + HK, or Old Forward + MK | +62 | 32 at +95 | +109 | +156 |
| Either loadout, Back + HK, or Old Back + MK | +62 | 32 at +101 | +109 | +162 |
| New Forward or Back + MK | +179 | 26 at +14, then 4 each at +48, +82, +116 and +150 | +179 | +229 |

Tick zero is the write of `$ffff` to the defender's throw word. Release is its first return to zero. Ready is the first fighter state write to zero after the throw. HP is source life out of 144. Forward Hell Wheel moves Ken from X688 to X546, with the defender ending at X296. Back Hell Wheel moves Ken to X808, with the defender ending at X984. In the forward Shoulder Throw Ken stays at X688 and puts Ryu at X391. Back Shoulder Throw finishes at Ken X681 and Ryu X977. These are the sampled midstage trajectories. The earlier forward Hell Wheel trial in the [Ryu throw audit](ryu-st-turbo2-throw-evidence.md#softening-and-priority) reported first damage at +79. A comparison with the earlier stage setup is still needed to explain the landing difference.

The source horizontal anchor influences the fall. At attacker X600 or X650, Back Shoulder Throw reaches damage +79 and ready +153, matching the forward throw. At X688 the camera constraint pauses that path and adds six ticks. Forward Hell Wheel at X720 or X752 reaches defender ready +168, versus +156 at X688. The port uses midstage clocks, representative trajectories from the source, and its native screen constraints. Its forward Hell Wheel uses damage +95 and ready +168, and its backward Hell Wheel uses damage +101 and ready +162 from the measured camera-bound branch. The unconstrained backward landing clock and dynamic corner delays need a wider arcade sweep. [Per-tick ground capture](ken-super-turbo-ground-throw-trajectory.tsv)

### Knee Bash hold

Both input directions show the same damage and release clock when the attacker and defender are idle after the command. The full hold does 42 HP. A single Forward + MP response at the sampled points from +8 through +17 leaves the damage and release clock unchanged. Alternating directions and neutral on the defender moved release to +79 or +82, with two damage packets. Alternating Ken's directions advanced the knee cycles, producing six packets and release at +191. With both players mashing, later actions can obscure the first free frame, so use the release write as the hold boundary. Repeat the direction sequence Down, Left, Up, Right, holding each direction two display events and leaving one neutral event between them. In the phase-zero capture, the first pulse starts at `N+3`.

The hold routine at `$08613c` starts a 120-update counter and seeds randomized defender/attacker budgets by `$0052a0/$00528c`. The phase-zero writes load 42 for Ryu and reset Ken to 12. Ken's initiating command debits four from his initial budget. The input decoder at `$00534a` debits three for direction from neutral and one for a fresh attack edge. Defender budget below zero releases the hold. When Ken's budget goes below zero, the routine reloads it and moves his current animation to its next transition. Each knee hit pauses the hold countdown for 12 ticks. The port uses the observed starting budgets, the measured neutral tap cycle, and the measured input debits. Source random budget variation and the exact interrupted release animation are open. [Additional control captures](ken-super-turbo-ground-throw-controls.tsv)

## Range and softening

The range sweep sampled MP, MK and HK in both directions and loadouts at presampled axis gaps of 76, 77, 78, 80 and 84 pixels. Every pair captures at 76 and 77 and fails at 78, 80 and 84. The range rows have display samples confirming the flags. Their capture coordinates and game-write columns are blank because the write tap was unavailable in that sweep.

For a forward Shoulder Throw or Hell Wheel, New Ryu's Forward + MP sampled at +8, +12 or +13 softened either loadout's throw. New and Old give full damage with the first verified late response at +14. Against the backward throws, New Ryu's Forward + MP and Back + MP both soften at +13. A softened Shoulder Throw does 16 HP at +44 and Ryu is ready at +100 in either direction. Softened Hell Wheel does 16 HP at +62, and Ryu is ready at +117 for the forward throw or +122 for the backward throw. Ken's neutral clock stays at +74 or +109. Old Ryu's corresponding +8 and +13 inputs left both directions at full damage. When New Ken presses MK+HK together, MK's hold wins. A sampled MP+MK or HP+HK uses Shoulder Throw.

## Air Throw

New Ken captured airborne New Ryu with Forward or Back plus MK or HK. Pressing a punch or using neutral plus MK or HK in the tested jump phases gave an ordinary air attack. The grounded Ryu control also failed to capture. The first 25 distinct successful captures selected `0x131ff4`, released at +46 and returned Ken to neutral at +84. Ryu lost 32 HP at +57 through +69 in those low-X screen-edge trials. At capture, their axes were 31–32 px apart.

Old Ken's matched jump and button recipes gave ordinary air attacks. This agrees with contemporary player reports describing Air Throw as a New Ken move. The earlier `2119…2121` Forward + HK recipe deals 26 HP in both loadouts through an ordinary jump kick in state `4`, substate `6`, with both throw flags zero. The new capture's explicit flag writes establish the distinction. [Contemporary New/Old comparison](https://archive.supercombo.gg/t/ssf2t-old-character-differences-list/157)

For a positive fixture, set X552 and X600 at display event 2070. Queue Ryu's neutral jump and Ken's forward jump at `2110…2112`. Queue either direction and MK or HK for three display events starting at `2122` or `2125`. The original and follow-up sweeps vary the input phase, buttons, direction, initial gap and whether Ryu jumps. Offsets in the TSV are relative to display event 2119. [Air Throw capture summaries](ken-super-turbo-air-throw-writes.tsv)

### Air Throw eligibility and softening

The New Ken airborne MK and HK handlers at `$085e58` and `$085e92` require a live horizontal direction and call the shared throw selector at `$00460c` with Ken throw rectangle index 2. The selector at `$085ecc` tests Ken's height above floor Y40 against 56 px while vertical velocity is nonnegative and 48 px while it is negative. Equality qualifies in both cases. An early button plays a jump kick if the height test fails.

Index 2 loads `Ken+$64…+$6a` with a center 24 px forward and 53 px above his axis and radii of 24 px horizontally and 16 px vertically. The left-facing capture has raw box X −24. At `$00471c` and `$00473c`, the shared routine rejects a difference beyond the sum of horizontal radii or beyond Ken's vertical radius. It uses the defender's push width at `+$6c` and vertical offset at `+$6e`. New Ryu's sampled neutral-jump values are 29 and +53 px. The resulting spatial gate for this defender pose is inclusive at ±53 px horizontally around Ken's forward box center and ±16 px in axis height. The shared routine also rejects incompatible defender states, empty hurtboxes and existing throw lockouts. The exact mixed-era target posture mapping remains open.

New Ryu can soften the throw with a live Forward or Back plus a fresh MP, HP, MK or HK edge through capture +13. The first rejected tick is +14. LP and LK fail at +12 and +13. The source runs its victim input check at `$0047d4`, increments `Ryu+$2bc` while it is below 14 and latches `Ryu+$2bd` on a qualifying edge. The state-release routine at `$004c22` applies half damage on the release tick after a successful response. The check bypasses Old human defenders.

### Midstage trajectory

Both button directions follow the same throw direction in the matched replays. At event 2109 set New Ryu X720, New Ken X768 and both fractions to zero. Queue Ryu neutral jump and Ken forward jump from 2110 through 2112. Queue Ken Forward or Back with HK for three events starting 2123 for phase 4 or 2126 for phase 7. The captured integer Y values are 104/99 for Ryu/Ken in phase 4 and 113/109 in phase 7. For the softened branch, queue Ryu Forward + MP at event 2128 or 2131 respectively. These are initial source setups. The positions at capture still include the jumps and separation.

| Branch | Both throw flags clear | Ryu first lands | Ryu damage | Ken ready | Ryu ready |
| --- | ---: | ---: | --- | ---: | ---: |
| Full | +46 | +79 | 32 HP at +79 | +84 | +153 |
| Softened | +46 | +95 | 16 HP at +46 | +84 | +101 |

At release, Ryu is at height 32 in all eight runs. Ken reaches the floor by +45. Ken's X shift from the capture origin is about −112.19 px in phase 4 or −106.32 px in phase 7. Ryu's release shifts are −204 and −197 px. After release, the full fall moves a further 112 px before its first floor contact. The softened branch moves a further 96 px before landing and returns to standing six ticks later. The raw selected animations, signed 16.16 X offsets, heights, state/substate, life and throw flags are preserved for every captured game tick. Offsets use each fighter's separate pre-capture axis and positive height above floor. The +0 row is the end of the capture update, so it already includes attachment displacement. The eight source runs are under `%TEMP%/ssf2-ryu-audit/ken-air-throw-precise2-*`. [Midstage Air Throw trajectories](ken-super-turbo-air-throw-trajectory.tsv)

### Port replay

Local Versus checks both Ken facing directions, Forward or Back with MK or HK, Old Ken's fallbacks, and the grounded, neutral, punch and light-kick controls. The port fixture gives Ryu's neutral jump a five-tick head start because its jump height develops later than Ken's directional jump. The resulting spacing and height satisfy the arcade catch rectangle. Both grab flags release on +46, Ken is ready by +84, and the victim's full and softened damage and ready clocks match the midstage source routes. The profile takes Ken's pre-capture axis as a shared origin, interpolates the two measured start heights, and uses nearest-pixel placement with resident SF1 poses. Arcade-equivalent jump arcs and Air Throw opponents beyond Ryu remain open. [Local Versus regression](../../crates/sf-engine/tests/st_ken_air_throw.rs)

## Reproduction

Use the [Ken selection recipe](ken-super-turbo-audit.md#reproduce-the-source-captures) with World Turbo 2 and skip cadence byte `$60`. Set both fighter X axes at display event 2160 and leave both players neutral until the throw input. At `N=2200` or `2201`, press P2 Left for Forward or P2 Right for Back and the tested button. For the forward softening sweep, press P1 Right + MP for one display event at `N+6`, `N+9`, `N+10`, `N+11`, `N+12` or `N+13`. For the back throw follow-up, use P1 Right or Left + MP at `N+10` or `N+11`. The arcade polls on its advancing game clock, so use the input write to determine the actual response tick. Tap throw `+$62`, life `+$2a`, fighter state `+$02`, and input `+$392`. The summary retains the selected animation from the first display sample with both throw flags set. `hold` and `backsoft` rows extend the capture through display 2410. [Source capture summaries](ken-super-turbo-ground-throw-writes.tsv)

## Port v1.5.22 check

Local Versus using the canonical Super Turbo cards now has Ken's three grounded throws against Ryu and the New Ken Air Throw against airborne Ryu. The tests feed the ordinary controller path, verify the 77/78 grounded boundary, both directions and loadouts, first and last softening results, exact normalized damage, release and action clocks, and deterministic snapshot replay. The tap Knee Bash test checks all five packets and the directional mash tests check earlier release with fewer hits for Ryu's input and an extended sequence for Ken's. The airborne tests verify both facings and kick strengths, grounded and Old Ken fallbacks, and exact recovery for both outcomes. [Ground tests](../../crates/sf-engine/tests/st_ken_throws.rs) · [Air tests](../../crates/sf-engine/tests/st_ken_air_throw.rs)

The port uses its resident fighter drawings, representative integer ground paths and two captured launch heights for the Air Throw position profile. The engine's existing jump arcs require a five-tick Ryu head start in the paired port fixture. Exact stage-edge travel and recovery, additional defender profiles, simultaneous air capture priority and the arcade RNG variation remain open. A source control against New Sagat captures with all three ground throws at a gap of 78 px. At 38 px, forward Shoulder Throw damages at +79 and Sagat recovers at +144. Use the distinct Sagat range and recovery measurements for his eventual port profile. [Sagat defender controls](ken-super-turbo-throw-sagat-controls.tsv)
