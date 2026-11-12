"""Adon's air set and specials (AP26, #190), round one: the free poses and the inpainting sources.

usage: ap26_make_r1.py <round folder> <touched folder>

Stance coordinates throughout (ap26_air.py): the top of his jump (0x3248) sits 3 right and 2 down in them.
- touched/jump_punch.png: his jab's upper body (0x30c4, rows 0-42, 3 pixels right, as on his crouching punch) on the
  tuck's shorts and legs (its rows from 39). Every pixel his ROM art; the recipe builds it from these two parts.
- touched/jump_kick.png: the tuck with the approved crouching kick's leg and hip join (pack f03c8989), whose shorts are
  the tuck's at the same place, in place of the tuck's front thigh; the tuck's folded front calf, wrap and foot stay
  under it as the tucked rear leg, and the tuck's second wrap and foot (the rear leg's) are taken away.
- round folder: sources and masks (white = inpaint) for the diagonal jumping punch, the heavy jumping kick and the
  Rising Jaguar's knee.
"""
import os, sys
import rompose as r, rot, ap26_air as a, ap26_knee as kn
from src import Src
RND, TOUCHED = sys.argv[1], sys.argv[2]
os.makedirs(RND, exist_ok=True); os.makedirs(TOUCHED, exist_ok=True)
PACK = r"../../../adon"

# Jumping punch.
PUNCH = a.on_tuck(0x30c4, 2, 2, dx=3)
PUNCH.save(f"{TOUCHED}/jump_punch.png")

# Jumping kick.
def jump_kick():
    k = r.load(f"{PACK}/crouching_kick.png")            # k(x - 2, y - 1) is stance (x, y)
    B = a.tuck()
    for y in range(40, 52):                            # the tuck's front thigh
        for x in range(24, 50):
            v = a.TUCK.p[y][x]
            if v != r.T and not (v in (0, 5, 6, 7) and x < 27): B.p[y + 2][x + 3] = r.T
    for y in range(61, 77):                            # the rear leg's wrap and foot, behind
        for x in range(4, 22):
            if y >= 68 or x <= 17: B.p[y + 2][x + 3] = r.T
    for ky in range(40, k.h):
        for kx in range(24, k.w):
            v = k.p[ky][kx]
            if v == r.T or (kx < 30 and v in (0, 5, 6, 7)) or (ky >= 56 and kx < 34): continue
            B.p[ky + 1][kx + 2] = v
    return B
jump_kick().save(f"{TOUCHED}/jump_kick.png")

# Diagonal jumping punch: the punch's body with the jab's arm taken away beyond the shoulder (x >= 41, rows 10-33),
# and the front arm of his flying kick's dive (0x330a: upper arm under his jaw to the fist, x >= 62 from row 22,
# x >= 68 from row 19) laid 24 left and 4 up, so it leaves the jab's shoulder and drives down and forward to the
# fist before his knee. The mask covers the shoulder join only: the jab's shoulder cap and the dive arm's root
# (x38-49, rows 13-31), sparing his head and the chest's front edge below row 24 left of x41.
s = Src(72, 80)
B = PUNCH.copy()
for y in range(10, 34):
    for x in range(41, 96):
        if not (y < 14 and x < 44): B.p[y][x] = r.T
D = r.grid(0x330a, 1, 1, 2)
ARM = r.P(96, 96)
for y in range(D.h):
    for x in range(D.w):
        v = D.p[y][x]
        if v != r.T and ((x >= 62 and y >= 22) or (x >= 68 and y >= 19)): ARM.p[y - 4][x - 24] = v
s.lay(B, 0, 0); s.lay(ARM, 0, 0)
s.paint(38, 13, 50, 32, keep=lambda x, y: (y < 15 and x < 42) or (x < 41 and y >= 24))
s.save(RND, 'jump-punch-down')

# Heavy jumping kick: his roundhouse (0x318a, stream 0x13850), the kicking leg swung up to head height, lifted into
# the air: its standing leg (from the bottom of his shorts, row 54) turned 45 degrees back about the hip so it trails
# down behind him, and the ground shadow left out. The mask covers the hip and the trailing thigh's root under the
# shorts (x16-42, rows 52-70), sparing the shorts above row 55 and the kicking thigh.
H = r.grid(0x318a, 1, 2, 2)
SL, HB = r.P(96, 96), H.copy()
for y in range(54, 96):
    for x in range(14, 44):
        v = H.p[y][x]
        if v != r.T and not (v in (0, 7) and y < 56): HB.p[y][x] = r.T; SL.p[y][x] = v if y < 94 else r.T
for y in range(93, 96):
    for x in range(96): HB.p[y][x] = r.T
s = Src(96, 96)
s.lay(rot.turn(SL, -45, (29, 55), 96, 96, (29, 55)), 0, 0); s.lay(HB, 0, 0)
s.paint(16, 52, 42, 71, keep=lambda x, y: (y < 55 and HB.at(x, y) in (0, 5, 6, 7)) or x >= 38 and y < 60)
s.save(RND, 'jump-kick-high')

# Rising Jaguar's knee: his rising pose (0x3244: fists by his face, the front knee up, the rear leg hanging) with the
# front thigh turned 60 degrees up about the hip, and the folded calf, wrap and foot turned 40 about the knee so they
# hang under it. The rear calf's edge the old wrap covered gets his outline by hand. The mask covers the hip, the
# thigh and the knee (x27-47, rows 32-50), sparing his shorts and the torso above row 33.
K, BODY, KNEE = kn.build(60, 40)
for y in range(59, 76):
    xs = [x for x in range(8, 30) if K.p[y][x] != r.T]
    if xs and K.p[y][max(xs)] not in (1,): K.p[y][max(xs)] = 1
s = Src(64, 96)
s.lay(K, 0, 0)
s.paint(27, 32, 48, 50, keep=lambda x, y: y < 33 or (x < 31 and kn.R.at(x, y) in (0, 5, 6, 7) and K.at(x, y) in (0, 5, 6, 7)))
s.save(RND, 'rj-knee')
