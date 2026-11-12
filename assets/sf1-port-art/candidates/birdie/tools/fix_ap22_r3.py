"""Birdie's heavy jumping kick (AP22, #186), round three, by hand. No generation.

usage: fix_ap22_r3.py <candidates/birdie folder> <out folder>

the maintainer, round two: "Nearly perfect. The kicking leg could just be a slight bit longer." The kicking leg is lengthened
5 pixels along its own line by stretching the shin: a cut is made across the shin, at right angles to the leg,
through (84, 46); everything beyond it (the rest of the shin, the cuff and the boot) moves 4 pixels out and 3 up, and
the 5-pixel gap is filled with the 5 pixels of shin just inside the cut, repeated, so the jeans' outline and
dither run on unbroken. Only leg pixels are copied (the jeans' dark pens, and the boot's beyond x88), so nothing of
his fist above it comes along. The hip join, the thigh, the boot's own pixels and everything else are round two's.
"""
import json, os, sys
import rompose as r
C, OUT = sys.argv[1], sys.argv[2]
os.makedirs(OUT, exist_ok=True)
src = r.load(f"{C}/touched-ap22-r2/jump_kick_high.png")
SX, SY = 4, -3                                       # 5 pixels along the leg, which rises at about 40 degrees
CX, CY = 84, 46                                      # the cut across the shin
JEANS = {0, 7, 11, 12, 13, 14}
s = lambda x, y: ((x - CX) * SX + (y - CY) * SY) / 5     # distance past the cut, along the leg
beyond = lambda x, y: x >= 78 and s(x, y) >= 0
leg = lambda x, y, v: v != r.T and (v in JEANS or x >= 88)
out = src.copy()
for y in range(src.h):
    for x in range(src.w):
        if beyond(x, y): out.p[y][x] = r.T
n = 0
for y in range(src.h):
    for x in range(src.w):
        if not beyond(x, y): continue
        v = src.at(x - SX, y - SY)
        if leg(x - SX, y - SY, v):
            out.p[y][x] = v; n += 1
out.save(f"{OUT}/jump_kick_high.png")
print(json.dumps({"jump_kick_high": [
    "the kicking leg lengthened 5 pixels along its own line, by hand: across the shin at right angles to the leg (through x84, y46) "
    "the rest of the shin, the cuff and the boot moved 4 pixels out and 3 up, and the gap filled with the 5 pixels of shin just inside "
    "the cut, repeated, so the outline and dither run on; the boot's pixels, the thigh and the hip join are round two's"]}, indent=1))
