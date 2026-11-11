//! Acceptance tests for ticket #78 (F38, label SM): tune transitions across
//! priorities (higher, equal, lower), saving to 0xc300 (0x01e5) and loading
//! into 0xc300 (0x01d3), popping the saved tune (0x03a7), discarding the saved
//! tune (0x0412), music stops 0x35, 0x3d, and 0x3f over a saved tune, and
//! repeated commands ignored.
//! Local-only reference captures and ROMs; skips on a clean checkout.

#![allow(clippy::type_complexity)]

use std::collections::BTreeMap;
use std::path::Path;

use sf_audio::check::{
    exact_clock_matches, latch_timeline, music_ram_at_frame_ends,
    music_ram_at_frame_ends_with_fault, read_device_stream, read_taps, reference_opm_writes,
    run_music_from_power_on, run_music_from_power_on_with_fault,
};
use sf_audio::music::MusicFault;
use sf_audio::opm::OPM_RATE;

const A: &str = "../../tools/reference/local/captures/sound-test-03-audio-a";
const B: &str = "../../tools/reference/local/captures/sound-test-03-audio-b";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const LAST_FRAME: u32 = 6249;

const COMMANDS: &[(u8, u32)] = &[
    (0x28, 507),
    (0x29, 597),
    (0x29, 677),
    (0x36, 887),
    (0x32, 2727),
    (0x28, 2907),
    (0x36, 5247),
    (0x35, 5337),
    (0x36, 5427),
    (0x33, 5537),
    (0x3d, 5717),
    (0x3a, 5827),
    (0x36, 5947),
    (0x3f, 6117),
];

/// Natural opcode 0x0f ends of priority-15 tunes 0x36 and 0x32 that pop the
/// saved priority-8 tune from 0xc300 back into 0xc100 via 0x03a7.
const POP_ENDS: &[(u8, u8, u32)] = &[(0x36, 0x29, 2558), (0x32, 0x28, 4895)];

fn present() -> bool {
    [A, B, ROMS].iter().all(|p| Path::new(p).is_dir())
}

fn meta(capture: &str, key: &str) -> String {
    std::fs::read_to_string(Path::new(capture).join("meta.tsv"))
        .unwrap()
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}\t")).map(str::to_owned))
        .unwrap()
}

fn checkpoints() -> Vec<u32> {
    std::fs::read_to_string(Path::new(A).join("checkpoints.tsv"))
        .unwrap()
        .lines()
        .skip(1)
        .filter_map(|line| line.split('\t').next()?.parse().ok())
        .collect()
}

fn ram(frame: u32) -> Vec<u8> {
    std::fs::read(Path::new(A).join(format!("ram_{frame:06}_music.bin"))).unwrap()
}

fn pointer(state: &[u8], base: usize, channel: usize) -> u16 {
    let offset = base + 0x30 * channel;
    u16::from_be_bytes([state[offset], state[offset + 1]])
}

#[test]
fn reference_runs_repeat_and_cover_every_tune_transition_and_stop() {
    if !present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    for file in [
        "audio_taps.tsv",
        "hook_index.tsv",
        "hook_ymsnd.f32",
        "audio.wav",
        "checkpoints.tsv",
    ] {
        let a = std::fs::read(Path::new(A).join(file)).unwrap();
        let b = std::fs::read(Path::new(B).join(file)).unwrap();
        assert!(a == b, "{file} differs between reference runs");
    }
    for capture in [A, B] {
        assert_eq!(meta(capture, "dsw1"), "5fff");
        assert_eq!(meta(capture, "dsw2"), "ffff");
    }

    let log = read_taps(&Path::new(A).join("audio_taps.tsv")).unwrap();
    let commands: Vec<(u8, u32)> = log
        .latch
        .iter()
        .filter(|&&(t, _)| t > 20.0 / 60.0)
        .map(|&(t, v)| (v, (t * 60.0).floor() as u32))
        .filter(|&(v, _)| v < 0x40)
        .collect();
    assert_eq!(
        commands, COMMANDS,
        "all planned tune transition and stop commands"
    );

    // Verify each required transition in the reference RAM dumps:
    // (frame, expected (c001, c011, c012, cur_ch0, sav_ch0) before at frame-1 and after at frame).
    let expected_transitions: &[(u32, (u8, u8, u8, u16, u16), (u8, u8, u8, u16, u16), &str)] = &[
        (
            507,
            (0xff, 0x00, 0x00, 0x0000, 0x0000),
            (0x28, 0x08, 0x00, 0x10ad, 0x0000),
            "start 0x28 (priority 8) from silence",
        ),
        (
            597,
            (0xff, 0x08, 0x00, 0x10b0, 0x0000),
            (0x29, 0x08, 0x00, 0x1283, 0x0000),
            "equal priority: 0x29 (priority 8) replaces 0x28 without saving to 0xc300",
        ),
        (
            677,
            (0xff, 0x08, 0x00, 0x1289, 0x0000),
            (0x29, 0x08, 0x00, 0x1283, 0x0000),
            "repeat command 0x29 at equal priority",
        ),
        (
            887,
            (0xff, 0x08, 0x00, 0x129b, 0x0000),
            (0x36, 0x0f, 0x08, 0x2675, 0x129b),
            "higher priority: 0x36 (priority 15) saves live 0x29 to 0xc300 (0x01e5)",
        ),
        (
            2558,
            (0xff, 0x0f, 0x08, 0x26dd, 0x129b),
            (0xff, 0x08, 0x00, 0x129b, 0x0000),
            "opcode 0x0f end of 0x36 pops saved 0x29 from 0xc300 (0x03a7) and discards 0xc300 (0x0412)",
        ),
        (
            2727,
            (0xff, 0x08, 0x00, 0x12a0, 0x0000),
            (0x32, 0x0f, 0x08, 0x2092, 0x12a0),
            "higher priority: 0x32 (priority 15) saves resumed 0x29 to 0xc300 (0x01e5)",
        ),
        (
            2907,
            (0xff, 0x0f, 0x08, 0x2093, 0x12a0),
            (0x28, 0x0f, 0x08, 0x2093, 0x10a2),
            "lower priority: 0x28 (priority 8) replaces occupied saved slot 0xc300 while 0x32 plays (0x01d3)",
        ),
        (
            4895,
            (0xff, 0x0f, 0x08, 0x20d8, 0x10a2),
            (0xff, 0x08, 0x00, 0x10a2, 0x0000),
            "opcode 0x0f end of 0x32 pops freshly-saved 0x28 from 0xc300 (0x03a7) and discards 0xc300 (0x0412)",
        ),
        (
            5247,
            (0xff, 0x08, 0x00, 0x10bd, 0x0000),
            (0x36, 0x0f, 0x08, 0x2675, 0x10bd),
            "higher priority: 0x36 saves live 0x28 to 0xc300 before 0x35 stop",
        ),
        (
            5337,
            (0xff, 0x0f, 0x08, 0x2676, 0x10bd),
            (0x35, 0x00, 0x00, 0x0000, 0x0000),
            "stop 0x35 over saved tune 0x28 discards 0xc300 (0x0412) and stops music (0x03a7)",
        ),
        (
            5427,
            (0xff, 0x00, 0x00, 0x0000, 0x0000),
            (0x36, 0x0f, 0x00, 0x2675, 0x0000),
            "start 0x36 (priority 15) from silence with empty 0xc300",
        ),
        (
            5537,
            (0xff, 0x0f, 0x00, 0x2676, 0x0000),
            (0x33, 0x0f, 0x08, 0x2676, 0x2338),
            "lower priority: 0x33 (priority 8) loads into empty saved slot 0xc300 while 0x36 plays (0x01d3)",
        ),
        (
            5717,
            (0xff, 0x0f, 0x08, 0x2678, 0x2338),
            (0x3d, 0x00, 0x00, 0x0000, 0x0000),
            "stop 0x3d over saved tune 0x33 discards 0xc300 (0x0412) and stops music (0x03a7)",
        ),
        (
            5827,
            (0xff, 0x00, 0x00, 0x0000, 0x0000),
            (0x3a, 0x08, 0x00, 0x2cfd, 0x0000),
            "start 0x3a (priority 8) from silence",
        ),
        (
            5947,
            (0xff, 0x08, 0x00, 0x2d07, 0x0000),
            (0x36, 0x0f, 0x08, 0x2675, 0x2d07),
            "higher priority: 0x36 saves live 0x3a to 0xc300 before 0x3f stop",
        ),
        (
            6117,
            (0xff, 0x0f, 0x08, 0x2677, 0x2d07),
            (0x3f, 0x00, 0x00, 0x0000, 0x0000),
            "stop 0x3f over saved tune 0x3a discards 0xc300 (0x0412) and stops music (0x03a7)",
        ),
    ];

    for &(frame, before_exp, after_exp, desc) in expected_transitions {
        let before = ram(frame - 1);
        let after = ram(frame);
        let before_got = (
            before[0x001],
            before[0x011],
            before[0x012],
            pointer(&before, 0x100, 0),
            pointer(&before, 0x300, 0),
        );
        let after_got = (
            after[0x001],
            after[0x011],
            after[0x012],
            pointer(&after, 0x100, 0),
            pointer(&after, 0x300, 0),
        );
        assert_eq!(
            before_got,
            before_exp,
            "before {desc} (frame {})",
            frame - 1
        );
        assert_eq!(after_got, after_exp, "after {desc} (frame {frame})");
    }
}

#[test]
fn music_from_power_on_matches_every_write_clock_and_hooked_sample() {
    if !present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let log = read_taps(&Path::new(A).join("audio_taps.tsv")).unwrap();
    let rom = sf_video::romset::load_region(ROMS, MANIFEST, "audiocpu").unwrap();
    let s_end = ((LAST_FRAME + 1) as f64 / 60.0 * OPM_RATE as f64).ceil() as u64;
    let run = run_music_from_power_on(rom, latch_timeline(&log, None), s_end);
    assert!(
        run.unrecovered.is_empty(),
        "untranslated paths reached: {:04x?}",
        run.unrecovered
    );

    let reference: Vec<_> = reference_opm_writes(&log)
        .into_iter()
        .filter(|w| w.sample < s_end)
        .collect();
    let native: Vec<_> = run
        .writes
        .iter()
        .copied()
        .filter(|w| w.sample < s_end)
        .collect();
    assert_eq!(native.len(), reference.len(), "every YM2151 write");
    for (i, (r, n)) in reference.iter().zip(&native).enumerate() {
        assert_eq!((r.reg, r.data), (n.reg, n.data), "write #{i} in order");
        assert!(
            r.sample.abs_diff(n.sample) <= 1,
            "write #{i} within one output sample"
        );
    }
    let exact = exact_clock_matches(&native, &reference);
    assert_eq!(exact, native.len(), "every write on the exact chip clock");

    let hook = read_device_stream(Path::new(A), "ymsnd", OPM_RATE as f64).unwrap();
    let n0 = hook.first_sample();
    let mut compared = 0u64;
    for (i, r) in hook.samples.iter().enumerate() {
        let n = n0 + i as u64;
        if n < s_end {
            assert_eq!(run.left[n as usize], *r, "YM2151 output sample {n}");
            compared += 1;
        }
    }
    assert!(
        compared > 5_000_000,
        "the full hook window is compared ({compared} samples)"
    );

    let obs = run.observations;
    assert!(
        obs.repeated_cmd_ignores >= 14,
        "every command latch window exercises 0x01a4 repeated-command ignore"
    );
    assert_eq!(
        obs.tune_equal_priority, 2,
        "equal-priority starts (29@597, 29@677)"
    );
    assert_eq!(
        obs.tune_higher_priority_saves, 7,
        "higher-priority starts saving 0xc100 to 0xc300 (0x01e5)"
    );
    assert_eq!(
        obs.tune_lower_priority_saved_loads, 2,
        "lower-priority starts loading into 0xc300 (28@2907, 33@5537)"
    );
    assert_eq!(
        obs.saved_pops, 6,
        "0x03a7 pops (boot 3f@18, 2 natural ends at 2558 and 4895, 3 stops 35/3d/3f)"
    );
    assert_eq!(
        obs.saved_discards, 10,
        "0x0412 discards (4 from 0x03a4 stops + 6 from 0x03a7 pops)"
    );
    println!(
        "sound-test-03: {} writes, all on the exact clock; {compared} output samples identical; observations={obs:?}",
        native.len()
    );
}

#[test]
fn music_ram_matches_at_every_idle_checkpoint_before_and_after_each_transition() {
    if !present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let log = read_taps(&Path::new(A).join("audio_taps.tsv")).unwrap();
    let rom = sf_video::romset::load_region(ROMS, MANIFEST, "audiocpu").unwrap();
    let frames = checkpoints();
    let (states, unrecovered) = music_ram_at_frame_ends(rom, latch_timeline(&log, None), &frames);
    assert!(
        unrecovered.is_empty(),
        "untranslated paths reached: {unrecovered:04x?}"
    );
    assert_eq!(states.len(), frames.len());

    let mut idle = 0usize;
    let mut not_idle = 0usize;
    let mut idle_by_frame = BTreeMap::new();
    for (frame, is_idle, native) in &states {
        idle_by_frame.insert(*frame, *is_idle);
        if !is_idle {
            not_idle += 1;
            continue;
        }
        idle += 1;
        let reference = ram(*frame);
        let differing: Vec<String> = (0..0x5d0)
            .filter(|&i| native[i] != reference[i])
            .map(|i| {
                format!(
                    "{:04x}: native {:02x} reference {:02x}",
                    0xc000 + i,
                    native[i],
                    reference[i]
                )
            })
            .collect();
        assert!(
            differing.is_empty(),
            "music RAM at idle checkpoint {frame}: {} bytes differ:\n{}",
            differing.len(),
            differing.join("\n")
        );
    }
    assert!(idle > 0, "acceptance requires idle checkpoints");

    // Every command and natural end transition must have at least one idle
    // checkpoint immediately before and at least one immediately after it.
    for &(_, latch) in COMMANDS {
        let has_idle_before = [latch - 2, latch - 1]
            .iter()
            .any(|f| idle_by_frame.get(f) == Some(&true));
        let has_idle_after = [latch, latch + 1, latch + 2, latch + 3]
            .iter()
            .any(|f| idle_by_frame.get(f) == Some(&true));
        assert!(
            has_idle_before,
            "no idle checkpoint before command at {latch}"
        );
        assert!(
            has_idle_after,
            "no idle checkpoint after command at {latch}"
        );
    }
    for &(_, _, end) in POP_ENDS {
        let has_idle_before = (end - 6..end).any(|f| idle_by_frame.get(&f) == Some(&true));
        let has_idle_after = (end..=end + 6).any(|f| idle_by_frame.get(&f) == Some(&true));
        assert!(
            has_idle_before,
            "no idle checkpoint before pop end at {end}"
        );
        assert!(has_idle_after, "no idle checkpoint after pop end at {end}");
    }
    println!("sound-test-03 RAM: {idle} idle checkpoints identical; {not_idle} not idle; 0 transition gaps");
}

#[test]
fn swapping_priority_comparison_or_dropping_saved_copy_breaks_write_sample_and_ram_checks() {
    if !present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let log = read_taps(&Path::new(A).join("audio_taps.tsv")).unwrap();
    let rom = sf_video::romset::load_region(ROMS, MANIFEST, "audiocpu").unwrap();
    let s_end = ((LAST_FRAME + 1) as f64 / 60.0 * OPM_RATE as f64).ceil() as u64;
    let reference: Vec<_> = reference_opm_writes(&log)
        .into_iter()
        .filter(|w| w.sample < s_end)
        .collect();
    let hook = read_device_stream(Path::new(A), "ymsnd", OPM_RATE as f64).unwrap();
    let n0 = hook.first_sample();
    let frames = checkpoints();

    for (label, fault) in [
        (
            "swap_priority_comparison",
            MusicFault {
                swap_priority_comparison: true,
                drop_saved_copy: false,
            },
        ),
        (
            "drop_saved_copy",
            MusicFault {
                swap_priority_comparison: false,
                drop_saved_copy: true,
            },
        ),
    ] {
        let run = run_music_from_power_on_with_fault(
            rom.clone(),
            latch_timeline(&log, None),
            s_end,
            fault,
        );
        let exact = exact_clock_matches(&run.writes, &reference);
        assert!(
            run.writes.len() != reference.len() || exact < reference.len(),
            "C-NEG ({label}): write check must fail"
        );

        let sample_mismatches = hook
            .samples
            .iter()
            .enumerate()
            .filter(|&(i, r)| {
                let n = n0 + i as u64;
                n < s_end && run.left[n as usize] != *r
            })
            .count();
        assert!(
            sample_mismatches > 0,
            "C-NEG ({label}): hooked sample check must fail"
        );

        let (states, _) = music_ram_at_frame_ends_with_fault(
            rom.clone(),
            latch_timeline(&log, None),
            &frames,
            fault,
        );
        let ram_mismatches = states
            .iter()
            .filter(|(_, is_idle, _)| *is_idle)
            .filter(|(frame, _, native)| {
                let reference = ram(*frame);
                (0..0x5d0).any(|i| native[i] != reference[i])
            })
            .count();
        assert!(
            ram_mismatches > 0,
            "C-NEG ({label}): idle-checkpoint RAM check must fail"
        );
        println!(
            "C-NEG ({label}): writes {}/{} exact (ref {}), {sample_mismatches} sample mismatches, {ram_mismatches} idle RAM checkpoint mismatches",
            exact,
            run.writes.len(),
            reference.len()
        );
    }
}
