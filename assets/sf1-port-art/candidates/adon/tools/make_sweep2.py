"""Inpainting source and mask for Adon's sweep, round two (AP25, #189).

usage: make_sweep2.py <round folder> <rising.png> [leg_deg upper_dx upper_dy stretch]

the maintainer, round one: "Too like the crouching kick", "Doesn't read as a sweep". The sweep is now its own shape, low
and leaning in, as Ryu's sweep (0x054b) reads:
- his body dropped onto the rising pose's kneeling legs (the seat 7 pixels below the crouch's, the rear knee down
  and its shin back along the floor; its front leg left out);
- leaning toward the sweep: the upper body of his body punch's strike (0x3304, stream 0x1336c, rows 0-41), bent
  forward, rear fist at his chin, front arm reaching forward, its belt on the kneel's lowered belt;
- the sweeping leg: his low kick's leg (0x31e4) turned `leg_deg` up about the hip so it runs long and nearly flat,
  the foot out at ground level, the shin lengthened `stretch` pixels by repeating a run of its own columns (the
  turned leg is nearly level, so the columns run across it), so the foot reaches well out.
The mask covers the waist band, and the hip and thigh where the leg leaves the shorts.
"""
import os, sys
import rompose as r, rot
from src import Src
RND, RISING = sys.argv[1], sys.argv[2]
LEG_DEG = int(sys.argv[3]) if len(sys.argv) > 3 else 31
UDX = int(sys.argv[4]) if len(sys.argv) > 4 else -16
UDY = int(sys.argv[5]) if len(sys.argv) > 5 else 7
os.makedirs(RND, exist_ok=True)
W, H = 112, 72
K = r.grid(0x31e4, 1, 2, 2)
P = r.grid(0x3304, 1, 2, 2)
LEG = r.P(96, 96)
for y in range(51, 92):
    for x in range(45, 96):
        v = K.p[y][x]
        if v != r.T and not (y < 62 and v in (0, 5, 6, 7)): LEG.p[y][x] = v
LEG_HIP = (52, 58)
HIP = (34, 58)                                   # the front hip on the kneel, under the shorts' front
STRETCH = int(sys.argv[6]) if len(sys.argv) > 6 else 8
SHIN_X = 55                                      # canvas column inside the shin, past the knee, before the wrap


def source():
    s = Src(W, H)
    R = r.load(RISING)                           # packed rising: 58x56, its canvas offset (0, 15)
    kneel = r.P(W, H)
    for y in range(R.h):
        for x in range(R.w):
            v = R.p[y][x]
            cy = y + 15
            if v != r.T and cy >= 47 and x < 31: kneel.p[cy][x] = v          # shorts, seat, rear leg, shadow
    leg = rot.turn(LEG, LEG_DEG, LEG_HIP, W, H, HIP)
    if STRETCH:                                  # the shin lengthened along itself: STRETCH columns of it repeated
        long = r.P(W, H)
        for y in range(H):
            for x in range(W): long.p[y][x] = leg.at(x if x < SHIN_X else x - STRETCH, y)
        leg = long
    s.lay(leg, 0, 0)
    s.lay(kneel, 0, 0)
    for y in (69, 70):
        for x in range(0, 30):
            if s.c.p[y][x] == r.T and kneel.p[y][x] == r.T: pass
    up = r.P(W, H)
    for y in range(0, 42):
        for x in range(P.w):
            v = P.p[y][x]
            if v != r.T and 0 <= x + UDX < W and 0 <= y + UDY < H: up.p[y + UDY][x + UDX] = v
    s.lay(up, 0, 0)
    return s


if __name__ == '__main__':
    s = source()
    s.paint(6, 42 + UDY - 3, 40, 51, keep=lambda x, y: y >= 48 and s.c.at(x, y) in (0, 5, 6, 7))   # waist band
    s.paint(24, 50, 52, 64, keep=lambda x, y: x < 31 and s.c.at(x, y) in (0, 5, 6, 7) and y < 58)  # hip and thigh
    s.save(RND, 'sweep')
    print(s.c.bbox())
