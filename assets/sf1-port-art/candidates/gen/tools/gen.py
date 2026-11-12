"""Gen's rebuilt poses as the recipe draws them (parts: crop, turn, mirror, at), for working on top of."""
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
SQ = dict(code=0x2064, large=True, rows=2, cols=1)
# His hit reaction's closing squat (0x2064) with rows 72-79 taken out of the thighs, so he squats lower.
CUT = [dict(rom=SQ, crop=[0, 0, 64, 72]), dict(rom=SQ, crop=[0, 80, 64, 16], at=[0, 72])]
def cut(): return build(CUT)
