//! Ticket #153 (`FE2`): the menus' sound through the existing mixer. The
//! headless session's latch stream from power-on drives the translated
//! sound programs (`sf_host::audio::LiveCapture`, the live output path) and
//! the output is measured: the attract's music plays, stops when the Main
//! Menu opens, the idle menu is silent, and cursor movement and confirm
//! each sound. With menu music enabled, the title cue carries into the
//! menus and the original country-select cue takes over at fighter select.
//! Local ROMs only; skips without them.

use std::path::Path;
use std::sync::Arc;

use sf_audio::{score, tunes::ExtendedTunes};
use sf_engine::config::MatchConfig;
use sf_engine::front::settings::Settings;
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

#[test]
fn title_theme_carries_into_the_menu_and_gives_way_to_the_country_select_tune() {
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

    // A single FM voice with a deliberately detuned pitch distinguishes the
    // optional title cue from the native melody and navigation effects.
    let frequency = 1760.0 * 2.0_f64.powf(0.75 / 12.0);
    let title = format!(
        "command {:#04x}\npriority 8\nunit 1\nchannel 0\nvoice 16 8 0 native\npan both\nforever {{ raw 0x6a 192 60000 0 }}",
        sound::TITLE_THEME
    );
    let mut table = ExtendedTunes::new();
    score::convert(&title, &start.audiocpu).unwrap().add_to(&mut table).unwrap();
    let table = Arc::new(table.image().to_vec());

    const FIRST: u32 = 1260;
    const LAST: u32 = 1860;
    let mut capture = LiveCapture::new(&start, FIRST, LAST);
    let mut settings = Settings::default();
    settings.jukebox.set_global("fight.ogg");
    let mut session = GameSession::new(assets, MatchConfig::default()).with_settings(settings);
    session.set_title_theme_available(true);
    let mut has_table = false;
    let mut latches = Vec::new();
    for frame in 1..=LAST {
        let mut input = FrameInputs::default();
        let expected_screen = match frame {
            1288 => Some("attract"),
            1500 => { input.menu.start = true; Some("main menu") }
            1560 => { input.menu.confirm = true; Some("single player") }
            1620 => { input.menu.confirm = true; Some("select") }
            1740 => { input.menu.back = true; Some("single player") }
            1800 => { input.menu.back = true; Some("main menu") }
            _ => None,
        };
        let out = session.step(input);
        if let Some(screen) = expected_screen {
            assert_eq!(out.screen, screen, "frame {frame}");
        }
        assert!(out.track.is_none(), "the Jukebox only replaces fight music");
        if has_table != out.tune_table {
            has_table = out.tune_table;
            capture.set_tune_table(has_table.then(|| table.clone()));
        }
        if out.sound_latch != sound::IDLE {
            latches.push((frame, out.sound_latch));
        }
        capture.push_frame(out.sound_frame, out.sound_latch, out.latch_cycles);
    }
    assert!(capture.unrecovered().is_empty(), "sound programs stayed translated");

    let starts = |cmd| {
        latches.iter().filter_map(|&(frame, latch)| (cmd == latch).then_some(frame)).collect::<Vec<_>>()
    };
    let title_starts = starts(sound::TITLE_THEME);
    assert_eq!(title_starts.len(), 2, "one start at the title and one after character select");
    assert!((1288..1293).contains(&title_starts[0]), "{title_starts:?}");
    assert!((1740..1746).contains(&title_starts[1]), "{title_starts:?}");
    let country_starts = starts(sound::COUNTRY_SELECT);
    assert_eq!(country_starts.len(), 1);
    assert!((1620..1626).contains(&country_starts[0]), "{country_starts:?}");
    for (enter, start) in [(1620, country_starts[0]), (1740, title_starts[1])] {
        assert!(latches.iter().any(|&(f, l)| (enter..start).contains(&f) && l == sound::STOP_MUSIC),
            "stop the old tune before starting the next at {start}");
    }
    assert!(latches.iter().all(|&(f, latch)| !(1500..1620).contains(&f) || !matches!(latch, sound::STOP_ALL | sound::STOP_MUSIC)),
        "the title cue plays through opening the main menu and its submenu");

    let samples = capture.into_samples();
    for (first, last, screen) in [(1350, 1390, "title"), (1510, 1540, "main menu"), (1580, 1600, "submenu"),
        (1660, 1690, "character select"), (1820, 1850, "return to main menu")] {
        let level = rms(&samples, first, last, FIRST);
        assert!(level > 0.005, "music is audible in {screen} ({level})");
    }

    let title_pitch = |frame| {
        let at = ((frame - FIRST) as f64 * RATE / 60.0) as usize;
        let count = (RATE / 10.0) as usize;
        let (sin, cos) = samples[at..at + count].iter().enumerate().fold(
            (0.0, 0.0),
            |(sin, cos), (i, &sample)| {
                let (s, c) = (std::f64::consts::TAU * frequency * i as f64 / RATE).sin_cos();
                let sample = f64::from(sample) * (0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / count as f64).cos());
                (sin + sample * s, cos + sample * c)
            },
        );
        sin.hypot(cos) * 2.0 / count as f64
    };
    let main = title_pitch(1510);
    let select = title_pitch(1660);
    let returned = title_pitch(1820);
    println!("title pitch amplitude: menu {main:.6}, select {select:.6}, returned {returned:.6}");
    assert!(main > 0.005 && returned > 0.005, "the title's test pitch plays in the main menu");
    assert!(select < main.min(returned) / 10.0, "the title's test pitch is gone at character select");
}
