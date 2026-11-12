"""Inpainting sources and masks for Lee's ground set (AP17, #181), round one.

usage: make_ground.py <out folder>

Every source is his rebuilt crouch (lee.CROUCH: his sweep's closing squat 0x1f00,
the head turned to face the opponent) with ROM limbs from his own poses laid on it;
the mask (white) covers only the joins and what the old limb left behind.
Writes inpaint-source-<pose>-<w>x<h>.png, inpaint-mask-<pose>.png and a preview.
"""
import os, sys
from PIL import Image
import rompose as r, lee
OUT = sys.argv[1]
os.makedirs(OUT, exist_ok=True)

class Src:
    def __init__(self, w, h):
        self.c = r.P(w, h); self.mask = set()
    def lay(self, g, ax, ay):
        self.c.paste(g, ax, ay)
    def clear(self, x0, y0, x1, y1, keep=lambda x, y: False):
        for y in range(max(0, y0), min(self.c.h, y1)):
            for x in range(max(0, x0), min(self.c.w, x1)):
                if not keep(x, y): self.c.p[y][x] = r.T
    def paint(self, x0, y0, x1, y1, keep=lambda x, y: False):
        self.mask |= {(x, y) for y in range(max(0, y0), min(self.c.h, y1)) for x in range(max(0, x0), min(self.c.w, x1)) if not keep(x, y)}
    def save(self, name):
        w, h = self.c.w, self.c.h
        self.c.save(f"{OUT}/inpaint-source-{name}-{w}x{h}.png")
        m = Image.new('RGB', (w, h), (0, 0, 0)); px = m.load()
        for x, y in self.mask: px[x, y] = (255, 255, 255)
        m.save(f"{OUT}/inpaint-mask-{name}.png")
        pv = self.c.image(); pp = pv.load()
        for x, y in self.mask:
            a = pp[x, y]; pp[x, y] = (255, 0, 255, 255) if a[3] == 0 else ((a[0] + 255) // 2, a[1] // 2, (a[2] + 255) // 2, 255)
        big = Image.new('RGBA', (w * 2 + 4, h), (90, 110, 90, 255)); big.alpha_composite(self.c.image(), (0, 0)); big.alpha_composite(pv, (w + 4, 0))
        big.resize((big.width * 6, big.height * 6), Image.NEAREST).save(f"{OUT}/preview-{name}.png")

C = lee.crouch()
head = lambda x, y: x < 26 and y < 18          # his ROM head stays, pixel for pixel

# Crouching block: his standing block's raised forearm (0x1f40, x40-53, y8-40) stands on the
# crouch's elbow in place of its guard fist; the elbow, the shoulder and the old fist's place are inpainted.
s = Src(64, 72); s.lay(C, 0, 8)
s.clear(27, 8 + 5, 51, 8 + 30)
fore = r.grid(0x1f40, 1, 2, 2).crop(40, 8, 14, 33)
s.lay(fore, 32, 8 - 2)
keepfore = lambda x, y: 32 <= x < 46 and y < 8 + 19 and s.c.at(x, y) != r.T
s.paint(24, 8 + 0, 54, 8 + 33, keep=lambda x, y: keepfore(x, y) or head(x, y - 8))
s.save('crouching-block')

# Crouching punch: his straight punch's arm (0x1cc4, x34-63, y4-28: sleeve, forearm and fist) from the
# crouch's right shoulder; the shoulder join and the space the guard arm left are inpainted.
s = Src(64, 64); s.lay(C, 0, 0)
s.clear(27, 5, 51, 31)
arm = r.grid(0x1cc4, 1, 2, 1).crop(34, 4, 30, 25)
s.lay(arm, 28, 5)
s.paint(22, 4, 42, 33, keep=lambda x, y: head(x, y))
s.paint(36, 18, 52, 33)
s.save('crouching-punch')

# Crouching kick: his sweep's outstretched leg (0x1f04, x44-84, y38-61) from the crouch's right hip in
# place of its right knee; the hip and the space under the jacket's hem are inpainted.
s = Src(72, 64); s.lay(C, 0, 0)
s.clear(29, 40, 52, 64)
leg = r.grid(0x1f04, 1, 1, 2).crop(44, 38, 41, 24)
s.lay(leg, 30, 38)
for x in range(29, 72): s.c.p[62][x] = 0 if s.c.p[62][x] == r.T and x < 66 else s.c.p[62][x]
s.paint(24, 33, 50, 62, keep=lambda x, y: x < 30 and y >= 44)
s.save('crouching-kick')

# Crouching hit: his hunched hit reaction (0x1fa0) from the head to the hands (rows 13-53), laid on the crouch's
# hem and knees, lined up by the frog buttons; the band where they meet and the floor hand's place are inpainted.
s = Src(68, 64); s.lay(C, 4, 0)
s.clear(0, 0, 68, 34)
s.clear(18, 34, 44, 41)
s.clear(16 + 4, 43, 31 + 4, 63)
hit = r.grid(0x1fa0, 1, 2, 2).crop(8, 13, 50, 41)
s.lay(hit, 4 + 8 - 12, 0)
hands = lambda x, y: 18 <= x < 44 and y < 40 and (y < 34 or s.c.at(x, y) in (0, 1, 2, 3, 4, 5))   # his ROM hands stay, outlines and all
s.paint(0, 28, 64, 44, keep=hands)
s.paint(16 + 4, 43, 31 + 4, 62)
s.save('crouching-hit')
