//! The announcer's voice at launch (spec #216, `EH8`): the voice pack
//! beside the ROM set, looked for as the art pack is, loaded into the store
//! his chip plays from in overhaul fights. Without a usable pack there is
//! no announcer, and the log says why in one line; nothing else changes.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use sf_audio::announcer::{find_pack, pack_folders, VoiceStore, VOICE_FOLDER};

use crate::log::Log;

/// The voice pack beside the ROM folders, or `None` (logged).
pub fn load(rom_folders: &[&Path], log: &mut Log) -> Option<Arc<VoiceStore>> {
    load_with_fallbacks(rom_folders, &[], log)
}

/// Search for a local override first, followed by bundled asset directories.
pub fn load_with_fallbacks(
    rom_folders: &[&Path],
    asset_roots: &[PathBuf],
    log: &mut Log,
) -> Option<Arc<VoiceStore>> {
    let mut folders = pack_folders(rom_folders);
    folders.extend(asset_roots.iter().map(|root| root.join(VOICE_FOLDER)));
    load_from(find_pack(&folders), log)
}

/// [`load`] with the pack given (for tests).
pub fn load_from(pack: Result<VoiceStore, String>, log: &mut Log) -> Option<Arc<VoiceStore>> {
    match pack {
        Ok(store) => {
            let folder = store.folder.as_deref().map(|f| f.display().to_string()).unwrap_or_default();
            log.line("voice", &format!("the voice pack at {folder}: {} announcer lines", store.len()));
            Some(Arc::new(store))
        }
        Err(why) => {
            log.line("voice", &format!("no announcer: {why}"));
            None
        }
    }
}
