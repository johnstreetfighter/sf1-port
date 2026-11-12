"""Add AP25 round three (the crouching punch, by hand) to Adon's generation ledger.

usage: ledger_r3.py <ledger.json> <crc32 of the crouching punch> <crc32 of the round-two crouch>
"""
import json, sys
PATH, CRC, CROUCH = sys.argv[1:4]
L = json.load(open(PATH))
for d in L["derived"]:
    if d["pose"] == "crouching_punch" and d["from"]["crc32"] == CROUCH and not d.get("note"):
        d["note"] = "Rejected by the maintainer in round two: \"the top of his leg's outline seems to disappear compared to the crouch\". Replaced in round three."
L["derived"].append({"pose": "crouching_punch", "crc32": CRC, "from": {"fighter": "adon", "pose": "crouch", "crc32": CROUCH},
    "touch_ups": ["his jab's upper body (0x30c4, rows 0-42) laid over the round-two crouch's above its belt, as in round two",
                  "where the jab's torso and arm do not pass (rows 40-42, from x31 right), the crouch's 19 pixels put back: the end of the belt and the front thigh's top outline and highlight (make_round3.py); no pixel is generated"]})
with open(PATH, 'w', newline='\n') as f:
    json.dump(L, f, indent=2)
    f.write("\n")
