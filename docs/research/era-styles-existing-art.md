# Era styles on existing art: visual recipes, move mappings and PixelLab scope

Research for **Project 4 (Era styles and gauges on the real fighters)**: how to realise `SF2` (`WW`–`ST`), `Alpha` (`SFZ1`–`SFZ3`), `SF3` (`NG`–`3S`), `SF4` (`SF4`–`USF4`), `SF5` (`SF5`–`CE`) and `SF6`—both canonical and what-if styles—using only the 1987 ROM assets, the approved `sf1-port-art` pose catalogues, and the overhaul compositor, while scoping the two November 27 PixelLab-gated tickets (Birdie's chain/food props and Gen's Crane stance). Gathered 2026-11-08.

---

## 1. Visual and audio primitives already in the repository

### 1.1 Sprite tiles, recipes and the extended sprite bank
- **ROM sprite region (`gfx3`):** `0x3800` (14,336) 16×16 4bpp tiles (`crates/sf-video/src/sprite_bank.rs`). Every 1987 pose, projectile, hit spark and dust cloud lives below `0x3800`.
- **Extended sprite bank (ADR-0007):** Loaded above `0x3800` in overhaul fights (`crates/sf-video/src/art.rs`). All twelve fighters' `SF1+` catalogues (`art/recipes/*.json` + `tools/reference/local/roms/sf1-port-art`) are finished and tested in `crates/sf-host/tests/art_at_launch.rs` (about 12,555 tile slots).
- **Per-fighter pose coverage (`art/recipes/*.json`):** Every one of the twelve tournament fighters now has:
  - standing, crouching and jumping normals across all six buttons;
  - standing and crouching guard poses;
  - standing and crouching hit reactions, airborne hit reactions, knock-out lying poses and rising get-up poses;
  - a ground throw (grab/drive sequence) and a being-thrown sequence;
  - every `SF1+` special move (including Retsu's 6-frame `Kiai Shot` ki blast in the art pack).

### 1.2 Projectiles and hit effects in the ROM and art pack
| Effect | Source | Details |
| :--- | :--- | :--- |
| **Hadoken & explosion** | ROM template `0x186dc` (`docs/reference/hadoken.md`) | Tile code `0x01e8`, attr `0x0404` (sub-palette 4), looping 2-frame flight stream `0x19034`, impact explosion stream on hit/guard/clash. |
| **Tiger Shot** | ROM stream `0x144b4` (`docs/reference/sagat.md`) | Sagat's projectile template; `Low Tiger Shot` reuses the same projectile spawned from his crouching thrust (`art/recipes/sagat.json`). |
| **Shuriken** | ROM stream `0x0bf98` (`docs/reference/geki.md`) | Geki's star templates, parameterized per button strength via `ROUTINE_PROJECTILE_BY_STRENGTH` (`crates/sf-engine/src/data_bank.rs`). |
| **Kiai Shot blast** | Art pack (`art/recipes/retsu.json`, `SP2` `#248`) | Six approved 1987-style ki-burst frames in `sf1-port-art`. Can also stand in for short-range palm bursts (such as `SF6` Ryu's *Hashogeki*). |
| **Teleport smoke** | ROM poses `0x11e6`, `0x1326`, `0x1329`, `0x132d` (`docs/planning/art-upgrades.md`) | Four-stage ninja smoke cloud used by Geki's teleport; reusable for vanish/shift transitions. |
| **Hit sparks & block flashes** | ROM object slots `0xffe100..` (`crates/sf-engine/src/fighter.rs`) | Spawned in the 12 effect slots (`0x40` bytes each) whenever a melee or projectile hit connects. |

### 1.3 Palette, compositor and screen-effect primitives (`crates/sf-video/`)
Because overhaul fights render through `sf-video`'s compositor (`crates/sf-video/src/compositor.rs`, `callouts.rs`, `era_hud.rs`), we can apply classic Capcom 2D visual treatments to any fighter or projectile sprite without adding a single tile to `gfx3`:
1. **Stage dim (`callouts::dim`, line 722):** Halves the brightness (`(*p >> 1) & 0x007f_7f7f`) of stage pixels while keeping the fighters, projectiles and HUD at full brightness during a Super/Ultra/Critical Art startup freeze.
2. **Burnout grey (`callouts::grey`, line 731):** Converts a burnt-out `SF6` fighter's sprite palette to luminance grey (`30/59/11` weights) for the duration of Burnout.
3. **Per-frame sprite palette tint / pulse:** Each fighter's sprites reference a 16-color sub-palette in `Palette` (`crates/sf-video/src/palette.rs`, `xRGB_444`). Overhaul presentation can tint or pulse a fighter's active sub-palette pens on specific frames:
   - **Gold pulse (`EX` / `Overdrive`):** Shift mid/highlight pens toward PLAYER1 gold (`#ffcc44` / `#ffaa00`) on alternate frames (`frame & 2 != 0`), matching *SF3 2nd Impact / 3rd Strike* and *SF4* EX moves.
   - **Blue/white flash (`Parry` / `Drive Parry` / `Alpha Counter`):** 2-frame bright cyan-white (`#ccf4ff`) flash on the defender's guard pose when a Parry or Alpha Counter triggers, matching *3rd Strike* and *Alpha*.
   - **Ink/white charge pulse (`Focus Attack` / `Red Focus`):** White outline/body pulse for Level 1–2 Focus, flashing bright crimson (`#ff3322`) on Level 3 unblockable or *USF4* `Red Focus`.
   - **Crimson/orange burn (`Shakunetsu Hadoken` / `Heat Rush`):** Recolour the *Hadoken* projectile sub-palette from cyan-blue to crimson-orange (`#ff4400` / `#ffaa22`), and tint the hit reaction orange for 12 frames.
   - **Aura tint (`V-Trigger` / `Denjin Renki` / `Angry Charge` / `Renkiko`):** Subtle 1-step highlight shift on the fighter's skin/gi pens while the buff or V-Trigger timer is active.
4. **Afterimage shadow trails (2–3 ghost copies):**
   - Classic Capcom games (*Super Turbo* Supers, *Alpha* `V-ISM` Custom Combos, *SF4* `FADC`, *SF6* `Drive Rush`) draw 2 or 3 trailing copies of the fighter's previous poses at `(x_prev, y_prev)` every 4 frames, rendered in a single-hue silhouette palette (cyan/blue for `ST` Super and `SF3` Super Art, gold/orange for `V-ISM`, green for `Drive Rush`, dark violet for `Drive Impact`).
   - Because the overhaul presenter knows each fighter's last 3 `(grid, x, y, flip)` sprite states, it can composite 2 trailing silhouettes behind the live fighter sprite with zero 68000 object-RAM overhead.

### 1.4 Audio primitives (`crates/sf-audio/`)
- **YM2151 FM sound effects (`music.rs`, `tunes.rs`):** Existing 1987 FM commands cover projectile launch (`0x07`), explosions (`0x06`), block clanks, and heavy impacts.
- **Dual MSM5205 ADPCM voices (`samples.rs`):** Fighter kiai shouts and special calls (`Hadoken`, `Shoryuken`, `Tatsumaki`, boss grunts).
- **Third MSM5205 Announcer (`announcer.rs`, ADR-0011):** Already holds all 11 word call-outs (`SUPER`, `COUNTER`, `PUNISH COUNTER`, `CRUSH COUNTER`, `PERFECT PARRY`, `BURNOUT`, `CUSTOM COMBO`, `ALPHA COUNTER`, `REVERSAL`, `FIRST ATTACK`, `PERFECT`) plus `5`–`15+` hit combo calls in `tools/reference/local/roms/sf1-port-voice`.

---

## 2. System mechanics visual recipe book (zero new sprites)

Every post-1987 system mechanic across `SF2`–`SF6` can be staged from existing poses plus the compositor primitives in section 1.3:

| Era | Mechanic | Pose stream used | Visual & audio treatment |
| :--- | :--- | :--- | :--- |
| **SF2 (`Super`/`ST`)** | *Shakunetsu Hadoken* | Ryu's *Hadoken* (`0x1a`) | Projectile drawn with crimson/orange sub-palette; hit target flashes orange during hitstun. |
| **SF2 (`ST`)** | Super Combo startup & trail | Fighter's Super stream | 20f startup freeze + `callouts::dim` stage darkening + `"Super!"` call-out + 2 blue afterimage shadows throughout the move. |
| **Alpha (`Z1`–`Z3`)** | `Air Block` | Standing/crouching guard pose (`move 4`/`8`) held at airborne `(x, y)` | Keeps airborne trajectory; spawns guard spark and block sound on impact. |
| **Alpha (`Z1`–`Z3`)** | `Alpha Counter` | Guard pose (4f) → fighter's standing heavy or rising special | Gauge flashes (`GaugeState::Flashing`), cyan-white flash on guard pose, 10f freeze, `"Alpha counter!"` call-out, invincible startup strike. |
| **Alpha (`Z2`–`Z3`)** | `Custom Combo` (`V-ISM`) | Any normal/special stream | 15f startup freeze + `"Custom combo!"` call-out + draining gauge + 2 gold/yellow afterimage shadows trailing the fighter while `custom_left > 0`. |
| **SF3 (`NG`–`3S`)** | `Parry` & `Red Parry` | Standing guard (`move 4`) for high/air, crouching guard (`move 8`) for low | 2f cyan-white palette flash (`Red Parry` flashes crimson-gold first), 8f defender recovery advantage, guard sound + spark. |
| **SF3 (`2I`–`3S`)** | `EX Special` | Fighter's special stream | Alternating 2f gold palette pulse + 1 gold afterimage shadow + multi-hit property. |
| **SF3 (`NG`–`3S`)** | `Universal Overhead` (`MP+MK`) | Short 14f hop (`dy` arc peaking 16 px off ground) into standing/jumping heavy attack pose | Hits standing-block only (overhead flag); no new art needed. |
| **SF4 (`SF4`–`USF4`)** | `Focus Attack` & `Red Focus` | Chamber/guard pose held while charging (`MP+MK`) → standing heavy strike on release | 1 hit of armor (3 in `Red Focus`); white pulse on Lv1, yellow on Lv2, red on Lv3 / `Red Focus`. Lv2/Lv3 hit causes **Crumple** (standing hit pose held 20f → crouch hit 12f → lying pose, hittable on the way down). |
| **SF4 (`SF4`–`USF4`)** | `FADC` (Focus Attack Dash Cancel) | 2-frame step-in / walk poses translated `±32 px` over 10–12f | Yellow flash on cancel + 2 white afterimage shadows. |
| **SF4 (`USF4`)** | `Delayed Wakeup` | Knock-out lying pose (`art/recipes/*.json` `get-up` frame 0) | Holds the lying pose for 20 extra frames before running the rising get-up stream. |
| **SF5 (`SF5`–`CE`)** | `Crush Counter` | Heavy normal counter-hit | 6f extra hitstop + `"Crush counter!"` call-out + defender plays airborne-hit spin into knockdown. |
| **SF5 (`SF5`–`CE`)** | `V-Trigger` & `V-Reversal` | Chamber/taunt pose (15f freeze + dim) for V-Trigger; guard → heavy pushback strike for V-Reversal | Draining V-Gauge (`GaugeState::Draining`) + subtle red/gold highlight tint while active. |
| **SF5 (`CE`)** | `V-Shift` & `V-Shift Break` | Back-walk / hop-back pose with blue afterimages (`V-Shift`) → lunging heavy strike (`V-Shift Break`) | Invulnerable backstep; if it dodges an attack box within its active window, triggers brief slow-motion and enables `V-Shift Break`. |
| **SF6 (`SF6`)** | `Drive Parry` & `Perfect Parry` | Standing guard pose held while `MP+MK` is down | Blue tint while held; `Perfect Parry` (within 2f) triggers stage dim + `"Perfect parry!"` call-out. |
| **SF6 (`SF6`)** | `Drive Rush` (`Parry` / `Cancel` / `Raw`) | Fast forward dash using step-in / lunge pose (`+48 px` over 11f) | Bright green flash on start + 2 green afterimage trails; grants +4f frame advantage on the next normal. |
| **SF6 (`SF6`)** | `Drive Impact` & `Wall Splat` | Chamber wind-up (2 hits of armor, crimson-violet splash tint) → heavy strike | In the corner (or against a defender in `Burnout`), a hit or block pushes the defender to the stage edge, holds their airborne/standing hit pose pinned against the wall for 24f (`Wall Splat`), then drops them to their knees/crumple. |

---

## 3. Fighter-by-fighter move & Super mapping from existing poses

Every canonical and what-if move across the twelve fighters maps onto existing ROM streams and `sf1-port-art` catalogue poses:

| Fighter | Era Moves & Supers / Ultras | Existing Pose / Stream Recipe |
| :--- | :--- | :--- |
| **Ryu** | *Shakunetsu Hadoken* (`SF2 Super`+), *Joudan Sokutogeri* (`SF3`+), *Hashogeki* & *Denjin Charge* (`SF6`); *Shinku Hadoken*, *Shin Shoryuken*, *Denjin Hadoken*, *Metsu Hadoken* / *Metsu Shoryuken*, *Shin Hashogeki* | *Joudan*: ROM standing high kick (`0x16`) with forward step (`dx = +12`) and wall-bounce on counter/EX. *Hashogeki*: Hadoken palm thrust (`0x042a`) spawning Retsu's short ki-burst effect (`SP2`) with Ryu's palette. *Denjin Charge*: Hadoken gather pose held with electric-yellow palette pulse. *Shinku* / *Metsu* / *Denjin Hadoken*: 5-hit fast Hadoken stream with Super freeze + blue/gold trails. *Shin* / *Metsu Shoryuken*: deep crouch → close hit → rising multi-hit Shoryuken (`0x19`) at 1.5× vertical velocity. |
| **Ken** | Flaming *Shoryuken* (`SF2 Super`+), *Jinrai Kick* (`Low`/`Mid`/`Overhead` follow-ups) & *Dragonlash Kick* (`SF6`), *Quick Step* (`SF5`); *Shoryureppa*, *Shinryuken*, *Shippu Jinraikyaku*, *Guren Senpukyaku* | Flaming *Shoryuken*: crimson-orange tint on Ryu/Ken's Shoryuken strike frames. *Jinrai Kick*: standing MK (`0x15`) cancelable into crouching LK (`Low`), standing LK (`Mid`), or hopping HK (`Overhead`, `dy` hop). *Dragonlash*: diagonal jump kick arc (`0x24`) starting from standing. *Shoryureppa*: two sequential short-arc Shoryukens (`0x19`) advancing forward. *Shinryuken*: stationary vertical multi-hit Shoryuken spinning through Tatsumaki/Shoryuken frames. *Shippu Jinraikyaku*: 4 rapid standing kicks (`0x14`, `0x15`, `0x16`) into a fast rising multi-hit Tatsumaki (`0x1b`). |
| **Sagat** | *Tiger Uppercut* (`SF2`+), *Angry Charge* (`SF4`+), *Tiger Nexus* (`SF6`); *Tiger Genocide*, *Tiger Cannon*, *Tiger Destruction*, *Tiger Raid*, *Tiger Assault* | *Tiger Uppercut*: his `SF1+` rising uppercut / high lunging punch (`0x2d00`) on an upward `dy` arc (`SP12`). *Angry Charge*: Tiger Shot chamber pose with red/gold flash, buffing the next Tiger Uppercut/Ultra. *Tiger Genocide*: *Tiger Knee* (`0x14242`) straight into multi-hit *Tiger Uppercut*. *Tiger Cannon*: 5-hit fast high *Tiger Shot* (`0x144b4`) with blue/gold Super trail. *Tiger Raid* / *Destruction*: two low kicks into advancing *Tiger Knee* → *Tiger Uppercut*. |
| **Adon** | *Jaguar Tooth* (`Alpha`+), *Rising Jaguar*, *Jaguar Kick* (ground & air); *Jaguar Varied Assault*, *Jaguar Revolver*, *Jaguar Avalanche*, *Jaguar Thousand* | *Jaguar Tooth*: back-step to screen edge (hop pose `0x3240`) followed by long-arc flying kick (`0x1348c`). *Jaguar Revolver*: three consecutive arcing *Jaguar Kicks* (`0x1348c`), the third rising into *Rising Jaguar*. *Jaguar Varied Assault* / *Avalanche*: lunging punch + elbow/knee combo (`0x3304`, `0x3128`) into multi-hit *Rising Jaguar*. |
| **Gen** | *Mantis* (`Hyakurenko`, `Gekiro`, *Zan'ei*, *Shitenshuu*, *Zetsuei*, *Ryukoha*) & *Crane* (*Jasen*, *Oga*, *Jakorifu*, *Teiga*, *Kouga*) | *Mantis* uses his `SF1+` kit (*Hyakurenko* `0x10b4e`, *Gekiro* somersault recipes); *Zan'ei* / *Zetsuei* is a high-speed dash-through strike (`0x22a0` + afterimages) with delayed multi-hit damage; *Shitenshuu* / *Ryukoha* is a multi-hit *Hyakurenko* into *Gekiro*. *Crane* uses temporary ROM stand-ins (section 4.2) until the Nov 27 PixelLab ticket lands. |
| **Birdie** | *Bull Head* (`0x11b42`), *Bull Horn* (`AP22`), *Bull Revenger* (leaping grab); *The Birdie* (3× *Bull Head*), *Murderer Chain*, *Break Time* / *Enjoy Time* | *Bull Revenger*: hop arc (`0x240c`) landing in his `AP21` head-slam throw. *The Birdie*: three rapid-fire *Bull Head* rushes with Super trail. Chain & food moves use temporary ROM stand-ins (section 4.1) until the Nov 27 PixelLab ticket lands. |
| **Eagle** | *Manchester Black* (`0x124d8`), *St. Andrew's Green* (`0x129bc`), *Liverpool White*, *Canterbury Blue*; *Manchester Gold*, *Union Jack* | *Liverpool White* / *Canterbury Blue*: advancing stick combo (`0x2980` → `two-stick smash`). *Manchester Gold* / *Union Jack*: extended multi-hit spinning-sticks rush (`0x124d8`) finishing with leaping stick smash (`0x129bc`). |
| **Retsu** | *Kiai Shot*, *Tobigeri* (ROM leaping kick `0x0cd18`), *Renkiko* (ki charge buff), *Hundred Monk Slaps*, *Izuna Shoulder Throw*; *Tenchi Shorinji Drop* | *Tobigeri* is already in Retsu's ROM (`0x0cd18`). *Renkiko*: Kiai Shot gather pose held 20f with gold pulse, buffing next special (`sf6.rs`). *Tenchi Shorinji Drop*: *Tobigeri* leap into multi-hit *Kiai Shot* + wrist throw. |
| **Geki, Joe, Mike, Lee** *(What-If)* | Each fighter's `what_if.rs` / `flagship.rs` specials & Supers (*Ansatsu Geki-Retsuzan*, *Thrash Kick* / *Meteor Kick*, *Crazy Rush*, *Hundred Spears* / *One Man Army*) | Built directly by chaining each fighter's `SF1+` special poses and ROM streams with Super freeze, stage dim, and 2 afterimage shadows: Geki (teleport smoke → shuriken volley → diving claw), Joe (*Rolling Sobat* → *Dynamite Straight* flurry), Mike (3-step *Bison Rush* → *Iron Upper*), Lee (*Zesshouohken* → *Double Kick* flurry). |

---

## 4. Scope of the two November 27 PixelLab tickets

Both items below are included as dedicated tickets in Project 4, marked **waiting on the November 27 PixelLab credit reset**. So that no other ticket in Project 4 is blocked by them, the engine and move tables ship with clean ROM stand-ins first and swap in the generated poses once approved on the `sf-art` review page.

### 4.1 Ticket A: Birdie's chain and food props (`Alpha` & `SF5`)
- **Why needed:** Birdie's `Alpha` and `SF5` kits feature his signature chain grabs (*Murderer Chain*, *Hanging Chain*) and `SF5` V-Skill items (*Break Time* banana peel / energy drink can / donut). Neither the 1987 ROM nor `AP21`–`AP22` includes chain links or food item sprites.
- **Poses to generate (after Nov 27 reset):**
  1. **Chain extension (2–3 poses + repeatable chain-link effect tile):** Birdie whipping his chain forward horizontally (`Murderer Chain` / standing chain poke) and diagonally upward (`Hanging Chain`), using his existing `0x2480` / `0x26a0` torso with the chain drawn in his steel/gold palette pens.
  2. **`SF5` V-Skill item props (3 small 16×16 or 32×16 effect sprites + 1 eating/toss pose):**
     - **Donut** (chewed in place, +V-meter),
     - **Banana peel** (placed on ground as a low-hitting trap effect entity),
     - **Drink can** (rolled forward along the ground as a slow low projectile).
  - **Estimated cost:** ~6 poses/sprites × ~17 generations = **~100 PixelLab credits** (out of the 2,000 monthly allowance).
- **Immediate ROM stand-in (until Nov 27):** *Murderer Chain* uses his two-handed reach (`0x26a0`) into his `AP21` head-slam throw at extended grab range; *Break Time* uses his wind-up pose (`0x2820`) to grant the V-meter/buff directly.

### 4.2 Ticket B: Gen's `Crane` (*Ki*) stance and normals (`Alpha` & `SF4`)
- **Why needed:** In `SF1+`, Gen uses a single stance (*Mantis*). In `Alpha 2`/`3` and `SF4`, pressing `PPP` / `KKK` switches between *Mantis* (*Sou*) and *Crane* (*Ki*), which has a one-legged crane idle stance, different ground normals, a long jump arc, *Jasen* (rolling attack, already in his ROM at `0x2300`!), and *Oga* (wall dive kick).
- **Poses to generate (after Nov 27 reset):**
  1. **Crane idle stance (1–2 poses):** Gen balanced on one leg, arms raised like crane wings, in his 1987 robe and palette.
  2. **Crane distinctive strikes (3–4 poses):** His crane-beak two-handed overhead strike, crane snap kick, and wall-cling / dive-kick pose for *Oga*.
  - **Estimated cost:** ~5–6 poses × ~17 generations = **~90–100 PixelLab credits**.
- **Immediate ROM stand-in (until Nov 27):** Switching to *Crane* (`KKK`) uses his high-guard / one-hand-raised ROM pose (`0x20cc` / `0x2180`) as the Crane stance, binds his ROM rolling low kick (`0x2300`) as *Jasen* and his ROM high flying kick (`0x10580` / `0x21e0`) as *Oga*, and swaps his normal button table to his kick-heavy ROM streams so Mantis/Crane switching is 100% playable and testable before the new sprites arrive.
