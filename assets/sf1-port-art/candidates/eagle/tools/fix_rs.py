"""Touch-ups for the rising strike's shoulder (round one). fix(c) -> list of notes."""
import rompose as r
def fix(c):
    n = 0
    for y in range(60, 70):
        for x in range(62, 80):
            if c.p[y][x] != r.T: c.p[y][x] = r.T; n += 1
    for x in range(61, 80):
        if c.p[59][x] != r.T or x == 61: c.p[59][x] = 12
    for y in range(60, 68):
        xs = [x for x in range(50, 64) if c.p[y][x] != r.T]
        if xs: c.p[y][max(xs)] = 12
    return [f"{n} pixels of the turned sleeve's underside left hanging below it, under the armpit (x62-79, y60-69), removed; "
            "the sleeve's lowest row there (y59, x61-79) set to his outline pen (12), and the torso's front edge under the arm "
            "(y60-67), uncovered when the jab's arm was turned away, given his outline pen as the ROM draws it lower down"]
if __name__ == '__main__':
    import zoom
    c = r.load('../c3e4604b-0.png'); print(fix(c)); c.save('../look/rs_fixed.png'); zoom.zoom(c, '../look/zrs_fixed.png', 8, (40, 20, 104, 90))
