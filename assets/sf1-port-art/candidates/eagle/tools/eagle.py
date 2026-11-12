"""Eagle's crouch and the poses built on it (AP23, #187). Canvas coordinates are his stance's (0x2920) in x, and
his stance's minus 2 in y, so his ROM head lands on row 1; his crouch's ground shadow is row 72 (the ground line is
row 73). Read-only use of the ROM set.

- The crouch: his stance from the waist up (rows 0-45, down to his belt), upright with both sticks crossed before
  his chest, on the bent knees of the top of his jump (0x2a40, from row 36 down, 5 pixels right and 8 down), its
  hanging feet taken away and his stance's own trouser hems, shoes and ground shadow set under the shins.
"""
import rompose as r
S = r.grid(0x2920, 1, 2, 1)
TUCK = r.grid(0x2a40, 1, 2, 1)
W, H = 72, 76
FLOOR = 72                           # the shadow's lower row
WAIST = 44                           # the crouch's first row of trousers under his belt
STICK = (8, 11)

def legs(rear_cut=54, front_cut=55, fdx=7, rdx=5, ftop=84):
    """The crouch's legs alone: the tuck's thighs, knees and rear leg; his stance's front trouser leg from row
    `ftop` (its hem) and shoes under them, the front one straight under the knee; one ground shadow."""
    c = r.P(W, H)
    for y in range(86, 96):                                     # the rear hem, shoe and shadow
        for x in range(4, 26):
            v = S.p[y][x]
            if v != r.T: c.p[FLOOR - 95 + y][x + rdx] = v
    for y in range(ftop, 96):                                   # the front shin's foot, hem, shoe and shadow
        for x in range(30, 56):
            v = S.p[y][x]
            if v != r.T: c.p[FLOOR - 95 + y][x + fdx] = v
    for y in range(36, 72):
        for x in range(TUCK.w):
            v = TUCK.p[y][x]
            if v == r.T or (v in STICK and y < 40): continue      # the tuck's stick ends at his waist
            if x < 22 and y > rear_cut: continue
            if x >= 22 and (y > front_cut or (y > 50 and x < 36)): continue
            c.p[y + 8][x + 5] = v
    for y in (FLOOR - 1, FLOOR):                                # one ground shadow under both shoes
        for x in range(W):
            if c.p[y][x] != r.T: continue
            if any(c.p[y][q] != r.T for q in range(x)) and any(c.p[y][q] != r.T for q in range(x + 1, W)):
                c.p[y][x] = 0
    return c

def lay_upper(c, g, dx, dy, upto, keep=lambda x, y, v: True):
    """g's rows above `upto` (g's own rows), moved by (dx, dy) in stance coordinates, laid over c."""
    for y in range(min(upto, g.h)):
        for x in range(g.w):
            v = g.p[y][x]
            if v == r.T or not keep(x, y, v): continue
            X, Y = x + dx, y + dy - 2
            if 0 <= X < c.w and 0 <= Y < c.h: c.p[Y][X] = v
    return c

def crouch_source():
    c = legs()
    lay_upper(c, S, 0, 0, 46)
    return c

JAB = r.grid(0x2980, 1, 2, 3)        # his stick jab's strike (stream 0x1239e, x-offset 16)
JAB_DX, JAB_DY = -16, -5             # its body onto his stance's: belt on belt, head on head
SHOULDER = (63, 25)                  # the jab arm's shoulder joint, in the jab's own coordinates
ARM_X = 61                           # the jab's arm: its pixels right of this, rows 10-37

def in_arm(x, y, armpit=True):
    """The jab's striking arm: right of ARM_X in rows 8-37, less (with `armpit`) the sleeve's inner underside by
    the armpit (rows 31-37 left of x70), which would hang under a raised arm."""
    return 8 <= y < 38 and x >= ARM_X and not (armpit and y >= 31 and x < 70)

def jab_arm(armpit=True):
    """The jab's striking arm alone: shaded sleeve, bare forearm, fist and stick."""
    a = r.P(JAB.w, JAB.h)
    for y in range(JAB.h):
        for x in range(JAB.w):
            if in_arm(x, y, armpit): a.p[y][x] = JAB.p[y][x]
    return a

def jab_body(armpit=True):
    """The jab without its striking arm; with `armpit`, the torso's front edge under the arm (rows 28-37) is given
    his outline pen, as the ROM draws it below."""
    b = JAB.copy()
    for y in range(8, 38):
        for x in range(ARM_X, JAB.w):
            b.p[y][x] = r.T
    if armpit:
        for y in range(28, 38):
            xs = [x for x in range(50, ARM_X) if b.p[y][x] != r.T]
            if xs: b.p[y][max(xs)] = 12
    return b

def turned_arm(deg, armpit=True):
    """The jab's arm turned by deg about its shoulder (anticlockwise: up), in the jab's coordinates moved
    down by UP rows (so a raised stick fits)."""
    import rot
    return rot.turn(jab_arm(armpit), deg, SHOULDER, JAB.w + 32, JAB.h + UP, (SHOULDER[0], SHOULDER[1] + UP))

UP = 48

CROUCH_CAND = "../df197c8d-0.png"
HOLES = [(34, 44), (33, 45)]     # the crouch's kept inpainting (round one, second call)

def crouch():
    """The kept crouch inpainting with its touch-ups. Returns (picture, notes)."""
    import clean as k
    c = r.load(CROUCH_CAND)
    n = k.lone(c, region=lambda x, y: y >= 56)
    for y in range(59, 63):                        # the rear leg's right edge, left without its outline
        if c.p[y][27] == r.T: c.p[y][27] = 0
    for x, y in HOLES: c.p[y][x] = 0               # where the tuck's stick crossed his waistband
    return c, [f"inpainting's colours reduced to his nearest pens; {n} single pixels in the inpainted shins unlike all four neighbours set to the pens around them",
               "the rear trouser leg's right edge (x27, y59-62), where the tuck's leg was cut, given his black outline",
               "two holes in his black waistband (x34 y44, x33 y45), where the tuck's stick crossed it, filled black"]

def on_crouch(g, dx, dy, upto, keep=lambda x, y, v: True, w=W, h=H, ox=0, oy=0):
    """The crouch's legs (rows from WAIST) with g's rows above `upto` moved by (dx, dy) in stance coordinates laid
    on them, on a w x h canvas whose (ox, oy) is the crouch canvas's (0, 0)."""
    C, _ = crouch()
    c = r.P(w, h)
    for y in range(WAIST, C.h):
        for x in range(C.w):
            if C.p[y][x] != r.T: c.p[y + oy][x + ox] = C.p[y][x]
    for y in range(min(upto, g.h)):
        for x in range(g.w):
            v = g.p[y][x]
            if v == r.T or not keep(x, y, v): continue
            X, Y = x + dx + ox, y + dy - 2 + oy
            if 0 <= X < w and 0 <= Y < h: c.p[Y][X] = v
    return c

def jab_on_crouch(deg, w, h, top):
    """The jab's upper body (rows above its trousers, 51) on the crouch's legs, its arm turned by deg, laid whole;
    `top` rows above the crouch canvas."""
    c = on_crouch(jab_body(), JAB_DX, JAB_DY, 51, w=w, h=h, oy=top)
    arm = turned_arm(deg)
    for y in range(arm.h):
        for x in range(arm.w):
            v = arm.p[y][x]
            if v == r.T: continue
            X, Y = x + JAB_DX, y - UP + JAB_DY - 2 + top
            if 0 <= X < w and 0 <= Y < h: c.p[Y][X] = v
    return c

# Round two (the maintainer, round one: the sweep's arm "looks pasted on"; the rising strike's upper arm "does not look as
# full as it should be"): the jab's upper arm kept at its full width, its own shaded underside included, set a few
# pixels back into the torso.
SET_BACK = 3

def jab_arm_full():
    """The jab's striking arm with its whole sleeve down to its own dark underside (rows 8-33), outlined below where
    the armpit was cut away (row 34, x62-69)."""
    a = r.P(JAB.w, JAB.h)
    for y in range(8, 34):
        for x in range(ARM_X, JAB.w):
            a.p[y][x] = JAB.p[y][x]
    for x in range(62, 70):
        if a.p[33][x] != r.T: a.p[34][x] = 12
    return a

def turned_arm_full(deg):
    """jab_arm_full turned by deg about the shoulder, SET_BACK pixels toward his back, UP rows down."""
    import rot
    return rot.turn(jab_arm_full(), deg, SHOULDER, JAB.w + 32, JAB.h + UP,
                    (SHOULDER[0] - SET_BACK, SHOULDER[1] + UP))

def arm_mask(arm, top_dx, top_dy, reach=17, cap=7):
    """The whole shoulder and upper arm: the turned sleeve's pixels within `reach` of the shoulder joint, and the
    shoulder's cap (within `cap` of it, from one pixel behind the joint and above its lower edge); offsets place
    the arm canvas on the source."""
    sx, sy = SHOULDER[0] - SET_BACK, SHOULDER[1] + UP
    m = set()
    for y in range(arm.h):
        for x in range(arm.w):
            d2 = (x - sx) ** 2 + (y - sy) ** 2
            if (arm.p[y][x] != r.T and d2 <= reach * reach) or (d2 <= cap * cap and x >= sx - 1 and y <= sy + 3):
                m.add((x + top_dx, y + top_dy))
    return m
