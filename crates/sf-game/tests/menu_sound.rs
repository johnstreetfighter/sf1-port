//! Ticket #153 (`FE2`): the menus' sound through the existing mixer. The
//! headless session's latch stream from power-on drives the translated
//! sound programs (`sf_host::audio::LiveCapture`, the live output path) and
//! the output is measured: the attract's music plays, stops when the Main
//! Menu opens, the idle menu is silent, and cursor movement and confirm
//! each sound. Local ROMs only; skips without them.

use std::path::Path;

use sf_engine::config::MatchConfig;
use sf_engine::front::{sound, FrameInputs, GameSession};
use sf_host::audio::{LiveCapture, SoundStart};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

const RATE: f64 = sf_audio::check::WAV_RATE as f64;

fn rms(samples: &[f32], first: u32, last: u32, from: u32) -> f64 {
    let at = |f: u32| (((f - from) as f64 / 60.0) * RATE) as usize;
    let window = &samples[at(first)..at(last + 1).min(samples.len())];
    (window.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / window.len() as f64).sqrt()
}

#[test]
fn attract_music_stops_under_the_menu_and_cursor_and_confirm_sound() {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).unwrap();
    let region = |name| sf_video::romset::load_region(ROMS, MANIFEST, name).unwrap();
    let start = SoundStart {
        audio2: region("audio2"),
        audiocpu: region("audiocpu"),
        frame: 0,
        preroll: sf_audio::LatchTimeline::default(),
    };
    const FIRST: u32 = 2150;
    const LAST: u32 = 2650;
    let mut capture = LiveCapture::new(&start, FIRST, LAST);
    let mut session = GameSession::new(assets, MatchConfig::default());

    // The demo fight's round music (0x28) starts at 1859; Start at 2300
    // opens the menu, down at 2421 moves the cursor, confirm at 2501 opens
    // Single Player.
    let mut latches = Vec::new();
    for f in 1..=LAST {
        let mut input = FrameInputs::default();
        match f {
            2300 => input.menu.start = true,
            2421 => input.p1.directions[2] = true,
            2501 => input.menu.confirm = true,
            _ => {}
        }
        let out = session.step(input);
        assert_eq!(out.sound_frame, f);
        latches.push((f, out.sound_latch));
        capture.push_frame(out.sound_frame, out.sound_latch, out.latch_cycles);
    }
    assert!(capture.unrecovered().is_empty(), "sound programs stayed translated");
    let menu_latches: Vec<_> = latches.iter().filter(|(f, l)| *f >= 2300 && *l != sound::IDLE).collect();
    assert_eq!(
        menu_latches,
        [&(2300, sound::STOP_ALL), &(2301, sound::STOP_SAMPLES), &(2421, sound::CURSOR), &(2501, sound::CONFIRM)],
        "the menus send only the stop and their two effects"
    );

    let out = capture.into_samples();
    let music = rms(&out, 2200, 2299, FIRST);
    let silence = rms(&out, 2315, 2420, FIRST);
    let cursor = rms(&out, 2422, 2445, FIRST);
    let after_cursor = rms(&out, 2470, 2500, FIRST);
    let confirm = rms(&out, 2502, 2540, FIRST);
    println!("rms: music {music:.5}, menu idle {silence:.6}, cursor {cursor:.5}, after {after_cursor:.6}, confirm {confirm:.5}");
    assert!(music > 0.01, "the attract's music plays before the menu ({music})");
    assert!(silence < 0.001, "the attract's sound stops under the menu ({silence})");
    assert!(cursor > 0.005, "cursor movement sounds effect 0x09 ({cursor})");
    assert!(after_cursor < 0.001, "the effect ends and the menu is silent again ({after_cursor})");
    assert!(confirm > 0.005, "confirm sounds effect 0x0f ({confirm})");
}
