"""Add AP23 round two (#187) to Eagle's generation ledger: the maintainer's round-one reasons on the three rejected poses,
the two retries' calls, and the smash's hand fix. usage: ledger_r2.py <ledger.json>"""
import json, sys
OUT = sys.argv[1]
TU = json.load(open('../touched-r2/touch-ups.json'))
L = json.load(open(OUT))
PAL = L["calls"][0]["palette"]
NOTE = L["calls"][0]["note"]
JAB = {"code": "0x2980", "large": True, "rows": 2, "cols": 3}
SRC = "inpaint-source-{0}-{1}.png and inpaint-mask-{0}.png in the pack's candidates/eagle/r2 (make_r2.py, eagle.py in candidates/eagle/tools)."
PRE = "1987 Capcom arcade fighter sprite, flat simple shading, dark outline, exactly the colours and the drawing of the rest of the sprite. "
REASONS = {
    "crouching_sweep": "Rejected by the maintainer in round one (2026-11-07): \"Arm looks pasted on.\" Replaced in round two (6735d064).",
    "rising_strike": "Rejected by the maintainer in round one (2026-11-07): \"the upper part of the arm (bicep/tricep) does not look as full as it should be, looks malformed.\" Replaced in round two (295e0a1c).",
    "smash": "Rejected by the maintainer in round one (2026-11-07): \"between the forearms should look transparent.\" Fixed by hand in round two (81babe5e, derived).",
}
for c in L["calls"]:
    for k in c["kept"]:
        if k["pose"] in REASONS: k["note"] = REASONS[k["pose"]]
ARM = ("its whole striking arm, the sleeve kept at its full width down to its own dark underside (right of x61, rows 8-33, outlined "
       "below at row 34 where the armpit is cut away), turned {0} degrees {1} about the shoulder (63, 25) and set 3 pixels back into "
       "the torso (eagle.py, turned_arm_full); the mask covers the whole shoulder and upper arm (the turned arm within 17 pixels of "
       "the joint, and the shoulder's cap within 7, from one pixel behind it), never his head")
SWEEP = ("Eagle the English stick fighter crouching, sweeping a short fighting stick low: his front arm hangs down and forward from a "
         "round, full shoulder. His upper arm is thick and muscular inside a rolled-up lilac-grey shirt sleeve with grey shading and a "
         "dark outline, growing out of his shoulder as one piece with his shirt, down to the rolled cuff at his elbow.")
STRIKE = ("Eagle the English stick fighter swinging a short fighting stick upward: his front arm rises up and forward from a round, "
          "full shoulder. His upper arm is thick and muscular, a full bicep and tricep inside a rolled-up lilac-grey shirt sleeve with "
          "grey shading and a dark outline, growing out of his shoulder as one piece with his shirt, up to the rolled cuff at his elbow.")
def call(at, move, job, cost, canvas, seed, usage, name, size, prompt, kept=None, note=""):
    return {"at": at, "move": move, "tool": "PixelLab inpaint_image_pro_flash", "job": job, "cost": cost, "canvas": canvas,
            "seed": seed, "palette": PAL, "references": [{"rom": JAB, "usage": usage, "note": SRC.format(name, size)}],
            "prompt": PRE + prompt, "candidates": 1, "kept": [kept] if kept else [], "note": (NOTE + " " + note).strip()}
SW_USE = ("his stick jab's strike (stream 0x1239e) down to its belt (rows 0-50), 16 left and 5 up onto his stance's place, its torso's "
          "front edge under the arm outlined, on the approved crouch's legs from row 44 down; " + ARM.format(50, "down")
          + "; and the waist band (rows 43-46) as in round one")
RS_USE = "his stick jab's strike (stream 0x1239e), 32 rows down the canvas, its torso's front edge under the arm outlined; " + ARM.format(35, "up")
TWIN = "One of two seeds sent with the same source, mask and prompt."
L["calls"] += [
    call("2026-11-07T15:58:40Z", "crouching stick sweep", "19e72928-a667-4df8-97de-3735729b82c8", 5, [96, 76], 1987, SW_USE,
         "crouching-sweep", "96x76", SWEEP, note=TWIN + " Not kept: a fold across the shoulder; seed 2026's sleeve is cleaner."),
    call("2026-11-07T15:58:45Z", "crouching stick sweep", "a0edde65-b241-457e-8a66-85b9c25e5526", 5, [96, 76], 2026, SW_USE,
         "crouching-sweep", "96x76", SWEEP,
         kept={"candidate": 0, "pose": "crouching_sweep", "crc32": "6735d064", "touch_ups": TU["crouching_sweep"]}, note=TWIN),
    call("2026-11-07T15:58:50Z", "rising stick strike", "1089a4d9-de6b-46de-8e87-540e24443d30", 6, [104, 128], 1987, RS_USE,
         "rising-strike", "104x128", STRIKE, note=TWIN + " Not kept: light specks outside the sleeve's outline at the armpit; seed 2026's upper arm is fuller and cleaner."),
    call("2026-11-07T15:58:55Z", "rising stick strike", "09fb9b9b-5448-4038-955d-7b843a4c7370", 6, [104, 128], 2026, RS_USE,
         "rising-strike", "104x128", STRIKE,
         kept={"candidate": 0, "pose": "rising_strike", "crc32": "295e0a1c", "touch_ups": TU["rising_strike"]}, note=TWIN),
]
L["derived"].append({"pose": "smash", "crc32": "81babe5e", "from": {"fighter": "eagle", "pose": "smash", "crc32": "65ea49af"},
                     "touch_ups": TU["smash"], "note": "fix_smash_r2.py in the pack's candidates/eagle/tools; no generation."})
with open(OUT, 'w', newline='\n') as f:
    json.dump(L, f, indent=2)
    f.write("\n")
