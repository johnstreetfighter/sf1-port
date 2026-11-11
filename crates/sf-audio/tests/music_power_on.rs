//! Acceptance test for ticket #36: the translated music CPU from power-on
//! over the whole fixed fight (sf-fixed-fight-01, 5,000 frames, 83 s): the
//! Z80 from its reset vector at clock 0 — the boot code's RAM clear, chip
//! init and timer loads, then the idle loop — on the reference's latch
//! writes, with no RAM fixture and no fitted idle-loop phase. Every YM2151
//! write of the run must match the reference's in order within one output
//! sample, and the chip output must be sample-identical to the reference's
//! hooked stream over the run. docs/reference/music-power-on.md.
//!
//! Consumes local-only data (fight-audio-a: the taps; fight-audio-ym: the
//! same run with the YM2151 stream hooked over all frames; the ROM set);
//! skips on a clean checkout.

use std::path::Path;

use sf_audio::check::{
    exact_clock_matches, latch_timeline, read_device_stream, read_taps, reference_opm_writes,
    run_music_from_power_on,
};
use sf_audio::opm::OPM_RATE;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const TAPS: &str = "../../tools/reference/local/captures/fight-audio-a";
const HOOK: &str = "../../tools/reference/local/captures/fight-audio-ym";
const LAST_FRAME: u32 = 4999;

fn present() -> bool {
    [TAPS, HOOK, ROMS].iter().all(|p| Path::new(p).is_dir())
}

#[test]
fn the_hooked_run_repeats_the_tap_log() {
    if !present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let a = std::fs::read(Path::new(TAPS).join("audio_taps.tsv")).unwrap();
    let b = std::fs::read(Path::new(HOOK).join("audio_taps.tsv")).unwrap();
    assert!(
        a == b,
        "the run with the whole-fight hook logs the same chip writes"
    );
}

/// From reset at clock 0 through frame 4999: every write of the reference
/// in order, on the exact chip clock, and the output sample-identical.
#[test]
fn the_music_cpu_from_power_on_plays_the_whole_fixed_fight_like_the_reference() {
    if !present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let log = read_taps(&Path::new(TAPS).join("audio_taps.tsv")).expect("tap log");
    let rom = sf_video::romset::load_region(ROMS, MANIFEST, "audiocpu").expect("audiocpu region");
    let s_end = ((LAST_FRAME + 1) as f64 / 60.0 * OPM_RATE as f64).ceil() as u64;
    let run = run_music_from_power_on(rom, latch_timeline(&log, None), s_end);
    assert!(
        run.unrecovered.is_empty(),
        "translated paths only: {:04x?}",
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
    assert!(
        reference.len() > 140_000,
        "the whole fight's writes ({})",
        reference.len()
    );
    assert_eq!(native.len(), reference.len(), "every write");
    // The boot code's first chip write (channel 7's first total level) from
    // reset at clock 0: address at 43,213, data at 43,239.
    assert_eq!(
        (native[0].reg, native[0].data, native[0].clock),
        (0x67, 0x7f, 43_239),
        "the first write of the boot"
    );
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
    assert_eq!(exact, native.len(), "every write on the exact chip clock");

    let hook =
        read_device_stream(Path::new(HOOK), "ymsnd", OPM_RATE as f64).expect("ymsnd hook stream");
    let n0 = hook.first_sample();
    let mut compared = 0u64;
    for (i, r) in hook.samples.iter().enumerate() {
        let n = n0 + i as u64;
        if n < s_end {
            assert_eq!(
                run.left[n as usize],
                *r,
                "output sample {n} (frame {:.1})",
                n as f64 / OPM_RATE as f64 * 60.0
            );
            compared += 1;
        }
    }
    assert!(
        compared > 4_600_000,
        "the whole run's output is compared ({compared} samples)"
    );
    println!(
        "power-on run: {} writes, all on the exact clock; {compared} output samples identical",
        native.len()
    );
}
