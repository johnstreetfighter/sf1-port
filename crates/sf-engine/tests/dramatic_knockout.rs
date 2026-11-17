//! George's Dramatic Battle playtest notes, 2026-11-17: a knocked-out boss
//! stays on the screen instead of flying past its edge. Driven through the
//! front end's Fight on the native three-fighter slots. Local ROMs only.

use std::path::Path;
use std::sync::Arc;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::data_bank::DataBank;
use sf_engine::dramatic::{DramaticRules, LifebarMode};
use sf_engine::era::Revision;
use sf_engine::front::cabinet::{flow_position, Players};
use sf_engine::front::fight::{Fight, FightDips, FightKind, FightSetup};
use sf_engine::front::PlayerInputs;
use sf_engine::roster::{PlayMode, TournamentFighter};
use sf_video::render::{VISAREA_X, WIDTH};
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const FIGHT_LOOP: u32 = 0x3eafa;
/// The winner's victory pose playing out.
const VICTORY_LOOP: u32 = 0x3ec0e;

fn ready_fight(boss: TournamentFighter, lifebar: LifebarMode) -> Option<(Assets, Fight)> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    let assets = load_assets(ROMS, MANIFEST).expect("local ROM set assembles");
    let data = Arc::new(DataBank::build(&assets.maincpu, &[]));
    let mut config = MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul);
    config.roster.mode = PlayMode::Versus;
    config.roster.p1 = TournamentFighter::Ryu;
    config.roster.p2 = boss;
    config.dramatic = Some(DramaticRules {
        co_op: true,
        ally2: TournamentFighter::Ken,
        stage: boss,
        lifebar,
        friendly_fire: false,
        styles: [Revision::Sf1Plus; 3],
        rivalry: None,
    });
    let setup = FightSetup {
        kind: FightKind::Dramatic { co_op: true },
        players: Players::One,
        config,
        dsw1: FightDips::DSW1,
        dsw2: FightDips::default().dsw2(),
    };
    let mut fight = Fight::start_banked(&assets.maincpu, setup, Some(0), None, Some(data))
        .expect("Dramatic starts the native cabinet driver");
    for _ in 0..1_500 {
        if flow_position(fight.machine()) == FIGHT_LOOP && !fight.driving() {
            return Some((assets, fight));
        }
        step(&mut fight, &assets);
    }
    panic!("Dramatic never entered the active native fight loop");
}

fn step(fight: &mut Fight, assets: &Assets) {
    fight.step(assets, PlayerInputs::default(), PlayerInputs::default(), false);
}

fn health(fight: &Fight) -> [u16; 3] {
    fight.machine().toggle_state().dramatic.health
}

/// Walk in and throw a heavy punch every 50 frames.
fn pressing(t: u32) -> PlayerInputs {
    let mut pad = PlayerInputs::default();
    let phase = t % 50;
    pad.directions[0] = phase < 20;
    pad.buttons[2] = (30..32).contains(&phase);
    pad
}

/// The allies in `attackers` walk in and punch a boss on its last health
/// point until it is knocked out.
fn knock_out_the_boss(fight: &mut Fight, assets: &Assets, attackers: [bool; 2]) {
    for t in 0..3_000 {
        // The boss may not win the exchange: the allies stay healthy.
        let mut snapshot = fight.save_netplay_snapshot(0);
        let hp = &mut snapshot.machine.toggle_state.dramatic.health;
        hp[1] = hp[1].min(1);
        for (slot, attacking) in [(0, attackers[0]), (2, attackers[1])] {
            if attacking {
                hp[slot] = hp[slot].max(100);
            }
        }
        fight.restore_netplay_snapshot(&snapshot);
        let pads = attackers.map(|on| if on { pressing(t) } else { PlayerInputs::default() });
        fight.step(assets, pads[0], pads[1], false);
        if health(fight)[1] == 0 {
            return;
        }
    }
    panic!("the boss was never knocked out");
}

fn boss_centre(fight: &Fight) -> i32 {
    fight.machine().diagnostic_telemetry().p2.center_x
}

/// Move the boss's native sprite grid `dx` pixels across the screen.
fn shift_boss(fight: &mut Fight, dx: i32) {
    let entity = usize::from(fight.machine().diagnostic_telemetry().p2.entity_offset);
    let mut snapshot = fight.save_netplay_snapshot(0);
    let objects = &mut snapshot.machine.state.objectram;
    let cells = (usize::from(objects[entity + 0xc]) + 1) * (usize::from(objects[entity + 0xd]) + 1);
    for cell in 0..cells {
        let x = entity + cell * 0x40 + 6;
        let value = u16::from_be_bytes(objects[x..x + 2].try_into().unwrap()).wrapping_add(dx as u16);
        objects[x..x + 2].copy_from_slice(&value.to_be_bytes());
    }
    fight.restore_netplay_snapshot(&snapshot);
}

#[test]
fn a_knocked_out_boss_stays_on_the_screen() {
    let right_edge = VISAREA_X as i32 + WIDTH as i32;
    let visible = VISAREA_X as i32 + 16..=right_edge - 16;
    for boss in [TournamentFighter::Geki, TournamentFighter::Ken] {
        let Some((assets, mut fight)) = ready_fight(boss, LifebarMode::Independent) else { return };
        knock_out_the_boss(&mut fight, &assets, [true, true]);
        // On the knockout's own frame, stand the boss by the edge of the
        // screen it is about to be knocked toward: away from ally 1.
        let telemetry = fight.machine().diagnostic_telemetry();
        let rightward = telemetry.p2.center_x > telemetry.p1.center_x;
        let edge = if rightward { right_edge - 24 } else { VISAREA_X as i32 + 24 };
        shift_boss(&mut fight, edge - telemetry.p2.center_x);
        let mut path = vec![boss_centre(&fight)];
        let mut flows = std::collections::BTreeSet::new();
        for _ in 0..900 {
            step(&mut fight, &assets);
            let flow = flow_position(fight.machine());
            flows.insert(flow);
            if flow == VICTORY_LOOP {
                break;
            }
            path.push(boss_centre(&fight));
        }
        eprintln!("{boss:?}: {flows:x?} {path:?}");
        let travelled = path.last().unwrap() - path[0];
        assert!(path.len() > 8, "{boss:?}'s knockdown played out: {path:?}");
        eprintln!("{boss:?} was sent {} and travelled {travelled}", if rightward { "right" } else { "left" });
        assert!(flows.contains(&VICTORY_LOOP), "{boss:?}'s knockout reached the victory pose: {flows:x?}");
        for (frame, x) in path.iter().enumerate() {
            assert!(visible.contains(x), "{boss:?} left the screen at x={x} on frame {frame} of its knockout: {path:?}");
        }
        assert!(fight.machine().unrecovered().is_empty());
    }
}
