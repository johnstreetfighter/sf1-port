# Art pipeline tickets (spec #164)

Published 2026-11-02 as sub-issues of #164, with native blocking edges.

## AP1: The extended sprite bank, empty (prefactor) (#165)

Blocked by: none

Overhaul sessions load the game's graphics with an **extended sprite bank** above the ROM's 14,336 sprite tiles (`0x3800`), empty for now. Nothing on screen changes. First, settle the two renderer questions ADR-0007 raises:
- Today a tile code past the ROM wraps around. Does the 1987 game ever draw a code at or past `0x3800`?
- How do new tile codes go through the renderer's tile-code swizzle?

- A sweep over the existing scenarios and full-game runs records every sprite tile code the 1987 game draws. It either shows none at or past `0x3800`, or documents each one and how the bank keeps it drawing as before.
- Overhaul sessions (front-end fights) get graphics with the bank; Faithful 1987 and the faithful cabinet never load it.
- Tile codes in the bank are placed so the renderer's swizzle maps them to the intended tiles, and a test proves it with a known pattern.
- Every existing comparison fixture passes byte for byte, and with the empty bank loaded every ROM pose draws exactly as before.
- The whole workspace suite passes.

## AP2: Describe every throw and special for approval (#166)

Blocked by: none

Before any generation is spent on them, write one line for each of these, and have the user approve the list:
- each fighter's ground throw (no 1987 fighter has one);
- each special with no Capcom original: Kiai Shot, Rolling Sobat, Dynamite Straight, Bison Rush, Iron Upper, and the strike after Geki's teleport;
- Mike's and Eagle's extra punch and stick moves, which their kick buttons start instead of kicks.

Also list which existing ROM streams stand in for named specials (the art inventory names candidates). The descriptions may draw on the M.U.G.E.N research notes for what a move does, never for how it was drawn (ADR-0006).

- A committed document lists, per fighter: his throw; each special, with its ROM stand-in or 'new'; and, for Mike and Eagle, the moves on their kick buttons.
- Each new move has a one-line description of what it looks like, from its name, inputs and the earlier batch's frame data.
- The user approves the list (recorded in a comment on this ticket), and the document records the approval.

## AP3: Recipes and the pose catalogue, with Ryu's get-up as the first move (#167)

Blocked by: #165

The art pipeline's first path end to end, using only free art:
- The **art-loading seam** rebuilds **recipe** poses from the player's ROM into the extended sprite bank at launch.
- The first recipe is Ryu's get-up: his knockdown fall played in reverse into his stance.
- Ryu's **pose catalogue** lists the move.
- The pose draws through the game's own pose code.

- A recipe format records a pose as the fighter's ROM tiles plus the transforms (mirror, 90° turn, crop, offset, piecing order). Recipes are committed; no pixels are.
- The art-loading seam takes the ROM set (and, later, the art pack). It returns the graphics with the bank filled and the pose catalogue, or the ROM graphics with a fallback reason.
- Ryu's catalogue lists the get-up's poses in a suggested order, each with its source, its place in the bank (base tile, rows, columns, entry size) and an anchor against his ROM stance.
- Each pose, placed through the engine's pose-grid code, draws through the real renderer exactly as expected, facing right and mirrored facing left.
- Rebuilding the recipes twice from the ROM gives identical tiles; every pose uses only Ryu's palette pens with pen 15 transparent, and fits 24 sprite entries.
- The game's log names what art it loaded.

## AP4: The review page (#168)

Blocked by: #167

A local HTML page per fighter, where the user judges his new moves at game scale and approves or rejects each pose. It is never published (grilling Q8).

- For a chosen fighter, the page shows every move in his pose catalogue. Each plays in a real fight frame rendered by the game, against a real stage and opponent, next to his own ROM art.
- Each pose can be approved or rejected. The decisions are saved in a committed review-decisions file, and a fighter counts as approved only when every pose is.
- A rejected free (recipe) move is marked to be generated instead; a pose rejected after three rounds is flagged for the user's decision.
- Shown working on Ryu's get-up from AP3.
- The page and the frames it shows stay on the user's machine.

## AP5: The art pack, with Sagat's crouch as the first generated pose (#169)

Blocked by: #167

The path for **generated** poses end to end. The tracer is Sagat's crouch, which his ROM art lacks. It follows the spike's method:
1. PixelLab Pro draws it, with Sagat's own ROM frames setting his look; another fighter's frames may guide the pose. It is held to his exact palette.
2. The real ROM head is pasted back if the face drifted, and seams are cleaned by hand.
3. It is packed into the local **art pack**.

- Each generated pose's references, prompt, canvas, seed, cost and checksum are recorded and committed; its pixels go only into the art pack.
- The reduction to the fighter's palette uses only his pens, with pen 15 transparent and hard alpha.
- The art pack sits beside the ROM set. The game looks for it in the same folders as the ROMs and checks it against its manifest.
- A missing, corrupt, mismatched or out-of-date pack falls back to the ROM graphics with the reason logged, in a test. A fighter whose approved set is incomplete keeps his ROM art.
- A private backup is created when the first generated pose is approved and updated after each approval.
- Generations spent are recorded per move; the project uses at most 90% of the month's PixelLab allowance.

## AP6: Ryu and Ken: throws, thrown sets and get-ups (#170)

Blocked by: #166, #168, #169

Ryu's and Ken's missing art: a ground throw, the being-thrown set, and a get-up for each. These are made free first from their own frames where possible, and generated where not.

Ken's poses are made from Ryu's approved ones recoloured into Ken's palette, with Ken's differences (hair, face) fixed by hand.

- Ryu's and Ken's throws follow the approved descriptions from AP2.
- Every new pose has as many poses per move as their own ROM moves of the same kind.
- Each passes the art-loading seam's checks and appears in their pose catalogues.
- The user approves both sets on the review page, recorded in the review decisions. Both fighters' new poses then load in overhaul sessions.

## AP7: Sagat: ground set (#171)

Blocked by: #166, #168, #169

Sagat's ground set: crouch, crouching block, crouching hit, three crouching normals, a ground throw, the being-thrown set, a get-up. **This is the pilot**: Sagat goes end to end before the other nine bosses.

- Every pose in this half for Sagat, as the art inventory counts it at Ryu's level of distinct animations, is made: free first from his own ROM frames, generated where not.
- Each new move has as many poses as Sagat's own ROM moves of the same kind; throws and specials follow AP2's approved descriptions, or the existing ROM stream that stands in for them.
- Sagat's own ROM frames set his look in every generated pose; drifted faces get his real ROM head back.
- Every pose passes the art-loading seam's checks (his palette pens, pen 15 transparent, at most 24 sprite entries, checksums) and is listed in his pose catalogue with its bank place and anchor.
- The user approves this half on the review page, recorded in the review decisions; a pose still rejected after three rounds goes back to the user to decide.
- When both halves (ground and air and specials) are approved, Sagat's new poses load in overhaul sessions; until then he stays on his ROM moves.

## AP8: Sagat: air set and specials (#172)

Blocked by: #171

Sagat's air set and specials: a jump arc (he has none), five jumping normals, low Tiger Shot (high Tiger Shot and Tiger Knee are his ROM art). With the ground set this completes the pilot.

- Every pose in this half for Sagat, as the art inventory counts it at Ryu's level of distinct animations, is made: free first from his own ROM frames, generated where not.
- Each new move has as many poses as Sagat's own ROM moves of the same kind; throws and specials follow AP2's approved descriptions, or the existing ROM stream that stands in for them.
- Sagat's own ROM frames set his look in every generated pose; drifted faces get his real ROM head back.
- Every pose passes the art-loading seam's checks (his palette pens, pen 15 transparent, at most 24 sprite entries, checksums) and is listed in his pose catalogue with its bank place and anchor.
- The user approves this half on the review page, recorded in the review decisions; a pose still rejected after three rounds goes back to the user to decide.
- When both halves (air and specials and ground) are approved, Sagat's new poses load in overhaul sessions; until then he stays on his ROM moves.
- The pilot's real generation rate is reported (generations per approved pose and per move), with a revised timeline for the other nine bosses, before any of them begins.

## AP9: Retsu: ground set (#173)

Blocked by: #172

Retsu's ground set: two crouching normals, a ground throw, the being-thrown set, a get-up.

- Every pose in this half for Retsu, as the art inventory counts it at Ryu's level of distinct animations, is made: free first from his own ROM frames, generated where not.
- Each new move has as many poses as Retsu's own ROM moves of the same kind; throws and specials follow AP2's approved descriptions, or the existing ROM stream that stands in for them.
- Retsu's own ROM frames set his look in every generated pose; drifted faces get his real ROM head back.
- Every pose passes the art-loading seam's checks (his palette pens, pen 15 transparent, at most 24 sprite entries, checksums) and is listed in his pose catalogue with its bank place and anchor.
- The user approves this half on the review page, recorded in the review decisions; a pose still rejected after three rounds goes back to the user to decide.
- When both halves (ground and air and specials) are approved, Retsu's new poses load in overhaul sessions; until then he stays on his ROM moves.

## AP10: Retsu: air set and specials (#174)

Blocked by: #172

Retsu's air set and specials: four jumping normals, Kiai Shot.

- Every pose in this half for Retsu, as the art inventory counts it at Ryu's level of distinct animations, is made: free first from his own ROM frames, generated where not.
- Each new move has as many poses as Retsu's own ROM moves of the same kind; throws and specials follow AP2's approved descriptions, or the existing ROM stream that stands in for them.
- Retsu's own ROM frames set his look in every generated pose; drifted faces get his real ROM head back.
- Every pose passes the art-loading seam's checks (his palette pens, pen 15 transparent, at most 24 sprite entries, checksums) and is listed in his pose catalogue with its bank place and anchor.
- The user approves this half on the review page, recorded in the review decisions; a pose still rejected after three rounds goes back to the user to decide.
- When both halves (air and specials and ground) are approved, Retsu's new poses load in overhaul sessions; until then he stays on his ROM moves.

## AP11: Geki: ground set (#175)

Blocked by: #172

Geki's ground set: a standing block and a crouching block (he has no blocks), two standing normals, two crouching normals, a ground throw, the being-thrown set, a get-up.

- Every pose in this half for Geki, as the art inventory counts it at Ryu's level of distinct animations, is made: free first from his own ROM frames, generated where not.
- Each new move has as many poses as Geki's own ROM moves of the same kind; throws and specials follow AP2's approved descriptions, or the existing ROM stream that stands in for them.
- Geki's own ROM frames set his look in every generated pose; drifted faces get his real ROM head back.
- Every pose passes the art-loading seam's checks (his palette pens, pen 15 transparent, at most 24 sprite entries, checksums) and is listed in his pose catalogue with its bank place and anchor.
- The user approves this half on the review page, recorded in the review decisions; a pose still rejected after three rounds goes back to the user to decide.
- When both halves (ground and air and specials) are approved, Geki's new poses load in overhaul sessions; until then he stays on his ROM moves.

## AP12: Geki: air set and specials (#176)

Blocked by: #172

Geki's air set and specials: four jumping normals, Teleport Strike's strike (Shuriken and the teleport are his ROM art).

- Every pose in this half for Geki, as the art inventory counts it at Ryu's level of distinct animations, is made: free first from his own ROM frames, generated where not.
- Each new move has as many poses as Geki's own ROM moves of the same kind; throws and specials follow AP2's approved descriptions, or the existing ROM stream that stands in for them.
- Geki's own ROM frames set his look in every generated pose; drifted faces get his real ROM head back.
- Every pose passes the art-loading seam's checks (his palette pens, pen 15 transparent, at most 24 sprite entries, checksums) and is listed in his pose catalogue with its bank place and anchor.
- The user approves this half on the review page, recorded in the review decisions; a pose still rejected after three rounds goes back to the user to decide.
- When both halves (air and specials and ground) are approved, Geki's new poses load in overhaul sessions; until then he stays on his ROM moves.

## AP13: Joe: ground set (#177)

Blocked by: #172

Joe's ground set: crouch, crouching block, crouching hit, two crouching normals, a ground throw, the being-thrown set, a get-up.

- Every pose in this half for Joe, as the art inventory counts it at Ryu's level of distinct animations, is made: free first from his own ROM frames, generated where not.
- Each new move has as many poses as Joe's own ROM moves of the same kind; throws and specials follow AP2's approved descriptions, or the existing ROM stream that stands in for them.
- Joe's own ROM frames set his look in every generated pose; drifted faces get his real ROM head back.
- Every pose passes the art-loading seam's checks (his palette pens, pen 15 transparent, at most 24 sprite entries, checksums) and is listed in his pose catalogue with its bank place and anchor.
- The user approves this half on the review page, recorded in the review decisions; a pose still rejected after three rounds goes back to the user to decide.
- When both halves (ground and air and specials) are approved, Joe's new poses load in overhaul sessions; until then he stays on his ROM moves.

## AP14: Joe: air set and specials (#178)

Blocked by: #172

Joe's air set and specials: four jumping normals, Rolling Sobat, Dynamite Straight.

- Every pose in this half for Joe, as the art inventory counts it at Ryu's level of distinct animations, is made: free first from his own ROM frames, generated where not.
- Each new move has as many poses as Joe's own ROM moves of the same kind; throws and specials follow AP2's approved descriptions, or the existing ROM stream that stands in for them.
- Joe's own ROM frames set his look in every generated pose; drifted faces get his real ROM head back.
- Every pose passes the art-loading seam's checks (his palette pens, pen 15 transparent, at most 24 sprite entries, checksums) and is listed in his pose catalogue with its bank place and anchor.
- The user approves this half on the review page, recorded in the review decisions; a pose still rejected after three rounds goes back to the user to decide.
- When both halves (air and specials and ground) are approved, Joe's new poses load in overhaul sessions; until then he stays on his ROM moves.

## AP15: Mike: ground set (#179)

Blocked by: #172

Mike's ground set: crouching block, crouching hit, two standing normals and two crouching normals (punches: he gets no kicks), a ground throw, the being-thrown set, a get-up.

- Every pose in this half for Mike, as the art inventory counts it at Ryu's level of distinct animations, is made: free first from his own ROM frames, generated where not.
- Each new move has as many poses as Mike's own ROM moves of the same kind; throws and specials follow AP2's approved descriptions, or the existing ROM stream that stands in for them.
- Mike's own ROM frames set his look in every generated pose; drifted faces get his real ROM head back.
- Every pose passes the art-loading seam's checks (his palette pens, pen 15 transparent, at most 24 sprite entries, checksums) and is listed in his pose catalogue with its bank place and anchor.
- The user approves this half on the review page, recorded in the review decisions; a pose still rejected after three rounds goes back to the user to decide.
- When both halves (ground and air and specials) are approved, Mike's new poses load in overhaul sessions; until then he stays on his ROM moves.

## AP16: Mike: air set and specials (#180)

Blocked by: #172

Mike's air set and specials: five jumping normals (punches), Bison Rush, Iron Upper.

- Every pose in this half for Mike, as the art inventory counts it at Ryu's level of distinct animations, is made: free first from his own ROM frames, generated where not.
- Each new move has as many poses as Mike's own ROM moves of the same kind; throws and specials follow AP2's approved descriptions, or the existing ROM stream that stands in for them.
- Mike's own ROM frames set his look in every generated pose; drifted faces get his real ROM head back.
- Every pose passes the art-loading seam's checks (his palette pens, pen 15 transparent, at most 24 sprite entries, checksums) and is listed in his pose catalogue with its bank place and anchor.
- The user approves this half on the review page, recorded in the review decisions; a pose still rejected after three rounds goes back to the user to decide.
- When both halves (air and specials and ground) are approved, Mike's new poses load in overhaul sessions; until then he stays on his ROM moves.

## AP17: Lee: ground set (#181)

Blocked by: #172

Lee's ground set: crouch, crouching block, crouching hit, two crouching normals, a ground throw, the being-thrown set, a get-up.

- Every pose in this half for Lee, as the art inventory counts it at Ryu's level of distinct animations, is made: free first from his own ROM frames, generated where not.
- Each new move has as many poses as Lee's own ROM moves of the same kind; throws and specials follow AP2's approved descriptions, or the existing ROM stream that stands in for them.
- Lee's own ROM frames set his look in every generated pose; drifted faces get his real ROM head back.
- Every pose passes the art-loading seam's checks (his palette pens, pen 15 transparent, at most 24 sprite entries, checksums) and is listed in his pose catalogue with its bank place and anchor.
- The user approves this half on the review page, recorded in the review decisions; a pose still rejected after three rounds goes back to the user to decide.
- When both halves (ground and air and specials) are approved, Lee's new poses load in overhaul sessions; until then he stays on his ROM moves.

## AP18: Lee: air set and specials (#182)

Blocked by: #172

Lee's air set and specials: three jumping normals, Zesshouohken, Double Kick.

- Every pose in this half for Lee, as the art inventory counts it at Ryu's level of distinct animations, is made: free first from his own ROM frames, generated where not.
- Each new move has as many poses as Lee's own ROM moves of the same kind; throws and specials follow AP2's approved descriptions, or the existing ROM stream that stands in for them.
- Lee's own ROM frames set his look in every generated pose; drifted faces get his real ROM head back.
- Every pose passes the art-loading seam's checks (his palette pens, pen 15 transparent, at most 24 sprite entries, checksums) and is listed in his pose catalogue with its bank place and anchor.
- The user approves this half on the review page, recorded in the review decisions; a pose still rejected after three rounds goes back to the user to decide.
- When both halves (air and specials and ground) are approved, Lee's new poses load in overhaul sessions; until then he stays on his ROM moves.

## AP19: Gen: ground set (#183)

Blocked by: #172

Gen's ground set: crouch, crouching block, crouching hit, one standing normal, two crouching normals, a ground throw, the being-thrown set, a get-up.

- Every pose in this half for Gen, as the art inventory counts it at Ryu's level of distinct animations, is made: free first from his own ROM frames, generated where not.
- Each new move has as many poses as Gen's own ROM moves of the same kind; throws and specials follow AP2's approved descriptions, or the existing ROM stream that stands in for them.
- Gen's own ROM frames set his look in every generated pose; drifted faces get his real ROM head back.
- Every pose passes the art-loading seam's checks (his palette pens, pen 15 transparent, at most 24 sprite entries, checksums) and is listed in his pose catalogue with its bank place and anchor.
- The user approves this half on the review page, recorded in the review decisions; a pose still rejected after three rounds goes back to the user to decide.
- When both halves (ground and air and specials) are approved, Gen's new poses load in overhaul sessions; until then he stays on his ROM moves.

## AP20: Gen: air set and specials (#184)

Blocked by: #172

Gen's air set and specials: three jumping normals, Hyakurenko, Gekiro.

- Every pose in this half for Gen, as the art inventory counts it at Ryu's level of distinct animations, is made: free first from his own ROM frames, generated where not.
- Each new move has as many poses as Gen's own ROM moves of the same kind; throws and specials follow AP2's approved descriptions, or the existing ROM stream that stands in for them.
- Gen's own ROM frames set his look in every generated pose; drifted faces get his real ROM head back.
- Every pose passes the art-loading seam's checks (his palette pens, pen 15 transparent, at most 24 sprite entries, checksums) and is listed in his pose catalogue with its bank place and anchor.
- The user approves this half on the review page, recorded in the review decisions; a pose still rejected after three rounds goes back to the user to decide.
- When both halves (air and specials and ground) are approved, Gen's new poses load in overhaul sessions; until then he stays on his ROM moves.

## AP21: Birdie: ground set (#185)

Blocked by: #172

Birdie's ground set: crouch, crouching block, crouching hit, one standing normal, three crouching normals, a ground throw, the being-thrown set, a get-up.

- Every pose in this half for Birdie, as the art inventory counts it at Ryu's level of distinct animations, is made: free first from his own ROM frames, generated where not.
- Each new move has as many poses as Birdie's own ROM moves of the same kind; throws and specials follow AP2's approved descriptions, or the existing ROM stream that stands in for them.
- Birdie's own ROM frames set his look in every generated pose; drifted faces get his real ROM head back.
- Every pose passes the art-loading seam's checks (his palette pens, pen 15 transparent, at most 24 sprite entries, checksums) and is listed in his pose catalogue with its bank place and anchor.
- The user approves this half on the review page, recorded in the review decisions; a pose still rejected after three rounds goes back to the user to decide.
- When both halves (ground and air and specials) are approved, Birdie's new poses load in overhaul sessions; until then he stays on his ROM moves.

## AP22: Birdie: air set and specials (#186)

Blocked by: #172

Birdie's air set and specials: four jumping normals, Bull Head, Bull Horn.

- Every pose in this half for Birdie, as the art inventory counts it at Ryu's level of distinct animations, is made: free first from his own ROM frames, generated where not.
- Each new move has as many poses as Birdie's own ROM moves of the same kind; throws and specials follow AP2's approved descriptions, or the existing ROM stream that stands in for them.
- Birdie's own ROM frames set his look in every generated pose; drifted faces get his real ROM head back.
- Every pose passes the art-loading seam's checks (his palette pens, pen 15 transparent, at most 24 sprite entries, checksums) and is listed in his pose catalogue with its bank place and anchor.
- The user approves this half on the review page, recorded in the review decisions; a pose still rejected after three rounds goes back to the user to decide.
- When both halves (air and specials and ground) are approved, Birdie's new poses load in overhaul sessions; until then he stays on his ROM moves.

## AP23: Eagle: ground set (#187)

Blocked by: #172

Eagle's ground set: crouch, crouching block, crouching hit, two standing normals and two crouching normals (stick moves: he gets no kicks), a ground throw, the being-thrown set, a get-up.

- Every pose in this half for Eagle, as the art inventory counts it at Ryu's level of distinct animations, is made: free first from his own ROM frames, generated where not.
- Each new move has as many poses as Eagle's own ROM moves of the same kind; throws and specials follow AP2's approved descriptions, or the existing ROM stream that stands in for them.
- Eagle's own ROM frames set his look in every generated pose; drifted faces get his real ROM head back.
- Every pose passes the art-loading seam's checks (his palette pens, pen 15 transparent, at most 24 sprite entries, checksums) and is listed in his pose catalogue with its bank place and anchor.
- The user approves this half on the review page, recorded in the review decisions; a pose still rejected after three rounds goes back to the user to decide.
- When both halves (ground and air and specials) are approved, Eagle's new poses load in overhaul sessions; until then he stays on his ROM moves.

## AP24: Eagle: air set and specials (#188)

Blocked by: #172

Eagle's air set and specials: four jumping normals (stick moves), Manchester Black, St. Andrew's Green.

- Every pose in this half for Eagle, as the art inventory counts it at Ryu's level of distinct animations, is made: free first from his own ROM frames, generated where not.
- Each new move has as many poses as Eagle's own ROM moves of the same kind; throws and specials follow AP2's approved descriptions, or the existing ROM stream that stands in for them.
- Eagle's own ROM frames set his look in every generated pose; drifted faces get his real ROM head back.
- Every pose passes the art-loading seam's checks (his palette pens, pen 15 transparent, at most 24 sprite entries, checksums) and is listed in his pose catalogue with its bank place and anchor.
- The user approves this half on the review page, recorded in the review decisions; a pose still rejected after three rounds goes back to the user to decide.
- When both halves (air and specials and ground) are approved, Eagle's new poses load in overhaul sessions; until then he stays on his ROM moves.

## AP25: Adon: ground set (#189)

Blocked by: #172

Adon's ground set: crouch, crouching block, crouching hit, three crouching normals, a ground throw, the being-thrown set, a get-up.

- Every pose in this half for Adon, as the art inventory counts it at Ryu's level of distinct animations, is made: free first from his own ROM frames, generated where not.
- Each new move has as many poses as Adon's own ROM moves of the same kind; throws and specials follow AP2's approved descriptions, or the existing ROM stream that stands in for them.
- Adon's own ROM frames set his look in every generated pose; drifted faces get his real ROM head back.
- Every pose passes the art-loading seam's checks (his palette pens, pen 15 transparent, at most 24 sprite entries, checksums) and is listed in his pose catalogue with its bank place and anchor.
- The user approves this half on the review page, recorded in the review decisions; a pose still rejected after three rounds goes back to the user to decide.
- When both halves (ground and air and specials) are approved, Adon's new poses load in overhaul sessions; until then he stays on his ROM moves.

## AP26: Adon: air set and specials (#190)

Blocked by: #172

Adon's air set and specials: four jumping normals, Jaguar Kick, Rising Jaguar.

- Every pose in this half for Adon, as the art inventory counts it at Ryu's level of distinct animations, is made: free first from his own ROM frames, generated where not.
- Each new move has as many poses as Adon's own ROM moves of the same kind; throws and specials follow AP2's approved descriptions, or the existing ROM stream that stands in for them.
- Adon's own ROM frames set his look in every generated pose; drifted faces get his real ROM head back.
- Every pose passes the art-loading seam's checks (his palette pens, pen 15 transparent, at most 24 sprite entries, checksums) and is listed in his pose catalogue with its bank place and anchor.
- The user approves this half on the review page, recorded in the review decisions; a pose still rejected after three rounds goes back to the user to decide.
- When both halves (air and specials and ground) are approved, Adon's new poses load in overhaul sessions; until then he stays on his ROM moves.

