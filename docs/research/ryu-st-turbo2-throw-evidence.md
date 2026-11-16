# Super Turbo Ryu throws at World Turbo 2

This follows the character-select and two-player inputs in the local audit at
`9646b61` on `codex/ryu-era-audit`. The source is the World 940223 `ssf2t`
set in MAME 0.289. The match is New or Old Ryu (P1) against New Ken (P2).

## Capture, release, and recovery

Elapsed tick 0 is the write setting the defender's throw word to `0xffff`.
Release is its first subsequent clear. Landing is the first ground-height
write after release. Recovery is the first corresponding neutral state write.
All ticks below are elapsed game ticks.

| Ryu input | Release | Ryu neutral | Ken land and full damage | Ken recovery | Ken land and neutral after softening |
| --- | ---: | ---: | ---: | ---: | ---: |
| Forward or back + MP / HP, shoulder | 44 | 74 | 79 | 153 | 94 / 100 |
| Forward or back + MK / HK, flip | 38 | 80 | 71 | 144 | 87 / 93 |

The four buttons select the two families with identical timing within each
family. New and Old Ryu share the measured throw timings. Forward throws send
Ken to the side he started on, while back throws send him across Ryu's original
side. The table is from a stage position with room for travel in both
directions. Ryu's axis stays fixed.
Both loadouts and throw families capture at an initial axis gap of 77 px
and miss at 78 px with either direction against standing neutral Ken.
The gap uses the raw integer fighter axes before capture.

At 32 px before capture, Ryu's axis starts at X650 and Ken's at X682.
At release, either family puts Ken 127 px from Ryu's pre-throw axis in
the launch direction. The release heights are 44 px for shoulder and
32 px for flip. The attached [per-tick trajectory](ryu-st-turbo2-throw-trajectory.tsv)
contains the last integer X/Y write on each game tick, carrying values forward
between writes. X is relative to Ryu's axis before capture and Y is height
above the floor (Y40). The trajectory samples are New Ryu against New Ken.
Each softened sample used a directional MP first sampled at +13.

At X552/X584, a full leftward throw reaches the stage's X296 limit and
Ken's recovery advances to +141 for shoulder and +132 for flip. The
leftward shoulder moves Ryu 7 px right at +43 in that setup and releases
Ken 120 px left of Ryu's initial axis. At X650/X682, the same directional
inputs preserve Ryu's axis and the full recovery times in the table.

With New Ken on P1 and New or Old Ryu on P2, release and first damage
stay at +44/+79 for shoulder and +38/+71 for flip. Ryu starts at X584.
Ken releases at X464/X711 for forward/back shoulder and X457/X711 for
forward/back flip. The fixed stage limit affects full leftward wakeup.
The corresponding first recovery transitions were +145/+153 for shoulder
and +142/+144 for flip.

On a full knockdown Ken starts an 8-tick turning animation on the same
tick as the first neutral write. A fresh directional LP cancels the turn
and starts a normal on the next game tick. At X552/X584, the verified
LP start ticks are +154/+142 for forward/back shoulder and +145/+133 for
forward/back flip. A press already sampled before the recovery transition
and still held during it did not start a normal in any of those four cases.
The unclamped samples' turns end at +161 for shoulder and +152 for flip.

## Softening and priority

New Ken's forward + MP input at +13 softens both families from either
direction. The same input at +14 fails. This holds for both Ryu loadouts.
Neutral MP at +13 fails for both New Ken and New Ryu.
Directional HK also succeeds at +13 and fails at +14 for New Ken
receiving either Ryu's shoulder and for New Ryu receiving Ken's forward
shoulder or HK. Old Ryu's HK at +13 fails against both Ken throws.
A softened throw deducts 16 of 144 source HP at release and leaves Ken in
airborne recovery. A full throw deducts 32 at the first landing and leaves
Ken knocked down. When New Ken throws Ryu, the same +13 response softens
Ken's shoulder for New Ryu and fails for Old Ryu. New Ken's shoulder release
and first full damage are +44 and +79. His forward and back HK throws both
release at +62, with first full damage at +79 and +101 respectively.
The softened forward HK deducts 16 HP at +62, with New Ryu neutral at +128.

### Simultaneous capture priority

With no active throw, the arcade selects fighter processing order at
`0x77f76–0x77fc0`. It calls `0x2e96` to advance the 16-bit word at
`0xff82d0`. The new seed is the old seed shifted right by one, with the
XOR of old bits 1 and 9 entering bit 15. The selector tests the new low
byte against alternating bits in `0x55555555`. An even low byte visits P2
first. An odd low byte visits P1 first. `0xff833e` records the selected side.
An active throw instead reuses its saved fighter order.

Both input words were present before capture in all twelve trials below.
The forced trials override only the selector's read of the post-rotate
word at PC `0x2eb4` with caller return address `0x77f90`. P1 held forward +
MP and P2 held forward + MP at 32 px. The capture winner matched
`0xff833e` in each trial.

| P1 Ryu | Capture tick | Pre-selector seed | Natural output and winner | Forced `0x0000` | Forced `0x8001` |
| --- | ---: | ---: | --- | --- | --- |
| New | 215 | `0xc780` | `0xe3c0`, P2 | P2 | P1 |
| New | 218 | `0x8f1e` | `0x478f`, P1 | P2 | P1 |
| Old | 219 | `0x23c7` | `0x11e3`, P1 | P2 | P1 |
| Old | 222 | `0xc88f` | `0xe447`, P1 | P2 | P1 |

The first selector call in either fixture was at display 1878, game tick
156, with seed `0x7420` after two calls from return address `0x7824a`.
The measured ordinary neutral updates make a second RNG call at return
address `0x29e00` after fighter dispatch. The first fixture update instead
made its trailing call at `0x779f2`. Captures can make additional calls,
including at return addresses `0x7fb4c`, `0x8584e`, and `0x85404`.

## Port behavior

The port uses one game tick per update for these throw clocks. The tested
New and Old Ryu throws use 214 Sub-HP for full damage and 107 when
softened, based on the measured 32 and 16 of 144 arcade HP. The port uses
its existing shoto throw and reaction art with the measured motion.
Mixed-era throws continue to use the existing response and match switch.
The port uses an independent 16-bit selector for simultaneous eligible
Classic shoto throws. It starts from the first observed pre-selector seed,
advances for the selection and trailing call on ordinary match updates,
and skips selection during capture. It applies the arcade parity rule
when both sides request an eligible direct throw. Character-specific
arcade RNG calls and selection-screen history can change the arcade's
later sequence. The port's other input paths use their existing order.

## Reproduction

Start with the follow-up input stream and `trace.lua` from
`docs/research/evidence/ryu-super-turbo` at `9646b61`. Run `ssf2t` with
the same saved EEPROM. At display 2070, after selecting the fighters,
write `0xff83a9=2` and `0xff8cd3=0x60` for World Turbo 2. Set fighter
integer axes through `+0x06` to 650 and 682 for unclamped trajectories
or 552 and 584 for the window, priority, and wake action probes. Zero
their `+0x08` X fractions. Keep the fight neutral until the throw input at display
2120 or 2121. A press on each selected display suffices for the family,
range, and priority probes. The +13/+14 boundary used a three-display
opening press and a one-display response ten or eleven displays later.
Vary the opening display by one to recover the target game ticks across
Turbo 2's skipped updates.

For exact timing, install 16-bit main CPU write taps on each fighter
structure's state `+0x02`, X `+0x06`, Y `+0x0a`, life `+0x2a`,
throw flags `+0x62`, and input `+0x392`. Log `0xff801d` at each write
and subtract the capture timestamp modulo 256. The first ground Y after
the initial lift is the first landing for Ryu's two throws. Ken's HK
animation can touch the ground before releasing his opponent, so a
ground-height sample during his capture is a different event.

For the priority trials, tap reads of `0xff82d0..0xff82d1` and log the
current `PC` plus the 32-bit return address at `USP & 0xffffff`. Return
`0x0000` or `0x8001` only for PC `0x2eb4`, return `0x77f90`, and current
input words `0x0021` on P1 and `0x0022` on P2. Pass through all other
reads. Record `0xff833e` and the writes to each fighter's throw byte
`+0x63` at `0x476c/0x4772`.
