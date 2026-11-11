//! Ticket #200 (`ST8`): the Jukebox's tracks are read from OGG Vorbis,
//! FLAC and WAV files, and their loop points come from each format's tags.
//!
//! The files in `tests/data` are half a second of a 440 Hz sine at 8 kHz
//! (4000 frames), generated for these tests (original, no music) and made
//! with `ffmpeg -f lavfi -i sine=...`, with the tags the names say.

use sf_host::track::{LoopPoints, Track};

fn load(name: &str) -> Track {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data").join(name);
    Track::load(&path).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn loud(t: &Track) -> f32 {
    t.frames.iter().fold(0.0f32, |m, f| m.max(f[0].abs()))
}

#[test]
fn an_ogg_vorbis_file_decodes_and_loops_at_its_tags() {
    let t = load("loop.ogg");
    assert_eq!((t.rate, t.frames.len()), (8000, 4000));
    assert_eq!(t.looped, Some(LoopPoints { start: 1000, end: 3000 }), "LOOPSTART 1000, LOOPLENGTH 2000");
    assert!(loud(&t) > 0.1, "it is the tone");
}

#[test]
fn a_flac_file_decodes_and_loops_at_its_tags() {
    let t = load("loop.flac");
    assert_eq!((t.rate, t.frames.len()), (8000, 4000));
    assert_eq!(t.looped, Some(LoopPoints { start: 1500, end: 4000 }), "LOOPSTART 1500, LOOPLENGTH 2500");
    assert!(loud(&t) > 0.1);
}

#[test]
fn a_loop_start_alone_loops_to_the_end() {
    assert_eq!(load("start-only.ogg").looped, Some(LoopPoints { start: 1000, end: 4000 }));
}

#[test]
fn a_file_with_no_loop_tags_has_no_loop_and_replays_from_the_top() {
    for name in ["no-loop.ogg", "no-loop.flac"] {
        let t = load(name);
        assert_eq!((t.frames.len(), t.looped), (4000, None), "{name}");
        let mut head = sf_host::track::Playhead::new(std::sync::Arc::new(t.clone()));
        let first: Vec<_> = (0..4000).map(|_| head.next(8000.0)).collect();
        assert_eq!(first, t.frames, "{name}: plays through");
        assert_eq!(head.next(8000.0), t.frames[0], "{name}: and restarts from the top");
    }
}

#[test]
fn a_file_that_is_not_audio_is_an_error_naming_it() {
    let path = std::env::temp_dir().join("sf1-port-track-garbage.ogg");
    std::fs::write(&path, b"not audio").unwrap();
    let e = Track::load(&path).unwrap_err();
    assert!(e.contains("sf1-port-track-garbage.ogg"), "{e}");
    assert!(Track::load(&path.with_extension("gone")).is_err());
}

/// A mono 16-bit 8 kHz WAV whose data chunk says `claimed` bytes, with only
/// `actual` of them there.
fn wav_claiming(claimed: u32, actual: usize) -> Vec<u8> {
    let mut out = b"RIFF".to_vec();
    out.extend((36 + claimed).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16u32.to_le_bytes());
    for v in [1u16, 1] {
        out.extend(v.to_le_bytes());
    }
    out.extend(8000u32.to_le_bytes());
    out.extend(16000u32.to_le_bytes());
    for v in [2u16, 16] {
        out.extend(v.to_le_bytes());
    }
    out.extend(b"data");
    out.extend(claimed.to_le_bytes());
    out.extend(vec![0u8; actual]);
    out
}

#[test]
fn a_file_that_states_a_length_past_the_limit_is_refused_before_it_is_decoded() {
    // 20 MB of sound at 8 kHz is 20 minutes; only a few bytes are there, so
    // a refusal for length is the stated length being read, not the sound.
    let e = Track::decode(wav_claiming(20_000_000, 64), Some("wav")).unwrap_err();
    assert!(e.contains("longer than"), "{e}");
    // The same file under the limit decodes.
    assert!(Track::decode(wav_claiming(16_000, 16_000), Some("wav")).is_ok());
}

#[test]
fn a_decode_nobody_wants_any_more_stops() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/loop.ogg");
    let cancelled = std::sync::atomic::AtomicBool::new(true);
    assert_eq!(Track::load_unless(&path, &cancelled).unwrap_err(), format!("{}: cancelled", path.display()));
    assert!(Track::load_unless(&path, &std::sync::atomic::AtomicBool::new(false)).is_ok());
}
