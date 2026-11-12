"""Touch up Mike's ground-set candidates (AP15, #179) by hand.

usage: ground-touch-ups.py <out folder>

- The body hook, the low body hook and the overhand right are his ROM
  poses with one arm inpainted (make-inpaint-1.py makes the sources and
  masks): outside the mask every pixel is his ROM pose's own.
- Inpainting takes no palette, so its pixels inside the mask take his
  nearest pens. Stray specks floating off his body are cleared.
- His crouching block, crouching hit, crouching uppercut, throw, thrown
  set and get-up are built from his ROM poses in the recipe and need no
  file here.

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
PAL = ["#000000", "#664400", "#775500", "#996655", "#bb8877", "#cc9988", "#bbbbbb", "#777777",
       "#550000", "#880000", "#bb0000", "#0099bb", "#007799", "#005577", "#004455"]
RGB = [tuple(int(h[i:i + 2], 16) for i in (1, 3, 5)) for h in PAL]
IDX = {c: n for n, c in enumerate(RGB)}
N8 = ((1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1))
N4 = ((1, 0), (-1, 0), (0, 1), (0, -1))


def load_near(source):
    """Every opaque pixel as his pen: exact, or his nearest."""
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
                exact = min(range(15), key=lambda n: sum((a - b) ** 2 for a, b in zip(RGB[n], c[:3])))
                near += 1
            row.append(exact)
        grid.append(row)
    return grid, near


def load_mask(name):
    im = Image.open(os.path.join(HERE, name)).convert("L")
    px = im.load()
    return {(x, y) for y in range(im.height) for x in range(im.width) if px[x, y] > 127}


class Pose:
    def __init__(self, name, grid):
        self.name, self.g = name, grid
        self.before = [row[:] for row in grid]
        self.H, self.W = len(grid), len(grid[0])
        self.log = []

    def at(self, x, y):
        return self.g[y][x] if 0 <= x < self.W and 0 <= y < self.H else 15

    def islands(self, smallest=6):
        """Opaque groups of fewer than `smallest` pixels touching nothing else."""
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

    def lone(self, inside, keep=(0,), where="inside the mask"):
        """Inside the mask: a pixel none of whose four neighbours shares its
        pen, all four opaque and three or more of one other pen, takes that
        pen. Black (his outlines and eyes) is kept."""
        changed = 0
        for _ in range(2):
            for x, y in sorted(inside):
                p = self.g[y][x]
                if p == 15 or p in keep:
                    continue
                ns = [self.at(x + dx, y + dy) for dx, dy in N4]
                if 15 in ns or p in ns:
                    continue
                best = max(set(ns), key=ns.count)
                if ns.count(best) >= 3:
                    self.g[y][x] = best
                    changed += 1
        if changed:
            self.log.append(f"single pixels {where} that no neighbour shares, set to the pen around them ({changed} pixels), so the inpainted shading runs in bands as his ROM shading does")

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


INPAINTED = [
    ("body_hook", "11d68ba1-0.png", "inpaint-mask-body-hook.png",
     "his near arm inpainted from the shoulder (x18-49, y27-49, and before him x50-95, y33-61), swung out and driven forward at the opponent's body, the fist taped; the rest is his ROM lunging hook's wind-up (0x1409), pixel for pixel"),
    ("low_body_hook", "9bd0dd17-0.png", "inpaint-mask-low-body-hook.png",
     "his near arm inpainted from the shoulder (x20-43, y22-45, and before him x44-95, y27-49), driven forward low at the opponent's body, as thick as his other arm, the fist taped (the second try: the first, 4212da96, drew it thin); the rest is his ROM ducking punch's wind-up (0x15aa), pixel for pixel"),
    ("overhand", "c9271648-0.png", "inpaint-mask-overhand.png",
     "his striking arm inpainted from the shoulder (x48-57, y20-43, and x58-95, y0-55), looped over the top, the elbow high, the forearm driven down and forward to the fist; the rest is his ROM lunging hook's strike (0x1560), pixel for pixel"),
]
for name, source, mask, what in INPAINTED:
    grid, near = load_near(source)
    p = Pose(name + "-round-1", grid)
    p.log.append(f"{what}; {near} pixels the inpainting drew off his palette set to his nearest pens")
    p.lone(load_mask(mask))
    p.islands()
    p.save(source)


# Round two (the maintainer, 2026-11-06): the punches read as the wrong punches and
# their arms looked pasted on, so each strike was inpainted again with his
# shoulder, upper arm and chest in the mask; the pieced crouching block and
# hit were wrong at the belt, so the join was inpainted; the throw and the
# rising pose were new art. make-inpaint-2.py and make-headbutt.py make the
# sources and masks; every mask keeps his ROM head.
ROUND_2 = [
    ("body_hook", "a90732cd-0.png", "inpaint-mask-body-hook-2.png",
     "his shoulders, chest and near arm inpainted (x8-95, y16-55, his head and guard fist kept: x36-63, y0-32), the chest turned into the punch and the arm hooked round, elbow out, the fist arcing in at his belt; the rest is his ROM lunging hook's wind-up (0x1409), pixel for pixel"),
    ("low_body_hook", "79012fad-0.png", "inpaint-mask-low-body-hook-2.png",
     "his shoulders, back and near arm inpainted (x8-95, y8-47, his head kept: x42-65, y0-26), the arm hooked round at belt height, as thick as his ROM arms (the second try this round: 2f12c61c drew it thin, with a small fist); the rest is his ROM ducking punch's wind-up (0x15aa), pixel for pixel"),
    ("overhand", "70017147-0.png", "inpaint-mask-overhand-2.png",
     "his striking shoulder, upper back and arm inpainted (x40-95, y0-63, the back of his head kept: x28-55, y10-30), the shoulder rolled up, the elbow high and the forearm chopping down and forward; the rest is his ROM lunging hook's strike (0x1560), pixel for pixel"),
    ("crouching_block", "ce550f4a-0.png", "inpaint-mask-crouching-block-2.png",
     "the join at his belt inpainted (x0-63, y50-71): his shirt's hem as wide as his hips, his belt and buckle, the top of his jeans into his bent thighs; above it his ROM guard (0x1648), below it his ROM crouch's legs (0x156a), pixel for pixel"),
    ("crouching_hit", "1b7e453f-0.png", "inpaint-mask-crouching-hit-2.png",
     "the join at his belt inpainted (x0-71, y58-81): his shirt's hem, belt and buckle and the top of his jeans; above it his ROM hit pose's upper body (0x160c), below it his ROM crouch's legs (0x156a), pixel for pixel"),
    ("clinch", "5b255020-0.png", "inpaint-mask-clinch.png",
     "his shoulders, chest and both arms inpainted (x8-111, y16-59, his head kept: x38-59, y4-30), reaching forward at shoulder height, open hands gripping a collar; the rest is his ROM straight's step-in (0x147b), pixel for pixel. The first try (0edcfd21) drew the opponent as well"),
    ("headbutt", "e875e8d1-0.png", "inpaint-mask-headbutt.png",
     "the clinch (5b255020) with his ROM head moved 8 pixels forward and 6 down, and his shoulders, neck and arms round it inpainted (x16-103, y2-49, every pixel of the moved head kept), hunched in behind the head; below the mask his ROM straight's step-in, pixel for pixel"),
    ("shove", "68e6c375-0.png", "inpaint-mask-shove.png",
     "his shoulders, chest and both arms inpainted (x8-111, y16-59, his head kept: x38-59, y4-30), thrust straight out at chest height with both palms open; the rest is his ROM straight's step-in (0x147b), pixel for pixel"),
    ("rising", "9cc3b908-0.png", "inpaint-mask-rising.png",
     "everything below his head inpainted (x0-95, y8-63, his head kept: x58-81, y0-26): kneeling, his near hand pressed on the floor, pushing himself up; his head is his ROM ducking punch's wind-up (0x15aa), pixel for pixel"),
]
for name, source, mask, what in ROUND_2:
    grid, near = load_near(source)
    p = Pose(name + "-round-2", grid)
    p.log.append(f"{what}; {near} pixels the inpainting drew off his palette set to his nearest pens")
    # A light grey line down the outer edge of his shirt (the headbutt's
    # back): his ROM shirt is outlined in black.
    edge = [(x, y) for y in range(p.H) for x in range(p.W) if p.g[y][x] in (6, 7) and p.at(x - 1, y) == 15 and p.at(x + 1, y) in (0, 8, 9, 10)]
    for x, y in edge:
        p.g[y][x] = 0
    if edge:
        p.log.append(f"light grey pixels on the outer edge of his shirt set to his black outline ({len(edge)} pixels)")
    p.lone(load_mask(mask))
    p.islands()
    p.save(source)

# The crouching uppercut: round one's whole generation (f1b0a750, candidate
# 2: upright on his crouched legs, fist driven up) with his ROM head laid
# back by uppercut-head.py. Two inpainted tries this round were not kept
# (f6d305a8 drew a second head; 377ec8cb drew the arm apart from his body).
grid, near = load_near("uppercut-with-rom-head.png")
p = Pose("crouching_uppercut-round-2", grid)
p.log.append("his ROM head from his victory pose (0x166c), upright, facing right, mouth open, laid over the generated head 26 pixels right and 8 up (uppercut-head.py): the generated head's hair, skin and outline cleared first, 5 holes inside his outline given his shirt's red, and 3 collar pixels left sticking out before his jaw cleared")
everywhere = {(x, y) for y in range(p.H) for x in range(p.W) if not (51 <= x < 72 and 24 <= y < 47)}
p.lone(everywhere, where="outside his ROM head")
p.islands()
p.save("f1b0a750-2.png")
# Round three (the maintainer, 2026-11-06): the strike arms did not reach out as
# they should, the bodies looked off, the throw's joins were messy, and the
# crouching block and hit still did not read as one body at the waist.
# make-inpaint-3.py lays his ROM forearm and fist where each punch lands and
# inpaints only the join to his ROM shoulder; the block and hit join their
# upper body to the crouch at the chest. make-headbutt.py (round three)
# moves his ROM head on the clinch and inpaints only his neck and shoulders.
ROUND_3 = [
    ("crouching_block", "e5582605-0.png", "inpaint-mask-crouching-block-3.png",
     "his guard (0x1648) down to the chest laid on his crouch (0x156a, without its forearms), and the band where they meet inpainted (y50-63) so his shirt runs into his hips and jeans as one leaning body; above and below the band his ROM pixels"),
    ("crouching_hit", "c6755374-0.png", "inpaint-mask-crouching-hit-3.png",
     "his hit pose (0x160c) down to the chest laid on his crouch (0x156a, without its forearms), and the band where they meet inpainted (y56-69); above and below the band his ROM pixels"),
    ("body_hook", "1fa37e96-0.png", "inpaint-mask-body-hook-3.png",
     "his hook's wind-up (0x1409) with its folded near forearm taken out, his jab's forearm and fist (0x1470, x46-79, y17-35) laid at the opponent's stomach height (from x52, y32), and only the upper arm and elbow joining it to his shoulder inpainted (x16-57, y26-51); the rest is ROM pixels"),
    ("low_body_hook", "b03d6f43-0.png", "inpaint-mask-low-body-hook-3.png",
     "his ducking wind-up (0x15aa) with its low forearm taken out, his jab's forearm and fist (0x1470) laid at belt height (from x52, y24), and only the upper arm and elbow joining it inpainted (x20-65, y18-43, his head kept); the rest is ROM pixels"),
    ("overhand", "06cec615-0.png", "inpaint-mask-overhand-3b.png",
     "his jab's strike (0x1470), facing us, with its arm taken out, the jab's forearm from the elbow (x54-79, y17-35) laid 8 pixels lower and further out (from x58, y25), and only his shoulder and upper arm inpainted (x30-61, y4-41), rising over the top to it; the rest is ROM pixels. The first try this round (cddbbda3, forearm laid nearer) drew the upper arm arching past the forearm, so the two did not meet"),
    ("clinch", "2df5bbb9-0.png", "inpaint-mask-clinch-3.png",
     "his straight's step-in (0x147b) with its fists taken out, two of his jab's forearms and fists (0x1470) laid reaching out at collar height (from x52, y22 and x56, y28), and only his shoulders, chest and upper arms joining them inpainted (x14-57, y18-45, his head kept); the rest is ROM pixels"),
    ("headbutt", "2bcda950-0.png", "inpaint-mask-headbutt-3.png",
     "the clinch (2df5bbb9) with his ROM head moved 8 pixels forward and 8 down, over his gripping forearms, and only his neck and shoulders behind it inpainted (where the head was, and x30-55, y18-43); the forearms laid from his ROM jab are kept"),
]
for name, source, mask, what in ROUND_3:
    grid, near = load_near(source)
    p = Pose(name + "-round-3", grid)
    p.log.append(f"{what}; {near} pixels the inpainting drew off his palette set to his nearest pens")
    edge = [(x, y) for y in range(p.H) for x in range(p.W) if p.g[y][x] in (6, 7) and p.at(x - 1, y) == 15 and p.at(x + 1, y) in (0, 8, 9, 10)]
    for x, y in edge:
        p.g[y][x] = 0
    if edge:
        p.log.append(f"light grey pixels on the outer edge of his shirt set to his black outline ({len(edge)} pixels)")
    p.lone(load_mask(mask))
    p.islands()
    p.save(source)
# The crouching uppercut, round three. Two inpainted tries this round were
# not kept (28d6a0d1 joined the arm to his chest fist and left the laid
# forearm floating; ec6cd544 drew the arm across his stomach), so it was
# generated whole (44eb451d, candidate 0: the arm bent sharply, the fist
# driven up before his face, his chest toward us), with his ROM head laid
# back by uppercut-head-3.py.
grid, near = load_near("uppercut-3-with-rom-head.png")
p = Pose("crouching_uppercut-round-3", grid)
p.log.append("his ROM head from his victory pose (0x166c), upright, facing right, mouth open, laid over the generated head 28 pixels right and 16 up (uppercut-head-3.py): the generated head's hair, skin and outline cleared first (176 pixels), the generated fist and forearm drawn back over it where they pass before his face, 19 holes inside his outline given his shirt's red, and 2 one-pixel spurs left of his head cleared")
everywhere = {(x, y) for y in range(p.H) for x in range(p.W) if not (53 <= x < 74 and 16 <= y < 39)}
p.lone(everywhere, where="outside his ROM head")
p.islands()
p.save("44eb451d-0.png")
# Round four (the maintainer, 2026-11-06, round three): the punches still looked
# straight and their shoulders did not join; the throw's joins were messy;
# the crouching block's join still did not read; the crouching uppercut was a
# different size from his crouch. make-inpaint-4.py makes the sources and
# masks. For the three punches his jab's ROM fist was laid where the punch
# lands as a guide, but each inpainting drew its own whole bent arm and fist
# and left the laid fist floating apart, so the laid fist is taken out again.
sys.path.insert(0, sys.argv[2]) if len(sys.argv) > 2 else None
import rompose as rp  # noqa: E402
JAB = rp.grid(0x1470, 1, 3, 2)
FIST = [68, 17, 12, 19]
ROUND_4 = [
    ("body_hook", "7391c683-0.png", "inpaint-mask-body-hook-4.png", (74, 36),
     "his near arm inpainted whole from the shoulder (x12-75, y18-61, his head and guard fist kept), bent at the elbow with the fist driven in at his belt; the rest is his ROM hook's wind-up (0x1409)"),
    ("low_body_hook", "f1f7ea13-0.png", "inpaint-mask-low-body-hook-4.png", (72, 28),
     "his near arm inpainted whole from the shoulder (x10-73, y8-51, his head kept), the elbow bent and the fist driven forward low; the rest is his ROM ducking wind-up (0x15aa)"),
    ("overhand", "457bbf93-0.png", "inpaint-mask-overhand-4.png", (70, 30),
     "his striking arm inpainted whole from the shoulder (x30-71, y0-51, his head, guard fist and body kept), the elbow high and the forearm chopping down and forward over the top; the rest is his ROM jab strike (0x1470), his chest toward us"),
    ("crouching_block", "b28c268a-0.png", "inpaint-mask-crouching-block-4.png", None,
     "his approved crouch (0x156a) with only his forearms and fists inpainted (x28-71, y0-45 of the padded canvas, his head down to his cheek and everything below his arms kept), raised into a high guard before his face; no waist join at all"),
    ("crouching_uppercut", "e3b97ba4-0.png", "inpaint-mask-uppercut-4b.png", None,
     "his ducking wind-up (0x15aa, his crouch's scale) with its low forearm taken out, the bent arm of round three's uppercut (44eb451d, candidate 0, x74-91, y16-49) laid upright beside his face, and only the upper arm joining them inpainted (x24-85, y36-61, his head, back and legs kept). The first try this round (93ffd720, the near arm inpainted whole) drew no arm"),
    ("clinch", "a745ea94-0.png", "inpaint-mask-clinch-4.png", None,
     "round three's clinch with only the joins inpainted again (x28-65, y16-49): his shoulders and upper arms into the forearms; his head, the forearms from the wrist on and everything below his chest kept"),
    ("headbutt", "84ba11dd-0.png", "inpaint-mask-headbutt-4.png", None,
     "round three's headbutt with only the joins inpainted again (x28-65, y16-49): his neck, shoulders and upper arms into the forearms; his moved head, the forearms from the wrist on and everything below his chest kept"),
]
for name, source, mask, pinned, what in ROUND_4:
    grid, near = load_near(source)
    p = Pose(name, grid)
    p.log.append(f"{what}; {near} pixels the inpainting drew off his palette set to his nearest pens")
    if pinned:
        x0, y0, cw, ch = FIST
        gone = 0
        for y in range(ch):
            for x in range(cw):
                if JAB[y0 + y][x0 + x] != 15:
                    p.g[pinned[1] + y][pinned[0] + x] = 15
                    gone += 1
        p.log.append(f"his jab's fist, laid at x{pinned[0]}, y{pinned[1]} as a guide and left floating apart from the inpainted arm, taken out again ({gone} pixels)")
    edge = [(x, y) for y in range(p.H) for x in range(p.W) if p.g[y][x] in (6, 7) and p.at(x - 1, y) == 15 and p.at(x + 1, y) in (0, 8, 9, 10)]
    for x, y in edge:
        p.g[y][x] = 0
    if edge:
        p.log.append(f"light grey pixels on the outer edge of his shirt set to his black outline ({len(edge)} pixels)")
    p.lone(load_mask(mask))
    p.islands()
    p.save(source)