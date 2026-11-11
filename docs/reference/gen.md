# Ryu beats Gen and reaches the stage-2 bonus

Issue #129 (`F57`) continues `sf-win-05` from Gen's round-1 start (`22265`)
through his defeat, the stage-2 bonus (`0x41a02`), and into stage 3 (`0x17a9c`,
Birdie). This is a development-fixture continuation (**DF**): it starts from a
captured fight-loop checkpoint and follows the pinned input script, without
editing emulated RAM. The implementation is in `crates/sf-engine/src/fighter.rs`.

**Identity.** `0x1782c` is Gen. In `win-05-run-a` the VS screen before
this fight reads `RYU VS GEN` (frame `21868`), and the fight is on the China
street stage with `0xffc676 = 0x1782c` in the same frame's work-RAM dump
(`22266`). Before 2026-11-02 this page was `eagle.md` and called this
opponent Eagle.

## Translation

Gen's eight hit functions belong to the AI attack streams selected for his
move table `0x1782c`. They use the shared CPU hit shape (`HitFn::Cpu`), with
Gen's spark templates (`0x18984`, `0x189a4`, `0x18aa4`, `0x18ac4`, `0x18b04`),
damage and reaction entries. Six block against standing block state `0x22`;
`0x47310` and `0x47536` block against crouching state `0x24`.

| Hit function | Stream | Active frame(s) | Spark template | Depth-1 entry | Other-depth entry | Block state | Status in `sf-win-06` |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `0x47234` | `0x103b0` | `0x103d6` | `0x18984` | `0x4729a` | `0x47292` | `0x22` | unexercised |
| `0x472a2` | `0x10440` | `0x10486` | `0x18984` | `0x47308` | `0x47300` | `0x22` | unexercised |
| `0x47310` | `0x10b4e` | `0x10bd4`, `0x10bf4`, `0x10c14` | `0x189a4` | `0x47376` | `0x4736e` | `0x24` | landed (frames `23845`, `p1_health: 36 → 24`, and `23862`, `p1_health: 24 → 12`) |
| `0x4737e` | `0x104f0` | `0x10516` | `0x18984` | `0x473e4` | `0x473dc` | `0x22` | unexercised |
| `0x473ec` | `0x10884` | `0x10930`, `0x10950`, `0x10970`, `0x10990` | `0x18ac4` | `0x47452` | `0x4744a` | `0x22` | unexercised |
| `0x4745a` | `0x10580` | `0x1064c`, `0x1066c` | `0x18b04` | `0x474c0` | `0x474b8` | `0x22` | landed (frame `23691`, `p1_health: 48 → 36`) |
| `0x474c8` | `0x10702` | `0x107ce`, `0x107ee` | `0x18b04` | `0x4752e` | `0x47526` | `0x22` | unexercised |
| `0x47536` | `0x10a26` | `0x10a72`, `0x10a98`, `0x10abe` | `0x18aa4` | `0x4759c` | `0x47594` | `0x24` | unexercised |

Two of Gen's hit functions (`0x47310` with the new spark `0x189a4` and
crouching block state `0x24`, and `0x4745a` with `0x18b04`) land in `sf-win-06`.
The other six entries (`0x47234`, `0x472a2`, `0x4737e`, `0x473ec`, `0x474c8` and
`0x47536`) are translated and traceable in `crates/sf-engine/src/fighter.rs`
but unexercised by this fixture.

## Fixture and verification

`sf-win-06` uses `win-06-run-a` and `win-06-run-b`, with factory-default DIPs
`DSW1=0xdfff` and `DSW2=0xffff`. The scenario is 26,373 frames long and has
checkpoints on every frame from `22265` through `26372` (4,108 checkpoints),
covering Gen's fight (`22265..=24854`), the stage-2 bonus (`24854..=25582`),
and the transition through the travel map, England intro, and Birdie's stage intro /
`ROUND 1` / setup (`0x17a9c`) to Birdie's first fight-loop frame (`26372`).
Before divergence frame `22266` (and through frame `22266`, since Gen's first
input is at `22276`), its computed inputs and reference observations equal
`sf-win-05`.

Gen's strategy is `shoryuken_hp` over `22276..24150` with a 60-frame period,
15 walk frames and move offset 22, followed by a `p1_hp` press at frame `25027`
(`hold_frames = 6`) for the stage-2 bonus break. There are no scenario RAM
edits. Native replay starts at fight-loop checkpoint `22265` and applies the
scenario inputs.

| Frame | Observed transition |
| --- | --- |
| 22265 | Gen's round-1 fixture; slot 1 resumes at `0x3ea3e` |
| 22266 | Gen's first fight-loop frame; slot 1 resumes at `0x3eafa`, `SCENE_MODE = 0x80` |
| 22488 | Ryu wins round 1; P1 wins change `0 → 1` and event `7` begins |
| 23047 | round-end column wipe, event `6` |
| 23146 | Gen's ROUND 2 announce, event `5` |
| 23691 | Gen's `0x4745a` hit lands (`p1_health: 48 → 36`, spark `0x18b04`) |
| 23845 | Gen's `0x47310` hit lands (`p1_health: 36 → 24`, spark `0x189a4`) |
| 23862 | Gen's `0x47310` hit lands (`p1_health: 24 → 12`, spark `0x189a4`) |
| 24108 | Ryu wins round 2; P1 wins change `1 → 2` and event `7` begins |
| 24484 | Gen's match-won screen, event `0xb` |
| 24755 | the match-end wipe, event `6` |
| 24854 | `0x3ef96` dispatches stage 2 to `0x41a02` (`0x41aa6` after its 1-frame setup sleep) |
| 25027 | `p1_hp` press during the stage-2 bonus meter (power 10 break) |
| 25582 | `0x3efc4` advances `STAGE_NO` (`0xffc003`) from `2` to `3` |
| 26261 | Birdie's move-table pointer (`0xffc676`) is `0x17a9c` after opponent setup (`0x44880`) |
| 26371 | Birdie's round-1 fixture; slot 1 resumes at `0x3ea3e` |
| 26372 | Birdie's first fight-loop frame; slot 1 resumes at `0x3eafa`, `SCENE_MODE = 0x80` |

The two reference runs repeat byte-for-byte over all 4,108 checkpoints and all
4,107 post-fixture frames `22266..=26372`. Native capture comparison matches
every captured screen hash and sound-latch observation, with checkpoint pixels
and video/palette data checked on every frame `22266..=24853` (2,588 frames).
The owned-RAM comparison is byte-identical after each of those 2,588 frames, and
reaches `0x3ef96`'s dispatch of stage 2 to `0x41a02` (sleeping at `0x41aa6`) at
frame `24854`; #130 (`F58`, [match-win](match-win.md)) continues native comparison
of `sf-win-06` and `sf-bonus-03` across `0x41a02..0x41f5c` through Birdie's `ROUND 1`
fight-loop entry (`26372` / `26550`).

`crates/sf-engine/tests/gen.rs` contains the reference-repeat, prefix
equality against `sf-win-05`, observable, owned-RAM, direct-replay, landed-hit
and landmark checks. The named negative control alters a checkpoint pixel CRC
at frame `22488` (round-1 KO) and verifies that `compare` rejects the candidate
at that exact frame; a second negative control patches Gen's `0x47310` frame
pointers (`0x10bd4`, `0x10bf4`, `0x10c14`) and verifies that `FightMachine`
stops at frame `23845` when `0x47310` lands.
`tools/reference/tests/test_search_win.py` also checks the scenario shape,
input prefix, per-frame observations through `22266`, and the work/video/palette/music/pixel
dumps at frames `22265` and `22266`.

## Not covered

- Gen's six unexercised hit functions (`0x47234`, `0x472a2`, `0x4737e`,
  `0x473ec`, `0x474c8` and `0x47536`) have no behavioral witness in this
  fixture.
- Native execution of the stage-2 bonus (`0x41a02..0x41f74`) through stage 3's
  `ROUND 1` is covered in #130 (`F58`).
- Birdie's hit functions (`0x17a9c`) and his fight are covered in #131 (`F59`, see `docs/reference/birdie.md`).
