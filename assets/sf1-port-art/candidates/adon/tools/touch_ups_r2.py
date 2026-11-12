"""Touch up Adon's round-two inpainted candidates (AP25, #189).

usage: touch_ups_r2.py <candidates/adon folder> <out folder>

As touch_ups.py: outside the mask every pixel must be the source's; inside, the inpainting's colours are taken to
his nearest pens, opaque specks under 4 pixels are removed, and the fixes listed per pose are made. Writes
<out>/<pose>.png and <out>/touch-ups.json. The crouch, crouching block, hit and punch are written by
make_round2b.py.
"""
import glob, json, os, sys
from PIL import Image
import rompose as r, clean as k, adon
C, OUT = sys.argv[1], sys.argv[2]
os.makedirs(OUT, exist_ok=True)


def sweep_fix(c, inside):
    # The belt: the inpainting redrew the lowered belt's studs in his bright yellow and purple; his crouch's own
    # belt rows (40-42, the stance's), lowered 7 as the kneel's shorts are, put back.
    B = adon.crouch(2)
    n = 0
    for y in range(40, 43):
        for x in range(10, 34):
            v = B.p[y][x]
            if v != r.T and c.p[y + 7][x] != v: c.p[y + 7][x] = v; n += 1
    for x in (10, 11): c.p[47][x] = r.T
    return ["two pixels of the belt's top outline jutting out behind his lower back (x10-11, row 47) removed, so his back runs into the seat",
            f"the lowered belt's studs, which the inpainting redrew in bright yellow and purple, given back his crouch's own belt rows (the stance's rows 40-42, 7 lower): {n} pixels"]


POSES = [  # pose, kept candidate, round, source name, fixes
    ("sweep", "f01c4b4b", "round2", "sweep", sweep_fix),
    ("rising", "54ff78aa", "round2", "rising", None),
]
report = {}
for pose, job, rnd, name, fix in POSES:
    src = r.load(glob.glob(f"{C}/{rnd}/inpaint-source-{name}-*.png")[0])
    m = Image.open(f"{C}/{rnd}/inpaint-mask-{name}.png").convert('RGB')
    inside = lambda x, y, m=m: m.getpixel((x, y)) == (255, 255, 255)
    off = k.offpal(f"{C}/{job}-0.png")
    c = r.load(f"{C}/{job}-0.png")
    bad = [(x, y) for y in range(c.h) for x in range(c.w) if not inside(x, y) and c.p[y][x] != src.p[y][x]]
    assert not bad, (pose, bad[:5])
    drawn = sum(1 for y in range(c.h) for x in range(c.w) if inside(x, y) and c.p[y][x] != src.p[y][x])
    isl = k.islands(c)
    extra = fix(c, inside) if fix else []
    c.save(f"{OUT}/{pose}.png")
    report[pose] = [f"outside the mask every pixel is the source's; inside, {drawn} pixels differ from the source, and the {off} pixels the inpainting drew off his palette were taken to his nearest pens"] \
        + ([f"{isl} stray pixels removed"] if isl else []) + extra
with open(f"{OUT}/touch-ups.json", 'w', newline='\n') as f:
    json.dump(report, f, indent=1)
print(json.dumps(report, indent=1))
