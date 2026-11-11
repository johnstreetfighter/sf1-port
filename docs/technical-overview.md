# Technical overview and verification

This document covers the architecture, verification harness, developer tooling, repository layout, and completed ticket history for `sf1-port`. For gameplay features and player controls, see the root [README.md](../README.md).

## Architecture

The port is a direct routine-by-routine Rust translation of the original 1987 arcade programs (US set 1, six-button controls, MAME set `sf`), rather than a CPU emulator or a clean-room remake:

- **Translated game and sound programs.** The 68000 main program and both Z80 sound programs (the music/FM driver and the ADPCM sample player) are translated into Rust with each routine traceable to its original ROM addresses ([ADR-0001](adr/0001-recover-original-logic-in-rust.md)).
- **Hardware-level sound synthesis.** Only the sound synthesis chips are emulated: the YM2151 FM synthesizer runs through `ymfm`, and the two MSM5205 ADPCM decoders run through a cycle-accurate state model.
- **Two-tier overhaul boundary.** Optional gameplay improvements and multi-era expansions are separated from the faithful 1987 baseline through surgical core toggles in `sf-engine` and a data-driven overhaul layer above it ([ADR-0002](adr/0002-two-tier-overhaul-architecture.md), [ADR-0003](adr/0003-offline-mugen-extraction-and-pose-synthesis.md)).
- **Pure netplay and shared host.** `GameSession` exchanges typed transport commands/events. Its connection state, real-`Fight` rollback and confirmed spectator playback use no sockets or wall-clock timers. `sf-host` owns nonblocking UDP, LAN discovery, measured link timing, adapter selection and the optional directory worker; `sf-game` polls it each game tick ([ADR-0018](adr/0018-rollback-netplay-and-virtual-net-bus-on-the-1987-engine.md), [netplay guide](netplay.md)).

Netplay integration tests drive actual sessions, approved fighter tables and native
fights through `VirtualNetBus`, including loss, jitter, duplicates, reordering,
late spectators and shared room transitions. They compare confirmed checksums and
native snapshots. Host tests also run two sessions over real localhost UDP.
The current online scope is the twelve tournament fighters in 1v1 plus four
spectators; online Dramatic Battle remains gated on its local three-fighter mode.

## Verification against the reference baseline

Every translated slice is checked frame by frame against a pinned MAME 0.289 build running the verified `sf` ROM set ([reference-verification.md](reference-verification.md), [mame-baseline.md](reference/mame-baseline.md)):

| Checked | Result |
| --- | --- |
| Pinned scenarios (`tools/reference/scenarios/*.json`) | 137 scenarios (274 independent MAME 0.289 runs), each verified repeatable before comparison |
| Executed instruction coverage ([coverage-ledger.md](reference/coverage-ledger.md)) | 10,916 / 10,916 executed instruction PCs translated (9,299 `:maincpu`, 1,394 `:audiocpu`, 223 `:audio2`), 0 untranslated |
| Full 1-player campaign (`sf-full-1p-01`) | 44,200 frames and 873 full checkpoints from power-on through all 10 opponents, 4 bonus stages, ending, staff roll, ranking entry, and return to attract |
| Full 2-player session (`sf-full-2p-01`) | 21,300 frames and 112 full checkpoints from power-on through versus, mode-4 Ken carry-on, challenger join, mode-2 Ryu carry-on, continue, and game over |
| Both sound programs ([both-chips.md](reference/both-chips.md)) | YM2151 and dual MSM5205 streams verified across all recorded fights, full-game runs, and all 236 service sound-test commands |

The full-game completeness and fidelity audit, including the 9 documented MAME 0.289 vs. `sf-rs` divergences, the 5 adopted MAME 0.289 hardware defaults, human listening and playtest reviews, and the service-mode audit addendum, is recorded in [full-game-audit.md](reference/full-game-audit.md) (with the earlier first-playable audit in [milestone-audit.md](reference/milestone-audit.md)).

### Verification components

- **Scenarios** (`tools/reference/scenarios/*.json`): frame-indexed inputs from power-on, DIP switch settings, and observation schedules.
- **Reference captures** (`tools/reference/local/captures/*`): two MAME 0.289 runs per scenario recording per-frame screen hashes, sound-latch writes, full checkpoints (pixels and work/object/video/palette RAM), and hooked sound-chip streams. Kept local because they contain ROM-derived data.
- **Comparison runner** (`crates/sf-compare`): reports the first frame where a native run differs from the reference on any observable ([comparison-runner.md](reference/comparison-runner.md)).
- **Reference slice docs** (`docs/reference/*.md`): one document per verified slice recording the translated ROM addresses, verification evidence, and explicit boundaries.

## Developer commands and tooling

Run the full workspace test suite:

```bash
cargo test --workspace --release
```

`sf-host`'s `tracker` feature (tracker modules through libopenmpt) is on by default. Its tests are `crates/sf-host/tests/modules.rs`; the build without it has its own, which only run with the feature off:

```bash
cargo test -p sf-host --release --no-default-features
```

That checks that a build with no C++ compiler still plays OGG, WAV and FLAC, that a module is an error saying so, and that the Jukebox screen offers no module files (`tests/no_tracker.rs`, `tests/track_listing.rs`). Note that `cargo test --workspace --no-default-features` does not leave libopenmpt out: Cargo builds `sf-openmpt` as a workspace member of its own, and so needs the C++ compiler. Turn the feature off per crate (`-p sf-host`, `-p sf-game`, `-p sf-dev`) to build without it.

Run the Python reference-tooling unit tests:

```bash
python -m unittest discover -s tools/reference/tests -v
```

Without local ROMs and reference captures, the capture-backed acceptance tests skip cleanly and pass. See [tools/reference/README.md](../tools/reference/README.md) for local ROM placement and MAME capture scripts.

### Building libopenmpt (tracker modules)

`sf-host`'s `tracker` feature (default on; `sf-game` and `sf-dev` forward it) pulls in `crates/sf-openmpt`, which compiles libopenmpt 0.8.10 from the official source release (`libopenmpt-0.8.10+release.makefile.tar.gz` from lib.openmpt.org, SHA-256 `95073a10d33d0ed4352f41b68edccb9636002d2626f9fd2e00b191fcaabdd876`), trimmed to `common`, `soundlib`, `sounddsp`, `libopenmpt`, `src` and the three small decoders in `include`, with the C++ compiler and the `cc` crate. Nothing is downloaded at build time, and the result is a static library inside the executable. The steps on Windows are in the README ([Tracker modules and the C++ build](../README.md#tracker-modules-and-the-c-build)); the only requirement is the MSVC C++ compiler. The build defines `LIBOPENMPT_BUILD`, `MPT_WITH_MINIZ`, `MPT_WITH_MINIMP3` and `MPT_WITH_STBVORBIS` (libopenmpt's own makefile fallbacks when zlib, mpg123 and vorbis are absent), so compressed modules (MO3's Vorbis and MP3 samples, gzip'd modules) need no outside library.

To update libopenmpt, replace `vendor/libopenmpt` from a newer release's makefile tarball the same way (keep the directories above, plus `build/svn_version`) and run `cargo test -p sf-host`.

The alternatives, as tried on this machine (Windows 10, `stable-x86_64-pc-windows-msvc`, Visual Studio 2022 and 18 Community, no CMake or vcpkg):

- **The crates.io bindings** (`openmpt` 0.3.1, `openmpt-sys` 0.2.0, `libopenmpt-sys` 0.3.0) only declare the C API; none builds the library. `openmpt-sys` just emits `rustc-link-lib=openmpt`, and `libopenmpt-sys` finds an installed copy through `pkg-config`, which Windows lacks. They do not remove the build problem.
- **The official Windows development package** (`libopenmpt-0.8.10+release.dev.windows.vs2022.zip`) is import libraries plus DLLs: the executable then needs `libopenmpt.dll` and its four helper DLLs beside it (and beside every test executable), and a fresh clone needs a separate download and an extraction step. A missing DLL stops the game at start, not just the Jukebox.
- **vcpkg** is not installed here and adds a toolchain to set up.
- **The vendored source** (chosen) needs only a compiler, adds nothing to the shipped files, and is the same on any platform with a C++17 compiler.

### `sf-dev` CLI reference

In addition to the interactive game modes (`power-on`, `cpu`, and default `versus`), `crates/sf-dev` provides verification and diagnostic subcommands ([launcher.md](reference/launcher.md)):

```bash
# Rebind one of the 25 logical inputs and save %APPDATA%\sf1-port\bindings.toml
cargo run -p sf-dev --release -- rebind <input> <Key> [bindings.toml]

# Record an interactive keyboard session from the cpu (frame 2389) or versus (frame 2450) fixture
cargo run -p sf-dev --release -- record <cpu|versus> <session.tsv>

# Record the deterministic 1,011-frame synthetic session exercising all 20 player keys
cargo run -p sf-dev --release -- record-synthetic <session.tsv>

# Convert a recorded session TSV into a MAME scenario JSON
cargo run -p sf-dev --release -- convert <session.tsv> <scenario.json> [scenario-id]

# Replay a recorded session natively and verify it frame by frame against two MAME captures
cargo run -p sf-dev --release -- verify <session.tsv> <scenario.json> <run-a> <run-b>

# Render side-by-side MAME and native WAVs for the fixed fight (#49) or service sound test (#126)
cargo run -p sf-dev --release -- kit
cargo run -p sf-dev --release -- kit-sound-test

# Run the host window/audio smoke check or play the standalone round music phrase
cargo run -p sf-dev --release -- smoke
cargo run -p sf-dev --release -- music
```

When an interactive session reaches an original ROM branch that is not translated, the launcher prints the frame number and original 68000 PC to `stderr` and resets cleanly rather than guessing behavior.

## Repository layout

| Path | Contents |
| --- | --- |
| `crates/sf-engine` | Translated 68000 game program, pure front-end sessions, rollback protocol and deterministic virtual transport |
| `crates/sf-video` | ROM tile/sprite decoding and hardware-accurate layer compositor (`384x224`) |
| `crates/sf-audio` | Translated Z80 sound programs, YM2151 and MSM5205 chip models, stereo mixer |
| `crates/sf-compare` | Scenario loader, ROM manifest verifier, frame-by-frame comparison runner |
| `crates/sf-openmpt` | libopenmpt 0.8.10 (BSD-3-Clause), built from its vendored source into a static library by `build.rs`, with the few C calls the Jukebox's tracker modules need (`sf-host`'s `tracker` feature, on by default) |
| `crates/sf-dev` | Windows launcher, keyboard input mapper, live audio stream, session recorder/verifier |
| `crates/sf-host` | Window, audio, devices, persistence, and nonblocking UDP/discovery/directory host services |
| `crates/sf-game` | Standalone game loop over `GameSession` and the shared host services |
| `tools/reference` | MAME 0.289 capture scripts, winning-script search tool (`search_win.py`), coverage ledger generator (`coverage_ledger.py`), and 137 scenario JSONs |
| `docs/reference` | Per-slice technical documentation, `coverage-ledger.md`, `milestone-audit.md`, and `full-game-audit.md` |
| `docs/planning` | Batch specifications and dependency-ordered ticket breakdowns |
| `docs/design`, `docs/research` | Community research and architectural designs for the Overhaul and M.U.G.E.N expansions |
| `CONTEXT.md`, `docs/adr` | Canonical domain glossary and Architecture Decision Records |

## Completed faithful-port batches (Spec #1)

All 116 implementation and verification tickets of the faithful 1987 arcade port (#2-#90 and #122-#149) are complete:

- **Foundations (#2-#4):** Pinned MAME 0.289 baseline, `sf-compare` runner, and first rendered frame from decoded ROM graphics.
- **First slices (#5-#13):** Versus movement, standing punch, hit sound, round music phrase, Retsu's opening AI decision, and second-batch planning.
- **First playable fight (#14-#39):** Cooperative task scheduler (`0x3b050..0x3b2b2`), standing/crouching/jumping normals, *Hadoken*, *Shoryuken*, and *Tatsumaki Senpukyaku*, high/low blocking, Retsu's complete AI and hit reactions, rounds, timer, KO/time-over/draw/double-KO outcomes, both Z80 sound programs, vblank sound-latch timing, keyboard launcher, and the first-playable milestone audit ([milestone-audit.md](reference/milestone-audit.md)).
- **Faithful arcade-flow batch ([arcade-flow-batch.md](planning/arcade-flow-batch.md), #40-#90, #122-#149):**
  - **Power-on, attract, and DIP switches:** Hardware reset (`0x3b000..0x3b04e`), boot setup, warning screen, attract wall and title, attract ranking screen, `1,365`-record attract demo fight (`0x4030c..0x40334`) and round-end return (`0x3ce8c..0x3ceba`), complete attract loop (`0..=6000`), power-on fixed fight (`0..=5014`), pre-fight `IN1` PRNG folding (`0..=2910`), full boot DIP decode, difficulty/timer/attract-music/demo-sounds/coinage/continuation/buy-in DIPs, slow-game Speed DIP, Freeze DIP, and per-layer Flip Screen (#41-#43, #55-#61, #75, #77, #88, #89, #123, #127).
  - **Credits, start, country select, and versus flow:** Coin/service credit tasks, 64 `(Coin A, Coin B)` ratios, 9-credit lockout, coin counters, attract-to-start interrupt (`0x48228..0x48248`), 1P/2P Start, 2-country and 4-country select with all four 15-entry opponent tables, fresh 2P start into versus, P1 (`GAME_MODE = 2`) and P2 (`GAME_MODE = 4` Ken) versus-winner carry-on, mid-fight challenger join (`0x3f4a2..0x3f4da`, `0x423b8..0x4241c`), lose quote, continue countdown and rematch, ranking check, name entry (`0x42486..0x425c2`), ranking insertion (`0x425c4..0x4268a`), marked-entry blink (`0x42750..0x427e0`), game over, return to attract or start screen, and full `21,300`-frame 2P session `sf-full-2p-01` (#44, #45, #52, #65, #66, #73, #74, #79-#82, #85-#87, #122, #128, #142).
  - **Combat breadth, opponents, bonus stages, ending, and full 1P campaign:** Ken's 16 twin attack functions and command trackers (#47), Ryu's specials, crouching/jumping attacks, blocks, held inputs, and 3-round splits/draws against live CPU Retsu (#53, #54, #72), winning-script search tool (`search_win.py`, #46), all remaining tournament opponents (Geki `0x16bfc`, Joe `0x1734c`, Mike `0x170dc`, Lee `0x175bc`, Gen `0x1782c`, Birdie `0x17a9c`, Eagle `0x17d0c`, Adon `0x17f7c`, and Sagat `0x181ec` with Tiger Shot `0x45ace` / `0x43652`), all four bonus stages (Japan and England plank/block timing bonus `0x40d30` / `0x41a02`, USA and China 10-second concrete-stack bonus `0x41740` / `0x41fe6`), inter-country travel map (`0x3f812..0x3f8e0`), stage-4 ending cinematics (`0x44a40..0x44c1c`), congratulate scroll (`0x44d60`), 8-block staff roll and `THE END` (`0x44f58..0x452dc`), stage-5 ranking/game-over transition, and full `44,200`-frame 1P campaign `sf-full-1p-01` (#67-#69, #83, #84, #124, #125, #129-#141).
  - **Sound, controls, service mode, and audits:** Native-latch sound across all recorded fights and both full-game runs (#48, #143), every service-test FM effect and special command (#62), every tune loop/end (#63), tune priority transitions, saved-tune slot (`0xc300`) and music stops (#78), every ADPCM sample across both channels and banks (#64), fixed-fight and full sound-test listening kits and human listening reviews (#49, #70, #126), session recording/verification and human keyboard playtests (#50, #71), TOML-persisted remappable keyboard bindings (#76), PC-coverage ledger (#51), service mode from power-on (`0x4831a..0x4902e`: RAM/ROM check, colour bars, Sound Test, Voice Test, Game Option DIP display, Scroll Test, and `CH0`/`CH1` Object Test, #145-#148), and the full-game completeness and fidelity audit (#144, #149).

### Explicitly deferred in the faithful baseline

As documented in [full-game-audit.md](reference/full-game-audit.md) (§7.4.4), the faithful baseline explicitly defers:

- 38 of 67 CPU hit-function table entries in `0x46472..0x480d0` that were never selected by the CPU AI across the recorded winning chains (#1 decision 8; any unreached entry still calls `bus.note(pc)` if ever triggered).
- Full 10-opponent winning chains starting from USA, England, or China (#40 decisions 2 and 11), Ken's mode-4 1P campaign past Retsu (#40 decision 10), and P2 high-score name entry (#40 decision 3).
