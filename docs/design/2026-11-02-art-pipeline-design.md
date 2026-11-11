# Art pipeline: the frames SF1+ needs, made from the ROM's own art

- **Date:** 2026-11-02
- **Status:** approved by the user, 2026-11-02
- **Project:** 2 of the seven the user set on 2026-11-02 (front end; **art pipeline**; SF1+ on the real engine; era styles and gauges; endings, Dramatic Battle and World Tour; bonus challengers; netplay transport)
- **Starting map:** [`docs/reference/art-inventory.md`](../reference/art-inventory.md), which records what each fighter's ROM art holds and lacks
- **Context:** [spec #1](https://github.com/johnstreetfighter/sf1-port/issues/1), [ADR-0002](../adr/0002-two-tier-overhaul-architecture.md), [ADR-0003](../adr/0003-offline-mugen-extraction-and-pose-synthesis.md) (superseded in part, see below), [`docs/design/2026-11-02-overhaul-front-end-design.md`](2026-11-02-overhaul-front-end-design.md)

## 1. Intent

The ten bosses of 1987 were CPU-only, and their art shows it. Each has 13 table moves against Ryu's 31, plus six to nine attacks the CPU picks from. Sagat has no crouch and no jumps at all. Nobody, Ryu included, has a throw or a get-up. The next project, SF1+ on the real engine, gives every fighter a complete moveset in the real 1987 fight. It cannot do that without frames to draw.

This project makes those frames. Every new frame must pass for 1987 Capcom art standing next to the originals. That means the 384×224 screen, the fighter's own 15 colours plus transparent, and 16×16 tiles, drawn by the game's own sprite code, in overhaul modes only.

It is a pipeline, not a one-off drawing session. Later projects (era costumes, endings, win screens, bonus challengers) will feed it new pose lists and get frames the same way.

## 2. Goals, non-goals, anti-goals

### Goals

1. **Every pose SF1+ needs, for every fighter, at Ryu's level of art.** "Ryu's level" has two parts:
   - **The number of distinct animations Ryu has.** Ryu himself shares art across button strengths (4 standing normal animations cover his 6 buttons), so the bosses get no more than he has.
   - **Smoothness: as many poses as the fighter's own ROM moves of the same kind.** A new kick has as many poses as his existing kicks, not the inventory's 2-pose minimum.

   The inventory's figure at that minimum is about 254 new poses before specials; at the fighter's own smoothness it is likely 1.5 to 2 times that.
2. **Pixels from the ROM only.** Each new frame is either the fighter's own ROM art reused or recombined, or generated and held to his exact palette. When it is generated, his own ROM frames set his look, and another fighter's ROM frames may guide the pose (decision 5).
3. **The user's eye is the final gate.** Nothing reaches the game unapproved.
4. **The game draws new frames with its own sprite code.** They get the original behaviour (mirroring, draw order, palette effects) for free.
5. **No Capcom-derived pixels in the public repository.**

### Non-goals

- **Animation, timing, hitboxes and frame data.** This project delivers poses and suggested frame orders. The SF1+ project builds the move streams, hitboxes and timing that use them.
- **Endings, era costumes, win-screen art, lose poses, a separate diagonal-jump somersault, "command heavy" moves, effects and projectiles.** These are for the projects that need them. They will use this pipeline.
- **New sound.**
- **Any change to Faithful 1987.** It never loads the new art.

### Anti-goals (failure even if it works)

- **Art that reads as AI-made:** soft gradients, stray colours, mushy outlines, faces that drift. The spike's beard is the cautionary example: candidate 4 gave Sagat a beard he never had.
- **Identity drift:** a boss who looks like a different person between his ROM moves and his new ones.
- **Frames from outside the ROM:** fan M.U.G.E.N sheets, later Capcom games' sprites, hand-traced references.
- **A pack the game depends on to run.** Without it, the game must still work, with the bosses on their ROM moves.
- **Spending past the PixelLab plan's monthly allowance** without the user saying so.

## 3. Constraints

### The sprite hardware and the engine

Read from `crates/sf-video/src/render.rs` and `crates/sf-engine/src/fighter.rs`:

- **A pose is a rectangle of tiles** from the sprite tile sheet. The sheet is 16 tiles wide. The pose header gives a base tile code, a row count (`+0xc`) and a column count (`+0xd`). The attribute selects 16-pixel entries, or 32-pixel entries (2×2 tiles, bit `0x400`). `write_grid` / `write_grid_flipped` lay it out.
- **At most 24 sprite entries per fighter.** The object region is `0x600` bytes. A grid that runs past it is the untranslated case `0x44506` (`park_from`) that the ladder report found on Joe.
- **One palette per pose.** The attribute's low 4 bits choose one of 16 sprite palettes, each 16 pens. **Pen 15 is transparent for sprites.** Note that `sf_video::asset_pack` uses pen 0 as transparent, so the pack format must follow the renderer, not `asset_pack`. Each fighter's colours come from his ROM palette. Sagat's body uses 14 colours.
- **The sprite ROM holds 14,336 tiles** (`0x3800`, `gfx3` is 1,835,008 bytes at 128 bytes a tile). Tile codes are 16-bit, so there is room above `0x3800` for new tiles.
- **Two renderer details bear on an extended bank:**
  - `invert(code)` swaps tile-code bits 3 and 4 in two of every four 8-tile groups.
  - `draw_sprite_tile` takes the code modulo the tile count. Today a code past the ROM wraps around; with a larger sheet it would not.
- **Tiles are 4-bit planar,** 128 bytes each, two 64-byte plane halves, as `decode_sprite_tile` reads them.

### Everything else

- **Determinism:** the art is data, and drawing it is the existing renderer, so snapshots, Replay Takeover and future rollback are unaffected.
- **PixelLab:** the user's Tier 1 plan is 2,000 generations a month, resetting on the 27th, with no paid credits. One `create_image_pro` call costs 10 generations for a 128×128 canvas and returns 4 candidates. PixelLab's `pixelart_workbench` pixel-editing tool is free.
- **The ROM set is local and gitignored.** So is anything made from it that is not rebuilt from it at launch.

## 4. What the spike showed (2026-11-02)

A throwaway spike built a crouch and a crouching kick for Sagat, who has neither. It spent 20 generations. Its pictures are in the session's scratchpad only.

- **Pieced from his own frames** (his stance torso on his Tiger Shot legs): seamless, because every pixel is his. But it makes only poses his parts cover. It gave a low stance, not a crouch.
- **Generated:** PixelLab Pro, given his stance and lunge frames as references, held to his 14 colours.
  - Every candidate stayed in palette and read as 1987 art at game scale. Seen in a real fight frame next to Gen, the frames passed.
  - One crouch of four was a real crouch.
  - Built on that crouch, three of four kick candidates came back on the identical body. That was enough for a five-frame move from one call: crouch, extending, kicked, extending, crouch.
  - **Weak spots:** about one candidate in four was unusable. One grew a beard; one forgot to kick. Faces drift: the eyepatch fades or hides behind his gloves. The muscle shading is slightly busier than the ROM's.

## 5. Approaches

**A. New tiles in an extended sprite bank (chosen).** Each approved frame is reduced to the fighter's palette, cut into a tile rectangle, and appended above the ROM's 14,336 tiles in overhaul modes. Reused and pieced frames are rebuilt from his ROM tiles at launch. The unchanged renderer and pose code draw everything; the SF1+ project's moves point at the new tiles. Each frame must fit the pose rules (a rectangle, at most 24 entries, one palette), and the pipeline checks this when it packs a frame.

**B. An overlay painted on top.** The engine plays a placeholder ROM pose while a separate layer paints the new image over it. There are no pose rules. But it means two drawing paths for one fighter, and mirroring, draw order and hit flashes would all be rebuilt to match. Every mismatch shows.

**C. Images only, format decided later.** This project stops at approved images. It is the cheapest now, but nothing checks frames against the engine's limits, so approved frames may need redoing.

**Why A (the user chose it on the recommendation):** it is how 1987 draws fighters, so new frames inherit the original behaviour. Its rules are enforced before a frame reaches review, not after.

## 6. Decided

### With the user (2026-11-02)

1. **ROM-only sources.** No fan-made M.U.G.E.N sprite sets, not even as pose references, unless an artist gives permission. This supersedes ADR-0003's plan to take poses from fan sets: [ADR-0006](../adr/0006-rom-only-art-sources.md). Fan-made move data (timings, hitboxes, move lists) may still be referenced.
2. **Scope: what SF1+ needs, nothing more.** Per fighter, from the inventory:
   - crouch, crouching block and crouching hit;
   - the missing standing, crouching and jumping normals, at Ryu's level of distinct animations (the inventory's per-fighter counts);
   - a jump arc where there is none (Sagat);
   - a standing block where there is none (Geki has no blocks);
   - a ground throw and the thrown reaction;
   - a get-up;
   - the SF1+ specials.

   Ryu and Ken need only a throw, a thrown reaction and a get-up.
3. **The specials:**
   - **Already in the README:**
     - Lee: Zesshouohken, Double Kick;
     - Gen: Hyakurenko, Gekiro;
     - Birdie: Bull Head, Bull Horn;
     - Eagle: Manchester Black, St. Andrew's Green;
     - Adon: Jaguar Kick, Rising Jaguar;
     - Sagat: high and low Tiger Shot, Tiger Knee.
   - **Taken from the earlier batch's `sf1plus.rs`:**
     - Retsu: Kiai Shot;
     - Geki: Shuriken, Teleport Strike, both already in his ROM art;
     - Joe: Rolling Sobat, Dynamite Straight;
     - Mike: Bison Rush, Iron Upper.
4. **Smoothness (user, on the builder check):** each new animation has as many poses as the fighter's own ROM moves of the same kind. The review page catches any that still feel stiff.
5. **Guides from other fighters (user, on the builder check):** a generated frame may take its *pose* from another fighter's ROM frames, such as Ryu's sweep guiding Sagat's. The fighter's own ROM frames always set his look: face, build, outfit and palette.
   - Ryu's approved new frames may be recoloured into Ken's palette for Ken, with Ken's own differences (hair, face) fixed by hand. In 1987 Ken's art is a near-copy of Ryu's.
6. **Mike and Eagle get no kicks (user, on the builder check).** Neither kicks anywhere in the ROM; Mike is a boxer and Eagle a stick fighter. Their kick buttons start extra punch or stick moves instead: an uppercut for Mike, a low stick sweep for Eagle, and so on. These count as their new standing, crouching and jumping normals. The README's line that "Mike's SF1+ kicks stay on LK, MK, and HK" is for the SF1+ project to reword.
7. **Free first, generation on rejection.**
   - Every move that can be made from the fighter's own frames is made that way first: reversed, mirrored, turned 90°, re-timed or pieced together.
   - A free version the user rejects goes to generation. Moves that cannot be pieced go straight to generation.
   - If the user's rejections show a pattern ("pieced throws look stiff"), that kind of move is generated from the start.
8. **Who decides.** The builder generates, chooses the best candidate for each frame and assembles the moves. The user approves one fighter at a time on a review page:
   - each move plays in a real fight frame next to the fighter's ROM art;
   - any single frame can be rejected for another try.
9. **Budget:** within the plan's monthly allowance, however long that takes.
   - At the 2-pose minimum, the estimate was 1,500 to 2,000 generations, with about a third of the poses free.
   - At the fighter's own smoothness (item 4) it is likely 2,500 to 4,000 generations: about two to three months of the plan, perhaps four.

   These are estimates. Sagat's pilot will give a real rate.
10. **Where the art lives:**
   - **The public repo** holds the pipeline's code, the pose list, the recipes for reused and pieced frames, each generated frame's settings and checksum, and the review decisions.
   - **A local art pack** beside the ROMs holds the generated frames.
   - **A private GitHub repository** backs the pack up. Generation is not repeatable, so a lost pack is about a month's work and the approvals.

### The builder's calls, open to the user

1. **The pack is looked up in the same folders as the ROM set** (beside the executable, the app-data folder, the settings file's folder), and is checked against its own manifest. If it is missing, broken, or out of date with the pose list, the bosses fall back to their ROM moves and the log says why. The front end's ROM-missing screen is the model; no screen is shown for a missing pack.
2. **Faces.** Where a generated face drifts, the fighter's real ROM head (or head pieces) is pasted back over it before review, and the seams are cleaned by hand with `pixelart_workbench`. Paid inpainting is the fallback.
3. **Specials are described before they are drawn.** Five specials have no Capcom original to follow: Kiai Shot, Rolling Sobat, Dynamite Straight, Bison Rush and Iron Upper. Teleport Strike's teleport is Geki's ROM art; the strike after it may need new frames. The builder writes a one-line description of each, from its name, inputs and the earlier batch's frame data, for the user to approve before any generation. The descriptions may draw on the M.U.G.E.N research notes for what a move does, never for how it was drawn (grilling, Q8).
4. **A reserve in the plan.** The project uses at most 90% of each month's PixelLab allowance, leaving the rest for other uses. On 2026-11-07 the maintainer lifted this to the whole allowance, so the last bosses' sets fit before the 27 November reset.
5. **Order of work.**
   1. Ryu, Ken and the throw/get-up set first, because they need the least and are mostly free.
   2. Then one boss end to end, before the other nine: Sagat, the largest gap. That proves the whole path, from the pose list to the pack, the renderer and review.
   3. Then the rest, in the order SF1+ will need them.

## 7. What the pipeline delivers to SF1+

A per-fighter **pose catalogue** (committed) listing each move by name with:

- its frames, in a suggested order;
- each frame's source: a ROM recipe, or generated, with the pack entry;
- its pose grid: base code in the extended bank, rows, columns, entry size;
- an anchor for each frame, so frames line up with the fighter's existing stance (the feet and body centre the ROM frames use).

Plus the art itself: rebuilt from the ROM at launch, and loaded from the local pack. SF1+ writes the move streams, timing and hitboxes against the catalogue.

## 8. Testing

- **Fidelity of drawing:** a packed pose drawn through `render_frame` (through the real pose grid, in both facings) equals the approved image pixel for pixel.
- **Faithful 1987 unchanged:** with no bank loaded, every existing comparison fixture passes byte for byte. An overhaul session with the bank loaded draws every ROM pose exactly as before. That second check covers the modulo-wrap and `invert` questions in section 3.
- **Recipes are deterministic:** rebuilding a recipe from the ROM twice gives identical tiles.
- **Pack checks:**
  - every frame uses only its fighter's palette pens, with pen 15 transparent;
  - every frame fits 24 entries;
  - the manifest's checksums match.

  A missing or corrupt pack falls back to the ROM moves, in a test.
- **The review page** is checked by the user's eye; that is its purpose.

## 9. Settled in the grilling (2026-11-02)

1. **PixelLab may train on the outputs.** Its help agent says Pro outputs may be used for training, with no opt-out, and it could not say how uploads are handled. The user accepted this and generation goes ahead as planned.
2. **Thrown poses:** one generic being-thrown set per fighter, used by every throw.
3. **Throws are described before they are drawn**, like the specials: a one-line description of each, approved by the user first.
4. **The backup:** keep the art pack in private storage and update the backup after each approval.
5. **ROM-only, art only:** [ADR-0006](../adr/0006-rom-only-art-sources.md) supersedes ADR-0003's use of fan sprite sets. Fan-made move data may still be used as a reference.
6. **The extended sprite bank** is [ADR-0007](../adr/0007-extended-sprite-bank.md).
7. **The glossary** gains *pose*, *recipe*, *art pack*, *pose catalogue* and *extended sprite bank* (`CONTEXT.md`, Art pipeline).
8. **The review page is a local HTML file**, never published as an Artifact. Descriptions of specials and throws may draw on the fan research notes for what a move does.
9. **Rollout per fighter.** A fighter's new poses reach the game as soon as his whole set is approved. Fighters not yet done stay on their ROM moves.
10. **The retry cap.** A pose still rejected after three rounds of generation and repair comes back to the user to decide: take the best attempt, simplify the move, or drop that pose.

11. **Compromises are recorded (user, 2026-11-02).** A move shipped short of what new art could do goes on [`docs/planning/art-upgrades.md`](../planning/art-upgrades.md). That includes a reused or pieced ROM stand-in approved as serviceable, and a generated pose accepted with known flaws. The list is the queue for new art generation once the game is complete with everything else. Ryu's get-up is its first entry.

## 10. Left to the builder

- **The pack's file format** and the pose catalogue's format.
- **Where the tool lives:** `tools/asset-gen` as the earlier design named it, or a new crate, and its commands.
- **How the extended bank is loaded:** for example, an extended `gfx3` in `Assets` for overhaul sessions. Also how it handles `invert` and the modulo wrap, after checking whether the 1987 game ever draws a tile code at or past `0x3800`.
- **The PixelLab recipes:** which references, prompts, canvas sizes and seeds, recorded per frame.
- **The reduction to the fighter's palette:** nearest colour with his exact pens, hard alpha, pen 15 transparent.
- **The review page's technology.** It must stay local or private, because it shows the generated art.
- **Anchors and alignment** of new frames against the ROM stance.
- **How the work is split into tickets.**

## 11. Settled while building

### AP1 (the extended sprite bank, #165)

1. **The 1987 game never draws past the ROM's tiles.** Over all 137 committed scenarios (874,344 frames from power-on), the highest sprite tile code in object RAM is `0x33bf`, and none is at or past `0x3800`. A bank above the ROM therefore changes nothing the game draws, and the modulo wrap never comes into play. `extended_sprite_bank.rs` runs this sweep by default; it takes about two seconds.
2. **The sprite region is split in two halves.** MAME's `sprite_layout` keeps planes 0 and 1 of every tile in the first half and planes 2 and 3 in the second, 64 bytes of each tile in each half. Appending tiles to the end would have moved the second half and broken every ROM tile. The bank's tiles go after the ROM's in *both* halves (`sf_video::sprite_bank`), and a test checks all 14,336 ROM tiles decode unchanged with a bank loaded.
3. **The tile-code swizzle.** The renderer reads tile `code` from slot `invert(code)`, which swaps bits 3 and 4 in two of every four 8-tile groups and maps each 32-tile group onto itself. The bank stores each tile in that slot. A test draws 32 small entries and one large entry through `render_frame` and checks every pixel; with the slot mapping removed, it fails.
4. **Which fights get the bank.** `GameSession::with_sprite_bank` gives fights under an overhaul preset the bank's graphics. The attract, the faithful cabinet and Faithful 1987 fights keep the ROM's own. The bank is still empty: the game binary loads it from AP3 on.

### AP3 (recipes and the pose catalogue, #167)

1. **Recipes are JSON**, one file per fighter in `art/recipes/`, committed and embedded in the game (`sf_host::art::RECIPES`) as the ROM manifest is. A pose is either one of the fighter's **ROM poses used as it is**, which needs no bank tiles, or **built**: parts of his ROM poses, each cropped, turned in quarter turns, mirrored and placed, drawn in order onto a canvas.
2. **Built poses go into the bank** as 16-pixel entries, or 32-pixel ones when 16-pixel entries would pass the 24-entry limit. Each pose starts on a fresh 16-tile row of the sheet. Recipes hold no pixels, and the same ROM always gives the same tiles (tested).
3. **The anchor** is the top-left of a pose's grid relative to the fighter's centre and the ground line (`0xe8`, where his stance and crouch both stand), facing right. The engine centres a grid on the fighter, so the SF1+ project turns an anchor into a frame record's `dy` and x-offset.
4. **Ryu's get-up needs no new tiles.** His knock-out played in reverse is lying (`0x0484`), the fall pose (`0x0480`) set on the ground (it reads as rising to his knees), crouch (`0x0120`) and stance (`0x0063`). So the bank's first **rebuilt** pose is the start of his being-thrown set: the fall pose turned a quarter clockwise. AP6 completes that set.
5. **The engine's pose-grid layout is one function**, `sf_engine::fighter::pose_grid_entries`. Both of the engine's grid writers use it, and the drawing checks call it, so "drawn through the engine's grid" means the engine's own code. The comparison fixtures and the coverage ledger (137 scenarios identical) confirm the refactor changed nothing.
6. **The seam** is `sf_video::art::load_art(rom, recipes)`, which returns the bank, the catalogues and each built pose's pens, or a reason. `sf_host::art::load` logs either `art: ryu (2 moves; 4 ROM poses, 1 rebuilt); the bank spans 44 tile slots`, or `art: keeping the ROM's own graphics: <reason>`. The game gives overhaul fights the bank. The span is 44 slots, not 64, because the swizzle folds the pose's fourth tile row into slots `0x3828..0x382b`.

### AP4 (the review page, #168)

1. **`sf-art review <fighter>`** serves the page on `127.0.0.1` only. A fresh machine with the fighter on P1 (against Ken for Ryu, Ryu for everyone else) is driven by the cabinet driver to round 1. The fighter's 24 object entries in that real frame are then replaced by each pose, laid out by `pose_grid_entries`, so the game's renderer draws it on the real stage, in his real palette, beside the opponent. The frame is rendered both facing right and mirrored. His own ROM art in the same frame sits on the left. Moves play at their recipe `timing` (game frames per pose).
2. **Recipes gained `timing`** per move: a suggestion for review and for SF1+, ten frames a pose by default.
3. **Verdicts** go to `art/review/<fighter>.json` (committed) through the page:
   - a rejected free pose is marked `generate`;
   - a rejected generated pose is marked `retry`, and `user-decides` at the third rejection;
   - the fighter is approved when every pose is.

   A new catalogue keeps old verdicts for poses whose origin is unchanged. The server answers each connection on its own thread, because a browser opens connections it does not use at once.
4. **The anchor check.** Drawing Ryu's catalogue stance at its anchor reproduces the real fight frame pixel for pixel, so anchors put poses exactly where the game would.
5. **Throw direction (user's question, 2026-11-02).** Poses face right, the fighter's own facing toward his opponent. In the over-the-top throws (shoulder throw, hip toss, circle throw, over-the-back throw, stick fling) the victim travels toward and over the thrower, head first. So `thrown_1`, head to the right, is the right way round. Throws that shove the victim away (Mike's, Gen's, Adon's) play the set mirrored. The user approved it on that reading.

### AP5 (the art pack, with Sagat's crouch, #169)

1. **The art pack** is a folder, `sf1-port-art`, beside the ROM set's folder (`roms\sf` and `roms\sf1-port-art`). The game looks beside the ROM set it loaded, then beside each other folder it looks for ROMs in, and takes the first that exists.
   - `manifest.json` (format 1) lists each pose file with its size and CRC-32, as the ROM manifest does.
   - Each pose is an indexed PNG, `<fighter>/<pose>.png`. A pixel's index is its pen. The palette holds his colours, so the file looks like him in any viewer, with entry 15 clear. The game reads only the pens.
   - Anything else in the folder is ignored: the raw candidates (`candidates/<fighter>/`), the touch-up script, and the backup's git files.
2. **A generated pose in a recipe** names its pack file by CRC-32 (`"pack": { "crc32": "…" }`). ROM parts may be drawn over it, as recipes draw them, for a ROM head over a drifted face.
3. **The game embeds the review files** (`sf_host::art::REVIEWS`) beside the recipes. A fighter's new poses load only when every pose in his moves is approved. Otherwise he keeps his ROM art, and the log says why:
   - his poses are not reviewed, or are not all approved;
   - the pack is missing or damaged (the pack's own reason is quoted);
   - the pack lacks his pose (out of date);
   - the pack holds a different file (not the approved one).

   The other fighters still load. A broken recipe or review file still keeps the ROM's graphics for everyone.
4. **A verdict holds for one file.** A generated pose's review entry records the CRC-32 it was judged on. A new file, such as a retry, is pending again, and keeps its rejections toward the three.
5. **The spike's palette had no black.** The spike cut Sagat's frames from a render whose backdrop was black. So his black (pen 0) showed as transparent, and the 14-colour palette and the references both lacked it. His eyepatch's rim, his strap and his ground shadow are black.
   - Generation drew the patch and strap in skin and greys, and used `#444444` and `#440044` where his outlines are brown.
   - From now on `sf-art palette <fighter>` gives his 15 opaque pens from palette RAM, and `sf-art reference` draws ROM poses from their pens, transparent only at pen 15.
6. **Touch-ups by hand** are listed in the ledger's `touch_ups` for each kept candidate. Every ROM pose of Sagat's carries a two-row black ground shadow, so generated poses get one too. The crouch's touch-ups came from the user's review rounds:
   - his eyepatch: his own oblong patch from the stance, partly behind his hand, with the strap in black;
   - the light streak between his forearms;
   - a second stripe across his shorts (they have only the side stripe);
   - stray grey and purple specks on his outlines.
7. **The generation ledger** is `art/generations/<fighter>.json`. Each call records its move, references (ROM grids or earlier candidates), prompt, palette, canvas, seed, cost and candidates, and which candidates became which poses, with the packed file's CRC-32.
   - A test checks every call is complete, and that each generated pose a recipe names is a kept candidate with the same CRC-32.
   - It also checks no month of the plan (from the 27th, UTC) passes 2,000 generations, the whole allowance (1,800, 90%, until the maintainer lifted it on 2026-11-07).
   - `sf-art budget` prints spend per move and per month.
   - The spike's two calls (20 generations) are recorded. AP5 packed the spike's crouch and spent no new generations.
8. **`sf-art pack <fighter> <pose> <picture.png>`** reduces a candidate to his pens and crops it:
   - hard alpha (opaque only from half alpha up);
   - the nearest of his opaque pens, never pen 15.

   It then writes the pack and its manifest, reads the pose back as the game will, and prints the recipe line: the CRC-32 and a first anchor, centred, with the lowest row on the ground. The crouch reduced exactly: 3,693 pixels, none approximated, no soft alpha.
9. **Sagat's set approved (user, 2026-11-02).** His stance and crouch were approved on the review page after two rejected rounds of repair. The pack was backed up privately. On this machine the game's own loader finds the pack beside the ROMs and loads the crouch into the bank: 4×6 16-pixel entries (the full 24), anchored at `[-31, -91]`.

### AP6 (Ryu and Ken: throws, thrown sets and get-ups, #170)

1. **Ryu's set, as AP2 describes it.** Of his new poses only the throw's turn and heave are generated; the rest are his own ROM art.
   - The grab is his Hadoken thrust (`0x042a`, stream 0x01362), placed as the Hadoken places it (x-offset 16).
   - His being-thrown set is seized (the first pose of his standing-hit reaction, `0x00c0`, stream 0x01b82), lifted and over the top (his fall pose `0x0480` turned a quarter and a half), then lying. The get-up is AP3's.
   - **Poses per move (section 6, decision 4).** Counted as the inventory counts them, with the stance a move ends on: the throw has 4 (grab, turn, heave, stance), as his MP and Hadoken do. The thrown set has 4, against his hit reactions' 3 to 4 and his knock-outs' 2. The get-up has 4.
2. **The generation canvas sets his size.** PixelLab draws the figure at about three quarters of the canvas. 128 pixels drew him too big and 96 too small; the user chose 108. It has no facing control, so poses are mirrored by hand.
3. **His own ROM art as the style image.** The first turn and heave read as later art: bulkier, with modern shading, a heavy black outline (73% of the heave's edge, against 8 to 25% in his ROM poses) and a flying belt end. The turn also faced the opponent, and both stood where the grab stands.
   - The retry gave PixelLab his ROM grab as `style_image`, with `style_copy` outline, detail and shading. All eight candidates came back lean and flat-shaded like his 1987 art, black on 8 to 12% of the edge.
   - Mirrored, the turn has his back to the opponent. Both step 20 pixels in toward him.
   - The user approved them on 2026-11-03, at the second try. The throw cost 110 generations (11 calls); the month from 2026-10-27 stands at 130 of 1,800.
   - The first prompts described him from memory (a red headband, bare feet). His ROM art has red hair, a white headband and red shoes. Prompts now describe a fighter from his ROM art.
4. **Tests and the private pack.** A fighter with a generated pose needs the pack, which the public repository never holds. `sf_video::art::rom_only(recipe)` drops a recipe's generated poses and the moves that use them, so the public tests build the rest. A local-only test loads the embedded recipes and reviews against the pack beside the ROMs, as the game does, and skips without it.
5. **Ken's ROM poses** were read from his own streams, not assumed. Each pose his set needs is at Ryu's code + 0x800 with the same grid: stance `0x0863`, crouch `0x0920`, fall `0x0c80`, lying `0x0c84`, the standing-hit pose `0x08c0` (stream 0x03c60), and the Hadoken thrust `0x0c2a` (stream 0x03440, x-offset 16). His knock-outs shift the lying pose 48 pixels back, as Ryu's do. So his recipe is Ryu's at his codes, with the same anchors.
6. **Ken's throw is Ryu's approved turn and heave, recoloured** (section 6, decision 5).
   - **The pens line up.** Across seven of their ROM poses, each of Ryu's gi pens lands on the same pen in Ken's art 50 to 82% of the time. His red hair and wristbands land on Ken's golds. So Ryu's pens drawn in Ken's colours give Ken's red gi and gold wristbands.
   - **What Ken's ROM art does differently is fixed by hand**, by a script kept in the pack (`candidates/ken/throw-from-ryu.py`), measured against his stance, grab and his back in the Tatsumaki (`0x0da0`):
     - no headband;
     - his hair outlined and streaked in brown, mostly his light gold, and over his ears; black only for his eyes and brows;
     - his hands bare, the gold band on the wrist only;
     - his flat feet are his grab's front foot, mirrored; the turn's rear heel, seen from behind, is shaded in skin; the heave's rear foot faces us, so it is his stance's rear foot, toes toward us.
   - **Three rounds.** The user rejected the first for his hands, hair and feet, and the second for the heave's rear foot (a heel where his toes should face us). The third was approved on 2026-11-03. No generations were spent.
7. **Derived poses in the ledger.** A pose made from another fighter's kept one is recorded under the ledger's `derived`: its file's CRC-32, the source file (fighter, pose, CRC-32) and its touch-ups. It costs nothing. The ledger check refuses one whose source no call kept, or with no touch-ups listed. Each rejected round stays in the record with the user's reasons.
8. **Both sets approved and loading.** Ryu's turn and heave were approved in chat on 2026-11-03. The page had not saved those two verdicts, so they were recorded there from the user's word. Ken's ten poses were approved on the page. The pack is backed up to the private repository (`64b38c3`, `638bb0d`). On this machine the game's own loader logs `ryu (3 moves; 6 ROM poses, 2 rebuilt, 2 generated), ken (3 moves; 6 ROM poses, 2 rebuilt, 2 generated), sagat (1 move; 1 ROM pose, 0 rebuilt, 1 generated); the bank spans 716 tile slots`, with no fighter kept on his ROM art.
9. **Compromises.** The grab (a reused Hadoken thrust), the thrown set (a turned fall pose) and Ken's get-up (his knock-out reversed) are ROM stand-ins. They are listed in [`docs/planning/art-upgrades.md`](../planning/art-upgrades.md) beside Ryu's get-up.

### AP7 (Sagat's ground set, #171)

1. **His ground set, and where each pose comes from.** Seventeen poses make nine moves.
   - **Free, from his ROM art:**
     - the throw, a tiger clinch (AP2): the grab is his Tiger Shot thrust (`0x2f06`), the knee his Tiger Knee pose (`0x2d0a`), the throw down his lunging punch's strike (`0x2d00`);
     - the thrown set: seized is his standing hit (`0x2e00`), and lifted and over the top are the recoil his knock-outs start with (`0x2e0c`), turned a quarter and a half;
     - lying (`0x2e88`), rebuilt (item 6);
     - the get-up: lying, his crouch, his half-crouched guard (`0x2d80`) as rising, his stance.

     His reaction tables are at `0x198a8` (13 hit streams) and `0x198dc` (11 knock-outs), 4 bytes an entry.
   - **Generated:** the crouching hit, the crouching punch, the crouching kick's chamber and full kick, and the sweep's three poses, beside AP5's crouch.
   - **The user's calls:** his crouch serves as his crouching block ("the crouch frame can already function as a block"). The kick's half-way frame was dropped as not needed.
   - **Poses per move (section 6, decision 4):** the punch has one new pose, as his jab and lunge have. The kick has two new poses (chamber and strike), as his low kick (`0x13fa0`) has. The sweep has three, as his high kick (`0x14030`) has. It strikes at full reach first, as Ryu's sweep does, then the leg comes back as he rises.
2. **How the generated poses were asked for.** One `create_image_pro` call for each pose, on a 128-pixel canvas, held to his 15 opaque pens. The references:
   - his approved crouch as the pose base;
   - his stance for identity, and again as the style image (`style_copy` outline, detail, shading);
   - his own ROM part (his jab's arm, his high kick's leg, his standing hit's face);
   - Ryu's matching ROM pose as a posing guide only (section 6, decision 5).

   Those that held his crouch's body and size were kept. Three kinds of candidate were not:
   - those that stood up out of the crouch;
   - those that turned his back;
   - one that kicked upward.
3. **His eyepatch.** Every generated patch came back solid black. In his ROM art it is his dark grey (pen 10) inside a one-pixel black rim, so the inside of each one was set to pen 10 and the rim and strap kept black. One came back his ROM way. The speck clean-up leaves his head alone, because his eyes and face lines are dark pens.
4. **Reusing earlier candidates.** The spike's crouching-kick call had come back on the crouch candidate's body, offset a few pixels. Its full kick was kept, for no new generations:
   - Above the waistband it carries the approved crouch itself, eyepatch and guard included, placed at that offset.
   - Below it the kick's own leg stays.
   - The second, angled stripe the generator drew across his shorts was painted out, as on the crouch. The user caught it.
5. **Inpainting for the chamber.** Two `create_image_pro` tries for the chamber (knee drawn up, foot lifted) stood him out of his squat, 108 to 117 pixels tall against the crouch's 91. Inpainting (`inpaint_image_pro_flash`, 5 generations on a 96-pixel canvas) repaints only a mask and leaves the rest pixel for pixel.
   - The first mask covered the shin and foot alone. The user found them wrong, and the knee came out squared at the mask's edge.
   - The second covered the leg from mid-thigh down, sparing the shorts. It was approved.
   - Inpainting takes no palette, so its pixels are reduced to his nearest pens (56 of them), and the ledger records his pens with a note saying so.
6. **The lying pose keeps his eyepatch mid-round (user).** In his ROM knock-out pose the patch has come off: it lies on the floor by his head, and his eye is bare. After a throw or a mid-round knockdown it should still be on him. The pose is rebuilt from his ROM pixels alone:
   - two crops of it leave out the floor's corner;
   - his stance's patch is laid over his eye a column at a time, each one-pixel column turned a quarter anticlockwise to his head's angle, so that only its black and grey pixels are drawn;
   - the strap is trimmed to his head's outline.

   His head lies crown to the left, face up. The user identified the features the builder had misread: the black mark it first took for his eye is his mouth, the dot below it his ear, and his eye is just before his nose. The knock-out at the end of a round still plays the ROM pose.
7. **A fighter's set can come in parts.** A recipe may say what is `still_to_come` (Sagat's: his air set and specials, AP8). The game's loader keeps him on his ROM art until that is gone, however much is approved, and logs `sagat keeps his ROM art: his set is not finished, with his air set and specials (AP8, #172) still to come`. The review page reads no reviews and still builds him. With the line removed, the local-pack test loads his ground set from the pack: `sagat (9 moves; 6 ROM poses, 3 rebuilt, 8 generated)`.
8. **Free poses carry no fingerprint.** A verdict on a generated pose holds for one file (AP5, item 4), but a free pose's verdict carries over when its recipe changes. When the lying pose was rebuilt, its entry was set back to pending by hand. So were the two sweep poses rejected only as part of the set. The page did the rest.
9. **The real rate (section 6, decision 9).** AP7 spent 115 generations:
   - 9 `create_image_pro` calls (90);
   - 5 inpaintings (25).

   Six generated poses were kept from it, and the spike's kick cost nothing new. With the spike's 20, Sagat's ground set took 135 generations for 8 generated poses, about 17 a pose with retries, in three review rounds. The month from 2026-10-27 stands at 245 of 1,800, and PixelLab agrees. The approved pack is backed up (`d2c1c5a`).
10. **Compromises.** His crouching block (the crouch), the throw (three ROM stand-ins), the thrown set (a turned recoil) and the get-up (the half-crouch as rising) are listed in [`docs/planning/art-upgrades.md`](../planning/art-upgrades.md).

### AP8 (Sagat's air set and specials, #172)

1. **His air set.**
   - **The jump arc is free: his own ROM hop.** His intro and Tiger Knee hop on the same poses: half-crouch (`0x2d80`), leg lift (`0x2c8c`), and the knee pose (`0x2d0a`) at the top. It is one arc for every direction, landing in his crouch.
   - **The five jumping normals are generated**, at Ryu's five:
     - punch (straight out at head height);
     - diagonal punch (down and forward);
     - light/medium kick (straight out at waist height);
     - heavy kick (turned away and high, as Ryu's heavy jumping kick is his turned spin pose);
     - diagonal kick (turned away, down and forward).

     Each starts and ends on the knee pose. His knee pose was first meant as a free flying knee, but it is the jump's own top pose, so pressing the button would change nothing on screen.
   - **The airborne poses** are shown 48 pixels up, a suggested mid-jump height. Each strike lines up with the knee pose by the centre of his shorts. SF1+ moves him through the arc.
2. **The low Tiger Shot, as AP2 describes it.** A squatting wind-up with both hands at his rear hip, then both arms thrust forward at knee height from a deep lunge. That is two new poses, as his high Tiger Shot has. The shot is his ROM projectile, launched lower by SF1+. His high Tiger Shot and Tiger Knee stay his ROM streams (`0x144b4`, `0x14242`) and are not in the catalogue.
3. **The canvas follows the figure.** The airborne strikes stand about 110 pixels tall, so they were asked for on a 160-pixel canvas (15 generations a call). The crouching poses stayed on 128 (10).
4. **A strike must keep the legs it starts from (user).** The first punch and kick, generated whole, drew his legs another way than the knee pose they start and end on. So the legs swapped between frames. Both were rebuilt by inpainting on the knee pose itself: the punch's arm, and the kick's leg. The kick's foot came back a stub twice and was inpainted again on its own. Outside their masks both are the ROM knee pose, pixel for pixel, and the speck clean-up is kept inside the masks.
5. **Two review rounds.** The user approved seven of the nine new poses at the first round; the punch and kick at the second.
6. **His set is finished and loads.** The recipe's `still_to_come` is gone. On this machine the game's own loader logs `sagat (16 moves; 8 ROM poses, 3 rebuilt, 15 generated)` beside Ryu and Ken, with no fighter kept on his ROM art. The bank spans 2,512 tile slots. The pack is backed up (`f4d52ce`).
7. **The pilot's real rate.** Sagat's whole set, the spike included, took 251 generations in 28 calls: 17 `create_image_pro` calls and 11 inpaintings. AP7 spent 115, AP8 116.
   - **Per approved generated pose:** about 17 (15 poses), retries and rejected candidates included. Counting his 10 new free poses too, about 10 a new pose.
   - **Per move:** about 16 across his 16 moves. About 25 for each of the 10 moves that needed newly generated art (the crouching hit, punch, kick and sweep, the five jumping normals and the low Tiger Shot).
   - **Against the inventory:** it priced his gaps at 33 poses with specials. He took 25 new poses, of which 15 were generated. His own ROM covered the throw, the thrown set, the get-up, the jump and the lying pose. The user let his crouch serve as his crouching block.
   - **Review:** five rounds, three for the ground set and two for the air set, were needed for 26 poses.
8. **The revised timeline for the other nine bosses.**
   - **Their gaps:** the inventory prices them at about 249 new poses, counting the AP2 specials that have no ROM stand-in. By fighter:
     - Retsu 23;
     - Geki about 27;
     - Joe 30;
     - Mike 35;
     - Lee 24;
     - Gen 26;
     - Birdie 30;
     - Eagle 26;
     - Adon 28.
   - **Generated share:** Sagat generated 45% of his inventory figure. The others have jumps in their ROM art but fewer free stand-ins elsewhere, so take 45 to 60%: about 110 to 150 generated poses.
   - **Generations:** at 17 a pose, that is about 1,900 to 2,500. The design's estimate for the whole project was 2,500 to 4,000; with Sagat's 251 the project should come in at about 2,150 to 2,750, the low end.
   - **When:** this month's cycle has 1,439 of the project's 1,800 left until 2026-11-27. That covers about 85 generated poses: five to seven bosses at 12 to 17 each. The rest falls in the cycle from 2026-11-27, so the generation itself fits in two cycles, finishing by late December 2026.
   - **What will set the pace:** the review, more than the budget. At Sagat's five rounds, the nine take about 40 to 45 rounds of the user's review.
   - **Order:** as section 6 has it, in the order SF1+ will need them.
9. **What to carry to the other nine.**
   - Inpaint one part of an approved pose rather than regenerate it whenever the rest must stay the same. That covers a chamber, a strike from a held pose, or a foot.
   - Give the posing guide from Ryu's matching ROM move, and the fighter's own ROM pose for the body.
   - Check a generated pose before review for:
     - his eyepatch or face against his ROM art;
     - a second stripe on the shorts;
     - the legs between frames of one move;
     - his size against the frames around it.
10. **Compromises.** His jump (his ROM hop) is listed in [`docs/planning/art-upgrades.md`](../planning/art-upgrades.md).

### AP9 (Retsu's ground set, #173)

1. **His ROM streams.** His move table is at `0x16e6c` (13 moves). His hit and knock-out tables are at `0x195a8` (13 streams) and `0x195dc` (11), 4 bytes an entry. His attacks are the streams from `0x0c9b6` to `0x0d074`: jab, lunging punch, mid kick, high kick, the flying kick twice, and the sweep. Together they draw the inventory's 38 distinct poses.
   - His ROM art already holds a crouch (`0x124d`), a crouching block (`0x120d`), a crouching hit (`0x12ec`) and a spinning sweep (`0x0cf1a`). Those stay his ROM streams, outside the catalogue, so his ground set needed only the two crouching normals drawn new.
2. **His ground set, and where each pose comes from.** Fourteen poses make five moves.
   - **Free, from his ROM art:**
     - the throw, a kempo wrist throw (AP2): the seize is his jab's open-hand strike (`0x1320`), the step across his lunging punch (`0x10ca`), the flip his sweep's last pose (`0x12a4`) mirrored, dropped low with his hands down in front;
     - the thrown set: seized is his standing hit (`0x1069`), and lifted and over the top are his fall pose (`0x1209`), turned a quarter and a half, as Ryu's is;
     - the get-up: lying (`0x11e9`), his fall pose set on the floor as sitting up, his crouch, the half-crouch his crouch rises through (`0x1129`), his stance.
   - **Generated:** the crouching punch's two poses and the crouching kick.
   - **The user's call:** the kick's chamber was dropped as not needed, after two tries.
3. **Poses per move (section 6, decision 4).**
   - The punch has two new poses, a wind-up and an open-palm strike, as his jab draws in (`0x1120`) and strikes (`0x1320`).
   - The kick has one new pose, as Ryu's crouching kick strikes in one (`0x03a0`).
   - The throw has three poses and the thrown set four, as Ryu's and Sagat's do. The get-up has five: his own crouch rises through the half-crouch.
4. **Inpainting on a ROM pose.** The punch's wind-up and strike are his ROM crouch with only the front arm inpainted (5 generations each, 96×64), so everything else is his ROM pixels. Both were approved at the first round.
5. **Inpainting cannot change his stance.** The first kick and chamber were a leg inpainted on his kneeling crouch. The user rejected the kick for kicking out from a kneel, and the chamber as needing another pose altogether.
   - The kick was then generated whole (`create_image_pro`, 120×88). The references were his crouch as the pose base, his own kick (`0x10c4`) for the leg, Ryu's crouching kick (`0x03a0`) as the posing guide, and his stance as the style image. All four candidates rose off the knee onto his rear foot, at his crouch's height, and their faces held his ROM face.
   - The second chamber was that kick with only the front leg inpainted, so his body stayed the kick's. The user rejected it too and dropped the chamber.
   - Before inpainting a limb, check that the pose under it stands the way the move needs.
6. **His pens.** His ROM body is drawn in pens 0 to 6 and 8 to 11.
   - The maroon lining (7) shows only where his robe flares.
   - The lilac greys (12, 13) appear in a few dozen pixels of his jump and sweep, and pen 14 never.
   - Generation and inpainting reached for the lilac greys on his trousers, sleeve and belt ends, so the touch-ups (`candidates/retsu/ground-touch-ups.py`) set them, and inpainting's off-palette pixels, to his nearest body pens.
7. **A send can fail without charge.** The kick's first send, four references and the style image, failed at PixelLab with "Maximum 5 images allowed, got 6". The chamber's call, with the same count, ran. With three references the kick ran. It charged nothing, so the ledger notes it on the call that ran.
8. **Review.** The user approved 13 of 15 poses at the first round, the kick at the second, and dropped the chamber, so all 14 are approved (`art/review/retsu.json`).
9. **His set comes in halves, as Sagat's did.** His recipe's `still_to_come` names his air set and specials (AP10, #174). Until it is gone the game logs `retsu keeps his ROM art: his set is not finished, with his air set and specials (AP10, #174) still to come`. With the line removed, the local-pack test loads his ground set from the pack: `retsu (5 moves; 8 ROM poses, 3 rebuilt, 3 generated); the bank spans 364 tile slots`.
10. **Cost.** AP9 spent 52 generations in 8 calls: 2 `create_image_pro` calls (20) and 6 inpaintings (32). That is about 17 for each of the 3 kept generated poses, Sagat's rate. Of the 52, 20 made the approved poses; the other 32 went to kicks and chambers that were rejected or dropped.
   - The month from 2026-10-27 stands at 413 of 1,800, and PixelLab agrees.
   - The approved pack is backed up (`218956a`).
11. **Compromises.** His throw, thrown set and get-up (ROM stand-ins) are listed in [`docs/planning/art-upgrades.md`](../planning/art-upgrades.md).

### AP10 (Retsu's air set and specials, #174)

1. **His air set.**
   - **His ROM art covers the jumps.** His three jumps arc on the same poses (`0x0c4a8`, `0x0c5c8`, `0x0c708`). His flying kick (`0x1248`, streams `0x0ccd6` and `0x0cdf8`) is the diagonal jumping kick, the same shape as Ryu's (`0x03e0`). Those stay his ROM streams. The catalogue holds his top-of-jump pose (`0x1288`), shown 48 pixels up, and four jumping normals that start and end on it.
   - **Punch and light/medium kick:** his top-of-jump pose with one limb inpainted, so his tucked legs stay the ROM pose's. The kick's thin shin and stub of a foot were inpainted again on their own, as Sagat's foot was. Both were approved at the first round.
   - **Diagonal punch:** two inpainted arms were rejected, one driving too steeply and one keeping his guard hand as a third hand. The user wanted his body to lean into the punch. It was generated whole, leaning, with his knees tucked, posed as Ryu's (`0x03e5`), and approved at the second round.
   - **Heavy kick:** this took four rounds.
     - His turned high-kick follow-through (`0x1184`), free, aimed its leg into the background.
     - A high kick inpainted on the jump pose had a section missing. The thigh was cut off by a straight edge where it met his raised elbow, which lay outside the mask.
     - Its repaired seam was not clean enough.
     - Generated whole, posed as Ryu's jumping kick (`0x058a`), it was approved.
2. **The Kiai Shot is a ROM stand-in (user, 2026-11-03).** It is his jab's wind-up (`0x1120`) and open-palm strike (`0x1320`), a one-handed palm thrust, as a short burst of ki leaves his hand (the SF1+ project's effect). Four rounds of AP2's two-handed thrust were rejected first:
   - a generated thrust with the gathering and recovery inpainted on it: badly stitched, the hands unclear;
   - the whole upper body inpainted: the user found the belt's hanging ends too long;
   - the ends shortened by hand and the robe below them inpainted again: not clean enough;
   - all three generated whole, short belt ends asked for: the belt still wrong.

   At the review limit the user chose the stand-in. It is listed in [`docs/planning/art-upgrades.md`](../planning/art-upgrades.md) for AP2's poses later.
3. **What to carry to the other eight.**
   - **A mask must cover what the new limb crosses.** An inpainted limb ends at the mask's edge, so a thigh rising past an elbow needs the elbow inside the mask.
   - **A mask must also clear everything the old pose had there.** The thrust's fingertips and the top line of its sleeve were left hanging in the air.
   - **Hand edits to clothing read as edits.** The user rejected a belt shortened pixel by pixel and the patched robe below it. When a part must change, inpaint the whole region it sits in or generate the pose whole.
   - **Whole generations lose exact legs between frames but leave no seams.** The user approved the whole-generated diagonal punch and heavy kick.
   - **Canvas size and figure size.** A 128-pixel canvas drew a narrow gathering pose 115 pixels tall; on 104 it came out at the thrust's 92.
   - **Failed calls charge nothing.** A heavy-load failure and a "too many images" failure both cost nothing, and PixelLab's count agrees.
4. **The review page and chat verdicts.**
   - When a pose's origin changes (the heavy kick from free to generated), the page starts its rejections again; its free rejection shows only in the notes.
   - Verdicts the user gave in chat (round three) were written into `art/review/retsu.json` by hand, as AP6 recorded Ryu's approval.
5. **Review.** Five rounds for this half; the page holds 21 approved poses for his whole set.
   - First round: the top-of-jump pose, punch and kick approved.
   - Second round: the diagonal punch approved.
   - Third round: all four remaining poses rejected in chat.
   - Fourth round: the heavy kick approved.
   - Fifth round: the Kiai stand-in approved.
6. **His set is finished and loads.** The recipe's `still_to_come` is gone. On this machine the game's own loader logs `retsu (10 moves; 11 ROM poses, 3 rebuilt, 7 generated)` beside Ryu, Ken and Sagat, with no fighter kept on his ROM art; the bank spans 3,278 tile slots.
7. **Cost.** AP10 spent 216 generations in 30 calls: 9 `create_image_pro` calls and 21 inpaintings.
   - By move: the Kiai Shot 142, the diagonal punch 28, the heavy kick 28, the kick 12, the punch 6.
   - The Kiai Shot's 142 bought no pose.
   - Retsu's whole set took 268 generations for 7 kept generated poses, about 38 each, twice Sagat's rate; without the Kiai Shot about 18.
   - The month from 2026-10-27 stands at 629 of 1,800.
8. **Compromises.** His Kiai Shot (a ROM stand-in) is listed in [`docs/planning/art-upgrades.md`](../planning/art-upgrades.md).

### AP11 (Geki's ground set, #175)

1. **His ROM streams.**
   - His move table is at `0x16bfc`. Moves 4, 8 and 0xc are his teleport, so he has no block.
   - His hit and knock-out tables are at `0x19548` (13 streams) and `0x1957c` (11).
   - His attacks are the streams from `0x0b968` to `0x0c20e`: claw thrust, crouching claw, front kick, the diving claw twice, the hopping claw, the shuriken hop and the smoke variant.
2. **His ground set, and where each pose comes from.** Eighteen poses make nine moves.
   - **Free, from his ROM art:**
     - the crouching block: the squat his jumps take off from (`0x0fc4`), claw and fist raised;
     - the circle throw (AP2): the grab is his claw thrust's wind-up (`0x0ea0`), the fall back his knock-out fall pose (`0x0eac`) with a leg kicked up, the roll back up his somersault tucks (`0x066a`, `0x0667`), into his crouch;
     - the thrown set: his standing hit (`0x0f00`), then his fall pose turned a quarter and a half, then lying (`0x0f66`);
     - the get-up: lying, his somersault tuck rolling up, his crouch, his half-crouch (`0x0f04`), his stance;
     - the light kick's chamber: his front kick's knee-up (`0x0f08`).
   - **Generated:**
     - the standing block, whole on his stance;
     - the jab, his front arm inpainted on his stance;
     - the light kick, the crouching kick and the sweep, whole.
   - **Not used:** his standing-hit pose (`0x0f00`) could guard, but a block that looks like being hit would read wrong, so the block was generated.
3. **Poses per move.**
   - The jab has one new pose: the user dropped its wind-up (his walk pose) as not needed.
   - The light kick is his front kick's chamber and a new strike, as his front kick (`0x0bb54`) chambers and strikes.
   - The crouching kick and the sweep have one new pose each.
   - The blocks have one pose each, as Ryu's.
   - The throw has four poses before his crouch, the thrown set four, and the get-up five.
   - His claw thrust, crouching claw and front kick stay his heavier punch, crouching punch and kick, as ROM streams.
4. **Masks, again.**
   - The jab's first mask kept its top edge below his face and squeezed the arm thin. Freeing the space right of his hood gave the arm its full thickness.
   - The light kick's first mask took only the hanging shin, and it came back a stub; the thigh had to be inside the mask too.
   - Even so, the user rejected both inpainted kicks: the edit showed, and his body should move into the kick. Generated whole, both were approved at the second round.
5. **Two pens can share a colour.** Geki's body blue (pen 14) is also his teleport smoke's colour (pen 6), so a picture cannot tell them apart.
   - `sf-art pack` now gives an exact tie to the pen his stance draws (`sf_art::pack::stance_pens`).
   - Ken has two blacks (pens 0 and 6), but his stance draws pen 0, so his packing is unchanged.
   - Tested: a shared colour goes to the stance's pen, and Geki's stance draws pen 14 and never pen 6.
6. **The sweep took three picks.**
   - His diving claw's landing pose (`0x0fca`) was not a sweep and had a blade in his hand.
   - The first whole generation, turned half away, did not read as one.
   - The user asked for another candidate from the same call; the one facing us, leg swept along the floor toward the opponent, was approved. No generations were spent on the swap.
7. **Review.** Three rounds, all 18 poses approved (`art/review/geki.json`). First round: 15 of 19. Second: the kicks. Third: the sweep.
8. **His set comes in halves.** His recipe's `still_to_come` names his air set and specials (AP12, #176), and the game logs `geki keeps his ROM art: his set is not finished, with his air set and specials (AP12, #176) still to come`. With the line removed, the local-pack test loads his ground set: `geki (9 moves; 11 ROM poses, 2 rebuilt, 5 generated)`.
9. **Cost.** 87 generations in 12 calls: 4 `create_image_pro` calls (40) and 8 inpaintings (47). That is about 17 for each of the 5 kept generated poses, Sagat's rate. The month from 2026-10-27 stands at 716 of 1,800, and PixelLab agrees. The pack is backed up (`72007f4`).
10. **Compromises.** His crouching block, throw, thrown set and get-up (ROM stand-ins) are listed in [`docs/planning/art-upgrades.md`](../planning/art-upgrades.md).

### AP12 (Geki's air set and specials, #176)

1. **His air set and specials, and where each pose comes from.** Eleven poses make five moves; with his ground set, 29 poses in 14 moves.
   - **Free, from his ROM art:**
     - the top of the jump: the squat his straight jump and hopping attacks hold (`0x0fc4`), shown 48 pixels up; his jumping attacks start and end on it;
     - the Teleport Strike (AP2's stand-in): his teleport's smoke cloud (`0x11e6`) thinning to wisps (`0x1326`, `0x1329`, `0x132d`), the cloud again, then his claw thrust's wind-up (`0x0ea0`) and strike (`0x0ea6`), placed as the thrust places them (x-offsets 20 and 44).
   - **Generated:**
     - the jumping punch: his front arm inpainted on the top-of-jump squat, so his tucked legs are the ROM's, pixel for pixel;
     - the jumping kick, the heavy jumping kick and the diagonal jumping kick, whole on the squat and his own front kick's leg (`0x0f60`).
   - **Stay his ROM streams, not in the catalogue:** his jumps, his diving claw (`0x0bc24`, `0x0be5c`: his diagonal jumping attack, so he needs no new diagonal punch) and his Shuriken (`0x0bf98`).
2. **Poses per move.** Each jumping attack is one strike between two top-of-jump poses (6, 14 and 8 frames). The Teleport Strike plays ten frames from his stance, four smoke poses and the claw thrust's two.
3. **The two high kicks came from one call.**
   - All four of the jumping kick's candidates kicked higher than the waist height asked for. Two were kept: the boot at head height as the jumping kick, the boot above his head as the heavy one.
   - The heavy kick's own call gave a straight hanging leg with a blade in his hand, flying belt ends, or lower kicks.
   - The two kicks look alike, so the heavy one is listed for a redraw.
4. **Lining up.** Each generated kick is anchored so the top of his hood meets the top-of-jump pose's. Their hoods measure 13 to 15 pixels wide against the ROM squat's 13.
5. **Review.** One round: all 11 new poses approved. The page holds 29 approved poses for his whole set (`art/review/geki.json`).
6. **His set is finished and loads.** The recipe's `still_to_come` is gone. On this machine the game's own loader logs `geki (14 moves; 18 ROM poses, 2 rebuilt, 9 generated)` beside Ryu, Ken, Sagat and Retsu, with no fighter kept on his ROM art; the bank spans 4,206 tile slots.
7. **Cost.** AP12 spent 42 generations in 5 calls: 3 `create_image_pro` calls (30) and 2 inpaintings (12), about 10 for each of the 4 kept generated poses.
   - Geki's whole set took 129 generations for 9 kept generated poses, about 14 each.
   - The month from 2026-10-27 stands at 758 of 1,800, and PixelLab agrees. The pack is backed up (`c24b70a`).
8. **Compromises.** His heavy jumping kick (a near twin of the jumping kick) and his Teleport Strike (a ROM stand-in) are listed in [`docs/planning/art-upgrades.md`](../planning/art-upgrades.md).

### AP13 (Joe's ground set, #177)

1. **His ROM streams.**
   - His move table is at `0x1734c`. Move 4 is his standing block (`0x0e370`, pose `0x176c`); move 5's stream pointer (`0x0e36a`) lands inside a frame record, and move 8 is empty, so he has no crouch.
   - His hit and knock-out tables are at `0x19668` (13 streams) and `0x1969c` (11). His attacks are the streams from `0x0e3ba` to `0x0eade`.
   - His fifteen pens are all different colours, so packing needs no tie-break.
2. **His ground set, and where each pose comes from.** Fourteen poses make eight moves.
   - **Free, from his ROM art:**
     - the hip toss (AP2): the grab is his straight's step-in (`0x1768`), the load onto his hip the back-turned punch of his kick-and-punch (`0x194a`), the throw over his body punch's low strike (`0x1a00`), the recovery its wind-up (`0x17c4`);
     - the thrown set: his standing hit (`0x182c`), then his airborne-hit pose (`0x19ac`) turned a quarter and a half, then lying face down (`0x1a64`);
     - the get-up: lying, the tuck at the top of his jump (`0x1a88`) set on the ground, his crouch, his stance.
   - **Generated:**
     - the crouching hit, the crouching kick and the sweep, whole;
     - the crouch: the approved crouching kick with its front leg inpainted, then its hair inpainted.
   - **The crouching block holds the crouch**, its fists already up before his face, as Sagat's does.
3. **The crouch took five rounds.**
   - First round: his ROM duck (`0x17c0`, the posture his ducking body punch holds) as the crouch, and his standing block (`0x176c`) as the crouching block. The user rejected both as not low enough.
   - Second: the approved crouching kick with its front leg inpainted, drawn in under him. The user rejected its hair, the kick's own generated hair.
   - Third: the hair inpainted as a spiky flat-top. The user said he looked like Guile; his hair is rounder and smoother.
   - Fourth: the hair inpainted as a smooth rounded crew cut. The user found it see-through: skin tones mottled into it read as his scalp.
   - Fifth: those 13 pixels given his hair's own olive and pale yellow, as his ROM hair is drawn. Approved.
   - Rounds three and four were the user's words in chat, so the page counts two rejections.
4. **Whole generation would not crouch him low at his own scale.**
   - At his stance's scale, three calls drew crouches 71 to 84 pixels tall against his 94-pixel stance, no lower than his 74-pixel duck.
   - A shorter canvas (96x72) drew deep squats 49 to 58 pixels tall, but about 15% smaller than his stance, head and chest narrower. Poses posed on such a squat came out at his full scale anyway (the crouching kick and sweep) or at the squat's (the crouching block, 42 to 52 tall).
   - The way down was the approved crouching kick, already squatting at his scale with his hips low: inpainting only its front leg gave a crouch 67 pixels tall, as low as the kick and the sweep.
5. **Hair is part of his look.** The generator draws Joe's hair softer, paler and less round than his ROM crew cut. A flat-top reads as another fighter (Guile). His ROM hair is drawn only in olive, pale yellow, brown and black; skin tones inside it read as thinning. His crouching hit, crouching kick and sweep keep their generated hair, listed for a redraw.
6. **The crouching hit stands taller than the crouch.** It was posed on his duck in the first round, so it is 80 pixels tall against the crouch's 68. The user was told and kept it; it is listed.
7. **Review.** Five rounds, all 14 poses approved (`art/review/joe.json`). The first round approved 13 of 15; the crouching block's own pose then went, as it holds the crouch.
8. **His set comes in halves.** His recipe's `still_to_come` names his air set and specials (AP14, #178), and the game logs `joe keeps his ROM art: his set is not finished, with his air set and specials (AP14, #178) still to come`. With the line removed, the local-pack test loads his ground set: `joe (8 moves; 8 ROM poses, 2 rebuilt, 4 generated)`.
9. **Cost.** 122 generations in 15 calls: 9 `create_image_pro` calls (90) and 6 inpaintings (32).
   - By move: the crouch 72, the crouching hit 20, the crouching block 10, the crouching kick 10, the sweep 10.
   - That is about 30 for each of the 4 kept generated poses, nearly twice Sagat's rate, most of it on the crouch.
   - The month from 2026-10-27 stands at 880 of 1,800, and PixelLab agrees. The pack is backed up (`778a1fd`).
10. **Compromises.** His crouching block, throw, thrown set and get-up (stand-ins), his crouching hit's height and the generated hair on three poses are listed in [`docs/planning/art-upgrades.md`](../planning/art-upgrades.md).

### AP15 (Mike's ground set, #179)

1. **His ROM streams.** His move table is at `0x170dc`: move 4 is his one guard (`0x1648`), move 5 his crouch (`0x156a`), moves 6 to 8 are empty. His hit and knock-out tables are at `0x19608` (13 streams) and `0x1963c` (11). His attacks are the jab `0x0d8c6`, straight `0x0d956`, ducking punch `0x0da26` and lunging hook `0x0db1c`. A grid's row and column bytes are counts less one, as the recipes give them.
2. **His ground set, 14 poses in nine moves, approved after four rounds.**
   - **Free, from his ROM art:** the thrown set (his standing hit `0x15d4`, his knocked-back pose `0x1650` turned a quarter and a half, lying `0x15e8`); his crouch as the crouching block; his straight (`0x147b`, `0x14e0`) for the body hook and, after his hook's follow-through (`0x14e7`), the overhand right; his ducking punch (`0x15aa`, `0x16a0`) for the low body hook and the crouching uppercut; the throw as his hook's wind-up (`0x1409`) for the grab and his straight's strike for the shove, with no headbutt.
   - **Generated:** the crouching hit (his hit pose's upper body on his crouch, joined at the chest, only the join inpainted) and the get-up's rising pose (his ducking wind-up with everything below his head inpainted, kneeling and pushing up).
   - **ROM stand-ins at the review limit (the maintainer, 2026-11-06).** The seven new punch and throw poses AP2 describes (the hooks, the overhand, the crouching uppercut, the clinch and headbutt, and a crouching guard) were rejected four times each. The maintainer chose his ROM moves instead, with no more generation. They are listed in [`docs/planning/art-upgrades.md`](../planning/art-upgrades.md).
3. **What failed, for the next fighters.**
   - **Upper bodies pieced onto a crouch** read as two bodies at the waist when joined at the belt, and still did not read as one when the join was inpainted. Joined at the chest, where a leaning crouch still shows the hips, with only that band inpainted, the crouching hit was approved; the crouching block made the same way was not.
   - **Inpainting a punching arm** gave arms that looked pasted on (the arm alone), did not reach out (shoulders and chest in the mask), looked straight (his jab's ROM forearm laid at the target), or ignored a ROM fist laid as a guide and drew its own nearer one. A mask with room above the head drew a second head; a pose's ROM parts laid apart from it were left floating.
   - **Whole generation** at 128 pixels drew him about 15% smaller than his stance. A whole-generated crouching uppercut was a good punch but a different size from his crouch.
   - **His ROM head laid back** on a generated pose worked when the source head matched its angle (his victory pose's head on an upright uppercut), and not when it did not (his stance's tucked head).
   - **Pro Flash inpainting needs canvas sides in multiples of 4**; a refused send charges nothing, as does "Maximum 5 images" on `create_image_pro`.
4. **Review.** Four rounds. Round one approved 9 of 20, round two the rising pose, round three the crouching hit; the rest went to the maintainer at the limit.
5. **His set comes in halves.** His recipe's `still_to_come` names his air set and specials (AP16, #180). With the line removed, the local-pack test loads his ground set: `mike (9 moves; 10 ROM poses, 2 rebuilt, 2 generated)`.
6. **Cost.** 243 generations in 40 calls: 5 `create_image_pro` calls (50) and 35 inpaintings (193), for 2 kept generated poses. With main merged (AP14's ledger included), the month from 2026-10-27 stands at 1,216 of 1,800, and PixelLab agrees.

### AP16 (Mike's air set and specials, #180)

1. **His air set and specials, and where each pose comes from.** Seven new poses make seven moves; with his ground set, 21 poses in 16 moves.
   - **Free, from his ROM art:** the top of the jump, the tuck all three of his jumps pass through (`0x1608`), shown 48 pixels up; his five jumping punches start and end on it.
   - **Pieced from his ROM poses, only the waist inpainted:** the jumping punch (his jab's upper body, `0x1470`) and the jumping hook, his MK (his hook's strike, `0x1560`), each on the tuck's trousers, crotch and shoes, untouched. Only the shirt's hem, belt and waistband between them are inpainted.
   - **Generated whole, his ROM head laid back:** the diagonal punch (his straight's turned head), the low jab, his LK, and the two-fisted hammer, his HK (his jab's head).
   - **The Bison Rush (AP2) is all ROM poses, no new art:** his ducking wind-up as the low crouch, his hook's wind-up as the charge, his straight as the finish.
   - **The Iron Upper (AP2):** his crouch and ducking wind-up, his hook's follow-through (`0x14e7`) as the rising uppercut, then a peak inpainted on that same turned pose: the striking arm driven straight up, his legs hanging off the ground, his ROM sneakers laid on.
2. **His size.** Whole generation drew him at 85 to 93% of his size, as AP15 found, and a larger canvas or pixel sizes in the prompt made it worse. A pose base pieced from his ROM poses at his real size (an upper body on the tuck's legs) brought the punches to 93 to 100%. The hammer stands at about 90%, and a generated rising pose for the Iron Upper at about 90% was not used. Pieced and inpainted poses keep his scale exactly.
3. **Joining an upper body to the tuck (the maintainer).**
   - Inpainting the top of the jeans changed the crotch, and the hook's trousers looked turned away from the player. Keep the legs' own pixels and redraw only the hem and belt.
   - The torso's sides must run straight into the trousers' top. Round two left both torsos 7 pixels right of the trousers, a step at the waist. Line the torso up over the seat's outer edge and check it over the tuck and an approved jump pose.
   - A mask must also cover any hole left where the legs' pose had its own belt.
4. **The Iron Upper's peak must turn toward the opponent (the maintainer).** His win pose's raised fist on his take-off step faced the player and read as a victory pose. The peak carries on from the wind-up instead: his back to us, the striking arm on the opponent's side.
5. **Review.** Three rounds, all 21 poses approved (`art/review/mike.json`). Round one approved 4 of 7; round two the peak; round three the jump punch and hook.
6. **His set is finished and loads.** The recipe's `still_to_come` is gone. On this machine the game's own loader logs `mike (16 moves; 11 ROM poses, 2 rebuilt, 8 generated)` beside the other six fighters, with no fighter kept on his ROM art; the bank spans 6,372 tile slots.
7. **Cost.** AP16 spent 168 generations in 22 calls: 11 `create_image_pro` calls (110) and 11 inpaintings (58), about 28 for each of the 6 kept new pack poses. Mike's whole set took 411. The month from 2026-10-27 stands at 1,384 of 1,800, and PixelLab agrees.
8. **Compromises.** His pieced jumping punch and hook, the hammer's size, and the Bison Rush and Iron Upper (mostly ROM stand-ins) are listed in [`docs/planning/art-upgrades.md`](../planning/art-upgrades.md).

### AP17 (Lee's ground set, #181)

1. **His ROM streams.** His move table is at `0x175bc`: move 4 is his block (`0x1f40`), moves 5 to 8 are empty, so he has no crouch. His hit and knock-out tables are at `0x196c8` (13 streams) and `0x196fc` (11), 4 bytes an entry. His attacks are the streams from `0x0f366` to `0x0fc8e`: palm, punch, lunging punch, side kick, high kick, the flying kicks and the sweep. His fifteen pens are all different colours.
2. **His ground set, 15 poses in eight moves, approved at the first round (the maintainer, 2026-11-07).**
   - **The crouch is rebuilt** from the squat his sweep ends on (`0x1f00`): low on both knees, a hand on the floor between them, his near fist raised. In the ROM his head is turned back over his shoulder from the spin, so only his head (x11-24, y0-17) is mirrored in place, in four recipe parts, and he looks at the opponent with his pigtail trailing behind.
   - **Generated on the crouch, inpainted only at the joins:** the crouching block (his standing block's raised forearm, `0x1f40`, stood on its elbow), the crouching punch (his straight punch's whole arm, `0x1cc4`, from its shoulder), the crouching kick (his sweep's outstretched leg, `0x1f04`, from its hip) and the crouching hit (his hunched hit reaction, `0x1fa0`, head to hands, laid on its knees and lined up by the frog buttons). Every head, hand and limb is his ROM pixels; outside each mask every pixel is the source's.
   - **Free, from his ROM art:** the over-the-back throw (AP2) as his palm strike (`0x1c66`), then his sweep's spin (`0x1dec`), strike (`0x1f04`) and closing squat (`0x1f00`); the thrown set (his standing hit `0x1f46`, his airborne hit `0x1f5d` turned a quarter and a half, lying `0x1388`); the get-up through his sweep's first pose (`0x1c06`), down on one knee.
   - **Poses per move.** The crouching punch and kick strike in one new pose each, as his palm and Ryu's crouching kick do. His spinning sweep (`0x0fc8e`) stays his crouching heavy kick, a ROM stream.
3. **What worked, for the next fighters.**
   - **A fighter's own ROM pose can hold his crouch** where his moves have none: a squat inside an attack stream. Check which way his head faces; a head turned by a spin can be mirrored in place with recipe crops alone.
   - **Whole ROM limbs laid on the approved crouch, with the mask on the joins only** (shoulder, elbow, hip), kept his scale exactly and needed one call per pose. The mask clears what the old limb left (the guard fist, the knee the kick replaces).
   - **Keep a laid part's outlines out of the mask, and crop it whole.** The crouching hit's first mask ran through his lower hand and muddled its fingers; the second crop stopped at the knuckles. The third kept both hands whole, outlines and all.
4. **His set comes in halves.** His recipe's `still_to_come` names his air set and specials (AP18, #182), and the game logs `lee keeps his ROM art: his set is not finished, with his air set and specials (AP18, #182) still to come`. With the line removed, the local-pack test loads his ground set: `lee (8 moves; 8 ROM poses, 3 rebuilt, 4 generated)`.
5. **Cost.** 30 generations in 6 inpaintings, for 4 kept generated poses, about 8 each: the block, punch and kick 5 each, the crouching hit 15 (two tries not kept). With main merged (AP16's ledger included), the month from 2026-10-27 stands at 1,414 of 1,800, and PixelLab agrees (586 left).
6. **Compromises.** His crouch (the sweep's squat, his pigtail still flying, a hand on the floor in every crouching pose), throw, thrown set and get-up (ROM stand-ins) are listed in [`docs/planning/art-upgrades.md`](../planning/art-upgrades.md).

### AP18 (Lee's air set and specials, #182)

1. **His air set and specials, and where each pose comes from.** Fifteen new poses make five moves; with his ground set, 30 poses in 13 moves.
   - **Free, from his ROM art:** the top of the jump, the tuck all three of his jumps pass through (`0x1e4c`), shown 48 pixels up; his new jumping attacks start and end on it. The Double Kick (AP2's stand-in) is his side kick (stream `0x0f5c8`) straight into his high kick (stream `0x0f678`), every pose placed at its stream's x-offset, stepping forward between them.
   - **Rebuilt:** the Zesshouohken's gather, his step-in (`0x1ccc`) with its rear arm, the fist chambered at the hip, mirrored onto the front side in recipe parts, so both fists are drawn back.
   - **Inpainted only at the joins:** the jumping punch (his straight punch's arm, `0x1cc4`, on the tuck's front shoulder), the diagonal jumping punch (his flying kick's arm, `0x1ec0`, whose head lies exactly on the tuck's) and the jumping kick (his jumping heavy kick's leg, `0x1ea8`, at the tuck's front hip). The Zesshouohken's strike is his lunging punch (`0x1d20`) with his palm strike's open hand (`0x1c66`) on the forearm, and the same forearm and palm laid again below it at his chest; only both wrists and the lower arm's root are inpainted.
   - **Stay his ROM streams, not in the catalogue:** his jumps, his flying kick (`0x0f788`, `0x0f92a`, his diagonal jumping kick) and his jumping heavy kick (`0x0faec`, the leg kicked straight out from an upright, turned body).
2. **Poses per move.** Each jumping attack is one strike between two top-of-jump poses, as his flying kicks strike in one. The Zesshouohken has two new poses, the gather and the strike, where AP2 proposed three; it recovers through his step-in and ready pose as his lunging punch does.
3. **What worked, for the next fighters.**
   - **Line a donor pose up by its head.** Matching his head's pixels against the tuck's found the flying kick's head at the same place to the pixel, so its arm went on at no offset and joined at the shoulder.
   - **A mirrored ROM limb can stand in for an arm no pose of his holds.** Two inpaintings of a front arm drawn back to the hip came back limp or not at all; his own rear arm, mirrored, needed no generation.
   - **Clear what a donor limb's old neighbours left.** The flying kick's arm brought the outline of the leg that ran under its cuff; with no leg there it hung in the air and was removed by hand.
4. **Review.** One round: the maintainer approved all 15 new poses at the first round (2026-11-07). The page holds 30 approved poses for his whole set (`art/review/lee.json`).
5. **His set is finished and loads.** The recipe's `still_to_come` is gone. On this machine the game's own loader logs `lee (13 moves; 18 ROM poses, 4 rebuilt, 8 generated)` beside the other seven fighters, with no fighter kept on his ROM art; the bank spans 7,336 tile slots.
6. **Cost.** AP18 spent 32 generations in 6 inpaintings: the jumping punch and diagonal punch 5 each, the jumping kick 6, the Zesshouohken 16 (its strike 6, and two gathers of 5 not kept). That is 8 for each of the 4 kept generated poses. Lee's whole set took 62 generations for 8 kept generated poses, about 8 each, the lowest rate yet. PixelLab's count agrees: 514 of the cycle's 2,000 left until 2026-11-27.
7. **Compromises.** His pieced jumping attacks (the light kick on the heavy kick's leg), the Zesshouohken's rebuilt gather (fists, where the strike is open palms) and the Double Kick (a ROM stand-in) are listed in [`docs/planning/art-upgrades.md`](../planning/art-upgrades.md).

### AP19 (Gen's ground set, #183)

1. **His ROM streams.** His move table is at `0x1782c`: move 4 is his block (`0x200c`, stream `0x10340`), moves 5 to 8 are empty, so he has no crouch. His hit and knock-out tables are at `0x19728` (13 streams) and `0x1975c` (11), 4 bytes an entry. His attacks are the streams from `0x103b0` to `0x10b4e`: jab, straight punch, front kick, the flying kicks (`0x10580`, `0x10702`, `0x10884`), the rolling low kick (`0x10a26`) and the arm flurry (`0x10b4e`). His fifteen pens are all different colours.
2. **His ground set, 17 poses in nine moves, approved after three rounds (the maintainer, 2026-11-07).**
   - **The crouch is pieced from his ROM poses:** his stance (`0x2000`) from the head to the waist, upright with both arms as they are, on the bent legs of the squat his hit reactions recover through (`0x2064`, 8 rows taken out of the thighs). His stance's own front panel hangs from the waist to the floor between the knees, with its red trim and hem, and the robe around it is inpainted. He stands 64 pixels tall against his stance's 88.
   - **Generated on the crouch, inpainted only at the joins:** the crouching punch (his straight punch's arm, `0x2126`, from the shoulder), the crouching block (his standing block's raised arm, `0x200c`), the crouching kick (his flying kick's outstretched leg, `0x2309`, from the hip). Where the arm his stance holds out hung over his forward thigh, a flat hint of the thigh was drawn into the source before inpainting.
   - **Also generated:** the crouching hit (his standing hit's upper body, `0x2060`, on the first crouch's legs, the waist inpainted) and his second standing kick, a low kick (his front kick's strike, `0x2180`, with the flying kick's leg at knee height, the hip inpainted), chambered through his front kick's knee-up (`0x212c`).
   - **Free, from his ROM art:** the palm throw (AP2) as his jab's reach (`0x2120`), the arm flurry's wheeling strike (`0x2246`) and lunging palm (`0x22a0`); the thrown set (his standing hit `0x2060`, his airborne hit `0x2068` turned a quarter and a half, lying `0x2348`); the get-up through his rolling low kick's tuck (`0x2300`, on hands and knees) and first pose (`0x20cc`, a hand reaching to the floor), then his crouch.
   - **Poses per move.** The crouching punch and kick and the low kick strike in one new pose each, as his straight punch and Ryu's crouching kick do. His rolling low kick (`0x10a26`) stays his crouching medium kick, a ROM stream.
3. **The crouch took three rounds.**
   - First: the hit-recovery squat itself, lowered, its seam's red trim redrawn by hand. The maintainer found it technically good but reading more like a hit than a crouch, and asked for his stance from the waist up on bent legs, the robe redrawn to the height.
   - Second: the stance's upper body on the squat's legs, the robe's skirt inpainted over the knees. The upper body was right; the robe looked cut short above the knees instead of hanging low.
   - Third: his stance's own front panel hung from the waist to the floor, the robe around it inpainted. Approved.
   - The punch, block and kick were rebuilt on each new crouch with the same joins, the kick reopened after its first approval so all four match.
4. **What worked, for the next fighters.**
   - **A crouch reads from the posture of the upper body, not only its height.** A squat out of a hit reaction, head down and hands clawed, read as a hit. The stance's own upper body on bent legs read as a crouch.
   - **A long garment keeps its length when the wearer crouches.** Gen's robe had to hang to the floor between the knees, not shorten with the legs. His stance's own panel, hung lower, gave the trim and hem for free.
   - **Inpainting leaves empty space empty.** Where an arm is taken away from over a thigh, the inpainting drew a straight edge and left the gap. A flat hint of the missing shape in the source, inside the mask, made it draw the thigh.
   - **Check a crop's edges for the next limb.** A raised-arm crop took in the edge of his other hand; a leg crop's top outline showed inside a sleeve.
5. **His set comes in halves.** His recipe's `still_to_come` names his air set and specials (AP20, #184), and the game logs `gen keeps his ROM art: his set is not finished, with his air set and specials (AP20, #184) still to come`. With the line removed, the local-pack test loads his ground set: `gen (9 moves; 9 ROM poses, 2 rebuilt, 6 generated)`.
6. **Cost.** 100 generations in 20 inpaintings (`inpaint_image_pro_flash`, 5 each), for 6 kept generated poses, about 17 each. By move: the crouch 25, the crouching block 25, the crouching punch 20, the crouching kick 15, the crouching hit 10, the low kick 5. PixelLab's count went from 586 to 426 left in the month from 2026-10-27; the other 60 were other sessions'.
7. **Compromises.** His crouch (pieced), his crouching hit (its robe ends at the knee, the crouch's hangs to the floor), throw, thrown set and get-up (ROM stand-ins) are listed in [`docs/planning/art-upgrades.md`](../planning/art-upgrades.md).

### AP20 (Gen's air set and specials, #184)

1. **His air set and specials, and where each pose comes from.** Thirteen new poses make five moves; with his ground set, 30 poses in 14 moves.
   - **Free, from his ROM art:** the top of the jump, the tuck all three of his jumps and his flying kicks pass through (`0x22a6`), shown 48 pixels up; his new jumping attacks start and end on it. The light jumping kick is the first strike pose of his straight-up flying kick (`0x2186`, stream `0x10884`), its head on the tuck's to the pixel, held at full reach; his heavy jumping kick strikes through it into its high kick (`0x21e0`).
   - **The Gekiro (AP2's rising somersault of kicks), free:** from his crouch, his high flying kick (`0x21e0`), that kick turned a quarter back so the leg is straight up over him, his tuck turned a half and three quarters as a backward somersault, his jump's falling pose (`0x224c`), and his crouch to land. Its three turned poses are recipes.
   - **The Hyakurenko, AP2's stand-in:** his ROM arm flurry (stream `0x10b4e`) as the ROM plays it, in the catalogue so SF1+ can re-time it; its two strikes are the throw's poses.
   - **Inpainted on the tuck:** the jumping punch (his straight punch's arm, `0x2126`, lined up by his head, 8 pixels off) and the diagonal jumping punch (his arm flurry's cuff and open hand, `0x2246`, on a slim sleeve from the shoulder).
   - **Stay his ROM streams, not in the catalogue:** his jumps, his diagonal flying kick (`0x10580`, `0x10702`) and his heavy jumping kick (`0x10884`).
2. **Poses per move.** Each jumping attack is one strike between two top-of-jump poses, as his flying kicks strike in one or two. The Gekiro has three new poses where AP2 proposed four to five, between two of his ROM poses.
3. **The diagonal punch's shoulder took the maintainer's call.**
   - Round one laid the flurry arm's whole wide sleeve on the tuck and inpainted only a small join. The maintainer: "Arm looks pasted on."
   - Round two inpainted the whole shoulder, upper sleeve and armpit over a guide that filled out his chest and shaded the robe under the arm. The maintainer: "the robe/torso is extending too much to the right with the arm, doesn't look good at all."
   - Round three kept the tuck's torso and robe as the ROM has them, outside the mask, and inpainted only the shoulder join and a slim sleeve to the flurry's cuff and hand. The maintainer: "It's very close now, the only issue I have with it is that the arm should be slightly more set back in his torso."
   - At the review limit his call was that fix: round three's whole arm moved 3 pixels back toward his back by hand, his head and rear hand laid back on top as the tuck draws them. Approved, with no more generation.
4. **What worked, for the next fighters.**
   - **A guide inside the mask becomes the body.** Inpainting keeps a guide's shape; a chest and armpit filled out in the guide came back as a wider torso. Paint a guide only where the new limb goes.
   - **Keep the mask off the body.** With the torso, the robe and its front edge outside the mask, only the arm can change, as on the approved jumping punch.
   - **A flat guide comes back flat.** Two inpaintings over a flat sleeve kept it flat. The fighter's own shaded sleeve, each column squeezed to the slimmer width by nearest sampling, gave his ROM outline, highlight and shade bands.
   - **An arm sits inside the body.** The shoulder reads right a few pixels back into the torso, behind the rear hand, not at its outer edge. Moving a finished arm by hand costs nothing.
   - **A flip from his own tuck.** Turned quarter by quarter, his top-of-jump tuck and a high kick made a somersault the maintainer approved at the first round.
5. **Review.** Twelve of the 13 new poses were approved at the first round (the maintainer, 2026-11-07). The diagonal punch was rejected three times, then approved at his call with the arm set back. The page holds 30 approved poses for his whole set (`art/review/gen.json`).
6. **His set is finished and loads.** The recipe's `still_to_come` is gone. On this machine the game's own loader logs `gen (14 moves; 17 ROM poses, 5 rebuilt, 8 generated)` beside the other eight fighters, with no fighter kept on his ROM art; the bank spans 8,357 tile slots.
7. **Cost.** AP20 spent 40 generations in 8 inpaintings (`inpaint_image_pro_flash`, 5 each): the jumping punch 5, the diagonal punch 35 (round one 5, round two 10, round three 20, two seeds a round from round two on). Gen's whole set took 140 generations for 8 kept generated poses, about 18 each. PixelLab's count went from 393 to 353 left in the month from 2026-10-27.
8. **Compromises.** His pieced jumping attacks (the light kick on the heavy kick's first pose), the Gekiro (turned ROM poses) and the Hyakurenko (a ROM stand-in) are listed in [`docs/planning/art-upgrades.md`](../planning/art-upgrades.md).

### AP21 (Birdie's ground set, #185)

1. **His ROM streams.** His move table is at `0x17a9c`: move 1 holds his stance (`0x2400`, four rows of 32-pixel entries), move 4 his block (`0x2588`), moves 5 to 8 are empty, so he has no crouch. His hit and knock-out tables are at `0x19788` (13 streams) and `0x197bc` (11), 4 bytes an entry. His attacks are the streams from `0x1141c` to `0x11b42`: jab, straight, overhead hammer, a low front kick (`0x115cc`), the diving attack twice and the lunging headbutt. Pens 7 and 14 are the same grey (`#777777`); packing gives it pen 14, which his stance draws more.
2. **His ground set, 18 poses in ten moves, approved after five rounds (the maintainer, 2026-11-07).**
   - **The crouch is pieced:** his stance (`0x2400`) from the waist up, upright and on guard, on the bent knees of the top of his jump (`0x240c`), whose upper body is his stance's with the head 4 pixels forward, so the two match pixel for pixel from the waist down. It stands on his stance's own boots and two-row shadow, and only the ankles are inpainted. He stands 90 pixels tall against his stance's 124.
   - **His ROM upper bodies on the crouch's legs, nothing generated:** the crouching block (his standing block, `0x2588`) and the crouching punch (his jab, `0x2480`), whose waists match the crouch's row for row.
   - **Inpainted only at the joins:** the crouching hit (his standing hit's upper body, `0x2600`, the waist band inpainted), the crouching kick and the sweep (his kick's leg, `0x27a8`, from the front hip at shin height and low along the floor), and his second standing kick, a high kick (his kick's leg turned 40 degrees up as a guide, the leg and hip inpainted), between his kick's chamber (`0x2726`) and recovery (`0x2780`).
   - **The get-up's rising pose is pieced:** his headbutt lunge's upper body (`0x28a0`), bent far forward with his head low and both fists over the front knee, on kneeling legs (the crouch's front leg planted, the rear knee down), the waist inpainted and the belt and seat finished by hand.
   - **Free, from his ROM art:** the head slam (AP2) as his overhead hammer's two-handed reach (`0x26a0`), arms-up (`0x246c`) and follow-through (`0x26a6`); the thrown set (his standing hit `0x2600`, his airborne hit `0x2608` turned a quarter and a half, lying `0x2680`).
   - **Poses per move.** The crouching punch, crouching kick, sweep and high kick strike in one new pose each, as his jab and his kick do. His low front kick (`0x115cc`) stays his other standing kick, a ROM stream.
3. **The crouching kick took five rounds, all on the join's height.**
   - The leg first joined at the waist, just under the vest; the maintainer asked for it "a little lower", then said 5 pixels lower was "still too high".
   - Moved down to the hip crease, where the crouch's thigh meets the seat, it had to be turned 14 degrees up to keep the boot at shin height; the maintainer found it "too low", and accepted the previous round's as a fallback.
   - Halfway between, about 3 pixels below round two's and 4 above round three's, the join was "perfect".
   - That round's source laid round three's picture from x52 on, 4 pixels higher, to keep its leg. The crop held the crouch's front fist as well, so a second fist sat 4 pixels above his own. The fifth round restored the crouch's pixels above the leg by hand. Rounds one to three never had it.
4. **The get-up took five rounds.**
   - His airborne-hit pose set on the ground read as a hit; his crouch with the rear knee down read as the crouch. A rising pose needs its own upper body, lower and leaning forward over a planted foot: the headbutt lunge's was "almost good".
   - Its waist had clipping: the lunge's belt crop left holes and the kneeling legs' jeans stuck up above it. A re-inpainting of just the seat changed 71 pixels and kept the holes; his belt drawn straight across the back by hand fixed it.
   - The seat below then ran straight down to a square corner: "his butt still isn't rounded". It was redrawn by hand as a curve that swells out under the belt and rounds in to the kneeling thigh, as his crouch's seat does.
5. **What worked, for the next fighters.**
   - **A fighter's jump tuck can hold his crouch's legs.** Its upper body is often the stance's, so the stance's own upper body (and his block's and jab's) sits on its knees with no join at all.
   - **A kick's join height is read against the body above it.** Under the vest read as the waist, the hip crease as too low; halfway was right. Move the join a few pixels at a time, and turn the leg to keep the strike's height.
   - **A crop laid again carries everything inside its rectangle.** Re-laying part of an earlier picture brought a fist along with the leg; crop to the limb's own pixels, and compare every new round with the approved pose outside the mask.
   - **Inpainting keeps transparent holes in a source.** Fill holes before inpainting, or fix small regions by hand.
   - **A seat is a curve.** Where an upper body is laid on other legs, check the back's outline below the belt rounds into the thigh.
6. **His set comes in halves.** His recipe's `still_to_come` names his air set and specials (AP22, #186), and the game logs `birdie keeps his ROM art: his set is not finished, with his air set and specials (AP22, #186) still to come`. With the line removed, the local-pack test loads his ground set: `birdie (10 moves; 8 ROM poses, 2 rebuilt, 8 generated)`.
7. **Cost.** 61 generations in 11 inpaintings (`inpaint_image_pro_flash`, 5 or 6 each), for 6 kept generated poses, about 10 each; the crouching block and punch were derived from the crouch, and the last two fixes were by hand. By move: the crouching kick 24, the get-up 15, the high kick 6, the sweep 6, the crouch 5, the crouching hit 5. PixelLab's count went from 504 to 393 left in the month from 2026-10-27; the other 50 were other sessions'.
8. **Compromises.** His crouch, crouching block and punch, high kick and get-up (pieced), and his throw and thrown set (ROM stand-ins) are listed in [`docs/planning/art-upgrades.md`](../planning/art-upgrades.md).

### AP22 (Birdie's air set and specials, #186)

1. **His air set and specials, and where each pose comes from.** Nine new poses make five moves; with his ground set, 27 poses in 15 moves.
   - **Free, from his ROM art:** the top of the jump, the tuck all three of his jumps pass through (`0x240c`), shown 48 pixels up; his new jumping attacks start and end on it. Bull Horn's wind-up, lunge and recovery are his headbutt's own (`0x2820`, `0x28a0`, `0x282a`, stream `0x11b42`).
   - **Rebuilt:** the jumping punch, his jab's upper body (`0x2480`) on the tuck's legs. The tuck's upper body is his stance's, so the jab's waist matches it row for row, as on his crouching punch.
   - **Pieced from his approved ground set:** the jumping kick is the approved crouching kick's leg and hip join laid on the tuck, which the crouch's legs are from row 47 down; nothing generated.
   - **Inpainted:** the diagonal jumping kick (that leg turned 35 degrees down and forward, the leg and hip inpainted); the heavy jumping kick (item 4); and Bull Horn's strike (item 3).
   - **Stay his ROM streams, not in the catalogue:** his jumps, his diving attack (`0x1165c`, `0x119a0`: both fists driven forward and down from the tuck, his diagonal jumping attack, so he needs no new diagonal punch) and Bull Head, AP2's stand-in, his headbutt stream (`0x11b42`) itself.
2. **Poses per move.** Each jumping attack is one strike between two top-of-jump poses, as his jab and kick strike in one. Bull Horn has one new pose where AP2 proposed three: his headbutt's wind-up and lunge already lower his head and drive him in, so the new pose is the strike, around them as his headbutt strikes in one lunge. The maintainer accepted the four jumping normals and the one-pose plan.
3. **Bull Horn's strike took two rounds.** It is his dive landing (`0x271a`): crouched low, leaning in, the near arm driven forward at waist height. Its upright head is replaced by the headbutt lunge's lowered head (crown forward, face down).
   - Round one set the head low between his shoulders, behind the near arm, and laid the near forearm and fist again higher as his far arm. The maintainer: "Looks like 4 arms and the head seems malformed", "Head hidden or lost", "Arms look wrong."
   - Round two kept the one real arm, made the far arm only its fist, peeking past the near one, and drew the head in front of everything, resting on the arm's top line. Only the back of the neck was inpainted. Approved.
4. **The heavy jumping kick took three rounds.**
   - Round one was the tuck with the kick's leg turned 40 degrees up, only the hip inpainted. The maintainer: "redo his whole lower half, the non-kicking leg should probably be at a different angle to look more animated", "Leg looks pasted on", "Too like the jump kick."
   - Round two built a flying split kick from his approved standing high kick (AP21): lifted off the floor, its standing leg turned 26 degrees back to trail down and behind him, its floor shadow gone, and everything from the belt down inpainted around both legs as guides. His upper body is his kick's (`0x27a8`), as the jumping punch's is his jab's. The maintainer: "Nearly perfect. The kicking leg could just be a slight bit longer."
   - Round three lengthened the kicking leg 5 pixels by hand: a cut across the shin at right angles to the leg, everything past it moved out along the leg, and the gap filled by repeating the 5 pixels of shin inside the cut. The boot, thigh and hip join are round two's. Approved.
5. **What worked, for the next fighters.**
   - **A copied limb reads as an extra limb.** A forearm laid again above the near one read as four arms. A far arm that only peeks, its fist past the near fist, reads as two.
   - **The head goes in front.** A head set behind an arm or between the shoulders was lost; drawn over everything, resting on the arm, it read at once. Check the face is whole before review.
   - **A big kick needs its own lower half.** Turning one leg on the light kick's body read as the same kick. The standing high kick lifted into the air, the other leg trailing back, read as a bigger kick, and inpainting the whole hip and both upper thighs around the guides left no seam.
   - **Length along a limb is a hand fix.** Stretching a shin by repeating a few pixels of it, across a cut at right angles to the limb, keeps its outline and dither and costs nothing.
6. **Review.** Three rounds. Round one approved seven of the nine new poses (the maintainer, 2026-11-07); round two Bull Horn; round three the heavy jumping kick. The page holds 27 approved poses for his whole set (`art/review/birdie.json`).
7. **His set is finished and loads.** The recipe's `still_to_come` is gone. On this machine the game's own loader logs `birdie (15 moves; 12 ROM poses, 3 rebuilt, 12 generated)` beside the other nine fighters, with no fighter kept on his ROM art; the bank spans 9,958 tile slots.
8. **Cost.** AP22 spent 28 generations in 5 inpaintings (`inpaint_image_pro_flash`, 5 or 6 each): the heavy jumping kick 12, Bull Horn 10, the diagonal jumping kick 6. The jumping kick and the third round were by hand. Birdie's whole set took 89 generations for 9 kept generated poses, about 10 each. PixelLab's count went from 373 to 288 left in the month from 2026-10-27; the other 57 were other sessions'.
9. **Compromises.** His pieced jumping attacks (the punch his jab's upper body, the kicks on his ground set's legs, the heavy kick on his standing high kick) and Bull Horn (mostly his headbutt's ROM poses, its strike pieced) are listed in [`docs/planning/art-upgrades.md`](../planning/art-upgrades.md).

### AP23 (Eagle's ground set, #187)

1. **His ROM streams.** His move table is at `0x17d0c`: move 1 holds his stance (`0x2920`), move 4 his block (`0x2aa0`), moves 5 to 7 are empty, so he has no crouch; move 8 holds his second guard (`0x2aa4`) on one leg, which is not a crouching block. His hit and knock-out tables are at `0x197e8` (13 streams) and `0x1981c` (11), 4 bytes an entry. His attacks are the stick jab (`0x1239e`), the spinning sticks (`0x12408`, and `0x124d8` run longer), the low lunging two-stick thrust (`0x12628`) and the jumping stick strike (`0x126f8`, `0x1285a`, `0x129bc`). His fifteen pens are all different colours.
2. **His ground set, 17 poses in ten moves, approved after two rounds (the maintainer, 2026-11-07).**
   - **The crouch is pieced:** his stance (`0x2920`) down to his belt, upright with both sticks crossed, on the thighs and knees of the top of his jump (`0x2a40`), cut above its hanging feet, with his stance's own hems, shoes and two-row shadow set under them, the front shin straight under the front knee. Only the shins' lower ends are inpainted. He stands 72 pixels tall against his stance's 93.
   - **His ROM upper bodies on the crouch's legs, nothing generated:** the crouching block (his standing block, `0x2aa0`, sticks crossed before his face) and the crouching hit (his standing hit, `0x2aa8`).
   - **His stick jab's arm turned at the shoulder:** the crouching stick sweep (50 degrees down) and uppercut (60 up) on the crouch's legs, and the rising stick strike (35 up) standing. The jab's whole arm, shaded sleeve to stick, is turned; the sweep and strike have the whole shoulder and upper arm inpainted, the uppercut only the shoulder and waist joins.
   - **The two-stick smash is pieced:** his jumping stick strike's upper body (`0x2a44`), hunched forward with both sticks driven down before him, on his stance's legs. `0x2a44` is the top of his jump with the arms forward, so its waist sits on his stance's belt as the crouch's does; only the waist is inpainted. It plays from his ROM wind-up with both sticks over his shoulder (`0x2988`).
   - **The get-up's rising pose is pieced:** his jump take-off squat's upper body (`0x29e8`), bent forward with a stick in each hand, set 16 rows lower and 8 further forward than the crouch's upper body, so his hands come down on his knees; only the waist is inpainted. It stands 56 pixels tall.
   - **Free, from his ROM art:** the stick hook (AP2) as his jab's strike (`0x2980`) hooking behind the neck, its wind-up (`0x292c`) as the pull and `0x2988` as the fling; the thrown set (his standing hit `0x2aa8`, his airborne hit `0x2aac` turned a quarter and a half, lying `0x2b69`).
3. **His kick buttons, as Mike's (the maintainer, 2026-11-07).** AP2 gives him three stick moves at each height; with his ROM streams he needs two new ones at each.
   - **Standing:** LK, the low stick poke, is his ROM stick jab; MK (the rising stick strike) and HK (the two-stick smash) are new.
   - **Crouching:** MK, the low two-stick thrust, is his ROM low thrust (`0x12628`), which AP2's line describes exactly and which also stays his crouching punch; LK (the sweep) and HK (the uppercut) are new.
   - Each new strike is one pose between ROM poses, as his jab strikes in one.
4. **A frame's x-offset is its own byte.** A frame record's byte 7 (`dx`) moves the fighter: the engine adds it to his stored position (`place_frame`), so it accumulates, as his barrage's 28 then 4 a frame and his hit streams' knock-back show. Byte `0x1b` is a draw offset, added to the grid's x and taken back off at the next frame. Earlier recipes placed ROM frames by `dx`, as approved, and Eagle's jab poses follow them. His wind-ups also have a byte `0x1b`: `0x2988` is drawn 24 back and `0x298c` 8 forward, so their anchors carry it (`[-56, -96]`, `[-24, -96]`). The maintainer approved the wind-up drawn 24 back, leaning away.
5. **Round two (the maintainer's round-one reasons).**
   - The crouching sweep: "Arm looks pasted on." Round one inpainted only a small join at the shoulder, and the shoulder's top stuck out as a ledge.
   - The rising strike: "the upper part of the arm (bicep/tricep) does not look as full as it should be, looks malformed." Round one's turned sleeve had its underside trimmed by hand under the armpit, which left the upper arm thin.
   - The smash: "between the forearms should look transparent." Where the two forearms meet, his ROM pose has their outlines side by side, a maroon band 3 to 6 pixels wide; it read as a filled gap.
6. **What worked, for the next fighters.**
   - **Rebuild a turned arm's shoulder over his own shaded sleeve.** The jab's sleeve, kept whole down to its own dark underside and outlined where the armpit is cut away, was turned with the arm and set 3 pixels back into the torso as the guide. The whole shoulder cap and upper arm were inpainted, with his head and the torso's front edge outside the mask. The arm then grew out of the shoulder (AP20's lessons, again).
   - **Keep the upper arm its full width.** Trimming a turned limb by hand along a row cuts it thin at an angle. Cut it in the source, parallel to the limb, at the width the ROM draws it.
   - **Open the gap between two limbs.** Where two outlines touch, keep one outline pixel on each limb and make the pixels between them transparent, by hand, so the background shows through.
   - **Two seeds a retry.** Both retries were sent with two seeds; one of each was cleaner.
7. **His set comes in halves.** His recipe's `still_to_come` names his air set and specials (AP24, #188), and the game logs `eagle keeps his ROM art: his set is not finished, with his air set and specials (AP24, #188) still to come`. With the line removed, the local-pack test loads his ground set: `eagle (10 moves; 7 ROM poses, 2 rebuilt, 8 generated)`.
8. **Cost.** 59 generations in 11 inpaintings (`inpaint_image_pro_flash`, 5 or 6 each), for 6 kept generated poses, about 10 each; the crouching block and hit were derived from the crouch, and the smash's round two was by hand. By move: the rising strike 18, the sweep 15, the crouch 10, the uppercut 6, the smash 5, the get-up 5. PixelLab's count went from 336 to 241 left in the month from 2026-10-27; the other 36 were other sessions'.
9. **Compromises.** His crouch, crouching block and hit, the three turned-arm strikes, the smash and the get-up (pieced), and his throw and thrown set (ROM stand-ins) are listed in [`docs/planning/art-upgrades.md`](../planning/art-upgrades.md).

### AP24 (Eagle's air set and specials, #188)

1. **His air set and specials, and where each pose comes from.** Twelve new poses make seven moves; with his ground set, 29 poses in 17 moves.
   - **Free, from his ROM art:** the top of the jump, the tuck all three of his jumps pass through (`0x2a40`, drawn 4 back), shown 48 pixels up; his jumping attacks start and end on it. His jumping HK, AP2's two-stick downward smash, is his ROM jumping stick strike (`0x2a44`, drawn 24 forward), both sticks driven down before him.
   - **Manchester Black and St. Andrew's Green, AP2's stand-ins:** his spinning-sticks barrage (stream `0x124d8`: the wind-up `0x2988`, then `0x2b00`, `0x2a4a`, `0x2b08`, `0x23a8` twice round) and his leaping stick strike (stream `0x129bc`: the squat `0x29e8`, the rise `0x29ec`, the top of the jump, `0x2a44`, and down again), as the ROM plays them. They are in the catalogue so SF1+ can re-time them and move him; nothing is new.
   - **Pieced on the tuck's legs, nothing generated:** the jumping high strike (the approved rising strike's upper body, the stick swung up 35 degrees), the jumping stick jab (his stick jab, `0x2980`, its own ROM arm level) and the jumping stick poke, his LK (the approved crouching sweep, the stick driven 50 degrees down and forward). The crouch's thighs and knees are the tuck's own pixels, so the crouch's shins, hems and shoes were swapped for the tuck's hanging shins and feet with no seam; the jab's waist band is the approved sweep's, the same jab body on the same thighs.
   - **Inpainted at the waist only:** the jumping stick swing, his MK, AP2's horizontal stick swing: the arms-spread pose of his spinning sticks (`0x2b00`) down to its belt on the tuck's legs, its belt on his stance's as the crouch sets it.
   - **Stay his ROM streams, not in the catalogue:** his jumps and his three leaping stick strikes (`0x126f8`, `0x1285a`, `0x129bc`).
2. **His buttons in the air, the builder's mapping (the maintainer kept it).** AP2's kick-button lines give the LK poke, the MK swing and the HK smash; the punches take the two slots left at Ryu's five: the high strike from a straight-up jump and the stick jab from forward and back jumps. Sagat's diagonal punch, down and forward, was not used for the punch, because it is the poke's shape. Each new strike is one pose between two top-of-jump poses, as his stick jab strikes in one.
3. **What worked, for the next fighters.**
   - **A crouch built on the jump tuck's thighs gives the air set for free.** Every pose built on that crouch (the sweep, the rising strike's upper body) goes into the air by swapping only what lies below the knees for the tuck's hanging feet, since the thighs and knees are already the tuck's.
   - **Lay an approved pose's waist again rather than inpaint it.** Where the same upper body sits on the same thighs, the approved waist band joins it with no generation.
   - **His own attack frames hold the bigger moves.** The barrage's arms-spread pose read as a horizontal swing on the tuck's legs, and needed only its waist joined.
4. **Review.** One round: the maintainer approved all twelve new poses at the first round (2026-11-07), and of Manchester Black said: "manchester black looks amazing". The page holds 29 approved poses for his whole set (`art/review/eagle.json`).
5. **His set is finished and loads.** The recipe's `still_to_come` is gone. On this machine the game's own loader logs `eagle (17 moves; 15 ROM poses, 2 rebuilt, 12 generated)` beside the other ten fighters, with no fighter kept on his ROM art but Adon (his air set, AP26, still to come); the bank spans 11,280 tile slots.
6. **Cost.** AP24 spent 5 generations in one inpainting (`inpaint_image_pro_flash`): the stick swing's waist. The high strike, stick jab and poke were derived from his approved ground-set poses. Eagle's whole set took 64 generations for 7 kept generated poses, about 9 each. PixelLab's count went from 220 to 215 left in the month from 2026-10-27. The approved pack is backed up (`ed01b07`).
7. **Compromises.** His pieced jumping attacks (the jab's upper body on the tuck's legs, the swing's shorter torso), the jumping two-stick smash (his ROM strike, like his standing smash) and Manchester Black and St. Andrew's Green (ROM stand-ins) are listed in [`docs/planning/art-upgrades.md`](../planning/art-upgrades.md).

### AP25 (Adon's ground set, #189)

1. **His ROM streams.** His move table is at `0x17f7c`: move 1 holds his stance (`0x3060`), move 4 his block (`0x312c`, a Muay Thai check with the front knee raised), moves 5 to 8 are empty, so he has no crouch. His hit and knock-out tables are at `0x19848` (13 streams) and `0x1987c` (11), 4 bytes an entry. His attacks are the jab (`0x132dc`), body punch (`0x1336c`), low kick (`0x133fc`), the flying kick (`0x1348c`, `0x1366e`) and the knee-and-kick combination (`0x13850`, `0x1398c`). His fifteen pens are all different colours.
2. **His ground set, 15 poses in nine moves, approved after three rounds (the maintainer, 2026-11-07).**
   - **The crouch is pieced:** his stance (`0x3060`) down to its belt, upright with both fists on guard, on the shorts and front thigh of the top of his jump (`0x3248`), whose belt lines up with the stance's, set on his stance's own ankle wraps, feet and shadow. Under the front knee stands his stance's front lower leg as one piece, the shaded calf running into the wrap (item 4). He stands 70 pixels tall against his stance's 95.
   - **His ROM upper bodies on the crouch's legs, nothing generated:** the crouching block (his standing block, `0x312c`, its lifted knee left out), the crouching hit (his standing hit, `0x3180`) and the crouching punch (his jab, `0x30c4`, whose belt is his stance's). An inpainting of the block's waist changed 1 pixel of 166, so it was kept as pieced.
   - **Inpainted only at the joins:** the crouching kick (his low kick's leg, `0x31e4`, turned 27 degrees up from the crouch's hip) and the sweep (item 5).
   - **The get-up's rising pose is pieced:** his hit recovery's hunched upper body (`0x324c`), head down and his front hand on his front knee, on kneeling legs (the seat 7 pixels below the crouch's, the rear knee down, the shin back along the floor); the waist, hip and rear leg inpainted. It stands 56 pixels tall.
   - **Free, from his ROM art:** the clinch knee (AP2) as his jab's reach (`0x30c4`), the knee strike of his knee-and-kick combination (`0x3128`), mirrored so the knee drives toward the opponent (no armband shows in it), and his body punch's thrust (`0x3304`) as the shove; the thrown set (his standing hit `0x3180`, his airborne hit `0x32a0` turned a quarter and a half, lying `0x32e8`), played mirrored as the shove sends him away (AP4).
   - **Poses per move.** The crouching punch and kick strike in one new pose each, as his jab and low kick do. The throw has three poses before his stance, the thrown set four, the get-up four.
3. **Round one (the maintainer's reasons).** Nine of 15 approved: the crouching kick, the throw, the thrown set, lying and stance.
   - The crouch: "calf/foot join on the front leg is bad." The knee sat directly on the ankle wrap, with no calf between them.
   - The crouching block, hit and punch: "Same as the crouch."
   - The rising pose: the same calf and foot join.
   - The sweep: "Too like the crouching kick", "Doesn't read as a sweep." It was the kick's leg at another angle on the same crouch.
4. **The calf rebuilt with his own shaded calf as the guide.** Under a horizontal thigh at his crouch's height there was no room for a shin. The front thigh was turned 8 degrees up about the hip and 4 rows of the wrap's plain band were left out, so the knee leaves room for the calf and he stays 70 pixels tall. His stance's front lower leg was laid under the knee as one piece, the calf's last 10 rows running into the wrap's top outline, and the whole calf from the knee to the wrap was inpainted over it, with the hip under the thigh. Turning the thigh 25 degrees instead made room too, but the knee stood high and read as a raised knee. The block, hit and punch were laid again on the new legs, and the rising pose took the same leg wherever its hand left it showing. Approved at round two.
5. **The sweep as its own shape.** A second angle of the crouching kick's leg read as the same kick. The sweep now drops onto the rising pose's kneeling legs, rear knee down, and leans in with his body punch's upper body (`0x3304`); his low kick's leg is turned nearly flat along the floor, its calf lengthened 8 pixels by repeating a run of its own columns, the foot out at ground level. Only the waist and hip were inpainted, and the belt's studs were given back his crouch's own belt rows by hand. It stands 61 pixels tall, as Ryu's sweep (`0x054b`) reads. Approved at round two.
6. **The thigh outline a laid-on upper body can hide.** The jab's upper body was laid over the crouch's rows down to its belt, and its own rows there were empty or its belt's end, exactly where the crouch's front thigh starts: the maintainer, round two, "the top of his leg's outline seems to disappear compared to the crouch." Putting back the crouch's 19 pixels where the jab's torso and arm do not pass, by hand, fixed it at round three.
7. **What worked, for the next fighters.**
   - **A calf needs room under the knee.** A deep squat's knee sits a shin's length off the floor. Lift the knee a few degrees, or shorten a plain wrap or hem, then lay the fighter's own shaded lower leg as the guide and inpaint the whole calf, not the seam.
   - **A sweep is its own pose.** Drop the body onto a kneel, lean it in, and lay the leg long and flat along the floor; a turned crouching kick reads as the kick.
   - **Compare a laid-on upper body with the pose under it along every edge it crosses.** Rows the donor leaves empty take the base pose's outline away with them.
   - **An inpainting can return its source.** With the guide already right, the block's waist and the first calf came back nearly unchanged (1 and 2 pixels); the pieced pose was kept and the call recorded as not kept.
8. **His set comes in halves.** His recipe's `still_to_come` names his air set and specials (AP26, #190), and the game logs `adon keeps his ROM art: his set is not finished, with his air set and specials (AP26, #190) still to come`. With the line removed, the local-pack test loads his ground set: `adon (9 moves; 5 ROM poses, 3 rebuilt, 7 generated)`.
9. **Cost.** 46 generations in 9 inpaintings (`inpaint_image_pro_flash`, 5 or 6 each), for 4 kept generated poses, about 12 each; the crouching block, hit and punch were derived from the crouch, and round three was by hand. By move: the crouch 15, the sweep 11, the get-up 10, the crouching block 5 (not kept), the crouching kick 5. PixelLab's count went from 288 to 215 left in the month from 2026-10-27; the other 27 were other sessions'.
10. **Compromises.** His crouch, crouching block, hit, punch and kick, the sweep and the get-up (pieced), and his throw and thrown set (ROM stand-ins) are listed in [`docs/planning/art-upgrades.md`](../planning/art-upgrades.md).

### AP26 (Adon's air set and specials, #190)

1. **His air set and specials, and where each pose comes from.** Eleven new poses make six moves; with his ground set, 26 poses in 15 moves.
   - **Free, from his ROM art:** the top of the jump, the tuck all three of his jumps and his flying kick pass through (`0x3248`), shown 48 pixels up; his new jumping attacks start and end on it. His jump's take-off squat (`0x3240`) and rising pose (`0x3244`).
   - **Rebuilt:** the jumping punch, his jab's upper body (`0x30c4`) on the tuck's shorts and legs, laid as his crouching punch lays it on the crouch: the crouch's shorts are the tuck's, 3 pixels right and 2 down in his stance's coordinates.
   - **Inpainted only at the joins:** the diagonal jumping punch (his flying kick's dive arm, `0x330a`, on the jab's shoulder), the heavy jumping kick (his roundhouse, `0x318a`, in the air with its standing leg turned 45 degrees back, the hip inpainted) and the Rising Jaguar's knee (his rising pose with the front thigh turned 60 degrees up and the shin turned to hang under the knee, the hip, thigh and knee inpainted).
   - **The jumping kick** (item 3): the tuck's rear wrap and foot kept in place, the front leg replaced by his approved crouching kick's leg from the same hip, and the rear shin it hid redrawn by hand from his stance's calf.
   - **The Jaguar Kick, AP2's stand-in:** his ROM flying kick (stream `0x1348c`) as the ROM plays it, in the catalogue so SF1+ can re-time it. AP2 asked whether it really leaves the ground: it does, its frames rise 12, 6 and 2 pixels before the dive and 16 more into it. It dives with the fist driven down, somersaults (`0x32a4`) and drives his feet out (`0x32a8`). The same stream stays his diagonal jumping kick.
   - **The Rising Jaguar:** take-off squat, rising pose, the knee, the rising pose and the squat again; one new pose where AP2 proposed three.
2. **Poses per move.** Each jumping attack is one strike between two top-of-jump poses, as his jab, low kick and roundhouse strike in one. His jumps and flying kick stay his ROM streams.
3. **The jumping kick took three rounds.**
   - Round one laid the crouching kick's leg in place of the tuck's front thigh and kept the tuck's folded front calf, wrap and foot under it as the tucked leg, taking the rear wrap and foot away. The maintainer: "It looks like it switches legs; the rear leg (from jump_top) becomes the front leg in jump_kick."
   - Round two kept the tuck's rear wrap and foot exactly where they hang and took the whole front leg away, so the kick grows from the front hip; the rear shin the front leg had hidden was hinted with his rising pose's calf, turned, and the hip, thigh and rear shin inpainted. The inpainting kept the guides almost unchanged (6 pixels). The maintainer: "shin/calf area on rear leg looks weird or lumpy."
   - Round three, by hand: only the rear shin, from under the kicking thigh to the top of its wrap, replaced by his stance's own front calf (`0x3060`, rows 66-79), taken down to the 9 visible rows along the leg and sheared a pixel a row into the wrap, behind his shorts and the kicking thigh. Approved.
4. **What worked, for the next fighters.**
   - **Flip between the top-of-jump pose and the strike before review.** A strike that keeps the wrong leg's lower half reads as the legs swapping. Keep the rear leg's own pixels exactly where the jump pose has them and grow the strike from the front hip; an overlay of the two poses shows at once which pixels moved.
   - **A turned limb piece reads as lumpy.** The rising pose's calf turned 40 degrees came back blotchy and the inpainting kept it. A straight ROM calf, cut to length along the leg and sheared a pixel a row, keeps his outline, width and shading; let the kicking thigh and shorts hide the knee.
   - **One fighter's crouch and tuck share coordinates.** Because his crouch is built on the tuck's shorts, his approved crouching poses' upper bodies and legs lay on the tuck at the same offsets, with no join.
   - **Check AP2's conditions in the ROM.** The flying kick's frames settled that its stand-in holds, so no new Jaguar Kick was drawn.
5. **Review.** Three rounds. Round one approved ten of the eleven new poses (the maintainer, 2026-11-07); the jumping kick was approved at round three. The page holds 26 approved poses for his whole set (`art/review/adon.json`).
6. **His set is finished and loads.** The recipe's `still_to_come` is gone. On this machine the game's own loader logs `adon (15 moves; 11 ROM poses, 4 rebuilt, 11 generated)` beside the other ten fighters on main, with no fighter but Eagle (AP24, not yet merged) kept on his ROM art; the bank spans 11,235 tile slots.
7. **Cost.** AP26 spent 25 generations in 5 inpaintings (`inpaint_image_pro_flash`, 5 each): the jumping kick 10 (round two, two seeds), the diagonal punch, heavy kick and Rising Jaguar 5 each. The jumping punch, round one's jumping kick and round three were by hand. Adon's whole set took 71 generations for 8 kept generated poses, about 9 each. PixelLab's count went from 215 to 190 left in the month from 2026-10-27.
8. **Compromises.** His pieced jumping attacks (the punches on his jab's upper body, the light kick on his crouching kick's leg, the heavy kick his roundhouse in the air), the Jaguar Kick (a ROM stand-in) and the Rising Jaguar (one new pose) are listed in [`docs/planning/art-upgrades.md`](../planning/art-upgrades.md).
