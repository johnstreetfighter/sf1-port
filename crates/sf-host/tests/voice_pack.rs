//! Ticket #224 (EH8): the voice pack at launch. Beside the ROM set it loads
//! into the announcer's store with a log line; missing, there is no
//! announcer and exactly one log line says why.

use sf_audio::announcer::VoiceStore;
use sf_host::log::Log;
use sf_video::announcer::AnnouncerLine;
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

    let clip = encode_adpcm(&[0, 400, 800, 400, 0, -400]).unwrap();
    VoiceStore::from_clips([(AnnouncerLine::Super, clip)]).unwrap().write(&dir.join("roms").join("sf1-port-voice")).unwrap();
    let path = dir.join("found.log");
    let mut log = Log::create(&path);
    let store = sf_host::voice::load(&[roms.as_path()], &mut log).expect("the pack beside the ROMs");
    assert_eq!(store.len(), 1);
    let lines = voice_lines(&path);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("1 announcer lines"), "{}", lines[0]);
    drop(log);
    let _ = std::fs::remove_dir_all(&dir);
}
