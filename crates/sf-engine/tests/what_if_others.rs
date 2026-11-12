//! Native match checks for the post-1987 What-If kits.

#[path = "support/alpha.rs"]
#[allow(dead_code)]
mod support;

use sf_engine::era::{GaugeMode, Loadout, Revision};
use sf_engine::front::fight::Fight;
use sf_engine::front::session::GameSession;
use sf_engine::front::PlayerInputs;
use sf_engine::roster::TournamentFighter::{self, Adon, Birdie, Eagle, Gen, Sagat};
use sf_video::callouts::{CalloutKind, Side};
use sf_video::era_hud::{GaugeRow, GaugeState};
use sf_video::romset::Assets;
use support::{
    assert_training_move_demo, local_fight, local_select_with_fighter, pad,
    training_fight_with_fighter, DOWN, LEFT, LP, RIGHT, UP,
};

const LK: usize = 3;
const MK: usize = 4;
const HK: usize = 5;
const QCF: &[u8] = &[DOWN, DOWN | RIGHT, RIGHT];
const QCB: &[u8] = &[DOWN, DOWN | LEFT, LEFT];
const QCF2: &[u8] = &[DOWN, DOWN | RIGHT, RIGHT, DOWN, DOWN | RIGHT, RIGHT];
const QCB2: &[u8] = &[DOWN, DOWN | LEFT, LEFT, DOWN, DOWN | LEFT, LEFT];

fn bout(fighter: TournamentFighter, revision: Revision, loadout: Loadout) -> Option<GameSession> {
    let select = local_select_with_fighter(GaugeMode::EraDefault, fighter, revision, loadout)?;
    Some(local_fight(&select, Revision::Sf1Plus, Loadout::Standard))
}

fn step(fight: &mut Fight, assets: &Assets, input: PlayerInputs) {
    fight.step(assets, input, PlayerInputs::default(), false);
}

fn hold(fight: &mut Fight, assets: &Assets, input: PlayerInputs, frames: usize) {
    for _ in 0..frames {
        step(fight, assets, input);
    }
}

fn command(fight: &mut Fight, assets: &Assets, motion: &[u8], buttons: &[usize]) {
    let (&last, before) = motion.split_last().unwrap();
    for &direction in before {
        step(fight, assets, pad(direction, None));
    }
    let mut input = pad(last, None);
    for &button in buttons {
        input.buttons[button] = true;
    }
    step(fight, assets, input);
}

fn move_id(fight: &Fight) -> u8 {
    fight.fight_read().fighters[0].move_id
}

fn meter(fight: &Fight) -> u8 {
    fight.machine().era_fighters()[0].super_meter
}

fn close_range(fight: &mut Fight, assets: &Assets) {
    for _ in 0..100 {
        let telemetry = fight.machine().diagnostic_telemetry();
        if telemetry.p1.center_x.abs_diff(telemetry.p2.center_x) <= 48 {
            hold(fight, assets, PlayerInputs::default(), 8);
            return;
        }
        step(fight, assets, pad(RIGHT, None));
    }
    panic!("the fighters did not reach close range");
}

fn set_meter(fight: &mut Fight, amount: u8) {
    let mut snapshot = fight.save_netplay_snapshot(0);
    snapshot.machine.toggle_state.era.fighters[0].super_meter = amount;
    fight.restore_netplay_snapshot(&snapshot);
}

fn assert_super(fight: &Fight, slot: u8, name: &'static str, remaining: u8) {
    assert_eq!(move_id(fight), slot, "{name}");
    assert_eq!(meter(fight), remaining, "{name} Super cost");
    assert!(fight.machine().era_super_freeze() > 0);
    assert!(fight
        .callouts()
        .frame()
        .side(Side::P1)
        .words
        .contains(&Some(CalloutKind::Super { name })));
}

#[test]
fn gen_can_switch_stances_use_crane_charge_and_activate_genei_jin() {
    let Some(session) = bout(Gen, Revision::Sf3ThreeS, Loadout::Art3) else {
        return;
    };
    let assets = session.assets();
    let base = session.fight().unwrap();
    let mut mantis = base.clone();
    command(&mut mantis, assets, &[RIGHT, DOWN, DOWN | RIGHT], &[LK]);
    assert_eq!(move_id(&mantis), 0x1b, "Mantis still has later-era Gekirou");
    let mut gen = base.clone();

    command(&mut gen, assets, &[0], &[LK, MK, HK]);
    assert_eq!(move_id(&gen), 0x1f, "KKK selects Crane");
    hold(&mut gen, assets, PlayerInputs::default(), 12);

    for (charge, release, button, slot) in [(LEFT, RIGHT, LP, 0x20), (DOWN, UP, HK, 0x21)] {
        let mut crane = gen.clone();
        hold(&mut crane, assets, pad(charge, None), 48);
        command(&mut crane, assets, &[release], &[button]);
        assert_eq!(move_id(&crane), slot, "Crane charge move");
        assert!((0..100).any(|_| {
            step(&mut crane, assets, PlayerInputs::default());
            crane.fight_read().fighters[0].attack_active
        }));
    }

    command(&mut gen, assets, &[0], &[0, 1, 2]);
    assert_eq!(move_id(&gen), 0x1f, "PPP returns to Mantis");

    let mut gen = base.clone();
    set_meter(&mut gen, 144);
    command(&mut gen, assets, QCF2, &[LP]);
    assert_super(&gen, 0x2b, "GENEI-JIN", 96);
    let mut active = false;
    for _ in 0..42 {
        step(&mut gen, assets, PlayerInputs::default());
        active |= gen.fight_read().fighters[0].attack_active;
    }
    assert!(!active, "Genei-Jin activation deals no damage");
    assert_eq!(
        gen.fight_read().fighters[0].state & 0x80,
        0,
        "Gen is actionable during the install"
    );

    let can_chain_normals = |mut fight: Fight| {
        command(&mut fight, assets, &[0], &[LP]);
        let first = move_id(&fight);
        assert!(matches!(first, 0x0d | 0x0e));
        step(&mut fight, assets, PlayerInputs::default());
        command(&mut fight, assets, &[0], &[MK]);
        move_id(&fight) == 0x15
    };
    assert!(
        can_chain_normals(gen.clone()),
        "Genei-Jin allows immediate normal cancels"
    );
    hold(&mut gen, assets, PlayerInputs::default(), 130);
    assert!(!can_chain_normals(gen), "the timed cancel window expires");
}

#[test]
fn birdie_has_charged_bull_head_negative_edge_horn_and_an_ultra() {
    let Some(session) = bout(Birdie, Revision::Usf4, Loadout::Ultra1) else {
        return;
    };
    let assets = session.assets();
    let base = session.fight().unwrap();
    let mut birdie = base.clone();
    hold(&mut birdie, assets, pad(LEFT, None), 48);
    command(&mut birdie, assets, &[RIGHT], &[LP]);
    assert_eq!(move_id(&birdie), 0x19);

    let mut birdie = base.clone();
    let mut pair = PlayerInputs::default();
    pair.buttons[..2].fill(true);
    hold(&mut birdie, assets, pair, 48);
    step(&mut birdie, assets, PlayerInputs::default());
    assert_eq!(move_id(&birdie), 0x1a);

    let mut birdie = base.clone();
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
        if revenge_ready(&birdie) {
            break;
        }
        close_range(&mut birdie, assets);
        birdie.step(assets, PlayerInputs::default(), pad(0, Some(2)), false);
        hold(&mut birdie, assets, PlayerInputs::default(), 70);
    }
    assert!(revenge_ready(&birdie));
    let super_before = meter(&birdie);
    command(&mut birdie, assets, QCF2, &[0, 1, 2]);
    assert_super(&birdie, 0x2a, "THE BIRDIE", super_before);
}

#[test]
fn eagle_and_adon_have_distinct_native_specials_and_finishers() {
    for (fighter, motion, button, special, finisher) in [
        (Eagle, QCF, LP, 0x20, "ALBION TEMPEST"),
        (
            Adon,
            &[RIGHT, DOWN, DOWN | RIGHT][..],
            LK,
            0x19,
            "JAGUAR AVALANCHE",
        ),
    ] {
        let Some(session) = bout(fighter, Revision::Sf3ThreeS, Loadout::Art3) else {
            return;
        };
        let assets = session.assets();
        let base = session.fight().unwrap();
        let mut fight = base.clone();
        command(&mut fight, assets, motion, &[button]);
        assert_eq!(move_id(&fight), special, "{fighter:?} special");

        let mut fight = base.clone();
        set_meter(&mut fight, 96);
        command(&mut fight, assets, QCF2, &[LP]);
        assert_super(&fight, 0x2b, finisher, 48);
        assert!((0..70).any(|_| {
            step(&mut fight, assets, PlayerInputs::default());
            fight.fight_read().fighters[0].attack_active
        }));
    }
}

#[test]
fn sagat_has_tiger_nexus_od_and_three_sf6_super_arts() {
    let Some(session) = bout(Sagat, Revision::Sf6, Loadout::Standard) else {
        return;
    };
    let assets = session.assets();
    let base = session.fight().unwrap();
    let mut sagat = base.clone();
    let drive_before = sagat.machine().era_fighters()[0]
        .sf6_drive_points()
        .unwrap();
    command(&mut sagat, assets, QCB, &[LK, MK]);
    assert_eq!(move_id(&sagat), 0x24);
    assert_eq!(
        sagat.machine().era_fighters()[0].sf6_drive_points(),
        Some(drive_before - 96)
    );

    let mut shot = base.clone();
    command(&mut shot, assets, QCF, &[LP, 1]);
    assert_eq!(move_id(&shot), 0x1a);
    let mut previous_hp = shot.machine().era_fighters()[1].sub_hp;
    let mut hits = 0;
    for _ in 0..80 {
        step(&mut shot, assets, PlayerInputs::default());
        let hp = shot.machine().era_fighters()[1].sub_hp;
        hits += usize::from(hp < previous_hp);
        previous_hp = hp;
    }
    assert_eq!(hits, 2, "OD Tiger Shot connects twice");

    for (motion, button, slot, name, remaining) in [
        (QCF2, LP, 0x29, "TIGER CANNON", 96),
        (QCB2, LK, 0x2a, "SAVAGE TIGER", 48),
        (QCF2, LK, 0x2b, "TIGER VANQUISHER", 0),
    ] {
        let mut sagat = base.clone();
        set_meter(&mut sagat, 144);
        command(&mut sagat, assets, motion, &[button]);
        assert_super(&sagat, slot, name, remaining);
    }
}

#[test]
fn eagle_st_andrews_green_requires_contact_before_riposte() {
    let Some(session) = bout(Eagle, Revision::St, Loadout::Standard) else {
        return;
    };
    let assets = session.assets();
    let base = session.fight().unwrap().clone();
    let hp = base.machine().era_fighters()[0].sub_hp;
    let mut whiff = base.clone();
    let motion = &[RIGHT, DOWN, DOWN | RIGHT];
    command(&mut whiff, assets, motion, &[LP]);
    assert_eq!(move_id(&whiff), 0x1f);
    for _ in 0..27 {
        step(&mut whiff, assets, PlayerInputs::default());
        assert_ne!(move_id(&whiff), 0x23);
        assert!(!whiff.fight_read().fighters[0].attack_active);
    }

    let mut probe = base.clone();
    let enemy_attack = |frame| match frame {
        0 => pad(DOWN, None),
        1 => pad(DOWN | LEFT, None),
        2 => pad(LEFT, Some(LP)),
        _ => PlayerInputs::default(),
    };
    let impact = (0..80)
        .find(|&frame| {
            probe.step(assets, PlayerInputs::default(), enemy_attack(frame), false);
            probe.machine().era_fighters()[0].sub_hp < hp
        })
        .expect("Ken's Hadoken reaches Eagle");
    assert!(impact >= 12);
    let eagle = (impact - 12..=impact + 12)
        .find_map(|trigger| {
            let mut eagle = base.clone();
            for frame in 0..=impact + 24 {
                let p1 = match frame {
                    frame if frame == trigger - 2 => pad(RIGHT, None),
                    frame if frame == trigger - 1 => pad(DOWN, None),
                    frame if frame == trigger => pad(DOWN | RIGHT, Some(LP)),
                    _ => PlayerInputs::default(),
                };
                eagle.step(assets, p1, enemy_attack(frame), false);
                if move_id(&eagle) == 0x23 {
                    return Some(eagle);
                }
            }
            None
        })
        .expect("the counter intercepts the Hadoken during startup");
    assert_eq!(eagle.machine().era_fighters()[0].sub_hp, hp);
    assert!(eagle
        .callouts()
        .frame()
        .side(Side::P1)
        .words
        .contains(&Some(CalloutKind::Move { word: "PARRY" })));
}

#[test]
fn tiger_nexus_selects_three_finishes_after_contact() {
    let Some(session) = bout(Sagat, Revision::Sf6, Loadout::Standard) else {
        return;
    };
    let assets = session.fight_assets().unwrap();
    let base = session.fight().unwrap();
    let mut close = base.clone();
    close_range(&mut close, assets);
    let mut finish_poses = std::collections::BTreeSet::new();
    for (kick, expected) in [(LK, 0x21), (MK, 0x22), (HK, 0x23)] {
        let mut sagat = close.clone();
        let hp = sagat.machine().era_fighters()[1].sub_hp;
        command(&mut sagat, assets, QCB, &[LK, MK]);
        assert_eq!(move_id(&sagat), 0x24);
        assert!(
            (0..44).any(|_| {
                step(&mut sagat, assets, pad(RIGHT, Some(kick)));
                move_id(&sagat) == expected
            }),
            "Tiger Nexus selects kick {kick}"
        );
        assert!(sagat.machine().era_fighters()[1].sub_hp < hp);

        let strike = (0..16).find_map(|_| {
            step(&mut sagat, assets, PlayerInputs::default());
            let fighter = sagat.fight_read().fighter(Side::P1);
            if fighter.move_id != expected {
                return None;
            }
            let machine = sagat.machine();
            let offset = usize::from(machine.diagnostic_telemetry().p1.entity_offset);
            let grid = &machine.state.objectram[offset..];
            let code = u16::from_be_bytes([grid[0], grid[1]]);
            if code == 0x2c00 {
                return None;
            }
            let top = i32::from(u16::from_be_bytes([grid[4], grid[5]]));
            let cell = if grid[2] & 4 == 0 { 16 } else { 32 };
            assert!(top + (i32::from(grid[0xc]) + 1) * cell <= sf_engine::data_bank::GROUND);
            Some(code)
        }).expect("the follow-up reaches its active pose");
        assert_ne!(strike, 0x2d0a, "a follow-up kick has its own strike pose");
        assert!(finish_poses.insert(strike), "the follow-up kicks use distinct poses");
    }

    let mut whiff = base.clone();
    command(&mut whiff, assets, QCB, &[LK]);
    assert_eq!(move_id(&whiff), 0x20);
    for _ in 0..44 {
        step(&mut whiff, assets, pad(RIGHT, Some(HK)));
        assert_ne!(move_id(&whiff), 0x23, "a whiff cannot branch");
    }
}

#[test]
fn training_lists_shows_and_coaches_a_native_what_if_move_for_each_fighter() {
    for (fighter, revision, loadout, name, shown_slot) in [
        (Gen, Revision::Sf3ThreeS, Loadout::Art3, "GENEI-JIN", 0x2b),
        (Birdie, Revision::Usf4, Loadout::Ultra1, "THE BIRDIE", 0x2a),
        (
            Eagle,
            Revision::Sf3ThreeS,
            Loadout::Art3,
            "ALBION TEMPEST",
            0x2b,
        ),
        (
            Eagle,
            Revision::St,
            Loadout::Standard,
            "ST ANDREWS GREEN",
            0x23,
        ),
        (
            Adon,
            Revision::Sf5Ce,
            Loadout::Skill1Trigger1,
            "JAGUAR VARIED ASSAULT",
            0x29,
        ),
        (
            Sagat,
            Revision::Sf6,
            Loadout::Standard,
            "TIGER CANNON",
            0x29,
        ),
    ] {
        let Some(mut session) = training_fight_with_fighter(fighter, revision, loadout) else {
            return;
        };
        assert!(
            meter(session.fight().unwrap()) > 0,
            "Fill Meters for {fighter:?}"
        );
        assert_training_move_demo(&mut session, name, shown_slot, name == "ST ANDREWS GREEN");
    }
}
