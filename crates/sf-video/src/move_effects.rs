//! Native-size, indexed move effects for the Overhaul comparison build.

use std::io::Cursor;
use std::sync::LazyLock;

use crate::asset_pack::{quantize_sheet, IndexedTile, Rgba};
use crate::compositor::IndexedSprite;

pub mod accent;
pub mod signature;
mod bonus;
mod kicks;
mod power;
mod roster;

const SIZE: usize = 16;
static LANDING_DUST: LazyLock<Vec<IndexedTile>> =
    LazyLock::new(|| decode_strip(include_bytes!("move_effects/landing-dust-16px_strip7.png")));
static PARRY: LazyLock<Vec<IndexedTile>> =
    LazyLock::new(|| decode_strip(include_bytes!("move_effects/parry-flash-16px_strip7.png")));
static KEN_FIRE: LazyLock<Vec<IndexedTile>> =
    LazyLock::new(|| decode_strip(include_bytes!("move_effects/ken-fire-16px_strip8.png")));
static KEN_BURN: LazyLock<Vec<IndexedTile>> =
    LazyLock::new(|| decode_strip(include_bytes!("move_effects/ken-burn-16px_strip7.png")));

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectKind {
    LandingDust,
    Parry,
    KenFire,
    KenBurn,
}

impl EffectKind {
    /// Duration in game ticks, or one animation period for Ken's looping fire.
    pub const fn lifetime(self) -> u8 {
        match self {
            Self::LandingDust | Self::Parry => 14,
            Self::KenFire => 24,
            Self::KenBurn => 36,
        }
    }
}

/// Select a frame and place its anchor at `(x, y)` in native game pixels.
/// Landing dust anchors at the ground. Sparks and fire anchor at the contact, fist or body.
/// Directional art is authored facing right. Pass `flip` for a left-facing fighter.
pub fn effect_frame(
    kind: EffectKind,
    age: u8,
    x: i32,
    y: i32,
    flip: bool,
) -> Option<IndexedSprite> {
    let period = kind.lifetime();
    let age = if kind == EffectKind::KenFire {
        age % period
    } else if age >= period {
        return None;
    } else {
        age
    };
    let (strip, anchor_x, anchor_y) = match kind {
        EffectKind::LandingDust => (&*LANDING_DUST, 8, 14),
        EffectKind::Parry => (&*PARRY, 10, 8),
        EffectKind::KenFire => (&*KEN_FIRE, 7, 4),
        EffectKind::KenBurn => (&*KEN_BURN, 8, 9),
    };
    let frame_index = match kind {
        EffectKind::KenBurn if age < 24 => (age / 2 % 3).into(),
        EffectKind::KenBurn => usize::from(3 + (age - 24) / 3),
        _ => usize::from(age) * strip.len() / usize::from(period),
    };
    let tile = &strip[frame_index];
    let mut pixels = tile.pixels;
    let anchor_x = if flip {
        for row in pixels.chunks_exact_mut(SIZE) {
            row.reverse();
        }
        SIZE as i32 - 1 - anchor_x
    } else {
        anchor_x
    };
    Some(IndexedSprite {
        x: x - anchor_x,
        y: y - anchor_y,
        pixels,
        palette: tile
            .palette
            .map(|c| u32::from_be_bytes([c.a, c.r, c.g, c.b])),
    })
}

fn decode_strip(bytes: &[u8]) -> Vec<IndexedTile> {
    let (tiles, _, height) = decode_sheet(bytes);
    assert_eq!(height, SIZE);
    tiles
}

fn decode_sheet(bytes: &[u8]) -> (Vec<IndexedTile>, usize, usize) {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().expect("embedded effect PNG header");
    let mut rgba = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut rgba)
        .expect("embedded effect PNG data");
    assert_eq!(info.color_type, png::ColorType::Rgba);
    let pixels: Vec<_> = rgba[..info.buffer_size()]
        .chunks_exact(4)
        .map(|pixel| Rgba {
            r: pixel[0],
            g: pixel[1],
            b: pixel[2],
            a: pixel[3],
        })
        .collect();
    let (width, height) = (info.width as usize, info.height as usize);
    let tiles = quantize_sheet(&pixels, width, height).expect("embedded move effect sheet");
    (tiles, width, height)
}
