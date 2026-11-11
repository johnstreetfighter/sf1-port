//! Acceptance test for ticket #34: the FM sound effects, played through the
//! translated effect start (0x0240) and effects player (0x0ab9-0x0faf) from
//! the native engine's hit event. The sf-punch-01 hit at frame 2716 queues
//! the impact sample 0x84 (latched in 2717) and the hit-spark effect 0x01
//! (latched in 2718); the native run of the fight supplies both latch
//! writes, and the music CPU, run from the reference's music-RAM dump at
//! the end of frame 2716 through 2741, must reproduce every YM2151 write of
//! the reference (the effect's 408 writes on channels 6 and 7 interleaved
//! with tune 0x28's and the timer re-arms) in order within one output
//! sample, with the hooked chip output sample-identical, and its RAM equal
//! to the reference's dumps at idle frame ends after the effect.
//! docs/reference/fm-effects.md. Local-only data; skips without it.

use std::path::Path;

use sf_audio::check::{
    exact_clock_matches, latch_timeline, native_latch, read_device_stream, read_taps,
    reference_opm_writes, run_music, MusicRun, NativeLatch, RefLog,
};
use sf_audio::opm::OPM_RATE;
use sf_audio::{IRQ_LATENCY_CYCLES, IRQ_LATENCY_MID};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
/// The reference run's per-frame RAM dumps (the fixture, the RAM checks).
const PUNCH_REF: &str = "../../tools/reference/local/captures/punch-01-run-a";
/// The same run's chip-write taps and hooked YM2151 stream (frames 2714-2760).
const HOOK: &str = "../../tools/reference/local/captures/hit-sound-hook";
const SCENARIO: &str = "../../tools/reference/scenarios/sf_punch_01.json";
/// The native versus run starts at the fixture frame of punch-01-run-a.
const NATIVE_FROM: u32 = 2450;
/// The window: the end of 2716 (the music CPU idle, both effect slots empty,
/// tune 0x28 playing on channels 0-5) through 2741 (idle; the effect ended
/// at 2728 and the tune's channels 4/5 resumed at 2729-2730).
const FIXTURE: u32 = 2716;
const LAST: u32 = 2741;
/// The idle-loop phase at the fixture (`music_phase` example).
const PHASE: u64 = 4;
/// Frame ends at which the music CPU is idle, for the RAM comparison.
const RAM_CHECKS: [u32; 3] = [2734, 2738, 2741];

fn present() -> bool {
    [PUNCH_REF, HOOK, ROMS]
        .iter()
        .all(|p| Path::new(p).is_dir())
}

fn rom() -> Vec<u8> {
    sf_video::romset::load_region(ROMS, MANIFEST, "audiocpu").expect("audiocpu region")
}

fn music_ram(frame: u32) -> Vec<u8> {
    std::fs::read(Path::new(PUNCH_REF).join(format!("ram_{frame:06}_music.bin")))
        .expect("music RAM dump")
}

fn sample_at_end_of(frame: u32) -> u64 {
    ((frame + 1) as f64 / 60.0 * OPM_RATE as f64).ceil() as u64
}

/// The native punch run's per-frame latch (value, handler cycles), produced
/// by the native engine once per test process (hit_sound.rs does the same).
fn native_punch_latch() -> Vec<(u32, u8, u32)> {
    static LATCH: std::sync::OnceLock<Vec<(u32, u8, u32)>> = std::sync::OnceLock::new();
    LATCH
        .get_or_init(|| {
            let assets = sf_video::romset::load_assets(ROMS, MANIFEST).expect("ROM regions");
            let out = sf_engine::run::ScratchDir::new("sf-audio-effect");
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

/// Run the music CPU over the window on a latch timeline, through `last`.
fn run(log: &RefLog, latch: sf_audio::LatchTimeline, last: u32) -> MusicRun {
    run_music(
        rom(),
        music_ram(FIXTURE),
        log,
        (FIXTURE + 1) as f64 / 60.0,
        latch,
        PHASE,
        sample_at_end_of(last),
    )
}

/// Every reference write of the window in order within one output sample,
/// no untranslated branch, and the hooked output sample-identical. Returns
/// the exact-clock count.
fn check(run: &MusicRun, log: &RefLog, what: &str) -> usize {
    assert!(
        run.unrecovered.is_empty(),
        "{what}: translated paths only: {:04x?}",
        run.unrecovered
    );
    let (s_fix, s_end) = (sample_at_end_of(FIXTURE), sample_at_end_of(LAST));
    let reference: Vec<_> = reference_opm_writes(log)
        .into_iter()
        .filter(|w| w.sample >= s_fix && w.sample < s_end)
        .collect();
    let native: Vec<_> = run
        .writes
        .iter()
        .copied()
        .filter(|w| w.sample < s_end)
        .collect();
    assert_eq!(native.len(), reference.len(), "{what}: every write");
    for (i, (r, n)) in reference.iter().zip(&native).enumerate() {
        assert_eq!(
            (r.reg, r.data),
            (n.reg, n.data),
            "{what}: write #{i} in order"
        );
        assert!(
            r.sample.abs_diff(n.sample) <= 1,
            "{what}: write #{i} within one sample ({} vs {})",
            n.sample,
            r.sample
        );
    }
    let hook =
        read_device_stream(Path::new(HOOK), "ymsnd", OPM_RATE as f64).expect("ymsnd hook stream");
    let n0 = hook.first_sample();
    let mut compared = 0;
    for (i, r) in hook.samples.iter().enumerate() {
        let n = n0 + i as u64;
        if n >= s_fix && n < s_end {
            assert_eq!(run.left[n as usize], *r, "{what}: output sample {n}");
            compared += 1;
        }
    }
    assert!(
        compared > 23_000,
        "{what}: the window's output is compared ({compared} samples)"
    );
    let exact = exact_clock_matches(&native, &reference);
    println!(
        "{what}: {} writes, {exact} on the exact clock, {compared} output samples identical",
        native.len()
    );
    exact
}

/// The music CPU's RAM the translation keeps (0xc000-0xc5cf) against the
/// reference's dump at an idle frame end.
fn assert_ram_matches(run: &MusicRun, frame: u32) {
    let reference = music_ram(frame);
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

/// The hit-spark effect from the native hit event: the native run's latch
/// writes (the sample command 0x84 at 2717, the effect 0x01 at 2718) placed
/// at the vblank handler's cycles plus the interrupt latency drive the
/// music CPU; at the latency band's middle the result is write-for-write
/// and sample-for-sample the reference's, and at both band ends every write
/// still lands within one output sample.
#[test]
fn the_hit_spark_effect_from_the_native_hit_event_plays_like_the_reference() {
    if !present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let frames = native_punch_latch();
    assert!(
        frames.iter().any(|&(f, v, _)| f == 2717 && v == 0x84),
        "the native impact sample command"
    );
    assert!(
        frames.iter().any(|&(f, v, _)| f == 2718 && v == 0x01),
        "the native hit-spark effect command"
    );
    let log = read_taps(&Path::new(HOOK).join("audio_taps.tsv")).expect("tap log");
    let timeline = |latency| {
        latch_timeline(
            &log,
            Some(NativeLatch {
                frames: &frames,
                from: NATIVE_FROM,
                latency,
            }),
        )
    };
    let run = self::run(&log, timeline(IRQ_LATENCY_MID), LAST);
    check(&run, &log, "native latch, latency 92");
    for &frame in &RAM_CHECKS {
        let at = self::run(&log, timeline(IRQ_LATENCY_MID), frame);
        assert_ram_matches(&at, frame);
        assert_eq!(at.ram[0x020], 0, "frame {frame}: effect slot A free again");
        assert_eq!(at.ram[0x500], 0, "frame {frame}: effect slot A empty");
        assert_eq!(at.ram[0x011], 0x08, "frame {frame}: tune 0x28 still loaded");
    }
    for latency in [*IRQ_LATENCY_CYCLES.start(), *IRQ_LATENCY_CYCLES.end()] {
        let run = self::run(&log, timeline(latency), LAST);
        check(&run, &log, &format!("native latch, latency {latency}"));
    }
}

/// The same window driven by the reference's own latch writes: isolates the
/// effect translation from the 68000 side; every write on the exact chip
/// clock.
#[test]
fn the_reference_latch_drives_the_same_effect_on_the_exact_clock() {
    if !present() {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let log = read_taps(&Path::new(HOOK).join("audio_taps.tsv")).expect("tap log");
    let run = self::run(&log, latch_timeline(&log, None), LAST);
    let exact = check(&run, &log, "reference latch");
    let n = run
        .writes
        .iter()
        .filter(|w| w.sample < sample_at_end_of(LAST))
        .count();
    assert!(
        exact + 8 >= n,
        "all but one handler on the exact chip clock ({exact} of {n})"
    );
}

/// Two more windows the captures hold, compared on the YM2151 writes (no
/// hooked stream there), driven by the reference's latch.
const FIGHT_AUDIO: &str = "../../tools/reference/local/captures/fight-audio-a";
const WIN_01_AUDIO: &str = "../../tools/reference/local/captures/win-01-audio";

fn run_window(capture: &str, fixture: u32, last: u32, phase: u64) -> (MusicRun, RefLog) {
    let log = read_taps(&Path::new(capture).join("audio_taps.tsv")).expect("tap log");
    let ram = std::fs::read(Path::new(capture).join(format!("ram_{fixture:06}_music.bin")))
        .expect("music RAM dump");
    let run = run_music(
        rom(),
        ram,
        &log,
        (fixture + 1) as f64 / 60.0,
        latch_timeline(&log, None),
        phase,
        sample_at_end_of(last),
    );
    (run, log)
}

/// Every reference write of `fixture..=last` in order; the sample lag and
/// the exact-clock count returned.
fn check_writes(run: &MusicRun, log: &RefLog, fixture: u32, last: u32) -> (u64, usize, usize) {
    assert!(
        run.unrecovered.is_empty(),
        "translated paths only: {:04x?}",
        run.unrecovered
    );
    let (s_fix, s_end) = (sample_at_end_of(fixture), sample_at_end_of(last));
    let reference: Vec<_> = reference_opm_writes(log)
        .into_iter()
        .filter(|w| w.sample >= s_fix && w.sample < s_end)
        .collect();
    let native: Vec<_> = run
        .writes
        .iter()
        .copied()
        .filter(|w| w.sample < s_end)
        .collect();
    assert_eq!(native.len(), reference.len(), "every write");
    let mut lag = 0;
    for (i, (r, n)) in reference.iter().zip(&native).enumerate() {
        assert_eq!((r.reg, r.data), (n.reg, n.data), "write #{i} in order");
        lag = lag.max(r.sample.abs_diff(n.sample));
    }
    (lag, exact_clock_matches(&native, &reference), native.len())
}

/// The fixed fight, frames 2460..=2700: effect 0x03 (channels 6 and 7,
/// priority 3) at 2535 over tune 0x28; the landing effect 0x19 (priority 1)
/// at 2545 rejected while 0x03 plays; the hit spark 0x01 at 2620.
#[test]
fn a_priority_3_effect_a_rejected_landing_and_a_hit_spark_in_the_fixed_fight() {
    if ![FIGHT_AUDIO, ROMS].iter().all(|p| Path::new(p).is_dir()) {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let (run, log) = run_window(FIGHT_AUDIO, 2460, 2700, 7);
    let (lag, exact, n) = check_writes(&run, &log, 2460, 2700);
    println!("fixed fight 2460..=2700: {n} writes, {exact} on the exact clock, largest lag {lag} samples");
    assert!(lag <= 1, "largest lag {lag} samples");
    assert!(
        exact + 8 >= n,
        "all but one handler on the exact chip clock ({exact} of {n})"
    );
    // While 0x03 plays (priority 3 in slot A) the 0x19 at 2545 is ignored:
    // the slot's priority byte stays 3 through 2546.
    let (at, _) = run_window(FIGHT_AUDIO, 2460, 2546, 7);
    assert_eq!(
        at.ram[0x020], 3,
        "slot A holds the priority-3 effect; the landing was rejected"
    );
}

/// sf-win-01, frames 3179..=3292: the continue countdown's tally effect 0x0a
/// restarted every three frames (the equal-priority restart path, the same
/// mask: no hand-back), about 36 times.
#[test]
fn rapid_restarts_of_the_tally_effect_play_like_the_reference() {
    if ![WIN_01_AUDIO, ROMS].iter().all(|p| Path::new(p).is_dir()) {
        eprintln!("skipping: local reference captures/ROMs not present");
        return;
    }
    let (run, log) = run_window(WIN_01_AUDIO, 3179, 3292, 3);
    let (lag, exact, n) = check_writes(&run, &log, 3179, 3292);
    println!(
        "sf-win-01 3179..=3292: {n} writes, {exact} on the exact clock, largest lag {lag} samples"
    );
    assert!(lag <= 1, "largest lag {lag} samples");
    assert!(
        exact + 8 >= n,
        "all but one handler on the exact chip clock ({exact} of {n})"
    );
    let reference = std::fs::read(Path::new(WIN_01_AUDIO).join("ram_003292_music.bin")).unwrap();
    let differing = (0..0x5d0).filter(|&i| run.ram[i] != reference[i]).count();
    assert_eq!(differing, 0, "music RAM at the end of frame 3292");
}
