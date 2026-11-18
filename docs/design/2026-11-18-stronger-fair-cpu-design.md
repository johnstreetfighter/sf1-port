# A stronger, more active Fair CPU

Status: approved by the maintainer on 2026-11-18, then sharpened in a question round the same day (see "Settled in the question round"). Decision recorded in [ADR-0026](../adr/0026-fair-cpu-plays-from-game-plans-and-a-route-book.md). Nothing here is built.

Words: situation, game plan, route, route book, pressure sequence, combo reset, focus and habit memory are defined in `CONTEXT.md`. Where this document says "plan", "string" or "reset" it means game plan, pressure sequence and combo reset.

## Intent

The maintainer's words: "right now I don't feel like its as active as a human at the highest level". The Fair CPU should press the fight the way a top player does: punish whiffs, take real combos off a hit, stay on the opponent after a hit, a blocked move or a knockdown, move with purpose, use each era style's tools, vary its routes, and learn the opponent's habits. It should do this for the whole cast, in every mode that uses the Fair CPU, while staying fair.

## What exists today

`crates/sf-engine/src/fair_cpu.rs` and `fair_cpu/era.rs`. The Fair CPU sees object RAM and health through a reaction delay, scores a list of single actions, and presses Player 2's pad.

- Its reflexes are already near a person's best. Very Difficult: 12-frame delay, 92% of threats answered, 90% best choice, 97% clean motions.
- It attacks only when the opponent is neither attacking nor in a hit reaction (`open` in `choose`), so it never follows up a hit. `Choice::Combo` taps light punch. `Choice::Cancel` is one normal into one special.
- Every action is chosen from scratch after the last one settles. It holds no plan across actions: no block strings, no pressure on a knocked-down opponent, no walk-in throw as a set-up.
- In range at Very Difficult, guarding (score 50) competes evenly with attacking (44 to 58); waiting and walking back are always on the list.
- Era mechanics are considered on about one neutral choice in five (`Tactics::neutral`, `quality / 4`). Supers come only after a seen hit or on wake-up.
- It learns its own normals' reach and its projectile's range. It learns nothing about the opponent.
- For the SF2 to SF6 era styles a second layer, the Utility AI (`utility/live.rs`, ADR-0019), adds short commands when the Fair CPU's pad is idle. Its delays are 12, 8, 4 and 1 frames by difficulty level, it uses the current distance, and it techs throws from the engine's own tech-window flag. At Very Difficult that answers an attack one frame after it shows, which breaks the Fair CPU's promise.
- Under an overhaul preset the 1987 era style still uses Capcom's CPU in CPU fights (`fair_cpu_press` drops the Fair CPU for `EraFamily::Sf1987` in game modes 2 and 4). Faithful 1987 always does.
- The combo search (`crates/sf-host/examples/combo_clips.rs`, index in `docs/media/combos/INDEX.md`) found a route of five or more hits for 207 of 301 fighter and style pairs. 12 pairs cannot combo by rule (1987). 82 hit search limits (Sagat 26, Eagle 12, Birdie 12, Ken 9, all twelve SF1+ pairs). No person has watched the 207, and some route names begin with a two-button press that may do nothing.

## Goals

1. **Real combos.** After a hit it sees land, the Fair CPU continues into a route that works for its fighter, era style, the rules that are on, the opponent's stance and its meter.
2. **Whiff punishes** that lead into those routes, not one heavy normal.
3. **Staying on the opponent.** After a hit, a blocked move or a knockdown it keeps the initiative: strings with gaps, throws, lows and overheads, and timed attacks on a rising opponent.
4. **Purposeful movement.** It walks and dashes in and out of its ranges to draw a whiff or take space, and stands still only for a reason.
5. **Era style tools as part of the plan.** Drive Rush, Drive Impact, parries, Focus and FADC, V-Skill and V-Trigger, EX moves, Alpha Counters, Custom Combo and supers are chosen where they fit, with meter spent on purpose.
6. **Creative play**, as the maintainer defined it:
   - several routes per pair, so the follow-up is not predictable;
   - the route picked for the moment: by health, meter, corner and round state;
   - resets and mix-ups: sometimes stopping a combo early to throw, go low or overhead, or bait a reversal;
   - adapting to the opponent's habits during the match.
7. **Every difficulty level changes.** Each gets the new play scaled down from the top. Existing levels will be harder than today; that is accepted.
8. **The top level plays as well as the fair limits allow.**
9. **One Fair CPU everywhere:** Arcade, Local against the CPU, Training's CPU, Dramatic Battle's CPUs, World Tour, and matchmaking's CPU entries.
10. **The 1987 era style under an overhaul preset moves to the Fair CPU** in CPU fights (the maintainer, 2026-11-18). Its rules allow no combos, so it plays with plans, pressure, punishes and habit memory only. Arcade and World Tour fights in that style will play differently from today.
11. **It reaches players all at once** (the maintainer, 2026-11-18). The work lands on main behind a switch that stays off, pair by pair, and is turned on for everyone when the whole cast is ready. One CPU version bump.

## The fair limits (all four hold at every level)

- **Never reads inputs.** It decides from what the screen shows.
- **Human reaction delay, with a focus.** At the top level it reacts in about 12 to 13 frames to the one thing it is watching for, and in 16 to 18 frames to anything else. What it watches for depends on the situation and on the opponent's habits.
  - Evidence: Woods et al. 2015 give 213 ms (12.8 frames) for one expected signal after removing equipment delay; Infil gives about 265 ms (16 frames) in a game and notes that good players still get hit by 20-frame moves when several threats are open (`docs/research/sf1plus-reference-numbers.md` section 4.1). A search on 2026-11-18 found no controlled study of top players that supports going below 12.
- **Plays through the pad.** Real motions and buttons, through the same recognizers as a player.
- **Still makes mistakes.** A small chance to drop a route or pick a worse option stays at the top level, larger below it and under pressure.

## Non-goals

- The Faithful 1987 preset keeps Capcom's CPU, unchanged.
- No machine-learned CPU.
- No looking ahead in the engine during a fight.
- No new difficulty level above Very Difficult.

## Anti-goals

- A CPU that feels like it cheats: answering on the first frame, never dropping anything, knowing a throw is coming before it shows.
- A CPU that runs the same loop every round.
- A CPU that is strong only by being fast to react while still passive.

## Approaches

1. **Grow the current scorer.** Add choices (a string, a wake-up attack, a dash, a long combo) to `choose` and retune. Cheapest. It still forgets its intent after each action, so pressure and resets would come out by luck of the scores, and one crowded function would have to be tuned for 301 pairs.
2. **Situations and plans, with a route book.** The CPU knows its situation and runs multi-step plans that branch on what it sees; combos come from a generated route book. Most work up front; the route book has gaps to fill. This is the structure Capcom described for SF6's CPU (research section 4.3).
3. **Look ahead in the engine during the fight.** Strongest on paper. Heavy every frame, worse under rollback and Dramatic Battle, hard to scale down or keep human-looking.

**The maintainer chose approach 2 on 2026-11-18.** Approach 3 survives only offline, as the search that fills the route book.

## Design

Parts the maintainer decided are stated plainly. Parts marked *(my call)* are the designer's proposals and open to change.

### Situations

The Fair CPU classifies each seen frame into one situation and chooses only among that situation's plans *(my call on the list)*:

| Situation | Seen as | What the plans are for |
| --- | --- | --- |
| Neutral, far | outside both fighters' longest reach | take space, projectiles, Drive Rush or dash in, bait a jump |
| Neutral, close | inside someone's reach, nobody committed | pokes, walk in and out to draw a whiff, throw, jump-in |
| It hit | opponent's health dropped or hit reaction seen | route from the route book, or a reset |
| It was blocked | its attack met a guard | string with a gap, throw, back off and punish the answer |
| Opponent knocked down | opponent in a knockdown or rising | timed meaty attack, throw, bait a reversal, hold position |
| Opponent whiffed | attack box gone, recovering in reach | punish route |
| Defending | opponent's attack, jump or projectile seen | guard, anti-air, parry, guard cancel, tech, escape |
| It is knocked down | its own knockdown or rise | guard, reversal, tech, back off |
| Corner | either fighter near a wall | modifies the scores of the above |

### Plans

A plan is a short script with branches, not one action. Each step says what to press, what to watch for, and where to go on each outcome (hit, block, whiff, opponent jumped, nothing). Example: walk to crouching medium kick range, press it; on hit go to "It hit"; on block continue to a string step or a throw step; on whiff retreat and guard.

- Plans are chosen by score within the situation, with the difficulty level's decision quality and a little noise, as today.
- A plan can be interrupted by a threat the CPU answers, as today.
- The confirm is seen through the reaction delay. A route's first cancel is therefore either committed before the confirm (a normal into a special that is safe enough on block) or taken from a hit with enough hitstun to confirm; the route book records which.

### Route book

A generated data file, one set of routes per tournament fighter and era style, checked in and loaded by the Fair CPU.

- **Each route records:** opener and how it is reached (ground hit, jump-in, punish, anti-air, Drive Rush), the pad script with its timing windows, the rules it needs (`true_combos`, `cancels`), hits, damage, meter spent and gained, whether it ends in a knockdown, whether it needs the corner, which stances it works on, and whether its first cancel can be confirmed.
- **Filled by search, not by hand.** The combo search grows from "one route of five or more hits" to "the useful routes": short ones (two to four hits) included, several per pair, with damage and ending measured. The 82 pairs at search limits and SF1+ need this most.
- **Every route in the book is proven by a test** that replays it in the engine and checks its hits, so a moveset change that breaks a route fails the suite instead of making the CPU drop combos silently.
- **Picking:** in "It hit" and "Opponent whiffed" the CPU filters routes by what is possible now, then scores by the moment: a route that wins the round scores highest; otherwise damage, a knockdown ending, corner carry and meter kept are weighed, with variety (a route used recently scores lower).
- **Resets:** a route can be cut at marked points into a throw, a low, an overhead or a pause that baits a reversal. How often is a difficulty-level value and shifts with the opponent's habits.
- **Keyed by the attacker's pair, checked against every opponent** *(my call)*. The book is not multiplied by the opponent. The search proves each route against one defender; a separate offline pass replays it against each opponent fighter and style, standing and crouching, and the route records the ones it fails on, so the CPU does not pick it there. The suite's route tests cover the reference defender only, to keep the suite's run time down; the full pass is a developer command.
- **Pairs with no route** (1987 rules, or none found yet) fall back to the single cancel and link choices the CPU has today.

### Focus and reaction

- The CPU holds one focus at a time: the event it expects most (the opponent's jump while it controls the ground with a projectile, a wake-up reversal while it pressures, a throw while it guards up close, a whiff while it hovers at the edge of range).
- Something matching the focus is answered at the level's focused delay; anything else at the slower one *(my call on the numbers)*:

| Level | Focused | Otherwise |
| --- | --- | --- |
| Easy | 27 | 32 |
| Normal | 19 | 24 |
| Difficult | 15 | 20 |
| Very Difficult | 12 | 17 |

- Motion specials given in answer keep their extra cost.
- The Utility AI's separate delays go. In CPU fights the Fair CPU owns every command, era mechanics included, under this one rule. Throw techs come from what is seen (the opponent walking into range, a grab starting), not from the engine's tech-window flag. Reading its own fighter's state (in blockstun, can cancel) stays: a player feels those.
- Replay takeover's opponent becomes the Fair CPU in this project (the maintainer). The Utility AI remains only for Training's scripted drills and combo trials.

### Habit memory

A small, per-match record of what the opponent did in recurring spots, held in the Fair CPU's state so snapshots and rollback carry it:

- after its projectile: jumped, guarded, answered with a projectile;
- rising from a knockdown: guarded, reversal, button, throw, back off;
- after guarding its string: pressed a button, jumped, threw, kept guarding;
- when grabbed: teched or not;
- jumping in: how often and from where.

Counts fade over time so a change of habit is noticed. They shift plan scores and the focus (an opponent who always jumps the projectile gets a CPU that throws one and watches for the jump). Lower difficulty levels use the memory less; Easy does not use it *(my call)*. The memory starts empty every match.

### Difficulty levels

Each level is a set of values over the same machinery *(my call on the shape; numbers come from tuning)*:

- which plans and how long a route it may take (Easy: short routes, little pressure; Very Difficult: everything);
- decision quality, execution and guard chances, as today;
- focused and unfocused delays;
- how much it uses habit memory, resets and meter;
- how long it stays on the opponent before backing off.

Easy must stay beatable by someone new to the game *(my proposal)*.

### Determinism and versions

- The same inputs give the same fight. All new state (situation, plan and step, focus, habit memory, recent routes) lives in `FairCpu`, is part of the snapshot, and is in the canonical hash.
- The route book's content is part of what both sides of a netplay session must agree on.
- While the switch is off the old scorer stays in the build and is what plays; the measuring run uses it as the baseline. When the switch is turned on for good the old scorer is deleted *(my call)*: matchmaking is not built, so no recordings made under an old CPU version exist that would need it.
- The CPU version number (ADR-0025) is bumped, so CPU entries start fresh ratings. The CPU ceiling at the top of Gold is looked at again once the new play is measured.

### Measuring it

*(my proposal)* Before changing behaviour, build a headless CPU-against-CPU run (an untracked `crates/sf-host/examples/cpu_match.rs` already records such matches through the takeover path) that plays the new Fair CPU against the old and reports, per fighter and style pair and per level:

- rounds won;
- attacks started per minute and time spent neither moving nor attacking;
- hits per connected opening and damage per opening;
- how often a seen hit was followed up.

These numbers say whether a change made the CPU more active. Whether it feels like a person is the maintainer's call with a pad in hand.

## Settled in the question round

All of these are the maintainer's decisions (2026-11-18).

- **Mistakes at the top:** Very Difficult keeps today's rates: 97% clean motions (less under pressure) and 90% best choice. On long routes that is roughly one long combo dropped in six or seven.
- **Habit memory lasts one match**, all its rounds, and starts empty at the next, in every mode. An Arcade opponent or a CPU entry never knows the player from an earlier fight.
- **Leaning on what works:** Very Difficult and Difficult may repeat a winning option for as long as it keeps working; variety then only chooses among options about equally good. Normal and Easy put variety first.
- **What each level is tuned against:**
  - Easy loses to a newcomer who mashes and blocks a little.
  - Normal (the factory setting) loses to someone who knows their specials and one short combo.
  - Difficult needs solid defence and punishes to beat.
  - Very Difficult is the ceiling.
- **Dramatic Battle:** the boss and the ally each play the stronger game on their own. No teamwork between CPUs and no special two-on-one handling in this project. The measuring run reports Dramatic Battle so an unbeatable boss is seen.
- **The switch is developer-only:** a launch flag or config line, never a row in Options.
- **First pair:** Ryu in the SF6 era style proves the whole chain before the rest of the cast.
- **Ready gate:** the measuring run must show the new Fair CPU beating the old and more active for every pair and level, and then the maintainer plays it and says yes. Nothing is switched on without the maintainer's say.

## Constraints

- `fair_cpu.rs` tests and `tests/fair_cpu.rs` pin today's behaviour; many will be rewritten. The delay test ("nothing seen more recently than the delay changes the pad") must hold in its new two-delay form.
- ADR-0019 (Utility AI) and ADR-0025 (CPU entries) are affected; the glossary's **Fair CPU** entry gains focus, plans, the route book and habit memory. A new ADR records this design's decision.
- Three Fair CPUs can run at once in Dramatic Battle, and all of it re-runs under rollback, so choosing a plan and a route must stay cheap.
- Era style movesets are still changing (spec #138); the route tests are what keep the book honest through that.

## Left to the builder

- The route book's file format and how the Fair CPU loads it.
- How the search is widened and how long it may run.
- The plan script representation.
- All scores and level values, set by the measuring run and then by play.
- The order of work. A sensible one: the measuring run; situations and plans for one pair (Ryu, SF6); the route book and its tests; pressure and knockdown play; focus and folding in the Utility AI's commands; habit memory; level scaling; then the rest of the cast.
