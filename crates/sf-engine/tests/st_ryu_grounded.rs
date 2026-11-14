//! Controller-input witnesses for the grounded Super Turbo Ryu audit (#139).

#[path = "support/alpha.rs"]
mod support;

use sf_engine::config::MatchConfig;
use sf_engine::era::{GaugeMode, Loadout, Revision};
use sf_engine::fair_cpu::FIGHT_LOOP;
use sf_engine::fighter::P1_STRUCT;
use sf_engine::front::cabinet::{flow_position, Players};
use sf_engine::front::fight::{Fight, FightDips, FightKind, FightSetup};
use sf_engine::front::PlayerInputs;
use sf_engine::overlay::FighterTelemetry;
use sf_engine::roster::PlayMode;
use sf_engine::session_io::ControlScheme;
use sf_video::callouts::Side;
use sf_video::romset::{load_assets, Assets};
use support::{local_fight, local_select_with_p1, pad, DOWN, LEFT, LP, RIGHT};

const LK: usize = 3;
const MK: usize = 4;
const HK: usize = 5;
const HADOKEN: u8 = 0x1a;
const FREEZE: u8 = 14;
const LOADOUTS: [Loadout; 2] = [Loadout::Standard, Loadout::Old];

fn ken_world_x(fight: &Fight) -> i32 {
    let machine = fight.machine();
    let ken = machine.diagnostic_telemetry().p2;
    let camera = u16::from_be_bytes(machine.work_ram()[0x400c..0x400e].try_into().unwrap());
    let pose_offset = machine.state.objectram[usize::from(ken.entity_offset) + 0x26] as i8;
    i32::from(camera) + ken.center_x - i32::from(pose_offset)
}

fn at_audited_spacing(mut fight: Fight, assets: &Assets) -> Fight {
    for _ in 0..94 {
        fight.step(assets, pad(0, None), pad(0, None), false);
    }
    let gap = |fight: &Fight| {
        let read = fight.machine().diagnostic_telemetry();
        read.p1.center_x.abs_diff(read.p2.center_x)
    };
    for _ in 0..180 {
        if gap(&fight) <= 32 {
            break;
        }
        fight.step(assets, pad(0, None), pad(LEFT, None), false);
    }
    for _ in 0..16 {
        fight.step(assets, pad(0, None), pad(0, None), false);
    }
    assert_eq!(gap(&fight), 32, "midscreen collision-center gap");
    fight
}

fn strike(fight: &mut Fight, assets: &Assets, kick: usize, guard: bool) -> u32 {
    let health = fight.fight_read().fighters[1].health;
    for tick in 0..20 {
        let ken = pad(if guard { DOWN | RIGHT } else { 0 }, None);
        fight.step(assets, pad(DOWN, (tick == 0).then_some(kick)), ken, false);
        let read = fight.fight_read();
        if read.connected(Side::P1) {
            assert_eq!(read.blocked[0], guard);
            assert_eq!(read.fighters[1].health < health, !guard);
            return tick;
        }
    }
    panic!("crouching kick did not connect from the audited gap");
}

fn pose_clock(fight: &Fight, fighter: FighterTelemetry) -> ([u8; 4], u8) {
    let entity = usize::from(fighter.entity_offset);
    let objects = &fight.machine().state.objectram;
    (
        objects[entity + 0x2c..entity + 0x30].try_into().unwrap(),
        objects[entity + 0x33],
    )
}

fn ken_height(fight: &Fight) -> u16 {
    let machine = fight.machine();
    let ken = usize::from(machine.diagnostic_telemetry().p2.entity_offset);
    let objects = &machine.state.objectram;
    let attr = u16::from_be_bytes([objects[ken + 2], objects[ken + 3]]);
    let top = u16::from_be_bytes([objects[ken + 4], objects[ken + 5]]);
    let tile_height = if attr & 0x400 != 0 { 32 } else { 16 };
    (sf_engine::data_bank::GROUND as u16).wrapping_sub(top + (u16::from(objects[ken + 0xc]) + 1) * tile_height)
}

fn replay_step(fight: &mut Fight, assets: &Assets, tick: u32, ryu: PlayerInputs) {
    let before = fight.save_netplay_snapshot(tick);
    fight.step(assets, ryu, pad(0, None), false);
    let after = fight.save_netplay_snapshot(tick + 1);
    let checksum = fight.state_checksum();
    fight.restore_netplay_snapshot(&before);
    fight.step(assets, ryu, pad(0, None), false);
    assert_eq!(fight.state_checksum(), checksum, "rollback at {tick}");
    assert!(
        fight.save_netplay_snapshot(tick + 1) == after,
        "rollback at {tick}"
    );
}

fn check_grounded_reaction(mut fight: Fight, assets: &Assets, kick: usize, guard: bool) {
    let origin = ken_world_x(&fight);
    let contact = strike(&mut fight, assets, kick, guard);
    // World 940223, Turbo 2. The four staggered capture phases recover
    // successive clean-contact steps hidden by any one frame-skip phase.
    // See docs/research/ryu-st-turbo2-contact-evidence.md.
    let steps: &[i32] = if kick == MK {
        &[6, 6, 6, 5, 5, 5, 4, 4, 3, 3, 2, 1, 1, 1, 1, 1]
    } else {
        &[4, 4, 4, 4, 3, 3, 3, 2, 1, 1, 1]
    };
    let stun = steps.len() as u8;
    let push = steps.iter().sum::<i32>();
    let defender_stun = |fight: &Fight| {
        let combo = &fight.machine().toggle_state().combo;
        if guard {
            combo.blockstun[1]
        } else {
            combo.hitstun[1]
        }
    };
    let read = fight.machine().diagnostic_telemetry();
    let poses = [pose_clock(&fight, read.p1), pose_clock(&fight, read.p2)];
    assert_eq!(defender_stun(&fight), stun);
    assert_eq!(ken_world_x(&fight), origin);

    for elapsed in 1..=FREEZE {
        let combo = &fight.machine().toggle_state().combo;
        assert_eq!(combo.contact_freeze[..2], [FREEZE + 1 - elapsed; 2]);
        if elapsed == 7 {
            replay_step(&mut fight, assets, contact + 6, pad(DOWN, None));
        } else {
            fight.step(assets, pad(DOWN, None), pad(0, None), false);
        }
        let read = fight.machine().diagnostic_telemetry();
        assert_eq!(
            [pose_clock(&fight, read.p1), pose_clock(&fight, read.p2)],
            poses
        );
        assert_eq!(defender_stun(&fight), stun, "stun waits for contact freeze");
        assert_eq!(
            ken_world_x(&fight),
            origin,
            "Ken stays in place during freeze"
        );
    }
    assert_eq!(
        fight.machine().toggle_state().combo.contact_freeze[..2],
        [0; 2]
    );

    let mut previous_x = origin;
    for elapsed in 1..=stun + 3 {
        if elapsed == 5 {
            replay_step(
                &mut fight,
                assets,
                contact + u32::from(FREEZE + elapsed - 1),
                pad(DOWN, None),
            );
        } else {
            fight.step(assets, pad(DOWN, None), pad(0, None), false);
        }
        let world_x = ken_world_x(&fight);
        if !guard {
            let step = if (2..=stun + 1).contains(&elapsed) {
                steps[usize::from(elapsed - 2)]
            } else {
                0
            };
            assert_eq!(
                world_x - previous_x,
                step,
                "pushback at contact +{}",
                FREEZE + elapsed
            );
        }
        previous_x = world_x;
        let ken = fight.fight_read().fighters[1];
        assert_ne!(
            ken.state & 3,
            1,
            "Ken stays grounded at +{elapsed} after freeze"
        );
        let busy = if guard { ken.blockstun } else { ken.hitstun() };
        if !busy {
            // The setup tick after freeze is stationary. The next tick is
            // the first of the advancing grounded reaction ticks.
            assert_eq!(
                elapsed,
                2 + stun,
                "first guard-ready or neutral tick after freeze"
            );
            assert_eq!(defender_stun(&fight), 0);
            assert_eq!(world_x - origin, push);
            return;
        }
    }
    panic!("the grounded contact never released Ken");
}

#[test]
fn new_and_old_ryu_use_arcade_grounded_contact_timing_and_pushback() {
    for loadout in LOADOUTS {
        let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, loadout)
        else {
            return;
        };
        let session = local_fight(&select, Revision::St, Loadout::Standard);
        let assets = session.assets();
        let base = at_audited_spacing(session.fight().unwrap().clone(), assets);
        for (kick, guard) in [(LK, false), (MK, false), (LK, true)] {
            eprintln!("{loadout:?} Ryu, crouching kick {kick}, guard={guard}");
            check_grounded_reaction(base.clone(), assets, kick, guard);
        }
    }
}

#[test]
fn crouching_punches_and_sweep_use_world_startup_and_recovery() {
    for loadout in LOADOUTS {
        let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, loadout)
        else { return };
        let session = local_fight(&select, Revision::St, Loadout::Standard);
        let assets = session.assets();
        let mut base = session.fight().unwrap().clone();
        for _ in 0..94 {
            base.step(assets, pad(0, None), pad(0, None), false);
        }
        for (button, active, end) in [(LP, 3..7, 12), (1, 4..8, 15), (2, 4..15, 38), (HK, 4..10, 35)] {
            for direction in [DOWN, DOWN | LEFT, DOWN | RIGHT] {
                let mut fight = base.clone();
                for tick in 0..=end {
                    fight.step(assets, pad(direction, (tick == 0).then_some(button)), pad(0, None), false);
                    let read = fight.fight_read().fighters[0];
                    assert_eq!(read.attack_active, active.contains(&tick), "{loadout:?} button {button} direction {direction} active at {tick}");
                    assert_eq!(read.state & 0x40 != 0, tick < end, "{loadout:?} button {button} direction {direction} end at {tick}");
                }
            }
        }
    }
}

#[test]
fn crouching_punches_and_sweep_use_world_contacts_and_wakeup() {
    for loadout in LOADOUTS {
        let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, loadout)
        else { return };
        let session = local_fight(&select, Revision::St, Loadout::Standard);
        let assets = session.assets();
        let base = at_audited_spacing(session.fight().unwrap().clone(), assets);
        for (button, startup, stun, recoil) in [(LP, 3, 11, 30), (1, 4, 16, 54), (2, 4, 20, 78), (HK, 4, 20, 78)] {
            for direction in [0, RIGHT, DOWN | RIGHT, DOWN] {
                let mut fight = base.clone();
                if direction == DOWN {
                    for _ in 0..8 { fight.step(assets, pad(DOWN, None), pad(DOWN, None), false); }
                }
                let first = (0..12).find(|&tick| {
                    fight.step(assets, pad(DOWN, (tick == 0).then_some(button)), pad(if tick == 0 && direction != DOWN { 0 } else { direction }, None), false);
                    fight.fight_read().connected(Side::P1)
                });
                let label = format!("{loadout:?} button {button} defense {direction}");
                assert_eq!(first, Some(startup), "{label} startup");
                let crouch_guard = direction == (DOWN | RIGHT);
                let sweep = button == HK && !crouch_guard;
                let guarded = crouch_guard || (direction == RIGHT && !sweep);
                assert_eq!(fight.fight_read().blocked[0], guarded, "{label} block, Ken {:?}", fight.fight_read().fighters[1]);
                let combo = &fight.machine().toggle_state().combo;
                assert_eq!(combo.contact_freeze[..2], [14, 14], "{label} freeze");
                if !sweep {
                    assert_eq!(if guarded { combo.blockstun[1] } else { combo.hitstun[1] }, stun, "{label} stun");
                }
                let x = ken_world_x(&fight);
                for tick in 1..=if sweep { 95 } else { 16 + stun } {
                    if sweep && tick == 31 {
                        replay_step(&mut fight, assets, u32::from(tick), pad(DOWN, None));
                    } else {
                        fight.step(assets, pad(DOWN, None), pad(0, None), false);
                    }
                    let ken = fight.fight_read().fighters[1];
                    if sweep {
                        assert_eq!(ken_world_x(&fight) - x, i32::from(tick.saturating_sub(16).min(40)) * 2, "{label} at +{tick}");
                        assert_eq!(ken.reacting(), tick < 95, "{label} reaction at +{tick}");
                        let height = match tick {
                            16 | 56 => Some(0),
                            17 => Some(5),
                            34..=38 => Some(55),
                            55 => Some(4),
                            _ => None,
                        };
                        if let Some(height) = height {
                            assert_eq!(ken_height(&fight), height, "{label} height at +{tick}");
                        }
                    } else if tick == 16 + stun {
                        assert_eq!(ken_world_x(&fight) - x, recoil, "{label} recoil");
                        assert!(!ken.hitstun() && !ken.blockstun, "{label} release");
                    }
                }
            }
        }
    }
}

fn hadoken_after_contact(
    mut fight: Fight,
    assets: &Assets,
    kick: usize,
    delay: u32,
) -> HadokenRoute {
    for _ in 0..25 {
        fight.step(assets, pad(DOWN, None), pad(0, None), false);
    }
    let contact = strike(&mut fight, assets, kick, false);
    let mut route = HadokenRoute::default();
    for elapsed in 1..=80 {
        let ryu = if elapsed == delay - 1 {
            pad(DOWN | RIGHT, None)
        } else if elapsed == delay {
            pad(RIGHT, Some(LP))
        } else if elapsed < delay {
            pad(DOWN, None)
        } else {
            pad(0, None)
        };
        if elapsed == 3 && delay == 2 {
            replay_step(&mut fight, assets, contact + elapsed - 1, ryu);
        } else {
            fight.step(assets, ryu, pad(0, None), false);
        }
        let read = fight.fight_read();
        if read.fighters[0].move_id == HADOKEN {
            route.start.get_or_insert(elapsed);
        }
        if route.start.is_some()
            && route.spawn.is_none()
            && fight
                .machine()
                .diagnostic_telemetry()
                .projectiles
                .iter()
                .any(|projectile| projectile.owner_struct == P1_STRUCT)
        {
            route.spawn = Some(elapsed);
        }
        if let Some(hit) = read.landed_by(Side::P1) {
            route.hit = Some((elapsed, hit.into_hitstun));
            break;
        }
    }
    route
}

#[test]
fn crouching_punches_and_sweep_cancel_at_the_recorded_light_hadoken_boundaries() {
    for loadout in LOADOUTS {
        let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, loadout)
        else { return };
        let session = local_fight(&select, Revision::St, Loadout::Standard);
        let assets = session.assets();
        let base = at_audited_spacing(session.fight().unwrap().clone(), assets);
        let old = loadout == Loadout::Old;
        let cases = [
            (LP, if old { 16 } else { 15 }, HadokenRoute::contact(if old { 16 } else { 15 }, 26, true)),
            (LP, if old { 17 } else { 16 }, HadokenRoute::contact(if old { 17 } else { 16 }, 27, false)),
            (1, 16, HadokenRoute::contact(16, if old { 26 } else { 27 }, true)),
            (1, 17, HadokenRoute::default()),
            (2, if old { 15 } else { 16 }, HadokenRoute::contact(if old { 15 } else { 16 }, if old { 25 } else { 27 }, true)),
            (2, if old { 16 } else { 17 }, HadokenRoute::default()),
            (HK, 16, HadokenRoute { start: Some(16), spawn: Some(if old { 26 } else { 27 }), hit: None }),
            (HK, 17, HadokenRoute::default()),
        ];
        for (button, delay, expected) in cases {
            assert_eq!(hadoken_after_contact(base.clone(), assets, button, delay), expected,
                "{loadout:?} button {button} final LP at contact +{delay}");
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
struct HadokenRoute {
    start: Option<u32>,
    spawn: Option<u32>,
    hit: Option<(u32, bool)>,
}

impl HadokenRoute {
    fn contact(start: u32, impact: u32, combo: bool) -> Self {
        Self {
            start: Some(start),
            spawn: Some(impact),
            hit: Some((impact, combo)),
        }
    }
}

#[test]
fn new_and_old_ryu_combo_light_hadoken_at_the_verified_offsets() {
    for loadout in LOADOUTS {
        let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, loadout)
        else {
            return;
        };
        let session = local_fight(&select, Revision::St, Loadout::Standard);
        let assets = session.assets();
        let base = at_audited_spacing(session.fight().unwrap().clone(), assets);
        // The source records the final LP and the projectile contact. In
        // this close fixture, the port projectile connects as it spawns.
        let cases: &[(usize, u32, HadokenRoute)] = match loadout {
            Loadout::Old => &[
                (LK, 16, HadokenRoute::contact(16, 26, true)),
                (LK, 17, HadokenRoute::contact(17, 27, false)),
                (MK, 2, HadokenRoute::contact(15, 25, true)),
                (MK, 16, HadokenRoute::contact(16, 26, true)),
                (MK, 17, HadokenRoute::default()),
            ],
            _ => &[
                (LK, 15, HadokenRoute::contact(15, 26, true)),
                (LK, 16, HadokenRoute::contact(16, 27, false)),
                (MK, 16, HadokenRoute::contact(16, 27, true)),
                (MK, 17, HadokenRoute::default()),
            ],
        };
        for (kick, delay, expected) in cases {
            assert_eq!(
                &hadoken_after_contact(base.clone(), assets, *kick, *delay),
                expected,
                "{loadout:?} Ryu, crouching kick {kick}, final LP at contact +{delay}"
            );
        }
    }
}

#[test]
fn modern_hadoken_shortcut_cancels_after_contact_freeze() {
    let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, Loadout::Standard)
    else {
        return;
    };
    let session = local_fight(&select, Revision::St, Loadout::Standard);
    let assets = session.assets();
    let mut base = at_audited_spacing(session.fight().unwrap().clone(), assets);
    let mut rules = base.save_netplay_snapshot(0);
    rules.setup.config.p1_scheme = ControlScheme::Modern;
    rules.machine.match_config.p1_scheme = ControlScheme::Modern;
    rules.setup.config.modern_control_damage_penalty = true;
    rules.machine.match_config.modern_control_damage_penalty = true;
    base.restore_netplay_snapshot(&rules);

    // Modern's down-back macro supplies MP+MK. The native diagonal
    // dispatch takes the kick first and starts crouching MK.
    base.step(assets, pad(DOWN | LEFT, Some(5)), pad(0, None), false);
    assert_eq!(base.fight_read().fighters[0].move_id, 0x17);
    let contact = (1..20)
        .find(|_| {
            base.step(assets, pad(0, None), pad(0, None), false);
            base.fight_read().landed_by(Side::P1).is_some()
        })
        .expect("Modern crouching MK connected");
    assert_eq!(
        base.machine().toggle_state().combo.contact_freeze[..2],
        [FREEZE; 2]
    );

    for elapsed in 1..=50 {
        // Neutral + Special is a one-frame Modern Hadoken shortcut.
        let ryu = pad(0, (elapsed == 2).then_some(3));
        if elapsed == 3 {
            replay_step(&mut base, assets, contact + elapsed - 1, ryu);
        } else {
            base.step(assets, ryu, pad(0, None), false);
        }
        let read = base.fight_read();
        if elapsed <= u32::from(FREEZE) {
            assert_eq!(read.fighters[0].move_id, 0x17, "crouching MK freezes");
        } else if elapsed == u32::from(FREEZE) + 1 {
            assert_eq!(
                read.fighters[0].move_id, HADOKEN,
                "buffered shortcut starts after freeze"
            );
        }
        if let Some(hit) = read.landed_by(Side::P1) {
            assert!(
                hit.into_hitstun,
                "the shortcut combos into the grounded reaction"
            );
            return;
        }
    }
    panic!("the buffered Modern Hadoken never hit");
}

#[test]
fn super_turbo_ryus_grounded_normal_respects_the_defenders_low_parry() {
    let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, Loadout::Standard)
    else {
        return;
    };
    let session = local_fight(&select, Revision::Sf3ThreeS, Loadout::Art1);
    let assets = session.assets();
    let base = at_audited_spacing(session.fight().unwrap().clone(), assets);
    let contact = strike(&mut base.clone(), assets, LK, false);
    let health = base.fight_read().fighters[1].health;
    // A down tap can move Ken's hurtbox, so the parry contact may occur
    // on a different active tick from the hit against standing Ken.
    for tap in (contact.saturating_sub(3)..=contact + 1).rev() {
        let mut fight = base.clone();
        for tick in 0..=contact + 5 {
            let ken = pad(if tick == tap { DOWN } else { 0 }, None);
            fight.step(assets, pad(DOWN, (tick == 0).then_some(LK)), ken, false);
            let read = fight.fight_read();
            if read.fighters[1].health != health {
                break;
            }
            if read.fighters[1].move_id == 0x2e {
                assert_eq!(
                    fight.machine().toggle_state().combo.contact_freeze[..2],
                    [0; 2]
                );
                assert_eq!(
                    fight.machine().era_super_freeze(),
                    6,
                    "Third Strike parry freeze"
                );
                return;
            }
        }
    }
    panic!("Ken never parried the Super Turbo crouching kick with a timed Third Strike low tap");
}

#[test]
fn faithful_1987_keeps_its_airborne_medium_kick_reaction() {
    if !std::path::Path::new(support::ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return;
    }
    let assets = load_assets(support::ROMS, support::MANIFEST).expect("local ROMs assemble");
    let mut config = MatchConfig::faithful();
    config.roster.mode = PlayMode::Versus;
    let mut fight = Fight::start(
        &assets.maincpu,
        FightSetup {
            kind: FightKind::Local,
            players: Players::Two,
            config,
            dsw1: FightDips::DSW1,
            dsw2: FightDips::default().dsw2(),
        },
        Some(0),
        None,
    )
    .expect("local versus on the 1987 ruleset");
    for _ in 0..1500 {
        if flow_position(fight.machine()) == FIGHT_LOOP && !fight.driving() {
            break;
        }
        fight.step(&assets, pad(0, None), pad(0, None), false);
    }
    assert_eq!(flow_position(fight.machine()), FIGHT_LOOP);
    fight = at_audited_spacing(fight, &assets);
    strike(&mut fight, &assets, MK, false);
    let ken = fight.fight_read().fighters[1];
    assert_eq!(ken.move_id, 0x33);
    assert_eq!(ken.state & 3, 1);
    assert_eq!(
        fight.machine().toggle_state().combo.contact_freeze[..2],
        [0; 2]
    );
}
