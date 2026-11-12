"""Inpainting source and mask for Adon's get-up rising pose (AP25, #189).

usage: make_rising.py <round folder> [deg drop]

His ROM art has no pose that reads as getting up. Getting up off one knee, leaning far forward:
- the upper body of his body punch's strike (0x3304, stream 0x1336c, rows 0-43): bent forward, rear fist at his
  chin, front arm reaching forward and down; turned `deg` further forward about its waist (rot.py), with his ROM
  head (x43-59, rows 3-19) laid back unturned at the turned head's place, every pixel his;
- on his crouch's legs with the seat `drop` pixels lower: the crouch's shorts and belt lowered, its front thigh,
  knee, ankle wrap and foot planted where they are; the rear leg brought down onto its knee, the shin back along
  the floor (his stance's rear shin, wrap and foot turned to lie along the floor as a guide).
The mask covers the waist band, the hip where the lowered shorts meet the front thigh, and the rear leg below the
seat; outside it every pixel is his ROM art.
"""
import math, os, sys
import rompose as r, adon, rot
from src import Src
RND = sys.argv[1]
DEG = int(sys.argv[2]) if len(sys.argv) > 2 else -20
DROP = int(sys.argv[3]) if len(sys.argv) > 3 else 5
os.makedirs(RND, exist_ok=True)
W, H = 96, 72
C = adon.crouch(1)
P = r.grid(0x324c, 1, 2, 1)
S = r.grid(0x3060, 1, 2, 1)
PIVOT = (24, 46)                       # the hit recovery's waist
BELT = (22, 41 + DROP)                 # the crouch's belt centre, lowered

HEAD = (27, 13, 19, 12)                # his head, x27-45, rows 13-24: hair, brow, eyes, nose and open mouth
in_head = lambda x, y: HEAD[0] <= x < HEAD[0] + HEAD[2] and HEAD[1] <= y < HEAD[1] + HEAD[3] and P.p[y][x] != r.T

def turned_point(x, y, deg, pivot, at):
    t = math.radians(deg); dx, dy = x - pivot[0], y - pivot[1]
    # rot.turn maps canvas -> source by (dx cos - dy sin, dx sin + dy cos); invert it
    return (at[0] + dx * math.cos(t) + dy * math.sin(t), at[1] - dx * math.sin(t) + dy * math.cos(t))

def source():
    s = Src(W, H)
    # Legs: the crouch's front leg where it is; its belt and shorts (x < 31 above row 57) lowered by DROP.
    for y in range(40, H):
        for x in range(C.w):
            v = C.p[y][x]
            if v == r.T: continue
            if x >= 31 and y < 57: s.c.p[y][x] = v                  # front thigh and knee
            elif x >= 36 and y >= 55: s.c.p[y][x] = v               # front ankle wrap, foot, its shadow
    for y in range(40, 57):
        for x in range(0, 31):
            v = C.p[y][x]
            if v != r.T and y + DROP < H: s.c.p[y + DROP][x] = v
    # Rear leg on its knee, the shin back along the floor: his stance's rear shin, wrap and foot (x8-25,
    # rows 66-95) turned to lie along the floor, the knee under the seat.
    shin = S.crop(0, 60, 30, 36)
    t = rot.turn(shin, -80, (18, 4), W, H, (24, 64))
    for y in range(H):
        for x in range(W):
            if t.p[y][x] != r.T and s.c.p[y][x] == r.T: s.c.p[y][x] = t.p[y][x]
    for y in (69, 70):                                            # his ground shadow, under both legs
        for x in range(0, 56):
            if s.c.p[y][x] == r.T: s.c.p[y][x] = 0
    # Upper body, turned forward about the waist, the waist on the lowered belt.
    up = r.P(96, 96)
    for y in range(0, 55):
        for x in range(P.w):
            if (y < 46 or (x >= 36 and y < 54)) and not in_head(x, y): up.p[y][x] = P.p[y][x]
    if DEG:
        tb = rot.turn(up, DEG, PIVOT, W, H, BELT)
    else:                                   # unturned: his ROM pixels as they are, the waist on the belt
        tb = r.P(W, H); tb.paste(up, BELT[0] - PIVOT[0], BELT[1] - PIVOT[1])
    for y in range(H):
        for x in range(W):
            if tb.p[y][x] != r.T and (y < BELT[1] - 1 or x >= 40): s.c.p[y][x] = tb.p[y][x]
    hx, hy = turned_point(HEAD[0] + HEAD[2] / 2, HEAD[1] + HEAD[3] / 2, DEG, PIVOT, BELT)
    head = P.crop(*HEAD)
    ax, ay = round(hx - HEAD[2] / 2), round(hy - HEAD[3] / 2)
    s.lay(head, ax, ay)
    return s, (ax, ay)

if __name__ == '__main__':
    s, (ax, ay) = source()
    keep_head = lambda x, y: ax <= x < ax + HEAD[2] and ay <= y < ay + HEAD[3] and s.c.at(x, y) != r.T
    by = BELT[1]
    s.paint(6, by - 5, 31, by + 2, keep=lambda x, y: (y >= by and s.c.at(x, y) in (0, 5, 6, 7)))  # waist band
    s.paint(24, 44 + DROP - 4, 40, 58, keep=lambda x, y: x < 31 and s.c.at(x, y) in (0, 5, 6, 7) and y < 57)  # hip
    s.paint(0, 56, 36, 69)                                                          # rear leg below the seat
    if DEG: s.paint(ax - 2, ay + HEAD[3] - 3, ax + HEAD[2] + 2, ay + HEAD[3] + 4, keep=keep_head)                      # the neck under his head
    s.save(RND, 'rising')
    print(s.c.bbox(), (ax, ay))
