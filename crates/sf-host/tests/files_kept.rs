//! Ticket #200 (`ST8`): a settings file that fails to load is not lost to the
//! game's next save: a copy is kept beside it before anything can overwrite
//! it, and a file whose only problem is a line of the Jukebox's tables loads
//! with the rest of its settings.

use sf_host::files;
use sf_host::log::Log;

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("sf1-port-files-kept-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_settings_file_that_fails_to_parse_is_kept_as_a_copy() {
    let dir = scratch("bad");
    let path = files::settings_path(&dir);
    std::fs::write(&path, "scale = eleven\n[jukebox]\nglobal = \"mine.ogg\"\n").unwrap();
    let mut log = Log::create(&files::log_path(&dir));
    let loaded = files::load(&dir, &mut log);
    assert_eq!(loaded.settings, Default::default(), "the defaults are used");
    let kept = std::fs::read_to_string(dir.join("settings.toml.rejected")).unwrap();
    assert!(kept.contains("mine.ogg"), "the player's file survives: {kept}");
    assert!(std::fs::read_to_string(files::log_path(&dir)).unwrap().contains("kept a copy"));
}

#[test]
fn a_bad_jukebox_line_keeps_the_other_settings_and_the_file() {
    let dir = scratch("line");
    std::fs::write(files::settings_path(&dir), "master = 4\n[jukebox.stages]\nretsu = nope\n\"geki\" = \"g.ogg\"\n").unwrap();
    let mut log = Log::create(&files::log_path(&dir));
    let loaded = files::load(&dir, &mut log);
    assert_eq!(loaded.settings.master, 4);
    assert_eq!(loaded.settings.jukebox.stage("geki"), Some("g.ogg"));
    assert!(dir.join("settings.toml.rejected").exists(), "the file as written is kept");
    assert!(std::fs::read_to_string(files::log_path(&dir)).unwrap().contains("skipped"));
}
