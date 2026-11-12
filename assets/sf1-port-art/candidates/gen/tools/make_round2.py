"""Gen's crouch, round two (AP19, #183): his stance from the waist up on bent legs.

usage: make_round2.py <out folder> crouch | derived | derived-b | kick

The maintainer (2026-11-07) rejected the first crouch (his hit-recovery squat, lowered): it read more
like a hit than a crouch. He asked for his stance from the waist up, with his legs bent and
the robe redrawn to the lower height. So the crouch's source is:
- his stance (0x2000) from the head to the waist (rows 8-51), upright, arms as they are,
  set 6 pixels right and 16 down;
- below it the round-one crouch's bent legs (touched/crouch.png from row 62: thighs, the
  robe's hem between the knees, shoes and ground shadow), its forward hand left out;
- the mask covers only the band where they meet (rows 60-75), sparing his stance's hand,
  so the robe's skirt is redrawn short over the bent knees.
The canvas is 72 wide, so his forward hand is not cut off. With the crouch finished
(touched2/crouch.png), `derived` builds the crouching punch and block on it.
"""
import os, sys
import rompose as r
from src import Src
OUT, STEP = sys.argv[1], sys.argv[2]
HINT = STEP == 'derived-b'   # round 2b: the thigh hinted under the arm
HERE = os.path.dirname(os.path.abspath(__file__))
os.makedirs(OUT, exist_ok=True)
C = r.load(os.path.join(HERE, '..', 'touched', 'crouch.png'))
SKIN, GREY = (1, 2, 3, 4, 5), (8, 9, 10)
DX, DY = 6, 16

def upright():
    """The new crouch's source picture (also the base for its punch and block)."""
    c = r.P(72, 88)
    legs = C.crop(0, 62, 64, 26)
    for y in range(6):                      # the round-one crouch's forward hand, left out
        for x in range(44, 64):
            if legs.p[y][x] in SKIN + GREY + (0,): legs.p[y][x] = r.T
    c.paste(legs, 0, 62)
    c.paste(r.grid(0x2000, 1, 2, 1).crop(0, 8, 64, 44), DX, 8 + DY)
    return c

def derived(out):
    """Round two's crouching punch and block, on the round-two crouch once it is finished (touched2/crouch.png)."""
    N = r.load(os.path.join(HERE, '..', 'touched2', 'crouch.png'))      # 72x88, figure x11-64, y24-87
    head = lambda x, y: 26 <= x < 46 and y < 40
    # The forward arm his stance holds out (sleeve x47-56, cuff and clawed hand x51-64, rows 52-66) is taken away.
    arm_gone = lambda x0: [(x, y) for y in range(52, 67) for x in range(x0, 72)]

    def thigh(s, top):
        """Round 2b: a flat hint of his forward thigh where the old arm hung over it (its edge curving
        from x52 at row 60 out to the knee at row 66), so the inpainting draws the thigh there instead
        of leaving the space empty, as the first punch and block did."""
        for y in range(top, 67):
            e = 52 + max(0, y - 60)
            for x in range(46, e):
                if s.c.p[y][x] == r.T or x >= 47: s.c.p[y][x] = 12
            s.c.p[y][e] = 11

    # Crouching punch: his straight punch's arm (0x2126, x56-92, y11-31: sleeve, cuff and fist) driven out
    # from the forward shoulder at chest height; the shoulder join and the robe's side under it are inpainted.
    s = Src(88, 88); s.lay(N, 0, 0)
    for x, y in arm_gone(47): s.c.p[y][x] = r.T
    s.lay(r.grid(0x2126, 1, 2, 2).crop(56, 11, 37, 21), 48, 42)
    if HINT: thigh(s, 60); s.paint(38, 40, 61, 69, keep=head)
    else: s.paint(38, 40, 58, 69, keep=head)
    s.save(out, 'crouching-punch')

    # Crouching block: his standing block's raised arm (0x200c, x33-51, y9-41: the sleeve from the shoulder,
    # the grey cuff, forearm and fist; his other hand left out) on the forward shoulder, fist before his face;
    # the shoulder join and the space the old arm left below the sleeve are inpainted.
    s = Src(72, 88); s.lay(N, 0, 0)
    for x, y in arm_gone(47): s.c.p[y][x] = r.T
    s.lay(r.grid(0x200c, 1, 2, 1).crop(33, 9, 19, 33), 44, 26)
    if HINT: thigh(s, 59)
    s.paint(38, 40, 52, 52, keep=head)
    s.paint(40, 54, 64, 69)
    s.save(out, 'crouching-block')


def leg():
    """His flying kick's outstretched leg (0x2309, x43-95, y37-63), his hand above it left out: round one's crop (make_ground.py)."""
    g = r.grid(0x2309, 1, 1, 2).crop(43, 37, 53, 27)
    for y in range(0, 4):
        for x in range(10, g.w):
            if g.p[y][x] in GREY + SKIN: g.p[y][x] = r.T
    for y in range(0, 8):
        for x in range(10, g.w):
            if g.p[y][x] != r.T and all(g.at(x + dx, y + dy) == r.T for dx in (-1, 0, 1) for dy in (-1, 0, 1) if dx or dy): g.p[y][x] = r.T
    return g

def kick(out):
    """Round 2c: the crouching kick rebuilt on the round-two crouch, as round one's was on the first:
    his flying kick's outstretched leg (0x2309) from the hip along the floor in place of the forward
    knee and shoe, his stance's forward hand kept above the thigh; only the hip join is inpainted."""
    N = r.load(os.path.join(HERE, '..', 'touched2', 'crouch.png'))
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
    s.paint(34, 60, 58, 80, keep=lambda x, y: hand(x, y))
    s.save(out, 'crouching-kick')

if STEP == 'kick':
    kick(OUT)
elif STEP == 'crouch':
    s = Src(72, 88); s.c = upright()
    hand = lambda x, y: x >= 48 and y < 66 and s.c.at(x, y) in SKIN + GREY + (0,)
    fig = lambda x, y: any(s.c.at(x + dx, y) != r.T for dx in (-2, -1, 0, 1, 2))
    s.paint(4, 60, 66, 76, keep=lambda x, y: hand(x, y) or not fig(x, y))
    s.save(OUT, 'crouch')
else:
    derived(OUT)
