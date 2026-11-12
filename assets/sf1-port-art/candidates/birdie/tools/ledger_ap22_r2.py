"""Add AP22 round two (Birdie's heavy jumping kick and Bull Horn, #186) to his generation ledger, with the maintainer's
round-one words on the two files he rejected.

usage: ledger_ap22_r2.py <ledger.json> <crc32 jump_kick_high> <crc32 bull_horn>
"""
import json, sys
OUT, HIGH, HORN = sys.argv[1:4]
TU = json.load(open('../touched-ap22-r2/touch-ups.json'))
L = json.load(open(OUT))
PAL = L["calls"][0]["palette"]
NOTE = L["calls"][0]["note"]
WORDS = {
    "fd378b71": "Rejected by the maintainer in round one: \"redo his whole lower half, the non-kicking leg should probably be at a different angle to look more animated\", \"Leg looks pasted on\", \"Too like the jump kick.\" Replaced by round two's.",
    "3c9e3536": "Rejected by the maintainer in round one: \"Looks like 4 arms and the head seems malformed\", \"Head hidden or lost\", \"Arms look wrong.\" Replaced by round two's.",
}
for c in L["calls"]:
    for k in c["kept"]:
        if k["crc32"] in WORDS and not k.get("note"):
            k["note"] = WORDS[k["crc32"]]
SRC = "make_air_r2.py in the pack's candidates/birdie/tools builds the sources and masks; inpaint-source-{0}-{1}.png and inpaint-mask-{0}.png in candidates/birdie/ap22-r2."
L["calls"] += [
    {"at": "2026-11-07T15:17:45Z", "move": "jumping heavy kick", "tool": "PixelLab inpaint_image_pro_flash",
     "job": "a79b952d-b21e-4f5f-8640-2fd8f40feccb", "cost": 6, "canvas": [148, 128], "seed": 1987, "palette": PAL,
     "references": [
         {"candidate": "fc5942a0-01f2-4a33-94d5-8795d22f135e#0",
          "usage": "his approved standing high kick (AP21, as touched: his kick's strike 0x27a8, the leg swung up from an inpainted hip) placed 20 pixels in from the left, with its standing leg (below row 62, left of the kicking thigh) taken out and turned 26 degrees back about the hip (rot.py) so it trails down and back, and its floor shadow removed; the mask covers everything from the belt down (x24-75, y61-93: hip, seat, both upper thighs) and the whole turned leg with 2 pixels around it; the kicking shin and boot and everything above the belt stay",
          "note": SRC.format("jump-kick-high", "148x128")},
     ],
     "prompt": ("1987 Capcom arcade fighter sprite, flat simple shading, dark outline, exactly the colours and the drawing of the rest of the sprite. "
                "Birdie the punk brawler in a flying high kick, legs split wide in mid-air: his front leg swung straight up toward head height, his rear "
                "leg stretched straight down and back behind him; his dark grey ripped jeans with a black outline and black-and-grey dithered shading run "
                "from his studded black belt into both thighs through one rounded hip and seat, joined cleanly, no seams. Empty background between the legs."),
     "candidates": 1,
     "kept": [{"candidate": 0, "pose": "jump_kick_high", "crc32": HIGH, "touch_ups": TU["jump_kick_high"]}], "note": NOTE},
    {"at": "2026-11-07T15:17:50Z", "move": "bull horn", "tool": "PixelLab inpaint_image_pro_flash",
     "job": "2c2786de-7ac7-439a-b28f-8cf406f7497c", "cost": 5, "canvas": [96, 96], "seed": 1987, "palette": PAL,
     "references": [
         {"rom": {"code": "0x271a", "large": True, "rows": 3, "cols": 2},
          "usage": "his diving attack's landing as the body, rows 16-111, with its one real arm driven forward at waist height; its upright head and neck taken away (x36-63, y18-46, sparing the near shoulder); its near fist and the end of the wristband (x74-93, y62-79) laid again 2 pixels forward and 6 higher, behind everything, as the far fist peeking past it; the mask covers only the back of the neck between his shoulders and head (x37-49, y30-47 of the pose), sparing the head and the near arm",
          "note": SRC.format("bull-horn", "96x96") + " His legs below row 111 are the pose's own, laid back after."},
         {"rom": {"code": "0x28a0", "large": True, "rows": 2, "cols": 2},
          "usage": "his headbutt lunge's lowered head (x79-95, y6-27: crown forward, face down), every pixel, drawn in front of everything at (48, 25) of the pose, resting on the near arm's top line"},
     ],
     "prompt": ("1987 Capcom arcade fighter sprite, flat simple shading, dark outline, exactly the colours and the drawing of the rest of the sprite. "
                "Birdie the punk brawler charging head-first like a bull, his bald head with its blond mohawk lowered and thrust forward: the back of his "
                "thick neck runs cleanly from the top of his bare muscular shoulders into the back of his head. One body, no gaps."),
     "candidates": 1,
     "kept": [{"candidate": 0, "pose": "bull_horn", "crc32": HORN, "touch_ups": TU["bull_horn"]}], "note": NOTE},
]
with open(OUT, 'w', newline='\n') as f:
    json.dump(L, f, indent=2)
    f.write("\n")
