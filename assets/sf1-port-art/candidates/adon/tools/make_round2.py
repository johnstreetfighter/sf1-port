"""Adon's ground set (AP25, #189), round two: the crouch's front leg, and the sweep.

usage: make_round2.py <round folder> [thigh_deg]

- crouch (the maintainer, round one: "calf/foot join on the front leg is bad"): the round-one crouch with its front leg
  rebuilt from the hip down. Its horizontal thigh is turned `thigh_deg` up about the hip (rot.py), so the knee
  rises a little and leaves room for the calf; under the knee stands his stance's own front lower leg as one ROM
  piece (x33-50: the shaded calf's rows 70-79 running into the ankle wrap, 4 rows of the wrap's plain band left
  out, the bare foot and the ground shadow), at the same ground line, so he stays 70 pixels tall. The mask covers the whole calf from the knee's
  underside down to the wrap's top, with the knee and the hip under the turned thigh; the calf's ROM pixels are the
  guide. His head, arms, body, shorts, rear leg, the wrap and the foot stay his ROM pixels.
"""
import os, sys
import rompose as r, adon, rot
from src import Src
RND = sys.argv[1]
DEG = int(sys.argv[2]) if len(sys.argv) > 2 else 8
os.makedirs(RND, exist_ok=True)
S = r.grid(0x3060, 1, 2, 1)
HIP = (31, 47)
CALF, CUT = 70, 4                         # the stance calf's first row; wrap rows left out


def crouch_source(base=None):
    """The crouch with its front leg rebuilt: the thigh turned DEG about the hip, and under the knee his stance's
    front lower leg (x33-50) as one ROM piece: the calf's rows 70-79, the wrap's top outline (row 80), the wrap from
    row 85 (CUT rows of the wrap's plain band left out, so the calf shows at his crouch's height), foot and shadow."""
    C = base or adon.crouch(1)
    s = Src(64, 72)
    thigh = r.P(64, 72)
    for y in range(72):
        for x in range(64):
            v = C.p[y][x]
            if v == r.T: continue
            front = y >= 42 and ((x >= 31 and y < 56) or (x >= 33 and y >= 53))
            if front and y < 56: thigh.p[y][x] = v
            elif not front: s.c.p[y][x] = v
    for y in (69, 70):                                     # the rear foot's shadow only; the front's comes with it
        for x in range(30, 64): s.c.p[y][x] = r.T
    rows = list(range(CALF, 81)) + list(range(81 + CUT, 96))
    piece = r.P(18, len(rows))
    for k, sy in enumerate(rows):
        for x in range(18): piece.p[k][x] = S.p[sy][33 + x]
    s.lay(piece, 37, 71 - len(rows))
    s.lay(rot.turn(thigh, DEG, HIP, 64, 72, HIP), 0, 0)
    for y in range(40, 57):                                # the crouch's shorts back over the thigh's root
        for x in range(0, 31):
            v = C.p[y][x]
            if v in (0, 5, 6, 7): s.c.p[y][x] = v
    return s, 71 - len(rows) + (80 - CALF)                 # and the canvas row of the wrap's top outline


if __name__ == '__main__':
    s, wrap_top = crouch_source()
    ok = lambda x, y: y >= wrap_top and s.c.at(x, y) != r.T
    s.paint(28, 44, 56, wrap_top + 1, keep=lambda x, y: ok(x, y) or (x < 31 and s.c.at(x, y) in (0, 5, 6, 7)) or (y < 47 and x < 40))
    s.save(RND, 'crouch')
    print(s.c.bbox(), 'wrap top', wrap_top)
