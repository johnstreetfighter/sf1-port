# The samples CPU over the whole fixed fight

This document records the boundary established by
issue #32: the translated
samples-CPU program (hit-sound.md) run over the whole fixed fight —
power-on through the end of round 2, 5000 frames, 83 s — reproduces every
port write of the pinned MAME reference in order, data and time, and
both MSM5205 outputs match the reference's device streams over the
round-2 KO. To get there the translation gained the Z80's T-state timing
and a model of how the pinned MAME orders the 68000's latch writes against
the Z80's polls; both are verified instruction by instruction against a
MAME trace. Code: `crates/sf-audio/src/samples.rs`, `lib.rs`, `check.rs`.

## The reference runs

`python tools/reference/run_audio_probe.py tools/reference/scenarios/sf_fixed_fight_01.json
--out tools/reference/local/captures/fight-audio-a --hook-window 4740 4840`
(and `-b`): the fixed fight with the sound taps (every latch write, every
samples-CPU port write, every YM2151 write with its machine time), MAME's
mixed WAV, and the MSM5205 / YM2151 device output streams over frames
4740–4840 (the round-2 KO at 4773). The two runs are byte-identical in
the tap log, the three device streams and the WAV; the hook index lists
each update's devices in Lua's table order, which varies, so it is
compared as a set of rows.

**What the fight plays** (47 sample commands on the latch):

| Commands | Channel | Bank | Count |
| --- | --- | --- | --- |
| 0x81, 0x84, 0x87, 0x8f, 0x94, 0x97, 0x98 | 0 | 0 | 18 |
| 0xa1, 0xa2, 0xa6, 0xa7 | 1 | 2 | 27 |
| 0x80 (stop) | — | — | 2 |

458,020 samples-CPU port writes: 61,515 channel-0 and 79,946 channel-1
nibbles, 1,206 frames with channel 1 playing, 584 frames with both chips
playing at once (overlapping playback), bank 2 selected on every channel-1
service. The punch-hit slice (ticket #10) had exercised none of channel 1,
the banked tables or the overlap.

## The samples CPU's timing (new)

The program's paths are the ticket-#10 translation; each now charges the
T-states of the original instructions it stands for, at the CPU's clock
(sf.cpp: 3.579545 MHz), with the standard Z80 costs and the branch taken
or not — the handler prologue (0x0038: 46 T-states to a channel-0
continuation, 51 to channel 1), the poll's branches by latch value (bit 7
clear: 31 from the `in`; 0xff: 61; a repeated command: 88; a new command
into the start path), the start path's range chain and carries, the
play, release and stop paths. Between interrupts the program sits in
`ei; jr $-1` (0x0028): a 16-T-state lap.

**Interrupt acceptance**, as the pinned MAME does it. The 8 kHz request
k arrives at k / 8000 s. MAME's Z80 core checks interrupts only at an
instruction start, after `ei` only from the following instruction on, and
it hands the CPU whole T-states up to the request (stopping inside an
instruction when the budget runs out) — so the request is taken at the
first end of a `jr` lap at or after floor(r), where r is the request in
T-states; the IM 1 response costs 13 T-states before 0x0038 runs. A
request that arrives during a handler (interrupts disabled) is taken one
lap after the handler returns to the loop.

**I/O placement.** The I/O cycle is the last 4 T-states of `in a,(n)` /
`out (n),a`, and the access happens at its start: a write is logged, and
the latch is read, 4 T-states before the instruction's end.

**What the latch read sees.** The 68000 writes the latch at its own
times; a write earlier than the Z80's read is visible, as expected. But
the pinned MAME schedules CPUs in slices of its scheduling quantum — with
no quantum set in sf.cpp, the executing devices' minimum: the 68000's four
cycles at 8 MHz, 500 ns, counted from the last timer expiry (the 8 kHz
tick, or a frame boundary inside it) — and in each slice the 68000 runs
before the Z80s. So a latch write later in the same 500-ns slice as the
Z80's read is already visible to it, and one in a later slice is not. The
fight has both cases (42.250147 s: a write 0.1 T-state after the read,
seen; 31.117428 s: a write 33 µs after the read, not seen); the
`SamplesSound::tick` read models the slice, and because a Z80 step starting
at `t` finishes one Z80 T-state later (`t + 1 / CLOCK_HZ`), a 68000 write on
the same Z80 T-state is also visible when the slice ends inside that T-state
(tick 538801 of sf-win-01 at 67.350146 s: the poll starts 43 ns before the
slice end and the 68000 writes `0xff` 125 ns after it, on the same Z80
clock), while a poll finishing 4.78 ns past a slice boundary at the 0.2 s
rational alignment (`528 / 3,579,545 = 147.504781 us`, tick 1758401 of
sf-full-2p-01 at 219.800147 s) still executes inside the slice ending at
`ceil(slice_end * CLOCK_HZ)`.

**Evidence.** A MAME trace of the samples CPU (every code fetch and every
latch read with its machine time, taps on the `audio2` program and I/O
spaces; 0–5.7 s and 39.8–42.6 s of the fixed fight, local-only) showed:
the loop's fetches 4, 4 and 8 T-states apart; the handler's first fetch
always 21 T-states after the `jr`'s operand fetch (13 after the lap's
end); the handler start 12–28 T-states after the request, uniformly; the
latch read 7 T-states into the `in`; and every path's fetch-to-fetch
costs equal to the model's (idle 28 / 159 / 169 / 189 / 199 / 216 / 218 /
228, play 272 / 281 / 282 / 291, release 95 / 100, the start path 582).
The two latch races above are in the trace with their read values. With
the model, every one of the 458,020 port writes of the fight — and the
198,753 of the punch-hit run — lands at the reference's logged time to
the T-state (`check::compare_write_times`).

Two attribution details of the reference log were settled on the way: a
bank select (port 2) precedes the chip write of its service and is
attributed to that write's tick when the service spilled past the tick
(the start path can), and the DAC comparison's last sample must lie
inside the hooked data (the window ends three ticks before the hook's
end).

The former constant `POLL_DELAY` (20 µs, hit-sound.md) is gone: the read
time is the modelled `in`. The 68000-side write time of a native run's
latch, a constant (`LATCH_DELAY`) when this was written, is the vblank
handler's accounted cycles since ticket #35 (latch-timing.md).

## Verification

`crates/sf-audio/tests/fight_samples.rs` (local data; skips without it):

| Check | Result |
| --- | --- |
| The two reference runs | identical taps, device streams, WAV; the same hook-index rows |
| Every samples-CPU port write over the whole run, from power-on on the reference's latch writes | MATCH, 458,020 writes (tick, port, data) |
| Every write's modelled time against its logged time | MATCH, largest difference 0 T-states |
| Both MSM5205 DAC outputs against the hooked device streams over 4740–4840, every channel tick | MATCH |
| Channel 1 plays (79,946 nibbles), bank 2 is selected, both chips play at once (584 frames) | yes |

The punch-hit tests (hit_sound.rs) pass unchanged on the new timing.

## Not covered

- **The native engine's latch over the fight:** these checks drive the
  samples CPU from the reference's latch writes; the native engine's
  per-frame latch drives it in the hit-sound slice and, over the fixed
  fight from the round-1 fixture, in both-chips.md (ticket #37), with the
  68000-side write time from #35 and its latency limit.
- **Scheduler boundaries other than the tick and the frame:** a YM2151
  timer expiring inside a tick re-anchors MAME's 500-ns slices; no latch
  race in this fight fell near one, so it is not modelled. A race lands
  inside the same slice only within 500 ns of the read.
- **Banks other than 0 and 2, the command ranges above 0x28, and the
  stop command's channel paths:** translated, not reached here.
