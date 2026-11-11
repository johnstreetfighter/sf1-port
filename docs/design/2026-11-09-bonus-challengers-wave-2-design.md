# Bonus Challengers — Wave 2 (*Hyper Street Fighter: Bloodlines*) on the Real Engine

- **Date:** 2026-11-09
- **Status:** Approved
- **Project:** Project 9 (`Bonus Challengers — Wave 2: Hyper Street Fighter: Bloodlines on the real engine`)
- **Context:** [`CONTEXT.md`](../../CONTEXT.md), [`2026-10-27-definitive-overhaul-design.md`](2026-10-27-definitive-overhaul-design.md), [`2026-11-02-art-pipeline-design.md`](2026-11-02-art-pipeline-design.md), [`2026-11-03-bonus-roster-by-lore-design.md`](2026-11-03-bonus-roster-by-lore-design.md), [`2026-11-03-stages-and-music-design.md`](2026-11-03-stages-and-music-design.md), [`2026-11-08-era-styles-and-gauges-design.md`](2026-11-08-era-styles-and-gauges-design.md), [`2026-11-08-endings-dramatic-battle-and-world-tour-design.md`](2026-11-08-endings-dramatic-battle-and-world-tour-design.md), [`2026-11-08-bonus-challengers-wave-1-design.md`](2026-11-08-bonus-challengers-wave-1-design.md), [`2026-11-09-training-drills-replay-takeover-and-control-schemes-design.md`](2026-11-09-training-drills-replay-takeover-and-control-schemes-design.md), [`ADR-0004`](../adr/0004-reach-fights-by-driving-the-faithful-flow.md), [`ADR-0006`](../adr/0006-rom-only-art-sources.md), [`ADR-0008`](../adr/0008-bonus-roster-chosen-by-lore.md), [`ADR-0009`](../adr/0009-stages-and-themes-in-extended-banks.md), [`ADR-0014`](../adr/0014-bosses-become-1987-player-fighters.md), [`ADR-0015`](../adr/0015-era-styles-and-sub-hp-on-the-1987-engine.md), [`ADR-0016`](../adr/0016-dramatic-battle-and-composite-endings-on-the-1987-engine.md), [`ADR-0017`](../adr/0017-forty-fighter-roster-and-satsui-awakening-on-the-1987-engine.md)

---

## 1. Intent

Once the planned Wave 1 work in [`docs/design/2026-11-08-bonus-challengers-wave-1-design.md`](2026-11-08-bonus-challengers-wave-1-design.md) (`Spec #292`, `BC1`–`BC10`) is implemented, it will widen `RosterFighter` (`0..40`), `DataBank` (`0x10_0000..=0x37_ffff`), and the 40-slot Character Select screen once for all 40 fighters and ship **Wave 1 (*Super Street Fighter: Rising*)**: **Evil Ryu** (`slot 12`, `0x1c_0000`) and **M. Bison** (`slot 13`, `0x1d_0000`), bringing the active roster to `14` (`7×2` select grid).

This design specifies **Wave 2 (*Hyper Street Fighter: Bloodlines*)**—the **10 challengers** (`slots 14..24`, `0x1e_0000..=0x27_ffff`) whose 1987 lore is directly tied to one of the 12 original tournament fighters (`ADR-0008`, [`2026-11-03-bonus-roster-by-lore-design.md` §4](2026-11-03-bonus-roster-by-lore-design.md)):
1. **Gou Hibiki** (`slot 14`, `0x1e_0000`) — Took Sagat's right eye in their 1977 duel; Sagat killed him (appears in 1987 as a spirit).
2. **Dan** (`slot 15`, `0x1f_0000`) — Gou's son; expelled by Gouken; sets out after Sagat in 1987.
3. **Gouken** (`slot 16`, `0x20_0000`) — Master of Ryu and Ken; friend of Retsu.
4. **Akuma** (`slot 17`, `0x21_0000`) — Gouken's brother; killed Goutetsu; rival of Gen; tied to Ryu's Satsui no Hado.
5. **Goutetsu** (`slot 18`, `0x22_0000`) — Master of Gouken and Akuma; aided by Retsu; killed by Akuma.
6. **Chun-Li** (`slot 19`, `0x23_0000`) — 19 years old in 1987; daughter of Dorai; trained by Gen.
7. **Guile** (`slot 20`, `0x24_0000`) — 26 years old in 1987; Ken's brother-in-law; fought Eagle in *Alpha 3*.
8. **Dhalsim** (`slot 21`, `0x25_0000`) — 34 years old in 1987; friend of Retsu; trained with Sagat.
9. **Oro** (`slot 22`, `0x26_0000`) — 140+ year-old Senjutsu hermit known to Retsu.
10. **Dorai** (`slot 23`, `0x27_0000`) — Chun-Li's father, Hong Kong ICPO inspector, and Gen's old friend who taught Gen the *Genden* kick (making his playable debut with a bespoke 1987 Northern Kung Fu & Bajiquan moveset).

Following the same **Code-First + Deterministic ROM Stand-In Catalogue** pattern as Wave 1 (`Spec #292`), all 10 fighters' `DataBank` movesets, airborne special dispatch (`ryu_jump`), native `SF1`/`SF1+` and `SF2`–`SF6` Era Styles, 6 Bloodlines stages and YM2151 FM `.score` themes (`0x43`–`0x48`), composite VS/ending screens, Secret Arcade Intrusions (including Akuma's *Shun Goku Satsu* boss ambush), 6 Dramatic Battle presets (`Presets 9–14`), and 5 World Tour Bloodlines nodes are built and verified in code immediately (`BW1`–`BW7`), while flipping `RELEASED_WAVES` from `1` to `2` (expanding the player-facing Select Screen from `7×2 = 14` to **`8×3 = 24`**) is gated on the PixelLab art tickets (`BW8`–`BW11`).

---

## 2. Goals, Non-Goals, Anti-Goals

### Goals
1. **Complete `DataBank` Movesets (`0x1e_0000..=0x27_ffff`), Airborne Specials, & Era Styles (`SF1`–`SF6`) for All 10 Wave 2 Challengers:**
   - Every Wave 2 challenger has **native `SF1` and `SF1+` styles** (never marked `[WHAT-IF]`), **Canon** styles for every entry they appeared in, and **`[WHAT-IF]`** flagship styles (`SF2 ST`, `SFZ3`, `SF3 3S`, `USF4`, `SF5 CE`, `SF6`) for eras they missed (including all 6 later eras for `Gou Hibiki`, `Goutetsu`, and `Dorai`), plus full entries in [`crates/sf-engine/src/moves.rs`](../../crates/sf-engine/src/moves.rs) (`moves::REGISTRY`), `front/moves.rs` (`ML1`–`ML6`), and `ComboTrialSpec` (`TRD4` `#316`).
   - Add airborne special recognition in `fighter.rs` (`ryu_jump`) for fighters/styles with air specials (Akuma's *Zanku Hadoken* `air 236+P`, Dhalsim's *Yoga Drill* `air 2+K` / *Yoga Mummy* `air 2+P` / air *Yoga Teleport*, and Oro's air *Jinchu Watari* `air 236+K`).
   - Synthesize deterministic ROM-backed stand-in sprite catalogues in `DataBank::build` when a challenger's PixelLab sprite pack is not yet present on disk so all 10 fighters run through the real 68000 `fighter.rs` stream/hitbox/projectile engine in tests immediately.
2. **Six 1987-Style Bloodlines Home Stages & Original YM2151 FM Themes (`0x43`–`0x48`):**
   - Register 6 Wave 2 stages (`wave = 2`) in `BackgroundBank` and 6 committed original 1987-style YM2151 `.score` compositions (`0x43`–`0x48`, with optional `sf1-port-music` pack override) in `art/themes/` and `crates/sf-host/src/themes.rs`:
     1. **Goutetsu's Mountain Dojo (`0x43`, `"Japan"`):** Listed under owner `"Gouken"`, shared by **Gouken**, **Goutetsu**, and **Akuma**.
     2. **Chao Phraya Twilight Shrine (`0x44`, `"Thailand"`):** Listed under owner `"Gou Hibiki"`, shared by **Gou Hibiki** and **Dan**.
     3. **Hong Kong Kowloon Courtyard (`0x45`, `"China"`):** Listed under owner `"Dorai"`, shared by **Dorai** and **Chun-Li**.
     4. **US Air Force Tarmac (`0x46`, `"USA"`):** Listed under owner `"Guile"`.
     5. **Ganges River Temple (`0x47`, `"India"`):** Listed under owner `"Dhalsim"`.
     6. **Deep Hermit Cavern (`0x48`, `"Brazil"`):** Listed under owner `"Oro"`.
3. **Wave 2 Composite VS Screens, Congratulatory Endings (`0x44b00`), & Post-Match Win Quotes (`0x4014c`):**
   - Extend `BackgroundBank::add_vs_screens` and `BackgroundBank::add_ending_screens` (`ED1` `#284`, `BC5` `#297`) across all 24 `Wave 0..=2` fighters (`24 × 24 = 576` VS pairs and `24` ending views), with character-specific `TextWriter` epilogues and post-match win quotes (`0x4014c`).
4. **Wave 2 Secret Arcade Intrusions Under the Exhibition Rule (`tasks.rs`):**
   - When `enable_bonus_challengers && enable_secret_arcade_intrusions && released_waves >= 2`:
     1. ***Akuma's Shun Goku Satsu Pre-Final Ambush*:** When playing as `Ryu`, `Ken`, `Gen`, `Gouken`, `Goutetsu`, or `Evil Ryu` with `continues_used == 0 && perfect_rounds >= 2 && special_or_super_kos >= 3`, **Akuma** ambushes before ladder step 9 (`Sagat`), flashes the screen black with a `15`-hit *Shun Goku Satsu* intro sequence, and fights a 2-out-of-3 Exhibition bout on **Goutetsu's Mountain Dojo** (`0x43`) before the player proceeds to the canonical Thailand final (`Sagat`).
     2. ***Master's Trial (`Gouken` / `Goutetsu`)*:** Playing as `Ryu`, `Ken`, or `Dan` (or `Gouken`, `Akuma`, `Retsu` for `Goutetsu`) with `continues_used == 0 && special_or_super_kos >= 3` triggers a mid-tournament Master Exhibition after Country 3 (`opponent_index == 5`).
     3. ***Bloodline Grudge (`Gou Hibiki` / `Dorai`)*:** Playing as `Sagat`, `Adon`, or `Dan` with `continues_used == 0 && perfect_rounds >= 1` triggers a spirit grudge exhibition against **Gou Hibiki** on *Chao Phraya Twilight Shrine* (`0x44`); playing as `Gen` or `Chun-Li` with `continues_used == 0 && perfect_rounds >= 1` triggers a sparring exhibition against **Dorai** on *Hong Kong Kowloon Courtyard* (`0x45`).
5. **Wave 2 in 2v1 Dramatic Battle (`Presets 9–14`) & 1987 World Tour (`Nodes #21–#25`):**
   - Add 6 Bloodlines Rivalry Presets to `DramaticSetupScreen` and 5 `[WAVE 2]` nodes (one per country region, including the 1977 `Gou Hibiki vs. Sagat` flashback duel) to `WorldTourScreen`.

### Non-Goals
- **Implementing Wave 3 challengers (`slots 24..40`):** Covered by Project 10 (`Wave 3: Ultra Street Fighter: Final Round`).

### Anti-Goals
- **Never expose half-finished Wave 2 fighters on default menus:** Keep `RELEASED_WAVES = 1` until all 4 Wave 2 PixelLab art tickets (`BW8`–`BW11`) are complete and flip `RELEASED_WAVES = 2`.
- **Never commit copyrighted Capcom melodies in `.score` files:** Per [`2026-11-03-stages-and-music-design.md` §2 Anti-Goal 4](2026-11-03-stages-and-music-design.md), committed `art/themes/*.score` files (`0x43`–`0x48`) must be original 1987-style YM2151 compositions; canonical SF2/Alpha arrangements live only in the optional local `sf1-port-music` pack.

---

## 3. Design

### 3.1 Pillar 1 — Wave 2 Fighters, `DataBank` Addresses (`0x1e_0000..=0x27_ffff`), Movesets & Era Styles

| Slot | `BonusChallenger` | `name()` | `short_name()` (`<= 8` chars) | 3-Char Initials | `DataBank` Address | Home Stage (`StageView`) | FM Theme | ROM Stand-In Base (Pre-Art Fallback) |
|---|---|---|---|---|---|---|---|---|
| `14` | **`GouHibiki`** | `"Gou Hibiki"` | `"G.HIBIKI"` | `"GOU"` | `0x1e_0000` | `"gou-hibiki"` (*Chao Phraya Twilight Shrine*) | `0x44` | Ryu/Ken gi + Tengu mask/haori palette tint |
| `15` | **`Dan`** | `"Dan"` | `"DAN"` | `"DAN"` | `0x1f_0000` | `"gou-hibiki"` (*Chao Phraya Twilight Shrine*) | `0x44` | Ken gi + vibrant pink gi palette tint |
| `16` | **`Gouken`** | `"Gouken"` | `"GOUKEN"` | `"GKN"` | `0x20_0000` | `"gouken"` (*Goutetsu's Mountain Dojo*) | `0x43` | Retsu/Ryu heavy gi + white-beard master palette |
| `17` | **`Akuma`** | `"Akuma"` | `"AKUMA"` | `"AKU"` | `0x21_0000` | `"gouken"` (*Goutetsu's Mountain Dojo*) | `0x43` | Ryu/Evil Ryu gi + charcoal/crimson hair palette |
| `18` | **`Goutetsu`** | `"Goutetsu"` | `"GOUTETSU"` | `"GTS"` | `0x22_0000` | `"gouken"` (*Goutetsu's Mountain Dojo*) | `0x43` | Retsu/Gen robes + indigo grandmaster palette |
| `19` | **`ChunLi`** | `"Chun-Li"` | `"CHUN-LI"` | `"CHU"` | `0x23_0000` | `"dorai"` (*Hong Kong Kowloon Courtyard*) | `0x45` | Lee/Geki agile martial-arts pose base + blue/gold qipao palette |
| `20` | **`Guile`** | `"Guile"` | `"GUILE"` | `"GUI"` | `0x24_0000` | `"guile"` (*US Air Force Tarmac*) | `0x46` | Joe/Eagle kick/strike pose base + olive fatigues palette |
| `21` | **`Dhalsim`** | `"Dhalsim"` | `"DHALSIM"` | `"DHA"` | `0x25_0000` | `"dhalsim"` (*Ganges River Temple*) | `0x47` | Retsu/Gen meditation/reach pose base + saffron/ochre palette |
| `22` | **`Oro`** | `"Oro"` | `"ORO"` | `"ORO"` | `0x26_0000` | `"oro"` (*Deep Hermit Cavern*) | `0x48` | Gen/Retsu hermit pose base + ochre/teal robe palette |
| `23` | **`Dorai`** | `"Dorai"` | `"DORAI"` | `"DOR"` | `0x27_0000` | `"dorai"` (*Hong Kong Kowloon Courtyard*) | `0x45` | Gen/Lee Kung Fu pose base + navy/gold inspector coat palette |

#### A. Airborne Special Dispatch (`fighter.rs` `ryu_jump`) & `BanFormat::allows_roster`
1. **Airborne Special Recognition in `fighter.rs` (`ryu_jump`):**
   - Today, the 1987 engine only calls `validate_specials` (`0x16ca2`) from grounded states (`ryu_ground`).
   - Add an airborne special check inside `ryu_jump` (before the standard air normal check) for fighters/styles whose `DataBank` table flags one or more extended special slots (`0x21..=0x23`) with `SPECIAL_AIRBORNE_ONLY`:
     - **Akuma:** Airborne `236 + P` (`slot 0x23`) fires ***Zanku Hadoken*** (diagonal downward air fireball) during any forward/neutral jump.
     - **Dhalsim:** Airborne `2 + K` (`slot 0x21`) performs ***Yoga Drill*** (steep diving kick), Airborne `2 + P` (`slot 0x22`) performs ***Yoga Mummy*** (diving spinning headbutt), and Airborne `623/421 + P/K` (`slots 0x1f/0x20`) performs airborne ***Yoga Teleport***.
     - **Oro:** Airborne `236 + K` (`slot 0x1b`) performs airborne ***Jinchu Watari*** (double-stomp arc).
   - When an airborne special touches `GROUND_Y`, it transitions cleanly into landing recovery (`ryu_land`).
2. **`BanFormat::allows_roster` Rules (`roster.rs`, per `BC1` `#293`):**
   - `BanFormat::Unrestricted` allows all 10 Wave 2 challengers when `enable_bonus_challengers` is `true`.
   - `BanFormat::LowTierNoShoto` bans the Ansatsuken/Saikyo shoto lineage (`GouHibiki`, `Dan`, `Gouken`, `Akuma`, `Goutetsu`) alongside `Ryu`, `Ken`, `Sagat`, and `EvilRyu`.
   - `BanFormat::StandardTournament` bans boss-tier `Akuma` and `Goutetsu` (alongside `Sagat`, `EvilRyu`, `Bison`) when `balanced` is `false` and allows all 10 when `balanced` is `true`.

#### B. Ansatsuken & Saikyo Bloodline (`Gou Hibiki`, `Dan`, `Gouken`, `Akuma`, `Goutetsu`)
1. **Challenger #15 (`slot 14`, `0x1e_0000`) — Gou Hibiki (*Vengeful Spirit of Saikyo*):**
   - **Archetype:** Unorthodox counter-and-burst martial artist (`960` Sub-HP) blending early Ansatsuken with theatrical Saikyo feints—deadly enough to take Sagat's right eye in 1977.
   - **Specials:**
     - `236 + P` (`0x1a`): ***Gadoken*** (one-handed medium-range ki blast with larger hitbox and faster startup than Dan's).
     - `623 + P` (`0x19`): ***Koryuken*** (rising uppercut with invulnerable startup).
     - `214 + K` (`0x1b`): ***Tatsumaki Dankukyaku*** (advancing triple flying kick).
     - `214 + P` (`0x1f`): ***Tengu Counter (Eye-Taker Parry)*** (high/mid counter stance that retaliates with the blinding palm-claw strike that took Sagat's eye).
   - **Era Styles:** Native `SF1`, `SF1+`; What-If (`enable_what_if_styles`) in `SF2 ST`, `SFZ3`, `SF3 3S`, `USF4`, `SF5 CE`, `SF6` (Supers: *Haoh Gadoken*, *Koryu Rekka*, *Tengu Ranshin*).
2. **Challenger #16 (`slot 15`, `0x1f_0000`) — Dan (*Scion of Saikyo-Ryu*):**
   - **Archetype:** Short-range pressure brawler (`960` Sub-HP) whose taunts build extra Era meter and cancel into specials/Supers.
   - **Specials:**
     - `236 + P` (`0x1a`): ***Gadoken*** (short-range one-handed energy burst, safe on block at tip range).
     - `623 + P` (`0x19`): ***Koryuken*** (rising punch; Heavy version has startup invincibility flash).
     - `214 + K` (`0x1b`): ***Dankukyaku*** (1-hit `LK`, 2-hit `MK`, 3-hit `HK` flying knee/kick).
     - `236 + K` (`0x1f`) / `623 + K` (`0x20`): ***Saikyo Roll & Crouch Taunt*** (builds `+8` Era meter units on completion).
   - **Era Styles:**
     - **Native:** `SF1`, `SF1+`.
     - **Canon:** `SFZ` (`Alpha 1`, `Alpha 2`, `Alpha 3`: *Hissho Buraiken*, *Haoh Gadoken*, *Shinkuu Gadoken*, *Chouhatsu Densetsu* Legendary Taunt), `SF4` (`SSF4`–`USF4`: `Ultra I: Shisso Buraiken`, `Ultra II: Haoh Gadoken`), `SF5` (`CE`: *Saikyo-ryu Ougi* / *V-Skill* taunt cancels).
     - **What-If:** `SF2 ST`, `SF3 3S`, `SF6`.
3. **Challenger #17 (`slot 16`, `0x20_0000`) — Gouken (*Grandmaster of the Fist of Nothingness*):**
   - **Archetype:** Powerful zoning and counter master (`1000` Sub-HP) who turned Ansatsuken into a non-lethal art.
   - **Specials:**
     - `236 + P` (`0x1a`): ***Gohadoken*** (`LP` straight, `MP` 30-degree diagonal upward anti-air fireball, `HP` fast two-hit surge).
     - `623 + P` (`0x19`): ***Senkugoshoha*** (sliding palm dash strike; rising ***Forbidden Shoryuken*** in `SF1+`/`Super`).
     - `214 + K` (`0x1b`): ***Tatsumaki Gorasen*** (vertical rising hurricane kick).
     - `623 + K` (`0x1f`): ***Hyakkishu (Demon Flip)*** (follow-up dive kick `K`, sweep `P`, or air grab `LP+LK`).
     - `421 + P` (`0x20`): ***Kongoshin*** (one-arm palm/parry counter).
   - **Era Styles:**
     - **Native:** `SF1`, `SF1+`.
     - **Canon:** `SF4` (`SF4`–`USF4`: `Ultra I: Shin Shoryuken`, `Ultra II: Denjin Hadoken`, back-throw into Ultra confirm).
     - **What-If:** `SF2 ST`, `SFZ3`, `SF3 3S` (Denjin mastery), `SF5 CE`, `SF6`.
4. **Challenger #18 (`slot 17`, `0x21_0000`) — Akuma (*Master of the Fist / Satsui no Hado*):**
   - **Archetype:** Supreme glass-cannon vortex monster (`816` max Sub-HP = `85%` of `960` in `DefinitiveOverhaul` / `Balanced`, or `1.18×` incoming damage in `SF1`) with air fireballs, demon flip, teleport, and the *Shun Goku Satsu*.
   - **Specials:**
     - `236 + P` (`0x1a` ground / `0x23` airborne): ***Gohadoken*** (ground) & ***Zanku Hadoken*** (airborne `236 + P` during any jump).
     - `623 + P` (`0x19`): ***Goshoryuken*** (3-hit rising dragon uppercut).
     - `214 + K` (`0x1b`): ***Tatsumaki Zankukyaku*** (juggles into *Goshoryuken*).
     - `63214 + P` (`0x1f`): ***Shakunetsu Hadoken*** (1–3 hit red burning fireball).
     - `623 + K` (`0x20`) / `421 + K` (`0x21`): ***Ashura Senku*** (forward/backward invulnerable warp).
     - `236 + K` (`0x22`): ***Hyakkishu (Demon Flip)***.
   - **Era Styles:**
     - **Native:** `SF1`, `SF1+`.
     - **Canon:** `SF2 ST` (secret boss kit + *Messatsu Gou Hadou*), `SFZ` (`Alpha 1`–`Alpha 3`: *Messatsu Gou Hadou*, *Messatsu-Goshoryu*, *Tenma Gou Zanku*, Lv3 *Shun Goku Satsu*), `SF3` (`2nd Impact`, `3rd Strike`: `SA I`–`SA III` + *Kongou Kokuretsu Zan*), `SF4` (`SF4`–`USF4`: `Ultra I: Wrath of the Raging Demon`, `Ultra II: Demon Armageddon`), `SF5` (`AE`, `CE`: *Rakan*, *Dohatsu Shoten*), `SF6` (`Year 1`: *Adamant Flame* `214+P`, *Emptiness*, `SA3: Shun Goku Satsu`).
5. **Challenger #19 (`slot 18`, `0x22_0000`) — Goutetsu (*Founder of the Ansatsuken Dojo*):**
   - **Archetype:** Disciplined, heavy-hitting grandmaster (`1000` Sub-HP) wielding the undivided original Ansatsuken before Gouken and Akuma split its light and dark halves.
   - **Specials:**
     - `236 + P` (`0x1a`): ***Ansatsuken Hadou*** (dense indigo/crimson shockwave projectile).
     - `623 + P` (`0x19`): ***Soryuken (Twin Dragon Fist)*** (two-fisted rising uppercut).
     - `214 + K` (`0x1b`): ***Rasen Senpukyaku*** (low-trajectory advancing whirlwind kick).
     - `214 + P` (`0x1f`): ***Kiai Counter Throw*** (counters strikes with an armored neck-chop takedown).
     - `623 + K` (`0x20`): ***Shunpo (Founder's Step)*** (short invulnerable forward warp).
   - **Era Styles:** Native `SF1`, `SF1+`; What-If (`enable_what_if_styles`) in `SF2 ST`, `SFZ3`, `SF3 3S`, `USF4`, `SF5 CE`, `SF6` (Supers: *Shin Ansatsuken Ougi*, *Tenchi Soryuken*).

#### C. World Warriors, Masters & Legends (`Chun-Li`, `Guile`, `Dhalsim`, `Oro`, `Dorai`)
6. **Challenger #20 (`slot 19`, `0x23_0000`) — Chun-Li (*19-Year-Old ICPO Prodigy in 1987*):**
   - **Archetype:** Fast walk speed, wall jump, pokes, and multi-hit kick specials (`912` Sub-HP = `95%` of `960`), trained by her father Dorai and his friend Gen.
   - **Specials:**
     - `236 + K` (`0x1b`): ***Hyakuretsukyaku (Lightning Legs)*** (rapid multi-hit kick flurry, accessible via motion `236+K` or rapid `K` taps).
     - `[4]6 + P` or `63214 + P` (`0x1a`): ***Kikoken*** (travelling chi projectile).
     - `[2]8 + K` (`0x19`): ***Spinning Bird Kick*** (horizontal inverted helicopter kick).
     - `623 + K` (`0x1f`): ***Tenshokyaku*** (vertical rising kicks).
     - `214 + K` (`0x20`): ***Hazanshu*** (flipping overhead axe kick, borrowed from Gen/Dorai's Kung Fu).
   - **Era Styles:**
     - **Native:** `SF1`, `SF1+`.
     - **Canon:** `SF2` (`WW`–`ST`: *Senretsukyaku* Super), `SFZ` (`Alpha 1`–`Alpha 3` in her iconic 1987/Alpha acrobatic track outfit: *Kikosho*, *Senretsukyaku*, *Hazanshu*, *Sora Kyaku*), `SF3` (`2I`, `3S`: `SA I: Kikosho`, `SA II: Houyoku-sen`, `SA III: Tensei Ranka`), `SF4` (`SF4`–`USF4`), `SF5` (`SF5`–`CE`), `SF6` (`Serenity Stream` stance `214+P`).
7. **Challenger #21 (`slot 20`, `0x24_0000`) — Guile (*USAF Captain & Ken's Brother-in-Law*):**
   - **Archetype:** Premier charge zoner (`960` Sub-HP) with ultra-fast recovery *Sonic Boom*, invulnerable *Somersault Kick (Flash Kick)*, and command normals (*Spinning Back Knuckle* `6+HP`, *Knee Bazooka* `4/6+LK`, *Guile High Kick* `3+HK`).
   - **Specials:**
     - `[4]6 + P` (`0x1a`): ***Sonic Boom*** (low-recovery crescent shockwave projectile).
     - `[2]8 + K` (`0x19`): ***Somersault Kick (Flash Kick)*** (invulnerable rising anti-air backflip kick).
     - `214 + P` (`0x1b`): ***Sonic Blade*** (stationary spinning sonic orb that empowers `[4]6+P` into a 2-hit *Sonic Cross* in `SF1+` and `SF6`).
   - **Era Styles:**
     - **Native:** `SF1`, `SF1+`.
     - **Canon:** `SF2` (`WW`–`ST`: `[4]646+K` *Double Somersault*), `SFZ` (`Alpha 3`: *Sonic Hurricane* + *Somersault Strike*), `SF4` (`SF4`–`USF4`: *Somersault Justice* / *Sonic Hurricane*), `SF5` (`SF5`–`CE`: *Sonic Blade* / *Solid Puncher*), `SF6` (*Sonic Blade* / *Crossfire Somersault*).
     - **What-If:** `SF3 3S` (Parries + `SA I: Double Somersault`, `SA II: Sonic Hurricane`, `SA III: Sonic Tempest`).
8. **Challenger #22 (`slot 21`, `0x25_0000`) — Dhalsim (*Ascetic Master of Yoga*):**
   - **Archetype:** Long-limb zoner (`912` Sub-HP) whose normal punches and kicks extend across half the screen (`Far` normals stretch, `Close`/`Back+Normal` perform short rapid strikes), paired with *Yoga Fire*, *Yoga Flame*, air *Yoga Drill / Mummy*, and *Yoga Teleport*.
   - **Specials:**
     - `236 + P` (`0x1a`): ***Yoga Fire*** (travelling flame projectile; arcs upward in `SF5`/`SF6`).
     - `63214 + P` (`0x19`): ***Yoga Flame*** (close-range sustained flame sphere) & `63214 + K` (`0x1b`) ***Yoga Blast*** (upward anti-air flame).
     - `623 + P/K` (`0x1f`) / `421 + P/K` (`0x20`): ***Yoga Teleport*** (ground and airborne warp behind or in front of opponent).
     - Airborne `2 + K` (`0x21`): ***Yoga Drill*** (steep diving kick) & Airborne `2 + P` (`0x22`) ***Yoga Mummy*** (diving spinning headbutt).
   - **Era Styles:**
     - **Native:** `SF1`, `SF1+`.
     - **Canon:** `SF2` (`WW`–`ST`: *Yoga Inferno* Super), `SFZ` (`Alpha 2`, `Alpha 3`), `SF4` (`SF4`–`USF4`: *Yoga Catastrophe* / *Yoga Shangri-La*), `SF5` (`SF5`–`CE`: *Yoga Sunburst*), `SF6` (*Yoga Arch* / *Merciless Yoga*).
     - **What-If:** `SF3 3S`.
9. **Challenger #23 (`slot 22`, `0x26_0000`) — Oro (*140-Year-Old Senjutsu Hermit*):**
   - **Archetype:** Eccentric, double-jumping hermit (`960` Sub-HP) fighting with one arm bound inside his gi (unbinding both arms during Supers/V-Trigger/Level 3).
   - **Specials:**
     - `[4]6 + P` or `236 + P` (`0x1a`): ***Nichirin Shou*** (floating sun-disc energy orb).
     - `[2]8 + P` or `623 + P` (`0x19`): ***Oni Yanma*** (spinning vertical uppercut).
     - `236 + K` (`0x1b`, ground & airborne): ***Jinchu Watari*** (leaping double-stomp arc).
     - `63214 + P` (`0x1f`): ***Niou Riki*** (command grab slam).
   - **Era Styles:**
     - **Native:** `SF1`, `SF1+`.
     - **Canon:** `SF3` (`New Generation`, `2nd Impact`, `3rd Strike`: `SA I: Kishin Riki`, `SA II: Yagyou Dama`, `SA III: Tengu Stone`), `SF5` (`CE`: *Onibi*, *Tsuranekeashi*, *Tengu Stone* `V-Trigger II`).
     - **What-If:** `SF2 ST`, `SFZ3`, `USF4`, `SF6`.
10. **Challenger #24 (`slot 23`, `0x27_0000`) — Dorai (*Hong Kong Inspector & Master of the Genden Kick*):**
    - **1987 Lore & Bespoke Design ([`2026-11-03-bonus-roster-by-lore-design.md` §4](2026-11-03-bonus-roster-by-lore-design.md)):** Chun-Li's father and Gen's old comrade, from whom Gen learned the *Genden* kick before investigating Shadaloo in Hong Kong. Designed as a powerful, upright **Northern Shaolin Chaquan & Bajiquan** martial artist whose moveset visibly bridges Gen's lethal precision and Chun-Li's acrobatic kicks:
    - **Normals & Command Normals:** Crisp Northern Kung Fu palm strikes, sweeping long-range kicks, **`6+MK` *Genden Step Kick*** (the advancing mid/overhead kick he taught Gen), and **`6+HP` *Bajiquan Elbow* (lunging guard-crush elbow)**.
    - **Special Moves (`DataBank` `0x27_0000`):**
      1. **`236 + K` (`0x1b`) — *Genden Kick (Original Dragon Arc)*:** Lunging two-part crescent-to-axe kick (`LK` fast low-recovery poke, `MK` mid-screen advancing strike, `HK` leaping overhead knockdown that Gen later adapted into his Crane/Mantis arsenal).
      2. **`236 + P` (`0x1a`) — *Tie Shan Kao (Iron Mountain Shoulder & Chi Pulse)*:** Advancing Bajiquan shoulder-and-palm shockwave (`SPECIAL_MELEE` with a short-range golden chi burst that destroys single-hit projectiles).
      3. **`623 + K` (`0x19`) — *Rising Phoenix Kick*:** Disciplined vertical spinning anti-air kick—the direct martial precursor to Chun-Li's *Tenshokyaku*.
      4. **`214 + K` (`0x1f`) — *Hundred Dragon Strikes*:** Advancing 4-hit combination of three lightning kicks into a finishing palm thrust (the fatherly precursor to *Hyakuretsukyaku*).
      5. **`214 + P` (`0x20`) — *Inspector's Reversal (Chin Na Arrest Counter)*:** Parries high/mid strikes and throws the attacker across the stage.
    - **Era Styles:** Native `SF1`, `SF1+`; What-If (`enable_what_if_styles`) in `SF2 ST` (*Super: Grand Genden Combination*), `SFZ3`, `SF3 3S` (`SA I: Genden Reppu`, `SA II: Kowloon Phoenix`, `SA III: Dragon of Hong Kong`), `USF4`, `SF5 CE`, `SF6`.

---

### 3.2 Pillar 2 — Six Wave 2 Bloodlines Stages, Shared-Owner Resolution & Original YM2151 FM Themes (`0x43`–`0x48`)

Per [`docs/design/2026-11-03-stages-and-music-design.md`](2026-11-03-stages-and-music-design.md), Wave 2 adds **6 home stages** (`wave = 2`) in `BackgroundBank` and **6 committed original 1987-style YM2151 `.score` compositions** (`0x43`–`0x48`) in `art/themes/` and `crates/sf-host/src/themes.rs`:

| Stage ID | Stage Title & Setting | Primary Listed `owner` (`list_wave_stage`) | Shared Fighters (`StageScreen::home_owner`) | Country / `place` | FM Command & `.score` File | Pre-Art Stand-In `StageView` Assembly |
|---|---|---|---|---|---|---|
| `"gouken"` | **Goutetsu's Mountain Dojo** (Bamboo cliff shrine & waterfall at dusk) | `"Gouken"` | `Gouken`, `Goutetsu`, `Akuma` -> `"Gouken"` | `"Japan"` | `0x43` (`art/themes/gouken.score`) | Re-paletted Country 1 (Japan Retsu temple/courtyard) in warm twilight gold & pine green |
| `"gou-hibiki"` | **Chao Phraya Twilight Shrine** (Riverside Muay Thai / Saikyo pier with lanterns) | `"G.Hibiki"` | `GouHibiki`, `Dan` -> `"G.Hibiki"` | `"Thailand"` | `0x44` (`art/themes/gou-hibiki.score`) | Re-paletted Country 5 (Thailand Adon riverside temple) in sunset violet & lantern amber |
| `"dorai"` | **Hong Kong Kowloon Courtyard** (Neon-lit 1987 Victoria Harbour terrace & stone dragon gate) | `"Dorai"` | `Dorai`, `ChunLi` -> `"Dorai"` | `"China"` | `0x45` (`art/themes/dorai.score`) | Re-paletted Country 3 (China Lee/Gen courtyard) in harbour night blue, jade & crimson |
| `"guile"` | **US Air Force Tarmac** (1987 fighter-jet hangar & chain-link runway at dusk) | `"Guile"` | `Guile` -> `"Guile"` | `"USA"` | `0x46` (`art/themes/guile.score`) | Re-paletted Country 2 (USA Joe/Mike railyard/dock) in steel grey, olive & sunset orange |
| `"dhalsim"` | **Ganges River Temple** (Carved sandstone elephants, ghats & burning braziers) | `"Dhalsim"` | `Dhalsim` -> `"Dhalsim"` | `"India"` | `0x47` (`art/themes/dhalsim.score`) | Re-paletted Country 5/1 temple tiles in warm sandstone ochre, saffron & brass |
| `"oro"` | **Deep Hermit Cavern** (Subterranean stalactite cavern with glowing Senjutsu talismans) | `"Oro"` | `Oro` -> `"Oro"` | `"Brazil"` | `0x48` (`art/themes/oro.score`) | Re-paletted Country 3/4 stone cavern tiles in deep emerald, moss & talisman cyan |

1. **`StageScreen::home_owner` & `JukeboxScreen` Fighter Rows (`front/stage_select.rs` & `front/jukebox.rs`):**
   - `BackgroundBank::list_wave_stage` lists each of the 6 Wave 2 stages **once** under its primary `owner` (`"Gouken"`, `"G.Hibiki"`, `"Dorai"`, `"Guile"`, `"Dhalsim"`, `"Oro"`, each `<= 8` chars so it fits inside `TILE_NAME_CELLS = 9` without truncation), giving `10 + 2 + 6 = 18` stage tiles + `RANDOM` (`19` tiles total across 3 rows on `StageScreen`).
   - `StageScreen::home_owner(fighter: RosterFighter)` aliases fighters who share a Wave 2 stage to that stage's primary `owner` (just as `Ryu -> "Retsu"` and `Ken -> "Joe"` in 1987):
     - `Akuma | Goutetsu | Gouken => "Gouken"`
     - `Dan | GouHibiki => "G.Hibiki"`
     - `ChunLi | Dorai => "Dorai"`
   - `JukeboxScreen` (`crates/sf-engine/src/front/jukebox.rs`) populates its `FIGHTERS` section from `RosterFighter::released(released_waves, bonus_challengers)` rather than only `shown_stages` owners, so all 24 released fighters (including `Dan`, `Akuma`, `Goutetsu`, and `Chun-Li`) have their own per-fighter Jukebox track row.

---

### 3.3 Pillar 3 — Wave 2 Secret Arcade Intrusions, Priority Rules, Endings (`0x44b00`), & Win Quotes (`0x4014c`)

All Wave 2 Secret Arcade Intrusions are gated on **`!config.is_faithful_1987() && config.enable_bonus_challengers && config.enable_secret_arcade_intrusions && released_waves >= 2`** and obey the **Bonus Rival Exhibition Rule** (losing costs `0` credits and never halts the 1P Arcade run):

1. **Mid-Tournament Bloodlines Intrusions (After Country 3 / Ladder Step 5 in `next_opponent` `0x3ef96`, max 1 per run):**
   - Evaluated in deterministic priority order so at most **one** mid-tournament intrusion runs after ladder step 5:
     1. **Priority 1 — Bloodline Grudge (`Gou Hibiki` or `Dorai`):**
        - When playing as `Sagat`, `Adon`, or `Dan` with `continues_used == 0 && perfect_rounds >= 1`, **Gou Hibiki** intrudes on *Chao Phraya Twilight Shrine* (`"gou-hibiki"`, `0x44`).
        - When playing as `Gen` or `Chun-Li` with `continues_used == 0 && perfect_rounds >= 1`, **Dorai** intrudes on *Hong Kong Kowloon Courtyard* (`"dorai"`, `0x45`).
     2. **Priority 2 — Master's Trial (`Gouken` or `Goutetsu`):**
        - When playing as `Ryu`, `Ken`, or `Dan` (if `Dan` did not trigger Priority 1) with `continues_used == 0 && special_or_super_kos >= 3`, **Gouken** intrudes on *Goutetsu's Mountain Dojo* (`"gouken"`, `0x43`).
        - When playing as `Gouken`, `Akuma`, or `Retsu` with `continues_used == 0 && special_or_super_kos >= 3`, **Goutetsu** intrudes on *Goutetsu's Mountain Dojo* (`"gouken"`, `0x43`).
2. **Pre-Final *Akuma Shun Goku Satsu Ambush* (Between Ladder Steps 8 and 9 in Thailand, Priority Over Wave 1 `Evil Ryu`):**
   - **Priority Rule:** At the transition between ladder step 8 (`Adon`) and ladder step 9 (`Sagat`), when `released_waves >= 2` and the player is `Ryu`, `Ken`, `Gen`, `Gouken`, `Goutetsu`, or `Evil Ryu` with **`continues_used == 0 && perfect_rounds >= 2 && special_or_super_kos >= 3`**, **Akuma's ambush takes priority over Wave 1's Evil Ryu intrusion** (whereas if only the Wave 1 threshold `perfect_rounds >= 2 || special_or_super_kos >= 3` is met, Wave 1's `Evil Ryu` intrusion runs for non-`Ryu`/`Sagat`/`EvilRyu` fighters).
   - **Presentation:** As the pre-final ambush begins, the screen flashes pitch-black for `30` frames with the `15`-hit *Shun Goku Satsu* impact sound sequence, revealing **Akuma** standing in the center of **Goutetsu's Mountain Dojo** (`"gouken"`, `0x43`) with **`A NEW CHALLENGER ENTERS THE RING!`** (`0x3e`).
   - **Post-Match:** Under the **Exhibition Rule**, after the 2-out-of-3 match against CPU **Akuma** finishes (win or lose), `active_intrusion` and `arcade_stage_override` clear and the player proceeds to ladder step 9 (`Sagat`).
3. **Composite Congratulatory Endings (`0x44b00`) & Win Quotes (`0x4014c`) for All 10 Wave 2 Fighters:**
   - Every Wave 2 fighter has a composite `ending:{Name}` view in `BackgroundBank` (6×6 left-facing bust, 4×2 nameplate, home-stage palette), a `TextWriter` epilogue in `DataBank` (e.g., Gou Hibiki's spirit finding peace as Dan watches from the temple steps; Dorai uncovering Shadaloo's smuggling ring in Hong Kong alongside Gen and young Chun-Li; Guile returning home to Julia and Eliza alongside Ken), and standard + final post-match win quotes at `0x4014c`.

---

### 3.4 Pillar 4 — Wave 2 in 2v1 Dramatic Battle (`Presets 9–14`) & 1987 World Tour (`Nodes #21–#25`)

#### A. Six New Wave 2 Rivalry Presets in `DramaticSetupScreen` (`Presets 9–14`, gated on `released_waves >= 2`)
| # | Preset Title | Allies (Team of 2) | Boss (1536 Sub-HP, 1.25× Dmg) | Stage & Theme | Lore Tie |
|---|---|---|---|---|---|
| 9 | **Father & Son's Vengeance** | **Gou Hibiki & Dan** | **Sagat** | Thailand — *Chao Phraya Twilight Shrine* (`0x44`) | Father and son unite across life and death against the Emperor of Muay Thai |
| 10 | **Disciples of Ansatsuken** | **Gouken & Retsu** | **Goutetsu** | Japan — *Goutetsu's Mountain Dojo* (`0x43`) | Preserved from [`2026-11-03-bonus-roster-by-lore-design.md` §8](2026-11-03-bonus-roster-by-lore-design.md): Gouken and Retsu test their mastery against Goutetsu |
| 11 | **Assassins' Blood Feud** | **Gen & Dorai** | **Akuma** | China — *Hong Kong Kowloon Courtyard* (`0x45`) | Old friends Gen and Inspector Dorai stand together against the Demon of the Satsui no Hado |
| 12 | **Father & Daughter** | **Dorai & Chun-Li** | **M. Bison** | China — *Hong Kong Kowloon Courtyard* (`0x45`) | Inspector Dorai and 19-year-old Chun-Li confront Lord Bison's Hong Kong operation |
| 13 | **Brothers-in-Law** | **Ken & Guile** | **M. Bison** | USA — *US Air Force Tarmac* (`0x46`) | Brothers-in-law Ken Masters and Captain Guile team up against Shadaloo |
| 14 | **The Hermit & The Monk** | **Retsu & Dhalsim** | **Oro** | Brazil — *Deep Hermit Cavern* (`0x48`) | Old friends Retsu and Dhalsim spar against the 140-year-old Senjutsu immortal |

#### B. Five Wave 2 Bloodlines Nodes in `1987 World Tour` (`Nodes #21–#25`, gated on `released_waves >= 2`)
| # | Node ID | Country Region | Prerequisite Node | Mission Type | Opponent(s) & Stage | Reward |
|---|---|---|---|---|---|---|
| 21 | `jp-5-ansatsuken` | Japan | `jp-4-boss` | **2v1 Dramatic Gauntlet** | **Gouken & Goutetsu** (`Goutetsu's Mountain Dojo`, `0x43`) | `+350 XP` + *Fist of Nothingness* Perk |
| 22 | `us-5-airbase` | USA | `us-4-boss` | **Rule-Modifier Trial (Charge Mastery)** | **Guile** (`US Air Force Tarmac`, `0x46`) | `+300 XP` + *Sonic Hurricane* Badge |
| 23 | `cn-5-kowloon` | China | `cn-4-boss` | **2v1 Dramatic Bout** | **Dorai & Chun-Li** (`Hong Kong Kowloon Courtyard`, `0x45`) | `+350 XP` + *Genden Heritage* Perk |
| 24 | `en-5-ashram` | England | `en-4-boss` | **2v1 Era Style Mastery Bout** | **Dhalsim & Oro** (`Ganges River Temple`, `0x47`) | `+400 XP` + *Senjutsu Hermit* Perk |
| 25 | `th-5-eyepatch` | Thailand | `th-2-era` | **1977 Historical Flashback Bout** | **Player locked as Gou Hibiki** vs. **CPU Sagat** (`Chao Phraya Twilight Shrine`, `0x44`, recreating the 1977 duel that took Sagat's right eye) | `+500 XP` + *Wave 2: Bloodlines* ★ Star |

---

### 3.5 Pillar 5 — PixelLab Art Pass (`BW8`–`BW11`) & Flipping `RELEASED_WAVES = 2` (`8×3 = 24` Select Grid)

To keep PixelLab credit usage disciplined and explicit (`ADR-0006`):
1. **`BW8` (Ansatsuken & Saikyo Sprites & Busts):** Generate 1987-style sprite sheets and 6×6 VS busts for **Gou Hibiki**, **Dan**, **Gouken**, **Akuma**, and **Goutetsu** (reusing/re-paletting shared Ansatsuken gi base frames where appropriate to save credits, and generating bespoke masks, hair/beards, haori, and unique specials).
2. **`BW9` (Chun-Li, Guile, Dhalsim, Oro & Dorai Sprites & Busts):** Generate full 1987-proportioned sprite sheets and 6×6 VS busts for **Chun-Li**, **Guile**, **Dhalsim** (including stretchy-limb tiles), **Oro**, and **Dorai**.
3. **`BW10` (6 Bloodlines Stage Backdrops):** Generate and pack the 1987-style `bg`/`fg` tilemaps for `"gouken"`, `"gou-hibiki"`, `"dorai"`, `"guile"`, `"dhalsim"`, and `"oro"`.
4. **`BW11` (10 Drawn-to-Size `26×30` Select Portraits & `RELEASED_WAVES = 2` Gate Flip):** Generate the 10 drawn-to-size `26×30` (`24×28` inner) Character Select portraits for `slots 14..24` and **flip `RELEASED_WAVES` from `1` to `2`**, automatically expanding the Character Select screen from `7×2 = 14` to **`8×3 = 24`** under the Even-Rows rule (`BC2` `#294`).

---

## 4. What's Left to the Builder

The following implementation details are intentionally left to engineering judgment during ticket execution:
- Exact byte offsets within `0x1e_0000..=0x27_ffff` in `DataBank` for each Wave 2 challenger's move streams, projectile templates, and teleport/counter tables.
- Exact YM2151 FM voice patches and note sequences in `art/themes/{gouken,gou-hibiki,dorai,guile,dhalsim,oro}.score` (`0x43`–`0x48`).
- Exact `(x, y)` coordinates of the 5 `[WAVE 2]` nodes (`jp-5-ansatsuken`, `us-5-airbase`, `cn-5-kowloon`, `en-5-ashram`, `th-5-eyepatch`) on `WorldTourScreen`.
