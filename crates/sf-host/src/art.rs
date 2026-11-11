//! The game's art at launch (spec #164): the recipes and review decisions,
//! embedded so the game stays one file, built from the player's ROM and art
//! pack into the extended sprite bank and the fighters' pose catalogues.
//! - A fighter's new poses load only when all of them are approved.
//! - The art pack is looked for beside the ROM set, then beside each other
//!   folder the ROMs are looked for in. A fighter whose generated poses it
//!   does not hold keeps his ROM art, and the log says why.
//! - A recipe that fails to build is logged and the game keeps the ROM's
//!   own graphics.

use std::path::{Path, PathBuf};

use sf_video::art::{load_art_from, plural, Art, ArtSources};
use sf_video::art_pack::{ArtPack, PACK_FOLDER};
use sf_video::romset::Assets;

use crate::log::Log;

/// Every fighter's recipe file.
pub const RECIPES: [&str; 12] = [
    include_str!("../../../art/recipes/ryu.json"),
    include_str!("../../../art/recipes/ken.json"),
    include_str!("../../../art/recipes/sagat.json"),
    include_str!("../../../art/recipes/retsu.json"),
    include_str!("../../../art/recipes/geki.json"),
    include_str!("../../../art/recipes/joe.json"),
    include_str!("../../../art/recipes/mike.json"),
    include_str!("../../../art/recipes/lee.json"),
    include_str!("../../../art/recipes/gen.json"),
    include_str!("../../../art/recipes/birdie.json"),
    include_str!("../../../art/recipes/eagle.json"),
    include_str!("../../../art/recipes/adon.json"),
];

/// Every fighter's review decisions.
pub const REVIEWS: [&str; 12] = [
    include_str!("../../../art/review/ryu.json"),
    include_str!("../../../art/review/ken.json"),
    include_str!("../../../art/review/sagat.json"),
    include_str!("../../../art/review/retsu.json"),
    include_str!("../../../art/review/geki.json"),
    include_str!("../../../art/review/joe.json"),
    include_str!("../../../art/review/mike.json"),
    include_str!("../../../art/review/lee.json"),
    include_str!("../../../art/review/gen.json"),
    include_str!("../../../art/review/birdie.json"),
    include_str!("../../../art/review/eagle.json"),
    include_str!("../../../art/review/adon.json"),
];

/// The folders the art pack may be in: beside each ROM folder, in order,
/// each once.
pub fn pack_folders(rom_folders: &[&Path]) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for dir in rom_folders {
        let pack = dir.parent().unwrap_or(dir).join(PACK_FOLDER);
        if !out.contains(&pack) {
            out.push(pack);
        }
    }
    out
}

/// The first of `folders` that exists, opened and checked; or why there is
/// no usable pack.
pub fn find_pack(folders: &[PathBuf]) -> Result<ArtPack, String> {
    match folders.iter().find(|f| f.is_dir()) {
        Some(folder) => ArtPack::open(folder),
        None => Err(format!("there is no {PACK_FOLDER} folder beside the ROMs")),
    }
}

/// Build the art from `rom`, the embedded recipes and reviews, and the art
/// pack beside the ROM folders, logging what loaded.
pub fn load(rom: &Assets, rom_folders: &[&Path], log: &mut Log) -> Option<Art> {
    let pack = find_pack(&pack_folders(rom_folders));
    load_from(rom, &RECIPES, &REVIEWS, pack, log)
}

/// [`load`] with the recipes, reviews and pack given (for tests).
pub fn load_from(rom: &Assets, recipes: &[&str], reviews: &[&str], pack: Result<ArtPack, String>, log: &mut Log) -> Option<Art> {
    if let Ok(p) = &pack {
        log.line("art", &format!("the art pack at {}: {}", p.folder.display(), plural(p.len(), "generated pose")));
    }
    let no_pack = pack.as_ref().err().cloned().unwrap_or_default();
    let sources = ArtSources { recipes, reviews: Some(reviews), pack: pack.as_ref().map_err(|_| no_pack.as_str()) };
    match load_art_from(rom, &sources) {
        Ok(art) => {
            log.line("art", &art.describe());
            for (fighter, why) in &art.kept {
                log.line("art", &format!("{fighter} keeps his ROM art: {why}"));
            }
            Some(art)
        }
        Err(e) => {
            log.line("art", &format!("keeping the ROM's own graphics: {e}"));
            None
        }
    }
}
