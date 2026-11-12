"""Rising Jaguar's knee (AP26): his rising pose (0x3244) with the front thigh turned up about the hip and the folded
shin, wrap and foot turned about the knee to hang under it."""
import math
import rompose as r, rot
R = r.grid(0x3244, 1, 2, 1)
HIP, KNEE = (27, 48), (44, 52)
FIRST_WRAP = {}
for y in range(53, 78):
    xs = [x for x in range(24 if y <= 74 else 28, 50) if R.p[y][x] in (11, 12, 13)]
    FIRST_WRAP[y] = min(xs) if xs else None


def part(x, y, v):
    """'thigh', 'lower' (front calf, wrap, foot) or None."""
    if 43 <= y <= 52 and x >= 24 and not (v in (0, 5, 6, 7) and x < 30): return 'thigh'
    if 53 <= y <= 58 and x >= 32: return 'lower'
    if 59 <= y <= 77:
        fw = FIRST_WRAP.get(y)
        if y >= 73 and x >= 29: return 'lower'
        if fw is not None and x >= fw: return 'lower'
        if y <= 60 and x >= 33: return 'lower'
    return None


def build(t1, t2):
    TH, LO, BODY = r.P(64, 96), r.P(64, 96), R.copy()
    for y in range(96):
        for x in range(64):
            v = R.p[y][x]
            if v == r.T: continue
            p = part(x, y, v)
            if p == 'thigh': TH.p[y][x] = v; BODY.p[y][x] = r.T
            elif p == 'lower': LO.p[y][x] = v; BODY.p[y][x] = r.T
    a = math.radians(t1); dx, dy = KNEE[0] - HIP[0], KNEE[1] - HIP[1]
    knee = (round(HIP[0] + dx * math.cos(a) + dy * math.sin(a)), round(HIP[1] - dx * math.sin(a) + dy * math.cos(a)))
    out = r.P(64, 96)
    out.paste(rot.turn(LO, t2, KNEE, 64, 96, knee), 0, 0)
    out.paste(BODY, 0, 0)
    out.paste(rot.turn(TH, t1, HIP, 64, 96, HIP), 0, 0)
    for y in range(36, 53):              # the shorts over the thigh's root
        for x in range(10, 32):
            v = R.p[y][x]
            if v in (0, 5, 6, 7) and part(x, y, v) is None: out.p[y][x] = v
    return out, BODY, knee
