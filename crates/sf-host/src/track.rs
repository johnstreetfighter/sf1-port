//! The Jukebox's tracks (ADR-0009, design section 6a, ticket #200 `ST8`):
//! decoding an OGG Vorbis, WAV or FLAC file, reading where it loops, and
//! playing it into the mix.
//!
//! - **Decoders.** [symphonia], one pure-Rust library for all three formats
//!   (MPL-2.0, as librashader is). The alternatives were a decoder per
//!   format: `lewton` (Vorbis) and `claxon` (FLAC) have had no release for
//!   years, and `hound` reads WAV only and exposes none of its chunks.
//!   Symphonia is maintained, rates its Vorbis, FLAC and WAV readers
//!   "excellent", hands the Vorbis comments of OGG and FLAC back as tags,
//!   and is the one dependency instead of three. Its WAV reader does not
//!   give the `smpl` chunk, so [`wav_loop`] reads that from the bytes.
//! - **Tracker modules** (ticket #201 `ST9`) are decoded by libopenmpt
//!   instead, in [`crate::tracker`]: see there.
//! - **Loop points** (design 6a.5; *my call* on the names and the rule):
//!   the tags `LOOPSTART` and `LOOPLENGTH`, in sample frames, in OGG and
//!   FLAC (tag names compare without regard to case); in WAV the first
//!   forward loop of the `smpl` chunk, whose end is the last frame played.
//!   `LOOPSTART` alone loops to the end of the file. A file with no loop
//!   information, or loop points that lie outside it, plays through and
//!   restarts from the top. Before the loop start is an intro, played once.
//! - **Playback** is a [`Playhead`] over the decoded frames, stepping by
//!   the file's rate over the device's with linear interpolation, even
//!   across the loop's seam. A track is held decoded as stereo `f32`
//!   (mono on both sides; of more channels the first two), so a file is
//!   limited to [`MAX_SECONDS`].

use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use symphonia::core::audio::GenericAudioBufferRef;
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::{MediaSource, MediaSourceStream};
use symphonia::core::meta::{MetadataOptions, MetadataRevision};

/// The longest track taken: ten minutes. A decoded track is eight bytes a
/// frame, so this bounds one at about 230 MB at 48 kHz.
pub const MAX_SECONDS: u64 = 600;

/// The largest file read: 256 MiB. Checked before anything is read, so a
/// huge file costs nothing; a track within this and [`MAX_SECONDS`] is held
/// decoded.
pub const MAX_FILE_BYTES: u64 = 256 << 20;

/// The tag names of the loop, as the design's section 6a.5 gives them.
pub const LOOP_START_TAG: &str = "LOOPSTART";
pub const LOOP_LENGTH_TAG: &str = "LOOPLENGTH";

/// A loop inside a track, in frames: `start..end` repeats for ever.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoopPoints {
    pub start: u64,
    pub end: u64,
}

/// A decoded track.
#[derive(Clone, Debug, PartialEq)]
pub struct Track {
    /// The file's sample rate (Hz).
    pub rate: u32,
    pub frames: Vec<[f32; 2]>,
    /// Where it loops, if the file says; `None` replays it from the top.
    pub looped: Option<LoopPoints>,
}

impl Track {
    /// The frames a play-through ends at: the loop's end, or the file's.
    fn end(&self) -> u64 {
        self.looped.map_or(self.frames.len() as u64, |l| l.end)
    }

    /// Where playing restarts from at that end.
    fn restart(&self) -> u64 {
        self.looped.map_or(0, |l| l.start)
    }

    /// Read and decode the file at `path`.
    pub fn load(path: &Path) -> Result<Track, String> {
        Track::load_unless(path, &AtomicBool::new(false))
    }

    /// [`Track::load`], giving up (with an error) once `cancel` is set, so a
    /// decode nobody wants any more stops at its next packet.
    pub fn load_unless(path: &Path, cancel: &AtomicBool) -> Result<Track, String> {
        let named = |e: String| format!("{}: {e}", path.display());
        let mut file = std::fs::File::open(path).map_err(|e| named(e.to_string()))?;
        let size = file.metadata().map_err(|e| named(e.to_string()))?.len();
        if size > MAX_FILE_BYTES {
            return Err(named(format!("{} MiB is more than the {} MiB taken", size >> 20, MAX_FILE_BYTES >> 20)));
        }
        let extension = path.extension().map(|e| e.to_string_lossy().to_lowercase());
        let read_all = |file: &mut std::fs::File| -> Result<Vec<u8>, String> {
            let mut bytes = Vec::with_capacity(size as usize);
            file.seek(SeekFrom::Start(0)).map_err(|e| named(e.to_string()))?;
            file.read_to_end(&mut bytes).map_err(|e| named(e.to_string()))?;
            Ok(bytes)
        };
        if extension.as_deref().is_some_and(is_tracker) {
            return Track::module_first(read_all(&mut file)?, extension.as_deref(), cancel).map_err(named);
        }
        let wav = wav_loop_read(&mut file);
        file.seek(SeekFrom::Start(0)).map_err(|e| named(e.to_string()))?;
        match Track::from_source(file.try_clone().map_err(|e| named(e.to_string()))?, wav, extension.as_deref(), cancel) {
            Err(e) if e.starts_with(NOT_AUDIO) => {
                // Not OGG, WAV or FLAC: libopenmpt reads by content, so it is offered the file.
                let module = decode_module(&read_all(&mut file)?, cancel);
                other_format(&e, module).map_err(named)
            }
            other => other.map_err(named),
        }
    }

    /// Decode `bytes`, which `extension` (`ogg`, `flac`, `wav`, a tracker
    /// module's) hints at. Whichever kind the extension names is tried
    /// first, and the other if that fails, so a file with a wrong extension
    /// still plays.
    pub fn decode(bytes: Vec<u8>, extension: Option<&str>) -> Result<Track, String> {
        let cancel = AtomicBool::new(false);
        if extension.is_some_and(is_tracker) {
            return Track::module_first(bytes, extension, &cancel);
        }
        let wav = wav_loop(&bytes);
        let copy = if cfg!(feature = "tracker") { bytes.clone() } else { Vec::new() };
        match Track::from_source(Cursor::new(bytes), wav, extension, &cancel) {
            Err(e) if e.starts_with(NOT_AUDIO) => other_format(&e, decode_module(&copy, &cancel)),
            other => other,
        }
    }

    /// `bytes` as a tracker module and, if libopenmpt cannot play it, as an
    /// OGG, WAV or FLAC file, which a wrong extension may have hidden.
    fn module_first(bytes: Vec<u8>, extension: Option<&str>, cancel: &AtomicBool) -> Result<Track, String> {
        match decode_module(&bytes, cancel) {
            Err(module) if module != CANCELLED => {
                let wav = wav_loop(&bytes);
                Track::from_source(Cursor::new(bytes), wav, extension, cancel).map_err(|audio| format!("{audio}; nor a tracker module ({module})"))
            }
            other => other,
        }
    }

    /// Decode `source`; `wav` is the loop its `smpl` chunk gave, if any. The
    /// length the container states is checked against [`MAX_SECONDS`]
    /// before any sound is decoded.
    fn from_source<S: MediaSource + 'static>(source: S, wav: Option<LoopPoints>, extension: Option<&str>, cancel: &AtomicBool) -> Result<Track, String> {
        let mut hint = Hint::new();
        if let Some(extension) = extension {
            hint.with_extension(extension);
        }
        let stream = MediaSourceStream::new(Box::new(source), Default::default());
        let mut format = symphonia::default::get_probe()
            .probe(&hint, stream, FormatOptions::default(), MetadataOptions::default())
            .map_err(|e| format!("{NOT_AUDIO} ({e})"))?;
        let track = format.default_track(TrackType::Audio).ok_or("no audio in the file")?;
        let id = track.id;
        let stated = track.num_frames;
        let params = track.codec_params.as_ref().and_then(|p| p.audio()).ok_or("no audio in the file")?;
        if let (Some(frames), Some(rate)) = (stated, params.sample_rate) {
            if frames > MAX_SECONDS * u64::from(rate.max(1)) {
                return Err(format!("{} minutes is longer than the {} taken", frames / u64::from(rate.max(1)) / 60, MAX_SECONDS / 60));
            }
        }
        let mut decoder = symphonia::default::get_codecs()
            .make_audio_decoder(params, &AudioDecoderOptions::default())
            .map_err(|e| format!("the codec is not supported ({e})"))?;
        let mut tags = Vec::new();
        collect_tags(format.metadata().current(), &mut tags);
        let (mut rate, mut frames, mut scratch) = (0u32, Vec::<[f32; 2]>::new(), Vec::<f32>::new());
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err(CANCELLED.to_owned());
            }
            let packet = match format.next_packet() {
                Ok(Some(packet)) => packet,
                Ok(None) => break,
                Err(Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(format!("cannot read the file ({e})")),
            };
            while !format.metadata().is_latest() {
                format.metadata().pop();
                collect_tags(format.metadata().current(), &mut tags);
            }
            if packet.track_id != id {
                continue;
            }
            match decoder.decode(&packet) {
                Ok(buffer) => {
                    rate = buffer.spec().rate();
                    push_frames(&buffer, &mut scratch, &mut frames);
                }
                Err(Error::DecodeError(_)) => {}
                Err(e) => return Err(format!("cannot decode the file ({e})")),
            }
            if frames.len() as u64 > MAX_SECONDS * u64::from(rate.max(1)) {
                return Err(format!("longer than {} minutes", MAX_SECONDS / 60));
            }
        }
        collect_tags(format.metadata().current(), &mut tags);
        if frames.is_empty() || rate == 0 {
            return Err("no sound in the file".to_owned());
        }
        let looped = tag_loop(&tags, frames.len() as u64).or_else(|| wav.and_then(|l| clamp(l.start, l.end, frames.len() as u64)));
        Ok(Track { rate, frames, looped })
    }
}

/// The file extensions of everything the Jukebox can play in this build,
/// lower case and without the dot: OGG, WAV and FLAC always, and, only when
/// the `tracker` feature is on, every format libopenmpt reads. The Jukebox
/// screen offers files of these and no others, so a build with no tracker
/// player never offers a module it could not play.
pub fn supported_extensions() -> Vec<String> {
    let mut all: Vec<String> = sf_engine::jukebox::TRACK_EXTENSIONS.iter().map(|e| (*e).to_owned()).collect();
    #[cfg(feature = "tracker")]
    all.extend(sf_openmpt::supported_extensions());
    all.sort();
    all.dedup();
    all
}

/// What the error of a file symphonia cannot place begins with.
const NOT_AUDIO: &str = "not an OGG, WAV or FLAC file";

/// The error of a decode its caller gave up on.
pub(crate) const CANCELLED: &str = "cancelled";

/// Whether `extension` names a tracker module for libopenmpt to read first.
#[cfg(feature = "tracker")]
fn is_tracker(extension: &str) -> bool {
    crate::tracker::is_tracker_extension(extension)
}

#[cfg(not(feature = "tracker"))]
fn is_tracker(_extension: &str) -> bool {
    false
}

/// Decode `bytes` as a tracker module.
#[cfg(feature = "tracker")]
fn decode_module(bytes: &[u8], cancel: &AtomicBool) -> Result<Track, String> {
    crate::tracker::decode(bytes, cancel)
}

#[cfg(not(feature = "tracker"))]
fn decode_module(_bytes: &[u8], _cancel: &AtomicBool) -> Result<Track, String> {
    Err("this build has no tracker support (built without the `tracker` feature)".to_owned())
}

/// The outcome for a file symphonia could not place (`symphonia` is its
/// reason): the module libopenmpt made of it, or both reasons.
fn other_format(symphonia: &str, module: Result<Track, String>) -> Result<Track, String> {
    module.map_err(|m| format!("{symphonia}; nor a tracker module ({m})"))
}

fn collect_tags(revision: Option<&MetadataRevision>, tags: &mut Vec<(String, String)>) {
    let Some(revision) = revision else { return };
    let containers = std::iter::once(&revision.media).chain(revision.per_track.iter().map(|t| &t.metadata));
    for container in containers {
        tags.extend(container.tags.iter().map(|t| (t.raw.key.clone(), t.raw.value.to_string())));
    }
}

/// Append `buffer`'s frames as stereo.
fn push_frames(buffer: &GenericAudioBufferRef<'_>, scratch: &mut Vec<f32>, frames: &mut Vec<[f32; 2]>) {
    let channels = buffer.spec().channels().count().max(1);
    buffer.copy_to_vec_interleaved(scratch);
    frames.extend(scratch.chunks_exact(channels).map(|c| [c[0], c[channels.min(2) - 1]]));
}

/// `start..end` as loop points inside a track of `len` frames, if they lie
/// in it and advance.
fn clamp(start: u64, end: u64, len: u64) -> Option<LoopPoints> {
    (start < end && end <= len).then_some(LoopPoints { start, end })
}

/// The loop the tags of an OGG or FLAC file give for a track of `len`
/// frames. `LOOPSTART` alone loops to the end; a `LOOPLENGTH` past the end
/// is cut to it; `LOOPLENGTH` without `LOOPSTART` says nothing.
pub fn tag_loop(tags: &[(String, String)], len: u64) -> Option<LoopPoints> {
    let find = |name: &str| tags.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).and_then(|(_, v)| v.trim().parse::<u64>().ok());
    let start = find(LOOP_START_TAG)?;
    let end = match find(LOOP_LENGTH_TAG) {
        Some(length) if length > 0 => start.saturating_add(length).min(len),
        _ => len,
    };
    clamp(start, end, len)
}

/// The first forward loop of a WAV file's `smpl` chunk (start, and end as
/// the last frame played, so the loop is `start..end + 1`), or `None` if
/// the bytes are no RIFF WAVE or have no such loop. Not checked against the
/// file's length: [`Track::decode`] does that.
pub fn wav_loop(bytes: &[u8]) -> Option<LoopPoints> {
    wav_loop_read(&mut Cursor::new(bytes))
}

/// [`wav_loop`] over a file: only the chunk headers and the `smpl` chunk
/// are read, never the sound.
pub fn wav_loop_read<R: Read + Seek>(r: &mut R) -> Option<LoopPoints> {
    fn word<R: Read>(r: &mut R) -> Option<usize> {
        let mut b = [0u8; 4];
        r.read_exact(&mut b).ok()?;
        Some(u32::from_le_bytes(b) as usize)
    }
    let mut head = [0u8; 12];
    r.read_exact(&mut head).ok()?;
    if &head[..4] != b"RIFF" || &head[8..] != b"WAVE" {
        return None;
    }
    loop {
        let mut id = [0u8; 4];
        r.read_exact(&mut id).ok()?;
        let size = word(r)?;
        if &id != b"smpl" {
            r.seek(SeekFrom::Current((size + (size & 1)) as i64)).ok()?;
            continue;
        }
        let mut body = vec![0u8; size.min(36 + 24 * 64)];
        r.read_exact(&mut body).ok()?;
        let at = |i: usize| body.get(i..i + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize);
        for k in 0..at(28)?.min(64) {
            let entry = 36 + 24 * k;
            // cue id, type (0 forward), start, end, fraction, play count
            if at(entry + 4)? == 0 {
                let (start, end) = (at(entry + 8)? as u64, at(entry + 12)? as u64);
                return Some(LoopPoints { start, end: end + 1 }).filter(|l| l.start < l.end);
            }
        }
        return None;
    }
}

/// A track being played: a position in its frames, stepped at the device's
/// rate.
#[derive(Clone, Debug)]
pub struct Playhead {
    track: Arc<Track>,
    /// The position in the file's frames.
    pos: f64,
}

impl Playhead {
    pub fn new(track: Arc<Track>) -> Self {
        Playhead { track, pos: 0.0 }
    }

    /// The position, in the file's frames.
    pub fn position(&self) -> f64 {
        self.pos
    }

    /// The next output frame at `out_rate` Hz, the head moved on past it.
    pub fn next(&mut self, out_rate: f64) -> [f32; 2] {
        let t = &*self.track;
        let (end, restart) = (t.end() as usize, t.restart() as usize);
        let i = (self.pos.floor() as usize).min(end - 1);
        let frac = (self.pos - i as f64) as f32;
        let a = t.frames[i];
        let b = t.frames[if i + 1 >= end { restart } else { i + 1 }];
        self.pos += f64::from(t.rate) / out_rate;
        while self.pos >= end as f64 {
            self.pos -= (end - restart) as f64;
        }
        [a[0] + (b[0] - a[0]) * frac, a[1] + (b[1] - a[1]) * frac]
    }
}

#[cfg(test)]
pub(crate) mod fixtures {
    /// A mono 16-bit WAV of `frames` frames at 8 kHz with `extra` chunks
    /// between the format and the data.
    pub(crate) fn wav(frames: u32, extra: &[u8]) -> Vec<u8> {
        let mut fmt = Vec::new();
        for v in [1u16, 1] {
            fmt.extend(v.to_le_bytes());
        }
        fmt.extend(8000u32.to_le_bytes());
        fmt.extend(16000u32.to_le_bytes());
        for v in [2u16, 16] {
            fmt.extend(v.to_le_bytes());
        }
        let data: Vec<u8> = (0..frames).flat_map(|i| (((i % 100) as i16 - 50) * 100).to_le_bytes()).collect();
        let mut body = b"WAVE".to_vec();
        body.extend(b"fmt ");
        body.extend((fmt.len() as u32).to_le_bytes());
        body.extend(&fmt);
        body.extend(extra);
        body.extend(b"data");
        body.extend((data.len() as u32).to_le_bytes());
        body.extend(&data);
        let mut out = b"RIFF".to_vec();
        out.extend((body.len() as u32).to_le_bytes());
        out.extend(body);
        out
    }

    /// A `smpl` chunk with the loops `(type, start, end)`.
    pub(crate) fn smpl(loops: &[(u32, u32, u32)]) -> Vec<u8> {
        let mut body = vec![0u8; 28];
        body.extend((loops.len() as u32).to_le_bytes());
        body.extend(0u32.to_le_bytes());
        for (i, (kind, start, end)) in loops.iter().enumerate() {
            for v in [i as u32, *kind, *start, *end, 0, 0] {
                body.extend(v.to_le_bytes());
            }
        }
        let mut chunk = b"smpl".to_vec();
        chunk.extend((body.len() as u32).to_le_bytes());
        chunk.extend(body);
        chunk
    }

}

#[cfg(test)]
mod tests {
    use super::fixtures::{smpl, wav};
    use super::*;

    fn tags(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect()
    }

    #[test]
    fn the_tags_give_the_loop() {
        let lp = |pairs: &[(&str, &str)], len| tag_loop(&tags(pairs), len);
        assert_eq!(lp(&[("LOOPSTART", "100"), ("LOOPLENGTH", "300")], 1000), Some(LoopPoints { start: 100, end: 400 }));
        assert_eq!(lp(&[("loopstart", "100"), ("LoopLength", "300")], 1000), Some(LoopPoints { start: 100, end: 400 }), "case does not matter");
        assert_eq!(lp(&[("LOOPSTART", "100")], 1000), Some(LoopPoints { start: 100, end: 1000 }), "to the end");
        assert_eq!(lp(&[("LOOPSTART", "100"), ("LOOPLENGTH", "9999")], 1000), Some(LoopPoints { start: 100, end: 1000 }), "cut to the end");
        assert_eq!(lp(&[("LOOPSTART", " 0 "), ("LOOPLENGTH", "0")], 1000), Some(LoopPoints { start: 0, end: 1000 }));
    }

    #[test]
    fn no_usable_tags_means_no_loop() {
        let lp = |pairs: &[(&str, &str)]| tag_loop(&tags(pairs), 1000);
        assert_eq!(lp(&[]), None);
        assert_eq!(lp(&[("LOOPLENGTH", "300")]), None, "a length without a start says nothing");
        assert_eq!(lp(&[("LOOPSTART", "soon")]), None);
        assert_eq!(lp(&[("LOOPSTART", "1000")]), None, "a start at the end does not advance");
        assert_eq!(lp(&[("LOOPSTART", "5000")]), None);
        assert_eq!(lp(&[("TITLE", "x")]), None);
    }

    #[test]
    fn a_wav_loop_chunk_gives_the_loop() {
        let bytes = wav(1000, &smpl(&[(0, 200, 599)]));
        assert_eq!(wav_loop(&bytes), Some(LoopPoints { start: 200, end: 600 }), "the end is the last frame played");
        let t = Track::decode(bytes, Some("wav")).unwrap();
        assert_eq!((t.rate, t.frames.len(), t.looped), (8000, 1000, Some(LoopPoints { start: 200, end: 600 })));
        // The first forward loop; a backward one is not taken.
        assert_eq!(wav_loop(&wav(1000, &smpl(&[(1, 10, 20), (0, 30, 40)]))), Some(LoopPoints { start: 30, end: 41 }));
        assert_eq!(wav_loop(&wav(1000, &smpl(&[(1, 10, 20)]))), None);
        // A chunk of odd size before it is padded to even.
        let mut odd = b"junk\x03\0\0\0abc\0".to_vec();
        odd.extend(smpl(&[(0, 5, 9)]));
        assert_eq!(wav_loop(&wav(100, &odd)), Some(LoopPoints { start: 5, end: 10 }));
    }

    #[test]
    fn a_wav_without_a_loop_chunk_or_with_one_outside_it_has_no_loop() {
        assert_eq!(Track::decode(wav(1000, &[]), Some("wav")).unwrap().looped, None);
        assert_eq!(Track::decode(wav(1000, &smpl(&[(0, 200, 5000)])), Some("wav")).unwrap().looped, None);
        assert_eq!(Track::decode(wav(1000, &smpl(&[(0, 600, 200)])), Some("wav")).unwrap().looped, None);
        assert_eq!(wav_loop(b"RIFX"), None);
        assert_eq!(wav_loop(b""), None);
    }

    fn ramp(len: usize, rate: u32, looped: Option<LoopPoints>) -> Arc<Track> {
        Arc::new(Track { rate, frames: (0..len).map(|i| [i as f32, -(i as f32)]).collect(), looped })
    }

    fn played(head: &mut Playhead, n: usize) -> Vec<f32> {
        (0..n).map(|_| head.next(1.0)[0]).collect()
    }

    #[test]
    fn a_loop_plays_its_intro_once_then_repeats_between_the_points() {
        let mut head = Playhead::new(ramp(10, 1, Some(LoopPoints { start: 3, end: 7 })));
        assert_eq!(played(&mut head, 14), [0., 1., 2., 3., 4., 5., 6., 3., 4., 5., 6., 3., 4., 5.]);
    }

    #[test]
    fn with_no_loop_information_it_replays_from_the_top() {
        let mut head = Playhead::new(ramp(4, 1, None));
        assert_eq!(played(&mut head, 10), [0., 1., 2., 3., 0., 1., 2., 3., 0., 1.]);
    }

    #[test]
    fn the_seam_is_interpolated_and_the_rate_is_converted() {
        // A file at 1 Hz read at 2 Hz: half a frame a step, and at the loop
        // end the half-way value is between the last frame and the start.
        let mut head = Playhead::new(ramp(4, 1, Some(LoopPoints { start: 1, end: 4 })));
        let out: Vec<f32> = (0..8).map(|_| head.next(2.0)[0]).collect();
        assert_eq!(out, [0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 2.0]);
        // Stereo is kept side by side.
        let mut head = Playhead::new(ramp(4, 1, None));
        assert_eq!(head.next(1.0), [0.0, 0.0]);
        assert_eq!(head.next(1.0), [1.0, -1.0]);
    }
}
