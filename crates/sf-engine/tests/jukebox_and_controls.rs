//! Audio jukebox, sound test, and the three control schemes (`#100`, `O9`).

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::jukebox::Jukebox;
use sf_engine::session_io::{
    host_smoke_line, scaled_damage, sound_test_catalog, ControlScheme, ControlSession, FramePads,
    Playback, SoundEntry, SoundTest, SpecialKind,
};

fn hold_punch(frames: u16) -> [bool; 6] {
    let mut session = ControlSession::new(ControlScheme::Pneumatic, ControlScheme::Classic, false);
    let mut pads = FramePads::default();
    pads.punch = true;
    let mut last = [false; 6];
    for _ in 0..frames {
        last = session.step(&pads, &FramePads::default()).0.buttons;
    }
    last
}

#[test]
fn pneumatic_hold_duration_picks_light_medium_and_heavy() {
    assert_eq!(hold_punch(1), [true, false, false, false, false, false]);
    assert_eq!(hold_punch(2), [true, false, false, false, false, false]);
    assert_eq!(hold_punch(3), [false, true, false, false, false, false]);
    assert_eq!(hold_punch(4), [false, true, false, false, false, false]);
    assert_eq!(hold_punch(5), [false, false, true, false, false, false]);
    assert_eq!(hold_punch(8), [false, false, true, false, false, false]);

    let mut session = ControlSession::new(ControlScheme::Pneumatic, ControlScheme::Classic, false);
    let mut pads = FramePads::default();
    pads.kick = true;
    pads.kick_pressure = Some(0.2);
    assert_eq!(
        session.step(&pads, &FramePads::default()).0.buttons,
        [false, false, false, true, false, false]
    );
    pads.kick_pressure = Some(0.5);
    assert_eq!(
        session.step(&pads, &FramePads::default()).0.buttons,
        [false, false, false, false, true, false]
    );
    pads.kick_pressure = Some(0.9);
    assert_eq!(
        session.step(&pads, &FramePads::default()).0.buttons,
        [false, false, false, false, false, true]
    );
}

#[test]
fn modern_neutral_specials_wait_two_frames_and_lose_twenty_percent() {
    let mut penalized = ControlSession::new(ControlScheme::Modern, ControlScheme::Classic, true);
    let mut pads = FramePads::default();
    pads.special = true;
    let first = penalized.step(&pads, &FramePads::default()).0;
    let second = penalized.step(&pads, &FramePads::default()).0;
    let third = penalized.step(&pads, &FramePads::default()).0;
    assert!(first.special.is_none());
    assert!(second.special.is_none());
    let commit = third
        .special
        .expect("neutral special after two startup frames");
    assert_eq!(commit.kind, SpecialKind::Hadoken);
    assert!(commit.penalized);
    assert_eq!(commit.startup_extra, 2);
    assert_eq!(scaled_damage(100, commit.penalized), 80);
    assert_eq!(scaled_damage(100, false), 100);

    let mut open = ControlSession::new(ControlScheme::Modern, ControlScheme::Classic, false);
    let immediate = open.step(&pads, &FramePads::default()).0.special.unwrap();
    assert_eq!(immediate.kind, SpecialKind::Hadoken);
    assert!(!immediate.penalized);
    assert_eq!(immediate.startup_extra, 0);
    assert_eq!(scaled_damage(50, immediate.penalized), 50);

    pads.down = true;
    let mut directional = ControlSession::new(ControlScheme::Modern, ControlScheme::Classic, true);
    let dp = directional
        .step(&pads, &FramePads::default())
        .0
        .special
        .unwrap();
    assert_eq!(dp.kind, SpecialKind::Shoryuken);
    assert!(!dp.penalized);
    assert_eq!(scaled_damage(40, dp.penalized), 40);

    pads.down = false;
    pads.heavy = true;
    let mut super_move = ControlSession::new(ControlScheme::Modern, ControlScheme::Classic, true);
    assert!(super_move
        .step(&pads, &FramePads::default())
        .0
        .special
        .is_none());
    assert!(super_move
        .step(&pads, &FramePads::default())
        .0
        .special
        .is_none());
    let super_commit = super_move
        .step(&pads, &FramePads::default())
        .0
        .special
        .unwrap();
    assert_eq!(super_commit.kind, SpecialKind::Super);
    assert!(super_commit.penalized);
    assert_eq!(scaled_damage(25, true), 20);

    pads.heavy = false;
    pads.manual_motion = true;
    pads.forward = true;
    let mut manual = ControlSession::new(ControlScheme::Modern, ControlScheme::Classic, true);
    assert!(manual
        .step(&pads, &FramePads::default())
        .0
        .special
        .is_none());
    assert_eq!(scaled_damage(100, false), 100);
}

#[test]
fn players_pick_schemes_independently_and_competitive_defaults_the_penalty() {
    let mut session = ControlSession::new(ControlScheme::Classic, ControlScheme::Modern, true);
    let mut p1 = FramePads::default();
    p1.classic = [false, true, false, false, false, false];
    let mut p2 = FramePads::default();
    p2.light = true;
    p2.assist = true;
    let (a, b) = session.step(&p1, &p2);
    assert_eq!(a.buttons, p1.classic);
    assert!(a.special.is_none());
    assert_eq!(b.buttons, [false, false, false, true, false, false]);

    let faithful = MatchConfig::faithful();
    assert_eq!(faithful.p1_scheme, ControlScheme::Classic);
    assert!(!faithful.modern_control_damage_penalty);
    let competitive = MatchConfig::from_preset(RulesetPreset::ClassicCompetitive);
    assert!(competitive.modern_control_damage_penalty);
    let text = competitive.to_toml();
    let parsed = MatchConfig::from_toml(&text).unwrap();
    assert_eq!(parsed.p1_scheme, ControlScheme::Classic);
    assert!(parsed.modern_control_damage_penalty);
    let mut custom = parsed;
    custom.p2_scheme = ControlScheme::Pneumatic;
    let again = MatchConfig::from_toml(&custom.to_toml()).unwrap();
    assert_eq!(again.p2_scheme, ControlScheme::Pneumatic);
    assert_eq!(again.preset, RulesetPreset::ClassicCompetitive);
}

#[test]
fn the_sound_test_follows_the_jukebox_and_covers_every_latch() {
    let mut jukebox = Jukebox::default();
    assert!(jukebox.resolve(None, None, None).is_none());
    jukebox.set_stage("retsu", "retsu.ogg");
    jukebox.set_fighter("Ken", "ken.flac");

    let catalog = sound_test_catalog();
    assert_eq!(
        catalog
            .iter()
            .filter(|e| matches!(e, SoundEntry::Fm { .. }))
            .count(),
        40
    );
    assert_eq!(
        catalog
            .iter()
            .filter(|e| matches!(e, SoundEntry::Music { .. }))
            .count(),
        5
    );
    assert_eq!(
        catalog
            .iter()
            .filter(|e| matches!(e, SoundEntry::Adpcm { .. }))
            .count(),
        64
    );
    let menu = SoundTest::open();
    assert_eq!(menu.len(), catalog.len());
    assert_eq!(
        menu.audition(&Jukebox::default(), None, None),
        Playback::Native { latch: 0x00 }
    );

    let mut music = SoundTest::open();
    music.move_by(40);
    assert!(matches!(music.selected(), SoundEntry::Music { cmd: 0x28 }));
    assert_eq!(
        music.audition(&jukebox, Some("retsu"), Some("Ken")),
        Playback::Track { file: "ken.flac".into() },
        "music follows the fighter's track"
    );
    assert_eq!(
        music.audition(&jukebox, Some("retsu"), Some("Ryu")),
        Playback::Track { file: "retsu.ogg".into() }
    );
    let native = music.audition(&Jukebox::default(), None, None);
    assert_eq!(native, Playback::Native { latch: 0x28 });
}

#[test]
fn host_smoke_line_switches_schemes_without_leaving_faithful_audio() {
    let (ok, line) = host_smoke_line();
    assert!(ok, "{line}");
    assert!(line.contains("native when no track is set"));
}
