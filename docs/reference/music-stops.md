# Music special commands, stops and tune ends

This document records the boundary established by
issue #33: the music
CPU's special commands (the dispatch at 0x02a5 and its seven handlers,
0x02b5–0x0423), the stops among them, and the sequencer's opcode 0xf (the
end of a tune, 0x09e7), translated into `crates/sf-audio/src/music.rs`
with the T-state timing of the music-phrase slice (music-phrase.md), and
verified against the pinned MAME reference over four windows in which a
stop is followed by a new tune, a special command runs over a live tune,
and a tune ends by itself. Every YM2151 write of each window matches in
order within one output sample, the chip output is sample-identical over
the hooked streams, and the music CPU's RAM equals the reference's
checkpoint dumps. The windows also exposed a timer-flag race in the chip
model, fixed here (`opm.rs`).

## What the original does

The command intake (0x019d) looks a command up in the table at 0x1011
(two bytes per command); an entry with 0xff in its high byte is a special
command and its low byte selects one of seven handlers through
`ld a,e; rst 0x30` at 0x02a5:

| Entry | Handler | What it does | Commands |
| --- | --- | --- | --- |
| 0 | 0x02b5 | stop everything: 0x03a4, then both effect slots | 0x00 |
| 1 | 0x02b8 | stop both effect slots (slot B then A) | 0x0c, 0x0e, 0x15, 0x1a, 0x1d, 0x1e, 0x1f, 0x23–0x27 |
| 2 / 3 | 0x02bb / 0x02f3 | stop one effect slot (0xc500 / 0xc580): its priority byte (0xc020 / 0xc021) cleared, the noise register (0x0f) cleared if the slot used channel 7, the music re-established on the channels it owned (0x032b), the slot cleared | — (called by 1 and 0, and by an effect ending by itself) |
| 4 | 0x03a4 | **stop the music**: the saved slot discarded, then 0x03a7 | 0x35, 0x3d, **0x3f** |
| 5 | 0x03a7 | pop the saved tune into the current slot: 0xc011 = 0xc012, the saved channels copied over the current ones, the saved slot discarded, then every channel no effect owns muted and keyed off and — if it has a stream — its patch reloaded in full (0x08e4) and its current note keyed on | — (0x03a4's tail; opcode 0xf's target) |
| 6 | 0x0412 | discard the saved tune: 0xc012 = 0, 0xc300–0xc47f cleared | — |

**The channel hand-back (0x032b)**, which the effect-slot stops call with
E = ~mask | the other slot's mask (so an empty slot hands back every
channel the other slot does not own): per channel whose E bit is clear, a
channel without a stream is muted and keyed off (the mute skipped while
an effect ends by itself, 0xc024), one with a stream is muted, keyed off,
its patch registers rewritten from patch + 5 (0x0924: no LFO block) and
its current note keyed on again from the attack; then the four LFO
registers (0x1b, 0x18, 0x19, 0x19) are rewritten from 0xc015–0xc018. So
a "stop effects" command arriving while a tune plays and no effect does
retriggers every sounding note — twice for command 0x27 (both slots).

**The end of a tune (opcode 0xf, 0x09e7)** pops three words (the returns
into the event reader and the channel loop, the loop's counter) and
jumps to 0x03a7 — the saved tune, if any, is popped; with none, every
free channel is muted and keyed off and 0xc011 becomes 0 — or, for the
silently advancing saved tune (0xc010 set), clears 0xc010 and jumps to
0x0412. The rest of that tick's channel loop is abandoned; 0x03a7's `ret`
lands at 0x0440 (the 0xc100 channels were entered by `call 0x0453`) or
ends the tick (the saved channels fall through from 0x044f). No tune
header in this ROM has bit 6 set, so the saved-tune branch is dead with
this data.

The translation (`special`, `stop_effect`, `reestablish`, `stop_music`,
`pop_saved`, `discard_saved`, `load_patch_registers`, the `Flow` unwinding
through `opcode` → `read_events` → `count_down` → `channel_tick` →
`tick_channels` → `music_tick`) charges the standard Z80 T-states of every
instruction (the dispatch 125; `ldir` 21 per byte and 16 for the last:
the 383-byte copies 8038, the clears 8105 with their `ret`; the stop of a
live tune with all channels free ≈ 33,350 T-states, 9.3 ms — three timer-A
periods, so its ticks coalesce as the module doc describes). Opcode 0xf's
unwinding assumes the event read is not nested in a `call` — the
double-speed extra count-down (0x0489) or a tie's nested read (0x04f7);
there the original's stack differs, and the translation reports 0x09e7
as untranslated instead of guessing (`MusicCpu::nesting`).

## The timer-flag race (chip model)

Window D's phase sweep gave the right writes at six phases but none on
the exact clock, and windows C and D diverged after a late sequencer
tick. The trace showed why: the handler wrote register 0x14 (resetting
timer A's flag) eight clocks before timer A's exact deadline, and the
chip model, which told the core of expiries at output-sample boundaries
(~18 µs), had already set the flag within that sample — the reset wiped
it, the tick was lost, and the tune ran a tick behind. MAME's timers fire
at the exact clock, so there the reset lands first and the new flag
stands. `Opm` now keeps the timer flags and the IRQ line itself at exact
clocks: a flag is set at its deadline and cleared by a reset write only
if the deadline had passed by the write's time; the core still learns of
the expiry at the sample boundary (it re-arms the timer from the deadline,
so the periods stay exact), but its own flag bits are not used. The
music-phrase tests pass unchanged on the new model.

## The windows

All on the 1P boot path of sf-music-01 (the same inputs) except C:

| Window | Scenario, frames (fixture = end of the first) | What happens | Idle phase |
| --- | --- | --- | --- |
| A | sf-music-02, 1864..=2109 | tune 0x34 live; **0x3f at 1866** (0x03a4): 8 × (four TL = 0x7f, key-off) over 9.3 ms; 92 frames of timer re-arms only; **tune 0x33 at 1959** through the 0x01e5 path (an empty current tune), 1,024 writes to 2117; the samples commands 0x8f (1868) and 0x97 (2117) through the intake's bit-7 path | 8 |
| B | sf-music-02, 2274..=2357 | tune 0x33 live; **0x27 at 2275** (0x02b8): slot B then A handed back, every channel's patch rewritten and note re-keyed, twice, 489 writes over 50 ms; **the 0x3f at 2277 is never read** — the handler's last write is at 37.969997 s, after the 68000 returned the latch to idle at 37.966765 s; **0x28 at 2280** through the equal-priority path (the music-phrase slice's tune) | 12 |
| C | sf-win-01, 5059..=5583 | **tune 0x37 at 5060** (the match win) to its sustained final chord (5378–5379, then nothing but re-arms for 49 frames); **0x3f at 5429 on the chord**; **0x3f at 5430 rejected** (the intake ignores a latch equal to the last command taken); **tune 0x33 at 5431** | 2 |
| D | sf-music-03, 7185..=7385 | **tune 0x39 at 7186** (the attract ranking screen's tune from `0x4268c` -> `0x426b2`, reached after the game-over screen returned to the attract loop at 6329 and scrolled through the title over 6329..=7184; issue #56) **ends by itself at 7301**: opcode 0xf → 0x03a7 with no saved tune, 8 × (mute, key-off) with no stop on the latch; 92 frames idle | 7 |

The scenarios (`tools/reference/scenarios/sf_music_02.json`, `sf_music_03.json`)
checkpoint every frame of their windows (the music CPU's RAM among the
dumps); the audio runs are `run_audio_probe.py --hook-window 1860 2362`
(two runs, identical taps, hook stream and checkpoints) and
`--hook-window 7185 7395` (two runs, identical); window C uses the
sf-win-01 capture's dumps and taps (win-01-audio) with a hook run over
5059–5591 (its taps equal win-01-audio's). The exploratory 9,000-frame
run that found the natural tune end (the fixed fight continued through
the loss: the continue tune 0x3c stopped by 0x3f at 6101, the countdown
effects 0x1b, the game-over screen at 6240–6328, the return to the attract
loop at 6329, the attract ranking tune 0x39 at 7186 and its end, and the
attract demo's 0x28 at 7396) is local-only.

Each fixture's idle-loop phase comes from the `music_phase` example, as
the music-phrase slice derived its own: the phase that puts the native
writes on the reference's exact chip clock (A: all 6,815; B: all 2,773;
C: all 14,709; D: all 5,815); every other phase puts none or a few hundred
there. (C and D first had five writes 12 T-states late each, one timer
re-arm handler after an NMI: the NMI taken behind `ei` lets MAME accept the
pending timer interrupt at `jr`'s start; the idle loop does the same since
ticket #36, music-power-on.md.)

**A second timing constant settled here.** With the first phase fits
(A 7, B 11, C 1, D 6) 4–15% of the writes stayed off the exact clock:
whole handlers one idle-loop lap (16 T-states) late, and in window D one
of them carried a key-on, so the output differed from the 28th frame on.
Over the reference's 46,146 promptly served timer-A handlers, the
acceptance lap end sat in [−1, 14] clocks around the model's deadlines —
a 16-wide band starting one clock early. The timer chain was right (the
re-arm writes sit on an exact 10,752-clock grid); the model's cost from
acceptance to the first write was one T-state long: the 7-T-state
indirect accesses (`ld a,(bc)`, `ld (bc),a`) perform their bus cycle in
their last three T-states, 4 after the instruction start, where the model
had 5 (`IND_ACCESS`). A constant error there is absorbed by the fitted
phase — except in the acceptance decision, which is why the phrase slice
could be sample-identical with 1,828 of 2,283 writes on the clock. With
4, the sf-music-01 fixture's phase is 12 and all its writes are exact. A fixture must be a frame
end at which the music CPU is idle: the end of 1865 (the first choice for
A) fell 96 T-states into a timer handler's prologue in the reference, so
the native run — which starts in the idle loop — accepted that interrupt
late and its first three writes lagged two samples; 1864 is idle. The
frames whose RAM is compared are chosen the same way (no write within
1.5 ms before, the timer grids' last expiries 1.2 / 0.3 ms before, the
dump's channel index 0xc014 = 7): a dump taken inside a handler is a
state the native run never ends in.

## Verification

`crates/sf-audio/tests/music_stops.rs` (local data; skips without it):

| Check | A | B | C | D |
| --- | --- | --- | --- | --- |
| Reference writes in the window | 6,815 (5,788 timer re-arms, 84 key-ons, 106 key-offs) | 2,773 (1,932 re-arms, 27 key-ons, 40 key-offs) | 14,709 (12,273 re-arms, 220 key-ons, 278 key-offs) | 5,815 (4,701 re-arms, 89 key-ons, 109 key-offs) |
| Native writes: count, order, data | equal | equal | equal | equal |
| Every write within one output sample | yes | yes | yes | yes |
| Untranslated branches reached | none | none | none | none |
| Output samples compared, identical | 228,381 | 77,369 | 488,455 | 186,433 |
| RAM (0xc000–0xc5cf) equal to the dumps at | 1868 (stopped), 1985 (0x33 loaded), 2109 | 2282 (0x28 loaded), 2357 | 5224 (playing), 5428 (the chord held), 5435 (stopped, then 0x33 loaded), 5583 | 7249 (playing), 7309 (ended: 0xc011 = 0, channels unloaded), 7385 |

The window B test also asserts the 0x27 handler's last write comes after
37.966765 s (the latch's return to idle), the mechanism by which the 0x3f
is missed. The two reference runs of each new scenario repeat exactly
(taps, hook stream, checkpoints). The music-phrase suite passes with its
fixture's phase re-derived (12), the hit-sound, fight-samples and
latch-timing suites unchanged.

Two harness details were settled on the way: the hooked stream's first
sample index is floor(T₁ × rate) + 1 − n₁ from the hook's first update
(time T₁, n₁ samples), as MAME's stream updates deliver the samples with
index ≤ floor(T × rate) — the earlier `round(start × rate) + 1` agreed
only when the fraction of T₁ × rate was small, and a hook recorded
without a WAV sink (`--no-wav`) carries no sound, since nothing pulls the
device streams.

## Tune transitions, priorities, the saved tune and stops (`sf-sound-test-03`, ticket #78)

Issue #78 (F38, label SM)
verifies the tune-priority intake (`0x01c4–0x0227`), the saved-tune slot
(`0xc012`, `0xc300–0xc47f`), `pop_saved` (`0x03a7`), `discard_saved`
(`0x0412`), and the music stops (`0x35`, `0x3d`, `0x3f` at `0x03a4`) from
power-on over `tools/reference/scenarios/sf_sound_test_03.json` (6,250 frames,
114 checkpoints, hook window `506..=6249` on `ymsnd`):

- **Priorities in `sf-02.7k`:** tunes `0x32` and `0x36` have header `0x0f`
  (priority 15, bit 6 = 0) and end naturally via opcode `0x0f`; the remaining
  19 tunes have header `0x08` (priority 8, bit 6 = 0).
- **Equal priority (`0x01cf: jr z, 0x01f8`):** `0x29@597` replaces live
  `0x28@507` in `0xc100–0xc27f` without copying to `0xc300` (`0xc012` remains
  `0x00`), and `0x29@677` repeats the same selection at equal priority.
- **Higher priority (`0x01d1: jr c, 0x01e5`) and saved-tune pop (`0x03a7`):**
  `0x36@887` (priority 15) saves live `0x29` (`0xc100–0xc27e` -> `0xc300–0xc47e`,
  `0xc012 = 0x08`) and plays through its opcode `0x0f` end at frame `2558`,
  where `0x03a7` pops `0x29` back into `0xc100`, reloads its patches (`0x08e4`),
  re-keys its notes (`0x0551`), and calls `0x0412` to clear `0xc012` and
  `0xc300–0xc47f`.
- **Lower priority into `0xc300` (`0x01d3–0x01e3`):** while `0x32@2727`
  (priority 15, having saved `0x29`) plays, `0x28@2907` (priority 8) takes the
  `0x01d3` `saved == c` branch (`0x01d9: jr z, 0x01dc`) and loads `0x28` into
  `0xc300–0xc47f` while `0x32` continues undisturbed in `0xc100`; when `0x32`
  reaches opcode `0x0f` at frame `4895`, `0x03a7` pops `0x28` into `0xc100`.
  While `0x36@5427` plays with an empty saved slot (`0xc012 = 0x00`),
  `0x33@5537` takes the `0x01d3` `saved < c` branch (`0x01db: ret nc` not taken)
  and loads `0x33` into `0xc300–0xc47f`.
- **Stops `0x35`, `0x3d`, and `0x3f` over a saved tune (`0x03a4`):**
  `0x35@5337` (over saved `0x28` and live `0x36`), `0x3d@5717` (over saved
  `0x33` and live `0x36`), and `0x3f@6117` (over saved `0x3a` and live `0x36`)
  each discard `0xc300` via `0x0412` and stop `0xc100` via `0x03a7`.
- **Repeated commands ignored (`0x01a4: cp c; ret z`):** every command latch
  window exercises `0x01a4` across subsequent Timer B polls while `0xc001`
  holds the command byte (68 ignored polls over the run), alongside the
  service screen's paired-release toggle and the repeated `0x29@677` selection.
- **Verification (`crates/sf-audio/tests/sound_test_03.rs`):** `sound-test-03-audio-a`
  and `sound-test-03-audio-b` match byte-for-byte; from power-on, all **169,897
  YM2151 writes** match on the exact chip clock, all **5,354,738 hooked output
  samples** are identical, and all **54 idle checkpoints** (including idle
  checkpoints before and after every command and natural-end transition, with 0
  transition gaps) match `0xc000–0xc5cf` byte-for-byte. Swapping `cur < c` to
  `cur > c` (`12,770 / 156,257` exact writes, `4,937,585` sample mismatches,
  `82` idle RAM mismatches) or dropping the `0xc300` copy (`69,058 / 169,349`
  exact writes, `279,857` sample mismatches, `55` idle RAM mismatches) breaks
  all three checks (`C-NEG`).

## Not covered

- **Opcode 0xf inside a nested event read** (double speed's extra
  count-down, a tie's nested read): reported as untranslated; no tune in
  the windows ends there.
- **The silently advancing saved tune's end** (0xc010 set at 0x09e7):
  translated, dead with this ROM's data (no tune header in `sf-02.7k` has
  bit 6 set).
- **Special command 0x00 (stop everything) and the effect-slot stops
  with a non-empty slot**: translated, not reached here (no window has an
  effect running at a stop). The hand-back's 0xc024 branch and the slot
  stop with channel 7 in the mask are reached by an effect's self-end in
  ticket #34's windows (fm-effects.md).
- **Commands beyond the dispatch table** (an entry past 6): the original
  jumps through tune data; reported as untranslated.
- **Effects over the windows**: window A ends before the 0x0b effect at
  2118; the others contain no effect start.
