"""Birdie's ground set (AP21, #185), round four: the maintainer's round-three notes.

usage: make_round4.py <round folder>

- crouching kick (the maintainer, round three: "try again, now it's too low. If we have to accept one, the previous iteration
  was good enough"): round three's level leg, from its thigh outward (x52 on), raised 4 pixels, so the join sits about
  halfway between round two's and round three's; only the hip join is inpainted. Round two's candidate is the fallback.
- rising (the maintainer, round three: "Almost good, there is clipping or a bad join where his bottom/butt is"): round three's
  pose with only the seat and the back of the waist re-inpainted, where the lunge's belt met the kneeling legs.
"""
import os, sys
import rompose as r, birdie as b
from src import Src
RND = sys.argv[1]
os.makedirs(RND, exist_ok=True)
T3 = os.path.join(os.path.dirname(RND), 'touched3')
C = b.crouch()

R3 = r.load(f"{T3}/crouching_kick.png")
s = Src(112, 92)
s.lay(C, 0, 0)
s.clear(44, 50, 112, 92)                         # the crouch's front thigh, shin and boot
s.clear(40, 76, 112, 92)
UP = 4
s.lay(R3.crop(52, 0, 60, 88), 52, -UP)           # round three's leg, thigh to boot, 4 pixels higher
for y in (88, 89):
    for x in range(14, 110):
        if s.c.p[y][x] == r.T: s.c.p[y][x] = 0
s.paint(28, 50, 56, 84, keep=lambda x, y: y < 54 and x < 40 and s.c.at(x, y) != r.T)
s.save(RND, 'crouching-kick')

s = Src(96, 92)
s.lay(r.load(f"{T3}/rising.png"), 0, 0)
s.paint(12, 46, 35, 63)
s.save(RND, 'rising')
