# CPU match win: lose quote, ranking check, continue countdown, and continue with a credit

This document records the boundaries established by
issue #45 (`F5` of the
arcade-flow batch) and issue #79
(`F39`): the CPU's match win (`0x3eef0–0x3ef54`), event `0xb`'s CPU
lose-quote card (`0x40240–0x4029a`) and the wipe (`0x3fc8a–0x3fcc8`), the
departing player's ranking check (`0x42422–0x42484`) with the factored compare
helper (`0x3baf6–0x3bb32`) and 4-word clear (`0x3b934`), the continue
countdown screen (`0x3efda–0x3f162`), and continue acceptance with a credit
(`0x3f166–0x3f1b8`) into the rematch against the same opponent (`0x3e6ca`),
verified from the round-1 fixture at checkpoint `2389` through continue expiry
at frame `6100` (`sf-match-end-01`) and through frame `7000` of the rematch
(`sf-continue-01`). Continue-expiry game over continues through frame `6330`
and reports the attract boundary at `6331`; see [game-over](game-over.md).
Translated in `crates/sf-engine/src/tasks.rs` and
`crates/sf-engine/src/fighter.rs`.

## Reference capture (`sf-match-end-01`)

- **Scenario:** `tools/reference/scenarios/sf_match_end_01.json`
  (`sf-match-end-01`: `sf-fixed-fight-01`'s inputs and factory-default DIPs
  (`dsw1 = 0xdfff`, `dsw2 = 0xffff`), frames `0–6400` (`duration_frames = 6401`),
  full checkpoints on every frame `5000–6400`).
- **Determinism (`C-REF`):** two independent MAME 0.289 runs
  (`match-end-01-run-a`, `match-end-01-run-b`, local-only) match byte-for-byte
  across all 7,008 files.
- **Baseline equality:**
  - `frames.tsv` rows `0..=5040` and all five checkpoint artifacts on every
    frame `5000..=5040` are byte-for-byte identical to
    `sf-fixed-fight-audit-01` (`fight-audit-run-a`).
  - `frames.tsv` rows `0..=6400` are byte-for-byte identical to
    `music-03-audio-a`'s.
- **Evidence label:** `DF at 2389. The CPU wins.`

## Exact timeline (`sf-match-end-01`, frames 5014–6100)

| Frames | Slot 1 resume | Slot 2 resume | `gfxctrl` | `fg_scroll` | `bg_scroll` | Latch | Event |
| ---: | --- | --- | --- | --- | --- | --- | --- |
| 5014 | `0x3ec2a` | `0x3f7d6` | `0xe0` | `0x0080` | `0x0160` | `0xff` | Round-2 result settle finishes (`0xff8004` bit `0x20` cleared by slot 2) |
| 5015 (1) | `0x422e0` | `0x40104` | `0xe0` | `0x0080` | `0x0160` | `0xff` | Slot 1 counts Retsu's second win (`0xffc66c = 2`), sets `0xffc009 = 4`, enters `0x3eefa` (`match_end` mode 2), and requests event `0xb`; slot 2 (`0x4001c`) erases `0x1561c`, draws `"YOU LOST"` (`0x157b8` from `0x4010c[2]`), and holds 60 frames |
| 5016–5074 (59) | `0x422e0` | `0x40104` | `0xe0` | `0x0080` | `0x0160` | `0xff` | `"YOU LOST"` 60-frame hold |
| 5075–5314 (240) | `0x422e0` | `0x4029a` | `0xa0` | `0x0080` | `0x1d00` | `0x9d` at 5076 | Slot 2 (`0x4014c` -> `0x40240`) erases `0x157b8`, clears objects, turns off `fg` (`0x40`), clears the text layer, scrolls `bg` to `0x1d00` (`0x4029e[0]`), draws lose quote `0x15848`, queues `0x9d` (popped by vblank 5076), and holds 240 frames (`0x3b57a`) |
| 5315 (1) | `0x422e0` | `0x3f7d6` | `0xa0` | `0x0080` | `0x1d00` | `0xff` | Slot 2 (`0x402ee`) clears bit `0x20` of `0xffc008` and returns to `0x3f7d6` |
| 5316–5413 (98) | `0x422e0` | `0x3fc28` | `0xa0` | `0x0080` | `0x1d00` | `0xff` | Slot 1 (`0x3ef04`) requests event 6; slot 2 (`0x3fc8a`) wipes 49 columns (`0x170` down to `0x110`) with tile `0x004a`, 2 frames per column |
| 5414 (1) | `0x422e0` | `0x3f7d6` | `0xa0` | `0x0080` | `0x1d00` | `0xff` (`0x3f` queued) | Slot 2 (`0x3fcc0`) queues `0x3f` and clears bit `0x20` of `0xffc008` |
| 5415 (1) | `0x3f04c` | `0x3f7d6` | `0x80` | `0x0a80` | `0x0600` | `0x3f` at 5415 | Slot 1 (`0x3ef0e..0x3f046`) sets `0xffc008 = 2`, runs `0x42422` (`0xff8e5e = 0xff86b0`, `0xff8e62 = 0xff868f`, score `0000100` < 10th place `0005000`, clears `0xff86d2..0xff86da` via `0x3b934`), enters `0x3efda` (`0xffc000 = 5 > 0xffc003 = 0`), turns off `bg + fg` (`0x60`), sets `0xffc009 = 0`, `0xffc00a = 0`, `0xffc008 = 8`, clears text layer and objects, sets `fg_scroll = 0x0a80` and `bg_scroll = 0x0600`, requests palette `0x1a268 -> 0x200`, and sleeps 1 frame |
| 5416 (1) | `0x3f066` | `0x3f7d6` | `0x80` | `0x0a80` | `0x0600` | `0xff` | Vblank 5416 uploads palette `0x1a268 -> 0x200`; slot 1 (`0x3f04c`) requests palette `0x1ae68 -> 0x000` and sleeps 1 frame |
| 5417 (1) | `0x3f092` | `0x3f7d6` | `0xe0` | `0x0a80` | `0x0600` | `0xff` (`0x3c` queued) | Vblank 5417 uploads palette `0x1ae68 -> 0x000`; slot 1 (`0x3f066`) turns on `bg + fg` (`0x60`), sets `0xff8e16 = 2` (`0x3f070`), queues `0x3c`, and sleeps 1 frame |
| 5418–6099 (682) | `0x3f162` | `0x3f7d6` | `0xe0` | `0x0a80` | `0x0600` | `0x3c` at 5418 | Vblank 5418 pops `0x3c`; slot 0 starts counter 1 (`0xff8e1e..23 = 01 00 00 00 02 00`) at 5418 and ticks it down to `00 00 00 00 01 00` at 6099; slot 1 (`0x3f092..0x3f162`) blinks `CONTINUE` (`0x15b04`: on at `0x0909`, off at `0x0303`), draws the 2×2 big digits (`0x15ab4`) and small fractional digits (`0xff8e20`), and checks credits/start and `0xff8e23` |
| 6100 | `0x3f204` (`F25`) | `0x3f7d6` | `0xe0` | `0x0a80` | `0x0600` | `0xff` | Slot 0 sets `0xff8e23 = 1`; slot 1 (`0x3f152`) sees `0xff8e23 == 1` and enters the game-over sequence at `0x3f1bc` |

In `sf-round2-02` (from checkpoint `3900` with P1 idle), Retsu KOs Ryu at frame
`4568`, the round-2 result settles at `4796`, event `0xb` runs `4797..=5097`
(`0x9d` on the latch at `4858`), the wipe runs `5098..=5196` (`0x3f` at `5197`),
`0x42422` and `0x3efda..0x3f08c` run `5197..=5199`, and the continue countdown
loop runs from frame `5200` (`0x3c` on the latch at `5200`) through the end of
the capture at `5600`.

## Compare helper (`0x3baf6–0x3bb32`), `0x3b934`, and `0x42422`

1. **`0x3baf6–0x3bb32` (`Bus::compare_bytes_{3,7,15}` / `Bus::compare_bytes`):**
   - `0x3baf6`: `D0 = 3` (`Bus::compare_bytes_3`, exercised in `F29` / `F61`).
   - `0x3bafe`: `D0 = 7` (`Bus::compare_bytes_7`, called by `check_hiscore` at
     `0x3ba9a` and `ranking_check` at `0x4244a`).
   - `0x3bb06`: `D0 = 0xf` (`Bus::compare_bytes_15`, no direct reference found
     in the ROM; labelled unexercised).
   - `0x3bb0e`: `dbra D0, $3bb14` compares `D0` bytes at `(A0)+` and `(A1)+`
     (`Bus::compare_bytes`, also called directly by Geki's `0x4596e` in `F27`),
     returning to `+0` when equal (`Ordering::Equal`), `+4` when `(A1) < (A0)`
     (`Ordering::Less`), and `+8` when `(A1) > (A0)` (`Ordering::Greater`).
2. **`0x3b934–0x3b96a` (`Bus::clear_words_4`):** clears 4 words (`8` bytes)
   starting at `A0` (`D1 = 3` into `0x3b964`).
3. **`0x42422–0x42484` (`ranking_check`):** writes `(ps + 2)` to `0xff8e62` and
   `ps` to `0xff8e5e`, then compares the player's 7-digit score (`ps + 0x22`)
   against the 10 ranking entries (`0xff8e6e + 16 * d2 + 5`, `d2 = 0..=9`). A
   strictly higher score (`Ordering::Greater`) or a tie with entries `0..=8`
   (`0x42450 -> 0x42468: addq.b #1, D2; cmpi.w #9, D2; bls $42458`) stores
   the qualifying entry address (`0xff8e6e + 16 * rank`) at `0xff8e66` and
   reports the name-entry setup `0x42486`; a lower score across all 10 entries
   or a tie with 10th place (`d2 = 9 -> 10 > 9`) reaches `0x42470`, clearing
   the player's 8-byte score via `0x3b934`.

## Native verification (`C-NAT`, `C-STOP`, `C-NEG`)

Verified in `crates/sf-engine/tests/cpu_match_win.rs` and
`crates/sf-engine/tests/round_result.rs`:

1. **Full comparison on `sf-match-end-01` (`2389..=6330`):** native execution
   from checkpoint `2389` matches `sf-match-end-01` across 3,941 per-frame rows
   (`2390..=6330`: `screen_hash`, sound latch `0x9d` at `5076`, `0x3f` at
   `5415` and `6101`, `0x3c` at `5418`, and the game-over sound sequence), all
   1,331 full checkpoints `5000..=6330`, `CPU_RAM_REGIONS` including
   `0xff8e52..0xff8e6e`, and every work-RAM byte outside task slots' saved
   SR/PC/USP and stack memory. It reports `0x3ccae` at frame `6331`.
2. **Full comparison on `sf-round2-02` (`3900..=5600`):** native execution from
   checkpoint `3900` continues past `0x3eefa` (`4797`) through the lose card,
   wipe, `0x42422`, and continue screen (`5200..=5600`), matching all 1,700
   frames and checkpoints.
3. **Boundaries (`C-STOP`):**
   - Continue expiry enters game over at frame `6100`; comparison continues
     through `6330`, with `0x3ccae` reported at `6331` ([game-over](game-over.md)).
   - `0x42486` when the departing player's score beats 10th place or ties
     entries `0..=8`; its ranking-entry pointer is retained at `0xff8e66`
     (while tying 10th place clears the score and continues).
   - When `(0xffc000 as i8) <= (0xffc003 as i8)` at `0x3efda`, the shared
     game-over screen and 240-frame hold lead to `0x3ccae` at frame `5656`.
4. **Continue with a credit (`sf-continue-01`, issue #79, `F39`):**
   - **Scenario:** `tools/reference/scenarios/sf_continue_01.json` (`sf-fixed-fight-01`
     inputs plus `coin1` at `5500` held 12 frames and `start1` at `5560` held 12
     frames, `duration_frames = 7001`, full checkpoints on every frame
     `5400..=7000`). Two independent MAME 0.289 captures (`continue-01-run-a`,
     `continue-01-run-b`) match byte-for-byte across all `8,008` `.tsv`/`.bin`/`.rgba`
     files, and `frames.tsv` rows `0..=5500` plus checkpoints `5400..=5500` equal
     `sf-match-end-01` (`match-end-01-run-a`) before the coin.
   - **Timeline (`5500–7000`):** `coin1` is debounced on release at `5512` and
     slot 7 awards 1 credit (`0xff865d = 1`, `0x0f` latched at `5514`) with a
     6-frame counter pulse (`5513..5519`). At frame `5561`, `continue_frame` sees
     `0xff865a..0xff865d >= 1` and `start1` pressed (`0x3f13a..0x3f13e`), calls
     `0x3b7a8` (`queue_boot_sounds`: `0x27` at `5562`, `0x3f` at `5563`, `0x80` at
     `5564`), and sleeps 1 frame (`0x3f16c -> 0x3f172`). At `5562`,
     `Flow::ContinueAccept` (`0x3f172–0x3f1ac`) erases `CONTINUE` (`0x15b04`) and
     the five countdown digit longs (`0x800738`, `0x80073c`, `0x8007b8`,
     `0x8007bc`, `0x8007c4`), clears `0xff8e1e..0xff8e22`, decrements the credit
     via `0x3bb74` (`0xff865d = 0`), requests the country palette (`0x436b8`), and
     sleeps 1 frame (`0x3f1b2`). At `5563`, `Flow::ContinueCountryPalette`
     requests the stage palette (`0x43748`) and sleeps 1 frame (`0x3f1b8`). At
     `5564`, `Flow::ContinueStagePalette` (`0x3f1b8: bra $3e6ca`) re-enters
     `next_opponent` with `0xffc003 = 0, 0xffc004 = 0` (Retsu), requesting event 4
     (`0x33` at `5565`), event 5 (`0x97` at `5723`, `0x0b` at `5724`), clearing
     both fighters' wins to `0` and queueing Retsu's stage tune `0x28` (`5881..5886`),
     showing `FIGHT!` (`0x94` at `5967`), and entering the rematch fight loop from
     `5996` through `7000`.
   - **Native verification (`C-NAT`):** native execution from `cpu-01-run-a`
     checkpoint `2389` matches all `4,611` per-frame rows (`2390..=7000`) and all
     `1,601` full checkpoints (`5400..=7000`: pixels, work RAM, video RAM, palette
     RAM), with `CPU_RAM_REGIONS` (including `0xff863a..0xff865e` coin slots and
     credits, `0xffc003`/`0xffc004` stage/fight, and `0xff86b8`/`0xff86e2`/`0xffc66c`
     win bytes) and all work-RAM bytes outside task slots' saved SR/PC/USP and
     stack memory matching on every checkpoint.
5. **Mode 4's CPU match win, P2 ranking check, P2 continue, and P2 match win (`sf-versus-carry-02`, issue #80, `F40`):**
   - **Scenario:** `tools/reference/scenarios/sf_versus_carry_02.json` (`2450..=8510`,
     see [versus-carry](versus-carry.md)). After P2 Ken wins the versus match at
     `5222` and enters game mode 4 at `5503`, CPU Retsu wins rounds 1 (`6310`)
     and 2 (`7835`).
   - **Mode-4 CPU match win (`8079..=8479`):** at frame `8079`, `0x3ef22 -> 0x3ef2c`
     sets `0xffc009 = 4` (`MATCH_WINNER = 4`) and requests event `0xb` (`0x40140`
     with `who = 4`, drawing Retsu's lose quote `0x1586c` at `8139` and queueing
     `0x9c`, latched at `8140`). After the wipe (`8380..=8478`), frame `8479` runs
     `0x3ef40..0x3ef50`: sets `0xffc008 = 1`, calls `ranking_check(0xff86da)`
     (`0xff8e5e = 0xff86da`, `0xff8e62 = 0xff868e`, clearing Ken's non-qualifying
     `4,890` score via `0x3b934`), and enters the continue screen (`0x3efda`,
     `0xffc008 = 8`).
   - **Mode-4 P2 continue (`8499..=8510`):** with 1 credit from `coin2` at `6069`
     (`0xff865d = 1`), pressing `start2` at `8499` triggers `0x3f146` (`mask = 2`
     in mode 4) at `8500`, decrementing `0xff865d` to `0` at `8501`, uploading
     country and stage palettes (`8501..=8502`), and re-entering `0x3e6ca`
     (`0xff8e36 = 4`) at `8503` for a mode-4 rematch against Retsu.
   - **Mode-4 P2 match win (`0x3ef22 -> 0x3ef54`):** verified natively in
     `crates/sf-engine/tests/versus_carry.rs` by driving Ken to a 2–0 Hadoken win
     over Retsu in mode 4, confirming `0xffc009 = 2`, `0x40140` (`who = 2`, Ken's
     win quote `0x15800`), `0x4014c` (`0x158e4`, sound `0x9e`), `0x3ef72`
     (`0xffc004 = 1`), and Geki's setup at `0x3e6ca`.
6. **Negative controls (`C-NEG`):**
   - Starting counter 1 one pass late (frame `5418` instead of `5417`) fails
     `compare_cpu_ram` at frame `5417` on `0xff8e16` and fails
     `sf_compare::compare` at frame `5418` when the initial continue digits
     render as `00.00` instead of `10.00`.
   - Zeroing `0x3f1a4..0x3f1a5` (`0x065e` displacement in `lea ($65e,A5), A0;
     jsr $3bb74.l`) so `Flow::ContinueAccept` does not take a credit fails
     `compare_cpu_ram_with_inputs` at frame `5562` on `0xff865d` (`reference 00,
     native 01`).

## Not covered

- `0x3bb06` (`Bus::compare_bytes_15`): translated here, labelled unexercised
  (no direct reference found in the ROM).
- `0x3baf6` (`Bus::compare_bytes_3`) and direct calls to `0x3bb0e`
  (`Bus::compare_bytes`): translated here, exercised in `F27`, `F29`, `F61`.
- Qualifying score and name entry (`0x42486–0x4268a`, `F42`).
- The reset's with-credits branch to `0x3dec2` is covered by `F45` (#85).
