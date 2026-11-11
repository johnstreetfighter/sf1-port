//! Ticket #200 (`ST8`): the launcher's side of the Jukebox. A Local fight is
//! driven through the front end as `sf-game` drives it, the session's cues
//! go to the host's deck and mixer, and the music CPU is given the latches
//! the launcher gives it.
//!
//! - With a track set, the stage theme's command is left out of the music
//!   CPU's input and the track is what the mixer plays, from the theme's
//!   frame to the round's end.
//! - A track whose file is missing leaves the native theme: its command
//!   reaches the music CPU, and the log says so.
//! - With no track set, every latch reaches the music CPU as the session
//!   made it.
//!
//! The track is a short tone generated here, in a scratch `jukebox` folder
//! beside a scratch ROM folder. Local ROMs only; each test skips without them.

use std::path::{Path, PathBuf};

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::settings::Settings;
use sf_engine::front::{FrameInputs, GameSession};
use sf_engine::jukebox::{ends_music, Jukebox};
use sf_host::jukebox::{folders, Action, Deck, TrackMixer};
use sf_engine::jukebox::TrackCue;
use sf_host::log::Log;
use sf_video::romset::{load_assets, Assets};
use sf_video::stage::pack_records;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const TEST_STAGE: &str = include_str!("../../../art/stages/test-stage.json");
const FIGHT_LOOP: u32 = 0x3eafa;
const DOWN: usize = 2;

fn idle() -> FrameInputs {
    FrameInputs::default()
}

fn with(f: impl Fn(&mut FrameInputs)) -> FrameInputs {
    let mut i = FrameInputs::default();
    f(&mut i);
    i
}

/// A scratch folder holding `roms/sf` and `roms/jukebox`.
struct Scratch {
    dir: PathBuf,
}

impl Scratch {
    fn new(name: &str) -> Scratch {
        let dir = std::env::temp_dir().join(format!("sf1-port-jukebox-game-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("roms").join("sf")).unwrap();
        std::fs::create_dir_all(dir.join("roms").join("jukebox")).unwrap();
        Scratch { dir }
    }

    fn rom_folder(&self) -> PathBuf {
        self.dir.join("roms").join("sf")
    }

    fn log_path(&self) -> PathBuf {
        self.dir.join("sf-game.log")
    }

    /// A mono 16-bit WAV of a loud 220 Hz tone at 8 kHz, a quarter second.
    fn tone(&self, file: &str) {
        let data: Vec<u8> = (0..2000u32).map(|i| ((f64::from(i) * 220.0 * std::f64::consts::TAU / 8000.0).sin() * 20_000.0) as i16).flat_map(i16::to_le_bytes).collect();
        let mut fmt = Vec::new();
        for v in [1u16, 1] {
            fmt.extend(v.to_le_bytes());
        }
        fmt.extend(8000u32.to_le_bytes());
        fmt.extend(16000u32.to_le_bytes());
        for v in [2u16, 16] {
            fmt.extend(v.to_le_bytes());
        }
        let mut body = b"WAVEfmt ".to_vec();
        body.extend((fmt.len() as u32).to_le_bytes());
        body.extend(&fmt);
        body.extend(b"data");
        body.extend((data.len() as u32).to_le_bytes());
        body.extend(&data);
        let mut out = b"RIFF".to_vec();
        out.extend((body.len() as u32).to_le_bytes());
        out.extend(body);
        std::fs::write(self.dir.join("roms").join("jukebox").join(file), out).unwrap();
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// What the launcher did over a fight.
struct Run {
    /// The latches the music CPU was given, by frame.
    given: Vec<(u32, u8)>,
    /// The latches the session made, by frame.
    made: Vec<(u32, u8)>,
    /// Each made latch sent again on the frame after (a held or repeated
    /// command), as the music CPU would be given it.
    repeats_given: Vec<(u32, u8)>,
    /// Frames of the Start and Stop actions.
    started: Vec<u32>,
    stopped: Vec<u32>,
    /// The first frame of the fight (its machine exists).
    fight_from: u32,
    mixer: TrackMixer,
}

fn rom() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(load_assets(ROMS, MANIFEST).expect("local ROM set assembles"))
}

/// A Local fight on Joe's stage with `jukebox` set, played to the end of its
/// first round or so (the round's stop), the launcher's handling of each
/// frame applied.
fn fight(a: Assets, scratch: &Scratch, jukebox: Jukebox) -> Run {
    let mut bank = pack_records(&a, &[TEST_STAGE]).unwrap();
    bank.add_rom_stages(&a).unwrap();
    bank.add_vs_screens(&a).unwrap();
    let config = MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul);
    let mut session = GameSession::new(a, config).with_settings(Settings { jukebox, ..Settings::default() }).with_background_bank(bank);
    let rom_folder = scratch.rom_folder();
    let mut deck = Deck::new(folders(&[rom_folder.as_path()]));
    let mut log = Log::create(&scratch.log_path());
    let mut run = Run { given: Vec::new(), made: Vec::new(), repeats_given: Vec::new(), started: Vec::new(), stopped: Vec::new(), fight_from: 0, mixer: TrackMixer::default() };
    let mut step = |session: &mut GameSession, run: &mut Run, input: FrameInputs| {
        let out = session.step(input);
        let action = out.track.as_ref().map_or(Action::None, |cue| deck.cue(cue, &mut log));
        // The decode runs on its own thread while the fight's intro plays:
        // by the theme it is done.
        if matches!(out.track, Some(TrackCue::Prepare(_))) {
            deck.settle();
        }
        let given = deck.latch(&action, out.sound_latch);
        if out.sound_latch != 0xff {
            // The same command sent again on a frame of its own.
            let again = deck.latch(&Action::None, out.sound_latch);
            if again != 0xff {
                run.repeats_given.push((out.sound_frame, again));
            }
        }
        let at = f64::from(out.sound_frame) / 60.0;
        match action {
            Action::Start(track) => {
                run.started.push(out.sound_frame);
                run.mixer.start(at, track);
            }
            Action::Stop => {
                run.stopped.push(out.sound_frame);
                run.mixer.stop(at);
            }
            Action::None => {}
        }
        if out.sound_latch != 0xff {
            run.made.push((out.sound_frame, out.sound_latch));
        }
        if given != 0xff {
            run.given.push((out.sound_frame, given));
        }
        if run.fight_from == 0 && session.fight_machine().is_some() {
            run.fight_from = out.sound_frame;
        }
        out.screen
    };
    for _ in 0..200 {
        step(&mut session, &mut run, idle());
    }
    let mut tap = |session: &mut GameSession, run: &mut Run, input: FrameInputs| {
        step(session, run, input);
        step(session, run, idle());
    };
    tap(&mut session, &mut run, with(|i| i.menu.start = true));
    tap(&mut session, &mut run, with(|i| i.p1.directions[DOWN] = true));
    tap(&mut session, &mut run, with(|i| i.menu.confirm = true));
    tap(&mut session, &mut run, with(|i| i.menu.confirm = true));
    tap(&mut session, &mut run, with(|i| i.menu.confirm = true));
    step(&mut session, &mut run, with(|i| i.p2.buttons[0] = true));
    step(&mut session, &mut run, idle());
    assert_eq!(session.screen(), "stage select");
    // Joe's stage is under the cursor.
    step(&mut session, &mut run, with(|i| i.menu.confirm = true));
    let mut n = 0;
    while session.fight_machine().map(flow_position) != Some(FIGHT_LOOP) {
        step(&mut session, &mut run, idle());
        n += 1;
        assert!(n < 4_000, "the fight loop (at {})", session.screen());
    }
    // Player 1 wins a round: the stop at its end comes within the fight.
    for f in 0..20_000u32 {
        let input = with(|i| {
            let phase = f % 120;
            i.p1.directions[0] = phase < 40;
            if phase >= 40 && phase % 10 < 3 {
                i.p1.buttons[((f / 10) % 6) as usize] = true;
            }
        });
        step(&mut session, &mut run, input);
        if run.made.last().is_some_and(|(_, l)| *l == 0x35) {
            break;
        }
    }
    run
}

/// Joe's stage theme: the tune command the ROM's table gives his stage
/// (`rom_stage`, opponent 2), and the frame the session latched it on.
fn theme(run: &Run, maincpu: &[u8]) -> (u32, u8) {
    let command = sf_video::rom_stages::rom_stage(maincpu, 2).unwrap().theme as u8;
    *run.made.iter().find(|(f, l)| *f >= run.fight_from && *l == command).expect("a stage theme")
}

fn peak(mixer: &mut TrackMixer, from: f64, seconds: f64) -> f32 {
    let mut loudest = 0.0f32;
    let mut t = from;
    while t < from + seconds {
        let [l, r] = mixer.sample(t, true, 8000.0);
        loudest = loudest.max(l.abs()).max(r.abs());
        t += 1.0 / 8000.0;
    }
    loudest
}

#[test]
fn a_track_replaces_the_stage_theme_for_the_music_cpu_and_stops_at_the_round_end() {
    let Some(a) = rom() else { return };
    let scratch = Scratch::new("play");
    scratch.tone("joe.wav");
    let mut jukebox = Jukebox::default();
    jukebox.set_stage("joe", "joe.wav");
    let rom = a.maincpu.clone();
    let mut run = fight(a, &scratch, jukebox);
    let (frame, command) = theme(&run, &rom);
    assert_eq!(run.started, [frame], "the track starts on the theme's frame");
    assert!(!run.given.iter().any(|(f, _)| *f == frame), "the theme's command {command:#04x} is left out of the music CPU's input");
    assert!(!run.given.iter().any(|(_, l)| *l == command), "and so is every repeat of it");
    assert!(!run.repeats_given.iter().any(|(f, l)| *l == command && *f >= frame && *f < run.stopped[0]), "a repeated command while the track plays is left out too");
    assert!(run.repeats_given.iter().any(|(_, l)| ends_music(*l)), "stops are not");
    // Every other latch reaches the music CPU as made.
    let others: Vec<_> = run.made.iter().filter(|(f, _)| *f != frame).copied().collect();
    assert_eq!(run.given, others);
    // The mixer plays the tone from that frame, and is silent after the stop.
    let at = f64::from(frame) / 60.0;
    assert_eq!(run.stopped.len(), 1, "stopped once, by the round's end: {:?}", run.stopped);
    let stop = f64::from(run.stopped[0]) / 60.0;
    assert!(stop > at + 1.0, "the round lasted: {at} to {stop}");
    assert!(peak(&mut run.mixer, at - 0.5, 0.4) == 0.0, "silent before");
    assert!(peak(&mut run.mixer, at, 0.2) > 0.3, "the tone plays");
    assert!(peak(&mut run.mixer, at + 0.2, stop - at - 0.4) > 0.3, "and repeats from the top: the 0.25 s file loops for the whole round");
    assert!(peak(&mut run.mixer, stop, 0.5) == 0.0, "silent after the round's end");
    let log = std::fs::read_to_string(scratch.log_path()).unwrap();
    assert!(log.contains("joe.wav") && log.contains("no loop information"), "{log}");
}

#[test]
fn a_missing_track_leaves_the_native_theme_and_says_so() {
    let Some(a) = rom() else { return };
    let scratch = Scratch::new("missing");
    let mut jukebox = Jukebox::default();
    jukebox.set_global("not-there.ogg");
    let rom = a.maincpu.clone();
    let run = fight(a, &scratch, jukebox);
    let (frame, command) = theme(&run, &rom);
    assert!(run.started.is_empty());
    assert!(run.given.contains(&(frame, command)), "the native theme's command reaches the music CPU");
    assert_eq!(run.given, run.made, "every latch as made");
    let log = std::fs::read_to_string(scratch.log_path()).unwrap();
    assert!(log.contains("not-there.ogg") && log.contains("the native theme plays"), "{log}");
}

#[test]
fn with_no_track_set_every_latch_reaches_the_music_cpu_as_made() {
    let Some(a) = rom() else { return };
    let scratch = Scratch::new("none");
    let run = fight(a, &scratch, Jukebox::default());
    assert!(run.started.is_empty() && run.stopped.is_empty());
    assert_eq!(run.given, run.made);
    assert!(std::fs::read_to_string(scratch.log_path()).unwrap_or_default().is_empty());
}
