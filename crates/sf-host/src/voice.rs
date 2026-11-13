//! Load the local announcer packs and route frame requests to the third MSM5205.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use sf_audio::announcer::{find_pack, pack_folders, Announcer, VoiceStore, MANIFEST, VOICE_FOLDER};
use sf_audio::mix::GameSound;
use sf_audio::score::NativeScore;
use sf_audio::IRQ_LATENCY_MID;
use sf_engine::front::sound;
use sf_video::announcer::{AnnouncerLine, AnnouncerVoice};

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
    load_from(find_pack(&folders(rom_folders, asset_roots)), log)
}

/// [`load`] with the pack given (for tests).
pub fn load_from(pack: Result<VoiceStore, String>, log: &mut Log) -> Option<Arc<VoiceStore>> {
    read_pack(pack, "announcer", log).map(Arc::new)
}

fn folders(rom_folders: &[&Path], asset_roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut folders = pack_folders(rom_folders);
    folders.extend(asset_roots.iter().map(|root| root.join(VOICE_FOLDER)));
    folders
}

fn read_pack(pack: Result<VoiceStore, String>, name: &str, log: &mut Log) -> Option<VoiceStore> {
    match pack {
        Ok(store) => {
            let folder = store.folder.as_deref().map(|f| f.display().to_string()).unwrap_or_default();
            log.line("voice", &format!("the voice pack at {folder}: {} announcer lines", store.len()));
            Some(store)
        }
        Err(why) => {
            log.line("voice", &format!("no {name}: {why}"));
            None
        }
    }
}

/// Both stores loaded at startup, including the ROM's Fight! for the
/// Ringside Settings audition.
pub struct VoicePacks {
    ringside: Arc<VoiceStore>,
    japanese: Arc<VoiceStore>,
}

impl VoicePacks {
    pub fn new(ringside: VoiceStore, japanese: VoiceStore) -> Self {
        Self { ringside: Arc::new(ringside), japanese: Arc::new(japanese) }
    }

    /// Search independently for each pack so a local Japanese pack can be
    /// used alongside the bundled Ringside pack.
    pub fn load(rom_folders: &[&Path], asset_roots: &[PathBuf], audio2: &[u8], log: &mut Log) -> Self {
        let roots = folders(rom_folders, asset_roots);
        // A Japanese-only install creates the parent directory, which
        // by itself is no Ringside override.
        let ringside_folders: Vec<_> = roots.iter().filter(|root| root.join(MANIFEST).exists()).cloned().collect();
        let mut ringside = read_pack(find_pack(&ringside_folders), "Ringside pack", log).unwrap_or_default();
        if let Err(error) = ringside.add_rom_fight(audio2) {
            log.line("voice", &format!("Ringside Fight! audition: {error}"));
        }
        let japanese_folders: Vec<_> = roots.iter().map(|root| root.join("japanese")).collect();
        let japanese = read_pack(find_pack(&japanese_folders), "Japanese pack", log).unwrap_or_default();
        Self::new(ringside, japanese)
    }

    pub fn store(&self, voice: AnnouncerVoice) -> &VoiceStore {
        self.shared_store(voice)
    }

    fn shared_store(&self, voice: AnnouncerVoice) -> &Arc<VoiceStore> {
        match voice {
            AnnouncerVoice::Ringside => &self.ringside,
            AnnouncerVoice::Japanese => &self.japanese,
        }
    }
}

/// The sound data needed to route the game's announcer on one frame.
pub struct AnnouncerFrame<'a> {
    pub frame: u32,
    pub latch: u8,
    pub latch_cycles: u32,
    pub overhaul_fight: bool,
    pub voice: AnnouncerVoice,
    pub lines: &'a [AnnouncerLine],
}

/// The host's announcer delivery, independent of the output device.
pub struct VoicePlayback {
    packs: VoicePacks,
    selected: AnnouncerVoice,
    active: AnnouncerVoice,
    overhaul_fight: bool,
}

impl VoicePlayback {
    pub fn new(packs: VoicePacks, sound: &mut GameSound) -> Self {
        let mut playback = Self {
            packs,
            selected: AnnouncerVoice::Ringside,
            active: AnnouncerVoice::Ringside,
            overhaul_fight: false,
        };
        playback.reset(sound);
        playback
    }

    pub(crate) fn reset(&mut self, sound: &mut GameSound) {
        self.active = self.selected;
        self.overhaul_fight = false;
        sound.set_announcer(Some(Announcer::new(self.packs.shared_store(self.active).clone())));
    }

    pub fn push_frame(&mut self, board: &mut GameSound, request: AnnouncerFrame<'_>) {
        self.push_frame_with_score(board, request, None);
    }

    pub(crate) fn push_frame_with_score(
        &mut self,
        board: &mut GameSound,
        request: AnnouncerFrame<'_>,
        score: Option<Arc<NativeScore>>,
    ) {
        let AnnouncerFrame { frame, latch, latch_cycles, overhaul_fight, voice, lines } = request;
        let store = self.packs.store(voice);
        let replacement = if overhaul_fight && voice == AnnouncerVoice::Japanese {
            AnnouncerLine::from_rom_latch(latch).filter(|&line| store.line(line).is_some())
        } else {
            None
        };
        let audition = !overhaul_fight && lines.contains(&AnnouncerLine::Fight);
        let stopping = latch == sound::STOP_SAMPLES || (self.overhaul_fight && !overhaul_fight);
        let lines = if audition {
            &[AnnouncerLine::Fight][..]
        } else if stopping {
            &[]
        } else {
            lines
        };
        let source = if audition && store.line(AnnouncerLine::Fight).is_none() {
            AnnouncerVoice::Ringside
        } else {
            voice
        };
        if let Some(chip) = board.announcer_mut() {
            if stopping || audition {
                chip.stop_at_frame(frame);
            }
            // Keep the ROM store active until an audition using it finishes
            // or a new request needs the selected pack.
            if source != self.active && (voice != self.selected || replacement.is_some() || !lines.is_empty()) {
                chip.set_store_at_frame(frame, self.packs.shared_store(source).clone());
                self.active = source;
            }
        }
        self.selected = voice;
        self.overhaul_fight = overhaul_fight;

        let latch = if replacement.is_some() { sound::IDLE } else { latch };
        board.push_frame_with_score(frame, latch, latch_cycles, IRQ_LATENCY_MID, score);
        if let Some(line) = replacement {
            board.announce(frame, &[line]);
        }
        board.announce(frame, lines);
    }
}
