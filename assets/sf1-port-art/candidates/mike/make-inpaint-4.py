"""Inpainting sources and masks for Mike's ground set, round four (AP15, #179).

usage: make-inpaint-4.py <folder holding rompose.py> <round three's touched-up folder>

the maintainer's round-three calls (2026-11-06):
- the punches still looked straight (his jab's forearm), and the redrawn
  shoulders did not join: so each striking arm is inpainted whole, from the
  shoulder to the wrist, bent, with only his ROM fist laid where the punch
  lands so it keeps its reach; his ROM head and the rest of the torso stay;
- the throw reads, but its joins are messy: so the clinch and headbutt are
  inpainted again only where the arms meet the shoulders and the forearms
  begin, keeping his head and the forearms from the wrist on;
- the crouching block's join still did not read: so the block is his
  approved crouch with only its forearms inpainted into a high guard, no
  waist join at all;
- the crouching uppercut was a different size from his crouch: so it is his
  ducking wind-up (his crouch's scale and height) with only the near arm
  inpainted, driving the fist up from the low position.
"""
import os
import sys

from PIL import Image

sys.path.insert(0, sys.argv[1])
import rompose as r  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
TU3 = sys.argv[2]
JAB = r.grid(0x1470, 1, 3, 2)
FIST = [68, 17, 12, 19]  # the jab's taped wrist and fist


class Source:
    def __init__(self, g):
        self.g = g
        self.h, self.w = len(g), len(g[0])
        self.white = set()
        self.keep = set()

    def erase(self, x0, y0, x1, y1):
        for y in range(y0, y1):
            for x in range(x0, x1):
                self.g[y][x] = 15

    def lay(self, grid, at, crop):
        x0, y0, cw, ch = crop
        for y in range(ch):
            for x in range(cw):
                p = grid[y0 + y][x0 + x]
                if p != 15:
                    self.g[at[1] + y][at[0] + x] = p
                    self.keep.add((at[0] + x, at[1] + y))

    def paint(self, x0, y0, x1, y1):
        self.white |= {(x, y) for y in range(y0, y1) for x in range(x0, x1)}

    def protect(self, x0, y0, x1, y1):
        self.keep |= {(x, y) for y in range(y0, y1) for x in range(x0, x1) if self.g[y][x] != 15}

    def save(self, name):
        r.save(self.g, os.path.join(HERE, f"inpaint-source-{name}-{self.w}x{self.h}.png"))
        m = Image.new("RGB", (self.w, self.h), (0, 0, 0))
        px = m.load()
        for x, y in self.white - self.keep:
            if 0 <= x < self.w and 0 <= y < self.h:
                px[x, y] = (255, 255, 255)
        m.save(os.path.join(HERE, f"inpaint-mask-{name}.png"))


def placed(code, rows, cols, w, h, at):
    c = r.blank(w, h)
    r.paste(c, r.grid(code, 1, rows, cols), at)
    return c


# Body hook: his hook's wind-up (0x1409). The near arm is inpainted whole,
# bent, from the shoulder to the jab's fist laid at the opponent's stomach.
s = Source(placed(0x1409, 2, 2, 96, 96, (0, 0)))
s.erase(16, 26, 52, 50)
s.protect(36, 0, 64, 33)          # his head and guard fist
s.lay(JAB, (74, 36), FIST)
s.paint(12, 18, 76, 62)
s.save("body-hook-4")

# Low body hook: his ducking wind-up (0x15aa), the near arm inpainted whole.
s = Source(placed(0x15AA, 1, 1, 96, 64, (0, 0)))
s.erase(24, 27, 66, 42)
s.protect(42, 0, 66, 27)          # his head
s.lay(JAB, (72, 28), FIST)
s.paint(10, 8, 74, 52)
s.save("low-body-hook-4")

# Overhand right: his jab's strike (0x1470), chest toward us; the striking
# arm inpainted whole from the shoulder, elbow high, to the fist laid low.
s = Source(placed(0x1470, 3, 2, 96, 128, (0, 0)))
s.erase(40, 14, 96, 38)
s.protect(14, 0, 42, 33)          # his head
s.protect(0, 33, 40, 128)         # his guard fist, torso and legs
s.lay(JAB, (70, 30), FIST)
s.paint(30, 0, 72, 52)
s.save("overhand-4")

# Crouching block: his approved crouch (0x156a), padded above; only his
# forearms and fists inpainted, raised into a high guard over his face.
s = Source(placed(0x156A, 1, 1, 72, 80, (0, 16)))
s.protect(40, 16, 62, 36)         # his head down to his eyes and cheek
s.protect(0, 46, 72, 80)          # his body below the arms and his legs
s.paint(28, 0, 72, 46)
s.save("crouching-block-4")

# Crouching uppercut: his ducking wind-up (0x15aa), at his crouch's scale,
# with room above only before his face for the rising fist.
s = Source(placed(0x15AA, 1, 1, 80, 80, (0, 16)))
s.protect(42, 16, 66, 43)         # his head
s.paint(12, 26, 80, 64)
s.paint(62, 0, 80, 40)
s.save("uppercut-4")

# Clinch and headbutt: round three's poses, the arms' joins only.
for name in ("clinch", "headbutt"):
    g = r.load(os.path.join(TU3, f"{name}.png"))
    s = Source(g)
    if name == "clinch":
        s.protect(38, 4, 60, 31)      # his head
    else:
        s.protect(46, 12, 68, 39)     # his head, moved
    s.protect(66, 0, 112, 112)        # the forearms from the wrist on
    s.protect(0, 50, 112, 112)        # below his chest
    s.paint(28, 16, 66, 50)
    s.save(f"{name}-4")

# The crouching uppercut's first inpainting this round drew no arm at all.
# The second lays the bent arm of round three's whole generation (44eb451d,
# candidate 0: forearm upright, fist up, x74-91, y16-49), which matches his
# ROM scale (his ROM head fitted it pixel for pixel), clear of the face of his
# ducking wind-up (0x15aa, his crouch's scale and height), takes out the
# wind-up's low forearm and fist, and inpaints only the upper arm and
# shoulder joining them.
s = Source(placed(0x15AA, 1, 1, 88, 80, (0, 16)))
s.erase(24, 43, 68, 59)
s.protect(42, 16, 66, 43)         # his head
s.protect(0, 59, 88, 80)          # his legs
s.protect(0, 16, 42, 43)          # his back and shoulder
cand = r.load(os.path.join(HERE, "44eb451d-0.png"))
s.lay(cand, (66, 8), [74, 16, 18, 34])
s.paint(24, 36, 86, 62)
s.save("uppercut-4b")