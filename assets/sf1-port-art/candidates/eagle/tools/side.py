"""side.py out.png scale a.png b.png ...: bottom-aligned on his stage-ish green, real scale x s."""
import sys
from PIL import Image
s=int(sys.argv[2]); ims=[Image.open(f).convert('RGBA') for f in sys.argv[3:]]
ims=[i.crop(i.getbbox()) for i in ims]
w=sum(i.width for i in ims)+10*(len(ims)+1); h=max(i.height for i in ims)+10
o=Image.new('RGBA',(w,h),(90,110,90,255)); x=10
for i in ims: o.alpha_composite(i,(x,h-5-i.height)); x+=i.width+10
o.resize((w*s,h*s),Image.NEAREST).save(sys.argv[1])
