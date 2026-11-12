"""Adon's air set and specials (AP26, #190): shared pieces.

Coordinates are his stance's (0x3060, 64 x 96), as his crouch's are: the top of his jump (0x3248) sits 3 pixels
right and 2 down in them, so its belt meets the stance's (AP25), and his standing upper bodies lay on its legs.
"""
import rompose as r
TUCK = r.grid(0x3248, 1, 2, 1)
TX, TY = 3, 2                     # the tuck's place in stance coordinates


def tuck(w=96, h=96):
    out = r.P(w, h); out.paste(TUCK, TX, TY); return out


def on_tuck(code, rows, cols, dx=0, dy=0, upto=43, legs_from=41, w=96, h=96, keep=lambda x, y: True, legs=None):
    """The tuck's shorts and legs (its rows from legs_from, stance coordinates) with one of his standing ROM poses'
    upper body (its rows above upto, taken from donor(x + dx, y + dy)) laid on them."""
    L = legs or tuck(w, h)
    D = r.grid(code, 1, rows, cols)
    out = r.P(w, h)
    for y in range(legs_from, h):
        for x in range(w):
            if L.at(x, y) != r.T: out.p[y][x] = L.p[y][x]
    for y in range(upto):
        for x in range(w):
            v = D.at(x + dx, y + dy)
            if v != r.T and keep(x, y): out.p[y][x] = v
    return out
