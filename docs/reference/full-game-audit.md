# Full-game completeness and fidelity audit

Evidence class: **Audit** (tickets #144 `F72` and #149 `F77`).

This document records the full-game completeness and fidelity audit required by
issue #144 (`F72`) at the end of the
pre-test-mode arcade-flow batch (`F1–F71`, tickets `#41–#90` and `#122–#143`),
together with the closing test-mode addendum required by
issue #149 (`F77`) after completing
service / test mode (`F73–F76`, tickets `#145–#148`):

1. A pass/fail account against every user story (`US-1..US-34`), implementation
   decision, and testing decision in the parent specification
   (issue #1).
2. The pre-test-mode run (`133` scenarios, `266` runs in §3) and final
   post-test-mode rerun (`137` scenarios, `274` runs in §7.2) of the translation
   coverage ledger ([coverage-ledger.md](coverage-ledger.md), `F11` / `#51`).
3. An explicit disposition (`retired`, `unreachable under MAME` with the reason,
   `deferred` with its open decision, or `not covered` with the ticket that
   reaches it) for every `note()` / `note_unrecovered()` report site in the
   ledger and every `"Not covered"` item across `docs/reference/*.md`.
4. Every adopted MAME `0.289` behaviour with its exact source location (including
   the unmapped-port read value `0xffff` added by `F73` / `#145`).
5. Citations and outcomes of the three person checks (`F30` / `#70`, `F31` /
   `#71`, and `F54` / `#126`).
6. The closing test-mode addendum (`F77` / `#149` in §7): the results of
   `F73–F76` (`#145–#148`), the ledger rerun over `0x4834e..0x492d5` and its
   helpers, every open decision and every deferred item with its outcome, and
   which requirements pass and which are deferred or fail.

**Scope and non-certification statement.** Sections 1–6 record the pre-test-mode
audit (`F72`, `#144`), where test mode (`F73–F76`, issues `#145–#148`), the
test-mode audit addendum (`F77`, issue `#149`), and the explicitly deferred
items come after it. Section 7 (`F77`, `#149`) appends the completed test-mode
results and final account. **This audit does not certify the complete port:**
the explicitly deferred items recorded in §3 and §7.4 remain uncertified.
Nothing that does not pass is certified.

Code: `crates/sf-engine/tests/full_game_audit.rs`,
`crates/sf-engine/tests/service_mode.rs`,
`crates/sf-engine/tests/full_1p.rs`, `crates/sf-engine/tests/full_2p.rs`,
`crates/sf-audio/tests/both_chips.rs`, `tools/reference/coverage_ledger.py`.

---

## 1. End-to-end full-game baseline before test mode

Before evaluating individual user stories, the pre-test-mode batch (`F1–F71`)
establishes two continuous power-on-to-completion verification baselines on
`PowerOnMachine` (`sf_engine::tasks::PowerOnMachine`) and both sound Z80
translations (`sf_audio::music::MusicCpu`, `sf_audio::samples::SamplesCpu`):

| Baseline | Scenario | Frames | 68000 comparison (`C-NAT`) | Both sound programs (`F71` / `#143`) | Evidence |
| --- | --- | --- | --- | --- | --- |
| Complete one-player game from power-on (boot -> attract -> coin/start -> country select -> all 10 opponents Retsu, Geki, Joe, Mike, Lee, Gen, Birdie, Eagle, Adon, Sagat -> all 4 bonus stages -> 3 travel maps -> ending -> ranking check & name entry -> attract ranking with blinking marked entry) | `sf-full-1p-01` | `0..=45,260` (`45,261` frames, `9,105` full checkpoints across 15 windows) | `45,260` frames (`1..=45,260`) and `9,104` post-frame-0 checkpoints match `full-1p-01-run-a`; owned RAM byte-identical at all `9,105` checkpoints; `unrecovered` empty (`0` untranslated branches) | Reference write times (`95` latch writes): YM2151 (`1,572` writes), MSM5205 (`400` port writes, `302,045` ticks), and music RAM at all `23,365` idle of `45,260` frame ends match (`0` diffs); native latch identical to reference (`95 == 95`) | [full-1p.md](full-1p.md) (`#137` / `F65`), [both-chips.md](both-chips.md) §12 (`#143` / `F71`) |
| Complete two-player and challenger session from power-on (boot -> attract demo -> coin kills demo slot 2 -> 2P start -> 2P versus -> P2 win carry-over into mode 4 vs Retsu -> P1 challenger joins P2 -> P1 win carry-over into mode 2 vs Retsu -> P1 loss -> continue accepted -> P1 loss -> continue expired -> game over -> attract) | `sf-full-2p-01` | `0..=14,100` (`14,101` frames, `5,021` full checkpoints across 6 windows) | `14,100` frames (`1..=14,100`) and `5,020` post-frame-0 checkpoints match `full-2p-01-run-a`; owned RAM byte-identical at all `5,021` checkpoints; `unrecovered` empty (`0` untranslated branches) | Reference write times (`59` latch writes): YM2151 (`765` writes), MSM5205 (`169` port writes, `94,098` ticks), and music RAM at all `9,195` idle of `14,100` frame ends match (`0` diffs); native latch identical to reference (`59 == 59`) | [full-2p.md](full-2p.md) (`#138` / `F66`), [both-chips.md](both-chips.md) §12 (`#143` / `F71`) |

---

## 2. Pass/fail account against the parent specification (`#1`)

### 2.1 User stories (`US-1..US-34`)

Verdict vocabulary:
- **Pass:** verified end-to-end against the pinned MAME `0.289` reference (and
  by a person where the requirement calls for human review or keyboard play)
  within the approved pre-test-mode scope.
- **Pass (with deferred items noted):** passes on all ticketed paths; explicitly
  deferred paths (with their approved user decisions from
  [arcade-flow-batch.md](../planning/arcade-flow-batch.md) §7) are listed in the
  rightmost column and are not certified.

| Story | Requirement summary | Verdict | Primary evidence (`docs/reference/*.md`, tests) | Deferred / uncertified items |
| --- | --- | --- | --- | --- |
| **US-1** | Windows-native port playable on PC | **Pass** | `sf-dev` native launcher (`--mode power-on`, `cpu`, `versus`), [launcher.md](launcher.md) (`#38`, `#76` / `F36`, `#71` / `F31`) | Host 60 fps wall-clock pacing and 3x window upscale are smoke-checked and human-played (`F31`), not byte-compared |
| **US-2** | US set 1 (`sf`) with six-button controls as the single reference version | **Pass** | `tools/reference/rom-manifest.json` (`44` ROM entries verified by size/CRC32/SHA1), [mame-baseline.md](mame-baseline.md) (`#2`) | None |
| **US-3** | Original movement and facing behavior (walks, proximity stop, camera, stage edges, slot swap, turn) | **Pass** | [movement.md](movement.md) (`#6`), [slot-swap.md](slot-swap.md) (`#16`), [posture-dispatch.md](posture-dispatch.md) (`#15`), [one-player-combat.md](one-player-combat.md) (`#54` / `F14`, `sf-1p-held-01`) | None |
| **US-4** | Original jump and crouch behavior (neutral/diagonal jumps, crouch, timing and limits) | **Pass** | [crouch.md](crouch.md) (`#7`), [jump.md](jump.md) (`#8`), [posture-dispatch.md](posture-dispatch.md) (`#15`), [one-player-combat.md](one-player-combat.md) (`#54` / `F14`, `sf-1p-crouch-01`, `sf-1p-jump-01`) | None |
| **US-5** | Each original normal attack and strength (standing, crouching, jumping LP/MP/HP/LK/MK/HK for Ryu and Ken) | **Pass** | [punch.md](punch.md) (`#9`), [standing-attacks.md](standing-attacks.md) (`#17`), [crouch-attacks.md](crouch-attacks.md) (`#18`), [jump-attacks.md](jump-attacks.md) (`#19`), [ken-attacks.md](ken-attacks.md) (`#47` / `F7`), [one-player-combat.md](one-player-combat.md) (`#54` / `F14`) | Unreached Ken blocked-kick recoil streams (`0x3028`, `0x3192`, `0x3172`) are translated from the listing, not separately reached in `sf-ken-05` |
| **US-6** | Original special-move recognition and execution (Hadoken, Shoryuken, Tatsumaki, negative-edge button latch, command trackers) | **Pass** | [specials.md](specials.md) (`#21`), [hadoken.md](hadoken.md) (`#22`), [ken-attacks.md](ken-attacks.md) (`#47` / `F7`), [one-player-specials.md](one-player-specials.md) (`#53` / `F13`, `sf-1p-hadoken-01`, `sf-1p-tatsumaki-01`) | Stages `1–5` of the 1P Hadoken chip/box tables (`0x4340a`, `0x434d0`) and Tatsumaki chip table (`0x45fa8`) are translated as indexed ROM reads, exercised at stage 0 (`F13`) |
| **US-7** | Original hit detection, damage, blocking, and reactions (including projectile slow motion event 9) | **Pass (with deferred items noted)** | [blocking.md](blocking.md) (`#20`), [projectile-slow.md](projectile-slow.md) (`#33`), [one-player-combat.md](one-player-combat.md) (`#54` / `F14`), [sagat.md](sagat.md) (`#135` / `F63`, Tiger Shot chip and hit) | 38 of the 67 CPU hit-function table entries (`0x46472..0x480d0`) that no pinned scenario reaches are deferred (`#1` open decision 8; see §3.1 and §3.2) |
| **US-8** | Original opponent decisions and timing across all 10 CPU opponents | **Pass (with deferred items noted)** | [cpu-decision.md](cpu-decision.md) (`#12`), [ai-tables.md](ai-tables.md) (`#23`), [cpu-follow-up.md](cpu-follow-up.md) (`#25`), [geki.md](geki.md) (`#67` / `F27`), [joe.md](joe.md) (`#68` / `F28`), [mike.md](mike.md) (`#84` / `F44`), [lee.md](lee.md) (`#125` / `F53`), [gen.md](gen.md) (`#129` / `F57`), [birdie.md](birdie.md) (`#131` / `F59`), [eagle.md](eagle.md) (`#132` / `F60`), [adon.md](adon.md) (`#134` / `F62`), [sagat.md](sagat.md) (`#135` / `F63`), [full-1p.md](full-1p.md) (`#137` / `F65`) | Unreached CPU move-stream and hit-function branches (such as Geki's `0x4594c` and `Equal`/`Greater` shuriken branches) are deferred (`#1` open decision 8) |
| **US-9** | Original health, timer, round results, and transitions (KO, double KO, time-over win/loss/draw, round 3, bonus tallies) | **Pass (with deferred items noted)** | [round-timer.md](round-timer.md) (`#24`), [round-two.md](round-two.md) (`#26`), [round-result.md](round-result.md) (`#28`), [round-transition.md](round-transition.md) (`#29`), [round-outcomes.md](round-outcomes.md) (`#30`, `#72` / `F32`: `sf-1p-round3-01`, `sf-1p-draw-01`, `sf-1p-doubleko-01`), [match-win.md](match-win.md) (`#31`) | A 1P time-over draw with equal vitality (`0x3edc2`) was not found within `F32`'s 200-candidate search budget against the live CPU; `0x3edc2 -> 0x3ee04` is verified in 2P `sf-timeover-01`, and the mode-2 `0x3ee04 -> 0x3eefa` continuation is verified in `sf-1p-draw-01` and `sf-1p-doubleko-01` (`round-outcomes.md`) |
| **US-10** | Exact original game pixels before scaling (bg, fg, tx, sprites, palettes, scroll, layer enables, flip screen) | **Pass** | [first-frame.md](first-frame.md) (`#4`), [flip-screen.md](flip-screen.md) (`#58` / `F18`), [power-on-dips.md](power-on-dips.md) (`#139` / `F67`), `sf-video` byte-exact `384x224` pixel checks across all scenario checkpoints | None |
| **US-11** | Original sound events, timing, pitch, and character (YM2151 + dual MSM5205 ADPCM, all 7 sample banks `0–6`, all FM effects) | **Pass** | [hit-sound.md](hit-sound.md) (`#10`), [fight-samples.md](fight-samples.md) (`#32`), [fm-effects.md](fm-effects.md) (`#34`), [adpcm-samples.md](adpcm-samples.md) (`#78` / `F38`), [service-sound-test.md](service-sound-test.md) (`#62` / `F22`, `#64` / `F24`), [both-chips.md](both-chips.md) (`#37`, `#48` / `F8`, `#49` / `F9`, `#70` / `F30`, `#126` / `F54`, `#143` / `F71`) | Constant interrupt-latency stand-in (`51` in tests, `92` in launcher) within the `45–138`-cycle band (`#40` decision 6; see §4); unreached ROM-dead FM effect opcodes (`0x0e62`, slot B, noise tick) are unreachable with this ROM's data |
| **US-12** | Original music behavior (all 21 tunes `0x28..0x3e`, stop/pause/resume `0x27`/`0x3f`/`0x80`, round-tune loop, Attract Music & Demo Sounds DIPs) | **Pass** | [music-command.md](music-command.md) (`#11`), [music-tunes.md](music-tunes.md) (`#13`, `#63` / `F23`), [music-stops.md](music-stops.md) (`#35`), [music-power-on.md](music-power-on.md) (`#36`), [attract-music-off.md](attract-music-off.md) (`#77` / `F37`, `#87` / `F47`), [both-chips.md](both-chips.md) (`#70` / `F30`, `#126` / `F54`, `#143` / `F71`) | Unreached second-level sub-phrase nesting (`0x09e7`) and null opcode `0x0a` (`0x0000`) are unreachable with this ROM's music streams (`F23`) |
| **US-13** | Keyboard controls for the first playable milestone | **Pass** | [launcher.md](launcher.md) (`#38`, `#50` / `F10`, `#71` / `F31`) | None |
| **US-14** | Remappable keyboard controls in the completed port | **Pass** | [launcher.md](launcher.md) (`#76` / `F36`: `--rebind`, `%APPDATA%\sf1-port\bindings.toml`, duplicate-key rejection, `Esc` cancel) | Host option to swap Ken onto P1 keys excluded by `#40` decision 16 (`--rebind` already allows binding any key to P1/P2) |
| **US-15** | Original two-player modes supported with keyboard controls (fresh 2P start, challenger join, P2-wins carry-over into mode 4, P1-wins carry-over into mode 2, versus game over) | **Pass (with deferred items noted)** | [coins.md](coins.md) (`#52` / `F12`), [two-player-start.md](two-player-start.md) (`#86` / `F46`), [challenger.md](challenger.md) (`#81` / `F41`), [versus-carry.md](versus-carry.md) (`#66` / `F26`, `#80` / `F40`), [full-2p.md](full-2p.md) (`#138` / `F66`), [launcher.md](launcher.md) (`#71` / `F31` `human-versus-01.tsv`) | Ken's 1P game past Retsu in mode 4 (`#40` decision 10) and P2's name entry / two-name game over (`#40` decision 3) are deferred |
| **US-16** | Original startup and title flow from power-on (reset RAM clear, vblank hold, boot palette/tilemap setup, warning screen, join into attract title) | **Pass (with test mode in F73–F76)** | [power-on.md](power-on.md) (`#41` / `F1`, `#55` / `F15`), [dip-switches.md](dip-switches.md) (`#42` / `F2`, `#57` / `F17`), [power-on-dips.md](power-on-dips.md) (`#139` / `F67`) | Self-test branch (`0x3b02a -> 0x4834e`) is reported at reset here and translated in `F73–F76` (`#145–#148`) |
| **US-17** | Original credit and start behavior (coin debounce, all 8 Coin A/B settings, service credits, 9-credit cap and coin lockout, 1P/2P start screen, country select for 2 and 4 countries) | **Pass** | [coins.md](coins.md) (`#52` / `F12`), [coinage.md](coinage.md) (`#75` / `F35`), [start-screen.md](start-screen.md) (`#73` / `F33`), [country-select.md](country-select.md) (`#74` / `F34`), [prefight.md](prefight.md) (`#44` / `F4`, `#89` / `F49`), [power-on-dips.md](power-on-dips.md) (`#140` / `F68`, `#142` / `F70`) | Coin slots 3 and 4 are never configured or debounced in the original ROM (`arcade-flow-batch.md` §6) |
| **US-18** | Original attract behavior (scroll, 300-frame title wait, ranking screen with/without marked entry, scripted demo fight through round end and loop back) | **Pass** | [attract.md](attract.md) (`#43` / `F3`, `#56` / `F16`, `#88` / `F48`, `#127` / `F55`), [name-entry.md](name-entry.md) (`#122` / `F50`), [full-1p.md](full-1p.md) (`#137` / `F65`), [full-2p.md](full-2p.md) (`#138` / `F66`) | Attract demo time-over (`0x3ee1e -> 0x3ce8c`) and double-KO (`0x3ed44 -> 0x3ce8c`) cleanup hooks are translated (`F55`); the deterministic scripted demo in ROM ends by KO at frame `3636`, so the time-over/double-KO demo exits are unreachable under standard demo playback |
| **US-19** | Full original progression and all its required content (all 5 countries, all 10 opponents, all 4 bonus stages, travel maps, stage transitions) | **Pass (with deferred items noted)** | [geki.md](geki.md) (`#67`), [joe.md](joe.md) (`#68`), [bonus-japan.md](bonus-japan.md) (`#69` / `F29`, `#128` / `F56`), [travel-map.md](travel-map.md) (`#83` / `F43`), [mike.md](mike.md) (`#84`), [bonus-stage1.md](bonus-stage1.md) (`#124` / `F52`), [lee.md](lee.md) (`#125`), [gen.md](gen.md) (`#129`), [bonus-stage2.md](bonus-stage2.md) (`#130` / `F58`), [birdie.md](birdie.md) (`#131`), [eagle.md](eagle.md) (`#132`), [bonus-stage3.md](bonus-stage3.md) (`#133` / `F61`), [adon.md](adon.md) (`#134`), [sagat.md](sagat.md) (`#135`), [full-1p.md](full-1p.md) (`#137`) | Winning chains from USA start (`#40` decision 2), 4-country starts UK/China (`#40` decision 11), and Ken mode 4 past Retsu (`#40` decision 10) are deferred |
| **US-20** | Original continuation and ending behavior (continue countdown, continue acceptance, continue expiry, game over with/without credits, ending scenes, staff roll, ranking check and name entry) | **Pass (with deferred items noted)** | [cpu-match-win.md](cpu-match-win.md) (`#45` / `F5`), [game-over.md](game-over.md) (`#65` / `F25`, `#85` / `F45`), [continue-accept.md](continue-accept.md) (`#79` / `F39`, `#123` / `F51`), [name-entry.md](name-entry.md) (`#82` / `F42`, `#122` / `F50`), [ending.md](ending.md) (`#136` / `F64`), [full-1p.md](full-1p.md) (`#137`), [full-2p.md](full-2p.md) (`#138`) | Ending scene orders for starting countries 1..3 (`0x44c1c` rows 1..3) and mode-4 ending scroll `0x44d5e` are translated as table-driven/branch code, deferred by `#40` decisions 2, 10, and 11; P2 name entry deferred by `#40` decision 3 |
| **US-21** | Original bugs and unusual timing preserved in the baseline (negative-edge latches, start-screen erase block quirk `0x3e038`, 19-frame timer cadence, vblank hold across reset clear and Freeze spin, Demo Sounds coin override `0x3b802`) | **Pass** | [punch.md](punch.md), [coins.md](coins.md), [round-timer.md](round-timer.md), [power-on.md](power-on.md), [freeze.md](freeze.md), [attract-music-off.md](attract-music-off.md) | None |
| **US-22** | Personal local use first; original ROMs and captures kept local | **Pass** | `.gitignore` excludes `tools/reference/local/`; `sf-video` and tests skip cleanly if local data is absent | None |
| **US-23** | Verified local game data checked repeatably (`44` ROM files by size, CRC32, SHA1) | **Pass** | `tools/reference/rom-manifest.json`, `sf_video::romset`, [mame-baseline.md](mame-baseline.md) (`#2`) | None |
| **US-24** | Pinned comparison baseline (MAME `0.289` commit `774a180df2b6ef9c53e48337792256817c50f768`, deterministic dual-run `C-REF`) | **Pass** | [mame-baseline.md](mame-baseline.md) (`#2`), [comparison-runner.md](comparison-runner.md) (`#3`), `133` pinned scenarios with `run-a == run-b` | None |
| **US-25** | Repeatable starting conditions and frame-indexed inputs | **Pass** | `tools/reference/scenarios/*.json` (`133` scenarios), `sf-compare` scenario schema and loader | None |
| **US-26** | First differing frame and observable reported on mismatch | **Pass** | `sf_compare::compare`, [comparison-runner.md](comparison-runner.md) (`#3`) | None |
| **US-27** | One-frame gameplay difference fails comparison (`C-NEG` negative controls across slices) | **Pass** | Every slice's `C-NEG` test (e.g. `milestone_audit.rs`, `power_on_boot.rs`, `full_1p.rs`, `full_2p.rs`) | None |
| **US-28** | Direct launch into one fixed fight (`--mode cpu` and `--mode versus`) | **Pass** | [launcher.md](launcher.md) (`#38`), [milestone-audit.md](milestone-audit.md) (`#39`) | None |
| **US-29** | Recovery notes tied to original program addresses and observed behavior | **Pass** | `docs/reference/*.md` (`55` reference docs) and [coverage-ledger.md](coverage-ledger.md) (`#51` / `F11`) mapping every executed PC to its Rust citation | None |
| **US-30** | Gameplay and sound-program logic translated into native Rust (no 68000 or Z80 CPU emulation in production) | **Pass** | `crates/sf-engine/src/{fighter,tasks,fight}.rs`, `crates/sf-audio/src/{music,samples}.rs`, [ADR-0001](../adr/0001-native-translation-with-chip-level-sound.md) | None |
| **US-31** | Sound-chip emulation available for synthesis (YM2151 + two MSM5205s driven by native Rust Z80 translations) | **Pass** | `crates/sf-audio/src/{ym2151,msm5205,mix}.rs`, [both-chips.md](both-chips.md) | None |
| **US-32** | Refactoring backed by passing behavior comparisons (such as `FightMachine` unification in `#66` / `F26`) | **Pass** | `crates/sf-engine/src/fight.rs` (`#66` / `F26`), workspace suite passing across all slices | None |
| **US-33** | Known MAME discrepancies and adopted emulator behaviours recorded explicitly with source locations | **Pass** | Section 4 below; [fight-samples.md](fight-samples.md), [music-power-on.md](music-power-on.md), [latch-timing.md](latch-timing.md), [both-chips.md](both-chips.md), [coins.md](coins.md), [power-on.md](power-on.md), [freeze.md](freeze.md) | None |
| **US-34** | Verified facts distinguished from incomplete recovery and untested behavior | **Pass** | [coverage-ledger.md](coverage-ledger.md), [milestone-audit.md](milestone-audit.md), and Sections 2–6 of this audit | Test mode (`F73–F76`) and deferred items are explicitly uncertified |

### 2.2 Testing decisions in the parent specification (`#1`)

| # | Testing decision (`#1`) | Verdict | Account |
| --- | --- | --- | --- |
| 1 | Prefer one high-level scenario replay/comparison boundary over tests coupled to internal translated functions | **Pass** | `sf_compare::compare` and `sf_engine::run` compare frame-end `frames.tsv` observables, full RGBA pixels, video/palette RAM, and owned work/object RAM across all slices |
| 2 | Check externally observable behavior for known starting conditions and inputs | **Pass** | Every scenario test drives `PowerOnMachine` or `FightMachine` from a documented power-on or checkpoint state using the scenario JSON's frame-indexed inputs |
| 3 | Exercise native game session with input handling, rendering, and sound-program behavior; use a narrow host smoke check for keyboard/window/audio integration | **Pass** | `sf-dev --smoke` verifies `PowerOn`, `Cpu`, and `Versus` modes; `F10` (`#50`) and `F31` (`#71`) record and verify synthetic and human keyboard sessions against dual MAME runs |
| 4 | Establish deterministic reference replay (`run-a == run-b`) before treating a scenario as a regression oracle | **Pass** | All `133` scenarios have byte-identical `run-a` and `run-b` captures and byte-identical `run-a` and `run-b` PC-coverage probes |
| 5 | Match gameplay outcomes and timing with no one-frame allowance; compare exact game pixels before host display scaling | **Pass** | `0` frames of timing tolerance across all `C-NAT` tests; `384x224` pre-scale RGBA pixels compared byte-for-byte at every checkpoint |
| 6 | Compare sound events and their timing, pitch, and character; define measurable checks and retain human listening review; byte-identical host audio not required | **Pass** | Hooked register/port/DAC checks (`C-SND`), WAV onset/RMS/correlation/duration/pitch/band-energy checks, and human listening reviews `F30` (`#70`) and `F54` (`#126`) |
| 7 | Test the comparison runner with deliberately altered frames or events (`C-NEG`) | **Pass** | Every slice includes a negative control (`C-NEG`) proving a 1-frame or 1-value perturbation fails comparison at the expected frame |
| 8 | Report the first mismatch, expected and actual observations, baseline identity, and scenario | **Pass** | `sf_compare::compare::Outcome::Mismatch` and `CpuRamOutcome` report the exact frame, field/address, expected value, and actual value |
| 9 | Distinguish a controlled development fixture from an actual complete fight | **Pass** | Every reference doc and [arcade-flow-batch.md](../planning/arcade-flow-batch.md) labels evidence as `PO` (power-on), `DF` (development fixture with live CPU), or `CF` (controlled fixture) |
| 10 | Cover movement, normal attacks, special moves, defensive/damage interactions, opponent behavior, round outcomes, arcade flow, progression, and two-player modes; do not declare full-game fidelity from a few passing captures | **Pass** | Covered across `133` scenarios culminating in `sf-full-1p-01` (`45,261` frames) and `sf-full-2p-01` (`14,101` frames), with explicit non-certification of test mode and deferred items |
| 11 | Record and investigate known baseline discrepancies; never silently weaken expectations | **Pass** | All 8 adopted MAME behaviours and their exact source locations are recorded in Section 4; no silent tolerance is used |
| 12 | Treat prior ROM verification as input evidence, not a gameplay test | **Pass** | ROM verification (`rom-manifest.json`) is used only as asset input; all gameplay and sound claims rest on `C-REF`, `C-NAT`, `C-SND`, and person checks |
| 13 | Finish each slice with its focused behavioral checks and broaden tests when shared changes warrant it | **Pass** | All 72 completed tickets in the arcade-flow batch ship with dedicated integration tests in `crates/sf-engine/tests/`, `crates/sf-audio/tests/`, or `crates/sf-dev` |

---

## 3. Final pre-test-mode coverage ledger (`F11` / `#51`) and dispositions

### 3.1 Final pre-test-mode coverage ledger run

Running `python tools/reference/coverage_ledger.py --check` over all `133`
pinned scenarios (`266` MAME `0.289` probe runs, `run-a == run-b` on all `133`
scenarios) after fixing the 1-frame input-application timing offset in
`build_lua_probe` produces the following totals
([coverage-ledger.md](coverage-ledger.md) §1):

| Scope | Scenarios | `:maincpu` (`0x3b000..0x492d5`) | `:audiocpu` (`0x0000..0x0ffd`) | `:audio2` (`0x0000..0x01b1`) | Total executed PCs | Translated PCs | Untranslated PCs | Unnamed untranslated PCs |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| **Normal play (`Service Mode = Off`, full `F1–F71` pre-test-mode scope)** | `128` | `8,389` exec / `8,389` trans / **`0` untrans** | `1,378` exec / `1,378` trans / **`0` untrans** | `223` exec / `223` trans / **`0` untrans** | **`9,990`** | **`9,990`** | **`0`** | **`0`** |
| **All scenarios (including the 5 `Service Mode = On` scenarios `sf-dip-smoke-03` and `sf-sound-test-01..04`)** | `133` | `8,661` exec / `8,389` trans / `272` service-mode (`F73–F76`) | `1,394` exec / `1,394` trans / **`0` untrans** | `223` exec / `223` trans / **`0` untrans** | **`10,278`** | **`10,006`** | **`272` (`F73–F76`)** | **`0`** |

- **Acceptance 1 verification:**
  - Across the entire pre-test-mode normal-play scope (`128` scenarios with
    `Service Mode = Off`), **no address was executed and not translated (`0`
    untranslated PCs across `:maincpu`, `:audiocpu`, and `:audio2`)**.
  - Across all `133` scenarios, `:audiocpu` (`1,394` PCs) and `:audio2` (`223`
    PCs) have **`0` executed untranslated addresses**, `:maincpu` has **`0`
    unnamed executed untranslated addresses**, and the `272` `:maincpu` PCs in
    `0x3b02a`, `0x3b29a..0x3b2b2`, `0x3b5c8..0x3b5f6`, `0x3b6b4..0x3b6c4`,
    `0x3c014..0x3c032`, and `0x4834e..0x4874a` were executed only by MAME's
    68000 in the 5 `Service Mode = On` scenarios (`sf-dip-smoke-03` and
    `sf-sound-test-01..04`) after `reset()` reported `note(0x4834e)`. Those 6
    routine intervals are owned by `F73–F76` (`#145–#148`) and re-audited in
    `F77` (`#149`).
  - Of the `114` routine intervals in `MAINCPU_ROUTINES`, `112` have
    `executed_pc_count > 0`; the only `2` unexecuted intervals in the entire
    68000 ROM listing are `0x3b664..0x3b672` (`coin-control output helper
    0x3b664`, called by service-menu item 7 in `F75` / `#147`) and
    `0x3bb06..0x3bb0c` (`compare_bytes_15`, which has no caller in the original
    ROMs and is unreachable under MAME).

### 3.2 Disposition of every `note()` and `note_unrecovered()` report site in the ledger

Every one of the `25` report sites in [coverage-ledger.md](coverage-ledger.md)
§4 (`2` reached in `Service Mode = On` scenarios, `23` unreached) has an
explicit disposition (`Unreachable under MAME` with the reason, `Deferred` with
its open decision, or `Not covered` with the ticket that reaches it):

| # | Site (`file:line fn`) | Call | Status | Category | Disposition (reason, open decision, or ticket) |
| --- | --- | --- | --- | --- | --- |
| 1 | `fighter.rs:526 park_from` | `note_unrecovered(0x44506)` | unreached (`0`) | **Unreachable under MAME** | Defensive loop-bound guard (`d0 > 0x600`); caller `0x437fc` (`stage_reset`) passes `d0 = 0x100` and `0x44506` steps `d0` by `0x40` up to `0x600`. |
| 2 | `fighter.rs:840 animate` | `note_unrecovered(routine)` | unreached (`0`) | **Deferred (`#1` open decision 8)** | Dynamic fighter stream-call dispatch (`0xfe` control byte at `0x42c20`); all scenario-reached targets (`0x4141a`, `0x414f0`, `0x41558` in `F29` `#69` / `F58` `#130`; `0x4596e`, `0x45a34` in `F27` `#67`; `0x4558a` in `F62` `#134`; `0x45ace` in `F63` `#135`) are translated, and static translation of unreached move-stream pointers is deferred under `#1` open decision 8. |
| 3 | `fighter.rs:1450 judge` | `note_unrecovered(f)` | unreached (`0`) | **Deferred (`#1` open decision 8)** | Dynamic hit-function dispatch (`+0x28` at `0x43f9e`); all scenario-reached entries across Ryu, Ken, and all 10 CPU opponents (`F27` `#67`, `F28` `#68`, `F44` `#84`, `F53` `#125`, `F57` `#129`, `F59` `#131`, `F60` `#132`, `F62` `#134`, `F63` `#135`) are translated, and the 38 unreached CPU hit-function entries in `0x46472..0x480d0` are deferred under `#1` open decision 8. |
| 4 | `fighter.rs:1913 run_effect` | `note_unrecovered(target)` | unreached (`0`) | **Unreachable under MAME** | Dynamic effect stream-call fallback (`0xff` control byte at `0x42dd2`); all effect stream-call targets in the ROM (`0x427e2`, `0x42920` in `F42` `#82`; `0x432c4`, `0x432e0`, `0x43306`, `0x4331a` in `F43` `#83`) are translated. |
| 5 | `fighter.rs:2360 effect_handler` | `note_unrecovered(h)` | unreached (`0`) | **Deferred (`#1` open decision 8)** | Dynamic effect-handler dispatch (`+0x14` / `+0x18`); all scenario-reached handlers (`0x433ba` in `F9` `#49`, `0x43348` in `F27` `#67`, `0x434e4..0x4360a` in `F52` `#124` / `F61` `#133`, `0x43652` in `F63` `#135`) are translated, and unreached effect-template handler entries are deferred under `#1` open decision 8. |
| 6 | `tasks.rs:712 enter_new_task` | `note(entry)` | **reached** (`5`, `0x483f8`) | **Not covered (`F73`, `#145`)** | Dynamic new-task entry (`0x3b1e8`); all normal-play task entries (`0x3c0c4`, `0x3ccae`, `0x3dec2`, `0x3f7b0`, `0x3f374`, `0x4813e`) are translated (`F15` `#55`, `F34` `#74`, `F48` `#88`), and test-mode slot 0 (`0x483f8`) is translated by `F73` (`#145`). |
| 7 | `tasks.rs:1421 run` | `note(0x3b1d6)` | unreached (`0`) | **Unreachable under MAME** | Scheduler invariant guard when an active RAM slot (`state == 4`) has no corresponding Rust `Task` state. |
| 8 | `tasks.rs:1443 run` | `note(0x3b1f0)` | unreached (`0`) | **Unreachable under MAME** | Scheduler invariant guard when a state-1 (new) RAM slot does not hold `Task::New`. |
| 9 | `tasks.rs:1453 run` | `note(0x3b1d6)` | unreached (`0`) | **Unreachable under MAME** | Scheduler invariant guard when an active RAM slot has no corresponding Rust `Task` state after `enter_new_task`. |
| 10 | `tasks.rs:1474 run` | `note(*entry)` | unreached (`0`) | **Unreachable under MAME** | Rust enum-exhaustiveness guard on `Task::New` inside `Task::run`; `Scheduler::run` always resolves `Task::New` via `enter_new_task` (`0x3b1e8`) before invoking `Task::run`. |
| 11 | `tasks.rs:1529 reset` | `note(0x4834e)` | **reached** (`5`, `0x4834e`) | **Not covered (`F73`, `#145`)** | Reset self-test branch (`0x3b02a -> 0x4834e`) when `DSW1` bit 15 is low (`Service Mode = On`); reached in `sf-dip-smoke-03` and `sf-sound-test-01..04` and translated by `F73` (`#145`). |
| 12 | `tasks.rs:1635 boot` | `note(0x3b602)` | unreached (`0`) | **Unreachable under MAME** | Boot palette-upload repeat (`0x3b602`) when `SYSTEM` bit 6 is low; MAME hardwires `SYSTEM` bit 6 high via `IPT_UNKNOWN` (`sf-mame0289.cpp:561`). |
| 13 | `tasks.rs:2652 flow` | `note(target)` | unreached (`0`) | **Unreachable under MAME** | Country-end per-stage dispatch fallback (`0x3ef96`, table `0x3efb0`); all five ROM table entries — stage 0 (`0x40d30`, `F29` `#69`), stage 1 (`0x41740`, `F52` `#124`), stage 2 (`0x41a02`, `F58` `#130`), stage 3 (`0x41fe6`, `F61` `#133`), and stage 4 (`0x44a40`, `F64` `#136`) — are translated. |
| 14 | `tasks.rs:3792 bonus_japan_meter_frame` | `note(cont)` | unreached (`0`) | **Unreachable under MAME** | Dynamic power-meter continuation fallback (`0x40f1e` / `0x41cf0`); all ten ROM continuation states (`0x40f20..0x40ffe` and `0x41bf2..0x41cd0`) are translated (`F29` `#69`, `F58` `#130`). |
| 15 | `tasks.rs:4784 screen` | `note(handler)` | unreached (`0`) | **Unreachable under MAME** | Dynamic screen-event handler fallback (`0x3f7b0`, table `0x3f7d8`); all ten non-null ROM event handlers (`2..=11`: `0x3f808`, `0x3f8e4`, `0x3f9ec`, `0x3fba2`, `0x3fc8a`, `0x3fcd4`, `0x3fef2`, `0x3ff2c`, `0x3ff5a`, `0x4001c`) are translated (`F2–F5`, `F8`, `F10`, `F16`, `F18`, `F25`, `F43`), and entry 1 (`0x00000`) is never requested. |
| 16 | `tasks.rs:5714 watch` | `note(other)` | unreached (`0`) | **Unreachable under MAME** | Dynamic watch-task continuation fallback (`0xff8f20` / `0xff8f24`); all five attract states (`0x3f3a4..0x3f40e`) and all four game states (`0x3f41e`, `0x3f454`, `0x3f47e`, `0x3f4c6`) are translated (`F34` `#74`, `F41` `#81`). |
| 17 | `tasks.rs:5782 watch` | `note(other)` | unreached (`0`) | **Unreachable under MAME** | Dynamic watch-task continuation fallback (`0xff8f20` / `0xff8f24`); all five attract states (`0x3f3a4..0x3f40e`) and all four game states (`0x3f41e`, `0x3f454`, `0x3f47e`, `0x3f4c6`) are translated (`F34` `#74`, `F41` `#81`). |
| 18 | `tasks.rs:5957 vblank` | `note(0x3b100)` | unreached (`0`) | **Unreachable under MAME** | Guard for bounded callers that call `vblank` directly with Freeze On; `FightMachine` (`CpuMachine`) translates the Freeze spin via `vblank_prefix` / `vblank_suffix`, verified against `sf-freeze-01` (`F21` `#61`). |
| 19 | `tasks.rs:5982 vblank_prefix` | `note(0x3b0f0)` | unreached (`0`) | **Unreachable under MAME** | Vblank skip when `SYSTEM` bit 7 is high (`0x3b0f0` branch taken to `0x3b194`); MAME hardwires `SYSTEM` bit 7 low via `IPT_UNKNOWN` (`sf-mame0289.cpp:562`). |
| 20 | `tasks.rs:5998 vblank_prefix` | `note(0x3b378)` | unreached (`0`) | **Unreachable under MAME** | Vblank palette-upload repeat (`0x3b378`) when `SYSTEM` bit 6 is low; MAME hardwires `SYSTEM` bit 6 high via `IPT_UNKNOWN` (`sf-mame0289.cpp:561`). |
| 21 | `music.rs:640 special` | `note(0x02a5)` | unreached (`0`) | **Unreachable under MAME** | Special command index `>= 5` past the 5-entry table at `0x02a7`; `0x01e2` only routes commands `0x30..0x3f` here, which map to indices `0..4`. |
| 22 | `music.rs:1200 effect_opcode` | `note(0x0e62)` | unreached (`0`) | **Unreachable under MAME** | FM effect stream opcode `> 5` at `0x0e62`; no ROM FM effect stream contains an opcode byte in `0x06..=0x1f`. |
| 23 | `music.rs:1693 div16` | `note(0x0159)` | unreached (`0`) | **Unreachable under MAME** | Divide-by-zero early return (`DE == 0`) at `0x0159`; all ROM tempo divisors passed to `0x0155` are non-zero. |
| 24 | `music.rs:1871 opcode` | `note(0x0000)` | unreached (`0`) | **Unreachable under MAME** | Music stream opcode `0x0a` (null pointer `0x0000` in table `0x0865`); no ROM music stream contains opcode `0x0a`. |
| 25 | `music.rs:1886 opcode` | `note(0x09e7)` | unreached (`0`) | **Unreachable under MAME** | Second-level nested music sub-phrase call (`0x0f` at `0x09e7` when return pointer `ix+0x12..0x13` is already non-zero); ROM tunes only nest one level deep (`F23` `#63`). |

### 3.3 Disposition of every `"Not covered"` item across `docs/reference/*.md`

Every item listed under `## Not covered` in `docs/reference/*.md` falls into one
of four dispositions:
1. **Retired by a later completed ticket in `F1–F71`** (verified end-to-end in
   that ticket's scenario and test).
2. **Unreachable under MAME `0.289`** (with the hardware/ROM/driver reason).
3. **Deferred by an explicit open decision** (`#1` open decision 8 or `#40`
   decisions `2, 3, 6, 7, 10, 11, 12, 15, 16`).
4. **Not covered before test mode, owned by `F73–F77` (`#145–#149`)**.

| Source doc(s) | `"Not covered"` item | Disposition | Detail / ticket / open decision |
| --- | --- | --- | --- |
| `first-frame.md`, `idle-scene.md`, `movement.md`, `crouch.md`, `jump.md`, `posture-dispatch.md`, `punch.md`, `standing-attacks.md`, `crouch-attacks.md`, `jump-attacks.md`, `blocking.md`, `slot-swap.md`, `cpu-decision.md`, `ai-tables.md`, `cpu-follow-up.md`, `round-timer.md`, `round-two.md`, `round-result.md`, `round-transition.md`, `match-win.md`, `hit-sound.md`, `music-command.md`, `music-stops.md`, `milestone-audit.md` | Early slice-boundary exclusions (boot/attract before 2389, country select, pre-fight, round 3, continue screen `0x3eefa`, game over, name entry, bonus stages, travel map, later opponents Geki–Sagat, ending, 1P specials/crouch/jump/block/held, Ken attacks, flip screen, DIP switches, keyboard remapping, person reviews) | **Retired (`F1–F71`)** | All retired by `#41–#90` and `#122–#143`; see Section 2.1 and [full-1p.md](full-1p.md) / [full-2p.md](full-2p.md). |
| `service-sound-test.md` (`F22` / `#62`), `music-tunes.md` (`F23` / `#63`), `sound-test-03.md` / `adpcm-samples.md` (`F38` / `#78`) | Non-idle music-CPU RAM checkpoints (`36` of `75` checkpoints in `sf-sound-test-01`, `41` of `55` checkpoints in `sf-sound-test-02`, and `3` of `133` checkpoints in `sf-sound-test-03` where the frame boundary falls mid-handler inside the Z80's `0x0038` timer interrupt) | **Deferred (`ADR-0001` / `#40` R4)** | `MusicCpu` advances per Timer A tick and compares music RAM (`0xc000..0xc5cf`) at every idle frame end (`39/75` in `F22`, `14/55` in `F23`, `130/133` in `F38`, `23,365/45,260` in `F71` `sf-full-1p-01`, `9,195/14,100` in `F71` `sf-full-2p-01`), while every YM2151 register write and sample is compared across all frames including non-idle frames. Mid-instruction Z80 partial-tick state is outside `ADR-0001`. |
| `round-outcomes.md` (`F32` / `#72`) | One-player time-over draw (`0x3edc2` when P1 and CPU have equal vitality at `TIME = 0`) | **Deferred (`F32` search bound)** | `F32`'s 200-candidate deterministic search over `sf-cpu-01` found no equal-vitality time-over at frame `4270`. The equal-vitality branch `0x3edc2 -> 0x3ee04` is verified in 2P `sf-timeover-01` (`#30`), and the mode-2 continuation `0x3ee04 -> 0x3eefa` (`three_rounds` equal-wins draw -> CPU match win) is verified in `sf-1p-draw-01` and `sf-1p-doubleko-01` (`#72`). |
| `geki.md` (`F27`), `joe.md` (`F28`), `mike.md` (`F44`), `lee.md` (`F53`), `gen.md` (`F57`), `birdie.md` (`F59`), `eagle.md` (`F60`), `adon.md` (`F62`), `sagat.md` (`F63`) | Unexercised CPU hit-function entries (`38` of `67` entries in `0x46472..0x480d0`: `0x46500` in Geki; `7` in Joe; `3` in Mike; `6` in Lee; `6` in Gen; `4` in Birdie; `4` in Eagle; `2` in Adon; `3` in Sagat; plus Retsu's `2` unexercised entries) and Geki's unexercised stream call `0x4594c` / `0x4596e` `Equal` & `Greater` branches | **Deferred (`#1` open decision 8)** | All `29` scenario-reached CPU hit-function entries are translated and verified; the remaining entries share the same `HitFn::Cpu` shape and are deferred under `#1` open decision 8 unless reached by a scenario (`judge` reports any unreached entry via `note_unrecovered`). |
| `one-player-specials.md` (`F13` / `#53`), `hadoken.md`, `specials.md` | Stages `1–5` (`0xffc003 == 1..=5`) of the 1P Hadoken chip table (`0x4340a`), 1P Hadoken attack-box table (`0x434d0`), and 1P Tatsumaki chip table (`0x45fa8`), plus template `0x1871c` (`0x454ae` / `0x434d6`) and Hadoken-then-Tatsumaki fall-through (`0x40798 -> 0x407b4`) | **Deferred (`#1` open decision 8)** | Translated as direct stage-indexed ROM table reads and verified at stage 0 in `sf-1p-hadoken-01` and `sf-1p-tatsumaki-01` (with `C-NEG` proving stage-index sensitivity); winning scripts `sf-win-02..10` use Shoryuken rather than Hadoken/Tatsumaki on stages `1–4`. |
| `ending.md` (`F64` / `#136`) | Ending scene order rows `1..3` in `0x44c1c` (starting countries USA `1`, UK `2`, China `3`) and `0x44b48`'s mode-4 bg scroll `0x44d5e = 0x7880` | **Deferred (`#40` decisions 2, 10, 11)** | Translated as table-driven (`0x44c1c + starting_country * 20`) and mode-branched (`0x44b48`) code; separate winning chains from USA (`#40` decision 2), UK/China (`#40` decision 11), and Ken mode 4 (`#40` decision 10) were explicitly deferred by the user on 2026-10-23. |
| `versus-carry.md` (`F40` / `#80`), `bonus-japan.md` (`F29`), `bonus-stage1.md` (`F52`), `bonus-stage2.md` (`F58`), `bonus-stage3.md` (`F61`) | Ken's one-player game past Retsu (game mode 4 in stages `1–4` and bonus stages `0x40d36`, `0x414f6`, `0x4175e`, `0x418ac`, `0x41962`, `0x41a08`, `0x42004`, `0x42164`, `0x4221a`) | **Deferred (`#40` decision 10)** | Game mode 4 is verified from versus win through Retsu's match end and continue screen (`sf-versus-carry-02`, `#80`); the 9 bonus-stage mode branches are translated from the listing, and a full 10-opponent Ken chain was deferred by `#40` decision 10. |
| `name-entry.md` (`F42` / `#82`, `F50` / `#122`), `versus-carry.md` | P2's name entry (`0x3eef0` / `0x42422` for P2) and both players entering names in one game over | **Deferred (`#40` decision 3)** | Explicitly deferred by the user in `#40` decision 3 (`2026-10-23`). |
| `both-chips.md` (`F8` / `#48`, `F71` / `#143`), `latch-timing.md` | Single constant interrupt latency (`51` in tests, `92` in launcher) vs cycle-exact 68000 task-cycle accounting across the `45–138`-cycle band | **Deferred (`#40` decision 6 / `ADR-0001`)** | At the reference's own latch write times, every sound device and idle music-RAM dump is 100% identical across every fight and both full games (`sf-full-1p-01`, `sf-full-2p-01`). Modelling per-frame 68000 cycle remainder across all tasks is outside `ADR-0001`; `#40` decision 6 retains the constant stand-in with no silent tolerance. |
| `task-scheduler.md`, `latch-timing.md`, `freeze.md`, `attract.md` (`F55` / `#127`) | General frame-overrun model (68000 slowdown when a frame's tasks exceed `133,333` cycles) and non-owned 68000/Z80 stack and saved-register RAM bytes | **Deferred (`#40` decision 12 / `ADR-0001`)** | Explicitly excluded by `#40` decision 12 (`2026-10-23`) and `ADR-0001` (only the deterministic boot setup chunk overrun at frame 6 is encountered and modelled in `power-on.md`; `sf-attract-demo-01`, `sf-full-1p-01`, and `sf-full-2p-01` have `0` fight-loop overruns). |
| `dip-switches.md` (`F17` / `#57`), `continue-accept.md` (`F51` / `#123`) | Unlisted `Game Continuation` and `Buy-In` DIP bit patterns (the 2 of 8 3-bit patterns not listed by MAME's DIP definitions in `sf-mame0289.cpp:521–527, 538–544`) | **Deferred / excluded (`#40` decision 15)** | In the boot decode tables `0x3c16a` and `0x3c188`, the unlisted patterns decode to the exact same bytes as `None` (`0x00`), which `F17` and `F51` verify; excluded by `#40` decision 15. |
| `launcher.md` (`F36` / `#76`) | Gamepad/joystick input and dedicated CLI flag to swap Ken onto P1 keys | **Deferred / excluded (`#1` Out of Scope, `#40` decision 16)** | Gamepad/stick support is Out of Scope in `#1`; a P1-key swap option was excluded by `#40` decision 16 (`--rebind` already allows arbitrary key assignment). |
| `power-on.md` (`F1`), `dip-switches.md` (`F2`), `music-tunes.md` (`F23`), `fm-effects.md` (`F34`) | `SYSTEM` bits 6 and 7 branches (`0x3b0f0`, `0x3b378`, `0x3b602`), unreferenced `compare_bytes_15` (`0x3bb06`), `trap #4` (`0x3b72a`), stubs `0x3b2ca..0x3b2e2`, assertion `0x3b28c..0x3b298`, coin slots 3–4, and ROM-dead music/effect paths (`0x0000` opcode `0x0a`, `0x09e7` 2nd-level nesting, `0x0159` div-by-zero, `0x02a5` special `>=5`, `0x0e62` effect opcode `>5`, slot B, noise tick) | **Unreachable under MAME** | `SYSTEM` bits 6 and 7 are hardwired `IPT_UNKNOWN` in `sf-mame0289.cpp:561–562`; the dead ROM routines/branches have no caller or no matching byte in the verified `sf` ROM data (`coverage-ledger.md` §§4, 8). |
| `power-on.md` (`F1`), `dip-switches.md` (`F2`), `service-sound-test.md` (`F22`), `adpcm-samples.md` (`F38`), `coverage-ledger.md` (`F11`) | Native translation of service / test mode (`0x3b02a`, `0x3b29a..0x3b2b2`, `0x3b5c8..0x3b5f6`, `0x3b664..0x3b672`, `0x3b6b4..0x3b6c4`, `0x3bffc..0x3c032`, `0x4834e..0x492d5`: RAM test, input/DIP/sound test, exit trap `#3`, service menu and items `1–7`) | **Not covered (`F73–F76`, `#145–#148`; audited in `F77`, `#149`)** | Owned by the four test-mode implementation tickets `F73` (`#145`), `F74` (`#146`), `F75` (`#147`), and `F76` (`#148`), followed by the test-mode audit addendum `F77` (`#149`). |

---

## 4. Adopted MAME `0.289` behaviours and their source locations

As required by Acceptance 3 of `#144` (`F72`), Acceptance 1–3 of `#149` (`F77`),
and User Story 33 of `#1`, every place where the port adopts a behaviour of the
pinned MAME `0.289` baseline is listed below with its MAME source location,
repository documentation, and native Rust implementation:

| # | Adopted MAME `0.289` behaviour | MAME `0.289` source location | Repository doc & native implementation |
| --- | --- | --- | --- |
| 1 | **The 500-ns scheduler slice** (`2,000,000 Hz` quantum between the `8 MHz` 68000 and the `3.579545 MHz` Z80s, determined by the highest inter-device sync rate `4,000 Hz * 500` in MAME's round-robin `device_scheduler::timeslice`, through which the samples Z80 observes a 68000 latch write at the next 500-ns slice boundary) | `tools/reference/local/mamesrc/sf-mame0289.cpp:756–775` (`sf_state::sf`, `MC68000(config, m_maincpu, 8_MHz_XTAL)`, `Z80(config, m_audiocpu, 3.579545_MHz_XTAL)`, `Z80(config, m_audio2, 3.579545_MHz_XTAL)`) and MAME `emu/schedule.cpp` | [fight-samples.md](fight-samples.md) (`#32`), `crates/sf-audio/src/samples.rs` (`SLICE: f64 = 500e-9`) |
| 2 | **The NMI at a slice start** (`soundcmd_w` pulses `:audiocpu` `INPUT_LINE_NMI` with `attotime::zero`, which MAME delivers at the start of the current 500-ns scheduler slice, up to 2 Z80 clocks before the 68000 write's exact timestamp; modelled natively by a 1-Z80-clock stand-in and accepting a pending IM 1 timer interrupt immediately after an NMI taken behind `ei`) | `tools/reference/local/mamesrc/sf-mame0289.cpp:95–102` (`sf_state::soundcmd_w`: `m_soundlatch->write(data); m_audiocpu->pulse_input_line(INPUT_LINE_NMI, attotime::zero);`) | [music-power-on.md](music-power-on.md) (`#36`), [fm-effects.md](fm-effects.md) (`#34`), `crates/sf-audio/src/music.rs` |
| 3 | **The interrupt-latency band (`45–138` 68000 cycles)** from vblank assertion to the first instruction fetch of `0x3b052` (arising from MAME's 68000 executing the masked `0x2700` cooperative slot-scan loop `0x3b19a..0x3b1d2` plus the 44-cycle autovectored level-6 exception entry; represented natively by a disclosed constant stand-in inside the band — `51` in tests, `92` in the launcher — while frame 1's post-reset vblank is placed by the reset path's exact `186,275` summed cycles) | `tools/reference/local/mamesrc/sf-mame0289.cpp:756–758` (`m_maincpu->set_vblank_int("screen", FUNC(sf_state::irq1_line_hold))`) and MAME `devices/cpu/m68000/m68kcpu.cpp` | [latch-timing.md](latch-timing.md) (`#35`), [both-chips.md](both-chips.md) (`#37`, `#48` / `F8`, `#143` / `F71`), `crates/sf-engine/src/tasks.rs` (`RESET_TO_VBLANK1_CYCLES`, `vblank`), `crates/sf-dev/src/main.rs` (`LATENCY_CYCLES = 92`) |
| 4 | **The output resampler** (MAME resamples the YM2151 `55,930.39 Hz` and MSM5205 `4,000 Hz` / `8,000 Hz` streams through its internal band-limited output resampler to `48,000 Hz` `audio.wav` with a slight high-frequency rolloff; the native mixer uses direct linear interpolation from the chip sample rates to `48,000 Hz`, and acceptance compares the pre-resampler chip streams sample-for-sample plus acoustic/human listening checks on the `48 kHz` mix) | `tools/reference/local/mamesrc/sf-mame0289.cpp:777–796` (`YM2151` `0.60`, `MSM5205` `0.50` / `0.50` routing into `SPEAKER`) and MAME `emu/sound.cpp` | [both-chips.md](both-chips.md) (`#37`, `#49` / `F9`, `#70` / `F30`, `#126` / `F54`), `crates/sf-audio/src/mix.rs` |
| 5 | **The coin lockout** (`coin_w` at `0xc00010` low byte routes bits `0..1` to `coin_counter_w` and inverted bits `4..5` (`~data & 0x10`, `~data & 0x20`) to `machine().bookkeeping().coin_lockout_w(0/1, ...)`; when `coin_lockout_w` is active — i.e. `coin_ctrl` bit 4 or 5 is `0`, as before `0x3c26a` at frame 794 or at 9 credits after `0x4830c` — MAME suppresses `PORT_IMPULSE(2)` coin edges on `IN0` bits `0..1` at the ioport level so the 68000 still reads `IN0` bit 0/1 high (`1`)) | `tools/reference/local/mamesrc/sf-mame0289.cpp:110–116` (`sf_state::coin_w`) and `:442–443` (`PORT_IMPULSE(2)` on `COIN1` / `COIN2`) | [coins.md](coins.md) (`#52` / `F12`), [start-screen.md](start-screen.md) (`#73` / `F33`), `crates/sf-engine/src/tasks.rs` (`coin_ctrl_or`, `coin_ctrl_off`, `vblank_prefix`) |
| 6 | **The power-on RAM contents (`F1`)** (MAME initializes work RAM `0xff8000..0xffdfff` to `0x00`, object RAM `0xffe000..0xffffff` to `0x00` with `m_objectram[0] = 0x00c0` written by `sf_v.cpp` / driver init before frame 0's end, video RAM `0x800000..0x800fff` to tile `0x0020` (`0x20, 0x00` per word: 2,048 big-endian `0x0020` words), and palette RAM `0xb00000..0xb007ff` to `0x0000`) | `tools/reference/local/mamesrc/sf-mame0289.cpp:166–184` (`sfus_map` RAM shares `workram`, `objectram`, `videoram`, `paletteram`), `sf_v.cpp`, and `sf-boot-01` frame-0 dumps | [power-on.md](power-on.md) (`#41` / `F1`), `crates/sf-engine/src/tasks.rs` (`PowerOnMachine::new`) |
| 7 | **The fixed `IPT_UNKNOWN` `SYSTEM` bits** (MAME defines `:SYSTEM` at `0xc00002` with active-low bits `2, 3, 4, 6` and active-high bit `7` as `IPT_UNKNOWN`, fixing the idle readback to `0x007f` and making the `SYSTEM` bit 7 vblank-skip branch at `0x3b0f0` and the `SYSTEM` bit 6 palette-repeat branches at `0x3b378` and `0x3b602` unreachable under MAME) | `tools/reference/local/mamesrc/sf-mame0289.cpp:558–570` (`INPUT_PORTS_START( sf )` `:SYSTEM` port definition) | [power-on.md](power-on.md) (`#41` / `F1`), [dip-switches.md](dip-switches.md) (`#42` / `F2`), `crates/sf-engine/src/tasks.rs` (`boot`, `vblank_prefix`, `service_input`) |
| 8 | **The vblank held (`irq1_line_hold`) across the reset's RAM clear** (MAME configures the 68000 vblank interrupt with `irq1_line_hold` (`HOLD_LINE` on autovector level 6 in `sf_state`), so frame 0's end-of-frame vblank assertion remains latched while reset's `6,144`-longword `clr.l (A0)+; dbra` loop `0x3b012..0x3b01a` runs for `184,398` cycles with `SR = 0x2700` across frame 0's `133,333 1/3`-cycle boundary into frame 1, and is taken immediately when `0x3b04a: move #$2000, SR` unmasks interrupts at `186,275` cycles; the same `irq1_line_hold` latches vblank across the Freeze DIP spin at `0x3b100..0x3b108` and across the `15`-frame `ram_test_start` work/object + video RAM test loop `0x4834e..0x483f2` taking `2,171,478` cycles across frames `1..=15` and completing on frame `16` at `SERVICE_RAM_TEST_TO_VBLANK_CYCLES = 38,720`) | `tools/reference/local/mamesrc/sf-mame0289.cpp:758` (`m_maincpu->set_vblank_int("screen", FUNC(sf_state::irq1_line_hold))`) | [power-on.md](power-on.md) (`#41` / `F1`, `#55` / `F15`), [freeze.md](freeze.md) (`#61` / `F21`), [service-mode.md](service-mode.md) (`#145` / `F73`), `crates/sf-engine/src/tasks.rs` (`reset`, `RESET_TO_VBLANK1_CYCLES`, `SERVICE_RAM_TEST_END_CYCLES`, `SERVICE_RAM_TEST_TO_VBLANK_CYCLES`, `vblank_prefix`, `vblank_suffix`) |
| 9 | **The unmapped-port read value (`unmap_value_high` = `0xffff` on `0xc00004`, `0xc00006`, `0xc0000e`)** (MAME declares `map.unmap_value_high()` on `sfus_map`, maps `0xc00000` (`IN0`), `0xc00002` (`IN1`), `0xc00008` (`DSW1`), `0xc0000a` (`DSW2`), and `0xc0000c` (`SYSTEM`), and maps `0xc00004`, `0xc00006`, and `0xc0000e` as `nopr()`; when `0x484d4..0x48502` shifts 16 bits out of each of the 8 words `0xc00000..0xc0000e` into the `8x17` binary cell matrix at `0xff8732..0xff8841` — inverting every word via `not.w D0` except `D2 == 5` for `0xc00004` and `D2 == 4` for `0xc00006` — MAME's `0xffff` readback produces `11111111 11111111` on rows 2–3 (`0xff8776..0xff87ba`) and `00000000 00000000` on row 7 (`0xff8820..0xff8841`)) | `tools/reference/local/mamesrc/sf-mame0289.cpp:167, 173–174, 178` (`sfus_map`: `map.unmap_value_high();`) | [service-mode.md](service-mode.md) §§2–3.1 (`#145` / `F73`), `crates/sf-engine/src/tasks.rs` (`Io::port_word`, `unmap_c00004 = 0xffff`, `unmap_c00006 = 0xffff`, `unmap_c0000e = 0xffff`) |

---

## 5. Citations and results of the three person checks (`F30`, `F31`, `F54`)

All three `ready-for-human` verification tickets in the arcade-flow batch have
been executed by the project owner, recorded in the repository, and closed:

1. **Ticket #70 (`F30`):
   "Listening review of the fixed fight (person)"** —
   [both-chips.md](both-chips.md) §9 (`L436–465`) and
   [milestone-audit.md](milestone-audit.md) (`L292–311`).
   - **Artefacts reviewed:** `round2-mame-reference.wav`,
     `round2-native-mix-51.wav`, `round2-native-mix-92.wav`,
     `round2-launcher-live-92.wav`, and the full-fight `fight-*.wav` set in
     `tools/reference/local/renders/` (`cargo run -p sf-dev --release -- kit`).
   - **Verdict:** **Pass** across all five listening checks (round-2 native mix
     at `51` vs MAME reference, round-2 native mix at `92` vs `51` and MAME
     reference, round-2 live launcher capture vs batch render, full-fight native
     mix at `51` vs MAME reference, and full-fight native mix at `92` vs `51`
     and MAME reference). The reviewer noted only a slight high-frequency rolloff
     ("softer character, almost as if there's a low-pass filter") on MAME's
     resampled `audio.wav` relative to the direct native mix, with all timings,
     pitches, and events matching (`0` follow-up tickets).
2. **Ticket #71 (`F31`):
   "Human keyboard sessions verified against the reference (person)"** —
   [launcher.md](launcher.md) (`L174–219`).
   - **Artefacts recorded and verified:**
     - `tools/reference/scenarios/human-cpu-01.tsv` (`1,105` frames from the
       one-player fixture at frame `2389`, `2389..=3494`, exercising all 4
       directions and all 6 attack buttons against the live CPU Retsu, with a
       focus-loss pause/release cycle): converted to
       `tools/reference/scenarios/sf-human-cpu-01.json`, verified against two
       byte-identical MAME `0.289` runs (`human-cpu-01-run-a ==
       human-cpu-01-run-b`) and native replay (`Outcome::Match` over all `1,105`
       frames and `1,105` checkpoints, owned RAM byte-identical, `unrecovered`
       empty, mean frame time `6.985 ms`, max `24.852 ms`).
     - `tools/reference/scenarios/human-versus-01.tsv` (`1,141` frames from the
       two-player versus fixture at frame `2450`, `2450..=3591`, exercising both
       P1 and P2 keyboard bindings through a Ryu KO win over Ken at frame `3231`
       and into round 2): converted to
       `tools/reference/scenarios/sf-human-versus-01.json`, verified against two
       byte-identical MAME `0.289` runs (`human-versus-01-run-a ==
       human-versus-01-run-b`) and native replay (`Outcome::Match` over all
       `1,141` frames and `1,141` checkpoints, owned RAM byte-identical,
       `unrecovered` empty, mean frame time `7.026 ms`, max `14.544 ms`).
   - **Verdict:** **Pass** (`0` follow-up tickets; a minor CLI flag interaction
     when passing `--record` without `--bindings` in `crates/sf-dev/src/main.rs`
     was noted and resolved).
3. **Ticket #126 (`F54`):
   "Listening review of every tune, effect and sample (person)"** —
   [both-chips.md](both-chips.md) §11 (`L540–637`).
   - **Artefacts reviewed:** `sound-test-01-native-mix.wav` vs
     `sound-test-01-mame-reference.wav`, `sound-test-02-native-mix.wav` vs
     `sound-test-02-mame-reference.wav`, `sound-test-03-native-mix.wav` vs
     `sound-test-03-mame-reference.wav`, `sound-test-04-native-mix.wav` vs
     `sound-test-04-mame-reference.wav`, and `sound-test-measures.tsv` (`236`
     per-command windows across all `21` tunes `0x28..0x3e`, all `28` distinct
     FM effects `0x01..0x27`, and all `56` ADPCM samples across banks `0–6`
     `0x40..0xbe`, generated by `cargo run -p sf-dev --release --
     kit-sound-test`).
   - **Verdict:** **Pass** across every tune (`0x28..0x3e`, including `0x28`
     full round-tune loop), every FM effect group (`0x01..0x27`), and every
     ADPCM sample bank `0–6` (`0x40..0xbe`), with `0` audible defects and `0`
     follow-up tickets.

---

## 6. Explicit non-certification statement

This audit (`F72`, issue `#144`) evaluates the port at the completion of the
pre-test-mode arcade-flow batch (`F1–F71`). **It does not certify the complete
port:**

- **Test mode (`F73–F76`, issues `#145–#148`) and its audit addendum (`F77`,
  issue `#149`) come after this audit.** Specifically, the 68000 self-test
  branch (`0x3b02a -> 0x4834e`), the RAM test (`0x4834e..0x483f2`), the
  input/DIP/sound-test task (`0x483f8..0x48642`), task-exit `trap #3`
  (`0x3b29a..0x3b2b2`), the service menu (`0x48644..0x487f6`), the seven
  service screen/sprite/scroll/output tests (`0x487f8..0x492d5`), and helpers
  `0x3b5c8`, `0x3b664`, `0x3b6b4`, `0x3bffc / 0x3c014` are not yet translated at
  this milestone.
- **The deferred items listed in Section 3.2 and Section 3.3 are not certified:**
  - the 38 unreached CPU hit-function table entries in `0x46472..0x480d0`,
    unreached fighter/effect stream pointers, and stages `1–5` of the 1P
    Hadoken/Tatsumaki chip tables (`#1` open decision 8);
  - the unfound 1P equal-vitality time-over draw against the live CPU (`F32` /
    `#72`);
  - the USA-start (`#40` decision 2) and 4-country UK/China (`#40` decision 11)
    winning chains and their ending scene orders;
  - Ken's one-player game past Retsu in game mode 4 (`#40` decision 10);
  - P2's name entry and two-name game over (`#40` decision 3);
  - cycle-exact 68000 interrupt-latency modelling across the `45–138`-cycle band
    (`#40` decision 6 / `ADR-0001`), a general 68000 frame-overrun model (`#40`
    decision 12 / `ADR-0001`), and mid-handler non-idle Z80 RAM checkpoints
    (`ADR-0001`);
  - the 2 unlisted `Game Continuation` and `Buy-In` DIP bit patterns (`#40`
    decision 15) and a dedicated CLI flag to swap Ken onto P1 keys (`#40`
    decision 16).
- **Nothing that does not pass is certified.**

---

## 7. Addendum: Test-mode result and final batch closure (`F77` / `#149`)

This addendum (`F77`, issue
#149) records the completion
of the four test-mode tickets (`F73–F76`, issues `#145–#148`), the coverage
ledger rerun across all `137` scenarios (`274` MAME `0.289` probe runs), the
addition of the unmapped-port read value (`F73` / `#145`) to the adopted MAME
`0.289` behaviours (Row 9 of Section 4), the outcome of every open decision and
deferred item across `#1`, `#13`, and `#40`, and the final pass / deferred / fail
certification boundary for the complete `78`-ticket arcade-flow batch (`#40–#90`,
`#122–#149`).

### 7.1 Results of `F73–F76` (`#145–#148`)

All four test-mode tickets in [service-mode.md](service-mode.md) are implemented
in `crates/sf-engine/src/tasks.rs` and `crates/sf-engine/src/run.rs` and
verified in `crates/sf-engine/tests/service_mode.rs`:

| Ticket | Scope & ROM address ranges | Scenario(s) (`run-a == run-b`) | Verification summary (`cargo test --workspace --release`) |
| --- | --- | --- | --- |
| #145 (`F73`: "Service RAM test and input/DIP/sound test") | `0x3b02a` (`reset` branch to `0x4834e` when `DSW1` bit 15 is low), `0x4834e..0x483f6` (`ram_test_start` / `ram_test_finish` 2-region `0x5555` / `0xaaaa` test across work+object RAM `0xff8000..0xffffff` and video RAM `0x800000..0x800fff`, clearing `0xff8000..0xffffff` and filling `0x800000..0x800fff` with `0x103c` over `SERVICE_RAM_TEST_END_CYCLES = 2,171,478` cycles / `15` held frames `1..=15`, completing on frame `16` at `SERVICE_RAM_TEST_TO_VBLANK_CYCLES = 38,720`, setting `0xff8003 = 0x55`, and spawning slot 0 at `0x483f8`), `0x3b5c8..0x3b5f6` (`service_screen_init`), `0x3b6b4..0x3b6c4` (`aux_ctrl_off` on `0xff8686` & `0xc00012`), `0x3bffc..0x3c032` (`write_hex_word` / `write_hex_byte`), `0x483f8..0x48642` (`service_input` / `service_input_loop`: `SWITCH TEST` `8x17` binary cell matrix `0xff8732..0xff8841` from `0xc00000..0xc0000e`, `MEMORY TEST` `OK` at `0x8006be` / `0x8005be`, `PUSH SW.` strengths `0xff8862..0xff8865`, `SOUND TEST` selector `D7` stepped every 5th frame on P1 Up/Down and queued as `D7 & 0xbf` on any P1 punch release `0xff86b0 > 4`, exit on P1 Up held + any P1 kick release `0xff86b1 > 4` spawning slot 1 at `0x48644`), `0x3b29a..0x3b2b2` (`apply_yield` `trap #3` task exit) | `sf-service-01` (`42` frames `0..=41`, `42` full checkpoints, `213` files), plus `sf-dip-smoke-03` (`0..=40`) and `sf-sound-test-01..04` | `sf_service_01_reference_runs_are_identical`, `service_mode_from_power_on_matches_reference_and_exits_to_0x48644`, and `negative_control_wrong_unmapped_port_value_caught_at_frame_20` pass: `Outcome::Match` across all `42` frames and `42` checkpoints of `sf-service-01` (`0..=41`, owned work RAM, video RAM, palette RAM, object RAM, RGBA framebuffers, and sound latch writes at frames `24` -> `0x00`, `29` -> `0x01`, `34` -> `0x02`, `38` -> `0x01`), exiting into `0x48644` at frame `41` with `unrecovered` empty; `C-NEG` proves zeroing `Io::unmap_c00004` (`0xffff -> 0x0000`) fails at frame `20` on row 2 (`0xff8776..0xff8798`) and video RAM. |
| #146 (`F74`: "Service menu and screen tests") | `0x48644..0x487f6` (`service_menu` in slot 1, cursor `D7` over items `0..=6` with 6-frame blink and `0xff884a` edge latch, any P1 punch release `0xff86b0 > 4` spawning the selected item into slot 2 via `trap #0`, P1 Up held + any P1 kick release `0xff86b1 > 4` killing slot 2 via `trap #1` `0x3b230` and redrawing `0x48678`), `0x4929c..0x492d5` (`erase_service_menu`), `0x487f8..0x48856` (item 1 Crosshatch `service_crosshatch_init` & `service_item`), `0x488f8..0x4898e` (item 2 Color Bar `service_color_bar_init`, palette `0x1be68 -> 0x0600`), `0x48858..0x488f6` (item 3 Character Test 1 `service_char_test1_init`), `0x48990..0x48b68` (item 4 Character ROM Test `service_char_rom_init`: P1 Up/Down steps 2-bit page `0xff884c` at `0x8003a2` & `0x800624`, P1 Left/Right steps 4-bit colour `0xff884e` at `0x8003ba` & `0x800624`) | `sf-service-02` (`2,449` frames `0..=2448`, `2,449` full checkpoints, `12,248` files) | `sf_service_02_reference_runs_are_identical`, `service_menu_and_first_four_items_match_reference_and_report_items_5_to_7`, and `negative_control_wrong_char_rom_page_caught_in_item_4` pass: `Outcome::Match` across all `2,449` frames and `2,449` checkpoints of `sf-service-02` (owned work RAM, video RAM, palette RAM, object RAM, and RGBA framebuffers across the menu and items `1–7` with `unrecovered` empty); `C-NEG` proves omitting the frame-`1812` `p1_up` input in item 4 fails at frame `1813` on `0xff884c`, `0x8003a2`, and `0x800624`. |
| #147 (`F75`: "Service sprite and output tests") | `0x48b6a..0x48da2` (item 5 Sprite / Object ROM Test `service_sprite_test_init`: populates `32` (`4x8`) `16x32` sprites at `0xffe080`, P1 Up/Down steps `0xff8851` by `0x20` (`0x00..=0x80`) wrapping into page `0xff8850` (`0..=0x3f`) at `0x8003a2` & `0x800618`, P1 Left/Right steps 4-bit colour `0xff884e` at `0x8003ba` and `+2` across all 32 sprites), `0x490f8..0x4929a` (item 7 Output Port Cycle `service_output_test_init`: automatic 25-step output cycle `0x491ee..0x49283` alternating 30-frame baseline resets `0x49196..0x491ce` and 30-frame active steps `0x491d0..0x491ec`, wrapping `0x96 -> 0` and reading no inputs), `0x3b664..0x3b672` (`aux_ctrl_or` on `0xff8686` / `0xc00012`), `0x3b934..0x3b96a` (`clear_words_8` `0x3b93c`) | `sf-service-03a` (`2,235` frames `0..=2234`, `2,235` full checkpoints, `11,178` files) | `sf_service_03a_reference_runs_are_identical`, `service_sprite_test_and_output_cycle_match_reference`, and `negative_control_wrong_sprite_page_caught_in_item_5` pass: `Outcome::Match` across all `2,235` frames and `2,235` checkpoints of `sf-service-03a` (owned work RAM, video RAM, palette RAM, object RAM, and RGBA framebuffers across item 5 and the full 25-step item 7 output cycle); `C-NEG` proves omitting the 5th `p1_up` input at frame `142` in item 5 fails at frame `143` on `0xff8850..0xff8852`, `0xffe080`, and `0x8003a2`. |
| #148 (`F76`: "Service scroll test") | `0x48da4..0x490f6` (item 6 Scroll Test `service_scroll_test_init_fg`: FG init `0x48da4..0x48e68` with control block `0xff8852..0xff885a` & copy `0xffc680`, BG init `0x48e6a..0x48e8a` with control block `0xff885a..0xff8862` & copy `0xffc682`, per-frame P1 FG `0x48e96..0x48f3f` and P2 BG `0x48f40..0x48ff3` scroll steps using `0x48ff4..0x4906b` direction/auto-repeat decode — `Up +64/1`, `Down -64/1`, `Left +32/12`, `Right -32/12` — and 6-stage palette threshold lookups `0x48f10..0x48f3f` / `0x48fba..0x48fe9`) | `sf-service-03b` (`911` frames `0..=910`, `911` full checkpoints, `4,558` files) | `sf_service_03b_reference_runs_are_identical`, `service_scroll_test_matches_reference`, and `negative_control_wrong_fg_scroll_step_caught_in_item_6` pass: `Outcome::Match` across all `911` frames and `911` checkpoints of `sf-service-03b` (owned work RAM including `0xff8852..0xff8862` and `0xffc680..0xffc684`, video RAM, palette RAM, object RAM, and RGBA framebuffers across P1 FG and P2 BG scroll sweeps and palette thresholds); `C-NEG` proves omitting the frame-`74` `p1_down` input in item 6 fails at frame `75` on `0xff8852..0xff8856` and at frame `76` on `0xffc680..0xffc682` and `0x80069c..0x8006b0`. |

### 7.2 Coverage ledger rerun over `0x4834e..0x492d5` and its helpers (Acceptance 1)

Rerunning `python tools/reference/coverage_ledger.py` across all `137` scenarios (`274`
MAME `0.289` probe runs, `run-a == run-b` on all `137`) updates
[coverage-ledger.md](coverage-ledger.md) and
`tools/reference/local/coverage/ledger.json`:

| CPU | Routine intervals | Executed instruction PCs | Translated | Untranslated (`documented_gaps`) | Unnamed gaps |
| --- | --- | --- | --- | --- | --- |
| `:maincpu` (M68000) | `114` (`113` executed, `1` unexecuted) | `9,299` | `9,299` (`100.0%`) | `0` | `0` |
| `:audiocpu` (music Z80) | `42` (`42` executed, `0` unexecuted) | `1,394` | `1,394` (`100.0%`) | `0` | `0` |
| `:audio2` (samples Z80) | `6` (`6` executed, `0` unexecuted) | `223` | `223` (`100.0%`) | `0` | `0` |
| **Total** | **`162` (`161` executed, `1` unexecuted)** | **`10,916`** | **`10,916` (`100.0%`)** | **`0`** | **`0`** |

Every executed instruction PC in the test-mode range `0x4834e..0x492d5` (`858`
executed PCs across all `11` intervals) and every one of its `8` helper
intervals (`67` executed PCs) is translated:

| Start | End | Routine | Native module | Exec PCs | Trans PCs | Untrans PCs | Scen # | First scenario@frame |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `0x3b000` | `0x3b026` | Reset vector, work RAM clear & `DSW1` test-mode check (`0x3b000`) | `tasks.rs:1726 reset` | `10` | `10` | `0` | `137` | `sf-1p-block-01@0` |
| `0x3b02a` | `0x3b02a` | Reset test-mode jump (`jmp 0x4834e.l`) (`0x3b02a`) | `tasks.rs:1778 ram_test_start` | `1` | `1` | `0` | `9` | `sf-dip-smoke-03@1` |
| `0x3b29a` | `0x3b2b2` | Task exit (`trap #3`: clear slot & yield) (`0x3b29a`) | `tasks.rs:1572 apply_yield` | `6` | `6` | `0` | `5` | `sf-service-02@22` |
| `0x3b5c8` | `0x3b5f6` | Service screen init (`0x3b5c8`: clear planes, upload default palette & clear sprites) | `tasks.rs:1975 service_screen_init` | `10` | `10` | `0` | `9` | `sf-dip-smoke-03@16` |
| `0x3b664` | `0x3b672` | Auxiliary control OR helper (`0x3b664`) | `tasks.rs:1815 aux_ctrl_or` | `5` | `5` | `0` | `1` | `sf-service-03a@1205` |
| `0x3b6b4` | `0x3b6c4` | Auxiliary control clear helper (`0x3b6b4`: `0xff8686` & `0xc00012`) | `tasks.rs:1822 aux_ctrl_off` | `6` | `6` | `0` | `9` | `sf-dip-smoke-03@16` |
| `0x3b934` | `0x3b96a` | Block clear helpers (`clear_words_4` `0x3b934`, `clear_words_8` `0x3b93c`, `clear_words_64` `0x3b95c`) | `fighter.rs:1729 clear_words_4` | `7` | `7` | `0` | `31` | `sf-service-03b@70` |
| `0x3bffc` | `0x3c032` | Service hex-word / hex-byte tile writer (`0x3bffc` / `0x3c014`) | `tasks.rs:2010 write_hex_word` | `22` | `22` | `0` | `9` | `sf-dip-smoke-03@20` |
| `0x4834e` | `0x483f6` | Service RAM/VRAM/palette/object self-test & task-0 init (`0x4834e`) | `tasks.rs:1778 ram_test_start` | `48` | `48` | `0` | `9` | `sf-dip-smoke-03@1` |
| `0x483f8` | `0x48642` | Service input / DIP switch / sound test task (`0x483f8`) | `tasks.rs:1845 service_input` | `143` | `143` | `0` | `9` | `sf-dip-smoke-03@16` |
| `0x48644` | `0x487f6` | Service menu task (`0x48644`: items `1–7` selector & dispatch) | `tasks.rs:2034 service_menu` | `80` | `80` | `0` | `5` | `sf-service-02@22` |
| `0x487f8` | `0x48856` | Service item 1: cross-hatch test (`0x487f8`) | `tasks.rs:2164 service_item`, `tasks.rs:2282 service_crosshatch_init` | `26` | `26` | `0` | `1` | `sf-service-02@82` |
| `0x48858` | `0x488f6` | Service item 3: character test 1 (`0x48858`) | `tasks.rs:2305 service_char_test1_init` | `36` | `36` | `0` | `1` | `sf-service-02@1234` |
| `0x488f8` | `0x4898e` | Service item 2: colour bar test (`0x488f8`) | `tasks.rs:2344 service_color_bar_init` | `35` | `35` | `0` | `1` | `sf-service-02@658` |
| `0x48990` | `0x48b68` | Service item 4: character ROM test (`0x48990`) | `tasks.rs:2376 service_char_rom_init` | `110` | `110` | `0` | `1` | `sf-service-02@1810` |
| `0x48b6a` | `0x48da2` | Service item 5: object (sprite) ROM test (`0x48b6a`) | `tasks.rs:2487 service_sprite_test_init` | `135` | `135` | `0` | `2` | `sf-service-03a@130` |
| `0x48da4` | `0x490f6` | Service item 6: scroll test (`0x48da4`) | `tasks.rs:2626 service_scroll_test_init_fg` | `180` | `180` | `0` | `3` | `sf-service-03b@70` |
| `0x490f8` | `0x4929a` | Service item 7: output port test (`0x490f8`) | `tasks.rs:2804 service_output_test_init` | `59` | `59` | `0` | `2` | `sf-service-03a@693` |
| `0x4929c` | `0x492d5` | Service menu tile-erase helper (`0x4929c`) | `tasks.rs:2156 erase_service_menu` | `6` | `6` | `0` | `3` | `sf-service-03b@70` |

Updated disposition of the report sites from Section 3.2:
- **Zero (`0`) of the `25` `note()` / `note_unrecovered()` report sites are
  reached across all `137` scenarios.**
- In particular, the two sites that were previously reached in `Service Mode =
  On` scenarios before `F73` (`#145`) are now resolved:
  - `tasks.rs:873 enter_new_task` (`note(entry)`): `unreached (0)` — **Unreachable
    under MAME** (`enter_new_task` now translates all `14` ROM task entries across
    slots `0, 1, 2, 3, 7`: the `6` normal-play entries `0x3c0c4`, `0x3ccae`,
    `0x3dec2`, `0x3f7b0`, `0x3f374`, `0x4813e`, plus `0x483f8` in slot 0,
    `0x48644` in slot 1, and the `7` service item entries `0x487f8`, `0x488f8`,
    `0x48858`, `0x48990`, `0x48b6a`, `0x48da4`, `0x490f8` in slot 2).
  - `reset`'s `0x3b02a -> 0x4834e` branch no longer calls `note(0x4834e)`; it
    translates `ram_test_start` (`0x4834e`) directly, while `service_screen_init`
    (`tasks.rs:1985`) carries the **Unreachable under MAME** guard `note(0x3b602)`
    for the `SYSTEM` bit 6 palette-upload repeat inside `0x3b5c8` (`SYSTEM` bit 6
    is hardwired high via `IPT_UNKNOWN` in `sf-mame0289.cpp:561`).
- The only unexecuted routine interval in the entire 68000 ROM (`1` of `114`) is
  `0x3bb06..0x3bb0c` (`compare_bytes_15`), which has no caller in the original
  ROMs and is **Unreachable under MAME**.

### 7.3 Unmapped-port value (`F73` / `#145`) added to adopted MAME `0.289` behaviours

As required by `#149` (`F77`), the unmapped I/O port read value discovered and
documented in `F73` (`#145`, [service-mode.md](service-mode.md) §§2–3.1) has been
added as **Row 9 of Section 4 (`## 4. Adopted MAME 0.289 behaviours and their
source locations`)**:
- In `tools/reference/local/mamesrc/sf-mame0289.cpp:167, 173–174, 178`,
  `sf_state::sfus_map` configures `map.unmap_value_high()` (`0xffff`), maps
  `0xc00000` (`IN0`), `0xc00002` (`IN1`), `0xc00008` (`DSW1`), `0xc0000a`
  (`DSW2`), and `0xc0000c` (`SYSTEM`), and maps `0xc00004`, `0xc00006`, and
  `0xc0000e` as `nopr()` in the `0xc00000..0xc0000f` input range.
- When the service input/switch loop at `0x484d4..0x48502` shifts 16 bits out of
  each of the eight words `0xc00000..0xc0000e` into the `8x17` binary cell matrix
  at `0xff8732..0xff8841`, it inverts every word via `not.w D0` except `D2 == 5`
  (`0xc00004`) and `D2 == 4` (`0xc00006`). MAME's `0xffff` readback on the three
  unmapped word addresses therefore produces `11111111 11111111` on rows 2–3
  (`0xff8776..0xff87ba`) and `00000000 00000000` on row 7 (`0xff8820..0xff8841`).
- `Io::port_word` in `crates/sf-engine/src/tasks.rs` models this explicitly via
  `unmap_c00004 = 0xffff`, `unmap_c00006 = 0xffff`, and `unmap_c0000e = 0xffff`.

### 7.4 Every open decision and every deferred item with its outcome (Acceptance 2)

#### 7.4.1 Outcome of every open decision in `#1` (Specification) (`1..8`)

| # (`#1`) | Open decision | Outcome & ticket(s) |
| --- | --- | --- |
| 1 | **Which MAME version to pin as the reference** (record exact version string, build flags, and platform in the first reference-harness ticket) | **Resolved (`#2`, `F1`–`F77`):** Pinned to **MAME `0.289`** (`tools/reference/local/mame/mame.exe`, `sf-mame0289.cpp`) across all `137` scenarios (`274` deterministic runs). |
| 2 | **Which parent ROM set to target (`sf`, `sfj`, `sfu`, `sfan`, `sfp`)** | **Resolved (`#2`, `#3`):** Targeted the parent **`sf` (World, button version)** ROM set (`sfd-03..sfd-22`, `sf-01..sf-42`, `sf-00`) with SHA-1 manifest in `tools/reference/rom-manifest.json`, `crates/sf-video/src/romset.rs`, and [mame-baseline.md](mame-baseline.md). |
| 3 | **Button version vs pneumatic-pad version** | **Resolved (`#2`, `#18`, `#76`):** Targeted the **6-button (`sf`) version** (`P1_BUTTON1..6` / `P2_BUTTON1..6` on `IN0` and `IN1`), not the pneumatic-pad (`sfan` / `sfp`) analog ports. |
| 4 | **Exact frame number and scenario for the first end-to-end milestone** (Ryu vs Retsu, Round 1, both standing idle) | **Resolved (`#12`, `#14`):** Fixed at **frame `2389`** of `sf-idle-01` (`Ryu` vs `Retsu`, Round 1, Japan, both standing idle). |
| 5 | **Format of the per-frame reference dump** (`JSON` + binary blobs vs single binary bundle) | **Resolved (`#2`):** `manifest.json` + per-checkpoint `workram.bin`, `videoram.bin`, `paletteram.bin`, `objectram.bin`, `audiocpu_ram.bin`, `audio2_ram.bin`, `ym2151_regs.bin`, `frame.png` + run-level `ym2151_writes.tsv`, `ym2151_samples.s16le`, `msm5205_1_samples.s16le`, `msm5205_2_samples.s16le`, `audio.wav`, and `executed_pcs.json`. |
| 6 | **Whether to vendor a known-good YM2151 core (`ymfm` / Nuked-OPM) or port from MAME's YM2151 implementation** | **Resolved (`#36`, `#37`, `F8`, `F22`, `F23`, `F54`, `F71`):** Implemented a native Rust YM2151 (`crates/sf-audio/src/ym2151.rs`) and dual MSM5205 (`crates/sf-audio/src/msm5205.rs`) directly matching MAME `0.289`'s chip cores sample-for-sample (`0` sample diffs across every scenario including `sf-full-1p-01` and `sf-full-2p-01`). |
| 7 | **Default keyboard layout for 1P and 2P** | **Resolved (`#76` / `F36`):** Default 1P (`W/S/A/D`, `U/I/O/J/K/L`, `5` Coin 1, `1` Start 1) and 2P (`Up/Down/Left/Right`, `Numpad 4/5/6/1/2/3`, `6` Coin 2, `2` Start 2) bindings with `--bindings` TOML and interactive `--rebind` ([launcher.md](launcher.md)). |
| 8 | **How to verify ROM table entries not reached by the test scenarios** | **Resolved by hybrid policy (`#39`, `#51` / `F11`, `#144` / `F72`, `#149` / `F77`):** Scenario-first verification (`10,916 / 10,916` executed PCs translated across `137` scenarios, `113 / 114` 68000 intervals executed), plus static table translation of all ROM data tables (`fighters.rs`, `stages.rs`, `tables.rs`, `scroll.rs`), with explicit `note_unrecovered` reporting at the 3 dynamic table-pointer dispatch sites (`fighter.rs:840 animate`, `fighter.rs:1450 judge`, `fighter.rs:2360 effect_handler`). Unreached CPU hit-function entries (`38` of `67`), unreached move/effect stream pointers, and stages `1–5` of the 1P Hadoken/Tatsumaki chip tables remain explicitly deferred and uncertified under this decision. |

#### 7.4.2 Outcome of every open decision in `#13` / `docs/planning/next-batch.md` (`1..4`)

| # (`#13`) | Open decision (`docs/planning/next-batch.md` §§3, 5) | Outcome & ticket(s) |
| --- | --- | --- |
| 1 | **When the 68000 writes the sound latch within a frame (`§3.1`)** | **Resolved (`#35`, `#37`, `#48` / `F8`, `#143` / `F71`, `2026-10-22`):** Investigated in [latch-timing.md](latch-timing.md) (`#35`), proving every 68000 sound-latch write lands inside the `45–138`-cycle vblank interrupt-latency band; resolved on `2026-10-22` by adopting the disclosed constant latency stand-in (`51` in tests, `92` in the launcher) and requiring exact sample-for-sample device identity at reference write times plus human listening reviews (`F30` `#70`, `F54` `#126`). |
| 2 | **Clean late-game references (`§3.2`)** | **Resolved (`#31` / `N18`, `#46` / `F6`, `#68` / `F28`, `#84` / `F44`, `#125` / `F53`, `#129` / `F57`, `#131` / `F59`, `#132` / `F60`, `#134` / `F62`, `#135` / `F63`, `#137` / `F65`):** Every stage win (`sf-win-01..10`) and the full one-player campaign (`sf-full-1p-01`) were produced by deterministic winning input searches without RAM patching (`tools/reference/search_win.py`). |
| 3 | **Random state from power-on (`§3.3`)** | **Resolved (`#41` / `F1`, `#55` / `F15`, `#89` / `F49`, `#137` / `F65`, `#138` / `F66`):** `0x3b18a` (`0xff802c` RNG mixing in `vblank_suffix`) and slot 7's `0x4813e` background pass task are translated from power-on frame 0 so `0xff802c` matches the MAME reference byte-for-byte across all power-on scenarios. |
| 4 | **Hidden code entry points (`0xfe` / `0xff` stream calls, `§3.4`) and test/service mode (`§5`, `2026-10-22`)** | **Resolved (`#51` / `F11`, `#144` / `F72`, `#145–#149` / `F73–F77`):** Every slice's MAME probe records executed instruction PCs into the `F11` (`#51`) coverage ledger (`10,916 / 10,916` executed PCs translated across all `137` scenarios with `0` reached `note()` / `note_unrecovered()` sites), and test/service mode (`0x4834e..0x492d5`) was kept in scope and completed in `F73–F77` (`#145–#149`). |

#### 7.4.3 Outcome of every open decision in `#40` / `docs/planning/arcade-flow-batch.md` §7 (`1..16`)

All `16` open decisions in [arcade-flow-batch.md](../planning/arcade-flow-batch.md)
§7 (`L2314–2413`) were answered by the project owner on `2026-10-23` and
executed across `F1–F78`:

| # (`#40`) | Decision question & user resolution (`2026-10-23`) | Outcome & ticket(s) |
| --- | --- | --- |
| 1 | **Publish all 77 tickets at once, or in waves:** Publish waves `0–2` first (`F1–F49`, `#41–#89`), with `F78` (`#90`) re-checking and publishing `F50–F77` (`#122–#149`) after `F15` (`#55`) and `F49` (`#89`). | **Completed (`#41–#90`, `#122–#149`):** Waves `0–2` and waves `3–4` published and closed across all `78` tickets. |
| 2 | **The USA-start ending chain (country 2 at `0x44a76`):** Deferred (`2026-10-23`), listed as not covered in `F64` (`#136`) and `F72` (`#144`). | **Japan chain completed (`F29` `#69`, `F52` `#124`, `F58` `#130`, `F61` `#133`, `F64` `#136`, `F65` `#137`, `F71` `#143`); USA-start chain deferred (`#40` decision 2).** |
| 3 | **P2's name entry and a two-name game over:** Deferred (`2026-10-23`), listed as not covered in `F50` (`#122`), `F66` (`#138`), and `F72` (`#144`). | **P1 name entry completed (`F42` `#82`, `F50` `#122`, `F65` `#137`, `F71` `#143`); P2 name entry and two-name game over deferred (`#40` decision 3).** |
| 4 | **Fetching `mame0289`'s `tilemap.cpp` for `F18`:** Approved (`2026-10-23`) to fetch into `tools/reference/local/`. | **Completed (`#58` / `F18` [flip-screen.md](flip-screen.md), `#139` / `F67` `sf-flip-02`).** |
| 5 | **The fallback if `F6`'s search fails at a later opponent:** Widen the search first (`2026-10-23`); use continues only if that fails. | **Completed without continues (`#46` / `F6`, `#68` / `F28`, `#84` / `F44`, `#125` / `F53`, `#129` / `F57`, `#131` / `F59`, `#132` / `F60`, `#134` / `F62`, `#135` / `F63`):** All 9 searches (`sf-win-02..10`) succeeded cleanly without continues. |
| 6 | **The launcher latency after `F8`:** Decide after `F8` (`#48`)'s results (`2026-10-23`). | **Completed (`#48` / `F8`, `#70` / `F30`, `#126` / `F54`, `#143` / `F71`):** Retained `92` (`LATENCY_CYCLES = 92`) with no tolerance; cycle-exact 68000 task-cycle remainder across the `45–138`-cycle band remains deferred (`#40` decision 6 / `ADR-0001`). |
| 7 | **The disk budget for long-session checkpoints (`F65`, `F66`):** Keep full checkpoints only in each ticket's named windows and `frames.tsv` fields between them (`2026-10-23`). | **Completed (`#137` / `F65` `sf-full-1p-01` with `9,105` checkpoints across 15 windows of `45,261` frames, `#138` / `F66` `sf-full-2p-01` with `5,021` checkpoints across 6 windows of `14,101` frames, and `#139–#142` / `F67–F70`).** |
| 8 | **The DIP mechanism for `F2`:** Left to `F2` (`#42`) from the pinned MAME's behaviour (`2026-10-23`). | **Completed (`#42` / `F2` [dip-switches.md](dip-switches.md)):** Implemented via `dipswitches` / `dip_changes` ioport field overrides in `sf-compare` and the MAME driver generator. |
| 9 | **Where `F36`'s bindings file lives:** `%APPDATA%\sf1-port\bindings.toml` (`2026-10-23`). | **Completed (`#76` / `F36` [launcher.md](launcher.md)).** |
| 10 | **Ken's one-player game past Retsu (game mode 4):** Deferred (`2026-10-23`); verify versus win -> Retsu (`F40`) and label mode-4 bonus/ending branches unexercised. | **Mode 4 into Retsu completed (`#80` / `F40`); 10-opponent Ken chain past Retsu deferred (`#40` decision 10).** |
| 11 | **The ending's four-country starts (countries 1 and 3):** Deferred (`2026-10-23`); verify `Countries = 4` cursor and UK/China starts (`F47`, `F67`) and list full 10-opponent chains from UK and China as deferred. | **`Countries = 4` select & UK/China starts completed (`#87` / `F47`, `#139` / `F67` `sf-select4-03`); full 10-opponent chains from UK/China deferred (`#40` decision 11).** |
| 12 | **A general frame-overrun model:** No general 68000 frame-overrun model; report each overrun met as a scope question (`2026-10-23`). | **Completed (`#127` / `F55`, `#137` / `F65`, `#138` / `F66`: `0` fight-loop overruns across `sf-attract-demo-01`, `sf-full-1p-01`, and `sf-full-2p-01`; general overrun model excluded by `#40` decision 12 / `ADR-0001`).** |
| 13 | **The 30-instruction bound:** Approved (`2026-10-23`) so size-0 verification tickets reaching an untranslated branch under ~30 instructions may translate it directly. | **Completed (applied in `#72` / `F32`, `#79` / `F39`, `#80` / `F40`, `#127` / `F55`, `#138` / `F66`, `#139` / `F67`).** |
| 14 | **How `F18` is published before decision 4:** Moot (`2026-10-23`) because decision 4 was resolved before publishing wave 1. | **Completed (`#58` / `F18` published normally in wave 1).** |
| 15 | **The `Game Continuation` and `Buy-In` values MAME does not list:** Exclude the 2 of 8 3-bit patterns not listed by MAME (`sf-mame0289.cpp:521–527, 538–544`, `2026-10-23`). | **Excluded (`#40` decision 15; `0x3c156..0x3c18f` decode both unlisted patterns to `0x00`, identical to `None` verified in `F17` `#57`, `F51` `#123`, and `F70` `#142`).** |
| 16 | **A host option to put Ken on the P1 keys:** Exclude a dedicated CLI swap flag (`--rebind` already supports any key mapping, `2026-10-23`). | **Excluded (`#40` decision 16).** |

#### 7.4.4 Complete list of deferred / excluded items and their outcomes

With `F73–F76` (`#145–#148`) complete, the test-mode item from Section 3.3 is
**retired (`F73–F76`)**. The complete, final list of items that remain deferred
or excluded at the close of the `78`-ticket batch is:

1. **Unreached dynamic ROM table entries (`#1` open decision 8):**
   - `38` of `67` CPU hit-function entries in `0x46472..0x480d0` (Section 3.3;
     all `29` scenario-reached CPU hit-function entries are translated and
     verified, and `fighter.rs:1450 judge` reports any unreached entry via
     `note_unrecovered`);
   - unreached fighter/effect stream-call pointers at `fighter.rs:840 animate`
     and `fighter.rs:2360 effect_handler` (`0` unreached pointers exist in any
     executed scenario);
   - stages `1–5` (`0xffc003 == 1..=5`) of the 1P Hadoken chip table
     (`0x4340a`), 1P Hadoken attack-box table (`0x434d0`), and 1P Tatsumaki chip
     table (`0x45fa8`), plus template `0x1871c` (`0x454ae` / `0x434d6`) and
     Hadoken-then-Tatsumaki fall-through (`0x40798 -> 0x407b4`), which are
     translated from the ROM tables and verified at stage 0 (`F13` / `#53`).
2. **One-player equal-vitality time-over draw against the live CPU (`F32` /
   `#72`):** Not found within `F32`'s 200-candidate deterministic search over
   `sf-cpu-01`; the equal-vitality branch `0x3edc2 -> 0x3ee04` is verified in 2P
   `sf-timeover-01` (`#30`) and the mode-2 equal-wins draw continuation
   `0x3ee04 -> 0x3eefa` is verified in `sf-1p-draw-01` and `sf-1p-doubleko-01`
   (`#72`).
3. **Multi-country and mode-4 full winning chains (`#40` decisions 2, 10, 11):**
   - Full 10-opponent winning chains starting from USA (`#40` decision 2), UK, or
     China (`#40` decision 11) and their ending scene order rows `1..3` in
     `0x44c1c`;
   - Ken's one-player game past Retsu in game mode 4 (`#40` decision 10),
     including the mode-4 branches in stages `1–4` bonus stages and ending
     `0x44b48` (translated from the listing, verified through Retsu in `F40` /
     `#80`).
4. **P2 high-score name entry and two-name game over (`#40` decision 3):**
   Explicitly deferred in `#40` decision 3 (`P1` name entry and table insertion
   are verified in `F42` `#82`, `F50` `#122`, and `F71` `#143`).
5. **Cycle-level CPU emulation exclusions (`ADR-0001`, `#40` decisions 6 & 12):**
   - Per-frame 68000 task-cycle remainder accounting across the `45–138`-cycle
     vblank interrupt-latency band (`#40` decision 6; constant stand-in `51` /
     `92` retained, with 100% sound/chip match at reference latch times);
   - A general 68000 frame-overrun model beyond the deterministic boot frame-6
     chunk overrun (`#40` decision 12; `0` fight-loop overruns occur);
   - Non-idle mid-handler Z80 music-RAM checkpoints where the frame boundary
     splits the Z80's `0x0038` timer interrupt (`ADR-0001`; all idle checkpoints
     and all YM2151 writes/samples match across every frame);
   - Non-owned 68000 / Z80 stack and scratch-register RAM bytes (`ADR-0001`).
6. **Excluded DIP / CLI conveniences (`#1` Out of Scope, `#40` decisions 15 &
   16):**
   - The 2 unlisted `Game Continuation` and `Buy-In` 3-bit DIP patterns (`#40`
     decision 15);
   - A dedicated CLI flag to swap Ken onto P1 keys (`#40` decision 16) and
     gamepad/joystick input (`#1` Out of Scope).

### 7.5 Final pass / deferred / fail account and certification boundary (Acceptance 3)

Updating Section 2.1 and Section 2.2 after `F73–F76` (`#145–#148`):

- **User Story 16 (`#1`: Original startup and title flow from power-on,
  including the `0x3b02a -> 0x4834e` service/test-mode branch) and User Story 34
  (`#1`: Verified facts distinguished from incomplete recovery) now include the
  completed service / test mode (`Pass`):** Every DIP switch (`DSW1` and `DSW2`,
  including `Service Mode` `DSW1` bit 15) and the entire service / test mode
  (`0x3b02a`, `0x4834e..0x492d5`, and helpers `0x3b29a`, `0x3b5c8`, `0x3b664`,
  `0x3b6b4`, `0x3b93c`, `0x3bffc / 0x3c014`) are translated into native Rust and
  verified against MAME `0.289` in `sf-service-01`, `sf-service-02`,
  `sf-service-03a`, `sf-service-03b`, `sf-dip-smoke-01..03`,
  `sf-dip-boot-01..08`, and `sf-sound-test-01..04`.
- **Overall requirement tally across `#1`:**
  - **Pass (`28` of `34` user stories + all `13` testing decisions):**
    - User Stories `US-1`, `US-2`, `US-3`, `US-4`, `US-5`, `US-6`, `US-10`,
      `US-11`, `US-12`, `US-13`, `US-14`, **`US-16`**, `US-17`, `US-18`, `US-21`,
      `US-22`, `US-23`, `US-24`, `US-25`, `US-26`, `US-27`, `US-28`, `US-29`,
      `US-30`, `US-31`, `US-32`, `US-33`, and `US-34` (with `US-5` and `US-6`'s
      listing-translated unreached recoil/stage-1..5 table entries noted in
      Section 2.1 and Section 7.4.4).
    - All `13` testing decisions in Section 2.2 (`13 / 13 Pass`).
  - **Pass on all ticketed scenarios with explicitly deferred items uncertified
    (`6` of `34` user stories in Section 2.1):**
    - `US-7` (Hit detection, damage, blocking, reactions: all normal/special/Tiger
      Shot hits, blocks, and projectile slow motion pass; `38` of `67` unreached
      CPU hit-function table entries in `0x46472..0x480d0` deferred under `#1`
      open decision 8).
    - `US-8` (Opponent decisions and timing across all 10 CPU opponents: all 10
      CPU opponents pass; unreached CPU move-stream and hit-function branches
      deferred under `#1` open decision 8).
    - `US-9` (Health, timer, round results, and transitions: 9 of 10
      round-outcome matrix cells + 1P equal-wins draw continuation pass; 1P
      equal-vitality time-over draw against live CPU deferred under `F32` search
      bound).
    - `US-15` (Two-player modes with keyboard controls: fresh 2P start,
      challenger join, P1/P2 carry-over, and versus game over pass; Ken's 1P
      game past Retsu in mode 4 deferred under `#40` decision 10 and P2 name
      entry / two-name game over deferred under `#40` decision 3).
    - `US-19` (Full original progression: Japan 10-opponent chain through Sagat,
      all 4 bonus stages, and travel maps pass; full 10-opponent chains from
      USA/UK/China and Ken mode 4 past Retsu deferred under `#40` decisions 2,
      10, 11).
    - `US-20` (Continuation and ending behavior: 1P & 2P continue countdown,
      accept, Buy-In, expiry, game over, Japan ending, staff roll, and P1 name
      entry pass; ending scene order rows `1..3`, mode-4 ending scroll `0x44d5e`,
      and P2 name entry deferred under `#40` decisions 2, 3, 10, 11).
  - **Fail (`0`):** Zero requirements fail.

#### Explicit certification boundary (`F77` / `#149`)

- **What is certified:** All `137` deterministic scenarios (`274` MAME `0.289`
  runs), all `10,916` executed instruction PCs across `:maincpu` (`9,299`),
  `:audiocpu` (`1,394`), and `:audio2` (`223`) (`100.0%` translated, `0`
  untranslated, `0` reached `note()` / `note_unrecovered()` sites), the three
  human verification reviews (`F30` `#70`, `F31` `#71`, `F54` `#126`), the full
  one-player campaign (`sf-full-1p-01`, `45,260` frames), the full two-player
  versus session (`sf-full-2p-01`, `14,100` frames), and the complete service /
  test mode (`sf-service-01`, `sf-service-02`, `sf-service-03a`,
  `sf-service-03b`).
- **What is NOT certified:** The deferred and excluded items enumerated in
  Section 7.4.4 (`#1` open decision 8 unreached table entries, `F32` 1P
  equal-vitality time-over draw against live CPU, `#40` decisions `2, 3, 6, 10,
  11, 12, 15, 16`, and `ADR-0001` sub-tick / stack exclusions) are **not**
  certified. **Nothing that does not pass is certified.**
