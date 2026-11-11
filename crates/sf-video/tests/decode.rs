//! Tile graphics decoding, hand-verified against the gfx layouts in
//! src/mame/capcom/sf.cpp @ mame0289 and the gfx_element::decode bit
//! order in src/emu/drawgfx.cpp @ mame0289: planeoffset[0] carries the
//! MOST significant pen bit (`planebit = 1 << (planes - 1)`).

use sf_video::decode::{decode_char_tile, decode_sprite_tile};

/// sprite_layout: 16x16, 4bpp, 64 bytes per tile; plane bit offsets
/// {4, 0, half*8+4, half*8} mapped MSB-first (pen bits 3..0), x bit
/// offsets {0..3, 8..11, 256..259, 264..267}, y stride 16 bits.
#[test]
fn sprite_tile_pen_bits_come_from_the_four_plane_offsets() {
    // One 64-byte tile needs 64 bytes per plane half -> 128-byte region.
    let half = 64; // RGN_FRAC(1,2) in bytes
    let mut region = vec![0u8; 128];

    // Tile 0, pixel (0,0): set all four plane bits -> pen 0b1111 = 15.
    // bit3 at bit offset 4 (byte 0, 0x08); bit2 at offset 0 (byte 0, 0x80);
    // bit1 at half*8+4 (byte `half`, 0x08); bit0 at half*8 (byte `half`, 0x80).
    region[0] = 0x88;
    region[half] = 0x88;

    // Tile 0, pixel (1,0): only bit0 -> pen 1. x=1 adds 1 to each offset:
    // bit0 at byte `half`, 0x40.
    region[half] |= 0x40;

    // Tile 0, pixel (0,1): bit1 only -> pen 2. y=1 adds 16 bits = 2 bytes:
    // bit1 at byte `half` + 2, 0x08.
    region[half + 2] = 0x08;

    // Tile 0, pixel (4,0): x=4 uses the second x-offset group (8..11):
    // bit2 at bit offset 8 -> byte 1, 0x80 -> pen 4.
    region[1] = 0x80;

    // Tile 0, pixel (8,0): x=8 uses the third x-offset group (256..259):
    // bit2 at bit offset 256 -> byte 32, 0x80 -> pen 4; bit3 at 256+4 ->
    // byte 32, 0x08 -> pen 8. Both set -> pen 12.
    region[32] = 0x88;

    let tile = decode_sprite_tile(&region, 0);
    assert_eq!(tile[0][0], 15);
    assert_eq!(tile[0][1], 1);
    assert_eq!(tile[1][0], 2);
    assert_eq!(tile[0][4], 4);
    assert_eq!(tile[0][8], 12);
    assert_eq!(tile[15][15], 0);
}

/// The same layout, addressed at tile index 1: each 16x16 tile spans
/// 64 bytes, so tile 1 starts at byte 64, and the plane 2/3 halves move
/// with the region fraction, not the tile.
#[test]
fn sprite_tile_indexing_offsets_into_the_plane_halves() {
    // Two 64-byte tiles need 128 bytes per plane half -> 256-byte region.
    let half = 128;
    let mut region = vec![0u8; 256];
    // Tile 1, pixel (0,0), bit3: tile base 64 + bit offset 4 -> 0x08.
    region[64] = 0x08;
    // Tile 1, pixel (0,0), bit1: half + 64, 0x08.
    region[half + 64] = 0x08;

    let tile = decode_sprite_tile(&region, 1);
    assert_eq!(tile[0][0], 10);
}

/// char_layout: 8x8, 2bpp, 16 bytes per tile; plane bit offsets {4, 0}
/// mapped MSB-first (pen bits 1..0), x offsets {0..3, 8..11}, y stride 16.
#[test]
fn char_tile_pen_bits_come_from_two_planes() {
    let mut region = vec![0u8; 16];
    // Pixel (0,0): bit1 at offset 4 (0x08), bit0 at offset 0 (0x80) -> pen 3.
    region[0] = 0x88;
    // Pixel (4,0): x=4 -> bit offsets 8..11; bit0 at byte 1, 0x80 -> pen 1.
    region[1] = 0x80;
    // Pixel (0,1): y stride 16 bits = 2 bytes; bit1 at byte 2, 0x08 -> pen 2.
    region[2] = 0x08;

    let tile = decode_char_tile(&region, 0);
    assert_eq!(tile[0][0], 3);
    assert_eq!(tile[0][4], 1);
    assert_eq!(tile[1][0], 2);
    assert_eq!(tile[0][1], 0);
}
