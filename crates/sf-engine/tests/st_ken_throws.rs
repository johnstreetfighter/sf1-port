//! World 940223 Ken ground throws through the Local Versus controller path.

#[allow(dead_code)]
#[path = "support/alpha.rs"]
mod support;

use sf_engine::combo::throw::{THROWN_MOVE, THROW_MOVE};
use sf_engine::era::{GaugeMode, Loadout, Revision};
use sf_engine::front::{fight::Fight, GameSession};
use sf_video::callouts::Side;
use support::{local_fight, local_select_with_p1, pad, DOWN, LEFT, RIGHT, UP};

const MP: usize = 1;
const HP: usize = 2;
const MK: usize = 4;
const HK: usize = 5;

fn base_fight(ryu: Loadout, ken: Loadout) -> Option<(GameSession, Fight)> {
    let select = local_select_with_p1(GaugeMode::EraDefault, Revision::St, ryu)?;
    let session = local_fight(&select, Revision::St, ken);
    let mut fight = session.fight().unwrap().clone();
    for _ in 0..94 {
        fight.step(session.assets(), pad(0, None), pad(0, None), false);
    }
    Some((session, fight))
}

fn at_gap(mut fight: Fight, gap: i32) -> Fight {
    let telemetry = fight.machine().diagnostic_telemetry();
    let mut snapshot = fight.save_netplay_snapshot(0);
    for (fighter, target) in [(telemetry.p1, 212 - gap), (telemetry.p2, 212)] {
        let entity = usize::from(fighter.entity_offset);
        let ram = &mut snapshot.machine.state.objectram;
        let pose_x = i32::from(ram[entity + 0x26] as i8);
        let delta = (target - fighter.center_x + pose_x) as u16;
        let cells = (usize::from(ram[entity + 0xc]) + 1) * (usize::from(ram[entity + 0xd]) + 1);
        for cell in 0..cells {
            let index = entity + cell * 0x40 + 6;
            let x = u16::from_be_bytes([ram[index], ram[index + 1]]).wrapping_add(delta);
            ram[index..index + 2].copy_from_slice(&x.to_be_bytes());
        }
    }
    fight.restore_netplay_snapshot(&snapshot);
    fight
}

#[test]
fn both_ken_loadouts_throw_at_77_and_fall_back_at_78() {
    for loadout in [Loadout::Standard, Loadout::Old] {
        let Some((session, base)) = base_fight(Loadout::Standard, loadout) else {
            return;
        };
        for direction in [LEFT, RIGHT] {
            for (gap, button, capture) in [
                (77, MP, true),
                (77, HP, true),
                (77, MK, true),
                (77, HK, true),
                (78, MP, false),
                (78, MK, false),
                (78, HK, false),
                (38, 0, false),
                (38, 3, false),
            ] {
                let mut fight = at_gap(base.clone(), gap);
                fight.step(
                    session.assets(),
                    pad(0, None),
                    pad(direction, Some(button)),
                    false,
                );
                assert_eq!(
                    fight.fight_read().fighters[0].move_id == THROWN_MOVE,
                    capture,
                    "{loadout:?} direction {direction} gap {gap} button {button}"
                );
            }
        }
    }
}

#[test]
fn ken_ground_throws_preserve_damage_release_and_recovery() {
    for (ryu, ken) in [
        (Loadout::Standard, Loadout::Standard),
        (Loadout::Standard, Loadout::Old),
        (Loadout::Old, Loadout::Standard),
    ] {
        let Some((session, base)) = base_fight(ryu, ken) else {
            return;
        };
        for direction in [LEFT, RIGHT] {
            let back = direction == RIGHT;
            for button in [MP, MK, HK] {
                let knee = ken == Loadout::Standard && button == MK;
                let wheel = button != MP && !knee;
                for reply in [13, 14] {
                    let softened = ryu == Loadout::Standard && !knee && reply == 13;
                    let (release, ken_ready, ryu_ready, damage): (_, _, _, &[(u8, u16)]) = if knee {
                        (
                            179,
                            179,
                            229,
                            &[(14, 173), (48, 27), (82, 27), (116, 27), (150, 27)],
                        )
                    } else if wheel && softened {
                        (62, 109, if back { 122 } else { 117 }, &[(62, 107)])
                    } else if wheel && back {
                        (62, 109, 162, &[(101, 213)])
                    } else if wheel {
                        (62, 109, 168, &[(95, 213)])
                    } else if softened {
                        (44, 74, 100, &[(44, 107)])
                    } else {
                        (44, 74, 153, &[(79, 213)])
                    };
                    let mut fight = at_gap(base.clone(), 38);
                    let mut previous_hp = fight.machine().era_fighters()[0].sub_hp;
                    for tick in 0..=ryu_ready {
                        let p1 = if tick == reply {
                            pad(RIGHT, Some(MP))
                        } else {
                            pad(0, None)
                        };
                        let p2 = if tick == 0 {
                            pad(direction, Some(button))
                        } else {
                            pad(0, None)
                        };
                        let saved = (tick == reply || tick == release || tick == 82 && knee)
                            .then(|| fight.save_netplay_snapshot(u32::from(tick)));
                        fight.step(session.assets(), p1, p2, false);
                        if let Some(saved) = saved {
                            let checksum = fight.state_checksum();
                            fight.restore_netplay_snapshot(&saved);
                            fight.step(session.assets(), p1, p2, false);
                            assert_eq!(fight.state_checksum(), checksum);
                        }
                        let read = fight.fight_read();
                        let hp = fight.machine().era_fighters()[0].sub_hp;
                        let lost = damage
                            .iter()
                            .find(|(at, _)| *at == tick)
                            .map_or(0, |(_, lost)| *lost);
                        let context = || {
                            format!("{ryu:?} {ken:?} button {button}, back {back}, reply {reply}, tick {tick}")
                        };
                        assert_eq!(previous_hp - hp, lost, "{}", context());
                        if lost > 0 {
                            assert!(read.landed_by(Side::P2).is_some(), "{}", context());
                        }
                        previous_hp = hp;
                        assert_eq!(
                            fight
                                .machine()
                                .toggle_state()
                                .throw
                                .st_ryu_capture_active(0),
                            tick < release,
                            "{}",
                            context()
                        );
                        assert_eq!(
                            read.fighters[1].move_id == THROW_MOVE,
                            tick < ken_ready,
                            "{}",
                            context()
                        );
                        assert_eq!(
                            read.fighters[0].move_id == 1,
                            tick == ryu_ready,
                            "{}",
                            context()
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn knee_bash_direction_edges_change_the_hold_through_controller_inputs() {
    let Some((session, base)) = base_fight(Loadout::Standard, Loadout::Standard) else {
        return;
    };
    let mut results = Vec::new();
    for (mash_ryu, mash_ken) in [(false, false), (true, false), (false, true)] {
        let mut fight = at_gap(base.clone(), 38);
        let initial_hp = fight.machine().era_fighters()[0].sub_hp;
        fight.step(session.assets(), pad(0, None), pad(LEFT, Some(MK)), false);
        for tick in 1..=260 {
            let pulse = if tick % 4 == 0 {
                [DOWN, LEFT, UP, RIGHT][tick / 4 % 4]
            } else {
                0
            };
            let ryu = pad(if mash_ryu { pulse } else { 0 }, None);
            let ken = pad(if mash_ken { pulse } else { 0 }, None);
            fight.step(session.assets(), ryu, ken, false);
            if !fight.machine().toggle_state().throw.st_ryu_capture_active(0) {
                let loss = initial_hp - fight.machine().era_fighters()[0].sub_hp;
                results.push((tick, loss));
                break;
            }
        }
    }
    let [tap, escaped, mashed] = <[_; 3]>::try_from(results).unwrap();
    assert_eq!(tap, (179, 281));
    assert!(escaped.0 < tap.0 && escaped.1 < tap.1, "{escaped:?}");
    assert!(mashed.0 > tap.0 && mashed.1 > tap.1, "{mashed:?}");
}
