"""Gen's diagonal jumping punch, round three (AP20, #184).

usage: ap20_make_r3.py <out folder> [shaded]

`shaded` (round 3b): the two flat-hint tries came back with the sleeve still flat, so the hint is
the flurry arm's own shaded sleeve (0x2246, where round one laid it), each column squeezed to the
slim sleeve's height by nearest sampling: his ROM outline, highlight, mid and shade bands, slimmer.

The maintainer (2026-11-07) rejected round two's: "the robe/torso is extending too much to the right with
the arm, doesn't look good at all." Round two's hint and the shadow under the armpit widened his
body toward the opponent. Here the tuck's own torso and robe stay his ROM pixels, outside the mask:
only his front cuff and clawed hand are taken away (as in round one), and the arm leaves the body
from the tuck's own upper sleeve at the shoulder. A slim sleeve, hinted flat in his purples, runs
down and forward from there to the flurry arm's cuff and open hand (0x2246), which are kept. The
mask covers only the shoulder join (the tuck's sleeve stub under his rear hand) and the new sleeve.
"""
import os, sys
import rompose as r
from src import Src
OUT = sys.argv[1]
SHADED = len(sys.argv) > 2 and sys.argv[2] == 'shaded'
os.makedirs(OUT, exist_ok=True)
TUCK = r.grid(0x22a6, 1, 2, 1)
GREY, SKIN = (8, 9, 10), (1, 2, 3, 4, 5)
DARK, MID = 11, 13
head = lambda x, y: y < 14 and 30 <= x < 48

s = Src(88, 80); s.lay(TUCK, 0, 0)
s.clear(48, 10, 64, 30, keep=head)                       # his front cuff and clawed hand
rear = lambda x, y: x < 47 and y < 22 and s.c.at(x, y) in SKIN + (0,)   # his rear hand, outline and all

# The flurry arm's cuff and open hand only (0x2246), where round one laid them (8 left, 22 up).
g = r.grid(0x2246, 1, 2, 2)
hand = set()
for y in range(52, 77):
    for x in range(70, 92):
        v = g.p[y][x]
        if v != r.T and (v in GREY + SKIN or (v == 0 and x >= 76)):
            X, Y = x - 8, y - 22
            s.c.p[Y][X] = v; hand.add((X, Y))
# A slim sleeve from the shoulder down and forward to the cuff: upper outline (49,15)->(70,34),
# lower outline (46,26)->(65,44), hinted flat.
top = lambda x: 15 + (x - 49) * 19 / 21
bot = lambda x: 26 + (x - 46) * 18 / 19
sleeve = set()
for x in range(44, 71):
    for y in range(12, 46):
        if (x >= 49 and y >= top(x) or x < 49 and y >= 15) and y <= bot(x) and (x, y) not in hand:
            if not (rear(x, y) or head(x, y)):
                sleeve.add((x, y))
if SHADED:
    import tempfile
    sys.argv = [sys.argv[0], tempfile.mkdtemp()]   # ap20_make_air writes round one's sources on import
    import ap20_make_air as a
    F = r.P(88, 80); F.paste(a.flurry_arm(), 48, 16)
    cols = {}
    for x in range(44, 71):
        run = [y for y in range(80) if F.at(x, y) in (0, 11, 12, 13, 14)]
        if run:
            y0 = run[0]; y1 = y0
            while F.at(x, y1 + 1) in (0, 11, 12, 13, 14): y1 += 1
            cols[x] = [F.p[y][x] for y in range(y0, y1 + 1)]
for x, y in sleeve:
    ys = [v for u, v in sleeve if u == x]
    if SHADED and x in cols and x >= 48:
        c = cols[x]; t, b = min(ys), max(ys)
        s.c.p[y][x] = c[min(len(c) - 1, (y - t) * len(c) // (b - t + 1))]
        continue
    edge = any((x + dx, y + dy) not in sleeve and (x + dx, y + dy) not in hand and s.c.at(x + dx, y + dy) == r.T
               for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)))
    if s.c.p[y][x] == r.T or x >= 48:
        s.c.p[y][x] = DARK if edge else MID
# The mask: the shoulder join (the tuck's sleeve stub under his rear hand, x42-49, rows 14-29)
# and the new sleeve with a one-pixel margin; never his robe's front edge (x < 42) or his knees.
s.paint(42, 14, 50, 30, keep=lambda x, y: rear(x, y) or head(x, y))
s.mask |= {(x + dx, y + dy) for x, y in sleeve for dx in (-1, 0, 1) for dy in (-1, 0, 1)
           if x + dx >= 42 and not rear(x + dx, y + dy) and not head(x + dx, y + dy) and (x + dx, y + dy) not in hand}
s.save(OUT, 'jump-punch-down-shaded' if SHADED else 'jump-punch-down')
