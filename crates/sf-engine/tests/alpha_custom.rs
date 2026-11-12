//! V-ISM on native fighters, with the actual player input and renderer.

#[path = "support/alpha.rs"]
mod alpha;
use alpha::{local_fight, local_select_with_p1, pad, MANIFEST, RIGHT, ROMS};
use sf_engine::era::{GaugeMode, Loadout, Revision};
use sf_engine::front::fight::Fight;
use sf_engine::front::PlayerInputs;
use sf_video::announcer::AnnouncerLine;
use sf_video::romset::{load_assets, Assets};

const STOCK_AT: u8 = 48;

fn fight(mode: GaugeMode, loadout: Loadout) -> Option<(Assets, Fight)> {
    let session = local_select_with_p1(mode, Revision::Sfz3, loadout)?;
    let mut fight = local_fight(&session, Revision::Sf1Plus, Loadout::Standard)
        .fight()
        .expect("local fight")
        .clone();
    let assets = load_assets(ROMS, MANIFEST).expect("ROMs assemble");
    let idle = PlayerInputs::default();
    for _ in 0..100 {
        let telemetry = fight.machine().diagnostic_telemetry();
        if telemetry.p1.center_x.abs_diff(telemetry.p2.center_x) <= 38 {
            for _ in 0..8 {
                fight.step(&assets, idle, idle, false);
            }
            return Some((assets, fight));
        }
        fight.step(&assets, pad(RIGHT, None), idle, false);
    }
    panic!("fighters did not reach close range");
}

fn set_meter(fight: &mut Fight, value: u8) {
    let mut snapshot = fight.save_netplay_snapshot(0);
    snapshot.machine.toggle_state.era.fighters[0].super_meter = value;
    fight.restore_netplay_snapshot(&snapshot);
}

fn pair(punch: usize, kick: usize) -> PlayerInputs {
    let mut inputs = PlayerInputs::default();
    inputs.buttons[punch] = true;
    inputs.buttons[kick] = true;
    inputs
}

#[test]
fn custom_combo_requires_its_ism_stock_and_matching_buttons() {
    let Some((assets, base)) = fight(GaugeMode::EraDefault, Loadout::VIsm) else {
        return;
    };
    let idle = PlayerInputs::default();
    for (meter, buttons, active) in [
        (STOCK_AT - 1, pair(0, 3), false),
        (STOCK_AT, pair(0, 4), false),
        (STOCK_AT, pair(0, 3), true),
        (2 * STOCK_AT, pair(2, 5), true),
    ] {
        let mut native = base.clone();
        set_meter(&mut native, meter);
        let frame = native.step(&assets, buttons, idle, false);
        assert_eq!(
            native.machine().era_fighters()[0].custom_combo_active(),
            active
        );
        assert_eq!(
            frame.announcements.contains(&AnnouncerLine::CustomCombo),
            active
        );
        if active {
            assert_eq!(
                native.fight_read().fighters[0].move_id,
                sf_engine::data_bank::ALPHA_COUNTER_SLOT
            );
        }
    }
    for (gauge, loadout) in [
        (GaugeMode::EraDefault, Loadout::AIsm),
        (GaugeMode::NoGauges, Loadout::VIsm),
    ] {
        let (assets, mut native) = fight(gauge, loadout).expect("local ROMs");
        set_meter(&mut native, STOCK_AT);
        native.step(&assets, pair(1, 4), idle, false);
        assert!(!native.machine().era_fighters()[0].custom_combo_active());
    }
}

#[test]
fn custom_combo_scales_contact_allows_chains_and_drains_to_zero() {
    let Some((assets, base)) = fight(GaugeMode::EraDefault, Loadout::VIsm) else {
        return;
    };
    let idle = PlayerInputs::default();
    let mut ordinary = base.clone();
    let hp = ordinary.machine().era_fighters()[1].sub_hp;
    ordinary.step(&assets, pad(0, Some(2)), idle, false);
    for _ in 0..22 {
        ordinary.step(&assets, idle, idle, false);
    }
    let ordinary_damage = hp - ordinary.machine().era_fighters()[1].sub_hp;
    assert!(ordinary_damage > 0);
    let mut custom = base;
    set_meter(&mut custom, STOCK_AT);
    custom.step(&assets, pair(0, 3), idle, false);
    for _ in 0..17 {
        custom.step(&assets, idle, idle, false);
    }
    assert!(custom.machine().era_fighters()[0].custom_combo_active());
    assert_ne!(
        custom.save_netplay_snapshot(0).trails[0],
        Default::default()
    );
    custom.step(&assets, pad(0, Some(2)), idle, false);
    let heavy = custom.fight_read().fighters[0].move_id;
    for _ in 0..20 {
        if custom.machine().era_fighters()[1].sub_hp < hp {
            break;
        }
        custom.step(&assets, idle, idle, false);
    }
    assert_eq!(
        hp - custom.machine().era_fighters()[1].sub_hp,
        ordinary_damage / 2
    );
    custom.step(&assets, pad(0, Some(3)), idle, false);
    assert_ne!(custom.fight_read().fighters[0].move_id, heavy);
    for _ in 0..110 {
        custom.step(&assets, idle, idle, false);
    }
    let state = custom.machine().era_fighters()[0];
    assert!(!state.custom_combo_active());
    assert_eq!(state.super_meter, 0);
}
