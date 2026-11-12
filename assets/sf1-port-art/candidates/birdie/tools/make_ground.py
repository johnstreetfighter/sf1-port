"""Birdie's ground set (AP21, #185), round one: the poses built on his crouch.

usage: make_ground.py <round folder> <touched folder>

- touched/: the crouch (his stance's upper body on the kept crouch inpainting's legs), and the crouching block and
  crouching punch, which are his standing block's (0x2588) and jab's (0x2480) upper bodies on the same legs; their
  waists match the crouch's row for row, so they need no generation.
- round folder: inpainting sources and masks (white = inpaint) for the crouching hit, crouching kick, sweep and
  high kick, each his ROM pixels with only the joins masked.
"""
import os, sys
import rompose as r, birdie as b
from src import Src
RND, TOUCHED = sys.argv[1], sys.argv[2]
os.makedirs(RND, exist_ok=True); os.makedirs(TOUCHED, exist_ok=True)
M = b.MARGIN
C = b.crouch()
C.save(f"{TOUCHED}/crouch.png")
b.upper_on_legs(0x2588, 3, 1).save(f"{TOUCHED}/crouching_block.png")
b.upper_on_legs(0x2480, 3, 3, dx=37).save(f"{TOUCHED}/crouching_punch.png")

def shadow(s, x0, x1, y=88):
    for yy in (y, y + 1):
        for x in range(x0, x1):
            if s.c.at(x, yy) == r.T and 0 <= x < s.c.w: s.c.p[yy][x] = 0

# Crouching hit: the first pose of his standing-hit reaction (0x2600, every hit stream starts on it), head knocked
# back and the near hand flung behind his hip, laid on the crouch's legs at its torso's place down to its belly;
# only the waist band where its belly and vest meet the jeans is inpainted, sparing the flung hand.
s = Src(68, 92)
base = b.upper_on_legs(0x2600, 3, 1, dx=5, upto=53)
s.lay(base, 0, 0)
hand = lambda x, y: x < 12
s.paint(10, 45, 60, 58, keep=lambda x, y: hand(x, y) or (y < 50 and s.c.at(x, y) in (1, 2, 3, 4, 5) and x < 44))
s.save(RND, 'crouching-hit')

# Crouching kick: his kick's leg (0x27a8, stream 0x115cc: thigh to boot, x34-99, y63-88) laid from the crouch's
# front hip, at the place his kick's own body would put it on these legs, in place of the crouch's front thigh,
# shin and boot. Only the hip, and the space the old front leg leaves under the thigh, are inpainted.
KICK = r.grid(0x27a8, 1, 3, 3)
def kick_source(dy=0, reach=0, lower=0):
    s = Src(112, 92)
    s.lay(C, 0, 0)
    s.clear(44, 50, 112, 92)                         # the crouch's front thigh, shin and boot
    s.clear(40, 76, 112, 92)
    leg = KICK.crop(34, 63, 66, 26)
    s.lay(leg, 34 + 3 + M + reach, 63 - 4 + dy)
    shadow(s, 14, 34 + 3 + M + reach + 64)
    return s
s = kick_source(dy=-10)
s.paint(28, 48, 52, 68, keep=lambda x, y: y < 52 and s.c.at(x, y) not in (r.T,) and x < 40)
s.paint(30, 72, 54, 88)
s.save(RND, 'crouching-kick')

# Sweep: the same leg laid low along the floor, reaching further, the boot's sole on the ground; the hip is
# inpainted as the thigh runs down to it from the crouch's hip.
s = kick_source(dy=4, reach=6)
s.paint(28, 48, 58, 76, keep=lambda x, y: y < 52 and s.c.at(x, y) != r.T and x < 40)
s.paint(30, 76, 56, 88)
s.save(RND, 'sweep')

# High kick, his second standing kick: his kick's body (0x27a8) standing on its rear leg, with the kicking leg
# swung up from the hip to head height. The leg is his kick's own, turned 40 degrees (rot.py) and laid as a guide;
# the mask covers the turned leg and the hip it rises from. His ground shadow is his chamber's (0x2726).
import rot
K = r.grid(0x27a8, 1, 3, 3)
s = Src(128, 128)
body = K.copy()
for y in range(56, 90):
    for x in range(34, 128): body.p[y][x] = r.T
for y in (126, 127):
    for x in range(128): body.p[y][x] = r.T
s.lay(body, 0, 0)
s.lay(r.grid(0x2726, 1, 3, 1).crop(0, 126, 64, 2), 0, 126)
leg = K.crop(30, 63, 70, 26)
turned = rot.turn(leg, 40, (6, 9), 128, 128, (36, 70))
s.lay(turned, 0, 0)
near = {(x + dx, y + dy) for y in range(128) for x in range(128) if turned.p[y][x] != r.T for dx in (-2, -1, 0, 1, 2) for dy in (-2, -1, 0, 1, 2)}
s.mask |= {(x, y) for x, y in near if 0 <= x < 128 and 0 <= y < 128}
s.paint(24, 54, 50, 80, keep=lambda x, y: x < 30 and y > 66)
s.save(RND, 'high-kick')
