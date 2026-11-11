# Street Fighter Rust PC port

## Objective

Create a native Rust PC port of the original arcade Street Fighter by recovering and translating its original gameplay and sound-program logic. The completed port should reproduce the complete original arcade experience, including full progression and two-player modes.

Windows is the first target, with other platforms kept possible. Initial use is personal and local. Distribution and delivery of original game data are deferred decisions.

## Reference and fidelity

- Reference version: US set 1 with six-button controls, MAME set `sf`.
- Practical comparison baseline: a pinned MAME version running verified reference ROMs. Investigate known emulation discrepancies instead of silently adopting them as original behavior.
- Gameplay: matching outcomes and timing under the same inputs and starting state. A one-frame difference in when an attack hits is a fidelity failure.
- Graphics: exact original game pixels before display scaling.
- Audio: matching events, timing, pitch, and character; byte-identical output audio is not required.
- Preserve original awkward controls, timing quirks, and bugs. Any later gameplay improvements must be optional.

All 44 required reference ROM entries passed size, CRC32, and SHA1 checks. See [reference verification](reference-verification.md) for the source commit, archive hash, method, and limits. The runnable MAME build for comparisons has not yet been selected or pinned.

## First playable milestone

Launch directly into one fixed fight through a development entry point. Demonstrate faithful movement, attacks, opponent behavior, damage, graphics, and sound. Use keyboard controls.

The original startup, title, credit, and start flow can follow this milestone. A development entry point is an intermediate aid, not the final arcade experience.

## Completed port

Include the original arcade flow, full game progression, and original two-player modes. Require remappable keyboard controls; gamepad and arcade-stick support are deferred.

## Recovery approach

Follow [ADR-0001](adr/0001-recover-original-logic-in-rust.md): traceable native translation with original addresses and behavior documented, followed by refactoring after comparisons pass. Sound-chip emulation may supply synthesis, while the original sound-program logic is translated into Rust.

## Current evidence and remaining engineering work

The ROM inputs are verified; game execution, native recovery feasibility, and fidelity are not yet demonstrated. No Rust implementation or runnable comparison harness exists yet.

Implementation planning must select and pin the runnable comparison baseline, establish repeatable initial states and input sequences, and determine the recovery path into the fixed fight. Renderer, audio libraries, code structure, and exact development fight are engineering choices still to be evaluated against this brief.

## Review status

The user confirmed this consolidated brief on 2026-10-15, completing the design interview. Implementation has not started.
