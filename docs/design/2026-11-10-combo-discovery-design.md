# Combo discovery for new fighters

## Intent

Find which normals lead into a new fighter’s signature moves and measure how much timing variation each route allows. Use the results to tune the moveset and choose routes for Training trials. This is a developer tool. Trial selection and moveset edits stay with the designer.

## Approaches

1. **Shortlist from frame data, then replay.** Startup, hitstun and cancel rules can narrow the list quickly. The shortlist’s coverage depends on how well it predicts reach, late contact and input buffering.
2. **Sweep actual fight inputs.** Run every supported pair in the real engine and measure which activation frames produce a true combo. This runs more attempts and accounts for the rules, spacing and input handling together. The maintainer chose this approach for moveset tuning and trial selection.

## First pass

- Run the selected fighter’s native **SF1+** moveset in **Definitive Overhaul**, starting at close range against both a standing and a crouching defender. Use a two-player Local fight so the tool can drive both pads. The reference setup uses Ken as the defender on one fixed stage at mid-screen. Settle into a reproducible close position and record the actual separation. Ken uses neutral inputs when standing and holds Down when crouching.
- Try each implemented normal with a reproducible close-range input sequence, including standing, crouching, jumping and command normals where available. The designer selects the signature moves to test. Check each supported strength separately. Reuse move-list input definitions where they fit and accept an authored input sequence for other commands. A fighter must have a playable moveset and its normal game assets loaded before the sweep can measure it.
- Start each attempt from the same pre-input state for that setup. Run the normal and one signature move through the game’s input handling. Sweep the signature move’s activation input one frame at a time, from the earliest buffered timing the tested sequence allows through the end of the opener’s hitstun. Measure each attempt relative to the normal’s first clean hit. Keep the rest of the command recipe fixed so the result describes a repeatable technique. Inputs before contact have negative offsets.
- Verify a clean hit from the intended normal and a clean hit from the intended follow-up during the opener’s hitstun. Attribute the follow-up hit to the signature move, accounting for extra hits from the opener and for projectiles that arrive after the attacker has recovered. Where the engine exposes the transition, show whether the follow-up canceled the normal or started after recovery.

## Results and playback

- Report the normal, signature move and strength, defender stance, starting separation, era style, ruleset preset and tested input sequence. Save all combo timing windows and the length of the widest one. For example, `−1 to +2` is a four-frame window measured from the normal’s hit. Report pairs with no successes under the tested inputs and mark unsupported inputs separately. Each result applies to its recorded setup and command sequence.
- Save one replay per successful pair and stance. Pick a frame near the middle of the widest window. Store the fight setup, build and asset identifiers, both pads’ exact inputs from setup through the combo, and the expected moves and hit frames. My proposed delivery is a developer command that can play the route in the existing developer window or verify it headlessly. Keep replay artifacts local.
- Verify the tool first with the existing Ryu crouching light kick into Hadoken case. Exercise a timing that falls outside the combo window and confirm a saved replay reproduces its recorded hits. The implementation can choose the report format, replay schema and how to reuse in-memory snapshots for the sweep.
