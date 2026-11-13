//! Real input and rollback checks for the Utility CPU.
mod support {
    #[allow(dead_code)]
    pub mod alpha;
}

use sf_engine::config::RulesetPreset;
use sf_engine::era::Revision;
use sf_engine::fair_cpu::FIGHT_LOOP;
use sf_engine::fighter::{P1_STRUCT, P2_STRUCT};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::fight::Fight;
use sf_engine::front::settings::Settings;
use sf_engine::front::{FrameInputs, GameSession, PlayerInputs, Transition};
use sf_engine::utility::UtilityAi;
use sf_video::romset::load_assets;
use std::path::Path;
use support::alpha::{pad, MANIFEST, RIGHT, ROMS, UP};

fn training_cpu(steps: usize) -> Option<GameSession> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    let assets = load_assets(ROMS, MANIFEST).expect("local ROMs");
    let mut config = sf_engine::config::MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul);
    config.difficulty = 4;
    let mut session = GameSession::new(assets, config).with_settings(Settings {
        fill_meters: true,
        ..Settings::default()
    });
    let input = |p1| FrameInputs {
        p1,
        ..FrameInputs::default()
    };
    let tap = |session: &mut GameSession, input| {
        let out = session.step(input);
        session.step(FrameInputs::default());
        out.transition
    };
    let confirm = input(pad(0, Some(0)));
    for _ in 0..200 {
        session.step(FrameInputs::default());
    }
    tap(
        &mut session,
        FrameInputs {
            menu: sf_engine::front::MenuKeys {
                start: true,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    tap(&mut session, confirm);
    tap(&mut session, input(pad(4, None)));
    tap(&mut session, confirm);
    tap(&mut session, confirm);
    tap(&mut session, confirm);
    tap(&mut session, confirm);
    assert_eq!(tap(&mut session, confirm), Some(Transition::StyleOpened));
    for _ in 0..steps {
        tap(&mut session, input(pad(RIGHT, None)));
    }
    tap(&mut session, confirm);
    for _ in 0..2000 {
        if session
            .fight_machine()
            .is_some_and(|machine| flow_position(machine) == FIGHT_LOOP)
        {
            return Some(session);
        }
        session.step(FrameInputs::default());
    }
    panic!("Training CPU did not reach the fight");
}

fn play(fight: &mut Fight, assets: &sf_video::romset::Assets, frame: usize) {
    let input = match frame % 80 {
        0..=19 => pad(RIGHT, None),
        20..=23 => pad(0, Some(0)),
        40..=44 => pad(RIGHT | UP, None),
        51..=55 => pad(0, Some(5)),
        _ => PlayerInputs::default(),
    };
    assert!(fight
        .step(assets, input, PlayerInputs::default(), false)
        .event
        .is_none());
}

#[test]
fn native_drive_commands_and_utility_inputs_replay_exactly_after_a_snapshot() {
    let Some(session) = training_cpu(6) else {
        return;
    };
    let assets = session.assets();
    let mut fight = session.fight().unwrap().clone();
    assert_eq!(
        fight.machine().era_fighters()[1].style.revision,
        Revision::Sf6
    );
    let mut had_drive_move = false;
    let mut had_airborne = false;
    let mut replayed_macro = false;
    let mut frame = 0;
    while frame < 1500 {
        play(&mut fight, assets, frame);
        let view = UtilityAi::observe(fight.machine(), P2_STRUCT, P1_STRUCT);
        assert!(view.distance <= 512);
        had_airborne |= view.airborne;
        let move_id = fight.fight_read().fighters[1].move_id;
        had_drive_move |= matches!(move_id, 0x2c | 0x2f);
        let remaining = fight.machine().toggle_state().utility[1].macro_remaining();
        assert!(remaining < 12);
        if remaining > 0 && !replayed_macro {
            let saved = fight.save_netplay_snapshot(frame as u32);
            let mut resumed = fight.clone();
            resumed.restore_netplay_snapshot(&saved);
            for next in frame + 1..frame + 61 {
                play(&mut fight, assets, next);
                play(&mut resumed, assets, next);
                assert_eq!(
                    fight.state_checksum(),
                    resumed.state_checksum(),
                    "resimulation frame {next}"
                );
                assert_eq!(
                    fight.machine().fair_cpu_pad(),
                    resumed.machine().fair_cpu_pad()
                );
            }
            replayed_macro = true;
            frame += 60;
        }
        if had_drive_move && had_airborne && replayed_macro {
            break;
        }
        frame += 1;
    }
    assert!(had_airborne, "the controller sees the real opponent's jump");
    assert!(had_drive_move, "the CPU uses a native Drive Rush or Impact");
    assert!(
        replayed_macro,
        "a rollback restarts during a queued native macro"
    );
}

#[test]
fn faithful_arcade_cpu_keeps_the_rom_table_even_if_an_assisted_pad_is_enabled() {
    use sf_engine::config::MatchConfig;
    use sf_engine::fighter::{CPU_STRUCT, GAME_MODE, WORK_BASE};
    use sf_engine::front::cabinet::Players;
    use sf_engine::front::fight::{FightDips, FightKind, FightSetup};
    use sf_engine::roster::PlayMode;

    let Some(assets) = Path::new(ROMS)
        .is_dir()
        .then(|| load_assets(ROMS, MANIFEST).unwrap())
    else {
        eprintln!("skipping: local ROM set not present");
        return;
    };
    let mut config = MatchConfig::from_preset(RulesetPreset::Faithful1987);
    config.roster.mode = PlayMode::Arcade;
    let mut setup = FightSetup {
        kind: FightKind::Arcade,
        players: Players::One,
        config,
        dsw1: FightDips::DSW1,
        dsw2: FightDips::default().dsw2(),
    };
    let mut original = Fight::start(&assets.maincpu, setup, Some(0), None).unwrap();
    setup.config.p2_fair_cpu = true;
    let mut assisted = Fight::start(&assets.maincpu, setup, Some(0), None).unwrap();
    let idle = PlayerInputs::default();
    let cpu = (CPU_STRUCT - WORK_BASE) as usize;
    let mut wrote_range = false;
    for _ in 0..1800 {
        original.step(&assets, idle, idle, false);
        assisted.step(&assets, idle, idle, false);
        assert_eq!(original.machine().work_ram(), assisted.machine().work_ram());
        assert_eq!(
            original.machine().state.objectram,
            assisted.machine().state.objectram
        );
        assert!(assisted.machine().fair_cpu_pad().is_none());
        if flow_position(assisted.machine()) == FIGHT_LOOP {
            wrote_range |= assisted.machine().work_ram()[cpu + 0x16..cpu + 0x1b] != [0; 5];
            if wrote_range {
                break;
            }
        }
    }
    let machine = assisted.machine();
    assert_eq!(machine.work_ram()[(GAME_MODE - WORK_BASE) as usize], 2);
    assert!(wrote_range, "ROM AI wrote its preferred range and choices");
}
