"""Touch up Sagat's ground-set candidates (AP7, #171) by hand.

usage: ground-touch-ups.py <out folder>

Reads the raw candidates beside this script and writes, for each pose,
<out>/<pose>.png (RGBA, his colours only), <out>/<pose>.json (the
touch-up list) and <out>/<pose>-sheet.png (before, after, and the changed
pixels in green). Coordinates are the candidate's 128x128 canvas.
"""
import json
import os
import sys

from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = sys.argv[1]
PAL = ["#000000", "#664400", "#996644", "#cc9977", "#ddaa88", "#ddccaa", "#bbbbbb", "#777777",
       "#999999", "#555555", "#444444", "#440044", "#660099", "#990088", "#bb00bb"]
RGB = [tuple(int(h[i:i + 2], 16) for i in (1, 3, 5)) for h in PAL]
IDX = {c: n for n, c in enumerate(RGB)}
BLACK, OUTLINE, PATCH = 0, 1, 10
SKIN, TAPE, SHORTS = {1, 2, 3, 4, 5}, {6, 7, 8, 9}, {11, 12, 13, 14}
N4 = ((1, 0), (-1, 0), (0, 1), (0, -1))
N8 = N4 + ((1, 1), (-1, -1), (1, -1), (-1, 1))


def load(name):
    im = Image.open(os.path.join(HERE, name)).convert("RGBA")
    px = im.load()
    return [[(IDX[px[x, y][:3]] if px[x, y][3] >= 128 else 15) for x in range(im.width)] for y in range(im.height)]


class Pose:
    def __init__(self, name, grid):
        self.name, self.g = name, grid
        self.before = [row[:] for row in grid]
        self.H, self.W = len(grid), len(grid[0])
        self.log = []

    def at(self, x, y, grid=None):
        grid = grid or self.g
        return grid[y][x] if 0 <= x < self.W and 0 <= y < self.H else 15

    def put(self, points, pen, why):
        changed = [(x, y) for x, y in points if self.g[y][x] != pen]
        for x, y in changed:
            self.g[y][x] = pen
        if changed:
            self.log.append(f"{why}: {len(changed)} pixels to pen {pen} ({PAL[pen]})")

    def crown(self):
        return min(y for y in range(self.H) if any(p != 15 for p in self.g[y]))

    def eyepatch(self, head_rows):
        """His ROM eyepatch is an oblong of his dark grey (pen 10) inside a
        one-pixel black rim; the generator fills it with black. Black pixels
        in his head whose four neighbours are all black are the patch's
        inside: they become pen 10, and the rim and the strap stay black."""
        top = self.crown()
        inside = [(x, y) for y in range(top, top + head_rows) for x in range(self.W)
                  if self.g[y][x] == BLACK and all(self.at(x + dx, y + dy, self.before) == BLACK for dx, dy in N4)]
        self.put(inside, PATCH, f"the eyepatch's inside, rows {top}-{top + head_rows - 1}, in his dark grey with its black rim kept, as in his ROM poses")

    def shadow(self, under_kick=False):
        """Every ROM pose of his stands on a two-row black ground shadow,
        drawn in his feet's last two rows wherever they leave a gap, as the
        approved crouch's is. Under his ROM kicks it runs on under the
        kicking foot."""
        bottom = max(y for y in range(self.H) if any(p != 15 for p in self.g[y]))
        if sum(1 for p in self.g[bottom] if p == BLACK) > 20:
            return
        feet = [x for x in range(self.W) if any(self.g[y][x] != 15 for y in range(bottom - 4, bottom + 1))]
        x0, x1 = min(feet), max(feet)
        if under_kick:
            x1 = max(x for x in range(self.W) if any(self.g[y][x] != 15 for y in range(self.H)))
        pts = [(x, y) for y in (bottom - 1, bottom) for x in range(x0, x1 + 1) if self.g[y][x] == 15]
        self.put(pts, BLACK, f"ground shadow in black on rows {bottom - 1}-{bottom}, x{x0}-{x1}, in the gaps between his feet, as under his ROM poses")

    def specks(self, head_rows=22, only=None):
        """Dark grey or dark purple specks on his skin's outline become his
        brown outline, as the crouch's did; the tape and shorts keep theirs,
        and so does his head, whose eyes and face lines are dark pens. With
        `only` (x0, y0, x1, y1), nothing outside that box changes."""
        top = self.crown()
        keep = {(x, y) for y in range(top, top + head_rows) for x in range(self.W)}
        if only:
            keep |= {(x, y) for y in range(self.H) for x in range(self.W)
                     if not (only[0] <= x <= only[2] and only[1] <= y <= only[3])}

        def near(x, y, pens, r):
            return any(self.at(x + dx, y + dy, self.before) in pens
                       for dy in range(-r, r + 1) for dx in range(-r, r + 1) if dx or dy)
        pts = [(x, y) for y in range(self.H) for x in range(self.W)
               if (x, y) not in keep and self.before[y][x] in (PATCH, 11) and self.g[y][x] == self.before[y][x]
               and not near(x, y, TAPE, 1) and not near(x, y, SHORTS - {11}, 1) and not near(x, y, {BLACK}, 1)
               and near(x, y, SKIN - {1}, 1)]
        self.put(pts, OUTLINE, "dark grey and dark purple specks on his skin's outline, to his brown outline")

    def one_stripe(self):
        """His shorts carry one light stripe, down the side (the leftmost
        light run on each row of the shorts). The generator added a second,
        angled one, as it did on the crouch: its light greys, and the dark
        grey edge touching them, go to the shorts' purples round them."""
        light, purple = {6, 8}, SHORTS
        stray = []
        for y in range(self.H):
            if sum(1 for p in self.g[y] if p in purple) < 6:
                continue
            runs, x = [], 0
            while x < self.W:
                if self.g[y][x] in light and any(self.at(x + dx, y + dy) in purple for dx, dy in N8):
                    x0 = x
                    while x < self.W and self.g[y][x] in light:
                        x += 1
                    runs.append(range(x0, x))
                x += 1
            stray += [(x, y) for run in runs[1:] for x in run]
        edge = [(x + dx, y + dy) for x, y in stray for dx, dy in N4 if self.at(x + dx, y + dy) == PATCH]
        for x, y in stray + edge:
            around = [self.at(x + dx, y + dy) for dx, dy in N8]
            purples = [p for p in around if p in purple]
            self.g[y][x] = max(set(purples), key=purples.count) if purples else 12
        if stray:
            ys = [y for _, y in stray]
            self.log.append(f"the second, angled stripe across his shorts ({len(stray)} light pixels and {len(set(edge))} of its dark edge, rows {min(ys)}-{max(ys)}) painted out in the shorts' purples; the side stripe stays")

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


GENERATED = {
    "crouching_hit": "e5e840cf-0.png",
    "crouching_punch": "2feeb762-0.png",
    "sweep_1": "23e73d7e-0.png",
    "sweep_2": "e95d0232-2.png",
    "sweep_3": "e95d0232-1.png",
}
for name, source in GENERATED.items():
    p = Pose(name, load(source))
    p.eyepatch(22)
    p.specks()
    p.shadow()
    p.save(source)

# The crouching kick: the spike's kick candidates came back on the crouch
# candidate's body, a few pixels off. Above his waistband each takes the
# approved crouch (touched, e57623f6-1-touched.png) placed at that offset,
# so his head, eyepatch and guard match the crouch exactly; below it the
# kick's own legs stay.
crouch = load("e57623f6-1-touched.png")
waist = min(y for y in range(128) if any(p in (12, 13, 14) for p in crouch[y]))
for name, source, (dx, dy) in (("crouching_kick_1", "5116ff54-1.png", (-8, -5)), ("crouching_kick_2", "5116ff54-2.png", (-9, -6))):
    p = Pose(name, load(source))
    cleared = [(x, y) for y in range(max(0, waist + dy)) for x in range(128) if p.g[y][x] != 15]
    for x, y in cleared:
        p.g[y][x] = 15
    pasted = 0
    for y in range(waist):
        for x in range(128):
            if crouch[y][x] != 15 and 0 <= x + dx < 128 and 0 <= y + dy < 128:
                p.g[y + dy][x + dx] = crouch[y][x]
                pasted += 1
    p.log.append(f"above his waistband (rows 0-{waist + dy - 1}) the approved crouch's upper body, touched, placed {dx},{dy} from the crouch candidate: {pasted} pixels, so his head, eyepatch and guard are the crouch's")
    p.one_stripe()
    p.specks()
    p.shadow(under_kick=True)
    p.save(source)

# The kick's chamber: the approved crouch itself, padded to 96x96 at (4, 3),
# with only his front leg below mid-thigh repainted by inpainting (job
# e4be1b81, mask inpaint-mask-front-leg-2.png, which spares his shorts):
# everything outside the mask is the crouch's own pixels. Inpainting has no
# palette, so its pixels take his nearest pens first.
source = "e4be1b81-0.png"
im = Image.open(os.path.join(HERE, source)).convert("RGBA")
px = im.load()
near, grid = 0, []
for y in range(im.height):
    row = []
    for x in range(im.width):
        c = px[x, y]
        if c[3] < 128:
            row.append(15)
        elif c[:3] in IDX:
            row.append(IDX[c[:3]])
        else:
            near += 1
            row.append(min(range(15), key=lambda n: sum((a - b) ** 2 for a, b in zip(RGB[n], c[:3]))))
    grid.append(row)
p = Pose("crouching_kick_chamber", grid)
p.log.append(f"inpainted his front leg below mid-thigh only (x38-80, y60-91 of the padded crouch, not his shorts): the shin tucked back under the knee and the foot lifted off the floor, toes down; the rest is the approved crouch, pixel for pixel; {near} pixels the inpainting drew off his palette set to his nearest pens")
p.specks(only=(38, 60, 80, 91))
p.shadow(under_kick=True)
p.save(source)
