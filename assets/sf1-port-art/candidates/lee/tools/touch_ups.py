"""Touch up Lee's inpainted ground-set candidates (AP17, #181).

usage: touch_ups.py <candidates/lee folder> <out folder>

Each candidate is checked against its inpainting source: outside the mask every pixel
must be the source's (his ROM pixels). Inside the mask, inpainting's colours are taken
to his nearest pens, single pixels no 4-neighbour shares are set to the pen around them,
and opaque specks under 4 pixels are removed. Writes <out>/<pose>.png (his colours only)
and prints each pose's touch-ups as JSON.
"""
import json, os, sys
from PIL import Image
import rompose as r, clean as k
C, OUT = sys.argv[1], sys.argv[2]
os.makedirs(OUT, exist_ok=True)
POSES = [  # pose, kept candidate, source folder, source name
    ("crouching_block", "6c8738ee", "round1", "crouching-block-64x72"),
    ("crouching_punch", "2a2fb3db", "round1", "crouching-punch-64x64"),
    ("crouching_kick", "e7de0bac", "round1", "crouching-kick-72x64"),
    ("crouching_hit", "a42c0543", "round1c", "crouching-hit-68x64"),
]
report = {}
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
    isl = k.islands(c)
    extra = []
    if pose == "crouching_kick":   # his ROM ground shadow is two rows deep; the source drew one under the outstretched leg
        n = 0
        for x in range(c.w):
            if c.p[63][x] == r.T and c.p[62][x] == 0: c.p[63][x] = 0; n += 1
        extra = [f"his black ground shadow's second row carried on under the outstretched leg ({n} pixels), as his ROM sweep (0x1f04) draws it"]
    c.save(f"{OUT}/{pose}.png")
    report[pose] = [f"outside the mask every pixel is the ROM source's; inside, {drawn} pixels differ from the source, and the {off} pixels the inpainting drew off his palette were taken to his nearest pens",
                    f"{lone} single pixels inside the mask set to the pen around them" + (f"; {isl} stray pixels removed" if isl else "")] + extra
print(json.dumps(report, indent=1))
