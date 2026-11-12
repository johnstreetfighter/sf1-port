"""The headbutt's inpainting source and mask (AP15, rounds two and three).

usage: make-headbutt.py <folder holding rompose.py> <dx> <dy> [3]

Round three (the maintainer: the arms and hands were wrong and the headbutt did not
read) starts from round three's clinch (2df5bbb9), whose forearms are his
ROM jab's, and inpaints only what the moved head leaves: the space where
the head was and his neck, never the forearms.

The clinch (5b255020, candidate 0, already in his pens) with his ROM head
(from his straight's step-in, 0x147b, as the clinch's source holds it)
moved forward and down by (dx, dy): his head driven into the opponent's
face. The mask covers his shoulders, neck and arms round the head, so they
are redrawn hunched in behind it, and keeps every pixel of the moved head.
"""
import os
import sys

from PIL import Image

sys.path.insert(0, sys.argv[1])
import rompose as r  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
dx, dy = int(sys.argv[2]), int(sys.argv[3])
clinch = r.load(os.path.join(HERE, "2df5bbb9-0.png" if len(sys.argv) > 4 else "5b255020-0.png"))
forearms = r.load(os.path.join(HERE, "inpaint-source-clinch-3-112x112.png"))
forearms = [[p if not (x < 52 or y < 20) else 15 for x, p in enumerate(row)] for y, row in enumerate(forearms)]
src = r.load(os.path.join(HERE, "inpaint-source-clinch-112x112.png"))
FACE = {0, 1, 2, 3, 4, 5, 6, 7}
# His head down to the fist his straight's step-in holds at his chin; the
# fist and its tape stay out.
head = [(x, y, src[y][x]) for y in range(4, 30) for x in range(38, 60)
        if src[y][x] in FACE - {6, 7} and (y < 24 or x < 44)]
W, H = len(clinch[0]), len(clinch)
g = [row[:] for row in clinch]
for x, y, _ in head:
    g[y][x] = 15
moved = set()
for x, y, p in head:
    g[y + dy][x + dx] = p
    moved.add((x + dx, y + dy))
ROUND3 = len(sys.argv) > 4
suffix = "-3" if ROUND3 else ""
r.save(g, os.path.join(HERE, f"inpaint-source-headbutt{suffix}-112x112.png"))
m = Image.new("RGB", (W, H), (0, 0, 0))
px = m.load()
if ROUND3:
    # Where the head was, and a neck under the moved head; never the
    # forearms laid from his ROM jab, nor the moved head.
    pinned = {(x, y) for y in range(H) for x in range(W) if forearms[y][x] != 15}
    white = {(x, y) for x, y, _ in head} | {(x, y) for y in range(18, 44) for x in range(30, 56)}
    for x, y in white - moved - pinned:
        px[x, y] = (255, 255, 255)
else:
    for y in range(2, 50):
        for x in range(16, 104):
            if (x, y) not in moved:
                px[x, y] = (255, 255, 255)
m.save(os.path.join(HERE, f"inpaint-mask-headbutt{suffix}.png"))
print("head pixels", len(head))
