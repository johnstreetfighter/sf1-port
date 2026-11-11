# Pre-fight screens of the fixed fight from country select's exit

This document records the boundary established by
issue #44 (`F4` of the
arcade-flow batch): the pre-fight sequence of `sf-fixed-fight-01` from country
select's exit at checkpoint `1865` through the round-1 fight-start checkpoint at
`2389`, and continuing seamlessly into the translated round-1 and round-2 fight
loop through frame `5014` (`DF at 1865, then the live CPU`). Translated in
`crates/sf-engine/src/tasks.rs`.

## Reference capture (`sf-prefight-01`) and the 1900-start probe

- **Scenario:** `tools/reference/scenarios/sf_prefight_01.json` (`sf-prefight-01`:
  `sf-fixed-fight-01`'s inputs and factory-default DIPs (`dsw1 = 0xdfff`,
  `dsw2 = 0xffff`), frames `0–2400` (`duration_frames = 2401`), full checkpoints
  on every frame `1860–2400`).
- **Determinism (`C-REF`):** two independent MAME 0.289 runs
  (`prefight-01-run-a`, `prefight-01-run-b`, local-only) match byte-for-byte
  across all 2,708 files.
- **Baseline equality:** `frames.tsv` rows `0..=2399` and checkpoints `1860`,
  `1980`, `2160`, `2280` are byte-for-byte identical to `fight-run-a`'s;
  checkpoint `2389` is byte-for-byte identical to `cpu-01-run-a`'s.
- **Frame-1900 `start1` press inside event 3:** country select (`0x3e04c–0x3e596`,
  `F34`) times out and exits at frame `1865` (`0x3e18c–0x3e1e0` / `0x3e372–0x3e389` /
  `0x3e586–0x3e596` spawns slot 2 at `0x3f7b0` and sets slot 3's
  `0xff8f24 = 0x3f41e`). During frames `1866–1957` slot 2 runs event 3
  (`0x3f8e4–0x3f97a`) while slot 3 (`0x3f41e`) watches P2 start (`SYSTEM` bit 1)
  in 1P mode (`0xff8002 == 2`) and never reads P1 start (`SYSTEM` bit 0). A
  probe capture recorded with the `frame: 1900` `start1` press removed
  (`prefight-no1900-probe`) is byte-for-byte identical to `prefight-01-run-a`
  across all 2,708 files.
- **Evidence label:** `DF at 1865, then the live CPU`.

## Checkpoint 1865 task state

At the end of frame `1865`, country select has just exited via `0x3e372–0x3e389`
and `0x3e586–0x3e596` (which queued sound `0x3f`, turned off `tx + bg + fg`,
cleared the text layer, and spawned slot 2 at `0x3f7b0` so slot 2 ran
`0x3f7b0–0x3f7d0` in frame `1865` and went to sleep at `0x3f7d6`).
`Scheduler::from_fixture` decodes all five active slots:

| Slot | Task | Resume address | Extra state |
| ---: | --- | --- | --- |
| 0 | Round timer and score tallies (`0x3f4e2`) | `0x3f7ac` | — |
| 1 | Game flow: stage start (`0x3e59c`) | `0x3e59c` | `0xff8002 = 2`, `0xff8003 = 0xff`, `0xffc003 = 0`, `0xffc004 = 0`, `0xffc006 = 0` |
| 2 | Screen events (`0x3f7b4`) | `0x3f7d6` | `0xff8004 = 1`, `0xff8e36 = 0` |
| 3 | Challenger watch (`0x3f388`) | `0x3f3a2` | `D2 = 0x0002`, `0xff8f24 = 0x3f41e` |
| 7 | Random state and coins (`0x4813e`) | `0x4819e` | — |

## Exact pre-fight timeline (frames 1865–2389)

| Frames | Slot 1 resume | Slot 2 resume | `gfxctrl` | `fg_scroll` | `bg_scroll` | Latch | Event |
| ---: | --- | --- | --- | --- | --- | --- | --- |
| 1865 | `0x3e59c` | `0x3f7d6` | `0x80` | `0x0900` | `0x6a80` | `0xff` (`0x3f` queued) | Country select exits (`0x3e372–0x3e389`, `0x3e586–0x3e596`); slot 2 spawned and sleeping at `0x3f7d6` |
| 1866 (1) | `0x422e0` | `0x436e6` | `0xc8` | `0x6600` | `0x6a80` | `0x3f` at 1866 | Slot 1 (`0x3e59c–0x3e5c2`) clears object regions (`0x3b9bc`/`0x3b9c8`/`0x3b9d4`), checks `0xff8003 != 0`, requests event 2 (`0x422d0`: `0xff8e36 = 2`, `0xff8004 = 0x21`); slot 2 (`0x3f808`) sees `0xffc003 == 0`, falls through to event 3 (`0x3f8e4–0x3f936`), sets `fg_scroll = 0x6600`, enables `fg + tx` (`0x48`), and requests country palette `0x1bf68 -> 0x200` (`0x436b8`) |
| 1867–1926 (60) | `0x422e0` | `0x3f95e` | `0xc8` | `0x6600` | `0x6a80` | `0x8f` at 1868 | Vblank 1867 uploads palette `0x1bf68 -> 0x200`; slot 2 (`0x3f93c–0x3f958`) queues country voice `0x8f` (`0x3f992[0]`, popped by vblank 1868), draws Japan text block `0x15c82` (`0x3f97e[0]`), and sleeps 60 frames (`0x3b562`) |
| 1927–1956 (30) | `0x422e0` | `0x3f964` | `0xc8` | `0x6600` | `0x6a80` | `0xff` | Slot 2 (`0x3f95e`) sleeps 30 more frames (`0x3b552`) |
| 1957 (1) | `0x422e0` | `0x3f7d6` | `0xc8` | `0x6600` | `0x6a80` | `0xff` | Slot 2 (`0x3f964–0x3f97a`) erases `0x15c82`, clears bit `0x20` of `0xff8004` (`0x01`), and returns to `0x3f7d6` |
| 1958 (1) | `0x422e0` | `0x43776` | `0xa0` | `0x6600` | `0x0000` | `0xff` (`0x33` queued) | Slot 1 wakes from `0x422e0`, runs player placement `0x3e5d6–0x3e6c6` (`place_players`) and `0x3e6ca–0x3e6fa` (`next_opponent`: clears fighter regions, checks `0xff8003 != 0`, requests event 4); slot 2 (`0x3f9ec` / `0x3fa56` / `0x3fae6`) queues sound `0x33`, turns off `fg + tx` (`0x48`), scrolls `bg` to `0x0000` (`0x3fa76[0]`), turns on `bg` (`0x20`), and requests stage palette `0x18000 -> 0x000` (`0x43748`). Note: `sf_fixed_fight_01.json`'s checkpoint `1980` is inside this 150-frame stage intro (event 4), not a VS screen |
| 1959–2078 (120) | `0x422e0` | `0x3fb2e` | `0xa0` | `0x6600` | `0x0000` | `0x33` at 1959 | Vblank 1959 uploads stage palette `0x18000 -> 0x000` and pops sound `0x33`; slot 2 (`0x3fb28`) holds 120 frames |
| 2079–2108 (30) | `0x422e0` | `0x3fb34` | `0xa0` | `0x6600` | `0x0000` | `0xff` | Slot 2 (`0x3fb2e`) holds 30 more frames |
| 2109 (1) | `0x422e0` | `0x3f7d6` | `0x88` | `0x6600` | `0x0000` | `0xff` | Slot 2 (`0x3fb34`) turns off `bg + fg` (`0x60`), turns on `tx` (`0x08`), and clears bit `0x20` of `0xff8004` |
| 2110–2115 (6) | `0x422e0` | `0x3fbc2` | `0x80` | `0x00c0` | `0x0180` | `0xff` | Slot 1 (`0x3e6ea–0x3e6fa`) runs `stage_reset` (`0x437d8`: `fg_scroll = 0x00c0`, `bg_scroll = 0x0180`, camera `0xc0`) and requests event 5; slot 2 (`0x3fba2`) turns off `bg + fg`, fills text RAM with `0x004a`, and sleeps 6 frames |
| 2116–2175 (60) | `0x422e0` | `0x3fbf8` | `0x80` | `0x00c0` | `0x0180` | `0x97` at 2117, `0x0b` at 2118 | Slot 2 (`0x3fbc2`) draws `ROUND 1` (`0x154d0`), queues `0x97` and `0x0b`, and holds 60 frames |
| 2176–2273 (98) | `0x422e0` | `0x3fc28` | `0x80` | `0x00c0` | `0x0180` | `0xff` | Slot 2 (`0x3fbf8–0x3fc28`) wipes 49 columns (`0x170` down to `0x110`) with blank tile `0x003c`, 2 frames per column |
| 2274–2278 (5) | `0x422e0` | `0x3fc48` / `0x3f7d6` | `0xe0` | `0x00c0` | `0x0180` | `0x27` at 2275, `0x3f` at 2277 | Slot 2 (`0x3fc2e–0x3fc58`) turns on `bg + fg` (`0x60`), queues `0x27` (2274) and `0x3f` (2276), and clears bit `0x20` of `0xff8004` at 2278 |
| 2279 (1) | `0x3ea30` | `0x3f7d6` | `0xe0` | `0x00c0` | `0x0180` | `0xff` (`0x28` queued) | Slot 1 wakes from `0x422e0`, sets up CPU Retsu (`0x44860`/`0x44880`), draws the round-1 HUD and places Ryu and Retsu (`0x3e714–0x3e95a`), queues stage music `0x28` (`0x42332`), runs P1's input handler and `cpu_decide` once (`0x3e960–0x3e9b8`), and enters the walk-in loop `0x3e9bc–0x3ea30` (`Flow::Intro`) |
| 2280–2359 (80) | `0x3ea30` | `0x3f7d6` | `0xe0` | `0x00c0` | `0x0180`..`0x0190` | `0x28` at 2280 | Walk-in loop (`Flow::Intro`): stage drift and fighter dispatch until both fighters finish move 0 |
| 2360–2388 (29) | `0x3ea0e` | `0x3f7d6` | `0xe0` | `0x00c0` | `0x0190` | `0x94` at 2361 | Frame 2360 draws `FIGHT!` (`0x14f44`), queues `0x94`, and holds 29 frames (`Flow::FightShown`) |
| 2389 | `0x3eafa` | `0x3f7d6` | `0xe0` | `0x00c0` | `0x0190` | `0xff` | Frame 2388 erases `FIGHT!`, sets timer reload flag (`0xff8e16 |= 1`), and yields to `Flow::RoundStart` (`0x3ea3e`); frame 2389 reloads timer to `100`, sets `0xff8004 = 0x80`, draws `TIME`, and runs the first fight frame (`0x3ea3e–0x3eafa`), matching `cpu-01-run-a`'s checkpoint `2389` |

## Player placement (`0x3e5d6–0x3e6c6`)

`place_players` (`0x3e5d6–0x3e6c6`) sets each active player structure's HUD and
entity offsets and writes the two fighter entities' structure pointers
(`0xffe438`/`0xffe43c` and `0xffea38`/`0xffea3c`):

| Mode (`0xff8002`) | Active structures | `+0x0a` (health bar) | `+0x0c` (round markers) | `+0x0e` (opponent entity) | `+0x10` (own entity) | `+0x20` (score cell) | Status |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | --- |
| `2` (1P) | P1 `0xff86b0`, CPU `0xffc664` | `0x2bc` / `0x3bc` | `0x298` / `0x2e0` | `0x0a00` / `0x0400` | `0x0400` / `0x0a00` | `0x19a` / — | Exercised in `F4` (#44, frame `1958`) |
| `4` (2P single) | P2 `0xff86da`, CPU `0xffc664` | `0x2bc` / `0x3bc` | `0x2e0` / `0x298` | `0x0a00` / `0x0400` | `0x0400` / `0x0a00` | `0x1dc` / — | Translated in `F4`, labelled unexercised (`F40`) |
| `1` / default (versus / demo) | P1 `0xff86b0`, P2 `0xff86da` | `0x2bc` / `0x3bc` | `0x298` / `0x2e0` | `0x0a00` / `0x0400` | `0x0400` / `0x0a00` | `0x19a` / `0x1dc` | Exercised in `F46` (#86, frame `1998` of `sf-start-2p-01`) |

**The round-win marker slots.** The round HUD (`round_hud`,
`0x3e78c..0x3e81c`) draws the text blocks `0x14ef0` and `0x14f06` in every
mode: at `0x298` and `0x2e0` (text row 5, columns 12 and 48), two rows of four
cells, attribute `0x2000`, tiles `0x102 0x103 0x102 0x103` over `0x112 0x113
0x112 0x113`. These are two empty 2 x 2 slots per side, shown from round 1,
before anyone has won. `round_markers` (`0x3bed2`) then paints one filled
marker (`0x1100..`) over slot *k* for each win in `+0x08`, from the
structure's `+0x0c`. So the two blocks in each top corner of a round-1 frame
are the ROM's empty slots, in Arcade, Local and Training alike, not wins
carried over and not leftovers of the hidden drive (ticket #240: at the first
fight loop of a Training CPU, Training dummy, Local and Arcade fight, `+0x08`
is 0 in every structure and those cells hold `0x2102 0x2103 0x2102 0x2103`).
Training hides them with the rest of its round furniture.

## Native verification (`C-NAT`, `C-STOP`, `C-NEG`)

Verified in `crates/sf-engine/tests/task_scheduler.rs` and
`crates/sf-engine/tests/prefight_from_1865.rs`:

1. **Fixture decoder:** `Scheduler::from_fixture` decodes checkpoint `1865` to
   `[(0, 0x3f7ac), (1, 0x3e59c), (2, 0x3f7d6), (3, 0x3f3a2), (7, 0x4819e)]`.
2. **Full checkpoint-window comparison (`1866..=2389`):** native execution from
   checkpoint `1865` matches `sf-prefight-01` on all 524 frames across
   `screen_hash`, 384×224 checkpoint pixels, video RAM, palette RAM, sound
   latch (`0x3f` at `1866`, `0x8f` at `1868`, `0x33` at `1959`, `0x97`/`0x0b`
   at `2117`/`2118`, `0x27`/`0x3f` at `2275`/`2277`, `0x28` at `2280`, `0x94`
   at `2361`), `CPU_RAM_REGIONS` (including `0xff8e36`) + slot state/sleep
   bytes, and every work-RAM byte outside the unowned stack/slot-SR-PC-USP
   exception.
3. **Seamless continuation into the fight (`2389..=5014`):** continuing the same
   native run from `1865` with `sf-fixed-fight-01`'s inputs matches
   `sf-fixed-fight-audit-01` on every owned RAM byte from frame `2389` through
   frame `5014` and stops at frame `5015` reporting `0x3eefa` (the CPU-win
   continue screen, `F5`).
4. **Stop boundaries and demo branches (`C-STOP` / `C-NAT`):**
   - `0xff8003 == 0` at `0x3e59c` executes the demo country-palette branch
     `0x3e5c6..0x3e5d0` (issue #88,
     `F48`; see [attract.md](attract.md)).
   - `0xffc003 != 0` in event 2 (`0x3f808`) executes the between-country
     travel map `0x3f812..0x3f8a4` and plane stream helpers `0x432c4..0x43344`
     (issue #83, `F43`; see
     [match-win.md](match-win.md)).
   - `0xff8003 == 0` at `0x3e6ca` executes the demo stage-palette branch
     `0x3e6fe..0x3e70e` (issue #88,
     `F48`; see [attract.md](attract.md)).
5. **Negative control (`C-NEG`):** shortening event 3's 60-frame hold (`0x3b562`
   at `0x3f958`) by 1 frame (`59` instead of `60`) fails `compare_cpu_ram` at
   frame `1867` on slot 2's sleep byte (`0xff8027`) and fails
   `sf_compare::compare` at frame `1956` when the country text block is erased
   one frame early.

## Not covered

- Exercising `place_players` in mode 4 (`F40`, covered in [versus-carry.md](versus-carry.md)).
