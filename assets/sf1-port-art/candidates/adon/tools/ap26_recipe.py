"""Add Adon's air set and specials (AP26, #190) to his recipe (art/recipes/adon.json): poses and moves.

usage: ap26_recipe.py <recipe> <crc32s as pose=crc ...>   (AP26's entries are replaced, the rest kept)
"""
import json, sys
PATH = sys.argv[1]
CRC = dict(a.split('=') for a in sys.argv[2:])
G = lambda c, rows, cols: {"code": c, "large": True, "rows": rows, "cols": cols}
POSES = [
 {"name": "jump_top", "rom": G("0x3248", 2, 1), "anchor": [-32, -144],
  "note": "The top of every jump of his (streams 0x12de6, 0x12f66, 0x130e6) and of his flying kick: tucked, both knees drawn up, his fists on guard before his face. Shown 48 pixels up, a suggested height mid-jump; SF1+ moves him through his ROM arcs. His new jumping attacks start and end on it."},
 {"name": "jump_punch", "parts": [{"rom": G("0x3248", 2, 1), "crop": [0, 39, 64, 40], "at": [3, 41]}, {"rom": G("0x30c4", 2, 2), "crop": [3, 0, 93, 43]}],
  "anchor": [-35, -146],
  "note": "His jab's upper body (0x30c4, stream 0x132dc, rows 0-42), the front arm driven straight out at head height, on the shorts and tucked legs of the top of his jump (0x3248, its rows from 39), laid as his approved crouching punch lays it: the tuck sits 3 pixels right and 2 down so its belt meets the stance's, and the jab's belt is his stance's 3 pixels right, so the waist needs no join. Every pixel his ROM art; nothing generated. Drawn where the top of his jump draws the tuck."},
 {"name": "jump_punch_down", "pack": {"crc32": CRC["jump_punch_down"]}, "anchor": [-33, -145],
  "note": "The jumping punch's body (his jab's upper body on the tuck's legs) with the jab's arm taken away past the shoulder, and the front arm of his flying kick's dive (0x330a, stream 0x1348c: the arm under his jaw to the fist, driven down and forward) laid 24 pixels left and 4 up so it leaves the jab's shoulder and drives down to the fist before his knee. Only the shoulder join (the jab's shoulder cap and the dive arm's root) is inpainted, his own shaded arm as the guide; his head, chest, belt, shorts and tucked legs are the ROM pixels. By hand: one pale pixel outside the arm's lower outline at the shoulder removed."},
 {"name": "jump_kick", "pack": {"crc32": CRC["jump_kick"]}, "anchor": [-25, -143],
  "note": "The top of his jump (0x3248) with its rear leg kept exactly where it hangs (the wrap and foot behind, every ROM pixel in place) and only its front leg (the horizontal thigh, the calf folded under it, the front wrap and foot) taken away; the kick grows from the same front hip, the leg of his approved crouching kick (his low kick's leg, 0x31e4) laid as the shaded guide, kicked straight out at waist height. The rear shin the front leg hid is hinted with his own shaded calf (0x3244's hanging rear calf, turned 40 degrees so it runs down and back into the rear wrap). The whole hip and thigh and the rear shin above its wrap are inpainted; the inpainting kept the guides almost as they were (6 pixels changed). By hand: the rear knee's outline closed where it shows under the kicking thigh, and specks removed. Round one kept the tuck's front calf, wrap and foot as the tucked leg and took the rear wrap away (the maintainer: \"It looks like it switches legs; the rear leg (from jump_top) becomes the front leg in jump_kick.\"). Round three, by hand (the maintainer, round two: \"shin/calf area on rear leg looks weird or lumpy\"): only the rear shin, from under the kicking thigh to the top of its wrap (rows 55-63), redrawn as his stance's own front calf (0x3060, rows 66-79), taken down to the 9 visible rows along the leg and sheared a pixel a row so it runs down and back into the rear wrap, behind his shorts and the kicking thigh; everything else is round two's, pixel for pixel."},
 {"name": "jump_kick_high", "pack": {"crc32": CRC["jump_kick_high"]}, "anchor": [-24, -144],
  "note": "His roundhouse (0x318a, stream 0x13850: leaning back, fists on guard, the front leg swung up to head height) lifted into the air, its head on the tuck's: the standing leg below his shorts turned 45 degrees back about the hip so it trails down behind him, and the ground shadow left out. Only the hip and the trailing thigh's root under the shorts are inpainted, his own shaded leg as the guide; his head, arms, body, shorts and kicking leg are the ROM pixels."},
 {"name": "takeoff", "rom": G("0x3240", 2, 1), "anchor": [-32, -96],
  "note": "His jump's take-off and landing squat (0x3240, streams 0x12de6-0x130e6 and his flying kick), knees bent, fists at his chest."},
 {"name": "rise", "rom": G("0x3244", 2, 1), "anchor": [-32, -120],
  "note": "The pose his jumps rise and fall through (0x3244): upright, fists by his face, the front knee drawn up and the rear leg hanging. Shown 24 pixels off the ground."},
 {"name": "jk_dive", "rom": G("0x330a", 1, 2), "anchor": [-61, -148],
  "note": "His flying kick's first strike (stream 0x1348c): diving forward flat out, the front fist driven down and forward, the legs trailing; its head on the tuck's."},
 {"name": "jk_flip", "rom": G("0x32a4", 2, 1), "anchor": [-32, -136],
  "note": "His flying kick's somersault (stream 0x1348c): tucked and upside down, his feet over him. Shown 40 pixels off the ground."},
 {"name": "jk_kick", "rom": G("0x32a8", 1, 3), "anchor": [-64, -88],
  "note": "His flying kick's second strike (stream 0x1348c): laid back in the air, the front leg driven out feet first at the opponent. Shown 24 pixels off the ground."},
 {"name": "rj_knee", "pack": {"crc32": CRC["rj_knee"]}, "anchor": [-26, -152],
  "note": "The Rising Jaguar's knee: his rising pose (0x3244: fists by his face, the rear leg hanging) with the front thigh turned 60 degrees up about the hip, so the knee is driven up to his chest, and the folded calf, wrap and foot turned 40 degrees about the knee so they hang under it. The hip, thigh and knee are inpainted over his own turned leg as the guide; his head, fists, body, shorts and rear leg are the ROM pixels. By hand: the rear calf's edge the old wrap covered given his dark brown outline, the old thigh's hem left under his shorts' corner removed, and a purple speck on the wrap made grey. Shown 56 pixels off the ground."},
]
MOVES = [
 {"name": "jumping punch", "poses": ["jump_top", "jump_punch", "jump_top"], "timing": [6, 14, 8],
  "note": "Every strength, from the top of his jump: a straight punch out at head height. One new pose, as his jab strikes in one (0x30c4). His jumps and his flying kick (0x1348c, 0x1366e: his diagonal jumping kick, a dive and a flying feet-first kick) stay his ROM streams."},
 {"name": "jumping diagonal punch", "poses": ["jump_top", "jump_punch_down", "jump_top"], "timing": [6, 14, 8],
  "note": "Every strength, for forward and back jumps: the fist driven down and forward, as his flying kick's dive drives it. One new pose."},
 {"name": "jumping kick", "poses": ["jump_top", "jump_kick", "jump_top"], "timing": [6, 14, 8],
  "note": "Light and medium, from the top of his jump: the front leg kicked straight out at waist height, his fists on guard and the rear leg tucked under. One new pose, as his low kick strikes in one (0x31e4)."},
 {"name": "jumping heavy kick", "poses": ["jump_top", "jump_kick_high", "jump_top"], "timing": [6, 16, 8],
  "note": "Heavy, from the top of his jump: his roundhouse in the air, leaning back with the front leg swung up to head height and the other trailing down behind him. One new pose, as his roundhouse strikes in one (0x318a)."},
 {"name": "jaguar kick", "poses": ["takeoff", "rise", "jump_top", "jk_dive", "jk_flip", "jk_kick", "rise", "takeoff", "stance"], "timing": [2, 3, 3, 4, 4, 5, 3, 3, 10],
  "note": "Quarter-circle back and kick (AP2's stand-in, nothing new): his ROM flying kick (stream 0x1348c, and 0x1366e, the same poses) as the ROM plays it. It leaves the ground: its frames rise 12, 6 and 2 pixels before the dive and 16 more into it, so AP2's condition holds. He springs up, dives with the fist driven down, somersaults and drives his feet out, then lands. SF1+ carries him along the arc and re-times it. Every pose is his ROM art."},
 {"name": "rising jaguar", "poses": ["takeoff", "rise", "rj_knee", "rise", "takeoff", "stance"], "timing": [4, 4, 16, 6, 6, 10],
  "note": "Dragon punch and punch (AP2): a rising knee. He springs straight up out of his take-off squat, the front knee coming up through his rising pose (0x3244) and driven high to his chest with both fists by his face, then drops back through the rising pose and lands. One new pose, the knee, where AP2 proposed three: his take-off squat and rising pose, both his ROM art, are the other two. SF1+ moves him up and down."},
]


def inline(v):
    """A small object or list on one line, the recipes' style."""
    if isinstance(v, dict):
        return "{ " + ", ".join(f'"{k}": {inline(x)}' for k, x in v.items()) + " }"
    if isinstance(v, list) and any(isinstance(x, dict) for x in v):
        return "[" + ", ".join(inline(x) for x in v) + "]"
    return json.dumps(v, ensure_ascii=False)


def entry(o):
    items = list(o.items())
    return "    {\n" + ",\n".join(f'      "{k}": {inline(v)}' for k, v in items) + "\n    }"


d = json.load(open(PATH, encoding='utf-8'))
names = {p["name"] for p in POSES}
mnames = {m["name"] for m in MOVES}
d["poses"] = [p for p in d["poses"] if p["name"] not in names] + POSES
d["moves"] = [m for m in d["moves"] if m["name"] not in mnames] + MOVES
out = "{\n" + f'  "fighter": {json.dumps(d["fighter"])},\n'
if "still_to_come" in d: out += f'  "still_to_come": {json.dumps(d["still_to_come"])},\n'
out += '  "poses": [\n' + ",\n".join(entry(p) for p in d["poses"]) + "\n  ],\n"
out += '  "moves": [\n' + ",\n".join(entry(m) for m in d["moves"]) + "\n  ]\n}\n"
json.loads(out)
open(PATH, 'w', encoding='utf-8', newline='\n').write(out)
