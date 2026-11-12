"""Touch up round one's kept candidates (AP23, #187). usage: touch_ups.py <touched folder>

Outside each mask every pixel is the source's (his ROM pixels); inside it, inpainting's colours go to his nearest
pens and single pixels unlike all four neighbours take the pens around them. Fixes made by hand are listed.
Writes <touched>/<pose>.png and <touched>/touch-ups.json."""
import json, sys
from PIL import Image
import rompose as r, clean as k, eagle as e, fix_rs
OUT = sys.argv[1]
TU = {}
def mask(name):
    m = Image.open(f"../r1/inpaint-mask-{name}.png").convert('RGB')
    return lambda x, y: 0 <= x < m.width and 0 <= y < m.height and m.getpixel((x, y)) == (255, 255, 255)
BASE = "inpainting's colours reduced to his nearest pens"
def lone(c, inside):
    n = k.lone(c, region=inside)
    return f"{BASE}; {n} single pixels inside the mask unlike all four neighbours set to the pens around them" if n else BASE

C, notes = e.crouch(); C.save(f"{OUT}/crouch.png"); TU['crouch'] = notes
e.on_crouch(r.grid(0x2aa0, 1, 2, 1), 3, -3, 49).save(f"{OUT}/crouching_block.png")
e.on_crouch(r.grid(0x2aa8, 1, 2, 1), 2, -2, 48).save(f"{OUT}/crouching_hit.png")

def crouch_waist(c, top):
    """His waistband's right end and the front thigh's top (rows 44-47, from x33) as the approved crouch has them."""
    n = 0
    for y in range(44, 48):
        for x in range(33, e.W):
            if c.p[y + top][x] != C.p[y][x]: c.p[y + top][x] = C.p[y][x]; n += 1
    return f"{n} pixels where the waist mask met the front thigh's top (rows 44-47 of the crouch, from x33) set back to the approved crouch's: a stray light pixel and specks the inpainting drew in the waistband's two holes"

for pose, job, name, top in (('crouching_sweep', '7fe080a7', 'crouching-sweep', 0), ('crouching_uppercut', '99ca28e8', 'crouching-uppercut', 44)):
    c = r.load(f"../{job}-0.png"); inside = mask(name)
    TU[pose] = [lone(c, inside), crouch_waist(c, top)]
    c.save(f"{OUT}/{pose}.png")

c = r.load("../c3e4604b-0.png"); inside = mask('rising-strike')
TU['rising_strike'] = [lone(c, inside)] + fix_rs.fix(c)
c.save(f"{OUT}/rising_strike.png")

c = r.load("../285597a3-0.png"); inside = mask('smash')
n = 0
for y in (46, 47):                       # holes the inpainting kept where his stance's hands were taken off his belt
    row = [x for x in range(c.w) if c.p[y][x] != r.T and x < 42]
    for x in range(min(row), max(row) + 1):
        if c.p[y][x] == r.T: c.p[y][x] = 0; n += 1
TU['smash'] = [lone(c, inside), f"{n} transparent holes in his black waistband (rows 46-47), left where his stance's hands were taken off the belt and kept by the inpainting, filled black"]
c.save(f"{OUT}/smash.png")

c = r.load("../9190a924-0.png"); inside = mask('rising')
TU['rising'] = [lone(c, inside)]
c.save(f"{OUT}/rising.png")
json.dump(TU, open(f"{OUT}/touch-ups.json", 'w'), indent=1)
print(json.dumps(TU, indent=1))
