//! Find a ROM folder or archive whose files match the embedded manifest.

use std::path::{Path, PathBuf};

use sf_engine::front::rom_missing::Looked;
use sf_video::romset::{self, Assets, RomsetError, VerifiedRomSet};

mod archive;
#[cfg(windows)]
mod picker;

/// Which kind of ROM source to choose in the missing-ROM screen.
#[derive(Clone, Copy)]
pub enum BrowseKind {
    Folder,
    Archive,
}

/// Show the host's ROM picker. Cancellation returns no path.
pub fn browse(window: &crate::gpu_window::WindowState, kind: BrowseKind) -> Result<Option<PathBuf>, String> {
    #[cfg(windows)]
    {
        picker::browse(window, kind).map_err(|error| error.to_string())
    }
    #[cfg(not(windows))]
    {
        let _ = (window, kind);
        Err("the ROM picker is unavailable on this platform".to_owned())
    }
}

/// The ROM manifest, embedded so the game is one file plus the ROM folder.
pub const MANIFEST: &str = include_str!("../../../tools/reference/rom-manifest.json");

/// A folder or archive to look in. `path` is `None` when the setting is unset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub label: String,
    pub path: Option<PathBuf>,
}

/// The set, loaded from the source that matched.
pub struct RomSet {
    /// The selected directory or archive path.
    pub folder: PathBuf,
    /// How many files matched the manifest.
    pub files: usize,
    pub assets: Assets,
    pub audio2: Vec<u8>,
    pub audiocpu: Vec<u8>,
}

/// The folders, in order: `--roms` when given (a development aid),
/// `roms\sf` beside the executable, `roms\sf` in the app-data folder, then
/// the settings file's `rom_folder`.
pub fn candidates(
    roms_arg: Option<&Path>,
    exe_dir: Option<&Path>,
    app_dir: &Path,
    settings_folder: Option<&str>,
) -> Vec<Candidate> {
    let mut list = Vec::new();
    let named = |path: PathBuf| Candidate {
        label: path.display().to_string(),
        path: Some(path),
    };
    if let Some(dir) = roms_arg {
        list.push(named(dir.to_owned()));
    }
    if let Some(dir) = exe_dir {
        list.push(named(dir.join("roms").join("sf")));
    }
    list.push(named(app_dir.join("roms").join("sf")));
    list.push(match settings_folder {
        Some(dir) => named(PathBuf::from(dir)),
        None => Candidate {
            label: "rom_folder in settings.toml".to_owned(),
            path: None,
        },
    });
    list
}

/// The set's name and description from the manifest.
pub fn set_name() -> (String, String) {
    romset::set_name(MANIFEST).unwrap_or_else(|_| ("sf".to_owned(), String::new()))
}

/// The files the manifest lists.
pub fn file_names() -> Vec<String> {
    romset::file_names(MANIFEST).unwrap_or_default()
}

/// The first candidate whose files match the manifest, loaded; or, when
/// none does, what was wrong in each.
pub fn find(candidates: &[Candidate]) -> Result<RomSet, Vec<Looked>> {
    let mut looked = Vec::new();
    for candidate in candidates {
        let Some(dir) = &candidate.path else {
            looked.push(Looked {
                folder: candidate.label.clone(),
                problem: "not set".to_owned(),
            });
            continue;
        };
        match load(dir) {
            Ok(set) => return Ok(set),
            Err(problem) => looked.push(Looked {
                folder: candidate.label.clone(),
                problem,
            }),
        }
    }
    Err(looked)
}

/// Validate and load a ROM directory, ZIP, or 7z archive.
pub fn load(path: &Path) -> Result<RomSet, String> {
    let VerifiedRomSet { files, assets, audio2, audiocpu } = if path.is_dir() {
        romset::load_verified_set(MANIFEST, |name, _| {
            std::fs::read(path.join(name)).map_err(|_| RomsetError(format!("{name} is missing")))
        }).map_err(|error| error.0)?
    } else if path.is_file() {
        archive::load(path, MANIFEST)?
    } else {
        let extension = path.extension().and_then(|value| value.to_str()).unwrap_or("");
        return Err(if extension.eq_ignore_ascii_case("zip") || extension.eq_ignore_ascii_case("7z") {
            "the archive does not exist".to_owned()
        } else {
            "the folder does not exist".to_owned()
        });
    };
    Ok(RomSet {
        folder: path.to_owned(),
        files,
        assets,
        audio2,
        audiocpu,
    })
}
