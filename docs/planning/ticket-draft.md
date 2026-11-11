# Approved first recovery batch

The user approved this bounded initial batch on 2026-10-15. The full-port specification and tickets #2 through #13 are published with ready-for-agent labels. All 13 issue bodies and labels and all 15 native blocking relationships were verified after publication. This batch is not the complete first playable milestone or full implementation backlog.

## Approved testing seam

One high-level scenario replay/comparison boundary: documented initial conditions and frame-indexed inputs in; gameplay events, original-resolution game pixels, and sound observations out. Reference and native adapters may use different internal state representations. A narrow host smoke check covers actual keyboard, window, and audio integration. The user confirmed this seam on 2026-10-15.

## Overview

1. **Reproduce one fixed fight in a pinned MAME baseline** — Blocked by: none. Run one bounded, repeatable reference scenario from verified local ROMs and capture the observations needed to compare later native slices.

2. **Run a scenario comparison and prove that it detects divergence** — Blocked by: T1. Use a Rust command-line runner to validate local input data, consume a reference scenario, and report the first observable mismatch.

3. **Display the fixed fight's first frame from native asset decoding** — Blocked by: T2. Launch a Rust Windows window displaying the selected fight's initial game frame using original graphics data and documented scene state.

4. **Advance a bounded native idle scene with matching timing** — Blocked by: T3. Step a short controlled scene in native Rust and match its idle animation and relevant timer behavior frame by frame.

5. **Move the player left and right from keyboard input** — Blocked by: T4. Control native horizontal movement from the keyboard and reproduce reference positions, facing behavior, and animation for bounded movement scenarios.

6. **Reproduce crouch and return to standing** — Blocked by: T5. Use the keyboard to crouch and stand with the reference's state transitions, animation, and timing.

7. **Reproduce one complete jump and landing** — Blocked by: T5. Trigger a native jump with keyboard input and reproduce its ascent, descent, and landing for the selected reference trajectory.

8. **Land one light punch against a controlled target** — Blocked by: T5. Press one attack button in the development fight and reproduce a single selected standing light-punch interaction, including hit/miss and target response.

9. **Play one recovered hit sound at the correct event time** — Blocked by: T8. Hear the selected punch-hit sound in the native development fight, driven by translated sound-program logic and permitted sound-chip synthesis.

10. **Play one bounded phrase of the fixed fight's music** — Blocked by: T2. Run a native sound-program slice that plays one identified music phrase from the fixed fight and compare it with the reference.

11. **Recover one decision of the fixed opponent** — Blocked by: T5. Advance one selected opponent decision from a documented state in the native scene and match its observable action and timing.

12. **Size the next recovery batch from the working native slices** — Blocked by: T6, T7, T9, T10, T11. Produce an evidence-based next ticket proposal that covers the remaining first-fight work and maps every full-release requirement to later bounded recovery work.

## Publication record

The specification was published first, followed by one issue per ticket in dependency order. Each ticket links to the specification in its body; the parent issue was not modified after creation. Native GitHub blocking relationships represent the listed edges. T1 through T12 correspond to GitHub issues #2 through #13 respectively; publication.json records their URLs and verification results.

## T1: Reproduce one fixed fight in a pinned MAME baseline

### Parent

historical issue #1

### What to build

Run one bounded, repeatable reference scenario from verified local ROMs and capture the observations needed to compare later native slices.

### Acceptance criteria

- [ ] Select and record a runnable MAME version, its build identity, reference ROM checks, relevant machine configuration, and exact fixed fight; explain the selection using observed behavior.
- [ ] Provide a repeatable local command that initializes the scenario, supplies frame-indexed inputs, and captures original game pixels and relevant gameplay/sound observations.
- [ ] Repeat the same scenario from a fresh initialization and verify identical deterministic observations; identify nondeterministic or unavailable observations explicitly.
- [ ] Record how a native initialization can represent the same relevant starting conditions; do not assume MAME save states can be loaded by native code.
- [ ] Keep ROMs and any data-bearing captures local; separate tooling and metadata from original game data.

### Blocked by

None (can start immediately).

## T2: Run a scenario comparison and prove that it detects divergence

### Parent

historical issue #1

### What to build

Use a Rust command-line runner to validate local input data, consume a reference scenario, and report the first observable mismatch.

### Acceptance criteria

- [ ] Establish the minimal Rust executable and one documented invocation for the selected scenario with clear invalid-ROM, absent-input, and incompatible-baseline errors.
- [ ] Define the high-level scenario boundary for initial conditions, logical inputs, observations, and baseline identity without coupling it to translated function names.
- [ ] A control comparison of identical reference observations passes; a deliberately shifted gameplay event and a changed game pixel each fail at the expected frame.
- [ ] Emit a readable first-mismatch report and a nonzero result for failed comparisons; a reference-versus-reference self-check is explicitly identified as such.
- [ ] Record the boundary and test invocation for subsequent slices; no native game behavior is claimed yet.

### Blocked by

- T1

## T3: Display the fixed fight's first frame from native asset decoding

### Parent

historical issue #1

### What to build

Launch a Rust Windows window displaying the selected fight's initial game frame using original graphics data and documented scene state.

All implementation slices inherit the confirmed fidelity and native-translation requirements. Use the scenario comparison boundary, document original-address traceability, and keep original data local. A fixture-only result must be labelled as such; unsupported paths must remain explicit rather than silently approximated.

### Acceptance criteria

- [ ] Decode the relevant local original graphics data and reproduce the selected fixture frame's layers, sprites, palette, and HUD from scene state.
- [ ] Compare the game-resolution output exactly against the reference before display scaling.
- [ ] The displayed scene is reconstructed rather than a reference screenshot used as the renderer's output.
- [ ] Document original data addresses and scene-state interpretation used by the slice, and identify any fixture-only initialization.
- [ ] Show a reproducible Windows launch and clean exit; this static scene does not count as a playable fight.

### Blocked by

- T2

## T4: Advance a bounded native idle scene with matching timing

### Parent

historical issue #1

### What to build

Step a short controlled scene in native Rust and match its idle animation and relevant timer behavior frame by frame.

All implementation slices inherit the confirmed fidelity and native-translation requirements. Use the scenario comparison boundary, document original-address traceability, and keep original data local. A fixture-only result must be labelled as such; unsupported paths must remain explicit rather than silently approximated.

### Acceptance criteria

- [ ] Choose and document a bounded reference interval that isolates the selected idle behavior; any development-fixture controls are applied equivalently and disclosed.
- [ ] Translate only the state updates needed for this interval, retaining original-address traceability and relevant counters.
- [ ] Replay repeated runs through the comparison boundary and match the interval's exact game pixels and relevant gameplay event timing.
- [ ] Demonstrate that native state changes drive the output; replaying captured frames or executing original game code through a CPU emulator does not satisfy the slice.
- [ ] Report uncovered behavior explicitly rather than extending the slice into full opponent AI or complete round logic.

### Blocked by

- T3

## T5: Move the player left and right from keyboard input

### Parent

historical issue #1

### What to build

Control native horizontal movement from the keyboard and reproduce reference positions, facing behavior, and animation for bounded movement scenarios.

All implementation slices inherit the confirmed fidelity and native-translation requirements. Use the scenario comparison boundary, document original-address traceability, and keep original data local. A fixture-only result must be labelled as such; unsupported paths must remain explicit rather than silently approximated.

### Acceptance criteria

- [ ] Map development keyboard bindings to frame-indexed logical direction inputs and record/replay the same logical input stream.
- [ ] Cover left, right, release-to-neutral, and an observed boundary condition in the chosen fixture.
- [ ] Translate the required movement and facing paths with original-address notes; compare their game pixels and timing through the shared scenario boundary.
- [ ] Show the behavior in the Windows development launch, including release behavior so held keys cannot remain stuck after input reset.
- [ ] Use controlled opponent conditions explicitly; do not present this as complete collision, AI, or keyboard remapping.

### Blocked by

- T4

## T6: Reproduce crouch and return to standing

### Parent

historical issue #1

### What to build

Use the keyboard to crouch and stand with the reference's state transitions, animation, and timing.

All implementation slices inherit the confirmed fidelity and native-translation requirements. Use the scenario comparison boundary, document original-address traceability, and keep original data local. A fixture-only result must be labelled as such; unsupported paths must remain explicit rather than silently approximated.

### Acceptance criteria

- [ ] Capture and reproduce one standing-to-crouch-to-standing sequence and the relevant direction interaction observed in the chosen fixture.
- [ ] Match the externally visible transitions and exact game pixels without timing tolerance.
- [ ] Document the recovered paths and control conditions and demonstrate the sequence in the native development launch.
- [ ] Keep crouching attacks and unrelated combat branches outside this slice; record them for the remaining-work map.

### Blocked by

- T5

## T7: Reproduce one complete jump and landing

### Parent

historical issue #1

### What to build

Trigger a native jump with keyboard input and reproduce its ascent, descent, and landing for the selected reference trajectory.

All implementation slices inherit the confirmed fidelity and native-translation requirements. Use the scenario comparison boundary, document original-address traceability, and keep original data local. A fixture-only result must be labelled as such; unsupported paths must remain explicit rather than silently approximated.

### Acceptance criteria

- [ ] Select one observed jump trajectory and reproduce its full sequence from the documented fixture state.
- [ ] Match motion, animation, and takeoff/landing timing through the comparison runner.
- [ ] Demonstrate a second jump after landing to verify return to the expected state.
- [ ] Document unimplemented jump directions or attack interactions for later tickets instead of silently accepting approximations.

### Blocked by

- T5

## T8: Land one light punch against a controlled target

### Parent

historical issue #1

### What to build

Press one attack button in the development fight and reproduce a single selected standing light-punch interaction, including hit/miss and target response.

All implementation slices inherit the confirmed fidelity and native-translation requirements. Use the scenario comparison boundary, document original-address traceability, and keep original data local. A fixture-only result must be labelled as such; unsupported paths must remain explicit rather than silently approximated.

### Acceptance criteria

- [ ] Trace the reference's selected attack from input recognition through animation, collision decision, damage, and the bounded target reaction.
- [ ] Demonstrate both a hit and a miss from documented starting distances; match their event timing and exact game pixels.
- [ ] Show the target health change and reaction, including the recovered return state needed to repeat the bounded test.
- [ ] Run from keyboard input as well as replayed logical input; document original addresses and controlled target conditions.
- [ ] Do not substitute hardcoded captured outcomes for recovered attack rules, or expand this ticket to all attacks, blocking, or complete opponent AI.

### Blocked by

- T5

## T9: Play one recovered hit sound at the correct event time

### Parent

historical issue #1

### What to build

Hear the selected punch-hit sound in the native development fight, driven by translated sound-program logic and permitted sound-chip synthesis.

All implementation slices inherit the confirmed fidelity and native-translation requirements. Use the scenario comparison boundary, document original-address traceability, and keep original data local. A fixture-only result must be labelled as such; unsupported paths must remain explicit rather than silently approximated.

### Acceptance criteria

- [ ] Trace the chosen reference sound command through the relevant sound-program behavior and document the recovered addresses and timing.
- [ ] Translate the bounded program path into Rust; the production path does not execute the original sound program through an emulated CPU.
- [ ] Use an evaluated sound-chip implementation or model for synthesis and record its provenance, integration constraints, and configuration.
- [ ] Trigger the sound from the native hit event; compare event timing and scenario-specific pitch/duration characteristics and provide a repeatable listening comparison.
- [ ] Do not require byte-identical host audio or declare all sound behavior complete from this one effect.

### Blocked by

- T8

## T10: Play one bounded phrase of the fixed fight's music

### Parent

historical issue #1

### What to build

Run a native sound-program slice that plays one identified music phrase from the fixed fight and compare it with the reference.

All implementation slices inherit the confirmed fidelity and native-translation requirements. Use the scenario comparison boundary, document original-address traceability, and keep original data local. A fixture-only result must be labelled as such; unsupported paths must remain explicit rather than silently approximated.

### Acceptance criteria

- [ ] Select a bounded phrase with documented initial sound state, commands, timing, and a clear stopping point.
- [ ] Translate only the required sound-program path into Rust with original-address notes, using sound-chip synthesis as allowed by the architecture decision.
- [ ] Provide a runnable local audio demonstration and machine-checkable event/timing observations through the comparison tooling.
- [ ] Record a listening comparison for pitch and character alongside measurable checks; disclose unimplemented loops, transitions, or channels.
- [ ] Choose sound-chip integration consistently with any already-landed sound slice; do not invent a blocking dependency on gameplay or the hit effect.

### Blocked by

- T2

## T11: Recover one decision of the fixed opponent

### Parent

historical issue #1

### What to build

Advance one selected opponent decision from a documented state in the native scene and match its observable action and timing.

All implementation slices inherit the confirmed fidelity and native-translation requirements. Use the scenario comparison boundary, document original-address traceability, and keep original data local. A fixture-only result must be labelled as such; unsupported paths must remain explicit rather than silently approximated.

### Acceptance criteria

- [ ] Use the fixed opponent selected by the reference scenario and bound the slice to one observed decision branch and its visible outcome.
- [ ] Record all relevant initial conditions, including any random or timer state that influences the branch.
- [ ] Demonstrate the resulting native movement or action through the development launch and scenario comparison.
- [ ] Retain original-address traceability and identify adjacent branches not yet recovered.
- [ ] Do not claim a complete AI opponent or a playable full fight from one matching decision.

### Blocked by

- T5

## T12: Size the next recovery batch from the working native slices

### Parent

historical issue #1

### What to build

Produce an evidence-based next ticket proposal that covers the remaining first-fight work and maps every full-release requirement to later bounded recovery work.

### Acceptance criteria

- [ ] Demonstrate the completed slices and enumerate exactly which behaviors pass, fail, or remain unimplemented.
- [ ] Use recovered call paths and data dependencies to identify the smallest remaining slices for all first-fight attacks, special moves, defense/damage interactions, opponent behavior, round outcomes, and full fight audio.
- [ ] Map the complete-port requirements to later work: startup/title/attract and credit/start flow, progression and remaining content, continuation/endings, two-player behavior, keyboard remapping, and full-game verification.
- [ ] Give each proposed next ticket a demoable or verifiable outcome, concrete acceptance criteria, and only genuine blocking edges; split any broad behavior that exceeds a fresh agent context.
- [ ] Identify any native recovery feasibility blocker explicitly. Do not substitute emulation or approximate behavior without an explicit change to the confirmed scope.
- [ ] Present the next breakdown for approval using the to-tickets workflow; this planning ticket does not certify the first playable milestone or complete port.

### Blocked by

- T6
- T7
- T9
- T10
- T11
