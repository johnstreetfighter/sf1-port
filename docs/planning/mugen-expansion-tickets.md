# Approved M.U.G.E.N integration, What-If era styles, and 1987 bonus roster ticket batch (issue #113)

The user approved this 8-ticket breakdown (`M1`–`M8`) on 2026-10-28 for the M.U.G.E.N Community Integration, What-If Era Styles, and 1987 Bonus Roster Expansion specification.

Each ticket is a vertical tracer-bullet slice published to GitHub Issues with the `ready-for-agent` label, a `## Parent` link to `#113`, and native GitHub `blocked_by` dependency edges linking into both this batch (`M1`–`M8`) and the core overhaul batch (`O7`–`O21`, `#98`–`#112`).

---

## M1: Offline M.U.G.E.N Extractor, 4-to-6-Button Normal Bridge, and Zero-Tile SF1 Boss Throw-Victim Offsets

### Parent

historical issue #113

### What to build

Build the offline M.U.G.E.N character extraction pipeline (ADR-0003) that parses local reference M.U.G.E.N packages into deterministic overhaul move tables and pose-conditioning sprite sheets, bridges 4-button reference normals into full 6-button (`LP, MP, HP, LK, MK, HK`) normal suites, and enables all ten CPU **tournament fighters** to be thrown in raw `SF1` (*SF1++*) mode using their existing 1987 knockdown and recoil tiles without requiring new ROM tiles.

Reference M.U.G.E.N packages (`.def`, `.sff` v1 PCX and v2 PNG/LZ5, `.act` 768-byte palettes, `.air` animations and `Clsn1`/`Clsn2` boxes, `.cns` constants and `HitDef`/`Statedef` blocks, and `.cmd` input definitions) are ingested offline from a gitignored local directory. The extractor converts `.air` `Clsn2` hurtboxes and `Clsn1` hitboxes into center-origin `(dx, dy, half_w, half_h)` dual-depth (`depth 2` outer loose union and `depth 1` inner tight torso) and multi-box collision tables, quantizes floating-point velocities and gravity into deterministic 16.16 fixed-point integers, bridges 4-button (`X, Y, A, B`) M.U.G.E.N conversions (such as Falchion22's 1987 set and Cody) into 6-button normal tables using 1987 ROM or 6-button reference data (with interpolation fallback), and exports ground-center-aligned `.sff` pose keyframes for the 1987 asset generator. In raw `SF1` mode, applying extracted `Action 820` target-bind `(dx, dy)` offsets and horizontal/vertical tile flips allows all twelve **tournament fighters** to be thrown cleanly using only unmodified 1987 ROM tiles.

### Acceptance criteria

- [ ] Parse `.def`, `.sff` (v1 and v2), `.act`, `.air`, `.cns`, and `.cmd` fixtures offline without embedding any runtime `.cns`/`.zss` interpreter or committing third-party `.sff`/`.snd` binaries to version control.
- [ ] Convert `.air` `Clsn1` and `Clsn2` rectangles relative to `(0, 0)` ground-center into deterministic center-origin `(dx, dy, half_w, half_h)` dual-depth (`depth 2` and `depth 1`) and multi-box collision tables, and quantize floating-point velocities/accelerations into 16.16 fixed-point integers.
- [ ] Bridge 4-button (`X, Y, A, B`) M.U.G.E.N character sources into complete 6-button (`LP, MP, HP, LK, MK, HK`) standing, crouching, and jumping normal tables by populating Medium (`MP`/`MK`) normals from 1987 ROM tables or 6-button reference packs (with interpolation fallback).
- [ ] Export aligned pose-conditioning keyframes and missing-state checklists (crouch, jump, block, throw) for the 1987 asset generator, and validate that generated sprite packs obey the 15-color-plus-transparent (`pen 15`) `xRGB_444` and `16×16` 4bpp tile constraints.
- [ ] Enable all ten CPU **tournament fighters** to be thrown in raw `SF1` mode using per-frame `(dx, dy)` target-bind offsets and `flipx`/`flipy` flags mapped onto existing 1987 knockdown/recoil tiles, verified across the frame-stepped scenario session seam with zero new ROM tiles mounted.

### Blocked by

- O7 (#98)
- O8 (#99)

---

## M2: Expansion Ruleset Toggles, Character/Style Filter UI, and Flagship What-If Era Styles for SF2 & SFZ3

### Parent

historical issue #113

### What to build

Add the match and **ruleset preset** toggles for optional community-inspired content (`enable_what_if_styles`, `enable_bonus_challengers`, and `enable_secret_arcade_intrusions`), add Character Select filtering and `[WHAT-IF]` / `[BONUS '87]` pixel-art badges, and deliver the `SF2` (`SF2 HF` & `SF2 ST`) and `SFZ3` (`A-Ism`, `X-Ism`, `V-Ism`) **what-if styles** for the Forgotten Five (**Retsu**, **Geki**, **Joe**, **Mike**, and **Lee**) plus `SF2` **what-if styles** for **Gen**, **Birdie**, **Eagle**, and **Adon**.

In `Faithful 1987`, `Classic Competitive`, and `Pure Footsies (1987+)`, the expansion toggles default to `false` so canonical rosters and styles are never altered; in `Definitive Overhaul`, they default to `true`, and a one-button filter on Character Select switches between `1987 Tournament 12` and `Expanded 25 Roster` and between `Canonical Styles Only` and `All Styles (Incl. What-If)`. Each `[WHAT-IF]` Era Family exposes its flagship revision(s) (`SF2 HF` for meterless *SF2* and `SF2 ST` for Super *SF2*, plus `SFZ3` with full ISM loadouts). In `SF1+`, **Mike** retains standard kick animations on `LK, MK, HK` (including crouching kicks and jumping attacks), whereas in his `SF2` and `SFZ3` **what-if styles** he transitions to the 6-punch heavyweight boxer layout (Head Punches on `LP/MP/HP`, Body Hooks and Uppercuts on `LK/MK/HK`).

### Acceptance criteria

- [ ] Add `enable_what_if_styles`, `enable_bonus_challengers`, and `enable_secret_arcade_intrusions` to `MatchConfig` (defaulting to `false` in `Faithful 1987`, `Classic Competitive`, and `Pure Footsies`, and `true` in `Definitive Overhaul`), with Character Select one-button filters and `[WHAT-IF]` / `[BONUS '87]` badges.
- [ ] Enforce the Flagship Revision Rule in the two-step `Era Family -> Revision (& Loadout)` selector so `[WHAT-IF]` `SF2` exposes `SF2 HF` and `SF2 ST` and `[WHAT-IF]` `SFZ` exposes `SFZ3` (`A-Ism`, `X-Ism`, `V-Ism`).
- [ ] Implement `SF2 HF`, `SF2 ST`, and `SFZ3` **what-if styles** for **Retsu** (*Tobi Geri*, *Shorinji Palm Thrust*, *Hundred Monk Slaps*, *Rising Dragon Knee*, *Izuna Shoulder Throw*, *Ranbu Kempo Barrage*, *Tenchi Shorinji Drop*), **Geki** (multi-angle *Shuriken*, 3-destination *Utsusemi* teleport, *Geki-Zan* claw slice, *Izuna Claw Dive*, *Fuma Shuriken Storm*, *Ansatsu Geki-Retsuzan*), and **Joe** (*Step High Kick*, *Rolling Sobat*, 3-stage *Fist Combination* rekka, *Double Kick*, *Sobat Kick*, *Ground Kick*, *Super Sobat*, *Thrash Kick*).
- [ ] Implement `SF2 HF`, `SF2 ST`, and `SFZ3` **what-if styles** for **Mike** (transitioning from `SF1+` kick normals on `LK/MK/HK` to the 6-punch boxer layout with *Murder Straight*, *Weaving Upper*, *Rolling Blow*, *Ducking Sway*, and *Crazy Rush*) and **Lee** (Shadaloo C.R.I. *Open Canvas* punch/kick branching rekkas, *Upward Lance -> Cut/Slice/Stab Kick*, *Hurricane Kick*, *Firewater*, *Sunrider*, *Fiery Breath*, *Hundred Spears*), plus `SF2 HF`/`ST` **what-if styles** for **Gen**, **Birdie**, **Eagle**, and **Adon**.
- [ ] Verify all new **what-if styles**, gauges, and toggle filters across the frame-stepped scenario session seam and confirm that all synthesized sprites and audio packs pass the 1987 `16×16` 4bpp tile and 8 kHz ADPCM/FM asset validator.

### Blocked by

- M1
- O13 (#104)

---

## M3: Flagship What-If Era Styles (SF3 3S, USF4, SF5 CE, SF6) for All 12 Tournament Fighters

### Parent

historical issue #113

### What to build

Complete the **what-if style** matrix across `SF3 3S`, `USF4`, `SF5 CE`, and `SF6` for the Forgotten Five (**Retsu**, **Geki**, **Joe**, **Mike**, and **Lee**) and every missing modern era slot for **Sagat**, **Gen**, **Adon**, **Birdie**, and **Eagle** when `enable_what_if_styles` is enabled.

Every **tournament fighter** can now be selected in every major *Street Fighter* generation from 1987 through *Street Fighter 6*. Each `[WHAT-IF]` Era Family exposes its flagship revision and full Loadout options: `SF3 3S` (Parry, universal overhead, EX specials, and selectable Super Art I, II, or III), `USF4` (Focus Attack, FADC, Red Focus, Delayed Wakeup, and Ultra I, II, or Ultra Combo Double), `SF5 CE` (Crush Counters, V-Reversal, V-Shift, and selectable V-Skill I/II and V-Trigger I/II), and `SF6` (6-bar Drive Gauge, Overdrive, Drive Parry, Drive Impact, Drive Rush, and Level 1–3 Super Arts, including Joe's *Meteor Kick* and Lee's *One Man Army* Level 3 finishers). Veterans missing specific modern eras (such as **Sagat**, **Gen**, **Adon**, **Birdie**, and **Eagle** in `SF3 3S`, or **Sagat**, **Gen**, **Adon**, **Birdie**, **Eagle**, **Geki**, **Joe**, **Mike**, and **Lee** in `SF6`) receive complete flagship kits rendered by default in their 1987 outfits (using 1987 punk Birdie, 1987 Gen, and unscarred 1987 Sagat pose references).

### Acceptance criteria

- [ ] Deliver `SF3 3S` (with selectable Super Arts I/II/III), `USF4` (with selectable Ultras I/II/Double), `SF5 CE` (with selectable V-Skills I/II and V-Triggers I/II), and `SF6` (with 6-bar Drive Gauge and Level 1–3 Super Arts) **what-if styles** for **Geki**, **Joe**, **Mike** (6-punch boxer layout), and **Lee**, plus `SF3 3S`, `USF4`, and `SF5 CE` **what-if styles** for **Retsu** (connecting cleanly to his canonical `SF6` style).
- [ ] Deliver the missing flagship **what-if styles** among `SF3 3S`, `USF4`, `SF5 CE`, and `SF6` for **Sagat**, **Gen**, **Adon**, **Birdie**, and **Eagle**, preserving their 1987 default outfits (alongside optional **era costumes**).
- [ ] Verify across the scenario session seam that all `[WHAT-IF]` styles remain hidden and unselectable when `enable_what_if_styles` is `false`, and execute their era-specific gauges, loadouts, cancels, and Level 3 / Ultra / Critical Art finishers deterministically when `true`.
- [ ] Validate all newly synthesized 1987-style animation tiles and 8 kHz ADPCM/FM audio clips through the offline asset pack validator.

### Blocked by

- M2
- O17 (#108)

---

## M4: Ansatsuken & Martial Masters Bonus Challengers (Gouken, Akuma, Evil Ryu '87, Goutetsu, Sheng Long)

### Parent

historical issue #113

### What to build

Deliver the first five optional 1987 **bonus challengers** when `enable_bonus_challengers` is enabled: **Gouken**, **Akuma (Gouki)**, **Evil Ryu ('87)**, **Goutetsu**, and **Sheng Long**.

Each challenger is rendered strictly in `384×224`, 15-color-plus-transparent `xRGB_444` 1987 arcade pixel art (`16×16` 4bpp tiles) with 8 kHz MSM5205 ADPCM voice callouts, a dedicated 1987-styled stage variant/remix (such as a stormy/nighttime Japan Temple, Dojo, or Great Wall Courtyard), and an authentic 1987 YM2151 FM character theme arrangement (with Jukebox Red Book CD support). Every challenger includes a `[WHAT-IF] SF1` style modeled on Ryu and Ken's complete 1987 player state-machine rules (full crouch, jump, and block states governed by active **core toggles**, 1987 special damage, and 26-frame invulnerable hitstun with no artificial boss state lockouts), a balanced `SF1+` style with full 6-button normals, and their canonical and `[WHAT-IF]` flagship styles across `SF2` through `SF6`.

### Acceptance criteria

- [ ] Add **Gouken** (*Gohadoken*, *Senkugoshoha*, *Tatsumaki Gorasen*, *Forbidden Shoryuken*), **Akuma** (*Gohadoken*, *Zanku Hadoken*, *Goshoryuken*, *Hyakkishu*, *Shun Goku Satsu*), **Evil Ryu ('87)** (*Hadoken*, *Shakunetsu*, *Ashura Senkuu*, chest-scarring *Metsu Shoryuken*, *Shun Goku Satsu*), **Goutetsu** (ancient Ansatsuken strikes, teleport step, *Kiai* shockwave, lethal counter-throws), and **Sheng Long** (*Flying Dragon Fist*, flaming rising dragon kick, teleports) to the `[BONUS '87]` roster row when `enable_bonus_challengers` is `true`.
- [ ] Provide `[WHAT-IF] SF1` (using Ryu/Ken's complete 1987 player state-machine rules under active **core toggles**), `SF1+`, and canonical/flagship `[WHAT-IF]` `SF2`–`SF6` **era styles** for all five challengers.
- [ ] Include a dedicated 1987-styled stage variant and 1987 YM2151 FM theme arrangement (plus Jukebox CD support) for each of the five challengers.
- [ ] Verify across the scenario session seam that all five challengers can fight any **tournament fighter** or **bonus challenger** across cross-era matchups with bit-exact state-snapshot round-trips, and pass the 1987 tile/audio asset validator.

### Blocked by

- M3

---

## M5: Street Fighter '89 (Final Fight) & Saikyo Bonus Challengers (Gou Hibiki, Dan Hibiki, Cody Travers, Guy, Mike Haggar)

### Parent

historical issue #113

### What to build

Deliver five additional 1987 **bonus challengers** when `enable_bonus_challengers` is enabled: **Gou Hibiki** (Dan's father prior to his fatal duel with Sagat), **Dan Hibiki**, **Cody Travers** (1987 Metro City street brawler), **Guy** (Bushinryu ninja rival to Geki), and **Mike Haggar** (former pro-wrestler grappler rival to Birdie).

Each challenger is synthesized in authentic `384×224`, 15-color-plus-transparent 1987 pixel art with 8 kHz ADPCM voices, a dedicated 1987-styled stage variant (such as a Thailand Shrine variant for Gou/Dan and 1987-tiled Metro City Alley and Wrestling Ring stages for Cody, Guy, and Haggar), and an authentic 1987 YM2151 FM character theme arrangement. Each challenger includes `[WHAT-IF] SF1` (complete Ryu/Ken 1987 player state-machine rules), `SF1+` (full 6-button normals), and their canonical and `[WHAT-IF]` flagship `SF2`–`SF6` **era styles**, plus optional 1987-pixel-art **era costumes** (such as Gou Hibiki's Tengu mask toggle and Cody's `SFZ3` prison and `SF5` Mayor outfits).

### Acceptance criteria

- [ ] Add **Gou Hibiki** (proto-Saikyo counters, one-handed *Gadoken*, *Koryuken*, *Haoh Gadoken*, Tengu mask toggle), **Dan Hibiki** (*Gadoken*, *Koryuken*, *Dankukyaku*, rolling taunts, *Hissho Buraiken*, *Legendary Taunt*), **Cody Travers** (*Bad Stone*, brawler combo, *Criminal Upper*, *Ruffian Kick*, *Final Destruction*, plus `SFZ3`/`SF5` **era costumes**), **Guy** (*Bushin Senpukyaku*, *Hayagake* run follow-ups, *Hozanto*, *Bushin Izuna Otoshi*), and **Mike Haggar** (*Double Lariat*, *Violent Axe*, *Flying Piledriver*, *Spinning Piledriver*, *Giant Haggar Press*) when `enable_bonus_challengers` is `true`.
- [ ] Provide `[WHAT-IF] SF1`, `SF1+`, and canonical/flagship `[WHAT-IF]` `SF2`–`SF6` **era styles** for all five challengers.
- [ ] Include a dedicated 1987-styled stage variant and 1987 YM2151 FM character theme arrangement (plus Jukebox CD support) for each of the five challengers.
- [ ] Verify all five challengers across the frame-stepped scenario session seam (including Haggar's command throws and cross-era throw-loop protection) and the 1987 tile/audio asset validator.

### Blocked by

- M3

---

## M6: Pre-Shadaloo & Alpha Bridge Bonus Challengers (Charlie Nash, M. Bison, Chun-Li '87)

### Parent

historical issue #113

### What to build

Complete the 25-fighter expanded roster by delivering the final three 1987 **bonus challengers** when `enable_bonus_challengers` is enabled: **Charlie Nash** (pre-*Alpha* military investigator), **M. Bison (Dictator)** (who recruits Sagat and Birdie into Shadaloo immediately after the 1987 tournament), and **Chun-Li ('87)** (young martial artist trained in China by 1987 boss **Gen**).

Each challenger is rendered in authentic `384×224`, 15-color-plus-transparent 1987 pixel art with 8 kHz ADPCM voices, a dedicated 1987-styled stage variant (US Harbor/Airfield variant for Charlie, Thailand Shadaloo Temple variant for M. Bison, and China Courtyard variant for Chun-Li), and a 1987 YM2151 FM character theme arrangement. Each challenger includes `[WHAT-IF] SF1` (complete Ryu/Ken 1987 state-machine rules), `SF1+`, and their canonical and `[WHAT-IF]` flagship `SF2`–`SF6` **era styles**, along with optional 1987-pixel-art **era costumes** (Charlie's `SF5` stitched outfit; Chun-Li's classic `SF2` qipao and `SF6` master outfit).

### Acceptance criteria

- [ ] Add **Charlie Nash** (one-handed *Sonic Boom*, *Somersault Justice*, *Moonsault Slash*, *Crossfire Blitz*, plus `SF5` **era costume**), **M. Bison** (*Psycho Crusher*, *Double Knee Press*, *Head Press* / *Somersault Skull Diver*, *Psycho Shot*, cape intro), and **Chun-Li ('87)** (*Hyakuretsukyaku*, *Kikoken*, *Spinning Bird Kick*, *Senretsukyaku*, default 1987/*Alpha* training suit plus `SF2` and `SF6` **era costumes**) when `enable_bonus_challengers` is `true`.
- [ ] Provide `[WHAT-IF] SF1`, `SF1+`, and canonical/flagship `[WHAT-IF]` `SF2`–`SF6` **era styles** for all three challengers.
- [ ] Include a dedicated 1987-styled stage variant and 1987 YM2151 FM character theme arrangement (plus Jukebox CD support) for each of the three challengers.
- [ ] Verify all three challengers across the scenario session seam and the offline 1987 tile/audio asset validator.

### Blocked by

- M3

---

## M7: Bonus Challenger Arcade Mode Campaigns, Secret Rival Intrusions, and 1987 Ending Cinematics

### Parent

historical issue #113

### What to build

Enable all thirteen **bonus challengers** to play through the full 5-country Arcade Mode ladder as Player 1 with bespoke 1987-style pixel-art rival dialogues and character endings, and implement performance-triggered **"Here Comes A New Challenger!"** secret rival intrusions in Arcade Mode under the **Bonus Rival Exhibition Rule**.

When a player selects any of the thirteen **bonus challengers** in Arcade Mode, they choose their starting country (Japan, USA, China, England), battle the ten tournament opponents through Thailand (Adon and Sagat) plus bonus stages, and unlock a unique 1987-pixel-art rival dialogue and character ending (such as Gou Hibiki's "what-if" survival ending after defeating Sagat, or M. Bison recruiting Sagat into Shadaloo). Furthermore, during any Arcade Mode run when both `enable_bonus_challengers` and `enable_secret_arcade_intrusions` are `true`, meeting performance milestones (such as Perfect rounds, no-continues streaks, or Super finishes) triggers a lore-matched **"Here Comes A New Challenger!"** rival intrusion stage before or after Thailand without ever replacing the ten standard country opponents. Under the **Bonus Rival Exhibition Rule**, losing to an intruding **bonus challenger** never costs a credit or triggers a Game Over; the player is offered an optional instant Rematch or advances cleanly to the next scheduled stage.

### Acceptance criteria

- [ ] Allow all thirteen **bonus challengers** to play the full 5-country Arcade Mode campaign (including bonus stages) and display bespoke 1987-pixel-art rival dialogues and character ending cinematics upon defeating Sagat.
- [ ] Trigger lore-matched **"Here Comes A New Challenger!"** secret rival intrusions in Arcade Mode when `enable_bonus_challengers` and `enable_secret_arcade_intrusions` are `true` and performance thresholds are met, while keeping all ten country opponents intact on the ladder.
- [ ] Enforce the **Bonus Rival Exhibition Rule** on secret intrusions: losing to an intruding **bonus challenger** does not consume a credit or end the run, offering an instant Rematch option or immediate progression to the next ladder stage (or ending).
- [ ] Verify across the scenario session seam that disabling `enable_secret_arcade_intrusions` (or `enable_bonus_challengers`) suppresses all secret intrusions even when performance thresholds are met.

### Blocked by

- M4
- M5
- M6

---

## M8: Expanded Roster Integration for Utility AI, 2v1 Dramatic Battle Lore Episodes, 1987 World Tour, and Online Room Browser Filters

### Parent

historical issue #113

### What to build

Integrate all `[WHAT-IF]` **era styles** and the thirteen **bonus challengers** into the Style-Aware Utility AI, Training Mode combo trials, **2v1 Dramatic Battle**, **1987 World Tour**, and online Rollback Netplay matchmaking and Room Browser filtering.

The Style-Aware Utility AI drives CPU opponents across all `[WHAT-IF]` styles and **bonus challengers** (using their rekkas, command throws, teleports, Parries, FADCs, V-Triggers, and Drive mechanics in Arcade, Versus, CPU vs. CPU, World Tour, and Dramatic Battle). Training Mode adds character-specific combo trials for the Forgotten Five's **what-if styles** and the thirteen **bonus challengers**. When `enable_bonus_challengers` is active, **2v1 Dramatic Battle** and **1987 World Tour** unlock preset 1987 lore episodes (*Ryu & Ken vs. Akuma / Gouken*, *Gouken & Retsu vs. Goutetsu*, *Cody & Guy vs. Mike Haggar*, *Gen & Chun-Li ('87) vs. M. Bison*, and *Sagat vs. Gou Hibiki & Dan*), and all thirteen **bonus challengers** can play through the 1987 World Tour mission map. In online Rollback Netplay, `enable_what_if_styles` and `enable_bonus_challengers` are synchronized in the host's `MatchConfig` at handshake and exposed as visible room badges and search filters in the Open Room Browser.

### Acceptance criteria

- [ ] Extend the Style-Aware Utility AI and Training Mode combo trials to cover all `[WHAT-IF]` **era styles** and all thirteen **bonus challengers**.
- [ ] Unlock the five preset 1987 lore team-up episodes (*Ryu & Ken vs. Akuma / Gouken*, *Gouken & Retsu vs. Goutetsu*, *Cody & Guy vs. Mike Haggar*, *Gen & Chun-Li ('87) vs. M. Bison*, and *Sagat vs. Gou Hibiki & Dan*) in **2v1 Dramatic Battle** and **1987 World Tour** when `enable_bonus_challengers` is `true`, and allow all thirteen **bonus challengers** to play the full 1987 World Tour campaign.
- [ ] Synchronize `enable_what_if_styles` and `enable_bonus_challengers` in the host's `MatchConfig` during online Rollback Netplay handshakes so Character Select enforces identical roster and style availability on both peers.
- [ ] Expose `enable_what_if_styles` and `enable_bonus_challengers` as visible rule badges and search filters in the online Room Browser so players can filter for strict `1987 Tournament 12` rooms or expanded `25-Fighter / What-If` rooms.
- [ ] Verify deterministic rollback snapshot save/restore/resimulation across 2v1 Dramatic Battle and online 1v1 matches featuring **bonus challengers** and **what-if styles**.

### Blocked by

- M7
- O18 (#109)
- O19 (#110)
- O20 (#111)
- O21 (#112)
