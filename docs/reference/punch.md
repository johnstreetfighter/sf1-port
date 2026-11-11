# Standing light punch

This document records the boundary established by
issue #9: one attack button
in the development fight plays Ryu's standing light punch, including hit and
miss, the target's damage and reaction, and the return to a repeatable
state. Unlike the movement, crouch, and jump slices (whose per-frame tables
were translated from observed sprite data), this slice is driven by
**recovered original data and translated original routines**: move headers,
animation streams, damage tables, and reaction tables are read from the
local program ROM, and the code that interprets them is a traceable Rust
translation (`crates/sf-engine/src/fighter.rs`). No captured outcome is
replayed and no original code is executed.

## The bounded interval and the controlled target

**Frames 2450..=3399 of scenario `sf-punch-01`** (950 frames), the same 2P
versus boot path and checkpoint-2450 fixture as [movement](movement.md):
Ryu (P1) against Ken (P2), both defaulting to Japan. **The target is
controlled**: versus mode has no CPU opponent and P2's inputs are never
touched, so Ken only stands, takes hits, and recovers. This is a
controlled-fixture result, not complete gameplay.

P1 throws six neutral standing light punches (button 1 held 12 frames),
walking right between some of them. Distances are the gap between the
fighters' anchor (column-0) x positions, Ken's minus Ryu's, when the punch
starts:

| Press | Punch frames | Gap | Outcome |
| --- | --- | --- | --- |
| 2460 | 2473-2479 | 0x84 | whiff |
| 2580 | 2593-2599 | 0x5c | whiff |
| 2700 | 2713-2719 | 0x34 | **partial hit** on 2716: 1 damage, +100 |
| 2830 | 2843-2849 | 0x2c | **clean hit** on 2846: 2 damage, +100 |
| 2980 | 2993-2999 | 0x4c | whiff (Ken was pushed back 0x20 per hit) |
| 3120 | 3133-3139 | 0x24 | **clean hit** on 3136: 2 damage, +100, one health cell lost; Ken's stun meets the screen edge and the camera scrolls (+0x10 at 3137, +8 at 3162) |

Walks between punches (right, 10 frames each) at 2520, 2640, 2760 (pinned
against Ken), and 3040. Each hit returns Ken to his stance (move 1) 27 or
28 frames later (2744, 2873, 3164; the frame order below decides which);
each punch returns Ryu to his stance (move 1 restarted) 7 frames after it
starts — the state the next punch starts from, which is what makes the test
repeatable: the fifth punch starts straight from the fourth's recovery, the
others from a stance reached after a walk.

Evidence: reference captures `punch-01-run-a` / `punch-01-run-b`
(repeatability: MATCH, 950 frames, 950 checkpoints; byte-identical). Probe
captures and MAME-debugger watchpoint logs under `tools/reference/local/`
(local-only) located every routine below.

## The recovered path

Addresses are in the 68000 program region of the reference set (`sf`,
ROM_LOAD16_BYTE of sfd-19/22/20/23/21/24, assembled by
`sf_video::romset::load_program_rom`).

### 1. Input recognition (vblank, 0x3b3ae)

The reference set is the six-button cabinet, which emulates the original
pressure-sensing pads in software. Each player has a punch group and a kick
group of three buttons. While any button of a group is held, the vblank
routine counts frames (state +3, saturating at 0xff) and keeps the strongest
button (state +4: 1 light, 2 medium, 3 heavy). **On release** — unless the
count saturated — it writes the pad-equivalent strength from the table at
0x3b450 (0x40 / 0xe0 / 0xfe) to the player's latch (P1 punch: 0xff86b0).
This is why the punch starts one frame after the button is *released*
(probe `punch-lat-a`: holds of 1, 5, 30, and 60 frames all started on
release + 1), and why a hold past 255 frames never attacks.

Ports read (sfus_map): IN1 low byte bits 4/5 = P1 buttons 1/2, IN0 high
byte bit 1 = P1 button 3; kicks IN1 bits 6/7 and IN0 low bit 2. Versus mode
runs all four latches (0x3b0a4).

### 2. Attack selection (input handler 0x402f8)

The fight loop runs both players' input handlers before the object
runners (0x3ea80). For a fighter not busy (+9 & 0xc9 gating at 0x403d4)
with no direction held (dispatch table 0x404cc, entry 0 → 0x4052c), the
latched strength goes through the threshold table at 0x405b6 (routine
0x40b72): `10 0d d0 0f f0 10 ff 10` — strengths above 0x10 select move 0x0d
(light, < 0xd0), 0x0f (medium, < 0xf0), or 0x10 (heavy). The latch is
consumed, the command-tracker progress bytes reset (0x4059c), and the move
starts (0x441b0). The handler's tail (0x4050c) drops latched strengths
while an attack is in progress, so presses during a punch are not
buffered.

### 3. Move start (0x441b0) and the animation stream (0x42ad0 / 0x42be2)

Ryu's move table is at 0x1605c (player structure +0x12; Ken's at 0x1662c).
Each move is a 0x30-byte header copied over the fighter's entity (object
RAM 0xffe400; the first 8 bytes of an entity are its first sprite entry):
move 0x0d's header at 0x162cc holds the anticipation pose (code 0x220,
3x2 big sprites), duration 2, state 0xc2, sound command 0x81 (+0x10), and
the stream pointer 0x9ca. The position is kept around the fighter's centre
minus the frame x-offset (+0x26).

On the move's first frame the object runner (0x42ad0) queues the header's
sound (0x81, the swing) and runs separation (0x43d7c), the screen clamps
(0x43c18 / 0x43c9a), and the collision judge (0x43e82). Afterwards the
animation routine (0x42be2) holds each frame for duration + 1 frames and
then loads the next 0x20-byte stream frame (0x442e2: hold count, duration,
code, attr, dy/dx step, grid size, box bytes +0x16..+0x25, flags, x-offset,
hit function), redrawing the grid around the kept centre (0x43b6a, flipped
for a left-facing fighter at 0x4444a) and parking unused entries of the
fighter's 24-entry region. Stream control bytes: 0xfe jumps, 0xfd ends the
move (busy bits cleared; the next input pass restarts the stance, move 1),
0xff calls a routine (not reached here).

The light punch's stream (decoded from ROM):

| Source | Frames | Code | Grid | Notes |
| --- | --- | --- | --- | --- |
| header 0x162cc | 3 (idx 0-2) | 0x220 | 3x2 big | swing sound 0x81 queued on idx 0 → latch on idx 1 |
| 0x9ca | 2 (idx 3-4) | 0x224 | 3x3 big | **active**: flags 0xa0, x-offset 0x10, attack box +0x22..+0x25 = 1c 04 1c 08, hit function 0x45af0 |
| 0x9ea | 1 (idx 5) | 0x063 | 3x2 big | stance pose; the third column parks |
| 0xa0a | 1 (idx 6) | — | — | 0xfd: move ends |

### 4. The collision judge (0x43e82)

Runs every frame of the attacker's object runner. When the attacker's frame
is active (+0xf bit 7) and the defender is not invulnerable (+0xf bit 3),
it compares the attack box (centre + signed x offset +0x24, half-width
+0x25; centre − y offset +0x22, half-height +0x23) with the defender's loose
box (+0x20/+0x21, +0x1e/+0x1f) and tight box (+0x1c/+0x1d, +0x1a/+0x1b),
centres from the sprite grid (0x43be8 / 0x43bda). Outside the loose box is a
miss; inside the loose box but outside the tight box is depth 2 (partial);
inside both is depth 1 (clean). The hit function clears the attacker's
active bit, so the second active frame never hits again.

Standing Ken's boxes (+0x1a..+0x21): `20 08 08 06 00 2e 08 0a`. With Ryu's
attack box this gives: loose-x reach 0x12, tight-x reach 0x0e around the
contact point — at the scenario's gaps: 0x84/0x5c/0x4c miss, 0x34 partial,
0x2c/0x24 clean.

### 5. The hit (0x45af0) and its effects

- **Block check**: a defender in move 4 only gets the block sound (8).
- **Damage entry** 0x45b6e + (depth & 1) * 8: `00 01 30 3a 36 40 48 44`
  (partial) / `00 02 …` (clean): score index 0, damage 1 or 2, then the
  reaction moves selected by the defender's state (+9 & 7: 4 → 0x3a, 1 →
  0x36, otherwise 0x30 — standing Ken has 2) and the matching knock-down
  moves (0x40 / 0x48 / 0x44) used when no health is left.
- **Score** (0x3b9f0): +100 as 7 BCD digits from 0x146d6 into P1 +0x22..,
  redrawn at P1 +0x20 (0x3be6a); a score above the top score replaces it
  (0x3ba84; not reached here — the fixture's top score outranks 300).
- **Health** (0x3c034): P2 +6 remainder (4 per cell) and +7 cells (11):
  damage drains the remainder, borrowing whole cells; the active cell's tile
  comes from 0x3c0a6 indexed by the remainder; lost cells fill with 0x1124.
- **Reaction** (0x4451a): the defender turns to face the attacker, takes
  reaction move 0x30 with stream 0x3c60 and state 0x8a from P2's reaction
  table (0x4473a), queues that entry's sound **0x84** (impact), and its
  current hold is cut short. Ken's stun stream (facing-relative steps
  negated for a left-facing fighter): 0x8c0 for 9 frames (step +0x10),
  0x8c4 for 16 frames (+8), 0x863 for 1 frame (+8), end — a +0x20 pushback,
  invulnerable throughout (+0xf = 0x08), then the stance restarts.
- **Spark** (0x42e1e): template 0x184fc into the first free effect slot
  (0xffe100), at the attack box's contact point. The effect runner
  (0x42d22) queues the template's sound **0x01** on its first frame, holds
  frame 0x0584 (stream 0x18f7a), and frees the slot (position zeroed) 13
  frames after the hit.

### 6. Frame order and the random state

The dispatcher (0x429f2) runs P1's or P2's objects first according to the
parity of byte 2 of the random state (0xff8e4a, table 0x42a4e), as left by
the previous frame. The random task (0x4813e) folds the inverted P2 input
byte into the 8-byte state with increments 3,3,5,7,9,11,13,15 once per pass
of its 7-frame loop (passes at 2452 + 7k; since ticket #14 the phase comes
from the task's position on its stack, docs/reference/task-scheduler.md). When P1 runs first, Ken's
reaction starts on the hit frame itself (2846: the stun pose shows on the
hit frame); when P2 runs first, Ken has already run and the stun shows from
the next frame (2716 → 2717, 3136 → 3137). The translated state reproduces the
reference's random bytes on every frame.

### 7. Sound commands

All commands go through the 16-word queue (0x3b7c8, 0xff8662); each vblank
pops one into the sound latch (0x3b12c), so a command queued on frame f is
on the latch on frame f+1 and two queued on the same frame land on
consecutive frames: swing 0x81 on punch idx 1; impact 0x84 on hit + 1;
spark 0x01 on hit + 2. (The jump slice's 0x82 / 0x19 now use the same
queue.)

### 8. Screen clamp and camera push (0x43c9a, 0x4392c)

After each frame load the fighter is clamped to its screen band (facing
right: front extent +0x18 ≤ 0x180, back +0x19 ≥ 0x60; facing left: front
+0x19 ≤ 0x1a0, back +0x18 ≥ 0x80). A clamped fighter leaves its frame step
(+0x15) in +0x36 as a camera request. The camera routine resolves both
fighters' requests: a single request scrolls the camera (0xffc00c, range
0..=0x180; fg_scroll copy 0xffc680), moves the *other* fighter back by it
(0x43b34) and all effect objects with the stage (0x4302c). The sixth hit
exercises it: Ken's +0x10 stun step overshoots 0x1a0 by 0x0a, so he is
clamped and the camera scrolls 0x10, moving Ryu and the spark back 0x10
(Ken's world position changes by +0x16); his stance-recovery step repeats
it with +8 at 3162.

## Native implementation

- `crates/sf-engine/src/fighter.rs`: the translated routines on an
  original-address memory view (`Bus`: work RAM 0xff8000-0xffdfff from the
  fixture dump, object RAM, text RAM, the fg_scroll register, and the
  program ROM as data). Untranslated branches record their address instead
  of guessing (`unrecovered`); the capture runner fails if any is reached,
  and the acceptance test asserts none is.
- `crates/sf-engine/src/versus.rs`: the frame order (vblank latches/IN1
  copy/sound pop, P1 then P2 input, object runners in dispatch order,
  effects, camera, random task). Ryu's walk/crouch/jump stay on the earlier
  tables; when he stands with no direction held, the translated input
  handler takes over, and a started attack runs on the translated runner
  until its stream ends. Ken is always translated.

## Development launcher

`cargo run -p sf-dev --release` (from the crate directory): arrows move as
before, **A** is P1 button 1 (light punch, fires on release). The launcher
replays nothing; it feeds the same logical inputs the capture runner takes
from the scenario. Attacks released while walking, crouching, or jumping
take dispatch branches not recovered here and are ignored (nothing is
guessed); an untranslated branch reloads the fixture.

## Verification

- Reference repeatability: `punch-01-run-a` vs `punch-01-run-b` — MATCH,
  950 frames, 950 checkpoints.
- Native vs reference: `sf-engine capture-punch` vs `punch-01-run-a` —
  MATCH, 950 frames, 950 checkpoints (checkpoint pixels and video/palette
  RAM byte-identical; screen hashes and latch timing equal), with no
  untranslated branch reached.
- Machine state beyond the screen (example `compare_ram`, local data): over
  all 950 frames Ken's entity (0xffea00-0xffefff), the effect slots
  (0xffe100-0xffe3ff), Ryu's sprite entries, both player structures (except
  the untranslated command trackers +0x16..+0x1e), the sound queue and
  indices, the button-latch state, the random state, the camera, the top
  score, and the fg_scroll copy are byte-identical to the reference. Ryu's
  entity fields beyond the sprite words differ from the first walk on,
  because the walk tables (ticket #6) write only sprite entries; the
  movement, crouch, and jump scenarios pass the same check.
- Acceptance test: `crates/sf-engine/tests/punch_scene.rs` (reference
  repeatability, native match, and the hit/miss outcomes, health, score,
  pushback, and camera pushes at the documented frames). Unit tests in
  `fighter.rs` cover the button latch, the strength table, the sound queue,
  and the random step.
- The idle (#5), movement (#6), crouch (#7), and jump (#8) acceptance tests
  pass unchanged with the translated routines attached.

The keyboard path was built but not driven interactively in the recording
session; it calls the same `set_p1_punch` input the replay path uses.

## Not covered by this slice (recorded for the remaining-work map)

- Other attacks: medium/heavy punches (moves 0x0f / 0x10) and kicks run
  through the same translated path but are unverified against the
  reference; attacks with a direction held (dispatch 0x404cc entries 1-10)
  and attacks from a walk, crouch, or jump are not recovered.
- Blocking (move 4 sound-only branch is translated but unexercised),
  crouching/airborne reactions, knock-downs (no health left), special moves
  and their command trackers (0x40360, 0x40ae4), attack chaining (0x42b34,
  0xfd re-latch at 0x42c60), stream routine calls (0xff), projectile
  collisions (0x430b6 / 0x43144 / 0x431c6), frame-flag effect spawns
  (0x3b852), turning to face right (0x4405a is translated, unexercised).
- Opponent behavior: Ken only reacts; no CPU decision is made (ticket #12).
- Ryu's walk/crouch/jump still came from observed tables in this slice, so
  his entity fields were not maintained during them; separation (0x43d7c)
  against a walking Ryu used his last stance boxes. Since ticket #15 they
  come from the translated posture dispatch and the tables are deleted
  (docs/reference/posture-dispatch.md).
- Audio synthesis (tickets #10/#11): only the latch commands are produced.
