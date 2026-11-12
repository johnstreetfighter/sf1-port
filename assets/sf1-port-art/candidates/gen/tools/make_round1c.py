"""Gen's crouching hit, second try (AP19, #183): his standing hit's upper body set lower on the crouch.

usage: make_round1c.py <out folder>

The first try laid his standing hit's upper body (0x2060) so high that the pose stood 75 pixels
tall against the crouch's 63. Here the same upper body (rows 8-55: head knocked back, hands drawn
in, down to the waist) is laid 6 pixels lower, over the tops of the crouch's thighs, so he stands
69 tall; only the band where they meet is inpainted, sparing his hands.
"""
import os, sys
import rompose as r
from src import Src
OUT = sys.argv[1]
HERE = os.path.dirname(os.path.abspath(__file__))
os.makedirs(OUT, exist_ok=True)
C = r.load(os.path.join(HERE, '..', 'touched', 'crouch.png'))
SKIN, GREY = (1, 2, 3, 4, 5), (8, 9, 10)

s = Src(64, 88); s.lay(C, 0, 0)
s.clear(0, 0, 64, 68)
hit = r.grid(0x2060, 1, 2, 1).crop(0, 8, 60, 48)
s.lay(hit, 5, 16)
hands = lambda x, y: s.c.at(x, y) in SKIN + GREY or (s.c.at(x, y) == 0 and y < 64 and x > 30)
s.paint(4, 58, 64, 74, keep=lambda x, y: hands(x, y) and y < 68)
s.save(OUT, 'crouching-hit')
