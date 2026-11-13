//! Palette RAM interpretation: 1024 big-endian u16 entries in xRGB_444
//! format (bits 8-11 red, 4-7 green, 0-3 blue), matching
//! palette_device::xRGB_444 in the pinned MAME source.

/// The decoded palette: 1024 opaque ARGB entries.
pub struct Palette {
    entries: [u32; 1024],
}

fn pal4bit(v: u16) -> u32 {
    let v = (v & 0xf) as u32;
    (v << 4) | v
}

pub(crate) fn xrgb444(word: u16) -> u32 {
    let (r, g, b) = (pal4bit(word >> 8), pal4bit(word >> 4), pal4bit(word));
    0xff00_0000 | (r << 16) | (g << 8) | b
}

impl Palette {
    pub fn from_ram(bytes: &[u8]) -> Self {
        assert!(bytes.len() >= 2048, "palette RAM dump must be 2048 bytes");
        let mut entries = [0u32; 1024];
        for (i, entry) in entries.iter_mut().enumerate() {
            let v = u16::from_be_bytes([bytes[2 * i], bytes[2 * i + 1]]);
            *entry = xrgb444(v);
        }
        Palette { entries }
    }

    pub fn argb(&self, index: usize) -> u32 {
        self.entries[index]
    }
}
