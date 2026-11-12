"""Birdie's air set (AP22, #186): his top-of-jump tuck (0x240c) and the pieces his jumping attacks are built from.

Canvas coordinates are the tuck's plus MARGIN on the left, as his crouch's are (birdie.py): the crouch's legs from
row 47 down are the tuck's, so his approved crouching kick's leg and hip join lie on the tuck where they lie on the
crouch. The tuck's upper body is his stance's with the head 4 pixels forward; his jab's lands on it with its waist
row for row at jab(x + 33, y + 4).
"""
import rompose as r, birdie as b
M = b.MARGIN
JOIN = b.JOIN
TUCK = r.grid(0x240c, 1, 2, 1)
JAB = r.grid(0x2480, 1, 3, 3)
CKICK = r"../touched5/crouching_kick.png"        # his approved crouching kick (AP21, round five), crouch canvas

def tuck(w=72, h=96):
    out = r.P(w, h); out.paste(TUCK, M, 0); return out

def upper(c, src, dx, dy, upto=JOIN):
    """Lay src's rows above `upto` (canvas rows) at canvas(x, y) = src(x + dx, y + dy), clearing what was there."""
    for y in range(upto):
        for x in range(c.w):
            c.p[y][x] = src.at(x + dx, y + dy)

def jump_punch():
    """His jab's upper body (0x2480) on the tuck's legs: every pixel his ROM art."""
    c = tuck(96)
    upper(c, JAB, 33, 4)
    return c

def body_without_front_leg(w=120, h=100, cut=70):
    """The tuck's upper body (rows above 47), his approved crouching kick's seat, hip and rear thigh (rows 47 to
    cut, left of x44, where the crouch's front leg was cleared), and the tuck's own rear shin and hanging foot
    below `cut`. The front leg is left out."""
    c = r.P(w, h)
    c.paste(TUCK.crop(0, 0, 64, JOIN), M, 0)
    k = r.load(CKICK)
    for y in range(JOIN, cut):
        for x in range(0, 44):
            if k.p[y][x] != r.T: c.p[y][x] = k.p[y][x]
    for y in range(cut, 96):
        for x in range(0, 40):
            v = TUCK.at(x - M, y)
            if v != r.T: c.p[y][x] = v
    return c

def crouching_kick_leg():
    """The approved crouching kick from x44 on, rows 47-79: the kicked leg, thigh to boot, without the shadow."""
    k = r.load(CKICK)
    out = r.P(k.w, k.h)
    for y in range(JOIN, 80):
        for x in range(44, k.w):
            out.p[y][x] = k.p[y][x]
    return out
