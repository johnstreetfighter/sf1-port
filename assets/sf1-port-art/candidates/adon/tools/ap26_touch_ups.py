"""Touch up Adon's inpainted air-set candidates (AP26, #190), round one.

usage: ap26_touch_ups.py <candidates/adon folder> <out folder>

Each candidate is checked against its inpainting source: outside the mask every pixel must be the source's (his
ROM pixels). Inside the mask, inpainting's colours are taken to his nearest pens, opaque specks under 4 pixels are
removed, and the fixes listed per pose are made. Writes <out>/<pose>.png and adds to <out>/touch-ups.json.
"""
import glob, json, os, sys
from PIL import Image
import rompose as r, clean as k
C, OUT = sys.argv[1], sys.argv[2]
os.makedirs(OUT, exist_ok=True)


def punch_down_fix(c, inside):
    c.p[29][43] = r.T
    return ["a pale skin pixel the inpainting left outside the arm's lower outline at the shoulder (43,29) removed"]


def knee_fix(c, inside):
    for x in (27, 28, 29): c.p[52][x] = r.T
    c.p[48][34] = 11
    return ["the old thigh's hem left under his shorts' front corner, three pixels hanging below it (x27-29, row 52), removed",
            "a dark purple speck on the wrap's back edge (34,48) given the wrap's grey outline"]


POSES = [  # pose, kept candidate, round, source name, fixes
    ("jump_punch_down", "1e47265c", "ap26/r1", "jump-punch-down", punch_down_fix),
    ("jump_kick_high", "2f8f8e66", "ap26/r1", "jump-kick-high", None),
    ("rj_knee", "805bef93", "ap26/r1", "rj-knee", knee_fix),
]
path = f"{OUT}/touch-ups.json"
report = json.load(open(path)) if os.path.exists(path) else {}
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
    report[pose] = [f"outside the mask every pixel is the ROM source's; inside, {drawn} pixels differ from the source, and the {off} pixels the inpainting drew off his palette were taken to his nearest pens"] \
        + ([f"{isl} stray pixels removed"] if isl else []) + extra
with open(path, 'w', newline='\n') as f:
    json.dump(report, f, indent=1)
print(json.dumps(report, indent=1))
