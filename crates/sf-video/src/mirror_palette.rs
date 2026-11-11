//! Mirror palettes (spec #246, SP16): the second palette each fighter wears
//! on P2's side of a mirror match, so the two can be told apart.
//!
//! A fighter's file, `art/palettes/<fighter>.json`, names the ROM sprite
//! palette his body is drawn in (`from`), the free sprite palette that
//! stands in for it on P2's side (`slot`), and the 15 opaque pens' colours in
//! it (pen 15 is transparent and keeps the ROM's word). Only his body is
//! recoloured: projectiles, bursts and hit sparks keep their colours (the maintainer,
//! 2026-11-08). The engine writes the slot into palette RAM and points P2's
//! sprites at it, through an overhaul-only gate (`sf_engine::mirror`).
//!
//! The colours are hand-picked from the fighter's own, as SF2 did its
//! alternate colours, so they are original choices and not data dumped from
//! a ROM. They are 12-bit like the hardware's, written `#rrggbb` with each
//! pair of digits doubled, the way `sf-art palette` prints a ROM pen.
//!
//! The files are embedded: the game stays one file, and the engine, the
//! review page and the tests read the same bytes.

use std::sync::OnceLock;

use serde::Deserialize;

use crate::romset::crc32;

/// Opaque pens in a sprite palette (pen 15 is transparent).
pub const PENS: usize = 15;
/// The sprite palette every hit spark uses, which no mirror palette may use.
pub const SPARK_PALETTE: u8 = 13;

/// Every fighter's file, by lowercase name.
pub const FILES: [(&str, &str); 12] = [
    ("ryu", include_str!("../../../art/palettes/ryu.json")),
    ("ken", include_str!("../../../art/palettes/ken.json")),
    ("retsu", include_str!("../../../art/palettes/retsu.json")),
    ("geki", include_str!("../../../art/palettes/geki.json")),
    ("joe", include_str!("../../../art/palettes/joe.json")),
    ("mike", include_str!("../../../art/palettes/mike.json")),
    ("lee", include_str!("../../../art/palettes/lee.json")),
    ("gen", include_str!("../../../art/palettes/gen.json")),
    ("birdie", include_str!("../../../art/palettes/birdie.json")),
    ("eagle", include_str!("../../../art/palettes/eagle.json")),
    ("adon", include_str!("../../../art/palettes/adon.json")),
    ("sagat", include_str!("../../../art/palettes/sagat.json")),
];

/// A fighter's mirror palette.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirrorPalette {
    pub fighter: String,
    pub note: String,
    /// The ROM sprite palette his body is drawn in.
    pub from: u8,
    /// The free sprite palette P2's body is drawn in instead.
    pub slot: u8,
    /// The 15 opaque pens as xRGB_444 words (bits 8-11 red, 4-7 green,
    /// 0-3 blue), the hardware's own.
    pub pens: [u16; PENS],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    fighter: String,
    note: String,
    from: u8,
    slot: u8,
    pens: Vec<String>,
}

/// `#rrggbb` (each pair of digits doubled) as an xRGB_444 word.
pub fn parse_colour(text: &str) -> Result<u16, String> {
    let digits = text.strip_prefix('#').filter(|d| d.len() == 6).ok_or_else(|| format!("{text:?} is not #rrggbb"))?;
    let mut word = 0u16;
    for pair in digits.as_bytes().chunks(2) {
        let [hi, lo] = [pair[0], pair[1]];
        let value = |c: u8| (c as char).to_digit(16).ok_or_else(|| format!("{text:?} is not #rrggbb"));
        let (hi, lo) = (value(hi)?, value(lo)?);
        if hi != lo {
            return Err(format!("{text:?} is not a 12-bit colour: each pair of digits must be doubled"));
        }
        word = word << 4 | hi as u16;
    }
    Ok(word)
}

/// An xRGB_444 word as `#rrggbb`, each digit doubled.
pub fn colour_text(word: u16) -> String {
    let d = |shift: u32| {
        let n = (word >> shift) & 0xf;
        format!("{n:x}{n:x}")
    };
    format!("#{}{}{}", d(8), d(4), d(0))
}

/// An xRGB_444 word as opaque ARGB.
pub fn argb(word: u16) -> u32 {
    let c = |shift: u32| {
        let n = ((word >> shift) & 0xf) as u32;
        n << 4 | n
    };
    0xff00_0000 | c(8) << 16 | c(4) << 8 | c(0)
}

impl MirrorPalette {
    /// Read a fighter's file, checking what any file must hold: 15 pens,
    /// the palette and the slot among the 16, different, and neither the
    /// hit sparks' (13).
    pub fn parse(text: &str) -> Result<MirrorPalette, String> {
        let file: File = serde_json::from_str(text).map_err(|e| format!("a mirror palette file: {e}"))?;
        let who = file.fighter.clone();
        if file.from > 15 || file.slot > 15 {
            return Err(format!("{who}: sprite palettes are 0 to 15"));
        }
        if file.from == file.slot {
            return Err(format!("{who}: the slot is the palette it stands in for"));
        }
        if file.from == SPARK_PALETTE || file.slot == SPARK_PALETTE {
            return Err(format!("{who}: palette {SPARK_PALETTE} is the hit sparks'"));
        }
        if file.pens.len() != PENS {
            return Err(format!("{who}: {} pens, expected {PENS}", file.pens.len()));
        }
        let mut pens = [0u16; PENS];
        for (pen, text) in file.pens.iter().enumerate() {
            pens[pen] = parse_colour(text).map_err(|e| format!("{who}: pen {pen}: {e}"))?;
        }
        Ok(MirrorPalette { fighter: file.fighter, note: file.note, from: file.from, slot: file.slot, pens })
    }

    /// The CRC-32 of what the engine uses (the palette, the slot and the
    /// pens): a review verdict holds for the colours it was given on, and a
    /// changed palette is pending again.
    pub fn digest(&self) -> String {
        let mut bytes = vec![self.from, self.slot];
        for pen in self.pens {
            bytes.extend(pen.to_be_bytes());
        }
        format!("{:08x}", crc32(&bytes))
    }
}

/// The twelve committed palettes, read once.
pub fn committed() -> &'static [MirrorPalette] {
    static ALL: OnceLock<Vec<MirrorPalette>> = OnceLock::new();
    ALL.get_or_init(|| {
        FILES
            .iter()
            .map(|(name, text)| {
                let p = MirrorPalette::parse(text).unwrap_or_else(|e| panic!("art/palettes/{name}.json: {e}"));
                assert_eq!(p.fighter, *name, "art/palettes/{name}.json names {}", p.fighter);
                p
            })
            .collect()
    })
}

/// One fighter's committed palette, by lowercase name.
pub fn for_fighter(name: &str) -> Option<&'static MirrorPalette> {
    committed().iter().find(|p| p.fighter.eq_ignore_ascii_case(name))
}
