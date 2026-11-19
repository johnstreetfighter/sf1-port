# SF KESSEN kit sheet: Evil Ryu

Status: approved by the maintainer on 2026-11-18, ahead of the playtest of Ryu, Ken and Sagat (KS16, #233). He has no ticket yet: the design builds him after that playtest, and it may change rules this sheet leans on (guard break, the three-stock privileges, dizzy). Not built.

Each line says where it comes from: **blueprint** (Clayton Chapman's design, as restated in the [design document](2026-11-18-sf-kessen-era-style-design.md)), an existing profile of Evil Ryu's or style of Ryu's that already has the move in the game, or **ours** (this project's choice where the blueprint is silent). Numbers are starting values; the playtest may change them.

## 1. What he is for

Evil Ryu wins by breaking the guard. His Denjin Hadoken wears down the guard gauge and builds dizzy even when blocked, his teleport makes blocking it a guess, and his heavy Shoryuken and Level 3 Super turn one opening into a dizzy. He is the kit that shows what the three-stock rules are for: nearly everything he threatens is weaker against an opponent who has kept three stocks. *(blueprint: "guard break and system showcase")*

What he gives up: less health than Ryu, normals that lose to Ryu's, and a heavy Shoryuken that does not knock down, so a blocked or whiffed one leaves the opponent standing beside him. *(blueprint for the normals and the Shoryuken; the health is his existing value)*

## 2. Body

- **Hurtbox tier:** medium. *(ours; Ryu's body)*
- **Wall jump:** no. *(blueprint: agile fighters only)*
- **Health:** 864, against the standard 960. *(his existing profiles; kept, ours)*

## 3. Normals

Ryu's **ST** close and far normals, with lower priority. *(blueprint: "normals with lower priority than Ryu's")*

| Group | Moves | Notes |
|---|---|---|
| Standing, close and far | Ryu's six buttons, close and far | In a trade with an equal normal, his loses. *(blueprint; how it is done is ours: his attack boxes are active one frame later than Ryu's)* |
| Crouching | Ryu's six buttons | His crouching light kick does not have the checking priority Ryu's has. |
| Jumping | Ryu's ST jump attacks | Jumping medium punch is the plain two-hit ST version, not Ryu's two-stage move. *(ours)* |

**Command normal:**

| Move | Input | Source | Notes |
|---|---|---|---|
| Axe Kick | back + heavy kick | blueprint; the frames of his SF4 and SF6 Ryusokyaku | Overhead. He steps into it. The Ryusokyaku special itself is not in the kit. *(the maintainer chose these frames, 2026-11-18)* |

Ryu's Collarbone Breaker and Solar Plexus Strike are not in this kit, and neither is Alpha's Senpukyaku hop kick. *(ours)*

## 4. Target combo

**Close standing medium punch into heavy punch,** an auto-combo. *(blueprint)*

- Press heavy punch during close medium punch. It comes out on hit or on block, not on whiff. *(ours)*
- On hit it causes a **long rolling knockdown**: the victim rolls most of the way across the screen and cannot use a paid wakeup roll out of it. *(blueprint for the knockdown; the wakeup limit is ours)*
- It cancels into the teleport only. *(ours)*
- It counts as one move for juggle rules.

## 5. Specials and Supers

| Move | Input | Source | Notes |
|---|---|---|---|
| Shakunetsu Hadoken | quarter-circle forward + punch | his ST, Alpha 3, SF4 and SF6 profiles | His only plain fireball is the red one. A flame special: it keeps the victim's posture. *(ours; Ryu's sheet left the red Hadoken for him)* |
| Shoryuken, light and medium | forward, down, down-forward + punch | every profile | As Ryu's. |
| Shoryuken, heavy | forward, down, down-forward + heavy punch | blueprint | Fully invincible until it stops rising. High damage and dizzy. **Does not knock down:** the victim stays standing in hitstun. |
| Hurricane Kick | quarter-circle back + kick | every profile | Ryu's rules: passes over slow projectiles only, never juggles into Shoryuken without a pose. |
| Air Hurricane Kick | quarter-circle back + kick in the air | his Alpha and SF4 profiles | His one air special, so he has a pose exit in the air. |
| Denjin Hadoken | quarter-circle back + punch | blueprint; the frames of Ryu's SF3 Denjin Hadoken, in his red fireball pens | See below. |
| Ashura Senku (teleport) | three punches to glide forward, three kicks to glide back | his Alpha and SF4 profiles | See below. |
| **Level 1:** Metsu Hadoken | two quarter-circles forward + punch | his SF4 Ultra I | One Super stock. Not chargeable here. *(ours; the maintainer chose it, 2026-11-18)* |
| **Level 3:** Messatsu Gou Shoryu | two quarter-circles forward + kick | blueprint; his Alpha Super and SF4 Ultra II | Two Super stocks. Three distance zones, below. |

**Denjin Hadoken** *(blueprint, with its starting values)*:

| Strength | Guard gauge damage | Dizzy |
|---|---|---|
| Light | about 30% | 15% |
| Medium | about 70% | 30% |
| Heavy | all of it | 40% |

- It does this on block as well as on hit.
- Each one pushes him back a little, so it cannot be looped.
- The heavy version breaks guard at once only against an opponent holding fewer than three Versatile stocks. Against three or more, and against any fighter in another style, the gauge empties and does not break. *(design document, three-stock privileges and mixed styles)*

**Ashura Senku** *(blueprint)*:

- If his sprite touches the opponent's during the glide, the opponent is frozen briefly. Starting value: 8 frames. *(the number is ours)*
- Partway through the glide he may spend one stock on a branch (section 6).
- He can be thrown at the end of it, as in his other profiles.

**Messatsu Gou Shoryu's zones** *(blueprint)*:

| Where it connects | Damage | Dizzy |
|---|---|---|
| Close | 40% | none added |
| Mid range | 50% | none added |
| Tip, a sweet spot 3 to 4 frames long | 60% | 20% |

**Left out on purpose** *(following the blueprint's "no filler")*: the Shun Goku Satsu, which the blueprint's kit does not mention *(the maintainer agreed, 2026-11-18; the playtest may add it)*; the plain Hadoken; the Ryusokyaku special; Shinku Hadoken and Shinku Tatsumaki; Alpha 2's fake Hadoken.

## 6. EX moves, one stock each

Both are branches of the teleport, taken partway through the glide. *(blueprint)*

| Move | Input during the glide | Source | Notes |
|---|---|---|---|
| EX Hurricane Kick | two kicks | blueprint; his SF4 EX Tatsumaki | Multi-hit, from where the glide has carried him. |
| EX Shakunetsu Hadoken | two punches | blueprint; his SF6 two-hit OD Shakunetsu | He reappears where the glide began and throws it from there. *(the blueprint says "a fireball from where he started"; reading it as a return to the start is ours)* |

What cancels into them: only the teleport. The teleport itself is a special, so any normal that cancels into a special reaches them through it, and so does the target combo.

## 7. Guard-cancel counter

**One stock.** Back, down-back, down + kick while blocking: a **phase teleport** that passes through the attacker and leaves him behind them. It does no damage. *(blueprint; the input and the no-damage rule are ours)*

## 8. Wakeup attack

**One stock.** A rising teleport that goes straight into the EX Hurricane Kick branch, the EX at no further cost. *(ours, built the way Ryu's is; the blueprint names none for him)*

## 9. The Special Pose

- **Stance shown:** the raised-knee pose of his Ashura glide, held still, with the existing Satsui accent drawn on it. *(his teleport's frames; the maintainer chose it, 2026-11-18)*
- **Enter from:** Shakunetsu Hadoken, Shoryuken (any strength), Hurricane Kick, air Hurricane Kick, Denjin Hadoken, the Axe Kick, the target combo.
- **Leave into, on the ground:** Shakunetsu Hadoken, Shoryuken, Hurricane Kick, Denjin Hadoken, the Axe Kick, the target combo, the teleport and through it either EX branch.
- **Leave into, in the air:** air Hurricane Kick.

Routes the tests should prove:

- Heavy Shoryuken in the corner, which leaves the victim standing; pose; a grounded follow-up that would not connect after a knockdown. *(blueprint)*
- Heavy Denjin Hadoken, blocked by an opponent with two stocks: guard breaks. Blocked at three stocks, or by a fighter in another style: the gauge empties and holds.
- Three light Denjin Hadokens in a row cannot keep the opponent in blockstun, because of the pushback.
- The teleport through the opponent freezes them; each EX branch costs one stock and comes out from the right place.
- Messatsu Gou Shoryu at each of its three distances, with dizzy only on the tip.
- The target combo's rolling knockdown, and no paid roll out of it.

## 10. Posture overrides

- **Shakunetsu Hadoken** and its EX version keep the victim's posture, as flame specials.
- **Heavy Shoryuken** leaves a standing victim standing and, against a crouching one, stands them up; it never knocks down.
- Everything else follows the general rule and stands a crouching victim up.

## 11. What is new work

- **He is not a fighter of his own.** Today Evil Ryu is Ryu playing one of eleven profiles, and the code says he has no SF KESSEN kit. The style has to be added for his profile and appear on his style card whenever bonus roster options make him selectable. *(design document)*
- Ryu's ST close and far normals, which are switched off for every Evil Ryu profile today, switched on for this one with their lower priority.
- The Denjin Hadoken as a guard-gauge and dizzy tool. It needs the guard gauge outside Alpha 3 (KS3, #220) and stun build-up (KS17, #234).
- A hit that does not knock down where the move normally would.
- A rolling knockdown. Nothing in the engine has one.
- The teleport's freeze on contact and its two branches.
- A Super with three distance zones.
- The phase teleport as a guard-cancel counter.
- The Axe Kick as a command normal, and the target combo link.

## 12. Art that does not exist

- **A rolling knockdown.** No fighter has frames for one. A stand-in from the existing knockdown and slide frames would go on the art upgrades list.
- **His own Denjin Hadoken.** Ryu's SF3 Denjin frames in Evil Ryu's red pens stand in; nothing new is drawn.
- **A dizzy animation** and **a recovery roll,** as on Ryu's sheet: none exists for any fighter.
