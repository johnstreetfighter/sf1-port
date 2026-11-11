//! SF2 revisions, the era selector, and the Super gauge (`#103`, `O12`).

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::era::{
    costumes, families, revisions, Costume, EraFamily, GaugeMode, Loadout, Revision, StylePick,
    Sf2Session, PALETTES,
};
use sf_engine::roster::{CharacterSelectScreen, MenuInput, PlayMode, RosterSettings, TournamentFighter};
use sf_engine::sf1plus::{CombatEvent, EraStyle, Pad, Sf1PlusRules};
use sf_video::era_hud::{EraGauge, Fill, GaugeRow};

fn session(
    fighter: TournamentFighter,
    revision: Revision,
    loadout: Loadout,
    gauge: GaugeMode,
) -> Sf2Session {
    Sf2Session::new(
        fighter,
        TournamentFighter::Ken,
        revision,
        loadout,
        gauge,
        Sf1PlusRules::for_preset(RulesetPreset::DefinitiveOverhaul),
    )
}

fn fill(session: &mut Sf2Session, name: &str) {
    for _ in 0..6 {
        if session.stock() == 1 {
            break;
        }
        assert!(session.play(name), "{name} did not start while building the gauge");
    }
    assert_eq!(session.stock(), 1, "gauge did not fill from {name}");
}

#[test]
fn selector_lists_only_canonical_eras_and_eight_palettes() {
    assert!(!families(TournamentFighter::Lee).contains(&EraFamily::Sf2));
    assert_eq!(families(TournamentFighter::Ryu).len(), 8);
    assert_eq!(
        revisions(TournamentFighter::Sagat, EraFamily::Sf2),
        vec![Revision::Ww, Revision::Ce, Revision::Hf, Revision::Super, Revision::St]
    );
    assert_eq!(revisions(TournamentFighter::Gen, EraFamily::Sf2), Vec::<Revision>::new());
    assert_eq!(
        costumes(TournamentFighter::Ken, EraFamily::Sf2),
        vec![Costume::Classic1987, Costume::Sf2]
    );
    assert_eq!(costumes(TournamentFighter::Lee, EraFamily::Sf1Plus), vec![Costume::Classic1987]);
    assert_eq!(PALETTES, 8);

    let mut settings = RosterSettings::for_preset(RulesetPreset::DefinitiveOverhaul);
    settings.mode = PlayMode::Arcade;
    let mut screen = CharacterSelectScreen::open(settings, true);
    screen.apply(MenuInput::Confirm);
    assert_eq!(screen.phase_name(), "Country");
    screen.apply(MenuInput::Confirm);
    assert_eq!(screen.phase_name(), "Era family");
    let listed = screen.rows().into_iter().find(|row| row.starts_with("ERA FAMILY")).unwrap();
    assert!(listed.contains("SF2"));
    screen.apply(MenuInput::Right);
    screen.apply(MenuInput::Confirm);
    assert_eq!(screen.phase_name(), "Revision");
    for _ in 0..4 {
        screen.apply(MenuInput::Right);
    }
    screen.apply(MenuInput::Up);
    screen.apply(MenuInput::Confirm);
    assert_eq!(screen.phase_name(), "Costume");
    screen.apply(MenuInput::Right);
    for _ in 0..7 {
        screen.apply(MenuInput::Up);
    }
    screen.apply(MenuInput::Confirm);
    assert!(screen.is_ready());
    assert_eq!(
        screen.p1_style(),
        StylePick::sf2(Revision::St, Loadout::Old, Costume::Sf2, 7)
    );

    let mut lee = CharacterSelectScreen::open(settings, true);
    for _ in 0..16 {
        if lee.rows().iter().any(|row| row.starts_with("> ") && row.contains("Lee")) {
            lee.apply(MenuInput::Confirm);
            break;
        }
        lee.apply(MenuInput::Right);
    }
    lee.apply(MenuInput::Confirm);
    let lee_families = lee.rows().into_iter().find(|row| row.starts_with("ERA FAMILY")).unwrap();
    assert!(!lee_families.contains("SF2"), "{lee_families}");
}

#[test]
fn revisions_change_stun_hits_air_tatsu_and_the_super_moves() {
    let rules = Sf1PlusRules::for_preset(RulesetPreset::DefinitiveOverhaul);
    let ww = session(TournamentFighter::Ryu, Revision::Ww, Loadout::Standard, GaugeMode::EraDefault);
    let hf = session(TournamentFighter::Ryu, Revision::Hf, Loadout::Standard, GaugeMode::EraDefault);
    assert!(ww.stat("stand LP").unwrap().hitstun > hf.stat("stand LP").unwrap().hitstun);
    assert!(hf.stat("Hadoken").unwrap().startup < ww.stat("Hadoken").unwrap().startup);
    assert!(!ww.stat("Tatsumaki").unwrap().air_ok);
    assert!(hf.stat("Tatsumaki").unwrap().air_ok);

    let mut ww_play = session(TournamentFighter::Ryu, Revision::Ww, Loadout::Standard, GaugeMode::EraDefault);
    assert!(ww_play.play("stand LP"));
    assert!(matches!(ww_play.hits()[0], CombatEvent::Hit { hitstun: 20, .. }));
    assert!(!ww_play.play_air("Tatsumaki"));

    let mut hf_play = session(TournamentFighter::Ryu, Revision::Hf, Loadout::Standard, GaugeMode::EraDefault);
    assert!(hf_play.play_air("Tatsumaki"));

    let mut ken = session(TournamentFighter::Ken, Revision::Ce, Loadout::Standard, GaugeMode::EraDefault);
    assert!(ken.play("Shoryuken"));
    assert_eq!(ken.hits().len(), 2);
    let mut ken_ww = session(TournamentFighter::Ken, Revision::Ww, Loadout::Standard, GaugeMode::EraDefault);
    assert!(ken_ww.play("Shoryuken"));
    assert_eq!(ken_ww.hits().len(), 1);

    let super_ryu = session(TournamentFighter::Ryu, Revision::Super, Loadout::Standard, GaugeMode::EraDefault);
    assert!(super_ryu.stat("Shakunetsu Hadoken").is_some());
    assert!(super_ryu.stat("Shoryuken").unwrap().damage > ww.stat("Shoryuken").unwrap().damage);
    let mut flame = session(TournamentFighter::Ryu, Revision::Super, Loadout::Standard, GaugeMode::EraDefault);
    assert!(flame.play("Shakunetsu Hadoken"));
    assert!(flame.hits().iter().any(|event| matches!(event, CombatEvent::Hit { damage, .. } if *damage == super_ryu.stat("Shakunetsu Hadoken").unwrap().damage)));

    let old = session(TournamentFighter::Sagat, Revision::St, Loadout::Old, GaugeMode::EraDefault);
    let new = session(TournamentFighter::Sagat, Revision::St, Loadout::Standard, GaugeMode::EraDefault);
    assert!(old.stat("Tiger Knee").unwrap().damage > new.stat("Tiger Knee").unwrap().damage);
    assert!(old.stat("Tiger Genocide").unwrap().super_move);

    let mut modern = Sf2Session::versus(
        TournamentFighter::Ryu,
        TournamentFighter::Ken,
        Revision::Ww,
        Loadout::Standard,
        GaugeMode::EraDefault,
        rules,
        EraStyle::Sf1Plus,
    );
    let guard = Pad { down: true, right: true, ..Pad::default() };
    assert!(modern.play_against("crouch LK", guard));
    assert!(modern.blocked(), "SF1+ defender blocked the low");
    assert!(modern.hits().is_empty());

    let mut classic = Sf2Session::versus(
        TournamentFighter::Ryu,
        TournamentFighter::Ken,
        Revision::Ww,
        Loadout::Standard,
        GaugeMode::EraDefault,
        rules,
        EraStyle::Sf1987,
    );
    assert!(classic.play_against("crouch LK", guard));
    assert!(!classic.blocked(), "1987 defender does not block lows");
    assert!(!classic.hits().is_empty());
}

#[test]
fn super_stock_fills_from_hits_spends_on_super_and_obeys_gauge_mode() {
    assert_eq!(GaugeMode::for_preset(RulesetPreset::PureFootsies1987Plus), GaugeMode::NoGauges);
    let footsies = MatchConfig::from_preset(RulesetPreset::PureFootsies1987Plus);
    assert_eq!(footsies.gauge_mode, GaugeMode::NoGauges);
    let round = MatchConfig::from_toml(&footsies.to_toml()).unwrap();
    assert_eq!(round.gauge_mode, GaugeMode::NoGauges);

    let mut muted = session(TournamentFighter::Ryu, Revision::St, Loadout::Standard, footsies.gauge_mode);
    assert!(muted.play("Hadoken"));
    assert_eq!(muted.stock(), 0);
    assert!(!muted.play("Shinku Hadoken"));

    let mut ww = session(TournamentFighter::Ryu, Revision::Ww, Loadout::Standard, GaugeMode::EraDefault);
    assert!(ww.play("Hadoken"));
    assert_eq!(ww.stock(), 0, "World Warrior has no super stock");

    let mut unified = session(TournamentFighter::Ryu, Revision::Ww, Loadout::Standard, GaugeMode::Unified);
    fill(&mut unified, "Hadoken");
    assert!(unified.play("Shinku Hadoken"));
    assert_eq!(unified.stock(), 0);

    let mut st = session(TournamentFighter::Ken, Revision::St, Loadout::Standard, GaugeMode::EraDefault);
    fill(&mut st, "Hadoken");
    assert!(st.super_cancel("stand LP", "Shoryureppa"));
    assert!(st.canceled("Shoryureppa"));
    assert_eq!(st.stock(), 0);

    let mut sagat = session(TournamentFighter::Sagat, Revision::St, Loadout::Standard, GaugeMode::EraDefault);
    fill(&mut sagat, "High Tiger Shot");
    assert_eq!(sagat.hud(), Some(EraGauge::one(GaugeRow::stock(1, 1, Fill::EMPTY))), "one stock cell, full");
    assert!(sagat.play("Tiger Genocide"));
    assert_eq!(sagat.stock(), 0);
    assert_eq!(sagat.hud(), Some(EraGauge::one(GaugeRow::stock(1, 0, Fill::of(0, 48)))));
}

#[test]
fn the_super_gauge_is_one_cell_filling_from_the_meter() {
    let mut ryu = session(TournamentFighter::Ryu, Revision::St, Loadout::Standard, GaugeMode::EraDefault);
    assert_eq!(ryu.hud(), Some(EraGauge::one(GaugeRow::stock(1, 0, Fill::of(0, 48)))));
    assert!(ryu.play("Hadoken"));
    let meter = ryu.meter();
    assert!(meter > 0 && ryu.stock() == 0);
    assert_eq!(ryu.hud(), Some(EraGauge::one(GaugeRow::stock(1, 0, Fill::of(meter.into(), 48)))));
    let unified = session(TournamentFighter::Ryu, Revision::Ww, Loadout::Standard, GaugeMode::Unified);
    assert!(unified.hud().is_some(), "Unified Gauge shows the cell in any revision");
    let ww = session(TournamentFighter::Ryu, Revision::Ww, Loadout::Standard, GaugeMode::EraDefault);
    assert_eq!(ww.hud(), None, "World Warrior has no gauge");
    let off = session(TournamentFighter::Ryu, Revision::St, Loadout::Standard, GaugeMode::NoGauges);
    assert_eq!(off.hud(), None, "gauges off");
}
