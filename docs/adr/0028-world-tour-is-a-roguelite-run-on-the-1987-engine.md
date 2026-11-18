---
status: accepted
---

# World Tour is a roguelite run on the 1987 engine, within the three-fighter limit

On 2026-11-18 the maintainer asked to make World Tour much richer by turning it into a single-player roguelite run built around wild fight twists, *Rainbow Edition*-style mutators, hybrid cross-era builds, partner tagging, and retro-homage courses ([design](../design/2026-11-18-world-tour-roguelite-design.md)). Because every fighter, era style, costume, and palette starts unlocked outside World Tour, the fixed 18-node XP checklist of [ADR-0016](0016-dramatic-battle-and-composite-endings-on-the-1987-engine.md) left little reason to replay once cleared. We decided to replace World Tour's static campaign with a seeded **Tour run** across the five 1987 countries: fresh profiles start on vanilla `SF1` Ryu and Ken, players draft three-choice **Upgrade cards** after round wins, match wins, and **Tour courses**, match losses trigger escalating **Loss burns** (`1, 2, 3…` banked cards), and milestone/run clears earn **Starting unlocks** for future runs. Dramatic Battle and the twelve-fighter composite endings of ADR-0016 remain unchanged.

We chose running matches, Active Tag, and `2,048`-column side-scrolling Tour courses inside continuous `Fight` instances with per-slot `[TourBuild; 3]` runtime dispatch over two rejected approaches. Rebooting `Fight` between rounds, synthesizing custom 64-slot `DataBank` move tables per hybrid build, and running Tour courses on a separate `Screen` outside `FightMachine` broke native round flow and meter continuity while duplicating simplified fighter physics for courses. Keeping both the lead fighter and Tag partner permanently live in `P1_STRUCT` and `P2_STRUCT` all round simplified rapid tag-cancels, but widened risk across every `tasks.rs`, HUD, camera, and KO rule that expects the human player on `P1_STRUCT`.

Three boundary choices would surprise a later reader:

- **Active Tag, `1v2` Ambushes, and `2v1` Country Finales share the third fighter slot (`P2_STRUCT`, `0x1000..0x15ff`) rather than adding a fourth.** In `1v1` matches and Tour courses, Active Tag leaps the partner in through slot `2` before handing `P1_STRUCT` to the partner on a shared lifebar. In `2v1` Country Finales, a recruited Tag partner replaces the scripted AI ally in slot `2` on independent `960` Sub-HP bars, handing player control to the surviving partner if `P1` falls first and restoring both roles next round. In `1v2` Ambushes, where slot `2` holds the second CPU opponent, Active Tag swaps in-place on `P1_STRUCT`. Pre-translation scheme chords (`LP+HK` Classic, `Special+Macro` Modern, `Down,Down + Punch+Kick` Pneumatic) trigger Tag so `bindings.toml` adds no new button.
- **Move Grafting dispatches meterless attack specials and command normals at recognition time, with signature `SF6` command normals authored in `DataBank`.** Metered and era-system donor moves (`EX`/`OD`, `Drive`, `Focus`, `V-Skill`/`V-Trigger`, `Super Arts`) and stateful installs (`Denjin Charge`, stance switches) are excluded so grafted moves never require cross-era gauge conversion or secondary mini-bars.
- **World Tour matches lock out round restarts and mid-match quit-dodging.** `PauseKind::WorldTour` removes `RESTART ROUND` and replaces `QUIT TO MENU` with `FORFEIT MATCH`, and entering a match flushes `in_match = true` to `world_tour.toml` (`schema = 2`) until the match resolves so forfeits and mid-match force-closes enforce Loss burns. Tour courses never set `in_match`, drain no HP on hazards (penalizing `TIME` and position only), and never burn cards.

## Consequences

- Supersedes the 18-node World Tour graph, RPG stat/perk model, and `v1` `world_tour.toml` schema of [ADR-0016](0016-dramatic-battle-and-composite-endings-on-the-1987-engine.md); reading a legacy `v1` file initializes a fresh `v2` profile.
- Open Bonus Challenger and Fair CPU World Tour items (`#33`, `#51`, `#64`, `#215`) feed released challengers, stages, props, and the stronger Fair CPU into World Tour's roguelite pools rather than static nodes `19..33`.
- Faithful 1987 and every other play mode remain completely unchanged.
