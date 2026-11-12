"""Round one's inpainting sources and masks (AP23, #187). usage: make_r1.py <round folder>

Each source records `origin`: the canvas column of his centre and the canvas row of the ground line, so a packed
pose's anchor is [x0 - cx, y0 - gy] for its cropped top-left (x0, y0)."""
import json, os, sys
import rompose as r, eagle as e
from src import Src
RND = sys.argv[1]; os.makedirs(RND, exist_ok=True)
ORIGINS = {}

def dilate(points, n=1):
    return {(x + dx, y + dy) for x, y in points for dx in range(-n, n + 1) for dy in range(-n, n + 1)}

def save(s, name, origin):
    s.mask = {(x, y) for x, y in s.mask if 0 <= x < s.c.w and 0 <= y < s.c.h}
    s.save(RND, name); ORIGINS[name] = origin

def crouch_canvas(top=0, right=0):
    """A source on the crouch's canvas, `top` rows higher and `right` columns wider."""
    return Src(e.W + right, e.H + top)

# --- The crouch: only the shins' lower ends, where the tuck's legs meet his stance's hems, are inpainted.
s = crouch_canvas(); s.lay(e.crouch_source(), 0, 0)
near = lambda x, y: any(s.c.at(x + d, y) != r.T for d in (-1, 0, 1))
s.mask = {(x, y) for y in range(56, 67) for x in range(36, 62) if near(x, y)}          # the front knee onto its shin
s.mask |= {(x, y) for y in range(60, 66) for x in range(10, 32) if near(x, y)}         # the rear leg onto its hem
save(s, 'crouch-b', (32, 73))

# --- Rising stick strike (his standing MK): his stick jab's strike (0x2980) with its striking arm, sleeve, forearm,
# fist and stick, turned 35 degrees up about the shoulder; only the shoulder's join is inpainted, off the torso.
def jab_turned(deg, w, h, top):
    """The jab's body, and its arm turned by deg, `top` rows down a w x h canvas."""
    s = Src(w, h)
    s.lay(e.jab_body(False), 0, top)
    s.lay(e.turned_arm(deg, False), 0, top - e.UP)
    return s
def shoulder_mask(s, top, deg, rad=7, torso_x=60):
    """The shoulder's join: within rad of the shoulder joint, right of the torso's front edge, and the gap the
    turned sleeve leaves against the torso."""
    sx, sy = e.SHOULDER[0], e.SHOULDER[1] + top
    s.mask |= {(x, y) for y in range(sy - rad, sy + rad + 1) for x in range(torso_x, sx + rad + 1)
               if (x - sx) ** 2 + (y - sy) ** 2 <= rad * rad}
TOP = 32
s = jab_turned(35, 104, 128, TOP)
shoulder_mask(s, TOP, 35)
save(s, 'rising-strike', (48, TOP + 96))

# --- Two-stick overhead smash (his standing HK): the upper body of his jumping stick strike (0x2a44), hunched
# forward with both arms driven down before him and both sticks pointing down at the opponent's knees, on his
# stance's legs. 0x2a44 is the top of his jump (0x2a40) with the arms forward, so its waist is the tuck's, which
# the crouch puts on his stance's belt: 7 right and 10 down. Its arms and sticks below the waist are laid too.
SM = r.grid(0x2a44, 1, 2, 2)
s = Src(96, 96)
s.lay(e.S, 0, 0, keep=lambda x, y, v: y >= 46 and not (y < 50 and v in (1, 2, 3, 4, 5)))   # his legs, without his hands' last rows
arm = lambda x, y, v: y < 36 or (x >= 34 and v in (1, 2, 3, 4, 5)) or (x >= 50 and y < 60 and v in (0, 8, 11, 14))
s.lay(SM, 7, 10, keep=arm)
s.mask = {(x, y) for y in range(42, 50) for x in range(10, 46) if not (x >= 30 and s.c.at(x, y) in (1, 2, 3, 4, 5))
          and any(s.c.at(x + d, y) != r.T for d in (-1, 0, 1))}
save(s, 'smash', (32, 96))
json.dump(ORIGINS, open(f"{RND}/origins.json", 'w'), indent=1)
