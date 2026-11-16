# Ken Tatsumaki in World Super Turbo 2

World 940223 in MAME 0.289, cabinet Turbo 2. P1 is New Ryu, P2 is New or Old Ken. See the [verified ROM and selection recipe](ken-super-turbo-audit.md#reproduce-the-source-captures). The clock counts advancing game ticks from the state-12 write. It can advance more than once per display.

## Ground Tatsumaki

Both loadouts share the measured trajectory and spin clocks. At entry Ken is on the floor. The opening kick is active at +1 through +3. The spin begins at +11. Each seven-tick cycle has the forward box on cycle ticks 0–1 and the back box on 3–4. The opening uses strike ID 48 with facing-local center (18, 52) and radii (21, 15). The spin uses centers (35, 67) and (−35, 67), both with radii (34, 16). Add Ken's height to those Y centers.

| Kick | Spins | Unclipped travel | Full floor / stance on whiff | 55 px clean contacts / stance | 55 px crouch block / stance |
| --- | ---: | ---: | --- | --- | --- |
| LK | 3 | 107.5 px | +44 / +50 | +1, +39 / +78 | +1 / +64 |
| MK | 4 | 156.25 px | +51 / +57 | +1, +32 / +85 | +1 / +73 |
| HK | 5 | 213.75 px | +58 / +64 | +1, +32, +67 / +106 | +1 / +80 |

Every strength rises 23 px. Horizontal velocity is 2.5, 3.125, or 3.75 px per advancing step, respectively. The first descent tick has zero travel before the original speed resumes. On the 55 px clean fixture each recorded strike does 11 HP in phase zero. MK and HK sometimes roll 13 HP on the second hit in phase one. The opening does 2 HP through crouch guard. At 90 px with Back held from two displays before the motion finishes, phase zero gives LK chip at +11, MK at +11/+32, and HK at +11/+32/+67. Those spin contacts cost 2 or 3 HP each, depending on the hit. At 40 px clean, LK's second hit moves to +32 and HK's third moves to +60.

Each sampled clean or guarded hit loads 14 freeze on each fighter. Ken's byte clears at contact +14 and Ryu's at +15. A settled ground reaction first moves Ryu away from Ken at +16 and travels 78 px through +35. The first ready write is +36. The 55 px MK second hit arrives during the first reaction. The corresponding LK hit and the sampled HK third hit arrive outside the preceding reaction. Body overlap and crossover can affect the last part of the guard route.

The middle and heavy second clean strikes at +32 meet Ryu's late recoil head ID 26. In his facing-local coordinates its center is (−4, 80) with radii (10, 10), higher than his initial heavy recoil. The +67 heavy third strike meets the standing hurtboxes. [Contact hurtboxes](ken-super-turbo-tatsumaki-hurtboxes.tsv)

The v1.5.24 port uses the three source ground trajectories, their spin cycles, and one contact allowance per cycle. The ROM-backed replay checks both loadouts at 55 px against New Ryu, with room for recoil. It verifies clean and crouch-block contact ticks and damage, the 14-tick freeze write on both players, whether each hit combos, a 78 px recoil control, and rollback from before a second hit. A 240 px control verifies whiff landing, peak height and return to stance. The port uses 11 HP per clean hit and the opening's 2 HP chip, and rounds the trajectory to the nearest port pixel. The source's damage rolls and later spin-chip variability, late movement against walls, and more defender profiles still need paired checks. [Controller regression](../../crates/sf-engine/tests/sf2_native.rs)

## Air Tatsumaki

Both loadouts can activate from neutral, forward and backward jumps. Forward jump into an early spin rises again on the existing velocity. A late forward-jump entry can hit on its first game tick and land early. At 118 px in phase zero, pressing the final kick 25 displays after the jump starts gives New two clean contacts at +7 and +24. The same trial gives Old one hit at +10. Phase one changes this comparison, so the files keep each input phase and entry position. At 164 px with Ryu neutral jumping, the source captures a clean airborne hit. The forced-whiff back-jump recipes reach the edge. Their recorded total X travel includes the clamp.

At display 2070, put New Ryu at X552 and Ken at X552 plus the named gap, with both X fractions zero and the configured Turbo 2 state. Set `N=2150` or `2151`. Queue Ken Down on `N−8…N−5`, Down-Back on `N−4…N−1`, and Back plus the named kick on `N…N+2`. For an air route, add its jump direction at `N−lead…N−lead+2`. Keep Ryu neutral for standing hit. Hold Down or Down-Back at `N−11…N+82` for crouch or crouch guard, Back alone at `N−2…N+82` for standing guard, or Up at `N−4…N+1` for the air target. For a forced whiff, pin Ryu at Y600 from displays 2100–2250. The movement and contact exports preserve 16.16 X, pre-write health and the state, position, freeze and life write clocks. A blank special entry means the input did not enter state 12. At small gaps a side switch can change the expected direction before the final kick, and very late input sometimes selects a grounded command kick after landing.

[Capture recipes and results](ken-super-turbo-tatsumaki-cases.tsv) · [Forced-whiff trajectories](ken-super-turbo-tatsumaki-flight.tsv) · [Contact writes](ken-super-turbo-tatsumaki-contacts.tsv)
