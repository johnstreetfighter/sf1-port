# Street Fighter Community Research: Original Game Criticisms, Fan Wishlists, and Series-Wide Expectations

This document synthesizes primary-source research across online *Street Fighter* communities—including **Reddit (`r/StreetFighter`, `r/Fighters`)**, **SuperCombo Wiki (formerly Shoryuken/SRK Wiki)**, **Arcade-Projects**, **EventHubs**, **ResetEra**, **Steam Community**, **SF1Cord / Fightcade**, **GameBanana / M.U.G.E.N / Ikemen GO modding communities**, **historical developer interviews (*Gamest*, *Polygon*)**, and **verified 68000 ROM disassembly documentation in this repository (`docs/reference/*.md`)**.

In accordance with [`CONTEXT.md`](../../CONTEXT.md) and [`docs/project-brief.md`](../project-brief.md):
- **Original game** refers to the 1987 arcade *Street Fighter*.
- **Reference version** refers to US set 1 with six-button controls (`sf` in MAME).
- **Port** refers to this project's PC version of the original game, which preserves original gameplay, timing quirks, and bugs as its initial **gameplay fidelity** baseline.
- **Optional improvement** refers to any deliberately selectable change to original gameplay, kept strictly separate from the faithful baseline.

---

## Part I: Criticisms of the Original Game (1987 Arcade)

Across retro-gaming retrospectives, FGC forums, and Steam reviews of compilations (*Street Fighter 30th Anniversary Collection*, *Capcom Arcade 2nd Stadium*), criticisms of the **original game** cluster into five mechanical and structural areas. Each community complaint below is paired with the exact hardware or 68000 ROM mechanism recovered in [`docs/reference/`](../reference/).

### 1.1 Controls, Button-Release Latching, and Special Move Unresponsiveness

#### A. Mandatory "Negative Edge" (Button-Release Registration) on Normal Attacks
- **Community Criticism:** Players and reviewers universally cite the original game's controls as sluggish, inconsistent, or "broken," noting that normal punches and kicks feel delayed or fail to register during fast exchanges. *Hardcore Gaming 101* writes: *"The original Street Fighter just doesn't work very well. The controls are among the worst in fighting game history, as they just don't work most of the time. Trying to execute a special move is a crapshoot."* ([Hardcore Gaming 101](https://www.hardcoregaming101.net/fuuun-shourinken/); [SuperCombo Wiki – Controls](https://wiki.supercombo.gg/w/Street_Fighter_(1987)/Controls)).
- **ROM Mechanism ([`docs/reference/punch.md`](../reference/punch.md), `0x3b3ae`):**
  - Even in the six-button **reference version** (`sf`), the game does not trigger attacks on button press (positive edge). Instead, the VBlank routine at `0x3b3ae` emulates the original cabinet's pneumatic pressure pads in software.
  - While any button in the punch group or kick group is held down, `0x3b3ae` increments a frame hold counter (`state +3`, saturating at `0xff`) and records the strongest button pressed (`state +4`: `1` = Light, `2` = Medium, `3` = Heavy).
  - **Only when all buttons in that group are physically released** does the routine look up the simulated pad pressure from table `0x3b450` (`0x40` Light, `0xe0` Medium, `0xfe` Heavy) and write it to the player's attack latch (`0xff86b0`).
  - Consequently:
    1. Every normal attack is delayed until at least **1 frame after button release** (`release + 1`).
    2. Holding a button for **255 frames (`0xff`)** saturates the counter, causing the release to produce **no attack at all** ([`docs/reference/punch.md`](../reference/punch.md)).
    3. The input handler's tail (`0x4050c`) drops latched button strengths whenever an attack is already in progress (`+9 & 0xc0 == 0xc0`), discarding inputs made during attack recovery rather than buffering them ([`docs/reference/posture-dispatch.md`](../reference/posture-dispatch.md)).

#### B. Why Special Moves (*Hadoken*, *Shoryuken*, *Tatsumaki Senpukyaku*) Fail with Standard Muscle Memory
- **Community Criticism:** Players using standard *Street Fighter II* quarter-circle or Dragon Punch muscle memory report that special moves fail 80–90% of the time unless they use the community-discovered "hold-and-release" or joystick-wiggling workarounds ([SuperCombo Wiki – Controls](https://wiki.supercombo.gg/w/Street_Fighter_(1987)/Controls); [Steam Community Discussions](https://steamcommunity.com/app/586200/discussions/); [Indie Gamer Chick Review](https://indiegamerchick.com/2022/08/21/capcom-arcade-2nd-stadium-the-definitive-review-complete-34-game-review-ranking/)).
- **ROM Mechanism ([`docs/reference/posture-dispatch.md`](../reference/posture-dispatch.md), [`docs/reference/specials.md`](../reference/specials.md), [`docs/reference/hadoken.md`](../reference/hadoken.md)):**
  Four compounding rules in the input handler (`0x402f8`) and command trackers (`0x40360–0x403b2`, routine `0x40ae4`) cause special moves to drop:
  1. **Strict 18-Frame (`0x12`) Total Window:** From the frame the first direction of a command is entered (`Down` for *Hadoken* `05 06 02` or *Tatsumaki* `05 07 03`; `Forward` for *Shoryuken* `02 05 06`), a timer counts up to `0x12` (**18 frames / 0.3 seconds**). If the third direction is not reached and the button release is not dispatched within 18 frames of the first direction, the tracker resets.
  2. **Zero Directional Release Leniency:** Stage 3 of a command tracker (`done = 0xff`) **only remains valid while the final direction (`Forward`, `Down-Forward`, or `Back`) is continuously held**. Leaving the final direction for even 1 frame before the attack dispatches immediately resets the tracker.
  3. **Walk-Cycle Busy Gating Eats Special Inputs (`0x403d4`):** Because the final direction of *Hadoken* is `Forward` and *Tatsumaki* is `Back`, holding that final direction starts a forward or backward walk. However, Ryu and Ken's walk stream alternates the entity state byte (`+9`) between **interruptible frames (`0x92`, only 2 frames per 10-frame cycle)** and **gated/busy frames (`0x91`, 8 frames per 10-frame cycle)**. If the player completes the motion and releases the button during the 8 gated walk frames (`0x91`), dispatch is blocked until the next `0x92` frame; if the player releases the joystick to neutral during those 8 frames, the tracker resets and a normal standing punch/kick fires instead ([`docs/reference/hadoken.md`](../reference/hadoken.md), [`docs/reference/specials.md`](../reference/specials.md)).
  4. **Tracker Resets on Normal Attacks (`0x406ac` / `0x407d0`):** Any forward or back normal attack resets both the *Hadoken* and *Tatsumaki* trackers and clears both latches ([`docs/reference/standing-attacks.md`](../reference/standing-attacks.md)).

#### C. The Deluxe Pneumatic Cabinet (`sfan`) vs. Six-Button Conversion (`sf`)
- **Historical & Community Criticisms:** Directed by **Takashi Nishiyama** and **Hiroshi Matsumoto**, the original deluxe cabinet (co-designed with Atari Games in the US) featured two large rubber-covered mechatronic pads connected to **SMC Pneumatics air cylinders (model CQ2B32)** and semiconductor pressure transducers ([Polygon Oral History by Matt Leone](https://www.polygon.com/2020/7/7/21270906/street-fighter-1-oral-history-takashi-nishiyama); [Shmuplations – 1987 *Gamest* Developer Interview](https://shmuplations.com/streetfighter1987/); [HCI Interface Museum](https://interfacemuseum.com/exhibits/street-fighter-pneumatic/); [Arcade-Projects Thread #10707](https://www.arcade-projects.com/threads/street-fighter-1-pneumatic-board.10707/)).
- Matsumoto noted in *Gamest* (1987) that testing the pads caused developers' hands to swell, and producer **Yoshiki Okamoto** recalled bloodying his fists ([Shmuplations](https://shmuplations.com/streetfighter1987/); [Polygon](https://www.polygon.com/2020/7/7/21270906/street-fighter-1-oral-history-takashi-nishiyama)). In arcades, players bruised wrists, bloodied knuckles on the cabinet trim, or slammed/kicked the pads until air hoses and sensors broke, prompting Capcom to recall/replace the ~1,000 deluxe units in favor of standard 6-button microswitch cabinets (`sf`) ([HCI Interface Museum](https://interfacemuseum.com/exhibits/street-fighter-pneumatic/); [The Cutting Room Floor](https://tcrf.net/Street_Fighter)).

---

### 1.2 Extreme Damage Scaling, Chip Damage, and Gameplay Degeneration

#### A. Special Moves Deleting 41.7% to 100% Health vs. Low-Damage Normals
- **Community Criticism:** Players criticize the extreme imbalance between normal attacks (which deal scratch damage) and special moves (which end rounds in 1 to 3 hits), turning matches into frantic special-move spam rather than spacing duels ([StrategyWiki – Moves](https://strategywiki.org/wiki/Street_Fighter/Moves); [Hardcore Gaming 101](https://www.hardcoregaming101.net/fuuun-shourinken/)).
- **ROM Mechanism ([`docs/reference/standing-attacks.md`](../reference/standing-attacks.md), [`docs/reference/specials.md`](../reference/specials.md), [`docs/reference/hadoken.md`](../reference/hadoken.md)):**
  - Fighter health (`0x3c034`) starts at **48 HP** (11 HUD cells of 4 HP + 4 HP remainder).
  - **Standing Normals (`depth 2` loose box / `depth 1` tight box):**
    - Light Punch (`0x0d`): **1 / 2 HP** (2.1% – 4.2% life)
    - Light Kick (`0x14`): **4 / 4 HP** (8.3% life)
    - Medium Punch (`0x0f`): **6 / 6 HP** (12.5% life)
    - Medium Kick (`0x15`): **6 / 8 HP** (12.5% – 16.7% life)
    - Heavy Punch (`0x10`): **8 / 8 HP** (16.7% life)
    - Heavy Kick (`0x16`): **8 / 10 HP** (16.7% – 20.8% life)
  - **Special Moves:**
    - *Hadoken* (`0x4345e`), *Shoryuken* (`0x45f52`), and *Tatsumaki Senpukyaku* (`0x46012`) each inflict **20 HP (41.7% of total health)** per landed hit—equivalent to 10 clean Light Punches or 20 partial Light Punches.
    - **Unblockable, Frame-1 Invincible *Shoryuken*:** Ryu's *Shoryuken* hit function (`0x45efa`) **contains no block test at all** (it hits through guard for full 20 HP damage) and sets the entity invulnerability bit (`+0xf` bit 3) from frame 1, causing the collision judge (`0x43e82`) to skip all incoming attacks ([`docs/reference/specials.md`](../reference/specials.md); [SuperCombo Wiki – Ryu](https://wiki.supercombo.gg/w/Street_Fighter_(1987)/Ryu)).
    - **Multi-Hit Glitches / OHKOs:** When multi-hit specials or certain collisions interact with moving/airborne hurtboxes (such as Lee's "Scuttle Cancel" walk state or deep *Shoryuken* collisions), multiple hits register without knockdown immunity, producing 80%–100% One-Hit KOs ([SuperCombo Wiki – System](https://wiki.supercombo.gg/w/Street_Fighter_(1987)/System)).

#### B. Severe Stage-Indexed Chip Damage (Zero Against Thailand Bosses)
- **ROM Mechanism ([`docs/reference/hadoken.md`](../reference/hadoken.md), [`docs/reference/specials.md`](../reference/specials.md)):**
  - In 2P Versus mode (`game mode 1`), blocking a *Hadoken* (`0x43466`) or *Tatsumaki* (`0x4601a`) deals **8 HP (16.7% of max health)** in chip damage—equal to an unblocked Heavy Punch.
  - In 1P mode (`game mode 2`), chip damage is determined by stage index (`0xffc003`) via tables `0x4340a` (*Hadoken*) and `0x45fa8` (*Tatsumaki*): `8, 8, 16, 16, 0, 0xff`.
    - **Stages 0–1:** **8 HP (16.7%)** chip damage per blocked special.
    - **Stages 2–3:** **16 HP (33.3%—one third of a life bar)** chip damage per blocked special.
    - **Stage 4 (Thailand – Adon & Sagat):** **`0` HP** chip damage (only the block sound plays; Adon and Sagat cannot be chipped by specials).

---

### 1.3 Movement, Jump Scripts, Broken Blocking Rules, and Slowdown

#### A. Airborne "Hop Walk" and 16-Pixel Back-Walk Step
- **Community Criticism:** Rather than gliding smoothly, Ryu and Ken move via a stiff, bobbing "hop walk" that locks out crouching and attacking for stretches of frames ([Hardcore Gaming 101](https://www.hardcoregaming101.net/fuuun-shourinken/); [SuperCombo Wiki – System](https://wiki.supercombo.gg/w/Street_Fighter_(1987)/System)).
- **ROM Mechanism ([`docs/reference/movement.md`](../reference/movement.md), [`docs/reference/posture-dispatch.md`](../reference/posture-dispatch.md), [`docs/reference/standing-attacks.md`](../reference/standing-attacks.md)):**
  - Walk cycles are 17-frame loops where vertical sprite coordinates (`sy`) rise and fall (`88 → 84 → 88`), flagging the fighter as airborne mid-step and deferring crouch/jump/attack inputs during the 8 gated `0x91` frames of every 10-frame step.
  - Holding `Back` immediately shifts the fighter **16 pixels away** on the first frame of the back-walk, causing a point-blank `Back + Light Kick` to whiff unless `Back` is pressed on the exact button-release frame (`sf-normals-04` vs. `sf-normals-05` in [`docs/reference/standing-attacks.md`](../reference/standing-attacks.md)).

#### B. 9-Frame Pre-Jump Delay & Apex-Only Jump Attacks
- **Community Criticism:** Jumps feel sluggish to leave the ground, and pressing an attack button late in a jump (for a deep jump-in) fails to produce an aerial attack.
- **ROM Mechanism ([`docs/reference/jump.md`](../reference/jump.md), [`docs/reference/jump-attacks.md`](../reference/jump-attacks.md), `0x452e0`):**
  - Neutral jumps take 40 frames and diagonal jumps take 52 frames, beginning with a **9-frame grounded anticipation crouch** before the character leaves the floor.
  - Releasing a button during the jump ascent buffers the strength (`0x42b34`), but the jump stream **only checks for and launches a jump attack at one single frame: the exact apex (`0x452e0` for Ryu, `0x453b6` for Ken)**. If a player releases the button even 1 frame after the apex while descending, no aerial attack occurs; Instead, the buffered strength is copied back to the ground latch upon landing (`0x42c4a`) and fires a grounded normal after landing.

#### C. Broken Blocking Rules & The Crouching-Block vs. Jump-Attack Bug
- **Community Criticism:** Blocking feels inconsistent, with fighters walking backward into low attacks or getting hit while visibly holding guard ([SuperCombo Wiki – System](https://wiki.supercombo.gg/w/Street_Fighter_(1987)/System)).
- **ROM Mechanism ([`docs/reference/blocking.md`](../reference/blocking.md)):**
  - Block moves (`move 4` standing block, `move 8` crouching block) only start if the opponent's entity state (`+9`) is *already* in an attack state (`0xc1` jump, `0xc2` standing, `0xc4` crouching).
  - **Standing Guard vs. Lows (`sf-blocks-01`, `sf-blocks-03`):** Holding `Back` only checks for `0xc2` (standing) or `0xc1` (jump). Against a crouching attack (`0xc4`), standing guard never activates—the defender simply walks backward and gets hit.
  - **Crouching Guard vs. Jump Attacks Bug (`sf-blocks-05`):** When the opponent performs a jumping attack (`+9 = 0xc1`), holding `Down-Back` (`0x408da`) recognizes `0xc1` and puts the defender into **Crouching Block (`move 8`)**. However, the jump attack's hit function (`0x45e10` / `0x4638e`) **only honors Standing Block (`move 4`)**! The defender visually enters crouching block and is hit for full damage anyway ([`docs/reference/blocking.md`](../reference/blocking.md)).
  - **"Omniblock" Hurtbox Disappearance:** On CPU characters Lee and Adon, entering block removes the upper half of their hurtbox for 16 frames, causing fireballs and high attacks to pass harmlessly through them ([SuperCombo Wiki – System](https://wiki.supercombo.gg/w/Street_Fighter_(1987)/System)).

#### D. 90-Frame Slow-Motion Crawl on Every Unblocked Projectile Hit
- **ROM Mechanism ([`docs/reference/hadoken.md`](../reference/hadoken.md), `Scene Event 9` / `0x3ff2c`):**
  - Every unblocked *Hadoken* hit (`0x433ba`) triggers **Scene Event 9 (`0x3ff2c`)**, which reuses the KO slow-motion mechanism (`0xffc00b`). For **90 steps (60 + 30)**, the fight loop's speed sleep (`0x3b5aa`) sleeps **6 frames per step instead of 1**, dropping the entire game to 1/6th speed (10 FPS) on every landed fireball ([SuperCombo Wiki – Ryu](https://wiki.supercombo.gg/w/Street_Fighter_(1987)/Ryu); [`docs/reference/hadoken.md`](../reference/hadoken.md)).

#### E. Invulnerable Hitstun (No Combos) & No Special Cancels
- **ROM Mechanism ([`docs/reference/punch.md`](../reference/punch.md), `0x4451a`):**
  - Throughout the 26-frame hitstun reaction stream (`move 0x30`), the defender's invulnerability flag is set (`+0xf = 0x08`) and the defender is pushed back `+0x20` (32 pixels). Because stunned fighters are invulnerable until returning to neutral stance (and busy gating `0x403d4` + latch clearing `0x4050c` prevent canceling normals into specials), traditional multi-hit combos do not exist.

---

### 1.4 Roster Restrictions and Hidden Ryu vs. Ken Asymmetries

1. **Only 2 Playable Characters (Out of 12 in the Game):**
   - Due to memory and schedule limits, only Ryu (1P) and Ken (2P) were made playable, while the 10 opponents across 5 countries (**Retsu, Geki, Joe, Mike, Lee, Gen, Birdie, Eagle, Adon, Sagat**) are CPU-only ([Polygon Oral History](https://www.polygon.com/2020/7/7/21270906/street-fighter-1-oral-history-takashi-nishiyama)).
2. **Ken Cannot Be Picked Directly in 1P Mode (`Game Mode 4`):**
   - Pressing 1P Start always launches Ryu (`game mode 2`). Pressing 2P Start requires 2 credits (`0x3dfda`) and starts a 2P Versus match (`game mode 1`). The only way to play the single-player campaign as Ken (`game mode 4`) is for Player 2 to join in, defeat Player 1 in Versus, and continue into the CPU ladder (`0x3eeac`, [`docs/planning/arcade-flow-batch.md`](../planning/arcade-flow-batch.md)).
3. **Hidden Frame-Data & Code Asymmetries Favoring Ryu Over Ken:**
   - Despite being considered palette swaps, SuperCombo Wiki frame analysis and [`docs/reference/blocking.md`](../reference/blocking.md) document significant differences:
     - **Shoryuken:** Ryu's is 100% invincible from frame 1; Ken's has 4 frames of startup vulnerability ([SuperCombo Wiki – Ryu](https://wiki.supercombo.gg/w/Street_Fighter_(1987)/Ryu); [SuperCombo Wiki – Ken](https://wiki.supercombo.gg/w/Street_Fighter_(1987)/Ken)).
     - **Hadoken Recovery:** Ryu's recovers in `7f` on block / `16f` on whiff; Ken's takes `19f` on block / `30f` on whiff.
     - **Standing Medium Kick:** Ryu's is `21f` total vs. Ken's `26f`.
     - **Block Tracker Reset:** Facing left (`0x40714`), P1 (Ryu) resets the *Hadoken*/*Tatsumaki* trackers and latches on block; facing right (`0x40626`), P2 (Ken) does not ([`docs/reference/blocking.md`](../reference/blocking.md)).

---

### 1.5 CPU Input Reading, Sagat's Static Difficulty, and Progression Quirks

1. **Frame-1 CPU Input Reading (`0x4571a`, [`docs/reference/cpu-follow-up.md`](../reference/cpu-follow-up.md)):**
   - At the end of each CPU move/walk step (`0x45630`), the CPU React branch (`0x4571a`) directly reads P1's entity state (`+9 bit 6`, set on frame 1 of any P1 attack), looks up the exact pixel reach of P1's move ID (`+0xa`) in ROM table `0x146aa`, and immediately dispatches a counter or block from `0xffc0e4` / `0xffc164`.
2. **Unblockable CPU Attacks (`0x468de`, [`docs/reference/cpu-follow-up.md`](../reference/cpu-follow-up.md)):**
   - Even Stage 1 opponent Retsu has an attack (`move 0x18`, hit function `0x468de`) that omits the block check (`+9 == 0x22`), making it 100% unblockable.
3. **Sagat Ignores the Difficulty DIP Switch ([`docs/reference/ai-tables.md`](../reference/ai-tables.md)):**
   - While Opponents 0–3 (Retsu, Geki, Joe, Mike) have 8 distinct AI table images across the 4 Difficulty DIP settings (`D1`) and 2 Country variants (`D2`), **Opponent 9 (Sagat) has only 1 distinct AI table image across all 8 settings**—all rows in his ROM block (`0x1c048 + 9 * 0x300e`) are identical. Setting the cabinet to "Easy" has zero effect on Sagat's AI.
   - Additionally, at Difficulty `D1 = 0` (Normal), routine `0x448ee` skips the country variant (`D2`) offset for move tables ([`docs/reference/ai-tables.md`](../reference/ai-tables.md)).
4. **Fast "100-Second" Timer (~31.7 Real Seconds) & Ties Count as Losses:**
   - The round timer (`0x3f4e2`) ticks every **19 scheduler passes (`0x12`)** rather than every 60 frames, expiring a "100" timer in **1,900 frames (~31.67 seconds)** ([`docs/reference/round-timer.md`](../reference/round-timer.md)).
   - If a 1P match ends tied after 3 rounds, `0x3eefa` awards the win to the CPU, forcing a Continue/Game Over ([`docs/planning/arcade-flow-batch.md`](../planning/arcade-flow-batch.md)).

---

## Part II: What Fans Have Wanted Out of the Original Game (Wishlists, ROM Hacks, and Remakes)

Because official re-releases (*Street Fighter 30th Anniversary Collection*, *Capcom Arcade 2nd Stadium*, *SF6* Battle Hub) only ship raw emulation of the 1987 ROM, fans, modders, and ROM hackers have spent decades building or requesting improvements. For this project, these represent candidate **optional improvements** layered on top of the faithful **port**.

### 2.1 Playable 12-Character Roster: From *SF1++* to M.U.G.E.N / Ikemen GO Remakes

#### A. Yumeji's *SF1++* (`sf1++`, 2014 Arcade ROM Hack)
- Created by Japanese ROM hacker **Yumeji**, ***SF1++*** patches the 1987 arcade ROM so all 10 CPU opponents can be selected in 1P and 2P modes by holding a direction + attack button while pressing Start (e.g., `Left + LP + Start` for Retsu, `Up + MK + Start` for Sagat) ([SuperCombo Wiki – *SF1++*](https://wiki.supercombo.gg/w/Street_Fighter_(1987)/SF1%2B%2B); [SF1Cord Guide](https://fightingstreet.neocities.org/)).
- **Incomplete CPU State Machines Exposed by *SF1++*:** Because the 10 bosses were only coded for CPU routines in the 1987 ROM, making them playable directly exposes missing states ([SuperCombo Wiki – *SF1++*](https://wiki.supercombo.gg/w/Street_Fighter_(1987)/SF1%2B%2B)):
  - **Sagat** cannot jump or crouch at all ([SuperCombo Wiki – Sagat](https://wiki.supercombo.gg/w/Street_Fighter_(1987)/Sagat)).
  - **Mike** has no jumping attacks and only four grounded/crouching punch normals ([SuperCombo Wiki – Mike](https://wiki.supercombo.gg/w/Street_Fighter_(1987)/Mike)).
  - **Geki** has no standard block; holding Back triggers his teleport vanish, which has vulnerable startup frames ([SuperCombo Wiki – Geki](https://wiki.supercombo.gg/w/Street_Fighter_(1987)/Geki)).
  - **Air Attacks for CPU Characters:** None of the 10 CPU characters use Ryu/Ken's apex jump-attack buffer; players must hold `↖`, `↗`, or `↙` + **Punch** mid-air to trigger their aerial attacks ([SuperCombo Wiki – *SF1++*](https://wiki.supercombo.gg/w/Street_Fighter_(1987)/SF1%2B%2B)).

#### B. Fan Remakes That Flesh Out the 10 Bosses (*Street Fighter One* M.U.G.E.N & Ikemen GO)
- In ***Street Fighter One*** (by **The Valkyrie Project**, **Falchion22**, and **Shadow Blade**) and **Pilgrim's *Street Fighter 1* Ikemen GO Remake**, modders give all 12 fighters complete *SFII*/*Alpha*-style movesets ([GameBanana – *Street Fighter One*](https://gamebanana.com/mods/382658); [LaunchBox DB – *Street Fighter One*](https://gamesdb.launchbox-app.com/games/details/166560-street-fighter-one); [GameJolt – Ikemen GO Remake](https://gamejolt.com/games/sf2scie/831224)):
  - Adding missing crouch/jump states for **Sagat** and air normals for **Mike**.
  - Giving **Geki** a proper guard alongside command shuriken (`QCF+P`) and teleport (`↓,↓+P/K`) specials.
  - Turning CPU signature moves—**Retsu's** leaping *Hakai Geri*, **Joe's** `"Power!"` spin kick, **Lee's** Bajiquan lunges, **Gen's** *Hyakurenko* slaps and dive kicks, **Birdie's** *Bull Head* headbutt, **Eagle's** escrima combos, and **Adon's** *Jaguar Kick*—into standard quarter-circle and charge specials.

### 2.2 Optional Control & Input Improvements ("SF2-Style" Input Mode)
The single most requested gameplay improvement across Reddit, Steam, and GameFAQs is an optional **"Modern / SF2 Input Mode"** that fixes the 1987 input parser while keeping the original visuals and stage progression:
1. **Positive-Edge + Negative-Edge Button Registration:** Firing normal and special attacks immediately on **button press** (1-frame response) rather than waiting for button release in `0x3b3ae`, while keeping optional negative edge for special moves ([SuperCombo Wiki – Controls](https://wiki.supercombo.gg/w/Street_Fighter_(1987)/Controls); [Infil's Glossary – Negative Edge](https://glossary.infil.net/?t=Negative%20Edge)).
2. **Expanded Special-Move Window & Release Leniency:** Expanding the `0x12` (18-frame) command tracker limit (`0x40ae4`) and allowing the completed `done = 0xff` state to persist for a short buffer window (e.g., 6–10 frames) after the joystick leaves the final direction.
3. **Bypassing Walk-Cycle Busy Gating (`0x91`) for Specials & Crouches:** Allowing special moves, blocks, crouches, and jumps to interrupt any frame of the walk cycle rather than waiting up to 8 frames for an interruptible `0x92` frame.
4. **Apex-Independent Jump Attacks:** Allowing aerial normals to fire at any point during jump ascent or descent rather than exclusively at the apex call (`0x452e0` / `0x453b6`).

### 2.3 Optional Gameplay & Balance Fixes
1. **Normalizing Ryu vs. Ken Frame Data:** Giving players the option to make Ryu and Ken truly symmetrical (or giving Ryu's *Shoryuken* Ken's 4-frame startup vulnerability and making *Shoryuken* blockable), addressing the imbalance that forces the **SF1Cord** competitive community to ban Ryu in standard *SF1++* tournaments ([SuperCombo Wiki – Ryu](https://wiki.supercombo.gg/w/Street_Fighter_(1987)/Ryu); [SF1Cord Formats](https://fightingstreet.neocities.org/); [Challonge – *Adonarchy*](https://challonge.com/ko/q6bg4phg)).
2. **Damage & Chip Scaling Option:** Reducing special move damage from 20 HP (41.7%) and stage chip damage from 8–16 HP (16.7%–33.3%) to balanced ratios so rounds do not end in two fireballs.
3. **Fixing the Crouching-Guard vs. Jump-Attack Bug (`sf-blocks-05`) & Standing-Guard Walk-Back:** Making Standing Block (`move 4`) and Crouching Block (`move 8`) follow intuitive high/low blocking rules (standing blocks high/jump; crouching blocks low; no involuntary walk-back into low attacks when holding Back).
4. **Disabling the 90-Frame Fireball Slowdown (`Scene Event 9`):** Making the 1.5-second 10 FPS slow-motion crawl on unblocked *Hadoken* hits (`0x3ff2c`) toggleable so matches maintain a smooth 60 FPS pace.

### 2.4 Modes, Training Tools, Audio, and Netplay Wishlist
1. **Direct 1P Ken Selection & Full Character Select Screen:** Allowing Player 1 to select Ken (or any unlocked CPU fighter) directly from the start menu without requiring a 2-credit Versus win (`game mode 4`), and exposing the 4-Country Select DIP option (`Japan, USA, England, China`) in the UI.
2. **Training Mode with Hitbox & Frame Viewer:**
   - Reviews of *Street Fighter 30th Anniversary Collection* (2018) heavily criticized the omission of Training Mode for *Street Fighter* (1987) ([GameFAQs Review](https://gamefaqs.gamespot.com/pc/230139-street-fighter-30th-anniversary-collection/reviews/166449)), and community Lua scripts like [`fbneo-training-mode`](https://github.com/peon2/fbneo-training-mode/blob/master/fbneo-training-mode.lua) only support CPS-1/2/3 games.
   - A native PC port is uniquely positioned to display the original game's **dual hurtboxes** (outer **loose box** `depth 2` vs. inner **tight/crit box** `depth 1`, attack box, and invulnerability state `+0xf bit 3`), command-tracker stages (`+0x16`, `+0x19`, `+0x1c`), and walk-gate states (`0x91` vs. `0x92`).
3. **Online Rollback Netplay:** Both *30th Anniversary Collection* and *Capcom Arcade 2nd Stadium* excluded online play for the 1987 original ([Nintendo Life Review](https://www.nintendolife.com/reviews/switch-eshop/capcom-arcade-2nd-stadium)), leaving Fightcade 2 as the only online venue.
4. **Optional PC Engine CD (*Fighting Street*) Arranged Soundtrack:** While the 1988 PC Engine CD port *Fighting Street* had inferior visuals and 2-button controls, its **Red Book CD arranged soundtrack** is widely loved by fans and used in nearly every M.U.G.E.N/Ikemen remake as an audio option alongside the arcade YM2151 + MSM5205 audio ([Nintendo Life – *Fighting Street* Review](https://www.nintendolife.com/reviews/2009/11/fighting_street_virtual_console); [GameBanana – *Street Fighter One*](https://gamebanana.com/mods/382658)).
5. **Renewed Lore Interest via *Street Fighter 6*:** The return of **Retsu** as a 3D NPC/sub-boss in *SF6* World Tour ("Monk Without a Cause") and the inclusion of the 1987 arcade cabinet in the *SF6* Battle Hub Game Center sparked widespread community interest in revisiting the 1987 tournament and its "forgotten four" (**Geki, Joe, Mike, Lee**) ([Street Fighter Wiki – Retsu](https://streetfighter.fandom.com/wiki/Retsu)).

---

## Part III: What Fans Have Wanted Out of the *Street Fighter* Series as a Whole & Recurring Criticisms (*SFII* – *SF6*)

Zooming out to the entire franchise across *Street Fighter II*, *Alpha*, *III*, *IV*, *V*, and *6*, FGC discourse centers on six recurring tensions.

### 3.1 Core Combat Philosophy: "Honest Footsies" vs. "Neutral Skips" & Comeback Robbery

The defining philosophical divide in the *Street Fighter* community is between **ground-based space control ("footsies")**—codified in Maj's [*Sonic Hurricane Footsies Handbook*](https://sonichurricane.com/?page_id=1702) ([Chapter 1](https://sonichurricane.com/?p=688))—and **system-heavy comeback or "neutral-skip" mechanics**:

1. ***Street Fighter Alpha 3* & *3rd Strike*:**
   - In *Alpha 3*, purists criticized **V-Ism** for enabling 50%-meter crouch-cancel infinites and guard-crush loops that bypassed neutral.
   - In *SFIII: 3rd Strike*, while the **Parry** is iconic, critics noted that a 10-frame tap parry invalidates traditional fireball zoning and mid-range pokes, making pure zoners (like Remy) bottom-tier ([Sonic Hurricane Footsies Handbook](https://sonichurricane.com/?page_id=1702)).
2. ***Street Fighter IV* (Revenge Ultras & USFIV Elena Healing):**
   - Because the **Revenge Gauge** filled exclusively by taking damage, players criticized "Ultra robbery" (losing 45–55% health to a single FADC into Ultra). In *USFIV*, **Elena's Healing (Ultra 2)** allowed defensive players to heal twice per round, culminating in spectator backlash during Evo 2015 Top 8.
3. ***Street Fighter V* (V-Trigger Robbery & Crush Counters):**
   - Tying V-Gauge to damage taken and giving top tiers plus-on-block V-Trigger installs (Balrog, Urien, Abigail, G, Seth) earned *SFV* the nickname *"Robbery Fighter V,"* while heavy **Crush Counters** encouraged fishing in neutral.
4. ***Street Fighter 6* (Universal Drive Rush & Volatility Debates):**
   - While *SF6* removed health-gated comeback meters by starting every round at full Drive Gauge, its **1-bar Raw Parry Drive Rush** and **3-bar Cancel Drive Rush** (+4 frame advantage on block/hit with sliding inertia) sparked fierce debate over "skipping neutral":
     - [`r/Fighters`: "[SF6] Drive rush slide/inertia does a lot of heavy lifting in neutral, and should be nerfed."](https://www.reddit.com/r/Fighters/comments/183m4u1/sf6_drive_rush_slideinertia_does_a_lot_of_heavy/)
     - [`r/StreetFighter`: "What do people mean when they say a character 'skips the neutral phase?'"](https://www.reddit.com/r/StreetFighter/comments/16wggx3/what_do_people_mean_when_they_say_a_character/)
     - [`r/StreetFighter`: "Infilament's thoughts on the topic of 'neutral skipping'"](https://www.reddit.com/r/StreetFighter/comments/18oey1n/infilaments_thoughts_on_the_topic_of_neutral/)
   - Two-time Capcom Cup champion **MenaRD** remarked in an interview with Brian_F: *"The game is extremely fun, but competitively is torture. This is the most random SF I've ever played"* ([`r/StreetFighter` thread](https://www.reddit.com/r/StreetFighter/comments/18anew5/menard_with_brianf_the_game_is_extremely_fun_but/)) and later argued that removing or nerfing Drive Rush would improve competitive play ([`r/StreetFighter` thread](https://www.reddit.com/r/StreetFighter/comments/1i9v5g8/menard_thinks_removing_drive_rush_makes_sf6_a_better_game/)). Evo 2024 champion **Punk** raised similar criticisms of raw Drive Rush and throw loops ([`r/StreetFighter` thread](https://www.reddit.com/r/StreetFighter/comments/18hky6b/punks_honest_thoughts_on_state_of_sf6/)).

### 3.2 Defensive Options vs. Overwhelming Corner Offense & Throw Loops

1. **Vortex (*SFIV*) vs. Meterless Corner Throw Loops (*SF6*):**
   - Before Delayed Wakeup in *USFIV*, knockdown-heavy characters (Akuma, Ibuki, Cammy) locked defenders into ambiguous 50/50 "Vortex" loops.
   - In *SF6*, because wakeup **Drive Parry** defeats high/low/left/right strike mixups, Capcom gave most of the cast **walk-up meterless throw loops in the corner** paired with **coast-to-coast corner carry** off a single Drive Rush conversion. This creates a binary 3-guess Rock-Paper-Scissors (Throw vs. Shimmy vs. Reversal/Delay Tech) that remains one of the loudest mechanical complaints in modern *Street Fighter*:
     - [`r/StreetFighter`: "What makes sf6 throw loops so notable?"](https://www.reddit.com/r/StreetFighter/comments/1fjg00f/what_makes_sf6_throw_loops_so_notable/)
     - [`r/StreetFighter`: "What's special about throw loops?"](https://www.reddit.com/r/StreetFighter/comments/185vovt/whats_special_about_throw_loops/)
     - [`r/StreetFighter`: "Throw loops"](https://www.reddit.com/r/StreetFighter/comments/18xue06/throw_loops/)
     - [`r/StreetFighter`: Sajam analysis – "Why Throws Are Designed the Way that They Are in Street Fighter 6"](https://www.reddit.com/r/StreetFighter/comments/14wtr7r/why_throws_sre_designed_the_way_that_they_are_in/)
2. **Chip Deaths & Burnout:**
   - *SFV* eliminated special-move chip deaths (requiring Critical Art to chip KO), a change widely praised for preventing cheap fireball checkmates. *SF6* tied special chip damage, +4 extra blockstun frames, and corner Drive Impact wall-stun checkmates to the **Burnout** state, rewarding meter management while creating tense corner sequences.

### 3.3 Execution Barriers, Modern Controls, and Input Latency

1. **From 1-Frame Links (*SFIV*) to 3–4 Frame Buffers (*SFV* / *SF6*):**
   - *SFIV* had zero combo buffer, requiring 1-frame links and hardware tricks like **Plinking** (Priority Linking). The shift to a **3-frame buffer in *SFV*** and **4-frame buffer in *SF6*** was overwhelmingly welcomed for letting players focus on spacing and decisions rather than 16.6ms finger timing.
2. **Classic vs. Modern Controls in *SF6*:**
   - Tournament-legal **Modern Controls** (removing motion inputs via a Special button + direction at a 20% damage penalty and reduced normal moveset) succeeded massively in onboarding new players, especially in Japan ([`r/Fighters` discussion](https://www.reddit.com/r/Fighters/comments/1ed8v99/why_is_it_only_sf6_modern_control_that_take_off/)).
   - However, Classic players criticize Modern's ability to fire **instant 1-frame reaction Supers and anti-air DPs** in neutral or burnout without buffering motions:
     - [`r/StreetFighter`: "Change my mind: Modern super arts in neutral should have a few frames of delay"](https://www.reddit.com/r/StreetFighter/comments/14dd8p3/change_my_mind_modern_super_arts_in_neutral_should/)
     - [`r/StreetFighter`: "modern controls should have to manually input supers"](https://www.reddit.com/r/StreetFighter/comments/174q0do/modern_controls_should_have_to_manually_input/)
3. **Input Lag Controversies & Screen-Freeze "Eaten Inputs":**
   - ***SFV* "8 Frames of Lag" (2016–2018):** Hardware researcher Loïc **"WydD"** Petit ([inputlag.science](https://inputlag.science)) proved launch *SFV* had **8 frames (~133ms) of input delay**, crippling reaction anti-airs and whiff punishing ([Core-A Gaming Analysis](https://www.youtube.com/watch?v=27bM3KZNkc4); [EventHubs report on producer Koichi Sugiyama's remarks](https://www.eventhubs.com/news/2016/jun/15/street-fighter-5s-eight-frames-input-delay-intentional-design-choice-says-producer-koichi-sugiyama/); [`r/StreetFighter` thread](https://www.reddit.com/r/StreetFighter/comments/4o7q20/street_fighter_5_intentional_lag_was_copied_from_tekken/)). Capcom eventually patched *SFV* down to ~4.4f in 2018.
   - ***SF6* Platform Latency & Drive Rush Freeze Eating Inputs:** While *SF6* achieves ~3.5–4f latency on PC with Input Delay Reduction ([One Frame Ahead Substack](https://oneframeahead.substack.com/p/street-fighter-6-exploring-platform); [`r/StreetFighter` thread](https://www.reddit.com/r/StreetFighter/comments/1hd02m0/recent_input_lag_test_between_consoles_and_pc/)), players frequently complain that the **10-frame Drive Rush screen freeze "eats inputs"** during motion buffers and wakeup reversals:
     - [`r/StreetFighter`: "SF6 eating inputs - anyone else have this happen?"](https://www.reddit.com/r/StreetFighter/comments/13ns189/sf6_eating_inputs_anyone_else_have_this_happen/)
     - [`r/StreetFighter`: "Street Fighter 6 still has an 'my inputs got eaten' problem and I'm tired of pretending it doesn't exist."](https://www.reddit.com/r/StreetFighter/comments/1lglq09/street_fighter_6_still_has_an_my_inputs_got_eaten/)
     - [`r/StreetFighter`: "Can some one explain why this happened?"](https://www.reddit.com/r/StreetFighter/comments/1ba3xfl/can_some_one_explain_why_this_happened/)

### 3.4 Roster Demands, Character Popularity Polls, and Timeline Progression

1. **The Scar of *SFIII: New Generation* (1997) & "World Warrior Fatigue":**
   - Because cutting the *SFII* cast (save Ryu and Ken) hurt *SFIII: New Generation* commercially ([`r/StreetFighter` retrospective](https://www.reddit.com/r/StreetFighter/comments/wp1byl/what_do_thought_of_sf3_new_generation_in_retrospect/)), Capcom has included all 8 original 1991 *SFII* World Warriors in *SFIV*, *SFV*, and *SF6*'s base roster. This divides fans between those who feel the World Warriors are essential brand icons and those frustrated that low-pick-rate characters (Honda, Blanka, Dhalsim) occupy base slots over fan favorites ([`r/StreetFighter` debate 1](https://www.reddit.com/r/StreetFighter/comments/11my98d/do_you_think_the_whole_sf2_cast_should_be_included/), [`r/StreetFighter` debate 2](https://www.reddit.com/r/StreetFighter/comments/zy7m2o/unpopular_opinion_the_world_warriors_are_street/)).
2. **Official & Community Character Polls:**
   - **Capcom's Official 30th Anniversary Worldwide Poll (>150,000 votes):** **#1 Sakura**, **#2 Makoto**, **#3 Juri**, **#4 Q**, **#5 Cody**, **#9 Ryu**, **#14 Dudley**, **#16 Skullomania** ([Official Capcom Poll Graphic](https://game.capcom.com/cfn/sfv/as/img/vote/complete/mv_en_sp.jpg?h=d10cadee87158801067266c0a74dd8fb)).
   - **Recurring Community DLC Poll Leaders (`r/StreetFighter`):** **Makoto**, **Dudley**, **Sakura**, **Cody**, **Guy**, **C. Viper**, **Fei Long**, **Q**, **Sean**, **Alex**, and *SF1*/*Alpha* veterans **Eagle**, **Adon**, **Gen**, and **Sodom**:
     - [`r/StreetFighter`: "SF6 Season 3 Wishlist Community Poll Results"](https://www.reddit.com/r/StreetFighter/comments/1ekq116/sf6_season_3_wishlist_community_poll_results/)
     - [`r/StreetFighter`: "Most Wanted Future DLC Characters !"](https://www.reddit.com/r/StreetFighter/comments/14scv96/most_wanted_future_dlc_characters_i_spent_too_much/)
     - [`r/StreetFighter`: "SF6 Most Wanted DLC Characters From SF3 & SF4!"](https://www.reddit.com/r/StreetFighter/comments/vhwi2v/sf6_most_wanted_dlc_characters_from_sf3_sf4/)
     - [`r/StreetFighter`: "r/SF's DLC Character picks, based on poll data"](https://www.reddit.com/r/StreetFighter/comments/wuzuxc/rsfs_dlc_character_picks_based_on_poll_data/)
     - [`r/StreetFighter`: "Results of the roster poll"](https://www.reddit.com/r/StreetFighter/comments/16axl29/results_of_the_roster_poll/)
3. **Advancing the Timeline Past *3rd Strike*:**
   - After 24 years of prequels (*SFIV* and *SFV* wedged before *SFIII*), fans celebrated *SF6* finally advancing the canon past *3rd Strike* (showing a matured Ryu, Chun-Li mentoring Li-Fen, and Ken's Nayshall storyline), though lore fans criticized Capcom for immediately resurrecting **M. Bison** in Year 2 DLC and reducing secondary characters to one-note food/hobby tropes.

### 3.5 Single-Player Content, Aesthetics, and Music

1. **The *SFV* 2016 Launch Backlash vs. *Alpha 3* & *SF6* World Tour:**
   - *SFV*'s February 2016 launch without an Arcade Mode, VS CPU mode, or Combo Trials—and with basic costume colors (3–10) locked behind a punishing 30–100 stage RNG **Survival Mode** that wiped progress on server disconnect—remains a cautionary tale in the FGC ([Forbes](https://www.forbes.com/sites/insertcoin/2016/02/18/the-5-biggest-problems-with-street-fighter-v/); [Ryan Markel Survival Mode Analysis](https://www.ryanmarkel.com/blog/2016/2/18/lets-talk-about-street-fighter-v-s-survival-mode); [`r/StreetFighter` retrospective](https://www.reddit.com/r/StreetFighter/comments/1c31hsz/how-bad_was_street_fighter_5_back_at_or_original_2016_launch/); [GameFAQs thread](https://gamefaqs.gamespot.com/boards/895976-street-fighter-v/80193181)).
   - Conversely, ***Street Fighter Alpha 3*'s World Tour** and ***Street Fighter 6*'s World Tour** (which teaches fighting game fundamentals through open-zone RPG encounters and lets players unlock classic **Outfit 2** skins for free via Master Bond) are hailed as the franchise's single-player gold standards ([`r/StreetFighter` newcomer review](https://www.reddit.com/r/StreetFighter/comments/1le553q/my_review_of_world_tour_as_an_fgc_newcomer/); [`r/StreetFighter` critique of late-game RPG grind](https://www.reddit.com/r/StreetFighter/comments/1f54k5u/i_hate_the_world_tour_mode_in_sf6_its_ugly_not_fun/)).
2. **Aesthetics & The *SF6* Soundtrack Controversy:**
   - *SF6*'s RE Engine visuals and classic/modern costume designs resolved years of mockery over *SFV*'s "clay/banana hair" models (and *SFV* broadcast censorship controversies at Evo 2016/2017: [Kotaku](https://kotaku.com/espn-makes-street-fighter-player-change-characters-thon-1797034458); [TechRaptor](https://techraptor.net/gaming/news/broadcast-standards-force-street-fighter-v-costume-change-at-evo-2017); [`r/StreetFighter`](https://www.reddit.com/r/StreetFighter/comments/6nknz4/sfv_evo_top_8_censorship/)).
   - However, *SF6*'s ambient hip-hop soundtrack drew heavy criticism from fans who missed the iconic melodic hooks of *SFII* and *3rd Strike*, prompting Capcom to add a **Jukebox** feature to equip classic series BGM ([`r/StreetFighter` thread 1](https://www.reddit.com/r/StreetFighter/comments/10ube37/honest_thoughts_on_the_sf6_soundtrack_so_far/), [`r/StreetFighter` thread 2](https://www.reddit.com/r/StreetFighter/comments/1101968/can_someone_explain_to_me_why_all_the_dislike_for/), [`r/StreetFighter` thread 3](https://www.reddit.com/r/StreetFighter/comments/19b8v8h/dont_get_the_hate_for_sf6s_soundtrack/), [Steam Discussions](https://steamcommunity.com/app/1364780/discussions/0/3882722254347702816/)).

### 3.6 Technical Infrastructure, Training Mode Standards, and Monetization

1. **Netcode, Crossplay, Wi-Fi Indicators, and Region Locks:**
   - After *SFV*'s infamous September 2016 `Capcom.sys` kernel rootkit patch ([PC Gamer](https://www.pcgamer.com/capcom-promises-street-fighter-5-rollback-after-rootkit-discovered-in-the-latest-update/); [PC Gamer Removal Tool](https://www.pcgamer.com/capcom-releases-tool-to-fix-security-risk-in-recent-street-fighter-v-patch/); [IGN](https://www.ign.com/articles/street-fighter-v-update-adds-potentially-dangerous-files-for-pc-players)) and one-sided rollback clock-drift bug (fixed by community modder **Altimor** in Jan 2020 before Capcom's official patch: [IGN](https://www.ign.com/articles/street-fighter-5s-pc-netcode-problem-reportedly-fixed-by-modder)), *SF6* delivered industry-leading GGPO-grade rollback netcode, crossplay, and a **Wi-Fi vs. Wired indicator** ([`r/StreetFighter`](https://www.reddit.com/r/StreetFighter/comments/13vpv96/street_fighter_6_seems_to_have_a_wifi_indicator_now/); [TwistedVoxel](https://twistedvoxel.com/street-fighter-6-features-indicator-to-distinguish-between-online-players-on-wi-fi-wired-connections/)).
   - **Remaining Networking Criticisms:** Players want an automatic "Wired Only" matchmaking filter and the removal of **strict proximity region locks** in *SF6* that trap players in the Middle East, Turkey, South Africa, and Eastern Europe in 15-minute queues despite having 30–50ms ping to EU/Asia ([`r/StreetFighter` thread 1](https://www.reddit.com/r/StreetFighter/comments/18k83x2/street_fighter_6s_region_based_matchmaking_is_one/), [`r/StreetFighter` thread 2](https://www.reddit.com/r/StreetFighter/comments/1mfj2m6/capcom_needs_to_remove_arbitrary_region_locks_in/), [`r/StreetFighter` thread 3](https://www.reddit.com/r/StreetFighter/comments/1jj2x6p/all_i_want_for_season_3_is_for_regional/)).
2. **Gold-Standard Training Tools:**
   - *SF6*'s **60-cell Visual Frame Meter**, one-click scenario drills (anti-air, whiff punish, Drive Impact defense), and **Replay Takeover** (allowing players to take control of either character directly inside any replay for 10 seconds, added in late 2024: [`r/Fighters`](https://www.reddit.com/r/Fighters/comments/1f1n97m/2_new_features_coming_with_the_sf6_terry_update/); [MP1st](https://mp1st.com/news/street-fighter-6-update-1-015-out-now-adds-terry-dlc-replay-takeover); [Capcom Patch Notes](https://www.streetfighter.com/6/buckler/information/detail/terry_20241202)) represent the modern benchmark for fighting game training modes. The main unfulfilled request is an official **in-game Hitbox/Hurtbox Viewer**.
3. **Live-Service Monetization & Content Cadence Backlash:**
   - **Uneven Premium Currency ("Fighter Coins"):** Pricing **Outfit 3** skins at **300 FC (~$6)** while selling Fighter Coins only in **250 FC ($4.99)** or **610 FC ($11.99)** packs—forcing a $10–$12 minimum spend for a single costume and charging ~$100 for all 18 base costumes with no bundle discount—triggered widespread outrage ([IGN](https://www.ign.com/articles/street-fighter-6-fans-call-out-capcom-for-awful-monetisation-after-outfit-3-prices-cause-uproar)).
   - **$15 Avatar Skins & Battle Pass Filler:** Charging **750 FC ($15 each / ~$100 total)** for *Teenage Mutant Ninja Turtles* avatar costumes usable only in Battle Hub/World Tour (more than double the ~$7 cost of a full DLC character) and filling monthly Fighting Passes with avatar cosmetics rather than roster skins/colors drew sharp criticism ([IGN](https://www.ign.com/articles/street-fighter-6-fans-hit-out-at-15-teenage-mutant-ninja-turtles-dlc-costumes); [Eurogamer](https://www.eurogamer.net/street-fighter-6s-extortionate-teenage-mutant-ninja-turtles-items-total-80)).
   - **Slow DLC Cadence:** Releasing only **4 DLC characters per year** (with half of Year 2 occupied by guest characters Terry and Mai) and waiting a full year between costume waves remains a top community frustration.

---

## Summary: Implications for the `sf1-port` Project

In keeping with [`CONTEXT.md`](../../CONTEXT.md) and [`docs/project-brief.md`](../project-brief.md), the **port** must first preserve every quirk of the **reference version** (`sf`) to achieve verified **gameplay fidelity**, **visual fidelity**, and **audio fidelity**. Once the **complete arcade experience** is established, the community research above provides a clear, prioritized roadmap of candidate **optional improvements**:

| Area | Faithful Port Baseline (Reference Version `sf`) | Candidate Optional Improvements (Strictly Opt-In / Separate from Baseline) |
| :--- | :--- | :--- |
| **Attack Registration** | VBlank button-release emulation (`0x3b3ae`), firing on `release + 1`; 255-frame hold saturation; recovery latch drop (`0x4050c`). | Positive-edge (button press) attack registration + optional negative edge for specials; recovery input buffer. |
| **Special Move Parser** | 18-frame (`0x12`) total tracker window (`0x40ae4`); resets immediately on leaving final direction; blocked during 8 of 10 walk frames (`0x91`). | *SFII*-style directional buffer and release leniency; allow specials/crouches/jumps to interrupt `0x91` walk frames. |
| **Jumping & Blocking** | 9f pre-jump; jump attacks only fire at exact apex (`0x452e0`); standing guard fails vs. lows; crouch guard (`move 8`) gets hit by jump attacks (`sf-blocks-05`). | Allow jump attacks during ascent/descent; fix `sf-blocks-05` so standing/crouching guard follow standard high/low rules. |
| **Balance & Pacing** | 20/48 HP (41.7%) specials; unblockable frame-1 invincible Ryu *Shoryuken*; 8–16 HP stage chip; 90-frame slowdown on fireball hit (`Scene Event 9`); Sagat static AI (`1` image). | Optional Ryu/Ken symmetry & blockable *Shoryuken*; balanced special/chip damage; toggle off 90-frame fireball hit slowdown; working difficulty scaling for Sagat. |
| **Roster & Modes** | 1P Ryu only; 1P Ken requires 2P Versus win (`game mode 4`); 10 CPU bosses unplayable; 2-country select default. | Direct 1P Ken selection; 4-country select toggle; *SF1++*-style playable 12-character Versus mode with completed crouch/jump/block states. |
| **Training & Audio** | Original arcade attract/fight flow; YM2151 + MSM5205 audio synthesis matching reference latch timing. | Training overlay showing loose (`depth 2`) & tight (`depth 1`) hurtboxes, attack boxes, command trackers, and a *SF6*-style Frame Meter; optional PC Engine CD (*Fighting Street*) arranged BGM jukebox. |
