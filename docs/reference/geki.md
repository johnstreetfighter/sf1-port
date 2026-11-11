# Ryu beats Geki and reaches the bonus-stage boundary

Issue #67 continues `sf-win-02` from Geki's round-1 start through his defeat
and the country-end dispatch. This is a development-fixture continuation
(**DF**): it starts from a captured fight-loop checkpoint and follows the
pinned input script, without editing emulated RAM. The implementation is in
`crates/sf-engine/src/fighter.rs` and `crates/sf-engine/src/tasks.rs`.

## Translation

Geki's four hit functions use the shared CPU hit shape, with his own spark,
damage and reaction entries. Three block against defender state `0x22` and
`0x46500` blocks against state `0x24`; Retsu's unblockable attack is represented
by disabling the block-state test.

The stream routines translate Geki's disappearing-smoke effect, shuriken throw
and teleport choice. The throw's two-byte comparison and low-health check
select the three-shuriken branch; every other path selects one template by
centre distance and queues sound `0x20`. The teleport uses the random-nibble
table to select a frame within a `0x20`-byte group.

The shuriken handlers transform a hitting projectile into template `0x18944`
and apply its inline damage/reaction entry without score or a normal spark.
Projectile-on-projectile contact deflects the shuriken, reverses its horizontal
step and queues sound `0x22`.

After the second fight in a country, `0x3ef96` clears the fight counter and
selects a target from the stage table at `0x3efb0`. The selected target remains
an explicit boundary until that stage is implemented.

## Fixture and verification

`sf-win-02` uses `win-02-run-a` and `win-02-run-b`, with factory-default DIPs
`DSW1=0xdfff` and `DSW2=0xffff`. The Geki window has checkpoints on every frame
from `5860` through `9278`. Native replay starts at fight-loop checkpoint `5860`
and applies the scenario inputs.

| Frame | Observed transition |
| --- | --- |
| 5860 | Geki's round-1 fight loop starts |
| 7077 | Ryu wins round 1; P1 wins change `0 → 1` and event `7` begins |
| 7777 | Geki's round-2 fight loop starts |
| 8412 | Ryu wins round 2; P1 wins change `1 → 2` and event `7` begins |
| 8908 | Geki's match-won screen, event `0xb` |
| 9180 | the match-end wipe, event `6` |
| 9277 | stage/fight/winner/round are `0/1/1/1` |
| 9278 | country-end dispatch reports stage-0 target `0x40d30`; fight, winner and round are cleared |

The two reference runs repeat exactly over all 3,417 post-fixture frames
`5861..=9277`. Native capture comparison matches every captured screen hash and
sound-latch observation, with checkpoint pixels and video/palette data checked
on every frame. The owned-RAM comparison is byte-identical after each of those
3,417 frames. At frame `9278`, native execution stops with only `0x40d30`
unrecovered. A direct replay independently checks the same dispatch and the
cleared match counters.

`crates/sf-engine/tests/geki.rs` contains the reference-repeat, observable,
RAM, boundary and landmark checks. The named negative control changes the
recorded pixel CRC in the shuriken-handler checkpoint at `7534`; comparison
rejects that checkpoint at the same frame.

## Not covered

- Hit functions `0x46492`, `0x4656e` and `0x465dc` are exercised; `0x46500`
  is translated but not exercised by `sf-win-02`.
- Stream routine `0x4594c` is translated but not exercised by this fixture.
- For `0x4596e`, the reference exercises the `Less` timer-comparison path and
  both distance-selected single-throw directions. It does not exercise
  `Equal`, the `Greater`/health selection, or the low-health triple throw.
- Both shuriken handlers, `0x43348` and `0x43396`, are exercised. The `+8` bit-3
  early-exit branch of `0x43348` is translated but not separately exercised.
- Only the stage-0 country-end target is exercised. The other stage-table
  targets and the bonus-stage implementation beginning at `0x40d30` are not
  covered by this ticket.
