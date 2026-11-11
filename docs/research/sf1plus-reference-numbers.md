# SF1+ reference numbers: later Capcom data, design theory and fair CPU

Research for making the ten 1987 bosses playable beside Ryu and Ken at a **classic 1987-rules level**, a **modern level** (true combos, normal-to-special cancels, techable throws), an optional **balanced roster** layer, and a **fair, human-like CPU**. Gathered 2026-11-07.

**The ROM is the authority for 1987 values.** Everything below about 1987 from the web is secondary, and section 0 lists what the repo's own reference docs already settle.

## How to read this file

- **Sources.** Modern frame data (SF6, SFV, USF4) comes from the FAT frame-data app's published JSON on GitHub: [SF6FrameData.json](https://github.com/D4RKONION/FAT/blob/main/src/js/constants/framedata/SF6FrameData.json) (last commit `a8101eb02d`, 2026-10-22), [SFVFrameData.json](https://github.com/D4RKONION/FAT/blob/main/src/js/constants/framedata/SFVFrameData.json) (`dee6b85a7f`, 2025-02-17) and [USF4FrameData.json](https://github.com/D4RKONION/FAT/blob/main/src/js/constants/framedata/USF4FrameData.json) (`0ee248cda4`, 2025-08-01). FAT's README says the data comes from the team's own spreadsheet and corrections go through them ([FAT README](https://github.com/D4RKONION/FAT)). Older games (ST, Alpha 2, Alpha 3, CvS2) and system pages come from the SuperCombo wiki. Their HTML pages sit behind bot protection, so I read the same pages' MediaWiki wikitext (`index.php?title=…&action=raw`). Links below point at the normal page URLs.
- **What I could not open.** Capcom's official SF6 frame page ([Sagat](https://www.streetfighter.com/6/en-us/character/sagat/frame)) returned HTTP 403 to automated fetches, and the in-app browser could not read pages in this session. SF6 numbers therefore come from FAT, not from Capcom directly. I have not cross-checked them against the in-game frame meter.
- **Startup conventions differ.** SF6, SFV and USF4 (FAT) and SuperCombo's Alpha 2 tables count startup **including the first active frame** ("FAF"). A 5f move hits on frame 5 ([SF6 Game Data](https://wiki.supercombo.gg/w/Street_Fighter_6/Game_Data); [SFA2 Glossary](https://wiki.supercombo.gg/w/Street_Fighter_Alpha_2/Glossary)). SuperCombo's ST and Alpha 3 tables list animation-frame counts, often in "true startup" (frames *before* the first active frame), so add 1 to compare. The SF6 Game Data page warns about exactly this.
- **Damage units differ by game.** SF6 health is about 10,000 (so 100 = 1%). SFV and USF4 health is about 1,000. CvS2 values are in hundreds. Alpha 2/3 and ST use small integers on their own scales. Compare damage within a game, or as a percentage of health where a health figure is given.
- **Frame advantage** is attacker minus defender: + means the attacker recovers first.

---

## 0. What the repo already settles (1987, from the ROM)

| Topic | Repo doc | What it says |
| --- | --- | --- |
| Buttons latch on release | [punch.md](../reference/punch.md) | A punch starts one frame after the button is *released*. Holds of 1, 5, 30 and 60 frames all started on release + 1, and a hold past 255 frames never attacks. |
| Hit reaction (why 1987 has no combos) | [punch.md](../reference/punch.md) | Ken's stun stream runs 9 + 16 + 1 = **26 frames**, with a +0x20 pushback, **invulnerable throughout** (`+0xf = 0x08`); then the stance restarts. This matches the README's "26-frame player hitstun". |
| Command window | [specials.md](../reference/specials.md) | Each command tracker completes when its third direction arrives within **0x12 = 18 frames** of the first, and stays done while that direction is held. |
| Shoryuken | [specials.md](../reference/specials.md) | Shoryuken sets the invulnerable flag (`+0xf` bit 3). |
| Fireball hit slow motion | [hadoken.md](../reference/hadoken.md) | An unblocked projectile hit runs the KO slow-motion handler: sleeps of 60 then 30 frames, during which the fight loop advances one step every **6 frames** instead of every frame. |
| Round timer | [round-timer.md](../reference/round-timer.md) | TIME ticks every **19 frames**. |
| Blocking | [blocking.md](../reference/blocking.md) | High/low block entry rules for both players. |
| CPU setup and decisions | [ai-tables.md](../reference/ai-tables.md), [cpu-decision.md](../reference/cpu-decision.md), [cpu-follow-up.md](../reference/cpu-follow-up.md) | Per-opponent, per-difficulty AI tables. The follow-up decision tests whether **the player is attacking (+9 bit 6)**. The overhaul design notes this branch reads that bit on frame 1, i.e. instant input reading ([overhaul design](../design/2026-10-27-definitive-overhaul-design.md)). |
| Each boss's own hit functions | [retsu-reactions.md](../reference/retsu-reactions.md), [geki.md](../reference/geki.md), [joe.md](../reference/joe.md), [mike.md](../reference/mike.md), [lee.md](../reference/lee.md), [gen.md](../reference/gen.md), [birdie.md](../reference/birdie.md), [eagle.md](../reference/eagle.md), [adon.md](../reference/adon.md), [sagat.md](../reference/sagat.md) | The ROM's own boss code paths, translated. |

The repo has no 1987 throws (the game has none), no cancels and no combo scaling. Those need outside references, below.

---

## 1. Per-fighter references from later Capcom games

### 1.1 Who has an official playable appearance at all

| Fighter | Official playable appearances found (with data) | Notes |
| --- | --- | --- |
| **Sagat** | SF2 (boss in the original, playable from Champion Edition), Super Turbo, Alpha 1/2/3, CvS2, SF4 series, SFV, SF6 | The SF2-boss note is footnote 2 of [Wikipedia's character list](https://en.wikipedia.org/wiki/List_of_Street_Fighter_characters). SF6 added him as the first Year 3 character on August 5 ([Game Developer press release](https://www.gamedeveloper.com/press-release/embody-sagat-s-fierce-tiger-spirit-in-street-fighter-6-as-the-game-s-first-year-3-character-arrives-on-august-5-)). Data exists for every game here. |
| **Adon** | Alpha 1 (unlockable), Alpha 2, Alpha 3, Super/Ultra SF4 | Wikipedia also lists an SF6 cameo. Not in SFV or SF6 as a fighter: he is absent from FAT's SFV and SF6 rosters. |
| **Gen** | Alpha 2, Alpha 3, Super/Ultra SF4 (two stances: Mantis and Crane) | Wikipedia lists cameos in SFV and SF6. Not playable in SFV or SF6 (absent from FAT's rosters). |
| **Birdie** | Alpha 1, Alpha 2, Alpha 3, SFV | Wikipedia lists an SF6 cameo. Not playable in SF6. |
| **Eagle** | Alpha 3 (SuperCombo: "only playable on the Handheld versions"), CvS2 | SuperCombo's [SFA3 Eagle page](https://wiki.supercombo.gg/w/Street_Fighter_Alpha_3/Eagle) has a move list but **no frame data**. CvS2 is the only data source found. |
| **Retsu** | **None playable.** In SF6's World Tour he is a non-playable opponent. | Wikipedia note 6: "Non-playable opponent in World Tour mode". EventHubs reports his World Tour moveset is pulled mainly from Ryu ([EventHubs, 2023-05-12](https://www.eventhubs.com/news/2023/may/12/street-fighter-1-character-sf6); search-result summary, page not fetched). No frame data published. |
| **Geki, Joe, Lee** | **None.** | Wikipedia's table shows only the 1987 CPU-only appearance for each ("All characters in the original Street Fighter other than Ryu and Ken are CPU-only opponents"). |
| **Mike** | **None playable.** | Wikipedia's row shows one later "Cameo" entry. The scraped column alignment was off, so I could not confirm which game. |

**Fan, not official:** the "Yumeji Hack SF1++" (2014) makes the ten CPU fighters playable in the 1987 ROM ([SuperCombo: Street Fighter (1987)](https://wiki.supercombo.gg/w/Street_Fighter_(1987))). Its community writes the only "playable" data for Retsu, Geki, Joe, Mike and Lee (section 2). Mizuumi has a "Street Fighter 1 MR" page ([Adon](https://mizuumi.wiki/w/Street_Fighter_1_MR/Adon)) that I could not open (bot protection), so I can't say what it is.

### 1.2 Sagat

**SF6** (FAT; FAF startup; damage out of 10,000 HP, so 100 = 1%):

| Move | Startup | Active | Recovery | On hit | On block | Damage | Notes (FAT) |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 5LP | 5 | 3 | 11 | +4 | -3 | 300 | range 1.26, "great range for a 5F jab" |
| 2LP | 4 | 2 | 10 | +5 | -1 | 300 | chains into lights |
| 5LK | 7 | 2 | 15 | 0 | -4 | 400 | range 1.56; whiff-punish tool |
| 5MK | 11 | 4 | 19 | +3 | -3 | 700 | range 1.87; "extends a wide leg hurtbox" |
| 2MK | 9 | 3 | 18 | +5 | -2 | 600 | range 1.66; "amazing hitbox priority / disjoint for a low poke" |
| 5HP | 15 | 4 | 22 | 0 | -5 | 800 | range 2.17; target combo |
| 5HK | 10 | 2/4 | 21 | +1 | -5 | 900 (2 hits) | upper body and kicking leg invulnerable to airborne strikes on f9–15 (anti-air) |
| 2HK | 11 | 3 | 26 | KD +29 | -12 | 900 | range 1.91 |
| LP Low Tiger Shot | 14 | – | 36 | -3 | -7 | 600 | projectile speed 0.105; can't be crouched under |
| MP High Tiger Shot | 16 | – | 26 | +3 | +3 | 700 | speed 0.08; whiffs on crouchers |
| HP High Tiger Shot | 12 | – | 30 | -1 | -1 | 700 | speed 0.14 |
| OD High Tiger Shot | 21 | – | 27 | KD | +6 | 1000 | 3 hits |
| LP Tiger Uppercut | 5 | 10 | 35+12 | KD +32 | -37 | 1100 | invulnerable to airborne strikes f1–14 |
| HP Tiger Uppercut | 18 | 10 | 40+15 | KD +27 | -45 | 1600 | air-invulnerable f1–21; range 2.38–2.56 |
| OD Tiger Uppercut | 8 | 2×6 | 38+15 | KD +25 | -43 | 1600 | fully invulnerable f1–12 |
| LK Tiger Knee Crush | 14 | 11 | 3+15 | KD +36 | -8 (to +2 spaced) | 800 | airborne f13–27, lower-body projectile-invulnerable |
| MK Tiger Knee Crush | 18 | 11 | 3+15 | KD +39 | -7 (to +3) | 1000 | |
| HK Tiger Knee Crush | 22 | 4(4)11 | 3+15 | KD +40 | -5 (to +5) | 1400 | |
| LK Tiger Nexus | 17 | 3 | 23 | +2 | -5 | 500 | new SF6 knee with follow-ups |

Stats: HP 10,000. Throw range 0.85 against a roster norm of 0.80. Throw hurtbox 0.38. Fastest normal 4f.

**SFV** (FAT; damage out of 1,025 HP): LP Tiger Shot 13f, +1 on block, 70 dmg (cannot hit crouching). LK Grand (low) Tiger Shot 13f, -4. LP Tiger Uppercut 4f, -39, air-invulnerable f1–12. HP Tiger Uppercut 8f, -39. LK Tiger Knee Crush 12f, -5 (+1 spaced). HK Tiger Knee Crush 20f, -7 (+3 spaced). Normals: 5LP 4f +4 block, 2MK 9f -4, 5HK 8f with head and shoulders invulnerable f4–11. Throw range 0.85.

**USF4** (FAT; damage out of 1,050): High Tiger Shot 11f, -5 on block, 65 dmg. Low Tiger Shot 12f, -7. LP/HP Tiger Uppercut 5f, -31 / -34. Tiger Knee LK/MK/HK 7f, -10 / -13 / -18 on block. EX Tiger Knee -1. Normals: far LP 5f +3, cr.LP 3f, far MK 9f 0, cr.MK 7f -4, cr.HK 8f -6. Throw range 0.94.

**CvS2** ([SuperCombo CvS2 Sagat](https://wiki.supercombo.gg/w/Capcom_vs_SNK_2/Sagat)): Tiger Shot LP/MP/HP startup 10, recovery 44–46, -11 / -12 / -13. Low Tiger Shot 10f, -12 to -14. Tiger Uppercut 3f startup, 3 frames full invulnerability (plus lower-body), -35 / -43 / -35. I found **no Tiger Knee data block** on the page, though its text mentions roll-cancelled Tiger Knee.

**Alpha 3** ([SuperCombo SFA3 Sagat](https://wiki.supercombo.gg/w/Street_Fighter_Alpha_3/Sagat); animation-frame counts):

- **Tiger Shot:** 15 frames before release. Block advantage -12 / -13 / -14 (Jab/Strong/Fierce).
- **Ground Tiger Shot:** -13 / -14 / -15.
- **Tiger Uppercut** (X-ism): 6 frames startup, invulnerable 0–7, -27 / -38 / -48.
- **Tiger Crush** (Tiger Knee): 10 / 10 / 11 frames startup, recovery 23, -5 on block.

**Alpha 2** ([SuperCombo SFA2 Sagat](https://wiki.supercombo.gg/w/Street_Fighter_Alpha_2/Sagat); FAF startup; on hit = on block + 1):

- **High Tiger Shot:** 16f, -4 / -5 / -6.
- **Low Tiger Shot:** 16f, -5 / -6 / -7.
- **Tiger Blow** (uppercut): LP 9f -19, MP 5f -19, HP 5f -32.
- **Tiger Crush:** 9 / 9 / 10f, 0 on block against a standing blocker and -5 against a crouching one.

**Super Turbo, N.Sagat** ([SuperCombo ST Sagat](https://wiki.supercombo.gg/w/Super_Street_Fighter_2_Turbo/Sagat); simplified frame counts, true startup):

- **Tiger Shot:** startup 10, recovery 38 / 40 / 42.
- **Ground Tiger Shot:** startup 12, recovery 38 / 40 / 42 (46 / 48 / 50 in Hyper SF2).
- **Tiger Uppercut:** startup 3, recovery 27 / 29 / 35. The Fierce version hits up to 5 times.
- **Tiger Knee:** startup 4 / 4 / 5, then 6 active frames, recovery 20 / 21 / 22. Advantage is listed as "-9 / -4, -11 / -6, -13 / -8" per strength; the row doesn't say what the two values are.
- **Throw range:** 48 px from the character's axis, or **16 px beyond the opponent's throwable box**. The page calls it one of the worst throw ranges.
- **Command windows:** ↓ [7~14f] ↘ [7~14f] → [10/9/7f] for Jab/Strong/Fierce.

**Version notes (designer's own words).** David Sirlin, who rebalanced ST for HD Remix, says Old Sagat's Tiger Shot recovery was "about 12 frames" better than New Sagat's. HD Remix set it "about 4 frames worse than" Old Sagat's, which is similar to Ryu's. HD Remix also made Tiger Knee a DP motion that always knocks down (allowing a 3-hit juggle), with damage and dizzy reduced to pay for it ([sirlin.net/sf-hdr/sagat](https://sirlin.net/sf-hdr/sagat)).

**Pattern across versions.** The low Tiger Shot is slightly slower or worse than the high one. The Tiger Uppercut is 3–8f with invulnerability and very unsafe (-27 to -48). The Tiger Knee is 4–22f depending on era; it is a near-safe frame-trap tool in SF2/Alpha and becomes minus-but-spacing-dependent in SFV/SF6.

### 1.3 Adon

**USF4** (FAT; health 950, below average):

| Move | Startup | Active | Recovery | On block | Damage |
| --- | --- | --- | --- | --- | --- |
| Far LP | 3 | 3 | 6 | +2 | 30 |
| Crouch MK | 6 | 2 | 14 | -2 | 70 |
| Far HP | 8 | 3 | 14 | +1 | 120 |
| LK Jaguar Kick | 20 | 6 | 21 | -3 (hit -2) | 130 |
| HK Jaguar Kick | 22 | 8 | 20 | -4 | 120 |
| EX Jaguar Kick | 20 | 2,3 | 22 | -2 | 80+70 |
| LK Rising Jaguar | 4 | 2(6)4 | 30 | -13 | 50+80 |
| MK Rising Jaguar | 5 | 2(6)5 | 35 | -19 | 90+50 |
| HK Rising Jaguar | 5 | 2(6)9 | 35 | -23 | 50+100 |
| EX Rising Jaguar | 5 | 4(8)2,6 | 34 | -33 | |
| LK/MK/HK Jaguar Tooth | – | 4/5/7 | – | -4 / -1 / -2 | 120 |

FAT notes: LK Jaguar Kick goes the longest distance with the lowest apex, HK the shortest with the highest. Throw range is 0.93 forward and 1.05 back.

**Alpha 3** ([SuperCombo SFA3 Adon](https://wiki.supercombo.gg/w/Street_Fighter_Alpha_3/Adon); A-ism and V-ism command Jaguar Kick; in X-ism every air kick is a Jaguar Kick):

- **Jaguar Kick:** total startup 25 / 30 / 31, active 4 / 5 / 9, recovery 17 / 18. Block advantage -2 / +0 / -6. The page says that "when spaced well it can be safe or plus".
- **Rising Jaguar:** Short version starts in 2 frames with invulnerability 0–3 and is -15 on block. Forward and Roundhouse start in 4 frames and are -25 / -30.
- **Jaguar Tooth:** recovery 23.

**Alpha 2** ([SuperCombo SFA2 Adon](https://wiki.supercombo.gg/w/Street_Fighter_Alpha_2/Adon)): Jaguar Kick 22 / 27 / 28f, listed -10 / -8 (two values matching two damage values). Rising Jaguar 3 / 5 / 5f, -17 / -23 / -28. Jaguar Tooth 43–47f.

**Pattern.** Jaguar Kick is a slow (20–31f) jumping kick that is roughly even when spaced. Rising Jaguar is a fast (2–5f) reversal that is very unsafe on block.

### 1.4 Gen

**USF4** (FAT; health 900, the lowest on this list):

| Move | Startup | Active | Recovery | On block | Damage |
| --- | --- | --- | --- | --- | --- |
| (Mantis) 5LP | 3 | 2 | 7 | +3 | 25 |
| (Mantis) 2MK | 7 | 3 | 14 | -3 | 70 |
| (Mantis) LP/MP/HP Hyakurenkou | 5 / 4 / 3 | 21 total | 17 | +5 (hit +8) | 16 per hit; mash to extend |
| (Mantis) LK Gekiro | 5 | 15 | 14+8 land | -35 | 100 + follow-ups |
| (Mantis) MK/HK Gekiro | 7 | 15 | 17–18+8 | -35 | |
| (Mantis) EX Gekiro | 7 | 15 | 18+8 | -20 | |
| (Crane) LP/MP/HP Jyasen | 6 | 8 / 11 / 14 total | 20 | -1 / -1 / -3 | |

Throw range is 0.99 forward and 1.09 back. Gen has a stance switch in SF4; the README's SF1+ plan says "no stance switch in SF1+".

**Alpha 2** ([SuperCombo SFA2 Gen](https://wiki.supercombo.gg/w/Street_Fighter_Alpha_2/Gen)): Hyakurenko 5 / 4 / 3f and **+14 / +13 / +12 on block**. Gekiro 8f; the first hit is -21, the second ranges from -24 at worst to +3 at best. Jyasen 7f, -6 to -9.

**Alpha 3:** the [SFA3 Gen page](https://wiki.supercombo.gg/w/Street_Fighter_Alpha_3/Gen) has normals only (for example close jab 3/4/7, +3; crouch roundhouse 4/1/42, -20) and **no special-move data**.

**Pattern.** The mash punch is fast (3–5f) and plus on block in every version. Gekiro is a DP-like rising kick string and badly unsafe on block.

### 1.5 Birdie

**SFV** (FAT; health 1,050):

| Move | Startup | Active | Recovery | On block | Damage |
| --- | --- | --- | --- | --- | --- |
| 5LK | 4 | 3 | 6 | +3 | 30 |
| 2LP | 5 | 3 | 10 | +2 | 40 |
| 5MK | 9 | 3 | 16 | -2 | 60 |
| 5HK | 12 | 3 | 20 | 0 | 100 |
| 2HK | 12 | 3 | 25 | -14 | 100 |
| LP / MP / HP Bull Head | 14 / 18 / 27 | 3 | 29 / 30 / 24 | -10 / -10 / -5 | 100 / 110 / 140 |
| EX Bull Head | 14 | 4(26)4 | 28 | -5 | 150 |
| Bull Horn | 15 | 6 | 15+14 | -18 | 120 |
| EX Bull Horn | 15 | 9(11)4 | 32 | -21 | 150 (one hit of armor f1–20) |

In SFV, Bull Horn "charges in 50 frames" and is projectile-invulnerable f6–20.

**Alpha 2** ([SuperCombo SFA2 Birdie](https://wiki.supercombo.gg/w/Street_Fighter_Alpha_2/Birdie)):

- **Bull Head** (charge back, forward + P): startup 7–10 / 11–26 / 15–45 depending on strength. Block advantage from -20 to -15 (Jab) down to -26 to -21 (Fierce).
- **Bull Horn** (hold PP or KK, then release): startup 26–60 depending on hold, -5 to 0 on block. Damage grows with hold time: level 1 is 7 and FINAL is 33.

**Alpha 3** ([SuperCombo SFA3 Birdie](https://wiki.supercombo.gg/w/Street_Fighter_Alpha_3/Birdie)): Bull Head -25 / -28 / -31 on block (damage 14 / 15 / 16). Bull Horn -9 / -8 / -6, with damage rising by hold level.

**Pattern.** Bull Head is a slow-ish charging headbutt (14–27f in SFV) that is unsafe on block. Bull Horn is a hold-to-power-up move whose damage scales with hold time.

### 1.6 Eagle (CvS2 is the only data found)

[SuperCombo CvS2 Eagle](https://wiki.supercombo.gg/w/Capcom_vs_SNK_2/Eagle). The counting convention isn't stated on the page. In CvS2 the listed hit and block advantage of normals are identical.

| Move | Startup | Active | Recovery | On block | Damage | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| Far LP | 3 | 4 | 5 | +7 | 300 | |
| Far MK | 4 | 6 | 16 | 0 | | |
| Far HP | 8 | 3 | 38 | -17 | | |
| Far HK | 7 | 3 | 32 | -11 | | |
| Crouch MK | 5 | 6 | 11 | +6 | | |
| Crouch HK | 11 | 4 | 29 | -9 | | sweep, hard knockdown |
| Manchester Black LP / MP / HP | 4 / 5 / 6 | multi-hit spin | 18 | -28 / -52 / -76 | 1200 / 1300 / 1200 | lower-body invulnerable for 4 / 5 / 6 frames |
| Saint Andrew's Green | 14 | 5 | 49 | -30 | 1200 | hard knockdown on hit |
| Canterbury Blue (hold) | 3, then hold 13–73, then 5 on release | 12 | 30–34 | -10 to -14 | 1000–1200 | |
| Oxford Red LK / MK / HK | 16 / 17 / 25 | stab flurry | 28 | -2 / -29 / -41 | 300 + 200×(4 / 6 / 8) | |
| Liverpool White | 36 / 38 / 41 | 4 | 34 | -14 | 1300–1500 | overhead hop |

The page's strategy text says Eagle's best anti-air special "only has upper body invuln on the startup and doesn't go into the first active frame", so players roll-cancel it.

### 1.7 Retsu, Geki, Joe, Mike, Lee

**No official Capcom frame data exists** for any of the five as playable fighters (section 1.1). The only numbers anywhere are SF1++ community notes on the 1987 ROM (section 2), which the repo's own translations supersede.

---

## 2. 1987 Street Fighter Ryu/Ken data online

**What exists:**

- **SuperCombo's Street Fighter (1987) pages, marked "Very WIP".** These are written by the SF1(++) community, for the 2014 Yumeji SF1++ hack ([index](https://wiki.supercombo.gg/w/Street_Fighter_(1987))). The Ryu and Ken pages give these Ryu/Ken differences ([Ryu](https://wiki.supercombo.gg/w/Street_Fighter_(1987)/Ryu), [Ken](https://wiki.supercombo.gg/w/Street_Fighter_(1987)/Ken)):
  - **Medium kick startup:** Ryu 21, Ken 26.
  - **Hadoken end lag:** on hit Ryu 37 vs Ken 94 ("bear in mind there is a slow down effect on hit"); on block 7 vs 19; on whiff 16 vs 30.
  - **Tatsumaki end lag:** Ryu 28, Ken 24.
  - **Shoryuken:** Ryu's is fully invincible. Ken's "is vulnerable for the first 4 frames".
  - **Jump:** both have "an awful nine frame jump squat".
- **SF1++ character pages** add notes on the bosses:
  - **Geki:** "Fastest attack comes out on frame 10". The same page also says "his fastest move is frame 9", so the two figures conflict ([Geki](https://wiki.supercombo.gg/w/Street_Fighter_(1987)/Geki)).
  - **Mike:** "a 39 frame fireball block chip animation and the worst crouch cancel in the game at 30 frames (the rest of the cast is 10)" ([Mike](https://wiki.supercombo.gg/w/Street_Fighter_(1987)/Mike)).
  - **Lee:** for the first 16 frames of his block animation his hurtbox is "lowered dramatically", even low-profiling Hadoken ([Lee](https://wiki.supercombo.gg/w/Street_Fighter_(1987)/Lee)).
  - **Retsu:** his HK is "the highest priority normal in the game" ([Retsu](https://wiki.supercombo.gg/w/Street_Fighter_(1987)/Retsu)).
  - **Joe:** "amazing frame data and hitboxes" but low damage ([Joe](https://wiki.supercombo.gg/w/Street_Fighter_(1987)/Joe)).
  - **Eagle:** "incredible framedata on standing attacks" ([Eagle](https://wiki.supercombo.gg/w/Street_Fighter_(1987)/Eagle)).
- **Controls page:** "All attack buttons will only be registered by the game on release" ([Controls](https://wiki.supercombo.gg/w/Street_Fighter_(1987)/Controls)). This agrees with the repo's [punch.md](../reference/punch.md).
- **Hitbox viewers:** I found **none for the 1987 `sf` set**. The known MAME-rr and FBA-rr Lua hitbox scripts cover SF2 variants, SF3 and CPS-2 games ([TASVideos forum: hitbox scripts](https://tasvideos.org/Forum/Topics/9942)). TCRF documents a prototype of the 1987 game added to MAME in 2002 ([TCRF](https://tcrf.net/Proto:Street_Fighter_(Arcade))).

**How reliable.** The pages don't say how the numbers were measured (frame stepping, RAM watching, or eyeballing), and they describe the hack, which adds character select but should leave move data alone. Treat them as leads to check against the ROM translation, not as values. I have not compared the Hadoken end-lag figures with [hadoken.md](../reference/hadoken.md). The repo's own [community research](community-expectations-and-criticisms.md) already covers the complaints side.

---

## 3. Design theory

### 3.1 Boxes

- **Hitbox:** the attack's contact area. "The active period of a move is defined to be when a hitbox is present", and its size sets the move's range ([Infil: Hitbox](https://glossary.infil.net/?t=Hitbox)).
- **Hurtbox:** where a character can be hit. Usually a separate throw hurtbox shows where throws can grab you ([Infil: Hurtbox](https://glossary.infil.net/?t=Hurtbox)). SF6 data lists both a **throw range** (attacker reach) and a **throw hurtbox** (defender size): Sagat 0.85 and 0.38 (FAT SF6).
- **Extended hurtboxes:** attacks put the attacker's hurtbox out with the limb, so a long poke can be whiff-punished by hitting the limb. Disjointed hitboxes break that rule ([Infil: Disjointed Hitbox](https://glossary.infil.net/?t=Disjointed%20Hitbox)). FAT's notes track this per move, for example SF6 Sagat 5MK "extends a wide leg hurtbox", SF6 Ryu Hadoken "no hurtbox extension at all", and SFV Sagat 5HP "no hurtbox extension during startup".
- **Pushbox:** the body-collision box that stops characters overlapping. It is not the hurtbox ([Infil: Pushbox](https://glossary.infil.net/?t=Pushbox)).
- **Combo-only hitboxes:** SF6 gives some moves hitboxes that only touch an opponent already in hitstun, to make combos consistent without making neutral hitboxes huge ([SF6 Game Data](https://wiki.supercombo.gg/w/Street_Fighter_6/Game_Data)).
- **Standing-to-crouching hurtbox:** in SF6 the crouch hurtbox fully shifts on the 5th frame, and proximity guard skips this ([SF6 Game Data](https://wiki.supercombo.gg/w/Street_Fighter_6/Game_Data)).

### 3.2 Startup, active, recovery, hitstun, blockstun, hitstop

- **Definitions:** [Infil: Startup](https://glossary.infil.net/?t=Startup), [Active](https://glossary.infil.net/?t=Active), [Recovery](https://glossary.infil.net/?t=Recovery), [Hit Stun](https://glossary.infil.net/?t=Hit%20Stun), [Block Stun](https://glossary.infil.net/?t=Block%20Stun), [Hitstop](https://glossary.infil.net/?t=Hitstop).
- **Hitstop** sits outside startup, active and recovery. It "helps to give players time to input a special move cancel and hitconfirm" ([SF6 Game Data](https://wiki.supercombo.gg/w/Street_Fighter_6/Game_Data)).
- **Hit and block advantage relationship:** in Alpha 2, hit advantage is block advantage + 1 for every move ([SFA2 Glossary](https://wiki.supercombo.gg/w/Street_Fighter_Alpha_2/Glossary)). In CvS2 the wiki lists identical hit and block advantage for normals ([CvS2 Eagle](https://wiki.supercombo.gg/w/Capcom_vs_SNK_2/Eagle)). Modern games separate the two widely.

**SF6 roster-wide medians, computed from FAT's SF6 JSON (32 characters).** Values are medians with the [min–max] range; startup is FAF.

| Normal | Startup | Active | Recovery | On block | Hitstun | Blockstun | Hitstop |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 5LP | 4 [4–7] | 3 | 8 | -1 [-3..+2] | 15 [13–18] | 9 [7–14] | 9 |
| 2LP | 4 [4–6] | 3 | 8.5 | -1 | 16 | 10 | 9 |
| 5LK | 5 [4–9] | 3 | 11 | -2 | 16 | 11.5 | 9 |
| 2LK | 5 [4–6] | 2.5 | 10 | -2 | 15 | 10 | 9 |
| 5MP | 7 [5–14] | 4 | 15 | -1 [-6..+2] | 22 [19–26] | 17.5 | 11 |
| 2MP | 6.5 [6–12] | 3 | 14 | -2 | 22 | 16.5 | 11 |
| 5MK | 8 [5–12] | 3 | 17.5 | -3 | 24 | 17 | 11 |
| 2MK | 8 [7–10] | 3 | 18 | -5 | 23 | 16.5 | 9 |
| 5HP | 10 [5–19] | 3 | 20 | -3 [-11..+1] | 26 [21–28] | 20.5 | 13 |
| 5HK | 12 [7–17] | 3.5 | 20 | -3.5 | 27 | 20 | 13 |
| 2HP | 9.5 [7–19] | 4 | 21 | -6 | 27 | 20 | 13 |
| 2HK (sweep) | 10 [8–14] | 3 | 24 | -11 [-16..-3] | knockdown | 18 | 13 |

**Same computation for older games** (startup medians, on-block medians):

| Game | Lights | Mediums | Heavies | Sweeps |
| --- | --- | --- | --- | --- |
| SFV (46 characters) | 4f startup, **+1 to +2** on block | 6–7.5f, +2 (punches) / -2 (kicks) | 9–12f, -3 to -5 | 9f, -12 |
| USF4 (45 characters, far normals) | 4f, 0 to +3 | 5–7f, -3 to 0 | 8–9f, -3 to -6 | 8f, -8 |

**Damage as a share of health** (FAT, medians):

| Game | Health | Light | Medium | Heavy |
| --- | --- | --- | --- | --- |
| SF6 | 10,000 [9,000–11,000] | 300 (3%) | 600 (6%) | 900 (9%) |
| SFV | 1,000 [900–1,075] | 30 | 60 | 90 |
| USF4 | 950 [850–1,100] | 30 | 70 | 100 |

Special-move references for scale: SF6 Hadoken is 700 (7%), Shoryuken 1,100–1,400. The repo's overhaul spec mentions an optional "unscaled 1987 42% special damage" ([overhaul spec](../planning/overhaul-spec.md)).

### 3.3 Frame advantage and pushback

- **Frame advantage** decides who acts first after contact ([Infil: Frame Advantage](https://glossary.infil.net/?t=Frame%20Advantage)). A move that is -N on block is punishable by any attack with startup of N or less (FAF counting, [SF6 Game Data](https://wiki.supercombo.gg/w/Street_Fighter_6/Game_Data)).
- **Pushback** limits how much frame advantage matters. A very minus move can still be safe if the attacker is pushed out of the defender's reach ([Infil: Pushback](https://glossary.infil.net/?t=Pushback)). FAT gives qualitative pushback notes (SF6 Sagat 5MP: "High pushback on block (outside of throw range unless used from Drive Rush)").
- **No numeric data found.** I found no published cross-game table of pushback distances. In 1987, the reaction stream itself carries a +0x20 pushback ([punch.md](../reference/punch.md)).

### 3.4 Throws: range, startup, tech windows

| Game | Throw startup / active / recovery | Range | Tech window | Source |
| --- | --- | --- | --- | --- |
| SF2 World Warrior | – | – | **No teching.** Some later versions let you "soften" a throw: about half damage and you land on your feet. | [Infil: Throw Tech](https://glossary.infil.net/?t=Throw%20Tech) |
| Super Turbo | startup 0, active 1 (Ryu's Seoi Nage) | **48 px from axis**, 16–19 px beyond the opponent's throwable box (Sagat 16, Ryu 19) | Softening; "After getting up from being knocked down, you are unthrowable for 13 frames." | [ST Ryu](https://wiki.supercombo.gg/w/Super_Street_Fighter_2_Turbo/Ryu), [ST Sagat](https://wiki.supercombo.gg/w/Super_Street_Fighter_2_Turbo/Sagat), [ST FAQ](https://wiki.supercombo.gg/w/Super_Street_Fighter_2_Turbo/FAQ) |
| CvS2 | punch throw 3f, kick throw 5f | – | **≈7f** for punch throws, **3f** for kick throws; opens after the grab connects | [CvS2 System](https://wiki.supercombo.gg/w/Capcom_vs_SNK_2/System) |
| SF3 3rd Strike | – | – | LP+LK "during the 5 frames on and after which it connects" | [3S System](https://wiki.supercombo.gg/w/Street_Fighter_3:_3rd_Strike/System) |
| USF4 | 3 / 2 / 20 (Ryu) | 0.79–1.07 (Ryu 0.923) | "Right when a Throw connects, you have **7 frames**" | FAT USF4; [SF4 Tech page](https://wiki.supercombo.gg/w/Street_Fighter_IV/Universal_Abilities/Tech_Hits_Counter_Throws) |
| SFV | 5 / 3 / 17 (Ryu) | 0.8–1.0 (Ryu 0.8) | Community: 7 frames after the first active frame | FAT SFV; [r/StreetFighter thread](https://www.reddit.com/r/StreetFighter/comments/4cf6wj/) (community, unverified) |
| SF6 | 5 / 3 / 23 (Ryu): 30 frames total | 0.8 for 25 of 32 characters (0.85–0.9 for six, incl. Sagat 0.85; Zangief 1.02) | The escape window "lasts until the **9th frame** of being thrown (including the frame that the Throw connects)" | FAT SF6; [SF6 Defense](https://wiki.supercombo.gg/w/Street_Fighter_6/Defense) |

**Related rules:**

- **Throw recovery in SF6:** EventHubs reports SF6 throws kept SFV's startup and active frames but gained 6 frames of recovery, which makes a whiffed throw costlier ([EventHubs, 2022-11-02](https://www.eventhubs.com/news/2022/nov/02/street-fighter-6-change-throw); search summary).
- **Strike vs throw on the same frame:** in SF6, "strikes actually have priority over throws when they collide on the same frame" ([SF6 Game Data](https://wiki.supercombo.gg/w/Street_Fighter_6/Game_Data)). In SFV, throws have priority over strikes ([SFV Game Data](https://wiki.supercombo.gg/w/Street_Fighter_V/Game_Data)).
- **Throws that can't be escaped:** in SF6, throws that land during the defender's recovery can't be escaped ([SF6 Defense](https://wiki.supercombo.gg/w/Street_Fighter_6/Defense)).
- **Throw-tech guessing:** delayed tech is the standard defensive option select, and shimmies (walking out of range) bait it ([Infil: Delayed Tech](https://glossary.infil.net/?t=Delayed%20Tech), [SF6 Defense](https://wiki.supercombo.gg/w/Street_Fighter_6/Defense)).
- **Screen-resolution note.** ST runs at the same 384×224 resolution as the 1987 game. ST's pixel throw ranges are therefore the most directly transferable throw numbers found.

### 3.5 Combo scaling

**SF4.** 100, 100, 80, 70, 60, 50, 40, 30, 20%, then each hit after the ninth drops another 10%. Separately, "guts" scaling reduces damage taken at low health: 95% at 50% health, 90% at 25%, 75% at 15% ([EventHubs SF4 guide, 2009](https://eventhubs.com/guides/2009/may/09/how-combo-and-damage-scaling-works-street-fighter-iv)). That guide cites contributors, not an official source. The fan wiki instead floors scaling at 10%.

**SF6** ([SF6 Game Data](https://wiki.supercombo.gg/w/Street_Fighter_6/Game_Data)):

| Attack # | General | Light normal or 2MK starter |
| --- | --- | --- |
| 1 | 100% | 100% |
| 2 | 100% | 80% |
| 3 | 80% | 70% |
| 4 | 70% | 60% |
| 5 | 60% | 50% |
| 6 | 50% | 40% |
| 7 | 40% | 30% |
| 8 | 30% | 20% |
| 9 | 20% | 10% |
| 10+ | 10% | 10% |

Other SF6 scaling rules:

- An "attack" is each player input, not each hit.
- Super Arts never fall below 30% / 40% / 50% (Level 1 / 2 / 3).
- A throw comboed into takes 20% immediate scaling.
- Drive Rush adds 15% to the remaining hits; a Perfect Parry punish is scaled ×0.5.
- Per move, FAT records SF6's starter penalties: lights "20% Start", Shoryuken "30% Start", throws "20% Immediate".

Damage scaling and per-move proration are defined at [Infil: Damage Scaling](https://glossary.infil.net/?t=Damage%20Scaling) and [Proration](https://glossary.infil.net/?t=Proration).

### 3.6 Cancels, links, buffers, command windows

- **Definitions:** cancel ([Infil: Cancel](https://glossary.infil.net/?t=Cancel)), link ([Link](https://glossary.infil.net/?t=Link)), chain ([Chain](https://glossary.infil.net/?t=Chain)), kara cancel ([Kara Cancel](https://glossary.infil.net/?t=Kara%20Cancel)). Kara cancels are usually an accident of lenient cancel windows on the first frames of a move.
- **Cancel windows:** FAT lists SF6 cancel windows per move. Sagat 5LP's special/super cancel window is "13(*15)", where * is the Drive Rush cancel.
- **Which normals cancel is a balance lever.** In ST, Old Sagat could cancel standing short, standing forward and standing strong into specials, while New Sagat could cancel only the first hit of short and forward. HD Remix removed further cancels to stop "far standing short into more tiger shots" spam ([sirlin.net/sf-hdr/sagat](https://sirlin.net/sf-hdr/sagat)).
- **Input buffer (SF6):** any move can be buffered 4 frames early, a 5-frame window. Dashes and wakeup reversals get 7 frames, an 8-frame window ([SF6 Game Data](https://wiki.supercombo.gg/w/Street_Fighter_6/Game_Data)). Charge moves keep their charge for about 10–12 frames after release ("Charge Keep") on the same page.
- **Command windows:** ST allows 7–14 frames between motion steps and 7–10 frames from the last direction to the button ([ST Sagat](https://wiki.supercombo.gg/w/Super_Street_Fighter_2_Turbo/Sagat)). The 1987 game allows 18 frames from the first direction to the third ([specials.md](../reference/specials.md)).

### 3.7 How designers balance a roster

- **Viable options and fairness.**
  - Sirlin calls a dominant move the worst thing in a competitive game.
  - He prefers a compact roster where nearly all characters are viable: ST's 16, Guilty Gear's 23 and Yomi's 20 are his examples.
  - Moves and characters should "touch as many corners of the design space as possible", differing in kind, not just in damage or speed.
  - Matchups may be locally unfair as long as the game is globally fair.
  - ([Sirlin, "Balancing Multiplayer Games, Part 2: Viable Options", 2014](https://www.sirlin.net/articles/balancing-multiplayer-games-part-2-viable-options))
- **Process.** You can't solve a complex game, so you balance by expert intuition, playtest with experts, and repeat until ship ([Part 4: Intuition](https://www.sirlin.net/articles/balancing-multiplayer-games-part-4-intuition)).
- **A worked rebalance (HD Remix).**
  - The goal was to "reduce the number of really bad match ups".
  - "We're trying not to nerf the power level of anything unless it's absolutely necessary."
  - Weak characters (Honda, Fei Long) got many upgrades, and any power cut was paired with a boost elsewhere.
  - ([sirlin.net/sf-hdr](https://sirlin.net/sf-hdr))
- **Capcom on SF6.** Director Takayuki Nakayama said in a 2024 Famitsu interview that character usage "doesn't factor in as much as you'd think" when changes are made (reported by [EventHubs](https://eventhubs.com/news/2024/jun/10/sf6-usage-character-balance); search summary, interview not read directly).
- **Talks.** No GDC talk I found publishes fighting-game balance numbers. GDC 2019's "'09 to '19: A Decade of Approachability in Fighting Games" (Noah Sasso, Iron Galaxy) covers approachability, not tuning ([GDC Vault](https://gdcvault.com/play/1025792/-09-to-19-A)).
- **Levers the data shows designers actually use:**
  - **Health:** SF6 spans 9,000–11,000 and USF4 850–1,100. Gen has 900 in USF4; Adon 950.
  - **Damage:** per strength, as in the table in 3.2.
  - **Recovery:** this sets on-block advantage.
  - **Range:** FAT's SF6 `range` field.
  - **Walk and dash speed.**
  - **Invulnerability frames:** for example, the anti-air invulnerability on Sagat's 5HK.
  - **Cancel permissions:** which normals can cancel, as in 3.6.

---

## 4. Fair, human-like CPU AI

### 4.1 Human reaction time, in frames at 60 fps

| Measure | Value | Frames | Source |
| --- | --- | --- | --- |
| Simple visual reaction time (N = 1,469) | 230.8 ms mean; about 213 ms after removing measured display and mouse delay (17.8 ms). Ages 18–24: 217.9 ms | 13.8 raw, about 12.8 corrected | [Woods et al. 2015, Frontiers in Human Neuroscience](https://www.frontiersin.org/journals/human-neuroscience/articles/10.3389/fnhum.2015.00131/full) |
| Single, expected, easy-to-identify stimulus in a fighting game | "around 265 milliseconds" | about 16 | [Infil, "On Reaction Times" (KI)](https://ki.infil.net/reaction.html) |
| Typical human response quoted for KI's Shadow AI | – | "12–19 frames" | [Game Developer, "The Killer Groove", 2017](https://www.gamedeveloper.com/programming/the-killer-groove-the-shadow-ai-of-killer-instinct) |
| Practical "reactable" bar | "if you can reliably react to moves that have around 20 frames of startup, you're doing a super great job" | 20 | [Infil: Reactable](https://glossary.infil.net/?t=Reactable) |

What Infil's article adds:

- **Reactions are slower in practice than the measured figure.** Crouch-to-stand block changes and jumps are the fastest responses, a button press is a little slower, and motion-input specials like a DP are the slowest.
- **Multiple threats are much harder.** Glacius's 27f Shatter is easy to jump when it's his main threat but hard during close-range mixups.
- **A 19f overhead with a clear tell is reactable but close to the limit.** A 15f one is mostly not.
- **Jumps (about 45f) are reactable even for beginners.**

Infil's glossary adds that even very good players "will often get hit by moves that have 20 frames of startup or more" ([Infil: Reaction](https://glossary.infil.net/?t=Reaction)).

### 4.2 Not reading inputs

- **The 1987 CPU reads inputs.** Its follow-up decision branches on "the player is attacking (+9 bit 6)", read on the first frame ([cpu-follow-up.md](../reference/cpu-follow-up.md), [overhaul design](../design/2026-10-27-definitive-overhaul-design.md)). The repo's planned `fair_cpu_and_sagat_scaling` toggle adds a configurable reaction delay there ([overhaul spec](../planning/overhaul-spec.md)).
- **FightingICE, the academic fighting-game AI platform, delays what bots see.** Its rules state "the information sent to AIs will have a delay of 15 frames" at 60 fps ([FTGAIC rules](https://ice.ci.ritsumei.ac.jp/~ftgaic/index-3.html)). The competition's front page gives the reason: "Such delay is introduced because we want to add more risks to the defense play style and simulate the reaction delay of human players" ([FTGAIC](https://ice.ci.ritsumei.ac.jp/~ftgaic/index-1.html)). The current rules page adds that the delay also pushes entrants toward vision-based AIs, which read the screen and avoid it.
- **The SF6 CPU goes through the same input pipeline as a player.** It enters commands internally, like a player, and an internal "pressure" coefficient makes command inputs fail more often at lower levels. The example given: Zangief, pressured, enters the Screw Piledriver motion with a late punch and jumps instead ([Famitsu CEDEC 2024 report](https://www.famitsu.com/article/202408/15462)). The headline of [Automaton's English summary](https://automaton-media.com/en/news/street-fighter-6s-cpu-is-designed-to-feel-pressured-just-like-a-human-and-make-input-mistakes/) says the same (I could read only the headline).

### 4.3 Difficulty scaling and decision structures (documented examples)

- **SF6 (Capcom, CEDEC 2024, planner Naohiro Yasuhara).**
  - **Structure:** each action is a **card**, with startup frames printed as its "cost" and damage as its "firepower". Situational decks of cards are **tactics**, groups of tactics are **strategies**, and each strategy is assigned to a level. An **execution ability** check decides whether the chosen card is actually performed.
  - **Levels:** an 8-level × 8-category "draft" table (movement, strikes, guard, jumps, special moves and so on) sets behaviour per level. Famitsu reports Lv1 as almost only movement, Lv3 occasionally jumping, Lv6 boss-class with simple tactics, and Lv7 advanced but with unstable execution (judgment and input errors). GameBusiness reports Lv1 with no specials, Lv3 starting specials, Lv5 mixing in feints and Lv8 keeping optimal spacing. The two reports describe different rows of the table.
  - **What higher levels know:** low levels can't tell when they have advantage; higher levels can, and the top levels know the exact frame advantage.
  - **Inputs:** decisions read eight state categories: round wins, vitality, SA gauge, Drive gauge, position and distance, recovery-frame difference, posture, and special counters.
  - **No published reaction delays:** neither report gives per-level reaction delays in frames.
  - Sources: [Famitsu](https://www.famitsu.com/article/202408/15462), [GameBusiness.jp](https://www.gamebusiness.jp/article/2024/09/06/23466.html).
- **Killer Instinct Shadow AI (Iron Galaxy).**
  - **Method:** case-based reasoning over recorded player behaviour. About 400–700 patterns per match, up to 40 matches per opponent.
  - **Matching:** nearest-neighbour retrieval over 40+ metrics, many stored relative to the opponent (distance, health).
  - **Choice:** candidate actions are ranked by value. The top one is usually taken, but "with a small probability" a lower-ranked one is picked to stay unpredictable, and priorities shift when an action keeps getting punished.
  - **No reaction cap:** the write-up does not describe a reaction-time limit.
  - Sources: [Game Developer, 2017](https://www.gamedeveloper.com/programming/the-killer-groove-the-shadow-ai-of-killer-instinct); [GDC 2016 "Designing AI for Competitive Games", Hayles and Neal](https://gdcvault.com/play/1023291/Designing-AI-for-Competitive).
- **Tekken 8 Super Ghost.** An AI that learns a player's habits from their matches. Bandai Namco has published no technical detail that I found ([esports.gg](https://esports.gg/news/tekken-8/tekken-8-super-ghost-battle/); search summary).
- **Academic dynamic difficulty adjustment.**
  - Demediuk et al. (CIG 2017) adjust MCTS agents in real time, aiming for a **zero health difference** at the end of the fight, tested in FightingICE against bots and humans ([UTS record](https://opus.lib.uts.edu.au/handle/10453/127485)).
  - Ishihara et al. (CIG 2018) extend this with "believable behaviors" ([project page](https://www.ice.ci.ritsumei.ac.jp/~ruck/dda-cig2018.htm); I did not read the paper itself).
  - Nimpattanavong et al. (2023) use a delay mechanism to make agent evaluation fair regardless of processing time ([arXiv 2312.16010](https://arxiv.org/abs/2312.16010)).
- **Utility AI and behaviour trees.** I found **no published fighting-game CPU write-up** that names either. The documented production designs are SF6's card/deck state machine and KI's case-based retrieval with ranked choice. Both are utility-style: score the options, then pick.

---

## 5. Synthesis: numbers to apply at 60 fps

These are proposals derived from the sources above, labelled with where each number comes from. The **classic level** should keep every value the ROM gives. The proposals matter most for moves the bosses never had in 1987 and for the modern level.

### 5.1 Classic level (1987 rules)

- **Hit reaction:** keep the ROM's **26-frame, fully invulnerable** reaction with its +0x20 pushback ([punch.md](../reference/punch.md)). It is what makes combos impossible, so new boss normals need no hitstun tuning here, only startup, active, recovery, damage and boxes.
- **Inputs:** keep release-latched buttons, the 18-frame command window and the fireball slow motion as the ROM has them.
- **Boss move data:** use each boss's own 1987 data where it exists (the boss docs in section 0). For moves he never had, take startup and recovery *shapes* from the later games in section 1, scaled to the 1987 cast's slower pace. The SF1++ notes put Geki's fastest attack at frame 9 or 10, Ryu's medium kick at 21 frames and the jump squat at 9 frames; check all of these against the ROM.

### 5.2 Modern level (startup counted on the first active frame)

| Quantity | Proposal | Basis |
| --- | --- | --- |
| Light normal startup | 4–5f (jabs 3–4f in older eras) | SF6 median 4–5; SFV and USF4 4 |
| Medium normal startup | 6–8f | SF6 6.5–8; SFV 6–7.5 |
| Heavy normal startup | 9–12f; sweeps 8–10f | SF6 9.5–12; sweep 10 |
| Active frames | 2–4 | all three games |
| Hitstun (L / M / H) | 15–16 / 22–24 / 26–27 | SF6 medians |
| Blockstun (L / M / H) | 9–11 / 16–18 / 20 | SF6 medians |
| Hitstop (L / M / H) | 9 / 11 / 13 | SF6 medians |
| On block | lights -1 to -2 (SF6 style) or +1 to +2 (SFV style, more pressure); mediums -1 to -5; heavies -3 to -6; sweeps -11 to -12 | 3.2 |
| Projectiles | about 12–16f startup; slightly minus to slightly plus at point blank | Sagat and Ryu across SF6 / SFV / USF4 / Alpha 2 |
| DP-type reversals | 3–5f with invulnerability; -20 to -45 on block | Tiger Uppercut, Rising Jaguar, Gekiro across versions |
| Cancels | lights and most mediums cancel into specials; heavies selectively; per-normal cancel permissions are the main tool against spam | FAT cancel fields; Sirlin's HD Remix Sagat notes |
| Input buffer | 4f (5f window); 7f for reversals and dashes | SF6 Game Data |
| Throw | 5f startup, 3 active, about 23 recovery (30 total); range about **48 px from axis**, or 16–19 px beyond the defender's throw box | SF6 timing; ST pixel ranges (same 384×224 resolution) |
| Throw tech window | **7–8 frames after the throw connects** (accept LP+LK up to the 8th or 9th frame counting the connect frame) | SF4 7f; SF6 up to 9th frame incl. connect; CvS2 about 7f; 3S 5f is the tight end |
| Throw protection | throw-invulnerable for a short time after wakeup (ST: 13f); throws that land during the defender's recovery can't be teched; decide strike vs throw on the same frame deliberately (SF6: strike wins; SFV: throw wins) | ST FAQ, SF6/SFV pages |
| Combo scaling | 100, 100, 80, 70, 60, 50, 40, 30, 20, 10% (floor 10%); light starters 100, 80, 70…; throw comboed into: 20% immediate | SF6 Game Data |
| Damage | lights about 3%, mediums about 6%, heavies about 9% of health; specials about 6–16% | FAT medians; Sagat specials 6–16% in SF6 |

### 5.3 Balanced roster layer

**Levers**, in the order designers seem to reach for them:

1. Recovery (on-block advantage).
2. Damage.
3. Health, kept within about ±10% (SF6 9,000–11,000).
4. Cancel permissions.
5. Invulnerability frames on reversals.
6. Range and walk speed.

**Principles from Sirlin:**

- Buff the weak before nerfing the strong.
- Aim to remove "really bad match ups" rather than to make every matchup 5-5.
- Keep each fighter different in kind.

My suggestion, not sourced: use CPU-vs-CPU win rates only as a smoke test. Every source says expert human playtesting decides balance.

### 5.4 CPU

- **Perception:** the CPU sees only game state delayed by its reaction time, never input bits. This replaces the 1987 frame-1 read of +9 bit 6.
- **Reaction delay by difficulty** (my proposal, anchored on the sources). Never go below about 12–13 frames (the corrected human simple reaction time) at any level.

| Level | Delay | Anchor |
| --- | --- | --- |
| Easy | 24–30f | |
| Normal | 18–20f | Infil's "reactable" bar |
| Hard | 15–16f | FightingICE's 15f; Infil's 16f |
| Very Hard | 12–13f | Woods corrected simple reaction time |

- **Response cost:** add a few frames when the response is a motion special rather than a block or jump, and more when several threats are possible (Infil's ordering).
- **Execution:**
  - Commands are entered through the same input path as a player's.
  - Give a per-level failure probability that rises with "pressure" (SF6). A failed DP should come out as whatever the bad input produces, not as nothing.
  - Choose by scoring: rank options, usually take the best, sometimes take a lower-ranked one (KI), and down-weight options that keep getting punished.
- **What changes with level:**
  - Move variety: specials appear from mid levels in SF6.
  - Whether the CPU knows its frame advantage: only higher SF6 levels do.
  - Execution reliability.
  - Optionally, a DDA target of a near-zero health difference for casual modes (Demediuk 2017).

---

## Gaps and caveats

- **No official data for five fighters.** Retsu, Geki, Joe, Mike and Lee have no playable Capcom appearance and no frame data. Retsu is a non-playable SF6 World Tour opponent. The only numbers are SF1++ community notes on the 1987 ROM.
- **Missing pages and blocks:**
  - Eagle has data only in CvS2; SuperCombo's Alpha 3 Eagle page has none.
  - Gen's Alpha 3 special moves have no data.
  - CvS2 Sagat's Tiger Knee has no data block.
  - Birdie is not playable in SF6.
  - SF2 World Warrior, Champion Edition and Hyper Fighting numbers were not read, beyond HSF2 Low Tiger Shot recovery from the ST page.
- **SF6 numbers are not first-hand from Capcom.** They come from FAT's maintained data, because Capcom's official frame pages refused automated access.
- **Older tables need care.** Alpha 3 and ST numbers were read from wikitext tables with mixed counting conventions. Where a row was ambiguous (ST Tiger Knee's paired advantage values), I gave it as listed.
- **No numeric pushback values** were found for any game.
- **Weakly sourced numbers:** the SFV tech window comes from a community post. The SF6 throw-recovery change and the Retsu moveset detail come from search summaries of EventHubs pages I didn't open.
- **Unpublished AI internals.** Tekken 8 Super Ghost's internals are unpublished. SF6's per-level reaction delays are not in the CEDEC reports.
- **Unverified 1987 claims.** The SF1++ hack data (for example Ryu's Hadoken end lag of 7 frames on block versus Ken's 19) has not been checked against the repo's ROM translation.
