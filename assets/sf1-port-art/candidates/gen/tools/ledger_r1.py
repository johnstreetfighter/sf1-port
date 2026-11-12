"""Write Gen's generation ledger for AP19 round one (art/generations/gen.json in the repository).

usage: ledger_r1.py <out file>   (run from this folder, after touch_ups.py)
"""
import json, sys
OUT = sys.argv[1]
TU = json.load(open('../touched/touch-ups.json'))
PAL = ["#000000", "#664400", "#996655", "#cc9977", "#ddaa88", "#ddccaa", "#880000", "#660000", "#bbbbbb", "#888888",
       "#555555", "#442266", "#552277", "#772299", "#8822aa"]
PRE = "1987 Capcom arcade fighter sprite, flat simple shading, dark outline, exactly the colours and the drawing of the rest of the sprite. "
CRC = {"crouch": "9eb131e3", "crouching_block": "96e3e1b5", "crouching_hit": "3af06ff4", "crouching_punch": "2df6e407",
       "crouching_kick": "aaa60f12", "low_kick": "59055c75"}
G = lambda code, large, rows, cols: {"code": code, "large": large, "rows": rows, "cols": cols}
TOOLS = "make_crouch.py, make_ground.py, make_round1b.py and make_round1c.py in the pack's candidates/gen/tools build the sources and masks from his ROM poses."
CROUCH_JOB = "5cb120f1-6881-4582-8cd4-a2ee8908d7ae"
ON_CROUCH = {"candidate": CROUCH_JOB + "#0", "usage": "the picture inpainted is built on his finished crouch (the kept crouch candidate with its touch-ups, touched/crouch.png), so his ROM head, other hand, knees and shoes stay pixel for pixel",
             "note": TOOLS}
NOTE = "Inpainting takes no palette: these are his 15 opaque pens, the ones its pixels were reduced to afterwards."
def kept(pose):
    return [{"candidate": 0, "pose": pose, "crc32": CRC[pose], "touch_ups": TU[pose]}]
def call(at, move, job, canvas, refs, prompt, keep, note=NOTE):
    return {"at": at, "move": move, "tool": "PixelLab inpaint_image_pro_flash", "job": job, "cost": 5, "canvas": canvas, "seed": 1987,
            "palette": PAL, "references": refs, "prompt": PRE + prompt, "candidates": 1, "kept": keep, "note": note}
BLOCK1 = ("Gen the old kung fu master squatting low in a crouching block: his near forearm raised straight up before him with the fist at head height, the elbow bent down at his side, the wide purple sleeve of his robe with its grey cuff joining his shoulder; his purple robe's chest and knee below the raised arm. Nothing else in the empty space right of the arm.")
calls = [
    call("2026-11-07T05:58:00Z", "crouch", CROUCH_JOB, [64, 88],
         [{"rom": G("0x2064", True, 2, 1), "usage": "the squat his hit reactions recover through, with rows 72-79 across his thighs taken out (the recipe's crouch before its seam); the mask covers the band where the two halves meet (rows 68-76, over the figure and two pixels either side), sparing his head, hands and shoes",
           "note": "inpaint-source-crouch-64x88.png and inpaint-mask-crouch.png in candidates/gen/round1; " + TOOLS}],
         "Gen the old kung fu master squatting low: his baggy purple trousers over both bent thighs, knees wide apart, the hem of his long purple robe falling between them with its dark red trim running down diagonally and continuing the line above without a break; the thighs' dark outlines continuing smoothly from above to the knees below. One body, nothing cut off.",
         kept("crouch")),
    call("2026-11-07T06:04:00Z", "crouching block", "7fdd93ec-5303-4a07-a21f-5ab51538fecc", [64, 88],
         [ON_CROUCH, {"rom": G("0x200c", True, 2, 1), "usage": "his standing block's raised forearm and fist (x40-53, y9-36), stood on the crouch's forward sleeve in place of its clawed hand; the mask covers the elbow, the sleeve's end and the old hand's place (x38-61, y44-67), sparing his head",
                      "note": "candidates/gen/round1."}],
         BLOCK1, [], NOTE + " Not kept: the space below the elbow, where his robe and knee should be, came back empty, so the robe looked cut off."),
    call("2026-11-07T06:04:05Z", "crouching punch", "fc269754-392e-4adc-a44e-39113fd4fb48", [84, 88],
         [ON_CROUCH, {"rom": G("0x2126", True, 2, 2), "usage": "his straight punch's arm (x56-92, y11-31: sleeve, grey cuff and fist) laid from the crouch's forward shoulder in place of its clawed arm; the mask covers the shoulder join (x38-53, y42-65, sparing his head) and the space the old arm left (x52-65, y58-67)",
                      "note": "inpaint-source-crouching-punch-84x88.png and inpaint-mask-crouching-punch.png in candidates/gen/round1."}],
         "Gen the old kung fu master squatting low and punching straight forward: his near arm driven out from his shoulder at full reach in the wide purple sleeve of his robe, the sleeve joining his shoulder cleanly; his purple robe's side and chest under the arm, and the top of his bent purple knee below. Nothing else in the empty space under the arm.",
         kept("crouching_punch")),
    call("2026-11-07T06:04:10Z", "crouching kick", "9a186246-695e-47e1-b944-fd4055dc5c92", [96, 88],
         [ON_CROUCH, {"rom": G("0x2309", True, 1, 2), "usage": "his flying kick's outstretched leg (x43-95, y37-63: purple trouser leg and red shoe, his hand above it left out) laid from the crouch's hip in place of its forward knee and shoe, the shoe on the floor; his black ground shadow carried on under it; the mask covers the hip and the space under the robe (x34-57, y60-79), sparing his forward hand",
                      "note": "inpaint-source-crouching-kick-96x88.png and inpaint-mask-crouching-kick.png in candidates/gen/round1."}],
         "Gen the old kung fu master squatting on his rear leg with his front leg kicked straight out low along the floor: the hem of his long purple robe over his hip, his baggy purple trouser leg running from under the robe into the outstretched leg, joined cleanly at the hip. Empty space under the thigh.",
         kept("crouching_kick")),
    call("2026-11-07T06:04:15Z", "crouching hit", "6f651011-8e31-4e39-991d-1e009ff3db83", [64, 88],
         [ON_CROUCH, {"rom": G("0x2060", True, 2, 1), "usage": "his standing hit's upper body (rows 8-57), laid on the crouch's thighs with its top at row 10; the mask covers the band where they meet (rows 56-71), sparing his hands",
                      "note": "candidates/gen/round1."}],
         "Gen the old kung fu master hit while crouching, his head knocked back: his long purple robe from his chest down over his waist, its dark red trim running on down the front, its skirt falling over his bent knees in baggy purple trousers; one body, the robe's sides joining the knees cleanly at the waist.",
         [], NOTE + " Not kept: laid so high, he stood 75 pixels tall against the crouch's 63."),
    call("2026-11-07T06:04:20Z", "low kick", "3ca907f3-588d-45f7-8f8e-67df2dcaf9da", [96, 96],
         [{"rom": G("0x2180", True, 2, 2), "usage": "his front kick's strike, the picture inpainted: its high kicking leg taken away (everything right of x45 above row 50)",
           "note": TOOLS},
          {"rom": G("0x2309", True, 1, 2), "usage": "his flying kick's outstretched leg (x43-95, y37-63, his hand above it left out) driven out from the hip at knee height; the mask covers the hip and the robe behind the old leg (x32-59, y32-59), sparing his fist",
           "note": "inpaint-source-low-kick-96x96.png and inpaint-mask-low-kick.png in candidates/gen/round1."}],
         "Gen the old kung fu master standing on one leg, kicking straight out low at knee height: his baggy purple trouser leg driven out from his hip under his long purple robe, joined cleanly at the hip; the robe's back flap hanging behind his standing leg. Nothing else in the empty space above the leg.",
         kept("low_kick")),
    call("2026-11-07T06:09:30Z", "crouching block", "2164ace6-c91b-4734-92d6-1d0f9e7432fc", [64, 88],
         [ON_CROUCH, {"rom": G("0x200c", True, 2, 1), "usage": "his standing block's whole raised arm (x32-51, y9-41: the sleeve from the shoulder, the grey cuff, forearm and fist) laid on the crouch's forward shoulder, the sleeve hanging over where the old arm was; the mask covers the shoulder join (x36-49, y42-51, sparing his head) and the band where the sleeve meets his knee (x38-62, y56-68)",
                      "note": "inpaint-source-crouching-block-64x88.png and inpaint-mask-crouching-block.png in candidates/gen/round1b (make_round1b.py)."}],
         "Gen the old kung fu master squatting low in a crouching block, his near forearm raised before his face: the wide purple sleeve of his robe hangs from his shoulder down to the elbow and joins his shoulder cleanly; below the sleeve, the top of his bent purple trouser knee, one body with no gaps.",
         kept("crouching_block")),
    call("2026-11-07T06:13:00Z", "crouching hit", "fd13227c-6b71-44ce-90d5-d122fb5d0750", [64, 88],
         [ON_CROUCH, {"rom": G("0x2060", True, 2, 1), "usage": "his standing hit's upper body (rows 8-55: head knocked back, hands drawn in, down to the waist) laid 6 pixels lower than the first try, over the tops of the crouch's thighs; the mask covers the band where they meet (rows 58-73), sparing his hands",
                      "note": "inpaint-source-crouching-hit-64x88.png and inpaint-mask-crouching-hit.png in candidates/gen/round1c (make_round1c.py)."}],
         "Gen the old kung fu master hit while crouching low, his head knocked back: his long purple robe from his chest down over his waist, its dark red trim running on down the front, its skirt falling over his bent knees in baggy purple trousers; one body, the robe's sides joining the knees cleanly at the waist, nothing cut off.",
         kept("crouching_hit")),
]
with open(OUT, 'w', newline='\n') as f:
    json.dump({"fighter": "gen", "calls": calls}, f, indent=2)
    f.write("\n")
