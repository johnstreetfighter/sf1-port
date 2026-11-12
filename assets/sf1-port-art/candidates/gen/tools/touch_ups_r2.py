"""Touch up Gen's round-two candidates (AP19, #183): the crouch, then the punch and block built on it.

usage: touch_ups_r2.py <candidates/gen folder> <out folder> crouch | derived | derived-b | kick

As round one's: outside the mask every pixel must be the source's; inside, inpainting's
colours are taken to his nearest pens, single pixels no 4-neighbour shares are set to the pen
around them, and opaque specks under 4 pixels are removed. Writes <out>/<pose>.png and adds
each pose's touch-ups to <out>/touch-ups.json.
"""
import json, os, sys
from PIL import Image
import rompose as r, clean as k
C, OUT, STEP = sys.argv[1], sys.argv[2], sys.argv[3]
os.makedirs(OUT, exist_ok=True)
POSES = {  # pose, kept candidate, source folder, source name
    "crouch": [("crouch", "8a1d8eba", "round2", "crouch-72x88")],
    "derived": [("crouching_punch", "a7bd1f35", "round2", "crouching-punch-88x88"),
                ("crouching_block", "9cd2e6dd", "round2", "crouching-block-72x88")],
    # Round 2b: the first punch and block left the space under the arm empty; these have the thigh hinted there.
    "derived-b": [("crouching_punch", "5cf70eb1", "round2b", "crouching-punch-88x88"),
                  ("crouching_block", "874a79b7", "round2b", "crouching-block-72x88")],
    # Round 2c: the crouching kick rebuilt on the round-two crouch.
    "kick": [("crouching_kick", "f28b3bbd", "round2c", "crouching-kick-96x88")],
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
    isl = k.islands(c)
    extra = []
    if STEP == "derived-b" and pose == "crouching_punch":
        # Two small holes between the sleeve's underside and the thigh, where the backdrop showed through.
        holes = [(47, 58), (48, 58), (50, 59), (51, 59), (52, 59)]
        assert all(c.p[y][x] == r.T for x, y in holes), "the punch's holes moved"
        for x, y in holes: c.p[y][x] = 11
        extra = ["5 transparent pixels between the sleeve's underside and the thigh (x 47-48 row 58, x 50-52 row 59), where the backdrop showed through, set to the dark purple of the sleeve's shadow"]
    if STEP == "kick":
        # The inpainting cleared the robe's front panel just above its hem, leaving the hem hanging
        # under empty space: those rows go back to the source's (his ROM pixels).
        n = 0
        for y in range(76, 80):
            for x in range(34, 44):
                if c.p[y][x] != src.p[y][x]: c.p[y][x] = src.p[y][x]; n += 1
        # Two black pixels inside the sleeve (x 45-46, row 61): the leg crop's top outline left in the source.
        for x in (45, 46):
            assert c.p[61][x] == 0, "the kick's stray pixels moved"
            c.p[61][x] = 11
        extra = [f"his robe's front panel just above its hem (x 34-43, rows 76-79, {n} pixels) set back to the source's: the inpainting had cleared it and left the hem hanging under empty space",
                 "2 black pixels inside the sleeve (x 45-46, row 61), the leg crop's top outline, set to the sleeve's dark purple"]
    c.save(f"{OUT}/{pose}.png")
    report[pose] = [f"outside the mask every pixel is the ROM source's; inside, {drawn} pixels differ from the source, and the {off} pixels the inpainting drew off his palette were taken to his nearest pens",
                    f"{lone} single pixels inside the mask set to the pen around them" + (f"; {isl} stray pixels removed" if isl else "")] + extra
json.dump(report, open(path, 'w', newline='\n'), indent=1)
print(json.dumps({p: report[p] for p, *_ in POSES}, indent=1))
