"""Write Birdie's generation ledger for AP21 round one (art/generations/birdie.json in the repository).

usage: ledger_r1.py <out.json> <crc32 of: crouch crouching_block crouching_hit crouching_punch crouching_kick sweep high_kick>
"""
import json, sys
OUT = sys.argv[1]
CRC = dict(zip(["crouch", "crouching_block", "crouching_hit", "crouching_punch", "crouching_kick", "sweep", "high_kick"], sys.argv[2:9]))
TU = json.load(open('../touched/touch-ups.json'))
PAL = ["#000000", "#664400", "#996655", "#cc9977", "#ddaa88", "#ddccaa", "#bbbbbb", "#777777", "#995500", "#888855",
       "#bbbb77", "#444444", "#555555", "#666666", "#777777"]
PRE = "1987 Capcom arcade fighter sprite, flat simple shading, dark outline, exactly the colours and the drawing of the rest of the sprite. "
G = lambda code, large, rows, cols: {"code": code, "large": large, "rows": rows, "cols": cols}
NOTE = ("Inpainting takes no palette: these are his 15 opaque pens, the ones its pixels were reduced to afterwards "
        "(pens 7 and 14 are the same grey; packing gives it pen 14, which his stance draws more).")
CROUCH = {"candidate": "ef1bf0cc-7679-4168-a1d3-7397092837e4#0",
          "usage": "the picture inpainted is built on his crouch as packed (his stance's upper body on this candidate's legs), so his ROM head, arms, body, rear thigh and boots stay pixel for pixel",
          "note": "make_ground.py in the pack's candidates/birdie/tools builds the sources and masks from his ROM poses."}

def call(at, move, job, cost, canvas, refs, prompt, kept):
    return {"at": at, "move": move, "tool": "PixelLab inpaint_image_pro_flash", "job": job, "cost": cost, "canvas": canvas, "seed": 1987,
            "palette": PAL, "references": refs, "prompt": PRE + prompt, "candidates": 1, "kept": kept, "note": NOTE}

calls = [
    call("2026-11-07T12:50:30Z", "crouch", "ef1bf0cc-7679-4168-a1d3-7397092837e4", 5, [64, 92],
         [{"rom": G("0x240c", True, 2, 1), "usage": "the top of his jump, upright with his fists on guard and his knees drawn up, from the head down to the knees (rows 0-75), set on the ground",
           "note": "inpaint-source-crouch-64x92.png and inpaint-mask-crouch.png in candidates/birdie/round1 (make_crouch.py)."},
          {"rom": G("0x2400", True, 3, 1), "usage": "his stance's boots and two-row ground shadow (rows 117-127), laid under the knees; the mask covers only the ankles between them (x8-61, y70-81), sparing the boots"}],
         "Birdie the punk brawler squatting low on his heels: his dark grey ripped jeans folded tight at the knees, the lower legs short and tucked under his thighs, the jeans' cuffs going into his brown boots, which stand flat on the floor. Empty background around the legs.",
         [{"candidate": 0, "pose": "crouch", "crc32": CRC["crouch"], "touch_ups": [
             "his stance's upper body (0x2400, rows 4-50) laid over the tuck's above row 47: the tuck's is his stance's with the head 4 pixels forward, and from row 47 down the two match pixel for pixel, so his crouch is his stance from the waist up",
             "a second rip the inpainting drew low on the rear leg, above the boot, given his jeans' black-and-grey dither",
             "a stray brown pixel jutting from the front boot's heel, and two pixels of the tuck's near elbow under his vest, removed"]}]),
    call("2026-11-07T13:00:20Z", "crouching hit", "a098315a-3ad6-4b31-9d89-12436f7c1f2d", 5, [68, 92],
         [CROUCH, {"rom": G("0x2600", True, 3, 1), "usage": "the first pose of his standing-hit reaction from the head to the belly (rows 4-56), laid on the crouch's legs at its torso's place; the mask covers the waist band where its belly and vest meet the jeans (x10-59, y45-57), sparing the flung hand",
                   "note": "inpaint-source-crouching-hit-68x92.png and inpaint-mask-crouching-hit.png in candidates/birdie/round1."}],
         "Birdie the punk brawler hit while squatting, his upper body jerked back: his bare belly and the bottom edges of his open black leather vest run down into the top of his dark grey ripped jeans over his bent knees; one body, joined cleanly at the waist. Nothing else.",
         [{"candidate": 0, "pose": "crouching_hit", "crc32": CRC["crouching_hit"], "touch_ups": TU["crouching_hit"]}]),
    call("2026-11-07T13:00:25Z", "crouching kick", "44889e0f-9fc1-40bf-93d3-a37d86592ef9", 6, [112, 92],
         [CROUCH, {"rom": G("0x27a8", True, 3, 3), "usage": "his kick's leg, thigh to boot (x34-99, y63-88), laid from the crouch's front hip 10 pixels above his kick's own height, in place of the crouch's front thigh, shin and boot; the mask covers the hip (x28-51, y48-67) and the space under the thigh (x30-53, y72-87)",
                   "note": "inpaint-source-crouching-kick-112x92.png and inpaint-mask-crouching-kick.png in candidates/birdie/round1."}],
         "Birdie the punk brawler squatting on his folded rear leg and kicking his front leg straight out: the thigh of his dark grey ripped jeans runs from his hip under his bare belly into the outstretched leg, joined cleanly at the hip; under the thigh, his folded rear leg's jeans and empty space down to the floor.",
         [{"candidate": 0, "pose": "crouching_kick", "crc32": CRC["crouching_kick"], "touch_ups": TU["crouching_kick"]}]),
    call("2026-11-07T13:00:30Z", "sweep", "89d140db-2af2-4501-a1f9-b0d41f2554da", 6, [112, 92],
         [CROUCH, {"rom": G("0x27a8", True, 3, 3), "usage": "his kick's leg, thigh to boot, laid low along the floor, 4 pixels below his kick's own height and 6 further out; the mask covers the hip as the thigh runs down to it (x28-57, y48-75) and the space under it (x30-55, y76-87)",
                   "note": "inpaint-source-sweep-112x92.png and inpaint-mask-sweep.png in candidates/birdie/round1."}],
         "Birdie the punk brawler squatting low on his folded rear leg and sweeping his front leg out along the floor: the thigh of his dark grey ripped jeans runs down from his hip under his bare belly into the outstretched leg lying low along the ground, joined cleanly at the hip; under the hip, his folded rear leg's jeans and empty space down to the floor.",
         [{"candidate": 0, "pose": "sweep", "crc32": CRC["sweep"], "touch_ups": TU["sweep"]}]),
    call("2026-11-07T13:00:35Z", "high kick", "fc5942a0-01f2-4a33-94d5-8795d22f135e", 6, [128, 128],
         [{"rom": G("0x27a8", True, 3, 3), "usage": "his kick's strike as the body: head, arms, vest, belt and standing leg, with the kicking leg taken away; that leg (x30-99, y63-88) turned 40 degrees up about the hip (rot.py: Scale2x three times, turned, sampled back) and laid as a guide; the mask covers the turned leg, 2 pixels around it, and the hip it rises from (x24-49, y54-79), sparing the standing leg",
           "note": "inpaint-source-high-kick-128x128.png and inpaint-mask-high-kick.png in candidates/birdie/round1."},
          {"rom": G("0x2726", True, 3, 1), "usage": "his kick chamber's ground shadow (rows 126-127), under the standing foot"}],
         "Birdie the punk brawler standing on one leg doing a high front kick: his kicking leg, in dark grey ripped jeans with a black outline and black-and-grey dithered shading, swung straight up from his hip toward head height, his brown boot at its end; the jeans leg joins his hip under his studded black belt cleanly. Empty background around the leg.",
         [{"candidate": 0, "pose": "high_kick", "crc32": CRC["high_kick"], "touch_ups": TU["high_kick"]}]),
]
FROM = {"fighter": "birdie", "pose": "crouch", "crc32": CRC["crouch"]}
derived = [
    {"pose": "crouching_block", "crc32": CRC["crouching_block"], "from": FROM,
     "touch_ups": ["his standing block's upper body (0x2588, rows 4-50) laid over the crouch's above row 47, as his stance's is: its waist matches the crouch's row for row, so no pixel is generated"]},
    {"pose": "crouching_punch", "crc32": CRC["crouching_punch"], "from": FROM,
     "touch_ups": ["his jab's upper body (0x2480, rows 4-50, from 33 pixels in) laid over the crouch's above row 47, as his stance's is: its waist matches the crouch's row for row, so no pixel is generated"]},
]
with open(OUT, 'w', newline='\n') as f:
    json.dump({"fighter": "birdie", "calls": calls, "derived": derived}, f, indent=2)
    f.write("\n")
