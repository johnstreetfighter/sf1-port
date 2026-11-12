"""Inpainting source and mask for Gen's crouch (AP19, #183).

usage: make_crouch.py <out folder>

His hit reaction's closing squat (0x2064) is 71 pixels tall against his stance's 88, higher
than the crouches approved so far (about 70% of the stance). Rows 72-79, across his thighs,
are taken out (gen.CUT), so he squats 63 tall; the mask covers only the band where the two
halves meet, so the red trim and the thighs' outlines run on cleanly. His head, hands and
shoes stay his ROM pixels.
"""
import os, sys
import rompose as r, gen
from src import Src
OUT = sys.argv[1]
os.makedirs(OUT, exist_ok=True)

s = Src(64, 88); s.lay(gen.cut(), 0, 0)
# The band where the cut meets: rows 68-76, over the figure and a pixel either side of it.
fig = lambda x, y: any(s.c.at(x + dx, y) != r.T for dx in (-2, -1, 0, 1, 2))
s.paint(4, 68, 64, 77, keep=lambda x, y: not fig(x, y))
s.save(OUT, 'crouch')
