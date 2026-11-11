//! Ticket #161 (`FE11`): the ROM set is checked against the manifest
//! before the game uses it. Every file the manifest lists must be there,
//! at its size, with its CRC-32; the error names the first that is not.
//! The set checks need the local ROMs and skip without them.

use std::path::{Path, PathBuf};

use sf_video::romset::{check_set, crc32, file_names, set_name};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = include_str!("../../../tools/reference/rom-manifest.json");

#[test]
fn crc32_is_the_ieee_checksum() {
    assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    assert_eq!(crc32(b""), 0);
}

#[test]
fn the_manifest_names_the_set() {
    let (set, description) = set_name(MANIFEST).unwrap();
    assert_eq!(set, "sf");
    assert_eq!(description, "US set 1 (MAME set sf), six-button controls");
    assert_eq!(file_names(MANIFEST).unwrap().len(), 44);
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sf1-port-romset-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for file in file_names(MANIFEST).unwrap() {
        std::fs::copy(Path::new(ROMS).join(&file), dir.join(&file)).unwrap();
    }
    dir
}

#[test]
fn the_local_set_matches_and_each_fault_is_named() {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return;
    }
    assert_eq!(check_set(ROMS, MANIFEST).unwrap(), 44, "every listed file matches");

    let missing = Path::new(ROMS).join("no-such-folder");
    assert_eq!(check_set(&missing, MANIFEST).unwrap_err().0, "the folder does not exist");

    let dir = scratch("faults");
    assert_eq!(check_set(&dir, MANIFEST).unwrap(), 44, "a copy of the listed files matches");
    // One byte changed: the CRC-32 names the file.
    let first = dir.join("sfd-19.2a");
    let mut bytes = std::fs::read(&first).unwrap();
    bytes[0x100] ^= 0xff;
    std::fs::write(&first, &bytes).unwrap();
    let err = check_set(&dir, MANIFEST).unwrap_err().0;
    assert!(err.starts_with("sfd-19.2a has CRC-32 ") && err.ends_with(", not faaf6255"), "{err}");
    // Cut short: its size.
    std::fs::write(&first, &bytes[..1000]).unwrap();
    assert_eq!(check_set(&dir, MANIFEST).unwrap_err().0, "sfd-19.2a is 1000 bytes, not 65536");
    // Gone.
    std::fs::remove_file(&first).unwrap();
    assert_eq!(check_set(&dir, MANIFEST).unwrap_err().0, "sfd-19.2a is missing");
    let _ = std::fs::remove_dir_all(&dir);
}
