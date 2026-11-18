//! `SF KESSEN` on the style card and its two meters (KS2, #219): the style
//! is picked on the style card, reaches a real fight on Ryu's Super Turbo
//! moves, and holds a Super meter and round-rationed Versatile stocks.

#[path = "support/alpha.rs"]
mod alpha;
use alpha::{local_fight, local_select_with_p1, pad, DOWN, RIGHT};
use sf_engine::era::{
    listed_families, loadouts, Costume, EraFamily, GaugeMode, Loadout, Revision, StylePick,
};
use sf_engine::front::fight::Fight;
use sf_engine::front::{GameSession, PlayerInputs};
use sf_engine::roster::TournamentFighter;
use sf_video::era_hud::{EraGauge, Fill, GaugeRow};
use sf_video::romset::Assets;

const IDLE: PlayerInputs = PlayerInputs {
    directions: [false; 4],
    buttons: [false; 6],
};
const KESSEN: [Loadout; 3] = [Loadout::Balanced, Loadout::Versatile, Loadout::SuperHeavy];

/// Kessen Ryu on Player 1 against SF1+ Ken, at the start of round 1.
fn bout(loadout: Loadout) -> Option<(Fight, GameSession)> {
    let session = local_select_with_p1(GaugeMode::EraDefault, Revision::Kessen, loadout)?;
    let fight = local_fight(&session, Revision::Sf1Plus, Loadout::Standard)
        .fight()
        .expect("local fight")
        .clone();
    Some((fight, session))
}

fn versatile(fight: &Fight) -> u8 {
    fight.machine().era_fighters()[0]
        .versatile_stocks()
        .expect("SF KESSEN is selected")
}

fn super_meter(fight: &Fight) -> u8 {
    fight.machine().era_fighters()[0].super_meter
}

fn set_super_meter(fight: &mut Fight, value: u8) {
    let mut snapshot = fight.save_netplay_snapshot(0);
    snapshot.machine.toggle_state.era.fighters[0].super_meter = value;
    fight.restore_netplay_snapshot(&snapshot);
}

/// End the round on a timeout that `loser` (0 or 1) loses, and wait for
/// the next round, numbered `next` from zero, to reach its idle stance.
fn next_round(fight: &mut Fight, assets: &Assets, loser: usize, next: u8) {
    use sf_engine::fighter::{P1_STRUCT, P2_STRUCT, WORK_BASE};
    use sf_engine::tasks::TIME_DIGITS;

    let mut snapshot = fight.save_netplay_snapshot(0);
    let health = ([P1_STRUCT, P2_STRUCT][loser] + 6 - WORK_BASE) as usize;
    snapshot.machine.work[health] = 1;
    snapshot.machine.work[health + 1] = 0;
    snapshot.machine.toggle_state.era.fighters[loser].sub_hp = 20;
    let timer = (TIME_DIGITS - WORK_BASE) as usize;
    snapshot.machine.work[timer..timer + 3].fill(0);
    fight.restore_netplay_snapshot(&snapshot);
    for _ in 0..900 {
        fight.step(assets, IDLE, IDLE, false);
        let fighters = fight.fight_read().fighters;
        if fight.machine().work_ram()[0x400a] == next
            && fighters[0].move_id == 1
            && fighters[loser].health > 1
        {
            return;
        }
    }
    panic!("round {} did not reach idle", next + 1);
}

#[test]
fn the_style_card_offers_sf_kessen_for_built_kits_only_and_never_to_the_arcade_cpu() {
    for what_if in [false, true] {
        assert!(listed_families(TournamentFighter::Ryu, what_if).contains(&EraFamily::Kessen));
        for fighter in TournamentFighter::ALL {
            if fighter != TournamentFighter::Ryu {
                assert!(!listed_families(fighter, what_if).contains(&EraFamily::Kessen), "{fighter:?}");
            }
        }
    }
    assert_eq!(loadouts(Revision::Kessen), KESSEN);
    for loadout in KESSEN {
        let pick = StylePick {
            family: EraFamily::Kessen,
            revision: Revision::Kessen,
            loadout,
            costume: Costume::Classic1987,
            palette: 0,
        };
        for step in 0..12 {
            for opponent in TournamentFighter::ALL {
                let given = pick.arcade_opponent(opponent, true, step);
                assert_ne!(given.family, EraFamily::Kessen, "{opponent:?} at step {step}");
            }
        }
    }
}

#[test]
fn each_loadout_reaches_a_fight_with_its_super_stocks_and_opening_versatile_stocks() {
    // The blueprint's table: (Super stocks, Versatile stocks in round 1, cap).
    for (loadout, supers, opening, cap) in [
        (Loadout::Balanced, 2, 3, 5),
        (Loadout::Versatile, 1, 4, 6),
        (Loadout::SuperHeavy, 3, 3, 3),
    ] {
        let Some((fight, _)) = bout(loadout) else { return };
        let ryu = fight.machine().era_fighters()[0];
        assert_eq!(ryu.pick().revision, Revision::Kessen);
        assert_eq!(ryu.pick().loadout, loadout);
        assert_eq!(versatile(&fight), opening, "{loadout:?}");
        assert_eq!(
            fight.era_hud().p1,
            Some(EraGauge::two(
                GaugeRow::stock(supers, 0, Fill { num: 0, den: 48 }),
                GaugeRow::stock(cap, opening, Fill::EMPTY),
            )),
            "{loadout:?}"
        );
        assert_eq!(fight.era_hud().p2, None, "SF1+ Ken has no gauge");
    }
}

#[test]
fn versatile_stocks_are_granted_by_round_up_to_the_cap() {
    // Round 2 and round 3 gains: Balanced +2 / +1 to a cap of 5,
    // Versatile +2 / +2 to 6, Super +1 / +1 to a cap of 3.
    for (loadout, round_two, round_three) in [
        (Loadout::Balanced, 5, 5),
        (Loadout::Versatile, 6, 6),
        (Loadout::SuperHeavy, 3, 3),
    ] {
        let Some((mut fight, session)) = bout(loadout) else { return };
        next_round(&mut fight, session.assets(), 1, 1);
        assert_eq!(versatile(&fight), round_two, "{loadout:?} in round 2");
        next_round(&mut fight, session.assets(), 0, 2);
        assert_eq!(versatile(&fight), round_three, "{loadout:?} in round 3");
    }
}

#[test]
fn the_ration_repeats_round_threes_gain_in_later_rounds_and_a_single_round_gets_the_opening() {
    let balanced = Loadout::Balanced.kessen_ration().unwrap();
    assert_eq!(balanced.at_round(1, 0), 3);
    assert_eq!(balanced.at_round(1, 5), 3, "a new match starts from the opening allotment whatever was held");
    assert_eq!(balanced.at_round(2, 1), 3, "one stock left, plus two");
    assert_eq!(balanced.at_round(3, 0), 1);
    assert_eq!(balanced.at_round(4, 1), 2, "round 4 gains what round 3 does");
    assert_eq!(balanced.at_round(5, 5), 5, "never above the cap");
    let versatile = Loadout::Versatile.kessen_ration().unwrap();
    assert_eq!(versatile.at_round(3, 1), 3);
    assert_eq!(Loadout::Standard.kessen_ration(), None);
}

#[test]
fn no_gauges_removes_both_meters() {
    let Some(session) = local_select_with_p1(GaugeMode::NoGauges, Revision::Kessen, Loadout::Balanced) else { return };
    let fight = local_fight(&session, Revision::Sf1Plus, Loadout::Standard).fight().unwrap().clone();
    assert_eq!(versatile(&fight), 0);
    assert_eq!(fight.era_hud().p1, None);
}

#[test]
fn super_meter_carries_into_the_next_round() {
    let Some((mut fight, session)) = bout(Loadout::Balanced) else { return };
    set_super_meter(&mut fight, 70);
    next_round(&mut fight, session.assets(), 1, 1);
    assert_eq!(super_meter(&fight), 70);
}

#[test]
fn nothing_during_a_round_adds_a_versatile_stock() {
    let Some((mut fight, session)) = bout(Loadout::Balanced) else { return };
    // Walk in and attack for five seconds; meter builds, stocks do not.
    for frame in 0..300 {
        let input = if frame % 20 < 3 { pad(RIGHT, Some(frame / 20 % 6)) } else { pad(RIGHT, None) };
        fight.step(session.assets(), input, IDLE, false);
    }
    assert!(super_meter(&fight) > 0, "the attacks built Super meter");
    assert_eq!(versatile(&fight), 3);
}

/// A Hadoken's worth of Super meter from an empty gauge.
fn meter_from_one_hadoken(loadout: Loadout) -> Option<u8> {
    let (mut fight, session) = bout(loadout)?;
    for (i, direction) in [DOWN, DOWN | RIGHT, RIGHT].into_iter().enumerate() {
        fight.step(session.assets(), pad(direction, (i == 2).then_some(0)), IDLE, false);
    }
    for _ in 0..20 {
        fight.step(session.assets(), IDLE, IDLE, false);
    }
    Some(super_meter(&fight))
}

#[test]
fn the_versatile_loadout_builds_super_meter_at_half_the_rate() {
    let Some(balanced) = meter_from_one_hadoken(Loadout::Balanced) else { return };
    let versatile = meter_from_one_hadoken(Loadout::Versatile).unwrap();
    assert!(balanced >= 2, "a Hadoken builds meter: {balanced}");
    assert_eq!(versatile, balanced / 2);
}

#[test]
fn shinku_hadoken_costs_one_super_stock_and_leaves_the_rest() {
    let Some((mut fight, session)) = bout(Loadout::Balanced) else { return };
    set_super_meter(&mut fight, 96);
    let motion = [DOWN, DOWN | RIGHT, RIGHT, DOWN, DOWN | RIGHT, RIGHT];
    for (i, direction) in motion.into_iter().enumerate() {
        fight.step(session.assets(), pad(direction, (i == 5).then_some(0)), IDLE, false);
    }
    assert_eq!(fight.fight_read().fighters[0].move_id, sf_engine::era_fight::SF2_SUPER);
    // The stock is spent as the super's flash ends, not on the input.
    for _ in 0..60 {
        fight.step(session.assets(), IDLE, IDLE, false);
    }
    assert_eq!(super_meter(&fight), 48, "one of two stocks spent");
}

#[test]
fn a_kessen_fight_replays_identically_from_a_snapshot() {
    let Some((mut fight, session)) = bout(Loadout::Versatile) else { return };
    let start = fight.save_netplay_snapshot(0);
    let run = |fight: &mut Fight| {
        for frame in 0..240 {
            let p1 = if frame % 30 < 3 { pad(RIGHT, Some(frame / 30 % 6)) } else { pad(RIGHT, None) };
            fight.step(session.assets(), p1, IDLE, false);
        }
        (fight.machine().work_ram().to_vec(), *fight.machine().era_fighters())
    };
    let first = run(&mut fight);
    fight.restore_netplay_snapshot(&start);
    let second = run(&mut fight);
    assert!(first == second, "the same inputs from the same snapshot give the same fight");
}

#[test]
fn a_kessen_pick_survives_the_lobby_packet() {
    use sf_engine::netplay::protocol::{LobbyPhase, NetPacket};
    use sf_engine::roster::StagePick;

    for loadout in KESSEN {
        let pick = StylePick {
            family: EraFamily::Kessen,
            revision: Revision::Kessen,
            loadout,
            costume: Costume::Classic1987,
            palette: 0,
        };
        let packet = NetPacket::LobbyPick {
            generation: 1, seq: 2, side: 0, fighter: TournamentFighter::Ryu,
            stage: StagePick::Bank(0), locked: true, phase: LobbyPhase::Fighters,
            random: false, style: Some(pick),
        };
        assert_eq!(NetPacket::from_bytes(&packet.to_bytes()), Some(packet), "{loadout:?}");
    }
}
