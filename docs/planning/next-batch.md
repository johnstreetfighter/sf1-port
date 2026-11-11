# Next recovery batch: proposal (ticket #13)

Status: **approved by the user on 2026-10-22 and published** (issue numbers
in `next-batch-publication.json`: N1-N27 are #14-#40, in local-id order except
N11 = #25 and N12 = #24). Written 2026-10-21 for
issue #13. It sizes the next
batch from the slices that work now and maps every remaining full-port
requirement to later bounded work. It does not certify the first playable
milestone or the complete port.

Evidence labels used below: **[code]** read from the original program at the
given address; **[cap]** observed in a pinned-reference capture; **[inf]**
inferred, not verified. Instruction counts are linear-listing counts of the
untranslated ranges (approximate; they include some data bytes the listing
decodes as code). The address surveys behind this document were read-only;
their scratch scripts and extra probe captures are session-local and not
part of the repository.

## 1. What works now

Checked 2026-10-21: `cargo test --workspace --release`, 87 tests, all
passing with the local reference data present (no acceptance test skipped).
One earlier full-suite run that day failed once: two hit-sound tests shared
a temporary capture directory. That race is fixed. The failure
did not reproduce in 3 runs before the fix, or in 5 runs of that suite after
it.

| Slice | Issue | Verified result | Kind of logic |
| --- | --- | --- | --- |
| Pinned MAME baseline | #2 | two runs byte-identical over 5000 frames; a 10-frame input shift is detected | reference |
| Comparison runner | #3 | identical runs pass; a shifted event and a changed pixel each fail at the expected frame | tooling |
| First frame | #4 | 3 checkpoints byte-exact at 384×224 | native renderer from decoded ROM graphics |
| Idle scene | #5 | 109 frames: pixels, video/palette RAM, latch timing | **tables built from observed objectram** |
| Walk left/right | #6 | 901 frames match | **tables built from observed behavior** |
| Crouch | #7 | 950 frames match | **tables built from observed behavior** |
| Jump | #8 | 550 frames match | **fixed 40-frame script built from observation** |
| Standing light punch | #9 | 950 frames match; RAM regions identical; hit and miss | translated ROM path |
| Opponent's first decision | #12 | 44 frames; full objectram identical | translated ROM path |
| Punch-hit sound | #10 | 198,753 and 68,968 port writes identical; DAC identical over 6238 channel ticks | translated samples CPU + MSM5205 model |
| First music phrase | #11 | all 2283 YM2151 writes in order within 1 sample; 74,574 output samples identical | translated music CPU + ymfm |

**Failing:** none of the recorded acceptance checks.

**Not ADR-0001-compliant yet:** walk, crouch, jump and idle (#5–#8) reproduce
the reference exactly but are driven by tables built from observed behavior,
not by translated logic. Each slice doc discloses this. Ryu's entity fields
are therefore not maintained while he walks, crouches or jumps. Ticket N2
below replaces them.

**Unimplemented**, from each slice doc's "not covered" list:

- every attack other than the standing light punch;
- special moves;
- blocking, crouching and airborne reactions, and knock-downs;
- the CPU being hit;
- every opponent decision after the first;
- the round timer as logic (the HUD is observed only);
- round outcomes and transitions;
- sound effects on the FM chip, music beyond one phrase, and both sound
  chips together;
- the whole arcade flow outside the fight;
- two-player play beyond the versus fixture;
- keyboard remapping.

## 2. What the recovered code says remains

### 2.1 Game structure (affects every later slice)

- **Cooperative scheduler** [code 0x3b19a–0x3b2ca]: 16 task slots of 0x10
  bytes at 0xff8006 (state, sleep count, SR, PC, user stack). Trap #0 spawns,
  #1/#3 kill, #2 sleeps; sleep helpers at 0x3b51a–0x3b57a. Each frame, after
  vblank, the ready tasks run once in slot order.
  - Slots: t0 = timer and score tallies (0x3f4de); t1 = flow sequencer;
    t2 = screen/event task (0x3f7b0, handler table 0x3f7d8); t3 = INSERT COIN
    and challenger watch (0x3f374); t7 = random state and coins (0x4813e,
    already translated).
  - With vblank, about 520 instructions.
  - The native frame today hard-codes the fight loop's order. Round flow and
    the arcade flow need the real scheduler.
- **Screen events** [code 0x422d0]: t1 sets 0xff8e36 = N and SCENE
  (0xffc008) |= 0x20; t2 runs handler N and clears the bit. Handlers 2–11
  are the map, VS, round intro, next round, round result, draw/time-up,
  KO/explosion, game over and quote screens (about 500 instructions).
- **Entity-slot swap** [code 0x3b852, about 90 instructions]: a frame flag
  (0x10 / 0x20) swaps the fighters' 0x600-byte object regions and the player
  structs' +0xe words. In the fixed fight it happens during round 1 [cap]
  and persists into round 2 (P1 at 0xa00, CPU at 0x400). The Rust code
  hard-codes Ryu = 0x400, which is wrong after a swap.
- **Random state** depends on every input since power-on (t7 folds the live
  input byte each frame) [code]. A development launch from a documented
  fixture carries it; the full arcade flow must reproduce it from frame 0.

### 2.2 Player combat (Ryu; Ken uses parallel code and data)

- **Posture dispatch** 0x404cc [code]: entries 1/2/4/5/6/8/9/10 at 0x405c6,
  0x406ee, 0x40812, 0x40894, 0x4098e, 0x40a86, 0x40a98, 0x40abe (about 280
  lines). A busy gate at 0x403d4; walk state 0x91 is gated and 0x92 is
  interruptible (explains the observed crouch deferral) [inf].
- **Moves**: Ryu has 31 move headers (0x00–0x1e) [code]. Plus 7 jump-attack
  ids (0x20–0x26) without headers, 13 hit reactions (0x30–0x3c) and 11
  knock-downs (0x40–0x4a).

  | Posture | Punch L/M/H | Kick L/M/H | Neither |
  | --- | --- | --- | --- |
  | Standing / forward / back | 0d / 0f / 10 | 14 / 15 / 16 | 1, 2, 3 |
  | Down / down-forward / down-back | 11–13, then 1c–1e | 17 / 17 / 18 | 5 / 6 / 7 |

  Back starts block move 4 when the opponent is attacking; down-back starts
  move 8. Jumps are 9 / 0xa / 0xb.
- **Special moves** [code]: tracker 0x40ae4 (36 lines), call sites
  0x40360–0x403b2 (23). Each tracker is 3 bytes (stage, timer, done) with an
  18-frame window. Directions are mapped facing-relative through 11-byte
  tables that indices 11–15 read past (a quirk to keep).

  | Special | Command | Move | Notes |
  | --- | --- | --- | --- |
  | Tatsumaki | D, DB, B + kick | 0x1b | 5 loops through globals 0xff8704/6; hit 0x45f5a; chip damage |
  | Shoryuken | F, D, DF + punch | 0x19 | invulnerable header; unblockable hit 0x45efa |
  | Hadoken | D, DF, F + punch | 0x1a | spawns effect 0x186dc; collisions 0x430b6 / 0x43144 / 0x431c6 and handlers 0x43348–0x43520 |

- **Hit functions** 0x45af0–0x46500 [code]: 620 lines, about 17 near-copies
  of about 6 shapes. A high block is +0xa == 4; a low block is +0xa == 8
  only. Some blocked kicks redirect the attacker's stream. Damage tables
  hold 8 bytes per entry; damage ranges from 1 to 0x14.
- **Jump attacks** [code]: apex call 0x452e0 (44 lines), tables
  0x194b8–0x19530, hit functions 0x45e10 / 0x4638e / 0x463ae / 0x463ce.
- **Not found**: stun/dizzy and throws [inf: absent in this game's hit path
  and dispatch].
- No self-modifying code. Nothing needs 68000 cycle timing. The
  return-address tricks (0x40b72 skips its caller's `bra`; 0x4304c, 0x43072)
  must be translated faithfully.

### 2.3 Opponent behavior

- One AI for all ten opponents; only the data differs [code].
- **Table setup** 0x44880 (about 140 instructions): opponent index =
  0xffc003·2 + 0xffc004, ten blocks of 0x300e bytes at 0x1c048 + k·0x300e.
  It copies 0xffc058..0xffc663, with rows chosen by difficulty (0xffc007,
  DSW2) and a variant bit. Retsu's RAM copy at frame 2460 equals ROM block
  0x1f056 byte for byte [cap].
- **Follow-up decision** 0x45630 (about 200 instructions), run at the end of
  every CPU move stream:
  - it reacts to player attacks within reach (table 0x146aa) by blocking,
    crouch-blocking, jumping or walking;
  - otherwise it picks an attack by range class and the player's posture;
  - otherwise it shifts the range class.

  0x40bd4's other branches (96 instructions, translated) are unexercised.
- **Retsu's attacks**: 7 attack streams with 7 hit functions from the
  template at 0x4664a; no specials [code].
- **CPU being hit**: 0x447c8 (about 40 instructions; tables through
  0xffc050 / 0xffc054); never reached in the fixed fight, where Retsu
  takes no damage [cap].
- Other opponents add extra stream routines (Geki 0x4594c / 0x4596e /
  0x45a34, Adon 0x4558a, Sagat 0x45ace) and their own data.

### 2.4 Round outcomes (fixed fight timeline [cap])

| Frames | Event |
| --- | --- |
| 2117 | "ROUND 1" |
| 2390 | Round 1 fight starts |
| 3328 | KO detected |
| 3362–3569 | Round result |
| 3570–3668 | Wipe |
| 3669–3837 | "ROUND 2" |
| 3949 | Round 2 fight starts |
| 4773 | KO detected |
| 4807 | CPU wins the match |
| 5014 | Result ends |
| 5015–5314 | Quote |
| 5415–6099 | Continue countdown |
| 6100–6330 | Game over |
| 6331 | Title |

`fight-run-a` ends at frame 5000.

- **Timer** [code 0x3f4e2]: three BCD bytes at 0xff8e18..1a, reload table
  0x3f544 by DSW2 (100/150/200/250), a tick every 19 frames. It keeps
  counting after a KO (the HUD freezes a copy at 0xff8e3c).
  - **Open discrepancy**: the arcade-flow probe measured about 23 frames per
    tick [cap, unexplained]; the CPU-decision slice observed ticks at 2409
    with 19-frame spacing. Settle this in N12.
- **KO and round end** [code 0x3eb22–0x3ee26]: time-over (text 0x14f74,
  sound 0x96, 130-frame wait, health comparison), draw, double KO
  (0x3eca2), and three rounds played (0x3ee04; in 1P a draw counts as a
  loss). The score bonus (0x3fcd4) is paid only when a human wins.
- **Never captured yet**: time-over in a fight (only in the attract demo
  [cap]), draw, double KO, any player round or match win, CPU damage.

### 2.5 Sound

- **Music CPU** [code, table 0x1011]: 64 commands, made up of 21 tunes,
  24 distinct FM effects in 27 entries, and 16 special commands.
  - 13 tunes loop through translated opcodes; 8 end with opcode 0xf.
  - Effects use channels 5–7 only and never the stage tune's channels 0–5.
  - Slot 0xc580 is never started.
- **Untranslated music code**, about 670 instructions:

  | Range | Instructions | Role |
  | --- | --- | --- |
  | 0x0240 | 51 | effect start |
  | 0x02a5–0x032a | ~57 | special-command dispatch and stops |
  | 0x032b | 57 | channel handback |
  | 0x03a4 | 51 | stop or return to the saved tune |
  | 0x09e7 | 9 | opcode 0xf |
  | 0x0a16 | 3 | effect-owned channel skip |
  | 0x0ab9–0x0faf | ~440 | effects player |

- **The fixed fight sends** music commands 01–04, 09, 0b, 0f, 19, 1c, 27,
  28, 33, 34 and 3f [cap]. All its effects, 0x27 and 0x3f reach
  untranslated paths.
- **Samples CPU** [code]: commands 0x81–0xbf, 63 commands for 59 samples
  (about 102 s of ADPCM, banks 0–6); 0x80 stops.
  - The fixed fight uses bank 0 on channel 0 and bank 2 on channel 1 [cap].
    Channel 1 and banks 1–6 are still unverified because `fight-run-a` was
    captured without sound.
  - No new code is expected [inf].
- **68000 side** [code]: 175 call sites of the sound queue 0x3b7c8, which
  filters tunes and attract-mode sound by DSW1. Events include the round
  announcer (table 0x3fc62), FIGHT! (0x94), move sounds from move header
  +0x10 (specials 0x89 / 0x8a / 0x8b), impacts, sparks, blocks (0x08) and
  KO (87, 04, 09, a7).

### 2.6 Arcade flow (complete port)

The flow code (0x3c0c4–0x40000) ran about 1800 instructions in the probes.
Shared helpers ran about 560 more, and 0x40000–0x4834e (fight engine, bonus
games, screens, ending) about 5700. No new video hardware is needed: one
vblank interrupt, no raster effects. Every full-screen picture is a scroll
offset into the tile ROMs plus the text layer and palette uploads. The `sf`
set has no MCU or protection [code + driver].

| Requirement | Code [code] | Size |
| --- | --- | --- |
| Boot and warning | 0x3c0c4–0x3c290 | ~255 |
| Attract | wall 0x3ccae, ranking 0x4268c, demo fight 0x3ce54 (demo inputs: 3 bytes per frame from 0x3cebf) | ~165 + demo |
| Coins, credits, start | 0x3dec2–0x3e04c, coinage 0x1456a | ~270 |
| Country select and opponent order | 0x3e04c–0x3e39a; order records 0x19908 + 0x28·country; Thailand is always last | ~250 |
| Pre-fight map, VS, round intro | 0x3e5d6–0x3e960, screen handlers | ~550 |
| Match end, quote, continue, game over | 0x3ee26–0x3f374 | ~350 |
| Name entry and ranking | 0x42422 | ~300 |
| Power/timing bonus | 0x40d30, 0x41a02 | ~300 |
| 10-second bonus | 0x41740, 0x41fe6 | ~250 + objects |
| Ending | 0x44a40 ("KING OF THE HILL", staff) | ~250 incl. map |
| Two-player | versus (mode 1), the winner carries on against the CPU (0x3ee5c / 0x3ee8e), challenger joins (t3 at 0x3f41e, 0x423b8) | ~150 + reuse |
| DIP switches | difficulty, round time, continue and challenger limits, countries, speed, attract sound, freeze, flip | decode at boot |
| Keyboard remapping | host-side; no original code | — |
| Test mode | 0x4834e–0x49aca | ~1.4k; last, after full-game verification |

**Renderer gap**: flip-screen flips only sprites today; MAME also flips the
tilemaps. It matters only with that DIP switch on.

## 3. Native-recovery feasibility blockers

None found for game logic: no self-modifying code, no protection, no raster
effects. Four items need a decision or an investigation before the work
they gate:

1. **When the 68000 writes the sound latch within a frame.** It is not
   modelled; 102 µs after the frame start in the music scene versus 183 µs in
   the versus scenes [cap].
   - The music CPU polls every 4 ms, so a write near a poll boundary moves a
     whole tune or effect by 4 ms. Write identity of native gameplay sound
     over a whole fight can fail on this alone.
   - Resolving it needs the 68000's position in the frame at each latch
     write, i.e. a model of its vblank-relative timing.
   - Ticket N22 investigates. If a translation-level model is infeasible,
     that is a scope question for you. It will not be silently replaced by
     a tolerance.
2. **Clean late-game references.** Bonus games, later countries, the ending
   and name entry need a reproducible winning input script for 1P. The
   probes reached them only by changing RAM, which is diagnostic evidence,
   not a reference. Searching for a deterministic winning input run is
   planned work in the later batch.
3. **Random state from power-on.** The development launch carries it in its
   fixture; the full arcade flow must reproduce the input mixing from frame 0.
4. **Hidden code entry points.** Move and object streams call code through
   pointers in ROM data (0xff stream calls), which static analysis cannot
   enumerate. Each slice sizes its own with a coverage probe. This is a
   sizing risk, not a blocker.

## 4. Proposed next batch: the first playable fight

**Goal**: the fixed fight (1P Ryu vs CPU Retsu), launched from its
development fixture, plays natively with keyboard input. Its combat,
opponent, round and sound behavior is translated, and the batch ends in a
milestone audit. Every ticket inherits the specification's
native-translation and fidelity rules, keeps original data local, and
labels fixture-only results.

Fixtures: the 2P versus fixture (P2 = Ken, scripted through scenario inputs)
isolates player combat. The 1P fixture covers Retsu. Each ticket that
needs a new pinned reference capture records it (scenario JSON plus two
repeat runs).

Ticket ids are local (N1…); "Blocked by" lists genuine gates only.

| # | Title | Blocked by | Delivers |
| --- | --- | --- | --- |
| N1 | Run the native frame through the recovered task scheduler | — | the scheduler, traps, sleeps and vblank order translated; the existing fight steps become tasks; every existing scenario still matches |
| N2 | Walk, crouch and jump from the translated posture dispatch | — | 0x404cc entries 1–10 and the busy gate replace the observed tables of #5–#8; adds diagonal jumps |
| N3 | Follow the fighters through the entity-slot swap | — | 0x3b852 translated; entities found through +0xe instead of fixed slots; a standing heavy kick (move 0x16) that triggers the swap matches |
| N4 | All standing normal attacks | N2, N3 | moves 0d–10 and 14–16 from neutral, forward and back, and their hit functions, against Ken |
| N5 | Crouching attacks | N2 | moves 11–13, 17, 18 and 1c–1e, crouch reactions |
| N6 | Jump attacks | N2 | ids 0x20–0x26 from neutral and diagonal jumps (0x452e0), air reactions |
| N7 | High and low blocking | N4, N5 | moves 4 and 8, the high/low rules, attacker stream redirects, block sound 0x08 |
| N8 | Shoryuken and Tatsumaki from the command trackers | N4, N7 | 0x40ae4 trackers (with the table-overrun quirk), moves 0x19 and 0x1b, invulnerability, chip damage |
| N9 | Hadoken and projectile collisions | N8, N1 | move 0x1a, effect 0x186dc, the three collision tests and their handlers, and the explosion event through the event task |
| N10 | Opponent AI tables for every opponent and difficulty | — | 0x44880 translated; Retsu's RAM at 2460 matches, and all 80 opponent/difficulty/variant table images match the ROM |
| N11 | Retsu's follow-up decisions and attacks against idle Ryu | N1, N3, N10, N12 | 0x45630, Retsu's 7 attacks and hit functions; a round-2 fixture at 3948 (new capture) matches until the next untranslated branch |
| N12 | Round timer and TIME HUD from the timer task | N1 | t0 translated; timer digits and HUD match over round 1 and across the KO freeze; the 19- vs 23-frame tick discrepancy settled |
| N13 | Retsu wins round 2 natively, through the KO | N11 | frames 3949–4773 match end to end from the 3948 fixture; KO detection |
| N14 | Retsu takes hits, blocks and is knocked out | N2, N4, N11 | the CPU reaction path 0x447c8; 0x45630's reaction branch (blocks, jumps); new 1P scenarios where Ryu hits and KOs Retsu |
| N15 | Round result after a CPU win | N1, N12, N13 | KO path 0x3eb22, event 7 (CPU branch), round markers, typed text; 3328–3569 and 4773–5014 match |
| N16 | Round transition into round 2 | N15 | events 6 and 5, round init 0x3e77c, intro, FIGHT!, timer reload; 3569–3949 match; the observed idle/intro tables of #5 retired |
| N17 | Time-over, draw and double KO | N15, N14 | 0x3ed20, 0x3eca2 and 0x3ee04 from new scenarios (round time DIP 100, a time-out draw, a trade KO) |
| N18 | Ryu wins a round and the match | N14, N15, N16 | the bonus tally (0x3fcd4, time and vitality), the player-win branches, match end into the next opponent's setup; needs a winning input script |
| N19 | Samples-CPU sound over the whole fixed fight | — | a new sound-enabled capture of the fixed fight; every samples-CPU write and the DAC over the KO window match (channel 1, bank 2, overlapping playback) |
| N20 | Music special commands, stops and tune ends | — | 0x02a5–0x0423 and 0x09e7; write and stream identity over a window with a stop and a tune start (new checkpoint) |
| N21 | FM sound effects from the effects player | N20 | 0x0240 and 0x0ab9–0x0faf; the hit-spark effect 0x01 driven by the native hit event matches write for write |
| N22 | Sound-latch write time within the frame | — | investigation: the 68000's position in the frame at each latch write in the fixed fight, and a translation-level model or a documented infeasibility for your decision |
| N23 | Music CPU from power-on over the whole fixed fight | N20, N21 | removes the music fixture and the idle-phase constant; every YM2151 write over 5000 frames matches |
| N24 | Both sound chips from the native engine over a fight | N19, N22, N23, N13 | the native engine's latch drives both translated sound programs over round 2; per-device stream identity and the measured mix against MAME's WAV |
| N25 | Keyboard play of the fixed fight in the launcher | N9, N13, N14, N16 | the six attack keys, specials and both players in the dev launcher; host smoke check (fixed bindings; remapping is later) |
| N26 | First playable milestone audit | N6, N17, N18, N24, N25 | the whole fixed fight (fight-run-a's inputs) from the round-1 fixture through the match end compared end to end; a written pass/fail account against the brief's milestone |
| N27 | Size the arcade-flow batch | N26 | turns §2.6 into published tickets, re-checking sizes against the then-current code |

### Acceptance criteria per ticket

- **N1**
  - Translate 0x3b19a–0x3b2ca, the traps, the sleep helpers and the vblank
    order as resumable task state machines, with original addresses.
  - Every existing scene scenario (#5–#12) still matches.
  - No scenario's behavior changes; the frame order now comes from the
    tasks.
- **N2**
  - Translate 0x403d4 and 0x405c6–0x40abe without the attack branches.
  - sf-move-01, sf-crouch-01 and sf-jump-01 match with Ryu's entity RAM
    byte-identical, which the observed tables could not do.
  - A new diagonal-jump scenario matches.
  - The observed walk, crouch and jump tables are deleted.
- **N3**
  - Translate 0x3b828–0x3b8ea; remove the fixed-slot assumptions.
  - A versus-fixture heavy kick matches across the swap frame.
  - A swapped fixture (taken after round 1) loads and runs.
- **N4**
  - Each of the six standing strengths, and the forward/back variants,
    matches hit and miss against Ken.
  - The hit-function shapes are translated, each traceable to its address.
- **N5**
  - Every crouch move matches, including the first-frame hold of 0x455fc
    (table 0x45620) and the crouch reaction entries.
- **N6**
  - Neutral, forward and back jump attacks match for punch and kick,
    including a press after the apex re-latched on landing.
- **N7**
  - P2 holds back, then down-back, against high and low attacks.
  - A standing block fails against a low attack and a crouching block fails
    against a standing attack, matching the reference.
  - Blocked kicks redirect the attacker's stream, as observed.
- **N8**
  - Each special fires from its command inside the 18-frame window and
    fails outside it.
  - Shoryuken's invulnerability and unblockability, and Tatsumaki's 5 loops
    and chip damage, match.
- **N9**
  - Hadoken versus a standing Ken, a blocking Ken and a Ken hadoken
    (projectile versus projectile) match.
  - The explosion event's palette and sound match.
  - Investigate the event-number conflict: the combat survey read event 9
    as the projectile explosion, the flow survey as the KO slow-motion.
- **N10**
  - Ten opponents × difficulty × variant table images match the ROM.
  - Retsu's images match the 2460 RAM dump.
- **N11**
  - Record a pinned capture with a checkpoint at 3948.
  - The native run from it matches until the first untranslated branch,
    which is reported, never guessed.
- **N12**
  - The timer digits, the per-tick frames and the HUD cells match over
    round 1 and through the frozen copy.
  - The tick-period discrepancy has a recorded cause.
- **N13**
  - From the 3948 fixture, 3949–4773 match per frame (pixels at
    checkpoints, objectram, health cells, latch).
- **N14**
  - New 1P scenarios: Ryu's punches and kicks land on Retsu, Retsu blocks
    when the reference does, and Ryu KOs Retsu.
  - These match through the CPU reaction tables reached through
    0xffc050 / 0xffc054.
- **N15**
  - Both rounds' result sequences match, including the typed text, the
    holds and the round markers.
- **N16**
  - 3569–3949 match, including checkpoint 3840 and the timer reload.
  - The round intro is translated, not observed.
- **N17**
  - Each outcome has a pinned scenario and a native match.
  - In 1P, a draw counts as a loss, as the code says.
- **N18**
  - Find and pin a reproducible winning input script.
  - The time and vitality bonuses, the score and the next-opponent setup
    match.
- **N19**
  - Two sound-enabled reference runs of the fixed fight are byte-identical.
  - The translated samples CPU matches every write, and the DAC matches
    over the KO window.
- **N20**
  - Write and stream identity over the chosen window (e.g. the 0x3f at 1906
    followed by tune 0x33).
- **N21**
  - Write and stream identity for the spark effect from the native hit
    event, as #10 did for the ADPCM channel.
- **N22**
  - A recorded latch-write time distribution per scene from the reference.
  - Either a translation-level model that places them, or a documented
    infeasibility presented to you as a scope decision.
- **N23**
  - Every YM2151 write over the 5000-frame fight, in order and within 1
    sample, and output-sample identity.
  - The power-on idle phase is derived, not fitted.
- **N24**
  - Per-device output identity over round 2 from native gameplay.
  - The mixed output measured against MAME's WAV, with a listening note.
- **N25**
  - The launcher plays the fixture with fixed keyboard bindings for both
    players' six buttons.
  - A host smoke check covers keyboard, window and audio.
- **N26**
  - A native end-to-end run from the round-1 fixture matches fight-run-a's
    observables through the match end, or the audit lists each mismatch.
  - The audit states what the milestone does and does not demonstrate.
- **N27**
  - Publishable tickets for the arcade flow, with sizes rechecked.

**Parallel starts** (no blockers): N1, N2, N3, N10, N19, N20, N22.

**Oversized-risk tickets**, split further if a coverage probe shows they
exceed one fresh context:

- N4: the hit-function shapes.
- N11: 0x45630 plus Retsu's hit functions.
- N21: about 500 instructions.
- N23: whole-run timing.

## 5. Later work (mapped, not proposed for publication yet)

Ticketed by N27, from §2.6, in dependency order:

1. **Boot.** Scheduler boot, DIP decode and the warning screen (frames
   0–794).
2. **Attract.** Attract wall, title, ranking and INSERT COIN blink (frames
   795–1855).
3. **Coins.** Coins, credits and start, with the coinage DIP variants.
4. **Country select.** Country select and opponent order, both DIP
   variants and the timeout.
5. **Pre-fight.** Map, VS and the pre-fight setup (frames 1865–2279).
6. **Match end and game over.** Match end, the lose quote, continue, game
   over and return to attract (needs a capture extended to about 6400
   frames).
7. **Two-player.** Versus, the winner carrying on and the challenger.
8. **Demo fight.** Needs the attract and round-end slices and enough fight
   engine for the demo's moves.
9. **Name entry.** Name entry and ranking insertion.
10. **Power/timing bonus.** Both copies.
11. **10-second bonus.** Including its breakable objects.
12. **Countries and ending.** Country-to-country map, Thailand and the
    ending.

Other later tracks:

- **Other opponents**: each opponent's attacks, hit functions and extra
  routines (Geki, Adon, Sagat). One ticket per opponent after N11 and N14,
  verified on a fixture of that fight.
- **Ken as P1**: his parallel hit functions and data, verified against the
  same combat scenarios.
- **Complete sound**: the round tune's loop point (needs a round longer than
  about 50 s), every command (all 21 tunes, 24 effects and 59 samples)
  through the service sound test or a scripted command run, and tune
  transitions.
- **Keyboard remapping**: host-side bindings with persistence.
- **Flip-screen tilemaps**: only with that DIP switch on.
- **Full-game verification**: a final completeness and fidelity audit
  across every mapped requirement.

**Test/service mode** (about 1.4k instructions): kept in scope as the final
optional item, after full-game verification (the user's decision,
2026-10-22).
