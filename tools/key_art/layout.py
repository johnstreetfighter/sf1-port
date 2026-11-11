"""The key art's layout (ticket #234, KA1): every size and box the sprite
guide (`guide.py`) and the formats (`formats.py`) share, in one place.
The lettering boxes and seed serve only `formats.py --lettering`.

Boxes are (left, top, right, bottom) in master pixels, right and bottom
exclusive, as PIL takes them.
"""

from __future__ import annotations

# The master: 2:1, GitHub's social-preview size, and the README banner.
CANVAS = (2560, 1280)

# The square safe area: Ryu, Ken and Sagat (and Adon, Sagat's countryman)
# stand inside it. The square format is cut from exactly this box.
SAFE_AREA = (640, 0, 1920, 1280)

# The square format's size.
SQUARE = (1080, 1080)

# The banner's lettering band along the bottom: the guide keeps every
# fighter's feet above it, and the fan-made line is set in it.
TITLE_BAND = (0, 1090, 2560, 1280)

# The banner's lettering area: the title band, and above it the strip of
# the painting the torn title may overlap (the fighters' feet).
BANNER_LETTERING = (0, 1000, 2560, 1280)

# The square's lettering area, in square pixels.
SQUARE_LETTERING = (0, 770, 1080, 1080)

# The title's lines: each line's text is fitted inside its box and centred
# in it. One box sets "THE WORLD TOURNAMENT" on one line; two stack "THE"
# over "WORLD TOURNAMENT".
BANNER_TITLE = [(260, 1006, 2300, 1226)]
SQUARE_TITLE = [(370, 778, 710, 876), (40, 872, 1040, 1010)]
BANNER_FAN_LINE = (90, 1228, 2470, 1270)
SQUARE_FAN_LINE = (80, 1020, 1000, 1052)

# The dark scrim behind the lettering: it fades in over this many pixels
# from the top of its area, so the lettering reads over any painting.
SCRIM_FADE = 120

# The seed of the title's brush tears, so every run letters alike.
LETTERING_SEED = 1987

# The guide's ground lines: the front of the picture (Ryu and Ken, and the
# near pair each side) and the far pair each side, on their own stage's floor.
FRONT_GROUND = 1080
BACK_GROUND = 610


def inside(inner: tuple[int, int, int, int], outer: tuple[int, int, int, int]) -> bool:
    """Whether box `inner` lies wholly within box `outer`."""
    return outer[0] <= inner[0] and outer[1] <= inner[1] and inner[2] <= outer[2] and inner[3] <= outer[3]


def size(box: tuple[int, int, int, int]) -> tuple[int, int]:
    return box[2] - box[0], box[3] - box[1]
