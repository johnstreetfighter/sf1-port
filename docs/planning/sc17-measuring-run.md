# SC17: SF3 stronger Fair CPU measuring report

2026-11-24. Version `1.5.28`. Ticket [#209](https://github.com/johnstreetfighter/sf1-port/issues/209).

All 384 new-versus-old cells meet both win and idle bars. This remains true for every primary win bar when every engine-stopped match is counted as a loss. The six secondary misses are Joe's Difficult-over-Normal mirrors, tracked in [#273](https://github.com/johnstreetfighter/sf1-port/issues/273); their cause has not been isolated. The stronger play remains behind `--stronger-cpu`.

## Source and method

Every table below uses code [9bded71](https://github.com/johnstreetfighter/sf1-port/commit/9bded71f0b870dcc09c2e71cc5a12ee0d52d507e), with measuring executable SHA-256 `2f701ab2163409ebcfe2fde66986c5d6b079f3d9f92e94fd2d2ab9a3b03b5095`. The executable was built with `rtk cargo build --profile suite -p sf-host --example cpu_measure --locked -j 4`.

The grid contains 48 selectable fighter/revision/Super Art pairs: all twelve tournament fighters in Third Strike, plus Ryu and Ken in New Generation and Second Impact, each with Arts I, II and III. Each pair has eight new-versus-old cells (four levels, both sides), six adjacent-level cells, and one old-versus-old control at Very Difficult. Every cell uses seeds 1-240 and eight worker threads. The whole run took 25 minutes 45 seconds.

There were **172,800 attempted matches, 171,950 completed matches, 850 untranslated-branch stops and zero unfinished matches**. All 384 primary cells pass; 282 of 288 adjacent-level cells pass. Counting all stops as losses adds no further win-bar failure.

Reproduce a cell from the full table with the existing example, replacing the placeholders:

```text
rtk cargo run --profile suite -p sf-host --example cpu_measure --locked -- <out.tsv> <fighter> <style> <p1-old-or-new> <fighter> <style> <p2-old-or-new> <level-or-p1/p2> 1 240 8
```

Levels 1-4 are Easy, Normal, Difficult and Very Difficult. In new-versus-old rows, `first/second` means new/old; in adjacent rows it means higher/lower; in old-versus-old rows it means P1/P2. Idle and attack rates are weighted by live frames, using the per-match rates rounded to two decimals. Idle comparisons use completed matches only. The pessimistic win column includes stopped matches in the denominator as losses. Every stopped seed, frame and branch address is listed with its cell below; the existing engine gap is tracked in [#270](https://github.com/johnstreetfighter/sf1-port/issues/270).

## Switch-off and existing-era controls

The old CPU control build uses an archived snapshot of [aef5426](https://github.com/johnstreetfighter/sf1-port/commit/aef5426), version `1.5.27`. Its measuring executable SHA-256 is `d14a965673888c12104842479eff563fbeb3d45024557d75bd32be8da13f64d5`.

The frozen SC17 build reproduces **all 86 cells / 3,440 attempted rows byte for byte**, including stop lines. These controls cover all 48 SF3 pairs with the switch off at Very Difficult; Ryu SF6 at all four levels against the old CPU, plus Very Difficult versus Difficult on both sides; and representative SF2/SFZ role and tool paths. They completed 3,414 matches and reproduced 26 existing stops.

## Gen's tuning

A 240-seed check confirmed excess idle at Normal and Difficult with Gen's original charge-plan priority of 58. Only his SF3 Jasen priority was lowered, to 36, letting jump-ins compete with the slow backward charge. His charge, Crane stance, route book, reaction delays and level values remain available. All three Arts now pass every primary and adjacent bar, including pessimistic win checks. His narrowest idle improvement is 0.70 percentage points.

## Validation

Independent standards and spec reviews of `aef5426...9bded71` have zero remaining findings. The EX-facing issue and the fight tests' attribution gaps were addressed and reviewed.

- `rtk cargo suite-parallel`: **2,317 passed, zero failures, three skipped**; Nextest test runtime 2,557.165 seconds.
- `rtk cargo suite-serial`: **166 passed, zero failures, one skipped**; Nextest test runtime 355.194 seconds.

Both commands returned exit code 0. They ran sequentially on the code above, using the whole locked workspace and the configured suite profile. No MAME recording or other suite ran alongside them. All nine SC17 fight tests and all nine Dramatic Battle/World Tour stronger-CPU tests passed in the full parallel run.

The game reports `Street Fighter': The World Tournament v1.5.28 (9bded71f)`. The complete logs and copied JUnit files are retained locally under `target/sc17/` as `suite-parallel.log`, `suite-serial.log`, `suite-parallel.junit.xml` and `suite-serial.junit.xml`.

The main full-1P/full-2P captures and the SC17 ROM/art fixtures are present. Existing audio-hook captures (`full-1p-01-hook-a/b`, `full-2p-01-hook-a/b`), synthetic session captures and human session captures remain absent locally; their fixture-dependent checks retain their existing self-skip paths. No human playtest was performed in this session.

## Per-fighter results

All 384 new-versus-old cells pass win and idle bars over completed matches. All primary win bars also pass with every engine-stopped match counted as a loss. Adjacent-level ordering passes in 282 of 288 cells; Joe Difficult over Normal misses in all three Arts on both sides. Idle numbers are computed over completed matches only.

| Fighter | Primary passed | Lowest wins % of completed | Lowest wins % of 240 attempts | Smallest idle improvement (points) | Adjacent passed |
| --- | --- | --- | --- | --- | --- |
| ryu | 72/72 | 79.58 | 79.58 | 13.55 | 54/54 |
| ken | 72/72 | 80.42 | 80.42 | 11.81 | 54/54 |
| retsu | 24/24 | 60.76 | 60.00 | 11.61 | 18/18 |
| geki | 24/24 | 90.00 | 90.00 | 8.51 | 18/18 |
| joe | 24/24 | 82.50 | 82.50 | 13.90 | 12/18 |
| mike | 24/24 | 59.17 | 59.17 | 10.80 | 18/18 |
| lee | 24/24 | 75.83 | 75.83 | 10.48 | 18/18 |
| gen | 24/24 | 66.53 | 65.42 | 0.70 | 18/18 |
| birdie | 24/24 | 80.33 | 80.00 | 3.46 | 18/18 |
| eagle | 24/24 | 88.46 | 85.00 | 5.28 | 18/18 |
| adon | 24/24 | 92.92 | 92.92 | 9.63 | 18/18 |
| sagat | 24/24 | 86.25 | 86.25 | 6.15 | 18/18 |

## Branch stops

850 stopped matches: 41 old-versus-old, 333 primary, 476 adjacent. No unfinished matches; every stop is an untranslated-branch message. Every stopped seed, frame and address is included with its cell in the stopped-match section below.

| Fighter | Old/old | Primary | Adjacent | Addresses |
| --- | --- | --- | --- | --- |
| adon | 10 | 6 | 3 | 0x1a25c7, 0x1a30c8, 0x1a30d0, 0x1a370a |
| birdie | 1 | 17 | 22 | 0x1823c7, 0x182d02, 0x183545 |
| eagle | 24 | 132 | 75 | 0x192b55, 0x192b65, 0x1930e7, 0x1930ef, 0x193220, 0x193c4c |
| geki | 0 | 0 | 8 | 0x133bb7 |
| gen | 0 | 99 | 110 | 0x172467, 0x17255e, 0x1725cd, 0x17302c, 0x1730e1, 0x173e8c |
| joe | 0 | 5 | 14 | 0x14256f, 0x142577, 0x14257f, 0x1435e2 |
| ken | 0 | 0 | 1 | 0x11208e |
| lee | 0 | 12 | 69 | 0x162b1f |
| mike | 0 | 6 | 6 | 0x152bd0 |
| retsu | 6 | 24 | 15 | 0x122d35, 0x122d45, 0x123db0 |
| ryu | 0 | 13 | 0 | 0x10192f, 0x101b61 |
| sagat | 0 | 19 | 153 | 0x1b88b0 |

## Adjacent-level misses

| Fighter / Art | Higher-level side | Wins-losses | Stopped |
| --- | --- | --- | --- |
| joe sf3threes/art1 | higher P1 | 112-127 | 1 |
| joe sf3threes/art1 | higher P2 | 113-125 | 2 |
| joe sf3threes/art2 | higher P1 | 112-128 | 0 |
| joe sf3threes/art2 | higher P2 | 107-131 | 2 |
| joe sf3threes/art3 | higher P1 | 116-124 | 0 |
| joe sf3threes/art3 | higher P2 | 110-128 | 2 |

No additional win failure appears when every stopped match counts as a loss.

## Full measuring tables

Source: `9bded71f0b870dcc09c2e71cc5a12ee0d52d507e`.

Invocation: `target/sc17/measure_sc17.py final --binary target/sc17/bin/cpu_measure_sc17-9bded71.exe --out target/sc17/final-9bded71 --source 9bded71f0b870dcc09c2e71cc5a12ee0d52d507e --count 240 --threads 8`.

Seeds 1–240 on each side; 720 of 720 cells completed. 6 cells miss a bar.

New against old: at least half of played matches won, and less idle. Adjacent levels: the higher level must win more than half. Idle and attack rates are weighted by live frames from the per-match values rounded to two decimals. Engine-stopped matches are counted separately; the worst-case win column counts all stopped matches as losses. Draws and unfinished matches are reported, not silently discarded.

| Fighter | Style | Level | Run | Wins-losses-draws | Played (stopped) | Idle first/second % | Attacks/min first/second | Bar | Worst-case wins |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ryu | sf3ng/art1 | 4 | old vs old | 145-95-0 | 240 (0) | 30.46/27.04 | 61.63/58.09 | - | - |
| ryu | sf3ng/art1 | 1 | new P1 | 232-8-0 | 240 (0) | 25.31/41.02 | 50.17/33.06 | pass | pass |
| ryu | sf3ng/art1 | 1 | new P2 | 231-8-0 | 239 (1) | 25.52/40.89 | 49.63/33.31 | pass | pass |
| ryu | sf3ng/art1 | 2 | new P1 | 235-5-0 | 240 (0) | 22.19/36.70 | 52.95/35.75 | pass | pass |
| ryu | sf3ng/art1 | 2 | new P2 | 232-8-0 | 240 (0) | 21.97/36.69 | 53.06/36.11 | pass | pass |
| ryu | sf3ng/art1 | 3 | new P1 | 216-24-0 | 240 (0) | 20.41/34.60 | 56.35/44.85 | pass | pass |
| ryu | sf3ng/art1 | 3 | new P2 | 218-22-0 | 240 (0) | 19.63/35.16 | 56.30/44.72 | pass | pass |
| ryu | sf3ng/art1 | 4 | new P1 | 211-29-0 | 240 (0) | 19.28/33.19 | 61.14/50.32 | pass | pass |
| ryu | sf3ng/art1 | 4 | new P2 | 195-45-0 | 240 (0) | 17.07/35.53 | 58.96/52.71 | pass | pass |
| ryu | sf3ng/art1 | 2/1 | higher P1 | 217-23-0 | 240 (0) | 23.45/28.85 | 48.01/39.55 | pass | pass |
| ryu | sf3ng/art1 | 1/2 | higher P2 | 213-27-0 | 240 (0) | 23.44/28.95 | 48.04/40.19 | pass | pass |
| ryu | sf3ng/art1 | 3/2 | higher P1 | 199-41-0 | 240 (0) | 23.27/24.46 | 47.65/44.93 | pass | pass |
| ryu | sf3ng/art1 | 2/3 | higher P2 | 189-51-0 | 240 (0) | 22.48/24.41 | 47.30/44.44 | pass | pass |
| ryu | sf3ng/art1 | 4/3 | higher P1 | 187-53-0 | 240 (0) | 21.26/22.94 | 49.36/47.31 | pass | pass |
| ryu | sf3ng/art1 | 3/4 | higher P2 | 176-64-0 | 240 (0) | 21.12/23.24 | 49.72/47.05 | pass | pass |
| ryu | sf3ng/art2 | 4 | old vs old | 147-93-0 | 240 (0) | 30.49/27.19 | 61.53/58.01 | - | - |
| ryu | sf3ng/art2 | 1 | new P1 | 230-10-0 | 240 (0) | 25.47/40.98 | 49.73/33.26 | pass | pass |
| ryu | sf3ng/art2 | 1 | new P2 | 232-7-0 | 239 (1) | 25.64/40.65 | 49.44/33.68 | pass | pass |
| ryu | sf3ng/art2 | 2 | new P1 | 233-7-0 | 240 (0) | 22.18/36.76 | 52.57/35.92 | pass | pass |
| ryu | sf3ng/art2 | 2 | new P2 | 232-8-0 | 240 (0) | 22.36/36.59 | 52.39/36.38 | pass | pass |
| ryu | sf3ng/art2 | 3 | new P1 | 224-16-0 | 240 (0) | 19.96/34.58 | 56.78/44.77 | pass | pass |
| ryu | sf3ng/art2 | 3 | new P2 | 232-8-0 | 240 (0) | 19.61/35.38 | 56.17/44.13 | pass | pass |
| ryu | sf3ng/art2 | 4 | new P1 | 216-23-0 | 239 (1) | 19.43/33.73 | 61.81/50.43 | pass | pass |
| ryu | sf3ng/art2 | 4 | new P2 | 201-38-0 | 239 (1) | 17.41/35.64 | 59.06/52.72 | pass | pass |
| ryu | sf3ng/art2 | 2/1 | higher P1 | 219-21-0 | 240 (0) | 23.50/28.93 | 47.74/39.59 | pass | pass |
| ryu | sf3ng/art2 | 1/2 | higher P2 | 222-18-0 | 240 (0) | 23.75/28.99 | 48.30/39.98 | pass | pass |
| ryu | sf3ng/art2 | 3/2 | higher P1 | 201-39-0 | 240 (0) | 23.02/24.24 | 47.04/44.53 | pass | pass |
| ryu | sf3ng/art2 | 2/3 | higher P2 | 188-52-0 | 240 (0) | 22.57/24.40 | 47.21/44.15 | pass | pass |
| ryu | sf3ng/art2 | 4/3 | higher P1 | 170-70-0 | 240 (0) | 21.18/23.01 | 49.42/47.05 | pass | pass |
| ryu | sf3ng/art2 | 3/4 | higher P2 | 180-60-0 | 240 (0) | 21.44/22.88 | 49.65/47.19 | pass | pass |
| ryu | sf3ng/art3 | 4 | old vs old | 154-86-0 | 240 (0) | 30.35/26.99 | 61.20/57.63 | - | - |
| ryu | sf3ng/art3 | 1 | new P1 | 232-8-0 | 240 (0) | 25.31/41.02 | 50.17/33.06 | pass | pass |
| ryu | sf3ng/art3 | 1 | new P2 | 231-8-0 | 239 (1) | 25.52/40.89 | 49.63/33.31 | pass | pass |
| ryu | sf3ng/art3 | 2 | new P1 | 235-5-0 | 240 (0) | 22.17/36.65 | 52.96/35.75 | pass | pass |
| ryu | sf3ng/art3 | 2 | new P2 | 233-7-0 | 240 (0) | 21.99/36.66 | 52.93/36.13 | pass | pass |
| ryu | sf3ng/art3 | 3 | new P1 | 225-15-0 | 240 (0) | 20.11/34.71 | 56.32/44.90 | pass | pass |
| ryu | sf3ng/art3 | 3 | new P2 | 219-21-0 | 240 (0) | 19.46/35.27 | 56.08/44.70 | pass | pass |
| ryu | sf3ng/art3 | 4 | new P1 | 215-25-0 | 240 (0) | 19.37/33.16 | 61.09/50.37 | pass | pass |
| ryu | sf3ng/art3 | 4 | new P2 | 204-36-0 | 240 (0) | 17.10/35.69 | 58.96/52.03 | pass | pass |
| ryu | sf3ng/art3 | 2/1 | higher P1 | 217-23-0 | 240 (0) | 23.45/28.85 | 48.01/39.55 | pass | pass |
| ryu | sf3ng/art3 | 1/2 | higher P2 | 213-27-0 | 240 (0) | 23.44/28.95 | 48.04/40.19 | pass | pass |
| ryu | sf3ng/art3 | 3/2 | higher P1 | 199-41-0 | 240 (0) | 23.27/24.46 | 47.65/44.93 | pass | pass |
| ryu | sf3ng/art3 | 2/3 | higher P2 | 189-51-0 | 240 (0) | 22.48/24.41 | 47.30/44.44 | pass | pass |
| ryu | sf3ng/art3 | 4/3 | higher P1 | 187-53-0 | 240 (0) | 21.26/22.94 | 49.36/47.31 | pass | pass |
| ryu | sf3ng/art3 | 3/4 | higher P2 | 176-64-0 | 240 (0) | 21.12/23.24 | 49.72/47.05 | pass | pass |
| ryu | sf3twoi/art1 | 4 | old vs old | 144-96-0 | 240 (0) | 30.48/27.13 | 61.58/58.08 | - | - |
| ryu | sf3twoi/art1 | 1 | new P1 | 229-11-0 | 240 (0) | 25.28/41.06 | 49.66/32.84 | pass | pass |
| ryu | sf3twoi/art1 | 1 | new P2 | 229-10-0 | 239 (1) | 25.53/40.85 | 49.14/33.27 | pass | pass |
| ryu | sf3twoi/art1 | 2 | new P1 | 235-5-0 | 240 (0) | 22.11/36.42 | 52.44/36.07 | pass | pass |
| ryu | sf3twoi/art1 | 2 | new P2 | 229-11-0 | 240 (0) | 22.17/36.76 | 52.67/36.45 | pass | pass |
| ryu | sf3twoi/art1 | 3 | new P1 | 225-15-0 | 240 (0) | 20.18/34.29 | 55.69/44.10 | pass | pass |
| ryu | sf3twoi/art1 | 3 | new P2 | 218-22-0 | 240 (0) | 20.03/34.72 | 55.26/44.32 | pass | pass |
| ryu | sf3twoi/art1 | 4 | new P1 | 209-31-0 | 240 (0) | 19.10/33.26 | 61.08/50.81 | pass | pass |
| ryu | sf3twoi/art1 | 4 | new P2 | 195-45-0 | 240 (0) | 17.39/35.41 | 58.62/52.45 | pass | pass |
| ryu | sf3twoi/art1 | 2/1 | higher P1 | 215-25-0 | 240 (0) | 23.46/28.95 | 47.53/39.65 | pass | pass |
| ryu | sf3twoi/art1 | 1/2 | higher P2 | 221-19-0 | 240 (0) | 23.73/28.97 | 47.62/40.39 | pass | pass |
| ryu | sf3twoi/art1 | 3/2 | higher P1 | 189-51-0 | 240 (0) | 22.84/23.91 | 46.82/44.09 | pass | pass |
| ryu | sf3twoi/art1 | 2/3 | higher P2 | 202-38-0 | 240 (0) | 22.51/24.53 | 47.10/43.76 | pass | pass |
| ryu | sf3twoi/art1 | 4/3 | higher P1 | 184-56-0 | 240 (0) | 21.34/22.82 | 49.59/47.30 | pass | pass |
| ryu | sf3twoi/art1 | 3/4 | higher P2 | 189-51-0 | 240 (0) | 21.43/23.12 | 49.69/46.77 | pass | pass |
| ryu | sf3twoi/art2 | 4 | old vs old | 148-92-0 | 240 (0) | 30.60/27.16 | 61.53/57.88 | - | - |
| ryu | sf3twoi/art2 | 1 | new P1 | 229-11-0 | 240 (0) | 25.28/41.06 | 49.66/32.84 | pass | pass |
| ryu | sf3twoi/art2 | 1 | new P2 | 229-10-0 | 239 (1) | 25.53/40.85 | 49.14/33.27 | pass | pass |
| ryu | sf3twoi/art2 | 2 | new P1 | 234-6-0 | 240 (0) | 22.14/36.41 | 52.34/36.09 | pass | pass |
| ryu | sf3twoi/art2 | 2 | new P2 | 231-9-0 | 240 (0) | 22.17/36.76 | 52.83/36.46 | pass | pass |
| ryu | sf3twoi/art2 | 3 | new P1 | 224-16-0 | 240 (0) | 20.27/34.32 | 55.78/44.15 | pass | pass |
| ryu | sf3twoi/art2 | 3 | new P2 | 221-19-0 | 240 (0) | 19.82/34.94 | 55.61/44.41 | pass | pass |
| ryu | sf3twoi/art2 | 4 | new P1 | 213-27-0 | 240 (0) | 19.42/33.19 | 60.68/50.39 | pass | pass |
| ryu | sf3twoi/art2 | 4 | new P2 | 200-39-0 | 239 (1) | 17.43/35.80 | 58.81/52.57 | pass | pass |
| ryu | sf3twoi/art2 | 2/1 | higher P1 | 216-24-0 | 240 (0) | 23.44/28.96 | 47.56/39.64 | pass | pass |
| ryu | sf3twoi/art2 | 1/2 | higher P2 | 220-20-0 | 240 (0) | 23.74/28.98 | 47.67/40.42 | pass | pass |
| ryu | sf3twoi/art2 | 3/2 | higher P1 | 191-49-0 | 240 (0) | 22.83/23.92 | 46.81/44.07 | pass | pass |
| ryu | sf3twoi/art2 | 2/3 | higher P2 | 200-40-0 | 240 (0) | 22.50/24.53 | 47.19/43.74 | pass | pass |
| ryu | sf3twoi/art2 | 4/3 | higher P1 | 185-55-0 | 240 (0) | 21.31/22.82 | 49.58/47.25 | pass | pass |
| ryu | sf3twoi/art2 | 3/4 | higher P2 | 190-50-0 | 240 (0) | 21.39/23.11 | 49.71/46.73 | pass | pass |
| ryu | sf3twoi/art3 | 4 | old vs old | 154-86-0 | 240 (0) | 30.39/27.06 | 61.16/57.67 | - | - |
| ryu | sf3twoi/art3 | 1 | new P1 | 229-11-0 | 240 (0) | 25.28/41.06 | 49.66/32.84 | pass | pass |
| ryu | sf3twoi/art3 | 1 | new P2 | 229-10-0 | 239 (1) | 25.53/40.85 | 49.14/33.27 | pass | pass |
| ryu | sf3twoi/art3 | 2 | new P1 | 239-1-0 | 240 (0) | 22.10/36.38 | 52.37/35.97 | pass | pass |
| ryu | sf3twoi/art3 | 2 | new P2 | 231-9-0 | 240 (0) | 22.20/36.71 | 52.69/36.45 | pass | pass |
| ryu | sf3twoi/art3 | 3 | new P1 | 228-12-0 | 240 (0) | 20.20/34.23 | 55.63/43.87 | pass | pass |
| ryu | sf3twoi/art3 | 3 | new P2 | 221-19-0 | 240 (0) | 19.85/34.87 | 55.38/44.25 | pass | pass |
| ryu | sf3twoi/art3 | 4 | new P1 | 207-33-0 | 240 (0) | 19.39/33.12 | 60.40/50.41 | pass | pass |
| ryu | sf3twoi/art3 | 4 | new P2 | 201-39-0 | 240 (0) | 17.78/35.14 | 57.87/52.18 | pass | pass |
| ryu | sf3twoi/art3 | 2/1 | higher P1 | 215-25-0 | 240 (0) | 23.46/28.95 | 47.53/39.65 | pass | pass |
| ryu | sf3twoi/art3 | 1/2 | higher P2 | 221-19-0 | 240 (0) | 23.73/28.97 | 47.62/40.39 | pass | pass |
| ryu | sf3twoi/art3 | 3/2 | higher P1 | 189-51-0 | 240 (0) | 22.84/23.91 | 46.82/44.09 | pass | pass |
| ryu | sf3twoi/art3 | 2/3 | higher P2 | 202-38-0 | 240 (0) | 22.51/24.53 | 47.10/43.76 | pass | pass |
| ryu | sf3twoi/art3 | 4/3 | higher P1 | 184-56-0 | 240 (0) | 21.34/22.82 | 49.59/47.30 | pass | pass |
| ryu | sf3twoi/art3 | 3/4 | higher P2 | 189-51-0 | 240 (0) | 21.43/23.12 | 49.69/46.77 | pass | pass |
| ryu | sf3threes/art1 | 4 | old vs old | 144-96-0 | 240 (0) | 30.49/26.99 | 61.78/58.21 | - | - |
| ryu | sf3threes/art1 | 1 | new P1 | 227-13-0 | 240 (0) | 25.63/41.00 | 49.12/32.62 | pass | pass |
| ryu | sf3threes/art1 | 1 | new P2 | 226-13-0 | 239 (1) | 25.63/40.87 | 48.73/33.13 | pass | pass |
| ryu | sf3threes/art1 | 2 | new P1 | 235-5-0 | 240 (0) | 22.50/36.11 | 52.16/36.07 | pass | pass |
| ryu | sf3threes/art1 | 2 | new P2 | 232-8-0 | 240 (0) | 21.91/37.06 | 52.71/36.79 | pass | pass |
| ryu | sf3threes/art1 | 3 | new P1 | 221-19-0 | 240 (0) | 20.34/34.16 | 55.65/44.18 | pass | pass |
| ryu | sf3threes/art1 | 3 | new P2 | 211-29-0 | 240 (0) | 19.66/34.84 | 55.64/44.62 | pass | pass |
| ryu | sf3threes/art1 | 4 | new P1 | 214-26-0 | 240 (0) | 19.06/33.24 | 60.98/50.55 | pass | pass |
| ryu | sf3threes/art1 | 4 | new P2 | 191-49-0 | 240 (0) | 17.46/35.56 | 58.71/52.73 | pass | pass |
| ryu | sf3threes/art1 | 2/1 | higher P1 | 223-17-0 | 240 (0) | 23.74/29.02 | 47.72/39.53 | pass | pass |
| ryu | sf3threes/art1 | 1/2 | higher P2 | 216-24-0 | 240 (0) | 23.79/28.76 | 47.53/39.81 | pass | pass |
| ryu | sf3threes/art1 | 3/2 | higher P1 | 183-57-0 | 240 (0) | 22.85/24.21 | 47.28/44.09 | pass | pass |
| ryu | sf3threes/art1 | 2/3 | higher P2 | 191-49-0 | 240 (0) | 22.59/24.28 | 47.39/43.80 | pass | pass |
| ryu | sf3threes/art1 | 4/3 | higher P1 | 178-62-0 | 240 (0) | 21.30/22.83 | 49.41/47.28 | pass | pass |
| ryu | sf3threes/art1 | 3/4 | higher P2 | 191-49-0 | 240 (0) | 20.88/22.97 | 49.76/46.59 | pass | pass |
| ryu | sf3threes/art2 | 4 | old vs old | 152-88-0 | 240 (0) | 30.62/27.11 | 61.71/58.01 | - | - |
| ryu | sf3threes/art2 | 1 | new P1 | 227-13-0 | 240 (0) | 25.63/41.00 | 49.12/32.62 | pass | pass |
| ryu | sf3threes/art2 | 1 | new P2 | 226-13-0 | 239 (1) | 25.63/40.87 | 48.73/33.13 | pass | pass |
| ryu | sf3threes/art2 | 2 | new P1 | 235-5-0 | 240 (0) | 22.46/36.08 | 52.17/36.00 | pass | pass |
| ryu | sf3threes/art2 | 2 | new P2 | 232-8-0 | 240 (0) | 21.88/37.04 | 52.71/36.76 | pass | pass |
| ryu | sf3threes/art2 | 3 | new P1 | 223-17-0 | 240 (0) | 20.54/34.16 | 55.67/44.21 | pass | pass |
| ryu | sf3threes/art2 | 3 | new P2 | 218-22-0 | 240 (0) | 19.66/35.04 | 55.91/44.80 | pass | pass |
| ryu | sf3threes/art2 | 4 | new P1 | 212-28-0 | 240 (0) | 19.15/33.24 | 60.82/50.42 | pass | pass |
| ryu | sf3threes/art2 | 4 | new P2 | 199-40-0 | 239 (1) | 17.51/35.89 | 58.68/52.67 | pass | pass |
| ryu | sf3threes/art2 | 2/1 | higher P1 | 224-16-0 | 240 (0) | 23.73/29.04 | 47.69/39.51 | pass | pass |
| ryu | sf3threes/art2 | 1/2 | higher P2 | 217-23-0 | 240 (0) | 23.83/28.74 | 47.53/39.82 | pass | pass |
| ryu | sf3threes/art2 | 3/2 | higher P1 | 184-56-0 | 240 (0) | 22.86/24.15 | 47.27/44.08 | pass | pass |
| ryu | sf3threes/art2 | 2/3 | higher P2 | 191-49-0 | 240 (0) | 22.58/24.26 | 47.39/43.79 | pass | pass |
| ryu | sf3threes/art2 | 4/3 | higher P1 | 178-62-0 | 240 (0) | 21.29/22.81 | 49.38/47.24 | pass | pass |
| ryu | sf3threes/art2 | 3/4 | higher P2 | 192-48-0 | 240 (0) | 20.88/23.04 | 49.80/46.59 | pass | pass |
| ryu | sf3threes/art3 | 4 | old vs old | 150-90-0 | 240 (0) | 30.42/26.92 | 61.46/57.84 | - | - |
| ryu | sf3threes/art3 | 1 | new P1 | 227-13-0 | 240 (0) | 25.63/41.00 | 49.12/32.62 | pass | pass |
| ryu | sf3threes/art3 | 1 | new P2 | 226-13-0 | 239 (1) | 25.63/40.87 | 48.73/33.13 | pass | pass |
| ryu | sf3threes/art3 | 2 | new P1 | 235-5-0 | 240 (0) | 22.52/36.07 | 52.11/35.97 | pass | pass |
| ryu | sf3threes/art3 | 2 | new P2 | 232-8-0 | 240 (0) | 21.83/37.04 | 52.70/36.71 | pass | pass |
| ryu | sf3threes/art3 | 3 | new P1 | 224-16-0 | 240 (0) | 20.31/34.10 | 55.53/43.95 | pass | pass |
| ryu | sf3threes/art3 | 3 | new P2 | 220-20-0 | 240 (0) | 19.66/34.95 | 55.55/44.77 | pass | pass |
| ryu | sf3threes/art3 | 4 | new P1 | 213-27-0 | 240 (0) | 19.07/33.27 | 60.58/50.44 | pass | pass |
| ryu | sf3threes/art3 | 4 | new P2 | 201-39-0 | 240 (0) | 17.76/35.34 | 57.86/52.39 | pass | pass |
| ryu | sf3threes/art3 | 2/1 | higher P1 | 223-17-0 | 240 (0) | 23.74/29.02 | 47.72/39.53 | pass | pass |
| ryu | sf3threes/art3 | 1/2 | higher P2 | 216-24-0 | 240 (0) | 23.79/28.76 | 47.53/39.81 | pass | pass |
| ryu | sf3threes/art3 | 3/2 | higher P1 | 183-57-0 | 240 (0) | 22.85/24.21 | 47.28/44.09 | pass | pass |
| ryu | sf3threes/art3 | 2/3 | higher P2 | 191-49-0 | 240 (0) | 22.59/24.28 | 47.39/43.80 | pass | pass |
| ryu | sf3threes/art3 | 4/3 | higher P1 | 178-62-0 | 240 (0) | 21.30/22.83 | 49.41/47.28 | pass | pass |
| ryu | sf3threes/art3 | 3/4 | higher P2 | 191-49-0 | 240 (0) | 20.88/22.97 | 49.76/46.59 | pass | pass |
| ken | sf3ng/art1 | 4 | old vs old | 142-98-0 | 240 (0) | 34.99/31.20 | 58.04/56.89 | - | - |
| ken | sf3ng/art1 | 1 | new P1 | 225-15-0 | 240 (0) | 28.93/41.40 | 50.09/33.51 | pass | pass |
| ken | sf3ng/art1 | 1 | new P2 | 235-5-0 | 240 (0) | 28.93/41.31 | 49.94/32.76 | pass | pass |
| ken | sf3ng/art1 | 2 | new P1 | 227-13-0 | 240 (0) | 26.04/39.40 | 51.77/35.46 | pass | pass |
| ken | sf3ng/art1 | 2 | new P2 | 230-10-0 | 240 (0) | 25.55/39.81 | 52.25/35.48 | pass | pass |
| ken | sf3ng/art1 | 3 | new P1 | 228-12-0 | 240 (0) | 24.04/39.30 | 58.28/38.96 | pass | pass |
| ken | sf3ng/art1 | 3 | new P2 | 220-20-0 | 240 (0) | 23.07/40.24 | 57.28/40.20 | pass | pass |
| ken | sf3ng/art1 | 4 | new P1 | 223-17-0 | 240 (0) | 22.48/40.79 | 61.67/43.79 | pass | pass |
| ken | sf3ng/art1 | 4 | new P2 | 210-30-0 | 240 (0) | 21.54/42.77 | 61.06/44.73 | pass | pass |
| ken | sf3ng/art1 | 2/1 | higher P1 | 223-17-0 | 240 (0) | 28.73/31.46 | 45.20/39.89 | pass | pass |
| ken | sf3ng/art1 | 1/2 | higher P2 | 219-21-0 | 240 (0) | 28.80/31.72 | 45.54/39.35 | pass | pass |
| ken | sf3ng/art1 | 3/2 | higher P1 | 200-39-0 | 239 (1) | 27.07/28.08 | 46.55/41.86 | pass | pass |
| ken | sf3ng/art1 | 2/3 | higher P2 | 203-37-0 | 240 (0) | 27.17/27.86 | 46.58/41.84 | pass | pass |
| ken | sf3ng/art1 | 4/3 | higher P1 | 189-51-0 | 240 (0) | 24.96/25.61 | 46.84/44.98 | pass | pass |
| ken | sf3ng/art1 | 3/4 | higher P2 | 188-52-0 | 240 (0) | 25.14/25.51 | 46.66/45.20 | pass | pass |
| ken | sf3ng/art2 | 4 | old vs old | 149-91-0 | 240 (0) | 35.35/31.33 | 58.89/57.54 | - | - |
| ken | sf3ng/art2 | 1 | new P1 | 229-11-0 | 240 (0) | 28.98/41.53 | 50.19/33.68 | pass | pass |
| ken | sf3ng/art2 | 1 | new P2 | 228-12-0 | 240 (0) | 28.85/41.32 | 50.05/33.24 | pass | pass |
| ken | sf3ng/art2 | 2 | new P1 | 217-23-0 | 240 (0) | 26.14/39.21 | 51.40/35.36 | pass | pass |
| ken | sf3ng/art2 | 2 | new P2 | 231-9-0 | 240 (0) | 25.56/39.92 | 52.52/35.36 | pass | pass |
| ken | sf3ng/art2 | 3 | new P1 | 230-10-0 | 240 (0) | 23.86/39.72 | 58.91/39.69 | pass | pass |
| ken | sf3ng/art2 | 3 | new P2 | 213-27-0 | 240 (0) | 23.20/40.45 | 58.00/41.01 | pass | pass |
| ken | sf3ng/art2 | 4 | new P1 | 220-20-0 | 240 (0) | 22.55/41.10 | 62.96/44.58 | pass | pass |
| ken | sf3ng/art2 | 4 | new P2 | 207-33-0 | 240 (0) | 21.34/42.98 | 62.65/46.07 | pass | pass |
| ken | sf3ng/art2 | 2/1 | higher P1 | 217-23-0 | 240 (0) | 28.40/31.79 | 45.05/39.45 | pass | pass |
| ken | sf3ng/art2 | 1/2 | higher P2 | 215-25-0 | 240 (0) | 28.73/31.76 | 45.38/39.69 | pass | pass |
| ken | sf3ng/art2 | 3/2 | higher P1 | 203-37-0 | 240 (0) | 27.19/28.32 | 46.75/42.21 | pass | pass |
| ken | sf3ng/art2 | 2/3 | higher P2 | 196-44-0 | 240 (0) | 26.89/28.28 | 47.02/42.33 | pass | pass |
| ken | sf3ng/art2 | 4/3 | higher P1 | 193-47-0 | 240 (0) | 25.06/25.52 | 47.42/45.27 | pass | pass |
| ken | sf3ng/art2 | 3/4 | higher P2 | 183-57-0 | 240 (0) | 24.75/25.87 | 47.65/46.12 | pass | pass |
| ken | sf3ng/art3 | 4 | old vs old | 152-88-0 | 240 (0) | 35.15/31.38 | 58.27/56.76 | - | - |
| ken | sf3ng/art3 | 1 | new P1 | 236-4-0 | 240 (0) | 28.79/41.28 | 49.61/33.42 | pass | pass |
| ken | sf3ng/art3 | 1 | new P2 | 232-8-0 | 240 (0) | 29.21/41.04 | 49.76/33.67 | pass | pass |
| ken | sf3ng/art3 | 2 | new P1 | 217-23-0 | 240 (0) | 25.98/39.19 | 51.61/36.13 | pass | pass |
| ken | sf3ng/art3 | 2 | new P2 | 226-14-0 | 240 (0) | 25.44/39.77 | 52.22/35.37 | pass | pass |
| ken | sf3ng/art3 | 3 | new P1 | 220-20-0 | 240 (0) | 24.18/39.12 | 58.36/40.04 | pass | pass |
| ken | sf3ng/art3 | 3 | new P2 | 212-28-0 | 240 (0) | 23.68/39.92 | 57.11/40.17 | pass | pass |
| ken | sf3ng/art3 | 4 | new P1 | 207-33-0 | 240 (0) | 22.56/40.84 | 61.41/43.93 | pass | pass |
| ken | sf3ng/art3 | 4 | new P2 | 193-47-0 | 240 (0) | 21.77/42.14 | 60.59/44.79 | pass | pass |
| ken | sf3ng/art3 | 2/1 | higher P1 | 217-23-0 | 240 (0) | 28.44/32.02 | 45.03/39.78 | pass | pass |
| ken | sf3ng/art3 | 1/2 | higher P2 | 213-27-0 | 240 (0) | 28.47/32.22 | 45.20/39.42 | pass | pass |
| ken | sf3ng/art3 | 3/2 | higher P1 | 202-38-0 | 240 (0) | 27.03/28.17 | 46.18/41.91 | pass | pass |
| ken | sf3ng/art3 | 2/3 | higher P2 | 204-36-0 | 240 (0) | 26.99/28.29 | 46.82/42.25 | pass | pass |
| ken | sf3ng/art3 | 4/3 | higher P1 | 187-53-0 | 240 (0) | 24.76/25.46 | 46.79/45.12 | pass | pass |
| ken | sf3ng/art3 | 3/4 | higher P2 | 176-64-0 | 240 (0) | 24.65/25.44 | 46.74/45.03 | pass | pass |
| ken | sf3twoi/art1 | 4 | old vs old | 140-100-0 | 240 (0) | 34.98/31.16 | 57.90/56.82 | - | - |
| ken | sf3twoi/art1 | 1 | new P1 | 232-8-0 | 240 (0) | 28.81/41.45 | 49.53/33.33 | pass | pass |
| ken | sf3twoi/art1 | 1 | new P2 | 232-8-0 | 240 (0) | 29.21/41.02 | 49.77/33.73 | pass | pass |
| ken | sf3twoi/art1 | 2 | new P1 | 224-16-0 | 240 (0) | 26.09/39.16 | 51.81/36.10 | pass | pass |
| ken | sf3twoi/art1 | 2 | new P2 | 229-11-0 | 240 (0) | 25.53/39.73 | 52.32/35.32 | pass | pass |
| ken | sf3twoi/art1 | 3 | new P1 | 230-10-0 | 240 (0) | 24.05/39.21 | 58.36/40.08 | pass | pass |
| ken | sf3twoi/art1 | 3 | new P2 | 221-19-0 | 240 (0) | 23.64/40.00 | 57.27/39.92 | pass | pass |
| ken | sf3twoi/art1 | 4 | new P1 | 216-24-0 | 240 (0) | 22.47/40.77 | 61.43/43.58 | pass | pass |
| ken | sf3twoi/art1 | 4 | new P2 | 210-30-0 | 240 (0) | 21.78/42.22 | 61.08/45.03 | pass | pass |
| ken | sf3twoi/art1 | 2/1 | higher P1 | 218-22-0 | 240 (0) | 28.53/31.88 | 44.95/39.79 | pass | pass |
| ken | sf3twoi/art1 | 1/2 | higher P2 | 215-25-0 | 240 (0) | 28.49/32.25 | 45.21/39.34 | pass | pass |
| ken | sf3twoi/art1 | 3/2 | higher P1 | 207-33-0 | 240 (0) | 27.19/28.16 | 46.32/41.99 | pass | pass |
| ken | sf3twoi/art1 | 2/3 | higher P2 | 203-37-0 | 240 (0) | 26.96/28.29 | 46.76/42.23 | pass | pass |
| ken | sf3twoi/art1 | 4/3 | higher P1 | 187-53-0 | 240 (0) | 24.82/25.42 | 46.68/45.09 | pass | pass |
| ken | sf3twoi/art1 | 3/4 | higher P2 | 169-71-0 | 240 (0) | 24.64/25.42 | 46.68/44.86 | pass | pass |
| ken | sf3twoi/art2 | 4 | old vs old | 150-90-0 | 240 (0) | 35.34/31.31 | 58.85/57.53 | - | - |
| ken | sf3twoi/art2 | 1 | new P1 | 232-8-0 | 240 (0) | 28.81/41.45 | 49.53/33.33 | pass | pass |
| ken | sf3twoi/art2 | 1 | new P2 | 232-8-0 | 240 (0) | 29.21/41.02 | 49.77/33.73 | pass | pass |
| ken | sf3twoi/art2 | 2 | new P1 | 224-16-0 | 240 (0) | 25.95/39.38 | 51.85/36.09 | pass | pass |
| ken | sf3twoi/art2 | 2 | new P2 | 229-11-0 | 240 (0) | 25.50/39.84 | 52.37/35.37 | pass | pass |
| ken | sf3twoi/art2 | 3 | new P1 | 224-16-0 | 240 (0) | 23.88/39.48 | 58.53/40.30 | pass | pass |
| ken | sf3twoi/art2 | 3 | new P2 | 220-20-0 | 240 (0) | 23.44/40.42 | 57.62/40.43 | pass | pass |
| ken | sf3twoi/art2 | 4 | new P1 | 210-30-0 | 240 (0) | 22.38/41.36 | 62.27/44.66 | pass | pass |
| ken | sf3twoi/art2 | 4 | new P2 | 208-32-0 | 240 (0) | 21.53/42.93 | 62.13/45.88 | pass | pass |
| ken | sf3twoi/art2 | 2/1 | higher P1 | 219-21-0 | 240 (0) | 28.53/31.88 | 44.93/39.78 | pass | pass |
| ken | sf3twoi/art2 | 1/2 | higher P2 | 215-25-0 | 240 (0) | 28.48/32.28 | 45.20/39.31 | pass | pass |
| ken | sf3twoi/art2 | 3/2 | higher P1 | 206-34-0 | 240 (0) | 27.16/28.19 | 46.35/41.99 | pass | pass |
| ken | sf3twoi/art2 | 2/3 | higher P2 | 204-36-0 | 240 (0) | 26.96/28.32 | 46.79/42.24 | pass | pass |
| ken | sf3twoi/art2 | 4/3 | higher P1 | 186-54-0 | 240 (0) | 24.81/25.45 | 46.67/45.13 | pass | pass |
| ken | sf3twoi/art2 | 3/4 | higher P2 | 170-70-0 | 240 (0) | 24.67/25.43 | 46.66/44.90 | pass | pass |
| ken | sf3twoi/art3 | 4 | old vs old | 152-88-0 | 240 (0) | 35.14/31.37 | 58.26/56.76 | - | - |
| ken | sf3twoi/art3 | 1 | new P1 | 232-8-0 | 240 (0) | 28.81/41.45 | 49.53/33.33 | pass | pass |
| ken | sf3twoi/art3 | 1 | new P2 | 232-8-0 | 240 (0) | 29.21/41.02 | 49.77/33.73 | pass | pass |
| ken | sf3twoi/art3 | 2 | new P1 | 218-22-0 | 240 (0) | 25.95/39.20 | 51.65/36.10 | pass | pass |
| ken | sf3twoi/art3 | 2 | new P2 | 227-13-0 | 240 (0) | 25.48/39.75 | 52.22/35.35 | pass | pass |
| ken | sf3twoi/art3 | 3 | new P1 | 220-20-0 | 240 (0) | 24.18/39.12 | 58.36/40.04 | pass | pass |
| ken | sf3twoi/art3 | 3 | new P2 | 212-28-0 | 240 (0) | 23.68/39.93 | 57.11/40.17 | pass | pass |
| ken | sf3twoi/art3 | 4 | new P1 | 207-33-0 | 240 (0) | 22.59/40.81 | 61.40/43.96 | pass | pass |
| ken | sf3twoi/art3 | 4 | new P2 | 193-47-0 | 240 (0) | 21.81/42.09 | 60.57/44.77 | pass | pass |
| ken | sf3twoi/art3 | 2/1 | higher P1 | 218-22-0 | 240 (0) | 28.52/31.90 | 44.95/39.80 | pass | pass |
| ken | sf3twoi/art3 | 1/2 | higher P2 | 214-26-0 | 240 (0) | 28.50/32.21 | 45.15/39.39 | pass | pass |
| ken | sf3twoi/art3 | 3/2 | higher P1 | 201-39-0 | 240 (0) | 27.14/28.10 | 46.19/41.97 | pass | pass |
| ken | sf3twoi/art3 | 2/3 | higher P2 | 205-35-0 | 240 (0) | 26.96/28.34 | 46.82/42.28 | pass | pass |
| ken | sf3twoi/art3 | 4/3 | higher P1 | 187-53-0 | 240 (0) | 24.78/25.40 | 46.70/45.14 | pass | pass |
| ken | sf3twoi/art3 | 3/4 | higher P2 | 169-71-0 | 240 (0) | 24.65/25.44 | 46.66/44.89 | pass | pass |
| ken | sf3threes/art1 | 4 | old vs old | 140-100-0 | 240 (0) | 34.98/31.16 | 57.90/56.82 | - | - |
| ken | sf3threes/art1 | 1 | new P1 | 232-8-0 | 240 (0) | 28.82/41.43 | 49.52/33.34 | pass | pass |
| ken | sf3threes/art1 | 1 | new P2 | 232-8-0 | 240 (0) | 29.21/41.02 | 49.77/33.73 | pass | pass |
| ken | sf3threes/art1 | 2 | new P1 | 224-16-0 | 240 (0) | 26.09/39.16 | 51.81/36.10 | pass | pass |
| ken | sf3threes/art1 | 2 | new P2 | 229-11-0 | 240 (0) | 25.53/39.73 | 52.32/35.32 | pass | pass |
| ken | sf3threes/art1 | 3 | new P1 | 230-10-0 | 240 (0) | 24.08/39.18 | 58.34/40.08 | pass | pass |
| ken | sf3threes/art1 | 3 | new P2 | 221-19-0 | 240 (0) | 23.68/39.92 | 57.18/39.88 | pass | pass |
| ken | sf3threes/art1 | 4 | new P1 | 216-24-0 | 240 (0) | 22.54/40.65 | 61.48/43.55 | pass | pass |
| ken | sf3threes/art1 | 4 | new P2 | 210-30-0 | 240 (0) | 21.74/42.22 | 61.22/44.97 | pass | pass |
| ken | sf3threes/art1 | 2/1 | higher P1 | 218-22-0 | 240 (0) | 28.53/31.88 | 44.95/39.79 | pass | pass |
| ken | sf3threes/art1 | 1/2 | higher P2 | 215-25-0 | 240 (0) | 28.49/32.25 | 45.21/39.34 | pass | pass |
| ken | sf3threes/art1 | 3/2 | higher P1 | 207-33-0 | 240 (0) | 27.19/28.16 | 46.32/41.99 | pass | pass |
| ken | sf3threes/art1 | 2/3 | higher P2 | 203-37-0 | 240 (0) | 26.96/28.29 | 46.76/42.23 | pass | pass |
| ken | sf3threes/art1 | 4/3 | higher P1 | 187-53-0 | 240 (0) | 24.82/25.42 | 46.68/45.09 | pass | pass |
| ken | sf3threes/art1 | 3/4 | higher P2 | 169-71-0 | 240 (0) | 24.64/25.42 | 46.68/44.86 | pass | pass |
| ken | sf3threes/art2 | 4 | old vs old | 150-90-0 | 240 (0) | 35.34/31.31 | 58.85/57.53 | - | - |
| ken | sf3threes/art2 | 1 | new P1 | 232-8-0 | 240 (0) | 28.82/41.43 | 49.52/33.34 | pass | pass |
| ken | sf3threes/art2 | 1 | new P2 | 232-8-0 | 240 (0) | 29.21/41.02 | 49.77/33.73 | pass | pass |
| ken | sf3threes/art2 | 2 | new P1 | 224-16-0 | 240 (0) | 25.95/39.38 | 51.85/36.09 | pass | pass |
| ken | sf3threes/art2 | 2 | new P2 | 229-11-0 | 240 (0) | 25.50/39.84 | 52.37/35.39 | pass | pass |
| ken | sf3threes/art2 | 3 | new P1 | 224-16-0 | 240 (0) | 23.88/39.48 | 58.53/40.30 | pass | pass |
| ken | sf3threes/art2 | 3 | new P2 | 220-20-0 | 240 (0) | 23.44/40.42 | 57.62/40.43 | pass | pass |
| ken | sf3threes/art2 | 4 | new P1 | 210-30-0 | 240 (0) | 22.38/41.36 | 62.27/44.66 | pass | pass |
| ken | sf3threes/art2 | 4 | new P2 | 208-32-0 | 240 (0) | 21.53/42.93 | 62.13/45.88 | pass | pass |
| ken | sf3threes/art2 | 2/1 | higher P1 | 219-21-0 | 240 (0) | 28.53/31.88 | 44.93/39.78 | pass | pass |
| ken | sf3threes/art2 | 1/2 | higher P2 | 215-25-0 | 240 (0) | 28.48/32.28 | 45.20/39.31 | pass | pass |
| ken | sf3threes/art2 | 3/2 | higher P1 | 206-34-0 | 240 (0) | 27.16/28.19 | 46.35/41.99 | pass | pass |
| ken | sf3threes/art2 | 2/3 | higher P2 | 204-36-0 | 240 (0) | 26.96/28.32 | 46.79/42.24 | pass | pass |
| ken | sf3threes/art2 | 4/3 | higher P1 | 186-54-0 | 240 (0) | 24.81/25.45 | 46.67/45.13 | pass | pass |
| ken | sf3threes/art2 | 3/4 | higher P2 | 170-70-0 | 240 (0) | 24.67/25.43 | 46.66/44.90 | pass | pass |
| ken | sf3threes/art3 | 4 | old vs old | 152-88-0 | 240 (0) | 35.14/31.37 | 58.26/56.76 | - | - |
| ken | sf3threes/art3 | 1 | new P1 | 232-8-0 | 240 (0) | 28.82/41.43 | 49.52/33.34 | pass | pass |
| ken | sf3threes/art3 | 1 | new P2 | 232-8-0 | 240 (0) | 29.21/41.02 | 49.77/33.73 | pass | pass |
| ken | sf3threes/art3 | 2 | new P1 | 218-22-0 | 240 (0) | 25.95/39.20 | 51.65/36.10 | pass | pass |
| ken | sf3threes/art3 | 2 | new P2 | 227-13-0 | 240 (0) | 25.48/39.75 | 52.22/35.35 | pass | pass |
| ken | sf3threes/art3 | 3 | new P1 | 220-20-0 | 240 (0) | 24.18/39.12 | 58.36/40.04 | pass | pass |
| ken | sf3threes/art3 | 3 | new P2 | 212-28-0 | 240 (0) | 23.68/39.93 | 57.11/40.17 | pass | pass |
| ken | sf3threes/art3 | 4 | new P1 | 207-33-0 | 240 (0) | 22.59/40.81 | 61.40/43.96 | pass | pass |
| ken | sf3threes/art3 | 4 | new P2 | 193-47-0 | 240 (0) | 21.81/42.09 | 60.57/44.77 | pass | pass |
| ken | sf3threes/art3 | 2/1 | higher P1 | 218-22-0 | 240 (0) | 28.52/31.90 | 44.95/39.80 | pass | pass |
| ken | sf3threes/art3 | 1/2 | higher P2 | 214-26-0 | 240 (0) | 28.50/32.21 | 45.15/39.39 | pass | pass |
| ken | sf3threes/art3 | 3/2 | higher P1 | 201-39-0 | 240 (0) | 27.14/28.10 | 46.19/41.97 | pass | pass |
| ken | sf3threes/art3 | 2/3 | higher P2 | 205-35-0 | 240 (0) | 26.96/28.34 | 46.82/42.28 | pass | pass |
| ken | sf3threes/art3 | 4/3 | higher P1 | 187-53-0 | 240 (0) | 24.78/25.40 | 46.70/45.14 | pass | pass |
| ken | sf3threes/art3 | 3/4 | higher P2 | 169-71-0 | 240 (0) | 24.65/25.44 | 46.66/44.89 | pass | pass |
| retsu | sf3threes/art1 | 4 | old vs old | 118-120-0 | 238 (2) | 49.72/50.04 | 38.39/38.13 | - | - |
| retsu | sf3threes/art1 | 1 | new P1 | 214-24-0 | 238 (2) | 39.81/53.64 | 43.01/26.73 | pass | pass |
| retsu | sf3threes/art1 | 1 | new P2 | 209-30-0 | 239 (1) | 39.44/53.16 | 43.26/26.97 | pass | pass |
| retsu | sf3threes/art1 | 2 | new P1 | 177-62-0 | 239 (1) | 35.02/48.95 | 45.83/33.78 | pass | pass |
| retsu | sf3threes/art1 | 2 | new P2 | 192-47-0 | 239 (1) | 34.64/49.29 | 46.23/33.80 | pass | pass |
| retsu | sf3threes/art1 | 3 | new P1 | 149-90-0 | 239 (1) | 33.56/46.50 | 45.09/38.49 | pass | pass |
| retsu | sf3threes/art1 | 3 | new P2 | 149-89-0 | 238 (2) | 33.66/46.14 | 45.17/39.06 | pass | pass |
| retsu | sf3threes/art1 | 4 | new P1 | 217-23-0 | 240 (0) | 34.90/46.51 | 45.81/38.53 | pass | pass |
| retsu | sf3threes/art1 | 4 | new P2 | 227-13-0 | 240 (0) | 34.05/47.02 | 45.95/38.25 | pass | pass |
| retsu | sf3threes/art1 | 2/1 | higher P1 | 195-44-0 | 239 (1) | 36.58/41.27 | 42.63/33.31 | pass | pass |
| retsu | sf3threes/art1 | 1/2 | higher P2 | 204-35-0 | 239 (1) | 36.67/41.73 | 42.59/32.45 | pass | pass |
| retsu | sf3threes/art1 | 3/2 | higher P1 | 161-78-0 | 239 (1) | 33.48/35.20 | 42.81/39.00 | pass | pass |
| retsu | sf3threes/art1 | 2/3 | higher P2 | 136-102-0 | 238 (2) | 33.16/35.14 | 42.58/39.55 | pass | pass |
| retsu | sf3threes/art1 | 4/3 | higher P1 | 151-89-0 | 240 (0) | 30.94/32.21 | 44.81/43.42 | pass | pass |
| retsu | sf3threes/art1 | 3/4 | higher P2 | 143-97-0 | 240 (0) | 30.58/31.91 | 45.87/43.76 | pass | pass |
| retsu | sf3threes/art2 | 4 | old vs old | 126-112-0 | 238 (2) | 49.46/49.89 | 38.74/38.06 | - | - |
| retsu | sf3threes/art2 | 1 | new P1 | 214-24-0 | 238 (2) | 39.81/53.64 | 43.01/26.73 | pass | pass |
| retsu | sf3threes/art2 | 1 | new P2 | 209-30-0 | 239 (1) | 39.44/53.16 | 43.26/26.97 | pass | pass |
| retsu | sf3threes/art2 | 2 | new P1 | 169-70-0 | 239 (1) | 34.86/48.76 | 46.04/33.98 | pass | pass |
| retsu | sf3threes/art2 | 2 | new P2 | 189-50-0 | 239 (1) | 34.51/49.28 | 46.23/33.69 | pass | pass |
| retsu | sf3threes/art2 | 3 | new P1 | 151-88-0 | 239 (1) | 33.57/46.47 | 45.01/38.53 | pass | pass |
| retsu | sf3threes/art2 | 3 | new P2 | 147-92-0 | 239 (1) | 33.64/46.20 | 45.14/39.03 | pass | pass |
| retsu | sf3threes/art2 | 4 | new P1 | 215-25-0 | 240 (0) | 34.87/46.51 | 45.89/38.50 | pass | pass |
| retsu | sf3threes/art2 | 4 | new P2 | 226-14-0 | 240 (0) | 34.00/47.07 | 45.99/38.28 | pass | pass |
| retsu | sf3threes/art2 | 2/1 | higher P1 | 196-43-0 | 239 (1) | 36.58/41.26 | 42.68/33.33 | pass | pass |
| retsu | sf3threes/art2 | 1/2 | higher P2 | 205-34-0 | 239 (1) | 36.68/41.72 | 42.60/32.45 | pass | pass |
| retsu | sf3threes/art2 | 3/2 | higher P1 | 160-79-0 | 239 (1) | 33.49/35.16 | 42.79/38.99 | pass | pass |
| retsu | sf3threes/art2 | 2/3 | higher P2 | 137-101-0 | 238 (2) | 33.16/35.13 | 42.56/39.60 | pass | pass |
| retsu | sf3threes/art2 | 4/3 | higher P1 | 147-93-0 | 240 (0) | 30.93/32.19 | 44.76/43.38 | pass | pass |
| retsu | sf3threes/art2 | 3/4 | higher P2 | 142-98-0 | 240 (0) | 30.58/31.89 | 45.89/43.84 | pass | pass |
| retsu | sf3threes/art3 | 4 | old vs old | 119-119-0 | 238 (2) | 49.49/49.94 | 38.33/37.84 | - | - |
| retsu | sf3threes/art3 | 1 | new P1 | 214-24-0 | 238 (2) | 39.81/53.64 | 43.01/26.73 | pass | pass |
| retsu | sf3threes/art3 | 1 | new P2 | 209-30-0 | 239 (1) | 39.44/53.16 | 43.26/26.97 | pass | pass |
| retsu | sf3threes/art3 | 2 | new P1 | 177-62-0 | 239 (1) | 34.82/48.87 | 46.02/33.79 | pass | pass |
| retsu | sf3threes/art3 | 2 | new P2 | 186-53-0 | 239 (1) | 34.59/49.23 | 46.23/33.81 | pass | pass |
| retsu | sf3threes/art3 | 3 | new P1 | 155-84-0 | 239 (1) | 33.59/46.46 | 45.10/38.50 | pass | pass |
| retsu | sf3threes/art3 | 3 | new P2 | 144-93-0 | 237 (3) | 33.69/46.23 | 45.24/39.10 | pass | pass |
| retsu | sf3threes/art3 | 4 | new P1 | 217-23-0 | 240 (0) | 34.88/46.54 | 45.86/38.44 | pass | pass |
| retsu | sf3threes/art3 | 4 | new P2 | 226-14-0 | 240 (0) | 34.12/47.01 | 45.88/38.42 | pass | pass |
| retsu | sf3threes/art3 | 2/1 | higher P1 | 195-44-0 | 239 (1) | 36.56/41.26 | 42.68/33.37 | pass | pass |
| retsu | sf3threes/art3 | 1/2 | higher P2 | 205-34-0 | 239 (1) | 36.68/41.71 | 42.61/32.46 | pass | pass |
| retsu | sf3threes/art3 | 3/2 | higher P1 | 161-78-0 | 239 (1) | 33.48/35.15 | 42.80/39.00 | pass | pass |
| retsu | sf3threes/art3 | 2/3 | higher P2 | 137-101-0 | 238 (2) | 33.15/35.13 | 42.59/39.60 | pass | pass |
| retsu | sf3threes/art3 | 4/3 | higher P1 | 147-93-0 | 240 (0) | 30.93/32.19 | 44.78/43.38 | pass | pass |
| retsu | sf3threes/art3 | 3/4 | higher P2 | 143-97-0 | 240 (0) | 30.57/31.89 | 45.89/43.83 | pass | pass |
| geki | sf3threes/art1 | 4 | old vs old | 122-118-0 | 240 (0) | 38.95/38.88 | 51.67/51.36 | - | - |
| geki | sf3threes/art1 | 1 | new P1 | 223-17-0 | 240 (0) | 35.40/46.65 | 46.76/33.89 | pass | pass |
| geki | sf3threes/art1 | 1 | new P2 | 216-24-0 | 240 (0) | 35.26/47.17 | 46.79/33.86 | pass | pass |
| geki | sf3threes/art1 | 2 | new P1 | 227-13-0 | 240 (0) | 32.03/42.03 | 49.57/37.71 | pass | pass |
| geki | sf3threes/art1 | 2 | new P2 | 223-17-0 | 240 (0) | 32.00/42.60 | 50.00/37.76 | pass | pass |
| geki | sf3threes/art1 | 3 | new P1 | 222-18-0 | 240 (0) | 29.64/38.15 | 51.73/41.77 | pass | pass |
| geki | sf3threes/art1 | 3 | new P2 | 231-9-0 | 240 (0) | 29.26/38.89 | 52.03/41.32 | pass | pass |
| geki | sf3threes/art1 | 4 | new P1 | 216-24-0 | 240 (0) | 29.42/39.99 | 54.29/41.80 | pass | pass |
| geki | sf3threes/art1 | 4 | new P2 | 226-14-0 | 240 (0) | 29.16/39.47 | 54.57/42.42 | pass | pass |
| geki | sf3threes/art1 | 2/1 | higher P1 | 175-65-0 | 240 (0) | 35.35/37.42 | 46.78/40.77 | pass | pass |
| geki | sf3threes/art1 | 1/2 | higher P2 | 175-65-0 | 240 (0) | 34.97/37.97 | 46.92/40.21 | pass | pass |
| geki | sf3threes/art1 | 3/2 | higher P1 | 169-70-0 | 239 (1) | 31.55/33.93 | 49.01/45.05 | pass | pass |
| geki | sf3threes/art1 | 2/3 | higher P2 | 168-71-0 | 239 (1) | 31.55/34.17 | 48.25/45.32 | pass | pass |
| geki | sf3threes/art1 | 4/3 | higher P1 | 145-95-0 | 240 (0) | 29.39/30.38 | 49.69/47.27 | pass | pass |
| geki | sf3threes/art1 | 3/4 | higher P2 | 142-98-0 | 240 (0) | 28.91/30.94 | 49.96/47.36 | pass | pass |
| geki | sf3threes/art2 | 4 | old vs old | 113-127-0 | 240 (0) | 38.72/38.80 | 51.99/52.04 | - | - |
| geki | sf3threes/art2 | 1 | new P1 | 223-17-0 | 240 (0) | 35.40/46.65 | 46.76/33.89 | pass | pass |
| geki | sf3threes/art2 | 1 | new P2 | 216-24-0 | 240 (0) | 35.26/47.17 | 46.79/33.86 | pass | pass |
| geki | sf3threes/art2 | 2 | new P1 | 216-24-0 | 240 (0) | 31.93/42.07 | 49.45/37.53 | pass | pass |
| geki | sf3threes/art2 | 2 | new P2 | 217-23-0 | 240 (0) | 31.92/42.42 | 49.85/37.79 | pass | pass |
| geki | sf3threes/art2 | 3 | new P1 | 220-20-0 | 240 (0) | 29.64/38.18 | 51.76/41.56 | pass | pass |
| geki | sf3threes/art2 | 3 | new P2 | 230-10-0 | 240 (0) | 29.30/38.85 | 51.94/41.33 | pass | pass |
| geki | sf3threes/art2 | 4 | new P1 | 220-20-0 | 240 (0) | 29.09/39.81 | 54.41/41.88 | pass | pass |
| geki | sf3threes/art2 | 4 | new P2 | 227-13-0 | 240 (0) | 29.13/39.43 | 54.94/42.21 | pass | pass |
| geki | sf3threes/art2 | 2/1 | higher P1 | 174-66-0 | 240 (0) | 35.33/37.38 | 46.75/40.75 | pass | pass |
| geki | sf3threes/art2 | 1/2 | higher P2 | 182-58-0 | 240 (0) | 34.84/37.95 | 46.89/40.22 | pass | pass |
| geki | sf3threes/art2 | 3/2 | higher P1 | 167-72-0 | 239 (1) | 31.52/33.94 | 48.73/44.88 | pass | pass |
| geki | sf3threes/art2 | 2/3 | higher P2 | 157-82-0 | 239 (1) | 31.54/34.15 | 48.25/45.33 | pass | pass |
| geki | sf3threes/art2 | 4/3 | higher P1 | 147-92-0 | 239 (1) | 29.22/30.34 | 49.64/47.19 | pass | pass |
| geki | sf3threes/art2 | 3/4 | higher P2 | 146-94-0 | 240 (0) | 28.74/30.89 | 49.86/47.10 | pass | pass |
| geki | sf3threes/art3 | 4 | old vs old | 110-130-0 | 240 (0) | 38.91/38.84 | 51.78/51.78 | - | - |
| geki | sf3threes/art3 | 1 | new P1 | 223-17-0 | 240 (0) | 35.40/46.65 | 46.76/33.89 | pass | pass |
| geki | sf3threes/art3 | 1 | new P2 | 216-24-0 | 240 (0) | 35.26/47.17 | 46.79/33.86 | pass | pass |
| geki | sf3threes/art3 | 2 | new P1 | 218-22-0 | 240 (0) | 31.84/42.11 | 49.53/37.41 | pass | pass |
| geki | sf3threes/art3 | 2 | new P2 | 219-21-0 | 240 (0) | 31.97/42.47 | 50.07/37.73 | pass | pass |
| geki | sf3threes/art3 | 3 | new P1 | 220-20-0 | 240 (0) | 29.61/38.17 | 51.68/41.57 | pass | pass |
| geki | sf3threes/art3 | 3 | new P2 | 230-10-0 | 240 (0) | 29.31/38.85 | 51.86/41.42 | pass | pass |
| geki | sf3threes/art3 | 4 | new P1 | 221-19-0 | 240 (0) | 29.14/39.60 | 54.32/41.90 | pass | pass |
| geki | sf3threes/art3 | 4 | new P2 | 228-12-0 | 240 (0) | 29.05/39.40 | 55.02/42.29 | pass | pass |
| geki | sf3threes/art3 | 2/1 | higher P1 | 174-66-0 | 240 (0) | 35.33/37.38 | 46.76/40.77 | pass | pass |
| geki | sf3threes/art3 | 1/2 | higher P2 | 182-58-0 | 240 (0) | 34.85/37.92 | 46.94/40.22 | pass | pass |
| geki | sf3threes/art3 | 3/2 | higher P1 | 165-74-0 | 239 (1) | 31.53/33.94 | 48.79/44.89 | pass | pass |
| geki | sf3threes/art3 | 2/3 | higher P2 | 159-80-0 | 239 (1) | 31.58/34.15 | 48.19/45.29 | pass | pass |
| geki | sf3threes/art3 | 4/3 | higher P1 | 149-90-0 | 239 (1) | 29.25/30.28 | 49.62/47.21 | pass | pass |
| geki | sf3threes/art3 | 3/4 | higher P2 | 145-95-0 | 240 (0) | 28.69/30.89 | 49.85/47.11 | pass | pass |
| joe | sf3threes/art1 | 4 | old vs old | 137-103-0 | 240 (0) | 42.95/42.96 | 41.36/41.03 | - | - |
| joe | sf3threes/art1 | 1 | new P1 | 233-7-0 | 240 (0) | 32.15/46.04 | 49.58/28.23 | pass | pass |
| joe | sf3threes/art1 | 1 | new P2 | 235-5-0 | 240 (0) | 31.63/45.89 | 50.27/28.24 | pass | pass |
| joe | sf3threes/art1 | 2 | new P1 | 223-16-0 | 239 (1) | 27.22/41.45 | 54.40/33.16 | pass | pass |
| joe | sf3threes/art1 | 2 | new P2 | 218-22-0 | 240 (0) | 27.16/41.31 | 54.93/33.94 | pass | pass |
| joe | sf3threes/art1 | 3 | new P1 | 214-25-0 | 239 (1) | 24.17/39.86 | 59.11/36.88 | pass | pass |
| joe | sf3threes/art1 | 3 | new P2 | 220-20-0 | 240 (0) | 24.11/39.87 | 60.42/37.47 | pass | pass |
| joe | sf3threes/art1 | 4 | new P1 | 205-35-0 | 240 (0) | 23.19/40.72 | 62.19/37.35 | pass | pass |
| joe | sf3threes/art1 | 4 | new P2 | 217-23-0 | 240 (0) | 22.78/41.26 | 63.59/37.50 | pass | pass |
| joe | sf3threes/art1 | 2/1 | higher P1 | 169-70-0 | 239 (1) | 30.89/35.41 | 45.96/41.25 | pass | pass |
| joe | sf3threes/art1 | 1/2 | higher P2 | 155-85-0 | 240 (0) | 30.60/35.29 | 46.33/41.80 | pass | pass |
| joe | sf3threes/art1 | 3/2 | higher P1 | 112-127-0 | 239 (1) | 28.79/31.18 | 46.36/45.44 | MISS (wins) | miss |
| joe | sf3threes/art1 | 2/3 | higher P2 | 113-125-0 | 238 (2) | 28.92/31.35 | 46.27/46.07 | MISS (wins) | miss |
| joe | sf3threes/art1 | 4/3 | higher P1 | 132-106-0 | 238 (2) | 27.87/28.03 | 48.04/48.26 | pass | pass |
| joe | sf3threes/art1 | 3/4 | higher P2 | 135-105-0 | 240 (0) | 27.79/28.32 | 48.25/48.49 | pass | pass |
| joe | sf3threes/art2 | 4 | old vs old | 128-112-0 | 240 (0) | 42.70/42.96 | 41.68/41.51 | - | - |
| joe | sf3threes/art2 | 1 | new P1 | 233-7-0 | 240 (0) | 32.15/46.04 | 49.58/28.23 | pass | pass |
| joe | sf3threes/art2 | 1 | new P2 | 235-5-0 | 240 (0) | 31.63/45.89 | 50.27/28.24 | pass | pass |
| joe | sf3threes/art2 | 2 | new P1 | 226-13-0 | 239 (1) | 27.13/41.28 | 54.55/33.32 | pass | pass |
| joe | sf3threes/art2 | 2 | new P2 | 219-21-0 | 240 (0) | 27.10/41.29 | 54.83/33.95 | pass | pass |
| joe | sf3threes/art2 | 3 | new P1 | 212-27-0 | 239 (1) | 24.13/39.72 | 59.15/37.46 | pass | pass |
| joe | sf3threes/art2 | 3 | new P2 | 211-29-0 | 240 (0) | 24.16/40.06 | 59.87/37.02 | pass | pass |
| joe | sf3threes/art2 | 4 | new P1 | 198-42-0 | 240 (0) | 23.08/40.75 | 62.06/38.22 | pass | pass |
| joe | sf3threes/art2 | 4 | new P2 | 214-26-0 | 240 (0) | 22.53/41.12 | 63.84/37.67 | pass | pass |
| joe | sf3threes/art2 | 2/1 | higher P1 | 163-76-0 | 239 (1) | 30.88/35.31 | 45.79/41.31 | pass | pass |
| joe | sf3threes/art2 | 1/2 | higher P2 | 153-87-0 | 240 (0) | 30.58/35.29 | 46.32/41.82 | pass | pass |
| joe | sf3threes/art2 | 3/2 | higher P1 | 112-128-0 | 240 (0) | 28.89/31.11 | 46.13/45.54 | MISS (wins) | miss |
| joe | sf3threes/art2 | 2/3 | higher P2 | 107-131-0 | 238 (2) | 28.87/31.45 | 46.38/46.12 | MISS (wins) | miss |
| joe | sf3threes/art2 | 4/3 | higher P1 | 130-109-0 | 239 (1) | 27.89/28.03 | 47.77/48.38 | pass | pass |
| joe | sf3threes/art2 | 3/4 | higher P2 | 139-101-0 | 240 (0) | 27.76/28.38 | 48.16/48.48 | pass | pass |
| joe | sf3threes/art3 | 4 | old vs old | 126-114-0 | 240 (0) | 42.64/42.79 | 41.83/42.05 | - | - |
| joe | sf3threes/art3 | 1 | new P1 | 233-7-0 | 240 (0) | 32.15/46.04 | 49.58/28.23 | pass | pass |
| joe | sf3threes/art3 | 1 | new P2 | 235-5-0 | 240 (0) | 31.63/45.89 | 50.27/28.24 | pass | pass |
| joe | sf3threes/art3 | 2 | new P1 | 228-11-0 | 239 (1) | 27.22/41.32 | 54.29/33.11 | pass | pass |
| joe | sf3threes/art3 | 2 | new P2 | 221-19-0 | 240 (0) | 27.09/41.34 | 55.09/33.73 | pass | pass |
| joe | sf3threes/art3 | 3 | new P1 | 212-28-0 | 240 (0) | 24.06/39.78 | 59.13/37.20 | pass | pass |
| joe | sf3threes/art3 | 3 | new P2 | 214-26-0 | 240 (0) | 24.14/40.11 | 59.92/37.41 | pass | pass |
| joe | sf3threes/art3 | 4 | new P1 | 202-38-0 | 240 (0) | 23.11/40.78 | 62.39/37.93 | pass | pass |
| joe | sf3threes/art3 | 4 | new P2 | 216-24-0 | 240 (0) | 22.66/41.28 | 63.76/37.77 | pass | pass |
| joe | sf3threes/art3 | 2/1 | higher P1 | 164-75-0 | 239 (1) | 30.86/35.37 | 45.89/41.16 | pass | pass |
| joe | sf3threes/art3 | 1/2 | higher P2 | 152-88-0 | 240 (0) | 30.60/35.35 | 46.22/41.77 | pass | pass |
| joe | sf3threes/art3 | 3/2 | higher P1 | 116-124-0 | 240 (0) | 28.91/31.04 | 46.30/45.51 | MISS (wins) | miss |
| joe | sf3threes/art3 | 2/3 | higher P2 | 110-128-0 | 238 (2) | 28.84/31.44 | 46.45/46.11 | MISS (wins) | miss |
| joe | sf3threes/art3 | 4/3 | higher P1 | 131-108-0 | 239 (1) | 27.92/28.02 | 47.72/48.42 | pass | pass |
| joe | sf3threes/art3 | 3/4 | higher P2 | 139-101-0 | 240 (0) | 27.69/28.44 | 48.25/48.38 | pass | pass |
| mike | sf3threes/art1 | 4 | old vs old | 132-108-0 | 240 (0) | 44.08/43.69 | 40.68/40.07 | - | - |
| mike | sf3threes/art1 | 1 | new P1 | 188-51-0 | 239 (1) | 35.85/50.65 | 38.88/23.29 | pass | pass |
| mike | sf3threes/art1 | 1 | new P2 | 185-55-0 | 240 (0) | 36.03/50.80 | 38.18/23.04 | pass | pass |
| mike | sf3threes/art1 | 2 | new P1 | 173-66-0 | 239 (1) | 34.17/45.56 | 41.56/30.09 | pass | pass |
| mike | sf3threes/art1 | 2 | new P2 | 168-72-0 | 240 (0) | 34.44/45.27 | 40.78/30.12 | pass | pass |
| mike | sf3threes/art1 | 3 | new P1 | 164-76-0 | 240 (0) | 29.23/42.98 | 47.88/34.19 | pass | pass |
| mike | sf3threes/art1 | 3 | new P2 | 142-98-0 | 240 (0) | 28.74/43.33 | 47.73/36.35 | pass | pass |
| mike | sf3threes/art1 | 4 | new P1 | 236-4-0 | 240 (0) | 28.63/43.17 | 57.08/26.87 | pass | pass |
| mike | sf3threes/art1 | 4 | new P2 | 231-9-0 | 240 (0) | 27.42/43.95 | 56.50/28.75 | pass | pass |
| mike | sf3threes/art1 | 2/1 | higher P1 | 217-23-0 | 240 (0) | 38.11/37.07 | 36.29/32.88 | pass | pass |
| mike | sf3threes/art1 | 1/2 | higher P2 | 210-29-0 | 239 (1) | 38.70/37.26 | 35.18/32.57 | pass | pass |
| mike | sf3threes/art1 | 3/2 | higher P1 | 179-61-0 | 240 (0) | 33.57/34.55 | 38.70/35.61 | pass | pass |
| mike | sf3threes/art1 | 2/3 | higher P2 | 187-52-0 | 239 (1) | 33.55/34.55 | 39.04/35.22 | pass | pass |
| mike | sf3threes/art1 | 4/3 | higher P1 | 197-43-0 | 240 (0) | 31.42/30.05 | 42.20/36.93 | pass | pass |
| mike | sf3threes/art1 | 3/4 | higher P2 | 193-47-0 | 240 (0) | 31.33/30.16 | 41.89/36.48 | pass | pass |
| mike | sf3threes/art2 | 4 | old vs old | 124-116-0 | 240 (0) | 43.95/43.52 | 40.61/40.18 | - | - |
| mike | sf3threes/art2 | 1 | new P1 | 188-51-0 | 239 (1) | 35.85/50.65 | 38.88/23.29 | pass | pass |
| mike | sf3threes/art2 | 1 | new P2 | 185-55-0 | 240 (0) | 36.03/50.80 | 38.18/23.04 | pass | pass |
| mike | sf3threes/art2 | 2 | new P1 | 170-69-0 | 239 (1) | 34.00/45.51 | 41.64/30.01 | pass | pass |
| mike | sf3threes/art2 | 2 | new P2 | 165-75-0 | 240 (0) | 34.34/45.27 | 40.74/29.87 | pass | pass |
| mike | sf3threes/art2 | 3 | new P1 | 172-68-0 | 240 (0) | 29.12/42.88 | 47.93/33.84 | pass | pass |
| mike | sf3threes/art2 | 3 | new P2 | 151-89-0 | 240 (0) | 28.54/43.32 | 47.78/35.86 | pass | pass |
| mike | sf3threes/art2 | 4 | new P1 | 234-6-0 | 240 (0) | 28.41/43.16 | 56.96/26.92 | pass | pass |
| mike | sf3threes/art2 | 4 | new P2 | 227-13-0 | 240 (0) | 27.25/43.87 | 56.82/28.93 | pass | pass |
| mike | sf3threes/art2 | 2/1 | higher P1 | 217-23-0 | 240 (0) | 38.11/37.06 | 36.30/32.89 | pass | pass |
| mike | sf3threes/art2 | 1/2 | higher P2 | 210-29-0 | 239 (1) | 38.70/37.25 | 35.16/32.58 | pass | pass |
| mike | sf3threes/art2 | 3/2 | higher P1 | 180-60-0 | 240 (0) | 33.56/34.55 | 38.69/35.61 | pass | pass |
| mike | sf3threes/art2 | 2/3 | higher P2 | 187-52-0 | 239 (1) | 33.55/34.55 | 39.04/35.22 | pass | pass |
| mike | sf3threes/art2 | 4/3 | higher P1 | 197-43-0 | 240 (0) | 31.43/30.05 | 42.19/36.89 | pass | pass |
| mike | sf3threes/art2 | 3/4 | higher P2 | 194-46-0 | 240 (0) | 31.33/30.15 | 41.91/36.44 | pass | pass |
| mike | sf3threes/art3 | 4 | old vs old | 138-102-0 | 240 (0) | 43.89/43.54 | 40.95/40.14 | - | - |
| mike | sf3threes/art3 | 1 | new P1 | 188-51-0 | 239 (1) | 35.85/50.65 | 38.88/23.29 | pass | pass |
| mike | sf3threes/art3 | 1 | new P2 | 185-55-0 | 240 (0) | 36.03/50.80 | 38.18/23.04 | pass | pass |
| mike | sf3threes/art3 | 2 | new P1 | 173-66-0 | 239 (1) | 34.04/45.42 | 41.68/30.07 | pass | pass |
| mike | sf3threes/art3 | 2 | new P2 | 168-72-0 | 240 (0) | 34.39/45.19 | 40.84/29.83 | pass | pass |
| mike | sf3threes/art3 | 3 | new P1 | 175-65-0 | 240 (0) | 29.32/42.89 | 47.82/33.85 | pass | pass |
| mike | sf3threes/art3 | 3 | new P2 | 143-97-0 | 240 (0) | 28.38/43.29 | 47.97/35.99 | pass | pass |
| mike | sf3threes/art3 | 4 | new P1 | 235-5-0 | 240 (0) | 28.41/43.15 | 56.75/26.99 | pass | pass |
| mike | sf3threes/art3 | 4 | new P2 | 228-12-0 | 240 (0) | 27.30/43.88 | 56.73/29.03 | pass | pass |
| mike | sf3threes/art3 | 2/1 | higher P1 | 216-24-0 | 240 (0) | 38.12/37.07 | 36.28/32.90 | pass | pass |
| mike | sf3threes/art3 | 1/2 | higher P2 | 210-29-0 | 239 (1) | 38.70/37.25 | 35.20/32.57 | pass | pass |
| mike | sf3threes/art3 | 3/2 | higher P1 | 180-60-0 | 240 (0) | 33.58/34.53 | 38.68/35.59 | pass | pass |
| mike | sf3threes/art3 | 2/3 | higher P2 | 186-53-0 | 239 (1) | 33.54/34.55 | 39.02/35.20 | pass | pass |
| mike | sf3threes/art3 | 4/3 | higher P1 | 196-44-0 | 240 (0) | 31.41/30.06 | 42.20/36.96 | pass | pass |
| mike | sf3threes/art3 | 3/4 | higher P2 | 194-46-0 | 240 (0) | 31.33/30.17 | 41.91/36.45 | pass | pass |
| lee | sf3threes/art1 | 4 | old vs old | 130-110-0 | 240 (0) | 46.30/46.80 | 43.22/42.59 | - | - |
| lee | sf3threes/art1 | 1 | new P1 | 215-25-0 | 240 (0) | 52.01/62.79 | 32.91/20.11 | pass | pass |
| lee | sf3threes/art1 | 1 | new P2 | 211-29-0 | 240 (0) | 53.15/63.64 | 31.77/19.22 | pass | pass |
| lee | sf3threes/art1 | 2 | new P1 | 196-41-0 | 237 (3) | 40.73/55.18 | 43.91/28.75 | pass | pass |
| lee | sf3threes/art1 | 2 | new P2 | 215-25-0 | 240 (0) | 40.85/55.65 | 43.69/28.70 | pass | pass |
| lee | sf3threes/art1 | 3 | new P1 | 193-47-0 | 240 (0) | 31.91/51.00 | 51.50/38.03 | pass | pass |
| lee | sf3threes/art1 | 3 | new P2 | 192-48-0 | 240 (0) | 32.72/51.72 | 51.10/37.90 | pass | pass |
| lee | sf3threes/art1 | 4 | new P1 | 216-23-0 | 239 (1) | 28.59/50.09 | 57.40/39.07 | pass | pass |
| lee | sf3threes/art1 | 4 | new P2 | 213-27-0 | 240 (0) | 28.63/50.38 | 57.28/39.40 | pass | pass |
| lee | sf3threes/art1 | 2/1 | higher P1 | 187-49-0 | 236 (4) | 48.11/53.57 | 37.12/30.24 | pass | pass |
| lee | sf3threes/art1 | 1/2 | higher P2 | 177-62-0 | 239 (1) | 48.25/53.91 | 36.89/29.83 | pass | pass |
| lee | sf3threes/art1 | 3/2 | higher P1 | 162-72-0 | 234 (6) | 44.60/48.66 | 39.02/34.04 | pass | pass |
| lee | sf3threes/art1 | 2/3 | higher P2 | 152-85-0 | 237 (3) | 44.62/47.92 | 38.95/34.82 | pass | pass |
| lee | sf3threes/art1 | 4/3 | higher P1 | 143-94-0 | 237 (3) | 42.03/44.37 | 41.09/37.55 | pass | pass |
| lee | sf3threes/art1 | 3/4 | higher P2 | 132-102-0 | 234 (6) | 42.23/44.07 | 40.82/37.54 | pass | pass |
| lee | sf3threes/art2 | 4 | old vs old | 143-97-0 | 240 (0) | 46.02/46.86 | 43.60/42.58 | - | - |
| lee | sf3threes/art2 | 1 | new P1 | 215-25-0 | 240 (0) | 52.01/62.79 | 32.91/20.11 | pass | pass |
| lee | sf3threes/art2 | 1 | new P2 | 211-29-0 | 240 (0) | 53.15/63.64 | 31.77/19.22 | pass | pass |
| lee | sf3threes/art2 | 2 | new P1 | 196-41-0 | 237 (3) | 40.63/55.02 | 43.98/28.95 | pass | pass |
| lee | sf3threes/art2 | 2 | new P2 | 217-23-0 | 240 (0) | 40.90/55.81 | 43.48/28.56 | pass | pass |
| lee | sf3threes/art2 | 3 | new P1 | 182-58-0 | 240 (0) | 32.13/51.07 | 51.23/37.74 | pass | pass |
| lee | sf3threes/art2 | 3 | new P2 | 194-46-0 | 240 (0) | 32.61/51.58 | 51.32/38.05 | pass | pass |
| lee | sf3threes/art2 | 4 | new P1 | 227-12-0 | 239 (1) | 28.40/50.19 | 57.44/38.97 | pass | pass |
| lee | sf3threes/art2 | 4 | new P2 | 222-18-0 | 240 (0) | 28.47/50.48 | 57.42/39.34 | pass | pass |
| lee | sf3threes/art2 | 2/1 | higher P1 | 184-52-0 | 236 (4) | 48.11/53.58 | 37.08/30.24 | pass | pass |
| lee | sf3threes/art2 | 1/2 | higher P2 | 179-60-0 | 239 (1) | 48.30/53.96 | 36.87/29.79 | pass | pass |
| lee | sf3threes/art2 | 3/2 | higher P1 | 162-72-0 | 234 (6) | 44.68/48.69 | 38.99/33.96 | pass | pass |
| lee | sf3threes/art2 | 2/3 | higher P2 | 151-86-0 | 237 (3) | 44.68/47.92 | 38.93/34.81 | pass | pass |
| lee | sf3threes/art2 | 4/3 | higher P1 | 142-95-0 | 237 (3) | 42.04/44.45 | 41.15/37.52 | pass | pass |
| lee | sf3threes/art2 | 3/4 | higher P2 | 133-101-0 | 234 (6) | 42.22/44.03 | 40.81/37.62 | pass | pass |
| lee | sf3threes/art3 | 4 | old vs old | 145-95-0 | 240 (0) | 46.18/46.80 | 43.43/42.91 | - | - |
| lee | sf3threes/art3 | 1 | new P1 | 215-25-0 | 240 (0) | 52.01/62.79 | 32.91/20.11 | pass | pass |
| lee | sf3threes/art3 | 1 | new P2 | 211-29-0 | 240 (0) | 53.15/63.64 | 31.77/19.22 | pass | pass |
| lee | sf3threes/art3 | 2 | new P1 | 190-47-0 | 237 (3) | 40.56/54.97 | 44.13/29.15 | pass | pass |
| lee | sf3threes/art3 | 2 | new P2 | 213-27-0 | 240 (0) | 40.96/55.70 | 43.56/28.71 | pass | pass |
| lee | sf3threes/art3 | 3 | new P1 | 188-52-0 | 240 (0) | 32.01/50.92 | 51.44/38.09 | pass | pass |
| lee | sf3threes/art3 | 3 | new P2 | 189-51-0 | 240 (0) | 32.58/51.63 | 51.15/38.29 | pass | pass |
| lee | sf3threes/art3 | 4 | new P1 | 219-20-0 | 239 (1) | 28.43/50.11 | 57.66/38.98 | pass | pass |
| lee | sf3threes/art3 | 4 | new P2 | 221-19-0 | 240 (0) | 28.76/50.39 | 57.34/39.59 | pass | pass |
| lee | sf3threes/art3 | 2/1 | higher P1 | 187-49-0 | 236 (4) | 48.11/53.57 | 37.12/30.24 | pass | pass |
| lee | sf3threes/art3 | 1/2 | higher P2 | 179-60-0 | 239 (1) | 48.26/53.94 | 36.90/29.79 | pass | pass |
| lee | sf3threes/art3 | 3/2 | higher P1 | 163-71-0 | 234 (6) | 44.60/48.64 | 39.01/34.04 | pass | pass |
| lee | sf3threes/art3 | 2/3 | higher P2 | 154-83-0 | 237 (3) | 44.63/47.90 | 38.93/34.80 | pass | pass |
| lee | sf3threes/art3 | 4/3 | higher P1 | 138-99-0 | 237 (3) | 42.00/44.36 | 41.14/37.61 | pass | pass |
| lee | sf3threes/art3 | 3/4 | higher P2 | 130-104-0 | 234 (6) | 42.23/44.10 | 40.79/37.52 | pass | pass |
| gen | sf3threes/art1 | 4 | old vs old | 132-108-0 | 240 (0) | 43.41/43.22 | 57.29/57.86 | - | - |
| gen | sf3threes/art1 | 1 | new P1 | 192-41-0 | 233 (7) | 46.02/53.16 | 40.90/39.07 | pass | pass |
| gen | sf3threes/art1 | 1 | new P2 | 188-42-0 | 230 (10) | 45.90/53.42 | 40.79/38.60 | pass | pass |
| gen | sf3threes/art1 | 2 | new P1 | 170-62-0 | 232 (8) | 44.63/45.46 | 46.52/49.31 | pass | pass |
| gen | sf3threes/art1 | 2 | new P2 | 157-79-0 | 236 (4) | 44.89/45.83 | 45.63/49.22 | pass | pass |
| gen | sf3threes/art1 | 3 | new P1 | 176-62-0 | 238 (2) | 41.57/42.51 | 49.57/54.23 | pass | pass |
| gen | sf3threes/art1 | 3 | new P2 | 164-73-0 | 237 (3) | 42.04/42.74 | 49.24/55.27 | pass | pass |
| gen | sf3threes/art1 | 4 | new P1 | 213-27-0 | 240 (0) | 39.58/42.29 | 53.40/55.73 | pass | pass |
| gen | sf3threes/art1 | 4 | new P2 | 205-32-0 | 237 (3) | 40.35/42.61 | 52.25/55.72 | pass | pass |
| gen | sf3threes/art1 | 2/1 | higher P1 | 169-57-0 | 226 (14) | 45.17/45.11 | 45.78/40.53 | pass | pass |
| gen | sf3threes/art1 | 1/2 | higher P2 | 177-60-0 | 237 (3) | 45.15/45.26 | 45.66/40.93 | pass | pass |
| gen | sf3threes/art1 | 3/2 | higher P1 | 167-69-0 | 236 (4) | 43.51/42.80 | 51.29/48.99 | pass | pass |
| gen | sf3threes/art1 | 2/3 | higher P2 | 170-62-0 | 232 (8) | 42.99/43.27 | 51.42/48.94 | pass | pass |
| gen | sf3threes/art1 | 4/3 | higher P1 | 150-88-0 | 238 (2) | 42.25/40.12 | 52.19/53.34 | pass | pass |
| gen | sf3threes/art1 | 3/4 | higher P2 | 143-95-0 | 238 (2) | 41.71/40.33 | 52.52/53.98 | pass | pass |
| gen | sf3threes/art2 | 4 | old vs old | 121-119-0 | 240 (0) | 43.44/43.16 | 57.15/57.99 | - | - |
| gen | sf3threes/art2 | 1 | new P1 | 204-28-0 | 232 (8) | 45.88/53.72 | 41.02/37.96 | pass | pass |
| gen | sf3threes/art2 | 1 | new P2 | 201-30-0 | 231 (9) | 45.74/53.51 | 41.41/38.66 | pass | pass |
| gen | sf3threes/art2 | 2 | new P1 | 187-49-0 | 236 (4) | 44.26/45.47 | 46.93/48.93 | pass | pass |
| gen | sf3threes/art2 | 2 | new P2 | 166-69-0 | 235 (5) | 44.47/45.67 | 46.79/49.38 | pass | pass |
| gen | sf3threes/art2 | 3 | new P1 | 189-49-0 | 238 (2) | 41.11/42.90 | 50.66/54.33 | pass | pass |
| gen | sf3threes/art2 | 3 | new P2 | 206-32-0 | 238 (2) | 40.50/43.42 | 51.47/53.19 | pass | pass |
| gen | sf3threes/art2 | 4 | new P1 | 220-19-0 | 239 (1) | 39.65/42.38 | 54.64/55.01 | pass | pass |
| gen | sf3threes/art2 | 4 | new P2 | 219-21-0 | 240 (0) | 39.90/42.76 | 53.78/54.95 | pass | pass |
| gen | sf3threes/art2 | 2/1 | higher P1 | 157-66-0 | 223 (17) | 44.66/45.16 | 46.22/40.80 | pass | pass |
| gen | sf3threes/art2 | 1/2 | higher P2 | 163-72-0 | 235 (5) | 44.74/44.83 | 46.62/41.26 | pass | pass |
| gen | sf3threes/art2 | 3/2 | higher P1 | 165-70-0 | 235 (5) | 42.59/42.60 | 52.39/49.14 | pass | pass |
| gen | sf3threes/art2 | 2/3 | higher P2 | 165-72-0 | 237 (3) | 42.64/42.83 | 51.55/49.08 | pass | pass |
| gen | sf3threes/art2 | 4/3 | higher P1 | 150-89-0 | 239 (1) | 42.13/39.85 | 52.60/53.77 | pass | pass |
| gen | sf3threes/art2 | 3/4 | higher P2 | 132-105-0 | 237 (3) | 42.21/40.10 | 52.05/54.47 | pass | pass |
| gen | sf3threes/art3 | 4 | old vs old | 125-115-0 | 240 (0) | 43.46/43.28 | 57.31/57.92 | - | - |
| gen | sf3threes/art3 | 1 | new P1 | 204-28-0 | 232 (8) | 45.88/53.72 | 41.03/37.98 | pass | pass |
| gen | sf3threes/art3 | 1 | new P2 | 201-30-0 | 231 (9) | 45.74/53.51 | 41.41/38.66 | pass | pass |
| gen | sf3threes/art3 | 2 | new P1 | 183-52-0 | 235 (5) | 44.15/45.61 | 47.12/49.69 | pass | pass |
| gen | sf3threes/art3 | 2 | new P2 | 180-56-0 | 236 (4) | 44.15/45.89 | 47.25/49.66 | pass | pass |
| gen | sf3threes/art3 | 3 | new P1 | 194-43-0 | 237 (3) | 41.10/42.96 | 50.88/55.26 | pass | pass |
| gen | sf3threes/art3 | 3 | new P2 | 199-40-0 | 239 (1) | 40.71/43.51 | 51.28/53.79 | pass | pass |
| gen | sf3threes/art3 | 4 | new P1 | 213-26-0 | 239 (1) | 39.53/42.35 | 54.71/55.89 | pass | pass |
| gen | sf3threes/art3 | 4 | new P2 | 220-20-0 | 240 (0) | 39.57/42.85 | 54.37/55.83 | pass | pass |
| gen | sf3threes/art3 | 2/1 | higher P1 | 147-74-0 | 221 (19) | 44.73/44.90 | 46.32/40.98 | pass | pass |
| gen | sf3threes/art3 | 1/2 | higher P2 | 157-77-0 | 234 (6) | 44.69/44.79 | 46.79/41.43 | pass | pass |
| gen | sf3threes/art3 | 3/2 | higher P1 | 168-65-0 | 233 (7) | 42.78/42.76 | 52.64/48.97 | pass | pass |
| gen | sf3threes/art3 | 2/3 | higher P2 | 154-80-0 | 234 (6) | 42.76/43.05 | 51.39/49.02 | pass | pass |
| gen | sf3threes/art3 | 4/3 | higher P1 | 145-93-0 | 238 (2) | 41.98/39.91 | 53.23/53.74 | pass | pass |
| gen | sf3threes/art3 | 3/4 | higher P2 | 128-109-0 | 237 (3) | 41.85/40.10 | 52.33/54.79 | pass | pass |
| birdie | sf3threes/art1 | 4 | old vs old | 124-116-0 | 240 (0) | 36.86/36.56 | 50.00/50.60 | - | - |
| birdie | sf3threes/art1 | 1 | new P1 | 222-18-0 | 240 (0) | 37.99/42.01 | 42.33/34.76 | pass | pass |
| birdie | sf3threes/art1 | 1 | new P2 | 219-21-0 | 240 (0) | 38.42/41.91 | 41.68/34.41 | pass | pass |
| birdie | sf3threes/art1 | 2 | new P1 | 211-25-0 | 236 (4) | 34.30/38.38 | 49.88/38.29 | pass | pass |
| birdie | sf3threes/art1 | 2 | new P2 | 211-26-0 | 237 (3) | 34.73/38.44 | 49.55/38.68 | pass | pass |
| birdie | sf3threes/art1 | 3 | new P1 | 213-26-0 | 239 (1) | 32.83/36.58 | 53.88/41.37 | pass | pass |
| birdie | sf3threes/art1 | 3 | new P2 | 218-21-0 | 239 (1) | 32.71/36.92 | 53.96/40.62 | pass | pass |
| birdie | sf3threes/art1 | 4 | new P1 | 227-12-0 | 239 (1) | 31.02/36.28 | 57.31/41.72 | pass | pass |
| birdie | sf3threes/art1 | 4 | new P2 | 229-11-0 | 240 (0) | 30.76/36.97 | 58.31/41.69 | pass | pass |
| birdie | sf3threes/art1 | 2/1 | higher P1 | 174-63-0 | 237 (3) | 34.61/38.46 | 46.67/39.43 | pass | pass |
| birdie | sf3threes/art1 | 1/2 | higher P2 | 181-59-0 | 240 (0) | 34.41/38.35 | 46.19/38.97 | pass | pass |
| birdie | sf3threes/art1 | 3/2 | higher P1 | 178-57-0 | 235 (5) | 31.92/34.32 | 50.27/44.92 | pass | pass |
| birdie | sf3threes/art1 | 2/3 | higher P2 | 169-65-0 | 234 (6) | 32.10/34.33 | 50.41/45.54 | pass | pass |
| birdie | sf3threes/art1 | 4/3 | higher P1 | 182-55-0 | 237 (3) | 30.38/32.31 | 53.74/51.51 | pass | pass |
| birdie | sf3threes/art1 | 3/4 | higher P2 | 172-65-0 | 237 (3) | 31.31/32.43 | 52.43/51.20 | pass | pass |
| birdie | sf3threes/art2 | 4 | old vs old | 128-112-0 | 240 (0) | 36.80/36.66 | 50.17/50.65 | - | - |
| birdie | sf3threes/art2 | 1 | new P1 | 220-20-0 | 240 (0) | 37.94/41.98 | 41.45/34.57 | pass | pass |
| birdie | sf3threes/art2 | 1 | new P2 | 213-27-0 | 240 (0) | 38.03/41.91 | 42.25/34.63 | pass | pass |
| birdie | sf3threes/art2 | 2 | new P1 | 209-30-0 | 239 (1) | 34.72/38.53 | 49.45/38.62 | pass | pass |
| birdie | sf3threes/art2 | 2 | new P2 | 202-38-0 | 240 (0) | 34.81/38.27 | 48.24/39.07 | pass | pass |
| birdie | sf3threes/art2 | 3 | new P1 | 192-47-0 | 239 (1) | 32.86/36.74 | 53.86/41.85 | pass | pass |
| birdie | sf3threes/art2 | 3 | new P2 | 201-38-0 | 239 (1) | 32.27/36.82 | 53.81/41.55 | pass | pass |
| birdie | sf3threes/art2 | 4 | new P1 | 224-14-0 | 238 (2) | 30.92/36.12 | 58.72/41.38 | pass | pass |
| birdie | sf3threes/art2 | 4 | new P2 | 233-7-0 | 240 (0) | 30.96/36.59 | 58.74/41.34 | pass | pass |
| birdie | sf3threes/art2 | 2/1 | higher P1 | 178-62-0 | 240 (0) | 34.72/38.45 | 46.15/38.54 | pass | pass |
| birdie | sf3threes/art2 | 1/2 | higher P2 | 162-77-0 | 239 (1) | 34.69/38.26 | 46.09/38.97 | pass | pass |
| birdie | sf3threes/art2 | 3/2 | higher P1 | 183-57-0 | 240 (0) | 31.96/34.26 | 51.89/44.81 | pass | pass |
| birdie | sf3threes/art2 | 2/3 | higher P2 | 170-69-0 | 239 (1) | 32.64/33.73 | 50.64/46.34 | pass | pass |
| birdie | sf3threes/art2 | 4/3 | higher P1 | 195-45-0 | 240 (0) | 30.16/31.62 | 55.92/54.22 | pass | pass |
| birdie | sf3threes/art2 | 3/4 | higher P2 | 189-51-0 | 240 (0) | 30.95/31.51 | 54.49/53.53 | pass | pass |
| birdie | sf3threes/art3 | 4 | old vs old | 126-113-0 | 239 (1) | 36.90/36.59 | 49.94/50.59 | - | - |
| birdie | sf3threes/art3 | 1 | new P1 | 222-18-0 | 240 (0) | 37.99/42.01 | 42.33/34.76 | pass | pass |
| birdie | sf3threes/art3 | 1 | new P2 | 219-21-0 | 240 (0) | 38.42/41.91 | 41.68/34.41 | pass | pass |
| birdie | sf3threes/art3 | 2 | new P1 | 215-24-0 | 239 (1) | 34.26/38.49 | 49.61/38.26 | pass | pass |
| birdie | sf3threes/art3 | 2 | new P2 | 218-22-0 | 240 (0) | 34.80/38.41 | 49.09/38.76 | pass | pass |
| birdie | sf3threes/art3 | 3 | new P1 | 212-28-0 | 240 (0) | 33.14/36.79 | 53.46/41.45 | pass | pass |
| birdie | sf3threes/art3 | 3 | new P2 | 211-29-0 | 240 (0) | 33.02/36.84 | 53.20/41.09 | pass | pass |
| birdie | sf3threes/art3 | 4 | new P1 | 225-14-0 | 239 (1) | 31.22/36.26 | 56.96/41.89 | pass | pass |
| birdie | sf3threes/art3 | 4 | new P2 | 229-11-0 | 240 (0) | 30.81/37.02 | 58.24/41.79 | pass | pass |
| birdie | sf3threes/art3 | 2/1 | higher P1 | 176-64-0 | 240 (0) | 34.76/38.55 | 46.70/39.33 | pass | pass |
| birdie | sf3threes/art3 | 1/2 | higher P2 | 182-58-0 | 240 (0) | 34.46/38.39 | 46.31/38.68 | pass | pass |
| birdie | sf3threes/art3 | 3/2 | higher P1 | 183-57-0 | 240 (0) | 32.10/34.41 | 50.06/44.72 | pass | pass |
| birdie | sf3threes/art3 | 2/3 | higher P2 | 174-66-0 | 240 (0) | 32.18/34.37 | 50.36/45.61 | pass | pass |
| birdie | sf3threes/art3 | 4/3 | higher P1 | 191-49-0 | 240 (0) | 30.43/32.40 | 54.06/51.26 | pass | pass |
| birdie | sf3threes/art3 | 3/4 | higher P2 | 180-60-0 | 240 (0) | 31.16/32.42 | 52.78/51.45 | pass | pass |
| eagle | sf3threes/art1 | 4 | old vs old | 112-120-0 | 232 (8) | 49.30/48.30 | 40.01/39.10 | - | - |
| eagle | sf3threes/art1 | 1 | new P1 | 214-22-0 | 236 (4) | 48.31/58.63 | 34.55/21.29 | pass | pass |
| eagle | sf3threes/art1 | 1 | new P2 | 223-16-0 | 239 (1) | 47.98/58.84 | 35.29/21.28 | pass | pass |
| eagle | sf3threes/art1 | 2 | new P1 | 212-21-0 | 233 (7) | 44.96/50.90 | 45.16/28.83 | pass | pass |
| eagle | sf3threes/art1 | 2 | new P2 | 219-16-0 | 235 (5) | 44.77/51.18 | 45.27/29.01 | pass | pass |
| eagle | sf3threes/art1 | 3 | new P1 | 204-26-0 | 230 (10) | 40.92/48.25 | 50.46/31.87 | pass | pass |
| eagle | sf3threes/art1 | 3 | new P2 | 224-6-0 | 230 (10) | 40.76/48.84 | 49.22/31.74 | pass | pass |
| eagle | sf3threes/art1 | 4 | new P1 | 228-9-0 | 237 (3) | 37.99/47.62 | 56.39/30.85 | pass | pass |
| eagle | sf3threes/art1 | 4 | new P2 | 220-9-0 | 229 (11) | 38.39/49.85 | 55.09/31.58 | pass | pass |
| eagle | sf3threes/art1 | 2/1 | higher P1 | 184-52-0 | 236 (4) | 48.08/43.33 | 40.50/34.58 | pass | pass |
| eagle | sf3threes/art1 | 1/2 | higher P2 | 183-50-0 | 233 (7) | 48.00/43.59 | 40.05/34.25 | pass | pass |
| eagle | sf3threes/art1 | 3/2 | higher P1 | 178-57-0 | 235 (5) | 43.72/42.41 | 43.45/38.58 | pass | pass |
| eagle | sf3threes/art1 | 2/3 | higher P2 | 168-64-0 | 232 (8) | 43.55/42.48 | 43.06/38.38 | pass | pass |
| eagle | sf3threes/art1 | 4/3 | higher P1 | 193-44-0 | 237 (3) | 41.26/40.63 | 44.64/42.77 | pass | pass |
| eagle | sf3threes/art1 | 3/4 | higher P2 | 185-52-0 | 237 (3) | 41.03/40.69 | 44.24/42.44 | pass | pass |
| eagle | sf3threes/art2 | 4 | old vs old | 123-110-0 | 233 (7) | 48.52/47.32 | 40.38/39.46 | - | - |
| eagle | sf3threes/art2 | 1 | new P1 | 218-21-0 | 239 (1) | 48.15/58.45 | 34.90/21.44 | pass | pass |
| eagle | sf3threes/art2 | 1 | new P2 | 215-23-0 | 238 (2) | 48.27/58.71 | 35.08/21.62 | pass | pass |
| eagle | sf3threes/art2 | 2 | new P1 | 207-27-0 | 234 (6) | 44.90/50.18 | 45.21/29.60 | pass | pass |
| eagle | sf3threes/art2 | 2 | new P2 | 212-23-0 | 235 (5) | 45.18/51.08 | 44.83/28.84 | pass | pass |
| eagle | sf3threes/art2 | 3 | new P1 | 214-18-0 | 232 (8) | 40.42/47.91 | 49.95/31.43 | pass | pass |
| eagle | sf3threes/art2 | 3 | new P2 | 219-18-0 | 237 (3) | 40.94/48.57 | 49.27/32.07 | pass | pass |
| eagle | sf3threes/art2 | 4 | new P1 | 219-17-0 | 236 (4) | 38.47/46.88 | 56.20/31.16 | pass | pass |
| eagle | sf3threes/art2 | 4 | new P2 | 221-13-0 | 234 (6) | 38.37/49.28 | 55.50/32.16 | pass | pass |
| eagle | sf3threes/art2 | 2/1 | higher P1 | 186-46-0 | 232 (8) | 48.12/43.39 | 40.15/34.59 | pass | pass |
| eagle | sf3threes/art2 | 1/2 | higher P2 | 170-67-0 | 237 (3) | 48.05/43.18 | 39.47/34.76 | pass | pass |
| eagle | sf3threes/art2 | 3/2 | higher P1 | 185-52-0 | 237 (3) | 43.60/42.36 | 43.61/38.11 | pass | pass |
| eagle | sf3threes/art2 | 2/3 | higher P2 | 177-62-0 | 239 (1) | 43.37/42.18 | 43.32/38.64 | pass | pass |
| eagle | sf3threes/art2 | 4/3 | higher P1 | 178-60-0 | 238 (2) | 41.08/40.50 | 44.42/43.17 | pass | pass |
| eagle | sf3threes/art2 | 3/4 | higher P2 | 182-56-0 | 238 (2) | 40.82/40.70 | 44.47/42.54 | pass | pass |
| eagle | sf3threes/art3 | 4 | old vs old | 128-103-0 | 231 (9) | 48.66/47.93 | 40.77/39.48 | - | - |
| eagle | sf3threes/art3 | 1 | new P1 | 214-22-0 | 236 (4) | 48.31/58.63 | 34.55/21.29 | pass | pass |
| eagle | sf3threes/art3 | 1 | new P2 | 223-16-0 | 239 (1) | 47.98/58.84 | 35.29/21.28 | pass | pass |
| eagle | sf3threes/art3 | 2 | new P1 | 221-14-0 | 235 (5) | 44.95/50.90 | 45.18/28.92 | pass | pass |
| eagle | sf3threes/art3 | 2 | new P2 | 215-17-0 | 232 (8) | 44.81/51.11 | 45.29/29.25 | pass | pass |
| eagle | sf3threes/art3 | 3 | new P1 | 211-19-0 | 230 (10) | 41.15/48.27 | 50.14/31.83 | pass | pass |
| eagle | sf3threes/art3 | 3 | new P2 | 220-14-0 | 234 (6) | 40.84/48.68 | 49.03/32.11 | pass | pass |
| eagle | sf3threes/art3 | 4 | new P1 | 231-7-0 | 238 (2) | 37.89/47.44 | 56.41/31.20 | pass | pass |
| eagle | sf3threes/art3 | 4 | new P2 | 223-7-0 | 230 (10) | 38.06/49.82 | 54.97/31.83 | pass | pass |
| eagle | sf3threes/art3 | 2/1 | higher P1 | 185-50-0 | 235 (5) | 47.87/43.34 | 40.82/34.67 | pass | pass |
| eagle | sf3threes/art3 | 1/2 | higher P2 | 182-52-0 | 234 (6) | 47.94/43.43 | 39.91/34.49 | pass | pass |
| eagle | sf3threes/art3 | 3/2 | higher P1 | 169-66-0 | 235 (5) | 43.56/42.38 | 43.54/38.58 | pass | pass |
| eagle | sf3threes/art3 | 2/3 | higher P2 | 168-68-0 | 236 (4) | 43.57/42.45 | 43.11/38.90 | pass | pass |
| eagle | sf3threes/art3 | 4/3 | higher P1 | 189-47-0 | 236 (4) | 41.21/40.54 | 44.62/42.93 | pass | pass |
| eagle | sf3threes/art3 | 3/4 | higher P2 | 175-63-0 | 238 (2) | 41.03/40.72 | 44.22/42.69 | pass | pass |
| adon | sf3threes/art1 | 4 | old vs old | 117-118-0 | 235 (5) | 48.03/48.28 | 36.71/36.82 | - | - |
| adon | sf3threes/art1 | 1 | new P1 | 231-9-0 | 240 (0) | 40.54/50.81 | 39.60/24.68 | pass | pass |
| adon | sf3threes/art1 | 1 | new P2 | 223-16-0 | 239 (1) | 41.12/50.76 | 38.92/24.66 | pass | pass |
| adon | sf3threes/art1 | 2 | new P1 | 223-17-0 | 240 (0) | 33.76/48.75 | 48.68/27.93 | pass | pass |
| adon | sf3threes/art1 | 2 | new P2 | 229-11-0 | 240 (0) | 33.41/48.91 | 48.80/28.03 | pass | pass |
| adon | sf3threes/art1 | 3 | new P1 | 226-14-0 | 240 (0) | 28.85/49.37 | 54.36/30.66 | pass | pass |
| adon | sf3threes/art1 | 3 | new P2 | 229-11-0 | 240 (0) | 28.80/49.23 | 53.66/31.25 | pass | pass |
| adon | sf3threes/art1 | 4 | new P1 | 240-0-0 | 240 (0) | 24.22/50.87 | 59.93/30.02 | pass | pass |
| adon | sf3threes/art1 | 4 | new P2 | 240-0-0 | 240 (0) | 25.38/50.99 | 59.47/29.72 | pass | pass |
| adon | sf3threes/art1 | 2/1 | higher P1 | 208-32-0 | 240 (0) | 36.92/42.31 | 40.18/30.87 | pass | pass |
| adon | sf3threes/art1 | 1/2 | higher P2 | 208-32-0 | 240 (0) | 37.45/42.04 | 40.32/31.82 | pass | pass |
| adon | sf3threes/art1 | 3/2 | higher P1 | 179-61-0 | 240 (0) | 34.10/38.15 | 42.96/37.83 | pass | pass |
| adon | sf3threes/art1 | 2/3 | higher P2 | 165-75-0 | 240 (0) | 34.36/38.07 | 42.93/38.18 | pass | pass |
| adon | sf3threes/art1 | 4/3 | higher P1 | 163-77-0 | 240 (0) | 32.05/35.15 | 46.04/42.51 | pass | pass |
| adon | sf3threes/art1 | 3/4 | higher P2 | 154-86-0 | 240 (0) | 31.98/34.82 | 46.77/42.58 | pass | pass |
| adon | sf3threes/art2 | 4 | old vs old | 128-110-0 | 238 (2) | 48.07/47.78 | 36.24/36.71 | - | - |
| adon | sf3threes/art2 | 1 | new P1 | 231-9-0 | 240 (0) | 40.54/50.81 | 39.60/24.68 | pass | pass |
| adon | sf3threes/art2 | 1 | new P2 | 223-16-0 | 239 (1) | 41.12/50.76 | 38.92/24.66 | pass | pass |
| adon | sf3threes/art2 | 2 | new P1 | 226-14-0 | 240 (0) | 33.67/48.78 | 48.85/27.88 | pass | pass |
| adon | sf3threes/art2 | 2 | new P2 | 225-15-0 | 240 (0) | 33.40/48.87 | 48.78/27.97 | pass | pass |
| adon | sf3threes/art2 | 3 | new P1 | 224-16-0 | 240 (0) | 28.82/49.61 | 54.56/30.37 | pass | pass |
| adon | sf3threes/art2 | 3 | new P2 | 225-15-0 | 240 (0) | 28.88/49.21 | 53.63/31.26 | pass | pass |
| adon | sf3threes/art2 | 4 | new P1 | 240-0-0 | 240 (0) | 24.02/50.77 | 60.64/29.59 | pass | pass |
| adon | sf3threes/art2 | 4 | new P2 | 239-1-0 | 240 (0) | 24.93/50.87 | 60.42/29.28 | pass | pass |
| adon | sf3threes/art2 | 2/1 | higher P1 | 205-35-0 | 240 (0) | 36.89/42.32 | 40.12/30.90 | pass | pass |
| adon | sf3threes/art2 | 1/2 | higher P2 | 203-37-0 | 240 (0) | 37.51/42.04 | 40.16/31.77 | pass | pass |
| adon | sf3threes/art2 | 3/2 | higher P1 | 177-63-0 | 240 (0) | 34.18/38.19 | 43.10/37.82 | pass | pass |
| adon | sf3threes/art2 | 2/3 | higher P2 | 173-67-0 | 240 (0) | 34.30/37.99 | 43.16/38.14 | pass | pass |
| adon | sf3threes/art2 | 4/3 | higher P1 | 159-81-0 | 240 (0) | 32.02/35.00 | 46.15/42.55 | pass | pass |
| adon | sf3threes/art2 | 3/4 | higher P2 | 163-77-0 | 240 (0) | 31.71/34.90 | 47.06/42.48 | pass | pass |
| adon | sf3threes/art3 | 4 | old vs old | 111-126-0 | 237 (3) | 48.19/47.89 | 37.18/37.67 | - | - |
| adon | sf3threes/art3 | 1 | new P1 | 232-8-0 | 240 (0) | 40.45/50.86 | 39.57/24.39 | pass | pass |
| adon | sf3threes/art3 | 1 | new P2 | 226-14-0 | 240 (0) | 41.12/50.88 | 39.18/24.45 | pass | pass |
| adon | sf3threes/art3 | 2 | new P1 | 229-11-0 | 240 (0) | 33.18/49.05 | 49.31/28.02 | pass | pass |
| adon | sf3threes/art3 | 2 | new P2 | 223-15-0 | 238 (2) | 33.37/48.95 | 49.48/28.44 | pass | pass |
| adon | sf3threes/art3 | 3 | new P1 | 227-13-0 | 240 (0) | 29.11/49.20 | 54.00/30.76 | pass | pass |
| adon | sf3threes/art3 | 3 | new P2 | 229-11-0 | 240 (0) | 28.61/49.52 | 54.29/30.61 | pass | pass |
| adon | sf3threes/art3 | 4 | new P1 | 239-0-0 | 239 (1) | 24.00/51.17 | 60.91/30.09 | pass | pass |
| adon | sf3threes/art3 | 4 | new P2 | 237-2-0 | 239 (1) | 24.69/51.19 | 60.42/30.13 | pass | pass |
| adon | sf3threes/art3 | 2/1 | higher P1 | 206-34-0 | 240 (0) | 36.89/42.11 | 40.91/31.72 | pass | pass |
| adon | sf3threes/art3 | 1/2 | higher P2 | 209-30-0 | 239 (1) | 37.66/42.06 | 39.92/31.50 | pass | pass |
| adon | sf3threes/art3 | 3/2 | higher P1 | 178-62-0 | 240 (0) | 33.92/38.54 | 42.88/38.04 | pass | pass |
| adon | sf3threes/art3 | 2/3 | higher P2 | 160-80-0 | 240 (0) | 34.10/37.98 | 43.30/38.53 | pass | pass |
| adon | sf3threes/art3 | 4/3 | higher P1 | 155-85-0 | 240 (0) | 31.94/35.06 | 46.39/42.33 | pass | pass |
| adon | sf3threes/art3 | 3/4 | higher P2 | 154-84-0 | 238 (2) | 31.74/34.89 | 46.41/43.01 | pass | pass |
| sagat | sf3threes/art1 | 4 | old vs old | 123-117-0 | 240 (0) | 45.45/45.61 | 46.73/46.44 | - | - |
| sagat | sf3threes/art1 | 1 | new P1 | 221-19-0 | 240 (0) | 41.61/50.76 | 41.03/29.31 | pass | pass |
| sagat | sf3threes/art1 | 1 | new P2 | 207-33-0 | 240 (0) | 41.45/50.74 | 41.44/29.47 | pass | pass |
| sagat | sf3threes/art1 | 2 | new P1 | 217-20-0 | 237 (3) | 39.32/45.47 | 47.10/33.63 | pass | pass |
| sagat | sf3threes/art1 | 2 | new P2 | 210-29-0 | 239 (1) | 39.21/45.84 | 46.51/34.01 | pass | pass |
| sagat | sf3threes/art1 | 3 | new P1 | 219-20-0 | 239 (1) | 36.30/43.13 | 51.97/35.67 | pass | pass |
| sagat | sf3threes/art1 | 3 | new P2 | 225-14-0 | 239 (1) | 36.03/43.33 | 52.55/36.38 | pass | pass |
| sagat | sf3threes/art1 | 4 | new P1 | 219-21-0 | 240 (0) | 35.21/45.36 | 54.00/33.90 | pass | pass |
| sagat | sf3threes/art1 | 4 | new P2 | 209-31-0 | 240 (0) | 34.91/45.48 | 53.86/34.69 | pass | pass |
| sagat | sf3threes/art1 | 2/1 | higher P1 | 215-18-0 | 233 (7) | 40.35/42.57 | 43.81/36.36 | pass | pass |
| sagat | sf3threes/art1 | 1/2 | higher P2 | 206-29-0 | 235 (5) | 40.05/42.69 | 43.83/36.86 | pass | pass |
| sagat | sf3threes/art1 | 3/2 | higher P1 | 191-41-0 | 232 (8) | 36.21/40.54 | 48.81/42.67 | pass | pass |
| sagat | sf3threes/art1 | 2/3 | higher P2 | 179-48-0 | 227 (13) | 36.61/40.36 | 48.54/43.60 | pass | pass |
| sagat | sf3threes/art1 | 4/3 | higher P1 | 180-48-0 | 228 (12) | 35.35/37.79 | 51.13/47.21 | pass | pass |
| sagat | sf3threes/art1 | 3/4 | higher P2 | 190-43-0 | 233 (7) | 35.29/38.38 | 51.87/47.24 | pass | pass |
| sagat | sf3threes/art2 | 4 | old vs old | 120-120-0 | 240 (0) | 45.50/45.44 | 46.64/46.75 | - | - |
| sagat | sf3threes/art2 | 1 | new P1 | 221-19-0 | 240 (0) | 41.47/50.64 | 41.49/29.13 | pass | pass |
| sagat | sf3threes/art2 | 1 | new P2 | 216-24-0 | 240 (0) | 41.60/50.91 | 41.08/28.78 | pass | pass |
| sagat | sf3threes/art2 | 2 | new P1 | 226-12-0 | 238 (2) | 38.87/45.82 | 47.61/33.48 | pass | pass |
| sagat | sf3threes/art2 | 2 | new P2 | 222-13-0 | 235 (5) | 39.22/45.72 | 46.76/34.07 | pass | pass |
| sagat | sf3threes/art2 | 3 | new P1 | 234-6-0 | 240 (0) | 36.30/42.77 | 52.14/35.76 | pass | pass |
| sagat | sf3threes/art2 | 3 | new P2 | 235-4-0 | 239 (1) | 36.34/43.07 | 51.99/36.40 | pass | pass |
| sagat | sf3threes/art2 | 4 | new P1 | 235-5-0 | 240 (0) | 35.05/45.07 | 54.78/34.01 | pass | pass |
| sagat | sf3threes/art2 | 4 | new P2 | 232-8-0 | 240 (0) | 35.14/45.47 | 54.63/34.43 | pass | pass |
| sagat | sf3threes/art2 | 2/1 | higher P1 | 205-26-0 | 231 (9) | 40.42/42.79 | 44.05/36.79 | pass | pass |
| sagat | sf3threes/art2 | 1/2 | higher P2 | 211-24-0 | 235 (5) | 39.98/42.61 | 44.51/36.83 | pass | pass |
| sagat | sf3threes/art2 | 3/2 | higher P1 | 196-37-0 | 233 (7) | 36.63/40.37 | 48.79/43.32 | pass | pass |
| sagat | sf3threes/art2 | 2/3 | higher P2 | 188-41-0 | 229 (11) | 36.41/40.08 | 49.71/43.56 | pass | pass |
| sagat | sf3threes/art2 | 4/3 | higher P1 | 181-48-0 | 229 (11) | 35.61/37.90 | 51.28/47.88 | pass | pass |
| sagat | sf3threes/art2 | 3/4 | higher P2 | 183-44-0 | 227 (13) | 35.12/38.24 | 52.05/47.60 | pass | pass |
| sagat | sf3threes/art3 | 4 | old vs old | 115-125-0 | 240 (0) | 45.16/45.21 | 46.59/46.18 | - | - |
| sagat | sf3threes/art3 | 1 | new P1 | 211-29-0 | 240 (0) | 41.49/50.68 | 41.47/29.31 | pass | pass |
| sagat | sf3threes/art3 | 1 | new P2 | 212-28-0 | 240 (0) | 41.26/50.70 | 41.57/29.43 | pass | pass |
| sagat | sf3threes/art3 | 2 | new P1 | 223-16-0 | 239 (1) | 39.16/45.55 | 47.18/33.82 | pass | pass |
| sagat | sf3threes/art3 | 2 | new P2 | 231-9-0 | 240 (0) | 39.24/45.58 | 46.61/33.61 | pass | pass |
| sagat | sf3threes/art3 | 3 | new P1 | 235-5-0 | 240 (0) | 36.64/43.11 | 51.61/36.31 | pass | pass |
| sagat | sf3threes/art3 | 3 | new P2 | 235-3-0 | 238 (2) | 36.29/43.06 | 52.10/36.06 | pass | pass |
| sagat | sf3threes/art3 | 4 | new P1 | 237-3-0 | 240 (0) | 35.49/45.18 | 53.81/34.09 | pass | pass |
| sagat | sf3threes/art3 | 4 | new P2 | 233-5-0 | 238 (2) | 35.41/45.12 | 54.68/34.86 | pass | pass |
| sagat | sf3threes/art3 | 2/1 | higher P1 | 207-29-0 | 236 (4) | 39.95/42.31 | 43.96/36.96 | pass | pass |
| sagat | sf3threes/art3 | 1/2 | higher P2 | 201-33-0 | 234 (6) | 40.19/42.57 | 44.30/36.75 | pass | pass |
| sagat | sf3threes/art3 | 3/2 | higher P1 | 194-39-0 | 233 (7) | 36.23/40.36 | 49.36/42.74 | pass | pass |
| sagat | sf3threes/art3 | 2/3 | higher P2 | 181-48-0 | 229 (11) | 36.27/40.41 | 48.91/43.19 | pass | pass |
| sagat | sf3threes/art3 | 4/3 | higher P1 | 197-38-0 | 235 (5) | 35.32/37.87 | 51.51/47.08 | pass | pass |
| sagat | sf3threes/art3 | 3/4 | higher P2 | 180-48-0 | 228 (12) | 35.51/38.13 | 51.52/47.66 | pass | pass |

## Stopped matches

- `ryu__sf3ng-art1__old__new__1.tsv`: # seed 117 failed: untranslated branches at frame 4265: [101b61]
- `ryu__sf3ng-art2__old__new__1.tsv`: # seed 117 failed: untranslated branches at frame 4265: [101b61]
- `ryu__sf3ng-art2__new__old__4.tsv`: # seed 148 failed: untranslated branches at frame 5556: [10192f]
- `ryu__sf3ng-art2__old__new__4.tsv`: # seed 26 failed: untranslated branches at frame 4268: [10192f]
- `ryu__sf3ng-art3__old__new__1.tsv`: # seed 117 failed: untranslated branches at frame 4265: [101b61]
- `ryu__sf3twoi-art1__old__new__1.tsv`: # seed 117 failed: untranslated branches at frame 4265: [101b61]
- `ryu__sf3twoi-art2__old__new__1.tsv`: # seed 117 failed: untranslated branches at frame 4265: [101b61]
- `ryu__sf3twoi-art2__old__new__4.tsv`: # seed 70 failed: untranslated branches at frame 5750: [10192f]
- `ryu__sf3twoi-art3__old__new__1.tsv`: # seed 117 failed: untranslated branches at frame 4265: [101b61]
- `ryu__sf3threes-art1__old__new__1.tsv`: # seed 117 failed: untranslated branches at frame 4265: [101b61]
- `ryu__sf3threes-art2__old__new__1.tsv`: # seed 117 failed: untranslated branches at frame 4265: [101b61]
- `ryu__sf3threes-art2__old__new__4.tsv`: # seed 70 failed: untranslated branches at frame 5750: [10192f]
- `ryu__sf3threes-art3__old__new__1.tsv`: # seed 117 failed: untranslated branches at frame 4265: [101b61]
- `ken__sf3ng-art1__new__new__3-2.tsv`: # seed 11 failed: untranslated branches at frame 3305: [11208e]
- `retsu__sf3threes-art1__old__old__4.tsv`: # seed 54 failed: untranslated branches at frame 5989: [122d35]; # seed 220 failed: untranslated branches at frame 3442: [122d35]
- `retsu__sf3threes-art1__new__old__1.tsv`: # seed 148 failed: untranslated branches at frame 5029: [122d35]; # seed 155 failed: untranslated branches at frame 2066: [122d35]
- `retsu__sf3threes-art1__old__new__1.tsv`: # seed 232 failed: untranslated branches at frame 3996: [122d35]
- `retsu__sf3threes-art1__new__old__2.tsv`: # seed 90 failed: untranslated branches at frame 5693: [122d35]
- `retsu__sf3threes-art1__old__new__2.tsv`: # seed 191 failed: untranslated branches at frame 1820: [122d35]
- `retsu__sf3threes-art1__new__old__3.tsv`: # seed 25 failed: untranslated branches at frame 3514: [122d35]
- `retsu__sf3threes-art1__old__new__3.tsv`: # seed 203 failed: untranslated branches at frame 2067: [122d35]; # seed 229 failed: untranslated branches at frame 5983: [122d35]
- `retsu__sf3threes-art1__new__new__2-1.tsv`: # seed 55 failed: untranslated branches at frame 1446: [122d35]
- `retsu__sf3threes-art1__new__new__1-2.tsv`: # seed 16 failed: untranslated branches at frame 6447: [122d35]
- `retsu__sf3threes-art1__new__new__3-2.tsv`: # seed 175 failed: untranslated branches at frame 5287: [122d35]
- `retsu__sf3threes-art1__new__new__2-3.tsv`: # seed 216 failed: untranslated branches at frame 4136: [122d45]; # seed 218 failed: untranslated branches at frame 1977: [123db0]
- `retsu__sf3threes-art2__old__old__4.tsv`: # seed 54 failed: untranslated branches at frame 5989: [122d35]; # seed 220 failed: untranslated branches at frame 3442: [122d35]
- `retsu__sf3threes-art2__new__old__1.tsv`: # seed 148 failed: untranslated branches at frame 5029: [122d35]; # seed 155 failed: untranslated branches at frame 2066: [122d35]
- `retsu__sf3threes-art2__old__new__1.tsv`: # seed 232 failed: untranslated branches at frame 3996: [122d35]
- `retsu__sf3threes-art2__new__old__2.tsv`: # seed 90 failed: untranslated branches at frame 5693: [122d35]
- `retsu__sf3threes-art2__old__new__2.tsv`: # seed 191 failed: untranslated branches at frame 1820: [122d35]
- `retsu__sf3threes-art2__new__old__3.tsv`: # seed 25 failed: untranslated branches at frame 3514: [122d35]
- `retsu__sf3threes-art2__old__new__3.tsv`: # seed 203 failed: untranslated branches at frame 2067: [122d35]
- `retsu__sf3threes-art2__new__new__2-1.tsv`: # seed 55 failed: untranslated branches at frame 1446: [122d35]
- `retsu__sf3threes-art2__new__new__1-2.tsv`: # seed 16 failed: untranslated branches at frame 6447: [122d35]
- `retsu__sf3threes-art2__new__new__3-2.tsv`: # seed 175 failed: untranslated branches at frame 5287: [122d35]
- `retsu__sf3threes-art2__new__new__2-3.tsv`: # seed 216 failed: untranslated branches at frame 4136: [122d45]; # seed 218 failed: untranslated branches at frame 1977: [123db0]
- `retsu__sf3threes-art3__old__old__4.tsv`: # seed 54 failed: untranslated branches at frame 5989: [122d35]; # seed 220 failed: untranslated branches at frame 3442: [122d35]
- `retsu__sf3threes-art3__new__old__1.tsv`: # seed 148 failed: untranslated branches at frame 5029: [122d35]; # seed 155 failed: untranslated branches at frame 2066: [122d35]
- `retsu__sf3threes-art3__old__new__1.tsv`: # seed 232 failed: untranslated branches at frame 3996: [122d35]
- `retsu__sf3threes-art3__new__old__2.tsv`: # seed 90 failed: untranslated branches at frame 5693: [122d35]
- `retsu__sf3threes-art3__old__new__2.tsv`: # seed 191 failed: untranslated branches at frame 1820: [122d35]
- `retsu__sf3threes-art3__new__old__3.tsv`: # seed 25 failed: untranslated branches at frame 3514: [122d35]
- `retsu__sf3threes-art3__old__new__3.tsv`: # seed 200 failed: untranslated branches at frame 6089: [122d35]; # seed 203 failed: untranslated branches at frame 2067: [122d35]; # seed 237 failed: untranslated branches at frame 3050: [122d45]
- `retsu__sf3threes-art3__new__new__2-1.tsv`: # seed 55 failed: untranslated branches at frame 1446: [122d35]
- `retsu__sf3threes-art3__new__new__1-2.tsv`: # seed 16 failed: untranslated branches at frame 6447: [122d35]
- `retsu__sf3threes-art3__new__new__3-2.tsv`: # seed 175 failed: untranslated branches at frame 5287: [122d35]
- `retsu__sf3threes-art3__new__new__2-3.tsv`: # seed 216 failed: untranslated branches at frame 4136: [122d45]; # seed 218 failed: untranslated branches at frame 1977: [123db0]
- `geki__sf3threes-art1__new__new__3-2.tsv`: # seed 239 failed: untranslated branches at frame 3770: [133bb7]
- `geki__sf3threes-art1__new__new__2-3.tsv`: # seed 217 failed: untranslated branches at frame 4504: [133bb7]
- `geki__sf3threes-art2__new__new__3-2.tsv`: # seed 239 failed: untranslated branches at frame 3770: [133bb7]
- `geki__sf3threes-art2__new__new__2-3.tsv`: # seed 217 failed: untranslated branches at frame 4504: [133bb7]
- `geki__sf3threes-art2__new__new__4-3.tsv`: # seed 139 failed: untranslated branches at frame 4002: [133bb7]
- `geki__sf3threes-art3__new__new__3-2.tsv`: # seed 239 failed: untranslated branches at frame 3770: [133bb7]
- `geki__sf3threes-art3__new__new__2-3.tsv`: # seed 217 failed: untranslated branches at frame 4504: [133bb7]
- `geki__sf3threes-art3__new__new__4-3.tsv`: # seed 139 failed: untranslated branches at frame 4002: [133bb7]
- `joe__sf3threes-art1__new__old__2.tsv`: # seed 239 failed: untranslated branches at frame 2058: [14256f]
- `joe__sf3threes-art1__new__old__3.tsv`: # seed 22 failed: untranslated branches at frame 3425: [14256f]
- `joe__sf3threes-art1__new__new__2-1.tsv`: # seed 104 failed: untranslated branches at frame 4390: [1435e2]
- `joe__sf3threes-art1__new__new__3-2.tsv`: # seed 10 failed: untranslated branches at frame 4538: [142577]
- `joe__sf3threes-art1__new__new__2-3.tsv`: # seed 98 failed: untranslated branches at frame 2060: [1435e2]; # seed 195 failed: untranslated branches at frame 5010: [14256f]
- `joe__sf3threes-art1__new__new__4-3.tsv`: # seed 149 failed: untranslated branches at frame 5057: [142577]; # seed 204 failed: untranslated branches at frame 2102: [14257f]
- `joe__sf3threes-art2__new__old__2.tsv`: # seed 239 failed: untranslated branches at frame 2058: [14256f]
- `joe__sf3threes-art2__new__old__3.tsv`: # seed 185 failed: untranslated branches at frame 5295: [142577]
- `joe__sf3threes-art2__new__new__2-1.tsv`: # seed 104 failed: untranslated branches at frame 4390: [1435e2]
- `joe__sf3threes-art2__new__new__2-3.tsv`: # seed 98 failed: untranslated branches at frame 2060: [1435e2]; # seed 195 failed: untranslated branches at frame 5010: [14256f]
- `joe__sf3threes-art2__new__new__4-3.tsv`: # seed 204 failed: untranslated branches at frame 2102: [14257f]
- `joe__sf3threes-art3__new__old__2.tsv`: # seed 239 failed: untranslated branches at frame 2058: [14256f]
- `joe__sf3threes-art3__new__new__2-1.tsv`: # seed 104 failed: untranslated branches at frame 4390: [1435e2]
- `joe__sf3threes-art3__new__new__2-3.tsv`: # seed 98 failed: untranslated branches at frame 2060: [1435e2]; # seed 195 failed: untranslated branches at frame 5010: [14256f]
- `joe__sf3threes-art3__new__new__4-3.tsv`: # seed 204 failed: untranslated branches at frame 2102: [14257f]
- `mike__sf3threes-art1__new__old__1.tsv`: # seed 179 failed: untranslated branches at frame 6878: [152bd0]
- `mike__sf3threes-art1__new__old__2.tsv`: # seed 165 failed: untranslated branches at frame 5805: [152bd0]
- `mike__sf3threes-art1__new__new__1-2.tsv`: # seed 64 failed: untranslated branches at frame 5432: [152bd0]
- `mike__sf3threes-art1__new__new__2-3.tsv`: # seed 202 failed: untranslated branches at frame 1933: [152bd0]
- `mike__sf3threes-art2__new__old__1.tsv`: # seed 179 failed: untranslated branches at frame 6878: [152bd0]
- `mike__sf3threes-art2__new__old__2.tsv`: # seed 165 failed: untranslated branches at frame 5805: [152bd0]
- `mike__sf3threes-art2__new__new__1-2.tsv`: # seed 64 failed: untranslated branches at frame 5432: [152bd0]
- `mike__sf3threes-art2__new__new__2-3.tsv`: # seed 202 failed: untranslated branches at frame 1933: [152bd0]
- `mike__sf3threes-art3__new__old__1.tsv`: # seed 179 failed: untranslated branches at frame 6878: [152bd0]
- `mike__sf3threes-art3__new__old__2.tsv`: # seed 165 failed: untranslated branches at frame 5805: [152bd0]
- `mike__sf3threes-art3__new__new__1-2.tsv`: # seed 64 failed: untranslated branches at frame 5432: [152bd0]
- `mike__sf3threes-art3__new__new__2-3.tsv`: # seed 202 failed: untranslated branches at frame 1933: [152bd0]
- `lee__sf3threes-art1__new__old__2.tsv`: # seed 38 failed: untranslated branches at frame 1457: [162b1f]; # seed 43 failed: untranslated branches at frame 3440: [162b1f]; # seed 173 failed: untranslated branches at frame 2264: [162b1f]
- `lee__sf3threes-art1__new__old__4.tsv`: # seed 173 failed: untranslated branches at frame 3343: [162b1f]
- `lee__sf3threes-art1__new__new__2-1.tsv`: # seed 88 failed: untranslated branches at frame 7740: [162b1f]; # seed 118 failed: untranslated branches at frame 1920: [162b1f]; # seed 166 failed: untranslated branches at frame 3807: [162b1f]; # seed 177 failed: untranslated branches at frame 4900: [162b1f]
- `lee__sf3threes-art1__new__new__1-2.tsv`: # seed 195 failed: untranslated branches at frame 2226: [162b1f]
- `lee__sf3threes-art1__new__new__3-2.tsv`: # seed 36 failed: untranslated branches at frame 5836: [162b1f]; # seed 49 failed: untranslated branches at frame 6403: [162b1f]; # seed 73 failed: untranslated branches at frame 1392: [162b1f]; # seed 118 failed: untranslated branches at frame 2325: [162b1f]; # seed 181 failed: untranslated branches at frame 4413: [162b1f]; # seed 222 failed: untranslated branches at frame 4586: [162b1f]
- `lee__sf3threes-art1__new__new__2-3.tsv`: # seed 4 failed: untranslated branches at frame 2001: [162b1f]; # seed 62 failed: untranslated branches at frame 4395: [162b1f]; # seed 226 failed: untranslated branches at frame 4320: [162b1f]
- `lee__sf3threes-art1__new__new__4-3.tsv`: # seed 5 failed: untranslated branches at frame 3061: [162b1f]; # seed 151 failed: untranslated branches at frame 4216: [162b1f]; # seed 157 failed: untranslated branches at frame 3616: [162b1f]
- `lee__sf3threes-art1__new__new__3-4.tsv`: # seed 49 failed: untranslated branches at frame 4029: [162b1f]; # seed 86 failed: untranslated branches at frame 2126: [162b1f]; # seed 127 failed: untranslated branches at frame 3987: [162b1f]; # seed 215 failed: untranslated branches at frame 1878: [162b1f]; # seed 218 failed: untranslated branches at frame 3522: [162b1f]; # seed 237 failed: untranslated branches at frame 5333: [162b1f]
- `lee__sf3threes-art2__new__old__2.tsv`: # seed 38 failed: untranslated branches at frame 1457: [162b1f]; # seed 43 failed: untranslated branches at frame 3440: [162b1f]; # seed 173 failed: untranslated branches at frame 2264: [162b1f]
- `lee__sf3threes-art2__new__old__4.tsv`: # seed 173 failed: untranslated branches at frame 3343: [162b1f]
- `lee__sf3threes-art2__new__new__2-1.tsv`: # seed 88 failed: untranslated branches at frame 7740: [162b1f]; # seed 118 failed: untranslated branches at frame 1920: [162b1f]; # seed 166 failed: untranslated branches at frame 3807: [162b1f]; # seed 177 failed: untranslated branches at frame 4900: [162b1f]
- `lee__sf3threes-art2__new__new__1-2.tsv`: # seed 195 failed: untranslated branches at frame 2226: [162b1f]
- `lee__sf3threes-art2__new__new__3-2.tsv`: # seed 36 failed: untranslated branches at frame 5836: [162b1f]; # seed 49 failed: untranslated branches at frame 6403: [162b1f]; # seed 73 failed: untranslated branches at frame 1392: [162b1f]; # seed 118 failed: untranslated branches at frame 2325: [162b1f]; # seed 181 failed: untranslated branches at frame 4413: [162b1f]; # seed 222 failed: untranslated branches at frame 4586: [162b1f]
- `lee__sf3threes-art2__new__new__2-3.tsv`: # seed 4 failed: untranslated branches at frame 2001: [162b1f]; # seed 62 failed: untranslated branches at frame 4395: [162b1f]; # seed 226 failed: untranslated branches at frame 4320: [162b1f]
- `lee__sf3threes-art2__new__new__4-3.tsv`: # seed 5 failed: untranslated branches at frame 3061: [162b1f]; # seed 151 failed: untranslated branches at frame 4216: [162b1f]; # seed 157 failed: untranslated branches at frame 3616: [162b1f]
- `lee__sf3threes-art2__new__new__3-4.tsv`: # seed 49 failed: untranslated branches at frame 4029: [162b1f]; # seed 86 failed: untranslated branches at frame 2126: [162b1f]; # seed 127 failed: untranslated branches at frame 3987: [162b1f]; # seed 215 failed: untranslated branches at frame 1878: [162b1f]; # seed 218 failed: untranslated branches at frame 3522: [162b1f]; # seed 237 failed: untranslated branches at frame 5333: [162b1f]
- `lee__sf3threes-art3__new__old__2.tsv`: # seed 38 failed: untranslated branches at frame 1457: [162b1f]; # seed 43 failed: untranslated branches at frame 3440: [162b1f]; # seed 173 failed: untranslated branches at frame 2264: [162b1f]
- `lee__sf3threes-art3__new__old__4.tsv`: # seed 173 failed: untranslated branches at frame 3343: [162b1f]
- `lee__sf3threes-art3__new__new__2-1.tsv`: # seed 88 failed: untranslated branches at frame 7740: [162b1f]; # seed 118 failed: untranslated branches at frame 1920: [162b1f]; # seed 166 failed: untranslated branches at frame 3807: [162b1f]; # seed 177 failed: untranslated branches at frame 4900: [162b1f]
- `lee__sf3threes-art3__new__new__1-2.tsv`: # seed 195 failed: untranslated branches at frame 2226: [162b1f]
- `lee__sf3threes-art3__new__new__3-2.tsv`: # seed 36 failed: untranslated branches at frame 5836: [162b1f]; # seed 49 failed: untranslated branches at frame 6403: [162b1f]; # seed 73 failed: untranslated branches at frame 1392: [162b1f]; # seed 118 failed: untranslated branches at frame 2325: [162b1f]; # seed 181 failed: untranslated branches at frame 4413: [162b1f]; # seed 222 failed: untranslated branches at frame 4586: [162b1f]
- `lee__sf3threes-art3__new__new__2-3.tsv`: # seed 4 failed: untranslated branches at frame 2001: [162b1f]; # seed 62 failed: untranslated branches at frame 4395: [162b1f]; # seed 226 failed: untranslated branches at frame 4320: [162b1f]
- `lee__sf3threes-art3__new__new__4-3.tsv`: # seed 5 failed: untranslated branches at frame 3061: [162b1f]; # seed 151 failed: untranslated branches at frame 4216: [162b1f]; # seed 157 failed: untranslated branches at frame 3616: [162b1f]
- `lee__sf3threes-art3__new__new__3-4.tsv`: # seed 49 failed: untranslated branches at frame 4029: [162b1f]; # seed 86 failed: untranslated branches at frame 2126: [162b1f]; # seed 127 failed: untranslated branches at frame 3987: [162b1f]; # seed 215 failed: untranslated branches at frame 1878: [162b1f]; # seed 218 failed: untranslated branches at frame 3522: [162b1f]; # seed 237 failed: untranslated branches at frame 5333: [162b1f]
- `gen__sf3threes-art1__new__old__1.tsv`: # seed 31 failed: untranslated branches at frame 4539: [17302c]; # seed 54 failed: untranslated branches at frame 3581: [17302c]; # seed 110 failed: untranslated branches at frame 2261: [17302c]; # seed 144 failed: untranslated branches at frame 2031: [17302c]; # seed 155 failed: untranslated branches at frame 4807: [17302c]; # seed 212 failed: untranslated branches at frame 2582: [17302c]; # seed 220 failed: untranslated branches at frame 3109: [17302c]
- `gen__sf3threes-art1__old__new__1.tsv`: # seed 10 failed: untranslated branches at frame 5539: [17302c]; # seed 16 failed: untranslated branches at frame 4081: [17302c]; # seed 36 failed: untranslated branches at frame 1544: [17302c]; # seed 41 failed: untranslated branches at frame 3139: [17302c]; # seed 59 failed: untranslated branches at frame 3797: [17302c]; # seed 169 failed: untranslated branches at frame 2652: [1730e1]; # seed 177 failed: untranslated branches at frame 1763: [17302c]; # seed 185 failed: untranslated branches at frame 1875: [17302c]; # seed 198 failed: untranslated branches at frame 6875: [17302c]; # seed 223 failed: untranslated branches at frame 4764: [17302c]
- `gen__sf3threes-art1__new__old__2.tsv`: # seed 55 failed: untranslated branches at frame 2106: [17302c]; # seed 63 failed: untranslated branches at frame 1873: [17302c]; # seed 99 failed: untranslated branches at frame 6295: [172467]; # seed 108 failed: untranslated branches at frame 1707: [17302c]; # seed 116 failed: untranslated branches at frame 1415: [173e8c]; # seed 149 failed: untranslated branches at frame 3150: [173e8c]; # seed 177 failed: untranslated branches at frame 5463: [17302c]; # seed 186 failed: untranslated branches at frame 7433: [172467]
- `gen__sf3threes-art1__old__new__2.tsv`: # seed 29 failed: untranslated branches at frame 1562: [1725cd]; # seed 85 failed: untranslated branches at frame 3046: [1725cd]; # seed 91 failed: untranslated branches at frame 1431: [1725cd]; # seed 230 failed: untranslated branches at frame 4557: [17302c]
- `gen__sf3threes-art1__new__old__3.tsv`: # seed 39 failed: untranslated branches at frame 3166: [1725cd]; # seed 85 failed: untranslated branches at frame 1989: [173e8c]
- `gen__sf3threes-art1__old__new__3.tsv`: # seed 43 failed: untranslated branches at frame 5999: [172467]; # seed 128 failed: untranslated branches at frame 1375: [1725cd]; # seed 158 failed: untranslated branches at frame 2244: [173e8c]
- `gen__sf3threes-art1__old__new__4.tsv`: # seed 67 failed: untranslated branches at frame 1768: [173e8c]; # seed 182 failed: untranslated branches at frame 3588: [17302c]; # seed 185 failed: untranslated branches at frame 4899: [1725cd]
- `gen__sf3threes-art1__new__new__2-1.tsv`: # seed 4 failed: untranslated branches at frame 5981: [17302c]; # seed 49 failed: untranslated branches at frame 1461: [1725cd]; # seed 58 failed: untranslated branches at frame 1864: [17302c]; # seed 60 failed: untranslated branches at frame 4188: [1730e1]; # seed 66 failed: untranslated branches at frame 2412: [17302c]; # seed 77 failed: untranslated branches at frame 5736: [17302c]; # seed 100 failed: untranslated branches at frame 6094: [17302c]; # seed 122 failed: untranslated branches at frame 5205: [17302c]; # seed 126 failed: untranslated branches at frame 3960: [17302c]; # seed 139 failed: untranslated branches at frame 4907: [17302c]; # seed 146 failed: untranslated branches at frame 4857: [17302c]; # seed 171 failed: untranslated branches at frame 2167: [17302c]; # seed 194 failed: untranslated branches at frame 2459: [1730e1]; # seed 203 failed: untranslated branches at frame 6082: [1725cd]
- `gen__sf3threes-art1__new__new__1-2.tsv`: # seed 52 failed: untranslated branches at frame 4615: [173e8c]; # seed 121 failed: untranslated branches at frame 1805: [173e8c]; # seed 229 failed: untranslated branches at frame 4061: [17302c]
- `gen__sf3threes-art1__new__new__3-2.tsv`: # seed 11 failed: untranslated branches at frame 3367: [1725cd]; # seed 121 failed: untranslated branches at frame 1574: [17302c]; # seed 128 failed: untranslated branches at frame 5187: [172467]; # seed 156 failed: untranslated branches at frame 4056: [173e8c]
- `gen__sf3threes-art1__new__new__2-3.tsv`: # seed 120 failed: untranslated branches at frame 4402: [172467]; # seed 122 failed: untranslated branches at frame 3119: [17302c]; # seed 146 failed: untranslated branches at frame 5742: [173e8c]; # seed 150 failed: untranslated branches at frame 1558: [1725cd]; # seed 170 failed: untranslated branches at frame 3889: [17302c]; # seed 181 failed: untranslated branches at frame 4051: [17302c]; # seed 223 failed: untranslated branches at frame 4313: [173e8c]; # seed 224 failed: untranslated branches at frame 1899: [17302c]
- `gen__sf3threes-art1__new__new__4-3.tsv`: # seed 40 failed: untranslated branches at frame 3030: [172467]; # seed 236 failed: untranslated branches at frame 1956: [173e8c]
- `gen__sf3threes-art1__new__new__3-4.tsv`: # seed 44 failed: untranslated branches at frame 4830: [1725cd]; # seed 182 failed: untranslated branches at frame 3881: [172467]
- `gen__sf3threes-art2__new__old__1.tsv`: # seed 29 failed: untranslated branches at frame 3556: [17302c]; # seed 83 failed: untranslated branches at frame 4817: [17302c]; # seed 110 failed: untranslated branches at frame 2261: [17302c]; # seed 115 failed: untranslated branches at frame 3997: [17302c]; # seed 144 failed: untranslated branches at frame 2031: [17302c]; # seed 175 failed: untranslated branches at frame 4411: [17302c]; # seed 212 failed: untranslated branches at frame 2582: [17302c]; # seed 220 failed: untranslated branches at frame 3109: [17302c]
- `gen__sf3threes-art2__old__new__1.tsv`: # seed 36 failed: untranslated branches at frame 1544: [17302c]; # seed 53 failed: untranslated branches at frame 3882: [17302c]; # seed 85 failed: untranslated branches at frame 7021: [17302c]; # seed 113 failed: untranslated branches at frame 8669: [17302c]; # seed 151 failed: untranslated branches at frame 3563: [17302c]; # seed 177 failed: untranslated branches at frame 1763: [17302c]; # seed 185 failed: untranslated branches at frame 1875: [17302c]; # seed 207 failed: untranslated branches at frame 4673: [17302c]; # seed 238 failed: untranslated branches at frame 3445: [17302c]
- `gen__sf3threes-art2__new__old__2.tsv`: # seed 11 failed: untranslated branches at frame 3346: [17302c]; # seed 66 failed: untranslated branches at frame 2053: [1725cd]; # seed 116 failed: untranslated branches at frame 1415: [173e8c]; # seed 172 failed: untranslated branches at frame 5208: [17302c]
- `gen__sf3threes-art2__old__new__2.tsv`: # seed 91 failed: untranslated branches at frame 1431: [1725cd]; # seed 105 failed: untranslated branches at frame 4175: [173e8c]; # seed 155 failed: untranslated branches at frame 5746: [1725cd]; # seed 159 failed: untranslated branches at frame 4027: [17255e]; # seed 173 failed: untranslated branches at frame 4282: [17302c]
- `gen__sf3threes-art2__new__old__3.tsv`: # seed 146 failed: untranslated branches at frame 1539: [173e8c]; # seed 218 failed: untranslated branches at frame 1959: [173e8c]
- `gen__sf3threes-art2__old__new__3.tsv`: # seed 87 failed: untranslated branches at frame 3366: [17302c]; # seed 128 failed: untranslated branches at frame 1375: [1725cd]
- `gen__sf3threes-art2__new__old__4.tsv`: # seed 165 failed: untranslated branches at frame 3801: [17302c]
- `gen__sf3threes-art2__new__new__2-1.tsv`: # seed 5 failed: untranslated branches at frame 4362: [17302c]; # seed 33 failed: untranslated branches at frame 1810: [17302c]; # seed 48 failed: untranslated branches at frame 5782: [173e8c]; # seed 49 failed: untranslated branches at frame 1461: [1725cd]; # seed 56 failed: untranslated branches at frame 4372: [17302c]; # seed 93 failed: untranslated branches at frame 1905: [17302c]; # seed 102 failed: untranslated branches at frame 3528: [1730e1]; # seed 109 failed: untranslated branches at frame 4181: [17302c]; # seed 114 failed: untranslated branches at frame 3268: [17302c]; # seed 147 failed: untranslated branches at frame 4001: [173e8c]; # seed 163 failed: untranslated branches at frame 5748: [17302c]; # seed 166 failed: untranslated branches at frame 1691: [17302c]; # seed 174 failed: untranslated branches at frame 1746: [17302c]; # seed 181 failed: untranslated branches at frame 3846: [17302c]; # seed 182 failed: untranslated branches at frame 6256: [17302c]; # seed 206 failed: untranslated branches at frame 6644: [17302c]; # seed 227 failed: untranslated branches at frame 3970: [17302c]
- `gen__sf3threes-art2__new__new__1-2.tsv`: # seed 32 failed: untranslated branches at frame 4027: [17302c]; # seed 74 failed: untranslated branches at frame 2072: [17302c]; # seed 181 failed: untranslated branches at frame 4139: [17302c]; # seed 208 failed: untranslated branches at frame 5954: [17302c]; # seed 217 failed: untranslated branches at frame 3978: [17302c]
- `gen__sf3threes-art2__new__new__3-2.tsv`: # seed 64 failed: untranslated branches at frame 3712: [17302c]; # seed 121 failed: untranslated branches at frame 1574: [17302c]; # seed 155 failed: untranslated branches at frame 3950: [17302c]; # seed 208 failed: untranslated branches at frame 2126: [17302c]; # seed 230 failed: untranslated branches at frame 3890: [17302c]
- `gen__sf3threes-art2__new__new__2-3.tsv`: # seed 98 failed: untranslated branches at frame 3681: [17302c]; # seed 134 failed: untranslated branches at frame 1814: [1725cd]; # seed 135 failed: untranslated branches at frame 1873: [17302c]
- `gen__sf3threes-art2__new__new__4-3.tsv`: # seed 24 failed: untranslated branches at frame 4500: [1725cd]
- `gen__sf3threes-art2__new__new__3-4.tsv`: # seed 46 failed: untranslated branches at frame 5316: [1725cd]; # seed 168 failed: untranslated branches at frame 3330: [17302c]; # seed 176 failed: untranslated branches at frame 4898: [1725cd]
- `gen__sf3threes-art3__new__old__1.tsv`: # seed 29 failed: untranslated branches at frame 3556: [17302c]; # seed 83 failed: untranslated branches at frame 4817: [17302c]; # seed 110 failed: untranslated branches at frame 2261: [17302c]; # seed 115 failed: untranslated branches at frame 3997: [17302c]; # seed 144 failed: untranslated branches at frame 2031: [17302c]; # seed 175 failed: untranslated branches at frame 4411: [17302c]; # seed 212 failed: untranslated branches at frame 2582: [17302c]; # seed 220 failed: untranslated branches at frame 3109: [17302c]
- `gen__sf3threes-art3__old__new__1.tsv`: # seed 36 failed: untranslated branches at frame 1544: [17302c]; # seed 53 failed: untranslated branches at frame 3882: [17302c]; # seed 85 failed: untranslated branches at frame 7021: [17302c]; # seed 113 failed: untranslated branches at frame 8669: [17302c]; # seed 151 failed: untranslated branches at frame 3563: [17302c]; # seed 177 failed: untranslated branches at frame 1763: [17302c]; # seed 185 failed: untranslated branches at frame 1875: [17302c]; # seed 207 failed: untranslated branches at frame 4673: [17302c]; # seed 238 failed: untranslated branches at frame 3445: [17302c]
- `gen__sf3threes-art3__new__old__2.tsv`: # seed 11 failed: untranslated branches at frame 3346: [17302c]; # seed 66 failed: untranslated branches at frame 2053: [1725cd]; # seed 116 failed: untranslated branches at frame 1415: [173e8c]; # seed 119 failed: untranslated branches at frame 6029: [1725cd]; # seed 172 failed: untranslated branches at frame 5208: [17302c]
- `gen__sf3threes-art3__old__new__2.tsv`: # seed 91 failed: untranslated branches at frame 1431: [1725cd]; # seed 105 failed: untranslated branches at frame 4175: [173e8c]; # seed 173 failed: untranslated branches at frame 4282: [17302c]; # seed 180 failed: untranslated branches at frame 4133: [1725cd]
- `gen__sf3threes-art3__new__old__3.tsv`: # seed 107 failed: untranslated branches at frame 6367: [1725cd]; # seed 146 failed: untranslated branches at frame 1539: [173e8c]; # seed 218 failed: untranslated branches at frame 1959: [173e8c]
- `gen__sf3threes-art3__old__new__3.tsv`: # seed 128 failed: untranslated branches at frame 1375: [1725cd]
- `gen__sf3threes-art3__new__old__4.tsv`: # seed 165 failed: untranslated branches at frame 3801: [17302c]
- `gen__sf3threes-art3__new__new__2-1.tsv`: # seed 3 failed: untranslated branches at frame 3782: [173e8c]; # seed 5 failed: untranslated branches at frame 4362: [17302c]; # seed 14 failed: untranslated branches at frame 2875: [17302c]; # seed 33 failed: untranslated branches at frame 1810: [17302c]; # seed 48 failed: untranslated branches at frame 5782: [173e8c]; # seed 49 failed: untranslated branches at frame 1461: [1725cd]; # seed 56 failed: untranslated branches at frame 4372: [17302c]; # seed 58 failed: untranslated branches at frame 4661: [17302c]; # seed 93 failed: untranslated branches at frame 1905: [17302c]; # seed 100 failed: untranslated branches at frame 5077: [17302c]; # seed 102 failed: untranslated branches at frame 3528: [1730e1]; # seed 109 failed: untranslated branches at frame 4181: [17302c]; # seed 129 failed: untranslated branches at frame 6087: [1725cd]; # seed 163 failed: untranslated branches at frame 5748: [17302c]; # seed 166 failed: untranslated branches at frame 1691: [17302c]; # seed 174 failed: untranslated branches at frame 1746: [17302c]; # seed 181 failed: untranslated branches at frame 3846: [17302c]; # seed 182 failed: untranslated branches at frame 6143: [17302c]; # seed 227 failed: untranslated branches at frame 3970: [17302c]
- `gen__sf3threes-art3__new__new__1-2.tsv`: # seed 32 failed: untranslated branches at frame 4027: [17302c]; # seed 36 failed: untranslated branches at frame 7228: [17302c]; # seed 40 failed: untranslated branches at frame 5737: [173e8c]; # seed 74 failed: untranslated branches at frame 2072: [17302c]; # seed 80 failed: untranslated branches at frame 5637: [17302c]; # seed 181 failed: untranslated branches at frame 4139: [17302c]
- `gen__sf3threes-art3__new__new__3-2.tsv`: # seed 6 failed: untranslated branches at frame 5471: [1725cd]; # seed 64 failed: untranslated branches at frame 3712: [17302c]; # seed 121 failed: untranslated branches at frame 1574: [17302c]; # seed 126 failed: untranslated branches at frame 5601: [1725cd]; # seed 155 failed: untranslated branches at frame 3950: [17302c]; # seed 208 failed: untranslated branches at frame 2126: [17302c]; # seed 230 failed: untranslated branches at frame 3890: [17302c]
- `gen__sf3threes-art3__new__new__2-3.tsv`: # seed 55 failed: untranslated branches at frame 4969: [17302c]; # seed 98 failed: untranslated branches at frame 3681: [17302c]; # seed 134 failed: untranslated branches at frame 1814: [1725cd]; # seed 135 failed: untranslated branches at frame 1873: [17302c]; # seed 171 failed: untranslated branches at frame 4799: [17302c]; # seed 220 failed: untranslated branches at frame 4927: [17302c]
- `gen__sf3threes-art3__new__new__4-3.tsv`: # seed 21 failed: untranslated branches at frame 5825: [17302c]; # seed 95 failed: untranslated branches at frame 6579: [17302c]
- `gen__sf3threes-art3__new__new__3-4.tsv`: # seed 166 failed: untranslated branches at frame 5706: [17302c]; # seed 168 failed: untranslated branches at frame 3330: [17302c]; # seed 176 failed: untranslated branches at frame 4898: [1725cd]
- `birdie__sf3threes-art1__new__old__2.tsv`: # seed 144 failed: untranslated branches at frame 1825: [183545]; # seed 172 failed: untranslated branches at frame 3771: [1823c7]; # seed 182 failed: untranslated branches at frame 3984: [1823c7]; # seed 236 failed: untranslated branches at frame 4497: [1823c7]
- `birdie__sf3threes-art1__old__new__2.tsv`: # seed 55 failed: untranslated branches at frame 3611: [1823c7]; # seed 75 failed: untranslated branches at frame 3925: [1823c7]; # seed 157 failed: untranslated branches at frame 4003: [1823c7]
- `birdie__sf3threes-art1__new__old__3.tsv`: # seed 28 failed: untranslated branches at frame 3508: [1823c7]
- `birdie__sf3threes-art1__old__new__3.tsv`: # seed 193 failed: untranslated branches at frame 5850: [1823c7]
- `birdie__sf3threes-art1__new__old__4.tsv`: # seed 123 failed: untranslated branches at frame 3077: [183545]
- `birdie__sf3threes-art1__new__new__2-1.tsv`: # seed 90 failed: untranslated branches at frame 4210: [1823c7]; # seed 109 failed: untranslated branches at frame 4329: [1823c7]; # seed 150 failed: untranslated branches at frame 5626: [1823c7]
- `birdie__sf3threes-art1__new__new__3-2.tsv`: # seed 27 failed: untranslated branches at frame 5080: [1823c7]; # seed 45 failed: untranslated branches at frame 3636: [1823c7]; # seed 73 failed: untranslated branches at frame 5531: [183545]; # seed 74 failed: untranslated branches at frame 5790: [1823c7]; # seed 233 failed: untranslated branches at frame 5194: [1823c7]
- `birdie__sf3threes-art1__new__new__2-3.tsv`: # seed 12 failed: untranslated branches at frame 3896: [1823c7]; # seed 87 failed: untranslated branches at frame 4188: [1823c7]; # seed 110 failed: untranslated branches at frame 3523: [1823c7]; # seed 123 failed: untranslated branches at frame 5369: [1823c7]; # seed 142 failed: untranslated branches at frame 3768: [183545]; # seed 234 failed: untranslated branches at frame 5084: [1823c7]
- `birdie__sf3threes-art1__new__new__4-3.tsv`: # seed 50 failed: untranslated branches at frame 3978: [1823c7]; # seed 186 failed: untranslated branches at frame 4608: [1823c7]; # seed 227 failed: untranslated branches at frame 5027: [1823c7]
- `birdie__sf3threes-art1__new__new__3-4.tsv`: # seed 103 failed: untranslated branches at frame 3328: [1823c7]; # seed 180 failed: untranslated branches at frame 3554: [1823c7]; # seed 236 failed: untranslated branches at frame 3575: [1823c7]
- `birdie__sf3threes-art2__new__old__2.tsv`: # seed 144 failed: untranslated branches at frame 1825: [183545]
- `birdie__sf3threes-art2__new__old__3.tsv`: # seed 113 failed: untranslated branches at frame 3109: [183545]
- `birdie__sf3threes-art2__old__new__3.tsv`: # seed 233 failed: untranslated branches at frame 3271: [183545]
- `birdie__sf3threes-art2__new__old__4.tsv`: # seed 123 failed: untranslated branches at frame 3077: [183545]; # seed 174 failed: untranslated branches at frame 3612: [183545]
- `birdie__sf3threes-art2__new__new__1-2.tsv`: # seed 88 failed: untranslated branches at frame 5442: [183545]
- `birdie__sf3threes-art2__new__new__2-3.tsv`: # seed 95 failed: untranslated branches at frame 1719: [183545]
- `birdie__sf3threes-art3__old__old__4.tsv`: # seed 209 failed: untranslated branches at frame 4000: [182d02]
- `birdie__sf3threes-art3__new__old__2.tsv`: # seed 144 failed: untranslated branches at frame 1825: [183545]
- `birdie__sf3threes-art3__new__old__4.tsv`: # seed 123 failed: untranslated branches at frame 3077: [183545]
- `eagle__sf3threes-art1__old__old__4.tsv`: # seed 2 failed: untranslated branches at frame 5922: [192b55]; # seed 23 failed: untranslated branches at frame 5549: [192b55]; # seed 45 failed: untranslated branches at frame 5992: [193220]; # seed 64 failed: untranslated branches at frame 3664: [193220]; # seed 66 failed: untranslated branches at frame 5207: [192b55]; # seed 73 failed: untranslated branches at frame 3564: [193220]; # seed 139 failed: untranslated branches at frame 5486: [192b55]; # seed 188 failed: untranslated branches at frame 5546: [192b55]
- `eagle__sf3threes-art1__new__old__1.tsv`: # seed 22 failed: untranslated branches at frame 2801: [193220]; # seed 37 failed: untranslated branches at frame 3422: [193220]; # seed 122 failed: untranslated branches at frame 2117: [192b65]; # seed 201 failed: untranslated branches at frame 1876: [192b65]
- `eagle__sf3threes-art1__old__new__1.tsv`: # seed 150 failed: untranslated branches at frame 3540: [193220]
- `eagle__sf3threes-art1__new__old__2.tsv`: # seed 39 failed: untranslated branches at frame 1823: [193c4c]; # seed 84 failed: untranslated branches at frame 3515: [193220]; # seed 130 failed: untranslated branches at frame 3797: [193c4c]; # seed 133 failed: untranslated branches at frame 3914: [193220]; # seed 157 failed: untranslated branches at frame 4388: [192b55]; # seed 192 failed: untranslated branches at frame 2107: [193220]; # seed 211 failed: untranslated branches at frame 5981: [193220]
- `eagle__sf3threes-art1__old__new__2.tsv`: # seed 57 failed: untranslated branches at frame 6550: [192b55]; # seed 67 failed: untranslated branches at frame 2005: [193220]; # seed 154 failed: untranslated branches at frame 4452: [193220]; # seed 174 failed: untranslated branches at frame 3400: [192b65]; # seed 236 failed: untranslated branches at frame 3242: [193220]
- `eagle__sf3threes-art1__new__old__3.tsv`: # seed 1 failed: untranslated branches at frame 3612: [193220]; # seed 6 failed: untranslated branches at frame 1803: [193c4c]; # seed 53 failed: untranslated branches at frame 4089: [193220]; # seed 102 failed: untranslated branches at frame 2754: [193220]; # seed 125 failed: untranslated branches at frame 1980: [193220]; # seed 144 failed: untranslated branches at frame 3558: [193220]; # seed 170 failed: untranslated branches at frame 1700: [193220]; # seed 172 failed: untranslated branches at frame 3423: [193220]; # seed 196 failed: untranslated branches at frame 4078: [193220]; # seed 209 failed: untranslated branches at frame 3795: [193220]
- `eagle__sf3threes-art1__old__new__3.tsv`: # seed 3 failed: untranslated branches at frame 3446: [192b55]; # seed 19 failed: untranslated branches at frame 3265: [193220]; # seed 27 failed: untranslated branches at frame 3371: [193220]; # seed 99 failed: untranslated branches at frame 3457: [193220]; # seed 121 failed: untranslated branches at frame 5086: [193220]; # seed 170 failed: untranslated branches at frame 3266: [193220]; # seed 183 failed: untranslated branches at frame 3475: [193220]; # seed 207 failed: untranslated branches at frame 5545: [192b55]; # seed 227 failed: untranslated branches at frame 4121: [192b55]; # seed 239 failed: untranslated branches at frame 5773: [193220]
- `eagle__sf3threes-art1__new__old__4.tsv`: # seed 47 failed: untranslated branches at frame 3142: [193220]; # seed 58 failed: untranslated branches at frame 3474: [193c4c]; # seed 211 failed: untranslated branches at frame 5436: [193220]
- `eagle__sf3threes-art1__old__new__4.tsv`: # seed 63 failed: untranslated branches at frame 3407: [193c4c]; # seed 86 failed: untranslated branches at frame 2087: [193220]; # seed 104 failed: untranslated branches at frame 1944: [193c4c]; # seed 114 failed: untranslated branches at frame 3609: [193220]; # seed 116 failed: untranslated branches at frame 3802: [193220]; # seed 169 failed: untranslated branches at frame 4239: [193220]; # seed 178 failed: untranslated branches at frame 2244: [193220]; # seed 179 failed: untranslated branches at frame 3945: [192b55]; # seed 184 failed: untranslated branches at frame 1751: [193220]; # seed 205 failed: untranslated branches at frame 3173: [193c4c]; # seed 207 failed: untranslated branches at frame 3979: [193220]
- `eagle__sf3threes-art1__new__new__2-1.tsv`: # seed 70 failed: untranslated branches at frame 3268: [193220]; # seed 121 failed: untranslated branches at frame 4018: [193220]; # seed 175 failed: untranslated branches at frame 3289: [193c4c]; # seed 217 failed: untranslated branches at frame 2340: [193220]
- `eagle__sf3threes-art1__new__new__1-2.tsv`: # seed 28 failed: untranslated branches at frame 5336: [193c4c]; # seed 55 failed: untranslated branches at frame 6084: [193c4c]; # seed 85 failed: untranslated branches at frame 2057: [193c4c]; # seed 156 failed: untranslated branches at frame 3952: [193c4c]; # seed 165 failed: untranslated branches at frame 5365: [193220]; # seed 198 failed: untranslated branches at frame 1813: [193c4c]; # seed 230 failed: untranslated branches at frame 6251: [192b65]
- `eagle__sf3threes-art1__new__new__3-2.tsv`: # seed 87 failed: untranslated branches at frame 5410: [192b55]; # seed 122 failed: untranslated branches at frame 1713: [193c4c]; # seed 130 failed: untranslated branches at frame 3302: [193220]; # seed 177 failed: untranslated branches at frame 4028: [193c4c]; # seed 230 failed: untranslated branches at frame 1450: [193c4c]
- `eagle__sf3threes-art1__new__new__2-3.tsv`: # seed 7 failed: untranslated branches at frame 3232: [193c4c]; # seed 27 failed: untranslated branches at frame 3936: [193c4c]; # seed 83 failed: untranslated branches at frame 5250: [193c4c]; # seed 144 failed: untranslated branches at frame 3622: [193c4c]; # seed 170 failed: untranslated branches at frame 3323: [193c4c]; # seed 189 failed: untranslated branches at frame 5422: [192b55]; # seed 211 failed: untranslated branches at frame 3601: [192b55]; # seed 223 failed: untranslated branches at frame 5624: [193220]
- `eagle__sf3threes-art1__new__new__4-3.tsv`: # seed 56 failed: untranslated branches at frame 1526: [193c4c]; # seed 68 failed: untranslated branches at frame 1517: [193c4c]; # seed 213 failed: untranslated branches at frame 3402: [193c4c]
- `eagle__sf3threes-art1__new__new__3-4.tsv`: # seed 13 failed: untranslated branches at frame 4156: [192b55]; # seed 52 failed: untranslated branches at frame 3627: [193220]; # seed 127 failed: untranslated branches at frame 3005: [193220]
- `eagle__sf3threes-art2__old__old__4.tsv`: # seed 3 failed: untranslated branches at frame 7014: [193220]; # seed 45 failed: untranslated branches at frame 5992: [193220]; # seed 64 failed: untranslated branches at frame 3664: [193220]; # seed 73 failed: untranslated branches at frame 3564: [193220]; # seed 102 failed: untranslated branches at frame 4134: [192b55]; # seed 128 failed: untranslated branches at frame 6862: [193220]; # seed 152 failed: untranslated branches at frame 5992: [193220]
- `eagle__sf3threes-art2__new__old__1.tsv`: # seed 123 failed: untranslated branches at frame 5034: [192b65]
- `eagle__sf3threes-art2__old__new__1.tsv`: # seed 26 failed: untranslated branches at frame 4694: [192b65]; # seed 104 failed: untranslated branches at frame 6222: [193220]
- `eagle__sf3threes-art2__new__old__2.tsv`: # seed 2 failed: untranslated branches at frame 4393: [192b55]; # seed 39 failed: untranslated branches at frame 1823: [193c4c]; # seed 63 failed: untranslated branches at frame 4061: [193220]; # seed 141 failed: untranslated branches at frame 2468: [193220]; # seed 145 failed: untranslated branches at frame 4344: [193220]; # seed 192 failed: untranslated branches at frame 2107: [193220]
- `eagle__sf3threes-art2__old__new__2.tsv`: # seed 49 failed: untranslated branches at frame 3788: [193220]; # seed 67 failed: untranslated branches at frame 2005: [193220]; # seed 68 failed: untranslated branches at frame 3796: [193220]; # seed 174 failed: untranslated branches at frame 3400: [192b65]; # seed 225 failed: untranslated branches at frame 2310: [193220]
- `eagle__sf3threes-art2__new__old__3.tsv`: # seed 1 failed: untranslated branches at frame 3612: [193220]; # seed 6 failed: untranslated branches at frame 1803: [193c4c]; # seed 53 failed: untranslated branches at frame 4089: [193220]; # seed 54 failed: untranslated branches at frame 3365: [193220]; # seed 111 failed: untranslated branches at frame 2070: [193220]; # seed 125 failed: untranslated branches at frame 1980: [193220]; # seed 139 failed: untranslated branches at frame 2164: [193220]; # seed 170 failed: untranslated branches at frame 1700: [193220]
- `eagle__sf3threes-art2__old__new__3.tsv`: # seed 19 failed: untranslated branches at frame 3265: [193220]; # seed 170 failed: untranslated branches at frame 3266: [193220]; # seed 232 failed: untranslated branches at frame 5626: [193220]
- `eagle__sf3threes-art2__new__old__4.tsv`: # seed 80 failed: untranslated branches at frame 3275: [193220]; # seed 132 failed: untranslated branches at frame 5406: [193220]; # seed 184 failed: untranslated branches at frame 3780: [193220]; # seed 219 failed: untranslated branches at frame 3500: [193220]
- `eagle__sf3threes-art2__old__new__4.tsv`: # seed 81 failed: untranslated branches at frame 5580: [192b55]; # seed 86 failed: untranslated branches at frame 2087: [193220]; # seed 92 failed: untranslated branches at frame 3354: [193c4c]; # seed 178 failed: untranslated branches at frame 2244: [193220]; # seed 184 failed: untranslated branches at frame 1751: [193220]; # seed 216 failed: untranslated branches at frame 1672: [193c4c]
- `eagle__sf3threes-art2__new__new__2-1.tsv`: # seed 34 failed: untranslated branches at frame 2513: [193c4c]; # seed 72 failed: untranslated branches at frame 4727: [193220]; # seed 136 failed: untranslated branches at frame 4112: [193220]; # seed 147 failed: untranslated branches at frame 4281: [193220]; # seed 161 failed: untranslated branches at frame 1662: [192b65]; # seed 213 failed: untranslated branches at frame 2202: [193c4c]; # seed 217 failed: untranslated branches at frame 2340: [193220]; # seed 221 failed: untranslated branches at frame 3266: [193c4c]
- `eagle__sf3threes-art2__new__new__1-2.tsv`: # seed 85 failed: untranslated branches at frame 2057: [193c4c]; # seed 198 failed: untranslated branches at frame 1813: [193c4c]; # seed 202 failed: untranslated branches at frame 5559: [193220]
- `eagle__sf3threes-art2__new__new__3-2.tsv`: # seed 6 failed: untranslated branches at frame 2187: [193c4c]; # seed 197 failed: untranslated branches at frame 4103: [193c4c]; # seed 230 failed: untranslated branches at frame 1450: [193c4c]
- `eagle__sf3threes-art2__new__new__2-3.tsv`: # seed 186 failed: untranslated branches at frame 3800: [193220]
- `eagle__sf3threes-art2__new__new__4-3.tsv`: # seed 56 failed: untranslated branches at frame 1526: [193c4c]; # seed 68 failed: untranslated branches at frame 1517: [193c4c]
- `eagle__sf3threes-art2__new__new__3-4.tsv`: # seed 38 failed: untranslated branches at frame 3060: [193c4c]; # seed 127 failed: untranslated branches at frame 3005: [193220]
- `eagle__sf3threes-art3__old__old__4.tsv`: # seed 20 failed: untranslated branches at frame 3881: [1930ef]; # seed 45 failed: untranslated branches at frame 5992: [193220]; # seed 64 failed: untranslated branches at frame 3664: [193220]; # seed 73 failed: untranslated branches at frame 3564: [193220]; # seed 102 failed: untranslated branches at frame 4134: [1930e7]; # seed 128 failed: untranslated branches at frame 6862: [193220]; # seed 150 failed: untranslated branches at frame 4647: [193220]; # seed 201 failed: untranslated branches at frame 5194: [193220]; # seed 240 failed: untranslated branches at frame 4250: [1930ef]
- `eagle__sf3threes-art3__new__old__1.tsv`: # seed 22 failed: untranslated branches at frame 2801: [193220]; # seed 37 failed: untranslated branches at frame 3422: [193220]; # seed 122 failed: untranslated branches at frame 2117: [192b65]; # seed 201 failed: untranslated branches at frame 1876: [192b65]
- `eagle__sf3threes-art3__old__new__1.tsv`: # seed 150 failed: untranslated branches at frame 3540: [193220]
- `eagle__sf3threes-art3__new__old__2.tsv`: # seed 39 failed: untranslated branches at frame 1823: [193c4c]; # seed 84 failed: untranslated branches at frame 3515: [193220]; # seed 133 failed: untranslated branches at frame 3914: [193220]; # seed 192 failed: untranslated branches at frame 2107: [193220]; # seed 211 failed: untranslated branches at frame 5981: [193220]
- `eagle__sf3threes-art3__old__new__2.tsv`: # seed 20 failed: untranslated branches at frame 4891: [192b65]; # seed 26 failed: untranslated branches at frame 4153: [1930e7]; # seed 67 failed: untranslated branches at frame 2005: [193220]; # seed 79 failed: untranslated branches at frame 3886: [193220]; # seed 107 failed: untranslated branches at frame 5790: [193220]; # seed 154 failed: untranslated branches at frame 4452: [193220]; # seed 174 failed: untranslated branches at frame 3400: [192b65]; # seed 236 failed: untranslated branches at frame 3242: [193220]
- `eagle__sf3threes-art3__new__old__3.tsv`: # seed 1 failed: untranslated branches at frame 3612: [193220]; # seed 6 failed: untranslated branches at frame 1803: [193c4c]; # seed 53 failed: untranslated branches at frame 4089: [193220]; # seed 102 failed: untranslated branches at frame 2754: [193220]; # seed 125 failed: untranslated branches at frame 1980: [193220]; # seed 144 failed: untranslated branches at frame 3558: [193220]; # seed 170 failed: untranslated branches at frame 1700: [193220]; # seed 172 failed: untranslated branches at frame 3423: [193220]; # seed 196 failed: untranslated branches at frame 4078: [193220]; # seed 209 failed: untranslated branches at frame 3795: [193220]
- `eagle__sf3threes-art3__old__new__3.tsv`: # seed 19 failed: untranslated branches at frame 3265: [193220]; # seed 27 failed: untranslated branches at frame 3371: [193220]; # seed 66 failed: untranslated branches at frame 5438: [193220]; # seed 99 failed: untranslated branches at frame 3457: [193220]; # seed 170 failed: untranslated branches at frame 3266: [193220]; # seed 183 failed: untranslated branches at frame 3475: [193220]
- `eagle__sf3threes-art3__new__old__4.tsv`: # seed 47 failed: untranslated branches at frame 3142: [193220]; # seed 58 failed: untranslated branches at frame 3474: [193c4c]
- `eagle__sf3threes-art3__old__new__4.tsv`: # seed 63 failed: untranslated branches at frame 3407: [193c4c]; # seed 86 failed: untranslated branches at frame 2087: [193220]; # seed 104 failed: untranslated branches at frame 1944: [193c4c]; # seed 114 failed: untranslated branches at frame 3609: [193220]; # seed 116 failed: untranslated branches at frame 3802: [193220]; # seed 178 failed: untranslated branches at frame 2244: [193220]; # seed 179 failed: untranslated branches at frame 3945: [1930e7]; # seed 184 failed: untranslated branches at frame 1751: [193220]; # seed 205 failed: untranslated branches at frame 3173: [193c4c]; # seed 207 failed: untranslated branches at frame 3979: [193220]
- `eagle__sf3threes-art3__new__new__2-1.tsv`: # seed 70 failed: untranslated branches at frame 3268: [193220]; # seed 121 failed: untranslated branches at frame 4018: [193220]; # seed 130 failed: untranslated branches at frame 5402: [1930e7]; # seed 175 failed: untranslated branches at frame 3289: [193c4c]; # seed 217 failed: untranslated branches at frame 2340: [193220]
- `eagle__sf3threes-art3__new__new__1-2.tsv`: # seed 28 failed: untranslated branches at frame 5336: [193c4c]; # seed 85 failed: untranslated branches at frame 2057: [193c4c]; # seed 165 failed: untranslated branches at frame 5365: [193220]; # seed 166 failed: untranslated branches at frame 3855: [193220]; # seed 198 failed: untranslated branches at frame 1813: [193c4c]; # seed 215 failed: untranslated branches at frame 3618: [1930e7]
- `eagle__sf3threes-art3__new__new__3-2.tsv`: # seed 87 failed: untranslated branches at frame 5413: [1930e7]; # seed 122 failed: untranslated branches at frame 1713: [193c4c]; # seed 130 failed: untranslated branches at frame 3302: [193220]; # seed 177 failed: untranslated branches at frame 4028: [193c4c]; # seed 230 failed: untranslated branches at frame 1450: [193c4c]
- `eagle__sf3threes-art3__new__new__2-3.tsv`: # seed 7 failed: untranslated branches at frame 3232: [193c4c]; # seed 83 failed: untranslated branches at frame 5250: [193c4c]; # seed 170 failed: untranslated branches at frame 3323: [193c4c]; # seed 223 failed: untranslated branches at frame 5624: [193220]
- `eagle__sf3threes-art3__new__new__4-3.tsv`: # seed 18 failed: untranslated branches at frame 3739: [1930ef]; # seed 56 failed: untranslated branches at frame 1526: [193c4c]; # seed 68 failed: untranslated branches at frame 1517: [193c4c]; # seed 213 failed: untranslated branches at frame 3402: [193c4c]
- `eagle__sf3threes-art3__new__new__3-4.tsv`: # seed 52 failed: untranslated branches at frame 3627: [193220]; # seed 127 failed: untranslated branches at frame 3005: [193220]
- `adon__sf3threes-art1__old__old__4.tsv`: # seed 45 failed: untranslated branches at frame 5304: [1a25c7]; # seed 72 failed: untranslated branches at frame 4356: [1a25c7]; # seed 80 failed: untranslated branches at frame 6325: [1a25c7]; # seed 93 failed: untranslated branches at frame 4313: [1a25c7]; # seed 167 failed: untranslated branches at frame 4902: [1a25c7]
- `adon__sf3threes-art1__old__new__1.tsv`: # seed 14 failed: untranslated branches at frame 4016: [1a370a]
- `adon__sf3threes-art2__old__old__4.tsv`: # seed 149 failed: untranslated branches at frame 6028: [1a25c7]; # seed 204 failed: untranslated branches at frame 6691: [1a25c7]
- `adon__sf3threes-art2__old__new__1.tsv`: # seed 14 failed: untranslated branches at frame 4016: [1a370a]
- `adon__sf3threes-art3__old__old__4.tsv`: # seed 14 failed: untranslated branches at frame 4495: [1a30c8]; # seed 161 failed: untranslated branches at frame 5098: [1a30c8]; # seed 164 failed: untranslated branches at frame 4077: [1a30c8]
- `adon__sf3threes-art3__old__new__2.tsv`: # seed 50 failed: untranslated branches at frame 5116: [1a30c8]; # seed 107 failed: untranslated branches at frame 3786: [1a30c8]
- `adon__sf3threes-art3__new__old__4.tsv`: # seed 228 failed: untranslated branches at frame 3182: [1a30c8]
- `adon__sf3threes-art3__old__new__4.tsv`: # seed 19 failed: untranslated branches at frame 3911: [1a30c8]
- `adon__sf3threes-art3__new__new__1-2.tsv`: # seed 220 failed: untranslated branches at frame 5043: [1a370a]
- `adon__sf3threes-art3__new__new__3-4.tsv`: # seed 1 failed: untranslated branches at frame 3316: [1a30d0]; # seed 227 failed: untranslated branches at frame 4708: [1a30d0]
- `sagat__sf3threes-art1__new__old__2.tsv`: # seed 42 failed: untranslated branches at frame 3749: [1b88b0]; # seed 68 failed: untranslated branches at frame 1690: [1b88b0]; # seed 145 failed: untranslated branches at frame 3196: [1b88b0]
- `sagat__sf3threes-art1__old__new__2.tsv`: # seed 9 failed: untranslated branches at frame 5622: [1b88b0]
- `sagat__sf3threes-art1__new__old__3.tsv`: # seed 8 failed: untranslated branches at frame 4698: [1b88b0]
- `sagat__sf3threes-art1__old__new__3.tsv`: # seed 3 failed: untranslated branches at frame 3369: [1b88b0]
- `sagat__sf3threes-art1__new__new__2-1.tsv`: # seed 3 failed: untranslated branches at frame 3654: [1b88b0]; # seed 28 failed: untranslated branches at frame 2073: [1b88b0]; # seed 101 failed: untranslated branches at frame 1430: [1b88b0]; # seed 119 failed: untranslated branches at frame 2834: [1b88b0]; # seed 139 failed: untranslated branches at frame 3517: [1b88b0]; # seed 173 failed: untranslated branches at frame 1764: [1b88b0]; # seed 238 failed: untranslated branches at frame 5436: [1b88b0]
- `sagat__sf3threes-art1__new__new__1-2.tsv`: # seed 156 failed: untranslated branches at frame 5545: [1b88b0]; # seed 173 failed: untranslated branches at frame 1661: [1b88b0]; # seed 185 failed: untranslated branches at frame 5080: [1b88b0]; # seed 205 failed: untranslated branches at frame 1895: [1b88b0]; # seed 217 failed: untranslated branches at frame 4169: [1b88b0]
- `sagat__sf3threes-art1__new__new__3-2.tsv`: # seed 12 failed: untranslated branches at frame 1569: [1b88b0]; # seed 81 failed: untranslated branches at frame 2014: [1b88b0]; # seed 97 failed: untranslated branches at frame 2730: [1b88b0]; # seed 110 failed: untranslated branches at frame 3502: [1b88b0]; # seed 116 failed: untranslated branches at frame 3264: [1b88b0]; # seed 120 failed: untranslated branches at frame 1910: [1b88b0]; # seed 175 failed: untranslated branches at frame 1852: [1b88b0]; # seed 240 failed: untranslated branches at frame 1659: [1b88b0]
- `sagat__sf3threes-art1__new__new__2-3.tsv`: # seed 26 failed: untranslated branches at frame 1813: [1b88b0]; # seed 48 failed: untranslated branches at frame 1630: [1b88b0]; # seed 63 failed: untranslated branches at frame 1605: [1b88b0]; # seed 67 failed: untranslated branches at frame 2123: [1b88b0]; # seed 108 failed: untranslated branches at frame 1901: [1b88b0]; # seed 118 failed: untranslated branches at frame 5176: [1b88b0]; # seed 123 failed: untranslated branches at frame 1872: [1b88b0]; # seed 168 failed: untranslated branches at frame 3515: [1b88b0]; # seed 199 failed: untranslated branches at frame 3065: [1b88b0]; # seed 206 failed: untranslated branches at frame 1613: [1b88b0]; # seed 230 failed: untranslated branches at frame 1498: [1b88b0]; # seed 233 failed: untranslated branches at frame 3406: [1b88b0]; # seed 240 failed: untranslated branches at frame 3329: [1b88b0]
- `sagat__sf3threes-art1__new__new__4-3.tsv`: # seed 13 failed: untranslated branches at frame 3199: [1b88b0]; # seed 57 failed: untranslated branches at frame 1678: [1b88b0]; # seed 73 failed: untranslated branches at frame 3168: [1b88b0]; # seed 75 failed: untranslated branches at frame 5164: [1b88b0]; # seed 106 failed: untranslated branches at frame 4583: [1b88b0]; # seed 117 failed: untranslated branches at frame 1539: [1b88b0]; # seed 122 failed: untranslated branches at frame 2998: [1b88b0]; # seed 128 failed: untranslated branches at frame 1695: [1b88b0]; # seed 138 failed: untranslated branches at frame 3181: [1b88b0]; # seed 180 failed: untranslated branches at frame 3403: [1b88b0]; # seed 198 failed: untranslated branches at frame 3601: [1b88b0]; # seed 233 failed: untranslated branches at frame 3056: [1b88b0]
- `sagat__sf3threes-art1__new__new__3-4.tsv`: # seed 2 failed: untranslated branches at frame 3029: [1b88b0]; # seed 91 failed: untranslated branches at frame 4996: [1b88b0]; # seed 110 failed: untranslated branches at frame 3283: [1b88b0]; # seed 125 failed: untranslated branches at frame 1831: [1b88b0]; # seed 162 failed: untranslated branches at frame 3521: [1b88b0]; # seed 190 failed: untranslated branches at frame 4408: [1b88b0]; # seed 191 failed: untranslated branches at frame 1650: [1b88b0]
- `sagat__sf3threes-art2__new__old__2.tsv`: # seed 5 failed: untranslated branches at frame 4118: [1b88b0]; # seed 68 failed: untranslated branches at frame 1690: [1b88b0]
- `sagat__sf3threes-art2__old__new__2.tsv`: # seed 36 failed: untranslated branches at frame 3522: [1b88b0]; # seed 50 failed: untranslated branches at frame 1620: [1b88b0]; # seed 131 failed: untranslated branches at frame 3900: [1b88b0]; # seed 180 failed: untranslated branches at frame 1733: [1b88b0]; # seed 216 failed: untranslated branches at frame 3527: [1b88b0]
- `sagat__sf3threes-art2__old__new__3.tsv`: # seed 3 failed: untranslated branches at frame 3369: [1b88b0]
- `sagat__sf3threes-art2__new__new__2-1.tsv`: # seed 25 failed: untranslated branches at frame 1658: [1b88b0]; # seed 28 failed: untranslated branches at frame 2073: [1b88b0]; # seed 30 failed: untranslated branches at frame 3519: [1b88b0]; # seed 97 failed: untranslated branches at frame 1793: [1b88b0]; # seed 101 failed: untranslated branches at frame 1430: [1b88b0]; # seed 116 failed: untranslated branches at frame 2912: [1b88b0]; # seed 139 failed: untranslated branches at frame 3517: [1b88b0]; # seed 157 failed: untranslated branches at frame 3435: [1b88b0]; # seed 170 failed: untranslated branches at frame 3014: [1b88b0]
- `sagat__sf3threes-art2__new__new__1-2.tsv`: # seed 1 failed: untranslated branches at frame 2279: [1b88b0]; # seed 43 failed: untranslated branches at frame 1586: [1b88b0]; # seed 106 failed: untranslated branches at frame 5800: [1b88b0]; # seed 174 failed: untranslated branches at frame 2981: [1b88b0]; # seed 205 failed: untranslated branches at frame 1895: [1b88b0]
- `sagat__sf3threes-art2__new__new__3-2.tsv`: # seed 25 failed: untranslated branches at frame 1447: [1b88b0]; # seed 96 failed: untranslated branches at frame 3447: [1b88b0]; # seed 97 failed: untranslated branches at frame 2730: [1b88b0]; # seed 127 failed: untranslated branches at frame 3206: [1b88b0]; # seed 134 failed: untranslated branches at frame 3121: [1b88b0]; # seed 175 failed: untranslated branches at frame 1852: [1b88b0]; # seed 240 failed: untranslated branches at frame 1659: [1b88b0]
- `sagat__sf3threes-art2__new__new__2-3.tsv`: # seed 11 failed: untranslated branches at frame 5310: [1b88b0]; # seed 18 failed: untranslated branches at frame 3267: [1b88b0]; # seed 19 failed: untranslated branches at frame 3420: [1b88b0]; # seed 48 failed: untranslated branches at frame 1630: [1b88b0]; # seed 67 failed: untranslated branches at frame 5018: [1b88b0]; # seed 108 failed: untranslated branches at frame 1901: [1b88b0]; # seed 123 failed: untranslated branches at frame 1872: [1b88b0]; # seed 205 failed: untranslated branches at frame 2106: [1b88b0]; # seed 206 failed: untranslated branches at frame 1613: [1b88b0]; # seed 211 failed: untranslated branches at frame 1620: [1b88b0]; # seed 230 failed: untranslated branches at frame 1498: [1b88b0]
- `sagat__sf3threes-art2__new__new__4-3.tsv`: # seed 13 failed: untranslated branches at frame 3199: [1b88b0]; # seed 14 failed: untranslated branches at frame 3831: [1b88b0]; # seed 37 failed: untranslated branches at frame 1646: [1b88b0]; # seed 53 failed: untranslated branches at frame 3211: [1b88b0]; # seed 93 failed: untranslated branches at frame 3782: [1b88b0]; # seed 122 failed: untranslated branches at frame 2998: [1b88b0]; # seed 128 failed: untranslated branches at frame 1695: [1b88b0]; # seed 138 failed: untranslated branches at frame 1683: [1b88b0]; # seed 164 failed: untranslated branches at frame 1757: [1b88b0]; # seed 171 failed: untranslated branches at frame 1940: [1b88b0]; # seed 208 failed: untranslated branches at frame 3724: [1b88b0]
- `sagat__sf3threes-art2__new__new__3-4.tsv`: # seed 1 failed: untranslated branches at frame 4810: [1b88b0]; # seed 6 failed: untranslated branches at frame 1514: [1b88b0]; # seed 9 failed: untranslated branches at frame 3089: [1b88b0]; # seed 33 failed: untranslated branches at frame 1709: [1b88b0]; # seed 74 failed: untranslated branches at frame 3344: [1b88b0]; # seed 96 failed: untranslated branches at frame 3439: [1b88b0]; # seed 125 failed: untranslated branches at frame 1831: [1b88b0]; # seed 147 failed: untranslated branches at frame 5287: [1b88b0]; # seed 150 failed: untranslated branches at frame 1796: [1b88b0]; # seed 184 failed: untranslated branches at frame 1791: [1b88b0]; # seed 191 failed: untranslated branches at frame 1650: [1b88b0]; # seed 206 failed: untranslated branches at frame 2830: [1b88b0]; # seed 228 failed: untranslated branches at frame 3861: [1b88b0]
- `sagat__sf3threes-art3__new__old__2.tsv`: # seed 68 failed: untranslated branches at frame 1690: [1b88b0]
- `sagat__sf3threes-art3__old__new__3.tsv`: # seed 3 failed: untranslated branches at frame 3369: [1b88b0]; # seed 9 failed: untranslated branches at frame 1845: [1b88b0]
- `sagat__sf3threes-art3__old__new__4.tsv`: # seed 145 failed: untranslated branches at frame 2943: [1b88b0]; # seed 179 failed: untranslated branches at frame 3276: [1b88b0]
- `sagat__sf3threes-art3__new__new__2-1.tsv`: # seed 28 failed: untranslated branches at frame 2073: [1b88b0]; # seed 81 failed: untranslated branches at frame 2153: [1b88b0]; # seed 101 failed: untranslated branches at frame 1430: [1b88b0]; # seed 139 failed: untranslated branches at frame 3517: [1b88b0]
- `sagat__sf3threes-art3__new__new__1-2.tsv`: # seed 5 failed: untranslated branches at frame 3102: [1b88b0]; # seed 43 failed: untranslated branches at frame 6048: [1b88b0]; # seed 52 failed: untranslated branches at frame 1881: [1b88b0]; # seed 141 failed: untranslated branches at frame 4906: [1b88b0]; # seed 205 failed: untranslated branches at frame 1895: [1b88b0]; # seed 211 failed: untranslated branches at frame 2096: [1b88b0]
- `sagat__sf3threes-art3__new__new__3-2.tsv`: # seed 9 failed: untranslated branches at frame 2226: [1b88b0]; # seed 36 failed: untranslated branches at frame 2954: [1b88b0]; # seed 97 failed: untranslated branches at frame 2730: [1b88b0]; # seed 148 failed: untranslated branches at frame 3654: [1b88b0]; # seed 175 failed: untranslated branches at frame 1852: [1b88b0]; # seed 202 failed: untranslated branches at frame 1756: [1b88b0]; # seed 240 failed: untranslated branches at frame 1659: [1b88b0]
- `sagat__sf3threes-art3__new__new__2-3.tsv`: # seed 26 failed: untranslated branches at frame 1813: [1b88b0]; # seed 40 failed: untranslated branches at frame 3174: [1b88b0]; # seed 48 failed: untranslated branches at frame 1630: [1b88b0]; # seed 95 failed: untranslated branches at frame 1869: [1b88b0]; # seed 108 failed: untranslated branches at frame 1901: [1b88b0]; # seed 123 failed: untranslated branches at frame 1872: [1b88b0]; # seed 138 failed: untranslated branches at frame 3431: [1b88b0]; # seed 171 failed: untranslated branches at frame 3415: [1b88b0]; # seed 206 failed: untranslated branches at frame 1613: [1b88b0]; # seed 210 failed: untranslated branches at frame 3028: [1b88b0]; # seed 230 failed: untranslated branches at frame 1498: [1b88b0]
- `sagat__sf3threes-art3__new__new__4-3.tsv`: # seed 13 failed: untranslated branches at frame 3199: [1b88b0]; # seed 37 failed: untranslated branches at frame 1646: [1b88b0]; # seed 84 failed: untranslated branches at frame 4650: [1b88b0]; # seed 128 failed: untranslated branches at frame 1695: [1b88b0]; # seed 179 failed: untranslated branches at frame 1744: [1b88b0]
- `sagat__sf3threes-art3__new__new__3-4.tsv`: # seed 2 failed: untranslated branches at frame 3029: [1b88b0]; # seed 55 failed: untranslated branches at frame 3330: [1b88b0]; # seed 61 failed: untranslated branches at frame 3285: [1b88b0]; # seed 64 failed: untranslated branches at frame 4030: [1b88b0]; # seed 67 failed: untranslated branches at frame 3670: [1b88b0]; # seed 77 failed: untranslated branches at frame 1927: [1b88b0]; # seed 125 failed: untranslated branches at frame 1831: [1b88b0]; # seed 182 failed: untranslated branches at frame 1981: [1b88b0]; # seed 184 failed: untranslated branches at frame 1791: [1b88b0]; # seed 191 failed: untranslated branches at frame 1650: [1b88b0]; # seed 197 failed: untranslated branches at frame 3290: [1b88b0]; # seed 221 failed: untranslated branches at frame 3215: [1b88b0]
