"""Touch up Ryu's two generated throw poses (AP6): join the ground shadow
across the gap between his feet, as his ROM poses draw it, and mirror the
heave to face where the opponent lands. Writes <scratch>/ap6/<pose>.png."""
import json, os, sys
from PIL import Image, ImageOps

S = sys.argv[1]
BLACK = (0, 0, 0, 255)
log = {}
for pose, src, mirror in [("throw_turn", "65922905-2.png", True), ("throw_heave", "8b5a8652-0.png", True)]:
    im = Image.open(os.path.join(S, "ap6", "gen", src)).convert("RGBA")
    notes = []
    if mirror:
        im = ImageOps.mirror(im)
        notes.append("mirrored left to right (the generator has no facing control)")
    p = im.load()
    W, H = im.size
    bottom = max(y for y in range(H) if any(p[x, y][3] for x in range(W)))
    filled = 0
    for y in (bottom - 1, bottom):
        xs = [x for x in range(W) if p[x, y][3]]
        for x in range(min(xs), max(xs) + 1):
            if not p[x, y][3]:
                p[x, y] = BLACK
                filled += 1
    notes.append(f"the ground shadow joined across the gap between his feet in black, rows {bottom - 1}-{bottom} ({filled} pixels), as under his ROM poses")
    im.save(os.path.join(S, "ap6", pose + ".png"))
    log[pose] = notes
json.dump(log, open(os.path.join(S, "ap6", "touch_ups.json"), "w"), indent=2)
print(json.dumps(log, indent=2))
