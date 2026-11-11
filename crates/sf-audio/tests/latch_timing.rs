//! Acceptance test for ticket #35: the sound latch's write time within the
//! frame. The native engine accounts the vblank handler's 68000 cycles to
//! the write on every frame; the interrupt's acceptance latency before the
//! handler is the one quantity the translation cannot give (the main loop's
//! cycle position at the frame start), disclosed as the observed band
//! `IRQ_LATENCY_CYCLES`. Runs sf-win-01 natively (frames 2389..=6251: the
//! 1P fight with heavy punches, KO and result screens, a palette load, the
//! match end) and checks every reference latch write against the native
//! handler cycles. docs/reference/latch-timing.md.
//!
//! Consumes local-only data (the reference capture and audio run, the ROM
//! set); skips on a clean checkout.

use std::collections::BTreeSet;
use std::path::Path;

use sf_audio::check::{latch_residuals, native_latch, read_taps};
use sf_audio::IRQ_LATENCY_CYCLES;

const WIN_REF: &str = "../../tools/reference/local/captures/win-01-run-a";
const WIN_AUDIO: &str = "../../tools/reference/local/captures/win-01-audio";
const SCENARIO: &str = "../../tools/reference/scenarios/sf_win_01.json";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const FIXTURE: u32 = 2389;
const LAST: u32 = 6251;

fn present() -> bool {
    [WIN_REF, WIN_AUDIO, ROMS]
        .iter()
        .all(|p| Path::new(p).is_dir())
}

/// Every reference write of the run lies the interrupt latency after the
/// native handler cycles — a latency inside the observed band — with the
/// same value; the run exercises the handler's idle, heavy first-press,
/// heavy held, release and palette-load paths, and the band's whole extent.
#[test]
fn win_01_handler_cycles_put_every_reference_write_inside_the_latency_band() {
    if !present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions");
    let out = sf_engine::run::ScratchDir::new("sf-audio-latch-win");
    sf_engine::run::run_cpu_fight_capture_with_inputs(
        Path::new(WIN_REF),
        &assets,
        &out,
        "sf-win-01",
        Some(Path::new(SCENARIO)),
        FIXTURE,
        LAST,
    )
    .expect("native win-01 run");
    let frames = native_latch(&out).expect("native latch with cycles");
    let log = read_taps(&Path::new(WIN_AUDIO).join("audio_taps.tsv")).expect("reference tap log");
    let residuals = latch_residuals(&log, &frames);
    assert_eq!(
        residuals.len() as u32,
        LAST - FIXTURE,
        "every native frame has a reference write to check"
    );
    let (mut lo, mut hi) = (f64::MAX, f64::MIN);
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
        lo = lo.min(r.latency);
        hi = hi.max(r.latency);
    }
    let costs: BTreeSet<u32> = residuals.iter().map(|r| r.cycles).collect();
    for (cost, what) in [
        (696, "in-game frame without button latches"),
        (1098, "1P fight, buttons idle"),
        (1154, "heavy button first pressed"),
        (1188, "heavy button held"),
        (1214, "button released"),
        (6010, "in-game palette load"),
    ] {
        assert!(
            costs.contains(&cost),
            "the run has a {what} frame ({cost} cycles); costs seen: {costs:?}"
        );
    }
    assert!(
        lo < 50.0 && hi > 130.0,
        "the run spans the band's extent ({lo:.1}..{hi:.1})"
    );
}

const FIGHT_REF: &str = "../../tools/reference/local/captures/cpu-01-run-a";
const FIGHT_AUDIO: &str = "../../tools/reference/local/captures/fight-audio-a";
const FIGHT_SCENARIO: &str = "../../tools/reference/scenarios/sf_fixed_fight_01.json";
const FIGHT_LAST: u32 = 5000;

/// The fixed fight from checkpoint 2389 through the end of round 2, with
/// its light punch and kick: every reference write inside the band, with
/// the same value; the light button's first-press and held paths occur.
#[test]
fn fixed_fight_handler_cycles_put_every_reference_write_inside_the_latency_band() {
    if ![FIGHT_REF, FIGHT_AUDIO, ROMS]
        .iter()
        .all(|p| Path::new(p).is_dir())
    {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions");
    let out = sf_engine::run::ScratchDir::new("sf-audio-latch-fight");
    sf_engine::run::run_cpu_fight_capture_with_inputs(
        Path::new(FIGHT_REF),
        &assets,
        &out,
        "sf-fixed-fight-01",
        Some(Path::new(FIGHT_SCENARIO)),
        FIXTURE,
        FIGHT_LAST,
    )
    .expect("native fixed-fight run");
    let frames = native_latch(&out).expect("native latch with cycles");
    let log = read_taps(&Path::new(FIGHT_AUDIO).join("audio_taps.tsv")).expect("reference tap log");
    let residuals = latch_residuals(&log, &frames);
    assert_eq!(
        residuals.len() as u32,
        FIGHT_LAST - FIXTURE,
        "every native frame has a reference write to check"
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
    let costs: BTreeSet<u32> = residuals.iter().map(|r| r.cycles).collect();
    for (cost, what) in [
        (1134, "light button first pressed"),
        (1168, "light button held"),
        (1214, "button released"),
    ] {
        assert!(
            costs.contains(&cost),
            "the run has a {what} frame ({cost} cycles); costs seen: {costs:?}"
        );
    }
}
