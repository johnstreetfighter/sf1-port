"""Side-by-side zoomed view of candidates: view.py out.png scale a.png b.png ..."""
import sys
from PIL import Image
out, s = sys.argv[1], int(sys.argv[2])
ims = [Image.open(f).convert('RGBA') for f in sys.argv[3:]]
w = sum(i.width for i in ims) + 4 * len(ims); h = max(i.height for i in ims)
bg = Image.new('RGBA', (w, h), (90, 110, 90, 255)); x = 0
for i in ims: bg.alpha_composite(i, (x, 0)); x += i.width + 4
bg.resize((w * s, h * s), Image.NEAREST).save(out)
