"""Touch up Sagat's air-set and low Tiger Shot candidates (AP8, #172) by hand.

usage: air-touch-ups.py <out folder>

As ground-touch-ups.py, whose Pose steps this reuses: for each pose,
<out>/<pose>.png (RGBA, his colours only), <out>/<pose>.json (the touch-up
list) and <out>/<pose>-sheet.png. Airborne poses get no ground shadow, as his
ROM airborne pose (0x2d0a in his hop) has none.
"""
import importlib.util
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = sys.argv[1]

# The Pose steps live in ground-touch-ups.py, which runs its own poses when
# loaded; load it into a scratch folder so only its definitions are kept.
spec = importlib.util.spec_from_file_location("ground", os.path.join(HERE, "ground-touch-ups.py"))
ground = importlib.util.module_from_spec(spec)
saved_argv = sys.argv
sys.argv = [saved_argv[0], os.path.join(OUT, "_ground")]
spec.loader.exec_module(ground)
sys.argv = saved_argv
ground.OUT = OUT
Pose, load, N8 = ground.Pose, ground.load, ground.N8


def islands(p, smallest=12):
    """Stray specks floating off his body (the release's sparks): opaque
    groups of fewer than `smallest` pixels that touch nothing else."""
    seen, gone = set(), []
    for y in range(p.H):
        for x in range(p.W):
            if p.g[y][x] == 15 or (x, y) in seen:
                continue
            group, stack = [], [(x, y)]
            seen.add((x, y))
            while stack:
                a, b = stack.pop()
                group.append((a, b))
                for dx, dy in N8:
                    q = (a + dx, b + dy)
                    if q not in seen and 0 <= q[0] < p.W and 0 <= q[1] < p.H and p.g[q[1]][q[0]] != 15:
                        seen.add(q)
                        stack.append(q)
            if len(group) < smallest:
                gone += group
    for x, y in gone:
        p.g[y][x] = 15
    if gone:
        p.log.append(f"stray specks floating off his body cleared ({len(gone)} pixels in groups of fewer than {smallest})")


AIR = {
    "jump_punch": "4c96df5a-2.png",
    "jump_punch_down": "2271d612-2.png",
    "jump_kick": "7d16a998-3.png",
    "jump_kick_high": "7d16a998-2.png",
    "jump_kick_down": "f352097f-1.png",
}
GROUND = {
    "low_tiger_shot_1": "902885b9-1.png",
    "low_tiger_shot_2": "70fd3709-1.png",
}
def no_stripe(p):
    """Seen from behind, his side stripe is out of sight: the light line the
    generator drew across the back of his shorts goes to their purples."""
    light, purple = {6, 8}, ground.SHORTS
    pts = [(x, y) for y in range(p.H) for x in range(p.W)
           if p.g[y][x] in light and any(p.at(x + dx, y + dy) in purple for dx, dy in N8)]
    for x, y in pts:
        around = [p.at(x + dx, y + dy) for dx, dy in N8]
        purples = [q for q in around if q in purple]
        p.g[y][x] = max(set(purples), key=purples.count) if purples else 12
    if pts:
        p.log.append(f"the light line across the back of his shorts ({len(pts)} pixels) painted out in their purples: from behind his side stripe is out of sight")


def load_near(source):
    """Inpainting has no palette: its pixels take his nearest pens."""
    from PIL import Image
    im = Image.open(os.path.join(HERE, source)).convert("RGBA")
    px = im.load()
    near, grid = 0, []
    for y in range(im.height):
        row = []
        for x in range(im.width):
            c = px[x, y]
            if c[3] < 128:
                row.append(15)
            elif c[:3] in ground.IDX:
                row.append(ground.IDX[c[:3]])
            else:
                near += 1
                row.append(min(range(15), key=lambda n: sum((a - b) ** 2 for a, b in zip(ground.RGB[n], c[:3]))))
        grid.append(row)
    return grid, near


# The second round (user, 2026-11-03: the legs changed between his knee pose
# and the strike). Both strikes are now his ROM knee pose (0x2d0a, padded to
# 128x128 at (8, 0)) with one part inpainted, so his legs stay the knee pose's:
# the punch's front arm (job a7a48e95), and the kick's raised leg (67f2e191)
# whose stub of a foot was then inpainted again on its own (a2212e1d).
INPAINTED = {
    "jump_punch": ("a7a48e95-0.png", (50, 0, 127, 36), "his front arm inpainted, straight out at head height (x50-127, y0-36 of the padded knee pose); the rest is his ROM knee pose, pixel for pixel"),
    "jump_kick": ("a2212e1d-0.png", (55, 36, 127, 100), "his raised leg inpainted, kicked straight out at waist height (x55-127, y48-100, not his shorts), then its foot inpainted again (x110-127, y36-69), flexed with the toes up; the rest is his ROM knee pose, pixel for pixel"),
}
for name, (source, box, what) in INPAINTED.items():
    grid, near = load_near(source)
    p = Pose(name, grid)
    p.log.append(f"{what}; {near} pixels the inpainting drew off his palette set to his nearest pens")
    islands(p)
    p.specks(only=box)
    p.save(source)
AIR.pop("jump_punch")
AIR.pop("jump_kick")

BACK = {"jump_kick_down"}
for name, source in list(AIR.items()) + list(GROUND.items()):
    p = Pose(name, load(source))
    islands(p)
    p.eyepatch(22)
    if name in BACK:
        no_stripe(p)
    else:
        p.one_stripe()
    p.specks()
    if name in GROUND:
        p.shadow()
    p.save(source)
