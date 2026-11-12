"""view.py out.png scale file... : candidates side by side (reduced to his pens), on the review green."""
import os, sys
import rompose as r, sheet
items = [(os.path.basename(f), r.load(f)) for f in sys.argv[3:]]
sheet.sheet(items, sys.argv[1], int(sys.argv[2]), cols=len(items))
