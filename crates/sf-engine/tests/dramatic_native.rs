//! Native SF1+ Dramatic roles and collision through the front-end Fight.

use std::sync::Arc;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::data_bank::DataBank;
use sf_engine::dramatic::{DramaticRules, LifebarMode, ALLY_SUB_HP, BOSS_SUB_HP};
use sf_engine::era::Revision;
use sf_engine::fighter::{CPU_STRUCT, DRAMATIC_ALLY2, P1_STRUCT, P2_STRUCT};
use sf_engine::front::cabinet::{flow_position, Players};
use sf_engine::front::fight::{Fight, FightDips, FightKind, FightSetup, FightSnapshot};
use sf_engine::front::PlayerInputs;
use sf_engine::roster::{PlayMode, TournamentFighter};
use sf_video::romset::{load_assets, Assets};

const ENTITIES: [usize; 3] = [0x400, 0xa00, DRAMATIC_ALLY2 as usize];

fn ready_fight(lifebar: LifebarMode, friendly_fire: bool, co_op: bool) -> (Assets, Fight) {
    let assets = load_assets(
        "../../tools/reference/local/roms/sf",
        "../../tools/reference/rom-manifest.json",
    )
    .expect("Dramatic integration tests require the verified local ROM set");
    let data = Arc::new(DataBank::build(&assets.maincpu, &[]));
    let mut config = MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul);
    config.roster.mode = PlayMode::Versus;
    config.roster.p1 = TournamentFighter::Ryu;
    config.roster.p2 = TournamentFighter::Ken;
    config.dramatic = Some(DramaticRules {
        co_op,
        ally2: TournamentFighter::Ryu,
        stage: TournamentFighter::Retsu,
        lifebar,
        friendly_fire,
        styles: [Revision::Sf1Plus; 3],
        rivalry: None,
    });
    let setup = FightSetup {
        kind: FightKind::Dramatic { co_op },
        players: Players::One,
        config,
        dsw1: FightDips::DSW1,
        dsw2: FightDips::default().dsw2(),
    };
    let mut fight = Fight::start_banked(&assets.maincpu, setup, Some(0), None, Some(data))
        .expect("Dramatic starts the native cabinet driver");
    for _ in 0..1_500 {
        if flow_position(fight.machine()) == 0x3eafa && !fight.driving() {
            return (assets, fight);
        }
        fight.step(
            &assets,
            PlayerInputs::default(),
            PlayerInputs::default(),
            false,
        );
    }
    panic!("Dramatic never entered the active native fight loop");
}

fn health(fight: &Fight) -> [u16; 3] {
    fight.machine().toggle_state().dramatic.health
}

fn active_punch(snapshot: &mut FightSnapshot, attacker: usize) {
    let objects = &mut snapshot.machine.state.objectram;
    // A native light punch on its active frame. Extend the attack box so
    // both defenders are in range regardless of the opening placement.
    objects[attacker + 9] = 0xc2;
    objects[attacker + 0xa] = 0x0e;
    objects[attacker + 0xf] = 0x80;
    objects[attacker + 0x22..attacker + 0x26].copy_from_slice(&[0, 0xff, 0, 0xff]);
    objects[attacker + 0x28..attacker + 0x2c].copy_from_slice(&0x45af0u32.to_be_bytes());
    objects[attacker + 0x33] = 2;
    for entity in ENTITIES {
        objects[entity + 0xf] &= !0x08;
    }
}

#[test]
fn one_native_boss_projectile_can_hit_both_allies_on_the_same_frame() {
    let (assets, mut fight) = ready_fight(LifebarMode::Independent, false, true);
    let mut snapshot = fight.save_netplay_snapshot(0);
    let objects = &mut snapshot.machine.state.objectram;
    let shot = &mut objects[0x100..0x128];
    shot[..0x20].copy_from_slice(&assets.maincpu[0x186dc..0x186fc]);
    shot[4..8].copy_from_slice(&[0, 120, 0, 200]);
    shot[8] = 1;
    shot[0xc..0xe].fill(0xff);
    shot[0x20..0x24].copy_from_slice(&CPU_STRUCT.to_be_bytes());
    shot[0x24..0x28].copy_from_slice(&P1_STRUCT.to_be_bytes());
    for entity in ENTITIES {
        objects[entity + 0xf] &= !0x08;
    }
    fight.restore_netplay_snapshot(&snapshot);
    fight.step(
        &assets,
        PlayerInputs::default(),
        PlayerInputs::default(),
        false,
    );
    let [ally1, boss, ally2] = health(&fight);
    assert!(ally1 < ALLY_SUB_HP);
    assert_eq!(boss, BOSS_SUB_HP);
    assert_eq!(ally1, ally2);
    for _ in 0..4 {
        fight.step(
            &assets,
            PlayerInputs::default(),
            PlayerInputs::default(),
            false,
        );
    }
    assert_eq!(health(&fight), [ally1, boss, ally2]);
}

#[test]
fn three_native_slots_and_the_second_pad_remain_independent() {
    let (assets, mut fight) = ready_fight(LifebarMode::Independent, false, true);
    assert_eq!(health(&fight), [ALLY_SUB_HP, BOSS_SUB_HP, ALLY_SUB_HP]);
    let work = fight.machine().work_ram();
    let object = &fight.machine().state.objectram;
    for (ps, entity) in [P1_STRUCT, CPU_STRUCT, P2_STRUCT].into_iter().zip(ENTITIES) {
        let offset = (ps - 0xff8000) as usize + 0x10;
        assert_eq!(
            u16::from_be_bytes(work[offset..offset + 2].try_into().unwrap()),
            entity as u16
        );
        assert_eq!(
            u32::from_be_bytes(object[entity + 0x38..entity + 0x3c].try_into().unwrap()),
            ps
        );
        assert_ne!(object[entity + 8], 0);
    }

    let opening = fight.save_netplay_snapshot(0);
    let mut jump = PlayerInputs::default();
    jump.directions[3] = true;
    let mut saw_ally2_jump = false;
    for _ in 0..16 {
        fight.step(&assets, PlayerInputs::default(), jump, false);
        let object = &fight.machine().state.objectram;
        assert!(!(9..=0x0b).contains(&object[ENTITIES[0] + 0xa]));
        saw_ally2_jump |= (9..=0x0b).contains(&object[ENTITIES[2] + 0xa]);
    }
    assert!(
        saw_ally2_jump,
        "the P2 up input should start Ally 2's native jump"
    );
    let telemetry = fight.machine().diagnostic_telemetry();
    assert!(telemetry.show_ally2_inputs && telemetry.p1.active && telemetry.p2.active);
    assert!(telemetry.ally2.is_some_and(|ally| ally.active));

    fight.restore_netplay_snapshot(&opening);
    let mut punch = PlayerInputs::default();
    punch.buttons[0] = true;
    let mut saw_ally2_punch = false;
    for _ in 0..8 {
        fight.step(&assets, PlayerInputs::default(), punch, false);
        let object = &fight.machine().state.objectram;
        assert!(!(0x0d..=0x10).contains(&object[ENTITIES[0] + 0xa]));
        saw_ally2_punch |= (0x0d..=0x10).contains(&object[ENTITIES[2] + 0xa]);
    }
    assert!(
        saw_ally2_punch,
        "the P2 punch button should start Ally 2's native punch"
    );
}

#[test]
fn native_contacts_filter_friendly_fire_and_scale_boss_damage() {
    let (assets, mut fight) = ready_fight(LifebarMode::Independent, false, true);
    let ready = fight.save_netplay_snapshot(0);
    let mut ally_attack = ready.clone();
    active_punch(&mut ally_attack, ENTITIES[2]);
    fight.restore_netplay_snapshot(&ally_attack);
    fight.step(
        &assets,
        PlayerInputs::default(),
        PlayerInputs::default(),
        false,
    );
    let [ally1, boss, ally2] = health(&fight);
    assert_eq!([ally1, ally2], [ALLY_SUB_HP; 2]);
    let damage = BOSS_SUB_HP - boss;
    assert!(damage > 0 && damage % 20 == 0);

    ally_attack
        .setup
        .config
        .dramatic
        .as_mut()
        .unwrap()
        .friendly_fire = true;
    ally_attack
        .machine
        .match_config
        .dramatic
        .as_mut()
        .unwrap()
        .friendly_fire = true;
    fight.restore_netplay_snapshot(&ally_attack);
    fight.step(
        &assets,
        PlayerInputs::default(),
        PlayerInputs::default(),
        false,
    );
    assert_eq!(health(&fight), [ALLY_SUB_HP - damage, boss, ALLY_SUB_HP]);

    let mut boss_attack = ready;
    active_punch(&mut boss_attack, ENTITIES[1]);
    fight.restore_netplay_snapshot(&boss_attack);
    fight.step(
        &assets,
        PlayerInputs::default(),
        PlayerInputs::default(),
        false,
    );
    let boosted = damage / 20 * 25;
    assert_eq!(
        health(&fight),
        [ALLY_SUB_HP - boosted, BOSS_SUB_HP, ALLY_SUB_HP - boosted]
    );
}

#[test]
fn native_ko_uses_the_lifebar_mode_and_preserves_the_surviving_ally() {
    for mode in [LifebarMode::Independent, LifebarMode::Shared] {
        let (assets, mut fight) = ready_fight(mode, false, true);
        let mut snapshot = fight.save_netplay_snapshot(0);
        active_punch(&mut snapshot, ENTITIES[1]);
        let hp = &mut snapshot.machine.toggle_state.dramatic.health;
        hp[0] = 1;
        if mode == LifebarMode::Shared {
            hp[2] = 1;
        }
        fight.restore_netplay_snapshot(&snapshot);
        fight.step(
            &assets,
            PlayerInputs::default(),
            PlayerInputs::default(),
            false,
        );
        assert_eq!(health(&fight)[0], 0);
        assert_eq!(fight.machine().state.objectram[ENTITIES[0] + 8], 8);
        if mode == LifebarMode::Shared {
            assert_eq!(health(&fight)[2], 0);
            assert_eq!(fight.machine().state.objectram[ENTITIES[2] + 8], 8);
        } else {
            assert!(health(&fight)[2] > 0);
            assert_eq!(
                fight.machine().work_ram()[(P1_STRUCT - 0xff8000) as usize + 6],
                1
            );
            for _ in 0..6 {
                fight.step(
                    &assets,
                    PlayerInputs::default(),
                    PlayerInputs::default(),
                    false,
                );
            }
            assert_eq!(flow_position(fight.machine()), 0x3eafa);
        }
    }
}

#[test]
fn independent_time_over_compares_the_healthiest_ally_to_the_boss() {
    let (assets, mut fight) = ready_fight(LifebarMode::Independent, false, true);
    let mut snapshot = fight.save_netplay_snapshot(0);
    snapshot.machine.toggle_state.dramatic.health = [400, 900, 650];
    let time = (sf_engine::tasks::TIME_DIGITS - 0xff8000) as usize;
    snapshot.machine.work[time..time + 3].fill(0);
    fight.restore_netplay_snapshot(&snapshot);

    let winner = (0xff8e38 - 0xff8000) as usize;
    for _ in 0..200 {
        fight.step(
            &assets,
            PlayerInputs::default(),
            PlayerInputs::default(),
            false,
        );
        let work = fight.machine().work_ram();
        let ps = u32::from_be_bytes(work[winner..winner + 4].try_into().unwrap());
        if ps != 0 {
            // Ally 2 has 67.7% remaining versus the boss's 58.6%. Averaging
            // both allies would reverse the result of this time over.
            assert_eq!(ps, P1_STRUCT);
            return;
        }
    }
    panic!("the native time-over path never recorded a round winner");
}

#[test]
fn solo_dramatic_ally_uses_its_cpu_controller() {
    let (assets, mut fight) = ready_fight(LifebarMode::Shared, false, false);
    let mut with_p2_input = fight.clone();
    let neutral = PlayerInputs::default();
    let held = PlayerInputs {
        directions: [false, true, false, true],
        buttons: [true; 6],
    };
    let mut ally_pressed = false;
    for frame in 1..=90 {
        fight.step(&assets, neutral, neutral, false);
        with_p2_input.step(&assets, neutral, held, false);
        let controllers = fight
            .save_netplay_snapshot(frame)
            .machine
            .dramatic_cpu
            .unwrap();
        ally_pressed |= controllers[1].pad() != 0;
        assert_eq!(fight.state_checksum(), with_p2_input.state_checksum());
    }
    assert!(
        ally_pressed,
        "the independent Ally 2 CPU drives the native pad"
    );
}
