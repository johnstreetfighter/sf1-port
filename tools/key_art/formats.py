"""Write a key art master's two formats (ticket #234, KA1; reused for
KA2's cartoon style).

Given a 2560x1280 master and a style name, it writes:

- key-art-<style>-banner.png, 2560x1280: the master as it is;
- key-art-<style>-square.png, 1080x1080: the safe area (around Ryu, Ken and
  Sagat) cut from the master and scaled to 1080.

By default the art carries no text at all (the maintainer, 2026-11-06): the README
and the posts carry the name and the fan-made note. With --lettering it
also sets the torn-brush title "THE WORLD TOURNAMENT" and the fan-made
line, as an earlier round did; that code is kept for a later change of mind.

The title is torn brush-stroke capitals in the spirit of the 1987 arcade
lettering (the maintainer, 2026-11-06), but the letters are our own, built here
procedurally; nothing traces, redraws or samples Capcom's letterforms or any
image of their logo, and the art never says STREET FIGHTER. Each letter
starts as a glyph of a heavy condensed font, set at a slightly irregular
size, height and tilt; then:

1. the outline wobbles (a mesh warp from a seeded noise grid);
2. thin horizontal and vertical strips tear in and out of the edges, so
   stroke ends and sides turn ragged like a brush's bristles;
3. dry-brush streaks run back along the stroke from the ends of its runs;
4. the whole line leans forward (a shear);
5. it is painted: a hot orange-to-yellow flame gradient, a dark red-brown
   inner edge, a heavy black outline and a black drop shadow.

The tears come from LETTERING_SEED in layout.py, so every run letters alike.

Fonts, both from C:\\Windows\\Fonts on the maintainer's machine (Microsoft's licence;
the fonts are only rendered into the images, never committed or shipped):
Impact (impact.ttf) under the title's tears, and Franklin Gothic Book
(FRABK.TTF) for the fan-made line.

Every size and box is in layout.py. With --lettering, the lettering is drawn on its own
transparent layer and composited over the picture; the checks confirm the
sizes exactly and that the output differs from the picture only where that
layer is.

    python tools/key_art/formats.py <master.png> <style> [--out DIR]
    python tools/key_art/formats.py <master.png> <style> --lettering [--layers] [--out DIR]
"""

from __future__ import annotations

import argparse
import random
import re
import sys
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFilter, ImageFont

from layout import (
    BANNER_FAN_LINE,
    BANNER_LETTERING,
    BANNER_TITLE,
    CANVAS,
    LETTERING_SEED,
    SAFE_AREA,
    SCRIM_FADE,
    SQUARE,
    SQUARE_FAN_LINE,
    SQUARE_LETTERING,
    SQUARE_TITLE,
    inside,
    size,
)

TITLE = ("THE", "WORLD TOURNAMENT")
FAN_LINE = "Fan-made \u00b7 not affiliated with Capcom"

FONTS = Path("C:/Windows/Fonts")
TITLE_FONT = FONTS / "impact.ttf"  # Impact: the heavy condensed glyphs under the tears
FAN_FONT = FONTS / "FRABK.TTF"  # Franklin Gothic Book

# The torn title is built at this cap size, then scaled into its box.
WORK = 360

# Each letter's size, baseline and tilt vary by up to these.
LETTER_SIZE = (0.95, 1.06)
LETTER_RISE = 0.025  # of the cap size, up or down
LETTER_TILT = 2.0  # degrees
LETTER_SPACING = 0.94  # of each letter's advance: brush letters crowd

# The tears, in pixels at WORK.
WOBBLE_CELL, WOBBLE = 40, 5.0  # the outline's slow wobble
STRIP = (3, 9)  # the strips' thickness between zigzag points
BRISTLE = 5.0  # most zigzag points shift up to this
TEAR, TEAR_CHANCE = 30.0, 0.08  # a few tear in this far
FINGER, FINGER_CHANCE = 24.0, 0.05  # and a few reach out this far
DRY_STREAKS = 0.1  # wedge streaks per row of the line's height
DRY_LENGTH = (0.25, 0.7)  # of the run a streak eats back from its end
DRY_WIDTH = (4.0, 11.0)  # a streak's width at the end

# The lean: x moves this much per pixel of height (about 14 degrees).
SLANT = 0.25

# The paint, in pixels at WORK.
FLAME = [(0.0, (255, 244, 130)), (0.45, (255, 196, 36)), (1.0, (238, 92, 14))]  # top to bottom
INNER_EDGE, INNER_EDGE_COLOUR = 7, (104, 28, 14)
OUTLINE, OUTLINE_COLOUR = 15, (0, 0, 0)
SHADOW_OFFSET, SHADOW_COLOUR = (16, 20), (0, 0, 0, 230)

# The fan-made line: white, a dark outline and a soft shadow, as fractions of its size.
FAN_FILL, FAN_OUTLINE = (255, 255, 255), (18, 16, 46)
FAN_OUTLINE_WIDTH = 0.06
FAN_SHADOW, FAN_SHADOW_OFFSET = (0, 0, 0, 170), (0.05, 0.07)
SCRIM_DARK = 200  # the scrim's alpha at the foot of its area


# --- The torn title ----------------------------------------------------------


def glyphs(text: str, rng: random.Random) -> Image.Image:
    """`text` as a mask of heavy capitals, each at its own size, height and tilt."""
    base = ImageFont.truetype(str(TITLE_FONT), WORK)
    ascent = base.getbbox("H")[1]
    height = int(WORK * 1.8)
    width = int(sum(base.getlength(c) for c in text) * 1.3) + WORK
    mask = Image.new("L", (width, height))
    baseline = int(WORK * 1.35)
    x = WORK // 2
    for c in text:
        if c == " ":
            x += int(base.getlength(" ") * 0.9)
            continue
        font = ImageFont.truetype(str(TITLE_FONT), round(WORK * rng.uniform(*LETTER_SIZE)))
        letter = Image.new("L", (int(font.getlength(c)) + WORK, height))
        ImageDraw.Draw(letter).text((WORK // 2, baseline), c, font=font, fill=255, anchor="ls")
        pivot = (WORK // 2 + font.getlength(c) / 2, baseline - (WORK - ascent) / 2)
        letter = letter.rotate(rng.uniform(-LETTER_TILT, LETTER_TILT), Image.BICUBIC, center=pivot)
        rise = round(WORK * rng.uniform(-LETTER_RISE, LETTER_RISE))
        mask.paste(ImageChops.lighter(mask.crop((x - WORK // 2, rise, x - WORK // 2 + letter.width, rise + height)), letter), (x - WORK // 2, rise))
        x += int(font.getlength(c) * LETTER_SPACING)
    l, t, r, b = mask.getbbox()
    pad = WORK // 4
    return mask.crop((l - pad, t - pad, r + pad, b + pad))


def wobble(mask: Image.Image, rng: random.Random) -> Image.Image:
    """The outline bent by a mesh warp over a grid of random corner offsets."""
    cols, rows = mask.width // WOBBLE_CELL + 2, mask.height // WOBBLE_CELL + 2
    off = [[(rng.uniform(-WOBBLE, WOBBLE), rng.uniform(-WOBBLE, WOBBLE)) for _ in range(rows)] for _ in range(cols)]
    mesh = []
    for i in range(cols - 1):
        for j in range(rows - 1):
            x0, y0, x1, y1 = i * WOBBLE_CELL, j * WOBBLE_CELL, (i + 1) * WOBBLE_CELL, (j + 1) * WOBBLE_CELL
            corner = lambda a, b, x, y: (x + off[a][b][0], y + off[a][b][1])
            quad = (*corner(i, j, x0, y0), *corner(i, j + 1, x0, y1), *corner(i + 1, j + 1, x1, y1), *corner(i + 1, j, x1, y0))
            mesh.append(((x0, y0, x1, y1), quad))
    return mask.transform(mask.size, Image.MESH, mesh, Image.BILINEAR)


def strips(mask: Image.Image, rng: random.Random, across: bool, reach: float, chance: float, spike: float) -> Image.Image:
    """`mask` warped along thin strips (rows when `across`, else columns):
    each boundary between strips shifts by its own random amount, mostly up
    to `reach`, now and then (`chance`) by up to `spike`, and the strips
    between slant from one to the next. The edges the strips cross turn
    into jagged zigzags with sharp tears."""
    length = mask.width if across else mask.height
    span = mask.height if across else mask.width
    cuts = [0]
    while cuts[-1] < span:
        cuts.append(min(cuts[-1] + rng.randint(*STRIP), span))

    def shift() -> float:
        if rng.random() < chance:
            return rng.choice((-1, 1)) * rng.uniform(spike / 3, spike)
        return rng.uniform(-reach, reach)

    shifts = [shift() for _ in cuts]
    mesh = []
    for a, b, sa, sb in zip(cuts, cuts[1:], shifts, shifts[1:]):
        if across:
            mesh.append(((0, a, length, b), (sa, a, sb, b, length + sb, b, length + sa, a)))
        else:
            mesh.append(((a, 0, b, length), (a, sa, a, length + sa, b, length + sb, b, sb)))
    return mask.transform(mask.size, Image.MESH, mesh, Image.BILINEAR)


def tear(mask: Image.Image, rng: random.Random) -> Image.Image:
    """Torn edges: two independent zigzag warps cut in (the darker of them)
    and a third, rarer one reaches out (the lighter), across and then down,
    so each edge tears on its own."""
    for across, scale in ((True, 1.0), (False, 0.7)):
        warp = lambda: strips(mask, rng, across, BRISTLE * scale, TEAR_CHANCE, TEAR * scale)
        cut = ImageChops.darker(warp(), warp())
        reach = strips(mask, rng, across, 0.0, FINGER_CHANCE, FINGER * scale)
        mask = ImageChops.lighter(cut, ImageChops.darker(reach, grow(mask, int(FINGER * scale))))
    return mask


def dry_brush(mask: Image.Image, rng: random.Random) -> Image.Image:
    """Wedge-shaped streaks eaten back along the stroke from the end of a
    run of ink, widest at the end and tapering to nothing, as a brush
    running dry leaves them."""
    draw = ImageDraw.Draw(mask)
    rows = mask.height
    for _ in range(int(DRY_STREAKS * rows)):
        y = rng.randrange(rows)
        line = mask.crop((0, y, mask.width, y + 1)).tobytes()
        runs, start = [], None
        for x, v in enumerate(line):
            if v > 127 and start is None:
                start = x
            elif v <= 127 and start is not None:
                runs.append((start, x))
                start = None
        runs = [r for r in runs if r[1] - r[0] > 40]
        if not runs:
            continue
        a, b = rng.choice(runs)
        length = int((b - a) * rng.uniform(*DRY_LENGTH))
        half = rng.uniform(*DRY_WIDTH) / 2
        if rng.random() < 0.7:  # most strokes run dry at their right end
            draw.polygon([(b + 2, y - half), (b + 2, y + half), (b - length, y)], fill=0)
        else:
            draw.polygon([(a - 2, y - half), (a - 2, y + half), (a + length, y)], fill=0)
    return mask


def lean(image: Image.Image) -> Image.Image:
    """`image` widened and slanted forward about its bottom row."""
    wide = Image.new(image.mode, (image.width + int(SLANT * image.height) + 1, image.height))
    wide.paste(image, (0, 0))
    return wide.transform(wide.size, Image.AFFINE, (1, SLANT, -SLANT * image.height, 0, 1, 0), Image.BICUBIC)


def grow(mask: Image.Image, radius: int) -> Image.Image:
    """`mask` spread outward by about `radius` pixels, with soft edges."""
    return mask.filter(ImageFilter.GaussianBlur(radius / 2)).point(lambda v: min(255, v * 8))


def shrink(mask: Image.Image, radius: int) -> Image.Image:
    """`mask` pulled inward by about `radius` pixels, with soft edges."""
    return mask.filter(ImageFilter.GaussianBlur(radius / 2)).point(lambda v: max(0, (v - 224) * 8))


def flame(width: int, height: int, top: int, bottom: int) -> Image.Image:
    """The flame gradient running from row `top` to row `bottom`."""
    column = Image.new("RGB", (1, height))
    for y in range(height):
        t = min(max((y - top) / max(bottom - top, 1), 0.0), 1.0)
        for (t0, c0), (t1, c1) in zip(FLAME, FLAME[1:]):
            if t0 <= t <= t1:
                k = (t - t0) / (t1 - t0)
                column.putpixel((0, y), tuple(round(a + (b - a) * k) for a, b in zip(c0, c1)))
                break
    return column.resize((width, height))


def torn(text: str, rng: random.Random) -> Image.Image:
    """`text` as torn, painted brush capitals on transparent, at WORK size."""
    mask = glyphs(text, rng)
    mask = wobble(mask, rng)
    mask = tear(mask, rng)
    mask = dry_brush(mask, rng)
    pad = OUTLINE + max(SHADOW_OFFSET) + 4
    mask = lean(mask.crop((-pad, -pad, mask.width + pad, mask.height + pad)))
    outline = grow(mask, OUTLINE)
    edge = ImageChops.subtract(mask, shrink(mask, INNER_EDGE))
    ink = mask.getbbox()
    out = Image.new("RGBA", mask.size)
    shadow = Image.new("RGBA", mask.size, SHADOW_COLOUR)
    shadow.putalpha(ImageChops.multiply(outline, Image.new("L", mask.size, SHADOW_COLOUR[3])))
    out.alpha_composite(shadow, SHADOW_OFFSET, (0, 0, mask.width - SHADOW_OFFSET[0], mask.height - SHADOW_OFFSET[1]))
    for colour, alpha in ((OUTLINE_COLOUR, outline), (None, mask), (INNER_EDGE_COLOUR, edge)):
        paint = flame(*mask.size, ink[1], ink[3]).convert("RGBA") if colour is None else Image.new("RGBA", mask.size, colour + (255,))
        paint.putalpha(alpha)
        out.alpha_composite(paint)
    return out.crop(out.getchannel("A").getbbox())


def set_title(layer: Image.Image, text: str, box, rng: random.Random) -> None:
    """The torn `text`, scaled to fit `box` and centred in it, onto `layer`."""
    art = torn(text, rng)
    width, height = size(box)
    k = min(width / art.width, height / art.height)
    art = art.resize((max(1, round(art.width * k)), max(1, round(art.height * k))), Image.LANCZOS)
    layer.alpha_composite(art, (box[0] + (width - art.width) // 2, box[1] + (height - art.height) // 2))


# --- The fan-made line -------------------------------------------------------


def set_fan_line(layer: Image.Image, box) -> None:
    """The fan-made line, as large as fits `box` and centred in it."""
    width, height = size(box)
    lo, hi = 4, 400
    while lo < hi:
        mid = (lo + hi + 1) // 2
        font = ImageFont.truetype(str(FAN_FONT), mid)
        l, t, r, b = font.getbbox(FAN_LINE, stroke_width=round(mid * FAN_OUTLINE_WIDTH))
        if r - l + mid * FAN_SHADOW_OFFSET[0] <= width and b - t + mid * FAN_SHADOW_OFFSET[1] <= height:
            lo = mid
        else:
            hi = mid - 1
    font = ImageFont.truetype(str(FAN_FONT), lo)
    stroke = round(lo * FAN_OUTLINE_WIDTH)
    l, t, r, b = font.getbbox(FAN_LINE, stroke_width=stroke)
    origin = (box[0] + (width - (r - l)) // 2 - l, box[1] + (height - (b - t)) // 2 - t)
    dx, dy = (round(lo * k) for k in FAN_SHADOW_OFFSET)
    draw = ImageDraw.Draw(shadow := Image.new("RGBA", layer.size))
    draw.text((origin[0] + dx, origin[1] + dy), FAN_LINE, font=font, fill=FAN_SHADOW, stroke_width=stroke, stroke_fill=FAN_SHADOW)
    layer.alpha_composite(shadow)
    ImageDraw.Draw(layer).text(origin, FAN_LINE, font=font, fill=FAN_FILL, stroke_width=stroke, stroke_fill=FAN_OUTLINE)


# --- The formats -------------------------------------------------------------


def scrim(layer: Image.Image, area) -> None:
    """A dark fade behind the lettering: clear at the top of `area`, SCRIM_DARK
    from SCRIM_FADE pixels down."""
    draw = ImageDraw.Draw(layer)
    for y in range(area[1], area[3]):
        alpha = round(SCRIM_DARK * min(1.0, (y - area[1]) / SCRIM_FADE))
        draw.line([(area[0], y), (area[2] - 1, y)], fill=(8, 8, 20, alpha))


def title_lines(boxes) -> list[str]:
    """One box: the title on one line; two: THE over WORLD TOURNAMENT."""
    if len(boxes) == 1:
        return [" ".join(TITLE)]
    if len(boxes) == 2:
        return list(TITLE)
    sys.exit(f"the title takes one or two boxes, not {len(boxes)}")


def lettering(canvas: tuple[int, int], area, title_boxes, fan_box, seed: int) -> Image.Image:
    """The text layer: the scrim, the torn title and the fan-made line, on transparent."""
    layer = Image.new("RGBA", canvas, (0, 0, 0, 0))
    scrim(layer, area)
    rng = random.Random(seed)
    for text, box in zip(title_lines(title_boxes), title_boxes):
        if not inside(box, area):
            sys.exit(f"the line {text!r} is set outside its area: {box} not in {area}")
        set_title(layer, text, box, rng)
    if not inside(fan_box, area):
        sys.exit(f"the fan-made line is set outside its area: {fan_box} not in {area}")
    set_fan_line(layer, fan_box)
    return layer


def letter(picture: Image.Image, layer: Image.Image, what: str) -> Image.Image:
    """`layer` over `picture`, checked: the same size, and changed only where the layer is."""
    if picture.size != layer.size:
        sys.exit(f"{what}: the picture is {picture.size}, the lettering {layer.size}")
    out = picture.copy()
    out.alpha_composite(layer)
    drawn = layer.getchannel("A").getbbox()
    changed = ImageChops.difference(out.convert("RGB"), picture.convert("RGB")).getbbox()
    if drawn is None:
        sys.exit(f"{what}: the text layer is empty")
    if changed is not None and not inside(changed, drawn):
        sys.exit(f"{what}: the output changed outside the text layer: {changed} not in {drawn}")
    return out


def check_size(image: Image.Image, want: tuple[int, int], what: str) -> None:
    if image.size != want:
        sys.exit(f"{what} is {image.width}x{image.height}, not {want[0]}x{want[1]}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("master", type=Path, help="the 2560x1280 master PNG")
    parser.add_argument("style", help="the style's name, e.g. illustrated or cartoon")
    parser.add_argument("--out", type=Path, default=Path("tools/reference/local/key-art"))
    parser.add_argument("--lettering", action="store_true", help="also set the torn-brush title and the fan-made line")
    parser.add_argument("--layers", action="store_true", help="with --lettering, also write each format's text layer alone")
    args = parser.parse_args()
    if not re.fullmatch(r"[a-z][a-z0-9-]*", args.style):
        sys.exit(f"a style name is lower-case letters, digits and dashes: {args.style!r}")
    if args.layers and not args.lettering:
        sys.exit("--layers needs --lettering")
    if args.lettering:
        for font in (TITLE_FONT, FAN_FONT):
            if not font.is_file():
                sys.exit(f"the font {font} is missing")
        if not inside(BANNER_LETTERING, (0, 0, *CANVAS)) or not inside(SQUARE_LETTERING, (0, 0, *SQUARE)):
            sys.exit("a lettering area is outside its format")
    if size(SAFE_AREA)[0] != size(SAFE_AREA)[1] or not inside(SAFE_AREA, (0, 0, *CANVAS)):
        sys.exit(f"the safe area {SAFE_AREA} is not a square inside the master")

    master = Image.open(args.master).convert("RGBA")
    check_size(master, CANVAS, "the master")

    # The square: exactly the safe area, scaled to the square's size.
    cut = master.crop(SAFE_AREA)
    check_size(cut, size(SAFE_AREA), "the safe area's cut")
    picture = cut.resize(SQUARE, Image.LANCZOS)

    banner, square, layers = master, picture, {}
    if args.lettering:
        banner_layer = lettering(CANVAS, BANNER_LETTERING, BANNER_TITLE, BANNER_FAN_LINE, LETTERING_SEED)
        banner = letter(master, banner_layer, "the banner")
        square_layer = lettering(SQUARE, SQUARE_LETTERING, SQUARE_TITLE, SQUARE_FAN_LINE, LETTERING_SEED + 1)
        square = letter(picture, square_layer, "the square")
        if args.layers:
            layers = {"banner": banner_layer, "square": square_layer}
    check_size(banner, CANVAS, "the banner")
    check_size(square, SQUARE, "the square")

    args.out.mkdir(parents=True, exist_ok=True)
    written = {
        f"key-art-{args.style}-banner.png": banner.convert("RGB"),
        f"key-art-{args.style}-square.png": square.convert("RGB"),
    }
    for form, layer in layers.items():
        written[f"key-art-{args.style}-{form}-lettering.png"] = layer
    for name, image in written.items():
        path = args.out / name
        image.save(path)
        check_size(Image.open(path), image.size, name)
        print(f"wrote {path} ({image.width}x{image.height})")
    print(f"the square is the safe area {SAFE_AREA}, scaled from {size(SAFE_AREA)[0]} to {SQUARE[0]}")
    if not args.lettering:
        print("no text on the art: the README and the posts carry the name and the fan-made note")


if __name__ == "__main__":
    main()
