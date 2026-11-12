"""Add Gen's round-two calls to his generation ledger (AP19, #183), after ledger_r1.py.

usage: ledger_r2.py <art/generations/gen.json>   (run from this folder, after touch_ups_r2.py)

Round one's kept crouch, crouching punch and crouching block get a note on their fate.
"""
import json, sys
import ledger_r1 as l1   # rewrites the file with round one first
P = sys.argv[1]
TU = json.load(open('../touched2/touch-ups.json'))
d = json.load(open(P))
GONE = {
    "crouch": "Rejected by the maintainer in round one (2026-11-07): technically good, but it read more like a hit than a crouch. Replaced in round two by his stance from the waist up on these legs.",
    "crouching_punch": "Rejected by the maintainer in round one (2026-11-07) with the crouch it was built on. Replaced in round two.",
    "crouching_block": "Rejected by the maintainer in round one (2026-11-07) with the crouch it was built on. Replaced in round two.",
}
GONE["crouching_kick"] = ("Approved by the maintainer in round one (2026-11-07). Rebuilt the same way on the round-two crouch, "
                          "so its upper body matches the crouch it starts and ends on, as the coordinator asked before the maintainer's second look; "
                          "the rebuilt pose is reopened for review.")
for c in d["calls"]:
    for k in c["kept"]:
        if k["pose"] in GONE: k["note"] = GONE[k["pose"]]
CRC = {"crouch": "bd21a7ee", "crouching_punch": "3b93f9c5", "crouching_block": "d35a0ebb", "crouching_kick": "7c74cdb8"}
kept = lambda pose: [{"candidate": 0, "pose": pose, "crc32": CRC[pose], "touch_ups": TU[pose]}]
TOOLS = "make_round2.py in the pack's candidates/gen/tools builds the sources and masks from his ROM poses."
ON_CROUCH = {"candidate": "8a1d8eba-e19f-4970-9575-03a156181d85#0", "usage": "the picture inpainted is built on his round-two crouch (the kept candidate with its touch-ups, touched2/crouch.png), so his ROM head, other arm and legs stay pixel for pixel", "note": TOOLS}
CROUCH_PROMPT = ("Gen the old kung fu master crouching low, his upper body upright as in his fighting stance: below his waist his long purple robe is short, its skirt falling just over his bent knees, the front panel's dark red trim running down from his waist without a break to the hem between his knees; his baggy purple trousers bent wide at the knees below. One body, the robe joining the thighs cleanly, nothing cut off.")
CROUCH_REFS = [
    {"rom": l1.G("0x2000", True, 2, 1), "usage": "his stance from the head to the waist (rows 8-51), upright with both arms as they are, set 6 pixels right and 16 down on the legs (the maintainer's request, 2026-11-07)",
     "note": "inpaint-source-crouch-72x88.png and inpaint-mask-crouch.png in candidates/gen/round2; " + TOOLS},
    {"candidate": l1.CROUCH_JOB + "#0", "usage": "the bent legs of round one's crouch (touched/crouch.png from row 62: thighs, the robe's hem between the knees, shoes and ground shadow; its forward hand left out); the mask covers the band where the stance's waist meets them (rows 60-75), sparing his stance's hand",
     "note": "his hit-recovery squat (0x2064) lowered, with the trim redrawn by hand in round one"},
]
PUNCH_PROMPT = ("Gen the old kung fu master crouching low, upper body upright, punching straight forward: his front arm driven out from his shoulder at full reach in the wide purple sleeve of his robe, the sleeve joining his shoulder cleanly; under the arm, his purple robe's side running down to his bent knee. One body, nothing cut off.")
BLOCK_PROMPT = ("Gen the old kung fu master crouching low in a crouching block, upper body upright: his front forearm raised before his face, the wide purple sleeve of his robe hanging from his shoulder to the elbow and joining his shoulder cleanly; below the sleeve, his purple robe's side running down to the top of his bent knee, one body with no gaps.")
PUNCH_ARM = "his straight punch's arm (x56-92, y11-31: sleeve, grey cuff and fist) driven out from the crouch's forward shoulder at chest height, in place of the arm his stance holds out (taken away: x47 and right, rows 52-66)"
BLOCK_ARM = "his standing block's raised arm (x33-51, y9-41: the sleeve from the shoulder, the grey cuff, forearm and fist; his other hand left out) on the crouch's forward shoulder, in place of the arm his stance holds out (taken away: x47 and right, rows 52-66)"
calls = [
    l1.call("2026-11-07T12:10:00Z", "crouch", "8a1d8eba-e19f-4970-9575-03a156181d85", [72, 88], CROUCH_REFS, CROUCH_PROMPT, kept("crouch")),
    dict(l1.call("2026-11-07T12:10:05Z", "crouch", "5aed9525-0e22-4d08-b4e2-7772f82d62ff", [72, 88], CROUCH_REFS, CROUCH_PROMPT, [],
                 l1.NOTE + " Not kept: a dark line came back across his waist, like a seam; the other candidate (seed 1987) joined cleanly."), seed=1988),
    l1.call("2026-11-07T12:25:00Z", "crouching punch", "a7bd1f35-57b8-4f77-a1c4-baaa7563357a", [88, 88],
            [ON_CROUCH, {"rom": l1.G("0x2126", True, 2, 2), "usage": PUNCH_ARM + "; the mask covers the shoulder join and the robe's side under the arm (x38-57, y40-68), sparing his head", "note": "candidates/gen/round2."}],
            PUNCH_PROMPT, [], l1.NOTE + " Not kept: the space under the arm, where the old arm had hung over his forward thigh, came back empty, the robe ending in a straight edge."),
    l1.call("2026-11-07T12:25:05Z", "crouching block", "9cd2e6dd-4147-44aa-ac32-2b4d18ee85be", [72, 88],
            [ON_CROUCH, {"rom": l1.G("0x200c", True, 2, 1), "usage": BLOCK_ARM + "; the mask covers the shoulder join (x38-51, y40-51, sparing his head) and the space below the sleeve (x40-63, y54-68)", "note": "candidates/gen/round2."}],
            BLOCK_PROMPT, [], l1.NOTE + " Not kept: as the punch's, the space below the sleeve came back empty, the robe ending in a straight edge."),
    l1.call("2026-11-07T12:40:00Z", "crouching punch", "5cf70eb1-101f-4d4a-83ca-48c8c2b1a2a4", [88, 88],
            [ON_CROUCH, {"rom": l1.G("0x2126", True, 2, 2), "usage": PUNCH_ARM + "; under the arm his forward thigh hinted in flat purple (its edge curving from x52 at row 60 to the knee at row 66); the mask covers the shoulder join, the robe's side and the hint (x38-60, y40-68), sparing his head", "note": "inpaint-source-crouching-punch-88x88.png and inpaint-mask-crouching-punch.png in candidates/gen/round2b."}],
            "Gen the old kung fu master crouching low, upper body upright, punching straight forward: his front arm driven out from his shoulder at full reach in the wide purple sleeve of his robe, the sleeve joining his shoulder cleanly; under the arm, his robe's side and his bent purple front thigh filling the space down to the knee, rounded, shaded like his other thigh. One body, nothing cut off, no straight edges.",
            kept("crouching_punch")),
    l1.call("2026-11-07T12:40:05Z", "crouching block", "874a79b7-79b8-411c-a573-1ae7a3703118", [72, 88],
            [ON_CROUCH, {"rom": l1.G("0x200c", True, 2, 1), "usage": BLOCK_ARM + "; below the sleeve his forward thigh hinted in flat purple, as the punch's; the mask covers the shoulder join (x38-51, y40-51, sparing his head) and the space below the sleeve with the hint (x40-63, y54-68)", "note": "inpaint-source-crouching-block-72x88.png and inpaint-mask-crouching-block.png in candidates/gen/round2b."}],
            "Gen the old kung fu master crouching low in a crouching block, upper body upright: his front forearm raised before his face, the wide purple sleeve of his robe hanging from his shoulder to the elbow and joining his shoulder cleanly; below the sleeve, his robe's side and his bent purple front thigh filling the space down to the knee, rounded, shaded like his other thigh. One body, no gaps, no straight edges.",
            kept("crouching_block")),
]
calls.append(l1.call("2026-11-07T13:05:00Z", "crouching kick", "f28b3bbd-5be4-43b4-bd57-c49c898206c5", [96, 88],
    [ON_CROUCH, {"rom": l1.G("0x2309", True, 1, 2), "usage": "his flying kick's outstretched leg (x43-95, y37-63, his hand above it left out; round one's crop) laid from the round-two crouch's hip along the floor in place of its forward knee and shoe (taken away: x44-71, rows 66-87), his stance's forward hand kept above the thigh; his black ground shadow carried on under it; the mask covers the hip join (x34-57, y60-79), sparing the hand",
                 "note": "inpaint-source-crouching-kick-96x88.png and inpaint-mask-crouching-kick.png in candidates/gen/round2c (make_round2.py kick)."}],
    "Gen the old kung fu master crouching low, upper body upright, squatting on his rear leg with his front leg kicked straight out low along the floor: the short skirt of his purple robe over his hip, his baggy purple trouser leg running from under the robe into the outstretched leg, joined cleanly at the hip. Empty space under the thigh.",
    kept("crouching_kick")))
d["calls"] += calls
with open(P, 'w', newline='\n') as f:
    json.dump(d, f, indent=2)
    f.write("\n")
