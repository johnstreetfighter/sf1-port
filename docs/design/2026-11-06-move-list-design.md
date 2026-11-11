# The move list, and Training's show-and-check coach

## 1. Intent

A player can open a fighter's move list in any fight to remember an input, and in Training can have the game show a move and then coach them through doing it. The coach says when the move really came out and, when it didn't, what went wrong. It is for the moment a player forgets an input mid-match, and for the longer session of learning a new fighter's kit in Training (the maintainer, 2026-11-06).

The coach must never contradict the game: when it says "too slow", the game would not have taken that input. So it reads the engine's own command state instead of keeping a second copy of the rules.

## 2. Goals, non-goals, anti-goals

### Goals
- A `MOVES` row in the pause box (Local, Arcade, Training) opens the fighter's move list (the maintainer: both moments).
- The list shows the moves this fighter has in this fight: specials, supers, era system moves (parry, Alpha Counter, Custom Combo and the like) and throws, each with its input; normals as one compact line per button (the maintainer: "your suggestion").
- In Training, picking a move offers **show** and **try** (the maintainer):
  - **Show:** the player's own fighter performs it, the input drawn as it goes, then control returns.
  - **Try:** the player does it. A call-out says when the move came out. On a miss the last inputs stay on screen with a short fix such as `TOO SLOW` or `MISSED ↘` (the maintainer: both the inputs and the fix).
- Inputs written as drawn arrows by default; numpad numbers and motion words as Settings choices (the maintainer).

### Non-goals
- Combo trials, challenge lists, scoring or progress tracking.
- Showing or checking moves outside Training (the fight's list is read-only).
- Teaching the CPU's moves the player can't use, or moves that don't run in real fights yet.
- New moves. The list documents what exists; it adds nothing to fighting.

### Anti-goals (failure even if it works)
- The coach saying an input was wrong when the game accepted it, or right when it didn't.
- The list showing a move that won't come out in this fight, or missing one that will.
- Any write to the machine's RAM for the list, show's playback aside (ADR-0004); show drives the fighter only through the same inputs a player gives.
- A look that breaks the 1987 style the front end and era HUD share.

## 3. What exists today (found while designing)

- **Real fights run on the translated 1987 engine.** Today a fighter's usable moves there are the 1987 ones: Ryu and Ken have Hadoken, Shoryuken and Tatsumaki (`docs/reference/specials.md`, `hadoken.md`), plus normals. A boss picked under the full cast plays his 1987 move table. The bosses' new specials (`sf1plus.rs` names such as Kiai Shot), throws, and the era styles' supers and system moves are not in real fights yet: the README says real fights don't carry era state "until the era styles move onto the 1987 fighters", and the art pipeline (AP14 onward) is still drawing specials and throws. `sf1plus.rs` is a separate move model, not what real fights run.
  - **So the first build lists little:** Ryu and Ken's three specials, every fighter's normals. The list must grow by itself as moves reach real fights (see 5).
- **The 1987 command trackers** (`docs/reference/posture-dispatch.md`): the input handler (`0x402f8`) runs three trackers per human player structure (P1 `0xff86b0`, P2 `0xff86da`), each three bytes (stage, frame count, done) at `+0x16` Tatsumaki (D, DB, B), `+0x19` Shoryuken (F, D, DF), `+0x1c` Hadoken (D, DF, F). Directions are facing-relative. A command completes when its third direction arrives within `0x12` frames of the first and stays done while it is held; the button branch then turns the attack into the special and clears the tracker.
- **The fighter's current move** is readable per player structure (Hadoken is move `0x1a`, Shoryuken `0x19`, Tatsumaki `0x1b`).
- **The pause box** (`crates/sf-engine/src/front/pause.rs`): RESUME, OVERLAYS, LOOK, GAUGE ART, RESTART ROUND, QUIT TO MENU; Training adds PRESET and HOLD COMBO.
- **The tuning panel** (`crates/sf-engine/src/front/tune.rs`) is the model for the panel: a gold frame over the left of the screen, drawn sharp over the look, the fight visible beside it.
- **The ROM font** has letters, digits and a little punctuation, and no arrows.
- **Call-outs** (`crates/sf-video/src/callouts.rs`) already place words without overlapping; the coach's messages go through them.

## 4. Approaches considered

1. **A separate input checker for the coach.** Its own copy of each move's rules. Quick, but two copies of the rules drift, and the coach would one day contradict the game. Rejected: it breaks the anti-goal.
2. **Replay the attempt against the engine.** On a miss, snapshot before the attempt and replay the player's inputs with small corrections until one produces the move; the winning correction is the advice. Never contradicts the game, but heavy (many replays per miss) and ambiguous when several corrections work.
3. **Read the engine's own command trackers (chosen; the maintainer: "your rec").** As inputs arrive the trackers already record how far into each motion the player is and how long the step has left. The coach reads that state, read-only, the way the era HUD reads the fight, and a miss gets an exact reason. Cheap, honest and always in step with the engine. The cost is a read-only view onto every recognizer: the 1987 trackers now, and each new recognizer as later moves arrive.

All three share: show feeds the listed input in as a player's input, and a test proves every listed move comes out.

## 5. The design

### The move registry
- Each fighter's list comes from a registry of entries. An entry holds: the move's display name; its kind (special, super, system, throw, normal); its input as data (a sequence of facing-relative directions with button and timing, independent of notation); the recognizer that reads its progress (for the 1987 specials, the tracker at `+0x16`, `+0x19` or `+0x1c`); and the engine move it produces.
- An entry also says when it is available: which fighter, and under which presets and era styles. The list shows only entries available in the current fight.
- **The registry can't lie:** a test feeds every entry's input, through the show path, into a real fight under every preset where the entry is available, and checks its move comes out. It also checks that no move is missing, against an expected-moves list kept by hand in the test, per fighter and preset, taken from the reference docs (*my call, after the builder check: the engine keeps no list of what a human can do, so only an independent list makes this check mean anything*). A feature that brings a move into real fights (an era style, a boss special, a throw) adds its entries and its expected moves, and this test holds it to them.
- **Names:** the 1987 specials keep their documented names (HADOKEN, SHORYUKEN, TATSUMAKI). Other moves take their names from the reference docs as they arrive.

### The panel (the maintainer: layout A)
- The pause box gets a `MOVES` row; confirm opens the panel. It is drawn like the tuning panel: a gold frame over the left of the screen, the fight still visible behind it, the cursor row yellow.
- The header is the fighter's name (`RYU · MOVES`). Below are headed sections in this order: SPECIALS, SUPER, SYSTEM, THROWS, then NORMALS as one line per button. A section with no entries is left out. The list scrolls when it is longer than the panel.
- Each row is the move's name on the left and its input on the right.
- Back closes the panel and returns to the pause box. Outside Training, confirm does nothing on a row.
- **Which fighter:** in Local and Arcade, the side whose pad opened the pause, or P1 when that isn't known (a key, or a lost pad opening it). The tab pair switches to the other side's list. In Training, the player's fighter. *My call, after the builder check: the session doesn't record who paused today, and the tab pair covers the unknown case.*

### Writing the inputs (the maintainer: A by default, B and C as options)
- **Arrows (default):** new 8×8 arrow tiles (eight directions) drawn in the ROM font's style and palette, and the buttons as their letters in small boxes (`P`, `K`, `LP`, `HP` and so on). The arrows are drawn for the fighter's current facing, so they match the stick as held.
- **Numpad:** `236 P`, `623 P`, in the ROM font's digits, always written facing right, as the community writes them.
- **Words:** `QCF+P`, `DP+P`, `QCB+K` and their kind.
- A Settings row `MOVE INPUTS: ARROWS / NUMPAD / WORDS`, saved with the other settings.

### Show (Training)
- On a move's row, confirm offers `SHOW` and `TRY`.
- **Show:** the panel closes and the player's fighter performs the move. The entry's input is fed to that side as player input, through the same path a pad takes, so it is the real move from the real recognizer. A strip under the health block draws the input, each step lighting as it is fed. Control returns once the move ends. The dummy does what Training already has it doing.
- **Try** starts at once after a show, or straight from the menu.

### Try and the coach (Training)
- **The strip shows the input to do.** Each step lights as the move's recognizer reports it reached: for the 1987 specials, that tracker's stage byte.
- **Success:** the move's engine move appears on the player's side, and a call-out names it (`HADOKEN!`). The strip resets for another go. Try runs until the player opens the pause box or picks another move.
- **An attempt** is the tried move's own recognizer passing its first step: for a 1987 special, its tracker reaching stage 2. Other trackers, and a first step alone, are not attempts, so ordinary crouching and walking never count as misses. *My call, after the builder check: anything looser floods the screen during normal movement.*
- **A miss** is an attempt that ends without the move. On a miss:
  - the player's last inputs stay on the strip, under the input to do, for a few seconds;
  - a short fix appears through the call-outs.
- **The fix: the engine decides, the inputs word it (the maintainer: "the split").** A 1987 tracker only advances on the next right direction and resets when its `0x12`-frame window runs out (or when the last direction is let go once done). A wrong direction leaves no mark, so the trackers alone can't tell a missed direction from a slow one, and hold nothing about the button. So:
  - **the engine decides that it failed and where:** the tracker says the attempt ended and at which stage; the ROM's pattern table (`0x4038e`, `0x403a2`, `0x403ba`) says which direction it was waiting for; the busy gate says whether the fighter could act; the current move says whether the move came out;
  - **the player's raw inputs only choose the words:**
    - the awaited direction arrived, but after the window: `TOO SLOW`;
    - it never arrived: `MISSED ↘`, in the chosen notation;
    - the motion was done and then released with no button: `PRESS P SOONER`;
    - the button came before the motion was done: `BUTTON TOO EARLY`;
    - motion and button were right but the busy gate held the fighter: `WAIT TO RECOVER`.
  - **A test holds the words to the engine.** It replays recorded attempts of each kind and checks each fix's claim against what the engine did (for `TOO SLOW`, that the same inputs inside the window bring the move out; for `MISSED ↘`, that adding the direction does).
- **The coach only reads.** Recognizer state, the busy gate and the current move are read from the machine, never written (ADR-0004).

### Settled in grilling (the maintainer, 2026-11-06)
- MOVES is in the pause box under every preset, Faithful 1987 included. The panel is front end only; the fight is unchanged.
- Show waits until the busy gate frees the fighter, standing, with the strip saying `WAIT`, then performs the move.
- Show and Try leave the dummy as Training has it.
- Against the CPU (Arcade, Training vs CPU) the tab pair does nothing: only the player's own list shows.
- NORMALS is one compact line (`LP MP HP LK MK HK · STAND CROUCH JUMP`). Command normals get named rows once they exist.
- An attempt that ends because the player's fighter was hit, or blocked, is neither a success nor a miss, and the coach shows nothing.
- Terms are in `CONTEXT.md` under "Move list and coach".

## 6. Constraints
- ADR-0004: no RAM writes for the list or the coach; show drives the fighter only through player input.
- Faithful 1987 fights are unchanged: the panel is front end, and the list there shows the 1987 moves. One exception, the coordinator's call (2026-11-06): what a Show or a Try draws is drawn under Faithful 1987 too, while one the player asked for in Training runs: the strip (the input to do), a Try's call-out for a move that came out, and after a miss the strip's second line (the player's last inputs) and the coach's fix words, in the same call-out column the overhaul draws them in. Show and Try are opt-in and Training-only; the machine and its RAM are untouched, and nothing else of the era HUD is drawn.
- The arrow tiles are new art in the ROM font's style. They are drawn, not taken from another game (ADR-0006's spirit).
- The coach's messages and the strip share the call-outs' no-overlap rules.

## 7. Left to the builder
- The registry's form and where it lives, and how an entry names its recognizer.
- The strip's exact place, size and timing, inside the call-outs' rules.
- How long a miss's inputs stay up, and the fixes' exact wording beyond the examples.
- How the panel scrolls, and the `SHOW` / `TRY` choice's look.
- How the arrow tiles are drawn, within the ROM font's style and palette.

## 8. As built

### ML1 (#227): the MOVES row, the panel and the registry
- **Registry.** `crates/sf-engine/src/moves.rs`: `REGISTRY` of `MoveEntry` (name, `MoveKind`, `MoveInput` as facing-relative `Dir`s plus a button class, `Recognizer`, the engine move it produces, `Availability` by fighter and preset). `Recognizer::Tracker(Tracker1987)` carries the tracker's offset and ROM pattern address and has a read-only `read` (stage, frame count, done) that the coach (ML5) will use. `move_list(fighter, preset)` is the registry filtered by the fight. `MoveInput::frames` builds the player inputs that perform an input (what a later Show feeds). The first entries are Ryu's and Ken's Hadoken, Shoryuken and Tatsumaki, available under all five presets (the toggles loosen the trackers, they never take a move away).
- **Panel.** `crates/sf-engine/src/front/moves.rs`: a `Panel` layer 40 x 28 cells at the left of the screen (`Panel` gained a `side`; the tuning panel is `Right`), drawn by the host sharp over the look with the picture moved to the right edge (`crates/sf-host/src/panel.rs`, which now takes any panel size and either side). The session's frame while it is up is the paused fight as it was, undimmed, so the fight beside the panel is untouched. The session exposes it read-only as `GameSession::move_list`.
- **Builder's calls** (all within section 7's "left to the builder", or where the design left a detail open):
  - The MOVES row is the **last** row of the pause box in every mode, so no existing row moved (Training's box now runs to the screen's last text row).
  - The normals' separator is a middle dot, made in the ROM font's style (`MADE_GLYPHS` in `sf-video`), because the ROM has none; the line is exactly `LP MP HP LK MK HK · STAND CROUCH JUMP` and needs 37 cells, so the panel is 40 cells wide (the tuning panel's 32 would not hold it).
  - The header is the fighter's name alone (`RYU`), as the ticket has it; section 5's `RYU · MOVES` is not drawn. Headings are bracketed (`[ SPECIALS ]`) like the tuning panel's; the normals have a `[ NORMALS ]` heading too.
  - Up and down move the cursor over moves only (wrapping); a list with no move (a boss) scrolls instead. Start in the list resumes the fight straight away, as it does from the pause box; back returns to the pause box. A pad lost while the list is up closes it back to the pause box, so the disconnect notice shows. Confirm on a row does nothing until Show and Try (ML4).
  - The fighter is P1's (`config.roster.p1`) in every mode; whose list opens is ML3. Faithful 1987 binds Ryu to P1 and Ken to P2, so Ken's Faithful list is only reachable once ML3 opens P2's list; the hand-kept test checks it through the panel's constructor.
  - Arcade's select takes Ryu or Ken only for now, so a boss's list is tested in Local and Training.
- **Tests.** `crates/sf-engine/tests/move_list.rs` (session, frames, registry, hand-kept list, snapshot), unit tests in `moves.rs` and `front/moves.rs`, and the panel placement tests in `sf-host`.

### ML3 (#229): whose move list opens
- **Who paused.** The session records the side whose pad's Start opened the pause box (`paused_by`); a key (Start, Esc, Backspace, a cabinet start) or a lost pad leaves it at P1. If both pads press Start on the same frame, P1 wins.
- **Which list.** Local opens the list of the side that paused; the tab pair (Q / E, L1 / R1) in the list switches to the other side's list, both ways, and a switch replaces the panel (cursor back at the first move). Every other fight has one player: Arcade, Training against a CPU and Training against the dummy always show P1's fighter, and the tab pair does nothing there.
- **Builder's calls.**
  - A P2 pad's Start against the CPU, or against the dummy, still shows P1's list: P2 is not the player there. The ticket's "Training: the player's fighter" has the dummy case the same way.
  - The tab pair on the pause box itself (no list up) does nothing.
  - The cabinet start buttons (`start2`) count as keys, so they open P1's list; only a pad names a side.
  - The header already names the fighter (ML1), so a switch is visible without a new mark.
- **Tests.** `crates/sf-engine/tests/move_list.rs`: P2's pad shows P2's fighter (header checked against a list opened for them); P1's pad, the keyboard's Start, Esc and a lost pad show P1's; the tab pair switches in Local (both ways, both tab keys) and changes nothing in Arcade or Training against the CPU, even when P2's pad paused.

### ML2 (#228): drawn arrows and the MOVE INPUTS setting
- **Notation.** `InputNotation` (`crates/sf-engine/src/moves.rs`: `Arrows` the default, `Numpad`, `Words`) with `MoveInput::write(notation, facing_right)`, which gives pieces: text in the ROM font, arrows included, and a button chip. `input_text` writes a button as `[P]` for tests (`HADOKEN ↓↘→ [P]`). `Dir::arrow(facing_right)` picks the arrow, `Dir::numpad` the digit; numpad is `236 P`, always facing right, and words are ML1's `QCF+P`. The panel (`MovesPanel::with_inputs(notation, facing_right)`) draws the pieces right-aligned as ML1's words were.
- **Tiles.** The eight arrows are `MADE_GLYPHS` in `crates/sf-video/src/compositor.rs`, so `RomFont::text` draws them like the `+` and the middle dot; `RomFont::button` draws a chip, a cell a letter. Both serviceable choices are on `docs/planning/art-upgrades.md` ("Move list inputs").
- **The setting.** `Settings::move_inputs`, saved as `move_inputs = "arrows"` (a file from before it loads ARROWS; a stray value is refused). The row is `MOVE INPUTS` on the Controls tab, after `P2 SCHEME` and before the rebind rows; `RESET TAB` on Controls puts it back to ARROWS. It is a player's own choice, never in the match config.
- **Builder's calls.**
  - The row is on the **Controls** tab: neither the design nor section 8 placed it, and it is how the player's own inputs are shown.
  - The panel reads the setting and the fighter's facing **when it opens**, and keeps them: the fight is frozen behind it and the setting cannot be changed from the pause box. The facing is read from the machine (`facing_right`: the player structure's `+0x10` names the entity, whose `+0x37` is 4 and over facing left), P1's structure for now (whose list opens is ML3's).
  - The button is a **filled chip with the letter knocked out**, not an outlined box (the ROM's glyphs fill their cell, so an outline has no room for a letter). Only the letters P, K, L, M, H have a chip face; any other label is set as plain text.
  - Numpad writes `236 P` with a space, not `236P`, as the ticket has it.
  - `MovesPanel::open` keeps its signature (arrows facing right until `with_inputs`), so ML3's and ML4's callers are untouched.
- **Tests.** `crates/sf-engine/tests/move_inputs.rs`: the setting changed on the Settings screen through the session reaches the rows in each notation (text and pixels), a saved choice opens the list in it, the panel drawn with the arrows facing each way, numpad and words not turning, and a Local fighter who has crossed over reads as facing left. The settings round trip, the cycle and `RESET TAB` are in `front_end_settings.rs`; unit tests in `moves.rs` and `front/moves.rs`.

### ML4 (#230): Show in Training
- **The choice.** In Training, confirm on a move's row opens `SHOW` and `TRY` in the panel's foot (left and right pick, confirm chooses, back drops the choice, then the list). **TRY is drawn dim and inert** until ML5: the cursor cannot land on it. Outside Training confirm on a row still does nothing. `MovesPanel::with_show(training)` turns it on, so `open`'s signature is unchanged.
- **The path.** `crates/sf-engine/src/front/show.rs`: a `Show` stands in for the pad. `Fight::step` replaces P1's input with the show's for that frame (the dummy's P2 handling is untouched), so the move comes out of the real recognizer; nothing is written to the machine. The registry's `MoveInput::frames` (medium strength) is what is fed, the mirror when the fighter faces left (read from the entity's `+0x37`, per frame). `Fight::show` is Training only.
- **Waiting.** The show feeds nothing until the fighter is free by the busy gate (`+9 & 0xc9` is 0, 1, 8 or 9, or move 0 or 0xc) **and** in its stance (move 0 or 1), read from the fight reader's last frame. The strip says `WAIT`. SHOW from neutral feeds on the first stepped frame.
- **The strip.** `sf_video::callouts::regions::STRIP`: 24 letters wide, centred at y 62, in the gap between the health block (with TIME) and the call-out columns, and a `Region::Strip` in `regions::all()` so the no-overlap checks cover it. It is drawn by `Fight::render` over every preset, Faithful 1987 included (call-outs are not drawn there; the strip is the one exception, the coordinator's call, see section 6). Steps are the motion's initials then the button letter (`D DF F P`), lit gold as each is fed and grey before. **The strip does not use ML2's notation yet**: arrows are made glyphs drawn by ink, the strip letters by pen, and the setting would have to reach the fight; so it keeps the initials (`MoveInput::strip_steps`), a later swap through `MoveInput::write`.
- **The end.** After the input, neutral is fed until the move the entry produces has come out and ended; the show then drops and the pad has control again.
- **My calls** (not in the design):
  - **The pad is ignored once the input is being fed**, so the show never gives the pad half a fighter. **While the show is still waiting the pad is the player's**: it is not fed, so a held crouch or walk still stops at the stance, but a direction or button newly pressed while waiting (a held one, such as the confirming key, is the baseline and does not count) cancels the show and returns the pad, so the player is never locked out of blocking for the wait.
  - **A show gives up** and returns the pad when the flow leaves the fight loop (a time-over or round end, so the strip never rides into the round-result text), when either fighter is knocked out, after 600 frames of waiting (ten seconds), 90 frames after the input if the move never came out (the fighter was hit while it was fed), or 600 frames into a move that came out and never ends. A Restart Round drops it. A hit during a wait just keeps waiting.
  - **The facing** is read per frame from the fighter's entity (`+0x37` bit 2 set is left, as the engine tests it), and only from a fighter slot. **`came_out`** starts over when the input begins, so a move already out before it does not count.
  - **Show uses the medium button**, as the registry's old test did.
  - **A pause keeps the show**: resuming goes on from where it was.
  - **Opponent.** Against the CPU the show is whatever the fight does with a pad's input; the CPU is never touched.
- **Tests.** `crates/sf-engine/tests/show.rs`: the choice, Show from neutral (strip lights one step at a time, move comes out, control returns), from a jump and a recovery (WAIT, then fed from the stance), bit-for-bit equality of a show with a pad giving the same inputs (dummy and CPU), the dummy unchanged with P2's pad held, the strip's place and its lettering in every preset. `tests/move_list.rs`: "every listed move comes out" now goes through Show's path in Training against the dummy under every preset. Unit tests in `show.rs` and `moves.rs`.

### ML5 (#231): Try and the coach's success
- **The view.** `crates/sf-engine/src/moves.rs`: `Recognizer::view(MachineRead, player_struct, window)` gives a `RecognizerView`: the raw tracker bytes (`RecognizerState`), `time_left` (the window less the tracker's frame count), `awaiting` (the next direction, read from the ROM's pattern table at `0x4038e` / `0x403a2` / `0x403ba` by `Dir::from_pattern`, none once the motion is complete), `accepts` (every direction the engine takes for the next step: under lenient trackers the Shoryuken's shortcuts from `tracker_matches_stage`, otherwise the table's one), `gate_free` and `current_move`. The view's gate is `input_gate_free`, which mirrors the input handler exactly (`0x403d4`, `0x404b4`): the state bits or move `0xc`, the `0x92` walk phase (`& 0xc9 == 0x80`) but not exactly `0x80`, the interruptible walk under `interruptible_walk_cycle` (and `standard_high_low_block` guarding), a fighter who is down never; it takes the fight's `CoreToggles`. `busy_gate_free` is only the gate's first test, which Show keeps (with the stance) as its stricter "free and standing". `MachineRead` holds three shared slices (ROM, work RAM, object RAM), so a view cannot write. `PowerOnMachine::program_rom` is the one new read accessor. The window is the ROM's `0x12` frames, or the configured one under lenient trackers (`tracker_window`).
- **The coach.** `crates/sf-engine/src/front/coach.rs`: a `Coach` is handed the view and the fight reader's read of the player's fighter once a stepped frame (`Fight::step`, after the reader), keeps the strip's lit count, the attempt and the outcomes, and writes nothing. `Fight::try_move`, `stop_try` and `coach` (and `GameSession::coach`) are Training's; `Outcome` is `Success`, `Miss(Miss)` or `Interrupted`.
  - **Strip.** The same `Strip::Input` the show draws, lit by the tracker's stage byte as the engine has it (the strip is drawn by `Fight::render`, so a frame's lit steps are that frame's stage). It follows the tracker frame by frame, and a tracker that keeps running (a direction still held after the move) lights again as the engine counts it.
  - **Attempt.** The stage rising to two, from below it. Stage one alone (a crouch, a walk) never starts one. **Success** is the fighter entering the move the entry produces while an attempt is open (it comes out on the same frame the tracker is cleared); the strip resets on that frame. **Interrupted** is, while an attempt is open, the fighter in a hit reaction (`reacting`), a hit or chip landing on P1 (`landed[1]`), a block the reader saw, or the guard posture against an attacking opponent. The guard posture alone is not (the Tatsumaki's motion passes through down-back and back). **Miss** is the tracker falling back below the attempt stage with no move out, held open for the recovery buffer's window plus one frame (`settle`, when `recovery_input_buffer` is on) and settled as a success if the move comes out meanwhile, or as `Other` if another registry move did. A move out is a success when an attempt is open or settling, or when the tracker had counted two steps or more a frame before, even if an interruption closed the attempt.
  - **The seam for ML6.** `Outcome::Miss(Miss)` carries what the engine knew: the furthest stage, the direction the table awaited at it and every direction the engine accepted, the frames counted from the first direction, the frames the command was done with the busy gate closed, the window's last time left, how the tracker fell back (`Ended::WindowRanOut` or `Released`), and the busy gate and current move when it was settled. `Outcome::Other(move)` is a different registry move coming out. ML6 chooses the fix's words from these and the player's raw inputs; the coach shows nothing for a miss yet.
- **Starting and stopping.** TRY in the panel's foot is live (left and right pick, confirm chooses; `MovesPanel::take_try`), and the session resumes the fight with the coach watching. A show that ran its course (`Show::completed`: the move came out and ended) goes on as a Try of the same move; a show given up on does not. The pause box ends the Try, and picking another move (Try or Show) replaces it.
- **The call-out.** `CalloutKind::Move { word }` (`HADOKEN!`, from the entry's `call_out`) goes through the call-out board on the player's side, P1's column, so the no-overlap placement holds. It is left out of `CalloutKind::all`, and the announcer says nothing for it (`AnnouncerLine::for_callout` is now an `Option`).
- **My calls** (not in the design):
  - **The first frame only takes note.** A tracker already at stage two or more when the Try starts (or when a hit ended an attempt) has to fall back before it can start another, so progress made before the Try is no attempt of it.
  - **The Try is dormant outside the fight loop** (between rounds, a time-over): the strip is not drawn and nothing is observed, and it picks up again when the loop does, so a round change does not drop the move. The pause box ends it.
  - **The P step never lights from the tracker.** The 1987 trackers count directions only, and the move comes out in the frame the button is taken, so the button's step stays grey and the strip resets on the move. (The engine's latch at entity `+0x34` could light it for a frame; not done, it would be a second reading of the same moment.)
  - **Faithful 1987 draws the Try's strip and its call-out**, the coordinator's call for Show's strip carried over: opt-in, Training-only, nothing written to the machine. The overhaul layout draws call-outs; under Faithful the fight draws the Try's own `Move` words itself (and no other call-out), in the same slot.
  - **A Local P2's list never starts a Try** (Try is Training-only, and Training is P1's list, as Show).
  - **Other special out.** A different special coming out during an attempt of this move ends it as a miss with that move as `current_move`; ML6 may choose to say nothing then.
- **Tests.** `crates/sf-engine/tests/try_coach.rs`: the choice, a recorded input for each registry move giving its call-out (pixels too) and the strip following the tracker's stage byte read straight from the RAM every frame, every preset including Faithful 1987, Show into Try, pause and another move ending a Try, walking and crouching starting no attempt, a motion dropped half way a miss with the stage and awaited direction from the ROM, an interruption by a fed hit reaction or block and by a real CPU hit counting as nothing, and bit-for-bit equal machines with and without a Try watching. Unit tests in `coach.rs`, `moves.rs` (the view over a miniature machine) and `front/moves.rs`; the call-out's placement in `sf-video/tests/callouts.rs`.
- **Review fixes (Opus review of ML5).** Guard posture is no longer a hit (ten Tatsumakis came out uncounted); a move out no longer needs an open attempt; the view's gate mirrors the input handler (a walking fighter was read as held); misses settle after the recovery buffer's window; the Miss carries the fields above; `accepts` reports the lenient trackers' directions. Tests: a Tatsumaki against the CPU called out every time (one came out after an interrupted attempt), the view's gate checked against what the engine does with a button on each walk frame in both presets, the recovery buffer bringing the move out after the tracker is gone (PureFootsies, a heavy punch's recovery), walking and crouching into motions that cannot finish, Other, a real CPU hit that landed, the strip pixel-checked for every move under Faithful 1987, and a Faithful frame during a Try equal to the plain frame outside the strip and the call-out column.

### ML6 (#232): the coach's fixes
- **Where.** `crates/sf-engine/src/front/coach.rs`: `Fix` and `FixKind` (the words, `Fix::text(notation)`), `Coach::input` (the fight hands in the player's pad and facing each frame, before `observe`), `fix`, `take_fix` (once, for the call-outs), `deciding` and `shown_inputs`. `crates/sf-engine/src/moves.rs`: `RecognizerView` gained `window` (the tracker's window under the fight's toggles) and `latched` (the punch and kick latch bytes at `P1_STRUCT` and `+1`, read only); `Dir::from_stick` (pad to facing-relative direction, opposites cancel), `Dir::written(notation, facing_right)`, `ButtonClass::down` and `other`. `Fight` puts a fix through the call-outs as `CalloutKind::Fix` (`crates/sf-video/src/callouts.rs`; `FixText` holds the words inline so the kind stays `Copy`) and draws the player's inputs; the session gives it the MOVE INPUTS choice when a Show or Try starts (`Fight::set_move_inputs`).
- **The engine decides, the inputs word it** (ADR-0012). The coach records every stepped frame with the engine's reading of it (the tracker's counted stage after the frame, the gate as the frame before left it, and whether the engine **took** a punch or a kick) and the player's stick, buttons and facing. A button is *taken* when its latch byte goes up (the engine took it while the gate held the fighter) or the fighter enters an attack, not the tried move, on a frame a button of that class went down or came up (taken with the gate open, it starts its attack at once). Which frame the engine takes a button on is the engine's own business: Faithful 1987 takes the attack on the release (`fighter.rs`, the latch path), the overhaul on the press, and on the release only when a special is ready (negative edge). The coach never guesses that from the pad; the pad only says which class. When an attempt ends as a miss (after the recovery buffer's wait), the words are:
  - the motion stalled at step two (`Miss::stage < 3`):
    - the tracker was cleared with time left (`Ended::Released`): a button of either class the engine took before the motion was done (a few frames before its first direction up to the fall) is **BUTTON TOO EARLY**; otherwise **MISSED** and the direction the ROM's table awaited. It is never TOO SLOW.
    - the window ran out (`Ended::WindowRanOut`): a direction the engine took (`Miss::accepted`, so a lenient Shoryuken's DF or F) arriving once the tracker had fallen is **TOO SLOW**, the frame it comes; none arriving in a window's length again (the tracker's own window, `RecognizerView::window`: the ROM's 18 frames, the configured one under lenient trackers) is **MISSED**, the direction in the chosen notation (`MISSED ↘`, `MISSED 3`, `MISSED DF`). Starting another attempt ends the wait with what there is.
  - the motion was done (`Miss::stage == 3`):
    - a button taken before it was done, of either class: **BUTTON TOO EARLY**;
    - the class's button taken or pressed while the gate held the fighter: **WAIT TO RECOVER**;
    - the other class taken while the command was done: **PRESS P NOT K**, the class the move needs first, with no SOONER;
    - the class's button down while the command was done and the gate open, and not taken (the engine takes it on the release, which came after): **RELEASE P SOONER**;
    - the gate shut on every frame the command was done: **WAIT TO RECOVER**; otherwise (no button, or one that came after the motion was dropped) **PRESS P SOONER**.
  - `Outcome::Other` and `Outcome::Interrupted` give no fix.
- **A button on the frame of the second direction.** It clears the tracker before stage two is ever seen. When the tracker was at step one, the stick shows the direction the table awaited there and the engine took a button on that frame, the coach counts an attempt (a `Miss` at stage two, `awaited` none) and words it BUTTON TOO EARLY. Crouching and punching (the stick still on the first direction) is no attempt.
- **The strip's second line.** After a fix is chosen the player's last inputs, one label to a step as the strip's (`D DF F P`, the newest if they do not fit), are drawn under the input to do for `INPUTS_FRAMES` (240, four seconds), in the whitest pen. A new attempt, a move out or the fight loop's end clears them. The fix is a call-out word like `HADOKEN!` (90 frames), under every preset including Faithful 1987 (the Try's own words, as ML5; the design's Faithful exception, section 6, now says so).
- **The layout.** The strip's second line needs the ten pixels under the first, where the call-out columns start. The columns sit at `COLUMN_TOP` 72 in every fight but a Training Try, and ten pixels lower (`regions::TRY_DROP`, 82) only while a Try runs (`CalloutFrame::drop`, set by `Fight` from whether a coach is running), so Arcade, Local and Training without a Try keep the approved era-HUD layout and its goldens. `regions::all()` is the ordinary layout (no second strip line); `regions::all_try()` and `all_with(lift, drop)` are the Try's.
- **Call-out lettering reaches made glyphs.** The call-outs' `letter` used to set the ROM's tiles only; it now sets glyphs made in the ROM font's style too (`RomFont::glyph_in`), so `MISSED ↘` can draw its arrow. That also draws the hyphen in a super's name such as `Ansatsu Geki-Retsuzan`, which used to leave a gap there. Not a layout change (a hyphen takes its own cell as the gap did); `crates/sf-video/tests/callouts.rs` checks the hyphen's cell. A made glyph is drawn in the letters' body pen over the shadow pen.
- **My calls** (not in the design):
  - **The strip keeps its initials**, and the player's line is in the same labels so the two lines compare; only the fix is in the chosen notation, as the ticket asks. Swapping the strip to ML2's notation (ML4's note) would swap both lines together.
  - **TOO SLOW and MISSED wait a window** after the stalled motion falls back; TOO SLOW needs that look, so `MISSED` is that much later than the tracker's fall (a third of a second in 1987) rather than guessed at.
  - **Two words beyond the ticket's five:** RELEASE P SOONER (Faithful 1987 holds a button down through the motion and takes it on the release, so a release that comes after the window is the late input, not an early button) and PRESS P NOT K (the engine took the other class's button). Both are claims the engine confirms: the release moved into the done window, or the right class's button, brings the move out.
  - **A press the gate shut out** (a walk's held frames in 1987) is latched by the engine and used once the motion is done; the coach reads the latch, so such a press is a move or WAIT TO RECOVER, never early.
  - A direction both held together on a pad (left and right) is neutral. The arrow in the fix points the way the fighter faced on the frame the motion stalled.
  - The same-frame attempt above has no awaited direction (the third step was never seen), so its `Miss` carries none; its fix needs none.
- **Tests.** `crates/sf-engine/tests/coach_fixes.rs`:
  - each kind of miss recorded for every registry move under Faithful 1987 and the Definitive Overhaul preset, run through the session; each claim read from the engine's own facts (tracker stage, `input_gate_free`, the latch bytes, the fighter's move, the stick) and then put to it by an input **derived from the recorded one** and the engine's facts (TOO SLOW: the late direction brought inside the window; MISSED: the awaited direction inserted at the stall; PRESS P SOONER: the button added in the done window; RELEASE P SOONER: the button let go inside it; PRESS P NOT K: the right class swapped in; BUTTON TOO EARLY: the early button dropped and pressed after the motion; WAIT TO RECOVER: the same inputs after waiting);
  - the review's probes as fixtures (Faithful: P held 14 frames and taken on release, the stick let go before P, P pressed on DF and released after the stick; overhaul: P held from before and let go on DF, a kick tapped then F within the window, a kick in time, a button on the frame of DF), and a press-and-hold let go in time (a move in 1987, early in the overhaul);
  - WAIT TO RECOVER under every preset; MISSED in each notation drawn in the column; the inputs line drawn, held and cleared; the wait following each preset's window; Other and a hit giving no fix; the machine bit for bit with and without the Try; the fix, the strip's two lines and the era gauges never overlapping (regions, and pixels against the same fight with no Try); the columns at 72 outside a Try and 82 during one.
  - Unit tests in `coach.rs` (synthetic frames for every word and the same-frame attempt), `moves.rs`, `sf-video` (`tests/callouts.rs`: fixes in the column at both layouts, the made glyph lettered, the hyphen, the inputs line's place, the columns' two layouts) and `training_overlay_clearance.rs` (a Try's layout against the overlay's boxes and the gauges).
