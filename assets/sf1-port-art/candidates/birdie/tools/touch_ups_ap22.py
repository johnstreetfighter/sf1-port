"""Touch up Birdie's inpainted air-set and Bull Horn candidates (AP22, #186).

usage: touch_ups_ap22.py <candidates/birdie folder> <out folder>

As touch_ups.py does for his ground set: outside the mask every pixel must be the source's (his ROM pixels, and his
approved crouching kick's); inside it, inpainting's colours go to his nearest pens, light specks inside his dark
jeans are set to the pens around them, opaque specks under 4 pixels are removed, and the fixes listed per pose are
made. Writes <out>/<pose>.png and prints each pose's touch-ups as JSON. The jumping kick is written by make_air.py.
"""
import glob, json, os, sys
from PIL import Image
import rompose as r, clean as k
C, OUT = sys.argv[1], sys.argv[2]
os.makedirs(OUT, exist_ok=True)

DARK = {0, 7, 11, 12, 13, 14}

def specks(c, inside):
    """Light pixels inside his dark jeans, with only dark pens around them: set to the commonest of those."""
    from collections import Counter
    n = 0
    for y in range(c.h):
        for x in range(c.w):
            if not inside(x, y) or c.p[y][x] not in (3, 4, 5, 6): continue
            n8 = [c.at(x + dx, y + dy) for dx in (-1, 0, 1) for dy in (-1, 0, 1) if dx or dy]
            if all(v in DARK for v in n8):
                c.p[y][x] = Counter(n8).most_common(1)[0][0]; n += 1
    return n

def horn_fix(c):
    c.p[23][39] = 1                     # a hair-yellow pixel on the top line of his near shoulder: outline brown
    c.p[22][48] = 1                     # a hole in the top line of his neck
    n = 0
    for x in range(44, 49):             # a gap the inpainting left between the neck and the near arm's top line
        if c.p[30][x] == r.T: c.p[30][x] = 3; n += 1
    return ["a hair-yellow pixel on the outline of his near shoulder set to his outline brown, and a one-pixel hole in the top line of his neck filled",
            f"a {n}-pixel gap the inpainting left open between the back of his neck and the top of his near arm filled with his skin"]

POSES = [  # pose, kept candidate, round, source name, fixes
    ("jump_kick_high", "7b64bd99", "ap22-r1", "jump-kick-high", None),
    ("jump_kick_down", "6c7fef02", "ap22-r1", "jump-kick-down", None),
    ("bull_horn", "d383ad74", "ap22-r1", "bull-horn", horn_fix),
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
    sp = specks(c, inside) if pose.startswith("jump") else 0
    isl = k.islands(c)
    extra = fix(c) if fix else []
    if pose == "bull_horn":             # the source was rows 16-111 of the pose: his legs below it, and the canvas above
        full = r.P(96, 128); full.paste(c, 0, 16)
        full.paste(r.grid(0x271a, 1, 3, 2).crop(0, 112, 96, 16), 0, 112)
        c = full
    c.save(f"{OUT}/{pose}.png")
    report[pose] = [f"outside the mask every pixel is the source's; inside, {drawn} pixels differ from the source, and the {off} pixels the inpainting drew off his palette were taken to his nearest pens"] \
        + ([f"{sp} light specks inside his dark jeans set to the pens around them"] if sp else []) \
        + ([f"{isl} stray pixels removed"] if isl else []) + extra
print(json.dumps(report, indent=1))
