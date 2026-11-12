"""Birdie's crouch, and his standing ROM upper bodies laid on its legs (AP21, #185).

Canvas coordinates are the jump tuck's (0x240c) plus MARGIN on the left. His tuck's upper body is his stance's
moved 4 pixels left and 4 up (the waist rows match pixel for pixel), so a standing pose's upper body lands on the
crouch's legs at canvas(x, y) = donor(x - MARGIN + 4, y + 4), and rows from JOIN down are the crouch's own.
"""
import rompose as r
CAND = r"../ef1bf0cc-0.png"            # the crouch's kept inpainting (round one)
MARGIN = 4
JOIN = 47                              # first row of the crouch's own pixels under a laid upper body

def candidate():
    """The kept crouch inpainting with its touch-ups (64 wide, tuck coordinates)."""
    c = r.load(CAND)
    # A second rip the inpainting drew low on the rear leg, above the boot: his jeans' black-and-grey dither instead.
    for y in range(72, 77):
        for x in range(19, 29):
            if c.p[y][x] in (1, 2, 3, 4): c.p[y][x] = 0 if (x + y) % 2 else 11
    c.p[79][39] = r.T                   # a stray brown pixel jutting from the front boot's heel
    return c

def upper_on_legs(code, rows, cols, dx=4, dy=4, upto=JOIN, keep=lambda x, y: True, legs=None, width=None):
    """The crouch's legs (rows from JOIN) with one of his standing ROM poses' upper body (its rows above
    `upto`, shifted by its torso's offset dx, dy from his stance's 4, 4) laid on them."""
    L = legs or candidate()
    D = r.grid(code, 1, rows, cols)
    w = width or max(L.w + MARGIN, D.w - dx + MARGIN)
    out = r.P(w, L.h)
    for y in range(JOIN, L.h):
        for x in range(L.w):
            if L.p[y][x] != r.T: out.p[y][x + MARGIN] = L.p[y][x]
    out.p[JOIN][3 + MARGIN] = out.p[JOIN][4 + MARGIN] = r.T     # the tuck's near elbow, under his vest
    for y in range(0, upto):
        for x in range(w):
            v = D.at(x - MARGIN + dx, y + dy)
            if v != r.T and keep(x, y): out.p[y][x] = v
    return out

def crouch():
    """His crouch: his stance from the waist up (0x2400) on the crouch's legs."""
    return upper_on_legs(0x2400, 3, 1)
