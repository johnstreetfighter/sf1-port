#[allow(dead_code)]
#[path = "support/alpha.rs"]
mod alpha;

use sf_engine::era::{EraFamily, GaugeMode, Loadout, Revision};
use sf_engine::front::{fight::Fight, PlayerInputs};
use sf_engine::roster::TournamentFighter::{self, Geki, Joe, Lee, Mike, Retsu};
use sf_video::romset::Assets;

use alpha::{
    assert_training_move_demo, assert_training_show, local_fight, local_select_with_fighter, pad,
    training_fight_with_fighter, DOWN, LEFT, LP, RIGHT, UP,
};
const MP: usize = 1;
const MK: usize = 4;
const STOCK: u8 = 48;

fn step(fight: &mut Fight, assets: &Assets, direction: u8, buttons: &[usize]) {
    let mut input: PlayerInputs = pad(direction, None);
    for &button in buttons {
        input.buttons[button] = true;
    }
    fight.step(assets, input, pad(0, None), false);
}

fn motion(fight: &mut Fight, assets: &Assets, directions: &[u8], buttons: &[usize]) {
    let (&last, earlier) = directions.split_last().expect("a direction motion");
    for &direction in earlier {
        step(fight, assets, direction, &[]);
    }
    step(fight, assets, last, buttons);
}

fn set_super(fight: &mut Fight, amount: u8) {
    let mut snapshot = fight.save_netplay_snapshot(0);
    snapshot.machine.toggle_state.era.fighters[0].super_meter = amount;
    fight.restore_netplay_snapshot(&snapshot);
}

#[test]
fn five_what_if_kits_execute_in_the_native_fight_loop() {
    for (fighter, revision, loadout, directions, buttons, slot, meter) in [
        (
            Retsu,
            Revision::St,
            Loadout::Standard,
            &[DOWN, DOWN | RIGHT, RIGHT, DOWN, DOWN | RIGHT, RIGHT][..],
            &[LP][..],
            0x29,
            STOCK,
        ),
        (
            Geki,
            Revision::Sf3ThreeS,
            Loadout::Art1,
            &[DOWN, DOWN | RIGHT, RIGHT],
            &[LP, MP],
            0x24,
            40,
        ),
        (
            Joe,
            Revision::Sfz3,
            Loadout::AIsm,
            &[DOWN, DOWN | RIGHT, RIGHT, DOWN, DOWN | RIGHT, RIGHT],
            &[LP],
            0x29,
            STOCK,
        ),
        (
            Mike,
            Revision::Usf4,
            Loadout::Ultra1,
            &[0],
            &[MP, MK],
            0x2e,
            0,
        ),
        (
            Lee,
            Revision::Sf6,
            Loadout::Standard,
            &[DOWN, DOWN | LEFT, LEFT, DOWN, DOWN | LEFT, LEFT],
            &[LP],
            0x2b,
            3 * STOCK,
        ),
    ] {
        let Some(select) =
            local_select_with_fighter(GaugeMode::EraDefault, fighter, revision, loadout)
        else {
            return;
        };
        let assets = select.assets();
        let mut fight = local_fight(&select, Revision::Sf1Plus, Loadout::Standard)
            .fight()
            .expect("a local fight")
            .clone();
        let state = fight.machine().era_fighters()[0];
        assert_eq!(state.style.revision, revision, "{fighter:?}");
        assert!(state.gauge().is_some(), "{fighter:?}");
        set_super(&mut fight, meter);
        motion(&mut fight, assets, directions, buttons);
        assert_eq!(fight.fight_read().fighters[0].move_id, slot, "{fighter:?}");
        assert_eq!(
            fight.machine().era_fighters()[0].super_meter,
            0,
            "{fighter:?}"
        );
        if fighter == Geki {
            let mut most_stars = 0;
            for _ in 0..20 {
                step(&mut fight, assets, 0, &[]);
                let stars = fight
                    .machine()
                    .toggle_state()
                    .era
                    .projectiles()
                    .iter()
                    .filter(|shot| shot.move_id == 0x24 && shot.hits_remaining != 0);
                most_stars = most_stars.max(stars.clone().count());
                assert!(stars.into_iter().all(|shot| shot.hits_remaining == 1));
            }
            assert_eq!(most_stars, 3, "EX Shuriken Fan sends three separate stars");
        }
    }
}

#[test]
fn geki_air_utsusemi_lands_and_returns_to_stance() {
    let Some(select) =
        local_select_with_fighter(GaugeMode::EraDefault, Geki, Revision::St, Loadout::Standard)
    else {
        return;
    };
    let assets = select.assets();
    let mut fight = local_fight(&select, Revision::Sf1Plus, Loadout::Standard)
        .fight()
        .expect("a local fight")
        .clone();
    for _ in 0..12 {
        step(&mut fight, assets, UP, &[]);
    }
    for direction in [DOWN, DOWN | LEFT] {
        for _ in 0..3 {
            step(&mut fight, assets, direction, &[]);
        }
    }
    step(&mut fight, assets, LEFT, &[]);
    step(&mut fight, assets, LEFT, &[LP]);
    assert_eq!(fight.fight_read().fighters[0].move_id, 0x21);

    let mut landed = false;
    let recovered = (0..100).any(|_| {
        step(&mut fight, assets, 0, &[]);
        let geki = fight.fight_read().fighters[0];
        landed |= geki.move_id == 0x21 && geki.state & 7 == 4;
        geki.move_id == 1
    });
    assert!(landed, "Geki reaches the Air Utsusemi landing pose");
    assert!(recovered, "Geki returns to stance after Air Utsusemi");
}

#[test]
fn training_show_advances_after_geki_air_utsusemi_recovers() {
    let Some(mut session) = training_fight_with_fighter(Geki, Revision::Sfz3, Loadout::AIsm) else {
        return;
    };
    assert_training_show(&mut session, "AIR UTSUSEMI", 0x21);
}

#[test]
fn bank_retains_distinct_headers_for_each_what_if_family() {
    for fighter in [Retsu, Geki, Joe, Mike, Lee] {
        let Some(select) = local_select_with_fighter(
            GaugeMode::EraDefault,
            fighter,
            Revision::St,
            Loadout::Standard,
        ) else {
            return;
        };
        let bank = select.data_bank().expect("fighter table");
        let mut headers = Vec::new();
        for family in [
            EraFamily::Sf2,
            EraFamily::SfZ,
            EraFamily::Sf3,
            EraFamily::Sf4,
            EraFamily::Sf5,
            EraFamily::Sf6,
        ] {
            if (fighter, family) == (TournamentFighter::Retsu, EraFamily::Sf6) {
                continue;
            }
            for slot in [0x19, 0x1a, 0x1b, 0x1f, 0x29, 0x2a, 0x2b] {
                let header = bank
                    .era_move_header(fighter, family, slot)
                    .unwrap_or_else(|| panic!("{fighter:?} {family:?} missing slot {slot:02x}"));
                assert!(
                    !headers.contains(&header),
                    "{fighter:?} {family:?} slot {slot:02x}"
                );
                headers.push(header);
            }
        }
    }
}

#[test]
fn training_lists_shows_and_coaches_a_native_what_if_move_for_each_fighter() {
    for (fighter, revision, loadout, name, slot) in [
        (Retsu, Revision::St, Loadout::Standard, "SHIN KIAIHOU", 0x29),
        (
            Geki,
            Revision::Sf3ThreeS,
            Loadout::Art1,
            "EX SHURIKEN FAN",
            0x24,
        ),
        (Joe, Revision::Sfz3, Loadout::AIsm, "SUPER SOBAT RUSH", 0x29),
        (
            Mike,
            Revision::Usf4,
            Loadout::Ultra1,
            "LETHAL KNOCKOUT",
            0x2a,
        ),
        (Lee, Revision::Sf6, Loadout::Standard, "ONE MAN ARMY", 0x2b),
    ] {
        let Some(mut session) = training_fight_with_fighter(fighter, revision, loadout) else {
            return;
        };
        assert!(session.fight_machine().unwrap().era_fighters()[0].super_meter > 0);
        assert_training_move_demo(&mut session, name, slot, false);
    }
}
