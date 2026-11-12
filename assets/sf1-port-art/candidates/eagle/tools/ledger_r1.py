"""Write Eagle's generation ledger for AP23 round one (#187). usage: ledger_r1.py <ledger.json>"""
import json, sys
OUT = sys.argv[1]
TU = json.load(open('../touched/touch-ups.json'))
PAL = ["#000000", "#660000", "#996655", "#cc9977", "#ddaa88", "#ddccaa", "#ccccdd", "#aaaabb", "#888899", "#887700",
       "#bbaa00", "#666677", "#665555", "#554444", "#443333"]
NOTE = "Inpainting takes no palette: these are his 15 opaque pens, the ones its pixels were reduced to afterwards."
TOOL = "PixelLab inpaint_image_pro_flash"
G = lambda code, rows, cols: {"code": code, "large": True, "rows": rows, "cols": cols}
STANCE, TUCK, JAB = G("0x2920", 2, 1), G("0x2a40", 2, 1), G("0x2980", 2, 3)
SRC = "inpaint-source-{0}-{1}.png and inpaint-mask-{0}.png in the pack's candidates/eagle/r1 ({2} in candidates/eagle/tools)."
PRE = "1987 Capcom arcade fighter sprite, flat simple shading, dark outline, exactly the colours and the drawing of the rest of the sprite. "
CROUCH = "the approved crouch's legs (this ledger's kept crouch, df197c8d#0 touched up) from row 44 down, his black waistband and below"


def call(at, move, job, cost, canvas, refs, prompt, kept=(), note=""):
    return {"at": at, "move": move, "tool": TOOL, "job": job, "cost": cost, "canvas": canvas, "seed": 1987, "palette": PAL,
            "references": refs, "prompt": PRE + prompt, "candidates": 1, "kept": list(kept), "note": (NOTE + " " + note).strip()}


def jab_refs(deg, name, canvas, direction):
    return [{"rom": JAB,
             "usage": "his stick jab's strike (stream 0x1239e) down to its belt (rows 0-50), 16 left and 5 up onto his stance's place, on "
             + CROUCH + f"; its whole striking arm (right of x61, rows 8-37: shaded sleeve, forearm, fist and stick) turned {abs(deg)} degrees "
             + f"{direction} about the shoulder (63, 25) (rot.py); the mask covers the shoulder's join (within 7 pixels of the joint, right of the "
             + "torso's front edge) and the waist band where the jab's belt meets the crouch's trousers (rows 43-46), sparing his rear hand",
             "note": SRC.format(name, canvas, "make_r1b.py, eagle.py")}]


L = {"fighter": "eagle", "calls": [
    call("2026-11-07T15:05:40Z", "crouch", "44ab99c5-03da-4b8a-ac97-67393e9b24aa", 5, [72, 76],
         [{"rom": STANCE, "usage": "his stance from the head down to his belt (rows 0-45), upright, both sticks crossed before his chest"},
          {"rom": TUCK, "usage": "the top of his jump from its waist down (rows 36-55), 5 right and 8 down: the thighs, knees and shins, cut above the hanging feet"},
          {"rom": STANCE, "usage": "his stance's trouser hems, shoes and ground shadow (rows 86-95) under the cut shins, the front one 10 pixels in; the mask covered the shins' lower ends and the hems (rows 57-66)"}],
         "Eagle the English stick fighter crouching low: his baggy dark brown trousers run down from his bent knees, over his shins, to the trouser hems resting on his red shoes, both feet planted flat on the floor. Each trouser leg is one continuous shape with a black outline and the same brown shading as his thighs. Empty background between the legs.",
         note="Not kept: the front knee came back cut off square, and the front shin and hem as two loose pieces. The source was rebuilt with the front shin straight under the knee (the next call); this first source was overwritten."),
    call("2026-11-07T15:17:50Z", "crouch", "df197c8d-3b89-4e7a-bda6-6fc96f75053e", 5, [72, 76],
         [{"rom": STANCE, "usage": "his stance from the head down to his belt (rows 0-45), upright, both sticks crossed before his chest",
           "note": SRC.format("crouch-b", "72x76", "make_r1.py, eagle.py")},
          {"rom": TUCK, "usage": "the top of his jump from its waist down (row 36), 5 right and 8 down: the thighs and knees, the rear leg to row 54 and the front knee to row 55, its shin going back under him taken away"},
          {"rom": STANCE, "usage": "his stance's rear hem, shoe and ground shadow (rows 86-95) under the rear leg, and its front trouser leg's foot, hem, shoe and shadow (rows 84-95) 7 pixels in, straight under the front knee; the mask covers the front knee's lower end onto its shin (x36-61, rows 56-66) and the rear leg onto its hem (x10-31, rows 60-65)"}],
         "Eagle the English stick fighter crouching low: his bent front knee, in baggy dark brown trousers, comes down into a short shin that ends in the trouser hem resting on his red shoe; his rear trouser leg runs straight down into its hem on his rear shoe. Each trouser leg is one continuous shape with a black outline and the same brown shading as his thighs.",
         kept=[{"candidate": 0, "pose": "crouch", "crc32": "7a057856", "touch_ups": TU["crouch"]}]),
    call("2026-11-07T15:17:55Z", "rising stick strike", "c3e4604b-6430-40fd-98e3-de478c76c879", 6, [104, 128],
         [{"rom": JAB, "usage": "his stick jab's strike (stream 0x1239e), 32 rows down the canvas, with its whole striking arm (right of x61, rows 8-37: shaded sleeve, forearm, fist and stick) turned 35 degrees up about the shoulder (63, 25) (rot.py); the mask covers the shoulder's join (within 7 pixels of the joint, right of x60)",
           "note": SRC.format("rising-strike", "104x128", "make_r1.py, eagle.py")}],
         "Eagle the English stick fighter swinging a short fighting stick upward: his front arm, in a rolled-up lilac-grey shirt sleeve, rises from his shoulder up and forward, his fist holding the stick high. The sleeve grows cleanly out of his shoulder, its outline and folds running on from his shirt; his chest and suspenders stay as they are.",
         kept=[{"candidate": 0, "pose": "rising_strike", "crc32": "0fe2c7d7", "touch_ups": TU["rising_strike"]}]),
    call("2026-11-07T15:18:00Z", "two-stick smash", "285597a3-3fd8-4033-9f60-3f27ffaded20", 5, [96, 96],
         [{"rom": G("0x2a44", 2, 2), "usage": "his jumping stick strike (streams 0x126f8, 0x1285a, 0x129bc), 7 right and 10 down: its upper body above its waist (rows 0-35), hunched forward, and below it only its forearms, fists and sticks; the mask covers the waist band (rows 42-49, x10-45) where its shirt meets his stance's trousers, sparing the forearms",
           "note": SRC.format("smash", "96x96", "make_r1.py")},
          {"rom": STANCE, "usage": "his stance's legs from his belt down (rows 46-95), without his hands' last rows (46-49)"}],
         "Eagle the English stick fighter bent forward over his legs, smashing both sticks down before him: the bottom of his lilac-grey shirt is tucked into the black waistband of his baggy dark brown trousers, one clean waist with a black outline, no hands at his waist.",
         kept=[{"candidate": 0, "pose": "smash", "crc32": "65ea49af", "touch_ups": TU["smash"]}]),
    call("2026-11-07T15:22:40Z", "crouching stick sweep", "7fe080a7-8462-438c-aabc-5d25ff9750df", 5, [96, 76],
         jab_refs(-50, "crouching-sweep", "96x76", "down"),
         "Eagle the English stick fighter crouching, sweeping a short fighting stick low along the floor: his front arm, in a rolled-up lilac-grey shirt sleeve, swings down and forward from his shoulder. The sleeve grows cleanly out of his shoulder, its outline and folds running on from his shirt. His shirt is tucked into the black waistband of his baggy dark brown trousers, one clean waist.",
         kept=[{"candidate": 0, "pose": "crouching_sweep", "crc32": "9efc9e03", "touch_ups": TU["crouching_sweep"]}],
         note="The turned arm leaves out the sleeve's inner underside by the armpit (rows 31-37 left of x70), and the torso's front edge under the arm takes his outline pen (eagle.py, jab_arm and jab_body)."),
    call("2026-11-07T15:22:45Z", "crouching stick uppercut", "99ca28e8-8c97-4d38-a2c7-c3e203a79139", 6, [72, 120],
         jab_refs(60, "crouching-uppercut", "72x120", "up"),
         "Eagle the English stick fighter crouching, driving a short fighting stick straight up in an uppercut: his front arm, in a rolled-up lilac-grey shirt sleeve, rises steeply from his shoulder. The sleeve grows cleanly out of his shoulder, its outline and folds running on from his shirt. His shirt is tucked into the black waistband of his baggy dark brown trousers, one clean waist.",
         kept=[{"candidate": 0, "pose": "crouching_uppercut", "crc32": "f5d0aba0", "touch_ups": TU["crouching_uppercut"]}],
         note="The crouch sits 44 rows lower on this canvas, to fit the raised stick. The turned arm leaves out the sleeve's inner underside by the armpit, as on the sweep."),
    call("2026-11-07T15:22:50Z", "get-up", "9190a924-06cb-46a6-afab-b51732127d79", 5, [72, 76],
         [{"rom": G("0x29e8", 2, 1), "usage": "the squat his jumps take off from (stream 0x11eb4), its upper body down to its belt (rows 0-58) and both hands (to row 61), bent forward with a stick in each hand, 8 right and 16 lower than the crouch's upper body, so the hands come down on his knees, on "
           + CROUCH + "; the mask covers the waist band where its shirt and belt meet the thighs (rows 52-59), sparing his hands",
           "note": SRC.format("rising", "72x76", "make_r1b.py, eagle.py")}],
         "Eagle the English stick fighter getting up from the floor, bent forward low over his knees with his hands on them: the bottom of his lilac-grey shirt tucked into the black waistband of his baggy dark brown trousers, his hips low between his bent legs, one clean waist with a black outline.",
         kept=[{"candidate": 0, "pose": "rising", "crc32": "d1fe7055", "touch_ups": TU["rising"]}]),
], "derived": [
    {"pose": "crouching_block", "crc32": "f6f2ccdf", "from": {"fighter": "eagle", "pose": "crouch", "crc32": "7a057856"},
     "touch_ups": ["his standing block's upper body (0x2aa0, rows 0-48, 3 right and 3 up) laid over the crouch's above row 44, its belt on the crouch's black waistband; no pixel is generated"]},
    {"pose": "crouching_hit", "crc32": "613f1b7a", "from": {"fighter": "eagle", "pose": "crouch", "crc32": "7a057856"},
     "touch_ups": ["the first pose of his standing-hit reaction (0x2aa8, rows 0-47, 2 right and 2 up) laid over the crouch's above row 44, its belt on the crouch's black waistband; no pixel is generated"]},
]}
with open(OUT, 'w', newline='\n') as f:
    json.dump(L, f, indent=2)
    f.write("\n")
