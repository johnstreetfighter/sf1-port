# High and low blocking

This document records the boundary established by
issue #20: holding back
or down-back against the opponent's attacks, for both players, in the
versus fixture (Ken scripted through P2 inputs). This is a controlled-fixture
result, not complete gameplay.

## The rules

**Entering a block.** The input handler (docs/reference/posture-dispatch.md)
starts a block move only while the opponent's entity state (+9) says it is
attacking:

- **Back held** (facing right: left; facing left: right) starts move 4,
  the standing block, if the opponent's +9 is 0xc2 (a standing attack) or
  0xc1 (a jump attack). Otherwise the fighter walks back.
  - P2's back-held branch is 0x40626 → 0x4063c: block without a tracker
    reset.
  - P1's is 0x40714 → 0x4072a: it resets the Hadoken and Tatsumaki trackers
    and both latches first. The translation keeps this asymmetry.
- **Down-back held** starts move 8, the crouching block, if the opponent's
  +9 is 0xc4 (a crouching attack) or 0xc1 (a jump attack). Otherwise the
  fighter crouches back (move 7). Both players use 0x408da / 0x409b4.

**Whether a block stops the hit.** Each hit function tests the defender's
current move (+0xa):

| Attacks | Hit functions | Blocked by |
| --- | --- | --- |
| Standing | 0x45af0 … 0x461c4 | move 4 only |
| Jump | 0x45e10, 0x4638e … | move 4 only |
| Crouching | 0x45d26, 0x46250, 0x46300 | move 8 only |

A blocked hit queues the block sound 0x08 and nothing else. Blocked medium
and heavy kicks also redirect the attacker's stream into its recoil (0xf4a,
0x10b4 and 0x1094 for Ryu; 0x3028, 0x3192 and 0x3172 for Ken).

**The quirk: a jump attack beats the crouching block it triggers.** A jump
attack (+9 0xc1) makes a down-back guard enter the crouching block, but its
hit function only honours the standing block, so it lands.

## Scenarios and outcomes

There are five new versus scenarios, all from the checkpoint-2450 fixture
with checkpoints every frame. Ken faces left, so his "back" is right.

| Scenario | Interval | What the reference shows |
| --- | --- | --- |
| sf-blocks-01 | 2450–4320 | Ken holds back against Ryu's standing LP, MP, HP, LK, MK, HK and a neutral jump MK: block move 4 each time; sound 0x08 on every block that connected (the LP did not reach); MK recoil 0xf4a, HK recoil 0x10b4. Holding back against crouching LP and MK: no block (Ken walks back), both hit (48→46→42). Holding down-back against crouching LP, MP, LK and the sweep: crouching block 8, sound 0x08, no damage |
| sf-blocks-02 | 2450–3680 | Ryu holds back against Ken's standing LP, HP (block 4, not reached), MK (blocked, recoil 0x3028) and HK (blocked, recoil 0x3172). Holding back against Ken's crouching LP: walks back, not reached. Holding down-back against Ken's sweep and crouching MP: crouching block 8, sound 0x08. Holding down-back against Ken's standing HP: crouch back 7, not reached |
| sf-blocks-03 | 2450–3360 | Holding back (pressed once Ken's crouching LK / MK had started) does not block them: Ryu walks back and is hit (48→47→43). Standing HKs pass over each side's crouching guard. A neutral jump MK against each low guard starts the crouching block 8 but does not reach |
| sf-blocks-04 | 2450–3400 | Ken holds down-back against each of Ryu's six standing attacks at the pin: he crouches back (7), never blocks, and every attack passes over him |
| sf-blocks-05 | 2450–2600 | Ken holds down-back while Ryu forward-jumps into him with a heavy kick (0x24): Ken enters the crouching block 8 and is hit anyway (48→38, reaction 0x3c) |

**What this shows.**

- **Standing guard:** it blocks standing and jump attacks. It cannot block a
  crouching attack, because it never starts against one and the defender
  is hit while walking back.
- **Crouching guard:** it blocks crouching attacks. Against standing
  attacks it never becomes a block: the defender crouches back and, in
  every case recorded, the attacks pass over.
- **Jump attack against the crouching guard:** the crouching block it
  triggers does not stop it.

## Verification

The acceptance test is `crates/sf-engine/tests/blocks_scene.rs`, run for
each of the five scenarios:

- **Repeatability:** the two reference runs are identical.
- **Observables:** the native run matches every observable.
- **Owned RAM:** byte-identical to the reference after every frame. That
  covers both fighters' block and recoil moves, Ken's attacks through his
  twin hit functions, health, the block sounds and the trackers.
- **Translated paths only:** no untranslated or unverified branch is
  reached.

**Time-over limit.** The versus fixture's TIME runs out at frame 4330
(99 ticks of 19 frames from 2449), and the reference then judges a
time-over, which is round flow (#30). The first recording of sf-blocks-01
ran past it and diverged exactly there. Versus intervals are now refused at
or beyond that frame (`run::VERSUS_TIME_OVER`).

## Not covered

- **Special moves against blocks:** Shoryuken is unblockable, Tatsumaki
  does chip damage (#21, #53), and the Hadoken has its own projectile block
  handling (#22, #53).
- **Time-over and the round result** (#30).

## One-player blocking against Retsu and held inputs (issue #54)

Issue #54 (`F14`) adds two
one-player scenarios on the `sf-cpu-01` boot path (`DF at 2389, with the live
CPU`):

### `sf-1p-block-01` (`2389..=2700`, `tools/reference/scenarios/sf_1p_block_01.json`)

Ryu steps right (`2390..2400`), holds back (`p1_left` over `2434..2530`), and
then holds down-back (`p1_left + p1_down` over `2530..2680`):

- **High block (`move 4`, `+9 = 0x22`):** While holding back over `2434..2530`,
  Ryu enters standing block (`move 4`) at 2435 and blocks Retsu's jump-in kick
  `0x23` (`0x46850`, frame 2461), standing punch `0x0f` (`0x466d0`, frame 2487)
  and standing kick `0x14` (`0x46790`, frame 2514), each queueing block sound
  `0x08` with zero damage (`Ryu 48` HP).
- **Low block (`move 8`, `+9 = 0x24`) and wrong block hit:** While holding
  down-back over `2530..2680`, Retsu's attacks that set `+9 = 0xc4` (`0x45a96`
  in move `0x18`) or `+9 = 0xc1` (`0x45a7a` in move `0x23`) cause Ryu's
  `down_back` handler (`0x408da` / `0x409b4`) to enter crouching block (`move 8`,
  `+9 = 0x24`) at frames `2539..2547`, `2571..2579` and `2647..2672` (between
  attacks Ryu crouches back in `move 7`, `+9 = 0x14`). Because Retsu's move
  `0x18` (`0x468de`) has no block check and his other six hit functions
  (`0x4664a..0x46850`) check only for standing block (`+9 == 0x22`), Retsu's
  move `0x18` lands on Ryu's crouching block (`move 8`, the wrong block) at
  frames `2548` (`48 -> 40` HP) and `2580` (`40 -> 32` HP), while the low block
  over `2647..2672` completes untouched.
- The scenario ends at frame `2700` before its match decision (`Ryu 32` HP,
  `Retsu 48` HP).

### `sf-1p-held-01` (`2389..=2680`, `tools/reference/scenarios/sf_1p_held_01.json`)

Exercises the simultaneous-direction and long-button-hold paths listed in
`docs/reference/milestone-audit.md`:

- **Both horizontal directions held (`2395..2410`):** `p1_left` and `p1_right`
  held together produce direction nibble `d0 = 3` in `0x402f8` (`!0xff868f & 0xf == 3`),
  taking the dispatch table's `0x4050c` (`input_tail`) entry while Ryu is in
  interruptible stance (`move 1`, `+9 = 0x12`).
- **Both vertical directions held (`2415..2430`):** `p1_up` and `p1_down` held
  together produce direction nibble `d0 = 12` (`!0xff868f & 0xf == 12`),
  taking the dispatch table's `0x4050c` (`input_tail`) entry while Ryu is in
  interruptible stance (`move 1`, `+9 = 0x12`).
- **Button held past 255 frames (`2390..2666`):** `p1_lp` is held for 276
  frames (`> 255`), saturating the punch group's hold counter at `0xff8693` at
  `0xff` (`LatchPath::Held { saturated: true, .. }`) from frame `2646` through
  `2666`. After Ryu blocks Retsu (`p1_left` over `2435..2650`) and returns to
  idle stance (`move 1`, `+9 = 0x12`), releasing `p1_lp` at frame `2667` takes
  `LatchPath::Release { saturated: true }` in `0x3b3ae`, clearing `0xff8690..0xff8694`
  without writing a strength to `0xff86b0` or starting a punch.
- The scenario ends at frame `2680` before its match decision (`Ryu 34` HP,
  `Retsu 48` HP).

Verified in `crates/sf-engine/tests/one_player_breadth_scene.rs` (`C-REF`,
`C-NAT`, `C-NEG`).
