//! Tile graphics decoding for the original game's gfx ROM regions.
//!
//! Layouts transcribed from src/mame/capcom/sf.cpp @ mame0289:
//! `sprite_layout` (gfx1/gfx2/gfx3: 16x16, 4bpp) and `char_layout`
//! (gfx4: 8x8, 2bpp). Bit offsets follow MAME's convention: bit offset b
//! addresses bit (7 - b%8) of byte b/8 (MSB first).

/// Fetch one bit at a MAME gfx-layout bit offset.
fn bit(region: &[u8], offset: usize) -> u8 {
    (region[offset / 8] >> (7 - offset % 8)) & 1
}

/// Decode a 16x16 4bpp tile from a sprite_layout region (gfx1/2/3).
///
/// Plane bit offsets are {4, 0, half*8+4, half*8} where half is half the
/// region in bytes (RGN_FRAC(1,2)); x offsets are {0..3, 8..11, 256..259,
/// 264..267}; y stride is 16 bits. A tile spans 64 bytes within each half.
pub fn decode_sprite_tile(region: &[u8], tile_index: usize) -> [[u8; 16]; 16] {
    let half_bits = region.len() / 2 * 8;
    let plane_offsets = [4usize, 0, half_bits + 4, half_bits];
    let x_offsets: [usize; 16] = {
        let mut x = [0usize; 16];
        for (i, o) in x.iter_mut().enumerate() {
            // STEP4(0,1), STEP4(8,1), STEP4(256,1), STEP4(264,1)
            *o = [0, 8, 256, 264][i / 4] + i % 4;
        }
        x
    };
    let base = tile_index * 64 * 8; // tile size in bits, per plane-half
    let mut tile = [[0u8; 16]; 16];
    for (y, row) in tile.iter_mut().enumerate() {
        for (x, pen) in row.iter_mut().enumerate() {
            let mut value = 0u8;
            // MAME's gfx decode assigns planeoffset[0] to the MOST
            // significant pen bit (`planebit = 1 << (planes - 1)`,
            // shifting right per plane) — drawgfx.cpp @ mame0289.
            for (plane, plane_offset) in plane_offsets.iter().enumerate() {
                let plane_bit = plane_offsets.len() - 1 - plane;
                let offset = plane_offset + base + x_offsets[x] + y * 16;
                value |= bit(region, offset) << plane_bit;
            }
            *pen = value;
        }
    }
    tile
}

/// Decode an 8x8 2bpp tile from the char_layout region (gfx4).
///
/// Plane bit offsets {4, 0} (pen bits 0..1), x offsets {0..3, 8..11},
/// y stride 16 bits; 16 bytes per tile.
pub fn decode_char_tile(region: &[u8], tile_index: usize) -> [[u8; 8]; 8] {
    let plane_offsets = [4usize, 0];
    let x_offsets = [0usize, 1, 2, 3, 8, 9, 10, 11];
    let base = tile_index * 16 * 8;
    let mut tile = [[0u8; 8]; 8];
    for (y, row) in tile.iter_mut().enumerate() {
        for (x, pen) in row.iter_mut().enumerate() {
            let mut value = 0u8;
            // MSB-first plane assignment, as in decode_sprite_tile.
            for (plane, plane_offset) in plane_offsets.iter().enumerate() {
                let plane_bit = plane_offsets.len() - 1 - plane;
                let offset = plane_offset + base + x_offsets[x] + y * 16;
                value |= bit(region, offset) << plane_bit;
            }
            *pen = value;
        }
    }
    tile
}
