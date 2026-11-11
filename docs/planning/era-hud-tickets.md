# Era HUD tickets

Spec: #216 (`docs/planning/era-hud-spec.md`). Design: `docs/design/2026-11-04-era-hud-and-callouts-design.md`. ADR 0011. Published 2026-11-04 as sub-issues of #216 with native blocking edges, all `ready-for-agent`.

| Ticket | Issue | Blocked by | Delivers |
|---|---|---|---|
| EH1 | #217 | none | The era HUD description replaces the placeholder strip. The six era models fill it in and it draws in Ruled gauge art in the bottom corners. A dev preview of two era sessions. |
| EH2 | #218 | none | The overhaul fight layout: TIME under the health bars, scores off in Local and Training, the Faithful 1987 preset untouched. |
| EH3 | #219 | EH1 | Blocks and Plate gauge art, and `GAUGE ART` in VIDEO and the pause box. |
| EH4 | #220 | EH1, EH2 | Call-outs, the fixed screen regions, the era events and the no-overlap stress test. |
| EH5 | #221 | EH4 | The combo counter in real fights. Training shows damage and has a hold row. |
| EH6 | #222 | EH5 | FIRST ATTACK, REVERSAL and PERFECT in real fights. |
| EH7 | #223 | EH4 | The Training overlay kept clear of the HUD. |
| EH8 | #224 | EH4 | The announcer's third MSM5205, 4 kHz ADPCM, the voice-pack loader, the priority queue and the `ANNOUNCER` volume. |
| EH9 | #225 | EH8 | The designed voice, its pipeline and the voice pack. The maintainer picks the takes, and the generations cost money. |
