//! World Super Turbo jump-normal cancel witnesses through Local Versus.

#[path = "support/alpha.rs"]
mod support;

use sf_engine::era::{GaugeMode, Loadout, Revision};
use sf_engine::front::fight::Fight;
use sf_video::{callouts::Side, romset::Assets};
use support::{local_fight, local_select_with_p1, pad, DOWN, LEFT, LP, RIGHT, UP};

const MP: usize = 1;
const HP: usize = 2;
const LK: usize = 3;
const MK: usize = 4;
const HK: usize = 5;
const AIR_TATSUMAKI: u8 = 0x20;

fn position(fight: &Fight, ryu_x: i16, gap: i16) -> Fight {
    let read = fight.machine().diagnostic_telemetry();
    let mut placed = fight.clone();
    let mut snapshot = placed.save_netplay_snapshot(0);
    for (fighter, target) in [(read.p1, ryu_x), (read.p2, ryu_x + gap)] {
        let objects = &mut snapshot.machine.state.objectram;
        let entity = usize::from(fighter.entity_offset);
        let axis = fighter.center_x - i32::from(objects[entity + 0x26] as i8);
        let cells =
            (usize::from(objects[entity + 0xc]) + 1) * (usize::from(objects[entity + 0xd]) + 1);
        let delta = (i32::from(target) - axis) as u16;
        for cell in (entity..).step_by(0x40).take(cells) {
            let x = u16::from_be_bytes([objects[cell + 6], objects[cell + 7]]);
            objects[cell + 6..cell + 8].copy_from_slice(&x.wrapping_add(delta).to_be_bytes());
        }
    }
    placed.restore_netplay_snapshot(&snapshot);
    placed
}

fn air_tatsumaki(fight: &Fight) -> bool {
    let machine = fight.machine();
    let ryu = machine.diagnostic_telemetry().p1;
    let e = usize::from(ryu.entity_offset);
    let stream = u32::from_be_bytes(
        machine.state.objectram[e + 0x2c..e + 0x30]
            .try_into()
            .unwrap(),
    );
    ryu.move_id == AIR_TATSUMAKI && sf_engine::data_bank::in_bank(stream)
}

fn jumping_punch_contact(
    base: &Fight,
    assets: &Assets,
    button: usize,
    air_target: bool,
) -> Option<Fight> {
    let neutral = pad(0, None);
    let candidates = if air_target {
        vec![(32, 7)]
    } else {
        [55, 72, 96, 128]
            .into_iter()
            .flat_map(|gap| (11..=42).map(move |press| (gap, press)))
            .collect()
    };
    candidates.into_iter().find_map(|(gap, press)| {
        let mut fight = position(base, 160, gap);
        for tick in 0..press {
            let jump = tick < 3;
            fight.step(
                assets,
                if jump { pad(UP | RIGHT, None) } else { neutral },
                if air_target && jump {
                    pad(UP, None)
                } else {
                    neutral
                },
                false,
            );
        }
        for tick in 0..12 {
            fight.step(
                assets,
                pad(
                    if tick == 0 { 0 } else { DOWN },
                    (tick == 0).then_some(button),
                ),
                neutral,
                false,
            );
            if fight.fight_read().landed_by(Side::P1).is_some()
                && fight.fight_read().fighters[0].state & 3 == 1
            {
                return Some(fight);
            }
        }
        None
    })
}

#[test]
fn both_st_ryus_can_cancel_a_connected_directional_jump_punch_into_air_tatsumaki() {
    for loadout in [Loadout::Standard, Loadout::Old] {
        let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, loadout)
        else {
            return;
        };
        let session = local_fight(&select, Revision::St, Loadout::Standard);
        let assets = session.assets();
        let mut base = session.fight().unwrap().clone();
        let neutral = pad(0, None);
        for _ in 0..94 {
            base.step(assets, neutral, neutral, false);
        }
        for (button, air_target) in [(MP, true), (LP, false), (MP, false), (HP, false)] {
            let Some(mut fight) = jumping_punch_contact(&base, assets, button, air_target) else {
                panic!("{loadout:?} button {button} air target {air_target} did not hit")
            };
            assert_eq!(
                fight.machine().toggle_state().combo.contact_freeze[..2],
                [14, 14]
            );
            let snapshot = fight.save_netplay_snapshot(0);
            let motion = [pad(DOWN, None), pad(DOWN | LEFT, None), pad(LEFT, Some(HK))];
            for input in motion {
                fight.step(assets, input, neutral, false);
            }
            assert!(
                air_tatsumaki(&fight),
                "{loadout:?} button {button} air target {air_target}, state {:?}",
                fight.fight_read().fighters
            );
            let checksum = fight.state_checksum();
            fight.restore_netplay_snapshot(&snapshot);
            for input in motion {
                fight.step(assets, input, neutral, false);
            }
            assert_eq!(fight.state_checksum(), checksum);
        }

        let contact = jumping_punch_contact(&base, assets, MP, true).unwrap();
        for final_press in [15, 16] {
            let mut fight = contact.clone();
            for tick in 1..=final_press {
                let input = match final_press - tick {
                    2 => pad(DOWN, None),
                    1 => pad(DOWN | LEFT, None),
                    0 => pad(LEFT, Some(HK)),
                    _ => neutral,
                };
                fight.step(assets, input, neutral, false);
            }
            assert_eq!(
                air_tatsumaki(&fight),
                final_press == 15,
                "{loadout:?} MP hit +{final_press}"
            );
        }
    }
}

#[test]
fn an_early_whiffed_jump_punch_can_cancel_but_a_late_one_cannot() {
    let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, Loadout::Standard)
    else {
        return;
    };
    let session = local_fight(&select, Revision::St, Loadout::Standard);
    let assets = session.assets();
    let mut base = session.fight().unwrap().clone();
    let neutral = pad(0, None);
    for _ in 0..94 {
        base.step(assets, neutral, neutral, false);
    }
    let mut fight = position(&base, 150, 180);
    for tick in 0..7 {
        fight.step(
            assets,
            if tick < 3 {
                pad(UP | RIGHT, None)
            } else {
                neutral
            },
            neutral,
            false,
        );
    }
    fight.step(assets, pad(0, Some(MP)), neutral, false);
    while !fight.fight_read().fighters[0].attack_active {
        assert_eq!(fight.fight_read().fighters[0].move_id, 0x21);
        fight.step(assets, neutral, neutral, false);
    }
    for wait in [0, 10] {
        let mut replay = fight.clone();
        for _ in 0..wait {
            replay.step(assets, neutral, neutral, false);
        }
        for input in [pad(DOWN, None), pad(DOWN | LEFT, None), pad(LEFT, Some(HK))] {
            replay.step(assets, input, neutral, false);
            assert!(replay.fight_read().landed_by(Side::P1).is_none());
        }
        assert_eq!(air_tatsumaki(&replay), wait == 0, "whiff delay {wait}");
    }
}

#[test]
fn st_ryu_air_tatsumaki_uses_world_turbo_two_strike_windows() {
    let neutral = pad(0, None);
    for loadout in [Loadout::Standard, Loadout::Old] {
        let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, loadout)
        else {
            return;
        };
        let session = local_fight(&select, Revision::St, Loadout::Standard);
        let assets = session.assets();
        let mut base = session.fight().unwrap().clone();
        for _ in 0..94 {
            base.step(assets, neutral, neutral, false);
        }
        let (reach, radius) = if loadout == Loadout::Old {
            (35, 34)
        } else {
            (54, 15)
        };
        let phases: [(usize, &[u8], &[u8], u8); 3] = [
            (LK, &[0], &[6], 42),
            (MK, &[0, 12], &[6, 18], 51),
            (HK, &[0, 10, 20], &[5, 15, 25], 54),
        ];
        let (neutral_landings, back_landings) = if loadout == Loadout::Old {
            ([42, 50, 54], [43, 51, 55])
        } else {
            ([43, 52, 55], [44, 52, 56])
        };
        for (strength, (kick, front, rear, forward_landing)) in phases.into_iter().enumerate() {
            for (jump, first_step, landing) in [
                (UP | RIGHT, [4, 5, 5][strength], forward_landing),
                (UP, [1, 2, 2][strength], neutral_landings[strength]),
                (UP | LEFT, [-5, -5, -6][strength], back_landings[strength]),
            ] {
                // Leave space behind Ryu as well as in front of him. The
                // forward case needs the larger gap to keep its last kick clear.
                let mut fight = if jump == (UP | LEFT) {
                    position(&base, 210, 125)
                } else {
                    position(&base, 105, 230)
                };
                for tick in 0..10 {
                    let input = if tick < 3 { pad(jump, None) } else { neutral };
                    fight.step(assets, input, neutral, false);
                }
                for input in [pad(DOWN, None), pad(DOWN | LEFT, None)] {
                    fight.step(assets, input, neutral, false);
                }
                let launch_axis = ryu_axis(&fight);
                fight.step(assets, pad(LEFT, Some(kick)), neutral, false);
                let variant = format!("{loadout:?} kick {kick} jump {jump}");
                assert!(air_tatsumaki(&fight), "{variant} never activated");
                assert_eq!(
                    ryu_axis(&fight) - launch_axis,
                    first_step,
                    "{variant} initial movement"
                );
                let recovery = if loadout == Loadout::Standard && kick == MK && jump == (UP | LEFT)
                {
                    6
                } else {
                    5
                };

                for tick in 0..=landing + recovery {
                    let machine = fight.machine();
                    let ryu = machine.diagnostic_telemetry().p1;
                    let entity = usize::from(ryu.entity_offset);
                    let object = &machine.state.objectram[entity..entity + 0x40];
                    let expected = if front
                        .iter()
                        .any(|start| (*start..*start + 3).contains(&tick))
                    {
                        Some(reach)
                    } else if rear
                        .iter()
                        .any(|start| (*start..*start + 3).contains(&tick))
                    {
                        Some(-reach)
                    } else {
                        None
                    };
                    let stream = u32::from_be_bytes(object[0x2c..0x30].try_into().unwrap());
                    let phase = format!("{variant} at +{tick}, state {:02x} move {:02x} stream {stream:#x} hold {} duration {}",
                    ryu.state_byte, ryu.move_id, object[0xb], object[0x33]);
                    assert_eq!(ryu.attack_active, expected.is_some(), "{phase}");
                    if let Some(x) = expected {
                        let attack = ryu.attack_hitbox.unwrap();
                        let axis = ryu.center_x - i32::from(object[0x26] as i8);
                        let top = i32::from(i16::from_be_bytes([object[4], object[5]]));
                        let cell = if object[2] & 4 == 0 { 16 } else { 32 };
                        let foot = top + (i32::from(object[0xc]) + 1) * cell;
                        assert_eq!(
                            (
                                attack.center_x - axis,
                                foot - attack.center_y,
                                attack.half_w,
                                attack.half_h
                            ),
                            (x, 67, radius, 16),
                            "{phase}"
                        );
                    }
                    if tick < landing {
                        assert_eq!(ryu.state_byte & 4, 0, "{phase}");
                    } else if tick < landing + recovery {
                        assert_ne!(ryu.state_byte & 4, 0, "{phase}");
                    } else {
                        assert_eq!(ryu.state_byte & 0x80, 0, "{phase}");
                    }
                    fight.step(assets, neutral, neutral, false);
                }
            }
        }
    }
}

fn ryu_axis(fight: &Fight) -> i32 {
    let machine = fight.machine();
    let ryu = machine.diagnostic_telemetry().p1;
    let offset = machine.state.objectram[usize::from(ryu.entity_offset) + 0x26] as i8;
    ryu.center_x - i32::from(offset)
}

#[test]
fn world_turbo_two_directional_medium_punch_uses_loadout_strikes() {
    let neutral = pad(0, None);
    for loadout in [Loadout::Standard, Loadout::Old] {
        let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, loadout)
        else {
            return;
        };
        let session = local_fight(&select, Revision::St, Loadout::Standard);
        let assets = session.assets();
        let mut base = session.fight().unwrap().clone();
        for _ in 0..94 {
            base.step(assets, neutral, neutral, false);
        }

        for jump in [UP | RIGHT, UP | LEFT, UP] {
            let mut fight = position(&base, 190, 140);
            for tick in 0..7 {
                fight.step(
                    assets,
                    if tick < 3 { pad(jump, None) } else { neutral },
                    neutral,
                    false,
                );
            }
            fight.step(assets, pad(0, Some(MP)), neutral, false);
            for _ in 0..4 {
                if (0x20..=0x22).contains(&fight.fight_read().fighters[0].move_id) {
                    break;
                }
                fight.step(assets, neutral, neutral, false);
            }
            for tick in 0..25 {
                let ryu = fight.machine().diagnostic_telemetry().p1;
                let (active, expected) = if jump == UP {
                    ((4..10).contains(&tick), None)
                } else if loadout == Loadout::Old {
                    ((4..24).contains(&tick), Some((30, 31, 11)))
                } else {
                    (
                        (4..13).contains(&tick),
                        Some(if tick < 9 { (43, 31, 11) } else { (29, 15, 17) }),
                    )
                };
                assert_eq!(ryu.attack_active, active, "{loadout:?} jump {jump} +{tick}");
                if let Some((reach, w, h)) = expected.filter(|_| active) {
                    let hitbox = ryu.attack_hitbox.unwrap();
                    assert_eq!(
                        (
                            hitbox.center_x - ryu_axis(&fight),
                            hitbox.half_w,
                            hitbox.half_h
                        ),
                        (reach, w, h),
                        "{loadout:?} jump {jump} +{tick}"
                    );
                }
                fight.step(assets, neutral, neutral, false);
            }
        }
    }
}

#[test]
fn st_new_directional_medium_punch_can_rehit_airborne_ken() {
    let neutral = pad(0, None);
    for loadout in [Loadout::Standard, Loadout::Old] {
        let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, loadout)
        else {
            return;
        };
        let session = local_fight(&select, Revision::St, Loadout::Standard);
        let assets = session.assets();
        let mut base = session.fight().unwrap().clone();
        for _ in 0..94 {
            base.step(assets, neutral, neutral, false);
        }
        let Some(mut fight) = jumping_punch_contact(&base, assets, MP, true) else {
            panic!("{loadout:?} jump MP did not hit jumping Ken");
        };
        assert_eq!(
            fight.machine().toggle_state().combo.contact_freeze[..2],
            [14, 14]
        );
        let snapshot = fight.save_netplay_snapshot(0);
        let mut checkpoints = Vec::new();
        for _ in 0..2 {
            fight.restore_netplay_snapshot(&snapshot);
            let mut followup = Vec::new();
            for tick in 1..=25 {
                let airborne = fight.fight_read().fighters[1].state & 3 == 1;
                fight.step(assets, neutral, neutral, false);
                if fight.fight_read().landed_by(Side::P1).is_some() {
                    assert!(airborne, "Ken landed before the follow-up at +{tick}");
                    assert_eq!(
                        fight.machine().toggle_state().combo.contact_freeze[..2],
                        [14, 14]
                    );
                    followup.push(tick);
                }
            }
            checkpoints.push((followup, fight.state_checksum()));
        }
        assert_eq!(checkpoints[0], checkpoints[1]);
        if loadout == Loadout::Standard {
            assert_eq!(checkpoints[0].0, [19]);
        } else {
            assert!(checkpoints[0].0.is_empty());
        }
    }
}

#[test]
fn st_ryu_fresh_jump_mp_keeps_ken_in_the_same_juggle() {
    let neutral = pad(0, None);
    let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, Loadout::Standard)
    else {
        return;
    };
    let session = local_fight(&select, Revision::St, Loadout::Standard);
    let assets = session.assets();
    let mut base = session.fight().unwrap().clone();
    for _ in 0..94 {
        base.step(assets, neutral, neutral, false);
    }
    let mut fight = jumping_punch_contact(&base, assets, MP, true).unwrap();
    let snapshot = fight.save_netplay_snapshot(0);
    let mut previous_checksum = None;
    for _ in 0..2 {
        fight.restore_netplay_snapshot(&snapshot);
        let mut hits = Vec::new();
        for tick in 1..=110 {
            if tick <= 81 {
                let ken = fight.fight_read().fighters[1];
                assert!(
                    ken.hitstun() && ken.state & 3 == 1,
                    "Ken left airborne reaction before +{tick}"
                );
            }
            let input = match tick {
                71..=73 => pad(UP | RIGHT, None),
                77 => pad(0, Some(MP)),
                _ => neutral,
            };
            fight.step(assets, input, neutral, false);
            if fight.fight_read().landed_by(Side::P1).is_some() {
                let hitbox = fight
                    .machine()
                    .diagnostic_telemetry()
                    .p1
                    .attack_hitbox
                    .unwrap();
                hits.push((tick, hitbox.half_w));
                assert_eq!(
                    fight.machine().toggle_state().combo.contact_freeze[..2],
                    [14, 14]
                );
            }
        }
        assert_eq!(hits, [(19, 15), (81, 31)]);
        let checksum = fight.state_checksum();
        if let Some(previous) = previous_checksum {
            assert_eq!(checksum, previous);
        }
        previous_checksum = Some(checksum);
    }
}

#[test]
fn st_ryu_fresh_jump_mp_can_feed_an_airborne_shinku() {
    let neutral = pad(0, None);
    let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, Loadout::Standard)
    else {
        return;
    };
    let session = local_fight(&select, Revision::St, Loadout::Standard);
    let assets = session.assets();
    let mut base = session.fight().unwrap().clone();
    for _ in 0..94 {
        base.step(assets, neutral, neutral, false);
    }
    let mut fight = jumping_punch_contact(&base, assets, MP, true).unwrap();
    let mut stock = fight.save_netplay_snapshot(0);
    stock.machine.toggle_state.era.fighters[0].super_meter = 48;
    fight.restore_netplay_snapshot(&stock);
    for tick in 1..=81 {
        let input = match tick {
            71..=73 => pad(UP | RIGHT, None),
            77 => pad(0, Some(MP)),
            _ => neutral,
        };
        fight.step(assets, input, neutral, false);
    }
    for (first, expected) in [(112u32, None), (120, Some((125, 157)))] {
        let mut replay = fight.clone();
        let mut hits = Vec::new();
        let mut start = None;
        for tick in 82u32..=190 {
            let input = match tick.checked_sub(first) {
                Some(0) => pad(DOWN, None),
                Some(1) => pad(DOWN | RIGHT, None),
                Some(2) => pad(RIGHT, None),
                Some(3) => pad(DOWN, None),
                Some(4) => pad(DOWN | RIGHT, None),
                Some(5) => pad(RIGHT, Some(LP)),
                _ => neutral,
            };
            let airborne = replay.fight_read().fighters[1].state & 3 == 1;
            replay.step(assets, input, neutral, false);
            if replay.fight_read().fighters[0].move_id == sf_engine::era_fight::SF2_SUPER {
                start.get_or_insert(tick);
            }
            if let Some(hit) = replay.fight_read().landed_by(Side::P1) {
                hits.push((tick, airborne, hit.into_hitstun));
            }
        }
        match expected {
            Some((entry, hit)) => {
                assert_eq!(start, Some(entry));
                assert_eq!(hits, [(hit, true, true)]);
            }
            None => {
                assert_eq!(start, None);
                assert!(hits.is_empty());
            }
        }
    }
}

#[test]
fn st_ryu_three_jump_mp_hits_leave_two_airborne_shinku_hits() {
    let neutral = pad(0, None);
    let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, Loadout::Standard)
    else {
        return;
    };
    let session = local_fight(&select, Revision::St, Loadout::Standard);
    let assets = session.assets();
    let mut base = session.fight().unwrap().clone();
    for _ in 0..94 {
        base.step(assets, neutral, neutral, false);
    }
    let mut fight = position(&base, 160, 32);
    let mut stock = fight.save_netplay_snapshot(0);
    stock.machine.toggle_state.era.fighters[0].super_meter = 48;
    fight.restore_netplay_snapshot(&stock);
    assert_eq!(
        fight.machine().era_fighters()[1].sub_hp,
        sf_engine::era_fight::FULL_SUB_HP
    );

    let mut hits = Vec::new();
    let mut super_start = None;
    let mut first_shot = None;
    for tick in 0..=195 {
        let ryu = match tick {
            0..=2 | 74..=76 => pad(UP | RIGHT, None),
            7 | 80 => pad(0, Some(MP)),
            133 | 136 => pad(DOWN, None),
            134 | 137 => pad(DOWN | RIGHT, None),
            135 => pad(RIGHT, None),
            138 => pad(RIGHT, Some(LP)),
            _ => neutral,
        };
        let ken = if tick < 3 { pad(UP, None) } else { neutral };
        let before = fight.fight_read().fighters[1];
        if (12..=178).contains(&tick) {
            assert!(
                before.hitstun() && before.state & 3 == 1,
                "Ken left airborne reaction at {tick}"
            );
        }
        fight.step(assets, ryu, ken, false);
        if fight.fight_read().fighters[0].move_id == sf_engine::era_fight::SF2_SUPER {
            super_start.get_or_insert(tick);
        }
        if let Some(hit) = fight.fight_read().landed_by(Side::P1) {
            assert_eq!(before.state & 3, 1);
            assert_eq!(hit.into_hitstun, tick != 11);
            hits.push(tick);
            if tick == 166 {
                first_shot = Some(fight.save_netplay_snapshot(tick));
            }
        }
    }
    assert_eq!(super_start, Some(138));
    assert_eq!(hits, [11, 30, 84, 166, 178]);
    assert!(fight.machine().era_fighters()[1].sub_hp > 0);
    let checksum = fight.state_checksum();
    fight.restore_netplay_snapshot(&first_shot.unwrap());
    for tick in 167..=195 {
        fight.step(assets, neutral, neutral, false);
        assert_eq!(
            fight.fight_read().landed_by(Side::P1).is_some(),
            tick == 178
        );
    }
    assert_eq!(fight.state_checksum(), checksum);
}

#[test]
fn st_ryu_one_airborne_shinku_hit_allows_a_fresh_mp() {
    let neutral = pad(0, None);
    let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, Loadout::Standard)
    else {
        return;
    };
    let session = local_fight(&select, Revision::St, Loadout::Standard);
    let assets = session.assets();
    let mut base = session.fight().unwrap().clone();
    for _ in 0..94 {
        base.step(assets, neutral, neutral, false);
    }
    let mut fight = position(&base, 160, 32);
    let mut stock = fight.save_netplay_snapshot(0);
    stock.machine.toggle_state.era.fighters[0].super_meter = 48;
    fight.restore_netplay_snapshot(&stock);
    assert_eq!(
        fight.machine().era_fighters()[1].sub_hp,
        sf_engine::era_fight::FULL_SUB_HP
    );

    let input = |tick| match tick {
        24 | 27 => pad(DOWN, None),
        25 | 28 => pad(DOWN | RIGHT, None),
        26 => pad(RIGHT, None),
        29 => pad(RIGHT, Some(LP)),
        102..=104 => pad(UP | RIGHT, None),
        106 => pad(0, Some(MP)),
        _ => neutral,
    };
    let mut snapshot = None;
    let mut super_start = None;
    let mut hits = Vec::new();
    for tick in 0..=136 {
        // Meet the first projectile on the descent of Ken's measured neutral jump.
        let ken = if (2..=4).contains(&tick) {
            pad(UP, None)
        } else {
            neutral
        };
        let before = fight.fight_read().fighters[1];
        if (58..=111).contains(&tick) {
            assert!(
                before.hitstun() && before.state & 3 == 1,
                "Ken left airborne reaction at {tick}"
            );
        }
        fight.step(assets, input(tick), ken, false);
        if fight.fight_read().fighters[0].move_id == sf_engine::era_fight::SF2_SUPER {
            super_start.get_or_insert(tick);
        }
        if let Some(hit) = fight.fight_read().landed_by(Side::P1) {
            assert_eq!(before.state & 3, 1);
            assert_eq!(hit.into_hitstun, tick != 57);
            hits.push(tick);
            if tick == 57 {
                snapshot = Some(fight.save_netplay_snapshot(tick));
            }
        }
    }
    assert_eq!(super_start, Some(29));
    assert_eq!(hits, [57, 111]);
    let checksum = fight.state_checksum();
    fight.restore_netplay_snapshot(&snapshot.unwrap());
    for tick in 58..=136 {
        fight.step(assets, input(tick), neutral, false);
        assert_eq!(
            fight.fight_read().landed_by(Side::P1).is_some(),
            tick == 111
        );
    }
    assert_eq!(fight.state_checksum(), checksum);
}

#[test]
fn st_ryu_single_mp_opener_can_continue_across_fresh_jumps() {
    let neutral = pad(0, None);
    let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, Loadout::Standard)
    else {
        return;
    };
    let session = local_fight(&select, Revision::St, Loadout::Standard);
    let assets = session.assets();
    let mut base = session.fight().unwrap().clone();
    for _ in 0..94 {
        base.step(assets, neutral, neutral, false);
    }
    for (gap, jump_starts, expected) in [
        (80, &[0, 55][..], [(11, 31), (64, 31), (86, 15)]),
        (96, &[0, 55, 108][..], [(11, 31), (64, 31), (118, 31)]),
    ] {
        let mut fight = position(&base, 160, gap);
        let mut hits = Vec::new();
        for tick in 0..=expected[2].0 + 20 {
            let ryu = if jump_starts
                .iter()
                .any(|start| (*start..*start + 3).contains(&tick))
            {
                pad(UP | RIGHT, None)
            } else if tick == 7 || jump_starts[1..].iter().any(|start| tick == *start + 4) {
                pad(0, Some(MP))
            } else {
                neutral
            };
            let ken = if tick < 3 { pad(UP, None) } else { neutral };
            let before = fight.fight_read().fighters[1];
            if (12..=expected[2].0).contains(&tick) {
                assert!(
                    before.hitstun() && before.state & 3 == 1,
                    "Ken left airborne reaction at {tick}"
                );
            }
            fight.step(assets, ryu, ken, false);
            if fight.fight_read().landed_by(Side::P1).is_some() {
                let hitbox = fight
                    .machine()
                    .diagnostic_telemetry()
                    .p1
                    .attack_hitbox
                    .unwrap();
                hits.push((tick, hitbox.half_w));
                assert_eq!(
                    fight.machine().toggle_state().combo.contact_freeze[..2],
                    [14, 14]
                );
            }
        }
        assert_eq!(hits, expected, "starting gap {gap}");
    }
}
