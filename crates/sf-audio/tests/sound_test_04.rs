//! Acceptance tests for ticket #64 (F24, label SM): every ADPCM command to
//! its natural end, both channel-stop paths, and overlapping playback on both
//! MSM5205s. Local-only reference captures and ROMs; skips on a clean checkout.

use std::collections::BTreeSet;
use std::io::{BufRead, BufReader};
use std::path::Path;

use sf_audio::check::{
    compare_dac, compare_write_times, compare_writes, frame_tick, latch_timeline, read_hook,
    read_taps, RefLog,
};
use sf_audio::samples::PortWrite;

const A: &str = "../../tools/reference/local/captures/sound-test-04-audio-a";
const B: &str = "../../tools/reference/local/captures/sound-test-04-audio-b";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const HOOK_FIRST: u32 = 1376;
const LAST_FRAME: u32 = 12759;

const COMMANDS: &[(u8, u32)] = &[
    (0x81, 1377),
    (0x82, 1454),
    (0x83, 1531),
    (0x84, 1609),
    (0x85, 1687),
    (0x86, 1797),
    (0x87, 1868),
    (0x88, 2006),
    (0x89, 2130),
    (0x8a, 2232),
    (0x8b, 2342),
    (0x8c, 2459),
    (0x8d, 2563),
    (0x8e, 2701),
    (0x8f, 2811),
    (0x90, 2923),
    (0x91, 3059),
    (0x92, 3168),
    (0x93, 3258),
    (0x94, 3373),
    (0x95, 3473),
    (0x96, 3617),
    (0x97, 3708),
    (0x98, 3815),
    (0x99, 3935),
    (0x9a, 4054),
    (0x9b, 4175),
    (0x9c, 4295),
    (0x9d, 4464),
    (0x9e, 4927),
    (0x9f, 5236),
    (0xa0, 5410),
    (0xa1, 5635),
    (0xa2, 5711),
    (0xa3, 5810),
    (0xa4, 5897),
    (0xa5, 5988),
    (0xa6, 6118),
    (0xa7, 6218),
    (0xa8, 6366),
    (0xa9, 6491),
    (0xaa, 6716),
    (0xab, 6810),
    (0xac, 6935),
    (0xad, 7045),
    (0xae, 7244),
    (0xaf, 7331),
    (0xb0, 7461),
    (0xb1, 7568),
    (0xb2, 7775),
    (0xb3, 7892),
    (0xb4, 8094),
    (0xb5, 8190),
    (0xb6, 8389),
    (0xb7, 8483),
    (0xb8, 8677),
    (0xb9, 8787),
    (0xba, 8973),
    (0xbb, 9267),
    (0xbc, 9446),
    (0xbd, 9533),
    (0xbe, 9873),
    (0xbf, 10175),
    (0x9d, 10807),
    (0x80, 11217),
    (0x9d, 11567),
    (0xa0, 11647),
    (0x80, 12087),
    (0xa0, 12527),
    (0x27, 12592),
    (0x3f, 12593),
    (0x80, 12594),
];

fn present() -> bool {
    [A, B, ROMS].iter().all(|p| Path::new(p).is_dir())
}

fn audio2() -> Vec<u8> {
    sf_video::romset::load_region(ROMS, MANIFEST, "audio2").expect("audio2 region")
}

fn meta(capture: &str, key: &str) -> String {
    std::fs::read_to_string(Path::new(capture).join("meta.tsv"))
        .unwrap()
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}\t")).map(str::to_owned))
        .unwrap()
}

fn commands(log: &RefLog) -> Vec<(u8, u32, f64)> {
    log.latch
        .iter()
        .filter(|&&(t, v)| t > 20.0 / 60.0 && v != 0xff)
        .map(|&(t, v)| (v, (t * 60.0).floor() as u32, t))
        .collect()
}

fn checkpoints() -> BTreeSet<u32> {
    std::fs::read_to_string(Path::new(A).join("checkpoints.tsv"))
        .unwrap()
        .lines()
        .skip(1)
        .filter_map(|line| line.split('\t').next()?.parse().ok())
        .collect()
}

fn observation_files(capture: &str) -> BTreeSet<String> {
    std::fs::read_dir(capture)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_file())
        .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
        .filter(|name| {
            !matches!(
                name.as_str(),
                "generated_driver.lua" | "summary.json" | "hook_index.tsv"
            ) && !name.ends_with(".png")
        })
        .collect()
}

fn assert_same_file(a: &Path, b: &Path, name: &str) {
    let mut a = BufReader::with_capacity(1024 * 1024, std::fs::File::open(a).unwrap());
    let mut b = BufReader::with_capacity(1024 * 1024, std::fs::File::open(b).unwrap());
    loop {
        let (count, done) = {
            let x = a.fill_buf().unwrap();
            let y = b.fill_buf().unwrap();
            assert_eq!(x.is_empty(), y.is_empty(), "{name}: different lengths");
            if x.is_empty() {
                (0, true)
            } else {
                let count = x.len().min(y.len());
                assert!(x[..count] == y[..count], "{name}: content differs");
                (count, false)
            }
        };
        if done {
            break;
        }
        a.consume(count);
        b.consume(count);
    }
}

fn hook_span(device: &str) -> (f64, f64) {
    let index = std::fs::read_to_string(Path::new(A).join("hook_index.tsv")).unwrap();
    let mut start = f64::MAX;
    let mut end = 0.0f64;
    for line in index.lines().skip(1) {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields[2] != device {
            continue;
        }
        let t: f64 = fields[1].parse().unwrap();
        let count: f64 = fields[4].parse().unwrap();
        start = start.min(t - count / 384_000.0);
        end = end.max(t);
    }
    (start, end)
}

#[derive(Debug)]
struct Sample {
    channel: u8,
    bank: u8,
    pointer: u16,
    length: u16,
}

fn rom_read(rom: &[u8], bank: u8, address: u16) -> u8 {
    let offset = if address < 0x8000 {
        address as usize
    } else {
        0x8000 + bank as usize * 0x8000 + address as usize - 0x8000
    };
    rom[offset]
}

fn sample(rom: &[u8], code: u8) -> Sample {
    assert!((1..=0x3f).contains(&code));
    let channel = rom_read(rom, 0, 0x01b1 + code as u16);
    let bank = rom_read(rom, 0, 0x01f1 + code as u16) >> 5;
    let (base, index) = if code < 0x10 {
        (0x0231, code)
    } else {
        let mut index = code.wrapping_sub(0x3e);
        for (limit, subtract) in [
            (0x1c, 0x10),
            (0x1f, 0x1c),
            (0x29, 0x1f),
            (0x32, 0x29),
            (0x3a, 0x32),
            (0x3e, 0x3a),
        ] {
            if code < limit {
                index = code - subtract;
                break;
            }
        }
        (0x8000, index)
    };
    let entry = base + 2 * index as u16;
    let pointer = u16::from_le_bytes([rom_read(rom, bank, entry), rom_read(rom, bank, entry + 1)]);
    let length = u16::from_be_bytes([
        rom_read(rom, bank, pointer),
        rom_read(rom, bank, pointer + 1),
    ]);
    Sample {
        channel,
        bank,
        pointer,
        length,
    }
}

fn segment<'a>(log: &'a RefLog, actual: &[(u8, u32, f64)], index: usize) -> Vec<&'a PortWrite> {
    let start = actual[index].2;
    let end = actual.get(index + 1).map_or(f64::INFINITY, |entry| entry.2);
    log.writes
        .iter()
        .filter(|write| start <= write.t && write.t < end)
        .collect()
}

fn nibble(write: &PortWrite, channel: u8) -> bool {
    write.port == channel && write.data & 0xc0 == 0x40
}

#[test]
fn reference_runs_repeat_and_cover_every_command_with_full_checkpoints() {
    if !present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let files = observation_files(A);
    assert_eq!(files, observation_files(B));
    for name in files {
        assert_same_file(&Path::new(A).join(&name), &Path::new(B).join(&name), &name);
    }
    // Lua's device iteration order can differ; the hook rows themselves must not.
    let rows = |capture: &str| {
        let mut rows: Vec<String> =
            std::fs::read_to_string(Path::new(capture).join("hook_index.tsv"))
                .unwrap()
                .lines()
                .map(str::to_owned)
                .collect();
        rows.sort();
        rows
    };
    assert_eq!(rows(A), rows(B));
    for capture in [A, B] {
        assert_eq!(meta(capture, "dsw1"), "5fff");
        assert_eq!(meta(capture, "dsw2"), "ffff");
        assert_eq!(meta(capture, "frames_executed"), "12760");
    }

    let log = read_taps(&Path::new(A).join("audio_taps.tsv")).unwrap();
    let actual = commands(&log);
    let observed: Vec<(u8, u32)> = actual.iter().map(|entry| (entry.0, entry.1)).collect();
    assert_eq!(observed.as_slice(), COMMANDS);
    for code in 0x81..=0xbf {
        assert!(
            observed.iter().any(|entry| entry.0 == code),
            "missing {code:02x}"
        );
    }
    let checkpoints = checkpoints();
    for &(_, latch) in COMMANDS {
        assert!(
            checkpoints.contains(&(latch - 1)),
            "checkpoint before {latch}"
        );
    }
    assert!(checkpoints.contains(&LAST_FRAME));
    for device in [":msm1", ":msm2"] {
        let (start, end) = hook_span(device);
        assert!(
            start <= HOOK_FIRST as f64 / 60.0,
            "{device} hook starts at {start}"
        );
        assert!(
            (end - start) * 60.0 >= (LAST_FRAME - HOOK_FIRST) as f64,
            "{device} hook ends early"
        );
    }
}

#[test]
fn translated_samples_cpu_matches_every_reference_write_and_time() {
    if !present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let log = read_taps(&Path::new(A).join("audio_taps.tsv")).unwrap();
    let last = log.writes.last().unwrap().tick;
    assert!(last > frame_tick(LAST_FRAME - 10));
    let count = compare_writes(
        audio2(),
        latch_timeline(&log, None),
        &log.writes,
        0,
        last + 1,
    )
    .unwrap_or_else(|message| panic!("{message}"));
    assert_eq!(count, log.writes.len());
    let (timed, worst) = compare_write_times(
        audio2(),
        latch_timeline(&log, None),
        &log.writes,
        0,
        last + 1,
    )
    .unwrap_or_else(|message| panic!("{message}"));
    assert_eq!(timed, log.writes.len());
    assert!(
        worst.abs() < 1.0,
        "largest time difference {worst:.3} T-states over {timed} writes"
    );
    println!("{timed} port writes identical; worst time difference {worst:.6} T-states");
}

#[test]
fn both_dacs_match_every_tick_and_an_altered_rom_nibble_fails_at_its_tick() {
    if !present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let log = read_taps(&Path::new(A).join("audio_taps.tsv")).unwrap();
    let hook = read_hook(Path::new(A)).unwrap();
    let (ticks, active) = compare_dac(audio2(), latch_timeline(&log, None), &hook)
        .unwrap_or_else(|message| panic!("{message}"));
    assert!(
        ticks > 1_500_000,
        "whole two-chip hook window: {ticks} ticks"
    );
    assert!(
        active > 400_000,
        "substantial non-silent DAC coverage: {active} ticks"
    );
    println!("both MSM5205 DACs identical at {ticks} ticks ({active} non-silent)");

    // C-NEG: change the high nibble of command 0x81's first ROM data byte.
    // The release write precedes it; the next write is that exact nibble/tick.
    let actual = commands(&log);
    let first_data = *segment(&log, &actual, 0)
        .into_iter()
        .filter(|write| nibble(write, 0))
        .nth(1)
        .unwrap();
    let rom = audio2();
    let first = sample(&rom, 0x01);
    let address = first.pointer + 2;
    assert_eq!(
        first_data.data & 0x0f,
        rom_read(&rom, first.bank, address) >> 4
    );
    let mut altered = rom;
    let offset = address as usize;
    altered[offset] ^= 0x10;
    let error = compare_dac(altered, latch_timeline(&log, None), &hook).unwrap_err();
    assert!(
        error.starts_with(&format!("tick {} ", first_data.tick)),
        "altered nibble at tick {} reported a different first mismatch: {error}",
        first_data.tick
    );
    println!(
        "C-NEG: one altered ROM nibble caught at tick {}: {error}",
        first_data.tick
    );
}

#[test]
fn every_sample_ends_and_all_banks_stops_overlap_and_durations_are_covered() {
    if !present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let log = read_taps(&Path::new(A).join("audio_taps.tsv")).unwrap();
    let actual = commands(&log);
    let rom = audio2();
    let mut banks = BTreeSet::new();
    let mut unique = BTreeSet::new();
    let mut total_nibbles = 0u64;
    let mut unique_nibbles = 0u64;
    for (index, &(command, _)) in COMMANDS.iter().enumerate().take(63) {
        assert_eq!(command, 0x81 + index as u8);
        let sample = sample(&rom, command - 0x80);
        let writes = segment(&log, &actual, index);
        let nibbles: Vec<&&PortWrite> = writes
            .iter()
            .filter(|write| nibble(write, sample.channel))
            .collect();
        assert_eq!(
            nibbles.len(),
            sample.length as usize * 2 + 1,
            "{command:02x}: all sample nibbles plus the release write"
        );
        let next_latch = actual[index + 1].2;
        assert!(
            next_latch - nibbles.last().unwrap().t > 10.0 / 60.0,
            "{command:02x} natural-end margin"
        );
        assert!(writes
            .iter()
            .any(|write| write.port == 2 && write.data == sample.bank));
        total_nibbles += sample.length as u64 * 2;
        if unique.insert((sample.bank, sample.pointer, sample.length)) {
            unique_nibbles += sample.length as u64 * 2;
        }
    }
    for write in &log.writes {
        if write.port == 2 {
            banks.insert(write.data);
        }
    }
    assert_eq!(banks, (0..=6).collect());
    assert_eq!(total_nibbles, 436_506);
    assert_eq!(unique_nibbles, 409_530);
    assert!((total_nibbles as f64 / 4000.0 - 109.1265).abs() < 1e-9);
    assert!((unique_nibbles as f64 / 4000.0 - 102.3825).abs() < 1e-9);
    assert!((unique_nibbles as f64 / 4000.0 - 102.0).abs() < 0.5);

    let assert_reset_stops = |stop: usize, channel: u8| {
        let stopped = segment(&log, &actual, stop);
        let reset = stopped
            .iter()
            .position(|write| write.port == channel && write.data == 0xc0)
            .unwrap_or_else(|| panic!("channel {channel} has no reset after 0x80"));
        let in_flight = stopped[..reset]
            .iter()
            .filter(|write| nibble(write, channel))
            .count();
        assert!(
            in_flight <= 1,
            "channel {channel} emitted {in_flight} nibbles before its reset"
        );
        assert!(
            !stopped[reset + 1..]
                .iter()
                .any(|write| nibble(write, channel)),
            "channel {channel} emitted a nibble after its reset"
        );
    };

    // Each stop follows an active long sample. The channel-1 interval spans
    // the exit helper's FM-only 27 and 3f commands before its final 80.
    for (source, stop, channel) in [(63, 64, 0), (68, 71, 1)] {
        let start_time = actual[source].2;
        let stop_time = actual[stop].2;
        let playing: Vec<&PortWrite> = log
            .writes
            .iter()
            .filter(|write| start_time <= write.t && write.t < stop_time && nibble(write, channel))
            .collect();
        assert!(
            playing.len() > 1_000,
            "channel {channel} active before 0x80"
        );
        assert!(
            stop_time - playing.last().unwrap().t < 0.1,
            "0x80 stops active channel {channel}"
        );
        assert_reset_stops(stop, channel);
    }

    // After the second start, channel 1 has substantial simultaneous playback
    // alongside channel 0 (within two services, as in fight_samples.rs).
    let overlap = segment(&log, &actual, 66);
    let channel_0_ticks: BTreeSet<u64> = overlap
        .iter()
        .filter(|write| nibble(write, 0))
        .map(|write| write.tick)
        .collect();
    let overlapping = overlap
        .iter()
        .filter(|write| nibble(write, 1))
        .filter(|write| {
            (write.tick.saturating_sub(4)..=write.tick + 4)
                .any(|tick| channel_0_ticks.contains(&tick))
        })
        .count();
    assert!(
        overlapping > 1_000,
        "both chips overlap for {overlapping} channel-1 nibbles"
    );
    for channel in [0, 1] {
        assert_reset_stops(67, channel);
    }
    println!(
        "63 isolated commands: {total_nibbles} nibbles / {:.4} s; unique samples: {unique_nibbles} / {:.4} s; banks {banks:?}; overlap {overlapping}",
        total_nibbles as f64 / 4000.0,
        unique_nibbles as f64 / 4000.0
    );
}

#[test]
fn samples_cpu_has_no_untranslated_report_sites() {
    let source = include_str!("../src/samples.rs");
    assert!(!source.contains("note("));
    assert!(!source.contains("note_unrecovered"));
}
