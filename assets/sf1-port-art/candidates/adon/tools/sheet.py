"""Contact sheet of poses: sheet.py out.png scale code:L|s:rows:cols ... (rows/cols are the ROM's count-less-one bytes)."""
import sys
from PIL import Image, ImageDraw
import rompose as r
def sheet(items, out, s=3, bg=(90, 110, 90, 255), cols=8):
    ims = []
    for label, p in items:
        im = p.image(); im = im.resize((im.width * s, im.height * s), Image.NEAREST)
        ims.append((label, im))
    cw = max(i.width for _, i in ims) + 8; ch = max(i.height for _, i in ims) + 20
    n = len(ims); cols = min(cols, n); rows = (n + cols - 1) // cols
    sh = Image.new('RGBA', (cw * cols, ch * rows), bg); d = ImageDraw.Draw(sh)
    for k, (label, im) in enumerate(ims):
        x, y = (k % cols) * cw, (k // cols) * ch
        sh.alpha_composite(im, (x + 4, y + 16)); d.text((x + 4, y + 2), label, fill=(255, 255, 255, 255))
    sh.save(out)
if __name__ == '__main__':
    out, s = sys.argv[1], int(sys.argv[2])
    items = []
    for a in sys.argv[3:]:
        c, sz, rr, cc = a.split(':')
        items.append((a, r.grid(int(c, 16), sz == 'L', int(rr), int(cc))))
    sheet(items, out, s)
