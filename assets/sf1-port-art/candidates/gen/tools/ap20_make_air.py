"""Inpainting sources and masks for Gen's jumping punches (AP20, #184), round one.

usage: ap20_make_air.py <out folder>

Each source is his ROM top-of-jump tuck (0x22a6) with a whole ROM arm from another of his poses
laid on it, lined up by his head; the mask (white) covers only the shoulder join and what the
tuck's front arm left.
"""
import os, sys
import rompose as r
from src import Src
OUT = sys.argv[1]
os.makedirs(OUT, exist_ok=True)
TUCK = r.grid(0x22a6, 1, 2, 1)
head = lambda x, y: y < 14 and 30 <= x < 48       # his ROM head stays, pixel for pixel

def tuck(w, h):
    s = Src(w, h); s.lay(TUCK, 0, 0)
    s.clear(48, 10, 64, 30, keep=lambda x, y: head(x, y))   # the tuck's front cuff and clawed hand
    return s

# Jump punch: his straight punch's arm (0x2126, x56-92, y11-31: sleeve, cuff and fist), whose head
# lies 8 pixels right of the tuck's, laid 8 left so it leaves the tuck's front shoulder at shoulder height.
s = tuck(88, 80)
s.lay(r.grid(0x2126, 1, 2, 2).crop(56, 11, 37, 21), 48, 11)
fist = lambda x, y: x >= 60 and s.c.at(x, y) != r.T
rear = lambda x, y: x < 48 and y < 23                   # his rear hand at his chest, outline and all
s.paint(38, 10, 58, 33, keep=lambda x, y: head(x, y) or fist(x, y) or rear(x, y))
s.save(OUT, 'jump-punch')

# Jump punch down: the front arm of his arm flurry's wheeling strike (0x2246, from x56, y40-76:
# sleeve, cuff and open hand driven down and forward), laid 8 left and 22 up so its sleeve leaves the
# tuck's front shoulder and the hand reaches down past his knees.
def flurry_arm():
    g = r.grid(0x2246, 1, 2, 2).crop(56, 38, 36, 39)      # x56-91, y38-76
    for y in range(g.h):
        for x in range(g.w):
            X, Y = x + 56, y + 38
            if Y < 40 and X < 66: g.p[y][x] = r.T            # his beard
            if X < 74 and Y > 59 + 0.7 * (X - 56): g.p[y][x] = r.T   # his thigh under the sleeve
    return g
s = tuck(88, 80)
s.lay(flurry_arm(), 48, 16)
hand = lambda x, y: x >= 62 and y >= 30 and s.c.at(x, y) != r.T
s.paint(38, 10, 60, 40, keep=lambda x, y: head(x, y) or hand(x, y) or (x < 48 and y < 23))
s.save(OUT, 'jump-punch-down')
