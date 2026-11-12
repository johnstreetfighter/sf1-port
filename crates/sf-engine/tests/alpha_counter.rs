//! Native Alpha Counters in ROM-backed fights.

#[path = "support/alpha.rs"]
mod alpha;

use alpha::{local_fight, local_select, pad, DOWN, LP, MANIFEST, RIGHT, ROMS};
use sf_engine::data_bank::ALPHA_COUNTER_SLOT;
use sf_engine::era::{GaugeMode, Loadout, Revision};
use sf_engine::front::fight::{Fight, FightFrame};
use sf_engine::front::PlayerInputs;
use sf_engine::netplay::{encode_inputs, RollbackEngine};
use sf_video::announcer::AnnouncerLine;
use sf_video::callouts::{CalloutKind, Side};
use sf_video::romset::{load_assets, Assets};

const HK: usize = 5;
const STOCK: u8 = 48;

fn start_fight(
    gauge_mode: GaugeMode,
    revision: Revision,
    loadout: Loadout,
) -> Option<(Assets, Fight)> {
    let select = local_select(gauge_mode)?;
    let session = local_fight(&select, revision, loadout);
    let assets = load_assets(ROMS, MANIFEST).expect("local ROMs assemble");
    let mut fight = session.fight().expect("local fight").clone();
    let idle = PlayerInputs::default();
    for _ in 0..100 {
        let fighters = fight.machine().diagnostic_telemetry();
        if fighters.p2.center_x - fighters.p1.center_x <= 38 {
            for _ in 0..8 {
                fight.step(&assets, idle, idle, false);
            }
            return Some((assets, fight));
        }
        fight.step(&assets, pad(RIGHT, None), idle, false);
    }
    panic!(
        "Ryu could not approach Ken: {:?}",
        fight.machine().diagnostic_telemetry()
    );
}

fn block_hard_kick(fight: &mut Fight, assets: &Assets) {
    let idle = PlayerInputs::default();
    let back = pad(RIGHT, None);
    fight.step(assets, pad(0, Some(HK)), back, false);
    for _ in 0..30 {
        if fight.machine().toggle_state().combo.blockstun[1] > 0 {
            assert_eq!(fight.fight_read().fighters[1].move_id, 4, "standing guard");
            return;
        }
        fight.step(assets, idle, back, false);
    }
    panic!(
        "Ken never blocked Ryu's hard kick: {:?}",
        fight.fight_read()
    );
}

fn set_meter(fight: &mut Fight, meter: u8) {
    let mut snapshot = fight.save_netplay_snapshot(0);
    snapshot.machine.toggle_state.era.fighters[1].super_meter = meter;
    fight.restore_netplay_snapshot(&snapshot);
}

fn counter_inputs(button: usize) -> [PlayerInputs; 3] {
    [
        pad(RIGHT, None),
        pad(RIGHT | DOWN, None),
        pad(DOWN, Some(button)),
    ]
}

fn attempt_counter(fight: &mut Fight, assets: &Assets, button: usize) -> FightFrame {
    let [back, down_back, activation] = counter_inputs(button);
    let idle = PlayerInputs::default();
    fight.step(assets, idle, back, false);
    fight.step(assets, idle, down_back, false);
    fight.step(assets, idle, activation, false)
}

fn counter_announcements(frame: &FightFrame) -> usize {
    frame
        .announcements
        .iter()
        .filter(|&&line| line == AnnouncerLine::AlphaCounter)
        .count()
}

#[test]
fn alpha_counter_spends_a_stock_or_half_custom_gauge_and_blows_back_the_attacker() {
    for (revision, loadout, button) in [
        (Revision::Sfz1, Loadout::Standard, LP),
        (Revision::Sfz2, Loadout::Standard, HK),
        (Revision::Sfz2A, Loadout::Standard, LP),
        (Revision::Sfz3, Loadout::AIsm, LP),
        (Revision::Sfz3, Loadout::VIsm, HK),
    ] {
        let Some((assets, mut fight)) = start_fight(GaugeMode::EraDefault, revision, loadout)
        else {
            return;
        };
        block_hard_kick(&mut fight, &assets);
        set_meter(&mut fight, 2 * STOCK);
        let initial_hp = fight.machine().era_fighters()[0].sub_hp;

        let activation = attempt_counter(&mut fight, &assets, button);
        assert_eq!(
            fight.fight_read().fighters[1].move_id,
            ALPHA_COUNTER_SLOT,
            "{revision:?} {loadout:?}"
        );
        assert_eq!(fight.machine().toggle_state().combo.blockstun[1], 0);
        assert_eq!(fight.machine().era_fighters()[1].super_meter, STOCK);
        assert_eq!(counter_announcements(&activation), 1);
        assert!(fight.machine().diagnostic_telemetry().p2.invulnerable);
        let callouts = fight.callouts().frame();
        let defender = callouts.side(Side::P2);
        assert!(defender.words.contains(&Some(CalloutKind::AlphaCounter)));
        assert!(defender.flash.is_some(), "the defender's gauge flashes");

        let idle = PlayerInputs::default();
        let mut hit = false;
        let mut vulnerable_recovery = false;
        for _ in 0..30 {
            let output = fight.step(&assets, idle, idle, false);
            assert_eq!(counter_announcements(&output), 0, "announced once");
            let defender = fight.machine().diagnostic_telemetry().p2;
            vulnerable_recovery |= defender.move_id == ALPHA_COUNTER_SLOT && !defender.invulnerable;
            if !hit && fight.machine().era_fighters()[0].sub_hp < initial_hp {
                let attacker = fight.fight_read().fighters[0];
                assert!(attacker.hitstun());
                assert_eq!(attacker.state & 3, 1, "airborne blowback");
                hit = true;
            }
        }
        assert!(hit, "the native counter connected");
        assert!(vulnerable_recovery, "the counter has punishable recovery");
    }
}

#[test]
fn alpha_counter_requires_ground_blockstun_enough_meter_and_an_eligible_style() {
    for (mode, loadout, meter, guard) in [
        (GaugeMode::EraDefault, Loadout::AIsm, STOCK, false),
        (GaugeMode::EraDefault, Loadout::AIsm, STOCK - 1, true),
        (GaugeMode::EraDefault, Loadout::VIsm, STOCK - 1, true),
        (GaugeMode::EraDefault, Loadout::XIsm, STOCK, true),
        (GaugeMode::NoGauges, Loadout::AIsm, 0, true),
    ] {
        let Some((assets, mut fight)) = start_fight(mode, Revision::Sfz3, loadout) else {
            return;
        };
        if guard {
            block_hard_kick(&mut fight, &assets);
        }
        set_meter(&mut fight, meter);
        let activation = attempt_counter(&mut fight, &assets, LP);
        assert_ne!(fight.fight_read().fighters[1].move_id, ALPHA_COUNTER_SLOT);
        assert_eq!(fight.machine().era_fighters()[1].super_meter, meter);
        assert_eq!(counter_announcements(&activation), 0);
    }
}

#[test]
fn alpha_counter_cancels_a_projectile_guard_after_native_chip_damage() {
    let Some((assets, mut fight)) =
        start_fight(GaugeMode::EraDefault, Revision::Sfz3, Loadout::AIsm)
    else {
        return;
    };
    set_meter(&mut fight, STOCK);
    let health = fight.machine().era_fighters()[1].sub_hp;
    let idle = PlayerInputs::default();
    let back = pad(RIGHT, None);
    for frame in 0..90 {
        let attacker = match frame {
            0..3 => pad(DOWN, None),
            3..6 => pad(DOWN | RIGHT, None),
            6..9 => pad(RIGHT, Some(LP)),
            _ => idle,
        };
        fight.step(&assets, attacker, back, false);
        if fight.machine().toggle_state().combo.blockstun[1] != 0 {
            break;
        }
    }
    assert_eq!(fight.machine().era_fighters()[1].sub_hp, health - 20);
    assert!(fight.fight_read().fighters[1].blockstun);
    assert_eq!(fight.fight_read().fighters[1].move_id, 4);
    let meter = fight.machine().era_fighters()[1].super_meter;
    let activation = attempt_counter(&mut fight, &assets, LP);
    assert_eq!(fight.fight_read().fighters[1].move_id, ALPHA_COUNTER_SLOT);
    assert_eq!(fight.machine().era_fighters()[1].super_meter, meter - STOCK);
    assert_eq!(counter_announcements(&activation), 1);
}

#[test]
fn late_alpha_counter_input_replays_the_meter_hit_and_announcement_once() {
    let Some((assets, mut reference)) =
        start_fight(GaugeMode::EraDefault, Revision::Sfz3, Loadout::AIsm)
    else {
        return;
    };
    block_hard_kick(&mut reference, &assets);
    set_meter(&mut reference, STOCK);
    let mut predicted = reference.clone();
    let mut net = RollbackEngine::new(0, 0, 8);
    const DELAY: u32 = 4;
    const FRAMES: u32 = 40;
    let command = counter_inputs(LP);
    let idle = PlayerInputs::default();
    let remote = |frame: u32| command.get(frame as usize).copied().unwrap_or(idle);
    let mut expected_calls = 0;
    let mut actual_calls = 0;

    for tick in 0..FRAMES + DELAY {
        if let Some(frame) = tick.checked_sub(DELAY) {
            net.apply_remote_inputs(
                &mut predicted,
                &assets,
                frame,
                &[encode_inputs(remote(frame))],
            );
        }
        if tick < FRAMES {
            expected_calls +=
                counter_announcements(&reference.step(&assets, idle, remote(tick), false));
            let output = net
                .step(&mut predicted, &assets, idle)
                .expect("under the rollback limit");
            actual_calls += counter_announcements(&output);
            if tick == 2 {
                assert_eq!(reference.machine().era_fighters()[1].super_meter, 0);
                assert_eq!(predicted.machine().era_fighters()[1].super_meter, STOCK);
            }
        }
    }
    assert_eq!(net.confirmed_inputs().len(), FRAMES as usize);
    assert_eq!(predicted.state_checksum(), reference.state_checksum());
    assert_eq!(
        predicted.machine().era_fighters(),
        reference.machine().era_fighters()
    );
    assert_eq!((expected_calls, actual_calls), (1, 1));
}
