# Bonus roster by lore: 28 challengers in three waves

- **Date:** 2026-11-03
- **Status:** approved by the user, 2026-11-03
- **Project:** 6 of the seven the user set on 2026-11-02 (front end; art pipeline; SF1+ on the real engine; era styles and gauges; endings, Dramatic Battle and World Tour; **bonus challengers**; netplay transport)
- **Replaces:** section 5.4 (the 13 bonus challengers) of [`2026-10-28-mugen-integration-and-roster-expansion-design.md`](2026-10-28-mugen-integration-and-roster-expansion-design.md). The rest of that design stands. See section 8.
- **Context:** [`CONTEXT.md`](../../CONTEXT.md), [`2026-11-02-art-pipeline-design.md`](2026-11-02-art-pipeline-design.md), [`2026-11-02-overhaul-front-end-design.md`](2026-11-02-overhaul-front-end-design.md), [`docs/research/mugen-sf1-style-characters.md`](../research/mugen-sf1-style-characters.md)

## 1. Intent

The 2026-10-28 design picked 13 bonus challengers largely by which ones the M.U.G.E.N community had already drawn in a 1987 style. The art pipeline turned out able to make 1987 frames from the ROM's own art, so a ready-made sprite reference no longer decides who can be added. The user wants the roster chosen on purpose instead: every added fighter must belong in 1987, the year of the first tournament.

Alex is the user's example of a fighter who does not belong: he is from Street Fighter III and was a child, or not yet born, in 1987. Gou Hibiki and Dan are the example of fighters who do: their story runs straight through Sagat.

## 2. Goals, non-goals, anti-goals

### Goals

1. **A roster of 40**: the 12 tournament fighters plus 28 bonus challengers, every one admitted by the rules in section 3.
2. **Three waves, released in order**, each wave a closer tie to 1987 than the next (section 4).
3. **Every fighter in a wave fully finished before that wave reaches the select screen** (section 5).
4. **A select screen redesigned once for 40**, with portraits drawn for their tile size (section 6).

### Non-goals

- **Any schedule.** This is new content for after the game is feature complete. There is no deadline and no pressure to finish a wave.
- **Changing the 12 tournament fighters, the default Arcade, or Faithful 1987.**
- **Detailed movesets, frame data, stages and endings.** Each wave's spec settles those when the wave starts.

### Anti-goals (failure even if it works)

- **A fighter who could not have been in 1987**: unborn, a child, or made after 1987.
- **A fan-made or joke character.** Sheng Long is out for this reason.
- **A half-finished fighter on the select screen**, or a "coming soon" tile.
- **A select screen that looks like a spreadsheet**, or one redesigned at every wave.
- **Lore resting on one weak source**, such as the EX series alone or a film.

## 3. Who qualifies

These are the user's rules (2026-11-03) unless marked as a call.

1. **Official characters only.** Anything Capcom or UDON publishes counts: games, profiles, guidebooks, the World Warrior Encyclopedia, UDON comics. No fan creations, no jokes. A character who was never playable still qualifies if Capcom created them (Dorai).
2. **Alive and old enough in 1987, or already dead.** The series has spirits, so the dead are fine (Gou appears as a spirit in *Super Gem Fighter Mini Mix*). The unborn and children are out. Eighteen in 1987 is old enough. Ancient is fine.
3. **Where no birth year is known**, being an adult or a fighter in the Alpha series counts as being one in 1987. A known age beats this rule (Karin, R. Mika and Sakura stay out).
4. **Final Fight's cast counts**, as adults of the same world, except where a known age says otherwise. Maki and Lucia were born in 1973 and are out.
5. **Artificial beings** count only if they were made before 1987. Cammy (born 1974 per the Super SF2 SNES manual) and the Dolls are out.
6. **The EX series is an alternate timeline.** It may fill a gap but may not be a fighter's only source.
7. **Films do not count.** *My call.* Akuma's only tournament tie comes from the 2026 film, so he stays in wave 2.
8. **Masahiko Nakahira's Street Fighter Alpha manga counts as official.** *My call.* It is a Capcom-licensed manga but not UDON.
9. **Rival-battle lists are not ties.** *My call.* A later game's rival fight (Zangief against Birdie in Alpha 3, C. Viper against Ryu) does not make a tie to the 12. Story does.
10. **The cap is 40.** If more fighters qualify than fit, the ones with the most lore set near 1987 win. That means the Alpha games, which the wiki dates to 1987–1990, and Final Fight, which it dates to 1989. Total lore across the series does not decide it.

## 4. The roster

### The 12 tournament fighters (unchanged)

Ryu, Ken, Retsu, Geki, Joe, Mike, Lee, Gen, Birdie, Eagle, Adon, Sagat.

### Wave 1: tied to the tournament itself (2)

Something happened to them at the 1987 tournament, or because of it.

| Fighter | Tie | Sources |
|---|---|---|
| **Evil Ryu** | In the final, Sagat is about to win when Ryu, without knowing it, taps the Satsui no Hado and lands the Metsu Shoryuken that scars Sagat's chest. Evil Ryu is the Ryu of that moment. | Nakahira's Alpha manga; the main-series Satsui no Hado lore (Sagat knows that power made his scar); Bison's main-series relationship with Sagat names the Metsu Shoryuken. EX's background says the same as a supporting source. |
| **M. Bison** | Recruits the beaten Sagat into Shadaloo, promising him revenge on Ryu. | Bison's main-series relationship with Sagat. EX states the recruitment outright as a supporting source. UDON's version, where Sagat worked with Shadaloo before the tournament, supports the tie. |

### Wave 2: tied to one of the 12 (10)

| Fighter | Tie | 1987 status |
|---|---|---|
| **Gou Hibiki** | Took Sagat's right eye; Sagat killed him. That is why Sagat wears the patch in 1987. | Dead (the wiki dates the duel to 1977); appears as a spirit |
| **Dan Hibiki** | Gou's son. Sets out after Sagat in 1987. Trained under Gouken, who expelled him. | Adult: a child in 1977, about Ryu and Ken's age. *My call*: the wiki gives no birth year. |
| **Gouken** | Master of Ryu and Ken; friend of Retsu. | Alive |
| **Akuma** | Gouken's brother; tied to Ryu's Satsui no Hado; a rival of Gen. | Alive |
| **Goutetsu** | Master of Gouken and Akuma. UDON has Retsu helping him. | Dead, killed by Akuma |
| **Chun-Li** (young) | Gen was her father's old friend and, in the UDON comics, her teacher. | 19 (born 1 March 1968) |
| **Guile** | Ken's brother-in-law. Eagle fights him in Eagle's story in the handheld Alpha 3. | 26 (born 23 December 1960) |
| **Dhalsim** | Retsu's friend. Sagat trains with him in UDON and *Street Fighter X Tekken*. | 34 (born 22 November 1952) |
| **Oro** | Retsu knew him. | Over 140 in SF3 (about 1997) |
| **Dorai** | Chun-Li's father, Gen's old friend, from whom he learned the Genden kick. Never playable: his moveset is ours to design, starting from that kick. | Adult |

### Wave 3: existed in 1987 (16)

Ranked by lore near 1987.

| # | Fighter | Lore near 1987 |
|---|---|---|
| 1 | **Guy** | Final Fight; his own story in all three Alpha games |
| 2 | **Cody** | Final Fight; Alpha 3 |
| 3 | **Rolento** | Final Fight boss; Alpha 2 and 3, with his own story |
| 4 | **Sodom** | Final Fight boss; Alpha 1 to 3 |
| 5 | **Rose** | Alpha 1 to 3, built around her opposing Bison |
| 6 | **Charlie Nash** | Alpha 1 to 3, the Shadaloo investigation |
| 7 | **Mike Haggar** | Final Fight: the mayor's war on Mad Gear |
| 8 | **Zangief** | Alpha 2 and 3 story |
| 9 | **Vega** | Alpha 3, a Shadaloo agent |
| 10 | **Balrog** | Alpha 3, a Shadaloo agent |
| 11 | **Zeku** | The Bushin master who trained Guy. He is from SF5, but his story is the past. |
| 12 | **Belger** | Mad Gear's boss; Final Fight's final boss |
| 13 | **Poison** | Final Fight, and the most-used of its cast after the three heroes |
| 14 | **Hugo** | As Final Fight's Andore, which the wiki treats as Hugo's first appearance. His SF3 look is an era costume. *My call.* |
| 15 | **Abigail** | Final Fight's stage 5 boss; playable in SF5 |
| 16 | **Damnd** | Final Fight's stage 1 boss; his kidnapping of Jessica starts the plot |

The user chose to give the last five slots to Final Fight over the SF2 cast in Alpha 3. Final Fight shares SF1's era: it began as *Street Fighter '89*, before SF2. Its cast also gets little representation in later games.

### Alternates, in order

If a wave 3 fighter drops out (a lore check fails, or the user swaps one), the next in line takes the slot. *My call* on the order:

1. **Edi E.**, the Metro City cop on Mad Gear's payroll.
2. **Blanka.** His years in the jungle come before 1987.
3. Then, unranked: E. Honda, Fei Long, T. Hawk, Dee Jay, Carlos, Genryusai, Dean, Necalli, C. Viper, Juri.

If a wave 1 or wave 2 fighter fails their last lore check, that wave gets smaller and wave 3 takes the next alternate, so the roster stays at 40. No fighter moves to an earlier wave to fill a gap, because a wave is defined by its tie. *My call.*

## 5. Delivery

1. **This comes after the game is feature complete**, as new content. It does not block the port, the overhaul, or the six other projects. Era styles and gauges, including what-if styles for the 12 and the `enable_what_if_styles` switch, belong to project 4. They are in place before wave 1 starts, so wave 1 builds on them and does not deliver them.
2. **A wave ships whole.** No fighter in a wave reaches any select screen until every fighter in that wave is fully finished.
3. **"Fully finished"** (user, 2026-11-03) means everything the 2026-10-28 design promised a bonus challenger:
   - 1987-style sprites and select portraits made by the art pipeline;
   - SF1 and SF1+ styles;
   - every era style the fighter would have: their canon later era styles, plus what-if styles for the eras they missed;
   - a stage, a theme, voice and sound effects;
   - an Arcade ending and rival dialogue;
   - their place in secret-challenger intrusions, Dramatic Battle and World Tour.

   For a fighter who was never playable (Dorai), every later era style is a what-if.
4. **Waves release in order: 1, 2, 3.**
5. **Wave 1 adds to Arcade, as an option.** Evil Ryu is the Ryu who scarred Sagat, and Bison appears after Thailand. Both appear only as secret challengers, when `enable_bonus_challengers` and `enable_secret_arcade_intrusions` are on, under the Bonus Rival Exhibition Rule: losing to them costs nothing. The default Arcade and its 10 country opponents do not change (user, 2026-11-03). Wave 1's spec designs the exact shape.
6. **SF1 and SF1+ are native styles for every bonus challenger**, because this is their 1987 debut. They are not what-ifs. `enable_what_if_styles` covers only the later eras a fighter missed, so all 28 appear whenever `enable_bonus_challengers` is on (user, 2026-11-03). This replaces the 2026-10-28 design's `[WHAT-IF] SF1` marking for bonus challengers.
7. **Each wave gets its own spec and tickets when work on it starts**, not before.
8. **One switch.** `enable_bonus_challengers` covers all three waves, and the select screen filter stays two-way: the 12, or everyone released so far. No switch per wave (user, 2026-11-03, grilling).
9. **Each wave release has its own name** (user, 2026-11-03, grilling):

   | Wave | Release name |
   |---|---|
   | 1 | *Super Street Fighter: Rising* |
   | 2 | *Hyper Street Fighter: Bloodlines* |
   | 3 | *Ultra Street Fighter: Final Round* |

   This moves *Hyper* off era styles. The era-styles edition (project 4) is now *Street Fighter: The World Tournament, Anniversary Edition*, after *Hyper Street Fighter II: The Anniversary Edition*, which let each fighter pick any revision. The edition lineage in `CONTEXT.md` records both.

## 6. The select screen

The select screen is redesigned once, for 40, and that redesign ships with wave 1. Later waves fill slots in a layout that already holds the full roster.

**Why it must change.** In Local today, the two 102-pixel busts take the top of the screen. The 12 tiles (46×30, in two rows of six) fill the whole inside of the frame, down to the preset line (`TWO_PLAYERS` in [`select.rs`](../../crates/sf-engine/src/front/select.rs); frame constants in [`draw.rs`](../../crates/sf-engine/src/front/draw.rs)). Arcade has a little spare room. Local has none.

**Capacity.** With tiles cropped from the busts, about 30 still looks good. With portraits drawn for the tile size, as the CPS2 games did, about 36–40 looks good and 48 is crowded. The user chose portraits drawn for the tile size, and the cap of 40 comes from that.

**What is decided:** one redesign; drawn-to-size portraits made by the art pipeline; room for 40; the 1987 look the front end already uses (the gold frame, ROM fonts, P1 yellow and P2 blue).

**The layout, from the 2026-11-03 mockup (user's choice): a centre grid with frames on either side, in both Arcade and Local.**
- **Grid.** Up to 8 columns by 5 rows of 26×30 tiles on a 27×32 pitch, centred between two tall frames (64×108) on the full-width frame.
- **Local.** P1's frame is on the left and P2's on the right. Each fighter's name and roster wave sit under their frame.
- **Arcade.** The left frame is P1's, and the right column lists the fighter's era styles, with SF1 and SF1+ lit as native.
- **Partial rosters, "even rows".** Only released fighters get tiles. The grid takes the widest column count (8, 7 or 6) that fills every row, and centres the block:
  - the 12 alone: 6×2;
  - after wave 1 (14 fighters): 7×2;
  - after wave 2 (24): 8×3;
  - after wave 3 (40): 8×5.
- **Tile size** stays the same for every wave, so each portrait is drawn at one size only.

The mockup is a throwaway prototype, kept on the branch `prototype/select-40`.

**What wave 1's spec decides:** the exact pixel positions, whether the 26×30 tile holds up with real portraits, and how the filters from the 2026-10-28 design (the 12 only, or everyone) carry over.

## 7. Approaches considered

1. **A new design that replaces only section 5.4 of the 2026-10-28 design (chosen).** The phased roster gets a record of its own. The rest of the old design stands, and each wave is planned when it starts. This suits content with no deadline.
2. **Edit the 2026-10-28 design in place.** One less document, but it rewrites an approved design, mixes M.U.G.E.N-first and lore-first reasoning, and loses the record of why the list changed.
3. **Plan all three waves in full now, with specs and tickets.** It locks in detail for work years away, which would go stale as the art pipeline and the overhaul change.

The user chose approach 1 on 2026-11-03.

## 8. What this changes elsewhere

- **The 2026-10-28 design, section 5.4**, is replaced by section 4 here. Its toggles (`enable_bonus_challengers`, `enable_secret_arcade_intrusions`), the Bonus Rival Exhibition Rule and the idea of lore episodes stand. The episodes listed there are rewritten per wave. *Cody & Guy vs. Mike Haggar* survives as written. *Gouken & Retsu vs. Goutetsu* survives, since dead fighters are allowed. Episodes built on dropped ties do not survive: Guy as Geki's rival and Haggar as Birdie's rival were never canon. The "25 roster" wording becomes 40.
- **`CONTEXT.md`, Bonus challenger:** the example list loses Sheng Long and points here.
- **The M4–M8 tickets (#117–#121) and `bonus*.rs`** are closed history on the standalone combat model, kept as reference material. Nothing here reopens them. Sheng Long's tables there are reference code only and are not carried forward.
- **[ADR-0008](../adr/0008-bonus-roster-chosen-by-lore.md)** records the selection rules and the cap.
- **ADR-0003's M.U.G.E.N extraction** stays an option for frame data. It no longer decides the roster.

## 9. Left to the builder (each wave's spec)

- Each fighter's moveset and frame data, sourced from the games they appear in (or designed, for Dorai and for what-if styles).
- Which era styles count as canon for each fighter, and which are what-ifs.
- The select screen layout within section 6's constraints.
- Stages, themes, endings, rival dialogue and intrusion triggers.
- A last lore check per fighter before their wave starts, against the rules in section 3.

## 10. Sources

Lore was checked on the Street Fighter Fandom wiki (read through its MediaWiki API, since its pages block direct fetches), using the Capcom profiles, encyclopedias and manuals the wiki cites. Some SF2-era birth dates were cross-checked on secondary sites. Where only the wiki states a fact, the row says so or it is marked as a call. Research notes from 2026-11-03 are summarised here and were not kept as separate files.
