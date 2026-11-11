# Ryu wins a round and the match

This document records the boundary established by
issue #31: a player's
round win with its result text and bonus tally, the match win, the
match-won screen, and the match end into the next opponent's setup, all
played natively from a pinned input script until the next opponent's first
untranslated routine. Code: `crates/sf-engine/src/tasks.rs` (the screen
events and the game flow) and the vblank's palette load. See
round-result.md for the KO path that leads here and round-transition.md
for the next-round sequence it shares.

## The winning script

**sf-win-01**: 1P Ryu vs CPU Retsu on the sf-cpu-01 boot path, P1 played
from the scenario's inputs (retsu-reactions.md): from 2440 Ryu walks in for
20 frames and throws a heavy Shoryuken, every 60 frames, for the whole run.
The Shoryuken is unblockable and does 20; Retsu is knocked out in round 1 at
3018 and in round 2 at 4537. No RAM is edited. The script keeps going into
the next fight, where Ryu's attacks land on Geki.

## The round result for a human winner (event 7, 0x3fcf4 / 0x3fd12)

The screen-event task's result (0x3fcd4) branches on the winner recorded
at 0xff8e38: the CPU's branch was translated by ticket #28; a human
winner's goes:

1. **Sound 0x35**, and the round's result text typed (0x3bb98, 3 frames a
   cell) from the P1 table 0x3fd52 (P2: 0x3fd66) by round.
2. **A 30-frame hold** (0x3fd8e), the text erased (0x3bd00) and the bonus
   block 0x1561c shown.
3. **The numbers** (0x3fda8..): the frozen TIME copy (0xff8e3c, three
   digits kept by the KO path) is drawn at 0x80063e when it is not zero;
   the winner's vitality — health cells × 4 + the remainder (0x3c0b0) —
   becomes decimal digits at 0xff8e40 (0x3bf5a, below) and is drawn at
   0x80073e; the five bonus digits at 0xff8e44 are cleared; TIME and
   vitality are drawn again; a 30-frame wait.
4. **The time tally** (0x3fdf8): while the TIME copy is not zero, each
   tick adds the seven digits before 0x3fef1 (10 points) to the winner's
   score (0x3ba0e) and the four before it (10) to the bonus (0x3ba12),
   counts the TIME copy down by one (0x3bb48), redraws TIME, the bonus
   (0x800846), the score and the top score, queues sound 0xa and sleeps 3
   frames.
5. **The vitality tally** (0x3fe64): the same per tick with the vitality
   counted down, while its digits are not all zero (0x3bada).
6. **Sound 0x3f**, a 30-frame wait, and the event is done (0x3fed0).

**The vitality digits (0x3bf5a).** The binary value is converted nibble by
nibble through the table 0x3bfbc (four rows: a digit count, five digits, a
shift and a mask): each row's digits (1, 16, 256, 4096) are added once per
unit of the masked, shifted nibble. The low nibble's row has no shift and
skips the count's decrement, so it adds one time more than its nibble; a
nibble of zero skips its row. So 40 health points (0x28) tally as 41 and
48 (0x30) as 48 — the reference confirms both. The digits are drawn
through 0x3bde6, which blanks leading zeros (tile 0x3c) and always draws
the last digit; the bonus's five digits go through 0x3bdf2 unblanked.

## The match end (0x3ee26) and the next opponent

After the result, the game-flow task counts the winner's second win as
the match (0xffc009 = 1 for P1). By game mode:

- **A player's match win in a one-player mode** (0x3ef54): sound 0x37;
  the match-won screen (event 0xb) and the wipe (event 6) are requested in
  turn; sound 0x3f; the round counter and the winner cleared, the fight
  counter up (0xffc004). A country's second fight won enters the country-end
  dispatch (0x3ef96), covered by [geki](geki.md) (#67); the selected bonus
  stage remains a boundary.
- Then **the next opponent** (0x3e6ca): the fighter object regions are
  cleared; the stage intro (event 4) is requested; the stage position is
  reset (0x437d8) and the round announcement (event 5) requested; after
  it, the new opponent's move table and AI tables are set up (0x44860 /
  0x44880, ai-tables.md) and both fighters' wins cleared; then the HUD,
  the music, the walk-in and FIGHT! as for any round (round-transition.md).
- **The CPU's match win** (0x3eefa / 0x3ef2c: the lose screen, wipe,
  ranking check 0x42422 and continue screen) was reported here and is now
  translated by ticket #45 ([cpu-match-win](cpu-match-win.md)). **The
  versus match end** (0x3ee3a) is translated by ticket #30
  (round-outcomes.md); its ranking check and continuation against the CPU
  are covered by [versus-carry](versus-carry.md) (#66).

**Event 0xb (0x4001c), the match-won screen.** The bonus block is erased
and the match text shown for 60 frames (mode 2: table 0x4010c by the
winner; mode 4: 0x40140; versus: 0x40064). Then the objects are cleared,
the sprites switched off and the text layer cleared; a human winner's card
(text 0x158e4 with sound 0x9e; the last opponent's 0x159b2 with 0xba) is
shown over the background scrolled to the country table's word for the
beaten opponent (0x4018a; Japan's Retsu: 0x1b80) for 180 + 30 frames. The
CPU's card (0x40240: text 0x15848, sound 0x9d, table 0x40250, 240 frames)
is translated here and exercised in [cpu-match-win](cpu-match-win.md)
(ticket #45).

**Event 4 (0x3f9ec), the stage intro.** In the one-player modes: sound
0x33; the text and sprite layers off; the background scrolled to the
country table's word for the new opponent (0x3fa66 / 0x3fad6; Geki:
0x1e80); the background on; the stage palette requested (0x43748: 0x43778
by country and stage → 0x1b068 for Japan) and the fight loop's sleep
taken; 120 + 30 frames; then the scroll layers off and the text layer on
(0x3fb34). The versus shape (0x3fa00: background 0x780, palette 0x1ae68,
the stage palette after the holds) is translated but not exercised.

**The vblank's palette load (0x3b362).** A request (0xff8633) copies 0x200
bytes from the ROM pointer 0xff8634 into the palette at the offset
0xff8638 and clears the flag; the bus now carries the palette RAM. The
original's repeat while the system port's bit 6 is low never happens with
the fixtures' idle port and is reported.

## Verification

`crates/sf-engine/tests/match_win.rs`, on sf-win-01 (checkpoints every
frame over 2389–6300, two reference runs identical):

| Frames | What happens |
| --- | --- |
| 2390–3018 | round 1: Retsu knocked out by the third Shoryuken |
| 3072–3537 | the result: text, bonus block, 67 time ticks and 41 vitality ticks (score 1500 → 2580, bonus digits 10800) |
| 3538–3805 | the wipe and ROUND 2 |
| 3917–4537 | round 2: Retsu knocked out |
| 4572–5058 | the result: 67 + 48 ticks (score 4080 → 5230, bonus 11500) |
| 5059–5328 | the match-won screen: the text, then the card over the scrolled background |
| 5330–5427 | the wipe; fight 1 (Geki) |
| 5429–5579 | Geki's stage intro, the stage palette loaded at 5430 |
| 5581–5748 | ROUND 1; Geki's tables set up at 5749 |
| 5750–5860 | the walk-in and FIGHT! |
| 5861–6251 | the Geki fight: his jumps, his attacks, Ryu's Shoryukens landing |
| 6252 | Historical #31 boundary: Geki's attack lands on Ryu and hit function 0x4656e is reported; its translation and continuation are covered by [geki](geki.md) (#67) |

- **Observables:** every one matches on 2390–6251 — per-frame screen
  hashes, checkpoint pixels and video/palette RAM on every frame (the
  result texts, the tally digits, the win card, the stage intro with its
  loaded palette, Geki's sprites), and sound-latch timing.
- **Owned RAM:** byte-identical after every one of the 3862 frames. The
  compared regions now include the result's numbers (0xff8e38–0xff8e48),
  the game state bytes (0xffc002–0xffc00b: mode, stage, fight, country,
  difficulty, scene, winner, round, slow motion) and the palette request
  (0xff8633–0xff8639).
- **Earlier runs:** the CPU's match win in sf-round2-02 now stops at
  0x3eefa (the continue screen) instead of 0x3ee26, at the same frame.

## Winning-script search through Japan into stage 1 (sf-win-02, ticket #46 / F6)

`tools/reference/search_win.py` extends a pinned scenario's input prefix
fight by fight using a parametrized strategy family (`StrategySpec`), runs
candidates in the pinned MAME 0.289 build with a read-only Lua probe that
never writes emulated memory, and reads the fight outcome (`0xffc009`,
`0xffc003` / `0xffc004`, round wins, health, and slot-1 resume addresses).

**sf-win-02** (`tools/reference/scenarios/sf_win_02.json`, recorded as
`win-02-run-a` and `win-02-run-b`, 10,842 frames, factory-default DIPs
`DSW1=0xdfff`, `DSW2=0xffff`, Label: **TOOL, with a PO reference**):
continues `sf-win-01`'s 60-frame `shoryuken_hp` cycle (`walk_frames = 20`,
`move_offset = 22`) from Geki's round-1 start (`5860`) through `8420`, adds a
fixed `p1_hp` press at `9450` (held 6 frames) in Bonus 1 (`0x40d30`), and
runs through the flight (`0x3f812–0x3f8e0`), event 3 (`0x3f8e4`), stage 1's
intro and ROUND 1 into `0x1734c` (Joe). Full checkpoints are recorded on
every frame of one continuous window `5860..=10841` (4,982 checkpoints) in
two parts (`5860..=9278` for F27 / #67; `9278..=10841` for F43 / #83). Over
`0..=6219` (through Geki's round-1 start `5860` and all 360 shared checkpoints
`5860..=6219` before cycle 63's input at `6220`), `sf-win-02` is identical to
`sf-win-01` on every field and dump.

| Frames | What happens | Fixture / consumer |
| --- | --- | --- |
| 5860 | Geki's round-1 start: slot 1 at `0x3ea3e` (`0xffc003 = 0`, `0xffc004 = 1`, `0xffc00a = 0`) | **F27 (#67)** Geki round-1 fixture |
| 5861–7077 | Geki round 1: Geki knocked out at `7077` (`p1_wins = 1`, `p1_health = 22`) | F27 (#67) |
| 7778–8412 | Geki round 2 (`0x3ea3e` at `7777`): Geki knocked out at `8412` (`p1_wins = 2`, `p1_health = 48`) | F27 (#67) |
| 8908–9277 | Geki's match-won events `0xb` (`8908–9178`) and `6` (`9180–9277`, `0x3ef5e–0x3ef6c`): `0xffc009 = 1` and `0xffc004 = 1` | F27 (#67) |
| 9278 | `0x3ef96` clears `0xffc004` (`0`), `0xffc00a` (`0`), `0xffc009` (`0`) and dispatches stage 0 (`0x3efb0`) to `0x40d30`; slot 1 sleeps at `0x40dce` (resume `0x40dd4`, `0xffc004 = 0`, `0xffc008 = 0x40`) | **F27 (#67)** end / **F29 (#69)** & **F43 (#83)** bonus-start fixture |
| 9278–10028 | Bonus 1 (`0x40d30–0x41220`): fixed `p1_hp` press at `9450` (released at `9456`) takes the success branch (`0x410b6–0x41220`) | F29 (#69), F43 (#83) |
| 10029 | `0x3efc4` increments `0xffc003` from `0` to `1` (`0xffc003 = 1`) and branches (`0x3efd6`) to `0x3e59c` (event 2: flight `0x3f812–0x3f8e0` and event 3 `0x3f8e4`) | F29 (#69), F43 (#83) |
| 10029–10839 | Flight to USA (stage 1), event 3 VS screen, stage 1 intro (event 4), ROUND 1 (event 5), `0x44860 / 0x44880` for `0x1734c`, walk-in and FIGHT! | F43 (#83) |
| 10840 | `0x1734c`'s round-1 start: slot 1 at `0x3ea3e` (`0xffc003 = 1`, `0xffc004 = 0`, `0xffc00a = 0`, `0xffc008 = 0x00`) | **F28 (#68)** `0x1734c` round-1 fixture |
| 10841 | `0x1734c`'s first fight-loop frame: slot 1 at `0x3eafa` (`0xffc008 = 0x80`) | **F43 (#83)** window end |

## Related continuations and boundaries

- **Geki and the country-end dispatch** are translated and compared in
  [geki](geki.md) (#67).
- **The Japan bonus stage (`0x40d30–0x412a4`) and stage step (`0x3efc4`)** are
  translated and compared in `crates/sf-engine/tests/bonus_japan.rs` (ticket
  #69 / F29, Label: **DF**):
  - **Setup (`0x40d30–0x40dce`) and resume point (`0x40dd4`):** sets
    `0xffc008 = 0x40`, selects the active player structure pointer (`0xff86b0`
    in mode 2 at `0x40d40`; `0xff86da` in mode 4 at `0x40d4e`) into `0xff8f14`,
    clears the text layer (`0x3bdb0`), draws blocks `0x14786`, `0x147a2`, and
    `0x147c0` (`0x3bba0`), draws empty gauge index `0` (`0x4147e`), clears all
    object slots (`0x3b9bc / 0x3b9c8 / 0x3b9d4`), places the 2x4 plank sprite
    grid from `0x412aa` (`0x4141a`), scrolls fg to `0x0d80` and bg to `0x0c00`,
    requests palette `0x1ae68` at offset `0`, and sleeps 1 frame into `0x40dd4`.
    On waking at `0x40dd4`, `0x414f0` copies 32 palette bytes (`0x41518` in
    mode 2, `0x41538` otherwise) to palette RAM `0x3e0..0x400`, enables layers
    `0xe0` (`0x3b644`), and blinks `PUSH PUNCH BUTTON` (`0x147d2`) five times
    (12 frames on, 12 frames off, `0x40df6–0x40e1c`), then clears `0xff8f0e`,
    arms countdown timer 3 (`0xff8e16 = 0x10`), clears `*(0xff8f14)`, sets
    `0xffc008 |= 0x80`, scrolls bg to `0x0d80`, resets button latches
    (`0x3b6c6`), and sleeps 1 frame into `0x40e56`.
  - **Power-meter loop (`0x40e56–0x4103c`, resume `0x40f14`):** queues tune
    `0x3a`, clears `0xff8f0f`, and enters `0x40e6e`. Each frame draws timer-3
    digits (`0xff8e30` at `0x800492`, `0xff8e32` at `0x800498`), steps the
    3-digit rolling power display at `0xff8f11..0xff8f14` (wrapping `999` to
    `000` via `0x3b96c` `Bus::clear_bytes_3` or adding `111` from `0x40f1b` via
    `0x3ba12`) and draws it at `0x800712`, checks for timer-3 expiry
    (`0xff8e35 != 0`) or a latched punch (`*(0xff8f14) != 0`), and every 4
    frames (`0xff8f0f = 3..0`) steps the gauge (`0x0c` down to `0x00`, updating
    `0xff8f0e` through phases `0, 1, 2` and scrolling fg through `0x0d80,
    0x0f00, 0x1080`; when gauge `0` is reached at `0x41004`, jumps to `0x40f20`
    or `0x40f68` via `0x4101e[((0xff8e4b & 0x0f) >> 1) << 2]`).
  - **Strike resolution (`0x4103e–0x41158`, helper `0x41558–0x4159c`), success
    (`0x410b6–0x41220`) and failure (`0x41226–0x412a4`):** `0x41558` clears
    `+0x34` on the fighter entity, maps `*(0xff8f14)` through `0x4159e` via
    `0x40b72` (`Bus::pick_by_strength`, storing `1..=10` in `+0x34` or leaving
    `0` when timer 3 expired with no punch), and draws the final 3-digit power
    from `0x415b4[d2]` (indexed by `((0xff8e4f & 0x0f) >> 1) << 2`) at
    `0x800712`. After voice `0x8c` and the wind-up scroll stream
    (`0x4107a[0xff8f0e]`, `0x41054–0x41078`), `0x4108a` compares `+0x34` against
    `0x410d0[0xff8f0e]` (`0x410dc`, `0x410e5`, `0x410ee`):
    - **Success (`sf-win-02`, `9277..=10028`):** `p1_hp` at `9450` in phase 2
      resolves `d3 = 10` (`tier d2 = 1`, `1,000` points from `0x41402`, crash
      sound `0x12`, 4-step plank break `0x412f8` through `0x41468`), queues
      voice `0x8e`, scrolls fg to `0x1380`, runs 15 bg-scroll cheer cycles
      (`0x41184–0x411ca`, sound `0xa0`), draws `BONUS PTS.` (`0x147e2`) and the
      4 bonus digits (`0xff8f1c` at `0x8008c4`), adds the 7-digit bonus onto
      the player's score (`0x3ba0e`) and top score (`0x3ba84`), holds `120 +
      120` frames (`0x41206–0x41210`), queues `0x3f`, clears `0xffc008`, and
      enters `0x3efc4` (`stage_step`) on frame `10029` (`0xffc003: 0 -> 1`,
      jumping to `0x3e59c` and starting event 2's travel map `0x3f812` in slot 2).
    - **Failure (`sf-bonus-01`, `9277..=10179`):** gauge reaches `0` at `9553`
      (`0x40fea–0x4101c` resets into `0x40f20`), timer 3 expires at `9721` in
      phase 0 with `d3 = 0 < 1` (`0x41570: clr.w D2` and `0x41072–0x41078`
      4-frame wind-up sleeps), and `0x41226–0x412a4` queues thud `0x10` (90
      frames), voice `0x8d` with fg scroll `0x1500` (60 frames), and three
      60/60/180-frame laugh cycles (`0x9f` with bg scroll `0x1200`), then
      queues `0x3f`, clears `0xffc008`, and enters `0x3efc4` on frame `10180`
      (starting event 2's travel map `0x3f812` in slot 2).
  - **Game-mode branches in `0x40d30` and `0x414f0`:**
    - `0x40d36` tests `0xffc002` against `4`: Ryu's one-player chain runs in
      mode 2, so the mode-4 branch (`0x40d4e: move.l #$ff86da, ($f14,A5)`) is
      labelled **unexercised** in the mode-2 capture chain (exercised directly
      in unit/integration tests; Ken's mode-4 chain is open decision 10).
    - `0x414f6` (in `0x414f0`, called from `0x40dd4`) tests `0xffc002` against
      `2`: the non-mode-2 branch (`bne $41508`, loading palette tail `0x41538`)
      is likewise labelled **unexercised** in Ryu's mode-2 capture chain.
- **The between-country travel map (`0x3f812–0x3f8e0` and plane stream helpers
  `0x432c4–0x43344`)** is translated and compared in
  `crates/sf-engine/tests/travel_map.rs` (ticket #83 / F43, Label: **DF**):
  - **Event 2 setup (`0x3f812–0x3f87a`, resume `0x3f87c`):** when
    `0xffc003 != 0`, slot 2 indexes the starting-country route table at
    `0x3f822[0xffc005]` (`0x3f8a4` for Japan, `0x3f8b0` for USA, `0x3f8bc` for
    China, `0x3f8c8` for England) and selects the route entry
    `table + (0xffc003 - 1) * 4`, clears the text layer (`0x3bdb0`), scrolls fg
    to `0x6600` and bg to `0x1f80`, sets `gfxctrl = 0x80 | 0x60` (`tx + bg +
    fg`), requests country palette `0x1bf68 -> 0x200` and map palette `0x1b468
    -> 0x000`, draws text block `0x154f2`, clears all object regions (`0x3b9bc /
    0x3b9c8 / 0x3b9d4`), and sleeps 1 frame into `0x3f87c` with `A1` saved on
    its stack.
  - **Plane spawn and flight loop (`0x3f87c–0x3f8a4` and `0x432c4–0x43344`):**
    on waking at `0x3f87c`, slot 2 spawns two effect objects (`0x43072`) in
    effect slots 0 and 1 (`0xffe100` and `0xffe140`) from the two 12-byte
    specs at `A1` (`0x44f46` / `0x44f52` for Japan -> USA) and loops at
    `0x3f890` calling `0x4304c` (`run_effects`) each frame until effect slot 0's
    state byte (`0xffe100`) becomes `0` (`0x3f89a`). The plane's `0xff` stream
    dispatches `0x432c4` (`plane_loop_start`: latches loop count at `+0x28` and
    loop target at `+0x2a`), `0x432e0` (`plane_loop_next`: steps `(x, y)` by a
    signed `(dx, dy)` byte pair each frame until `+0x28` reaches `0`), `0x43306`
    (`plane_flip`: toggles sprite attribute bit `0x02` at `+0x03`), and
    `0x4331a` (`plane_sound`: queues sound command from the stream byte, e.g.
    `0x1f` at frame `10030` and `0x3f` at frame `10343`). When effect slot 0
    terminates at frame `10343`, slot 2 sleeps 30 frames (`0x3b552`) into
    `0x3f8a0`, clears the text layer (`0x3bdb0`), and falls through into event 3
    (`0x3f8e4`: country VS screen, `10373..=10464`), event 4 (stage-1 intro,
    `10465..=10616`), and event 5 (`ROUND 1`, `10617..=10839`), reaching Joe's
    round-1 start (`0x3ea3e` at `10840`) and first fight-loop frame (`0x3eafa`
    at `10841`) with `0xffc676 == 0x1734c`.
  - **Route-table coverage:** `sf-win-02` exercises Japan's stage-1 entry in
    `0x3f8a4`; stages 2–4 of `0x3f8a4` and the USA (`0x3f8b0`), China
    (`0x3f8bc`), and England (`0x3f8c8`) route tables are verified across all
    16 `(country, stage)` combinations in `travel_map.rs`.
- **Joe's defeat and Mike's setup** are translated and compared in
  [joe](joe.md) (#68), ending at Mike's first new hit function.
- **Mike's defeat and the stage-1 dispatch** are translated and compared in
  [mike](mike.md) (#84).
- **Lee's defeat and Gen's setup** are translated and compared in
  [lee](lee.md) (#125), **Gen's defeat and the stage-2 dispatch** in
  [gen](gen.md) (#129), **Birdie's defeat and Eagle's setup** in [birdie](birdie.md)
  (#131), **Eagle's defeat and the stage-3 dispatch** in [eagle](eagle.md) (#132),
  **Adon's defeat and Sagat's setup** in [adon](adon.md) (#134), and **Sagat's
  defeat and the stage-4 dispatch to the ending (`0x44a40`)** in
  [sagat](sagat.md) (#135).
- **The stage-1 10-second bonus (`0x41740–0x419fc`) and stage-2 arrival** are
  translated and compared in `crates/sf-engine/tests/bonus_stage1.rs` (ticket
  #124 / F52, Label: **DF**):
  - **Setup (`0x41740–0x41812`, resume `0x41814` and `0x4182e`) and blink
    prompt (`0x4185c / 0x4186e`):** clears the text layer (`0x3bdb0`) and all
    three object regions (`0x3b9bc..0x3b9d4`), sets `0xffc008 = 0x40`, places
    the active player at `x = 0x100` facing right (`+0x37 = 2`, stance move
    `mv = 1` via `0x44174` for P1 `0xff86b0` in mode 2; `0x44192` for P2
    `0xff86da` in mode 4, mode 4 unexercised), clears `*(ps)`, spawns the three
    concrete block stack effects (`0x18e24` in slot `0x100`, `0x18e44` in slot
    `0x140`, `0x18e64` in slot `0x180`) with `owner = rw(ps + 0x0e)` via
    `0x42f1e` (`Bus::spawn_effect_absolute`, running `0x42d22` after each
    spawn), runs the object dispatcher (`0x429f2`), draws text blocks `0x14832`
    (`TIME`) and `0x1484e` (`1000`), scrolls fg to `0x1680` and bg to `0x1380`,
    requests palette `0x1ae68 -> 0x000` (1-frame sleep into `0x41814`), then
    requests palette `0x1a068 -> 0x200` (1-frame sleep into `0x4182e`), turns
    on layers `0xe8`, queues tune `0x3a`, and blinks `10 SECONDS BONUS`
    (`0x14802`) five times (12 frames on at `0x4185c`, 12 frames off at
    `0x4186e`). On the final blink (`d1 == 0`), `0x41878–0x418a8` erases
    `0x1484e`, clears `0xff8f18..0xff8f20` (`0x3b934`), sets `0xffc008 |= 0x80`,
    arms countdown timer 3 (`0xff8e16 = 0x20`), resets button latches
    (`0x3b6c6`), and sleeps 1 frame into `0x418ac`.
  - **10-second loop (`0x418ac–0x41938`, resume `0x4191c`) and stack handlers
    (`0x434e4`, `0x4352c`, `0x4357a`):** each frame runs the active player's
    input handler (`0x402f8` on `0xff86b0` in mode 2; `0x418c4` on `0xff86da`
    in mode 4, mode 4 unexercised), draws timer-3 integer and fractional digits
    (`0xff8e30` at `0x8003c0`, `0xff8e32` at `0x8003c6`, attr `0xa000`), and
    branches on timer expiry (`0xff8e35 != 0` -> `0x4193a`: draws `TIME OVER`
    `0x14f74`, queues voice `0x96`, and enters the settle loop at `0x41950`);
    otherwise runs `0x429f2` (`dispatch_order`) and `0x42d22` (`run_effects`),
    sleeps via `0x3b5aa` into `0x4191c`, and checks `0x3baf6`
    (`Bus::compare_bytes_3(0xff8f1c, 0x41936) == Equal`, `05 00 00` = `5,000`
    bonus points = all three stacks broken) to exit early into `0x41950`.
    When a player attack hits a stack (`0x43144` `effect_vs_attack` -> `+0x18`),
    `0x434e4` (right stack `0x18e24`, stream `-> 0x1943a`, `+1,000` from
    `0x435d2`), `0x4357a` (lower-left stack `0x18e64`, stream `-> 0x19472`,
    `+1,000` from `0x435e0`), or `0x4352c` (upper-left stack `0x18e44`, stream
    `-> 0x19456`, `+3,000` from `0x435d9` to `0xff8f18..0xff8f1f` and directly
    to `ps + 0x22..+0x28` via `0x43566`) clears the stack's hold/collision bytes
    (`+9 = 0`, `+0x0e = 0`), spawns debris `0x18ec4` (`0x42e1e`), queues crash
    sound `0x12`, adds the stack's points onto `0xff8f18..0xff8f1f` (`0x3ba0e`),
    and updates the player's score display (`0x3be6a`) and top score
    (`0x3ba84`).
  - **Settle (`0x41950–0x419b2`, resume `0x41962` and `0x419a0`), bonus display
    (`0x419b4–0x419ec`), and stage step (`0x419ee–0x419fc` -> `0x3efc4`):**
    waits at `0x41962` while the active player's move is still in progress
    (`+9` bit 7 set on `rw(0xff86b0 + 0x10)` in mode 2; `0x41974`
    `rw(0xff86da + 0x10)` in mode 4, mode 4 unexercised), runs 10 post-settle
    frames at `0x419a0`, sleeps 120 frames into `0x419b4`, erases `TIME OVER`
    (`0x14f74`), draws `BONUS PTS.` (`0x14812`) and the 4 bonus digits
    (`0xff8f1c` at `0x8005bc`, attr `0x8000`), sleeps 120 frames into `0x419ee`,
    queues `0x3f`, clears `0xffc008`, and enters `0x3efc4` (`stage_step`,
    `0xffc003: 1 -> 2` into China's travel map and Lee's setup `0x175bc`).
    - **Timeout path (`sf-win-04`, `17483..=19380`):** P1's 60-frame
      `shoryuken_hp` cycle through `18000` misses all three stacks; timer 3
      expires at `18244` (`TIME OVER`), `BONUS PTS. 0000` is shown at `18387`,
      `0x3efc4` advances `STAGE_NO` to `2` at `18527`, and native execution
      matches all 1,897 frames and checkpoints through Lee's round-1 start
      (`19379`, `(1, 0x3ea3e)`) and first fight-loop frame (`19380`,
      `(1, 0x3eafa)`, `0xffc676 == 0x175bc`).
    - **All-three-broken path (`sf-bonus-02`, `17483..=18110`):** P1 breaks the
      right stack (`0x18e24`, `0x434e4`) at `17667` with standing HK, the
      lower-left stack (`0x18e64`, `0x4357a`) at `17768` with crouching HP, and
      the upper-left stack (`0x18e44`, `0x4352c`) at `17833` with a forward-jump
      HP, triggering `0x3baf6`'s early exit at `17834`, `BONUS PTS. 5000` at
      `17867`, and `0x3efc4` (`STAGE_NO: 1 -> 2`) at `18107`; native execution
      matches all 627 frames and checkpoints and reaches Lee's fight loop
      (`0x3eafa`, `0xffc676 == 0x175bc`) at `18959`, stopping cleanly at
      Lee's first untranslated routine.
- **Stage 2's power/timing bonus (`0x41a02–0x41fe6`, ticket #130 / F58):**
  dispatched by `0x3ef96` when `0xffc003 == 2` (`0x3efb0 + 8 -> 0x41a02`), and
  translated as `BonusCopy::Stage2` parameters on F29's `bonus_japan_*`
  routines (`crates/sf-engine/src/tasks.rs`).
  - **Code structure and addressing difference from Copy 1 (`0x40d30–0x412a4`):**
    - `0x41a02–0x41d26` is instruction-for-instruction identical to Copy 1's
      `0x40d30–0x41054` (shifted by `+0xcd2`), with initial sleep resume point
      `0x41aa6` (calling shared helper `0x414f0` `bonus_load_palette_tail`) and
      subsequent resume points `0x41ad2`, `0x41af8`, `0x41b28`, `0x41b36`, and
      `0x41b72`, except `0x41a68: lea ($4f4,PC), A1` loads the stage-2 grid
      descriptor `0x41f5e` instead of `0x412aa` (`0x40d96: lea ($512,PC), A1`).
    - **One addressing difference (`0x41d28–0x41d2e` vs `0x41056–0x41058`):**
      Copy 1 indexes its inline 16-byte wind-up scroll table at `0x4107a..0x4108a`
      with a single PC-indexed `0x41056: lea ($22,PC,D0.w), A0`. Copy 2 omits a
      duplicate wind-up table and instead uses two instructions
      (`0x41d28: lea (-$cb0,PC), A0` -> `0x4107a`, followed by
      `0x41d2c: lea (0,A0,D0.w), A0`) to index Copy 1's shared wind-up table at
      `0x4107a`, before entering the wind-up loop `0x41d30–0x41d4e` (resume point
      `0x41d4e`, shifted by `+0xcd6` from `0x4105a–0x41078`).
    - `0x41d50–0x41de4` (`bonus_japan_judge` and inline threshold/result/sound
      tables) corresponds to `0x4108a–0x41130`, with 3 break tiers (`0..=2`)
      instead of 4 (`0..=3`).
    - `0x41de6–0x41f5c` is instruction-for-instruction identical to Copy 1's
      `0x41132–0x412a4` (shifted by `+0xcb4`), with resume points `0x41e0c`,
      `0x41e1a`, `0x41e44`, `0x41e5a`, `0x41e6e`, `0x41ec0`, `0x41ed0`,
      `0x41eee`, `0x41f14`, and `0x41f4a`, jumping to `0x3efc4` (`stage_step`,
      `0xffc003: 2 -> 3`) at `0x41f56`.
  - **Every data difference from Copy 1 (`0x40d30–0x4141a`) with addresses:**
    1. **2×4 sprite grid descriptor (`0x41f5e..0x41f74` vs `0x412aa..0x412c0`):**
       - `0x41f5e..0x41f62`: `y0 = 0x00a8`, `x0 = 0x00e0` (identical to
         `0x412aa..0x412ae`).
       - `0x41f62` (vs `0x412ae`): base attribute word `0x040f` (vs `0x0403` in
         Copy 1).
       - `0x41f64..0x41f74` (vs `0x412b0..0x412c0`): eight sprite code words
         `[0x3000, 0x3002, 0xb002, 0xb000, 0x3020, 0x3022, 0xb022, 0xb020]` (vs
         `[0x1ae0, 0x1ae2, 0x9ae2, 0x9ae0, 0x1b00, 0x1b02, 0x9b02, 0x9b00]` in
         Copy 1).
    2. **Rolling power increment constant (`0x41bed..0x41bf0` vs
       `0x40f1b..0x40f1e`):** 3-byte BCD increment `01 01 01` (`111`, identical
       bytes at `+0xcd2`).
    3. **Gauge-zero restart pointer table (`0x41cf0..0x41d10` vs
       `0x4101e..0x4103e`):** eight long pointers
       `[0x41bf2, 0x41c3a, 0x41bf2, 0x41c3a, 0x41bf2, 0x41c3a, 0x41c3a, 0x41bf2]`
       (Copy 1's `[0x40f20, 0x40f68, ...]` shifted by `+0xcd2` for Copy 2's
       meter continuation labels `0x41bf2..0x41cd0`).
    4. **Phase threshold pointer table (`0x41d96..0x41da2` vs
       `0x410d0..0x410dc`) and threshold `(min_d3, tier)` byte lists
       (`0x41da2..0x41db2` vs `0x410dc..0x410f4`):**
       - `0x41d96..0x41da2`: `[0x41da2, 0x41da9, 0x41dae]` (vs `0x410d0..0x410dc`:
         `[0x410dc, 0x410e5, 0x410ee]`).
       - Phase 0 (`0x41da2..0x41da9` vs `0x410dc..0x410e5`):
         `(4, 0), (7, 1), (10, 2), 0xff` (7 bytes; vs Copy 1's 9-byte
         `(1, 0), (4, 1), (7, 2), (10, 3), 0xff`).
       - Phase 1 (`0x41da9..0x41dae` vs `0x410e5..0x410ee`):
         `(7, 0), (10, 1), 0xff` (5 bytes; vs Copy 1's 9-byte
         `(1, 0), (4, 1), (7, 2), (10, 2), 0xff`).
       - Phase 2 (`0x41dae..0x41db2` vs `0x410ee..0x410f4`):
         `(10, 0), 0xff, 0xff` (4 bytes including trailing alignment `0xff`; vs
         Copy 1's 6-byte `(7, 0), (10, 1), 0xff, 0xff`).
    5. **Result table (`0x41dc8..0x41de0` vs `0x4110a..0x4112a`):** 3 entries
       `(anim_ptr, score_ptr)` (24 bytes) instead of Copy 1's 4 entries (32
       bytes), referencing shared score BCD rows in `0x41402..0x4141a`:
       - Tier 0 (`0x41dc8..0x41dd0`): `(0x41f74, 0x41402)` (2,000 pts; vs Copy
         1's `0x4110a`: `(0x412c0, 0x413fa)`, 1,000 pts).
       - Tier 1 (`0x41dd0..0x41dd8`): `(0x41f88, 0x4140a)` (3,000 pts; vs Copy
         1's `0x41112`: `(0x412f8, 0x41402)`, 2,000 pts).
       - Tier 2 (`0x41dd8..0x41de0`): `(0x41fae, 0x41412)` (5,000 pts; vs Copy
         1's `0x4111a`: `(0x41342, 0x4140a)`, 3,000 pts, and `0x41122`:
         `(0x4139e, 0x41412)`, 5,000 pts).
    6. **Crash sound table (`0x41de0..0x41de6` vs `0x4112a..0x41132`):** three
       words `[0x0016, 0x0017, 0x0017]` (vs Copy 1's four words
       `[0x0011, 0x0012, 0x0012, 0x0014]`).
    7. **Break animation streams (`0x41f74..0x41fe6` vs `0x412c0..0x413fa`):**
       - Tier 0 (`0x41f74..0x41f88`, `count = 0`, 1 step, sleep `8`):
         `[0x3004, 0x3006, 0x3006, 0x3004, 0x3024, 0x3026, 0x3026, 0x3024]`.
       - Tier 1 (`0x41f88..0x41fae`, `count = 1`, 2 steps, sleeps `8, 8`): step
         0 as above; step 1
         `[0x3008, 0x300a, 0x300a, 0x3008, 0x3028, 0x302a, 0x302a, 0x3028]`.
       - Tier 2 (`0x41fae..0x41fe6`, `count = 2`, 3 steps, sleeps `8, 8, 8`):
         steps 0–1 as above; step 2
         `[0x300c, 0x300e, 0x300e, 0x300c, 0x302c, 0x302e, 0x302e, 0x302c]`.
  - **Game-mode tests and unexercised branches:**
    - `0x41a08..0x41a24`: `cmpi.b #4, ($4002,A5)` (`GAME_MODE == 4` takes
      `0x41a10: beq.w $41a1e` to load P2's player structure `0xff86da` at
      `0x41a1e..0x41a24`, whereas `GAME_MODE != 4` loads P1's `0xff86b0` at
      `0x41a14..0x41a1c`). The mode-4 branch (`0x41a1e..0x41a24`) is translated
      and labelled unexercised in `sf-win-06` and `sf-bonus-03` (both run in
      mode 2).
    - `0x414f6..0x4150a` in F29's helper `0x414f0` (`bonus_load_palette_tail`,
      called from resume point `0x41aa6`): `cmpi.b #2, ($4002,A5)` (`GAME_MODE == 2`
      takes `0x414fc: beq.s $414fe` to copy `0x41518..0x41538`, whereas other
      modes take `0x414fe: bra.s $41504` to copy `0x41538..0x41558` at
      `0x41504..0x4150a`). Its other-mode branch (`0x41504..0x4150a`) is
      translated and labelled unexercised.
  - **Verification (`crates/sf-engine/tests/bonus_stage2.rs`):**
    - **Success outcome (`sf-win-06`, `24853..=26372`, `1,519` frames):**
      `p1_hp` at `25027` latches `3` before the meter loop begins (`25082`),
      resolving power `10` (`999`) in phase `0`, stepping the phase-0 wind-up
      sleeps (`0x4107a..0x41084` via `0x41d28..0x41d4e`), selecting tier `2`
      (`0x41dd8 -> anim 0x41fae, score 0x41412 = 5,000`, sound `0x0017` from
      `0x41de4`), running the 3-step break animation and 15-cycle cheer loop,
      advancing `STAGE_NO` `2 -> 3` at `25582` (`0x3efc4`), and continuing
      through the flight to England, VS screen (`RYU VS BIRDIE`), stage-3 intro, and
      `ROUND 1` into Birdie's first fight-loop frame (`26372`, `0x3eafa`,
      `0xffc676 == 0x17a9c`).
    - **Failure / timer-expiry outcome (`sf-bonus-03`, `24853..=25764` captured,
      continued natively through `26550`):** replaces the `25027` `p1_hp` press
      with a 5-frame `p1_right` hold at `25115`, exercising the gauge-zero reset
      and random-phase jump (`0x41cbc..0x41cee` into `0x41cf0`) at `25129` (and
      subsequent gauge-zero resets), timer-3 expiry at `25306` with no punch
      pressed (`0x41570: clr.w D2`), phase-1 wind-up sleeps (`0x41080` via
      `0x41d28..0x41d4e`), and the failure sequence (`0x41eda..0x41f5c`: thud
      `0x10`, fail pose + disappointment `0x90`, 15-cycle head-shake loop
      `0x41f18..0x41f40`, and `0x3efc4` at `25760`), then continues natively
      through the flight to stage 3, VS screen, stage-3 intro, and `ROUND 1`
      into Birdie's fight loop (`0x3eafa`, `0xffc676 == 0x17a9c`) at `26550`.
- **Stage 3's 10-second bonus (`0x41fe6–0x422b8`, ticket #133 / F61):**
  dispatched by `0x3ef96` when `0xffc003 == 3` (`0x3efb0 + 12 -> 0x41fe6`), and
  translated as `Bonus10Copy::Stage3` parameters on F52's `bonus_stage1_*`
  routines (`crates/sf-engine/src/tasks.rs`) and effect handlers `0x435c2` and
  `0x4360a` (`crates/sf-engine/src/fighter.rs`).
  - **Code structure and every data difference from Copy 1 (`0x41740–0x419fc`):**
    1. **Setup (`0x41fe6..0x420ca` vs `0x41740..0x41812`, sleep resume `0x420cc`
       vs `0x41814`):**
       - Spawns **four** concrete block stack templates via `0x42f1e`
         (`Bus::spawn_effect_absolute`) with `0x42d22` (`Bus::run_effects`) after
         each (`0x42028..0x42074`, 18 extra bytes = `+0x12`, shifting all code
         from `0x420cc` onward by `+0x8b8` relative to `0x41814`):
         - `0x18e44` in slot `0x100` (`0x42028..0x42038`, upper-left at `y = 0x70,
           x = 0x90`, stream `0x19448`, handler `0x4352c` -> broken stream
           `0x19456`, `+3,000` from `0x43579` to `0xff8f18..0xff8f1f` and
           directly to `ps + 0x22..+0x29` via `0x43566`).
         - `0x18e64` in slot `0x140` (`0x4203c..0x4204c`, lower-left at `y = 0xb8,
           x = 0x80`, stream `0x19464`, handler `0x4357a` -> broken stream
           `0x19472`, `+1,000` from `0x435c1` to `0xff8f18..0xff8f1f`).
         - `0x18e84` in slot `0x180` (`0x42050..0x42060`, upper-right at `y = 0x70,
           x = 0x170`, stream `0x19480`, handler `0x435c2` -> broken stream
           `0x1948e`, `+3,000` from `0x43609` to `0xff8f18..0xff8f1f`).
         - `0x18ea4` in slot `0x1c0` (`0x42064..0x42074`, lower-right at `y = 0xb8,
           x = 0x180`, stream `0x1949c`, handler `0x4360a` -> broken stream
           `0x194aa`, `+1,000` from `0x43651` to `0xff8f18..0xff8f1f`).
       - Scrolls fg to `0x1800` (`0x4209a: move.w #$1800, D1`, vs `0x1680` at
         `0x417e2` in Copy 1) and bg to `0x1380` (`0x420a4`).
    2. **Body (`0x420cc..0x422b8` vs `0x41814..0x419fc`):**
       - Every instruction in `0x420cc..0x422b4` is byte-for-byte identical to
         Copy 1's `0x41814..0x419fc` shifted by `+0x8b8` (with sleep resume
         points `0x420cc`, `0x420e6`, `0x42114`, `0x42126`, `0x42164`,
         `0x421d4`, `0x4221a`, `0x42258`, `0x4226c`, and `0x422a6`), except the
         4-byte early-exit bonus threshold table at `0x421ee..0x421f2` (vs
         `0x41936..0x4193a` in Copy 1):
         - Copy 1 (`0x41936`): `05 00 00 00` (`5,000` bonus points = all 3 stacks
           broken).
         - Copy 2 (`0x421ee`): `08 00 00 00` (`8,000` bonus points = all 4 stacks
           broken).
       - `0x422b4: jmp $3efc4.l` advances `STAGE_NO` `3 -> 4` (Thailand).
  - **Game-mode-4 tests (`0x42004`, `0x42164`, `0x4221a`):**
    - `0x42004..0x42024`: `cmpi.b #4, ($4002,A5)` (`GAME_MODE == 4` takes
      `0x4200c: beq.w $4201c` to call `0x44192` on `0xff86da`, whereas
      `GAME_MODE != 4` calls `0x44174` on `0xff86b0` at `0x42010..0x4201a`).
    - `0x42164..0x42180`: `cmpi.b #4, ($4002,A5)` (`GAME_MODE == 4` takes
      `0x4216c: beq.w $4217c` to run `0x402f8` on `0xff86da`, whereas
      `GAME_MODE != 4` runs `0x402f8` on `0xff86b0` at `0x42170..0x4217a`).
    - `0x4221a..0x42232`: `cmpi.b #4, ($4002,A5)` (`GAME_MODE == 4` takes
      `0x42222: beq.w $4222c` to select `0xff86da`, whereas `GAME_MODE != 4`
      selects `0xff86b0` at `0x42224..0x4222a`).
    - In `sf-win-08` and `sf-bonus-04` (both run in `GAME_MODE == 2`), the three
      mode-4 branches (`0x4201c..0x42024`, `0x4217c..0x42180`, `0x4222c..0x42232`)
      are translated and labelled unexercised; both outcomes of all three checks
      are verified in `bonus_stage3.rs`.
  - **Verification (`crates/sf-engine/tests/bonus_stage3.rs`):**
    - **Timeout outcome (`sf-win-08`, `32315..=34184`, `1,869` frames):** P1 is
      idle through the 10-second loop (`32426..=33076`), timer 3 expires at
      `33077` (`0x14f74` `TIME OVER`, voice `0x96`), `BONUS PTS. 0000` is drawn
      at `33219` (`0x4226c`), `0x3efc4` advances `STAGE_NO` `3 -> 4` at `33359`
      (`0x422a6 -> 0x3efc4`), and native execution matches all `1,869` frames and
      checkpoints through the flight to Thailand, VS screen (`RYU VS ADON`),
      stage-4 intro, and `ROUND 1` into Adon's first fight-loop frame (`34184`,
      `0x3eafa`, `0xffc676 == 0x17f7c`).
    - **All-four-broken outcome (`sf-bonus-04`, `32315..=33017` captured,
      continued natively through `33842`):** P1 walks right at `32427` and breaks
      the lower-right stack (`0x18ea4`, `0x4360a`, `+1,000`) at `32498` with
      crouching HP and the upper-right stack (`0x18e84`, `0x435c2`, `+3,000`) at
      `32563` with forward-jump HP, then walks left at `32590` and breaks the
      lower-left stack (`0x18e64`, `0x4357a`, `+1,000`) at `32678` with crouching
      HP and the upper-left stack (`0x18e44`, `0x4352c`, `+3,000` to bonus and P1
      score) at `32743` with forward-jump HP, triggering `0x3baf6`'s `8,000`-point
      early exit at `32744`, settle at `32767`, `BONUS PTS. 8000` at `32897`, and
      `0x3efc4` (`STAGE_NO: 3 -> 4`) at `33017`, then continues natively through
      the flight to Thailand, VS screen, stage-4 intro, and `ROUND 1` into Adon's
      fight loop (`0x3eafa`, `0xffc676 == 0x17f7c`) at `33842`.
- **Stage 4 (Thailand) ending sequence** (`0x44a40–0x44c1c`,
  [ending](ending.md), #136): dispatched by `0x3ef96` when `STAGE_NO == 4`
  (index 4 of `0x3efb0`, `0x3efc0: dc.l $00044a40`). Runs the five
  country-ordered ending scenes (`0x44a40–0x44b00`, tables `0x44c2c–0x44d5b`),
  the congratulate message (`0x44b00–0x44b88`, scroll `0x44d5c / 0x44d5e`,
  text `0x44d60–0x44f57` with `TEXT_SOUND = 0x000d`), and the staff roll and
  `THE END` (`0x44b88–0x44c1c`, pointers `0x44f58–0x44f7f`, blocks
  `0x44f80–0x452dc`), then restores boot sounds (`0x3b7a8`) and jumps to
  `0x3efc4` (`STAGE_NO: 4 -> 5`) into `0x3f2d0`'s ranking check (`0x42422`),
  name entry (`0x42486`), game over (`0x3f2f0`), and return to attract
  (`0x3f33c -> 0x3ccae`).
- **Complete one-player game from power-on** (`sf-full-1p-01`, `44,200` frames,
  `873` checkpoints, evidence class **PO**, [ending](ending.md), #137 / F65):
  extends `sf-win-10` from `43618` with three `p1_lp` presses (`43630`, `43655`,
  `43680`) that commit initials `AAA` at rank 1 (`494,300` pts) in `0x42486`,
  insert the marked rank-1 entry (`0x425c4` at `43694`), blink the marked entry for
  3 seconds (`0x4268c` / `0x42750`, `43695..=43900`), run the game-over screen
  (`0x3f2e0 -> 0x3f2f0`, `43901..=44141`), and return via `0x3f33c` (`44142`) to
  the attract loop (`0x3ccc2 / 0x3ccec`, `44142..=44199`), verified from frame `0`
  without fixture in `crates/sf-engine/tests/full_1p.rs`.
- **The CPU's match win** (the lose screen, ranking check and continue
  screen; translated by ticket #45, [cpu-match-win](cpu-match-win.md)). The
  versus winner's ranking check and continuation are translated and compared
  in [versus-carry](versus-carry.md) (#66).
- **Mode 4 and versus paths** of events 0xb and 4: translated, not
  exercised.
