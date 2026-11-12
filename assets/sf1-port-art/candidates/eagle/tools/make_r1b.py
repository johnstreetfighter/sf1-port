"""Round one, the poses built on the kept crouch (AP23, #187). usage: make_r1b.py <round folder> <touched folder>

- touched/: the crouch (its touch-ups), and the crouching block and crouching hit, his standing block's (0x2aa0)
  and standing hit's (0x2aa8) upper bodies down to their belts on the crouch's legs; nothing generated.
- round folder: inpainting sources and masks for the crouching sweep and uppercut (his stick jab's upper body on
  the crouch's legs, its arm turned about the shoulder) and the get-up's rising pose.
"""
import json, os, sys
import rompose as r, eagle as e
from src import Src
RND, TOUCHED = sys.argv[1], sys.argv[2]
os.makedirs(RND, exist_ok=True); os.makedirs(TOUCHED, exist_ok=True)
ORIGINS = json.load(open(f"{RND}/origins.json")) if os.path.exists(f"{RND}/origins.json") else {}
SKIN = (1, 2, 3, 4, 5)
C, notes = e.crouch()
C.save(f"{TOUCHED}/crouch.png")
e.on_crouch(r.grid(0x2aa0, 1, 2, 1), 3, -3, 49).save(f"{TOUCHED}/crouching_block.png")
e.on_crouch(r.grid(0x2aa8, 1, 2, 1), 2, -2, 48).save(f"{TOUCHED}/crouching_hit.png")
json.dump({"crouch": notes}, open(f"{TOUCHED}/touch-ups.json", 'w'), indent=1)

def near(s, x, y): return any(s.c.at(x + d, y) != r.T for d in (-1, 0, 1))
def shoulder(s, top, rad=7, torso_x=e.SHOULDER[0] + e.JAB_DX - 3):
    sx, sy = e.SHOULDER[0] + e.JAB_DX, e.SHOULDER[1] + e.JAB_DY - 2 + top
    return {(x, y) for y in range(sy - rad, sy + rad + 1) for x in range(torso_x, sx + rad + 1)
            if (x - sx) ** 2 + (y - sy) ** 2 <= rad * rad}
def waist(s, top, y0=43, y1=47):
    return {(x, y + top) for y in range(y0, y1) for x in range(8, 46) if near(s, x, y + top) and s.c.at(x, y + top) not in SKIN}

for name, deg, w, h, top in (('crouching-sweep', -50, 96, 76, 0), ('crouching-uppercut', 60, 72, 120, 44)):
    s = Src(w, h); s.lay(e.jab_on_crouch(deg, w, h, top), 0, 0)
    s.mask = shoulder(s, top) | waist(s, top)
    s.mask = {(x, y) for x, y in s.mask if 0 <= x < w and 0 <= y < h}
    s.save(RND, name); ORIGINS[name] = [32, 73 + top]

# The get-up's rising pose: his jump take-off squat's upper body (0x29e8), bent forward with both hands low and a
# stick in each, set 8 right and 16 lower than the crouch's upper body, so his hands come down on his knees; only
# the waist band where its shirt and belt meet the crouch's thighs is inpainted, sparing the hands and sticks.
SQ = r.grid(0x29e8, 1, 2, 1)
keep = lambda x, y, v: y < 59 or (v in SKIN + (0,) and (x < 22 or x > 36))
s = Src(72, 76); s.lay(e.on_crouch(SQ, 8, 0, 62, keep=keep), 0, 0)
s.mask = {(x, y) for y in range(52, 60) for x in range(10, 50) if near(s, x, y) and s.c.at(x, y) not in SKIN
          and not (x < 30 and y < 55)}
s.save(RND, 'rising'); ORIGINS['rising'] = [32, 73]
json.dump(ORIGINS, open(f"{RND}/origins.json", 'w'), indent=1)
