# First frame: reconstructing the fixed fight's opening frame

This document records the boundary established by
issue #4: the fixed fight's
initial game frame is reproduced from decoded original graphics data plus
documented scene state, compared byte-exactly against the pinned reference
capture at game resolution (384x224) before display scaling. The displayed
scene is reconstructed; no reference screenshot is used as renderer output.

Everything below is pinned to `src/mame/capcom/sf.cpp` and
`src/emu/drawgfx.cpp` at MAME tag `mame0289`, and validated against the
`fight-run-a` capture (local-only reference data).

## Original data addresses

From the `sfus_map` address map (the `sf` set uses `sf_map`, identical in
the relevant ranges):

| Data | Address range | Form |
| --- | --- | --- |
| video RAM (text tilemap) | 0x800000-0x800fff | 2048 big-endian u16 words |
| palette RAM | 0xb00000-0xb007ff | 1024 big-endian u16, xRGB_444 |
| work RAM | 0xff8000-0xffdfff | 24 KiB |
| object RAM (sprites) | 0xffe000-0xffffff | 8 KiB, big-endian u16 words; the tail of the capture's 32 KiB `work` dump at bytes 0x6000..0x8000 |
| fg scroll (write-only) | 0xc00014-0xc00015 | u16 |
| bg scroll (write-only) | 0xc00018-0xc00019 | u16 |
| gfxctrl (write-only) | 0xc0001a-0xc0001b | low byte: bit2 flip screen, bit3 tx, bit5 bg, bit6 fg, bit7 sprites |

The three write-only registers cannot be read back from RAM dumps. They
were recovered with a write-tap probe of the reference run
(`tools/reference/local/probe_regs.lua`, local-only; tap handles must be
stored in Lua globals or they are garbage-collected and silently removed).
Per frame N, the effective values are the last writes logged at or before
frame N:

| Frame | gfxctrl low byte | fg_scroll | bg_scroll | Scene |
| --- | --- | --- | --- | --- |
| 790 | 0x08 | - | - | warning screen (tx only) |
| 794 | 0x00 | 0x0000 | 0x0000 | attract entry: all layers disabled (`0xff8682 = 0x00`) while copyright and `INSERT COIN` are in video RAM |
| 1500 | 0x08 | - | - | title screen, logo and text are all tx layer |
| 2280 | 0xe8 | 0x1d40 | 0x1800 | round-1 fight start: tx + bg + fg + sprites |

## Renderer conventions (validated byte-exactly)

- **Bit order inside gfx layouts** (drawgfx.cpp `gfx_element::decode`):
  `planeoffset[0]` carries the MOST significant pen bit; the bit position
  is `1 << (planes - 1)` for the first listed plane, shifting right per
  plane. A bit offset `b` addresses bit `7 - b%8` of byte `b/8`.
- **Palette granularity** is `1 << planes` per color code: the 4bpp layers
  (gfx1/2/3) stride the palette by 16 pens per color, the 2bpp text layer
  (gfx4) by 4. GFXDECODE color bases: gfx1=0, gfx2=256, gfx3=512, gfx4=768.
  Palette entry 0 is the backdrop when every layer is disabled.
- **Visible area**: the screen bitmap is 512x256
  (`screen.set_size(64*8, 32*8)`) and the visible area starts at bitmap
  pixel (64, 16) (`screen.set_visarea(8*8, (64-8)*8-1, 2*8, 30*8-1)`).
  Screen pixel (x, y) is bitmap pixel (x+64, y+16). Tilemaps and sprite
  coordinates live in bitmap space.
- **bg/fg tilemaps** (`get_bg_tile_info`/`get_fg_tile_info`): 2048x16
  tiles of 16px, column-major index `col*16+row`. bg tilerom base 0, fg
  base 0x20000; at `base+2*i`: color `base[0]` (low nibble used), code
  `base[0x10001]<<8 | base[1]`, flip `base[0x10000] & 3`. Codes are reduced
  modulo the tile count of the region (region_bytes/128). fg pen 15 is
  transparent. Horizontal scroll wraps within the tilemap's 32768 px width;
  tilemap_x = screen_x + 64 + scroll.
- **tx tilemap** (`get_tx_tile_info`): 64x32 tiles of 8px, row-major over
  video-RAM words; code `word & 0x3ff`, color `word >> 12`, flip
  `(word >> 10) & 3`; pen 3 transparent.
- **Sprites** (`draw_sprites`): object-RAM entries of 0x20 words iterated
  from offset 0xfe0 down to 0; per entry `c/attr/sy/sx` at words
  `offs..offs+3`; color `attr & 0xf`, flipx 0x100, flipy 0x200, large
  sprite 0x400 (2x2 tiles `c, c+1, c+16, c+17`, swapped per flip). Every
  code passes through the driver's `invert()`:
  `c ^ {0x00,0x18,0x18,0x00}[(c>>3)&3]`. Pen 15 transparent. Sprite
  coordinates are bitmap coordinates (subtract the visible-area origin).
- **Layer order** (`screen_update`): bg, fg, sprites, tx.

## Fixture-only initialization

The `SceneState` consumed by `sf_video::render::render_frame` is a
reconstruction, not yet an emulated machine state: video RAM, object RAM,
and palette RAM come from the capture's checkpoint dumps, and the three
write-only registers come from the register-tap table above. Later tickets
replace these with live translation state; the fixture values stay as the
regression baseline.

Checkpoint 794 was initially noted as a transition frame because its pixel
snapshot is black while its video-RAM dump already holds the copyright and
`INSERT COIN` tiles. Re-checking `0x3ccae–0x3cce6` and `ram_000794_work.bin`
(ticket #43, [attract](attract.md)) showed that checkpoint 794 is a
**consistent state**: in frame 794, `0x3ccc2` disables all layers
(`0xff8682 = 0x00`) before slot 3 writes the attract text to video RAM, so
rendering checkpoint 794 with `gfxctrl = 0x00` reproduces `pixels_000794.rgba`
byte-for-byte (`86,016 / 86,016` pixels). Checkpoint 790 sits inside the
240-frame warning-screen hold (`0x3c254 -> 0x3b57a` over frames `554..=793`,
after the four warning blocks finish typing at frame `540` and pause 12 frames
through `553`, with checkpoint `pixels` constant over `540..=793` and
`screen_hash` constant over `541..=794`; reconciled with
`sf_fixed_fight_01.json`'s earlier `"790=static black window"` note by ticket
#55, [power-on](power-on.md)), and checkpoint 794 is the attract-entry join.

## Byte-exact fixtures

Three checkpoints of the fixed-fight capture reproduce the reference pixels
exactly (86016/86016 pixels), each as a `cargo test -p sf-video` fixture
(skipped on a clean checkout, since ROMs and captures stay local):

- frame 790 (warning screen) — text layer only
- frame 1500 (title screen) — text layer only; the title logo is drawn by
  the text layer, not by bg/fg
- frame 2280 (round-1 fight start) — all four layers, sprites, scrolls

## Windows launch

`crates/sf-dev` opens a native window showing the reconstructed fight-start
frame (checkpoint 2280), upscaled 3x with nearest neighbour — the
acceptance comparison happens at game resolution before this scaling:

```
cd crates/sf-dev
cargo run
```

Esc or the window close button exits cleanly. This static scene is a
reconstruction demo; it does not count as a playable fight.
