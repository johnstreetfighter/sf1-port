"""Touch up Joe's air-set and specials candidates (AP14, #178) by hand.

usage: air-touch-ups.py <out folder>

As ground-touch-ups.py, whose Pose steps this reuses: for each pose,
<out>/<pose>.png (RGBA, his colours only), <out>/<pose>.json (the touch-up
list) and <out>/<pose>-sheet.png.

- The jumping punch, diagonal jumping punch and jumping kick are his ROM
  top-of-jump tuck (0x1a88, padded to 96x80 at (8, 4):
  inpaint-source-jump-96x80.png) with one limb inpainted.
- The Dynamite Straight's strike is his ROM jump take-off (0x1884, padded to
  80x112 at (8, 16): inpaint-source-dynamite-80x112.png) with his front arm
  inpainted.
- The heavy jumping kick is generated whole; its upper body came back as his
  ROM high kick's (0x1826, rom-0x1826.png) three rows lower, all but the
  hair, so his ROM head is laid back over it. Airborne poses get no shadow.

Round two (user, 2026-11-06: the colours and pixels of the punch, kick, heavy
kick and Dynamite Straight messy; the kick's legs and the Dynamite
Straight's proportions wrong). Each is rebuilt on his ROM pixels so its
shading is his own:
- the jumping punch keeps round one's shoulder and upper arm, with his jab's
  forearm and fist (0x18e4, rom-0x18e4.png) in place of the inpainted ones;
- the jumping kick is his ROM tuck with his flying kick's kicking leg
  (0x1944, rom-0x1944.png) laid over it, no generation;
- the heavy jumping kick is his ROM high kick (0x1826) above the hip, with
  the generated tucked rear leg below it;
- the Dynamite Straight is a new whole generation (608b0142) whose head and
  legs are put back to his ROM take-off (0x1884, rom-0x1884.png).
Inside what is still generated, single pixels no neighbour shares are set
to the pen most of their neighbours have.

Round three (user, 2026-11-06):
- the jumping punch: its face had lost pixels (round two cleared right of
  x40 from row 10); his whole ROM head goes back;
- the jumping kick: back to round one's, whose rear leg read as both feet
  tucked; the rear leg and the hip behind it inpainted again as one bent
  leg (inpaint-source-jump-kick-3.png, inpaint-mask-jump-kick-3.png);
- the jumping diagonal punch: the tuck's folded upper arm still showed in
  his torso; his back inpainted without it (inpaint-source-jump-punch-down-3.png,
  inpaint-mask-jump-punch-down-3.png);
- the Rolling Sobat's kick: the kicking foot is skin in his ROM high kick;
  inpainted as his grey shoe (inpaint-source-sobat-96x96.png,
  inpaint-mask-sobat-foot.png), its ground shadow left off as in the hop.
An inpainting's pixels are held to the pens of what they draw: red to his
trouser pens, grey to his shoe pens, the rest to his skin and outline pens.
- the heavy jumping kick (user, 2026-11-06, after round three): its kicking
  foot, his ROM high kick's bare skin, takes the Rolling Sobat kick's grey
  shoe; both are the same ROM pose, so the shoe is laid on pixel for pixel.

Round four of the jumping kick (user, 2026-11-06, his call after three
rejections), by hand on round three's: the tucked leg showed two feet (the
ROM tuck's two shoes), so the lower one is erased; and the kicking foot
looked cut off, so it is redrawn whole as the grey shoe on his ROM flying
kick's straight leg (0x1944, rom-0x1944.png).
"""
import importlib.util
import os
import sys

from PIL import Image

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
Pose, load_near = ground.Pose, ground.load_near

INPAINTED = [
    ("jump_punch", "94e064c2-0.png", "his front arm inpainted from the shoulder (x18-95, y21-44, and right of his face x45-95, y0-20), driven straight out at shoulder height, the fist clenched; the rest is his ROM top-of-jump tuck (0x1a88), pixel for pixel. The second mask: the first (from x22, below his shoulder) left the arm floating, cut off from his shoulder"),
    ("jump_punch_down", "108d05ed-0.png", "his front arm inpainted from the shoulder (x18-95, y21-44, and before his knees x42-95, y45-79), driven down and forward at 45 degrees past his knees, the fist clenched; the rest is his ROM top-of-jump tuck (0x1a88), pixel for pixel"),
    ("jump_kick", "c8439da8-0.png", "his legs inpainted below the belt (x18-95, y39-79, sparing his rear shoe x0-19, y56-79), the front leg kicked straight out at waist height, the rear knee still tucked; the rest is his ROM top-of-jump tuck (0x1a88), pixel for pixel"),
    ("dynamite_straight", "da0d0e5e-0.png", "his front arm, his chin and the chest behind them inpainted (x18-79, y31-48, and right of his face x44-79, y0-30), the arm driven up and forward at 45 degrees, the fist above his head; the rest is his ROM jump take-off (0x1884), pixel for pixel, his hair and eyes included"),
]
# Round one. The user rejected five on 2026-11-06 (rounds two and three below): those
# are saved as <pose>-round-1.
REJECTED = {"jump_punch", "jump_kick", "dynamite_straight", "jump_kick_high", "jump_punch_down"}
for name, source, line in INPAINTED:
    grid, near = load_near(source)
    p = Pose(name + "-round-1" if name in REJECTED else name, grid)
    p.log.append(f"{line}; {near} pixels the inpainting drew off his palette set to his nearest pens")
    p.islands()
    p.save(source)

# The heavy jumping kick: candidate 2 of 4564fc20. Its upper body is his ROM
# high kick's pose (0x1826) moved three rows down, pen for pen, except the
# hair (dark red and skin pens in it, no black dots along its top). His ROM
# head, rows 0-8 and columns 0-16 of 0x1826, goes back over it.
source = "4564fc20-2.png"
grid, near = load_near(source)
p = Pose("jump_kick_high-round-1", grid)
if near:
    p.log.append(f"{near} pixels off his palette set to his nearest pens")
xs = [x for x in range(p.W) if any(p.g[y][x] != 15 for y in range(p.H))]
ys = [y for y in range(p.H) if any(p.g[y][x] != 15 for x in range(p.W))]
bx, by = xs[0], ys[0]
rom, _ = load_near("rom-0x1826.png")
changed = 0
for y in range(9):
    for x in range(17):
        if p.g[by + 3 + y][bx + x] != rom[y][x]:
            p.g[by + 3 + y][bx + x] = rom[y][x]
            changed += 1
p.log.append(f"his ROM head from his high kick (0x1826, rows 0-8, columns 0-16) laid over the generated one, three rows down where the generated upper body sits, as his body below it is that pose's: {changed} pixels, all in his hair and the back of his head")
p.islands()
p.save(source)


def despeckle(p, region):
    """Inside `region`, a pixel whose pen none of its four neighbours shares,
    with at least five opaque neighbours, takes the pen four or more of its
    eight neighbours have."""
    n = 0
    for _ in range(2):
        ch = []
        for x, y in sorted(region):
            v = p.g[y][x]
            if v == 15 or v in [p.at(x + dx, y + dy) for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))]:
                continue
            op = [p.at(x + dx, y + dy) for dx in (-1, 0, 1) for dy in (-1, 0, 1) if dx or dy]
            op = [q for q in op if q != 15]
            if len(op) < 5:
                continue
            best = max(sorted(set(op)), key=op.count)
            if op.count(best) >= 4:
                ch.append((x, y, best))
        for x, y, b in ch:
            p.g[y][x] = b
        n += len(ch)
    return n


def bbox(grid):
    xs = [x for x in range(len(grid[0])) if any(row[x] != 15 for row in grid)]
    ys = [y for y in range(len(grid)) if any(v != 15 for v in grid[y])]
    return xs[0], ys[0]


# Round two: the jumping punch. Round one's pose (94e064c2, 96x80); right of
# x40 its inpainted forearm and fist go, and his jab's (0x18e4 columns 40 on,
# rows 8-29) go in two rows lower, so the arm is his own length and shading.
grid, _ = load_near("94e064c2-0.png")
p = Pose("jump_punch-round-2", grid)
jab, _ = load_near("rom-0x18e4.png")
for y in range(10, 34):
    for x in range(40, p.W):
        p.g[y][x] = 15
for y in range(8, 30):
    for x in range(40, len(jab[0])):
        if jab[y][x] != 15:
            p.g[y + 2][x] = jab[y][x]
for y in range(27, 31):
    for x in range(40, 60):
        p.g[y][x] = 15
for x in range(40, 48):
    p.g[26][x] = 1
n = despeckle(p, {(x, y) for y in range(21, 45) for x in range(18, 40)})
p.log.append("round one's inpainted forearm and fist (right of x40) replaced by his ROM jab's (0x18e4, columns 40 on, rows 8-29, two rows down); the jab's chest below its arm left out and the arm's lower outline closed in his dark brown (pen 1, row 26, x40-47)")
p.log.append(f"{n} single pixels in the inpainted shoulder and chest set to the pen around them")
p.save("94e064c2-0.png")

# Round two: the jumping kick. His ROM tuck (inpaint-source-jump-96x80.png)
# with his flying kick's kicking leg (0x1944, columns 29 on, rows 36-63;
# above row 42 only from column 44, so his tucked forearm stays) laid over it
# four columns left and two rows down: the tuck's folded rear leg and shoe
# stay, the front leg kicks out at his hip.
grid, _ = load_near("inpaint-source-jump-96x80.png")
p = Pose("jump_kick-round-2", grid)
kick, _ = load_near("rom-0x1944.png")
for y in range(36, 64):
    for x in range(29, len(kick[0])):
        if (x >= 44 or y >= 42) and kick[y][x] != 15:
            p.g[y + 2][x - 4] = kick[y][x]
p.log.append("his ROM top-of-jump tuck (0x1a88) with the kicking leg of his ROM flying kick (0x1944, columns 29 on, rows 36-63, above row 42 only from column 44) laid over it at (-4, +2); every pixel is his ROM art's")
p.save("inpaint-source-jump-96x80.png")

# Round two: the heavy jumping kick. Candidate 2 of 4564fc20; above the hip
# (rows 0-50 of 0x1826, three rows down) every pixel is his ROM high kick's,
# replacing the generated copy and its stray pixels; below, the generated
# tucked rear leg stays. The canvas is the candidate's 128x128.
grid, _ = load_near("4564fc20-2.png")
p = Pose("jump_kick_high-round-2", grid)
bx, by = bbox(p.g)
high, _ = load_near("rom-0x1826.png")
for y in range(by, by + 3):
    for x in range(p.W):
        p.g[y][x] = 15
for y in range(51):
    for x in range(len(high[0])):
        p.g[by + 3 + y][bx + x] = high[y][x]
n = despeckle(p, {(x, y) for y in range(by + 54, p.H) for x in range(p.W)})
p.log.append("above his hip every pixel is his ROM high kick's (0x1826, rows 0-50, three rows below the candidate's top), the generated copy of it and the rows above it cleared; below, the generated tucked rear leg")
p.log.append(f"{n} single pixels in the generated rear leg set to the pen around them")
p.save("4564fc20-2.png")

# Round two: the Dynamite Straight. Candidate 0 of 608b0142 (128x128): its
# legs, belt and rear fist are his ROM take-off's (0x1884, rows 33 on, one
# row down) and its head his ROM head (rows 0-12, two rows down, over the
# generated one cleared); the torso and raised arm stay generated.
grid, _ = load_near("608b0142-0.png")
p = Pose("dynamite_straight", grid)
bx, by = bbox(p.g)
rise, _ = load_near("rom-0x1884.png")
for y in range(33, len(rise)):
    for x in range(len(rise[0]), 59):
        p.g[by + y + 1][bx + x] = 15
    for x in range(len(rise[0])):
        p.g[by + y + 1][bx + x] = rise[y][x]
for y in range(15):
    for x in range(14, 34):
        p.g[by + y][bx + x] = 15
for y in range(13):
    for x in range(len(rise[0])):
        if rise[y][x] != 15:
            p.g[by + y + 2][bx + x] = rise[y][x]
region = {(bx + x, by + y) for y in range(34) for x in range(59) if not (y < 15 and 14 <= x < 34)}
n = despeckle(p, region)
p.log.append("his legs, belt and rear fist put back to his ROM take-off (0x1884, rows 33 on, one row down), and his head to its ROM head (rows 0-12, two rows down, the generated head cleared first); the torso and the raised arm are generated")
p.log.append(f"{n} single pixels in the generated torso and arm set to the pen around them")
p.save("608b0142-0.png")


# Round three.
RGBS = ground.RGB
TROUSER, SHOE, SKIN = [0, 11, 12, 13, 14], [0, 6, 7, 8], [0, 1, 2, 3, 4, 5]


def held(source, mask):
    """An inpainting with each masked pixel held to its material's pens."""
    from PIL import Image
    im = Image.open(os.path.join(HERE, source)).convert("RGBA")
    m = Image.open(os.path.join(HERE, mask)).convert("RGB")
    px, mk = im.load(), m.load()
    grid, region = [], set()
    for y in range(im.height):
        row = []
        for x in range(im.width):
            r, g, b, a = px[x, y]
            if a < 128:
                row.append(15)
                continue
            if mk[x, y] != (255, 255, 255):
                row.append(ground.IDX.get((r, g, b), 15))
                continue
            region.add((x, y))
            if r > g + 50 and g < 90:
                pens = TROUSER
            elif abs(r - g) < 24 and abs(g - b) < 24 and r < 210:
                pens = SHOE
            else:
                pens = SKIN
            row.append(min(pens, key=lambda n: sum((c - d) ** 2 for c, d in zip(RGBS[n], (r, g, b)))))
        grid.append(row)
    return grid, region


# The jumping punch: round two's, with his ROM head (the tuck's rows 0-17,
# and rows 18-21 left of x39) put back whole.
grid, _ = load_near("94e064c2-0.png")
p = Pose("jump_punch", grid)
for y in range(10, 34):
    for x in range(40, p.W):
        p.g[y][x] = 15
for y in range(8, 30):
    for x in range(40, len(jab[0])):
        if jab[y][x] != 15:
            p.g[y + 2][x] = jab[y][x]
for y in range(27, 31):
    for x in range(40, 60):
        p.g[y][x] = 15
for x in range(40, 48):
    p.g[26][x] = 1
despeckle(p, {(x, y) for y in range(21, 45) for x in range(18, 40)})
tuck, _ = load_near("inpaint-source-jump-96x80.png")
head = 0
for y in range(22):
    for x in range(20, 52):
        if y < 18 or x < 39:
            if p.g[y][x] != tuck[y][x]:
                head += 1
            p.g[y][x] = tuck[y][x]
p.log.append("as round two: round one's inpainted forearm and fist replaced by his ROM jab's (0x18e4, columns 40 on, rows 8-29, two rows down), the jab's chest left out, the arm's lower outline closed in pen 1; single pixels in the inpainted shoulder and chest set to the pen around them")
p.log.append(f"his whole ROM head from the tuck put back (rows 0-17, and rows 18-21 left of x39): {head} pixels, the nose, mouth and chin round two had cleared")
p.save("94e064c2-0.png")

ROUND3 = [
    ("jump_kick", "eadf6bda-0.png", "inpaint-mask-jump-kick-3.png", "round one's jumping kick (c8439da8) with its rear leg and the hip behind the kick inpainted again (x8-29, y39-79, and below the kicking leg x30-52, y53-79): one rear leg, the thigh down from the hip, the knee bent, the shin folded back to the shoe below his hip"),
    ("jump_punch_down", "0cb9d28c-0.png", "inpaint-mask-jump-punch-down-3.png", "his back and shoulder inpainted (x13-29, y17-38, and x30-35, y22-38) without the tuck's folded upper arm, so only the punching arm reads; the rest is the approved diagonal punch, pixel for pixel"),
    ("sobat_kick", "6648b05c-0.png", "inpaint-mask-sobat-foot.png", "his ROM high kick (0x1826, padded to 96x96 at (7, 2)) with the kicking foot inpainted as his grey shoe and sock (x69-90, y2-20); the rest is the ROM pose, pixel for pixel"),
]
for name, source, mask, line in ROUND3:
    grid, region = held(source, mask)
    p = Pose(name + "-round-3" if name == "jump_kick" else name, grid)
    p.log.append(line + "; the inpainted pixels held to his pens for what they draw (red to his trouser pens, grey to his shoe pens, the rest to his skin and outline pens)")
    n = despeckle(p, region)
    p.log.append(f"{n} single pixels in the inpainting set to the pen around them")
    if name == "sobat_kick":
        # His ground shadow, as the hop's other poses leave it off: the
        # bottom row, and the second row's black either side of his sole.
        bx, by = 7, 2
        for x in range(p.W):
            p.g[by + 92][x] = 15
        for x in list(range(0, bx + 19)) + list(range(bx + 34, p.W)):
            p.g[by + 91][x] = 15
        p.log.append("his ground shadow left off: the bottom row, and the second row's black either side of his sole (x26-40 kept)")
    p.islands()
    p.save(source)


# The heavy jumping kick's grey shoe: round two's pose, with the masked foot
# of the Rolling Sobat kick (the same ROM pose, 0x1826, at (7, 2) in its
# canvas; here at the candidate's top-left plus three rows) laid over it.

from PIL import Image
sk = Image.open(os.path.join(OUT, "sobat_kick.png")).convert("RGBA").load()
grid, _ = load_near("4564fc20-2.png")
p = Pose("jump_kick_high", grid)
bx, by = bbox(p.g)
for y in range(by, by + 3):
    for x in range(p.W):
        p.g[y][x] = 15
for y in range(51):
    for x in range(len(high[0])):
        p.g[by + 3 + y][bx + x] = high[y][x]
despeckle(p, {(x, y) for y in range(by + 54, p.H) for x in range(p.W)})
shoe = 0
for y in range(2, 21):
    for x in range(69, 91):
        c = sk[x, y]
        pen = 15 if c[3] < 128 else ground.IDX[c[:3]]
        X, Y = x - 7 + bx, y - 2 + by + 3
        if p.g[Y][X] != pen:
            shoe += 1
        p.g[Y][X] = pen
p.log.append("as round two: above his hip every pixel his ROM high kick's (0x1826, rows 0-50, three rows below the candidate's top), the generated copy of it and the rows above it cleared; single pixels in the generated rear leg set to the pen around them")
p.log.append(f"the kicking foot, bare skin in the ROM, replaced by the Rolling Sobat kick's grey shoe and sock (6648b05c's inpainting as touched up, its mask x69-90, y2-20, laid on at the same place in the ROM pose): {shoe} pixels")
p.save("4564fc20-2.png")


# Round four: the jumping kick, by hand on round three's.
grid, region = held("eadf6bda-0.png", "inpaint-mask-jump-kick-3.png")
p = Pose("jump_kick-round-4", grid)
despeckle(p, region)
for y in range(68, 76):
    for x in range(8, 19):
        p.g[y][x] = 15
for x in range(8, 18):
    p.g[68][x] = 0
p.log.append("as round three: round one's jumping kick with its rear leg and the hip behind the kick inpainted again as one bent leg, held to his trouser and shoe pens, single pixels set to the pen around them")
p.log.append("the tuck's second, lower shoe erased (x8-18, y68-75) and the upper shoe's sole closed in black (pen 0, row 68), so the tucked leg has one foot")
for y in range(37, 54):
    for x in range(72, p.W):
        p.g[y][x] = 15
fly, _ = load_near("rom-0x1944.png")
n = 0
for y in range(45, 59):
    for x in range(69, len(fly[0])):
        if fly[y][x] in (0, 6, 7, 8):
            p.g[y - 6][x + 3] = fly[y][x]
            n += 1
p.log.append(f"the kicking foot redrawn whole: the generated shoe right of x72 (y37-53) cleared and the grey shoe of his ROM flying kick's straight leg (0x1944, columns 69 on, rows 45-58) laid on at (+3, -6) at the trouser cuff: {n} pixels")
p.save("eadf6bda-0.png")


# Round five: the jumping kick, generated whole (7b8a18d2, candidate 3, its
# canvas 128x128; the figure's top-left at (40, 22), where the ROM tuck
# lines up with it pixel for pixel at the head).
grid, near = load_near("7b8a18d2-3.png")
p = Pose("jump_kick", grid)
bx, by = bbox(p.g)
tuck, _ = load_near("rom-0x1a88.png")
head = 0
for y in range(13):
    for x in range(18, 38):
        if p.g[by + y][bx + x] != tuck[y][x]:
            head += 1
        p.g[by + y][bx + x] = tuck[y][x]
p.log.append(f"his ROM head from the tuck (0x1a88, rows 0-12, columns 18-37) laid back where the generated one lines up with it: {head} pixels")
gone = 0
for y in range(53, 60):
    for x in range(0, 6):
        if p.g[by + y][bx + x] != 15:
            gone += 1
        p.g[by + y][bx + x] = 15
for y in range(54, 60):
    p.g[by + y][bx + 6] = 11
for x in range(0, 7):
    p.g[by + 60][bx + x] = 0
p.log.append(f"the upper of the two rear shoes the generation copied from the tuck erased ({gone} pixels, columns 0-5, rows 53-59 of the figure), the trouser's back edge closed in his dark red outline (pen 11) and the lower shoe's top in black (pen 0, row 60), so the tucked leg has one foot")
specks = 0
for y in range(31, 48):
    for x in range(59, 67):
        v = p.g[by + y][bx + x]
        if v in (1, 2, 3, 4, 5, 11, 12, 13, 14) and not (x == 59 and v == 11):
            p.g[by + y][bx + x] = 8
            specks += 1
p.log.append(f"brown and red specks inside the kicking shoe (columns 59-66, rows 31-47 of the figure) to his dark grey (pen 8): {specks} pixels")
n = despeckle(p, {(bx + x, by + y) for y in range(13, 72) for x in range(70)})
p.log.append(f"{n} single pixels below the head set to the pen around them")
p.islands()
p.save("7b8a18d2-3.png")
