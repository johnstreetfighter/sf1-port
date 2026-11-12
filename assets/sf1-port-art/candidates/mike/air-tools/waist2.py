"""Round two of the jump punch and jump hook (AP16): the tuck's trousers kept exactly; the upper body set so its belt
sits on the tuck's waist; only the shirt hem and belt band between them inpainted."""
import sys, rompose as r
from PIL import Image
JEANS = (11, 12, 13, 14)
tuck = r.grid(0x1608, 1, 1, 1)
W, H, OY = 96, 84, 16   # the tuck at canvas (0, OY)
def legs():
    """The tuck's trousers and shoes: its jeans from row 30 (the top of his trousers) down, and everything from row 44 down that is not his shirt or arm."""
    c = r.P(W, H)
    for y in range(64):
        for x in range(64):
            v = tuck.p[y][x]
            if v == r.T: continue
            if (v in JEANS and y >= 30) or y >= 44:
                c.p[OY + y][x] = v
    return c
def build(upper, ux, uy, strip_from, belt_rows):
    c = legs()
    keep = [[c.p[y][x] != r.T for x in range(W)] for y in range(H)]
    up = upper.copy()
    for y in range(strip_from, up.h):      # the upper body's own jeans and everything under its belt left off
        for x in range(up.w):
            if y > belt_rows[1] or up.p[y][x] in JEANS: up.p[y][x] = r.T
    for y in range(up.h):
        for x in range(up.w):
            v = up.p[y][x]; X, Y = ux + x, uy + y
            if v != r.T and 0 <= X < W and 0 <= Y < H and not keep[Y][X]:
                c.p[Y][X] = v
    return c, keep
def mask(c, keep, y0, y1, x0, x1):
    m = Image.new('RGB', (W, H), (0, 0, 0)); n = 0
    for y in range(y0, y1):
        for x in range(x0, x1):
            if not keep[y][x]:
                m.putpixel((x, y), (255, 255, 255)); n += 1
    return m, n
