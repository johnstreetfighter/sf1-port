# Power-on: reset, task spawning, the warning screen, the attract title, and the fixed fight from frame 0

This document records the boundary established by
issue #41 (`F1` of the
arcade-flow batch), extended through the warning screen, the boot-to-attract
join at frame `794`, and the attract title through frame `1647` by
issue #55 (`F15`), and extended
through the full fixed fight (`0..=5014`) and non-coin `IN1` random-state
folding (`0..=2910`) with no checkpoint fixture by
issue #89 (`F49`): a power-on
Street Fighter machine (`PowerOnMachine`) built from the program ROM alone with
zeroed RAM and no checkpoint fixture, verified from frame `0` through the
attract title (`1647`) and from frame `0` through the entire fixed fight's
match decision (`5014`), with both sound programs driven from power-on on the
native latch with no pre-roll. Translated in `crates/sf-engine/src/tasks.rs`,
`crates/sf-engine/src/fighter.rs`, `crates/sf-engine/src/run.rs`,
`crates/sf-audio/src/lib.rs`, `crates/sf-audio/src/check.rs`, and
`crates/sf-dev/src/main.rs` (`power-on`).

## Reference captures (`sf-boot-01`, `sf-attract-01`, `attract-01-hook`, `sf-fixed-fight-power-on-01`, `sf-rng-inputs-01`)

- **Boot scenario (`sf-boot-01`):** `tools/reference/scenarios/sf_boot_01.json`
  (power-on, no inputs, default DIP switches `dsw1 = 0xdfff`, `dsw2 = 0xffff`,
  frames `0–829`, full checkpoints at every frame `0–799`). Two independent
  MAME 0.289 runs (`boot-01-run-a`, `boot-01-run-b`, local-only) match
  byte-for-byte across all 4,003 observation files, and every `frames.tsv` row
  `0..=829` equals `fight-run-a`'s.
- **Attract scenario (`sf-attract-01`):**
  `tools/reference/scenarios/sf_attract_01.json` (power-on, no inputs, default
  DIP switches, frames `0–1900`, full checkpoints at every frame `794–1900`).
  Matches `sf-boot-01` on the shared checkpoint window `794..=799` and on
  `frames.tsv` rows `0..=829`.
- **Audio hook capture (`attract-01-hook-a`, `attract-01-hook-b`):** two
  independent deterministic MAME 0.289 audio-probe runs of `sf-attract-01`
  with `--hook-window 0 1648 --hook-devices ymsnd,msm1,msm2 --no-wav`, matching
  byte-for-byte across `audio_taps.tsv` (`73,246` samples-CPU writes and
  `42,759` YM2151 writes over `0..=1647`), `hook_msm1.f32` and `hook_msm2.f32`
  (`10,552,321` samples each at 384 kHz; `21,104,642` total), `hook_ymsnd.f32`
  (`1,536,957` samples at 55,930 Hz; `1,536,211` in `0..=1647`), and the
  4,123 sorted rows of `hook_index.tsv`.
- **Fixed fight from power-on (`sf-fixed-fight-power-on-01` + `sf-fixed-fight-audit-01`):**
  `tools/reference/scenarios/sf_fixed_fight_power_on_01.json` records full
  checkpoints on every frame `0..=2389` under `sf-fixed-fight-01`'s exact
  inputs (`coin1` at `1450..1462`, `start1` at `1520..1532`, two-country select
  timeout to Japan at `1761`, pre-fight country and stage intros, and `ROUND 1`
  / `FIGHT`), joining seamlessly with `fight-audit-run-a` (`2389..=5014`) so
  every single frame `0..=5014` (`5,015` frames) has a full checkpoint. Two
  independent MAME 0.289 runs (`fixed-fight-power-on-01-run-a`,
  `fixed-fight-power-on-01-run-b`) match byte-for-byte across all `11,953`
  observation files.
- **Pre-fight `IN1` random-state folding (`sf-rng-inputs-01`):**
  `tools/reference/scenarios/sf_rng_inputs_01.json` (`0..=2910`, full
  checkpoints on all `2,911` frames) adds `p2_left` at `1200..1220` during the
  attract title (`GAME_MODE == 0`, folded via `0xff868e`) and `p1_right` at
  `2050..2070` during the stage-1 intro (`GAME_MODE == 2`, folded via
  `0xff868f`), shifting `0xff8e4a..0xff8e51` at frame `2389` from
  `[0xa9, 0xeb, 0x5f, 0x66, 0x63, 0xfb, 0x45, 0xbe]` to
  `[0xb0, 0x16, 0x18, 0xf6, 0xe7, 0x13, 0xf9, 0x7e]` and running through Retsu's
  shifted AI decisions, round 1's KO at `2861`, and win award at `2897`. Two
  independent MAME 0.289 runs (`rng-inputs-01-run-a`, `rng-inputs-01-run-b`)
  match byte-for-byte across all `14,558` observation files.
- **Evidence label:** `PO, no input` (`sf-boot-01`, `sf-attract-01`) and
  `PO, scripted` (`sf-fixed-fight-power-on-01`, `sf-rng-inputs-01`).

## MAME power-on state at frame 0

Per Acceptance 3, MAME's power-on contents are checked against
`sf-boot-01`'s frame-0 dumps in every region the reset cannot have modified
by the end of frame 0:

- **Object RAM (`0xffe000–0xffffff`):** all `0x00` in `ram_000000_work.bin`
  (`0x6000..0x8000`).
- **Video RAM (`0x800000–0x800fff`):** all `0x00` in `ram_000000_video.bin`.
- **Palette RAM (`0xffb800–0xffbfff`):** all `0x00` in
  `ram_000000_palette.bin`.
- **Uncleared work-RAM suffix (`0xffc564–0xffdfff`, 1,703 longwords):**
  all `0x00` in `ram_000000_work.bin` (`0x4564..0x6000`), matching the
  already-cleared prefix (`0xff8000–0xffc563`, 4,441 longwords).

`PowerOnMachine` therefore starts with zero in every RAM region.

### Frame-0 screen snapshot and frame-1 scanout hash

Before the 68000 reaches `0x3b77e` (`jsr $3b644.l` -> `gfxctrl_w`) in
frame 1, MAME's frame-0 `screen_update` (`pixels_000000.rgba`, CRC
`a285fa10`, scanned out as frame 1's `screen_hash = 78865155`) renders the
hardware's pre-`gfxctrl_w` device state (`sf-mame0289.cpp:266–274, 341–359,
456–470, 745–751`):

1. `sf_state::video_start()` (`sf-mame0289.cpp:266–274`) creates
   `m_fg_tilemap` and `m_tx_tilemap` with `tilemap_t::m_enable = true`, and
   `sf_state::machine_reset()` (`sf-mame0289.cpp:745–751`) sets
   `m_active = 0` without calling `enable()`, leaving the foreground and
   text tilemaps enabled (`active = 0x48` in `screen_update` at
   `sf-mame0289.cpp:456–470`).
2. `palette_device::device_start()` (`emupal.cpp`) initializes its decoded
   pens to the 8-color RGB cube
   (`((i & 1) * 0x0f00) | (((i >> 1) & 1) * 0x00f0) | (((i >> 2) & 1) * 0x000f)`)
   before any write to `m_paletteram` (`paletteram16_xGGGGGRRRRRBBBBB_word_w`),
   while `m_paletteram` itself (`ram_000000_palette.bin`) remains `0x00`.

In frame 1, `0x3b77e` and `0x3b784` write `gfxctrl_w`
(`sf-mame0289.cpp:341–359`), syncing MAME's tilemap enable flags to
`m_active = 0` (`c6a9e1c5` from `pixels_000001.rgba` onward).

## What is translated

### 1. Reset path (`0x3b000–0x3b04e`)

- **`0x3b000–0x3b01a`:** `A6 = 0xc00000`, `A5 = 0xff8000`, `A4 = 0xffe000`,
  and the 6,144-longword clear of `0xff8000–0xffdfff` (`D0 = 0x17ff`,
  `clr.l (A0)+; dbra D0, $3b018`) with interrupts masked (`SR = 0x2700`).
- **`0x3b01e–0x3b02a`:** self-test DIP check (`dsw1 & 0x8000 == 0` jumps to
  `0x4834e`, reported).
- **`0x3b030` -> `0x3b2e4` (`decode_coinage`):** decodes `dsw1` bits `0–2`
  and `3–5` through the 8-word table at `0x1456a` into the two coin slots'
  coinage words (`0xff863c`, `0xff8644`; `0x0101` with default `dsw1 = 0xdfff`).
- **`0x3b036` -> `0x3b77a` (`boot_init`):** pulses `gfxctrl` bit 0 via
  `0x3b644` and `0x3b690` (`0xff8682 = 0`), zeroes the sound queue indices
  (`0xff865e`, `0xff8660`), fills the 16 sound-queue words at `0xff8662`
  with `0xffff` via `0x3b914` (`fill_words_16`), and queues the three boot
  commands `0x27`, `0x3f`, `0x80` via `0x3b7a8` (`queue_boot_sounds`) and
  the unfiltered entry `0x3b802` (`Bus::queue_sound_unfiltered`).
- **`0x3b03c–0x3b044` -> `0x3b20a` (`Scheduler::spawn`):** spawns slot 0 at
  `0x3c0c4` (`state = 1, sleep = 0`).
- **`0x3b04a–0x3b04e`:** `move #$2000, SR` releases the held vblank before
  jumping to the scheduler at `0x3b19a`.

### 2. Scheduler pieces (`0x3b19a–0x3b2be`)

- **Rescan from slot 0 (`0x3b19a`):** after every task yield (`TRAP #2` at
  `0x3b24e`, `bra $3b19a` at `0x3b286`), the scheduler rescans from slot 0
  rather than continuing a single pass, so any lower-numbered slot made
  ready (`state = 1` or `2`) by a higher-numbered task runs in the same
  frame.
- **New-task entry (`0x3b1e8`):** a slot in state `1` transitions to state
  `4` (`sleep = 0`) and enters its task body (`0x3c0c4` for slot 0).
- **Spawn (`0x3b20a`, `Scheduler::spawn`):** writes `state = 1, sleep = 0`
  to slot `D0`'s header (`0x3b2be`, `slot_address`).
- **Kill (`0x3b230`, `Scheduler::kill`, trap `#1`, unexercised):** clears
  slot `D0`'s state byte (`+0` at `0xff8006 + 0x10 * D0`) and drops its
  continuation.
- **Current slot address (`0x3b2b8`, `current_slot_address`):**
  `0xff8006 + 0x10 * [0xff8001]`.

### 3. Boot task's first half (`0x3c0c4–0x3c20a`)

- **`0x3c0c4–0x3c0c8`:** sets slot 0's stack and sleeps 1 frame (`0x3b51a`).
- **`0x3c0ce`:** increments `0xff8622` (`0x0000 -> 0x0001`).
- **`0x3c0d2–0x3c10c` -> `0x3b5f8` (`upload_palette`):** four 512-byte
  direct palette uploads into `0xffb800 + D1`, each sleeping 1 frame at
  `0x3b62e` (with `D1` and `A0` saved on the stack):
  1. `0x1ae68 -> 0xffb800` (frame 2)
  2. `0x1a268 -> 0xffba00` (frame 3)
  3. `0x1ba68 -> 0xffbc00` (frame 4)
  4. `0x1bc68 -> 0xffbe00` (frame 5)
- **`0x3c112–0x3c204` (`boot_setup_chunk`):**
  - clears object RAM (`0xffe000–0xffffff`, `0x800` longwords);
  - clears the text tilemap (`0x3bdb0`, `clear_text_layer`, factored out of
    its inline copy in `RoundEnd::ClearScreen`);
  - disables layers `0xe8` (`0x3b690`, `0xff8682 = 0`);
  - decodes `Io::dsw2` (default `0xffff`) into difficulty (`0xffc007 = 0`,
    `0xff8f28 = 0`), round-time setting (`0xff8e1c = 0`), continuation
    (`0xffc000 = 5` via `0x3c16a`), buy-in (`0xffc001 = 5` via `0x3c188`),
    and countries count (`0xffc005 = 0xff`), plus flip-screen from `dsw1`
    high byte bit 0 (`0x3c1ac–0x3c1bc`);
  - copies the 160-byte ranking table from `0x14598` to `0xff8e6e–0xff8f0d`
    (`0x28` longwords);
  - copies the 8-byte top score from `0xff8e73–0xff8e7a` to
    `0xffc012–0xffc019`;
  - copies the 48 words (96 bytes) of initial player structures from
    `0x3c296` to `0xff86b0–0xff870f`;
  - disables layers `0xe8` again (`0x3c1fa–0x3c1fe`) and runs `0x3bdb0`
    (`clear_text_layer`, `0x3c204`) a second time before sleeping 1 frame
    at `0x3c20a`.

### 4. Warning screen, slot 7 and slot 0 handoff (`0x3c210–0x3c290`, `0x3f4de`, `0x4813e`)

- **`0x3c210–0x3c236` (`Boot::Warning`, `warning_block_ptr`, `start_warning_block`, frame 8):**
  enables the text layer (`0x3b644` with `D0 = 0x08`, setting `0xff8682 = 0x08`)
  and starts typing the 4 warning blocks (`D2 = 3`, `index = 0..4`) from the
  pointer table at `0x3c300 + sext(byte_at(0x3cc9a))` (`[0x16a18, 0x16a26,
  0x16a5e, 0x16bca]` for the US ROM's `0x3cc9a = 0`) using `TextWriter`
  (`0x3bb98`, typewriter delay `D4 = 2`).
- **`0x3c230–0x3c252` (`Boot::WarningTyping` and `Boot::WarningPause`, frames
  8–553):** for each of the 4 warning blocks (`index = 0..4`), runs `0x3bb98`
  (`TextWriter`, finishing block 3's typing at frame `540` with `249` non-blank
  text cells in `videoram`) and then sleeps 12 frames at `0x3c238` (`jsr $3b53a.l`,
  `Boot::WarningPause`, finishing the fourth 12-frame pause at frame `553`).
- **`0x3c254` -> `0x3b57a` (`Boot::WarningHold`, frames 554–793):** reads the
  240-frame duration word (`0x00f0`) at `0x3b57c` and holds the completed
  warning screen over frames `554..=793` with only the text layer enabled
  (`gfxctrl = 0x08`).
- **`0x3c25a–0x3c290` (frame 794):**
  - `0x3c25a–0x3c26e`: loops `i = 0..4` over `warning_block_ptr` (`0x3c242–0x3c24e`)
    and erases all 4 warning blocks via `0x3bd00` (`erase_block`);
  - `0x3c270–0x3c274`: calls `0x3b654` with `D0 = 0x7070` (`coin_ctrl_or(bus, 0x7070)`),
    enabling coin control;
  - `0x3c27a–0x3c280` (`Scheduler::spawn` slot 1 at `0x3ccae`): spawns slot 1
    (`Task::Flow(Flow::AttractJoin)`);
  - `0x3c286–0x3c28c` (`Scheduler::spawn` slot 7 at `0x4813e`): spawns slot 7
    (`Task::Random(Random::Pass)` at `0x4813e`);
  - `0x3c290: jmp $3f4de.l`: slot 0 transitions from `Task::Boot` directly into
    the round-timer and score-tally task (`Task::Timer`), which sets its stack
    at `0x3f4de`, runs one pass of `0x3f4e2` (`round_timer`), and sleeps 1 frame
    at `0x3f7a6` (resume `0x3f7ac`).

### 5. Boot-to-attract join at frame 794 (`0x3ccae–0x3ccbc`, `0x3ccc2–0x3cce6`, `0x3f374–0x3f386`, `0x4813e`)

Because the scheduler (`0x3b19a`) rescans from slot 0 after every yield, all
three newly entered tasks run in frame `794` in slot order:

1. **Slot 0 (`0x3c25a–0x3c290` -> `0x3f4de–0x3f7ac`):** erases the 4 warning
   blocks, enables coin control `0x7070`, spawns slots 1 and 7, enters
   `0x3f4de` (`round_timer`, drawing the initial top-of-screen `1P`/`HI`/`2P`
   HUD scores), and yields at `0x3f7ac`.
2. **Slot 1 (`0x3ccae–0x3ccbc` and `0x3ccc2–0x3cce6`, `Flow::AttractJoin`):**
   - `0x3ccae–0x3ccbc`: sets slot 1's stack and spawns slot 3 (`0x3b20a` at
     `0x3f374`, `Task::Watch { d2: 0 }`);
   - `0x3ccc2–0x3cce6` (attract loop head): disables layers `0xe8` (`0x3b690`,
     setting `0xff8682 = 0x00`), clears the text layer (`0x3bdb0`), requests
     palette `0x1a268` at offset `0x200` (`0x3ccda`), and sleeps 1 frame
     (`0x3b51a` at `0x3cce6`) into `0x3ccec` (`Flow::AttractStart`).
3. **Slot 3 (`0x3f374–0x3f386` -> `0x3f388–0x3f3a2`, `Task::Watch`):** sets slot
   3's stack (`0x3f374`), initializes `0xff8f20 = 0x3f3be` and
   `0xff8f24 = 0x3f41e` (`0x3f378–0x3f386`), and runs the first pass of `watch`
   (`0x3f388–0x3f3a2`), which draws the copyright line (`0x14e9c`), jumps to
   `0x3f3be` to draw `INSERT COIN` (`0x1457a`), sets `D2 = 0x14` (`20`) and
   `0xff8f20 = 0x3f3d8`, and sleeps 1 frame at `0x3f39c` (resume `0x3f3a2`).
4. **Slot 7 (`0x4813e–0x4819e`, `Task::Random(Random::Pass)`):** sets slot 7's
   stack (`0x4813e`), runs the first pass of the 8-byte random-state update
   (`0x48142–0x48178`) from all zeros with an idle `IN1` low byte (`0xff`),
   producing `[0x03, 0x06, 0x0b, 0x12, 0x1b, 0x26, 0x33, 0x42]` at
   `0xff8e4a..0xff8e52`, checks the coin/service counters (`0x4817c–0x48196`),
   and sleeps 1 frame at `0x48198` (resume `0x4819e`).

At the end of frame `794`, the native state matches checkpoint `794` of
`sf-attract-01` (and `sf-boot-01`) across every owned work-RAM region, video
RAM, palette RAM, object RAM, and screen pixels, and continues directly through
the attract wall and title sequence (`795..=1647`, see
[attract.md](attract.md)) until slot 1 reaches the attract ranking screen at
`0x4268c` in frame `1648` (`C-STOP`).

**Reconciled frame-790 note (`docs/reference/first-frame.md`, `sf_fixed_fight_01.json`):**
Frame `790` lies inside `0x3c254`'s 240-frame warning hold (`554..=793`), where
only the text layer is enabled (`gfxctrl = 0x08`); the attract wall scroll does
not begin until frame `794`/`795`. Both `docs/reference/first-frame.md` and the
comment in `tools/reference/scenarios/sf_fixed_fight_01.json` are updated
accordingly.

### 6. The fixed fight from power-on, with no fixture, and IN1 folding into random state (`F49`, issue #89)

- **Zero-fixture fixed fight (`0..=5014`):** `PowerOnMachine` runs
  `sf-fixed-fight-01` from frame `0` (zeroed RAM, no checkpoint fixture):
  - `coin1` at `1450..1462` enters `0x48228` -> `0x3dec2` (`Flow::StartScreen`);
  - `start1` at `1520..1532` enters two-country select (`0x3e0f2..0x3e1e2`),
    which times out to Japan (`0xffc01c = 0x3e4c2`, Retsu first) at frame `1761`;
  - stage start (`0x3e59c`), stage-1 country intro (`0x3d3b6`), stage-1 fight
    intro (`0x3cd42`), and `ROUND 1` / `FIGHT` (`0x3f88e..0x3fc50`) bring the
    machine to checkpoint `2389` with
    `0xff8e4a..0xff8e51 = [0xa9, 0xeb, 0x5f, 0x66, 0x63, 0xfb, 0x45, 0xbe]`,
    matching `fight-run-a`'s checkpoint `2389` across every byte that
    `CpuMachine::from_checkpoint_2389` loads, and continuing through frame
    `5014` (`5,015` frames total from power-on) matching `fight-audit-run-a` on
    every single frame with `0` untranslated branches (`unrecovered().is_empty()`).
- **`IN1` folding in `Bus::step_random` (`0x4813e..0x481a2`):**
  Every 7 frames after slot 7 is spawned at frame `794`, `step_random` samples
  `0xff868e` (P2's active-low `IN1` byte) when `GAME_MODE != 2`
  (`0x4814a: cmpi.w #2, ($4002,A5); beq $48158; move.b ($68e,A5), D0`) or
  `0xff868f` (P1's active-low `IN1` byte) when `GAME_MODE == 2`
  (`0x48158: move.b ($68f,A5), D0`), bitwise-inverts `D0` (`not.b D0`), and
  folds `D0` into the 8-byte random state `0xff8e4a..0xff8e51`. In
  `sf-rng-inputs-01` (`0..=2910`), holding `p2_left` at `1200..1220` during the
  attract title (`GAME_MODE == 0`, folded at `1207` and `1214` via
  `0xff868e = 0xfd`) and `p1_right` at `2050..2070` during the stage-1 intro
  (`GAME_MODE == 2`, folded at `2053`, `2060`, and `2067` via `0xff868f = 0xfe`)
  shifts `0xff8e4a..0xff8e51` at frame `2389` to
  `[0xb0, 0x16, 0x18, 0xf6, 0xe7, 0x13, 0xf9, 0x7e]` and runs through Retsu's
  shifted AI decisions, round 1's KO at `2861`, and win award at `2897`.

## Summed 68000 instruction costs for the two overruns

In MAME 0.289 (`sf-mame0289.cpp:756, 770`), the 68000 runs at 8 MHz and the
screen refreshes at 60 Hz (`133,333⅓` cycles per frame). Both overruns in
`sf-boot-01` frames `0..=7` follow directly from the standard 68000
instruction costs:

### Overrun 1: Reset RAM clear (`0x3b000–0x3b01a`) across frame 0 into frame 1

`sf-mame0289.cpp:758` configures `screen_vblank` as `irq1_line_hold`. Because
reset starts with `SR = 0x2700`, frame 0's vblank interrupt is held pending
until `0x3b04a: move #$2000, SR`.

| Address | Instruction | Passes | Cycles |
| --- | --- | ---: | ---: |
| Reset vector | SSP + PC fetch | 1 | 40 |
| `0x3b000` | `movea.l #$c00000, A6` | 1 | 12 |
| `0x3b006` | `movea.l #$ff8000, A5` | 1 | 12 |
| `0x3b00c` | `movea.l #$ffe000, A4` | 1 | 12 |
| `0x3b012` | `movea.l A5, A0` | 1 | 4 |
| `0x3b014` | `move.w #$17ff, D0` | 1 | 8 |
| `0x3b018` | `clr.l (A0)+` (20) + `dbra D0, $3b018` (10 taken × 6,143 + 14 final) | 6,144 | 184,324 |
| **Total (`0x3b000–0x3b01a`)** | | | **184,412** |

- **At the end of frame 0 (`133,333⅓` cycles):** `88 + 4,441 × 30 = 133,318`
  cycles have elapsed (`4,441` longwords = `17,764` bytes cleared,
  `0xff8000..0xffc564`, `RESET_CLEAR_FRAME0_END = 0xffc564`), leaving
  `0xffc564..0xffe000` (`1,703` longwords) not yet reached. Because vblank 0
  is still held by `SR = 0x2700`, `0xff8000..0xff8005` are `0` and the sound
  latch is `0x00` at frame 0.
- **In frame 1 (`RESET_TO_VBLANK1_CYCLES = 186,275` cycles from `t = 0` to `0x3b052`):**
  the remaining `51,078⅔` cycles of the clear loop finish, followed by
  `0x3b01e–0x3b04a` and the held vblank's autovector entry into `0x3b052`:

  | Block (`0x3b000–0x3b052`) | Cycles |
  | --- | ---: |
  | Reset vector (`m_AfterReset = 2` -> `26` cycles in MAME's m68000 core) + `0x3b000–0x3b01a` (`48 + 184,324`) | 184,398 |
  | `0x3b01e–0x3b02a` (self-test DIP check) | 30 |
  | `0x3b030` -> `0x3b2e4` (`decode_coinage`, two slots) | 176 |
  | `0x3b036` -> `0x3b77a` (`boot_init`: `gfx_on`, `gfx_off`, `fill_words_16`, `queue_boot_sounds` × 3) | 1,114 |
  | `0x3b03c–0x3b044` -> `0x3b20a` (`Scheduler::spawn` slot 0) | 488 |
  | `0x3b04a` (`move #$2000, SR`: 12) + `0x3b04e` (`jmp $3b19a.l`: 12) + autovector IRQ1 entry (`42 + 3` E-clock alignment) | 69 |
  | **Total to `0x3b052` (`RESET_TO_VBLANK1_CYCLES`)** | **186,275** |
  | Vblank 1 handler to latch write (`0x3b052–0x3b136`) | 682 |
  | **Frame 1 latch write (`0x27`) from `t = 0`** | **186,957 (`0.023369625 s`, exact)** |

  `0x3b04a: move #$2000, SR` releases the held vblank (`0x3b052`), which pops
  `0x27` to the sound latch at `186,957` cycles (`0.023369625 s`), sets
  `0xff8004 = 1`, and returns to `0x3b19a`, where slot 0 wakes at `0x3c0c4`
  and sleeps 1 frame at `0x3c0c8`.

### Overrun 2: Boot set-up chunk (`0x3c112–0x3c204`) across frame 6 into frame 7

After the fourth palette upload sleeps 1 frame at `0x3b62e` in frame 5,
frame 6 runs:

| Address | Work in frame 6 | Passes | Cycles |
| --- | --- | ---: | ---: |
| `0x3b052–0x3b198` | Vblank 6 (idle, no palette request) + exception entry/exit | 1 | ~1,150 |
| `0x3b19a–0x3b1d8`, `0x3b634–0x3b642` | Scheduler wake of slot 0 + `0x3b5f8` wakeup (`movem.l`, `SYSTEM` bit 6 check, `rts`) | 1 | ~500 |
| `0x3c112–0x3c11a` | Object RAM clear: `movea.l` (4) + `move.w #$7ff, D0` (8) + 2,048 × (`clr.l (A0)+` 20 + `dbra` 10) + 4 | 2,048 | 61,456 |
| `0x3c11e` -> `0x3bdb0` | First text-layer clear: setup (28) + 2,048 × (`move.w D0, (A0)+` 8 + `dbra` 10) + `rts` (20) | 2,048 | 36,912 |
| `0x3c124–0x3c1bc` | `clr.w ($16,A6)`, `gfx_off(0xe8)`, `dsw2` / `dsw1` DIP decode | 1 | ~340 |
| `0x3c1c2–0x3c1d2` | Ranking table copy: `lea`, `lea`, `move.w #$27, D0` (24) + 40 × (`move.l (A1)+, (A0)+` 20 + `dbra` 10) + 4 | 40 | 1,228 |
| `0x3c1d6–0x3c1e4` | Top score copy: `lea`, `lea`, `move.w #7, D0` (24) + 8 × (`move.b (A1)+, (A0)+` 12 + `dbra` 10) + 4 | 8 | 204 |
| `0x3c1e8–0x3c1f6` | Player structures copy: `lea`, `lea`, `move.w #$2f, D0` (24) + 48 × (`move.w (A1)+, (A0)+` 12 + `dbra` 10) + 4 | 48 | 1,084 |
| `0x3c1fa–0x3c1fe` | `move.w #$e8, D0; jsr $3b690.l` (`gfx_off(0xe8)`) | 1 | 72 |
| **Subtotal before `0x3c204`** | | | **~102,946** |
| `0x3c204` -> `0x3bdb0` | Second text-layer clear: setup (28) + 2,048 × (`move.w D0, (A0)+` 8 + `dbra` 10) + `rts` (20) | 2,048 | 36,912 |
| **Total through `0x3c204`** | | | **~139,858** |

Because `~102,946 < 133,333⅓ < ~139,858`:

- **At the end of frame 6:** every memory write of `0x3c112–0x3c1fe` is
  already complete and the second `0x3bdb0` (`0x3c204`) is ~82% through
  refilling the text layer with the same `0x103c` words that `0x3c11e`
  already wrote. Slot 0 has not yet reached `0x3c20a`, so at frame 6's
  checkpoint its slot state is still `4` (`sleep = 0`) and `0xff8000` is
  `0x00` (cleared when the scheduler woke slot 0 at the start of frame 6).
- **In frame 7:** vblank 7 fires while slot 0 is in state `4` (so vblank 7
  does not decrement slot 0), sets `0xff8004 = 7` and `0xff8000 = 0x88`, and
  returns into `0x3c204`. Roughly `6,525` cycles later, `0x3c204` finishes
  and `0x3c20a` (`jsr $3b51a.l`) puts slot 0 to sleep for 1 frame
  (`state = 8, sleep = 1`), where `0x3b286` jumps to `0x3b19a`, clearing
  `0xff8000 = 0x00` and waiting for vblank 8.
- **In frame 8:** vblank 8 wakes slot 0 (`state = 2, sleep = 0`), and slot 0
  resumes at `0x3c210`, starting the warning screen.

## Verification (`crates/sf-engine/tests/power_on_boot.rs`, `crates/sf-audio/tests/both_chips.rs`)

- **`sf_boot_01_reference_runs_are_identical_and_equal_the_fixed_fight`
  (`C-REF`):** `boot-01-run-a` and `boot-01-run-b` match on all 4,003
  files, and all 830 `frames.tsv` rows match `fight-run-a`.
- **`mame_power_on_contents_at_frame_0_agree_with_the_zero_initial_state`
  (Acceptance 3 & 4):** verifies `ram_000000_work.bin` (both the cleared
  prefix `0xff8000..0xffc564`, the uncleared suffix `0xffc564..0xffe000`,
  and object RAM `0xffe000..0x1000000`), `ram_000000_video.bin`, and
  `ram_000000_palette.bin` against `PowerOnMachine::new`.
- **`power_on_matches_sf_boot_01_through_the_boot_setup_frames_and_stops_at_0x3c210`
  (`C-NAT` over `0..=7`):** compares the native capture over frames `0..=7`
  against `boot-01-run-a` via `sf_compare::compare`, verifies the latch
  sequence `0x00, 0x27, 0x3f, 0x80` at frames `0..=3`, and checks all owned
  work RAM.
- **`power_on_matches_sf_boot_01_and_sf_attract_01_through_1647_and_stops_at_0x4268c`
  (`C-NAT`, `C-STOP` over `0..=1647`, `F15`):** runs `PowerOnMachine` from
  frame `0` through `1647` (`WARNING_TITLE_LAST`) with no fixture, verifies
  `Outcome::Match` against `boot-01-run-a` (`830` frames `0..=829`, `800`
  checkpoints `0..=799`) and against `attract-01-run-a` (`1,648` frames
  `0..=1647`, `854` checkpoints `794..=1647`), checks every owned work-RAM
  region and non-stack/non-slot byte at all `1,654` checkpoints, asserts the
  frame-`794` random state `[0x03, 0x06, 0x0b, 0x12, 0x1b, 0x26, 0x33, 0x42]`
  at `0xff8e4a..0xff8e52`, and verifies that frame `1648` stops and reports
  `0x4268c`.
- **`negative_control_one_palette_upload_moved_a_frame_later_is_caught` &
  `negative_control_warning_hold_shortened_by_one_is_caught` (`C-NEG`):**
  delaying the first palette upload (`0x3c0d4`) by 1 frame is caught at frame
  `2` on `ram.palette`; shortening `0x3b57c`'s 240-frame warning hold to `239`
  is caught at frame `554` on slot 0's sleep byte (`0xff8007`) and at frame
  `793` by `sf_compare::compare` when the join to attract happens one frame
  early.
- **`power_on_warning_and_title_on_the_native_latch` (`both_chips.rs`, `F15`):**
  verifies `attract-01-hook-a` vs `b` determinism (`C-REF`), checks that frame
  `1`'s write is exact (`0.000` residual) and frames `2..=1647` lie inside
  `45..=138` cycles, and proves that over `0..=1647` with no pre-roll (`from: 1`)
  **every device is 100% identical (`0` differing samples, `first_diff_frame = None`)
  both at the reference's own write times and at constant latencies `45`, `51`,
  `92`, and `138`**: `73,246` samples-CPU writes (`0` off logged time),
  `219,837` MSM5205 DAC ticks (`0` active) and `21,104,642` 384 kHz stream
  samples (`0` differing), `42,759` music-CPU YM2151 writes in order within `2`
  samples (`42,759` exact at reference times, `42,704` at `51`, `42,711` at
  `92`), `1,536,211` YM2151 output samples (`0` differing), `1,211 / 1,211`
  idle frame-end music RAM dumps (`0..=793` in `boot-01-run-a`, `794..=1647` in
  `attract-01-run-a`) equal below the stack (`0xc000..0xc5cf`), and
  frame-by-frame streaming from power-on equal to the batch mix (`0 / 1,317,600`
  differing over `0..=1646`).
- **`fixed_fight_and_rng_inputs_from_power_on_with_no_fixture_match_reference`
  (`C-REF`, `C-NAT`, `C-NEG` over `0..=5014` and `0..=2910`, `F49`):**
  verifies `fixed-fight-power-on-01-run-a == b` (`11,953` files) and
  `rng-inputs-01-run-a == b` (`14,558` files); runs `PowerOnMachine` from frame
  `0` with zeroed RAM and no fixture through `5014` (`5,015` frames), verifying
  `Outcome::Match` against `fixed-fight-power-on-01-run-a` on `0..=2389` and
  `fight-audit-run-a` on `2389..=5014`, checking every owned work-RAM region on
  every frame, and asserting that frame `2389`'s state matches `fight-run-a` on
  every byte `CpuMachine::from_checkpoint_2389` loads (`0xff8e4a..0xff8e51 =
  [0xa9, 0xeb, 0x5f, 0x66, 0x63, 0xfb, 0x45, 0xbe]`); runs `sf-rng-inputs-01`
  from frame `0` through `2910`, verifying `Outcome::Match` and owned work RAM
  on all `2,911` frames and asserting the shifted random state
  `[0xb0, 0x16, 0x18, 0xf6, 0xe7, 0x13, 0xf9, 0x7e]` at frame `2389`, round 1
  KO at `2861`, and win award at `2897`; and proves the negative control (`C-NEG`)
  that moving `p2_left` by 7 frames (`1207` instead of `1200`, one `0x4813e`
  fold pass) is caught at frame `1201` on `0xff868e`, at frame `1207` on
  `0xff8e4a..0xff8e51`, and at frame `2389` on `0xff8e4a..0xff8e51`.
- **Launcher `power-on` mode (`crates/sf-dev/src/main.rs`):**
  `cargo run -p sf-dev --release -- power-on` starts `PowerOnMachine` at frame
  `0` with `GameSound::power_on` (empty `LatchTimeline`, no pre-roll) and wires
  both players' directions and buttons alongside the cabinet inputs, so inserting
  a coin (`5`), pressing Start (`1`), selecting a country, and fighting plays
  from frame `0` with no fixture; verified in `smoke` over `0..=2500`.

## Not covered

- **Attract demo fight (`0x3cd42` with `GAME_MODE == 0`):** `F48`.
- **`0x4834e` (self-test / service mode):** reached from `0x3b024` when
  `dsw1` bit 15 is low; reported, not translated (`F84`).
- **`0x3b602` (`0x3b5f8` palette upload repeat when `SYSTEM` bit 6 is
  low):** MAME defines bit 6 of `SYSTEM` (`0xc0000c`) as `IPT_UNKNOWN`
  (active low, so always high); reported if low, not translated.
- **`0x3b29a` (trap `#3` self-exit):** belongs to `F73`.

## Scope decision

Presented on 2026-10-28 under Acceptance 3 & 4: while MAME's power-on RAM
contents at frame 0 are all zero (`work`, `object`, `video`, `palette`), its
frame-0 `screen_update` (`pixels_000000.rgba`, scanned out as frame 1's
`screen_hash`) draws before `0x3b77e` writes `gfxctrl_w` in frame 1, so
`m_fg_tilemap` and `m_tx_tilemap` are still enabled from `video_start()`
(`sf-mame0289.cpp:266–274`) and `palette_device` holds its power-on 8-color
RGB cube pens (`emupal.cpp`). Modelling that pre-`gfxctrl_w` frame-0 screen
state in `power_on_frame0_scene` (`crates/sf-engine/src/run.rs`) and
documenting it here was approved so `sf_compare::compare` verifies every
frame `0..=1647` in full.
