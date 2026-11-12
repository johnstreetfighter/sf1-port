"""Touch up round two's kept candidates (AP23, #187). usage: touch_ups_r2.py <touched folder>
As touch_ups.py: inpainting's colours to his nearest pens, single odd pixels inside the mask to the pens around
them; the sweep's waist where it meets the front thigh set back to the approved crouch's; the smash by hand."""
import json, sys
from PIL import Image
import rompose as r, clean as k, eagle as e, fix_smash_r2
OUT = sys.argv[1]
TU = {}
C, _ = e.crouch()
def mask(name):
    m = Image.open(f"../r2/inpaint-mask-{name}.png").convert('RGB')
    return lambda x, y: 0 <= x < m.width and 0 <= y < m.height and m.getpixel((x, y)) == (255, 255, 255)
BASE = "inpainting's colours reduced to his nearest pens"
def lone(c, inside):
    n = k.lone(c, region=inside)
    return f"{BASE}; {n} single pixels inside the mask unlike all four neighbours set to the pens around them" if n else BASE

c = r.load("../a0edde65-0.png"); inside = mask('crouching-sweep')
TU['crouching_sweep'] = [lone(c, inside)]
n = 0
for y in range(44, 48):
    for x in range(33, e.W):
        if c.p[y][x] != C.p[y][x]: c.p[y][x] = C.p[y][x]; n += 1
if n: TU['crouching_sweep'].append(f"{n} pixels where the waist mask met the front thigh's top (rows 44-47, from x33) set back to the approved crouch's")
c.save(f"{OUT}/crouching_sweep.png")

c = r.load("../09fb9b9b-0.png"); inside = mask('rising-strike')
TU['rising_strike'] = [lone(c, inside)]
c.save(f"{OUT}/rising_strike.png")

c = r.load("../touched/smash.png")
n, gap = fix_smash_r2.fix(c)
TU['smash'] = [f"round one's file (65ea49af) with the gap between his forearms opened (the maintainer, round one: \"between the forearms should look transparent\"): "
               f"{n} pixels of the maroon band where the two forearms' outlines met (x53-66, y40-51) made transparent, keeping one outline pixel on each forearm"]
c.save(f"{OUT}/smash.png")
json.dump(TU, open(f"{OUT}/touch-ups.json", 'w'), indent=1)
print(json.dumps(TU, indent=1))
