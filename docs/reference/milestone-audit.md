# The first playable milestone audit

This document records the audit asked for by
issue #39: the whole fixed
fight played natively, from the round-1 fixture through the match end,
compared end to end with the pinned reference, and a pass/fail account
against the specification's first playable milestone. The specification
defines that milestone as "one fixed fight, entered through a development
launch, with movement, attacks, opponent behavior, damage, exact game
pixels, and faithful sound", played with keyboard controls.

**Result.** The fixed fight plays natively from the round-1 fixture
(frame 2389) through round 1, round 2 and the match decision. After every
one of the 2,625 frames from 2390 through 5014 it matches the reference on
every 68000-side observable a capture records: the screen hash, the sound
latch and its changed flag, the full screen pixels, video and palette RAM,
and work RAM. The exception is the 68000 CPU state the translation does
not keep by design: the task slots' saved registers and the stack memory.
The music CPU's RAM, which the capture also dumps every frame, is compared
by a separate sound run below the Z80's stack, at the 1,370 of 2,610
frame ends of 2390–4999 where the music CPU is idle, and matches there. At
frame 5015 the fight reaches the CPU's match win (0x3eefa, the continue
screen), which is not translated, and stops there. Against the
milestone, the fixed fight passes on movement, attacks, opponent behavior,
damage and exact pixels as the fight is scripted. Sound passes on
measurement, with a reported latch-timing limit, but has had no listening
review. Keyboard play is built and smoke-checked, but no person playing it
has been recorded. The fixed fight's script gives P1 little to do: a walk,
a light punch and a light kick. Most of what a player can do at the
keyboard is verified only in other live-CPU fights or slices of them, or
in controlled fixtures. The milestone does not certify the complete port.

Code: `crates/sf-engine/tests/milestone_audit.rs`,
`sf_engine::run::observe_cpu_fight`, `cpu_work_image`,
`cpu_owned_addresses`, `sf_audio::check::music_ram_at_frame_ends`;
scenario `tools/reference/scenarios/sf_fixed_fight_audit_01.json`.

## The audit capture

The pinned baseline of the fixed fight, fight-run-a (sf-fixed-fight-01),
ends at frame 4999. In the span from the fixture it holds full checkpoints
at only 7 frames (2460, 2595, 2660, 3360, 3840, 4800, 4990), none inside
the fighting of round 2 or of round 1 after 2660. Round 2's result runs
to 5014 and the match decision follows it.

The audit capture **sf-fixed-fight-audit-01** therefore runs the same
inputs, machine and observation settings to frame 5040. It records a full
checkpoint after every frame from 2389: screen pixels and the work, video,
palette and music RAM. Two runs on the pinned MAME were made:

- the two runs' observations are byte-identical: all 13,263 frame,
  checkpoint, meta, pixel and RAM files match (the runner metadata —
  summary.json and the generated driver, which embeds the output path —
  and the PNG previews are not compared, as in mame-baseline.md);
- their frames.tsv rows through 4999 equal fight-run-a's on every field,
  and fight-run-a's checkpoint screens and RAM dumps in the span equal
  theirs;
- the round-1 fixture's capture, sf-cpu-01 (cpu-01-run-a), equals
  fight-run-a on every field of every frame through 2389, so its
  checkpoint is the fixed fight's state;
- fight-audio-a, the sound comparisons' reference run, equals fight-run-a
  on every field of every frame.

## The end-to-end comparison

The native one-player machine starts from checkpoint 2389 and plays the
scenario's inputs. Each row is compared after every frame from 2390
through 5014 (2,625 frames) unless the row gives a narrower scope:

| Observable | Result |
| --- | --- |
| Screen hash (every 4th pixel of every 4th row), sound latch, latch-changed flag | identical on every frame |
| Full screen pixels, video RAM, palette RAM | identical after every frame (2,625 checkpoints) |
| Work and object RAM the translated routines own (fighters, objects, AI tables, CPU and P1 structures, game state, round timer and result, random state, sound queue, camera, scheduler and task slots' state and sleep) | identical after every frame |
| The rest of work RAM (0xff8000–0xffffff) | differs after every frame, only in the task slots' saved SR / PC / USP words (859 byte-frames) and the stack memory 0xff8106–0xff8631 (204,786 byte-frames); no other byte differs on any frame |
| Work-RAM CRC (the reference's every 10 frames) | differs at all 263 points, which follows from the row above |
| Music CPU RAM below its stack (0xc000–0xc5cf), at the frame ends where the music CPU is idle (in its idle loop or in a handler's closing instructions, which store nothing), 2390–4999 | identical at all 1,370 idle frame ends, both at the reference's latch write times and from the native latch at latency 51 |
| Music-RAM CRC (the reference's every 10 frames), music RAM at the other frame ends, above the Z80 stack and after 4999 | not compared (the translation does not keep the Z80's stack; the sound run ends at 5000) |
| Untranslated branches | none through 5014; 0x3eefa (the CPU's match win, the continue screen) at 5015 |

The stack memory holds the supervisor stack under the reset SSP 0xff81a2
and the tasks' stacks, whose saved USPs lie at 0xff825e–0xff85de. The
translation keeps the tasks' continuations as Rust state instead of 68000
register images, and task-scheduler.md records that the saved words and
stack images are not reproduced.

**The negative control.** The same comparison run on a native fight whose
light punch comes ten frames late (2600 instead of 2590) reports a
mismatch at frame 2601, in the checkpoint screen pixels. That is the
reference's punch start, release plus one frame, as punch.md records.

**What happens in the fight.** The inputs come from the scenario; the
rest comes from the reference's dumps, and the native run's owned RAM
equals them on every frame:

- The script holds right over 2400–2579 and presses light punch at 2590
  and light kick at 2630, ten frames each. From 2640 P1 has no input.
- Ryu walks over 2401–2443, 2470–2532 and 2580–2596; Retsu's hits at 2444
  and 2533 interrupt the walk. The light punch (move 0x0d) runs from 2601,
  release + 1. Retsu hits Ryu at 2617, and the light kick (move 0x14)
  starts at 2662, after the hit stun.
- Retsu's health bytes never change.
- Ryu's health falls at 2444, 2533, 2617, 2846, 3077 and 3327, the last
  hit emptying it; the KO is detected at 3328.
- In round 2 it falls at 4278, 4346, 4440, 4567 and 4772; the KO is
  detected at 4773.
- The CPU wins 2–0.

**Sound.** The sound is a separate native run of the same fight
(both-chips.md): both sound programs from power-on on this run's latch.

- **Per device over round 2.** At any constant interrupt latency from 49
  to 53 cycles, the samples CPU's port writes are identical, and so are
  both MSM5205 levels at every channel tick and the YM2151's every output
  sample.
- **Exact timing.** With the reference's write times, every device is
  identical, its timing included.
- **The mix** measures like MAME's recording. The rest of the sound, the
  latency limit included, is in both-chips.md.

## The milestone, item by item

Verdicts:

- **Pass:** verified in the complete fixed fight played natively, on every
  observable.
- **Pass on measurement:** verified by the machine checks, with an
  assessment that needs a person still outstanding.
- **Partial:** built and verified in part, with the rest listed.

| Item | Verdict | In the fixed fight | Limits and gaps |
| --- | --- | --- | --- |
| **Development launch** | Pass | The launcher's one-player mode starts from the fixed fight's round-1 fixture (checkpoint 2389) and runs natively through the match decision. At 0x3eefa it reports the branch and reloads the fixture. | The fixture loads the reference's RAM dumps at 2389: the fight's start is a development fixture, and everything before 2389 (boot, attract, credit, country select, VS, ROUND 1) is the reference's state. The launch needs the local ROM set and four local captures. |
| **Movement** | Pass | P1's forward walk (2401–2596, interrupted by Retsu's hits at 2444 and 2533), Retsu's movement in both rounds, both fighters' positions and the stage scroll, after every frame. | The fixed fight has no crouch, jump, back walk or turn by P1 (table below). |
| **Attacks** | Pass | P1's light punch (from 2601) and light kick (from 2662, after hit stun): their frames, animation and sounds, after every frame. Neither damages Retsu. | Medium and heavy strengths, crouching and jumping attacks and special moves are not in the fixed fight (table below). |
| **Opponent behavior** | Pass | Retsu over the whole fight: his decisions, moves, attacks, hits and both round wins, with the AI tables and the CPU structure equal after every frame. | Six of Retsu's seven attacks (0x0d, 0x0f, 0x14, 0x15, 0x23, 0x24) are in the fixed fight; the seventh, the unblockable 0x18 (stream 0xcf1a), is not, and is verified only in sf-round2-02 (cpu-follow-up.md). Only the default DIP settings are captured, and difficulty changes Retsu's tables. The other opponents are later work: the Geki fight stops at his hit function 0x4656e. |
| **Damage** | Pass | Retsu's eleven hits on Ryu (two of them knock-outs), Ryu's vitality and health bars, both KOs, both round results, the round transition and the match decision, after every frame. | Damage dealt by P1 to the CPU is not in the fixed fight (table below). Neither are time-over, draw and double KO. |
| **Exact game pixels** | Pass | Every frame's full screen 2390–5014. | The launcher's display path, a 3x upscale presented after each step, is not compared, and neither is host 60 fps pacing. Flip-screen tilemaps are not covered. |
| **Faithful sound** | Pass on measurement | Both sound programs from the native latch: the per-device identity at the fitted latency, the music CPU's RAM at every idle frame end, and the mix against MAME's recording (both-chips.md). | No person has listened to the native mix or to the launcher's live output. The launcher runs at the latency band's middle (92), where this fight's samples CPU differs from frame 2619 on and 0.43% of round 2's YM2151 samples differ. The latency is a disclosed band, not modelled (latch-timing.md). The sound is MAME 0.289's scheduling (below). |
| **Keyboard** | Partial | — | The launcher plays both fixtures with fixed bindings, and a unit test pins every key to its documented input. The smoke check opens the window and audio, checks the bindings and drives both fixtures through scripted play on translated paths with non-silent output. The key reader itself (window focus, key state) is not exercised by any automated check. No session of a person playing at the keyboard is recorded, and interactive play is not a verification (launcher.md). |

**Beyond the fixed fight.** With the keyboard a player can do far more than
the fixed fight's script. That breadth is verified in three classes of
evidence. The specification excludes controlled fixtures, where the
opponent is untouched or scripted, as milestone evidence. They are listed
for what they verify.

| What a player can do | In the fixed fight | Against the live CPU in another fight (sf-win-01 a whole match; sf-retsu-01, -02 round-1 slices, 2389–3400) | Only in a controlled versus fixture |
| --- | --- | --- | --- |
| Walk forward | yes | sf-retsu-02, sf-win-01 | sf-move-01 |
| Walk back, the stage edge, the forward pin | — | — | sf-move-01 |
| Crouch | — | down held only inside the Shoryuken command (sf-retsu-02, sf-win-01) | sf-crouch-01 |
| Jump, neutral and diagonal | — | — | sf-jump-01, sf-jump-02 |
| Turn to face the other side | — | — | sf-jump-02 (2861) |
| Standing normals, every strength | light punch and kick | all six buttons (sf-retsu-01, a round-1 slice with no KO) | sf-normals-01..05, sf-punch-01, sf-swap-01 |
| Crouching normals | — | — | sf-crouch-attacks-01..05 |
| Jump attacks | — | — | sf-jump-attacks-01..07 |
| Shoryuken | — | heavy, every 60 frames (sf-retsu-02, sf-win-01) | sf-specials-01, -02 |
| Hadoken | — | — | sf-hadoken-01..03; its one-player tables (0x4340a, 0x434d0) are uncovered |
| Tatsumaki | — | — | sf-specials-03, -04; its one-player chip table (0x45fa8) is uncovered |
| Block | — | — (Retsu's blocks: sf-win-01, which shares sf-retsu-02's inputs and reference rows through 3289; sf-retsu-01) | sf-blocks-01..05 |
| Damage the CPU, knock it out, win a round and the match | — | sf-retsu-02 (KO at 3018), sf-win-01 (match win, then Geki's stage) | — |
| Time-over, draw, double KO | — | — | sf-timeover-01, sf-doubleko-01; the one-player draw goes to 0x3eefa |
| Play as P2 | — | — | the versus fixtures (P2 scripted); the P2-versus-CPU mode is not exercised |

Paths a player can reach in the launcher that no scenario pins run on the
same translated routines, unverified until a scenario covers them. They
include:

- both horizontal or both vertical directions held;
- a one-player round 3 after a 1–1 split;
- a one-player time-over, draw or double KO;
- a Hadoken or Tatsumaki against Retsu;
- a button held past 255 frames;
- frame overruns.

No coin, start or service key is bound, so the credit and challenger paths
cannot be reached.

## What the milestone does and does not demonstrate

It demonstrates the following:

- **One complete fight.** The fixed fight, entered from a development
  fixture, is played by translated game logic and rendered from decoded
  ROM graphics. It matches the reference after every frame through the
  match decision on every recorded 68000-side observable except the CPU
  state the translation does not keep; the music CPU's RAM is compared
  only at its idle frame ends through 4999 (above).
- **Both sound programs.** They are translated and driven by that fight's
  latch, and they reproduce the reference device by device within a
  disclosed latch-timing limit.
- **A keyboard launcher.** It plays it.

It does not demonstrate the following:

- **The arcade flow before 2389:** boot, attract, credit and start,
  country select, VS and the round announcement. This includes the random
  state from power-on.
- **What follows the match:** the continue screen, the lose quote and game
  over (0x3eefa).
- **The rest of the game:** the other opponents, the bonus stages, the
  ending and name entry.
- **Other settings and features:** DIP variants other than the defaults,
  flip-screen tilemaps, keyboard remapping, the full sound set and the
  round tune's loop point, and test mode.
- **A person's assessment:** the listening review and a recorded session
  at the keyboard.
- **The complete port.** The milestone does not certify it.

**MAME behaviour adopted as the target.** The reference is MAME 0.289.
None of the following has been investigated against hardware, as the
specification asks for known emulator discrepancies:

- the 500-ns scheduler slice through which the samples CPU sees a latch
  write, modelled as found (fight-samples.md);
- the NMI's delivery at a slice start, modelled by a one-clock stand-in
  without the slice anchors (music-power-on.md);
- the 45–138-cycle interrupt-latency band observed in the reference. It is
  not modelled: a constant inside it stands in (51 in the tests, 92 in the
  launcher) and the checks run its ends. The band follows from the
  program's masked slot scan and the 44-cycle exception entry under MAME's
  68000 timing (latch-timing.md).

## Corrections to earlier records

- **Round 2 of the fixed fight itself.** Issues #26 and #28 were closed on
  sf-round2-02, a P1-idle path with its own KO at 4568. #26's closing
  comment calls it "Round 2 of the fixed fight", and #28's acceptance
  criterion names the fixed fight's round-2 sequence (4773–5014), while
  its closing comment reports sf-round2-02 over 3901–4796. The fixed
  fight's own round 2 (3838–5014) is now compared after every frame here.
- **Issue #38's closing comment** says each launcher fixture runs "as far
  as the translation goes ... the match end". It also says the window
  shows "exactly what the comparison boundary verified". The one-player
  CPU match win stops at 0x3eefa, and interactive play is not a
  verification (launcher.md). In the one-player fixture only P1's keys are
  read.
- **fight-run-a holds 14 checkpoints** (790 through 4990, as its scenario
  lists). The "13 checkpoints" of mame-baseline.md (issue #2) and
  comparison-runner.md's recorded output (issue #3) come from before
  issue #4, which added checkpoint 790 to the scenario and to fight-run-a
  (first-frame.md); fight-run-b still holds the original 13.
- **When control begins.** The fixed fight's scenario notes "fight control
  begins ~2280"; the fight loop runs from 2390, after the round-1 fixture
  (cpu-decision.md).
- **The stale notes** that the idle intro and Ryu's walk, crouch and jump
  still use observed tables are corrected in task-scheduler.md, punch.md
  and posture-dispatch.md. Those tables were deleted by tickets #29 and
  #15.

## Verification

`crates/sf-engine/tests/milestone_audit.rs` (local data; skips without it):

| Test | Result |
| --- | --- |
| The fixture and the audit capture are the fixed fight | sf-cpu-01 equals fight-run-a on every field through 2389; fight-audio-a equals it on every field; the two audit runs are identical; the audit capture equals fight-run-a on all 5,000 frames and its checkpoints in the span |
| The comparison boundary through the match end | 2,625 frames and 2,625 full checkpoints (pixels, video and palette RAM) match, 2390..=5014 |
| Work RAM after every frame | the owned regions equal on all 2,625 frames; every other difference in the task slots' saved words or the stack memory |
| Past the match end | stops at frame 5015 at 0x3eefa |
| The negative control | a punch 10 frames late is caught at 2601 |

`crates/sf-audio/tests/both_chips.rs`, the music CPU's RAM at idle frame
ends: 1,370 of 2,610 frame ends idle, all equal, at the reference's write
times and at latency 51. The rest of that file's tests are in
both-chips.md. `crates/sf-dev` checks every key's binding against
launcher.md's table.

The workspace suite on this tree, with the local reference data present:
159 tests in 56 test binaries pass, none skipped (no test printed its
"skipping" line), and clippy is clean. The tests' printed outcomes are
quoted above.

## Not covered

- **The listening review** of the native mix and of the launcher's live
  output, and a recorded keyboard session. Both need a person.
- **The scripted fight's narrow player input** (above): breadth beyond it
  rests on other fights and controlled fixtures. *(Follow-up 2026-10-31,
  ticket #80 / F40:
  retires the the P2-versus-CPU mode is not exercised entry from the
  audit's breadth table above, which is kept as recorded at the milestone
  audit; game mode 4 is now verified end-to-end against sf-versus-carry-02
  in docs/reference/versus-carry.md and docs/reference/cpu-match-win.md.)*
- **The continue screen and what follows** (0x3eefa) belong to the
  arcade-flow batch (ticket #40).
- **Sound is outside the comparison boundary.** The capture format has no
  synthesized-audio observable, so the sound is compared by its own tests
  on the same fight.

## Listening kit and review follow-up (tickets #49 / F9 and #70 / F30)

Ticket #49 (F9) provides the
listening kit (`cargo run -p sf-dev --release -- kit`) and protocol documented
in `docs/reference/both-chips.md`: local `48 kHz` WAV renders in
`tools/reference/local/renders/` (`round2-mame-reference.wav`,
`round2-native-mix-51.wav`, `round2-native-mix-92.wav`,
`round2-launcher-live-92.wav`, and the full-fight `fight-*.wav` set over frames
`2389..=4999`), where the live launcher capture equals the batch mix at latency
`92` sample for sample (`0` differing out of `936,000` on Round 2 and
`2,088,800` on the full fight). Ticket #70
(F30, `PERSON`) records the human listening verdict here, and ticket #71 (F31, `PERSON`) records the human keyboard sessions of both fixtures in `docs/reference/launcher.md`:

| Axis | Files / mode compared | Machine reference summary (`both-chips.md`) | Human listening verdict (ticket #70 / F30) | Findings / opened tickets |
| --- | --- | --- | --- | --- |
| **Timing** | `round2-native-mix-51.wav` & `round2-native-mix-92.wav` vs `round2-mame-reference.wav`; `fight-*.wav` | Onset `180 ms` and active span `19,225 ms` identical on Round 2; cross-correlation peak at `+4` samples (`0.08 ms`) lag | **Pass** — *"Everything else like the timings seem to match."* | None (all cue onsets, cadence, and durations match). |
| **Pitch** | `round2-native-mix-51.wav` & `round2-native-mix-92.wav` vs `round2-mame-reference.wav`; `fight-*.wav` | YM2151 at `3,579,545 Hz` (`55,930 Hz` output) and MSM5205 at `384 kHz` (`8 kHz` ticks); zero-crossing rates `1862 /s` (`51`) and `1858 /s` (`92`) vs `1584 /s` (MAME) | **Pass** — Pitches of both YM2151 FM music/effects and MSM5205 ADPCM voices/impacts match the MAME reference. | None. |
| **Character** | `round2-native-mix-51.wav` & `round2-native-mix-92.wav` vs `round2-mame-reference.wav`; `fight-*.wav` | Spectral centroid `3445 Hz` (`51` & `92`) vs `3040 Hz` (MAME) on Round 2 (`3344 Hz` vs `2498 Hz` full fight); RMS ratio `1.0037` (`51`) and `1.0045` (`92`) | **Pass (with noted resampler character difference)** — *"The only difference I could discern from the mame reference and the others is that the mame reference has a 'softer' character, even if slightly. Almost as if there's a lp filter on it. But I'm not entirely sure. It's close for sure but the others seem to sound more 'crisp'."* | Matches the machine-measured spectral centroid (`3040 Hz` MAME vs `3445 Hz` native on Round 2; `2498 Hz` vs `3344 Hz` full fight): MAME's `sound_stream` downsampler band-limits the `384 kHz` MSM5205 and `55,930 Hz` YM2151 streams to `48 kHz`, whereas `GameSound` uses zero-order hold steps for the MSM5205 DAC and linear interpolation for the YM2151 (byte-identical output audio / MAME resampler modeling is out of scope per `README.md` and `both-chips.md`). |
| **Native against MAME** | `round2-native-mix-51.wav` & `round2-native-mix-92.wav` vs `round2-mame-reference.wav` (and `fight-*.wav`) | At `51`: Round-2 devices identical, mix correlation `0.9895` (`0.9889` full fight). At `92`: correlation `0.9743` (`0.9843` vs `51` on Round 2), differing from frame `2619` (`3.83 s` in `fight-*.wav`) | **Pass** — No audible divergence between latency `51`, latency `92`, and MAME beyond the slight high-frequency rolloff of MAME's resampler. | None (`IRQ_LATENCY_MID = 92` retained for the launcher). |
| **The live launcher** | `round2-launcher-live-92.wav` vs `round2-native-mix-92.wav`, `fight-launcher-live-92.wav` vs `fight-native-mix-92.wav`, and interactive `cargo run -p sf-dev --release -- cpu` | Scripted live capture equals batch mix at latency `92` sample for sample (`0 / 936,000` on Round 2; `0 / 2,088,800` on full fight); interactive host playback reviewed by ear in #70 | **Pass** — Live launcher playback matches the batch mix and plays cleanly during interactive 1P and 2P sessions (#70 / #71). | First-launch terminal warning on missing `%APPDATA%\sf1-port\bindings.toml` noted during #71 and fixed in `crates/sf-dev/src/main.rs` (`load_active_bindings` / `smoke`). |
