# SF KESSEN kit sheet: Ryu

Status: approved by the maintainer on 2026-11-18 (KS1, #218). Not built.

Each line says where it comes from: **blueprint** (Clayton Chapman's design, as restated in the [design document](2026-11-18-sf-kessen-era-style-design.md)), **ST** or another era style of Ryu's that already has the move in the game, or **ours** (this project's choice where the blueprint is silent). Numbers are starting values; the playtest may change them.

## 1. What he is for

Ryu wins by checking and spacing. His crouching light kick is the best fast check in the game, his fireball and uppercut are the plain honest versions, and his damage comes from converting a well-spaced hit, not from pressure. *(blueprint)*

What he gives up: his standing pokes go over a crouching opponent, he has no juggle without paying for one, and nothing in his kit moves him forward for free. *(blueprint, stated generally; applied to Ryu by us)*

## 2. Body

- **Hurtbox tier:** medium. *(ours; he is the reference body)*
- **Wall jump:** no. *(blueprint: agile fighters only)*

## 3. Normals

All from **ST** (New Ryu), which already has them with measured boxes.

| Group | Moves | Notes |
|---|---|---|
| Standing, close and far | Six buttons, each with a close and a far version chosen by distance | Close light kick hits low. Close heavy kick hits twice. |
| Crouching | Six buttons | Crouching light kick keeps top checking priority *(blueprint)*. Heavy kick sweeps. |
| Jumping | The ST jump attacks | Forward or back jumping medium punch is the two-stage move below. |

**Command normals** *(ST)*:

| Move | Input | Notes |
|---|---|---|
| Collarbone Breaker | forward + medium punch | Two hits, overhead. |
| Solar Plexus Strike | forward + heavy punch | Two hits. Cancels only into an EX move *(ours)*. |

**Two-stage jumping medium punch** *(blueprint; adapted from ST's two-hit version)*: raw, it is one hit and a hard knockdown. As the second hit of a combo it pops the victim up instead, and one follow-up may juggle.

## 4. Target combo

**Far standing heavy punch into standing heavy kick.** *(blueprint; both halves exist in ST, the link is new)*

- Press heavy kick during far heavy punch. It comes out on hit, on block or on whiff.
- Holding back during the kick pulls his body back out of sweep range; holding forward steps him in.
- The kick's late frames hit a crouching opponent who is not in the small tier.
- It cancels only into EX Thrust Kick.
- It counts as one move for juggle rules.

## 5. Specials and Supers

| Move | Input | Source | Notes |
|---|---|---|---|
| Hadoken | quarter-circle forward + punch | ST | Stands a crouching victim up. |
| Shoryuken | forward, down, down-forward + punch | ST | Invincible on the way up. One hit against an airborne victim. |
| Hurricane Kick | quarter-circle back + kick | ST, blueprint rules | Passes over slow projectiles only. Never juggles into Shoryuken without a pose. Knocks down. |
| Air Hurricane Kick | quarter-circle back + kick in the air | ST | Kept as his one air special, so he has a pose exit in the air. *(ours; the maintainer agreed, 2026-11-18)* |
| **Level 1:** Shinku Hadoken | two quarter-circles forward + punch | ST | One Super stock. May juggle. |
| **Level 3:** Shin Shoryuken | two quarter-circles forward + kick | SF3 | Two Super stocks. Close range, his highest damage. The blueprint names no Level 3 for Ryu. *(ours; the maintainer chose it, 2026-11-18)* |

**Left out on purpose** *(ours, following the blueprint's "no filler"; the maintainer agreed, 2026-11-18)*: the Shakunetsu (red) Hadoken, which the blueprint gives to Evil Ryu's kit in spirit, and the plain Joudan Sokutogeri, which exists here only as an EX move.

## 6. EX moves, one stock each

| Move | Input | Source | Notes |
|---|---|---|---|
| EX Thrust Kick (Joudan Sokutogeri) | half-circle forward + two kicks | blueprint; SF3's EX Joudan | Wall-bounces. As a pose-exit attack it wall-splats instead. |
| EX Hurricane Kick | quarter-circle back + two kicks | blueprint; SF3's EX Tatsumaki | Three hits, the last launches. |

What cancels into them: the target combo and Solar Plexus Strike into EX Thrust Kick only; any normal that cancels into a special may cancel into either.

## 7. Guard-cancel counter

**One stock.** Back, down-back, down + punch while blocking: an invincible straight punch that knocks down for little damage. *(SFZ's Alpha Counter)*

## 8. Wakeup attack

**One stock.** A rising roundhouse that goes straight into the EX Hurricane Kick, the EX at no further cost. *(blueprint; built from close heavy kick and SF3's EX Tatsumaki)*

## 9. The Special Pose

- **Stance shown:** his Denjin Charge stance. *(SF6; existing frames; the maintainer chose it, 2026-11-18)*
- **Enter from:** Hadoken, Shoryuken, Hurricane Kick, air Hurricane Kick, Collarbone Breaker, Solar Plexus Strike, the target combo, EX Thrust Kick, EX Hurricane Kick.
- **Leave into, on the ground:** Hadoken, Shoryuken, Hurricane Kick, Collarbone Breaker, Solar Plexus Strike, the target combo, EX Thrust Kick, EX Hurricane Kick.
- **Leave into, in the air:** air Hurricane Kick.

Routes the blueprint names, which the tests should prove:

- Shoryuken, pose, Shoryuken, Hurricane Kick.
- Shoryuken, pose, Hurricane Kick, Shoryuken.
- Never three Shoryukens.
- Target combo, EX Thrust Kick, wall bounce, follow-up.
- Pose into EX Thrust Kick for a wall splat.

## 10. Posture overrides

None. Ryu follows the general rule: his specials stand a crouching victim up. He has no flame special in this kit.

## 11. What is new work

Everything above is built from moves and frames Ryu already has. New behaviour, not new drawings:

- the target combo link and its drift;
- the two-stage jumping medium punch (ST's always hits twice);
- the Hurricane Kick's projectile rule;
- the wakeup attack as a sequence;
- EX-only cancels.

## 12. Art that does not exist

- **A recovery roll.** No fighter has one. The universal paid roll (KS8) needs frames; a stand-in would be recorded in the art upgrades list.
- **A dizzy animation.** None exists for any fighter.
- Nothing specific to Ryu's own moves.
