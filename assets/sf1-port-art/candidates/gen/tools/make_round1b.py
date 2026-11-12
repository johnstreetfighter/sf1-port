"""Gen's crouching block, second try (AP19, #183): the whole raised arm of his standing block.

usage: make_round1b.py <out folder>

The first try stood only the forearm on the crouch's sleeve; the inpainting left the space
below the elbow empty, so the robe looked cut off there. Here his standing block's whole arm
(0x200c, x32-51, y9-41: the sleeve from the shoulder, the grey cuff, forearm and fist) is laid
on the crouch's forward shoulder, the sleeve hanging over where the old arm was; only the
shoulder join and the band where the sleeve meets his knee are inpainted.
"""
import os, sys
import rompose as r
from src import Src
OUT = sys.argv[1]
HERE = os.path.dirname(os.path.abspath(__file__))
os.makedirs(OUT, exist_ok=True)
C = r.load(os.path.join(HERE, '..', 'touched', 'crouch.png'))
head = lambda x, y: 22 <= x < 42 and y < 48

s = Src(64, 88); s.lay(C, 0, 0)
s.clear(45, 50, 64, 67, keep=lambda x, y: y >= 63 and x < 50)
arm = r.grid(0x200c, 1, 2, 1).crop(32, 9, 20, 33)
s.lay(arm, 40, 28)
s.paint(36, 42, 50, 52, keep=lambda x, y: head(x, y))
s.paint(38, 56, 63, 69)
s.save(OUT, 'crouching-block')
