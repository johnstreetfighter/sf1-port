# Adon Jaguar Tooth recovery — GitHub #274

2026-11-25. Fix for [GitHub #274](https://github.com/johnstreetfighter/sf1-port/issues/274), based on `cc5729f`. Patch version: 1.5.30.

The exact SC18 Adon USF4 / Ultra 2 / Normal / seed 48 match reproduced the unfinished result: old Player 1 against stronger Player 2, 35,550 reported frames, 6,882 live frames, rounds 1–1. The new regression failed on that row before the fix. The original terminal evidence remains under `target/sc18/adon-final-u2-normal-seed48-2c19f81`; it is separate from the untranslated-branch stops in GitHub #270.

Tooth repeatedly travels backward until its native rear-wall check succeeds. When the opponent crosses behind Adon, ordinary body separation can undo that backward step on every frame. The animation keeps running, but never reaches the wall. This already happens during live combat; Victory later waits for the move's busy bit to clear. A temporary trace confirmed separation opposing the retreat at the pending wall-check call. The trace was removed.

The fix allows bodies to cross while Adon's native stream is waiting at that wall check. It reads the actual bank routine rather than a style's aliased move ID, and applies symmetrically when either participant is seeking the wall. Hurtboxes, attack collision, screen clamps, the wall check, forward kick and landing remain native. Gen's Ouga uses the same routine but is excluded by the fighter check. No busy flag, round timer or result task is forced.

The regression also starts Tooth through real half-circle-back and kick inputs, then arranges an opponent behind its locked facing. Only after actual rear-wall contact does it place that grounded opponent in the returning kick's path, so the damage check has equal target spacing in both orientations. Alpha A-ISM and USF4 Ultra 2 are covered on both sides. Each case requires rear-wall contact, forward rebound, returning-kick damage and recovery at the standing line.

Validation: the original seeded replay failed before the fix and passes afterward. All 72 focused tests pass: two new regressions (including four mirrored native move cases), six Alpha Gen, five Alpha Thailand, seventeen SF3 native, fifteen SF4 native, nine stronger SF3, nine stronger SF4 and nine Dramatic Battle / World Tour checks. The latter include deterministic rollback with three stronger CPUs. Independent Standards and Spec reviews both have zero remaining findings. All eleven local lockfile package versions match workspace version 1.5.30. Logs are retained under `target/fix274` and `target/issue274`.

The first extended fixture run passed the exact match replay and the mirrored wall/rebound/landing checks, but missed a returning-kick damage assertion because its target spacing differed across orientations. The final fixture explicitly arranges the return target after genuine wall contact; all final checks above passed. No production change was needed for that fixture correction.

The maintainer explicitly waived both full-suite passes for this work; neither was run. The SC18 report remains evidence for its pinned, pre-fix gameplay source, not a measuring run of this patch.
