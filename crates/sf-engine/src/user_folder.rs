//! The player's folder: settings, saves, the log, drop-in looks and the ROM
//! folder all live in one `sf1-port` folder per user.
//!
//! - Windows: `%APPDATA%\sf1-port`.
//! - Linux and other Unix hosts: `$XDG_DATA_HOME/sf1-port`, which is
//!   `~/.local/share/sf1-port` unless the desktop names another data home.
//!
//! A package update replaces the files beside the executable and leaves this
//! folder alone. With none of the variables set the folder is `sf1-port` in
//! the working folder.

use std::ffi::OsString;
use std::path::PathBuf;

/// The folder's name inside the platform's per-user base.
pub const FOLDER: &str = "sf1-port";

/// Which rule finds the per-user base.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    Windows,
    Unix,
}

impl Platform {
    /// The platform this build runs on.
    pub const fn native() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else {
            Self::Unix
        }
    }
}

/// The player's folder on this machine.
pub fn app_dir() -> PathBuf {
    resolve(Platform::native(), |name| std::env::var_os(name))
}

/// The player's folder under `platform`'s rule, reading the environment
/// through `var`.
pub fn resolve(platform: Platform, var: impl Fn(&str) -> Option<OsString>) -> PathBuf {
    let set = |name: &str| var(name).filter(|value| !value.is_empty()).map(PathBuf::from);
    let base = match platform {
        Platform::Windows => set("APPDATA"),
        // The XDG base directory specification: a relative path is invalid
        // and the default applies.
        Platform::Unix => set("XDG_DATA_HOME")
            .filter(|path| path.to_string_lossy().starts_with('/'))
            .or_else(|| set("HOME").map(|home| home.join(".local").join("share"))),
    };
    base.unwrap_or_else(|| PathBuf::from(".")).join(FOLDER)
}
