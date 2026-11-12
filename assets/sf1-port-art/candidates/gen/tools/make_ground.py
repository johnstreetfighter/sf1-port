"""Inpainting sources and masks for Gen's ground set (AP19, #183), round one.

usage: make_ground.py <out folder>

The crouching poses are his finished crouch (touched/crouch.png: his hit reaction's closing
squat 0x2064, lowered, its seam finished by hand) with ROM limbs from his own poses laid on it;
the low kick is his front kick's strike (0x2180) with his flying kick's leg (0x2309). Each mask
(white) covers only the joins and what the old limb left behind.
"""
import os, sys
import rompose as r
from src import Src
OUT = sys.argv[1]
HERE = os.path.dirname(os.path.abspath(__file__))
os.makedirs(OUT, exist_ok=True)

C = r.load(os.path.join(HERE, '..', 'touched', 'crouch.png'))      # 64x88, figure x10-63, y25-87
head = lambda x, y: 22 <= x < 42 and y < 48                         # his ROM head stays, pixel for pixel
SKIN, GREY = (1, 2, 3, 4, 5), (8, 9, 10)

def leg():
    """His flying kick's outstretched leg (0x2309, x43-95, y37-63): thigh to red shoe, his hand above it left out."""
    g = r.grid(0x2309, 1, 1, 2).crop(43, 37, 53, 27)
    for y in range(0, 4):
        for x in range(10, g.w):
            if g.p[y][x] in GREY + SKIN: g.p[y][x] = r.T
    for y in range(0, 8):          # the fingertips' last specks
        for x in range(10, g.w):
            if g.p[y][x] != r.T and all(g.at(x + dx, y + dy) == r.T for dx in (-1, 0, 1) for dy in (-1, 0, 1) if dx or dy): g.p[y][x] = r.T
    return g

# Crouching block: his standing block's raised forearm and fist (0x200c, x40-53, y9-36) stood on
# the crouch's forward sleeve in place of its clawed hand; the elbow, the sleeve's end and the old
# hand's place are inpainted.
s = Src(64, 88); s.lay(C, 0, 0)
s.clear(45, 52, 64, 67, keep=lambda x, y: y >= 62 and x < 50)
fore = r.grid(0x200c, 1, 2, 1).crop(40, 9, 14, 28)
s.lay(fore, 44, 24)
s.paint(38, 44, 62, 68, keep=lambda x, y: head(x, y) or (y < 46 and s.c.at(x, y) != r.T and 44 <= x < 58))
s.save(OUT, 'crouching-block')

# Crouching punch: his straight punch's arm (0x2126, x56-92, y11-31: sleeve, cuff and fist) driven
# out from the crouch's forward shoulder in place of the clawed arm; the shoulder join and the space
# the old arm left are inpainted.
s = Src(84, 88); s.lay(C, 0, 0)
s.clear(45, 50, 64, 67, keep=lambda x, y: y >= 62 and x < 50)
arm = r.grid(0x2126, 1, 2, 2).crop(56, 11, 37, 21)
s.lay(arm, 48, 43)
s.paint(38, 42, 54, 66, keep=lambda x, y: head(x, y))
s.paint(52, 58, 66, 68)
s.save(OUT, 'crouching-punch')

# Crouching kick: his flying kick's outstretched leg (0x2309) kicked out low from the crouch's hip,
# along the floor, in place of his forward knee and shoe; the hip and the space under the robe are
# inpainted. His forward hand stays above the thigh.
s = Src(96, 88); s.lay(C, 0, 0)
s.clear(44, 66, 64, 88)
s.lay(leg(), 38, 61)
for x in range(44, 92):   # his black ground shadow, two rows, carried on under the leg
    for y in (86, 87):
        if s.c.p[y][x] == r.T: s.c.p[y][x] = 0
s.paint(34, 60, 58, 80, keep=lambda x, y: y < 66 and s.c.at(x, y) in SKIN + GREY)
s.save(OUT, 'crouching-kick')

# Crouching hit: his standing hit's upper body (0x2060, rows 8-57: head knocked back, hands drawn in)
# laid on the crouch's thighs, its sides lined up with the robe's at the waist; only the band where
# they meet is inpainted. Head and hands are his ROM pixels.
s = Src(64, 88); s.lay(C, 0, 0)
s.clear(0, 0, 64, 66)
hit = r.grid(0x2060, 1, 2, 1).crop(0, 8, 60, 50)
s.lay(hit, 5, 10)
hands = lambda x, y: s.c.at(x, y) in SKIN + GREY or (s.c.at(x, y) == 0 and y < 62)
s.paint(4, 56, 64, 72, keep=lambda x, y: hands(x, y) and y < 66)
s.save(OUT, 'crouching-hit')

# Low kick: his front kick's strike (0x2180) with its high leg taken away and his flying kick's
# outstretched leg (0x2309) driven out from the hip at knee height instead; the hip and the robe
# behind the old leg are inpainted.
s = Src(96, 96); s.lay(r.grid(0x2180, 1, 2, 2), 0, 0)
s.clear(45, 0, 96, 50)
s.clear(40, 34, 45, 50, keep=lambda x, y: s.c.at(x, y) in SKIN)
s.lay(leg(), 40, 41)
s.paint(32, 32, 60, 60, keep=lambda x, y: s.c.at(x, y) in SKIN or (x < 40 and y < 40))
s.save(OUT, 'low-kick')
