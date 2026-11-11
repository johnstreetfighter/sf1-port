# Ship the game as its own binary, with screens as pure state machines and a shared host crate

The development launcher (`sf-dev`) is a 2,900-line tool of fixtures, recorders and listening kits that starts quick fights from gitignored reference captures, and the front end needed a home. We decided that the shipped game is a separate executable (`sf-game`) holding only the window loop; that every front-end screen is a pure state machine in the engine crate, taking menu inputs and returning a frame and a transition, tested without a window; and that window, audio, keyboard and gamepad glue moves out of the launcher into a shared host crate (`sf-host`) both binaries use. We chose this over extending the launcher because a shipped game must not depend on dev fixtures, screens must be testable before they are drawn, and later projects (the era picker, World Tour, the online lobby) should add screens rather than plumbing.

## Consequences

The launcher keeps its fixtures and subcommands unchanged and gains pads for free through the host crate. Two binaries share one copy of the host glue, so a change to bindings, audio or the window reaches both.
