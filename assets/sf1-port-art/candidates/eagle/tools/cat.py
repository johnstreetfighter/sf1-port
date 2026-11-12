"""cat.py out.png a.png b.png ...: side by side."""
import sys
from PIL import Image
ims=[Image.open(f) for f in sys.argv[2:]]
w=sum(i.width for i in ims)+10*len(ims); h=max(i.height for i in ims)
o=Image.new('RGBA',(w,h)); x=0
for i in ims: o.paste(i,(x,0)); x+=i.width+10
o.save(sys.argv[1])
