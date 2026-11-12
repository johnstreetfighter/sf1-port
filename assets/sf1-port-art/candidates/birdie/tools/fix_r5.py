"""Birdie's ground set (AP21, #185), round five: the maintainer's round-four notes, fixed by hand. No generation.

usage: fix_r5.py <candidates/birdie folder> <out folder>

- crouching kick ("the leg join is perfect now, but somehow his front hand is doubled"): round four's source laid
  round three's picture from x52 on, 4 pixels higher, and that crop held the crouch's front fist as well as the leg,
  so a second fist sat 4 pixels above his own. Above the leg (rows 0-49, from x44 on) the pixels are the approved
  crouch's again, his one fist; the leg and its join are round four's, untouched.
- rising ("his butt still isn't rounded"): below his belt the back of his seat ran straight down and turned in at a
  square corner. It is redrawn as a curve, rows 54-71, swelling out to its rearmost at rows 59-62 and rounding in to
  the kneeling thigh, as his crouch's seat (his jump tuck's) does: one-pixel black outline, the jeans' black-and-grey
  dither inside.
"""
import json, os, sys
import rompose as r, birdie as b
C, OUT = sys.argv[1], sys.argv[2]
os.makedirs(OUT, exist_ok=True)
report = {}

k = r.load(f"{C}/touched4/crouching_kick.png")
crouch = b.crouch()
n = 0
for y in range(0, 50):
    for x in range(44, k.w):
        v = crouch.at(x, y)
        if k.p[y][x] != v: k.p[y][x] = v; n += 1
k.save(f"{OUT}/crouching_kick.png")
report["crouching_kick"] = [f"the second front fist, 4 pixels above his own, removed: above the leg (rows 0-49, from x44) the {n} pixels that differed are the approved crouch's again; the leg and its join are round four's, untouched"]

s = r.load(f"{C}/touched4/rising.png")
EDGE = {54: 17, 55: 15, 56: 14, 57: 13, 58: 12, 59: 11, 60: 11, 61: 11, 62: 11, 63: 12, 64: 12, 65: 13, 66: 14,
        67: 15, 68: 16, 69: 18, 70: 19, 71: 20}
n = 0
for y, e in EDGE.items():
    old = next(x for x in range(s.w) if s.p[y][x] != r.T)
    for x in range(min(e, old), max(e, old) + 1):
        v = 0 if x == e else (0 if (x + y) % 2 else 11)
        if x < e: v = r.T
        if s.p[y][x] != v: s.p[y][x] = v; n += 1
s.save(f"{OUT}/rising.png")
report["rising"] = [f"the back of his seat below the belt redrawn as a rounded curve (rows 54-71, rearmost at rows 59-62, rounding in to the kneeling thigh), with his black outline and the jeans' black-and-grey dither inside, as his crouch's seat curves: {n} pixels"]
print(json.dumps(report, indent=1))
