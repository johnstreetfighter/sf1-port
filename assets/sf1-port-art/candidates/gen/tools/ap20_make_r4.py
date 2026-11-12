"""Gen's diagonal jumping punch, the maintainer's call after round three (AP20, #184).

usage: ap20_make_r4.py <out folder> <dx> <dy>

The maintainer (2026-11-07), on round three: "It's very close now, the only issue I have with it is that the
arm should be slightly more set back in his torso." So round three's whole arm (the shoulder join
and slim sleeve inpainted in round three, and the flurry's cuff and open hand) is moved by hand,
dx pixels back toward his back (and dy down), onto the ROM tuck with its front cuff and clawed hand
taken away; his head and rear hand are laid back on top, as the ROM tuck draws them in front of the
front shoulder. No generation. Writes <out>/jump_punch_down.png and a zoom of it.
"""
import os, sys
from PIL import Image
import rompose as r, zoom
OUT, DX, DY = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
os.makedirs(OUT, exist_ok=True)
HERE = os.path.dirname(os.path.abspath(__file__))
R3 = r.load(os.path.join(HERE, '..', 'ap20', 'touched3', 'jump_punch_down.png'))
M = Image.open(os.path.join(HERE, '..', 'ap20', 'r3', 'inpaint-mask-jump-punch-down.png')).convert('RGB')
SKIN = (1, 2, 3, 4, 5)
TUCK = r.grid(0x22a6, 1, 2, 1)
base = r.P(88, 80); base.paste(TUCK, 0, 0)
for y in range(10, 30):
    for x in range(48, 64):
        if not (y < 14 and x < 48): base.p[y][x] = r.T          # his front cuff and clawed hand, as in every round
arm = {(x, y): R3.p[y][x] for y in range(80) for x in range(88)
       if R3.p[y][x] != r.T and (M.getpixel((x, y)) == (255, 255, 255) or (x >= 60 and y >= 30))}
c = base.copy()
for (x, y) in arm:                                              # where the arm was, the tuck shows (nothing, past the body)
    if x >= 42: c.p[y][x] = base.p[y][x]
for (x, y), v in arm.items(): c.p[y + DY][x + DX] = v
front = lambda x, y: (y < 14 and 30 <= x < 48) or (x < 47 and y < 22 and TUCK.at(x, y) in SKIN + (0,))
for y in range(0, 22):                                          # his head and rear hand back on top
    for x in range(30, 48):
        if front(x, y): c.p[y][x] = TUCK.p[y][x]
c.save(f"{OUT}/jump_punch_down.png")
zoom.zoom(c, f"{OUT}/z.png", 10, box=(24, 0, 84, 60))
T = TUCK
changed = [(x, y) for y in range(80) for x in range(88) if c.at(x, y) != T.at(x, y)]
print("changed vs ROM tuck at x<42:", sorted(p for p in changed if p[0] < 42))
print("knee pixels covered (x<58, y>=34):", [p for p in changed if p[0] < 58 and p[1] >= 34])
