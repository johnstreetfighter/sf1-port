//! The music pack (ADR-0009, ticket #199 `ST7`): a local folder of scores,
//! arrangements of Capcom melodies the public repository never holds. It sits
//! beside the ROM set as the art pack and the voice pack do, in
//! [`MUSIC_FOLDER`], and holds `.score` files in the converter's format
//! (docs/reference/extended-tunes.md), each naming its own command. Nothing
//! here converts them: [`crate::tunes::add_pack`] does, against the player's
//! sound ROM, so a pack stays plain text.
//!
//! A pack is read only when it is opened, and the game opens it only for
//! overhaul presets (`sf_host::themes`).

use std::path::{Path, PathBuf};

/// The music pack's folder name, beside the ROM set's folder.
pub const MUSIC_FOLDER: &str = "sf1-port-music";
/// What a score file's name ends with.
pub const SCORE_EXTENSION: &str = "score";

/// One score of the pack.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackScore {
    /// The file's name in the pack.
    pub file: String,
    pub text: String,
}

/// An opened music pack: its scores, in file-name order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MusicPack {
    pub folder: PathBuf,
    pub scores: Vec<PackScore>,
    /// Files that could not be read (not UTF-8 text, say), each as a
    /// message naming it; the pack holds the rest.
    pub unreadable: Vec<String>,
}

impl MusicPack {
    /// Read the `.score` files directly inside `folder`. A file that cannot
    /// be read as text is set aside in [`MusicPack::unreadable`] and the
    /// others load; only a folder that cannot be listed is an error.
    pub fn open(folder: &Path) -> Result<MusicPack, String> {
        let entries = std::fs::read_dir(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
        let mut files: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case(SCORE_EXTENSION)))
            .collect();
        files.sort();
        let (mut scores, mut unreadable) = (Vec::new(), Vec::new());
        for path in files {
            let file = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            match std::fs::read_to_string(&path) {
                Ok(text) => scores.push(PackScore { file, text }),
                Err(e) => unreadable.push(format!("{file}: {e}")),
            }
        }
        Ok(MusicPack { folder: folder.to_owned(), scores, unreadable })
    }

    pub fn len(&self) -> usize {
        self.scores.len()
    }

    pub fn is_empty(&self) -> bool {
        self.scores.is_empty()
    }
}

/// The folders the music pack may be in: beside each ROM folder, in order,
/// each once (as the art pack is looked for).
pub fn pack_folders(rom_folders: &[&Path]) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for dir in rom_folders {
        let pack = dir.parent().unwrap_or(dir).join(MUSIC_FOLDER);
        if !out.contains(&pack) {
            out.push(pack);
        }
    }
    out
}

/// The first of `folders` that exists, opened; or why there is no pack.
pub fn find_pack(folders: &[PathBuf]) -> Result<MusicPack, String> {
    match folders.iter().find(|f| f.is_dir()) {
        Some(folder) => MusicPack::open(folder),
        None => Err(format!("there is no {MUSIC_FOLDER} folder beside the ROMs")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sf1-port-music-pack-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_pack_holds_its_score_files_in_name_order() {
        let dir = scratch("open");
        std::fs::write(dir.join("b.score"), "two").unwrap();
        std::fs::write(dir.join("a.SCORE"), "one").unwrap();
        std::fs::write(dir.join("notes.txt"), "not a score").unwrap();
        let pack = MusicPack::open(&dir).unwrap();
        let files: Vec<&str> = pack.scores.iter().map(|s| s.file.as_str()).collect();
        assert_eq!(files, ["a.SCORE", "b.score"]);
        assert_eq!(pack.scores[0].text, "one");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_score_that_cannot_be_read_is_set_aside_and_the_rest_load() {
        let dir = scratch("unreadable");
        std::fs::write(dir.join("bad.score"), [0xff, 0xfe, 0x80]).unwrap();
        std::fs::write(dir.join("good.score"), "fine").unwrap();
        let pack = MusicPack::open(&dir).unwrap();
        assert_eq!(pack.scores.len(), 1);
        assert_eq!(pack.scores[0].file, "good.score");
        assert_eq!(pack.unreadable.len(), 1);
        assert!(pack.unreadable[0].starts_with("bad.score:"), "{:?}", pack.unreadable);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_pack_is_looked_for_beside_each_rom_folder_once() {
        let a = Path::new("games").join("roms").join("sf");
        let b = Path::new("games").join("roms").join("sf2");
        let folders = pack_folders(&[a.as_path(), b.as_path()]);
        assert_eq!(folders, [Path::new("games").join("roms").join(MUSIC_FOLDER)]);
        let none = find_pack(&[std::env::temp_dir().join("sf1-port-no-such-music-pack")]);
        assert!(none.unwrap_err().contains(MUSIC_FOLDER));
    }
}
