# Recover original game logic into native Rust

The project will recover and translate the original arcade Street Fighter gameplay and sound-program logic into native Rust, rather than executing those programs through CPU emulation or recreating their behavior from scratch. Sound-chip emulation is allowed for synthesis. This defines the intended kind of PC port; the scope of the recovery effort remains to be established.

During recovery, favor a translation traceable to original program addresses and documented behavior over immediate idiomatic restructuring. Refactor after behavior comparisons pass, so changes can be checked against a faithful baseline. Preserve original quirks and bugs; later gameplay improvements must be optional.

See the [project brief](../project-brief.md) for release scope, milestones, and acceptance criteria, and [reference verification](../reference-verification.md) for the verified inputs.
