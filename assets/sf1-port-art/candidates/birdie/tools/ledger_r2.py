"""Add AP21 round two to Birdie's generation ledger (art/generations/birdie.json in the repository).

usage: ledger_r2.py <ledger.json> <crc32 of the new crouching_kick> <crc32 of rising>
"""
import json, sys
OUT, KICK, RISE = sys.argv[1], sys.argv[2], sys.argv[3]
TU = json.load(open('../touched2/touch-ups.json'))
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
for c in L["calls"]:
    for k in c["kept"]:
        if k["pose"] == "crouching_kick" and not k.get("note"):
            k["note"] = "Rejected by the maintainer in round one: \"The leg join should be a little lower.\" Replaced by round two's."
CROUCH2 = dict(CROUCH, note="make_round2.py in the pack's candidates/birdie/tools builds the sources and masks.")

def call(at, move, job, cost, canvas, refs, prompt, kept):
    return {"at": at, "move": move, "tool": "PixelLab inpaint_image_pro_flash", "job": job, "cost": cost, "canvas": canvas, "seed": 1987,
            "palette": PAL, "references": refs, "prompt": PRE + prompt, "candidates": 1, "kept": kept, "note": NOTE}

L["calls"] += [
    call("2026-11-07T13:48:00Z", "crouching kick", "2d194362-ef0e-49b8-9a93-b0a7ab96657d", 6, [112, 92],
         [CROUCH2, {"rom": G("0x27a8", True, 3, 3), "usage": "his kick's leg, thigh to boot (x34-99, y63-88), laid 5 pixels lower on the crouch's front hip than round one's; the mask covers the hip (x28-53, y50-71) and the space under the thigh (x30-55, y76-87)",
                    "note": "inpaint-source-crouching-kick-112x92.png and inpaint-mask-crouching-kick.png in candidates/birdie/round2."}],
         "Birdie the punk brawler squatting on his folded rear leg and kicking his front leg straight out: the thigh of his dark grey ripped jeans comes out low from his hip, below his bare belly, and runs into the outstretched leg, joined cleanly at the hip; under the thigh, his folded rear leg's jeans and empty space down to the floor.",
         [{"candidate": 0, "pose": "crouching_kick", "crc32": KICK, "touch_ups": TU["crouching_kick"]}]),
    call("2026-11-07T13:48:05Z", "get-up", "9c04a44f-1f21-4bd8-9c90-e0582f92f54e", 5, [68, 92],
         [CROUCH2, {"rom": G("0x2400", True, 3, 1), "usage": "through the crouch: his stance's upper body, guard up, kept as it is; the crouch's rear boot and the bottom of its rear leg are cleared and the mask covers the rear leg below the seat (x0-35, y66-87), sparing the front leg",
                    "note": "inpaint-source-rising-68x92.png and inpaint-mask-rising.png in candidates/birdie/round2. His ROM art has no pose that reads as getting up."}],
         "Birdie the punk brawler getting up from the floor, down on one knee: his rear knee rests on the ground under his hip, the shin of his dark grey ripped jeans lying back along the floor behind him, the toe of his brown boot on the ground at the back; his front foot is planted. Empty background around the leg.",
         [{"candidate": 0, "pose": "rising", "crc32": RISE, "touch_ups": TU["rising"]}]),
]
with open(OUT, 'w', newline='\n') as f:
    json.dump(L, f, indent=2)
    f.write("\n")
