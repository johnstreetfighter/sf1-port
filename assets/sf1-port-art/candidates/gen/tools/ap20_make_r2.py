"""Gen's diagonal jumping punch, round two (AP20, #184).

usage: ap20_make_r2.py <out folder>

The maintainer (2026-11-07) rejected round one's: "Arm looks pasted on." Round one laid the flurry arm's
whole sleeve from the tuck's front shoulder and inpainted only a small join, so the sleeve began
abruptly beside his rear hand with the old sleeve's outline still under it. Here only the arm's
lower sleeve, cuff and open hand (0x2246) are kept; the upper sleeve, the shoulder and the robe
around the armpit are cleared, a flat hint of one continuous sleeve from the shoulder to the kept
lower sleeve is drawn in his purples, and the mask covers all of it, so the sleeve's folds and
outline can run from the body into the arm.
"""
import os, sys
import rompose as r
from src import Src
import ap20_make_air as a
OUT = sys.argv[1]
os.makedirs(OUT, exist_ok=True)
DARK, MID = 11, 13

s = a.tuck(88, 80)
arm = a.flurry_arm()
s.lay(arm, 48, 16)
head = a.head
rear = lambda x, y: x < 47 and y < 22 and s.c.at(x, y) in (0, 1, 2, 3, 4, 5)   # his rear hand, outline and all
kept = lambda x, y: x >= 60 and y >= 30                                    # lower sleeve, cuff and hand
# Clear the upper sleeve and the old sleeve's outline under his rear hand.
for y in range(12, 44):
    for x in range(40, 64):
        if not (head(x, y) or rear(x, y) or kept(x, y)) and y >= 12 and (x >= 47 or y >= 22):
            if not (y >= 34 and x < 58 and s.c.at(x, y) != r.T and y > 0.45 * (x - 40) + 36):
                s.c.p[y][x] = r.T
# The hint: a sleeve from the shoulder (top edge on from his back's line, by the head) down to the kept sleeve.
top = lambda x: 13 + 0.75 * (x - 44)          # upper outline, from (44,13) to (64,28)
bot = lambda x: 27 + 0.75 * (x - 44)          # lower outline, from (44,27) to (64,42)
for x in range(44, 64):
    for y in range(int(top(x)), int(bot(x)) + 1):
        if 0 <= y < 80 and s.c.p[y][x] == r.T and not kept(x, y):
            s.c.p[y][x] = DARK if y in (int(top(x)), int(bot(x))) else MID
# His chest under the rear hand, where the old sleeve's outline ran: filled in his purple, one body with the sleeve.
for y in range(22, 36):
    for x in range(38, 48):
        if not rear(x, y) and s.c.p[y][x] in (r.T, 0): s.c.p[y][x] = MID
# The robe under the armpit, in shadow, down to the top of his knee.
for x in range(44, 58):
    for y in range(int(bot(x)) + 1, 42):
        if s.c.p[y][x] == r.T: s.c.p[y][x] = 12
s.paint(34, 10, 64, 46, keep=lambda x, y: head(x, y) or rear(x, y) or (kept(x, y) and (x >= 64 or y >= 34)))
s.save(OUT, 'jump-punch-down')
