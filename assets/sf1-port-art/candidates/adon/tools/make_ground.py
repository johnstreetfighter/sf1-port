"""Adon's ground set (AP25, #189), round one: the poses built on his crouch.

usage: make_ground.py <round folder> <touched folder>

- touched/: the crouch (his stance's upper body on the kept crouch inpainting's legs, touched up), the crouching
  punch (his jab's upper body, 0x30c4, whose belt and shorts are his stance's 3 pixels right, so no join) and the
  crouching hit (his standing hit's upper body, 0x3180, on the crouch's belt and legs).
- round folder: inpainting sources and masks (white = inpaint) for the crouching block, crouching kick and
  sweep, each his ROM pixels with only the joins masked.
"""
import os, sys
import rompose as r, adon, rot
from src import Src
RND, TOUCHED = sys.argv[1], sys.argv[2]
os.makedirs(RND, exist_ok=True); os.makedirs(TOUCHED, exist_ok=True)
C = adon.crouch(1)
C.save(f"{TOUCHED}/crouch.png")
adon.upper_on_legs(0x30c4, 2, 2, dx=3, upto=43, width=96, rnd=1).save(f"{TOUCHED}/crouching_punch.png")
HIT = adon.upper_on_legs(0x3180, 2, 1, dx=-1, dy=-2, upto=47, keep=lambda x, y: y < 40 or x >= 36, legs_from=40, rnd=1)
HIT.save(f"{TOUCHED}/crouching_hit.png")

# Crouching block: his standing block (0x312c, move 4) from the head to above its belt, the front fist raised above
# his head and the rear fist at his chin, laid on the crouch's belt and legs 5 pixels lower (its lifted knee left
# out). Only the waist band where its narrower, turned waist meets the crouch's belt is inpainted.
s = Src(64, 72)
s.lay(adon.upper_on_legs(0x312c, 2, 1, dx=-1, dy=-5, upto=40, keep=lambda x, y: not (x >= 34 and y >= 33), legs_from=40, rnd=1), 0, 0)
s.paint(6, 34, 34, 41, keep=lambda x, y: s.c.at(x, y) == r.T and (x < 9 or y < 37))
s.save(RND, 'crouching-block')
# Its inpainting changed one pixel of 166, so the block is kept as pieced: the same upper body on the crouch.
adon.upper_on_legs(0x312c, 2, 1, dx=-1, dy=-5, upto=40, keep=lambda x, y: not (x >= 34 and y >= 33), legs_from=40, rnd=1).save(f"{TOUCHED}/crouching_block.png")

# The kicks: the leg of his low kick (0x31e4, stream 0x133fc), thigh to foot (x >= 45, rows 51-89, without his
# shorts' pens), turned up about its hip and laid from the crouch's front hip in place of the crouch's front thigh,
# knee, ankle wrap and foot; the crouch's shorts drawn back over the thigh's top. He sits on his folded rear leg.
# The mask covers the hip and the turned thigh (its outline stepped by the turn), sparing the shorts.
K = r.grid(0x31e4, 1, 2, 2)
LEG = r.P(96, 96)
for y in range(51, 92):
    for x in range(45, 96):
        v = K.p[y][x]
        if v != r.T and not (y < 62 and v in (0, 5, 6, 7)): LEG.p[y][x] = v
HIP = (52, 58)                                   # the thigh's root under his shorts, in the kick

def kick_source(deg, at):
    s = Src(96, 72)
    base = C.copy()
    for y in range(42, 72):
        for x in range(30, 64):
            if y >= 53 or x >= 31: base.p[y][x] = r.T     # front thigh, knee, wrap, foot and their shadow
    for y in (69, 70):                                    # his ground shadow, under the rear foot only
        for x in range(28, 64): base.p[y][x] = r.T
    s.lay(rot.turn(LEG, deg, HIP, 96, 72, at), 0, 0)
    s.lay(base, 0, 0)
    return s

s = kick_source(27, (31, 49))
s.paint(22, 43, 52, 60, keep=lambda x, y: (x < 31 and s.c.at(x, y) in (0, 5, 6, 7) and y < 53))
s.save(RND, 'crouching-kick')

s = kick_source(17, (31, 51))
s.paint(22, 43, 54, 66, keep=lambda x, y: (x < 31 and s.c.at(x, y) in (0, 5, 6, 7) and y < 53))
s.save(RND, 'sweep')
