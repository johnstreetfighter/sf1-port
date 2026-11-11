//! ROM input-data validation: the runner must refuse to compare against an
//! incorrect or incomplete local ROM set, with errors that name the entry.

mod common;

use common::{manifest_json, temp_dir, write_file};

#[test]
fn valid_rom_set_passes_verification() {
    let dir = temp_dir("roms-valid");
    let manifest_path = dir.join("manifest.json");
    let roms = dir.join("roms");
    std::fs::create_dir_all(&roms).unwrap();

    let main_bytes = vec![0xabu8; 0x100];
    let sound_bytes = vec![0x01u8; 0x80];
    write_file(
        &dir,
        "manifest.json",
        manifest_json(
            "sf",
            &[
                ("sfd-19.2a", "maincpu", 0, &main_bytes),
                ("sf-02.7k", "audiocpu", 0, &sound_bytes),
            ],
        )
        .as_bytes(),
    );
    write_file(&roms, "sfd-19.2a", &main_bytes);
    write_file(&roms, "sf-02.7k", &sound_bytes);

    sf_compare::roms::verify_roms(&manifest_path, &roms).expect("valid set must verify");
}

#[test]
fn corrupt_rom_names_the_entry_and_the_actual_crc() {
    let dir = temp_dir("roms-corrupt");
    let manifest_path = dir.join("manifest.json");
    let roms = dir.join("roms");
    std::fs::create_dir_all(&roms).unwrap();

    let good = vec![0xabu8; 0x100];
    write_file(
        &dir,
        "manifest.json",
        manifest_json("sf", &[("sfd-19.2a", "maincpu", 0, &good)]).as_bytes(),
    );
    write_file(&roms, "sfd-19.2a", &vec![0xcd_u8; 0x100]);

    let err = sf_compare::roms::verify_roms(&manifest_path, &roms).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("sfd-19.2a"),
        "error must name the entry: {msg}"
    );
    assert!(
        msg.contains("crc32"),
        "error must name the failed check: {msg}"
    );
    assert!(
        msg.contains(&common::crc32_hex(&vec![0xcd_u8; 0x100])),
        "error must report the actual crc32: {msg}"
    );
}

#[test]
fn missing_rom_is_reported_as_absent_input() {
    let dir = temp_dir("roms-missing");
    let roms = dir.join("roms");
    std::fs::create_dir_all(&roms).unwrap();
    let bytes = vec![7u8; 16];
    write_file(
        &dir,
        "manifest.json",
        manifest_json("sf", &[("sf-02.7k", "audiocpu", 0, &bytes)]).as_bytes(),
    );

    let err = sf_compare::roms::verify_roms(dir.join("manifest.json"), &roms).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("sf-02.7k"), "error must name the entry: {msg}");
    assert!(
        msg.contains("missing"),
        "error must say the file is missing: {msg}"
    );
}
