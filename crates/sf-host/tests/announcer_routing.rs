//! The game's frame output at the host sound boundary (#72, #74).
use std::path::Path;

use sf_audio::{FRAME_HZ, LatchTimeline};
use sf_engine::front::sound;
use sf_host::audio::{Gains, SoundStart};
use sf_host::voice::{AnnouncerFrame, VoicePacks, VoicePlayback};
use sf_video::announcer::{AnnouncerLine as Line, AnnouncerVoice as Voice};
use sf_video::asset_pack::AdpcmClip;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

fn board() -> Option<sf_audio::mix::GameSound> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    let region = |name| sf_video::romset::load_region(ROMS, MANIFEST, name).unwrap();
    let mut board = SoundStart {
        audio2: region("audio2"),
        audiocpu: region("audiocpu"),
        frame: 0,
        preroll: LatchTimeline::default(),
    }.build();
    board.record_port_writes();
    Some(board)
}

fn voices(japanese_fight: bool) -> VoicePacks {
    use sf_audio::announcer::VoiceStore;
    let clip = |nibble| AdpcmClip { sample_rate: 4_000, nibbles: vec![nibble; 4000] };
    let mut japanese = vec![(Line::Round1, clip(6)), (Line::Super, clip(5))];
    if japanese_fight {
        japanese.push((Line::Fight, clip(0xa)));
    }
    VoicePacks::new(
        VoiceStore::from_clips([(Line::Fight, clip(2))]).unwrap(),
        VoiceStore::from_clips(japanese).unwrap(),
    )
}

#[test]
fn japanese_routes_only_a_loaded_overhaul_latch_and_plays_it_before_other_calls() {
    for (voice, overhaul_fight, latch, expected) in [
        (Voice::Japanese, true, 0x97, Some(Line::Round1)),
        (Voice::Japanese, true, 0x94, None),
        (Voice::Ringside, true, 0x97, None),
        (Voice::Japanese, false, 0x97, None),
        (Voice::Japanese, true, 0x9c, None),
        (Voice::Japanese, true, 0x81, None),
    ] {
        let Some(mut board) = board() else { return };
        let mut playback = VoicePlayback::new(voices(false), &mut board);
        for frame in 1..=16 {
            let lines: &[Line] = if frame == 6 && expected.is_some() { &[Line::Super] } else { &[] };
            playback.push_frame(&mut board, AnnouncerFrame {
                frame,
                latch: if frame == 6 { latch } else { sound::IDLE },
                latch_cycles: 0,
                overhaul_fight,
                voice,
                lines,
            });
        }
        let t = 15.0 / f64::from(FRAME_HZ);
        assert!(board.wait_ready(t));
        board.sample_at(t);
        let announcer = board.announcer().unwrap();
        assert_eq!(announcer.playing(), expected, "latch {latch:#x}, {voice:?}, overhaul {overhaul_fight}");
        if expected.is_some() {
            assert_eq!(announcer.waiting(), [Line::Super]);
        }
        let board_played = board.take_port_writes().iter().any(|write| write.port < 2 && write.t > 0.10 && write.data == 0x40);
        assert_eq!(board_played, expected.is_none(), "latch {latch:#x}, {voice:?}, overhaul {overhaul_fight}");
    }
}

#[test]
fn stopping_or_leaving_overhaul_clears_calls_and_menu_can_still_audition() {
    for stop_on_latch in [true, false] {
        let Some(mut board) = board() else { return };
        let mut playback = VoicePlayback::new(voices(false), &mut board);
        for frame in 1..=30 {
            let latch = match frame {
                6 => 0x97,
                12 if stop_on_latch => sound::STOP_SAMPLES,
                _ => sound::IDLE,
            };
            let lines: &[Line] = match frame {
                6 | 12 => &[Line::Super],
                20 => &[Line::Fight],
                _ => &[],
            };
            playback.push_frame(&mut board, AnnouncerFrame {
                frame,
                latch,
                latch_cycles: 0,
                overhaul_fight: frame < if stop_on_latch { 19 } else { 12 },
                voice: Voice::Japanese,
                lines,
            });
        }
        for (frame, expected) in [(8, Some(Line::Round1)), (14, None), (22, Some(Line::Fight))] {
            let t = f64::from(frame) / f64::from(FRAME_HZ);
            assert!(board.wait_ready(t));
            board.sample_at(t);
            assert_eq!(board.announcer().unwrap().playing(), expected, "frame {frame}");
        }
        assert!(board.announcer().unwrap().waiting().is_empty());
    }
}

#[test]
fn changing_voices_restarts_fight_immediately_at_announcer_volume() {
    let Some(mut board) = board() else { return };
    let mut playback = VoicePlayback::new(voices(true), &mut board);
    for frame in 1..=15 {
        let lines: &[Line] = if matches!(frame, 6 | 10) { &[Line::Fight] } else { &[] };
        playback.push_frame(&mut board, AnnouncerFrame {
            frame,
            latch: if lines.is_empty() { sound::IDLE } else { sound::CURSOR },
            latch_cycles: 0,
            overhaul_fight: false,
            voice: if frame < 10 { Voice::Japanese } else { Voice::Ringside },
            lines,
        });
    }
    for (frame, positive) in [(8, false), (12, true)] {
        let t = f64::from(frame) / f64::from(FRAME_HZ);
        assert!(board.wait_ready(t));
        let mix = Gains::from_volumes(10, 0, 0, 4).sample(&mut board, t);
        assert_eq!(board.announcer().unwrap().playing(), Some(Line::Fight));
        assert_ne!(mix[0], 0.0);
        assert_eq!(mix[0] > 0.0, positive, "the newly selected voice on frame {frame}");
        assert_eq!(mix[0], mix[1]);
        assert_eq!(Gains::from_volumes(10, 0, 0, 0).sample(&mut board, t), [0.0, 0.0]);
    }
}
