//! Ticket #224 (EH8, ADR 0011): the announcer's own MSM5205 at the audio
//! mix's seam, proven with test clips before any voice is generated.
//!
//! - His lines play on his chip at 4 kHz: the chip's output is each clip's
//!   decode, nibble for nibble.
//! - One line at a time: a higher line cuts off a lower one, an equal or
//!   lower one waits, and a line that waited more than 30 frames is dropped.
//! - The voice pack round-trips through its folder, and a pack that is not
//!   4 kHz 4-bit MSM5205 ADPCM, or does not match its manifest, is refused.
//! - With the ROM set and the fixed fight's reference audio run: the samples
//!   CPU's port writes and the board's two chips' output are identical with
//!   him on and off, volume 0 is silent, and a missing pack is quiet.
//!
//! The clips are tones made here and encoded with the asset pack's 4 kHz
//! encoder. The ROM-gated tests skip on a clean checkout.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use sf_audio::announcer::{find_pack, pack_folders, Announcer, VoiceStore, ANNOUNCER_HZ, WAIT_TICKS};
use sf_audio::check::{latch_timeline, read_taps};
use sf_audio::mix::GameSound;
use sf_audio::msm5205::Msm5205;
use sf_audio::FRAME_HZ;
use sf_video::announcer::AnnouncerLine;
use sf_video::asset_pack::{encode_adpcm, AdpcmClip};

const RUN_A: &str = "../../tools/reference/local/captures/fight-audio-a";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

/// A test clip: `nibbles` long, a tone of its own.
fn clip(nibbles: usize, hz: f64) -> AdpcmClip {
    let pcm: Vec<i16> = (0..nibbles)
        .map(|n| (1500.0 * (2.0 * std::f64::consts::PI * hz * n as f64 / ANNOUNCER_HZ as f64).sin()) as i16)
        .collect();
    let clip = encode_adpcm(&pcm).unwrap();
    assert_eq!(clip.sample_rate, 4_000);
    clip
}

fn store(lines: &[(AnnouncerLine, usize)]) -> Arc<VoiceStore> {
    let clips = lines.iter().enumerate().map(|(i, &(line, n))| (line, clip(n, 300.0 + 120.0 * i as f64)));
    Arc::new(VoiceStore::from_clips(clips).unwrap())
}

/// A clip's levels as the chip decodes it from reset.
fn decode(nibbles: &[u8]) -> Vec<f32> {
    let mut chip = Msm5205::new();
    chip.write(0x80);
    nibbles
        .iter()
        .map(|n| {
            chip.write(0x40 | n);
            chip.output()
        })
        .collect()
}

/// The chip's level at every tick `0..ticks`, and the line on it.
fn render(a: &mut Announcer, ticks: u64) -> Vec<(f32, Option<AnnouncerLine>)> {
    (0..ticks)
        .map(|k| {
            let v = a.output_at(k as f64 / ANNOUNCER_HZ as f64);
            (v, a.playing())
        })
        .collect()
}

/// The tick a frame's start reaches the chip on.
fn frame_tick(frame: u32) -> u64 {
    (frame as u64 * ANNOUNCER_HZ as u64).div_ceil(FRAME_HZ as u64)
}

/// `out` from `start` holds `line`'s clip, decoded from reset.
fn assert_plays(out: &[(f32, Option<AnnouncerLine>)], store: &VoiceStore, line: AnnouncerLine, start: u64, len: usize) {
    let want = decode(&store.line(line).unwrap()[..len]);
    for (i, w) in want.iter().enumerate() {
        let (v, on) = out[start as usize + i];
        assert_eq!(on, Some(line), "tick {}: {line:?} on the chip", start as usize + i);
        assert_eq!(v, *w, "tick {}: {line:?}'s nibble {i}", start as usize + i);
    }
}

#[test]
fn a_line_plays_its_clip_on_his_chip_at_4_khz_and_then_he_is_silent() {
    let voice = store(&[(AnnouncerLine::Super, 1200)]);
    let mut a = Announcer::new(voice.clone());
    a.call_at_frame(3, &[AnnouncerLine::Super]);
    let out = render(&mut a, 3000);
    let start = frame_tick(3);
    assert!(out[..start as usize].iter().all(|&(v, on)| v == 0.0 && on.is_none()), "silent before");
    assert_plays(&out, &voice, AnnouncerLine::Super, start, 1200);
    assert!(out[start as usize + 1200..].iter().all(|&(v, on)| v == 0.0 && on.is_none()), "silent after");
    assert!(out.iter().any(|&(v, _)| v.abs() > 0.2), "the tone is loud enough to hear");
}

#[test]
fn a_higher_line_cuts_off_a_lower_one_which_never_resumes() {
    let voice = store(&[(AnnouncerLine::Burnout, 3000), (AnnouncerLine::Super, 900)]);
    let mut a = Announcer::new(voice.clone());
    a.call_at_frame(0, &[AnnouncerLine::Burnout]);
    a.call_at_frame(6, &[AnnouncerLine::Super]);
    let out = render(&mut a, 4000);
    let cut = frame_tick(6);
    assert_plays(&out, &voice, AnnouncerLine::Burnout, 0, cut as usize);
    assert_plays(&out, &voice, AnnouncerLine::Super, cut, 900);
    assert!(out[(cut + 900) as usize..].iter().all(|&(v, on)| v == 0.0 && on.is_none()), "Burnout is not resumed");
}

#[test]
fn equal_and_lower_lines_wait_and_the_highest_waiting_goes_next() {
    let voice = store(&[
        (AnnouncerLine::Counter, 800),
        (AnnouncerLine::Reversal, 600),
        (AnnouncerLine::CrushCounter, 700),
    ]);
    let mut a = Announcer::new(voice.clone());
    a.call_at_frame(0, &[AnnouncerLine::Counter]);
    a.call_at_frame(1, &[AnnouncerLine::Reversal]);
    a.call_at_frame(2, &[AnnouncerLine::CrushCounter]);
    let out = render(&mut a, 2600);
    // Neither cuts the counter; the equal one goes before the older lower one.
    assert_plays(&out, &voice, AnnouncerLine::Counter, 0, 800);
    assert_plays(&out, &voice, AnnouncerLine::CrushCounter, 800, 700);
    assert_plays(&out, &voice, AnnouncerLine::Reversal, 1500, 600);
    assert!(out[2100..].iter().all(|&(v, _)| v == 0.0));
    // Equal lines on one frame: the first plays, the second after it.
    let voice = store(&[(AnnouncerLine::Counter, 400), (AnnouncerLine::PunishCounter, 400)]);
    let mut a = Announcer::new(voice.clone());
    a.call_at_frame(0, &[AnnouncerLine::PunishCounter, AnnouncerLine::Counter]);
    let out = render(&mut a, 900);
    assert_plays(&out, &voice, AnnouncerLine::PunishCounter, 0, 400);
    assert_plays(&out, &voice, AnnouncerLine::Counter, 400, 400);
}

#[test]
fn a_line_that_waited_more_than_half_a_second_is_dropped() {
    assert_eq!(WAIT_TICKS, 2000, "30 frames at 4 kHz");
    let asked = frame_tick(1);
    for (super_len, called) in [(asked + WAIT_TICKS, true), (asked + WAIT_TICKS + 1, false)] {
        let voice = store(&[(AnnouncerLine::Super, super_len as usize), (AnnouncerLine::Counter, 300)]);
        let mut a = Announcer::new(voice.clone());
        a.call_at_frame(0, &[AnnouncerLine::Super]);
        a.call_at_frame(1, &[AnnouncerLine::Counter]);
        let out = render(&mut a, super_len + 600);
        assert_plays(&out, &voice, AnnouncerLine::Super, 0, super_len as usize);
        if called {
            assert_plays(&out, &voice, AnnouncerLine::Counter, super_len, 300);
        } else {
            assert!(
                out[super_len as usize..].iter().all(|&(v, on)| v == 0.0 && on.is_none()),
                "a counter that waited {} ticks is dropped",
                super_len - asked
            );
        }
    }
}

#[test]
fn a_scheduled_stop_and_voice_change_keep_the_preceding_audio() {
    let ringside = Arc::new(VoiceStore::from_clips([
        (AnnouncerLine::Fight, clip(3000, 300.0)),
        (AnnouncerLine::Counter, clip(300, 650.0)),
    ]).unwrap());
    let japanese = Arc::new(VoiceStore::from_clips([
        (AnnouncerLine::Fight, clip(100, 500.0)),
    ]).unwrap());
    let mut a = Announcer::new(ringside.clone());
    a.call_at_frame(0, &[AnnouncerLine::Fight, AnnouncerLine::Counter]);
    a.stop_at_frame(5);
    a.set_store_at_frame(5, japanese.clone());
    a.call_at_frame(5, &[AnnouncerLine::Fight]);
    let out = render(&mut a, 600);

    let switched = frame_tick(5);
    assert_plays(&out, &ringside, AnnouncerLine::Fight, 0, switched as usize);
    assert_plays(&out, &japanese, AnnouncerLine::Fight, switched, 100);
    assert!(out[switched as usize + 100..].iter().all(|&(v, line)| v == 0.0 && line.is_none()));
}

#[test]
fn stop_silences_the_chip_and_cancels_waiting_and_future_calls() {
    let voice = store(&[(AnnouncerLine::Perfect, 800), (AnnouncerLine::Counter, 200)]);
    let mut a = Announcer::new(voice);
    a.call_at_frame(0, &[AnnouncerLine::Perfect, AnnouncerLine::Counter]);
    a.call_at_frame(3, &[AnnouncerLine::Perfect]);
    a.output_at(0.004);
    assert_eq!(a.playing(), Some(AnnouncerLine::Perfect));

    a.stop();
    assert_eq!(a.playing(), None);
    assert_eq!(a.level(), 0.0);
    assert_eq!(a.output_at(0.1), 0.0);
}

#[test]
fn a_line_the_pack_lacks_is_not_called() {
    let voice = store(&[(AnnouncerLine::Combo { hits: 5 }, 300)]);
    let mut a = Announcer::new(voice.clone());
    // Twelve hits beyond the pack's cap: he never says a wrong number.
    a.call_at_frame(0, &[AnnouncerLine::Combo { hits: 12 }, AnnouncerLine::Perfect]);
    assert!(render(&mut a, 600).iter().all(|&(v, on)| v == 0.0 && on.is_none()));
    a.call_at_frame(10, &[AnnouncerLine::Combo { hits: 5 }]);
    let out = render(&mut a, 1200);
    assert_plays(&out, &voice, AnnouncerLine::Combo { hits: 5 }, frame_tick(10), 300);
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sf1-port-eh8-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn the_voice_pack_round_trips_and_a_bad_pack_is_refused_whole() {
    let roms = scratch("pack").join("roms");
    let folder = roms.join("sf1-port-voice");
    let mut lines: Vec<(AnnouncerLine, usize)> = AnnouncerLine::WORDS.iter().map(|&l| (l, 401)).collect();
    lines.extend((5..=9).map(|hits| (AnnouncerLine::Combo { hits }, 250)));
    let voice = store(&lines);
    voice.write(&folder).unwrap();
    // Looked for beside the ROM set's folder, as the art pack is.
    let found = find_pack(&pack_folders(&[&roms.join("sf")])).unwrap();
    assert_eq!(found.folder.as_deref(), Some(folder.as_path()));
    assert_eq!(found.len(), voice.len());
    for &(line, _) in &lines {
        assert_eq!(found.line(line), voice.line(line), "{line:?} back, nibble for nibble (odd lengths too)");
    }

    let manifest = folder.join("manifest.json");
    let text = std::fs::read_to_string(&manifest).unwrap();
    // Not 4 kHz: the validator refuses it.
    std::fs::write(&manifest, text.replace("\"sample_rate\": 4000", "\"sample_rate\": 8000")).unwrap();
    let e = VoiceStore::open(&folder).unwrap_err();
    assert!(e.contains("4000 Hz"), "{e}");
    // A damaged line.
    std::fs::write(&manifest, &text).unwrap();
    let vox = folder.join("super.vox");
    let mut bytes = std::fs::read(&vox).unwrap();
    bytes[3] ^= 0x11;
    std::fs::write(&vox, &bytes).unwrap();
    assert!(VoiceStore::open(&folder).unwrap_err().contains("does not match the manifest"));
    // A file that is no line of his.
    voice.write(&folder).unwrap();
    std::fs::write(&manifest, text.replace("\"super\"", "\"hadoken\"")).unwrap();
    assert!(VoiceStore::open(&folder).unwrap_err().contains("not an announcer line"));
    // Missing: no pack.
    assert!(find_pack(&pack_folders(&[&scratch("none").join("roms").join("sf")])).is_err());
    let _ = std::fs::remove_dir_all(roms.parent().unwrap());

    // The store itself takes 4 kHz 4-bit clips only.
    let wide = AdpcmClip { sample_rate: 4_000, nibbles: vec![3, 16] };
    assert!(VoiceStore::from_clips([(AnnouncerLine::Super, wide)]).is_err(), "a nibble wider than 4 bits");
    let fast = AdpcmClip { sample_rate: 8_000, nibbles: vec![3, 4] };
    assert!(VoiceStore::from_clips([(AnnouncerLine::Super, fast)]).is_err(), "8 kHz");
}

// ---- the board's mix, with the ROM set --------------------------------------

fn regions() -> Option<(Vec<u8>, Vec<u8>)> {
    if !Path::new(ROMS).is_dir() || !Path::new(RUN_A).is_dir() {
        eprintln!("skipping: local reference audio run/ROMs not present");
        return None;
    }
    let region = |name| sf_video::romset::load_region(ROMS, MANIFEST, name).unwrap();
    Some((region("audio2"), region("audiocpu")))
}

/// The fixed fight's sound from power-on, on the reference's latch writes.
fn fight_sound(audio2: &[u8], audiocpu: &[u8]) -> GameSound {
    let log = read_taps(&Path::new(RUN_A).join("audio_taps.tsv")).unwrap();
    let mut sound = GameSound::power_on(audio2.to_vec(), audiocpu.to_vec(), latch_timeline(&log, None), f64::INFINITY);
    sound.record_port_writes();
    sound
}

/// The round-2 KO window, where both chips play (fight-samples.md).
const FIRST: u32 = 4740;
const LAST: u32 = 4840;
const RATE: f64 = 8_000.0;

/// Every line, called every few frames over the window, on both of the
/// fight's busiest frames.
fn calls() -> Vec<(u32, Vec<AnnouncerLine>)> {
    let mut lines: Vec<AnnouncerLine> = AnnouncerLine::WORDS.to_vec();
    lines.push(AnnouncerLine::Combo { hits: 6 });
    (FIRST..LAST).step_by(4).zip(lines.iter().cycle()).map(|(f, &l)| (f, vec![l, AnnouncerLine::Combo { hits: 6 }])).collect()
}

fn full_voice() -> Arc<VoiceStore> {
    let mut lines: Vec<(AnnouncerLine, usize)> = AnnouncerLine::WORDS.iter().map(|&l| (l, 900)).collect();
    lines.push((AnnouncerLine::Combo { hits: 6 }, 700));
    store(&lines)
}

#[test]
fn the_samples_cpu_and_the_boards_two_chips_are_identical_with_him_on_and_off() {
    let Some((audio2, audiocpu)) = regions() else { return };
    let mut off = fight_sound(&audio2, &audiocpu);
    let mut on = fight_sound(&audio2, &audiocpu);
    on.set_announcer(Some(Announcer::new(full_voice())));
    for (frame, lines) in calls() {
        on.announce(frame, &lines);
        // Without his chip, a call does nothing.
        off.announce(frame, &lines);
    }
    let t0 = FIRST as f64 / FRAME_HZ as f64;
    assert!(off.skip_to(t0) && on.skip_to(t0));
    let n = ((LAST - FIRST) as f64 / FRAME_HZ as f64 * RATE) as usize;
    let (mut board_active, mut his_active, mut both) = (0, 0, 0);
    for i in 0..n {
        let t = t0 + i as f64 / RATE;
        assert!(on.wait_ready(t) && off.wait_ready(t));
        let plain = off.sample_at(t);
        // Even samples with him at volume 0: the board's mix, bit for bit.
        let mixed = on.sample_at_mix(t, 1.0, 1.0, if i % 2 == 0 { 0.0 } else { 1.0 });
        assert_eq!(on.chip_levels(), off.chip_levels(), "sample {i}: the board's two chips");
        let his = on.announcer().unwrap().level();
        if i % 2 == 0 {
            assert_eq!(mixed, plain, "sample {i}: ANNOUNCER 0 leaves the mix as it is");
        } else {
            for ch in 0..2 {
                if (plain[ch] + his).abs() < 0.999 {
                    assert!((mixed[ch] - (plain[ch] + his)).abs() < 1e-6, "sample {i}: his chip beside the board's");
                }
            }
        }
        let board = off.chip_levels().iter().any(|v| *v != 0.0);
        board_active += usize::from(board);
        his_active += usize::from(his != 0.0);
        both += usize::from(board && his != 0.0);
    }
    let (a, b) = (off.take_port_writes(), on.take_port_writes());
    assert!(a.len() > 10_000, "the samples CPU wrote ({} writes)", a.len());
    assert_eq!(a, b, "the samples CPU's port writes");
    assert!(a.iter().zip(&b).all(|(x, y)| x.t == y.t), "and their times");
    println!("{n} samples: board {board_active}, announcer {his_active}, both at once {both}");
    assert!(board_active > n / 4 && his_active > n / 4 && both > n / 10, "he speaks over a busy board");
}

#[test]
fn volume_0_is_silent_and_a_missing_pack_is_quiet() {
    let Some((audio2, audiocpu)) = regions() else { return };
    // A short stretch from power-on: the board muted, only his chip left.
    let mut muted = fight_sound(&audio2, &audiocpu);
    let mut missing = fight_sound(&audio2, &audiocpu);
    let mut plain = fight_sound(&audio2, &audiocpu);
    muted.set_announcer(Some(Announcer::new(full_voice())));
    let none = find_pack(&pack_folders(&[&scratch("missing").join("roms").join("sf")]));
    assert!(none.is_err(), "no pack");
    missing.set_announcer(none.ok().map(|s| Announcer::new(Arc::new(s))));
    for (frame, lines) in calls() {
        let frame = frame - FIRST + 30;
        muted.announce(frame, &lines);
        missing.announce(frame, &lines);
    }
    let n = (100.0 / FRAME_HZ as f64 * RATE) as usize;
    let t0 = 30.0 / FRAME_HZ as f64;
    let mut heard = 0;
    for i in 0..n {
        let t = t0 + i as f64 / RATE;
        assert!(muted.wait_ready(t) && missing.wait_ready(t) && plain.wait_ready(t));
        assert_eq!(muted.sample_at_mix(t, 0.0, 0.0, 0.0), [0.0, 0.0], "sample {i}: every volume 0 is silence");
        assert_eq!(missing.sample_at(t), plain.sample_at(t), "sample {i}: no pack, no announcer");
        heard += usize::from(muted.announcer().unwrap().level() != 0.0);
    }
    assert!(heard > n / 4, "he was speaking while muted ({heard} of {n})");
}
