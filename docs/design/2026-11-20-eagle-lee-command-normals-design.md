# Eagle and Lee command normals from Falchion22's frames

The maintainer, 2026-11-20. Brainstormed in the session that surveyed the MUGEN collection (`docs/planning/mugen-collection-survey.md`, decision 3). Status: approved by the maintainer, 2026-11-20. Nothing is built or imported yet.

## Intent

Falchion22's MUGEN Eagle and Lee have a few poses our sets lack: Eagle driving both sticks straight ahead, crouched or in the air, and two of Lee's flying kicks. The maintainer has permission to use them, with credit (`THIRD_PARTY_NOTICES.md`). He wants them in play as new moves, not as replacements: Eagle and Lee keep every move they have now, and these come out on new inputs as **command normals**.

Each move has a job in a fight, and the job sets its numbers:

| Fighter | Move | Input | Job | Frames (`sff.sff` group, image) |
| --- | --- | --- | --- | --- |
| Eagle | Two-stick low thrust | down-forward + HP, on the ground | A long-reaching low poke | chamber `410,0`, thrust `410,1` |
| Eagle | Two-stick dive thrust | down + HP, in the air | A jump-in that drives down | chamber `600,0`, thrust `600,1` |
| Lee | Dive kick | down + MK, in the air | Changes his jump's angle: he drops steeply forward | chamber `600,0`, kick `600,1` |
| Lee | Knee-up kick | forward + HK, in a straight-up or back jump | Air-to-air | knees up `650,0`, kick `650,1` |

The inputs and jobs were proposed by the builder and taken by the maintainer ("your rec").

## Goals

- The four moves in **SF1+** and in **every what-if era style** Eagle and Lee have (`era.rs`: Eagle's what-if SF2, SF3, USF4, SF5 and SF6; Lee's what-if SF2, SFZ, SF3, USF4, SF5 and SF6).
- One shared version of each move: drawn and timed once, the same in every style that has it.
- Each move listed in the move list under COMMAND NORMALS (the heading exists: `moves.rs`, `MoveKind::Normal`; move-list design line 99).
- Their art adapted from Falchion22's frames, mapped to each fighter's pens and approved on the review page like any pack pose.

## Non-goals

- **Faithful 1987 does not change.** It never sees the data bank (`data_bank.rs` module docs); nothing here may reach it.
- **Eagle's canon SFZ3 kit does not change.** Capcom's Eagle plays as Capcom made him. His SFZ3 normals read the shared SF1+ base table (`fighter.rs:927-937`, `data_bank.rs:570-579`), so any base-table change must be gated away from SFZ3.
- No retuning per era (approach 2, not taken).
- No other new poses. If a move turns out to need one, that is a finding for the maintainer, not a silent art job (SF1+ design, line 51).
- Teaching the CPU to choose these moves on purpose is not part of this work (*builder's call*: the Fair CPU and Utility AI pick from what they already score; if the registry makes them visible to the Utility AI for free, that is fine, but no CPU plans or tests are written for them).

## Anti-goals

- Changing any move Eagle or Lee already has, its input, its timing or its boxes. A plain down-forward + MP must still be Eagle's ROM low thrust (`0x1d` is shared by d/f+MP and d/f+HP: see below).
- A new move leaking into Faithful 1987, Eagle's SFZ3, or a fighter other than its own.
- Colours off the fighter's pens.

## Constraints found in the code

From a read-only survey of the engine (2026-11-20). The file references are the agent's and should be re-checked when building.

1. **SF1+ has no command-normal path for bosses.** `era_start_banked_move` (`fighter.rs:7347-7368`) runs only for styles that `uses_family_bank()` (what-if, SF2 air tatsumaki, SF3-SF6; `era_fight.rs:792-796`); SF1+ has no per-style header table (`data_bank.rs:182-193`). ST Ryu's command normals and the Alpha `overhead_normal` are ground-only and forward-only.
2. **No path starts an air command normal** (direction + button while jumping) anywhere. Jump attacks come from the apex record, indexed by jump type and strength only (`fighter.rs:2026-2051`, `9096-9125`).
3. **d/f + MP and d/f + HP are the same move, `0x1d`**, told apart only by the strength byte (`+0x34`; `fighter.rs:8440-8458`, `posture-dispatch.md:120`). Eagle maps `0x11-0x13` and `0x1c-0x1e` all to his ROM low thrust (`data_bank.rs:5129-5131`).
4. **Move IDs carry meaning:** `0x0d-0x18` normals, `0x19-0x1b` specials, `0x20-0x26` jump attacks (`fighter.rs:6664-6666`, `6996`, `9060`). An air move needs jump-attack handling.
5. **The landing correction** for anytime jump attacks rescales a stream's vertical path to land on the apex record's landing centre (`fighter.rs:9126-9154`). Lee's dive kick must keep its steep angle.
6. **Free slots:** Eagle `0x1b`, `0x1c`, `0x24-0x26` free in every style; Lee `0x1c`, `0x22-0x26` (in the what-if tables). `0x24-0x26` are jump-attack IDs already in use in the SF1+ base table.
7. **`ButtonClass` has no heavy-punch or heavy-kick class** (`moves.rs:128-148`); the registry has no `MoveKind::Normal` entries yet, and the COMMAND NORMALS heading only shows in the Evil Ryu playtest panel (`front/moves.rs:336-340`).
8. **Every move ID needs a `combo::move_stun` row** (SF1+ design line 195).
9. **Two recipe timings disagree with the bank** (Eagle's jump_poke and two-stick smash). Not ours to fix here; noted so the builder copies the bank's numbers, not the recipe's.

## Approaches

1. **One shared version of each move, started by a new command-normal hook** *(taken; the maintainer, "your rec")*. Each move is authored once as a header and stream in the fighter's data bank, in a header area of its own, and every style that has it points at that header. A ground hook swaps Eagle's `0x1d` when the strength is heavy; an air hook reads the held direction and the button while jumping. The move registry lists all four. Trade-off: one set of numbers in every style; a what-if SF6 Eagle's thrust is timed like his SF1+ one.
2. **A separate version per style**, retuned to each era's pace. Rejected: four moves times about five styles to tune and test, for moves no Capcom game has.
3. **A one-off input hook per fighter**, as ST Ryu's two command normals are done. Rejected: bypasses the move registry and the move list, and copies code per fighter.

## What was decided

### Inputs and gating

- Eagle's low thrust: crouching or standing with the stick down-forward, heavy punch. Only heavy: d/f + LP and d/f + MP stay his ROM low thrust.
- Eagle's dive thrust: in any jump (straight, forward, back), stick down (down, down-forward or down-back), heavy punch. Without the stick down, HP is his existing jump attack.
- Lee's dive kick: in any jump, stick down (down, down-forward or down-back), medium kick. Without the stick down, MK is his existing jump attack.
- Lee's knee-up kick: in a **straight-up or back jump only**, stick forward or up-forward, heavy kick. In a forward jump, HK stays his 1987 flying kick whatever the stick says, because players hold forward through a forward jump and every jump + HK is already one of his moves (the maintainer, 2026-11-20, after the builder check). Down-forward + HK stays his plain jump HK (down-forward counts as down for the dive moves, and as nothing for the knee-up kick).
- One air command normal per jump, as with any jump attack: once a jump attack has started, no other starts.
- **The air moves follow the ANYTIME JUMP ATTACKS toggle like every jump attack** (*builder's call*): with it on (Definitive Overhaul, Classic Competitive, Pure Footsies) they start whenever a plain jump attack could, rising or falling; with it off (possible under Custom) they start only where the apex record would start a plain jump attack. The hook therefore sits on both paths. The dive kick's path is its own in both cases and must not be rescaled by the landing correction.
- Available in SF1+ and the listed what-if styles only; never in Faithful 1987, never in Eagle's SFZ3, never under the Pure Footsies preset if that preset strips command normals (`2026-11-08-endings...-design.md:280` says it keeps them: keep them).

### Numbers (builder's proposal, approved with the design)

Taken from the closest move each fighter already has, then moved toward the job. Frames at 60 per second.

| Move | Startup | Active | Recovery | Hit function (damage) | Guard | Path |
| --- | --- | --- | --- | --- | --- | --- |
| Eagle low thrust | 8 (chamber `410,0`) | 4 (thrust `410,1`) | 18 (`410,0` again, then crouch) | his crouching punch's `0x47ba8`, heavy strength | low (crouching guard only) | none; he stays put |
| Eagle dive thrust | 6 (chamber `600,0`) | until landing (thrust `600,1`) | landing 4 + 1, as his jump attacks | his neutral jump HK's `0x47acc`, the 1987 leaping stick strike (16 deep / 12 otherwise) | high (standing guard only) | his jump's own arc, unchanged |
| Lee dive kick | 5 (chamber `600,0`) | until landing (kick `600,1`) | landing 10 | his flying kick's `0x470ea` | high | after startup the jump arc is replaced: 5 px down and 4 px a frame in the way he faces (toward the opponent, even from a back jump) to the ground |
| Lee knee-up kick | 4 (knees up `650,0`) | 8 (kick `650,1`) | `650,0` held until landing, landing 4 + 1 | his neutral HK's `0x4707c` | high | his jump's own arc, unchanged |

Reasons: Eagle's crouching stick sweep is 6 / 14 / 12; the low thrust reaches farther (the thrust frame is 105 pixels wide), so it starts slower and is punishable on a whiff (18 recovery). Eagle's jumping stick attacks start at 6 and stay out until landing; the dive thrust is one of them with a downward reach. Lee's flying kicks start at 4 and travel 5 down and 5 across; the dive kick starts at 5, falls at a steeper 5 down and 4 across, and pays 10 frames on landing so a blocked or whiffed dive can be punished. Lee's neutral HK starts at 3; the knee-up kick starts at 4 and is out for 8, enough to meet an opponent in the air.

Hitboxes come from the drawings, as every pack pose's do (`PixelBoxes` in each fighter's `BOXES`, checked against the pens by `tests/<fighter>_player.rs`): the attack box on the stick tips or the striking foot, the hurtbox on the body. The knee-up frame's hurtbox stays on his body, not his raised knees, so it wins air-to-air from in front.

Stun and combo values: each new move gets a `combo::move_stun` row equal to the move its hit function comes from: the ROM low thrust's (`0x1d`) for the low thrust, the neutral jump HK's (`0x26`) for the dive thrust, the flying kick's for the dive kick, the neutral HK's for the knee-up kick (*builder's call*, settling the review's question).

### Art

- Frames from `A-Eagle/sff.sff` (`410,0`, `410,1`, `600,0`, `600,1`) and `A-Lee/sff.sff` (`600,0`, `600,1`, `650,0`, `650,1`), mapped to each fighter's pens as Mike's crouching block was (ba341a3).
- **Lee's pigtail** is drawn dark red (`#420000`, about 95 pixels a frame) in Falchion22's art; Lee has no dark-red pen. It becomes **black (pen 0)**, as his ROM pigtail is (the maintainer, "your rec").
- **Eagle's hair shade** `#9c7300` (25 pixels a frame) becomes his hair pen 9 (`#887700`), its nearest (*builder's call*, told to the maintainer).
- Eagle's outline `#191919` becomes pen 0, as Mike's `#1c1c1c` did.
- Group `610` duplicates `600` and group `400` is a one-stick poke like his existing crouching punch; neither is used. Group `430,0` (the thrust angled down) is not used.
- Each pose goes in the art pack with a manifest entry, a recipe pose (`origin` generated, as Mike's crouching block), an approval on the review page, and a note naming its source frame. Credit is already in place.
- Placement: each pose is anchored so its feet (crouch) or its body (air) sit where the fighter's existing pose of that kind sits, found by matching shared pixels as with Mike's block, or from the SFF axis where nothing matches.

### Where they show

- Move list: four `MoveKind::Normal` rows (names: TWO-STICK THRUST, DIVE THRUST, DIVE KICK, KNEE-UP KICK, *builder's call on wording*) under COMMAND NORMALS, in SF1+ and the listed styles, and the heading appears in normal fights, not only the Evil Ryu panel.
- Training: the ground thrust gets Show and Try like any registry move. The three air moves are listed but get no Show and Try in this work: the registry's `MoveInput` cannot yet say "in the air" (*builder's call*; a later ticket can add a jump-then-press path).

## Left to the builder

- Where the new headers live (a header area per fighter after `ERA_HEADER_END`, as `RYU_JAB_HEADERS` and `ST_RYU_AIR_HEADERS` are) and which move IDs they take, within the free slots above, so that the air moves keep jump-attack handling (they may reuse the jump-attack IDs through a header redirect, as ST Ken's air normals do).
- The ground hook (a strength-gated redirect of `0x1d` in `start_move`, as ST Ryu's crouching one) and the air hook (in the jumping branch near `fighter.rs:6668`, modelled on `capture_st_ken_jump_attack` and `try_anytime_jump_attack`), including how the dive kick keeps its angle past the landing correction.
- How the what-if style tables point at the shared headers (`bind_era_header` / `alias_era_header`).
- The `ButtonClass` additions needed to describe heavy-only inputs in the registry.
- Tests: each move comes out from its input in SF1+ and in each listed style; not in Faithful 1987, not in Eagle's SFZ3; d/f + LP and d/f + MP still give the ROM low thrust; plain jump attacks unchanged; boxes checked against the pens; the dive kick's path and landing recovery; replays deterministic.

## Grill decisions (the maintainer, 2026-11-20)

- **Words:** "command normal" and "adapted pose" are now in `CONTEXT.md`. The eight frames here, and Mike's crouching block, are adapted poses.
- **Permission:** recorded in ADR-0029, under ADR-0006's exception. The source is not named.
- **Order:** art first. The eight frames are imported and approved on the review page before any work on the two engine hooks. A rejected frame stops its move before engine time is spent on it.
- **State hashes:** new move IDs and headers change Eagle's and Lee's data bank, so state hashes and canonical tags differ from older builds; play does not change. Accepted. The changed tags go in the handoff.
