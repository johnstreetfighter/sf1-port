//! The announcer's chip (spec #216, `EH8`, ADR 0011): a third MSM5205 of the
//! board's kind, mixed beside its two ([`crate::mix::GameSound`]), playing
//! the announcer's lines from the voice pack at the hardware's 4 kHz. The
//! samples CPU, its latch and its two chips never see it, so he never cuts
//! off a 1987 hit or shout.
//!
//! **One line at a time.** He has one chip:
//!
//! - a line asked for while nothing plays starts at once;
//! - a higher line ([`AnnouncerLine::priority`]) cuts off the one playing,
//!   which is not resumed;
//! - an equal or lower one waits, and when the chip falls free the highest
//!   waiting line starts (the oldest among equals);
//! - a line that has waited more than [`WAIT_FRAMES`] (half a second) is
//!   dropped, so he never calls something long past.
//!
//! **The chip.** Every [`ANNOUNCER_HZ`] tick it decodes one nibble of the
//! line playing (a line's start resets it first, as the samples CPU resets
//! a channel before a sample); while idle it is held in reset, its output
//! 0, as an idle channel of the board's is. Its output holds between ticks.
//!
//! **The voice pack** ([`VoiceStore`]) is a folder named [`VOICE_FOLDER`].
//! The game looks beside the ROM set for overrides and falls back to its
//! bundled pack:
//!
//! - `manifest.json`: `{"format": 1, "sample_rate": 4000, "lines": {"super":
//!   {"nibbles": 2412, "crc32": "1a2b3c4d"}, ...}}`, one entry per line by
//!   its file stem ([`AnnouncerLine::file_stem`]: `super`, `perfect`,
//!   `perfect_parry`, `counter`, `punish_counter`, `crush_counter`,
//!   `alpha_counter`, `reversal`, `first_attack`, `burnout`,
//!   `custom_combo`, `combo_5`, `combo_6`, ...);
//! - `<stem>.vox`: the line's 4-bit MSM5205 ADPCM, two nibbles a byte, high
//!   nibble first (the samples ROM's order); an odd count's last low nibble
//!   is 0 and not played;
//! - anything else (takes, prompts, the backup's git files) is not the
//!   game's business.
//!
//! Every line must validate as 4 kHz 4-bit MSM5205 ADPCM
//! ([`sf_video::asset_pack::validate_adpcm`]) and match its CRC-32, or the
//! pack is refused whole. A finished combo whose count has no line is not
//! called (he never says a wrong number).

use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sf_video::announcer::AnnouncerLine;
use sf_video::asset_pack::{validate_adpcm, AdpcmClip, ADPCM_RATE};
use sf_video::romset::crc32;

use crate::msm5205::Msm5205;
use crate::FRAME_HZ;

/// The announcer's chip rate: the board's 4 kHz a chip.
pub const ANNOUNCER_HZ: u32 = ADPCM_RATE;
/// The longest a line may wait for the chip (half a second).
pub const WAIT_FRAMES: u32 = 30;
/// [`WAIT_FRAMES`] in chip ticks.
pub const WAIT_TICKS: u64 = (WAIT_FRAMES * ANNOUNCER_HZ / FRAME_HZ) as u64;

/// The voice pack's folder name, beside the ROM set's folder.
pub const VOICE_FOLDER: &str = "sf1-port-voice";
/// The manifest's file name in the pack.
pub const MANIFEST: &str = "manifest.json";
/// The manifest format this game reads.
pub const FORMAT: u32 = 1;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    format: u32,
    sample_rate: u32,
    lines: BTreeMap<String, LineEntry>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LineEntry {
    nibbles: usize,
    crc32: String,
}

/// The announcer's lines, each validated 4 kHz 4-bit ADPCM: the voice pack
/// as loaded, used only by overhaul fights.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VoiceStore {
    /// Where it was loaded from, if from disk.
    pub folder: Option<PathBuf>,
    lines: BTreeMap<AnnouncerLine, Vec<u8>>,
}

impl VoiceStore {
    /// A store of `clips`; refused if any is not 4 kHz 4-bit MSM5205 ADPCM.
    pub fn from_clips(clips: impl IntoIterator<Item = (AnnouncerLine, AdpcmClip)>) -> Result<VoiceStore, String> {
        let mut lines = BTreeMap::new();
        for (line, clip) in clips {
            validate_adpcm(&clip).map_err(|e| format!("{}: {}", line.file_stem(), e.message))?;
            lines.insert(line, clip.nibbles);
        }
        Ok(VoiceStore { folder: None, lines })
    }

    /// Open the pack in `folder` and check every line. An error is the
    /// reason the pack cannot be used.
    pub fn open(folder: &Path) -> Result<VoiceStore, String> {
        let text = std::fs::read_to_string(folder.join(MANIFEST))
            .map_err(|e| format!("{}: {MANIFEST}: {e}", folder.display()))?;
        let manifest: Manifest =
            serde_json::from_str(&text).map_err(|e| format!("{}: {MANIFEST}: {e}", folder.display()))?;
        if manifest.format != FORMAT {
            return Err(format!(
                "{}: format {} (this game reads format {FORMAT})",
                folder.display(),
                manifest.format
            ));
        }
        let mut clips = Vec::new();
        for (stem, entry) in &manifest.lines {
            let where_ = || format!("{}: {stem}", folder.display());
            let line = AnnouncerLine::from_file_stem(stem)
                .ok_or_else(|| format!("{}: not an announcer line", where_()))?;
            let file = folder.join(format!("{stem}.vox"));
            let bytes = std::fs::read(&file).map_err(|e| format!("{}.vox: {e}", where_()))?;
            if bytes.len() != entry.nibbles.div_ceil(2) || format!("{:08x}", crc32(&bytes)) != entry.crc32 {
                return Err(format!("{}.vox: does not match the manifest (damaged or changed)", where_()));
            }
            let nibbles = bytes.iter().flat_map(|b| [b >> 4, b & 0x0f]).take(entry.nibbles).collect();
            clips.push((line, AdpcmClip { sample_rate: manifest.sample_rate, nibbles }));
        }
        let mut store = VoiceStore::from_clips(clips).map_err(|e| format!("{}: {e}", folder.display()))?;
        store.folder = Some(folder.to_owned());
        Ok(store)
    }

    /// Write the store as a voice pack in `folder` (the voice pipeline's
    /// last step, and the tests'): each line's `.vox` and the manifest.
    pub fn write(&self, folder: &Path) -> Result<(), String> {
        std::fs::create_dir_all(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
        let mut lines = BTreeMap::new();
        for (line, nibbles) in &self.lines {
            let bytes: Vec<u8> = nibbles
                .chunks(2)
                .map(|p| (p[0] << 4) | p.get(1).copied().unwrap_or(0))
                .collect();
            let path = folder.join(format!("{}.vox", line.file_stem()));
            std::fs::write(&path, &bytes).map_err(|e| format!("{}: {e}", path.display()))?;
            lines.insert(
                line.file_stem(),
                LineEntry { nibbles: nibbles.len(), crc32: format!("{:08x}", crc32(&bytes)) },
            );
        }
        let manifest = Manifest { format: FORMAT, sample_rate: ANNOUNCER_HZ, lines };
        let text = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())? + "\n";
        let path = folder.join(MANIFEST);
        std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// A line's nibbles, if the pack has it.
    pub fn line(&self, line: AnnouncerLine) -> Option<&[u8]> {
        self.lines.get(&line).map(Vec::as_slice)
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}

/// The folders the voice pack may be in: beside each ROM folder, in order,
/// each once (as the art pack is looked for).
pub fn pack_folders(rom_folders: &[&Path]) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for dir in rom_folders {
        let pack = dir.parent().unwrap_or(dir).join(VOICE_FOLDER);
        if !out.contains(&pack) {
            out.push(pack);
        }
    }
    out
}

/// The first of `folders` that exists, opened and checked; or why there is
/// no usable pack.
pub fn find_pack(folders: &[PathBuf]) -> Result<VoiceStore, String> {
    match folders.iter().find(|f| f.is_dir()) {
        Some(folder) => VoiceStore::open(folder),
        None => Err(format!("there is no {VOICE_FOLDER} folder beside the ROMs")),
    }
}

#[derive(Clone, Copy, Debug)]
struct Playing {
    line: AnnouncerLine,
    /// The next nibble.
    pos: usize,
}

/// The announcer's chip and his queue, rendered one 4 kHz tick at a time
/// on the machine's clock.
#[derive(Clone, Debug)]
pub struct Announcer {
    store: Arc<VoiceStore>,
    chip: Msm5205,
    /// Lines asked for and not yet reached: (tick, line), in order.
    arriving: VecDeque<(u64, AnnouncerLine)>,
    /// Lines waiting for the chip: (tick asked for, line), oldest first.
    waiting: Vec<(u64, AnnouncerLine)>,
    playing: Option<Playing>,
    /// The next tick to run.
    tick: u64,
    level: f32,
}

impl Announcer {
    /// Silent, his chip in reset, with the lines of `store`.
    pub fn new(store: Arc<VoiceStore>) -> Self {
        let mut chip = Msm5205::new();
        chip.write(0x80);
        Announcer {
            store,
            chip,
            arriving: VecDeque::new(),
            waiting: Vec::new(),
            playing: None,
            tick: 0,
            level: 0.0,
        }
    }

    /// Ask for `lines` at frame `frame`'s start, in order. Frames must not
    /// go back; a line the pack lacks is not called.
    pub fn call_at_frame(&mut self, frame: u32, lines: &[AnnouncerLine]) {
        for &line in lines {
            self.call(frame as f64 / FRAME_HZ as f64, line);
        }
    }

    /// Ask for `line` at machine time `t` (seconds, non-decreasing): it
    /// reaches the chip on the first tick at or after `t`.
    pub fn call(&mut self, t: f64, line: AnnouncerLine) {
        if self.store.line(line).is_none() {
            return;
        }
        let tick = (((t * ANNOUNCER_HZ as f64) - 1e-6).ceil().max(0.0) as u64).max(self.tick);
        debug_assert!(
            self.arriving.back().is_none_or(|&(k, _)| k <= tick),
            "announcer lines asked for out of order"
        );
        self.arriving.push_back((tick, line));
    }

    /// The line on the chip now, if any.
    pub fn playing(&self) -> Option<AnnouncerLine> {
        self.playing.map(|p| p.line)
    }

    /// The lines waiting for the chip, oldest first.
    pub fn waiting(&self) -> Vec<AnnouncerLine> {
        self.waiting.iter().map(|&(_, l)| l).collect()
    }

    /// The chip's output as last reached.
    pub fn level(&self) -> f32 {
        self.level
    }

    /// Ticks run so far (tick k runs at k / [`ANNOUNCER_HZ`] s).
    pub fn ticks(&self) -> u64 {
        self.tick
    }

    /// The chip's output at machine time `t` (non-decreasing): every tick
    /// at or before `t` is run first.
    pub fn output_at(&mut self, t: f64) -> f32 {
        while self.tick as f64 / ANNOUNCER_HZ as f64 <= t {
            self.run_tick();
        }
        self.level
    }

    fn start(&mut self, line: AnnouncerLine) {
        self.chip.write(0x80);
        self.playing = Some(Playing { line, pos: 0 });
    }

    fn run_tick(&mut self) {
        let k = self.tick;
        self.tick += 1;
        // A line whose last nibble has played leaves the chip free.
        if let Some(p) = self.playing {
            if p.pos >= self.store.line(p.line).map_or(0, <[u8]>::len) {
                self.playing = None;
                self.chip.write(0x80);
            }
        }
        while let Some(&(at, line)) = self.arriving.front() {
            if at > k {
                break;
            }
            self.arriving.pop_front();
            match self.playing {
                None => self.start(line),
                Some(p) if line.priority() > p.line.priority() => self.start(line),
                Some(_) => self.waiting.push((at, line)),
            }
        }
        self.waiting.retain(|&(at, _)| k - at <= WAIT_TICKS);
        if self.playing.is_none() {
            // The highest waiting line, the oldest among equals.
            let best = self
                .waiting
                .iter()
                .enumerate()
                .max_by_key(|&(i, &(_, line))| (line.priority(), std::cmp::Reverse(i)))
                .map(|(i, _)| i);
            if let Some(i) = best {
                let (_, line) = self.waiting.remove(i);
                self.start(line);
            }
        }
        if let Some(p) = self.playing.as_mut() {
            let nibble = self.store.line(p.line).expect("a stored line")[p.pos];
            p.pos += 1;
            self.chip.write(0x40 | nibble);
        }
        self.level = self.chip.output();
    }
}
