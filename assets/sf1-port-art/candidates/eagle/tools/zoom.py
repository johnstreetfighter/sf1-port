"""Zoomed picture of a pen grid with a coordinate grid every 8 pixels."""
from PIL import Image, ImageDraw
import rompose as r
def zoom(c, f, s=8, box=None):
    im = c.image()
    x0, y0 = (box[0], box[1]) if box else (0, 0)
    if box: im = im.crop(box)
    im = im.resize((im.width * s, im.height * s), Image.NEAREST)
    bg = Image.new('RGBA', im.size, (90, 110, 90, 255)); bg.alpha_composite(im)
    d = ImageDraw.Draw(bg)
    for x in range(0, im.width, s):
        if (x // s + x0) % 8 == 0:
            d.line([(x, 0), (x, im.height)], fill=(255, 255, 0, 90)); d.text((x + 1, 1), str(x // s + x0), fill=(255, 255, 0, 255))
    for y in range(0, im.height, s):
        if (y // s + y0) % 8 == 0:
            d.line([(0, y), (im.width, y)], fill=(255, 255, 0, 90)); d.text((1, y + 1), str(y // s + y0), fill=(255, 255, 0, 255))
    bg.save(f)
