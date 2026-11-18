//! Where the game keeps a player's settings, saves, log, drop-in looks and
//! ROM folder on each platform (#125).

use std::ffi::OsString;
use std::path::PathBuf;

use sf_engine::user_folder::{resolve, Platform};

fn with(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
    let vars: Vec<(String, OsString)> = vars.iter().map(|(k, v)| ((*k).to_owned(), OsString::from(v))).collect();
    move |name| vars.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone())
}

#[test]
fn windows_keeps_the_folder_under_appdata() {
    let folder = resolve(Platform::Windows, with(&[("APPDATA", r"C:\Users\ryu\AppData\Roaming")]));
    assert_eq!(folder, PathBuf::from(r"C:\Users\ryu\AppData\Roaming").join("sf1-port"));
}

#[test]
fn windows_ignores_the_linux_variables() {
    let folder = resolve(
        Platform::Windows,
        with(&[("XDG_DATA_HOME", "/data"), ("HOME", "/home/ryu"), ("APPDATA", r"C:\Roaming")]),
    );
    assert_eq!(folder, PathBuf::from(r"C:\Roaming").join("sf1-port"));
}

#[test]
fn linux_uses_the_data_home_the_desktop_names() {
    let folder = resolve(Platform::Unix, with(&[("XDG_DATA_HOME", "/mnt/saves"), ("HOME", "/home/deck")]));
    assert_eq!(folder, PathBuf::from("/mnt/saves").join("sf1-port"));
}

#[test]
fn linux_falls_back_to_local_share_in_the_home_folder() {
    let folder = resolve(Platform::Unix, with(&[("HOME", "/home/deck")]));
    assert_eq!(folder, PathBuf::from("/home/deck").join(".local").join("share").join("sf1-port"));
}

#[test]
fn linux_ignores_an_empty_or_relative_data_home() {
    // The XDG base directory specification calls a relative path invalid.
    for bad in ["", "saves", "./saves"] {
        let folder = resolve(Platform::Unix, with(&[("XDG_DATA_HOME", bad), ("HOME", "/home/deck")]));
        assert_eq!(folder, PathBuf::from("/home/deck").join(".local").join("share").join("sf1-port"), "{bad:?}");
    }
}

#[test]
fn linux_ignores_appdata() {
    let folder = resolve(Platform::Unix, with(&[("APPDATA", r"C:\Roaming"), ("HOME", "/home/deck")]));
    assert_eq!(folder, PathBuf::from("/home/deck").join(".local").join("share").join("sf1-port"));
}

#[test]
fn a_platform_with_nothing_set_uses_the_working_folder() {
    for platform in [Platform::Windows, Platform::Unix] {
        assert_eq!(resolve(platform, with(&[])), PathBuf::from(".").join("sf1-port"));
    }
}
