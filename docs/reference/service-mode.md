# Service Mode from Power-On (`0x3b01e..0x3b02a`, `0x3b664`, `0x3b93c`, `0x4834e..0x492d5`)

Evidence class: **SM** (tickets #145 `F73`, #146 `F74`, #147 `F75`, and #148 `F76`).

## Overview

When `DSW1` bit 15 (`0x8000`, active-low `Service Mode`) is low (`0`, `Service Mode = On`) at power-on, the reset handler `0x3b000..0x3b02a` clears work RAM `0xff8000..0xffe000` (`6,144` longwords), reads `0xc00008` (`($8,A6)` = `DSW1`) at `0x3b01e`, and branches via `0x3b02a` (`jmp $4834e.l`) into the service-mode RAM test `0x4834e..0x483f2` instead of normal boot (`0x3b030`).

After the work+object-RAM (`0xff8000..0xffffff`) and video-RAM (`0x800000..0x800fff`) pattern tests complete with interrupts masked (`SR = 0x2700`), `0x483d8..0x483f2` sets `0xff8003 = 0x55` (`MODE_DEMO`), spawns slot `0` at `0x483f8` (`trap #0`), lowers `SR` to `0x2000`, and enters the cooperative scheduler loop `0x3b19a`. Slot `0` (`0x483f8..0x48642`) uploads the four boot palettes (`BOOT_PALETTES`: `0x1ae68`, `0x1a268`, `0x1ba68`, `0x1bc68`), draws the `SWITCH TEST`, `SOUND TEST`, `MEMORY TEST`, and `PUSH SW.` screen, and runs the per-frame input/DIP/sound test loop `0x484d4..0x48642` until P1 Up + P1 Kick release exits at `0x485f2..0x48618` (`trap #3`, `0x3b29a`), clearing slot `0`'s state byte (`+0`) and rescanning from slot `0` (`0x3b19a`) into the newly spawned slot `1` at the service menu `0x48644` (`F74`, `#146`).

---

## 1. Summed 68000 Instruction Costs (`0x3b000..0x3b02a` and `0x4834e..0x483d4`)

Both the reset RAM clear (`0x3b000..0x3b01c`) and the service RAM test (`0x4834e..0x483d4`) run with `SR = 0x2700` (all maskable interrupts disabled, including level-6 vblank). At the 8 MHz 68000 clock (`8,000,000 / 60 = 133,333.33` cycles per frame), their exact cycle count from power-on (`t = 0`) is determined by standard Motorola 68000 instruction timings (`crates/sf-engine/src/tasks.rs:1587–1617`):

| Address range | Routine / pass | Iterations | Per-iteration / fixed 68000 cycles | Subtotal (cycles) | Cumulative cycles |
| --- | --- | --- | --- | --- | --- |
| `0x00000..0x00007` | Reset vector fetch (`SSP`, `PC -> 0x3b000`) | 1 | `24 + 2` | `26` | `26` |
| `0x3b000..0x3b01c` | Register setup (`48`) + `6,144` x `clr.l (A0)+` (`122,880`) + `dbra` (`61,444`) over `0xff8000..0xffe000` | `6,144` longs | `48 + 122,880 + 61,444` | `184,372` | `184,398` |
| `0x3b01e..0x3b02a` | `move.w ($8,A6),D0; andi.w #$8000,D0; bne $3b030; jmp $4834e.l` | 1 (taken to `0x4834e`) | `12 + 8 + 12 + 12` | `44` | `184,442` |
| `0x4834e..0x48356` | `move.b #$ff,D0; jsr $3b690.l` (`gfx_off(0xff)`) | 1 | `8 + 80` | `88` | `184,530` |
| `0x48358..0x483a8` | Work+object RAM (`0xff8000..0xffffff`) `0x5555` / `0xaaaa` write + verify loop | `16,384` words | `1,507,502` | `1,507,502` | `1,692,032` |
| `0x483aa..0x483b8 -> 0x4836a..0x483a8` | Video RAM (`0x800000..0x800fff`) `0x5555` / `0xaaaa` write + verify loop | `2,048` words | `188,590` | `188,590` | `1,880,622` |
| `0x483ba..0x483c4` | `8,192` x `clr.l (A0)+` + `dbra` over `0xff8000..0xffffff` | `8,192` longs | `245,776` | `245,776` | `2,126,398` |
| `0x483c6..0x483d4` | `2,048` x `move.w #$103c,(A0)+` + `dbra` over `0x800000..0x800fff` | `2,048` words | `45,080` | `45,080` | `2,171,478` |

- `SERVICE_RAM_TEST_END_CYCLES = 2,171,478` cycles (`floor(2,171,478 / (400,000 / 3)) = 16`).
- Frame `0` ends at `133,333` cycles (inside the reset RAM clear `0x3b012..0x3b01a` after `4,441` longwords, `RESET_CLEAR_FRAME0_END = 0xffc564`, matching F1 `#41`).
- Frames `1..=15` (`SERVICE_RAM_TEST_FRAMES = 15`) end at `266,666 .. 2,133,333` cycles inside `0x4834e..0x483d4`.
- Frame `16` (`SERVICE_RAM_TEST_DONE_FRAME = 16`) is the first frame whose end (`2,266,666` cycles) falls after `0x483d8` writes `0xff8003 = 0x55` (`MODE_DEMO`), spawns slot `0` at `0x483f8`, lowers `SR` to `0x2000`, and takes the held level-6 vblank interrupt (`SERVICE_RAM_TEST_TO_VBLANK_CYCLES = (2,171,478 + 575) - (16 * 400,000 / 3) = 38,720` cycles from the start of frame `16` to `0x3b052`).

---

## 2. Per-Frame RAM State During the Power-On Reset Clear and RAM Test (Frames `0..=15`)

On frames `0..=15`, `screen_hash`, `latch`, `latch_changed`, checkpoint pixels (`pixels_*.rgba`), and palette RAM (`ram_*_palette.bin`) match between reference and native on every frame (`0x0000` black screen, latch `0`). Work RAM (`0xff8000..=0xffffff`) and video RAM (`0x800000..=0x800fff`) differ from the native zeroed/cleared state only on the 15 in-flight RAM-test frames `1..=15`:

| Frame | Cumulative 68000 cycles at frame end | In-flight routine | Work RAM (`0xff8000..=0xffffff`) differing byte range | Video RAM (`0x800000..=0x800fff`) differing byte range |
| --- | --- | --- | --- | --- |
| `0` | `133,333` | `0x3b012..0x3b01a` (reset clear at `0xffc564`) | None (`0` bytes; uncleared tail is already `0x00` at power-on) | None (`0` bytes) |
| `1` | `266,666` | `0x48358..0x483a8` (work RAM `0x5555` write, `4,560` words) | `0xff8000..=0xffa39f` (`9,120` bytes) | None |
| `2` | `400,000` | `0x48358..0x483a8` (work RAM `0x5555` write, `11,968` words) | `0xff8000..=0xffdd7f` (`23,936` bytes) | None |
| `3..=11` | `533,333 .. 1,600,000` | `0x48358..0x483a8` (work RAM `0x5555`/`0xaaaa` fill and verify) | `0xff8000..=0xffffff` (`32,768` bytes) | None |
| `12..=13` | `1,733,333 .. 1,866,666` | `0x483aa..0x483b8 -> 0x4836a..0x483a8` (work RAM `0xaaaa` + video RAM `0x5555`/`0xaaaa`) | `0xff8000..=0xffffff` (`32,768` bytes) | `0x800000..=0x800fff` (`4,096` bytes) |
| `14` | `2,000,000` | `0x483ba..0x483c4` (work RAM clear at `0xffbe2c`; video RAM `0xaaaa`) | `0xffbe2c..=0xffffff` (`16,852` bytes) | `0x800000..=0x800fff` (`4,096` bytes) |
| `15` | `2,133,333` | `0x483c6..0x483d4` (work RAM fully `0x0000`; video RAM `0x103c` fill in progress) | None (`0` bytes) | `0x800000..=0x800fff` (`4,096` bytes) |

From frame `16` (`SERVICE_RAM_TEST_DONE_FRAME`) onward, `ram_test_finish` (`0x483ba..0x483f2`) has cleared `0xff8000..0xffffff`, filled video RAM `0x800000..0x800fff` with `0x103c`, set `0xff8003 = 0x55`, spawned slot `0` at `0x483f8`, and run frame `16`'s vblank and scheduler; every frame from `16` onward matches on `screen_hash`, `latch`, `latch_changed`, checkpoint pixels, video RAM, palette RAM, and all CPU-owned work RAM.

---

## 3. Service Input, DIP and Sound Test (`0x483f8..0x48642`)

### 3.1 Screen Initialization and Palette Uploads (`0x483f8..0x484d2`)

1. **`0x3b5c8..0x3b5f6` (`service_screen_init`)**:
   - Calls `boot_init` (`0x3b77a`), `clear_text_layer` (`0x3bdb0`), `gfx_off(0xff)` (`0x3b690`), `coin_ctrl_off(0xffff)` (`0x3b6a2`), and `aux_ctrl_off(0x00ff)` (`0x3b6b4`, clearing the low byte of `0xff8686` and writing `0xc00012`), then clears `*bus.fg_scroll = 0` (`clr.w ($14,A6)`).
2. **`0x48400..0x48440` (4-step palette upload)**:
   - Calls `upload_palette` (`0x3b5f8`, sleeping 1 frame via `trap #2` at `0x3b62e` per chunk) for the four `BOOT_PALETTES` chunks (`0x1ae68 -> 0x0000` at frame `16`, `0x1a268 -> 0x0200` at frame `17`, `0x1ba68 -> 0x0400` at frame `18`, `0x1bc68 -> 0x0600` at frame `19`).
   - On frame `20` (`SERVICE_TEST_SCREEN_FRAME`, matching F2 `#42`), the 4th sleep returns and `0x48442..0x484d2` draws the service test screen.
3. **`0x48442..0x484d2` (text layout and latch enable)**:
   - Draws the 6 static text blocks listed at `0x486c0..0x486d8` (`0x49340`, `0x493aa`, `0x493e4`, `0x4940e`, `0x49a9e`, `0x492d6`).
   - Checks `D7` (low word = video RAM error flag, high word = work RAM error flag): both are `0` on a passing RAM test, leaving `OK` at `0x8006be` and `0x8005be` (if `1`, writes `0x0017_0010` = `NG`).
   - Calls `gfx_on(9)` (`0x3b644`) and `gfx_off(1)` (`0x3b690`), leaving `GFXCTRL = 0x0008` (text layer enabled).
   - Initializes the dynamic 8x17 text-block header at `0xff872c..0xff8731` (`dest = 0x0936`, `dims = 0x0811`, `attr = 0x9000`) and clears the 5-frame sound-selector divider `0xff8848 = 0`.
   - Draws initial `00` (`write_hex_byte`, `0x3c014`, with attribute `0x5000`) at the four video-RAM offsets in `0x49a8e`: `0x05aa` (P1 Punch), `0x06aa` (P1 Kick), `0x05b6` (P2 Punch), `0x06b6` (P2 Kick).
   - Calls `reset_button_latches` (`0x3b6c6`) and sets `0xffc002 = 1` and `0xffc008 = 0x80` so `vblank_buttons` (`0x3b05a..0x3b0c8`) latches P1/P2 punch and kick release strengths into `0xff86b0..0xff86b1` and `0xff86da..0xff86db`.

### 3.2 Per-Frame Input, DIP, Strength and Sound Loop (`0x484d4..0x48642`)

1. **Port word binary matrix (`0x484d4..0x48502`)**:
   - Iterates `idx = 0..8` (`D2 = 7..=0`) over the 8 consecutive words at `0xc00000..0xc0000f` (`Io::port_word(idx)`):
     - `idx = 0` (`D2 = 7`, `0xc00000`): `IN0` (inverted with `not.w D0`)
     - `idx = 1` (`D2 = 6`, `0xc00002`): `IN1` (inverted)
     - `idx = 2` (`D2 = 5`, `0xc00004`): unmapped port (`cmp.w #5,D2; beq.s $484f2` skips `not.w D0`)
     - `idx = 3` (`D2 = 4`, `0xc00006`): unmapped port (`cmp.w #4,D2; beq.s $484f2` skips `not.w D0`)
     - `idx = 4` (`D2 = 3`, `0xc00008`): `DSW1` (inverted)
     - `idx = 5` (`D2 = 2`, `0xc0000a`): `DSW2` (inverted)
     - `idx = 6` (`D2 = 1`, `0xc0000c`): `SYSTEM` (inverted)
     - `idx = 7` (`D2 = 0`, `0xc0000e`): unmapped port (inverted)
   - For each word, shifts out 16 bits from MSB to LSB (`D1 = 16..=1`), inserting a space cell (`0x003c`) before bit 7 (`D1 == 8`), writing 17 word cells (`0x0000` or `0x0001`) per row into `0xff8732..0xff8841`, then calls `text_block(0xff872c)` (`0x3bba0`).
2. **Adopted MAME `0.289` behaviour on unmapped ports `0xc00004`, `0xc00006`, `0xc0000e` (for `F77` / `#149`)**:
   - In `sf-mame0289.cpp:167, 173–174, 178`, `sf_state::sfus_map` sets `map.unmap_value_high()`, and maps `0xc00004`, `0xc00006`, and `0xc0000e` as `nopr()` while mapping `0xc00000..0xc00003` (`IN0`, `IN1`), `0xc00008..0xc0000b` (`DSW1`, `DSW2`), and `0xc0000c..0xc0000d` (`SYSTEM`) for reads in `0xc00000..0xc0000f`.
   - Reads of `0xc00004`, `0xc00006`, and `0xc0000e` therefore return `0xffff`. Because `0x484de..0x484f2` does not invert `0xc00004` or `0xc00006` (`D2 == 5` and `D2 == 4`) and does invert `0xc0000e` (`D2 == 0`), `Io` models `unmap_c00004 = 0xffff`, `unmap_c00006 = 0xffff`, and `unmap_c0000e = 0xffff`, producing `11111111 11111111` on rows 2–3 (`0xff8776..0xff87ba`) and `00000000 00000000` on row 7 (`0xff8820..0xff8841`).
3. **Punch/kick release strength display (`0x48504..0x48548`)**:
   - Reads the 4 latched release bytes via pointer table `0x49a96` (`0xff86b0`, `0xff86b1`, `0xff86da`, `0xff86db`); whenever a byte is nonzero and differs from the stored byte in `0xff8862..0xff8865`, updates `0xff8862..0xff8865`.
   - Calls `write_hex_byte` (`0x3c014`) with attribute `0x5000` for all 4 stored bytes at the video-RAM offsets in `0x49a8e` (`0x8005aa`, `0x8006aa`, `0x8005b6`, `0x8006b6`). Under button-version `sf` (`vblank_buttons`), releasing light/medium/heavy buttons latches `0x40` / `0xe0` / `0xfe`.
4. **Sound selector and playback (`0x4854a..0x485c2`)**:
   - Increments `0xff8848` every frame; on every 5th frame (`0xff8848 == 5`), resets `0xff8848 = 0` and checks `(!IN1) & 0x000f`:
     - If `== 8` (P1 Up alone), flips `0xff8846 ^= 0xff`; if the result is `0`, branches directly to `0x485c4` (skipping both the hex redraw at `0x4858e` and the punch check at `0x4859e`); if `0xff`, increments `D7` by `1`.
     - If `== 4` (P1 Down alone), flips `0xff8847 ^= 0xff`; if the result is `0`, branches directly to `0x485c4`; if `0xff`, decrements `D7` by `1`.
     - Unless branched to `0x485c4`, writes `D7` as 2 hex digits at `0x800652` (`write_hex_byte` with attribute `0x9000`).
   - At `0x4859e..0x485c2`: if `0xff86b0 > 4` (any P1 punch released on this frame), flips `0xff8849 ^= 0xff` and, when `0xff8849 != 0`, queues sound `(D7 & 0xbf)` via `queue_sound` (`0x3b7c8`).
5. **Exit to Service Menu (`0x485c4..0x48618` and `trap #3` at `0x3b29a..0x3b2b2`)**:
   - Checks `move.w ($2,A6),D0; andi.w #$8,D0` (P1 Up held, `IN1` bit 3 low at `0xc00002`) and `cmpi.w #4,D0` on `0xff86b1` (`0xff86b1 > 4`, P1 kick released on this frame).
   - When both hold (`0x485f2..0x48618`), clears `0xffc002 = 0` and `0xffc008 = 0`, clears `0xff86b0..0xff86b1 = 0` and `0xff86da..0xff86db = 0`, calls `queue_boot_sounds` (`0x3b7a8`), spawns slot `1` at `0x48644` (`trap #0`), and executes `trap #3` (`0x3b29a..0x3b2b2`), which clears byte `+0` of the current slot (`0xff8006`) and jumps to `0x3b19a` to rescan from slot `0` in the same vblank.

---

## 4. Verification (`sf-service-01`, `crates/sf-engine/tests/service_mode.rs`)

- **Scenario**: `tools/reference/scenarios/sf_service_01.json` (`42` frames `0..=41`, `Service Mode = On`, full checkpoints on every frame).
  - Frames `0..=15`: reset clear (`0`) and RAM test (`1..=15`).
  - Frames `16..=19`: 4 boot palette uploads (`0x1ae68`, `0x1a268`, `0x1ba68`, `0x1bc68`).
  - Frame `20`: first service test screen (`SERVICE_TEST_SCREEN_FRAME = 20`).
  - Frame `21` (held 1 frame, active at frame `22`, released at frame `23`): `start1` (`SYSTEM` bit 0), `service1` (`SYSTEM` bit 2), `p2_left` (`IN1` bit 9), `p1_button4` (P1 LK -> `0x40` at `0xff8863`), `p2_button2` (P2 MP -> `0xe0` at `0xff8864`), and `p2_button6` (P2 HK -> `0xfe` at `0xff8865`).
  - Frame `23` (held 2 frames): `p1_up` increments `D7` to `0x01` on `tick == 5` at frame `24`.
  - Frame `26` (held 1 frame, released at frame `28`): `p1_button3` (P1 HP -> `0xfe` at `0xff8862`, toggles `0xff8849` to `0xff`, and queues sound `0x01`).
  - Frame `30` (held 1 frame, released at frame `32`): `p1_button1` (P1 LP -> `0x40` at `0xff8862`, toggles `0xff8849` to `0x00`).
  - Frame `33` (held 2 frames): `p1_down` decrements `D7` back to `0x00` on `tick == 5` at frame `34`.
  - Frame `38` (`p1_button6` held 2 frames, released at frame `41`) + frame `40` (`p1_up` held 2 frames, active at frame `41`): triggers exit at `0x485f2..0x48618` (`trap #3` on slot `0`, `trap #0` spawn of slot `1` at `0x48644`).
- **Results**:
  - `service-01-run-a` and `service-01-run-b` are byte-for-byte identical across all `213` files.
  - Frame `0` matches with zero allowance (`RESET_CLEAR_FRAME0_END = 0xffc564`); frames `1..=15` match on `screen_hash`, `latch`, `latch_changed`, pixels, and palette RAM and differ only on the exact work/video RAM ranges in the table above; frames `16..=40` match 100% on `sf_compare::compare` and all CPU-owned work RAM; frame `41` exits `0x48618` and reports `0x48644` (`F74`, `#146`).

---

## 5. Service Menu and Screen Tests 1–4 (`0x48644..0x48b68` and `0x4929c..0x492d5`, `F74` / `#146`)

### 5.1 Service Menu (`0x48644..0x487f6`, Slot 1) and Menu Erase (`0x4929c..0x492d5`)

1. **Menu initialization (`0x48644..0x48690`)**:
   - Calls `reset_button_latches` (`0x3b6c6`), sets `0xffc002 = 1` and `0xffc008 = 0x80` (enabling `vblank_buttons` release latches), clears `0xff86b0..0xff86b1 = 0` and `0xff86da..0xff86db = 0`, erases the 5 input-screen text blocks at `0x486c0..0x486d0` via `erase_block` (`0x3bd00`), and zeros `D7 = 0` (`0x48676`).
   - At `0x48678` (`service_menu_draw`, re-entered from `0x487e4` after leaving a test item without resetting `D7`), draws the 9 menu text blocks via pointer table `0x486d4..0x486f4` (2 headers plus the 7 item lines at `0x486dc..0x486f4`) and initializes `0xff884a = 8`. Because `0xff884a` starts at `8` (`P1 Up`), any P1 Up still held from the `P1 Up + Kick` exit chord (`0x485c4` or `0x4877e`) flips `0xff884a` from `8` to `0` on the first cursor step (`0x48692..0x486a6`) without moving `D7`.
2. **Cursor step and 6-frame blink (`0x48692..0x48744`)**:
   - Every 12 frames — 6 frames (`0x3b532` at `0x48716`) before erasing the highlighted item `D7` (`0x486dc + 4 * D7` via `erase_block` at `0x4871c..0x48728`), and 6 frames (`0x3b532` at `0x4872a`) before redrawing the highlighted item `D7` (`0x486dc + 4 * D7` via `text_block` at `0x4873a..0x48744`) and stepping the cursor — `0x48692..0x48715` (`service_menu_step_cursor`) reads `(!IN1) & 0x000c` (`0xc00002`):
     - If `0` (neither P1 Up nor P1 Down held), clears `0xff884a = 0`.
     - If `8` (P1 Up held): XORs `0xff884a ^= 8`; when the low byte becomes nonzero, decrements `D7` and wraps `-1 -> 6` (`0x486ba`).
     - If `4` (P1 Down held): XORs `0xff884a ^= 4`; when the low byte becomes nonzero, increments `D7` and wraps `7 -> 0` (`0x4870c`).
3. **Entering and leaving an item (`0x48730..0x487f6`)**:
   - **Which buttons give a latched strength above 4 (`[inf]`)**: under button-version `sf`, `vblank_buttons` (`0x3b05a..0x3b0c8` -> `button_latch` `0x3b3ae..0x3b3fe`) latches `0x40` for light (`p1_button1` LP / `p1_button4` LK), `0xe0` for medium (`p1_button2` MP / `p1_button5` MK), and `0xfe` for heavy (`p1_button3` HP / `p1_button6` HK) on release (`0xff86b0` for P1 punches, `0xff86b1` for P1 kicks). All three P1 punch buttons (`p1_button1..3`) therefore yield `0xff86b0 > 4` on release to enter an item (`0x48730..0x48772`), and all three P1 kick buttons (`p1_button4..6`) yield `0xff86b1 > 4` on release while P1 Up (`IN1` bit 3 low) is held to leave the input screen (`0x485c4..0x485dc`) or leave a test item (`0x4877e..0x4879e`).
   - When `0xff86b0 > 4` (`0x48730`), `0x4876a..0x4877c` looks up the item entry in the 7-longword table at `0x4874e..0x48769` (`0x487f8`, `0x488f8`, `0x48858`, `0x48990`, `0x48b6a`, `0x48da4`, `0x490f8`) and spawns slot `2` (`trap #0` with `D0 = 2`).
   - In the item wait loop `0x4877e..0x487f6`, slot `1` sleeps 1 frame (`trap #2` at `0x4877e`) and checks P1 Up held (`($2,A6) & 8 == 0`) with `0xff86b1 > 4`. On exit (`0x487a0..0x487e4`), it kills slot `2` (`trap #1` at `0x3b230` with `D0 = 2`), calls `service_screen_init` (`0x3b5c8`), enables the text layer (`*bus.gfxctrl = 8`, `0xff8682 = 8`), requests palette `0x1bc68 -> 0x0600` (`0xff8634 = 0x1bc68`, `0xff8638 = 0x0600`, `0xff8633 = 0xff`) if `0xff8634 == 0x1be68` (set by Item 2 Color Bar), clears `0xff86b0..0xff86b1 = 0` and `0xff86da..0xff86db = 0`, sleeps 1 frame (`trap #2` at `0x487e2`), and branches to `0x48678` (`service_menu_draw`, preserving `D7`).
4. **Menu erase helper (`0x4929c..0x492d5`, `erase_service_menu`)**:
   - Loops `9` times (`D1 = 8..=0`) over the 9 menu text pointers at `0x492b2..0x492d2`, calling `erase_block` (`0x3bd00`) on each.

### 5.2 Screen Test Items 1–4 (`0x487f8..0x48b68`, Slot 2)

1. **Item 1 — Crosshatch (`0x487f8..0x48856`)**:
   - Draws the `14 x 24` `2x2` crosshatch grid (`0x1043..0x1046`) across video RAM starting at row offsets `0x0110` and `0x0190` (stepping rows by `0x0100` and columns by `4`), and loops on `0x3b542` (`Yield::Sleep(0x12)` at `0x48850`). Reads no inputs directly.
2. **Item 2 — Color Bar (`0x488f8..0x4898e`)**:
   - Calls `erase_service_menu` (`0x4929c`), draws the 4 headers listed at `0x48934..0x48940` (`0x49754`, `0x49762`, `0x4977e`, `0x4978c`), requests palette `0x1be68 -> 0x0600` (`0xff8634 = 0x1be68`, `0xff8638 = 0x0600`, `0xff8633 = 0xff`), fills 4 color bands of `4` rows x `32` words starting at `0x8004aa` (`0x403c`, `0x503c`, `0x603c`, `0x703c`, incrementing the tile index on each of the 4 rows in a band), and loops on `0x3b542` (`Yield::Sleep(0x12)` at `0x48988`). Reads no inputs directly.
3. **Item 3 — Character Test 1 (`0x48858..0x488f6`)**:
   - Calls `erase_service_menu` (`0x4929c`), draws the 4 headers listed at `0x488e8..0x488f4` (`0x496d2`, `0x496e0`, `0x49714`, `0x49722`), writes `16` column headers (`0x5000..=0x500f`, stride `4`) at `0x800420`, fills 4 bands of `4` rows x `16` longwords starting at `0x800520` (`0x003c_003c`, stepping color attributes by `0x1000_1000` across the 16 columns and tile indices by `0x0001_0001` across the 4 bands), and loops on `0x3b542` (`Yield::Sleep(0x12)` at `0x488e0`). Reads no inputs directly.
4. **Item 4 — Character ROM Test (`0x48990..0x48b68`)**:
   - Calls `erase_service_menu` (`0x4929c`), draws the 7 headers listed at `0x489d0..0x489e8` (`0x496d2`, `0x49874`, `0x49714`, `0x498ac`, `0x498ba`, `0x498c8`, `0x498d8`), writes `16` column headers (`0x9000..=0x900f`, stride `4`) at `0x800524`, fills the `16 x 16` character table (`0x0000..=0x00ff`, column stride `4`, row stride `0x0080`) at `0x800624` with row headers `((0x8000 + row) << 16) | 0x8000` at `0x80061c`, and clears `0xff884a = 0` (direction edge latch), `0xff884c = 0` (2-bit page selector), and `0xff884e = 0` (4-bit color attribute selector).
   - In the per-frame loop `0x48a40..0x48b06` (`service_char_rom_loop`), reads `(!IN1) & 0x000f` at `0x48a40`:
     - `8` (P1 Up alone): when `0xff884a != 8`, sets `0xff884a = 8`, increments `0xff884c = (0xff884c + 1) & 3`, and calls `char_rom_update_page` (`0x48b08..0x48b1a`), which writes `0xff884c` as hex at `0x8003a2` (`write_hex_byte` with attribute `0x5000`) and updates bits `8..9` (`(cell & 0xfcff) | (page << 8)`) across the `16 x 16` table at `0x800624` (`0x48b2e..0x48b68`).
     - `4` (P1 Down alone): when `0xff884a != 4`, sets `0xff884a = 4`, decrements `0xff884c = (0xff884c - 1) & 3`, and calls `char_rom_update_page` (`0x48b08..0x48b1a`).
     - `1` (P1 Right alone): when `0xff884a != 1`, sets `0xff884a = 1`, increments `0xff884e = (0xff884e + 1) & 0x0f`, and calls `char_rom_update_color` (`0x48b1c..0x48b2c`), which writes `0xff884e` as hex at `0x8003ba` (`write_hex_byte` with attribute `0x5000`) and updates bits `12..15` (`(cell & 0x0fff) | (color << 12)`) across the `16 x 16` table at `0x800624` (`0x48b2e..0x48b68`).
     - `2` (P1 Left alone): when `0xff884a != 2`, sets `0xff884a = 2`, decrements `0xff884e = (0xff884e - 1) & 0x0f`, and calls `char_rom_update_color` (`0x48b1c..0x48b2c`).
     - Any other value: clears `0xff884a = 0`.

### 5.3 Verification (`sf-service-02`, `crates/sf-engine/tests/service_mode.rs`)

- **Scenario**: `tools/reference/scenarios/sf_service_02.json` (`2,449` frames `0..=2448`, `Service Mode = On`, full checkpoints on every frame).
  - Frame `22`: exits the input screen (`0x485f2..0x48618`) to the service menu (`0x48644`).
  - Frame `46`: P1 Up wraps the menu cursor `0 -> 6` (`0x486a8..0x486bc`).
  - Frame `70`: P1 Down wraps the menu cursor `6 -> 0` (`0x486f8..0x4870e`).
  - Frames `82..=2385`: enters and leaves Items 1–4 (`0x487f8` at frame `82`, `0x488f8` at frame `658`, `0x48858` at frame `1235`, `0x48990` at frame `1811`) in turn; in each item, holds all 8 P1/P2 directions alone for 60 frames each and presses all 17 buttons once with no direction held before exiting via P1 Up + Kick release (last exit from Item 4 at frame `2361`, redrawing the menu at frame `2362` and stepping the cursor to Item 5 through frame `2385`).
  - Frames `2386..=2448`: enters Item 5 (`0x48b6a`) at frame `2386`, leaves it at frame `2392`, enters Item 6 (`0x48da4`) at frame `2417`, leaves it at frame `2423`, and enters Item 7 (`0x490f8`) at frame `2448`.
- **Results**:
  - `service-02-run-a` and `service-02-run-b` are byte-for-byte identical across all `12,248` files.
  - Under `F74` (`#146`), frames `0..=2385` matched under C-NAT (`frame 0` with zero allowance, `1..=15` with `RAM_TEST_DIFF_RANGES`, and `16..=2385` with 100% `sf_compare::compare` and CPU-owned work-RAM match), and frames `2386..=2448` reported `vec![0x48b6a, 0x48da4, 0x490f8]`. With `F75` (`#147`) and `F76` (`#148`) translating Items 5 (`0x48b6a`), 6 (`0x48da4`), and 7 (`0x490f8`), `sf-service-02` now matches across the entire scenario (`0..=2448`) under C-NAT with zero unrecovered PCs.

---

## 6. Service Sprite Test and Output Cycle (`0x48b6a..0x48da2` and `0x490f8..0x492d5`, with `0x3b93c` and `0x3b664`, `F75` / `#147`)

### 6.1 Helpers `0x3b93c..0x3b96a` (`Bus::clear_words_8`) and `0x3b664..0x3b672` (`aux_ctrl_or`)

1. **`0x3b93c..0x3b96a` (`Bus::clear_words_8`)**:
   - Sets `D1 = 7` and falls through `0x3b964` (`clr.w (A0)+; dbra D1,$3b964; rts`), clearing 8 consecutive words (`16` bytes) from `A0` (used by `0x48e4e` and `0x48e70` in `F76` on `0xff8852..0xff885a` and `0xff885a..0xff8862`).
2. **`0x3b664..0x3b672` (`aux_ctrl_or`)**:
   - ORs word `D0` into `AUX_CONTROL` (`0xff8686`) and writes the resulting word to `0xc00012` (`($12,A6)`).

### 6.2 Item 5 — Sprite / Object ROM Test (`0x48b6a..0x48da2`, Slot 2)

1. **Initialization (`0x48b6a..0x48c1c`, `service_sprite_test_init`)**:
   - Sets `A7 = 0xff83a2` and `A1 = 0x800000`, calls `erase_service_menu` (`0x4929c`), draws the 7 headers at `0x48d88..0x48da0` (`0x496d2`, `0x498e6`, `0x49714`, `0x498ac`, `0x498ba`, `0x498c8`, `0x498d8`), and writes `16` column headers (`0x9000..=0x900f`, stride `4`) at `0x800520`.
   - Clears `0xff8850 = 0` (`16`-bit sprite page/code base: high byte `0xff8850` = `0..=0x3f` page, low byte `0xff8851` = `0x00, 0x20, ..., 0x80` sub-page) and `0xff884e = 0` (`4`-bit sprite color attribute).
   - Calls `sprite_test_row_headers` (`0x48d52..0x48d86`) with `D0 = 0` to write 8 two-digit hex row headers (`0x9000_9000 | (hi << 16) | lo`, advancing `D0` by `0x10`) at `0x800618` (row stride `0x0100`).
   - Populates 32 object-RAM sprite entries (`4` rows x `8` columns) starting at `0xffe080` (`($80,A4)`, entry stride `0x40`):
     - `+0` (code): `row * 0x20 + col * 2` (`0x0000..=0x006e`);
     - `+2` (attr): `0x0400` (`16x32` two-tile vertical height bit `0x0400`, palette `0`);
     - `+4` (`y, x` longword): `((0x0060 + row * 0x20) << 16) | (0x0080 + col * 0x20)`.
   - Clears `0xff884a = 0` (direction edge latch), calls `gfx_on(0x80)` (`0x3b644`, enabling the object/sprite layer in `GFXCTRL`), and sleeps 1 frame (`0x3b51a` at `0x48c18`).
2. **Per-frame `IN1` selector loop (`0x48c1e..0x48d50`, `service_sprite_test_loop`)**:
   - Reads `(!IN1) & 0x000f` (`0xc00002` low nibble, P1 directions) into `D1` and tests `D0 = 8, 4, 2, 1` via `0x48c4e`:
     - If `D1` is not one of `{8, 4, 2, 1}` (no single P1 cardinal direction held), clears `0xff884a = 0` (`0x48c48`) and sleeps 1 frame.
     - If `0xff884a == D1 as u8` (same direction still held), sleeps 1 frame without repeating.
     - Otherwise stores `0xff884a = D1 as u8` and dispatches via `0x48c6a`:
       - `D1 == 1` (P1 Right alone, `0x48d0e`): increments `0xff884e = (0xff884e + 1) & 0x000f` and calls `sprite_test_update_color` (`0x48d20..0x48d50`).
       - `D1 == 2` (P1 Left alone, `0x48d16`): decrements `0xff884e = (0xff884e - 1) & 0x000f` and calls `sprite_test_update_color` (`0x48d20..0x48d50`).
       - `D1 == 8` (P1 Up alone, `0x48c82..0x48ca0`): adds `0x20` to `0xff8851`; if `0xff8851 > 0x80` (unsigned `bls $48cd2` not taken when `0xff8851 == 0xa0`), increments `0xff8850 = (0xff8850 + 1) & 0x3f`, resets `0xff8851 = 0x00`, and writes `0xff8850` as hex at `0x8003a2` (`write_hex_byte` with attribute `0x5000`); then runs `sprite_test_update_page` (`0x48cd2..0x48d0c`).
       - `D1 == 4` (P1 Down alone, `0x48ca4..0x48cbe`): subtracts `0x20` from `0xff8851`; if borrow (`bcc $48cd2` not taken), decrements `0xff8850 = (0xff8850 - 1) & 0x3f`, sets `0xff8851 = 0x80`, and writes `0xff8850` as hex at `0x8003a2`; then runs `sprite_test_update_page` (`0x48cd2..0x48d0c`).
   - `sprite_test_update_page` (`0x48cd2..0x48d0c`) updates the 8 row headers at `0x800618` from `0xff8851` (`0x48d52`) and rewrites `+0` across all 32 sprites (`0xffe080 + i * 0x40`) from the 16-bit base word at `0xff8850`.
   - `sprite_test_update_color` (`0x48d20..0x48d50`) writes `0xff884e` as hex at `0x8003ba` (`write_hex_byte` with attribute `0x5000`) and updates `+2` across all 32 sprites (`0xffe082 + i * 0x40`) to `0x0400 | (0xff884e & 0x000f)`.

### 6.3 Item 7 — Output Port Cycle (`0x490f8..0x4929a`, Slot 2)

1. **Initialization (`0x490f8..0x49178`, `service_output_test_init`)**:
   - Sets `A7 = 0xff83a2` and `A1 = 0x800000`, calls `erase_service_menu` (`0x4929c`), draws the 7 headers at `0x4917a..0x49192` (`0x49920`, `0x4992e`, `0x4994c`, `0x4995a`, `0x49990`, `0x499a8`, `0x499de`), clears `+4` (`y, x`) of all 32 sprite entries (`0xffe084 + i * 0x40`), and places a single test sprite at `0xffe080` (`+0 = 0x008c`, `+2 = 0x0402`, `+4 = 0x0090_0108`).
   - Clears `0xffc680 = 0` and `0xffc682 = 0`, calls `add_fg_scroll(0x13a0)` (`0x3b674`) and `add_bg_scroll(0x1380)` (`0x3b682`), requests palette `0x1a268 -> 0x0200` (`0x49070`, sleeping 1 frame), then requests palette `0x1ae68 -> 0x0000` (`0x4908c`, sleeping 1 frame), and enters the output reset/apply loop with `D1 = 0`.
2. **30-frame baseline reset and 30-frame output step (`0x49196..0x4929a`)**:
   - `output_test_reset_outputs` (`0x49196..0x491ce`) sets `GFXCTRL = 0x00e8` (`0xc0001b` and `0xff8682`, enabling tx/fg/bg/obj with flip screen off), clears `AUX_CONTROL = 0` (`0xc00012` and `0xff8686`), clears `COIN_CONTROL = 0` (`0xc00010` and `0xff8684`), clears the H/V flip bits on sprite 0 (`0xffe082 &= 0x04ff`), clears the H/V flip bits on character cell `0x8005c2` (`0x8005c2 &= 0xf3ff`), and sleeps `30` frames (`0x3b552`).
   - `output_test_apply_step` (`0x491d0..0x491ec`) reads the 6-byte entry `(fn_addr, d0)` at `0x491ee + D1` (`25` entries for `D1 = 0, 6, ..., 0x90`, wrapping `0x96 -> 0` at `0x491e4..0x491ec`), calls `fn_addr(d0)`, and sleeps `30` frames (`0x3b552`):
     - Entries `0..=5` (`D1 = 0x00..=0x1e`): `coin_ctrl_or` (`0x3b654`) with `0x0010`, `0x0020`, `0x1000`, `0x2000`, `0x0001`, `0x0002` (`0xff8684` / `0xc00010`).
     - Entries `6..=7` (`D1 = 0x24..=0x2a`): `coin_ctrl_or` (`0x3b654`) with `0x0100`, `0x0200` (`0xff8684` / `0xc00010`).
     - Entries `8..=15` (`D1 = 0x30..=0x5a`): `aux_ctrl_or` (`0x3b664`) with `0x0001`, `0x0002`, `0x0004`, `0x0008`, `0x0010`, `0x0020`, `0x0040`, `0x0080` (`0xff8686` / `0xc00012`).
     - Entries `16..=18` (`D1 = 0x60..=0x6c`): `gfx_off` (`0x3b690`) with `0x40` (bg off), `0x80` (obj off), `0x08` (tx off).
     - Entries `19..=20` (`D1 = 0x72..=0x78`): `0x49290` (`0x8005c2 |= d0`) with `0x0800` (char V-flip) and `0x0400` (char H-flip).
     - Entries `21..=23` (`D1 = 0x7e..=0x8a`): `0x49284` (`0xffe082 |= d0`) with `0x0400`, `0x0200` (sprite V-flip), and `0x0100` (sprite H-flip).
     - Entry `24` (`D1 = 0x90`): `gfx_on` (`0x3b644`) with `0x04` (flip screen on).

### 6.4 Verification (`sf-service-03a`, `crates/sf-engine/tests/service_mode.rs`)

- **Scenario**: `tools/reference/scenarios/sf_service_03a.json` (`2,235` frames `0..=2234`, `Service Mode = On`, full checkpoints on every frame).
  - Frame `22`: exits the input screen to the service menu (`0x48644`).
  - Frame `130`: steps the cursor to Item 5 (`0x48b6a`) and enters it.
  - Frames `131..=643`: taps P1 Up 5 times (wrapping `0xff8851: 0x80 -> 0x00` and incrementing `0xff8850: 0x00 -> 0x01` at frame `144`), taps P1 Down twice (borrowing `0xff8851: 0x00 -> 0x80`, `0xff8850: 0x01 -> 0x00` at frame `147`, and stepping `0xff8851: 0x80 -> 0x60` without borrow at frame `150`), and holds each of the 8 P1/P2 cardinal directions alone for at least 60 frames before exiting Item 5 via P1 Up + Kick release at frame `644`.
  - Frame `693`: steps the cursor to Item 7 (`0x490f8`) and enters it.
  - Frames `694..=2229`: runs Item 7 for `1,537` frames (`>= 600` frames), exercising all `25` entries (`D1 = 0x00..=0x90`) of `0x491ee..0x49283` and the `0x96 -> 0x00` wrap at frame `2195`, then exits Item 7 at frame `2230` back to the service menu through frame `2234`.
- **Results**:
  - `service-03a-run-a` and `service-03a-run-b` are byte-for-byte identical across all `11,178` files.
  - Frames `0..=2234` match 100% under C-NAT (`frame 0` with zero allowance, `1..=15` with `RAM_TEST_DIFF_RANGES`, and `16..=2234` with 100% `sf_compare::compare`, CPU-owned work RAM including `0xff8684..0xff8688`, and full object RAM `0xffe000..0x1000000`).
  - The coverage ledger (`docs/reference/coverage-ledger.md`) shows `0x3b664..0x3b672`, `0x3b934..0x3b96a`, `0x48b6a..0x48da2`, and `0x490f8..0x4929a` translated.

---

## 7. Service Scroll Test (`0x48da4..0x490f6`, `F76` / `#148`)

### 7.1 Initialization (`0x48da4..0x48e8a`, Slot 2)

1. **FG initialization (`0x48da4..0x48e68`, `service_scroll_test_init_fg`)**:
   - Sets `A7 = 0xff83a2` and `A1 = 0x800000`, calls `erase_service_menu` (`0x4929c`), and draws the 5 static text blocks listed at `0x48dd6..0x48de6` (`0x4979c`, `0x497aa`, `0x497f8`, `0x49818`, `0x49830`).
   - Draws the `7 x 6` grid of `0x6045` tick marks starting at `0x8001c0` (`7` rows with row stride `+0x0200`, `6` columns per row with column stride `+8`, `0x48dea..0x48e08`).
   - Draws the horizontal (`0x6000..=0x6006`) and vertical (`0x6001..=0x6007`) axis digits at `0x8001bc`, `0x8001c4 + 8 * i`, `0x8002bc + 0x0200 * i`, and `0x800e3c` (`0x48e0a..0x48e3c`).
   - Clears `FG_SCROLL_COPY = 0` (`0xffc680`) and `BG_SCROLL_COPY = 0` (`0xffc682`), clears the 8 words at `0xff8852..0xff8862` via `Bus::clear_words_8` (`0x3b93c`), calls `add_fg_scroll(0)` (`0x3b674`), calls `scroll_test_draw_fg_coords` (`0x490a4..0x490cc`), stores the initial FG palette pointer `0x1a268` (`0x48f28`) at `0xff8856`, and requests FG palette `0x1a268 -> 0x0200` (`0x4906c..0x49086`, sleeping 1 frame via `0x3b51a` at `0x49080`).
2. **BG initialization (`0x48e6a..0x48e8a`, `service_scroll_test_init_bg`)**:
   - On wakeup (`0x49086 -> 0x48e68`), sets `A2 = 0xff885a`, clears the 8 words at `0xff885a..0xff886a` via `Bus::clear_words_8` (`0x3b93c`), calls `add_bg_scroll(0)` (`0x3b682`), calls `scroll_test_draw_bg_coords` (`0x490ce..0x490f6`), stores the initial BG palette pointer `0x1ae68` (`0x48fd2`) at `0xff885e`, and requests BG palette `0x1ae68 -> 0x0000` (`0x49088..0x490a2`, sleeping 1 frame via `0x3b51a` at `0x4909c`).
   - On wakeup (`0x490a2 -> 0x48e8a`), calls `gfx_on(0x60)` (`0x3b644`, enabling both FG and BG tilemap layers in `GFXCTRL`) and falls through `0x48e96` into the per-frame FG/BG scroll loop.

### 7.2 Per-Frame FG and BG Scroll Steps (`0x48e96..0x490f6`)

1. **Direction decode and auto-repeat (`0x48ff4..0x4906b`, `scroll_test_step_input`)**:
   - Each 8-byte scroll control block (`0xff8852..0xff885a` for FG, `0xff885a..0xff8862` for BG) holds:
     - `+0` (`u8`): frame delay counter before consuming `+1`;
     - `+1` (`i8`): pending signed scroll delta (`+64`, `-64`, `+32`, or `-32`);
     - `+2` (`u8`): held cardinal direction latch (`8`, `4`, `2`, or `1`);
     - `+3` (`u8`): 2-bit auto-repeat counter (`0..=3`);
     - `+4..+8` (`u32`): current layer palette pointer (`0xff8856` for FG, `0xff885e` for BG).
   - If `+1 != 0` (a pending step has not yet been consumed because `+0` is still counting down), `0x48ff4` returns immediately (`0x48ff8: bne.s $4905e`).
   - Otherwise reads `D1 = (!raw_byte) & 0x0f` (`0xc00003` for P1 FG, `0xc00002` for P2 BG) and searches `0x49060..0x49063` (`[8, 4, 2, 1]` for `D0 = 3..=0`):
     - If `D1 == 0` (`0x49002`) or `D1` is a diagonal not matching `[8, 4, 2, 1]` (`0x49016` `dbra` fallthrough), clears `+2 = 0` and `+3 = 0` (`0x49052..0x4905e`).
     - If `+2 != D1` (`0x49020`), latches `+2 = D1`, clears `+3 = 0`, and loads `(step, delay)` from `0x49064 + 2 * D0` into `(+1, +0)`:
       - `D0 = 3` (`D1 = 8`, Up): `step = +0x40` (`+64`), `delay = 1`;
       - `D0 = 2` (`D1 = 4`, Down): `step = -0x40` (`0xc0`, `-64`), `delay = 1`;
       - `D0 = 1` (`D1 = 2`, Left): `step = +0x20` (`+32`), `delay = 12` (`0x0c`);
       - `D0 = 0` (`D1 = 1`, Right): `step = -0x20` (`0xe0`, `-32`), `delay = 12` (`0x0c`).
     - If `+2 == D1` (`0x49038..0x49050`), increments `+3 = (+3 + 1) & 3`; when `+3` wraps to `0`, clears `+2 = 0` so a held cardinal direction re-triggers on the next check.
2. **FG step and stage palette lookup (`0x48e96..0x48f3f`, `service_scroll_test_fg_step`)**:
   - Calls `scroll_test_step_input` on `A2 = 0xff8852` and `0xc00003`.
   - If `0xff8852 != 0`, decrements `0xff8852` and skips to BG (`0x48f40`).
   - Otherwise, if `0xff8853 != 0`, sign-extends `0xff8853` to 16 bits, clears `0xff8853 = 0`, calls `add_fg_scroll(delta)` (`0x3b674`), and calls `scroll_test_draw_fg_coords` (`0x490a4..0x490cc`), which writes `FG_SCROLL_COPY` (`0xffc680`) at `0x80069c` (`attr = 0x9000`) and `FG_SCROLL_COPY + 0x0100` at `0x8006a8` (`attr = 0x5000`) via `write_hex_word` (`0x3bffc`).
   - Scans the 6 word-pair bounds at `0x48f10..0x48f27` (`0x0000..=0x1aff`, `0x1b00..=0x35ff`, `0x3600..=0x50ff`, `0x5100..=0x6bff`, `0x6c00..=0x83ff`, `0x8400..=0xffff`) against `FG_SCROLL_COPY` (`0xffc680`). When the corresponding FG palette pointer in `0x48f28..0x48f3f` (`0x1a268`, `0x1a468`, `0x1a668`, `0x1a868`, `0x1aa68`, `0x1ac68`) differs from `0xff8856`, stores the new pointer at `0xff8856`, requests it at `0x0200` (`0x4906c..0x49086`), and sleeps 1 frame (`0x49080`) before `0x48f0c: bra $48f40` runs the BG step on the wakeup frame (`ServiceItem::ScrollTestBgOnly`).
3. **BG step and stage palette lookup (`0x48f40..0x48ff3`, `service_scroll_test_bg_step`)**:
   - Calls `scroll_test_step_input` on `A2 = 0xff885a` and `0xc00002`.
   - If `0xff885a != 0`, decrements `0xff885a`, sleeps 1 frame (`0x48fea`), and loops to `0x48e96`.
   - Otherwise, if `0xff885b != 0`, sign-extends `0xff885b` to 16 bits, clears `0xff885b = 0`, calls `add_bg_scroll(delta)` (`0x3b682`), and calls `scroll_test_draw_bg_coords` (`0x490ce..0x490f6`), which writes `BG_SCROLL_COPY` (`0xffc682`) at `0x80051c` (`attr = 0x9000`) and `BG_SCROLL_COPY + 0x0100` at `0x800528` (`attr = 0x5000`) via `write_hex_word` (`0x3bffc`).
   - Scans the 6 word-pair bounds at `0x48fba..0x48fd1` (`0x0000..=0x14ff`, `0x1500..=0x29ff`, `0x2a00..=0x3eff`, `0x3f00..=0x53ff`, `0x5400..=0x65ff`, `0x6600..=0xffff`) against `BG_SCROLL_COPY` (`0xffc682`). When the corresponding BG palette pointer in `0x48fd2..0x48fe9` (`0x1ae68`, `0x1b068`, `0x1b268`, `0x1b468`, `0x1b668`, `0x1b868`) differs from `0xff885e`, stores the new pointer at `0xff885e` and requests it at `0x0000` (`0x49088..0x490a2`, sleeping 1 frame at `0x4909c` before `0x48fb6: bra $48e96`); otherwise sleeps 1 frame at `0x48fea` and loops to `0x48e96`.

### 7.3 Verification (`sf-service-03b`, `crates/sf-engine/tests/service_mode.rs`)

- **Scenario**: `tools/reference/scenarios/sf_service_03b.json` (`911` frames `0..=910`, `Service Mode = On`, full checkpoints on every frame).
  - Frame `22`: exits the input screen to the service menu (`0x48644`).
  - Frames `34`, `58`, `70`: steps the cursor via P1 Up (`0 -> 6 -> 5`) and enters Item 6 (`0x48da4`) at frame `70`.
  - Frames `74..=568`: holds each of the 8 P1/P2 cardinal directions alone for 60 frames (`p1_down` `74..134`, `p1_right` `136..196`, `p1_left` `198..258`, `p1_up` `260..320`, `p2_down` `322..382`, `p2_right` `384..444`, `p2_left` `446..506`, `p2_up` `508..568`), exercising all 4 `(step, delay)` entries in `0x49064..0x4906b` and the 4-step auto-repeat wrap (`0x49038..0x49050`) on both FG and BG.
  - Frames `570..=572`: holds `p1_up + p1_left` (`D1 = 0x0a`), exercising the diagonal `dbra` fallthrough at `0x49016`.
  - Frames `574..=900`: pulses `p1_up + p2_up` (held 2 frames, released 1 frame) 109 times, advancing `BG_SCROLL_COPY` across `0x1500` (triggering `0x48fb2: bsr $49088` -> `0x1b068` at `0x0000`) and `FG_SCROLL_COPY` across `0x1b00` (triggering `0x48f08: bsr $4906c` -> `0x1a468` at `0x0200` and `0x48f0c: bra $48f40`).
  - Frame `906`: exits Item 6 via P1 Up + Kick release back to the service menu through frame `910`.
- **Results**:
  - `service-03b-run-a` and `service-03b-run-b` are byte-for-byte identical across all `4,558` files (`911` checkpoints * `5` files + `3` `.tsv` files).
  - Frames `0..=910` match 100% under C-NAT (`frame 0` with zero allowance, `1..=15` with `RAM_TEST_DIFF_RANGES`, and `16..=910` with 100% `sf_compare::compare` and CPU-owned work-RAM match including `0xff8852..0xff8862` and `0xffc680..0xffc684`).
  - The coverage ledger (`docs/reference/coverage-ledger.md`) shows all `180` instruction PCs in `0x48da4..0x490f6` executed and translated, bringing the repository total across all `137` scenarios to `10,916 / 10,916` executed PCs translated (`9,299` `:maincpu`, `1,394` `:audiocpu`, `223` `:audio2`, `0` untranslated).

---

## 8. Not Covered

- **`0x3b602` (`service_input` palette-upload repeat when `SYSTEM` bit 6 is low)**: unreachable under MAME `0.289`, which hardwires `SYSTEM` bit 6 high via `IPT_UNKNOWN` (`sf-mame0289.cpp:561`); recorded via `bus.note(0x3b602)`.
