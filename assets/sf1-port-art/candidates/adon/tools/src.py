"""An inpainting source: a pen canvas with ROM parts laid on it, and a mask (white = inpaint)."""
from PIL import Image
import rompose as r

class Src:
    def __init__(self, w, h):
        self.c = r.P(w, h); self.mask = set()
    def lay(self, g, ax, ay, keep=lambda x, y, v: True):
        """Paste g at (ax, ay); keep(x, y, pen) in g's own coordinates picks which opaque pixels."""
        for y in range(g.h):
            for x in range(g.w):
                v = g.p[y][x]
                if v != r.T and keep(x, y, v) and 0 <= ax + x < self.c.w and 0 <= ay + y < self.c.h:
                    self.c.p[ay + y][ax + x] = v
    def clear(self, x0, y0, x1, y1, keep=lambda x, y: False):
        for y in range(max(0, y0), min(self.c.h, y1)):
            for x in range(max(0, x0), min(self.c.w, x1)):
                if not keep(x, y): self.c.p[y][x] = r.T
    def paint(self, x0, y0, x1, y1, keep=lambda x, y: False):
        self.mask |= {(x, y) for y in range(max(0, y0), min(self.c.h, y1)) for x in range(max(0, x0), min(self.c.w, x1)) if not keep(x, y)}
    def save(self, out, name):
        w, h = self.c.w, self.c.h
        self.c.save(f"{out}/inpaint-source-{name}-{w}x{h}.png")
        m = Image.new('RGB', (w, h), (0, 0, 0)); px = m.load()
        for x, y in self.mask: px[x, y] = (255, 255, 255)
        m.save(f"{out}/inpaint-mask-{name}.png")
        pv = self.c.image(); pp = pv.load()
        for x, y in self.mask:
            a = pp[x, y]; pp[x, y] = (255, 0, 255, 255) if a[3] == 0 else ((a[0] + 255) // 2, a[1] // 2, (a[2] + 255) // 2, 255)
        big = Image.new('RGBA', (w * 2 + 4, h), (90, 110, 90, 255)); big.alpha_composite(self.c.image(), (0, 0)); big.alpha_composite(pv, (w + 4, 0))
        big.resize((big.width * 5, big.height * 5), Image.NEAREST).save(f"{out}/preview-{name}.png")
