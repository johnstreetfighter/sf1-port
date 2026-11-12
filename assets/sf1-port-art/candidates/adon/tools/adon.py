"""Adon's crouch, and his standing ROM upper bodies laid on its legs (AP25, #189).

The crouch is his stance (0x3060) from the head down to its belt (rows 0-42) on the kept inpainting's shorts and
folded legs, in the stance's own coordinates (64 x 72, the ground shadow on rows 69-70). A standing pose whose
belt matches the stance's (his jab's, 0x30c4, 3 pixels right) lays its upper body on the crouch's legs with no join.
"""
from collections import Counter
import rompose as r
CAND = r"../b938e83f-0.png"            # the crouch's kept inpainting (round one)
BELT = 43                              # first row of the crouch's own pixels under a laid upper body
STANCE = r.grid(0x3060, 1, 2, 1)

def candidate():
    """The kept crouch inpainting with its touch-ups."""
    c = r.load(CAND)
    fixes = []
    # The front ankle: the inpainting widened the wrap's top to the left of the shin (x32-38, rows 55-57); his
    # stance's wrap top (row 80) closes the wrap under the knee instead.
    n = 0
    for y in (55, 56, 57):
        for x in range(30, 39):
            if c.p[y][x] in (7, 11, 12, 13): c.p[y][x] = r.T; n += 1
    for x in range(39, 48): c.p[56][x] = STANCE.p[80][x - 4]
    fixes.append(f"the front ankle wrap's top: {n} grey pixels the inpainting spread left of the shin removed, and the wrap closed under the knee with the top row of his stance's own wrap (row 80)")
    # Dark purple (his shorts' pen 7) specks drawn inside the grey wraps and skin of the ankles.
    m = 0
    for y in range(52, 60):
        for x in range(64):
            if c.p[y][x] == 7 and y > 53:
                n8 = [c.at(x + dx, y + dy) for dx in (-1, 0, 1) for dy in (-1, 0, 1) if dx or dy]
                c.p[y][x] = Counter(v for v in n8 if v != r.T).most_common(1)[0][0]; m += 1
    if m: fixes.append(f"{m} specks of his shorts' dark purple inside the ankle wraps set to the pens around them")
    return c, fixes

CAND2 = r"../10bd952f-0.png"           # round two: the front leg rebuilt (make_round2.py)

def candidate2():
    """Round two's crouch inpainting with its touch-ups: the thigh's underside outline closed where the inpainting
    left transparent holes (inpainting keeps a source's holes)."""
    c = r.load(CAND2)
    pts = [(31, 51), (31, 52), (31, 53), (33, 53), (34, 53), (36, 53), (37, 53), (38, 53)]
    for x, y in pts: c.p[y][x] = 2 if y == 51 else 1
    return c, [f"{len(pts)} transparent holes the inpainting left in the front thigh's underside (x31-38, rows 51-53) filled, the outline in his legs' dark brown"]

def crouch(rnd=2):
    return candidate2()[0] if rnd == 2 else candidate()[0]

def upper_on_legs(code, rows, cols, dx=0, dy=0, upto=BELT, keep=lambda x, y: True, width=None, legs_from=BELT, rnd=2):
    """The crouch's legs (rows from BELT) with one of his standing ROM poses' upper body (its rows above `upto`,
    taken from donor(x + dx, y + dy)) laid on them."""
    L = crouch(rnd)
    D = r.grid(code, 1, rows, cols)
    w = width or L.w
    out = r.P(w, L.h)
    for y in range(legs_from, L.h):
        for x in range(L.w):
            if L.p[y][x] != r.T: out.p[y][x] = L.p[y][x]
    for y in range(0, upto):
        for x in range(w):
            v = D.at(x + dx, y + dy)
            if v != r.T and keep(x, y): out.p[y][x] = v
    return out
