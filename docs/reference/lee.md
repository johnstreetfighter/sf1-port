# Ryu beats Lee and reaches Gen's setup

Issue #125 (`F53`) continues `sf-win-04` from Lee's round-1 start (`19379`)
through his defeat and Gen's setup (`0x1782c`). This is a development-fixture
continuation (**DF**): it starts from a captured fight-loop checkpoint and
follows the pinned input script, without editing emulated RAM. The
implementation is in `crates/sf-engine/src/fighter.rs`.

**Identity.** `0x175bc` is Lee. In `win-04-run-a` the VS screen before
this fight reads `RYU VS LEE` (frame `19003`), and the fight is on the Great
Wall stage with `0xffc676 = 0x175bc` in the same frame's work-RAM dump
(`19380`). Before 2026-11-02 this page was `birdie.md` and called this
opponent Birdie.

## Translation

Lee's nine hit functions belong to the AI attack streams selected for his
move table `0x175bc`. They use the shared CPU hit shape (`HitFn::Cpu`), with
Lee's spark templates (`0x18944`, `0x18984`, `0x18a24`, `0x18a44`, `0x18aa4`,
`0x18ac4`, `0x18b04`), damage and reaction entries. Eight block against
standing block state `0x22`; `0x471c6` blocks against crouching state `0x24`.

| Hit function | Stream | Active frame(s) | Spark template | Depth-1 entry | Other-depth entry | Block state | Status in `sf-win-05` |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `0x46e56` | `0xf366` | `0xf38c` | `0x18944` | `0x46ebc` | `0x46eb4` | `0x22` | landed (frame `19755`, `p1_health: 22 → 16`) |
| `0x46ec4` | `0xf3d6` | `0xf41c` | `0x18984` | `0x46f2a` | `0x46f22` | `0x22` | unexercised |
| `0x46f32` | `0xf4a6` | `0xf538` | `0x18984` | `0x46f98` | `0x46f90` | `0x22` | landed (frame `19476`, `p1_health: 36 → 22`) |
| `0x46fa0` | `0xf5c8` | `0xf60e` | `0x18a24` | `0x47006` | `0x46ffe` | `0x22` | unexercised |
| `0x4700e` | `0xf678` | `0xf6de` | `0x18a44` | `0x47074` | `0x4706c` | `0x22` | unexercised |
| `0x4707c` | `0xfaec` | `0xfbd8`, `0xfbf8` | `0x18ac4` | `0x470e2` | `0x470da` | `0x22` | landed (frame `19443`, `p1_health: 48 → 36`) |
| `0x470ea` | `0xf788` | `0xf874`, `0xf894` | `0x18b04` | `0x47150` | `0x47148` | `0x22` | unexercised |
| `0x47158` | `0xf92a` | `0xfa16`, `0xfa36` | `0x18b04` | `0x471be` | `0x471b6` | `0x22` | unexercised |
| `0x471c6` | `0xfc8e` | `0xfcfa` | `0x18aa4` | `0x4722c` | `0x47224` | `0x24` | unexercised |

Three of Lee's hit functions (`0x46e56` with the new spark `0x18944`,
`0x46f32` with `0x18984`, and `0x4707c` with `0x18ac4`) land in `sf-win-05`.
The other six entries (`0x46ec4`, `0x46fa0`, `0x4700e`, `0x470ea`, `0x47158`
and `0x471c6`) are translated and traceable in `crates/sf-engine/src/fighter.rs`
but unexercised by this fixture.

## Fixture and verification

`sf-win-05` uses `win-05-run-a` and `win-05-run-b`, with factory-default DIPs
`DSW1=0xdfff` and `DSW2=0xffff`. The scenario is 22,267 frames long and has
checkpoints on every frame from `19379` through `22266` (2,888 checkpoints).
Before divergence frame `19380` (and through frame `19380`, since Lee's
first input is at `19410`), its computed inputs and reference observations
equal `sf-win-04`.

Lee's strategy is `shoryuken_hp` over `19410..21200` with a 50-frame period,
10 walk frames and move offset 22. There are no scenario RAM edits. Native
replay starts at fight-loop checkpoint `19379` and applies the scenario inputs.

| Frame | Observed transition |
| --- | --- |
| 19379 | Lee's round-1 fixture; slot 1 resumes at `0x3ea3e` |
| 19380 | Lee's first fight-loop frame; slot 1 resumes at `0x3eafa`, `SCENE_MODE = 0x80` |
| 19443 | Lee's `0x4707c` hit lands (`p1_health: 48 → 36`, spark `0x18ac4`) |
| 19476 | Lee's `0x46f32` hit lands (`p1_health: 36 → 22`, spark `0x18984`) |
| 19755 | Lee's `0x46e56` hit lands (`p1_health: 22 → 16`, spark `0x18944`) |
| 19902 | Ryu wins round 1; P1 wins change `0 → 1` and event `7` begins |
| 20314 | round-end column wipe, event `6` |
| 20413 | Lee's ROUND 2 announce, event `5` |
| 20902 | Ryu wins round 2; P1 wins change `1 → 2` and event `7` begins |
| 21464 | Lee's match-won screen, event `0xb` |
| 21735 | the match-end wipe, event `6` |
| 21834 | Gen's stage intro, event `4`; fight counter (`0xffc004`) is `1` |
| 21986 | Gen's ROUND 1 announce, event `5` |
| 22155 | Gen's move-table pointer (`0xffc676`) is `0x1782c` after opponent setup (`0x44880`) |
| 22265 | Gen's round-1 fixture; slot 1 resumes at `0x3ea3e` |
| 22266 | Gen's first fight-loop frame; slot 1 resumes at `0x3eafa`, `SCENE_MODE = 0x80` |

The two reference runs repeat byte-for-byte over all 2,888 checkpoints and all
2,887 post-fixture frames `19380..=22266`. Native capture comparison matches
every captured screen hash and sound-latch observation, with checkpoint pixels
and video/palette data checked on every frame `19380..=22266`. The owned-RAM
comparison is byte-identical after each of those 2,887 frames, with zero
untranslated branches reached.

`crates/sf-engine/tests/lee.rs` contains the reference-repeat, prefix
equality against `sf-win-04`, observable, owned-RAM, direct-replay, landed-hit
and landmark checks. The named negative control alters acheckpoint pixel CRC
at frame `19902` (round-1 KO) and verifies that `compare` rejects the candidate
at that exact frame; a second negative control removes `0x46e56` and verifies
that `FightMachine` stops at frame `19755` reporting `0x46e56`.
`tools/reference/tests/test_search_win.py` also checks the scenario shape,
input prefix, per-frame observations through `19380`, and the work/video/palette/music/pixel
dumps at frames `19379` and `19380`.

## Not covered

- Lee's six unexercised hit functions (`0x46ec4`, `0x46fa0`, `0x4700e`,
  `0x470ea`, `0x47158` and `0x471c6`) have no behavioral witness in this
  fixture.
- Gen's hit functions (`0x1782c`, spark `0x189a4`) and his fight are covered
  in #129 (`F57`, see `docs/reference/gen.md`).
