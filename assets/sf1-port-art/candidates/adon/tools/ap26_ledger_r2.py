"""Add AP26 round two (the jumping kick) to Adon's generation ledger.

usage: ap26_ledger_r2.py <ledger.json> <crc32 of the round-two jumping kick> <crc32 of the round-one jumping kick>
"""
import json, sys
PAL = ["#000000", "#664400", "#996644", "#cc8866", "#ddaa88", "#ccaa00", "#aa7700", "#443344", "#550000", "#990000",
       "#cc4400", "#555555", "#777777", "#bbbbbb", "#ddccaa"]
PRE = "1987 Capcom arcade fighter sprite, flat simple shading, dark outline, exactly the colours and the drawing of the rest of the sprite. "
G = lambda code, large, rows, cols: {"code": code, "large": large, "rows": rows, "cols": cols}
NOTE = "Inpainting takes no palette: these are his 15 opaque pens, the ones its pixels were reduced to afterwards."


def call(at, move, job, cost, canvas, refs, prompt, kept, note=NOTE, seed=1987):
    return {"at": at, "move": move, "tool": "PixelLab inpaint_image_pro_flash", "job": job, "cost": cost, "canvas": canvas, "seed": seed,
            "palette": PAL, "references": refs, "prompt": PRE + prompt, "candidates": 1, "kept": kept, "note": note}


PATH, CRC, R1 = sys.argv[1:4]
TU = json.load(open('../ap26/touched2/touch-ups.json'))
SWITCH = "the maintainer's round-one reason (2026-11-07): \"It looks like it switches legs; the rear leg (from jump_top) becomes the front leg in jump_kick.\""
L = json.load(open(PATH))
for d in L["derived"]:
    if d["pose"] == "jump_kick" and d["crc32"] == R1 and not d.get("note"):
        d["note"] = "Rejected by the maintainer in round one: \"It looks like it switches legs; the rear leg (from jump_top) becomes the front leg in jump_kick.\" Replaced in round two."
REFS = [{"rom": G("0x3248", True, 2, 1), "usage": "the top of his jump, 3 pixels right and 2 down in his stance's coordinates, with its rear leg kept exactly (the wrap and foot behind, left of the black line the front wrap draws over it, every ROM pixel in place, outside the mask) and only its front leg taken away (the horizontal thigh, the calf folded under it, the front wrap and foot)"},
        {"candidate": "3605d103-1b06-4744-a8e2-fdfa6c44d497#0", "usage": "the approved crouching kick's leg and hip (his low kick's leg, 0x31e4, turned 27 degrees up), laid where it is in the crouching kick, from the same front hip, as the shaded guide; its shin and foot past the knee outside the mask"},
        {"rom": G("0x3244", True, 2, 1), "usage": "its hanging rear calf (rows 58-76, x10-30, without its wrap), turned 40 degrees clockwise about the ankle with the ankle on the rear wrap's top, as the guide for the rear shin the front leg hid; the mask covers the whole hip and thigh (x20-46, rows 40-58) and the rear shin above its wrap (x8-29, rows 52-65), sparing his upper body, the shorts away from the hip and the rear wrap and foot",
         "note": "inpaint-source-jump-kick-80x80.png and inpaint-mask-jump-kick.png in candidates/adon/ap26/r2 (ap26_make_r2.py). " + SWITCH}]
PROMPT = "Adon the Muay Thai kickboxer in mid-air kicking straight forward: his front bare tan muscular thigh comes out of the front of his dark purple-black shorts at the hip and runs straight out into the outstretched shin and grey ankle wrap, joined cleanly at the hip with a smooth dark brown outline. His rear leg stays tucked behind it: under the kicking thigh the rear knee is bent and the rear shin runs down and back into the grey ankle wrap and bare foot hanging below him. Two legs only."
L["calls"] = [c for c in L["calls"] if not (c["move"] == "jumping kick")]
L["calls"] += [
    call("2026-11-07T17:19:00Z", "jumping kick", "42a451a8-4973-4968-99ff-f6e8b1436caf", 5, [80, 80], REFS, PROMPT, [],
         NOTE + " Not kept: it changed 44 pixels and put a dark red streak at the hip; the other seed's was clean.", seed=1987),
    call("2026-11-07T17:19:05Z", "jumping kick", "03dd2aa6-176f-4ab7-9220-0c72a703a0f2", 5, [80, 80], REFS, PROMPT,
         [{"candidate": 0, "pose": "jump_kick", "crc32": CRC, "touch_ups": TU["jump_kick"]}],
         NOTE + " It kept the guides almost as they were (6 pixels changed in the mask), as AP25's block waist did; the candidate is kept with its touch-ups.", seed=2026),
]
with open(PATH, 'w', newline='\n') as f:
    json.dump(L, f, indent=2)
    f.write("\n")
