"""Gen's crouch, round three (AP19, #183): the robe hanging long to the floor.

usage: make_round3.py <out folder> crouch | derived

The maintainer (2026-11-07) kept round two's upright upper body and bent legs, but the robe looked cut
short above the knees: it should hang or fold down low toward the floor over and around them.
So the crouch's source is round two's crouch (touched2/crouch.png) with the lower part of his
stance's own front panel (0x2000, x21-38, rows 62-85: the panel with its red trim down both
edges and its red hem) hung from the waist to the floor between his knees, shifted 3 right to
meet the trim at his waist, the hem just above his ground shadow. Two masks, one call each:
- 'a': only the waist band where the panel meets the robe (rows 60-71) and the panel's edges
  where it falls past his knees (x22-27 and x38-45, rows 70-85), the panel's middle kept;
- 'b': the whole robe below the waist between and over the knees (x16-50, rows 60-85), the
  panel there as the starting point, his shoes kept.
`derived` then builds the punch, block and kick on the finished crouch (touched3/crouch.png),
with round two's joins.
"""
import os, sys
import rompose as r
from src import Src
OUT, STEP = sys.argv[1], sys.argv[2]
HERE = os.path.dirname(os.path.abspath(__file__))
os.makedirs(OUT, exist_ok=True)
SKIN, GREY = (1, 2, 3, 4, 5), (8, 9, 10)
SHOE = (6, 7)

def draped():
    c = r.load(os.path.join(HERE, '..', 'touched2', 'crouch.png'))
    panel = r.grid(0x2000, 1, 2, 1).crop(21, 62, 18, 24)
    for y in range(panel.h):
        for x in range(panel.w):
            if panel.p[y][x] in SKIN: panel.p[y][x] = r.T          # the ankle of his rear foot, at the panel's left
    c.paste(panel, 24, 62)
    return c

shoes = lambda c: (lambda x, y: y >= 76 and (x < 23 or x >= 47) and c.at(x, y) != r.T)

if STEP == 'crouch':
    for which in ('a', 'b'):
        s = Src(72, 88); s.c = draped()
        keep = shoes(s.c)
        if which == 'a':
            s.paint(8, 60, 64, 72, keep=lambda x, y: x >= 48 and s.c.at(x, y) in SKIN + GREY + (0,))
            s.paint(22, 70, 28, 86, keep=keep)
            s.paint(38, 70, 46, 86, keep=keep)
        else:
            s.paint(16, 60, 51, 86, keep=lambda x, y: keep(x, y) or (x >= 48 and y < 67 and s.c.at(x, y) in SKIN + GREY + (0,)))
        s.save(OUT, f'crouch-{which}')

if STEP == 'derived':
    # Round two's joins (make_round2.py derived-b and kick), on the round-three crouch.
    N = r.load(os.path.join(HERE, '..', 'touched3', 'crouch.png'))      # 72x88, figure x11-64, y24-87
    head = lambda x, y: 26 <= x < 46 and y < 40
    arm_gone = [(x, y) for y in range(52, 67) for x in range(47, 72)]   # the arm his stance holds out

    def thigh(s, top):
        """A flat hint of his forward thigh where the old arm hung over it, as in round 2b."""
        for y in range(top, 67):
            e = 52 + max(0, y - 60)
            for x in range(46, e):
                if s.c.p[y][x] == r.T or x >= 47: s.c.p[y][x] = 12
            s.c.p[y][e] = 11

    def leg():
        """His flying kick's outstretched leg (0x2309, x43-95, y37-63), his hand above it left out (round one's crop)."""
        g = r.grid(0x2309, 1, 1, 2).crop(43, 37, 53, 27)
        for y in range(0, 4):
            for x in range(10, g.w):
                if g.p[y][x] in GREY + SKIN: g.p[y][x] = r.T
        for y in range(0, 8):
            for x in range(10, g.w):
                if g.p[y][x] != r.T and all(g.at(x + dx, y + dy) == r.T for dx in (-1, 0, 1) for dy in (-1, 0, 1) if dx or dy): g.p[y][x] = r.T
        g.p[0][7] = g.p[0][8] = r.T            # the crop's top outline, which showed inside the sleeve in round two
        return g

    s = Src(88, 88); s.lay(N, 0, 0)
    for x, y in arm_gone: s.c.p[y][x] = r.T
    s.lay(r.grid(0x2126, 1, 2, 2).crop(56, 11, 37, 21), 48, 42)
    thigh(s, 60)
    s.paint(38, 40, 61, 69, keep=head)
    s.save(OUT, 'crouching-punch')

    s = Src(72, 88); s.lay(N, 0, 0)
    for x, y in arm_gone: s.c.p[y][x] = r.T
    s.lay(r.grid(0x200c, 1, 2, 1).crop(33, 9, 19, 33), 44, 26)
    thigh(s, 59)
    s.paint(38, 40, 52, 52, keep=head)
    s.paint(40, 54, 64, 69)
    s.save(OUT, 'crouching-block')

    s = Src(96, 88); s.lay(N, 0, 0)
    s.clear(44, 66, 72, 88)
    s.lay(leg(), 38, 61)
    hand = lambda x, y: x >= 50 and y < 67 and N.at(x, y) in SKIN + GREY + (0,)
    for y in range(52, 67):
        for x in range(50, 72):
            if hand(x, y): s.c.p[y][x] = N.p[y][x]
    for x in range(44, 92):   # his black ground shadow, two rows, carried on under the leg
        for y in (86, 87):
            if s.c.p[y][x] == r.T: s.c.p[y][x] = 0
    s.paint(34, 60, 58, 80, keep=hand)
    s.save(OUT, 'crouching-kick')
