import rompose as r
from collections import Counter
from PIL import Image
def offpal(path):
    im=Image.open(path).convert('RGBA'); n=0
    for (R,G,B,A) in im.getdata():
        if A>=128 and (R,G,B) not in r.IDX: n+=1
    return n
def lone(c, region=None, rounds=2):
    """Set single pixels unlike every 4-neighbour to the 8-neighbourhood's most common pen; returns count."""
    fixed=0
    for _ in range(rounds):
        changes=[]
        for y in range(c.h):
            for x in range(c.w):
                if region and not region(x,y): continue
                v=c.p[y][x]
                n4=[c.at(x+dx,y+dy) for dx,dy in ((1,0),(-1,0),(0,1),(0,-1))]
                if v in n4: continue
                n8=[c.at(x+dx,y+dy) for dx in (-1,0,1) for dy in (-1,0,1) if dx or dy]
                pen,cnt=Counter(n8).most_common(1)[0]
                if cnt>=5 and pen!=v: changes.append((x,y,pen))
        for x,y,p in changes: c.p[y][x]=p
        fixed+=len(changes)
    return fixed
def islands(c, smallest=4):
    """Remove opaque groups (8-connected) smaller than `smallest` pixels."""
    seen=set(); removed=0
    for y in range(c.h):
        for x in range(c.w):
            if c.p[y][x]==r.T or (x,y) in seen: continue
            stack=[(x,y)]; grp=[]; seen.add((x,y))
            while stack:
                a,b=stack.pop(); grp.append((a,b))
                for dx in (-1,0,1):
                    for dy in (-1,0,1):
                        q=(a+dx,b+dy)
                        if q not in seen and 0<=q[0]<c.w and 0<=q[1]<c.h and c.p[q[1]][q[0]]!=r.T:
                            seen.add(q); stack.append(q)
            if len(grp)<smallest:
                for a,b in grp: c.p[b][a]=r.T
                removed+=len(grp)
    return removed
def diff(a,b):
    return sum(1 for y in range(a.h) for x in range(a.w) if a.p[y][x]!=b.p[y][x])
