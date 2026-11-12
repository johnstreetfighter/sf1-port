"""Touch up Gen's inpainted ground-set candidates (AP19, #183).

usage: touch_ups.py <candidates/gen folder> <out folder>

Each candidate is checked against its inpainting source: outside the mask every pixel
must be the source's (his ROM pixels). Inside the mask, inpainting's colours are taken
to his nearest pens, single pixels no 4-neighbour shares are set to the pen around them,
and opaque specks under 4 pixels are removed. The crouch's seam is then finished by hand
(crouchfix.py). Writes <out>/<pose>.png (his colours only), touch-ups.json, and prints it.
"""
import json, os, sys
from PIL import Image
import rompose as r, clean as k, crouchfix
C, OUT = sys.argv[1], sys.argv[2]
os.makedirs(OUT, exist_ok=True)
POSES = [  # pose, kept candidate, source folder, source name
    ("crouch", "5cb120f1", "round1", "crouch-64x88"),
    ("crouching_block", "2164ace6", "round1b", "crouching-block-64x88"),
    ("crouching_punch", "fc269754", "round1", "crouching-punch-84x88"),
    ("crouching_kick", "9a186246", "round1", "crouching-kick-96x88"),
    ("crouching_hit", "fd13227c", "round1c", "crouching-hit-64x88"),
    ("low_kick", "3ca907f3", "round1", "low-kick-96x96"),
]
report = {}
for pose, job, folder, name in POSES:
    srcf = f"{C}/{folder}/inpaint-source-{name}.png"
    src = r.load(srcf)
    m = Image.open(f"{C}/{folder}/inpaint-mask-{name.rsplit('-', 1)[0]}.png").convert('RGB')
    inside = lambda x, y: m.getpixel((x, y)) == (255, 255, 255)
    off = k.offpal(f"{C}/{job}-0.png")
    c = r.load(f"{C}/{job}-0.png")
    bad = [(x, y) for y in range(c.h) for x in range(c.w) if not inside(x, y) and c.p[y][x] != src.p[y][x]]
    assert not bad, (pose, bad[:5])
    drawn = sum(1 for y in range(c.h) for x in range(c.w) if inside(x, y) and c.p[y][x] != src.p[y][x])
    extra = []
    if pose == "crouch":
        c = crouchfix.crouch(f"{C}/{job}-0.png", srcf)
        lone = k.lone(c, lambda x, y: inside(x, y) and x >= 31 and not 36 <= x < 48)
        extra = ["his left thigh (x < 31) set back to the source's inside the mask: the inpainting had bulged it outward",
                 "his robe's red trim redrawn by hand inside the mask as one line bending over his knee (rows 68-76, x 39 to 45), each row's trim pixel and its dark-purple edges moved onto the line and the robe's colour filled in behind it; the inpainting had left it broken where the two halves of the cut meet"]
    else:
        lone = k.lone(c, inside)
    if pose == "crouching_block":
        # The arm's crop (0x200c from x32) took in the edge of his other hand, the one at his chest
        # in the standing block: five skin pixels in a column under the sleeve, outside the mask.
        n = 0
        for y in range(50, 62):
            if c.p[y][40] in (1, 2, 3, 4, 5): c.p[y][40] = 11; n += 1
        extra = [f"{n} skin pixels in a column under the raised arm's sleeve (x 40, rows 54-58), the edge of his other hand that the standing block's crop took in, set to the sleeve's dark purple around them"]
    isl = k.islands(c)
    c.save(f"{OUT}/{pose}.png")
    report[pose] = [f"outside the mask every pixel is the ROM source's; inside, {drawn} pixels differ from the source, and the {off} pixels the inpainting drew off his palette were taken to his nearest pens",
                    f"{lone} single pixels inside the mask set to the pen around them" + (f"; {isl} stray pixels removed" if isl else "")] + extra
json.dump(report, open(f"{OUT}/touch-ups.json", 'w', newline='\n'), indent=1)
print(json.dumps(report, indent=1))
