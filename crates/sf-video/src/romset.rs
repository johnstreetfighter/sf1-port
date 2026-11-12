//! ROM region assembly from the local, verified ROM set.
//!
//! Region membership and offsets come from the committed manifest
//! (`tools/reference/rom-manifest.json`), which mirrors ROM_START(sf) in
//! the pinned MAME source. The regions mirror MAME's memory regions:
//! gfx1 (background b tiles), gfx2 (background m tiles), gfx3 (sprites),
//! gfx4 (characters), tilerom (background tilemaps), plus the 68000 program
//! region (maincpu), whose tables and frame records the native gameplay
//! translation reads as data.

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use serde::Deserialize;

#[derive(Debug)]
pub struct RomsetError(pub String);

impl fmt::Display for RomsetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cannot load ROM regions: {}", self.0)
    }
}

impl std::error::Error for RomsetError {}

#[derive(Deserialize)]
struct Manifest {
    #[serde(default)]
    rom_set: String,
    #[serde(default)]
    description: String,
    entries: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    name: String,
    region: String,
    offset: u64,
    size: u64,
    #[serde(default)]
    crc32: String,
}

/// The decoded graphics ROM regions and the program-data region.
pub struct Assets {
    /// 0x000000-0x05ffff, assembled by [`load_program_rom`]. Read as data
    /// only (move tables, frame records, damage tables); never executed.
    pub maincpu: Vec<u8>,
    pub gfx1: Vec<u8>,
    pub gfx2: Vec<u8>,
    pub gfx3: Vec<u8>,
    pub gfx4: Vec<u8>,
    pub tilerom: Vec<u8>,
}

/// The validated ROM files assembled into all regions the game needs.
pub struct VerifiedRomSet {
    pub files: usize,
    pub assets: Assets,
    pub audio2: Vec<u8>,
    pub audiocpu: Vec<u8>,
}

/// Validate each file once, then assemble the regions from those same bytes.
/// The reader receives each required name and its declared size.
pub fn load_verified_set(
    manifest_json: &str,
    mut read: impl FnMut(&str, u64) -> Result<Vec<u8>, RomsetError>,
) -> Result<VerifiedRomSet, RomsetError> {
    let manifest = parse_manifest(manifest_json)?;
    let mut files = BTreeMap::new();
    for entry in &manifest.entries {
        let bytes = read(&entry.name, entry.size)?;
        check_entry(entry, &bytes)?;
        files.insert(entry.name.as_str(), bytes);
    }
    let mut part = |entry: &Entry| Ok(&files[entry.name.as_str()]);
    Ok(VerifiedRomSet {
        files: manifest.entries.len(),
        assets: assets(&manifest, &mut part)?,
        audio2: region(&manifest.entries, "audio2", &mut part)?,
        audiocpu: region(&manifest.entries, "audiocpu", &mut part)?,
    })
}

fn read_part(dir: &Path, part: &Entry) -> Result<Vec<u8>, RomsetError> {
    std::fs::read(dir.join(&part.name))
        .map_err(|e| RomsetError(format!("{}: {e} (ROMs stay local; see docs)", part.name)))
}

fn region<B: AsRef<[u8]>>(
    entries: &[Entry],
    name: &str,
    read: &mut impl FnMut(&Entry) -> Result<B, RomsetError>,
) -> Result<Vec<u8>, RomsetError> {
    let mut parts: Vec<&Entry> = entries.iter().filter(|e| e.region == name).collect();
    parts.sort_by_key(|e| e.offset);
    let size = parts.iter().map(|e| e.offset + e.size).max().unwrap_or(0) as usize;
    let mut region = vec![0u8; size];
    for part in parts {
        let bytes = read(part)?;
        let bytes = bytes.as_ref();
        region[part.offset as usize..part.offset as usize + bytes.len()].copy_from_slice(bytes);
    }
    Ok(region)
}

/// Assemble one contiguous ROM region by its manifest name (for example
/// `audio2`, the samples CPU's program and ADPCM data, or `audiocpu`).
pub fn load_region(
    rom_dir: impl AsRef<Path>,
    manifest_path: impl AsRef<Path>,
    name: &str,
) -> Result<Vec<u8>, RomsetError> {
    load_region_from_manifest(rom_dir, &read_manifest(manifest_path.as_ref())?, name)
}

/// [`load_region`] with the manifest's JSON text in hand (the game embeds it).
pub fn load_region_from_manifest(
    rom_dir: impl AsRef<Path>,
    manifest_json: &str,
    name: &str,
) -> Result<Vec<u8>, RomsetError> {
    region(&parse_manifest(manifest_json)?.entries, name, &mut |part| read_part(rom_dir.as_ref(), part))
}

fn read_manifest(path: &Path) -> Result<String, RomsetError> {
    std::fs::read_to_string(path).map_err(|e| RomsetError(format!("{}: {e}", path.display())))
}

fn parse_manifest(text: &str) -> Result<Manifest, RomsetError> {
    serde_json::from_str(text).map_err(|e| RomsetError(format!("bad manifest: {e}")))
}

/// The set's name and description from the manifest's JSON text (`sf`,
/// "US set 1 (MAME set sf), six-button controls").
pub fn set_name(manifest_json: &str) -> Result<(String, String), RomsetError> {
    let manifest = parse_manifest(manifest_json)?;
    Ok((manifest.rom_set, manifest.description))
}

/// Check every file the manifest lists in `rom_dir`: present, its size,
/// and its CRC-32. Returns how many files matched; the error names the
/// first that did not (or the folder, when it is not there).
pub fn check_set(rom_dir: impl AsRef<Path>, manifest_json: &str) -> Result<usize, RomsetError> {
    let manifest = parse_manifest(manifest_json)?;
    let rom_dir = rom_dir.as_ref();
    if !rom_dir.is_dir() {
        return Err(RomsetError("the folder does not exist".to_owned()));
    }
    for entry in &manifest.entries {
        let bytes = std::fs::read(rom_dir.join(&entry.name))
            .map_err(|_| RomsetError(format!("{} is missing", entry.name)))?;
        check_entry(entry, &bytes)?;
    }
    Ok(manifest.entries.len())
}

fn check_entry(entry: &Entry, bytes: &[u8]) -> Result<(), RomsetError> {
    if bytes.len() as u64 != entry.size {
        return Err(RomsetError(format!("{} is {} bytes, not {}", entry.name, bytes.len(), entry.size)));
    }
    if !entry.crc32.is_empty() {
        let crc = format!("{:08x}", crc32(bytes));
        if !crc.eq_ignore_ascii_case(&entry.crc32) {
            return Err(RomsetError(format!("{} has CRC-32 {crc}, not {}", entry.name, entry.crc32)));
        }
    }
    Ok(())
}

/// The file names and uncompressed sizes in manifest order.
pub fn file_sizes(manifest_json: &str) -> Result<Vec<(String, u64)>, RomsetError> {
    Ok(parse_manifest(manifest_json)?.entries.into_iter().map(|e| (e.name, e.size)).collect())
}

/// The files the manifest lists, in its order.
pub fn file_names(manifest_json: &str) -> Result<Vec<String>, RomsetError> {
    Ok(parse_manifest(manifest_json)?.entries.into_iter().map(|e| e.name).collect())
}

/// CRC-32 (IEEE 802.3, reflected), the checksum MAME and zip list.
pub fn crc32(bytes: &[u8]) -> u32 {
    const TABLE: [u32; 256] = {
        let mut table = [0u32; 256];
        let mut i = 0;
        while i < 256 {
            let mut c = i as u32;
            let mut k = 0;
            while k < 8 {
                c = if c & 1 != 0 { 0xedb8_8320 ^ (c >> 1) } else { c >> 1 };
                k += 1;
            }
            table[i] = c;
            i += 1;
        }
        table
    };
    !bytes
        .iter()
        .fold(!0u32, |c, &b| TABLE[((c ^ b as u32) & 0xff) as usize] ^ (c >> 8))
}

/// Assemble the graphics regions from a ROM directory, guided by the
/// manifest (which file goes to which region offset).
pub fn load_assets(
    rom_dir: impl AsRef<Path>,
    manifest_path: impl AsRef<Path>,
) -> Result<Assets, RomsetError> {
    load_assets_from_manifest(rom_dir, &read_manifest(manifest_path.as_ref())?)
}

/// [`load_assets`] with the manifest's JSON text in hand (the game embeds it).
pub fn load_assets_from_manifest(
    rom_dir: impl AsRef<Path>,
    manifest_json: &str,
) -> Result<Assets, RomsetError> {
    assets(&parse_manifest(manifest_json)?, &mut |part| read_part(rom_dir.as_ref(), part))
}

fn assets<B: AsRef<[u8]>>(
    manifest: &Manifest,
    read: &mut impl FnMut(&Entry) -> Result<B, RomsetError>,
) -> Result<Assets, RomsetError> {
    Ok(Assets {
        maincpu: program_rom(manifest, read)?,
        gfx1: region(&manifest.entries, "gfx1", read)?,
        gfx2: region(&manifest.entries, "gfx2", read)?,
        gfx3: region(&manifest.entries, "gfx3", read)?,
        gfx4: region(&manifest.entries, "gfx4", read)?,
        tilerom: region(&manifest.entries, "tilerom", read)?,
    })
}

/// Assemble the 68000 program region (`maincpu`, 0x000000-0x05ffff) from a
/// ROM directory. ROM_START(sf) loads it with ROM_LOAD16_BYTE: an entry at
/// an even manifest offset supplies the high (even) bytes of each word, an
/// entry at offset+1 the low (odd) bytes. The native translation reads
/// original game data (frame records, tables) from this region; the code in
/// it is never executed.
pub fn load_program_rom(
    rom_dir: impl AsRef<Path>,
    manifest_path: impl AsRef<Path>,
) -> Result<Vec<u8>, RomsetError> {
    let manifest = parse_manifest(&read_manifest(manifest_path.as_ref())?)?;
    program_rom(&manifest, &mut |part| read_part(rom_dir.as_ref(), part))
}

fn program_rom<B: AsRef<[u8]>>(
    manifest: &Manifest,
    read: &mut impl FnMut(&Entry) -> Result<B, RomsetError>,
) -> Result<Vec<u8>, RomsetError> {
    let parts: Vec<&Entry> = manifest
        .entries
        .iter()
        .filter(|e| e.region == "maincpu")
        .collect();
    let size = parts
        .iter()
        .map(|e| (e.offset & !1) + 2 * e.size)
        .max()
        .unwrap_or(0) as usize;
    let mut region = vec![0u8; size];
    for part in parts {
        let bytes = read(part)?;
        let base = (part.offset & !1) as usize;
        let lane = (part.offset & 1) as usize;
        for (i, b) in bytes.as_ref().iter().enumerate() {
            region[base + 2 * i + lane] = *b;
        }
    }
    Ok(region)
}
