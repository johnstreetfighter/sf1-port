# MUGEN collection survey: which art gaps it can fill

Surveyed 2026-11-20 for the maintainer, who has been told we may use sprites from `C:\Users\George\Downloads\sf1-mugen collection` with credit. Nothing has been imported or committed. This report is for choosing pieces, if any.

**Short answer:** the collection fills **none of our largest gaps** (throws, being thrown, get-ups, dizzy). It has no such art. Most of its fighter sprites are the same 1987 ROM poses we already use. The new poses it does have are crouches and jumping attacks pieced from ROM parts, the method our pack already uses, and our approved versions are at least as good. What it adds that we lack: one complete demade fighter (Cody, for bonus roster wave 3), and per-fighter ending scenes with some poses drawn for them. Both mix in sprites from other Capcom games.

## 1. What the collection is

One folder, `SF1/`: **"Street Fighter by Falchion22, made with Mugen"**, a full-game M.U.G.E.N conversion, updated for M.U.G.E.N 1.0 and rehosted by zesensei "with author permission" (`sf1-readme.txt`). It ships with `mugen.exe` and Elecbyte's DLLs. They were not run for this survey, and nothing else from the folder was run either. A `mugen.log` shows someone ran it on this machine once.

| Item | Contents | Author (from the files) |
| --- | --- | --- |
| 12 fighters, `chars/A-<Name>/` | `.def`, `.air`, `.cmd`, `.cns`, `.snd`, one `sff.sff` (SFF v1.01, 117–157 sprites each), 1–4 `.act` palettes, an `ending.def` + `ending.sff` | Falchion22. Geki: "CWDEVINE & Falchion22". Mike's line reads "Falchion22 &&" (a second name cut off). |
| Cody, `chars/space2/` | A complete hidden fighter (216 sprites, knife states, ending); its folder is hidden from the select screen | Falchion22 |
| Bonus stage, `chars/sfbonus/` | Board-break target as a character (tiny SFF) | DCelso |
| 13 stages, `stages/sf-*.def/.sff` | The 1987 backgrounds plus a dojo; `stage0` is Elecbyte's sample training room | Kitsune Sniper (dojo and bonus stage unsigned) |
| Screen pack, `data/` | `system.sff` (title, select, 27×27 select icons), `fight.sff` (life bars), `fightfx.sff` (common hit sparks), `sf1intro.sff` and `sf1ending.sff` (384×224 story frames), `sf1.sff`, `warning.sff` | Falchion22 ("motif author") |
| Music, `sound/*.mp3` | 21 tracks: per-stage themes, select, vs, credits | Unsigned; ripped game music |
| Portraits | Group 9000 in each fighter SFF: a 25×25 icon and a ~102×103 portrait | Falchion22 |

No licence file. The only permission statement is zesensei's: hosted with Falchion22's permission. The SF1 sprites themselves are Capcom's, as are ours.

Decoded with a stdlib-only read-only reader in the session scratchpad (`tools/sffdump.py`, `overview.py`). Contact sheets are under `…\scratchpad\ov\` and `…\scratchpad\mugen-out\`.

## 2. Our gaps (from `art-upgrades.md` and the pack)

- **Throws and being thrown:** every fighter, all pieced from ROM poses.
- **Get-ups:** every fighter, from lying, fall, crouch and half-crouch poses.
- **Dizzy:** no art exists. The standing hit pose sways.
- **Crouch family** (crouch, crouch block, crouch hit) for the CPU-only fighters: pieced.
- **Jumping attacks** (Mike, Lee, Gen, Birdie, Eagle, Adon): pieced on top-of-jump tucks.
- **Specials still on AP2 ROM stand-ins:** Geki Teleport Strike, Joe Rolling Sobat, Lee Double Kick, Gen Hyakurenko, Birdie Bull Head, Eagle Manchester Black / St. Andrew's Green, Adon Jaguar Kick.
- **Sagat:** crouch block, jump (ROM hop), get-up.
- **HUD and menu lettering:** level digits, call-out type, arrows and chips.

## 3. Matches

### Gaps it does not fill

| Gap | What the collection has |
| --- | --- |
| Throws / thrown | None. No fighter has a throw or thrown animation (actions 800+ absent; no custom throw groups). |
| Get-up | Action 5120 is lying → crouch (×1–3) → stance, ROM poses only. That is the same idea as ours, with fewer in-between poses than ours has. |
| Dizzy | Only Mike has one (action 5400), and it alternates his two hit poses, the same stand-in as ours. |
| AP2 special stand-ins | Same ROM streams as ours (e.g. Geki's teleport smoke, Lee's kicks). |
| Sagat crouch block, jump, get-up | His crouch, crouching block, crouching hit and every crouching attack are one pose: a half-kneel on one knee, not a crouch. Ours is a real crouch with six crouching attack poses (corrected 2026-11-20 from the side-by-side sheets; this row first said his crouch was "fists up, as ours is"). His jump is the ROM knee lift. His get-up is lying → half-kneel → stance. |
| HUD lettering | Life bars and fonts are M.U.G.E.N screen-pack art, not the ROM font. |
| Stages, title, intro, music | ROM rips of what we already draw and play from the ROM. |

### Alternatives worth a side-by-side (optional)

These are Falchion22's pieced poses where his solution differs from ours. They fit our style well because they are ROM pixels: 15–16 colours plus transparent per sprite, the 1987 scale. Conversion would be: remap each sprite's PCX palette to the fighter's pens by colour (pen 15 transparent), check none uses a 17th colour (a few sprites use one more index than our pens allow), set the foot anchor from the SFF axis, and add a manifest entry. They would only replace approved art if you prefer them.

| Fighter | Pose (SFF group) | Ours now | Theirs | Worth it? |
| --- | --- | --- | --- | --- |
| Birdie | crouching block (121) | Standing block's upper body on the crouch legs, arms crossed before his face | The same: arms crossed before his face on the crouch | No. Both versions cross his arms, so the two are near twins (corrected 2026-11-20; this row first called the crossed arms a difference). |
| Lee | crouch (11) | Sweep-end squat, head mirrored, pigtail flying | A forward-facing squat; pigtail placement needs a closer look | Maybe. Only if it fixes the pigtail. |
| Eagle | crouching stick attacks (400–430), jumping stick attacks (600–620) | Stick-jab arm turned about the shoulder | Both sticks driven forward on the crouch or tuck | Low. Same idea, different arms. |
| Mike | crouching MK/HK (410, 430), jumping hook (610, 630) | ROM ducking punch for both | A lunging low straight on the crouch | Low. It is close to his ROM ducking punch. |
| Joe, Adon, Gen, Retsu, Geki | crouch / crouch hit / jumping attacks | Approved pieced poses | Similar pieces | No. |

### New art we lack

| Item | What it is | Fit | Credit risk |
| --- | --- | --- | --- |
| **Cody** (`chars/space2/space2.sff`, 216 sprites) | A full SF1-scale moveset: stance, walk, crouch, jumps, punches, a lunging hook, an uppercut, knife states, hits, knockdown and get-up, win poses, plus a large portrait (group 9000). | Strong for **bonus roster wave 3, #2 Cody** (`2026-11-03-bonus-roster-by-lore-design.md`): already in 1987 proportions and a 16-colour palette. Conversion as above, plus a palette of his own. | His big portrait and probably his face and body come from Capcom's Final Fight / Alpha art. Falchion22's demake work should be credited as such. ADR-0006 permits fan art only "unless an artist gives permission". Confirm that permission covers Cody. |
| **Per-fighter ending scenes** (`chars/A-*/ending.sff`) | Text cards, scene poses (e.g. Eagle's 12-frame stick routine; Geki vanishing in smoke; Sagat beaten and lying), full-screen backdrops. | Possible richer art for ADR-0016's composite boss endings. | Mixed sources: Eagle's has a 1940s car and a suited gangster from another game; some backdrops are 600×400 images not drawn to SF1 scale. Poses taken would need per-sprite checks. |

## 4. Credit text required, per item

If any piece is used, the credits screen and `assets/sf1-port-art/README.md` would carry the matching line:

| Item | Credit text |
| --- | --- |
| Any fighter pose from `A-Ryu` … `A-Sagat` (except Geki and Mike) | "Additional fighter sprite edits by Falchion22 (Street Fighter M.U.G.E.N full game), used with permission. Original sprites © Capcom." |
| Any Geki pose | "Geki sprite edits by CWDEVINE and Falchion22, used with permission. Original sprites © Capcom." |
| Any Mike pose | As Falchion22's, plus the second author once known (his `.def` reads "Falchion22 &&"). |
| Cody | "Cody 1987-style sprites by Falchion22, used with permission. Cody © Capcom." |
| Ending-scene art | "Ending scene art by Falchion22, used with permission. Original art © Capcom." Plus any other game the sprite came from. |
| Stages (not recommended) | "Stage conversions by Kitsune Sniper." |
| Bonus stage (not recommended) | "Bonus stage by DCelso." |
| Distribution of the full game (not used) | "Mugen 1.0 update and hosting by zesensei." |

## 5. Before anything is imported

1. **Permission scope.** zesensei's readme says only that it is hosted with Falchion22's permission. Find out who gave you permission and whether it covers Falchion22's edits, CWDEVINE's Geki and Mike's unnamed co-author, and inclusion in a distributed build.
2. **ADR-0006.** Fan sprite sets are excluded unless an artist gives permission. Record the permission in a short ADR note, or an amendment to 0006, naming which sets it covers.
3. **ADR-0003.** No `.sff`/`.snd` files in the repository. Only converted, approved indexed PNGs would go in the pack, with their provenance in `art/`.

## Decisions (the maintainer, 2026-11-20, after the side-by-side sheets)

The sheets compared each candidate with ours at 2x on one background. They were not committed (`docs/media/mugen-compare/`).

1. **Mike's crouching block:** use Falchion22's gloves-up crouching block (`A-Mike/sff.sff` group 121), remapped to Mike's ROM pens. It replaces his ROM crouch held as the block in SF1+ and the era styles. The faithful 1987 mode draws from the ROM and does not change.
2. **Credit:** the line only, without the permission source: "Additional sprite frames for Mike, Eagle and Lee adapted from Falchion22's MUGEN characters, used with permission." It goes in `THIRD_PARTY_NOTICES.md` and the art pack README.
3. **New moves, design first:** Eagle's two-stick jumping thrust (groups 600/610) and his crouching stick attacks (400/410/430), Lee's jumping punch (600, which reads as a kick) and Lee's jumping kick (650) become new moves in SF1+ and the era styles. The faithful 1987 mode does not change. Nothing is imported or built until the design (inputs, frame data, hitboxes, damage, which era styles get them) is approved.
4. **Everything else stays ours.** Our versions are as close or closer for Birdie, Lee's crouch family, Sagat, Joe, Adon, Gen, Retsu, Geki and the get-ups.

## Recommendation (as first written)

Skip the collection for the gaps on `art-upgrades.md`: it has nothing for throws, get-ups or dizzy, and our approved crouches and jumping attacks are as good as its pieces. Keep two uses open:

- **Cody**, when wave 3 starts, once permission is confirmed.
- **A side-by-side of Birdie's crouching block and Lee's crouch**, if you want to see them. It costs nothing but review time.
