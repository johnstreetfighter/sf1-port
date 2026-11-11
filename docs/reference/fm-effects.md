# FM sound effects from the effects player

This document records the boundary established by
issue #34: the music
CPU's sound-effect start (0x0240–0x02a4) and effects player (0x0a9b–0x0faf)
translated into `crates/sf-audio/src/music.rs` with the T-state timing of
the music slices (music-phrase.md, music-stops.md), and verified on the
hit-spark effect 0x01 played from the **native** engine's hit event: the
native run of sf-punch-01 supplies the latch writes (the impact sample
0x84 in frame 2717, the effect 0x01 in 2718), and the music CPU, run from
the reference's RAM dump at the end of frame 2716, reproduces every YM2151
write of the reference in order on the exact chip clock, with the hooked
chip output sample-identical and its RAM equal to the dumps after the
effect. Two more windows check other effects on the writes. The player's
size (about 325 new instructions) fit one pass; the ticket's split was not
needed.

## What the original does

**The effect start (0x0240)** is where the command intake goes when a
command's header byte has bit 7 set. Header bit 6 selects the slot (0xc500
with its priority byte 0xc020; 0xc580 / 0xc021 — never with this ROM's
data: every effect header is 0x81 or 0x83). The header's low nibble is the
priority: equal to the slot's byte starts the effect, higher is recorded
first, lower is ignored (`ret nc` — so a priority-3 effect, commands 0x02
and 0x03, blocks every priority-1 effect until it ends). The slot is zeroed
(0x50 bytes) and loaded: +0 = the channel mask (header + 1; bit 7 − ch =
channel ch, bit 0 = channel 7, the noise channel), +1/+2 = the stream
pointer (header + 2, big-endian). If the mask has channel 7, register 0x0f
(noise) is cleared. Then, if the mask differs from the slot's previous one,
the music is re-established on every channel whose ownership changed
(0x032b with E = ~(new ^ old); 0xc023 is read as it is, not written) — the
channels the effect takes are muted and keyed off (tune 0x28 has no stream
on 6 or 7), and channels a replaced effect frees get their music back.

**The effects player (0x0a9b)** runs in every timer-A handler before the
sequencer tick: slot A (by `call`), then slot B (by fall-through). A slot
with a non-zero mask ticks: a running countdown (+3/+4) is decremented and
every channel of the mask (7 down to 0, 0xc022; the mask rotated right per
channel, eight rotations restoring it; 8-byte records from slot + 0x10,
channel 7 first) slides (0x0afe: the signed step +2 added to note.fraction
+0.+1; a changed integer part re-keys the note without a key-off; then
KF = the fraction), after which the noise frequency advances (+6 += +7,
register 0x0f = 0x80 | ((acc >> 3) & 0x1f)) when the increment is set. A
zero countdown reads each channel's events instead (0x0b49): bytes below
0x80 are opcodes — 0 countdown = 0x00nn, 1 noise register and accumulator /
increment, 2 the RL bits of register 0x20 + ch, 3 the patch number for the
next note, 4 jump (count 0) or loop `count` times through slot + 0xf, 5 the
end — read in a loop; 0x80 is a rest (key off, KF = 0, a discarded byte);
0x81–0xfe a note (& 0x7f) with a slide-step byte: key off, KF = 0, the
patch loaded (0x0ebc: 0x6b00 + 0x29 × patch into +4/+5, the LFO block
written directly if patch + 2 bit 7, register 0x20 + ch = the record's RL
bits with the patch's FB/CON, 0x38 + ch, then the 20 operator registers),
then keyed on (0x0b9f: the note transposed by the patch's first byte and
folded into 0x00..0x60 — the folded value replaces the record's note, so a
slide's re-keys transpose again — the key code from 0x0fb0 into 0xc002, the
four operator levels, KC, key on = ch | patch + 8, the LFO reset if patch +
2 bit 1; no KF write); 0xff a slide step with no key event. **The end
(opcode 5)** pops three words, sets 0xc024, stops the slot (0x02bb /
0x02f3 — music-stops.md: the priority byte cleared, 0x0f cleared if the
mask has channel 7, the hand-back with 0xc024 skipping the mute of a free
channel, the slot zeroed), clears 0xc024 and returns to the slot-B check
(slot A) or the handler (slot B); the rest of the channel loop is
abandoned. The player's writes go through 0x00fc directly — never gated by
0xc023, which gates only the music's writes to channels an effect owns.

The operator-level computation exists twice in the ROM: the music's
(0x059b) and the player's copy (0x0bfc) — byte-identical but for the
channel source (0xc022), the direct writes, and the tables' addresses
(0x0cd6 / 0x0d62 instead of 0x0675 / 0x0701, which changes the `rst 0x20`
page-carry costs). The translation parameterises `levels` / `key_scale` /
`curve` / `mul_table` with a `Levels` variant.

**The hit spark (command 0x01)**: header 0x3407 = `81 03` (slot A,
priority 1, channels 6 and 7), stream 0x3409–0x342d. Tick 1: countdown 10,
patch 0x40, RL bits, notes 0x2b (channel 7) and 0x37 (channel 6) with slide
−127; ten ticks of descending slides (a re-key every second tick: KC 0x34,
0x2e, 0x29, 0x24, 0x1e, 0x19 on channel 7 — one slide semitone plus the
patch's transpose of −3 per re-key); a rest; countdown 23, patch 0x41,
notes with slide +53 (KF 0x35, 0x6a, 0x9f, 0xd4, re-keys); countdown 7,
notes with slide +63; then the end. 408 writes on channels 6 and 7 over
166 ms, interleaved with tune 0x28's writes on channels 4 and 5. All 27
effects of this ROM use opcodes 0, 2, 3 and 5 and end with a 05 that is the
first byte of channel 7's segment; opcode 4 appears only in 0x04, the 0xff
event only in 0x05 and 0x0f, patches with an LFO block only in 0x0f, 0x1b,
0x1c; opcode 1 (noise) and slot B in none.

## Verification

`crates/sf-audio/tests/effect_hit.rs` (local data; skips without it):

| Window | What | Result |
| --- | --- | --- |
| sf-punch-01, 2716..=2741, **the native hit event's latch** (the native versus run's per-frame latch and handler cycles; music-stops.md's harness) | tune 0x28 on channels 0–5; the impact sample 0x84 (2717) through the intake's bit-7 path; **the hit spark 0x01 (2718)**: start, hand-back of channels 6/7, 11 ticks of slides and re-keys, rest, two more phrases, the self-end with 0xc024 | 1,060 reference writes: equal in order, **all 1,060 on the exact chip clock at latency 92 and at both band ends (45, 138)**; **23,304 output samples identical** (hit-sound-hook's stream); RAM (0xc000–0xc5cf) equal to the dumps at 2734, 2738, 2741 (idle frame ends; slot and priority cleared, tune 0x28 loaded) |
| the same window, the reference's latch | | 1,060 writes, all on the exact clock, output identical |
| the fixed fight, 2460..=2700, reference latch, writes only | effect 0x03 (priority 3, channels 6/7) at 2535; **the landing effect 0x19 at 2545 rejected** (priority 1 < 3: the slot's priority byte stays 3); the hit spark 0x01 at 2620 | 7,561 writes equal in order, all on the exact clock (7,544 before ticket #36's NMI timings, music-power-on.md) |
| sf-win-01, 3179..=3292, reference latch, writes only | the continue countdown's tally effect 0x0a restarted every three frames (**the equal-priority restart**: the slot reloaded, the same mask, no hand-back), 36 times | 5,048 writes equal in order, all on the exact clock; RAM equal at 3292 |

The fixture frames (2716, 2460, 3179) and the RAM-check frames are frame
ends at which the music CPU is idle, chosen as music-stops.md describes;
the idle-loop phases (4, 7, 3) come from the `music_phase` example. The
native hit event's latch write for 2718 lands at the vblank handler's 1,406
cycles plus the latency (latch-timing.md); at every latency of the band
the effect's writes are identical, so the band does not touch this window.

**Found and fixed on the way (chip model):** a timer flag standing from an
earlier deadline lost its set time when the core processed the *next*
deadline early at a sample boundary — the model recorded the new deadline
as the flag's time, so a status read between the two saw the flag clear.
In the fixed fight at 43.697 s the reference served timer B in a handler's
tail where the model (B's flag "not yet set") let the handler end and
served timer A first; the write order differed from there. A standing flag
now keeps its time; a later deadline re-sets the flag only after a reset
written before it (`opm.rs`). The music-phrase, music-stops and effect
suites pass on the corrected model.

## Not covered

- **Slot B, opcode 1 and the noise tick** (dead with this ROM's data),
  **opcode 4** (only effect 0x04), **the 0xff event** (0x05, 0x0f), **the
  LFO block and LFO reset** (0x0f, 0x1b, 0x1c), **a note with slide 0**
  (0x22): translated from the listing, reached by no window.
- **A replaced effect freeing a channel** (a higher-priority effect over a
  running one) and **an effect over a tune that uses channels 6/7** (0x33
  has a stream on 6: the music's writes there are skipped by 0xc023, the
  path `owned()` implements): not in the windows.
- **Opcode bytes 0x06–0x7f**: the original jumps through code bytes;
  reported as unrecovered (0x0e62).
- **The NMI's scheduler-slice delivery**: a 68000 latch write reaches the
  Z80 at the start of MAME's 500-ns slice, up to two clocks before the
  write's time. Ticket #36 (music-power-on.md) places the NMI at the clock
  the write falls in and accepts a pending timer interrupt right after an
  NMI taken behind `ei`; with that, every write of the windows above is on
  the exact clock.
- **The coin effect 0x0f's slide-only event, LFO patch and restart**, listed
  above as reached by no window, are reached by the whole-fight power-on
  run (music-power-on.md).
