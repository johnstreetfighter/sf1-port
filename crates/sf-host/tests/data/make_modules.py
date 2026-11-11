#!/usr/bin/env python3
"""Writes the tracker modules the ST9 tests (tests/modules.rs) play.

Original and tiny: each is one 32-byte sine cycle, looped, played as three
one-note patterns of eight rows. The notes are C-4, G-4 and C-5 (frequency
ratios 1 : 1.5 : 2), so a test can tell which pattern it is hearing by its
pitch. The song restarts at order 1, so it plays

    pattern 0, then patterns 1 and 2 for ever.

Where the format has a restart position (MOD, XM, 669) the header says 1;
where it has none (S3M, IT, MTM) the last pattern ends in a position jump
(Bxx) to order 1, which libopenmpt treats as the loop. loop-top.mod is the
same song with restart position 0, so it loops from the top. loop-tempo.mod
is loop.mod with the speed set to 6 in order 0 and to 3 in order 2: the speed
carries over the jump back, so the loop's second pass runs at 3 from its first
row, which a length measured by playing it again does not give.

Run:  python make_modules.py   (writes beside this script; no arguments)
"""
import math
import struct
from pathlib import Path

OUT = Path(__file__).resolve().parent
ROWS = 8
CYCLE = 32
SINE = [round(100 * math.sin(2 * math.pi * i / CYCLE)) for i in range(CYCLE)]  # signed 8-bit


def signed(data):
    return bytes(v & 0xFF for v in data)


def unsigned(data):
    return bytes((v + 128) & 0xFF for v in data)


def delta(data):
    out, prev = [], 0
    for v in data:
        out.append((v - prev) & 0xFF)
        prev = v
    return bytes(out)


def pad(text, n):
    return text.encode("ascii").ljust(n, b"\0")[:n]


# --- MOD (M.K., ProTracker) -------------------------------------------------

def mod(restart, speeds=(0, 0, 0)):
    periods = [428, 285, 214]  # C-2, G-2, C-3 in ProTracker's table: 1 : 1.5 : 2
    head = pad("sf1-port test loop", 20)
    sample = pad("sine", 22) + struct.pack(">HBBHH", CYCLE // 2, 0, 64, 0, CYCLE // 2)
    head += sample + (b"\0" * 30) * 30
    head += bytes([3, restart]) + bytes([0, 1, 2]) + bytes(125) + b"M.K."
    patterns = b""
    for p, period in enumerate(periods):
        rows = [bytes(4)] * 64
        rows_ch0 = list(rows)
        speed = speeds[p]
        # sample 1; Fxx sets the speed (ticks a row) where `speeds` has one, and it carries on
        rows_ch0[0] = bytes([(period >> 8) & 0x0F, period & 0xFF, 0x1F if speed else 0x10, speed])
        rows_ch0[ROWS - 1] = bytes([0, 0, 0x0D, 0x00])  # Dxx: pattern break (the last one ends the song)
        for r in range(64):
            patterns += rows_ch0[r] + bytes(12)  # channels 1-3 empty
    return head + patterns + signed(SINE)


# --- XM (FastTracker 2) -----------------------------------------------------

def xm():
    notes = [49, 56, 61]  # C-4, G-4, C-5
    header = b"Extended Module: " + pad("sf1-port test loop", 20) + b"\x1a" + pad("make_modules.py", 20)
    body = struct.pack("<HHHHHHHH", 3, 1, 2, 3, 1, 1, 6, 125) + bytes([0, 1, 2]) + bytes(253)
    # version word, then the header size (which counts itself and what follows)
    out = header + struct.pack("<H", 0x0104) + struct.pack("<I", 4 + len(body)) + body
    for note in notes:
        data = bytearray()
        for r in range(ROWS):
            if r == 0:
                data += bytes([0x83, note, 1])  # note + instrument present
            else:
                data += bytes([0x80])
            data += bytes([0x80])  # channel 2 empty
        out += struct.pack("<IBHH", 9, 0, ROWS, len(data)) + bytes(data)
    instrument = struct.pack("<I", 263) + pad("sine", 22) + bytes([0]) + struct.pack("<H", 1)
    instrument += struct.pack("<I", 40) + bytes(96) + bytes(48) + bytes(48)
    # envelope counts, sustain/loop points and types (10), vibrato (4), fadeout (2), reserved (22)
    instrument += bytes(10) + bytes(4) + struct.pack("<H", 0) + bytes(22)
    assert len(instrument) == 263, len(instrument)
    sample = struct.pack("<IIIBbBBbB", CYCLE, 0, CYCLE, 64, 0, 1, 128, 0, 0) + pad("sine", 22)
    assert len(sample) == 40
    return out + instrument + sample + delta(SINE)


# --- S3M (Scream Tracker 3) -------------------------------------------------

def s3m():
    notes = [0x40, 0x47, 0x50]  # C-4, G-4, C-5 (octave << 4 | semitone)
    orders = bytes([0, 1, 2, 255])
    fixed = 96 + len(orders) + 2 + 6
    ins_off = (fixed + 15) // 16 * 16
    pats = []
    for p, note in enumerate(notes):
        rows = bytearray()
        for r in range(64):
            if r == 0:
                rows += bytes([0x20, note, 1])
            if r == ROWS - 1:
                # Cxx pattern break; the last pattern jumps back to order 1 (Bxx)
                rows += bytes([0x80, 0x03, 0x00]) if p < 2 else bytes([0x80, 0x02, 0x01])
            rows += bytes([0])
        pats.append(struct.pack("<H", len(rows) + 2) + bytes(rows))
    sample_off = ins_off + 80
    pat_off = []
    at = (sample_off + CYCLE + 15) // 16 * 16
    for pat in pats:
        pat_off.append(at)
        at = (at + len(pat) + 15) // 16 * 16
    header = pad("sf1-port test loop", 28) + b"\x1a\x10\0\0"
    header += struct.pack("<HHHHHH", len(orders), 1, 3, 0, 0x1320, 2) + b"SCRM"
    header += bytes([64, 6, 125, 0xB0, 0, 0xFC]) + bytes(8) + struct.pack("<H", 0)
    header += bytes([0] + [255] * 31)
    header += orders + struct.pack("<H", ins_off // 16) + b"".join(struct.pack("<H", o // 16) for o in pat_off)
    assert len(header) == fixed
    out = header.ljust(ins_off, b"\0")
    sample = bytes([1]) + pad("SINE.SMP", 12) + bytes([0]) + struct.pack("<H", (ins_off + 80) // 16)
    sample += struct.pack("<IIIBBBB", CYCLE, 0, CYCLE, 64, 0, 0, 1) + struct.pack("<I", 8363) + bytes(12)
    sample += pad("sine", 28) + b"SCRS"
    assert len(sample) == 80, len(sample)
    out += sample + unsigned(SINE)
    for off, pat in zip(pat_off, pats):
        out = out.ljust(off, b"\0") + pat
    return out


# --- IT (Impulse Tracker) ---------------------------------------------------

def it():
    notes = [48, 55, 60]  # C-4, G-4, C-5
    orders = bytes([0, 1, 2, 255])
    head_len = 192 + len(orders) + 4 + 12
    sample_off = head_len
    pat_start = sample_off + 80 + CYCLE
    pats = []
    for p, note in enumerate(notes):
        data = bytearray()
        for r in range(ROWS):
            if r == 0:
                data += bytes([0x81, 0x03, note, 1])
            if r == ROWS - 1 and p == 2:
                data += bytes([0x81, 0x08, 0x02, 0x01])  # Bxx: jump to order 1
            data += bytes([0])
        pats.append(struct.pack("<HH", len(data), ROWS) + bytes(4) + bytes(data))
    header = b"IMPM" + pad("sf1-port test loop", 26) + struct.pack("<H", 0x1004)
    header += struct.pack("<HHHHHHHH", len(orders), 0, 1, 3, 0x0214, 0x0200, 0x0001, 0)
    header += bytes([128, 48, 6, 125, 128, 0]) + struct.pack("<HI", 0, 0) + bytes(4)
    header += bytes([32] * 64) + bytes([64] * 64)
    assert len(header) == 192, len(header)
    offs, at = [], pat_start
    for pat in pats:
        offs.append(at)
        at += len(pat)
    out = header + orders + struct.pack("<I", sample_off) + b"".join(struct.pack("<I", o) for o in offs)
    sample = b"IMPS" + pad("sine.raw", 12) + bytes([0, 64, 0x11, 64]) + pad("sine", 26) + bytes([1, 0x80])
    sample += struct.pack("<IIIIIII", CYCLE, 0, CYCLE, 8363, 0, 0, sample_off + 80) + bytes(4)
    assert len(sample) == 80, len(sample)
    return out + sample + signed(SINE) + b"".join(pats)


# --- 669 (Composer 669) -----------------------------------------------------

def f669():
    notes = [12, 19, 24]  # C-4, G-4, C-5 (semitones above C-3)
    out = b"if" + pad("sf1-port test loop", 108)
    orders = bytes([0, 1, 2]) + bytes([0xFF] * 125)
    out += bytes([1, 3, 1]) + orders + bytes([4] * 128) + bytes([ROWS - 1] * 3 + [0] * 125)
    out += pad("sine.smp", 13) + struct.pack("<III", CYCLE, 0, CYCLE)
    for note in notes:
        for r in range(64):
            for ch in range(8):
                out += bytes([note << 2, 0x0F, 0xFF]) if (r == 0 and ch == 0) else bytes([0xFF, 0x00, 0xFF])
    return out + unsigned(SINE)


# --- MTM (MultiTracker) -----------------------------------------------------

def mtm():
    notes = [12, 19, 24]
    out = b"MTM" + bytes([0x10]) + pad("sf1-port test loop", 20)
    out += struct.pack("<HBBHBBBB", 3, 2, 2, 0, 1, 0, ROWS, 1) + bytes([8] * 32)
    out += pad("sine", 22) + struct.pack("<IIIbBB", CYCLE, 0, CYCLE, 0, 64, 0)
    out += bytes([0, 1, 2]) + bytes(125)
    for t, note in enumerate(notes):
        rows = bytearray()
        for r in range(64):
            cell = bytearray(3)
            if r == 0:
                cell = bytearray([note << 2, 0x10, 0])  # note, instrument 1
            if r == ROWS - 1 and t == 2:
                cell = bytearray([0, 0x0B, 0x01])  # Bxx: jump to order 1
            rows += cell
        out += bytes(rows)
    for t in range(3):
        out += struct.pack("<H", t + 1) + bytes(62)
    return out + unsigned(SINE)


FILES = {
    "loop.mod": mod(1),
    "loop-top.mod": mod(0),
    "loop-tempo.mod": mod(1, (6, 0, 3)),
    "loop.xm": xm(),
    "loop.s3m": s3m(),
    "loop.it": it(),
    "loop.669": f669(),
    "loop.mtm": mtm(),
}

if __name__ == "__main__":
    for name, data in FILES.items():
        (OUT / name).write_bytes(data)
        print(f"{name}: {len(data)} bytes")
