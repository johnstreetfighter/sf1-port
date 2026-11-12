"""Add Gen's AP20 calls (#184, air set and specials) to his generation ledger (art/generations/gen.json).

usage: ap20_ledger.py <ledger>   (run from this folder, after ap20_touch_ups.py for both rounds; AP20's calls are replaced, the rest kept)
"""
import json, sys
PATH = sys.argv[1]
TU = json.load(open('../ap20/touched/touch-ups.json'))
TU2 = json.load(open('../ap20/touched2/touch-ups.json'))
TU3 = json.load(open('../ap20/touched3/touch-ups.json'))
WIDE = "the maintainer's round-two reason (2026-11-07): \"the robe/torso is extending too much to the right with the arm, doesn't look good at all.\""
PASTED = "the maintainer's round-one reason (2026-11-07): \"Arm looks pasted on.\""
PAL = ["#000000", "#664400", "#996655", "#cc9977", "#ddaa88", "#ddccaa", "#880000", "#660000", "#bbbbbb", "#888888",
       "#555555", "#442266", "#552277", "#772299", "#8822aa"]
PRE = "1987 Capcom arcade fighter sprite, flat simple shading, dark outline, exactly the colours and the drawing of the rest of the sprite. "
G = lambda code, large, rows, cols: {"code": code, "large": large, "rows": rows, "cols": cols}
NOTE = "Inpainting takes no palette: these are his 15 opaque pens, the ones its pixels were reduced to afterwards."
TUCK = {"rom": G("0x22a6", True, 2, 1), "usage": "the picture inpainted is built on his top-of-jump tuck (0x22a6), its front cuff and clawed hand taken away (x48-63, y10-29), so his ROM head, rear hand at his chest, robe and tucked legs stay pixel for pixel",
        "note": "ap20_make_air.py in the pack's candidates/gen/tools builds the sources and masks from his ROM poses."}


def call(at, move, job, canvas, cost, refs, prompt, kept, note=NOTE):
    return {"at": at, "move": move, "tool": "PixelLab inpaint_image_pro_flash", "job": job, "cost": cost, "canvas": canvas, "seed": 1987,
            "palette": PAL, "references": refs, "prompt": PRE + prompt, "candidates": 1, "kept": kept, "note": note}


CALLS = [
    call("2026-11-07T16:20:00Z", "jumping punch", "0dda1d67-b7e9-4eba-b47a-1b4ca12654b8", [88, 80], 5,
         [TUCK, {"rom": G("0x2126", True, 2, 2), "usage": "his straight punch's arm (x56-92, y11-31: sleeve, cuff and fist), whose head lies 8 pixels right of the tuck's, laid 8 left from the tuck's front shoulder at shoulder height; the mask covers the shoulder join (x38-57, y10-32), sparing his head, his rear hand and the fist",
                 "note": "inpaint-source-jump-punch-88x80.png and inpaint-mask-jump-punch.png in candidates/gen/ap20/r1."}],
         "Gen the old kung fu master in mid-air with his knees tucked, punching straight forward: his front arm driven out from his shoulder at shoulder height in the wide purple sleeve of his robe, the sleeve joining his shoulder cleanly behind his other hand at his chest; his purple robe's side and chest under the arm. Nothing else in the empty space under the arm.",
         [{"candidate": 0, "pose": "jump_punch", "crc32": "26caaa14", "touch_ups": TU["jump_punch"]}]),
    call("2026-11-07T16:20:05Z", "jumping diagonal punch", "1fee1e49-a9b2-4db0-8ec7-926aa3cefc2a", [88, 80], 5,
         [TUCK, {"rom": G("0x2246", True, 2, 2), "usage": "the front arm of his arm flurry's wheeling strike (from x56, y40-76: sleeve, cuff and open hand reaching down and forward; his beard and the thigh under the sleeve left out), laid 8 left and 22 up so the sleeve leaves the tuck's front shoulder and the hand reaches down past his knees; the mask covers the shoulder join (x38-59, y10-39), sparing his head, his rear hand and the hand",
                 "note": "inpaint-source-jump-punch-down-88x80.png and inpaint-mask-jump-punch-down.png in candidates/gen/ap20/r1."}],
         "Gen the old kung fu master in mid-air with his knees tucked, striking down and forward: his front arm driven down from his shoulder toward the open hand past his knees, in the wide purple sleeve of his robe, the sleeve joining his shoulder cleanly behind his other hand at his chest; his purple robe's chest behind the arm. Nothing else in the empty space above the arm.",
         [{"candidate": 0, "pose": "jump_punch_down", "crc32": "3c543735", "touch_ups": TU["jump_punch_down"],
           "note": f"Rejected by the maintainer in round one (2026-11-07): \"Arm looks pasted on.\" Replaced in round two."}]),
]
R2SRC = {"rom": G("0x2246", True, 2, 2), "usage": "only the lower sleeve, cuff and open hand of his arm flurry's down-reaching arm (round one's arm from x60, y30 on), laid where round one laid it; the upper sleeve, the shoulder and the old sleeve's outline under his rear hand cleared, and a flat hint drawn in his purples: one sleeve from the shoulder to the kept lower sleeve (outlined in his darkest purple), his chest filled under the rear hand, and the robe in shadow under the armpit down to the knee. The mask covers the whole shoulder, upper sleeve and armpit (x34-63, y10-45), sparing his head, his rear hand and the kept sleeve end, cuff and hand",
         "note": "inpaint-source-jump-punch-down-88x80.png and inpaint-mask-jump-punch-down.png in candidates/gen/ap20/r2 (ap20_make_r2.py). " + PASTED}
R2PROMPT = ("Gen the old kung fu master in mid-air with his knees tucked, striking down and forward: his front arm grows out of his shoulder in the wide purple sleeve of his robe, one continuous sleeve from the shoulder down to the grey cuff and open hand past his knees; the sleeve's dark outline and its folds run unbroken from his shoulder and chest into the arm, the robe gathered in shadow under the armpit down to the top of his knee. One body, nothing pasted on, no straight edges.")
CALLS += [
    dict(call("2026-11-07T17:05:00Z", "jumping diagonal punch", "2ff6105c-60fe-44d8-871b-bef03fa12894", [88, 80], 5, [TUCK, R2SRC], R2PROMPT, [],
              NOTE + " Not kept: black specks broke the sleeve's top and bottom outlines; the other seed's outline was clean."), seed=1987),
    dict(call("2026-11-07T17:05:05Z", "jumping diagonal punch", "0cf86e6f-a547-4365-937e-6d0318f9f847", [88, 80], 5, [TUCK, R2SRC], R2PROMPT,
              [{"candidate": 0, "pose": "jump_punch_down", "crc32": "195ef45c", "touch_ups": TU2["jump_punch_down"],
                "note": "Rejected by the maintainer in round two (2026-11-07): \"the robe/torso is extending too much to the right with the arm, doesn't look good at all.\" The hint and the shadow under the armpit widened his body toward the opponent. Replaced in round three."}]), seed=2026),
]
R3 = lambda hint: {"rom": G("0x2246", True, 2, 2), "usage": "only the cuff and open hand of his arm flurry's down-reaching arm, where round one laid them; the tuck's torso, robe and its front edge left as the ROM has them, outside the mask. A slim sleeve hinted from the tuck's own upper sleeve at the shoulder, under his rear hand, down and forward to the cuff (upper outline (49,15)-(70,34), lower (46,26)-(65,44)), " + hint + ". The mask covers only the shoulder join (x42-49, rows 14-29, sparing his rear hand and head) and the new sleeve with a one-pixel margin, never x < 42",
                    "note": "candidates/gen/ap20/r3 (ap20_make_r3.py). " + WIDE}
R3PROMPT = ("Gen the old kung fu master in mid-air with his knees tucked, striking down and forward: a slim arm in the purple sleeve of his robe leaves his shoulder just below his other hand and runs straight down and forward to the grey cuff and open hand, its dark outline and a fold or two along it, joined cleanly at the shoulder. His body and robe stay as they are; only the arm reaches out. Nothing else in the empty space around the arm.")
R3BPROMPT = ("Gen the old kung fu master in mid-air with his knees tucked, striking down and forward: a slim arm in the purple sleeve of his robe leaves his shoulder just below his other hand and runs straight down and forward to the grey cuff and open hand, lit along its top and shaded underneath like his other sleeves, its dark outline clean and unbroken, joined cleanly at the shoulder. His body and robe stay as they are; only the arm reaches out. Nothing else in the empty space around the arm.")
FLAT = "flat in his mid purple inside his darkest purple outline (inpaint-source-jump-punch-down-88x80.png)"
SHADED = "taken from the flurry arm's own shaded sleeve, each column squeezed to the slim sleeve's height by nearest sampling, so it keeps his ROM outline, highlight and shade bands (inpaint-source-jump-punch-down-shaded-88x80.png, the same mask)"
CALLS += [
    dict(call("2026-11-07T17:40:00Z", "jumping diagonal punch", "69a4622d-cafc-41c3-8d70-17f2f838f70c", [88, 80], 5, [TUCK, R3(FLAT)], R3PROMPT, [],
              NOTE + " Not kept: the sleeve came back almost as flat as the hint, without his sleeves' highlight and shade."), seed=1987),
    dict(call("2026-11-07T17:40:05Z", "jumping diagonal punch", "6ac95010-195a-44ff-909f-95f911955ff1", [88, 80], 5, [TUCK, R3(FLAT)], R3PROMPT, [],
              NOTE + " Not kept: the sleeve came back flat, as the other seed's."), seed=2026),
    dict(call("2026-11-07T17:46:00Z", "jumping diagonal punch", "9e1fab7e-e044-4405-8ed2-e6b311906b6f", [88, 80], 5, [TUCK, R3(SHADED)], R3BPROMPT, [],
              NOTE + " Not kept: a heavy black outline along both edges of the sleeve, where his sleeves are outlined in dark purple."), seed=1987),
    dict(call("2026-11-07T17:46:05Z", "jumping diagonal punch", "5f794b52-43d8-46c1-b036-7ae1a89dfbce", [88, 80], 5, [TUCK, R3(SHADED)], R3BPROMPT,
              [{"candidate": 0, "pose": "jump_punch_down", "crc32": "a62bf3e3", "touch_ups": TU3["jump_punch_down"],
                "note": "Rejected by the maintainer in round three (2026-11-07), the third rejection, so the pose went to him: \"It's very close now, the only issue I have with it is that the arm should be slightly more set back in his torso.\" His call was that small fix, made by hand on this same candidate (the next entry)."},
               {"candidate": 0, "pose": "jump_punch_down", "crc32": "78676183",
                "touch_ups": TU3["jump_punch_down"] + ["the maintainer's call after round three (2026-11-07): the whole arm (the inpainted shoulder join and slim sleeve, and the flurry's cuff and open hand) moved 3 pixels back toward his back by hand, so the shoulder sits further inside his torso; where the arm was, the tuck shows, and his head and rear hand are laid back on top as the ROM tuck draws them (ap20_make_r4.py, dx -3, dy 0). His torso and robe below row 29, the robe's front edge and his knees are the ROM tuck's, pixel for pixel. No generation."]}]), seed=2026),
]
led = json.load(open(PATH))
jobs = {c["job"] for c in CALLS}
led["calls"] = [c for c in led["calls"] if c["job"] not in jobs] + CALLS
with open(PATH, 'w', newline='\n') as f:
    json.dump(led, f, indent=2)
    f.write("\n")
print(sum(c["cost"] for c in CALLS), "generations in", len(CALLS), "calls")
