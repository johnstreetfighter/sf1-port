"""Line a donor pose up on another by a region's pixels (his head): best (dx, dy) and the match share."""
import rompose as r
def align(base, box, donor, search=24):
    x0, y0, x1, y1 = box
    pts = [(x, y, base.p[y][x]) for y in range(y0, y1) for x in range(x0, x1) if base.p[y][x] != r.T]
    best = None
    for dy in range(-search, search + 1):
        for dx in range(-search, search + 1):
            n = sum(1 for x, y, v in pts if donor.at(x + dx, y + dy) == v)
            if best is None or n > best[0]: best = (n, dx, dy)
    return best[1], best[2], best[0] / len(pts)
