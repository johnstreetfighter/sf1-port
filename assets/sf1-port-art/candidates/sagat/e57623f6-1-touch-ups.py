"""Touch up the spike's crouch candidate (job e57623f6, candidate 1) by hand.

Writes <scratch>/ap5/crouch_touched.png (RGBA, Sagat's colours only), a
before/after sheet and the touch-up list as JSON. Canvas coordinates are the
candidate's 128x128 canvas.
"""
import json
import os
import sys

from PIL import Image

S = sys.argv[1]
PAL = ["#000000", "#664400", "#996644", "#cc9977", "#ddaa88", "#ddccaa", "#bbbbbb", "#777777",
       "#999999", "#555555", "#444444", "#440044", "#660099", "#990088", "#bb00bb"]
RGB = [tuple(int(h[i:i + 2], 16) for i in (1, 3, 5)) for h in PAL]
IDX = {c: n for n, c in enumerate(RGB)}

im = Image.open(os.path.join(S, "gen", "crouch_1.png")).convert("RGBA")
W, H = im.size
px = im.load()
g = [[(IDX[px[x, y][:3]] if px[x, y][3] else 15) for x in range(W)] for y in range(H)]
before = [row[:] for row in g]
log = []


def put(points, pen, why):
    changed = [(x, y) for x, y in points if g[y][x] != pen]
    for x, y in changed:
        g[y][x] = pen
    log.append(f"{why}: {len(changed)} pixels to pen {pen} ({PAL[pen]})")


# 1. The eyepatch pasted from his ROM stance: its black and pen-10 pixels
#    (a slanted oblong, x34-38 y7-11 of the stance reference) moved to his
#    eye at x70-74 y35-39. His near hand is in front of his face: the fist's
#    outline (x75,y32 down to x72,y35) and the forearm's tape hide the
#    patch's top edge and right side, so only the pixels on his face are
#    drawn.
TAPE_PENS = {6, 7, 8, 9}


def on_glove(x, y):
    q = before[y][x]
    if q in TAPE_PENS:
        return True
    n4 = [before[y + dy][x + dx] for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))]
    return q in (1, 10) and any(n in TAPE_PENS for n in n4)


FIST_OUTLINE = {(75, 32), (74, 33), (73, 34), (74, 34), (72, 35), (73, 35)}
rom = Image.open(os.path.join(S, "ap5", "stance.png")).convert("RGBA")
rp = rom.load()
PATCH_FROM, PATCH_AT = (34, 7), (70, 35)
pasted = {0: [], 10: []}
hidden = 0
for y in range(5):
    for x in range(5):
        c = rp[PATCH_FROM[0] + x, PATCH_FROM[1] + y]
        if c[3] and IDX[c[:3]] in pasted:
            at = (PATCH_AT[0] + x, PATCH_AT[1] + y)
            if at in FIST_OUTLINE or on_glove(*at):
                hidden += 1
            else:
                pasted[IDX[c[:3]]].append(at)
put(pasted[0], 0, f"eyepatch rim pasted from his ROM stance (stance x34-38 y7-11 to x70-74 y35-39; {hidden} pixels hidden behind his hand), his black")
put(pasted[10], 10, "eyepatch centre pasted from his ROM stance, pen 10")
# The strap: drawn as a line of his lightest skin from the top of his head
# to his eye; in the ROM it is a black line rising straight from the
# patch's top-right corner, just inside his outline. Here it shows above
# his fist and goes behind it at y33.
STRAP = [(74, 30), (74, 31), (74, 32)]
put(STRAP, 0, "the eyepatch strap (x74, y30-32) in black, as in his ROM poses")
# The rest of the light line the generator drew, to the skin round it.
for x, y in [(72, 33), (73, 33), (72, 34), (71, 35), (70, 36)]:
    around = [before[y + dy][x + dx] for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))]
    skin = [p for p in around if p in (2, 3, 4)]
    put([(x, y)], max(set(skin), key=skin.count) if skin else 3, f"the generator's light strap line at x{x} y{y} to the skin round it")

# 1b. The shorts carry one stripe, down the side. The crouch added a second,
#     angled across them (cream and light grey, x38-44 y80-85): back to the
#     shorts' purple of the pixels round it.
EXTRA_STRIPE = [(43, 80), (44, 80), (41, 81), (42, 81), (40, 82), (41, 82), (39, 83), (40, 83),
                (38, 84), (39, 84), (38, 85)]
for x, y in EXTRA_STRIPE:
    around = [before[y + dy][x + dx] for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1))]
    purples = [p for p in around if p in (11, 12, 13, 14)]
    g[y][x] = max(set(purples), key=purples.count) if purples else 12
log.append(f"the second, angled stripe across his shorts ({len(EXTRA_STRIPE)} pixels, x38-44 y80-85) painted out in the shorts' purples")

# 2. The light streak between his forearms becomes a dark seam.
put([(82, y) for y in range(45, 51) if before[y][82] == 5], 1, "the light streak between his forearms (x82, y45-50) darkened")

# 3. Grey and dark purple specks the generator used for the black it lacked.
SKIN, TAPE, SHORTS = {1, 2, 3, 4, 5}, {6, 7, 8, 9}, {12, 13, 14}
STRIPE_X = {38}  # the shorts' side stripe keeps its grey outline


def near(x, y, pens, r=2):
    return any(0 <= x + dx < W and 0 <= y + dy < H and before[y + dy][x + dx] in pens
               for dy in range(-r, r + 1) for dx in range(-r, r + 1) if dx or dy)


patch = {(x, y) for x in range(70, 75) for y in range(35, 40)} | set(STRAP) | set(EXTRA_STRIPE)
skin_edge, shorts_in, tape_edge = [], [], []
for y in range(H):
    for x in range(W):
        p = before[y][x]
        if (x, y) in patch or x in STRIPE_X:
            continue
        if p == 10 and not near(x, y, TAPE):
            (shorts_in if near(x, y, SHORTS, 1) and not near(x, y, SKIN - {1}, 1) else skin_edge).append((x, y))
        if p == 11 and not near(x, y, SHORTS):
            (tape_edge if near(x, y, TAPE, 1) else skin_edge).append((x, y))
put(skin_edge, 1, "dark grey and dark purple specks on his skin's outline, to his brown outline")
put(tape_edge, 10, "dark purple specks on the tape's outline, to the tape's dark grey")
put(shorts_in, 11, "dark grey specks in the shorts, to the shorts' darkest purple")

# 4. The ground shadow every ROM pose of his carries: black under and
#    between his feet, down to the row his back foot stands on.
bottom = max(y for y in range(H) if any(g[y][x] != 15 for x in range(W)))
left_end = max(x for x in range(W) if g[bottom][x] != 15)
right = [x for x in range(W) if any(g[y][x] != 15 for y in range(bottom - 4, bottom))]
x1 = max(right)
shadow = [(x, y) for y in (bottom - 1, bottom) for x in range(left_end + 1, x1 + 1) if g[y][x] == 15]
put(shadow, 0, f"ground shadow in black on rows {bottom - 1}-{bottom}, x{left_end + 1}-{x1}, as under his ROM poses")

out = Image.new("RGBA", (W, H), (0, 0, 0, 0))
o = out.load()
for y in range(H):
    for x in range(W):
        if g[y][x] != 15:
            o[x, y] = RGB[g[y][x]] + (255,)
os.makedirs(os.path.join(S, "ap5"), exist_ok=True)
out.save(os.path.join(S, "ap5", "crouch_touched.png"))
json.dump(log, open(os.path.join(S, "ap5", "touch_ups.json"), "w"), indent=2)

# Before and after, 4x, changed pixels outlined on a third copy.
box = (30, 25, 98, 120)
def show(grid, mark=False):
    c = Image.new("RGBA", (W, H), (40, 40, 50, 255))
    p = c.load()
    for y in range(H):
        for x in range(W):
            if grid[y][x] != 15:
                p[x, y] = RGB[grid[y][x]] + (255,)
            if mark and grid[y][x] != before[y][x]:
                p[x, y] = (0, 255, 0, 255)
    return c.crop(box).resize(((box[2] - box[0]) * 5, (box[3] - box[1]) * 5), Image.NEAREST)
a, b, c = show(before), show(g), show(g, True)
sheet = Image.new("RGBA", (a.width * 3 + 40, a.height), (20, 20, 25, 255))
for i, im2 in enumerate((a, b, c)):
    sheet.paste(im2, (i * (a.width + 20), 0))
sheet.save(os.path.join(S, "ap5_touch_sheet.png"))
print("\n".join(log))
print("changed:", sum(1 for y in range(H) for x in range(W) if g[y][x] != before[y][x]))
