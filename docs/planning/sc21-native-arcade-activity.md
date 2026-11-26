# SC21 native Arcade activity

Actual authored native Arcade Capcom AI with the switch off versus the stronger Fair CPU with it on; this is separate from the Local Fair CPU versus Fair CPU grid.

Source: `13a3e219b131e924e469d8d6f29d6369c8ee177e`. Log SHA-256: `c381233b9cd5fbf35f55b0af9579644352717276c10df6f49e33f190fc9b572a`.

Coverage: ten authentic Capcom nonshotos, modes 2 and 4, level 4 (Very Hard), switch off/on: 40 trials and 20 paired fixture cells. Each trial retains up to 6,000 live frames or its first natural match completion. No rows are excluded.

| Scope | Capcom frames | Stronger frames | Capcom attacks/min | Stronger attacks/min | Capcom idle % | Stronger idle % | APM up | Idle down | Both |
|---|---:|---:|---:|---:|---:|---:|:---:|:---:|:---:|
| All trials, frame weighted | 27537 | 37578 | 40.396557 | 47.708766 | 23.579184 | 43.735696 | yes | no | no |
| Mode 2, frame weighted | 12500 | 21414 | 40.032000 | 44.886523 | 25.216000 | 44.582983 | yes | no | no |
| Mode 4, frame weighted | 15037 | 16164 | 40.699608 | 51.447661 | 22.218528 | 42.613215 | yes | no | no |
| Retsu mode 2 | 2190 | 2900 | 21.369863 | 39.724138 | 47.716895 | 47.586207 | yes | yes | yes |
| Retsu mode 4 | 3089 | 1756 | 25.639365 | 38.952164 | 39.656847 | 47.949886 | yes | no | no |
| Geki mode 2 | 1956 | 1634 | 57.055215 | 63.892289 | 6.850716 | 34.394125 | yes | no | no |
| Geki mode 4 | 2348 | 1329 | 47.529813 | 78.555305 | 6.473595 | 27.689992 | yes | no | no |
| Joe mode 2 | 1228 | 3480 | 52.768730 | 39.310345 | 14.413681 | 46.954023 | no | no | no |
| Joe mode 4 | 1436 | 3419 | 37.604457 | 37.905820 | 12.813370 | 46.270839 | yes | no | no |
| Mike mode 2 | 3051 | 1189 | 20.058997 | 54.499579 | 44.706654 | 41.883936 | yes | yes | yes |
| Mike mode 4 | 3456 | 1384 | 25.000000 | 65.028902 | 38.975694 | 45.447977 | yes | no | no |
| Lee mode 2 | 476 | 3389 | 52.941176 | 38.241369 | 14.915966 | 44.673945 | no | no | no |
| Lee mode 4 | 596 | 1520 | 66.442953 | 37.894737 | 12.248322 | 42.697368 | no | no | no |
| Gen mode 2 | 512 | 1889 | 42.187500 | 45.738486 | 4.101562 | 39.227104 | yes | no | no |
| Gen mode 4 | 417 | 1123 | 69.064748 | 64.113980 | 6.714628 | 40.783615 | no | no | no |
| Birdie mode 2 | 837 | 1247 | 47.311828 | 75.060144 | 14.814815 | 24.458701 | yes | no | no |
| Birdie mode 4 | 1297 | 1316 | 52.737086 | 79.331307 | 10.562837 | 23.480243 | yes | no | no |
| Eagle mode 2 | 651 | 2862 | 49.769585 | 32.704403 | 11.674347 | 59.818309 | no | no | no |
| Eagle mode 4 | 1105 | 1131 | 35.837104 | 63.660477 | 13.212670 | 45.004421 | yes | no | no |
| Adon mode 2 | 1100 | 1884 | 65.454545 | 42.038217 | 8.272727 | 46.284501 | no | no | no |
| Adon mode 4 | 825 | 2253 | 82.909091 | 38.348868 | 3.878788 | 51.797603 | no | no | no |
| Sagat mode 2 | 499 | 940 | 50.501002 | 61.276596 | 9.819639 | 35.000000 | yes | no | no |
| Sagat mode 4 | 468 | 933 | 76.923077 | 50.160772 | 3.632479 | 40.192926 | no | no | no |

Across the 20 pairs: attacks/minute rises in 12; idle fraction falls in 2; both directions occur in 2. Attacks/minute ties: 0; idle-fraction ties: 0.

After stepping, live_frames counts now.fighting. On adjacent live frames, an attack starts when the observed CPU is attacking and the previous state was not attacking or its move ID changed; idle requires both observed states free and unchanged horizontal position. Guarding can count as idle. plan_attacks additionally requires a stronger plan before or after the step.

All rates use live frames, 60 frames/second: attacks/minute = 3600 * total attacks / total live_frames; idle fraction = total idle_frames / total live_frames. Aggregates pool integer counters, never average per-trial rates. Exact reduced fractions and integer cross-products govern comparisons; displayed decimals are rounded.

Deterministic fixture observations at Very Hard only, with a reactive human sparring script. The script is shared, but trajectories and match lengths can differ. Authored opponent selection uses synthetic ladder wiring before native round setup; it is not an authored campaign-node test. All short completed matches remain included.

Descriptive attack and idle activity only. A direction count is not an acceptance threshold or superiority bar. No win, difficulty-wide superiority, combat quality, statistical independence, or Capcom Ryu/Ken comparison is established. Repeated attacks with no observed intervening state count once; observations sample once per frame.

The caller supplies the independently verified run source SHA. Git verifies that this commit contains the reviewed observer/schema blobs. The log itself has no source attestation; these checks cannot prove which executable produced it.

The companion JSON preserves every raw trial and its log line, all integer counters, exact rates/deltas, and source/helper/log fingerprints.
