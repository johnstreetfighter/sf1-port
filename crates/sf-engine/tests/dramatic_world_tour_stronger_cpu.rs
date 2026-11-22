//! Ticket #215 (`SC23`, spec #192): Dramatic Battle and World Tour with the
//! stronger Fair CPU.
//!
//! Dramatic Battle's boss and its solo ally each play the stronger game on
//! their own with the developer-only switch on, alone on their pads, each
//! with its own state and random numbers and its own opponent; with the
//! switch off they play the old game beside the Utility AI. Three Fair
//! CPUs at once (the measuring run's, on Player 1, with the boss's and the
//! ally's) re-run under rollback to the same fight. A World Tour mission
//! that allows only normals and throws is kept to: no special, era command
//! or special route is pressed for, where the same fight without the
//! mission has them. Local ROMs only; each test skips without them.

use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use sf_engine::config::{MatchConfig, RulesetPreset, WorldTourRules};
use sf_engine::data_bank::DataBank;
use sf_engine::dramatic::{DramaticRules, LifebarMode};
use sf_engine::era::{loadouts, Revision, StylePick};
use sf_engine::fair_cpu::{pad_parts, perceive_fighters, Body, DOWN, LEFT, RIGHT};
use sf_engine::fighter::{CPU_STRUCT, P1_STRUCT, P2_STRUCT};
use sf_engine::front::cabinet::{flow_position, Players};
use sf_engine::front::fight::{Fight, FightDips, FightKind, FightSetup};
use sf_engine::front::PlayerInputs;
use sf_engine::measure::{dramatic_machine, DramaticSpec};
use sf_engine::roster::{PlayMode, TournamentFighter};
use sf_engine::run::PowerOnMachine;
use sf_engine::world_tour::native::{MissionKind, RuleModifier};
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const FIGHT_LOOP: u32 = 0x3eafa;
/// Pad bits of the buttons (`fair_cpu::pad_parts`): LP is 0x10.
const MP: u16 = 0x20;
const HP: u16 = 0x40;
const MK: u16 = 0x100;
const HK: u16 = 0x200;
const BUTTONS: u16 = 0x3f0;
/// The most frames netplay rolls back.
const MAX_ROLLBACK: usize = 8;

fn assets() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(load_assets(ROMS, MANIFEST).expect("local ROM set assembles"))
}

/// Run `f` on a thread with a large stack: a fight is deep in the
/// translated program's calls.
fn deep<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new().stack_size(256 << 20).spawn(f).unwrap().join().unwrap()
}

fn inputs(pad: u16) -> PlayerInputs {
    let (directions, buttons) = pad_parts(pad);
    PlayerInputs { directions, buttons }
}

/// Player 1's pad on frame `i` of a script that walks in, presses light
/// punches, holds a guard, jumps in and stands, in four-second turns.
fn p1_script(i: u32) -> PlayerInputs {
    let pad = match i % 240 {
        0..60 => RIGHT,
        60..90 if i % 6 == 0 => 0x10,
        90..150 => LEFT,
        150..156 => RIGHT | 8,
        _ => 0,
    };
    inputs(pad)
}

/// A solo Dramatic fight: Ryu as Ally 1, the boss and Ally 2, in `styles`,
/// at `level`, with the switch as given, its two CPUs seeded with `seed`.
fn dramatic_fight(assets: &Assets, stronger: bool, styles: [Revision; 3], level: u8, seed: u64) -> Fight {
    let data = Arc::new(DataBank::build(&assets.maincpu, &[]));
    let mut config = MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul);
    config.roster.mode = PlayMode::Versus;
    config.roster.p1 = TournamentFighter::Ryu;
    config.roster.p2 = TournamentFighter::Ryu;
    config.roster.what_if_styles = true;
    config.difficulty = level;
    config.stronger_cpu = stronger;
    config.dramatic = Some(DramaticRules {
        co_op: false,
        ally2: TournamentFighter::Ryu,
        stage: TournamentFighter::Retsu,
        lifebar: LifebarMode::Independent,
        friendly_fire: false,
        styles,
        rivalry: None,
    });
    let setup = FightSetup {
        kind: FightKind::Dramatic { co_op: false },
        players: Players::One,
        config,
        dsw1: FightDips::DSW1,
        dsw2: FightDips { difficulty: level, round_time: 100 }.dsw2(),
    };
    let mut fight = Fight::start_banked(&assets.maincpu, setup, Some(0), None, Some(data)).expect("Dramatic starts");
    fight.seed_dramatic_cpu(seed);
    fight
}

/// The three fighters as the frame shows them: Ally 1, the boss, Ally 2.
fn bodies(m: &PowerOnMachine) -> [Body; 3] {
    let live = flow_position(m) == FIGHT_LOOP;
    let at = |me, foe| perceive_fighters(m.state.frame, live, m.work_ram(), &m.state.objectram, me, foe).me;
    [at(P1_STRUCT, CPU_STRUCT), at(CPU_STRUCT, P1_STRUCT), at(P2_STRUCT, CPU_STRUCT)]
}

/// What a Dramatic fight's two CPUs did over `frames` frames of
/// [`p1_script`].
#[derive(Default)]
struct Watched {
    /// Frames the round was fought.
    fought: u32,
    /// Each followed a game plan at some point.
    plans: [bool; 2],
    /// Frames the Utility AI held a command queued for each fighter (its
    /// era-style commands beside the old play's Fair CPU).
    utility: [u32; 2],
    /// Views each decided from that were checked against the frames.
    views: [u32; 2],
    hashes: Vec<u64>,
    /// The stronger boss turned from one ally to the other, and frames the
    /// other ally was the nearer while it kept facing the one it held.
    turns: u32,
    held_off: u32,
}

/// Which ally is nearer the boss in `at` (Ally 1, the boss, Ally 2), as
/// the boss's old target rule picks: Ally 2 when Ally 1 is out of health.
fn nearer(at: &[Body; 3], health: [u16; 3]) -> u32 {
    let gap = |ally: &Body| ally.x.abs_diff(at[1].x);
    if health[0] == 0 || (health[2] > 0 && gap(&at[2]) < gap(&at[0])) { P2_STRUCT } else { P1_STRUCT }
}

fn watch(assets: &Assets, fight: &mut Fight, frames: u32) -> Watched {
    let mut w = Watched::default();
    let mut seen: HashMap<u32, ([Body; 3], u32)> = HashMap::new();
    // The nearer ally after each frame, and the health then: what the
    // boss's next press decides from.
    let mut nearest: VecDeque<u32> = VecDeque::new();
    let (mut held, mut health_before) = (0u32, [0u16; 3]);
    let delay = sf_engine::fair_cpu::Level::from_difficulty(fight.setup().config.difficulty).unfocused_reaction() as usize;
    for i in 0..frames {
        fight.step(assets, p1_script(i), PlayerInputs::default(), false);
        let m = fight.machine();
        let at = bodies(m);
        let target = m.toggle_state().dramatic.boss_target;
        seen.insert(m.state.frame, (at, target));
        let health = m.toggle_state().dramatic.health;
        // The stronger boss turns to the other ally only once he has been
        // the nearer for its slower reaction delay in a row (or the one it
        // faced is out of health).
        if target != 0 && held != 0 && target != held {
            let faced_out = health_before[if held == P1_STRUCT { 0 } else { 2 }] == 0;
            let waited = nearest.len() >= delay && nearest.iter().rev().take(delay).all(|&n| n == target);
            assert!(faced_out || waited, "frame {i}: turned to {target:x} with the nearer {:?}", nearest.iter().rev().take(delay).collect::<Vec<_>>());
            w.turns += 1;
        }
        let now_nearer = nearer(&at, health);
        w.held_off += u32::from(target != 0 && now_nearer != target);
        nearest.push_back(now_nearer);
        if nearest.len() > 64 {
            nearest.pop_front();
        }
        (held, health_before) = (target, health);
        w.fought += u32::from(flow_position(m) == FIGHT_LOOP);
        w.hashes.push(fight.state_checksum());
        let cpus = m.dramatic_fair_cpus().expect("the boss's and the ally's CPUs");
        let [_, boss_hp, ally_hp] = m.toggle_state().dramatic.health;
        // What the boss's and the ally's fighters take is what their CPUs
        // ended on (the old play's Utility AI writes its command back into
        // the Fair CPU).
        if boss_hp > 0 {
            assert_eq!(inputs(cpus[0].pad()), m.toggle_state().dramatic.boss_pad, "frame {i}: the boss's pad");
        }
        if boss_hp > 0 && ally_hp > 0 {
            assert_eq!(inputs(cpus[1].pad()), m.applied_p2_pad(), "frame {i}: the ally's pad");
        }
        for (k, slot) in [1, 2].into_iter().enumerate() {
            w.utility[k] += u32::from(m.toggle_state().utility[slot].macro_remaining() > 0);
        }
        for k in 0..2 {
            w.plans[k] |= cpus[k].following_game_plan();
            let view = cpus[k].last_view();
            let Some(&(at, target)) = seen.get(&view.frame).filter(|_| view.fighting) else { continue };
            // Each sees its own fighter, and its own opponent: the stronger
            // boss the ally it held as its target on that frame (the old
            // boss the nearer, either), the ally the boss.
            let foes = match (k, target) {
                (1, _) => vec![at[1]],
                (_, 0) => vec![at[0], at[2]],
                (_, t) => vec![at[if t == P1_STRUCT { 0 } else { 2 }]],
            };
            let me = at[if k == 0 { 1 } else { 2 }];
            assert_eq!(view.me, me, "frame {i}: CPU {k} sees its own fighter");
            assert!(
                foes.iter().any(|foe| foe.x == view.foe.x && foe.health == view.foe.health),
                "frame {i}: CPU {k} faces its opponent: {:?} not in {foes:?}",
                view.foe,
            );
            w.views[k] += 1;
        }
    }
    w
}

#[test]
fn with_the_switch_on_the_boss_and_the_ally_each_play_the_stronger_game_alone() {
    let Some(assets) = assets() else { return };
    deep(move || {
        let mut fight = dramatic_fight(&assets, true, [Revision::Sf6; 3], 4, 5);
        let w = watch(&assets, &mut fight, 3_600);
        assert!(w.fought > 900, "the round was fought: {} frames", w.fought);
        // The stronger game: each takes up game plans (Ryu's SF6 ones).
        assert!(w.plans[0] && w.plans[1], "game plans followed: {:?}", w.plans);
        // Alone on its pad: no Utility AI command is ever queued beside it.
        assert_eq!(w.utility, [0, 0], "frames with a Utility AI command queued");
        assert!(w.views[0] > 600 && w.views[1] > 600, "views checked: {:?}", w.views);
        // The held target was exercised: the other ally was the nearer on
        // some frames while the boss kept its target, and it did turn.
        assert!(w.turns >= 1 && w.held_off > 0, "turns {}, frames held against the nearer {}", w.turns, w.held_off);
    });
}

#[test]
fn with_the_switch_off_the_boss_and_the_ally_play_the_old_game_beside_the_utility_ai() {
    let Some(assets) = assets() else { return };
    deep(move || {
        let mut fight = dramatic_fight(&assets, false, [Revision::Sf6; 3], 4, 5);
        let w = watch(&assets, &mut fight, 3_600);
        assert!(w.fought > 900, "the round was fought: {} frames", w.fought);
        assert_eq!(w.plans, [false, false], "no game plan in the old play");
        assert_eq!(w.turns + w.held_off, 0, "the old boss holds no target: it faces the nearer");
        assert!(w.utility[0] > 0 && w.utility[1] > 0, "the Utility AI adds commands: {:?}", w.utility);
        let mut on = dramatic_fight(&assets, true, [Revision::Sf6; 3], 4, 5);
        let stronger = watch(&assets, &mut on, 3_600);
        assert_ne!(stronger.hashes, w.hashes, "the switch changes the fight");
    });
}

#[test]
fn the_boss_and_the_ally_each_have_their_own_random_numbers_and_the_seed_reaches_the_fight() {
    let Some(assets) = assets() else { return };
    deep(move || {
        let fight = dramatic_fight(&assets, true, [Revision::Sf6; 3], 4, 5);
        let cpus = fight.machine().dramatic_fair_cpus().expect("seeded");
        assert_ne!(cpus[0], cpus[1], "two fresh CPUs seeded apart");
        let first = watch(&assets, &mut dramatic_fight(&assets, true, [Revision::Sf6; 3], 4, 5), 1_800).hashes;
        let again = watch(&assets, &mut dramatic_fight(&assets, true, [Revision::Sf6; 3], 4, 5), 1_800).hashes;
        assert_eq!(first, again, "the same seed gives the same fight");
        let other = watch(&assets, &mut dramatic_fight(&assets, true, [Revision::Sf6; 3], 4, 6), 1_800).hashes;
        assert_ne!(first, other, "another seed gives another fight");
    });
}

/// The measuring run's three Fair CPUs on a solo Dramatic fight of Ryu in
/// the SF6 era style, stepped to a second into the first round.
fn three_cpus(assets: &Assets, seed: u64) -> PowerOnMachine {
    let spec = DramaticSpec {
        fighters: [TournamentFighter::Ryu; 3],
        styles: [Revision::Sf6; 3],
        difficulty: [4; 3],
        seed,
        stronger: [true; 3],
    };
    let mut m = dramatic_machine(&assets.maincpu, None, &spec).expect("the Dramatic fight starts");
    let mut live = 0;
    for _ in 0..3_000 {
        m.set_cabinet_inputs(false, false, false, false, false);
        m.step();
        live += u32::from(flow_position(&m) == FIGHT_LOOP);
        if live == 60 {
            break;
        }
    }
    assert_eq!(live, 60, "the round starts");
    assert!(m.dramatic_fair_cpus().is_some() && m.p1_fair_cpu().is_some(), "three Fair CPUs");
    m
}

fn step(m: &mut PowerOnMachine) {
    m.set_cabinet_inputs(false, false, false, false, false);
    m.step();
}

#[test]
fn three_fair_cpus_at_once_rerun_under_rollback_to_the_same_fight() {
    // A snapshot before every step, as netplay keeps them; every 25th
    // frame the worst rollback it allows: back 8 frames and re-run them
    // and this one. The machine must come out where one that never rolled
    // back is, the three CPUs' state included.
    let Some(assets) = assets() else { return };
    deep(move || {
        let mut m = three_cpus(&assets, 3);
        let mut reference = m.clone();
        let mut ring: VecDeque<_> = VecDeque::new();
        let (mut rollbacks, mut pressed) = (0, [false; 3]);
        for k in 0..600 {
            ring.push_back(m.save_snapshot());
            if ring.len() > MAX_ROLLBACK + 1 {
                ring.pop_front();
            }
            step(&mut reference);
            if k % 25 == 24 && ring.len() == MAX_ROLLBACK + 1 {
                m.restore_snapshot(&ring[0]);
                for _ in 0..=MAX_ROLLBACK {
                    step(&mut m);
                }
                rollbacks += 1;
            } else {
                step(&mut m);
            }
            assert_eq!(m.gameplay_hash(), reference.gameplay_hash(), "frame {k}");
            let cpus = m.dramatic_fair_cpus().unwrap();
            for (seen, pad) in pressed.iter_mut().zip([m.p1_fair_cpu().unwrap().pad(), cpus[0].pad(), cpus[1].pad()]) {
                *seen |= pad != 0;
            }
        }
        assert!(rollbacks >= 20, "{rollbacks} rollbacks");
        assert_eq!(pressed, [true; 3], "all three play");
        // The CPUs are in the snapshot: one restored without them would not
        // play on the same (a fresh boss CPU in place of the live one).
        let mut stripped = m.save_snapshot();
        stripped.dramatic_cpu = stripped.dramatic_cpu.map(|cpus| Box::new([Default::default(), cpus[1]]));
        let mut other = m.clone();
        other.restore_snapshot(&stripped);
        let mut same = m.clone();
        let mut parted = false;
        for _ in 0..240 {
            step(&mut other);
            step(&mut same);
            parted |= other.gameplay_hash() != same.gameplay_hash();
        }
        assert!(parted, "the boss's CPU state reaches the fight");
    });
}

/// A solo Dramatic `Fight` (drawn as the front end draws it) with the
/// measuring run's three Fair CPUs on Ryu in the SF6 era style, a second
/// into the first round.
fn three_cpu_fight(assets: &Assets, stronger: bool, seed: u64) -> Fight {
    let spec = DramaticSpec {
        fighters: [TournamentFighter::Ryu; 3],
        styles: [Revision::Sf6; 3],
        difficulty: [4; 3],
        seed,
        stronger: [stronger; 3],
    };
    let mut fight = sf_engine::measure::dramatic_fight(&assets.maincpu, None, &spec).expect("the Dramatic fight starts");
    let mut live = 0;
    for _ in 0..3_000 {
        fight.step(assets, PlayerInputs::default(), PlayerInputs::default(), false);
        live += u32::from(flow_position(fight.machine()) == FIGHT_LOOP);
        if live == 60 {
            break;
        }
    }
    assert_eq!(live, 60, "the round starts");
    fight
}

/// One frame as netplay's rollback engine runs it (`RollbackEngine::simulate`):
/// a snapshot saved, the frame stepped and drawn, the checksum taken.
fn simulate(fight: &mut Fight, assets: &Assets, frame: u32) {
    std::hint::black_box(fight.save_netplay_snapshot(frame));
    fight.step(assets, PlayerInputs::default(), PlayerInputs::default(), false);
    std::hint::black_box(fight.state_checksum());
}

/// The worst rollback netplay allows, timed: back 8 frames, and those and
/// the new one re-run as netplay re-runs them. The fight comes out one
/// frame on.
fn worst_rollback(fight: &mut Fight, assets: &Assets) -> Duration {
    let back = fight.save_netplay_snapshot(0);
    for frame in 0..MAX_ROLLBACK as u32 {
        simulate(fight, assets, frame);
    }
    let t = Instant::now();
    fight.restore_netplay_snapshot(&back);
    for frame in 0..=MAX_ROLLBACK as u32 {
        simulate(fight, assets, frame);
    }
    t.elapsed()
}

fn median(times: &mut [Duration]) -> Duration {
    times.sort();
    times[times.len() / 2]
}

#[test]
fn a_worst_rollback_with_three_stronger_fair_cpus_costs_little_more_than_with_the_old_play() {
    // SC23 (`#215`): the stronger play's share of a worst rollback, as
    // netplay re-runs one, measured in this process against the old play's
    // on the same machine, taken in turns so both see the same load. The
    // median must stay under twice the old play's: what the frames cost
    // with drawing and checksums is the same for both, so a stronger play
    // that grew costly (a search each frame, a slow route lookup) shows.
    let Some(assets) = assets() else { return };
    deep(move || {
        let mut fights = [three_cpu_fight(&assets, false, 1), three_cpu_fight(&assets, true, 1)];
        let mut times = [Vec::new(), Vec::new()];
        for _ in 0..24 {
            for (fight, times) in fights.iter_mut().zip(times.iter_mut()) {
                times.push(worst_rollback(fight, &assets));
            }
        }
        let [old, new] = times.map(|mut t| median(&mut t));
        assert!(new < old * 2, "a worst rollback: stronger play {new:?}, old play {old:?}");
    });
}

/// A World Tour fight of Ryu against the Fair CPU's Ryu, both in `style`,
/// at Very Difficult with the switch on, under `mission` (or none).
fn tour_fight(assets: &Assets, mission: Option<MissionKind>, style: Revision) -> Fight {
    let data = Arc::new(DataBank::build(&assets.maincpu, &[]));
    let mut config = MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul);
    config.roster.mode = PlayMode::Versus;
    config.roster.p1 = TournamentFighter::Ryu;
    config.roster.p2 = TournamentFighter::Ryu;
    config.roster.what_if_styles = true;
    config.p2_fair_cpu = true;
    config.difficulty = 4;
    config.stronger_cpu = true;
    config.world_tour = mission.map(|mission| WorldTourRules { mission, bonuses: None, starting_hp: None });
    let setup = FightSetup {
        kind: FightKind::WorldTour,
        players: Players::Two,
        config,
        dsw1: FightDips::DSW1,
        dsw2: FightDips { difficulty: 4, round_time: 100 }.dsw2(),
    };
    let mut fight = Fight::start_banked(&assets.maincpu, setup, Some(0), None, Some(data)).expect("the fight starts");
    let pick = StylePick::for_revision(style, loadouts(style)[0]);
    fight.set_selected_styles([Some(pick); 2]);
    fight
}

/// What the Fair CPU pressed for over a fight that needs a special or an
/// era command, counted in frames.
#[derive(Debug, Default)]
struct Specials {
    /// A button pressed within six frames of the stick down-forward: the
    /// end of a motion special (a quarter circle forward, a dragon punch).
    motions: u32,
    /// Two buttons of an era command at once: medium punch and kick (a
    /// Drive Rush or Parry), heavy punch and kick (a Drive Impact).
    chords: u32,
    /// Frames it entered a route from its book.
    routes: u32,
    /// Frames its pad pressed a button at all.
    buttons: u32,
}

fn specials(assets: &Assets, fight: &mut Fight, frames: u32) -> Specials {
    let mut found = Specials::default();
    let mut recent: VecDeque<u16> = VecDeque::new();
    for i in 0..frames {
        fight.step(assets, p1_script(i), PlayerInputs::default(), false);
        let Some(cpu) = fight.machine().fair_cpu() else { continue };
        let pad = cpu.pad();
        let forward = if cpu.last_view().me.facing_left { LEFT } else { RIGHT };
        if pad & BUTTONS != 0 {
            found.buttons += 1;
            if recent.iter().any(|&p| p & 0xf == DOWN | forward) {
                found.motions += 1;
            }
        }
        if pad & (MP | MK) == MP | MK || pad & (HP | HK) == HP | HK {
            found.chords += 1;
        }
        found.routes += u32::from(cpu.route_index().is_some());
        recent.push_back(pad);
        if recent.len() > 6 {
            recent.pop_front();
        }
    }
    found
}

#[test]
fn a_world_tour_mission_of_normals_and_throws_is_kept_to_by_the_stronger_play() {
    // The Pure Footsies trial (normals and throws only). The engine refuses
    // a special to either fighter whatever is pressed, so what is checked
    // is what the Fair CPU presses for: no motion special, no era command
    // and no route from its book. The same fight without the mission
    // presses for them, so the check can fail. In the SF6 era style (game
    // plans, routes, Drive moves) and in 1987+ (the trial's own style).
    let Some(assets) = assets() else { return };
    deep(move || {
        let footsies = MissionKind::RuleModifierTrial(RuleModifier::PureFootsies);
        for style in [Revision::Sf6, Revision::Sf1Plus] {
            let open = specials(&assets, &mut tour_fight(&assets, None, style), 4_800);
            let kept = specials(&assets, &mut tour_fight(&assets, Some(footsies), style), 4_800);
            assert!(open.motions + open.chords + open.routes > 0, "{style:?}: without the mission it presses for specials: {open:?}");
            assert_eq!((kept.motions, kept.chords, kept.routes), (0, 0, 0), "{style:?}: under the mission: {kept:?}");
            assert!(kept.buttons > 60, "{style:?}: it still attacks: {kept:?}");
        }
    });
}

#[test]
fn another_world_tour_mission_leaves_the_stronger_play_its_specials() {
    // Missions that restrict no move (a rival bout, the trial that turns
    // off Player 1's guard) leave the stronger play its specials, era
    // commands and routes.
    let Some(assets) = assets() else { return };
    deep(move || {
        for mission in [MissionKind::RivalBout, MissionKind::RuleModifierTrial(RuleModifier::NoBlocking)] {
            let found = specials(&assets, &mut tour_fight(&assets, Some(mission), Revision::Sf6), 4_800);
            assert!(found.motions + found.chords + found.routes > 0, "{mission:?}: {found:?}");
        }
    });
}

/// Player 1 stands and kicks the air, so the Fair CPU has whiffs to
/// punish as it comes in: a heavy kick every 40 frames.
fn whiff_script(i: u32) -> PlayerInputs {
    inputs(if i % 40 < 4 { 0x200 } else { 0 })
}

#[test]
fn under_pure_footsies_a_fighter_whose_book_has_plain_normal_routes_takes_only_those() {
    // SC23 (`#215`): Ryu in the CE era style (no art pack needed) has a
    // plain-normal ground route, light punch twice, beside routes with
    // specials; he takes routes when he punishes a whiff. Under the trial
    // every route he takes is plain normals, and he takes one; without it
    // he takes routes with specials too.
    let Some(assets) = assets() else { return };
    deep(move || {
        let book = sf_engine::route_book::book(TournamentFighter::Ryu, Revision::Ce, loadouts(Revision::Ce)[0]).expect("Ryu's CE book");
        assert!(book.routes.iter().any(|r| r.normals_only()) && book.routes.iter().any(|r| !r.normals_only()));
        let footsies = MissionKind::RuleModifierTrial(RuleModifier::PureFootsies);
        for mission in [None, Some(footsies)] {
            let mut fight = tour_fight(&assets, mission, Revision::Ce);
            let (mut plain, mut other, mut last) = (0, 0, None);
            for i in 0..9_600 {
                fight.step(&assets, whiff_script(i), PlayerInputs::default(), false);
                let Some(cpu) = fight.machine().fair_cpu() else { continue };
                let entered = cpu.route_index().filter(|_| cpu.route_step().is_none_or(|step| step == 0));
                if entered.is_some() && entered != last {
                    let route = &book.routes[entered.unwrap()];
                    if route.normals_only() { plain += 1 } else { other += 1 }
                }
                last = cpu.route_index();
            }
            match mission {
                None => assert!(other > 0, "without the mission: plain {plain}, with specials {other}"),
                Some(_) => assert!(plain > 0 && other == 0, "under the trial: plain {plain}, with specials {other}"),
            }
        }
    });
}
