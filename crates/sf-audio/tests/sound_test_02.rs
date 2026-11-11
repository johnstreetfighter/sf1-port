//! Acceptance tests for ticket #63 (F23, label SM): every tune through its
//! whole-tune loop or end, including one complete pass after tune 0x28's loop.
//! Local-only reference captures and ROMs; skips on a clean checkout.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;

use sf_audio::check::{
    exact_clock_matches, latch_timeline, music_ram_at_frame_ends, read_device_stream, read_taps,
    reference_opm_writes, run_music_from_power_on,
};
use sf_audio::opm::{OPM_CLOCK, OPM_RATE};

const A: &str = "../../tools/reference/local/captures/sound-test-02-audio-a";
const B: &str = "../../tools/reference/local/captures/sound-test-02-audio-b";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const LAST_FRAME: u32 = 39774;

const COMMANDS: &[(u8, u32)] = &[
    (0x28, 507),
    (0x3f, 6042),
    (0x29, 6302),
    (0x3f, 8462),
    (0x2a, 8712),
    (0x3f, 10179),
    (0x2b, 10419),
    (0x3f, 11601),
    (0x2c, 11831),
    (0x3f, 13002),
    (0x2d, 13222),
    (0x3f, 14190),
    (0x2e, 14400),
    (0x3f, 16161),
    (0x2f, 16361),
    (0x3f, 17809),
    (0x30, 17999),
    (0x3f, 19830),
    (0x31, 20010),
    (0x3f, 21172),
    (0x32, 21342),
    (0x3f, 23760),
    (0x33, 23920),
    (0x3f, 24496),
    (0x34, 24646),
    (0x3f, 27057),
    (0x36, 27187),
    (0x3f, 29068),
    (0x37, 29188),
    (0x3f, 29799),
    (0x38, 29909),
    (0x3f, 35871),
    (0x39, 35971),
    (0x3f, 36266),
    (0x3a, 36356),
    (0x3f, 37612),
    (0x3b, 37692),
    (0x3f, 38247),
    (0x3c, 38317),
    (0x3f, 39152),
    (0x3e, 39202),
    (0x3f, 39562),
];

// (tune, reference loop frame, latest channel, recurring pointer after its jump).
const LOOPS: &[(u8, u32, usize, u16)] = &[
    (0x28, 3113, 0, 0x10af),
    (0x28, 5692, 0, 0x10af),
    (0x29, 8123, 0, 0x12a0),
    (0x2a, 9849, 0, 0x14c2),
    (0x2b, 11281, 1, 0x165e),
    (0x2c, 12692, 1, 0x174f),
    (0x2d, 13891, 3, 0x195b),
    (0x2e, 15871, 1, 0x1ab1),
    (0x2f, 17529, 0, 0x1bb4),
    (0x30, 19553, 5, 0x1f09),
    (0x31, 20912, 0, 0x1f4b),
    (0x34, 26828, 4, 0x2547),
    (0x3a, 37442, 0, 0x2d17),
    (0x3b, 38087, 0, 0x2f1e),
];
const ENDS: &[(u8, u32)] = &[
    (0x32, 23510),
    (0x33, 24257),
    (0x36, 28858),
    (0x37, 29599),
    (0x38, 35682),
    (0x39, 36086),
    (0x3c, 39002),
    (0x3e, 39432),
];

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

fn pointer(state: &[u8], channel: usize) -> u16 {
    let offset = 0x100 + 0x30 * channel;
    u16::from_be_bytes([state[offset], state[offset + 1]])
}

#[test]
fn reference_runs_repeat_and_cover_every_tune_alone() {
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
    assert_eq!(commands, COMMANDS, "every tune and isolating stop");
    let tunes: BTreeSet<u8> = commands
        .iter()
        .map(|&(command, _)| command)
        .filter(|&command| command != 0x3f)
        .collect();
    assert_eq!(tunes.len(), 21);
    assert_eq!(
        LOOPS
            .iter()
            .map(|entry| entry.0)
            .collect::<BTreeSet<_>>()
            .len(),
        13
    );
    assert_eq!(ENDS.len(), 8);

    let frames: BTreeSet<u32> = checkpoints().into_iter().collect();
    for &(_, latch) in COMMANDS {
        assert!(frames.contains(&(latch - 1)), "checkpoint before {latch}");
    }
    for &(_, frame, _, _) in LOOPS {
        for required in [frame - 1, frame, frame + 1] {
            assert!(frames.contains(&required), "loop checkpoint {required}");
        }
    }
    for &(_, frame) in ENDS {
        for required in [frame - 1, frame, frame + 1] {
            assert!(frames.contains(&required), "end checkpoint {required}");
        }
    }
}

#[test]
fn reference_ram_records_every_loop_and_natural_end() {
    if !present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    for &(command, frame, channel, expected) in LOOPS {
        let before = ram(frame - 1);
        let at = ram(frame);
        assert_ne!(
            pointer(&before, channel),
            expected,
            "{command:02x} before loop {frame}"
        );
        assert_eq!(
            pointer(&at, channel),
            expected,
            "{command:02x} loop {frame}"
        );
    }
    for &(command, frame) in ENDS {
        let before = ram(frame - 1);
        let at = ram(frame);
        assert_ne!(before[0x11], 0, "{command:02x} before end {frame}");
        assert_eq!(at[0x11], 0, "{command:02x} end {frame}");
        for channel in 0..8 {
            assert_eq!(
                pointer(&at, channel),
                0,
                "{command:02x} channel {channel} at end"
            );
        }
    }
    assert_eq!(LOOPS.iter().filter(|entry| entry.0 == 0x28).count(), 2);
    assert_eq!(
        5692 - 3113,
        2713 - 134,
        "one complete 0x28 pass after its loop"
    );
}

#[test]
fn every_tune_stream_scan_reports_no_opcode_0a() {
    if !present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_region(ROMS, MANIFEST, "audiocpu").unwrap();
    let word = |address: usize| u16::from_le_bytes([rom[address], rom[address + 1]]) as usize;
    for &(command, _) in COMMANDS.iter().filter(|&&(command, _)| command != 0x3f) {
        let header = word(0x1011 + command as usize * 2);
        let mut queue = VecDeque::new();
        for channel in 0..8 {
            let start = word(header + 1 + 2 * channel);
            if start != 0 {
                queue.push_back(start);
            }
        }
        let mut seen = BTreeSet::new();
        let mut opcode_0a = Vec::new();
        let mut ends = Vec::new();
        let mut backward = false;
        while let Some(pc) = queue.pop_front() {
            if !seen.insert(pc) {
                continue;
            }
            assert!(seen.len() < 10000, "stream scan runaway for {command:02x}");
            let op = rom[pc];
            if op < 0x10 {
                match op {
                    0x0a => opcode_0a.push(pc),
                    0x0f => ends.push(pc),
                    0x0c..=0x0e => {
                        let target = word(pc + 2);
                        backward |= target <= pc;
                        queue.push_back(pc + 4);
                        queue.push_back(target);
                    }
                    _ => queue.push_back(pc + 2),
                }
            } else {
                queue.push_back(pc + 1);
            }
        }
        assert!(opcode_0a.is_empty(), "tune {command:02x} reaches opcode 0a");
        if ENDS.iter().any(|entry| entry.0 == command) {
            assert!(
                !ends.is_empty(),
                "ending tune {command:02x} has no opcode 0f"
            );
        } else {
            assert!(backward, "looping tune {command:02x} has no backward jump");
        }
        println!(
            "stream {command:02x}: {} events, opcode 0a=no, opcode 0f={}, backward jump={backward}",
            seen.len(),
            !ends.is_empty()
        );
    }
}

#[test]
fn timer_a_rearm_during_tune_38_matches_the_reference_clocks() {
    if !present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let log = read_taps(&Path::new(A).join("audio_taps.tsv")).unwrap();
    let rom = sf_video::romset::load_region(ROMS, MANIFEST, "audiocpu").unwrap();
    let first_sample = (33547.0 / 60.0 * OPM_RATE as f64).floor() as u64;
    let s_end = (33549.0 / 60.0 * OPM_RATE as f64).ceil() as u64;
    let run = run_music_from_power_on(rom, latch_timeline(&log, None), s_end);
    let reference: Vec<_> = reference_opm_writes(&log)
        .into_iter()
        .filter(|w| (first_sample..s_end).contains(&w.sample))
        .collect();
    let native: Vec<_> = run
        .writes
        .iter()
        .copied()
        .filter(|w| (first_sample..s_end).contains(&w.sample))
        .collect();
    assert_eq!(native.len(), reference.len());
    let errors: Vec<String> = native
        .iter()
        .zip(&reference)
        .enumerate()
        .filter(|(_, (native, reference))| {
            native.clock != reference.clock
                || native.reg != reference.reg
                || native.data != reference.data
        })
        .map(|(i, (native, reference))| {
            format!(
                "#{i} {:02x}/{:02x}: native clock {} reference {}",
                reference.reg, reference.data, native.clock, reference.clock
            )
        })
        .collect();
    assert!(
        errors.is_empty(),
        "Timer A rearm at frame 33548 differs:\n{}",
        errors.join("\n")
    );
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
        "untranslated paths reached (0x0000 is opcode 0a; 0x09e7 is a nested end): {:04x?}",
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
    let clock_errors: Vec<String> = native
        .iter()
        .zip(&reference)
        .enumerate()
        .filter(|(_, (native, reference))| native.clock != reference.clock)
        .take(12)
        .map(|(i, (native, reference))| {
            format!(
                "#{i} {:02x}/{:02x} at frame {}: native clock {} reference {} (samples {} / {})",
                reference.reg,
                reference.data,
                reference.sample * 60 / OPM_RATE as u64,
                native.clock,
                reference.clock,
                native.sample,
                reference.sample
            )
        })
        .collect();
    assert_eq!(
        exact,
        native.len(),
        "every write on the exact chip clock:\n{}",
        clock_errors.join("\n")
    );

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
        compared > 36_000_000,
        "the full hook window is compared ({compared} samples)"
    );
    println!(
        "all tunes: {} writes, all on the exact clock; {compared} output samples identical",
        native.len()
    );
    println!("opcode 0a / nested end 0x09e7: no untranslated path reached for any tune");
}

#[test]
fn music_ram_matches_at_idle_checkpoints_and_reports_end_gaps() {
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
    assert!(idle > 0, "acceptance requires at least one idle checkpoint");

    let mut not_compared = Vec::new();
    for &(command, end) in ENDS {
        let next = COMMANDS
            .iter()
            .find(|&&(_, latch)| latch > end)
            .map(|&(_, latch)| latch)
            .unwrap();
        let has_idle = idle_by_frame
            .iter()
            .any(|(&frame, &is_idle)| frame > end && frame < next && is_idle);
        if !has_idle {
            not_compared.push(format!("{command:02x}@{end}"));
        }
    }
    println!("music RAM: {idle} idle checkpoints identical; {not_idle} not idle");
    println!(
        "ends without an idle checkpoint before the next command (F72): {}",
        if not_compared.is_empty() {
            "none".to_owned()
        } else {
            not_compared.join(", ")
        }
    );
}

#[test]
fn moving_the_round_tune_by_one_sequencer_tick_breaks_its_loop_comparison() {
    if !present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let log = read_taps(&Path::new(A).join("audio_taps.tsv")).unwrap();
    let rom = sf_video::romset::load_region(ROMS, MANIFEST, "audiocpu").unwrap();
    let s_end = (3115.0 / 60.0 * OPM_RATE as f64).ceil() as u64;
    let reference: Vec<_> = reference_opm_writes(&log)
        .into_iter()
        .filter(|w| w.sample < s_end)
        .collect();
    let mut events = latch_timeline(&log, None).into_events();
    let mut moved = 0;
    for (t, command) in &mut events {
        if *command == 0x28 && (*t * 60.0).floor() as u32 == 507 {
            *t += 168.0 * 64.0 / OPM_CLOCK as f64;
            moved += 1;
        }
    }
    assert_eq!(moved, 1, "the control moves only the initial 28@507 latch");
    let shifted = run_music_from_power_on(rom, sf_audio::LatchTimeline::new(events), s_end);
    assert!(shifted.unrecovered.is_empty());
    let exact = exact_clock_matches(&shifted.writes, &reference);
    assert!(
        exact < reference.len(),
        "C-NEG: moving 28@507 by one Timer A sequencer tick must be caught"
    );
    let first_off = shifted
        .writes
        .iter()
        .zip(&reference)
        .position(|(native, reference)| {
            native.clock != reference.clock
                || native.reg != reference.reg
                || native.data != reference.data
        })
        .expect("a differing write before the first loop");
    let frame = reference[first_off].sample * 60 / OPM_RATE as u64;
    assert!(
        frame <= 3113,
        "C-NEG mismatch must be caught by the first loop, got {frame}"
    );
    println!(
        "C-NEG: 28@507 +1 Timer A tick leaves {exact}/{} exact writes; first mismatch at frame {frame}",
        reference.len()
    );
}
