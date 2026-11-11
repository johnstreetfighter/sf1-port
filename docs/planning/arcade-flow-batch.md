# Arcade-flow batch: proposal (ticket #40)

Status: **approved by the user on 2026-10-23 (ticket #40); all waves 0–16
(F1–F78) published (#41–#90, #122–#149).** The decisions are recorded at
the end of section 7. Waves 3–16 (F50–F77) were re-checked in the 2026-10-31 review (section 1) and published by F78 (#90).
Written 2026-10-23 and re-checked 2026-10-31. Written for
issue #40. It sizes the
arcade-flow batch that next-batch.md §2.6 and §5 mapped. It does not certify
the complete port, and no ticket in it claims to.

This file is the record of the re-check that ticket #40's first criterion
asks for (section 1). After approval and publication,
`docs/planning/arcade-flow-publication.json` will map the local ids F1–F77
to issue numbers, as `next-batch-publication.json` did for #13.

How this was made. The sizes and addresses were re-read from the local
listing (tools/reference/local/sf_maincpu.dis), the Rust source and the
reference docs. Nothing was built, nothing was run in MAME, and nothing was
published. A review of the first draft listed 25 corrections. Each was
checked against the listing, the code and the docs before it was applied
(section 9). A second check of this file listed 24 more defects (D1–D24).
They were checked the same way. Section 9 lists the one not applied as
stated. A third check listed 7 more (R1–R7). They were checked the same
way, and section 9 records the form in which each was applied.

Evidence labels:
- **[code]**: read from the listing, the Rust source, or the local MAME
  sources (tools/reference/local/, not in Git) at the given address or line.
- **[doc]**: a repository doc says so (cited).
- **[cap]**: read from a pinned local capture's frames.tsv, dumps or file
  sizes.
- **[inf]**: inferred, not verified.

Instruction counts are linear-listing counts. Original game data stays
local: this file holds addresses, instruction counts and names only.

## 1. What changed since next-batch.md

Re-checked 2026-10-23.

| Item | next-batch.md | Re-check | Evidence |
| --- | --- | --- | --- |
| Boot and warning | 0x3c0c4–0x3c290, ~255 | The boot task is 106 instructions (76 + 30). The listing shows 114 lines, because the 8-byte tables at 0x3c16a and 0x3c188 decode as code. The boot needs about 183 untranslated instructions plus 21 scheduler instructions (spawn 10, slot address 6, new-task entry 5), about 204 in all. Kill (8) and exit (7) are not reached in frames 0–794, going by their callers. F1 also translates kill, labelled unexercised. A 16-instruction join leads into the attract. (The first draft said 191; that counted 0x3bdb0's 8, which is translated.) | [code] |
| Boot frames | 0–794 | Confirmed. Frame 0 runs no vblank handler. One handler runs per frame from frame 1. The set-up chunk overruns one frame. | [cap]; overrun [inf] |
| Scheduler | ~520 with vblank; one pass per frame | 36 untranslated: spawn 10, kill 8, exit 7, slot address 6, new-task entry 5. The reset is 18 more. Every sleep and exit rescans from slot 0. Exit has one caller, 0x48618, inside test mode. Kill has four: 0x3ceb2, 0x3f368, 0x48232 and 0x4879e. No direct reference was found to 0x3b2ca–0x3b2e2, 0x3b28c–0x3b298 or trap #4. Calls through pointers were not excluded. | [code] |
| Coin-control helpers | not listed | 0x3b654 (OR, 5 instructions) and 0x3b6a2 (AND-NOT, 6). Their RAM effects are already inline in t7 (tasks.rs:2030, 2034). The 0xc00010 port write is not modelled. MAME's coin_w turns that write into the coin lockout (sf-mame0289.cpp:110–116). Boot init calls 0x3b644 and 0x3b690 (at 0x3b77e and 0x3b786). The first 0x3b654 call is at 0x3c26a, in the boot task's second half. | [code] |
| DIP switches | decoded at boot | 34 instructions at boot plus 15 at reset. Speed, Attract Music, Demo Sounds, Freeze and Self-test are read live. Attract Music Off drops 0x28–0x3e in every mode. DSW2 is read at boot and by test mode, which reads all eight port words at 0x484d4–0x484f6. | [code]; other indexed reads not searched [inf] |
| SYSTEM bits 6 and 7 | not listed | 0x3b0f0's branch, 0x3b378 and 0x3b602 depend on them. MAME defines both bits as IPT_UNKNOWN (sf-mame0289.cpp:561–562), so MAME's inputs cannot reach these paths. The vblank already reports 0x3b0f0 and 0x3b378 (tasks.rs:2102, 2123). | [code] |
| Random state | folds inputs from frame 0, every frame | Cleared at reset, spawned at 794, one fold per 7-frame loop. | [code] + [cap] |
| Attract | ~165 + demo | ~165 is reproduced exactly but misses slot 3's texts (33). INSERT COIN is slot 3's. 794 is a valid fixture. The exact timeline is 795–1854. | [code] + [cap] |
| Ranking 0x4268c | counted under both attract and name entry | 79: 47 reached in the attract, 32 for the marked entry. | [code] |
| Demo 0x3ce54 | "+ demo" | It is the 11-instruction setup. The demo is the game's own flow with 0xff8003 = 0. 0x4030c is 10 instructions with 1365 records. About 30 new after sharing, plus probed branches. | [code]; branches [inf] |
| Coins, credits, start | 0x3dec2–0x3e04c, ~270 | The range is the start screen only (74). The credits come from t7's 0x482a8–0x4834c. About 230 in total, plus the coin-control helpers above, of which only the two port writes are new. | [code] |
| Country select | 0x3e04c–0x3e39a, ~250 | About 339 including the handlers, the tail and the 2×2 writers. 15 pointers per handler, not 13. The fixed fight's select times out at 1865. | [code]; [cap] |
| Pre-fight | ~550 | 166 in range (0x3e5d6–0x3e960): 120 already translated and 46 left (the placement 0x3e5d6–0x3e6ca). About 56 more lie outside the range: 0x3e59c–0x3e5c2 (8), event 2's stage-0 test (2), event 3 (33) and 0x436b8 (13). About 102 left in all. The map is not in the fixed fight. Event 3 is the country screen and event 4 the stage intro. | [code] + [doc]; names [inf] |
| Match end … game over | 0x3ee26–0x3f374, ~350 | 262 in range, 44 translated, about 310 needed including event 10 and the objects. 0x42422 is the ranking check and name entry. | [code] |
| Country's end, stage step | not listed | 0x3ef96 is 8 instructions and 0x3efc4 is 5. 0x3ef80 increments 0xffc004 and 0x3ef96 clears it with no sleep between, so 0xffc004 = 2 never appears at a frame end. | [code] |
| Name entry | 0x42422, ~300 | 0x42422–0x429f1 plus helpers, about 400. | [code] |
| Two-player | ~150 + reuse | About 137, with different contents. Mode 4 is reached only after P2 wins a versus (written only at 0x3eeac). The challenger path jumps to 0x3e5d6 (at 0x4241c) and skips events 2 and 3. A game over with credits left branches to the start screen at 0x3f358, before the slot-2 kill. | [code] |
| Power/timing bonus | ~300 | About 366 per copy; copy 2 is a near-copy. Stage 0 → 0x40d30, stage 2 → 0x41a02. | [code] |
| 10-second bonus | ~250 + objects | About 395 including 66 handler instructions. Stage 1 → 0x41740, stage 3 → 0x41fe6. | [code] |
| Ending incl. map | ~250 | About 320–335. The scenes are indexed by the starting country (0x44c1c by 0xffc006, at 0x44a76). USA (country 2) is selectable at the default DIPs. "KING OF THE HILL" was not re-checked. | [code]; [doc] |
| Other opponents | extra routines per opponent | No new hit-function shape (67 functions). Entries only for six opponents. The handlers 0x43348, 0x43396 and 0x43652 are additions. | [code] |
| Ken as P1 | parallel hit functions to translate | Already translated, about 0 new. One-player Ken is mode 4. | [code] |
| Music CPU | ~670 untranslated | Translated (the effects player from 0x0a9b, about 325). 5 reported sites remain. | [doc]; [code] |
| Samples CPU | no new code expected | Complete, with no report sites. | [code] |
| Round tune loop | needs a round longer than ~50 s | Default rounds are about 31.7 s; the sound test reaches the loop. | [inf] |
| Latch latency (§3.1) | blocker | Resolved by the scope decision: band 45–138, launcher at 92. | [doc] |
| Flip | tilemaps not flipped | Natively nothing flips today (flip_screen is fixed at false). MAME's tilemap.cpp is not held locally. | [code] |
| Test mode | ~1.4k | The range 0x4834e–0x492d5 is about 870 ±30 (0x492d6–0x49aca is data). Its tickets F73–F76 sum to about 945, because they also take helpers outside the range and the exit trap. It reads unmapped ports. | [code] |
| Compare helper | 0x3bafe ~14, 0x3bb0e ~12, 0x3baf6 ~16 | One routine, 0x3baf6–0x3bb32, 18 instructions, with four entries: 0x3baf6, 0x3bafe, 0x3bb06 and the loop at 0x3bb0e. The 0x3bafe path is already translated inline in check_hiscore (fighter.rs:1544), for the top-score call at 0x3ba9a. No direct reference to 0x3bb06 was found. | [code] |
| 0x3b802 | 3 or 13 instructions | 2 new; its tail is the translated queue tail. At the default DIPs, queueing 0x0f through it behaves the same as through the filtered entry (0x3b7c8–0x3b7fa). | [code] |
| 0x3bb74, 0x3bdb0, sleep wrappers | listed as untranslated | Already translated (decimal_down; inline at tasks.rs:1336). The wrappers set D0 and trap #2. Resume state is needed for the nested sleeps 0x3b582, 0x3b58a and 0x3b596. It is also needed for 0x3b5f8, which sleeps at 0x3b62e with D1 and A0 on the stack (and at 0x3b60c in its unreachable repeat). | [code] |
| 0xff8003 at 0x3e5ae | "meaning not identified" | The demo / attract flag. | [code] |
| Checkpoint size | not listed | One full checkpoint (pixels plus the work, video, palette and music RAM dumps) is 419,042 bytes per run. The work dump covers object RAM. | [cap: fight-audit-run-a] |

### F78 re-check of waves 3–16 (F50–F77) (2026-10-31)

Every size, address, helper and blocker of F50–F77 was re-read against `tools/reference/local/sf_maincpu.dis`, `crates/sf-engine/src/{fighter,tasks}.rs`, `tools/reference/coverage_ledger.py` and the completed wave 0–2 docs after all agent-owned tickets in F1–F49 closed. No ticket's scope, size or blockers changed materially:

| Ticket | Re-check against listing and implementation | Evidence |
| --- | --- | --- |
| F50 | Insertion `0x425c4–0x4268a` is 51 instructions; marked-entry blink `0x42750–0x427e0` is 32 instructions; `0x3b904–0x3b908` is the 2-instruction 4-word fill entry before `0x3b90c–0x3b910` (2) and `F1`'s translated `0x3b914–0x3b932` (`fill_words_16`), 85–87 instructions in all ("About 86"). Blockers F16 (#56) and F42 (#82) are complete. | [code] |
| F51 | Size 0: `0xffc000` (`0x3f002` in `continue_start`) and `0xffc001` (`0x3f4a2` in `watch`) are translated by F2 (#42), F25 (#65) and F41 (#81). | [code] |
| F52 | Copy 1 `0x41740–0x419fc` is 137 reachable instructions (139 listing lines including the two unreachable duplicate `bra $418ac` instructions at `0x4192e` and `0x41932` after `0x4192a: bra $41950`), with resume point `0x41814` and game-mode-4 tests at `0x4175e`, `0x418ac` and `0x41962`. `0x44174 / 0x44192` are 14 instructions (7 + 7 before `0x441b0` `start_move`). Object handlers `0x434e4` (13), `0x4352c` (14) and `0x4357a` (13) total 40 instructions (each followed by an 8-byte BCD table at `0x43524`, `0x43572` and `0x435ba`). Sum: `137 + 14 + 40 = 191`. Shared helpers `0x3baf6`, `0x3b934` and `0x42f1e` are translated. Blockers F43 (#83) and F44 (#84) are complete (`sf-win-04` stops at `0x41740` on frame 17484). | [code] + [cap: win-04-run-a] |
| F53, F57, F59, F60, F62, F63 | The 44 remaining CPU hit-function entries in `0x46e56–0x480d0` partition as 9 (`0x46e56–0x471c6`, spark `0x18944`, for `0x175bc` in F53), 8 (`0x47234–0x47536`, spark `0x189a4`, for `0x1782c` in F57), 8 (`0x475a4–0x478a6`, including `0x47838`, for `0x17a9c` in F59), 7 (`0x47914–0x47ba8` for `0x17d0c` in F60), 6 (`0x47c16–0x47e3c` for `0x17f7c` in F62, plus `0x4558a–0x455d8` at 19 instructions) and 6 (`0x47eaa–0x480d0` for `0x181ec` in F63, plus `0x45ace–0x45aea` at 6 instructions and `0x43652–0x436a2` at 20 instructions with 8-byte reaction descriptors at `0x436a8` and `0x436b0`). | [code] |
| F54 | Size 0, `ready-for-human`; blockers F9 (#49), F22 (#62), F24 (#64) and F38 (#78) are complete. | [doc] |
| F55 | `0x3ce8c–0x3ceb8` is 9 instructions (`0x3ceb2` calls `F1`'s translated `0x3b230` `Scheduler::kill` for slot 2). F48 (#88)'s PC-coverage probe on `sf-attract-demo-01` through the demo's round end at frame 3636 found **0 untranslated fight-engine branches**, verified the 19-frame timer cadence with no frame overrun, and verified the 1,365-record table wrap at `0x0fff` (`docs/reference/attract.md` §5), resolving F55's oversized-branch risk to 0 additional instructions beyond the 9 at `0x3ce8c–0x3ceb8`. | [code] + [doc: attract.md] |
| F58 | Copy 2 `0x41a02–0x41f58` (resume `0x41aa6`, table `0x41f5e`, mode-4 test at `0x41a08`, helper `0x414f0` mode-2 test at `0x414f6`) is 283 reachable instructions, a near-copy of F29 (#69)'s `0x40d30–0x412a4` translated as parameters. | [code] |
| F61 | Copy 2 `0x41fe6–0x422b4` (mode-4 tests at `0x42004`, `0x42164` and `0x4221a`) is a near-copy of F52's `0x41740–0x419fc` (139 reachable / 141 listing lines), plus handlers `0x435c2–0x435fc` (13) and `0x4360a–0x43644` (13) = 26 new instructions for templates `0x18e84 / 0x18ea4`. | [code] |
| F64 | Ending `0x44a40–0x44c16` (resume `0x44a60`, table `0x44c1c`, mode-2 test at `0x44b48`, calling `F1`'s `0x3b7a8` at `0x44c10` and jumping to `F29`'s `0x3efc4` at `0x44c16`) is 110 instructions. Blockers F29 (#69) and F34 (#74) are complete. | [code] |
| F56, F65–F72 | Size-0 verification and audit slices; all addresses, checkpoint rules and wave 0–2 blockers confirmed against the completed implementations. | [code] + [doc] |
| F73–F77 | Test mode `0x4834e–0x492d5`: F73 has reset branch `0x3b01e–0x3b02a` (4), RAM test `0x4834e–0x483f2` (50), task `0x483f8–0x48642` (145), exit trap #3 `0x3b29a–0x3b2b2` (7), and helpers `0x3b5c8–0x3b5f6` (10), `0x3bffc–0x3c032` (22) and `0x3b6b4–0x3b6c4` (6); F74 has menu `0x48644–0x487f6` and items `0x487f8–0x48b68` (about 290); F75 has sprite test `0x48b6a–0x48da2` and output cycle `0x490f8–0x492d5` with `0x3b93c` and `0x3b664` (about 225); F76 has scroll test `0x48da4–0x490f6` (about 178); F77 is size 0. | [code] |

## 2. Conventions every ticket uses

- **C-REF.** Every capture a ticket names is recorded by that ticket on the
  pinned MAME 0.289 from its scenario.
  - It has two runs that are identical file for file (compare_captures).
  - It holds inputs only, with no RAM edits.
  - It states its DIP settings.
  - It states its checkpoint windows. A checkpoint window is a span of
    frames with a full checkpoint on every frame. A checkpoint costs about
    419 KB per run [cap].
  - Original game data stays local.
- **C-NAT.** The native run is compared after every frame.
  - Inside a checkpoint window the comparison is full: the screen hash, the
    checkpoint pixels, video RAM, palette RAM, the latch values and their
    frames, and owned work RAM, which must be byte-identical.
  - Outside a checkpoint window the comparison covers the frames.tsv fields
    only: the screen hash, the sound latch and the latch-changed flag. A
    ticket never reports a full match on a frame without a checkpoint.
  - Any other work-RAM difference may only be in the task slots' saved
    SR/PC/USP words and the stack memory 0xff8106–0xff8631. This is
    milestone-audit.md's criterion [doc].
  - Owned regions override that exception. An owned address inside
    0xff8106–0xff8631 must match. 0xff8622 is one: it is the boot counter at
    0x3c0ce and the top of slot 7's stack [code].
  - Each ticket adds the RAM regions it starts owning to the owned lists.
- **C-STOP.** The first untranslated branch a run reaches is reported with
  its frame and address, never guessed. A ticket passes only on the frames
  before that branch, unless its acceptance requires the window to be
  complete.
- **C-NEG.** A named negative control fails at its first differing frame.
- **C-DOC.** Every implementation ticket updates a docs/reference/*.md with
  the verified boundary, the label and a "Not covered" list.
- **Labels** (the evidence class of a ticket's result):
  - **PO**: complete gameplay from power-on, with no fixture.
  - **DF**: a development fixture (a named reference checkpoint), then real
    inputs and the live CPU. This is complete gameplay from that frame.
  - **CF**: a controlled fixture: a scripted or idle side, or a forced DIP
    variant.
  - **SM**: a controlled service-mode reference.
  - **TOOL**, **HOST**: tooling or host-side work.
  - **PERSON**: needs the user. Published as ready-for-human.
  - **Audit**: an account, with no new translation.
  - Every ticket that is not PERSON is published as ready-for-agent. F18
    is among them: decision 4 allowed fetching tilemap.cpp.
- **Size** counts newly translated original instructions only. It excludes
  verification, tooling and host code.
  - A size-0 ticket that reaches an untranslated branch reports it (C-STOP).
  - It may translate that branch if it is under about 30 instructions. A
    larger one becomes a new ticket. This bound is proposed for approval
    (open decision 13).
  - F65, F66 and F71 never translate: any branch they reach fails them.

## 3. Ownership of shared code

Each piece has one owner. The others reuse it and do not translate it
again.

| Code | Owner | Reused by |
| --- | --- | --- |
| Scheduler pieces [code]: spawn (trap #0) 0x3b20a–0x3b22e, kill (#1) 0x3b230–0x3b24c, slot address 0x3b2b8–0x3b2c8, new-task entry 0x3b1e8–0x3b1f8, and the rescan from slot 0 (0x3b19a) | F1 | F15, F25, F33, F34, F48, F55, F73–F76 |
| Exit (trap #3) 0x3b29a–0x3b2b2. Its one caller, 0x48618, is in test mode [code]. | F73 | none |
| Reset 0x3b000–0x3b04e (with the self-test check), coinage 0x3b2e4, boot init 0x3b77a, queue fill 0x3b914, the 0x3b7a8 boot commands, the unfiltered entry 0x3b802, palette upload 0x3b5f8 (its repeat 0x3b602–0x3b616 reported, not translated) | F1 | F12 (0x3b802, see the note below), F15, F17, F35, F39 (0x3b7a8 at 0x3f166), F50 (0x3b914 at 0x427b0), F64 (0x3b7a8 at 0x44c10), F73 |
| Coin-control helpers 0x3b654 (OR) and 0x3b6a2 (AND-NOT), factored out of t7 (tasks.rs:2030, 2034), with the 0xc00010 port write and MAME's coin lockout | F12 | F15 (0x3c26a), F33, F35, F39, F49, F66, F73 (0x3b5e2) |
| The live DSW1 in the queue filters 0x3b7cc / 0x3b7f2; DSW1 / DSW2 taken from the scenario | F2 | F17–F24, F35, F37, F47, F51, F67–F70 |
| Slot 3's attract texts 0x3f3a4–0x3f40e | F3 | F15, F33 |
| The boot-to-attract join: 0x3ccae–0x3ccbc, the loop head 0x3ccc2–0x3cce6, slot 3's entry 0x3f374–0x3f386 | F15 | F25 (return to attract), F55 |
| 0x3e59c–0x3e5c2, the player placement 0x3e5d6–0x3e6ca (modes 1, 2, 4), 0x436b8, event 2's stage-0 test, event 3 0x3f8e4 | F4 | F26, F39, F40, F41, F43, F46, F48 |
| Country head and two-country screen, exit 0x3e372, the four handlers 0x3e39a–0x3e585, the slot-2 spawn 0x3e586–0x3e596, the 2×2 writer 0x3bc3e and eraser 0x3bd3e | F34 | F47, F48, F64 |
| 0x42422's ranking check and not-qualified exit; the compare helper 0x3baf6–0x3bb32 (four entries, factored out of check_hiscore's inline copy at fighter.rs:1544); 0x3b934; the CPU match win 0x3eef0–0x3ef54 (with its mode-4 twin 0x3ef2c) | F5 | F25, F26, F27 (0x3bb0e), F40, F42, F52 (0x3baf6) |
| 0x42f1e (absolute spawn), 0x4329c (live-effect count), 0x42dd2 (the 0xff stream call), 0x3f2d0–0x3f372 (versus and after-ending game over, the reset, and 0x3f358's branch to 0x3dec2 with credits left) | F25 | F42, F43, F45, F52, F64, F65 |
| t7's coin arrival, pulse and service paths, the start-screen loop, 0x3bb5c | F12 | F33, F35, F39, F49, F66 |
| t7's attract start 0x48228–0x48248, the start-screen entry 0x3dec2–0x3df48 | F33 | F45, F49, F66 |
| Ranking screen 0x4268c–0x4274e | F16 | F50 |
| The country's-end dispatch 0x3ef96 | F27 | F44, F57, F60, F63 |
| The stage step 0x3efc4 | F29 | F43, F52, F58, F61, F64 |
| One native machine for every game mode (both players' inputs, any fight-loop checkpoint) | F26 | F40, F41, F42, F46, F56, F66 |
| The map 0x3f812… and the plane's stream routines | F43 | F52, F58, F61 |

**Note on 0x3b802.** F1 owns this 2-instruction entry. F12 also needs it,
for the 0x0f it queues at 0x48206 and 0x482da. F12 does not wait for F1:
at the default DIPs, which every F12 scenario uses, the translated filtered
entry queues 0x0f the same way [code 0x3b7c8–0x3b7fa]. Whichever of F1 and
F12 merges second points those two call sites at 0x3b802. F68 is the first
scenario that tells the two entries apart.

## 4. Tickets in dependency order

Ids are local (F1…). Every blocking edge points to a lower id. "Blocked by"
lists genuine gates only. The first draft's ids are mapped in the appendix.

| Id | Title | Size | Label | Blocked by |
| --- | --- | --- | --- | --- |
| F1 | Power-on: reset, task spawning and the boot set-up frames | ~177 | PO | none |
| F2 | DIP switch settings in reference scenarios and the native inputs | ~3 + tooling | TOOL | none |
| F3 | Attract wall, title and INSERT COIN from the attract-entry fixture | ~83 | DF | none |
| F4 | Pre-fight screens of the fixed fight from country select's exit | ~102 | DF | none |
| F5 | CPU match win: lose quote, ranking check and continue countdown | ~140 | DF | none |
| F6 | Winning-script search through Japan into stage 1 | 0 | TOOL | none |
| F7 | Ken's attacks against an idle Ryu | 0 [inf] | CF | none |
| F8 | Native-latch sound over every recorded fight | 0 | TOOL | none |
| F9 | Listening kit for the fixed fight | 0 | HOST | none |
| F10 | Record a keyboard session and verify it against the reference | 0 | HOST | none |
| F11 | Translation coverage ledger from every pinned reference | 0 | TOOL | none |
| F12 | Coins, service credits and 1P/2P start on the start screen | ~121 | DF | none |
| F13 | Ryu's Hadoken and Tatsumaki against Retsu | 0 [inf] | DF | none |
| F14 | Crouching attacks, jump attacks and blocks against Retsu, and held inputs | 0 [inf] | DF | none |
| F15 | The warning screen from power-on, into the title | ~50 | PO | F1, F3, F12 |
| F16 | Attract ranking screen | ~47 | DF | F3 |
| F17 | The boot DIP decode for every setting | ~0 | PO, CF | F1, F2 |
| F18 | Flip screen on every layer | 0 + renderer | DF / CF | F2 |
| F19 | Difficulty, round-time and attract-music DIPs in the fixed fight | 0 | DF, CF | F2 |
| F20 | The Speed DIP's slow game | 0 | DF / CF | F2 |
| F21 | The Freeze DIP | ~5 + tooling | DF / CF | F2 |
| F22 | Every FM effect and special command from the service sound test | 0–30 | SM | F2 |
| F23 | Every tune through its loop point or its end | 0–30 | SM | F2 |
| F24 | Every ADPCM sample on both channels and every bank | 0 | SM | F2 |
| F25 | Game over and the return to the attract boundary | ~132 + probe | DF, CF | F1, F5 |
| F26 | A versus winner carries on against the CPU (one machine for every mode) | ~9 + harness | CF, DF | F4, F5 |
| F27 | Ryu beats Geki (0x16bfc) | ~98 + 4 entries | DF | F5, F6 |
| F28 | Ryu beats the first stage-1 opponent (0x1734c) | 8 entries | DF | F6 |
| F29 | Power/timing bonus after Japan (0x40d30) | ~370 | DF | F6 |
| F30 | Listening review of the fixed fight | 0 | PERSON | F9 |
| F31 | A person's keyboard session of both fixtures | 0 | PERSON | F10 |
| F32 | One-player round 3 after a 1–1 split, a time-over and a draw | 0 [inf] | DF | F5, F6 |
| F33 | A coin during the title opens the start screen | ~36 | DF | F1, F3, F12 |
| F34 | Two-country select and the opponent order | ~242 | DF | F1, F12 |
| F35 | Coinage DIP: coins at non-default rates | 0 | DF / CF | F1, F2, F12 |
| F36 | Remappable keyboard bindings with persistence | 0 (host) | HOST | F12 |
| F37 | Demo Sounds and Attract Music DIPs over the attract | 0 | DF / CF | F2, F16 |
| F38 | Tune transitions: priorities, the saved tune and the stops | 0 | SM | F23 |
| F39 | Continue with a credit: the rematch against the same opponent | ~17 | DF | F1, F4, F5, F12 |
| F40 | P2 wins the versus and plays the CPU (game mode 4) | ≤50 [inf] | CF, DF | F26 |
| F41 | A challenger joins a one-player fight | ~33 | DF, CF | F12, F26 |
| F42 | Name entry for a ranking player | ~204 | DF, CF | F25, F26 |
| F43 | Country to country: the first bonus to stage 1's first fight | ~54 | DF | F4, F25, F29 |
| F44 | Ryu beats the second stage-1 opponent (0x170dc) | 4 entries | DF | F27, F28 |
| F45 | Game over with a credit left returns to the start screen | 0 | DF | F25, F33 |
| F46 | A fresh two-player start into the versus fight | 0 [inf] | DF, CF | F4, F26, F34 |
| F47 | Four-country select (DIP) | ~86 | DF / CF | F2, F34 |
| F48 | Attract demo: setup through FIGHT! | ~30 + calls | DF | F4, F16, F34 |
| F49 | The fixed fight from power-on, with no fixture | ~0 | PO | F4, F15, F33, F34 |
| F50 | Ranking insertion and the new entry on the ranking screen | ~86 | DF / CF | F16, F42 |
| F51 | Continuation and buy-in DIPs | 0 | DF / CF | F2, F25, F41 |
| F52 | 10-second bonus after stage 1 (0x41740) | ~191 | DF | F43, F44 |
| F53 | Ryu beats stage 2's first opponent (0x175bc) | 9 entries | DF | F44 |
| F54 | Listening review of every tune, effect and sample | 0 | PERSON | F9, F22, F24, F38 |
| F55 | Attract demo to its round end, and the attract loop from power-on | 9 + probe | PO, DF | F15, F48 |
| F56 | Name entry end to end from a won one-player fight | 0 | DF, CF | F41, F50 |
| F57 | Ryu beats stage 2's second opponent (0x1782c) | 8 entries | DF | F53 |
| F58 | Power/timing bonus after stage 2 (0x41a02) | 283 near-copy | DF | F29, F43, F57 |
| F59 | Ryu beats stage 3's first opponent (0x17a9c) | 8 entries | DF | F57 |
| F60 | Ryu beats stage 3's second opponent (0x17d0c) | 7 entries | DF | F59 |
| F61 | 10-second bonus after stage 3 (0x41fe6) | ~26 + 140 near-copy | DF | F52, F60 |
| F62 | Ryu beats Adon (0x17f7c) | ~19 + 6 entries | DF | F60 |
| F63 | Ryu beats Sagat (0x181ec) and reaches the ending | ~26 + 6 entries | DF | F62 |
| F64 | The ending (0x44a40) | ~110 | DF | F29, F34, F63 |
| F65 | A complete one-player game from power-on | 0 | PO | F49, F50, F58, F61, F64 |
| F66 | A two-player session from power-on | 0 | PO, with CF spans | F25, F39, F40, F41, F46, F49, F55 |
| F67 | DIP settings from power-on: flip and the country count | 0 | PO / CF | F18, F47, F49 |
| F68 | DIP settings from power-on: attract and demo | 0 | PO / CF | F33, F37, F55 |
| F69 | DIP settings from power-on: the fight | 0 | PO / CF | F19, F20, F21, F49 |
| F70 | DIP settings from power-on: coinage, continuation and buy-in | 0 | PO / CF | F35, F49, F51 |
| F71 | Both sound programs over the complete game from the native latch | 0 | PO | F8, F22, F23, F65, F66 |
| F72 | Full-game completeness and fidelity audit | 0 | Audit | F7, F11, F13, F14, F17, F30, F31, F32, F36, F45, F54, F56, F67, F68, F69, F70, F71 |
| F73 | Service mode from power-on: RAM test, input display and sound test | ~252 | SM | F72 |
| F74 | Service menu and its first four screen tests | ~290 | SM | F73 |
| F75 | Service sprite test and output cycle | ~225 | SM | F74 |
| F76 | Service scroll test | ~178 | SM | F74 |
| F77 | Append the test-mode result to the audit | 0 | Audit | F75, F76 |

### F1. Power-on: reset, task spawning and the boot set-up frames
- **Delivers.** A power-on machine with no fixture, built from the program
  ROM alone. It covers:
  - **The reset path** (0x3b000–0x3b04e): the RAM clear with interrupts
    masked; the self-test check, which reports 0x4834e; coinage 0x3b2e4;
    boot init 0x3b77a with the queue fill 0x3b914, the boot commands 0x27 /
    0x3f / 0x80 through 0x3b7a8 and the unfiltered entry 0x3b802; slot 0
    spawned; and the SR write that releases the held vblank.
  - **The scheduler pieces**: spawn, kill, the slot address, the new-task
    entry, and the rescan from slot 0 after every yield. The rescan replaces
    today's single in-order pass. Every sleep jumps to 0x3b19a [code]. Exit
    belongs to F73.
  - **The boot task's first half** (0x3c0c4–0x3c20a): the four palette
    uploads (0x3b5f8, which sleeps at 0x3b62e with D1 and A0 on the stack),
    the object and text clears, the DIP decode 0x3c132–0x3c1bc, the ranking
    table from 0x14598, the top score, the player-structure images, and the
    chunk's overrun across a frame boundary.
  - **Reported, not translated**: 0x3b5f8's repeat 0x3b602–0x3b616. It runs
    only when SYSTEM bit 6 reads low, which MAME's inputs cannot produce
    [code 0x3b638–0x3b640; sf-mame0289.cpp:561].
  - **Machine changes**: 0x3bdb0 factored out of its inline copy
    (tasks.rs:1336–1338 [code]). `Io` gains `dsw2`, defaulting to the
    observed 0xffff [code scenarios/sf_cpu_01.json:22].
  - **The 0x0f call sites**: if F12 has merged first, F1 points F12's two
    0x0f calls, at 0x48206 (service) and 0x482da (coin), at 0x3b802 [code].
    Section 3's note on 0x3b802 explains why.
- **Acceptance.**
  1. C-REF: sf-boot-01 (power-on, no inputs, default DIPs, frames 0–829,
     checkpoints every frame 0–799). Its frames.tsv rows equal fight-run-a's
     on every frame it covers.
  2. C-NAT from power-on, every frame from 0 through the boot task's sleep
     after 0x3c20a (about frame 8 [inf]):
     - the latch reads 0x00, 0x27, 0x3f, 0x80 at frames 0–3 [cap], and the
       frame word equals the frame number;
     - the owned RAM adds 0xffc000–0xffc007, 0xff8f28, 0xff8e1c, 0xff863c /
       0xff8644, 0xff8e6e–0xff8f0d, the top score, the player structures,
       0xff8622, the gfxctrl copy 0xff8682, and slot 0's state and sleep
       bytes.
  3. MAME's power-on contents are read from sf-boot-01's frame-0 dumps
     only where the reset cannot have changed them by then:
     - object RAM 0xffe000–0xffffff, which the reset never clears;
     - video RAM and palette RAM, which nothing writes before the boot task.

     Work RAM 0xff8000–0xffdfff cannot show them in full. The reset clears
     it (0x3b012–0x3b01a [code]) with vblank 0 held. By summed costs the
     clear is still running at frame 0's end [inf: about 184,000 cycles,
     against about 133,000 per frame at 8 MHz]. Its part not yet cleared
     at frame 0 is read too. The point the clear has reached is placed by
     acceptance 4's costs. Where the dump shows that point, the placement
     is checked against it, not fitted.

     The native power-on state assumes zero in every region [inf]. Any
     difference is reported, not fitted.
  4. The two overruns are placed from summed 68000 instruction costs and
     tabulated in the reference doc, not fitted:
     - vblank 0 held across the RAM clear (MAME's irq1_line_hold,
       sf-mame0289.cpp:758);
     - 0x3c112–0x3c204 running across a frame boundary.

     A disagreement with the capture is reported as a scope question.
  5. Every existing scene, round, match and timer test, and the milestone
     audit, pass unchanged under the rescan scheduler.
  6. Kill is translated here and labelled unexercised. No unit test stands
     in for a scenario. F33 (0x48232), F25 (0x3f368) and F55 (0x3ceb2)
     exercise it first.
  7. The run reports 0x3c210 at the next frame.
  8. C-NEG: one palette upload moved a frame later is caught.
- **Size.** About 177 [code: reset 18; coinage 15; boot init 11 and boot
  commands 7; queue fill 5; 0x3b802 2; scheduler 29 (spawn 10, kill 8, slot
  address 6, new-task entry 5); task first half 76; palette upload 14, which
  is 19 less the reported 5]. Near the upper end of one context, because of
  the power-on machine and the cycle tabulation.
- **Label.** PO, no input.
- **Blocked by.** None.

### F2. DIP switch settings in reference scenarios and the native inputs
- **Delivers.**
  - A scenario field for DSW1 / DSW2, as port words or MAME field settings.
  - The generated Lua driver applies it before the 68000's reset. The
    ticket chooses between a field write plus a reset and a generated cfg
    that replaces -noreadconfig, from the pinned MAME's behaviour [inf].
  - meta.tsv's readback at frame 0 and at the end is checked against the
    request.
  - The native `Io` takes dsw1 / dsw2 from the scenario.
  - The filters at 0x3b7cc / 0x3b7f2 read the live DSW1 instead of
    `DSW1_DEFAULT` (fighter.rs:85 [code]).
- **Acceptance.**
  1. The loader's tests reject unknown DIP names and out-of-range values. A
     request the driver did not apply fails the run (C-NEG).
  2. Every existing scenario regenerates a byte-identical driver.
     sf-fixed-fight-01, re-run with its defaults stated explicitly, equals
     fight-run-a file for file.
  3. C-REF smoke scenarios, each with a readback equal to its request. All
     three start at power-on with no inputs.
     - Countries = 4 with coinage 2C1C / 1C6C: 20 frames, checkpoints every
       frame 0–20. The boot DIP decode ends near frame 8 [inf], so the dumps
       show the decoded bytes that F17 lists.
     - Flip On: 20 frames, checkpoints every frame 0–20. Bit 2 of 0xff8682
       is set after the decode [code 0x3c1ac–0x3c1bc].
     - Service On: checkpoints every frame from 0 until the test screen
       shows. The RAM test ends by filling video RAM (0x800000–0x800fff)
       with 0x103c (0x483d0–0x483d4), the word the text clear 0x3bdb0 also
       writes [code]. The test
       screen shows at the first frame end at which video RAM holds a word
       other than 0x103c, after a frame end at which it held only 0x103c.
       That frame is recorded [cap]. F73 checks its own run against it.
  4. Every native test passes with the DIP words taken from the scenario.
- **Size.** About 3 (the live DSW1 reads), plus tooling.
- **Label.** TOOL.
- **Blocked by.** None.

### F3. Attract wall, title and INSERT COIN from the attract-entry fixture
- **Delivers.**
  - Slot 1's attract loop, 0x3ccec–0x3cdc4 and 0x3ce30–0x3ce4e: palettes,
    layers, the ten scroll entries from 0x3cde0, the three title blocks from
    0x3cdc8, the logo hold, the 300-frame wait and the call to the ranking.
  - Slot 3's attract path 0x3f3a4–0x3f40e: the copyright line every frame,
    and INSERT COIN 20 frames on and 20 off through 0xff8f20 and D2.
  - Fixture-decoder positions for these loops, including the nested sleep
    0x3b582 / 0x3b588 and the loop registers read from the register image.
- **Acceptance.**
  1. C-REF: sf-attract-01 (power-on, no inputs, 1,900 frames, checkpoints
     every frame 794–1,900). Its rows 0–1449 equal fight-run-a's, and its
     checkpoint 794 equals fight-run-a's.
  2. The 794 fixture decodes to slot 0 at 0x3f7ac, slot 1 at 0x3ccec, slot
     3 at 0x3f3a2 (continuation 0x3f3d8) and slot 7 at 0x4819e [cap]. Any
     other position is refused.
  3. C-NAT from 794 over 795–1647, with 0x1c latched at 976 [cap]. At 1648
     the run reports 0x4268c.
  4. C-NEG: slot 3's D2 changed by one is caught at the first INSERT COIN
     toggle. A scroll-entry sleep one frame short is caught at that step.
  5. Records corrected:
     - attract.md records the timeline and mame-baseline.md's approximate
       timeline is corrected;
     - first-frame.md: 794 is a consistent state [cap];
     - sf_fixed_fight_01.json's checkpoint note calls 794 a "transition
       frame"; it is corrected to a consistent state [cap].
- **Size.** About 83 [code].
- **Label.** DF at 794, with no input.
- **Blocked by.** None.

### F4. Pre-fight screens of the fixed fight from country select's exit (1865–2389)
- **Delivers.**
  - 0x3e59c–0x3e5c2: the clears, the demo test and the event-2 request and
    wait.
  - Event 2's stage-0 test (0x3f808–0x3f80e).
  - Event 3, 0x3f8e4–0x3f97a, with 0x436b8.
  - The placement 0x3e5d6–0x3e6ca for modes 1, 2 and 4, handing over to the
    translated 0x3e6ca.
  - The decoder learns slot 1 at 0x3e59c.
- **Acceptance.**
  1. C-REF: sf-prefight-01 (the fixed fight's inputs, 2,400 frames,
     checkpoints every frame 1860–2400).
     - Its frames.tsv equals fight-run-a's over 0–2399.
     - Its checkpoints at 1860, 1980, 2160 and 2280 equal fight-run-a's.
     - Its checkpoint 2389 equals cpu-01-run-a's.
  2. The 1865 checkpoint decodes in full. Otherwise the fixture moves to the
     first frame that does, and the frame is recorded. Slot 2 is expected at
     0x3f7d6 [cap, per the re-check].
  3. C-NAT over 1866–2389:
     - the latch: 0x8f at 1868, 0x33 at 1959, 0x97 / 0x0b at 2117–2118,
       0x27 / 0x3f at 2275 / 2277, 0x28 at 2280 and 0x94 at 2361 [cap];
     - the owned RAM, which adds the player structures' +0xa, +0xc, +0xe,
       +0x10 and +0x20, the fighters' +0x38 / +0x3c, slot 2's bytes and
       0xff8633–0xff8639.
  4. At 2389 the owned RAM equals the round-1 fixture's. The run continues
     and matches sf-fixed-fight-audit-01 (checkpoints every frame
     2389–5040) through 5014.
  5. The 1900 start press falls inside event 3. A reference-side check
     shows whether it matters: a second capture without the 1900 press is
     compared with sf-prefight-01 on every frames.tsv field and checkpoint
     over 0–2400 [cap]. If they differ, the difference is recorded. F4 then
     passes over 1866–1899 only, because its native run has no start input
     (run.rs:228 drops it until F12). The comparison over 1900–2389 and
     acceptance 4 move to F49, which feeds start through F12's inputs and is
     blocked by F4.
  6. The map 0x3f812 and the demo branches 0x3e5c6 / 0x3e6fe are reported.
     The mode-1 and mode-4 placements are labelled unexercised.
  7. C-NEG: event 3's 60-frame hold shortened by one.
  8. Record: sf_fixed_fight_01.json's note calls 1980 the "VS screen". The
     re-check reads it as event 4's stage intro [doc + cap, per the
     re-check]. This ticket confirms the screen from its run and corrects the
     note.
- **Size.** About 102 [code: 8 + 46 + 2 + 33 + 13].
- **Label.** DF at 1865, then the live CPU.
- **Blocked by.** None.

### F5. CPU match win: lose quote, ranking check and continue countdown (5015–6100)
- **Delivers.**
  - The CPU's match win 0x3eef0–0x3ef54: mode 2 at 0x3eefa, and the mode-4
    twin 0x3ef2c, which is unverified here.
  - Event 0xb's CPU branch and the wipe.
  - 0x42422's ranking check through the compare helper, and the
    not-qualified exit with 0x3b934.
  - The compare helper 0x3baf6–0x3bb32, factored out with its four entries
    as F1 does with 0x3bdb0. check_hiscore's inline copy of the 0x3bafe path
    (fighter.rs:1544) then calls it. 0x3bb06 has no direct reference found;
    it is labelled unexercised.
  - The continue screen 0x3efda–0x3f162: scene byte 8, timer counter 1, the
    CONTINUE blink, the big and small digits, and the credit-and-start test.
  - Reported: 0x3f1bc (expiry), 0x3f166 (continue taken), 0x42458 (a
    qualifying score) and 0x3f2f0 (the continue limit).
- **Acceptance.**
  1. C-REF: sf-match-end-01 (the fixed fight's inputs, 6,400 frames,
     checkpoints every frame 5000–6400). It equals the audit capture through
     5040, and music-03-audio-a's frames.tsv over 0–6400.
  2. C-NAT from 2389 through 6100:
     - the latch: 0x9d at 5076, 0x3f at 5415, 0x3c at 5418 [cap];
     - the owned RAM adds 0xff8e16–0xff8e35, 0xff8e52…, P1's cleared score
       and 0xffc000–0xffc00b;
     - 0x3f1bc is reported at the reference's expiry frame.
  3. sf-round2-02 continues natively past 0x3eefa and matches through 5600,
     including its continue screen from 5200.
  4. Records corrected:
     - 0x42422 is named the departing player's ranking check and name entry
       (tasks.rs:666, round-outcomes.md, match-win.md);
     - the timer-flag labels at tasks.rs:402–404 and in round-timer.md: bit
       1 continue, bit 2 name entry, bit 3 ranking, bits 4–5 select and bonus
       [code];
     - the stale comment at tasks.rs:839–843.
  5. C-NEG: counter 1 started one pass late.
- **Size.** About 140 [code: match win 18, the 0x42422 check 31, the
  helper's two new entries 4, 0x3b934 5, continue screen 80]. The helper's
  0x3bafe path and loop are already translated inline.
- **Label.** DF at 2389. The CPU wins.
- **Blocked by.** None.

### F6. Winning-script search through Japan into stage 1 (sf-win-02)
- **Delivers.**
  - A reference-side search tool. It extends a pinned input prefix fight by
    fight, using a parametrized strategy family. It reads the outcome
    (0xffc009, 0xffc003 / 4, health) and never writes emulated memory.
  - sf-win-02: sf-win-01's inputs through Geki's round-1 start, a found
    segment that wins Geki, fixed bonus-1 inputs, and on to the round-1
    start of 0x1734c.
- **Acceptance.**
  1. The tool is documented in tools/reference/README.md. Re-running a
     candidate gives the same outcome.
  2. C-REF on sf-win-02, with checkpoints every frame over one continuous
     window in two parts:
     - Geki's round-1 start through the bonus start: Geki's fight, his
       match-won events 0xb and 6 (0x3ef5e–0x3ef6c) and the dispatch
       0x3ef96 [code]. F27 compares over it;
     - the bonus start through 0x1734c's first fight-loop frame: the bonus,
       the flight (0x3f812–0x3f8e0), event 3, stage 1's intro and ROUND 1.
       F43 compares over it.

     It equals sf-win-01 on every field and shared checkpoint through the
     divergence frame.
  3. The dumps show these frame-end values [code 0x3ef54–0x3efd6]:
     - 0xffc009 = 1 and 0xffc004 = 1 during Geki's events 0xb and 6;
     - the flow task inside 0x40d30 with 0xffc004 = 0;
     - 0xffc003 = 1 after 0x3efc4;
     - the flow task at 0x3ea3e for 0x1734c.
  4. C-DOC records the fixture frames later tickets use.
- **Size.** 0 (tooling).
- **Label.** TOOL, with a PO reference.
- **Risk.** One strategy family may not beat every later opponent at the
  default difficulty [inf]. The fallback (decision 5): widen the strategy
  family first; only if that fails, let the run use continues, and F39
  then joins the chain.
- **Blocked by.** None.

### F7. Ken's attacks against an idle Ryu (versus fixture)
- **Delivers.** New versus scenarios in which Ken lands and whiffs every
  normal, crouching and jump attack, the Shoryuken, the Tatsumaki (including
  chip damage on a blocking Ryu) and the Hadoken. They exercise his
  translated twins [code fighter.rs:149–189].
- **Acceptance.**
  1. C-REF on each, with checkpoints every frame from 2450 to the
     scenario's end.
  2. C-NAT from the 2450 fixture on each.
  3. All 16 twin hit functions land at least once. 0x453b6 with its tables
     0x4545c / 0x45474, and the trackers 0x4554a / 0x45566, are exercised.
  4. C-NEG: one of Ken's inputs moved by one frame is caught.
  5. C-DOC labels the results controlled.
- **Size.** 0 expected [inf].
- **Label.** CF.
- **Blocked by.** None.

### F8. Native-latch sound over every recorded fight
- **Delivers.** Both sound programs on the native latch over sf-win-01 and
  the versus punch run, a round-1 MSM5205 comparison for the fixed fight,
  and a per-fight table of the latency windows in which each device
  matches.
- **Acceptance.**
  1. New hook runs, two each, identical.
  2. At the reference's own write times every device is identical.
  3. A sweep of constant latencies 45–138 is tabulated per fight, with the
     first differing frame where no single window fits.
  4. The finding is presented as a scope note. The launcher stays at 92
     unless the user decides otherwise (open decision 6). No tolerance is
     adopted.
- **Size.** 0.
- **Label.** TOOL.
- **Blocked by.** None.

### F9. Listening kit for the fixed fight
- **Delivers.** Local renders of the native mix at latencies 92 and 51,
  MAME's audio.wav aligned to them, the launcher's live output captured
  during a scripted run, and a committed protocol with a verdict table.
- **Acceptance.**
  1. The 51 render's round-2 measures equal both-chips.md's, which are for
     round 2 at latency 51: onset 180 ms, RMS ratio within 2%, correlation
     at least 0.98 within 8 samples of lag [doc].
  2. The 92 render's measures are recorded, with no pass threshold.
  3. The live capture equals the batch mix sample for sample.
  4. The renders stay local.
- **Size.** 0.
- **Label.** HOST.
- **Blocked by.** None.

### F10. Record a keyboard session and verify it against the reference
- **Delivers.** A launcher record mode (per-frame logical inputs, focus
  changes, frame times), a converter to scenario JSON, and a verify command
  (a native replay against two MAME runs).
- **Acceptance.**
  1. A scripted synthetic session of at least 1,000 frames with every key
     goes through `Bindings::inputs`. It starts at the one-player fixture
     (sf-cpu-01's 2389). It converts to a scenario whose two MAME runs are
     identical, with checkpoints every frame from 2389 to the session's
     end. For every session it converts, the converter writes a window from
     the session's fixture frame (2389, or 2450 for the versus fixture) to
     its end.
  2. The native replay matches to the session's end, with the full
     comparison on every frame, or reports its first untranslated branch.
  3. The mean and maximum frame times are reported.
  4. C-NEG: one input moved by a frame.
- **Size.** 0.
- **Label.** HOST.
- **Blocked by.** None.

### F11. Translation coverage ledger from every pinned reference
- **Delivers.** A PC-coverage probe of the 68000 and both Z80s over every
  pinned scenario, and a generated ledger that maps:
  - each executed original address to the Rust that cites it;
  - each translated routine to the scenarios that execute it;
  - each note() / unrecovered site to reached or unreached.
- **Acceptance.**
  1. Two coverage runs per scenario are identical.
  2. On the tree of the day it reproduces the documented gaps and lists any
     executed, untranslated address that no doc names.
  3. It is generated by one command.
  4. C-NEG: a build with one translated routine's citation removed makes
     the ledger list that routine's executed addresses as untranslated.
- **Size.** 0.
- **Label.** TOOL.
- **Blocked by.** None.

### F12. Coins, service credits and 1P/2P start on the start screen
- **Delivers.**
  - **Translated**:
    - coin arrival 0x482a8–0x48324, with the award stopping at 9 credits
      (0x48308);
    - 0x3bb5c;
    - the coin-control helpers 0x3b654 (OR) and 0x3b6a2 (AND-NOT), factored
      out of t7's inline RAM updates (tasks.rs:2030, 2034), with the
      0xc00010 port write that MAME's coin_w turns into the coin lockout
      (sf-mame0289.cpp:110–116);
    - the counter pulse 0x48326–0x4834c (resume 0x48344, with D1 / D2 / A1
      kept as task state);
    - the service credit 0x481e2–0x4820c (resumes 0x481e8 / 0x48212);
    - the start-screen loop and exit 0x3df4c–0x3e020;
    - the credit digits 0x3be8a and their erase 0x3bd96;
    - the decoder accepting slot 1 at 0x3dfce.
  - **Inputs**: coin1 / coin2 / start1 / start2 / service1 become native
    logical inputs. run.rs:228 drops them today [code]. MAME's coin lockout
    is applied as the reference shows.
  - **The 0x0f sound**: the coin (0x482da) and service (0x48206) paths
    queue 0x0f through 0x3b802 [code]. If F1 has merged first, F12 points
    both call sites at F1's 0x3b802. Otherwise it uses the filtered entry,
    and F1 points them when it merges. Section 3's note on 0x3b802 explains
    why the two entries agree at the default DIPs.
  - **Captures**:
    - sf-credit-select-01: fight-run-a's inputs, checkpoints every frame
      1440–1870.
    - sf-credits-01, at the default coinage (1C1C on both slots). Each
      input is held 12 frames. Checkpoints every frame from 1440 to its
      end.
      - coin1 at 1450, as in fight-run-a: 1 credit, and the start screen
        by 1500 [cap: 0x3f / 0x80 at 1469 / 1470 in fight-run-a];
      - start2 at 1540 with 1 credit: ignored [code 0x3dfda–0x3dfe2];
      - coin2 at 1580 (2 credits) and coin1 at 1620 (3 credits);
      - service1 at 1660, 1720, 1780, 1840, 1900 and 1960, one credit
        each, which reaches 9 credits and the lockout;
      - service1 at 2020 and coin1 at 2080, both at 9 credits. t7 skips
        the service switch at 9 credits [code 0x481be–0x481d2]. The coin
        ends as the reference shows;
      - start1 at 2140: mode 2, and one credit taken.

      The frames are proposed. Each input comes at least 40 frames after
      the one before, longer than the coin path's 18 frames from press to
      0x0f [cap: 1450 → 1468] and the service path's 6 + 30-frame sleeps
      [code 0x481e2, 0x4820c]. If the reference shows a press was missed,
      it is moved and the frame is recorded.
    - sf-start-2p-01: sf-move-01's boot inputs (coin1 1450, coin2 1480,
      start2 1560, start1 1900, start2 2000 [code scenarios/sf_move_01.json])
      to 2,460 frames, with checkpoints every frame 1440–2450. F34 and F46
      continue it.
- **Acceptance.**
  1. C-REF on all three.
     - sf-credit-select-01 equals fight-run-a, including checkpoints 1500
       and 1860.
     - sf-credits-01 equals fight-run-a through 1519, including checkpoint
       1500. Its first input that differs is the missing start1 at 1520.
     - sf-start-2p-01 equals fight-run-a through 1479. It equals sf-move-01
       on every frames.tsv field and on checkpoint 2450.
  2. C-NAT from each capture's 1500 checkpoint through its start press. The
     owned RAM adds: 0x0f per credited coin, the coin slots, the credits
     0xff865a–0xff865d, 0xff8684, slot 7's bytes, the random state, 0xffc01a
     / 0xffc01b and the game mode.
  3. The pulse's 6-frame sleep and the service path's 30-frame sleep match
     slot 7's bytes and the random-state timing.
  4. At 9 credits the lockout clears 0x7070 and writes 0xc00010. A coin
     pressed during the lockout ends as the reference shows.
  5. Starts:
     - 1P start gives mode 2 and takes one credit.
     - 2P start with 1 credit is ignored.
     - 2P start with 2 credits gives mode 1 and takes two.
     - The erase uses 0x3e038's blocks [code quirk].
     - 0x3e04c is reported.
  6. C-NEG: a coin 10 frames late.
  7. Record: sf_fixed_fight_01.json's note calls 1500 the "title with
     credit". It is corrected to the start screen, with slot 1 at 0x3dfce
     [code 0x3dec2–0x3dfce; cap latch 0x3f / 0x80 at 1469 / 1470].
- **Size.** About 121 [code]. The first draft's 125 included 0x3b6a2 (6),
  and 0x3b654 (5) was added from F1, which gives 130. The RAM updates of
  both helpers are already inline in t7 (tasks.rs:2030, 2034), so 9 of
  their 11 instructions are not new. Only the two port writes, 0x3b65e and
  0x3b6ae, are new. This matches F5's rule for the compare helper.
- **Label.** DF at 1500, with real inputs.
- **Blocked by.** None.

### F13. Ryu's Hadoken and Tatsumaki against Retsu
- **Delivers.** One-player scenarios on the sf-cpu-01 boot path (FIGHT!
  erased at 2389, the live CPU from 2390) that milestone-audit.md lists as
  unpinned [doc, "Beyond the fixed fight"]:
  - sf-1p-hadoken-01: Hadokens from range that hit Retsu, are blocked by
    him, and miss;
  - sf-1p-tatsumaki-01: a Tatsumaki that hits, and one that Retsu blocks
    (chip damage).

  They exercise the translated one-player tables: the Hadoken's chip table
  0x4340a and attack-box table 0x434d0, and the Tatsumaki's chip table
  0x45fa8 [code fighter.rs:1312–1325, 1793–1830].
- **Acceptance.**
  1. C-REF on each, with checkpoints every frame from 2389 to the
     scenario's end. Each equals sf-cpu-01 on every field through 2389.
     Each scenario ends before its match decision. A CPU match win would
     reach 0x3eefa, which F5 owns and which runs in parallel [code].
  2. C-NAT from the 2389 fixture to each scenario's end.
  3. Each table is read at least once, at stage 0. Its frames are logged.
     The other stages' entries are listed as not covered.
  4. C-NEG: the native run reading the chip table at stage 2's entry, which
     differs from stage 0's [code], fails at the first chip hit.
  5. C-DOC: hadoken.md's and specials.md's "Not covered" lists are updated.
- **Size.** 0 expected [inf]. Section 2's rule for reached branches
  applies.
- **Label.** DF at 2389, with the live CPU.
- **Blocked by.** None.

### F14. Crouching attacks, jump attacks and blocks against Retsu, and held inputs
- **Delivers.** One-player scenarios on the sf-cpu-01 boot path for the
  remaining unpinned paths in milestone-audit.md [doc]:
  - sf-1p-crouch-01: the six crouching normals, landing and missing;
  - sf-1p-jump-01: neutral, forward and back jump attacks, punch and kick;
  - sf-1p-block-01: Ryu holds back, then down-back, against Retsu's
    attacks: a high block, a low block, and a wrong block that is hit;
  - sf-1p-held-01: both horizontal directions held, both vertical
    directions held, and a button held past 255 frames.
- **Acceptance.**
  1. C-REF on each, with checkpoints every frame from 2389 to the
     scenario's end. Each equals sf-cpu-01 through 2389. Each scenario
     ends before its match decision, as in F13.
  2. C-NAT from the 2389 fixture to each scenario's end.
  3. C-NEG: one input moved by one frame is caught at its first differing
     frame.
  4. C-DOC: crouch-attacks.md, jump-attacks.md and blocking.md gain the
     one-player results.
- **Size.** 0 expected [inf]. Section 2's rule for reached branches
  applies.
- **Label.** DF at 2389, with the live CPU.
- **Blocked by.** None.

### F15. The warning screen from power-on, into the title (frames 0–1647)
- **Delivers.**
  - The boot task's second half, 0x3c210–0x3c290: the typewriter, the
    12-frame pauses, the 240-frame hold, the erases, the 0x7070 coin control
    through F12's 0x3b654, slots 1 and 7 spawned, and the jump into 0x3f4de.
  - The join: 0x3ccae–0x3ccbc (which spawns slot 3), the loop head
    0x3ccc2–0x3cce6 and slot 3's entry 0x3f374–0x3f386.
  - The entries of the timer task (0x3f4de) and the random task (0x4813e).
  - Both sound programs driven by the native latch from power-on, with no
    pre-roll.
  - A launcher power-on mode.
- **Acceptance.**
  1. C-NAT from power-on, with no fixture, over 0–1647. Against sf-boot-01
     for 0–829 (full over 0–799) and sf-attract-01 for 794–1647 (full).
  2. The owned state at the end of 794 equals checkpoint 794. The random
     state equals one pass from zero with an idle byte [cap].
  3. Both sound programs over 0–1647:
     - at the reference's own write times every device is identical, and
       music RAM matches at idle frame ends;
     - from the native latch at latency 51 and at 92, each device's first
       differing frame is listed;
     - no tolerance is adopted;
     - the frame-1 write time, placed by the reset path's costs, is recorded
       as exact or as the measured offset. The 45–138 band does not apply to
       it.
  4. The run reports 0x4268c at 1648.
  5. HOST smoke check: the power-on mode types the warning and shows the
     title.
  6. C-NEG: the 240-frame hold shortened by one.
  7. The frame-790 note is reconciled in first-frame.md and
     sf_fixed_fight_01.json [doc].
- **Size.** About 50 [code: 30 + 4 + 7 + 5, plus the two task entries],
  plus the launcher mode and the sound hookup.
- **Label.** PO, with no player input.
- **Blocked by.** F1, F3, F12 (0x3b654 at 0x3c26a).

### F16. Attract ranking screen
- **Delivers.** 0x4268c–0x4274e: the header, tune 0x39, ten entries through
  0x3bdf2, the search for a marker, the hold on timer counter 3, then 0x3f.
  It is a resumable state at 0x42736 with its return 0x3ce54 taken from the
  stack. The marked-entry path 0x42750 is reported.
- **Acceptance.**
  1. C-NAT from sf-attract-01's 794 fixture over 795–1854, with 0x39 at
     1649. The run reports 0x3ce54 at 1855.
  2. C-NAT from sf-music-03's 7185 fixture over 7186–7391 (its checkpoint
     window), reaching 0x3ce54 at 7392.
  3. C-NEG: an exit one frame early is caught at 1854.
  4. music-stops.md's window-D label is corrected: 0x39 there is the attract
     ranking's tune, not a game-over tune [code + cap].
- **Size.** About 47 [code].
- **Label.** DF at 794 or 7185, with no input.
- **Blocked by.** F3.

### F17. The boot DIP decode for every setting
- **Delivers.** Power-on verification of every boot-decoded DIP setting
  through F1's decode.
- **Acceptance.**
  1. C-REF on sf-dip-boot-01 to -08 (power-on, no inputs, about 12 frames,
     a checkpoint every frame). Together they set every defined MAME setting
     of Coin A, Coin B, Game Continuation, Round Time, Difficulty, Buy-In,
     Number of Countries and Flip.
  2. C-NAT from power-on through the boot task's sleep after 0x3c20a in
     each, including 0xffc000, 0xffc001, 0xffc005, 0xffc007, 0xff8f28,
     0xff8e1c, 0xff863c, 0xff8644 and bit 2 of 0xff8682.
  3. With Self-test On, the native run reports 0x4834e at the reset.
- **Size.** About 0.
- **Label.** PO, CF (DIP variants).
- **Blocked by.** F1, F2.

### F18. Flip screen on every layer
- **Delivers.**
  - `MachineState.flip_screen` follows gfxctrl bit 2 on every write and on
    fixture load.
  - The bg, fg and tx tilemaps flip as MAME's flip_screen_set does. The
    arithmetic is derived from the pinned mame0289 tilemap.cpp.
  - The sprite flip is compared for the first time.
- **Precondition.** tilemap.cpp is not held locally [code: the local MAME
  sources are sf-mame0289.cpp, sf_v.cpp and a few device files]. The user
  allowed fetching it from mamedev/mame at tag mame0289 into
  tools/reference/local/ (decision 4). The capture verifies the derived
  arithmetic. It is not used to choose it.
- **Acceptance.**
  1. C-REF: sf-flip-01 (the fixed fight's inputs with Flip On, to 5040;
     checkpoints at 790 and 1500, and every frame 2389–5040). Its RAM equals
     the audit capture's except gfxctrl bit 2. Any other difference is
     listed.
  2. Rendered from dumps, 790, 1500 and 2389 are byte-exact at 384×224.
  3. C-NAT from the flip-on 2389 fixture over 2390–5014.
  4. Every flip-off comparison still matches.
  5. C-NEG: rendering unflipped fails at 2390.
- **Size.** 0, plus about 50 renderer lines [inf].
- **Label.** DF / CF.
- **Blocked by.** F2.

### F19. Difficulty, round-time and attract-music DIPs in the fixed fight
- **Delivers.** The fixed fight played natively under three difficulty and
  round-time pairs, and with Attract Music Off.
- **Acceptance.**
  1. C-REF: sf-fixed-fight-dsw-01 to -03 (Easy / 150, Difficult / 200, Very
     Difficult / 250) and -04 (Attract Music Off).
     - Each has the fixed fight's inputs and runs past its match decision.
       The decision frame is recorded. At these round times a time-over
       comes about 2,850–4,750 frames after FIGHT! [inf: 19-frame ticks,
       round-timer.md], and Easy changes Retsu's behaviour.
     - Each has checkpoints every frame from 2389 through its decision.
     - Each capture's 2389 decodes, or the fixture frame moves and is
       recorded.
  2. C-NAT from each 2389 through the match decision.
  3. The TIME reload gives 150, 200 and 250. Retsu's AI tables equal that
     difficulty's ROM rows.
  4. With Attract Music Off, no command 0x28–0x3e reaches either latch, and
     the two latch streams are identical.
- **Size.** 0 (F2 made the filter live).
- **Label.** DF, CF variants.
- **Blocked by.** F2.

### F20. The Speed DIP's slow game
- **Delivers.** A round of the fixed fight at Speed Slow. 0x3b5aa sleeps 6
  frames per pass [code], while the timer keeps its 19-frame tick.
- **Acceptance.**
  1. C-REF: sf-fixed-fight-slow-01. The coin, start and country inputs are
     re-placed from calibration captures. Checkpoints every frame from round
     1's start through its result.
  2. C-NAT from that fixture through the round's result. A time-over is
     expected [inf].
  3. C-NEG: the same run at Speed Normal is caught at the first wake of the
     fight loop.
- **Size.** 0.
- **Label.** DF / CF.
- **Blocked by.** F2.

### F21. The Freeze DIP
- **Delivers.**
  - The vblank Freeze spin 0x3b100–0x3b108, and the handler resuming on
    release. The resume is derived from the code and MAME's interrupt
    model, not from the capture:
    - the spin re-reads DSW1 bit 14 (0xc00008) until it reads 1, then the
      handler goes on at 0x3b10a in that frame [code];
    - the vblank is irq1_line_hold (sf-mame0289.cpp:758), and the handler
      runs with the level-1 mask set. So the vblanks during the spin stay
      pending as one held request, which is taken at the handler's RTE
      [inf: from the 68000's interrupt mask and HOLD_LINE].

    The capture verifies this derivation. It is not used to choose it.
  - Scenarios can change a DIP at given frames.
- **Acceptance.**
  1. The driver applies DIP changes at frames, with a readback logged at
     each change.
  2. C-REF: sf-freeze-01 (the fixed fight's inputs, Freeze On at 2500 and
     Off at 2560). Checkpoints every frame from 2389 through the match
     decision. The freeze shifts later events by about 60 frames [inf].
  3. C-NAT over 2390 through the match decision, including the frozen frames
     and the release.
  4. The latch-write times of the release frame are compared with a tap log
     and recorded.
- **Size.** About 5, plus tooling.
- **Label.** DF / CF.
- **Blocked by.** F2.

### F22. Every FM effect and special command from the service sound test
- **Delivers.** sf-sound-test-01 in service mode, with no test-mode
  translation needed (the sound programs run on the reference's latch
  [inf]). It contains:
  - each of the 27 effect entries and 16 specials played over silence;
  - effects played over tunes 0x33 and 0x28;
  - priority overlaps;
  - 0x00 sent with an effect running.

  Its inputs, from power-on with Service On: P1 up alone or down alone,
  sampled every 5 frames, steps the code up or down while held, and a P1
  punch whose latched strength exceeds 4 plays it through 0x3b7c8 [code
  0x4854a–0x485be]. Which button gives that strength is [inf]. Its windows:
  - one hook window (run_audio_probe's `--hook-window`) on the YM2151 from
    the frame before the first command to the run's end;
  - a full checkpoint on the frame before each latch write and at the
    run's end. A frame before a write need not be idle, because an effect
    or a tune may be playing. So acceptance 3 compares music RAM only at
    those checkpoints whose frame end finds the music CPU idle: in its
    idle loop or in a handler's closing instructions
    (milestone-audit.md:77 [doc]).
- **Acceptance.**
  1. Two run_audio_probe runs are identical (taps, hook streams, WAV). The
     DSW readback shows only the service DIP changed.
  2. The latch log holds every command 0x00–0x27, plus 0x35, 0x3d and 0x3f.
  3. From power-on, with no fixture and no fitted phase:
     - every YM2151 write matches on the exact clock;
     - the output is sample-identical in every hook window;
     - RAM 0xc000–0xc5cf matches at the idle checkpoints. The count of
       idle checkpoints, and of checkpoints that were not idle, is
       reported. An idle count of zero fails this acceptance, and each
       command with no idle checkpoint before the next command is listed
       as not compared, for F72.
  4. The paths never reached so far are reached and match: a replaced
     effect freeing a channel, the music skipping an effect-owned channel
     (0xc023), the stops with a non-empty slot, and slide 0 in effect 0x22.
  5. 0x0e62 and 0x02a5 are expected unreached. If reached, they are
     reported.
  6. C-NEG: one latch write moved by 4 ms.
- **Size.** 0, or a reached branch of up to about 30 instructions [inf].
- **Label.** SM.
- **Blocked by.** F2.

### F23. Every tune through its loop point or its end, including the round tune's loop
- **Delivers.** sf-sound-test-02: the 21 tunes played alone. The 13 looping
  tunes run past their loop point (0x28 past about 50 s). The 8 ending tunes
  run to opcode 0xf. Its inputs are F22's sound-test inputs.
- **Acceptance.**
  1. C-REF on sf-sound-test-02, with each loop point or end frame recorded.
     Its windows:
     - one hook window on the YM2151 from the frame before the first
       command to the run's end;
     - a full checkpoint on the frame before each command, and on the
       frame after each recorded end and each loop point.
  2. Every YM2151 write is exact. The output is identical around every loop
     point and end. RAM 0xc000–0xc5cf matches at each checkpoint after an
     end, where that checkpoint's frame end finds the music CPU idle
     (milestone-audit.md:77 [doc]). The count of idle and not-idle
     checkpoints is reported. An idle count of zero fails this acceptance,
     and each end with no idle checkpoint before the next command is
     listed as not compared, for F72.
  3. A scan of every tune stream states whether opcode 0xa (to 0x0000) or a
     nested end at 0x09e7 occurs. If one is reached, it is translated and
     matched.
  4. Tune 0x28 plays one full pass after its loop point.
  5. C-NEG: one loop point moved by one tick is caught.
- **Size.** 0 to about 30 [inf].
- **Label.** SM.
- **Blocked by.** F2.

### F24. Every ADPCM sample on both channels and every bank
- **Delivers.** sf-sound-test-04: commands 0x81–0xbf each played to its end,
  0x80 stopping a sample on each channel, and overlapping playback. Its
  inputs are F22's sound-test inputs.
- **Acceptance.**
  1. C-REF with run_audio_probe. Its windows:
     - one hook window on both MSM5205s from the frame before the first
       command to the run's end, which covers banks 0–6;
     - a full checkpoint on the frame before each command and at the run's
       end.
  2. Every samples-CPU port write matches at its logged time to the T-state.
     Both MSM5205 DACs are identical at every tick of the hook window.
  3. The report lists every command, banks 1 and 3–6, and the 0x80 channel
     paths. It measures the total ADPCM time against about 102 s.
  4. Any untranslated branch is reported.
  5. C-NEG: one DAC nibble altered at one tick is caught at that tick.
- **Size.** 0 [code: samples.rs has no report sites].
- **Label.** SM.
- **Blocked by.** F2.

### F25. Game over and the return to the attract boundary (one-player and versus)
- **Delivers.** This merges the match-end game-over slice and the versus
  game over.
  - 0x3f1bc–0x3f274.
  - Event 10 0x3ff5a: 25 timed colour writes, a 0xc00 scroll, sound 0x95
    and a 180-frame hold.
  - The game-over objects through 0x42f1e and 0x4329c, run by the effect
    runner.
  - The 0xff stream-call dispatch 0x42dd2.
  - 0x3f2d0–0x3f33a: both players' ranking checks, then the 0x3f2f0 screen
    with palette 0x1a268, 0x95 and a 240-frame hold.
  - The reset 0x3f33c–0x3f372: the clears, 0xff8003, then 0x3f358's test.
    With credits left it goes to 0x3dec2 (verified by F45). Otherwise slot 2
    is killed and the flow goes to 0x3ccae.
- **Acceptance.**
  1. C-REF: sf-doubleko-02 (sf-doubleko-01's inputs to about 6,000 frames,
     checkpoints at 2450 and every frame 5560–6000). It equals
     sf-doubleko-01 through 5800.
  2. C-NAT on sf-match-end-01, continuing F5's run from 6101 to the frame
     before 0x3ccae runs (about 6331 [inf]):
     - ten 0x1b at 6102–6137 and 0x95 at 6151 [cap];
     - the palette steps and the hold;
     - the cleared game state and slot 2.

     The run reports 0x3ccae at the reference's frame.
  3. C-NAT on the double KO. The run starts at the 2450 fixture. Frames
     2451–5559 are compared against sf-doubleko-01 (checkpoints every frame
     2450–5800), and 5560–6000 against sf-doubleko-02. It goes through both
     0x42422 calls (P1 with scene byte 2, then P2 with scene byte 1, both
     scores cleared), the screen, the clears and the kill. The run reports
     0x3ccae.
  4. A probe lists every stream routine the objects reach. Untranslated
     ones are reported.
  5. C-NEG: event 10's hold shortened by one.
- **Size.** About 132 [code: 46 + 22 + 17 + 13 + 2 + 32], plus any routines
  the probe finds.
- **Label.** DF at 2389, and CF for the versus double KO.
- **Blocked by.** F1 (kill), F5.

### F26. A versus winner carries on against the CPU (one machine for every mode)
- **Delivers.**
  - 0x3eec2–0x3eeec: the clears, 0x43748, then 0x436b8 into 0x3e5d6.
  - One native machine that loads any fight-loop checkpoint and takes both
    players' inputs. It compares the union of `VERSUS_RAM_REGIONS` and
    `CPU_RAM_REGIONS` (run.rs:347, 439) plus 0xff8e52–0xff8f0d.
- **Acceptance.**
  1. C-REF: sf-versus-carry-01 (from 2450, P1 with sf-win-01's pattern, P2
     lands one jab and then idles). Checkpoints every frame from 2450 to its
     end.
  2. Every existing versus and one-player scenario still matches on the one
     machine.
  3. C-NAT from 2450 through:
     - the versus match win and wipe;
     - 0x42422 for P2: no rank, the score cleared, 0xff8e5e / 0xff8e62
       written;
     - the mode-2 placement 0x3e632;
     - Retsu's stage intro, ROUND 1 and the AI tables;
     - the CPU round, to the capture's end.
  4. sf-timeover-01 now matches through 10300.
  5. A ranking score is reported at 0x42486.
  6. C-NEG: a byte changed only in a region the union adds (for example
     0xff8e52–0xff8f0d) is caught at its frame.
- **Size.** About 9, plus the machine harness.
- **Label.** CF for the versus, DF for the CPU round.
- **Blocked by.** F4, F5.

### F27. Ryu beats Geki (0x16bfc)
- **Delivers.**
  - Geki's 4 hit-function entries.
  - His stream routines 0x4594c (6), 0x4596e (42, using F5's helper through
    its loop entry 0x3bb0e) and 0x45a34 (17).
  - The shuriken handlers 0x43348 (18) and 0x43396 (7).
  - The country's-end dispatch 0x3ef96 (8).
- **Acceptance.**
  1. C-NAT from sf-win-02's Geki round-1 fixture through the flow task's
     entry to 0x40d30. The owned RAM adds the effect slots that hold
     shurikens. The comparison is full on every frame, inside the first
     part of sf-win-02's window (F6 acceptance 2).
  2. 0x3ef96 dispatches stage 0 through 0x3efb0. 0x40d30 is reported unless
     F29 is done.
  3. C-DOC lists which hit functions, which branches of 0x4596e and which
     handlers the fight exercises, and which it does not.
- **Size.** About 98, plus 4 hit-function entries [code: 6 + 42 + 17 + 18 +
  7 + 8].
- **Label.** DF.
- **Blocked by.** F5 (the compare helper), F6.

### F28. Ryu beats the first stage-1 opponent (0x1734c)
- **Delivers.**
  - 8 hit-function entries [code; identity per ai-tables.md, inf].
  - sf-win-03: sf-win-02 extended by a segment that wins this match.
- **Acceptance.**
  1. C-REF on sf-win-03. It equals sf-win-02 through the divergence. It has
     checkpoints every frame over this fight and the next fight's first
     frames.
  2. C-NAT from this fight's round-1 fixture through the match-won screen
     and the next opponent's events 4 and 5 and setup (0x44860 / 0x44880),
     up to that opponent's first untranslated hit function.
  3. The 8 entries are traceable. C-DOC lists landed and unexercised hit
     functions.
- **Size.** Entries only, plus a search.
- **Label.** DF.
- **Blocked by.** F6.

### F29. Power/timing bonus after Japan (0x40d30)
- **Delivers.**
  - Copy 1: 0x40d30–0x412a4 (282 instructions).
  - The helpers 0x4141a–0x4159c (79) and 0x3b96c (about 5).
  - The stage step 0x3efc4 (5).
  - The fixture resume point 0x40dd4.
- **Acceptance.**
  1. C-REF: a variant of sf-win-02 with other bonus inputs, equal to
     sf-win-02 through the divergence. Checkpoints every frame over the
     bonus in both captures.
  2. C-NAT from the bonus-start fixture in both captures. One takes the
     success branch (0x410b6–0x41220), the other the failure branch
     (0x41226–0x412a4).
  3. The countdown (0xff8e30 / 32, flag 0x10), the meter, the digits and the
     sounds match.
  4. After 0x3efc4 moves stage 0 to 1, the run reports 0x3e59c, or
     continues into F4's path.
  5. Two game-mode tests are listed, and their branches that Ryu's chain
     does not take are labelled unexercised, because that chain runs in
     mode 2 [code]:
     - 0x40d36 tests 0xffc002 against 4. Its mode-4 branch is unexercised;
     - 0x414f6, in the helper 0x414f0 called from the resume point
       0x40dd4, tests it against 2. Its branch for any other mode (the
       bne to 0x41508) is unexercised.

     Ken's mode-4 chain is open decision 10.
- **Size.** About 370 [code]. Near the limit; split before starting if a
  probe shows more.
- **Label.** DF.
- **Blocked by.** F6.

### F30. Listening review of the fixed fight (person)
- **Delivers.** The user's verdict, using F9's kit.
- **Acceptance.**
  1. The verdict table (timing, pitch, character, native against MAME, the
     live launcher) is filled in and recorded as a follow-up to
     milestone-audit.md.
  2. Each finding is opened as a ticket.
- **Size.** 0.
- **Label.** PERSON (ready-for-human).
- **Blocked by.** F9.

### F31. A person's keyboard session of both fixtures (person)
- **Delivers.** Sessions of a person playing the one-player and versus
  fixtures, recorded with F10.
- **Acceptance.**
  1. At least one round of each fixture is recorded. The files are kept
     locally.
  2. The verify command passes, or its first mismatch is opened as a
     ticket.
  3. The user's notes on the controls and pacing are recorded.
- **Size.** 0.
- **Label.** PERSON (ready-for-human).
- **Blocked by.** F10.

### F32. One-player round 3 after a 1–1 split, a time-over and a draw
- **Delivers.** Two or three one-player references found with F6's search
  tool, on the sf-cpu-01 boot path:
  - sf-1p-round3-01: Ryu wins round 1, loses round 2, and survives round 3
    to its time-over, which is decided on health. At round time 100 that
    is about 1,900 frames of round 3 [inf: 100 ticks of 19 frames].
  - sf-1p-draw-01: a match that ends with equal wins after three rounds,
    with at least one round drawn. Equal wins leave no winner, and in
    one-player mode that is the CPU's match win at 0x3eefa [code; doc
    round-outcomes.md].

  milestone-audit.md:172 lists a one-player time-over, draw and double KO
  separately [doc]. So the search looks for both kinds of drawn round: equal
  health at the time-over, and a double KO. They may come in one match or
  in two captures (sf-1p-draw-01 and sf-1p-doubleko-01).
- **Acceptance.**
  1. C-REF on each capture, with checkpoints every frame from 2389 through
     the match decision and the screen after it. Each equals sf-cpu-01
     through 2389.
  2. C-NAT from 2389 through the match decision: round 3's intro, the
     time-over (text 0x14f74, sound 0x96), the health comparison and the
     result.
  3. A CPU win or a draw continues into F5's continue screen and matches to
     the capture's end. A Ryu win continues into the match-won path and
     Geki's setup.
  4. C-NEG: round 3's time-over moved one tick late is caught.
  5. C-DOC: round-outcomes.md's "Not covered" list is updated.
  6. Each kind of drawn round that is not found is named as not covered in
     round-outcomes.md, for F72.
- **Size.** 0 expected [inf]. Section 2's rule for reached branches
  applies.
- **Label.** DF at 2389, with the live CPU.
- **Risk.** The search may find no drawn time-over, or no double KO,
  within its budget [inf]. Then that kind stays not covered by name, and
  the ticket records the budget spent.
- **Blocked by.** F5 (the continue screen), F6 (the search tool).

### F33. A coin during the title opens the start screen
- **Delivers.**
  - t7's attract start 0x48228–0x48248: 0xff8f28 copied to 0xffc007, slot 2
    killed (0x48232), slot 1 spawned at 0x3dec2 (0x48242), resume 0x4824e.
  - The start-screen entry 0x3dec2–0x3df48.
  - Slot 1 in F3's title wait (0x3cdb8) decodes. The new slot 1 runs in the
    same frame under F1's rescan.
- **Acceptance.**
  1. C-NAT from sf-credit-select-01's 1440 fixture through 1521:
     - the attract texts;
     - the debounce over 1450–1462;
     - 0x0f at 1468;
     - slot 1 re-created;
     - 0x3f / 0x80 at 1469 / 1470 [cap];
     - the 1P start.
  2. 0xffc007 equals 0xff8f28 after the attract start. 0xff8f28 is added to
     the owned lists.
  3. The killing of an occupied slot 2 is listed as not covered here. It is
     F66's.
  4. C-NEG: the coin 10 frames late.
- **Size.** About 36 [code: 7 + 29].
- **Label.** DF at 1440, with the real coin press.
- **Blocked by.** F1 (kill and spawn), F3, F12.

### F34. Two-country select: timeout to Japan, stick and button to USA, and the opponent order
- **Delivers.**
  - The head and the two-country screen 0x3e04c–0x3e1e3 (78).
  - The exit 0x3e372–0x3e389 (7).
  - The handlers 0x3e39a–0x3e585 (63), each writing 15 pointers.
  - The spawn of slot 2 at 0x3f7b0 (0x3e590), and the speed sleep, at
    0x3e586–0x3e596 [code].
  - The 2×2 writer 0x3bc3e (63) and eraser 0x3bd3e (27).
  - The capture sf-select-usa-01.
- **Acceptance.**
  1. C-NAT from 1500 (through F12's path) over 1501–1866 on
     sf-credit-select-01:
     - the select's texts, palette 0x1a468 and fg scroll;
     - 0x34 at 1522;
     - tally 4 at 0xff8e30–0xff8e35;
     - the timeout exit in frame 1865 with country 0;
     - 0x3f at 1866;
     - slot 2 spawned and run in frame 1865 [cap].

     At 1866 the run continues into F4's path if it is done. Otherwise it
     reports 0x3e59c.
  2. The arrays 0xffc01c–0xffc057 (added to the owned lists) equal the
     reference's at 1866. The reference's copies at 1980 (fight-run-a) and
     2389 (sf-cpu-01) are unchanged from 1866 [cap]. A static check confirms
     0x19908 + 0x28·country + 0xa0·n.
  3. C-REF on sf-select-usa-01 (right, left, right, a diagonal, then LP),
     with checkpoints every frame from 1440 through the exit. C-NAT through
     the exit: the diagonal is ignored, country 2 is chosen, the arrays
     start at 0x19958 and 0xffc006 bit 1 is set.
  4. sf-start-2p-01, continued through its select in mode 1, matches.
  5. Records corrected:
     - ai-tables.md: 15 pointers, not 13;
     - round-timer.md: "not set in any capture" is removed;
     - sf_fixed_fight_01.json: the comment on the 1900 "confirm" (the select
       ends by its timeout [cap]).
  6. C-NEG: LP 10 frames late.
- **Size.** About 242 [code].
- **Label.** DF at 1500.
- **Blocked by.** F1 (spawn), F12.

### F35. Coinage DIP: coins at non-default rates
- **Delivers.** Coins credited at 2C1C on slot 1 and 1C6C on slot 2, run
  natively.
- **Acceptance.**
  1. C-REF: sf-coinage-01, with the readback as requested and checkpoints
     every frame from 1440 through the last credit. Its coins, each held 12
     frames and at least 40 frames apart:
     - a slot-2 coin at 1450, in the title: 6 credits. The attract start
       then opens the start screen. That path is F33's, so it is compared
       in the reference only;
     - in the start screen, a slot-1 coin (no credit) and a second slot-1
       coin (1 credit, 7 in all);
     - a second slot-2 coin at 7 credits.
  2. C-NAT from its start-screen checkpoint, before the first slot-1 coin:
     - the first slot-1 coin gives no credit and the second gives one;
     - the second slot-2 coin gives 2 credits. The award stops at 9
       (0x48308), and 0x3b6a2 clears 0x7070. Its other 4 credits are not
       awarded [code 0x482fe–0x48324].
  3. A static check decodes all 64 settings through 0x1456a with F1's
     0x3b2e4, labelled controlled.
- **Size.** 0 (the decode is F1's).
- **Label.** DF / CF.
- **Blocked by.** F1 (0x3b2e4), F2, F12.

### F36. Remappable keyboard bindings with persistence
- **Delivers.** A bindings file, `%APPDATA%\sf1-port\bindings.toml` on
  Windows (decision 9), whose defaults are launcher.md's table plus coin,
  start and service keys. It is loaded at start and saved on change,
  with a rebind flow. Duplicate or missing bindings are refused, and Esc
  stays fixed. Its defaults include the coin and start keys. If F41 has
  bound fixed keys first, F36 replaces them.
- **Acceptance.**
  1. The file round-trips. A missing, corrupt or partial file falls back to
     the defaults with a message.
  2. A unit test pins every logical input of `Io` for the default table and
     for a non-default one.
  3. sf-fixed-fight-audit-01's P1 inputs, fed through a remapped table,
     reproduce the audit capture over 2390–5014.
  4. sf-blocks-01's inputs, fed through remapped tables, match.
  5. The smoke check reports the active table.
- **Size.** 0. About 200–300 host lines [inf].
- **Label.** HOST.
- **Blocked by.** F12 (coin, start and service become logical inputs
  there).

### F37. Demo Sounds and Attract Music DIPs over the attract
- **Delivers.** Both filters, made live by F2, verified over the attract.
- **Acceptance.**
  1. C-REF: sf-attract-demosound-off-01 and sf-attract-music-off-01
     (power-on, 1,900 frames, checkpoints every frame 794–1,900).
  2. C-NAT from each 794 fixture over 795–1854:
     - with Demo Sounds Off, no command is latched;
     - with Attract Music Off, 0x1c is latched at 976 and 0x39 never is
       [code].
  3. The default runs of F3 and F16 still match.
- **Size.** 0.
- **Label.** DF / CF.
- **Blocked by.** F2, F16.

### F38. Tune transitions: priorities, the saved tune and the stops
- **Delivers.** sf-sound-test-03:
  - a tune started over another at higher, equal and lower priority;
  - a tune saved to 0xc300 and popped (0x01d3 / 0x03a7);
  - the saved tune discarded (0x0412);
  - 0x35, 0x3d and 0x3f sent over a saved tune;
  - a repeated command ignored.

  Its inputs are F22's sound-test inputs.
- **Acceptance.**
  1. C-REF. Its windows:
     - one hook window on the YM2151 from the frame before the first
       command to the run's end;
     - a full checkpoint on the frame before each command and on the frame
       after each transition. Acceptance 3 compares RAM only at those
       whose frame end finds the music CPU idle, by milestone-audit.md:77's
       definition [doc]. A frame after a transition need not be idle.
  2. Exact writes, and identical output over the hook window around each
     transition.
  3. RAM 0xc011, 0xc012 and 0xc300–0xc47f matches at each idle checkpoint
     after a transition. The count is reported, and so is each transition
     with no idle checkpoint before the next command, as not compared, for
     F72. An idle count of zero fails this acceptance.
  4. Each path that music-phrase.md and music-stops.md list as unexercised
     is either matched or shown to be dead from the tune headers.
- **Size.** 0.
- **Label.** SM.
- **Blocked by.** F23 (which owns the possible opcode-0xa and nested-end
  translations).

### F39. Continue with a credit: the rematch against the same opponent
- **Delivers.** 0x3f166–0x3f1b8:
  - the sound reset through F1's 0x3b7a8 / 0x3b802 (called at 0x3f166);
  - CONTINUE and the digits erased;
  - a credit taken (`decimal_down`, which is 0x3bb74, already translated
    [code tasks.rs:490]);
  - 0x436b8 and 0x43748;
  - re-entry at 0x3e6ca.
- **Acceptance.**
  1. C-REF: sf-continue-01 (the fixed fight's inputs plus coin1 and start1
     inside the countdown, to about 7,000 frames, checkpoints every frame
     5400–7000). It equals sf-match-end-01 before the coin.
  2. C-NAT from F5's 2389 run through the coin's arrival, the continue, the
     rematch's FIGHT! and on to the capture's end. The credits, coin slots,
     and the stage, fight and win bytes are owned, and Retsu is the opponent
     again.
- **Size.** About 17 [code].
- **Label.** DF.
- **Blocked by.** F1 (0x3b7a8), F4, F5, F12.

### F40. P2 wins the versus and plays the CPU (game mode 4: Ken's one-player game)
- **Delivers.** P2's versus win carried into mode 4, the only route there.
  Mode 4 is written only at 0x3eeac [code], and P2 start alone needs 2
  credits and gives versus [code 0x3dfd2–0x3dff2]. Ken then fights the CPU
  from a P2 script.
- **Acceptance.**
  1. C-REF: sf-versus-carry-02 (Ken wins 2–0 by Hadokens, then a Ken script
     against CPU Retsu through the match decision). Checkpoints every frame
     from 2450 through the decision and the screen after it.
  2. C-NAT from 2450 through:
     - P1's ranking check with scene byte 2;
     - 0x3e674 and the mode-4 intro (0x3fad6);
     - the HUD and ROUND 1;
     - the fight: P2's input handler with the CPU decision, the dispatcher
       pair 0x42ac0, the camera pair, the latches, the random state on P2's
       input, and the CPU's hits on Ken (0x446f6 / 0x4473a / 0x4479c);
     - the decision. On a CPU win, F5's 0x3ef2c and the continue screen on
       P2's start bit. On a Ken win, 0x40140 and the next opponent's setup.
       Ken's game past Retsu is not covered here. It is open decision 10.
  3. A dated follow-up note retires milestone-audit.md's "P2-versus-CPU
     mode is not exercised" and latch-timing.md:132's "game mode 4" entry
     [doc]. The audit's verified tables stay as recorded.
- **Size.** Up to about 50 [inf].
- **Label.** CF for the versus, DF for the CPU fight.
- **Blocked by.** F26.

### F41. A challenger joins a one-player fight
- **Delivers.**
  - 0x3f4a2–0x3f4da: the limit test against 0xffc001, a credit taken, scene
    byte 0x10, and the wait at 0x3f4c6.
  - 0x423b8–0x4241c: sounds 0x3f / 0x3e, texts 0x15ba2 / 0x15c30, the 120-
    and 240-frame holds, the wipe and mode 1, then the jump to 0x3e5d6. The
    linear listing misdecodes 0x423b6–0x423bb [code].
  - Coin and start keys for both players in the launcher, as host code
    outside the size. If F36 has landed, they go into its table. Otherwise
    they are fixed keys, and F36 later makes them remappable.
- **Acceptance.**
  1. C-REF: sf-challenger-01 (sf-cpu-01's inputs plus coin2 at 1480, P2
     start in round 1's result and again in round 2). Checkpoints every
     frame from 2389 through the versus fight's first round result. Its
     frames through 2389 equal sf-cpu-01's except the credits, and every
     difference is listed.
  2. C-NAT from its 2389 through:
     - the ignored first press;
     - the join;
     - the sequence;
     - 0x3e5ea and event 4's versus shape (also exercised by F46);
     - ROUND 1 and the versus fight.
  3. HOST smoke check: from the two-credit fixture, the bound P2 start key
     reaches the challenger screen.
- **Size.** About 33 [code].
- **Label.** DF, then CF for the versus.
- **Blocked by.** F12 (start2 as a native input), F26.

### F42. Name entry for a ranking player
- **Delivers.**
  - 0x42486–0x425c2 (68).
  - The cursor routines 0x427e2–0x42809 / 0x42844–0x4291f (65) and
    0x42920–0x429f1 (51).
  - The spawn 0x42f72 (20).
  - The cursor object 0x1845c, which runs stream 0x18ee4 with its 0xff
    calls through F25's 0x42dd2.
- **Acceptance.**
  1. C-REF: sf-name-entry-01 and -02. They take the two-credit path (coin2
     at 1480), sf-win-01's pattern through Retsu, P2 start at Geki's FIGHT!,
     and P2 Hadokens. In -01 P1 enters letters, a back and END. In -02 P1
     enters nothing and the countdown runs out. Checkpoints at 2389 and
     every frame from the versus round-2 fixture through the last frame of
     Ken's mode-4 intro (0x3fad6). That window covers the entry, and also
     F50's insertion, the ranking screen with the blinking entry, 0x3eec2
     and the mode-4 start. Their frames through 2389 equal sf-cpu-01's
     except the second credit, and every difference is listed.
  2. P1's score ranks at the fixture under the compare.
  3. C-NAT from a versus round-2 fight-loop fixture through:
     - Ken's KO of Ryu and the wipe;
     - the ranking check finding a rank;
     - the entry screen (0x14656, 0x1466e, 0x14638, 0x1ac68, 0x38);
     - the cursor;
     - the countdown 0xff8e24–0xff8e29;
     - each letter and END (or the timeout).

     The run reports 0x425c4.
  4. A probe shows only 0x427e2 and 0x42920 as stream calls. Any other is
     reported.
  5. C-NEG: a letter 10 frames late.
- **Size.** About 204 [code]. Oversized risk: the stream format is
  inferred.
- **Label.** DF mid-game, CF for the scripted P2.
- **Blocked by.** F25, F26.

### F43. Country to country: the first bonus to stage 1's first fight (the map)
- **Delivers.**
  - Event 2's stage-above-0 branch 0x3f812–0x3f8e0: the scroll, palette
    0x1a268, and the plane chosen from 0x3f822 by country and stage.
  - The plane's stream routines 0x432c4, 0x432e0, 0x43306 and 0x4331a (24).
- **Acceptance.**
  1. C-NAT from F29's bonus-start fixture on sf-win-02 through the bonus,
     the flight, event 3, stage 1's intro and ROUND 1, up to the first
     fight-loop frame of 0x1734c. The owned RAM adds the plane's effect
     slot. The comparison is full on every frame, inside the second part
     of sf-win-02's window (F6 acceptance 2).
  2. The plane's loop, flip and sound calls match on their frames.
  3. No untranslated branch is reached before the fight loop.
- **Size.** About 54 [code].
- **Label.** DF.
- **Blocked by.** F4, F25, F29.

### F44. Ryu beats the second stage-1 opponent (0x170dc)
- **Delivers.**
  - 4 hit-function entries.
  - sf-win-04 (this match won, fixed bonus-2 inputs, stage 2's arrival).
- **Acceptance.**
  1. C-REF on sf-win-04, equal to sf-win-03 through the divergence, with
     checkpoints every frame over the fight, the bonus and stage 2's
     arrival.
  2. C-NAT from the round-1 fixture through 0x3ef96's stage-1 dispatch to
     0x41740, which is reported unless F52 is done.
  3. The entries are traceable. C-DOC lists landed and unexercised hit
     functions.
- **Size.** Entries only, plus a search.
- **Label.** DF.
- **Blocked by.** F27 (0x3ef96), F28 (the chain).

### F45. Game over with a credit left returns to the start screen
- **Delivers.** Verification of 0x3f358's branch to 0x3dec2 when credits
  remain [code 0x3f358–0x3f360]. F25 translates the branch and F33 the
  start-screen entry.
- **Acceptance.**
  1. C-REF: sf-gameover-credit-01. The fixed fight's inputs plus coin2 at
     1480, so one credit is left after the 1P start. No start is pressed in
     the countdown. Checkpoints at 2389 and every frame from 5000 to its end
     (about 6,700 [inf]). It equals fight-run-a through 1479. Every later
     difference from sf-match-end-01 is listed.
  2. C-NAT from its 2389 through:
     - the continue screen with a credit and no start;
     - the expiry (0x3f1bc) and the game over;
     - 0x3f358's branch with credits ≥ 1;
     - the start-screen entry 0x3dec2 and the start-screen loop with the
       credit digits, to the capture's end.

     The run reaches neither the slot-2 kill nor 0x3ccae.
  3. C-NEG: the native run with the credit count read as 0 at 0x3f358 fails
     at the first frame after the branch.
- **Size.** 0.
- **Label.** DF at 2389. The CPU wins.
- **Blocked by.** F25, F33.

### F46. A fresh two-player start into the versus fight
- **Delivers.** Verification of the mode-1 path from the start screen to the
  versus fight: events 2 and 3 in mode 1, the mode-1 placement at 0x3e5ea,
  event 4's versus shape, ROUND 1 and FIGHT! [code 0x3e5d6–0x3e62e]. The
  challenger path (F41) jumps to 0x3e5d6 at 0x4241c and skips events 2 and
  3 [code], so only a fresh start reaches them.
- **Acceptance.**
  1. It uses F12's sf-start-2p-01 (sf-move-01's boot inputs, checkpoints
     every frame 1440–2450). No new capture is needed.
  2. C-NAT from its 1500 checkpoint over 1501–2450: the 2P start, the select
     in mode 1 (F34's path), events 2 and 3, the placement, event 4's versus
     shape, ROUND 1 and FIGHT!.
  3. At 2450 the owned RAM equals sf-move-01's 2450 fixture. The run then
     continues on F26's machine with sf-move-01's inputs over 2451–3350,
     against sf-move-01's checkpoints.
  4. C-NEG: P1's and P2's placement words swapped are caught at the first
     frame the fighters are drawn.
- **Size.** 0 expected [inf]. Section 2's rule for reached branches
  applies.
- **Label.** DF at 1500 through FIGHT!. CF for the versus fight, where P2 is
  untouched in sf-move-01.
- **Blocked by.** F4, F26, F34.

### F47. Four-country select (DIP)
- **Delivers.** 0x3e1e4–0x3e371: palette 0x1a668, the 0x4380 scroll, text
  0x15e64, and a 2×2 cursor kept in D6 / D7 as task state, mapped by
  0x3e2ea and redrawn through 0x3e316.
- **Acceptance.**
  1. C-REF: sf-select4-01 and -02, with 0xffc005 = 0. Checkpoints every
     frame from 1440 through event 2's request.
  2. C-NAT from each start-screen checkpoint through the button exit and
     event 2's request: every cursor move, the ignored diagonal and the
     marker redraws.
  3. The pointer arrays for countries 1 and 3 equal the reference's. All
     four handlers are then matched against a reference (0 and 2 by F34).
- **Size.** About 86 [code].
- **Label.** DF / CF.
- **Blocked by.** F2, F34.

### F48. Attract demo: setup through FIGHT!
- **Delivers.**
  - The setup 0x3ce54–0x3ce86 (11) [code].
  - The demo branches 0x3e5c6–0x3e5d0 (3) and 0x3e6fe–0x3e70e (4), entering
    0x3e714 without events 4 and 5.
  - The demo input 0x4030c–0x40334 (10): records from 0x3cebf, wrapping at
    the length 0x3cebc, 1365 records [code].
  - It reuses F34's head, exit, handler and spawn, and F4's 0x3e5d6 and
    0x436b8.
- **Acceptance.**
  1. C-REF: sf-attract-demo-01 (power-on, no input, about 6,000 frames,
     checkpoints at 794 and every frame from 1,850 through the second
     ranking's exit). Its rows 0–1,900 equal sf-attract-01's. The capture
     contains the demo's end.
  2. C-NAT from 794 through 1855 (slot 1 at 0x3e59c, slot 2 at 0x3f7d6),
     0x28 at 1859, the walk-in and 0x94 at the reference's frame, which is
     recorded (about 1940 [inf, from an offset]), and the fight up to the
     first untranslated branch.
  3. Every frame's 0xff8f2a, P1's input byte and latches, and the random
     state match.
  4. A PC-coverage probe lists every routine the reference demo runs through
     its round end, with the untranslated ones' counts. This sizes F55.
  5. C-NEG: the index starting at record 1.
- **Size.** About 30, plus any stream calls met before FIGHT!.
- **Label.** DF at 794, with the live CPU.
- **Blocked by.** F4, F16, F34.

### F49. The fixed fight from power-on, with no fixture
- **Delivers.** The fixed fight played natively from the reset through the
  boot, the title, the coin, the start, the select and the pre-fight
  screens, with the random state reproduced. The launcher's power-on mode
  plays into the fight.
- **Acceptance.**
  1. C-REF: sf-fixed-fight-power-on-01 (the fixed fight's inputs,
     checkpoints every frame 0–2389, rows equal fight-run-a's). Also
     sf-rng-inputs-01: the same, with P2 directions held in the attract and
     P1 inputs held on a pre-fight screen. Its random state at 2389 differs
     from the fixed fight's, or the placement is revised until it does.
  2. C-NAT from power-on over 0–2389 for both, including 0xff8e4a–0xff8e51,
     slot 7, the coin slots and the credits.
  3. At 2389 the owned RAM equals sf-cpu-01's 2389.
     sf-fixed-fight-power-on-01 then matches through 5014 against the audit
     capture. This is the first playable milestone without a development
     fixture. sf-rng-inputs-01 matches through round 1's KO.
  4. C-NEG: the held P2 input moved by 7 frames (one fold pass) is caught.
  5. HOST smoke check: the power-on launcher plays into the fight with a
     scripted coin and start.
- **Size.** About 0.
- **Label.** PO.
- **Blocked by.** F4, F15, F33, F34.

### F50. Ranking insertion and the new entry on the ranking screen
- **Delivers.**
  - The insertion 0x425c4–0x4268a (51).
  - The marked-entry blink 0x42750–0x427e0 (32), with the fill 0x3b904 (the
    0x3b914 fill is F1's).
- **Acceptance.**
  1. C-NAT from F42's fixtures in both captures through:
     - the shifted entries and the new entry's initials;
     - the cleared score;
     - the ranking screen with the blinking entry;
     - 0x3f and the clears;
     - then 0x3eec2, the mode-4 start and Ken's intro.

     The comparison is full on every frame, inside F42's window, which
     runs through Ken's intro.
  2. 0xff8e6e–0xff8f0d (the 10 × 16-byte table) equals the reference's
     after the insertion.
  3. Not covered: P2's name entry, and both players entering names in one
     game over (open decision 3).
- **Size.** About 86 [code].
- **Label.** DF / CF.
- **Blocked by.** F16, F42.

### F51. Continuation and buy-in DIPs
- **Delivers.** Verification of the consumers of 0xffc000 and 0xffc001
  under non-default settings.
- **Acceptance.**
  1. C-REF:
     - sf-no-continue-01 (Game Continuation = None, about 5,700 frames),
       with checkpoints every frame from 2389 through 0x3ccae;
     - sf-buyin-01 (a buy-in setting below the country byte, then P2 start
       in a fight), with checkpoints every frame from 2389 through the
       watch's return.
  2. C-NAT for sf-no-continue-01: the loss goes 0x3efda → 0x3f2f0 → 0x3ccae,
     through to 0x3ccae.
  3. C-NAT for sf-buyin-01: 0x3f4a2 refuses and the watch returns to
     0x3f41e.
  4. Any frame difference from the default captures other than 0xffc000 /
     0xffc001 is recorded.
- **Size.** 0.
- **Label.** DF / CF.
- **Blocked by.** F2, F25, F41.

### F52. 10-second bonus after stage 1 (0x41740)
- **Delivers.**
  - Copy 1 0x41740–0x419fc (137).
  - 0x44174 / 0x44192 (14).
  - The object handlers 0x434e4, 0x4352c and 0x4357a (about 40).
  - It uses F5's 0x3baf6 and 0x3b934, and F25's 0x42f1e.
  - The resume point 0x41814.
- **Acceptance.**
  1. C-REF: a variant of sf-win-04, equal to it through the divergence,
     with checkpoints every frame over the bonus in both captures.
  2. C-NAT in both captures. One breaks every object. The other times out
     (0x14f74, 0x96).
  3. Each break (spark 0x18ec4, the broken stream, 0xff8f1c / 1f, the
     score) matches on its frame.
  4. The run continues through 0x3efc4, the flight, the VS, stage 2's intro
     and ROUND 1.
  5. The game-mode-4 tests at 0x4175e, 0x418ac and 0x41962 are listed.
     Their mode-4 branches are labelled unexercised, as in F29 [code].
- **Size.** About 191 [code].
- **Label.** DF.
- **Blocked by.** F43, F44.

### F53. Ryu beats stage 2's first opponent (0x175bc)
- **Delivers.** 9 entries (new spark 0x18944) and sf-win-05.
- **Acceptance.**
  1. C-REF on sf-win-05, equal to sf-win-04 through the divergence, with
     checkpoints every frame over the fight.
  2. C-NAT from the round-1 fixture through the next opponent's setup.
  3. The entries are traceable. C-DOC lists landed and unexercised hit
     functions.
- **Size.** Entries, plus a search.
- **Label.** DF.
- **Blocked by.** F44.

### F54. Listening review of every tune, effect and sample (person)
- **Delivers.** The user's verdict on F9's renders of F22, F23, F24 and F38
  against MAME's WAVs.
- **Acceptance.**
  1. The kit shows onset, RMS and correlation per command.
  2. A verdict is recorded per tune, effect group and sample bank.
  3. Each finding is opened as a ticket.
- **Size.** 0.
- **Label.** PERSON (ready-for-human).
- **Blocked by.** F9, F22, F24, F38. (F23 comes in through F38.)

### F55. Attract demo to its round end, and the attract loop from power-on
- **Delivers.** The demo's end 0x3ce8c–0x3ceb8 (9 [code]), F1's kill of
  slot 2 (0x3ceb2), and every fight-engine branch that F48's probe lists.
  The attract then loops back to the wall through F15's loop head.
- **Acceptance.**
  1. C-NAT from 794, and from power-on with no fixture, through:
     - the demo's round end, whichever outcome it is;
     - 0x3f;
     - the kill;
     - the second wall, title and ranking, up to the next demo setup.

     Full comparison uses sf-boot-01, sf-attract-01 and sf-attract-demo-01's
     windows.
  2. Every routine the probe listed is translated or reported. If they
     total more than about 400, they are split into separate tickets before
     this one starts.
  3. The round timer's ticks are 19 frames apart throughout. If not, each
     overrun frame is reported as a scope question, never covered by a
     tolerance.
  4. The input table's wrap matches if it is reached.
  5. C-NEG: the round end one frame late.
- **Size.** 9 known, plus the probe's branches. Oversized risk.
- **Label.** PO and DF, with no player input.
- **Blocked by.** F15, F48.

### F56. Name entry end to end from a won one-player fight
- **Delivers.** Verification of the whole path from sf-name-entry-01's and
  -02's 2389 checkpoints.
- **Acceptance.**
  1. C-NAT through:
     - the Retsu match won against the live CPU;
     - Geki's FIGHT!;
     - the challenger;
     - the versus lost;
     - the name entry;
     - the ranking;
     - Ken's carry-on, to the first untranslated branch.

     Full comparison covers F42's windows. The frames.tsv fields are
     compared between them.
  2. C-DOC marks which spans are complete gameplay and which are controlled.
- **Size.** 0.
- **Label.** DF, with CF spans.
- **Blocked by.** F41, F50.

### F57. Ryu beats stage 2's second opponent (0x1782c)
- **Delivers.** 8 entries (spark 0x189a4) and sf-win-06, through bonus 3
  into stage 3.
- **Acceptance.**
  1. C-REF on sf-win-06, prefix-equal to sf-win-05, with checkpoints every
     frame over the fight and the bonus.
  2. C-NAT from the round-1 fixture through 0x3ef96's dispatch to 0x41a02.
  3. The entries are traceable, with landed and unexercised hit functions
     listed.
- **Size.** Entries, plus a search.
- **Label.** DF.
- **Blocked by.** F53.

### F58. Power/timing bonus after stage 2 (0x41a02)
- **Delivers.** Copy 2 as parameters of F29's translation: the table
  0x41f5e, its result tables, one addressing difference, and the resume
  point 0x41aa6.
- **Acceptance.**
  1. C-REF: a variant of sf-win-06, with checkpoints every frame over the
     bonus.
  2. C-NAT for both outcomes, through the flight to stage 3, the VS, the
     intro and ROUND 1.
  3. C-DOC lists every data difference from copy 1 with its address.
  4. The game-mode-4 test at 0x41a08 is listed. So is 0x414f6's test
     against mode 2, in F29's helper 0x414f0, which the resume point
     0x41aa6 calls. Their mode-4 and other-mode branches are labelled
     unexercised, as in F29 [code].
- **Size.** 283 reachable, a near-copy (ratio 0.995 [code]), translated as
  parameters.
- **Label.** DF.
- **Blocked by.** F29, F43, F57.

### F59. Ryu beats stage 3's first opponent (0x17a9c)
- **Delivers.** 8 entries, including 0x47838 (its owner is inferred [inf]),
  and sf-win-07.
- **Acceptance.**
  1. C-REF on sf-win-07, prefix-equal to sf-win-06, with checkpoints every
     frame over the fight.
  2. C-NAT through the next opponent's setup.
  3. C-DOC settles which streams use 0x47838.
- **Size.** Entries, plus a search.
- **Label.** DF.
- **Blocked by.** F57.

### F60. Ryu beats stage 3's second opponent (0x17d0c)
- **Delivers.** 7 entries and sf-win-08, through bonus 4 into stage 4.
- **Acceptance.**
  1. C-REF on sf-win-08, prefix-equal to sf-win-07, with checkpoints every
     frame over the fight and the bonus.
  2. C-NAT through 0x3ef96's dispatch to 0x41fe6.
  3. The entries are traceable.
- **Size.** Entries, plus a search.
- **Label.** DF.
- **Blocked by.** F59.

### F61. 10-second bonus after stage 3 (0x41fe6)
- **Delivers.** Copy 2 as parameters of F52, plus the handlers 0x435c2 and
  0x4360a for templates 0x18e84 / 0x18ea4.
- **Acceptance.**
  1. C-REF: a variant of sf-win-08, with checkpoints every frame over the
     bonus.
  2. C-NAT for both outcomes, with all four objects broken in one, through
     the flight to stage 4 and ROUND 1.
  3. C-DOC lists copy 2's data differences.
  4. The game-mode-4 tests at 0x42004, 0x42164 and 0x4221a are listed.
     Their mode-4 branches are labelled unexercised, as in F29 [code].
- **Size.** About 26 new, plus a 140-instruction near-copy (ratio 0.989
  [code]).
- **Label.** DF.
- **Blocked by.** F52, F60.

### F62. Ryu beats Adon (0x17f7c)
- **Delivers.** 6 entries, 0x4558a (19), and sf-win-09.
- **Acceptance.**
  1. C-REF on sf-win-09, prefix-equal to sf-win-08, with checkpoints every
     frame over the fight.
  2. C-NAT through Sagat's setup.
  3. 0x4558a's turn branches are exercised, or the unexercised ones are
     listed.
- **Size.** About 19, plus entries and a search.
- **Label.** DF.
- **Blocked by.** F60.

### F63. Ryu beats Sagat (0x181ec) and reaches the ending
- **Delivers.** 6 entries, 0x45ace (6), the projectile handler 0x43652 (20),
  and sf-win-10 through the match win and the whole ending, with checkpoints
  every frame over the fight and the ending.
- **Acceptance.**
  1. C-REF on sf-win-10, prefix-equal to sf-win-09.
  2. C-NAT through the last match-won card (0x159b2, 0xba, exercised for
     the first time) and 0x3ef96's dispatch to 0x44a40.
  3. The projectile's hit (0x43652), the explosion 0x186fc, the chip damage
     and the slow motion (event 9) match where the reference reaches them.
- **Size.** About 26, plus entries and a search.
- **Label.** DF.
- **Blocked by.** F62.

### F64. The ending (0x44a40)
- **Delivers.** 0x44a40 (110): five scenes through 0x44c1c by 0xffc006, and
  the staff texts through F34's writers. The resume point is 0x44a60. It
  reuses F1's 0x3b7a8 (called at 0x44c10) and F29's 0x3efc4 (jumped to at
  0x44c16) [code].
- **Acceptance.**
  1. C-NAT from the ending fixture on sf-win-10 through 0x3efc4 (stage 4 to
     5), including the scrolls, palettes, texts and 0x32 / 0xff / 0x36 on
     their frames.
  2. The run then goes into F25's 0x3f2d0 path, to the first untranslated
     branch.
  3. Only the Japan-start variant is verified. The USA start (country 2) is
     selectable at the default DIPs [code 0x44a76; F34]; it is listed as not
     covered unless open decision 2 tickets it. Countries 1 and 3 need the
     four-country DIP and a winning chain from each; they are listed as not
     covered unless open decision 11 tickets them.
  4. The game-mode test at 0x44b48 (0xffc002 against 2, then beq 0x44b54)
     is listed. Its branch for any other mode, which reads the second word
     of 0x44d5c, is labelled unexercised, because Ryu's chain runs in
     mode 2 [code]. Ken's ending is open decision 10.
- **Size.** About 110 [code].
- **Label.** DF.
- **Blocked by.** F29 (0x3efc4), F34, F63.

### F65. A complete one-player game from power-on
- **Delivers.** sf-full-1p-01: sf-win-10's inputs extended through the name
  entry, the game over and the return to the attract. It is compared
  natively from power-on.
- **Acceptance.**
  1. C-REF, prefix-equal to sf-win-10. Its checkpoint windows follow open
     decision 7. Per-frame checkpoints over a session of about
     40,000–50,000 frames would cost about 17–21 GB per run, and twice that
     for the two runs [inf: 419 KB per checkpoint].
  2. C-NAT from power-on, with no fixture, through the return to the
     attract, including the random state from frame 0.
  3. No untranslated branch is reached. Any that is reached fails the
     ticket and becomes a new one.
  4. C-NEG: one input moved by one frame is caught at its first differing
     frame.
- **Size.** 0.
- **Label.** PO.
- **Blocked by.** F49, F50, F58, F61, F64.

### F66. A two-player session from power-on
- **Delivers.** sf-full-2p-01, compared natively from power-on:
  - the first coin during the demo fight (the attract start kills an
    occupied slot 2);
  - a second coin and a 2P start;
  - the versus, won by P2, so the winner plays the CPU in mode 4;
  - a mid-game coin and P1 joining P2's game as the challenger (the watch's
    mode-4 branch, D2 = 1 at 0x3f448 [code]);
  - a continue;
  - the game over with no credit left, and the return to the attract.
- **Acceptance.**
  1. C-REF. Its checkpoint windows follow open decision 7, with the same
     cost note as F65. They include a checkpoint after the boot's ranking
     fill and one at the session's end.
  2. No score in sf-full-2p-01 ranks. 0x42422 runs for each versus loser
     (0x3eebc), for a player the CPU beats (0x3ef18, 0x3ef4a) and for both
     players at the game over (0x3f2d0–0x3f2ea) [code]. A ranking score
     would lead to name entry, including P2's, which open decision 3
     defers. This is checked in the reference before the native
     run: no name-entry screen appears, and the ranking table (F1's
     0xff8e6e–0xff8f0d) is the same at the session's end as after the
     boot's fill. If a score ranks, either the script is changed until none
     does, or F50 joins the blockers and open decision 3 is settled first.
  3. C-NAT from power-on through the return to the attract.
  4. No untranslated branch is reached. Any that is reached fails the
     ticket and becomes a new one.
  5. C-NEG: one input moved by one frame is caught at its first differing
     frame.
- **Size.** 0.
- **Label.** PO, with CF spans for the versus, where both sides are
  scripted, as in F26, F40, F41, F42 and F46.
- **Blocked by.** F25, F39, F40, F41, F46, F49, F55.

### F67. DIP settings from power-on: flip and the country count
- **Delivers.** Power-on reruns of F18's and F47's scenarios. They show that
  the boot decode reaches the states those tickets verified from fixtures.
  - sf-flip-02: Flip On, the fixed fight's inputs from power-on through
    5014: the flipped warning, the attract, the start, the select and the
    fight.
  - sf-select4-03: Countries = 4, from power-on through the select's exit
    and event 2's request.
- **Acceptance.**
  1. C-REF for each, with the readback as requested. Full checkpoints over
     0–830, at each fixture frame F18 and F47 used, and over their compared
     windows. The frames.tsv fields elsewhere.
  2. C-NAT from power-on over each scenario's span. At each fixture frame
     the owned RAM equals the fixture.
  3. C-NEG: the native run with Flip Off in its inputs only fails at the
     first frame the flip shows.
- **Size.** 0.
- **Label.** PO / CF.
- **Blocked by.** F18, F47, F49.

### F68. DIP settings from power-on: attract and demo
- **Delivers.** Power-on reruns of F37's scenarios through the demo's round
  end and the next attract loop, and one coin check.
  - sf-attract-music-off-02 and sf-demosound-off-02: power-on, no input,
    through the demo's round end and the second ranking.
  - sf-demosound-coin-01: Demo Sounds Off, then a coin during the title.
    The coin's 0x0f goes through the unfiltered entry 0x3b802 (at 0x482da)
    and is latched. The filtered entry would drop it while 0xff8003 is 0
    [code 0x3b7e8–0x3b7fa]. This is the first scenario that tells the two
    entries apart.
- **Acceptance.**
  1. C-REF for each, with the readback as requested and F67's checkpoint
     rule.
  2. C-NAT from power-on over each span. With Demo Sounds Off the filtered
     entry passes nothing while 0xff8003 is 0. The boot commands (frames
     1–3) and the coin's 0x0f still reach the latch, through 0x3b802.
  3. C-NEG: the coin's 0x0f routed through the filtered entry fails at the
     coin's latch frame.
- **Size.** 0.
- **Label.** PO / CF.
- **Blocked by.** F33, F37, F55.

### F69. DIP settings from power-on: the fight
- **Delivers.** Power-on reruns of F19's, F20's and F21's scenarios: Easy /
  150, Difficult / 200, Very Difficult / 250, Attract Music Off in the
  fight, Speed Slow and Freeze. Each runs through its match decision. Normal
  / 100 is F49's run.
- **Acceptance.**
  1. C-REF for each, with the readback as requested and F67's checkpoint
     rule.
  2. C-NAT from power-on over each span. At each fixture frame the earlier
     ticket used, the owned RAM equals the fixture.
  3. C-NEG: the round-time setting decoded one step off fails at the first
     TIME digit.
- **Size.** 0.
- **Label.** PO / CF.
- **Blocked by.** F19, F20, F21, F49.

### F70. DIP settings from power-on: coinage, continuation and buy-in
- **Delivers.** Power-on reruns of F35's coinage scenario and F51's
  no-continue and buy-in scenarios.
- **Acceptance.**
  1. C-REF for each, with the readback as requested and F67's checkpoint
     rule.
  2. C-NAT from power-on over each span. At each fixture frame the earlier
     ticket used, the owned RAM equals the fixture.
  3. C-NEG: Coin A decoded as 1C1C fails at the first slot-1 coin.
- **Size.** 0.
- **Label.** PO / CF.
- **Blocked by.** F35, F49, F51.

### F71. Both sound programs over the complete game from the native latch
- **Delivers.** Sound-enabled runs of F65 and F66, with both translated
  sound programs on the native latch.
- **Acceptance.**
  1. Two identical audio runs of each.
  2. At the reference's write times every device is identical over the
     whole game.
  3. At the launcher's latency, every differing frame is listed per scene
     and attributed to the latency band.
  4. No untranslated sound branch is reached.
- **Size.** 0.
- **Label.** PO.
- **Blocked by.** F8, F22, F23, F65, F66. F22 and F23 may translate small
  sound branches. F24 and F38 are size 0 and only verify, so they are not
  gates here. They still come before F72 through F54.

### F72. Full-game completeness and fidelity audit
- **Delivers.** A pass or fail account against every user story and testing
  decision in the specification, the final run of F11's ledger, and a
  disposition for every "Not covered" item.
- **Acceptance.**
  1. The final ledger shows no address that was executed and not
     translated.
  2. Every report site in the ledger (note() or unrecovered) that no
     scenario reached has a disposition: unreachable under MAME (with the
     reason), deferred (with its open decision), or not covered (with the
     ticket that would reach it).
  3. Each adopted MAME behaviour is recorded with its source location:
     - the 500-ns slice;
     - the NMI at a slice start;
     - the latency band;
     - the resampler;
     - the coin lockout (coin_w → coin_lockout_w, sf-mame0289.cpp:110–116);
     - the power-on RAM contents (F1);
     - the fixed IPT_UNKNOWN SYSTEM bits (sf-mame0289.cpp:558–570);
     - the vblank held (irq1_line_hold, sf-mame0289.cpp:758) across the
       reset's RAM clear.
  4. The person checks (F30, F31, F54) are cited.
  5. The audit states that it does not certify the complete port. Test
     mode (F73–F76), F77 and the deferred items come after it. Nothing that
     does not pass is certified.
- **Size.** 0.
- **Label.** Audit.
- **Blocked by.** F7, F11, F13, F14, F17, F30, F31, F32, F36, F45, F54, F56,
  F67, F68, F69, F70, F71. These are the pre-test-mode tickets that no other
  ticket depends on. Every other ticket from F1 to F71 is an ancestor of one
  of them.

### F73. Service mode from power-on: RAM test, input display and sound test
- **Delivers.**
  - The reset's test branch (0x3b01e–0x3b02a, where F1 reported it).
  - The RAM test 0x4834e–0x483f2 (about 50), with no stack, its duration
    taken from summed costs.
  - The input, DIP and sound-test task 0x483f8–0x48642 (about 145).
  - The exit trap #3, 0x3b29a–0x3b2b2 (7). Its one caller is 0x48618 in
    this task, after it spawns the menu [code].
  - The helpers 0x3b5c8, 0x3bffc / 0x3c014 and 0x3b6b4.
  - `Io` gains the unmapped ports 0xc00004, 0xc00006 and 0xc0000e with
    MAME's unmap value (sfus_map's nopr() with unmap_value_high
    [code sf-mame0289.cpp:167, 173–174, 178]).
- **Acceptance.**
  1. C-REF: sf-service-01 (service on, every frame checkpointed).
  2. C-NAT from power-on through the menu entry, with the full comparison
     on every frame.
     - The RAM test writes patterns to work RAM 0xff8000–0xffffff and video
       RAM 0x800000–0x800fff, then clears the one and fills the other
       (0x48358–0x483d4) [code]. Its progress at a frame end depends on its
       cycle cost. So on the frames whose end falls inside 0x4834e–0x483d4,
       only those two regions and the screen drawn from video RAM may
       differ. Each such frame is listed with its differing byte ranges.
     - A frame whose end falls inside the reset's clear (0x3b012–0x3b01a)
       gets no allowance here. It is held to F1's rule (F1 acceptances
       2–4): C-NAT applies, the clear's point is placed by summed costs,
       and the part not yet cleared is read from sf-service-01's own
       frame-0 dumps, as F1 reads sf-boot-01's.
     - Every other field matches on every frame.
     - From the first frame end after the 0x55 write at 0x483d8, RAM
       matches under C-NAT. Owned RAM is byte-identical. Only the task
       slots' saved SR/PC/USP words and the stack memory 0xff8106–0xff8631
       may differ. These are written from then on: the spawn at 0x483e8
       saves slot 0's PC, and the task sets A7 to 0xff82a2 (0x483f8) and
       pushes return addresses there [code]. That frame equals the
       reference's, and the number of RAM-test frames is derived from
       summed costs, not fitted.
     - The first test-screen frame equals the one F2 recorded.
  3. The run exercises exit at 0x48618 and reports 0x48644.
  4. The unmapped-port value is recorded as adopted MAME behaviour, for F77.
- **Size.** About 252 [code: the first draft's 245, plus exit's 7].
- **Label.** SM.
- **Blocked by.** F72. This is an ordering edge from the user's decision of
  2026-10-22. F1 and F2 are its technical prerequisites and are ancestors
  of F72.

### F74. Service menu and its first four screen tests
- **Delivers.** The menu 0x48644–0x487f6 (7 items, created and killed as
  task 2) and items 0x487f8, 0x488f8, 0x48858 and 0x48990–0x48b68.
- **Acceptance.**
  1. C-REF: sf-service-02, from power-on with Service On, with checkpoints
     every frame, as sf-service-01. Its inputs [code]:
     - P1 up with a kick whose latched strength exceeds 4 leaves the input
       screen for the menu (0x485c4–0x485dc);
     - in the menu, P1 up or down moves the cursor over the 7 items, with
       wrap (0x48692–0x48712);
     - a P1 punch whose latched strength exceeds 4 enters the item under
       the cursor (0x48730–0x48772);
     - P1 up with such a kick leaves the item (0x4877e–0x4879e).

     Items 1–4 are entered and left in turn. In each, every direction of
     each player is held alone for at least 60 frames, then each button
     is pressed once with no direction held, before the item is left. Only
     item 4 shows a direct input read, of IN1 at 0x48a40 [code; reads
     through other registers were not excluded]. Items 5–7 are then
     entered once each. Which buttons give a latched strength above 4 is
     [inf].
  2. C-NAT through the last exit from item 4.
  3. Items 5–7 report their entry addresses.
- **Size.** About 290 [code].
- **Label.** SM.
- **Blocked by.** F73.

### F75. Service sprite test and output cycle
- **Delivers.** 0x48b6a–0x48da2 and 0x490f8–0x492d5, with 0x3b93c and 0x3b664
  (0xc00012).
- **Acceptance.**
  1. C-REF: sf-service-03a, from power-on with Service On, with
     checkpoints every frame, as sf-service-01. It reaches the menu with
     F74's inputs. The sprite test (item 5) reads IN1 as a word [code
     0x48c1e], so each direction of each player is held alone for at least
     60 frames, then the item is left. The output cycle (item 7) shows no
     direct input read [code 0x490f8–0x492d5]. It is entered, run for at
     least 600 frames, and left.
  2. C-NAT, including object RAM and 0xff8684 / 0xff8686.
  3. The ledger shows these ranges translated.
- **Size.** About 225 [code].
- **Label.** SM.
- **Blocked by.** F74.

### F76. Service scroll test
- **Delivers.** 0x48da4–0x490f6.
- **Acceptance.**
  1. C-REF: sf-service-03b, from power-on with Service On, with
     checkpoints every frame, as sf-service-01. It reaches the menu with
     F74's inputs. The scroll test (item 6) reads both bytes of IN1, P1's
     and P2's directions [code 0x48e96, 0x48f40], so each direction of
     each player is held alone for at least 60 frames, then the item is
     left.
  2. C-NAT.
  3. The ledger shows the range translated. With F75, every code address in
     0x4834e–0x492d5 is covered.
- **Size.** About 178 [code].
- **Label.** SM.
- **Blocked by.** F74.

### F77. Append the test-mode result to the audit
- **Delivers.** An addendum to F72's audit: the results of F73–F76, the
  ledger rerun over 0x4834e–0x492d5 and its helpers, and the unmapped-port
  value (F73) added to the adopted MAME behaviour.
- **Acceptance.**
  1. The ledger shows every executed test-mode address translated, or
     reported with a disposition.
  2. The addendum lists every deferred item and every open decision with
     its outcome.
  3. It states which requirements pass and which are deferred or fail. It
     certifies nothing that does not pass.
- **Size.** 0.
- **Label.** Audit.
- **Blocked by.** F75, F76.

### F78. Re-check waves 3–16 and publish them
- **Delivers.** Decision 1 published waves 0–2 first. Once F15 (the
  warning screen from power-on) and F49 (the fixed fight from power-on)
  are done, this ticket re-checks F50–F77 against the code of that day and
  publishes them, with this file updated.
- **Acceptance.**
  1. Every size and address of F50–F77 is re-read against the listing and
     the code. Each change is recorded in section 1.
  2. A ticket whose scope, size or blockers change materially is shown to
     the user before it is published. The rest are published as approved
     on 2026-10-23.
  3. The issues carry the same body form, labels and native blocked_by
     edges as F1–F49. `arcade-flow-publication.json` maps every published
     id.
- **Size.** 0.
- **Label.** TOOL (planning).
- **Blocked by.** F15, F49.

## 5. Parallel starts, waves and the critical path

A ticket's wave is one more than the highest wave among its blockers.
Computed from the edges in section 4.

| Wave | Tickets | Count |
| --- | --- | --- |
| 0 (no blockers) | F1–F14 | 14 |
| 1 | F15–F36, F39 | 23 |
| 2 | F37, F38, F40–F49 | 12 |
| 3 | F50–F55, F67, F69, F78 | 9 |
| 4 | F56, F57, F66, F68, F70 | 5 |
| 5 | F58, F59 | 2 |
| 6 | F60 | 1 |
| 7 | F61, F62 | 2 |
| 8 | F63 | 1 |
| 9 | F64 | 1 |
| 10 | F65 | 1 |
| 11 | F71 | 1 |
| 12 | F72 | 1 |
| 13 | F73 | 1 |
| 14 | F74 | 1 |
| 15 | F75, F76 | 2 |
| 16 | F77 | 1 |

**The critical path** is 17 tickets long:
F6 → F28 → F44 → F53 → F57 → F59 → F60 → F62 → F63 → F64 → F65 → F71 →
F72 → F73 → F74 → F75 / F76 → F77.

- F27 joins it at F44 in the same wave as F28.
- The winning-script chain (F6 to F64) cannot run in parallel. Each
  fight's reference needs the one before it won [code: the progression
  through 0x3ef88 / 0x3efc4].
- Waves 0–4 hold 62 of the 77 tickets. Waves 5–16 are mostly that chain,
  the complete-game runs, the audit and test mode.

## 6. Merges, record corrections and requirements not ticketed

### Merges and conflicts resolved

1. Four readers each proposed DIP tooling. It is now one ticket, F2.
2. Traps #0 and #1 were claimed by five readers. They are in F1. Exit (#3)
   moved to F73, the ticket of its only caller.
3. Slot 3's attract texts were claimed twice. They are in F3.
4. 0x3e586–0x3e5c5 was split at the sleep. F34 owns up to 0x3e596, and F4
   owns 0x3e59c onwards [code].
5. 0x42422's check was claimed twice. It is in F5.
6. The versus game over and the one-player game over were merged into F25.
7. The ranking screen is split. F16 owns 0x4268c–0x4274e, and F50 the
   marked-entry path.
8. Flip moved to the 2389 fixture (F18). The power-on flipped warning is in
   F67.
9. A proposed Ken one-player capture (coin, then P2 start) cannot reach
   mode 4 [code]. It was merged into F40.
10. The "attract loop from power-on" slice was merged into F15 and F55.
11. sf-credit-select-01 is recorded by F12.
12. The scattered continuation and buy-in variants were gathered into F51.
13. The check of a challenger joining P2's game (0 instructions) was folded
    into F66, whose session now includes it.
14. The coin-control helpers 0x3b654 and 0x3b6a2 moved from F1 to F12, with
    the port write. F15 now waits for F12.
15. The every-DIP-from-power-on ticket was split into F67–F70, by the part
    of the game that consumes each setting.

### Record corrections and their owners

| Record | Correction | Owner |
| --- | --- | --- |
| first-frame.md (794 and 790); sf_fixed_fight_01.json's 790 note | 794 is a consistent state [cap]; the 790 note is reconciled | F3, F15 |
| sf_fixed_fight_01.json's 794 note ("transition frame") | a consistent state [cap] | F3 |
| sf_fixed_fight_01.json's 1500 note ("title with credit") | the start screen, slot 1 at 0x3dfce [code + cap] | F12 |
| sf_fixed_fight_01.json's 1980 note ("VS screen") | event 4's stage intro, per the re-check; F4 confirms from its run | F4 |
| sf_fixed_fight_01.json's 1900 comment ("confirm") | the select ends by its timeout [cap] | F34 |
| round-timer.md "not set in any capture"; ai-tables.md 13 → 15 pointers | removed; corrected | F34 |
| tasks.rs:402–404 and round-timer.md (timer flags); tasks.rs:666 and tasks.rs:839–843; round-outcomes.md and match-win.md (0x42422) | relabelled | F5 |
| music-stops.md window D | 0x39 is the attract ranking's tune | F16 |
| mame-baseline.md's attract timeline | replaced by the exact one | F3 |
| milestone-audit.md "P2-versus-CPU mode is not exercised"; latch-timing.md:132 (game mode 4) | a dated follow-up note | F40 |
| hadoken.md, specials.md, crouch-attacks.md, jump-attacks.md, blocking.md, round-outcomes.md "Not covered" | the one-player results | F13, F14, F32 |

### Requirements not ticketed, and why

- **A general frame-overrun model** (the game's slowdown). It needs every
  task's cycle cost, which ADR-0001's translation does not keep [doc
  task-scheduler.md, latch-timing.md]. This is a scope question, put to the
  user as open decision 12. F55 reports any overrun it meets.
- **Modelling the interrupt latency inside the 45–138 band.** That would be
  an ADR change, and the scope decision of 2026-10-22 stands [doc]. F8 only
  measures.
- **The ending's four-country starts** (countries 1 and 3, table 0x44c1c).
  Each needs the four-country DIP, which MAME lists, and a winning chain
  from that start. F64 lists them as not covered. They are open decision
  11. The USA start is reachable at the default DIPs and is open decision
  2.
- **Ken's one-player game past Retsu (game mode 4).** F40 stops at the next
  opponent's setup. The bonus code tests the game mode at nine places
  [code]. It tests mode 4 at 0x40d36, 0x4175e, 0x418ac, 0x41962, 0x41a08,
  0x42004, 0x42164 and 0x4221a, and mode 2 at 0x414f6 in the shared
  helper 0x414f0. The ending tests mode 2 at 0x44b48 [code]. F29, F52,
  F58, F61 and F64 label the branches that Ryu's chain does not take
  unexercised. This is open decision 10.
- **P2's name entry, and both players entering names in one game over.**
  These need a P2 ranking score and a P2 loss. F50 lists them as not
  covered. They are open decision 3.
- **Coin slots 3 and 4.** They are never configured or debounced [code].
- **The Game Continuation and Buy-In values MAME does not list.** MAME's DIP
  definitions list 6 of the 8 values of each [code sf-mame0289.cpp:521–527,
  538–544].
  - The boot decode indexes an 8-entry table by the inverted bits (0x3c16a
    for Game Continuation, 0x3c188 for Buy-In). The entries for the 2
    unlisted values of each equal the entry for the listed None [code
    0x3c156–0x3c18f]. So 0xffc000 and 0xffc001 get the same byte as under
    None.
  - The only direct reads of DSW2 found are the boot decode's and test
    mode's port loop at 0x484d4–0x484f6, which shows the raw bits [code].
    Reads through computed pointers were not excluded [inf].
  - So in the game they act as None, unless such a read exists. Only test
    mode's DIP display would differ.
  - The earlier reason, that MAME's settings cannot reach them, was not
    verified. F2 offers port words. The local copy of MAME's
    luaengine_input.cpp passes a field value on without checking it
    against the listed settings [code tools/reference/local/mamesrc/luaengine_input.cpp:531–537; its
    MAME version is not recorded]. ioport.cpp, which applies the value, is
    not held locally, so whether MAME keeps it is unknown [inf].

  They are put to the user as open decision 15.
- **Music paths dead with this ROM's data** (slot B, noise, the silently
  advancing saved tune, 0x0e62, 0x02a5) [doc]. They stay reported. F22 and
  F23 confirm they are unreached.
- **0x3b0f0's branch, 0x3b378 and 0x3b602.** They depend on SYSTEM bits 7
  and 6, which MAME defines as IPT_UNKNOWN (sf-mame0289.cpp:561–562), so
  MAME's inputs cannot reach them [code + doc]. The vblank already reports
  0x3b0f0 and 0x3b378 (tasks.rs:2102, 2123). F1 reports 0x3b602. F72 gives
  all three the disposition "unreachable under MAME".
- **Trap #4 (0x3b72a), the stubs 0x3b2ca–0x3b2e2 and the assertion
  0x3b28c–0x3b298.** No direct reference was found [code]. Calls through
  pointers were not excluded. The ledger (F11) would show them if a scenario
  reached them.
- **A host option to put Ken on the P1 keys.** It would change only the
  host's key table. Ken stays P2, driven by P2's logical inputs whatever
  keys feed them [doc launcher.md, "Fixed keyboard bindings"]. The earlier
  reason, that it is outside the faithful baseline, was not verified
  [inf]. CONTEXT.md sets that baseline against changes to gameplay, and a
  key table does not change gameplay [doc]. It is put to the user as open
  decision 16.

## 7. Open decisions for the user

1. **Publish all 77 tickets at once, or in waves.** The waves table gives
   the cut points. One choice is waves 0–2 first (49 tickets, with F18 as
   decision 14 says), then the rest after the first power-on results (F15,
   F49).
2. **The USA-start ending chain.** USA (country 2) is selectable at the
   default DIPs, and the ending indexes its scenes by the starting country
   [code 0x44a76]. Ticket it as a second winning-script chain from the USA
   start (about one search ticket per opponent, plus its ending [inf]), or
   defer it. Proposed: defer, listed as not covered in F64 and F72.
3. **P2's name entry and a two-name game over.** Defer (proposed), or
   ticket them after F66.
4. **Fetching mame0289's tilemap.cpp for F18.** It is not held locally. F18
   needs it to derive the flipped tilemap arithmetic. The file would stay
   local, like sf-mame0289.cpp. F18 waits for this decision.
5. **The fallback if F6's search fails at a later opponent.** Either the
   search may use continues (then F39 enters the chain, and every later
   chain ticket waits for it), or the strategy family is widened first.
6. **The launcher latency after F8.** Keep 92, or change it, after F8's
   per-fight table. No tolerance is adopted either way.
7. **The disk budget.** On 2026-10-23, drive C: had about 105 GB free, and
   the local captures used about 55 GB (measured with df and du). One
   checkpoint is about 419 KB per run [cap]. Per-frame checkpoints over F65's
   or F66's whole session would need about 17–21 GB per run, and twice that
   for the two runs [inf]. Long captures such as F19's add several GB each
   [inf].
   Options:
   - compare as the capture streams, keeping only hashes of the two runs'
     dumps;
   - keep full checkpoints only in named windows, with the frames.tsv fields
     between them;
   - raise the budget or move captures to another disk.
8. **The DIP mechanism for F2** (for information). A field write plus a
   reset, or a generated cfg. F2 decides from the pinned MAME's behaviour.
9. **Where F36's bindings file lives.**
10. **Ken's one-player game past Retsu (game mode 4).** F40 verifies Ken's
    first CPU fight and stops at the next opponent's setup. The bonus code
    branches on the game mode at nine places [code: mode 4 at 0x40d36,
    0x4175e, 0x418ac, 0x41962, 0x41a08, 0x42004, 0x42164 and 0x4221a; mode
    2 at 0x414f6]. The ending branches on mode 2 at 0x44b48 [code]. Ticket
    a Ken winning chain in mode 4 (a search ticket per opponent, plus the
    four bonuses' and the ending's mode branches [inf]), or defer it.
    Proposed: defer. F29, F52, F58, F61 and F64 label the branches that
    Ryu's chain does not take unexercised, and F40 and F72 list Ken's game
    past Retsu as not covered.
11. **The ending's four-country starts (countries 1 and 3).** They need
    Countries = 4, a DIP setting MAME lists, and a winning chain from each
    start [code 0x44a76]. Ticket them (a chain each [inf]), or defer them.
    Proposed: defer, listed as not covered in F64 and F72.
12. **A general frame-overrun model.** Modelling the game's slowdown needs
    every task's cycle cost, which ADR-0001's translation does not keep
    [doc task-scheduler.md]. Treat it as out of scope for this batch
    (proposed), with each overrun a ticket meets reported as a scope
    question, or plan it as an ADR change.
13. **The 30-instruction bound** (section 2). A size-0 ticket that reaches
    an untranslated branch of under about 30 instructions may translate it.
    A larger one becomes a new ticket. Approve this bound (proposed), or
    set another.
14. **How F18 is published before decision 4.** F18 cannot start until
    decision 4 is made. Hold it back from publication until then
    (proposed), or publish it as ready-for-human and relabel it when
    decision 4 is made.
15. **The Game Continuation and Buy-In values MAME does not list.** The
    boot decode gives 0xffc000 and 0xffc001 the same byte for them as for
    None [code 0x3c156–0x3c18f]. F17 verifies None's decode for both, and
    F51 runs Game Continuation = None. Whether F2's port words can set
    them in MAME is not verified (section 6). Leave them out of this batch
    (proposed exclusion), or, if F2 shows that MAME applies them, add one
    boot scenario per value to F17 and check test mode's DIP display of
    them in F73.
16. **A host option to put Ken on the P1 keys.** It changes only the host's
    key table (section 6). F36's rebind flow may already allow it, by
    binding P2's inputs to those keys once P1's are moved [inf: F36 is not
    built]. Leave it out of this batch (proposed exclusion), or add a swap
    of the two players' key tables to F36's acceptance.

### Decisions (2026-10-23)

The user answered every open decision:

| # | Decision |
| --- | --- |
| 1 | Publish waves 0–2 first (F1–F49), with F78 to publish the rest after F15 and F49 |
| 2 | Defer the USA-start ending chain |
| 3 | Defer P2's name entry and the two-name game over |
| 4 | Allow fetching mame0289's tilemap.cpp into tools/reference/local/ |
| 5 | Widen the search first; use continues only if that fails |
| 6 | Decided after F8's results |
| 7 | Keep full checkpoints only in each ticket's named windows; frames.tsv fields between them |
| 8 | Left to F2, from the pinned MAME's behaviour |
| 9 | `%APPDATA%\sf1-port\bindings.toml` |
| 10 | Defer Ken's one-player game past Retsu |
| 11 | Defer the four-country ending starts |
| 12 | No frame-overrun model; each overrun met is reported as a scope question |
| 13 | Approve the 30-instruction bound |
| 14 | Moot: decision 4 allows F18 to be published normally |
| 15 | Exclude the unlisted Game Continuation and Buy-In values |
| 16 | Exclude a host option to put Ken on the P1 keys |

## 8. Appendix: draft id mapping

The first draft numbered its tickets A1–A69. They were renumbered because
five tickets were added, one was split into four, one was folded into
another and a closing step was added. Every edge now points to a lower id.

| Draft | New | Draft | New | Draft | New | Draft | New |
| --- | --- | --- | --- | --- | --- | --- | --- |
| A1 | F1 | A19 | F21 | A37 | F40 | A55 | F59 |
| A2 | F2 | A20 | F22 | A38 | F41 | A56 | F60 |
| A3 | F3 | A21 | F23 | A39 | F42 | A57 | F61 |
| A4 | F4 | A22 | F24 | A40 | F43 | A58 | F62 |
| A5 | F5 | A23 | F25 | A41 | F44 | A59 | F63 |
| A6 | F6 | A24 | F26 | A42 | F47 | A60 | F64 |
| A7 | F7 | A25 | F27 | A43 | F48 | A61 | F65 |
| A8 | F8 | A26 | F28 | A44 | F49 | A62 | F66 |
| A9 | F9 | A27 | F29 | A45 | folded into F66 | A63 | F67–F70 |
| A10 | F10 | A28 | F30 | A46 | F50 | A64 | F71 |
| A11 | F11 | A29 | F31 | A47 | F51 | A65 | F72 |
| A12 | F15 | A30 | F33 | A48 | F52 | A66 | F73 |
| A13 | F12 | A31 | F34 | A49 | F53 | A67 | F74 |
| A14 | F16 | A32 | F35 | A50 | F54 | A68 | F75 |
| A15 | F17 | A33 | F36 | A51 | F55 | A69 | F76 |
| A16 | F18 | A34 | F37 | A52 | F56 | new | F13, F14, F32 (one-player combat breadth) |
| A17 | F19 | A35 | F38 | A53 | F57 | new | F45 (game over with a credit left), F46 (fresh 2P start) |
| A18 | F20 | A36 | F39 | A54 | F58 | new | F77 (closing step) |

## 9. Critic items not applied

None. All 25 review corrections were checked against the listing, the code
and the docs, and none was wrong in substance. Some are applied in a form
the review offered as an alternative, or in an equivalent form:

- **Item 1 (A13's edge on A1).** 0x3b654 and 0x3b6a2 went to F12 rather
  than being marked shared, so F15 is blocked by F12. 0x3b802 stays with
  F1, with the merge note in section 3.
- **Item 4 (A62's edges).** The challenger-joins-P2 check was folded into
  F66, not into F40.
- **Items 7 and 8 (coverage).** They became new tickets, F46 and F45,
  rather than extensions of the challenger or continue tickets. F46 reuses
  F12's sf-start-2p-01 and joins sf-move-01's existing versus comparison at
  its 2450 fixture.
- **Item 9.** Of the two options offered, 0x3b602–0x3b616 is reported as a
  note site. It is not translated.
- **Item 15.** Applied in F15 (name the latency, list each device's first
  differing frame at 51 and at 92, no tolerance). One clarification:
  both-chips.md shows the samples CPU differing at latency 92 from frame
  2619 on, which is already in round 1 [doc both-chips.md:57, 73–79].

### Checker items not applied

The second check listed 24 defects, D1–D24. Each was checked against the
listing, the code and the docs. 23 are applied as stated, and D16 in a
corrected form. Where the check offered two fixes, the first was applied,
with these exceptions:
- D10 named F12 or F49 for the moved frames. F49 was chosen, because it
  is blocked by F4 and has start as an input.
- D13: both fixes were applied.
- D18: the choice is left to open decision 14.
- D21: the size was changed to about 121, and the difference is explained.

One premise was not applied:

- **D16 (F1 acceptance 3).** The check said the reset's clear of
  0xff8000–0xffdfff (0x3b012–0x3b01a) ends before any frame end, so only
  object, video and palette RAM can show MAME's power-on contents. That
  premise was not applied.
  - The clear is 6,144 passes of `clr.l (A0)+` and `dbra` [code], about
    184,000 cycles [inf: 20 + 10 cycles per pass].
  - A frame is about 133,000 cycles: 8 MHz at 60 Hz [code
    sf-mame0289.cpp:756, 770].
  - So the clear is still running at frame 0's end [inf]. This file's own
    model already says so: vblank 0 is held across the clear (F1
    acceptance 4).
  - No pinned capture holds a frame-0 dump to settle it [cap: none found
    under tools/reference/local/captures].

  The check's conclusion stands: frame 0 cannot show all of work RAM's
  power-on contents. F1 acceptance 3 now reads object, video and palette
  RAM as the check proposed. It also reads the part of work RAM the clear
  has not reached, placed by costs and checked against the dump. F73
  holds the same frames to F1's rule (R5 below).

The third check listed 7 defects, R1–R7. Each was checked against the
listing, the code and the docs, and none was wrong in substance. All 7 are
applied. Some are applied in a wider or a chosen form:
- **R1 (F73 acceptance 2).** Applied as stated. The listing confirms the
  spawn at 0x483e8, the A7 load at 0x483f8 and the reset SSP 0xff81a2,
  all inside the C-NAT exception.
- **R2 (F6 acceptance 2).** Applied as stated. The window is now one
  window in two parts, and F27 and F43 each name their part. Events 0xb
  and 6 run at 0x3ef5e–0x3ef6c, after the fight and before 0x3ef96.
- **R3 (F29, F58, F64; section 6; decision 10).** Applied. 0x414f6 tests
  mode 2, not mode 4, so the text says "the game mode" and labels the
  branch for any other mode. No other direct reference to 0xffc002 was
  found in the bonus ranges or the ending [code]. Reads through other
  registers were not excluded.
- **R4 (F22, F38).** Applied as stated. It was also applied to F23's
  "RAM matches after each end", which had the same problem.
- **R5 (F73 against F1).** The first fix was applied: F73's allowance now
  covers only 0x4834e–0x483d4, and the reset's clear follows F1's rule.
- **R6 (section 6).** Both exclusions are now open decisions 15 and 16,
  proposed as exclusions.
  - Game Continuation and Buy-In: the in-game effect was verified through
    the direct reads [code 0x3c156–0x3c18f]. Reads through computed
    pointers were not excluded. Whether MAME can set the values was not
    verified, because ioport.cpp is not held locally.
  - Ken on the P1 keys: the old reason was not verified, and CONTEXT.md
    does not support it. The bullet now says so.
- **R7 (F26, F50).** Applied as stated. The ranking table is 40 long
  words from 0xff8e6e [code 0x3c1c2–0x3c1d2], so its last byte is
  0xff8f0d. run.rs writes regions half-open, which explains the old
  "0xff8f0e".

## 10. Totals

Sums of the sizes in section 4 [inf: sums of [code] counts].

| Measure | Value |
| --- | --- |
| Tickets | 78: 75 ready-for-agent (F18 included, decision 4) and 3 ready-for-human (F30, F31, F54) |
| Waves | 17 (0–16); critical path 17 tickets |
| Newly translated before test mode | about 2,556 |
| Possible small branches | up to about 30 each, only if reached, in any size-0 ticket except F65, F66 and F71 (section 2) |
| Near-copy parameters | about 420 (F58 283, F61 140) |
| Hit-function table entries | about 60 (F27 4, F28 8, F44 4, F53 9, F57 8, F59 8, F60 7, F62 6, F63 6) |
| Not yet sized | the demo's fight-engine branches (F48's probe sizes F55); stream routines found by F25's and F42's probes |
| Test mode | about 945 in F73–F76; the range 0x4834e–0x492d5 alone is about 870 ±30 |
| First draft, for comparison | about 2,600 before test mode and about 940 in test mode, in 69 tickets |

The change from the first draft's 2,600 comes from F1 (−17: 0x3b654 to
F12, exit to F73, the 0x3b602 repeat reported), F12 (−4: 0x3b654's 5 added,
less the 9 helper instructions already inline in t7), F5 (−15: the
helper's inline path) and F27 (−4: 0x3ef96 is 8, not about 12).

This proposal sizes the work. It does not certify the complete port, and
nothing in it is published until the user approves.
