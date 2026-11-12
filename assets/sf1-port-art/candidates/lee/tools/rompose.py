"""Lee's ROM poses as pen grids (uncropped, the recipe's coordinates). Read-only use of the ROM set."""
import json
from PIL import Image
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from project_paths import MAN, ROMS
PAL = ["#000000", "#664400", "#996655", "#cc9977", "#ddaa88", "#ddccaa", "#ee4455", "#4444dd", "#666677", "#888899",
       "#aaaabb", "#444466", "#444477", "#444499", "#4444bb"]
RGB = [tuple(int(h[i:i + 2], 16) for i in (1, 3, 5)) for h in PAL]
IDX = {c: n for n, c in enumerate(RGB)}
T = 15
_g = None
def gfx3():
    global _g
    if _g is None:
        m = json.load(open(MAN))
        parts = sorted([e for e in m['entries'] if e.get('region') == 'gfx3'], key=lambda e: e['offset'])
        size = max(e['offset'] + e['size'] for e in parts)
        buf = bytearray(size)
        for e in parts:
            b = open(f"{ROMS}/{e['name']}", 'rb').read(); buf[e['offset']:e['offset'] + len(b)] = b
        _g = bytes(buf)
    return _g
def invert(code): return code ^ [0, 0x18, 0x18, 0][(code >> 3) & 3]
def tile(idx):
    g = gfx3(); half = len(g) // 2 * 8
    po = [4, 0, half + 4, half]; xo = [[0, 8, 256, 264][i // 4] + i % 4 for i in range(16)]
    base = idx * 512; out = []
    for y in range(16):
        row = []
        for x in range(16):
            v = 0
            for p, off in enumerate(po):
                o = off + base + xo[x] + y * 16
                v |= ((g[o // 8] >> (7 - o % 8)) & 1) << (3 - p)
            row.append(v)
        out.append(row)
    return out
class P:
    def __init__(self, w, h, fill=T):
        self.w, self.h = w, h; self.p = [[fill] * w for _ in range(h)]
    def at(self, x, y): return self.p[y][x] if 0 <= x < self.w and 0 <= y < self.h else T
    def copy(self):
        c = P(self.w, self.h); c.p = [r[:] for r in self.p]; return c
    def crop(self, x0, y0, w, h):
        c = P(w, h)
        for y in range(h):
            for x in range(w): c.p[y][x] = self.at(x0 + x, y0 + y)
        return c
    def paste(self, src, ax, ay, over=True):
        for y in range(src.h):
            for x in range(src.w):
                v = src.p[y][x]
                if (v != T or not over) and 0 <= ax + x < self.w and 0 <= ay + y < self.h: self.p[ay + y][ax + x] = v
    def mirror(self):
        c = P(self.w, self.h); c.p = [r[::-1] for r in self.p]; return c
    def bbox(self):
        xs = [x for y in range(self.h) for x in range(self.w) if self.p[y][x] != T]
        ys = [y for y in range(self.h) for x in range(self.w) if self.p[y][x] != T]
        return (min(xs), min(ys), max(xs) + 1, max(ys) + 1) if xs else None
    def image(self):
        im = Image.new('RGBA', (self.w, self.h), (0, 0, 0, 0)); px = im.load()
        for y in range(self.h):
            for x in range(self.w):
                v = self.p[y][x]
                if v != T: px[x, y] = RGB[v] + (255,)
        return im
    def save(self, f): self.image().save(f)
def grid(code, large, rows, cols):
    cell = 32 if large else 16
    out = P((cols + 1) * cell, (rows + 1) * cell)
    g = gfx3(); n = len(g) // 128
    for r in range(rows + 1):
        for c in range(cols + 1):
            base = code + r * (0x20 if large else 0x10) + c * (2 if large else 1)
            subs = [(base, 0, 0), (base + 1, 16, 0), (base + 16, 0, 16), (base + 17, 16, 16)] if large else [(base, 0, 0)]
            for tc, ox, oy in subs:
                t = tile(invert(tc & 0xffff) % n)
                for y in range(16):
                    for x in range(16): out.p[r * cell + oy + y][c * cell + ox + x] = t[y][x]
    return out
def load(path):
    """An RGBA picture as pens: alpha < 128 transparent, else exact colour or nearest."""
    im = Image.open(path).convert('RGBA'); px = im.load(); out = P(im.width, im.height)
    for y in range(im.height):
        for x in range(im.width):
            r, g, b, a = px[x, y]
            if a < 128: continue
            out.p[y][x] = IDX.get((r, g, b)) if (r, g, b) in IDX else min(range(15), key=lambda i: sum((u - v) ** 2 for u, v in zip(RGB[i], (r, g, b))))
    return out
