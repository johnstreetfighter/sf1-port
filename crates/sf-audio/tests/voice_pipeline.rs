//! Ticket #225 (EH9): the voice pipeline's codec steps, through the
//! `sf-audio` binary as `tools/voice/pipeline.py` calls it.
//!
//! - `voice-encode` turns a 4 kHz 16-bit take into a `.vox` that unpacks to
//!   nibbles `validate_adpcm` accepts, with the CRC-32 and count it prints,
//!   and writes what the chip plays from it for listening.
//! - `voice-pack` writes the pack from chosen takes with `VoiceStore::write`,
//!   and `VoiceStore::open` loads every line back nibble for nibble.
//! - With the ROM set: `voice-rom-sample 0x94` is the ROM's "FIGHT!", its
//!   1,968 bytes as 3,936 nibbles (docs/reference/adpcm-samples.md).
//!
//! The ROM-gated test skips on a clean checkout.

use std::path::{Path, PathBuf};
use std::process::Command;

use sf_audio::announcer::VoiceStore;
use sf_audio::voice_pipeline::{decode, encode_take, rom_sample, unpack_nibbles};
use sf_video::announcer::AnnouncerLine;
use sf_video::asset_pack::{validate_adpcm, AdpcmClip};

const BIN: &str = env!("CARGO_BIN_EXE_sf-audio");
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sf1-port-eh9-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A 4 kHz mono 16-bit take: a tone with an attack and a decay, `n` long.
fn take(path: &Path, n: usize, hz: f64, rate: u32) {
    let spec = hound::WavSpec { channels: 1, sample_rate: rate, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut w = hound::WavWriter::create(path, spec).unwrap();
    for i in 0..n {
        let env = (i as f64 / 200.0).min(1.0) * (1.0 - i as f64 / n as f64);
        let v = 26000.0 * env * (2.0 * std::f64::consts::PI * hz * i as f64 / rate as f64).sin();
        w.write_sample(v as i16).unwrap();
    }
    w.finalize().unwrap();
}

fn run(args: &[&str]) -> Result<serde_json::Value, String> {
    let out = Command::new(BIN).args(args).output().unwrap();
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into_owned());
    }
    Ok(serde_json::from_slice(&out.stdout).unwrap())
}

fn encode(dir: &Path, name: &str, n: usize, hz: f64) -> (PathBuf, usize) {
    let wav = dir.join(format!("{name}.4k.wav"));
    take(&wav, n, hz, 4000);
    let vox = dir.join(format!("{name}.vox"));
    let heard = dir.join(format!("{name}.heard.wav"));
    let p = |p: &Path| p.to_str().unwrap().to_owned();
    let info = run(&["voice-encode", "--wav", &p(&wav), "--vox", &p(&vox), "--heard", &p(&heard)]).unwrap();
    (vox, info["nibbles"].as_u64().unwrap() as usize)
}

#[test]
fn an_encoded_take_round_trips_through_validate_adpcm() {
    let dir = scratch("encode");
    // An odd count: the last byte's low nibble is padding.
    let (vox, nibbles) = encode(&dir, "super", 2411, 330.0);
    assert_eq!(nibbles, 2411);
    let bytes = std::fs::read(&vox).unwrap();
    assert_eq!(bytes.len(), 1206);
    let info = run(&[
        "voice-encode",
        "--wav",
        dir.join("super.4k.wav").to_str().unwrap(),
        "--vox",
        dir.join("again.vox").to_str().unwrap(),
        "--heard",
        dir.join("again.heard.wav").to_str().unwrap(),
    ])
    .unwrap();
    assert_eq!(info["crc32"], format!("{:08x}", sf_video::romset::crc32(&bytes)), "the printed CRC-32 is the file's");
    assert_eq!(std::fs::read(dir.join("again.vox")).unwrap(), bytes, "encoding is repeatable byte for byte");

    let clip = AdpcmClip { sample_rate: 4000, nibbles: unpack_nibbles(&bytes, nibbles).unwrap() };
    validate_adpcm(&clip).unwrap();

    // The same encoder in-process gives the same nibbles.
    let mut r = hound::WavReader::open(dir.join("super.4k.wav")).unwrap();
    let pcm: Vec<i16> = r.samples::<i16>().map(Result::unwrap).collect();
    assert_eq!(encode_take(&pcm).unwrap().nibbles, clip.nibbles);

    // What the chip plays from it: 48 kHz, each nibble's output held 12
    // samples, and close to the take (the encoder tracks the signal).
    let heard = hound::WavReader::open(dir.join("super.heard.wav")).unwrap();
    assert_eq!(heard.spec().sample_rate, 48_000);
    assert_eq!(heard.len() as usize, nibbles * 12);
    let out = decode(&clip.nibbles);
    let err: f64 = out
        .iter()
        .zip(&pcm)
        .map(|(o, p)| (f64::from(*o) * 4096.0 - f64::from(*p >> 4)).abs())
        .sum::<f64>()
        / out.len() as f64;
    assert!(err < 120.0, "mean error {err} of 2048");
}

#[test]
fn a_take_that_is_not_4khz_mono_16bit_is_refused() {
    let dir = scratch("refuse");
    let wav = dir.join("raw.wav");
    take(&wav, 2400, 330.0, 24_000);
    let err = run(&[
        "voice-encode",
        "--wav",
        wav.to_str().unwrap(),
        "--vox",
        dir.join("x.vox").to_str().unwrap(),
        "--heard",
        dir.join("x.wav").to_str().unwrap(),
    ])
    .unwrap_err();
    assert!(err.contains("4000 Hz"), "{err}");
    assert!(!dir.join("x.vox").exists());
}

#[test]
fn voice_store_loads_the_pack_build_pack_writes() {
    let dir = scratch("pack");
    let (super_vox, super_n) = encode(&dir, "super", 2411, 330.0);
    let (combo_vox, combo_n) = encode(&dir, "combo_15", 3200, 210.0);
    let plan = serde_json::json!({"lines": {
        "super": {"vox": super_vox, "nibbles": super_n},
        "combo_15": {"vox": combo_vox, "nibbles": combo_n},
    }});
    let plan_path = dir.join("plan.json");
    std::fs::write(&plan_path, plan.to_string()).unwrap();
    let pack = dir.join("sf1-port-voice");
    let info = run(&["voice-pack", "--plan", plan_path.to_str().unwrap(), "--out", pack.to_str().unwrap()]).unwrap();
    assert_eq!(info["lines"], 2);

    let store = VoiceStore::open(&pack).unwrap();
    assert_eq!(store.len(), 2);
    for (line, vox, n) in [
        (AnnouncerLine::Super, &super_vox, super_n),
        (AnnouncerLine::Combo { hits: 15 }, &combo_vox, combo_n),
    ] {
        let want = unpack_nibbles(&std::fs::read(vox).unwrap(), n).unwrap();
        assert_eq!(store.line(line), Some(want.as_slice()), "{line:?}");
    }
    assert_eq!(store.line(AnnouncerLine::Perfect), None);

    // A stem that is not one of his lines, or a count the file can't hold, is refused.
    for bad in [
        serde_json::json!({"lines": {"fireball": {"vox": super_vox, "nibbles": super_n}}}),
        serde_json::json!({"lines": {"super": {"vox": super_vox, "nibbles": super_n + 2}}}),
    ] {
        std::fs::write(&plan_path, bad.to_string()).unwrap();
        let out = dir.join("refused");
        assert!(run(&["voice-pack", "--plan", plan_path.to_str().unwrap(), "--out", out.to_str().unwrap()]).is_err());
    }
}

#[test]
fn the_rom_fight_sample_is_its_1968_bytes() {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: the ROM set is not present");
        return;
    }
    let audio2 = sf_video::romset::load_region(ROMS, MANIFEST, "audio2").unwrap();
    // 0x94 "FIGHT!" (channel 0, 1,968 bytes); 0x8c (channel 1, 2,128 bytes).
    let fight = rom_sample(audio2.clone(), 0x94).unwrap();
    assert_eq!(fight.len(), 2 * 1968);
    let mut ringside = VoiceStore::default();
    ringside.add_rom_fight(&audio2).unwrap();
    assert_eq!(ringside.line(AnnouncerLine::Fight), Some(fight.as_slice()));
    assert_eq!(rom_sample(audio2.clone(), 0x8c).unwrap().len(), 2 * 2128);
    assert!(rom_sample(audio2, 0x80).is_err());

    let dir = scratch("fight");
    let out = dir.join("fight.wav");
    let info = run(&[
        "voice-rom-sample",
        "--command",
        "0x94",
        "--out",
        out.to_str().unwrap(),
        "--roms",
        ROMS,
        "--manifest",
        MANIFEST,
    ])
    .unwrap();
    assert_eq!(info["nibbles"], 3936);
    assert_eq!(hound::WavReader::open(&out).unwrap().len(), 3936 * 12);
}
