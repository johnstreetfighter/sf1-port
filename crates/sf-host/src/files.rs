//! The game's three files in the app-data folder (spec #150, section 5.7):
//! `match_config.toml`, `bindings.toml` and `settings.toml`. A file that is
//! not there is written with its defaults (so a first launch writes all
//! three); a file that fails to parse is logged and left as it is, its
//! defaults used, and a copy kept beside it as `<name>.rejected` before the
//! game can save over it. (A settings file whose only problem is a line of
//! the Jukebox's tables loads with that line skipped; the same copy is kept.)

use std::path::{Path, PathBuf};

use sf_engine::config::MatchConfig;
use sf_engine::front::settings::Settings;

use crate::bindings::BindingsTable;
use crate::log::Log;

/// `%APPDATA%\sf1-port` (the working folder when `APPDATA` is not set).
pub fn app_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("sf1-port")
}

pub fn settings_path(app_dir: &Path) -> PathBuf {
    app_dir.join("settings.toml")
}

pub fn match_config_path(app_dir: &Path) -> PathBuf {
    app_dir.join("match_config.toml")
}

pub fn bindings_path(app_dir: &Path) -> PathBuf {
    app_dir.join("bindings.toml")
}

pub fn log_path(app_dir: &Path) -> PathBuf {
    app_dir.join("sf-game.log")
}

/// The three files' contents.
pub struct Files {
    pub settings: Settings,
    pub config: MatchConfig,
    pub bindings: BindingsTable,
}

/// Load the three files from `app_dir`, writing the defaults of any that
/// is not there and logging any that fails to parse.
pub fn load(app_dir: &Path, log: &mut Log) -> Files {
    Files {
        settings: one(&settings_path(app_dir), log, Settings::load_or_default, Settings::save),
        config: one(&match_config_path(app_dir), log, MatchConfig::load_or_default, MatchConfig::save),
        bindings: one(&bindings_path(app_dir), log, BindingsTable::load_or_default, BindingsTable::save),
    }
}

fn one<T: Default>(
    path: &Path,
    log: &mut Log,
    load: fn(&Path) -> (T, Option<String>),
    save: fn(&T, &Path) -> Result<(), String>,
) -> T {
    if !path.exists() {
        let value = T::default();
        match save(&value, path) {
            Ok(()) => log.line("settings", &format!("wrote the defaults to {}", path.display())),
            Err(e) => log.line("settings", &e),
        }
        return value;
    }
    let (value, diag) = load(path);
    if let Some(msg) = diag {
        log.line("parse", &msg);
        let mut name = path.as_os_str().to_owned();
        name.push(".rejected");
        match std::fs::copy(path, PathBuf::from(&name)) {
            Ok(_) => log.line("parse", &format!("kept a copy of the file as {}", PathBuf::from(&name).display())),
            Err(e) => log.line("parse", &format!("could not keep a copy of {}: {e}", path.display())),
        }
    }
    value
}

/// The settings in force, for the log's `settings` lines.
pub fn describe(s: &Settings, config: &MatchConfig) -> String {
    format!(
        "scale {}x {}; volumes master {} announcer {} fm {} samples {}; rom_folder {}; preset {}, difficulty {}, round time {}; P1 {}, P2 {}",
        s.scale,
        if s.fullscreen { "fullscreen" } else { "windowed" },
        s.master,
        s.announcer,
        s.fm,
        s.samples,
        s.rom_folder.as_deref().unwrap_or("(not set)"),
        config.preset.display_name(),
        config.difficulty,
        config.round_time,
        s.devices[0].label().to_ascii_lowercase(),
        s.devices[1].label().to_ascii_lowercase(),
    )
}
