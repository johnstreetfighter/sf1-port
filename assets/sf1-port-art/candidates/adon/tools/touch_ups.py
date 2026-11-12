"""Touch up Adon's inpainted ground-set candidates (AP25, #189).

usage: touch_ups.py <candidates/adon folder> <out folder>

Each candidate is checked against its inpainting source: outside the mask every pixel must be the source's (his
ROM pixels). Inside the mask, inpainting's colours are taken to his nearest pens, opaque specks under 4 pixels are
removed, and the fixes listed per pose are made. Writes <out>/<pose>.png and <out>/touch-ups.json. The crouch,
crouching punch, crouching hit and crouching block are written by make_ground.py.
"""
import glob, json, os, sys
from PIL import Image
import rompose as r, clean as k
C, OUT = sys.argv[1], sys.argv[2]
os.makedirs(OUT, exist_ok=True)
SKIN = {2, 3, 4, 14}

def outline(c, inside, x0):
    """The leg's outline in his limbs' dark brown (pen 1), as his ROM legs are drawn: black (0) and his trunks'
    reds (8, 9) the inpainting drew on the thigh's edge, right of x0."""
    n = 0
    for y in range(c.h):
        for x in range(x0, c.w):
            if inside(x, y) and c.p[y][x] in (0, 8, 9):
                c.p[y][x] = 1; n += 1
    return n

def sweep_fix(c, inside):
    n = outline(c, inside, 32)
    for x, y in ((37, 58), (33, 57)): c.p[y][x] = 1
    # The swept foot's sole ran a row below his ground shadow: the leg beyond the mask (x >= 54) moved up a row.
    for y in range(40, 72):
        for x in range(54, c.w):
            c.p[y][x] = c.p[y + 1][x] if y + 1 < c.h else r.T
    return ["the leg beyond the inpainted hip (x 54 on: shin, wrap and foot) moved up a row, so the swept foot's sole rests on his ground line instead of a row below his shadow",
            f"{n} black and dark red pixels the inpainting drew along the thigh's outline given his legs' dark brown outline (pen 1)",
            "a pale and a grey speck in the thigh's underside outline (37,58 and 33,57) given the same dark brown"]

def rising_fix(c, inside):
    pts = [(18, 62), (19, 62), (18, 63)]
    for x, y in pts: c.p[y][x] = 1
    low = [x for x in range(c.w) if c.p[71][x] != r.T]
    for x in low: c.p[70][x] = 1; c.p[71][x] = r.T
    return [f"the rear knee's bottom outline, which ran a row below his ground shadow (x{low[0]}-{low[-1]}), moved up onto the shadow's row",
            f"{len(pts)} grey wrap-coloured specks inside the rear knee's outline given his legs' dark brown outline (pen 1)"]

POSES = [  # pose, kept candidate, round, source name, fixes
    ("crouching_kick", "3605d103", "round1", "crouching-kick", None),
    ("sweep", "1662670a", "round1", "sweep", sweep_fix),
    ("rising", "c572eea5", "round1", "rising", rising_fix),
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
    before = c.copy()
    isl = k.islands(c)
    extra = fix(c, inside) if fix else []
    c.save(f"{OUT}/{pose}.png")
    report[pose] = [f"outside the mask every pixel is the ROM source's; inside, {drawn} pixels differ from the source, and the {off} pixels the inpainting drew off his palette were taken to his nearest pens"] \
        + ([f"{isl} stray pixels removed"] if isl else []) + extra
with open(f"{OUT}/touch-ups.json", 'w', newline='\n') as f:
    json.dump(report, f, indent=1)
print(json.dumps(report, indent=1))
