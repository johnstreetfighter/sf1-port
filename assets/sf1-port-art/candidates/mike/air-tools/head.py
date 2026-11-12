"""His ROM heads, by exact row spans, and laying one over a generated pose."""
import rompose as r
SPANS = {
    # his turned head, as his straight (0x14e0) and hook (0x1560, 16 rows up) draw it
    'back': ((0x14e0, 1, 3, 3), {29: (45, 50), 30: (43, 52), 31: (42, 53), 32: (41, 54), 33: (41, 54), 34: (40, 55),
                                  35: (40, 55), 36: (40, 55), 37: (40, 55), 38: (40, 55), 39: (44, 52)}),
    # his facing head, as his jab (0x1470) draws it (the tuck and the hook's wind-up draw the same head)
    'face': ((0x1470, 1, 3, 2), {9: (26, 31), 10: (24, 33), 11: (23, 34), 12: (23, 35), 13: (22, 35), 14: (22, 36),
                                  15: (22, 36), 16: (22, 36), 17: (22, 36), 18: (22, 36), 19: (23, 35), 20: (23, 35), 21: (25, 36)}),
    # his ducking head, low and looking ahead, as his ducking punch's wind-up (0x15aa) draws it
    'duck': ((0x15aa, 1, 1, 1), {0: (53, 58), 1: (51, 60), 2: (50, 61), 3: (49, 62), 4: (49, 62), 5: (50, 63), 6: (51, 63),
                                  7: (51, 63), 8: (52, 63), 9: (53, 63), 10: (53, 62), 11: (52, 62), 12: (52, 63), 13: (52, 63),
                                  14: (52, 63), 15: (51, 62)}),
}
def rom_head(kind):
    grid, spans = SPANS[kind]
    g = r.grid(*grid)
    y0, y1 = min(spans), max(spans) + 1
    x0 = min(a for a, b in spans.values()); x1 = max(b for a, b in spans.values()) + 1
    h = r.P(x1 - x0, y1 - y0)
    for y, (a, b) in spans.items():
        for x in range(a, b + 1):
            h.p[y - y0][x - x0] = g.p[y][x]
    return h
def best_offset(c, h, search):
    best = None
    for ay in range(search[2], search[3]):
        for ax in range(search[0], search[1]):
            s = 0
            for y in range(h.h):
                for x in range(h.w):
                    hv = h.p[y][x]
                    if hv == r.T: continue
                    cv = c.at(ax + x, ay + y)
                    if hv == 0 and cv == 0: s += 2
                    elif (hv == 0) != (cv == 0): s -= 1
                    elif cv == r.T: s -= 2
                    elif abs(hv - cv) <= 1: s += 1
            if best is None or s > best[0]: best = (s, ax, ay)
    return best
def lay(c, h, ax, ay, clear_box=None, protect=None):
    """Clear the generated head's hair and skin in clear_box where the ROM head does not reach, then lay
    the ROM head; pixels inside `protect` (x0, y0, x1, y1) keep the candidate's own."""
    prot = lambda x, y: protect and protect[0] <= x < protect[2] and protect[1] <= y < protect[3]
    cleared = laid = 0
    if clear_box:
        x0, y0, x1, y1 = clear_box
        for y in range(y0, y1):
            for x in range(x0, x1):
                inside = 0 <= x - ax < h.w and 0 <= y - ay < h.h and h.p[y - ay][x - ax] != r.T
                if not inside and not prot(x, y) and c.at(x, y) in (0, 1, 2, 3, 4, 5):
                    c.p[y][x] = r.T; cleared += 1
    for y in range(h.h):
        for x in range(h.w):
            X, Y = ax + x, ay + y
            if h.p[y][x] != r.T and 0 <= X < c.w and 0 <= Y < c.h and not prot(X, Y):
                if c.p[Y][X] != h.p[y][x]: laid += 1
                c.p[Y][X] = h.p[y][x]
    return cleared, laid
