"""Inpainting sources and masks for Lee's Zesshouohken (AP18, #182), round one.

usage: make_special.py <out folder>

The strike is his lunging punch's strike (0x1d20) with his palm strike's open hand (0x1c66)
in place of the fist, and a second arm and palm laid below it; the gather is his step-in
(0x1ccc) with both hands drawn back to his hips as open palms. Masks cover the joins only.
"""
import os, sys
import rompose as r
from src import Src
OUT = sys.argv[1]
os.makedirs(OUT, exist_ok=True)
LUNGE = r.grid(0x1d20, 1, 2, 3)
STEP = r.grid(0x1ccc, 1, 2, 1)
PALM = r.grid(0x1c66, 1, 2, 3).crop(84, 12, 18, 23)     # his open hand, fingers up, heel forward, with the wrist
def turn(g):
    t = r.P(g.h, g.w)
    for y in range(g.h):
        for x in range(g.w): t.p[x][g.h - 1 - y] = g.p[y][x]
    return t

# Strike: canvas 112x84, the lunge raised 12 rows (its ground at the bottom).
DY = -12
s = Src(112, 84); s.lay(LUNGE, 0, DY)
s.clear(93, 22 + DY, 108, 34 + DY)                     # the fist goes
s.lay(PALM, 90, 12 + 4 + DY)                            # upper palm on the forearm's end, at shoulder height
arm = LUNGE.crop(76, 22, 18, 22)                        # sleeve's end and forearm, no fist
s.lay(arm, 76 - 6, 22 + 16 + DY, keep=lambda x, y, v: True)
s.lay(PALM, 84, 12 + 4 + 16 + DY)                       # lower palm, at his stomach's height
s.paint(86, 18 + DY, 98, 36 + DY, keep=lambda x, y: x >= 92 and s.c.at(x, y) in (1, 2, 3, 4, 5) and y < 30 + DY)
s.paint(64, 34 + DY, 92, 62 + DY, keep=lambda x, y: x >= 84 and s.c.at(x, y) in (1, 2, 3, 4, 5) and y >= 34 + DY)
s.save(OUT, 'zessh-strike')

# Gather: the step-in with its raised front fist drawn down to his front hip and both hands open.
s = Src(64, 96); s.lay(STEP, 0, 0)
s.clear(40, 10, 58, 44)
s.clear(4, 36, 19, 52)
P1 = turn(PALM)                                          # fingers forward, toward the opponent
s.lay(P1, 2, 34)
s.lay(P1, 40, 34)
s.paint(36, 18, 60, 52, keep=lambda x, y: (y < 27 and x < 41) or (x >= 46 and y >= 34 and s.c.at(x, y) in (1, 2, 3, 4, 5)))
s.paint(2, 32, 24, 54, keep=lambda x, y: x < 18 and s.c.at(x, y) in (1, 2, 3, 4, 5))
s.save(OUT, 'zessh-gather')

# Gather, second try: the step-in's rear fist stays chambered at its hip, untouched; its front fist
# is drawn down to the front hip as the rear fist is held (that fist mirrored), and only the front
# arm from the shoulder to that fist is inpainted.
s = Src(64, 96); s.lay(STEP, 0, 0)
s.clear(40, 10, 58, 44)
fist = STEP.crop(5, 36, 14, 16).mirror()
s.lay(fist, 42, 36, keep=lambda x, y, v: v in (0, 1, 2, 3, 4, 5))
s.paint(36, 22, 60, 50, keep=lambda x, y: (y < 27 and x < 41) or (x >= 46 and y >= 38 and s.c.at(x, y) in (0, 1, 2, 3, 4, 5)))
s.save(OUT, 'zessh-gather-fists')

# Gather, third try: the step-in's rear arm (sleeve, forearm and the fist chambered at its hip)
# mirrored onto the front side in place of the raised front arm, so both arms are his ROM arms;
# only the front shoulder's join is inpainted.
s = Src(64, 96); s.lay(STEP, 0, 0)
s.clear(40, 10, 58, 44)
rear = STEP.crop(4, 22, 18, 31).mirror()
s.lay(rear, 38, 22)
s.paint(34, 20, 50, 34, keep=lambda x, y: y < 27 and x < 41)
s.save(OUT, 'zessh-gather-arms')
