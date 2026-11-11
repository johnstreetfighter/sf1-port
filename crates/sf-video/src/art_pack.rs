//! The art pack (spec #164, AP5): the generated poses' pixels. It is a
//! local folder beside the ROM set, never part of the public repository,
//! and backed up to a private one.
//!
//! - The folder is named [`PACK_FOLDER`] and sits beside the ROM set's
//!   folder (`roms\sf` and `roms\sf1-port-art`).
//! - `manifest.json` lists every pose file with its size and CRC-32, as the
//!   ROM manifest lists the ROM's files.
//! - Each pose is an indexed PNG, `<fighter>/<pose>.png`. A pixel's index
//!   is its pen; pen 15 is transparent. The PNG's palette holds the
//!   fighter's own colours so the file looks like him in any viewer; the
//!   game reads only the pens.
//! - Anything else in the folder (the raw candidates, the backup's git
//!   files) is not the game's business.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::art::{Pens, TRANSPARENT};
use crate::romset::crc32;

/// The pack's folder name, beside the ROM set's folder.
pub const PACK_FOLDER: &str = "sf1-port-art";
/// The manifest's file name in the pack.
pub const MANIFEST: &str = "manifest.json";
/// The manifest format this game reads.
pub const FORMAT: u32 = 1;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    format: u32,
    files: BTreeMap<String, FileEntry>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileEntry {
    size: u64,
    crc32: String,
}

/// One pose from the pack.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackPose {
    /// The CRC-32 of its file, which the fighter's recipe names.
    pub crc32: u32,
    pub pens: Pens,
}

/// A pack that matched its manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtPack {
    pub folder: PathBuf,
    /// By `<fighter>/<pose>`.
    poses: BTreeMap<String, PackPose>,
}

impl ArtPack {
    /// Open the pack in `folder` and check every file against the
    /// manifest. An error is the reason the pack cannot be used.
    pub fn open(folder: &Path) -> Result<ArtPack, String> {
        let text = std::fs::read_to_string(folder.join(MANIFEST))
            .map_err(|e| format!("{}: {MANIFEST}: {e}", folder.display()))?;
        let manifest: Manifest =
            serde_json::from_str(&text).map_err(|e| format!("{}: {MANIFEST}: {e}", folder.display()))?;
        if manifest.format != FORMAT {
            return Err(format!(
                "{}: format {} (this game reads format {FORMAT})",
                folder.display(),
                manifest.format
            ));
        }
        let mut poses = BTreeMap::new();
        for (file, entry) in &manifest.files {
            let where_ = || format!("{}: {file}", folder.display());
            let key = file
                .strip_suffix(".png")
                .filter(|k| k.split('/').count() == 2)
                .ok_or_else(|| format!("{}: not a <fighter>/<pose>.png file", where_()))?;
            let bytes = std::fs::read(folder.join(file)).map_err(|e| format!("{}: {e}", where_()))?;
            let crc = crc32(&bytes);
            if bytes.len() as u64 != entry.size || format!("{crc:08x}") != entry.crc32 {
                return Err(format!("{}: does not match the manifest (damaged or changed)", where_()));
            }
            let pens = decode_png(&bytes).map_err(|e| format!("{}: {e}", where_()))?;
            poses.insert(key.to_owned(), PackPose { crc32: crc, pens });
        }
        Ok(ArtPack { folder: folder.to_owned(), poses })
    }

    pub fn pose(&self, fighter: &str, name: &str) -> Option<&PackPose> {
        self.poses.get(&format!("{fighter}/{name}"))
    }

    pub fn len(&self) -> usize {
        self.poses.len()
    }

    pub fn is_empty(&self) -> bool {
        self.poses.is_empty()
    }
}

/// Write `pens` into the pack at `folder` as `<fighter>/<name>.png`, with
/// the fighter's `colours` (ARGB, pen by pen) as its palette, and record it
/// in the manifest. Returns the file's CRC-32.
pub fn write_pose(folder: &Path, fighter: &str, name: &str, pens: &Pens, colours: &[u32; 16]) -> Result<u32, String> {
    let file = format!("{fighter}/{name}.png");
    let bytes = encode_png(pens, colours)?;
    let path = folder.join(&file);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(&path, &bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    let manifest_path = folder.join(MANIFEST);
    let mut manifest = match std::fs::read_to_string(&manifest_path) {
        Ok(text) => serde_json::from_str(&text).map_err(|e| format!("{}: {e}", manifest_path.display()))?,
        Err(_) => Manifest { format: FORMAT, files: BTreeMap::new() },
    };
    let crc = crc32(&bytes);
    manifest.files.insert(file, FileEntry { size: bytes.len() as u64, crc32: format!("{crc:08x}") });
    let text = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())? + "\n";
    std::fs::write(&manifest_path, text).map_err(|e| format!("{}: {e}", manifest_path.display()))?;
    Ok(crc)
}

/// An 8-bit indexed PNG of `pens`: palette entry 15 fully transparent, the
/// rest opaque.
pub fn encode_png(pens: &Pens, colours: &[u32; 16]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, pens.width as u32, pens.height as u32);
        enc.set_color(png::ColorType::Indexed);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_palette(colours.iter().flat_map(|c| [(c >> 16) as u8, (c >> 8) as u8, *c as u8]).collect::<Vec<u8>>());
        let mut alpha = vec![255u8; 16];
        alpha[TRANSPARENT as usize] = 0;
        enc.set_trns(alpha);
        let mut writer = enc.write_header().map_err(|e| e.to_string())?;
        writer.write_image_data(&pens.pens).map_err(|e| e.to_string())?;
    }
    Ok(out)
}

/// The pens of an indexed PNG (any bit depth), each index a pen 0..=15.
pub fn decode_png(bytes: &[u8]) -> Result<Pens, String> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::IDENTITY);
    let mut reader = decoder.read_info().map_err(|e| format!("not a PNG: {e}"))?;
    let (width, height, colour, depth) = {
        let info = reader.info();
        (info.width as usize, info.height as usize, info.color_type, info.bit_depth as usize)
    };
    if colour != png::ColorType::Indexed {
        return Err(format!("a {colour:?} PNG, not an indexed one"));
    }
    let mut buf = vec![0; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut buf).map_err(|e| format!("not a PNG: {e}"))?;
    let mut pens = Vec::with_capacity(width * height);
    for y in 0..height {
        let line = &buf[y * frame.line_size..(y + 1) * frame.line_size];
        for x in 0..width {
            let bit = x * depth;
            let byte = line[bit / 8];
            let pen = (byte >> (8 - depth - bit % 8)) & ((1u16 << depth) - 1) as u8;
            if pen > TRANSPARENT {
                return Err(format!("pixel {x},{y} is index {pen}, past the 16 pens"));
            }
            pens.push(pen);
        }
    }
    Ok(Pens { width, height, pens })
}
