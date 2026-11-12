"""Round two for the smash (the maintainer, round one: "between the forearms should look transparent"): the maroon band
the two forearms' outlines make where they meet (x53-66, y40-51) is opened up, keeping one outline pixel on each
forearm, so the background shows between them. By hand; no generation."""
import rompose as r
SKIN = (2, 3, 4, 5)
def fix(c):
    n = 0
    gap = []
    for y in range(40, 52):
        for x in range(53, 67):
            if c.p[y][x] == 1 and c.at(x - 1, y) not in SKIN and c.at(x + 1, y) not in SKIN:
                gap.append((x, y))
    for x, y in gap: c.p[y][x] = r.T; n += 1
    return n, gap
if __name__ == '__main__':
    import json, sys
    c = r.load('../touched/smash.png'); n, gap = fix(c); print(n, gap)
    c.save(sys.argv[1])
