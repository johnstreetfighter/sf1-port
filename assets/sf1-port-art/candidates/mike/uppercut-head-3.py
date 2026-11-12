"""The crouching uppercut, round three: candidate 0 of 44eb451d (generated
whole: his arm bent sharply, the fist driven up before his face, his chest
toward us) with his ROM head from his victory pose (0x166c) laid over the
generated head, and the generated fist and forearm drawn back over it where
they pass in front of his face.

usage: uppercut-head-3.py <folder holding rompose.py> <dx> <dy> <out.png>
"""
import os
import sys

sys.path.insert(0, sys.argv[1])
import rompose as r  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
dx, dy, out = int(sys.argv[2]), int(sys.argv[3]), sys.argv[4]
g = r.load(os.path.join(HERE, "44eb451d-0.png"))
before = [row[:] for row in g]
win = r.grid(0x166C, 1, 4, 1)
FACE = {0, 1, 2, 3, 4, 5, 6, 7}
# The fist and forearm he drives up: right of x65, above y37.
ARM = {(x, y) for y in range(0, 37) for x in range(65, len(g[0])) if g[y][x] != 15}
# His ROM head: hair, face and ear of the victory pose, above his collar;
# the raised arm (left of x27), the other fist and the shoulder's outline
# below and right of his jaw are left out.
head = [(x, y, win[y][x]) for y in range(32, 57) for x in range(27, 52)
        if win[y][x] in FACE and not (x > 47 and y > 50) and not (x > 43 and y > 45)]
box = (min(x for x, _, _ in head) + dx - 2, min(y for _, y, _ in head) + dy - 2,
       max(x for x, _, _ in head) + dx + 3, max(y for _, y, _ in head) + dy + 1)
cleared = 0
for y in range(box[1], box[3]):
    for x in range(box[0], box[2]):
        if g[y][x] in FACE and (x, y) not in ARM:
            g[y][x] = 15
            cleared += 1
for x, y, p in head:
    g[y + dy][x + dx] = p
for x, y in ARM:
    g[y][x] = before[y][x]
holes = 0
for y in range(box[1], box[3]):
    for x in range(box[0], box[2]):
        if g[y][x] == 15:
            left = any(g[y][k] != 15 for k in range(box[0] - 3, x))
            right = any(g[y][k] != 15 for k in range(x + 1, box[2] + 3))
            below = any(g[k][x] != 15 for k in range(y + 1, box[3] + 3))
            above = any(g[k][x] != 15 for k in range(box[1] - 3, y))
            if left and right and below and above:
                g[y][x] = 9
                holes += 1
# One-pixel-thick spurs left of his head (the victory pose's hair meets its
# raised arm there): pixels with nothing above or below them.
spurs = [(x, y) for y in range(box[1], box[3]) for x in range(box[0] - 3, box[0] + 6)
         if g[y][x] != 15 and g[y - 1][x] == 15 and g[y + 1][x] == 15]
for x, y in spurs:
    g[y][x] = 15
r.save(g, out)
print("head pixels", len(head), "box", box, "cleared", cleared, "holes filled", holes, "spurs", len(spurs))
