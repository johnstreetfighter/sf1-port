//! Controller-input witnesses for World Super Turbo Ryu's Red Hadoken (#142).

#[path = "support/alpha.rs"]
mod support;

use sf_engine::data_bank::GROUND;
use sf_engine::era::{GaugeMode, Loadout, Revision};
use sf_engine::era_fight::SF2_EXTRA_SPECIAL;
use sf_engine::fighter::{P1_STRUCT, P2_STRUCT};
use sf_engine::front::fight::Fight;
use sf_engine::overlay::{FighterTelemetry, ProjectileTelemetry};
use sf_video::callouts::Side;
use sf_video::romset::Assets;
use support::{local_fight, local_select_with_p1, pad, DOWN, LEFT, LP, RIGHT};

const LOADOUTS: [Loadout; 2] = [Loadout::Standard, Loadout::Old];
const RED_MOTION: [u8; 5] = [LEFT, DOWN | LEFT, DOWN, DOWN | RIGHT, RIGHT];
const GROUNDED_RECOIL: [i32; 20] = [8, 8, 8, 7, 7, 7, 6, 5, 5, 4, 3, 2, 1, 1, 1, 1, 1, 1, 1, 1];

fn camera_x(fight: &Fight) -> i32 {
    i32::from(u16::from_be_bytes(
        fight.machine().work_ram()[0x400c..0x400e]
            .try_into()
            .unwrap(),
    ))
}

fn axis_x(fight: &Fight, fighter: FighterTelemetry) -> i32 {
    let objects = &fight.machine().state.objectram;
    let pose = objects[usize::from(fighter.entity_offset) + 0x26] as i8;
    camera_x(fight) + fighter.center_x - i32::from(pose)
}

fn floor_height(fight: &Fight, fighter: FighterTelemetry) -> i32 {
    let entity = usize::from(fighter.entity_offset);
    let objects = &fight.machine().state.objectram;
    let attr = u16::from_be_bytes(objects[entity + 2..entity + 4].try_into().unwrap());
    let tile_height = if attr & 0x400 != 0 { 32 } else { 16 };
    let top = u16::from_be_bytes(objects[entity + 4..entity + 6].try_into().unwrap());
    GROUND - (i32::from(top) + tile_height * (i32::from(objects[entity + 0xc]) + 1))
}

fn idle(fight: &mut Fight, assets: &Assets) {
    fight.step(assets, pad(0, None), pad(0, None), false);
}

fn ready(fight: &mut Fight, assets: &Assets) {
    for _ in 0..94 {
        idle(fight, assets);
    }
}

// The source fixture writes both fighter positions to reproduce a one-pixel
// boundary. Move only the native sprite grids, retaining motion, pose, and HP.
fn place_at_gap(fight: &mut Fight, gap: i32) {
    let read = fight.machine().diagnostic_telemetry();
    let mut snapshot = fight.save_netplay_snapshot(0);
    let anchor = camera_x(fight) + 137;
    for (fighter, target) in [(read.p1, anchor), (read.p2, anchor + gap)] {
        let entity = usize::from(fighter.entity_offset);
        let objects = &mut snapshot.machine.state.objectram;
        let cells =
            (usize::from(objects[entity + 0xc]) + 1) * (usize::from(objects[entity + 0xd]) + 1);
        let dx = (target - axis_x(fight, fighter)) as u16;
        for cell in 0..cells {
            let x = entity + cell * 0x40 + 6;
            let value = u16::from_be_bytes(objects[x..x + 2].try_into().unwrap()).wrapping_add(dx);
            objects[x..x + 2].copy_from_slice(&value.to_be_bytes());
        }
    }
    fight.restore_netplay_snapshot(&snapshot);
}

fn red_input(fight: &mut Fight, assets: &Assets, punch: usize) {
    for dir in RED_MOTION {
        let button = (dir == RIGHT).then_some(punch);
        fight.step(assets, pad(dir, button), pad(0, None), false);
    }
    assert_eq!(
        fight.fight_read().fighter(Side::P1).move_id,
        SF2_EXTRA_SPECIAL
    );
}

fn live(fight: &Fight, owner: u32) -> Option<ProjectileTelemetry> {
    fight
        .machine()
        .diagnostic_telemetry()
        .projectiles
        .into_iter()
        .find(|projectile| {
            projectile.owner_struct == owner
                && fight.machine().state.objectram[usize::from(projectile.slot_offset) + 0x0e]
                    & 0xe0
                    != 0
        })
}

fn crimson_budget(fight: &Fight) -> Option<u8> {
    fight
        .machine()
        .toggle_state()
        .era
        .projectiles()
        .iter()
        .find(|projectile| projectile.crimson && projectile.hits_remaining != 0)
        .map(|projectile| projectile.hits_remaining)
}

fn replay_idle(fight: &mut Fight, assets: &Assets, tick: u32) {
    let before = fight.save_netplay_snapshot(tick);
    idle(fight, assets);
    let after = fight.save_netplay_snapshot(tick + 1);
    let checksum = fight.state_checksum();
    fight.restore_netplay_snapshot(&before);
    idle(fight, assets);
    assert_eq!(fight.state_checksum(), checksum, "rollback at {tick}");
    assert!(
        fight.save_netplay_snapshot(tick + 1) == after,
        "rollback at {tick}"
    );
}

fn verify_clean_red(mut fight: Fight, assets: &Assets, punch: usize, gap: i32, impact_age: u32) {
    red_input(&mut fight, assets, punch);
    place_at_gap(&mut fight, gap);
    let speed = punch as i32 + 3;
    let launches = impact_age <= 8;
    let verify_launch_profile = punch == 2 && impact_age == 0;
    let mut spawn = None;
    let mut impact = None;
    for tick in 0..150 {
        if spawn.is_some_and(|(start, _)| tick == start + 5)
            || verify_launch_profile && impact.is_some_and(|(at, _, _)| tick == at + 35)
        {
            replay_idle(&mut fight, assets, tick);
        } else {
            idle(&mut fight, assets);
        }
        let read = fight.machine().diagnostic_telemetry();
        let ken_x = axis_x(&fight, read.p2);
        let contact = fight.fight_read().landed_by(Side::P1);
        if let Some(projectile) = live(&fight, P1_STRUCT) {
            let x = camera_x(&fight) + projectile.hitbox.center_x;
            let (start, first_x) = *spawn.get_or_insert_with(|| {
                let ryu_x = axis_x(&fight, read.p1);
                assert_eq!(ken_x - ryu_x, gap, "actual spawn gap");
                assert_eq!(x - ryu_x, 93, "spawn X offset");
                assert_eq!(projectile.hitbox.center_y, GROUND - 54, "spawn Y offset");
                (tick, x)
            });
            assert_eq!(x - first_x, (tick - start) as i32 * speed);
            assert_eq!(crimson_budget(&fight), Some(1));
        }
        if let Some(hit) = contact {
            assert!(impact.is_none(), "Red Hadoken only lands one hit");
            assert!(!hit.chip);
            let start = spawn.map_or(tick, |(start, _)| start);
            assert_eq!(
                tick - start,
                impact_age,
                "projectile advances before impact"
            );
            assert_eq!(
                fight.machine().toggle_state().combo.contact_freeze[..2],
                [0, 14]
            );
            assert_eq!(
                read.p2.state_byte & 3 == 1,
                launches,
                "initial defender reaction"
            );
            assert_eq!(
                live(&fight, P1_STRUCT),
                None,
                "contact consumes the projectile"
            );
            impact = Some((tick, ken_x, ken_x));
        } else if let Some((at, origin, previous)) = impact {
            let elapsed = tick - at;
            let combo = &fight.machine().toggle_state().combo;
            assert_eq!(
                combo.contact_freeze[..2],
                [0, 14_u8.saturating_sub(elapsed as u8)]
            );
            if verify_launch_profile {
                let height = floor_height(&fight, read.p2);
                let ken = fight.fight_read().fighter(Side::P2);
                match elapsed {
                    1 | 15 => assert_eq!(height, 16, "initial rise at +{elapsed}"),
                    30 => assert_eq!(height, 57, "launch peak"),
                    53 | 75 => assert_eq!(height, 0, "floor at +{elapsed}"),
                    58 => assert_eq!(height, 2, "ground bounce starts"),
                    64 => assert_eq!(height, 10, "ground bounce peak"),
                    127 => {
                        assert_eq!(height, 0);
                        assert!(!ken.hitstun() && !ken.getting_up, "Ken regained control");
                        assert_eq!(ken_x - origin, 126);
                    }
                    _ => {}
                }
            }
            if !launches && elapsed <= 36 {
                let expected_step = elapsed
                    .checked_sub(16)
                    .and_then(|i| GROUNDED_RECOIL.get(i as usize))
                    .copied()
                    .unwrap_or(0);
                assert_eq!(
                    ken_x - previous,
                    expected_step,
                    "grounded recoil at +{elapsed}"
                );
                assert_eq!(fight.fight_read().fighter(Side::P2).hitstun(), elapsed < 36);
                if elapsed == 36 {
                    assert_eq!(ken_x - origin, 78);
                }
            }
            impact = Some((at, origin, ken_x));
        }
    }
    assert!(impact.is_some(), "Red Hadoken at {gap} px must hit");
    assert!(fight.machine().unrecovered().is_empty());
}

#[test]
fn all_st_red_hadoken_strengths_match_the_arcade_travel_boundary() {
    for loadout in LOADOUTS {
        let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, loadout)
        else {
            return;
        };
        let session = local_fight(&select, Revision::St, Loadout::Standard);
        let assets = session.assets();
        let mut base = session.fight().unwrap().clone();
        ready(&mut base, assets);
        // World 940223 Turbo 2 boundaries against stationary standing Ken.
        for (punch, last_launch) in [146, 154, 162].into_iter().enumerate() {
            eprintln!("{loadout:?} Red Hadoken strength {punch}");
            verify_clean_red(base.clone(), assets, punch, 42, 0);
            verify_clean_red(base.clone(), assets, punch, last_launch, 8);
            verify_clean_red(base.clone(), assets, punch, last_launch + 1, 9);
        }
    }
}

#[test]
fn st_red_hadoken_trades_with_an_ordinary_hadoken() {
    for loadout in LOADOUTS {
        let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, loadout)
        else {
            return;
        };
        let session = local_fight(&select, Revision::St, Loadout::Standard);
        let assets = session.assets();
        let mut fight = session.fight().unwrap().clone();
        ready(&mut fight, assets);
        place_at_gap(&mut fight, 230);
        let health = fight.fight_read().fighters.map(|fighter| fighter.health);
        let mut two_live = false;
        let mut traded = false;
        for tick in 0..70 {
            let ryu = RED_MOTION.get(tick).copied().unwrap_or(0);
            let ken = [DOWN, DOWN | LEFT, LEFT].get(tick).copied().unwrap_or(0);
            fight.step(
                assets,
                pad(ryu, (tick == 4).then_some(2)),
                pad(ken, (tick == 2).then_some(LP)),
                false,
            );
            assert_eq!(
                fight.fight_read().fighters.map(|fighter| fighter.health),
                health
            );
            let active = [P1_STRUCT, P2_STRUCT].map(|owner| live(&fight, owner).is_some());
            if active == [true; 2] {
                two_live = true;
                assert_eq!(crimson_budget(&fight), Some(1));
            }
            if two_live && active == [false; 2] {
                traded = true;
            }
        }
        assert!(traded, "{loadout:?}: both live Hadokens disappear on clash");
        assert!(fight.machine().unrecovered().is_empty());
    }
}

#[test]
fn st_heavy_red_hadoken_causes_one_grounded_guard_reaction() {
    for loadout in LOADOUTS {
        let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, loadout)
        else {
            return;
        };
        let session = local_fight(&select, Revision::St, Loadout::Standard);
        let assets = session.assets();
        let mut base = session.fight().unwrap().clone();
        ready(&mut base, assets);
        for guard in [RIGHT, DOWN | RIGHT] {
            let mut fight = base.clone();
            red_input(&mut fight, assets, 2);
            place_at_gap(&mut fight, 162);
            let health = fight.fight_read().fighter(Side::P2).health;
            let mut impact = None;
            for tick in 0..65 {
                // Establish the guard before impact, then release to
                // observe the first tick of regained control.
                let guard_now = impact.is_none() && (guard & DOWN != 0 || tick >= 16);
                fight.step(
                    assets,
                    pad(0, None),
                    pad(if guard_now { guard } else { 0 }, None),
                    false,
                );
                let read = fight.fight_read();
                let ken = read.fighter(Side::P2);
                let x = axis_x(&fight, fight.machine().diagnostic_telemetry().p2);
                let combo = &fight.machine().toggle_state().combo;
                if read.blocked[0] {
                    assert!(ken.health < health, "the guarded projectile deals chip");
                    assert!(impact.is_none());
                    assert_eq!(combo.contact_freeze[..2], [0, 14]);
                    assert_eq!(live(&fight, P1_STRUCT), None);
                    impact = Some((tick, x));
                } else if let Some((at, origin)) = impact {
                    let elapsed = tick - at;
                    assert_ne!(ken.state & 3, 1, "guard stays grounded");
                    assert_eq!(
                        combo.contact_freeze[..2],
                        [0, 14_u8.saturating_sub(elapsed as u8)]
                    );
                    if elapsed == 36 {
                        assert!(!ken.blockstun && !ken.hitstun());
                        assert_eq!(x - origin, 78);
                    }
                }
                assert!(
                    impact.is_some() || ken.health == health,
                    "Ken guards before damage"
                );
            }
            assert!(impact.is_some(), "Ken must guard in posture {guard}");
        }
    }
}

#[test]
fn st_heavy_red_hadoken_matches_whiff_startup_and_recovery() {
    for loadout in LOADOUTS {
        let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, loadout)
        else {
            return;
        };
        let session = local_fight(&select, Revision::St, Loadout::Standard);
        let assets = session.assets();
        let mut fight = session.fight().unwrap().clone();
        ready(&mut fight, assets);
        red_input(&mut fight, assets, 2);
        place_at_gap(&mut fight, 162);
        let origin = axis_x(&fight, fight.machine().diagnostic_telemetry().p1);
        let mut previous_x = origin;
        let mut spawned = None;
        let mut retreated = None;
        for tick in 1..=65 {
            // Ken jumps forward across the projectile while Ryu holds retreat.
            fight.step(
                assets,
                pad(LEFT, None),
                pad(support::UP | LEFT, None),
                false,
            );
            let read = fight.fight_read();
            assert!(
                !read.connected(Side::P1),
                "{loadout:?}: contact at {tick}, spawn {spawned:?}, Ken {:?}",
                read.fighter(Side::P2)
            );
            if live(&fight, P1_STRUCT).is_some() {
                spawned.get_or_insert(tick);
            }
            let ryu_x = axis_x(&fight, fight.machine().diagnostic_telemetry().p1);
            let move_id = read.fighter(Side::P1).move_id;
            if move_id == SF2_EXTRA_SPECIAL {
                assert_eq!(ryu_x, origin, "Ryu's axis stays fixed through Red Hadoken");
            } else if move_id == 3 && retreated.is_none() {
                assert_eq!(previous_x - ryu_x, 2, "the first retreat step");
                retreated = Some(tick);
            }
            previous_x = ryu_x;
        }
        assert_eq!(spawned, Some(11), "{loadout:?}: HP Red startup");
        assert_eq!(retreated, Some(52), "{loadout:?}: HP Red whiff recovery");
    }
}
