"""Touch up Retsu's ground-set candidates (AP9, #173) by hand.

usage: ground-touch-ups.py <out folder>

The crouching punch's two poses are his ROM crouch (0x124d, padded to 96x64
at (2, 0): inpaint-source-crouch-96x64.png) with his front arm inpainted;
outside each mask the pixels are the crouch's own. The first round's kick and
chamber were made the same way; the user rejected both, the kick for kicking
from a kneel (2026-11-03), so they are written as <pose>-round-1. The
second round's kick is generated whole, squatting on his rear foot (edae4568,
candidate 0, 120x88), and its chamber is that kick with only the front leg
inpainted (e34013b9). Reads the raw candidates beside this script and writes,
for each pose, <out>/<pose>.png (RGBA, his colours only), <out>/<pose>.json
(the touch-up list) and <out>/<pose>-sheet.png (before, after, and the changed
pixels in green). Coordinates are the candidate's canvas.
"""
import json
import os
import sys

from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = sys.argv[1]
PAL = ["#000000", "#664400", "#996655", "#cc9977", "#ddaa88", "#ddccaa", "#445555", "#883344",
       "#bbcccc", "#99aaaa", "#667777", "#334444", "#888899", "#666677", "#444455"]
RGB = [tuple(int(h[i:i + 2], 16) for i in (1, 3, 5)) for h in PAL]
IDX = {c: n for n, c in enumerate(RGB)}
BLACK = 0
# The pens his ROM body is drawn in. His lining's maroon (7) shows only where
# his robe flares, and the lilac greys (12-14) only in a few dozen pixels of
# his jump and sweep; inpainting reached for them on his trousers and sleeve.
BODY = [0, 1, 2, 3, 4, 5, 6, 8, 9, 10, 11]
N8 = ((1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1))


def load_near(source, mask=None):
    """Inpainting has no palette: inside the mask its pixels take his nearest
    body pen. Outside the mask the candidate is its source, already his pens.
    With no mask (a generation held to his pens) nothing is reduced."""
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
            if mp[x, y] == (0, 0, 0):
                row.append(exact)
                continue
            best = min(BODY, key=lambda n: sum((a - b) ** 2 for a, b in zip(RGB[n], c[:3])))
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

    def body_pens(self):
        """His lining's maroon and the lilac greys his ROM body is not drawn
        in, to his nearest body pens (on the belt's hanging ends here)."""
        pts = [(x, y) for y in range(self.H) for x in range(self.W) if self.g[y][x] in (7, 12, 13, 14)]
        for x, y in pts:
            pen = self.g[y][x]
            self.g[y][x] = min(BODY, key=lambda n: sum((a - b) ** 2 for a, b in zip(RGB[n], RGB[pen])))
        if pts:
            self.log.append(f"{len(pts)} pixels in his lilac greys or lining maroon, which his ROM body is not drawn in, set to his nearest body pens")

    def shadow(self, spans, why):
        """His ROM poses stand on a two-row black ground shadow (rows 62-63
        here). Where the inpainting lifted his foot off it, it is drawn back in
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


# (pose, candidate, mask, what the mask covers and what was drawn there)
INPAINTED = [
    ("crouching_punch_1", "08cbf189-0.png", "inpaint-mask-punch-windup.png",
     "his front arm inpainted (x31-52, y16-38), the hand drawn in before his chest, open and upright, as his jab draws it in (0x1120)"),
    ("crouching_punch_2", "2f6497ec-0.png", "inpaint-mask-punch.png",
     "his front arm inpainted (x32-95, y14-38), driven straight out at chest height with the open palm forward, as his jab strikes (0x1320)"),
    ("crouching_kick_chamber-round-1", "f92fa8f1-0.png", "inpaint-mask-kick-chamber.png",
     "his front leg inpainted from the knee down (x30-66, y38-63), the foot drawn up off the floor under the knee, toes down"),
    ("crouching_kick-round-1", "240bbc1e-0.png", "inpaint-mask-kick.png",
     "his front leg inpainted from the knee down (x30-95, y38-63), kicked straight out at shin height, the foot flexed with the toes up"),
]
# Where the crouch's front foot stood, the shadow it stood on; under the kick
# the shadow runs on under the leg, as under his ROM kick (0x10c4).
SHADOWS = {
    "crouching_kick_chamber-round-1": ({62: (19, 45), 63: (18, 46)}, "where his front foot stood in the crouch, now lifted, as his ROM chamber (0x10c0) keeps it under the raised foot"),
    "crouching_kick-round-1": ({62: (19, 50), 63: (18, 62)}, "run on under the kicking leg, as under his ROM kick (0x10c4)"),
}

for name, source, mask, what in INPAINTED:
    grid, near = load_near(source, mask)
    p = Pose(name, grid)
    p.log.append(f"{what}; the rest is his ROM crouch, pixel for pixel; {near} pixels the inpainting drew off his palette, or in his lilac greys, set to his nearest body pens")
    p.islands()
    if name in SHADOWS:
        p.shadow(*SHADOWS[name])
    p.save(source)

# The second round (user, 2026-11-03: the kick should not come from a kneel,
# and the chamber needs another pose). The kick squats on his bent rear foot,
# as Ryu's crouching kick (0x03a0) does; its chamber is the same picture with
# the front leg inpainted (x64-119, y42-75), so his body and rear leg are the
# kick's, pixel for pixel. His shadow is two rows (74-75) on this canvas.
grid, _ = load_near("edae4568-0.png")
p = Pose("crouching_kick", grid)
p.body_pens()
p.islands()
p.save("edae4568-0.png")

# Its chamber was rejected again and then dropped (user, 2026-11-03: the
# crouching kick plays crouch, kick, crouch); it is written as
# crouching_kick_chamber-round-2 for the record.
grid, near = load_near("e34013b9-0.png", "inpaint-mask-squat-chamber.png")
p = Pose("crouching_kick_chamber-round-2", grid)
p.log.append(f"his front leg inpainted (x64-119, y42-75), the knee drawn up before him and the foot lifted under it, toes down; the rest is the kick (edae4568, candidate 0), pixel for pixel; {near} pixels the inpainting drew off his palette set to his nearest body pens")
p.body_pens()
p.islands()
p.shadow({74: (27, 72), 75: (29, 76)}, "run on under the lifted foot, as his ROM chamber (0x10c0) keeps it under the raised foot")
p.save("e34013b9-0.png")
