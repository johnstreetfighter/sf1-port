//! Original, non-Capcom fixtures for explicit hardware-clock score spans.
use sf_audio::{
    check::{music_ram_at_frame_ends, run_music_from_power_on, MusicRun},
    opm::OPM_RATE,
    score,
    tunes::ExtendedTunes,
    LatchTimeline,
};
use std::path::Path;
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const CLOCK: f64 = 3_579_545.0;

fn rom() -> Option<Vec<u8>> {
    Path::new(ROMS)
        .is_dir()
        .then(|| sf_video::romset::load_region(ROMS, MANIFEST, "audiocpu").unwrap())
}
fn program(rom: &[u8], body: &str) -> Vec<u8> {
    let mut table = ExtendedTunes::new();
    score::convert(&format!("command 0x41\nunit 1\n{body}"), rom)
        .unwrap()
        .add_to(&mut table)
        .unwrap();
    table.program(rom).unwrap()
}
fn timeline(script: &[(f64, u8)]) -> LatchTimeline {
    LatchTimeline::new(
        script
            .iter()
            .flat_map(|&(t, c)| [(t, c), (t + 1.0 / 60.0, 0xff)])
            .collect(),
    )
}
fn play(program: &[u8], script: &[(f64, u8)], seconds: f64) -> MusicRun {
    let run = run_music_from_power_on(
        program.to_vec(),
        timeline(script),
        (seconds * OPM_RATE as f64) as u64,
    );
    assert!(run.unrecovered.is_empty(), "{:?}", run.unrecovered);
    run
}
fn attacks(run: &MusicRun, channel: u8) -> Vec<(f64, [u8; 256])> {
    let mut state = [0; 256];
    let mut out = Vec::new();
    for w in &run.writes {
        state[w.reg as usize] = w.data;
        if w.reg == 8
            && w.data & 7 == channel
            && w.data & 0x78 != 0
            && w.sample as f64 / OPM_RATE as f64 > 0.45
        {
            out.push((w.sample as f64 / OPM_RATE as f64, state));
        }
    }
    out
}

#[test]
fn explicit_pitch_and_tone_are_applied_on_every_attack_without_changing_envelopes() {
    let Some(rom) = rom() else { return };
    let body = |voice: &str| {
        format!(
            "channel 0 voice {voice} forever {{ raw 0x4e 236 4000 2000 raw 0x52 128 4000 2000 }}"
        )
    };
    let base = play(&program(&rom, &body("16 0 0 native")), &[(0.5, 0x41)], 3.0);
    let toned = play(&program(&rom, &body("16 4 12 5")), &[(0.5, 0x41)], 3.0);
    let a = attacks(&base, 0);
    let b = attacks(&toned, 0);
    assert_eq!(a.len(), b.len());
    assert!(a.len() > 6);
    for (i, ((_, native), (_, gentle))) in a.iter().zip(&b).enumerate() {
        assert_eq!(gentle[0x28], if i % 2 == 0 { 0x4e } else { 0x52 });
        assert_eq!(gentle[0x30], if i % 2 == 0 { 236 } else { 128 });
        for reg in (0x20..=0xf8).step_by(8) {
            let expected = match reg {
                0x20 => (native[reg] & !0x38) | (5 << 3),
                0x60 | 0x68 | 0x70 => native[reg].saturating_add(12).min(127),
                0x78 => native[reg].saturating_add(4).min(127),
                _ => native[reg],
            };
            assert_eq!(gentle[reg], expected, "attack{i} reg{reg:02x}");
        }
    }
    // Each ordinary attack reloads the LFO frequency, not only a voice change.
    let frequency_writes = toned
        .writes
        .iter()
        .filter(|w| w.reg == 0x18 && w.sample as f64 / OPM_RATE as f64 > 0.5)
        .count();
    assert!(frequency_writes >= b.len());
}

#[test]
fn explicit_spans_keep_wall_clock_phase_with_all_eight_channels_busy() {
    let Some(rom) = rom() else { return };
    let body = (0..8)
        .map(|ch| format!("channel {ch} voice 16 4 12 5 forever {{ raw 0x4e 236 4000 2000 }}\n"))
        .collect::<String>();
    let run = play(&program(&rom, &body), &[(0.5, 0x41)], 8.0);
    let period = 4000.0 * 256.0 / CLOCK;
    let mut maximum_error = 0.0f64;
    for ch in 0..8 {
        let ons = attacks(&run, ch);
        assert!(ons.len() > 20);
        for (i, (at, _)) in ons.iter().enumerate() {
            maximum_error = maximum_error.max((at - ons[0].0 - i as f64 * period).abs());
        }
    }
    eprintln!("eight-channel explicit-span maximum phase deviation: {maximum_error:.6}s");
    assert!(
        maximum_error < 0.004,
        "one native timer interval, no cumulative lost-tick drift: {maximum_error}s"
    );
}

#[test]
fn held_pitch_changes_and_gates_do_not_retrigger_notes() {
    let Some(rom) = rom() else { return };
    let p=program(&rom,"channel 0 voice 16 4 12 5 forever { raw 0x4e 16 2000 6000 pitch 0x50 240 wait 2000 off wait 2000 }");
    let run = play(&p, &[(0.5, 0x41)], 2.0);
    let ons = attacks(&run, 0);
    assert!(ons.len() >= 3);
    assert!(ons.iter().all(|(_, s)| s[0x28] == 0x4e && s[0x30] == 16));
    assert!(run.writes.iter().any(|w| w.reg == 0x30 && w.data == 240));
    let mut keyed = false;
    let mut effective_offs = 0;
    for w in &run.writes {
        if w.reg == 8 && w.data & 7 == 0 {
            if w.data & 0x78 != 0 {
                keyed = true;
            } else if keyed {
                effective_offs += 1;
                keyed = false;
            }
        }
    }
    assert!(effective_offs >= 3);
}

#[test]
fn raw_voice_state_survives_native_save_resume_and_expired_gates_stay_off() {
    let Some(rom) = rom() else { return };
    let p=program(&rom,"channel 0 voice 16 4 12 5 forever { raw 0x4e 236 60000 0 raw 0x52 128 60000 0 } channel 1 voice 124 21 12 0 forever { raw 0x4e 64 60000 1000 wait 60000 }");
    let script = [(0.5, 0x41), (1.5, 0x36)];
    let run = play(&p, &script, 36.0);
    assert!(run.observations.saved_pops >= 1);
    let ons = attacks(&run, 0);
    let resumed = ons
        .iter()
        .find(|(t, s)| *t > 1.5 && s[0x30] == 236 && s[0x20] & 0x3f == 0x2a)
        .expect("raw tune resumes");
    assert_eq!(
        (resumed.1[0x28], resumed.1[0x30], resumed.1[0x20] & 0x3f),
        (0x4e, 236, 0x2a)
    );
    let next = ons
        .iter()
        .find(|(t, s)| *t > resumed.0 && s[0x28] == 0x52 && s[0x30] == 128)
        .expect("paused duration resumes");
    assert!(
        (3.0..3.4).contains(&(next.0 - resumed.0)),
        "the interlude must not consume a paused raw duration"
    );
    assert!(
        !attacks(&run, 1)
            .iter()
            .any(|(t, _)| (*t - resumed.0).abs() < 0.1),
        "expired raw gate must not be re-keyed on hand-back"
    );
    let frames = [120, 1900, 2050];
    let (states, unrecovered) = music_ram_at_frame_ends(p, timeline(&script), &frames);
    assert!(unrecovered.is_empty());
    let saved = &states[0].2;
    assert_eq!(&saved[0x30f..0x312], &[0xa7, 0x4e, 236]);
    assert_eq!(&saved[0x313..0x316], &[4, 12, 5]);
    let stop = play(
        &program(
            &rom,
            "channel 0 voice 16 4 12 5 forever { raw 0x4e 236 60000 0 }",
        ),
        &[(0.5, 0x41), (1.0, 0x3f)],
        2.0,
    );
    assert!(stop.left[(1.5 * OPM_RATE as f64) as usize..]
        .iter()
        .all(|v| v.abs() < 0.0001));
}

#[test]
fn saved_raw_channel_seven_advances_silently_and_restores_its_own_voice() {
    let Some(rom) = rom() else { return };
    let mut table = ExtendedTunes::new();
    let lower="command 0x41 unit 1 channel 7 voice 16 13 0 native forever { raw 0x4e 236 20000 0 raw 0x52 128 20000 0 }";
    score::convert(lower, &rom)
        .unwrap()
        .add_to(&mut table)
        .unwrap();
    let mut higher = score::convert(
        "command 0x42 priority 15 unit 1 channel 0 voice 1 0 0 native raw 0x22 0 30000 1000 end",
        &rom,
    )
    .unwrap();
    higher.tune.header |= 0x40; // Native header bit: saved tune advances silently.
    higher.add_to(&mut table).unwrap();
    let p = table.program(&rom).unwrap();
    let script = [(0.5, 0x41), (1.0, 0x42)];
    let run = play(&p, &script, 6.0);
    let alone = play(&p, &script[..1], 6.0);
    assert!(run.observations.saved_pops >= 1);
    let ons = attacks(&run, 7);
    let original = attacks(&alone, 7);
    assert!(
        !ons.iter().any(|(t, _)| (1.05..3.1).contains(t)),
        "silent saved channel must not write key-ons"
    );
    let resumed = ons
        .iter()
        .find(|(t, _)| *t > 3.1)
        .expect("channel seven resumes");
    let sounding = original.iter().rev().find(|(t, _)| *t < resumed.0).unwrap();
    assert_eq!(
        (resumed.1[0x2f], resumed.1[0x37], resumed.1[0x27]),
        (sounding.1[0x2f], sounding.1[0x37], sounding.1[0x27])
    );
    assert_eq!(
        resumed.1[0x37], 128,
        "the silently advanced second raw pitch is restored, not the paused first one"
    );
    let next = ons.iter().find(|(t, _)| *t > resumed.0 + 0.01).unwrap();
    let expected = original.iter().find(|(t, _)| *t > resumed.0).unwrap();
    assert!(
        (next.0 - expected.0).abs() < 0.015,
        "silent advance must retain wall-clock phase"
    );
}

#[test]
fn effect_hand_back_restores_raw_pitch_and_keeps_cached_music_lfo_and_rests() {
    let Some(rom) = rom() else { return };
    let body = "channel 0 voice 1 0 0 native forever { raw 0x22 0 4000 1000 }
        channel 6 voice 16 13 0 native raw 0x4e 128 2000 1000 forever { wait 2000 }
        channel 7 voice 16 13 0 native raw 0x4e 236 2000 0 forever { pitch 0x4e 236 wait 2000 }";
    let p = program(&rom, body);
    // Native effect 0x01 borrows channels6/7; ongoing raw pitch writes must
    // respect that ownership, and its end hands the live tune back.
    let script = [(0.5, 0x41), (1.0, 0x01)];
    let run = play(&p, &script, 3.0);
    assert!(run.observations.owned_skips > 0);
    assert!(
        attacks(&run, 7)
            .iter()
            .any(|(t, s)| *t > 1.0 && s[0x37] == 236),
        "held raw channel is restored"
    );
    assert!(
        !attacks(&run, 6)
            .iter()
            .any(|(t, s)| *t > 1.0 && s[0x36] == 128),
        "expired raw gate stays off after effect"
    );
    let frames = [55, 90, 120, 150];
    let (states, unrecovered) = music_ram_at_frame_ends(p, timeline(&script), &frames);
    assert!(unrecovered.is_empty());
    for (_, _, ram) in states {
        assert_eq!(
            ram[0x16],
            rom[0x6b00 + 41 + 3],
            "hand-back must preserve the bass's latest global LFO cache"
        );
    }
}
