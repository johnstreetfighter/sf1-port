import rompose as r
from PIL import Image
tuck=r.grid(0x1608,1,1,1)
def zoom(c, f, box=None, s=8):
    im=c.image()
    if box: im=im.crop(box)
    im=im.resize((im.width*s,im.height*s),Image.NEAREST); bg=Image.new('RGBA',im.size,(90,110,90,255)); bg.alpha_composite(im); bg.save(f)
def on_tuck(upper, dx, dy, cut=30, strip_rows=None, W=112, H=80, oy=12):
    """tuck rows cut.. at (0,oy+cut); upper (a full grid) at (dx, oy+dy)."""
    c=r.P(W,H)
    legs=tuck.crop(0,cut,64,64-cut); c.paste(legs,0,oy+cut)
    up=upper.copy()
    if strip_rows:
        y0,y1=strip_rows
        for y in range(y0,y1):
            for x in range(up.w):
                if up.p[y][x] in (11,12,13,14): up.p[y][x]=r.T
        for y in range(y1, up.h):
            up.p[y]=[r.T]*up.w
    c.paste(up,dx,oy+dy)
    return c
