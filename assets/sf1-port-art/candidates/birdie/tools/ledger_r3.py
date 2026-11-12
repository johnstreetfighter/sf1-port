"""Add AP21 round three to Birdie's generation ledger (art/generations/birdie.json in the repository).

usage: ledger_r3.py <ledger.json> <crc32 of the new crouching_kick> <crc32 of rising>
"""
import json, sys
OUT, KICK, RISE = sys.argv[1], sys.argv[2], sys.argv[3]
TU = json.load(open('../touched3/touch-ups.json'))
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
WHY = {"8c9f4b91": "Rejected by the maintainer in round two: \"Join still too high.\" Replaced by round three's.",
       "e900dc7c": "Rejected by the maintainer in round two: \"It looks too similar to the crouch, Doesn't read as rising.\" Replaced by round three's."}
for c in L["calls"]:
    for k in c["kept"]:
        if k["crc32"] in WHY and not k.get("note"): k["note"] = WHY[k["crc32"]]
CROUCH3 = dict(CROUCH, note="make_round3.py in the pack's candidates/birdie/tools builds the sources and masks.")

def call(at, move, job, cost, canvas, refs, prompt, kept):
    return {"at": at, "move": move, "tool": "PixelLab inpaint_image_pro_flash", "job": job, "cost": cost, "canvas": canvas, "seed": 1987,
            "palette": PAL, "references": refs, "prompt": PRE + prompt, "candidates": 1, "kept": kept, "note": NOTE}

L["calls"] += [
    call("2026-11-07T14:10:00Z", "crouching kick", "a728b5b3-4bf4-485e-8216-c75087d61061", 6, [112, 92],
         [CROUCH3, {"rom": G("0x27a8", True, 3, 3), "usage": "his kick's leg, thigh to boot (x34-99, y63-88), turned 14 degrees up about the hip (rot.py) and laid from the crouch's hip crease, where the front thigh meets the seat (the thigh's top about 7 pixels below round two's), so the boot stays at shin height; the mask covers the turned leg, 2 pixels around it, and the hip (x28-55, y50-87), sparing his belly",
                    "note": "inpaint-source-crouching-kick-112x92.png and inpaint-mask-crouching-kick.png in candidates/birdie/round3."}],
         "Birdie the punk brawler squatting on his folded rear leg and kicking his front leg straight out: the leg comes out low from the crease of his hip, where his thigh meets his seat, and runs level into the outstretched dark grey ripped jeans leg and his brown boot; above it, his jeans' waist and hip under his bare belly; under the thigh, his folded rear leg and empty space down to the floor. Black outline, black-and-grey dithered shading on the jeans.",
         [{"candidate": 0, "pose": "crouching_kick", "crc32": KICK, "touch_ups": TU["crouching_kick"]}]),
    call("2026-11-07T14:10:05Z", "get-up", "83859433-2433-45ea-8e55-2953907110e6", 5, [96, 92],
         [{"candidate": "9c04a44f-1f21-4bd8-9c90-e0582f92f54e#0", "usage": "round two's kneeling legs (the crouch's front leg and boot planted, the rear knee down, the shin back along the floor), below the waist",
           "note": "make_round3.py in the pack's candidates/birdie/tools builds the source and mask."},
          {"rom": G("0x28a0", True, 2, 2), "usage": "his headbutt lunge's upper body (rows 0-45: head low, torso bent far forward, both fists before him), laid at the kneel's waist so the fists come over the front knee; the mask covers the waist band where its belt meets the jeans (x10-55, y46-61)",
           "note": "inpaint-source-rising-96x92.png and inpaint-mask-rising.png in candidates/birdie/round3."}],
         "Birdie the punk brawler pushing himself up off the floor from one knee, bent far forward over his front knee: the bottom of his black leather vest and his studded black belt run into the waist of his dark grey ripped jeans over his kneeling legs; one body, joined cleanly at the waist. Nothing else.",
         [{"candidate": 0, "pose": "rising", "crc32": RISE, "touch_ups": TU["rising"]}]),
]
with open(OUT, 'w', newline='\n') as f:
    json.dump(L, f, indent=2)
    f.write("\n")
