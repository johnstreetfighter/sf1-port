"""Inpainting sources and masks for Mike's ground set, round one (AP15, #179).

Each source is one of his ROM poses drawn from its pens (pen 15 clear) on
the canvas named; each mask is white where PixelLab redraws, black where
the ROM pixels stay. Needs the scratchpad's rompose module on the path.

- body hook: his lunging hook's wind-up (0x1409), 96x96: his near arm,
  folded across his chest, swung out at belt height.
- low body hook: his ducking punch's wind-up (0x15aa), padded to 96x64:
  the near arm swung out at belt height.
- overhand: his lunging hook's strike (0x1560), 96x128: the striking arm
  brought down over the top.
"""
import os
import sys

from PIL import Image

sys.path.insert(0, sys.argv[1])
import rompose as r  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))


def mask(w, h, rects, name):
    m = Image.new("RGB", (w, h), (0, 0, 0))
    px = m.load()
    for x0, y0, x1, y1 in rects:
        for y in range(y0, y1):
            for x in range(x0, x1):
                px[x, y] = (255, 255, 255)
    m.save(os.path.join(HERE, f"inpaint-mask-{name}.png"))


def source(code, large, rows, cols, w, h, at, name):
    c = r.blank(w, h)
    r.paste(c, r.grid(code, large, rows, cols), at)
    r.save(c, os.path.join(HERE, f"inpaint-source-{name}-{w}x{h}.png"))


source(0x1409, 1, 2, 2, 96, 96, (0, 0), "body-hook")
mask(96, 96, [(18, 27, 50, 50), (50, 33, 96, 62)], "body-hook")

source(0x15AA, 1, 1, 1, 96, 64, (0, 0), "low-body-hook")
mask(96, 64, [(20, 22, 44, 46), (44, 27, 96, 50)], "low-body-hook")

source(0x1560, 1, 3, 2, 96, 128, (0, 0), "overhand")
mask(96, 128, [(58, 0, 96, 56), (48, 20, 58, 44)], "overhand")
