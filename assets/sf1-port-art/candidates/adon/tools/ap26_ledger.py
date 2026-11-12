"""Add Adon's AP26 calls (#190, air set and specials) to his generation ledger (art/generations/adon.json).

usage: ap26_ledger.py <ledger> <crc32s as pose=crc ...>   (run from this folder, after ap26_touch_ups.py; AP26's
entries are replaced, the rest kept)
"""
import json, sys
PATH = sys.argv[1]
CRC = dict(a.split('=') for a in sys.argv[2:])
TU = json.load(open('../ap26/touched/touch-ups.json'))
PAL = ["#000000", "#664400", "#996644", "#cc8866", "#ddaa88", "#ccaa00", "#aa7700", "#443344", "#550000", "#990000",
       "#cc4400", "#555555", "#777777", "#bbbbbb", "#ddccaa"]
PRE = "1987 Capcom arcade fighter sprite, flat simple shading, dark outline, exactly the colours and the drawing of the rest of the sprite. "
G = lambda code, large, rows, cols: {"code": code, "large": large, "rows": rows, "cols": cols}
NOTE = "Inpainting takes no palette: these are his 15 opaque pens, the ones its pixels were reduced to afterwards."
AP26 = {"jumping diagonal punch", "jumping heavy kick", "rising jaguar"}


def call(at, move, job, cost, canvas, refs, prompt, kept, note=NOTE, seed=1987):
    return {"at": at, "move": move, "tool": "PixelLab inpaint_image_pro_flash", "job": job, "cost": cost, "canvas": canvas, "seed": seed,
            "palette": PAL, "references": refs, "prompt": PRE + prompt, "candidates": 1, "kept": kept, "note": note}


R1 = "candidates/adon/ap26/r1 (ap26_make_r1.py and ap26_air.py in the pack's candidates/adon/tools)."
CALLS = [
    call("2026-11-07T16:51:50Z", "jumping diagonal punch", "1e47265c-d216-40cb-8a9c-fe09e05e19f7", 5, [72, 80],
         [{"rom": G("0x3248", True, 2, 1), "usage": "the top of his jump: its shorts and tucked legs (rows from 39), 3 pixels right and 2 down in his stance's coordinates, as on his crouch"},
          {"rom": G("0x30c4", True, 2, 2), "usage": "his jab's upper body (rows 0-42, 3 pixels right, as on his crouching punch), its arm taken away past the shoulder (x >= 41, rows 10-33)"},
          {"rom": G("0x330a", True, 1, 2), "usage": "the front arm of his flying kick's dive, from under his jaw to the fist (x >= 62 from row 22, x >= 68 from row 19), laid 24 pixels left and 4 up so it leaves the jab's shoulder; the mask covers the shoulder join (x38-49, rows 13-31), sparing his head and the chest's front edge, the arm's root inside it as the guide",
           "note": "inpaint-source-jump-punch-down-72x80.png and inpaint-mask-jump-punch-down.png in " + R1}],
         "Adon the Muay Thai kickboxer in mid-air with his knees tucked, punching down and forward: his bare tan muscular front arm grows out of his shoulder beside his jaw and runs straight down and forward to the fist, the round shoulder cap joining the chest cleanly with a smooth dark brown outline, shaded like his other arm. One body, nothing pasted on, no spikes.",
         [{"candidate": 0, "pose": "jump_punch_down", "crc32": CRC["jump_punch_down"], "touch_ups": TU["jump_punch_down"]}]),
    call("2026-11-07T16:51:55Z", "jumping heavy kick", "2f8f8e66-0418-452a-8fea-21ad14c291ca", 5, [96, 96],
         [{"rom": G("0x318a", True, 2, 2), "usage": "his roundhouse, leaning back with the front leg swung up to head height, as it is above his shorts; its standing leg (from row 54) turned 45 degrees back about the hip (29, 55) so it trails down behind him (rot.py), the ground shadow left out; the mask covers the hip and the trailing thigh's root under the shorts (x16-41, rows 52-70), sparing the shorts and the kicking thigh, the turned leg as the guide",
           "note": "inpaint-source-jump-kick-high-96x96.png and inpaint-mask-jump-kick-high.png in " + R1}],
         "Adon the Muay Thai kickboxer in a flying high kick in mid-air: under his dark purple-black shorts his rear bare tan muscular thigh comes out of the shorts at the hip and trails down and back behind him to the knee and shin, joined cleanly at the hip with a smooth dark brown outline; the bottom edge of the shorts runs straight across over the thigh. Empty space below the shorts in front of the thigh.",
         [{"candidate": 0, "pose": "jump_kick_high", "crc32": CRC["jump_kick_high"], "touch_ups": TU["jump_kick_high"]}]),
    call("2026-11-07T16:52:00Z", "rising jaguar", "805bef93-0222-41e7-8e2e-2488c4864352", 5, [64, 96],
         [{"rom": G("0x3244", True, 2, 1), "usage": "his rising pose, fists by his face and the rear leg hanging, as it is; its front thigh turned 60 degrees up about the hip (27, 48) and the folded calf, wrap and foot turned 40 degrees about the knee to hang under it (ap26_knee.py, rot.py); the rear calf's edge the old wrap covered given his outline by hand; the mask covers the hip, thigh and knee (x27-47, rows 32-49), sparing his shorts and the torso above row 33, the turned leg as the guide",
           "note": "inpaint-source-rj-knee-64x96.png and inpaint-mask-rj-knee.png in " + R1}],
         "Adon the Muay Thai kickboxer springing straight up with a rising knee strike: his front bare tan muscular thigh comes out of the front of his dark purple-black shorts at the hip and drives up and forward to the knee at chest height, the shin folded down under the knee into his grey ankle wrap; joined cleanly at the hip and the knee with a smooth dark brown outline. Empty space between the thigh and his other leg.",
         [{"candidate": 0, "pose": "rj_knee", "crc32": CRC["rj_knee"], "touch_ups": TU["rj_knee"]}]),
]
DERIVED = [
    {"pose": "jump_kick", "crc32": CRC["jump_kick"], "from": {"fighter": "adon", "pose": "crouching_kick", "crc32": "f03c8989"},
     "touch_ups": ["the top of his jump (0x3248), 3 pixels right and 2 down in his stance's coordinates, with its front thigh (rows 40-51) and its second wrap and foot (behind, x4-21, rows 61-76) taken away",
                   "the approved crouching kick's leg and inpainted hip (x >= 24 from row 40, without its shorts' pens left of x30 or its folded rear leg) laid where it is in the crouching kick, whose shorts are the tuck's at the same place; the tuck's folded calf, wrap and foot stay under the kicking thigh as the tucked rear leg (ap26_make_r1.py); no pixel is generated"]},
]
L = json.load(open(PATH))
L["calls"] = [c for c in L["calls"] if c["move"] not in AP26] + CALLS
L["derived"] = [d for d in L["derived"] if d["pose"] != "jump_kick"] + DERIVED
with open(PATH, 'w', newline='\n') as f:
    json.dump(L, f, indent=2)
    f.write("\n")
