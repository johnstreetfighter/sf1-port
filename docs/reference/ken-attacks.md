# Ken's attacks against an idle Ryu

This document records the boundary established by
issue #47: Ken (P2, move
table `0x1731c`) whiffing and landing every standing normal, crouching,
jumping, Shoryuken, Tatsumaki Senpukyaku (including chip damage on a blocking
Ryu), and Hadoken attack against Ryu (P1) in the checkpoint-2450 versus
fixture (`move-run-a`). This is a controlled-fixture result (`CF`), not
complete gameplay.

## Ken's twin routines and tables

Ken uses the same input handler (`0x40360`) and object runner (`0x42ad0`) as
Ryu, reading his moves from `0x1731c` (`0xff86da + 0x12`) while facing left
(`+0x37 == 4`, so `p2_left` is forward and `p2_right` is back). His move
streams call his twin hit functions and stream helpers in
`crates/sf-engine/src/fighter.rs`:

### The 16 twin hit functions (`fighter::hit_function`)

Each of Ryu's 16 hit functions has a twin for Ken that shares the same
damage/reaction table and selects Ken's spark templates (`0x1875c..0x1891c`)
and recoil streams (`0x3028`, `0x3192`, `0x3172`):

| Category | Move(s) | Ken hit function | Ryu twin | Spark template(s) | Damage table | Recoil stream |
| --- | --- | --- | --- | --- | --- | --- |
| Standing LP | `0x0d` | `0x45afa` | `0x45af0` | `0x1875c` | `0x45b6e` | — |
| Standing MP | `0x0f` | `0x45c16` | `0x45c0c` | `0x1879c` | `0x45c88` | — |
| Standing HP | `0x10` | `0x45ca2` | `0x45c98` | `0x1879c` | `0x45d16` | — |
| Standing LK | `0x14` | `0x46034` | `0x4602a` | `0x1881c` | `0x460a8` | — |
| Standing MK | `0x15` | `0x460d8` | `0x460b8` | `0x1883c` | `0x46154` | `0x3028` |
| Standing HK (frame 1) | `0x16` | `0x461a4` | `0x46164` | `0x1887c` | `0x46240` | `0x3192` |
| Standing HK (frame 2) | `0x16` | `0x461c4` | `0x46184` | `0x1885c` | `0x46240` | `0x3172` |
| Crouching punches | `0x11..0x13`, `0x1c..0x1e` | `0x45d44` | `0x45d26` | `0x187bc` (`+0x34 <= 3`), `0x187dc` (`+0x34 > 3`) | `0x45de0` | — |
| Crouching L/M kick | `0x17` | `0x4625a` | `0x46250` | `0x1889c` | `0x462e0` | — |
| Crouching HK sweep | `0x18` | `0x4630a` | `0x46300` | `0x188bc` | `0x4637e` | — |
| Jump punches | `0x20..0x22` | `0x45e2e` | `0x45e10` | `0x187fc` (`0x20`), `0x1881c` (`0x21`, `0x22`) | `0x45eca` | — |
| Neutral jump kicks (frame 1) | `0x23`, `0x26` | `0x46398` | `0x4638e` | `0x188dc` | `0x46462` | — |
| Neutral jump LK/MK (frame 2) | `0x23` | `0x463b8` | `0x463ae` | `0x188fc` | `0x46462` | — |
| Diagonal jump kicks | `0x24`, `0x25` | `0x463d8` | `0x463ce` | `0x1891c` | `0x46462` | — |
| Shoryuken | `0x19` | `0x45f04` | `0x45efa` | `0x1879c` | `0x45f52` | — |
| Tatsumaki Senpukyaku | `0x1b` | `0x45f64` | `0x45f5a` | `0x1891c` | `0x46012` / `0x4601a` | — |

### Jump-attack apex (`0x453b6`) and tables (`0x4545c` / `0x45474`)

At the apex of a jump (`0x09` neutral, `0x0a` forward, `0x0b` back), Ken's
stream calls `0x453b6` (`Bus::jump_attack(me, 0x4545c, 0x45474)`):

- Without a buffered attack (`+9 != 0xc1`), the call returns `Resume::Load`
  and the jump continues.
- With a buffered punch (`+0x34`) or kick (`+0x35`), the jump index
  (`(move - 9) * 8`) selects the 8-byte `(strength_table, stream_table)` entry
  from `0x4545c` (punches: moves `0x20`, `0x21`, `0x22`) or `0x45474` (kicks:
  moves `0x23`/`0x26`, `0x24`, `0x25`).

### Tatsumaki spin loop (`0x4554a` / `0x45566`) and command trackers

- **Command trackers (`0x40ae4`):** Ken's player structure (`0xff86da`) holds
  three 3-byte trackers at `0xff86f0` (`+0x16`, Tatsumaki: D, DB, B),
  `0xff86f3` (`+0x19`, Shoryuken: F, D, DF), and `0xff86f6` (`+0x1c`, Hadoken:
  D, DF, F).
- **Tatsumaki spin loop (`0x4554a` / `0x45566`):** In move `0x1b`, the stream
  entry at `0x003588` calls `0x4554a`, writing repeat count `4` to `0xff870a`
  and loop-start address `0x00358e` to `0xff870c`. The entry at `0x0035fa`
  calls `0x45566`, decrementing `0xff870a` and jumping back to `0x00358e`
  until `0xff870a` reaches `0` (five spins total).
- **Hadoken (`0x1a`):** Calls `0x4548c`, queuing sound `7` and spawning
  projectile template `0x186dc` flipped (`attr | 0x100`, `dx = -4`). When it
  leaves the horizontal screen bounds (`0x60..0x1a0`), `0x43268` frees the
  effect slot; when it hits Ryu (`0x433ba`), it transforms into explosion
  `0x186fc` and either deals `20` damage with slow-motion event `9` (unblocked)
  or `8` chip damage (blocked in 2P versus).

## Scenarios and outcomes

Five versus scenarios (`tools/reference/scenarios/sf_ken_01..05.json`) start
from the checkpoint-2450 fixture (`move-run-a`) with checkpoints every frame.
Ryu's health starts at `48`.

| Scenario | Interval | Moves and outcomes | Twin hit functions landed | Final Ryu HP |
| --- | --- | --- | --- | --- |
| `sf-ken-01` | `2450..=3300` | Standing normals: `0x0d` (LP), `0x0f` (MP), `0x10` (HP), `0x14` (LK), `0x15` (MK), `0x16` (HK) whiff far (`2458..2695`), then walk left and land `0x0d` (`48->46`), `0x0f` (`46->40`), `0x10` (`40->32`), `0x14` (`32->28`), `0x15` (`28->20`), `0x16` frame 1 (`20->10`) | `0x45afa`, `0x45c16`, `0x45ca2`, `0x46034`, `0x460d8`, `0x461a4` | `10` |
| `sf-ken-02` | `2450..=3550` | Crouching attacks: `0x11` (D+LP), `0x12` (DF+LP), `0x13` (DB+LP), `0x1c` (D+MP), `0x1d` (DF+HP), `0x1e` (DB+MP), `0x17` (D+LK), `0x17` (D+MK), `0x18` (D+HK sweep) whiff far (`2459..2757`), then walk to pin and land all nine (`48->46->44->42->38->30->26->25->21->15`), followed by spaced standing `0x16` landing on its second active frame (`15->5`) | `0x45d44`, `0x4625a`, `0x4630a`, `0x461c4` | `5` |
| `sf-ken-03` | `2450..=3910` | Jump attacks via `0x453b6` (`0x4545c` / `0x45474`): `0x20`, `0x23`, `0x26`, `0x22`, `0x21`, `0x25`, `0x24` whiff (`2483..2972`), then land `0x21` (`48->46`), `0x24` (`46->42`), `0x20` (`42->38`), `0x26` (`38->28`), `0x23` frame 1 (`28->22`), `0x23` frame 2 against forward-jumping Ryu (`22->18`), `0x22` (`18->16`), and `0x25` (`16->12`) | `0x45e2e`, `0x46398`, `0x463b8`, `0x463d8` | `12` |
| `sf-ken-04` | `2450..=3120` | Specials: Shoryuken (`0x19`) and Tatsumaki (`0x1b`, spin loop `0x4554a` / `0x45566`) whiff far (`2522..2724`), then walk to pin and land `0x19` (`48->28`) and `0x1b` (`28->8`) | `0x45f04`, `0x45f64` | `8` |
| `sf-ken-05` | `2450..=3100` | Hadoken (`0x1a`) whiffs under a forward-jumping Ryu and leaves the screen (`2520..2600`), lands unblocked on an idle Ryu (`48->28`, slow-motion event `9`), deals chip damage on a blocking Ryu (`28->20`), and Tatsumaki (`0x1b`) deals two spins of chip damage on a blocking Ryu (`20->12->4`) | `0x45f64` (plus `0x433ba` projectile handler) | `4` |

Across `sf-ken-01..05`, all 16 twin hit functions in
`crates/sf-engine/src/fighter.rs:144-185` land at least once.

## Verification

The acceptance test is `crates/sf-engine/tests/ken_attacks_scene.rs`, run for
all five scenarios:

- **Repeatability (`C-REF`):** two independent MAME reference captures per
  scenario (`ken-01..05-run-a` and `-run-b`) match on every frame and
  checkpoint from `2450` to the scenario's end.
- **Observables (`C-NAT`):** the native run from the checkpoint-2450 fixture
  (`move-run-a`) matches the reference on per-frame screen hashes, full-pixel
  checkpoints, video RAM, palette RAM, and sound-latch timing, with no
  untranslated branches reached.
- **Owned RAM:** `sf_engine::run::compare_versus_ram` confirms that all owned
  work-RAM and object-RAM regions (including both fighters, effect slots, P1
  and P2 structures with command trackers, and Ken's Tatsumaki spin-loop
  globals `0xff870a` / `0xff870c`) are byte-identical to the reference after
  every frame, and non-owned work RAM differs only in task-slot saved context
  words and stack memory (`0xff8006..=0xff8631`).
- **Twin coverage:** `all_sixteen_twin_hit_functions_and_ken_helpers_are_exercised`
  verifies that all 16 twin hit functions land, every move `0x0d..0x26` (except
  unused `0x0e` and `0x1f`) both whiffs and hits, `0x453b6` maps through both
  `0x4545c` and `0x45474`, and `0x4554a` / `0x45566` cycle `0xff870a` /
  `0xff870c`.
- **Negative check (`C-NEG`):** `negative_check_catches_a_shifted_ken_input`
  shifts one of Ken's inputs by one frame and confirms that both
  `compare_versus_ram` and `sf_compare::compare::compare` reject the run.

## Not covered

- **Blocked normal, crouching, and jump attacks from Ken:** Ryu only blocks
  Ken's Hadoken and Tatsumaki in `sf-ken-05`, so Ken's blocked medium/heavy
  kick recoil streams (`0x3028`, `0x3192`, `0x3172` in `0x460d8`, `0x461a4`,
  `0x461c4`) are translated but not exercised here.
- **Ken's KO of Ryu:** Ryu's health stays positive in all five scenarios so
  the steady-state fight loop does not exit into round-end resolution.
