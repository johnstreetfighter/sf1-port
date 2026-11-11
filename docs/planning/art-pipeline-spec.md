## Problem Statement

The SF1+ project gives all twelve **tournament fighters** complete movesets in the real 1987 fight, but ten of them have no art to move with. In 1987 the bosses were CPU-only. Each holds 13 table moves against Ryu's 31, plus six to nine attacks his CPU picks from:

- Sagat has no crouch and no jumps.
- Joe, Lee, Gen, Birdie, Eagle and Adon have no crouch.
- Geki has no blocks; Mike and Eagle have no kicks.
- Only Ryu, Ken, Retsu and Geki have a crouching hit pose.
- No fighter, Ryu included, has a throw or a get-up.

Against Ryu's level of art, the bosses lack about 254 poses before any specials, where the ROM holds 442. No two fighters share any art, so nothing can be borrowed between them as it stands.

The new art has to pass for 1987 Capcom art next to the originals: the 384×224 screen, the fighter's own 15 colours plus transparent, 16×16 tiles, drawn the way the board draws fighters. It has to come from the ROM's own art, not fan-made sprite sets, and stay out of the public repository, which ships no Capcom material. It also has to fit within a PixelLab plan of 2,000 generations a month. And the player must never see a pose they didn't approve.

## Solution

An offline **art pipeline** builds every **pose** SF1+ needs, one fighter at a time:

1. **Free first.** Moves are made from the fighter's own ROM frames where possible: reversed, mirrored, turned 90°, re-timed or pieced together. These are stored as **recipes** and rebuilt from the player's ROM at launch.
2. **Generation for the rest.** PixelLab draws from the fighter's own frames, which set his look; another fighter's ROM frames may guide a pose. The result is held to his exact palette. Drifted faces get his real ROM head pasted back, and the rest is cleaned by hand.
3. **Review.** The builder chooses the best candidate for each pose. The user approves each fighter's set on a local review page, where every move plays in a real fight frame next to his ROM art. A rejected free version is generated instead; a rejected generated pose is tried again, up to three rounds, then comes back to the user to decide.
4. **Packing.** Approved poses are reduced to the fighter's palette and cut into tile rectangles in an **extended sprite bank**, above the ROM's 14,336 tiles, which only overhaul modes load. The game's own renderer and pose code draw them. The generated pixels live in a local **art pack** beside the ROM set, backed up to a private repository.
5. **Handover.** A per-fighter **pose catalogue** tells the SF1+ project which poses make up each move, and where they are.

Faithful 1987 never loads any of it. A fighter's new poses reach the game as soon as his set is approved. A missing or broken art pack leaves the bosses on their ROM moves, with the reason in the log.

## User Stories

### The art itself

1. As a player of SF1+, I want every boss to have a crouch, a crouching block and a crouching hit pose, so that he can crouch like Ryu instead of standing through every low attack.
2. As a player of SF1+, I want Sagat to jump, so that the tallest boss is not the only fighter stuck on the ground.
3. As a player of SF1+, I want Geki to have a standing block, so that he defends like everyone else instead of only teleporting.
4. As a player of SF1+, I want each boss's missing standing, crouching and jumping normals at Ryu's level of distinct animations, so that all twelve fighters have complete six-button movesets.
5. As a player of SF1+, I want every fighter, Ryu and Ken included, to have a ground throw and a being-thrown reaction, so that close-range play has a throw.
6. As a player of SF1+, I want every fighter to have a get-up, so that knockdowns end in rising, not a jump cut from lying to standing.
7. As a player of SF1+, I want each boss's SF1+ specials drawn as his own moves, so that his specials look like him.
8. As a player, I want Mike to stay a boxer and Eagle a stick fighter, with their kick buttons starting extra punch or stick moves, so that neither suddenly kicks.
9. As a player, I want new moves to have as many poses as the fighter's own ROM moves of the same kind, so that a new kick is as smooth as his old ones.
10. As a player, I want new poses in the fighter's exact 1987 colours, outline and shading, so that I cannot tell new art from old at game scale.
11. As a player, I want every boss to keep his own face, build and outfit in his new poses, so that he looks like the same person in every move.
12. As a player, I want new poses never to show soft gradients, stray colours or mushy outlines, so that nothing reads as AI-made.

### Where the art comes from

13. As the user, I want new art made only from the 1987 ROM's sprites, so that no fan's work or other game's sprites end up in mine.
14. As the user, I want moves that can be made from a fighter's own frames made that way first, so that the most authentic option is always tried first and generations are saved.
15. As the user, I want a rejected free move generated instead, so that a stiff pieced move never ships just because it was cheap.
16. As the user, I want a pattern in my rejections to change the default for that kind of move, so that I do not keep rejecting the same kind of pieced move.
17. As the user, I want another fighter's ROM frames allowed to guide a pose, such as Ryu's sweep guiding Sagat's, so that bosses can get poses none of their own frames show.
18. As the user, I want Ken's new poses made from Ryu's approved ones recoloured, with Ken's differences fixed by hand, so that Ken costs half and stays a near-copy of Ryu as in 1987.
19. As the user, I want a drifted face replaced with the fighter's real ROM head before I see it, so that I judge the pose, not the AI's face.
20. As the user, I want each throw and each special that has no Capcom original described in one line for my approval before anything is drawn, so that no generations are spent on a move I would not have chosen.
21. As the user, I want one being-thrown set per fighter that every throw uses, so that throws do not multiply the work by every pair.

### Review

22. As the user, I want to approve each fighter's whole set on one review page, so that I judge him as a whole.
23. As the user, I want every move on the review page playing in a real fight frame next to the fighter's ROM art, so that I judge it at game scale, against the real backgrounds and opponents.
24. As the user, I want to reject any single pose for another try, so that one bad pose does not cost a whole move.
25. As the user, I want a pose still rejected after three rounds of generation and repair brought back to me, so that I decide whether to take the best attempt, simplify the move or drop the pose, and no single pose quietly eats the budget.
26. As the user, I want the builder to choose among candidates and assemble the moves, so that I am not sorting roughly a thousand images.
27. As the user, I want the review page to be a local file, so that the generated art is never published.

### Budget

28. As the user, I want the work to stay within my PixelLab plan's monthly allowance, however long that takes, so that it never costs more than I pay now.
29. As the user, I want at least 10% of each month's allowance left unused by this project, so that I can use PixelLab for other things.
30. As the user, I want generations recorded per move, so that I can see where the budget went.
31. As the user, I want the pilot fighter's real generation rate reported before the other nine begin, so that I can revise the timeline.

### Where it lives

32. As the user, I want no Capcom-derived pixels in the public repository, so that it keeps its rule of shipping no Capcom material.
33. As the user, I want the generated poses in a local art pack beside my ROM set, so that the game finds them where it finds the ROMs.
34. As the user, I want the art pack backed up to a private repository after each approval, so that a lost machine does not cost months of work and my approvals.
35. As the user, I want recipes, generation settings, checksums and review decisions in the public repository, so that everything except the pixels is versioned and reviewable.
36. As a player, I want recipe poses rebuilt from my own ROM at launch, so that they work without the art pack.

### In the game

37. As a player, I want new poses drawn by the game's own sprite code, so that mirroring, draw order and hit flashes behave exactly as in 1987.
38. As a player choosing Faithful 1987, I want none of the new art loaded, so that the faithful edition stays byte-identical.
39. As a player, I want each fighter's new poses in the game as soon as his set is approved, so that finished fighters do not wait for the rest.
40. As a player without the art pack, I want the bosses to fall back to their ROM moves, so that the game still runs.
41. As a player with a broken or outdated art pack, I want the game to fall back the same way and log why, so that a bad file never crashes the game.
42. As a player, I want the log to name the art pack it loaded, or why it did not, so that a bug report says which art was in play.

### For the SF1+ project and later projects

43. As the developer of SF1+, I want a pose catalogue per fighter listing each move's poses in a suggested order, so that I can write move streams without inspecting tiles.
44. As the developer of SF1+, I want each pose's place in the extended sprite bank (base tile, rows, columns, entry size), so that a move stream can point at it directly.
45. As the developer of SF1+, I want an anchor for every pose that lines it up with the fighter's ROM stance, so that new moves do not jitter against old ones.
46. As the developer of SF1+, I want every pose already checked against the engine's pose rules (a tile rectangle, at most 24 sprite entries, one palette), so that no approved pose turns out undrawable.
47. As the developer of later projects (era costumes, endings, bonus challengers), I want the pipeline to take a pose list and produce poses the same way, so that new art does not need new tooling.
48. As the developer, I want recipes rebuilt identically every time, so that the same ROM always gives the same tiles.
49. As the developer, I want the order of work to start with Ryu, Ken and the throw and get-up set, then Sagat end to end as the pilot, so that the whole path is proven on the largest gap before the other nine.

## Implementation Decisions

- **Sources (ADR-0006):**
  - New poses come only from the 1987 ROM's sprites.
  - Fan-made M.U.G.E.N sprite sets, other games' sprites and traced references are never used as pose references or art, unless the artist gives permission.
  - Fan-made move data (timings, hitboxes, move lists, descriptions of what a move does) may be used as a reference. That includes the repository's M.U.G.E.N research notes, for describing specials and throws.
  - Supersedes ADR-0003's use of fan sprite sets.
- **The extended sprite bank (ADR-0007):**
  - New poses are tiles above the ROM's 14,336 sprite tiles (`0x3800`), loaded only in overhaul modes.
  - They are drawn by the unchanged renderer and pose-grid code.
  - Tiles are 16×16, 4-bit planar, 128 bytes each, in the fighter's sprite palette with **pen 15 transparent**. The existing offline asset-pack module uses pen 0, so the new format follows the renderer.
  - A pose is a rectangle of tiles laid out 16 tiles wide, with a base code, row and column counts, and 16-pixel or 32-pixel entries. It uses at most 24 sprite entries.
- **Before the bank is enabled**, it must be shown that the 1987 game never draws a tile code at or past `0x3800`. Today a code past the ROM wraps modulo the tile count; with a larger sheet it would not. The renderer's tile-code swizzle (bits 3 and 4 in two of every four 8-tile groups) applies to the new codes too.
- **The art-loading seam:**
  - **In:** the ROM set and the art pack.
  - **Out:** the game's graphics with the extended sprite bank filled, plus each fighter's pose catalogue. Or the ROM graphics alone with a fallback reason: missing, broken, checksum mismatch, out of date with the pose list.
  - **Which fighters get new art:** a fighter's new poses are used only when his whole approved set is present.
  - **Where it looks:** the host looks the art pack up in the same folders as the ROM set, and logs what it loaded.
- **Recipes:** per pose, the fighter's ROM tiles and the transform (mirror, 90° turn, crop, offset, piecing order). They are deterministic and committed.
- **Generated poses:**
  - **Tool:** PixelLab Pro, with the fighter's own ROM frames as identity references, another fighter's as pose guides where needed, and his exact palette passed as the palette constraint. Hard alpha.
  - **Faces:** the real ROM head is pasted back where a face drifts. Seams are cleaned by hand with PixelLab's free pixel tool, with paid inpainting as the fallback.
  - **Recorded per pose:** references, prompts, canvas, seed, cost and checksum, committed.
  - **The pixels** go in the art pack only.
- **Ken** takes Ryu's approved new poses recoloured into his palette, with his own differences fixed by hand.
- **Scope per fighter,** from the art inventory, at Ryu's level of distinct animations:
  - crouch, crouching block and crouching hit;
  - a standing block for Geki;
  - a jump arc for Sagat;
  - the missing standing, crouching and jumping normals;
  - a ground throw and one generic being-thrown set;
  - a get-up;
  - the SF1+ specials.

  Mike and Eagle get no kicks; their kick buttons map to extra punch or stick moves. Ryu and Ken need only a throw, a thrown set and a get-up.
- **The specials:**
  - **From the README:**
    - Lee: Zesshouohken, Double Kick;
    - Gen: Hyakurenko, Gekiro;
    - Birdie: Bull Head, Bull Horn;
    - Eagle: Manchester Black, St. Andrew's Green;
    - Adon: Jaguar Kick, Rising Jaguar;
    - Sagat: high and low Tiger Shot, Tiger Knee.
  - **From the earlier batch's SF1+ list:**
    - Retsu: Kiai Shot;
    - Geki: Shuriken, Teleport Strike, both largely in his ROM art;
    - Joe: Rolling Sobat, Dynamite Straight;
    - Mike: Bison Rush, Iron Upper.
  - **Stand-ins:** existing streams may stand in where they fit (the art inventory names candidates).
- **Smoothness:** each new animation has as many poses as the fighter's own ROM moves of the same kind.
- **Process:**
  - Throws and specials without a Capcom original are described in one line and approved before generation.
  - A rejected free move is generated.
  - A pose rejected after three rounds returns to the user.
  - Each fighter goes into the game when his set is approved.
- **Review page:** a local HTML file, never published. Each move plays in a real fight frame (rendered by the game) next to the fighter's ROM art, with per-pose reject.
- **Budget:** at most 90% of each month's PixelLab allowance (2,000 generations, Tier 1). The estimate is 2,500 to 4,000 generations in all, two to three months, perhaps four. The pilot measures the real rate. The user has accepted that PixelLab's Pro outputs may be used for its training, with no opt-out.
- **Backup:** a private art-pack backup, created when the first generated pose is approved and updated after each approval.
- **Order:**
  1. The pipeline itself, with recipes and the bank.
  2. Ryu, Ken and the throw and get-up set (mostly free).
  3. Sagat end to end as the pilot.
  4. The other nine in the order SF1+ needs them.
- **The pose catalogue,** committed per fighter: each move by name; its poses in a suggested order; each pose's source (recipe, or generated with its art pack entry); its place in the bank; and an anchor against the ROM stance.

## Testing Decisions

- **A good test** checks only what the seam returns or what the renderer draws: pixels, fallback reasons, catalogue contents. Never the pipeline's internals.
- **The art-loading seam** is the one new seam. Its tests:
  - recipes rebuild identically twice from the ROM;
  - every pose uses only its fighter's palette pens, with pen 15 transparent, and fits 24 sprite entries;
  - the art pack's checksums match;
  - a missing, corrupt, mismatched or outdated pack falls back to the ROM graphics with the right reason;
  - a fighter whose set is incomplete keeps his ROM art;
  - the pose catalogue lists every pose with its bank place and anchor.
- **Drawing,** through the existing renderer: a pose placed through the game's own pose-grid code draws exactly the approved image, pixel for pixel, facing right and mirrored facing left.
- **Faithful 1987 unchanged:**
  - every existing comparison fixture passes byte for byte without the bank;
  - with the bank loaded, every ROM pose draws exactly as before. This covers the tile-wrap and swizzle questions.
- **Prior art:**
  - the front end's headless session tests (`front_end_*`), which render through the real engine and compare pixels;
  - the VS-bust tests, which decode ROM art and compare it to the native machine and MAME captures;
  - the shipping tests' ROM lookup and fallback tests;
  - the coverage ledger and comparison runner, for the Faithful 1987 baseline.
- **Not under test:** the PixelLab calls and the review page. The user's eye is their check.

## Out of Scope

- Animation timing, move streams, hitboxes and frame data: the SF1+ project builds those on the pose catalogue.
- Endings, era costumes, win-screen art, lose poses, a separate diagonal-jump somersault, "command heavy" moves, effects and projectiles: later projects, through this pipeline.
- New sound.
- Palette variants for mirror matches and era costumes.
- Any change to Faithful 1987 or to engine behaviour.
- Bonus challengers' art (spec #113's project).

## Further Notes

- **Prior work:**
  - The design is approved in `docs/design/2026-11-02-art-pipeline-design.md`, with the grilling in its section 9.
  - The starting map is `docs/reference/art-inventory.md`. It also found errors in four other reference docs, to correct when those docs are next touched.
  - ADR-0006 and ADR-0007 record the two hard-to-reverse decisions.
  - The glossary's Art pipeline terms are pose, recipe, art pack, pose catalogue and extended sprite bank.
- **The spike (2026-11-02, 20 generations)** made a crouch and a crouching kick for Sagat:
  - Every candidate stayed inside his 14 colours.
  - Three of four kick candidates came back on one consistent body.
  - About one candidate in four was unusable, and faces drift.
  - In a real fight frame the poses passed as 1987 art.
- **Project order:** this is the second of the seven overhaul projects the user set on 2026-11-02, after the front end (spec #150) and before SF1+ on the real engine.
