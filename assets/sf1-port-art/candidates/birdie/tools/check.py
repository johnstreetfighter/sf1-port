"""Check an inpainting candidate against its source: check.py <job8> <round> <name> [out.png]"""
import sys
from PIL import Image
import rompose as r, clean as k, zoom
job, rnd, name = sys.argv[1:4]
import glob
srcf = glob.glob(f"../{rnd}/inpaint-source-{name}-*.png")[0]
s = r.load(srcf); c = r.load(f"../{job}-0.png")
m = Image.open(f"../{rnd}/inpaint-mask-{name}.png").convert('RGB')
inside = lambda x, y: m.getpixel((x, y)) == (255, 255, 255)
bad = [(x, y) for y in range(c.h) for x in range(c.w) if not inside(x, y) and c.p[y][x] != s.p[y][x]]
print(name, 'offpal', k.offpal(f"../{job}-0.png"), 'outside diffs', len(bad), bad[:6])
xs = [x for y in range(c.h) for x in range(c.w) if inside(x, y)]; ys = [y for y in range(c.h) for x in range(c.w) if inside(x, y)]
box = (max(0, min(xs) - 6), max(0, min(ys) - 6), min(c.w, max(xs) + 7), min(c.h, max(ys) + 7))
if len(sys.argv) > 4: zoom.zoom(c, sys.argv[4], 8, box)
