//! Acceptance test for ticket #10: the punch-hit (impact) sound of
//! sf-punch-01, played by the translated samples-CPU program from the native
//! engine's hit event, matches the pinned MAME reference — every samples-CPU
//! port write (chip data, resets, bank selects, at interrupt-tick
//! resolution) and both MSM5205 DAC outputs at every channel tick.
//!
//! Consumes local-only data (reference audio runs from
//! tools/reference/run_audio_probe.py, reference captures, ROM set); skips on
//! a clean checkout. docs/reference/hit-sound.md.

use std::path::Path;

use sf_audio::check::{
    compare_dac, compare_writes, frame_tick, latch_residuals, latch_timeline, native_latch,
    read_hook, read_taps, NativeLatch,
};
use sf_audio::samples::PortWrite;
use sf_audio::SamplesSound;
use sf_audio::{IRQ_LATENCY_CYCLES, IRQ_LATENCY_MID};

const RUN_A: &str = "../../tools/reference/local/captures/hit-sound-run-a";
const RUN_B: &str = "../../tools/reference/local/captures/hit-sound-run-b";
const HOOK: &str = "../../tools/reference/local/captures/hit-sound-hook";
const PUNCH_REF: &str = "../../tools/reference/local/captures/punch-01-run-a";
const SCENARIO: &str = "../../tools/reference/scenarios/sf_punch_01.json";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

fn present() -> bool {
    [RUN_A, RUN_B, HOOK, PUNCH_REF, ROMS]
        .iter()
        .all(|p| Path::new(p).is_dir())
}

fn audio2() -> Vec<u8> {
    sf_video::romset::load_region(ROMS, MANIFEST, "audio2").expect("audio2 region")
}

/// The native punch run's per-frame latch (the native hit event's sound
/// commands), produced fresh by the native engine — once per test process,
/// since the tests run in parallel and would share the capture directory.
fn native_punch_latch() -> Vec<(u32, u8, u32)> {
    static LATCH: std::sync::OnceLock<Vec<(u32, u8, u32)>> = std::sync::OnceLock::new();
    LATCH
        .get_or_init(|| {
            let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions");
            let out = sf_engine::run::ScratchDir::new("sf-audio-hit");
            sf_engine::run::run_punch_capture(
                Path::new(PUNCH_REF),
                &assets,
                &out,
                "sf-punch-01",
                Path::new(SCENARIO),
            )
            .expect("native punch run");
            native_latch(&out).expect("native latch")
        })
        .clone()
}

#[test]
fn reference_audio_runs_repeat_exactly() {
    if !present() {
        eprintln!("skipping: local reference audio runs/ROMs not present");
        return;
    }
    let a = std::fs::read(Path::new(RUN_A).join("audio_taps.tsv")).unwrap();
    let b = std::fs::read(Path::new(RUN_B).join("audio_taps.tsv")).unwrap();
    assert!(
        a == b,
        "two reference audio runs must log identical chip writes"
    );
}

#[test]
fn translated_program_reproduces_every_reference_write_from_power_on() {
    if !present() {
        eprintln!("skipping: local reference audio runs/ROMs not present");
        return;
    }
    let log = read_taps(&Path::new(RUN_A).join("audio_taps.tsv")).unwrap();
    let last = log.writes.last().unwrap().tick;
    let n = compare_writes(audio2(), latch_timeline(&log, None), &log.writes, 0, last)
        .unwrap_or_else(|m| panic!("{m}"));
    assert!(n > 190_000, "the whole run is compared ({n} writes)");
}

#[test]
fn native_hit_event_drives_the_reference_writes_and_dac_output() {
    if !present() {
        eprintln!("skipping: local reference audio runs/ROMs not present");
        return;
    }
    let frames = native_punch_latch();
    let log = read_taps(&Path::new(RUN_A).join("audio_taps.tsv")).unwrap();
    let tl = || {
        latch_timeline(
            &log,
            Some(NativeLatch {
                frames: &frames,
                from: 2450,
                latency: IRQ_LATENCY_MID,
            }),
        )
    };
    let n = compare_writes(
        audio2(),
        tl(),
        &log.writes,
        frame_tick(2450),
        frame_tick(3400),
    )
    .unwrap_or_else(|m| panic!("{m}"));
    assert_eq!(n, 68968, "all writes of the native interval");

    let hook_log = read_taps(&Path::new(HOOK).join("audio_taps.tsv")).unwrap();
    let hook = read_hook(Path::new(HOOK)).unwrap();
    let (ticks, active) = compare_dac(
        audio2(),
        latch_timeline(
            &hook_log,
            Some(NativeLatch {
                frames: &frames,
                from: 2450,
                latency: IRQ_LATENCY_MID,
            }),
        ),
        &hook,
    )
    .unwrap_or_else(|m| panic!("{m}"));
    assert!(
        ticks > 6000 && active > 2000,
        "the hit window is compared ({ticks} ticks, {active} active)"
    );
}

#[test]
fn the_impact_starts_three_ticks_into_the_latch_frame_and_plays_2496_nibbles() {
    if !present() {
        eprintln!("skipping: local reference audio runs/ROMs not present");
        return;
    }
    let frames = native_punch_latch();
    assert!(
        frames.iter().any(|&(f, v, _)| f == 2717 && v == 0x84),
        "impact command on the hit frame + 1"
    );
    let log = read_taps(&Path::new(RUN_A).join("audio_taps.tsv")).unwrap();
    let mut tl = latch_timeline(
        &log,
        Some(NativeLatch {
            frames: &frames,
            from: 2450,
            latency: IRQ_LATENCY_MID,
        }),
    );
    let mut snd = SamplesSound::new(audio2());
    let mut writes: Vec<PortWrite> = Vec::new();
    while snd.ticks() < frame_tick(2770) {
        snd.tick(&mut tl, &mut |w| writes.push(w));
    }
    let start = frame_tick(2717);
    let chip0: Vec<PortWrite> = writes
        .iter()
        .copied()
        .filter(|w| w.port == 0 && w.tick >= start)
        .collect();
    // The swing (0x81, same channel) is still playing; the poll that sees
    // 0x84 cuts it: reset, release (a 0 nibble), then the sample's nibbles at
    // one per channel service (4 kHz), then reset.
    let cut = chip0
        .iter()
        .position(|w| w.data == 0xc0)
        .expect("impact reset");
    assert!(
        chip0[..cut].iter().all(|w| w.data & 0xc0 == 0x40),
        "swing nibbles before the cut"
    );
    let chip0 = &chip0[cut..];
    assert_eq!(
        chip0[0],
        PortWrite {
            tick: 362269,
            port: 0,
            data: 0xc0,
            t: 0.0
        },
        "impact reset"
    );
    assert_eq!(chip0[1].data, 0x40, "reset released");
    let nibbles = chip0[2..].iter().take_while(|w| w.data != 0xc0).count();
    assert_eq!(
        nibbles,
        2 * 1248,
        "sample 0x04: 1248 bytes, two nibbles each"
    );
    assert_eq!(
        chip0[2].tick - chip0[1].tick,
        4,
        "first nibble two services after the release"
    );
}

/// Ticket #35: the native run's latch writes are placed in the frame from
/// the vblank handler's cycles. Every reference write over the native
/// interval lies the interrupt latency after the native cycles — a latency
/// inside the observed band — and the samples CPU's writes match the
/// reference at both ends of the band: its width does not change which
/// poll sees a command here.
#[test]
fn native_latch_timing_puts_the_reference_writes_inside_the_latency_band() {
    if !present() {
        eprintln!("skipping: local reference audio runs/ROMs not present");
        return;
    }
    let frames = native_punch_latch();
    let log = read_taps(&Path::new(RUN_A).join("audio_taps.tsv")).unwrap();
    let residuals = latch_residuals(&log, &frames);
    assert!(
        residuals.len() > 900,
        "the native interval's frames are compared ({})",
        residuals.len()
    );
    for r in &residuals {
        assert_eq!(r.native, r.reference, "frame {}: latch value", r.frame);
        assert!(
            IRQ_LATENCY_CYCLES.contains(&(r.latency.round() as u32)),
            "frame {}: the reference's write implies a latency of {:.1} cycles over the native {} (band {:?})",
            r.frame,
            r.latency,
            r.cycles,
            IRQ_LATENCY_CYCLES
        );
    }
    let costs: std::collections::BTreeSet<u32> = residuals.iter().map(|r| r.cycles).collect();
    assert!(
        costs.len() >= 3,
        "idle, held and release frames occur ({costs:?})"
    );
    for latency in [*IRQ_LATENCY_CYCLES.start(), *IRQ_LATENCY_CYCLES.end()] {
        let tl = latch_timeline(
            &log,
            Some(NativeLatch {
                frames: &frames,
                from: 2450,
                latency,
            }),
        );
        let n = compare_writes(
            audio2(),
            tl,
            &log.writes,
            frame_tick(2450),
            frame_tick(3400),
        )
        .unwrap_or_else(|m| panic!("latency {latency} cycles: {m}"));
        assert_eq!(
            n, 68968,
            "latency {latency} cycles: all writes of the native interval"
        );
    }
}
