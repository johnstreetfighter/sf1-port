"""An inpainting source: a pen grid built from ROM parts, and the mask (white) of what PixelLab redraws."""
from PIL import Image
import rompose as r

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
    def unpaint(self, x0, y0, x1, y1, where=lambda x, y: True):
        self.mask -= {(x, y) for y in range(y0, y1) for x in range(x0, x1) if where(x, y)}
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
        big.resize((big.width * 6, big.height * 6), Image.NEAREST).save(f"{out}/preview-{name}.png")
