//! Alpha revisions, the 3-level gauge, and Gen's stances (`#104`, `O13`).

use sf_engine::alpha::{super_name, AlphaEvent, AlphaSession, Stance};
use sf_engine::config::RulesetPreset;
use sf_engine::era::{costumes, families, loadouts, revisions, Costume, EraFamily, Loadout, Revision};
use sf_engine::roster::{CharacterSelectScreen, MenuInput, PlayMode, RosterSettings, TournamentFighter};
use sf_video::asset_pack::{validate_adpcm, validate_fm_patch, validate_tile};
use sf_video::era_hud::{EraGauge, Fill as GaugeFill, GaugeRow, GaugeState};

#[test]
fn alpha_roster_is_the_canonical_cast_and_sfz3_has_isms() {
    for fighter in [
        TournamentFighter::Ryu,
        TournamentFighter::Ken,
        TournamentFighter::Adon,
        TournamentFighter::Sagat,
        TournamentFighter::Birdie,
        TournamentFighter::Gen,
    ] {
        assert_eq!(
            revisions(fighter, EraFamily::SfZ),
            vec![Revision::Sfz1, Revision::Sfz2, Revision::Sfz2A, Revision::Sfz3]
        );
    }
    assert_eq!(revisions(TournamentFighter::Eagle, EraFamily::SfZ), vec![Revision::Sfz3]);
    assert!(revisions(TournamentFighter::Lee, EraFamily::SfZ).is_empty());
    assert!(!families(TournamentFighter::Lee).contains(&EraFamily::SfZ));
    assert_eq!(
        loadouts(Revision::Sfz3),
        vec![Loadout::AIsm, Loadout::VIsm, Loadout::XIsm]
    );
    assert_eq!(
        costumes(TournamentFighter::Birdie, EraFamily::SfZ),
        vec![Costume::Classic1987, Costume::Alpha]
    );

    let mut settings = RosterSettings::for_preset(RulesetPreset::DefinitiveOverhaul);
    settings.mode = PlayMode::Arcade;
    let mut screen = CharacterSelectScreen::open(settings, true);
    screen.apply(MenuInput::Confirm);
    screen.apply(MenuInput::Confirm);
    screen.apply(MenuInput::Right);
    screen.apply(MenuInput::Right);
    screen.apply(MenuInput::Confirm);
    for _ in 0..3 {
        screen.apply(MenuInput::Right);
    }
    screen.apply(MenuInput::Up);
    screen.apply(MenuInput::Confirm);
    screen.apply(MenuInput::Right);
    screen.apply(MenuInput::Confirm);
    assert_eq!(screen.p1_style().revision, Revision::Sfz3);
    assert_eq!(screen.p1_style().loadout, Loadout::VIsm);
    assert_eq!(screen.p1_style().costume, Costume::Alpha);
}

#[test]
fn levels_counters_custom_combos_and_gen_stances() {
    let mut ryu = AlphaSession::new(TournamentFighter::Ryu, Revision::Sfz2, Loadout::AIsm);
    assert!(validate_adpcm(&ryu.voice).is_ok());
    assert!(validate_fm_patch(&ryu.patch).is_ok());
    for tile in &ryu.costume {
        validate_tile(tile).unwrap();
    }
    assert!(!ryu.super_combo(1));
    ryu.land_hit();
    ryu.land_hit();
    ryu.land_hit();
    assert_eq!(ryu.stocks(), 3);
    assert!(ryu.super_combo(1));
    assert!(matches!(ryu.events.last(), Some(AlphaEvent::Super { name: "Shinku Hadoken", level: 1, .. })));
    assert_eq!(ryu.stocks(), 2);
    assert!(ryu.super_combo(2));
    assert_eq!(ryu.stocks(), 0);

    let mut blocked = AlphaSession::new(TournamentFighter::Ken, Revision::Sfz1, Loadout::AIsm);
    blocked.land_hit();
    assert!(!blocked.alpha_counter());
    blocked.suffer_block(8);
    assert!(blocked.alpha_counter());
    assert_eq!(blocked.stocks(), 0);
    assert!(blocked.events.contains(&AlphaEvent::AlphaCounter));

    let mut early = AlphaSession::new(TournamentFighter::Adon, Revision::Sfz1, Loadout::AIsm);
    early.land_hit();
    assert!(!early.custom_combo());
    let mut variable = AlphaSession::new(TournamentFighter::Sagat, Revision::Sfz2A, Loadout::AIsm);
    variable.land_hit();
    assert!(variable.custom_combo());
    assert!(variable.chain("Tiger Knee"));
    for _ in 0..90 {
        variable.tick();
    }
    assert!(!variable.chain("Tiger Knee"));
    let mut vism = AlphaSession::new(TournamentFighter::Birdie, Revision::Sfz3, Loadout::VIsm);
    vism.land_hit();
    assert!(vism.custom_combo());
    assert!(!vism.super_combo(1));

    let mut x = AlphaSession::new(TournamentFighter::Ryu, Revision::Sfz3, Loadout::XIsm);
    x.land_hit();
    x.land_hit();
    x.land_hit();
    assert!(!x.super_combo(1));
    assert!(x.super_combo(3));
    let x_damage = x.events.iter().find_map(|event| match event {
        AlphaEvent::Super { damage, .. } => Some(*damage),
        _ => None,
    });
    let mut a = AlphaSession::new(TournamentFighter::Ryu, Revision::Sfz3, Loadout::AIsm);
    a.stocks_for_test(3);
    assert!(a.super_combo(3));
    let a_damage = a.events.iter().find_map(|event| match event {
        AlphaEvent::Super { damage, .. } => Some(*damage),
        _ => None,
    });
    assert!(x_damage > a_damage);

    let mut gen = AlphaSession::new(TournamentFighter::Gen, Revision::Sfz3, Loadout::AIsm);
    assert!(gen.normal("Mantis Jab"));
    assert!(!gen.normal("Crane Jab"));
    gen.land_hit();
    gen.land_hit();
    gen.land_hit();
    assert_eq!(super_name(TournamentFighter::Gen, Stance::Mantis, 3), Some("Shitenshuu"));
    assert!(gen.super_combo(3));
    assert!(gen.switch_stance(false));
    assert_eq!(gen.events.last(), Some(&AlphaEvent::Stance(Stance::Crane)));
    assert!(gen.normal("Crane Jab"));
    assert!(!gen.normal("Mantis Jab"));
    gen.land_hit();
    gen.land_hit();
    gen.land_hit();
    assert!(gen.super_combo(1));
    assert!(matches!(gen.events.last(), Some(AlphaEvent::Super { name: "Jakorifu", .. })));
    assert!(!AlphaSession::new(TournamentFighter::Ryu, Revision::Sfz1, Loadout::AIsm).switch_stance(true));

    let mut eagle = AlphaSession::new(TournamentFighter::Eagle, Revision::Sfz3, Loadout::AIsm);
    eagle.land_hit();
    assert!(eagle.super_combo(1));
    assert!(matches!(eagle.events.last(), Some(AlphaEvent::Super { name: "Manchester Gold", .. })));

    let mut gauged = AlphaSession::new(TournamentFighter::Ryu, Revision::Sfz2, Loadout::AIsm);
    gauged.land_hit();
    gauged.land_hit();
    assert_eq!(gauged.hud(), Some(EraGauge::one(GaugeRow::level(2, 3, GaugeFill::of(2, 3)))), "a level bar at level 2");
    gauged.land_hit();
    assert_eq!(gauged.hud(), Some(EraGauge::one(GaugeRow::level(3, 3, GaugeFill::of(3, 3)))));
    assert!(gauged.custom_combo());
    let draining = |left: u16| {
        Some(EraGauge::one(GaugeRow::level(2, 3, GaugeFill::of(left, 90)).with_state(GaugeState::Draining)))
    };
    assert_eq!(gauged.hud(), draining(90), "a Custom Combo drains the bar with its timer");
    for _ in 0..45 {
        gauged.tick();
    }
    assert_eq!(gauged.hud(), draining(45));
    for _ in 0..45 {
        gauged.tick();
    }
    assert_eq!(gauged.hud(), Some(EraGauge::one(GaugeRow::level(2, 3, GaugeFill::of(2, 3)))));
}

trait Fill {
    fn stocks_for_test(&mut self, n: u8);
}

impl Fill for AlphaSession {
    fn stocks_for_test(&mut self, n: u8) {
        for _ in 0..n {
            self.land_hit();
        }
    }
}
