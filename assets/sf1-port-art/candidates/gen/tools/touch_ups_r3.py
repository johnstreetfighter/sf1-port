"""Touch up Gen's round-three candidates (AP19, #183): the crouch, then the punch, block and kick on it.

usage: touch_ups_r3.py <candidates/gen folder> <out folder> crouch | derived

As rounds one and two: outside the mask every pixel must be the source's; inside, inpainting's
colours are taken to his nearest pens, single pixels no 4-neighbour shares are set to the pen
around them, and opaque specks under 4 pixels are removed. Writes <out>/<pose>.png and adds each
pose's touch-ups to <out>/touch-ups.json.
"""
import json, os, sys
from PIL import Image
import rompose as r, clean as k
C, OUT, STEP = sys.argv[1], sys.argv[2], sys.argv[3]
os.makedirs(OUT, exist_ok=True)
POSES = {  # pose, kept candidate, source folder, source name
    "crouch": [("crouch", "9c6213f5", "round3", "crouch-b-72x88")],
    "derived": [("crouching_punch", "4fd906fe", "round3", "crouching-punch-88x88"),
                ("crouching_block", "0dad208b", "round3", "crouching-block-72x88"),
                ("crouching_kick", "5bdb7565", "round3", "crouching-kick-96x88")],
}[STEP]
path = f"{OUT}/touch-ups.json"
report = json.load(open(path)) if os.path.exists(path) else {}
for pose, job, folder, name in POSES:
    src = r.load(f"{C}/{folder}/inpaint-source-{name}.png")
    m = Image.open(f"{C}/{folder}/inpaint-mask-{name.rsplit('-', 1)[0]}.png").convert('RGB')
    inside = lambda x, y: m.getpixel((x, y)) == (255, 255, 255)
    off = k.offpal(f"{C}/{job}-0.png")
    c = r.load(f"{C}/{job}-0.png")
    bad = [(x, y) for y in range(c.h) for x in range(c.w) if not inside(x, y) and c.p[y][x] != src.p[y][x]]
    assert not bad, (pose, bad[:5])
    drawn = sum(1 for y in range(c.h) for x in range(c.w) if inside(x, y) and c.p[y][x] != src.p[y][x])
    lone = k.lone(c, inside)
    extra = []
    if pose == "crouch":
        # Beside the panel's right trim, a corner of round two's short skirt was left on his right knee
        # (x 42-48, rows 76-81), outlined in red like a second hem: taken away, so the panel's edge is clean.
        n = 0
        for y in range(76, 82):
            for x in range(42, 49):
                if c.p[y][x] != r.T: c.p[y][x] = r.T; n += 1
        extra = [f"a corner of round two's short skirt left on his right knee beside the panel (x 42-48, rows 76-81, {n} pixels), outlined in red like a second hem, taken away so the panel's edge is clean"]
    if pose == "crouching_punch":
        # A hole between the sleeve's underside and the thigh, where the backdrop showed through.
        holes = [(x, 59) for x in range(44, 51)]
        assert all(c.p[y][x] == r.T for x, y in holes), "the punch's hole moved"
        for x, y in holes: c.p[y][x] = 11
        extra = ["7 transparent pixels between the sleeve's underside and the thigh (x 44-50, row 59), where the backdrop showed through, set to the dark purple of the sleeve's shadow"]
    if pose == "crouching_kick":
        # Two light-grey pixels the inpainting left beside the panel's red trim, where the thigh meets it.
        greys = [(41, 74), (41, 75)]
        assert all(c.p[y][x] in (8, 9, 10) for x, y in greys), "the kick's grey pixels moved"
        for x, y in greys: c.p[y][x] = 11
        extra = ["2 grey pixels beside the panel's red trim where the thigh meets it (x 41, rows 74-75) set to the dark purple around them"]
    isl = k.islands(c)
    c.save(f"{OUT}/{pose}.png")
    report[pose] = [f"outside the mask every pixel is the ROM source's; inside, {drawn} pixels differ from the source, and the {off} pixels the inpainting drew off his palette were taken to his nearest pens",
                    f"{lone} single pixels inside the mask set to the pen around them" + (f"; {isl} stray pixels removed" if isl else "")] + extra
json.dump(report, open(path, 'w', newline='\n'), indent=1)
print(json.dumps({p: report[p] for p, *_ in POSES}, indent=1))
