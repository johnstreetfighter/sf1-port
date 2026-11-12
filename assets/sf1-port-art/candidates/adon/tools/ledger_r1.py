"""Write Adon's generation ledger for AP25 round one (art/generations/adon.json in the repository).

usage: ledger_r1.py <out.json> <crc32 of: crouch crouching_block crouching_hit crouching_punch crouching_kick sweep rising>
"""
import json, sys
OUT = sys.argv[1]
CRC = dict(zip(["crouch", "crouching_block", "crouching_hit", "crouching_punch", "crouching_kick", "sweep", "rising"], sys.argv[2:9]))
TU = json.load(open('../touched/touch-ups.json'))
PAL = ["#000000", "#664400", "#996644", "#cc8866", "#ddaa88", "#ccaa00", "#aa7700", "#443344", "#550000", "#990000",
       "#cc4400", "#555555", "#777777", "#bbbbbb", "#ddccaa"]
PRE = "1987 Capcom arcade fighter sprite, flat simple shading, dark outline, exactly the colours and the drawing of the rest of the sprite. "
G = lambda code, large, rows, cols: {"code": code, "large": large, "rows": rows, "cols": cols}
NOTE = "Inpainting takes no palette: these are his 15 opaque pens, the ones its pixels were reduced to afterwards."
CROUCH = {"candidate": "b938e83f-50fb-459a-a7eb-9d170db8b5e0#0",
          "usage": "the picture inpainted is built on his crouch as packed, so his ROM head, arms, body, shorts and rear leg stay pixel for pixel",
          "note": "make_ground.py and adon.py in the pack's candidates/adon/tools build the sources and masks from his ROM poses."}


def call(at, move, job, cost, canvas, refs, prompt, kept, note=NOTE):
    return {"at": at, "move": move, "tool": "PixelLab inpaint_image_pro_flash", "job": job, "cost": cost, "canvas": canvas, "seed": 1987,
            "palette": PAL, "references": refs, "prompt": PRE + prompt, "candidates": 1, "kept": kept, "note": note}


CROUCH_TU = ["the front ankle wrap's top: 15 grey pixels the inpainting spread left of the shin removed, and the wrap closed under the knee with the top row of his stance's own wrap (row 80)",
             "1 speck of his shorts' dark purple inside the ankle wraps set to the pens around it"]
calls = [
    call("2026-11-07T15:45:30Z", "crouch", "b938e83f-50fb-459a-a7eb-9d170db8b5e0", 5, [64, 72],
         [{"rom": G("0x3060", True, 2, 1), "usage": "his stance from the head down to its belt (rows 0-42), upright, fists on guard; and its ankle wraps, bare feet and ground shadow, the front leg's (x34-50, rows 80-95) under the knee and the rear leg's (x8-23, rows 79-95) under the seat",
           "note": "inpaint-source-crouch-64x72.png and inpaint-mask-crouch.png in candidates/adon/round1 (make_crouch.py)."},
          {"rom": G("0x3248", True, 2, 1), "usage": "the top of his jump: its shorts and horizontal front thigh (rows 40-48, and the shorts' seat to row 53), 3 pixels right and 2 down so its belt meets the stance's; the shin folded under the thigh left out. The mask covers the knee's underside and the ankles between the thigh and seat and the wraps (x8-59, rows 48-57), sparing the wraps"}],
         "Adon the Muay Thai kickboxer squatting low on his heels: under his dark purple-black shorts his bare tan legs are folded tight, the front knee under the horizontal thigh going straight down into his grey ankle wrap, the back of the shorts rounding into the rear calf and the rear grey ankle wrap; his bare feet flat on the floor. Empty background between the legs.",
         [{"candidate": 0, "pose": "crouch", "crc32": CRC["crouch"], "touch_ups": CROUCH_TU}]),
    call("2026-11-07T15:51:40Z", "crouching block", "9f044488-4375-4224-b9e9-bb7606e7e450", 5, [64, 72],
         [CROUCH, {"rom": G("0x312c", True, 2, 1), "usage": "his standing block from the head to above its belt (rows 0-34), its lifted knee left out, laid on the crouch's belt and legs 1 pixel right and 5 down; the mask covers the waist band where its narrower waist meets the crouch's belt (x6-33, rows 34-40)",
                   "note": "inpaint-source-crouching-block-64x72.png and inpaint-mask-crouching-block.png in candidates/adon/round1."}],
         "Adon the Muay Thai kickboxer crouching on guard: his bare muscular tan stomach and the sides of his waist run straight down into his black belt with its yellow studs and the top of his dark shorts; one body, joined cleanly at the waist. Nothing else.",
         [], NOTE + " Not kept: it changed one pixel of the 166 in its mask, so the crouching block is the pieced source itself, recorded as derived from the crouch."),
    call("2026-11-07T15:51:45Z", "crouching kick", "3605d103-1b06-4744-a8e2-fdfa6c44d497", 5, [96, 72],
         [CROUCH, {"rom": G("0x31e4", True, 2, 2), "usage": "his low kick's leg, thigh to foot (x45 on, rows 51-91, without his shorts' pens), turned 27 degrees up about its hip (rot.py: Scale2x three times, turned, sampled back) and laid from the crouch's front hip, in place of the crouch's front thigh, knee, wrap and foot, under the crouch's shorts; the mask covers the hip and the turned thigh (x22-51, rows 43-59), sparing the shorts",
                   "note": "inpaint-source-crouching-kick-96x72.png and inpaint-mask-crouching-kick.png in candidates/adon/round1."}],
         "Adon the Muay Thai kickboxer squatting on his folded rear leg and kicking his front leg straight out low: the bare tan muscular thigh comes out from under the front of his dark shorts at the hip and runs into the outstretched shin, joined cleanly at the hip with a smooth dark brown outline; under the thigh, empty space down to the floor.",
         [{"candidate": 0, "pose": "crouching_kick", "crc32": CRC["crouching_kick"], "touch_ups": TU["crouching_kick"]}]),
    call("2026-11-07T15:51:50Z", "sweep", "1662670a-db9f-408a-a196-c083972e4c35", 5, [96, 72],
         [CROUCH, {"rom": G("0x31e4", True, 2, 2), "usage": "the same leg turned 17 degrees up, laid 2 pixels lower, low along the floor; the mask covers the hip and the turned thigh (x22-53, rows 43-65), sparing the shorts",
                   "note": "inpaint-source-sweep-96x72.png and inpaint-mask-sweep.png in candidates/adon/round1."}],
         "Adon the Muay Thai kickboxer squatting low on his folded rear leg and sweeping his front leg out along the floor: the bare tan muscular thigh comes out from under the front of his dark shorts at the hip and runs down into the outstretched leg low along the ground, joined cleanly at the hip with a smooth dark brown outline; under the hip, empty space down to the floor.",
         [{"candidate": 0, "pose": "sweep", "crc32": CRC["sweep"], "touch_ups": TU["sweep"]}]),
    call("2026-11-07T15:56:20Z", "get-up", "c572eea5-22e0-4574-a53f-b437aba77bda", 5, [96, 72],
         [CROUCH, {"rom": G("0x324c", True, 2, 1), "usage": "the hunched upper body of his hit recovery, head down, front hand reaching to the knee (rows 0-45, the forearm and hand to row 53), unturned, its waist on the crouch's belt lowered 7 pixels with the crouch's shorts; the crouch's front thigh, knee, wrap and foot planted where they are",
                   "note": "inpaint-source-rising-96x72.png and inpaint-mask-rising.png in candidates/adon/round1 (make_rising.py, at 0 degrees and a 7-pixel drop)."},
          {"rom": G("0x3060", True, 2, 1), "usage": "his stance's rear shin, wrap and foot (rows 60-95) turned 80 degrees to lie back along the floor as the guide for the rear leg; the mask covers the waist band (x6-30, rows 43-49), the hip where the lowered shorts meet the front thigh (x24-39, rows 47-57) and the rear leg below the seat (x0-35, rows 56-68)"}],
         "Adon the Muay Thai kickboxer getting up off the floor, down on one knee: his bare tan stomach runs cleanly into his black belt with yellow studs and his dark shorts; the back of the shorts rounds into his rear thigh, which goes straight down to the rear knee resting on the floor, the rear shin lying back along the floor into his grey ankle wrap and bare foot; his front thigh comes out of the shorts at the hip to his raised front knee.",
         [{"candidate": 0, "pose": "rising", "crc32": CRC["rising"], "touch_ups": TU["rising"]}]),
]
FROM = {"fighter": "adon", "pose": "crouch", "crc32": CRC["crouch"]}
derived = [
    {"pose": "crouching_block", "crc32": CRC["crouching_block"], "from": FROM,
     "touch_ups": ["his standing block's upper body (0x312c, rows 0-34, its lifted knee left out) laid on the crouch's belt and legs 1 pixel right and 5 down: no pixel is generated"]},
    {"pose": "crouching_hit", "crc32": CRC["crouching_hit"], "from": FROM,
     "touch_ups": ["his standing hit's upper body (0x3180, rows 0-37, and its hanging front hand to row 44) laid on the crouch's belt and legs 1 pixel right and 2 down: its waist meets the belt with no join, so no pixel is generated"]},
    {"pose": "crouching_punch", "crc32": CRC["crouching_punch"], "from": FROM,
     "touch_ups": ["his jab's upper body (0x30c4, rows 0-42) laid over the crouch's above its belt, as his stance's is: the jab's belt is his stance's 3 pixels right, so no pixel is generated"]},
]
with open(OUT, 'w', newline='\n') as f:
    json.dump({"fighter": "adon", "calls": calls, "derived": derived}, f, indent=2)
    f.write("\n")
