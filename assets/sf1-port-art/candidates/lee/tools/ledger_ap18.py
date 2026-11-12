"""Add Lee's AP18 calls (air set and Zesshouohken) to his generation ledger (art/generations/lee.json).

usage: ledger_ap18.py <ledger>   (rewrites it; AP18's calls are replaced, the rest kept as they are)
"""
import json, sys
PATH = sys.argv[1]
TU = json.load(open('../touched-ap18/touch-ups.json'))
PAL = ["#000000", "#664400", "#996655", "#cc9977", "#ddaa88", "#ddccaa", "#ee4455", "#4444dd", "#666677", "#888899",
       "#aaaabb", "#444466", "#444477", "#444499", "#4444bb"]
PRE = "1987 Capcom arcade fighter sprite, flat simple shading, dark outline, exactly the colours and the drawing of the rest of the sprite. "
G = lambda code, large, rows, cols: {"code": code, "large": large, "rows": rows, "cols": cols}
NOTE = "Inpainting takes no palette: these are his 15 opaque pens, the ones its pixels were reduced to afterwards."
TUCK = {"rom": G("0x1e4c", True, 2, 1), "usage": "the picture inpainted is built on his top-of-jump tuck (0x1e4c), so his ROM head, rear hand, body and tucked legs stay pixel for pixel",
        "note": "make_air.py in the pack's candidates/lee/tools builds the sources and masks from his ROM poses."}
STEP = {"rom": G("0x1ccc", True, 2, 1), "usage": "the picture inpainted is his step-in (0x1ccc, his lunging punch's and high kick's first step), so his ROM head, body, rear arm and legs stay pixel for pixel",
        "note": "make_special.py in the pack's candidates/lee/tools builds the sources and masks from his ROM poses."}
PALM = {"rom": G("0x1c66", True, 2, 3), "usage": "his palm strike's open hand (x84-101, y12-34), fingers up, heel forward"}


def call(at, move, job, canvas, cost, refs, prompt, kept, note=NOTE):
    return {"at": at, "move": move, "tool": "PixelLab inpaint_image_pro_flash", "job": job, "cost": cost, "canvas": canvas, "seed": 1987,
            "palette": PAL, "references": refs, "prompt": PRE + prompt, "candidates": 1, "kept": kept, "note": note}


CALLS = [
    call("2026-11-07T06:38:00Z", "jumping punch", "c2fe6740-9472-46a4-96a6-5e5ae5b89559", [88, 80], 5,
         [TUCK, {"rom": G("0x1cc4", True, 2, 1), "usage": "his straight punch's arm (x34-63, y6-27: sleeve, forearm and fist) laid from the tuck's front shoulder in place of its guard fist, 8 pixels lower than his head-to-arm place in the punch so the fist is at shoulder height; the mask covers the shoulder join and the guard fist's place (x40-57, y12-41 and x46-65, y34-41), sparing his head and the forearm",
                 "note": "inpaint-source-jump-punch-88x80.png and inpaint-mask-jump-punch.png in candidates/lee/ap18-r1."}],
         "Lee the kung fu fighter in mid-air with his knees tucked, punching straight forward: his near arm driven out from his shoulder at shoulder height, in the wide royal-blue sleeve of his Chinese jacket, the sleeve joining his shoulder cleanly; his blue jacket's side and chest under the arm, down to the hem over his grey trousers. Nothing else in the empty space under the arm.",
         [{"candidate": 0, "pose": "jump_punch", "crc32": "cfb42dfa", "touch_ups": TU["jump_punch"]}]),
    call("2026-11-07T06:38:05Z", "jumping diagonal punch", "c2cd6eb6-4e1e-4b14-a03a-bbd3d1f5ff79", [84, 80], 5,
         [TUCK, {"rom": G("0x1ec0", True, 1, 2), "usage": "his flying kick's front arm (from x50, y16-46: sleeve and fist reaching down and forward), whose head lies exactly on the tuck's, laid at the same place in place of the guard fist; the mask covers the shoulder join and the guard fist's place (x40-63, y13-43), sparing his head and the fist",
                 "note": "inpaint-source-jump-punch-down-84x80.png and inpaint-mask-jump-punch-down.png in candidates/lee/ap18-r1."}],
         "Lee the kung fu fighter in mid-air with his knees tucked, punching down and forward: his near arm driven down from his shoulder toward the fist, in the wide royal-blue sleeve of his Chinese jacket, the sleeve joining his shoulder cleanly; his blue jacket's side and chest behind the arm, down to the hem over his grey trousers. Nothing else in the empty space above and below the arm.",
         [{"candidate": 0, "pose": "jump_punch_down", "crc32": "ea09c173", "touch_ups": TU["jump_punch_down"]}]),
    call("2026-11-07T06:38:10Z", "jumping kick", "e5ec59a1-c8b5-411e-b21c-0ca97d546616", [104, 80], 6,
         [TUCK, {"rom": G("0x1ea8", True, 2, 3), "usage": "his jumping heavy kick's leg (x44-105, y60-80: grey trouser leg and blue shoe) laid with its thigh at the tuck's front hip in place of the front knee, shin and foot, kicked straight out at waist height; the mask covers the hip (x32-53, y36-61), sparing the tucked rear knee",
                 "note": "inpaint-source-jump-kick-104x80.png and inpaint-mask-jump-kick.png in candidates/lee/ap18-r1."}],
         "Lee the kung fu fighter in mid-air kicking straight forward at waist height: the hem of his royal-blue Chinese jacket over his hip, his baggy light-grey trouser leg running from under the hem into the outstretched leg, joined cleanly at the hip; his other knee tucked up beside it. Empty space under the thigh.",
         [{"candidate": 0, "pose": "jump_kick", "crc32": "53c44f02", "touch_ups": TU["jump_kick"]}]),
    call("2026-11-07T06:46:00Z", "zesshouohken", "ba0968e3-1fdf-476d-b51b-fa6e6aaea90d", [112, 84], 6,
         [{"rom": G("0x1d20", True, 2, 3), "usage": "the picture inpainted is his lunging punch's strike (0x1d20), raised 12 rows, so his ROM head, body, near arm and lunge stay pixel for pixel; its sleeve's end and forearm (x76-93, y22-43) laid again 6 pixels back and 16 lower as the second arm",
           "note": "make_special.py in the pack's candidates/lee/tools; inpaint-source-zessh-strike-112x84.png and inpaint-mask-zessh-strike.png in candidates/lee/ap18-r1."},
          dict(PALM, usage=PALM["usage"] + ", laid in place of the lunge's fist on each forearm's end; the mask covers both wrists and the lower arm's root at his chest (x64-97, y6-49), sparing the palms")],
         "Lee the kung fu fighter lunging forward in a double-palm strike: both arms driven straight forward from the shoulders, one above the other, each in the wide royal-blue sleeve of his Chinese jacket, each ending in an open palm, heel forward, fingers up; the lower arm comes from his far shoulder, out from under the upper arm and in front of his chest. Clean wrists joining the forearms to the palms. Nothing else in the empty space around the arms.",
         [{"candidate": 0, "pose": "zessh_strike", "crc32": "5ebb906a", "touch_ups": TU["zessh_strike"]}]),
    call("2026-11-07T06:46:05Z", "zesshouohken", "4e35258d-b767-4993-99b6-249c62a9218d", [64, 96], 5,
         [STEP, dict(PALM, usage=PALM["usage"] + ", turned a quarter so the fingers point forward, laid at each hip in place of his fists; the mask covers his front arm from the shoulder (x36-59, y18-51) and his rear wrist")],
         "Lee the kung fu fighter stepping in, gathering his strength: both hands drawn back to his hips as open palms, fingers pointing forward; each arm bent at the elbow, hanging from its shoulder in the wide royal-blue sleeve of his Chinese jacket, the forearm running down to the palm at the hip. His blue jacket's chest and side behind the arms. Nothing else in the empty space around the arms.",
         [], NOTE + " Not kept: both arms came out long and limp in short sleeves, the turned palms reading as claws."),
    call("2026-11-07T06:50:00Z", "zesshouohken", "0eaf2211-9dd0-46e5-8ef3-e6ed5c38f2fe", [64, 96], 5,
         [dict(STEP, usage=STEP["usage"] + "; its rear fist, chambered at the rear hip, mirrored and laid at the front hip in place of the raised front fist; the mask covers the front arm from the shoulder to that fist (x36-59, y22-49)")],
         "Lee the kung fu fighter stepping in and drawing his front arm back: the upper arm hangs from his shoulder at his side in the wide royal-blue sleeve of his Chinese jacket, the elbow bent back, the bare forearm running forward from the sleeve's end to the fist chambered at his hip. His blue jacket's chest and side behind the arm. Nothing else in the empty space around the arm.",
         [], NOTE + " Not kept: no arm was drawn, so the fist floated at his hip. The gather was rebuilt instead from his own ROM pixels (the recipe's zessh_gather: the step-in's rear arm mirrored to the front side), with no generation."),
]
led = json.load(open(PATH))
jobs = {c["job"] for c in CALLS}
led["calls"] = [c for c in led["calls"] if c["job"] not in jobs] + CALLS
with open(PATH, 'w', newline='\n') as f:
    json.dump(led, f, indent=2)
    f.write("\n")
print(sum(c["cost"] for c in CALLS), "generations in", len(CALLS), "calls")
