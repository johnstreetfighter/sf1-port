# Coins, service credits and 1P/2P start on the start screen

This document records the boundaries established by
issue #52 (`F12` of the
arcade-flow batch), issue #73
(`F33`), issue #75 (`F35`), and
issue #74 (`F34`), and
issue #86 (`F46`): the
attract-to-start transition in slot 7 (`0x48228–0x48248`), the start-screen
entry (`0x3dec2–0x3df48`) and start-screen loop in slot 1 (`0x3df4c–0x3e020`,
resume `0x3dfce`), the coin-arrival, service-credit, 9-credit lockout and
coin-counter pulse paths in slot 7 (`0x4819e–0x4834c`), the two-country select
(`0x3e04c–0x3e1e3`, `0x3e372–0x3e596`, `0x3bc3e`, `0x3bd3e`), and the fresh
two-player start (`GAME_MODE = 1`) through events 2/3, mode-1 placement
(`0x3e5ea`), event 4's versus shape, `ROUND 1`, `FIGHT!`, and continuation into
`sf-move-01` (`2450..=3350`), verified across `sf-credit-select-01`,
`sf-credits-01`, `sf-start-2p-01`, `sf-coinage-01`, `sf-select-usa-01`,
`sf-move-01`, and `sf-fixed-fight-01`. Translated in
`crates/sf-engine/src/tasks.rs` and `crates/sf-engine/src/machine.rs`.

## Reference captures (`C-REF`)

All scenarios run on the pinned MAME 0.289 baseline with full checkpoints on
every frame from `1440` through the scenario's final frame:

| Scenario | File | Frames | Checkpoints | Two-run determinism (`run-a == run-b`) | Evidence label |
| --- | --- | ---: | ---: | --- | --- |
| `sf-credit-select-01` | `tools/reference/scenarios/sf_credit_select_01.json` | `0–1870` (`1871`) | `1440–1870` (`431`) | `2,591 / 2,591` files byte-for-byte identical | `DF at 1440..1870, with coin1@1450, start1@1520` |
| `sf-credits-01` | `tools/reference/scenarios/sf_credits_01.json` | `0–2160` (`2161`) | `1440–2160` (`721`) | `4,331 / 4,331` files byte-for-byte identical | `DF at 1440..2160, with coin1/coin2/service1/start2/start1` |
| `sf-start-2p-01` | `tools/reference/scenarios/sf_start_2p_01.json` | `0–2459` (`2460`) | `1440–2450` (`1011`) | `6,071 / 6,071` files byte-for-byte identical | `DF at 1440..2450, then CF over 2451..3350 against sf-move-01` |
| `sf-select-usa-01` | `tools/reference/scenarios/sf_select_usa_01.json` | `0–1670` (`1671`) | `1440–1670` (`231`) | `1,391 / 1,391` files byte-for-byte identical | `DF at 1440..1670, with coin1@1450, start1@1520, right/left/right/up-left, p1_button1@1648` |

- `sf-credit-select-01` and `sf-fixed-fight-01` share identical inputs through
  frame `1870` (`coin1` at `1450`, `start1` at `1520`) and match `fight-run-a`'s
  `frames.tsv` rows `0..=1870` and checkpoints `1500` (`1500=start screen (slot
  1 at 0x3dfce)`) and `1860` byte-for-byte.
- `sf-credits-01` matches `fight-run-a` across `frames.tsv` rows `0..=1519` and
  checkpoint `1500`.
- `sf-start-2p-01` matches `fight-run-a` across `frames.tsv` rows `0..=1479` and
  matches `sf-move-01` across `frames.tsv` rows `0..=2459` and checkpoint `2450`.

## Original 68000 and MAME hardware behavior

### 1. Attract-to-start transition (`0x48228–0x48248`) and start-screen entry (`0x3dec2–0x3df48`, issue #73)

When `coin1` is held for 12 frames starting at frame `1450` during the attract
title wait (slot 1 sleeping inside `0x3b588` with `caller = 0x3cdb8` at frame
`1440`):

- Vblank `0x3b31a` (`coin_debounce`) increments slot 1's hold counter `+0`
  (`0xff863a`) each vblank while the switch is held (`1451..1462`, reaching
  `12`) and on release (`1463`), since `2 <= count < 0x64`, clears `+0 = 0` and
  increments `+1` (`0xff863b`, pending coins) to `1`.
- At frame `1467`, slot 7 wakes from its 6-frame loop sleep at `0x4819e`, sees
  `0xff863b == 1`, runs `0x482a8..0x48324` (`0xff863b = 0`, `0xff863e = 0`,
  `0xff8640 = 1`, queues sound `0x0f`, calls `0x3bb5c` (`decimal_up`) at
  `0xff865e` so `0xff865a..0xff865d = 0x00000001`), and at `0x48212` sees
  `0xff8003 == 0` (`MODE_DEMO == 0`) and `0xff865a..0xff865d != 0`.
- **`0x48228–0x48248` (`random_after_service`):**
  1. `0x48228`: copies the random byte `0xff8f28` into `0xffc007`
     (`move.b ($f28,A5), ($4007,A5)`).
  2. `0x4822e–0x48236`: sets `D0 = 2` and calls `0x3b230` (`scheduler.kill(bus, 2)`).
     During the title wait slot 2 is empty (`state == 0`, so `0x3b230` is a
     no-op after clearing the slot header); killing an occupied slot 2 during
     the attract demo is covered by `F66`.
  3. `0x48238–0x48246`: sets `D0 = 1`, `A1 = 0x3dec2`, calls `0x3b20a`
     (`scheduler.spawn(bus, 1, 0x3dec2)`, writing state `1` to `0xff8016` and
     setting `0xff8005 = 1` to request a scheduler rescan), and sleeps 1 frame
     (`0x3b51a`) into `0x4824e` (`Random::AttractStarted`).
- Because `0xff8005 == 1`, the task scheduler (`0x3b1ca`) rescans from slot 0 in
  the same frame (`1467`) and dispatches slot 1's new task at `0x3dec2`
  (`start_screen_entry`).
- **`0x3dec2–0x3df48` (`start_screen_entry`):**
  1. `0x3dec2`: sets `0xff8003 = 0xff` (`MODE_DEMO = 0xff`, causing slot 3 at
     `0x3f3ac` to exit its attract `INSERT COIN` loop into `0x3f410` on its
     very next turn in frame `1467`).
  2. `0x3dec8–0x3ded6`: clears all three object pools (`0x3b9bc` -> `0xff9000`,
     `0x3b9c8` -> `0xff9600`, `0x3b9d4` -> `0xffa200`).
  3. `0x3deda–0x3dee8`: queues `0x3f` (`0x3b802`, music stop) and `0x80`
     (`0x3b802`, voice stop); behind the coin chime `0x0f` queued earlier in
     frame `1467`, vblank pops `0x0f` at `1468`, `0x3f` at `1469`, and `0x80`
     at `1470` (returning `0xff89b8` to `0xff` at `1471`).
  4. `0x3deec–0x3defc`: clears `0xffc00b`, `0xffc002` (`GAME_MODE`), `0xffc008`,
     and `0xff8e36` (`SCENE_MODE`).
  5. `0x3df00–0x3df16`: disables layers `0xe8` (`0x3b692`), enables `tx` only
     (`0x08`, `0x3b644`), and clears the text tilemap (`0x3bdb0`).
  6. `0x3df1a–0x3df3e`: loads `A2 = 0x3e024` (`0x14dec`, `0x14e00`, `0x14e2a`
     `PUSH 1 PLAYER START BUTTON`, `0x14e7c`, `0x14ea8`) when
     `0xff865a..0xff865d <= 1`, or `A2 = 0x3e038` (`0x14e4e` `PUSH 1 OR 2
     PLAYERS START BUTTON` at index 2) when `> 1`, and draws all 5 text blocks
     with `0x3bba0`.
  7. `0x3df40–0x3df48`: clears the blink timers `0xffc01a = 0` and
     `0xffc01b = 0` and falls through into `0x3df4c` (`start_screen_frame`),
     which increments `0xffc01a = 1`, `0xffc01b = 1`, draws the credit digits
     (`0x3be8a`), and sleeps at `0x3dfce`.
- At frame `1468`, slot 7 wakes at `0x4824e` (`Random::AttractStarted`), pulses
  the slot-1 coin counter (`0x48332: D2 = 0x0001` from `0x48274`,
  `0xff8684 = 0x7071`), and sleeps 6 frames at `0x48344` (`caller = 0x48262`).

### 2. Slot 1 start-screen loop (`0x3df4c–0x3e020`, resume `0x3dfce`)

On each wakeup at `0x3dfce`:

1. Increments `0xffc01a` (`addq.b #1, ($401a,A5)`). When `0xffc01a == 0x0f`
   (`15`), erases (`0x3bd00`) `0x14e4e` (`PUSH 1 OR 2 PLAYERS START BUTTON`) if
   `0xff865a..0xff865d > 1` (`cmpi.l #1, ($65a,A5); bgt`), else `0x14e2a`
   (`PUSH 1 PLAYER START BUTTON`), and clears `0xffc01a = 0`.
2. Increments `0xffc01b` (`addq.b #1, ($401b,A5)`). When `0xffc01b == 0x1e`
   (`30`), draws (`0x3bba0`) `0x14e4e` if `0xff865a..0xff865d > 1`, else
   `0x14e2a`, and clears `0xffc01b = 0`.
3. Calls `0x3be8a` (`draw_credit_digits`), which calls `0x3bdfa` (`A1 = 0xff865b`,
   `D0 = 0x0e66`, `D2 = 0x4000`, `D4 = 2`) to draw the three credit digits at
   `0x800e66..0x800e6a` with leading zeros blanked (`0x403c`).
4. Reads `D1 = (~SYSTEM) & 0x0003` (`0x3dfca`). If `D1 == 0`, calls `0x3b5aa`
   (`speed_sleep`, 1 frame at default `DSW2`) and resumes at `0x3dfce`.
5. If `D1 == 1` (`start1` pressed): sets `0xffc002 = 2` (`GAME_MODE = 2`, 1P vs
   CPU) and `D2 = 0` (1 credit cost).
6. If `D1 != 1` (`start2` pressed): checks `cmpi.l #2, ($65a,A5); bcs $3dfca`.
   When credits `< 2`, `start2` is ignored and the loop sleeps at `0x3dfce`;
   when credits `>= 2`, sets `0xffc002 = 1` (`GAME_MODE = 1`, 2P versus) and
   `D2 = 1` (2 credits cost).
7. Calls `0x3bb74` (`decimal_down` at `0xff865e`) `D2 + 1` times, calls `0x3bd96`
   (`erase_credit_digits`, writing `0x103c` to `0x800e66..0x800e6a`), and erases
   the 5 text blocks listed at `0x3e038` (`0x14dec`, `0x14e00`, `0x14e4e`,
   `0x14e7c`, `0x14ea8` — note that `0x3e00c` unconditionally loads `0x3e038`,
   never `0x3e024`), then falls through to `0x3e04c` (`country select`, `F34`).

### 3. Slot 7 coins, service switch, 9-credit lockout and counter pulses (`0x4819e–0x4834c`)

- **Coin slot step (`0x4827c–0x48324`):**
  - If `+1` (`pending`) is `0`: when `0xff865c < 9` and `0xff8684 & 0x01e0 == 0`,
    calls `0x3b654` (`coin_ctrl_or(0x7070)`).
  - If `+1 != 0`: returns immediately when `0xff865c >= 9`. Otherwise ensures
    `0x7070` is set if `0xff8684 & 0x01e0 == 0`, decrements `+1`, increments
    `+4` (coins toward next credit) and `+6` (pending counter pulses), queues
    sound `0x0f` (`0x3b802`), and when `+4 == +2` (coins required), resets
    `+4 = 0` and loops `0x3bb5c` (`decimal_up(0xff865e)`) `+3` times, stopping
    early with `0x3b6a2` (`coin_ctrl_off(0x7070)`) as soon as `0xff865c >= 9`.
- **9-credit check and service switch (`0x481be–0x4820c`):**
  - At `0x481ca`: if `0xff865c >= 9`, branches to `0x4817a`
    (`coin_ctrl_off(0x7070)`) and jumps directly to `0x48212`, bypassing the
    service switch check completely.
  - Otherwise, if `SYSTEM` bit 2 is low (`service1` pressed, `0x481da`), sleeps
    6 frames (`0x3b502`, resume `0x481e8`). On wakeup (`0x481e8`), if `SYSTEM`
    bit 2 is still low, awards 1 credit (`0x3bb5c` at `0xff865e`), queues sound
    `0x0f` (`0x3b802`), and sleeps 30 frames (`0x3b552`, resume `0x48212`).
- **Coin counter pulse loop (`0x4824e–0x4826a`, `0x48326–0x4834c`):**
  - Iterates `D1 = 3..=0` across `0xff863a..0xff8652` with `D2` from `0x48274`
    (`[0x0200, 0x0100, 0x0002, 0x0001]`). When `+6 != 0`, decrements `+6`, calls
    `0x3b654(D2)` (setting the slot's counter bit, e.g. `0x0001` for slot 1 or
    `0x0002` for slot 2), sleeps 6 frames (`0x3b502`, resume `0x48344` with `D1`,
    `D2`, `A1` on the stack and caller `0x48262` at `usp + 68`), and on wakeup
    calls `0x3b6a2(D2)` before continuing `dbra D1`.
- **Hardware coin lockout (`0xc00010` / `sf_state::coin_w`):**
  - In `sf-mame0289.cpp` (lines 110–116), writing `0xc00010` calls
    `machine().bookkeeping().coin_lockout_w(0..3, ~data & 0x10..0x2000)`. When
    `0x3b6a2(0x7070)` clears bits `0x7070` at 9 credits, MAME's input port layer
    locks out `COIN1` (`0x0010`) and `COIN2` (`0x0020`) so `IN0` bits `0x01` and
    `0x02` remain high (released) even while a coin switch is held (`coin1` at
    frame `2080` in `sf-credits-01`). `MachineState::coin_ctrl` models this
    register directly.

## Event timeline in `sf-credits-01` (`1500–2141`)

| Frame(s) | Input (held 12 frames) | Credits (`0xff865c`) | `coin_ctrl` (`0xff8684`) | Behavior |
| ---: | --- | ---: | --- | --- |
| `1500` | — | `1` (`1` in `sf-start-2p-01` becomes `2` after `coin2@1480`) | `0x7070` | Start-screen fixture (`0x3dfce`) after `coin1` at `1450` |
| `1540–1551` | `start2` | `1` | `0x7070` | `0x3dfe8: cmpi.l #2, ($65a,A5); bcs $3dfca` ignores `start2` with 1 credit |
| `1580–1591` | `coin2` | `1 -> 2` (at `1593`) | `0x7072` (`1593–1598`) | Vblank debounces slot 2 on release (`1592`); slot 7 awards credit 2 at `1593`, queues `0x0f` (latch write at `1594`), pulses `0x0002`, and prompt switches to `PUSH 1 OR 2 PLAYERS START BUTTON` (`0x14e4e`) |
| `1620–1631` | `coin1` | `2 -> 3` (at `1634`) | `0x7071` (`1634–1639`) | Vblank debounces slot 1 on release (`1632`); slot 7 awards credit 3 at `1634`, queues `0x0f` (latch write at `1635`), and pulses `0x0001` |
| `1660–2009` | `service1` × 6 (`1660, 1720, 1780, 1840, 1900, 1962`) | `3 -> 9` (at `1973`) | `0x7070 -> 0x0000` (at `2010`) | Each `service1` press sleeps 6 frames (`0x481e8`), awards 1 credit + sound `0x0f` (`1668, 1732, 1789, 1853, 1910, 1974`) + 30-frame cooldown (`0x48212`); after the 6th cooldown (`1973..2003`) + 6-frame loop sleep (`2003..2009`) + 1-frame pass (`2009..2010`), `0x4817a` clears `0x7070` (`0xff8684 = 0x0000`) at frame `2010` |
| `2020–2031` | `service1` (7th) | `9` | `0x0000` | `0x481ca: cmpi.w #9, ($65c,A5); bcc $4817a` bypasses service switch at 9 credits |
| `2080–2091` | `coin1` | `9` | `0x0000` | `coin_ctrl & 0x0010 == 0` locks out `COIN1` in `IN0`; `0xff863a..0xff8641` and credits stay unchanged |
| `2141` | `start1` (at `2140`) | `9 -> 8` | `0x0000` | `GAME_MODE = 2` (`0xffc002`), `decimal_down` to `8`, erases credit digits and `0x3e038` blocks, halts at `0x3e04c` (`C-STOP`) |

## Native verification (`C-NAT`, `C-STOP`, `C-NEG`)

Verified in `crates/sf-engine/tests/start_screen_credits.rs`:

1. **Title-to-start transition (`1440..=1520`, `1440..=2140`, `1440..=1560`, issue #73):**
   native execution from the title-hold fixture at checkpoint `1440` matches every
   frame across `sf-credit-select-01` (`1441..=1520`), `sf-credits-01`
   (`1441..=2140`), and `sf-start-2p-01` (`1441..=1560`), verifying `coin1`
   debounce (`1451..=1462`, release at `1463`), slot 7's attract-to-start handoff
   at `1467` (`0xffc007 == 0xff8f28`, slot 3 switching to `0x3f410`), slot 1's
   `start_screen_entry` (`0x3dec2..0x3df48`), sound-latch transitions `0x0f` at
   `1468`, `0x3f` at `1469`, `0x80` at `1470` (`0xff` at `1471`), and the
   country-select stop (`0x3e04c`) at `1521` / `2141` / `1561`.
2. **`sf-credit-select-01` (`1500..=1520`):** native execution from checkpoint
   `1500` matches every frame `1501..=1520` across `screen_hash`, checkpoint
   RGBA pixels, video RAM, object RAM, palette RAM, sound latch,
   `CPU_RAM_REGIONS` (including `0xff863a..0xff865e`, `0xff8684..0xff8686`, and
   `0xffc01a..0xffc01c`) and full work RAM, and at frame `1521` sets
   `0xffc002 = 2`, deducts credits `1 -> 0`, erases all 5 blocks of `0x3e038`
   (including `0x14e4e`), and reports `0x3e04c` (`C-STOP`).
3. **`sf-credits-01` (`1500..=2140`):** native execution from checkpoint `1500`
   matches all `640` frames (`1501..=2140`) across every compared surface, two
   6-frame coin counter pulses (`0x48344`), six 6-frame service debounce sleeps
   (`0x481e8`) + six 30-frame service cooldown sleeps (`0x48212`), 9-credit
   lockout at frame `2010` (`0xff8684 = 0x0000`) suppressing `coin1` at `2080`,
   and at frame `2141` sets `0xffc002 = 2`, deducts credits `9 -> 8`, erases all
   5 blocks of `0x3e038`, and reports `0x3e04c` (`C-STOP`).
4. **`sf-start-2p-01` (`1500..=1560`):** native execution from checkpoint `1500`
   matches all `60` frames (`1501..=1560`) across every compared surface, and at
   frame `1561` sets `0xffc002 = 1` (2P versus), deducts credits `2 -> 0`,
   erases all 5 blocks of `0x3e038`, and reports `0x3e04c` (`C-STOP`).
5. **Coinage DIP at non-default rates (`sf-coinage-01`, issue #75, `F35`):**
   - `sf-coinage-01` (`tools/reference/scenarios/sf_coinage_01.json`,
     `Coin A = 2C_1C`, `Coin B = 1C_6C`, `dsw1 = 0xdfda`, `dsw2 = 0xffff`,
     frames `0..=1720`, checkpoints `1440..=1720`): two MAME 0.289 runs
     (`coinage-01-run-{a,b}`) match byte-for-byte across all 1,408 observation
     files (`C-REF`).
   - `coin2@1450` (`1C_6C`) awards 6 credits and opens the start screen at
     frame `1467` (`6` credits at `1500`); from both checkpoint `1500` and
     checkpoint `1440` through `1720`, native execution matches the reference on
     every frame (`C-NAT`):
     - first `coin1@1540` (`2C_1C`) awards `0` credits (`6` total) and
       increments slot 1's partial-coin counter (`0xff863e = 1`);
     - second `coin1@1600` (`2C_1C`) reaches `0xff863c == 2`, resets
       `0xff863e = 0`, and awards `1` credit (`7` total);
     - second `coin2@1660` (`1C_6C`) awards `2` credits (`7 -> 9`), stops at
       `9` inside `0x48308` (`cmpi.w #9, ($65c,A5)`), and calls `0x3b6a2(0x7070)`
       to clear `0xff8684 = 0x0000` (`state.coin_ctrl = 0x0000`).
   - Static check decodes all 64 `(Coin A, Coin B)` settings (`8 × 8`) through
     `0x1456a` with `0x3b2e4` (`decode_coinage`).
6. **Two-country select (`0x3e04c–0x3e1e3`, `0x3e372–0x3e596`, `0x3bc3e`, `0x3bd3e`, issue #74, `F34`):**
   - **`0x3e04c–0x3e0e0` (`country_select_start`):** spawns slot 2 at `0x3f7b0`
     (`Screen::Poll`), clears `0xffc003 = 0` (`STAGE`), `0xffc004 = 0` (`FIGHT`),
     sets `0xff8003 = 1` (`MODE_DEMO = 1`), enables layers `0xe8`, copies 64 words
     of scroll tables (`0x3e24e..0x3e2cd` -> `0xffc682..0xffc701`), writes
     `SCROLL1 = 0x2000` (`0xffc680`), `SCROLL2 = 0x1000` (`0xffc682`), draws four
     `0x3bba0` blocks from `0x3e1fe` (`0x14d0c`, `0x14d46`, `0x14d80`, `0x14d8e`),
     five `0x3bc3e` 2×2 blocks (`0x14dd8` at `0xffc006 = 0` plus `0x3e20e`:
     `0x14d9c`, `0x14daa`, `0x14db6`, `0x14dc6`), requests palette `0x19e68 -> 0x000`,
     queues tune `0x22` (`0x3b802`), and sleeps 1 frame (`0x3b4ea` into `0x3e0e6`).
   - **`0x3e0e6–0x3e10e` (`country_select_arm`):** sets `0xff8e30 = 5`
     (`SELECT_SECONDS = 5`, BCD), sets bit 4 (`0x10`) of `0xff8e16` (`TIMER_FLAGS`,
     arming the country-select countdown at `0x3f54e` / `0x3f65c`), draws `05` at
     `0x8007a0`, requests palette `0x1aa68 -> 0x200`, and sleeps 1 frame (`0x3b4ea`
     into `0x3e110`).
   - **`0x3e110–0x3e1e3` (`country_select_frame`):** polls `($0, A2)` (`0xff86b0`
     P1 button latch word, set on button release by `0x3b3ae`) and `0xff8e30 == 0`
     (5-second timeout after 344 frames: `1521 -> 1865` in `sf-credit-select-01`,
     `1561 -> 1905` in `sf-start-2p-01`), branching to `0x3e372` when either holds;
     otherwise sleeps 1 frame (`0x3b5aa` into `0x3e13a`), refreshes `0x8007a0`, and
     matches `0xff868f` (`IN1_COPY` low byte) against the exact byte table `0x3e1dc`
     (`0xfd` pure left -> `0`, `0xfe` pure right -> `2`; diagonals such as `0xf5`
     up-left do not match). When `D0 != 0xffc006`, erases the old cursor (`0x3bd3e`),
     stores `0xffc006 = D0`, draws the new cursor (`0x3bc3e`), and queues sound `0x09`.
   - **`0x3e372–0x3e596` (`country_select_exit`):** clears bit 4 (`0x10`) of
     `0xff8e16`, sets `0xffc005 = 0xffc006` (`COUNTRY`), queues sound `0x3f`
     (music stop), disables layers `0xe8`, resets scroll `0xffc680 = 0`,
     `0xffc682 = 0`, erases `0x80079e..0x8007a2` and all select text/2×2 blocks,
     copies the 16-byte opponent order from `0x3e420[(~DSW2) & 3] + (0xffc006 << 3)`
     into `0xffc058..0xffc067` (`[0, 1, 2, 3, 2, 4, 5, 6, 5, 7, 8, 9, 8, 10, 11, 12]`
     for Easy + Japan vs `[2, 3, 0, 1, 0, 4, ...]` for Easy + USA), fills all **15**
     per-opponent pointer arrays (`D2 = 14..=0`) at `0xffc01c..0xffc057`
     (`0x19908 + 0x28 * 0xffc006 + 0xa0 * n`) and the 15 × 64-byte AI tables at
     `0xffc068..0xffc427` from `0x18be8 + 0x40 * n`, and enters `0x3e59c`
     (`StageStart`).
7. **Negative controls (`C-NEG`):**
   - Delaying the title `coin1` by 10 frames (`1460` instead of `1450` in
     `sf-credit-select-01` from checkpoint `1440`) is caught at frame `1451` on
     `0xff863a` by `compare_cpu_ram_with_inputs` and by `sf_compare::compare`.
   - Delaying `coin2` by 10 frames (`1590` instead of `1580` in `sf-credits-01`
     from checkpoint `1500`) is caught at frame `1581` on `0xff8642` by
     `compare_cpu_ram_with_inputs` and by `sf_compare::compare` on `screen_hash` /
     `latch` / checkpoint RAM.
   - Delaying P1 LP (`p1_button1`) by 10 frames (`1658` instead of `1648` in
     `sf-select-usa-01` from checkpoint `1500`) is caught at frame `1649` on
     `0xff868f` by `compare_cpu_ram_with_inputs` and at frame `1661` by
     `sf_compare::compare`.
   - Swapping P1's and P2's placement words (`+0x0a..+0x20` at `0xff86b0` /
     `0xff86da` and initial X placement words `0x16062` / `0x16632`) in
     `sf-start-2p-01` is caught at frame `2319` (the first frame the fighters
     are drawn) by both `compare_cpu_ram_with_inputs` and `sf_compare::compare`
     (issue #86, `F46`).
   - Perturbing the 4-byte cursor-to-country map at `0x3e2ea` (`[0, 2, 3, 1]` ->
     `[0, 1, 3, 2]`) in `sf-select4-01` is caught at frame `1541` (the first
     cursor move) by both `compare_cpu_ram_with_inputs` and `sf_compare::compare`
     (issue #87, `F47`).
8. **Fresh two-player start into the versus fight (`sf-start-2p-01` -> `sf-move-01`, issue #86, `F46`):**
   - From `sf-start-2p-01`'s `1500` checkpoint (and `1440`) over `1501..=2450`,
     native execution matches the reference on every frame across all observables:
     `start2` at `1561` (`GAME_MODE = 1`), country-select timeout at `1905`,
     events 2 and 3 (`1906..=1997`), mode-1 player placement at `0x3e5ea` (`1998`),
     event 4's versus shape (`1998..=2149`), `ROUND 1` (`2150..=2318`), walk-in
     intro (`2319..=2399`), `FIGHT!` (`2400..=2429`), and round start at `2430`.
   - At frame `2450`, owned RAM equals `sf-move-01`'s `2450` fixture (`move-run-a`),
     and continuing on the shared `FightMachine` with `sf-move-01`'s inputs over
     `2451..=3350` matches `sf-move-01`'s checkpoints on every frame.
9. **Four-country select (`0x3e1e4–0x3e371`, issue #87, `F47`):**
   - **`0x3e1e4–0x3e250` (`country_select4_start`):** entered from `0x3e060` when
     `0xffc005 == 0` (`Number of Countries Selected = 4`, `DSW2` bit 10 low); draws
     `0x15d42` (`0x3bba0`), `0x15d1e` (`0x3bc3e` 2×2), and initial Japan marker
     `0x15e64` (`0x3bba0`), sets `fg_scroll = 0x4380` (`0x3b674`), requests palette
     `0x1a668` at `0x200`, turns off layers `0xa0` and turns on `0x48`, clears
     `0xffc006 = 0` (Japan), queues tune `0x34`, and sleeps 1 frame (`0x3b51a`) into
     `0x3e252`.
   - **`0x3e252–0x3e27a` (`country_select4_arm`):** clears `0xffc006 = 0`, arms
     timer counter 4 (`0xff8e16 = 0x10`, `05.00` at `0xff8e30..0xff8e35`), clears
     attack-button latches (`0x3b6c6`) and `0xff86b0`, sets `0xffc008 = 2`
     (`SCENE_MODE`), and sleeps 1 frame (`0x3b51a`) into `0x3e27c`.
   - **`0x3e27c–0x3e371` (`country_select4_frame`):** clears `D6 = 0, D7 = 0` on
     first entry (`0x3e27c`), exits to `0x3e330` on timeout (`0xff8e35 != 0`) or P1
     button release (`0xff86b0 != 0`), and otherwise reads `(!IN1_COPY) & 0x0f`,
     updating `D7` on pure Up (`8 -> 0`) / pure Down (`4 -> 1`) and `D6` on pure
     Left (`2 -> 0`) / pure Right (`1 -> 1`) while ignoring diagonals, mapping
     `(D7 << 1) + D6` through `0x3e2ea` (`[0, 2, 3, 1]`) and redrawing the active
     marker through `0x3e316` (`[0x15e64, 0x15ee2, 0x15f60, 0x15fde]`). On exit
     (`0x3e330..0x3e371`), queues `0x3f`, clears `SCENE_MODE = 0`, erases the active
     marker, `0x15d42`, and `0x15d1e` (`0x3bd3e` 2×2), turns off `0x48`, and joins
     `0x3e372` (`country_select_exit`), filling the 15 opponent pointer arrays for
     China (`1`: `0x19930 + 0xa0 * n` at `0x3e416`) or England (`3`: `0x19980 + 0xa0 * n`
     at `0x3e50e`). Verified against `sf-select4-01` and `sf-select4-02` (`C-REF`,
     `C-NAT`, `C-NEG`).

## Not covered

- Killing an occupied slot 2 at `0x48232` when a coin is inserted during the
  attract demo (`F66`; during the title wait slot 2 is empty).
