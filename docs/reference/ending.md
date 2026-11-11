# The ending (`0x44a40`, ticket #136 / F64)

After the player wins the second fight in stage 4 (`0xffc003 == 4`, Sagat) and
the post-match column wipe (event `6`) completes, `0x3ef72` increments
`0xffc004` to `2` and enters `0x3ef96` (`crates/sf-engine/src/tasks.rs`). `0x3ef96`
clears `0xffc004`, `0xffc00a`, and `0xffc009`, indexes the 5-long stage table
`0x3efb0` by `STAGE_NO == 4` (`0x3efc0`), and jumps to `0x44a40` (the ending).

## Code structure (`0x44a40..0x44c1c`, 110 instructions)

1. **Entry (`0x44a40..0x44a5e` -> resume `0x44a60`):**
   - Queues ending tune `0x32` (`0x3b7c8`), disables layers `0xe8` (`0x3b690`),
     clears the text layer (`0x3bdb0`), and sleeps 1 frame (`0x3b51a` at
     `0x44a5a` into `0x44a60`, the ending fixture in `sf-win-10` at frame
     `39776`).

2. **Five country-ordered scenes (`0x44a60..0x44aff`):**
   - Turns on `bg + tx` (`0x28` via `0x3b644`) and runs 5 outer iterations
     (`D2 = 4..=0`, `D1 = 0..=4`, `D3 = D1 << 2`):
   - `0x44a72..0x44a7e`: loads the starting country's 5-scene table pointer from
     `0x44c1c + 4 * 0xffc006` (`0x44c2c` Japan, `0x44c40` USA, `0x44c54` China,
     `0x44c68` England). Each scene entry is 4 bytes `(pal_idx, step0, step1, 0)`.
   - `0x44a84..0x44aa2`: requests palette `*(0x44c7c + 4 * (entry[0] & 0x0f))` at
     offset `0` (`0xff8633 = 0xff`).
   - `0x44aa4..0x44af8`: runs two inner sub-steps (`D4 = 1` using `entry[1]`,
     then `D4 = 0` using `entry[2]` at `0x44ab0`):
     - Clears `0xffc682` (`0x44ab4`) and adds `*(0x44c90 + 2 * (byte >> 4))` to
       `bg_scroll` (`0x3b682`).
     - Draws the 2x2-tile country card `*(0x44ca4 + 4 * (byte & 0x0f))` via
       `0x3bc3e` (`text_block_2x2`), sleeps 120 frames (`0x3b56a` at `0x44ae2`
       into `0x44ae8`), and erases the 2x2-tile card via `0x3bd3e`
       (`erase_block_2x2` at `0x44af0`).
   - For the Japan start (`0xffc006 == 0`, table `0x44c2c`), the 5 scenes and 10
     sub-steps (`39777..=40977`) are:

| Scene (`D1`) | Sub-step (`D4`) | Frame | Palette (`0x44c7c`) | `bg_scroll` (`0x44c90`) | 2x2 text block (`0x44ca4`) |
| --- | --- | --- | --- | --- | --- |
| 0 (Japan) | 1 | 39777 | `0x1b068` | `0x1b80` | `0x44ccc` (`RETSU`) |
| 0 (Japan) | 0 | 39897 | `0x1b068` | `0x2500` | `0x44cdc` (`GEKI`) |
| 1 (USA) | 1 | 40017 | `0x1b268` | `0x2e80` | `0x44cea` (`JOE`) |
| 1 (USA) | 0 | 40137 | `0x1b268` | `0x3800` | `0x44cf6` (`MIKE`) |
| 2 (China) | 1 | 40257 | `0x1b468` | `0x4180` | `0x44d04` (`LEE`) |
| 2 (China) | 0 | 40377 | `0x1b468` | `0x4b00` | `0x44d10` (`GEN`) |
| 3 (England) | 1 | 40497 | `0x1b668` | `0x5480` | `0x44d1c` (`BIRDIE`) |
| 3 (England) | 0 | 40617 | `0x1b668` | `0x5e00` | `0x44d2e` (`EAGLE`) |
| 4 (Thailand) | 1 | 40737 | `0x1b868` | `0x6780` | `0x44d3e` (`ADON`) |
| 4 (Thailand) | 0 | 40857 | `0x1b868` | `0x7100` | `0x44d4c` (`SAGAT`) |

3. **Congratulatory message (`0x44b00..0x44b86`):**
   - `0x44b00..0x44b1e` (frame `40977`): queues `0xff` (`0x3b7c8`), disables
     layers `0xe8` (`0x3b690`), clears the text layer (`0x3bdb0`), and sleeps 1
     frame (`0x3b51a` at `0x44b1a` into `0x44b20`).
   - `0x44b20..0x44b70` (frame `40978`): turns on `bg + tx` (`0x28`), requests
     palette `0x1b868` at offset `0`, selects `bg_scroll` from `0x44d5c` (`0x7700`
     when `0xffc002 == 2` at `0x44b48: cmpi.b #2, ($4002,A5); beq $44b54`, else
     `0x44d5e = 0x7880` at `0x44b52`), sets `0xffc682 = 0` and adds `bg_scroll`
     (`0x3b682`), sets `TEXT_SOUND` (`0xff868c = 0x000d`), and calls `0x3bb98`
     (`TextWriter`, `D4 = 5`) on block `0x44d60` (`13 x 33`, 169 non-fill
     characters, 845 frames across `40978..=41822`, queuing `0x0d` on each typed
     character and clearing `0xff868c = 0` at `41823`).
   - `0x44b72..0x44b86`: holds 120 frames (`0x3b56a` at `0x44b72` into `0x44b78`,
     frames `41823..=41943`), erases `0x44d60` (`0x3bd00` at `41943`), and sleeps
     1 frame (`0x3b51a` at `0x44b82` into `0x44b88`).

4. **Staff roll and closing (`0x44b88..0x44c1b`):**
   - `0x44b88..0x44ba6` (frame `41944`): queues staff tune `0x36` (`0x3b7c8`),
     disables layers `0xe8`, clears the text layer, and sleeps 1 frame (`0x3b51a`
     into `0x44ba8`).
   - `0x44ba8..0x44bc6` (frame `41945`): turns on `tx` (`0x08`), draws the 2x2
     staff header `*(0x44f58)` (`0x44f80`, `STREET FIGHTER STAFF`) via `0x3bc3e`,
     and sleeps 2 frames (`0x3b522` into `0x44bc8`).
   - `0x44bc8..0x44be4` (frames `41947..=43497`): iterates `D1 = 7..=0` over the
     8 staff block pointers at `0x44f5c..0x44f7b` (`0x44f90`, `0x44fce`,
     `0x4500a`, `0x45042`, `0x45120`, `0x4516e`, `0x451ae`, `0x45208`), typing
     each block through `0x3bb98` with `D4 = 2` (`TEXT_SOUND == 0`), holding 120
     frames (`0x3b56a` at `0x44bd4` into `0x44bda`), and erasing it (`0x3bd00`).
   - `0x44be6..0x44c1a` (frames `43497..=43617`): erases the 2x2 header
     `*(0x44f58)` (`0x3bd3e`), draws the 2x2 closing block `*(0x44f7c)`
     (`0x452cc`, `THE END`) via `0x3bc3e`, holds 120 frames (`0x3b56a` at
     `0x44bfe` into `0x44c04`), erases `*(0x44f7c)` (`0x3bd3e` at `43617`), calls
     `0x3b7a8` (queuing `0x27`, `0x3f`, `0x80` via `0x3b802`), and jumps to
     `0x3efc4` (`0x44c16`).
   - At `43617`, `0x3efc4` increments `0xffc003` (`STAGE_NO: 4 -> 5`) and branches
     on `STAGE_NO == 5` to `0x3f2d0` (`versus_game_over`, F25 / #65), which calls
     `0x42422` (`ranking_check`) for P1 (`0xff86b0`, `SCENE_MODE = 2`), entering
     `0x42486` (`name_entry_start`, F42 / #82, sleeping into `0x42552` at `43617`
     and `0x42584` at `43618`).

## Verification (`crates/sf-engine/tests/ending.rs`)

- **Reference repeat (`C-REF`):** `win-10-run-a` and `win-10-run-b` match
  byte-for-byte over all `3,842` frames and checkpoints (`39776..=43618`).
- **Native comparison (`C-NAT`):** `run_fight_capture` and
  `compare_fight_ram_with_inputs` from the ending fixture `39776` (`0x44a60`)
  through `43618` (`3,842` frames) — and continuously from Sagat's round-1
  fixture `37153` through `43618` (`6,465` frames) — match every captured
  `screen_sha256`, `sound_latch`, `latch_cycles`, checkpoint `pixels_*.rgba`,
  and owned RAM byte, then continue natively through `0x3f2d0`'s name-entry
  timeout, marked ranking screen (`0x4268c` / `0x4275c`), `0x3f2e0` / `0x3f2f0`
  game-over screen, and `0x3f33c` return to attract (`0x3ccec`) with zero
  untranslated branches.
- **Negative controls (`C-NEG`):**
  1. Tampering the checkpoint pixel CRC at frame `41823` (end of `0x44d60`
     typewriter) fails `compare` at frame `41823`.
  2. Patching stage 4's table entry at `0x3efc0` (`0x44a40` -> `0x492d6`) stops
     native execution from `39775` at frame `39776` with `unrecovered = [0x492d6]`.
  3. Tampering `0x44d5c` (`0x7700` -> `0x7880`) fails owned-RAM comparison at
     frame `40978` (`0xffc682`).

## Complete one-player game from power-on (`sf-full-1p-01`, ticket #137 / F65)

`tools/reference/scenarios/sf_full_1p_01.json` (`sf-full-1p-01`, recorded as
`full-1p-01-run-a` and `full-1p-01-run-b`, `44,200` frames, `873` checkpoints across
24 milestone windows per open decision 7, factory-default DIPs `DSW1 = 0xdfff`,
`DSW2 = 0xffff`, evidence class **PO**) extends `sf-win-10` from frame `43618`
through the return to the attract loop (`44199`):

1. **Prefix equality through `43618`:** `sf-full-1p-01`'s first `2,077` inputs equal
   `sf-win-10`'s inputs, `frames.tsv` rows `0..=43618` match `win-10-run-a`
   byte-for-byte, and all `55` shared checkpoints in `37153..=43618` match
   `win-10-run-a` byte-for-byte across all 6 checkpoint files.
2. **Post-ending name entry (`0x3efc4 -> 0x3f2d0 -> 0x42422 -> 0x42486`,
   `43617..=43693`):**
   - At `43617`, `0x44c04` erases `THE END`, calls `0x3b7a8`, and jumps to `0x3efc4`
     (`STAGE_NO: 4 -> 5`), which jumps to `0x3f2d0` (`versus_game_over`,
     `SCENE_MODE = 2`). `0x42422` (`ranking_check`) finds P1's final score
     (`494,300` pts) beats rank 1 (`0xff8e6e`, boot `101,800` pts), sets
     `0xff8e66 = 0xff8e6e`, and enters `0x42486` (`name_entry_start`, sleeping into
     `0x42552` at `43617` and arming timer 2 `0xff8e16 = 0x04` at `0x42552` into
     `0x42584` at `43618`).
   - Three `p1_lp` presses (`hold_frames = 4`) at frames `43630`, `43655`, and
     `43680` commit `'A'` (`0x0a`) at `43637` (sound `0x1b` latched at `43638`),
     `43661` (sound `0x1b` latched at `43662`), and `43685` (sound `0x1b` latched at
     `43686`, cursor deactivating at `43693` via `0x427e2 -> 0x42920`).
3. **Ranking insertion and marked-entry blink (`0x425c4 -> 0x4268c -> 0x42750`,
   `43694..=43901`):**
   - At `43694`, `0x425c4` shifts ranks `0..8 -> 1..9`, writes rank 1 (`0xff8e6e`)
     with marker `0xff` (`0xff8e72 = 0xff`), score `0494300`, and initials `AAA`
     (`0x0a, 0x0a, 0x0a`), clears P1's score (`0xff86d2..0xff86da = 0`), queues
     `0x3f` (latched at `43695`) and `0x39` (latched at `43696`), and enters
     `0x4268c` (`ranking_screen_start`).
   - `0x42750..0x427e0` blinks the marked rank-1 entry for 3 seconds (`43695..=43900`).
     At `43901`, `0x42792` clears `0xff8e72 = 0`, queues `0x3f` (latched at `43902`),
     and returns via `RankingReturn::GameOverP1` to `0x3f2e0` (`versus_game_over_p2`,
     `SCENE_MODE = 1`).
4. **Game-over screen and return to attract (`0x3f2e0 -> 0x3f2f0 -> 0x3f33c ->
   0x3ccc2`, `43901..=44199`):**
   - At `43901`, `0x42422` sees P2 (`0xff86da`) has `0` pts (`0xff8e66 = 0`) and
     enters `0x3f2f0` (`game_over_screen`), scrolling fg to `0x0c00`, requesting
     palette `0x1a268` at offset `0x200`, queuing voice `0x95` (latched at `43903`),
     and holding 240 frames (`43902..=44141`).
   - At `44142`, `0x3f33c` (`game_over_reset`) clears `MODE_DEMO`, `GAME_MODE`,
     `STAGE_NO`, `FIGHT_NO`, `SCENE_MODE`, `WINNER`, and `ROUND_NO` to `0`, kills
     slot 2 (`0xff8026 = 0`), spawns slot 3 at `0x3f374` (`0xff8036 = 8`), and
     jumps to `0x3ccc2` (`attract_loop_head`, entering `0x3ccec` at `44143..=44199`).

### Verification (`crates/sf-engine/tests/full_1p.rs`)

- **Reference repeat (`C-REF`):** `full-1p-01-run-a` and `full-1p-01-run-b` match
  byte-for-byte across all `5,241` observation files (`873` checkpoints $\times$ 6
  files + 3 TSVs) and `44,200` frames (`0..=44199`).
- **Native comparison (`C-NAT`):** `PowerOnMachine` (`observe_power_on_scenario` and
  `run_power_on_scenario_capture_for_reference`) from frame `0` (no fixture,
  including the 8-byte PRNG state `0xff8e4a..=0xff8e51` from power-on) through
  `44199` matches every captured `screen_sha256`, `sound_latch`, `latch_cycles`,
  checkpoint `pixels_*.rgba`, checkpoint `videoram` / `paletteram`, and owned RAM
  byte across all `44,200` frames and `873` checkpoints with zero untranslated
  branches reached (`unrecovered == []`).
- **Negative control (`C-NEG`):** shifting `start1` by 1 frame (`1520 -> 1519`) is
  caught at its first differing frame (`1520`) in both owned work RAM and
  `compare` (`checkpoint videoram` / `checkpoint screen pixels`).

## Unexercised branches and uncovered variants

- **Game-mode test at `0x44b48` (`cmpi.b #2, ($4002,A5); beq $44b54`):** Ryu's
  one-player chain runs in mode 2 (`0xffc002 == 2`), taking `beq $44b54` (`D0 =
  0x7700` from `0x44d5c`). The fall-through branch `0x44b52: move.w (A0)+, D0`
  (`D0 = 0x7880` from `0x44d5e`, Ken's ending scroll in mode 4, open decision 10)
  is translated, labelled unexercised in arcade play, and unit-verified in
  `crates/sf-engine/tests/ending.rs`.
- **Starting-country variants at `0x44a76` (`0x44c1c` table):** only the
  Japan-start variant (`0xffc006 == 0`, `0x44c2c`) is verified. The USA start
  (`0xffc006 == 2`, `0x44c40`, selectable at the default DIPs via F34 / #74) is
  not covered (open decision 2). Countries `1` (`0x44c54`, China) and `3`
  (`0x44c68`, England) require the four-country DIP (F47 / #87) and a winning
  chain from each; they are not covered (open decision 11).
