//! Ticket #202 (`ST10`): the host lists the `jukebox` folder for the
//! Jukebox screen, and the engine never touches it. The list is the names of
//! the formats the host plays (the engine's one list of extensions), in name
//! order, each once.

use std::path::{Path, PathBuf};

use sf_host::jukebox::{folders, list_tracks};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sf1-port-track-listing-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn touch(dir: &Path, names: &[&str]) {
    for name in names {
        std::fs::write(dir.join(name), b"x").unwrap();
    }
}

#[test]
fn only_files_of_the_supported_formats_are_listed_in_name_order() {
    let dir = scratch("one");
    touch(&dir, &["zeta.flac", "Alpha.ogg", "beta.WAV", "song.mp3", "notes.txt", "cover.png", "noext", ".hidden.ogg", "._Alpha.ogg"]);
    // A folder named like a track is not a file, and nothing inside it is listed.
    std::fs::create_dir_all(dir.join("folder.ogg")).unwrap();
    touch(&dir.join("folder.ogg"), &["inside.ogg"]);
    assert_eq!(list_tracks(&[dir.clone()]), ["Alpha.ogg", "beta.WAV", "zeta.flac"]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn tracker_modules_are_listed_only_in_a_build_that_plays_them() {
    let dir = scratch("modules");
    touch(&dir, &["Song.XM", "b.mod", "c.s3m", "d.it", "a.ogg", "notes.txt"]);
    let listed = list_tracks(&[dir.clone()]);
    if cfg!(feature = "tracker") {
        assert_eq!(listed, ["a.ogg", "b.mod", "c.s3m", "d.it", "Song.XM"]);
    } else {
        assert_eq!(listed, ["a.ogg"], "a build with no tracker player never offers a module");
    }
    let formats = sf_host::track::supported_extensions();
    assert!(["ogg", "wav", "flac"].iter().all(|e| formats.iter().any(|f| f == e)));
    assert_eq!(formats.iter().any(|f| f == "xm"), cfg!(feature = "tracker"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_missing_folder_lists_nothing_and_two_folders_list_each_name_once() {
    let (a, b) = (scratch("a"), scratch("b"));
    touch(&a, &["one.ogg", "two.wav"]);
    touch(&b, &["two.wav", "three.flac"]);
    let gone = a.join("not-there");
    assert_eq!(list_tracks(&[gone.clone()]), Vec::<String>::new());
    assert_eq!(list_tracks(&[gone, a.clone(), b.clone()]), ["one.ogg", "three.flac", "two.wav"]);
    for dir in [a, b] {
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[test]
fn the_folder_is_beside_the_rom_set() {
    let roms = scratch("roms").join("sf");
    let beside = folders(&[roms.as_path()]);
    assert_eq!(beside, [roms.parent().unwrap().join("jukebox")]);
    touch(&{
        std::fs::create_dir_all(&beside[0]).unwrap();
        beside[0].clone()
    }, &["a.ogg"]);
    assert_eq!(list_tracks(&beside), ["a.ogg"]);
    let _ = std::fs::remove_dir_all(roms.parent().unwrap());
}
