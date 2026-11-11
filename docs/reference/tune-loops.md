# Every tune through its loop or natural end

This document records the boundary established by issue #63 (F23, label SM): all 21 tunes played alone through a whole-tune loop or opcode 0xf, including one complete additional pass of tune 0x28 after its first loop. It extends the special-command and tune-end translation in music-stops.md; the implementation is in `crates/sf-audio/src/music.rs`, its timed interrupt runner in `crates/sf-audio/src/check.rs`, and the acceptance tests in `crates/sf-audio/tests/sound_test_02.rs`.

## Reference scenario

`tools/reference/scenarios/sf_sound_test_02.json` starts in Service Mode from power-on and uses the F22 sound-test navigation. Each tune is followed by command 0x3f before selecting the next one. This isolation is required: without the stops, tunes can remain in the saved slot and tunes 0x32 and 0x36 do not naturally end.

The run has 39,775 frames (last frame 39,774), 42 latch commands, 126 inputs, and 112 full checkpoints. The first command is 0x28 at frame 507. The YM2151 hook covers frames 506–39,774. Checkpoints precede every command and surround every observed boundary at `F-1`, `F`, and `F+1`. Two local captures, `sound-test-02-audio-a` and `sound-test-02-audio-b`, have byte-identical taps, hook index and stream, WAV, and checkpoints; both use DSW1 = 0x5fff and DSW2 = 0xffff.

The reference RAM dumps establish these boundaries. A loop frame is the latest first recurrence of a stream pointer among the active channels; the test also pins that channel and its recurring pointer. An end frame is where 0xc011 becomes zero and all eight current stream pointers are cleared by opcode 0xf.

| Tune | Boundary | Reference frame |
| --- | --- | ---: |
| 28 | first loop | 3,113 |
| 28 | next loop, one complete pass later | 5,692 |
| 29 | loop | 8,123 |
| 2a | loop | 9,849 |
| 2b | loop | 11,281 |
| 2c | loop | 12,692 |
| 2d | loop | 13,891 |
| 2e | loop | 15,871 |
| 2f | loop | 17,529 |
| 30 | loop | 19,553 |
| 31 | loop | 20,912 |
| 32 | end | 23,510 |
| 33 | end | 24,257 |
| 34 | loop | 26,828 |
| 36 | end | 28,858 |
| 37 | end | 29,599 |
| 38 | end | 35,682 |
| 39 | end | 36,086 |
| 3a | loop | 37,442 |
| 3b | loop | 38,087 |
| 3c | end | 39,002 |
| 3e | end | 39,432 |

For tune 0x28, `5,692 - 3,113 = 2,713 - 134`: the second recurrence is one complete pass after the first, relative to the initial latch delay. Thirteen distinct tunes loop and eight end.

## Interrupt return timing exposed by tune 0x38

Five writes in one timer-A re-arm at frame 33,548 were initially 14 T-states late. A local read tap on the original `audiocpu` around 559.1335 s established the instruction order: the timer-B half of the preceding handler reaches `ei` at 0x0058; an NMI is accepted immediately afterward; after the NMI's `retn`, the already pending timer interrupt is accepted before the outer handler's `reti` at 0x0059.

The timed runner now models that ordering. It charges `ei` separately, records an outer `reti` deferred by this interrupt sequence, and executes the postponed return when the nested interrupt chain unwinds. Dropping that outer return made following handlers two T-states late; retaining it makes this boundary and the rest of the run exact. No scenario-specific clock adjustment is involved.

## Verification

`cargo test --release -p sf-audio --test sound_test_02 -- --nocapture` uses local reference captures and ROMs, and skips on a clean checkout. Its seven checks establish:

- **C-REF:** both captures are repeatable and all command, loop, end, checkpoint, and hook-window requirements above hold.
- **C-NAT:** from power-on, all **1,089,126 YM2151 writes** match in count, order, register, data, and exact chip clock. All **36,605,067 hooked output samples** are identical. No dynamic run reports untranslated 0x0000 (opcode 0xa) or nested-end 0x09e7.
- **RAM:** **66 idle checkpoints are identical** over 0xc000–0xc5cf; **46 checkpoints are not idle** and are not compared. The only ending tune without an idle checkpoint after its end and before the next command is **0x32 at frame 23,510**, listed for F72 rather than treated as a comparison.
- **Streams:** a static scan of every tune reaches no opcode 0xa. Every ending tune contains opcode 0xf; every looping tune has a backward jump. The dynamic output comparison additionally rules out a reached untranslated nested end.
- **Full pass:** both 0x28 pointer recurrences and their one-pass separation are asserted.
- **C-NEG:** moving only the initial 28@507 latch by one Timer A sequencer tick, `168 * 64 / 3,579,545` seconds, leaves **86,006 / 86,009** writes exact before the first-loop comparison window; the first mismatch is at frame 507, so the check cannot pass vacuously.
- **Focused timing regression:** the frame-33,548 timer-A re-arm retains the full preceding latch history and matches every reference write clock in its two-frame window.

The earlier `sound_test` acceptance suite also remains green: 255,734 writes on their exact clocks and 8,095,308 identical output samples, with its previous RAM and required-path results unchanged.
