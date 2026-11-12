"""Birdie's ground set (AP21, #185), round three: the two poses the maintainer rejected in round two.

usage: make_round3.py <round folder>

- crouching kick (the maintainer, round two: "Join still too high"): his kick's leg (0x27a8) now comes out of the hip crease,
  where the crouch's front thigh meets the seat, turned 14 degrees up about the hip (rot.py) so the boot stays at shin
  height; the turned leg is laid as a guide and the mask covers it and the hip, as for the approved high kick.
- rising (the maintainer, round two: the kneel "looks too similar to the crouch"): his headbutt lunge's upper body (0x28a0),
  bent far forward with the head low and both fists over the front knee, laid on round two's kneeling legs (front boot
  planted, rear knee down) at the waist; only the waist band where its belt meets the jeans is inpainted.
"""
import os, sys
import rompose as r, birdie as b, rot
from src import Src
RND = sys.argv[1]
os.makedirs(RND, exist_ok=True)
M = b.MARGIN
C = b.crouch()
KICK = r.grid(0x27a8, 1, 3, 3)

def shadow(s, x0, x1):
    for y in (88, 89):
        for x in range(x0, x1):
            if 0 <= x < s.c.w and s.c.p[y][x] == r.T: s.c.p[y][x] = 0

s = Src(112, 92)
s.lay(C, 0, 0)
s.clear(44, 50, 112, 92)                         # the crouch's front thigh, shin and boot
s.clear(40, 76, 112, 92)
leg = KICK.crop(34, 63, 66, 26)
HIP = (40, 74)                                   # the hip crease on the crouch, where the thigh meets the seat
turned = rot.turn(leg, 14, (2, 12), 112, 92, HIP)
s.lay(turned, 0, 0)
shadow(s, 14, 108)
near = {(x + dx, y + dy) for y in range(92) for x in range(112) if turned.p[y][x] != r.T for dx in (-2, -1, 0, 1, 2) for dy in (-2, -1, 0, 1, 2)}
s.mask |= {(x, y) for x, y in near if 0 <= x < 112 and 0 <= y < 88}
s.paint(28, 50, 56, 88, keep=lambda x, y: y < 54 and x < 40 and s.c.at(x, y) != r.T)
s.save(RND, 'crouching-kick')

KNEEL = r.load(os.path.join(os.path.dirname(RND), 'touched2', 'rising.png'))
LUNGE = r.grid(0x28a0, 1, 2, 2)
s = Src(96, 92)
s.lay(KNEEL, 0, 0)
s.clear(0, 0, 96, 54)                            # the crouch's upper body goes
DX, DY = -8, 12                                  # the lunge's belt onto the kneel's waist
upper = LUNGE.crop(0, 0, 96, 46)                 # head, arms, vest and belt
s.lay(upper, DX, DY)
s.paint(10, 46, 56, 62, keep=lambda x, y: y < 52 and s.c.at(x, y) != r.T and x > 40)
s.save(RND, 'rising')
