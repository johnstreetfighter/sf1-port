//! Ticket #200 (`ST8`): the Jukebox's tracks in the settings file, until the
//! Jukebox screen (ST10) sets them in the game. They live in three tables,
//! `[jukebox]` (`global`), `[jukebox.stages]` and `[jukebox.fighters]`, each
//! entry `"name" = "file"`.

use sf_engine::front::settings::Settings;
use sf_engine::jukebox::Jukebox;

fn set() -> Settings {
    let mut jukebox = Jukebox::default();
    jukebox.set_global("everywhere.ogg");
    jukebox.set_stage("retsu", "temple.flac");
    jukebox.set_stage("test-stage", "sub folder\\with \"quotes\" # and hash.wav");
    jukebox.set_fighter("Ken", "ken.ogg");
    jukebox.set_fighter("Evil Ryu", "evil ryu.wav");
    Settings { jukebox, master: 7, ..Settings::default() }
}

#[test]
fn the_tracks_round_trip_through_the_settings_file() {
    let settings = set();
    let text = settings.to_toml();
    assert!(text.contains("[jukebox]\nglobal = \"everywhere.ogg\"\n"), "{text}");
    assert!(text.contains("[jukebox.stages]\n\"retsu\" = \"temple.flac\"\n"), "{text}");
    assert!(text.contains("[jukebox.fighters]\n\"evil ryu\" = \"evil ryu.wav\"\n"), "{text}");
    let back = Settings::from_toml(&text).unwrap();
    assert_eq!(back, settings);
    assert_eq!(back.jukebox.stage("TEST-STAGE"), Some("sub folder\\with \"quotes\" # and hash.wav"));
    assert_eq!(back.jukebox.fighter("Evil Ryu"), Some("evil ryu.wav"));
    // And through a file on disk.
    let path = std::env::temp_dir().join(format!("sf1-port-jukebox-settings-{}", std::process::id())).join("settings.toml");
    settings.save(&path).unwrap();
    let (loaded, diag) = Settings::load_or_default(&path);
    assert_eq!((diag, loaded), (None, settings));
}

#[test]
fn a_default_file_has_no_jukebox_tables() {
    let text = Settings::default().to_toml();
    assert!(!text.contains("jukebox"), "{text}");
    assert_eq!(Settings::from_toml(&text).unwrap().jukebox, Jukebox::default());
}

#[test]
fn keys_may_be_bare_or_quoted_and_comments_are_skipped() {
    let text = "master = 3\n[jukebox.stages]\n# Gen's temple\ngen = \"a.ogg\" # the file\n\"Mike\" = \"b.ogg\"\n";
    let s = Settings::from_toml(text).unwrap();
    assert_eq!((s.master, s.jukebox.stage("gen"), s.jukebox.stage("mike")), (3, Some("a.ogg"), Some("b.ogg")));
}

#[test]
fn a_bad_table_or_entry_is_refused_with_its_line() {
    for (text, want) in [
        ("[jukebox.nowhere]\n", "line 1: unknown table"),
        ("[jukebox\n", "line 1: expected ']'"),
        ("[jukebox]\nglobal = \"\"\n", "line 2: the track for 'global' names no file"),
        ("[jukebox]\nsoon = \"a.ogg\"\n", "line 2: unknown key 'soon'"),
        ("[jukebox.fighters]\nKen = a.ogg\n", "line 2: expected a name and a quoted file"),
        ("[jukebox.fighters]\nKen\n", "line 2: expected 'key = value'"),
        ("[jukebox.stages]\n\"\" = \"a.ogg\"\n", "line 2: a stage track names no stage"),
    ] {
        let e = Settings::from_toml(text).unwrap_err();
        assert!(e.contains(want), "{text:?}: {e}");
    }
}

#[test]
fn a_bad_jukebox_line_costs_only_that_line_when_the_game_loads_the_file() {
    let text = "master = 3\nscale = 2\n[jukebox]\nglobal = \"a.ogg\"\n[jukebox.stages]\nretsu = a.ogg\n\"geki\" = \"g.ogg\"\n\"GEKI\" = \"g2.ogg\"\n[jukebox.nowhere]\nx = \"y\"\n[jukebox.fighters]\nKen = \"k.ogg\"\n";
    // The strict reader still refuses the file.
    assert!(Settings::from_toml(text).is_err());
    let (s, warnings) = Settings::from_toml_lenient(text).unwrap();
    assert_eq!((s.master, s.scale), (3, 2), "the other settings stand");
    assert_eq!(s.jukebox.global(), Some("a.ogg"));
    assert_eq!(s.jukebox.stage("retsu"), None, "the bad line is skipped");
    assert_eq!(s.jukebox.stage("geki"), Some("g2.ogg"), "a key set twice: the later line wins");
    assert_eq!(s.jukebox.fighter("ken"), Some("k.ogg"), "lines after a skipped table's own header are read");
    assert_eq!(warnings.len(), 3, "{warnings:?}");
    assert!(warnings[0].contains("line 6") && warnings[0].contains("skipped"), "{warnings:?}");
    assert!(warnings[1].contains("line 8") && warnings[1].contains("set twice"), "{warnings:?}");
    assert!(warnings[2].contains("line 9") && warnings[2].contains("unknown table"), "{warnings:?}");
    // A non-Jukebox problem is still an error.
    assert!(Settings::from_toml_lenient("master = 11\n").is_err());
    // Loaded from a file: the settings come back with the problem as the message.
    let dir = std::env::temp_dir().join(format!("sf1-port-jukebox-lenient-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("settings.toml");
    std::fs::write(&path, text).unwrap();
    let (loaded, diag) = Settings::load_or_default(&path);
    assert_eq!(loaded.master, 3);
    assert!(diag.is_some_and(|d| d.contains("skipped")));
}
