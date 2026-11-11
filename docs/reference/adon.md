# Ryu beats Adon and reaches Sagat

Issue #134 (`F62`) continues `sf-win-08` from Adon's round-1 start (`34183`)
through his defeat, `0x3ef80` (`FIGHT_NO: 0 → 1`), and into stage 4's second
opponent (`0x181ec`, Sagat). This is a development-fixture continuation (**DF**):
it starts from a captured fight-loop checkpoint and follows the pinned input
script, without editing emulated RAM. The implementation is in
`crates/sf-engine/src/fighter.rs`.

## Translation

### Adon's hit functions (`0x47c16..0x47ea8`)

Adon's six hit functions (`0x47c16..0x47ea8`) belong to the AI attack streams
selected for his move table `0x17f7c`. They use the shared CPU hit shape
(`HitFn::Cpu`), with Adon's spark templates (`0x18964`, `0x18984`, `0x189c4`,
`0x18ac4`), damage and reaction entries. Four block against standing block state
`0x22`; `0x47c84` and `0x47e3c` block against crouching state `0x24`.

| Hit function | Stream(s) | Active frame(s) | Spark template | Depth-1 entry | Other-depth entry | Block state | Status in `sf-win-09` |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `0x47c16` | `0x132dc` | `0x132e2`, `0x13322` | `0x18984` | `0x47c7c` | `0x47c74` | `0x22` | landed (frame `35486`, `p1_health: 34 → 24`) |
| `0x47c84` | `0x1336c` | `0x13392` | `0x18964` | `0x47cea` | `0x47ce2` | `0x24` | landed (frame `35425`, `p1_health: 48 → 34`) |
| `0x47cf2` | `0x13850`, `0x1398c` | `0x13896`, `0x13922`, `0x139cc`, `0x13a58`, `0x13b04`, `0x13b90` | `0x18984` | `0x47d58` | `0x47d50` | `0x22` | landed (frame `34296`, `p1_health: 48 → 34`) |
| `0x47d60` | `0x1348c`, `0x1366e` | `0x13578`, `0x1375a` | `0x18964` | `0x47dc6` | `0x47dbe` | `0x22` | unexercised |
| `0x47dce` | `0x1348c`, `0x1366e` | `0x135b8`, `0x1379a` | `0x18ac4` | `0x47e34` | `0x47e2c` | `0x22` | landed (frame `35545`, `p1_health: 24 → 9`) |
| `0x47e3c` | `0x133fc` | `0x13422` | `0x189c4` | `0x47ea2` | `0x47e9a` | `0x24` | unexercised |

Four of Adon's six hit functions (`0x47cf2`, `0x47c84`, `0x47c16`, and `0x47dce`)
land in `sf-win-09`. The other two entries (`0x47d60` and `0x47e3c`) are
translated and traceable in `crates/sf-engine/src/fighter.rs` but unexercised by
this fixture.

### Stream-called turn helper (`0x4558a..0x455d8`)

Adon's multi-hit leaping kick streams `0x13850` (2 hits, calling `0x4558a` at
`0x138b6`) and `0x1398c` (4 hits, calling `0x4558a` at `0x139ec`, `0x13a78`, and
`0x13b24`) invoke `0x4558a` via a `0xff` control-stream entry between active
frames so Adon re-faces his opponent mid-combo:

- `0x4558a..0x45598`: computes the attacker's centre X in `D2` (`0x43be8`
  `center_x(me)`) and the defender's centre X in `D7` (`swap D1; bsr $43be8;
  swap D1`, `center_x(other)`), then compares `D2` against `D7` (`cmp.w D7, D2`).
- `0x4559a: bcs $455bc` (`my_cx < ot_cx`, attacker left of opponent):
  - `0x455bc..0x455c2`: if `eb(me, 0x37) == 2` (`beq $455d2`), already facing
    right — leaves facing unchanged (exercised in `sf-win-09` at frame `34259`,
    call site `0x13a78`).
  - `0x455c6..0x455cc`: otherwise sets `eb(me, 0x37) = 2` and calls `0x4405a`
    (`face_right(me)`) (exercised in `sf-win-09` at frame `34239`, call site
    `0x139ec`, turning `4 → 2`).
- `0x4559e: beq $455d2` (`my_cx == ot_cx`): leaves facing unchanged (translated,
  unexercised in `sf-win-09`; verified directly in `crates/sf-engine/tests/adon.rs`).
- `0x455a2..0x455b8` (`my_cx > ot_cx`, attacker right of opponent):
  - `0x455a2..0x455a8`: if `eb(me, 0x37) == 4` (`beq $455d2`), already facing
    left — leaves facing unchanged (translated, unexercised in `sf-win-09`;
    verified directly in `crates/sf-engine/tests/adon.rs`).
  - `0x455ac..0x455b8`: otherwise sets `eb(me, 0x37) = 4` and calls `0x43fa8`
    (`face_left(me)`) (exercised in `sf-win-09` at frame `34281`, call site
    `0x13b24`, turning `2 → 4`).
- `0x455d2..0x455d8`: `adda.l #6, A1; jmp $42cae.l` (`Resume::Load`, advancing
  past the 6-byte `0xff` call entry and loading the next frame record).

## Fixture and verification

`sf-win-09` uses `win-09-run-a` and `win-09-run-b`, with factory-default DIPs
`DSW1=0xdfff` and `DSW2=0xffff`. The scenario is 37,155 frames long and has
checkpoints on every frame from `34183` through `37154` (2,972 checkpoints),
covering Adon's fight and the transition into Sagat's (`0x181ec`) round-1 fight
loop (`37154`). Before divergence frame `34185` (and through frame `34184`), its
computed inputs and reference observations equal `sf-win-08`.

Adon's strategy is `shoryuken_hp` over `34223..35920` with a 69-frame period, 6
walk frames and move offset 18. There are no scenario RAM edits. Native replay
starts at fight-loop checkpoint `34183` and applies the scenario inputs.

| Frame | Observed transition |
| --- | --- |
| 34183 | Adon's round-1 fixture; slot 1 resumes at `0x3ea3e` |
| 34184 | Adon's first fight-loop frame; slot 1 resumes at `0x3eafa`, `SCENE_MODE = 0x80` |
| 34239 | `0x4558a` at `0x139ec` (`0x1398c` -> `0x13a12`) turns Adon right (`+0x37: 4 → 2`, `0x4405a`) |
| 34259 | `0x4558a` at `0x13a78` (`0x1398c` -> `0x13a9e`) keeps Adon facing right (`+0x37: 2 → 2`, `0x455c2`) |
| 34281 | `0x4558a` at `0x13b24` (`0x1398c` -> `0x13b4a`) turns Adon left (`+0x37: 2 → 4`, `0x43fa8`) |
| 34296 | Adon's `0x47cf2` hit lands (`p1_health: 48 → 34`, spark `0x18984`) |
| 34518 | Ryu wins round 1; P1 wins change `0 → 1` and event `7` begins |
| 35020 | round-end column wipe, event `6` |
| 35119 | Adon's ROUND 2 announce, event `5` |
| 35425 | Adon's `0x47c84` hit lands (`p1_health: 48 → 34`, spark `0x18964`, crouching block state `0x24`) |
| 35486 | Adon's `0x47c16` hit lands (`p1_health: 34 → 24`, spark `0x18984`) |
| 35545 | Adon's `0x47dce` hit lands (`p1_health: 24 → 9`, spark `0x18ac4`) |
| 35967 | Ryu wins round 2; P1 wins change `1 → 2` and event `7` begins |
| 36352 | Adon's match-won screen, event `0xb` |
| 36623 | the match-end wipe, event `6` |
| 36722 | `0x3ef80` advances `FIGHT_NO` (`0xffc004`) from `0` to `1` and starts Sagat's stage intro, event `4` |
| 36874 | Sagat's ROUND 1 announce, event `5` |
| 37043 | Sagat's move-table pointer (`0xffc676`) changes from `0x17f7c` to `0x181ec` after opponent setup (`0x44880`) |
| 37153 | Sagat's round-1 fixture; slot 1 resumes at `0x3ea3e` |
| 37154 | Sagat's first fight-loop frame; slot 1 resumes at `0x3eafa`, `SCENE_MODE = 0x80` |

The two reference runs repeat byte-for-byte over all 2,972 checkpoints and all
2,971 post-fixture frames `34184..=37154`. Native capture comparison matches
every captured screen hash and sound-latch observation, with checkpoint pixels
and video/palette data checked on every frame `34184..=37154`. The owned-RAM
comparison is byte-identical after each of those 2,971 frames. Continuing direct
native replay past `37154` into Sagat's fight originally stopped at frame
`37206` with `unrecovered = [0x47ff4]` before Sagat's hit functions (`0x47eaa..0x4813d`)
were translated in [sagat](sagat.md) (#135).

`crates/sf-engine/tests/adon.rs` contains the reference-repeat, prefix equality
against `sf-win-08`, observable, owned-RAM, direct-replay, `0x4558a` branch,
landed-hit and landmark checks. The named negative control alters a checkpoint
pixel CRC at frame `34518` (round-1 KO) and verifies that `compare` rejects the
candidate at that exact frame; a second negative control patches Adon's `0x47cf2`
frame pointer on `0x13b90` (`+0x1c = 0x13bac`) and verifies that `FightMachine`
stops at frame `34296` when `0x47cf2` lands.
`tools/reference/tests/test_search_win.py` also checks the scenario shape, input
prefix, per-frame observations through `34184`, and the
work/video/palette/music/pixel dumps at frames `34183` and `34184`.

## Not covered

- Adon's two unexercised hit functions (`0x47d60` and `0x47e3c`) have no
  behavioral witness in `sf-win-09`.
- Sagat's hit functions (`0x181ec`), projectile helpers (`0x45ace`, `0x43652`),
  and his fight are covered in [sagat](sagat.md) (#135 / `F63`).
