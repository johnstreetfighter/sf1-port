"""Ken's shoulder-throw turn and heave (AP6), made from Ryu's approved ones.

Ken's pens line up with Ryu's (gi greys where Ken's reds are, red hair and
wristbands where Ken's golds are), so his pens drawn in Ken's colours are
most of the way to Ken. What Ken's own ROM art does differently is fixed by
hand, measured against his ROM stance (0x0863), grab (0x0c2a) and his back
seen in the Tatsumaki (0x0da0):

- no headband: its pixels become hair;
- his hair is outlined and streaked in brown (pen 1), not black and dark
  red; black stays only where it touches skin (his eyes and brows);
- the generator's few white points in the face become pen 5, his lightest
  skin (pen 7 is his gi red);
- his hair covers his ears, and is mostly his light gold (pen 10), as in
  his ROM heads, where Ryu's is mostly mid red;
- his hands are bare: his gold bands sit on his wrists only;
- he is barefoot: his flat feet are his ROM grab's front foot, mirrored;
  the turn's rear heel, seen from behind, is shaded in his skin colours;
  the heave's rear foot faces us, so it is his ROM stance's rear foot,
  toes toward us, mirrored.

The first round (user, 2026-11-03) was rejected for his hands, hair and
feet, the second for the heave's rear foot, drawn as a heel where it faces
us; this is the third.

Usage: ken_throw.py <pack folder> <grab reference> <stance reference> <out folder>
The references are `sf-art reference ken 0x0c2a large 2 2 <png>` and
`sf-art reference ken 0x0863 large 2 1 <png>`.
Writes <out>/<pose>.png (RGBA, Ken's colours, for `sf-art pack ken`), a
before/after sheet and touch_ups.json.
"""
import json
import os
import sys

from PIL import Image

KEN = ["#000000", "#664400", "#996655", "#cc9977", "#ddaa88", "#ddccaa", "#000000", "#dd0000", "#775500", "#997700", "#cc9900", "#440000", "#660000", "#880000", "#aa0000"]
RYU = ["#000000", "#664400", "#996655", "#cc9977", "#ddaa88", "#ddccaa", "#00ddff", "#cccccc", "#550000", "#880000", "#bb0000", "#444444", "#666666", "#888888", "#aaaaaa"]
CLEAR = 15
HAIR = (8, 9, 10)
SKIN = (1, 2, 3, 4, 5)
NEAR = ((1, 0), (-1, 0), (0, 1), (0, -1))

# ROM feet, each drawn mirrored (the throw faces left): its box in the
# reference, the pens taken, and what it is.
# - His grab's front foot, pointing right, toes at x56-61: rows 81-88,
#   x46-61, skin only (the black beside it is the ground shadow).
# - His stance's rear foot, facing us, toes at rows 89-90: rows 86-91, x0-9,
#   skin and the black between his toes.
GRAB_FOOT = ("grab", (46, 81, 61, 88), (1, 2, 3, 4, 5),
             "his flat foot redrawn as his ROM grab's front foot (0x0c2a, rows 81-88), mirrored, toes to the left")
STANCE_FOOT = ("stance", (0, 86, 9, 91), (0, 1, 2, 3, 4, 5),
               "his rear foot, which faces us, redrawn as his ROM stance's rear foot (0x0863, rows 86-91), mirrored, his toes toward us")

POSES = {
    "throw_turn": {
        "head": lambda x, y: 14 <= x < 40 and y < 12,
        "feet": lambda x, y: y >= 78,
        "what": "the headband across the back of his head painted out as hair, and its knot by the ear as hair, ear and cheek",
        "set": {(29, 6): 9, (30, 6): 9, (31, 6): 9, (32, 6): 9, (33, 6): 9, (34, 6): 1,
                (25, 7): 10, (26, 7): 10, (27, 7): 10, (28, 7): 9, (29, 7): 9,
                (24, 8): 1, (25, 8): 9, (25, 9): 2, (24, 10): 5},
        # His hair over his ear and the eye the turn showed at its edge; a
        # skin point left in his hair.
        "hair": {(21, 8): 1, (22, 8): 10, (23, 8): 9, (30, 7): 10,
                 (25, 9): 9, (26, 9): 10, (27, 9): 9, (28, 9): 10,
                 (25, 10): 1, (26, 10): 9, (27, 10): 1, (28, 10): 9, (24, 9): 1, (23, 10): 1},
        # The raised fist: the gold over the back of his hand, rows 14-17,
        # made skin; the band below it, rows 18-25, is his wrist.
        "hand": lambda x, y: x <= 6 and 14 <= y <= 17,
        "arm_outline": [(1, 19), (2, 20), (3, 26)],
        # His flat foot, toes to the left: the grab's foot onto rows 81-88
        # (his ankle row 80 kept), toes at x12.
        "paste": [(GRAB_FOOT, 12, 81)],
        # His rear heel, seen from behind: rows 83-86 (Ryu's dark sole).
        "paint": {83: (56, "1233443221"), 84: (56, "12344433221"), 85: (57, "1233433221"), 86: (57, "1223332221")},
    },
    "throw_heave": {
        "head": lambda x, y: 14 <= x < 32 and y < 16,
        "feet": lambda x, y: (y >= 70 and x < 45) or (x >= 70 and y >= 62),
        "what": "the headband across his bowed forehead and its tail painted out as hair, the gap between the tail and his hair filled, and a white point under his eyes made his lightest skin",
        "set": {(29, 4): 9, (28, 5): 9, (25, 7): 10, (22, 8): 10, (28, 8): 10,
                (20, 8): 9, (20, 9): 9, (21, 9): 9, (21, 10): 9,
                (23, 11): 5},
        # His hair over his ear, outlined below it.
        "hair": {(28, 9): 10, (29, 9): 9, (28, 10): 9, (29, 10): 9, (29, 11): 1},
        "hand": None,
        # His flat foot: the grab's foot onto rows 72-79 (his ankle row 71
        # kept), toes at x24. His rear foot, facing us: the stance's onto
        # rows 74-79 under his shin, its sole on the ground row.
        "paste": [(GRAB_FOOT, 24, 72), (STANCE_FOOT, 78, 74)],
        "paint": {},
    },
}


def rgb(h):
    return tuple(int(h[i:i + 2], 16) for i in (1, 3, 5))


def pens_of(path, pal):
    im = Image.open(path).convert("RGBA")
    cols = [rgb(h) for h in pal]
    W, H = im.size
    out = [[CLEAR] * W for _ in range(H)]
    for y in range(H):
        for x in range(W):
            r, g, b, a = im.getpixel((x, y))
            if a >= 128:
                out[y][x] = min(range(15), key=lambda i: sum((c - d) ** 2 for c, d in zip(cols[i], (r, g, b))))
    return out


def main():
    pack, grab_ref, stance_ref, out = sys.argv[1:5]
    os.makedirs(out, exist_ok=True)
    refs = {"grab": pens_of(grab_ref, KEN), "stance": pens_of(stance_ref, KEN)}
    log = {}
    sheets = []
    for pose, rule in POSES.items():
        im = Image.open(os.path.join(pack, "ryu", pose + ".png"))
        W, H = im.size
        src = [[im.getpixel((x, y)) for x in range(W)] for y in range(H)]
        g = [row[:] for row in src]

        def at(grid, x, y):
            return grid[y][x] if 0 <= x < W and 0 <= y < H else CLEAR

        n = {"band": 0, "streak": 0, "outline": 0, "feet": 0, "hair": 0, "light": 0, "hand": 0, "heel": 0, "shadow": 0}
        pasted = []
        # Round one: the recolour's fixes.
        for (x, y), v in rule["set"].items():
            assert src[y][x] in (0, 5, 7, 8, 9, 10, 11, 12, 13, 14, CLEAR), (pose, x, y, src[y][x])
            g[y][x] = v
            n["band"] += 1
        painted = [row[:] for row in g]
        for (x, y), v in rule["hair"].items():
            painted[y][x] = v
        for y in range(H):
            for x in range(W):
                if (x, y) in rule["set"]:
                    continue
                v = src[y][x]
                if rule["head"](x, y):
                    if v == 8:
                        g[y][x] = 1
                        n["streak"] += 1
                    elif v == 0:
                        # Black stays only beside skin (eyes, brows, the
                        # face's lines), as in his ROM face; judged after
                        # the headband and his ear became hair.
                        if not any(2 <= at(painted, x + dx, y + dy) <= 5 for dx, dy in NEAR):
                            g[y][x] = 1
                            n["outline"] += 1
                elif rule["feet"](x, y) and v in HAIR:
                    if v == 8:
                        g[y][x] = 1
                    elif v == 9:
                        g[y][x] = 2
                    else:
                        inner = sum(at(src, x + dx, y + dy) == 10 for dx, dy in NEAR)
                        g[y][x] = 4 if inner >= 3 else 3
                    n["feet"] += 1

        # Round two (user, 2026-11-03: hands, hair, feet).
        for (x, y), v in rule["hair"].items():
            assert g[y][x] in (0, 1, 2, 3, 4, 5, 9, 10), (pose, x, y, g[y][x])
            g[y][x] = v
            n["hair"] += 1
        # His hair mostly his light gold: a mid pixel away from any line.
        before = [row[:] for row in g]
        for y in range(H):
            for x in range(W):
                if rule["head"](x, y) and before[y][x] == 9 and not any(at(before, x + dx, y + dy) in (0, 1) for dx, dy in NEAR):
                    g[y][x] = 10
                    n["light"] += 1
        if rule["hand"]:
            for y in range(H):
                for x in range(W):
                    if rule["hand"](x, y) and g[y][x] in HAIR:
                        g[y][x] = {8: 1, 9: 3, 10: 4}[g[y][x]]
                        n["hand"] += 1
            # His arm's outline brown, not black, as his ROM arms are.
            for x, y in rule["arm_outline"]:
                assert g[y][x] == 0, (pose, x, y, g[y][x])
                g[y][x] = 1
                n["hand"] += 1
        for (ref, (bx0, by0, bx1, by1), take, what), tx, ty in rule["paste"]:
            # Clear his foot below the kept ankle, then draw the ROM foot
            # mirrored under his trouser hem.
            for y in range(ty, ty + (by1 - by0) + 1):
                for x in range(tx - 1, min(W, tx + (bx1 - bx0) + 2)):
                    if g[y][x] in SKIN:
                        g[y][x] = CLEAR
            count = 0
            for sy in range(by0, by1 + 1):
                for sx in range(bx0, bx1 + 1):
                    v = refs[ref][sy][sx]
                    if v in take:
                        x, y = tx + (bx1 - sx), ty + (sy - by0)
                        if g[y][x] in (CLEAR, 0) + SKIN:
                            g[y][x] = v
                            count += 1
            pasted.append(f"{what} ({count} pixels)")
        for y, (x0, row) in rule["paint"].items():
            for i, c in enumerate(row):
                g[y][x0 + i] = int(c, 16)
                n["heel"] += 1
        # The ground shadow joined again across his feet, as under his ROM
        # poses (the pasted feet are a pixel or two shorter than Ryu's).
        bottom = max(y for y in range(H) if any(v != CLEAR for v in g[y]))
        for y in (bottom - 1, bottom):
            xs = [x for x in range(W) if g[y][x] != CLEAR]
            for x in range(min(xs), max(xs) + 1):
                if g[y][x] == CLEAR:
                    g[y][x] = 0
                    n["shadow"] += 1

        log[pose] = [
            f"drawn from Ryu's approved {pose} ({pose}.png in the pack's ryu folder), pen for pen in Ken's colours",
            f"{rule['what']} ({n['band']} pixels)",
            f"his hair's dark streaks brown, pen 8 to 1, as in his ROM art ({n['streak']} pixels)",
            f"his hair's outline and strand lines brown, pen 0 to 1 wherever the black touches no skin, as in his ROM art ({n['outline']} pixels)",
            f"barefoot: Ryu's red shoes made skin, pens 8, 9, 10 to 1, 2, 3 or 4 ({n['feet']} pixels)",
            f"his hair over his ear, as his ROM hair falls ({n['hair']} pixels)",
            f"his hair lightened to his ROM heads' mix: a mid gold pixel touching no line made light gold, pen 9 to 10 ({n['light']} pixels)",
        ]
        if n["hand"]:
            log[pose].append(f"his raised hand bare: the gold over the back of it made skin, pens 8, 9, 10 to 1, 3, 4, his band left on the wrist, and the arm's black outline brown ({n['hand']} pixels)")
        log[pose] += pasted
        if n["heel"]:
            log[pose].append(f"his rear heel, seen from behind, shaded in his skin colours where Ryu's dark sole was ({n['heel']} pixels)")
        log[pose].append(f"the ground shadow joined across his feet again ({n['shadow']} pixels)")
        rgba = Image.new("RGBA", (W, H), (0, 0, 0, 0))
        for y in range(H):
            for x in range(W):
                if g[y][x] != CLEAR:
                    rgba.putpixel((x, y), rgb(KEN[g[y][x]]) + (255,))
        rgba.save(os.path.join(out, pose + ".png"))
        sheets.append((src, g, W, H))
    json.dump(log, open(os.path.join(out, "touch_ups.json"), "w"), indent=2)
    print(json.dumps(log, indent=2))

    s = 4
    width = sum(3 * (W * s + 8) + 24 for _, _, W, H in sheets)
    height = max(H for _, _, _, H in sheets) * s
    sheet = Image.new("RGB", (width, height), (40, 40, 60))
    ox = 0
    for src, g, W, H in sheets:
        for k, (grid, pal) in enumerate([(src, RYU), (src, KEN), (g, KEN)]):
            for y in range(H):
                for x in range(W):
                    if grid[y][x] != CLEAR:
                        c = rgb(pal[grid[y][x]])
                        for i in range(s):
                            for j in range(s):
                                sheet.putpixel((ox + k * (W * s + 8) + x * s + i, y * s + j), c)
        ox += 3 * (W * s + 8) + 24
    sheet.save(os.path.join(out, "before-after.png"))


main()
