# Definitive Street Fighter (1987) Port & Overhaul Design

- **Date:** 2026-10-27
- **Status:** Approved (2026-10-27)
- **Context:** [`CONTEXT.md`](../../CONTEXT.md), [`docs/project-brief.md`](../project-brief.md), [`docs/adr/0001-recover-original-logic-in-rust.md`](../adr/0001-recover-original-logic-in-rust.md), [`docs/research/community-expectations-and-criticisms.md`](../research/community-expectations-and-criticisms.md)

---

## 1. Intent and Why

The original 1987 arcade *Street Fighter* (**original game**) launched one of the most influential franchises in gaming history, introducing Ryu, Ken, Sagat, the six-button punch/kick layout, and motion-input special moves (*Hadoken*, *Shoryuken*, *Tatsumaki Senpukyaku*). Yet for nearly four decades, fans returning to the 1987 game have hit the same wall:
1. **Hostile input handling:** Even in the six-button **reference version** (`sf`), the game emulates the deluxe cabinet's pneumatic pads in software (`0x3b3ae`), firing attacks only on button *release* (`release + 1`), locking out special moves unless completed within 18 frames (`0x12`) with zero directional release leniency (`0x40ae4`), and eating inputs during 8 out of every 10 walk frames (`0x91` at `0x403d4`).
2. **Broken combat & balance quirks:** Special moves strip 41.7% life (20/48 HP), Ryu's *Shoryuken* is frame-1 invincible and 100% unblockable (`0x45efa`), stage chip damage strips up to 33.3% life per blocked special (16/48 HP in Stages 2–3, dropping to 0 against Adon and Sagat), standing guard fails to activate against low attacks, crouching guard (`move 8`) is triggered by jump attacks (`0xc1`) that then hit it anyway (`sf-blocks-05`), every unblocked fireball triggers a 90-frame 10 FPS slow-motion crawl (`Scene Event 9`), and Sagat's AI ignores cabinet difficulty settings (`1` table image across all 8 DIP/variant combinations).
3. **Locked roster & missing modes:** Ten iconic martial artists across five countries (Retsu, Geki, Joe, Mike, Lee, Gen, Birdie, Eagle, Adon, Sagat) are unplayable CPU bosses with incomplete player state machines, Ken cannot be chosen directly in 1P mode (`game mode 4` requires winning a 2-credit Versus match first), and official re-releases (*Street Fighter 30th Anniversary Collection*, *Capcom Arcade 2nd Stadium*) omitted both Training Mode and online netplay.

**The vision of this design** is to deliver both historical preservation and the ultimate fan dream in one project:
- First, complete the verified, frame-exact 1987 **port** matching the **comparison baseline** (pinned MAME 0.289 running verified `sf` ROMs).
- Second, build a comprehensive **Definitive Overhaul** on top of that foundation—preserving the 1987 arcade pixel-art aesthetic while making all 12 fighters playable with full movesets across every *Street Fighter* era they appeared in (`SF1`, `SF1+`, `SFII`, `SFZ`, `SFIII`, `SFIV`, `SFV`, `SF6`), exposing every mechanical fix and subsystem as an individual **optional improvement** toggle, and surrounding the combat with modern training tools, single-player modes, audio jukebox options, and rollback netplay.
- Once the faithful **port** is complete, the **Definitive Overhaul** becomes the default experience, while the 100% faithful 1987 **port** remains a one-click preset at all times.

---

## 2. Goals, Non-Goals, and Anti-Goals

### Goals
1. **Uncompromised Faithful Baseline:** Preserve 100% **gameplay fidelity**, **visual fidelity**, and **audio fidelity** against the **reference version** (`sf`) when running in `Faithful 1987` mode, passing all per-frame screen, RAM, and sound-latch comparison tests.
2. **Granular Toggleability:** Expose every mechanical fix, balance adjustment, and era subsystem as an individual toggle, organized under one-click presets (*Faithful 1987*, *Classic Competitive*, *Pure Footsies / No Meters*, *Definitive Overhaul Default*, and *Custom*).
3. **1987 Visual Consistency:** Maintain the original `384×224` pixel-art resolution, palette constraints, and 1987 Capcom arcade visual style. All new animations (missing boss states, later-era moves) and new UI screens (12-character select, meters, endings) must look like authentic 1987 arcade artwork, produced via an AI image-generation and palette/tile quantization pipeline.
4. **Full 12-Fighter Roster with Historical Era Styles:** Make all 12 characters playable with complete movesets in `SF1` (raw 1987) and `SF1+` (completed, balanced 1987 kit), plus selectable era styles (`SFII`, `SFZ`, `SFIII`, `SFIV`, `SFV`, `SF6`) for every entry in which that character canonically appeared.
5. **Definitive Mode & Tooling Suite:** Include 12-Character Arcade Mode (with endings and rival dialogues), Versus, *Alpha 3*-style "1987 World Tour" & 2v1 Dramatic Battle, *SF6*-grade Training Mode (Hitbox/Loose+Tight Box viewer, 60-cell Frame Meter, Replay Takeover), Audio Jukebox (1987 YM2151+MSM5205 vs. arranged CD audio), full input remapping, and deterministic rollback netplay.

### Non-Goals
1. **3D Graphics or HD Sprite Redraws:** We are not replacing the 1987 sprites with high-resolution vector art, 3D models, or mismatched late-90s CPS-2/CPS-3 sprite rips that clash with the 1987 stage and character proportions.
2. **Adding Non-1987 Roster Characters Required for Core Scope:** The roster anchor is the 12 fighters of the 1987 tournament (Ryu, Ken, Retsu, Geki, Joe, Mike, Lee, Gen, Birdie, Eagle, Adon, Sagat).

### Anti-Goals (What Would Make This a Failure Even If It Works)
1. **Breaking Faithful Verification:** Any overhaul change that alters a single frame, pixel, RAM byte, or sound latch when `Faithful 1987` mode is selected is a hard failure.
2. **Inescapable Modern "Robbery" Without Toggles:** Forcing players into untoggleable comeback mechanics, uncheckable neutral skips, or meterless corner throw loops without the ability to switch to honest footsies.
3. **Grind Locks or Predatory Gating:** Locking palette colors, costumes, characters, or styles behind survival grinds (like launch *SFV*) or currency/unlock walls (like *SF6*). Everything is unlocked and selectable out of the box.
4. **Input-Eating Screen Freezes:** Screen-freeze effects (such as Super flashes or Drive Rush pauses) must never eat player directional or button inputs.

---

## 3. Constraints

- **Fidelity Contract ([`docs/project-brief.md`](../project-brief.md), [`ADR-0001`](../adr/0001-recover-original-logic-in-rust.md)):** The faithful **port** recovery (including the active arcade-flow batch in [`docs/planning/arcade-flow-batch.md`](../planning/arcade-flow-batch.md)) takes priority as the foundation. Overhaul features must not block or corrupt the traceable 68000-to-Rust recovery.
- **Local ROM & Asset Boundary:** Original 1987 ROM files (`tools/reference/local/roms/sf`) remain local and gitignored. Overhaul move tables, AI-generated sprite delta packs, and arranged audio tracks are loaded as modular external/overlay assets alongside the verified local ROM set.
- **Deterministic Fixed-Timestep Simulation:** Both the faithful engine (`sf-engine`) and the overhaul layer must advance deterministically at 60 Hz (`step(inputs) -> State`) with serializable state snapshots so that capture verification, **Replay Takeover**, and **Rollback Netplay** work from the exact same snapshot mechanism.

---

## 4. Architectural Approaches Considered

We evaluated three architectures for housing both the faithful 1987 **port** and the **Definitive Overhaul** in the Rust workspace:

### Approach 1: Dual-Engine Split (Frozen 1987 `Bus` + Separate Overhaul Engine)
- **Description:** Freeze [`sf-engine`](../../crates/sf-engine/src/lib.rs) (`Bus`, `VersusMachine`, `CpuMachine`) solely for `Faithful 1987` mode and build a completely separate `sf-overhaul` combat engine that imports decoded ROM assets.
- **Trade-offs:** Protects MAME RAM verification completely and frees the overhaul from 64 KB RAM limits, but duplicates core 1987 physics/collision logic and prevents toggling a single surgical fix (e.g., "Faithful 1987 + positive-edge buttons only") directly on the verified 1987 engine.

### Approach 2: Single Unified `Bus` Engine (Extend 68000 Translation In-Place)
- **Description:** Put all overhaul toggles, era styles (`SFII`–`SF6`), gauges, and new moves directly inside [`crates/sf-engine/src/fighter.rs`](../../crates/sf-engine/src/fighter.rs)'s memory-mapped `Bus` routines.
- **Trade-offs:** Single codebase for single-toggle tweaks, but shoehorning 8 eras of meters, Gen stance switching, and 3-fighter Dramatic Battles into 1987 bitpacked entity flags (`+9`, `0xff86b0`, `0xffe400`) makes [`fighter.rs`](../../crates/sf-engine/src/fighter.rs) brittle and risks breaking MAME RAM parity.

### Approach 3: Two-Tier Hybrid — Surgical `Bus` Toggles + Data-Driven Overhaul Layer (Selected)
- **Description:**
  - **Tier 1 (`sf-engine` Core Toggles):** Add a zero-cost `CoreToggles` configuration struct to [`sf-engine`](../../crates/sf-engine/src/fighter.rs) (defaulting to all-off for MAME tests) that implements surgical 1987 fixes directly at the recovered 68000 routine boundaries (`0x3b3ae`, `0x40ae4`, `0x403d4`, `0x452e0`, `0x408da`, `0x3ff2c`, `0x44880`).
  - **Tier 2 (`sf-overhaul` Extension Layer):** Build a data-driven overhaul crate on top of `sf-engine`, `sf-video`, and `sf-audio` that manages the AI-generated 1987-style sprite/tile extension pipeline, the per-character Era Style & Gauge state machines (`SF1+` through `SF6`), multi-fighter modes (2v1 Dramatic Battle, 1987 World Tour), Training Mode overlays, and Rollback Netplay.
- **Why Selected:** It allows single 1987 fixes to run natively on the verified `Bus` immediately, keeps 100% of MAME comparison tests untouched when `CoreToggles::faithful()` is active, and gives the multi-era overhaul (`SF1+`–`SF6`, new animations, Dramatic Battle) a clean, unconstrained Rust data model.

---

## 5. Detailed Design

### 5.1 Presets & Toggle Hierarchy

At launch (once the faithful port is completed), the game defaults to the **`Definitive Overhaul`** preset. Players can switch presets at any time from the title screen, mode select, or pause menu:

1. **`Faithful 1987` (Port Baseline):** All `CoreToggles` off; only Ryu (1P) and Ken (2P) selectable in their `SF1` style; 100% byte-identical behavior to the MAME **reference version** (`sf`).
2. **`Classic Competitive`:** Original 1987 engine rules with all 12 fighters selectable in their raw `SF1` ROM state, plus optional sub-format bans matching the SF1Cord community (*Standard*, *Ubers*, *Adonarchy*).
3. **`Pure Footsies (1987+)`:** All 12 fighters selectable with their completed `SF1+` (or later-era) movesets and Tier 1 input/blocking/pacing fixes enabled, but **all super/comeback/drive meters disabled** for pure ground-based neutral.
4. **`Definitive Overhaul` (Default after faithful completion):** All 12 fighters selectable across all of their historical Era Styles (`SF1`, `SF1+`, `SFII`, `SFZ`, `SFIII`, `SFIV`, `SFV`, `SF6`), each paired with its era's moveset and signature gauge/mechanics, with Tier 1 input/blocking/pacing fixes enabled.
5. **`Custom`:** Any combination of individual Tier 1 and Tier 2 toggles saved as a user profile.

---

### 5.2 Tier 1: Surgical 1987 Engine Toggles (`sf-engine::CoreToggles`)

Each toggle below hooks a specific translated routine in [`crates/sf-engine/src/fighter.rs`](../../crates/sf-engine/src/fighter.rs) or [`tasks.rs`](../../crates/sf-engine/src/tasks.rs):

| Category | Toggle Name | Faithful (`false`) Behavior | Overhaul (`true`) Behavior |
| :--- | :--- | :--- | :--- |
| **Inputs** | `positive_edge_attacks` | `0x3b3ae` latches button strength only on release (`release + 1`); 255-frame hold saturates. | Latches strength immediately on button press (positive edge) with no 255-frame lock; optional negative-edge toggle for specials. |
| **Inputs** | `recovery_input_buffer` | Tail `0x4050c` drops latched buttons while busy (`+9 & 0xc0 == 0xc0`). | Retains attack/special inputs within a configurable 4-frame buffer window before move recovery ends. |
| **Specials** | `lenient_command_trackers` | `0x40ae4` resets after `0x12` (18) frames from direction 1 or immediately upon leaving direction 3. | Expands total command window to 24 frames, keeps `done = 0xff` buffered for 8 frames after releasing the final direction, and accepts shortcut diagonals. |
| **Movement** | `interruptible_walk_cycle` | Walk stream sets `+9 = 0x91` (gated) for 8 of every 10 frames (`0x403d4`), deferring specials, crouches, and jumps. | Allows special moves, blocks, crouches, and jumps to interrupt `0x91` walk frames immediately, and removes the instant 16 px frame-1 back-walk shift (`sf-normals-04`) on attack startup. |
| **Jumping** | `anytime_jump_attacks` | Jump stream only checks buffered air attacks at the exact apex (`0x452e0` / `0x453b6`); reduces pre-jump crouch only if toggled. | Allows jump attacks to fire on any rising, apex, or descending frame; optionally trims the 9-frame pre-jump crouch to 3–4 frames. |
| **Blocking** | `standard_high_low_block` | Standing guard (`move 4`) ignores lows (`0xc4`), walking back into hits; crouching guard (`move 8`) is triggered by jump attacks (`0xc1`) that hit it anyway (`sf-blocks-05`). | Holding Back blocks standing and jumping attacks; holding Down-Back blocks low attacks (and blocks or transitions cleanly against jump/mid attacks); proximity block activates without requiring `+9` to already be active; fixes Lee/Adon 16f "Omniblock" hurtbox bug. |
| **Balance** | `balanced_damage_and_chip` (now part of `balanced_roster`, SP1 #247) | Specials deal 20/48 HP (41.7%); multi-hit collisions OHKO; chip deals 8–16/48 HP (0 on Stage 4). | Scales specials to ~15–20% life, prevents multi-hit/Scuttle instant-kill glitches, scales chip damage to ~2–3% life across all stages, and adds a `chip_death` on/off toggle. |
| **Balance** | `symmetric_shotos_and_blockable_dp` (now part of `balanced_roster`, SP1 #247) | Ryu's *Shoryuken* (`0x45efa`) is unblockable and frame-1 invincible through recovery; Ken has slower *Hadoken* recovery and `MK`. | Makes *Shoryuken* blockable, gives *Shoryuken* vulnerable landing recovery, and normalizes baseline frame data between Ryu and Ken in `SF1+`. |
| **Pacing** | `disable_fireball_slowdown` | Unblocked *Hadoken* triggers `Scene Event 9` (`0x3ff2c`), sleeping 6 frames/step for 90 steps. | Skips `Scene Event 9` on non-KO projectile hits so fights run at an uninterrupted 60 FPS (keeping slow-motion for the final round-winning KO hit). |
| **Timer & Rules** | `real_second_timer_and_draws` | Timer (`0x3f4e2`) ticks every 19 passes (~31.7s for "100"); 1P tied match counts as CPU win (`0x3eefa`). | Ticks timer once every 60 frames (true seconds) and awards an extra Draw/Sudden-Death round on a tied match. |
| **CPU AI** | `fair_cpu_and_sagat_scaling` | React branch (`0x4571a`) reads P1 `+9 bit 6` on frame 1; Retsu `0x18` is unblockable (`0x468de`); Sagat has 1 static AI table across all 8 DIP settings. | Adds human-like reaction delay to `0x4571a`, makes Retsu `0x18` blockable, and populates distinct Easy/Normal/Hard/Very Hard AI tables for Sagat and Adon. |

---

### 5.3 Tier 2: Character Roster, Historical Era Styles, and Gauge Systems (`sf-overhaul`)

#### A. Character × Era Style & Sub-Revision Matrix
Every fighter is selectable in **`SF1`** (raw 1987 ROM behavior) and **`SF1+`** (completed, balanced 1987 moveset). In addition, any fighter who appeared in a subsequent *Street Fighter* entry receives a distinct top-level **Era style** for **every arcade/major revision** they appeared in, selected on the Character Select screen via a **two-step `Era Family -> Revision (& Loadout)` selector** (where Loadout selects ISMs, Super Art I/II/III, Ultra I/II/Double, or V-Skill/V-Trigger I/II):

| Fighter | Country | Available Era Families & Sub-Revision Styles | Key Overhaul Moveset Additions (`SF1+` and Later Styles) |
| :--- | :--- | :--- | :--- |
| **Ryu** | Japan | `SF1`, `SF1+` \| `SF2 WW`, `CE`, `HF`, `Super`, `ST` \| `SFZ1`, `Z2`, `Z2A`, `Z3` \| `SF3 NG`, `2I`, `3S` \| `SF4`, `SSF4`, `AE2012`, `USF4` \| `SF5`, `AE`, `CE` \| `SF6` | Full 6-button normals, throws, *Shakunetsu Hadoken* (`Super`+), *Joudan Sokutou Geri* (`SF3`+), *Hashogeki* (`SF6`), era Supers/Ultras/Critical Arts. |
| **Ken** | USA | `SF1`, `SF1+` \| `SF2 WW`, `CE`, `HF`, `Super`, `ST` \| `SFZ1`, `Z2`, `Z2A`, `Z3` \| `SF3 NG`, `2I`, `3S` \| `SF4`, `SSF4`, `AE2012`, `USF4` \| `SF5`, `AE`, `CE` \| `SF6` | Full 6-button normals, multi-hit flaming *Shoryuken*, crazy kicks (*Kamabarai* / *Oosoto Mawashi Geri*), *Jinrai Kick* / *Dragonlash* (`SF6`), era Supers. |
| **Sagat** | Thailand | `SF1`, `SF1+` \| `SF2 CE`, `HF`, `Super`, `ST` \| `SFZ1`, `Z2`, `Z2A`, `Z3` \| `SF4`, `SSF4`, `AE2012`, `USF4` \| `SF5 AE`, `CE` | New 1987-style crouch, jump, and air-attack sprites; High/Low *Tiger Shot*, *Tiger Uppercut*, *Tiger Knee Crush*, *Angry Charge* (`SF4`+), *Tiger Cannon* / *Tiger Genocide* / *Tiger Destruction*. |
| **Gen** | China | `SF1`, `SF1+` \| `SFZ2`, `Z2A`, `Z3` \| `SF4`, `SSF4`, `AE2012`, `USF4` | Completed `SF1+` assassin kit (*Hyakurenko* hands, *Gekirou* kicks, wall/double-jump dive kick); full **Mantis (*Sou*) & Crane (*Ki*) stance switching** in `SFZ` and `SF4` families with *Zan'ei* and *Shitenshuu*. |
| **Adon** | Thailand | `SF1`, `SF1+` \| `SFZ1`, `Z2`, `Z2A`, `Z3` \| `SSF4`, `AE2012`, `USF4` | Command *Jaguar Kick* (grounded & airborne), *Rising Jaguar*, wall-rebound *Jaguar Tooth*, *Jaguar Varied Assault* / *Jaguar Avalanche*. |
| **Birdie** | England | `SF1`, `SF1+` \| `SFZ1`, `Z2`, `Z2A`, `Z3` \| `SF5`, `AE`, `CE` | Completed `SF1+` brawler kit (*Bull Head* headbutt, command grab); `SFZ` *Bull Horn*, *Murderer Chain*, *The Birdie*; `SF5` banana/can/donut items (*Break Time* / *Enjoy Time*) and *Hanging Chain*. |
| **Eagle** | England | `SF1`, `SF1+` \| `SFZ3 Upper` | Completed `SF1+` escrima normals and command specials; `SFZ3 Upper` *Manchester Black*, *Liverpool White*, *Canterbury Blue*, *St. Andrew's Green*, *Union Jack* / *Manchester Gold*. |
| **Retsu** | Japan | `SF1`, `SF1+` \| `SF6` | Completed `SF1+` Shorinji Kempo normals, advancing palm strike, and projectile-leaping *Hakai Geri*; `SF6` World Tour hybrid moveset + Drive mechanics. |
| **Geki** | Japan | `SF1`, `SF1+` | Dedicated standing/crouching guard (decoupled from teleport), 6-button claw/kick normals, command shuriken toss (`QCF+P`), directional ninja teleport (`↓,↓+P/K`), and assassination dive. |
| **Joe** | USA | `SF1`, `SF1+` | Full 6-button kickboxing normals, command `"Power!"` jumping spin kick, rolling sobat, low-to-high combination chains, and hurricane kick blitz. |
| **Mike** | USA | `SF1`, `SF1+` | New 1987-style jumping attacks and crouching kicks; dash straight, lunging uppercut, knee-dash blitz, and headbutt/turn-punch specials. |
| **Lee** | China | `SF1`, `SF1+` | Full 6-button Bajiquan normals, hopping double-kick special, spinning backfist, elbow charge, and multi-kick flurry. |

#### B. Cross-Era Interaction Rules, Throws, and Era Gauges
1. **Cross-Era Hitstun, Defense, & Health Normalization:**
   - **Attacker-Driven Hitstun & Combos:** When an `SF1+` through `SF6` fighter lands a hit on any opponent (including an `SF1` opponent), standard comboable hitstun/juggle rules apply until the combo ends; when an `SF1` fighter lands a hit, 1987 single-hit knockback and invulnerable recovery (`+0xf = 0x08`) apply.
   - **Defender-Driven Guard & Defense:** The defender's own Era style and active `CoreToggles` govern how they block, Parry, Focus, V-Shift, or Alpha Counter.
   - **Normalized Health with Optional Unscaled 1987 Lethality:** Internal health is normalized (e.g., 1,000 HP mapped to the 11 HUD cells) so `SFII`–`SF6` damage tables work natively, while `SF1` attacks deal balanced `SF1+` damage by default, with a toggle for *"Unscaled 1987 42% Special Damage"*.
2. **Throw Mechanics & Corner Throw-Loop Prevention:**
   - All 12 fighters receive ground throws in `SF1+` and later styles (raw `SF1` has no throws unless toggled).
   - By default in Overhaul modes, throws are **techable** and wakeup throws in the corner apply **anti-throw-loop pushback** so attackers cannot walk up and loop meterless corner throws.
   - Individual toggles allow enabling *"Authentic Untechable SF2 Throws"* (for `SF2 WW`–`Super` styles) and *"Allow Corner Throw Loops"*.
3. **Era Gauges (Coupled by Default, Separable by Toggle):**
   - **`SF1` / `SF1+` / `SF2 WW`–`Super`:** No meter.
   - **`SF2 ST`:** 1-stock **SUPER** gauge.
   - **`SFZ1`–`SFZ3`:** 3-level **Alpha Gauge** (with `A-Ism` / `X-Ism` / `V-Ism` Custom Combo selectable in `SFZ3`), **Alpha Counter**, chain combos, and air recovery/air block.
   - **`SF3 NG`–`3S`:** **Parry** (10f tap forward/down, air parry), **EX Specials** (`2I`/`3S`), universal overhead, and **Super Art I, II, or III**.
   - **`SF4`–`USF4`:** 4-bar **Super Gauge** + **Revenge (Ultra) Gauge**; **Focus Attack / FADC**, **Red Focus & Delayed Wakeup** (`USF4`), and **Ultra I, II, or Ultra Combo Double**.
   - **`SF5`–`SF5 CE`:** 3-bar **EX / Critical Art Gauge** + **V-Gauge** (**V-Skill I/II**, **V-Trigger I/II**, **V-Reversal**, **V-Shift** in `CE`, and **Crush Counters**).
   - **`SF6`:** 6-bar **Drive Gauge** (with **Burnout** on depletion, **Overdrive**, **Drive Parry / Perfect Parry**, **Drive Impact**, **Raw / Cancel Drive Rush**) + 3-level **Super Art Gauge**.
   - **Decoupling & Safety Toggles:** Match settings allow locking gauges to *Era Default (Asymmetric)*, *No Gauges (Pure Footsies)*, or *Unified Gauge*, plus individual switches to disable **Raw Drive Rush inertia**, **Meterless Corner Throw Loops**, **Health-Gated Comeback Scaling**, and **Screen-Freeze Input Lockout**.

---

### 5.4 Extended Scene Compositor, AI Sprite Pipeline, & 1987 Audio Encoding

1. **Extended Scene Compositor (`sf-video`):**
   - `Faithful 1987` mode continues to use the untouched `render_frame(assets, scene)` pipeline in [`crates/sf-video/src/render.rs`](../../crates/sf-video/src/render.rs) (64 object-RAM sprite entries, 16 sprite sub-palettes, and 2bpp 1,024-tile text layer).
   - Overhaul modes preserve the exact `384×224` pixel resolution and 15-color + transparent per-sprite art rules, but render through an **Extended Scene Compositor** in `sf-video` that lifts the 64-sprite-entry ceiling (needed for 3-fighter Dramatic Battles and multi-effect sequences), expands palette banks and tile indices beyond 16-bit ROM bounds, adds a pixel-art HUD layer for Era gauges, and composites a post-frame debug/training overlay layer for the Hitbox Viewer and 60-cell Frame Meter.
2. **AI Sprite Generation & Era Costumes (`tools/asset-gen`):**
   - **Reference Extraction:** Export existing 1987 sprite sheets and 16-color sub-palettes from the local `sf` ROMs using `sf-video::decode`.
   - **AI Pose Synthesis & Era Costumes:** Use AI image generation conditioned on the character's 1987 sprites (matching 1987 proportions, dark outlines, and shading) to generate missing move frames (e.g., Sagat crouch/jump, Mike air attacks, later-era specials, 12-character endings). Every Era style defaults to the fighter's 1987 outfit look, while also offering optional 1987-pixel-art **Era costumes** (such as bearded *SF6* Ryu or *Alpha*/*SFV* Birdie) alongside palette swaps.
   - **1987 Quantization & Tile Packing:** Quantize generated frames strictly to 15-color + transparent sub-palettes and slice them into `16×16` tiles in versioned overhaul packs (`assets/overhaul/*.pack`), mounted only in Overhaul modes.
3. **Era-Authentic 1987 Audio Encoding (`sf-audio`):**
   - All new voice callouts and move SFX for the 10 bosses and later Era styles are encoded into authentic **1987 4-bit 8 kHz MSM5205 ADPCM** and **YM2151 FM** formats by default so new moves sound indistinguishable from the 1987 arcade sound board, paired with the Jukebox toggle for Red Book CD arranged music.

---

### 5.5 Game Modes, Controls, CPU AI, & Netplay

1. **Control Schemes (Per-Player Selectable):**
   - **Classic 6-Button (Default):** Standard `LP, MP, HP, LK, MK, HK`.
   - **Simulated Pneumatic 2-Pad Mode:** Historical/novelty mode where button hold duration or gamepad analog trigger pressure (`L2`/`R2`) controls Light/Medium/Heavy punch and kick strength like the 1987 deluxe cabinet.
   - **Modern Controls:** 4-button layout (`Light, Medium, Heavy, Special` + `Assist`/macro buttons) with a toggleable **Neutral Shortcut Startup/Damage Penalty** (defaulting to a 20% damage penalty and startup balance on 1-button neutral specials/supers).
2. **12-Character Arcade Mode:**
   - Select any of the 12 fighters, any available Era style/revision, and any of the 4 starting countries (Japan, USA, China, England) before facing Adon and Sagat in Thailand.
   - Includes positive-edge reworked **Bonus Stages** (Tile/Table Breaking and Moving Board Breaking), mid-match 2P Challenger Join-Ins, and **1987-style pixel-art rival intros and character endings** for all 12 fighters.
3. **Versus, CPU vs. CPU, & Ruleset Presets:**
   - Full 12-character select screen with the 2-step `Era Family -> Revision (& Loadout)` selector, Era costume/palette selector, stage/BGM selector, and one-click Ruleset Presets.
   - **Dual-Tier CPU AI:** `SF1` style uses the translated 1987 table AI (`0x40bd4` / `0x45630`, with optional reaction-delay and Sagat difficulty-scaling fixes), while `SF1+` through `SF6` styles are driven by a **Style-Aware Utility AI** in `sf-overhaul` capable of executing special cancels, Parries, FADCs, V-Triggers, and Drive mechanics.
4. ***Alpha 3*-Style "1987 World Tour" & 2v1 Dramatic Battle:**
   - **1987 World Tour:** Play as any of the 12 canonical fighters across a 5-country branching mission map (with rule modifiers, 2v1 Dramatic Battles, Survival, Time Attack, and Era style mastery trials). All characters, Era styles, Era costumes, and palettes remain unlocked in Versus, Arcade, Training, and Online from the start; **RPG stat-leveling/perk equips** inside World Tour mode can be toggled on or off.
   - **2v1 Dramatic Battle Rules:** Clamps all 3 active fighters inside the `384×224` camera view; defaults the 2-player team to a shared *Alpha*-style lifebar against a boosted Boss (with a toggle for **Independent Lifebars**); defaults **Friendly Fire** to off (with a toggle to enable it).
5. ***SF6*-Grade Training Mode & Replay Takeover:**
   - **Hitbox / Hurtbox Overlay:** Renders the 1987 engine's outer **loose box** (`depth 2`), inner **tight/crit box** (`depth 1`), active attack/projectile boxes, pushboxes, and invulnerability state (`+0xf bit 3`).
   - **60-Cell Visual Frame Meter & Command Inspector:** Color-coded timeline bar showing startup, active, recovery, cancel windows, frame advantage (`+`/`-`), plus a live inspector for the 1987 command trackers (`0x40360–0x403b2`) and walk-gate state (`0x91`/`0x92`).
   - **Scenario Presets, Combo Trials, & Replay Takeover:** Built-in drills (anti-air, whiff punish, throw tech) and the ability to pause any saved match replay and immediately take control of either fighter for 10 seconds.
6. **Audio Jukebox, Input Remapping, Palette Unlocks, & Rollback Netplay:**
   - **Jukebox:** Switch per stage/character between native 1987 YM2151 + MSM5205 synthesis (`sf-audio`) and external Red Book audio tracks (such as the 1988 PC Engine CD *Fighting Street* arranged soundtrack), plus a full Sound Test.
   - **Controls & Palettes:** Full keyboard, XInput/DInput gamepad, and arcade-stick remapping; all character color palettes and Era costumes unlocked out of the box.
   - **Rollback Netplay:** Deterministic state-snapshot rollback netplay that synchronizes a host-defined `MatchConfig` (preset, active toggles, and PRNG seed) at handshake while allowing per-player Fighter/Style/Costume/Control picks and local-only Jukebox BGM. Includes an open room browser with ping indicators, a **Wired-Only Auto-Filter** toggle, and zero geographic region locks.

---

### 5.6 Phased Delivery Roadmap

Because this spans the faithful port plus a massive multi-era overhaul, the work is structured into five sequential phases:

- **Phase 1 — Complete Faithful 1987 Port (`sf-engine`, `sf-video`, `sf-audio`, `sf-dev`):** Finish the approved arcade-flow batch ([`docs/planning/arcade-flow-batch.md`](../planning/arcade-flow-batch.md), tickets F1–F78) so the complete 1987 arcade game runs natively from power-on through all opponents, bonus stages, continues, and endings, verified against MAME 0.289. Non-invasive `sf-dev` diagnostic overlays (such as the Hitbox/Hurtbox and Command-Tracker inspector) may be pulled forward earlier if useful for debugging the remaining 1987 fights.
- **Phase 2 — Tier 1 Core Toggles, *SF1++* Roster, & Training Overlay:** Implement `CoreToggles` in `sf-engine` (positive-edge buttons, lenient specials, walk-gate interrupt, apex-free jump attacks, fixed blocking, balanced damage/chip, no fireball slowdown, Sagat difficulty scaling), add the full Hitbox/Hurtbox + 60-cell Frame Meter + Replay Takeover overlay, and unlock all 12 fighters in raw `SF1` (*SF1++*) mode.
- **Phase 3 — Extended Compositor, AI Asset Pipeline, & `SF1+` Complete 12-Fighter Overhaul:** Build the Extended Scene Compositor in `sf-video`, `tools/asset-gen`, and `sf-overhaul`; generate missing 1987-style crouch/jump/block/attack sprites and 8 kHz ADPCM voice/SFX for the 10 bosses; and ship the complete, balanced **`SF1+`** moveset and 12-character Arcade Mode (with endings and Jukebox) as the new default preset.
- **Phase 4 — Historical Era Styles & Sub-Revisions (`SFII`–`SF6`):** Implement the modular Style & Gauge subsystem, the Style-Aware Utility AI, and the character Era styles/revisions (`SF2 WW`–`ST`, `SFZ1`–`Z3`, `SF3 NG`–`3S`, `SF4`–`USF4`, `SF5`–`CE`, `SF6`) along with optional Era costumes.
- **Phase 5 — 1987 World Tour, 2v1 Dramatic Battle, & Rollback Netplay:** Add the *Alpha 3*-style World Tour campaign (with optional RPG progression toggle), 3-fighter Dramatic Battle mode, and online rollback netplay.

---

## 6. What Is Left to the Builder

The following implementation details are intentionally left to engineering judgment during planning and execution:
- Exact Rust module/crate split between `sf-engine` and `sf-overhaul` (e.g., whether `sf-overhaul` is a sibling crate in `crates/` or a feature-gated module inside `sf-engine`).
- Binary/JSON schema for overhaul move tables, frame data, and quantized 16-color sprite delta packs.
- Specific frame-count tuning for custom `SF1+` boss moves, recovery buffers, and damage/chip scaling curves.
- Choice of Rust gamepad/windowing/netplay crates (e.g., `gilrs` / `sdl2`, `ggrs` for rollback netplay) when replacing or extending `minifb` in `sf-dev`.
- Specific AI image-generation model/workflow and palette-quantization CLI scripts used inside `tools/asset-gen`.
