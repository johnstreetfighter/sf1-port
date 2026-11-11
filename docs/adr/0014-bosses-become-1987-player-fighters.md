---
status: accepted
---

# The bosses become 1987 player fighters, and the modern rules are gates in the 1987 player code

In the 1987 engine, Ryu and Ken are one player program running two data tables; the move table a player structure points at is what makes him Ryu or Ken. SF1+ gives each of the ten bosses a table in Ryu's format, held in an extended data bank the translated player code reads through an overhaul-only gate, so all twelve fighters walk, jump, block and read commands through the same 1987 code. The modern rules (true combos, cancels, techable throws) are overhaul-only gates in that code, and the overhaul's CPU plays by feeding pad inputs into it. Faithful 1987 never sees the bank or the gates. (the maintainer, 2026-11-07, approach A of [`docs/design/2026-11-07-sf1plus-on-the-real-engine-design.md`](../design/2026-11-07-sf1plus-on-the-real-engine-design.md).)

## Considered options

- A new data-driven fighter engine (grown from the `sf1plus.rs` model) swapped in for the 1987 player code under the overhaul presets: the easiest to author and tune, with no format limits, but two engines, Ryu no longer the 1987 Ryu under the overhaul, and against the 2026-11-02 rule that overhaul mechanics act on the real 1987 fighters.
- A hybrid, the 1987 engine for movement and a new engine for attacks: the two-engine problem plus a seam between them.
- Keeping today's boss driving (each button starts one of the boss's CPU attack streams): no blocking, most bosses without a crouch, no command motions, and Joe's crouch on an untranslated branch.

## Consequences

Move data is written in the 1987 formats, frame records with their boxes, and any limit that's too small is extended through an overhaul-only gate, as ADR-0007 did for tiles. The boss-only driving path is deleted. Without the art pack the bosses can't be picked outside Faithful 1987, and netplay must check that both players hold the same pack.
