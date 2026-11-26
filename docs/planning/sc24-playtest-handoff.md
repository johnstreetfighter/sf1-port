# SC24: human playtest and activation decision

SC21 is delivered in version **1.5.33**, behind the developer-only `--stronger-cpu` switch. It remains **off by default**, and the CPU wire version is unchanged. [SC24 #216](https://github.com/johnstreetfighter/sf1-port/issues/216) requires the maintainer to review the whole-cast evidence, play the build, and record an explicit decision.

The verified game reports **`Street Fighter': The World Tournament v1.5.33 (fd9f7059)`**. Delivery code is `fd9f70596ddf58cf2bb0da015c7d7b43daa93abd`; measurements use `13a3e219b131e924e469d8d6f29d6369c8ee177e`. Only one integration-test expectation differs; all 4,246 other tracked entries are identical. Measurement runs have not been relabeled as the later commit.

## Evidence to review

| Evidence | Result and limit |
| --- | --- |
| [SC21 Local Fair CPU comparison](sc21-measuring-run.md) | 360 cells and 86,400 completed matches; no stops, unfinished matches or draws. Primary gates pass in 191/192 cells; adjacent gates pass in 130/144. All 192 primary idle bars pass. Fifteen win bars remain unmet. |
| Compatibility | All 434 files match exact EA4/v1.5.32 byte for byte, including 198 legacy stops: 17,162 completed of 17,360 attempts per binary. This is a 40-seed control grid, not a fresh 240-seed rerun of every earlier family. |
| [Native Arcade activity](sc21-native-arcade-activity.md) | Forty trials, twenty paired fixtures: ten authored nonshotos, 1987 style, level 4, cabinet modes 2 and 4. Pooled attacks/minute rise 40.396557 to 47.708766; idle rises 23.579184% to 43.735696%. Only 2/20 pairs improve both. No Ryu/Ken Capcom baseline or other level was measured. |
| [Supported SF1+ Dramatic diagnostic](sc21-dramatic-measuring-run.md) | All 48 cells and 1,920 fights completed: 341 boss wins, 1,577 allies wins and two draws; no stops or unfinished fights. Every sampled boss/level lost a round. The 1987 style is not permitted in Dramatic. No boss-win bar was added. |
| Tests and independent review | 445 original focused checks passed. The initial parallel run had 2,493 passes and one outdated assertion failure; the reviewed test-only correction passed its focused rerun. All 166 serial tests passed. Independent coverage review verified passing evidence for all 2,660 selected identities, retaining four not-selected skips. This is not two literal all-green full-suite passes. |
| [Performance follow-up #277](https://github.com/johnstreetfighter/sf1-port/issues/277) | Exact fifteen failed win cells and the separate mixed native pacing observation. The original bars remain unchanged. |

Prior era-family reports and their unresolved follow-ups remain separate evidence in the [project handoff](../session-handoff.md). SC21 implementation and measurement do not supply whole-cast activation approval.

## Launch and controls

From the repository root:

```powershell
rtk proxy C:/Users/George/Projects/sf1-port/target/suite/sf-game.exe --stronger-cpu
```

The same executable without the flag uses the existing play. There is no stronger-CPU menu switch; online sessions force it off. `--roms <folder-or-archive>` can supply a ROM location. Keep the build and its resources together.

Press Start (`Space`, or pad Options/Menu) at the title. Menus use arrows/D-pad, Enter/Cross/A to confirm, and Esc/Backspace/Circle/B to go back. Q/E or L1/R1 switch Settings tabs. **Settings → Controls** selects or rebinds devices. P1 uses arrows, punches A/S/D and kicks Z/X/C. P2 uses T/F/G/H for up/left/down/right, punches U/I/O and kicks J/K/L. Pause with Space; MOVES lists actual commands.

For the established starting point:

1. Select **Settings → Gameplay → Definitive Overhaul**, set DIFFICULTY 1–4, then enter **Single Player → Arcade Mode** and choose Retsu.
2. Confirm opens the Style card. Left/right changes the selected value, up/down changes the row, and Confirm/Start locks it. ERA SF1 / REV 1987 selects the 1987 style under overhaul; ERA SF1+ selects SF1+.
3. At the native country-select screen choose **JAPAN** (left on the initial two-country screen), then tap and release **Light Punch**: default P1 A, PlayStation Square, Xbox X. Japan's first Retsu ladder slot uses Ryu as the stand-in when you play Retsu. This is a country selection, not a direct Ryu-opponent menu.

Faithful 1987 deliberately retains Capcom's CPU even with the flag. Choose the fighters, styles and levels needed to judge the play; #216 adds no mandatory human match-count matrix.

Cross-mode checks include **Single Player → World Tour** and **Main Menu → Replays**. In Tour, arrows select an unlocked node, Confirm starts it, and Q/L1 opens Tour Fighter. Authored nodes determine opponent style; your own style does not force an arbitrary 1987 opponent.

In Replays, pause and choose an enabled TAKEOVER P1/P2; use REWIND TO BOOKMARK or Q/L1 during the branch. Native mode 2 has the human on P1; mode 4 has the human on P2. Use the chosen side's controls. Stronger-on replays stay in the current session's list, so keep that session open. Older disk replays can exercise takeover too.

## Record the decision

On [SC24 #216](https://github.com/johnstreetfighter/sf1-port/issues/216), record the actual build, what you played, any problems, and whether you approve proceeding after reviewing the failed and explained measuring cells. The earlier Ryu-SF6 cast-rollout approval is not this final approval.

After that decision, SC24 still owns default activation/removal of the developer switch, the one CPU-version bump and guard, retirement of the old scorer and Utility CPU-fight path, affected tests/docs, and the CPU ceiling reassessment. None of those activation changes is part of this delivery.
