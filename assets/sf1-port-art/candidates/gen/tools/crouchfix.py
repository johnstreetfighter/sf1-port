"""Gen's crouch: the kept inpainting (5cb120f1) with its seam finished by hand (AP19, #183).

Inside the mask (rows 68-76) the inpainting bulged his left thigh outward and left his robe's red
trim broken where the two halves of the cut meet. Left of his inner thigh line (x < 31) the rows are
set back to the source's (his ROM pixels); the trim is redrawn as one line, bending over his knee,
each row's trim pixel and its dark-purple edges moved left onto the line and the robe's colour
filled in behind it.
"""
import rompose as r
TRIM = {68: 39, 69: 39, 70: 40, 71: 40, 72: 41, 73: 42, 74: 43, 75: 44, 76: 45}
def crouch(cand, src):
    c = r.load(cand); s = r.load(src)
    for y in range(68, 77):
        for x in range(0, 31): c.p[y][x] = s.p[y][x]
    for y, t1 in TRIM.items():
        row = c.p[y]
        t0 = max(x for x in range(34, 50) if row[x] in (6, 7))
        pen = row[t0]; right = row[t0 + 2]
        for x in range(34, 50):
            if row[x] in (6, 7): row[x] = right
        row[t1 - 1] = 11; row[t1] = pen; row[t1 + 1] = 11
        for x in range(t1 + 2, t0 + 2): row[x] = right
    return c
