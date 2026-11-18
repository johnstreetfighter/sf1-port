# SF KESSEN: an original era style

Status: approved by the maintainer on 2026-11-18, then sharpened in a question round the same day (see "Settled in the question round"). Decision recorded in [ADR-0027](../adr/0027-sf-kessen-is-an-original-style-with-round-rationed-stocks.md). Nothing here is built.

Words: era style, era gauge, style card, revision delta, what-if style, loadout and sub-HP are used as `CONTEXT.md` defines them. New words this document introduces are listed under "Words to add".

## Intent

The maintainer wants a new era style for the twelve tournament fighters that does not come from any Street Fighter entry. It follows a blueprint for an original Street Fighter that Clayton Chapman (@SaishuuKessen) set out across his "Game Theory" video series, as synthesized in the maintainer's own design document. The maintainer's scope, in his words: "The original sf1 12 and as much of the document that we can implement, ideally all of it where it makes sense."

The idea behind the blueprint is creativity through restriction. The ground rules are strict, close to Super Street Fighter II Turbo: juggles are scarce, most specials end a sequence, standing high attacks miss a crouching opponent. A scarce, round-rationed resource lets a player bend those rules for a moment, and only through moves that belong to his fighter alone. Nothing a fighter does with the system is generic.

The style is named **`SF KESSEN`** on the style card (the maintainer's choice; "kessen" is "decisive battle").

## What exists today

- Every era style runs on the translated 1987 player engine (ADR-0015). A family (`EraFamily` in `crates/sf-engine/src/era.rs`) owns an era gauge and system rules; each fighter has a move table per family in the extended data bank; a revision delta adjusts numbers; a loadout picks a variant. `StylePick` carries family, revision and loadout from the style card.
- Per-fighter fight state for era styles lives in `EraFighterState` (`crates/sf-engine/src/era_fight.rs`), with one module per family under `era_fight/`.
- Ryu, Ken and Sagat have Super Turbo move tables matched against the source game, with close and far normals (`data_bank/st_ryu_*.rs`, `st_ken_*.rs`, `st_sagat_*.rs`; spec #138). The other nine fighters' `SF2 ST` styles are what-if styles and less finished.
- A first search of the engine by name (not a full audit) found:
  - **Present and used by some era style:** a guard gauge with guard break (`GuardPower`), dizzy, juggle limits, EX moves, guard-cancel counters (Alpha Counter), recoverable life, super freeze, backdashes, wakeup options.
  - **Present only in patches:** close and far normals and command normals (Ryu's ST normals).
  - **Not found:** wall jumps, hurtbox size tiers, a counter-hit state, target combos, kara cancels, forced stand or crouch on hit, proximity guard lag, crouch-to-stand lag, two separate meters with round rationing.
- In a mixed-style match each fighter plays by his own style's rules (`np5_sessions.rs`, "mixed era locks apply native rules").

## Goals

1. **A new era family, `SF KESSEN`,** selectable on the style card for all twelve tournament fighters, with three loadouts.
2. **The blueprint's rules in play,** as listed under "The rules", for every rule that makes sense in a one-on-one game on this engine.
3. **Twelve kits with no generic parts.** Each fighter has his own command normals, target combo, EX moves, guard-cancel counter, wakeup attack and Special Pose exits, on a one-page kit sheet the maintainer approves before it is built.
4. **A first playable slice of Ryu, Ken and Sagat** that proves the rules, before the other nine kits are built.
5. **A full citizen among era styles:** mixed-style matches, the style card, the move list, Training, replays and netplay work for it as they do for the others.
6. **A Fair CPU that knows the style** (added by the maintainer on 2026-11-18). For each built kit it has proven routes in the route book and plays the style's own economy: it uses the Special Pose and EX moves in its combos, weighs holding three Versatile stocks against spending them, pays for wakeup options and the guard-cancel counter when they are worth it, and presses an opponent who has dropped below three.

## Non-goals

- The blueprint's tag-team section (hitstun decay ceilings, tag refunds). Nothing in this game is a team fighter in that sense.
- Kits for fighters outside the twelve, apart from Evil Ryu (the blueprint's Guile, Zangief, Chun-Li, Guy, Dhalsim and Fei Long, and the bonus roster). Their case studies are reference for tone only.
- A match-wide Kessen ruleset. It is an era style, chosen per fighter.
- World Tour content for the style.

## Anti-goals

- **Homogenized kits.** A kit that gives a fighter a generic "faster two-hit EX" of each special, or a pose he can enter from any normal, fails the blueprint even if every rule is implemented.
- **The document over the feel.** The maintainer decided on 2026-11-18 that when a rule from the blueprint feels wrong in his hands, feel wins. The blueprint sets starting values; playtesting may overrule any of them.
- **Silent departures.** Every place the built style leaves the blueprint is written down (see "Departures log"), so it stays clear which parts are Chapman's and which are ours.
- **A real person's name in the game.** The style card says `SF KESSEN`. Chapman's name and handle do not appear in the game itself. He is credited in the README and in this document.

## Constraints

- It runs on the 1987 player engine with era-style tables, as ADR-0015 requires. No second fight engine.
- Fights stay deterministic at 60 Hz for rollback netplay and replays (ADR-0018). All new state is part of the canonical fight state.
- Art comes from the ROM tiles and approved `sf1-port-art` packs (ADR-0006). A move with no suitable sprites is listed as art to draw on the fighter's kit sheet and recorded in `docs/planning/art-upgrades.md` if a stand-in is used.
- The style card's rows are short and drawn in capitals.

## Approaches

Three ways to build it were considered.

1. **A new family of its own, built on each fighter's Super Turbo moves (chosen).** `SF KESSEN` becomes a family beside `SF2` and `SF6`, with its own era gauge and rules. Each kit starts from the fighter's ST move table and adds his command normals, target combo, EX moves and pose exits. It keeps mixed-style matches, the style card, the move list and the CPU working as for other styles. It is the most work, and the nine fighters whose ST tables are less finished will need those brought up.
2. **A revision inside the `SF2` family.** Cheapest, reusing SF2's plumbing. A revision delta only adjusts numbers and switches; this style replaces the meter and adds rules SF2 has no room for.
3. **A match-wide ruleset like Definitive Overhaul.** Closest to the blueprint's intent and free of mixed-style questions, but no longer an era style: no Kessen Ryu against SF6 Ken, and outside the style card.

The maintainer chose the first on 2026-11-18, on the recommendation that it is the only one where the style is a full citizen and the rules have room to be what the blueprint describes.

## Decisions made with the maintainer

- **Roster:** the twelve tournament fighters.
- **Scope:** as much of the blueprint as makes sense, ideally all of it.
- **Kits:** drafted by Claude from moves each fighter already has across his era styles, one page per fighter, approved by the maintainer before building. Each sheet marks what comes from the blueprint and what is ours.
- **Order:** Ryu, Ken and Sagat first, then the other nine.
- **Feel wins** over the blueprint.
- **Name:** `SF KESSEN`.
- **Approach:** a new family built on Super Turbo moves.
- **Who sees it:** `SF KESSEN` appears on the style card for a fighter once his kit is built, whether or not what-if styles are enabled. CPU opponents in Arcade are not handed it until every kit exists.
- **Evil Ryu** gets a thirteenth kit from the blueprint's case study, built after Ryu, Ken and Sagat.

## Settled in the question round

The maintainer's answers on 2026-11-18.

1. **The kind of style** is an **original style**: an era style that comes from no Street Fighter entry. `CONTEXT.md` defines it.
2. **An opponent with no Versatile stocks** (any other style) counts as holding three or more wherever a Kessen move checks them. Evil Ryu's heavy Denjin Hadoken damages such an opponent's guard but does not break it at once.
3. **Chip KO.** While a Kessen fighter holds three or more stocks, only the attacker's style's super moves (Super Arts, Ultras, Critical Arts, Level 3s) can KO him through a block. SF6 Burnout chip and ordinary or EX specials cannot.
4. **Stocks come only from the round grants.** Nothing during a round adds one.
5. **Other match lengths.** A one-round match gets only the round 1 allotment. Any round after the third repeats round 3's gain.
6. **Evil Ryu's `SF KESSEN` style** appears on his style card whenever bonus roster options make him selectable.
7. **The pose's picture** is an existing frame of the fighter (a taunt, a win pose or a charge stance), chosen on his kit sheet and tinted with the afterimage trail. New art is drawn only where nothing existing reads as a deliberate pose.
8. **ADR-0027** records the family, the round-rationed stocks and the mixed-style rule.
9. **Clayton Chapman is told before it ships.** Building starts now; the maintainer messages him before the first release that includes `SF KESSEN`. This is a release gate.

## Claude's calls, accepted as starting points on 2026-11-18

- **Mixed-style matches:** each rule follows the fighter it constrains. A Kessen fighter suffers proximity guard lag and pays for his wakeup options whoever he faces; an opponent in another style keeps that style's rules. Attack properties (forced posture, juggle points) belong to the attacker's style. This matches how mixed matches work today.
- **The neutral rules are in the first slice,** because the blueprint treats them as the floor the rest stands on. Two are held back until a fighter needs them: wall jumps wait for the first agile fighter among the nine, and charge-free input in the pose waits for the first charge fighter. Ryu, Ken and Sagat use neither.
- **Loadout names on the card:** `BALANCED`, `VERSATILE`, `SUPER`.
- **One revision** for the family to start with, named `KESSEN`.
- **The move list, Training, replays and netplay** are in scope from the first slice. The first two read the same move tables as every other style; the last two follow from determinism.
- **Chapman's own videos are checked** for Adon, Gen, Birdie, Eagle and any other of the twelve before their sheets are drafted. Where he has designed a fighter, his version leads.

## Claude's calls after the builder's read, accepted as starting points on 2026-11-18

A reviewer read this document as a builder and asked what it left open. These are the answers Claude chose; each is a starting point that playtesting may change.

### Where the kits and numbers come from

- The blueprint's case studies for Ryu, Ken and Sagat are restated in "Case studies" at the end of this document. Those are the source for the first three sheets.
- A number the blueprint does not give starts from the fighter's Super Turbo value where ST has one (walk speeds, normal frame data, damage, dizzy thresholds, Super meter gain). Where ST has none (guard gauge size, proximity guard lag, crouch-to-stand lag), the builder proposes a value and the playtest in step 3 judges it.

### The Special Pose as a state

- It interrupts the move it is entered from on hit, on block or on whiff. The blueprint's whiff bypass needs the last.
- The two stocks are spent on entry, whatever follows.
- The pose is a short stance of its own (starting value ~20 frames). The fighter can be hit throughout it. The freeze cancel adds its screen freeze before the stance; the quick cancel does not.
- If no exit is pressed the stance ends in a short recovery (starting value ~10 frames) and nothing else happens.
- In the air the fighter keeps the path he was on.
- A fighter may enter the pose once per combo.

### The floor the bonuses sit on

- **Juggles.** The floor is Super Turbo: a move cannot hit an airborne victim unless its entry in the kit says it can, and says how many times. A bonus juggle point is permission for one extra hit of that kind. It is a flag on moves and a short per-combo record of which moves have used their bonus, not a points budget every move must fill in.
- **Counter hits.** A fighter hit during the startup or active frames of his own attack takes a counter hit: starting values +4 frames of hitstun and a quarter more dizzy. The pose-exit attack's grounded stun is +8 frames. A kit may mark a move as leaving its user in a counter-hit state for longer (the blueprint does this for wall jumps and for Sagat's raw heavy Tiger Uppercut).

### Mixed-style matches, in detail

- **Hurtbox tier** is a tag on each of the twelve fighters, whatever style he is playing, so a Kessen attacker's moves can read it on any victim. It changes nothing about a fighter's real boxes in his other styles. A Kessen kit's own hurtboxes are that kit's data.
- **Forced posture and juggle permission** are properties of the Kessen attacker's move and apply to any victim. The victim's own style still governs the victim's attacks.
- **A counter-hit state** on a Kessen fighter applies whoever hits him, since it is his constraint.

### The meters under match options

- **No Gauges** (and Pure Footsies) removes both meters. A Kessen fighter then plays as if he held fewer than three stocks and could spend none: chip can KO, the guard gauge can break, the backdash has no invincibility, and there are no paid wakeup options, EX moves or pose.
- **Unified Gauge** and Dramatic Battle's **Team Era Gauge** replace the Super meter only. Versatile stocks are rationed by round, not built, so each Kessen fighter keeps his own.
- **Training's fill-meters** refills both. Because that turns the three-stock privileges on for good, Training gets a setting for how many Versatile stocks a Kessen fighter holds.
- **Between rounds** unspent Versatile stocks carry over up to the cap, and so does Super meter.
- **A deciding round** after round 3 grants what round 3 grants.

## The rules

These are the blueprint's rules, restated as starting values. Numbers marked ~ are loose in the source and are tuning starting points.

### Controls and neutral

- **Motion and charge inputs only.** No one-button specials. Double-tap commands and multi-hit cancel rules are strict, without lenient shortcuts.
- **Proximity guard lag.** Holding back against a poke that is out of range puts the defender in guard recovery, so the attacker can walk in. Standing still, or checking with a crouching light kick, avoids it.
- **Crouch-to-stand lag.** Rising from a crouch costs a few frames before walking back.
- **Close and far standing normals** are distinct moves.
- **Hurtbox size tiers:** small, medium, tall and wide. A fighter's tier changes which combo routes work on him.
- **Standing high attacks miss a crouching opponent** unless the kit says otherwise; a fighter whose standing pokes hit crouchers pays for it in the priority of his light checks.
- **Forced posture on hit.** A standard special forces a crouching victim to stand. A flame special keeps the victim's posture. Individual moves may override this (Ken's heavy flaming Shoryuken forces a crouch on its first hit).
- **Wall jumps** for agile fighters only: an air normal, an air throw, a corner backflip or a chosen special can cancel into a wall jump, or into the fighter's downward command normal on contact. A wall-jumping fighter is in a counter-hit state until he lands, gets +1 juggle point on his next air hit, and may cancel an air normal into an air command normal.

### Two meters

Super meter and Versatile stocks are separate. Versatile stocks are shown as orbs and are granted by round, not built by attacking.

| Loadout | Max Super stocks | Versatile in round 1 | Versatile cap | Gain in round 2 / round 3 |
|---|---|---|---|---|
| Balanced | 2 | 3 | 5 | +2 / +1 |
| Versatile-Heavy | 1 (builds slowly) | 4 | 6 | +2 / +2 |
| Super-Heavy | 3 | 3 | 3 | +1 / +1 |

A Level 1 Super costs 1 Super stock and a Level 3 costs 2. Versatile-Heavy therefore has no Level 3. Super-Heavy can follow a Level 3 with a Level 1.

### Holding three or more Versatile stocks

Four things are true while a fighter holds 3 or more:

- **No chip KO.** Block damage is dealt, but only a Super can KO through a block.
- **No guard break.** The guard gauge empties and flashes but does not shatter until he drops below 3.
- **Invincible backdash:** ~4–5 frames of startup invincibility. Below 3 stocks a backdash has none.
- **Free kara cancel into a Level 3 Super** from the startup, whiff or projectile-clash frames of a special.

### Spending one Versatile stock

- **An EX move.** Each fighter has only 1–3, each unique to him. Some normals or target combos cancel only into an EX move.
- **Special into Level 1 Super.** (Special into Level 3 Super costs no Versatile stock.)
- **Level 1 Super into a special,** a command normal, or an EX move that comes free with the cancel, through a brief cancel animation.
- **Wakeup options.** There is no free recovery roll or delayed wakeup. A knocked-down fighter rises normally, or pays 1 stock for a roll, a delayed wakeup, or his unique wakeup attack, each on its own button pair.
- **A guard-cancel counter** fitted to the fighter, for 1 or 2 stocks as his sheet says.

### The Special Pose: two Versatile stocks

- **Entering:** only from a special move, a command normal, or a target combo. Never from a standard normal or from movement.
- **Leaving:** only into a special move, a command normal, a target combo, or an EX move (free of its usual stock when it leaves the pose). Never into a dash, a jump or a standard normal.
- **In the air:** leaving is limited to the fighter's air specials and air command normals.
- **Charge fighters** need no charge time while in the pose.
- **Two inputs:** the freeze cancel (LP+LK) with a short screen freeze and a coloured afterimage trail, for confirming hits; and the quick cancel (MP+MK) with no freeze, for mix-ups.
- **The pose-exit attack** gains +1 juggle point against an airborne victim, or deals stun beyond counter-hit stun against a grounded one. Multi-hit specials have their own hit caps in this state.
- **The freebie state.** When the pose-exit attack hits an airborne victim, the next move of any kind connects once.
- **Once per move.** Each move can use a bonus juggle point only once in a combo. (Ryu: Shoryuken, pose, Shoryuken, Hurricane Kick works; three Shoryukens do not.)
- **A target combo is one move** for all juggle bookkeeping.
- **Scaling.** Landing the pose-exit attack halves damage and dizzy for what follows. Whiffing it on purpose while the victim is still in hitstun avoids the penalty.

### Earned volatility

Instant dizzies and touch-of-death combos are allowed when they cost heavy resources and specific spacing, and leave the attacker exposed afterwards (he has spent below 3 stocks).

## Kit sheets

One page per fighter, written before his kit is built:

1. What he is for and what he gives up for it, in one or two sentences.
2. Hurtbox tier, and whether he wall-jumps.
3. Close normals, far normals and command normals.
4. His target combo.
5. His specials and Supers (Level 1 and, where the loadout allows, Level 3).
6. His 1–3 EX moves and what cancels into them.
7. His guard-cancel counter and its cost.
8. His wakeup attack.
9. What he may cancel into the Special Pose and out of it.
10. Any forced-posture overrides.
11. Source of each line: blueprint, an existing era style of his (named), or new.
12. Art needed that does not exist.

Ryu's, Ken's, Sagat's and Evil Ryu's sheets start from the blueprint's own case studies.

## Order of work

1. **Rules slice, on Ryu.** The family, the style card entry and loadouts, the two meters and their HUD, the three-stock privileges, the neutral rules, the Special Pose and its juggle bookkeeping, with Ryu's sheet built.
2. **Ken and Sagat.** Their sheets, exercising forced-posture overrides, hurtbox tiers and a second and third take on EX moves.
3. **Maintainer playtest** of the three. Departures from the blueprint are decided here.
4. **The Fair CPU learns the three.** Their kits are settled by the playtest, so their routes can be searched and proven without being thrown away.
5. **Evil Ryu,** whose kit leans hardest on guard break and the three-stock rules.
6. **The other nine,** a sheet at a time, in an order the maintainer picks.

From step 5 on, a kit is done when the Fair CPU has been taught it, not when it is merely playable.

## Teaching the Fair CPU

This follows the stronger Fair CPU design ([2026-11-18-stronger-fair-cpu-design.md](2026-11-18-stronger-fair-cpu-design.md), ADR-0026, spec #192), which is approved and ticketed but not built. The Kessen work depends on it: the route book, the route search and game plans must exist before step 4 can start. Until then the CPU plays a Kessen fighter with its general behaviour.

- **Routes.** Each kit gets a route book found by searching the real game and proven by tests, as for every other era style. The search must understand what is new here: the pose's entries and exits, the bonus juggle point and its once-per-move limit, the freebie state, the whiffed pose-exit that avoids scaling, and routes that depend on the victim's hurtbox tier and posture.
- **The stock economy.** A route's cost in Versatile stocks is part of the route. Choosing one weighs what it gains against losing the three-stock privileges: a CPU at exactly three stocks gives up chip-KO immunity, guard-break immunity and its invincible backdash by spending one.
- **Reading the opponent's stocks.** The game plan changes when a Kessen opponent drops below three: his guard can now break and chip can now finish him.
- **Paid defence.** Wakeup rolls, delayed wakeups, wakeup attacks and the guard-cancel counter are choices with a price, taken when the situation warrants it.
- **Fairness holds.** The Fair CPU's promise is unchanged: it sees what a player sees, through its reaction delay, and presses the pad.
- **Mixed-style matches.** A CPU in another style facing a Kessen fighter needs no Kessen routes of its own, but its game plan should use what it can see of his stocks.

## Departures log

A section of this document (or a file beside it, the builder's choice) lists each rule that was built differently from the blueprint or left out, with the reason and the date. It starts empty.

## Words in `CONTEXT.md`

Original style, SF KESSEN, Versatile stock, three-stock privileges, Special Pose, pose-exit attack, freebie state, hurtbox tier and kit sheet were added on 2026-11-18.

## Left to the builder

- How the new rules are represented in `EraFighterState` and the move tables, and how the 64-entry move-header table is laid out for a Kessen kit.
- The HUD's drawing of Super stocks and Versatile orbs, within the Era HUD's existing conventions (spec #216).
- The afterimage trail's colour and the freeze length for the freeze cancel.
- Test layout, following the existing era-fight tests at the `GameSession` and fight seams.
- Every number marked ~ above, as a starting value.

## Case studies

The blueprint's own kits for the three fighters of the first slice, restated. Each fighter's sheet starts here.

### Ryu: checking footsies and spacing conversions

- Keeps a crouching light kick with the best checking priority.
- His Hurricane Kick passes only slow projectiles of the Sonic Boom and Yoga Fire kind, and there is no meterless Hurricane Kick into Shoryuken juggle.
- Jumping medium punch has two stages: a hard knockdown raw, a pop-up launcher as the second hit of a combo.
- **Target combo:** far standing heavy punch into standing heavy kick. It can be cancelled on whiff in neutral and drifts with a direction (back pulls his hurtbox away from sweeps, forward steps in). Its late frames hit crouchers who are not in the small tier. It cancels into the EX Thrust Kick.
- **EX moves:** a Thrust Kick that wall-bounces (and wall-splats when it leaves the pose), and a multi-hit Hurricane Kick.
- **Wakeup attack:** a roundhouse into the EX Hurricane Kick.

### Ken: posture and hurtbox-tier routing

- The first hit of his heavy flaming Shoryuken forces a crouch, and only its last hit juggles. From there a pose into the roundhouse Hurricane Kick flies over medium and large crouching bodies, hits from behind to stand them, and links into standing heavy punch.
- His forward-drifting command kicks link into crouching light kick depending on the victim's tier. Once per combo, the end of one can be kara-cancelled into a flaming follow-up kick.
- **EX move:** any of those command kicks cancels, on hit or whiff, into an EX Shippu Jinraikyaku whose ender is chosen by button: light kick low, medium kick overhead, heavy kick Hurricane Kick, or no button for a launch.
- **Two Level 3 Supers,** lower in damage, built for kara-cancel fireball traps in neutral.

### Sagat: paid power

- **EX Scar Charge:** 1 stock when cancelled into, 2 in neutral. It unlocks two-punch armoured Tiger Uppercut links, two-kick armoured Tiger Knees that hit crouchers and break guard, and Tiger Genocide enders with two charges stocked.
- His raw heavy Tiger Uppercut keeps Super Turbo damage and leaves him in a counter-hit state for longer.

### Evil Ryu: guard break and system showcase

- Normals with lower priority than Ryu's; an Axe Kick on back and heavy kick; a medium punch into heavy punch auto-combo that causes a long rolling knockdown.
- A fully invincible heavy Shoryuken with high damage and dizzy that does not knock down, so a pose can restand the victim in the corner.
- **Denjin Hadoken** as a regular special (quarter-circle back and punch). It damages the guard gauge and builds dizzy even when blocked, and pushes him back a little each time so it cannot loop. Starting values: light ~30% guard and 15% dizzy; medium ~70% and 30%; heavy a full guard break and 40%. The heavy version breaks guard at once only against an opponent holding fewer than three Versatile stocks.
- **Teleport** that briefly freezes the opponent when the two sprites touch, with 1-stock EX branches partway through into a multi-hit Hurricane Kick or a fireball from where he started.
- **Guard-cancel counter:** a phase teleport.
- **Level 3 Super** (Messatsu Gou Shoryu) with three distance zones: 40% damage close, 50% at mid range, and 60% plus 20% dizzy on a tip sweet spot of 3 to 4 frames.

## Credit

The design this style follows is Clayton Chapman's (@SaishuuKessen), from his "Game Theory" video series. The kits for fighters he did not cover are this project's work in the spirit of his rules.
