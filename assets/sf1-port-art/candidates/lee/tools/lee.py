"""Lee's rebuilt poses as the recipe draws them (parts: crop, turn, mirror, at), for working on top of."""
import rompose as r
def part(code, large, rows, cols, crop=None, turn=0, mirror=False):
    g = r.grid(code, large, rows, cols)
    if crop: g = g.crop(*crop)
    for _ in range(turn):
        t = r.P(g.h, g.w)
        for y in range(g.h):
            for x in range(g.w): t.p[x][g.h - 1 - y] = g.p[y][x]
        g = t
    return g.mirror() if mirror else g
def build(parts):
    ps = [(part(**p['rom'], crop=p.get('crop'), turn=p.get('turn', 0), mirror=p.get('mirror', False)), p.get('at', [0, 0])) for p in parts]
    w = max(p.w + a[0] for p, a in ps); h = max(p.h + a[1] for p, a in ps)
    c = r.P(w, h)
    for p, a in ps: c.paste(p, a[0], a[1])
    return c
SQ = dict(code=0x1f00, large=True, rows=1, cols=1)
CROUCH = [dict(rom=SQ, crop=[0, 0, 11, 64]), dict(rom=SQ, crop=[25, 0, 39, 64], at=[25, 0]),
          dict(rom=SQ, crop=[11, 18, 14, 46], at=[11, 18]), dict(rom=SQ, crop=[11, 0, 14, 18], mirror=True, at=[11, 0])]
def crouch(): return build(CROUCH)
