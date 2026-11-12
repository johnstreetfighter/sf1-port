"""Look at Gen's airborne ROM poses turned in quarter turns (AP20, read-only)."""
import sys
import rompose as r, sheet, gen as src
items = []
for code, rows, cols in [(0x22a6, 2, 1), (0x2368, 1, 1), (0x21e0, 2, 2), (0x2186, 2, 2), (0x2309, 1, 2), (0x224c, 2, 1)]:
    for t in range(4):
        items.append((f"{code:04x} t{t}", src.part(code, True, rows, cols, turn=t)))
sheet.sheet(items, sys.argv[1], 3, cols=8)
