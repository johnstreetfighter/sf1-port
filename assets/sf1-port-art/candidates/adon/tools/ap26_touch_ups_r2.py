"""Touch up Adon's round-two jumping kick (AP26, #190): the kept candidate (seed 2026) checked against its source,
taken to his pens, specks removed, and the fixes below. Writes <out>/jump_kick.png and adds to touch-ups.json.

usage: ap26_touch_ups_r2.py <candidates/adon folder> <out folder>
"""
import glob, json, os, sys
from PIL import Image
import rompose as r, clean as k
C, OUT = sys.argv[1], sys.argv[2]
os.makedirs(OUT, exist_ok=True)
JOB, NAME = "03dd2aa6", "jump-kick"
src = r.load(glob.glob(f"{C}/ap26/r2/inpaint-source-{NAME}-*.png")[0])
m = Image.open(f"{C}/ap26/r2/inpaint-mask-{NAME}.png").convert('RGB')
inside = lambda x, y: m.getpixel((x, y)) == (255, 255, 255)
off = k.offpal(f"{C}/{JOB}-0.png")
c = r.load(f"{C}/{JOB}-0.png")
bad = [(x, y) for y in range(c.h) for x in range(c.w) if not inside(x, y) and c.p[y][x] != src.p[y][x]]
assert not bad, bad[:5]
drawn = sum(1 for y in range(c.h) for x in range(c.w) if inside(x, y) and c.p[y][x] != src.p[y][x])
isl = k.islands(c)
for x, y in ((25, 59), (26, 59), (24, 60), (24, 61)): c.p[y][x] = 1
for x, y in ((25, 60), (26, 60), (25, 61), (26, 61), (25, 62)): c.p[y][x] = r.T
for x, y in ((27, 59), (28, 59), (29, 59), (30, 59), (31, 59), (32, 59), (26, 56)): c.p[y][x] = 1
c.p[42][28] = 0
c.save(f"{OUT}/jump_kick.png")
path = f"{OUT}/touch-ups.json"
report = json.load(open(path)) if os.path.exists(path) else {}
report["jump_kick"] = [
    f"outside the mask every pixel is the source's (his ROM upper body, shorts, rear wrap and foot, and the crouching kick's shin and foot); inside, {drawn} pixels differ from the source, and the {off} pixels the inpainting drew off his palette were taken to his nearest pens",
    f"{isl} stray pixels removed (a speck right of the belt's end)",
    "the rear knee's front edge: four black pixels given his legs' dark brown outline (pen 1) and five skin pixels standing out past it removed (x24-26, rows 59-62)",
    "the rear knee's underside, where it shows below the kicking thigh, closed with his dark brown outline (x27-32, row 59), and a grey speck in it (26, 56) given the same",
    "a pale pixel at the shorts' front corner beside the belt (28, 42) given the shorts' black"]
with open(path, 'w', newline='\n') as f:
    json.dump(report, f, indent=1)
print(json.dumps(report["jump_kick"], indent=1))
