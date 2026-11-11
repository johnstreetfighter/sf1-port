# Game over and the return to attract or the start screen

This document records the boundaries established by
issue #65 (`F25` of the
arcade-flow batch) and issue #85
(`F45`): continue-expiry game over (`0x3f1bc–0x3f274`), event 10's timed colours
and hold (`0x3ff5a–0x3ffb6`), versus game over and the shared screen
(`0x3f2d0–0x3f33a`), the reset (`0x3f33c–0x3f372`), and the return to the start
screen (`0x3f358 -> 0x3dec2`) when credits remain.
Translated in `crates/sf-engine/src/tasks.rs` and
`crates/sf-engine/src/fighter.rs`.

Evidence labels: **DF** at one-player checkpoint `2389` and **CF** at the
versus double-KO fixture at checkpoint `2450`. Local-only captures and ROMs
are required by `crates/sf-engine/tests/game_over.rs`; its tests skip when
those inputs are absent.

## Reference evidence

### One player (`sf-match-end-01`)

The existing scenario has factory DIPs, frames `0–6400`, and full checkpoints
on every frame `5000–6400`. Its two reference runs match byte-for-byte and it
preserves the `sf-fixed-fight-audit-01` and `sf-music-03` baselines as recorded
in [cpu-match-win](cpu-match-win.md).

Native execution from checkpoint `2389` is compared through frame `6330`:
`3,941` per-frame rows (`2390..=6330`), all `1,331` checkpoints
(`5000..=6330`), palette RAM from `6100`, and owned and non-stack work RAM
from `5000`. Frame `6331` reports the attract boundary `0x3ccae` before
executing it; no native/reference match is claimed for that frame. The reset
clears the specified game bytes, and no credits remain, so slot 2 is killed.

At continue expiry, game over queues `0x3f` (observed at `6101`), requests
event 10, and spawns and cycles the eleven absolute effects from the table at
`0x3f278`. Absolute spawn (`0x42f1e`) and the live-effect count (`0x4329c`)
are translated; the effect runner's `0xff` stream dispatch (`0x42dd2`) now
reports the stream's actual target. Event 10 writes the 25 palette words at
`0x3ffb8` to `0xb00022` with their table-specified delays, scrolls fg by
`0xc00`, enables sprites, queues `0x95`, and holds `180` frames. It then
clears the scene-event bit, allowing the reset.

The one-player sound sequence is `0x3f` at `6101`, `0x1b` at `6102`, `6105`,
`6109`, `6113`, `6117`, `6121`, `6125`, `6129`, `6133`, and `6137` (ten in
total), followed by `0x95` at `6151`.

### One player with a credit remaining (`sf-gameover-credit-01`, issue #85)

`tools/reference/scenarios/sf_gameover_credit_01.json` has the fixed fight's
inputs plus `coin2` at frame `1480` (held 12 frames), so 2 credits are present
before the 1P start at `1520` and 1 credit (`0xff865a..0xff865d = 0x00000001`)
remains throughout the fight, continue countdown, and game over. It runs for
`6,401` frames (`0..=6400`) with factory DIPs (`dsw1 = 0xdfff`, `dsw2 = 0xffff`)
and checkpoints at `2389` and every frame `5000..=6400` (`1,402` checkpoints).
The two reference captures (`gameover-credit-01-run-a`,
`gameover-credit-01-run-b`) match byte-for-byte across all `7,013` artifact
files, and `frames.tsv` rows `0..=1479` equal `fight-run-a`'s rows `0..=1479`.

Every difference from `sf-match-end-01` after frame `1479` traces to the
second coin at `1480`:

1. **Start-screen second coin (`1481–1520`):** vblank debounces `coin2` on
   release (`1492`), slot 7 awards credit 2 at `1495` (`0x0f` latched at `1496`,
   `0xff865d = 2`), draws `PUSH 1 OR 2 PLAYERS START BUTTON` (`0x14e4e` instead
   of `0x14e2a` over `1497–1511`), and sleeps 6 frames for the slot-2 coin
   counter pulse (`0x48344`, `1495..1501`).
2. **1-frame shift in slot 7's 6-frame `step_random` phase (`1501+`):** the
   extra coin-counter pulse sleep shifts slot 7's 6-frame `step_random` cycle by
   1 frame (at checkpoint `2389`, slot 7's sleep byte `0xff8077` is `1` instead
   of `6`, `0xff865d` is `1` instead of `0`, and the sound-queue pointers
   `0xff865f`/`0xff8661` are `4` instead of `2` from the second `0x0f`). Because
   `step_random` advances on a shifted 6-frame phase during the fight, Retsu's
   AI decisions finish the two rounds 28 frames earlier overall (round 1 KO at
   `3401` vs `3668`, round 2 KO at `4986` vs `5014`, continue start at `5387` vs
   `5415`, continue expiry and game-over start at `6072` vs `6100`, `0x95` at
   `6123` vs `6151`, and `game_over_reset` at `6303` vs `6331`).
3. **Continue screen with 1 credit (`5387–6071`):** `continue_frame` checks
   `0xff865a..0xff865d != 0` at `0x3f14a` and polls `start1` (`0x3f156`), which
   remains unpressed, while slot 3 (`watch`) sits in state `0x3f47e` (`d2 = 2`)
   instead of `0x3f454`.
4. **Reset branch to start screen (`6303–6400`):** at frame `6303`,
   `game_over_reset` (`0x3f33c–0x3f360`) clears `0xff8003` and
   `0xffc002/03/04/08/09/0a`, sees `0xff865a..0xff865d >= 1` (`0x3f358:
   cmpi.l #1, ($65a,A5); bcc $3dec2`), and branches directly to
   `start_screen_entry` (`0x3dec2–0x3df48`) without killing slot 2 (`0xff8026`
   stays `8`, sleeping at `0x3f7d6`) and without reaching `0x3ccae`.
   `start_screen_entry` sets `0xff8003 = 0xff`, clears the object pools, queues
   `0x3f` (`6304`) and `0x80` (`6305`), enables `tx` only (`0x08`), clears the
   text layer, draws the 5 start-screen blocks from `0x3e024`, and enters
   `start_screen_frame` (`0x3df4c–0x3e020`), drawing `1` credit at
   `0x800e66..0x800e6a` and blinking `PUSH 1 PLAYER START BUTTON` through frame
   `6400`.

Native execution from `sf-gameover-credit-01`'s checkpoint `2389` matches all
`4,011` per-frame rows (`2390..=6400`), all `1,401` checkpoints (`5000..=6400`),
palette RAM from `6070..=6400`, and owned and non-stack work RAM across
`5000..=6400`, with zero unrecovered branches (`unrecovered().is_empty()`).

### One player with `Game Continuation = None` (`sf-no-continue-01`, issues #123 and #142)

`tools/reference/scenarios/sf_no_continue_01.json` has `sf-fixed-fight-01`'s inputs with `Game Continuation = None` (`dsw1 = 0xdfff`, `dsw2 = 0xfffa`, `0xffc000 = 0x00`), running for `5,660` frames (`0..=5659`) with checkpoints at `0..=830` and `2389..=5656` (`4,099` checkpoints). Its two reference captures (`no-continue-01-run-a`, `no-continue-01-run-b`) match byte-for-byte across all `20,498` observation files, and match `sf-match-end-01` on `frames.tsv` rows `0..=5414` with only `0xffc000` (`0x00` vs `0x05`) differing in `work.bin` through `5414`.

At frame `5415`, `continue_start` (`0x3efda: move.b ($c000,A5), D0; cmp.b ($c003,A5), D0; bls $3f2f0`) sees `0xffc000 = 0 <= 0xffc003 = 0` and jumps directly to `0x3f2f0` (`game_over_screen`), skipping the 10-second continue countdown (`0x3f02a..0x3f1bc`) and event 10 (`0x3f1bc..0x3f274`). `0x3f2f0..0x3f33a` disables layers (`0x00`), scrolls fg by `0x0c00`, requests palette `0x1a268` at offset `0x200`, queues `0x95` (latched at `5416`), sleeps 1 frame (`5415`), enables sprites (`0x80`), and holds `240` frames (`5416..=5655`). At frame `5656`, `0x3f33c..0x3f372` clears `0xff8003` and `0xffc002/03/04/08/09/0a`, kills slot 2 (`0xff8026 = 0x00`), and enters `0x3ccae` (`attract_join`).

Native execution matches all `3,267` post-fixture frames (`2390..=5656`) from checkpoint `2389` (`#123`, `F51`) and all `5,657` frames (`0..=5656`) and `4,099` checkpoints from power-on (`#142`, `F70`), and verifies all six `Game Continuation` limits `0..=5` across `stage = 0..=4` at `0x3efda` (`continue_limit > stage` enters `0x3f02a`, `continue_limit <= stage` branches to `0x3f2f0`).

### Versus double KO (`sf-doubleko-02`)

`tools/reference/scenarios/sf_doubleko_02.json` retains all `141` input records
and factory DIPs from `sf-doubleko-01`, extends the duration to `6,001` frames,
and captures checkpoint `2450` plus every frame `5560–6000` (`442`
checkpoints). Reproduce a capture with:

```
python tools/reference/run_scenario.py tools/reference/scenarios/sf_doubleko_02.json \
  --out tools/reference/local/captures/doubleko-02-run-a
```

The local captures `doubleko-02-run-a` and `doubleko-02-run-b` have `2,213`
identical files: three TSVs plus five artifacts for each checkpoint. Their
`frames.tsv` rows through `5800`, and all `1,205` shared checkpoint artifacts
over `5560–5800`, are identical to `doubleko-01-run-a`.

Native execution compares frames `2451–5559` against `sf-doubleko-01`
(`3,110` per-frame rows and checkpoints), then frames `5560–5902` against
`sf-doubleko-02` (`343` rows and checkpoints). Owned and non-stack work RAM
matches on every frame through `5902`. Frame `5903` reports `0x3ccae`, and
reference frame `5904` already sleeps in the attract task. The remaining
reference frames `5903–6000` document the original's attract transition; they
are not included in the native comparison beyond that explicit boundary.

At frame `5662`, after the wipe, versus game over checks P1 with scene byte 2,
then P2 with scene byte 1. Both non-qualifying scores are cleared. The shared
screen disables layers, scrolls fg by `0xc00`, requests palette `0x1a268` at
`0x200`, queues `0x95`, sleeps one frame, enables sprites, and holds `240`
frames before the reset. The continuation-limit branch of `continue_start`
uses this same screen and reset.

## Stream-call probe, reset branch, and negative control

The static probe walks the object streams of all eleven templates at
`0x3f278`, following `0xfe` stream jumps and terminating at `0xfd`. It lists
every `0xff` routine pointer reached: the list is empty, so these objects
require no additional stream-called routine. It also verifies that none of
these absolute templates requests owner pointers.

The reset clears `0xff8003` and `0xffc00a`, `0xffc002`, `0xffc008`,
`0xffc009`, `0xffc004`, and `0xffc003`. With credits remaining (`0x3f358:
cmpi.l #1, ($65a,A5); bcc $3dec2`) it branches directly to `start_screen_entry`
(`0x3dec2`, `F45` #85); otherwise (`0x3f364..0x3f372`, connected by `F66` #138) it kills slot 2 (`0x3b230`) and enters `0x3ccae` (`Flow::AttractJoin`: spawns slot 3 at `0x3f374` and enters `0x3ccc2` `attract_loop_head`).

Two negative controls (`C-NEG`) are verified in `crates/sf-engine/tests/game_over.rs`:

1. Changing event 10's hold at `0x3b574` from `180` to `179` fails immediately
   at frame `6150` in `sf-match-end-01`, on slot 2's sleep byte `0xff8027`
   (reference `0xb4`, native `0xb3`).
2. Changing `0x3f358`'s operand displacement at `0x3f35e` from `0x065a` to
   `0x0656` so the credit count is read as `0` at `0x3f358` fails at frame
   `6303` in `sf-gameover-credit-01` (the exact frame of the branch: slot 2 is
   killed at `0xff8026 = 0x00` vs reference `0x08` and `0x3ccae` is reported).

## Remaining boundaries

- Qualifying scores still report the name-entry setup `0x42486` (`F42`).
- Continue taken (`0x3f166–0x3f1b8`) is translated by `F39` (#79; see [cpu-match-win](cpu-match-win.md)).
- The attract implementation beyond the reported `0x3ccae` boundary is not
  changed by this ticket.
