---
status: accepted
---

# Training's coach reads the engine's own command state, and the player's inputs only word its fixes

Training's coach tells a player whether a tried move came out and, when it didn't, why. It must never contradict the game: a fix that says TOO SLOW about an input the engine took would teach the wrong thing. We decided that the engine decides: the move's command tracker (or a later move's recognizer) says that an attempt ended and at which step, the ROM's pattern table says which direction it was waiting for, the busy gate says whether the fighter could act, and the fighter's current move says whether the move came out, all read and never written (ADR-0004). The 1987 trackers leave no mark for a wrong direction and hold nothing about the button, so the player's raw inputs choose only the words (TOO SLOW, MISSED ↘, PRESS P SOONER, BUTTON TOO EARLY), and a test replays recorded attempts to hold each fix's claim to what the engine does.

## Considered options

- A separate input checker for the coach: quick, but a second copy of the rules drifts from the engine's.
- Replaying each miss against a snapshot with small corrections until the move comes out: never wrong, but many replays per miss, and several corrections can work.
- The trackers alone: never wrong either, but its fixes stop at TOO SLOW · GOT TO STEP 2, NO BUTTON and WAIT TO RECOVER.

## Consequences

Every move that reaches real fights later (an era style's supers and system moves, a boss's specials, throws) needs a read-only view of its own recognizer before the coach can teach it. The design is [`docs/design/2026-11-06-move-list-design.md`](../design/2026-11-06-move-list-design.md).
