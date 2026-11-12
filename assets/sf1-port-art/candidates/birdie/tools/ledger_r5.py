"""Add AP21 round five to Birdie's generation ledger: two hand fixes, no generation.

usage: ledger_r5.py <ledger.json> <crc32 of the fixed crouching_kick> <crc32 of the fixed rising>
"""
import json, sys
OUT, KICK, RISE = sys.argv[1], sys.argv[2], sys.argv[3]
TU = json.load(open('../touched5/touch-ups.json'))
TU4 = json.load(open('../touched4/touch-ups.json'))
L = json.load(open(OUT))
for c in L["calls"]:
    for k in c["kept"]:
        if k["crc32"] == "65eea6da" and not k.get("note"):
            k["note"] = ("the maintainer, round four: \"the leg join is perfect now, but somehow his front hand is doubled.\" "
                         "The leg and join are kept; the doubled fist is fixed by hand for round five (derived).")
for d in L["derived"]:
    if d["crc32"] == "5e3717c8" and not d.get("note", "").startswith("Rejected"):
        d["note"] = "Rejected by the maintainer in round four: \"his butt still isn't rounded.\" " + d.get("note", "")
L["derived"] += [
    {"pose": "crouching_kick", "crc32": KICK, "from": {"fighter": "birdie", "pose": "crouching_kick", "crc32": "65eea6da"},
     "touch_ups": TU["crouching_kick"],
     "note": "fix_r5.py in the pack's candidates/birdie/tools. Rounds one to three never had the second fist: round four's source laid round three's picture from x52 on, 4 pixels higher, and that crop held the crouch's front fist as well as the leg."},
    {"pose": "rising", "crc32": RISE, "from": {"fighter": "birdie", "pose": "rising", "crc32": "2e7ecdf5"},
     "touch_ups": [t for d in L["derived"] if d["crc32"] == "5e3717c8" for t in d["touch_ups"]] + TU["rising"],
     "note": "seat_r4.py, then fix_r5.py, in the pack's candidates/birdie/tools. the maintainer, round four: \"his butt still isn't rounded.\""},
]
with open(OUT, 'w', newline='\n') as f:
    json.dump(L, f, indent=2)
    f.write("\n")
