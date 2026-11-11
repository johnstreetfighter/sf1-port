# Ryu beats Joe and reaches Mike's first new hit function

Issue #68 continues `sf-win-02` from Joe's round-1 start through his defeat
and Mike's setup. This is a development-fixture continuation (**DF**): it
starts from a captured fight-loop checkpoint and follows the pinned input
script, without editing emulated RAM. The implementation is in
`crates/sf-engine/src/fighter.rs`.

## Translation

Joe's eight hit functions are referenced by the streams in his move table
`0x1734c`. They use the shared CPU hit shape, with Joe's spark, damage and
reaction entries. Six block against defender state `0x22`; `0x46bc2` and
`0x46c9e` block against crouching state `0x24`.

| Hit function | Depth-1 entry | Other-depth entry | Block state |
| --- | --- | --- | --- |
| `0x46ae6` | `0x46b4c` | `0x46b44` | `0x22` |
| `0x46b54` | `0x46bba` | `0x46bb2` | `0x22` |
| `0x46bc2` | `0x46c28` | `0x46c20` | `0x24` |
| `0x46c30` | `0x46c96` | `0x46c8e` | `0x22` |
| `0x46c9e` | `0x46d04` | `0x46cfc` | `0x24` |
| `0x46d0c` | `0x46d72` | `0x46d6a` | `0x22` |
| `0x46d7a` | `0x46de0` | `0x46dd8` | `0x22` |
| `0x46de8` | `0x46e4e` | `0x46e46` | `0x22` |

Only `0x46c9e` is exercised by `sf-win-03`. The other seven entries
(`0x46ae6`, `0x46b54`, `0x46bc2`, `0x46c30`, `0x46d0c`, `0x46d7a` and
`0x46de8`) are translated but unexercised by this fixture. Mike's separate
hit-function block, including the boundary at `0x4699c`, is not translated
by this ticket.

## Fixture and verification

`sf-win-03` uses `win-03-run-a` and `win-03-run-b`, with factory-default DIPs
`DSW1=0xdfff` and `DSW2=0xffff`. The scenario is 15,545 frames long and has
checkpoints on every frame from `10840` through `15544`. Before divergence
frame `10840`, its computed inputs and reference observations equal
`sf-win-02`.

Joe's strategy is `shoryuken_hp` with a 60-frame period, 20 walk frames and
move offset 22 over `10840..13711`. The same parameters continue over
`13712..15544` against Mike. Native replay starts at fight-loop checkpoint
`10840` and applies the scenario inputs.

| Frame | Observed transition |
| --- | --- |
| 10840 | Joe's round-1 fixture; slot 1 resumes at `0x3ea3e` |
| 10841 | Joe's first fight-loop frame; slot 1 resumes at `0x3eafa` |
| 11172 | Ryu wins round 1; P1 wins change `0 → 1` and event `7` begins |
| 12372 | Ryu wins round 2; P1 wins change `1 → 2` and event `7` begins |
| 12910 | Joe's match-won screen, event `0xb` |
| 13181 | the match-end wipe, event `6` |
| 13280 | Mike's stage intro, event `4`; fight counter is `1` |
| 13432 | Mike's ROUND 1 event `5` |
| 13601 | Mike's move-table pointer is `0x170dc` after opponent setup |
| 13711 | Mike's round-1 fixture; slot 1 resumes at `0x3ea3e` |
| 13712 | Mike's first fight-loop frame; slot 1 resumes at `0x3eafa` |
| 15544 | native execution stops at Mike's untranslated hit function `0x4699c` |

The two reference runs repeat exactly over all 4,703 post-fixture frames
`10841..=15543`. Native capture comparison matches every captured screen hash
and sound-latch observation, with checkpoint pixels and video/palette data
checked on every frame. The owned-RAM comparison is byte-identical after each
of those 4,703 frames. At frame `15544`, native execution stops with only
`0x4699c` unrecovered. Direct replay independently reaches the same boundary
with stage/fight/winner/round counters `1/1/0/1`.

`crates/sf-engine/tests/joe.rs` contains the reference-repeat, observable,
RAM, direct-replay, boundary and landmark checks. The named negative control
changes the recorded pixel CRC at frame `12192` from `3b1b0813` to
`3b1b0812`; comparison rejects that checkpoint at the same frame.
`tools/reference/tests/test_search_win.py` also checks the scenario shape,
input prefix, per-frame observations through the divergence frame, and the
frame-`10840` work/video/palette/music/pixel dumps.

The continuation through Mike's defeat and the stage-1 bonus boundary is
[Mike](mike.md) (#84).
