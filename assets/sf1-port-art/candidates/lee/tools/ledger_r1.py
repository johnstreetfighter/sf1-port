"""Write Lee's generation ledger for AP17 round one (art/generations/lee.json in the repository)."""
import json, sys
OUT = sys.argv[1]
TU = json.load(open('../touched/touch-ups.json'))
PAL = ["#000000", "#664400", "#996655", "#cc9977", "#ddaa88", "#ddccaa", "#ee4455", "#4444dd", "#666677", "#888899",
       "#aaaabb", "#444466", "#444477", "#444499", "#4444bb"]
PRE = "1987 Capcom arcade fighter sprite, flat simple shading, dark outline, exactly the colours and the drawing of the rest of the sprite. "
G = lambda code, large, rows, cols: {"code": code, "large": large, "rows": rows, "cols": cols}
CROUCH = {"rom": G("0x1f00", True, 1, 1), "usage": "the picture inpainted is built on his crouch: his sweep's closing squat (0x1f00), its head turned to face the opponent (the recipe's crouch), so his ROM head, knees and floor hand stay pixel for pixel",
          "note": "make_ground.py in the pack's candidates/lee/tools builds the sources and masks from his ROM poses."}
NOTE = "Inpainting takes no palette: these are his 15 opaque pens, the ones its pixels were reduced to afterwards."
def call(at, move, job, canvas, seed, refs, prompt, kept, note=NOTE):
    return {"at": at, "move": move, "tool": "PixelLab inpaint_image_pro_flash", "job": job, "cost": 5, "canvas": canvas, "seed": seed,
            "palette": PAL, "references": refs, "prompt": PRE + prompt, "candidates": 1, "kept": kept, "note": note}
calls = [
    call("2026-11-07T05:36:00Z", "crouching block", "6c8738ee-54d9-4640-b2c9-f5f2b73ede8b", [64, 72], 1987,
         [CROUCH, {"rom": G("0x1f40", True, 2, 2), "usage": "his standing block's raised forearm and fist (x40-53, y8-40), stood on the crouch's elbow in place of its guard fist; the mask covers the elbow, the shoulder and the old fist's place (x24-53, y8-40 of the 64x72 canvas), sparing the forearm above the cuff and his head",
                   "note": "inpaint-source-crouching-block-64x72.png and inpaint-mask-crouching-block.png in candidates/lee/round1."}],
         "Lee the kung fu fighter squatting low in a crouching block: his near forearm raised straight up before him, the elbow bent at his side, the wide royal-blue sleeve of his Chinese jacket falling back to the elbow, its dark inside showing; the upper arm joins his shoulder; his blue jacket's chest and shoulder behind the raised arm. Nothing else in the empty space to the right of the arm.",
         [{"candidate": 0, "pose": "crouching_block", "crc32": "a383bc8e", "touch_ups": TU["crouching_block"]}]),
    call("2026-11-07T05:36:05Z", "crouching punch", "2a2fb3db-4d51-4240-852d-e2e0c8a25a97", [64, 64], 1987,
         [CROUCH, {"rom": G("0x1cc4", True, 2, 1), "usage": "his straight punch's arm (x34-63, y4-28: sleeve, forearm and fist) laid from the crouch's right shoulder in place of its guard arm; the mask covers the shoulder join and the space the guard arm left (x22-41, y4-32 and x36-51, y18-32), sparing his head",
                   "note": "inpaint-source-crouching-punch-64x64.png and inpaint-mask-crouching-punch.png in candidates/lee/round1."}],
         "Lee the kung fu fighter squatting low and punching straight forward: his near arm driven out from his shoulder at full reach, in the wide royal-blue sleeve of his Chinese jacket, the sleeve joining his shoulder cleanly; his blue jacket's side and chest under the arm. Nothing else in the empty space under the arm.",
         [{"candidate": 0, "pose": "crouching_punch", "crc32": "bd34e3ed", "touch_ups": TU["crouching_punch"]}]),
    call("2026-11-07T05:36:10Z", "crouching kick", "e7de0bac-3ecf-4b1f-98f7-f06540e20469", [72, 64], 1987,
         [CROUCH, {"rom": G("0x1f04", True, 1, 2), "usage": "his sweep's outstretched leg (x44-84, y38-61: grey trouser leg and blue shoe) laid from the crouch's right hip in place of its right knee; the mask covers the hip and the space under the jacket's hem (x24-49, y33-61), sparing his floor hand",
                   "note": "inpaint-source-crouching-kick-72x64.png and inpaint-mask-crouching-kick.png in candidates/lee/round1."}],
         "Lee the kung fu fighter squatting on his rear leg with his front leg kicked straight out low along the floor: the hem of his royal-blue Chinese jacket over his hip, his baggy light-grey trouser leg running from under the hem into the outstretched leg, joined cleanly at the hip. Empty space under the thigh.",
         [{"candidate": 0, "pose": "crouching_kick", "crc32": "250d800b", "touch_ups": TU["crouching_kick"]}]),
    call("2026-11-07T05:36:15Z", "crouching hit", "65f3a55d-767e-480e-9cf8-86603a14dbeb", [68, 64], 1987,
         [CROUCH, {"rom": G("0x1fa0", True, 2, 2), "usage": "his hunched hit reaction from the head to the hands (rows 13-48), laid on the crouch's hem and knees, lined up by the frog buttons; the mask covers the band where they meet (y28-43) and the floor hand's place between the knees",
                   "note": "candidates/lee/round1 (make_ground.py's first version)."}],
         "Lee the kung fu fighter hit while crouching, hunched over his knees: his royal-blue Chinese jacket from his chest down over his waist to its hem, which falls over his bent knees in baggy light-grey trousers; one body, the jacket joining the knees cleanly. Between the knees the jacket's hem and the dark inside of his trousers.",
         [], NOTE + " Not kept: the band ran through his lower hand, whose fingers came back muddled."),
    call("2026-11-07T05:37:50Z", "crouching hit", "0d5ba65c-b714-4554-8ee1-dbf2171c24f3", [68, 64], 1988,
         [CROUCH, {"rom": G("0x1fa0", True, 2, 2), "usage": "as the first try, his hands kept whole outside the mask (x18-43 above y37)",
                   "note": "candidates/lee/round1b."}],
         "Lee the kung fu fighter hit while crouching, hunched over his knees: his royal-blue Chinese jacket from below his hands down over his waist to its hem, which falls over his bent knees in baggy light-grey trousers; one body, the jacket joining the knees cleanly. Between the knees the jacket's hem hangs down, with a little dark shadow under it.",
         [], NOTE + " Not kept: the source's crop of his hit reaction stopped at row 48, cutting his lower hand off at the knuckles."),
    call("2026-11-07T05:38:40Z", "crouching hit", "a42c0543-d00b-4a4a-983e-006302cde52b", [68, 64], 1988,
         [CROUCH, {"rom": G("0x1fa0", True, 2, 2), "usage": "his hunched hit reaction from the head to the hands (rows 13-53), laid on the crouch's hem and knees, lined up by the frog buttons; the mask covers the band where they meet (y28-43), sparing his hands and their outlines, and the floor hand's place between the knees",
                   "note": "inpaint-source-crouching-hit-68x64.png and inpaint-mask-crouching-hit.png in candidates/lee/round1c."}],
         "Lee the kung fu fighter hit while crouching, hunched over his knees: his royal-blue Chinese jacket from below his hands down over his waist to its hem, which falls over his bent knees in baggy light-grey trousers; one body, the jacket joining the knees cleanly. Between the knees the jacket's hem hangs down, with a little dark shadow under it.",
         [{"candidate": 0, "pose": "crouching_hit", "crc32": "bef9f2ae", "touch_ups": TU["crouching_hit"]}]),
]
with open(OUT, 'w', newline='\n') as f:
    json.dump({"fighter": "lee", "calls": calls}, f, indent=2)
    f.write("\n")
