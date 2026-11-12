"""dump.py code L|s rows cols x0 y0 x1 y1: a ROM pose's pens as text (. transparent), absolute x in the header."""
import sys
import rompose as r
def show(g, x0, y0, x1, y1):
    print('    ' + ''.join('%x' % (x % 16) for x in range(x0, x1)))
    for y in range(y0, y1): print('%3d ' % y + ''.join('.' if g.at(x, y) == r.T else '%x' % g.at(x, y) for x in range(x0, x1)))
if __name__ == '__main__':
    a = sys.argv[1:]
    show(r.grid(int(a[0], 16), a[1] == 'L', int(a[2]), int(a[3])), *map(int, a[4:8]))
