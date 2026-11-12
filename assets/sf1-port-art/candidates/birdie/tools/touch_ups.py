"""Touch up Birdie's inpainted ground-set candidates (AP21, #185).

usage: touch_ups.py <candidates/birdie folder> <out folder>

Each candidate is checked against its inpainting source: outside the mask every pixel must be the source's (his
ROM pixels). Inside the mask, inpainting's colours are taken to his nearest pens, light specks inside his dark jeans
are set to the pens around them, opaque specks under 4 pixels are removed, and the fixes listed per pose are made.
His jeans' black-and-grey dither is left as drawn (a single-pixel clean-up would flatten it). Writes
<out>/<pose>.png and prints each pose's touch-ups as JSON. The crouch, crouching block and crouching punch are
written by make_ground.py.
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

def clear(c, pts):
    for x, y in pts: c.p[y][x] = r.T
    return len(pts)

def jeans(c, box, pens=(1, 2, 3)):
    """Brown pixels the inpainting drew across his jeans in box: his jeans' greys instead, by parity."""
    x0, y0, x1, y1 = box; n = 0
    for y in range(y0, y1):
        for x in range(x0, x1):
            if c.p[y][x] in pens: c.p[y][x] = 12 if (x + y) % 2 else 13; n += 1
    return n

def kick_fix(c):
    n = clear(c, [(x, y) for y in range(72, 76) for x in (42, 43) if c.p[y][x] != r.T])
    return [f"a {n}-pixel stub of black outline left hanging below the hip, between the thigh and his rear leg, removed"]

def sweep_fix(c):
    n = jeans(c, (48, 64, 56, 73))
    return [f"a brown seam the inpainting drew across the thigh ({n} pixels) given his jeans' greys"]

def high_fix(c):
    n = clear(c, [(x, y) for y in range(79, 83) for x in range(35, 40) if c.p[y][x] != r.T])
    return [f"a {n}-pixel spike of black outline hanging under the raised thigh, beside his standing leg, removed"]

POSES = [  # pose, kept candidate, round, source name, fixes
    ("crouching_hit", "a098315a", "round1", "crouching-hit", None),
    ("crouching_kick", "44889e0f", "round1", "crouching-kick", kick_fix),
    ("sweep", "89d140db", "round1", "sweep", sweep_fix),
    ("high_kick", "fc5942a0", "round1", "high-kick", high_fix),
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
    sp = specks(c, inside)
    isl = k.islands(c)
    extra = fix(c) if fix else []
    c.save(f"{OUT}/{pose}.png")
    report[pose] = [f"outside the mask every pixel is the ROM source's; inside, {drawn} pixels differ from the source, and the {off} pixels the inpainting drew off his palette were taken to his nearest pens"] \
        + ([f"{sp} light specks inside his dark jeans set to the pens around them"] if sp else []) \
        + ([f"{isl} stray pixels removed"] if isl else []) + extra
print(json.dumps(report, indent=1))
