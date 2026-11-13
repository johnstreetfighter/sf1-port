//! Street Fighter IV systems through the native fight and Style Card.

#[allow(dead_code)]
#[path = "support/alpha.rs"]
mod alpha;

use alpha::{local_fight, local_select_with_fighter, pad, DOWN, RIGHT};
use sf_engine::era::{GaugeMode, Loadout, Revision};
use sf_engine::front::fight::{Fight, FightFrame};
use sf_engine::front::show::ready;
use sf_engine::front::{FrameInputs, GameSession, PlayerInputs};
use sf_engine::roster::TournamentFighter;
use sf_video::callouts::{CalloutKind, Side};
use sf_video::era_hud::{Fill, GaugeRow, GaugeState};
use sf_video::romset::Assets;

const STOCK_AT: u8 = 48;
const FOCUS_CHARGE: u8 = 0x2e;
const FOCUS: u8 = 0x2f;
const DASH: u8 = 0x1e;
const MP: usize = 1;
const MK: usize = 4;
const HP: usize = 2;

fn select(revision: Revision, loadout: Loadout, fighter: TournamentFighter) -> Option<GameSession> {
    local_select_with_fighter(GaugeMode::EraDefault, fighter, revision, loadout)
}

fn bout(select: &GameSession) -> Fight {
    local_fight(select, Revision::Sf1Plus, Loadout::Standard)
        .fight()
        .expect("a local fight")
        .clone()
}

fn set_super(fight: &mut Fight, amount: u8) {
    let mut snapshot = fight.save_netplay_snapshot(0);
    snapshot.machine.toggle_state.era.fighters[0].super_meter = amount;
    fight.restore_netplay_snapshot(&snapshot);
}

fn held(directions: u8, buttons: &[usize]) -> PlayerInputs {
    let mut input = pad(directions, None);
    for &button in buttons {
        input.buttons[button] = true;
    }
    input
}

fn step(fight: &mut Fight, assets: &Assets, p1: PlayerInputs, p2: PlayerInputs) -> FightFrame {
    fight.step(assets, p1, p2, false)
}

fn p1_slot(fight: &Fight) -> u8 {
    fight.fight_read().fighters[0].move_id
}

fn close_range(fight: &mut Fight, assets: &Assets) {
    for _ in 0..90 {
        let telem = fight.machine().diagnostic_telemetry();
        if telem.p1.center_x.abs_diff(telem.p2.center_x) <= 38 {
            for _ in 0..8 {
                step(fight, assets, pad(0, None), pad(0, None));
            }
            return;
        }
        step(fight, assets, pad(RIGHT, None), pad(0, None));
    }
    panic!("Ryu and Ken never entered close range");
}

fn run_focus(fight: &mut Fight, assets: &Assets, red: bool, frames: usize, p2: PlayerInputs) {
    let buttons: &[usize] = if red { &[0, MP, MK] } else { &[MP, MK] };
    for _ in 0..frames {
        step(fight, assets, held(0, buttons), p2);
    }
}

#[test]
fn focus_absorbs_one_attack_then_recovers_lost_health() {
    let Some(select) = select(Revision::Usf4, Loadout::Ultra1, TournamentFighter::Ryu) else {
        return;
    };
    let assets = select.assets();
    let mut fight = bout(&select);
    close_range(&mut fight, assets);
    let hp_before = fight.machine().era_fighters()[0].sub_hp;
    assert!(matches!(
        fight.machine().era_fighters()[0].gauge().unwrap().main,
        GaugeRow::Stock { cells: 4, .. }
    ));

    run_focus(&mut fight, assets, false, 2, pad(0, None));
    assert_eq!(p1_slot(&fight), FOCUS_CHARGE);
    run_focus(&mut fight, assets, false, 1, pad(0, Some(MP)));
    run_focus(&mut fight, assets, false, 15, pad(0, None));
    assert_eq!(p1_slot(&fight), FOCUS_CHARGE, "the first punch is absorbed");
    let damaged = fight.machine().era_fighters()[0];
    assert!(damaged.sub_hp < hp_before);
    assert!(
        matches!(damaged.gauge().unwrap().second, Some(GaugeRow::Bar { fill, .. }) if fill.num > 0)
    );

    // Cancel the charge before it strikes to isolate grey-life recovery.
    set_super(&mut fight, 2 * STOCK_AT);
    for direction in [RIGHT, 0, RIGHT] {
        step(&mut fight, assets, held(direction, &[MP, MK]), pad(0, None));
    }
    assert_eq!(p1_slot(&fight), DASH);
    assert_eq!(fight.machine().era_fighters()[0].super_meter, 0);
    for _ in 0..125 {
        step(&mut fight, assets, pad(0, None), pad(0, None));
    }
    assert_eq!(fight.machine().era_fighters()[0].sub_hp, hp_before);
}

#[test]
fn red_focus_keeps_armor_for_multiple_hits_and_costs_two_stocks() {
    let Some(select) = select(Revision::Usf4, Loadout::Ultra1, TournamentFighter::Ryu) else {
        return;
    };
    let assets = select.assets();
    let mut fight = bout(&select);
    close_range(&mut fight, assets);
    set_super(&mut fight, 4 * STOCK_AT);
    run_focus(&mut fight, assets, true, 2, pad(0, None));
    assert_eq!(fight.machine().era_fighters()[0].super_meter, 2 * STOCK_AT);
    let before = fight.machine().era_fighters()[0].sub_hp;

    run_focus(&mut fight, assets, true, 1, pad(0, Some(0)));
    run_focus(&mut fight, assets, true, 15, pad(0, None));
    let after_first = fight.machine().era_fighters()[0].sub_hp;
    assert!(after_first < before);
    run_focus(&mut fight, assets, true, 1, pad(0, Some(0)));
    run_focus(&mut fight, assets, true, 12, pad(0, None));
    assert!(fight.machine().era_fighters()[0].sub_hp < after_first);
    assert_eq!(
        p1_slot(&fight),
        FOCUS_CHARGE,
        "Red Focus absorbs both punches"
    );
}

#[test]
fn ultra_double_offers_both_ryu_ultras_and_spends_revenge() {
    let Some(select) = select(Revision::Usf4, Loadout::UltraDouble, TournamentFighter::Ryu) else {
        return;
    };
    let assets = select.assets();
    let mut fight = bout(&select);
    close_range(&mut fight, assets);

    // Earn Revenge from clean attacks, then use the same charged snapshot
    // to verify both commands with no round reset or synthetic Ultra meter.
    let revenge_ready = |fight: &Fight| {
        matches!(
            fight.machine().era_fighters()[0].gauge().unwrap().second,
            Some(GaugeRow::Bar {
                state: GaugeState::Flashing { .. },
                ..
            })
        )
    };
    for _ in 0..6 {
        if revenge_ready(&fight) {
            break;
        }
        close_range(&mut fight, assets);
        step(&mut fight, assets, pad(0, None), pad(0, Some(HP)));
        for _ in 0..120 {
            step(&mut fight, assets, pad(0, None), pad(0, None));
            if fight.fight_read().fighters.iter().all(ready) {
                break;
            }
        }
    }
    assert!(
        revenge_ready(&fight),
        "clean damage reaches the Ultra threshold"
    );
    assert!(
        ready(&fight.fight_read().fighters[0]),
        "Ryu recovers before trying the Ultra"
    );
    let charged = fight.save_netplay_snapshot(0);
    for (slot, name, buttons) in [
        (0x2a, "METSU HADOKEN", &[0, 1, 2][..]),
        (0x2b, "METSU SHORYUKEN", &[3, 4, 5][..]),
    ] {
        fight.restore_netplay_snapshot(&charged);
        for direction in [DOWN, DOWN | RIGHT, RIGHT, DOWN, DOWN | RIGHT] {
            step(&mut fight, assets, pad(direction, None), pad(0, None));
        }
        step(&mut fight, assets, held(RIGHT, buttons), pad(0, None));
        assert_eq!(p1_slot(&fight), slot);
        assert!(fight
            .callouts()
            .frame()
            .side(Side::P1)
            .words
            .contains(&Some(CalloutKind::Super { name })));
        assert!(
            matches!(fight.machine().era_fighters()[0].gauge().unwrap().second,
            Some(GaugeRow::Bar { fill, .. }) if fill.num == 0)
        );
    }
}

fn metsu_shoryuken(fight: &mut Fight, assets: &Assets) -> u16 {
    let before = fight.machine().era_fighters()[1].sub_hp;
    for direction in [DOWN, DOWN | RIGHT, RIGHT, DOWN, DOWN | RIGHT] {
        step(fight, assets, pad(direction, None), pad(0, None));
    }
    step(fight, assets, held(RIGHT, &[3, 4, 5]), pad(0, None));
    assert_eq!(p1_slot(fight), 0x2b, "Metsu Shoryuken starts");
    for _ in 0..90 {
        step(fight, assets, pad(0, None), pad(0, None));
        let after = fight.machine().era_fighters()[1].sub_hp;
        if after < before {
            return before - after;
        }
    }
    panic!("Metsu Shoryuken does not connect");
}

#[test]
fn ultra_double_deals_three_quarters_of_the_same_ultra_hit() {
    let mut ordinary_damage = [0; 2];
    for (index, disable_comeback) in [false, true].into_iter().enumerate() {
        let mut damage = [0; 2];
        for (value, loadout) in damage.iter_mut().zip([Loadout::Ultra2, Loadout::UltraDouble]) {
            let Some(session) = alpha::training_fight_with_fighter(
                TournamentFighter::Ryu, Revision::Usf4, loadout,
            ) else {
                return;
            };
            let assets = session.assets();
            let mut fight = session.fight().unwrap().clone();
            let mut snapshot = fight.save_netplay_snapshot(0);
            snapshot.setup.config.disable_comeback_scaling = disable_comeback;
            snapshot.machine.match_config.disable_comeback_scaling = disable_comeback;
            fight.restore_netplay_snapshot(&snapshot);
            close_range(&mut fight, assets);
            *value = metsu_shoryuken(&mut fight, assets);
        }
        let [single, double] = damage;
        assert!(double.abs_diff(single * 3 / 4) <= 1, "single={single}, double={double}");
        ordinary_damage[index] = single;
    }
    assert!(ordinary_damage[1] < ordinary_damage[0], "the safety switch removes the Revenge bonus");
}

#[test]
fn a_special_focus_dash_cancel_can_follow_the_crumple_with_an_ultra() {
    let Some(session) = alpha::training_fight_with_fighter(
        TournamentFighter::Ryu, Revision::Usf4, Loadout::Ultra2,
    ) else {
        return;
    };
    let assets = session.assets();
    let mut fight = session.fight().unwrap().clone();
    close_range(&mut fight, assets);
    fight.set_fill_meters(false);
    step(&mut fight, assets, pad(DOWN, None), pad(0, None));
    step(&mut fight, assets, pad(DOWN | RIGHT, None), pad(0, None));
    step(&mut fight, assets, pad(RIGHT, Some(0)), pad(0, None));
    assert_eq!(p1_slot(&fight), 0x1a);
    let mut red = fight.clone();
    run_focus(&mut red, assets, true, 1, pad(0, None));
    assert_eq!(p1_slot(&red), FOCUS_CHARGE);
    assert_eq!(red.machine().era_fighters()[0].super_meter, STOCK_AT);

    run_focus(&mut fight, assets, false, 22, pad(0, None));
    assert_eq!(p1_slot(&fight), FOCUS_CHARGE);
    assert_eq!(fight.machine().era_fighters()[0].super_meter, 2 * STOCK_AT);
    let before = fight.machine().era_fighters()[1].sub_hp;
    assert!((0..25).any(|_| {
        step(&mut fight, assets, pad(0, None), pad(0, None));
        fight.machine().era_fighters()[1].sub_hp < before
    }), "level two Focus connects");
    for direction in [RIGHT, 0, RIGHT] {
        step(&mut fight, assets, pad(direction, None), pad(0, None));
    }
    assert_eq!(p1_slot(&fight), DASH);
    assert!((0..24).any(|_| {
        step(&mut fight, assets, pad(0, None), pad(0, None));
        ready(&fight.fight_read().fighters[0])
    }), "Ryu finishes the dash");
    assert!(fight.fight_read().fighters[1].reacting(), "the opponent is still crumpling");
    assert!(metsu_shoryuken(&mut fight, assets) > 0);
}

#[test]
fn the_super_gauge_carries_while_revenge_resets_for_round_two() {
    let Some(select) = select(Revision::Usf4, Loadout::Ultra1, TournamentFighter::Ryu) else {
        return;
    };
    let assets = select.assets();
    let mut fight = bout(&select);
    close_range(&mut fight, assets);
    let initial_hp = fight.machine().era_fighters()[0].sub_hp;
    step(&mut fight, assets, pad(0, None), pad(0, Some(HP)));
    assert!((0..30).any(|_| {
        step(&mut fight, assets, pad(0, None), pad(0, None));
        fight.machine().era_fighters()[0].sub_hp < initial_hp
    }));
    assert!(matches!(
        fight.era_hud().p1.unwrap().second,
        Some(GaugeRow::Bar { fill, .. }) if fill.num > 0
    ));
    set_super(&mut fight, 2 * STOCK_AT);
    alpha::round_two_after_timeout(&mut fight, assets);
    let gauge = fight.era_hud().p1.unwrap();
    assert_eq!(gauge.main, GaugeRow::stock(4, 2, Fill::of(0, STOCK_AT.into())));
    assert!(matches!(gauge.second, Some(GaugeRow::Bar { fill, .. }) if fill.num == 0));
}

#[test]
fn level_three_focus_crumple_is_deterministic_after_restore() {
    let Some(select) = select(Revision::Usf4, Loadout::Ultra1, TournamentFighter::Ryu) else {
        return;
    };
    let assets = select.assets();
    let mut fight = bout(&select);
    close_range(&mut fight, assets);
    let hp_before = fight.machine().era_fighters()[1].sub_hp;
    run_focus(&mut fight, assets, false, 25, pad(0, None));
    let charging = fight.save_netplay_snapshot(0);
    let mut checksum = None;
    for _ in 0..2 {
        fight.restore_netplay_snapshot(&charging);
        run_focus(&mut fight, assets, false, 26, pad(0, None));
        assert_eq!(p1_slot(&fight), FOCUS);
        for _ in 0..12 {
            step(&mut fight, assets, pad(0, None), pad(RIGHT, None));
        }
        assert!(
            fight.machine().era_fighters()[1].sub_hp < hp_before,
            "level 3 defeats guard"
        );
        let current = fight.state_checksum();
        assert_eq!(checksum.get_or_insert(current), &current);
    }
}

#[test]
fn level_two_focus_contact_can_dash_cancel_without_spending_stock() {
    let Some(select) = select(Revision::Usf4, Loadout::Ultra1, TournamentFighter::Ryu) else {
        return;
    };
    let assets = select.assets();
    let mut fight = bout(&select);
    close_range(&mut fight, assets);
    let before = fight.machine().era_fighters()[1].sub_hp;
    run_focus(&mut fight, assets, false, 23, pad(0, None));
    for _ in 0..8 {
        step(&mut fight, assets, pad(0, None), pad(0, None));
        if fight.machine().era_fighters()[1].sub_hp < before {
            break;
        }
    }
    assert!(fight.machine().era_fighters()[1].sub_hp < before);
    for direction in [RIGHT, 0, RIGHT] {
        step(&mut fight, assets, pad(direction, None), pad(0, None));
    }
    assert_eq!(p1_slot(&fight), DASH);
    assert_eq!(fight.machine().era_fighters()[0].super_meter, 0);
}

#[test]
fn ultra_delayed_wakeup_adds_eleven_lying_frames() {
    use sf_engine::combo::throw::{GET_UP_MOVE, THROWN_MOVE};

    let Some(select) = select(Revision::Usf4, Loadout::Ultra1, TournamentFighter::Ryu) else {
        return;
    };
    let assets = select.assets();
    let mut fight = bout(&select);
    close_range(&mut fight, assets);
    let before = fight.machine().era_fighters()[0].sub_hp;
    step(&mut fight, assets, pad(0, None), pad(alpha::LEFT, Some(HP)));
    assert_eq!(p1_slot(&fight), THROWN_MOVE);
    for _ in 0..75 {
        step(&mut fight, assets, pad(0, None), pad(0, None));
        if fight.machine().era_fighters()[0].sub_hp < before {
            break;
        }
    }
    assert_eq!(p1_slot(&fight), THROWN_MOVE);
    assert!(fight.machine().era_fighters()[0].sub_hp < before);
    let lying = fight.save_netplay_snapshot(0);
    let mut frames = [0; 2];
    for (index, delay) in [false, true].into_iter().enumerate() {
        fight.restore_netplay_snapshot(&lying);
        for frame in 0..65 {
            let input = if delay && frame == 1 {
                held(0, &[0, 3])
            } else {
                pad(0, None)
            };
            step(&mut fight, assets, input, pad(0, None));
            if p1_slot(&fight) == GET_UP_MOVE {
                frames[index] = frame;
                break;
            }
        }
    }
    assert!(frames[0] > 0 && frames[1] > 0, "both wakes occur");
    assert_eq!(frames[1], frames[0] + 11);
}

#[test]
fn unified_stock_pays_for_ultra_and_no_gauges_disables_focus() {
    for mode in [GaugeMode::NoGauges, GaugeMode::Unified] {
        let Some(select) = local_select_with_fighter(
            mode,
            TournamentFighter::Ryu,
            Revision::Usf4,
            Loadout::Ultra1,
        ) else {
            return;
        };
        let assets = select.assets();
        let mut fight = bout(&select);
        if mode == GaugeMode::NoGauges {
            assert!(fight.machine().era_fighters()[0].gauge().is_none());
            run_focus(&mut fight, assets, false, 3, pad(0, None));
            assert_ne!(p1_slot(&fight), FOCUS_CHARGE);
            for _ in 0..90 {
                step(&mut fight, assets, pad(0, None), pad(0, None));
                if ready(&fight.fight_read().fighters[0]) {
                    break;
                }
            }
            for direction in [DOWN, DOWN | RIGHT] {
                step(&mut fight, assets, pad(direction, None), pad(0, None));
            }
            step(&mut fight, assets, pad(RIGHT, Some(MP)), pad(0, None));
            assert_eq!(p1_slot(&fight), 0x1a, "ordinary Hadoken still works");
        } else {
            let gauge = fight.machine().era_fighters()[0].gauge().unwrap();
            assert!(matches!(gauge.main, GaugeRow::Stock { cells: 3, .. }));
            assert!(gauge.second.is_none());
            let neutral = fight.save_netplay_snapshot(0);
            set_super(&mut fight, STOCK_AT);
            run_focus(&mut fight, assets, false, 2, pad(0, None));
            for direction in [RIGHT, 0, RIGHT] {
                step(&mut fight, assets, held(direction, &[MP, MK]), pad(0, None));
            }
            assert_eq!(p1_slot(&fight), DASH);
            assert_eq!(fight.machine().era_fighters()[0].super_meter, 0);
            fight.restore_netplay_snapshot(&neutral);
            set_super(&mut fight, 3 * STOCK_AT);
            for direction in [DOWN, DOWN | RIGHT, RIGHT, DOWN, DOWN | RIGHT] {
                step(&mut fight, assets, pad(direction, None), pad(0, None));
            }
            step(&mut fight, assets, held(RIGHT, &[0, 1, 2]), pad(0, None));
            assert_eq!(p1_slot(&fight), 0x2a);
            assert_eq!(fight.machine().era_fighters()[0].super_meter, 0);
        }
    }
}

#[test]
fn late_remote_focus_input_replays_armor_and_revenge() {
    use sf_engine::netplay::{encode_inputs, RollbackEngine};

    let Some(select) = alpha::local_select(GaugeMode::EraDefault) else {
        return;
    };
    let assets = select.assets();
    let mut reference = local_fight(&select, Revision::Usf4, Loadout::Ultra1)
        .fight()
        .expect("a mixed SF1+ and SF4 fight")
        .clone();
    close_range(&mut reference, assets);
    for _ in 0..16 {
        step(&mut reference, assets, pad(RIGHT, None), pad(0, None));
    }
    for _ in 0..4 {
        step(&mut reference, assets, pad(0, None), pad(0, None));
    }
    let initial_hp = reference.machine().era_fighters()[1].sub_hp;
    let mut predicted = reference.clone();
    let mut net = RollbackEngine::new(0, 0, 8);
    const DELAY: u32 = 4;
    const FRAMES: u32 = 48;
    let remote = |frame| {
        if frame < 36 {
            held(0, &[MP, MK])
        } else {
            pad(0, None)
        }
    };

    let mut absorbed = false;
    for tick in 0..FRAMES + DELAY {
        if let Some(frame) = tick.checked_sub(DELAY) {
            net.apply_remote_inputs(
                &mut predicted,
                assets,
                frame,
                &[encode_inputs(remote(frame))],
            );
        }
        if tick < FRAMES {
            let attack = pad(0, (tick == 4).then_some(HP));
            step(&mut reference, assets, attack, remote(tick));
            net.step(&mut predicted, assets, attack)
                .expect("within rollback window");
            let fighter = reference.machine().era_fighters()[1];
            absorbed |= fighter.sub_hp < initial_hp
                && reference.fight_read().fighters[1].move_id == FOCUS_CHARGE
                && matches!(
                    fighter.gauge().unwrap().second,
                    Some(GaugeRow::Bar { fill, .. }) if fill.num > 0
                );
        }
    }
    assert!(absorbed, "the remote Focus absorbs a clean hit");
    assert_eq!(net.confirmed_inputs().len(), FRAMES as usize);
    assert_eq!(predicted.state_checksum(), reference.state_checksum());
}

#[test]
fn each_canonical_sf4_fighter_starts_a_metered_super_in_native_play() {
    for (fighter, revision, name, button, max_hp) in [
        (
            TournamentFighter::Ryu,
            Revision::Sf4,
            "SHINKU HADOKEN",
            HP,
            960,
        ),
        (
            TournamentFighter::Ken,
            Revision::Ssf4,
            "SHORYUREPPA",
            HP,
            960,
        ),
        (
            TournamentFighter::Sagat,
            Revision::Ae2012,
            "TIGER GENOCIDE",
            5,
            1008,
        ),
        (
            TournamentFighter::Adon,
            Revision::Usf4,
            "JAGUAR VARIED ASSAULT",
            HP,
            912,
        ),
        (TournamentFighter::Gen, Revision::Sf4, "ZAN'EI", HP, 864),
    ] {
        let Some(select) = select(revision, Loadout::Ultra1, fighter) else {
            return;
        };
        let mut fight = bout(&select);
        let assets = select.assets();
        assert_eq!(
            fight.machine().era_fighters()[0].max_sub_hp,
            max_hp,
            "{fighter:?}"
        );
        set_super(&mut fight, 4 * STOCK_AT);
        for direction in [DOWN, DOWN | RIGHT, RIGHT, DOWN, DOWN | RIGHT] {
            step(&mut fight, assets, pad(direction, None), pad(0, None));
        }
        step(&mut fight, assets, pad(RIGHT, Some(button)), pad(0, None));
        assert_eq!(p1_slot(&fight), 0x29, "{fighter:?}");
        assert_eq!(fight.machine().era_fighters()[0].super_meter, 0);
        assert!(fight
            .callouts()
            .frame()
            .side(Side::P1)
            .words
            .contains(&Some(CalloutKind::Super { name })));
    }
}

#[test]
fn training_fills_and_coaches_each_canonical_ultra_loadout() {
    use TournamentFighter::{Adon, Gen, Ken, Ryu, Sagat};

    for (fighter, first, second) in [
        (Ryu, "METSU HADOKEN", ("METSU SHORYUKEN", 0x2b)),
        (Ken, "SHINRYUKEN", ("GUREN SENPUKYAKU", 0x2b)),
        (Sagat, "TIGER DESTRUCTION", ("TIGER CANNON", 0x2b)),
        (Gen, "ZETSUEI", ("SHITENKETSU", 0x2c)),
        (Adon, "JAGUAR REVOLVER", ("JAGUAR AVALANCHE", 0x2b)),
    ] {
        for (loadout, (name, slot)) in [
            (Loadout::Ultra1, (first, 0x2a)),
            (Loadout::Ultra2, second),
            (Loadout::UltraDouble, second),
        ] {
            let Some(mut session) =
                alpha::training_fight_with_fighter(fighter, Revision::Usf4, loadout)
            else {
                return;
            };
            session.step(FrameInputs::default());
            let state = session.fight().unwrap().machine().era_fighters()[0];
            assert_eq!(state.super_meter, 4 * STOCK_AT, "{fighter:?} {loadout:?}");
            assert!(matches!(
                state.gauge().unwrap().second,
                Some(GaugeRow::Bar { fill, .. }) if fill.num == fill.den
            ));
            alpha::assert_training_move_demo(&mut session, name, slot, false);
        }
    }
}

#[test]
fn ex_special_spends_one_stock_and_focus_cancels_spend_two_or_three() {
    let Some(select) = select(Revision::Usf4, Loadout::Ultra1, TournamentFighter::Ryu) else {
        return;
    };
    let assets = select.assets();
    let base = bout(&select);
    for (red, focus_cost) in [(false, 2 * STOCK_AT), (true, 3 * STOCK_AT)] {
        let mut fight = base.clone();
        set_super(&mut fight, STOCK_AT + focus_cost);
        for direction in [DOWN, DOWN | RIGHT] {
            step(&mut fight, assets, pad(direction, None), pad(0, None));
        }
        step(&mut fight, assets, held(RIGHT, &[0, MP]), pad(0, None));
        assert_eq!(p1_slot(&fight), 0x1a);
        assert_eq!(fight.machine().era_fighters()[0].super_meter, focus_cost);
        if red {
            let mut underfunded = fight.clone();
            set_super(&mut underfunded, 2 * STOCK_AT);
            run_focus(&mut underfunded, assets, true, 1, pad(0, None));
            assert_eq!(p1_slot(&underfunded), 0x1a, "two stocks cannot EX Red Focus cancel");
            assert_eq!(underfunded.machine().era_fighters()[0].super_meter, 2 * STOCK_AT);
        }
        run_focus(&mut fight, assets, red, 1, pad(0, None));
        assert_eq!(p1_slot(&fight), FOCUS_CHARGE, "red={red}");
        assert_eq!(fight.machine().era_fighters()[0].super_meter, 0);
        if red {
            let mut released = fight.clone();
            step(&mut released, assets, held(0, &[MP, MK]), pad(0, None));
            assert_eq!(p1_slot(&released), FOCUS, "releasing LP ends Red Focus charge");
        }
        let buttons: &[usize] = if red { &[0, MP, MK] } else { &[MP, MK] };
        for direction in [RIGHT, 0, RIGHT] {
            step(&mut fight, assets, held(direction, buttons), pad(0, None));
        }
        assert_eq!(p1_slot(&fight), DASH);
    }
}

#[test]
fn gen_uses_the_selected_ultra_from_crane_stance() {
    let Some(select) = local_select_with_fighter(
        GaugeMode::Unified,
        TournamentFighter::Gen,
        Revision::Usf4,
        Loadout::Ultra1,
    ) else {
        return;
    };
    let mut fight = bout(&select);
    let assets = select.assets();
    step(&mut fight, assets, held(0, &[3, 4, 5]), pad(0, None));
    assert_eq!(p1_slot(&fight), 0x1f, "three kicks select Crane");
    for _ in 0..60 {
        step(&mut fight, assets, pad(0, None), pad(0, None));
        if ready(&fight.fight_read().fighters[0]) {
            break;
        }
    }
    set_super(&mut fight, 3 * STOCK_AT);
    for direction in [DOWN, DOWN | RIGHT, RIGHT, DOWN, DOWN | RIGHT] {
        step(&mut fight, assets, pad(direction, None), pad(0, None));
    }
    step(&mut fight, assets, held(RIGHT, &[3, 4, 5]), pad(0, None));
    assert_eq!(p1_slot(&fight), 0x2d);
    assert!(fight
        .callouts()
        .frame()
        .side(Side::P1)
        .words
        .contains(&Some(CalloutKind::Super { name: "RYUKOHA" })));
    assert_eq!(fight.machine().era_fighters()[0].super_meter, 0);
}
