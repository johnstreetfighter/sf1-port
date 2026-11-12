"""Inpainting sources and masks for Mike's ground set, round three (AP15, #179).

usage: make-inpaint-3.py <folder holding rompose.py>

the maintainer's round-two calls (2026-11-06): the strike arms did not reach out
the way they should, the bodies looked off, the throw's pixels and joins
were messy, and the crouching block and hit still did not read as one body
at the waist. So this round changes as little as it can:

- Each strike arm is his own ROM forearm and fist, laid where the punch
  lands: the jab's straight forearm (0x1470) for the hooks, the overhand
  and the clinch, and his victory pose's raised forearm (0x166c) for the
  uppercut. Only the join between his ROM shoulder and that forearm is
  inpainted, so the arm keeps his ROM length, thickness and fist.
- The crouching block and hit join their upper body to the crouch at the
  chest, where his leaning crouch still shows his hips, and only that join
  is inpainted.

Every pixel laid from the ROM is kept black in the mask, as is his head.
"""
import os
import sys

from PIL import Image

sys.path.insert(0, sys.argv[1])
import rompose as r  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
JAB = r.grid(0x1470, 1, 3, 2)
WIN = r.grid(0x166C, 1, 4, 1)
CROUCH = r.grid(0x156A, 1, 1, 1)
FOREARM = [46, 17, 34, 19]  # the jab's forearm and fist, skin to knuckles
RAISED = [12, 8, 16, 22]    # the victory pose's fist and forearm, upright


class Source:
    def __init__(self, w, h):
        self.w, self.h = w, h
        self.g = r.blank(w, h)
        self.keep = set()
        self.white = set()

    def lay(self, grid, at, crop=None, keep=True, clear=False):
        x0, y0, cw, ch = crop if crop else (0, 0, len(grid[0]), len(grid))
        for y in range(ch):
            for x in range(cw):
                p = grid[y0 + y][x0 + x]
                tx, ty = at[0] + x, at[1] + y
                if 0 <= tx < self.w and 0 <= ty < self.h and (p != 15 or clear):
                    self.g[ty][tx] = p
                    if keep and p != 15:
                        self.keep.add((tx, ty))
                    else:
                        self.keep.discard((tx, ty))

    def erase(self, x0, y0, x1, y1):
        for y in range(y0, y1):
            for x in range(x0, x1):
                self.g[y][x] = 15

    def paint(self, x0, y0, x1, y1):
        self.white |= {(x, y) for y in range(y0, y1) for x in range(x0, x1)}

    def save(self, name):
        r.save(self.g, os.path.join(HERE, f"inpaint-source-{name}-{self.w}x{self.h}.png"))
        m = Image.new("RGB", (self.w, self.h), (0, 0, 0))
        px = m.load()
        for x, y in self.white - self.keep:
            if 0 <= x < self.w and 0 <= y < self.h:
                px[x, y] = (255, 255, 255)
        m.save(os.path.join(HERE, f"inpaint-mask-{name}.png"))


def chest_join(up, cut_up, crouch_from, dx, dy, w=72, h=96):
    """An upper body on the crouch, joined at the chest (round three)."""
    s = Source(w, h)
    oy = h - 64
    s.lay(CROUCH, (4, oy + crouch_from), (0, crouch_from, 40, 64 - crouch_from))
    s.lay(CROUCH, (44, oy + 31), (40, 31, 24, 33))
    s.lay(up, (4 + dx, oy + crouch_from + dy - cut_up), (0, 0, len(up[0]), cut_up))
    s.keep = {(x, y) for y in range(h) for x in range(w) if s.g[y][x] != 15}
    return s, oy + crouch_from + dy


# Crouching block: his guard (0x1648) to the chest on his crouch.
s, join = chest_join(r.grid(0x1648, 1, 2, 1), 30, 18, 0, 6)
s.paint(0, join - 6, 72, join + 8)
s.keep = {p for p in s.keep if not (join - 6 <= p[1] < join + 8)}
s.save("crouching-block-3")

# Crouching hit: his hit pose (0x160c) to the chest on his crouch.
s, join = chest_join(r.grid(0x160C, 1, 2, 1), 40, 22, 0, 8)
s.paint(0, join - 6, 72, join + 8)
s.keep = {p for p in s.keep if not (join - 6 <= p[1] < join + 8)}
s.save("crouching-hit-3")

# Body hook: his hook's wind-up (0x1409); the folded near forearm goes, the
# jab's forearm is laid at the opponent's stomach height, the elbow joins.
s = Source(96, 96)
s.lay(r.grid(0x1409, 1, 2, 2), (0, 0))
s.keep = {(x, y) for y in range(96) for x in range(96) if s.g[y][x] != 15}
s.paint(16, 26, 52, 50)
s.keep -= s.white
s.erase(16, 26, 52, 50)
s.lay(JAB, (52, 32), FOREARM)
s.paint(40, 30, 58, 52)
s.save("body-hook-3")

# Low body hook: his ducking wind-up (0x15aa); the low forearm goes, the
# jab's forearm is laid at belt height reaching out, the elbow joins.
s = Source(96, 64)
s.lay(r.grid(0x15AA, 1, 1, 1), (0, 0))
s.keep = {(x, y) for y in range(64) for x in range(96) if s.g[y][x] != 15}
HEAD = {(x, y) for y in range(0, 27) for x in range(42, 66)}
s.paint(24, 27, 66, 42)
s.keep -= s.white
s.erase(24, 27, 66, 42)
s.lay(JAB, (52, 24), FOREARM)
s.paint(20, 18, 58, 44)
s.white -= HEAD
s.save("low-body-hook-3")

# Overhand right: his jab's strike (0x1470), facing us; the jab's forearm is
# laid 8 pixels lower, and his shoulder and upper arm are inpainted rising
# over the top of it.
s = Source(96, 128)
s.lay(JAB, (0, 0))
s.erase(40, 14, 96, 38)
s.keep = {(x, y) for y in range(128) for x in range(96) if s.g[y][x] != 15}
s.lay(JAB, (FOREARM[0] + 4, FOREARM[1] + 8), FOREARM)
s.paint(30, 4, 64, 40)
s.save("overhand-3")

# Crouching uppercut: his victory pose (0x166c) to the chest on his crouch,
# its straight raised arm gone; his victory fist and forearm laid upright
# before his face, the upper arm and elbow inpainted below it.
s, join = chest_join(WIN, 68, 18, 0, 6, 72, 136)
up_top = join - 68
s.erase(0, 0, 32, up_top + 50)
s.keep = {(x, y) for y in range(136) for x in range(72) if s.g[y][x] != 15}
head = {(x, up_top + y) for y in range(32, 58) for x in range(26, 52) if WIN[y][x] in (0, 1, 2, 3, 4, 5, 6, 7)}
s.lay(WIN, (52, up_top + 24), RAISED)
s.paint(8, up_top + 36, 30, up_top + 60)
s.paint(30, up_top + 54, 72, up_top + 78)
s.paint(46, up_top + 40, 72, up_top + 56)
s.paint(0, join - 6, 72, join + 8)
s.keep = {p for p in s.keep if not (join - 6 <= p[1] < join + 8)}
s.keep |= head
s.save("uppercut-3b")

# Clinch: his straight's step-in (0x147b); both fists move out to the
# opponent's collar as the jab's forearm, the shoulders join.
s = Source(112, 112)
s.lay(r.grid(0x147B, 1, 3, 1), (0, -4))
s.keep = {(x, y) for y in range(112) for x in range(112) if s.g[y][x] != 15}
s.erase(18, 20, 66, 40)
s.keep = {p for p in s.keep if not (18 <= p[0] < 66 and 20 <= p[1] < 40)}
s.lay(JAB, (52, 22), FOREARM)
s.lay(JAB, (56, 28), FOREARM)
s.paint(14, 18, 58, 46)
s.save("clinch-3")

# The overhand's first try this round drew the upper arm arching over to a
# point past the laid forearm's start, so the two did not meet. The second
# lays the forearm from the elbow on (8 pixels shorter) further out, where
# that arch comes down.
s = Source(96, 128)
s.lay(JAB, (0, 0))
s.erase(40, 14, 96, 38)
s.keep = {(x, y) for y in range(128) for x in range(96) if s.g[y][x] != 15}
s.lay(JAB, (58, FOREARM[1] + 8), [54, 17, 26, 19])
s.paint(30, 4, 62, 42)
s.save("overhand-3b")
# The uppercut's first try this round joined the arm to the victory pose's
# other fist at his chest and left the laid forearm floating. The second
# takes that fist out too, lays the upright forearm lower, its fist level
# with his face, and inpaints only the channel from its wrist down to the
# elbow at his chest and back up to his shoulder.
s, join = chest_join(WIN, 68, 18, 0, 6, 72, 136)
up_top = join - 68
s.erase(0, 0, 32, up_top + 50)
s.erase(42, up_top + 48, 68, up_top + 70)
s.keep = {(x, y) for y in range(136) for x in range(72) if s.g[y][x] != 15}
head = {(x, up_top + y) for y in range(32, 58) for x in range(26, 52) if WIN[y][x] in (0, 1, 2, 3, 4, 5, 6, 7)}
s.lay(WIN, (50, up_top + 30), RAISED)
s.paint(40, up_top + 44, 72, up_top + 72)
s.paint(0, join - 6, 72, join + 8)
s.keep = {p for p in s.keep if not (join - 6 <= p[1] < join + 8)}
s.keep |= head
s.save("uppercut-3c")