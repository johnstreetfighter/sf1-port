"""Touch up Retsu's air-set and Kiai Shot candidates (AP10, #174) by hand.

usage: air-touch-ups.py <out folder>

As ground-touch-ups.py, whose Pose steps this reuses: for each pose,
<out>/<pose>.png (RGBA, his colours only), <out>/<pose>.json (the touch-up
list) and <out>/<pose>-sheet.png.

- The jumping strikes are his ROM top-of-jump pose (0x1288, padded to 128x96
  at (8, 4): inpaint-source-jump-128x96.png) with one limb inpainted, so his
  tucked legs stay the ROM pose's. Airborne poses get no ground shadow, as
  0x1288 has none.
- The Kiai Shot's thrust is generated whole (31a2ffad, candidate 0, 128x128);
  its gathering and its recovery are that thrust with only the arms and chest
  inpainted, so his legs stay the thrust's.
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
Pose, load_near = ground.Pose, ground.load_near


def union(*masks):
    """One mask file covering every one of `masks`, for a candidate inpainted
    twice (the kick, then its foot)."""
    from PIL import Image, ImageChops
    out = None
    for m in masks:
        im = Image.open(os.path.join(HERE, m)).convert("RGB")
        out = im if out is None else ImageChops.lighter(out, im)
    name = "_union-" + "-".join(os.path.splitext(m)[0].replace("inpaint-mask-", "") for m in masks) + ".png"
    out.save(os.path.join(OUT, name))
    return os.path.join(OUT, name)


def short_straps(p, end=69, x0=56, x1=74, bottom=80):
    """His belt's two hanging ends, as the thrust drew them, hang almost to
    the hem; the user found them too long (2026-11-03). Below row `end` the
    straps' light greys, and the black outline along them, go to the robe's
    own dark greys on that row; on row `end` the strap closes in black."""
    light, robe = {8, 9, 12, 13}, {6, 11}
    changed = 0
    for y in range(end, bottom + 1):
        row = p.g[y]
        fill = [x for x in range(x0, x1 + 1) if row[x] in light]
        if not fill:
            continue
        if y == end:
            for x in fill:
                row[x] = 0
            changed += len(fill)
            continue
        edge = [x for x in range(x0, x1 + 1) if row[x] == 0 and any(row[x + d] in light for d in (-1, 1) if x0 <= x + d <= x1)]
        around = [row[x] for x in range(x0 - 8, x1 + 9) if row[x] in robe and x not in fill]
        pen = max(set(around), key=around.count) if around else 6
        for x in fill + edge:
            row[x] = pen
        changed += len(fill) + len(edge)
    if changed:
        p.log.append(f"his belt's hanging ends shortened to end on row {end} (about half their length; user, 2026-11-03: too long): {changed} pixels below it to the robe's own greys, the ends closed in black")


os.makedirs(OUT, exist_ok=True)
AIR = [
    ("jump_punch", "e9dc700e-0.png", ["inpaint-mask-jump-punch.png"],
     "his front arm inpainted (x48-127, y0-34 of the padded pose), driven straight out at head height with the open palm forward, as his jab strikes (0x1320)"),
    ("jump_kick", "4ec1939a-0.png", ["inpaint-mask-jump-kick.png", "inpaint-mask-jump-kick-foot.png"],
     "his front leg inpainted (x44-127, y38-79), kicked straight out at waist height (557a2804), then its thin shin and stub of a foot inpainted again (x74-127, y30-56), a full shin and the foot flexed, toes up"),
    ("jump_kick_high-round-3", "ebc711e7-0.png", ["inpaint-mask-jump-kick-high.png"],
     "his front leg inpainted (x44-127, y38-79, and the space up and forward of it, right of his raised hand, x64-127, y0-37), kicked up and forward at the opponent, the foot at head height"),
]
for name, source, masks, what in AIR:
    mask = masks[0] if len(masks) == 1 else union(*masks)
    grid, near = load_near(source, mask)
    p = Pose(name, grid)
    p.log.append(f"{what}; the rest is his ROM top-of-jump pose (0x1288), pixel for pixel; {near} pixels the inpainting drew off his palette, or in his lilac greys, set to his nearest body pens")
    p.body_pens()
    p.islands()
    if name == "jump_punch":
        # A light grey speck on his fingertips, left by the inpainting.
        p.g[12][82], p.g[12][83] = 1, 15
        p.log.append("a light grey speck on his fingertips (x82-83, y12): the first pixel to his brown outline, the second cleared")
    if name == "jump_kick_high-round-3":
        # Third round (user, 2026-11-03: a section missing): the thigh was
        # cut off by a straight edge where it met his raised arm's elbow,
        # which lay outside the mask. The join was inpainted again on this
        # pose, reduced to his pens (5baa2175), so the thigh runs whole in
        # front of the elbow.
        seam, near = load_near("5baa2175-0.png", "inpaint-mask-jump-kick-high-seam.png")
        for y in range(18, 47):
            p.g[y][50:77] = seam[y][50:77]
        p.log.append(f"the join of his thigh and raised arm's elbow inpainted again (5baa2175, x50-76, y18-46), the thigh drawn whole in front of the elbow where the first inpainting had cut it off at its mask's edge; {near} pixels to his nearest body pens")
        p.islands()
    p.save(source)

# The diagonal punch, second round (user, 2026-11-03: the arm should drive
# further forward and his body lean into it): generated whole, leaning, his
# knees still tucked (8dbd0769, candidate 1).
grid, _ = load_near("8dbd0769-1.png")
p = Pose("jump_punch_down", grid)
p.body_pens()
p.islands()
p.save("8dbd0769-1.png")

# Third round (user, 2026-11-03: the belt was not cleanly edited): below
# the shortened belt ends the straps' outlines ran on and the robe was a flat
# fill. That strip (x50-78, y70-84) was inpainted again on the shortened
# thrust (6be538b8), and the same strip goes on all three Kiai poses, whose
# pixels there are the thrust's, so the belt and robe match between frames.
robe, robe_near = load_near("6be538b8-0.png", "inpaint-mask-kiai-robe.png")


def robe_below_straps(p):
    for y in range(70, 85):
        p.g[y][50:79] = robe[y][50:79]
    p.log.append(f"the robe below the shortened belt ends inpainted again (6be538b8, x50-78, y70-84), its folds drawn whole where the straps' outlines had run on; {robe_near} pixels to his nearest body pens")


grid, _ = load_near("31a2ffad-0.png")
p = Pose("kiai_shot_2-round-3", grid)
p.body_pens()
p.islands()
short_straps(p)
robe_below_straps(p)
p.save("31a2ffad-0.png")

# The gathering and recovery, second round (user, 2026-11-03: the first were
# badly stitched and the arms and hands unclear): the mask frees his whole
# upper body round his head, so the robe and arms are drawn as one.
KIAI = [
    ("kiai_shot_1-round-3", "2c2eb0c6-0.png", "inpaint-mask-kiai-gather-3.png",
     "his whole upper body round his head inpainted (x36-104, y36-58; x36-56 and x72-104, y24-35; and his rear hip x30-62, y50-68), both arms pulled back along his side with the palms pressed together at his rear hip"),
    ("kiai_shot_3-round-3", "4f07c6c5-0.png", "inpaint-mask-kiai-recover-3.png",
     "his whole upper body round his head inpainted (x36-104, y36-58; x36-56 and x72-104, y24-35), both arms drawn back in with the palms upright before his chest"),
]
for name, source, mask, what in KIAI:
    grid, near = load_near(source, mask)
    p = Pose(name, grid)
    p.log.append(f"{what}; the rest is the thrust (31a2ffad, candidate 0), pixel for pixel; {near} pixels the inpainting drew off his palette, or in his lilac greys, set to his nearest body pens")
    p.body_pens()
    p.islands()
    short_straps(p)
    robe_below_straps(p)
    p.save(source)

# The fourth round (user, 2026-11-03: the edits were not clean enough; the
# gathering and recovery had reached three rejections and the user chose to
# generate all three Kiai Shot poses, and the heavy kick, whole, with no
# editing). Each is one generated picture; only his lilac greys and stray
# specks are touched. The thrust asked for short belt ends; the gathering and
# recovery were generated on it as their pose base, so robe, belt and lunge
# match it.
WHOLE = [
    ("jump_kick_high", "94d94cc2-1.png"),
    # The user rejected these three Kiai poses too (2026-11-03: the belt
    # still wrong) and chose a ROM stand-in, his jab's wind-up and palm
    # strike; they are written as <pose>-round-4 for the record.
    ("kiai_shot_1-round-4", "aee959fe-0.png"),
    ("kiai_shot_2-round-4", "f443efa7-0.png"),
    ("kiai_shot_3-round-4", "f24c1de5-1.png"),
]
for name, source in WHOLE:
    grid, _ = load_near(source)
    p = Pose(name, grid)
    p.body_pens()
    p.islands()
    p.save(source)
