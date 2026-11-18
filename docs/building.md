# Building from source

Players do not need any of this: the [Windows release](https://github.com/johnstreetfighter/sf1-port/releases/latest) is ready to run. This page is for building the game yourself and for the development tools.

## Requirements

- Windows and a recent stable Rust toolchain (`x86_64-pc-windows-msvc`).
- Visual Studio 2022 or the Build Tools with the **Desktop development with C++** workload. Rust's MSVC target needs it for linking, and tracker module support compiles libopenmpt from C++ source (see [Tracker modules](#tracker-modules-and-the-c-build)).
- A legally obtained `sf` arcade ROM set (MAME's `sf`, US set 1). The game checks each of its 44 files against the size and CRC-32 in the embedded manifest.

## Build and launch

From the repository root:

```bash
cargo build -p sf-game --release
```

The executable is `target\release\sf-game.exe`. It looks for the ROM set in these places, in order:

1. `roms\sf` beside the executable
2. `%APPDATA%\sf1-port\roms\sf`
3. The path saved as `rom_folder` in `%APPDATA%\sf1-port\settings.toml`

The `--roms` option checks a folder or archive first:

```bash
cargo run -p sf-game --release -- --roms tools/reference/local/roms/sf
```

`sf-game --version` prints the edition name, version and source revision.

`cargo run -p sf-game --release -- smoke` opens the window, switches it to fullscreen and back, and reports presents and game frames per second and late refreshes over five seconds.

## Linux

The Linux build is the same game for x86_64 desktop Linux and Steam Deck. Its runtime baseline is Ubuntu 22.04 (glibc 2.35): the release executable is built inside an `ubuntu:22.04` container by [`tools/release/build-linux.sh`](../tools/release/build-linux.sh), so it runs on that system and newer ones, including Fedora 36 and later, Debian 12 and SteamOS 3.

To build on Linux yourself, install a C and C++ compiler, `pkg-config`, and the ALSA, udev, X11, Wayland and xkbcommon development packages (on Ubuntu: `build-essential pkg-config libasound2-dev libudev-dev libx11-dev libxcursor-dev libxkbcommon-dev libwayland-dev`), then run the same `cargo build -p sf-game --release`. The executable is `target/release/sf-game`.

At run time the game needs a Vulkan driver, ALSA sound (PipeWire and PulseAudio provide it) and X11 or Wayland. It looks for the ROM set in `roms/sf` beside the executable, then `~/.local/share/sf1-port/roms/sf`, then the saved `rom_folder`. The missing-ROM screen's BROWSE choices are Windows-only for now, so on Linux use one of those folders or `--roms`.

## Display timing

The game draws through the graphics card (Vulkan, or Direct3D 12 without it) and waits for the display's refresh. When the refresh is a multiple of 60 Hz, every game frame is shown for the same number of refreshes: two at 120 Hz, one at 60 Hz. Other rates, such as 144 Hz, run the game on a 60 Hz clock.

## Files the game keeps

Everything is in one folder per user: `%APPDATA%\sf1-port` on Windows and `~/.local/share/sf1-port` on Linux (`$XDG_DATA_HOME/sf1-port` when the desktop sets that variable). A first launch writes the three settings files with their defaults. Replacing or deleting the game's own folder leaves this one alone.

| File | Holds |
| --- | --- |
| `settings.toml` | Video, sound, devices, ROM folder, netplay, the cabinet's DIP switches |
| `match_config.toml` | Preset, toggles, roster, difficulty and round time |
| `bindings.toml` | Keys and pad buttons |
| `sf-game.log` | One launch: the settings in force, the ROM folder, the graphics adapter and refresh rate, pads seen or lost, any file that failed to load, and any untranslated-branch notice. Overwritten at every launch. |

## Asset packs

The Windows ZIP and a source checkout carry three packs under `assets`. A folder of the same name beside the ROM set takes precedence as a local override.

- `assets/sf1-port-art`: the approved SF1+ art. If the chosen pack cannot be loaded, the game logs the reason and uses ROM art.
- `assets/sf1-port-voice`: the overhaul announcer, played on a third emulated MSM5205. See the [announcer reference](reference/announcer.md) for the pack format and fallback behavior.
- `assets/sf1-port-music`: 30 native character arrangements, used by the overhaul presets. Valid scores beside the ROM set take precedence for the matching theme. See the [music pack](../assets/sf1-port-music/README.md).

## Tracker modules and the C++ build

The Jukebox plays tracker modules (MOD, XM, S3M, IT, MPTM, 669, MTM and the rest of libopenmpt's list) as well as OGG, WAV and FLAC. Modules go through [libopenmpt](https://lib.openmpt.org/libopenmpt/) 0.8.10, a C++ library. Its official source release is committed, as an unmodified subset, in `crates/sf-openmpt/vendor/libopenmpt` (about 6.5 MB of source, no binaries), and `crates/sf-openmpt/build.rs` compiles it into a static library with the `cc` crate. A fresh clone needs no download, no DLL, no vcpkg and no `PATH` setup.

The one requirement is a C++17 compiler from the workload above. Open a normal shell; `cc` finds the compiler itself through `vswhere`. The first build compiles about 150 C++ files and takes about a minute on a multi-core machine. Later builds reuse it. libopenmpt ends up inside `sf-game.exe`.

To build without libopenmpt, on a machine with no C++ compiler, turn off the `tracker` feature:

```bash
cargo build -p sf-game --release --no-default-features
```

That build plays OGG, WAV and FLAC. A tracker module in the `jukebox` folder then cannot play: the game logs one line saying the build has no tracker support, and the native theme plays, as it does for any file it cannot play. `-p sf-dev` takes the same flag.

A module loops where the module says (its restart position). A broken or empty module falls back to the native theme with a log line.

## Development launcher

`sf-dev` runs the translated machine directly, without the front end:

```bash
cargo run -p sf-dev --release -- power-on
```

Or straight into a fight:

```bash
# 1-player quick match: Ryu (P1) vs. CPU Retsu
cargo run -p sf-dev --release -- cpu

# 2-player versus quick match: Ryu (P1) vs. Ken (P2)
cargo run -p sf-dev --release

# Portrait character select. Arrows move, Enter confirms, Backspace steps back.
cargo run -p sf-dev --release -- select
```

The launcher reads its ROMs from `tools/reference/local/roms/sf` (see [tools/reference/README.md](../tools/reference/README.md)).

| Action | Default key |
| --- | --- |
| Toggle hitbox and 60-cell frame meter overlay | `F1` |
| Pause / resume | `Space` |
| Save snapshot / take over P1 or P2 (`replay`) | `F2` / `F3` |
| Rewind to bookmark | `F4` |
| Quit | `Esc` |

The launcher reads both pads too: the first pad plays P1 and the second P2, alongside the keyboard. It can rebind any of the 25 player or cabinet inputs (`p1_up`, `p1_down`, `p1_left`, `p1_right`, `p1_lp`, `p1_mp`, `p1_hp`, `p1_lk`, `p1_mk`, `p1_hk`, the `p2_*` equivalents, `coin1`, `coin2`, `start1`, `start2`, `service1`) from the command line, into the same `bindings.toml`:

```bash
cargo run -p sf-dev --release -- rebind <input> <Key>
```

## README screenshots

The README's stills are the game's own frames, captured headlessly with the local ROM set and written at 384×224:

```bash
cargo run -p sf-host --release --example readme_shots -- <out folder> [fight frames folder]
```

They are enlarged to 768×448 without smoothing before they go in `docs/media`.

## Tests

Run the full suite with `cargo suite`. Plain `cargo test` is the quick path for a single test file. See [AGENTS.md](../AGENTS.md).

## More

- [Technical overview and verification](technical-overview.md)
- [Full-game verification audit](reference/full-game-audit.md)
- [Translation coverage ledger](reference/coverage-ledger.md)
- [Domain glossary](../CONTEXT.md)
- [Windows release packaging](../tools/release/README.md)
- [Netplay](netplay.md) and [running a relay](relay-deployment.md)
- [Roadmap and project history](roadmap.md)
