"""Eagle's air set (AP24, #188): poses pieced on the hanging legs of the top of his jump (0x2a40).
Canvas coordinates are the crouch's (eagle.py): his stance's x, his stance's y minus 2, the tuck laid 5 right and
8 down. Here the canvas is taller, so the tuck's hanging shins and feet fit. Read-only use of the ROM set."""
import sys
sys.path.insert(0, '../tools')
import rompose as r, eagle as e
H = 84
TX, TY = 5, 8                       # the tuck's (0, 0) on the canvas
CROUCH = r.load('../touched/crouch.png')
SWEEP = r.load('../touched-r2/crouching_sweep.png')
JAB = e.JAB
BAR = r.grid(0x2b00, 1, 2, 3)       # his spinning sticks, arms spread (stream 0x124d8)
LEG_ROW = 52                        # from this canvas row down, the crouch's legs give way to the tuck's

def legs(w):
    """The tuck's legs alone, rows 36 down (its thighs, knees, hanging shins and feet), less its stick ends."""
    c = r.P(w, H)
    T = e.TUCK
    for y in range(36, T.h):
        for x in range(T.w):
            v = T.p[y][x]
            if v == r.T or (v in e.STICK and y < 40): continue
            if 0 <= y + TY < H: c.p[y + TY][x + TX] = v
    return c

def on_jump(P, w):
    """An approved crouch-family pose P (crouch canvas) with the crouch's shins, hems, shoes and shadow taken off,
    and the tuck's hanging shins and feet set under its knees. Where P differs from the crouch (an arm or stick
    laid over the legs) P is kept."""
    c = legs(w)
    L = c.copy()
    for y in range(P.h):
        for x in range(min(P.w, w)):
            v = P.p[y][x]
            if y < LEG_ROW:
                if v != r.T: c.p[y][x] = v
                elif L.p[y][x] != r.T and y < 44: c.p[y][x] = r.T
                continue
            cr = CROUCH.at(x, y)
            if v != r.T and v != cr: c.p[y][x] = v          # an arm or stick over the legs
    return c

def jab_on_jump(w=116):
    """His stick jab's strike (0x2980) down to its belt, its own ROM arm thrust out level at head height, on the
    jump's legs. The waist band where the jab's belt meets the tuck's trousers is the approved crouching sweep's
    (the same jab body on the same thighs, its waist inpainted in AP23); the rest of the sweep is not used."""
    c = legs(w)
    for y in range(40, LEG_ROW):
        for x in range(0, 46):
            v = SWEEP.at(x, y)
            if v != r.T: c.p[y][x] = v
    for y in range(0, 51):
        for x in range(JAB.w):
            v = JAB.p[y][x]
            X, Y = x + e.JAB_DX, y + e.JAB_DY - 2
            if v != r.T and 0 <= X < w and 0 <= Y < H: c.p[Y][X] = v
    return c

def bar_on_jump(w=112, ox=8):
    """His spinning sticks' arms-spread pose (0x2b00) down to its belt, both sticks out level, on the jump's legs:
    its belt (rows 35-37, x46-68) on his stance's (rows 44-46, x14-35), as the crouch sets his stance's belt on the
    tuck's thighs. `ox` columns are added on the left for the rear stick."""
    c = r.P(w, H)
    L = legs(w)
    c.paste(L, ox, 0)
    for y in range(0, 38):
        for x in range(BAR.w):
            v = BAR.p[y][x]
            X, Y = x - 32 + ox, y + 7
            if v != r.T and 0 <= X < w and 0 <= Y < H: c.p[Y][X] = v
    return c

def jab_body_on_jump(w, h=None):
    """jab_on_jump without its striking arm: the torso's front edge under the arm outlined (eagle.jab_body)."""
    h = h or H
    c = r.P(w, h)
    c.paste(legs(w), 0, 0)
    for y in range(40, LEG_ROW):
        for x in range(0, 46):
            v = SWEEP.at(x, y)
            if v != r.T: c.p[y][x] = v
    B = e.jab_body(True)
    for y in range(0, 51):
        for x in range(B.w):
            v = B.p[y][x]
            X, Y = x + e.JAB_DX, y + e.JAB_DY - 2
            if v != r.T and 0 <= X < w and 0 <= Y < h: c.p[Y][X] = v
    return c

def poke_source(deg, w=96, h=None):
    """The jab's upper body on the jump's legs and its full upper arm turned deg degrees down, set 3 back; returns
    (Src, arm offset)."""
    from src import Src
    h = h or H
    s = Src(w, h)
    s.lay(jab_body_on_jump(w, h), 0, 0)
    arm = e.turned_arm_full(-deg)
    dx, dy = e.JAB_DX, -e.UP + e.JAB_DY - 2
    s.lay(arm, dx, dy)
    s.mask = {(x, y) for x, y in e.arm_mask(arm, dx, dy) if not (x <= 58 + e.JAB_DX and y <= 26 + e.JAB_DY - 2)}
    s.mask = {(x, y) for x, y in s.mask if 0 <= x < w and 0 <= y < h}
    return s

RISE = r.load('../touched-r2/rising_strike.png')   # the approved rising strike (AP23), 104x128, the jab 32 rows down
RISE_TOP = 32

def rise_on_jump(w=104, top=28):
    """The approved rising strike's upper body (the jab's rows above 51, its arm turned 35 degrees up, shoulder and
    upper arm inpainted in AP23) on the jump's legs, with the sweep's approved waist band; `top` rows added above."""
    c = r.P(w, H + top)
    c.paste(legs(w), 0, top)
    for y in range(40, LEG_ROW):
        for x in range(0, 46):
            v = SWEEP.at(x, y)
            if v != r.T: c.p[y + top][x] = v
    for y in range(0, RISE_TOP + 51):
        for x in range(RISE.w):
            v = RISE.p[y][x]
            X, Y = x + e.JAB_DX, y - RISE_TOP + e.JAB_DY - 2 + top
            if v != r.T and 0 <= X < w and 0 <= Y < H + top: c.p[Y][X] = v
    return c

SWING_OX, SWING_W = 34, 130
SWING_CROP = 18          # the inpainting's 96-pixel window starts at this column

def swing_canvas():
    """His spinning sticks' arms-spread pose (0x2b00) rows 0-37 (down to his belt and the top of its trousers) on
    the tuck's legs from its row 38 (below its own belt and stick ends); 2b00's belt on his stance's, as the crouch
    sets his stance's belt on the tuck's thighs."""
    c = r.P(SWING_W, H)
    T = e.TUCK
    for y in range(38, T.h):
        for x in range(T.w):
            v = T.p[y][x]
            if v != r.T: c.p[y + TY][x + TX + SWING_OX] = v
    for y in range(0, 38):
        for x in range(BAR.w):
            v = BAR.p[y][x]
            X = x - 32 + SWING_OX
            if v != r.T and 0 <= X < SWING_W: c.p[y + 7][X] = v
    return c
