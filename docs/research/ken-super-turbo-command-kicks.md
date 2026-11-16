# New Ken's command kicks in World Super Turbo

Source is the World 940223 `ssf2t` set at Turbo 2 in MAME 0.289. The [main Ken audit](ken-super-turbo-audit.md) pins the ROM, EEPROM, player selection and skip accumulator. These tests use New Ryu as P1 and New Ken as P2. Positions are the arcade fighter axes. A plus sign in a clock means advancing game ticks after the write of Ken state 12, including elapsed contact freeze when reporting an actual hit.

## Commands and whiff

The direction numbers are facing-relative. LK, MK and HK use the same three move streams. `236K`, `632K` and `41236K` enter at CPUs PCs `$085304`, `$085436` and `$0855aa`, respectively. Tap and prolonged hold were replayed from two successive display phases for all three motions. In the same phase the complete `63214K` starts Tatsumaki at `$085728`, with a rising airborne attack box at +1. It does not select Nata Otoshi Geri. A neutral or Up plus LK edge two display samples after the last matching direction selected the grounded command kick in both phases.

| Kick | Tap active ticks | Tap forward step | Tap neutral | Held active ticks | Held forward step and neutral |
| --- | --- | --- | --- | --- | --- |
| Kama Barai Geri, `236K` | +7…9, +10…14 | 42 px at +29 | +36 | +7…8, +27…29, +30…32 | 22 px at +40 |
| Nata Otoshi Geri, `632K` | +16…18, +19…21 | 32 px at +30 | +35 | +16…17, +36…38, +39…41 | 22 px at +49 |
| Osoto Mawashi Geri, `41236K` | +12…14, +15…18 | 32 px at +27 | +34 | +12…13, +34…36, +37…39 | 26 px at +47 |

Advance and return timing comes from the write taps, rather than interpolating a skipped MAME frame. The step distances are the words loaded by `$086488` and `$0864f8`. The held path can set `Ken+$ec` at the first active tick, when the source checks any pressed kick in the live pad at `Ken+$392`. Its first observed latched writes are +7, +16 and +12, respectively. The animation switches to its held path at +9, +18 and +14. In the phase with the final direction on display 2120, holding through four, eleven or eight display inputs, respectively, selected tap, and one additional input selected heel.

The normal input checks prioritize `41236K` before native Tatsumaki, `236K` and `632K`. The three ground kick sequences use the same 32-entry stage-length table at `$0856da`, `$0852de` and `$085410`. The final direction seeds 12. Same-tick and later checks try the kick press edge first, then release. The terminal stage decrements at each advancing call. At stages 12 through 2 the kick edge remains eligible, for up to ten advancing ticks after the final direction. Write taps confirm +10 accepts and +11 rejects for all three motions in the first phase. The port uses this table with its existing deterministic RNG, whose stream differs from the CPS2 RNG.

With a live `236` kick window, an MK on the second Down or Down-Forward still enters Kama Barai Geri with full Super gauge. For the [crouching MK into Shoryureppa route](ken-super-turbo-audit.md#shoryureppa-game-tick-capture-for-153), hold the second Down long enough for the first `236` kick window to expire before pressing MK. The Super continues from that same Down into Down-Forward + Punch.

## Attack rectangles and close contacts

Each rectangle is `(forward X, Y above floor, X radius, Y radius)` from Ken's arcade axis. Attack and hit metadata is stored in separate 16-byte records.

| Motion | Tap rectangles | Held follow-up rectangles |
| --- | --- | --- |
| `236K` | ID 61 `(66,79,15,14)`, ID 62 `(96,82,22,13)` | ID 67 `(105,78,14,16)`, ID 68 `(89,47,30,16)` |
| `632K` | ID 63 `(62,70,14,16)`, ID 64 `(80,79,15,24)` | ID 69 `(105,78,14,16)`, ID 70 `(89,47,30,16)` |
| `41236K` | ID 65 `(58,74,15,11)`, ID 66 `(93,78,19,14)` | ID 71 `(118,78,14,16)`, ID 72 `(102,47,30,16)` |

These are representative phase-zero P2 results from a 55 px setup against New Ryu. Walk and pushbox separation before contact can change the actual gap. Damage is in arcade HP out of 144. Entries give `action tick: damage`.

| Motion and input | Standing clean | Standing guard | Crouching guard |
| --- | --- | --- | --- |
| `236K` tap | +7: 7, +24: 27 | +7: 1, +24: 6 | Whiff |
| `236K` held | +7: 7, +41: 21 | +7: 1 | +30: 21 |
| `632K` tap | +16: 2, +33: 26 | +16: 1, +33: 6 | +16: 1, +33: 6 |
| `632K` held | +16: 2, +50: 23 | +16: 1 | +16: 1, +53: 21 |
| `41236K` tap | +12: 27 | +12: 7, +29: 6 | Whiff |
| `41236K` held | +12: 27 | +12: 7, +48: 5 | +37: 21 |

The recorded clean opener and standard guard contacts freeze Ken for 14 ticks. The held heel's lower attack can hit crouch guard. Different positions or random damage rolls can change the HP outcomes. The port uses SF1 Ken art and native receiver poses, movement and reactions. The fixture's spacing-dependent follow-up contacts, defense recovery, random damage spread, and unsupported Old Ken MK proximity throw still need separate port comparisons.

## Reproduce

Follow the main audit's boot and selection script through display 2040. At display 2070 write P1 X 552, P2 X 722 and clear both subpixel words for whiff. For the close cases use P2 X 607. For input, let N be display 2120 or 2121. Hold each cardinal or diagonal on the frame pair starting at `N−2×(number of directions remaining)`. Finish at N with either a two-display kick tap or a 49-display hold. For whiff, pin Ryu Y to 600 on display 2115–2260. For standing guard, hold P1 Back from `N+1`. For crouching guard, hold P1 Down-Back from `N−3`. For delayed neutral or Up controls, send the kick plus that direction at `N+2`.

The raw probes used `Ken+$03` state, `+$06` axis X, `+$1a` animation and `+$ec` heel flag with the game tick at `$ff801d`. Ryu's HP tap at `Ryu+$2a` uses write PC `$0be64c`. To identify a strike, resolve the active animation's attack-box ID at `anim+$0c` through the fighter box table at `+$34`. For held timing compare the tap at PC `$08644c` with the animation branch at `$086458`. For the late input boundary, use N=2120 and move the button tap to 2128 or 2129, leaving the motion unchanged. Full local capture directories are `%TEMP%/ssf2-ryu-audit/ken-st-kick-special-live`, `ken-st-kick-special-hold-threshold`, `ken-st-kick-special-boxes` and `ken-st-kick-special-window`.
