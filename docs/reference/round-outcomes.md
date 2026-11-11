# Time-over, draw and double KO

This document records the boundary established by
issue #30: the round
outcomes other than a clean KO — the time-over judged by health, the
time-out draw, the double KO — and the three-rounds rule that follows
them, with the versus match end they lead to. Code:
`crates/sf-engine/src/tasks.rs`. See round-result.md for the KO path they
share and match-win.md for the result and match-won screens.

## The time-over (0x3ed20)

When the fight loop wakes with TIME at zero:

1. a sleep; then scene mode 0, TIME OVER (text 0x14f74) shown, the TIME
   digits drawn (0x3beb4) and sound 0x96;
2. **the hold** (0x3ed46): a counter of 0x82 is counted down before each
   sleep — 0x81 sleeps — and while either fighter is still busy its frame
   runs (the stage drift, the dispatcher, effects, the camera, the TIME
   digits);
3. **the judgement** (0x3ed98): the TIME digits are copied to 0xff8e3c,
   the text erased, and both fighters' health points (cells × 4 + the
   remainder, 0x3c0b0) compared: the higher wins, joining the KO path's
   win record (0x3eba2: the win counted, the markers, the round result
   requested, the victory pose); equal health is a draw: outside demo
   play the draw text (event 8) is requested, and the round ends
   (0x3edf6).

**Event 8 (0x3fef2), the draw.** The round's text from the table 0x3ff18
is typed (3 frames a cell) and held 120 frames; nothing erases it — the
next wipe does.

## The double KO (0x3eca2)

Reached from the KO path when the fighter that is down finds its opponent
down too: scene mode 0 and one frame of the fighters' work (as above),
then the sleep, round again while either is busy; then DOUBLE K.O. (text
0x14f56) is held through the same 0x82 counter, erased, and outside demo
play the round ends (0x3edf6) with no win counted.

## After the round (0x3edf6 / 0x3ec40) and the three-rounds rule (0x3ee04)

The round counter counts up; a third round played goes to the rule,
otherwise the next round's wipe (event 6) is requested
(round-transition.md). **The rule:** the fighter with more wins takes the
match (0xffc009 = 1 for P1, 2 for P2, 4 for the CPU); equal wins leave no
winner (0xffc009 = 0). The match end (0x3ee26) then reads it: in the
one-player modes a draw is the CPU's match win (0x3eefa / 0x3ef2c: the
continue screen) — a draw counts as a loss, as the code says; in versus it
is the game over (0x3ee4e: the wipe, then 0x3f2d0), translated and covered in
[game-over](game-over.md).

**The versus match end** (0x3ee3a): a winner goes on against the CPU —
sound 0x37, the match-won screen (event 0xb's versus shape: the winner's
text, then the background scrolled to the winner's word of 0x400d8 with
the palette 0x1ae68 and sound 0x9c, and the text 0x15a78 for 120 frames),
the wipe, then game mode 2 (P1) or 4 (P2) with the scene byte 1 / 2. The
departing player's ranking check and the carry-over into Retsu's fight are
translated and verified in [versus-carry](versus-carry.md) (#66).

The versus machine no longer refuses intervals past the fixture's
time-over frame (4330).

## Verification

`crates/sf-engine/tests/round_outcomes.rs`: two versus scenarios from the
checkpoint-2450 fixture (Ryu P1 vs Ken P2), checkpoints every frame, two
reference runs each, identical.

**sf-doubleko-01, 2450–5800.** Mirrored play: every 200 frames both walk
in for 40 frames and throw a Hadoken at the walk pin; the projectiles
spawn past each other and land on both on the same frame (hadoken.md), 20
each; the third pair knocks both out together.

| Frames | What happens |
| --- | --- |
| 2522, 2722, 2922 | the mutual hits; the third a **double KO** (both +8 = 8) |
| 2929–3162 | DOUBLE K.O., no win counted; round 1 |
| 3163–3541 | the wipe, ROUND 2, the walk-in |
| 3722, 3922, 4122 | the same; a double KO; round 2 |
| 4922, 5122, 5322 | the same; a double KO; round 3 |
| 5563–5661 | the rule finds 0–0, no winner (0xffc009 = 0); the wipe |
| 5662 | versus game over begins: both players' ranking checks, then the shared screen |
| 5663–5902 | the palette sleep and 240-frame hold, then the game-state clears |
| 5903 | no credits: slot 2 is killed and the attract boundary `0x3ccae` is reported |

**sf-timeover-01, 2450–10300.** Ryu walks in and lands one jab (10
points; Ken at 11 cells + 2), then both players idle.

| Frames | What happens |
| --- | --- |
| 4330 | TIME reaches zero |
| 4331–4459 | TIME OVER held |
| 4460–4745 | Ryu ahead on health: the round win, the result, the tally (TIME 0: no time ticks; vitality 48: 48 ticks → score 490, bonus 480) |
| 4746–5124 | the wipe, ROUND 2, the walk-in |
| 7025–7154 | time-over with equal health |
| 7155–7365 | DRAW GAME (event 8) |
| 7366–7744 | the wipe, ROUND 3 |
| 9645–9985 | the second draw |
| 9986 | three rounds: 1–0, Ryu takes the match (0xffc009 = 1) |
| 9986–10167 | the versus match-won screen |
| 10168–10266 | the wipe |
| 10267 | non-qualifying ranking check, score clear and mode-2 carry-over |
| 10268–10269 | country palette sleep, direct player placement and Retsu's event-4 intro |
| 10270–10300 | Retsu's stage intro continues with no untranslated branch |

Each matches the reference throughout:

- **Observables:** every one matches — per-frame screen hashes,
  checkpoint pixels and video/palette RAM on every frame (TIME OVER,
  DOUBLE K.O., DRAW GAME, the result and tally, the versus match-won
  screen with its loaded palette), and sound-latch timing (0x96, the draw
  and result sounds, 0x37, 0x9c).
- **Owned RAM:** byte-identical after every compared frame — 3212 frames in
  sf-doubleko-01 through 5661 and 7850 in sf-timeover-01 through 10300. The
  double-KO run continues through its game over and reset in
  [game-over](game-over.md). The versus comparison also covers the result's
  numbers, game-state bytes and palette request.

**The screen hash and a palette load.** The versus match-won screen is
the first scene where a palette load changes the visible colours: the
request made in frame 10046 is served by the vblank of 10047, and the
reference's hash at 10047 — otherwise the scanout of the end-of-10046
state — already shows the new palette (rendering the 10046 dumps with
the 10047 palette reproduces d4ef80e0; with their own palette, c09de641).
The 68000's interrupt handler copies the palette before the screen is
drawn. The native capture now renders the hash frame with the current
frame's palette when it changed (`run::drive`; `examples/hash_probe.rs`
is the check).

## One-player round 3, time-over, draw and double KO (#72, F32)

`crates/sf-engine/tests/one_player_round3.rs`: three one-player scenarios on the `sf-cpu-01` boot path from checkpoint `2389` (Ryu P1 vs CPU Retsu), checkpoints every frame, two reference runs each, identical. All three share the same `1-1` split through `4911` (Ryu wins round 1 by heavy Shoryuken KO at `2898`, idles through round 2 for Retsu to even the match at `4307`, and enters round 3 at `4911` with `0xffc00a = 2, wins = 1-1`):

- **`sf-1p-round3-01`, 2389–7600.** Ryu throws heavy Hadokens in round 3 to survive to its time-over at `6800` (`0x3ed26`, text `0x14f74` `TIME OVER`, sound `0x96`). The health comparison at `6930` (`0x3ed98`) finds Ryu ahead (`48 > 16`), giving Ryu round 3 (`2-1`), the match decision at `7216` (`0xffc009 = 1`, `0x3ef54`), and Geki's setup at `7586..=7600` (`0x3e6ca`, `stage = 0, fight = 1, round = 0`).
- **`sf-1p-draw-01`, 2389–7560.** Ryu blocks high in round 3 to reach time-over at `6800` with equal health (`48 == 48`). The judgement at `6930` requests event 8 (`DRAW GAME`, `0x3fef2`), after which `0x3edf6 -> 0x3ee04` (`three_rounds`) sees equal wins (`1 == 1`, `0xffc009 = 0`) and enters the CPU's match win at `0x3eefa` (`WinCardCpu`, event 11) and `F5`'s continue screen (`0x3efda..0x3f162`) at `7541..=7560`.
- **`sf-1p-doubleko-01`, 2389–6320.** Ryu and Retsu trade a final blow in round 3 for a double KO at `5770..5771` (`0x3eca2..0x3ed00`, `DOUBLE K.O.` `0x14f56`). At `5900`, `0x3edf6 -> 0x3ee04` sees equal wins (`1 == 1`, `0xffc009 = 0`) and enters the CPU's match win at `0x3eefa` and `F5`'s continue screen at `6300..=6320`.

## Not covered

- **A mode-4 (2P vs CPU) equal-wins draw** (`0x3ef2c`, translated in [p2-one-player](p2-one-player.md), #80, where P2's KO loss exercises `0x3ef2c`; the mode-2 equal-wins draw `0x3ee04 -> 0x3eefa` is covered above by `sf-1p-draw-01` and `sf-1p-doubleko-01`).
- **The versus game over** (`0x3f2d0`) is translated and compared in [game-over](game-over.md) (#65).
- **The versus winner's continuation** and qualifying name entry are translated and compared in [versus-carry](versus-carry.md) (#66) and [name-entry](name-entry.md) (#82).
- **The attract demo's outcomes** (`0x3ce8c`) are translated and compared in [attract-demo](attract-demo.md) (#86).
