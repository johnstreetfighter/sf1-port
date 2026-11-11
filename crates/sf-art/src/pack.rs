//! Generated pictures into the art pack (spec #164, AP5).
//!
//! - **His colours** come from palette RAM in a real fight frame: all 15
//!   opaque pens. Sagat's black (pen 0, his eyepatch's rim) is one of them;
//!   the spike missed it because it cut his frames from a render whose
//!   backdrop was black too, so its palette and references had no black.
//! - **References** are his ROM poses drawn from their pens, transparent
//!   only where pen 15 is.
//! - **Reduction** turns a candidate into his pens: hard alpha (a pixel is
//!   opaque only at half alpha or more), each opaque pixel the nearest of
//!   his opaque pens, cropped to what is opaque. Where two of his pens share
//!   a colour (Geki's body blue and his smoke's), the pen his stance draws
//!   wins.

use sf_engine::roster::TournamentFighter;
use sf_video::art::{Grid, Pens, TRANSPARENT};
use sf_video::palette::Palette;
use sf_video::romset::Assets;

use crate::frame::fight_frame;
use crate::site::opponent;

/// The sprite palettes' place in palette RAM.
const SPRITE_PALETTES: usize = 512;

/// His 16 pens' colours (ARGB) in a real fight; pen 15 is transparent.
pub fn fighter_colours(rom: &Assets, fighter: TournamentFighter) -> Result<[u32; 16], String> {
    let ff = fight_frame(rom, fighter, opponent(fighter))?;
    let palette = Palette::from_ram(&ff.scene.paletteram);
    Ok(std::array::from_fn(|pen| palette.argb(SPRITE_PALETTES + ff.palette as usize * 16 + pen)))
}

/// `#rrggbb`.
pub fn hex(colour: u32) -> String {
    format!("#{:06x}", colour & 0xff_ffff)
}

/// A picture as RGBA bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rgba {
    pub width: usize,
    pub height: usize,
    pub bytes: Vec<u8>,
}

impl Rgba {
    pub fn read_png(bytes: &[u8]) -> Result<Rgba, String> {
        let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        decoder.set_transformations(png::Transformations::normalize_to_color8() | png::Transformations::ALPHA);
        let mut reader = decoder.read_info().map_err(|e| format!("not a PNG: {e}"))?;
        let mut buf = vec![0; reader.output_buffer_size()];
        let frame = reader.next_frame(&mut buf).map_err(|e| format!("not a PNG: {e}"))?;
        let (width, height) = (frame.width as usize, frame.height as usize);
        let bytes = match frame.color_type {
            png::ColorType::Rgba => buf[..width * height * 4].to_vec(),
            png::ColorType::GrayscaleAlpha => buf[..width * height * 2].chunks(2).flat_map(|p| [p[0], p[0], p[0], p[1]]).collect(),
            other => return Err(format!("an unexpected {other:?} PNG")),
        };
        Ok(Rgba { width, height, bytes })
    }

    pub fn png(&self) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut out, self.width as u32, self.height as u32);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut writer = enc.write_header().expect("a PNG header");
            writer.write_image_data(&self.bytes).expect("PNG data");
        }
        out
    }

    /// Pens drawn in `colours`, transparent where pen 15 is.
    pub fn from_pens(pens: &Pens, colours: &[u32; 16]) -> Rgba {
        let bytes = pens
            .pens
            .iter()
            .flat_map(|&pen| {
                let c = colours[pen as usize];
                let a = if pen == TRANSPARENT { 0 } else { 255 };
                [(c >> 16) as u8, (c >> 8) as u8, c as u8, a]
            })
            .collect();
        Rgba { width: pens.width, height: pens.height, bytes }
    }
}

/// One of his ROM poses as a reference picture, cropped to its opaque
/// pixels.
pub fn reference(rom: &Assets, grid: &Grid, colours: &[u32; 16]) -> Rgba {
    Rgba::from_pens(&crop_to_opaque(&Pens::from_grid(&rom.gfx3, grid)), colours)
}

/// What reducing a candidate found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reduced {
    pub pens: Pens,
    /// Opaque pixels that were exactly one of his colours, and those that
    /// were not and took the nearest.
    pub exact: usize,
    pub nearest: usize,
    /// Pixels neither fully clear nor fully opaque.
    pub soft_alpha: usize,
}

/// How many pixels of each pen his stance (move 1's header pose) draws. Two
/// of his pens can share a colour (Geki's body blue, pen 14, is his smoke's,
/// pen 6), and this says which one his body is drawn in.
pub fn stance_pens(rom: &Assets, fighter: TournamentFighter) -> [usize; 16] {
    let m = &rom.maincpu;
    let h = fighter.move_table() as usize + 0x30;
    let word = |a: usize| u16::from_be_bytes([m[a], m[a + 1]]);
    let grid = Grid { code: word(h), large: (word(h + 2) as i16) >= 0x400, rows: m[h + 0xc], cols: m[h + 0xd] };
    let mut drawn = [0; 16];
    for &pen in &Pens::from_grid(&rom.gfx3, &grid).pens {
        drawn[pen as usize] += 1;
    }
    drawn
}

/// Reduce a candidate to his pens, cropped to its opaque pixels. Where two
/// pens are equally near, the one `drawn` counts more of (his stance's)
/// wins, then the lower.
pub fn reduce(picture: &Rgba, colours: &[u32; 16], drawn: &[usize; 16]) -> Result<Reduced, String> {
    let (mut exact, mut nearest, mut soft_alpha) = (0, 0, 0);
    let mut pens = Vec::with_capacity(picture.width * picture.height);
    for p in picture.bytes.chunks(4) {
        let (r, g, b, a) = (p[0] as i32, p[1] as i32, p[2] as i32, p[3]);
        if a != 0 && a != 255 {
            soft_alpha += 1;
        }
        if a < 128 {
            pens.push(TRANSPARENT);
            continue;
        }
        let distance = |c: u32| {
            let (cr, cg, cb) = ((c >> 16 & 0xff) as i32, (c >> 8 & 0xff) as i32, (c & 0xff) as i32);
            (r - cr).pow(2) + (g - cg).pow(2) + (b - cb).pow(2)
        };
        let pen = (0..TRANSPARENT)
            .min_by_key(|&pen| (distance(colours[pen as usize]), std::cmp::Reverse(drawn[pen as usize])))
            .expect("15 pens");
        if distance(colours[pen as usize]) == 0 {
            exact += 1;
        } else {
            nearest += 1;
        }
        pens.push(pen);
    }
    let all = Pens { width: picture.width, height: picture.height, pens };
    if exact + nearest == 0 {
        return Err("the picture has no opaque pixels".to_owned());
    }
    Ok(Reduced { pens: crop_to_opaque(&all), exact, nearest, soft_alpha })
}

/// The smallest rectangle holding every opaque pixel.
pub fn crop_to_opaque(p: &Pens) -> Pens {
    let opaque = |x: usize, y: usize| p.at(x, y) != TRANSPARENT;
    let rows: Vec<usize> = (0..p.height).filter(|&y| (0..p.width).any(|x| opaque(x, y))).collect();
    let cols: Vec<usize> = (0..p.width).filter(|&x| (0..p.height).any(|y| opaque(x, y))).collect();
    let (Some(&y0), Some(&y1), Some(&x0), Some(&x1)) = (rows.first(), rows.last(), cols.first(), cols.last()) else {
        return Pens { width: 0, height: 0, pens: Vec::new() };
    };
    let (width, height) = (x1 - x0 + 1, y1 - y0 + 1);
    let pens = (y0..=y1).flat_map(|y| (x0..=x1).map(move |x| (x, y))).map(|(x, y)| p.at(x, y)).collect();
    Pens { width, height, pens }
}

/// A packed pose's first anchor: centred on him, its lowest row on the
/// ground. Review settles the rest.
pub fn default_anchor(pens: &Pens) -> [i16; 2] {
    [-((pens.width / 2) as i16), -(pens.height as i16)]
}
