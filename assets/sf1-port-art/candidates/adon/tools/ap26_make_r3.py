"""Adon's jumping kick, round three (AP26, #190), by hand: no generation.

usage: ap26_make_r3.py <out folder>

The maintainer (round two): "shin/calf area on rear leg looks weird or lumpy." The kicking leg and the leg swap were fixed.
Round two's kept pose stays exactly as it is except the rear shin, from below the knee to the top of its wrap
(rows 55-63, x8-38): there the turned rising-pose calf and the knee lump under the kicking thigh are taken away,
and his stance's front calf (0x3060, rows 66-79, x35-46: a straight ROM calf running into its wrap, outlined in
his dark brown, lit on the right) is laid in its place: its 14 rows taken down to the 9 visible rows by nearest
sampling along the leg, and sheared a pixel a row, so it leaves from under the kicking thigh and the seat and runs
down and back into the rear wrap's top, which stays every ROM pixel of the tuck's. The calf goes behind his shorts
and the kicking thigh, so the knee is hidden under them.
"""
import os, sys
import rompose as r
OUT = sys.argv[1]
os.makedirs(OUT, exist_ok=True)
R2 = r.load("../ap26/touched2/jump_kick.png")
ST = r.grid(0x3060, 1, 2, 1)
TOP, BOTTOM = 55, 63
c = R2.copy()
cleared = 0
for y in range(TOP, BOTTOM + 1):
    for x in range(8, 39):
        v = c.p[y][x]
        if v == r.T: continue
        if y == TOP and v in (0, 7): continue          # the shorts' bottom edge
        if y == BOTTOM and x >= 20 and v in (11, 12, 13): continue   # the rear wrap's top
        c.p[y][x] = r.T; cleared += 1
laid = 0
for y in range(TOP, BOTTOM + 1):
    s = 66 + round((y - TOP) * 13 / (BOTTOM - TOP))
    dx = -21 + (BOTTOM - y)
    for x in range(31, 47):
        v = ST.p[s][x]
        if v != r.T and c.at(x + dx, y) == r.T:
            c.p[y][x + dx] = v; laid += 1
c.save(f"{OUT}/jump_kick.png")
print("cleared", cleared, "laid", laid)
