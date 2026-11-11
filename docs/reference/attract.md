# Attract wall, title, INSERT COIN and ranking screen

This document records the boundaries established by
issue #43 (`F3` of the
arcade-flow batch) and issue #56
(`F16`): the attract wall, title blocks, logo hold, 300-frame title wait and
ranking handoff in slot 1 (`0x3ccec–0x3cdc4`, `0x3ce30–0x3ce4e`), the attract
ranking screen (`0x4268c–0x4274e`), and the copyright and `INSERT COIN` blink
in slot 3 (`0x3f3a4–0x3f40e`), verified from the attract-entry fixture at
checkpoint `794` through frame `1854` (`DF at 794, with no input`) and from
`sf-music-03`'s ranking-wait fixture at checkpoint `7185` through frame `7391`
(`DF at 7185, with no input`). Translated in `crates/sf-engine/src/tasks.rs`.

## Reference capture (`sf-attract-01`)

- **Scenario:** `tools/reference/scenarios/sf_attract_01.json` (`sf-attract-01`:
  power-on, no inputs, factory-default DIPs (`dsw1 = 0xdfff`, `dsw2 = 0xffff`),
  frames `0–1900` (`duration_frames = 1901`), full checkpoints on every frame
  `794–1900`).
- **Determinism (`C-REF`):** two independent MAME 0.289 runs
  (`attract-01-run-a`, `attract-01-run-b`, local-only) match byte-for-byte
  across all 5,538 files.
- **Prefix identity with `sf-fixed-fight-01`:** `frames.tsv` rows `0..=1449`
  (every frame before `coin1` is pressed at frame `1450` in
  `sf-fixed-fight-01`) and all five checkpoint `794` artifacts
  (`pixels_000794.rgba` and `ram_000794_{work,video,palette,music}.bin`) are
  byte-for-byte identical to `fight-run-a`'s.
- **Evidence label:** `DF at 794, with no input`.

## Checkpoint 794 is a consistent state

In frame `794`, the boot-to-attract join (`0x3ccae–0x3cce6`, `F15`) disables
layers `0xe8` (`0xff8682 = 0x00`), clears the text tilemap (`0x3bdb0`),
requests palette `0x1a268` at offset `0x200` (uploaded by vblank `795`), and
spawns slot 3 (`0x3f374`), which runs in frame `794` under the rescan
scheduler and writes the copyright block (`0x14e9c`) and `INSERT COIN`
(`0x1457a`) to video RAM while `gfxctrl = 0x00` keeps all layers disabled.
Rendering checkpoint `794` from its dumps with `active = work[0x682] = 0x00`
reproduces `pixels_000794.rgba` byte-for-byte (`86,016 / 86,016` pixels).

At the end of frame `794`, `Scheduler::from_fixture` decodes the four active
tasks:

| Slot | Task | Resume address | Extra state |
| ---: | --- | --- | --- |
| 0 | Round timer and score tallies (`0x3f4e2`) | `0x3f7ac` | — |
| 1 | Game flow / attract (`0x3ccec`) | `0x3ccec` | — |
| 3 | Attract texts / challenger watch (`0x3f388`) | `0x3f3a2` | `D2 = 0x0014` (`20`), `0xff8f20 = 0x3f3d8`, `0xff8f24 = 0x3f41e` |
| 7 | Random state and coins (`0x4813e`) | `0x4819e` | — |

## Exact attract timeline (frames 794–1855)

| Frames | Slot 1 resume | `gfxctrl` | `fg_scroll` | `bg_scroll` | Latch | Event |
| ---: | --- | --- | --- | --- | --- | --- |
| 794 | `0x3ccec` | `0x00` | `0x0000` | `0x0000` | `0xff` | Attract-entry fixture: layers off, palette `0x1a268 -> 0x200` requested, slot 3 writes copyright (`0x14e9c`) and `INSERT COIN` (`0x1457a`, `D2 = 20`) |
| 795–974 (180) | `0x3cd50` (`D1 = 0`) | `0x68` | `0x0000` | `0x0000` | `0xff` | Scroll entry 0 (`0x3cde0`): `tx + bg + fg` enabled, palette `0x1ae68 -> 0x000` requested at 795 (uploaded by vblank 796) |
| 975–979 (5) | `0x3cd50` (`D1 = 1`) | `0x68` | `0x0180` | `0x0000` | `0x1c` at 976 | Scroll entry 1 (`0x3cde8`): queues sound `0x1c` at 975 (popped by vblank at 976) |
| 980–984 (5) | `0x3cd50` (`D1 = 2`) | `0x68` | `0x0300` | `0x0000` | `0xff` | Scroll entry 2 (`0x3cdf0`) |
| 985–989 (5) | `0x3cd50` (`D1 = 3`) | `0x68` | `0x0480` | `0x0000` | `0xff` | Scroll entry 3 (`0x3cdf8`) |
| 990–994 (5) | `0x3cd50` (`D1 = 4`) | `0x68` | `0x0600` | `0x0000` | `0xff` | Scroll entry 4 (`0x3ce00`) |
| 995–999 (5) | `0x3cd50` (`D1 = 5`) | `0x68` | `0x0780` | `0x0000` | `0xff` | Scroll entry 5 (`0x3ce08`) |
| 1000–1059 (60) | `0x3cd50` (`D1 = 6`) | `0x68` | `0x0900` | `0x0000` | `0xff` | Scroll entry 6 (`0x3ce10`): fist breaks through wall |
| 1060–1099 (40) | `0x3cd50` (`D1 = 7`) | `0x68` | `0x0900` | `0x0180` | `0xff` | Scroll entry 7 (`0x3ce18`) |
| 1100–1139 (40) | `0x3cd50` (`D1 = 8`) | `0x68` | `0x0900` | `0x0300` | `0xff` | Scroll entry 8 (`0x3ce20`) |
| 1140–1229 (90) | `0x3cd86` (`D1 = 0`) | `0x68` | `0x0900` | `0x0480` | `0xff` | Scroll entry 9 (`0x3ce28`, `sleep = 0`) sets `bg_scroll = 0x0480` and immediately draws title block 0 (`0x1486a`) |
| 1230–1258 (29) | `0x3cd86` (`D1 = 1`) | `0x68` | `0x0900` | `0x0480` | `0xff` | Erases `0x1486a`, draws title block 1 (`0x14890`) |
| 1259–1287 (29) | `0x3cd86` (`D1 = 2`) | `0x68` | `0x0900` | `0x0480` | `0xff` | Erases `0x14890`, draws title block 2 (`0x14916`) |
| 1288–1347 (60) | `0x3cda8` | `0x68` | `0x0900` | `0x0480` | `0xff` | Erases `0x14916`, draws title logo (`0x14afc`) over wall (`0x3cd96–0x3cda2`) |
| 1348–1587 (240) | `0x3b588` (`caller = 0x3cdb8`) | `0x08` | `0x0900` | `0x0480` | `0xff` | `gfx_off(0x60)` leaves `tx` only; first 240 frames of nested sleep `0x3b582` (`jsr $3b57a.l`) |
| 1588–1647 (60) | `0x3cdb8` | `0x08` | `0x0900` | `0x0480` | `0xff` | Second 60 frames of `0x3b582` (`0x3b588: bra $3b562`) |
| 1648 | `0x42736` (`caller = 0x3ce54`) | `0x28` | `0x0900` | `0x7580` | `0xff` | `0x3cdb8–0x3cdc4`, `0x3ce30–0x3ce4e`, `0x4268c–0x42730`: erases `0x14afc`, enables `bg` (`0x20`), requests palette `0x1b868 -> 0x000`, sets `bg_scroll = 0x7580`, clears text layer (`0x3bdb0`), writes header (`0x14682`), queues `0x39`, draws 10 ranking entries (`0xff8e6e`), arms timer 3 (`0xff8e16 = 0x08`) and sleeps 1 frame into `0x42736` |
| 1649–1854 (206) | `0x42736` (`caller = 0x3ce54`) | `0x28` | `0x0900` | `0x7580` | `0x39` at 1649 | Attract ranking wait (`0x42736–0x4273c`, `F16`): polls `0xff8e2f` while timer 3 (`0x3f674–0x3f700`, `C = 2`, `N = 9`) counts 206 frames |
| 1855 | `0x3ce54` (`F48`) | `0x28` | `0x0900` | `0x7580` | `0xff` (`0x3f` queued) | `0xff8e2f = 1` in frame 1855: `0x4273e–0x4274e` queues `0x3f`, clears the text layer (`0x3bdb0`), and returns via `rts` to `0x3ce54` (`C-STOP` in `F16`) |

Throughout attract mode (`0xff8003 == 0`), slot 3 (`0x3f3a4–0x3f40e`) redraws
the copyright block (`0x14e9c`) every frame, resets `0xff8f24 = 0x3f41e`, and
toggles `INSERT COIN` (`0x1457a`) 20 frames on (`0xff8f20 = 0x3f3d8`, frames
`794–813`, `834–853`, …) and 20 frames off (`0xff8f20 = 0x3f402`, frames
`814–833`, `854–873`, …) by counting `D2` down from `0x14` (`20`) to `0`.

## Attract ranking screen (`0x4268c–0x4274e`, issue #56)

Entered by `jsr $4268c.l` at `0x3ce4e` (return address `0x3ce54` on slot 1's
stack at `usp + 68` while sleeping inside `0x3b51a`):

- **`0x4268c–0x426b6`:** zeroes `bg_scroll` (`0xffc682 = 0`), adds `0x7580` via
  `0x3b682`, clears the text tilemap (`0x3bdb0`), writes the ranking header
  text block `0x14682` (`0x3bba0`), and queues tune `0x39` (`0x3b7c8`).
- **`0x426bc–0x42700`:** iterates `D2 = 0..10` (`D3 = 9..=0`, `D4 = D2 << 4`)
  over the ten 16-byte ranking entries at `0xff8e6e..0xff8f0e`:
  - `0x3bdf2` (`draw_digits`) with `D1 = 7` (8 digits), attribute `0x9000`,
    source `entry + 5` (`8` BCD score digits), destination
    `0x800000 + sign_extend(rw(entry))`.
  - `0x3bdf2` (`draw_digits`) with `D1 = 2` (3 initials), attribute `0x9000`,
    source `entry + 13` (`3` name initials), destination
    `0x800000 + sign_extend(rw(entry + 2))`.
- **`0x42704–0x42726`:** scans the 10 entries for a marker byte `0xff` at
  `entry + 4` (`cmpi.b #-$1, ($4,A3,D1.w)` with `A3 = 0xff8e6e` and `D1 = D2 << 4`).
  In attract mode no entry is marked, so the loop falls through to `0x4272a`;
  if an entry has `entry + 4 == 0xff`, it branches to `0x42750` (translated in
  issue #122 (`F50`); see
  [versus-carry.md](versus-carry.md)).
- **`0x4272a–0x4274e`:** sets `0xff8e16 = 0x08` (arming timer counter 3 at
  `0x3f674–0x3f700` on the next frame with `0xff8e2a = 0x0003_0000`, `0xff8e2e =
  0x02` from ROM byte `0x3f6fb` in `0x3f6f8: move.b #$02, ($e2e,A5)`, and
  `0xff8e2f = 0x00`, reloading `0xff8e2c = 0x0909` on each borrow) and sleeps 1
  frame (`0x42730 -> 0x3b51a`) into `0x42736`. Each frame at `0x42736`, while
  `0xff8e2f == 0`, it sleeps 1 frame (`0x4273c -> 0x42730`). When `0xff8e2f != 0`
  (frame `1855` in `sf-attract-01`, frame `7392` in `sf-music-03`),
  `0x4273e–0x4274e` queues `0x3f`, clears the text tilemap (`0x3bdb0`), and
  returns via `rts` to `0x3ce54`.
- **Second window (`sf-music-03`, checkpoints `7185..=7392`):** in
  `sf-music-03`, after the continue countdown expires and the game-over screen
  (`6240–6328`) returns to the attract loop (`6329`), the attract wall and
  title run over `6329–7184` and enter `0x4268c` at frame `7185` (`0x39`
  latched at `7186`, natural tune end via sequencer opcode `0xf` at `7301`,
  `0xff8e2f = 1` at `7392`). Thus `0x39` in `music-stops.md` Window D is the
  attract ranking screen's tune, not a game-over tune.

## Native verification (`C-NAT`, `C-STOP`, `C-NEG`)

Verified in `crates/sf-engine/tests/task_scheduler.rs` and
`crates/sf-engine/tests/attract_from_794.rs`:

1. **Fixture decoder:** `Scheduler::from_fixture` decodes checkpoint `794` and
   every checkpoint `794..=1854` of `sf-attract-01` and `7185..=7391` of
   `sf-music-03`, including `0x3cd50` (`D1` in `0..10`), `0x3cd86` (`D1` in
   `0..3`), `0x3cda8`, `0x3b588` (verifying caller `0x3cdb8` at `usp + 68`),
   `0x3cdb8`, `0x42736` (verifying caller `0x3ce54` at `usp + 68`), and slot
   3's attract continuation at `0xff8f20` (`0x3f3be`, `0x3f3d8`, `0x3f402`),
   and refuses any out-of-range `D1`, wrong caller, or untranslated
   continuation.
2. **Full checkpoint-window comparisons (`795..=1854` in `sf-attract-01` and
   `7186..=7391` in `sf-music-03`):** native execution from checkpoint `794`
   matches `sf-attract-01` on all 1,060 frames (`795..=1854`, with `0x1c` at
   `976` and `0x39` at `1649`), and from checkpoint `7185` matches
   `sf-music-03` on all 206 frames (`7186..=7391`, with `0x39` at `7186`),
   across `screen_hash`, 384×224 checkpoint pixels, video RAM, palette RAM,
   sound latch, `CPU_RAM_REGIONS` + slot state/sleep bytes, and every work-RAM
   byte outside the unowned stack/slot-SR-PC-USP exception.
3. **Stop boundaries (`1855` and `7392`, plus `0x42750`):** at frame `1855` in
   `sf-attract-01` and frame `7392` in `sf-music-03`, after `0x4273e–0x4274e`
   queues `0x3f` and clears the text layer, slot 1 enters `0x3ce54`.
   When a ranking entry has byte `+4 == 0xff`, entering `0x4268c` at frame
   `1648` takes `0x42750..0x42756` into `0x4275c` (issue #122, `F50`).
4. **Demo Sounds and Attract Music DIPs over the attract (`sf-attract-demosound-off-01` and `sf-attract-music-off-01`, issue #77, `F37`):**
   - `sf-attract-demosound-off-01` (`Demo Sounds = Off`, `dsw1 = 0xffff`,
     `dsw2 = 0xffff`, frames `0..=1900`, checkpoints `794..=1900`): two MAME
     0.289 runs (`attract-demosound-off-01-run-{a,b}`) match byte-for-byte
     across all 5,538 observation files (`C-REF`). Over `795..=1854` from
     checkpoint `794`, `0x3b7e4..0x3b800` (`MODE_DEMO == 0` and `DSW1` bit 13
     high) filters out both `0x1c` at `975` and `0x39` at `1648`, so no sound
     command is latched (`latches == []`), while screen hashes, pixels, video
     RAM, palette RAM, and work RAM match the reference on every frame (`C-NAT`
     through `1900`).
   - `sf-attract-music-off-01` (`Attract Music = Off`, `dsw1 = 0xddff`,
     `dsw2 = 0xffff`, frames `0..=1900`, checkpoints `794..=1900`): two MAME
     0.289 runs (`attract-music-off-01-run-{a,b}`) match byte-for-byte across
     all 5,538 observation files (`C-REF`). Over `795..=1854` from checkpoint
     `794`, `0x3b7c8..0x3b7e2` (`DSW1` bit 9 low and `0x28 <= cmd <= 0x3e`)
     leaves `0x1c` latched at frame `976` while filtering out `0x39` at frame
     `1648` (`latches == [(976, 0x1c)]`), and screen hashes, pixels, video RAM,
     palette RAM, and work RAM match the reference on every frame (`C-NAT`
     through `1900`).
   - **Power-on attract/demo sound DIPs and unfiltered coin sound (`sf-attract-music-off-02`, `sf-demosound-off-02`, `sf-demosound-coin-01`, issue #140, `F68`):**
     - `sf-attract-music-off-02` (`Attract Music = Off`, `dsw1 = 0xddff`, frames `0..=5145`, checkpoints `0..=1900`, `3630..=3645`, `4485..=4500`, `5130..=5140`, `1,944` checkpoints, `9,723` files) and `sf-demosound-off-02` (`Demo Sounds = Off`, `dsw1 = 0xffff`, same `5,146` frames and `1,944` checkpoints): verified from power-on (`0..=5140`) through the demo's round end (`3636`) and the second ranking screen exit (`5137`). In `sf-demosound-off-02`, only the boot commands (`0x27` at frame `1`, `0x3f` at frame `2`, `0x80` at frame `3` via `0x3b802`, plus `0xff` at frame `4`) change the latch; over `5..=5145` while `0xff8003 == 0`, zero commands pass through `0x3b7c8`.
     - `sf-demosound-coin-01` (`Demo Sounds = Off`, `dsw1 = 0xffff`, `coin1@1450` held `12` frames, frames `0..=1520`, checkpoints `0..=830` and `1440..=1520`, `912` checkpoints, `4,563` files): verified from power-on (`0..=1520`). At frame `1467`, while `0xff8003` is still `0` (before `0x48228` sets `0xff8003 = 0xff`), `0x482da` calls the unfiltered entry `0x3b802` (`queue_sound_unfiltered(0x0f)`), so `0x0f` is latched at frame `1468`, followed by `0x3f` (`1469`) and `0x80` (`1470`) once `0xff8003 = 0xff`. Routing `0x0f` through the filtered entry `0x3b7c8` drops `0x0f` while `0xff8003 == 0` and fails comparison at frame `1468` (`C-NEG`).
5. **Attract demo setup through `FIGHT!`, round end (`0x3ce8c..0x3ceba`), and the attract loop from power-on (issue #88, `F48`; issue #127, `F55`):**
   - Translates `0x3ce54..0x3ce86` (`gfx_off(0x28)`, saving `0xffc007` to
     `0xff8f28`, clearing `0xff8f2a`, loading `0xffc007 = [0x3cebe] = 0`,
     clearing `0xffc00b`, `0xffc006`, `0xffc003`, `0xffc004`, setting
     `GAME_MODE = 2`, and jumping to `0x3e04c`), the demo stage/opponent
     palette branches `0x3e5c6..0x3e5d0` and `0x3e6fe..0x3e70e` (entering
     `0x3e714` without events 4 and 5), the demo input reader
     `0x4030c..0x40334` (`1,365` 3-byte records at `0x3cebf`, wrapping at
     `[0x3cebc] = 0x0fff`), and the demo round-end return `0x3ce8c..0x3ceb8`
     (`attract_demo_end`, called from `0x3ec02`, `0x3edaa`, and `0x3ecf6` when
     `MODE_DEMO == 0`: queues `0x3f`, restores `0xffc007` from `0xff8f28`,
     clears object regions `0x3b9bc / 0x3b9c8 / 0x3b9d4`, kills slot 2 at
     `0x3ceb2 -> 0x3b230`, and loops back to `0x3ccc2` `attract_loop_head`).
   - `sf-attract-demo-01` (`6,001` frames `0..=6000`, checkpoints at `794` and
     every frame `1850..=4750` through the second ranking's exit at `4697`):
     two MAME 0.289 runs (`attract-demo-01-run-{a,b}`) match byte-for-byte
     across all `14,513` observation files (`C-REF`), and rows `0..=1900` equal
     `sf-attract-01`'s.
   - From checkpoint `794` through `4750` (`3,956` frames and `2,901` full
     checkpoints `1850..=4750`) and from power-on (`frame 0`) with no fixture
     through `6000` (across `sf-boot-01` `0..=829`, `sf-attract-01` `794..=1900`,
     and `sf-attract-demo-01` `0..=6000`), native execution matches the
     reference on every frame (`screen_hash`, `latch`, `latch_changed`,
     checkpoint pixels, video RAM, palette RAM, owned + non-stack work RAM,
     `0xff8f2a`, P1's input byte `0xff868f` and latches `0xff86b0..0xff86b1`,
     and the random state `0xff8e4a..0xff8e51`), with slot 1 at `0x3e59c` and
     slot 2 at `0x3f7d6` on `1855`, `0x3f` latched at `1856`, `0x28` at `1859`,
     `0x94` (`FIGHT!`) at `1940`, the `1,365`-record table reaching `0x0ffc`
     and wrapping at `0x0fff` to `0` (ending at `0x015c`), KO body thud `0x09`
     at `3635`, `0x3ce8c..0x3ceb8` at `3636` killing slot 2 and entering
     `0x3ccec` (`0x3f` latched at `3637`), the second wall (`0x1c` at `3818`),
     title and ranking (`0x39` at `4491`), and the second demo setup at `4697`
     (`0x3f` at `4698`, `0x28` at `4701`) with `0` untranslated branches.
6. **Negative controls (`C-NEG`):**
   - Changing slot 3's `D2` by 1 (`0x13` instead of `0x14` in the `794`
     fixture) is caught at the first `INSERT COIN` toggle (frame `813` instead
     of `814`).
   - Shortening scroll entry 0's sleep by 1 frame (`0xb3` instead of `0xb4` at
     `0x3cde4`) is caught at frame `795` on slot 1's sleep byte (`0xff8017`)
     and at frame `974` by `sf_compare::compare` when scroll entry 1 runs one
     frame early.
   - Shortening timer 3's initial inner counter by 1 frame (`0x01` instead of
     `0x02` at `0x3f6fb`) exits the ranking screen one frame early and is
     caught by `sf_compare::compare` at frame `1854`.
   - Starting the demo input index (`0xff8f2a`) at record 1 (byte offset `3`)
     instead of record 0 (`0`) is caught on `0xff8f2a` at `1855`, on P1's input
     byte `0xff868f` (`0xfe` vs `0xff`) and `0xff8f2a` (`6` vs `3`) at `1858`,
     and shifts P1's forward-jump input (`0xf6`) one frame early to `1971`.
   - Delaying the demo round end by 1 frame (`3636 -> 3637`, by setting slot 1's
     sleep timer `0xff8017` at `3635` to `2` instead of `1`) is caught at frame
     `3636` on work RAM (`0xff8026` slot 2 still active), video RAM, and
     `sf_compare::compare`.

## Not covered

- Coin arrival during the attract demo killing an occupied slot 2 (`F66`; coin
  arrival during the attract title opening the start screen is now covered by
  issue #73 (`F33`), see
  [start-screen.md](start-screen.md)).

(Note: the warning screen `0x3c210–0x3c290`, the boot-to-attract join
`0x3ccae–0x3cce6`, and slot 3's entry `0x3f374–0x3f386` leading into frame
`794` and continuing through `1647` from power-on are now covered by
issue #55 (`F15`); see
[power-on.md](power-on.md).)
