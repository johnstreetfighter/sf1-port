"""Walk Birdie's ROM streams (read-only): frame records, 0xff 6-byte calls, 0xfe goto, 0xfd end."""
import sys
from streams import prog, rw, rl, sb
def walk(a, limit=40):
    out = []
    for _ in range(limit):
        b = prog[a]
        if b == 0xff:
            out.append(('call', a, rl(a + 2))); a += 6; continue
        if b == 0xfe:
            out.append(('goto', a, rl(a + 2))); break
        if b == 0xfd:
            out.append(('end', a)); break
        f = a
        code, attr = rw(f + 2), rw(f + 4)
        out.append(('frame', f, dict(hold=prog[f], dur=prog[f + 1], code=code, large=(attr >> 10) & 1,
                                     dy=sb(prog[f + 6]), dx=sb(prog[f + 7]), rows=prog[f + 8], cols=prog[f + 9],
                                     xoff=sb(prog[f + 0x1b]), fn=rl(f + 0x1c))))
        a += 0x20
    return out
if __name__ == '__main__':
    for s in sys.argv[1:]:
        print(s)
        for e in walk(int(s, 16)):
            if e[0] == 'frame':
                d = e[2]
                print(f"  {e[1]:06x} dur={d['dur']:3d} code={d['code']:04x} {'L' if d['large'] else 's'}{d['rows']}x{d['cols']} dy={d['dy']} dx={d['dx']} xoff={d['xoff']} fn={d['fn']:06x}")
            else:
                print('  ', e[0], hex(e[1]), *(hex(x) for x in e[2:]))
