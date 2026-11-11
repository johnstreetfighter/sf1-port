# Ryu beats Mike and reaches the stage-1 bonus

Issue #84 continues `sf-win-03` from Mike's round-1 start through his defeat.
The reference continues through the stage-1 bonus and stage 2's arrival.
This is a development-fixture continuation (**DF**): it starts from a captured
fight-loop checkpoint and follows the pinned input script, without editing
emulated RAM. The implementation is in `crates/sf-engine/src/fighter.rs`.

## Translation

Mike's four hit functions belong to the AI attack streams selected for his
move table `0x170dc`. They use the shared CPU hit shape, with Mike's spark,
damage and reaction entries. Three block against defender state `0x22`;
`0x4699c` blocks against crouching state `0x24`.

| Hit function | Stream | Active frame | Spark template | Depth-1 entry | Other-depth entry | Block state |
| --- | --- | --- | --- | --- | --- | --- |
| `0x4692e` | `0xd8c6` | `0xd8ec` | `0x18964` | `0x46994` | `0x4698c` | `0x22` |
| `0x4699c` | `0xda26` | `0xda8c` | `0x18984` | `0x46a02` | `0x469fa` | `0x24` |
| `0x46a0a` | `0xdb1c` | `0xdb62` | `0x18984` | `0x46a70` | `0x46a68` | `0x22` |
| `0x46a78` | `0xd956` | `0xd99c` | `0x18984` | `0x46ade` | `0x46ad6` | `0x22` |

Only `0x4699c` is exercised by `sf-win-04`; hit dispatch reaches it three
times. The other three entries (`0x4692e`, `0x46a0a` and `0x46a78`) are
translated but unexercised by this fixture.

## Fixture and verification

`sf-win-04` uses `win-04-run-a` and `win-04-run-b`, with factory-default DIPs
`DSW1=0xdfff` and `DSW2=0xffff`. The scenario is 19,381 frames long and has
checkpoints on every frame from `13711` through `19380` (5,670 checkpoints).
Before divergence frame `13712`, its computed inputs and reference
observations equal `sf-win-03`.

Mike's strategy is `shoryuken_hp` with a 60-frame period, 20 walk frames and
move offset 22 over `13712..18000`. The strategy supplies the fixed bonus-2
inputs; there are no scenario RAM edits. Native replay starts at fight-loop
checkpoint `13711` and applies the scenario inputs.

| Frame | Observed transition |
| --- | --- |
| 13711 | Mike's round-1 fixture; slot 1 resumes at `0x3ea3e` |
| 13712 | Mike's first fight-loop frame; slot 1 resumes at `0x3eafa` |
| 14164 | Ryu wins round 1; P1 wins change `0 → 1` and event `7` begins |
| 15695 | Mike wins round 2; CPU wins change `0 → 1`, P1 wins stay `1`, event `7` begins |
| 16564 | Ryu wins round 3; P1 wins change `1 → 2` and event `7` begins |
| 17114 | Mike's match-won screen, event `0xb` |
| 17482 | the match-end wipe, event `6` |
| 17484 | `0x3ef96` dispatches stage 1 (`0x41740`), entering `(1, 0x41814)` (translated by #124 / F52; see [match-win](match-win.md)) |
| 18528 | reference stage counter changes `1 → 2` |
| 19269 | the next opponent's move-table pointer is `0x175bc` |
| 19379 | stage-2 round-1 fixture; slot 1 resumes at `0x3ea3e` |
| 19380 | stage-2 first fight-loop frame; slot 1 resumes at `0x3eafa` |

The two reference runs repeat exactly over all 3,772 post-fixture frames
`13712..=17483`. Native capture comparison matches every captured screen hash
and sound-latch observation, with checkpoint pixels and video/palette data
checked on every frame. The owned-RAM comparison is byte-identical after each
of those 3,772 frames. At frame `17484`, native execution enters the stage-1
bonus (`0x41740 -> (1, 0x41814)`, translated by #124 / F52). Direct replay
independently reaches the same boundary with stage/fight/winner/round counters
`1/0/0/0`.

`crates/sf-engine/tests/mike.rs` contains the reference-repeat, observable,
RAM, direct-replay, boundary and landmark checks. The named negative control
changes the recorded pixel CRC at frame `14164` from `991de5fe` to `991de5fd`;
comparison rejects that checkpoint at the same frame.
`tools/reference/tests/test_search_win.py` also checks the scenario shape,
input prefix, per-frame observations through the divergence frame, and the
work/video/palette/music/pixel dumps at frames `13711` and `13712`.

## Not covered

- `0x41740..0x419fc` (the stage-1 ten-second bonus routine) and the transition
  into stage 2 (`17483..=19380`) are translated and compared in #124 (F52; see
  [match-win](match-win.md) and `crates/sf-engine/tests/bonus_stage1.rs`).
- Mike's three unexercised hit functions listed above have no behavioral
  witness in this fixture.
- Lee's hit functions (`0x175bc`) and the first stage-2 fight are covered in
  [Lee](lee.md) (#125, `F53`).
