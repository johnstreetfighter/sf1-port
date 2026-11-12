"""Round two's sources and masks (AP23, #187): the crouching sweep and the rising strike. usage: make_r2.py <round>"""
import json, os, sys
import rompose as r, eagle as e
from src import Src
RND = sys.argv[1]; os.makedirs(RND, exist_ok=True)
ORIGINS = {}

# Rising strike: the jab's body (its torso's front edge under the arm outlined) and its full upper arm turned 35
# degrees up, set 3 pixels back into the torso; the mask covers the whole shoulder and upper arm.
TOP = 32
s = Src(104, 128)
s.lay(e.jab_body(True), 0, TOP)
arm = e.turned_arm_full(35)
s.lay(arm, 0, TOP - e.UP)
s.mask = {(x, y) for x, y in e.arm_mask(arm, 0, TOP - e.UP) if not (x <= 58 and y <= 26 + TOP)}   # never his head
s.save(RND, 'rising-strike'); ORIGINS['rising-strike'] = [48, TOP + 96]

# Crouching sweep: the jab's upper body on the crouch's legs, its full upper arm turned 50 degrees down, set back
# 3 pixels; the mask covers the whole shoulder and upper arm, and the waist band as round one's did.
C, _ = e.crouch()
s = Src(96, 76)
s.lay(e.on_crouch(e.jab_body(True), e.JAB_DX, e.JAB_DY, 51, w=96, h=76), 0, 0)
arm = e.turned_arm_full(-50)
dx, dy = e.JAB_DX, -e.UP + e.JAB_DY - 2
s.lay(arm, dx, dy)
s.mask = {(x, y) for x, y in e.arm_mask(arm, dx, dy) if not (x <= 58 + e.JAB_DX and y <= 26 + e.JAB_DY - 2)}   # never his head
near = lambda x, y: any(s.c.at(x + d, y) != r.T for d in (-1, 0, 1))
s.mask |= {(x, y) for y in range(43, 47) for x in range(8, 46) if near(x, y) and s.c.at(x, y) not in (1, 2, 3, 4, 5)}
s.mask = {(x, y) for x, y in s.mask if 0 <= x < s.c.w and 0 <= y < s.c.h}
s.save(RND, 'crouching-sweep'); ORIGINS['crouching-sweep'] = [32, 73]
json.dump(ORIGINS, open(f"{RND}/origins.json", 'w'), indent=1)
