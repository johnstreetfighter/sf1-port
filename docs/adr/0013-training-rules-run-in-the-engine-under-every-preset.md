---
status: accepted
---

# Training's rules run in the engine under every preset, Faithful 1987 included

Training has no rounds: nobody is knocked out, health refills, the clock never runs, and no round result, score or round-win marker shows (the maintainer, 2026-11-07). A knockout is decided on the hit's own frame, so these rules can't be applied from outside the machine. They are an overhaul-only gate on the translated damage and timer routines, keyed on the play mode the engine already reads, and their state lives in the machine's snapshot. That is the kind of change ADR-0004 allows. The maintainer chose to apply the same rules under the Faithful 1987 preset. So Training is the one place where Faithful 1987 no longer replays the arcade machine byte for byte. Arcade, Local and the faithful cabinet still do, and keep their MAME checks.

## Considered options

- Keep real 1987 rounds in Faithful 1987 Training: it stays exact, but Training then behaves differently depending on the preset, and a combo practised to zero still ends the match there.
- The front end tops up health between frames: forbidden by ADR-0004, and too late, since the knockout has already happened on the hit's frame.
- Catching the knockout and restoring the round start: no engine change, but both fighters snap back to their starting places mid-combo.

## Consequences

Any future check that Faithful 1987 is exact must leave out Training fights. The design is [`docs/design/2026-11-07-training-without-rounds-design.md`](../design/2026-11-07-training-without-rounds-design.md).
