"""Inpainting sources and masks for Lee's air set (AP18, #182), round one.

usage: make_air.py <out folder>

Each source is his ROM top-of-jump tuck (0x1e4c) with a whole ROM limb from another of
his poses laid on it; the mask (white) covers only the join and what the old limb left.
"""
import os, sys
import rompose as r
from src import Src
OUT = sys.argv[1]
os.makedirs(OUT, exist_ok=True)
TUCK = r.grid(0x1e4c, 1, 2, 1)
GREYS = (8, 9, 10)
head = lambda x, y: y < 16 and x < 48          # his ROM head stays, pixel for pixel

# Jump punch: his straight punch's arm (0x1cc4, x34-63, y6-27: sleeve, forearm and fist), driven
# straight out from the tuck's front shoulder in place of the guard fist.
s = Src(88, 80); s.lay(TUCK, 0, 0)
s.clear(46, 26, 64, 41)
arm = r.grid(0x1cc4, 1, 2, 1).crop(34, 6, 30, 22)
s.lay(arm, 48, 14)
fore = lambda x, y: x >= 56 and s.c.at(x, y) != r.T
s.paint(40, 12, 58, 42, keep=lambda x, y: head(x, y) or fore(x, y))
s.paint(46, 34, 66, 42)
s.save(OUT, 'jump-punch')

# Jump punch down: his flying kick's front arm (0x1ec0), its head at the tuck's head to the pixel,
# reaching down and forward to the fist; laid at the same place in place of the guard fist.
s = Src(84, 80); s.lay(TUCK, 0, 0)
s.clear(46, 26, 64, 41)
fk = r.grid(0x1ec0, 1, 1, 2)
armd = lambda x, y, v: x >= 50 and 16 <= y <= 46 and v not in GREYS and (y <= 40 or x >= 62)
s.lay(fk, 0, 0, keep=armd)
hand = lambda x, y: x >= 64 and y >= 32 and s.c.at(x, y) != r.T
s.paint(40, 13, 64, 44, keep=lambda x, y: head(x, y) or hand(x, y))
s.save(OUT, 'jump-punch-down')

# Jump kick: his jumping heavy kick's leg (0x1ea8, x44-105, y60-79), its thigh at the tuck's
# front hip in place of the front knee, shin and foot, kicked straight out at waist height.
s = Src(104, 80); s.lay(TUCK, 0, 0)
s.clear(38, 39, 64, 62)
s.clear(30, 61, 50, 80)
leg = r.grid(0x1ea8, 1, 2, 3).crop(44, 60, 62, 21)
s.lay(leg, 40, 40)
s.paint(32, 36, 54, 62, keep=lambda x, y: x < 37 and y > 40 and s.c.at(x, y) in GREYS)
s.save(OUT, 'jump-kick')
