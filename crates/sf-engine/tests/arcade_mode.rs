//! Arcade route, positive-edge bonus stages, rival lines, and endings (`#102`, `O11`).

use sf_engine::config::MatchConfig;
use sf_engine::fighter::CPU_STRUCT;
use sf_engine::front::cabinet::{flow_position, Players};
use sf_engine::front::fight::{Fight, FightDips, FightKind, FightSetup};
use sf_engine::front::PlayerInputs;
use sf_engine::roster::{PlayMode, TournamentFighter};
use sf_engine::tasks::TIME_DIGITS;
use sf_video::romset::{load_assets, Assets};

const FIGHT_LOOP: u32 = 0x3eafa;
const STAGE_NUMBER: usize = 0x4003;
const MATCH_IN_STAGE: usize = 0x4004;
const BONUS_ENTRIES: [u32; 4] = [0x40dd4, 0x41814, 0x41aa6, 0x420cc];
const ENDING_START: u32 = 0x44a60;
const ENDING_FINAL_HOLD: u32 = 0x44c04;
const WORK_BASE: u32 = 0xff8000;

fn roms() -> Option<Assets> {
    let roms = "../../tools/reference/local/roms/sf";
    if !std::path::Path::new(roms).is_dir() {
        eprintln!("skipping: local ROM set is absent");
        return None;
    }
    Some(load_assets(roms, "../../tools/reference/rom-manifest.json").expect("local ROMs assemble"))
}

fn arcade(assets: &Assets, flag: u8, mut config: MatchConfig) -> Fight {
    config.roster.mode = PlayMode::Arcade;
    config.roster.p1 = TournamentFighter::Ken;
    Fight::start(
        &assets.maincpu,
        FightSetup {
            kind: FightKind::Arcade,
            players: Players::One,
            config,
            dsw1: FightDips::DSW1,
            dsw2: FightDips::default().dsw2(),
        },
        Some(flag),
        None,
    )
    .expect("native flag selection")
}

fn step(fight: &mut Fight, assets: &Assets) {
    fight.step(
        assets,
        PlayerInputs::default(),
        PlayerInputs::default(),
        false,
    );
}

fn win_on_time(fight: &mut Fight) {
    // Start at the round's timeout with P1 ahead on life. The native scheduler
    // must still award the round and handle every match/bonus transition.
    let mut snapshot = fight.save_netplay_snapshot(0);
    let time = (TIME_DIGITS - WORK_BASE) as usize;
    let cpu_life = (CPU_STRUCT - WORK_BASE) as usize + 6;
    snapshot.machine.work[time..time + 3].fill(0);
    snapshot.machine.work[cpu_life..cpu_life + 2].copy_from_slice(&[1, 0]);
    snapshot.machine.toggle_state.era.fighters[1].sub_hp = 20;
    fight.restore_netplay_snapshot(&snapshot);
}

#[test]
fn native_arcade_runs_ten_opponents_four_bonuses_and_the_ending() {
    use TournamentFighter::*;
    let Some(assets) = roms() else { return };
    let mut fight = arcade(&assets, 0, MatchConfig::definitive());
    let mut visited = Vec::new();
    let mut bonus_entries = Vec::new();
    let mut ending = None;
    for _ in 0..30_000 {
        step(&mut fight, &assets);
        let machine = fight.machine();
        let flow = flow_position(machine);
        if flow == FIGHT_LOOP {
            let stage = usize::from(machine.work_ram()[STAGE_NUMBER]);
            let match_in_stage = usize::from(machine.work_ram()[MATCH_IN_STAGE]);
            let step = stage * 2 + match_in_stage;
            let opponent = machine.arcade_opponent().expect("native ladder opponent");
            if visited.len() == step {
                visited.push(opponent);
            } else {
                assert_eq!(visited.get(step), Some(&opponent), "unexpected ladder jump");
            }
            win_on_time(&mut fight);
        }
        if BONUS_ENTRIES.contains(&flow) && bonus_entries.last() != Some(&flow) {
            bonus_entries.push(flow);
        }
        if flow == ENDING_START && ending.is_none() {
            ending = Some(fight.save_netplay_snapshot(0));
        }
        if flow == ENDING_FINAL_HOLD {
            break;
        }
    }
    assert_eq!(
        visited,
        [Retsu, Geki, Joe, Mike, Lee, Gen, Birdie, Eagle, Adon, Sagat]
    );
    assert_eq!(bonus_entries, BONUS_ENTRIES);
    assert_eq!(flow_position(fight.machine()), ENDING_FINAL_HOLD);
    assert!(fight.machine().unrecovered().is_empty());

    // Save/restore from the actual ending preserves its scheduler and work RAM.
    let ending = ending.expect("the Thailand victory entered the ending");
    fight.restore_netplay_snapshot(&ending);
    for _ in 0..150 {
        step(&mut fight, &assets);
    }
    let checksum = fight.state_checksum();
    fight.restore_netplay_snapshot(&ending);
    for _ in 0..150 {
        step(&mut fight, &assets);
    }
    assert_eq!(fight.state_checksum(), checksum);
}

#[test]
fn other_flags_start_the_native_ladder_with_their_local_rival() {
    use TournamentFighter::*;
    let Some(assets) = roms() else { return };
    for (flag, opponent) in [(1, Lee), (2, Joe), (3, Birdie)] {
        let mut fight = arcade(&assets, flag, MatchConfig::definitive());
        for _ in 0..1_500 {
            step(&mut fight, &assets);
            if flow_position(fight.machine()) == FIGHT_LOOP {
                break;
            }
        }
        assert_eq!(flow_position(fight.machine()), FIGHT_LOOP);
        assert_eq!(fight.country(), Some(flag));
        assert_eq!(fight.machine().arcade_opponent(), Some(opponent));
    }
}

#[test]
fn native_board_breaking_respects_press_and_release_rules() {
    const POWER_METER: u32 = 0x40f14;
    const STRIKE_WINDUP: u32 = 0x41078;
    let Some(assets) = roms() else { return };

    for positive_edge in [false, true] {
        let mut config = MatchConfig::definitive();
        config.toggles.positive_edge_attacks = positive_edge;
        let mut fight = arcade(&assets, 0, config);
        for _ in 0..5_000 {
            step(&mut fight, &assets);
            match flow_position(fight.machine()) {
                POWER_METER => break,
                FIGHT_LOOP => win_on_time(&mut fight),
                _ => {}
            }
        }
        assert_eq!(flow_position(fight.machine()), POWER_METER);
        let ready = fight.save_netplay_snapshot(0);
        for button in [0, 2] {
            fight.restore_netplay_snapshot(&ready);
            let mut punch = PlayerInputs::default();
            punch.buttons[button] = true;
            for _ in 0..2 {
                fight.step(&assets, punch, PlayerInputs::default(), false);
            }
            assert_eq!(
                flow_position(fight.machine()),
                if positive_edge { STRIKE_WINDUP } else { POWER_METER },
            );
            for _ in 0..2 { step(&mut fight, &assets); }
            assert_eq!(flow_position(fight.machine()), STRIKE_WINDUP);
            let power = fight.machine().state.objectram[0x434];
            if button == 0 {
                assert!(power < 5, "light punch keeps the native low power band, got {power}");
            } else {
                assert!(power >= 5, "heavy punch reaches the native high power band, got {power}");
            }
        }
        assert!(fight.machine().unrecovered().is_empty());
    }
}
