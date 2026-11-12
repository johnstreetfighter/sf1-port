"""Rising, round four: the maintainer's round-three note ("Almost good, there is clipping or a bad join where his bottom/butt
is"), fixed by hand on round three's packed pose. The round-four inpainting of the seat (fd4681cd) changed 71 pixels
and kept the holes, so it is not used.

usage: seat_r4.py <touched3/rising.png> <out.png>

- His studded black belt is drawn straight across the back of his waist, rows 50-53, from his back's outline to the
  belt round three drew (x38), with a stud every four pixels as his ROM belts have them (pens 6 over 7 on black):
  it covers the holes the lunge's belt crop left and the jeans that stuck up above it.
- Light specks on the back's outline above the belt (pens 4, 5, 9, 13 among black) are black outline.
- Below the belt the seat is the kneeling legs' own rounded jeans, as before.
"""
import sys
import rompose as r
c = r.load(sys.argv[1])
fixed = 0
def put(x, y, v):
    global fixed
    if c.p[y][x] != v: c.p[y][x] = v; fixed += 1
for y in range(50, 54):
    left = next(x for x in range(c.w) if c.p[y][x] != r.T)
    for x in range(left, 38): put(x, y, 0)
for x in range(left + 3, 38, 4):
    put(x, 51, 6); put(x, 52, 7)
for y in range(44, 50):
    for x in range(16, 26):
        if c.p[y][x] in (4, 5, 9, 13) and all(c.at(x + dx, y + dy) in (0, 1, 2, r.T) for dx in (-1, 1) for dy in (0,)):
            put(x, y, 0)
c.save(sys.argv[2])
print(fixed)
