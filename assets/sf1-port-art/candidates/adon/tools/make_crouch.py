"""Inpainting source and mask for Adon's crouch (AP25, #189).

usage: make_crouch.py <out folder> [front_dx rear_dx feet]

His ROM moves have no crouch (moves 5 to 8 are empty). The source is his stance (0x3060) from the head down to
its belt (rows 0-42), on the shorts and horizontal front thigh of the top of his jump (0x3248, rows 40-48, and
the shorts' seat to row 53), whose belt lines up with the stance's 3 pixels right and 2 down; the tuck's shin,
folded under the thigh, is left out. Under them stand his stance's own ankle wraps, bare feet and ground
shadow: the front leg's (x34-50, rows 80-95) under the knee, the rear leg's (x8-23, rows 79-95) under the seat.
The mask (white) covers only the knee's underside and the ankles between the thigh and seat and the wraps.
"""
import os, sys
import rompose as r
from src import Src
STANCE = r.grid(0x3060, 1, 2, 1)
TUCK = r.grid(0x3248, 1, 2, 1)
TX, TY = 3, 2                  # the tuck's place under the stance's belt
BELT = 43                      # stance rows 0..BELT-1 (head down to the belt)
THIGH = 49                     # tuck rows 40..THIGH-1: shorts and thigh; below it only the shorts' seat (x < 24)
SEAT = 54

def source(front_dx=4, rear_dx=4, feet=55, w=64, h=72):
    s = Src(w, h)
    t = TUCK.crop(0, 40, 64, SEAT - 40)
    s.lay(t, TX, 40 + TY, keep=lambda x, y, v: y < THIGH - 40 or x < 24)
    s.lay(STANCE.crop(0, 0, 64, BELT), 0, 0)
    front = STANCE.crop(34, 80, 17, 16)
    rear = STANCE.crop(8, 79, 16, 17)
    s.lay(rear, 8 + rear_dx, feet - 1)
    s.lay(front, 34 + front_dx, feet)
    return s

if __name__ == '__main__':
    out = sys.argv[1]; os.makedirs(out, exist_ok=True)
    a = [int(v) for v in sys.argv[2:5]] if len(sys.argv) > 4 else [4, 4, 55]
    s = source(*a)
    feet = a[2]
    wrap = lambda x, y: y >= feet + 2 and s.c.at(x, y) != r.T
    s.paint(8, THIGH + TY - 3, 60, feet + 3, keep=lambda x, y: wrap(x, y) or (y < THIGH + TY and s.c.at(x, y) != r.T and x < 50 and y < THIGH + TY - 1))
    s.save(out, 'crouch')
    print(s.c.bbox())
