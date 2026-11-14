//! Controller-input checks against the World Super Turbo Sagat throw trace.

#[allow(dead_code)]
#[path = "support/alpha.rs"]
mod support;

use sf_engine::combo::throw::{THROWN_MOVE, THROW_MOVE};
use sf_engine::data_bank::GROUND;
use sf_engine::era::{GaugeMode, Loadout, Revision};
use sf_engine::front::fight::Fight;
use sf_engine::overlay::FighterTelemetry;
use sf_engine::roster::TournamentFighter::Sagat;
use sf_video::callouts::Side;
use support::{local_fight, local_select_with_fighter, pad, LEFT, RIGHT};

const MP: usize = 1;
const HP: usize = 2;
const LOADOUTS: [Loadout; 2] = [Loadout::Standard, Loadout::Old];

fn axis_x(fight: &Fight, fighter: FighterTelemetry) -> i32 {
    let camera = u16::from_be_bytes(
        fight.machine().work_ram()[0x400c..0x400e]
            .try_into()
            .unwrap(),
    );
    let pose_x = fight.machine().state.objectram[usize::from(fighter.entity_offset) + 0x26] as i8;
    i32::from(camera) + fighter.center_x - i32::from(pose_x)
}

fn height(fight: &Fight, fighter: FighterTelemetry) -> i32 {
    let offset = usize::from(fighter.entity_offset);
    let ram = &fight.machine().state.objectram[offset..];
    let tile = if u16::from_be_bytes([ram[2], ram[3]]) & 0x400 != 0 {
        32
    } else {
        16
    };
    GROUND - i32::from(u16::from_be_bytes([ram[4], ram[5]])) - tile * (i32::from(ram[0xc]) + 1)
}

fn at_gap(mut fight: Fight, gap: i32) -> Fight {
    let read = fight.machine().diagnostic_telemetry();
    let mut snapshot = fight.save_netplay_snapshot(0);
    let anchor = axis_x(&fight, read.p1);
    for (fighter, target) in [(read.p1, anchor), (read.p2, anchor + gap)] {
        let entity = usize::from(fighter.entity_offset);
        let ram = &mut snapshot.machine.state.objectram;
        let cells = (usize::from(ram[entity + 0xc]) + 1) * (usize::from(ram[entity + 0xd]) + 1);
        let dx = (target - axis_x(&fight, fighter)) as u16;
        for cell in 0..cells {
            let at = entity + cell * 0x40 + 6;
            let x = u16::from_be_bytes([ram[at], ram[at + 1]]).wrapping_add(dx);
            ram[at..at + 2].copy_from_slice(&x.to_be_bytes());
        }
    }
    fight.restore_netplay_snapshot(&snapshot);
    fight
}

#[test]
fn st_sagat_ground_throw_accepts_only_medium_and_heavy_punch_at_77_pixels() {
    for loadout in LOADOUTS {
        let Some(select) =
            local_select_with_fighter(GaugeMode::EraDefault, Sagat, Revision::St, loadout)
        else {
            return;
        };
        let session = local_fight(&select, Revision::St, Loadout::Standard);
        let assets = session.assets();
        let mut base = session.fight().unwrap().clone();
        for _ in 0..94 {
            base.step(assets, pad(0, None), pad(0, None), false);
        }
        for direction in [RIGHT, LEFT] {
            for (gap, button, captures) in [
                (77, MP, true),
                (77, HP, true),
                (78, MP, false),
                (78, HP, false),
                (38, 0, false),
                (38, 3, false),
                (38, 4, false),
                (38, 5, false),
            ] {
                let mut fight = at_gap(base.clone(), gap);
                fight.step(assets, pad(direction, Some(button)), pad(0, None), false);
                assert_eq!(
                    fight.fight_read().fighters[1].move_id == THROWN_MOVE,
                    captures,
                    "{loadout:?}, direction {direction}, gap {gap}, button {button}",
                );
            }
        }
    }
}

#[test]
fn st_sagat_ground_throw_uses_world_release_damage_and_recovery_ticks() {
    for loadout in LOADOUTS {
        let Some(select) =
            local_select_with_fighter(GaugeMode::EraDefault, Sagat, Revision::St, loadout)
        else {
            return;
        };
        let session = local_fight(&select, Revision::St, Loadout::Standard);
        let assets = session.assets();
        let mut base = session.fight().unwrap().clone();
        for _ in 0..94 {
            base.step(assets, pad(0, None), pad(0, None), false);
        }
        for direction in [RIGHT, LEFT] {
            for (response, response_button, response_direction, softened) in [
                (0, MP, LEFT, true),
                (13, MP, LEFT, true),
                (13, 4, LEFT, true),
                (13, 0, LEFT, false),
                (13, MP, 0, false),
                (14, MP, LEFT, false),
            ] {
                let mut fight = at_gap(base.clone(), 38);
                let origin = axis_x(&fight, fight.machine().diagnostic_telemetry().p1);
                let initial_hp = fight.machine().era_fighters()[1].sub_hp;
                let leftward = direction == LEFT;
                let damage_tick = if softened { 39 } else { 83 };
                let landing = if softened { 96 } else { 82 };
                let recovery = if softened {
                    102
                } else if leftward {
                    145
                } else {
                    146
                };
                let lost = if softened { 107 } else { 213 };
                for tick in 0..=recovery + 1 {
                    let attacker = if tick == 0 {
                        pad(direction, Some(MP))
                    } else {
                        pad(0, None)
                    };
                    let defender = if tick == response {
                        pad(response_direction, Some(response_button))
                    } else if tick == recovery + 1 {
                        pad(0, Some(0))
                    } else {
                        pad(0, None)
                    };
                    if [13, 39, damage_tick].contains(&tick) {
                        let snapshot = fight.save_netplay_snapshot(tick);
                        fight.step(assets, attacker, defender, false);
                        let checksum = fight.state_checksum();
                        fight.restore_netplay_snapshot(&snapshot);
                        fight.step(assets, attacker, defender, false);
                        assert_eq!(fight.state_checksum(), checksum);
                    } else {
                        fight.step(assets, attacker, defender, false);
                    }
                    let read = fight.fight_read();
                    let telemetry = fight.machine().diagnostic_telemetry();
                    if tick == 0 {
                        assert_eq!(read.fighters[0].move_id, THROW_MOVE);
                        assert_eq!(read.fighters[1].move_id, THROWN_MOVE);
                    }
                    assert_eq!(
                        fight
                            .machine()
                            .toggle_state()
                            .throw
                            .st_ryu_capture_active(1),
                        tick < 39
                    );
                    assert_eq!(
                        read.fighters[0].move_id == 1,
                        tick >= 59,
                        "attacker at +{tick}"
                    );
                    if tick == 39 {
                        assert_eq!(
                            axis_x(&fight, telemetry.p2) - origin,
                            if leftward { -127 } else { 127 }
                        );
                        assert_eq!(height(&fight, telemetry.p2), 107);
                    }
                    if tick == landing {
                        assert_eq!(height(&fight, telemetry.p2), 0);
                    }
                    if tick == recovery {
                        assert!(!read.fighters[1].reacting() && !read.fighters[1].getting_up);
                    }
                    if tick == recovery + 1 {
                        assert!(read.fighters[1].attacking());
                    }
                    assert_eq!(
                        initial_hp - fight.machine().era_fighters()[1].sub_hp,
                        if tick >= damage_tick { lost } else { 0 },
                        "damage at +{tick}"
                    );
                    if tick == damage_tick {
                        assert!(read.landed_by(Side::P1).is_some());
                    }
                }
            }
        }
    }
}
