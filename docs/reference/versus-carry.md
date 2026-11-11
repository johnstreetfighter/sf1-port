# Versus winner carries on against the CPU

Issue #66 translates `0x3eec2–0x3eeec`: after a versus win and wipe, the
winner continues in a one-player mode instead of stopping at the ranking
check. The implementation is in `crates/sf-engine/src/tasks.rs`, on the shared
`FightMachine` in `crates/sf-engine/src/fight.rs`. The versus portion is a
controlled fixture (**CF**); the carry-over and following CPU round are a
development-fixture continuation (**DF**).

## One machine for both fight modes

`FightMachine::from_fight_checkpoint` loads any captured fight-loop checkpoint
and accepts both players' directions, buttons and the cabinet inputs. The old
`CpuMachine` and `VersusMachine` names are aliases, not separate state
machines. Capture and comparison use the same concrete implementation through
`run_fight_capture` and `compare_fight_ram_with_inputs`; existing one-player
and versus entry points remain compatibility wrappers. Freeze-DIP handling
and sound-latch write accounting are shared as well.

The compared RAM is the union of `VERSUS_RAM_REGIONS` and `CPU_RAM_REGIONS`.
The CPU list already covers every versus range, so `FIGHT_RAM_REGIONS` uses
that superset. It includes the ranking/name-entry state `0xff8e52–0xff8f0d`,
including `0xff8e5e`, `0xff8e62` and the qualifying-entry pointer at
`0xff8e66`, as well as the CPU reaction and AI tables.

## Carry-over translation

After the versus match-won screen and wipe:

1. The winner selects mode 2 / scene 1 (P1) or mode 4 / scene 2 (P2). The
   departing player is P2 (`0xff86da`) after a P1 win, P1 (`0xff86b0`) after
   a P2 win.
2. `ranking_check` writes the player and input pointers at `0xff8e5e` and
   `0xff8e62`. A non-qualifying score is cleared. A qualifying score stores
   `0xff8e6e + 16 * rank` at `0xff8e66` and reports name entry at `0x42486`.
3. Otherwise `0x3eec2–0x3eee0` clears round and winner, clears effect and
   fighter objects, clears the scene byte, requests the stage palette and
   sleeps. The next frame requests the country palette (`0x3eee6`) and sleeps.
4. The following frame branches directly to player placement (`0x3eeec` to
   `0x3e5d6`), without another country-event wait. Mode-2 HUD/entity pointers
   are installed before the normal next-opponent path requests event 4.
5. Retsu's stage intro, ROUND 1 announcement, fighter and AI setup, walk-in,
   FIGHT! and subsequent CPU decisions then run through the existing paths.

## sf-versus-carry-01

The scenario starts at checkpoint 2450 with checkpoints every frame through
6300. P1 Ryu uses `sf-win-01`'s walk / heavy-Shoryuken pattern. P2 Ken walks
left for 46 frames from 3905, presses LP for four frames from 3965, then
idles. This timing is intentional: the earlier out-of-range jab did not score
and was not accepted as evidence.

Two independent MAME runs produced 3,851 checkpoints and 19,258 observation
files; every `.bin`, `.rgba` and `.tsv` file is byte-identical between runs.
At frame 3973 Ken's jab leaves Ryu at health `02 0b`, Ken at `04 0b`, and Ken's
score at `00 00 00 00 00 01 00 00`. Ryu wins both rounds, 2–0.

| Frame | Observed transition |
| --- | --- |
| 5215 | Ryu wins the versus match; match-won event `0xb` begins |
| 5397 | the versus wipe (event 6) begins |
| 5496 | mode 2; Ken's non-qualifying score clears; stage palette requested |
| 5497 | country palette requested |
| 5498 | direct mode-2 player placement and Retsu's event-4 intro |
| 5650 | ROUND 1 announcement (event 5) |
| 5819 | fighter setup complete and Retsu's AI tables populated |
| 5930 | the one-player fight loop is active |
| 6300 | Retsu has acted and damaged Ryu; the following CPU round is live |

Native execution matches all 3,850 post-fixture frames in the unified RAM
comparison. Native capture comparison also matches all 3,850 screen-hash,
sound-latch and latch-change observations over `2451..=6300`. Explicit native
assertions cover the jab, win/wipe, score clear, ranking pointers, palette
requests, mode-2 placement, Retsu's intro/AI tables and the following fight.
A separate qualifying-score run stops at `0x42486` with `0xff8e66 =
0xff8e6e`.

The negative control changes `0xffc050` at frame 3973. That byte belongs to
the CPU reaction-table pointers: it is absent from `versus_owned_addresses`
and present only in the unified comparison. The comparison detects that exact
byte at that frame.

`sf-timeover-01` now also runs through 10300 with no flow stop: carry-over and
mode-2 placement occur at 10267–10269, followed by Retsu's event-4 intro.

## sf-versus-carry-02 (ticket #80, F40)

Ticket `#80` (`F40`) verifies the P2-win branch (`0x3eeac–0x3eebc`) and the full
game-mode-4 (`Ken` vs. CPU) fight and post-match decision on `FightMachine`:

1. `sf-versus-carry-02` (`tools/reference/scenarios/sf_versus_carry_02.json`)
   runs from checkpoint `2450` through `8510` (`6,061` checkpoints). In rounds 1
   and 2 of the versus match, P1 Ryu and P2 Ken walk together (`2454` and
   `4019`), Ryu lands two heavy Shoryukens per round (leaving Ken at health
   `00 07` = `28` HP while keeping Ryu's score below the 10th-place `5,000`
   internal threshold), and Ken KOs Ryu with three Hadokens (`2729, 2889, 3049`
   in round 1; `4294, 4454, 4614` in round 2). Ken wins the versus match 2–0 at
   frame `5222` (`0xffc009 = 2`, `0xff8e36 = 11`) with score
   `00 00 00 04 08 08 00 00` (`4,880` internal units = `48,800` displayed).
2. After the versus wipe (`5404..=5502`), frame `5503` takes `0x3eeac..0x3eebc`:
   `GAME_MODE = 4`, `SCENE_MODE = 2`, `ranking_check(0xff86b0)` writes
   `0xff8e5e = 0xff86b0` and `0xff8e62 = 0xff868f` and clears P1 Ryu's
   non-qualifying score (`[0; 8]`), and `0x3eebc` branches to `0x3e6ca` (requesting
   the stage palette at `5503` and country palette at `5504`).
3. At frame `5504`, `place_players` (`0x3e674`) installs mode-4 placement:
   P2 (`0xff86da`) at `(0x02bc, 0x02e0)` with HUD offsets `(0x0a, 4)` and
   `0xffe438 = 0xff86da`; CPU (`0xffc664`) at `(0x03bc, 0x0298)` with HUD
   offsets `(4, 0x0a)` and `0xffe43c = 0xffc664`.
4. Retsu's mode-4 opponent intro (`0x3fad6`, event 4) runs over `5505..=5656`,
   followed by mode-4 HUD and `ROUND 1` (`0x3e714` and `0x3e896`, event 5) over
   `5657..=5825` and fighter/AI setup at `5826`.
5. Mode-4 Round 1 begins at `5937` (`0x3ea88` running `input_handler(0xff86da)`
   with `cpu_decide()`, `0x42ac0` dispatching `(0xff86da, 0xffc664)`, camera
   tracking `(P2_STRUCT, CPU_STRUCT)`, `vblank_buttons` latching P2 buttons in
   mode 4, and `step_random` mixing `P2_STRUCT + 2`). Ken walks right (`5934`) to
   scroll the camera to `0xd6`, walks left into Retsu (`5984`), lands LP (`6004`,
   scoring at `6018` to `00 00 00 04 08 09 00 00`), and takes Retsu's hits
   through `0x446f6`'s alive reaction table `0x4473a` (`6045, 6118, 6157, 6233`)
   and KO reaction table `0x4479c` (`6310`). A coin on `coin2` at `6069` adds
   1 credit (`0xff863a = 1`).
6. Mode-4 Round 2 begins at `6933`; Retsu KOs Ken at `7835`. At `8079`,
   `0x3ef22 -> 0x3ef2c` sets `MATCH_WINNER = 4` and spawns event `0xb` (`0x40140`
   with `who = 4`, lose-quote card `0x1586c` and sound `0x9c`). After the wipe
   (`8380..=8478`), frame `8479` runs `0x3ef40..0x3ef50` (`SCENE_MODE = 1`,
   `ranking_check(0xff86da)` writing `0xff8e5e = 0xff86da` and
   `0xff8e62 = 0xff868e` and clearing Ken's `4,890` score) and enters the
   continue screen (`0xffc008 = 0x08`). Pressing `start2` at `8499` triggers
   `0x3f146` (mask `0x02` for P2 in mode 4) at `8503`, decrementing credits and
   re-entering `0x3e6ca` (`0xff8e36 = 4`) for a mode-4 rematch against Retsu.
7. A companion native test also drives a mode-4 P2 match win (`2–0` by Hadokens
   against Retsu), verifying `0x3ef22 -> 0x3ef54` (`MATCH_WINNER = 2`), `0x40140`
   (`who = 2`, Ken's win quote `0x15800`), `0x4014c` (`0x158e4`, sound `0x9e`),
   `0x3ef72` (`FIGHT_NO = 1`), and Geki's setup (`0x3e6ca` -> event 4).

## sf-challenger-01 (ticket #81, F41)

Ticket `#81` (`F41`) translates the challenger join during a one-player fight (`0x3f4a2–0x3f4da` in slot 3's `watch` and `0x423b8–0x4241c` in slot 1's `fight_frame` / `flow`):

1. `sf-challenger-01` (`tools/reference/scenarios/sf_challenger_01.json`) uses `sf-cpu-01`'s boot inputs (`coin1` at `1450`, `start1` at `1520`, `start1` at `1900`) plus `coin2` at `1480` (`12` frames), with checkpoints every frame over `2389..=5530` (`3,142` checkpoints).
2. At frame `2389`, comparing `challenger-01-run-a` against `cpu-01-run-a` shows identical `video.bin` and `palette.bin`, and in `work.bin` only `9` bytes differ across the entire 32 KB image: `0xff8001` (`0x07` vs `0x03`, `CURRENT_SLOT` after slot 7's 6-frame pulse shift from `coin2`), five bytes in slot 7's 68000 stack frame (`0xff85e1 = 0x01` vs `0x06`, `0xff85f9 = 0x72` vs `0x71`, `0xff8605 = 0x8f` vs `0x5a`, and `0xff8620..0xff8622 = 0x819e` vs `0x8270`), `0xff865d = 1` (`0` in `cpu-01`, the second credit remaining after 1P start), and `0xff865f = 4` / `0xff8661 = 4` (`2` in `cpu-01`, sound queue read/write indices).
3. In Round 1 against Retsu, P1 is idle and is knocked out at `3102`. During Round 1's result settle (`3136..=3342`, `0xffc008 = 0x20 != 0x80`), the first `start2` press at `3200` (`12` frames) is ignored while slot 3 stays at `0x3f41e` and credits stay `1`.
4. Round 2 starts at `3723` (`0xffc008 = 0x80`, `ROUND_NO = 1`, slot 3 armed at `0x3f47e` with `D2 = 2`). Pressing `start2` at `3750` (`12` frames) passes `0x3f4a2..0x3f4aa` (`0xffc001 = 3 > 0xffc003 = 0`), sets `0xffc008 = 0x10` and `0xff8f24 = 0x3f4c6`, and decrements credits (`0xff865d = 0`) at `3751`.
5. At `3752`, `fight_frame` enters `0x423b8`, queues sounds `0x3f` and `0x3e`, draws `0x15ba2` (the bordered challenger frame at `0x052c`) and holds `120` frames (`3752..=3871`), types `0x15c30` (`HERE COMES A NEW CHALLENGER` at `0x0636`) at `D4 = 3` over `3872..=3967` and holds `240` frames (`3968..=4207`), and wipes via event 6 (`4208..=4306`).
6. At `4307`, `0x423fc..0x4241c` sets `GAME_MODE = 1`, `ROUND_NO = 0`, `SCENE_MODE = 0`, clears all object regions, places both human players via `0x3e5ea` (`0xff86b0` at `0x0400`, `0xff86da` at `0x0a00`), and requests event 4 in its versus shape (`0x3f9fc`, `bg_scroll = 0x0780`, palette `0x1ae68`), while slot 3 at `0x3f4c6` sees `SCENE_MODE & 0x10 == 0` and resets `0xff8f24 = 0x3f41e`.
7. Event 5 (`ROUND 1`) runs over `4459..=4627`, the versus fight begins at `4739`, P1 Ryu KOs P2 Ken with three Shoryukens at `4938`, and the first versus round result completes at `5522` (`0xff86b8 = 1`, wipe for Round 2 at `5523`).
8. **Buy-in refused with `Buy-In Feature = None` (`sf-buyin-01`, tickets #123 and #142; `F51`, `F70`):** `tools/reference/scenarios/sf_buyin_01.json` has `sf-challenger-01`'s inputs through `start2` at `3750` (`12` frames) with `Buy-In Feature = None` (`dsw1 = 0xdfff`, `dsw2 = 0xfcff`, `0xffc001 = 0x00`), running for `3,775` frames (`0..=3774`) with checkpoints at `0..=830` and `2389..=3770` (`2,213` checkpoints). Two independent MAME 0.289 runs (`buyin-01-run-{a,b}`) match byte-for-byte across all `11,068` observation files, and match `sf-challenger-01` on `frames.tsv` rows `0..=3750` with only `0xffc001` (`0x00` vs `0x05`) differing in `work.bin` through `3750`. While `start2` is held (`3750..3762`) during Round 2 (`0xffc008 = 0x80`, `0xff865d = 1` credit), on every frame `3751..=3762` slot 3 runs `0x3f41e -> 0x3f454 -> 0x3f47e -> 0x3f4a2: move.b ($c001,A5), D0; cmp.b ($c003,A5), D0; bls $3f418` (`0 <= 0`), refuses the join, leaves `0xff865d = 1` and `SCENE_MODE = 0x80`, and resets `0xff8f24 = 0x3f41e`; on release at `3763`, `0xff8f24` re-arms at `0x3f47e`. Verified both from checkpoint `2389` (`#123`, `1,381` frames `2390..=3770`) and from power-on (`#142`, `3,771` frames `0..=3770`, `2,213` checkpoints), plus all six `Buy-In Feature` limits `0..=5` across `stage = 0..=4` at `0x3f4a2`.

## sf-name-entry-01 and sf-name-entry-02 (tickets #82, #122, #128; F42, F50, F56)

Ticket `#82` (`F42`) translates the ranking-player name-entry screen (`0x42486–0x425c2`), the small-offset effect spawner (`0x42f72–0x42fc4`), the effect stream-call opcode `0xff` (`0x42dd2–0x42dd6`), and the cursor stream routines (`0x427e2–0x429f0`). Ticket `#122` (`F50`) translates the ranking insertion and marked-entry return (`0x425c4–0x4268a`), the marked-entry ranking blink loop (`0x42750–0x427e0`), and `fill_words_4` (`0x3b904`). Ticket `#128` (`F56`) verifies the complete path from checkpoint `2389` in both captures through Ken's Stage 1 carry-on:

1. Both `sf-name-entry-01` and `sf-name-entry-02` share a power-on prefix (`0..=9746`): `sf-cpu-01`'s boot inputs (`coin1` at `1450`, `start1` at `1520`, `start1` at `1900`) plus `coin2` at `1480` (`12` frames); `sf-win-01`'s Retsu fight inputs (`2400 <= frame < 5000`), where P1 Ryu beats Retsu 2–0 with score `00 00 00 05 03 02 00 00` (`5,320` internal units = `53,200` displayed, ranking at entry 9 `0xff8efe` above the `5,000` 10th-place threshold); `start2` at `5860` on the first frame of Geki's `FIGHT!`, starting a versus match; and four P2 Ken Hadokens per versus round (`6860 + 140k` in Round 1, KO at `7349`; `8360 + 140k` in Round 2 from checkpoint `8343`, KO at `8855`).
2. After the versus match-won screen and wipe, frame `9745` takes `0x3eeac..0x3eeba` (`GAME_MODE = 4`, `SCENE_MODE = 2`, `ranking_check(0xff86b0)` writing `0xff8e5e = 0xff86b0`, `0xff8e62 = 0xff868f`, and `0xff8e66 = 0xff8efe` for rank 9) and enters `0x42486`: initializes `0xff8e6a` (`[0x31, 0x31, 0x31, 0x00]`, `"..."`), `0xff8e54` (`0x062e`), `0xff8e58` (`0x0444`), `0xff8e5a` (`0xff8e6a`), and `0xff8e52` (`0`), clears the text layer (`0x3bdb0`), draws `0x14656`, `0x1466e`, and the `3x10` character grid from `0x14638` (`A..Z . BACK END` at `0x80062e`), scrolls fg to `0x4080`, requests palette `0x1ac68` at offset `0x200`, turns off bg (`0x20`) and turns on `0xc8`, clears all object regions, arms timer 2 (`0xff8e16 = 4`), and sleeps 1 frame (`0x42552`).
3. At frame `9746`, slot 0 initializes timer 2 (`0xff8e24..0xff8e2a = [3, 0, 0, 0, 2, 0]`, 30 seconds), and slot 1 (`0x42552..0x42582`) spawns the cursor effect `0x1845c` via `0x42f72` into `0xffe100` (`(Y, X) = (0x5c, 0xb4)`, `(row, col) = (0, 0)` at `0xff8e52..0xff8e54`), clears `*(0xff8e5e)` and the attack-button latches (`0x3b6c6`), queues tune `0x38`, runs `0x42d22` once, and sleeps 1 frame into `0x42584`.
4. Each frame in `0x42584..0x425c2` latches the qualifying player's buttons via `0x3b6e2(A0 = *0xff8e62, A1 = *0xff8e5e)` in `vblank` when `(SCENE_MODE & 3) != 0`, draws the timer-2 integer (`0xff8e24` at `0x800b44`) and fractional (`0xff8e26` at `0x800b4a`) countdown digits (`0x3bdf2`), and exits to `0x425c4` when timer 2 expires (`0xff8e29 != 0`) or the cursor effect deactivates (`live_effect_count() == 0`), or else runs `0x42d22` and sleeps 1 frame.
5. In `sf-name-entry-01` (`checkpoints: [2389] + (8343..=10367)`), P1 presses `p1_lp` at `9800` (commits `'A'` = `0x0a` at `9813` -> `[0x0a, 0x31, 0x31, 0x00]`), `p1_right` at `9830` + `p1_lp` at `9840` (commits `'B'` = `0x0b` at `9853` -> `[0x0a, 0x0b, 0x31, 0x00]`), `p1_up` at `9870` (wraps row `0 -> 2` at `9873`) + `p1_left` at `9880` for 12 frames (wraps col `1 -> 0 -> 9 -> 8` `BACK` at `9889`) + `p1_lp` at `9900` (`BACK` = `0x32` at `9913` erases `'B'` -> `[0x0a, 0x31, 0x31, 0x00]`), `p1_up` at `9930` + `p1_lp` at `9940` (commits `'S'` = `0x1c` at `9953` -> `[0x0a, 0x1c, 0x31, 0x00]`), and `p1_down` at `9970` + `p1_right` at `9980` + `p1_lp` at `9990` (`END` = `0x33` at `10005` advances the cursor stream to `0x18f14 -> 0xfd`, deactivating `0xffe100`).
6. At frame `10006` (`#122`, `F50`), `0x425c4..0x42676` clears the 16 effect objects (`0x3b9bc`), queues `0x3f` (`10007`), shifts entries `8..=D2` (`+5..+16` of each 16-byte row at `0xff8e6e + 16 * rank`) down by 1 slot when `D2 < 9`, marks `a2 + 4 = 0xff` (`a2 = *0xff8e66 = 0xff8efe`), copies P1's 8-byte score (`[0, 0, 0, 5, 3, 2, 0, 0]`) to `a2 + 5..+13` and initials (`[0x0a, 0x1c, 0x31]`, `"AS."`) to `a2 + 13..+16`, clears P1's score (`0x3b934`), turns off fg (`0x40`) and turns on bg (`0x20`), requests palette `0x1b868 -> 0`, and calls `0x4268c` (queuing `0x39` at `10008`). Finding `entry + 4 == 0xff` at `0x42704..0x42726`, `0x42750..0x427e0` arms timer 3 (`0xff8e16 = 8`) and sleeps 1 frame into `0x4275c`, blinking the marked row's score and initials every 6 frames (`0x3bdf2` with `attr = 0x1000` / `0xa000` when `0xff8e2c == 0x0909`, and blanking with `0x3b914` / `0x3b904` to `0x003c` when `0xff8e2c == 0x0303`) through `10212`. At `10213`, `0xff8e2f == 1` triggers `0x427d0..0x427e0 -> 0x4267a..0x4268a`: queues `0x3f` (`10214`), clears the text layer (`0x3bdb0`), clears `SCENE_MODE = 0` and `a2 + 4 = 0`, and returns to `0x3eec2..0x3eeec -> 0x3e5d6`, starting Ken's mode-4 Stage 1 intro (`10215..=10366`) and `ROUND 1` (`10367`).
7. In `sf-name-entry-02` (`checkpoints: [2389] + (8343..=12183)`), P1 leaves controls neutral during name entry; timer 2 counts down 30 seconds over `9746..=11787` and sets `0xff8e29 = 1` at `11788`, where `0x425c4` inserts default initials `[0x31, 0x31, 0x31]` (`"..."`) into rank 9 (`0xff8efe..0xff8f0e`), blinks on the ranking screen over `11788..=11994`, exits at `11995`, and runs Ken's mode-4 Stage 1 intro (`11997..=12148`) and `ROUND 1` (`12149..=12183`).
8. **End-to-end verification (`#128`, `F56`) and controlled vs. complete-gameplay classification (`C-DOC`):**
   - **Controlled-fixture spans (`CF`):** `8343..=10367` in `sf-name-entry-01` (`2,024` frames / checkpoints) and `8343..=12183` in `sf-name-entry-02` (`3,840` frames / checkpoints), starting from checkpoint `8343` at the start of versus Round 2 with dense per-frame checkpoints (`screen_hash`, `latch`, `latch_changed`, checkpoint pixels, video RAM, palette RAM, and work RAM).
   - **End-to-end gameplay from checkpoint `2389` (`DF` with `CF` windows):** `2389..=10367` in `sf-name-entry-01` (`7,978` frames `2390..=10367`, `2,025` checkpoints) and `2389..=12183` in `sf-name-entry-02` (`9,794` frames `2390..=12183`, `3,841` checkpoints). Starting from the Retsu Round 1 `FIGHT!` checkpoint (`2389`), the native engine runs the complete Retsu match won against the live CPU, country travel to Geki's `FIGHT!`, P2's challenger join (`5860`), the two-round versus match lost to P2 Ken, P1's name entry (`"AS."` in `-01`, timeout `"..."` in `-02`), ranking insertion and marked-entry blink, and Ken's mode-4 Stage 1 carry-on through `ROUND 1`, matching `frames.tsv` (`screen_hash,latch,latch_changed`) on every frame in `2390..=10367` and `2390..=12183` and full checkpoints across `8343..=10367` and `8343..=12183` with `0` untranslated branches.

## sf-full-2p-01: full two-player session from power-on (ticket #138, F66)

Ticket `#138` (`F66`) connects `0x3f364..0x3f372 -> 0x3ccae` (`game_over_reset` to `Flow::AttractJoin`) and verifies a complete `21,300`-frame (`0..=21299`) two-player arcade session from power-on (`tools/reference/scenarios/sf_full_2p_01.json`, `2,057` sparse checkpoints around every transition):

1. **Power-on through attract demo and first coin (`0..=2014`):** runs from power-on (`frame 0`) through the warning screen (`7..=793`), attract wall, title, attract ranking (`1648..=1854`), and attract demo setup (`1855`, spawning slot 2 at `0x3f7b0 -> 0x3f7d6` and entering demo `FIGHT!` at `1940`). Pressing `coin1` at `2000` (`12` frames) during the demo fight exercises `0x4822e..0x48242` with an occupied slot 2 (`0xff8026 == 0x08`, `d0 & 2 == 0` at `0x48236`, or `0xff8026 == 0x02` when ready): at `2014`, slot 7 kills slot 2 (`0x4823e -> 0x3b230`, `0xff8026 = 0`) and spawns slot 1 at `0x3dec2` (`start_screen_entry`, `MODE_DEMO = 0xff`, `0xff865d = 1`).
2. **Second coin, 2P start, and country select (`2030..=2980`):** `coin2` at `2030` awards credit 2 (`0xff865d = 2` at `2042`); `start2` at `2110` takes `0x3e004 -> 0x3e486` (`GAME_MODE = 1`, `0xff865d = 0` at `2111`); `start1` at `2450` and `start2` at `2550` select Japan for both players, entering Versus Match #1 (`ROUND 1` `FIGHT!` at `2980`).
3. **Versus Match #1 (`2980..=7200`, P2 Ken wins 2–0):** P1 Ryu and P2 Ken walk together (`2990`, `4932`) and P2 Ken KOs P1 Ryu with three light Hadokens per round (`+120`, `+280`, `+960`), winning 2–0 with score `00 00 00 04 08 08 00 00` (`4,880 < 5,000`). At `6990`, `0x3eeac..0x3eebc` sets `GAME_MODE = 4`, checks loser P1 Ryu (`0xff8e5e = 0xff86b0`, `0xff8e62 = 0xff868f`, score `0 < 5,000`), and carries P2 Ken into Retsu's mode-4 fight (`FIGHT!` at `7200`).
4. **P1 challenger join into P2's mode-4 fight (`7200..=8249`, `0x3f448` `D2 = 1`):** `coin1` at `7220` awards 1 credit (`0xff865d = 1` at `7235`), arming slot 3 (`watch`) at `0x3f47e` with `D2 = 1` (`0x3f448` for `GAME_MODE == 4`). Pressing `start1` at `7260` triggers `0x3f4ae..0x3f4c0` at `7261` (`SCENE_MODE = 0x10`, `0xff865d = 0`, `0xff8f24 = 0x3f4c6`) and `0x423b8` (`HERE COMES A NEW CHALLENGER`) at `7262..=7816`, restoring `GAME_MODE = 1` at `7817` and starting Versus Match #2 (`8249`).
5. **Versus Match #2 (`8249..=12469`, P1 Ryu wins 2–0):** P1 Ryu KOs P2 Ken with three light Hadokens per round (`8249`, `10191`), winning 2–0 with score `00 00 00 04 08 08 00 00` (`4,880 < 5,000`). At `12259`, `0x3ee7a..0x3ee8a` sets `GAME_MODE = 2`, checks loser P2 Ken (`0xff8e5e = 0xff86da`, `0xff8e62 = 0xff868e`, clearing Ken's `4,880` score to `[0; 8]`), and carries P1 Ryu into Retsu's mode-2 fight (`FIGHT!` at `12469`).
6. **Mode-2 loss, continue, second loss, game over, and return to attract (`12469..=21299`):** `coin1` at `12489` adds 1 credit (`0xff865d = 1`), and P1 walks into Retsu (`12479`, `13914`) and loses 0–2. At `16533`, `0x3ef0a..0x3ef18` checks P1 Ryu (`4,880 < 5,000`, clearing P1's score to `[0; 8]`) and enters continue (`0xffc008 = 0x08`). Pressing `start1` at `16553` takes `0x3f146` (`1 -> 0` credits at `16556`) and restarts Retsu (`16989`, `18494`). P1 walks into Retsu again (`16999`, `18504`) and loses 0–2 with 0 credits remaining; the continue countdown expires at `20941` (`0x3f1bc`), game over runs over `20941..=21212`, and at `21213` `game_over_reset` (`0x3f33c..0x3f372`) kills slot 2 (`0xff8026 = 0`) and enters `0x3ccae` (`Flow::AttractJoin`: `MODE_DEMO = 0`, slot 3 spawned at `0x3f374`, slot 1 at `0x3ccec`), running cleanly in attract through `21299`. Neither player ever reaches the `5,000` 10th-place threshold, so `0xff8e6e..=0xff8f0d` at `21299` equals the boot fill at frame `8`.

## Not covered

- P2's name entry (`0x3eef0`), and both players entering names in one game over (open decision 3).
