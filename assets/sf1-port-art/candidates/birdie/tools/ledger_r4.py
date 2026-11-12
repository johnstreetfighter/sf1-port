"""Add AP21 round four to Birdie's generation ledger (art/generations/birdie.json in the repository).

usage: ledger_r4.py <ledger.json> <crc32 of the new crouching_kick> <crc32 of the hand-fixed rising>
"""
import json, sys
OUT, KICK, RISE = sys.argv[1], sys.argv[2], sys.argv[3]
TU = json.load(open('../touched4/touch-ups.json'))
PAL = ["#000000", "#664400", "#996655", "#cc9977", "#ddaa88", "#ddccaa", "#bbbbbb", "#777777", "#995500", "#888855",
       "#bbbb77", "#444444", "#555555", "#666666", "#777777"]
PRE = "1987 Capcom arcade fighter sprite, flat simple shading, dark outline, exactly the colours and the drawing of the rest of the sprite. "
G = lambda code, large, rows, cols: {"code": code, "large": large, "rows": rows, "cols": cols}
NOTE = ("Inpainting takes no palette: these are his 15 opaque pens, the ones its pixels were reduced to afterwards "
        "(pens 7 and 14 are the same grey; packing gives it pen 14, which his stance draws more).")
CROUCH = {"candidate": "ef1bf0cc-7679-4168-a1d3-7397092837e4#0",
          "usage": "the picture inpainted is built on his crouch as packed (his stance's upper body on this candidate's legs), so his ROM head, arms, body, rear thigh and boots stay pixel for pixel",
          "note": "make_ground.py in the pack's candidates/birdie/tools builds the sources and masks from his ROM poses."}

L = json.load(open(OUT))
WHY = {"874f7e2d": "Rejected by the maintainer in round three: \"try again, now it's too low. If we have to accept one, the previous iteration was good enough.\" Replaced by round four's.",
       "2e7ecdf5": "Rejected by the maintainer in round three: \"Almost good, there is clipping or a bad join where his bottom/butt is.\" Its seat fixed by hand for round four (derived)."}
FALLBACK = " the maintainer, round three: \"If we have to accept one, the previous iteration was good enough\": this file is the accepted fallback if round four's is rejected."
for c in L["calls"]:
    for k in c["kept"]:
        if k["crc32"] in WHY and not k.get("note"): k["note"] = WHY[k["crc32"]]
        if k["crc32"] == "8c9f4b91" and "fallback" not in k.get("note", ""): k["note"] = k.get("note", "") + FALLBACK
CROUCH4 = dict(CROUCH, note="make_round4.py in the pack's candidates/birdie/tools builds the sources and masks.")

def call(at, move, job, cost, canvas, refs, prompt, kept):
    return {"at": at, "move": move, "tool": "PixelLab inpaint_image_pro_flash", "job": job, "cost": cost, "canvas": canvas, "seed": 1987,
            "palette": PAL, "references": refs, "prompt": PRE + prompt, "candidates": 1, "kept": kept, "note": NOTE}

L["calls"] += [
    call("2026-11-07T14:16:20Z", "crouching kick", "3d73fb9a-3835-41a3-8b1a-ec4992152a84", 6, [112, 92],
         [CROUCH4, {"candidate": "a728b5b3-4bf4-485e-8216-c75087d61061#0", "usage": "round three's level leg from the thigh outward (x52 on, as touched up), raised 4 pixels so the join sits about halfway between round two's and round three's; the mask covers only the hip join (x28-55, y50-83), sparing his belly",
                    "note": "inpaint-source-crouching-kick-112x92.png and inpaint-mask-crouching-kick.png in candidates/birdie/round4."}],
         "Birdie the punk brawler squatting on his folded rear leg and kicking his front leg straight out: the thigh of his dark grey ripped jeans comes out of his hip just below his bare belly and runs straight into the outstretched leg, joined cleanly at the hip; under the thigh, his folded rear leg's jeans and empty space down to the floor. Black outline, black-and-grey dithered shading on the jeans.",
         [{"candidate": 0, "pose": "crouching_kick", "crc32": KICK, "touch_ups": TU["crouching_kick"]}]),
    dict(call("2026-11-07T14:16:25Z", "get-up", "fd4681cd-11e8-48b7-ba5b-2260802e795b", 5, [96, 92],
         [{"candidate": "83859433-2433-45ea-8e55-2953907110e6#0", "usage": "round three's rising pose as packed; the mask covers only the seat and the back of the waist (x12-34, y46-62)",
           "note": "inpaint-source-rising-96x92.png and inpaint-mask-rising.png in candidates/birdie/round4."}],
         "Birdie the punk brawler kneeling and bent far forward: the back of his waist and his seat, a clean rounded seat in his dark grey ripped jeans with a black outline and black-and-grey dithered shading, his black studded belt running continuously across the back of his waist into the jeans' top; no gaps, no skin.",
         []), note=NOTE + " Not kept: it changed 71 pixels and left the holes at the back of the belt; the seat was fixed by hand instead (seat_r4.py, derived)."),
]
L.setdefault("derived", []).append(
    {"pose": "rising", "crc32": RISE, "from": {"fighter": "birdie", "pose": "rising", "crc32": "2e7ecdf5"},
     "touch_ups": ["his studded black belt drawn straight across the back of his waist (rows 50-53, from his back's outline to x38) in black with a stud every four pixels (pens 6 over 7), as his ROM belts are: it covers the holes the lunge's belt crop left and the jeans that stuck up above it, so the seat below is the kneeling legs' own rounded jeans",
                   "light specks on his back's outline above the belt set to black outline"],
     "note": "seat_r4.py in the pack's candidates/birdie/tools; the maintainer, round three: \"Almost good, there is clipping or a bad join where his bottom/butt is.\""})
with open(OUT, 'w', newline='\n') as f:
    json.dump(L, f, indent=2)
    f.write("\n")
