"""Adon's jumping kick, round two (AP26, #190).

usage: ap26_make_r2.py <round folder>

The maintainer (round one): "It looks like it switches legs; the rear leg (from jump_top) becomes the front leg in
jump_kick." Round one kept the tuck's folded front calf, wrap and foot as the tucked leg and took the rear leg's
wrap and foot away. Here the top of his jump (0x3248) keeps its rear leg exactly: the wrap and foot that hang
behind (left of the black line where the front wrap overlaps it), every ROM pixel in place. Only the front leg is
taken away (the horizontal thigh, the calf folded under it, the front wrap and foot), and the kick grows from the
same front hip: the approved crouching kick's leg (his low kick's leg, 0x31e4) laid as the shaded guide. The rear
leg's shin, hidden behind the front leg in the tuck, is hinted with his own shaded calf (0x3244's hanging rear
calf, turned 40 degrees so it runs down and back into the rear wrap). The mask covers the whole hip and thigh, and
the rear shin above its wrap, sparing his upper body, his shorts away from the hip and the rear wrap and foot.
"""
import os, sys
import rompose as r, rot, ap26_air as a
from src import Src
RND = sys.argv[1]
os.makedirs(RND, exist_ok=True)
T = a.TUCK


def rear_leg():
    """The tuck's rear wrap and foot (tuck coordinates): left of the black line the front wrap draws over it."""
    out = set()
    for y in range(59, 77):
        divs = [x for x in range(16, 26) if T.p[y][x] == 0]
        lim = min(divs) if divs and y <= 66 else (21 if y <= 72 else 30)
        out |= {(x, y) for x in range(0, lim) if T.p[y][x] != r.T}
    return out


REAR = rear_leg()


def base():
    """The tuck in stance coordinates without its front leg."""
    B = r.P(80, 80)
    for y in range(64):
        for x in range(64):
            v = T.p[y][x] if y < T.h else r.T
            if v == r.T: continue
            if y < 38 or (y < 56 and (x < 24 or (v in (0, 5, 6, 7) and x < 28))) or (x, y) in REAR:
                B.p[y + 2][x + 3] = v
    for y in range(64, T.h):
        for x in range(64):
            if (x, y) in REAR: B.p[y + 2][x + 3] = T.p[y][x]
    return B


def kick_leg():
    """The approved crouching kick's leg and hip (stance coordinates), as round one laid it."""
    k = r.load("../../../adon/crouching_kick.png")
    L = r.P(80, 80)
    for ky in range(40, k.h):
        for kx in range(24, k.w):
            v = k.p[ky][kx]
            if v == r.T or (kx < 30 and v in (0, 5, 6, 7)) or (ky >= 56 and kx < 34): continue
            L.p[ky + 1][kx + 2] = v
    return L


def rear_shin():
    """0x3244's hanging rear calf (rows 60-75, x12-30, above its wrap), turned 40 degrees clockwise about the
    ankle so it runs down and back, the ankle on the rear wrap's top."""
    g = r.grid(0x3244, 1, 2, 1)
    c = r.P(64, 96)
    for y in range(58, 77):
        for x in range(10, 31):
            v = g.p[y][x]
            if v != r.T and v not in (11, 12, 13): c.p[y][x] = v
    return rot.turn(c, -40, (21, 76), 80, 80, (17, 64))


if __name__ == '__main__':
    s = Src(80, 80)
    B, L = base(), kick_leg()
    s.lay(rear_shin(), 0, 0)
    s.lay(B, 0, 0)
    s.lay(L, 0, 0)
    for (x, y) in REAR: s.c.p[y + 2][x + 3] = T.p[y][x]          # the rear wrap and foot on top, every ROM pixel
    rear_px = {(x + 3, y + 2) for x, y in REAR}
    upper = lambda x, y: y < 42                                   # his body above the belt
    shorts = lambda x, y: y < 54 and x < 22 and B.at(x, y) in (0, 5, 6, 7)
    s.paint(20, 40, 47, 59, keep=lambda x, y: upper(x, y) or shorts(x, y) or (x, y) in rear_px)
    s.mask |= {(x, y) for y in range(52, 66) for x in range(8, 30)
               if (x, y) not in rear_px and not shorts(x, y) and not upper(x, y)}
    s.save(RND, 'jump-kick')
