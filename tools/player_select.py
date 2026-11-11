"""Build the portrait character-select frame from local VS-screen captures.

The captures stay gitignored. This writes target/player-select.png only.
"""

from pathlib import Path

from PIL import Image

ROOT = Path("tools/reference/local/captures")
OUT = Path("target/player-select.png")
BG = (17, 17, 17)
W, H = 384, 224

SOURCES = {
    "RYU": ("prefight-01-run-a/pixels_002100.png", "left", False),
    "KEN": ("start-2p-01-run-a/pixels_002004.png", "right", False),
    "RETSU": ("prefight-01-run-a/pixels_002100.png", "right", False),
    "GEKI": ("win-01-run-a/pixels_005457.png", "right", False),
    "JOE": ("win-02-run-a/pixels_010440.png", "right", False),
    "MIKE": ("win-03-run-a/pixels_013320.png", "right", False),
    "LEE": ("win-04-run-a/pixels_019003.rgba", "right", True),
    "GEN": ("win-05-run-a/pixels_021868.png", "right", False),
    "BIRDIE": ("win-06-run-a/pixels_025991.png", "right", False),
    "EAGLE": ("win-07-run-a/pixels_028995.png", "right", False),
    "ADON": ("win-08-run-a/pixels_033797.png", "right", False),
    "SAGAT": ("win-09-run-a/pixels_036748.png", "right", False),
}

ORDER = list(SOURCES)

# 3x5 glyphs, rows of bits.
GLYPH = {
    "A": ["010", "101", "111", "101", "101"],
    "B": ["110", "101", "110", "101", "110"],
    "C": ["011", "100", "100", "100", "011"],
    "D": ["110", "101", "101", "101", "110"],
    "E": ["111", "100", "110", "100", "111"],
    "G": ["011", "100", "101", "101", "011"],
    "I": ["111", "010", "010", "010", "111"],
    "J": ["001", "001", "001", "101", "010"],
    "K": ["101", "110", "100", "110", "101"],
    "L": ["100", "100", "100", "100", "111"],
    "M": ["101", "111", "101", "101", "101"],
    "N": ["110", "101", "101", "101", "101"],
    "O": ["010", "101", "101", "101", "010"],
    "P": ["110", "101", "110", "100", "100"],
    "R": ["110", "101", "110", "101", "101"],
    "S": ["011", "100", "010", "001", "110"],
    "T": ["111", "010", "010", "010", "010"],
    "U": ["101", "101", "101", "101", "010"],
    "Y": ["101", "101", "010", "010", "010"],
    " ": ["000", "000", "000", "000", "000"],
}


def load_frame(rel: str, swap: bool) -> Image.Image:
    path = ROOT / rel
    if rel.endswith(".rgba"):
        data = path.read_bytes()
        im = Image.frombytes("RGBA", (W, H), data)
        if swap:
            r, g, b, a = im.split()
            im = Image.merge("RGBA", (b, g, r, a))
        return im
    return Image.open(path).convert("RGBA")


def crop_bust(im: Image.Image, side: str) -> Image.Image:
    """The bust is one connected blob. The other fighter and the nameplate stay out."""
    px = im.load()
    y0, y1 = 24, 142
    seen = [[False] * W for _ in range(y1)]
    best = None
    for y in range(y0, y1):
        for x in range(W):
            if seen[y][x] or px[x, y][:3] == BG:
                continue
            stack = [(x, y)]
            cells = []
            while stack:
                cx, cy = stack.pop()
                if cy < y0 or cy >= y1 or cx < 0 or cx >= W or seen[cy][cx]:
                    continue
                if px[cx, cy][:3] == BG:
                    continue
                seen[cy][cx] = True
                cells.append((cx, cy))
                stack.extend(((cx - 1, cy), (cx + 1, cy), (cx, cy - 1), (cx, cy + 1)))
            if len(cells) < 80:
                continue
            cx = sum(c[0] for c in cells) / len(cells)
            on_side = cx < W / 2 if side == "left" else cx > W / 2
            if on_side and (best is None or len(cells) > len(best)):
                best = cells
    if not best:
        raise RuntimeError(f"no bust on the {side}")
    xs = [c[0] for c in best]
    ys = [c[1] for c in best]
    box = (max(0, min(xs) - 1), max(0, min(ys) - 1), min(W, max(xs) + 2), min(H, max(ys) + 2))
    return im.crop(box)


def save_raw(path: Path, im: Image.Image) -> None:
    rgba = im.convert("RGBA")
    w, h = rgba.size
    path.write_bytes(w.to_bytes(2, "little") + h.to_bytes(2, "little") + rgba.tobytes())


def text(im: Image.Image, s: str, x: int, y: int, color, scale: int = 2):
    px = im.load()
    cx = x
    for ch in s:
        rows = GLYPH.get(ch, GLYPH[" "])
        for gy, row in enumerate(rows):
            for gx, bit in enumerate(row):
                if bit == "1":
                    for sy in range(scale):
                        for sx in range(scale):
                            px[cx + gx * scale + sx, y + gy * scale + sy] = color + (255,)
        cx += 4 * scale


def frame(im: Image.Image, box, color):
    x0, y0, x1, y1 = box
    px = im.load()
    for x in range(x0, x1):
        px[x, y0] = color
        px[x, y1 - 1] = color
    for y in range(y0, y1):
        px[x0, y] = color
        px[x1 - 1, y] = color
    # Chamfer the corners.
    for i in range(4):
        for p in (
            (x0 + i, y0),
            (x0, y0 + i),
            (x1 - 1 - i, y0),
            (x1 - 1, y0 + i),
            (x0 + i, y1 - 1),
            (x0, y1 - 1 - i),
            (x1 - 1 - i, y1 - 1),
            (x1 - 1, y1 - 1 - i),
        ):
            if 0 <= p[0] < W and 0 <= p[1] < H:
                px[p] = (8, 8, 12, 255)


def main():
    busts = {name: crop_bust(load_frame(rel, swap), side) for name, (rel, side, swap) in SOURCES.items()}
    raw_dir = Path("target/select-busts")
    raw_dir.mkdir(parents=True, exist_ok=True)
    for name, bust in busts.items():
        save_raw(raw_dir / f"{name.lower()}.raw", bust)
    canvas = Image.new("RGBA", (W, H), (8, 8, 12, 255))
    gold = (196, 148, 48, 255)
    blue = (72, 144, 220, 255)
    yellow = (240, 216, 32, 255)
    frame(canvas, (4, 4, 380, 220), gold)
    text(canvas, "PLAYER SELECT", 96, 12, blue[:3], 2)
    selected = "RYU"
    large = busts[selected].resize((148, 112), Image.NEAREST)
    canvas.paste(large, (18, 40))
    frame(canvas, (16, 38, 168, 154), gold)
    text(canvas, selected, 18, 164, blue[:3], 2)
    for i, name in enumerate(ORDER):
        col, row = i % 3, i // 3
        x, y = 188 + col * 62, 36 + row * 44
        tile = busts[name].resize((52, 34), Image.NEAREST)
        canvas.paste(tile, (x + 2, y + 2))
        color = yellow if name == selected else gold
        frame(canvas, (x, y, x + 56, y + 38), color)
    OUT.parent.mkdir(exist_ok=True)
    canvas.convert("RGB").save(OUT)
    print(OUT)


if __name__ == "__main__":
    main()
