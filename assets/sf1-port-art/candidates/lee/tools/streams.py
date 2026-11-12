"""Dump Lee's ROM stream frames (read-only)."""
import sys, json
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from project_paths import MAN, ROMS
m = json.load(open(MAN))
prog = bytearray(0x60000)
for e in m['entries']:
    if e.get('region') != 'maincpu': continue
    data = open(f"{ROMS}/{e['name']}", 'rb').read()
    base = e['offset'] & ~1; odd = e['offset'] & 1
    for i, b in enumerate(data): prog[base + 2*i + odd] = b
def rw(a): return prog[a] << 8 | prog[a+1]
def rl(a): return rw(a) << 16 | rw(a+2)
def sb(b): return b - 256 if b >= 128 else b
def dump(a, n=16):
    out = []
    for i in range(n):
        f = a + 0x20*i
        hold, dur, code, attr = prog[f], prog[f+1], rw(f+2), rw(f+4)
        dy, dx, rows, cols, xo = sb(prog[f+6]), sb(prog[f+7]), prog[f+8], prog[f+9], sb(prog[f+0x1b])
        print(f"  {f:06x}: hold={hold:02x} dur={dur:3d} code={code:04x} attr={attr:04x} large={(attr>>10)&1} dy={dy} dx={dx} rows={rows} cols={cols} flags={prog[f+0x1a]:02x} xoff={xo} fn={rl(f+0x1c):06x}")
        if hold & 0x80 or hold not in (0,1): break
if __name__ == '__main__':
    if sys.argv[1] == 'table':
        t = int(sys.argv[2], 16)
        for mv in range(16):
            s = rl(t + mv*0x30 + 0x2c)
            print(f"move {mv:x}: header pose {rw(t+mv*0x30):04x} stream {s:06x}")
    else:
        for a in sys.argv[1:]:
            print(a); dump(int(a, 16))
