## Problem Statement

The user wants to play the original arcade Street Fighter on Windows through a native Rust port that preserves the original game's behavior. A remake judged by resemblance or an emulator executing the original gameplay program would not satisfy this goal. The repository currently contains project decisions and verified local ROM inputs, but no Rust implementation or runnable behavior-comparison harness.

## Solution

Recover and translate US set 1, with six-button controls, into native Rust. Preserve the complete arcade experience, including original progression and two-player modes, with remappable keyboard controls. Start with a development launch directly into one fixed fight, then restore the complete arcade flow. Keep initial use personal and local; defer distribution decisions.

Judge fidelity against a pinned MAME version running verified reference ROMs. Match gameplay outcomes and timing, original game pixels before scaling, and sound events, timing, pitch, and character. Preserve original quirks and bugs; any later improvements must be optional.

## User Stories

1. As the player, I want a Windows-native port, so that I can play the original game on my PC.
2. As the player, I want US set 1 with six-button behavior, so that the port has one clearly defined reference version.
3. As the player, I want the original movement and facing behavior, so that positioning works as it does in the original game.
4. As the player, I want original jump and crouch behavior, so that movement options retain their timing and limits.
5. As the player, I want each original normal attack and strength, so that the six-button controls retain their meaning.
6. As the player, I want the original special-move recognition and execution, so that the controls and their quirks remain faithful.
7. As the player, I want original hit detection, damage, blocking, and reactions wherever the reference supports them, so that combat outcomes match.
8. As the player, I want original opponent decisions and timing, so that fights preserve their challenge.
9. As the player, I want original health, timer, round results, and transitions, so that a fight follows the original rules.
10. As the player, I want exact original game pixels before scaling, so that sprites, backgrounds, palettes, animation, and HUD preserve the original appearance.
11. As the player, I want original sound events, timing, pitch, and character, so that actions and scenes sound faithful.
12. As the player, I want original music behavior, so that playback and transitions match the game.
13. As the player, I want keyboard controls for the first playable milestone, so that no additional controller is required.
14. As the player, I want remappable keyboard controls in the completed port, so that I can choose usable bindings.
15. As a local player, I want the original two-player modes supported with keyboard controls, so that the completed port includes that part of the arcade experience.
16. As the player, I want the original startup and title flow, so that the completed port begins like the reference version.
17. As the player, I want original credit and start behavior, so that the arcade flow is preserved.
18. As the player, I want original attract behavior, so that the idle arcade experience is preserved.
19. As the player, I want full original progression and all its required content, so that I can play the complete game.
20. As the player, I want original continuation and ending behavior wherever present in the reference, so that game sessions have their original outcomes.
21. As the player, I want original bugs and unusual timing preserved in the baseline, so that fidelity is not silently replaced with redesign.
22. As the project owner, I want personal local use first, so that distribution does not expand the current work.
23. As the developer, I want the verified local game data checked repeatably, so that an incorrect or incomplete set cannot invalidate comparisons silently.
24. As the developer, I want a pinned comparison baseline, so that changes in emulator behavior do not silently change the target.
25. As the developer, I want repeatable starting conditions and frame-indexed inputs, so that differences can be reproduced.
26. As the developer, I want the first differing frame and observable reported, so that a fidelity failure can be investigated efficiently.
27. As the developer, I want a one-frame gameplay difference to fail comparison, so that timing drift is not hidden by visual similarity.
28. As the developer, I want a direct launch into one fixed fight, so that the first playable milestone can be built before the full startup flow.
29. As the developer, I want recovery notes tied to original program addresses and observed behavior, so that translated logic remains auditable.
30. As the developer, I want gameplay and sound-program logic translated into Rust, so that the result is a native port rather than original programs running through CPU emulation.
31. As the developer, I want sound-chip emulation available for synthesis, so that native sound-program logic can retain original sound character.
32. As the developer, I want refactoring backed by passing behavior comparisons, so that improvements to code structure do not change gameplay.
33. As the developer, I want known MAME discrepancies investigated explicitly, so that emulator behavior is not mistaken unquestioningly for original hardware behavior.
34. As the project owner, I want verified facts distinguished from incomplete recovery and untested behavior, so that progress reports remain reliable.

## Implementation Decisions

- Target Windows first while keeping future platform support possible. Linux and macOS delivery are not requirements for this specification.
- Use US set 1, MAME set sf, with six-button controls as the reference version.
- Recover native gameplay and sound-program logic. Sound-chip emulation is permitted for synthesis; execution of the original game and sound programs through emulated CPUs is not the production approach.
- Preserve traceability to original program addresses, data interpretation, timing, and observed behavior. Favor verifiable translation before idiomatic restructuring.
- Restore complete original progression, arcade flow, and two-player modes for the completed port.
- The first playable milestone is one fixed fight, entered through a development launch, with movement, attacks, opponent behavior, damage, exact game pixels, and faithful sound.
- Require keyboard controls initially and remappable keyboard controls in the completed port. Controller support is deferred.
- Keep optional gameplay changes outside the faithful baseline.
- Initial use is personal and local. Do not assume a distribution model or a decision to include original game data in releases.
- Proposed module responsibilities are a native game session, host input/display/audio integration, sound synthesis, and a comparison runner with reference and native adapters. Concrete libraries and internal decomposition remain engineering choices, not approved technology selections.
- Proposed comparison contract: a scenario describes reference identity, machine configuration, documented initialization, frame-indexed logical inputs, a bounded duration, and expected observations. Both adapters expose the same behavioral scenario boundary without requiring identical internal representations.
- MAME checkpoints and native initialization need not share a binary state format. Document the correspondence and relevant counters, random state, input history, and sound state needed for the chosen scenario.
- Native gameplay must advance without using rendered frame output or MAME execution as its implementation. Static graphics data, decoded assets, and reference captures can support diagnosis but cannot substitute for recovered logic.
- Fix and report comparison configuration, including relevant machine settings and timing assumptions. The pinned source used to verify ROM definitions does not itself pin a runnable MAME build.

## Testing Decisions

The user approved the scenario comparison boundary and publication of this specification on 2026-10-15.

- Prefer one high-level scenario replay/comparison boundary over tests coupled to internal translated functions.
- A good test checks externally observable behavior for known starting conditions and inputs. It does not assert implementation structure, renamed variables, or incidental memory layout.
- Exercise the native game session together with input handling, rendering, and sound-program behavior through the scenario boundary. Use a narrow host smoke check for actual keyboard/window/audio integration.
- First establish deterministic reference replay: repeated runs of the same reference scenario must agree before treating it as a regression oracle.
- Match gameplay outcomes and timing with no one-frame allowance. Compare exact game pixels before host display scaling.
- Compare sound events and their timing, pitch, and character. Byte-identical host audio is not required. Define scenario-specific measurable checks and retain listening review for qualities those checks cannot establish.
- Test the comparison runner with deliberately altered frames or events so a false all-pass result is detectable.
- Report the first mismatch, expected and actual observations, baseline identity, and the scenario needed to reproduce it.
- Scenarios must distinguish a controlled development fixture from an actual complete fight. A stationary target or frozen opponent is useful evidence for a bounded slice but cannot satisfy opponent behavior or the first playable milestone.
- As recovery expands, cover movement, normal attacks, special moves, defensive and damage interactions, opponent behavior, round outcomes, original arcade flow, progression, and two-player modes. Do not declare full-game fidelity from a few passing captures.
- Record and investigate known baseline discrepancies; never silently weaken expectations to accept unexplained divergence.
- No implementation tests or existing test seams are present. The prior ROM verification is input evidence, not a gameplay test or feasibility proof.
- Finish each slice with its focused behavioral checks. Broaden tests when new behavior or a shared change warrants it.

## Out of Scope

- Street Fighter II, later entries, other ROM revisions, or pressure-sensitive cabinet variants.
- A remake with newly designed gameplay or a production CPU-emulation wrapper around original game programs.
- Mandatory original arcade hardware captures as the initial verification baseline.
- Byte-identical output audio.
- Gamepad or arcade-stick support in the initial or completed-port scope described here.
- Linux/macOS delivery, network multiplayer, new characters, new stages, gameplay rebalancing, and player-facing enhancements such as save-state menus.
- Publishing game data, deciding distribution, or preparing a public release.
- Refactoring for style before behavior comparisons establish correctness.

## Further Notes

The confirmed project brief and architecture decision are the source of the scope above. All 44 required reference entries matched expected size, CRC32, and SHA1 against MAME source commit 774a180df2b6ef9c53e48337792256817c50f768. Archive SHA256: cda57772ef1dab11684e594929a742410cda9ca968f1389d0d75ee71c161c1b5. No ROM execution or native fidelity has yet been demonstrated.

The complete port is the product objective; the first playable fight is an intermediate milestone. Neither is complete when a static scene or isolated attack works.

Approved ticketing strategy: publish a bounded first recovery batch that establishes a trustworthy reference, native display and input, a small combat interaction, and initial sound recovery. Its final ticket measures the remaining work and proposes the next dependency-ordered batch. This does not reduce the complete-port scope or claim that unknown reverse-engineering work is already sized for one agent context.

Later batches must explicitly cover remaining normal attacks and special moves; complete opponent behavior; all round outcomes; complete music and sound behavior; the first-playable milestone audit; startup, title, attract, credit/start, continuation, progression, endings, and two-player behavior; remaining content; keyboard remapping; and a final completeness and fidelity audit. Generate those implementation slices from recovered evidence instead of publishing unbounded “port the rest” tickets.

Publish this specification and approved child tickets with the ready-for-agent label. Native issue dependencies gate child work; the aggregate specification is not itself an unbounded implementation assignment. Keep original ROM data local.
