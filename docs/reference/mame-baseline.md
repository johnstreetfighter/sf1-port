# Pinned MAME baseline and the fixed-fight reference scenario

This document records the runnable comparison baseline selected for
issue #2, the exact fixed
fight it reproduces, and how the captures answer each acceptance criterion.
It complements [reference verification](../reference-verification.md), which
verified the ROM inputs before any emulator was chosen.

## Selected baseline

- **Version**: MAME 0.289 (`mame -version` reports `0.289 (mame0289)`), the
  latest official stable release on the selection date (2026-10-15).
- **Build**: official Windows x64 binary package `mame0289b_x64.exe`
  (87,626,249 bytes, SHA256
  `a1aa7912168c9d1b05e611906bc21b8b9be3935822aead36d12a1da363150b7d`),
  downloaded from the mamedev.org release page, which serves the official
  GitHub release artifacts. The package was extracted locally with 7-Zip; it
  is not committed.
- **Why this build (observed behavior)**: it is the current official stable
  binary from the project that defines the `sf` driver; it runs the verified
  ROM set headlessly (`-video none -sound none`) at ~6x real time; its Lua
  engine exposes everything the capture harness needs (verified by smoke
  test: frame callbacks, per-pixel and whole-screen pixel reads, ioport
  field injection, address-space reads, file output). Building from source
  was rejected for this ticket: the official binary is a cleaner pin, and
  the driver source at tag `mame0289` is available read-only for reference.
- **ROM checks under the pinned build**: `mame sf -verifyroms` reports
  `romset sf is good` (the same 44 entries verified byte-for-byte in
  [reference verification](../reference-verification.md); the ROM
  definitions in `src/mame/capcom/sf.cpp` at tag `mame0289` match the pinned
  verification commit).

## Machine configuration

The runner owns a fixed option set (see `tools/reference/run_scenario.py`):

```
mame sf -rompath <local roms> -video none -sound none -nothrottle \
        -skip_gameinfo -noreadconfig -nowriteconfig \
        -cfg_directory <capture>/cfg -snapshot_directory <capture>/snap \
        -autoboot_script <generated driver> -autoboot_delay 0 \
        -seconds_to_run <safety cap>
```

- Headless (`-video none`): the screen device still renders every frame;
  `video:snapshot_pixels()` returns the full 384x224 ARGB buffer and
  `screen:pixel(x, y)` reads individual pixels (both smoke-tested).
- `-sound none`: sound chip synthesis is not advanced, but both sound CPUs
  execute normally and the sound command latch (see below) stays fully
  observable. Sound observations in this ticket are latch commands and
  music-driver RAM state, not synthesized audio.
- `-nothrottle`: frame pacing is emulated, not wall-clock; frame-indexed
  inputs are unaffected by host speed.
- DIP switches: factory defaults; the driver reads back and records
  `DSW1=0xdfff`, `DSW2=0xffff` into every capture's `meta.tsv`.
- Input injection: frame-indexed logical inputs are applied through
  `ioport_field:set_value(1)` / `clear_value()`, resolved by (port, bit
  mask). Writing the port's live value directly was tried first and
  **observed to have no effect** (MAME recomputes the port from field
  values before the game reads it); the field-level path was confirmed by
  the coin sound and game start in calibration.

Driver facts used by the harness, from `sf.cpp` at tag `mame0289`:

- Main 68000 @ 8 MHz; screen 384x224 visible @ 60 Hz; vblank IRQ1.
- Work RAM `0xff8000`-`0xffdfff` (24 KiB), object RAM `0xffe000`-`0xffffff`
  (8 KiB), video RAM `0x800000`-`0x800fff`, palette `0xb00000`-`0xb007ff`.
- Sound command latch written by the main CPU at `0xc0001d`; read by the
  music Z80 at `0xc800` and the sample Z80 at I/O port `0x01`. The driver
  polls the latch each frame with a no-side-effect (`readv`) read, so
  polling never consumes a pending command.
- Music Z80 ("audiocpu") RAM `0xc000`-`0xc7ff`; sample Z80 ("audio2") plus
  YM2151 and 2x MSM5205 complete the sound hardware.
- The `sf` set (US) uses the six-button input ports `IN0`/`IN1`/`SYSTEM`
  and, unlike `sfjp`, has no protection MCU.

## The fixed fight

`tools/reference/scenarios/sf_fixed_fight_01.json`: **1P Ryu vs CPU Retsu
(Japan stage), from power-on through the end of round 2**, 5000 frames
(~83 emulated seconds, ~10 s wall). The fight is "fixed" by a fixed input
timeline; every anchor below was calibrated from captured pixels and latch
events, not assumed:

| Frame | Observation (from captures) |
| ----: | --------------------------- |
| 0-793 | Boot: reset (`0-7`) and FBI warning types out, holds, clears (`8-793`) |
| 794 | Attract entry: layers off (`gfxctrl=0x00`), copyright and `INSERT COIN` written to video RAM |
| 795-1139 | Attract: CAPCOM wall scroll sequence (`0x1c` on the latch at 976) with blinking `INSERT COIN` |
| 1140-1287 | Attract: three title text blocks over the wall (90 + 29 + 29 frames) |
| 1288-1647 | Attract: title logo over the wall (`1288-1347`, 60 frames), then on black (`1348-1647`, 300 frames; see [attract](attract.md)) |
| 1450 | `coin1` pressed; coin sound commands on the latch at ~1468 |
| 1520 | `start1` pressed; start sound `0x34` at 1522 |
| ~1830-1865 | **Country select** (Japan/USA); exits on timeout at 1865 with default Japan (`0x3f` on the latch at 1866) |
| 1866-1957 | Pre-fight event 2/3: Japan country screen (`0x8f` voice at 1868; `start1` at 1900 falls inside event 3 and has no effect; see [prefight](prefight.md)) |
| 1958-2109 | Pre-fight event 4: Retsu stage intro (`0x33` at 1959; `1980` is inside this 150-frame stage intro) |
| 2110-2278 | Pre-fight event 5: "ROUND 1" (`0x97`/`0x0b` at 2117-2118) and 49-column wipe (`0x27`/`0x3f` at 2275/2277) |
| 2279-2389 | Round 1 HUD/walk-in (`0x28` stage music at 2280, `0x94` "FIGHT!" at 2361, fight loop entered at 2389) |
| 2400-2580 | `p1_right` held: Ryu walks toward Retsu |
| 2590, 2630 | `p1_button1`, `p1_button4`: two attacks; punch impact sound at ~2602 |
| ~3328-3364 | CPU KOs Ryu (blow sound `a1/87/04`, round-end jingle `09/a7`) |
| ~3840 | Round 2 begins (music `0x28` again) |
| ~4772-4809 | CPU KOs Ryu again; round-2 end jingle; match decided 2-0 |
| 5000 | Scenario bound |

Round timer observations: displays 100 at fight start, decrements about
one unit per second (97 at frame 2460, 58 at frame 3360). The round-1 KO
at ~3328 leaves the timer at ~58.

Two attract facts that shaped the scenario, established by observation:

- The attract loop contains a **demo fight** on the same stage; it is CPU
  vs CPU and ignores credits. Early calibration inputs fired during the
  boot warning were ignored entirely, which is why coin/start are placed
  inside the title window.
- The country select auto-confirms Japan on a timeout at frame 1865; the
  scenario's `start1` press at 1900 falls inside event 3's country screen and
  has no effect (verified byte-for-byte in [prefight](prefight.md)).

## Repeatable command

From the repository root:

```
python tools/reference/run_scenario.py tools/reference/scenarios/sf_fixed_fight_01.json --out tools/reference/local/captures/<run-name>
```

One run produces, in the capture directory:

- `frames.tsv` — 5000 rows: frame, downsampled screen hash (stride 4),
  sound latch value, latch-changed flag, and every 10th frame CRC32 of the
  32 KiB main work/object RAM and of the 2 KiB music-driver RAM.
- `checkpoints.tsv` + `pixels_<frame>.rgba` — full 384x224 ARGB pixel dumps
  and their CRC32 at 13 documented frames, plus `.png` previews for humans.
- `ram_<frame>_<region>.bin` — raw dumps of work+object, video, palette,
  and music RAM at each checkpoint.
- `meta.tsv` — DIP readback, screen geometry, frames executed.
- `summary.json` — runner metadata (MAME version, wall time, per-file
  SHA256). Tooling, not an observation.

## Determinism

Two fresh MAME processes (separate invocations, no shared state) running
the command above produced **byte-identical observations: 68 files match**
(`python tools/reference/compare_captures.py <dir-a> <dir-b>` →
`IDENTICAL: 68 files match byte-for-byte`). That covers all 5000 per-frame
records, all checkpoint pixels, and all RAM dumps.

Sensitivity check: the same scenario with the punch shifted 10 frames
later is detected (`checkpoints.tsv` differs at the affected checkpoint's
screen CRC32), so the identical result is not an artifact of a pipeline
that cannot see change.

Nondeterministic or unavailable observations, explicitly:

- None observed within the scenario scope; every compared artifact matched.
- The sound latch poll samples once per frame; multiple distinct commands
  within a single frame would collapse to the last one (not seen here: all
  observed commands lasted at least one frame and ended with `0xff`).
- Synthesized audio (YM2151/MSM5205 output) is not captured under
  `-sound none`; the latch command stream and music-driver RAM CRC are the
  sound observations for this ticket.
- `summary.json` records wall-clock time and is excluded from comparison
  as tooling, along with the generated driver (embeds the output path) and
  PNG previews (derived from the authoritative `.rgba` dumps).

## Native initialization correspondence

Later native slices initialize directly into the fixed fight; they must
not depend on replaying this boot path, and MAME save states cannot be
loaded by native code (none are used anywhere in this harness). The
correspondence for "the same relevant starting conditions" is the
**round-1 fight-start state at frame 2280**, observable in the captures:

- Fighters at their initial positions facing each other on the Japan stage
  (checkpoint pixels `aae397c7`).
- Both health bars full; score 0; round timer displaying 100.
- Stage music command `0x28` issued on the sound latch at frame 2280.
- Country selection resolved to Japan; round counter at round 1.
- The complete machine-level state is preserved, local-only, as the RAM
  dumps `ram_002280_{work,video,palette,music}.bin` (40 KiB total) plus
  the per-frame RAM CRC32 stream leading to them.

The native side's task is to initialize *equivalent logical state* (its
own representation of positions, health, timer, round, RNG/AI state, sound
state), not to byte-copy MAME RAM. The dumps exist so recovery work can map
which original addresses carry which meaning; until that mapping exists,
the per-frame screen hash and latch stream are the comparison surface.

## Data separation

Committed: scenario JSON, the Python tooling and its tests, the Lua driver
template, this document. Local only (`tools/reference/local/`, excluded
from Git): the MAME distribution and installer, extracted ROMs, all
captures including RAM dumps and pixel data. No original game data or
data-bearing capture is committed.
