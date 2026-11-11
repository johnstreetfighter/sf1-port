# Jump attacks

This document records the boundary established by
issue #19: Ryu's punch and
kick attacks from neutral, forward and back jumps, landing on Ken and
missing him, and Ken's airborne reactions. All of it runs natively in the
versus fixture. This is a controlled-fixture result, not complete gameplay.

## How a jump becomes a jump attack

1. **Buffering** (0x42b34, translated with ticket #9). During a jump
   (entity +9 = 0x81), a latched punch strength above 0x10 moves into the
   entity (+0x34). Otherwise a kick strength moves into +0x35. Then
   +9 |= 0x40, giving 0xc1.
2. **The apex call** (0x452e0 Ryu, 0x453b6 Ken; translated as
   `fighter::Bus::jump_attack`). Each jump stream calls it at its apex.
   - With +9 != 0xc1 it just skips its entry (ticket #15).
   - Otherwise the jump (move 9 / 0xa / 0xb) selects an 8-byte entry of the
     punch table, or failing that the kick table. An entry holds a
     (threshold, move) strength table and a stream table.
   - The buffered strength is mapped through the strength mapper's preloaded
     entry (0x40b8e), which sets +0x34 to the level and clears the byte at
     A0. Inside the object runner A0 is the player structure, so even a
     mapped kick clears the punch latch; the translation keeps this.
   - The mapped move replaces +0xa, a mapped kick clears +0x35, and the
     stream continues at the stream table's word for the level (`movea.w`,
     sign-extended).
3. **Re-latch on landing** (0x42c4a, translated with ticket #9). A strength
   buffered after the apex has no effect in the air. At the jump stream's
   end it is written back to the latch, and the input handler starts the
   standing attack on landing.

**Tables (Ryu: punch 0x45386, kick 0x4539e; Ken: 0x4545c, 0x45474).** The
strength tables are shared at 0x194b8–0x194e0. Ryu's stream tables are at
0x194e8–0x19530.

| Jump | Punch L / M / H | Stream | Kick L / M / H | Streams |
| --- | --- | --- | --- | --- |
| Neutral (9) | 20 / 20 / 20 | 0x1584 | 23 / 23 / 26 | 0x17fc / 0x17fc / 0x1904 |
| Forward (0xa) | 21 / 21 / 21 | 0x166c | 24 / 24 / 24 | 0x19f2 |
| Back (0xb) | 22 / 22 / 22 | 0x1734 | 25 / 25 / 25 | 0x1aba |

The moves 0x20–0x26 have no headers of their own: the jump's header stays,
and only +0xa and the stream change.

## Hit functions

Each hit function has a twin for the other fighter. All are blocked by the
standing block (+0xa == 4) and use a strength-indexed damage entry (16 bytes
per level, then 8 per depth).

| Hit function | Used by | Damage table | Spark |
| --- | --- | --- | --- |
| 0x45e10 / 0x45e2e | jump punches | 0x45eca | 0x1859c for move 0x20, else 0x185bc |
| 0x4638e / 0x46398 | neutral light / medium kick (frame 1), heavy kick | 0x46462 | 0x1867c |
| 0x463ae / 0x463b8 | neutral light / medium kick (frame 2) | 0x46462 | 0x1869c |
| 0x463ce / 0x463d8 | forward and back jump kicks | 0x46462 | 0x186bc |

`fighter::hit_function` records each as the strength-indexed shape with its
spark rule (`Spark::ByMove` for the jump punch).

## Scenarios and outcomes

There are seven new versus scenarios, all from the checkpoint-2450 fixture
with checkpoints every frame. A hit means Ken's health dropped during the
move. The reaction is Ken's reaction move when hit: 0x37–0x39 are his
airborne reactions (posture bits +9 & 7 = 1, reaction entries +4 / +7).

| Scenario | Interval | Attacks (move: outcome) |
| --- | --- | --- |
| sf-jump-attacks-01 | 2450–3580 | neutral far: 20, 23, 26 whiff; at the pin: 20 HIT, 20 HIT, 20 whiff, 23 HIT, 23 HIT, 26 HIT; LP after the apex → standing 0d on landing HIT |
| sf-jump-attacks-02 | 2450–3590 | forward: 21 HIT, 21 whiff, 24 whiff, 24 whiff; far: 21 whiff, 24 whiff |
| sf-jump-attacks-03 | 2450–3210 | back from the pin: 22, 25, 25 whiff; with Ken walking in (P2 left): 22, 22 whiff, 25 HIT |
| sf-jump-attacks-04 | 2450–3160 | Ken jumping (P2 up): standing 10 HIT (reaction 39), standing 16 HIT (39), neutral jump 20 HIT (38), standing 14 HIT (37) |
| sf-jump-attacks-05 | 2450–2560 | forward jump LK (24) from the fixture distance: HIT |
| sf-jump-attacks-06 | 2450–2560 | forward jump HK (24): HIT |
| sf-jump-attacks-07 | 2450–2920 | at the stage-left edge with Ken walked in: back jump LP (22) HIT, back jump HP (22) HIT |

Every jump-attack move (0x20–0x26) both hit and missed:

| Move | Hit | Whiff |
| --- | --- | --- |
| 20 | 01, 04 | 01 |
| 21 | 02 | 02 |
| 22 | 07 | 03 |
| 23 | 01 | 01 |
| 24 | 05, 06 | 02 |
| 25 | 03 | 03 |
| 26 | 01 | 01 |

The landing re-latch is shown in 01, and airborne reactions in 04.

## Verification

The acceptance test is `crates/sf-engine/tests/jump_attacks_scene.rs`, run
for each of the seven scenarios:

- **Repeatability:** the two reference runs are identical.
- **Observables:** the native run matches every observable: per-frame
  screen hashes; checkpoint pixels and video/palette RAM every frame;
  sound-latch timing.
- **Owned RAM:** byte-identical to the reference after every frame. This
  includes Ken's own jumps through his apex call's unbuffered path.
- **Translated paths only:** no untranslated or unverified branch is
  reached.

## Not covered

- **Blocked jump attacks** (#20, #54): covered in `docs/reference/blocking.md`.
- **Ken's jump attacks:** his apex tables and twin hit functions are
  translated, but Ken never attacks here (#47).
- **Cross-up facing:** a jump attack after crossing over Ken is not
  separately checked. Facing during jumps does run through the translated
  routines, and the forward jumps of sf-jump-02 cross over.
- **Knock-downs:** no scenario takes Ken's health to zero.

## One-player jump attacks against Retsu (issue #54)

Issue #54 (`F14`) adds
**`sf-1p-jump-01`** (`2389..=2957`, checkpoints every frame,
`tools/reference/scenarios/sf_1p_jump_01.json`) on the `sf-cpu-01` boot path
(`DF at 2389, with the live CPU`), exercising neutral, forward and back jump
punches and kicks (`0x20..=0x26`):

- **Back jump (`0xb`):** `p1_up + p1_left` at 2390 with `LP` at 2396 (`0x22`,
  level 1, whiff); `p1_up + p1_left` at 2449 with `HK` at 2455 (`0x25`,
  level 3, whiff).
- **Neutral jump (`0x9`, missing):** `p1_up` at 2515 with `LP` at 2521 (`0x20`,
  level 1, whiff); `p1_up` at 2570 with `LK` at 2576 (`0x23`, level 1, whiff);
  `p1_up` at 2625 with `HK` at 2631 (`0x26`, level 3, whiff).
- **Forward jump (`0xa`, landing):** `p1_up + p1_right` at 2691 with `HP` at
  2697 (`0x21`, level 3, hits Retsu: `48 -> 40` HP); `p1_up + p1_right` at
  2752 with `HK` at 2758 (`0x24`, level 3, hits Retsu: `40 -> 32` HP).
- **Neutral jump (`0x9`, landing):** after walking right at `2807..2822`,
  `p1_up` at 2822 with `HP` at 2828 (`0x20`, level 3, hits Retsu: `32 -> 24`
  HP); `p1_up` at 2882 with `HK` at 2888 (`0x26`, level 3, hits Retsu:
  `24 -> 14` HP).
- The scenario ends at frame `2957` before its match decision (`Ryu 48` HP,
  `Retsu 14` HP).

Verified in `crates/sf-engine/tests/one_player_breadth_scene.rs` (`C-REF`,
`C-NAT`, `C-NEG`).
