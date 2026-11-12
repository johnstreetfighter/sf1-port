//! Each side's rules are applied in mixed-era native bouts.

#[path = "support/alpha.rs"]
mod alpha;
use alpha::{local_fight, local_select_with_p1, pad};
use sf_engine::era::{GaugeMode, Loadout, Revision};
use sf_engine::front::PlayerInputs;

fn chord(punch: usize, kick: usize) -> PlayerInputs {
    let mut input = PlayerInputs::default();
    input.buttons[punch] = true;
    input.buttons[kick] = true;
    input
}

#[test]
fn mixed_sfz3_and_sf6_use_their_own_rows_or_one_unified_row() {
    for mode in [
        GaugeMode::EraDefault,
        GaugeMode::NoGauges,
        GaugeMode::Unified,
    ] {
        let Some(session) = local_select_with_p1(mode, Revision::Sfz3, Loadout::VIsm) else {
            return;
        };
        let native = local_fight(&session, Revision::Sf6, Loadout::Standard);
        let mut fight = native.fight().expect("local fight").clone();
        let states = fight.machine().era_fighters();
        if mode == GaugeMode::NoGauges {
            assert!(states.iter().all(|state| state.gauge().is_none()));
        } else {
            let secondary = mode == GaugeMode::EraDefault;
            assert!(states.iter().all(|state| state
                .gauge()
                .is_some_and(|gauge| gauge.second.is_some() == secondary)));
        }
        let mut snapshot = fight.save_netplay_snapshot(0);
        snapshot.machine.toggle_state.era.fighters[0].super_meter =
            if mode == GaugeMode::Unified { 144 } else { 96 };
        snapshot.machine.toggle_state.era.fighters[1].super_meter = 144;
        fight.restore_netplay_snapshot(&snapshot);
        fight.step(native.assets(), chord(0, 3), PlayerInputs::default(), false);
        assert_eq!(
            fight.machine().era_fighters()[0].custom_combo_active(),
            mode != GaugeMode::NoGauges
        );
        if mode == GaugeMode::Unified {
            assert_eq!(fight.machine().era_fighters()[0].super_meter, 48);
        }
        for _ in 0..18 {
            fight.step(
                native.assets(),
                PlayerInputs::default(),
                PlayerInputs::default(),
                false,
            );
        }
        fight.step(native.assets(), PlayerInputs::default(), chord(2, 5), false);
        assert_eq!(
            fight.fight_read().fighters[1].move_id == 0x2f,
            mode != GaugeMode::NoGauges
        );
        if mode == GaugeMode::Unified {
            assert_eq!(fight.machine().era_fighters()[1].super_meter, 96);
        } else if mode == GaugeMode::EraDefault {
            assert_eq!(
                fight.machine().era_fighters()[1].sf6_drive_points(),
                Some(240)
            );
        }
    }
}

#[test]
fn classic_style_uses_release_and_later_style_uses_press_in_the_same_match() {
    let Some(session) = local_select_with_p1(
        GaugeMode::EraDefault,
        Revision::Arcade1987,
        Loadout::Standard,
    ) else {
        return;
    };
    let native = local_fight(&session, Revision::St, Loadout::Standard);
    let mut fight = native.fight().expect("local fight").clone();
    let jab = pad(0, Some(0));
    let idle = PlayerInputs::default();
    fight.step(native.assets(), jab, jab, false);
    assert!(!fight.fight_read().fighters[0].attacking());
    assert!(fight.fight_read().fighters[1].attacking());
    for _ in 0..6 {
        fight.step(native.assets(), jab, idle, false);
    }
    fight.step(native.assets(), idle, idle, false);
    assert!(fight.fight_read().fighters[0].attacking());
}

#[test]
fn late_drive_impact_replays_with_the_same_mixed_style_state() {
    use sf_engine::netplay::{encode_inputs, RollbackEngine};

    let Some(session) = local_select_with_p1(GaugeMode::EraDefault, Revision::Sfz3, Loadout::VIsm)
    else {
        return;
    };
    let native = local_fight(&session, Revision::Sf6, Loadout::Standard);
    let mut expected = native.fight().expect("local fight").clone();
    let mut replayed = expected.clone();
    let mut rollback = RollbackEngine::new(0, 0, 8);
    let assets = native.assets();
    let idle = PlayerInputs::default();
    for tick in 0u32..48 {
        if let Some(frame) = tick.checked_sub(4) {
            let remote = if frame == 5 { chord(2, 5) } else { idle };
            rollback.apply_remote_inputs(&mut replayed, assets, frame, &[encode_inputs(remote)]);
        }
        if tick < 44 {
            let remote = if tick == 5 { chord(2, 5) } else { idle };
            expected.step(assets, idle, remote, false);
            rollback
                .step(&mut replayed, assets, idle)
                .expect("within rollback limit");
        }
    }
    assert_eq!(rollback.confirmed_inputs().len(), 44);
    assert_eq!(expected.state_checksum(), replayed.state_checksum());
    assert_eq!(
        expected.machine().era_fighters(),
        replayed.machine().era_fighters()
    );
    assert_eq!(
        replayed.machine().era_fighters()[1].sf6_drive_points(),
        Some(240)
    );
}

#[test]
fn every_fighter_can_start_a_real_bout_in_all_eight_era_families() {
    use alpha::local_select_with_fighter;
    use sf_engine::roster::TournamentFighter;

    let styles = [
        (Revision::Arcade1987, Loadout::Standard),
        (Revision::Sf1Plus, Loadout::Standard),
        (Revision::St, Loadout::Standard),
        (Revision::Sfz3, Loadout::AIsm),
        (Revision::Sf3ThreeS, Loadout::Art1),
        (Revision::Usf4, Loadout::Ultra1),
        (Revision::Sf5Ce, Loadout::Skill1Trigger1),
        (Revision::Sf6, Loadout::Standard),
    ];
    for fighter in TournamentFighter::ALL {
        for (index, (revision, loadout)) in styles.into_iter().enumerate() {
            let Some(session) =
                local_select_with_fighter(GaugeMode::EraDefault, fighter, revision, loadout)
            else {
                return;
            };
            let native = local_fight(&session, Revision::Sf1Plus, Loadout::Standard);
            let fight = native.fight().expect("local fight");
            let state = fight.machine().era_fighters()[0];
            assert_eq!(state.fighter, Some(fighter), "{revision:?}");
            assert_eq!(
                (state.style.revision, state.style.loadout),
                (revision, loadout),
                "{fighter:?}"
            );
            assert_eq!(
                state.gauge().is_some(),
                index >= 2,
                "{fighter:?} {revision:?}"
            );
            assert!(
                fight.fight_read().fighters[0].present,
                "{fighter:?} {revision:?}"
            );
        }
    }
}
