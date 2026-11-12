//! Native SF6 Drive and Super Art checks.

#[path = "support/alpha.rs"]
mod alpha;
use alpha::{local_fight, local_select_with_fighter, local_select_with_p1, pad, DOWN, LEFT, RIGHT};
use sf_engine::era::{GaugeMode, Loadout, Revision};
use sf_engine::front::fight::Fight;
use sf_engine::front::{GameSession, PlayerInputs};
use sf_engine::moves::{ButtonClass, Dir, MoveInput, Strength};
use sf_engine::roster::TournamentFighter;
use sf_video::announcer::AnnouncerLine;
use sf_video::romset::Assets;

const IDLE: PlayerInputs = PlayerInputs {
    directions: [false; 4],
    buttons: [false; 6],
};

fn bout(session: &GameSession) -> Fight {
    local_fight(session, Revision::Sf1Plus, Loadout::Standard)
        .fight()
        .expect("local fight")
        .clone()
}

fn pair(punch: usize, kick: usize) -> PlayerInputs {
    let mut input = IDLE;
    input.buttons[punch] = true;
    input.buttons[kick] = true;
    input
}

fn rest(fight: &mut Fight, assets: &Assets, ticks: usize) {
    for _ in 0..ticks {
        fight.step(assets, IDLE, IDLE, false);
    }
}

fn drive(fight: &Fight) -> u16 {
    fight.machine().era_fighters()[0]
        .sf6_drive_points()
        .expect("SF6 is selected")
}

fn meter(fight: &mut Fight, value: u8) {
    let mut snapshot = fight.save_netplay_snapshot(0);
    snapshot.machine.toggle_state.era.fighters[0].super_meter = value;
    fight.restore_netplay_snapshot(&snapshot);
}

#[test]
fn drive_impact_spends_one_bar_and_burnout_recovers_to_six() {
    let Some(session) =
        local_select_with_p1(GaugeMode::EraDefault, Revision::Sf6, Loadout::Standard)
    else {
        return;
    };
    let assets = session.assets();
    let mut fight = bout(&session);
    assert_eq!(drive(&fight), 288);
    for remaining in (0..6).rev() {
        let output = fight.step(assets, pair(2, 5), IDLE, false);
        assert_eq!(fight.fight_read().fighters[0].move_id, 0x2f);
        assert_eq!(drive(&fight), remaining * 48);
        assert_eq!(
            output.announcements.contains(&AnnouncerLine::Burnout),
            remaining == 0
        );
        rest(&mut fight, assets, 40);
    }
    assert_eq!(drive(&fight), 0);
    rest(&mut fight, assets, 450);
    assert_eq!(drive(&fight), 288);
}

#[test]
fn raw_rush_uses_a_forward_double_tap_and_advances() {
    let Some(session) =
        local_select_with_p1(GaugeMode::EraDefault, Revision::Sf6, Loadout::Standard)
    else {
        return;
    };
    let assets = session.assets();
    let mut fight = bout(&session);
    let start = fight.machine().diagnostic_telemetry().p1.center_x;
    let input = MoveInput {
        motion: &[Dir::Forward, Dir::Neutral, Dir::Forward],
        button: ButtonClass::MediumPair,
    };
    let mut started = false;
    for pad in input.frames(true, Strength::Medium) {
        fight.step(assets, pad, IDLE, false);
        started |= fight.fight_read().fighters[0].move_id == 0x2c;
    }
    assert!(started, "the Show input starts Rush");
    assert_eq!(drive(&fight), 240);
    assert!(fight.machine().diagnostic_telemetry().p1.center_x >= start + 30);
}

fn attack(fight: &mut Fight, assets: &Assets, directions: &[u8], button: usize) {
    for (i, &direction) in directions.iter().enumerate() {
        fight.step(
            assets,
            pad(direction, (i + 1 == directions.len()).then_some(button)),
            IDLE,
            false,
        );
    }
}

#[test]
fn ryu_can_use_all_three_super_arts_and_overdrive() {
    let Some(session) =
        local_select_with_p1(GaugeMode::EraDefault, Revision::Sf6, Loadout::Standard)
    else {
        return;
    };
    let assets = session.assets();
    let base = bout(&session);
    let forward = [DOWN, DOWN | RIGHT, RIGHT, DOWN, DOWN | RIGHT, RIGHT];
    let back = [DOWN, DOWN | LEFT, LEFT, DOWN, DOWN | LEFT, LEFT];
    for (slot, motion, button, spent) in [
        (0x29, forward, 0, 48),
        (0x2a, back, 0, 96),
        (0x2b, forward, 3, 144),
    ] {
        let mut fight = base.clone();
        meter(&mut fight, 144);
        attack(&mut fight, assets, &motion, button);
        assert_eq!(fight.fight_read().fighters[0].move_id, slot);
        assert_eq!(fight.machine().era_fighters()[0].super_meter, 144 - spent);
        assert_eq!(drive(&fight), 288);
    }
    let mut fight = base;
    fight.step(assets, pad(DOWN, None), IDLE, false);
    fight.step(assets, pad(DOWN | RIGHT, None), IDLE, false);
    let mut od = pad(RIGHT, Some(0));
    od.buttons[1] = true;
    fight.step(assets, od, IDLE, false);
    assert_eq!(fight.fight_read().fighters[0].move_id, 0x24);
    assert_eq!(drive(&fight), 192);
    for _ in 0..32 {
        fight.step(assets, IDLE, IDLE, false);
        if fight
            .machine()
            .toggle_state()
            .era
            .projectiles()
            .iter()
            .any(|p| p.hits_remaining == 2)
        {
            return;
        }
    }
    panic!("OD Hadoken never produced its two-hit projectile");
}

#[test]
fn retsu_uses_native_shot_and_arts_with_the_drive_gauge() {
    let Some(session) = local_select_with_fighter(
        GaugeMode::EraDefault,
        TournamentFighter::Retsu,
        Revision::Sf6,
        Loadout::Standard,
    ) else {
        return;
    };
    let mut fight = bout(&session);
    assert_eq!(drive(&fight), 288);
    meter(&mut fight, 144);
    attack(
        &mut fight,
        session.assets(),
        &[DOWN, DOWN | RIGHT, RIGHT, DOWN, DOWN | RIGHT, RIGHT],
        3,
    );
    assert_eq!(fight.fight_read().fighters[0].move_id, 0x2b);
    assert_eq!(fight.machine().era_fighters()[0].super_meter, 0);
}

#[test]
fn a_fresh_drive_parry_negates_actual_melee_contact_and_announces_perfect_parry() {
    let Some(session) =
        local_select_with_p1(GaugeMode::EraDefault, Revision::Sf6, Loadout::Standard)
    else {
        return;
    };
    let assets = session.assets();
    let mut fight = bout(&session);
    for _ in 0..100 {
        let pair = fight.machine().diagnostic_telemetry();
        if pair.p2.center_x.abs_diff(pair.p1.center_x) <= 38 {
            break;
        }
        fight.step(assets, pad(RIGHT, None), IDLE, false);
    }
    rest(&mut fight, assets, 8);
    let hp = fight.machine().era_fighters()[0].sub_hp;
    fight.step(assets, IDLE, pad(0, Some(5)), false);
    for _ in 0..25 {
        let mut parry = fight.clone();
        fight.step(assets, IDLE, IDLE, false);
        if fight.machine().era_fighters()[0].sub_hp == hp {
            continue;
        }
        let output = parry.step(assets, pair(1, 4), IDLE, false);
        assert_eq!(parry.machine().era_fighters()[0].sub_hp, hp);
        assert!(output.announcements.contains(&AnnouncerLine::PerfectParry));
        assert_eq!(drive(&parry), 288);
        return;
    }
    panic!("Ken's standing kick never reached Ryu");
}

#[test]
fn drive_reversal_cancels_real_ground_blockstun_and_spends_two_bars() {
    let Some(session) =
        local_select_with_p1(GaugeMode::EraDefault, Revision::Sf6, Loadout::Standard)
    else {
        return;
    };
    let assets = session.assets();
    let mut fight = bout(&session);
    for _ in 0..100 {
        let pair = fight.machine().diagnostic_telemetry();
        if pair.p2.center_x.abs_diff(pair.p1.center_x) <= 38 {
            break;
        }
        fight.step(assets, pad(RIGHT, None), IDLE, false);
    }
    rest(&mut fight, assets, 8);
    let back = pad(LEFT, None);
    fight.step(assets, back, pad(0, Some(5)), false);
    for _ in 0..27 {
        if fight.machine().toggle_state().combo.blockstun[0] != 0 {
            break;
        }
        fight.step(assets, back, IDLE, false);
    }
    assert_ne!(fight.machine().toggle_state().combo.blockstun[0], 0);
    let before = drive(&fight);
    let mut reversal = pair(2, 5);
    reversal.directions[0] = true;
    fight.step(assets, reversal, IDLE, false);
    assert_eq!(fight.fight_read().fighters[0].move_id, 0x2e);
    assert_eq!(fight.machine().toggle_state().combo.blockstun[0], 0);
    assert_eq!(drive(&fight), before - 96);
}

#[test]
fn drive_impact_armor_holds_through_two_hits_of_an_overdrive_projectile() {
    let Some(session) =
        local_select_with_p1(GaugeMode::EraDefault, Revision::Sf6, Loadout::Standard)
    else {
        return;
    };
    let session = local_fight(&session, Revision::Sf6, Loadout::Standard);
    let assets = session.assets();
    let mut fight = session.fight().expect("local fight").clone();
    fight.step(assets, IDLE, pad(DOWN, None), false);
    fight.step(assets, IDLE, pad(DOWN | LEFT, None), false);
    let mut od = pad(LEFT, Some(0));
    od.buttons[1] = true;
    fight.step(assets, IDLE, od, false);
    assert_eq!(fight.fight_read().fighters[1].move_id, 0x24);
    let hp = fight.machine().era_fighters()[0].sub_hp;
    for _ in 0..60 {
        let mut impact = fight.clone();
        fight.step(assets, IDLE, IDLE, false);
        if fight.machine().era_fighters()[0].sub_hp == hp {
            continue;
        }
        impact.step(assets, pair(2, 5), IDLE, false);
        let mut hits = 0;
        let mut previous = hp;
        for _ in 0..10 {
            let current = impact.machine().era_fighters()[0].sub_hp;
            if current < previous {
                hits += 1;
                assert_eq!(impact.fight_read().fighters[0].move_id, 0x2f);
            }
            previous = current;
            impact.step(assets, IDLE, IDLE, false);
        }
        assert_eq!(hits, 2);
        return;
    }
    panic!("Ken's overdrive projectile never reached Ryu");
}

#[test]
fn a_blocked_drive_impact_at_the_wall_keeps_the_defender_in_stun() {
    let Some(session) =
        local_select_with_p1(GaugeMode::EraDefault, Revision::Sf6, Loadout::Standard)
    else {
        return;
    };
    let session = local_fight(&session, Revision::Sf6, Loadout::Standard);
    let assets = session.assets();
    let mut fight = session.fight().expect("local fight").clone();
    let left = pad(LEFT, None);
    for _ in 0..140 {
        let pair = fight.machine().diagnostic_telemetry();
        if pair.p1.center_x <= 0x86 && pair.p2.center_x.abs_diff(pair.p1.center_x) <= 52 {
            break;
        }
        fight.step(assets, left, left, false);
    }
    let pair_pos = fight.machine().diagnostic_telemetry();
    assert!(pair_pos.p1.center_x <= 0x86, "P1 reached the left wall");
    fight.step(assets, left, pair(2, 5), false);
    let hp = fight.machine().era_fighters()[0].sub_hp;
    for _ in 0..40 {
        fight.step(assets, left, IDLE, false);
        if fight.fight_read().fighters[0].reacting() {
            assert_eq!(fight.machine().era_fighters()[0].sub_hp, hp);
            for _ in 0..8 {
                fight.step(assets, pad(0, Some(0)), IDLE, false);
                assert!(!fight.fight_read().fighters[0].attacking());
            }
            return;
        }
    }
    panic!("blocked Drive Impact did not cause a wall stun");
}

#[test]
fn ken_can_start_his_three_arts_and_jinrai_branches() {
    let Some(session) = local_select_with_fighter(
        GaugeMode::EraDefault,
        TournamentFighter::Ken,
        Revision::Sf6,
        Loadout::Standard,
    ) else {
        return;
    };
    let assets = session.assets();
    let base = bout(&session);
    let forward = [DOWN, DOWN | RIGHT, RIGHT, DOWN, DOWN | RIGHT, RIGHT];
    let back = [DOWN, DOWN | LEFT, LEFT, DOWN, DOWN | LEFT, LEFT];
    for (slot, motion, button, spent) in [
        (0x29, forward, 3, 48),
        (0x2a, back, 3, 96),
        (0x2b, forward, 0, 144),
    ] {
        let mut fight = base.clone();
        meter(&mut fight, 144);
        attack(&mut fight, assets, &motion, button);
        assert_eq!(fight.fight_read().fighters[0].move_id, slot);
        assert_eq!(fight.machine().era_fighters()[0].super_meter, 144 - spent);
    }
    for (direction, slot) in [(DOWN, 0x23), (0, 0x1e), (RIGHT, 0x1d)] {
        let mut fight = base.clone();
        attack(&mut fight, assets, &[DOWN, DOWN | RIGHT, RIGHT], 3);
        assert_eq!(fight.fight_read().fighters[0].move_id, 0x21);
        rest(&mut fight, assets, 6);
        fight.step(assets, pad(direction, Some(3)), IDLE, false);
        assert_eq!(fight.fight_read().fighters[0].move_id, slot);
        assert!(fight.fight_read().fighters[0].attacking());
    }
}

#[test]
fn unified_overdrive_costs_one_stock_and_has_no_drive_row() {
    let Some(session) = local_select_with_p1(GaugeMode::Unified, Revision::Sf6, Loadout::Standard)
    else {
        return;
    };
    let assets = session.assets();
    let mut fight = bout(&session);
    meter(&mut fight, 48);
    fight.step(assets, pad(DOWN, None), IDLE, false);
    fight.step(assets, pad(DOWN | RIGHT, None), IDLE, false);
    let mut od = pad(RIGHT, Some(0));
    od.buttons[1] = true;
    fight.step(assets, od, IDLE, false);
    let state = fight.machine().era_fighters()[0];
    assert_eq!(fight.fight_read().fighters[0].move_id, 0x24);
    assert_eq!(state.super_meter, 0);
    assert!(state.gauge().unwrap().second.is_none());
}
