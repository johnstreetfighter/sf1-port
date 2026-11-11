# Crouching attacks

This document records the boundary established by
issue #18: Ryu's crouching
punches and kicks with down, down-forward and down-back held, landing on Ken
and missing him, and Ken's reactions while he crouches. All of it runs
natively in the versus fixture. This is a controlled-fixture result, not
complete gameplay.

## The moves

The input handler's down branches (docs/reference/posture-dispatch.md) map
the latched strength through their tables:

| Held | Handler | Punch L / M / H | Kick L / M / H | Notes |
| --- | --- | --- | --- | --- |
| Down | 0x40812 | 11 / 1c / 1c (0x40884) | 17 / 17 / 18 (0x4088c) | Hadoken tracker reset |
| Down-forward | 0x40894 / 0x4098e | 12 / 1d / 1d (0x40976) | 17 / 17 / 18 (0x40986 / 0x40a7e) | the kick table is tried first; a punch also clears the kick latch, a kick the punch latch |
| Down-back | 0x40894 / 0x4098e | 13 / 1e / 1e (0x4097e) | 17 / 17 / 18 | |

Ryu's move table is at 0x1605c:

| Move | Stream | Hit function | Notes |
| --- | --- | --- | --- |
| 11 / 12 / 13 | 0x00b92 / 0x00c14 / 0x00c56 | 0x45d26 | light punches |
| 1c / 1d / 1e | 0x00d38 / 0x00d18 / 0x00d38 | 0x45d26 | medium and heavy punches; first entry calls 0x455fc |
| 17 | 0x01116 | 0x46250 | light and medium kick |
| 18 | 0x01158 | 0x46300 | sweep; whiff sound 0x454c6 |

**The medium/heavy punch hold (0x455fc).** The stream's first entry calls
this routine. It holds the first frame by the attack's strength level, from
the table at 0x45620: 0, 6, 9 for light / medium / heavy.

- **With a hold**, the stream pointer moves past the entry and the
  animation step restarts (jmp 0x42be2): the frame is held, and +0x30
  counts twice that frame.
- **Without a hold**, the control-byte loop continues (jmp 0x42c20).

The animation loop now distinguishes these continuations from the usual
"load the next frame" (0x42cae), as `fighter::Resume`.

**Hit functions.** Each has a twin for the other fighter, and a block is
only the crouching block (+0xa == 8):

| Hit function | Used by | Damage table |
| --- | --- | --- |
| 0x45d26 / 0x45d44 | punches | 0x45de0: 16 bytes per strength level (+0x34), then 8 per depth |
| 0x46250 / 0x4625a | kicks 17 | 0x462e0, same layout |
| 0x46300 / 0x4630a | sweep 18 | 0x4637e, by depth only |

The spark templates follow the strength (0x1855c, or 0x1857c above level 3;
0x1863c; 0x1865c).

## Scenarios and outcomes

There are five new versus scenarios, all from the checkpoint-2450 fixture
with checkpoints every frame.

- **"Held"** is the direction the handler read on the attack's first frame.
- **A hit** means Ken's health dropped during the move.
- **Ken +9 / reaction** is his state when hit. 0x8c with reactions
  0x3a–0x3c is his crouching reaction (posture bits +9 & 7 = 4, reaction
  entries +3 / +6 of the damage entry).

| Scenario | Interval | Content |
| --- | --- | --- |
| sf-crouch-attacks-01 | 2450–3570 | down: 11, 1c (M), 1c (H), 17 (L), 17 (M), 18 whiff far; at the pin 11, 1c, 1c HIT, 17 (L) whiff, 17 (M), 18 HIT |
| sf-crouch-attacks-02 | 2450–3570 | down-forward: 12, 1d, 1d, 17, 17, 18 whiff far; 12, 1d, 1d HIT, 17 (L) whiff, 17 (M), 18 HIT |
| sf-crouch-attacks-03 | 2450–3570 | down-back: 13, 1e, 1e, 17, 17, 18 whiff far; 13, 1e, 1e HIT, 17 (L) whiff, 17 (M), 18 HIT |
| sf-crouch-attacks-04 | 2450–3340 | Ken crouching (P2 down 2540–3239). Standing LP whiff; crouching LP HIT (Ken 8c, reaction 3a); standing MK whiff; crouching MK HIT (3b); standing HK HIT (3c); crouching HK HIT (3a); crouching HP HIT after he stood up (89, reaction 33) |
| sf-crouch-attacks-05 | 2450–2940 | crouching LK at the pin with down, down-forward, down-back: HIT each |

Every crouching strength in every posture both hit and missed. The light
kick hits in 05 and misses in 01–03.

## Verification

The acceptance test is `crates/sf-engine/tests/crouch_attacks_scene.rs`,
run for each of the five scenarios:

- **Repeatability:** the two reference runs are identical.
- **Observables:** the native run matches every observable: per-frame
  screen hashes; checkpoint pixels and video/palette RAM every frame;
  sound-latch timing.
- **Owned RAM:** byte-identical to the reference after every frame. This
  covers Ken's own crouch (P2 input through the translated handler), his
  crouching reactions, health, score, sparks, the sound queue and the
  trackers.
- **Translated paths only:** no untranslated or unverified branch is
  reached.

**P2 input (new).** The versus machine now takes P2's directions and six
buttons (IN1 high byte; P2 HP / HK on IN0 0x400 / 0x100), and scenarios
script them as `p2_*`.

## Not covered

- **Crouching blocks** (#20, #54): the low-block branches are covered in
  `docs/reference/blocking.md`.
- **Shoryuken** from the down-forward punch (#21).
- **Ken's attacks.** Their twin hit functions are translated, but Ken never
  attacks here (#47).
- **Knock-downs:** no scenario takes Ken's health to zero.

## One-player crouching attacks against Retsu (issue #54)

Issue #54 (`F14`) adds
**`sf-1p-crouch-01`** (`2389..=3223`, checkpoints every frame,
`tools/reference/scenarios/sf_1p_crouch_01.json`) on the `sf-cpu-01` boot
path (`DF at 2389, with the live CPU`):

- **Missing (`2390..2690`):** Ryu crouches (`p1_down` held) and presses each of
  the six attack buttons in turn (`LP` at 2391 -> `(0x11, 1)`, `MP` at 2436 ->
  `(0x1c, 2)`, `HP` at 2483 -> `(0x1c, 3)`, `LK` at 2557 -> `(0x17, 1)`, `MK`
  at 2602 -> `(0x17, 2)`, `HK` at 2647 -> `(0x18, 3)`). All six crouching
  normals whiff without damaging Retsu (`48` HP).
- **Landing (`2696..3203`):** Ryu crouches and lands each of the six crouching
  normals unblocked on Retsu, walking right at `2789..2809` and `3133..3153` to
  stay in range:
  - `LP` at 2697 (`(0x11, 1)`): hits (`48 -> 46` HP);
  - `MP` at 2810 (`(0x1c, 2)`): hits (`46 -> 42` HP);
  - `HP` at 2862 (`(0x1c, 3)`): hits (`42 -> 34` HP);
  - `LK` at 2965 (`(0x17, 1)`): hits (`34 -> 33` HP);
  - `MK` at 3016 (`(0x17, 2)`): hits (`33 -> 29` HP);
  - `HK` at 3154 (`(0x18, 3)`): hits (`29 -> 23` HP).
- The scenario ends at frame `3223` before its match decision (`Ryu 10` HP,
  `Retsu 23` HP).

Verified in `crates/sf-engine/tests/one_player_breadth_scene.rs` (`C-REF`,
`C-NAT`, `C-NEG`).
