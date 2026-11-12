"""Birdie's ground set (AP21, #185), round two: the two poses the maintainer rejected in round one.

usage: make_round2.py <round folder>

- crouching kick (the maintainer: "The leg join should be a little lower"): the same ROM leg of his kick (0x27a8) laid
  5 pixels lower on the crouch's front hip than round one's; only the hip and the space under the thigh inpainted.
- rising (the maintainer: the airborne-hit pose set on the ground "looks like he's getting hit"; his ROM art has no pose
  that reads as getting up): his approved crouch with the rear leg brought down onto its knee, the shin back along
  the floor; only the rear leg below the seat is inpainted. His head, guard, body, front leg and boot are the crouch's.
"""
import os, sys
import rompose as r, birdie as b
from src import Src
RND = sys.argv[1]
os.makedirs(RND, exist_ok=True)
M = b.MARGIN
C = b.crouch()
KICK = r.grid(0x27a8, 1, 3, 3)

s = Src(112, 92)
s.lay(C, 0, 0)
s.clear(44, 50, 112, 92)                         # the crouch's front thigh, shin and boot
s.clear(40, 76, 112, 92)
DY = -5                                          # round one's was -10
s.lay(KICK.crop(34, 63, 66, 26), 34 + 3 + M, 63 - 4 + DY)
for y in (88, 89):                               # his two-row ground shadow, under the kick
    for x in range(14, 34 + 3 + M + 64):
        if s.c.p[y][x] == r.T: s.c.p[y][x] = 0
s.paint(28, 50, 54, 72, keep=lambda x, y: y < 54 and s.c.at(x, y) != r.T and x < 40)
s.paint(30, 76, 56, 88)
s.save(RND, 'crouching-kick')

s = Src(68, 92)
s.lay(C, 0, 0)
s.clear(0, 74, 34, 88)                           # the rear boot and the bottom of the rear leg
s.paint(0, 66, 36, 88, keep=lambda x, y: x >= 30 and s.c.at(x, y) != r.T)
s.save(RND, 'rising')
