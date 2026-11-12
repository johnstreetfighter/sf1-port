"""Touch up Geki's air-set candidates (AP12, #176) by hand.

usage: air-touch-ups.py <out folder>

As ground-touch-ups.py, whose Pose steps this reuses: for each pose,
<out>/<pose>.png (RGBA, his colours only), <out>/<pose>.json (the touch-up
list) and <out>/<pose>-sheet.png.

- The jumping punch is his ROM top-of-jump squat (0x0fc4, padded to 128x64 at
  (0, 0): inpaint-source-jump-128x64.png) with his front arm inpainted.
- The three jumping kicks are generated whole on that squat; only stray
  specks are touched. Airborne poses get no ground shadow.
"""
import importlib.util
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = sys.argv[1]

# The Pose steps live in ground-touch-ups.py, which runs its own poses when
# loaded; load it into a scratch folder so only its definitions are kept.
spec = importlib.util.spec_from_file_location("ground", os.path.join(HERE, "ground-touch-ups.py"))
ground = importlib.util.module_from_spec(spec)
saved_argv = sys.argv
sys.argv = [saved_argv[0], os.path.join(OUT, "_ground")]
spec.loader.exec_module(ground)
sys.argv = saved_argv
ground.OUT = OUT
Pose, load_near = ground.Pose, ground.load_near

grid, near = load_near("3df80656-0.png", "inpaint-mask-jump-punch.png")
p = Pose("jump_punch", grid)
p.log.append(f"his front arm and the chest behind its fist inpainted (x24-127, y15-30, and right of his hood x38-127, y4-14), driven straight out at head height, the fist clenched; the rest is his ROM top-of-jump squat (0x0fc4), pixel for pixel; {near} pixels the inpainting drew off his palette set to his nearest body pens")
p.islands()
p.save("3df80656-0.png")

WHOLE = [
    ("jump_kick", "71afdcf4-1.png"),
    ("jump_kick_high", "71afdcf4-0.png"),
    ("jump_kick_down", "5e473d30-0.png"),
]
for name, source in WHOLE:
    grid, _ = load_near(source)
    p = Pose(name, grid)
    p.islands()
    p.save(source)
