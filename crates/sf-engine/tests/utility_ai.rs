//! Style-aware CPU, drills, and combo trials (`#109`, `O18`).

use sf_engine::era::EraFamily;
use sf_engine::roster::{PlayMode, TournamentFighter};
use sf_engine::utility::{decide, Action, ComboTrial, CpuKind, Difficulty, Drill, DrillRun, Spot};

fn spot() -> Spot {
    Spot {
        distance: 40,
        airborne: false,
        throw_attempt: false,
        can_cancel: false,
        attack_incoming: false,
        blocking: false,
        stocks: 0,
        cancel_ok: false,
        command_throw: false,
        teleport: false,
    }
}

#[test]
fn sf1_stays_on_the_table_and_later_styles_use_utility() {
    for mode in [PlayMode::Arcade, PlayMode::Versus, PlayMode::Cpu] {
        let table = decide(mode, EraFamily::Sf1987, Difficulty::Hard, true, 1, spot());
        assert_eq!(table.kind, CpuKind::Table);
        assert_eq!(table.delay, 8);
        let raw = decide(mode, EraFamily::Sf1987, Difficulty::Hard, false, 1, spot());
        assert_eq!(raw.delay, 0);
        let later = decide(mode, EraFamily::Sf6, Difficulty::VeryHard, true, 1, spot());
        assert_eq!(later.kind, CpuKind::Utility);
    }
}

#[test]
fn utility_cancels_antairs_techs_and_spends_the_gauge() {
    let mut air = spot();
    air.airborne = true;
    air.distance = 40;
    let anti = decide(PlayMode::Arcade, EraFamily::Sf2, Difficulty::VeryHard, true, 7, air);
    assert_eq!(anti.action, Action::AntiAir);
    assert_eq!(anti.delay, 1);

    let mut tech = spot();
    tech.throw_attempt = true;
    assert_eq!(
        decide(PlayMode::Versus, EraFamily::Sf1Plus, Difficulty::VeryHard, true, 7, tech).action,
        Action::ThrowTech
    );

    let mut cancel = spot();
    cancel.can_cancel = true;
    assert_eq!(
        decide(PlayMode::Cpu, EraFamily::Sf5, Difficulty::VeryHard, true, 7, cancel).action,
        Action::SpecialCancel
    );

    let mut gauge = spot();
    gauge.distance = 120;
    gauge.stocks = 1;
    assert_eq!(decide(PlayMode::Arcade, EraFamily::Sf2, Difficulty::VeryHard, true, 3, gauge).action, Action::Super);
    gauge.blocking = true;
    assert_eq!(decide(PlayMode::Arcade, EraFamily::SfZ, Difficulty::VeryHard, true, 3, gauge).action, Action::AlphaCounter);
    gauge.attack_incoming = true;
    assert_eq!(decide(PlayMode::Arcade, EraFamily::Sf3, Difficulty::VeryHard, true, 3, gauge).action, Action::Parry);
    gauge.stocks = 4;
    gauge.cancel_ok = true;
    assert_eq!(decide(PlayMode::Arcade, EraFamily::Sf4, Difficulty::VeryHard, true, 3, gauge).action, Action::Fadc);
    gauge.stocks = 1;
    assert_eq!(decide(PlayMode::Arcade, EraFamily::Sf5, Difficulty::VeryHard, true, 3, gauge).action, Action::VTrigger);
    gauge.distance = 20;
    assert_eq!(decide(PlayMode::Arcade, EraFamily::Sf6, Difficulty::VeryHard, true, 3, gauge).action, Action::DriveImpact);
    gauge.distance = 140;
    assert_eq!(decide(PlayMode::Arcade, EraFamily::Sf6, Difficulty::VeryHard, true, 3, gauge).action, Action::DriveRush);

    let easy = decide(PlayMode::Arcade, EraFamily::Sf2, Difficulty::Easy, true, 1, spot());
    let again = decide(PlayMode::Arcade, EraFamily::Sf2, Difficulty::Easy, true, 1, spot());
    assert_eq!(easy, again);
    assert_eq!(easy.delay, 12);
}

#[test]
fn drills_and_trials_track_each_step() {
    let mut anti = DrillRun::start(Drill::AntiAir);
    assert!(anti.respond(Action::AntiAir));
    anti.respond(Action::AntiAir);
    anti.respond(Action::AntiAir);
    assert!(anti.complete);

    let mut tech = DrillRun::start(Drill::ThrowTech);
    assert!(!tech.respond(Action::Normal));
    assert!(!tech.complete);
    tech.respond(Action::ThrowTech);
    tech.respond(Action::ThrowTech);
    tech.respond(Action::ThrowTech);
    assert!(tech.complete);

    let mut live = DrillRun::start(Drill::Survival);
    assert!(live.respond(Action::Hold));
    assert!(!live.respond(Action::Normal));
    assert_eq!(live.health, 120);
    live.respond(Action::Hold);
    live.respond(Action::Hold);
    live.respond(Action::Hold);
    assert!(live.complete);

    let mut trial = ComboTrial::new(TournamentFighter::Geki, EraFamily::Sf6);
    assert_eq!(trial.progress(), (0, 3));
    assert!(trial.advance("Normal"));
    assert!(trial.advance("Special"));
    assert!(trial.advance("Level 3"));
    assert!(trial.complete());
    assert_eq!(trial.progress(), (3, 3));

    let mut missed = ComboTrial::new(TournamentFighter::Ryu, EraFamily::Sf3);
    assert!(!missed.advance("Super Art"));
    assert!(missed.failed);
}
