"""Inpainting source and mask for Birdie's crouch (AP21, #185).

usage: make_crouch.py <out folder>

His ROM moves have no crouch. The source is the top of his jump (0x240c: upright, fists on guard,
knees drawn up) from the head down to the knees, set on the ground on his stance's own shoes and
two-row ground shadow (0x2400, rows 117-127); the mask (white) covers only the ankles between them.
"""
import os, sys
import rompose as r
from src import Src
OUT = sys.argv[1]
os.makedirs(OUT, exist_ok=True)
TUCK = r.grid(0x240c, 1, 2, 1)
STANCE = r.grid(0x2400, 1, 3, 1)
CUT = 76                       # tuck rows kept: 0..CUT-1 (head to knees; the hanging shins and shoes go)
FEET = 79                      # canvas row the stance's row 117 (top of the shoes) lands on

def crouch_source():
    s = Src(64, 92)
    s.lay(TUCK.crop(0, 0, 64, CUT), 0, 0)
    rear = STANCE.crop(0, 117, 16, 9)            # rear shoe, toe out behind
    front = STANCE.crop(38, 117, 24, 9)          # front shoe, toe toward the opponent
    s.lay(rear, 12, FEET)
    s.lay(front, 38, FEET)
    for y in (FEET + 9, FEET + 10):              # his two-row black ground shadow, heel to toe
        for x in range(12, 60):
            if s.c.p[y][x] == r.T: s.c.p[y][x] = 0
    return s

if __name__ == '__main__':
    s = crouch_source()
    shoe = lambda x, y: y >= FEET + 1 and s.c.at(x, y) != r.T
    s.paint(8, CUT - 6, 62, FEET + 3, keep=lambda x, y: shoe(x, y) or (y < CUT - 2 and s.c.at(x, y) == r.T and False))
    s.save(OUT, 'crouch')
