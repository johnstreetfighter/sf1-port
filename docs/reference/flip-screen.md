# Flip Screen on Every Layer (Tickets #58 / F18, #139 / F67)

- **Label:** `DF / CF`, `PO / CF`
- **Tickets:** `#58` (`F18`), `#139` (`F67`)

## Verified Boundary

1. **`MachineState.flip_screen` Synchronization (`crates/sf-engine`):**
   - `Bus<'a>` carries `pub flip_screen: &'a mut bool` (`crates/sf-engine/src/fighter.rs:242, 267`).
   - `gfx_off` (`0x3b690`) and `gfx_on` (`0x3b644`) (`crates/sf-engine/src/tasks.rs:2151–2164`) update `*bus.flip_screen = v & 4 != 0` on every write to `0xc0001a` (`gfxctrl`), matching `sf_state::gfxctrl_w` (`sf-mame0289.cpp:352–355`: `m_active = data & 0xff; flip_screen_set(data & 0x04);`).
   - Fixture loaders (`CpuMachine::load` at `crates/sf-engine/src/cpu.rs:86–97`, `VersusMachine::from_checkpoint_2450_with_program` at `crates/sf-engine/src/versus.rs:85–97`, and `cpu_opponent_setup` / `timer_pass` at `crates/sf-engine/src/run.rs:806, 828`) initialize `flip_screen` from `work[0x0682] & 4 != 0`.

2. **Tilemap Flip Arithmetic Derived from `tilemap.cpp` (`tools/reference/local/mamesrc/mame0289-emu/tilemap.cpp`, `crates/sf-video/src/render.rs:84–188`):**
   - `driver_device::flip_screen_set(data & 0x04)` calls `tilemap_manager::set_flip_all(on ? (TILEMAP_FLIPX | TILEMAP_FLIPY) : 0)` (`tilemap.cpp:1732–1736`), setting `m_attributes = 0x03` on `m_bg_tilemap`, `m_fg_tilemap`, and `m_tx_tilemap`.
   - **Visible-area extents (`tilemap.cpp:1010–1012`, `sf-mame0289.cpp:773`):**
     - `screen.set_visarea(8*8, (64-8)*8-1, 2*8, 30*8-1)` sets `visarea = (64..=447, 16..=239)` (`384 × 224` inside `512 × 256`).
     - `xextent = visarea.right() + visarea.left() + 1 = 447 + 64 + 1 = 512`.
     - `yextent = visarea.bottom() + visarea.top() + 1 = 239 + 16 + 1 = 256`.
   - **Row and column scroll (`effective_rowscroll`, `tilemap.cpp:27–46`; `effective_colscroll`, `tilemap.cpp:55–74`; `draw_instance`, `tilemap.cpp:1289–1292`):**
     - With `m_dx = m_dx_flipped = m_dy = m_dy_flipped = 0`, `m_scrollrows = m_scrollcols = 1`, `m_rowscroll[0] = scroll`, and `m_colscroll[0] = 0`:
       - Unflipped (`m_attributes == 0`): `scrollx = (-scroll) mod m_width`, `scrolly = 0`.
       - Flipped (`TILEMAP_FLIPX | TILEMAP_FLIPY`): `scrollx = (xextent - m_width + scroll) mod m_width`, `scrolly = (yextent - m_height) mod m_height = 0` (since `m_height = 256 = yextent` for all three tilemaps).
     - `draw_instance` maps bitmap pixel `(bx, by) = (sx + 64, sy + 16)` to pixmap coordinates `(px_x, px_y) = ((bx + m_width - scrollx) % m_width, by)`.
     - For `m_tx_tilemap` (`64 × 32` of `8 × 8` tiles, `m_width = 512 = xextent`, `m_height = 256 = yextent`, `scroll = 0`), `scrollx = scrolly = 0` in both orientations.
   - **Logical-to-memory tile index mapping (`mappings_update`, `tilemap.cpp:716–742`):**
     - Under `TILEMAP_FLIPX | TILEMAP_FLIPY`, pixmap tile `(pcol, prow)` maps to logical `(col, row) = (m_cols - 1 - pcol, m_rows - 1 - prow)` (`(2047 - pcol, 15 - prow)` for BG/FG before `TILEMAP_SCAN_COLS`, and `(63 - tx, 31 - ty)` for TX).
   - **Per-tile flip flags (`tile_update`, `tilemap.cpp:806`; `tile_draw`, `tilemap.cpp:839–853`):**
     - `u32 flags = m_tileinfo.flags ^ (m_attributes & 0x03)` XORs `scene.flip_screen` into both `flipx` and `flipy` on every BG, FG, and TX tile.
   - **Sprite flip (`sf-mame0289.cpp:386–392, 437–443`, `crates/sf-video/src/render.rs:197–253`):**
     - Large `32 × 32` (`2 × 2` of `16 × 16`) sprites (`attr & 0x0400 != 0`): `sx = 480 - sx`, `sy = 224 - sy`, `flipx = !flipx`, `flipy = !flipy` (before swapping the `2 × 2` quad columns on `flipx` and rows on `flipy`).
     - Normal `16 × 16` sprites (`attr & 0x0400 == 0`): `sx = 496 - sx`, `sy = 240 - sy`, `flipx = !flipx`, `flipy = !flipy`.

3. **Reference Capture `sf-flip-01` (`C-REF`):**
   - **Scenario:** `tools/reference/scenarios/sf_flip_01.json` (`sf-fixed-fight-01` inputs with `Flip Screen = On`, `dsw1 = 0xdeff`, `dsw2 = 0xffff`, frames `0..=5040`, checkpoints at `790`, `1500`, and `2389..=5040`).
   - **Determinism:** `flip-01-run-a` and `flip-01-run-b` match byte-for-byte across all `13,272` observation files (`2,654` checkpoints × 5 files + `frames.tsv` + `meta.tsv`).
   - **RAM comparison against unflipped baseline/audit captures (`fight-run-a` at `790`, `1500`; `fight-audit-run-a` over `2389..=5040`):**
     - `video` RAM (`0x800000..0x800fff`), `palette` RAM (`0xffb800..0xffbfff`), and `object` RAM (`0xffe000..0xffffff`) are 100% byte-for-byte identical at all `2,654` checkpoints.
     - Outside stack memory (`0xff8106..=0xff8631`), the **only** differing byte in work RAM (`0xff8000..0xffdfff`) across all `2,654` checkpoints is `0xff8682` (`gfxctrl` copy), which has bit 2 (`0x04`) set in `sf-flip-01` (`0x0c` vs `0x08` at `790` and `1500`; `0xec` vs `0xe8` across `2389..=5040`).
     - Within stack memory (`0xff8106..=0xff8631`), 7 stack-saved register bytes (`0xff81fb`, `0xff824d`, `0xff8279`, `0xff82f9`, `0xff834d`, `0xff8379`, `0xff83f9`, where `D0` holding `0xff8682` after `0x3b644` / `0x3b690` is pushed by `movem.l`) also differ solely in bit 2 (`flip_byte ^ unflipped_byte == 0x04`).
     - In `ram_000790_music.bin`, 2 Z80 stack bytes (`0xc7f6..0xc7f7`: `44 00` vs `02 04`) differ because the 66-cycle `0x3c1b2..0x3c1bc` `gfx_on(4)` call in frame 6 shifts vblank 7's return PC inside `0x3c204`'s text-clear loop; all music RAM below the stack (`0xc000..0xc5cf`) at `790` and `1500` and the entire `ram_*_music.bin` across `2389..=5040` are byte-for-byte identical.

4. **Dump Rendering & Native Fight Verification (`C-NAT`, `C-NEG`, `crates/sf-engine/tests/flip_screen.rs`):**
   - **Dump rendering (Acceptance 2 & 4):** Checkpoints `790` (TX warning screen), `1500` (TX start screen), and `2389` (BG + FG + TX + sprites round-1 fixture) of `sf-flip-01` render byte-exact at `384 × 224` (`0 / 86,016` differing pixels), and all unflipped checkpoints (`790`, `1500`, `2280` of `fight-run-a` and `2389` of `fight-audit-run-a`) continue to match byte-exact.
   - **Native fight over `2390..=5014` (Acceptance 3):** `CpuMachine::from_fight_checkpoint` loads `state.flip_screen == true` (`gfxctrl == 0xec`) from `flip-01-run-a` checkpoint `2389` and matches `sf-flip-01` across all `2,625` frames `2390..=5014` (`sf_compare::compare` screen hash, sound latch, checkpoint pixels, video RAM, palette RAM, plus all owned work RAM and non-slot/non-stack work/object RAM).
   - **Negative control (Acceptance 5):** Rendering frame `2390` unflipped (`flip_screen = false`) is caught by `sf_compare::compare` at frame `2390` on `checkpoint screen pixels`.
   - **Power-on verification (`sf-flip-02`, Ticket #139 / F67, `crates/sf-engine/tests/dip_switches.rs`):** `sf-flip-02` (`tools/reference/scenarios/sf_flip_02.json`, `flip-02-run-{a,b}`, `17,423` identical observation files) runs `sf-fixed-fight-01`'s inputs with `Flip Screen = On` (`dsw1 = 0xdeff`, `dsw2 = 0xffff`) from power-on (`frame 0`) through `5040`, with full checkpoints at `0..=830`, `1500`, and `2389..=5040` (`3,484` checkpoints). `PowerOnMachine` matches all `5,041` frames and `3,484` checkpoints (`0..=5040`, including `5014`) and matches all owned work RAM at fixture frames `790`, `1500`, and `2389`. Running `sf-flip-02` natively with `Flip Screen = Off` (`dsw1 = 0xdfff`) matches frames `0..=7` (blank screen before the first warning character is drawn) and fails at frame `8` (`screen_rgba`, the exact first frame the flip shows).

## Not Covered

- `Difficulty`, `Round Time Count`, and `Attract Music` outside the fixed-fight windows in [fixed-fight-dips](fixed-fight-dips.md) (`F19`, `#59`), and the in-fight effects of `Speed` and `Freeze` outside their verified fixed-fight windows (`F20`, `F21`; [fixed-fight-slow](fixed-fight-slow.md), [freeze](freeze.md)).
