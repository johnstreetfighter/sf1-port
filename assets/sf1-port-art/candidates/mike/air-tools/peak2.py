"""Round two's Iron Upper peak: inpainted legs' bluish pixels taken to his nearest jeans pen, not his grey."""
import rompose as r, clean as k
from PIL import Image
def load_legs_blue(path, from_row):
    im = Image.open(path).convert('RGBA'); c = r.load(path); n = 0
    lum = lambda t: 0.3 * t[0] + 0.59 * t[1] + 0.11 * t[2]
    for y in range(from_row, im.height):
        for x in range(im.width):
            R, G, B, A = im.getpixel((x, y))
            if A < 128 or (R, G, B) in r.IDX: continue
            if B > R + 25:   # a blue the inpainting drew off his palette: his jeans pen of the nearest lightness
                p = min((11, 12, 13, 14), key=lambda i: abs(lum(r.RGB[i]) - lum((R, G, B))))
                if c.p[y][x] != p: n += 1
                c.p[y][x] = p
    return c, n
def lone_tape(c, region):
    """Single white or grey pixels with no white or grey beside them (stray tape colour): to the most common opaque
    neighbour, or clear when most neighbours are clear."""
    from collections import Counter
    n = 0
    for y in range(c.h):
        for x in range(c.w):
            if not region(x, y) or c.p[y][x] not in (6, 7): continue
            nb = [c.at(x + dx, y + dy) for dx in (-1, 0, 1) for dy in (-1, 0, 1) if dx or dy]
            if any(v in (6, 7) for v in nb): continue
            op = [v for v in nb if v != r.T]
            c.p[y][x] = Counter(op).most_common(1)[0][0] if len(op) >= 4 else r.T
            n += 1
    return n
