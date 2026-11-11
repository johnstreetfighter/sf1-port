# Session handoff for 2026-11-11 — SF1+ Dramatic Battle and NP9 shipped

- **Release.** The annotated `v1.3.0` tag points to `95089c2e1be52ff50aa297c81c2fe8ed6a195de6` on `main`. The published Windows ZIP includes `sf-game.exe`, shaders, notices, and setup instructions. The downloaded archive matched the local build and its SHA-256 `894284044eb441d544c20ccd22853aaf2483477b1cfc6b2dff2e09bda9b41eac`. Both executables report `v1.3.0 (95089c2e)`. [GitHub Release](https://github.com/geox123/sf1-port/releases/tag/v1.3.0)
- **Scope.** Native SF1+ Dramatic Battle supports solo play with a CPU ally, local co-op, and NP9 online co-op with live spectating. Six rivalries and Custom 2v1 support shared or independent lifebars and optional friendly fire. The native three-fighter engine, setup, NP9, and netplay parent issues are closed. [#286](https://github.com/geox123/sf1-port/issues/286), [#287](https://github.com/geox123/sf1-port/issues/287), [#352](https://github.com/geox123/sf1-port/issues/352), [#303](https://github.com/geox123/sf1-port/issues/303)
- **Verification.** The locked release workspace run passed 1,654 tests across 250 suites, with two intentional ignores and zero missing-fixture skips. Both lifebar modes matched every confirmed frame in a 300-frame Ryu and Ken versus Sagat scenario at 50 ms RTT and 10% simulated loss. Forced replays crossed damage to both teams. A late spectator matched the host's confirmed checksum. Physical two-computer play and the live public directory remain unverified.
- **Open work.** Additional native era styles and the Team Era Gauge are tracked under #402, blocked by the native era work in #269. Controller QA #347 is held at George's request. [#402](https://github.com/geox123/sf1-port/issues/402), [#269](https://github.com/geox123/sf1-port/issues/269), [#347](https://github.com/geox123/sf1-port/issues/347)
- **Storage.** The Windows package is in `D:/sf1-port-storage/releases/v1.3.0`. The netplay worktree's `target-netplay/debug` now links to `D:/sf1-port-storage/build/np1-foundations-debug` after all 3,300 files were verified by SHA-256. Keep using `--target-dir C:/Users/George/.codex/worktrees/np1-foundations/sf1-port/target-netplay` in this worktree. C: had 80.67 GiB free at closeout.

# Session handoff: 2026-11-10 (NP1-NP8 shipped in v1.2.1)

- **Release:** [v1.2.1](https://github.com/geox123/sf1-port/releases/tag/v1.2.1) is published. Its annotated tag points to the exact tested source `4b0c1f67`. Main merged it at `9aae2f3`, preserving the three local combo-planning documentation commits. The Windows ZIP includes `sf-game.exe`, shaders, setup instructions, and notices. The local and GitHub ZIP SHA-256 matched (`d94a9b5ba042eac6bf3de56d5e37f57c4c028feed0e4069970195a9c5edad124`). The game and developer tool report `v1.2.1 (4b0c1f67)`; v1.0.0 through v1.1.2 also have verified Windows packages.
- **Scope:** NP1–NP8 shipped and #304–#311 are closed. Baseline includes native 1v1 rollback, Online lobby/settings, rematch/reselect, confirmed live spectating, Windows UDP/LAN, and the optional HTTP(S) directory. The legacy Sim/Peer prototype was retired. [#303](https://github.com/geox123/sf1-port/issues/303) stays open for NP9/#352, gated by local Dramatic #287. Controller QA #347 remains held. See [netplay.md](netplay.md) for controls and directory setup.
- **Verification:** the final locked release workspace run completed all 247 suites on `4b0c1f67` in 6,583.47 seconds: 1,631 passes, zero failures, two intentional ignores (a generated report and GPU timing), and zero fixture skips. The recovered diagnostic run at `da23fdba` had one failure: a preloaded bank could change the Faithful 1987 Arcade intro. The preset guard preserved Arcade's overhaul screens and restored exact Faithful replay across 137 scenarios and 874,344 frames; all 18 focused background-bank and VS tests passed. Coverage regeneration and check verified 137 scenarios, zero untranslated PCs and the 93-PC negative control. [NP8/#311](https://github.com/geox123/sf1-port/issues/311) records Standards and Spec review outcomes.
- **Verification boundaries:** native online session tests play a complete two-round match and exercise rematch, reselect, and spectator playback. KO torture fixtures seed health through public snapshots, then run native attacks/results. Host tests used actual Windows UDP loopback, broadcast, and adapter queries. Physical two-machine networking, live HTTPS, and Linux execution remain unverified.
- **Fixture recovery:** all 44 ROM files, pinned art `cf96ea83`, voice `99794aae`, and installed music were recovered. Pinned MAME 0.289 restored and independently audited 266 raw and 36 audio fixtures. The regenerated full-game audio streams passed the native latch and sample checks. The earlier run that lost its fixtures is not release evidence.
- **Build isolation:** use `--target-dir C:/Users/George/.codex/worktrees/np1-foundations/sf1-port/target-netplay` for builds/tests in the netplay worktree. The shared `target` path points to the original checkout cache and can mix build outputs.
- **Local storage:** the original Cargo target, netplay release and historical caches, Windows packages, and selected recovery/diagnostic data now use `D:/sf1-port-storage` through the existing Windows paths. Both checkouts reach all 302 restored captures on D:; the original checkout links directly to `D:/sf1-port-storage/reference/captures`. The native versus smoke suite passed all three tests from each checkout with no fixture skips. Package checksums, the extracted version banner, and all 44 ROMs were verified through the existing paths.

# Session handoff for 2026-11-10 — SF1+ combo discovery planned and ticketed

- **Approved design.** The [combo discovery sweep](design/2026-11-10-combo-discovery-design.md) tries normals into selected signature moves using the native SF1+ moveset in Definitive Overhaul. It starts at close range against standing and crouching Ken, reports verified timing windows relative to the opener’s hit, and saves a playable replay per successful pair and stance. The glossary defines the combo timing window. Local design commits are `36f9c7a` and `b467bb4`.
- **Tracker.** [Spec #398](https://github.com/geox123/sf1-port/issues/398) has three verified `ready-for-agent` sub-issues. [#399](https://github.com/geox123/sf1-port/issues/399) is the first implementation ticket, covering Ryu’s crouching light kick into Hadoken and replay playback. [#400](https://github.com/geox123/sf1-port/issues/400) expands fighter and stance coverage. [#401](https://github.com/geox123/sf1-port/issues/401) adds authored command inputs. GitHub shows #399 blocking both follow-ups.
- **Closeout.** This session completed the design and issue filing. Implementation starts with #399. The repository changes are documentation, and verification covered GitHub issue contents, labels, links and Git whitespace checks.

# Session handoff: 2026-11-10 (move-list sizing and Arcade VS fixes pushed as v1.1.2; session closed)

- **Shipped:** `c76656f` is pushed to `main` and fixes #394 and #395, both confirmed closed. The workspace version and all ten lockfile entries are `1.1.2`. This session created no release tag.
- **Move list (#394):** MOVES now scales its panel and lettering at readable whole-number sizes while preserving a strip of the paused fight. The minimum window is 384x224 so the native panel fits. GPU sampling now uses integer coordinates: the former float division selected neighboring source pixels at 3x and 6x and damaged the lettering.
- **Arcade VS (#395):** the built VS screen uses the selected player and resolved current ladder opponent, including Ryu stand-ins. Regression checks cover Lee/Ryu, Ken/Retsu, Lee/Retsu and Ken/Geki after winning the preceding match, plus restored stage pages/palettes and Faithful 1987 isolation with a preloaded bank.
- **Verification:** 172 targeted tests passed: 79 host, 15 move-list/input, 28 game and 50 Arcade/VS/roster/stage-select tests. One manual GPU timing test was ignored. Actual Training MOVES output was rendered through the production GPU layer at 384x224, 768x672 (Integer 4:3), 896x672, 1920x1080 and 3200x2000: all 3,870,720 panel pixels matched, contents fit, and the paused picture stayed visible. The permanent regression covers exact 3x/6x GPU pixel replication. This was a targeted run, not a full workspace test sweep.
- **Smoke and visual limits:** the final quiet release smoke check passed fullscreen/windowed transitions, all four CRT looks, pacing, settings and ROM lookup. The first smoke run failed three CRT pacing thresholds while other tests ran; both outcomes are recorded here. Native window capture timed out twice, so visual review used the actual-session offscreen GPU images. George has not yet playtested this batch.
- **Local artifacts:** `target/release/sf-game.exe` reports `v1.1.2 (fc8bef90)`; its source stamp predates the fix commit because the tested build was made before committing. The five PNGs and temporary QA source are retained under ignored `target/issue394-qa/`; the temporary example was removed from the tracked source tree.
- **Closeout:** George authorized the push and then requested session closeout. The independent netplay rollout chat/worktree remains separate; its earlier handoff below is preserved. No further work on #394 or #395 is pending in this chat.

# Session handoff: 2026-11-10 (sprite/throw playtest fixes released as v1.1.1; session closed)

- **Shipped:** `0ceb897` fixes held jumps, banked pose origins, crouching-hit placement, air-attack landings and lying/get-up grounding. `bf7b3ed` fixes visible post-grab facing for Ryu, Ken and Lee through explicit authored-pose metadata, preserving logical facing and victim travel. Both are on `main`. Release commit `68a839b` increments all ten workspace crates and their lockfile entries to `1.1.1`; annotated tag `v1.1.1` points to that commit and is pushed. George playtested the result and confirmed everything looks good.
- **Placement verification:** the full twelve-fighter rerun covered both orientations, 1,838 clips / 273,324 frames, all 372 targeted checks passing, and complete video decoding. Sampled visual review covered 140 sheets / 2,133 pose-event snapshots plus native frames. All twelve review receipts have no unresolved placement findings. The 23 missed scripted contacts remain explicit coverage limits; this is SF1+ verification, not every-era or every-frame clearance. The initial fix batch passed 268 selected tests.
- **Final facing verification:** 96 fresh forward/back HP/HK throw clips across all twelve fighters passed all 576 checks and decoded completely. All 14 throw tests passed, including Ryu/Ken/Lee's visible direction, Lee's ordinary sweep, all-roster centre cases, 40 representative corner cases, techs, wakeup, KO and snapshot replay. The final selected regression run passed 192 tests; it was not a full workspace test sweep.
- **Release verification:** all three version unit tests and the Settings version-header test passed. Default-feature release builds of `sf-game` and `sf-dev` succeeded. Both `--version` and `-V` report `v1.1.1 (68a839b3)` for both executables. The usual `C:/Users/George/Projects/sf1-port/target/release/sf-game.exe` is updated; the earlier temporary throw-facing executable is no longer the recommended launcher.
- **Tracker:** #396 (fighter pose placement) and #397 (held-jump timing) are closed with completed acceptance checklists and verification records. #394 (move-list sizing) and #395 (Arcade VS fighter identities) remain open and need their own fixes. No other project tickets were claimed completed by this session.
- **Standing rule:** George requires every bug-fix or patch delivery to increment the project's version (PATCH for SemVer), include metadata/lockfile updates, and verify the reported version before completion or push. Saved globally in `C:/Users/George/.codex/AGENTS.md`.
- **Local evidence:** retained in the `codex/pose-placement-throws` worktree under `tools/reference/local/pose-audit-fixed-2026-11-10` and `tools/reference/local/throw-facing-2026-11-10`; the original audit remains under the original checkout's `tools/reference/local/pose-audit-2026-11-10`. These ignored captures are local-only. Retain the managed worktree while those evidence links are needed. The separate netplay rollout chat/worktree remains independent; its earlier handoff follows below.

# Session handoff: 2026-11-10 (NP2 rollback engine verified; NP3-NP8 rollout continuing)

- **Authorization:** George requested the remaining NP tickets until human input is needed. NP2-NP8 are in scope; NP9 remains gated on the later local Dramatic Battle implementation. Work continues in the isolated `codex/netplay-rollout` branch/worktree. The music chat's approved Ryu theme landed in `4308f2c` and is preserved in the NP2 merge; #347 remains deferred at his request.
- **NP2 implementation:** `0496294` adds the socket-free `RollbackEngine`, borrowing the existing `Fight` per step/correction, with nine pre-step snapshots, frame-zero rollback, 0-4F delayed inputs, last-input prediction, confirmed input/checksum/event histories, rift pacing, window stalls and the 300-tick timeout. It preserves per-frame audio occurrence history outside snapshots and merges newly corrected delivery into the next successful `step` output. Session integration must continue consuming that output after remote inputs arrive; it must not transition on speculative match-end events.
- **Verification:** all seven `np2_rollback` integration tests pass. They compare independent native snapshots, public reads, hashes and events at every confirmation across 300-frame runs with each delay from 1 to 8 frames and both local sides. Further witnesses cover 0-4F bootstrap, pacing/stall/timeout boundaries, malformed/stale/future inputs, forfeit during bootstrap and after source-frame confirmation, delayed checksums after ring reuse, legitimate duplicate announcements and repeated A/B/A corrections, actual combos/cancels/throws/last-window tech, and corrected sound/First Attack delivery. Standards review: zero violations/unresolved smells. Spec review: zero findings.
- **KO verification boundary:** the KO/result tests use a controlled final-round fixture: after faithful fight entry, a public snapshot seeds health and round wins. Attacks, guarding, KO and the later result flow then execute in the native engine. This is not a claim to have played an entire match solely through pad inputs. The early lethal-contact state and the later `Fight::is_over`/MatchOver boundary are distinct; a six-tick-late correction is tested at each applicable boundary without changing native timing.
- **Rollout in progress:** protocol/bus/rules tests, online settings/lobby and online pause/after-match components have targeted green checks, with actual GameSession connection/fight integration, spectating, host UDP/discovery/directory, the torture suite and the full-workspace milestone checks still ahead. These later tickets are not claimed complete. NP8 remains the minor-version milestone; the current version is `1.1.0`.

# Session handoff: 2026-11-10 (NP1 foundations implemented; NP2 next)

- **Authorization and parallel work:** George explicitly continued into NP1 #304. #267 remains with its dedicated music child chat. #347 remains deferred at his request until the physical controller arrives; neither is an NP1 blocker. Work was isolated on `codex/np1-foundations` so the music chat could keep the original checkout.
- **Implementation:** `bc060c6` adds complete pre-step `FightSnapshot` save/restore, canonical 64-bit FNV-1a gameplay identity, ten-bit input masks, IPv4 room codes and the two glossary terms. Every mutable wrapper field is captured except current local overlay/gauge-art/notation choices, which survive restore. Machine/scheduler/config/toggle/training/fair-CPU state uses explicit tagged bytes; diagnostics, presentation and emitted-audio delivery are documented exclusions. Active Training Show input state is included. See the design's dated NP1 implementation boundary for the audit.
- **Behavioral verification:** the final combined release run of engine unit tests plus `np1_codecs`, `np1_snapshots`, `match_config_and_input_toggles`, `throws`, `coverage_ledger` and `training_refill` passed all **179 tests** across seven suites (90.13 seconds). The eight snapshot tests run actual local ROMs/art/tables without skipping: hidden drive, midfight, local preferences/output queues, Training Show, combos, cancels, throws/last-window tech, round transition and final KO through MatchOver plus eight frames. They compare full state, pixels, latch timing, announcements/events and hashes; hidden scheduler/hitstun changes are independently observed before simulation diverges. The eight codec tests cover all 1024 gameplay combinations and all 65,536 raw masks, literal bit positions, default/custom/raw endpoints and malformed inputs. Video unit tests also passed (2), for **181 passing tests** across these selected targets.
- **Independent review:** Standards found no rule violations and one low-priority duplicate encoder sequence; the shared borrowed `PowerOnGameplay` view resolved it. Follow-up Standards and Spec reviews both have zero unresolved findings. The view preserves exhaustive field audits without cloning the live machine. The workspace compile also exposed redundant trait imports and one stale, unread test-only refill field; both were removed without changing assertions or production behavior.
- **Compile and ledger:** `rtk cargo check --release -j 2 --workspace --all-targets` passes with no warnings after cleanup. The regenerated coverage ledger and `--check` verify all 137 reference pairs, zero untranslated PCs and the 93-PC negative control. Diff whitespace checks pass.
- **Verification boundary:** full workspace tests were not repeated for NP1. The earlier complete baseline sweep and repairs are recorded below; NP1 uses the targeted regressions above and an all-target workspace compile. No new online UI, sockets, rollback prediction/ring or audio de-duplication is claimed here. NP2 #305 owns the next implementation; NP3 owns immutable-bank/protocol compatibility and NP8 owns retiring the legacy prototype and the baseline minor-version bump. The release version stays `1.1.0`.

# Session handoff: 2026-11-10 (pre-netplay audit; test/fixture gaps repaired; music and hardware QA remain)

- **George's request:** check for leftover work before starting NP1 #304. He identified #267 as needing brainstorming and explicitly requested a child chat; **Brainstorm #267: Ryu's theme** was created and started. NP1 implementation has not started.
- **SF1+ status correction:** the implementation is landed and parent #246 is closed, but #267 is still open after George's music-quality complaint. Only 21 of the 22 SP tickets are closed. #267 now says it needs the dedicated design/listening discussion and is `needs-triage`, rather than `ready-for-agent`. README reflects this remaining quality follow-up. Physical Haute42 QA #347 remains open and `ready-for-human`. Optional announcer work #348-#351 is a future feature batch.
- **Complete baseline sweep:** `rtk cargo test --release -j 2 --workspace --no-fail-fast -- --show-output` completed all 227 harnesses in 3,384.6 seconds: 1,465 passed, 2 failed, 2 ignored, exit 101. The two failures were stale tests described below; four reported successes had skipped their assertions because reference fixtures were missing. Both failures and all four skipped assertions were subsequently rechecked after repair. This records the full-run failure and the targeted reruns separately; it is not a claim that the original full command exited successfully.
- **Round-result fixture repaired:** `front_end_layout` tried to select Lee/Mike without their approved art/player tables; SP13 correctly left Ryu/Ken selected. It now loads the approved tables/art, checks the selected identities, and renders expected pixels with the extended sprite bank. `rtk cargo test --release -j 2 -p sf-engine --test front_end_layout -- --nocapture`: all 8 passed, including both player names, CPU fallback and faithful rendering. No production result-routing change was needed.
- **Training refill witness repaired:** the old test required the modern CPU to allow an eventual refill exactly 60 frames after a block, but its later attacks prevented that scenario. It now directly observes a free, wounded player still waiting after the previous hit's deadline but before the zero-damage block's new deadline, with no intervening contact. The original per-frame 60-frame eligibility assertions remain intact. `rtk cargo test --release -j 2 -p sf-engine --test training_refill -- --nocapture`: all 7 passed across every preset and the existing exact-refill/replay/CPU/dummy cases. The repair is test-only.
- **Reference captures repaired locally:** restored 13 absent capture directories (held/crouch/jump/Hadoken A/B, blocking B, draw A/B and double-KO A/B), and repaired incomplete `1p-round3-01-run-a`. Regenerated data matches pinned MAME 0.289; eight A/B pairs contain 83,349 identical relevant artifact pairs. Round3 A's 9,841 surviving artifacts were preserved; 16,222 missing artifacts and 5,212 previews were copied only after verification against complete B and historical hashes. Local ROM/capture data remains outside source control.
- **Previously skipped tests now execute:** the repaired `one_player_round3` suite passed all 4 tests with `--nocapture`, no skip diagnostics, in 68.92 seconds. One-player breadth and special-move suites also executed successfully in the full sweep. The two explicit ignores are the opt-in boss ladder report and manual GPU timing test. Logs and reproducibility/repair reports are in `tools/reference/local/netplay-preflight-2026-11-10/`.
- **Documentation and warning cleanup:** published 14 previously untracked approved designs/ADRs/mockup/research artifacts. Fixed 15 stale ADR links and four misleading current-state claims about future eras/roster waves. Validation covered 245 relative links in 18 documents, with no missing targets. Removed three unused test declarations; their three suites passed all 9 tests. No production source or version change is part of this preflight cleanup.
- **Other work audit:** active SF1+ worktrees are clean and merged; no unmerged fixes were found. The unmerged select-40 branch is the known throwaway prototype. Private art/voice packs have no unpushed commits against their remote references. Machine-local agent launch settings/worktrees are locally excluded; temporary artifacts left by the earlier interrupted DIP test were removed after path verification.
- **Before George's clean start:** #267 remains with its dedicated child chat; #347 needs physical controller checks. NP1 #304 is technically unblocked (its two native prerequisites #246 and #346 are closed), unassigned and `ready-for-agent`; these remaining human tasks must not be described as completed.

# Session handoff: 2026-11-10 (VR1 published; netplay dependencies corrected; NP1 ready)

- **Authorization:** George asked to finish VR1, merge and push with both release tags, then sort netplay dependencies so NP1 can start.
- **VR1 published:** `867d3ec` implements #346; `68989f2` publishes the accepted versioning design and ADR-0022. Fast-forwarded to `main` and pushed atomically with annotated tags `v1.0.0` -> `72c08782` (#245 baseline) and `v1.1.0` -> `867d3ec` (SF1+/VR1). Issues #345 and #346 are closed with the verification record on #346.
- **VR1 verification:** Standards review 0 findings; Spec review 0 implementation findings. Version unit tests 3 passed; complete `front_end_settings` suite 16 passed, including the dim version stamp on every tab. Both `--version` and `-V` exit 0 for `sf-game` and `sf-dev`, printing `v1.1.0 (867d3ece)`. Release builds pass; Cargo metadata confirms all ten crates inherit `1.1.0` / edition `2021`. Coverage ledger `--check`: 137 identical scenarios, 0 untranslated PCs. Diff whitespace check passes. Window title and first-log-line wiring were source-reviewed, without a separate interactive window check.
- **Full-suite limit:** the broad `cargo test --release -j 2 --workspace` attempt was intentionally stopped during the unchanged long-running DIP-switch suite. RTK reported 514 passed and 1 ignored across 70 completed suites before the stop; this is not a full-workspace pass. The previously missing `1p-block-01-run-a` capture was regenerated locally with the pinned MAME 0.289 scenario, and the blocking suite completed in this run.
- **Netplay sequencing corrected:** baseline NP1-NP8 (#304-#311) delivers online 1v1 and live spectating for the existing twelve tournament fighters and SF1/SF1+ rules. Removed native gates NP1->BC1 #293, NP4->BC2 #294 and NP6->ED4 #287. NP1 now depends only on completed SF1+ #246 and VR1 #346. NP2 retains its VR1 gate; the remaining baseline chain stays intact.
- **Future integrations retained:** NP6 #309 is spectating only. NP9 #352 preserves online Dramatic co-op and is blocked by #308, #309 and local Dramatic setup #287. Era tickets #270-#277, third-fighter engine #286, roster/select/awakening #293/#294/#298 and controls #313 carry linked netplay acceptance items. Initial netplay does not advertise unsupported future modes or fighters. NP8 remains the baseline netplay minor-version milestone.
- **NP1 readiness corrected:** #304 audits the current `Fight` wrapper fields, reuses complete `PowerOnSnapshot`, and requires a canonical authoritative-state checksum with explicit local presentation/diagnostic/audio-delivery exclusions. It no longer assumes future roster types or asks for `Clone` derives that already exist. The design and ADR-0018 record the dated sequencing amendment.
- **Next:** NP1 #304 is unblocked, unassigned and `ready-for-agent`. Implementation has not started in this session.

# Session handoff: 2026-11-10 (SF1+: all 22 tickets of spec #246 done; SP20 Balanced roster tuning landed)

## Session close-out notes (2026-11-10, SP20 and the end of spec #246)

- **Spec #246 complete.** SP1-SP22 (#247-#268) are on `main` and closed; SP20 (#268) was the last.
- **SP20 Balanced roster (#268):** `crates/sf-engine/src/balanced_roster.rs` holds the tuning as tables with a reason per row (frame patches by ROM address, CPU hit-function damage, shoto light entries, Joe's second-strike stun), read only with `balanced_roster` on. Joe's standing guard now takes his standing hurtbox; Geki's walk body no longer stops him 24 px short, so all his normals reach a guard; the SP3 audit outliers are fixed; Sagat's CPU Tiger Shot spawns on tick 12; damage sits in the research's bands. Recorded in design §9.7.
- **Known local gap:** `blocking_and_shoto_symmetry_toggles::faithful_1987_preset_preserves_bit_exact_blocking_and_specials_baselines` needs the MAME capture `tools/reference/local/captures/1p-block-01-run-a`, which is not on this machine; it fails on the missing file, not on behaviour.
- **Open side idea:** #348 (an alternate announcer with a Japanese accent, `needs-triage`).
- **Next:** #346 (VR1, workspace versioning and the v1.0.0 / v1.1.0 tags; it was blocked by SP20 and is now unblocked), then whatever George picks from the future-project designs.

# Session handoff: 2026-11-09 (SF1+: SP4, SP5, SP17, SP18, SP21 merged and verified on local main, awaiting push & close OK)

## Session close-out notes (2026-11-09, SP5, SP18, SP21 finished and merged on local main)

- **Merged and verified on local `main`, not pushed (George's OK needed to push and close #251, #252, #261, #262, #264):**
  - **SP4 Geki (#251):** merge `469d77c`.
  - **SP17 true combos (#261):** merge `ff7624b`.
  - **SP18 cancels (#262):** review fixes finished on `sp18` (`739ee1e`: `SpecialPress` snapshot so a buffered normal press followed by a motion does not cancel into a special; `combo::class_of` and `SHOTO`/`RETSU`/`GEKI` stun rows corrected for diagonal crouching punches `0x12`, `0x13` and `0x1d`; regression tests for buffered normal + motion, strict commands, and hit closing the cancel window), merged to `main` at `09e838c`.
  - **SP5 Joe (#252):** two-axis review completed and fixes committed on `sp5` (`10155e1`: `combo::JOE` diagonal crouching punch stun rows `0x11..=0x13` and `0x1c..=0x1e` aligned with `SP18`'s `class_of`, `0x15` two-strike spacing documented, diagonal crouching punch checks added in `tests/joe_player.rs`), merged to `main` at `ba094a2`.
  - **SP21 fair CPU (#264):** verified on `sp21` (`760918e`), merged to `main` at `83b9d7b`.
  - **Post-merge integration (`ade4d5d`):** wired Joe into `combo::cancel` (`crates/sf-engine/src/combo/cancel.rs`, `tests/cancels.rs`) and `fair_cpu` (`crates/sf-engine/src/fair_cpu.rs`, `tests/fair_cpu.rs`, classifying melee specials on tracker `0x19` as `Role::AntiAir` and offering `Role::AntiAir` in close-range neutral for projectile-less fighters so Joe uses both `0x19` Dynamite Straight and `0x1b` Rolling Sobat).
- **Suite on `main`:** 1,327 passed, 0 failed, 2 ignored across 219 test binaries (`cargo test --release -j 2 --workspace`). Coverage ledger regenerated and `--check` passes (137 scenarios identical, 0 untranslated PCs).
- **Next:** once George gives the OK, push `main` and close #251, #252, #261, #262, #264 (and #247-#250 if also ready). Then SP6-SP12 (remaining bosses), SP19 throws, SP22 (CPU uses modern rules), SP13, SP14, SP15, SP20.

## Session close-out notes (2026-11-08 night, closed early for usage)

- **On local main, not pushed (George's OK needed):** SP4 Geki (#251, merge `469d77c`, Opus-reviewed, fixes in: table-driven special slots, build-time table checks) and SP17 true combos (#261, merge `ff7624b`, Opus-reviewed, fixes in). Ledger `--check` clean on each merge. No full workspace suite has been run on main since `0225450`; run it before pushing.
- **George's calls this session:** Geki: teleport invulnerability 5-26 kept, one Shuriken star, his button mapping. Combos: ground only, no juggles; blockstun locks the guard; one hit per move. CPU: fix Hard so the four levels are monotonic before merging; Joe, Lee, Birdie greyed as Training CPU until their tables land. Joe: specials and buttons as built. Cancels: the lights/mediums rule as built.
- **SP5 Joe (#252)**, worktree `sp5`, branch complete (suites passed: engine 900, art 22, host 105, game 27). Its Opus review was stopped unfinished: rerun the review before merging.
- **SP18 cancels (#262)**, worktree `sp18`: built; the Opus review found (1) a buffered press firing a special the player didn't enter (cancel_from_buffer replaying a slot whose command flags keep accumulating; fix: count only a special ready at the press) and (2) SP17's `combo::class_of` mislabelling 0x12/0x13 (crouching light punches) and 0x1d, with Ryu/Ken/Retsu/Geki rows tuned to the wrong targets. Fix (1) is in, untested, as WIP `3748cd1`; the regression tests, strict-commands and hit-closes-window tests, the class_of retune, rustfmt, suites and ledger are still to do.
- **SP21 new CPU (#264)**, worktree `sp21`: built; the Opus review found the CPU reading a buffered jump-attack press (+9 bit 6) before it's drawn, learned reach only growing, a hard-coded "KIAI SHOT" role, and loose tests. All four review fixes, main (SP4, SP17) and Geki as a CPU fighter are in, committed at `760918e`. The difficulty levels came out monotonic with 12 sparring partners per level (guard rate 0.24/0.27/0.45/0.63, fastest answer 27/19/15/12, jump-ins landed 66/12/6/0), now asserted by a test. Not done: the final sf-engine run reached 96 of 142 binaries with no failures but didn't finish; sf-host (105) and sf-game (27) passed on `760918e`; ledger not regenerated.
- **Recorded on SP20 (#268):** Joe's ducking standing block and Geki's spacing whiffing a blocking opponent, both for the Balanced roster.
- **Process notes:** a builder once edited the main checkout by mistake (stray `moves.rs` edit, not SP21's by its own account; reverted; diff saved in the scratchpad as `main-moves-stray.diff`); background cargo runs sometimes start late; parallel builds above three run out of memory (`-j 2`).
- **Next:** finish SP18 and SP21 fixes, re-review SP5, merge all three, run the full suite, ask George to push and close #251, #252, #261, #262, #264. Then SP6-SP12 (bosses), SP19 throws, SP22 (CPU uses modern rules), SP13, SP14, SP15, SP20.

## Session close-out notes (2026-11-08, SF1+ first batch: SP1, SP2, SP3, SP16)

- **Project 3, SF1+ on the real engine:** design `docs/design/2026-11-07-sf1plus-on-the-real-engine-design.md`, ADR-0014, spec #246, tickets SP1-SP22 = #247-#268. Research: `docs/research/sf1plus-reference-numbers.md`.
- **Merged on local main (`0225450`), each after an Opus review and its fixes:**
  - SP1 #247 (`d8c9585`): Classic Competitive (renamed from SF1++ Competitive; no ban list by default, George); Balanced roster absorbs `balanced_damage_and_chip` and `symmetric_shotos_and_blockable_dp`; inert `true_combos`, `cancels`, `throws`. Upgrade paths: an old SF1++ file loads with no ban list; a Custom file that names no modern rule loads them off.
  - SP3 #249 (`5d31c41`): the ROM box audit, `docs/reference/rom-box-audit.md` and pictures (reach now x-offset + dx + hw from the stance centre). George has the corrected report; it feeds SP20.
  - SP16 #250 (`137df05`): George-approved mirror palettes for all twelve (Retsu saffron and cream, Eagle navy among the redos), P2's body only, Local and both Training kinds, never Faithful 1987 (gate requires a non-Faithful preset).
  - SP2 #248 (`0225450`): Retsu as a 1987 player fighter on his table in the extended data bank at `0x100000` (overhaul machines only). The Kiai Shot is a short-range blast (George) with George's six approved frames (30 of 50 approved PixelLab credits); his 1987 attacks score like Ryu's; the overlay reads the bank.
- **Suite on `0225450`:** 1,243 passed, 0 failed, 2 ignored. Coverage ledger regenerated and `--check` passes (137 scenarios).
- **Not pushed. Nothing closed.** The art pack has one local commit, `cf96ea8` (Retsu's blast lines only), also unpushed. George's OK is needed for push and close (SF1+ batch rule).
- **Next unblocked:** SP4-SP12 (the nine other bosses, each blocked only by SP2), SP17-SP19 (modern rules), SP21 (the new CPU). The SP2 review lists what the other bosses will need: new trackers or move ids for Sagat's low Tiger Shot, charge and mash moves; moving `APEX_POINTER` when move ids are added; a new routine kind for looping specials.
## Session close-out notes (2026-11-07, evening: Training shipped, four follow-ups, project order)

- **Training without rounds shipped** by its session: TR1-TR7 and spec #238 closed, `main` pushed at `a5f7bea`. Suite on `a5f7bea`: 1,172 passed, 0 failed, 2 ignored. The Training and Main Menu sessions are archived. Specs #164, #192, #226 and #150 are closed at George's word.
- **Four follow-ups (George asked for all four):**
  - `1498805`: the after-match box is Local's only; Training fights never end (ADR-0013). Nothing was Training-only to delete, so two comments were corrected.
  - `74edd00`: the moved TIME gives way to ROM text under it. The bonus tally writes ROUND BONUS on text row 9, where TIME was drawn over it; TIME now draws only while rows 8 and 9, columns 28-36, are blank or wiped. Test `the_moved_time_gives_way_to_the_round_bonus`.
  - `a096ceb`: the README roadmap lists the finished specs, marks the engine's later-system models as not yet in real fights, and gives the project order.
  - `271428e`: the showcase reel is committed: `crates/sf-host/examples/showcase.rs`, `tools/showcase/assemble.py`, `docs/media/showcase.mp4` (60 s, 3.4 MB, re-recorded on the new Main Menu and Training).
- **Suite on `271428e`:** 1,173 passed, 0 failed, 2 ignored.
- **Project order (George, 2026-11-07):** 3 SF1+ on the real engine, then netplay (project 7), then 4 era styles, 5 endings / Dramatic Battle / World Tour, 6 bonus challengers.
- **Project 3 brainstorm, decisions so far (George):** both levels before netplay. Classic level = SF1++ Competitive renamed Classic Competitive (1987 rules: no combos, cancels or throws); modern level = Definitive Overhaul (true combos, normal-to-special cancels, techable throws). A Balanced roster switch (numbers only), off in Classic Competitive and on in Definitive Overhaul. The CPU learns the new kit and plays fair, like a human, outside Faithful 1987. Boss endings go to project 5; for now a boss's Arcade clear goes to the staff roll and name entry. The specials are the old spec's list. Claude tunes the moves; George play-tests after and asks for one-off fixes. Research on reference numbers is going to `docs/research/sf1plus-reference-numbers.md`. Next: George picks the build approach (recommended: bosses become 1987 player fighters through extended Ryu-format tables), then the design doc.
## Session close-out notes (2026-11-07, art batch done, round-result names, showcase reel)

- **Art batch finished.** George approved every half of AP14-AP26 (#178-#190) on its review page, and each is merged into local `main`; the last was AP24 at `4d10592`. All twelve fighters (Ryu, Ken, Sagat, Retsu, Geki, Joe, Mike, Lee, Gen, Birdie, Eagle, Adon) load their whole sets from the art pack. `art_at_launch.rs` checks every set, move by move. The art pack's commits are pushed to its private repo. Each rejected pose went through rounds with George's reasons; the serviceable compromises are on `docs/planning/art-upgrades.md`.
- **Art budget.** `ledger::CAP` is now the whole monthly PixelLab allowance, 2,000 (`4bad12c`, George 2026-11-07; 90% before). The allowance resets on 27 November.
- **Round result names the winner (`3d0df63`, George's request).** Where the ROM types RYU, KEN or ENEMY WON THE nTH ROUND, overhaul fights show the winning fighter's name and the rest of the ROM's line, re-centred. The fighter is read from the same move tables as the health-bar names. Tested for P1, P2 and CPU wins in `front_end_layout.rs`.
- **Showcase reel (not committed).** `crates/sf-host/examples/showcase.rs` records menus, a Lee vs Mike Local fight under the ARCADE CABINET look, and Training's move list with Show and Try, with no sound. `docs/media/showcase.mp4` is the assembled 60 s cut (3.45 MB), with title and end cards. George has not said whether to commit either.
- **Noticed, not fixed:** in overhaul fights the moved TIME overlaps the ROM's ROUND BONUS heading on the tally screen.
- **Training without rounds** (spec #238, TR1-TR7 #239-#245) is designed (`581274c`, `c5edee0`, ADR-0013) and being built by its own session. That session merged TR6 (`3396b6a`), TR3 (`b6925e8`), TR7 (`be06b6b`) and TR1 (`931de3e`) into `main`; TR5, TR2 and TR4 remain.
- **Main Menu redesign** (George's request: the frame all the way round, the title screen's logo under its top edge, no MAIN MENU heading, no preset line) is built by its own session (`3650074`) and merged into `main` (`7cf63c1`). It drops the preset line from every front-end screen, by George's choice.
- **Look cache fix and suite.** A GPU look test failed in two of four runs: librashader's cache made a wgpu pipeline cache on devices without `PIPELINE_CACHE` (DX12), and the look's load panicked. `look::load` now enables the cache only where the device has the feature, with a test that loads ARCADE CABINET on such a device (it panicked before the fix). Suite on `main` with the fix: 1,156 passed, 0 failed, 2 ignored.
- **Not pushed. Nothing closed.** Local `main` carries ST2, ST3, ST7-ST10 (#196, #197, #199-#202), ML1-ML6 (#227-#232), KA1 and KA3 (#234, #236; KA2 #235 dropped), AP14-AP26, the round-result fix, the cap change and the Training design and merges. The AP batch is authorized to push and close; the stages-and-music, move-list and key-art batches need George's OK.
## Session close-out notes (2026-11-06, video options shipped, ST1, ST6, names, move list and key art)

- **Video options shipped.** `main` was pushed at `2406a60`, and #194, #198, #215 and #203-#214 are closed. George's check found two issues, both fixed before the close:
  - `e32ca1d`: outside Arcade, the hidden score band let the round's column wipe show a strip of stage. Wiped cells are now neither hidden nor moved.
  - `2406a60`: SAVE AS's name entry now takes typing on the keyboard.
- **Fighter names on the health bars (`9a57fa8`, George's request).** In overhaul fights PLAYER1, PLAYER2 and ENEMY become the fighter's name. It's read from the move table each player structure holds (`+0x12`; P1 `0xff86b0`, P2 `0xff86da`, CPU `0xffc664`), in the label's lettering and colours. The Training overlay goldens and README era-HUD stills were re-blessed; only the label cells changed.
- **ST6 (#195) merged (`00a8d43`).**
  - Adds the extended tune table (commands `0x40..=0x7f`, at `0x8000..0xc000` in the music CPU's image, overhaul only), the score converter (`docs/reference/extended-tunes.md`) and an original test theme (`art/themes/test-theme.score`, command `0x40`).
  - Review fix: a dotted note after a tie needs `0x20` first, because opcode 6 skips the tie count-down.
  - Not wired into the game: `sf_host::themes::program` is for ST7 (#199).
- **ST1 (#193) merged (`ba6aa5b`).**
  - Local and Training get a plain stage screen after fighter select, listing the ten 1987 stages read from the ROM tables (`rom_stages.rs`).
  - An overhaul-only gate skips the country intro when a stage is picked, which also ends the black intro after `--stage`.
  - The pick rules (P1 first, loser picks, the cursor's start) are ST2 (#196).
  - In the 1-player modes, the hidden ladder step still drives the CPU's strength tables and Tatsumaki chip damage.
- **Suite on merged `main`:** 873 passed, 0 failed, 2 ignored. The coverage ledger was regenerated on the merge, and its `--check` passes (137 scenarios).
- **Move list and coach designed, not built.**
  - Design `docs/design/2026-11-06-move-list-design.md`, ADR 0012, glossary section "Move list and coach".
  - Spec #226 and tickets ML1-ML6 (#227-#232). ML1 is unblocked.
  - Real fights have only the 1987 moves today, so the first list is Ryu and Ken's three specials plus the normals.
- **Key art designed, not built.**
  - Research report `docs/research/reports/Street Fighter 1987 promotional art.md`.
  - Design `docs/design/2026-11-06-roster-key-art-design.md`, glossary section "Promotion".
  - Spec #233 and tickets KA1-KA3 (#234-#236).
  - Capcom's November 2025 Fan Content Guidelines bar the game's logo, so the title is set in our own lettering with a fan-made line.
  - Each generation round's credit cost needs George's OK.
- **Push and close:** the stages-and-music, move-list and key-art batches need George's OK before push or close.

## Session close-out notes (2026-11-05, ST4, ST5, #215 and video options)

- **Built and merged to local `main` (`061392f`):**
  - ST4 `#194` and ST5 `#198` from the stages and music batch;
  - the Local VS screen fix `#215`;
  - all eleven video options tickets, VO1-VO11 (`#204`-`#214`).
  - **Not pushed. No issues closed.** That waits on George's OK.
- **Suite (release):** 821 passed, 0 failed, 2 ignored. The two ignored are the opt-in `boss_ladder_report` and the by-hand GPU timing `time_looks`.
- **Review:** an independent Opus review found three crash or data-loss bugs and four lesser issues. All are fixed with regression tests (`3391002`..`d50ad3b`):
  - a long look name crashed Settings;
  - a `#` in a look path reset `settings.toml`;
  - a non-ASCII look error panicked;
  - top-level drop-in name clashes;
  - temp presets leaked, with a test race over their folder;
  - saved-look case clashes;
  - a failed look is never saved as OFF.

  A resize while a frame is held could panic off Windows; it is noted in the design as a known issue.
- **User decisions (2026-11-04/05):**
  - **Extended background pages are chosen by the gate,** not by scroll bit 15. This is an amendment to ADR 0009.
  - **The window is sized in real screen pixels.**
  - **ARCADE CABINET is accepted at the ship bar.** It ran 58.2-58.9 game fps at true 4:3 and passed in 5 of 7 runs, so the smoke check may sometimes fail on it.
  - **FRAME DELAY is capped by look:** 4 ms with CRT LOOK off, 0 ms with any look on. The saved value is kept and clamped when used.
- **VO11 measured on George's machine:**
  - Late refreshes through ARCADE CABINET fell from about 131 to about 10 per 5 s, and presents rose from about 94/s to 116-118.
  - The fix queues 3 frames while a look draws, and the late-refresh count no longer overcounts.
  - Exclusive fullscreen was not added: winit panics in `set_fullscreen` on this machine. DX12 can't run the looks through librashader's wgpu runtime.
- **Open:**
  - **Country intro:** with a test stage picked (dev `--stage test-stage` only), the country intro shows black for about 90 frames. ST1 will decide where the fight's fg palette request comes from.
  - **Training against the CPU:** its VS screen still shows Ryu vs the opponent (the mode-2 branch, outside #215).
  - **Settings file:** one bad key in `settings.toml` still refuses the whole file. That is the existing contract; changing it is a product decision.
  - **By eye, for George:** each look during a stage pan, the tuning panel's legibility, and the SAVE AS name entry.
- **Shared local data:** the coverage ledger regeneration rewrote the shared `tools/reference/local/coverage/ledger.json`. It now matches `main`.

## Session close-out notes (2026-11-04, era HUD built)

- **EH1-EH9 (`#217`-`#225`) are built, merged to `main` and verified locally. Ends at `c484a53`.**
  - **Not pushed. No issues closed.** That waits on George's OK.
  - **Suite (release):** 704 passed, 0 failed, 1 ignored, across 172 suites. No ROM-gated test skipped.
  - **Voice pipeline:** `python -m unittest discover -s tools/voice/tests` gave 23 OK.
  - **Review:** an independent Opus review found nothing blocking. Its fixes are in `2fac1d9`:
    - TIME is hidden only where the ROM drew it;
    - Restart Round clears call-outs;
    - the paused fight redraws on a gauge art change;
    - the hitbox viewer draws under the HUD;
    - the telemetry boxes keep their place at the top.
- **User decisions (2026-11-04), recorded in the design doc:**
  - **The combo count is true combos only.** A 1987 fighter is invulnerable for his whole hit reaction, so a real SF1 fight never shows a count.
  - **REVERSAL is literal:** only out of true blockstun or a get-up. 1987 has neither.
  - Both show once a style adds those states. FIRST ATTACK and PERFECT work in real fights now.
- **The announcer is voiced:**
  - ElevenLabs design round 1, preview 2 (voice `jSpNnJd6IV3EN0dVoIXm`), with George's 22 picks. It cost 288 credits; `tools/voice/ledger.json` has the record.
  - The pack is local at `tools/reference/local/roms/sf1-port-voice`, with its archive inside.
  - **Backed up** to the private `geox123/sf1-port-voice` (`99794aa`), the way the art pack is. Push it after each `build-pack`.
- **Where the era HUD stands in real fights:**
  - Real fights carry no era state yet, so gauges and the era call-outs show only in `sf-dev era-hud`, in the tests, and in the README stills (staged, from `cargo run -p sf-host --release --example era_hud_shots`).
  - Project 4 wires them in through `Fight::set_era_hud`, `push_callouts`, and the reader's `blockstun` / `getting_up` / `blocked` fields.
- **Open, for later:**
  - P1 call-out words longer than 12 letters, landed just before a KO, could cross the ROM's centre round-end text (x 104..256). It can't happen until project 4 emits those events.
  - The Blocks mirror test checks shape, not colours.
  - ADR 0003 and the 2026-10-27 design still say 8 kHz.
  - GAUGE ART goes beside LOOK when video options land.

## Session close-out notes (2026-11-04, AP13)

- **AP13 `#177` closed (`6dd38fa`):** Joe's ground set, 14 poses in eight moves, is approved; the crouch took five rounds. He stays on his ROM art in the game until AP14 (`#178`) finishes his set.
  - **His streams:** move table `0x1734c` (move 4 is his standing block `0x176c`; move 5's crouch pointer is broken and move 8 is empty, so he has no crouch); hit and knock-out tables `0x19668` / `0x1969c`; attacks `0x0e3ba`-`0x0eade`. His 15 pens are all different colours.
  - **Free:** the hip toss (`0x1768`, `0x194a`, `0x1a00`, `0x17c4`), the thrown set (`0x182c`, then `0x19ac` turned), and a get-up through his jump's tuck (`0x1a88`) set on the ground. The crouching block holds the crouch, as Sagat's does.
  - **Generated:** the crouching hit, crouching kick and sweep, whole; the crouch is the approved crouching kick with its front leg inpainted, then its hair.
  - **The user's calls on the crouch:** his ROM duck (`0x17c0`) and standing block were not low enough; the kick's generated hair did not look right; a spiky flat-top looked like Guile (his hair is rounder and smoother); the rounded hair looked see-through until the skin tones in it took his hair's olive and pale yellow.
  - **Lessons:** at his own scale the generator would not crouch him below his 74-pixel duck, and a shorter canvas shrank him; inpainting a leg on an approved squat got the height. His hair is part of his look: only olive, pale yellow, brown and black, rounded.
  - **Recipe says `still_to_come`** his air set and specials (AP14, #178). Remove it when AP14 is approved, and turn the local-pack test's joe assertion round, as AP12 did for Geki. Ground set alone logs `joe (8 moves; 8 ROM poses, 2 rebuilt, 4 generated)`.
  - **Cost:** 122 generations, about 30 a kept generated pose (72 on the crouch). The month from 2026-10-27 stands at 880 of 1,800, and PixelLab agrees. Pack backed up (`778a1fd`).
  - **Art upgrades:** six rows (crouching block, throw, thrown and get-up stand-ins; the crouching hit's height; the generated hair on three poses).
  - **Suite (release):** 618 passed, 0 failed, 1 ignored.
- The local review page for Joe is the `art-review-joe` entry (port 8793) in the untracked `.claude/launch.json`.

## Session close-out notes (2026-11-04, era HUD)

- **Era HUD specced: #216, tickets EH1-EH9 = #217-#225.** These are sub-issues with native blocking edges, all `ready-for-agent`. The work is the screen side of project 4.
  - **Design:** `docs/design/2026-11-04-era-hud-and-callouts-design.md`, then grilled.
  - **Spec and tickets:** `docs/planning/era-hud-spec.md` and `era-hud-tickets.md`.
  - **Decisions recorded:** ADR 0011 (the announcer on a third emulated MSM5205), and the Fight HUD terms in `CONTEXT.md`.
- **User decisions:**
  - **Layout:** one invented 1987 style for every era. Gauges sit in the bottom corners and TIME moves under the health bars. Local and Training drop the scores, and the Faithful 1987 preset keeps the plain 1987 screen.
  - **Gauge art:** Ruled (default), Blocks and Plate, chosen in VIDEO and the pause box.
  - **Call-outs:** every later-era moment gets one, plus a combo counter, FIRST ATTACK, REVERSAL and PERFECT in every overhaul fight. Nothing ever overlaps.
  - **Announcer:** a generated voice designed to match the 1987 announcer, done the Highfall way with ElevenLabs. `ANNOUNCER` is a 0-10 volume. CPU sides are treated like human ones.
- **Frontier:** EH1 `#217` and EH2 `#218`. EH9 needs George to pick the takes, and its generations cost money.
- **Not yet authorized:** pushing and closing for this batch. Ask first.
- **Mockups** are local in `.superpowers/brainstorm/19485-1793799909/content/`.

## Session close-out notes (2026-11-04, README stills)

- **The README shows the game as it draws itself (`0eb6996`, `db570ce`, pushed).** The old "Overhaul sessions" stills (SF2, Alpha, SF3, SF4 and SF6 gauges, the SF1+ ending, the Dramatic Battle GIF) were drawn by `crates/sf-engine/examples/readme_frames.rs` from the standalone model, with coloured boxes for fighters; no game path reaches those features yet. The user asked for in-game shots only, and for the painted player-select mockup to go now that the select is real.
  - **Removed:** the eight stand-in files in `docs/media`, `player-select-mockup.png` included, `readme_frames.rs`, and the README's pointer to the mockup.
  - **Added:** a "The front end" table with `main-menu.png`, `settings.png`, `player-select.png` (Local, Sagat vs. Geki) and `local-fight.png` (Sagat's high kick on a jumping Geki, art pack loaded).
  - **Recapture:** `cargo run -p sf-host --release --example readme_shots -- docs/media [fight frames folder]` drives a headless `GameSession` from power-on with the art pack. The optional second folder gets every 20th fight frame for picking a shot. `sf-host` gained `png` as a dev-dependency.
  - **Found on the way:** the Local VS screen bug, now `#215` (section below).
  - **Not done:** the README's "In the tree" roadmap still describes the era styles, Dramatic Battle, World Tour, rollback and bonus challengers as present, and its portrait-select line still says "three columns, large frame on the left" (Local is now a VS layout with two rows of six). The "Layout reference" note further down this file points at the deleted mockup.
  - No tests were run; the change is docs, media and one example, and `sf-engine`'s examples still build.

## Session close-out notes (2026-11-04, Local VS screen)

- **Bug filed as `#215`:** in Local the pre-fight VS screen always shows RYU VS KEN, whoever was picked. The fight itself binds the picked pair correctly. The ticket is `ready-for-agent`, under spec `#192`, and blocked by ST5 `#198`, which is blocked by ST4 `#194`.
  - **Cause:** event 4's versus branch (`0x3fa00`, `opponent_intro_start` in `tasks.rs`) always scrolls the background to `0x780` with palette `0x1ae68`. That translation is faithful. A VS screen is a fixed window of the ROM background tilemap with the busts and nameplate baked in, and the ROM has only Ryu vs each boss (mode 2), each boss vs Ken (mode 4) and Ryu vs Ken.
  - **ROM probe (Japan tables):** each bust uses one colour row.
    - Ryu and the name text use row 2, identical everywhere.
    - Ken uses row 3 (row 4 on the versus screen, same colours).
    - The bosses use row 4 (Retsu, Mike, Lee, Birdie, Adon), row 5 (Joe, Gen, Eagle, Sagat) or row 6 (Geki).
    - Every boss has a left and a right bust; Ryu has only a left one and Ken only a right one.
  - **The user's decisions:**
    - Composite VS screens in the extended background bank, built from ROM tiles and colour rows when the game loads, not a front-end overlay.
    - Same-row bosses move to a free row.
    - Ryu or Ken on his unusual side is mirrored with the map's flip bit, name unflipped.
    - Scope is two-player front-end fights under overhaul presets.
    - An overhaul-only gate on `0x3fa00` picks the screen.
  - **ADR-0009 amended (`29c8196`)** to cover VS screens.
  - No engine code changed and no tests were run.

## Session close-out notes (2026-11-04, AP12)

- **AP12 `#176` closed (`648d2c0`):** Geki's air set and specials, 11 poses in five moves, are approved in one round. His whole set (29 poses, 14 moves) now loads: `geki (14 moves; 18 ROM poses, 2 rebuilt, 9 generated)`, and the bank spans 4,206 tile slots.
  - **Free:**
    - the top of the jump (his squat `0x0fc4`, shown 48 pixels up);
    - the Teleport Strike, AP2's stand-in: smoke `0x11e6` / `0x1326` / `0x1329` / `0x132d`, then his claw thrust `0x0ea0` / `0x0ea6`.
  - **Generated:**
    - the jumping punch, his arm inpainted on the squat;
    - the jumping kick, heavy jumping kick and diagonal jumping kick, whole. The two high kicks are two candidates of one call.
  - **Stay ROM streams:** his jumps, his diving claw (his diagonal jumping attack) and his Shuriken.
  - **`still_to_come` is gone** and the local-pack test checks all five fighters.
  - **Cost:** 42 generations; his whole set took 129, about 14 a kept generated pose. The month from 2026-10-27 stands at 758 of 1,800, and PixelLab agrees. Pack backed up (`c24b70a`).
  - **Art upgrades:** his heavy jumping kick (a near twin of the jumping kick) and his Teleport Strike (a ROM stand-in) are listed.
  - **Suite (release):** 618 passed, 0 failed, 1 ignored.

## Session close-out notes (2026-11-04, AP11)

- **AP11 `#175` closed (`1aff874`):** Geki's ground set, 18 poses in nine moves, is approved after three rounds. He stays on his ROM art in the game until AP12 (`#176`) finishes his set.
  - **His streams:** move table `0x16bfc` (moves 4, 8 and 0xc are his teleport, so he has no blocks); hit and knock-out tables `0x19548` / `0x1957c`; attacks `0x0b968`-`0x0c20e`.
  - **Free:**
    - the crouching block (his jump squat `0x0fc4`);
    - the circle throw: claw-thrust wind-up `0x0ea0`, fall pose `0x0eac`, somersault tucks `0x066a` / `0x0667`;
    - the thrown set, and a get-up that rolls up in his somersault tuck;
    - the light kick's chamber (`0x0f08`).
  - **Generated:**
    - the standing block, whole (his hit pose `0x0f00` would read as being hit);
    - the jab, inpainted on his stance;
    - the light kick, crouching kick and sweep, whole.
  - **The user's calls:** the jab's wind-up dropped; the inpainted kicks redone whole (the edit showed, his body should move); the sweep a real low sweep from another candidate of the same call.
  - **`sf-art pack` fix:** two pens can share a colour (Geki's body blue, pen 14, is his smoke's, pen 6). An exact tie now goes to the pen his stance draws (`stance_pens`). Ken, the only other fighter with a shared colour, is unchanged. Tested.
  - **Recipe says `still_to_come`** his air set and specials (AP12, #176). Remove it when AP12 is approved, and turn the local-pack test's geki assertion round, as AP10 did for Retsu. Ground set alone logs `geki (9 moves; 11 ROM poses, 2 rebuilt, 5 generated)`.
  - **Cost:** 87 generations, about 17 a kept generated pose. The month from 2026-10-27 stands at 716 of 1,800. Pack backed up (`72007f4`).
  - **Art upgrades:** four stand-ins listed (crouching block, throw, thrown, get-up).
  - **Suite (release):** 615 passed, 0 failed, 1 ignored.
- The local review page for Geki is the `art-review-geki` entry (port 8792) in the untracked `.claude/launch.json`.

## Session close-out notes (2026-11-03, video options)

- **Video options designed, grilled, specced and ticketed.** Nothing is built in the workspace yet.
  - Design [`2026-11-03-video-options-design.md`](design/2026-11-03-video-options-design.md), approved (`48e0626`).
  - Grilling: [ADR-0010](adr/0010-gpu-window-and-retroarch-looks.md), **Look** and **True 4:3** in `CONTEXT.md`, and the design's settled answers (`3b61112`).
  - Spec [#203](https://github.com/geox123/sf1-port/issues/203), with a copy in `docs/planning/video-options-spec.md`.
  - Tickets VO1–VO11 = #204–#214, sub-issues of #203 with native blocking edges, all `ready-for-agent`.
- **A throwaway test proved the approach** on the user's machine (Radeon 780M, 3200×2000, 120 Hz). The test was winit 0.30, wgpu 30 and librashader 0.12.1, playing the real `GameSession` at true 4:3. Guest advanced and full Royale hold about 120/60. Guest NTSC reaches only about 47 fps, and crt-geom only about 3 fps (cause unknown). The user played it: "Pans felt better". The numbers are in the design's section 4.
  - The test lived in the session's scratchpad, outside the repo, and was thrown away. The shaders came from `libretro/slang-shaders` `53deb2ef`.
- **The user's ship bar:** the pacing they played (a handful of late refreshes per 5 s) is fine to ship. Cutting it further is VO11, which is ongoing work, not a gate.
- **Front-end spec story 5** (first launch "3× windowed") is superseded: first launch goes fullscreen with the `ARCADE CABINET` look (VO6).

## Session close-out notes (2026-11-03, stages and music)

- **Stages and music designed, grilled, specced and ticketed.** Nothing is built yet.
  - Design [`2026-11-03-stages-and-music-design.md`](design/2026-11-03-stages-and-music-design.md), approved (`a77d077`).
  - Grilling: [ADR-0009](adr/0009-stages-and-themes-in-extended-banks.md), a "Stages and music" section in `CONTEXT.md`, and the design's settled answers (`42d6a42`).
  - Spec [#192](https://github.com/geox123/sf1-port/issues/192), with a copy in `docs/planning/stages-music-spec.md` (`c6c1853`).
  - Tickets ST1–ST10 = #193–#202, sub-issues of #192 with native blocking edges, all `ready-for-agent`.
- **What it builds:**
  - **A stage select in Local and Training under overhaul presets.** P1 picks first, then the loser of the last match, with the last picker going again after a draw. The rule resets at the menu. Rematch goes through the stage screen with the stage just played kept by one press. There is a RANDOM tile. The cursor starts on P2's fighter's home stage (Retsu's for Ryu, Joe's for Ken), or on the CPU opponent's home stage in Training.
  - **The country intro is skipped there.** Faithful 1987 keeps the flag screen and the intro. Arcade is unchanged.
  - **The choice reaches the engine through overhaul-only gates.** A roster setting is read by gates in `stage_reset`, `request_stage_palette` (five call sites, chosen by country and ladder step only) and `stage_music`. Nothing writes RAM (ADR-0004).
  - **New stages and themes live in extended banks (ADR-0009).** An extended background bank holds maps past the ROM tilemap's 2048 columns. An extended tune table is played by the translated FM driver, with a score converter. Themes are FM-only for now.
  - **Proof content:** a test stage built from ROM tiles and an original test theme, both committed.
  - **New stages are for bonus challengers only, one each:** 38 stages in all. A challenger with a well-known theme gets it arranged for the 1987 chip; one with no theme gets new music in that style. Arrangements of Capcom melodies go in a local **music pack**, never the public repo.
  - **The Jukebox is wired into fights,** with a Jukebox screen behind the greyed Sound row. Tracks can be OGG, WAV, FLAC, or tracker modules through libopenmpt (its full list; the C++ build on Windows is part of ST9).
- **Push and close for this batch** were not pre-authorized at publication. The user approved this push; ask before closing tickets.
- **Not touched, still in flight in the checkout:** another session's art work: `sf-art` main/pack and its test, `sf-game` `art_at_launch`, `sf-host` art, the art-pipeline design, `art-upgrades.md`, and Geki's untracked generations, recipe and review files.

## Session close-out notes (2026-11-03, AP10)

- **AP10 `#174` closed (`f08f202`):** Retsu's air set and specials are approved after five rounds. His whole set, 21 poses in 10 moves, loads in overhaul fights: `retsu (10 moves; 11 ROM poses, 3 rebuilt, 7 generated)`, with the bank at 3,278 tile slots.
  - **Jumps:** his three ROM jumps and his flying kick (the diagonal jumping kick, `0x1248`) stay ROM streams. The catalogue holds his top-of-jump pose (`0x1288`), and the four jumping normals start and end on it.
  - **Punch and light/medium kick:** one limb inpainted on the jump pose.
  - **Diagonal punch and heavy kick:** generated whole after inpainted tries were rejected (the user wanted the body to lean; a section was missing, then a seam unclean).
  - **Kiai Shot is a ROM stand-in (user's call at the review limit):** his jab's wind-up and open-palm strike (`0x1120`, `0x1320`). Four rounds of generated two-handed poses were rejected, the last three for the belt. It is on `docs/planning/art-upgrades.md`.
  - **`still_to_come` is gone** from his recipe, and the local-pack test checks all of his moves.
  - **Lessons for the other eight** (design section 11, AP10):
    - a mask must cover what a new limb crosses, and clear what the old pose had there;
    - hand edits to clothing read as edits, so inpaint the whole region or generate the pose whole;
    - whole generations leave no seams.
  - **Chat verdicts:** the user's round-three verdicts, given in chat, were written into `art/review/retsu.json` by hand.
  - **Cost:** AP10 216 generations (Kiai Shot 142, for no pose). Retsu's set took 268 for 7 kept generated poses, about 38 each (18 without the Kiai Shot). The month from 2026-10-27 stands at 629 of 1,800. Pack backed up (`44d9233`).
  - **Suite (release):** 613 passed, 0 failed, 1 ignored.

## Session close-out notes (2026-11-03, AP9)

- **AP9 `#173` closed (`96d7249`):** Retsu's ground set, 14 poses in five moves, is approved after two review rounds. He stays on his ROM art in the game until AP10 (`#174`) finishes his set.
  - **His ROM art did most of it.** He already has a crouch, a crouching block, a crouching hit and a spinning sweep (his ROM streams, outside the catalogue). His streams: move table `0x16e6c`, hit and knock-out tables `0x195a8` / `0x195dc`, attacks `0x0c9b6`-`0x0d074`.
  - **Free:**
    - the wrist throw: his jab's open-hand strike, his lunging punch, his sweep's last pose mirrored;
    - the thrown set: his standing hit, his fall pose turned;
    - the get-up: lying, his fall pose on the floor as sitting up, crouch, half-crouch, stance.
  - **Generated:**
    - the crouching punch's wind-up and palm strike, his ROM crouch with only the arm inpainted (approved first time);
    - the crouching kick, generated whole, squatting on his rear foot as Ryu's crouching kick does.
  - **The user's calls:** a kick inpainted on his kneel was rejected, since it should not kick out from a kneel; the chamber was dropped as not needed after two tries.
  - **Recipe says `still_to_come`** his air set and specials (AP10, #174). Remove it when AP10 is approved, and turn the local-pack test's retsu assertion round as AP8 did for Sagat. Ground set alone logs `retsu (5 moves; 8 ROM poses, 3 rebuilt, 3 generated)`.
  - **His pens:** his ROM body is drawn in 0-6 and 8-11; the lilac greys (12-14) and the lining maroon (7) the generator reaches for go to his nearest body pens (`candidates/retsu/ground-touch-ups.py`).
  - **Cost:** 52 generations in 8 calls, about 17 a kept generated pose; the month from 2026-10-27 stands at 413 of 1,800. Pack backed up (`218956a`).
  - **Art upgrades:** three stand-ins listed (throw, thrown, get-up).
  - **Suite (release):** 613 passed, 0 failed, 1 ignored.
- The local review page for Retsu is the `art-review-retsu` entry (port 8791) in the untracked `.claude/launch.json`.

## Session close-out notes (2026-11-03, AP8)

- **AP8 `#172` closed (`69da63b`):** Sagat's air set and low Tiger Shot are approved after two rounds. His whole set, 26 poses in 16 moves, loads in overhaul fights: `sagat (16 moves; 8 ROM poses, 3 rebuilt, 15 generated)`, with the bank at 2,512 tile slots.
  - **Jump arc:** free, his ROM hop (half-crouch, leg lift, knee pose).
  - **Jumping normals:** five, generated. The punch and kick were inpainted on the ROM knee pose so his legs stay the same between frames.
  - **Low Tiger Shot:** two poses, generated.
  - **`still_to_come`** is gone from his recipe. The mechanism stays for any fighter whose set comes in parts.
  - **The pilot's rate:** 251 generations for Sagat (17 a generated pose, 16 a move). The other nine bosses look like 1,900 to 2,500 generations, finishing by late December 2026. The review, at about 40 to 45 rounds, will set the pace, not the budget. Details are in design section 11 (AP8).
  - **Month from 2026-10-27:** 361 of 1,800. Pack backed up (`f4d52ce`).
  - **Suite (release):** 613 passed, 0 failed, 1 ignored.

## Session close-out notes (2026-11-03, AP7)

- **AP7 `#171` closed (`5abb33d`):** Sagat's ground set, 17 poses in 9 moves, is approved after three review rounds. He stays on his ROM art in the game until AP8 finishes his set.
  - **Free:** the tiger-clinch throw (Tiger Shot thrust, Tiger Knee, lunging punch) and the thrown set (his hit pose, his recoil turned). The get-up is lying, crouch, half-crouch, stance. His lying pose is rebuilt from ROM pixels: the dropped eyepatch on the floor removed, his stance's patch laid over his eye.
  - **Generated:**
    - the crouching hit, punch and sweep, each on his crouch, posed by Ryu's ROM moves, with his stance as the style image;
    - the kick, from the spike's candidate with the crouch's upper body;
    - the kick's chamber, inpainted on the crouch.
  - **The user's calls:** his crouch serves as his crouching block; the kick's half-way frame was dropped.
  - **Recipes can say `still_to_come`:** the game keeps him on his ROM art until it is removed, and logs why. Remove it from `art/recipes/sagat.json` when AP8 is approved; the local-pack test then needs its sagat assertion turned round.
  - **Inpainting** (`inpaint_image_pro_flash`, 5 generations) keeps everything outside the mask. Use it to change one part of an approved pose; `create_image_pro` keeps standing him up.
  - **Free poses carry no fingerprint:** reset a free pose's verdict by hand when its recipe changes.
  - **Cost:** 115 generations, about 17 a generated pose counting the spike. The month from 2026-10-27 stands at 245 of 1,800. Pack backed up (`d2c1c5a`).
  - **Art upgrades:** four stand-ins listed.
  - **Suite (release):** 613 passed, 0 failed, 1 ignored.

## Session close-out notes (2026-11-03, bonus roster by lore)

- **Bonus challengers re-chosen by 1987 lore (`88f8f7e`).** Design [`2026-11-03-bonus-roster-by-lore-design.md`](design/2026-11-03-bonus-roster-by-lore-design.md) (approved), [ADR-0008](adr/0008-bonus-roster-chosen-by-lore.md), and spec [#191](https://github.com/geox123/sf1-port/issues/191), a sub-issue of #91. The spec replaces section 5.4 of the 2026-10-28 design, which now carries a pointer to it.
- **The roster:** 28 bonus challengers (40 with the twelve) in three **roster waves**:
  - Wave 1, *Super Street Fighter: Rising*: Evil Ryu and Bison.
  - Wave 2, *Hyper Street Fighter: Bloodlines*: ten fighters tied to the twelve.
  - Wave 3, *Ultra Street Fighter: Final Round*: sixteen fighters who existed in 1987, Final Fight-heavy.
- **Glossary changes:** *Hyper* moved off era styles, which are now *Anniversary Edition*. Sheng Long is gone. SF1 and SF1+ are native styles for bonus challengers.
- **Content after feature complete, no deadline.** A wave ships only when every fighter in it is fully finished.
- **No tickets yet, by design.** Each wave gets its own spec and tickets, as sub-issues of #191, when its work starts. Leave #191 open.
- **Select screen layout chosen (`3b99c44`).** Layout C: a centre grid of 26×30 tiles, up to 8×5, with tall frames on either side, in both Arcade and Local. In Arcade the right-hand column is the style list. Partial rosters use even rows: 6×2 for the 12, 7×2 after wave 1, 8×3 after wave 2, 8×5 after wave 3.
  - The mockup is on branch `prototype/select-40`, linked from a comment on #191.
  - A local copy is in the gitignored `.superpowers/select-40-prototype/`.
  - The `select-40-prototype` entry in the untracked `.claude/launch.json` serves it on port 8795.

## Session close-out notes (2026-11-03, AP6)

- **AP6 `#170` closed (`73cea04`):** Ryu's and Ken's throws, thrown sets and get-ups are approved on the review page and load in overhaul fights.
  - **Ryu:** the shoulder throw is his Hadoken thrust as the grab, then a generated turn and heave (PixelLab Pro, 108-pixel canvas, his ROM grab as the style image). It was approved at the second round and cost 110 generations; the month from 2026-10-27 stands at 130 of 1,800. His thrown set is ROM poses: standing hit, his fall pose turned a quarter and a half, then lying.
  - **Ken:** his ROM poses are Ryu's codes + 0x800 with the same grids. His turn and heave are **derived poses**: Ryu's approved files in Ken's colours, with his own differences fixed by hand against his ROM art (no headband, his hair, bare hands, his ROM feet). They were approved at the third round and cost no generations. The script and every round's files are in the pack's `candidates/ken/`.
  - **Ledger:** `derived` entries record the source file and touch-ups. The check refuses one whose source no call kept.
  - **Public tests** build from `sf_video::art::rom_only(recipe)`, since the pack is private. `the_local_pack_loads_every_approved_fighter` loads the real pack and skips without it.
  - **Pack backed up** to `geox123/sf1-port-art` (`64b38c3`, `638bb0d`). Five ROM stand-ins were added to `docs/planning/art-upgrades.md`.
  - **Suite (release):** 612 passed, 0 failed, 1 ignored.
- **Debug-build overflow fixed (`6a4cb23`):** `fighter.rs` `grid_entries` multiplied `k * 0x40` in u16. In a debug build that panicked on the garbage grid of Joe's untranslated branch (`front_end_local`'s `an_untranslated_branch_shows_the_notice_returns_to_the_menu_and_logs`); release builds wrapped. It now wraps in both, on the same lines, so the coverage ledger's line numbers hold. The whole suite with overflow checks and debug assertions on (`CARGO_TARGET_DIR=target/checked CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS=true CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS=true cargo test --release --workspace --no-fail-fast`) passed 612, failed 0, ignored 1; it found no other overflow. Run the suite in release anyway: a debug run takes over two and a half hours.

## Next work

- **Art pipeline frontier:** AP14 `#178` (Joe: air set and specials; AP2 describes his Rolling Sobat and Dynamite Straight, with no Capcom original). Sagat, Retsu and Geki are finished; Joe's ground set is approved.
  - Use `sf-art palette` and `sf-art reference` for every new generation, with his ROM stance as the style image, and reduce to his body pens.
  - Take the pose base from his own ROM poses and the posing guide from Ryu's matching ROM move.
  - Inpaint one part only when the pose under it already stands the way the move needs, the mask covers what the new part crosses, and clears what the old pose had there; otherwise generate the pose whole.
  - Don't edit clothing pixel by pixel: the user rejects visible edits. Inpaint the whole region or generate the pose whole.
  - Check Joe's hair in every generated pose: a smooth, rounded crew cut in olive, pale yellow, brown and black only. Skin tones in it read as see-through; a flat-top reads as Guile.
  - Keep each canvas tall enough for his full scale: a 72-pixel-tall canvas shrank him about 15%.
  - A call's other candidates cost nothing more.
  - Model the touch-ups on `candidates/joe/ground-touch-ups.py`.
  - When AP14 is approved, remove `still_to_come` from `art/recipes/joe.json` and turn the local-pack test's joe assertion round.
  - Before each review, check his face and hair, the legs between frames, his size against the frames around, and any seam at a mask's edge.
- **Stages and music frontier** (#192): ST1 `#193` (a chosen 1987 stage reaches the fight), ST4 `#194` (map the 1987 background format, the riskiest), ST6 `#195` (extended tune table, converter, test theme). ST3 `#197` needs the user to approve a mockup first.
- **Local VS screen** `#215`: starts after ST5 `#198`. Its acceptance criteria name the pairs to test.
- **Video options frontier** (#203): VO1 `#204` (vendor the shaders) and VO2 `#205` (the GPU window with even pacing) start at once; VO4 `#207` (the first look) then fans out. VO5 needs the user's eye on every built-in look.
- Leave `#91`, `#113`, `#150`, `#163`, `#164`, `#191`, `#192` and `#203` open.

## Session close-out notes (2026-11-02, after FE12: menu positions, art pipeline)

- **Menu positions (spec #150, story 69), `21d1a70`:** each of the four menus opens on the row it was left on, saved in `settings.toml`. The game writes them when it ends. That exit save is not checked live, because the desktop was locked.
- **Art inventory, `9abf444`:** `docs/reference/art-inventory.md` maps each fighter's ROM art against a full moveset. The ten bosses lack about 254 poses before specials. No fighter has a throw or a get-up, and Sagat has no crouch or jumps. It lists four errors in other reference docs (Ken's move table in `ken-attacks.md`, two in `sagat.md`, one in `adon.md`), still uncorrected.
- **The spike:** PixelLab Pro made a crouch and a crouching kick for Sagat from his ROM frames, for 20 generations. The results stayed in his 14 colours and passed in a real fight frame. About one candidate in four was unusable, and faces drift. The pictures are in the session scratchpad only.
- **Art pipeline, project 2:**
  - design approved, `0aa83d6`;
  - grilling recorded, `39dddf1`: ADR-0006 (ROM-only art; fan move data still allowed as reference), ADR-0007 (extended sprite bank), five glossary terms;
  - spec [#164](https://github.com/geox123/sf1-port/issues/164), a sub-issue of #91;
  - tickets AP1-AP26 (#165-#190), sub-issues of #164 with native blocking edges.
- **Key decisions:**
  - free reuse and piecing first, PixelLab generation for the rest;
  - the user approves each fighter on a local review page;
  - smoothness matches each fighter's own ROM moves;
  - Mike and Eagle get no kicks;
  - generated pixels live in a local art pack beside the ROMs, backed up to a private `geox123/sf1-port-art` repo once the first generated pose is approved;
  - at most 90% of the 2,000-generation monthly PixelLab allowance;
  - the user accepted that PixelLab Pro outputs may be used for its training.

- **AP2 `#166` closed (`b8e8c8d`):** `docs/planning/art-pipeline-moves.md` gives every fighter's throw, the new specials, eight ROM stand-ins, and Mike's punches and Eagle's stick moves on the kick buttons. The user approved it as written. Lee's two specials have no Capcom original either. Retsu's Kiai Shot burst is flagged as a new effect for SF1+.
- **AP1 `#165` closed (`b6f4f9a`):** `sf_video::sprite_bank` adds tiles after the ROM's 14,336 in *both* halves of the sprite region, because MAME's `sprite_layout` splits the planes. Each tile sits in the slot the renderer's swizzle reads. `GameSession::with_sprite_bank` gives overhaul fights the bank. A sweep of all 137 scenarios (874,344 frames) shows the game never draws past tile `0x33bf`. Suite: 587 passed, 0 failed, 1 ignored.
- **AP3 `#167` closed (`4f8a212`):**
  - Recipes (`art/recipes/ryu.json`, embedded in the game) are ROM poses used as they are, or poses built from parts into the bank.
  - `sf_video::art::load_art` returns the bank and pose catalogues, with anchors against the fighter's centre and the ground line.
  - Ryu's get-up is ROM poses only: lying, rising, crouch, stance. The first rebuilt pose is `thrown_1`, his fall pose turned a quarter.
  - The engine's grid layout is now `pose_grid_entries`; the coverage ledger was regenerated.
  - The game logs `art: ...`. Suite: 594 passed, 0 failed, 1 ignored.
- **AP4 `#168` closed (`e3a4f66`):**
  - `sf-art review <fighter>` serves a local page: each move plays in a real fight frame, drawn by the game, beside his own ROM art. Verdicts go to `art/review/<fighter>.json`.
  - The user approved Ryu's get-up as serviceable, the first entry in `docs/planning/art-upgrades.md`. `thrown_1` was approved on the over-the-top throw reading: head first, toward the thrower.
- **AP5 `#169` closed (`e95f3ff`):**
  - **The art pack** is a folder, `sf1-port-art`, beside the ROM folder. It holds a manifest with CRC-32s, poses as indexed PNGs, and raw candidates. Here it is `tools/reference/local/roms/sf1-port-art` (gitignored), a clone of the private [`geox123/sf1-port-art`](https://github.com/geox123/sf1-port-art). Commit and push in that folder after each approval.
  - **The game** embeds recipes and reviews and loads a fighter's new poses only when all are approved. Otherwise he keeps his ROM art, and the log says why: not approved, or the pack missing, damaged, out of date or mismatched.
  - **Recipes:** a generated pose names its pack file by CRC-32.
  - **Reviews:** a verdict on a generated pose holds only for the file it was given on.
  - **Sagat's crouch is approved:**
    - It is the spike's candidate, after two review rounds of touch-ups by hand: his own ROM eyepatch, partly behind his hand, with its strap in black; a second shorts stripe removed; the forearm streak; outline specks; a ground shadow.
    - The spike's palette had no black (pen 0), so use `sf-art palette` and `sf-art reference` for every new generation.
  - **Ledger:** `art/generations/<fighter>.json` records every PixelLab call; `sf-art budget` totals it, and a test enforces the 1,800-a-month cap. This month: 20 used, all from the spike, and PixelLab agrees.
  - **Suite:** 610 passed, 0 failed, 1 ignored.
- **FE9 `#159` closed (`a4c8830`):** a real DualShock 4 over USB drove the menus and a fight, and survived a pulled cable. Smoke had said `none connected` because Windows Gaming Input reports plugged-in pads a moment after it starts (41 ms). `PadReader::open` now waits up to 250 ms for them, so smoke lists `PS4 Controller`. Two pads at once are FE13's (`#163`).
- **Push and close** are authorized for AP1-AP26. Fighter sets close only after the user's approval on the review page.

## Next work (as of AP5, superseded above)

- **Art pipeline frontier:** AP6 `#170` (Ryu and Ken's throws, thrown sets and get-ups; add art-upgrade rows for stand-ins). Then AP7-AP8, Sagat's pilot.

## Session close-out notes (2026-11-02, FE7–FE12)

- **Closed and pushed:**
  - FE7 `#157` (`fce1b38`, pause box and Training);
  - FE8 `#158` and FE10 `#160` (`3eae748`, Settings and persistence; the faithful 1987 cabinet);
  - FE11 `#161` (`530917f`: the ROM check, the ROM-missing screen, the log and the first launch; the log's pad lines came with FE9);
  - FE12 `#162` (`5bb3d34`, acceptance walk, audit note `docs/planning/front-end-audit.md`, README).
  The Lee/Birdie and Gen/Eagle roster fix was merged (`3d828cf`).
- **FE9 `#159` (pads and rebinding, `8ba5c4f`) is committed but left open.** Everything is built and tested on synthetic pad frames:
  - `gilrs` pads;
  - the `AUTO` / `KEYBOARD` / `PAD 1` / `PAD 2` device rows;
  - the rebind walks;
  - the extended `bindings.toml`;
  - the disconnect pause;
  - pads in the launcher.
  The one criterion not met is a real DualShock 4 enumerating, because no pad was connected to this machine (the smoke check printed `pads: none connected`). To check it, plug in a pad and run `cargo run -p sf-game --release -- smoke`, which lists the pads. `#163` is the person's full session with two pads.
- **Fixed during FE9:** a pad's Circle (menu back) is MK in a fight, and it was opening the pause box. Only a pad's Options opens it now, and `pad_play.rs` checks every attack button on both pads.
- **Decisions recorded** in design section 10 (FE8, FE10, FE11, FE9):
  - `AUTO` is connection order: the keyboard until the side's pad is seen, then idle while that pad is lost.
  - A key is refused in the rebind walk when anything else on the keyboard has it. A pad button is refused only within the walk's own list.
  - A pad's Options is its side's cabinet Start, and Share its coin.
  - A settings file that fails to parse is logged and left on disk.
- **Deferred, in the audit note:**
  - "last menu positions" (spec story 69): no ticket asked for them, and menus open on their first row;
  - online never pausing on a disconnect: recorded for netplay;
  - the window-close quit: no automated test.

## Next work (front end)

- `#159`: confirm a real pad enumerates, then close. `#163` is for a person.
- Leave `#91`, `#113` and `#150` open. After `#163`, the spec's next project in the user's order is the art pipeline.

## Earlier close-out notes (2026-11-02, FE1–FE6; FE7 and the roster fix landed since)

- **Closed and pushed:**
  - FE1 `#151` (`e36ebaf`, `sf-host` crate);
  - FE3 `#152` (`7c91f72`, boss ladder report);
  - FE2 `#153` (`109bcde`, `sf-game` binary, attract, Main Menu);
  - FE4 `#154` (`9fbc367`, Local into a real fight);
  - FE6 `#156` (`6ec9640`, Arcade);
  - FE5 `#155` (`a3f261a`, portrait select with the ROM's VS busts).
  The user extended push and close authorization to FE1–FE12.
- **Decisions recorded** in design section 10:
  - Each fight powers on its own machine, because boot decodes DSW2 once.
  - Local uses the `coin1`, `coin2`, `start2` start.
  - The select follows the user's direction: framed in the menus' style, every bust drawn 1:1 (integer scaling only), all busts facing one way, and Local laid out like the VS screen with both busts.
- **FE7 `#157` (pause box and Training) is in progress and uncommitted in this checkout.** Done so far:
  - `front/pause.rs`.
  - Training's `OPPONENT: CPU / DUMMY` menu and sequential select.
  - `FightKind`, `OverlayMode` and round-start snapshots in `front/fight.rs`.
  - `render_diagnostic_overlay_parts` in `overlay.rs`.
  - `match_over` in `tasks.rs` / `run.rs`, which replaces `versus_match_over`.
  - Session wiring for pause and Training.
  It builds clean, and the pause box and Training select render as intended. **Not done:**
  - the FE7 acceptance tests (`front_end_training.rs`);
  - a run of the existing `front_end_*` tests after these edits (`front_end_menus` was updated for TRAINING but not re-run);
  - regenerating the coverage ledger (`python tools/reference/coverage_ledger.py`), since `tasks.rs` lines moved.
- **Training against a CPU** can offer only the opponents a flag brings first, because the 1987 one-player game picks the CPU by country. Those are move tables `0x16e6c`, `0x175bc`, `0x1734c`, `0x17a9c` for flags 0–3, the same on every difficulty. The rest are greyed with `SF1+`.
- **Roster swap.** `roster.rs` swaps Lee with Birdie and Gen with Eagle (found by the bust decoding). A separate session, "Fix the Lee/Birdie and Gen/Eagle roster swap", is fixing it on its own worktree branch. Merge it when done.


## Completed Previously (2026-11-02, overhaul front end design)

- **Review of the Cursor batch.** Only `O1`-`O7` (overlay, replay takeover, core toggles, twelve-fighter binding) act on the real 1987 fight. `O8`-`O21` and `M1`-`M8` run on a standalone combat model (`sf1plus.rs`, `era.rs`, `sf2`-`sf6`, `alpha`, `flagship`, `what_if`, `bonus*`, `arcade`, `dramatic`, `world_tour`, `utility`, `netplay`) that never touches work RAM; nothing in `sf-dev` reaches them; the README's overhaul stills come from the `readme_frames` example, not play. They stay in the tree as reference material.
- **Decisions (user, 2026-11-02):** overhaul mechanics must act on the real 1987 fighters in the real fight; missing boss art is generated from the ROM's own sprites; seven projects in order: front end, art pipeline, SF1+ on the real engine, era styles and gauges, endings/Dramatic Battle/World Tour, bonus challengers, netplay transport.
- **Front end designed, grilled, specified, ticketed.** Design: [`docs/design/2026-11-02-overhaul-front-end-design.md`](design/2026-11-02-overhaul-front-end-design.md) (approved; section 9 holds the three grilling rounds). Glossary: the Front end terms in `CONTEXT.md`. [ADR-0004](adr/0004-reach-fights-by-driving-the-faithful-flow.md) (drive the faithful flow) and [ADR-0005](adr/0005-game-binary-pure-screens-shared-host.md) (game binary, pure screens, shared host crate). Spec: [#150](https://github.com/geox123/sf1-port/issues/150) ([`docs/planning/front-end-spec.md`](planning/front-end-spec.md)), a sub-issue of `#91`. Tickets `FE1`-`FE13` = [#151-#163](https://github.com/geox123/sf1-port/issues/151) ([`docs/planning/front-end-tickets.md`](planning/front-end-tickets.md), [`front-end-publication.json`](planning/front-end-publication.json)), sub-issues of `#150` with native `blocked_by` edges; `#163` is `ready-for-human`.
- **Scope reminders.** Arcade is Ryu and Ken in this project; all twelve in Local and Training. No engine changes in the front end; the stand-in rule is recorded for the SF1+ project. Pads are DualShock 4 through `gilrs` (Windows Gaming Input backend), confirmed on real pads by `#159` and `#163`.
- **Do not close** `#91`, `#113` or `#150`; they are specs. Mockup session files live under the gitignored `.superpowers/` folder.
- **Push and close authorization** was given for the `#14`-`#40` and `#41`-`#90` batches. It has not been extended to `FE1`-`FE13`; ask before closing a front-end ticket.

## Next work (front end)

- The frontier is [#151](https://github.com/geox123/sf1-port/issues/151) (`FE1`, host crate prefactor) and [#152](https://github.com/geox123/sf1-port/issues/152) (`FE3`, boss ladder report), which have no blockers. [#153](https://github.com/geox123/sf1-port/issues/153) (`FE2`) waits on `#151`; everything else waits on `#154` (`FE4`).
- Leave `#91`, `#113` and `#150` open.

## Session close-out notes

- **Overhaul tickets are on `main` (`cbf7250`).** O7–O21 (`#98`–`#112`) and M1–M8 (`#114`–`#121`) are closed. The commit covers SF1+ through SF6, what-if styles, World Tour, Dramatic Battle, rollback, and the thirteen bonus challengers, with tests and README stills.
- **Parent specs stay open.** The only open issues are [#91](https://github.com/geox123/sf1-port/issues/91) and [#113](https://github.com/geox123/sf1-port/issues/113). They are the specs, not implementation tickets. Do not close them.
- **Portrait select (`caa4eb3`).** `cargo run -p sf-dev --release -- select`. Three columns, large frame on the left, yellow cursor. Up and down move in a column. Left and right move along the row. Enter confirms. Backspace steps back. Power-on is still the 1987 country select. A tile with no bust shows the fighter's name. `portrait_select` and `portrait_grid` passed before this commit.
- **Layout reference.** [`docs/media/player-select-mockup.png`](media/player-select-mockup.png) is the chosen mockup: large bust left, twelve tiles right, yellow cursor, gold frame. The faces in that file are painted. They are not ROM crops.
- **VS busts, local only.** The twelve busts are foreground tiles on the country VS screen (event 3). Crops come from `tools/player_select.py`, which reads gitignored captures and writes `target/select-busts/*.raw` and `target/player-select.png`. `sf-dev select` does not load those files yet.
- **Do not change** `fighter.rs` `button_latch` or the 68000 damage path. `Program::power_on` / `Program::new` start `MatchConfig::faithful()`. `MatchConfig::default()` is Definitive Overhaul.

## Next work (as of the earlier 2026-11-02 session, superseded above)

- Draw the local VS busts on `sf-dev select`. The crops are in `target/select-busts/` after `python tools/player_select.py`. Sagat, Birdie, Eagle, Adon, and Lee were checked as individual crops and look like the VS busts.
- Leave `#91` and `#113` open.
- Nothing else is unblocked. Do not invent a new ticket unless asked.

## Session close-out notes (2026-10-30)

- **Release name settled and pushed (`970c030`):** the faithful port is **Street Fighter: The World Tournament**, the base of an edition lineage whose prefixes stay reserved for the Overhaul editions. Recorded in the `README.md` heading and as the `CONTEXT.md` terms **The World Tournament** and **Edition lineage**. The `sf-dev` window titles deliberately still say `sf1-port dev`: they label development entry points, not the release.
- **Parallel sessions:** other sessions committed and pushed faithful arcade-flow work on `main` during this session (see *Landed on main* below). `main` is level with `origin/main`. Uncommitted edits under `crates/sf-audio`, `crates/sf-engine`, `crates/sf-compare`, `crates/sf-dev` and `crates/sf-video` in the working tree belong to in-flight work from another session and were left untouched.
- **GitHub auth reminder for future sessions**: `gh` is at `C:\Program Files\GitHub CLI\gh.exe`. In PowerShell, set `$env:GH_TOKEN = & "C:\Program Files\GitHub CLI\gh.exe" auth token --user geox123` for `gh` operations, and push with:
  ```powershell
  $token = & "C:\Program Files\GitHub CLI\gh.exe" auth token --user geox123
  $b64 = [Convert]::ToBase64String([Text.Encoding]::ASCII.GetBytes("x-access-token:$token"))
  git -c "http.extraHeader=Authorization: Basic $b64" push origin main
  ```

## Completed (2026-10-30 — Release name)

- Brainstormed the release name with the `brainstorming` skill. The user chose, in order: a name that is the base of a Capcom-style edition lineage, a subtitle that points at the 1987 tournament, and **The World Tournament**.
- The lineage, now in `CONTEXT.md`: the faithful edition takes the plain name; a prime or plus (`Street Fighter'`, matching the `SF1+` era style) marks core toggles; *Super* adds bonus challengers (after *Super Street Fighter II: The New Challengers*); *Hyper* adds era styles (after *Hyper Street Fighter II*, which let each fighter use any revision).
- Rejected candidates, for the record: *Arcade Edition* and *Six-Button Edition* (fidelity claims), *Native Edition* and *Fighting Street* (rebuild claims), *The First Warrior*, *Round One*, *World Circuit*, *Five Nations*, *The Road to Thailand*.

## Landed on main since the 2026-10-28 handoff (faithful arcade-flow batch)

Taken from the commit subjects of `5132cbf..67bda64` and the GitHub issue state on 2026-10-30; not re-verified in this session.

- **Arcade flow:** attract wall, title and INSERT COIN; warning screen and boot-to-attract join; attract ranking screen; start screen, coins, service credits and 1P/2P start; fixed-fight pre-fight screens; CPU match win, ranking check and continue countdown; game over and return to attract; versus winners carried into the CPU fight; Geki translated to the bonus boundary.
- **DIP switches:** boot DIP decode from power-on; difficulty, round-time and attract-music DIPs; the Speed DIP's slow game; the Freeze DIP; flip screen on every layer, with flipped warning text verified.
- **Sound:** native-latch sound over every recorded fight; every service-test FM command; every tune through its loop or end; every ADPCM sample on both channels; the fixed-fight listening kit and live capture.
- **Verification:** Ryu's Hadoken and Tatsumaki, Ken's normal, crouching, jump and special attacks, and one-player crouch, jump, block and held inputs, all against the reference; recorded keyboard sessions compared with MAME; the translation coverage ledger (`#51`); the winning-script search tool.
- **Tickets:** `#41`–`#67` are closed. `#68`–`#90` are open; `#70` and `#71` are `ready-for-human`, the rest `ready-for-agent`.

## Completed Previously (2026-10-28 — M.U.G.E.N Research & Roster/Style Expansion)

- **M.U.G.E.N & Ikemen GO Community Research:** Published [`docs/research/mugen-sf1-style-characters.md`](research/mugen-sf1-style-characters.md), cataloging:
  1. Authentic 1987 arcade-sprite conversions of all 12 **tournament fighters** (**Falchion22**, **HelloMyNameIsAAA**, **Electro**'s *Ultimate Ryu SF1* [2024] & *Ultimate Adon* [2025], **DCelso**, **The Valkyrie Project**'s *Street Fighter One*, and **Pilgrim**'s Ikemen GO *SF1 Remake*) and how they solved the 10 bosses' missing crouch/jump/block/thrown animations.
  2. 1987-style demakes of lore characters (**Falchion22**'s Cody, **Legorulez49**'s Charlie Nash, **Jessica Smoke**'s Dan, **Mugenmonkey**'s Gouken & Gou Hibiki, **M**'s Evil Ryu).
  3. Multi-era movesets and pose blueprints for the "Forgotten Five" (**Retsu**, **Geki**, **Joe**, **Mike**, **Lee**) by **David_Bazbar**, **Rabano**, **Trololo**, **R@CE AKIR@**, **Zero de Armentis**, **Duracelleur**, and **0megaturb0**, plus retro-1987 variants (`BIRDIE_SFONE`, 1987 Gen, 1987 unscarred Sagat).
  4. Technical mapping from Elecbyte `.def`/`.sff`/`.act`/`.air`/`.cns`/`.cmd` and Ikemen GO `.zss` to `sf1-port`'s Two-Tier Hybrid Architecture.
- **Expansion Design & Builder Review:** Authored, builder-checked, grilled, and approved [`docs/design/2026-10-28-mugen-integration-and-roster-expansion-design.md`](design/2026-10-28-mugen-integration-and-roster-expansion-design.md).
- **Domain Model & ADR-0003:** Updated [`CONTEXT.md`](../CONTEXT.md) with **Tournament fighter**, **Bonus challenger**, and **What-if style** (and updated **Overhaul**), and recorded [`docs/adr/0003-offline-mugen-extraction-and-pose-synthesis.md`](adr/0003-offline-mugen-extraction-and-pose-synthesis.md).
- **M.U.G.E.N Expansion Specification ([Issue #113](https://github.com/geox123/sf1-port/issues/113)):** Published the formal specification to GitHub Issue `#113` (`ready-for-agent`, extending `#91`) and saved [`docs/planning/mugen-expansion-spec.md`](planning/mugen-expansion-spec.md).
- **M.U.G.E.N Expansion Ticket Batch `M1`–`M8` ([Issues #114–#121](https://github.com/geox123/sf1-port/issues)):** Published 8 vertical tracer-bullet tickets (`#114`–`#121`) with `ready-for-agent` labels, sub-issue/parent links to `#113`, and native GitHub `blocked_by` edges connecting to both `M1`–`M8` and the core overhaul batch `O7`–`O21` (`#98`–`#112`). Recorded in [`docs/planning/mugen-expansion-tickets.md`](planning/mugen-expansion-tickets.md) and [`docs/planning/mugen-expansion-publication.json`](planning/mugen-expansion-publication.json).

## Completed Previously (2026-10-27 — Core Overhaul Design & Tickets)

- **Community & ROM Research (`27ddf57`):** [`docs/research/community-expectations-and-criticisms.md`](research/community-expectations-and-criticisms.md).
- **Definitive Overhaul Design & ADR-0002 (`3467e20`):** [`docs/design/2026-10-27-definitive-overhaul-design.md`](design/2026-10-27-definitive-overhaul-design.md) and [`docs/adr/0002-two-tier-overhaul-architecture.md`](adr/0002-two-tier-overhaul-architecture.md).
- **Overhaul Specification & Tickets `O1`–`O21` (`65d7d69`, `cdc42be`, [Issues #91–#112](https://github.com/geox123/sf1-port/issues/91)):** [`docs/planning/overhaul-spec.md`](planning/overhaul-spec.md), [`docs/planning/overhaul-tickets.md`](planning/overhaul-tickets.md), and [`docs/planning/overhaul-publication.json`](planning/overhaul-publication.json).

## Next work (as of 2026-10-30, superseded by the 2026-11-02 notes above)

- **Faithful Arcade-Flow Batch (`#41`–`#90`, parent `#1`):** the frontier was `#68` onward. `#70` (listening review) and `#71` (keyboard session of both fixtures) waited on a person; `#90` (`F78`) re-checks and publishes waves 3–16.
- **Non-Invasive Diagnostic Overlays & Replay Takeover (`#92` `O1` -> `#93` `O2`, parent `#91`):** `#92` had zero blockers then.
- **Core Overhaul Batch (`#94` `O3`–`#112` `O21`, parent `#91`) & M.U.G.E.N Expansion Batch (`#114` `M1`–`#121` `M8`, parent `#113`):** these implementation tickets were later closed. See the 2026-11-02 notes. When these editions are named, use the reserved prefixes from the `CONTEXT.md` **Edition lineage** entry rather than inventing new ones.
