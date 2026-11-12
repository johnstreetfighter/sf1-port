"""Append AP16 round two's PixelLab calls to art/generations/mike.json (run from the ap16 worktree root)."""
import json
P = ["#000000", "#664400", "#775500", "#996655", "#bb8877", "#cc9988", "#bbbbbb", "#777777",
     "#550000", "#880000", "#bb0000", "#0099bb", "#007799", "#005577", "#004455"]
def rom(code, large, rows, cols): return {"code": code, "large": large, "rows": rows, "cols": cols}
RN = "Drawn from its pens by sf-art reference, transparent only at pen 15, cropped to the figure."
JAB = rom("0x1470", True, 3, 2); HOOK = rom("0x1560", True, 3, 2); FT = rom("0x14e7", True, 3, 2)
WIN = rom("0x166c", True, 4, 1); STEP = rom("0x1406", False, 6, 2); STANCE = rom("0x1400", False, 6, 2)
def A(t): return f"2026-11-07T{t}Z"
def call(at, move, tool, job, cost, canvas, seed, refs, prompt, candidates, kept=(), note=""):
    return {"at": A(at), "move": move, "tool": f"PixelLab {tool}", "job": job, "cost": cost, "canvas": canvas, "seed": seed,
            "palette": P, "references": refs, "prompt": prompt, "candidates": candidates, "kept": list(kept), "note": note}
def kept(i, pose, crc, touch, note=""): return {"candidate": i, "pose": pose, "crc32": crc, "touch_ups": touch, "note": note}
WAIST = ("1987 Capcom arcade pixel art, flat simple shading in clean bands, dark brown outline, exactly the colours and the drawing "
         "of the rest of the sprite, no stray pixels. ")
PEAK = ("1987 Capcom arcade pixel art, flat simple shading in clean bands, dark brown outline, exactly the colours, size and drawing "
        "of the rest of the sprite, no stray pixels. ")
CALLS = [
    call("02:10:00", "jumping punch", "inpaint_image_pro_flash", "c27d80df-2dc6-4256-9ccb-863abc934ae3", 5, [96, 84], 1987,
         [{"rom": JAB, "usage": "the picture inpainted: his top-of-jump tuck's trousers, crotch and shoes (0x1608, its jeans from row 30 and everything from row 44), untouched, with his jab's upper body (0x1470, rows 9-53, its own jeans left off) set so its belt sits on the tuck's waist, on a 96x84 canvas; the mask covers only the shirt hem and belt band between them (x17-53, y40-48), never a tuck trouser pixel",
           "note": "inpaint-source-jump-jp-2-96x84.png and inpaint-mask-jump-jp-2.png in the pack's candidates; the source is all his ROM pixels."}],
         WAIST + "Mike the boxer's waist: the bottom hem of his red T-shirt, tucked into his brown belt with its small white buckle, the belt lying straight across the top of his blue jeans just below it. Only the shirt hem and the belt; the jeans below stay exactly as drawn.",
         1, [kept(0, "jump_punch", "770e222a", ["outside the mask every pixel is the source's: all 1,293 of the tuck's trouser and shoe pixels are its ROM pixels, checked; inside, the hem and belt inpainted, taken to his nearest pens; 1 single pixel inside the mask set to the pen around it"])],
         "Round two (the maintainer, 2026-11-07: the first round's waist join was wrong; keep the tuck's own trousers and crotch). Kept: the hem tucked into a belt with its buckle, sitting on the tuck's jeans."),
    call("02:10:30", "jumping hook", "inpaint_image_pro_flash", "fcadfa61-f97e-4411-9e51-eaf17ab2e6fa", 5, [96, 84], 1987,
         [{"rom": HOOK, "usage": "the picture inpainted: his top-of-jump tuck's trousers, crotch and shoes (0x1608, its jeans from row 30 and everything from row 44), untouched, with his hook's turned upper body (0x1560, rows 13-55, its own jeans left off) set so its belt sits on the tuck's waist, on a 96x84 canvas; the mask covers only the shirt hem and belt band between them (x17-51, y41-49), never a tuck trouser pixel",
           "note": "inpaint-source-jump-jh-2-96x84.png and inpaint-mask-jump-jh-2.png in the pack's candidates; the source is all his ROM pixels."}],
         WAIST + "Mike the boxer's waist, his back turned to us: the bottom hem of his red T-shirt, tucked into his brown belt, the belt lying straight across the small of his back along the top of his blue jeans just below it. Only the shirt hem and the belt; the jeans below stay exactly as drawn.",
         1, [kept(0, "jump_hook", "069ac7f1", ["outside the mask every pixel is the source's: all 1,293 of the tuck's trouser and shoe pixels are its ROM pixels, checked; inside, the hem and belt inpainted, taken to his nearest pens; 5 single pixels inside the mask set to the pen around them", "2 grey pixels in the belt (x40, y43-44) set to the belt's brown"])],
         "Round two (the maintainer, 2026-11-07: the first round's trousers looked turned away from the player, and the crotch need not change). Kept: the hem and belt across his back, his legs the tuck's, facing the player."),
    call("02:12:00", "iron upper", "create_image_pro", "8f11d3ac-8dfa-4a98-b3fe-2a96c22b57b7", 10, [80, 144], 1987,
         [{"rom": FT, "usage": "pose base, identity and exact pixel scale: his turned back, turned head and his striking arm cocked up on the right, the wind-up this rises from, the size to draw him at", "note": RN},
          {"rom": WIN, "usage": "his own raised arm: how his arm, taped wrist and fist are drawn straight up above his head", "note": RN},
          {"rom": STEP, "usage": "his legs leaving the ground: heels up, toes pointing down", "note": RN},
          {"rom": STANCE, "usage": "the style image (style_copy outline, detail, shading): his 1987 outline, detail and shading", "note": RN}],
         "The same fighter as the references, drawn exactly in the style image's 1987 Capcom arcade style: flat simple shading in clean bands, dark brown outline, no soft gradients, no stray pixels. Mike: a big, heavily muscled American boxer with short black hair, brown skin, a short-sleeved red T-shirt, white tape round his wrists, bare fists, baggy blue jeans with a brown belt, white and red sneakers. Facing right, toward the opponent on the right. The top of a rising uppercut, leaping off the ground: his broad red back turned toward us exactly as in the pose base, his head turned to the right as in the pose base; his striking arm, on the right side of his body toward the opponent, carried on up from the pose base and driven straight up above his head, fully extended from the shoulder, the taped fist high above his head and slightly forward to the right; his other arm low and bent at his side. His body stretched upward, both feet off the ground, his heels raised and toes pointing down. Exactly the size of the pose base: his head 15 pixels wide, his back 45 pixels across, about 64 pixels wide and 136 pixels tall. No floor and no shadow under him.",
         4, note="None kept. All four turned his back with the arm straight up and his feet off the ground, but at about 88 to 93% of his size; the peak was inpainted on his ROM follow-through instead, to keep his scale exactly."),
    call("02:16:00", "iron upper", "inpaint_image_pro_flash", "a1974dcc-cff2-404a-91cc-52adc1383598", 6, [80, 136], 1987,
         [{"rom": FT, "usage": "the picture inpainted: his hook's follow-through (0x14e7) at canvas (-12, -4), its turned back, rear arm and turned head kept; the mask covers the striking arm's forearm and fist and the space above (x60 on, y0-32), the outer elbow (x68 on, y33-48), room above for the fist (x52 on, y0-23) and everything below his belt (y67 on)",
           "note": "inpaint-source-iron-peak-80x136.png and inpaint-mask-iron-peak.png in the pack's candidates."}],
         PEAK + "Mike the boxer seen from behind, his red T-shirted back to us, leaping up at the top of a rising uppercut toward the right. His striking arm, on the right of his body, rises out of his red short sleeve at the right shoulder and drives straight up, nearly fully extended, a thick brown muscular arm as wide as his other arm, the white-taped fist clenched high above his head and a little forward to the right. Below his belt, his baggy blue jeans seen from behind with back pockets, his legs hanging below him off the ground, knees slightly bent, heels raised and toes pointing down, white and red sneakers. One body, the same size as his back and head.",
         1, note="None kept: the arm came thin out of a leftover stub of red sleeve, and the jeans dark."),
    call("02:19:00", "iron upper", "inpaint_image_pro_flash", "e031b402-18f1-4eda-ae9b-5f82451a0738", 6, [80, 136], 2026,
         [{"rom": FT, "usage": "the picture inpainted: his hook's follow-through (0x14e7) at canvas (-12, -4), its turned back, rear arm and turned head kept; the mask covers the whole striking arm from the shoulder, its sleeve included (x58 on, y0-48), room above for the fist (x50 on, y0-23) and everything below his belt (y67 on)",
           "note": "inpaint-source-iron-peak-2-80x136.png and inpaint-mask-iron-peak-2.png in the pack's candidates."}],
         PEAK + "Mike the boxer seen from behind, his red T-shirted back to us, leaping up at the top of a rising uppercut toward the right. From his right shoulder, his short red T-shirt sleeve, and out of it his striking arm driven straight up, nearly fully extended, a thick brown muscular arm as thick as his shoulder and his other arm, shaded in the same skin tones, the white-taped fist clenched high above his head and a little forward to the right. Below his belt, his baggy jeans seen from behind in the same light blue as the jeans in the rest of the sprite, with back pockets, his legs hanging below him off the ground, knees slightly bent, heels raised and toes pointing down, white and red sneakers. One body, the same size as his back and head.",
         1, note="Its arm kept: it rises cleanly from the shoulder, as thick as his other arm, the fist high above his head. Its legs were short and its shoes brown, so everything below the belt was inpainted again on this picture (8278babb)."),
    call("02:22:00", "iron upper", "inpaint_image_pro_flash", "8278babb-3473-43be-91b5-9c1f42f64a89", 6, [80, 136], 7,
         [{"candidate": "e031b402-18f1-4eda-ae9b-5f82451a0738#0", "usage": "the picture inpainted: the previous call's peak, its ROM back and head and inpainted arm kept; the mask covers everything below his belt (y67 on)",
           "note": "inpaint-source-iron-peak-3-80x136.png and inpaint-mask-iron-peak-3.png in the pack's candidates."}],
         PEAK + "Mike the boxer seen from behind, leaping off the ground: below his belt, his baggy light blue jeans seen from behind, with two back pockets, as wide at the hips as his belt; two long legs hanging straight down below him, about 60 pixels long from belt to soles, knees only slightly bent, one foot a little behind the other, heels raised and toes pointing down; white sneakers with red stripes and grey soles, the white the same as his wrist tape. Nothing else.",
         1, [kept(0, "iron_peak", "01e83726", [
             "the inpainting's off-palette blues in the legs (708 pixels) taken to his jeans pen of the nearest lightness, not his grey (peak2.py)",
             "his ROM sneakers laid on the cuffs, the generated shoes cleared first: the left foot from his take-off step (0x1406, x3-14, y97-111) at (24, 101), the right (x25-40, y97-111) at (36, 113)",
             "his ROM turned head (0x14e7, rows 29-39) laid back exactly; 4 pixels at its right edge, inside the previous call's mask, had changed",
             "17 single pixels in the inpainted arm and legs set to the pen around them; the shirt's right edge (10 pixels, y36-51) given his black outline, two of them stray grey and white"])],
         "Kept: legs of his length hanging below him, heels up, at his scale above the belt, every pixel of his back, rear arm and head his ROM follow-through's."),
]
path = "art/generations/mike.json"
ledger = json.load(open(path, encoding="utf-8"))
for c in ledger["calls"]:
    for k in c.get("kept", []):
        if k["pose"] in ("jump_punch", "jump_hook") and k["crc32"] in ("62f30233", "b9fa5b29"):
            k["note"] = ("Round one, rejected by the maintainer (2026-11-07): the waist join was wrong; the inpainting redrew the top of the jeans and the crotch"
                         + (", so the trousers looked turned away from the player" if k["pose"] == "jump_hook" else "")
                         + ". Round two keeps the tuck's trousers untouched.")
ledger["calls"].extend(CALLS)
with open(path, "w", encoding="utf-8", newline="\n") as f:
    json.dump(ledger, f, indent=2, ensure_ascii=False)
    f.write("\n")
print("added", len(CALLS), "calls,", sum(c["cost"] for c in CALLS), "generations")
