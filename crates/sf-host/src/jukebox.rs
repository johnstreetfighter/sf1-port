//! The Jukebox on the host (ADR-0009, design section 6a, ticket #200): finds
//! the files the session's cues name, decodes them off the game thread, and
//! mixes the playing track with the board's sound.
//!
//! - The files are in a `jukebox` folder beside the ROM set, as the art
//!   pack is looked for (beside each ROM folder, in order); an absolute path
//!   in the settings is used as it is.
//! - **Nothing is decoded on the game thread.** [`TrackCue::Prepare`] starts
//!   a thread decoding the file as the fight starts (or, in Arcade, the
//!   ladder moves on), and [`TrackCue::Play`] uses it if it is ready. If it
//!   is not (the file was not announced, or is still decoding), the decode
//!   goes on in its thread and the native theme plays that round. A decode
//!   that panics is caught in its thread and counts as a failed file.
//! - One track is kept, so the next round starts at once. Announcing another
//!   file stops the decode of the one replaced. A file that changed on disk
//!   (its modified time or size) is decoded again, a failed one included.
//! - A file that cannot be played logs one line, once: the file, why, and the
//!   native theme taking over.
//! - [`TrackMixer`] holds the playing track and the start and stop events,
//!   each at the machine time of the frame it came with, and is mixed into
//!   the output by `audio::Audio`.

use std::collections::{HashSet, VecDeque};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::SystemTime;

use sf_audio::mix::YM_ROUTE_GAIN;
use sf_engine::front::sound;
use sf_engine::jukebox::{Resolved, TrackCue, TrackFormats};

use crate::log::Log;
use crate::track::{Playhead, Track};

/// The folder of tracks, beside the ROM set.
pub const FOLDER: &str = "jukebox";

/// How loud a track plays against the native music it replaces. The ten
/// 1987 stage themes measure 0.07 to 0.13 RMS (0.09 on average) at the
/// output after [`YM_ROUTE_GAIN`] (`tests/loudness.rs` in sf-game repeats
/// the measure), which is 0.15 on the chip's own scale. A mastered track
/// sits near 0.15 RMS (about -16 dBFS), so a track takes the chip route's
/// gain and a reference level of 1: the same RMS as a native theme, with
/// the same headroom. *My call* on the reference.
pub const TRACK_REFERENCE: f32 = 1.0;
pub const TRACK_GAIN: f32 = YM_ROUTE_GAIN * TRACK_REFERENCE;

/// Where the mix starts to be bent, and the most it reaches: a track and the
/// board's sound together never clip.
const KNEE: f32 = 0.8;

/// `x` kept inside -1..=1: unchanged up to the knee, then bent smoothly
/// toward the limit.
pub fn soft_limit(x: f32) -> f32 {
    let a = x.abs();
    if a <= KNEE {
        x
    } else {
        x.signum() * (KNEE + (1.0 - KNEE) * ((a - KNEE) / (1.0 - KNEE)).tanh())
    }
}

/// The folders the tracks may be in: beside each ROM folder, in order, each
/// once.
pub fn folders(rom_folders: &[&Path]) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for dir in rom_folders {
        let folder = dir.parent().unwrap_or(dir).join(FOLDER);
        if !out.contains(&folder) {
            out.push(folder);
        }
    }
    out
}

/// The files the Jukebox screen offers (ticket #202): the names in the
/// `folders` that are of a format the host plays
/// ([`crate::track::supported_extensions`]: OGG, WAV and FLAC, and tracker
/// modules only in a build with the `tracker` feature; the screen is given
/// the same list), each once (the first folder's wins, as [`find`] finds
/// it), in name order. Folders inside a folder, names that are not text, and
/// a folder that is missing or cannot be read are left out: there is no
/// file browser, and a screen with no files still shows `NATIVE`.
pub fn list_tracks(folders: &[PathBuf]) -> Vec<String> {
    let formats = TrackFormats::new(crate::track::supported_extensions());
    let mut out: Vec<String> = Vec::new();
    for folder in folders {
        let Ok(dir) = std::fs::read_dir(folder) else { continue };
        for entry in dir.flatten() {
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else { continue };
            if formats.accepts(&name) && entry.path().is_file() && !out.contains(&name) {
                out.push(name);
            }
        }
    }
    out.sort_by_cached_key(|f| (f.to_lowercase(), f.clone()));
    out
}

/// Where the file the settings name is: itself if absolute, else in the
/// first folder that holds it. The error names the folders looked in.
pub fn find(file: &str, folders: &[PathBuf]) -> Result<PathBuf, String> {
    let path = Path::new(file);
    if path.is_absolute() {
        return if path.is_file() { Ok(path.to_owned()) } else { Err(format!("{file} does not exist")) };
    }
    folders
        .iter()
        .map(|f| f.join(path))
        .find(|p| p.is_file())
        .ok_or_else(|| {
            let looked: Vec<String> = folders.iter().map(|f| f.display().to_string()).collect();
            match looked.is_empty() {
                true => format!("{file} is in no {FOLDER} folder (there is none beside the ROMs)"),
                false => format!("{file} is not in {}", looked.join(" or ")),
            }
        })
}

type Decoded = Result<Arc<Track>, String>;

/// What tells a file has changed.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Stamp {
    modified: Option<SystemTime>,
    len: u64,
}

fn stamp(path: &Path) -> Stamp {
    let meta = std::fs::metadata(path).ok();
    Stamp { modified: meta.as_ref().and_then(|m| m.modified().ok()), len: meta.map_or(0, |m| m.len()) }
}

enum State {
    Decoding { handle: JoinHandle<Decoded>, cancel: Arc<AtomicBool> },
    Done(Decoded),
}

/// The one track kept: decoding, or decoded (or failed).
struct Slot {
    path: PathBuf,
    stamp: Stamp,
    state: State,
    /// What the log has said of this slot: that its decode was not ready, and
    /// the file's outcome (played, or why not).
    said_wait: bool,
    said_result: bool,
}

impl Slot {
    fn start(path: PathBuf, stamp: Stamp) -> Slot {
        let cancel = Arc::new(AtomicBool::new(false));
        let (at, flag) = (path.clone(), cancel.clone());
        let handle = std::thread::spawn(move || {
            catch_unwind(AssertUnwindSafe(|| Track::load_unless(&at, &flag).map(Arc::new)))
                .unwrap_or_else(|_| Err(format!("{}: the decoder stopped on this file", at.display())))
        });
        Slot { path, stamp, state: State::Decoding { handle, cancel }, said_wait: false, said_result: false }
    }

    /// Take the decode's result if its thread has finished.
    fn poll(&mut self) {
        if matches!(&self.state, State::Decoding { handle, .. } if handle.is_finished()) {
            self.join();
        }
    }

    fn join(&mut self) {
        let State::Decoding { handle, .. } = std::mem::replace(&mut self.state, State::Done(Err(String::new()))) else {
            return;
        };
        let done = handle.join().unwrap_or_else(|_| Err(format!("{}: the decoder stopped on this file", self.path.display())));
        self.state = State::Done(done);
    }
}

impl Drop for Slot {
    /// A replaced decode has no use: it stops at its next packet.
    fn drop(&mut self) {
        if let State::Decoding { cancel, .. } = &self.state {
            cancel.store(true, Ordering::Relaxed);
        }
    }
}

/// What the host does for a cue.
#[derive(Clone, Debug)]
pub enum Action {
    /// Nothing.
    None,
    /// Start the track, and leave the theme's latch out of the music CPU's
    /// input: the track is the fight's music.
    Start(Arc<Track>),
    /// Stop the track.
    Stop,
}

/// The host's side of the Jukebox: where the files are, and the track kept.
pub struct Deck {
    folders: Vec<PathBuf>,
    slot: Option<Slot>,
    /// What the log said of files that could not be found.
    missing: HashSet<String>,
    /// The theme's command while a track plays in its place.
    theme: Option<u8>,
}

impl Deck {
    pub fn new(folders: Vec<PathBuf>) -> Self {
        Deck { folders, slot: None, missing: HashSet::new(), theme: None }
    }

    /// The files the Jukebox screen offers from this deck's folders.
    pub fn tracks(&self) -> Vec<String> {
        list_tracks(&self.folders)
    }

    /// The slot for `track`'s file, decoding it in a thread if it is not the
    /// one kept (or the file changed); the error says why there is no file.
    fn ensure(&mut self, track: &Resolved) -> Result<&mut Slot, String> {
        let path = find(&track.file, &self.folders)?;
        let now = stamp(&path);
        if !self.slot.as_ref().is_some_and(|s| s.path == path && s.stamp == now) {
            self.slot = Some(Slot::start(path, now));
        }
        Ok(self.slot.as_mut().expect("the slot was just filled"))
    }

    /// Wait for the decode in progress (tests and headless runs; the game
    /// never waits).
    pub fn settle(&mut self) {
        if let Some(slot) = &mut self.slot {
            slot.join();
        }
    }

    /// What to do for `cue`.
    pub fn cue(&mut self, cue: &TrackCue, log: &mut Log) -> Action {
        match cue {
            TrackCue::Prepare(track) => {
                let _ = self.ensure(track);
                Action::None
            }
            TrackCue::Stop => Action::Stop,
            TrackCue::Play(track) => self.play(track, log),
        }
    }

    fn play(&mut self, track: &Resolved, log: &mut Log) -> Action {
        let why = track.source.describe();
        let slot = match self.ensure(track) {
            Ok(slot) => slot,
            Err(message) => {
                if self.missing.insert(message.clone()) {
                    log.line("jukebox", &format!("{} ({why}): {message}; the native theme plays", track.file));
                }
                return Action::None;
            }
        };
        slot.poll();
        match &slot.state {
            State::Done(Ok(decoded)) => {
                if !std::mem::replace(&mut slot.said_result, true) {
                    let lp = match decoded.looped {
                        Some(l) => format!("loops frames {}..{}", l.start, l.end),
                        None => "has no loop information and restarts from the top".to_owned(),
                    };
                    log.line("jukebox", &format!("{} ({why}) {lp}", track.file));
                }
                Action::Start(decoded.clone())
            }
            State::Done(Err(message)) => {
                if !std::mem::replace(&mut slot.said_result, true) {
                    log.line("jukebox", &format!("{} ({why}): {message}; the native theme plays", track.file));
                }
                Action::None
            }
            State::Decoding { .. } => {
                if !std::mem::replace(&mut slot.said_wait, true) {
                    log.line("jukebox", &format!("{} ({why}) is not decoded yet; the native theme plays this round", track.file));
                }
                Action::None
            }
        }
    }

    /// The latch the music CPU is given for a frame whose cue came to
    /// `action` and whose own latch is `latch`. A started track replaces the
    /// theme: the theme's command is left out on its frame, and on every
    /// frame it is held or sent again for as long as the track plays, so it
    /// never starts the native tune under the track. A stop, and the stops
    /// and the jingle with it, reach the music CPU as they are.
    pub fn latch(&mut self, action: &Action, latch: u8) -> u8 {
        match action {
            Action::Start(_) => {
                self.theme = Some(latch);
                sound::IDLE
            }
            Action::Stop => {
                self.theme = None;
                latch
            }
            Action::None if self.theme == Some(latch) => sound::IDLE,
            Action::None => latch,
        }
    }
}

enum Event {
    Start(Arc<Track>),
    Stop,
}

/// The playing track and the events that start and stop it, in machine time.
#[derive(Default)]
pub struct TrackMixer {
    events: VecDeque<(f64, Event)>,
    playing: Option<Playhead>,
    /// The last frame given, held while the output waits for the game.
    last: [f32; 2],
}

impl TrackMixer {
    /// Start `track` from its beginning at machine time `at`.
    pub fn start(&mut self, at: f64, track: Arc<Track>) {
        self.events.push_back((at, Event::Start(track)));
    }

    /// Stop the track at machine time `at`.
    pub fn stop(&mut self, at: f64) {
        self.events.push_back((at, Event::Stop));
    }

    /// Forget the track and the events (the sound restarted).
    pub fn clear(&mut self) {
        self.events.clear();
        self.playing = None;
        self.last = [0.0; 2];
    }

    pub fn is_playing(&self) -> bool {
        self.playing.is_some()
    }

    /// The track's next frame at the output rate `out_rate`, for the output
    /// at machine time `t`: events due by `t` happen first. `advance` is false
    /// while the output holds for the game: the track holds with it, at the
    /// last sample, not at zero.
    pub fn sample(&mut self, t: f64, advance: bool, out_rate: f64) -> [f32; 2] {
        while self.events.front().is_some_and(|(at, _)| *at <= t) {
            match self.events.pop_front() {
                Some((_, Event::Start(track))) => self.playing = Some(Playhead::new(track)),
                Some((_, Event::Stop)) => {
                    self.playing = None;
                    self.last = [0.0; 2];
                }
                None => {}
            }
        }
        if let (Some(head), true) = (&mut self.playing, advance) {
            self.last = head.next(out_rate);
        }
        if self.playing.is_some() {
            self.last
        } else {
            [0.0; 2]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sf_engine::jukebox::TrackSource;

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("sf1-port-jukebox-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn log(d: &Path) -> (Log, PathBuf) {
        let p = d.join("test.log");
        (Log::create(&p), p)
    }

    fn resolved(file: &str) -> Resolved {
        Resolved { file: file.into(), source: TrackSource::Global }
    }

    #[test]
    fn the_folder_is_beside_each_rom_folder() {
        let a = Path::new("x/roms/sf");
        let b = Path::new("x/roms/other");
        assert_eq!(folders(&[a, b, a]), [PathBuf::from("x/roms/jukebox")]);
        assert_eq!(folders(&[a, Path::new("y/sf")]), [PathBuf::from("x/roms/jukebox"), PathBuf::from("y/jukebox")]);
    }

    #[test]
    fn a_file_is_found_in_the_first_folder_that_holds_it_or_by_its_absolute_path() {
        let (a, b) = (dir("find-a"), dir("find-b"));
        std::fs::write(b.join("x.ogg"), b"x").unwrap();
        std::fs::write(a.join("y.ogg"), b"y").unwrap();
        std::fs::write(b.join("y.ogg"), b"y").unwrap();
        let both = [a.clone(), b.clone()];
        assert_eq!(find("x.ogg", &both), Ok(b.join("x.ogg")));
        assert_eq!(find("y.ogg", &both), Ok(a.join("y.ogg")));
        assert_eq!(find(b.join("x.ogg").to_str().unwrap(), &[]), Ok(b.join("x.ogg")));
        let err = find("gone.ogg", &both).unwrap_err();
        assert!(err.contains("gone.ogg") && err.contains(&a.display().to_string()), "{err}");
        assert!(find("gone.ogg", &[]).unwrap_err().contains("no jukebox folder"));
        assert!(find(a.join("gone.ogg").to_str().unwrap(), &[]).is_err());
    }

    #[test]
    fn a_missing_or_undecodable_file_logs_once_and_leaves_the_native_theme() {
        let d = dir("missing");
        std::fs::write(d.join("bad.ogg"), b"this is not audio at all").unwrap();
        let (mut log, path) = log(&d);
        let mut deck = Deck::new(vec![d.clone()]);
        for round in 0..3 {
            for file in ["gone.ogg", "bad.ogg"] {
                let cue = TrackCue::Play(resolved(file));
                let mut action = deck.cue(&cue, &mut log);
                deck.settle();
                if matches!(action, Action::None) && file == "bad.ogg" {
                    action = deck.cue(&cue, &mut log);
                }
                assert!(matches!(action, Action::None), "{file} round {round}");
                assert_eq!(deck.latch(&action, 0x29), 0x29, "the theme's latch reaches the music CPU");
            }
        }
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("gone.ogg") && text.contains("bad.ogg"), "{text}");
        assert_eq!(text.lines().filter(|l| l.contains("gone.ogg")).count(), 1, "said once: {text}");
        assert_eq!(text.lines().filter(|l| l.contains("bad.ogg")).count(), 2, "not decoded yet, then why not, once each: {text}");
    }

    #[cfg(feature = "tracker")]
    #[test]
    fn a_tracker_module_is_a_track_and_a_broken_one_falls_back_to_the_native_theme_with_a_log_line() {
        let d = dir("modules");
        let good = include_bytes!("../tests/data/loop.xm");
        std::fs::write(d.join("song.xm"), good).unwrap();
        std::fs::write(d.join("broken.xm"), &good[..100]).unwrap();
        std::fs::write(d.join("junk.it"), b"IMPM but nothing like a module").unwrap();
        let (mut log, path) = log(&d);
        let mut deck = Deck::new(vec![d.clone()]);

        let song = resolved("song.xm");
        assert!(matches!(deck.cue(&TrackCue::Prepare(song.clone()), &mut log), Action::None));
        deck.settle();
        let action = deck.cue(&TrackCue::Play(song), &mut log);
        let Action::Start(track) = &action else { panic!("the module plays: {action:?}") };
        assert_eq!(deck.latch(&action, 0x29), sound::IDLE, "the theme is held back while the module plays");
        assert_eq!(track.looped.map(|l| l.start), Some(46_080), "it loops at its restart position");
        let stop = deck.cue(&TrackCue::Stop, &mut log);
        assert!(matches!(stop, Action::Stop));
        assert_eq!(deck.latch(&stop, 0x35), 0x35);

        for round in 0..2 {
            for file in ["broken.xm", "junk.it"] {
                let cue = TrackCue::Play(resolved(file));
                let mut action = deck.cue(&cue, &mut log);
                deck.settle();
                if matches!(action, Action::None) {
                    action = deck.cue(&cue, &mut log);
                }
                assert!(matches!(action, Action::None), "{file} round {round}");
                assert_eq!(deck.latch(&action, 0x29), 0x29, "the native theme's latch reaches the music CPU");
            }
        }
        let text = std::fs::read_to_string(&path).unwrap();
        for file in ["broken.xm", "junk.it"] {
            assert!(text.lines().any(|l| l.contains(file) && l.contains("module") && l.ends_with("the native theme plays")), "a line says why {file} is native: {text}");
        }
    }

    #[test]
    fn nothing_is_decoded_on_the_calling_thread_and_the_native_theme_plays_until_it_is_ready() {
        let d = dir("inline");
        std::fs::write(d.join("t.wav"), crate::track::fixtures::wav(100, &[])).unwrap();
        let (mut log, _) = log(&d);
        let mut deck = Deck::new(vec![d]);
        // Played without being announced: the decode starts, this round is native.
        let first = deck.cue(&TrackCue::Play(resolved("t.wav")), &mut log);
        assert!(matches!(first, Action::None));
        assert_eq!(deck.latch(&first, 0x29), 0x29);
        deck.settle();
        assert!(matches!(deck.cue(&TrackCue::Play(resolved("t.wav")), &mut log), Action::Start(_)), "the next round has it");
    }

    #[test]
    fn a_started_track_holds_the_theme_back_for_as_long_as_it_plays() {
        let d = dir("start");
        std::fs::write(d.join("t.wav"), crate::track::fixtures::wav(100, &[])).unwrap();
        let (mut log, path) = log(&d);
        let mut deck = Deck::new(vec![d]);
        let track = resolved("t.wav");
        assert!(matches!(deck.cue(&TrackCue::Prepare(track.clone()), &mut log), Action::None));
        deck.settle();
        let action = deck.cue(&TrackCue::Play(track.clone()), &mut log);
        assert!(matches!(action, Action::Start(_)));
        assert_eq!(deck.latch(&action, 0x29), sound::IDLE);
        // The theme sent again, or held, while the track plays: left out. Other commands pass.
        assert_eq!(deck.latch(&Action::None, 0x29), sound::IDLE);
        assert_eq!(deck.latch(&Action::None, 0x29), sound::IDLE);
        assert_eq!(deck.latch(&Action::None, 0x27), 0x27);
        assert_eq!(deck.latch(&Action::None, sound::IDLE), sound::IDLE);
        // The next round's theme starts the same decoded track.
        let again = deck.cue(&TrackCue::Play(track), &mut log);
        let (Action::Start(a), Action::Start(b)) = (action, again) else { panic!("both start") };
        assert!(Arc::ptr_eq(&a, &b), "decoded once");
        // After the stop, the command is the music CPU's again.
        assert_eq!(deck.latch(&Action::Stop, 0x35), 0x35);
        assert_eq!(deck.latch(&Action::None, 0x29), 0x29);
        let text = std::fs::read_to_string(path).unwrap();
        assert_eq!(text.matches("has no loop information").count(), 1, "said once: {text}");
    }

    #[test]
    fn a_file_that_changed_is_decoded_again_even_after_it_failed() {
        let d = dir("changed");
        let file = d.join("t.wav");
        std::fs::write(&file, b"not audio").unwrap();
        let (mut log, path) = log(&d);
        let mut deck = Deck::new(vec![d]);
        let cue = TrackCue::Play(resolved("t.wav"));
        deck.cue(&cue, &mut log);
        deck.settle();
        assert!(matches!(deck.cue(&cue, &mut log), Action::None), "failed");
        std::fs::write(&file, crate::track::fixtures::wav(400, &[])).unwrap();
        deck.cue(&cue, &mut log);
        deck.settle();
        assert!(matches!(deck.cue(&cue, &mut log), Action::Start(_)), "the new file plays");
        assert!(std::fs::read_to_string(path).unwrap().contains("t.wav"));
    }

    #[test]
    fn the_mixer_starts_and_stops_at_the_frames_time_and_holds_the_last_sample_with_the_game() {
        let track = Arc::new(Track { rate: 1, frames: (0..4).map(|i| [i as f32 + 1.0; 2]).collect(), looped: None });
        let mut m = TrackMixer::default();
        m.start(1.0, track);
        m.stop(3.0);
        assert_eq!(m.sample(0.5, true, 1.0), [0.0, 0.0], "not yet");
        assert_eq!(m.sample(1.0, true, 1.0), [1.0, 1.0]);
        assert_eq!(m.sample(1.5, false, 1.0), [1.0, 1.0], "held at the last sample, not at zero");
        assert_eq!(m.sample(2.0, true, 1.0), [2.0, 2.0], "and resumes where it was");
        assert_eq!(m.sample(3.0, true, 1.0), [0.0, 0.0]);
        assert!(!m.is_playing());
    }

    #[test]
    fn the_soft_limit_leaves_quiet_sound_alone_and_never_passes_one() {
        for x in [-0.8, -0.3, 0.0, 0.5, 0.8] {
            assert_eq!(soft_limit(x), x);
        }
        let mut last = 0.8;
        for i in 1..=100 {
            let y = soft_limit(0.8 + 0.1 * i as f32);
            assert!((last..1.0).contains(&y) || y == 1.0, "{y}");
            last = y;
        }
        assert!((soft_limit(-5.0) + 1.0).abs() < 1e-3);
        assert!(soft_limit(0.9) < 0.9 && soft_limit(0.9) > 0.8);
    }
}
