"""Add AP25 round two to Adon's generation ledger (art/generations/adon.json in the repository).

usage: ledger_r2.py <ledger.json> <crc32 of: crouch crouching_block crouching_hit crouching_punch sweep rising>
"""
import json, sys
PAL = ["#000000", "#664400", "#996644", "#cc8866", "#ddaa88", "#ccaa00", "#aa7700", "#443344", "#550000", "#990000",
       "#cc4400", "#555555", "#777777", "#bbbbbb", "#ddccaa"]
PRE = "1987 Capcom arcade fighter sprite, flat simple shading, dark outline, exactly the colours and the drawing of the rest of the sprite. "
G = lambda code, large, rows, cols: {"code": code, "large": large, "rows": rows, "cols": cols}
NOTE = "Inpainting takes no palette: these are his 15 opaque pens, the ones its pixels were reduced to afterwards."
def call(at, move, job, cost, canvas, refs, prompt, kept, note=NOTE):
    return {"at": at, "move": move, "tool": "PixelLab inpaint_image_pro_flash", "job": job, "cost": cost, "canvas": canvas, "seed": 1987,
            "palette": PAL, "references": refs, "prompt": PRE + prompt, "candidates": 1, "kept": kept, "note": note}


PATH = sys.argv[1]
CRC = dict(zip(["crouch", "crouching_block", "crouching_hit", "crouching_punch", "sweep", "rising"], sys.argv[2:8]))
TU = json.load(open('../touched2/touch-ups.json'))
L = json.load(open(PATH))

R1 = {  # the maintainer's round-one reasons, on the round-one kept candidates and derived poses
    "crouch": 'Rejected by the maintainer in round one: "calf/foot join on the front leg is bad". Replaced in round two.',
    "sweep": 'Rejected by the maintainer in round one: "Too like the crouching kick", "Doesn\'t read as a sweep". Replaced in round two.',
    "rising": "Rejected by the maintainer in round one: the same calf and foot join on the front leg as the crouch. Replaced in round two.",
}
for c in L["calls"]:
    for k in c["kept"]:
        if k["pose"] in R1 and not k.get("note"): k["note"] = R1[k["pose"]]
for d in L["derived"]:
    if d["pose"] in ("crouching_block", "crouching_hit", "crouching_punch") and not d.get("note"):
        d["note"] = 'Rejected by the maintainer in round one: "same as the crouch". Replaced in round two, on the round-two crouch.'

CROUCH2 = {"candidate": "10bd952f-7e84-4263-9964-5ac9c219ff68#0",
           "usage": "the round-two crouch as packed: the picture inpainted is built on it",
           "note": "make_round2.py, make_round2b.py, make_sweep2.py and adon.py in the pack's candidates/adon/tools build the sources and masks."}
CROUCH1 = {"candidate": "b938e83f-50fb-459a-a7eb-9d170db8b5e0#0",
           "usage": "the round-one crouch: his stance's upper body, the tuck's shorts and thigh, the rear leg, all kept"}
CALF = {"rom": G("0x3060", True, 2, 1), "usage": "his stance's front lower leg as one piece (x33-50): the shaded calf (rows 70-79) as the guide inside the mask, running into the ankle wrap's top outline (row 80), the wrap from row 85, the bare foot and ground shadow, at the crouch's ground line"}
calls = [
    call("2026-11-07T16:20:40Z", "crouch", "c1fc086e-b71b-4338-84e1-a34f5b0a383d", 5, [64, 72],
         [CROUCH1, {"rom": G("0x3060", True, 2, 1), "usage": "his stance's front lower leg (x33-50, rows 73-95: the calf's bottom, wrap, foot and shadow) under the knee, the round-one thigh turned 25 degrees up about the hip; the mask covers the knee, calf and the hip under the thigh (x28-55, rows 44-55)",
                   "note": "Its source was overwritten by the next try's in candidates/adon/round2 (make_round2.py's first form)."}],
         "Adon the Muay Thai kickboxer squatting low: his front thigh rises from his dark shorts to a rounded bare knee, and under the knee his muscular tan calf, shaded like the rest of his legs with a dark brown outline, runs straight down into his grey ankle wrap and bare foot, one leg; under the thigh, his hip and empty space.",
         [], NOTE + " Not kept: it changed 2 pixels of 290, and with the thigh turned 25 degrees the knee stood high, reading as a raised knee rather than a squat."),
    call("2026-11-07T16:23:30Z", "crouch", "10bd952f-7e84-4263-9964-5ac9c219ff68", 5, [64, 72],
         [CROUCH1, dict(CALF, usage=CALF["usage"] + "; the round-one front thigh turned 8 degrees up about the hip (rot.py) above it. The mask covers the hip under the thigh, the knee and the whole calf down to the wrap's top (x28-55, rows 44-59), sparing the shorts, the wrap and the foot",
                        note="inpaint-source-crouch-64x72.png and inpaint-mask-crouch.png in candidates/adon/round2 (make_round2.py).")],
         "Adon the Muay Thai kickboxer squatting low: his front thigh runs from his dark shorts to a rounded bare knee, and under the knee his muscular tan calf, shaded like the rest of his legs with a dark brown outline, runs straight down into his grey ankle wrap and bare foot as one leg; under the thigh, his hip and empty space.",
         [{"candidate": 0, "pose": "crouch", "crc32": CRC["crouch"], "touch_ups": [
             "8 transparent holes the inpainting left in the front thigh's underside (x31-38, rows 51-53) filled, the outline in his legs' dark brown"]}]),
    call("2026-11-07T16:24:40Z", "sweep", "f01c4b4b-03a9-495c-9368-514a7cdea443", 6, [112, 72],
         [{"candidate": "c572eea5-22e0-4574-a53f-b437aba77bda#0", "usage": "the round-one rising pose as packed, its shorts, seat and kneeling rear leg (left of x31, from row 47): the body dropped onto the rear knee"},
          {"rom": G("0x3304", True, 2, 2), "usage": "his body punch's upper body (rows 0-41), bent forward, rear fist at his chin, front arm reaching forward, 16 pixels left and 7 down so its waist meets the kneel's lowered belt; the mask covers the waist band (x6-39, rows 45-50)"},
          {"rom": G("0x31e4", True, 2, 2), "usage": "his low kick's leg turned 31 degrees up about the hip (rot.py), laid from the front of the kneel's shorts long and nearly flat along the floor, its calf lengthened 8 pixels by repeating a run of its columns; the mask covers the hip and thigh (x24-51, rows 50-63), sparing the shorts",
           "note": "inpaint-source-sweep-112x72.png and inpaint-mask-sweep.png in candidates/adon/round2 (make_sweep2.py)."}],
         "Adon the Muay Thai kickboxer dropped low on his rear knee and leaning forward into a sweep: his bare tan stomach runs cleanly into his black belt with yellow studs and his dark shorts; from the front of the shorts his bare muscular thigh runs down and out into his front leg stretched long and flat along the floor, joined cleanly at the hip with a smooth dark brown outline; empty space above the leg.",
         [{"candidate": 0, "pose": "sweep", "crc32": CRC["sweep"], "touch_ups": TU["sweep"]}]),
    call("2026-11-07T16:25:10Z", "get-up", "54ff78aa-f948-4dcd-bf39-cef399133c53", 5, [96, 72],
         [{"candidate": "c572eea5-22e0-4574-a53f-b437aba77bda#0", "usage": "the round-one rising pose as packed, kept as it is: its upper body, hand, shorts and kneeling rear leg"},
          dict(CROUCH2, usage="the round-two crouch's front leg (thigh, calf, wrap, foot and shadow) in place of the round-one crouch's, wherever the rising pose's upper body and hand leave it showing; the mask covers the hip and the whole calf from the knee to the wrap's top (x28-55, rows 46-59), the calf as the guide, sparing the upper body, hand, wrap and foot",
               note="inpaint-source-rising-96x72.png and inpaint-mask-rising.png in candidates/adon/round2 (make_round2b.py).")],
         "Adon the Muay Thai kickboxer getting up off one knee, hand on his front knee: from his dark shorts his front thigh runs to a rounded bare knee under his hand, and under the knee his muscular tan calf, shaded like the rest of his legs with a dark brown outline, runs straight down into his grey ankle wrap and bare foot as one leg.",
         [{"candidate": 0, "pose": "rising", "crc32": CRC["rising"], "touch_ups": TU["rising"]}]),
]
L["calls"] += calls
FROM = {"fighter": "adon", "pose": "crouch", "crc32": CRC["crouch"]}
L["derived"] += [
    {"pose": "crouching_block", "crc32": CRC["crouching_block"], "from": FROM,
     "touch_ups": ["his standing block's upper body (0x312c, rows 0-34, its lifted knee left out) laid on the round-two crouch's belt and legs 1 pixel right and 5 down, as in round one: no pixel is generated"]},
    {"pose": "crouching_hit", "crc32": CRC["crouching_hit"], "from": FROM,
     "touch_ups": ["his standing hit's upper body (0x3180, rows 0-37, and its hanging front hand to row 44) laid on the round-two crouch's belt and legs 1 pixel right and 2 down, as in round one: no pixel is generated"]},
    {"pose": "crouching_punch", "crc32": CRC["crouching_punch"], "from": FROM,
     "touch_ups": ["his jab's upper body (0x30c4, rows 0-42) laid over the round-two crouch's above its belt, as in round one: no pixel is generated"]},
]
with open(PATH, 'w', newline='\n') as f:
    json.dump(L, f, indent=2)
    f.write("\n")
