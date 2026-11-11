# Approved overhaul ticket batch (issue #91)

The user approved this 21-ticket breakdown (`O1`–`O21`) on 2026-10-27 for the [Definitive Street Fighter (1987) Overhaul & Toggleable Multi-Era Suite specification](https://github.com/johnstreetfighter/sf1-port/issues/1).

Each ticket is a vertical tracer-bullet slice published to GitHub Issues with the `ready-for-agent` label, a `## Parent` link to `#91`, and native GitHub `blocked_by` dependency edges. Non-invasive diagnostic overlays and deterministic state snapshots (`O1` and `O2`) have no gameplay blockers and can start immediately during Phase 1 faithful recovery; all gameplay-altering **core toggles** and **overhaul** tickets (`O3`–`O21`) are gated by `#90` (*Re-check waves 3–16 and publish them*) so that the **complete arcade experience** and **gameplay fidelity** of the **reference version** are verified first.

---

## O1: Real-time Hitbox, Loose/Tight Hurtbox, and Command-Tracker Diagnostic Overlay

### Parent

https://github.com/johnstreetfighter/sf1-port/issues/1

### What to build

Provide a non-invasive, real-time diagnostic overlay in the development launcher and Training Mode that visualizes the 1987 collision geometry and input-parser state on top of any live or replayed fight without altering simulation state, pixel rendering of the underlying game frame, or audio output.

Players and developers can toggle visual bounding boxes showing the 1987 engine's two-stage hurtbox check—the outer loose bounding box (`depth 2`) and the inner tight/critical hurtbox (`depth 1`)—alongside active melee attack boxes, projectile collision boxes, character pushboxes, and invulnerability state indicators. Alongside the collision boxes, a live Command-Tracker and Walk-Gate Inspector displays the current button hold-duration counters, negative-edge release latches, special-move command-tracker progress and remaining countdown timers for *Hadoken*, *Shoryuken*, and *Tatsumaki Senpukyaku*, and whether the fighter is currently inside a gated walk animation frame.

### Acceptance criteria

- [ ] Toggle the collision and input diagnostic overlay on and off during any live or replayed fight without mutating underlying work RAM, PRNG state, sound latches, or base `384×224` game-frame pixels.
- [ ] Render distinct color-coded rectangles for the outer loose hurtbox (`depth 2`), inner tight hurtbox (`depth 1`), active melee and projectile hitboxes, pushboxes, and invulnerability states for both fighters and active projectiles on every frame.
- [ ] Display live per-player telemetry for button hold durations, negative-edge release triggers, special-move command-tracker stages (0 through completed), remaining command window timers, and walk-cycle gate states.
- [ ] Expose overlay telemetry through the frame-stepped scenario session boundary and verify in automated scenario tests that active hitbox/hurtbox coordinates and command-tracker states match the reference fight's collision and input checks frame by frame.
- [ ] Verify that all existing **comparison baseline** acceptance tests continue to pass with zero frame, pixel, RAM, or audio diffs.

### Blocked by

None (can start immediately).

---

## O2: 60-Cell Visual Frame Meter and Deterministic Replay Takeover

### Parent

https://github.com/johnstreetfighter/sf1-port/issues/1

### What to build

Add a 60-cell color-coded Visual Frame Meter and deterministic **Replay Takeover** to the diagnostic and Training Mode suite.

The 60-cell Visual Frame Meter tracks the rolling last 60 frames for both Player 1 and Player 2, color-coding each frame cell as startup, active, recovery, hitstun, blockstun, invulnerable, actionable cancel window, or neutral, and automatically computing and displaying the numeric frame advantage (`+` or `-`) after every hit or block interaction. Deterministic snapshot serialization allows any fight or recorded scenario replay to be saved and restored at any frame boundary; using **Replay Takeover**, a player watching a replay can pause at any frame, select Player 1 or Player 2, and immediately take live control of that fighter for a configurable window (default 10 seconds) while the opponent continues replaying their recorded inputs, then rewind to the takeover point to test another response.

### Acceptance criteria

- [ ] Render a 60-cell rolling frame meter for both fighters showing per-frame startup, active, recovery, hitstun, blockstun, invulnerable, and cancel-window states, plus calculated numeric frame advantage on hit and block.
- [ ] Support deterministic full-session state snapshot save and restore at any frame boundary across the scenario session seam.
- [ ] Verify snapshot determinism in automated round-trip tests: saving a snapshot at frame $N$, advancing 60 frames to $N+60$, restoring the snapshot at $N$, and re-simulating with identical inputs produces bit-identical state, pixel hashes, and audio latches at $N+60$.
- [ ] Enable **Replay Takeover** during any scenario or match replay so a player can pause, take live control of either fighter from that exact frame with branched inputs, and reset back to the takeover bookmark cleanly.
- [ ] Verify zero regression across all **comparison baseline** acceptance tests when running in `Faithful 1987` mode.

### Blocked by

- O1

---

## O3: MatchConfig, RulesetPreset, and Positive-Edge Input + Recovery Buffer Core Toggles

### Parent

https://github.com/johnstreetfighter/sf1-port/issues/1

### What to build

Establish the match configuration model, the five top-level **ruleset presets** (`Definitive Overhaul`, `Faithful 1987`, `Classic Competitive`, `Pure Footsies (1987+)`, and `Custom`), and the first two Tier 1 **core toggles**: positive-edge attack registration (`positive_edge_attacks`) and the recovery input buffer (`recovery_input_buffer`).

When `Faithful 1987` is selected, all **core toggles** are off and the game executes with 100% **gameplay fidelity** to the **reference version** (where normal and special attacks wait for button release, holds saturate at 255 frames, and inputs pressed during recovery are discarded). Once the faithful **port** milestone is complete, `Definitive Overhaul` becomes the default **ruleset preset**. Enabling `positive_edge_attacks` fires normal and special attacks on the frame a punch or kick button is pressed, removes the 255-frame button-hold saturation lock, and offers an optional negative-edge sub-toggle so special moves can trigger on either button press or button release. Enabling `recovery_input_buffer` retains attack and special inputs pressed within a configurable buffer window (default 4 frames) before the end of attack, hitstun, or blockstun recovery so they execute on the first actionable wakeup or recovery frame.

### Acceptance criteria

- [ ] Provide selection and persistence for the five **ruleset presets** (`Definitive Overhaul` as default once the faithful **port** is complete, `Faithful 1987`, `Classic Competitive`, `Pure Footsies (1987+)`, and `Custom` with individual toggle overrides).
- [ ] With `positive_edge_attacks` enabled, normal and special attacks trigger on button press (`F+1`) rather than waiting for button release, holding a button past 255 frames never locks out subsequent releases, and the optional negative-edge special sub-toggle triggers specials on either press or release.
- [ ] With `recovery_input_buffer` enabled, normal and special inputs entered within the configurable window (default 4 frames) before hitstun, blockstun, or attack recovery ends execute on the first actionable frame instead of being cleared.
- [ ] Prove both **core toggles** via paired differential scenario tests across the session seam (`false` vs. `true` under identical frame-indexed inputs).
- [ ] Verify that selecting the `Faithful 1987` **ruleset preset** passes every MAME 0.289 **comparison baseline** acceptance test with zero frame, pixel, RAM, or audio latch diffs.

### Blocked by

- #90
- O2

---

## O4: Lenient Special Command Trackers, Walk-Gate Interrupts, and Anytime Jump Attacks

### Parent

https://github.com/johnstreetfighter/sf1-port/issues/1

### What to build

Implement the Tier 1 **core toggles** that fix special-move motion execution, walk-cycle input lockouts, and jump-attack timing in the 1987 engine: `lenient_command_trackers`, `interruptible_walk_cycle`, and `anytime_jump_attacks`.

In the **original game**, special-move command trackers reset after 18 frames (`0x12`), clear their completed state immediately if the player leaves the final direction before the button release registers, and cannot start or fire during 8 of every 10 walk-cycle frames; additionally, holding Back while pressing an attack shifts the fighter backward by 16 pixels on the first frame, and jump attacks can only be triggered during the narrow jump apex routine after a 9-frame pre-jump crouch. With these **core toggles** enabled, players get an expanded motion window (default 28 frames), a directional release grace window (default 8 frames) after reaching the final command direction, diagonal shortcut tolerance for *Shoryuken*, immediate interruption of any walk-cycle frame by special moves, blocks, crouches, or jumps, suppression of the first-frame 16-pixel back-walk shift on back-held attacks, and responsive jump attacks during ascent, apex, or descent (with an optional faster 4-frame pre-jump crouch).

### Acceptance criteria

- [ ] With `lenient_command_trackers` enabled, *Hadoken*, *Shoryuken*, and *Tatsumaki Senpukyaku* trackers allow a configurable motion window (default 28 frames), retain their completed state for a configurable grace window (default 8 frames) after releasing the final direction, and accept diagonal *Shoryuken* shortcuts.
- [ ] With `interruptible_walk_cycle` enabled, special moves, crouches, blocks, and jumps interrupt gated walk frames immediately instead of being eaten, and starting a normal attack while holding Back no longer shifts the fighter 16 pixels backward on the first frame.
- [ ] With `anytime_jump_attacks` enabled, neutral and directional jump attacks can be triggered during ascent, at the apex, or during descent (up to 4 frames before landing), with an option to shorten pre-jump startup from 9 frames to 4 frames.
- [ ] Verify each toggle with paired differential scenario tests across the session seam (`false` vs. `true`), confirming that inputs eaten in `Faithful 1987` execute cleanly when the corresponding toggle is active.
- [ ] Confirm zero regression across all **comparison baseline** acceptance tests under the `Faithful 1987` **ruleset preset**.

### Blocked by

- O3

---

## O5: Standard High/Low/Jump Blocking, Lee/Adon Omniblock Fix, and Symmetrical Shoto Balance

### Parent

https://github.com/johnstreetfighter/sf1-port/issues/1

### What to build

Implement the Tier 1 **core toggles** that repair the 1987 defensive guard system (`standard_high_low_block`) and normalize balance between Ryu and Ken (`symmetric_shotos_and_blockable_dp`).

In the **original game**, holding Back walks backward during gated walk frames and gets hit by low attacks, holding Down-Back enters crouching guard (`move 8`) against jumping attacks that then hit straight through it because jumping hit checks only test standing guard (`move 4`), Lee and Adon suffer from a 16-frame hurtbox disappearance bug ("Omniblock") when blocking, and Player 1 Ryu has strictly superior normal frame data and a frame-1 invincible, completely unblockable *Shoryuken*. Enabling `standard_high_low_block` enforces intuitive fighting-game defense: standing guard (`Back`) blocks standing and jumping attacks while remaining vulnerable to low attacks; crouching guard (`Down-Back`) blocks low and standing attacks while remaining vulnerable to overhead jumping attacks; proximity blocking engages cleanly before active hit frames without backward walk drift; and Lee and Adon maintain valid hurtbox tables throughout guard animations. Enabling `symmetric_shotos_and_blockable_dp` makes *Shoryuken* blockable on standing and crouching guard, adds vulnerable landing recovery on whiff or block, and synchronizes baseline normal and special startup/recovery between Ryu and Ken.

### Acceptance criteria

- [ ] With `standard_high_low_block` enabled, standing guard blocks high and jumping attacks and is hit by low attacks; crouching guard blocks low and standing attacks and is hit by jumping overheads; and proximity guard triggers cleanly without gated backward walk drift.
- [ ] Fix the 1987 bug demonstrated in `sf-blocks-05` where holding Down-Back against a jumping attack enters crouching guard (`move 8`) only to be hit through guard, and fix the 16-frame hurtbox disappearance ("Omniblock") on Lee and Adon when `standard_high_low_block` is active.
- [ ] With `symmetric_shotos_and_blockable_dp` enabled, *Shoryuken* can be blocked, has vulnerable landing recovery frames on whiff/block, and Ryu and Ken share identical baseline frame data in `SF1` / `SF1+` styles.
- [ ] Verify all defensive and shoto-balance behaviors via paired differential scenario tests (`false` vs. `true`) across the session seam.
- [ ] Confirm zero regression across all **comparison baseline** acceptance tests (including `sf-blocks-05`’s original buggy behavior) when `Faithful 1987` is selected.

### Blocked by

- O3

---

## O6: Balanced Damage/Chip Scaling, Fireball Slowdown Toggle, True-Second Timer, and Sagat AI Scaling

### Parent

https://github.com/johnstreetfighter/sf1-port/issues/1

### What to build

Complete the Tier 1 **core toggles** by implementing `balanced_damage_and_chip`, `disable_fireball_slowdown`, `real_second_timer_and_draws`, and `fair_cpu_and_sagat_scaling`.

In the **original game**, special moves deal 20/48 HP (41.7% health) per hit and can multi-hit for an instant 60–80% KO, stage-indexed chip damage strips up to 16/48 HP (33.3%) on early stages before dropping to 0 in Thailand, unblocked fireballs trigger a jarring 90-frame slow-motion crawl even mid-round, the round timer ticks every 19 frames (~3.15 ticks per second) and awards tied 3-round matches to the CPU as a Game Over, and Sagat's AI ignores cabinet difficulty settings while reading player inputs on frame 1. These four **core toggles** rebalance special damage (default 8–10/48 HP with single-hit knockdown/invulnerability protection against multi-hit instant kills), normalize chip damage to 1–2/48 HP across all stages (including Thailand) with an independent `chip_death` toggle, restrict projectile slow-motion to final round-winning KOs, tick the round timer every 60 frames (1 real second) with a Sudden Death / Draw round on tied matches, add human-like reaction delay to the CPU's input-reading branch, make Retsu's unblockable jump kick blockable, and give Sagat and Adon distinct Easy, Normal, Hard, and Very Hard difficulty tables.

### Acceptance criteria

- [ ] With `balanced_damage_and_chip` enabled, special moves deal balanced damage (preventing 2-fireball or single-Shoryuken instant KOs), stage chip damage is normalized across all countries including Thailand, and the `chip_death` sub-toggle controls whether chip damage can KO a defender at 1 HP.
- [ ] With `disable_fireball_slowdown` enabled, non-lethal projectile hits run at a continuous 60 FPS without triggering the 90-frame slow-motion crawl, while round-winning KOs retain their dramatic slow-motion finish.
- [ ] With `real_second_timer_and_draws` enabled, each timer unit lasts 60 frames (1.0 real second) instead of 19 frames, and a 1–1 tied match or double KO advances to a Sudden Death / Draw decider round instead of forcing a player Game Over.
- [ ] With `fair_cpu_and_sagat_scaling` enabled, CPU input-reading reactions respect a configurable frame delay, Retsu's unblockable attack is blockable, and Sagat and Adon scale across Easy, Normal, Hard, and Very Hard difficulty settings.
- [ ] Verify all four toggles via paired differential scenario tests (`false` vs. `true`) across the session seam, and confirm zero regression under `Faithful 1987`.

### Blocked by

- O3

---

## O7: Playable 12-Character SF1 (SF1++) Roster, Direct 1P Ken, and 4-Country Select

### Parent

https://github.com/johnstreetfighter/sf1-port/issues/1

### What to build

Unlock all twelve fighters of the 1987 tournament—Ryu, Ken, Retsu, Geki, Joe, Mike, Lee, Gen, Birdie, Eagle, Adon, and Sagat—as human-selectable characters for both Player 1 and Player 2 in raw `SF1` (*SF1++*) style, enable direct 1-Player Ken selection without requiring a Player 2 versus join-in, and unlock 4-country starting selection (Japan, USA, China, England) by default in **overhaul** modes.

When a human selects any of the ten formerly CPU-only fighters in `SF1` (*SF1++*) style, the engine binds that fighter's recovered 1987 animation, movement, collision, and special/projectile routines to the human input parser rather than the AI decision tree. For the `Classic Competitive` **ruleset preset**, match settings also provide tournament sub-format ban-list filters (*Standard Tournament* banning Sagat and P1 Ryu when unpatched, *Low-Tier / No-Shoto*, and *Unrestricted / Anything Goes*).

### Acceptance criteria

- [ ] Allow Player 1 and Player 2 to select any of the twelve tournament fighters (Ryu, Ken, Retsu, Geki, Joe, Mike, Lee, Gen, Birdie, Eagle, Adon, Sagat) on a full-cast Character Select screen in Arcade, Versus, and Training modes.
- [ ] Support direct 1-Player Arcade starts as Ken (or any other fighter) without requiring a 2-Player versus join-in, and enable 4-country starting selection (Japan, USA, China, England) by default in **overhaul** presets.
- [ ] Bind human controller inputs cleanly to all ten formerly CPU-only fighters in raw `SF1` style so every existing 1987 normal, special, and projectile (including Geki's shuriken/teleport, Gen's darts/leap, and Sagat's *Tiger Shot*/*Tiger Knee*) can be executed by a human player.
- [ ] Enforce the selectable `Classic Competitive` tournament ban-list sub-formats (*Standard Tournament*, *Low-Tier / No-Shoto*, *Unrestricted*) on Character Select.
- [ ] Verify via scenario session tests that each of the twelve fighters can be selected by Player 1 and Player 2, move, attack, and complete a round without crashes or state corruption, while `Faithful 1987` preserves the original 1987 start/country flow.

### Blocked by

- O5
- O6

---

## O8: Extended Scene Compositor and Offline AI 1987 Sprite/Audio Asset Pipeline

### Parent

https://github.com/johnstreetfighter/sf1-port/issues/1

### What to build

Build the Extended Scene Compositor for **overhaul** modes alongside the offline AI-assisted 1987 sprite-generation, palette-quantization, and ADPCM/FM audio-encoding pipeline and asset-pack validator.

While `Faithful 1987` continues to render strictly through the unmodified 1987 hardware limits (64 sprite entries, 16 sprite sub-palettes, and the 1,024-tile 2bpp text layer), **overhaul** modes render through the Extended Scene Compositor at the exact same `384×224` pixel resolution and 15-color-plus-transparent per-sprite rule while removing the 64-sprite-entry ceiling and 16-palette ceiling, adding a dedicated 1987-styled pixel-art HUD layer for **era gauges** and extended UI text, and compositing the diagnostic/Training overlay layer. Accompanying offline pipeline tooling ingests decoded 1987 character sprites as style/proportion references for AI image generation of missing animation frames, **era costumes**, and endings, quantizes generated sheets to 15-color-plus-transparent `16×16` tiles in external overhaul asset packs, encodes new voice callouts and move sound effects into authentic 4-bit 8 kHz MSM5205 ADPCM and YM2151 FM formats, and validates all asset packs in CI.

### Acceptance criteria

- [ ] Render **overhaul** scenes through the Extended Scene Compositor at `384×224` native resolution with 15-color-plus-transparent sprite palettes, supporting more than 64 simultaneous sprite entries and more than 16 active sprite sub-palettes without flickering or palette corruption.
- [ ] Provide a dedicated pixel-art HUD layer for bottom-of-screen **era gauges**, extended character/style select typography, and post-frame Training Mode overlays.
- [ ] Deliver the offline asset-generation and quantization pipeline that converts AI-generated or artist-authored sprite sheets into indexed 15-color-plus-transparent `16×16` tile packs and encodes new voice/SFX clips into 4-bit 8 kHz MSM5205 ADPCM and YM2151 FM patch data.
- [ ] Implement an automated asset-pack validator test that verifies every overhaul sprite sheet obeys the 15-color-plus-transparent `16×16` tile schema and every audio clip obeys the 1987 ADPCM/FM encoding rules, without bundling copyrighted original arcade ROMs in the repository.
- [ ] Verify that `Faithful 1987` bypasses the Extended Scene Compositor completely and passes all **comparison baseline** pixel and audio checks with zero diffs.

### Blocked by

- O7

---

## O9: Audio Jukebox (1987 Synth vs. Red Book CD OST) and Control Schemes (Classic, Pneumatic 2-Pad, Modern)

### Parent

https://github.com/johnstreetfighter/sf1-port/issues/1

### What to build

Deliver the in-game Audio Jukebox / Sound Test and the three per-player selectable control schemes: Classic 6-Button, Simulated Pneumatic 2-Pad, and Modern Controls.

The Audio Jukebox allows players to choose globally or per stage/character between native 1987 YM2151 FM + MSM5205 ADPCM arcade synthesis (default) and external Red Book CD arranged soundtrack files (such as user-supplied or custom 1988 PC Engine CD *Fighting Street*-style arranged tracks) with seamless loop-point metadata, plus a full Sound Test menu for auditioning every music track, FM effect, and ADPCM voice sample. On the input side, each player can independently select Classic 6-Button controls (`LP, MP, HP, LK, MK, HK`), Simulated Pneumatic 2-Pad controls (mapping one Punch pad and one Kick pad where button hold duration of 1–2 frames, 3–4 frames, or 5+ frames—or analog gamepad trigger pressure—selects Light, Medium, or Heavy strength as on the 1987 deluxe cabinet), or Modern Controls (`Light, Medium, Heavy, Special` plus `Assist/Macro` with directional 1-button specials and a toggleable startup/damage penalty on 1-button neutral specials and supers).

### Acceptance criteria

- [ ] Provide an Audio Jukebox and Sound Test menu that switches cleanly per stage or character between native 1987 YM2151+MSM5205 synthesis and external Red Book CD audio tracks with seamless looping, falling back gracefully to native synthesis if external audio files are absent.
- [ ] Support per-player selection between Classic 6-Button, Simulated Pneumatic 2-Pad (both digital hold-duration thresholds and analog trigger pressure), and Modern Controls on the controller select / match setup screen.
- [ ] Implement the toggleable Modern Controls balance rule (`modern_control_damage_penalty`, default on in competitive presets) applying a 20% damage scaling and +2 frame startup penalty to 1-button neutral specials and supers while keeping manual motion inputs at 100% damage.
- [ ] Verify all three control schemes through deterministic scenario session tests (confirming Light/Medium/Heavy dispatch from 1–2f, 3–4f, and 5+f holds in Pneumatic mode, and penalized vs. unpenalized specials in Modern mode).
- [ ] Verify audio playback and controller switching in the host smoke check without regressing `Faithful 1987`.

### Blocked by

- O8

---

## O10: Cross-Era Combat Core, Ground Throws, and SF1+ Completed Movesets for Japan & USA (Ryu, Ken, Retsu, Geki, Joe, Mike)

### Parent

https://github.com/johnstreetfighter/sf1-port/issues/1

### What to build

Implement the Tier 2 data-driven combat state machine, cross-era interaction rules, universal ground throws, and complete `SF1+` movesets for the six Japan and USA fighters: Ryu, Ken, Retsu, Geki, Joe, and Mike.

In Tier 2 (`SF1+` and later styles), fighters execute data-driven state tables supporting 6-button standing, crouching, and jumping normals, hitstun/blockstun decay, normal-to-special cancel windows, and cross-era interaction rules (where the attacker's **era style** governs hitstun and comboability, the defender's **era style** governs guard rules, and internal health is normalized to 1,000 HP mapped to the 11 HUD cells with a toggle for unscaled 1987 42% special damage). All fighters gain techable ground throws (`Forward/Back + MP+MK` or `LP+LK` with a 7-frame tech window) and anti-throw-loop corner pushback on wakeup (with match toggles for untechable classic *SF2* throws and corner throw loops). Using this core and the 1987 asset pipeline, Ryu, Ken, Retsu, Geki, Joe, and Mike receive complete `SF1+` movesets—giving Retsu, Geki, Joe, and Mike full crouching/jumping normals, standard high/low guards, throws, and command specials (*Kiai Shot*, *Shuriken* & *Teleport Strike*, *Rolling Sobat* & *Dynamite Straight*, and *Bison Rush* & *Iron Upper*).

### Acceptance criteria

- [ ] Execute data-driven fighter state tables for `SF1+` with configurable startup, active, recovery, hitstun, blockstun, pushback, and normal-to-special cancel windows.
- [ ] Enforce the cross-era combat rules across the session seam: attacker-governed hitstun/combo rules, defender-governed guard rules, normalized 1,000 HP health mapping (with the `classic_1987_special_damage` toggle), and continuous input buffering through screen freezes (`disable_freeze_input_lockout`).
- [ ] Implement universal ground throws with a 7-frame throw-tech window and anti-throw-loop wakeup pushback by default, plus working match toggles for `untechable_sf2_throws` and `allow_corner_throw_loops`.
- [ ] Deliver complete `SF1+` movesets, 1987-styled missing sprite animations, and 1987 ADPCM/FM audio cues for Ryu, Ken, Retsu, Geki, Joe, and Mike.
- [ ] Verify through automated scenario session tests that all six fighters can execute their full `SF1+` normal, command-normal, throw, and special kits and combo from canceling normals into specials.

### Blocked by

- O8
- O9

---

## O11: SF1+ Completed Movesets for China, England, & Thailand (Lee, Gen, Birdie, Eagle, Adon, Sagat) and 12-Character Arcade Endings

### Parent

https://github.com/johnstreetfighter/sf1-port/issues/1

### What to build

Complete the `SF1+` 12-fighter roster by delivering full `SF1+` movesets for the six China, England, and Thailand fighters—Lee, Gen, Birdie, Eagle, Adon, and Sagat—alongside the full 12-character Arcade Mode progression, positive-edge-friendly bonus stages, pre-fight rival dialogues, and unique 1987 pixel-art character endings for all twelve fighters.

Lee, Gen, Birdie, Eagle, Adon, and Sagat gain complete 6-button standing, crouching, and jumping normals, standard high/low guards, ground throws, and signature command specials in `SF1+` (including Lee's *Zesshouohken* and *Double Kick*, Gen's *Hyakurenko* and *Gekiro* without *Alpha* stance-switching in `SF1+`, Birdie's *Bull Head* and *Bull Horn*, Eagle's *Manchester Black* and *St. Andrew's Green*, Adon's *Jaguar Kick* and *Rising Jaguar*, and Sagat's High/Low *Tiger Shot* and *Tiger Knee*). In Arcade Mode, any of the twelve fighters can clear the 4 countries plus Thailand, play plank-breaking and barrel/target bonus stages that work cleanly with positive-edge inputs, see custom 1987-style rival intro/win quotes, and unlock their own illustrated 1987 pixel-art ending sequence and credits roll.

### Acceptance criteria

- [ ] Deliver complete `SF1+` movesets, missing 1987-style sprite animations, and 1987 ADPCM/FM voice/SFX cues for Lee, Gen, Birdie, Eagle, Adon, and Sagat.
- [ ] Verify that every one of the twelve tournament fighters in `SF1+` has complete standing, crouching, and jumping L/M/H punches and kicks, high/low blocking, techable throws, and working command specials.
- [ ] Update the Arcade Mode bonus stages so timing and power checks work accurately when `positive_edge_attacks` is enabled.
- [ ] Provide unique 1987-styled pre-fight rival dialogues and illustrated pixel-art Arcade endings for all twelve fighters upon defeating Thailand.
- [ ] Verify full Arcade Mode progression and all six new `SF1+` movesets via automated scenario session tests and asset-pack validation.

### Blocked by

- O10

---

## O12: Two-Step Era/Revision Selector and SFII Era Styles (SF2 WW, CE, HF, Super, ST)

### Parent

https://github.com/johnstreetfighter/sf1-port/issues/1

### What to build

Build the two-step `Era Family -> Revision (& Loadout)` Character Select interface, the match-level **era gauge** decoupling toggle (*Era Default*, *No Gauges / Pure Footsies*, *Unified Gauge*), **era costume** and unlocked palette selection, and the five *Street Fighter II* **era styles** (`SF2 WW`, `SF2 CE`, `SF2 HF`, `SF2 Super`, and `SF2 ST`) for Ryu, Ken, and Sagat.

After picking a fighter on Character Select, players step through the fighter's canonical Era Families and specific Arcade Revisions, plus costume/palette selection (defaulting to the authentic 1987 outfit with optional 1987-pixel-art **era costumes** and 8 unlocked color palettes). Selecting an *SFII* revision configures the authentic revision-specific moveset, frame data, knockdown rules, and mechanics for Ryu, Ken, and Sagat: `SF2 WW` (high-stun grounded combos), `SF2 CE` (double-hit *Shoryuken* for Ken, playable boss Sagat), `SF2 HF` (air *Tatsumaki*, faster turbo speeds), `SF2 Super` (Ryu's *Shakunetsu Hadoken* and flaming *Shoryuken*), and `SF2 ST` (the 1-stock **SUPER** gauge rendered in 1987 HUD pixel art, *Shinku Hadoken*, *Shoryureppa*, *Tiger Genocide*, and O.Ryu / O.Ken / O.Sagat properties).

### Acceptance criteria

- [ ] Implement the two-step `Era Family -> Revision (& Loadout)` selector on Character Select, displaying only the eras and revisions each fighter canonically appeared in, plus **era costume** and 8-palette selection.
- [ ] Implement the match-level **era gauge** mode toggle (*Era Default*, *No Gauges / Pure Footsies*, and *Unified Gauge*) and wire `Pure Footsies (1987+)` to disable all **era gauges** automatically.
- [ ] Deliver `SF2 WW`, `SF2 CE`, `SF2 HF`, `SF2 Super`, and `SF2 ST` **era styles** for Ryu, Ken, and Sagat with revision-accurate moves, properties, and optional *SFII* **era costumes**.
- [ ] Implement the `SF2 ST` 1-stock **SUPER** **era gauge** (building on specials and attacks, enabling Super Combos and Super cancels when full) on the Extended Scene Compositor HUD.
- [ ] Verify revision-specific moves, Super gauge gain/spend, and cross-era interaction (`SF2` vs. `SF1`/`SF1+`) through automated scenario session tests.

### Blocked by

- O11

---

## O13: SFZ Era Styles (SFZ1, SFZ2, SFZ2A, SFZ3 / Upper) and 3-Level Alpha Gauge

### Parent

https://github.com/johnstreetfighter/sf1-port/issues/1

### What to build

Implement the four *Street Fighter Alpha / Zero* **era styles** (`SFZ1`, `SFZ2`, `SFZ2A`, and `SFZ3` / *Upper*) and the 3-Level Alpha **era gauge** for every 1987 fighter who canonically appeared in the *Alpha* series: Ryu, Ken, Adon, Sagat, Birdie, and Gen (across `SFZ1`–`SFZ3`), plus Eagle (in `SFZ3` *Upper*).

Players selecting an `SFZ` revision get that revision's moveset and the 3-Level Alpha **era gauge** (Levels 1, 2, and 3 Super Combos, Alpha Counters costing 1 stock out of blockstun, air blocking where applicable, and Custom Combos / Variable Combo in `SFZ2`, `SFZ2A`, and `SFZ3` V-ISM). Selecting `SFZ3` opens the Loadout sub-step for **A-ISM** (3-level Super gauge), **V-ISM** (Variable/Custom Combo gauge), or **X-ISM** (single max-damage Level 3 gauge with heightened damage). Gen's `SFZ` styles introduce real-time *Mantis* (*Sou*) and *Crane* (*Ki*) stance switching (`PPP` / `KKK`) with stance-specific normals, specials, and Super Combos (*Zan'ei*, *Shitenshuu*, *Jakorifu*, *Kouga*), while Birdie, Adon, Eagle, Ryu, Ken, and Sagat receive their signature *Alpha* supers and optional *Alpha* **era costumes** (such as punk/mohawk Birdie) in 1987 pixel art.

### Acceptance criteria

- [ ] Deliver `SFZ1`, `SFZ2`, `SFZ2A`, and `SFZ3` **era styles** for Ryu, Ken, Adon, Sagat, Birdie, and Gen, and `SFZ3` (*Upper*) for Eagle, including A-ISM, V-ISM, and X-ISM loadout selection in `SFZ3`.
- [ ] Implement the 3-Level Alpha **era gauge** supporting Level 1/2/3 Super Combos, Alpha Counters from blockstun, and timed Custom Combos (`SFZ2`/`SFZ2A`/`V-ISM`) on the 1987 HUD layer.
- [ ] Implement Gen's dual *Mantis* and *Crane* stance switching, stance-specific normals/specials, and Super Combos across `SFZ1`–`SFZ3`.
- [ ] Provide optional 1987-pixel-art *Alpha* **era costumes** (such as `SFZ` Birdie and `SFZ` young Ryu) and 1987 ADPCM/FM audio callouts for all *Alpha* supers.
- [ ] Verify Level 1/2/3 Super execution, Alpha Counters, Custom Combos, and Gen's stance switching across the scenario session seam.

### Blocked by

- O12

---

## O14: SFIII Era Styles (SF3 NG, 2I, 3S), Parry, and Super Arts I/II/III

### Parent

https://github.com/johnstreetfighter/sf1-port/issues/1

### What to build

Implement the three *Street Fighter III* **era styles** (`SF3 NG`, `SF3 2I`, and `SF3 3S`) alongside the *SFIII* Parry system, EX Specials (in `2I` and `3S`), Universal Overhead, and pre-match Super Art I / II / III loadout selection for Ryu and Ken.

When playing in an `SF3` style, Ryu and Ken can Parry high/mid attacks by tapping `Forward` within a 10-frame window (6 frames against projectiles) or low attacks by tapping `Down`, plus air parries and Red Parries out of blockstun, negating chip and blockstun to grant immediate counter-attack frame advantage. On Character Select, the Loadout step prompts the player to choose **Super Art I**, **Super Art II**, or **Super Art III**—each with its authentic *SFIII* bar length and stock count (Ryu: *Shinku Hadoken*, *Shin Shoryuken*, or *Denjin Hadoken* with unblockable stun charge; Ken: *Shoryureppa*, *Shinryuken*, or fast 1-frame-startup *Shippu Jinraikyaku*)—plus revision-specific moves like Ryu's *Joudan Sokutogeri* donkey kick and target combos.

### Acceptance criteria

- [ ] Deliver `SF3 NG`, `SF3 2I`, and `SF3 3S` **era styles** for Ryu and Ken with revision-accurate normals, target combos, *Joudan Sokutogeri*, Universal Overhead (`MP+MK`), and EX Specials (in `2I` and `3S`).
- [ ] Implement high/mid (`Forward`), low (`Down`), air, and Red Parry mechanics with authentic frame windows and frame-advantage transitions when defending in an `SF3` style.
- [ ] Support pre-match Loadout selection of Super Art I, II, or III with distinct bar lengths and stock capacities on the 1987 HUD layer, including Ryu's chargeable unblockable *Denjin Hadoken* and Ken's *Shippu Jinraikyaku*.
- [ ] Verify high/low/Red Parries, EX specials, and all six Super Arts (Ryu I/II/III and Ken I/II/III) through deterministic scenario session tests.

### Blocked by

- O12

---

## O15: SFIV Era Styles (SF4, SSF4, AE2012, USF4), Focus/FADC, and Revenge Ultra Gauge

### Parent

https://github.com/johnstreetfighter/sf1-port/issues/1

### What to build

Implement the four *Street Fighter IV* **era styles** (`SF4`, `SSF4`, `SF4 AE2012`, and `USF4`), the 4-bar Super + Revenge Ultra **era gauge**, Focus Attack (`MP+MK`), Focus Attack Dash Cancel (FADC), and Red Focus / Delayed Standing / Ultra Combo Double (in `USF4`) for Ryu, Ken, Sagat, Adon, and Gen.

Players selecting an `SF4` revision for Ryu, Ken, Sagat (from `SF4`), or Adon and Gen (from `SSF4` onward) gain the 4-segment Super Combo bar (spending 1 bar for EX Specials, 2 bars for Focus Attack Dash Cancels, or 4 bars for Super Combos) and the circular/horizontal Revenge Ultra meter that fills as the fighter takes damage. In the Loadout step, players select **Ultra Combo I**, **Ultra Combo II** (from `SSF4` onward), or **Ultra Combo Double** (`USF4`)—including Ryu's *Metsu Hadoken* / *Metsu Shoryuken*, Ken's *Shinryuken* / *Guren Senpukyaku*, Sagat's *Tiger Destruction* / *Tiger Cannon* (plus *Angry Charge*), Adon's *Jaguar Revolver* / *Jaguar Avalanche*, and Gen's four Ultras across *Mantis* and *Crane* stances (*Zetsuei* / *Ryukoha* and *Teiga* / *Kouga*).

### Acceptance criteria

- [ ] Deliver `SF4`, `SSF4`, `SF4 AE2012`, and `USF4` **era styles** for Ryu, Ken, and Sagat, and `SSF4`, `SF4 AE2012`, and `USF4` **era styles** for Adon and Gen, with revision-accurate balance and movesets.
- [ ] Implement the 3-level chargeable Focus Attack (`MP+MK`) with 1-hit armor, crumple states at Level 2/3, unblockable Level 3, and 2-bar Focus Attack Dash Cancels (FADC) out of special moves and Focus Attacks.
- [ ] Implement `USF4`-specific system additions: Red Focus Attack (multi-hit armor), Delayed Wakeup (`2 buttons` on knockdown), and Ultra Combo Double selection with scaled Ultra damage.
- [ ] Support pre-match Loadout selection of Ultra I, Ultra II, or Ultra Double alongside the Revenge Ultra gauge (respecting the `disable_comeback_scaling` safety toggle when active).
- [ ] Verify Focus armor, FADC combo extensions, and Ultra Combos for all five fighters through automated scenario session tests.

### Blocked by

- O13
- O14

---

## O16: SFV Era Styles (SF5, AE, CE) and V-System Gauge (V-Skill, V-Trigger, V-Reversal, V-Shift)

### Parent

https://github.com/johnstreetfighter/sf1-port/issues/1

### What to build

Implement the three *Street Fighter V* **era styles** (`SF5`, `SF5 AE`, and `SF5 CE`) and the V-System + Critical Art **era gauge** for Ryu, Ken, Birdie, and Sagat.

Players selecting an `SF5` revision for Ryu, Ken, Birdie (from `SF5`), or Sagat (from `SF5 AE` and `SF5 CE`) gain the 3-stock EX / Critical Art bar alongside the V-Gauge (`2` or `3` V-bars), Crush Counter heavy normals, V-Skills (`MP+MK`), V-Reversals (`Forward + PPP/KKK` in blockstun costing 1 V-bar), V-Triggers (`HP+HK`), and V-Shift / V-Shift Break (`MK+HP` in `SF5 CE`). On Character Select, the Loadout step lets players select **V-Skill I** or **V-Skill II** (in `CE`) and **V-Trigger I** or **V-Trigger II** (in `AE` and `CE`)—such as Ryu's *Mind's Eye* / *Thrust Strike* and *Denjin Renki* / *Kakko Fubatsu*, Ken's *Quick Step* / *Ryusenkyaku* and *Heat Rush* / *Shinryuken*, Birdie's snack-can/banana/chili-pepper *Break Time* / *Chewing Time* and *Enjoy Time* / *Birdie Time*, and Sagat's *Angry Charge* / *Tigerylight* and *Tiger Charge* / *Tiger Assault*.

### Acceptance criteria

- [ ] Deliver `SF5`, `SF5 AE`, and `SF5 CE` **era styles** for Ryu, Ken, and Birdie, and `SF5 AE` and `SF5 CE` **era styles** for Sagat.
- [ ] Implement the V-Gauge and 3-stock EX/Critical Art bar on the 1987 HUD layer, supporting V-Skills, V-Reversals, V-Triggers, Critical Arts, and Crush Counter states.
- [ ] Support Loadout selection of V-Skill I/II and V-Trigger I/II per revision, plus `SF5 CE`'s defensive V-Shift and follow-up V-Shift Break.
- [ ] Provide optional 1987-pixel-art *SFV* **era costumes** (such as `SF5` Hot Ryu and `SF5` Birdie) and 1987 ADPCM/FM voice/SFX assets.
- [ ] Verify V-Skill meter gain, V-Trigger activation/install properties, V-Reversals, V-Shifts, and Critical Arts for all four fighters across the scenario session seam.

### Blocked by

- O15

---

## O17: SF6 Era Style, 6-Bar Drive Gauge, and Safety Toggles (Ryu, Ken, Retsu)

### Parent

https://github.com/johnstreetfighter/sf1-port/issues/1

### What to build

Implement the *Street Fighter 6* **era style** (`SF6`), the 6-bar Drive Gauge + 3-level Super Art **era gauge**, and the modern competitive safety toggles for Ryu, Ken, and Retsu (inspired by Retsu's *SF6* World Tour appearance).

Fighters in `SF6` style start each round with 6 green Drive bars rendered on the 1987 HUD and a 3-level Super Art bar that persists across rounds. The Drive Gauge powers Overdrive (OD) Specials (`2 bars`), Drive Parry (`MP+MK`, `0.5` bar startup + drain, regenerating on successful block/parry, with a 2-frame **Perfect Parry** window), Parry Drive Rush (`1 bar`) and Cancel Drive Rush (`3 bars`), Drive Impact (`HP+HK`, `1 bar`, 2 hits of armor, wall splat in the corner), and Drive Reversal (`Forward + HP+HK`, `2 bars`). Emptying all 6 Drive bars enters **Burnout** (increased blockstun, chip damage on blocked specials, and wall stun from Drive Impact). Ryu gets *Hashogeki*, *Denjin Charge*, and Level 1/2/3 Super Arts (*Shinku Hadoken*, *Shin Hashogeki*, *Shin Shoryuken*); Ken gets *Jinrai Kick* follow-ups, *Dragonlash Kick*, Quick Dash, and Level 1/2/3 Super Arts (*Dragonlash Flame*, *Shippu Jinraikyaku*, *Shinryu Reppa*); and Retsu gets a complete *SF6*-era monk kit (*Kiai Shot*, *Tobigeri*, *Renkiko* buff, and Super Arts). Match safety toggles allow players to disable raw neutral Drive Rush inertia (`disable_raw_drive_rush`, while keeping 3-bar Cancel Drive Rush) and enforce continuous input buffering during all Drive/Super flashes (`disable_freeze_input_lockout`).

### Acceptance criteria

- [ ] Deliver the `SF6` **era style** for Ryu, Ken, and Retsu with full *SF6* movesets, Level 1/2/3 Super Arts, and optional 1987-pixel-art `SF6` **era costumes** (such as bearded/kasaya `SF6` Ryu and jacketed `SF6` Ken).
- [ ] Implement the 6-bar Drive Gauge and Burnout state machine on the 1987 HUD layer, supporting OD Specials, Drive Parry, 2-frame Perfect Parry, Parry Drive Rush, Cancel Drive Rush, Drive Impact, and Drive Reversal.
- [ ] Implement the `disable_raw_drive_rush` safety toggle so competitive rooms can disable 1-bar raw neutral Drive Rush while retaining 3-bar Cancel Drive Rush.
- [ ] Verify Drive gauge expenditure, Burnout transitions, Perfect Parry scaling, Drive Impact armor/wall-splat, and `disable_raw_drive_rush` behavior through automated scenario session tests.

### Blocked by

- O16

---

## O18: Style-Aware Utility AI and Training Mode Scenario Drills / Combo Trials

### Parent

https://github.com/johnstreetfighter/sf1-port/issues/1

### What to build

Build the Style-Aware Utility AI controller for CPU opponents across all **overhaul** styles (`SF1+` through `SF6`) alongside pre-built Training Mode Scenario Drills and Character Combo Trials.

When a CPU opponent plays in raw `SF1` style, it uses the translated 1987 table AI (enhanced by `fair_cpu_and_sagat_scaling` when enabled); when a CPU opponent plays in `SF1+`, `SF2`, `SFZ`, `SF3`, `SF4`, `SF5`, or `SF6` style (in Arcade, Versus, CPU vs. CPU exhibition, or World Tour), the Style-Aware Utility AI evaluates spacing, hit-confirms, normal-to-special cancels, anti-airs, throw techs, and the active **era gauge** (executing Super Combos, Alpha Counters, Parries, FADCs, V-Triggers, and Drive Impacts/Drive Rushes scaled by the selected Easy/Normal/Hard/Very Hard difficulty). In Training Mode, players can launch one-click Scenario Drills (Anti-Air Defense against randomized jump-ins + ground approach, Whiff Punish Practice, Throw-Tech & Shimmy Defense, and *SF1* Survival against unscaled 1987 *Tiger Shots*) and step-by-step Character Combo Trials for every fighter and **era style**.

### Acceptance criteria

- [ ] Route `SF1` CPU fighters to the 1987 table AI and `SF1+` through `SF6` CPU fighters to the Style-Aware Utility AI across Arcade, Versus, and CPU vs. CPU modes.
- [ ] Verify that the Style-Aware Utility AI performs normal-to-special cancels, anti-airs, throw techs, and era-specific gauge actions (Super Combos, Parries, FADCs, V-Triggers, Drive Rush/Impact) with reaction delays and execution rates governed by difficulty level.
- [ ] Provide one-click Training Mode Scenario Drills (Anti-Air, Whiff Punish, Throw-Tech/Shimmy, and *SF1* Survival) and per-character/per-era Combo Trials with live step completion tracking.
- [ ] Verify deterministic Utility AI decision-making (seeded solely by the session PRNG) and scenario drill completion across the scenario session seam.

### Blocked by

- O17

---

## O19: 2v1 Dramatic Battle Mode (3-Fighter Camera, Shared/Independent Lifebars, Friendly Fire Toggle)

### Parent

https://github.com/johnstreetfighter/sf1-port/issues/1

### What to build

Implement *Street Fighter Alpha*-inspired **2v1 Dramatic Battle Mode**, allowing two allied fighters (2P Co-op or 1P + CPU partner, such as Ryu and Ken) to fight simultaneously on-screen against a single boss or opponent (such as Boss Sagat, Mike, or any chosen fighter/style).

The Extended Scene Compositor renders all three fighters and their projectiles simultaneously at `384×224` with zero sprite-limit flickering, while the stage camera tracks the horizontal midpoint of the leftmost and rightmost active fighters and clamps movement so no fighter can walk off-screen. Match setup provides preset Dramatic Battles (such as *Ryu & Ken vs. Emperor Sagat*) as well as custom 2v1 team selection across any **era style**, with toggles for Lifebar Mode (**Shared Team Lifebar** by default, or **Independent Lifebars**) and Friendly Fire (**Off** by default, or **On** so allied attacks and projectiles can hit teammates).

### Acceptance criteria

- [ ] Step three active fighter instances and their projectiles simultaneously in 2v1 Dramatic Battle Mode across both 2-Player Co-op and 1-Player + CPU Ally configurations.
- [ ] Clamp all three fighters within the `384×224` camera viewport by tracking the horizontal span between the outermost active fighters.
- [ ] Support the **Shared Team Lifebar** (default) and **Independent Lifebars** match toggle for the 2-player team, plus scaled boss health/damage for the solo boss.
- [ ] Support the **Friendly Fire** toggle (**Off** by default so team hitboxes ignore each other; **On** so allied attacks and projectiles collide with teammates).
- [ ] Verify 3-fighter collision, camera clamping, shared vs. independent lifebar depletion, and friendly fire on/off through deterministic scenario session tests.

### Blocked by

- O8
- O11

---

## O20: Alpha 3-Style "1987 World Tour" Single-Player Campaign

### Parent

https://github.com/johnstreetfighter/sf1-port/issues/1

### What to build

Build the *Street Fighter Alpha 3*-inspired **"1987 World Tour"** single-player campaign, taking any of the twelve canonical 1987 fighters across a branching five-country world map (Japan, USA, China, England, and Thailand) with mission modifiers, Survival, Time Attack, Dramatic Battle boss encounters, and optional RPG progression.

Players select any of the twelve fighters and travel node-by-node across the five tournament countries, encountering themed match conditions (such as *Projectile-Only Defense*, *No Blocking Footsies*, *SF1 Raw Rules Survival*, *1-v-Many Dojo Gauntlet*, *2v1 Dramatic Boss Fights*, and *Cross-Era Style Showdowns*). An upfront mode toggle lets players choose between **RPG Progression Enabled** (earning XP to equip optional Attack/Defense/Speed stat boosts and passive *ISM-Plus* perks inside World Tour) or **Arcade Pure Rules** (playing every mission with unboosted competitive stats). In accordance with the project's anti-grind guarantee, all twelve fighters, all **era styles**, all **era costumes**, and all color palettes remain 100% unlocked from initial launch in Arcade, Versus, Training, and Online modes regardless of World Tour progress.

### Acceptance criteria

- [ ] Provide the interactive five-country branching World Tour map (Japan, USA, China, England, Thailand) playable with any of the twelve canonical fighters.
- [ ] Implement mission modifiers (damage/chip rules, gauge restrictions, survival endurance waves, time attack targets, and 2v1 Dramatic Battle nodes) driven through `MatchConfig` on the scenario session seam.
- [ ] Support the **RPG Progression** toggle (allowing optional stat leveling and passive perks within World Tour when on, or pure unboosted stats when off) with local save persistence.
- [ ] Confirm that zero characters, **era styles**, **era costumes**, or palettes in Arcade, Versus, Training, or Online modes are locked behind World Tour progression.
- [ ] Verify mission initialization, modifier enforcement, and campaign save/load persistence in automated scenario and integration tests.

### Blocked by

- O18
- O19

---

## O21: Deterministic Rollback Netplay, Open Room Browser, and Wired-Only Auto-Filter

### Parent

https://github.com/johnstreetfighter/sf1-port/issues/1

### What to build

Deliver peer-to-peer deterministic **Rollback Netplay**, an open worldwide Custom Room Browser and matchmaking queue with zero geographic region locks, live connection telemetry, and a permanent **Wired-Only Auto-Filter** toggle.

Building on the deterministic frame-snapshot system from `O2`, online matches synchronize only frame-indexed controller bitmasks across peers, automatically rolling back and re-simulating frames when remote inputs arrive late. At match handshake, the host's gameplay-affecting `MatchConfig` (**ruleset preset**, active **core toggles**, **era gauge** mode, safety toggles, and initial PRNG seed) is locked and synchronized across both peers, while each player independently selects their own fighter, **era style**, **era costume**/palette, control scheme, and local-only Jukebox soundtrack. Players can connect via direct IP/room codes or browse an open worldwide custom lobby list with no geographic region locks, viewing real-time ping (ms), rollback frame counts, jitter, and a hardware network-interface indicator (Wired Ethernet vs. Wi-Fi), and can enable a persistent **Wired-Only Auto-Filter** toggle to automatically hide or decline Wi-Fi opponents.

### Acceptance criteria

- [ ] Implement peer-to-peer rollback netplay using deterministic session snapshots and fast-forward re-simulation, supporting configurable rollback windows (up to 8 frames) and input delay (0–4 frames).
- [ ] Synchronize and hash-verify the host's gameplay `MatchConfig` and PRNG seed at match start while allowing per-player fighter, **era style**, **era costume**, control scheme, and local Jukebox BGM selection.
- [ ] Provide direct room-code/IP connection and an open worldwide Custom Room Browser with zero geographic region restrictions, supporting 2-player matches, spectator slots, and 2v1 Dramatic Battle co-op rooms.
- [ ] Detect local network adapter type (Ethernet vs. Wi-Fi), display live ping/jitter/rollback-frame indicators and Wired/Wi-Fi badges in lobbies and pre-match prompts, and enforce the user's persistent **Wired-Only Auto-Filter** toggle.
- [ ] Verify in automated multi-peer loopback rollback tests that simulated packet delay, out-of-order delivery, and 8-frame rollbacks converge to bit-identical game state hashes on both peers across `Faithful 1987` and `Definitive Overhaul` matches.

### Blocked by

- O2
- O18
