"""Turn a part of a pen grid by any angle (RotSprite-style: Scale2x three times, nearest turn, sample back),
then give his jeans their black-and-grey checker dither back where the source had it."""
import math
import rompose as r

def scale2x(g):
    o = r.P(g.w * 2, g.h * 2)
    for y in range(g.h):
        for x in range(g.w):
            p = g.p[y][x]; a, b, c, d = g.at(x, y - 1), g.at(x + 1, y), g.at(x - 1, y), g.at(x, y + 1)
            e0 = c if (c == a and c != d and a != b) else p
            e1 = a if (a == b and a != c and b != d) else p
            e2 = c if (d == c and d != b and c != a) else p
            e3 = b if (b == d and b != a and d != c) else p
            o.p[2 * y][2 * x], o.p[2 * y][2 * x + 1], o.p[2 * y + 1][2 * x], o.p[2 * y + 1][2 * x + 1] = e0, e1, e2, e3
    return o

def checker(g, x, y):
    """The two pens of a checker dither g has around (x, y), or None."""
    v = g.at(x, y)
    if v == r.T: return None
    n = [g.at(x + 1, y), g.at(x - 1, y), g.at(x, y + 1), g.at(x, y - 1)]
    d = [g.at(x + 1, y + 1), g.at(x - 1, y - 1), g.at(x + 1, y - 1), g.at(x - 1, y + 1)]
    if len(set(n)) == 1 and n[0] != v and n[0] != r.T and sum(1 for q in d if q == v) >= 3:
        return (v, n[0]) if (x + y) % 2 == 0 else (n[0], v)
    return None

def turn(g, deg, pivot, out_w, out_h, at):
    """g turned by deg (anticlockwise on screen, so a leg out to the right swings up) about pivot (in g),
    with the pivot landing on `at` in an out_w x out_h canvas. Returns the canvas."""
    big = scale2x(scale2x(scale2x(g)))
    t = math.radians(deg); cs, sn = math.cos(t), math.sin(t)
    out = r.P(out_w, out_h)
    for y in range(out_h):
        for x in range(out_w):
            # inverse: canvas -> g coordinates (screen y down; anticlockwise on screen)
            dx, dy = x + 0.5 - at[0], y + 0.5 - at[1]
            sx = dx * cs - dy * sn + pivot[0]
            sy = dx * sn + dy * cs + pivot[1]
            bx, by = int(math.floor(sx * 8)), int(math.floor(sy * 8))
            if 0 <= bx < big.w and 0 <= by < big.h:
                v = big.p[by][bx]
                if v != r.T:
                    ch = checker(g, int(sx), int(sy))
                    out.p[y][x] = ch[(x + y) % 2] if ch else v
    return out
