//! Native SF6 Drive and Super Art checks.

#[path = "support/alpha.rs"]
mod alpha;
use alpha::{local_fight, local_select_with_fighter, local_select_with_p1, pad, DOWN, LEFT, RIGHT};
use sf_engine::era::{GaugeMode, Loadout, Revision};
use sf_engine::front::fight::Fight;
use sf_engine::front::{FrameInputs, GameSession, PlayerInputs};
use sf_engine::moves::{ButtonClass, Dir, MoveInput, Strength};
use sf_engine::roster::TournamentFighter;
use sf_video::announcer::AnnouncerLine;
use sf_video::callouts::{CalloutKind, Side};
use sf_video::era_hud::{GaugeRow, GaugeState};
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
    assert!(matches!(
        fight.era_hud().p1.unwrap().main,
        GaugeRow::Stock { cells: 6, state: GaugeState::Greyed, .. }
    ));
    rest(&mut fight, assets, 450);
    assert_eq!(drive(&fight), 288);
}

#[test]
fn super_art_meter_carries_while_drive_resets_for_round_two() {
    let Some(session) =
        local_select_with_p1(GaugeMode::EraDefault, Revision::Sf6, Loadout::Standard)
    else {
        return;
    };
    let assets = session.assets();
    let mut fight = bout(&session);
    meter(&mut fight, 96);
    fight.step(assets, pair(2, 5), IDLE, false);
    assert_eq!(drive(&fight), 240);
    alpha::round_two_after_timeout(&mut fight, assets);
    assert_eq!(fight.machine().era_fighters()[0].super_meter, 96);
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

    let mut disabled = bout(&session);
    let mut snapshot = disabled.save_netplay_snapshot(0);
    snapshot.setup.config.disable_raw_drive_rush = true;
    snapshot.machine.match_config.disable_raw_drive_rush = true;
    disabled.restore_netplay_snapshot(&snapshot);
    for pad in input.frames(true, Strength::Medium) {
        disabled.step(assets, pad, IDLE, false);
        assert_ne!(disabled.fight_read().fighters[0].move_id, 0x2c);
    }
}

#[test]
fn cancel_rush_spends_three_bars_and_adds_four_frames_to_a_landed_normal() {
    let Some(session) =
        local_select_with_p1(GaugeMode::EraDefault, Revision::Sf6, Loadout::Standard)
    else {
        return;
    };
    let assets = session.assets();
    let mut fight = bout(&session);
    for _ in 0..100 {
        let position = fight.machine().diagnostic_telemetry();
        if position.p1.center_x.abs_diff(position.p2.center_x) <= 30 {
            break;
        }
        fight.step(assets, pad(RIGHT, None), IDLE, false);
    }
    rest(&mut fight, assets, 8);
    fight.step(assets, pad(0, Some(1)), IDLE, false);
    assert!((0..35).any(|_| {
        fight.step(assets, IDLE, IDLE, false);
        fight.machine().toggle_state().cancel.window[0] != 0
    }), "the first medium punch opens a real cancel window");
    fight.step(assets, pair(1, 4), IDLE, false);
    assert_eq!(fight.fight_read().fighters[0].move_id, 0x2c);
    assert_eq!(drive(&fight), 144);

    assert!((0..18).any(|_| {
        fight.step(assets, IDLE, IDLE, false);
        fight.fight_read().fighters[0].move_id != 0x2c
    }), "Drive Rush ends in time for a follow-up");
    let before = fight.machine().era_fighters()[1].sub_hp;
    fight.step(assets, pad(0, Some(1)), IDLE, false);
    let follow_up = fight.fight_read().fighters[0].move_id;
    assert!((0..20).any(|_| {
        fight.step(assets, IDLE, IDLE, false);
        fight.machine().era_fighters()[1].sub_hp < before
    }), "the follow-up medium punch lands");
    let ordinary = sf_engine::combo::move_stun(Some(TournamentFighter::Ryu), follow_up, 2).hit;
    assert_eq!(fight.machine().toggle_state().combo.hitstun[1], ordinary + 4);
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
fn higher_super_arts_require_their_full_stock_cost() {
    let Some(session) =
        local_select_with_p1(GaugeMode::EraDefault, Revision::Sf6, Loadout::Standard)
    else {
        return;
    };
    let assets = session.assets();
    let base = bout(&session);
    let forward = [DOWN, DOWN | RIGHT, RIGHT, DOWN, DOWN | RIGHT, RIGHT];
    let back = [DOWN, DOWN | LEFT, LEFT, DOWN, DOWN | LEFT, LEFT];
    for (slot, motion, button, cost) in [(0x2a, back, 0, 96), (0x2b, forward, 3, 144)] {
        let mut short = base.clone();
        meter(&mut short, cost - 1);
        attack(&mut short, assets, &motion, button);
        assert_ne!(short.fight_read().fighters[0].move_id, slot);
        assert!(short.machine().era_fighters()[0].super_meter >= cost - 1);

        let mut funded = base.clone();
        meter(&mut funded, cost);
        attack(&mut funded, assets, &motion, button);
        assert_eq!(funded.fight_read().fighters[0].move_id, slot);
        assert_eq!(funded.machine().era_fighters()[0].super_meter, 0);
    }
}

#[test]
fn the_critical_art_threshold_includes_exactly_one_quarter_health() {
    let Some(session) =
        local_select_with_p1(GaugeMode::EraDefault, Revision::Sf6, Loadout::Standard)
    else {
        return;
    };
    let assets = session.assets();
    let base = bout(&session);
    let threshold = base.machine().era_fighters()[0].max_sub_hp / 4;
    for (health, disable_comeback, name) in [
        (threshold + 1, false, "SHIN SHORYUKEN"),
        (threshold, false, "CRITICAL ART"),
        (threshold, true, "SHIN SHORYUKEN"),
    ] {
        let mut fight = base.clone();
        let mut snapshot = fight.save_netplay_snapshot(0);
        snapshot.setup.config.disable_comeback_scaling = disable_comeback;
        snapshot.machine.match_config.disable_comeback_scaling = disable_comeback;
        let state = &mut snapshot.machine.toggle_state.era.fighters[0];
        state.sub_hp = health;
        state.super_meter = 144;
        fight.restore_netplay_snapshot(&snapshot);
        attack(&mut fight, assets, &[DOWN, DOWN | RIGHT, RIGHT, DOWN, DOWN | RIGHT, RIGHT], 3);
        assert_eq!(fight.fight_read().fighters[0].move_id, 0x2b);
        assert!(fight.callouts().frame().side(Side::P1).words.contains(&Some(CalloutKind::Super { name })));
    }
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
fn training_fills_and_coaches_each_canonical_sf6_fighter() {
    use TournamentFighter::{Ken, Retsu, Ryu};

    for (fighter, name, slot) in [
        (Ryu, "SHINKU HADOKEN", 0x29),
        (Ken, "DRAGONLASH FLAME", 0x29),
        (Ken, "SHIPPU JINRAIKYAKU", 0x2a),
        (Retsu, "TENCHI SHORINJI DROP", 0x2b),
    ] {
        let Some(mut session) =
            alpha::training_fight_with_fighter(fighter, Revision::Sf6, Loadout::Standard)
        else {
            return;
        };
        session.step(FrameInputs::default());
        let fight = session.fight().unwrap();
        assert_eq!(drive(fight), 288, "{fighter:?} full Drive");
        assert_eq!(fight.machine().era_fighters()[0].super_meter, 144);
        alpha::assert_training_move_demo(&mut session, name, slot, false);
    }
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
fn an_sf6_punish_counter_drains_drive_even_without_drive_impact() {
    let Some(select) =
        local_select_with_p1(GaugeMode::EraDefault, Revision::Sf6, Loadout::Standard)
    else {
        return;
    };
    let session = local_fight(&select, Revision::Sf6, Loadout::Standard);
    let assets = session.assets();
    let mut base = session.fight().unwrap().clone();
    for _ in 0..65 {
        base.step(assets, pad(LEFT, None), pad(RIGHT, None), false);
    }
    rest(&mut base, assets, 5);
    attack(&mut base, assets, &[DOWN, DOWN | RIGHT, RIGHT], 0);
    let initial_hp = base.machine().era_fighters()[1].sub_hp;
    let mut idle_defender = base.clone();
    let contact_frame = (0_u8..100)
        .find(|_| {
            idle_defender.step(assets, IDLE, IDLE, false);
            idle_defender.machine().era_fighters()[1].sub_hp < initial_hp
        })
        .expect("the Hadoken reaches the defender");
    assert_eq!(idle_defender.machine().era_fighters()[1].sf6_drive_points(), Some(288));

    let mut startup = base.clone();
    let start_frame = contact_frame.saturating_sub(4);
    assert!((0..=contact_frame + 12).any(|frame| {
        let p2 = if frame == start_frame { pad(0, Some(2)) } else { IDLE };
        let before = startup.fight_read().fighters[1];
        let output = startup.step(assets, IDLE, p2, false);
        if startup.machine().era_fighters()[1].sub_hp == initial_hp {
            return false;
        }
        assert!(before.attacking() && !before.attack_active, "hit interrupts startup");
        assert!(!output.announcements.contains(&AnnouncerLine::PunishCounter));
        assert_eq!(startup.machine().era_fighters()[1].sf6_drive_points(), Some(288));
        true
    }), "Hadoken also reaches an opponent during startup");

    let whiff = contact_frame.checked_sub(39).expect("room to whiff a standing heavy kick");
    let mut fight = base;
    let mut was_active = false;
    for frame in 0..=contact_frame + 20 {
        let before = fight.fight_read().fighters[1];
        was_active |= before.attack_active;
        let p2 = if frame == whiff { pad(0, Some(5)) } else { IDLE };
        let output = fight.step(assets, IDLE, p2, false);
        if fight.machine().era_fighters()[1].sub_hp < initial_hp {
            assert!(was_active && before.attacking() && !before.attack_active);
            assert!(output.announcements.contains(&AnnouncerLine::PunishCounter));
            assert_eq!(fight.machine().era_fighters()[1].sf6_drive_points(), Some(240));
            return;
        }
    }
    panic!("the projectile never caught the end of the defender's whiff");
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
        (0x29, back, 3, 48),
        (0x29, back, 4, 48),
        (0x29, back, 5, 48),
        (0x2a, forward, 3, 96),
        (0x2a, forward, 4, 96),
        (0x2a, forward, 5, 96),
        (0x2b, forward, 0, 144),
    ] {
        let mut fight = base.clone();
        meter(&mut fight, 144);
        attack(&mut fight, assets, &motion, button);
        assert_eq!(fight.fight_read().fighters[0].move_id, slot);
        assert_eq!(fight.machine().era_fighters()[0].super_meter, 144 - spent);
    }
    for (motion, button, starting_meter) in [(forward, 3, 48), (back, 0, 144)] {
        let mut fight = base.clone();
        meter(&mut fight, starting_meter);
        attack(&mut fight, assets, &motion, button);
        assert!(!matches!(fight.fight_read().fighters[0].move_id, 0x29..=0x2b));
        assert_eq!(fight.machine().era_fighters()[0].super_meter, starting_meter);
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
