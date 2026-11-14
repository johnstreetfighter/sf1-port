# Ken moveset audit for Ultra Street Fighter IV

Source slice for [#148](https://github.com/johnstreetfighter/sf1-port/issues/148). The target is PC Steam v1.05 standard Ultra with Ken versus Ryu, using Ultra I, Ultra II and Ultra Combo Double. Port comparisons use Git base `0e336743760689ccdd192cbe4ebce5c4d638c9c4`, with Classic inputs and Era Default gauges. This pass inspected shipped regulation data and port code. The Steam executable, a paired live fight and exact cross-game displacement remain open. [Port revisions](../../crates/sf-engine/src/era.rs#L452) · [Capcom Ultra systems](https://news.capcomusa.com/lets/browse/new-battle-systems-in-ultra-sfiv)

## Regulation identity

The same unmodified-files backup used by the [Ultra Ryu audit](ryu-usfiv-moveset-audit.md) supplies Ken in `Unmodded/patch_ae2_tu2/battle/regulation/ae2_110/KEN/`. Its archive SHA-256 is `2cacd70aed9e54f17188b63577ae6cbccb4c9f7e3303911b962b4e9edfe92783`. The archive has a v1.05 replacement path, `patch_ae2_tu3/battle/regulation/ae2_111`, with no Ken member. UltraDiff maps the two regulation directories to v1.04 and v1.05, and the underlying PC extraction shows no Ken delta after v1.04. Capcom's April 21, 2015 PC announcement lists Decapre under standard Ultra and Ken only under Omega. These sources support using Ken's retained v1.04 files for the v1.05 standard balance. A live installation hash remains open. [Pinned backup](https://github.com/EternalStar/USF4-Bugfix/blob/670d47e4311a8952e3d0b35039c4bc6eccdd746b/Original%20Unmodded%20Files.zip) · [Pinned decoder and release map](https://github.com/dantarion/sf4tools/tree/e5c17132f3b6e63f66150c3a347a9e8f6e0d73a4/SFUltraDiff/src) · [Pinned Ken diff directory](https://github.com/dantarion/sf4tools/tree/e07764262003f3aa1feff1a0a5a66fc3c14efb43/characters/KEN) · [UltraDiff index](https://dantarion.github.io/sf4tools/) · [Capcom v1.05](https://news.capcomusa.com/lets/browse/usfiv-ver-1-05-patch-coming-tomorrow)

| Member | Bytes | SHA-256 |
|---|---:|---|
| `KEN.bac` | 276008 | `a356113acba6a6ffd9729ba910b289e2448de6d42e0f82516ad2c4dcf472e576` |
| `KEN.bcm` | 11367 | `6730218947559b8332034ce94ae16fb257a4af433bfba5826b099219df9aa406` |

The BCM contains 90 command rows, eight motion definitions and 21 cancel lists. The BAC has 235 decoded action names, four projectile scripts and 155 populated reaction records. BCM row order is zero-based below. Directions are facing-relative.

## Command inventory

| BCM rows | Command and source behavior |
|---|---|
| 0–14 | Close and far six standing normals. Rapid close LP, far LP and close LK have dedicated rows. Close MP uses `5MP_NEW`, far HK `5HKF_aer`. All close/far pairs use 1.2 native units. |
| 15–22 | Six crouching normals and rapid `2LP/2LK`. Crouch MP/MK use `2MP_NEW/2MK_NEW`; sweep uses `2HK_aer`. |
| 23–34 | Six neutral and six directional jump normals. Both forward and back jump have the directional normal list. |
| 35–39 | `4MK` Inazuma Kick, `6MK` Forward Step Kick, `6HK` press/hold Thunder Kick opening, HK release branch, and close `MP → HP` target combo. |
| 40–41 | Forward and backward dash, `66/44`. |
| 42–45 | `236LP/MP/HP` Hadoken and `236PP` EX Hadoken. Four distinct projectile scripts. |
| 46–49 | `214LK/MK/HK/KK` grounded Tatsumaki Senpukyaku. |
| 50–57 | `j.214LK/MK/HK/KK`. Ordinary and early forward-jump versions exist for all three normal strengths and EX. Each encoded height cutoff is 0.5. |
| 58–61 | `623LP/MP/HP/PP` Shoryuken. LP uses `SYOURYU_L_NEW`. MP, HP and EX have distinct contact sequences. |
| 62–64 | `236236LP/MP/HP` Shoryureppa. Three entries, same raw strike windows and varying geometry/motion. |
| 65–66 | `236236PPP` Shinryuken and `236236KKK` Guren Senpukyaku. U1 or U2 select restricts availability; Double admits both. |
| 67–68 | `5/6+LP+LK` Shoulder Throw and `4+LP+LK` Hell Wheel, with independent finishers. |
| 69–72 | `MP+MK` Focus maximum charge and the first two release levels, plus EX Focus. |
| 73 | Neutral `HP+HK` appeal. |
| 74–75 | Extra `UC01/02_CANCEL_DASH` rows with the Ultra motions, raw Revenge requirement 400 and cost 2000. The regular rows use 200/1000. Neither extra row belongs to a decoded cancel list or has an incoming BAC flow link. Their two dash actions route on contact to alternate Ultra scripts. Ordinary route access remains unverified. |
| 76–79 | Four alternate down actions, both orientations and fall directions. Their BCM rows encode `HP+MK`; Capcom describes delayed standing using any two buttons at eligible hard knockdown landing. The four records leave the complete input and eligibility mechanism open. |
| 80–85 | Red Focus maximum `LP+MP+MK` repeated over four identical move-input/script routes, and two release entries. |
| 86–89 | Four EX Red Focus aliases, all mapping to `SUPER_FOCUS_EX`. |

Each EX special costs 250 Super units, EX Focus and neutral Red Focus 500, EX Red Focus 750, and Shoryureppa 1000. The documented Revenge system unlocks at half gauge and uses the available gauge on activation. Capcom places Ken in the 75% damage tier for Double. [Capcom system documentation](https://news.capcomusa.com/lets/browse/new-battle-systems-in-ultra-sfiv) · [Capcom delayed standing and Double](https://news.capcomusa.com/lets/browse/ultra-street-fighter-4-change-list)

## Source input and contact anchors

Intervals are half-open raw BAC animation ticks. Boundaries are `IASAFrame` where present. Actual elapsed time also depends on rate commands and state transitions. Reaction fields are listed for ordinary standing hit unless specified.

| Property | Ken standard Ultra data |
|---|---|
| Forward/back walk | Physics sets `+0.046000000089 / −0.034499999136` units per tick, both with zero acceleration. Relative to the 1.2-unit normal cutoff, 16 advancing ticks move 0.6133/0.46 of the cutoff respectively. Keep port scaling open until Ken's cutoff and axis movement are verified in the same fixture. |
| Close and far jab | Close hits `[9,13)` and boundary 26; far `[8,12)` and boundary 22. Reactions 0/12 give 30/20 damage, 50 stun, 8/8 freeze and 0.3 push. Far has a distinct rate schedule, including 2× startup and 3× early recovery. The pinned frame table gives both jabs 4 startup, 2 active and 5 recovery. |
| Inazuma / Step / Thunder | `4MK` reaction 80 over `[21,23)`, boundary 39, overhead, 80 damage/100 stun, 10/10 freeze, 0.3 push. `6MK` reaction 81 over `[20,23)`, boundary 43, mid, 60/100, 10/10 freeze, 0.7 push. `6HK_FEINT_aer` opens on forward+HK and routes on release at raw `[11,13)` to `6HK` tick 0. Continued hold produces a feint with no strike and boundary 35. Released Thunder strikes `[21,23)`, boundary 45, overhead, reaction 82 with 120/150, 12/12 hit freeze and 12/16 standing guard freeze, 0.6 push. |
| Target combo | Close `5MP_NEW` hits `[11,14)`, reaction 65, with `TC_5HP` over `[11,35)` for conditions 6 and 1. The group contains only HP. Far `5MPF` strikes `[5,10)` with the `SUPER_COMBO` gate and no target HP path. Follow-up `TC_1_HP_NEW` has one reaction (83) across adjacent boxes `[16,18)` and `[18,20)`, 70 damage/50 stun, 12/12 freeze and 0.5 push. Its special/Focus/Super execution record ends at raw 18. |
| Crouching MK and sweep | MK strikes `[10,14)`, low, reaction 32, boundary 41. 60 damage/100 stun, 10/10 freeze and 0.5 push. Its `SUPER_MOVE` execution row uses `[10,14)`. Sweep hits `[14,17)`, low, reaction 34, boundary 42. 100/100, hit stop 14/12, standing guard stop 13/12 and 0.7 push. |
| Hadoken | All versions launch `[25,26)`. Ordinary action boundary 59, EX 56. VFX speed is 0.05/0.06/0.07/0.10 with zero acceleration. Reactions 100–102 are 70 damage, 100 stun, 18 standing chip, 8/8 stop and 0.8 push. EX reactions 103 and 149 are 50 damage/50 stun and 8/8 stop each. They use 0.5/0.8 push and 12/13 guard chip. EX stores both contacts in the same projectile. |
| Shoryuken | LP boxes `[9,12)`, `[12,20)`, reactions 66/67. MP boxes `[8,11)`, `[11,20)`, reactions 118/119, first launches on standing hit. HP boxes `[8,10)`, `[10,12)`, `[12,20)`, reactions 120–122, damage 70/50/30. First two HP boxes overlap the `SUPER_COMBO` condition-1 window `[8,12)`; third is outside. EX has boxes `[8,10)`, `[10,12)`, `[12,14)`, `[14,19)`, reactions 123/124/124/125, with group window `[8,12)`. The group contains Shoryureppa and EX Focus/Red. Capcom gives −5 for first-hit ordinary DP FADC forward on guard and −7 for HP second-hit FADC forward in the final change list. |
| Tatsumaki | Ground starts `[9,11)`, with L/M/H/EX respectively 3/6/8/8 authored groups. Air normal L/M/H use 4/5/7 groups and EX seven, each begins `[19,21)` or EX `[19,22)`. Forward jump switches from the early forward list over `[0,16)` to the ordinary list over `[16,36)`. L/M/H share the reaction across their two trajectories. Their first horizontal acceleration differs: ordinary 0.001/0.002/0.003 and early −0.0001/−0.0005/−0.001. The early EX has a separate upward command at raw 6. Count boxes independently from landed hits. |
| Both ground throws | Grab reaction 90 at `[3,5)`, then distinct `THROW_F_ATTACK/B_ATTACK`. Finishers are `[62,64)` and `[65,67)` on the new actions. Reactions 91/92 give 120/130 damage, 80 stun and hard knockdown. Both grabs use width 0.464 centered at x 0.35 in the source. |
| Focus and Red | Normal level 1/2 strikes `[17,19)`, reactions 60/61; maximum `[77,79)`, reaction 62. Red uses the same raw windows with reactions 70/71/72. Charge release branches are `[13,21)` and `[21,59)`. |
| Shoryureppa | All three versions have six authored strikes at `[52,54)`, `[54,56)`, `[56,65)`, `[94,96)`, `[96,99)`, `[99,108)`. Base standing damage 40/40/40/50/100/100. LP/MP use 126–131, HP uses 166–171. |
| Shinryuken U1 | `UC_SHINRYU_1` first attacks `[66,68)` with reaction 132. Its `ON_HIT` at `[67,68)` selects cinematic `UC_REPPA_2` with target lock on entry. Ordinary branch has ten authored strike groups including the opener; the cinematic has sixteen appended groups. Capcom's published non-cinematic damage is 360. Raw damage fields and displayed full-gauge values use different scales. |
| Guren Senpukyaku U2 | `UC_TATSUMAKI_1` first attacks `[124,126)` with reaction 180 and sets forward speed 0.12. A later group hit at `[213,215)` branches to `UC_TATSUMAKI_2`, whose target-locked finisher uses reaction 184 at `[84,88)`. A simultaneous KO rule skips to raw 215. Its non-cinematic path has a late `[221,224)` box. |

[Ken PC extraction and deltas](https://github.com/dantarion/sf4tools/tree/e07764262003f3aa1feff1a0a5a66fc3c14efb43/characters/KEN) · [Capcom launch changes](https://news.capcomusa.com/combofiend/blog/2014/05/22/usfiv-final-character-change-list) · [Capcom v1.04](https://news.capcomusa.com/lets/browse/usfiv-version-1-04-adjustments)

## Version comparison and published timing

The PC AE2012 v1.01 → Ultra v1.01 extraction records changes to the forward walk, target combo, jump paths, both Ultra scripts and ground special reactions. Capcom's final Ultra notes corroborate the walk speed increase, crouching HK startup 8 to 7, ordinary Hadoken damage 60 to 70, EX Hadoken 50+40 to 50+50, MP Shoryuken first-hit knockdown and HP second-hit FADC. Ken's latest extracted delta is Ultra v1.02 → v1.04. Capcom identifies restored AE2012 crouching MK geometry and removal of normal air Tatsumaki escape cancels from a jumping attack. The extracted `AIR_F` list loses normal air L/M/H entries and keeps EX. The plain forward-jump action still references both dedicated Tatsu lists, so test an uncommitted jump and a jump after an attack independently. [PC AE2012 v1.01 → Ultra v1.01](https://dantarion.github.io/sf4tools/characters/KEN/AE2012%20v1.01_TO_Ultra%20v1.01.html) · [PC Ultra v1.02 → v1.04](https://dantarion.github.io/sf4tools/characters/KEN/Ultra%20v1.02_TO_Ultra%20v1.04.html) · [Capcom v1.04](https://news.capcomusa.com/lets/browse/usfiv-version-1-04-adjustments)

The SuperCombo table preserved at revision `356186` gives standard Hadoken 14 startup, 33 recovery and 47 total; EX 14 startup, 29 recovery and 43 total. It corroborates the four projectile velocities and 50+50 EX damage. FAT agrees for those figures but stores 47/43 in its fields labeled `recovery`. Use the original field value when comparing datasets. The two references give the same 4/2/5 far jab. Capcom's final Ultra change list describes Thunder Kick reaching 0 on guard, and the archived table gives 0 on guard and −1 on hit. A January location-test note instead described the change as on hit. The final notes and retained data take precedence for the Ultra comparison. [Pinned SuperCombo Ken revision](https://wiki.supercombo.gg/api.php?action=parse&oldid=356186&format=json) · [Pinned Ultra FAT export](https://raw.githubusercontent.com/D4RKONION/FAT/dee6b85a7f8f166701ff3f8ab6cfa954949d99aa/src/js/constants/framedata/USF4FrameData.json) · [Capcom final launch notes](https://news.capcomusa.com/combofiend/blog/2014/05/22/usfiv-final-character-change-list)

## Port checkpoints and live recipes

| Property | Comparison status and next measurement |
|---|---|
| Three ground special families, Super and Ultras | Inputs and selectable loadouts match at the command level. The port U1/U2 map to 0x2a/0x2b and use shared bank slots. Verify each gauge threshold, source full cinematic versus late or air contacts, hit count, KO and Double scaling in the matched fight. |
| Three kicks, feint, target combo and Air Tatsumaki | Verified canonical dispatch differences. The native Ken SFIV command table omits these rows. Test `4MK/6MK`, `6HK` press then release around the threshold, and `close MP → HP` after hit, guard, whiff and at the near/far edge. Test `j.214K/KK` after forward, neutral and back jumps, with and without a preceding attack. |
| EX Hadoken | Verified configuration difference. Source encodes two effects and the shared port SFIV projectile path defaults to one contact. Whiff one projectile at each strength to measure spawn and recovery, then hit and guard at two gaps. Include a same-time projectile clash. |
| Focus cancel | Verified gate difference. Port uses active-special admission without a Ken-specific strength/hit-phase check. Compare first and second HP Shoryuken contacts and the first rejected input for each of `MP+MK` and `LP+MP+MK`, then dash. Keep hit, block and whiff separate in the log. |
| Classic throws, delayed standing and walk | Verify shared directional-heavy port throw against source light-pair, plus both source target lock and soft/hard knockdown behavior. Repeat eligible knockdown with early, accepted and late two-button presses. For walk, capture 16 unobstructed ticks and the far-jab cutoff, compare native ratios and confirm initial stance pose. |

[Ken native commands](../../crates/sf-engine/src/era_fight/sf4.rs#L293) · [Ken Ultra bank slots](../../crates/sf-engine/src/data_bank/sf4.rs#L20) · [Port projectile defaults](../../crates/sf-engine/src/era_fight.rs#L1592) · [Port Focus admission](../../crates/sf-engine/src/era_fight/sf4.rs#L651) · [Port Ultra loadout test](../../crates/sf-engine/tests/sf4_native.rs#L585)

Extract the two archive members into one directory and feed them to the pinned UltraDiff `BACFile`/`BCMFile` decoders. Resolve BCM `Moves` to BAC `Scripts`. Inspect `HITBOX`, `CANCELS`, `FLOW`, `SPEED` and `PHYSICS`, skipping proximity boxes when reading strike windows. Resolve `HitboxData` via the numeric key in `HitboxTable`. Compare hit rows 0–2 and guard rows 3–5. The emitter's `ETC` command `VFX_SCRIPT` uses the first parameter to index ordered `VFXScripts`. As extraction checks, require `6HK_FEINT_aer` → `6HK` at raw `[11,13)`, `TC_5HP = [TC_1]`, third HP Shoryuken reaction 122, and EX Hadoken reaction 149.

For paired captures, use Ken versus Ryu in PC Steam standard Ultra v1.05 and the port's USF4 card. Use all three Ultra loadouts with matching training gauge states. Reset at midscreen between attempts, then repeat the tested routes after exchanging sides and in the corner. Record raw input, first selected action, contact, both stop/recovery counters, meter changes and positions. The exact playable gates and endpoint agreement remain under #148.
