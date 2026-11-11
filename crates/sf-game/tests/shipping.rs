//! Ticket #161 (`FE11`): the shipping shape. The ROM set is found in each
//! of the three folders in order; with none, `RETRY` on the ROM-missing
//! screen finds files added since and carries on into the attract; a first
//! launch on a clean app-data folder writes the three files with the spec's
//! defaults; a file that fails to parse is logged; the log is overwritten
//! at every launch. The ROM checks need the local ROMs and skip without
//! them.

use std::path::{Path, PathBuf};

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::front::rom_missing::RomMissing;
use sf_engine::front::settings::Settings;
use sf_engine::front::{FrameInputs, GameSession, MenuItem, Transition};
use sf_host::bindings::BindingsTable;
use sf_host::files;
use sf_host::log::Log;
use sf_host::roms;

const ROMS: &str = "../../tools/reference/local/roms/sf";

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sf1-port-shipping-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn copy_set(to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for name in roms::file_names() {
        std::fs::copy(Path::new(ROMS).join(&name), to.join(&name)).unwrap();
    }
}

#[test]
fn the_set_is_found_in_each_folder_in_order() {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return;
    }
    let dir = scratch("lookup");
    let (exe, app, named) = (dir.join("exe"), dir.join("app").join("sf1-port"), dir.join("named"));
    let candidates = roms::candidates(None, Some(&exe), &app, Some(&named.display().to_string()));
    let folders: Vec<_> = candidates.iter().map(|c| c.path.clone().unwrap()).collect();
    assert_eq!(folders, [exe.join("roms").join("sf"), app.join("roms").join("sf"), named.clone()]);

    let none = roms::find(&candidates).err().expect("no set yet");
    assert_eq!(none.len(), 3);
    assert!(none.iter().all(|l| l.problem == "the folder does not exist"));
    // Each folder alone, last to first, then every folder at once: the
    // earliest folder that has the set wins.
    for folder in folders.iter().rev() {
        copy_set(folder);
        assert_eq!(&roms::find(&candidates).ok().unwrap().folder, folder);
    }
    // `--roms` comes first.
    let arg = roms::candidates(Some(Path::new(ROMS)), Some(&exe), &app, None);
    assert_eq!(roms::find(&arg).ok().unwrap().folder, Path::new(ROMS));
    // An unset settings folder is reported as such.
    let unset = roms::candidates(None, None, &dir.join("empty"), None);
    let looked = roms::find(&unset).err().unwrap();
    assert_eq!(looked[1].folder, "rom_folder in settings.toml");
    assert_eq!(looked[1].problem, "not set");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn retry_finds_files_added_since_and_carries_on_into_the_attract() {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return;
    }
    let dir = scratch("retry");
    let app = dir.join("sf1-port");
    let candidates = roms::candidates(None, Some(&dir.join("exe")), &app, None);
    let looked = roms::find(&candidates).err().expect("no set yet");
    let (set, description) = roms::set_name();
    let mut screen = RomMissing::open(&set.to_uppercase(), &description.to_uppercase(), looked);
    let confirm = FrameInputs {
        menu: sf_engine::front::MenuKeys { confirm: true, ..Default::default() },
        ..Default::default()
    };
    // RETRY with nothing added: still missing, the results refreshed.
    assert_eq!(screen.step(confirm), Some(Transition::Choose(MenuItem::Retry)));
    let again = roms::find(&candidates).err().expect("still none");
    screen.update(again);
    screen.step(FrameInputs::default());
    // The files are added to the app-data folder; RETRY finds them.
    copy_set(&app.join("roms").join("sf"));
    assert_eq!(screen.step(confirm), Some(Transition::Choose(MenuItem::Retry)));
    let found = roms::find(&candidates).expect("found on retry");
    assert_eq!(found.folder, app.join("roms").join("sf"));
    assert_eq!(found.files, 44);
    let mut session = GameSession::new(found.assets, MatchConfig::default());
    for _ in 0..60 {
        assert_eq!(session.step(FrameInputs::default()).screen, "attract");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_first_launch_writes_the_three_files_with_the_defaults() {
    let dir = scratch("first").join("sf1-port");
    let mut log = Log::create(&files::log_path(&dir));
    let first = files::load(&dir, &mut log);
    for path in [files::settings_path(&dir), files::match_config_path(&dir), files::bindings_path(&dir)] {
        assert!(path.is_file(), "{} written", path.display());
    }
    let s = &first.settings;
    // Spec #203 (amending front-end story 5): borderless fullscreen with the
    // ARCADE CABINET look at true 4:3, vsync on and no frame delay; 3x is
    // what WINDOWED opens at. The gameplay defaults are untouched (below).
    assert_eq!((s.scale, s.fullscreen, s.vsync, s.frame_delay_ms), (3, true, true, 0));
    assert_eq!((s.aspect, s.crt_look.as_str()), (sf_engine::front::settings::Aspect::True4x3, "arcade_cabinet"));
    assert_eq!([s.master, s.fm, s.samples], [10, 10, 10]);
    assert_eq!(first.config, MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul));
    assert_eq!(first.bindings, BindingsTable::default(), "keyboard for both players");
    let text = std::fs::read_to_string(files::log_path(&dir)).unwrap();
    assert_eq!(text.matches("settings: wrote the defaults to ").count(), 3, "{text}");

    // The second launch reads them back, writes nothing new, logs nothing.
    let mut log = Log::create(&files::log_path(&dir));
    let second = files::load(&dir, &mut log);
    assert_eq!(second.settings, first.settings);
    assert_eq!(second.config, first.config);
    assert_eq!(second.bindings, first.bindings);
    assert_eq!(std::fs::read_to_string(files::log_path(&dir)).unwrap(), "", "the log is overwritten");
    let _ = std::fs::remove_dir_all(dir.parent().unwrap());
}

#[test]
fn a_file_that_fails_to_parse_is_logged_and_left_alone() {
    let dir = scratch("parse").join("sf1-port");
    let saved = Settings { scale: 2, ..Settings::default() };
    saved.save(&files::settings_path(&dir)).unwrap();
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(files::match_config_path(&dir), "preset = \"no such preset\"\n").unwrap();
    let mut log = Log::create(&files::log_path(&dir));
    let loaded = files::load(&dir, &mut log);
    log.line("untranslated", "frame 1: untranslated original branches [0x12345] during a fight");
    assert_eq!(loaded.settings, saved, "the good file is read");
    assert_eq!(loaded.config, MatchConfig::default(), "the broken one falls back");
    assert_eq!(
        std::fs::read_to_string(files::match_config_path(&dir)).unwrap(),
        "preset = \"no such preset\"\n",
        "and is left for the player to see"
    );
    let text = std::fs::read_to_string(files::log_path(&dir)).unwrap();
    let kinds: Vec<&str> = text
        .lines()
        .map(|l| l.split_once("] ").unwrap().1.split_once(':').unwrap().0)
        .collect();
    // The settings file read, the match config's parse failure and
    // the note that a copy of it was kept, the missing bindings file written,
    // then the session's notice.
    assert_eq!(kinds, ["parse", "parse", "settings", "untranslated"], "{text}");
    assert!(files::match_config_path(&dir).with_extension("toml.rejected").exists(), "a copy is kept");
    assert!(text.contains("match_config.toml"), "{text}");
    let _ = std::fs::remove_dir_all(dir.parent().unwrap());
}
