//! Separate score storage at the live native audio boundary (#77, #78).

use std::path::Path;
use std::sync::Arc;

use sf_audio::check::{run_scored_music_from_power_on, MusicRun};
use sf_audio::mix::GameSound;
use sf_audio::opm::OPM_RATE;
use sf_audio::score::{compile, NativeScore};
use sf_audio::tunes::ExtendedTunes;
use sf_audio::{LatchTimeline, FRAME_HZ, IRQ_LATENCY_MID};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

fn regions() -> Option<(Vec<u8>, Vec<u8>)> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    let region =
        |name| sf_video::romset::load_region(ROMS, MANIFEST, name).expect("valid local sound ROM");
    Some((region("audio2"), region("audiocpu")))
}

fn score(rom: &[u8], command: u8, prelude: &str, loop_body: &str) -> Arc<NativeScore> {
    let text = format!("command {command}\nunit 10\nchannel 0\nvoice 24 0 0 native\n{prelude}\nforever {{ {loop_body} }}");
    compile(&text, rom).unwrap().score
}

fn render(mut sound: GameSound, frames: u32) -> Vec<[f32; 2]> {
    let end = (frames - 2) as f64 / FRAME_HZ as f64;
    assert!(sound.wait_ready(end), "native audio reaches the end");
    let samples = (0..(end * 8_000.0) as usize)
        .map(|n| sound.sample_at_gains(n as f64 / 8_000.0, 1.0, 0.0))
        .collect();
    assert!(
        sound.unrecovered().is_empty(),
        "native audio faults: {:?}",
        sound.unrecovered()
    );
    samples
}

type Cue = (u32, u8, Option<Arc<NativeScore>>);

fn play(audio2: &[u8], rom: &[u8], frames: u32, cues: &[Cue]) -> Vec<[f32; 2]> {
    let mut sound =
        GameSound::power_on(audio2.to_vec(), rom.to_vec(), LatchTimeline::default(), 0.0);
    for frame in 0..frames {
        let (_, command, score) = cues
            .iter()
            .find(|(at, _, _)| *at == frame)
            .cloned()
            .unwrap_or((frame, 0xff, None));
        sound.push_frame_with_score(frame, command, 0, IRQ_LATENCY_MID, score);
    }
    render(sound, frames)
}

fn known(audio2: &[u8], rom: Vec<u8>, frames: u32, cues: &[Cue]) -> Vec<[f32; 2]> {
    let mut timeline = LatchTimeline::default();
    for frame in 0..frames {
        let command = cues
            .iter()
            .find(|(at, _, _)| *at == frame)
            .map_or(0xff, |(_, cmd, _)| *cmd);
        timeline.push_frame(frame, command, 0, IRQ_LATENCY_MID);
    }
    render(
        GameSound::power_on(audio2.to_vec(), rom, timeline, f64::INFINITY),
        frames,
    )
}

#[test]
fn a_wide_backward_jump_plays_the_same_native_voice_as_a_small_score() {
    let Some((audio2, rom)) = regions() else {
        return;
    };
    // The long prefix writes voice parameters only. Its clock cost is real.
    // A short control-score comparison below checks the steady pitch once
    // the wide jump has reached the loop body.
    let prelude = "voice 24 0 0 native ".repeat(11_000);
    let wide = score(&rom, 0x41, &prelude, "raw 62 0 380 160");
    let small = score(&rom, 0x41, "", "raw 62 0 380 160");
    let other = score(&rom, 0x41, "", "raw 73 0 380 160");
    assert!(wide.bytes() > 0x10000);
    let wide = play(&audio2, &rom, 290, &[(30, 0x41, Some(wide))]);
    let small = play(&audio2, &rom, 290, &[(30, 0x41, Some(small))]);
    let other = play(&audio2, &rom, 290, &[(30, 0x41, Some(other))]);
    let pitch_period = |samples: &[[f32; 2]]| {
        let samples = &samples[28_000..36_000];
        (10..80)
            .max_by(|&a, &b| {
                let energy = |lag: usize| -> f32 {
                    samples[lag..]
                        .iter()
                        .zip(samples)
                        .map(|(x, y)| x[0] * y[0])
                        .sum()
                };
                energy(a).total_cmp(&energy(b))
            })
            .unwrap()
    };
    assert!(
        wide[28_000..36_000].iter().any(|s| s[0].abs() > 0.005),
        "the wide loop is audible"
    );
    assert!(small[28_000..36_000].iter().any(|s| s[0].abs() > 0.005));
    assert_eq!(pitch_period(&wide), pitch_period(&small));
    assert_ne!(pitch_period(&wide), pitch_period(&other));
}

#[test]
fn a_zero_time_loop_is_rejected_with_a_source_diagnostic() {
    let mut rom = vec![0; 0x8000];
    rom[0x6b00 + 0x29 * 24 + 6] = 0x3d;
    let text = "command 0x41\nunit 10\nchannel 0\nvoice 24 0 0 native\nforever { pitch 62 0 }";
    let error = compile(text, &rom).unwrap_err();
    assert_eq!(error.line, 5);
    assert!(error.to_string().contains("take no time"));
}

#[test]
fn mixed_native_priorities_and_effect_handback_restore_the_separate_source() {
    let Some((audio2, rom)) = regions() else {
        return;
    };
    let text = "command 0x41\nunit 10\nchannel 0\nvoice 24 2 7 3\nforever { raw 62 0 1500 1300 pitch 63 24 wait 800 off wait 100 }";
    let compiled = compile(text, &rom).unwrap().score;
    let mut legacy = ExtendedTunes::new();
    sf_audio::score::convert(text, &rom)
        .unwrap()
        .add_to(&mut legacy)
        .unwrap();
    let legacy = legacy.program(&rom).unwrap();
    let cues = [
        (30, 0x41, Some(compiled.clone())),
        (64, 0x09, None), // effect steals and hands back a channel
        (91, 0x37, None), // equal-priority native win jingle
        (160, 0x41, Some(compiled.clone())),
        (190, 0x36, None),           // higher-priority native staff roll
        (230, 0x41, Some(compiled)), // lower priority goes in the saved slot
        (2100, 0x3f, None),
        (2140, 0x29, None), // original music after discard and stop
    ];
    let separate = play(&audio2, &rom, 2290, &cues);
    let mapped = known(&audio2, legacy, 2290, &cues);
    // Before the staff roll ends, priority handling and effect handback
    // match the mapped driver sample for sample. The mapped driver loads
    // from ROM address zero when it pops a tune that has yet to run.
    assert!(separate[..30 * 8_000] == mapped[..30 * 8_000]);
    assert!(separate[32 * 8_000..33 * 8_000]
        .iter()
        .any(|s| s[0].abs() > 0.005));
    assert!(separate[35 * 8_000 + 2_000..35 * 8_000 + 5_000]
        .iter()
        .all(|s| s[0].abs() < 0.0001));
    assert!(separate[36 * 8_000..37 * 8_000]
        .iter()
        .any(|s| s[0].abs() > 0.005));
    let run = capture(&rom, 38, &cues);
    assert!(run.observations.tune_equal_priority > 0);
    assert!(run.observations.tune_higher_priority_saves > 0);
    assert!(run.observations.tune_lower_priority_saved_loads > 0);
    assert!(run.observations.saved_pops > 0);
    assert!(run.observations.saved_discards > 0);
}

#[test]
fn explicit_replacement_uses_its_own_score_even_without_an_idle_latch() {
    let Some((audio2, rom)) = regions() else {
        return;
    };
    let old = score(&rom, 0x41, "", "raw 62 0 1100 1000");
    let new = score(&rom, 0x41, "", "raw 73 0 1100 1000");
    let changed = play(
        &audio2,
        &rom,
        132,
        &[(30, 0x41, Some(old)), (31, 0x41, Some(new.clone()))],
    );
    let only_new = play(&audio2, &rom, 132, &[(31, 0x41, Some(new))]);
    let only_old = play(
        &audio2,
        &rom,
        132,
        &[(30, 0x41, Some(score(&rom, 0x41, "", "raw 62 0 1100 1000")))],
    );
    // Compare a steady pitch, independent of phase left by the first cue.
    let dominant = |mix: &[[f32; 2]]| {
        let samples = &mix[10_000..14_000];
        (10..80)
            .max_by(|&a, &b| {
                let correlation = |lag: usize| {
                    samples[lag..]
                        .iter()
                        .zip(samples)
                        .map(|(x, y)| x[0] * y[0])
                        .sum::<f32>()
                };
                correlation(a).total_cmp(&correlation(b))
            })
            .unwrap()
    };
    assert_eq!(dominant(&changed), dominant(&only_new));
    assert_ne!(dominant(&changed), dominant(&only_old));
}

#[test]
fn eight_voice_raw_loops_match_the_original_elapsed_clock_timing() {
    let Some((audio2, rom)) = regions() else {
        return;
    };
    let mut text = String::from("command 0x44\nunit 10\n");
    for ch in 0..8 {
        text.push_str(&format!("channel {ch}\nvoice 24 1 3 native\nforever {{ raw {} 0 250 180 pitch {} 20 wait 150 off wait 200 }}\n", 55+ch, 57+ch));
    }
    let compiled = compile(&text, &rom).unwrap().score;
    let mut legacy = ExtendedTunes::new();
    sf_audio::score::convert(&text, &rom)
        .unwrap()
        .add_to(&mut legacy)
        .unwrap();
    let legacy = legacy.program(&rom).unwrap();
    let cues = [(30, 0x44, Some(compiled))];
    assert!(play(&audio2, &rom, 370, &cues) == known(&audio2, legacy, 370, &cues));
}

fn capture(rom: &[u8], seconds: u64, cues: &[Cue]) -> MusicRun {
    let writes = (0..seconds as u32 * FRAME_HZ)
        .map(|frame| {
            let (_, command, score) = cues
                .iter()
                .find(|(at, _, _)| *at == frame)
                .cloned()
                .unwrap_or((frame, 0xff, None));
            (
                sf_audio::latch_write_time(frame, 0, IRQ_LATENCY_MID),
                command,
                score,
            )
        })
        .collect();
    let run = run_scored_music_from_power_on(rom.to_vec(), seconds * u64::from(OPM_RATE), writes);
    assert!(
        run.unrecovered.is_empty(),
        "native audio faults: {:?}",
        run.unrecovered
    );
    run
}

fn attacks(run: &MusicRun, ch: u8) -> Vec<(f64, [u8; 4])> {
    let mut registers = [0; 256];
    let mut notes = Vec::new();
    for write in &run.writes {
        registers[usize::from(write.reg)] = write.data;
        if write.reg == 8 && write.data & 7 == ch && write.data & 0x78 != 0 {
            let ch = usize::from(ch);
            notes.push((
                write.sample as f64 / f64::from(OPM_RATE),
                [
                    registers[0x28 + ch],
                    registers[0x30 + ch],
                    registers[0x20 + ch],
                    registers[0x78 + ch],
                ],
            ));
        }
    }
    notes
}

#[test]
fn channel_seven_restores_pitch_and_remaining_gate_after_pause_or_silent_advance() {
    let Some((_, rom)) = regions() else { return };
    let lower = "command 0x41 unit 1 channel 7 voice 16 13 0 native
        forever { raw 0x4e 236 20000 13000 raw 0x52 128 20000 17000 }";
    let higher = "command 0x42 priority 15 unit 1 channel 0 voice 1 0 0 native
        raw 0x22 0 30000 1000 end";
    let lower = compile(lower, &rom).unwrap().score;
    let alone = capture(&rom, 7, &[(30, 0x41, Some(lower.clone()))]);
    let baseline = attacks(&alone, 7);
    for mode in ["pause", "advance"] {
        let higher = compile(&format!("saved {mode}\n{higher}"), &rom)
            .unwrap()
            .score;
        let run = capture(
            &rom,
            7,
            &[(30, 0x41, Some(lower.clone())), (60, 0x42, Some(higher))],
        );
        let notes = attacks(&run, 7);
        assert_eq!(notes[0], baseline[0]);
        assert!(!notes.iter().any(|(t, _)| (1.05..3.10).contains(t)));
        let (resumed_at, voice) = notes[1];
        assert!((3.1..3.4).contains(&resumed_at));
        let next = notes[2];
        if mode == "advance" {
            assert_eq!(voice, baseline[1].1);
            assert!((next.0 - baseline[2].0).abs() < 0.02);
        } else {
            assert_eq!(voice, baseline[0].1);
            let paused_for = resumed_at - 1.0;
            assert!((next.0 - baseline[1].0 - paused_for).abs() < 0.03);
        }
        let first_off = run
            .writes
            .iter()
            .find(|w| {
                w.sample as f64 / f64::from(OPM_RATE) > resumed_at && w.reg == 8 && w.data == 7
            })
            .unwrap()
            .sample as f64
            / f64::from(OPM_RATE);
        assert!((resumed_at..next.0).contains(&first_off));
    }
}

#[test]
fn local_ryu_score_remains_sample_identical_to_legacy_playback() {
    let Some((audio2, rom)) = regions() else {
        return;
    };
    let path = Path::new(ROMS)
        .with_file_name("sf1-port-music")
        .join("ryu.score");
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("skipping: local Ryu score not present");
            return;
        }
        Err(error) => panic!("{}: {error}", path.display()),
    };
    let native = compile(&text, &rom).unwrap();
    assert_eq!(native.command, 0x41);
    let mut legacy = ExtendedTunes::new();
    sf_audio::score::convert(&text, &rom)
        .unwrap()
        .add_to(&mut legacy)
        .unwrap();
    let legacy = legacy.program(&rom).unwrap();
    let cues = [(30, native.command, Some(native.score))];
    let rendered = play(&audio2, &rom, 1_500, &cues);
    assert!(rendered.iter().any(|s| s[0].abs() > 0.05));
    let expected = known(&audio2, legacy, 1_500, &cues);
    let changed = rendered.iter().zip(expected).position(|(a, b)| *a != b);
    assert!(
        changed.is_none(),
        "Ryu's mix first differed at {:?} seconds",
        changed.map(|n| n as f64 / 8_000.0)
    );
}

#[test]
fn a_stop_supersedes_overdue_scores_without_dropping_the_next_request() {
    let Some((_, rom)) = regions() else { return };
    let busy = score(
        &rom,
        0x41,
        &"voice 24 0 0 native ".repeat(11_000),
        "raw 62 0 380 160",
    );
    let cancelled = score(&rom, 0x42, "", "raw 68 0 800 600");
    let next = score(&rom, 0x43, "", "raw 73 0 800 600");
    let run = capture(
        &rom,
        7,
        &[
            (30, 0x41, Some(busy)),
            (40, 0x42, Some(cancelled)),
            (42, 0x3f, None),
            (252, 0x43, Some(next)),
        ],
    );
    let notes = attacks(&run, 0);
    assert!(!notes.iter().any(|(_, voice)| voice[0] == 68));
    assert!(notes.iter().any(|(at, voice)| *at > 4.0 && voice[0] == 73));
}

#[test]
fn silently_advancing_notated_score_keeps_its_own_patch_pan_and_gate() {
    let Some((_, rom)) = regions() else { return };
    let lower = "command 0x41 unit 25 channel 0 patch 24 pan both gate 128
        forever { C4:4 patch 16 pan left D4:4 patch 24 pan both C4:4 }";
    let lower = compile(lower, &rom).unwrap().score;
    let higher = "command 0x42 saved advance priority 15 unit 1
        channel 0 voice 24 0 0 native raw 72 0 30000 0 end";
    let higher = compile(higher, &rom).unwrap().score;
    let baseline = capture(&rom, 7, &[(30, 0x41, Some(lower.clone()))]);
    let original = attacks(&baseline, 0);
    let run = capture(
        &rom,
        7,
        &[(30, 0x41, Some(lower)), (45, 0x42, Some(higher))],
    );
    let notes = attacks(&run, 0);
    assert_eq!(notes[0], original[0]);
    assert_eq!(notes[1].1[0], 72);
    assert!(
        notes.len() > 2 && original.len() > 2,
        "separate {notes:?} baseline {original:?}"
    );
    assert_eq!(
        notes[2].1, original[2].1,
        "the expired saved D4 gate stays off: {notes:?} vs {original:?}"
    );
    assert!((notes[2].0 - original[2].0).abs() < 0.08);
    assert!(
        !run.writes.iter().any(|w| {
            let time = w.sample as f64 / f64::from(OPM_RATE);
            (0.85..2.7).contains(&time) && w.reg == 0x20 && w.data & 0xc0 == 0x40
        }),
        "a saved pan change must not alter the currently audible voice"
    );
}

#[test]
fn an_overdue_music_stop_also_honors_an_earlier_stop_of_both_effects() {
    let Some((_, rom)) = regions() else { return };
    let busy = score(
        &rom,
        0x41,
        &"voice 24 0 0 native ".repeat(11_000),
        "raw 62 0 380 160",
    );
    let calls = |first_stop| {
        capture(
            &rom,
            7,
            &[
                (28, 0x09, None),
                (30, 0x41, Some(busy.clone())),
                (40, first_stop, None),
                (42, 0x3f, None),
            ],
        )
    };
    let music_only = calls(0x3f);
    let stop_all = calls(0x00);
    // The two effect stops each restore the cached LFO register sequence.
    // Distinguish them from any patch's single incidental waveform write.
    let effect_stops = |run: &MusicRun| {
        run.writes
            .windows(4)
            .filter(|window| {
                let time = window[0].sample as f64 / f64::from(OPM_RATE);
                time > 0.75
                    && window
                        .iter()
                        .map(|write| write.reg)
                        .eq([0x1b, 0x18, 0x19, 0x19])
            })
            .count()
    };
    assert!(effect_stops(&stop_all) > effect_stops(&music_only));
}
