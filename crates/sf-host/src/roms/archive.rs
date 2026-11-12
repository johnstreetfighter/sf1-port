//! Read only the manifest's ROM entries from an archive.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use sevenz_rust2::{ArchiveReader, EntryDecision, Password};
use sf_video::romset::{self, RomsetError, VerifiedRomSet};

/// Bound the decoder's dictionaries and archive headers for local ROM sets.
const SEVEN_Z_MEMORY_KIB: usize = 128 * 1024;

pub(super) fn load(path: &Path, manifest: &str) -> Result<VerifiedRomSet, String> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    let file = File::open(path).map_err(|error| error.to_string())?;
    if extension.eq_ignore_ascii_case("zip") {
        load_zip(file, manifest)
    } else if extension.eq_ignore_ascii_case("7z") {
        load_7z(file, manifest)
    } else {
        Err("choose a ROM folder, a .zip archive, or a .7z archive".to_owned())
    }
}

fn load_zip(file: File, manifest: &str) -> Result<VerifiedRomSet, String> {
    let mut archive = zip::ZipArchive::new(file).map_err(|error| error.to_string())?;
    romset::load_verified_set(manifest, |name, expected| {
        // A MAME set uses top-level entries. A user-created archive may
        // also preserve the enclosing sf folder.
        let index = archive
            .index_for_name(name)
            .or_else(|| archive.index_for_name(&format!("sf/{name}")))
            .or_else(|| archive.index_for_name(&format!("./{name}")))
            .or_else(|| archive.index_for_name(&format!("./sf/{name}")))
            .ok_or_else(|| RomsetError(format!("{name} is missing")))?;
        let mut entry = archive
            .by_index(index)
            .map_err(|error| RomsetError(format!("{name}: {error}")))?;
        if !entry.is_file() {
            return Err(RomsetError(format!("{name} is not a file")));
        }
        let size = entry.size();
        read_entry(&mut entry, name, size, expected)
    })
    .map_err(|error| error.0)
}

fn load_7z(file: File, manifest: &str) -> Result<VerifiedRomSet, String> {
    let mut archive =
        ArchiveReader::new_with_memory_limit(file, Password::empty(), SEVEN_Z_MEMORY_KIB)
            .map_err(|error| error.to_string())?;
    let required: BTreeMap<_, _> = romset::file_sizes(manifest)
        .map_err(|error| error.0)?
        .into_iter()
        .collect();
    let mut files = BTreeMap::new();

    archive
        .for_each_entries_with_decision(|entry, reader| {
            let name = entry.name().strip_prefix("./").unwrap_or(entry.name());
            let name = name.strip_prefix("sf/").unwrap_or(name);
            if let Some((key, &expected)) = required.get_key_value(name) {
                if !entry.is_directory() && !files.contains_key(key) {
                    let bytes = read_entry(reader, name, entry.size(), expected)
                        .map_err(|error| sevenz_rust2::Error::Other(error.0.into()))?;
                    files.insert(key.clone(), bytes);
                }
            }
            // Entries later in a solid block depend on the preceding data.
            // Stop when every required file has been read.
            Ok(if files.len() == required.len() {
                EntryDecision::Stop
            } else {
                EntryDecision::Continue
            })
        })
        .map_err(|error| error.to_string())?;

    romset::load_verified_set(manifest, |name, _| {
        files
            .remove(name)
            .ok_or_else(|| RomsetError(format!("{name} is missing")))
    })
    .map_err(|error| error.0)
}

fn read_entry(
    reader: &mut dyn Read,
    name: &str,
    size: u64,
    expected: u64,
) -> Result<Vec<u8>, RomsetError> {
    if size != expected {
        return Err(RomsetError(format!(
            "{name} is {size} bytes, not {expected}"
        )));
    }
    let mut bytes = Vec::with_capacity(expected as usize);
    reader
        .take(expected + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| RomsetError(format!("{name}: {error}")))?;
    Ok(bytes)
}
