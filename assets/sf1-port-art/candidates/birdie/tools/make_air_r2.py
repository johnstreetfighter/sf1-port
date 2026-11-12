"""Birdie's air set and Bull Horn (AP22, #186), round two: the two poses the maintainer rejected in round one.

usage: make_air_r2.py <round folder>

- jump_kick_high (the maintainer: "redo his whole lower half, the non-kicking leg should probably be at a different angle to
  look more animated", "Leg looks pasted on", "Too like the jump kick"): his approved standing high kick (AP21: his
  kick's strike, 0x27a8, its leg swung up from an inpainted hip), lifted off the floor, with its standing leg turned
  26 degrees back about the hip (rot.py) so it trails down and back, and the floor shadow gone. Everything from the
  belt down is inpainted around the guides: the hip, the seat and both upper thighs, and the whole trailing leg; the
  kicking shin and boot stay the approved high kick's. His ROM upper body and head (0x27a8's) are untouched.
- bull_horn (the maintainer: "Looks like 4 arms and the head seems malformed", "Head hidden or lost", "Arms look wrong"): his
  dive landing (0x271a) keeps its one real arm, driven forward at waist height; the far arm is only its fist, the
  near fist laid again 2 pixels forward and 6 higher, behind it, so only the top of the far fist and wristband
  peek past the near one. The headbutt lunge's lowered head (0x28a0, every pixel) is drawn in front of everything,
  resting on the near arm's top line, crown forward and face down. Only the neck between his shoulders and the back
  of his head is inpainted.
"""
import os, sys
import rompose as r, rot
from src import Src
RND = sys.argv[1]
os.makedirs(RND, exist_ok=True)

# Heavy jumping kick.
HK = r.load(r"../touched/high_kick.png")           # the approved high kick, 0x27a8's 128x128 canvas
stand = lambda x, y: y >= 62 and x < 40 and (x < 24 or y >= 82 - 0.5 * (x - 24))
leg = r.P(128, 128); body = HK.copy()
for y in range(128):
    for x in range(128):
        if (stand(x, y) or y >= 126) and HK.p[y][x] != r.T:
            if y < 126: leg.p[y][x] = HK.p[y][x]
            body.p[y][x] = r.T
M2 = 20
W, H = 148, 128
s = Src(W, H)
s.lay(body, M2, 0)
t = rot.turn(leg.crop(0, 60, 40, 66), -26, (24, 6), W, H, (24 + M2, 66))
s.lay(t, 0, 0, keep=lambda x, y, v: s.c.p[y][x] == r.T)
near = {(x + dx, y + dy) for y in range(H) for x in range(W) if t.p[y][x] != r.T
        for dx in (-2, -1, 0, 1, 2) for dy in (-2, -1, 0, 1, 2)}
s.mask |= {(x, y) for x, y in near if 0 <= x < W and 0 <= y < H}
s.paint(M2 + 4, 61, M2 + 56, 94)                    # from the belt down: hip, seat, both upper thighs
s.save(RND, 'jump-kick-high')

# Bull Horn.
LAND = r.grid(0x271a, 1, 3, 2)
LUNGE = r.grid(0x28a0, 1, 2, 2)
horn = LAND.copy()
for y in range(18, 47):
    for x in range(36, 64):
        if (y < 40 or x >= 40) and not (y >= 40 and x < 44 and horn.p[y][x] != r.T):
            horn.p[y][x] = r.T
fist = LAND.crop(74, 62, 20, 18)                    # the near fist and the end of its wristband
for y in range(fist.h):
    for x in range(fist.w):
        v = fist.p[y][x]
        if v != r.T and horn.at(76 + x, 56 + y) == r.T:
            horn.p[56 + y][76 + x] = v
HEAD = LUNGE.crop(79, 6, 17, 22)
HX, HY = 48, 25
horn.paste(HEAD, HX, HY)
head = {(HX + x, HY + y - 16) for y in range(HEAD.h) for x in range(HEAD.w) if HEAD.p[y][x] != r.T}
s = Src(96, 96)
s.lay(horn, 0, -16)
arm = lambda x, y: y + 16 >= 46 and LAND.at(x, y + 16) != r.T
s.paint(37, 14, 50, 32, keep=lambda x, y: (x, y) in head or arm(x, y))
s.save(RND, 'bull-horn')
