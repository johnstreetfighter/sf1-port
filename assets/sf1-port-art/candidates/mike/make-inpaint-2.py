"""Inpainting sources and masks for Mike's ground set, round two (AP15, #179).

usage: make-inpaint-2.py <folder holding rompose.py and piece3.py>

the maintainer's round-one calls (2026-11-06): the punches read as the wrong
punches and their arms looked pasted on, so each strike is inpainted with
his shoulder, upper arm and chest inside the mask; the pieced crouching
block and hit were wrong at the belt, so the join is inpainted; the throw
and the rising pose need new art. His head stays his ROM head: every mask
keeps a box round it black.
"""
import os
import sys

from PIL import Image

sys.path.insert(0, sys.argv[1])
import piece3  # noqa: E402
import rompose as r  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))


def mask(w, h, white, black, name):
    m = Image.new("RGB", (w, h), (0, 0, 0))
    px = m.load()
    for x0, y0, x1, y1 in white:
        for y in range(y0, y1):
            for x in range(x0, x1):
                px[x, y] = (255, 255, 255)
    for x0, y0, x1, y1 in black:
        for y in range(max(y0, 0), min(y1, h)):
            for x in range(max(x0, 0), min(x1, w)):
                px[x, y] = (0, 0, 0)
    m.save(os.path.join(HERE, f"inpaint-mask-{name}.png"))


def source(grid, w, h, at, name):
    c = r.blank(w, h)
    r.paste(c, grid, at)
    r.save(c, os.path.join(HERE, f"inpaint-source-{name}-{w}x{h}.png"))


pieced = piece3.poses()

# Standing body hook: his hook's wind-up (0x1409). Head and guard fist kept.
source(r.grid(0x1409, 1, 2, 2), 96, 96, (0, 0), "body-hook-2")
mask(96, 96, [(8, 16, 96, 56)], [(36, 0, 64, 33)], "body-hook-2")

# Low body hook: his ducking punch's wind-up (0x15aa). Head kept.
source(r.grid(0x15AA, 1, 1, 1), 96, 64, (0, 0), "low-body-hook-2")
mask(96, 64, [(8, 8, 96, 48)], [(42, 0, 66, 27)], "low-body-hook-2")

# Overhand right: his hook's strike (0x1560), back to us. Back of his head kept.
source(r.grid(0x1560, 1, 3, 2), 96, 128, (0, 0), "overhand-2")
mask(96, 128, [(40, 0, 96, 64)], [(28, 10, 56, 31)], "overhand-2")

# Crouching uppercut: his ducking punch's wind-up (0x15aa), with room above;
# his crouch hides his face behind his fists, so this lower pose. Head kept.
source(r.grid(0x15AA, 1, 1, 1), 80, 104, (0, 40), "uppercut-2")
mask(80, 104, [(0, 0, 80, 80)], [(42, 40, 66, 67)], "uppercut-2")

# Crouching block and crouching hit: the pieced poses, the join at the belt,
# on canvases padded to multiples of 4 (Pro Flash needs them).
source(piece3.build(pieced["crouching_block"]), 64, 96, (0, 0), "crouching-block-2")
mask(64, 96, [(0, 50, 64, 72)], [], "crouching-block-2")
source(piece3.build(pieced["crouching_hit"]), 72, 108, (0, 0), "crouching-hit-2")
mask(72, 108, [(0, 58, 72, 82)], [], "crouching-hit-2")

# Throw: his straight's step-in (0x147b), room before him. Head kept.
STEP = r.grid(0x147B, 1, 3, 1)
for name in ("clinch", "shove"):
    source(STEP, 112, 112, (0, -4), name)
    mask(112, 112, [(8, 16, 112, 60)], [(38, 4, 60, 31)], name)

# Rising: his ducking punch's wind-up (0x15aa), room behind. Head kept.
source(r.grid(0x15AA, 1, 1, 1), 96, 64, (16, 0), "rising")
mask(96, 64, [(0, 8, 96, 64)], [(58, 0, 82, 27)], "rising")

# Retries in the same round. The uppercut's first mask left room above his
# head, and PixelLab drew a second head there; this one leaves room only in
# front of his face for the rising forearm. The clinch's first try drew the
# opponent too; the same mask is tried again with a new seed.
source(r.grid(0x15AA, 1, 1, 1), 80, 104, (0, 40), "uppercut-3")
mask(80, 104, [(0, 40, 44, 80), (44, 60, 80, 80), (60, 4, 80, 60)], [(42, 40, 66, 67)], "uppercut-3")