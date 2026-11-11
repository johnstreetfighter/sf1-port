//! Finding the ROM set (spec #150, section 5.5): the folders the game looks
//! in, in order, and the first one whose files all match the embedded
//! manifest (each file present, its size and its CRC-32).

use std::path::{Path, PathBuf};

use sf_engine::front::rom_missing::Looked;
use sf_video::romset::{self, Assets};

/// The ROM manifest, embedded so the game is one file plus the ROM folder.
pub const MANIFEST: &str = include_str!("../../../tools/reference/rom-manifest.json");

/// A folder to look in. `path` is `None` when nothing names it (the
/// settings file's folder, unset).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub label: String,
    pub path: Option<PathBuf>,
}

/// The set, loaded from the folder that matched.
pub struct RomSet {
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

fn load(dir: &Path) -> Result<RomSet, String> {
    let files = romset::check_set(dir, MANIFEST).map_err(|e| e.0)?;
    let region = |name| romset::load_region_from_manifest(dir, MANIFEST, name).map_err(|e| e.0);
    Ok(RomSet {
        folder: dir.to_owned(),
        files,
        assets: romset::load_assets_from_manifest(dir, MANIFEST).map_err(|e| e.0)?,
        audio2: region("audio2")?,
        audiocpu: region("audiocpu")?,
    })
}
