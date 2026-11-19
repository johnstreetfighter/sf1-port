# SF KESSEN kit sheet: Sagat

Status: approved by the maintainer on 2026-11-18 (KS14, #231). Not built.

Each line says where it comes from: **blueprint** (Clayton Chapman's design, as restated in the [design document](2026-11-18-sf-kessen-era-style-design.md)), **ST** or another era style of Sagat's that already has the move in the game, or **ours** (this project's choice where the blueprint is silent). The blueprint says less about Sagat than about Ryu or Ken, so more of this sheet is ours. Numbers are starting values; the playtest may change them.

## 1. What he is for

Sagat wins by paid power. His high and low Tiger Shots hold the opponent at range, and his Versatile stocks buy Scar charges that turn his Tiger Uppercut and Tiger Knee into armoured moves and lengthen his Super. *(blueprint: "paid power")*

What he gives up: he is the tallest body of the first three, so more routes work on him. He has no command normal and no air special, so he has no air exit from the Special Pose. Without charges his Tiger Knee misses crouchers and his raw heavy Tiger Uppercut is a large risk. *(ours, drawn from the blueprint's notes and from what he has in the game)*

## 2. Body

- **Hurtbox tier:** tall. *(ours; no size tier exists in code yet, and the design documents call him the tallest of the cast)*
- **Wall jump:** no. *(blueprint: agile fighters only)*

## 3. Normals

All from **ST**, which already has them with measured boxes.

| Group | Moves | Notes |
|---|---|---|
| Standing, close and far | Six buttons, each with a close and a far version chosen by distance | His far kicks are the longest pokes of the first three. |
| Crouching | Six buttons | Heavy kick sweeps. |
| Jumping | The ST jump attacks | |

**Command normals:** none. He has none in any era style here, and the blueprint names none. *(ours)*

## 4. Target combo

**Close standing light kick into standing heavy kick.** *(ours; both halves exist in ST, the link is new. The blueprint names none for Sagat.)*

- Press heavy kick during close light kick. It comes out on hit or on block, not on whiff.
- It cancels into EX Scar Charge and into Tiger Knee, nothing else.
- It counts as one move for juggle rules.

## 5. Specials and Supers

| Move | Input | Source | Notes |
|---|---|---|---|
| Tiger Shot | quarter-circle forward + punch | ST | High. Passes over a crouching small or medium body. Stands a crouching tall or wide victim up. |
| Low Tiger Shot | quarter-circle forward + kick | ST | Low. Must be blocked or jumped. Stands a crouching victim up. |
| Tiger Knee | down, forward, up-forward + kick | ST | Two hits, moves him forward. Passes over a crouching small or medium body. *(the miss on crouchers is ours, read from the blueprint giving "hits crouchers" to the charged version)* |
| Tiger Uppercut | forward, down, down-forward + punch | ST | Invincible on the way up. The heavy version keeps its Super Turbo damage and leaves him in a counter-hit state for the whole of its recovery, not only its startup and active frames. *(blueprint)* |
| **Level 1:** Tiger Genocide | two quarter-circles forward + kick | ST | One Super stock. With two Scar charges stored it gains the ender in section 6. |
| **Level 3:** Tiger Cannon | two quarter-circles forward + punch | Alpha, SF4 | Two Super stocks. The large fireball: full-screen, his highest damage. *(ours; the maintainer chose it, 2026-11-18)* |

**Left out on purpose** *(ours, following the blueprint's "no filler")*: Tiger Nexus (a what-if SF6 move), Tiger Raid as a Super of its own (it is the Genocide ender), and the SF5 V-Trigger follow-ups.

## 6. EX move: Scar Charge, and what the charges buy

**EX Scar Charge** *(blueprint; the chamber-pose animation from Alpha's Angry Charge and SF4's Angry Scar)*

- **Input:** quarter-circle back + two punches. *(ours)*
- **Cost:** 1 Versatile stock when cancelled into, 2 from neutral. *(blueprint)*
- **What cancels into it:** any normal that cancels into a special, the target combo, and the Special Pose.
- **What it gives:** one stored **Scar charge**. He holds at most two. Charges stay until used or until the round ends; they do not carry into the next round. With two stored, Scar Charge does not come out. *(the maintainer chose stored charges, 2026-11-18)*

What a charge buys:

| Move | Input | Cost | What changes |
|---|---|---|---|
| Charged Tiger Uppercut | forward, down, down-forward + two punches | 1 charge | Armoured. On hit he recovers in time to link one follow-up. *(blueprint: "two-punch armoured Tiger Uppercut links"; the link reading is ours)* |
| Charged Tiger Knee | down, forward, up-forward + two kicks | 1 charge | Armoured. Hits a crouching body of any tier. Empties the guard gauge when blocked. *(blueprint)* |
| Tiger Genocide with the Raid ender | the Level 1 input, with two charges stored | 1 Super stock and both charges | Tiger Genocide ends in the Tiger Raid kicks for more damage and a corner carry. *(blueprint: "enders with two charges stocked"; the maintainer chose Tiger Raid, 2026-11-18)* |

With one charge stored, Tiger Genocide comes out plain and the charge is kept.

**Armour** *(the maintainer chose this, 2026-11-18)*: during the move's startup and active frames he absorbs one hit and takes its damage in full. He is not knocked out of the move. A throw, a Super, or a second hit beats the armour. If the absorbed hit would KO him, it does.

**Guard break:** the charged Tiger Knee empties the guard gauge. Whether the guard then breaks follows the general rule: it does not break against an opponent holding three or more Versatile stocks. *(design document, three-stock privileges)*

## 7. Guard-cancel counter

**One stock.** Back, down-back, down + kick while blocking: an invincible Tiger Knee that knocks down for little damage. *(SFZ's Alpha Counter)*

## 8. Wakeup attack

**One stock.** A rising light Tiger Uppercut that ends in the chamber pose and stores one Scar charge, at no further cost. If he already holds two charges it stores none. *(ours; the blueprint names none for Sagat. Ryu's and Ken's wakeup attacks lead into their EX move, and this does the same.)*

## 9. The Special Pose

- **Stance shown:** his Tiger Shot chamber pose, the same pose Scar Charge shows. The pose is brief and Scar Charge has its own flash, so the two read apart by context. *(Alpha's Angry Charge; existing frames; the maintainer chose it, 2026-11-18)*
- **Enter from:** Tiger Shot, Low Tiger Shot, Tiger Knee, Tiger Uppercut, the charged Uppercut and Knee, the target combo.
- **Leave into, on the ground:** Tiger Shot, Low Tiger Shot, Tiger Knee, Tiger Uppercut, the charged Uppercut and Knee, the target combo, and EX Scar Charge at its cancelled price of 1 stock.
- **Leave into, in the air:** nothing. He has no air special. *(ours; it is part of what he gives up)*

Routes the tests should prove *(ours, from the rules above)*:

- A cancellable normal into EX Scar Charge costs 1 stock and stores one charge; from neutral it costs 2.
- A third Scar Charge does not come out, and no stock is spent.
- Charged Tiger Uppercut through one hit: he takes the damage and the Uppercut lands. Through two hits, or a throw, he loses.
- Charged Tiger Uppercut on hit, then one linked follow-up.
- Tiger Knee passes over a crouching medium body; the charged Tiger Knee hits it.
- Charged Tiger Knee on block empties the guard gauge, and breaks guard only when the opponent holds fewer than three stocks.
- Tiger Genocide with two charges ends in Tiger Raid and leaves him with none; with one charge it is plain and the charge remains.
- Raw heavy Tiger Uppercut, blocked: a hit during its recovery is a counter hit.
- Charges are gone at the start of the next round.

## 10. Posture overrides

None of his own. His specials follow the general rule and stand a crouching victim up. What section 5 says about Tiger Shot and Tiger Knee passing over crouchers is reach, not posture.

## 11. What is new work

Everything above is built from moves and frames Sagat already has. New behaviour, not new drawings:

- stored Scar charges: a count of up to two, kept through the round;
- armour on a fighter's own special, which no Ken or Sagat move has today (the engine has armour for SF4 Focus and SF6 Drive Impact);
- the charged Tiger Uppercut's link on hit;
- the charged Tiger Knee's reach on crouchers and its guard damage;
- Tiger Genocide's ender, chosen by the charges held;
- the heavy Tiger Uppercut's longer counter-hit state;
- the tall hurtbox tier, with Tiger Shot and Tiger Knee reach by tier;
- the wakeup attack as a sequence;
- the target combo link.

## 12. Art that does not exist

- **A way to show stored Scar charges.** Neither the Era HUD nor his sprite has one. This sheet does not design it; a small mark beside his Versatile row is the likely answer, and it must also fit the compact gauge layout.
- **A dedicated Special Pose stance.** He shares the chamber pose with Scar Charge by the maintainer's choice; nothing is owed, but a distinct stance would be an upgrade.
- **A recovery roll** and **a dizzy animation,** as on Ryu's sheet: none exists for any fighter.
