"""Touch up Lee's inpainted air-set and Zesshouohken candidates (AP18, #182).

usage: touch_ups_ap18.py <candidates/lee folder> <out folder>

As touch_ups.py (AP17): outside the mask every pixel must be the source's (his ROM pixels);
inside, inpainting's colours are taken to his nearest pens, single pixels no 4-neighbour
shares are set to the pen around them, and opaque specks under 4 pixels are removed.
Writes <out>/<pose>.png and prints each pose's touch-ups as JSON.
"""
import json, os, sys
from PIL import Image
import rompose as r, clean as k
C, OUT = sys.argv[1], sys.argv[2]
os.makedirs(OUT, exist_ok=True)
POSES = [  # pose, kept candidate, source folder, source name
    ("jump_punch", "c2fe6740", "ap18-r1", "jump-punch-88x80"),
    ("jump_punch_down", "c2cd6eb6", "ap18-r1", "jump-punch-down-84x80"),
    ("jump_kick", "e5ec59a1", "ap18-r1", "jump-kick-104x80"),
    ("zessh_strike", "ba0968e3", "ap18-r1", "zessh-strike-112x84"),
]
only = sys.argv[3:]
report = {}
for pose, job, folder, name in POSES:
    if only and pose not in only: continue
    src = r.load(f"{C}/{folder}/inpaint-source-{name}.png")
    m = Image.open(f"{C}/{folder}/inpaint-mask-{name.rsplit('-', 1)[0]}.png").convert('RGB')
    inside = lambda x, y: m.getpixel((x, y)) == (255, 255, 255)
    off = k.offpal(f"{C}/{job}-0.png")
    c = r.load(f"{C}/{job}-0.png")
    bad = [(x, y) for y in range(c.h) for x in range(c.w) if not inside(x, y) and c.p[y][x] != src.p[y][x]]
    assert not bad, (pose, bad[:5])
    drawn = sum(1 for y in range(c.h) for x in range(c.w) if inside(x, y) and c.p[y][x] != src.p[y][x])
    lone = k.lone(c, inside)
    before = c.copy()
    isl = k.islands(c)
    outside = [(x, y) for y in range(c.h) for x in range(c.w) if not inside(x, y) and c.p[y][x] != before.p[y][x]]
    extra = []
    if pose == "jump_punch_down":   # what hung below the arm once the flying kick's leg was not under it
        gone = [(x, y) for y in range(39, 41) for x in range(50, 56) if c.p[y][x] != r.T]
        tail = [(x, y) for y in range(44, 47) for x in range(62, 69) if c.p[y][x] != r.T]
        for x, y in gone + tail: c.p[y][x] = r.T
        extra = [f"a loose fragment of jacket inpainted below the sleeve, over the knee ({len(gone)} pixels), removed",
                 f"the flying kick's (0x1ec0) outline of the leg under the cuff, left hanging below it ({len(tail)} ROM pixels), removed; the cuff ends on its own outline"]
    c.save(f"{OUT}/{pose}.png")
    report[pose] = [f"outside the mask every pixel is the ROM source's; inside, {drawn} pixels differ from the source, and the {off} pixels the inpainting drew off his palette were taken to his nearest pens",
                    f"{lone} single pixels inside the mask set to the pen around them" + (f"; {isl - len(outside)} stray pixels inside it removed" if isl - len(outside) else "")]
    report[pose] += extra
    if outside:   # ROM pixels left cut off from everything by the inpainting beside them
        report[pose].append(f"{len(outside)} ROM pixels just outside the mask, left as a speck cut off from the body ({', '.join(f'x{x} y{y}' for x, y in outside)}), removed")
print(json.dumps(report, indent=1))
