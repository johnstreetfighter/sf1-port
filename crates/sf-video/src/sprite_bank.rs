//! The extended sprite bank (ADR-0007): new sprite tiles above the ROM's
//! own 14,336, which only overhaul modes load. The renderer draws them like
//! any ROM tile.
//!
//! The sprite region follows MAME's `sprite_layout`: it is split in two
//! halves, the first holding planes 0 and 1 of every tile and the second
//! planes 2 and 3, 64 bytes of each tile in each half. So the bank's tiles
//! go after the ROM's in *both* halves, and every ROM tile keeps its place.
//! The renderer fetches tile `code` from slot [`sprite_slot`]`(code)`, so
//! the bank stores each tile in the slot the renderer will read.
//!
//! No 1987 scenario draws a tile code at or past `0x3800`: the highest is
//! `0x33bf` over all 137 committed scenarios (ticket #165, `AP1`), so the
//! bank changes nothing the game already draws.

use crate::romset::Assets;

/// The sprite tiles the ROM holds (`gfx3`, 1,835,008 bytes).
pub const ROM_SPRITE_TILES: usize = 0x3800;
/// Bytes of one tile in each half of the region.
const HALF_TILE: usize = 64;

/// The tile slot the renderer reads for sprite tile `code`: bits 3 and 4
/// of the code swapped in two of every four 8-tile groups (`sf.cpp`'s
/// sprite code inversion). It maps each 32-tile group onto itself.
pub fn sprite_slot(code: u16) -> usize {
    crate::render::invert(code) as usize
}

/// Encode a 16×16 tile of pens (0..=15, 15 transparent for sprites) into
/// its two 64-byte halves, the inverse of
/// [`decode_sprite_tile`](crate::decode::decode_sprite_tile).
pub fn encode_sprite_tile(pens: &[[u8; 16]; 16]) -> [[u8; HALF_TILE]; 2] {
    let mut halves = [[0u8; HALF_TILE]; 2];
    for (y, row) in pens.iter().enumerate() {
        for (x, &pen) in row.iter().enumerate() {
            let x_offset = [0, 8, 256, 264][x / 4] + x % 4;
            // Plane 0 is the pen's most significant bit (offset 4 in the
            // first half), plane 3 its least (offset 0 in the second).
            for plane in 0..4 {
                if (pen >> (3 - plane)) & 1 == 0 {
                    continue;
                }
                let offset = [4, 0, 4, 0][plane] + x_offset + y * 16;
                halves[plane / 2][offset / 8] |= 0x80 >> (offset % 8);
            }
        }
    }
    halves
}

/// Tiles above the ROM's, by the slot the renderer reads.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SpriteBank {
    first: Vec<u8>,
    second: Vec<u8>,
}

impl SpriteBank {
    pub fn new() -> Self {
        SpriteBank::default()
    }

    /// Store `pens` as sprite tile `code`. Panics if `code` falls among the
    /// ROM's tiles: the bank never replaces one.
    pub fn put(&mut self, code: u16, pens: &[[u8; 16]; 16]) {
        let slot = sprite_slot(code);
        assert!(slot >= ROM_SPRITE_TILES, "tile code {code:#06x} belongs to the ROM");
        let at = (slot - ROM_SPRITE_TILES) * HALF_TILE;
        if self.first.len() < at + HALF_TILE {
            self.first.resize(at + HALF_TILE, 0);
            self.second.resize(at + HALF_TILE, 0);
        }
        let [first, second] = encode_sprite_tile(pens);
        self.first[at..at + HALF_TILE].copy_from_slice(&first);
        self.second[at..at + HALF_TILE].copy_from_slice(&second);
    }

    /// Slots the bank spans above the ROM's tiles (gaps included).
    pub fn tiles(&self) -> usize {
        self.first.len() / HALF_TILE
    }

    pub fn is_empty(&self) -> bool {
        self.first.is_empty()
    }
}

impl Assets {
    /// These graphics with `bank` above the ROM's sprite tiles, for an
    /// overhaul session. Every other region is a copy.
    pub fn with_sprite_bank(&self, bank: &SpriteBank) -> Assets {
        let half = self.gfx3.len() / 2;
        assert_eq!(half, ROM_SPRITE_TILES * HALF_TILE, "the ROM's sprite region");
        let mut gfx3 = Vec::with_capacity(self.gfx3.len() + 2 * bank.first.len());
        gfx3.extend_from_slice(&self.gfx3[..half]);
        gfx3.extend_from_slice(&bank.first);
        gfx3.extend_from_slice(&self.gfx3[half..]);
        gfx3.extend_from_slice(&bank.second);
        Assets {
            maincpu: self.maincpu.clone(),
            gfx1: self.gfx1.clone(),
            gfx2: self.gfx2.clone(),
            gfx3,
            gfx4: self.gfx4.clone(),
            tilerom: self.tilerom.clone(),
        }
    }

    /// Sprite tiles these graphics hold, the ROM's and any bank's.
    pub fn sprite_tiles(&self) -> usize {
        self.gfx3.len() / (2 * HALF_TILE)
    }
}
