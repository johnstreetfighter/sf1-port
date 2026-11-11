//! Ticket #225 (EH9): the voice pack the maintainer picked, beside the ROM set. Every
//! line a fight can ask the announcer for is in it, and each one plays on
//! his chip. Local data: skips without the pack.

use std::path::Path;
use std::sync::Arc;

use sf_audio::announcer::{Announcer, VoiceStore};
use sf_video::announcer::{AnnouncerLine, COMBO_MIN_HITS};

const PACK: &str = "../../tools/reference/local/roms/sf1-port-voice";
/// The longest combo the line list voices (`tools/voice/lines.json`).
const COMBO_CAP: u16 = 15;

fn every_line() -> Vec<AnnouncerLine> {
    let mut lines = vec![
        AnnouncerLine::Super,
        AnnouncerLine::Perfect,
        AnnouncerLine::PerfectParry,
        AnnouncerLine::Counter,
        AnnouncerLine::PunishCounter,
        AnnouncerLine::CrushCounter,
        AnnouncerLine::AlphaCounter,
        AnnouncerLine::Reversal,
        AnnouncerLine::FirstAttack,
        AnnouncerLine::Burnout,
        AnnouncerLine::CustomCombo,
    ];
    lines.extend((COMBO_MIN_HITS..=COMBO_CAP).map(|hits| AnnouncerLine::Combo { hits }));
    lines
}

#[test]
fn every_line_is_in_the_picked_pack_and_plays_on_his_chip() {
    if !Path::new(PACK).is_dir() {
        eprintln!("skipping: voice pack not present");
        return;
    }
    let store = Arc::new(VoiceStore::open(Path::new(PACK)).expect("the pack validates"));
    for line in every_line() {
        let nibbles = store.line(line).unwrap_or_else(|| panic!("{line:?} is missing from the pack"));
        assert!(!nibbles.is_empty(), "{line:?} is empty");
        let mut announcer = Announcer::new(store.clone());
        announcer.call(0.0, line);
        // A quarter of a second at 8 kHz: he is talking, and not silently.
        let loud = (0..2000).map(|i| announcer.output_at(i as f64 / 8000.0).abs()).fold(0.0f32, f32::max);
        assert_eq!(announcer.playing(), Some(line), "{line:?} is playing");
        assert!(loud > 0.01, "{line:?} is audible (peak {loud})");
    }
}
