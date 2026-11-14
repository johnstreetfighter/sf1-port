//! Controller-input witnesses for World Super Turbo Ryu throws (#141).

#[path = "support/alpha.rs"]
mod support;

use sf_engine::combo::throw::{THROWN_MOVE, THROW_MOVE};
use sf_engine::data_bank::GROUND;
use sf_engine::era::{GaugeMode, Loadout, Revision};
use sf_engine::fair_cpu::FIGHT_LOOP;
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::fight::Fight;
use sf_engine::front::{FrameInputs, GameSession, PlayerInputs, Transition};
use sf_engine::overlay::FighterTelemetry;
use sf_engine::roster::TournamentFighter;
use sf_video::callouts::Side;
use sf_video::romset::Assets;
use support::{
    local_fight, local_select_with_fighter, local_select_with_p1, pad, LEFT, LP, RIGHT, UP,
};

const MP: usize = 1;
const HP: usize = 2;
const LK: usize = 3;
const MK: usize = 4;
const HK: usize = 5;
const LOADOUTS: [Loadout; 2] = [Loadout::Standard, Loadout::Old];

#[derive(Clone, Copy)]
struct ThrowTiming {
    release: u32,
    attacker_neutral: u32,
    full_damage: u32,
    defender_recovery: u32,
    soft_land: u32,
    soft_neutral: u32,
    release_height: i32,
}

fn timing(button: usize) -> ThrowTiming {
    // World 940223, Turbo 2. See docs/research/ryu-st-turbo2-throw-evidence.md.
    if matches!(button, MP | HP) {
        ThrowTiming {
            release: 44,
            attacker_neutral: 74,
            full_damage: 79,
            defender_recovery: 153,
            soft_land: 94,
            soft_neutral: 100,
            release_height: 44,
        }
    } else {
        ThrowTiming {
            release: 38,
            attacker_neutral: 80,
            full_damage: 71,
            defender_recovery: 144,
            soft_land: 87,
            soft_neutral: 93,
            release_height: 32,
        }
    }
}

fn camera_x(fight: &Fight) -> i32 {
    i32::from(u16::from_be_bytes(
        fight.machine().work_ram()[0x400c..0x400e]
            .try_into()
            .unwrap(),
    ))
}

fn axis_x(fight: &Fight, fighter: FighterTelemetry) -> i32 {
    let objects = &fight.machine().state.objectram;
    let pose_x = objects[usize::from(fighter.entity_offset) + 0x26] as i8;
    camera_x(fight) + fighter.center_x - i32::from(pose_x)
}

fn height(fight: &Fight, fighter: FighterTelemetry) -> i32 {
    let entity = usize::from(fighter.entity_offset);
    let objects = &fight.machine().state.objectram;
    let attr = u16::from_be_bytes(objects[entity + 2..entity + 4].try_into().unwrap());
    let tile = if attr & 0x400 != 0 { 32 } else { 16 };
    let top = u16::from_be_bytes(objects[entity + 4..entity + 6].try_into().unwrap());
    GROUND - (i32::from(top) + tile * (i32::from(objects[entity + 0xc]) + 1))
}

fn ready(fight: &mut Fight, assets: &Assets) {
    for _ in 0..94 {
        step(fight, assets, [PlayerInputs::default(); 2]);
    }
}

fn at_gap(fight: Fight, gap: i32) -> Fight {
    at_axis(fight, 192, gap)
}

fn at_axis(mut fight: Fight, p1_screen_x: i32, gap: i32) -> Fight {
    // Match the source axis fixture by moving only the native sprite grids.
    let read = fight.machine().diagnostic_telemetry();
    let mut snapshot = fight.save_netplay_snapshot(0);
    let anchor = camera_x(&fight) + p1_screen_x;
    for (fighter, target) in [(read.p1, anchor), (read.p2, anchor + gap)] {
        let entity = usize::from(fighter.entity_offset);
        let objects = &mut snapshot.machine.state.objectram;
        let cells =
            (usize::from(objects[entity + 0xc]) + 1) * (usize::from(objects[entity + 0xd]) + 1);
        let dx = (target - axis_x(&fight, fighter)) as u16;
        for cell in 0..cells {
            let x = entity + cell * 0x40 + 6;
            let value = u16::from_be_bytes(objects[x..x + 2].try_into().unwrap()).wrapping_add(dx);
            objects[x..x + 2].copy_from_slice(&value.to_be_bytes());
        }
    }
    fight.restore_netplay_snapshot(&snapshot);
    let read = fight.machine().diagnostic_telemetry();
    assert_eq!(axis_x(&fight, read.p2) - axis_x(&fight, read.p1), gap);
    fight
}

fn step(fight: &mut Fight, assets: &Assets, input: [PlayerInputs; 2]) {
    fight.step(assets, input[0], input[1], false);
}

fn replay_step(fight: &mut Fight, assets: &Assets, elapsed: u32, input: [PlayerInputs; 2]) {
    let before = fight.save_netplay_snapshot(elapsed);
    step(fight, assets, input);
    let after = fight.save_netplay_snapshot(elapsed + 1);
    let checksum = fight.state_checksum();
    fight.restore_netplay_snapshot(&before);
    step(fight, assets, input);
    assert_eq!(
        fight.state_checksum(),
        checksum,
        "throw replay at +{elapsed}"
    );
    assert!(
        fight.save_netplay_snapshot(elapsed + 1) == after,
        "throw replay at +{elapsed}"
    );
}

fn tap(session: &mut GameSession, input: FrameInputs) -> Option<Transition> {
    let transition = session.step(input).transition;
    session.step(FrameInputs::default());
    transition
}

fn local_fight_with_p2_ryu(mut session: GameSession, loadout: Loadout) -> GameSession {
    let p2 = |dir, button| FrameInputs {
        p2: pad(dir, button),
        ..FrameInputs::default()
    };
    tap(&mut session, p2(LEFT, None));
    assert_eq!(
        tap(&mut session, p2(0, Some(LP))),
        Some(Transition::StyleOpened)
    );
    tap(&mut session, p2(RIGHT, None));
    if loadout == Loadout::Old {
        tap(&mut session, p2(UP, None));
        tap(&mut session, p2(RIGHT, None));
    }
    assert!(matches!(
        tap(&mut session, p2(0, Some(LP))),
        Some(Transition::Fighters(_, _))
    ));
    for _ in 0..60 {
        if session.screen() == "country select" {
            break;
        }
        session.step(FrameInputs::default());
    }
    assert_eq!(session.screen(), "country select");
    tap(
        &mut session,
        FrameInputs {
            p1: pad(0, Some(LP)),
            ..FrameInputs::default()
        },
    );
    for _ in 0..1500 {
        if let Some(machine) = session
            .fight_machine()
            .filter(|machine| flow_position(machine) == FIGHT_LOOP)
        {
            let [p1, p2] = machine.era_fighters();
            assert_eq!(p1.fighter, Some(TournamentFighter::Ken));
            assert_eq!(p2.fighter, Some(TournamentFighter::Ryu));
            assert_eq!(
                (p2.style.revision, p2.style.loadout),
                (Revision::St, loadout)
            );
            return session;
        }
        session.step(FrameInputs::default());
    }
    panic!("the native fight loop did not open");
}

fn check_ryu_throw(
    mut fight: Fight,
    assets: &Assets,
    button: usize,
    back: bool,
    response: Option<(u32, PlayerInputs)>,
    softened: bool,
) -> u16 {
    let expected = timing(button);
    let origin = axis_x(&fight, fight.machine().diagnostic_telemetry().p1);
    let initial_hp = fight.machine().era_fighters()[1].sub_hp;
    let damage_tick = if softened {
        expected.release
    } else {
        expected.full_damage
    };
    let defender_recovery = if softened {
        expected.soft_neutral
    } else {
        expected.defender_recovery
    };
    let mut damage = 0;
    for elapsed in 0..=defender_recovery + 1 {
        let mut input = [PlayerInputs::default(); 2];
        if elapsed == 0 {
            input[0] = pad(if back { LEFT } else { RIGHT }, Some(button));
        }
        if let Some((tick, response)) = response {
            if elapsed == tick {
                input[1] = response;
            }
        }
        if elapsed == defender_recovery + 1 {
            input[1] = pad(RIGHT, Some(LP));
        }
        if [0, 13, expected.release, damage_tick].contains(&elapsed) {
            replay_step(&mut fight, assets, elapsed, input);
        } else {
            step(&mut fight, assets, input);
        }

        let fighters = fight.fight_read().fighters;
        let read = fight.machine().diagnostic_telemetry();
        let lost = initial_hp - fight.machine().era_fighters()[1].sub_hp;
        if elapsed == 0 {
            assert_eq!(fighters[0].move_id, THROW_MOVE);
            assert_eq!(fighters[1].move_id, THROWN_MOVE);
        }
        assert_eq!(
            fight
                .machine()
                .toggle_state()
                .throw
                .st_ryu_capture_active(1),
            elapsed < expected.release,
            "Ken capture at +{elapsed}"
        );
        assert_eq!(
            fighters[0].move_id == 1,
            elapsed >= expected.attacker_neutral,
            "Ryu recovery at +{elapsed}"
        );
        if elapsed < defender_recovery {
            assert_ne!(fighters[1].move_id, 1, "Ken still recovering at +{elapsed}");
        } else if elapsed == defender_recovery {
            assert!(
                !fighters[1].getting_up,
                "Ken finishes recovery at +{elapsed}"
            );
        } else {
            assert!(fighters[1].attacking(), "Ken can jab at +{elapsed}");
        }
        if elapsed == expected.release {
            let release_dx = if back { -127 } else { 127 };
            assert_eq!(axis_x(&fight, read.p2) - origin, release_dx);
            assert_eq!(height(&fight, read.p2), expected.release_height);
        }
        if softened && elapsed > expected.release && elapsed <= expected.soft_land {
            assert_eq!(
                height(&fight, read.p2) > 0,
                elapsed < expected.soft_land,
                "airborne recovery at +{elapsed}"
            );
        }
        if elapsed == damage_tick {
            assert_eq!(
                lost,
                if softened { 107 } else { 214 },
                "first throw damage at +{elapsed}"
            );
            if !softened {
                assert_eq!(height(&fight, read.p2), 0, "full damage on landing");
            }
            assert!(fight.fight_read().landed_by(Side::P1).is_some());
            damage = lost;
        }
        assert_eq!(lost, damage, "one damage event at +{damage_tick}");
    }
    assert!(fight.machine().unrecovered().is_empty());
    damage
}

#[test]
fn st_ryu_uses_both_throw_families_in_both_directions() {
    for loadout in LOADOUTS {
        let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, loadout)
        else {
            return;
        };
        let session = local_fight(&select, Revision::St, Loadout::Standard);
        let assets = session.assets();
        let mut base = session.fight().unwrap().clone();
        ready(&mut base, assets);
        let close = at_gap(base.clone(), 32);
        for button in [MP, HP, MK, HK] {
            for back in [false, true] {
                check_ryu_throw(close.clone(), assets, button, back, None, false);
                for gap in [77, 78] {
                    let mut fight = at_gap(base.clone(), gap);
                    let direction = if back { LEFT } else { RIGHT };
                    step(
                        &mut fight,
                        assets,
                        [pad(direction, Some(button)), pad(0, None)],
                    );
                    assert_eq!(
                        fight.fight_read().fighters[1].move_id == THROWN_MOVE,
                        gap == 77,
                        "{loadout:?} back={back} button={button} axis gap={gap}"
                    );
                }
            }
        }
    }
}

#[test]
fn st_ryu_throws_with_p2_controls() {
    let Some(select) = local_select_with_fighter(
        GaugeMode::EraDefault,
        TournamentFighter::Ken,
        Revision::St,
        Loadout::Standard,
    ) else {
        return;
    };
    for loadout in LOADOUTS {
        let session = local_fight_with_p2_ryu(select.clone(), loadout);
        let assets = session.assets();
        let mut base = session.fight().unwrap().clone();
        ready(&mut base, assets);
        for (button, back) in [(MP, false), (MK, true)] {
            let direction = if back { RIGHT } else { LEFT };
            let mut fight = at_gap(base.clone(), 32);
            let origin = axis_x(&fight, fight.machine().diagnostic_telemetry().p2);
            replay_step(
                &mut fight,
                assets,
                0,
                [pad(0, None), pad(direction, Some(button))],
            );
            let fighters = fight.fight_read().fighters;
            assert_eq!(fighters[1].move_id, THROW_MOVE);
            assert_eq!(fighters[0].move_id, THROWN_MOVE);
            assert!(fight
                .machine()
                .toggle_state()
                .throw
                .st_ryu_capture_active(0));
            let timing = timing(button);
            for elapsed in 1..=timing.release {
                if elapsed == timing.release {
                    replay_step(&mut fight, assets, elapsed, [PlayerInputs::default(); 2]);
                } else {
                    step(&mut fight, assets, [PlayerInputs::default(); 2]);
                }
            }
            let ken = fight.machine().diagnostic_telemetry().p1;
            assert!(!fight
                .machine()
                .toggle_state()
                .throw
                .st_ryu_capture_active(0));
            assert_eq!(axis_x(&fight, ken) - origin, if back { 127 } else { -127 });
            assert_eq!(height(&fight, ken), timing.release_height);
        }
    }
}

#[test]
fn st_ryu_throws_soften_through_tick_thirteen_for_new_ken() {
    for loadout in LOADOUTS {
        let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, loadout)
        else {
            return;
        };
        let session = local_fight(&select, Revision::St, Loadout::Standard);
        let assets = session.assets();
        let mut base = session.fight().unwrap().clone();
        ready(&mut base, assets);
        let base = at_gap(base, 32);
        for button in [MP, MK] {
            for back in [false, true] {
                let soften = pad(LEFT, Some(MP));
                let early =
                    check_ryu_throw(base.clone(), assets, button, back, Some((13, soften)), true);
                let late = check_ryu_throw(
                    base.clone(),
                    assets,
                    button,
                    back,
                    Some((14, soften)),
                    false,
                );
                assert_eq!(early * 2, late, "softened throw damage");
            }
        }
        check_ryu_throw(base, assets, MP, false, Some((13, pad(0, Some(MP)))), false);
    }
}

#[test]
fn st_ken_can_throw_either_ryu_but_only_new_can_soften() {
    for loadout in [Loadout::Old, Loadout::Standard] {
        let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, loadout)
        else {
            return;
        };
        let session = local_fight(&select, Revision::St, Loadout::Standard);
        let assets = session.assets();
        let mut fight = session.fight().unwrap().clone();
        ready(&mut fight, assets);
        let mut fight = at_gap(fight, 32);
        let initial_hp = fight.machine().era_fighters()[0].sub_hp;
        let damage_tick = if loadout == Loadout::Old { 79 } else { 44 };
        for elapsed in 0..=damage_tick {
            let input = [
                pad(
                    if elapsed == 13 { RIGHT } else { 0 },
                    (elapsed == 13).then_some(MP),
                ),
                pad(
                    if elapsed == 0 { LEFT } else { 0 },
                    (elapsed == 0).then_some(MP),
                ),
            ];
            if [0, 13, 44, damage_tick].contains(&elapsed) {
                replay_step(&mut fight, assets, elapsed, input);
            } else {
                step(&mut fight, assets, input);
            }
            if elapsed == 0 {
                let fighters = fight.fight_read().fighters;
                assert_eq!(fighters[1].move_id, THROW_MOVE);
                assert_eq!(fighters[0].move_id, THROWN_MOVE);
            }
            let damage = initial_hp - fight.machine().era_fighters()[0].sub_hp;
            assert_eq!(damage > 0, elapsed == damage_tick);
            if elapsed == damage_tick {
                let source_hp = if loadout == Loadout::Old { 32 } else { 16 };
                assert_eq!(damage, (source_hp * 960 + 72) / 144);
            }
        }
    }
}

#[test]
fn st_same_tick_throws_only_capture_one_fighter_and_replay() {
    for loadout in LOADOUTS {
        let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, loadout)
        else {
            return;
        };
        let session = local_fight(&select, Revision::St, Loadout::Standard);
        let assets = session.assets();
        let mut fight = session.fight().unwrap().clone();
        ready(&mut fight, assets);
        let mut fight = at_gap(fight, 32);
        let mut wins = [false; 2];
        for delay in 0..16 {
            let seed = fight
                .machine()
                .toggle_state()
                .throw
                .st_order_seed()
                .unwrap();
            // The source advances the seed before selection: even output
            // visits P2 first, and odd output visits P1 first.
            let winner = if seed & 2 == 0 { 1 } else { 0 };
            let mut replay = fight.clone();
            replay_step(
                &mut replay,
                assets,
                delay,
                [pad(RIGHT, Some(MP)), pad(LEFT, Some(MP))],
            );
            let moves = replay.fight_read().fighters.map(|fighter| fighter.move_id);
            assert_eq!(moves[winner], THROW_MOVE, "{loadout:?} seed {seed:#06x}");
            assert_eq!(moves[1 - winner], THROWN_MOVE, "only one capture");
            wins[winner] = true;
            step(&mut fight, assets, [PlayerInputs::default(); 2]);
        }
        assert_eq!(wins, [true; 2], "{loadout:?} visits either side first");
    }
}

#[test]
fn st_back_throw_into_left_edge_keeps_the_defender_on_the_launch_side() {
    let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, Loadout::Standard)
    else {
        return;
    };
    let session = local_fight(&select, Revision::St, Loadout::Standard);
    let assets = session.assets();
    let mut fight = session.fight().unwrap().clone();
    ready(&mut fight, assets);
    let mut fight = at_axis(fight, 116, 32);
    step(&mut fight, assets, [pad(LEFT, Some(MP)), pad(0, None)]);
    assert!(fight
        .machine()
        .toggle_state()
        .throw
        .st_ryu_capture_active(1));
    for elapsed in 1..=79 {
        step(&mut fight, assets, [PlayerInputs::default(); 2]);
        if elapsed >= 44 {
            let read = fight.machine().diagnostic_telemetry();
            assert!(
                axis_x(&fight, read.p2) < axis_x(&fight, read.p1),
                "leftward throw crossed the screen at +{elapsed}"
            );
        }
    }
}

#[test]
fn mixed_era_throws_keep_the_ordinary_response_and_match_switch() {
    for (ryu, ken) in [
        (Revision::St, Revision::Sf1Plus),
        (Revision::Sf1Plus, Revision::St),
    ] {
        let Some(select) = local_select_with_p1(GaugeMode::EraDefault, ryu, Loadout::Standard)
        else {
            return;
        };
        let session = local_fight(&select, ken, Loadout::Standard);
        let assets = session.assets();
        let mut base = session.fight().unwrap().clone();
        ready(&mut base, assets);
        let base = at_gap(base, 32);
        for (attacker, revision) in [ryu, ken].into_iter().enumerate() {
            let defender = 1 - attacker;
            for disabled in [false, true] {
                let mut fight = base.clone();
                let mut snapshot = fight.save_netplay_snapshot(0);
                snapshot.machine.match_config.sf1plus.untechable_sf2_throws = disabled;
                snapshot.setup.config.sf1plus.untechable_sf2_throws = disabled;
                fight.restore_netplay_snapshot(&snapshot);
                let initial_hp = fight.machine().era_fighters()[defender].sub_hp;
                let mut input = [PlayerInputs::default(); 2];
                input[attacker] = pad(if attacker == 0 { RIGHT } else { LEFT }, Some(HP));
                step(&mut fight, assets, input);
                assert_eq!(fight.fight_read().fighters[defender].move_id, THROWN_MOVE);
                let mut input = [PlayerInputs::default(); 2];
                input[defender] = pad(0, Some(LP));
                input[defender].buttons[LK] = true;
                step(&mut fight, assets, input);
                assert_eq!(
                    fight.fight_read().fighters[defender].move_id == THROWN_MOVE,
                    disabled && revision == Revision::St
                );
                assert_eq!(fight.machine().era_fighters()[defender].sub_hp, initial_hp);
                assert!(fight.machine().toggle_state().throw.st_order_seed().is_none());
            }
        }
    }
}
