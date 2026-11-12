"""Add AP22 round one (Birdie's air set and Bull Horn, #186) to his generation ledger.

usage: ledger_ap22.py <ledger.json> <crc32 jump_kick> <crc32 jump_kick_high> <crc32 jump_kick_down> <crc32 bull_horn>
"""
import json, sys
OUT, JK, HIGH, DOWN, HORN = sys.argv[1:6]
TU = json.load(open('../touched-ap22/touch-ups.json'))
L = json.load(open(OUT))
PAL = L["calls"][0]["palette"]
NOTE = L["calls"][0]["note"]
TUCK = {"code": "0x240c", "large": True, "rows": 2, "cols": 1}
SRC = "make_air.py in the pack's candidates/birdie/tools builds the sources and masks; inpaint-source-{0}-{1}.png and inpaint-mask-{0}.png in candidates/birdie/ap22-r1."
KICK = ("his top-of-jump tuck as the body: head, arms, vest and near knee down to the hanging rear foot, with the front "
        "leg taken away; below row 47, left of x44 (canvas, the tuck's plus 4 on the left), the seat, hip and rear thigh are his approved "
        "crouching kick's (AP21), whose legs there are the tuck's")
PROMPT_HIGH = ("1987 Capcom arcade fighter sprite, flat simple shading, dark outline, exactly the colours and the drawing of the rest of the sprite. "
               "Birdie the punk brawler in mid-air, knees tucked, kicking his front leg up high: his kicking leg, in dark grey ripped jeans with a "
               "black outline and black-and-grey dithered shading, swung straight up and forward from his hip toward head height, his brown boot "
               "at its end; the jeans leg joins his hip under his bare belly and the bottom of his black leather vest cleanly. Empty background "
               "around the leg.")
PROMPT_DOWN = ("1987 Capcom arcade fighter sprite, flat simple shading, dark outline, exactly the colours and the drawing of the rest of the sprite. "
               "Birdie the punk brawler in mid-air, his rear knee tucked, driving a flying kick down and forward: his kicking leg, in dark grey "
               "ripped jeans with a black outline and black-and-grey dithered shading, thrust straight down and forward from his hip, his brown "
               "boot at its end; the jeans leg joins his hip under his bare belly cleanly. Empty background around the leg.")
PROMPT_HORN = ("1987 Capcom arcade fighter sprite, flat simple shading, dark outline, exactly the colours and the drawing of the rest of the sprite. "
               "Birdie the punk brawler charging head-first like a bull, his head lowered between his shoulders and both arms driven straight "
               "forward: the back of his neck and his bare muscular shoulders run cleanly from his shoulders into the back of his lowered head; "
               "his far forearm comes out from under his chin. One body, no gaps.")
LEG = ("his approved crouching kick's leg (AP21, kick 0x27a8's leg, thigh to boot: x40-107, y54-73 of its picture) turned {0} degrees "
       "{1} about the hip (rot.py) and laid from the tuck's front hip as a guide; the mask covers the turned leg, 2 pixels around it, "
       "and the hip it comes from (x30-51, y{2}), sparing the near knee")
L["calls"] += [
    {"at": "2026-11-07T14:53:20Z", "move": "jumping heavy kick", "tool": "PixelLab inpaint_image_pro_flash",
     "job": "7b64bd99-f175-4e8e-873a-438d25c55080", "cost": 6, "canvas": [112, 96], "seed": 1987, "palette": PAL,
     "references": [
         {"rom": TUCK, "usage": KICK, "note": SRC.format("jump-kick-high", "112x96")},
         {"candidate": "3d73fb9a-3835-41a3-8b1a-ec4992152a84#0", "usage": LEG.format(40, "up", "44-71")},
     ],
     "prompt": PROMPT_HIGH, "candidates": 1,
     "kept": [{"candidate": 0, "pose": "jump_kick_high", "crc32": HIGH, "touch_ups": TU["jump_kick_high"]}], "note": NOTE},
    {"at": "2026-11-07T14:53:25Z", "move": "jumping diagonal kick", "tool": "PixelLab inpaint_image_pro_flash",
     "job": "6c7fef02-2815-4abb-b848-5868087d1016", "cost": 6, "canvas": [112, 108], "seed": 1987, "palette": PAL,
     "references": [
         {"rom": TUCK, "usage": KICK, "note": SRC.format("jump-kick-down", "112x108")},
         {"candidate": "3d73fb9a-3835-41a3-8b1a-ec4992152a84#0", "usage": LEG.format(35, "down", "50-75")},
     ],
     "prompt": PROMPT_DOWN, "candidates": 1,
     "kept": [{"candidate": 0, "pose": "jump_kick_down", "crc32": DOWN, "touch_ups": TU["jump_kick_down"]}], "note": NOTE},
    {"at": "2026-11-07T14:53:30Z", "move": "bull horn", "tool": "PixelLab inpaint_image_pro_flash",
     "job": "d383ad74-935f-4c7f-8917-dfc2c9745ae7", "cost": 5, "canvas": [96, 96], "seed": 1987, "palette": PAL,
     "references": [
         {"rom": {"code": "0x271a", "large": True, "rows": 3, "cols": 2},
          "usage": "his diving attack's landing (streams 0x1165c, 0x119a0) as the body, rows 16-111: crouched low and leaning in, the near arm driven straight forward at waist height; its upright head and neck taken away (x36-63, y18-46, sparing the near shoulder); its near forearm and fist (x56-95, y50-79) laid again 2 pixels forward and 9 higher, behind everything, as his far arm; the mask covers the neck between his shoulders and head (x38-50, y35-48 of the pose) and the far forearm's root under his chin (x58-69, y46-55), sparing the head and the near arm",
          "note": SRC.format("bull-horn", "96x96") + " His legs below row 111 are the pose's own, laid back after."},
         {"rom": {"code": "0x28a0", "large": True, "rows": 2, "cols": 2},
          "usage": "his headbutt lunge's lowered head (x79-95, y6-27: crown forward, face down, his mohawk toward the opponent), every pixel, set at (49, 30) of the pose, low between his shoulders and behind his near arm"},
     ],
     "prompt": PROMPT_HORN, "candidates": 1,
     "kept": [{"candidate": 0, "pose": "bull_horn", "crc32": HORN, "touch_ups": TU["bull_horn"]}], "note": NOTE},
]
L["derived"].append(
    {"pose": "jump_kick", "crc32": JK, "from": {"fighter": "birdie", "pose": "crouching_kick", "crc32": "65eea6da"},
     "touch_ups": [
         "the approved crouching kick (round five's file, 410b00bf: this candidate with fix_r5.py's second fist removed) from row 47 down: the kicked leg and hip join from x44 on (rows 47-79), and left of x44 his seat and rear thigh down to row 69, laid on his top-of-jump tuck (0x240c), whose legs the crouch's are from row 47; the tuck's own rear shin and hanging foot below row 70, and its head, arms and body above row 47. No pixel is generated."],
     "note": "make_air.py in the pack's candidates/birdie/tools (air.py: body_without_front_leg, crouching_kick_leg)."})
with open(OUT, 'w', newline='\n') as f:
    json.dump(L, f, indent=2)
    f.write("\n")
