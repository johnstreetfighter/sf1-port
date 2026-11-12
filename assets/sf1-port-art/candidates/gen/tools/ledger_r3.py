"""Add Gen's round-three calls to his generation ledger (AP19, #183), after rounds one and two.

usage: ledger_r3.py <art/generations/gen.json>   (run from this folder, after touch_ups_r3.py)

Round two's kept crouch, crouching punch, block and kick get a note on their fate.
"""
import json, sys
import ledger_r2 as l2   # rewrites the file with rounds one and two first
l1 = l2.l1
P = sys.argv[1]
TU = json.load(open('../touched3/touch-ups.json'))
d = json.load(open(P))
ROBE = "the robe looked cut short above the knees instead of hanging or folding low toward the floor"
GONE = {
    "8a1d8eba": f"Rejected by the maintainer in round two (2026-11-07): {ROBE}; the upright upper body was fine. Replaced in round three.",
    "5cf70eb1": f"Rejected by the maintainer in round two (2026-11-07), the same robe-length problem as the crouch it was built on. Replaced in round three.",
    "874a79b7": f"Rejected by the maintainer in round two (2026-11-07), the same robe-length problem as the crouch it was built on. Replaced in round three.",
    "f28b3bbd": f"Rejected by the maintainer in round two (2026-11-07), the same robe-length problem as the crouch it was built on. Replaced in round three.",
}
for c in d["calls"]:
    if c["job"][:8] in GONE:
        for k in c["kept"]: k["note"] = GONE[c["job"][:8]]
CRC = {"crouch": "a026300c", "crouching_punch": "ce7376ab", "crouching_block": "bdff7c1c", "crouching_kick": "6704b3a6"}
kept = lambda pose: [{"candidate": 0, "pose": pose, "crc32": CRC[pose], "touch_ups": TU[pose]}]
TOOLS = "make_round3.py in the pack's candidates/gen/tools builds the sources and masks from his ROM poses."
CROUCH_PROMPT = ("Gen the old kung fu master crouching low, upper body upright: his long purple robe hangs from his waist all the way down to the floor between and over his bent knees, its front panel falling straight down with the dark red trim along both edges and along the hem just above the floor; the robe folds over the tops of his baggy purple trouser knees. One body, nothing cut short.")
PANEL = {"rom": l1.G("0x2000", True, 2, 1), "usage": "the lower part of his stance's front panel (x21-38, rows 62-85: the panel with its red trim down both edges and its red hem, his rear ankle left out) hung from the waist to the floor between the round-two crouch's knees, 3 pixels right to meet the trim at his waist, the hem just above his ground shadow",
         "note": "the maintainer's round-two reason (2026-11-07): " + ROBE + "."}
R2 = lambda use: {"candidate": "8a1d8eba-e19f-4970-9575-03a156181d85#0", "usage": use, "note": TOOLS}
ON_CROUCH = {"candidate": "9c6213f5-865c-40dc-b001-6253f9af523e#0", "usage": "the picture inpainted is built on his round-three crouch (the kept candidate with its touch-ups, touched3/crouch.png), so his ROM head, other arm, robe and legs stay pixel for pixel", "note": TOOLS}
calls = [
    l1.call("2026-11-07T14:20:00Z", "crouch", "87059d51-3ee1-4779-93d3-4e63c0858c79", [72, 88],
            [R2("round two's crouch (upright upper body on bent legs), the picture inpainted; the mask covers only the waist band (x8-63, rows 60-71, sparing his forward hand) and the panel's edges where it falls past his knees (x22-27 and x38-45, rows 70-85), the panel's middle kept"), PANEL],
            CROUCH_PROMPT, [], l1.NOTE + " Not kept: its panel's right trim stepped sideways and its hem broke in two; the other mask's candidate was cleaner."),
    l1.call("2026-11-07T14:20:05Z", "crouch", "9c6213f5-865c-40dc-b001-6253f9af523e", [72, 88],
            [R2("round two's crouch (upright upper body on bent legs), the picture inpainted; the mask covers the whole robe below the waist between and over his knees (x16-50, rows 60-85), the panel there as the starting point, his shoes and forward hand kept"), PANEL],
            CROUCH_PROMPT, kept("crouch")),
    l1.call("2026-11-07T14:40:00Z", "crouching punch", "4fd906fe-6cea-455c-b3ab-21ea2d65b573", [88, 88],
            [ON_CROUCH, {"rom": l1.G("0x2126", True, 2, 2), "usage": "his straight punch's arm (x56-92, y11-31) driven out from the forward shoulder at chest height in place of the arm his stance holds out, his forward thigh hinted under it, as round 2b; the mask covers the shoulder join, the robe's side and the hint (x38-60, y40-68), sparing his head", "note": "candidates/gen/round3."}],
            "Gen the old kung fu master crouching low, upper body upright, punching straight forward: his front arm driven out from his shoulder at full reach in the wide purple sleeve of his robe, the sleeve joining his shoulder cleanly; under the arm, his long robe hanging down and his bent purple front thigh filling the space down to the knee, rounded, shaded like his other thigh. One body, nothing cut off, no straight edges.",
            kept("crouching_punch")),
    l1.call("2026-11-07T14:40:05Z", "crouching block", "0dad208b-ba77-49f1-98d0-6b31a0f1c234", [72, 88],
            [ON_CROUCH, {"rom": l1.G("0x200c", True, 2, 1), "usage": "his standing block's raised arm (x33-51, y9-41, his other hand left out) on the forward shoulder in place of the arm his stance holds out, his forward thigh hinted below the sleeve, as round 2b; the mask covers the shoulder join (x38-51, y40-51) and the space below the sleeve (x40-63, y54-68)", "note": "candidates/gen/round3."}],
            "Gen the old kung fu master crouching low in a crouching block, upper body upright: his front forearm raised before his face, the wide purple sleeve of his robe hanging from his shoulder to the elbow and joining his shoulder cleanly; below the sleeve, his long robe hanging down and his bent purple front thigh filling the space down to the knee, rounded, shaded like his other thigh. One body, no gaps, no straight edges.",
            kept("crouching_block")),
    l1.call("2026-11-07T14:40:10Z", "crouching kick", "5bdb7565-73bf-4ef3-9869-4c7c24ea0220", [96, 88],
            [ON_CROUCH, {"rom": l1.G("0x2309", True, 1, 2), "usage": "his flying kick's outstretched leg (x43-95, y37-63, his hand and the crop's top outline left out) from the hip along the floor in place of the forward knee and shoe (taken away: x44-71, rows 66-87), his stance's forward hand kept; the mask covers the hip join (x34-57, y60-79)", "note": "candidates/gen/round3."}],
            "Gen the old kung fu master crouching low, upper body upright, squatting on his rear leg with his front leg kicked straight out low along the floor: his long purple robe hangs down to the floor in front of his rear knee, and his baggy purple trouser leg comes out from under the robe at the hip into the outstretched leg, joined cleanly. Empty space under the thigh.",
            kept("crouching_kick")),
]
d["calls"] += calls
with open(P, 'w', newline='\n') as f:
    json.dump(d, f, indent=2)
    f.write("\n")
