# Shoryuken and Tatsumaki

This document records the boundary established by
issue #21 and extended by
issue #53: Ryu's Shoryuken
and Tatsumaki, started from the input handler's command trackers, in the
versus fixture (`CF`: a controlled-fixture result) and against Retsu in
one-player mode (`DF` at 2389, with the live CPU).

## The commands

The trackers (0x40360–0x403b2, tracker routine 0x40ae4) run on every frame
of the input handler, translated with ticket #15
(docs/reference/posture-dispatch.md). Their RAM has matched the reference
in every scenario since. Each completes when its third direction arrives,
and stays done while that direction is held, within 0x12 frames of the
first.

| Special | Command | Fires from | Move | Move header |
| --- | --- | --- | --- | --- |
| Shoryuken | F, D, DF + punch | the down-forward punch branch (0x40918 / 0x40a10) | 0x19 | +9 0xc1, sound 0x89, invulnerable (+0xf bit 3) |
| Tatsumaki | D, DB, B + kick | the back kick branches (0x40690 / 0x407b4) | 0x1b | +9 0xc1, sound 0x8b |

- **Shoryuken:** a completed command turns the down-forward punch into
  0x19, then clears the tracker's timer and done bytes.
- **Tatsumaki:** the forward punch's Hadoken check (0x40798) also falls
  through into this check (hadoken.md).

**Shoryuken (stream 0x1200).** The first frames are active, with hit
function 0x45efa (Ken 0x45f04). There is no block test: the hit always
lands, with the fixed damage entry 0x45f52 (score 0x50, damage 20). The
header's invulnerability flag makes the judge (0x43e82) skip any attack
against Ryu while it is set.

**Tatsumaki (stream 0x13ea).**

- **The spin loop.** Frame 0x146a carries the slot-swap flag 0x20
  (docs/reference/slot-swap.md). The entry at 0x14aa calls 0x4550a, which
  stores the entry's count byte (4) at 0xff8704 and the next frame
  (0x14b0) at 0xff8706; Ken's twin 0x4554a uses 0xff870a / 0xff870c. The
  entry at 0x1516 calls 0x45526 (Ken 0x45566), which loops back to the
  stored frame while the count is not zero, giving five spins.
- **Whiff sound.** Each spin's active frame has hit function 0x45f5a and
  the whiff sound 0x454c6.
- **Hit function** (0x45f5a / 0x45f64), deciding by the defender's
  +9 & 0x28:
  - **0x08** (in hit stun): no hit, so later spins cannot hit a reeling
    opponent.
  - **0x20** (the block moves: +9 0x22 standing, 0x24 crouching): chip
    damage. In 2P versus (game mode 1) this is entry 0x4601a (damage 8).
    Otherwise it is 0x46012 plus the stage's byte of the table at 0x45fa8
    (8, 8, 16, 16, 0, 0xff by 0xffc003); a 0 gives only the block sound.
  - **Anything else:** the full hit, entry 0x46012 (damage 20).

## Scenarios and outcomes

There are four versus scenarios from the checkpoint-2450 fixture and one
one-player scenario (`sf-1p-tatsumaki-01`, ticket #53) from the
checkpoint-2389 fixture of the `sf-cpu-01` boot path, all with checkpoints
every frame. "Ken hp" / "Retsu hp" is opponent health (48 at the start).

| Scenario | Interval | What the reference shows |
| --- | --- | --- |
| sf-specials-01 | 2450–3060 | Shoryuken LP and HP far from Ken: whiffs. At the pin: hit (48→28). Against Ken holding back (block move 4): hit anyway (28→8) |
| sf-specials-02 | 2450–2860 | Shoryuken command with the down-forward 24 frames after the forward: no special, a down-forward medium punch (0x1d) lands (48→44). A Shoryuken started as Ken throws a heavy kick: Ken's kick does no damage (Ryu stays at 48), and the Shoryuken hits Ken (44→24) |
| sf-specials-03 | 2450–3120 | Tatsumaki LK far: whiff (five spins, 2476–2587). Tatsumaki HK at the pin: hit (48→28; the later spins meet Ken in hit stun). Kick pressed 30 frames after the first direction: no special, a back medium kick (0x15) |
| sf-specials-04 | 2450–2720 | Tatsumaki MK against Ken holding back: chip damage (48→40) with his reaction 0x30 |
| sf-1p-tatsumaki-01 | 2389–2649 | Tatsumaki HK at 2390 (starts 2406): hits Retsu unblocked at 2438 (48→28). Tatsumaki HK at 2510 (starts 2520): blocked by Retsu at 2564, reading stage 0 of `0x45fa8` (byte 8 → entry `0x4601a`, chip damage 8: 28→20) |

**Holding back through the gate.** The Tatsumaki only fires if back is
still held when the kick is dispatched. A first recording released back
after 12 frames: the walk-back's gated phase (+9 0x91) delayed the
dispatch past the release, and a neutral kick ran instead. The scenarios
hold back for 22 frames.

## Verification

The acceptance tests are `crates/sf-engine/tests/specials_scene.rs` (versus)
and `crates/sf-engine/tests/one_player_specials_scene.rs` (one-player):

- **Repeatability:** the two reference runs are identical (and
  `sf-1p-tatsumaki-01` equals `sf-cpu-01` on every field through 2389).
- **Observables:** the native run matches every observable.
- **Owned RAM:** byte-identical to the reference after every frame. That
  covers both entities, both player structures with the trackers, the
  Tatsumaki's loop globals (0xff8704–0xff870d, now in the compared
  regions), health, sparks and the sound queue.
- **Negative control (ticket #53):** mutating stage 0 of the one-player chip
  table `0x45fa8` to stage 2's byte (`0x10`, damage 4 instead of 8) fails at
  frame 2564 (the first chip hit).
- **Translated paths only:** no untranslated or unverified branch is
  reached.

## Not covered

- **The Hadoken** (#22, #53): verified separately, see hadoken.md. The forward
  punch's Hadoken-then-Tatsumaki fall-through firing a Tatsumaki is still
  not exercised.
- **Ken's specials:** his twins are translated, but Ken never uses one
  here.
- **Stages 1–5 (`0xffc003 == 1..=5`) of the 1P chip table** (`0x45fa8` by
  stage): stage 0 is exercised at frame 2564 of `sf-1p-tatsumaki-01`; the
  entries for stages 1–5 are translated but not reached.
- **Knock-downs:** a third Shoryuken would KO Ken, and no scenario does.
