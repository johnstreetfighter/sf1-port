//! Offline overhaul asset packs (`#99`, `O8`).
//!
//! Artist sheets and externally generated images come in as RGBA. This module
//! records the proportions of a reference sprite, fits a new sheet to that
//! box, quantizes the result to 16×16 tiles with 15 colours plus a transparent
//! pen, encodes ADPCM and YM2151 patches, and checks a pack on disk. It does
//! not read arcade ROMs.

use std::fs;
use std::path::Path;

/// One indexed 16×16 tile. Pen 0 is transparent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedTile {
    pub pixels: [u8; 256],
    pub palette: [Rgba; 16],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const TRANSPARENT: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };

    fn opaque(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    fn dist2(self, other: Self) -> u32 {
        let dr = self.r as i32 - other.r as i32;
        let dg = self.g as i32 - other.g as i32;
        let db = self.b as i32 - other.b as i32;
        (dr * dr + dg * dg + db * db) as u32
    }
}

/// The MSM5205's sample rate on the board: the samples CPU gives each chip a
/// nibble on every other 8 kHz interrupt, so 4 kHz a chip
/// (`sf_audio::samples`). The announcer's third chip runs at it too (ADR 0011).
pub const ADPCM_RATE: u32 = 4_000;

/// 4-bit 4 kHz MSM5205 ADPCM clip. `nibbles` holds one 4-bit sample each.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AdpcmClip {
    pub sample_rate: u32,
    pub nibbles: Vec<u8>,
}

/// YM2151 register writes for one FM patch.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FmPatch {
    pub regs: Vec<(u8, u8)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackError {
    pub message: String,
}

impl PackError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Split an RGBA sheet into 16×16 tiles. Width and height must be multiples of 16.
pub fn quantize_sheet(
    pixels: &[Rgba],
    width: usize,
    height: usize,
) -> Result<Vec<IndexedTile>, PackError> {
    if width == 0 || height == 0 || width % 16 != 0 || height % 16 != 0 {
        return Err(PackError::new(
            "sprite sheet width and height must be positive multiples of 16",
        ));
    }
    if pixels.len() != width * height {
        return Err(PackError::new(
            "sprite sheet byte length does not match its size",
        ));
    }
    let mut tiles = Vec::new();
    for ty in (0..height).step_by(16) {
        for tx in (0..width).step_by(16) {
            let mut src = [Rgba::TRANSPARENT; 256];
            for py in 0..16 {
                for px in 0..16 {
                    src[py * 16 + px] = pixels[(ty + py) * width + tx + px];
                }
            }
            tiles.push(quantize_tile(&src));
        }
    }
    for tile in &tiles {
        validate_tile(tile)?;
    }
    Ok(tiles)
}

fn quantize_tile(src: &[Rgba; 256]) -> IndexedTile {
    let mut palette = [Rgba::TRANSPARENT; 16];
    let mut used = 1usize;
    let mut pixels = [0u8; 256];
    for (i, px) in src.iter().enumerate() {
        if px.a < 128 {
            pixels[i] = 0;
            continue;
        }
        let solid = Rgba::opaque(px.r, px.g, px.b);
        if let Some(pen) = (1..used).find(|&pen| palette[pen] == solid) {
            pixels[i] = pen as u8;
            continue;
        }
        if used < 16 {
            palette[used] = solid;
            pixels[i] = used as u8;
            used += 1;
            continue;
        }
        let (pen, _) = (1..16)
            .map(|pen| (pen, palette[pen].dist2(solid)))
            .min_by_key(|(_, d)| *d)
            .unwrap();
        pixels[i] = pen as u8;
    }
    IndexedTile { pixels, palette }
}

pub fn validate_tile(tile: &IndexedTile) -> Result<(), PackError> {
    if tile.pixels.len() != 256 {
        return Err(PackError::new("tile is not 16 by 16"));
    }
    if tile.palette.len() != 16 {
        return Err(PackError::new("tile palette must hold 16 entries"));
    }
    if tile.pixels.iter().any(|pen| *pen > 15) {
        return Err(PackError::new("tile pen is outside 0..=15"));
    }
    let mut seen = [false; 16];
    for pen in tile.pixels {
        seen[pen as usize] = true;
    }
    let colours = seen.iter().filter(|on| **on).count();
    if colours > 16 {
        return Err(PackError::new("tile uses more than 16 colours"));
    }
    Ok(())
}

pub fn validate_adpcm(clip: &AdpcmClip) -> Result<(), PackError> {
    if clip.sample_rate != ADPCM_RATE {
        return Err(PackError::new("ADPCM sample rate must be 4000 Hz"));
    }
    if clip.nibbles.is_empty() {
        return Err(PackError::new("ADPCM clip is empty"));
    }
    if clip.nibbles.iter().any(|n| *n > 15) {
        return Err(PackError::new("ADPCM nibble is wider than 4 bits"));
    }
    Ok(())
}

/// A patch must key an operator on and write at least one operator total-level
/// register. Those are the bytes a YM2151 actually plays.
pub fn validate_fm_patch(patch: &FmPatch) -> Result<(), PackError> {
    let keyed = patch
        .regs
        .iter()
        .any(|(reg, val)| *reg == 0x08 && *val & 0x78 != 0);
    let level = patch
        .regs
        .iter()
        .any(|(reg, _)| (0x60..=0x7f).contains(reg));
    if !keyed || !level {
        return Err(PackError::new(
            "YM2151 patch needs a key-on write (register 0x08) and an operator level (0x60..=0x7f)",
        ));
    }
    Ok(())
}

pub fn validate_pack(
    tiles: &[IndexedTile],
    clips: &[AdpcmClip],
    patches: &[FmPatch],
) -> Result<(), PackError> {
    if tiles.is_empty() {
        return Err(PackError::new("asset pack has no tiles"));
    }
    for tile in tiles {
        validate_tile(tile)?;
    }
    for clip in clips {
        validate_adpcm(clip)?;
    }
    for patch in patches {
        validate_fm_patch(patch)?;
    }
    Ok(())
}

const ADJUST: [i32; 8] = [-1, -1, -1, -1, 2, 4, 6, 8];

fn msm_step(index: i32) -> i32 {
    (16.0 * (11.0 / 10.0_f64).powi(index)).floor() as i32
}

fn msm_delta(step: i32, nibble: i32) -> i32 {
    let sign = if nibble & 8 != 0 { -1 } else { 1 };
    let bit = |n: i32| (nibble >> n) & 1;
    sign * (step * bit(2) + step / 2 * bit(1) + step / 4 * bit(0) + step / 8)
}

/// Encode PCM (at [`ADPCM_RATE`]) as 4-bit 4 kHz MSM5205 nibbles. Samples
/// are clamped to the chip's 12-bit signal, then the closest step is chosen.
pub fn encode_adpcm(pcm: &[i16]) -> Result<AdpcmClip, PackError> {
    if pcm.is_empty() {
        return Err(PackError::new("PCM clip is empty"));
    }
    let mut signal = 0i32;
    let mut index = 0i32;
    let mut nibbles = Vec::with_capacity(pcm.len());
    for sample in pcm {
        let target = i32::from(*sample).clamp(-2048, 2047);
        let step = msm_step(index);
        let mut best = 0i32;
        let mut best_err = i32::MAX;
        for nib in 0..16 {
            let next = (signal + msm_delta(step, nib)).clamp(-2048, 2047);
            let err = (next - target).abs();
            if err < best_err {
                best_err = err;
                best = nib;
            }
        }
        signal = (signal + msm_delta(step, best)).clamp(-2048, 2047);
        index = (index + ADJUST[(best & 7) as usize]).clamp(0, 48);
        nibbles.push(best as u8);
    }
    let clip = AdpcmClip {
        sample_rate: ADPCM_RATE,
        nibbles,
    };
    validate_adpcm(&clip)?;
    Ok(clip)
}

pub fn decode_adpcm(clip: &AdpcmClip) -> Result<Vec<i16>, PackError> {
    validate_adpcm(clip)?;
    let mut signal = 0i32;
    let mut index = 0i32;
    let mut pcm = Vec::with_capacity(clip.nibbles.len());
    for nibble in &clip.nibbles {
        let step = msm_step(index);
        signal = (signal + msm_delta(step, i32::from(*nibble))).clamp(-2048, 2047);
        index = (index + ADJUST[(nibble & 7) as usize]).clamp(0, 48);
        pcm.push(signal as i16);
    }
    Ok(pcm)
}

/// A one-operator tone the validator accepts. This is patch data, not a render of the chip.
pub fn simple_fm_patch(channel: u8, total_level: u8) -> FmPatch {
    let ch = channel & 7;
    FmPatch {
        regs: vec![
            (0x20 + ch, 0xc1),
            (0x60 + ch, total_level),
            (0x80 + ch, 0x1a),
            (0x08, 0x78 | ch),
        ],
    }
}

/// Opaque box of a reference sprite, in 16×16 tile pixels. Inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProportionRef {
    pub left: u8,
    pub top: u8,
    pub right: u8,
    pub bottom: u8,
}

/// A pack written next to the game, not inside a ROM set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetPack {
    pub tiles: Vec<IndexedTile>,
    pub clips: Vec<AdpcmClip>,
    pub patches: Vec<FmPatch>,
    pub proportions: Vec<ProportionRef>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct PackFile {
    tiles: Vec<TileFile>,
    clips: Vec<AdpcmClip>,
    patches: Vec<FmPatch>,
    proportions: Vec<ProportionRef>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct TileFile {
    pixels: Vec<u8>,
    palette: Vec<Rgba>,
}

impl PackFile {
    fn from(pack: &AssetPack) -> Self {
        Self {
            tiles: pack
                .tiles
                .iter()
                .map(|tile| TileFile {
                    pixels: tile.pixels.to_vec(),
                    palette: tile.palette.to_vec(),
                })
                .collect(),
            clips: pack.clips.clone(),
            patches: pack.patches.clone(),
            proportions: pack.proportions.clone(),
        }
    }

    fn into_pack(self) -> Result<AssetPack, PackError> {
        let mut tiles = Vec::with_capacity(self.tiles.len());
        for tile in self.tiles {
            if tile.pixels.len() != 256 || tile.palette.len() != 16 {
                return Err(PackError::new(
                    "tile is not 16 by 16 with a 16-colour palette",
                ));
            }
            let mut pixels = [0u8; 256];
            pixels.copy_from_slice(&tile.pixels);
            let mut palette = [Rgba::TRANSPARENT; 16];
            palette.copy_from_slice(&tile.palette);
            tiles.push(IndexedTile { pixels, palette });
        }
        Ok(AssetPack {
            tiles,
            clips: self.clips,
            patches: self.patches,
            proportions: self.proportions,
        })
    }
}

const PACK_FILE: &str = "pack.json";

/// Opaque bounds of a decoded 1987 sprite tile. Pen 15 is transparent, the
/// same rule the hardware renderer uses. Pen 0 is ink.
pub fn proportion_from_decoded_sprite(tile: &[[u8; 16]; 16]) -> ProportionRef {
    let mut left = 15u8;
    let mut top = 15u8;
    let mut right = 0u8;
    let mut bottom = 0u8;
    let mut any = false;
    for y in 0..16 {
        for x in 0..16 {
            if tile[y][x] == 15 {
                continue;
            }
            any = true;
            left = left.min(x as u8);
            top = top.min(y as u8);
            right = right.max(x as u8);
            bottom = bottom.max(y as u8);
        }
    }
    if !any {
        return ProportionRef {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
    }
    ProportionRef {
        left,
        top,
        right,
        bottom,
    }
}

/// Opaque bounds of an indexed tile. An empty tile is a 1×1 box at the origin.
pub fn proportion_from_tile(tile: &IndexedTile) -> ProportionRef {
    let mut left = 15u8;
    let mut top = 15u8;
    let mut right = 0u8;
    let mut bottom = 0u8;
    let mut any = false;
    for y in 0..16 {
        for x in 0..16 {
            if tile.pixels[y * 16 + x] == 0 {
                continue;
            }
            any = true;
            left = left.min(x as u8);
            top = top.min(y as u8);
            right = right.max(x as u8);
            bottom = bottom.max(y as u8);
        }
    }
    if !any {
        return ProportionRef {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
    }
    ProportionRef {
        left,
        top,
        right,
        bottom,
    }
}

/// Scale an authored sheet's opaque pixels into the reference box on a 16×16 canvas.
pub fn conform_to_proportion(
    pixels: &[Rgba],
    width: usize,
    height: usize,
    reference: ProportionRef,
) -> Result<Vec<Rgba>, PackError> {
    if width == 0 || height == 0 || pixels.len() != width * height {
        return Err(PackError::new(
            "authored sheet size does not match its pixels",
        ));
    }
    if reference.right < reference.left
        || reference.bottom < reference.top
        || reference.right > 15
        || reference.bottom > 15
    {
        return Err(PackError::new(
            "proportion box is outside the 16 by 16 tile",
        ));
    }
    let mut bounds: Option<(usize, usize, usize, usize)> = None;
    for y in 0..height {
        for x in 0..width {
            if pixels[y * width + x].a < 128 {
                continue;
            }
            bounds = Some(match bounds {
                None => (x, y, x, y),
                Some((l, t, r, b)) => (l.min(x), t.min(y), r.max(x), b.max(y)),
            });
        }
    }
    let mut out = vec![Rgba::TRANSPARENT; 256];
    let Some((l, t, r, b)) = bounds else {
        return Ok(out);
    };
    let src_w = r - l + 1;
    let src_h = b - t + 1;
    let dst_w = (reference.right - reference.left) as usize + 1;
    let dst_h = (reference.bottom - reference.top) as usize + 1;
    for dy in 0..dst_h {
        for dx in 0..dst_w {
            let sx = l + dx * src_w / dst_w;
            let sy = t + dy * src_h / dst_h;
            let x = reference.left as usize + dx;
            let y = reference.top as usize + dy;
            out[y * 16 + x] = pixels[sy * width + sx];
        }
    }
    Ok(out)
}

pub fn write_pack(dir: &Path, pack: &AssetPack) -> Result<(), PackError> {
    validate_pack(&pack.tiles, &pack.clips, &pack.patches)?;
    for prop in &pack.proportions {
        if prop.right > 15 || prop.bottom > 15 || prop.right < prop.left || prop.bottom < prop.top {
            return Err(PackError::new(
                "proportion box is outside the 16 by 16 tile",
            ));
        }
    }
    fs::create_dir_all(dir).map_err(|e| PackError::new(format!("cannot create pack dir: {e}")))?;
    let file = PackFile::from(pack);
    let text = serde_json::to_string_pretty(&file)
        .map_err(|e| PackError::new(format!("cannot encode pack: {e}")))?;
    fs::write(dir.join(PACK_FILE), text)
        .map_err(|e| PackError::new(format!("cannot write pack: {e}")))
}

/// Read `pack.json` and reject a pack that breaks the tile, ADPCM, or FM rules.
pub fn validate_pack_dir(dir: &Path) -> Result<AssetPack, PackError> {
    let text = fs::read_to_string(dir.join(PACK_FILE))
        .map_err(|e| PackError::new(format!("cannot read pack: {e}")))?;
    let file: PackFile = serde_json::from_str(&text)
        .map_err(|e| PackError::new(format!("pack json is not an asset pack: {e}")))?;
    let pack = file.into_pack()?;
    validate_pack(&pack.tiles, &pack.clips, &pack.patches)?;
    for prop in &pack.proportions {
        if prop.right > 15 || prop.bottom > 15 || prop.right < prop.left || prop.bottom < prop.top {
            return Err(PackError::new(
                "proportion box is outside the 16 by 16 tile",
            ));
        }
    }
    Ok(pack)
}
