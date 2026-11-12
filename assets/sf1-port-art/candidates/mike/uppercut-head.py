"""The crouching uppercut, round two: candidate 2 of f1b0a750 (round one's
whole generation) with his ROM head from his victory pose (0x166c) laid over
its generated head. Both are upright, facing right, mouth open, his fist
raised beside the head, so the victory head sits at the same angle.

usage: uppercut-head.py <folder holding rompose.py> <dx> <dy> <out.png>
"""
import os
import sys

sys.path.insert(0, sys.argv[1])
import rompose as r  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
dx, dy, out = int(sys.argv[2]), int(sys.argv[3]), sys.argv[4]
g = r.load(os.path.join(HERE, "f1b0a750-2.png"))
win = r.grid(0x166C, 1, 4, 1)
FACE = {0, 1, 2, 3, 4, 5, 6, 7}
# His ROM head: hair, face and ear of the victory pose, above his collar;
# the raised arm (left of x27) and the other fist (right of x47 below y50)
# are left out, and so is the shoulder's outline below and right of his jaw.
head = [(x, y, win[y][x]) for y in range(32, 57) for x in range(27, 52)
        if win[y][x] in FACE and not (x > 47 and y > 50) and not (x > 43 and y > 45)]
# The generated head goes first: its hair, skin and outline pixels in the
# head box, so nothing of it shows round the ROM head.
box = (min(x for x, _, _ in head) + dx - 2, min(y for _, y, _ in head) + dy - 2,
       max(x for x, _, _ in head) + dx + 3, max(y for _, y, _ in head) + dy + 1)
for y in range(box[1], box[3]):
    for x in range(box[0], box[2]):
        if g[y][x] in FACE:
            g[y][x] = 15
for x, y, p in head:
    g[y + dy][x + dx] = p
# Holes the generated head leaves inside his outline take his shirt's red.
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
# The generated collar stuck out in front of the ROM jaw (red and outline
# pixels right of the face, above his far shoulder): cleared.
laid = {(x + dx, y + dy) for x, y, _ in head}
wedge = [(x, y) for y in range(30, 47) for x in range(66, 82) if g[y][x] != 15 and (x, y) not in laid]
for x, y in wedge:
    g[y][x] = 15
r.save(g, out)
print("head pixels", len(head), "box", box, "holes filled", holes, "collar cleared", len(wedge))
