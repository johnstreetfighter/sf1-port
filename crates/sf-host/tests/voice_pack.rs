//! Ticket #224 (EH8): the voice pack at launch. Beside the ROM set it loads
//! into the announcer's store with a log line; missing, there is no
//! announcer and exactly one log line says why.

use sf_audio::announcer::VoiceStore;
use sf_host::log::Log;
use sf_video::announcer::{AnnouncerLine, AnnouncerVoice};
use sf_video::asset_pack::encode_adpcm;

fn voice_lines(log: &std::path::Path) -> Vec<String> {
    std::fs::read_to_string(log).unwrap().lines().filter(|l| l.contains("] voice: ")).map(str::to_owned).collect()
}

#[test]
fn a_missing_pack_means_no_announcer_and_one_log_line() {
    let dir = std::env::temp_dir().join(format!("sf1-port-eh8-host-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let roms = dir.join("roms").join("sf");
    std::fs::create_dir_all(&roms).unwrap();
    let path = dir.join("missing.log");
    let mut log = Log::create(&path);
    assert!(sf_host::voice::load(&[roms.as_path()], &mut log).is_none());
    let lines = voice_lines(&path);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("no announcer") && lines[0].contains("sf1-port-voice"), "{}", lines[0]);

    let asset_roots = [std::path::PathBuf::from("../../assets")];
    let mut bundled_log = Log::create(&dir.join("bundled.log"));
    let bundled = sf_host::voice::load_with_fallbacks(
        &[roms.as_path()], &asset_roots, &mut bundled_log,
    ).expect("the repository pack loads without a local override");
    assert!(bundled.len() > 1);

    let clip = encode_adpcm(&[0, 400, 800, 400, 0, -400]).unwrap();
    VoiceStore::from_clips([(AnnouncerLine::Super, clip)]).unwrap().write(&dir.join("roms").join("sf1-port-voice")).unwrap();
    let path = dir.join("found.log");
    let mut log = Log::create(&path);
    let store = sf_host::voice::load_with_fallbacks(&[roms.as_path()], &asset_roots, &mut log)
        .expect("the pack beside the ROMs overrides the bundle");
    assert_eq!(store.len(), 1);
    let lines = voice_lines(&path);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("1 announcer lines"), "{}", lines[0]);
    drop(log);
    drop(bundled_log);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn japanese_local_pack_and_bundled_ringside_load_independently_with_auditions() {
    use sf_host::voice::VoicePacks;
    use std::path::{Path, PathBuf};

    const ROMS: &str = "../../tools/reference/local/roms/sf";
    const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return;
    }
    let audio2 = sf_video::romset::load_region(ROMS, MANIFEST, "audio2").unwrap();
    let expected = sf_audio::voice_pipeline::rom_sample(audio2.clone(), 0x94).unwrap();

    let dir = std::env::temp_dir().join(format!("sf1-port-japanese-host-{}", std::process::id()));
    let roms = dir.join("roms").join("sf");
    std::fs::create_dir_all(&roms).unwrap();
    let japanese_folder = roms.with_file_name("sf1-port-voice").join("japanese");
    let clip = encode_adpcm(&[0, 400, 800, 400, 0, -400]).unwrap();
    let mut log = Log::create(&dir.join("voices.log"));
    VoiceStore::from_clips([(AnnouncerLine::Fight, clip.clone())]).unwrap().write(&japanese_folder).unwrap();
    let packs = VoicePacks::load(&[&roms], &[PathBuf::from("../../assets")], &audio2, &mut log);
    let ringside = packs.store(AnnouncerVoice::Ringside);
    assert_eq!(ringside.line(AnnouncerLine::Fight), Some(expected.as_slice()));
    assert!(ringside.line(AnnouncerLine::Super).is_some(), "the shipped Ringside pack also loads");
    let japanese = packs.store(AnnouncerVoice::Japanese);
    assert_eq!(japanese.line(AnnouncerLine::Fight), Some(clip.nibbles.as_slice()));
    assert!(japanese.line(AnnouncerLine::Round1).is_none(), "an unpicked line stays absent");
    drop(log);
    let _ = std::fs::remove_dir_all(&dir);
}
