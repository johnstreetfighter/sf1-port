# Ryu beats Birdie and reaches Eagle's setup

Issue #131 (`F59`) continues `sf-win-06` from Birdie's round-1 start (`26371`)
through his defeat and Eagle's setup (`0x17d0c`). This is a development-fixture
continuation (**DF**): it starts from a captured fight-loop checkpoint and
follows the pinned input script, without editing emulated RAM. The
implementation is in `crates/sf-engine/src/fighter.rs`.

**Identity.** `0x17a9c` is Birdie. In `win-06-run-a` the VS screen before
this fight reads `RYU VS BIRDIE` (frame `25991`), and the fight is on the
England street stage with `0xffc676 = 0x17a9c` in the same frame's work-RAM
dump (`26372`). Before 2026-11-02 this page was `lee.md` and called this
opponent Lee.

## Translation

Birdie's eight hit functions (`0x475a4..0x4790e`) belong to the AI attack streams
in Birdie's stream region (`0x1141c..0x11c63`), between Birdie's non-attack streams
(`0x17a9c..0x17d0b`, ending at `0x1141b`) and Eagle's streams (`0x17d0c..0x17f7b`,
starting at `0x11c64`). They use the shared CPU hit shape (`HitFn::Cpu`), with
Birdie's spark templates (`0x18964`, `0x18984`, `0x18a04`, `0x18ac4`), damage and
reaction entries. Six block against standing block state `0x22`; `0x47680` and
`0x476ee` block against crouching state `0x24`.

| Hit function | Stream | Active frame(s) | Spark template | Depth-1 entry | Other-depth entry | Block state | Status in `sf-win-07` |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `0x475a4` | `0x1141c` | `0x11422` | `0x18964` | `0x4760a` | `0x47602` | `0x22` | unexercised |
| `0x47612` | `0x1146c` | `0x114b2` | `0x18984` | `0x47678` | `0x47670` | `0x22` | landed (frame `26419`, `p1_health: 48 → 40`) |
| `0x47680` | `0x1151c` | `0x11542`, `0x11562`, `0x11582` | `0x18984` | `0x476e6` | `0x476de` | `0x24` | landed (frame `26507`, `p1_health: 40 → 28`) |
| `0x476ee` | `0x115cc` | `0x115f2` | `0x18964` | `0x47754` | `0x4774c` | `0x24` | unexercised |
| `0x4775c` | `0x119a0` | `0x11a8c`, `0x11aac`, `0x11acc` | `0x18984` | `0x477c2` | `0x477ba` | `0x22` | unexercised |
| `0x477ca` | `0x1165c` | `0x11748`, `0x11768`, `0x11788` | `0x18a04` | `0x47830` | `0x47828` | `0x22` | landed (frame `26695`, `p1_health: 28 → 14`) |
| `0x47838` | `0x117fe` | `0x118ea`, `0x1190a`, `0x1192a` | `0x18a04` | `0x4789e` | `0x47896` | `0x22` | unreferenced stream (`0x117fe..0x1199f`) |
| `0x478a6` | `0x11b42` | `0x11b8e`, `0x11bae`, `0x11bce`, `0x11bf4` | `0x18ac4` | `0x4790c` | `0x47904` | `0x22` | landed (frames `27797`, `p1_health: 48 → 36`, and `27798`, `p1_health: 36 → 22`) |

### `0x47838` stream ownership

In `docs/planning/arcade-flow-batch.md`, `0x47838`'s owner was marked `[inf]`
from its position inside Birdie's hit-function block (`0x475a4..0x4790e`).
Scanning the ROM settles its exact stream:

- `0x47838` is referenced at `+0x1c` by three active frame records at `0x118ea`,
  `0x1190a`, and `0x1192a` (`+0x1c = 0x11906, 0x11926, 0x11946`).
- Those three frames belong to the 11-frame airborne jump-kick stream at
  `0x117fe..0x1199f`, situated contiguously between Birdie's jump-kick stream
  `0x1165c..0x117fd` (which uses `0x477ca` on frames `0x11748`, `0x11768`,
  `0x11788`) and Birdie's stream `0x119a0..0x11b41` (which uses `0x4775c`).
- Stream `0x117fe` has the same `0xff` stream-call sequence (`0x45a7a`,
  `0x455de`, `0x45ab2`, `0x454ee`, `0x45630`), sprite tiles, spark template
  (`0x18a04`), and damage/reaction bytes (`0x4789e` / `0x47896` match `0x47830` /
  `0x47828`: `0x0e, 0x0e, 0x42, 0x19, 0x01, 0x00, 0x00, 0x00` and
  `0x0c, 0x0c, 0x41, 0x19, 0x01, 0x00, 0x00, 0x00`) as stream `0x1165c`,
  differing only in its per-frame airborne tick counts (7 ticks instead of 5 on
  its startup and active frames).
- No pointer in Birdie's AI attack tables (`0x2eca4..0x310a2`), move tables
  (`0x16bf4..0x1845c`), or `0xfe` stream jumps targets `0x117fe..0x1199f`
  (whereas `0x1165c` is referenced 414 times across Birdie's AI tables), making
  stream `0x117fe` and `0x47838` unreachable Birdie data in the shipped ROM.

Four of Birdie's hit functions (`0x47612` with `0x18984`, `0x47680` with `0x18984`
and crouching block state `0x24`, `0x477ca` with Birdie's new spark `0x18a04`, and
`0x478a6` with `0x18ac4`) land in `sf-win-07`. The other four entries
(`0x475a4`, `0x476ee`, `0x4775c`, and `0x47838`) are translated and traceable in
`crates/sf-engine/src/fighter.rs` but unexercised by this fixture.

## Fixture and verification

`sf-win-07` uses `win-07-run-a` and `win-07-run-b`, with factory-default DIPs
`DSW1=0xdfff` and `DSW2=0xffff`. The scenario is 29,428 frames long and has
checkpoints on every frame from `26371` through `29427` (3,057 checkpoints).
Before divergence frame `26372` (and through frame `26372`, since Birdie's first
input is at `26402`), its computed inputs and reference observations equal
`sf-win-06`.

Birdie's strategy is `shoryuken_hp` over `26402..28250` with a 60-frame period,
15 walk frames and move offset 22. There are no scenario RAM edits. Native
replay starts at fight-loop checkpoint `26371` and applies the scenario inputs.

| Frame | Observed transition |
| --- | --- |
| 26371 | Birdie's round-1 fixture; slot 1 resumes at `0x3ea3e` |
| 26372 | Birdie's first fight-loop frame; slot 1 resumes at `0x3eafa`, `SCENE_MODE = 0x80` |
| 26419 | Birdie's `0x47612` hit lands (`p1_health: 48 → 40`, spark `0x18984`) |
| 26507 | Birdie's `0x47680` hit lands (`p1_health: 40 → 28`, spark `0x18984`) |
| 26695 | Birdie's `0x477ca` hit lands (`p1_health: 28 → 14`, spark `0x18a04`) |
| 26974 | Ryu wins round 1; P1 wins change `0 → 1` and event `7` begins |
| 27374 | round-end column wipe, event `6` |
| 27473 | Birdie's ROUND 2 announce, event `5` |
| 27797 | Birdie's `0x478a6` hit lands (`p1_health: 48 → 36`, spark `0x18ac4`) |
| 27798 | Birdie's `0x478a6` hit lands (`p1_health: 36 → 22`, spark `0x18ac4`) |
| 28174 | Ryu wins round 2; P1 wins change `1 → 2` and event `7` begins |
| 28625 | Birdie's match-won screen, event `0xb` |
| 28896 | the match-end wipe, event `6` |
| 28995 | Eagle's stage intro, event `4`; fight counter (`0xffc004`) is `1` |
| 29147 | Eagle's ROUND 1 announce, event `5` |
| 29316 | Eagle's move-table pointer (`0xffc676`) is `0x17d0c` after opponent setup (`0x44880`) |
| 29426 | Eagle's round-1 fixture; slot 1 resumes at `0x3ea3e` |
| 29427 | Eagle's first fight-loop frame; slot 1 resumes at `0x3eafa`, `SCENE_MODE = 0x80` |

The two reference runs repeat byte-for-byte over all 3,057 checkpoints and all
3,056 post-fixture frames `26372..=29427`. Native capture comparison matches
every captured screen hash and sound-latch observation, with checkpoint pixels
and video/palette data checked on every frame `26372..=29427`. The owned-RAM
comparison is byte-identical after each of those 3,056 frames, with zero
untranslated branches reached.

`crates/sf-engine/tests/birdie.rs` contains the reference-repeat, prefix equality
against `sf-win-06`, observable, owned-RAM, direct-replay, landed-hit and
landmark checks. The named negative control alters a checkpoint pixel CRC at
frame `26974` (round-1 KO) and verifies that `compare` rejects the candidate at
that exact frame; a second negative control patches Birdie's `0x477ca` frame
pointer on `0x11768` (`+0x1c = 0x11784`) and verifies that `FightMachine`
stops at frame `26695` when `0x477ca` lands.
`tools/reference/tests/test_search_win.py` also checks the scenario shape,
input prefix, per-frame observations through `26372`, and the work/video/palette/music/pixel
dumps at frames `26371` and `26372`.

## Not covered

- Birdie's four unexercised hit functions (`0x475a4`, `0x476ee`, `0x4775c`, and
  unreferenced `0x47838`) have no behavioral witness in this fixture.
- Eagle's hit functions (`0x17d0c`, spark `0x189c4`) and his fight are covered in
  #132 (`F60`, see `docs/reference/eagle.md`).
