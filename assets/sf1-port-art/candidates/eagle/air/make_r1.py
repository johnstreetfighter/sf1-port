"""Round one's inpainting source and mask (AP24, #188): the jumping stick swing's waist. usage: make_r1.py <round>"""
import json, os, sys
import air as a, rompose as r
sys.path.insert(0, '../tools')
from src import Src
RND = sys.argv[1]; os.makedirs(RND, exist_ok=True)
c = a.swing_canvas()
s = Src(96, a.H)
s.lay(c.crop(a.SWING_CROP, 0, 96, a.H), 0, 0)
for y in range(40, 49):          # the transparent row between the two pieces given the trousers' row below it
    for x in range(96):
        if s.c.p[y][x] == r.T and s.c.at(x, y - 1) != r.T and s.c.at(x, y + 1) != r.T: s.c.p[y][x] = s.c.p[y + 1][x]
near = lambda x, y: any(s.c.at(x + dx, y + dy) != r.T for dx in (-1, 0, 1) for dy in (-1, 0, 1))
X0, X1 = 44 - a.SWING_CROP, 77 - a.SWING_CROP
s.mask = {(x, y) for y in range(41, 49) for x in range(X0, X1) if near(x, y)}
s.save(RND, 'jump-swing')
json.dump({'jump-swing': {'crop_x': a.SWING_CROP}}, open(f"{RND}/origins.json", 'w'), indent=1)
