"""Add AP26 round three (the jumping kick's rear shin, by hand) to Adon's generation ledger.

usage: ap26_ledger_r3.py <ledger.json> <crc32 of the round-three jumping kick> <crc32 of the round-two jumping kick>
"""
import json, sys
PATH, CRC, R2 = sys.argv[1:4]
L = json.load(open(PATH))
for c in L["calls"]:
    for k in c["kept"]:
        if k["pose"] == "jump_kick" and k["crc32"] == R2 and not k.get("note"):
            k["note"] = "Rejected by the maintainer in round two: \"shin/calf area on rear leg looks weird or lumpy.\" Its rear shin is redrawn by hand in round three."
L["derived"] = [d for d in L["derived"] if not (d["pose"] == "jump_kick" and d["crc32"] == CRC)]
L["derived"].append({"pose": "jump_kick", "crc32": CRC, "from": {"fighter": "adon", "pose": "jump_kick", "crc32": R2},
    "touch_ups": ["round two's kept pose, pixel for pixel, except the rear shin from under the kicking thigh to the top of its wrap (rows 55-63, x8-38): the turned rising-pose calf and the knee lump under the thigh taken away (119 pixels)",
                  "his stance's front calf (0x3060, rows 66-79, x35-46), a straight ROM calf outlined in his dark brown and lit on the right, laid in its place behind his shorts and the kicking thigh: its 14 rows taken down to the 9 visible rows by nearest sampling along the leg and sheared a pixel a row, so it runs down and back into the rear wrap's top, which stays the tuck's ROM pixels (ap26_make_r3.py); no pixel is generated"]})
with open(PATH, 'w', newline='\n') as f:
    json.dump(L, f, indent=2)
    f.write("\n")
