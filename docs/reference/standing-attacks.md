# All standing normal attacks

This document records the boundary established by
issue #17: Ryu's six standing
attacks, from neutral and with a direction held, landing on Ken and missing
him. They are reproduced natively in the versus fixture (2P versus, Ken never
touched). This is a controlled-fixture result, not complete gameplay.

## The moves

Each attack button is latched on release by the six-button emulation
(0x3b3ae). The resulting strength is mapped by the input handler's branch
for the held direction (docs/reference/posture-dispatch.md):

| Direction | Handler | Punch table | Kick table |
| --- | --- | --- | --- |
| Neutral | 0x4052c | 0x405b6 | 0x405be |
| Forward (right, facing right) | 0x405c6 | 0x406de | 0x406e6 |
| Back (left, facing right) | 0x406ee | 0x40802 | 0x4080a |

Every table gives the same moves: punches 0d / 0f / 10, kicks 14 / 15 / 16.
A forward or back attack first resets the Hadoken and Tatsumaki trackers and
both latches (0x406ac / 0x407d0). A neutral one resets only the Hadoken
tracker. Each move runs on the translated object runner from Ryu's move
table (0x1605c):

| Move | Attack | Stream | Hit function | Damage table | Spark | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| 0d | LP | 0x009ca | 0x45af0 | 0x45b6e | 0x184fc | ticket #9 |
| 0f | MP | 0x00a6e | 0x45c0c | 0x45c88 | 0x1853c | |
| 10 | HP | 0x00af0 | 0x45c98 | 0x45d16 | 0x1853c | |
| 14 | LK | 0x00e00 | 0x4602a | 0x460a8 | 0x185bc | |
| 15 | MK | 0x00e62 | 0x460b8 | 0x46154 | 0x185dc | whiff sound 0x454c6 |
| 16 | HK | 0x00fac | 0x46164, then 0x46184 | 0x46240 | 0x1861c / 0x185fc | whiff sound; slot swap (slot-swap.md) |

**Damage entries.** Each entry is 8 bytes: score index, damage, three
reactions and three knock-downs. Depth 1 (inside the tight box) uses the
second entry, depth 2 the first.

| Attack | Damage, depth 2 | Damage, depth 1 |
| --- | --- | --- |
| LP | 1 | 2 |
| MP | 6 | 6 |
| HP | 8 | 8 |
| LK | 4 | 4 |
| MK | 6 | 8 |
| HK | 8 | 10 |

Ken's health starts at 48 (11 cells of 4 plus 4).

### The hit-function shapes

The fourteen hit functions of the standing attacks (seven per fighter) are
near-copies of two shapes. Each has its own spark template and damage table, and each has a
twin for the other fighter's move data (Ken's), differing only in its
constants. They are translated as one routine per shape
(`fighter::hit_function` maps each original address to its shape and
constants):

- **Basic** (0x45af0, 0x45c0c, 0x45c98, 0x4602a and their twins
  0x45afa, 0x45c16, 0x45ca2, 0x46034):
  - A defender in the standing block move (+0xa == 4) gets only the block
    sound 0x08.
  - Otherwise the hit lands: score (0x3b9f0) and its redraw (0x3be6a),
    damage (0x3c034), the defender's reaction (0x4451a), the top-score
    check (0x3ba84), and the spark (0x42e1e).
  - The attacker's active bit clears either way.
  - The medium-punch copy omits the `clr.w D0` before loading the damage
    byte. The damage routine reads only D0's low byte, so it makes no
    difference.
- **Blocked kick** (0x460b8, 0x46164, 0x46184 and their twins 0x460d8,
  0x461a4, 0x461c4): as basic, except that a block also sends the
  attacker's stream to a recoil (0xf4a, 0x10b4, 0x1094).

The block branches are translated, but no scenario here blocks. Blocking is
ticket #20.

## Scenarios and outcomes

There are five new versus scenarios, all from the checkpoint-2450 fixture
with checkpoints every frame.

- **"Held"** is the P1 direction the input handler read on the attack's
  first frame (the IN1 copy at 0xff868f).
- **A hit** means Ken's health dropped during the move. Ken's reaction
  sometimes outlasts the next attack's start, so reaction state alone is not
  a hit test.

| Scenario | Interval | Attacks (move, held, outcome) |
| --- | --- | --- |
| sf-normals-01 | 2450–3360 | 0f / 10 / 14 / 15 neutral whiff; 0f, 10 neutral HIT; 14 neutral whiff; 15, 16 neutral HIT |
| sf-normals-02 | 2450–3100 | 0d, 0f forward whiff; 10, 0d, 0f, 10 forward HIT; 14 forward whiff; 15, 16 forward HIT |
| sf-normals-03 | 2450–3340 | 14, 15 back whiff; 0d, 0f, 10 back HIT; 14 back whiff; 15, 16 back HIT |
| sf-normals-04 | 2450–3540 | 14 neutral HIT; 14 forward HIT; 14 back whiff; 10, 15, 16 forward whiff; 0d, 0f, 10, 16 back whiff |
| sf-normals-05 | 2450–2820 | 14, 15 back HIT (back pressed on the button-release frame) |

Every strength in every posture both hit and missed. With sf-punch-01
(neutral LP hit and whiff) and sf-swap-01 (neutral HK whiff):

| | LP | MP | HP | LK | MK | HK |
| --- | --- | --- | --- | --- | --- | --- |
| Neutral hit / whiff | punch-01 / punch-01 | 01 / 01 | 01 / 01 | 04 / 01 | 01 / 01 | 01 / swap-01 |
| Forward hit / whiff | 02 / 02 | 02 / 02 | 02 / 04 | 04 / 02 | 02 / 04 | 02 / 04 |
| Back hit / whiff | 03 / 04 | 03 / 04 | 03 / 04 | 05 / 03 | 03 / 03 | 03 / 04 |

**Back light kick.** Holding back before the kick latches starts a back
walk, and its first frame steps 16 px away. A back light kick then misses
at the walk pin (sf-normals-04). Pressed on the release frame, the attack
starts first and lands (sf-normals-05).

## Verification

The acceptance test is `crates/sf-engine/tests/standing_attacks_scene.rs`,
run for each of the five scenarios:

- **Repeatability:** the two reference runs are identical.
- **Observables:** the native run matches every observable. These are
  per-frame screen hashes; checkpoint pixels and video/palette RAM on every
  frame; and sound-latch timing (swing, whiff, impact and spark commands).
- **Owned RAM:** the memory the translated routines own is byte-identical to
  the reference after every frame (`run::compare_versus_ram`). That covers
  both entities, both player structures (health, score, trackers), effect
  slots (sparks), the sound queue, latches, the random state, the camera
  and the top score.
- **Translated paths only:** the run never reaches an untranslated or
  unverified branch.

## Not covered

- **Blocked standing attacks** (#20). The block branches and recoil streams
  are translated but not exercised.
- **Ken's attacks.** The twin hit functions are translated, but Ken never
  attacks in these scenarios.
- **Knock-downs.** No hit here takes Ken's health to zero, so the KO
  reaction tables are not reached.
- **Move 0e** (hit function 0x45b7e): not reachable from input; the CPU
  uses it.
- **Attacks from a crouch or a jump** (#18, #19) and special moves
  (#21, #22).
