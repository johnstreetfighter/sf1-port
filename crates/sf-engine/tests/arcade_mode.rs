//! Arcade route, positive-edge bonus stages, rival lines, and endings (`#102`, `O11`).

use std::collections::HashSet;

use sf_engine::arcade::{
    ending_for, player_intro, press_at, release_at, rival_taunt, route_from, win_quote, BonusKind,
    BonusStage, Country, Sample, ROUTE,
};
use sf_engine::config::MatchConfig;
use sf_engine::fighter::CPU_STRUCT;
use sf_engine::front::cabinet::{flow_position, Players};
use sf_engine::front::fight::{Fight, FightDips, FightKind, FightSetup};
use sf_engine::front::PlayerInputs;
use sf_engine::roster::{PlayMode, TournamentFighter};
use sf_engine::tasks::TIME_DIGITS;
use sf_video::asset_pack::validate_tile;
use sf_video::romset::{load_assets, Assets};

#[test]
fn positive_edge_scores_the_press_and_release_mode_scores_the_let_go() {
    let plank_press = BonusStage::plank(true);
    let timed = plank_press.judge(&press_at(10, 3));
    assert_eq!(timed.hits, 1);
    assert_eq!(timed.points, 3000);

    let early_hold = vec![
        Sample {
            frame: 2,
            down: true,
            power: 3,
        },
        Sample {
            frame: 10,
            down: true,
            power: 3,
        },
    ];
    assert_eq!(
        plank_press.judge(&early_hold).hits,
        0,
        "a hold already down is not a new press"
    );

    let weak = plank_press.judge(&press_at(10, 1));
    assert_eq!(weak.hits, 0, "light power misses the plank");

    let outside = plank_press.judge(&press_at(4, 3));
    assert_eq!(outside.hits, 0, "a press outside the window misses");

    let release = BonusStage::plank(false);
    assert_eq!(release.judge(&release_at(10, 3)).hits, 1);
    assert_eq!(
        release.judge(&press_at(10, 3)).hits,
        0,
        "a press with no release does not score"
    );
    assert_eq!(release.judge(&release_at(4, 3)).hits, 0);

    let barrels = BonusStage::barrel(true);
    let mut swings = press_at(6, 1);
    swings.extend(press_at(13, 2));
    swings.extend(press_at(19, 3));
    let broken = barrels.judge(&swings);
    assert_eq!(broken.kind, BonusKind::Barrel);
    assert_eq!(broken.hits, 3);
    assert_eq!(broken.points, 6000);

    let mut soft_last = press_at(6, 1);
    soft_last.extend(press_at(13, 2));
    soft_last.extend(press_at(19, 1));
    assert_eq!(
        barrels.judge(&soft_last).hits,
        2,
        "the heavy barrel refuses a light tap"
    );
}

#[test]
fn all_fighters_have_distinct_arcade_lines_and_ending_panels() {
    let mut intros = HashSet::new();
    let mut taunts = HashSet::new();
    let mut wins = HashSet::new();
    let mut first_panels = HashSet::new();

    for fighter in TournamentFighter::ALL {
        assert!(intros.insert(player_intro(fighter)));
        assert!(taunts.insert(rival_taunt(fighter)));
        assert!(wins.insert(win_quote(fighter)));

        let ending = ending_for(fighter);
        assert_eq!(ending.panels.len(), 4);
        for panel in &ending.panels {
            validate_tile(panel).unwrap();
        }
        assert_ne!(ending.panels[0].pixels, ending.panels[1].pixels);
        assert!(first_panels.insert(ending.panels[0].pixels));
        assert!(ending.credits.iter().any(|line| line == fighter.name()));
        assert!(ending.credits.iter().any(|line| line == "Thailand"));
        assert!(ending.credits.iter().any(|line| line == "The end"));
    }
    assert!(ending_for(TournamentFighter::Eagle)
        .credits
        .iter()
        .any(|line| line.contains("Sticks back in the case")));
    assert_eq!(route_from(Country::Japan), ROUTE);
    assert_eq!(
        route_from(Country::England),
        &[Country::England, Country::Thailand]
    );
    assert_eq!(route_from(Country::Thailand), &[Country::Thailand]);
}

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
