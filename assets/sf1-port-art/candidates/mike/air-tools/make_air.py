"""Make Mike's air set and Iron Upper poses (AP16, #180) from his ROM poses and the kept PixelLab candidates.

usage: make_air.py <candidates/mike folder> <out folder>
Writes <out>/<pose>.png (his colours only) and prints each pose's anchor and touch-ups as JSON.
Needs rompose.py, head.py, clean.py and build.py beside it, and the ROM set (read only).
"""
import json, os, sys
import rompose as r, head as H, clean as k
from build import on_tuck
C, OUT = sys.argv[1], sys.argv[2]
os.makedirs(OUT, exist_ok=True)
report = {}
def bbox_anchor(c, wx, wy):
    """Anchor of c's opaque box when canvas pixel (0,0) sits at world (wx, wy)."""
    b = c.bbox(); return [b[0] + wx, b[1] + wy]
def save(name, c, anchor, notes):
    c.save(f"{OUT}/{name}.png"); report[name] = {"anchor": anchor, "touch_ups": notes}

# Rebuilt on the ROM tuck (0x1608, anchor [-32, -112]); the waist band inpainted.
# The tuck sits at canvas (0, 12), so canvas (x, y) is world (x - 32, y - 124).
for name, job, box in [("jump_punch", "0530702d", (10, 39, 47, 51)), ("jump_hook", "24a45a36", (10, 39, 51, 52))]:
    src = r.load(f"{C}/inpaint-source-jump-{'jp' if name == 'jump_punch' else 'jh'}-96x80.png")
    c = r.load(f"{C}/{job}-0.png")
    inside = lambda x, y, b=box: b[0] <= x < b[2] and b[1] <= y < b[3]
    assert all(c.p[y][x] == src.p[y][x] for y in range(80) for x in range(96) if not inside(x, y)), name
    changed = sum(1 for y in range(80) for x in range(96) if inside(x, y) and c.p[y][x] != src.p[y][x])
    lone = k.lone(c, inside)
    save(name, c, bbox_anchor(c, -32, -124),
         [f"outside the mask x{box[0]}-{box[2]-1}, y{box[1]}-{box[3]-1} every pixel is the ROM source's; inside, {changed} pixels inpainted, taken to his nearest pens; {lone} single pixels inside the mask set to the pen around them"])

# Generated whole: his ROM head laid over the generated one; single pixels outside the head cleaned.
# Lined up with the jump punch: the head's left edge at world x 5 (face) or the same place for the turned head,
# and the lowest pixel (his tucked feet, or his fist) at the tuck's feet, world y -48.
def pad(c, top=8):
    o = r.P(c.w, c.h + top); o.paste(c, 0, top); return o
def strip_red(h):
    for y in range(h.h):
        for x in range(h.w):
            if h.p[y][x] in (8, 9, 10): h.p[y][x] = r.T
    return h
for name, job, kind, air in [("jump_punch_down", "ce63f015-2", "back", True), ("jump_jab_down", "e20b5690-0", "face", True),
                             ("jump_hammer", "70fa07b5-0", "face", True), ("iron_rise", "0d5d8083-2", "face", False)]:
    c = pad(r.load(f"{C}/{job}.png")); h = strip_red(H.rom_head(kind)); b = c.bbox()
    _, ax, ay = H.best_offset(c, h, (b[0] - 4, b[2], b[1] - 4, b[1] + 16))
    protect = None
    if name == "iron_rise":   # his rising fist, in front of his face, keeps its own pixels
        pts = [(x, y) for y in range(ay - 2, ay + h.h + 4) for x in range(ax, ax + h.w + 12) if c.at(x, y) in (6, 7)]
        protect = (min(p[0] for p in pts) - 3, min(p[1] for p in pts) - 3, max(p[0] for p in pts) + 4, max(p[1] for p in pts) + 4)
    cleared, laid = H.lay(c, h, ax, ay, clear_box=(ax - 3, ay - 3, ax + h.w + 3, ay + h.h - 1), protect=protect)
    hb = (ax, ay, ax + h.w, ay + h.h)
    lone = k.lone(c, lambda x, y: not (hb[0] <= x < hb[2] and hb[1] <= y < hb[3]))
    isl = k.islands(c)
    src = {"face": "his jab's (0x1470, rows 9-21)", "back": "his straight's (0x14e0, rows 29-39)"}[kind]
    notes = [f"his ROM head, {src}, laid over the generated one at ({ax}, {ay}) of the candidate's canvas (padded 8 rows on top); "
             f"{cleared} generated hair and skin pixels round it cleared; {laid} pixels changed"
             + (f"; his rising fist ({protect}) keeps the generated pixels" if protect else ""),
             f"{lone} single pixels outside the head set to the pen around them" + (f", {isl} stray pixels removed" if isl else "")]
    head_x = ax - (0 if kind == "face" else 0)
    b = c.bbox()
    if air:
        wx = 5 - ax          # head's left edge at world x 5, as the jump punch's
        wy = -48 - b[3]      # lowest pixel on the tuck's lowest line
    else:
        wx = -(b[0] + b[2]) // 2
        wy = -b[3]           # standing on the ground
    save(name, c, bbox_anchor(c, wx, wy), notes)
print(json.dumps(report, indent=1))
