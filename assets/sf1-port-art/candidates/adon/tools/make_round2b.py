"""Adon's ground set (AP25, #189), round two: the poses rebuilt on the round-two crouch.

usage: make_round2b.py <round folder> <touched2 folder> <round-one rising.png>

- touched2/: the crouch (round two's inpainting, touched up), and the crouching block, hit and punch: the same ROM
  upper bodies as round one (his standing block 0x312c, standing hit 0x3180, jab 0x30c4) laid on the new crouch's
  legs (the maintainer, round one: "same as the crouch").
- round folder: the rising pose's inpainting source and mask (the maintainer, round one: the same calf and foot join as the
  crouch). Round one's rising pose, kept as it is, with its front leg (round one's crouch's thigh, knee, wrap and
  foot, wherever its upper body and hand leave it showing) replaced by the new crouch's front leg: the thigh, the
  shaded calf running into the ankle wrap, the foot and its shadow. The mask covers the hip where the thigh leaves
  the lowered shorts, and the whole calf from the knee down to the wrap's top, with the calf as the guide; the
  upper body, hand, shorts, rear leg, wrap and foot are outside it.
"""
import os, sys
import rompose as r, adon
from src import Src
RND, OUT, RISING = sys.argv[1:4]
os.makedirs(RND, exist_ok=True); os.makedirs(OUT, exist_ok=True)
C1, C2 = adon.crouch(1), adon.crouch(2)
C2.save(f"{OUT}/crouch.png")
adon.upper_on_legs(0x312c, 2, 1, dx=-1, dy=-5, upto=40, keep=lambda x, y: not (x >= 34 and y >= 33), legs_from=40).save(f"{OUT}/crouching_block.png")
adon.upper_on_legs(0x3180, 2, 1, dx=-1, dy=-2, upto=47, keep=lambda x, y: y < 40 or x >= 36, legs_from=40).save(f"{OUT}/crouching_hit.png")
adon.upper_on_legs(0x30c4, 2, 2, dx=3, upto=43, width=96).save(f"{OUT}/crouching_punch.png")


def front(x, y):
    """Round one's and round two's front leg: the thigh right of the shorts, the calf, wrap, foot and shadow."""
    return y >= 42 and x >= 31


R = r.load(RISING)
s = Src(96, 72)
s.lay(R, 0, 0)
leg = set()
for y in range(42, 72):
    for x in range(31, 64):
        if front(x, y) and R.p[y][x] != r.T and R.p[y][x] == C1.p[y][x]:
            s.c.p[y][x] = r.T; leg.add((x, y))
for y in range(42, 72):
    for x in range(31, 64):
        if front(x, y) and C2.p[y][x] != r.T and s.c.p[y][x] == r.T and (R.p[y][x] == r.T or (x, y) in leg):
            s.c.p[y][x] = C2.p[y][x]
body = lambda x, y: R.p[y][x] != r.T and (x, y) not in leg
WRAP_TOP = 59
s.paint(28, 46, 56, WRAP_TOP + 1, keep=lambda x, y: body(x, y) or (y >= WRAP_TOP and s.c.at(x, y) != r.T))
s.save(RND, 'rising')
print(len(leg), 'pixels of the old front leg replaced')
