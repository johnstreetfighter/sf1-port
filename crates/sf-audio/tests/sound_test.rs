//! Acceptance tests for ticket #62 (F22, label SM): every FM effect and
//! special command selected and played by the reference service sound test.
//! Local-only reference captures and ROMs; skips on a clean checkout.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use sf_audio::check::{
    exact_clock_matches, latch_timeline, music_ram_at_frame_ends, read_device_stream, read_taps,
    reference_opm_writes, replay_opm, run_music_from_power_on,
};
use sf_audio::opm::OPM_RATE;

const A: &str = "../../tools/reference/local/captures/sound-test-01-audio-a";
const B: &str = "../../tools/reference/local/captures/sound-test-01-audio-b";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const LAST_FRAME: u32 = 8699;

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

#[test]
fn reference_opm_replay_matches_the_hooked_samples() {
    if !present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let log = read_taps(&Path::new(A).join("audio_taps.tsv")).unwrap();
    let hook = read_device_stream(Path::new(A), "ymsnd", OPM_RATE as f64).unwrap();
    let n0 = hook.first_sample();
    let native = replay_opm(&log.opm, (n0 + hook.samples.len() as u64) as usize, 0.0);
    for (i, &reference) in hook.samples.iter().enumerate() {
        let sample = n0 + i as u64;
        assert_eq!(
            native[sample as usize], reference,
            "reference replay sample {sample}"
        );
    }
}

#[test]
fn reference_runs_repeat_and_cover_every_service_sound_command() {
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
    // DSW1 0xdfff -> 0x5fff changes only Service Mode; DSW2 is unchanged.
    assert_eq!(meta(A, "dsw1"), "5fff");
    assert_eq!(meta(A, "dsw2"), "ffff");
    assert_eq!(meta(B, "dsw1"), "5fff");
    assert_eq!(meta(B, "dsw2"), "ffff");

    let log = read_taps(&Path::new(A).join("audio_taps.tsv")).unwrap();
    let commands: Vec<u8> = log
        .latch
        .iter()
        .filter(|&&(t, _)| t > 20.0 / 60.0)
        .map(|&(_, v)| v)
        .filter(|&v| v < 0x40)
        .collect();
    assert_eq!(
        commands.len(),
        108,
        "the planned service-test commands, with no sample commands"
    );
    let covered: BTreeSet<u8> = commands.iter().copied().collect();
    for command in (0x00..=0x27).chain([0x28, 0x33, 0x35, 0x3d, 0x3f]) {
        assert!(
            covered.contains(&command),
            "command {command:02x} was not played"
        );
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
        compared > 8_000_000,
        "the whole hook window is compared ({compared} samples)"
    );
    assert!(
        run.observations.owned_skips > 0,
        "music must skip an effect-owned channel through 0xc023"
    );
    assert!(
        run.observations.zero_slide_commands & (1 << 0x22) != 0,
        "effect 0x22 must reach the zero-slide path"
    );
    println!(
        "sound test: {} writes, all on the exact clock; {compared} output samples identical",
        native.len()
    );
    println!(
        "required paths: {} owned-channel skips, {} zero slides",
        run.observations.owned_skips, run.observations.zero_slides
    );
}

#[test]
fn music_ram_matches_at_every_idle_checkpoint_and_reports_non_idle_commands() {
    if !present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let log = read_taps(&Path::new(A).join("audio_taps.tsv")).unwrap();
    let rom = sf_video::romset::load_region(ROMS, MANIFEST, "audiocpu").unwrap();
    let checkpoints = std::fs::read_to_string(Path::new(A).join("checkpoints.tsv")).unwrap();
    let frames: Vec<u32> = checkpoints
        .lines()
        .skip(1)
        .filter_map(|line| line.split('\t').next()?.parse().ok())
        .collect();
    let (states, unrecovered) = music_ram_at_frame_ends(rom, latch_timeline(&log, None), &frames);
    assert!(
        unrecovered.is_empty(),
        "untranslated paths reached: {unrecovered:04x?}"
    );
    assert_eq!(states.len(), frames.len());

    // The reference's pre-command bytes prove that each required path was
    // exercised. Native RAM is compared there only when idle, but the bytes
    // after each required transition are checked even at non-idle dumps.
    for (frame, before, after) in [
        (2480, [0x03, 0x00], [0x00, 0x00]), // 02@2468 -> 00@2480: both stop.
        (2707, [0x03, 0x00], [0x01, 0x00]), // 0f@2683 -> 10@2707: replace the changed mask.
        (2807, [0x81, 0x00], [0x00, 0x00]), // 0f@2763 -> 0c@2807: stop a non-empty effect.
    ] {
        for (checkpoint, expected) in [(frame - 1, before), (frame, after)] {
            let (_, is_idle, native) = states.iter().find(|(f, _, _)| *f == checkpoint).unwrap();
            let reference =
                std::fs::read(Path::new(A).join(format!("ram_{checkpoint:06}_music.bin"))).unwrap();
            for (offset, expected) in [0x500usize, 0x580].into_iter().zip(expected) {
                assert_eq!(
                    reference[offset],
                    expected,
                    "reference mask at frame {checkpoint}, address {:#06x}",
                    0xc000 + offset
                );
                if checkpoint == frame || *is_idle {
                    assert_eq!(
                        native[offset],
                        expected,
                        "native mask at frame {checkpoint}, address {:#06x}",
                        0xc000 + offset
                    );
                }
            }
        }
    }

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
        let reference =
            std::fs::read(Path::new(A).join(format!("ram_{frame:06}_music.bin"))).unwrap();
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
    for &(t, command) in &log.latch {
        let frame = (t * 60.0).floor() as u32;
        if frame > 20 && command < 0x40 && idle_by_frame.get(&(frame - 1)) != Some(&true) {
            not_compared.push(format!("{command:02x}@{frame}"));
        }
    }
    println!("music RAM: {idle} idle checkpoints identical; {not_idle} not idle");
    println!(
        "commands without an idle checkpoint before the next command (F72): {}",
        not_compared.join(", ")
    );
}

#[test]
fn moving_an_isolated_latch_write_breaks_exact_clock_comparison() {
    if !present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let log = read_taps(&Path::new(A).join("audio_taps.tsv")).unwrap();
    let rom = sf_video::romset::load_region(ROMS, MANIFEST, "audiocpu").unwrap();
    let s_end = (400.0 / 60.0 * OPM_RATE as f64).ceil() as u64;
    let reference: Vec<_> = reference_opm_writes(&log)
        .into_iter()
        .filter(|w| w.sample < s_end)
        .collect();
    let mut events = latch_timeline(&log, None).into_events();
    let mut moved = 0;
    for (t, command) in &mut events {
        if *command == 0x01 && (*t * 60.0).floor() as u32 == 157 {
            *t += 0.004;
            moved += 1;
        }
    }
    assert_eq!(moved, 1, "the control moves only 01@157");
    let shifted = run_music_from_power_on(rom, sf_audio::LatchTimeline::new(events), s_end);
    assert!(
        shifted.unrecovered.is_empty(),
        "untranslated paths reached: {:04x?}",
        shifted.unrecovered
    );
    let exact = exact_clock_matches(&shifted.writes, &reference);
    assert!(
        exact < reference.len(),
        "C-NEG: moving 01@157 by 4 ms must change an exact-clock/write comparison"
    );
    println!(
        "C-NEG: 01@157 +4 ms leaves {exact}/{} writes on the exact clock",
        reference.len()
    );
}
