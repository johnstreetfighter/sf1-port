# Test suite speed: final recommendation

Everything below comes from reading logs, source files and directory sizes. Nothing new was built or run for this report. **Measured** means read straight from a log or a file listing. **Estimate** means modelled, and none of the estimates has been checked by a run.

## 1. Where the time goes

### Today (captures missing)

**Measured:**
- The latest full run (suite2.log, 2026-11-19 17:49) ran 321 test binaries.
  - Their "finished in" times add up to 3,783 s, about 63 minutes.
  - On quieter runs the same sum is 3,078 to 3,105 s (suite-219.log, suite4.log). The heaviest logged run reached 4,535 s.
- Compile time ranges from 1m12s to 12m08s. The median incremental build is about 3m23s, so compiling is roughly 3 to 15% of a run.
- sf-engine accounts for 92% of the test time.
  - The 74 binaries that take 10 s or more make up 3,530 s.
  - The 192 binaries under 1 s make up 4.6 s in total.
  - The slowest binaries in suite2.log:

| Binary | Time in suite2.log | Range across runs |
|---|---|---|
| fair_cpu | 431 s | 344 to 631 s |
| held_jump | 156 s | |
| arcade_mode | 153 s | |
| joe_player | 117 s | |
| net_relay_rooms | 107 s | |
| net_sessions | 105 s | |

**Why it takes so long:**
- cargo test runs the binaries one after another. Inside a binary, libtest runs tests in parallel, so each binary takes as long as its slowest test.
- Most heavy binaries are one long test. Each of those tests loops through independent cases one at a time, for example fair_cpu.rs:453-499 and held_jump.rs:221-281.
- So most of the hour runs only about 2 of the 16 hardware threads (estimate). A disturbed run on 11-20 measured 1.4 to 2.2 busy threads per binary.
- The two network binaries, net_relay_rooms and net_sessions, spend most of their roughly 210 s sleeping in real time (net_relay_rooms.rs:228, net_sessions.rs:259).
- The run-to-run spread mostly comes from other heavy work on the machine: fair-CPU sweeps, release builds and MAME regeneration. Two suites overlapping happened in only 4 of 18 logs, and in none of those 4 did the overlap measurably slow either suite.

### After the ~430 capture-gated tests are restored (estimate)

- In the logs from 11-07 and 11-08, when the captures still existed (release profile), the 64-65 sf-engine binaries that now take about 0 s added up to **1,929-3,232 s (measured)**.
  - dip_switches: 257 to 647 s
  - full_1p: 130 to 260 s
  - power_on_boot: 57 to 180 s
  - one_player_round3: 163 to 201 s
  - fixed_fight_dips: 170 s
- Those logs do not cover the capture tests in sf-audio, sf-video and sf-dev, so the real figure is higher.
- **Estimate:** the sequential test phase grows to about **87 to 115 minutes**. The skeptics' estimates fall between 87 and 117 minutes. Compile time does not change.
- The restored run in progress (suite-restored1.log) already shows capture binaries that took 0 s before now taking 30 to 85 s each:
  - adon: 32.8 s
  - birdie: 57.4 s
  - bonus_stage1: 71.7 s
  - blocking_and_shoto_symmetry_toggles: 84.6 s
- That run is not a valid baseline (measured): it overlapped the MAME regeneration and has 104 abnormal exits.
  - Exit codes: 74 × 0xc0000409 (allocation failure), 12 × 0xc000012d (commit limit), 10 × 0xc00000fd (stack overflow), 7 × 0xc0000142 (DLL init failed).
  - The pagefile is fixed at about 32 GB and its peak usage has reached the cap.
  - **Do not run a suite while MAME regeneration is running.** If a run shows those exit codes, treat it as an environment failure, not a regression.

### Two corrections to the brief (measured)

- D: has about 57 GB free, not 15 GB.
- C: is the drive under pressure right now: it fell from about 108 GB to 77 GB free as captures regenerate.

## 2. Changes worth making, ranked

I left out any proposal that both skeptics rejected; those are listed at the end of this section. **[DISAGREED]** marks proposals where the skeptics split.

### 1. Run test binaries concurrently [DISAGREED on the nextest form; both skeptics accepted the no-install runner]

**The change.** Use either of these:
- **cargo-nextest**, behind a new alias, for example `nextest run --cargo-profile suite --workspace --locked --no-fail-fast --build-jobs 4`.
  - Use `--build-jobs 4`, not `-j 4`: in nextest, `-j` means test threads.
- **A small runner script.** It runs `cargo test --profile suite --workspace --locked --no-run --message-format=json`, then runs the test executables on 4 lanes, longest first.

Keep `cargo suite` as the gate until the new runner matches it.

**Saving (corrected, estimate).**
- Today: the test phase drops from 51-63 minutes to about 14-28 minutes, saving about 25-45 minutes.
- After the captures are restored: about 21-45 minutes instead of about 87-115, saving about 55-75 minutes.
- The reviewers' 10-15 minute figure is not reachable. Two things set a floor:
  - fair_cpu's walking test takes about 8-10 minutes when every thread is busy.
  - The machine gives about 8-10 cores' worth of throughput, not 16.
- Six lanes are no faster than four.

**Risks and required conditions.**
- **Timing-sensitive tests.** Several tests assert wall-clock limits (under 100 ms, or 2-20 s deadlines):
  - sf-host: net_directory, net_driver, net_relay, net_relay_rooms, net_sessions
  - sf-relay: service, rooms

  A nextest test group caps concurrency but does not shield these tests from CPU load. Run them in a separate serial phase afterwards; this adds about 3-7 minutes. They have failed under load before: suite-127.log:4057 recorded a 204 ms ping, after which commit 1787394 dropped that bound.
- **GPU tests.** The sf-host lib GPU tests already fail on a known flake (FXC error, 3 of 15 logs). Run them one at a time.
- **A real race once captures return.** Two tests write the same WAV files into tools/reference/local/renders:
  - the unit test in crates/sf-dev/src/main.rs:2402-2423
  - both_chips.rs `fixed_fight_listening_kit_renders_and_live_capture` (1382-1487)

  Run them one at a time, or better, have both write to a ScratchDir.
- **Capture concurrency.** Do not cap capture tests at 4 threads. That cap alone sets an 18-22 minute floor, and there is no memory reason for it. Use 8 or more threads, or no cap.
- **Order.** Start the longest tests first: fair_cpu, dip_switches, fixed_fight_dips, held_jump, full_1p, arcade_mode.
- **Runner script specifics.**
  - Run only artifacts with `profile.test == true`, so the examples and GUI bins never run.
  - Use the package directory as the working directory.
  - Do not set RUST_TEST_THREADS to 16 divided by the lane count; that serializes fair_cpu's 6 long tests.
  - Add a doctest step. There are 0 doctests today, but the step keeps future ones covered.
  - Treat a crash exit as a failure.
- **Fixture rebuilds.** Under nextest, each test rebuilds the per-process OnceLock fixtures, for example np5_sessions.rs:100 and both_chips.rs:92. Estimated cost: about 1-2 minutes of CPU.
- **Memory.** Peak RAM with 16 mixed processes is unmeasured. Make the first run with 8 threads, run nothing else at the same time, and watch commit headroom.
- **Validation.** Compare pass/fail test by test, not binary by binary. Do it only after the captures are restored, since about 430 tests skip silently today. Make 2-3 runs.

**Effort.** Small for nextest; medium for the script. **Coverage:** unchanged.

### 2. Run the serial sweeps inside fair_cpu, held_jump and era_cross on threads [both skeptics held]

**The change.** Use `std::thread::scope`, the pattern fair_cpu.rs:868 already uses. Keep the same seeds, inputs and assertions.
- **fair_cpu:**
  - the 12 walking cases (446-499)
  - the reaction levels (651-697)
  - the partner runs in measure_level (851-859)
  - ideally also the three independent phases of the modern-rules test (955-1055), which otherwise becomes the new critical path
- **held_jump:** each (fighter, side) pair in check().
- **era_cross:** clone the session per style.

**Saving (estimate).** About 5-8 minutes per serial `cargo suite` run on an otherwise idle machine:

| Binary | Now | After (estimate) |
|---|---|---|
| fair_cpu | 431 s | 150-275 s |
| held_jump | 156 s | 35-60 s |
| era_cross | 91 s | 10-30 s |

- Restoring the captures does not affect these binaries.
- Under a parallel runner (item 1) the gain mostly disappears, because this change adds no CPU capacity. Do not count both savings.

**Conditions.**
- Keep fair_cpu.rs:658's four classic levels together in one test, because the asserts at 682-683 compare them with each other.
- era_cross needs a new helper in support/alpha.rs that stops before P1's lock_style. Cloning at `local_select_with_fighter` would give every run the same style.
- Join results in input order and re-raise failures with `resume_unwind`, so failure messages stay the same.
- Check once with plain `cargo test` (debug build), per AGENTS.md.

**Effort.** Small to medium. **Coverage:** unchanged.

Splitting these cases into separate `#[test]` functions was rejected by both skeptics, because it breaks the cross-level asserts at fair_cpu.rs:682-683.

### 3. Clear stale build artifacts [both skeptics held; disk only]

**Saving (measured sizes).**
- **D:** up to about 80-86 GB from original-target.
  - Deleting target/debug outright frees about 93 GB; it gets rebuilt on next use.
  - About 29.7 GB more sits in hud-catchup-target, playtest-notes-target, linux-build-target and combo-clips-target.
- **C:** about 3.7 GB now (the two agent-* worktree targets plus target-check). sc13's 11 GB becomes available once that lane is finished.
- **Test time:** unchanged.

**Conditions.**
- Only do it when no cargo process from any session is running. debug, suite and release were all written to between 07:29 and 07:46 today, and sc13 was building at 08:20.
- Delete whole profile directories or use a careful sweep. Do not apply a raw "older than 3 days" rule to deps: it also deletes about 15 GB of current artifacts.
- If you use cargo-sweep, check how it decides a file is old. Last-access times are turned off on this machine.
- Never delete `<profile>/shaders` on its own; vendored_shaders.rs:138 would then fail. Also keep original-target\playtests, tmp and the root .ppm files, which cannot be rebuilt.
- hud-catchup and linux-build are still registered worktrees. Confirm those lanes are done, and unlink their junctions before removing them.

**Effort.** Small. **Coverage:** none.

### 4. Turn off incremental compilation for the suite profile [both skeptics held]

**The change.** Add `incremental = false` to `[profile.suite]` in the root Cargo.toml (lines 20-23). Then delete target\suite\incremental once.

**Saving.**
- About 6-6.4 GB steady state (measured by date).
- It also stops a new set of about 6 GB appearing with every version bump or lockfile change.
- Build time is unmeasured and could go either way by a few minutes. Single-file runs on the suite profile, which are the project's quick path, will likely rebuild more slowly.

**Conditions.**
- Time one sf-engine edit plus a single-file rebuild with incremental on and off before keeping it.
- Delete the directory only when no cargo process is running.
- Run one full suite afterwards to confirm the baseline holds, since inlining changes can affect stack depth.

**Effort.** Small. **Coverage:** none.

### 5. Recompress finished captures with WOF LZX [DISAGREED; disk only, low priority]

- **Saving (estimate):** about 3.4-5.6 GB on C:. Only the .rgba files can shrink; the PNGs and the one-cluster RAM dumps cannot.
- **Speed:** probably no gain, possibly slightly slower.
- **Before adopting:** try it on win-10-run-a first. If the files are already LZNT1-compressed, `compact /u` may be needed first, which briefly inflates the directory about 4×. Re-check sha256 against the D: manifests afterwards.
- **Timing:** never while regeneration or a suite is running.
- **Effort:** small. **Coverage:** none.
- Drop the optional "pack the RAM dumps" step. It is a format change touching 66 files plus the manifests.

### 6. Merge integration test files into a few binaries per crate [DISAGREED; defer]

- **Saving (estimate):** about 1-1.5 minutes per incremental build and about 4-5 GB of disk per suite generation.
- **Run time:** the run-time gain is real, but item 1 delivers it better without restructuring about 300 files.
- **Conditions if it is ever done:**
  - Keep the sf-host net_* files and np5_sessions.rs (known debug stack overflow, #191) as separate binaries.
  - First change the baseline and the push/close rule from failing files to failing test names.
  - Avoid filter collisions such as `gen::` also matching `alpha_gen::`.
  - Fix crates/sf-game/src/evil_ryu_vs.rs:20-24 and the relative include_str! paths.
- **Effort:** large. **Coverage:** none.

### Left out: both skeptics rejected

| Proposal | Reason |
|---|---|
| Native checkpoint filter | Saves about 15-60 s, and breaks power_on_boot.rs:756 |
| Skipping drawing in logic-only tests (both versions) | Unmeasured; crashes after-match tests; silently weakens determinism hashes; drops overhaul-render coverage |
| Lock so only one suite runs at a time | No measured speed gain (see the note below for a narrower version) |
| rust-lld linker | Saves about 25-45 s per build; costs a full rebuild on every switch |
| `-j 8` | 0-90 s saved; commit-limit risk on this machine |
| `strip = "debuginfo"` | Measured to do nothing on rustc 1.96 / MSVC |
| `--tests`-only builds | Net saving about 0 s; drops mugen-extract from the build entirely |
| Pruning target/suite by the reviewers' rule | Deletes current artifacts; item 3 covers the cleanup |
| Moving MAME fixture audits out of every run | Saves about 0 s; the pure audits are never on the critical path |
| Dropping native pixel files and merging dip_switches passes | Breaks 7+ tests; the fold loses the frame-5656 check |
| `slow_` fast-tier alias | Substring matches hit coach.rs:1208 and sagat.rs:307; saves only 4-8 minutes now |
| Replacing run-b's files with hashes | `compare()` would silently skip RAM; saves about 0-45 s |
| PNG/rgba swap | Option B touches 48 files |
| Not writing native rgba to %TEMP% | Breaks the 7 bad_crc tests |

Smaller leads the skeptics raised. None has been reviewed or measured:
- **Delete the duplicate A/B call at dip_switches.rs:1919.** One skeptic says fixed_fight_dips.rs, fixed_fight_slow.rs and freeze.rs already check those pairs byte for byte. Estimated saving 1.5-4 minutes once captures are back. Confirm the duplication before deleting.
- **draw_tile_layer (render.rs:318-363) does a SipHash HashMap lookup for every pixel.** A tile-indexed cache could speed up every rendering test with no coverage change. Profile first.
- **`doctest = false` on the 8 libraries plus `test = false` on the empty bin harnesses** (but not mugen-extract). Estimated 6-14 s.
- **A lock on target\suite in the main checkout** (narrower than the rejected lock). It is not a speed fix: it prevents the measured LNK1104 link abort when two suites share target/suite (9cd8cdd0 suite2.log line 12). If you add it, use an OS-held file lock on C:, not a marker file.

## 3. Recommended order

1. **Now, no code.** Don't run suites alongside MAME regeneration or another heavy job. Treat the 0xc000012d, 0xc0000409 and 0xc0000142 exit codes as environment failures.
2. **Item 3, artifact cleanup**, after George approves and while no cargo process is running. It frees a lot of disk immediately.
3. **Item 4, `incremental = false`**, with the one-edit timing check.
4. **Item 2, thread::scope in fair_cpu, held_jump and era_cross.** A small code change, worth about 5-8 minutes on every serial run.
5. **Once captures are fully restored, take one clean `cargo suite` baseline** on an idle machine. This replaces the estimates above with measured capture costs and gives the per-binary times for longest-first ordering.
6. **Item 1, the parallel runner.** Use nextest if George approves the install, otherwise the runner script. Put the timing-sensitive network tests in their own serial phase, fix the renders-folder race first, and validate test by test over 2-3 runs. This is the largest win, about 55-75 minutes per run after the restore.
7. **Item 5, LZX**, trialled on one capture if C: stays tight.
8. **Item 6, merging test files**, only if build time or disk still matters after item 1.

## 4. Decisions for the maintainer

- **Install cargo-nextest.** It is a download. Without it, item 1 falls back to the runner script.
- **Delete build artifacts** (item 3). The memory rule requires asking first. Also confirm whether the hud-catchup, linux-build, playtest-notes, combo-clips and sc13 lanes are finished.
- **Timing-sensitive tests under a parallel runner.** Either run them in a separate serial phase (recommended; costs about 3-7 minutes), or accept a higher risk of flakes.
- **What counts as the baseline** if item 1 or item 6 goes ahead: failing test names rather than failing files, plus updating the test note in AGENTS.md.
- **Coverage-reducing options.** None of the recommended items reduces coverage. Every proposal that would (a fast tier, moving fixture audits out, skipping drawing, thinning run-b, dropping PNGs) was rejected by both skeptics. If you still want one, the least risky is dropping the PNG previews: about 4 GB on C:. It removes the byte-compare of the PNG at full_1p.rs:138, which the rgba compare on the next line already covers, and the file counts at full_1p.rs:108 and full_2p.rs:87 change from ×6 to ×5. That is your call.
- **The 32 GB fixed pagefile** has been hit. Raising it is a system setting you would change yourself.

Scratch models and parsers cited above are in `C:\Users\George\AppData\Local\Temp\claude\371ab938eff6b48f\bd097673-ce38-40a4-8a33-e5cf3778fa75\scratchpad`.

---

# Completeness critic

Gaps and corrections in the speed recommendation, checked cheaply on 2026-11-20. Nothing was built or run.

1. **Memory is the main thing the report leaves out (measured).** The machine has 15.3 GB of RAM and 8 physical cores (16 threads, Ryzen 9 8945HS), read from Win32_ComputerSystem and Win32_Processor. With the 32 GB pagefile, the commit limit is about 47 GB.
   - Capture comparison reads each whole file into memory (`crates/sf-compare/src/compare.rs:353-354`, `std::fs::read`). Memory therefore grows with every capture test running at the same time.
   - The restored run already hit 12 × 0xc000012d (commit limit) and 74 × 0xc0000409.
   - So item 1's advice to run capture tests on "8 or more threads, or no cap", with "no memory reason" for a cap, has nothing measured behind it and goes against the only evidence there is. Peak memory per capture binary has to be measured before choosing a capture concurrency. The 21-45 minute post-restore figure depends on that number.

2. **Tests that skip quietly can make the parallel runner look valid when it is not (measured).** 65 test files return early with "skipping: local reference captures/ROMs not present", for example `crates/sf-audio/tests/both_chips.rs:134`. No environment switch makes a missing capture a failure (I grepped for `SF1_REQUIRE`/`REQUIRE_CAPTURES`/`env::var("SF1`: none).
   - The GPU tests also return early when there is no adapter (`crates/sf-host/src/look.rs:524-526`, callers at 617, 668, 703, 735, 804 and 866).
   - A test-by-test comparison between runners will still show green if captures are only partly restored. `captures-staging/` exists beside `captures/` today.
   - Fix: add a switch that turns these skips into failures, or count the "skipping" lines as part of the baseline.

3. **Item 2 is undersold, and the report contradicts itself.** It says item 2's gain "mostly disappears" under a parallel runner. It also says fair_cpu's walking test sets an 8-10 minute floor for item 1. Threading that sweep (`fair_cpu.rs:446-499`, loop at 467) is exactly what lowers that floor. So items 1 and 2 add up at the end of a run, when other lanes have finished. Item 2 also adds threads and memory per process, which ties back to point 1.

4. **Network race not listed.** `crates/sf-host/tests/net_driver.rs:497-499` picks a free port, drops it, then binds it again. It also sends a real broadcast to 255.255.255.255 (lines 508-511), and `crates/sf-host/src/net.rs:854` sets SO_REUSEADDR, which on Windows lets another process bind the same port. While other processes run, another test can take that port. The serial network phase covers this, but it is a second reason for that phase besides wall-clock limits.

5. **Temp-folder collisions checked: none found (measured).** Temp directories are named with the process ID plus the test name:
   - `crates/sf-host/tests/files_kept.rs:10`
   - `crates/sf-video/tests/romset_check.rs:28`
   - `crates/sf-art/tests/palettes.rs:161` and `:213`
   - `crates/sf-engine/tests/jukebox_settings.rs:31` and `:85`
   - `crates/sf-engine/tests/np5_sessions.rs:1520`
   - `crates/sf-dev/tests/combo_commands.rs:54`, which also adds a timestamp and a counter

   The renders race (`crates/sf-dev/src/main.rs:1617-1742`) is the only shared-path race found. The report cites `main.rs:2402-2423` for its unit test, so check that both line ranges point at the same code. Under nextest, every test is its own process, so leftover temp folders will build up faster (estimate).

6. **Windows cost per process not examined.** Defender real-time scanning is on (`DisableRealtimeMonitoring: False`). The exclusion list cannot be read without admin. nextest starts about 1,800 processes, and nobody has costed the scanning and process-start overhead on Windows. Also, nextest is not installed (`cargo nextest --version`: "no such command"), and there is no `.config/nextest.toml`.

7. **Drive space (measured with df).** D: has 57 GB free and C: has 75 GB, so the report's correction of the brief is right. C: has dropped again from the report's 77 GB. Captures are NTFS-compressed per memory and are read in full, so decompression costs CPU on every capture read. Nobody has measured that, and it bears on item 5 (LZX): a stronger compressor costs more to decompress.

8. **CI cannot take this load (measured).** No workflow runs cargo tests. The only test step is a Python unittest at `.github/workflows/linux-release.yml:51`. Offloading would need the ROMs and captures, so it is probably not an option, but the report never says so.

9. **The restored-run timings are skewed.** The 30-85 s capture binaries (for example blocking_and_shoto_symmetry_toggles at 84.6 s) come from a run that overlapped MAME regeneration. They should not be used to order lanes or to estimate per-capture cost.

10. **Debug stack check for item 2.** Threads created by `thread::scope` get the default 2 MiB stack. The repo already raises stack sizes in some tests (`measuring_run.rs:38`, `route_books.rs:32` use 256 MiB). Item 2 should state the stack size of any new worker threads, and the debug-build check that AGENTS.md asks for is required, not optional.