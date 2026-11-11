# M.U.G.E.N Community Integration, "What-If" Era Styles, and 1987 Bonus Roster Design

- **Date:** 2026-10-28
- **Status:** Approved (2026-10-28)
- **Context:** [`CONTEXT.md`](../../CONTEXT.md), [`docs/design/2026-10-27-definitive-overhaul-design.md`](2026-10-27-definitive-overhaul-design.md), [`docs/adr/0002-two-tier-overhaul-architecture.md`](../adr/0002-two-tier-overhaul-architecture.md), [`docs/adr/0003-offline-mugen-extraction-and-pose-synthesis.md`](../adr/0003-offline-mugen-extraction-and-pose-synthesis.md), [`docs/research/mugen-sf1-style-characters.md`](../research/mugen-sf1-style-characters.md)

---

## 1. Intent and Why

Our research into two decades of M.U.G.E.N and Ikemen GO community creations ([`docs/research/mugen-sf1-style-characters.md`](../research/mugen-sf1-style-characters.md)) showed that fan creators have already solved several of the hardest design and animation challenges facing `sf1-port`'s **overhaul**:

1. **Completing the 1987 Roster (`SF1` and `SF1+` Era Styles):** Creators such as **Falchion22**, **Electro**, and **HelloMyNameIsAAA** built playable 1987-sprite versions of all 12 fighters from the **original game**, solving the 10 CPU bosses' missing crouching, jumping, blocking, and thrown states.
2. **Giving the "Forgotten Five" Later-Era Movesets:** Because **Retsu**, **Geki**, **Joe**, **Mike**, and **Lee** never received official 2D sequels after 1987, a collaborative community effort (**David_Bazbar**, **Rabano**, **Trololo**, **R@CE AKIR@**, **Zero de Armentis**, **Duracelleur**, **0megaturb0**) engineered complete multi-era movesets, rekkas, command normals, and Supers for them.
3. **Expanding the 1987 World with Lore-Connected Challengers:** Community remakes (*Street Fighter One*, Pilgrim's *SF1 Remake*) and sprite artists (**Falchion22**, **Legorulez49**, **Jessica Smoke**, **Mugenmonkey**, **Shadow Blade**, **Vyn**) created 1987-era versions of characters deeply tied to the first tournament's lore—including **Gou Hibiki**, **Gouken**, **Evil Ryu ('87)**, **Akuma**, **Goutetsu**, **Sheng Long**, the *Street Fighter '89* (*Final Fight*) trio (**Cody**, **Guy**, **Mike Haggar**), **Charlie Nash**, **Dan Hibiki**, **M. Bison**, and young **Chun-Li** (trained by 1987 boss **Gen**).

**The intent of this design** is to integrate these community findings into `sf1-port` under two core principles:
- **Options, Never Replacements:** The 12 canonical tournament fighters and their canonical **era styles** remain the untouched foundation. Community-derived **"What-If" era styles** and the **13 Bonus 1987 Challengers** are additive options behind explicit toggles (`enable_what_if_styles` and `enable_bonus_challengers`).
- **100% 1987 Visual and Audio Consistency via Offline Extraction:** Rather than shipping raw CPS1/CPS2/CvS sprite rips or a runtime M.U.G.E.N interpreter, we build an offline extractor (`tools/mugen-extract`) that harvests frame timings, hitboxes, move trees, and pose keyframes from reference M.U.G.E.N packages and feeds them into `tools/asset-gen` to synthesize authentic `384×224`, 15-color-plus-transparent `xRGB_444` 1987 tiles and 8 kHz ADPCM audio.

---

## 2. Goals, Non-Goals, and Anti-Goals

### Goals
1. **Accelerate `SF1` & `SF1+` Core Roster Completion:** Use Falchion22's, Electro's, and HelloMyNameIsAAA's 1987 M.U.G.E.N conversions to supply axis-offset throw-victim tables for raw `SF1` mode (requiring zero new ROM tiles) and the frame-by-frame pose checklist and frame data for `SF1+` boss completion.
2. **Optional "What-If" Era Styles (`enable_what_if_styles`):** Provide selectable, clearly badged "What-If" **era styles** (`SF2`, `SFZ`, `SF3`, `SF4`, `SF5`, `SF6`) for the "Forgotten Five" (**Retsu**, **Geki**, **Joe**, **Mike**, **Lee**) and any other roster fighters in entries they canonically missed, plus a "What-If" raw `SF1` style for the 13 Bonus Challengers.
3. **Optional 13 Bonus 1987 Challengers (`enable_bonus_challengers`):** Expand the playable roster from 12 to 25 fighters when enabled by adding **Gou Hibiki**, **Gouken**, **Evil Ryu ('87)**, **Akuma (Gouki)**, **Goutetsu**, **Sheng Long**, **Cody Travers**, **Guy**, **Mike Haggar**, **Charlie Nash**, **Dan Hibiki**, **M. Bison (Dictator)**, and **Chun-Li ('87)**—selectable in Versus, Training, World Tour, Dramatic Battle, Online, and Arcade Mode, and appearing as conditional secret challengers in 12-Character Arcade Mode.
4. **Offline M.U.G.E.N Extraction Pipeline (`tools/mugen-extract`):** Automatically parse `.def`, `.sff` (v1/v2), `.air` (`Clsn1`/`Clsn2`), `.cns` (`Statedef`/`HitDef`), and `.cmd` files into deterministic `sf-overhaul` move tables and pose-conditioning sprite sheets for `tools/asset-gen`.
5. **Uncompromised 1987 Aesthetic:** Guarantee that every What-If move and every Bonus Challenger is rendered in authentic 1987 Capcom arcade pixel art (`384×224` resolution, 15 opaque `xRGB_444` colors + pen `15` transparent per `16×16` 4bpp tile) and 1987 8 kHz MSM5205 ADPCM + YM2151 FM audio.

### Non-Goals
1. **Runtime M.U.G.E.N Engine or End-User `.sff`/`.cns` Loader:** We are not interpreting `.cns`/`.zss` scripts at runtime or loading unquantized M.U.G.E.N `.sff` files directly into the game.
2. **Joke or Parody M.U.G.E.N Edits:** Meme characters (*Bootleg Ryu*, *Butthead Sagat*, *SHINGOMELTAKI*) are out of scope.

### Anti-Goals (What Would Make This a Failure Even If It Works)
1. **Overwriting Canonical Content:** Any configuration where What-If styles or Bonus Challengers replace the 12 original fighters, alter canonical **era styles**, or cannot be disabled via ruleset toggles.
2. **Visual Clash ("M.U.G.E.N Soup"):** Any fighter or animation that looks like a pasted CPS1, CPS2, CPS3, CvS, or KOF sprite rip with mismatched proportions, outlines, or palette depths.
3. **Committing Third-Party ROM/M.U.G.E.N Binaries to Git:** Checking copyrighted `.sff` or `.snd` files into version control.
4. **Delaying Core Phases 1–5:** Allowing Phase 6 expansion content (What-If styles and the 13 Bonus Challengers) to block the faithful 1987 **port** or the core 12-fighter **overhaul** and rollback netplay.

---

## 3. Constraints

- **Fidelity & Two-Tier Architecture Contract ([`ADR-0001`](../adr/0001-recover-original-logic-in-rust.md), [`ADR-0002`](../adr/0002-two-tier-overhaul-architecture.md)):** All What-If styles and Bonus Challengers live strictly in Tier 2 (`sf-overhaul`) and external overhaul asset packs (`assets/overhaul/*.pack`). `Faithful 1987` mode remains 100% untouched.
- **Clean-Room IP & Local Reference Boundary:** Reference M.U.G.E.N archives live only in a gitignored local developer directory (`tools/reference/local/mugen/`). Only extracted numeric metadata (normalized to `sf-overhaul` schemas) and newly synthesized 1987-style tile/audio packs are distributed with the overhaul.
- **Deterministic 60 Hz Integer Simulation:** Floating-point velocities (`velset`, `yaccel`) extracted from `.cns` files are converted offline into deterministic integer / 16.16 fixed-point values so rollback netplay and **Replay Takeover** remain bit-exact.

---

## 4. Architectural Approaches Considered

We evaluated three approaches for integrating the M.U.G.E.N community findings into `sf-overhaul` and `tools/asset-gen` (after ruling out runtime `.cns` interpretation and raw `.sff` bundling due to determinism, IP, and visual-consistency constraints):

### Approach 1: Offline M.U.G.E.N Extractor (`tools/mugen-extract`) + Pose-Conditioned 1987 Synthesizer (`tools/asset-gen`) — (Selected)
- **Description:** Build an offline developer CLI (`tools/mugen-extract`) that parses reference `.def`, `.sff` (v1/v2), `.air`, `.cns`, and `.cmd` packages. It converts `.air` frame timings, `Clsn1`/`Clsn2` boxes, and `.cns` `HitDef`/velocity blocks into `sf-overhaul` move tables, and exports aligned `.sff` frames as pose-conditioning keyframes for `tools/asset-gen` to synthesize clean 15-color 1987 tiles.
- **Trade-offs:** Requires writing an offline parser for Elecbyte's `.sff`/`.air`/`.cns` formats in `tools/`, but saves thousands of hours of manual frame-data transcription across 25 characters and preserves the exact move timings and silhouettes from the community's best builds while keeping the runtime 100% clean-room, deterministic, and visually uniform.

### Approach 2: Manual Clean-Room Transcription & Prompt-Only Sprite Generation
- **Description:** Use [`docs/research/mugen-sf1-style-characters.md`](../research/mugen-sf1-style-characters.md) purely as a written design spec, hand-coding all move tables, frame counts, and hitboxes in `sf-overhaul` and generating missing 1987 poses in `tools/asset-gen` from text prompts alone.
- **Trade-offs:** Avoids writing a M.U.G.E.N parser, but manually tuning hitboxes, frame data, and branching rekkas across 12 core fighters, What-If styles, and 13 bonus challengers is slow, error-prone, and loses the tested frame timings and pose silhouettes that already work well in M.U.G.E.N.

### Approach 3: Approach 1 + End-User Runtime `.sff`/`.air` Mod Importer
- **Description:** Build the offline extractor from Approach 1 for official overhaul packs, and also ship a runtime/user-facing importer that auto-quantizes and slices downloaded M.U.G.E.N `.sff`/`.air` folders into `sf-video` tile packs at launch.
- **Trade-offs:** Allows local sprite-swap experiments, but arbitrary community `.cns` state scripts cannot map onto `sf-overhaul` without a runtime interpreter, and blind 15-color quantization of 64-color CPS2/CvS sprites produces severe palette banding and height mismatches that violate our 1987 visual consistency goal.

---

## 5. Detailed Design

### 5.1 Ruleset Toggles, Presets, and UI Separation

Two match/ruleset toggles in `sf-overhaul` control all expanded community-inspired content:

| Toggle Name | Default in `Faithful 1987`, `Classic Competitive`, & `Pure Footsies` | Default in `Definitive Overhaul` | Behavior When `true` |
| :--- | :--- | :--- | :--- |
| `enable_what_if_styles` | `false` | `true` | Unlocks non-canonical **"What-If" era styles** (marked with a pixel-art `[WHAT-IF]` badge on the style selector) for fighters in eras they did not canonically appear in. |
| `enable_bonus_challengers` | `false` | `true` | Unlocks the **13 Bonus 1987 Challengers** (in a toggleable Bonus Row on Character Select marked `[BONUS '87]`) and enables conditional secret challenger intrusions in Arcade Mode (with a sub-toggle `enable_secret_arcade_intrusions` to disable intrusions independently). |

On the Character Select screen, pressing a dedicated filter button (`Start` / `Select` or UI toggle) lets players switch the visible grid instantly between **`1987 Tournament 12`** (**tournament fighters** only) and **`Expanded 25 Roster`** (including **bonus challengers**), and filter the style list between **`Canonical Styles Only`** and **`All Styles (Incl. What-If)`**.

In online rollback netplay, both `enable_what_if_styles` and `enable_bonus_challengers` are part of the host-synchronized `MatchConfig` enforced at handshake, and both are exposed as visible rule badges and search filters in the online Room Browser.

---

### 5.2 Accelerating `SF1` & `SF1+` for the 12 Core Fighters (Phases 2 & 3)

1. **Zero-Tile Throw-Victim Support in Raw `SF1` Mode (Phase 2):**
   - In the 1987 ROMs, the 10 CPU bosses lack dedicated thrown-rotation sprite states when grabbed by Ryu or Ken's throws.
   - Using **Falchion22**'s `.air` `Action 820` tables, `sf-engine` / `sf-overhaul` maps each boss's existing `gfx3` horizontal knockdown and recoil tiles with per-frame `(dx, dy)` target-bind offsets and `flipx`/`flipy` flags so all 12 fighters can be thrown cleanly in raw `SF1` mode without adding a single new tile.
2. **Missing-Pose Checklist & Movesets for `SF1+` (Phase 3):**
   - `tools/mugen-extract` extracts the exact missing-frame poses and `.air`/`.cns` timings from **Falchion22**'s 12-fighter 1987 set, **Electro**'s *Ultimate Ryu SF1* (2024) and *Ultimate Adon* (2025), and **HelloMyNameIsAAA**'s 1987 set to drive `tools/asset-gen`:
     - **Sagat:** Crouching idle, crouch block, standing block, crouching punch/kick normals, jump arc, and air punch/kick normals; *Tiger Cannon* / *Tiger Genocide* supers for later styles.
     - **Mike:** Crouching attacks, jumping attacks, standing/crouching guard, and standard 6-button kick animations on `LK, MK, HK` (including crouching kicks and jumping kicks) in `SF1+` (while reserving his 6-punch boxer layout for his `SF2`–`SF6` **what-if styles**).
     - **Geki:** Dedicated standing and crouching guard frames (decoupled from his 1987 teleport), ground/air *Shuriken* toss, directional *Utsusemi* teleport, diving claw slash, and ground throw.
     - **Retsu, Joe, Lee, Gen, Birdie, Eagle, Adon:** Missing crouching/jumping attack transitions, standing/crouching guard frames, ground throw startups, and 1987-style arcade ending illustrations (using Electro's Adon ending and Ninja Hayate's Ryu/Ken endings as structural references).

---

### 5.3 "What-If" Era Styles for the 12 Tournament Fighters (Phase 6A)

When `enable_what_if_styles` is active, every **tournament fighter** in the original 12-character roster who missed a canonical *Street Fighter* entry (`SF2`, `SFZ`, `SF3`, `SF4`, `SF5`, `SF6`) receives a `[WHAT-IF]` **what-if style** for that entry, paired with that entry's **era gauge** and system mechanics.

In the two-step `Era Family -> Revision (& Loadout)` selector, each **what-if style** Era Family exposes that era's flagship revision(s)—`SF2 HF` (meterless *SF2*) and `SF2 ST` (Super *SF2*), `SFZ3`, `SF3 3S`, `USF4`, `SF5 CE`, and `SF6`—complete with full Loadout selection (`A-Ism`/`X-Ism`/`V-Ism`, Super Arts I/II/III, Ultras I/II/Double, V-Skills/V-Triggers I/II), rather than fabricating synthetic intermediate patch revisions (`WW`, `NG`, `2I`, etc.).

| Fighter | Canonical Era Styles | Added `[WHAT-IF]` Era Styles | M.U.G.E.N Moveset & Pose Blueprint |
| :--- | :--- | :--- | :--- |
| **Retsu** | `SF1`, `SF1+`, `SF6` | `SF2`, `SFZ`, `SF3`, `SF4`, `SF5` | **David_Bazbar**, **Rabano**, & **0megaturb0**: *Tobi Geri* (leaping flying kick), *Shorinji Palm Thrust* (short-range ki wave), *Hundred Monk Slaps*, *Rising Dragon Knee*, *Izuna Shoulder Throw* command grab, *Ranbu Kempo Barrage* & *Tenchi Shorinji Drop* Supers. |
| **Geki** | `SF1`, `SF1+` | `SF2`, `SFZ`, `SF3`, `SF4`, `SF5`, `SF6` | **Zero de Armentis**, **Rabano**, & **Chuchoryu**: Multi-angle ground/air *Kunai / Shuriken Toss*, 3-destination *Utsusemi* smoke teleport, *Geki-Zan* lunging claw slice, *Izuna Claw Dive*, *Fuma Shuriken Storm* & *Ansatsu Geki-Retsuzan* assassination Supers. |
| **Joe** | `SF1`, `SF1+` | `SF2`, `SFZ`, `SF3`, `SF4`, `SF5`, `SF6` | **R@CE AKIR@**, **Trololo**, & **RagingRowen**: *Step High Kick* (`F+MK`), *Rolling Sobat* (`F+HK`), 3-stage *Fist Combination* rekka, *Double Kick*, *Sobat Kick*, *Ground Kick* low sweep, *Super Sobat*, *Thrash Kick*, and wall-breaking *Meteor Kick* Lv3 Super. |
| **Mike** | `SF1`, `SF1+` | `SF2`, `SFZ`, `SF3`, `SF4`, `SF5`, `SF6` | **Trololo**, **Duracelleur**, & **Rabano**: 6-punch motion-input heavyweight slugger (Head Punches on `LP/MP/HP`, Body Hooks/Uppercuts on `LK/MK/HK`), *Murder Straight* haymaker, *Weaving Upper*, *Rolling Blow*, projectile-invulnerable *Ducking Sway*, *Crazy Rush* & *Lethal Knockout* Supers. |
| **Lee** | `SF1`, `SF1+` | `SF2`, `SFZ`, `SF3`, `SF4`, `SF5`, `SF6` | **Rabano** (Shadaloo C.R.I. Bajiquan/Zui Quan kit): *Open Canvas* (`QCF+P`) branching into Punch rekka (*Oil -> Old Ink / Enamel*) and Kick rekka (*Aquarelle -> Graphite / Sangaree*), *Upward Lance* (`DP+K` -> *Cut / Slice / Stab Kick*), *Hurricane Kick*, *Firewater* (`[4]6+P`), *Sunrider*, *Fiery Breath*, *Hundred Spears*, & *One Man Army* Lv3. |
| **Sagat, Gen, Adon, Birdie, Eagle** | Their canonical entries (see [`2026-10-27-definitive-overhaul-design.md`](2026-10-27-definitive-overhaul-design.md)) | Missing entries among `SF2`, `SFZ`, `SF3`, `SF4`, `SF5`, `SF6` | Adapts each veteran's signature toolkit to the missing era's system mechanics (e.g., `SF3` Parry + Super Arts I/II/III; `SF6` Drive Gauge + Overdrive), using **Mazemerald / SFONE**'s 1987 Punk Birdie (`BIRDIE_SFONE`), **mwryly & varo_hades**'s 1987 Gen, and **varo_hades**'s 1987 unscarred Sagat as pose references for 1987-outfit animations. |

---

### 5.4 The 13 Optional Bonus 1987 Challengers (Phase 6B)

> **Replaced (2026-11-03).** This section's roster is replaced by [`2026-11-03-bonus-roster-by-lore-design.md`](2026-11-03-bonus-roster-by-lore-design.md): 28 bonus challengers chosen by 1987 lore, in three roster waves, with SF1 and SF1+ as their native styles. The toggles, the Bonus Rival Exhibition Rule and the idea of lore episodes below still stand. See [ADR-0008](../adr/0008-bonus-roster-chosen-by-lore.md).

When `enable_bonus_challengers` is enabled, 13 lore-connected **bonus challengers** join the roster. Every **bonus challenger** is rendered strictly in **1987 arcade pixel-art style** (`384×224`, 15-color `xRGB_444` sub-palettes, `16×16` tiles), includes a dedicated 1987-styled stage variant/tilemap remix and an authentic 1987 YM2151 FM character theme arrangement (plus Jukebox Red Book CD support), and offers:
- A **`[WHAT-IF] SF1`** style modeled on Ryu and Ken's complete 1987 player state-machine rules (full crouch, jump, and block states subject to 1987 frame/damage/hitstun rules and active **core toggles**, with no artificial boss state lockouts)
- A **`SF1+`** style (completed, balanced 1987 tournament kit with full 6-button normals and no meter)
- Their **canonical later era styles** (plus optional `[WHAT-IF]` styles for eras they missed, exposing flagship revisions `SF2 HF`/`ST`, `SFZ3`, `SF3 3S`, `USF4`, `SF5 CE`, `SF6`).

| # | Bonus Challenger | 1987 Lore Connection | Primary M.U.G.E.N Reference(s) | 1987 Sprite Design & Signature Moveset | Canonical + What-If Era Styles |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **13** | **Gou Hibiki** | Dan's father; took Sagat's right eye in a fatal duel shortly before the 1987 tournament. | **Mugenmonkey** (1987 sprite sheet), **Pilgrim** (*SF1 Remake* Ikemen GO) | 1987 karate gi with Tengu mask / bare-faced toggle; aggressive proto-Saikyo counters, one-handed *Gadoken*, *Koryuken*, and *Haoh Gadoken*. | `SF1`*, `SF1+`*, `SFZ`*, `SF4`*, `SF5`*, `SF6`* |
| **14** | **Gouken** | Master of Ryu and Ken, friend of Retsu, and brother of Akuma during the 1987 era. | **Mugenmonkey** (1987 Gouken demake) | 1987 muscular master proportions, prayer beads, one-shoulder sleeveless gi; angled/straight *Gohadoken*, *Senkugoshoha*, *Tatsumaki Gorasen*, *Forbidden Shoryuken*. | `SF1`*, `SF1+`*, `SFZ`*, `SF4` (`SF4`–`USF4`), `SF5`*, `SF6`* |
| **15** | **Evil Ryu ('87)** | Ryu's *Satsui no Hado* awakening at the climax of the 1987 final when scarring Sagat's chest. | **M** (`sf1_evilryu.rar`), **P.O.T.S.** | Dark charcoal gi, torn hem, glowing red eyes on 1987 Ryu base; *Hadoken*, *Shakunetsu*, *Ashura Senkuu* teleport, multi-hit *Metsu Shoryuken* (chest-scar finisher), *Shun Goku Satsu*. | `SF1`*, `SF1+`*, `SFZ` (`SFZ2`–`Z3`), `SF4` (`AE`–`USF4`), `SF5`* (*Kage* kit), `SF6`* |
| **16** | **Akuma (Gouki)** | Already wandering the world in 1987 after killing Goutetsu and dueling Gouken. | **Master Gouken**, **Basara-kun**, **P.O.T.S.** | 1987 shoto proportions with topknot, prayer beads, and sleeveless dark gi; *Gohadoken*, *Zanku Hadoken* (air fireball), *Goshoryuken*, *Hyakkishu* demon flip, *Shun Goku Satsu*. | `SF1`*, `SF1+`*, `SF2 ST`, `SFZ1`–`Z3`, `SF3 NG`–`3S`, `SF4`–`USF4`, `SF5`–`CE`, `SF6` |
| **17** | **Goutetsu** | Grandmaster of Ansatsuken who trained Gouken and Akuma and knew Retsu. | **Shadow Blade (Gerardo Chan)**, **Angelm + Colosse** | Tall, lean 1987 grandmaster in dark robes with long beard and topknot; ancient Ansatsuken strikes, teleport step, *Kiai* shockwave, and lethal counter-throws. | `SF1`*, `SF1+`*, `SFZ`*, `SF4`*, `SF5`*, `SF6`* |
| **18** | **Sheng Long** | The legendary 1992 *EGM* April Fools' master ("You must defeat Sheng Long to stand a chance"). | **Vyn & Angelm**, **Cypher44** | Classic braided white-haired Chinese martial-arts master robe in 1987 palette; Dragon-Tiger hybrid specials (*Flying Dragon Fist*, flaming rising dragon kick, high-speed teleports). | `SF1`*, `SF1+`*, `SF2`*, `SFZ`*, `SF3`*, `SF4`*, `SF6`* |
| **19** | **Cody Travers** | 1987 Metro City street brawler (two years before *Final Fight* / *Street Fighter '89*). | **Falchion22** (1987 Cody demake in *Street Fighter One*), **Pilgrim** | 1987 Joe/Ryu brawler build in blue jeans, white T-shirt, sneakers; *Bad Stone* rock toss, brawler punch combo, *Criminal Upper*, *Ruffian Kick*, *Final Destruction*. | `SF1`*, `SF1+`*, `SFZ3`, `SSF4`–`USF4`, `SF5 AE`–`CE` (+ `SFZ3` prison / `SF5` Mayor **era costumes**) |
| **20** | **Guy** | Bushinryu ninja training in Japan/Metro City in 1987; natural ninja rival to **Geki**. | **Memo**, **Fido**, **P.O.T.S.** | 1987 agile ninja build ( modeled alongside 1987 Geki/Lee proportions) in red Bushinryu gi and sneakers; *Bushin Senpukyaku*, *Hayagake* run-stop/follow-ups, *Hozanto*, *Bushin Izuna Otoshi*. | `SF1`*, `SF1+`*, `SFZ1`–`Z3`, `SSF4`–`USF4`, `SF6`* (*Kimberly/Bushin* hybrid) |
| **21** | **Mike Haggar** | Former Slam Masters pro-wrestler prior to his 1989 Metro City mayoral term; natural rival to **Birdie**. | **Electros** (*Charged Haggar*), **P.O.T.S.** | 1987 Birdie/Mike heavyweight build with olive wrestling trousers and single shoulder strap; *Double Lariat*, *Violent Axe*, *Flying Piledriver*, *Spinning Piledriver*, *Giant Haggar Press*. | `SF1`*, `SF1+`*, `SF2`*, `SFZ`*, `SF3`*, `SF4`*, `SF5`*, `SF6`* |
| **22** | **Charlie Nash** | U.S. military investigator tracking early Shadaloo operations around 1987. | **Legorulez49** (`CharlieSF1.rar` 1987 demake) | 1987 Joe/Ryu build with vertical blonde forelock, glasses, yellow vest, green military trousers; one-handed *Sonic Boom*, *Somersault Justice*, *Moonsault Slash*, *Crossfire Blitz*. | `SF1`*, `SF1+`*, `SFZ1`–`Z3`, `SF5`–`CE` (with `SF5` stitched **era costume**) |
| **23** | **Dan Hibiki** | Former student of Gouken seeking to avenge his father Gou Hibiki against Sagat. | **Jessica Smoke** (`sfdan.rar` 1987 demake) | 1987 shoto build with ponytail, pink gi over black undershirt, and Saikyo stance; short-range *Gadoken*, *Koryuken*, *Dankukyaku*, rolling taunts, *Hissho Buraiken*, *Legendary Taunt*. | `SF1`*, `SF1+`*, `SFZ1`–`Z3`, `SSF4`–`USF4`, `SF5 CE` |
| **24** | **M. Bison (Dictator)** | Shadaloo leader who recruits Sagat and Birdie immediately after the 1987 tournament. | **Fido**, **P.O.T.S.**, **Juke Kisaragi** | 1987 Sagat/Eagle commander build in red uniform, peaked cap, and shoulder pads (with cape intro); *Psycho Crusher*, *Double Knee Press*, *Head Press* / *Somersault Skull Diver*, *Psycho Shot*. | `SF1`*, `SF1+`*, `SF2 CE`–`ST`, `SFZ1`–`Z3`, `SF4`–`USF4`, `SF5`–`CE`, `SF6` |
| **25** | **Chun-Li ('87)** | Young martial artist trained in China by 1987 boss **Gen** while searching for her father Dorai. | **Fido**, **P.O.T.S.**, **BoyBoyz** | 1987 Lee/Gen agile martial-arts build in her early *Alpha* kung-fu training suit (with `SF2` qipao and `SF6` master **era costumes**); *Hyakuretsukyaku* lightning legs, *Kikoken*, *Spinning Bird Kick*, *Senretsukyaku*. | `SF1`*, `SF1+`*, `SF2 WW`–`ST`, `SFZ1`–`Z3`, `SF3 3S`, `SF4`–`USF4`, `SF5`–`CE`, `SF6` |

*(\* indicates a `[WHAT-IF]` era style for that character.)*

#### Single-Player Campaigns, Secret Intrusions, & Lore Matchup Presets
1. **Playing Arcade Mode & 1987 World Tour as a Bonus Challenger:**
   - All 13 **bonus challengers** can be selected as Player 1 for the full 5-country Arcade Mode ladder (Japan, USA, China, England -> Thailand Adon & Sagat) and the 1987 World Tour campaign, complete with bespoke 1987-style pixel-art rival dialogues and character endings (such as a "what-if" ending where Gou Hibiki survives his duel with Sagat, or M. Bison recruits Sagat into Shadaloo).
2. **Secret Challenger Intrusions in Arcade Mode (Bonus Rival Exhibition Rule):**
   - When both `enable_bonus_challengers` and `enable_secret_arcade_intrusions` are `true` during an Arcade Mode run, the standard 10 country opponents (Retsu, Geki, Joe, Mike, Lee, Gen, Birdie, Eagle, Adon, Sagat) are **never replaced** on the country ladder.
   - Meeting performance milestones (winning rounds without losing a life bar, landing Perfects, or finishing with Supers) triggers an optional **"Here Comes A New Challenger!"** rival intrusion stage before or after Thailand (for example: **Gou Hibiki** or **Dan** challenging Sagat; **Evil Ryu**, **Akuma**, **Gouken**, **Goutetsu**, or **Sheng Long** challenging Ryu/Ken/Retsu; **Guy** challenging Geki; **Haggar** challenging Birdie; **Chun-Li** challenging Gen; **M. Bison** appearing after Sagat).
   - **Bonus Rival Exhibition Rule:** Losing to an intruding **bonus challenger** does not cost a credit or cause a Game Over; the player is offered an optional instant Rematch or advances cleanly to the next scheduled stage.
3. **Lore Matchup Presets in 2v1 Dramatic Battle & 1987 World Tour:**
   - Enabling `enable_bonus_challengers` unlocks dedicated 1987 lore episodes in **2v1 Dramatic Battle** and **1987 World Tour**, including *Ryu & Ken vs. Akuma / Gouken*, *Gouken & Retsu vs. Goutetsu*, *Cody & Guy vs. Mike Haggar*, *Gen & Chun-Li ('87) vs. M. Bison*, and *Sagat vs. Gou Hibiki & Dan*.

---

### 5.5 Offline Extraction & 1987 Synthesis Pipeline (`tools/mugen-extract` -> `tools/asset-gen`)

```
Local Reference M.U.G.E.N Pack (gitignored)
  ├── char.def / char.act
  ├── char.sff (v1 PCX / v2 PNG/LZ5)
  ├── char.air (Actions, Durations, Clsn1/Clsn2)
  └── char.cns / char.cmd (Statedef, HitDef, Motions)
          │
          ▼
[tools/mugen-extract] (Offline Rust CLI)
  ├── 1. Collision Converter:
  │      Clsn2[0..n] -> depth 2 (outer loose union) + depth 1 (inner tight torso)
  │      Clsn1[0..m] -> active attack boxes
  ├── 2. Frame & Physics Normalizer:
  │      Startup / Active / Recovery ticks from .air + HitDef
  │      Float velocities -> 16.16 fixed-point integers
  │      Damage / Hitstun / Guardstun scaled to sf-overhaul 1,000 HP scale
  └── 3. Pose Sheet Exporter:
         Renders .sff frames aligned to (axis_x, axis_y) ground-center
          │
          ▼
[tools/asset-gen] (Offline 1987 Asset Synthesizer)
  ├── 1987 Sprite Conversions (Category 1 & 2):
  │      Direct xRGB_444 palette mapping (pen 15 = transparent) + 16x16 4bpp tile slicing
  ├── Later-Style Reference Sprites (Category 3 & Bonus Challengers):
  │      Pose-conditioned AI synthesis anchored on decoded 1987 gfx3 sprite sheets
  │      -> Quantized to 15 opaque xRGB_444 colors + pen 15 transparent -> 16x16 4bpp tiles
  └── Audio Encoder:
         Synthesized/recorded callouts -> 8000 Hz 4-bit MSM5205 ADPCM + YM2151 FM patches
          │
          ▼
assets/overhaul/*.pack (Mounted at runtime by sf-overhaul & sf-video)
```

1. **Sprite & Palette Rules (`SFF` + `ACT` -> `sf-video`):**
   - For 1987-sprite M.U.G.E.N packs (Falchion22, Electro, HelloMyNameIsAAA, Legorulez49, Jessica Smoke, Mugenmonkey), `tools/mugen-extract` extracts the 8-bit indexed PCX/PNG frames, remaps M.U.G.E.N color index `0` (transparent) to `sf-video` pen `15` (`0x0f`), quantizes each `(R, G, B)` triplet to 12-bit `xRGB_444` (`(r >> 4) << 8 | (g >> 4) << 4 | (b >> 4)`), and feeds any frankensprited seams into `tools/asset-gen` for cleanup before slicing into `16×16` 4bpp tiles.
   - For CPS1/CPS2/CvS/KOF reference packs (David_Bazbar, Rabano, Trololo, Shadow Blade, Vyn, etc.), `tools/mugen-extract` exports each animation frame's silhouette and keypoint skeleton relative to `(axis_x, axis_y)`. `tools/asset-gen` uses those keyframes as pose conditioning alongside the character's 1987 reference sheet to generate authentic 1987 pixel-art frames, quantizes them strictly to 15-color + transparent sub-palettes, and packs them into `16×16` tiles.
2. **Collision Box Mapping (`AIR` -> `sf-engine` / `sf-overhaul`):**
   - M.U.G.E.N `.air` coordinates `(left, top, right, bottom)` relative to ground-center `(0,0)` are converted to `sf-engine`'s center-origin `(dx, dy, half_w, half_h)` format:
     $$\text{dx} = \frac{x_{\text{left}} + x_{\text{right}}}{2}, \quad \text{dy} = \frac{y_{\text{top}} + y_{\text{bottom}}}{2}, \quad \text{half\_w} = \frac{x_{\text{right}} - x_{\text{left}}}{2}, \quad \text{half\_h} = \frac{y_{\text{bottom}} - y_{\text{top}}}{2}$$
   - For `SF1` and `SF1+` styles, `depth 2` (outer loose box) is computed from the bounding union of `Clsn2` boxes and `depth 1` (inner tight box) from the primary torso box. For `SF2`–`SF6` styles, `sf-overhaul` retains the full `Clsn2` hurtbox list (head, torso, legs, extended limb) and `Clsn1` hitbox list.
3. **4-Button to 6-Button Normal Bridging (`tools/mugen-extract`):**
   - Because several 1987 M.U.G.E.N conversions (including **Falchion22**'s 12-character 1987 set and **Cody**) use a 4-button (`X, Y, A, B`) layout with only Light and Heavy punches/kicks, `tools/mugen-extract` maps the 4-button source normals to `LP`/`HP` and `LK`/`HK` while populating the missing Medium (`MP`/`MK`) frame timings, hitboxes, and damage from the 1987 ROM tables (for the 12 **tournament fighters**) or 6-button POTS/CPS2 reference packs (with mathematical interpolation fallback) so every fighter in `SF1+` and later styles has a full 6-button (`LP, MP, HP, LK, MK, HK`) normal moveset.
4. **Audio Synthesis (`sf-audio`):**
   - No `.snd` WAV rips from third-party games are shipped. All new voice callouts and SFX for the Forgotten Five and the 13 Bonus Challengers are newly synthesized/recorded, downsampled to `8000 Hz`, and encoded as 4-bit **MSM5205 ADPCM** streams paired with **YM2151 FM** sound effects.

---

### 5.6 Updated Phased Delivery Roadmap

Phases 1–5 from [`docs/design/2026-10-27-definitive-overhaul-design.md`](2026-10-27-definitive-overhaul-design.md) remain in their exact order, with `tools/mugen-extract` accelerating Phases 2–4 and **Phase 6** delivering the What-If styles and 13 Bonus Challengers without delaying the core 12-fighter release:

- **Phase 1 — Complete Faithful 1987 Port (`sf-engine`, `sf-video`, `sf-audio`, `sf-dev`):** Complete the 1987 arcade-flow batch verified against MAME 0.289.
- **Phase 2 — Tier 1 Core Toggles, *SF1++* 12-Fighter Roster, & Training Overlay:** Implement `CoreToggles`, unlock all 12 original fighters in raw `SF1` mode (using Falchion22's `Action 820` throw-victim offset tables so all 10 bosses can be thrown without new ROM tiles), and ship the Training Mode overlay.
- **Phase 3 — Extended Compositor, `tools/mugen-extract` + `tools/asset-gen`, & `SF1+` 12-Fighter Overhaul:** Build `tools/mugen-extract` and `tools/asset-gen`; extract Falchion22/Electro's `SF1+` boss pose checklists and frame data; generate missing 1987 boss sprites and 8 kHz ADPCM audio; ship the balanced `SF1+` 12-fighter Arcade Mode and Jukebox.
- **Phase 4 — Canonical Historical Era Styles (`SFII`–`SF6`) for the 12 Core Fighters:** Deliver the canonical **era styles**, sub-revisions, **era gauges**, **era costumes**, and Style-Aware Utility AI for the 12 tournament fighters.
- **Phase 5 — 1987 World Tour, 2v1 Dramatic Battle, & Rollback Netplay:** Deliver World Tour, 3-fighter Dramatic Battle, and deterministic rollback netplay.
- **Phase 6 — "What-If" Era Styles & 13 Bonus 1987 Challengers Expansion:**
  - **Phase 6A (`enable_what_if_styles`):** Deliver the `[WHAT-IF]` **era styles** (`SF2`–`SF6`) for **Retsu**, **Geki**, **Joe**, **Mike**, **Lee**, and the remaining missing era slots for **Sagat**, **Gen**, **Adon**, **Birdie**, and **Eagle**.
  - **Phase 6B (`enable_bonus_challengers`):** Deliver the 13 Bonus 1987 Challengers (**Gou Hibiki**, **Gouken**, **Evil Ryu ('87)**, **Akuma**, **Goutetsu**, **Sheng Long**, **Cody**, **Guy**, **Mike Haggar**, **Charlie Nash**, **Dan**, **M. Bison**, **Chun-Li ('87)**) with their 1987 sprites, `SF1`/`SF1+`/later **era styles**, Arcade Mode secret challenger intrusions, and World Tour encounters.

---

## 6. What Is Left to the Builder

- Exact CLI flags, crate structure, and intermediate JSON/TOML schemas between `tools/mugen-extract` and `tools/asset-gen`.
- Specific numerical thresholds for triggering each secret Bonus Challenger intrusion in Arcade Mode (e.g., exact number of Perfects, Super finishes, or minimum round scores) and UI presentation of the `[WHAT-IF]` and `[BONUS '87]` badges.
- Per-move damage, hitstun, blockstun, and frame-advantage tuning when normalizing M.U.G.E.N `.cns` `HitDef` tables against `sf-overhaul`'s 1,000 HP internal scale.
- Specific AI image-generation model weights, ControlNet/pose-conditioning parameters, and palette-quantization heuristics inside `tools/asset-gen`.
