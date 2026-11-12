"""Birdie's air set (AP22, #186), round one: the jumping attacks built on his top-of-jump tuck (0x240c).

usage: make_air.py <round folder> <touched folder>

- touched/: the jumping kick, the tuck with his approved crouching kick's leg and hip join (AP21) laid where they lie
  on the crouch, whose legs are the tuck's; nothing generated. (The jumping punch, his jab's upper body on the tuck's
  legs, is all ROM pixels and is rebuilt by its recipe parts.)
- round folder: inpainting sources and masks (white = inpaint) for the heavy and diagonal jumping kicks: the same
  approved leg turned about the hip (rot.py) and laid as a guide, the mask over the turned leg and the hip it comes
  from, as for his approved high kick (AP21).
"""
import os, sys
import rompose as r, air, rot
from src import Src
RND, TOUCHED = sys.argv[1], sys.argv[2]
os.makedirs(RND, exist_ok=True); os.makedirs(TOUCHED, exist_ok=True)

base = air.body_without_front_leg()
jk = base.copy(); jk.paste(air.crouching_kick_leg(), 0, 0)
jk.save(f"{TOUCHED}/jump_kick.png")

k = r.load(air.CKICK)
LEG = k.crop(40, 54, 68, 20)                       # thigh to boot, without the crouch's belly above it
HIP = (40, 63)

def turned_kick(name, deg, w, h):
    s = Src(w, h)
    s.lay(base, 0, 0)
    t = rot.turn(LEG, deg, (0, 9), w, h, HIP)
    s.lay(t, 0, 0)
    near = {(x + dx, y + dy) for y in range(h) for x in range(w) if t.p[y][x] != r.T
            for dx in (-2, -1, 0, 1, 2) for dy in (-2, -1, 0, 1, 2)}
    s.mask |= {(x, y) for x, y in near if 0 <= x < w and 0 <= y < h}
    return s

# Heavy jumping kick: the leg swung up 40 degrees from the hip, the boot at head height, as his high kick swings it;
# the hip, and the bottom of the vest's front panel the thigh crosses, are in the mask.
s = turned_kick('heavy', 40, 112, 96)
s.paint(30, 44, 52, 72, keep=lambda x, y: x < 38 and y > 50)
s.save(RND, 'jump-kick-high')

# Diagonal jumping kick: the leg driven down and forward 35 degrees from the hip, the boot below his rear foot.
s = turned_kick('diagonal', -35, 112, 108)
s.paint(30, 50, 52, 76, keep=lambda x, y: x < 38 and y > 52)
s.save(RND, 'jump-kick-down')

# Bull Horn's strike (AP2: he lowers his head, puts both fists forward like horns and drives in at waist height):
# the landing of his diving attack (0x271a), crouched low and leaning in with his near arm driven straight forward
# at waist height, with its upright head taken away and his headbutt lunge's lowered head (0x28a0, crown forward,
# face down) set low between his shoulders, behind the near arm; the same forearm and fist laid again 9 pixels
# higher, behind it, as his far arm, so both fists point forward. Only the neck between the shoulders and the head,
# and the far forearm's root under his chin, are inpainted. Rows 16-111 of the pose (96 high, so 5 generations).
LAND = r.grid(0x271a, 1, 3, 2)
LUNGE = r.grid(0x28a0, 1, 2, 2)
horn = LAND.copy()
for y in range(18, 47):
    for x in range(36, 64):
        if (y < 40 or x >= 40) and not (y >= 40 and x < 44 and horn.p[y][x] != r.T):
            horn.p[y][x] = r.T
HEAD = LUNGE.crop(79, 6, 17, 22)
HX, HY = 49, 30
head = set()
for y in range(HEAD.h):
    for x in range(HEAD.w):
        v = HEAD.p[y][x]
        if v != r.T and horn.at(HX + x, HY + y) == r.T:
            horn.p[HY + y][HX + x] = v; head.add((HX + x, HY + y - 16))
FAR = (2, -9)                                      # the far arm, behind the head and the near arm
fore = LAND.crop(56, 50, 40, 30)
for y in range(fore.h):
    for x in range(fore.w):
        v = fore.p[y][x]
        if v != r.T and horn.at(56 + FAR[0] + x, 50 + FAR[1] + y) == r.T:
            horn.p[50 + FAR[1] + y][56 + FAR[0] + x] = v
s = Src(96, 96)
s.lay(horn, 0, -16)
arm = lambda x, y: y + 16 >= 46 and LAND.at(x, y + 16) != r.T and x < 50
s.paint(38, 19, 51, 33, keep=lambda x, y: (x, y) in head or arm(x, y))
s.paint(58, 30, 70, 40, keep=lambda x, y: (x, y) in head or (LAND.at(x, y + 16) != r.T and y + 16 >= 50))
s.save(RND, 'bull-horn')
