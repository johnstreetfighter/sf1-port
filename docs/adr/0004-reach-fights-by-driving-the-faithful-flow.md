# Reach fights by driving the faithful flow, never by writing fight state

The overhaul front end needs to start Arcade, Local and Training fights with chosen fighters, and the translated 1987 program has no entry point for "start a fight as this fighter in this country". We decided that the front end reaches every fight by feeding the running faithful machine the cabinet inputs a player would (coin, Start, the country flag), stepping the hidden frames, and handing over at the real pre-fight; the attract under the front end is the faithful machine with every toggle off; and fighter choice reaches the engine only through the roster settings the engine already reads. We will not add an engine entry point that writes work RAM, because every byte of fight state would then be a guess outside the verified flow, the byte-identical faithful path would gain a second way in, and ADR-0001's rule that nothing is guessed would break at the front door.

## Considered options

- Write the game-mode, fighter and country bytes directly and jump to the pre-fight task: instant, but unverifiable and a standing temptation to patch state.
- Run menus inside the 1987 machine as translated tasks: authentic, but puts overhaul UI on the byte-identical path and budgets menu state out of 64 KB of work RAM.

## Consequences

Starting a fight costs a few hundred hidden frames, stepped in milliseconds. Anything the front end wants the engine to do differently (a stand-in opponent, a free continue) is either an input it can feed or an overhaul-only gate on the translated path, never a RAM write.
