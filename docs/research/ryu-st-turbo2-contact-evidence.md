# Ryu contact timing in World Super Turbo 2

These follow-up captures use the World `ssf2t` parent, `940223`, in MAME 0.289. New and Old Ryu face New Ken at 32 source-world pixels. At frame-end sample 2070, set `0xff83a9` to `2` and `0xff8cd3` to `0x60`, the World Turbo 2 selector and threshold. Set Ryu and Ken's integer world X to 552 and 584 and clear their fractions. Leave the round clock and Y unchanged. The controls select New or Old Ryu through the character screen. The trace verifies `+0x3b6` at Ryu's object base. [MAME manifest](https://github.com/mamedev/mame/blob/mame0289/src/mame/capcom/cps2.cpp#L8152) · [Spec #138](https://github.com/johnstreetfighter/sf1-port/issues/138)

## LP Hadoken startup and late crouching LK cancel

The MAME write taps read the game tick at `0xff801d` as the 68000 writes Ryu's state or Ken's life. Ryu's state byte is `0xff8451` and Ken's life word is `0xff8878`. Use the first life decrement, because the life word is also copied on a subsequent tick. Ryu's first Hadoken animation pointer write agrees with the state transition. The separate frame-end trace can contain two game ticks or repeat a tick. The state write reports a new `17` input (Forward + LP) with previous `5` (Down-Forward) in each of the eight raw cases and each late cancel that started. [Raw write-tap rows](ryu-st-turbo2-contact-writes.tsv)

For the uncanceled move, each row advances the entire recipe by one display sample. Ken is hit at the first projectile position in this spacing. In both styles the projectile spawns at `Ryu X + 93` and advances three world pixels per game tick. The wider 132-pixel runs also confirm that trajectory. A first observation at `+96` occurs when the emulator sample covers the spawn and the first advance.

| Ryu | Final LP queued after display samples | Hadoken start game ticks | First damage game ticks | Startup to contact |
| --- | --- | --- | --- | --- |
| New | 2120, 2121, 2122, 2123 | 216, 217, 218, 220 | 227, 228, 229, 231 | 11 in all four |
| Old | 2120, 2121, 2122, 2123 | 219, 220, 222, 223 | 229, 230, 232, 233 | 10 in all four |

The late-cancel table uses the game tick of the crouching LK life write as the origin. The first five columns include the actual game tick, even when a display observation reported the following tick. `2LK → LP Hadoken` remains a true combo through a tested actual +15 start for New and +16 for Old. The next actual +16 New and +17 Old starts that spawned a projectile both hit at +27, after Ken returned to neutral.

| Ryu and phase | Opener display / tick | Hadoken start display / tick | First damage display / tick | Start → damage | Ken’s state before damage | Combo |
| --- | --- | --- | --- | ---: | --- | --- |
| New, early phase | 2124 / 219 | 2135 / 233 (+14) | 2144 / 245 (+26) | 12 | Busy, `14/2` | Yes |
| New, later phase | 2125 / 220 | 2136 / 235 (+15) | 2145 / 246 (+26) | 11 | Busy, `14/2` | Yes |
| New, first late | 2124 / 219 | 2136 / 235 (+16) | 2145 / 246 (+27) | 11 | Neutral, `0/2` | No |
| Old, early phase | 2123 / 222 | 2135 / 237 (+15) | 2142 / 247 (+25) | 10 | Busy, `14/2` | Yes |
| Old, later phase | 2124 / 223 | 2136 / 239 (+16) | 2144 / 249 (+26) | 10 | Busy, `14/2` | Yes |
| Old, first late | 2123 / 222 | 2136 / 239 (+17) | 2144 / 249 (+27) | 10 | Neutral, `0/2` | No |

## Old Ryu crouching MK into LP Hadoken

The crouching MK route was replayed at four staggered display phases with 11 delays per phase. A tap on the current input word at Ryu `+0x392` records the first actual Forward + LP write. The move-state tap identifies the Hadoken start and the first life decrement identifies contact. On all four phases, a final LP at kick contact +16 game ticks started Hadoken on that tick and hit at +26 while Ken was still in grounded stun. Ken’s combo counter rose to 1. Final LP at +17 reached the input word in every phase and produced no Hadoken start, projectile, or second hit. Tested later offsets through +24 also produced no fireball. Earlier +12 and +13 inputs missed in one display phase with the single-sample directions.

| Kick queued at display | Kick damage game tick | Latest LP input display / tick | Hadoken start display / tick | Second damage display / tick | First late LP input display / tick |
| --- | ---: | --- | --- | --- | --- |
| 2120 | 223 | 2136 / 239 | 2136 / 239 | 2144 / 249 | 2136 / 240 |
| 2121 | 224 | 2136 / 240 | 2137 / 240 | 2145 / 250 | 2137 / 241 |
| 2122 | 226 | 2138 / 242 | 2139 / 242 | 2146 / 252 | 2139 / 243 |
| 2123 | 227 | 2139 / 243 | 2139 / 243 | 2147 / 253 | 2139 / 244 |

To replay, use the Old Ryu setup below. Queue Down + MK from `N` through `N+2`, and one display each of Down, Down-Forward, and Forward + LP starting at `N+3+D`. For `N = 2120, 2121, 2122, 2123`, the last successful `D` values are `10, 10, 11, 10`, and the first late values are `11, 11, 12, 11`. The input write and state write can share a game tick while appearing in adjacent display samples. The raw cases are in `%TEMP%/ssf2-ryu-audit/st-t2-old-mk-old-{D}-{phase}`. Their `writes.tsv` files come from `st-t2-old-mk.py`.

## Crouching light and medium kick displacement

For both loadouts, clean grounded Ken displacement starts at game tick +16. New and Old gave the same cumulative sequence. Each entry below is the change during one advancing game tick. The shorter list ends at +26 and the longer at +31. Ken first returns to neutral at +27 for LK and +32 for MK. A frame-end sample may first see that return on the following tick. Ken's horizontal fractions remained zero.

| Ryu input | Ken movement at consecutive ticks from +16 | Total |
| --- | --- | ---: |
| Crouching LK | `4, 4, 4, 4, 3, 3, 3, 2, 1, 1, 1` | 30 px |
| Crouching MK | `6, 6, 6, 5, 5, 5, 4, 4, 3, 3, 2, 1, 1, 1, 1, 1` | 54 px |

## Replay setup

Start a two-player match from the default EEPROM. Hold each coin and start command for six display samples starting at 1250 (P1 coin), 1270 (P2 coin), 1300 (P1 start), and 1340 (P2 start). Press both LP for six samples starting at 1400, 1470 and 1540. For Old Ryu, queue Right at 1478…1479, 1482…1483 and 1486…1487, then Left + LP at 1490…1491. The original trace queues controls in the frame-end callback. Use a fresh copy of the same EEPROM for each replay.

- Raw Hadoken. Let `N` be 2120 through 2123. Queue Down `N−8…N−5`, Down-Forward `N−4…N−1`, and Forward+LP `N…N+4`.
- Crouching LK or MK. Queue Down + attack on `N…N+2`, where `N` is 2120 through 2123. Stay neutral afterwards to measure pushback and recovery. Sample game tick, signed integer X and unsigned fraction at Ken's object offsets `+0x06/+0x08`. Use all four phases to resolve gaps in the per-display trace.
- Late LK cancel. Queue Down + LK on `N…N+2`, where `N` is 2120 or 2121. Let `D` be 9 for the last-success replay or 10 for the late replay. Queue one sample each of Down, Down-Forward, Forward+LP starting at `N+3+D`.
- The live projectile slots start at `0xff97a2`, have a stride of `0xc0` and identify a live slot with the word `0x0101`. Read the projectile's X and fraction at `+0x06/+0x08`.

The capture machine has raw frame traces and write logs under `%TEMP%/ssf2-ryu-audit`. The replay scripts are `st-t2-push.py`, `st-t2-tap.py`, `st-t2-raw-tap.py`, and `st-turbo2.py`. Their output directories begin with `st-t2-push-`, `st-t2-tap-`, `st-t2-raw-tap-`, and `st-turbo2-` respectively.
