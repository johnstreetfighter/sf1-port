"""Touch up Geki's ground-set candidates (AP11, #175) by hand.

usage: ground-touch-ups.py <out folder>

- The jab, the light kick and the crouching kick are his ROM poses with one
  limb inpainted (his stance 0x0e00 and his front kick's knee-up pose 0x0f08,
  each padded to 128x96 at (0, 0); his crouch 0x0fc7 padded to 96x64): outside
  each mask the pixels are the ROM pose's own.
- The standing block is generated whole on his stance (ac8642f1, candidate 0).

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
PAL = ["#000000", "#664400", "#996655", "#cc9977", "#ddaa88", "#ddccaa", "#6666bb", "#7788dd",
       "#555555", "#777777", "#999999", "#000055", "#444477", "#555599", "#6666bb"]
RGB = [tuple(int(h[i:i + 2], 16) for i in (1, 3, 5)) for h in PAL]
# Pens 6 and 14 share a colour: 14 is his body's blue, 6 his teleport
# smoke's. A colour maps to the last pen that has it, so his blue is 14.
IDX = {c: n for n, c in enumerate(RGB)}
BLACK = 0
# The pens his ROM body is drawn in: every opaque pen but the smoke's.
BODY = [0, 1, 2, 3, 4, 5, 7, 8, 9, 10, 11, 12, 13, 14]
N8 = ((1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1))


def load_near(source, mask=None):
    """Inpainting has no palette: inside the mask its pixels take his nearest
    body pen. Outside the mask the candidate is its source, already his pens.
    With no mask (a generation held to his colours) nothing is reduced."""
    im = Image.open(os.path.join(HERE, source)).convert("RGBA")
    m = Image.open(os.path.join(HERE, mask)).convert("RGB") if mask else Image.new("RGB", im.size)
    px, mp = im.load(), m.load()
    near, grid = 0, []
    for y in range(im.height):
        row = []
        for x in range(im.width):
            c = px[x, y]
            if c[3] < 128:
                row.append(15)
                continue
            exact = IDX.get(c[:3])
            if mp[x, y] == (0, 0, 0) and exact is not None:
                row.append(exact)
                continue
            best = min(BODY, key=lambda n: (sum((a - b) ** 2 for a, b in zip(RGB[n], c[:3])), -n))
            if exact != best:
                near += 1
            row.append(best)
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

    def shadow(self, spans, why):
        """His ROM poses stand on a two-row black ground shadow; drawn back in
        black over `spans` ({row: (x0, x1)}), only where nothing is drawn."""
        pts = [(x, y) for y, (x0, x1) in spans.items() for x in range(x0, x1 + 1) if self.g[y][x] == 15]
        for x, y in pts:
            self.g[y][x] = BLACK
        if pts:
            rows = ", ".join(f"row {y} x{x0}-{x1}" for y, (x0, x1) in spans.items())
            self.log.append(f"ground shadow in black ({len(pts)} pixels; {rows}): {why}")

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


# (pose, candidate, mask, what the mask covers and what was drawn there, the ROM pose under it)
INPAINTED = [
    ("jab_2", "1e868ac3-0.png", "inpaint-mask-jab-2.png",
     "his front arm and the chest behind its fist inpainted (x33-127, y23-42, and right of his hood x52-127, y8-22), driven straight out at shoulder height, the fist clenched",
     "his stance (0x0e00)"),
    ("light_kick-round-1", "4a318662-0.png", "inpaint-mask-light-kick-2.png",
     "his raised leg inpainted from the hip (x38-127, y42-88), kicked straight out forward at waist height",
     "his front kick's knee-up pose (0x0f08)"),
    ("crouching_kick-round-1", "9d46139a-0.png", "inpaint-mask-crouching-kick.png",
     "his front leg inpainted from the hip (x28-95, y38-63 and x35-95, y33-37), kicked straight out forward just above the floor, squatting on his rear foot",
     "his crouch (0x0fc7)"),
]
SHADOWS = {
    "crouching_kick-round-1": ({62: (21, 50), 63: (0, 62)}, "run on under the kicking leg, as under his ROM kicks"),
}

for name, source, mask, what, base in INPAINTED:
    grid, near = load_near(source, mask)
    p = Pose(name, grid)
    p.log.append(f"{what}; the rest is {base}, pixel for pixel; {near} pixels the inpainting drew off his palette set to his nearest body pens")
    p.islands()
    if name in SHADOWS:
        p.shadow(*SHADOWS[name])
    p.save(source)

grid, _ = load_near("ac8642f1-0.png")
p = Pose("standing_block", grid)
p.islands()
p.save("ac8642f1-0.png")

# The second round (user, 2026-11-04: the inpainted kicks' seams showed and
# his body should move into the kick; the sweep was not a sweep and had a
# blade in his hand). The kicks and the sweep are generated whole; only stray
# specks are touched.
WHOLE = [
    ("light_kick", "7c21fffb-2.png"),
    ("crouching_kick", "1ca8a84a-0.png"),
    ("sweep-round-2", "72895f6f-1.png"),
    # The third round (user, 2026-11-04: candidate 1 did not read as a sweep;
    # use another): candidate 2, facing us, his front leg swept along the
    # floor toward the opponent.
    ("sweep", "72895f6f-2.png"),
]
for name, source in WHOLE:
    grid, _ = load_near(source)
    p = Pose(name, grid)
    p.islands()
    p.save(source)
