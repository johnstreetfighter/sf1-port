"""Touch up Birdie's round-three candidates (AP21, #185), as touch_ups.py does round one's.

usage: touch_ups_r3.py <candidates/birdie folder> <out folder>
"""
import glob, json, os, sys
from collections import Counter
from PIL import Image
import rompose as r, clean as k
C, OUT = sys.argv[1], sys.argv[2]
os.makedirs(OUT, exist_ok=True)
DARK = {0, 7, 11, 12, 13, 14}

def specks(c, inside):
    n = 0
    for y in range(c.h):
        for x in range(c.w):
            if not inside(x, y) or c.p[y][x] not in (3, 4, 5, 6): continue
            n8 = [c.at(x + dx, y + dy) for dx in (-1, 0, 1) for dy in (-1, 0, 1) if dx or dy]
            if all(v in DARK for v in n8):
                c.p[y][x] = Counter(n8).most_common(1)[0][0]; n += 1
    return n

POSES = [
    ("crouching_kick", "a728b5b3", "round3", "crouching-kick", None),
    ("rising", "83859433", "round3", "rising", None),
]
report = {}
for pose, job, rnd, name, fix in POSES:
    src = r.load(glob.glob(f"{C}/{rnd}/inpaint-source-{name}-*.png")[0])
    m = Image.open(f"{C}/{rnd}/inpaint-mask-{name}.png").convert('RGB')
    inside = lambda x, y: m.getpixel((x, y)) == (255, 255, 255)
    off = k.offpal(f"{C}/{job}-0.png")
    c = r.load(f"{C}/{job}-0.png")
    bad = [(x, y) for y in range(c.h) for x in range(c.w) if not inside(x, y) and c.p[y][x] != src.p[y][x]]
    assert not bad, (pose, bad[:5])
    drawn = sum(1 for y in range(c.h) for x in range(c.w) if inside(x, y) and c.p[y][x] != src.p[y][x])
    sp = specks(c, inside) if pose != "rising" else 0   # rising's light pixels in the band are his belt's studs
    isl = k.islands(c)
    extra = fix(c) if fix else []
    c.save(f"{OUT}/{pose}.png")
    report[pose] = [f"outside the mask every pixel is the ROM source's; inside, {drawn} pixels differ from the source, and the {off} pixels the inpainting drew off his palette were taken to his nearest pens"] \
        + ([f"{sp} light specks inside his dark jeans set to the pens around them"] if sp else []) \
        + ([f"{isl} stray pixels removed"] if isl else []) + extra
print(json.dumps(report, indent=1))
