# Both sound chips from the native engine over every recorded fight

This document records the boundary established by
issue #37, extended across
every recorded fight by
issue #48 (F8, evidence class
`TOOL`), and packaged with the fixed-fight listening kit and review protocol by
issue #49 (F9, evidence class
`HOST`): both translated sound programs — the samples CPU with its two
MSM5205s (`fight-samples.md`) and the music CPU with the YM2151
(`music-power-on.md`) — run from power-on and driven, from each fight's
fixture frame on, by the **native** engine's sound latch over the fixed fight
(`sf-fixed-fight-01`, rounds 1 and 2), the versus punch run (`sf-punch-01`),
and the winning fight (`sf-win-01`), compared device by device with the pinned
MAME reference and swept across every constant interrupt latency in `45–138`
cycles (`latch-timing.md`). The launcher plays both chips with the same mixer
at the band's middle latency (`92`). Code: `sf_audio::SamplesSound::tick`,
`sf_audio::mix::GameSound`, `check::render_mix`, `check::compare_dac_stream`,
`check::write_time_drift`, `check::native_at_reference_times`, the
`native_round2` example, `crates/sf-audio/tests/both_chips.rs`,
`crates/sf-host/src/audio.rs` (`LiveCapture`), `crates/sf-dev/src/main.rs`
(`kit`).

## The run

- **The native latch.** The native engine runs sf-fixed-fight-01's inputs
  from checkpoint 2389 of sf-cpu-01 (FIGHT! just erased) through frame
  5000: the round-1 fight, the KO, the result, round 2, its KO and result.
  Its screen hash equals the reference's on every frame after the fixture
  row (2390–4999, 2,610 frames), and its latch value on every frame. Each
  stepped frame gives the latch value and the vblank handler's cycles to
  the write.
- **The timeline.** The reference's own latch writes through the fixture
  frame 2389, whose vblank ran in the reference (the capture's row for it
  carries the checkpoint's latch with no handler cycles), then each native
  frame's write from 2390 at the frame start plus its handler cycles plus
  an assumed constant interrupt latency.
- **Both CPUs from power-on** on that timeline: no RAM fixture, no fitted
  phase.
- **The reference.** fight-audio-ym (the tap log and the YM2151 stream
  over the whole run), fight-audio-r2 (both MSM5205 streams at 384 kHz over
  frames 3830–5000), and fight-audio-a's `audio.wav` (MAME's `-wavwrite`
  recording, 48 kHz stereo). Round 2 is frames 3830..=4999: the round-2
  music, the fight, the KO and the result.

The comparisons, per device:

- **Samples CPU:** every port write in tick, port and data, and each
  write's time against the logged time.
- **MSM5205s:** both outputs once per channel tick (5 µs after the
  channel's next tick, as fight-samples.md), and at every 384 kHz stream
  sample, each nibble decoded six VCK clocks (15.6 µs) after its write.
- **Music CPU:** every YM2151 write in order, its sample and its chip
  clock.
- **YM2151:** every output sample.

## Per-device results over round 2

| Latch writes | Samples CPU: 143,569 port writes | MSM5205s per channel tick (156,157) | MSM5205 streams (14,991,360 samples) | Music CPU: 35,867 writes | YM2151 output (1,090,635 samples) |
| --- | --- | --- | --- | --- | --- |
| Native values at the reference's write times | identical, every write at its logged time | identical | identical | all on the exact chip clock | identical |
| Native, latency 51 (inside the window below) | identical data; every write up to 147 T-states off its logged time | identical | 37,280 differ (0.25%) | in order; 35,639 on the exact clock, at most 1 sample off | identical |
| Native, latency 45 (the band's low end) | as at 51 | identical | 37,280 differ | in order; 35,625 exact, at most 2 samples off | 320 differ |
| Native, latency 92 (the band's middle) | differ, from frame 2619 on | differ | 1,730,932 differ | in order; 35,801 exact, at most 2 samples off | 4,700 differ |
| Native, latency 138 (the band's high end) | differ, from frame 2619 on | differ | 1,732,146 differ | in order; 35,775 exact, at most 2 samples off | 5,007 differ |

No run reached an untranslated branch. Over the whole native span (frames
2389–4999) at the reference's write times, the samples CPU's 333,280
writes are all at their logged times and the music CPU's 79,521 writes all
on the exact clock, with its 2,433,887 output samples identical.

## The latch-timing limit

The native engine gives every write's handler cycles exactly; the
interrupt latency before the handler varies from frame to frame in the
reference (45–138 cycles) and is not modelled (latch-timing.md, scope
decision). A constant latency stands in for it. Swept one cycle at a time
over the band:

| Constant latency (cycles) | Samples CPU data and MSM5205 per tick, frames 2389–4999 | YM2151 output over round 2 |
| --- | --- | --- |
| 45–48 | identical | 320 samples differ |
| 49–53 | identical | identical |
| 54 | identical, through an f64 tie (below) | identical |
| 55–76 | differ from frame 2619 on (58: from frame 4569) | identical |
| 77–84 | differ from frame 2619 on | 2,251 differ |
| 85–112 | differ from frame 2619 on | 4,700 differ |
| 113–138 | differ from frame 2619 on | 5,007 differ |

- **Every constant latency from 49 through 53 cycles reproduces round 2
  device by device:** the samples CPU's writes in tick, port and data, both
  MSM5205 levels at every channel tick, the YM2151's every output sample
  (the test asserts 51). This window is fitted to this one fight. Another
  fight's commands could need a different constant, and no constant matches
  every frame of the reference. The launcher keeps the band's middle (92),
  as the scope decision set.
- **What no constant latency reproduces is the timing within the tick.** At
  every latency that keeps the data, 319,316 of the 333,280 samples-CPU
  writes from frame 2391 on are off their logged time, by up to 147
  T-states (41 µs), and over round 2 0.25% of the MSM5205 streams' 384 kHz
  samples differ: the level changes land up to 41 µs off within their
  tick. The
  first write off its time is at tick 318801, in frame 2391. The reference
  wrote that frame's command (0xa6) at latency 95, 149.1 µs into the frame;
  at a constant 45–54 the native write lands 5.1–6.25 µs earlier. The
  samples CPU presumably then takes a branch of different length at a
  poll without changing what it writes, and the shifted phase persists.
  With the reference's write times the timing is exact.
- **The samples CPU's data** differ from 55 cycles up, all from frame
  2619's impact command 0x84, except at 58. The reference wrote 0x84 at
  latency 45, 142.9 µs into the frame. The samples CPU's idle poll in tick
  349201 sees writes up to the end of a 500-ns scheduler slice, 19.0 µs
  into the tick. At 53 the native write lands before that point. At 55 it
  lands after it, the impact starts at a later poll, and the writes no
  longer match. At 54 the write falls exactly on the slice's end, which
  the model's strict `<` would miss; it is seen only because the f64 sum of
  the write time rounds a hair below the f64 slice end. At 58 the
  samples CPU's shifted phase moves that poll by about 0.3 µs, and the
  write is seen there through the same kind of tie at the next slice end;
  the data then first differ at frame 4569.
- **The music CPU** takes each write as an NMI a few chip clocks earlier or
  later with the assumed latency. Its writes stay in order and within two
  output samples at every latency of the band. The output samples that
  differ follow the writes that landed in a different output sample; with
  every write in its reference sample the output is identical. Over the
  whole span at latency 51, 2,453 of 2,433,887 output samples differ, all
  before round 2 (from frame 2453).
- **With the native values at the reference's own write times** every
  device is identical, timing included. The whole residual is the latency;
  the translated programs are not its source.

## Extension across every recorded fight (ticket #48, F8)

Ticket #48 (F8, evidence
class `TOOL`) extends the native-latch comparison across every recorded fight:
round 1 of the fixed fight (`sf-fixed-fight-01`), the versus punch run
(`sf-punch-01`), and the winning one-player fight through the power/timing
bonus (`sf-win-01`).

### 1. New audio-hook reference captures (two runs each, byte-identical)

Each new hook capture was recorded twice on the pinned MAME 0.289 (`-sound
none`, deterministic Lua tap/stream hook via `tools/reference/run_audio_probe.py`)
and verified byte-for-byte identical on `audio_taps.tsv`, every `hook_*.f32`
stream, and the sorted rows of `hook_index.tsv`:

| Capture pair (`tools/reference/local/captures/`) | Scenario | Hook window (frames / time) | Hooked devices & stream counts | Tap-log writes (from power-on) | Repeatability |
| --- | --- | --- | --- | --- | --- |
| `fight-audio-r1-a`, `fight-audio-r1-b` | `sf_fixed_fight_01.json` | `2389..=3830` (`39.800–63.840 s`) | `msm1`, `msm2` (`9,231,360` samples each at 384 kHz; `18,462,720` total) | `457,977` samples-CPU writes, `144,242` YM2151 writes | byte-identical |
| `punch-audio-hook-a`, `punch-audio-hook-b` | `sf_punch_01.json` | `2450..=3400` (`40.820–56.680 s`) | `ymsnd` (`887,053` samples at 55,930 Hz), `msm1`, `msm2` (`6,090,240` each; `12,180,480` total) | `198,710` samples-CPU writes, `96,596` YM2151 writes | byte-identical |
| `win-01-hook-a`, `win-01-hook-b` | `sf_win_01.json` | `2389..=6252` (`39.800–104.200 s`) | `ymsnd` (`3,601,892` samples at 55,930 Hz), `msm1`, `msm2` (`24,729,600` each; `49,459,200` total) | `653,385` samples-CPU writes, `190,736` YM2151 writes | byte-identical |
| `attract-01-hook-a`, `attract-01-hook-b` (`F15` / `#55`) | `sf_attract_01.json` | `0..=1648` (`0.000–27.470 s`) | `ymsnd` (`1,536,957` samples at 55,930 Hz; `1,536,211` in `0..=1647`), `msm1`, `msm2` (`10,552,321` each; `21,104,642` in `0..=1647`) | `73,246` samples-CPU writes, `42,759` YM2151 writes (over `0..=1647`) | byte-identical |

### 2. Every device at the reference's own write times (`native_at_reference_times`)

In every recorded fight, the native engine's screen hash matches the
reference's on every frame after the fixture, its latch value matches on every
frame, and every frame's implied interrupt latency lies inside `45–138` 68000
cycles. When the native latch values are placed at the reference's own write
times (`check::native_at_reference_times`), **every device and every hooked
stream is 100% identical across all three fights**:

| Fight & native span | Native screen hashes & latch values | Samples CPU port writes | MSM5205 per channel tick | MSM5205 384 kHz streams | Music CPU YM2151 writes | YM2151 55,930 Hz output |
| --- | --- | --- | --- | --- | --- | --- |
| `sf-fixed-fight-01` Round 1 (`2389..=3829`, splice `2390`) | part of `2,610` hashes / `2,611` latch values | `189,711` writes, `0` off logged time | `192,317` ticks (`67,186` active) identical | `18,462,720` samples, `0` differing | part of full span below | part of full span below |
| `sf-fixed-fight-01` Round 2 (`3830..=4999`, splice `2390`) | part of `2,610` hashes / `2,611` latch values | `143,569` writes, `0` off logged time | `156,157` ticks (`49,478` active) identical | `14,991,360` samples, `0` differing | `35,867` writes, `35,867` on exact clock | `1,090,635` samples, `0` differing |
| `sf-fixed-fight-01` full fight (`2389..=4999`, splice `2390`) | `2,610` hashes (`2390..=4999`) & `2,611` latch values equal | `333,280` writes, `0` off logged time | `348,474` ticks across R1 + R2 identical | `33,454,080` samples across R1 + R2, `0` differing | `79,521` writes, `79,521` on exact clock | `2,433,887` samples, `0` differing |
| `sf-punch-01` (`2450..=3399`, splice `2451`) | `949` hashes (`2451..=3399`) & `950` latch values equal | `68,968` writes, `0` off logged time | `126,877` ticks (`13,864` active) identical | `12,180,480` samples, `0` differing | `27,705` writes, `27,705` on exact clock | `885,558` samples, `0` differing |
| `sf-win-01` (`2389..=6251`, splice `2390`) | `3,862` hashes (`2390..=6251`) & `3,863` latch values equal | `528,845` writes, `0` off logged time | `515,197` ticks (`185,535` active) identical | `49,459,200` samples, `0` differing | `126,015` writes, `126,015` on exact clock | `3,600,959` samples, `0` differing |

**Z80 T-state step-end boundary at `tick 538801` of `sf-win-01` (`67.350125 s`, frame `4041`).**
At `tick 538801` in `sf-win-01`, the samples CPU's `in a,(1)` poll starts at
Z80 clock `241,082,880` (`t = 67.350146457 s`, `43 ns = 0.153` Z80 T-states
before the `500 ns` scheduler slice end `67.350146500 s`), and the 68000's
`0xff` latch write at `t_write = 67.350146625 s` (`125 ns` after the slice
end) has `floor(t_write * 3,579,545) = 241,082,880` — the exact same Z80
T-state (`+0.60` Z80 T-states after `t`). Because a Z80 step starting at clock
`C` finishes at `C + 1` (`t + 1 / CLOCK_HZ = 67.350146737 s`), MAME's Z80
cannot finish that T-state inside the slice ending at `67.350146500 s`
(`floor(67.350146500 * 3,579,545) == 241,082,880`), so a 68000 write on the
same Z80 T-state is already visible when the step completes. `SamplesSound::tick`
(`crates/sf-audio/src/lib.rs`) models this with
`latch.read_before(slice_end.max(t + 1.0 / samples::CLOCK_HZ))`. With that
boundary included, all `1,310,072` samples-CPU port writes from power-on
across the three scenarios (`457,977` in `sf-fixed-fight-01`, `198,710` in
`sf-punch-01`, `653,385` in `sf-win-01`) match the reference in tick, port,
data, and T-state time.

### 3. Constant-latency sweeps (`45–138` cycles) per fight

Sweeping a single constant interrupt latency `L` across the observed band
`45..=138` (`94` values per fight) gives the following latency windows and
first differing frames:

#### Summary of matching latency windows (`45–138`) by fight and device

| Fight & span | Samples CPU data (tick, port, data) & MSM5205 per-tick levels | Samples CPU T-state write times & MSM5205 384 kHz streams | Music CPU write order (`≤ 2` samples lag) | YM2151 55,930 Hz output samples | Joint window (Samples data + MSM5205 ticks + YM2151 output) |
| --- | --- | --- | --- | --- | --- |
| `sf-fixed-fight-01` Round 1 (`2389..=3829`) | **`45–54`** (`55–138`: first diff frame **`2619`**, `tick 349201`) | none (first diff frame **`2391`**, `tick 318801`, on `45–61`; **`2451`** on `62–74`; **`2529`** on `75–80`; **`2535`** on `81–138`) | **`45–138`** (`45–58` within 1 sample in R1; `59–138` 2-sample lag at frame **`2513`**) | **`53–107`** (`45–52`: first diff frame **`2453`**; `108–138`: frame **`2502`**) | **`53–54`** |
| `sf-fixed-fight-01` Round 2 (`3830..=4999`) | **`45–54`** (`55–138`: DAC first diff frame **`3920`**, `tick 522681`, from R1 divergence at **`2619`**) | none (first diff frame **`3920`**, `65.3352 s`, across `45–138`) | **`45–138`** | **`49–76`** (`45–48`: first diff frame **`4281`**; `77–138`: frame **`4579`**) | **`49–54`** |
| `sf-fixed-fight-01` full fight (`2389..=4999`) | **`45–54`** (`55–138`: frame **`2619`**) | none (first diff frame **`2391`**, `tick 318801`) | **`45–138`** (`53–56` within 1 sample; `45–52` lag 2 at **`2453`**; `57–58` at **`4301`**; `59–138` at **`2513`**) | **`53–76`** (`45–52`: frame **`2453`**; `77–107`: frame **`4579`**; `108–138`: frame **`2502`**) | **`53–54`** |
| `sf-punch-01` (`2450..=3399`) | **`45–138`** (entire band identical) | **`45–113`** (`0` drift, `0` stream diffs; `114–138`: first diff frame **`2474`**, `tick 329868`) | **`45–138`** (`45–108` within 1 sample; `109–112` lag 2 at **`3386`**; `113–138` at **`2996`**) | **`46–83`** (`45`: first diff frame **`2853`**; `84–133`: frame **`3162`**; `134–138`: frame **`2856`**) | **`46–83`** (also 100% identical on 384 kHz streams & T-states) |
| `sf-win-01` (`2389..=6251`) | **none** (`45–81`: first diff frame **`2535`**, `tick 338001`; `82–138`: first diff frame **`2595`**, `tick 346001`) | **none** (first diff frame **`2391`** on `45–61`; **`2451`** on `62–83`; **`2544`** on `84–138`) | **`45–80`** (`81–101`: order differs at frame **`3511`**; `102–138`: at frame **`3487`**, write `#37976`) | **none over `2389..=6251`** (`45–80`: identical through frame `5636`, first diff frame **`5637`**; `81–138`: first diff frame **`3511`**) | **none** (through frame `5636`, YM2151 matches on `45–78`, while samples data need `≥ 82` at frame `2535` and `≤ 81` at frame `2595`) |

#### Per-latency breakdown: `sf-fixed-fight-01` (`2389..=4999`, Round 1 `2389..=3829` + Round 2 `3830..=4999`)

| Constant latency (cycles) | Samples CPU data (`333,280` writes) & MSM5205 per-tick (`192,317` R1 + `156,157` R2) | MSM5205 384 kHz stream diffs (`18,462,720` R1 / `14,991,360` R2) & first diff frame | Music CPU writes (`79,521` full / `35,867` R2) | YM2151 output diffs (`2,433,887` full / `1,090,635` R2) & first diff frame |
| --- | --- | --- | --- | --- |
| `45–48` | identical (`319,316` writes drift up to `+147` T-states from frame **`2391`**) | R1: `51,393` (frame **`2391`**); R2: `37,280` (frame **`3920`**) | in order, max lag 2 samples (frame **`2453`**) | full: `2,773` differ (frame **`2453`**); R2: `320` differ (frame **`4281`**) |
| `49–52` | identical (as `45–48`) | R1: `51,393` (frame **`2391`**); R2: `37,280` (frame **`3920`**) | in order, max lag 2 samples (frame **`2453`**) | full: `2,453` differ (frame **`2453`**, all in R1); R2: **identical (`0` differ)** |
| **`53`** | **identical** (as `45–48`) | R1: `51,393` (frame **`2391`**); R2: `37,280` (frame **`3920`**) | **in order within 1 sample** (`79,014`–`79,070` exact on `53–56`) | **full & R2: identical (`0` differ)** |
| **`54`** | **identical** via f64 slice-end tie (`319,316` writes drift up to `+144` T-states from frame **`2391`**) | R1: `31,414` (frame **`2391`**); R2: `36,643` (frame **`3920`**) | **in order within 1 sample** | **full & R2: identical (`0` differ)** |
| `55–76` | differ from frame **`2619`** (`tick 349201`, write `#46414`); R2 DAC from frame **`3920`** (`tick 522681`) | R1: `1,843,537`–`1,870,269` (frame **`2391`** on `55–61`, **`2451`** on `62–74`, **`2529`** on `75–76`); R2: `1,729,482`–`1,732,908` (frame **`3920`**) | in order (`55–56` lag 1; `57–58` lag 2 at frame **`4301`**; `59–76` lag 2 at frame **`2513`**) | **full & R2: identical (`0` differ)** |
| `77–84` | differ from frame **`2619`** (`tick 349201`) | R1: `1,866,309`–`1,868,398` (frame **`2529`** on `77–80`, **`2535`** on `81–84`); R2: `1,727,931`–`1,732,828` (frame **`3920`**) | in order, max lag 2 samples (frame **`2513`**) | full & R2: `2,251` differ (frame **`4579`**, `sample 4,268,401`) |
| `85–107` (incl. **`92`**) | differ from frame **`2619`** (`tick 349201`) | R1: `1,868,398` (frame **`2535`**); R2: `1,730,932` (frame **`3920`**) | in order, max lag 2 samples (frame **`2513`**) | full & R2: `4,700` differ (frame **`4579`**, `sample 4,268,401`) |
| `108–112` | differ from frame **`2619`** (`tick 349201`) | R1: `1,868,398` (frame **`2535`**); R2: `1,730,932` (frame **`3920`**) | in order, max lag 2 samples (frame **`2513`**) | full: `4,738` differ (frame **`2502`**, `sample 2,332,693`; R2: `4,700` from frame **`4579`**) |
| `113–138` | differ from frame **`2619`** (`tick 349201`) | R1: `1,868,307`–`1,868,398` (frame **`2535`**); R2: `1,730,932`–`1,732,146` (frame **`3920`**) | in order, max lag 2 samples (frame **`2513`**) | full: `5,045` differ (frame **`2502`**, `sample 2,332,693`; R2: `5,007` from frame **`4579`**) |

#### Per-latency breakdown: `sf-punch-01` (`2450..=3399`, splice `2451`, `949` native frames)

| Constant latency (cycles) | Samples CPU data (`68,968` writes) & MSM5205 per-tick (`126,877` ticks) | Samples CPU write times & MSM5205 384 kHz streams (`12,180,480` samples) | Music CPU writes (`27,705` writes) | YM2151 output (`885,558` samples) & first diff frame |
| --- | --- | --- | --- | --- |
| `45` | **identical** | **identical (`0` writes off, `0` stream diffs)** | in order within 1 sample (`27,675` exact) | `344` differ (frame **`2853`**, `sample 2,659,488`) |
| **`46–83`** | **identical** | **identical (`0` writes off, `0` stream diffs)** | **in order within 1 sample** (`27,675`–`27,699` exact) | **identical (`0` differ)** |
| `84–108` (incl. **`92`**) | **identical** | **identical (`0` writes off, `0` stream diffs)** | in order within 1 sample (`27,674`–`27,699` exact; `27,699` at `92`) | `5,946` differ (frame **`3162`**, `sample 2,947,522`) |
| `109–113` | **identical** | **identical (`0` writes off, `0` stream diffs)** | in order, max lag 2 samples (frame **`3386`** on `109–112`, **`2996`** on `113`) | `5,946` differ (frame **`3162`**, `sample 2,947,522`) |
| `114–133` | **identical** | `19,233` writes drift `-144` T-states from frame **`2474`** (`tick 329868`); `3,559` stream samples differ (frame **`2474`**, `41.2352 s`, chip 0) | in order, max lag 2 samples (frame **`2996`**) | `5,946` differ (frame **`3162`**, `sample 2,947,522`) |
| `134–138` | **identical** | `19,233` writes drift `-144` T-states from frame **`2474`**; `3,559` stream samples differ (frame **`2474`**) | in order, max lag 2 samples (frame **`2996`**) | `5,950` differ (frame **`2856`**, `sample 2,662,291`) |

In `sf-punch-01`, every constant latency in **`46–83`** (`38` cycles wide)
reproduces every device and every stream sample identically — including exact
T-state write times (`0` drift) and all `12,180,480` 384 kHz MSM5205 stream
samples (`0` differing). Above `113`, frame `2474`'s `0x81` command (whose
reference latency was `111`, written at `41.233518875 s`) crosses the poll's
step end at `tick 329868` without changing the nibble sequence, shifting
`19,233` write times by `-144` T-states (`3,559` of `12,180,480` stream
samples, `0.029%`).

#### Per-latency breakdown: `sf-win-01` (`2389..=6251`, splice `2390`, `3,862` native frames)

| Constant latency (cycles) | Samples CPU data (`528,845` writes) & MSM5205 per-tick (`515,197` ticks) | MSM5205 384 kHz stream diffs (`49,459,200` samples) & first diff frame | Music CPU writes (`126,015` writes) | YM2151 output (`3,600,959` samples) & first diff frame |
| --- | --- | --- | --- | --- |
| `45–78` | differ from frame **`2535`** (`tick 338001`, `42.250125 s`, write `#31714`, DAC `tick 338001`) | `16,584,784`–`16,588,524` differ (frame **`2391`** on `45–61`, **`2451`** on `62–78`) | in order, max lag 2 samples (`125,825`–`125,876` exact; first lag 2 at frame **`2513`** on `45–47`, **`2951`** on `48–78`) | **identical through frame `5636` (`3,006,691` samples)**; `238` differ (`0.0066%`) from frame **`5637`** (`sample 5,254,696`) |
| `79–80` | differ from frame **`2535`** (`tick 338001`) | `16,585,093` differ (frame **`2451`**) | in order, max lag 2 samples (frame **`2951`**) | `5,602` differ from frame **`5637`** (`sample 5,254,696`) |
| `81` | differ from frame **`2535`** (`tick 338001`) | `16,585,093` differ (frame **`2451`**) | write order differs at frame **`3511`** (`115,150` exact) | `78,456` differ from frame **`3511`** (`sample 3,273,272`) |
| `82–89` | pass frame `2535`; differ from frame **`2595`** (`tick 346001`, `43.250125 s`, write `#46700`, DAC `tick 346015`) | `16,554,033`–`16,556,431` differ (frame **`2451`** on `82–83`, **`2544`** on `84–89`) | write order differs at frame **`3511`** (`115,150` exact) | `78,456` differ from frame **`3511`** (`sample 3,273,272`) |
| `90–101` (incl. **`92`**) | differ from frame **`2595`** (`tick 346001`, DAC `tick 346015`) | `16,554,033`–`16,555,580` differ (frame **`2544`**) | write order differs at frame **`3511`** (`114,745` exact at `92`) | `78,218` differ from frame **`3511`** (`sample 3,273,272`) |
| `102–138` | differ from frame **`2595`** (`tick 346001`, DAC `tick 346015`) | `16,554,849`–`16,556,881` differ (frame **`2544`**) | write order differs at frame **`3487`** (write `#37976`, `sample 3,250,425`) | `78,218` (`102–103`), `78,707` (`104–114`), `98,867` (`115–121`), `98,884` (`122–128`), or `105,049` (`129–138`) differ from frame **`3511`** |

Why no single constant latency fits `sf-win-01`:
- **Samples CPU (`2535` vs `2595`):** Once frame `2391`'s command (`0xa6`,
  reference latency `95`) shifts the samples CPU's poll phase under a constant
  latency, two later commands bracket the band from opposite sides:
  - At frame **`2535`** (`tick 338001`, `42.250125 s`), the reference wrote
    `0x89` at latency `75` (`42.250148000 s`), which in the reference's poll
    phase landed after `tick 338001`'s slice end and was read at `tick 338003`.
    With the shifted poll phase, constant latencies **`45–81`** place that
    write before `tick 338001`'s slice end so it is read two ticks early;
    only latencies **`82–138`** match frame `2535`.
  - Sixty frames later, at frame **`2595`** (`tick 346001`, `43.250125 s`), the
    reference wrote `0x89` at latency `45` (`43.250144250 s`), which was read
    at `tick 346001`. Under latencies **`82–138`**, that write lands after
    `tick 346001`'s slice end and is read two ticks late (`tick 346003`,
    first DAC tick difference at `346015`).
  Because `45–81` and `82–138` are disjoint, no single constant latency can
  satisfy both polls.
- **Music CPU / YM2151 (`5637` vs `3511`):** On **`45–78`**, all `126,015`
  music-CPU writes stay in order within 2 samples and **all `3,006,691` YM2151
  output samples from frame `2389` through frame `5636` (both rounds of the
  fight and the entire power/timing bonus gameplay) are 100% identical**; near
  the end of the bonus tally at frame **`5637`** (`sample 5,254,696`), a write
  crosses a sample boundary and `238` of the `3,600,959` output samples
  (`0.0066%`) differ. On **`81–138`**, an earlier write crosses a sample
  boundary in Round 2 at frame **`3511`** (`sample 3,273,272`).

### 4. Scope note (open decision 6)

1. **Both sound programs are exact:** When driven at the reference's own latch
   write times (`native_at_reference_times`), every device (`SamplesSound` port
   writes and T-state times, both `Msm5205` 384 kHz streams, `MusicProgram`
   chip-clock writes, and `Ym2151` 55,930 Hz output samples) is **100%
   identical** across `sf-fixed-fight-01` (rounds 1 and 2), `sf-punch-01`, and
   `sf-win-01`.
2. **No single constant latency fits every fight:** The 68000's vblank
   interrupt latency varies from frame to frame across `45–138` cycles
    depending on which instruction the 68000 was executing when vblank fired,
   and `ADR-0001`'s structured Rust translation does not model per-instruction
   68000 execution inside the task loops (`latch-timing.md`). A constant
   latency fits `sf-punch-01` on `46–83` and `sf-fixed-fight-01` on `53–54`
   (`49–54` over round 2), and has no overlap on `sf-win-01` (`45–81` vs
   `82–138` on the samples CPU; `45–78` through frame `5636` on the YM2151).
3. **Launcher setting and no tolerance:** Per the scope decision and open
   decision 6 (`docs/planning/arcade-flow-batch.md`), this finding is recorded
   as a scope note. **The launcher stays at `92` (`IRQ_LATENCY_MID`) unless the
   user decides otherwise (open decision 6). No tolerance is adopted.**

## The mix against MAME's recording

sf.cpp routes the YM2151's two outputs to the speaker's two channels at
0.6 and each MSM5205 to both channels at 1.0; MAME's recording saturates at
full scale. `mix::GameSound` mixes the same way: each MSM5205 as a step
function that changes when a nibble is decoded, 15.6 µs after its write,
the YM2151's 55930 Hz output linearly interpolated, both channels clamped
at ±1. The measurement takes the mono average of the two channels, as
`read_wav_mono` reads MAME's file.

Round 2 (frames `3830..=4999`, `19.500 s`, `936,000` samples at `48 kHz`),
native at latencies `51` (`FITTED_LATENCY`) and `92` (`IRQ_LATENCY_MID`, the
launcher's latency; recorded with no pass threshold per ticket #49) against
MAME's `audio.wav` (`63.833–83.333 s`):

| Measure | Native mix (latency `51`) | Native mix & live capture (latency `92`) | MAME's recording (`audio.wav`) |
| --- | --- | --- | --- |
| Onset / active span | `180 ms` / `19,225 ms` | `180 ms` / `19,225 ms` | `180 ms` / `19,225 ms` |
| RMS (`ratio vs MAME`) | `0.1003` (`1.0037`, `+0.37%`) | `0.1003` (`1.0045`, `+0.45%`) | `0.0999` |
| Peak | `1.0000` | `1.0000` | `1.0000` |
| Zero crossings | `1862 /s` | `1858 /s` | `1584 /s` |
| Spectral centroid | `3445 Hz` | `3445 Hz` | `3040 Hz` |
| Normalized cross-correlation vs MAME (`20 ms` lag search) | `0.9895` at `4` samples (`0.08 ms`) lag | `0.9743` at `4` samples (`0.08 ms`) lag | — |
| Least-correlated second vs MAME (`2 ms` lag search) | `0.9800` at `71.83 s` | `0.8750` at `64.83 s` | — |
| Normalized cross-correlation vs latency `51` | — | `0.9843` at `0` samples (`0.00 ms`) lag (least-correlated second `0.8877` at `64.83 s`) | — |

Whole fixed fight (frames `2389..=4999`, `43.517 s`, `2,088,800` samples at
`48 kHz`, `39.817–83.333 s`), native at latencies `51` and `92` against MAME's
`audio.wav`:

| Measure | Native mix (latency `51`) | Native mix & live capture (latency `92`) | MAME's recording (`audio.wav`) |
| --- | --- | --- | --- |
| Onset / active span | `0 ms` / `43,405 ms` | `0 ms` / `43,405 ms` | `0 ms` / `43,420 ms` |
| RMS (`ratio vs MAME`) | `0.0973` (`1.0037`, `+0.37%`) | `0.0973` (`1.0039`, `+0.39%`) | `0.0969` |
| Peak | `1.0000` | `1.0000` | `1.0000` |
| Zero crossings | `1974 /s` | `1970 /s` | `1646 /s` |
| Spectral centroid | `3344 Hz` | `3330 Hz` | `2498 Hz` |
| Normalized cross-correlation vs MAME (`20 ms` lag search) | `0.9889` at `4` samples (`0.08 ms`) lag | `0.9743` at `4` samples (`0.08 ms`) lag | — |
| Least-correlated active second vs MAME (`2 ms` lag search) | `0.9359` at `61.82 s` | `0.8588` at `43.82 s` (`0.8609` vs `51` at `43.82 s`) | — |

At latency `45` on round 2 the measures are the same as at `51`. At the
reference's write times, where every device output is identical, they are the
same too: correlation `0.9895`, zero crossings `1861/s`, RMS ratio `1.004`.
The remaining difference between latency `51` and MAME's recording is in the
export path, not in the sound programs: MAME's band-limiting resampler to
`48 kHz` against the linear interpolation and DAC steps here. `music-phrase.md`
and `hit-sound.md` found the same for each chip alone. Byte-identical output
audio is out of the specification's scope. At latency `92`, the least-correlated
seconds (`43.82 s` in round 1, covering frame `2619` at `43.650 s`, and
`64.83 s` in round 2, covering frame `3920` at `65.335 s`) correspond directly
to the samples-CPU poll boundary at frame `2619` documented above.

## Fixed-fight listening kit and review protocol (ticket #49, F9)

Ticket #49 (F9, evidence
class `HOST`) builds the local listening kit for the fixed fight (`sf-fixed-fight-01`)
and commits the protocol and verdict table for the human listening review
(ticket F30 / `#70`).

### 1. Generating the local renders

Run from the workspace root or `crates/sf-dev`:

```
cargo run -p sf-dev --release -- kit
```

All WAV renders are written into `tools/reference/local/renders/` (`48,000 Hz`,
16-bit mono PCM, local-only and gitignored under `/tools/reference/local/`):

| Local file (`tools/reference/local/renders/`) | Span (frames / machine time) | Duration & samples | Source |
| --- | --- | --- | --- |
| `round2-mame-reference.wav` | `3830..=4999` (`63.833–83.333 s`) | `19.500 s` (`936,000` samples) | MAME `fight-audio-a/audio.wav` (`-wavwrite`), mono-averaged and sliced to `3830..=4999` |
| `round2-native-mix-51.wav` | `3830..=4999` (`63.833–83.333 s`) | `19.500 s` (`936,000` samples) | Native batch mix (`check::render_mix`) at fitted latency `51` |
| `round2-native-mix-92.wav` | `3830..=4999` (`63.833–83.333 s`) | `19.500 s` (`936,000` samples) | Native batch mix (`check::render_mix`) at launcher latency `92` (`IRQ_LATENCY_MID`) |
| `round2-launcher-live-92.wav` | `3830..=4999` (`63.833–83.333 s`) | `19.500 s` (`936,000` samples) | Launcher live output (`audio::LiveCapture`) captured during a scripted `CpuMachine` run of `sf-fixed-fight-01` (`P1_KEYS.inputs`) at latency `92` |
| `fight-mame-reference.wav` | `2389..=4999` (`39.817–83.333 s`) | `43.517 s` (`2,088,800` samples) | MAME `fight-audio-a/audio.wav`, mono-averaged and sliced to `2389..=4999` |
| `fight-native-mix-51.wav` | `2389..=4999` (`39.817–83.333 s`) | `43.517 s` (`2,088,800` samples) | Native batch mix (`check::render_mix`) over `2389..=4999` at latency `51` |
| `fight-native-mix-92.wav` | `2389..=4999` (`39.817–83.333 s`) | `43.517 s` (`2,088,800` samples) | Native batch mix (`check::render_mix`) over `2389..=4999` at latency `92` |
| `fight-launcher-live-92.wav` | `2389..=4999` (`39.817–83.333 s`) | `43.517 s` (`2,088,800` samples) | Launcher live output (`audio::LiveCapture`) over `2389..=4999` during the scripted run at latency `92` |

**Live capture vs batch mix equality:**
- `round2-launcher-live-92.wav` vs `round2-native-mix-92.wav`: **`0` of `936,000` samples differ** (and the two WAV files are byte-for-byte identical, `1,872,044` bytes).
- `fight-launcher-live-92.wav` vs `fight-native-mix-92.wav`: **`0` of `2,088,800` samples differ** (and the two WAV files are byte-for-byte identical, `4,177,644` bytes).

### 2. Listening protocol (for ticket F30 / `#70`)

1. **Generate the local renders** with `cargo run -p sf-dev --release -- kit` and verify that the command exits `0`.
2. **Round-2 aligned A/B listening (`19.500 s`, frames `3830..=4999`):**
   Load `round2-mame-reference.wav`, `round2-native-mix-51.wav`, `round2-native-mix-92.wav`, and `round2-launcher-live-92.wav` into an audio player or DAW aligned at `0.000 s` (machine time `63.833 s`):
   - `0.00–1.50 s` (`63.83–65.33 s`, frames `3830..3920`): Round-2 start, music entry, and opening exchanges; at `1.50 s` (`65.335 s`, frame `3920`), compare `round2-native-mix-92.wav` against `round2-native-mix-51.wav` and `round2-mame-reference.wav` where the shifted samples-CPU poll phase from frame `2619` first affects Round 2's MSM5205 stream (`least-correlated second = 0.8750` at `1.00–2.00 s` / `64.83 s`).
   - `7.40–15.80 s` (`71.23–79.63 s`, frames `4274..4778`): Retsu's five Round-2 hits (`4278`, `4346`, `4440`, `4567`, and the KO blow at `4772`), including `8.00–9.00 s` (`71.83 s`, least-correlated second `0.9800` between `51` and MAME) and `12.48 s` (`76.317 s`, frame `4579`, where `4,700` of `1,090,635` YM2151 samples differ at latency `92`).
   - `15.80–19.50 s` (`79.63–83.33 s`, frames `4778..=4999`): Round-2 KO fanfare and result tally.
3. **Full-fight aligned A/B listening (`43.517 s`, frames `2389..=4999`):**
   Compare `fight-mame-reference.wav`, `fight-native-mix-51.wav`, `fight-native-mix-92.wav`, and `fight-launcher-live-92.wav` from `0.000 s` (machine time `39.817 s`), paying particular attention to:
   - `3.67–4.67 s` (`43.48–44.48 s`, frames `2609..2669`): P1's light punch (`2601`), Retsu's counter-hit (`2617`, command `0x84` at frame `2619`, `3.83 s` into the file, where latency `92` misses the `tick 349201` poll slice and takes `0x84` at the next poll), and P1's light kick (`2662`).
   - `15.65–24.02 s` (`55.47–63.83 s`, frames `3328..3830`): Round-1 KO, victory jingle, bonus tally, and transition into Round 2.
4. **Live interactive launcher check:**
   Run `cargo run -p sf-dev --release -- cpu` (and `cargo run -p sf-dev --release` for the versus fixture) on the host audio device and listen for pre-roll continuity at launch, stereo balance, latency, and absence of underruns or glitches during play.
5. **Record the verdict** in the table below (and in `docs/reference/milestone-audit.md` under ticket F30 / `#70`), and open a follow-up ticket for any finding.

### 3. Verdict table (ticket F30 / `#70`)

| Axis | Files / mode compared | Machine reference summary | Human listening verdict (ticket F30 / `#70`) | Notes / follow-up tickets |
| --- | --- | --- | --- | --- |
| **Timing** | `round2-native-mix-51.wav` & `round2-native-mix-92.wav` vs `round2-mame-reference.wav`; `fight-*.wav` | Onset `180 ms` and active span `19,225 ms` identical on Round 2 (`0 ms` / `43,405 ms` vs `43,420 ms` on full fight); cross-correlation peak at `+4` samples (`0.08 ms`) lag | **Pass** — *"Everything else like the timings seem to match."* | None (all cue onsets, cadence, and durations match). |
| **Pitch** | `round2-native-mix-51.wav` & `round2-native-mix-92.wav` vs `round2-mame-reference.wav`; `fight-*.wav` | YM2151 clocked at `3,579,545 Hz` (`55,930 Hz` output) and MSM5205 at `384 kHz` (`8 kHz` ticks); zero-crossing rates `1862 /s` (`51`) and `1858 /s` (`92`) vs `1584 /s` (MAME band-limited resampler) | **Pass** — Pitches of both YM2151 FM music/effects and MSM5205 ADPCM voices/impacts match the MAME reference. | None. |
| **Character** | `round2-native-mix-51.wav` & `round2-native-mix-92.wav` vs `round2-mame-reference.wav`; `fight-*.wav` | Spectral centroid `3445 Hz` (`51` & `92`) vs `3040 Hz` (MAME) on Round 2 (`3344 Hz` vs `2498 Hz` full fight) due to linear interpolation + step DAC vs MAME's band-limiting resampler; RMS ratio `1.0037` (`51`) and `1.0045` (`92`) | **Pass (with noted resampler character difference)** — *"The only difference I could discern from the mame reference and the others is that the mame reference has a 'softer' character, even if slightly. Almost as if there's a lp filter on it. But I'm not entirely sure. It's close for sure but the others seem to sound more 'crisp'."* | Matches the machine-measured spectral centroid (`3040 Hz` MAME vs `3445 Hz` native on Round 2; `2498 Hz` vs `3344 Hz` full fight) caused by MAME's `sound_stream` downsampling filter vs native zero-order hold (MSM5205) and linear interpolation (YM2151); byte-identical output audio / resampler modeling is out of scope (`README.md`). |
| **Native against MAME** | `round2-native-mix-51.wav` & `round2-native-mix-92.wav` vs `round2-mame-reference.wav` (and `fight-*.wav`) | At `51`: Round-2 devices identical, mix correlation `0.9895` (`0.9889` full fight). At `92`: correlation `0.9743` (`0.9843` vs `51` on Round 2, `0.9850` on full fight), differing from frame `2619` (`3.83 s` in `fight-*.wav`) | **Pass** — No audible divergence between latency `51`, latency `92`, and MAME beyond the slight high-frequency rolloff of MAME's resampler. | None (`IRQ_LATENCY_MID = 92` retained for the launcher). |
| **The live launcher** | `round2-launcher-live-92.wav` vs `round2-native-mix-92.wav`, `fight-launcher-live-92.wav` vs `fight-native-mix-92.wav`, and interactive `cargo run -p sf-dev --release -- cpu` | Scripted live capture equals batch mix at latency `92` sample for sample (`0 / 936,000` on Round 2; `0 / 2,088,800` on full fight); interactive host playback reviewed by ear in F30 | **Pass** — Live launcher playback matches the batch mix and plays cleanly during interactive 1P and 2P sessions (#70 / #71). | First-launch terminal warning on missing `%APPDATA%\sf1-port\bindings.toml` noted during #71 and fixed in `crates/sf-dev/src/main.rs` (`load_active_bindings` / `smoke`). |

## The launcher

The development launcher (launcher.md) now plays both chips through
`mix::GameSound`, in stereo. The sound starts from power-on and is
pre-rolled to the fixture frame on the reference's latch writes through
that frame, the start the tests above verify:

- the one-player fixture (checkpoint 2389 of sf-cpu-01) on fight-audio-a's
  writes, whose frames equal sf-cpu-01's through 2389;
- the versus fixture (checkpoint 2450 of sf-move-01) on hit-sound-run-a's
  writes (sf-punch-01), whose frames equal sf-move-01's through 2450. From
  power-on on those writes, the music CPU reproduces all 68,891 of the
  reference's YM2151 writes through frame 2450 on the exact chip clock.

The round music and any voice still playing at the fixture are now heard.
From the first stepped frame on, the engine's latch drives both programs,
at the band's middle latency.

**Frames pushed while the sound runs.** A music-CPU handler cannot stop
halfway, and its chains run past the point where a player stops it: up to
23.6 ms over frames 2389–4353 of this fight. A first version that ran the
music CPU only to the start of the last pushed frame took 9 NMIs late over
those frames, and its YM2151 output diverged from frame 2545 on. The music
CPU therefore runs on its own thread, and whenever it reaches a time the
pushed writes do not cover, it waits, inside a handler if need be, for the
next frame. A frame's write lies at or after the frame's start. The samples
CPU runs to a millisecond before that time. The live output equals the run
with every write known from the start, by construction; the tests confirm
it sample for sample. While the output plays, the live player forgets the
writes both CPUs have passed and takes its chip output every 0.1 s, so its
memory stays bounded.

The launcher's smoke check, run on the recording host (Windows 10, the
default output device):

```
ok   window opened
ok   audio output opened (default device)
ok   sound pre-rolled to frame 2450 on the reference's 2450 latch writes through it (0.93 s)
ok   20 distinct keys bound for P1 and P2
ok   versus fixture: 240 frames of scripted P1 and P2 play on translated paths
ok   versus fixture: sound commands reached the latch (16 frames)
ok   versus fixture: the music CPU stayed on translated paths
ok   versus fixture: the output played 1542 ms of the run, peak level 0.695
ok   one-player fixture: 240 frames of scripted P1 play against Retsu on translated paths
ok   one-player fixture: the music CPU stayed on translated paths
ok   one-player fixture: the output played 1520 ms of the run, peak level 0.426
smoke check passed
```

## Complete-game sessions from power-on on the native latch (ticket #143, F71)

Ticket #143 (F71, evidence
class `TOOL`) runs both translated sound programs from power-on (`from: 1`, no
fixture and no pre-roll) on the native engine's sound latch over the two
complete-game sessions (`sf-full-2p-01`, `0..=21299`, `355.00 s` and
`sf-full-1p-01`, `0..=44199`, `736.67 s`), compared device by device against two
byte-identical MAME audio-probe captures per session (`full-2p-01-hook-a/b` and
`full-1p-01-hook-a/b`).

### 1. At the reference's write times (`native_at_reference_times(..., 1)`)

With the native engine's latch values placed at MAME's logged write times from
power-on (`from: 1`, frame 1 exact at `186,957` cycles = `0.023369625 s`, frames
`2..=last` within `45.00..=138.33` cycles), **every device is 100% identical to
the pinned MAME reference over both complete games**:

| Session (`0..=last`) | Native latch writes (`frames 2..=last` latency band) | Samples-CPU port writes (`off > 0.5` T-states) | MSM5205 DAC channel ticks (`active`) | MSM5205 `384 kHz` stream samples (`differing`) | Music-CPU YM2151 writes (`exact`, `lag`, `unrecovered`) | YM2151 `55,930 Hz` output samples (`differing`) |
| --- | --- | --- | --- | --- | --- | --- |
| `sf-full-2p-01` (`0..=21299`, `355.00 s`) | `21,299` (`45.00..=138.33` cycles) | `1,762,231` (`0` off) | `2,839,837` (`453,190` active, `0` diff) | `272,624,642` (`0` diff) | `639,631` (`639,631` exact, `lag = 0`, `0` unrecovered) | `19,854,032` (`0` diff) |
| `sf-full-1p-01` (`0..=44199`, `736.67 s`) | `44,199` (`45.00..=138.33` cycles) | `4,821,331` (`0` off) | `5,893,277` (`1,530,499` active, `0` diff) | `565,754,882` (`0` diff) | `1,412,668` (`1,412,668` exact, `lag = 0`, `0` unrecovered) | `41,201,394` (`0` diff) |

Two timing details uncovered over the `355 s` / `737 s` spans are modelled in
`SamplesSound::tick` (`crates/sf-audio/src/lib.rs`) and `compare_dac_stream`
(`crates/sf-audio/src/check.rs`):
- At `tick 1758401` (`219.800125 s`, frame `13188` of `sf-full-2p-01`), `528`
  Z80 cycles after the `0.2 s` rational clock alignment (`219.800000 s`), the
  poll's `in a,(0)` finishes its operand read `4.78 ns` (`0.0171` Z80 T-states)
  past the `500 ns` slice boundary `219.800147500 s` (`786,784,518.983` Z80
  cycles); because MAME's scheduler runs `audio2` up to `ceil(slice_end *
  CLOCK_HZ) = 786,784,519` inside the slice ending at `219.800147500 s`, the
  poll runs in that slice (`t - 10 ns` before `slice_end`) and does not see the
  68000's `0x84` write in the next slice at `219.800147875 s`.
- Over `736.67 s` (`2.64 * 10^9` Z80 T-states in `sf-full-1p-01`), MAME's Z80
  advances `m_localtime` by `floor(10^18 / 3,579,545) = 279,365,114,840`
  attoseconds per T-state (`52,200` attoseconds/s = `5.22e-14 s/s` below `1 /
  CLOCK_HZ`), shifting `w.t` by `-24.5 ps` at `t = 468.692 s` (`sample
  179,977,861` of `msm2`); accounting for that `5.22e-14` attosecond truncation
  aligns all `565,754,882` `384 kHz` stream samples to `0` differing.

### 2. At the launcher's latency (`IRQ_LATENCY_MID = 92`, `from: 1`, no pre-roll), scene by scene

Consistent with the scope note in `docs/planning/arcade-flow-batch.md` (no
tolerance adopted; differences from the constant interrupt-latency assumption
reported per scene alongside the non-idle command latencies):

#### `sf-full-2p-01` (`0..=21299`, two-player + mode-4 + mode-2 + continue + game over)

| Scene (`frames`) | Non-idle commands & implied latency band (cycles) | MSM5205 DAC channel-tick diffs | MSM5205 `384 kHz` stream diffs (sub-tick phase drift) | YM2151 `55,930 Hz` output diffs |
| --- | --- | --- | --- | --- |
| Power-on, warning, and attract (`0..=2012`) | `9` cmds (`58.3..=128.3`) | `27` frames (`1,792` ticks, `1986..=2012` demo fight) | `27` frames (`169,948` samples, `1986..=2012`) | `0` frames (`0` samples) |
| Coins, start screen, and 2P country select (`2013..=2449`) | `5` cmds (`51.7..=95.0`) | `3` frames (`148` ticks, `2013..=2015` demo tail) | `4` frames (`14,041` samples, `2013..=2016`) | `1` frame (`163` samples, `2085`) |
| Versus match #1: P2 Ken wins 2–0 (`2450..=6990`) | `252` cmds (`48.3..=138.3`) | `0` frames (`0` ticks) | `1,178` frames (`35,757` samples) | `21` frames (`365` samples, `5916..=5935, 5937`) |
| P2 mode-4 intro & P1 challenger join (`6991..=7958`) | `11` cmds (`45.0..=121.7`) | `0` frames (`0` ticks) | `93` frames (`1,780` samples) | `0` frames (`0` samples) |
| Versus match #2: P1 Ryu wins 2–0 (`7959..=12259`) | `249` cmds (`45.0..=138.3`) | `0` frames (`0` ticks) | `1,104` frames (`35,277` samples) | `305` frames (`223,280` samples) |
| Mode-2 Retsu match #1: P1 loses 0–2 (`12260..=16533`) | `69` cmds (`45.0..=138.3`) | `0` frames (`0` ticks) | `1,634` frames (`54,600` samples) | `114` frames (`69,206` samples) |
| Continue #1 (`start1`) & Retsu match #2 (`16534..=20364`) | `95` cmds (`45.0..=138.3`) | `0` frames (`0` ticks) | `1,953` frames (`77,829` samples) | `12` frames (`674` samples, `17898..=17908, 17910`) |
| Continue expiry, game over, and return to attract (`20365..=21299`) | `12` cmds (`45.0..=125.0`) | `0` frames (`0` ticks) | `112` frames (`5,049` samples) | `0` frames (`0` samples) |

#### `sf-full-1p-01` (`0..=44199`, all 10 opponents + 4 bonuses + ending + staff roll + name entry + return to attract)

| Scene (`frames`) | Non-idle commands & implied latency band (cycles) | MSM5205 DAC channel-tick diffs | MSM5205 `384 kHz` stream diffs (sub-tick phase drift) | YM2151 `55,930 Hz` output diffs |
| --- | --- | --- | --- | --- |
| Power-on, warning, and attract (`0..=1513`) | `6` cmds (`78.3..=135.0`) | `0` frames (`0` ticks) | `0` frames (`0` samples) | `0` frames (`0` samples) |
| Coin, start screen, and 1P country select (`1514..=2388`) | `10` cmds (`55.0..=125.0`) | `0` frames (`0` ticks) | `0` frames (`0` samples) | `0` frames (`0` samples) |
| Stage 0 fight 1: Retsu (`2389..=5013`) | `287` cmds (`45.0..=138.3`) | `336` frames (`16,774` ticks) | `1,350` frames (`1,660,232` samples) | `300` frames (`72,854` samples) |
| Stage 0 fight 2: Geki (`5014..=9018`) | `312` cmds (`45.0..=138.3`) | `543` frames (`30,015` ticks) | `2,259` frames (`2,935,894` samples) | `7` frames (`5,364` samples, `5945..=5951`) |
| Japan power/timing bonus & flight to USA (`9019..=10065`) | `8` cmds (`51.7..=135.0`) | `0` frames (`0` ticks) | `498` frames (`20,586` samples) | `0` frames (`0` samples) |
| Stage 1 fight 1: Joe (`10066..=14033`) | `358` cmds (`45.0..=138.3`) | `89` frames (`4,796` ticks) | `1,411` frames (`527,008` samples) | `318` frames (`214,402` samples) |
| Stage 1 fight 2: Mike (`14034..=17483`) | `338` cmds (`45.0..=138.3`) | `89` frames (`4,920` ticks) | `1,591` frames (`540,063` samples) | `170` frames (`98,540` samples) |
| Stage-1 10-second bonus & flight to China (`17484..=19375`) | `22` cmds (`45.0..=135.0`) | `0` frames (`0` ticks) | `631` frames (`19,422` samples) | `7` frames (`5,647` samples, `17559..=17565`) |
| Stage 2 fight 1: Lee (`19376..=23161`) | `473` cmds (`45.0..=138.3`) | `94` frames (`5,639` ticks) | `1,479` frames (`615,214` samples) | `29` frames (`20,591` samples) |
| Stage 2 fight 2: Gen (`23162..=26070`) | `156` cmds (`45.0..=135.0`) | `120` frames (`6,723` ticks) | `1,568` frames (`731,152` samples) | `78` frames (`65,012` samples) |
| Stage-2 power/timing bonus & flight to England (`26071..=26955`) | `50` cmds (`45.0..=135.0`) | `0` frames (`0` ticks) | `618` frames (`38,891` samples) | `0` frames (`0` samples) |
| Stage 3 fight 1: Birdie (`26956..=29775`) | `268` cmds (`45.0..=138.3`) | `105` frames (`6,409` ticks) | `944` frames (`647,349` samples) | `2` frames (`65` samples, `29193..=29194`) |
| Stage 3 fight 2: Eagle (`29776..=32314`) | `241` cmds (`45.0..=138.3`) | `108` frames (`6,234` ticks) | `1,303` frames (`670,900` samples) | `8` frames (`6,588` samples, `31012..=31019`) |
| Stage-3 10-second bonus & flight to Thailand (`32315..=34183`) | `16` cmds (`45.0..=138.3`) | `0` frames (`0` ticks) | `402` frames (`12,549` samples) | `21` frames (`18,236` samples, `34003..=34023`) |
| Stage 4 fight 1: Adon (`34184..=37152`) | `288` cmds (`45.0..=138.3`) | `113` frames (`6,134` ticks) | `1,357` frames (`667,154` samples) | `843` frames (`559,054` samples) |
| Stage 4 fight 2: Sagat (`37153..=39776`) | `256` cmds (`45.0..=138.3`) | `70` frames (`4,515` ticks) | `1,498` frames (`520,306` samples) | `889` frames (`546,510` samples) |
| Ending: 5 country scenes, message, staff roll, `THE END` (`39777..=43616`) | `172` cmds (`45.0..=138.3`) | `0` frames (`0` ticks) | `0` frames (`0` samples) | `1,565` frames (`863,533` samples) |
| Name entry (`AAA`), ranking blink, game over, and return to attract (`43617..=44199`) | `11` cmds (`45.0..=131.7`) | `0` frames (`0` ticks) | `104` frames (`4,045` samples, `43903..=44006`) | `138` frames (`80,493` samples) |

## Sound-test listening kit and human listening review (ticket #126, F54)

`cargo run -p sf-dev --release -- kit-sound-test` renders the native `48,000 Hz`
mono mixes alongside MAME's `audio.wav` for all four service sound-test suites
into `tools/reference/local/renders/` (`sound-test-01..04-native-mix.wav` and
`sound-test-01..04-mame-reference.wav`) and writes the complete 236-row
per-command `onset`, `RMS`, and `correlation` table to
`tools/reference/local/renders/sound-test-measures.tsv`.

### Whole-suite summary

| Suite | Slice | Window (`frames`, `s`) | Commands | Native RMS | MAME RMS | RMS ratio | Correlation (`lag` samples) | Human verdict |
| --- | --- | --- | ---: | ---: | ---: | ---: | --- | --- |
| `sf-sound-test-01` | F22 (`#62`) FM effects & specials | `107..=8699` (`143.22 s`) | `108` | `0.07505` | `0.07325` | `1.0245` | `0.9528` (`+4`) | **Pass** |
| `sf-sound-test-02` | F23 (`#63`) all 21 tunes | `507..=39774` (`654.47 s`) | `42` | `0.07178` | `0.07056` | `1.0174` | `0.9680` (`+4`) | **Pass** |
| `sf-sound-test-03` | F38 (`#78`) tune transitions & stops | `507..=6249` (`95.72 s`) | `14` | `0.08383` | `0.08321` | `1.0074` | `0.9865` (`+4`) | **Pass** |
| `sf-sound-test-04` | F24 (`#64`) all 63 ADPCM samples & stops | `1377..=12759` (`189.72 s`) | `72` | `0.04205` | `0.04190` | `1.0037` | `0.9851` (`+4`) | **Pass** |

### Per-tune metrics and verdict (`sf-sound-test-02` & `sf-sound-test-03`, F23 / F38)

| Tune / transition | Window (`frames`, `s`) | Onset (`native` / `MAME`, ms) | RMS (`native` / `MAME` / ratio) | Correlation (`lag` samples) | Verdict |
| --- | --- | --- | --- | --- | --- |
| `0x28` (Japan / Retsu & Geki, 2 loops) | `507..=6041` (`92.25 s`) | `20.0` / `20.0` | `0.04967` / `0.04955` (`1.0024`) | `0.9938` (`+4`) | **Pass** |
| `0x29` (USA / Joe, loop) | `6302..=8461` (`36.00 s`) | `20.0` / `20.0` | `0.07501` / `0.07471` (`1.0040`) | `0.9900` (`+4`) | **Pass** |
| `0x2a` (USA / Mike, loop) | `8712..=10178` (`24.45 s`) | `20.0` / `20.0` | `0.07932` / `0.07862` (`1.0089`) | `0.9814` (`+4`) | **Pass** |
| `0x2b` (China / Lee, loop) | `10419..=11600` (`19.70 s`) | `20.0` / `20.0` | `0.08959` / `0.08808` (`1.0171`) | `0.9703` (`+4`) | **Pass** |
| `0x2c` (China / Gen, loop) | `11831..=13001` (`19.52 s`) | `130.0` / `135.0` | `0.08864` / `0.08713` (`1.0174`) | `0.9693` (`+4`) | **Pass** |
| `0x2d` (England / Birdie, loop) | `13222..=14189` (`16.13 s`) | `70.0` / `70.0` | `0.11979` / `0.11921` (`1.0049`) | `0.9874` (`+4`) | **Pass** |
| `0x2e` (England / Eagle, loop) | `14400..=16160` (`29.35 s`) | `205.0` / `210.0` | `0.09600` / `0.09543` (`1.0060`) | `0.9838` (`+4`) | **Pass** |
| `0x2f` (Thailand / Adon, loop) | `16361..=17808` (`24.13 s`) | `15.0` / `15.0` | `0.13344` / `0.13225` (`1.0090`) | `0.9823` (`+4`) | **Pass** |
| `0x30` (Thailand / Sagat, loop) | `17999..=19829` (`30.52 s`) | `20.0` / `20.0` | `0.07535` / `0.07499` (`1.0048`) | `0.9903` (`+4`) | **Pass** |
| `0x31` (Versus / bonus, loop) | `20010..=21171` (`19.37 s`) | `25.0` / `25.0` | `0.11475` / `0.10358` (`1.1079`) | `0.8342` (`+4`) | **Pass** |
| `0x32` (Ending / staff roll, end) | `21342..=23759` (`40.30 s`) | `25.0` / `25.0` | `0.07896` / `0.07779` (`1.0150`) | `0.9743` (`+4`) | **Pass** |
| `0x33` (Country / stage intro, end) | `23920..=24495` (`9.60 s`) | `30.0` / `30.0` | `0.06798` / `0.06494` (`1.0469`) | `0.9138` (`+4`) | **Pass** |
| `0x34` (Bonus stage, loop) | `24646..=27056` (`40.18 s`) | `130.0` / `130.0` | `0.06058` / `0.06004` (`1.0090`) | `0.9814` (`+4`) | **Pass** |
| `0x36` (Round start / result fanfare, end) | `27187..=29067` (`31.35 s`) | `35.0` / `35.0` | `0.09507` / `0.09481` (`1.0027`) | `0.9945` (`+4`) | **Pass** |
| `0x37` (Match won / VS fanfare, end) | `29188..=29798` (`10.18 s`) | `25.0` / `25.0` | `0.07970` / `0.07423` (`1.0737`) | `0.8722` (`+4`) | **Pass** |
| `0x38` (Attract / title theme, end) | `29909..=35870` (`99.37 s`) | `50.0` / `50.0` | `0.03346` / `0.03323` (`1.0069`) | `0.9769` (`+4`) | **Pass** |
| `0x39` (Game over fanfare, end) | `35971..=36265` (`4.92 s`) | `25.0` / `25.0` | `0.04679` / `0.04261` (`1.0981`) | `0.8389` (`+4`) | **Pass** |
| `0x3a` (Continue / name entry, loop) | `36356..=37611` (`20.93 s`) | `15.0` / `15.0` | `0.06332` / `0.06289` (`1.0069`) | `0.9840` (`+4`) | **Pass** |
| `0x3b` (Ranking screen, loop) | `37692..=38246` (`9.25 s`) | `10.0` / `10.0` | `0.05945` / `0.05905` (`1.0069`) | `0.9801` (`+4`) | **Pass** |
| `0x3c` (Travel map flight, end) | `38317..=39151` (`13.92 s`) | `20.0` / `20.0` | `0.06173` / `0.05800` (`1.0643`) | `0.8920` (`+4`) | **Pass** |
| `0x3e` (Challenger join fanfare, end) | `39202..=39561` (`6.00 s`) | `10.0` / `10.0` | `0.07418` / `0.07276` (`1.0196`) | `0.9632` (`+4`) | **Pass** |
| `sf-sound-test-03` transitions & stops (`0x28->0x29->0x36->0x32->0x28`, `0x35/0x3d/0x3f`) | `507..=6249` (`95.72 s`, `14` commands) | exact match on all `14` (`0.0..=35.0`) | `0.00248..=0.09920` (`0.9949..=1.0463`) | `0.9250..=0.9985` (`+4`) | **Pass** |

### Per-effect-group metrics and verdict (`sf-sound-test-01`, F22)

| Effect group (`sf-sound-test-01`) | Commands | Onset (`native` vs `MAME`, ms) | RMS ratio range (`native / MAME`) | Correlation range (`lag` samples) | Verdict |
| --- | --- | --- | --- | --- | --- |
| Isolated hit, block, and knockdown effects (`157..=706`) | `0x01..=0x0b` (`11` effects) | exact match on all `11` (`5.0..=20.0 ms`) | `1.0010..=1.0490` | `0.9059..=0.9952` (`+4`) | **Pass** |
| Isolated UI, coin, LFO, and swoosh effects (`707..=1856`) | `0x0c, 0x0d, 0x0f..=0x14, 0x16..=0x19, 0x1b..=0x1d, 0x20..=0x22` (`16` effects + `8` silent entries `0x00, 0x0e, 0x15, 0x1a, 0x1e, 0x1f, 0x23..=0x27`) | exact match on all `24` (`0.0..=10.0 ms`) | `0.9880..=1.1548` (`1.0000` on silent) | `0.7548..=0.9941` (`1.0000` on silent) | **Pass** |
| Effect mask replacement and stop (`2468..=2997`) | `0x02->0x00`, `0x0f->0x10`, `0x0f->0x0c->0x00` (`7` commands) | exact match on all `7` (`0.0..=5.0 ms`) | `0.9967..=1.0453` | `0.9206..=0.9985` (`+4`) | **Pass** |
| All 27 active FM effects over tune `0x33` (`3547..=5787`) | `0x33` + `27` effects + `0x3f` (`29` commands) | exact match on all `29` (`0.0..=30.0 ms`) | `1.0012..=1.1434` | `0.7666..=0.9949` (`+4` / `-8`) | **Pass** |
| All 27 active FM effects over tune `0x28` + stops (`6057..=8507`) | `0x28` + `27` effects + `0x3f, 0x35, 0x3d, 0x3f` (`32` commands) | exact match on all `32` (`0.0..=20.0 ms`) | `0.9983..=1.0879` | `0.8408..=1.0000` (`+4`) | **Pass** |

### Per-sample-bank metrics and verdict (`sf-sound-test-04`, F24)

| ADPCM bank / group (`sf-sound-test-04`) | Commands (`frames`) | Channel(s) | Onset (`native` vs `MAME`, ms) | RMS ratio range (`native / MAME`) | Correlation range (`lag` samples) | Verdict |
| --- | --- | --- | --- | --- | --- | --- |
| Bank `0` impacts, grunts, and announcer voices (`27` commands) | `0x81..=0x9b` (`1377..=4294`) | ch `0` (`26`), ch `1` (`0x8c`) | exact match on all `27` (`5.0..=75.0 ms`) | `1.0012..=1.0161` | `0.9324..=0.9951` (`+4`) | **Pass** |
| Bank `1` long announcer phrases (`3` commands) | `0x9c..=0x9e` (`4295..=5235`) | ch `0` | exact match on all `3` (`15.0..=75.0 ms`) | `1.0014..=1.0043` | `0.9844..=0.9952` (`+4`) | **Pass** |
| Bank `2` character/country voices (`10` commands) | `0x9f..=0xa8` (`5236..=6490`) | ch `1` | exact match on all `10` (`10.0..=120.0 ms`) | `1.0005..=1.0233` | `0.8934..=0.9980` (`+4`) | **Pass** |
| Bank `3` character/country voices (`9` commands) | `0xa9..=0xb1` (`6491..=7774`) | ch `1` | exact match on all `9` (`25.0..=75.0 ms`) | `1.0045..=1.0153` | `0.9308..=0.9803` (`+4`) | **Pass** |
| Bank `4` character/country voices (`8` commands) | `0xb2..=0xb9` (`7775..=8972`) | ch `1` | exact match on all `8` (`45.0..=100.0 ms`) | `1.0023..=1.0118` | `0.9436..=0.9905` (`+4`) | **Pass** |
| Bank `5` boss/bonus voices (`4` commands) | `0xba..=0xbd` (`8973..=9872`) | ch `0` (`3`), ch `1` (`0xbc`) | exact match on all `4` (`0.0..=50.0 ms`) | `1.0009..=1.0119` | `0.9453..=0.9977` (`+4`) | **Pass** |
| Bank `6` crowd/ambient samples (`2` commands) | `0xbe..=0xbf` (`9873..=10806`) | ch `0` | exact match on both (`5.0 ms`) | `1.0021` | `0.9926..=0.9927` (`+4`) | **Pass** |
| Stop (`0x80`), dual-channel overlap (`0x9d + 0xa0`), and exit (`9` commands) | `0x9d, 0x80, 0x9d+0xa0, 0x80, 0xa0, 0x27, 0x3f, 0x80` (`10807..=12759`) | ch `0` & ch `1` | exact match on all `9` (`0.0..=120.0 ms`) | `0.9485..=1.0229` | `0.8996..=1.0000` (`+4` / `0`) | **Pass** |

- **Human listening verdict (ticket `#126` / F54):** **Pass** across all 21 tunes
  and transitions (`F23` / `F38`), all FM effect groups and specials (`F22`), and
  all 7 ADPCM sample banks `0–6` plus stops and overlaps (`F24`). The only audible
  character difference is the same slight crispness vs MAME's band-limiting
  output resampler noted in `#70` (`F30`); `0` defects were found and `0`
  follow-up tickets were required.

## Verification

`crates/sf-audio/tests/both_chips.rs` (local data; skips without it):

| Test | Result |
| --- | --- |
| The native fight against the reference, frame for frame | 2,610 screen hashes (2390–4999) and 2,611 latch values equal |
| Native values at the reference's write times (Round 2) | samples CPU 143,569 writes at their logged times; both MSM5205 streams identical at all 14,991,360 samples; 35,867 YM2151 writes on the exact clock; 1,090,635 output samples identical |
| Native latch at latency 51 (Round 2) | samples CPU 143,569 writes and both DACs over 156,157 channel ticks identical; MSM5205 streams within 0.5% of samples (37,280 differ); YM2151 writes in order, 1,090,635 output samples identical |
| Native latch at latency 45, reported at 92 and 138 (Round 2) | at 45 the samples CPU and DACs identical, the music writes in order within 2 samples, YM2151 output within 0.5% of samples; the music writes in order at 92 and 138 |
| The mix against `audio.wav` at latency 51 | same onset and active span, RMS within 2%, correlation above 0.98 at no more than 8 samples' lag |
| The live player's YM2151 output against the batch runner's | 1,090,635 samples identical on both channels |
| Frames pushed one at a time, as the launcher does, against every write known | 2,088,800 mixed samples, frames 2389..=4999, identical |
| The versus fixture's pre-roll | 68,891 YM2151 writes through frame 2450, all on the exact clock |
| The music CPU's RAM at idle frame ends (ticket #39) | 1,370 of 2,610 frame ends idle; all 1,370 equal below the stack (`0xc000–0xc5cf`) at the reference's write times and at latency 51 |
| The new hook runs repeat identically (ticket #48, F8) | `fight-audio-r1-a/b`, `punch-audio-hook-a/b`, and `win-01-hook-a/b` repeat byte-for-byte on `audio_taps.tsv`, `hook_*.f32`, and sorted `hook_index.tsv` rows |
| Round 1 and the whole fixed fight (`sf-fixed-fight-01`, ticket #48, F8) | at reference write times: R1 189,711 writes, 192,317 DAC ticks, and 18,462,720 stream samples identical; full fight (`2389..=4999`) 333,280 writes, 79,521 exact YM2151 writes, and 2,433,887 YM2151 samples identical; window edges asserted at 52 (YM2151 frame 2453), 53 (joint full-fight match; R1 stream frame 2391), and 55 (samples frame 2619, `tick 349201`) |
| The versus punch run on the native latch (`sf-punch-01`, ticket #48, F8) | 949 screen hashes and 950 latch values equal; at reference write times and across `46–83` (asserted at 51 and 83), all 68,968 writes (`0` off), 126,877 DAC ticks, 12,180,480 384 kHz stream samples, and 885,558 YM2151 output samples are identical; window edges asserted at 45 (YM2151 frame 2853), 84 and 92 (YM2151 frame 3162), 113 (`0` stream diffs), and 114 and 138 (stream frame 2474, `3,559` samples) |
| The winning fight on the native latch (`sf-win-01`, ticket #48, F8) | 3,862 screen hashes and 3,863 latch values equal; at reference write times, all 528,845 writes (`0` off), 515,197 DAC ticks, 49,459,200 384 kHz stream samples, 126,015 exact YM2151 writes, and 3,600,959 YM2151 output samples are identical; non-overlapping constant-latency edges asserted at 45, 78, and 81 (samples frame 2535, `tick 338001`; YM2151 frame 5637 on `45–78` vs frame 3511 on `81–138`) and at 82, 92, and 138 (samples frame 2595, `tick 346001` / DAC `tick 346015`) |
| Fixed-fight listening kit renders and live capture (`fixed_fight_listening_kit_renders_and_live_capture` in `both_chips.rs` & `scripted_fixed_fight_live_capture_equals_batch_mix_and_writes_local_kit` in `sf-dev`, ticket #49, F9) | writes the 8 local WAV renders in `tools/reference/local/renders/`; Round-2 `51` render matches `both-chips.md` (`onset = 180 ms`, `active = 19,225 ms`, RMS ratio `1.0037` within `2%`, correlation `0.9895 >= 0.98` at `4 <= 8` samples lag); Round-2 `92` render measures recorded (`onset = 180 ms`, `rms = 0.1003`, `zcr = 1858 /s`, `centroid = 3445 Hz`, correlation `0.9743` at `4` samples lag); live captures (`round2-launcher-live-92.wav` and `fight-launcher-live-92.wav`) equal the batch mixes at latency `92` sample for sample (`0` differing out of `936,000` and `2,088,800` samples) and byte for byte |
| Power-on warning screen and attract title on the native latch with no pre-roll (`power_on_warning_and_title_on_the_native_latch`, ticket #55, F15) | `1,648` screen hashes (`0..=1647`) and `1,647` latch writes (`1..=1647`, with frame 1 at `186,957` cycles = `0.023369625 s`) match `attract-01-hook-a/b`; with no pre-roll (`from: 1`), **every device is 100% identical (`0` differing samples, `first_diff_frame = None`) both at reference write times and at constant latencies `45`, `51`, `92`, and `138`** (`73,246` samples-CPU writes with `0` off, `219,837` DAC ticks, `21,104,642` 384 kHz stream samples with `0` differing, `42,759` music-CPU writes in order within 2 samples, `1,536,211` YM2151 output samples with `0` differing, `1,211 / 1,211` idle frame-end music RAM dumps `0..=1647` equal below `0xc5cf`, and frame-by-frame streaming from power-on equal to the batch mix on all `1,317,600` samples over `0..=1646`) |
| Complete-game 2P and 1P sessions from power-on on the native latch (`full_game_2p_and_1p_on_the_native_latch`, ticket #143, F71) | `full-2p-01-hook-a/b` (`0..=21299`) and `full-1p-01-hook-a/b` (`0..=44199`) repeat byte-for-byte; at reference write times (`from: 1`), **every device is 100% identical over both complete games** (`1,762,231` and `4,821,331` samples-CPU writes with `0` off; `2,839,837` and `5,893,277` MSM5205 DAC ticks with `0` diff; `272,624,642` and `565,754,882` `384 kHz` stream samples with `0` diff; `639,631` and `1,412,668` exact YM2151 writes with `lag = 0` and `0` unrecovered; `19,854,032` and `41,201,394` YM2151 output samples with `0` diff); at launcher latency `92`, per-scene device differences and non-idle command latencies are measured across all 8 scenes of `sf-full-2p-01` and all 18 scenes of `sf-full-1p-01` |
| Sound-test listening kit and human listening review (`sound_test_listening_kit_renders_wavs_and_per_command_measures` in `sf-dev`, ticket #126, F54) | writes `sound-test-01..04-native-mix.wav`, `sound-test-01..04-mame-reference.wav`, and the 236-row `sound-test-measures.tsv` in `tools/reference/local/renders/`; whole-window correlation `0.9528..=0.9865` at `+4` samples lag; human listening verdict **Pass** across all 21 tunes and transitions (`F23` / `F38`), all FM effect groups (`F22`), and all 7 ADPCM sample banks `0–6` (`F24`) with `0` follow-up tickets |

## Not covered

- **The launcher's latency is the band's middle (`92`).** Per the scope note
  above and open decision 6 (`docs/planning/arcade-flow-batch.md`), no single
  constant interrupt latency in `45–138` fits all recorded fights (`46–83`
  on `sf-punch-01`, `53–54` on `sf-fixed-fight-01` full fight / `49–54` on
  round 2, and no single value on `sf-win-01`, `sf-full-2p-01`, or
  `sf-full-1p-01`). The launcher stays at `92` unless the user decides otherwise
  (open decision 6), and no tolerance is adopted.
- **The samples CPU's timing within the tick** follows the reference across
  every fight and both complete games with the reference's write times (and on
  `sf-punch-01` at constant latencies `45–113`); under a constant latency it
  drifts by up to `41 µs` when an idle or non-idle poll straddles a slice end.
- **MAME's output resampler** is not modelled. The mix is measured, not
  compared sample for sample, and byte-identical audio is out of scope (confirmed
  by the human listening review in ticket `#70` / F30 above).
- **The f64 tie at latency 54 on `sf-fixed-fight-01`** is a property of the
  model's floating-point arithmetic at the slice end, not of the reference.
- **A reload's pre-roll** (in the fixture modes `cpu` and `versus`; `power-on`
  mode needs no pre-roll) runs the sound from power-on to the fixture again
  before play resumes (about a second on the recording host, release build).
