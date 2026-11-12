"""Touch up Gen's AP20 candidates (#184): the jumping punch and diagonal punch on his top-of-jump tuck.

usage: ap20_touch_ups.py <candidates/gen folder> <out folder> [r2|r3]

Round two (r2) is the diagonal punch again, on the wider mask (ap20_make_r2.py); round three (r3)
on the shaded slim-sleeve hint (ap20_make_r3.py shaded).

As AP19: outside the mask every pixel must be the source's; inside, inpainting's colours are taken
to his nearest pens, single pixels no 4-neighbour shares are set to the pen around them, and opaque
specks under 4 pixels are removed. Writes <out>/<pose>.png and <out>/touch-ups.json.
"""
import json, os, sys
from PIL import Image
import rompose as r, clean as k
C, OUT = sys.argv[1], sys.argv[2]
os.makedirs(OUT, exist_ok=True)
ROUND = sys.argv[3] if len(sys.argv) > 3 else "r1"
POSES = [("jump_punch_down", "5f794b52", "ap20/r3", "jump-punch-down-shaded-88x80")] if ROUND == "r3" else         [("jump_punch_down", "0cf86e6f", "ap20/r2", "jump-punch-down-88x80")] if ROUND == "r2" else [("jump_punch", "0dda1d67", "ap20/r1", "jump-punch-88x80"),
         ("jump_punch_down", "1fee1e49", "ap20/r1", "jump-punch-down-88x80")]
path = f"{OUT}/touch-ups.json"
report = json.load(open(path)) if os.path.exists(path) else {}
for pose, job, folder, name in POSES:
    src = r.load(f"{C}/{folder}/inpaint-source-{name}.png")
    m = Image.open(f"{C}/{folder}/inpaint-mask-{name.rsplit('-', 1)[0]}.png").convert('RGB')
    inside = lambda x, y: m.getpixel((x, y)) == (255, 255, 255)
    off = k.offpal(f"{C}/{job}-0.png")
    c = r.load(f"{C}/{job}-0.png")
    bad = [(x, y) for y in range(c.h) for x in range(c.w) if not inside(x, y) and c.p[y][x] != src.p[y][x]]
    assert not bad, (pose, bad[:5])
    drawn = sum(1 for y in range(c.h) for x in range(c.w) if inside(x, y) and c.p[y][x] != src.p[y][x])
    lone = k.lone(c, inside)
    extra = []
    if ROUND == "r3":
        # A grey speck on the sleeve's lower outline, and a one-pixel gap in it under the shoulder.
        assert c.p[30][50] in (8, 9, 10) and c.p[28][47] == r.T, "the round-three specks moved"
        c.p[30][50] = 11; c.p[28][47] = 11
        extra = ["a grey pixel on the sleeve's lower outline (x 50, row 30) and a one-pixel gap in that outline under the shoulder (x 47, row 28) set to the dark purple of the outline"]
    isl = k.islands(c)
    c.save(f"{OUT}/{pose}.png")
    report[pose] = [f"outside the mask every pixel is the ROM source's; inside, {drawn} pixels differ from the source, and the {off} pixels the inpainting drew off his palette were taken to his nearest pens",
                    f"{lone} single pixels inside the mask set to the pen around them" + (f"; {isl} stray pixels removed" if isl else "")] + extra
json.dump(report, open(path, 'w', newline='\n'), indent=1)
print(json.dumps(report, indent=1))
