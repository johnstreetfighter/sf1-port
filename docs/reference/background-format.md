# The 1987 background format

This document records what issue #194
(`ST4`) mapped: the layout of the background tilemaps in `tilerom`, how the
foreground and background scroll registers address them and over what
range, the stage palettes, and where a map past the ROM's 2048 columns can
live. It is the groundwork for the extended background bank
([ADR-0009](../adr/0009-stages-and-themes-in-extended-banks.md), #198) and
the composite VS screens built in it (#215). The evidence is the pinned
MAME driver (`sf.cpp`, `tilemap.h` and `tilemap.cpp` @ mame0289), the
translated code in `crates/sf-engine/src/tasks.rs` and `fighter.rs`, the
renderer (`crates/sf-video/src/render.rs`), and three ROM-gated tests in
`crates/sf-engine/tests/background_format.rs`:

- `rom_map_layout` checks every ROM fact below that has a number in it;
- `no_committed_scenario_scrolls_or_reads_past_the_rom_map` is the sweep;
- `the_window_is_what_the_renderer_reads` checks that the columns and rows
  the sweep counts are exactly the cells the renderer fetches.

## 1. Two layers, four ROMs

The board has two scroll layers drawn from ROM maps, the background (`bg`,
layer `gfx1`, opaque, drawn first) and the foreground (`fg`, `gfx2`, pen 15
transparent, drawn over it). Both are `TILEMAP_SCAN_COLS` maps of 2048
columns by 16 rows of 16x16 tiles (`sf.cpp:268-269`), 32,768 pixels across
and 256 down. `tilerom` (0x40000 bytes) holds both, one 64 KiB ROM per byte
pair:

| `tilerom` | ROM | layer | even byte | odd byte |
|---|---|---|---|---|
| `0x00000-0x0ffff` | `sf-37.4h` | bg | colour | tile code, low byte |
| `0x10000-0x1ffff` | `sf-36.3h` | bg | attribute | tile code, high byte |
| `0x20000-0x2ffff` | `sf-32.3g` | fg | colour | tile code, low byte |
| `0x30000-0x3ffff` | `sf-33.4g` | fg | attribute | tile code, high byte |

So the "two halves" of `tilerom` are the two layers (`0x00000` bg,
`0x20000` fg), and each half is split again into a colour/code-low ROM and
an attribute/code-high ROM `0x10000` above it. The cell at (column, row) is
entry `col * 16 + row` of its layer, at byte `2 * (col * 16 + row)` in both
of the layer's ROMs (`get_bg_tile_info` / `get_fg_tile_info`,
`sf.cpp:242-258`):

- **code** = high-ROM odd byte << 8 | low-ROM odd byte: 16 bits.
- **colour** = low-ROM even byte: which of the layer's 16 palette rows.
- **attribute** = high-ROM even byte: bit 0 flips the tile in x, bit 1 in
  y (`TILE_FLIPYX(attr & 3)`); flip screen inverts both.

MAME reduces the code modulo the layer's tile count and the colour modulo
16 (`tilemap.h:391-393`); `render.rs` does the same.

**There is no room past column 2048 in these ROMs.** 32,768 cells of two
bytes fill each 64 KiB ROM exactly; column 2048's address would be the
first byte of the next ROM.

### What the ROM maps hold (`rom_map_layout`)

| | bg | fg |
|---|---|---|
| tiles in the layer's region | `0x1000` (`gfx1`, 512 KiB) | `0x2000` (`gfx2`, 1 MiB) |
| programmed columns | 0..=1955 | 0..=1107 |
| unprogrammed tail (all four bytes `0xff` in every row) | 1956..=2047 (92 columns) | 1108..=2047 (940 columns) |
| highest code in a programmed cell | `0x0f7f` | `0x1a9c` |
| colours used | 0..=13 | 0..=15 |
| attribute bits used | bit 0 (x flip) | bits 0 and 1 |

No programmed cell uses a code at or past its layer's tile count, a colour
byte above 15, or an attribute bit above 1. Each map's last programmed
column is the last column a ROM table can show: bg column 1955 is the right
edge of the Player 2 ending's screen (`0x44d5e`: scroll `0x7880`), fg column
1107 the right edge of the four-country flag screen (scroll `0x4380`, set at
`0x3e1e4`).

The tiles are `sprite_layout` 16x16 tiles of 128 bytes, split as the sprite
region is (`RGN_FRAC(1,2)`: planes 0-1 in the first half, planes 2-3 in the
second, 64 bytes of each tile in each half; `sf.cpp` `GFXDECODE`). A
16-bit code addresses 65,536 tiles, so tiles above the ROM's (bg `0x1000`
up, fg `0x2000` up) need no change to the cell format. They go after the
ROM's tiles in both halves of the region, exactly as the extended sprite
bank does (`crates/sf-video/src/sprite_bank.rs`, ADR-0007).

## 2. The scroll registers

| register | layer | work-RAM copy | written by |
|---|---|---|---|
| `0xc00014` (`fg_scroll_w`) | fg | `0xffc680` | `0x3b674` (copy += d, register written), the camera (`0x43a58` on) |
| `0xc00018` (`bg_scroll_w`) | bg | `0xffc682` | `0x3b682` (copy += d, register written), the stage drift (`0x438ea`) |

Both are write-only, 16 bits. There is one horizontal scroll value per
layer and no vertical scroll. The layers are enabled by `gfxctrl`
(`0xc0001a`) bits 5 (bg) and 6 (fg); a disabled layer fetches nothing
(`sf.cpp:458-463`).

**How a value addresses the map.** The visible area starts 64 pixels into
the 512x256 screen bitmap (`set_visarea(64, 447, 16, 239)`), so with scroll
`s` the screen shows map pixels `s + 64 ..= s + 447`, taken modulo the map's
32,768 pixels (`tilemap.cpp:27-46`, `effective_rowscroll`; `render.rs`).
Flip screen mirrors the same window in place: it shows the same columns, in
reverse. Vertically the screen shows bitmap lines 16..=239, which are map
rows **1..=14**; rows 0 and 15 are never on screen, whatever the scroll or
flip. A window is 24 columns when `s + 64` is a multiple of 16, 25 when it
is not.

So **MAME ignores bit 15**: `0x8000 + s` shows what `s` shows. Whether the
board's scroll counters have 15 or 16 bits is not known (section 6).

### The range the game uses (the sweep)

`no_committed_scenario_scrolls_or_reads_past_the_rom_map` runs every
committed scenario (137, 874,344 frames) from power-on through the native
engine and records, after every frame, both registers, whether each layer
is enabled, flip screen, and every map column an enabled layer shows. It
only reads the machine; it changes nothing.

| | bg | fg |
|---|---|---|
| frames shown (of which flipped) | 562,132 (6,644) | 555,212 (7,214) |
| register range while shown | `0x0000..=0x7700` | `0x0000..=0x4380` |
| highest value ever held (shown or not) | `0x7700` | `0x4380` |
| columns shown | 1,291, all in 4..=1931 | 1,019, all in 4..=1107 |
| highest code / colour / attribute bits shown | `0x0f7f` / 13 / `0x01` | `0x1a9c` / 15 / `0x03` |

Outside the service mode's scroll test, then, **the 1987 game never sets
bit 15 of either register, never shows a window that reaches past column
2047 (so it never wraps), and never shows an unprogrammed cell, a code at or
past its layer's tile count, or a colour above 15.** Every palette it
requests is one of the 0x200-byte tables in `0x1a268..0x1c068`.

### The one exception: the service SCROLL TEST

Service-mode item 6 (`0x48da4..=0x490f6`, `docs/reference/service-mode.md`
section 7) lets the operator step each register by `0x20` or `0x40` with no
bound, and its own palette lookup has bounds running to `0xffff`. In
`sf_service_03b` (845 frames in the test) both registers reach `0xfd00..=0xffc0`:
244 frames per layer show windows past column 2047, which under MAME's
32,768-pixel map wrap onto the unprogrammed tail (columns 2004..=2047,
drawn as tile `0x0fff` / `0x1fff` in colour 15) and column 0. The sweep
separates the scroll test's frames by the task position and asserts that it
does this. The operator can reach every register value here; nothing else
in the game can.

## 3. Where the stages sit on the map

A fight stage is a foreground and a background scroll position.
`stage_reset` (`0x437d8`) reads them from two tables chosen by the starting
country (`0xffc006`): fg from `0x437ee + country * 4` plus `0xc0`, bg from
`0x437fe + country * 4`, both by opponent index (stage `0xffc003` * 2 +
fight `0xffc004`). Each country's tables are the same ten words in another
order. Japan's, by opponent Retsu, Geki, Joe, Mike, Lee, Gen, Birdie,
Eagle, Adon, Sagat:

- fg: `1c80 1f80 2400 2700 2b80 2e80 3300 3600 3a80 3d80`
- bg: `1800 2180 2b00 3480 3e00 4780 5100 5a80 6400 6d80`

**A stage's footprint.** The foreground moves with the camera, which stays
within `0..=0x180` (`0x43a58`), so a fight shows fg pixels from the table
word + 64 to the word + `0x180` + 447: **48 columns**. The background moves
only with the drift (`0x438ea`): one pixel every fifth frame from 0 to
`0x200`, then a jump back `0x200` pixels to 0, so a fight shows bg pixels
from the word + 64 to the word + `0x200` + 447: **56 columns**. The jump
never shows because each stage's first 24 bg columns equal the 24 that
start 32 columns on, row for row; that is a 1987 background rule a new
stage must keep. (No committed scenario holds a fight on one stage long
enough for the drift to reach `0x200`, so the last columns of several
stages' backgrounds, Retsu's 438..=443 for one, are never shown in the
sweep; the table arithmetic covers them, and `rom_map_layout` checks the
rule over them.)

**The foreground**, per country (`0x780` pixels, 120 columns, from
`0x1b00`): the country intro screen (`0x3f8fa` table, 24 columns: Japan
`0x1b00`, USA `0x2280`, China `0x2a00`, England `0x3180`, Thailand
`0x3900`), then its two fights' stages, 48 columns each, back to back.

**The background**, per opponent (`0x980` pixels, 152 columns, from
`0x1500 + 0x980 * opponent`):

| offset | width | what | table |
|---|---|---|---|
| `+0x000` | 24 | VS screen, game mode 2 (Ryu left, opponent right) | `0x3fa66` |
| `+0x180` | 24 | VS screen, game mode 4 (opponent left, Ken right) | `0x3fad6` |
| `+0x300` | 56 | the stage's background | `0x437fe` |
| `+0x680` | 24 | win card, the human wins | `0x4018a` |
| `+0x800` | 24 | win card, the CPU wins | `0x40250` |

The rest of each map (fg columns 4..=435 and 1036..=1107, bg columns
4..=339 and 1860..=1955) holds the attract, title, flag, bonus-stage, name
entry, continue, game over, ranking, ending and service screens, each a
scroll constant or table in `tasks.rs`. The versus VS screen is bg `0x780`
(columns 124..=147) with palette `0x1ae68`; the versus win cards are
`0x0a80` and `0x0900` (`0x400d8`).

## 4. The palettes

Palette RAM (`0xb00000..0xb007ff`) is 1024 xRGB_444 words: the bg layer's
16 colours of 16 pens at entries 0..=255, the fg's at 256..=511, the
sprites' at 512..=767 and the text layer's (16 colours of 4 pens) at
768..=1023 (`GFXDECODE` colour bases 0, 256, 512, 768). Every load the game
makes is 0x200 bytes, one whole layer's palette, copied from the program
ROM: either by the vblank's palette request (`0x3b362`; flag `0xff8633`,
source `0xff8634`, palette offset `0xff8638`) or directly (`0x3b5f8`).

A stage's palettes are chosen by stage (`0xffc003`) and country, not by
fight, so a country's two fights share them:

| stage | bg palette (offset `0x000`) | fg palette (offset `0x200`) |
|---|---|---|
| Japan | `0x1b068` | `0x1a468` |
| USA | `0x1b268` | `0x1a668` |
| China | `0x1b468` | `0x1a868` |
| England | `0x1b668` | `0x1aa68` |
| Thailand | `0x1b868` | `0x1ac68` |

The bg palette is requested by `request_stage_palette` (`0x43748`, table
`0x43778 + country * 4`, then `stage * 4`), the fg palette by
`request_country_palette` (`0x436b8`, table `0x436e8`, same indexing). The
bg palette serves the stage's two backgrounds, both opponents' VS screens
and both win cards, so they share its 16 rows: in Japan's, Retsu's stage
background uses rows 0-1, Geki's 0 and 8, and the VS busts rows 2, 3, 4 and
6. The fg palette is split between the two fights' foregrounds (Retsu's
uses rows 0-5, Geki's 0 and 9-15; row 0 is in every stage). The other
tables the game requests: fg `0x1a268` (travel map, service, bonus stages,
continue, game over), bg `0x1ae68` (versus VS screen and win cards, the
continue screen, the scroll test, bonus stages), text `0x1bc68` and
`0x1be68` at `0x600`. `0x1b868` is also the ranking and ending palette.

## 5. Where a map past column 2048 can live

The ROM's maps are full, so an extended map is new data beside `tilerom`,
not in it. The natural layout keeps the ROM's: **an extended page is
another 0x40000-byte block with the same four-ROM layout**, so map column
`page * 2048 + c` of a layer reads its cells from
`0x40000 * page + layer_base + 2 * (c * 16 + row)` and `0x10000` above, and
page 0 is the ROM. Cells keep the 1987 format (16-bit code, colour,
flips), and the bank's tiles sit above the ROM's tiles (section 1).

How the scroll registers reach a page is ST5's choice. What the sweep
settles:

1. **Bit 15 of the register as the page bit.** A 16-bit register addresses
   65,536 pixels, which is 4096 columns: the ROM page and exactly one more.
   Outside the scroll test the 1987 game never sets bit 15 and never shows
   a window that crosses pixel `0x8000`, so with a 4096-column map every
   frame it draws is unchanged. Two limits:
   - the service scroll test does set bit 15 and does wrap, so it would
     show the extended page where MAME shows the unprogrammed tail and
     column 0. That is a change only if the bank is loaded under the
     service mode; the extended sprite bank's precedent loads banks only
     for overhaul fights (ADR-0007, `crates/sf-engine/tests/extended_sprite_bank.rs`).
   - one extra page is 2048 columns per layer. 38 stages need 38 x 48 =
     1,824 fg columns, which fits, but 38 x 56 = 2,128 bg columns, which does
     not, before any VS screen.
2. **A page selector beside the register**, set by the overhaul gate with
   the stage (renderer state, never a work-RAM write, as ADR-0004 requires).
   The register keeps MAME's 15-bit meaning everywhere, the scroll test is
   untouched, and there is no limit on pages. A stage's scroll positions
   are then ordinary values below `0x8000` within its page, and the 1987
   rule to keep is that no window crosses the page's pixel `0x8000`: fg
   word + `0x180` + 447 and bg word + `0x200` + 447 stay below `0x8000`.
   A built VS screen (#215) can even sit at bg `0x780` of its page, the
   scroll the versus branch already writes.

Capacity, for either: a stage needs 48 fg and 56 bg columns (section 3),
and a VS screen 24 bg columns with the blank cell (code 0, colour 0, pen 15
on every pen) around its busts. A 2048-column page holds 42 stages'
foregrounds and 36 stages' backgrounds, or 85 VS screens.

The ROM's unprogrammed tails (bg columns 1956..=2047, fg 1108..=2047) are
inside column 2048 and are never shown by the game outside the scroll
test, so they are spare space too; they are small for backgrounds (92
columns) and the scroll test shows them, so they are noted, not proposed.

**Palettes.** A stage needs its own bg and fg palettes of 0x200 bytes. The
1987 game only ever requests pointers inside `0x1a268..0x1c068`; the main
CPU's program ROM is mapped at `0x000000-0x04ffff` (`sfus_map`), so an
extended palette can be named by a pointer the game never uses, served to
`0x3b362` by the overhaul gate. The bg map never uses colour rows 14 and
15.

### As built (ST5, #198)

ST5 took option 2 (ADR-0009, second amendment). `sf_video::background_bank`
holds the bank: tiles above the ROM's in both halves of `gfx1` and `gfx2`,
pages after `tilerom`, and palettes named by pointers from `0x400000`. The
scene carries a page per layer (`bg_page`, `fg_page`) that only the stage
gate in `stage_reset` sets. Any add to a cleared scroll copy, which is how
the 1987 code positions every screen, puts the layer back on page 0, so the
camera and the drift keep the page and the next 1987 screen leaves it. The
palette gates in `request_stage_palette` and `request_country_palette` and
the music gate in `stage_music` read the same stage. `sf_video::stage`
packs a stage only if it keeps the rules of sections 1-3: 48 and 56
columns, the drift repeat, whole-column words, windows inside the page, no
cells over the page's, tiles the layer holds, colour rows its palette fills,
and only the flip bits. `extended_background_bank.rs` (sf-engine) checks
that with the bank loaded every committed scenario is byte for byte
unchanged.

### As built (#215)

`sf_video::vs_screens` builds the two-player VS screens at load: every
pair of the twelve but Ryu vs Ken (143), 85 to a page on the two pages
after the stages', each 24 columns from column 8 + 24k (scroll
`0x40 + 0x180 * k`), with its own palette. A screen is the left fighter's
bust block and name from a ROM screen where he stands on the left and the
right fighter's from one where he stands on the right; Ryu on the right
and Ken on the left are mirrored with the x-flip bit. A colour row the
right fighter shares with the left in other colours (Retsu and Mike, both
row 4) moves to the lowest free row. The gate `picked_vs` in the versus
branch (`0x3fa00`) scrolls to the pair's view, sets the page and names its
palette in Local and Training under an overhaul preset;
`crates/sf-engine/tests/vs_screens.rs` checks every pair's pixels against
the ROM screens and the session's Local and Training VS screens.

## 6. What is still unknown

- **The board's scroll counter width.** MAME takes the scroll modulo 32,768
  pixels, so bit 15 does nothing there; no schematic or real-board capture
  was consulted to say whether the board ignores it too. It only matters to
  the scroll test, the one place bit 15 is set.
- **The scroll test's pixels past column 2047.** The sweep shows where the
  scroll test scrolls, but those frames' pixels (the unprogrammed tail,
  drawn as tile `0x0fff` / `0x1fff` in colour 15 by MAME's modulo rules)
  were not compared with a MAME screenshot here.
- **Mid-frame scroll writes.** The sweep reads the registers after each
  frame, which is what the renderer and MAME's single end-of-frame screen
  update draw. Whether the board latches a scroll write made during the
  visible frame (a raster effect) is not known; no 1987 code was found
  relying on one.
- **Paths no scenario reaches.** The sweep covers what the committed
  scenarios run. Table values no scenario shows were checked by reading the
  tables instead: the Player 2 ending's `0x7880`, the drift's last columns, and
  the other countries' table words (the same ten words as Japan's). Scroll
  constants in code that no scenario reaches were not each traced.
- **The ROM tiles the maps never place** (bg codes `0x0f80..=0x0fff`, fg
  `0x1a9d..=0x1fff`) were not inspected; the bank's tiles go above the
  ROM's in any case.
