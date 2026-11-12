"""Adon's crouching punch (AP25, #189), round three, by hand.

usage: make_round3.py <touched2 folder> <touched3 folder>

the maintainer, round two: "the top of his leg's outline seems to disappear compared to the crouch." The jab's upper body
(0x30c4) was laid over the crouch's rows 0-42, and its rows 40-42 are transparent or the jab's own belt end where the
crouch's front thigh starts. Where the jab's torso and arm do not pass (rows 40-42, from x31 right), the approved
crouch's pixels are put back: the end of the belt and the thigh's top outline and highlight. Nothing else changes,
and nothing is generated.
"""
import os, sys
import rompose as r
T2, T3 = sys.argv[1], sys.argv[2]
os.makedirs(T3, exist_ok=True)
C = r.load(f"{T2}/crouch.png")
P = r.load(f"{T2}/crouching_punch.png")
n = 0
for y in range(40, 43):
    for x in range(31, C.w):
        if C.p[y][x] != r.T and P.p[y][x] != C.p[y][x]:
            P.p[y][x] = C.p[y][x]; n += 1
P.save(f"{T3}/crouching_punch.png")
print(f"{n} pixels of the crouch's belt end and front thigh's top outline put back (rows 40-42, x31 on)")
