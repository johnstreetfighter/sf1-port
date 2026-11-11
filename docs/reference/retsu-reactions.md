# Retsu takes hits, blocks and is knocked out

This document records the boundary established by
issue #27: the CPU
opponent on the receiving end. Ryu's attacks land on Retsu through the CPU
reaction path (0x447c8) and its tables, Retsu blocks through the follow-up
decision's reaction branch (0x4571a, translated by ticket #25 and first
exercised here), and Ryu knocks him out. Code:
`crates/sf-engine/src/fighter.rs` (`react`), `cpu.rs` and `run.rs` (P1
played on the one-player machine). See cpu-follow-up.md for the decision
and ai-tables.md for the tables.

## The CPU reaction path (0x447c8)

The defender's reaction (0x4451a) turns the defender to face the attacker,
picks the reaction move pair from the damage entry by the defender's state
(+9 & 7: 4, 1, or anything else), and then branches on the defender's
player structure (+0x38): 0xff86b0 and 0xff86da take the human players'
tables (0x4461e / 0x446f6); anything else is the CPU opponent, 0x447c8:

1. the CPU structure's countdown (+0x19) is cleared;
2. the opponent index is stage × 2 + fight (0xffc003 / 0xffc004, the fight
   added as a byte), × 4;
3. **alive** (health +6 | +7 not zero): the move is the pair's first; its
   entry is the per-opponent table from the pointer table at 0xffc050
   (0x1a128: Retsu's is 0x195a8), at (move − 0x30) × 4;
   **knocked out**: +8 = 8, the move is the pair's second, the entry from
   the table at 0xffc054 (0x1a1c8: Retsu's is 0x195dc), at (move − 0x40)
   × 4, and the landing sound slot +0x12 = 9;
4. the entry: the stream word (zero-extended; the human players' tables
   go through `movea.w`, sign-extended), the state byte to +9 (an alive
   entry whose state has bit 0 also sets +0x12 = 0x19), then +8 = 2;
5. the common tail (0x445d6): the entry's sound, +0xe cleared, and the
   hold cut short.

Retsu's alive table pairs stream, state and sound as 0x4f68 / 0x8a / 0xa4
for move 0x30 and on; his knock-down table starts at 0x559c / 0x8c / 0xa5
for move 0x40. Both are read from the ROM through the fixture's pointers,
which are now in the compared RAM.

## P1 on the one-player machine

The one-player machine takes P1's logical inputs (`set_p1_direction`,
`set_p1_buttons`: the active-low P1 byte of IN1 and HP / HK in IN0, as on
the versus machine), and the runners apply a scenario's P1 events before
each step (`run_cpu_fight_capture_with_inputs`,
`compare_cpu_ram_with_inputs`); without a scenario P1 stays idle, as
before.

## Verification

Two scenarios on the sf-cpu-01 boot path (1P Ryu vs CPU Retsu, round 1,
the checkpoint-2389 fixture), checkpoints every frame, two reference runs
each (`crates/sf-engine/tests/retsu_reactions.rs`):

**sf-retsu-01, 2389–3400: standing normals.** Ryu stays at his start
position and presses a button every 24 frames from 2434 (HP, HK, MP, MK,
LP, LK in turn) while Retsu comes in. Retsu's reactions:

| Frames | What happens |
| --- | --- |
| 2434–2466 | Retsu's opening attack (0x18) lands on Ryu (2448) |
| 2510–2559 | Retsu crouch-blocks (move 8) Ryu's kick: the decision's reaction branch, a record from the ground-attack table 0xffc164 |
| 2602 | a punch (move 0x10) lands: reaction 0x33 through 0x447c8, 8 damage (11 cells + 4 → 10) |
| 2678 | a kick (0x15) lands: reaction 0x39, 6 damage (→ 8 + 2) |
| 2754, 2803, 3028, 3121 | crouching blocks |
| 2852, 2981 | standing blocks (move 4) |
| 3178 | a punch (0x10) lands: reaction 0x33 (→ 6 + 2) |
| 3318 | Retsu's 0x14 lands on Ryu (→ 7 cells) |

**sf-retsu-02, 2389–3400: Shoryukens to the KO.** Ryu walks in and throws
a heavy Shoryuken every 60 frames from 2462. Ryu's state during it (0xc1)
counts as airborne, so Retsu's reactions come from the airborne-attack
table 0xffc0e4:

| Frames | What happens |
| --- | --- |
| 2722–2769 | Retsu crouch-blocks the Shoryuken |
| 2783 | a Shoryuken lands: reaction 0x3c, 20 damage (→ 7 cells) |
| 2838 | another lands: reaction 0x35 (→ 2 cells) |
| 2881, 2981 | standing blocks (the first with a 2-frame countdown) |
| 3018 | the third lands with no health left: **Retsu is knocked out** — +8 = 8, reaction 0x43 from the knock-down table (stream 0x56b4), +0x12 = 9 |
| 3020 | the fight loop sees the KO and takes the round-end path (0x3eb22) |
| 3072 on | the player's round-win result (0x3fcea; translated by ticket #31, match-win.md) through the capture's end at 3400 |

Each matches the reference throughout:

- **Reference repeatability:** the two runs are identical.
- **Observables:** every one matches on 2390–3400 in both —
  per-frame screen hashes, checkpoint pixels and video/palette RAM on
  every frame (Retsu's stun, block and fall sprites, the health bars), and
  sound-latch timing (the block and impact sounds from the tables).
- **Owned RAM:** byte-identical after every frame: object RAM, both
  player structures, the AI tables and the reaction table pointers
  (0xffc050–0xffc057, now compared), the round timer, the scheduler, the
  sound queue, the random state and the camera. (The KO run first stopped
  at 3072 at the player's round-win branch 0x3fcea; ticket #31 translated
  it, and the run now matches through 3400.)

## Not covered

- **The round result after a player's win** (0x3fcea on): verified by
  ticket #31 (match-win.md).
- **Retsu's reactions from state 4 or 1** (the first two pair columns):
  he is hit standing (state 2) in these runs.
- **A reaction record's jump or walk**: the records Retsu drew here are
  blocks (streams 0xc890 / 0xc900).
- **Other opponents' tables**: read through the same pointers, not
  exercised.
