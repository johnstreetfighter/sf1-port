import rompose as r, eagle as e, zoom
from PIL import Image
out=[]
for deg in (35, 60, -60):
    c = r.P(140, 150)
    body = e.jab_body(); arm = e.turned_arm(deg)
    c.paste(body, 0, 48); c.paste(arm, 0, 0)
    c.save(f'../look/arm{deg}.png')
