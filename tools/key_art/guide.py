"""The key art's sprite guide (ticket #234, KA1): the twelve tournament
fighters' 1987 sprites, enlarged nearest-neighbour and placed in the agreed
arrangement before their own stages' backdrops, at the master's 2560x1280.
The image model paints over it; it is a guide to pose, likeness and colour,
never a published picture.

- Sagat is largest, behind the centre, before his Thai temple.
- Ryu (Hadoken) and Ken (Shoryuken) stand front and centre, back to back.
- Adon, Sagat's countryman, stands at Sagat's side, inside the safe area.
- The other eight stand in country pairs either side, each on its own
  stage's floor, facing the centre: the far pairs (Japan, China) higher and
  smaller, the near pairs (USA, England) lower and larger.

The captures come from `cargo run -p sf-host --release --example
key_art_guide`. Everything read and written here is ROM-derived and stays
in tools/reference/local/key-art/, which is never committed.

    python tools/key_art/guide.py [--captures DIR] [--out DIR]

Writes guide.png and guide-debug.png (the safe area and the title band
marked) to the out folder.
"""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

from layout import BACK_GROUND, CANVAS, FRONT_GROUND, SAFE_AREA, TITLE_BAND, inside

LOCAL = Path("tools/reference/local/key-art")

# The row the fighters' feet stand on in a 384x224 fight frame.
FLOOR = 216

# Below the stages' floors: a plain dark field for the lettering.
FIELD = (14, 14, 30, 255)


@dataclass(frozen=True)
class Backdrop:
    """A stage's backdrop filling `box`, enlarged `scale` times, its floor on
    `ground` and centred across the box. Rows above the stage repeat its
    top row (sky); below its floor is the dark lettering field."""

    stage: str
    box: tuple[int, int, int, int]
    scale: int
    ground: int


@dataclass(frozen=True)
class Placement:
    """A capture enlarged `scale` times, centred on `x`, its frame's floor
    on `ground` (an airborne figure keeps its lift), facing `faces`."""

    capture: str
    scale: int
    x: int
    ground: int
    faces: str  # "left" or "right"


# Each pair's own 1987 stage. Mike's Mount Rushmore is left out on purpose:
# Rushmore with Retsu's pagoda echoes the export flyer's composition (the
# research report's do-not-copy list). Lee's Great Wall and Eagle's castle
# are landmarks the flyers did not use.
BACKDROPS = [
    Backdrop("sagat", (640, 0, 1920, 1280), 6, FRONT_GROUND),  # Thailand, behind Sagat and Adon
    Backdrop("retsu", (0, 0, 640, 640), 3, BACK_GROUND),  # Japan, far left
    Backdrop("joe", (0, 640, 640, 1280), 2, FRONT_GROUND),  # USA, near left
    Backdrop("lee", (1920, 0, 2560, 640), 3, BACK_GROUND),  # China, far right
    Backdrop("eagle", (1920, 640, 2560, 1280), 2, FRONT_GROUND),  # England, near right
]

# Drawn in order, back to front.
PLACEMENTS = [
    Placement("sagat-stance", 8, 1280, 1000, "left"),
    Placement("retsu-stance", 3, 213, BACK_GROUND, "right"),
    Placement("geki-stance", 3, 437, BACK_GROUND, "right"),
    Placement("lee-stance", 3, 2123, BACK_GROUND, "left"),
    Placement("gen-stance", 3, 2347, BACK_GROUND, "left"),
    Placement("adon-stance", 5, 1835, 1030, "left"),
    Placement("joe-stance", 4, 200, FRONT_GROUND, "right"),
    Placement("mike-stance", 4, 440, FRONT_GROUND, "right"),
    Placement("birdie-stance", 4, 2120, FRONT_GROUND, "left"),
    Placement("eagle-stance", 4, 2360, FRONT_GROUND, "left"),
    Placement("ken-shoryuken", 6, 1660, FRONT_GROUND, "right"),
    Placement("ryu-hadoken", 6, 1095, FRONT_GROUND, "left"),
]

# The fireball travels with Ryu: drawn at his scale, at its own distance
# from him in the release frame.
FIREBALL = ("hadoken-fireball", "ryu-hadoken")

# Every capture faces right as written (P1) except the P2 sides of the pair fights.
CAPTURED_FACING_LEFT = {"geki-stance", "mike-stance", "gen-stance", "eagle-stance", "sagat-stance", "ken-stance"}

# The three the square must hold: Ryu, Ken and Sagat.
CORE = ("ryu-hadoken", "ken-shoryuken", "sagat-stance", "hadoken-fireball")


def nearest(image: Image.Image, scale: int) -> Image.Image:
    return image.resize((image.width * scale, image.height * scale), Image.NEAREST)


def backdrop(captures: Path, b: Backdrop) -> Image.Image:
    """The stage enlarged and cut to its box, its floor on the box's ground."""
    stage = nearest(Image.open(captures / f"stage-{b.stage}.png").convert("RGBA"), b.scale)
    width, height = b.box[2] - b.box[0], b.box[3] - b.box[1]
    top = (b.ground - FLOOR * b.scale) - b.box[1]
    left = (width - stage.width) // 2
    panel = Image.new("RGBA", (width, height))
    panel.alpha_composite(stage, (max(left, 0), max(top, 0)), (max(-left, 0), max(-top, 0)))
    if top > 0:  # the sky's top row, repeated up
        row = stage.crop((max(-left, 0), 0, max(-left, 0) + width, 1)).resize((width, top), Image.NEAREST)
        panel.alpha_composite(row, (0, 0))
    bottom = top + stage.height
    if bottom < height:
        panel.paste(FIELD, (0, bottom, width, height))
    return panel


def figure(captures: Path, frames: dict, p: Placement) -> tuple[Image.Image, tuple[int, int, int, int]]:
    """The capture enlarged and turned to face `p.faces`, and its box on the canvas."""
    image = nearest(Image.open(captures / f"{p.capture}.png").convert("RGBA"), p.scale)
    facing = "left" if p.capture in CAPTURED_FACING_LEFT else "right"
    flip = facing != p.faces
    if flip:
        image = image.transpose(Image.FLIP_LEFT_RIGHT)
    f = frames[p.capture]
    lift = (FLOOR - (f["y"] + f["height"])) * p.scale
    left = p.x - image.width // 2
    top = p.ground - lift - image.height
    return image, (left, top, left + image.width, top + image.height)


def fireball(captures: Path, frames: dict, ryu: Placement, ryu_box: tuple[int, int, int, int]):
    """The fireball at its release-frame distance from Ryu, mirrored with him."""
    ball, owner = (frames[name] for name in FIREBALL)
    image = nearest(Image.open(captures / f"{FIREBALL[0]}.png").convert("RGBA"), ryu.scale)
    dx = (ball["x"] - owner["x"]) * ryu.scale  # from Ryu's left edge, facing right
    dy = (ball["y"] - owner["y"]) * ryu.scale
    if ryu.faces == "left":
        image = image.transpose(Image.FLIP_LEFT_RIGHT)
        left = ryu_box[2] - dx - image.width
    else:
        left = ryu_box[0] + dx
    top = ryu_box[1] + dy
    return image, (left, top, left + image.width, top + image.height)


def build(captures: Path) -> tuple[Image.Image, dict[str, tuple[int, int, int, int]]]:
    frames = json.loads((captures / "figures.json").read_text())
    guide = Image.new("RGBA", CANVAS, (0, 0, 0, 255))
    for b in BACKDROPS:
        guide.alpha_composite(backdrop(captures, b), b.box[:2])
    boxes = {}
    for p in PLACEMENTS:
        image, box = figure(captures, frames, p)
        guide.alpha_composite(image, box[:2])
        boxes[p.capture] = box
        if p.capture == FIREBALL[1]:
            ball, ball_box = fireball(captures, frames, p, box)
            guide.alpha_composite(ball, ball_box[:2])
            boxes[FIREBALL[0]] = ball_box
    return guide, boxes


def check(boxes: dict[str, tuple[int, int, int, int]]) -> None:
    whole = (0, 0, *CANVAS)
    for name, box in boxes.items():
        if not inside(box, whole):
            sys.exit(f"{name} spills off the canvas: {box}")
        if box[3] > TITLE_BAND[1]:
            sys.exit(f"{name}'s feet reach into the title band: {box}")
    for name in CORE:
        if not inside(boxes[name], SAFE_AREA):
            sys.exit(f"{name} is outside the safe area: {boxes[name]}")
    sagat = boxes["sagat-stance"]
    for name, box in boxes.items():
        if name != "sagat-stance" and box[3] - box[1] >= sagat[3] - sagat[1]:
            sys.exit(f"{name} stands as tall as Sagat")


def debug(guide: Image.Image) -> Image.Image:
    """The guide with the safe area and the title band marked."""
    marked = guide.copy()
    draw = ImageDraw.Draw(marked)
    font = ImageFont.load_default(size=36)
    l, t, r, b = SAFE_AREA
    draw.rectangle((l, t, r - 1, b - 1), outline=(0, 255, 0, 255), width=6)
    draw.text((l + 16, t + 12), "SAFE AREA: the square format", fill=(0, 255, 0, 255), font=font)
    l, t, r, b = TITLE_BAND
    draw.rectangle((l, t, r - 1, b - 1), outline=(255, 0, 255, 255), width=6)
    draw.text((l + 16, t + 12), "TITLE BAND", fill=(255, 0, 255, 255), font=font)
    return marked


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--captures", type=Path, default=LOCAL / "captures")
    parser.add_argument("--out", type=Path, default=LOCAL)
    args = parser.parse_args()
    guide, boxes = build(args.captures)
    check(boxes)
    args.out.mkdir(parents=True, exist_ok=True)
    guide.convert("RGB").save(args.out / "guide.png")
    debug(guide).convert("RGB").save(args.out / "guide-debug.png")
    print(f"wrote {args.out / 'guide.png'} and {args.out / 'guide-debug.png'} ({CANVAS[0]}x{CANVAS[1]})")


if __name__ == "__main__":
    main()
