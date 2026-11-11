# Ryu beats Eagle and reaches the stage-3 bonus

Issue #132 (`F60`) continues `sf-win-07` from Eagle's round-1 start (`29426`)
through his defeat, the stage-3 10-second bonus (`0x41fe6`), and into stage 4
(`0x17f7c`, Adon). This is a development-fixture continuation (**DF**): it
starts from a captured fight-loop checkpoint and follows the pinned input
script, without editing emulated RAM. The implementation is in
`crates/sf-engine/src/fighter.rs`.

**Identity.** `0x17d0c` is Eagle. In `win-07-run-a` the VS screen before
this fight reads `RYU VS EAGLE` (frame `28995`), and the fight is on the
England castle stage with `0xffc676 = 0x17d0c` in the same frame's work-RAM
dump (`29427`). Before 2026-11-02 this page was `gen.md` and called this
opponent Gen.

## Translation

Eagle's seven hit functions (`0x47914..0x47c14`) belong to the AI attack streams
selected for his move table `0x17d0c`. They use the shared CPU hit shape
(`HitFn::Cpu`), with Eagle's spark templates (`0x18964`, `0x18984`, `0x189c4`,
`0x18a04`, `0x18ac4`), damage and reaction entries. Five block against standing
block state `0x22`; `0x479f0` and `0x47ba8` block against crouching state
`0x24`.

| Hit function | Stream | Active frame(s) | Spark template | Depth-1 entry | Other-depth entry | Block state | Status in `sf-win-08` |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `0x47914` | `0x12408` | `0x1242e`, `0x1244e`, `0x1246e`, `0x1248e` | `0x18984` | `0x4797a` | `0x47972` | `0x22` | unexercised |
| `0x47982` | `0x124d8` | `0x124fe`, `0x1251e`, `0x1253e`, `0x1255e`, `0x1257e`, `0x1259e`, `0x125be`, `0x125de` | `0x18984` | `0x479e8` | `0x479e0` | `0x22` | unexercised |
| `0x479f0` | `0x1239e` | `0x123be` | `0x18964` | `0x47a56` | `0x47a4e` | `0x24` | unexercised |
| `0x47a5e` | `0x126f8` | `0x127c4` | `0x18984` | `0x47ac4` | `0x47abc` | `0x22` | landed (frames `31383`, `p1_health: 38 → 24`, and `31427`, `p1_health: 24 → 12`) |
| `0x47acc` | `0x129bc` | `0x12a88` | `0x18ac4` | `0x47b32` | `0x47b2a` | `0x22` | landed (frame `29881`, `p1_health: 36 → 24`) |
| `0x47b3a` | `0x1285a` | `0x12926` | `0x18a04` | `0x47ba0` | `0x47b98` | `0x22` | unexercised |
| `0x47ba8` | `0x12628` | `0x1266e` | `0x189c4` | `0x47c0e` | `0x47c06` | `0x24` | landed (frames `29776`, `p1_health: 48 → 36`, and `31264`, `p1_health: 48 → 38`) |

Three of Eagle's hit functions (`0x47ba8` with Eagle's new spark `0x189c4` and
crouching block state `0x24`, `0x47acc` with `0x18ac4`, and `0x47a5e` with
`0x18984`) land in `sf-win-08`. The other four entries (`0x47914`, `0x47982`,
`0x479f0`, and `0x47b3a`) are translated and traceable in
`crates/sf-engine/src/fighter.rs` but unexercised by this fixture.

## Fixture and verification

`sf-win-08` uses `win-08-run-a` and `win-08-run-b`, with factory-default DIPs
`DSW1=0xdfff` and `DSW2=0xffff`. The scenario is 34,185 frames long and has
checkpoints on every frame from `29426` through `34184` (4,759 checkpoints),
covering Eagle's fight (`29426..=32316`), the stage-3 10-second bonus timeout
(`32316..=33360`), and the transition through the travel map, Thailand intro,
and Adon's stage intro / `ROUND 1` / setup (`0x17f7c`) to Adon's first
fight-loop frame (`34184`). Before divergence frame `29427` (and through frame
`29427`, since Eagle's first input is at `29457`), its computed inputs and
reference observations equal `sf-win-07`.

Eagle's strategy is `shoryuken_hp` over `29457..31600` with a 50-frame period,
10 walk frames and move offset 22. There are no scenario RAM edits. Native
replay starts at fight-loop checkpoint `29426` and applies the scenario inputs.

| Frame | Observed transition |
| --- | --- |
| 29426 | Eagle's round-1 fixture; slot 1 resumes at `0x3ea3e` |
| 29427 | Eagle's first fight-loop frame; slot 1 resumes at `0x3eafa`, `SCENE_MODE = 0x80` |
| 29776 | Eagle's `0x47ba8` hit lands (`p1_health: 48 → 36`, spark `0x189c4`) |
| 29881 | Eagle's `0x47acc` hit lands (`p1_health: 36 → 24`, spark `0x18ac4`) |
| 30199 | Ryu wins round 1; P1 wins change `0 → 1` and event `7` begins |
| 30602 | round-end column wipe, event `6` |
| 30701 | Eagle's ROUND 2 announce, event `5` |
| 31264 | Eagle's `0x47ba8` hit lands (`p1_health: 48 → 38`, spark `0x189c4`) |
| 31383 | Eagle's `0x47a5e` hit lands (`p1_health: 38 → 24`, spark `0x18984`) |
| 31427 | Eagle's `0x47a5e` hit lands (`p1_health: 24 → 12`, spark `0x18984`) |
| 31549 | Ryu wins round 2; P1 wins change `1 → 2` and event `7` begins |
| 31946 | Eagle's match-won screen, event `0xb` |
| 32217 | the match-end wipe, event `6` |
| 32316 | `0x3ef96` dispatches stage 3 to `0x41fe6` (`0x420cc` after its 1-frame setup sleep) |
| 33360 | `0x3efc4` advances `STAGE_NO` (`0xffc003`) from `3` to `4` after the stage-3 bonus times out |
| 34073 | Adon's move-table pointer (`0xffc676`) changes from `0x17d0c` to `0x17f7c` after opponent setup (`0x44880`) |
| 34183 | Adon's round-1 fixture; slot 1 resumes at `0x3ea3e` |
| 34184 | Adon's first fight-loop frame; slot 1 resumes at `0x3eafa`, `SCENE_MODE = 0x80` |

The two reference runs repeat byte-for-byte over all 4,759 checkpoints and all
4,758 post-fixture frames `29427..=34184`. Native capture comparison matches
every captured screen hash and sound-latch observation, with checkpoint pixels
and video/palette data checked on every frame `29427..=32315` (2,889 frames).
The owned-RAM comparison is byte-identical after each of those 2,889 frames and
enters `0x41fe6` (`bonus_stage1_start(..., Bonus10Copy::Stage3)`, sleep resume
`0x420cc`) at frame `32316` when `0x3ef96` dispatches stage 3 to the stage-3
10-second bonus (#133 / `F61`).

`crates/sf-engine/tests/eagle.rs` contains the reference-repeat, prefix equality
against `sf-win-07`, observable, owned-RAM, direct-replay, landed-hit and
landmark checks. The named negative control alters a checkpoint pixel CRC at
frame `30199` (round-1 KO) and verifies that `compare` rejects the candidate at
that exact frame; a second negative control patches Eagle's `0x47ba8` frame
pointer on `0x1266e` (`+0x1c = 0x1268a`) and verifies that `FightMachine` stops
at frame `29776` when `0x47ba8` lands.
`tools/reference/tests/test_search_win.py` also checks the scenario shape,
input prefix, per-frame observations through `29427`, and the work/video/palette/music/pixel
dumps at frames `29426` and `29427`.

## Not covered

- Eagle's four unexercised hit functions (`0x47914`, `0x47982`, `0x479f0`, and
  `0x47b3a`) have no behavioral witness in this fixture.
- Adon's hit functions (`0x17f7c`), turn helper (`0x4558a`), and his fight are
  covered in #134 (`F62`).
