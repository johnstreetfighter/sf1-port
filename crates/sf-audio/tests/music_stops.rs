//! Acceptance tests for ticket #33: the music CPU's special commands, stops
//! and tune ends, translated with their T-state timing, against the pinned
//! MAME reference over windows where a stop is followed by a new tune:
//!
//! - **A** (sf-music-02, frames 1864..=2109): tune 0x34 live, the stop 0x3f
//!   (0x03a4) at 1866, 92 idle frames, tune 0x33 from 1959.
//! - **B** (sf-music-02, 2274..=2357): special command 0x27 (0x02b8: both
//!   effect slots stopped, the channels handed back to the live tune 0x33
//!   through 0x032b, twice), the 0x3f at 2277 the music CPU never reads
//!   (its handler outlasts the latch), tune 0x28 from 2280 — the phrase
//!   ticket #11 verified.
//! - **C** (sf-win-01, 5059..=5583): tune 0x37 to its sustained final chord,
//!   the stop on the chord at 5429, the repeated stop at 5430 the intake
//!   rejects, tune 0x33 from 5431.
//! - **D** (sf-music-03, 7185..=7385): tune 0x39 (the attract ranking screen's tune at 0x4268c, not a game-over tune; ticket #56) starts at
//!   7186 and ends by itself (opcode 0xf, 0x09e7 → 0x03a7) at 7301.
//!
//! In each: every YM2151 write matches in order within one output sample,
//! the chip output is sample-identical over the hooked stream, and the
//! music CPU's RAM equals the reference's checkpoint dumps at chosen
//! frames. docs/reference/music-stops.md. Local-only data; skips without it.

use std::path::Path;

use sf_audio::check::{
    exact_clock_matches, latch_timeline, read_device_stream, read_taps, reference_opm_writes,
    run_music, MusicRun, RefLog,
};
use sf_audio::opm::OPM_RATE;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const MUSIC_02_A: &str = "../../tools/reference/local/captures/music-02-audio-a";
const MUSIC_02_B: &str = "../../tools/reference/local/captures/music-02-audio-b";
const MUSIC_03_A: &str = "../../tools/reference/local/captures/music-03-audio-a";
const MUSIC_03_B: &str = "../../tools/reference/local/captures/music-03-audio-b";
const WIN_01_AUDIO: &str = "../../tools/reference/local/captures/win-01-audio";
const WIN_01_HOOK: &str = "../../tools/reference/local/captures/win-01-audio-hook";

/// A verification window: the audio capture (taps, hook, RAM dumps), the
/// fixture frame (the RAM dump at its end starts the native run), the last
/// frame compared, and the idle-loop phase at the fixture (derived with the
/// `music_phase` example: the phase that puts the most native writes on the
/// reference's exact chip clock; docs/reference/music-stops.md). The
/// fixture, the last frame and the frames whose RAM is compared are frame
/// ends at which the music CPU is idle — no handler in progress, judged
/// from the reference's write times and the timers' expiry grid — since a
/// RAM dump taken inside a handler is a state the native run (which ends
/// after a handler completes) never shows.
struct Window {
    capture: &'static str,
    hook: &'static str,
    fixture: u32,
    last: u32,
    phase: u64,
}

const A: Window = Window {
    capture: MUSIC_02_A,
    hook: MUSIC_02_A,
    fixture: 1864,
    last: 2109,
    phase: WINDOW_A_PHASE,
};
const B: Window = Window {
    capture: MUSIC_02_A,
    hook: MUSIC_02_A,
    fixture: 2274,
    last: 2357,
    phase: 12,
};
const C: Window = Window {
    capture: WIN_01_AUDIO,
    hook: WIN_01_HOOK,
    fixture: 5059,
    last: 5583,
    phase: WINDOW_C_PHASE,
};
const D: Window = Window {
    capture: MUSIC_03_A,
    hook: MUSIC_03_A,
    fixture: 7185,
    last: 7385,
    phase: WINDOW_D_PHASE,
};
const WINDOW_A_PHASE: u64 = 8;
const WINDOW_C_PHASE: u64 = 2;
const WINDOW_D_PHASE: u64 = 7;

fn present(w: &Window) -> bool {
    [w.capture, w.hook, ROMS]
        .iter()
        .all(|p| Path::new(p).is_dir())
}

fn rom() -> Vec<u8> {
    sf_video::romset::load_region(ROMS, MANIFEST, "audiocpu").expect("audiocpu region")
}

fn music_ram(capture: &str, frame: u32) -> Vec<u8> {
    std::fs::read(Path::new(capture).join(format!("ram_{frame:06}_music.bin")))
        .expect("music RAM dump")
}

fn sample_at_end_of(frame: u32) -> u64 {
    ((frame + 1) as f64 / 60.0 * OPM_RATE as f64).ceil() as u64
}

/// Run the window natively from its fixture through the end of `last`.
fn run(w: &Window, log: &RefLog, last: u32) -> MusicRun {
    let t_fix = (w.fixture + 1) as f64 / 60.0;
    run_music(
        rom(),
        music_ram(w.capture, w.fixture),
        log,
        t_fix,
        latch_timeline(log, None),
        w.phase,
        sample_at_end_of(last),
    )
}

/// The music CPU's RAM the translation keeps (0xc000-0xc5cf: globals,
/// channels, saved channels, effect slots) against a checkpoint's dump.
fn assert_ram_matches(run: &MusicRun, capture: &str, frame: u32) {
    let reference = music_ram(capture, frame);
    let differing: Vec<String> = (0..0x5d0)
        .filter(|&i| run.ram[i] != reference[i])
        .map(|i| {
            format!(
                "{:04x}: native {:02x} reference {:02x}",
                0xc000 + i,
                run.ram[i],
                reference[i]
            )
        })
        .collect();
    assert!(
        differing.is_empty(),
        "music RAM at the end of frame {frame}: {} bytes differ:\n{}",
        differing.len(),
        differing.join("\n")
    );
}

/// Every reference write of the window matches in order within one output
/// sample; the native run reaches no untranslated branch; the phase puts
/// most writes on the exact clock; the hooked output is sample-identical.
fn check_window(w: &Window, expect_min_samples: usize) -> (MusicRun, RefLog) {
    let log = read_taps(&Path::new(w.capture).join("audio_taps.tsv")).expect("tap log");
    let run = run(w, &log, w.last);
    assert!(
        run.unrecovered.is_empty(),
        "translated paths only: {:04x?}",
        run.unrecovered
    );
    let (s_fix, s_end) = (sample_at_end_of(w.fixture), sample_at_end_of(w.last));
    let reference: Vec<_> = reference_opm_writes(&log)
        .into_iter()
        .filter(|x| x.sample >= s_fix && x.sample < s_end)
        .collect();
    let native: Vec<_> = run
        .writes
        .iter()
        .copied()
        .filter(|x| x.sample < s_end)
        .collect();
    assert_eq!(native.len(), reference.len(), "every write");
    for (i, (r, n)) in reference.iter().zip(&native).enumerate() {
        assert_eq!((r.reg, r.data), (n.reg, n.data), "write #{i} in order");
        assert!(
            r.sample.abs_diff(n.sample) <= 1,
            "write #{i} within one sample ({} vs {})",
            n.sample,
            r.sample
        );
    }
    let exact = exact_clock_matches(&native, &reference);
    assert!(
        exact + 8 >= native.len(),
        "all but the fixture's first handler on the exact chip clock ({exact} of {})",
        native.len()
    );

    let hook =
        read_device_stream(Path::new(w.hook), "ymsnd", OPM_RATE as f64).expect("ymsnd hook stream");
    let n0 = hook.first_sample();
    let mut compared = 0;
    for (i, r) in hook.samples.iter().enumerate() {
        let n = n0 + i as u64;
        if n >= s_fix && n < s_end {
            assert_eq!(run.left[n as usize], *r, "output sample {n}");
            compared += 1;
        }
    }
    assert!(
        compared >= expect_min_samples,
        "the window's output is compared ({compared} samples)"
    );
    println!("window {}..={}: {} writes, {exact} on the exact clock, {compared} output samples identical", w.fixture, w.last, native.len());
    (run, log)
}

#[test]
fn reference_runs_repeat_exactly() {
    for (a, b) in [(MUSIC_02_A, MUSIC_02_B), (MUSIC_03_A, MUSIC_03_B)] {
        if !(Path::new(a).is_dir() && Path::new(b).is_dir()) {
            eprintln!("skipping {a}: local reference audio runs not present");
            continue;
        }
        for f in ["audio_taps.tsv", "hook_ymsnd.f32", "checkpoints.tsv"] {
            let x = std::fs::read(Path::new(a).join(f)).unwrap();
            let y = std::fs::read(Path::new(b).join(f)).unwrap();
            assert!(x == y, "{a}/{f} must repeat exactly");
        }
    }
}

/// Window A: the stop of a live tune, then a tune start.
#[test]
fn a_stop_of_the_live_tune_then_a_tune_start_play_like_the_reference() {
    if !present(&A) {
        eprintln!("skipping: local reference audio runs/ROMs not present");
        return;
    }
    let (run, log) = check_window(&A, 225_000);
    // The stop (0x3f at 1866, 0x03a4): the tune unloaded, every channel's
    // stream pointer zero, the saved slot empty.
    let stopped = self::run(&A, &log, 1868);
    assert_ram_matches(&stopped, A.capture, 1868);
    assert_eq!(stopped.ram[0x011], 0, "no tune after the stop");
    assert!(
        (0..8).all(|ch| stopped.ram[0x100 + 0x30 * ch] == 0 && stopped.ram[0x101 + 0x30 * ch] == 0),
        "channels unloaded"
    );
    // The start of 0x33 at 1959 (an empty current tune: the 0x01e5 path).
    let started = self::run(&A, &log, 1985);
    assert_ram_matches(&started, A.capture, 1985);
    assert_eq!(started.ram[0x011], 0x08, "tune 0x33 loaded (priority 8)");
    assert_ram_matches(&run, A.capture, A.last);
}

/// Window B: special command 0x27 over a live tune (the hand-back twice),
/// the stop the handler outlasts, the tune start the phrase test verified.
#[test]
fn special_command_0x27_hands_the_channels_back_and_the_late_stop_is_missed() {
    if !present(&B) {
        eprintln!("skipping: local reference audio runs/ROMs not present");
        return;
    }
    let (run, log) = check_window(&B, 74_000);
    // The 0x27 handler (37.9198 s on) outlasts the 0x3f on the latch
    // (37.950094..37.966765 s), so the intake never sees it: the next
    // command taken is the tune 0x28 (a stop taken would have added its 40
    // mute / key-off writes, which the write comparison above excludes).
    let after = self::run(&B, &log, 2282);
    assert_ram_matches(&after, B.capture, 2282);
    assert_eq!(
        after.ram[0x011], 0x08,
        "a priority-8 tune (0x28) is loaded after the special command"
    );
    let handler_end = after
        .writes
        .iter()
        .filter(|w| w.clock < (37.98 * sf_audio::opm::OPM_CLOCK as f64) as u64)
        .map(|w| w.clock)
        .max()
        .unwrap();
    assert!(
        handler_end as f64 / sf_audio::opm::OPM_CLOCK as f64 > 37.966765,
        "the 0x27 handler's last write ({:.6} s) comes after the latch returned to idle",
        handler_end as f64 / sf_audio::opm::OPM_CLOCK as f64
    );
    assert_ram_matches(&run, B.capture, B.last);
}

/// Window C: a tune to its final chord, the stop on the chord, the repeated
/// stop rejected, a tune start.
#[test]
fn stop_on_a_held_chord_and_a_repeated_stop_play_like_the_reference() {
    if !present(&C) {
        eprintln!("skipping: local reference audio runs/ROMs not present");
        return;
    }
    let (run, log) = check_window(&C, 480_000);
    let chord = self::run(&C, &log, 5428);
    assert_ram_matches(&chord, C.capture, 5428);
    assert_eq!(
        chord.ram[0x011], 0x08,
        "tune 0x37 still loaded, holding its final chord"
    );
    // The stop at 5429 is taken (0x03a4); its repeat at 5430 equals the
    // last command taken and the intake ignores it (a second stop would
    // add 40 mute / key-off writes, which the write comparison excludes).
    let stopped = self::run(&C, &log, 5435);
    assert_ram_matches(&stopped, C.capture, 5435);
    assert_eq!(
        stopped.ram[0x011], 0x08,
        "tune 0x33 (priority 8) loaded after the stop and its ignored repeat"
    );
    assert_ram_matches(&run, C.capture, C.last);
}

/// Window D: a tune that ends by itself (opcode 0xf).
#[test]
fn a_tune_ending_by_itself_plays_like_the_reference() {
    if !present(&D) {
        eprintln!("skipping: local reference audio runs/ROMs not present");
        return;
    }
    let (run, log) = check_window(&D, 180_000);
    let playing = self::run(&D, &log, 7249);
    assert_ram_matches(&playing, D.capture, 7249);
    assert_eq!(playing.ram[0x011], 0x08, "tune 0x39 playing");
    let ended = self::run(&D, &log, 7309);
    assert_ram_matches(&ended, D.capture, 7309);
    assert_eq!(
        ended.ram[0x011], 0,
        "the tune ended by itself: nothing loaded, no stop command"
    );
    assert!(
        (0..8).all(|ch| ended.ram[0x100 + 0x30 * ch] == 0 && ended.ram[0x101 + 0x30 * ch] == 0),
        "channels unloaded"
    );
    assert_ram_matches(&run, D.capture, D.last);
}
