//! Acceptance test for ticket #32: the translated samples-CPU program over
//! the whole fixed fight (sf-fixed-fight-01: power-on through the end of
//! round 2, 5000 frames), driven by the reference's sound-latch writes,
//! reproduces every samples-CPU port write of the pinned MAME reference —
//! channel 1, the banked sample tables and overlapping playback on both
//! chips included — and both MSM5205 DAC outputs match the reference's
//! hooked device streams at every channel tick over the round-2 KO window
//! (frames 4740..=4840). docs/reference/fight-samples.md.
//!
//! Consumes local-only data (two reference audio runs from
//! tools/reference/run_audio_probe.py, the ROM set); skips on a clean
//! checkout.

use std::collections::BTreeSet;
use std::path::Path;

use sf_audio::check::{
    compare_dac, compare_write_times, compare_writes, frame_tick, latch_timeline, read_hook,
    read_taps,
};
use sf_audio::samples::PortWrite;

const RUN_A: &str = "../../tools/reference/local/captures/fight-audio-a";
const RUN_B: &str = "../../tools/reference/local/captures/fight-audio-b";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
/// The hooked window: the round-2 KO.
const HOOK_FIRST: u32 = 4740;
const HOOK_LAST: u32 = 4840;

fn present() -> bool {
    [RUN_A, RUN_B, ROMS].iter().all(|p| Path::new(p).is_dir())
}

fn audio2() -> Vec<u8> {
    sf_video::romset::load_region(ROMS, MANIFEST, "audio2").expect("audio2 region")
}

#[test]
fn reference_audio_runs_repeat_exactly() {
    if !present() {
        eprintln!("skipping: local reference audio runs/ROMs not present");
        return;
    }
    for name in [
        "audio_taps.tsv",
        "hook_msm1.f32",
        "hook_msm2.f32",
        "hook_ymsnd.f32",
        "audio.wav",
    ] {
        let a = std::fs::read(Path::new(RUN_A).join(name)).unwrap();
        let b = std::fs::read(Path::new(RUN_B).join(name)).unwrap();
        assert!(
            a == b,
            "{name}: the two reference audio runs must be identical"
        );
    }
    // The hook index lists each update's devices in Lua's table order,
    // which varies between runs; the rows themselves are the same.
    let rows = |dir: &str| {
        let mut v: Vec<String> = std::fs::read_to_string(Path::new(dir).join("hook_index.tsv"))
            .unwrap()
            .lines()
            .map(str::to_string)
            .collect();
        v.sort();
        v
    };
    assert_eq!(rows(RUN_A), rows(RUN_B), "hook_index.tsv: the same rows");
}

#[test]
fn translated_program_reproduces_every_reference_write_over_the_fight() {
    if !present() {
        eprintln!("skipping: local reference audio runs/ROMs not present");
        return;
    }
    let log = read_taps(&Path::new(RUN_A).join("audio_taps.tsv")).unwrap();
    let last = log.writes.last().unwrap().tick;
    assert!(
        last > frame_tick(4990),
        "the run reaches the scenario's end"
    );
    let n = compare_writes(
        audio2(),
        latch_timeline(&log, None),
        &log.writes,
        0,
        last + 1,
    )
    .unwrap_or_else(|m| panic!("{m}"));
    assert_eq!(n, log.writes.len(), "every reference write compared");
}

#[test]
fn dac_outputs_match_the_reference_streams_over_the_ko_window() {
    if !present() {
        eprintln!("skipping: local reference audio runs/ROMs not present");
        return;
    }
    let log = read_taps(&Path::new(RUN_A).join("audio_taps.tsv")).unwrap();
    let hook = read_hook(Path::new(RUN_A)).unwrap();
    let span = (hook.end - hook.start) * 60.0;
    assert!(
        span >= (HOOK_LAST - HOOK_FIRST) as f64,
        "the hook covers the window ({span:.1} frames)"
    );
    let (ticks, active) =
        compare_dac(audio2(), latch_timeline(&log, None), &hook).unwrap_or_else(|m| panic!("{m}"));
    assert!(
        ticks > 13_000 && active > 1_000,
        "the KO window is compared ({ticks} ticks, {active} active)"
    );
}

/// The samples CPU's T-state timing: every port write's modelled time
/// agrees with the reference's logged time to within one T-state.
#[test]
fn every_write_lands_at_the_reference_time() {
    if !present() {
        eprintln!("skipping: local reference audio runs/ROMs not present");
        return;
    }
    let log = read_taps(&Path::new(RUN_A).join("audio_taps.tsv")).unwrap();
    let last = log.writes.last().unwrap().tick;
    let (n, worst) = compare_write_times(
        audio2(),
        latch_timeline(&log, None),
        &log.writes,
        0,
        last + 1,
    )
    .unwrap_or_else(|m| panic!("{m}"));
    assert_eq!(n, log.writes.len());
    assert!(
        worst.abs() < 1.0,
        "largest time difference {worst:.3} T-states over {n} writes"
    );
}

/// What the run exercises that the punch-hit slice (ticket #10) did not:
/// channel 1 playing, a bank other than 0 selected, and both chips playing
/// at once.
#[test]
fn the_fight_plays_channel_one_banked_samples_and_both_chips_at_once() {
    if !present() {
        eprintln!("skipping: local reference audio runs/ROMs not present");
        return;
    }
    let log = read_taps(&Path::new(RUN_A).join("audio_taps.tsv")).unwrap();
    let nibble = |w: &PortWrite| w.port < 2 && w.data & 0xc0 == 0x40;
    let ch1: Vec<&PortWrite> = log
        .writes
        .iter()
        .filter(|w| w.port == 1 && nibble(w))
        .collect();
    assert!(
        ch1.len() > 1000,
        "channel 1 plays samples ({} nibbles)",
        ch1.len()
    );
    let banks: BTreeSet<u8> = log
        .writes
        .iter()
        .filter(|w| w.port == 2)
        .map(|w| w.data)
        .collect();
    assert!(
        banks.contains(&2),
        "bank 2 is selected (banks seen: {banks:?})"
    );
    // Overlap: a channel-1 nibble within 4 ticks (two services) of a
    // channel-0 nibble, i.e. both chips mid-sample.
    let ch0_ticks: BTreeSet<u64> = log
        .writes
        .iter()
        .filter(|w| w.port == 0 && nibble(w))
        .map(|w| w.tick)
        .collect();
    let overlapping = ch1
        .iter()
        .filter(|w| (w.tick.saturating_sub(4)..=w.tick + 4).any(|t| ch0_ticks.contains(&t)))
        .count();
    assert!(
        overlapping > 100,
        "both chips play at once ({overlapping} channel-1 nibbles alongside channel 0)"
    );
}
