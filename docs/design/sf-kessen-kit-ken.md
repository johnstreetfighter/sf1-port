# SF KESSEN kit sheet: Ken

Status: approved by the maintainer on 2026-11-18 (KS12, #229). Not built.

Each line says where it comes from: **blueprint** (Clayton Chapman's design, as restated in the [design document](2026-11-18-sf-kessen-era-style-design.md)), **ST** or another era style of Ken's that already has the move in the game, or **ours** (this project's choice where the blueprint is silent). Numbers are starting values; the playtest may change them.

## 1. What he is for

Ken wins by pressure and routing. His command kicks carry him forward, and what they lead to depends on the body in front of him and on whether that body is standing or crouching, so his best damage comes from reading the victim, not from one fixed combo. *(blueprint)*

What he gives up: his fireball is the weaker of the two shotos' and his pokes are shorter than Ryu's, so he cannot win a spacing match. His strongest routes cost stocks and change by hurtbox tier, so they are harder to land than Ryu's. *(ours, from the blueprint's contrast with Ryu)*

## 2. Body

- **Hurtbox tier:** medium. *(ours; the same body as Ryu)*
- **Wall jump:** no. *(blueprint: agile fighters only)*

## 3. Normals

All from **ST** (New Ken), which already has them with measured boxes.

| Group | Moves | Notes |
|---|---|---|
| Standing, close and far | Six buttons, each with a close and a far version chosen by distance | Standing heavy punch is the link target after a Hurricane Kick restand (section 10). |
| Crouching | Six buttons | Crouching light kick is the link target after a command kick. Heavy kick sweeps. |
| Jumping | The ST jump attacks | No two-stage jump attack; that is Ryu's. *(ours)* |

**Command kicks** *(ST; the maintainer chose these three, 2026-11-18)*:

| Move | Input | Notes |
|---|---|---|
| Kama Barai Geri | quarter-circle forward + kick | Drifts forward. |
| Nata Otoshi Geri | forward, down-forward, down + kick | Drifts forward. Overhead. |
| Osoto Mawashi Geri | half-circle forward + kick | Drifts forward, the longest of the three. |

Rules for all three:

- **Link into crouching light kick by tier** *(blueprint)*. Starting values *(ours)*: against a large body all three link; against a medium body Kama Barai and Osoto Mawashi link; against a small body only Kama Barai links.
- **The ST heel follow-up is left out** *(ours)*. Its place is taken by the kara cancel and the EX move below, so he does not get the same extension twice.
- Alpha's Inazuma Kakato Wari is not in the kit, except as an EX ender (section 6).

**Kara cancel, once per combo** *(blueprint)*: the last frames of a command kick, on hit, may be cancelled into the **Dragonlash Kick** *(SF6; the maintainer chose it, 2026-11-18)*. Input: quarter-circle back + kick during those frames *(ours)*. It is a flame move, so it keeps the victim's posture. It is not available on its own, only as this cancel.

## 4. Target combo

**Close standing medium punch into standing heavy punch.** *(ours, after his SF3 target combo; both halves exist in ST, the link is new. The blueprint names none for Ken.)*

- Press heavy punch during close medium punch. It comes out on hit or on block, not on whiff.
- It cancels into any special, and into the EX move only through a command kick.
- It counts as one move for juggle rules.

## 5. Specials and Supers

| Move | Input | Source | Notes |
|---|---|---|---|
| Hadoken | quarter-circle forward + punch | ST | Stands a crouching victim up. Slower to recover than Ryu's. Its first frames may be kara-cancelled into either Level 3. |
| Shoryuken, light and medium | forward, down, down-forward + punch | ST | Invincible on the way up. One hit against an airborne victim. |
| Flaming Shoryuken, heavy | forward, down, down-forward + heavy punch | ST move, Alpha's flames | Three hits. See section 10 for its posture rule. Only the last hit juggles. *(blueprint)* |
| Hurricane Kick | quarter-circle back + kick | ST, blueprint rules | The heavy version flies over medium and large crouching bodies and hits from behind. See section 10. |
| Air Hurricane Kick | quarter-circle back + kick in the air | ST | Kept as his one air special, so he has a pose exit in the air, as Ryu has. *(ours)* |
| **Level 1:** Shoryureppa | two quarter-circles forward + punch | ST | One Super stock. May juggle. |
| **Level 3:** Shinryuken | two quarter-circles forward + kick | Alpha, SF3, SF4, SF5 | Two Super stocks. The rising pillar: an anti-air and a close trap ender. |
| **Level 3:** Guren Senpukyaku | two quarter-circles back + kick | SF4 (Guren Enjinkyaku in SF5) | Two Super stocks. Travels forward, so it punishes from fireball range. |

Both Level 3s do less damage than Ryu's Shin Shoryuken *(blueprint: "lower in damage")*. Starting value *(ours)*: three quarters of it. The maintainer chose the two, 2026-11-18.

**The fireball trap** *(blueprint; the mechanism is ours)*: the Hadoken's first frames, before the fireball leaves his hands, may be cancelled into either Level 3. The opponent who jumps on seeing the throw meets Shinryuken; the one who stays down and tries to walk in meets Guren Senpukyaku.

**Left out on purpose** *(ours, following the blueprint's "no filler")*: Jinrai Kick and Quick Dash from SF6, the plain Shippu Jinraikyaku Super (it exists here as the EX move), Shinryu Reppa and Dragonlash Flame.

## 6. EX move, one stock

Ken has one EX move, and it has four endings. *(blueprint)*

**EX Shippu Jinraikyaku** *(blueprint; the kicks from Alpha's and SF3's Shippu Jinraikyaku)*

- **Input:** two kicks during a command kick. It comes out on hit or on whiff, not on block. *(blueprint: "on hit or whiff")*
- **What cancels into it:** only the three command kicks. Not the kara-cancelled Dragonlash Kick.
- **Ender, chosen by the button held as the last kick lands:**

| Button | Ender | Source | Result |
|---|---|---|---|
| Light kick | Low | ST crouching medium kick | Hits low. Leaves him close and even. |
| Medium kick | Overhead | Alpha's Inazuma Kakato Wari | Hits high. Knocks down. |
| Heavy kick | Hurricane Kick | ST | Carries the victim toward the corner. Knocks down. |
| No button | Launch | The Super's last rising kick | Launches for one juggle. |

Choosing an ender by button already has a precedent in the engine (Sagat's what-if Tiger Nexus).

## 7. Guard-cancel counter

**One stock.** Back, down-back, down + kick while blocking: an invincible rising kick that knocks down for little damage. *(SFZ's Alpha Counter; a kick so it differs from Ryu's punch, ours)*

## 8. Wakeup attack

**One stock.** A rising Kama Barai Geri that goes straight into the EX Shippu Jinraikyaku with its launch ender, the EX at no further cost. *(ours, built the way Ryu's is; the blueprint names none for Ken)*

## 9. The Special Pose

- **Stance shown:** his Heat Rush activation pose, the fireball gather held still. *(SF5; existing frames; the maintainer chose it, 2026-11-18)*
- **Enter from:** Hadoken, Shoryuken (any strength), Hurricane Kick, air Hurricane Kick, the three command kicks, the Dragonlash Kick, the target combo, any EX Shippu Jinraikyaku ender.
- **Leave into, on the ground:** Hadoken, Shoryuken, Hurricane Kick, the three command kicks, the target combo.
- **Leave into, in the air:** air Hurricane Kick.

Routes the blueprint names, which the tests should prove:

- Heavy flaming Shoryuken on a standing victim, who is forced to crouch; pose; heavy Hurricane Kick over a medium or large body, hitting from behind; standing heavy punch.
- The same against a small body: the Hurricane Kick does not cross over, so the route ends.
- Command kick, crouching light kick, by tier as in section 3.
- Command kick, kara cancel into Dragonlash Kick, once; a second kara cancel in the same combo does not come out.
- Command kick, EX Shippu Jinraikyaku, each of the four enders.

## 10. Posture overrides

- **Heavy flaming Shoryuken:** a flame special would keep the victim's posture. This one forces a crouch on its first hit instead. *(blueprint)*
- **Heavy Hurricane Kick:** against a crouching medium or large body it passes over, hits from behind and stands the victim up, leaving time to link standing heavy punch. Against a crouching small body, and against any standing body, it hits from the front as usual. *(blueprint; the small-body case is ours)*
- **Dragonlash Kick:** keeps posture, as any flame special.
- Light and medium Shoryuken, Hadoken and the lighter Hurricane Kicks follow the general rule and stand a crouching victim up.

## 11. What is new work

Everything above is built from moves and frames Ken already has. New behaviour, not new drawings:

- the kara cancel, which no fighter in the engine has, and its once-per-combo count;
- the Hadoken's kara cancel into a Level 3;
- command-kick links that depend on the victim's hurtbox tier;
- the EX Shippu Jinraikyaku as a sequence with four enders;
- the heavy Shoryuken's forced crouch and its last-hit-only juggle;
- the heavy Hurricane Kick's cross-over by tier and posture;
- two Level 3 Supers on one fighter;
- the Dragonlash Kick and Guren Senpukyaku outside the styles they come from;
- the target combo link.

## 12. Art that does not exist

- **Flames on the ST heavy Shoryuken.** ST's has none. The flame cue Alpha's heavy Shoryuken uses is drawn over it; no new frames.
- **A recovery roll** and **a dizzy animation,** as on Ryu's sheet: none exists for any fighter.
- Nothing else specific to Ken.
