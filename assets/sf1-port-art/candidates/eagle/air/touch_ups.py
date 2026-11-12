"""Round one's poses (AP24, #188), full canvases with what was done to each. usage: touch_ups.py <touched folder>
Writes <pose>.png and touch-ups.json, and origins.json: each canvas's (0, 0) relative to his centre and the ground
line, set so the tuck's legs fall where the top of his jump (0x2a40, drawn 4 back, 48 up) has them."""
import json, os, sys
sys.path.insert(0, '../tools')
from PIL import Image
import rompose as r, clean as k, air as a
OUT = sys.argv[1]; os.makedirs(OUT, exist_ok=True)
TU, ORG = {}, {}
TOP = (-36, -144)                    # the tuck grid's top-left on screen (jump_top's anchor)

def put(name, c, ox, oy, notes):
    c.save(f"{OUT}/{name}.png"); TU[name] = notes
    ORG[name] = [TOP[0] - a.TX - ox, TOP[1] - a.TY - oy]

put('jump_jab', a.jab_on_jump(), 0, 0, [
    "his stick jab's strike (0x2980, stream 0x1239e) down to its belt (its rows 0-50, 16 left and 7 up), its own ROM arm thrust out level at head height, on the tuck's legs (0x2a40 from its row 36, 5 right and 8 down: thighs, knees, hanging shins and feet; its stick ends above row 40 left out)",
    "the waist band (rows 40-51, x0-45) is the approved crouching sweep's (6735d064): the same jab body on the same thighs, its waist inpainted in AP23; no pixel is generated"])
put('jump_poke', a.on_jump(a.SWEEP, 96), 0, 0, [
    "the approved crouching sweep (6735d064) above row 52, and its arm and stick wherever they differ from the approved crouch below it, on the tuck's hanging shins and feet (0x2a40, 5 right and 8 down) in place of the crouch's shins, hems, shoes and shadow; the crouch's thighs and knees are the tuck's own pixels, so the legs join with no seam; no pixel is generated"])
put('jump_high', a.rise_on_jump(), 0, 28, [
    "the approved rising strike's upper body (295e0a1c: the jab's rows 0-50, its arm turned 35 degrees up, shoulder and upper arm inpainted in AP23) on the tuck's legs (0x2a40, 5 right and 8 down), with the approved crouching sweep's waist band (rows 40-51, x0-45), the same jab body on the same thighs; no pixel is generated"])

c = a.swing_canvas()
g = r.load('../27a45233-0.png')
m = Image.open('r1/inpaint-mask-jump-swing.png').convert('RGB')
inside = lambda x, y: 0 <= x < m.width and 0 <= y < m.height and m.getpixel((x, y)) == (255, 255, 255)
n = k.lone(g, region=inside)
for y in range(g.h):
    for x in range(96): c.p[y][x + a.SWING_CROP] = g.p[y][x]
put('jump_swing', c, a.SWING_OX, 0, [
    "inpainting's colours reduced to his nearest pens" + (f"; {n} single pixels inside the mask unlike all four neighbours set to the pens around them" if n else ""),
    f"the inpainting's 96-pixel window laid back at column {a.SWING_CROP} of the 130-pixel canvas, between the arms' and sticks' ROM pixels outside it"])
json.dump(TU, open(f"{OUT}/touch-ups.json", 'w'), indent=1)
json.dump(ORG, open(f"{OUT}/origins.json", 'w'), indent=1)
print(json.dumps(TU, indent=1)); print(ORG)
