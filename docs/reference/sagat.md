# Ryu beats Sagat and reaches the ending

Issue #135 (`F63`) continues `sf-win-09` from Sagat's round-1 start (`37153`)
through his defeat, his match-won card (`0x159b2`, voice `0xba`), and `0x3ef96`'s
dispatch of `STAGE_NO == 4` to the ending (`0x44a40`). This is a
development-fixture continuation (**DF**): it starts from a captured fight-loop
checkpoint and follows the pinned input script, without editing emulated RAM.
The implementation is in `crates/sf-engine/src/fighter.rs`.

## Translation

### Sagat's hit functions (`0x47eaa..0x4813d`)

Sagat's six hit functions (`0x47eaa..0x4813d`) belong to the move table
`0x181ec` and the AI attack streams copied for stage 4, fight 1. They use the
shared CPU hit shape (`HitFn::Cpu`), with Sagat's spark templates (`0x189c4`,
`0x18a04`, `0x18a24`, `0x18ac4`), damage and reaction entries. Five block
against standing block state `0x22`; `0x47f18` blocks against crouching state
`0x24`.

| Hit function | Stream(s) | Active frame(s) | Spark template | Depth-1 entry | Other-depth entry | Block state | Status in `sf-win-10` |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `0x47eaa` | `0x13ea0` | `0x13ec6` | `0x18a04` | `0x47f10` | `0x47f08` | `0x22` | unexercised |
| `0x47f18` | `0x13f10` | `0x13f36` | `0x18a24` | `0x47f7e` | `0x47f76` | `0x24` | unexercised |
| `0x47f86` | `0x13fa0`, `0x143a4` | `0x13fc6`, `0x1444a` | `0x189c4` | `0x47fec` | `0x47fe4` | `0x22` | landed (frame `38640`, `p1_health: 23 → 13`) |
| `0x47ff4` | `0x14030`, `0x143a4` | `0x14056`, `0x14076`, `0x143ca`, `0x143ea`, `0x1440a` | `0x18a04` | `0x4805a` | `0x48052` | `0x22` | landed (frame `38437`, `p1_health: 48 → 36`) |
| `0x48062` | `0x14242` | `0x1428e`, `0x142ae`, `0x142ce`, `0x142ee`, `0x1430e` | `0x18a04` | `0x480c8` | `0x480c0` | `0x22` | unexercised |
| `0x480d0` | `0x13bfa`, `0x140e0` | `0x13c40`, `0x13c60`, `0x13c80`, `0x13ca0`, `0x13cc0`, `0x1412c`, `0x1414c`, `0x1416c`, `0x1418c`, `0x141ac` | `0x18ac4` | `0x48136` | `0x4812e` | `0x22` | landed (frame `38500`, `p1_health: 36 → 23`) |

Three of Sagat's six normal hit functions (`0x47ff4`, `0x480d0`, and `0x47f86`)
land in `sf-win-10`. The other three entries (`0x47eaa`, `0x47f18`, and
`0x48062`) are translated and traceable in `crates/sf-engine/src/fighter.rs` but
unexercised by this fixture.

### Tiger Shot projectile spawn helper (`0x45ace..0x45aee`)

Sagat's projectile attack stream `0x144b4` invokes `0x45ace` at `0x144fa` via a
`0xff` control-stream entry:

- `0x45ace..0x45ad6`: queues sound `0x0007` via `0x3b7c8` (`queue_sound`).
- `0x45ada..0x45ae4`: loads template `0x1873c` (`A3`) and spawns a Tiger Shot
  effect object (`+0x10 = 0x190b6`, `+0x14 = 0x43652`, `+0x18 = 0x43476`,
  `+0x1c = 0x43484`) at the attacker's contact point via `0x42e1e`
  (`spawn_effect`).
- `0x45ae6..0x45aee`: `adda.l #6, A1; jmp $42cae.l` (`Resume::Load`, advancing
  past the 6-byte `0xff` call entry and loading the next frame record).

In `sf-win-10`, `0x45ace` executes four times (frames `37170`, `37274`, `37462`,
and `38696`).

### Tiger Shot projectile collision handler (`0x43652..0x436b6`)

When Sagat's Tiger Shot (`0x1873c`, stream `0x190b6`) overlaps the opponent in
`0x430b6` (`effect_vs_fighter`), `effect_handler` dispatches `+0x14 = 0x43652`:

- `0x43652..0x43668`: transforms the projectile in place into explosion template
  `0x186fc` (stream `0x1904a`) via `0x42eb4` (`transform_effect`), swaps `D1` to
  the defender entity offset, and tests bit 5 of the defender's state byte `+9`
  (`0x43662: btst #5, (9,A6,D1.w)`).
- **Blocked branch (`0x43668: bne.s $4368a`, `+9 & 0x20 != 0`):**
  - `0x4368a..0x436a6`: loads chip-damage entry `0x436b0` (`0f 00 32 30 06 00`,
    15 damage, score index `0`, blocked reaction `0x32` / KO `0x30`, sound `6`),
    calls `0x3b9f0` (`add_score`), `0x3c034` (`damage`), `0x3ba84`
    (`check_hiscore`), and `0x4451a` (`react`), then swaps `D1` back (`0x43686`)
    and returns.
  - Exercised in `sf-win-10` at frame `37187` when Ryu blocks the first Tiger
    Shot (`+9 == 0x22`, `p1_health: 48 → 33`).
- **Unblocked branch (`0x4366a..0x43688`, `+9 & 0x20 == 0`):**
  - `0x4366a..0x43684`: sets bit 5 of `SCENE_MODE` (`0xffc008 |= 0x20`), writes
    screen event `9` (`0xff8e36 = 9`, projectile slow motion `0x3ff56`, which
    sets `0xffc00b = 0xff` for 90 frames), loads hit entry `0x436a8`
    (`1e 0c 36 44 09 00`, 30 damage, score index `0x0c`, hit reaction `0x36` /
    KO `0x44`, sound `9`), and calls `add_score`, `damage`, `check_hiscore`, and
    `react`.
  - Exercised in `sf-win-10` at frame `37292` when the second Tiger Shot hits
    Ryu unblocked (`p1_health: 33 → 3`, `0xffc00b: 0x00 → 0xff` through frame
    `37381`, clearing to `0x00` at frame `37382`).

### Projectile vs. projectile (`0x43484`) and projectile vs. attack (`0x43476`)

Template `0x1873c` sets `+0x18 = 0x43476` and `+0x1c = 0x43484`, the shared
fireball handlers translated in F9 (#49).

## Fixture and verification

`sf-win-10` uses `win-10-run-a` and `win-10-run-b`, with factory-default DIPs
`DSW1=0xdfff` and `DSW2=0xffff`. The scenario is 43,619 frames long and has
checkpoints on every frame from `37153` through `43618` (6,466 checkpoints),
covering Sagat's fight (`37153..=39775`), the ending (`39776..=43616`), and
`0x3efc4`'s transition (`STAGE_NO: 4 → 5`) into `0x3f2d0` / `0x42422` (`43617..=43618`).
Before divergence frame `37155` (and through frame `37154`), its computed inputs
and reference observations equal `sf-win-09`.

Sagat's strategy combines two `block_high` cycles over `37154..37180` (`p1_left`
held on `37155..37175` and `37180..37200` so the first Tiger Shot is blocked at
`37187` and the second hits unblocked at `37292`) followed by `shoryuken_hp` over
`37309..39009` with a 62-frame period, 12 walk frames and move offset 18. There
are no scenario RAM edits. Native replay starts at fight-loop checkpoint `37153`
and applies the scenario inputs.

| Frame | Observed transition |
| --- | --- |
| 37153 | Sagat's round-1 fixture; slot 1 resumes at `0x3ea3e` |
| 37154 | Sagat's first fight-loop frame; slot 1 resumes at `0x3eafa`, `SCENE_MODE = 0x80` |
| 37170 | Sagat spawns his first Tiger Shot (`0x45ace` at `0x144fa` → template `0x1873c`, stream `0x190b6`) |
| 37187 | First Tiger Shot hits blocking Ryu (`0x43652` chip-damage branch `0x436b0`, `p1_health: 48 → 33`, explosion `0x186fc` stream `0x1904a`) |
| 37274 | Sagat spawns his second Tiger Shot (`0x45ace` → template `0x1873c`) |
| 37292 | Second Tiger Shot hits unblocked Ryu (`0x43652` unblocked branch `0x436a8`, `p1_health: 33 → 3`, slow motion event `9` sets `0xffc00b = 0xff`) |
| 37382 | Slow motion event `9` finishes its 90-frame hold and clears `0xffc00b` (`0xff → 0x00`) |
| 37462 | Sagat spawns his third Tiger Shot (`0x45ace` → template `0x1873c`) |
| 37645 | Ryu wins round 1 (`p1_wins: 0 → 1`, `p1_health = 3`, event `7`) |
| 38126 | round-end column wipe, event `6` |
| 38225 | Sagat's ROUND 2 announce, event `5` |
| 38437 | Sagat's `0x47ff4` hit lands (`p1_health: 48 → 36`, spark `0x18a04`) |
| 38500 | Sagat's `0x480d0` hit lands (`p1_health: 36 → 23`, spark `0x18ac4`) |
| 38640 | Sagat's `0x47f86` hit lands (`p1_health: 23 → 13`, spark `0x189c4`) |
| 38696 | Sagat spawns his fourth Tiger Shot (`0x45ace` → template `0x1873c`) |
| 39009 | Ryu wins round 2 (`p1_wins: 1 → 2`, `p1_health = 13`, event `7`) |
| 39406 | Sagat's match-won screen (`0x159b2`, voice `0xba`, event `0xb`) |
| 39677 | the match-end wipe, event `6` |
| 39775 | last frame of the match-end wipe (`0xffc003 = 4`, `0xffc004 = 1`, `0xffc009 = 1`) |
| 39776 | `0x3ef96` clears `0xffc004`, `0xffc00a`, `0xffc009` and dispatches `STAGE_NO == 4` to `0x44a40`; slot 1 sleeps at `0x44a60` (ending fixture for #136 / `F64`) |
| 43617 | `0x44c16` jumps to `0x3efc4` (`STAGE_NO: 4 → 5`, `FIGHT_NO = 0`), entering `0x3f2d0` → `0x42422` (`0x42552`) |

The two reference runs repeat byte-for-byte over all 6,466 checkpoints and all
6,465 post-fixture frames `37154..=43618`. Native capture comparison over the
fight slice `37153..=39775` (2,622 frames) matches every captured screen hash
and sound-latch observation, with checkpoint pixels and video/palette data
checked on every frame `37154..=39775`. The owned-RAM comparison is
byte-identical after each of those 2,622 frames. Stepping one frame further to
`39776` reaches `0x3ef96`'s stage-4 dispatch into `0x44a40` (sleeping at
`0x44a60`, covered by [ending](ending.md), #136 / `F64`).

`crates/sf-engine/tests/sagat.rs` contains the reference-repeat, prefix equality
against `sf-win-09`, observable, owned-RAM, direct-replay, Tiger Shot spawn /
chip-damage / unblocked slow-motion, landed-hit and landmark checks. The named
negative control alters a checkpoint pixel CRC at frame `37645` (round-1 KO) and
verifies that `compare` rejects the candidate at that exact frame; three
additional negative controls patch `0x45ace` (`0x144fc`), `0x43652` (`0x18750`),
and `0x47ff4` (`0x14072..0x14426`) to untranslated `0x492d6` and verify that
native execution stops at frames `37170`, `37187`, and `38437` respectively.
`tools/reference/tests/test_search_win.py` also checks the scenario shape, input
prefix, per-frame observations through `37154`, and the
work/video/palette/music/pixel dumps at frames `37153` and `37154`.

## Not covered

- Sagat's three unexercised normal hit functions (`0x47eaa`, `0x47f18`, and
  `0x48062`) have no behavioral witness in `sf-win-10`.
- The ending (`0x44a40..0x44c1c`, frames `39776..=43618` of `sf-win-10`) is
  documented and verified in [ending](ending.md) (#136 / `F64`).
