"""Touch up Joe's ground-set candidates (AP13, #177) by hand.

usage: ground-touch-ups.py <out folder>

- The crouching hit, the crouching kick and the sweep are generated whole,
  held to his colours; only stray specks are touched.
- The crouch (the second round: the user found his ROM duck not low enough)
  is the crouching kick with its front leg inpainted, drawn in under him
  (inpaint-source-crouching-kick-128x72.png and inpaint-mask-crouch.png):
  outside the mask the pixels are the approved kick's own.
- His throw, thrown set and get-up are his ROM poses and need no file.

Writes, for each pose, <out>/<pose>.png (RGBA, his colours only),
<out>/<pose>.json (the touch-up list) and <out>/<pose>-sheet.png (before,
after, and the changed pixels in green). Coordinates are the candidate's
canvas.
"""
import json
import os
import sys

from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = sys.argv[1]
PAL = ["#000000", "#664400", "#996655", "#cc9977", "#ddaa88", "#ddccaa", "#bbbbcc", "#888899",
       "#666677", "#888855", "#bbbb77", "#440000", "#660000", "#990000", "#bb0000"]
RGB = [tuple(int(h[i:i + 2], 16) for i in (1, 3, 5)) for h in PAL]
IDX = {c: n for n, c in enumerate(RGB)}
# Every opaque pen of his draws his body: no two share a colour.
BODY = list(range(15))
N8 = ((1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1))


def load_near(source):
    """A generation held to his colours: every opaque pixel is already one of
    his pens; any that is not takes his nearest pen."""
    im = Image.open(os.path.join(HERE, source)).convert("RGBA")
    px = im.load()
    near, grid = 0, []
    for y in range(im.height):
        row = []
        for x in range(im.width):
            c = px[x, y]
            if c[3] < 128:
                row.append(15)
                continue
            exact = IDX.get(c[:3])
            if exact is None:
                exact = min(BODY, key=lambda n: sum((a - b) ** 2 for a, b in zip(RGB[n], c[:3])))
                near += 1
            row.append(exact)
        grid.append(row)
    return grid, near


class Pose:
    def __init__(self, name, grid):
        self.name, self.g = name, grid
        self.before = [row[:] for row in grid]
        self.H, self.W = len(grid), len(grid[0])
        self.log = []

    def at(self, x, y):
        return self.g[y][x] if 0 <= x < self.W and 0 <= y < self.H else 15

    def islands(self, smallest=6):
        """Stray specks floating off his body: opaque groups of fewer than
        `smallest` pixels that touch nothing else."""
        seen, gone = set(), []
        for y in range(self.H):
            for x in range(self.W):
                if self.g[y][x] == 15 or (x, y) in seen:
                    continue
                group, stack = [], [(x, y)]
                seen.add((x, y))
                while stack:
                    a, b = stack.pop()
                    group.append((a, b))
                    for dx, dy in N8:
                        q = (a + dx, b + dy)
                        if q not in seen and self.at(*q) != 15:
                            seen.add(q)
                            stack.append(q)
                if len(group) < smallest:
                    gone += group
        for x, y in gone:
            self.g[y][x] = 15
        if gone:
            self.log.append(f"stray specks floating off his body cleared ({len(gone)} pixels in groups of fewer than {smallest})")

    def save(self, source):
        os.makedirs(OUT, exist_ok=True)
        out = Image.new("RGBA", (self.W, self.H), (0, 0, 0, 0))
        o = out.load()
        for y in range(self.H):
            for x in range(self.W):
                if self.g[y][x] != 15:
                    o[x, y] = RGB[self.g[y][x]] + (255,)
        out.save(os.path.join(OUT, f"{self.name}.png"))
        json.dump({"source": source, "touch_ups": self.log}, open(os.path.join(OUT, f"{self.name}.json"), "w"), indent=2)

        def show(grid, mark=False):
            c = Image.new("RGBA", (self.W, self.H), (70, 110, 90, 255))
            p = c.load()
            for y in range(self.H):
                for x in range(self.W):
                    if grid[y][x] != 15:
                        p[x, y] = RGB[grid[y][x]] + (255,)
                    if mark and grid[y][x] != self.before[y][x]:
                        p[x, y] = (0, 255, 0, 255)
            return c.resize((self.W * 4, self.H * 4), Image.NEAREST)
        sheet = Image.new("RGBA", (self.W * 12 + 40, self.H * 4), (20, 20, 25, 255))
        for i, im2 in enumerate((show(self.before), show(self.g), show(self.g, True))):
            sheet.paste(im2, (i * (self.W * 4 + 20), 0))
        sheet.save(os.path.join(OUT, f"{self.name}-sheet.png"))
        changed = sum(1 for y in range(self.H) for x in range(self.W) if self.g[y][x] != self.before[y][x])
        print(f"{self.name} ({source}): {changed} pixels changed")
        for line in self.log:
            print("   ", line)


WHOLE = [
    ("crouching_hit", "3be48916-2.png"),
    ("crouching_kick", "d687eb8a-0.png"),
    ("sweep", "16329a05-3.png"),
]
for name, source in WHOLE:
    grid, near = load_near(source)
    p = Pose(name, grid)
    if near:
        p.log.append(f"{near} pixels off his palette set to his nearest pens")
    p.islands()
    p.save(source)

# Inpainting has no palette, so its pixels inside the mask take his nearest
# pens; outside the mask the source is already in his pens.
grid, near = load_near("ba6a2a51-0.png")
p = Pose("crouch-round-2", grid)
p.log.append(f"his front leg inpainted from the hip (x50-127, y38-71, and before his belt x60-90, y35-37), drawn in under him: the knee bent forward, the shin down to his shoe flat on the floor; the rest is his approved crouching kick (d687eb8a, candidate 0), pixel for pixel; {near} pixels the inpainting drew off his palette set to his nearest pens")
p.islands()
p.save("ba6a2a51-0.png")

# The third round (user, 2026-11-04: his hair did not look right). The
# round-two crouch, padded to 96x80 at (8, 13) (inpaint-source-crouch-96x80.png),
# with his hair inpainted (inpaint-mask-crouch-hair.png); then his ROM hair's
# top edge, an unbroken brown line dotted with black, closed between the tips.
grid, near = load_near("6c735ebc-0.png")
p = Pose("crouch-round-3", grid)
p.log.append(f"his hair inpainted (x32-54, y8-17, and the back of his head x34-39, y18-21) as a spiky olive-blond flat-top; the rest is the round-two crouch, pixel for pixel; {near} pixels the inpainting drew off his palette set to his nearest pens")
gaps = [x for x in range(37, 52) if p.g[10][x] == 15]
for x in gaps:
    p.g[10][x] = 1
p.log.append(f"the gaps between the black spike tips on the hair's top row (y10, {len(gaps)} pixels) closed in his dark brown outline (pen 1), as his ROM hair's top edge is drawn")
red = [(x, y) for y in range(9, 17) for x in range(30, 60) if p.g[y][x] in (11, 12, 13, 14)]
for x, y in red:
    p.g[y][x] = 9
p.log.append(f"dark red specks in his hair to his hair's olive (pen 9): {len(red)} pixels")
p.islands()
p.save("6c735ebc-0.png")

# The fourth round (user, 2026-11-04: the flat-top made him look like Guile;
# his hair is rounder and smoother). The same source and mask, the hair
# inpainted as a smooth rounded crew cut; nothing by hand.
grid, near = load_near("152a32dc-0.png")
p = Pose("crouch", grid)
p.log.append(f"his hair inpainted (x32-54, y8-17, and the back of his head x34-39, y18-21) as a smooth, rounded olive-blond crew cut; the rest is the round-two crouch, pixel for pixel; {near} pixels the inpainting drew off his palette set to his nearest pens")
# The fifth round (user, 2026-11-04: the hair looked see-through). His ROM
# hair is drawn only in olive (9), pale yellow (10), brown (1) and black; the
# skin tones mottled into this hair, above his hairline, read as his scalp.
SKIN_IN_HAIR = [(44, 12), (42, 13), (44, 13), (46, 13), (47, 13), (40, 14), (43, 14), (45, 14),
                (39, 15), (41, 15), (44, 15), (40, 16), (44, 16)]
TO_HAIR = {2: 1, 3: 9, 4: 9, 5: 10}
assert all(p.g[y][x] in TO_HAIR for x, y in SKIN_IN_HAIR)
for x, y in SKIN_IN_HAIR:
    p.g[y][x] = TO_HAIR[p.g[y][x]]
p.log.append(f"skin tones mottled into his hair above his hairline (rows 12-16, {len(SKIN_IN_HAIR)} pixels) to his hair's own colours, as his ROM hair is drawn: pale skin to pale yellow (pen 10), mid skin to olive (pen 9), and a light brown in its outline to his dark brown (pen 1)")
p.islands()
p.save("152a32dc-0.png")
