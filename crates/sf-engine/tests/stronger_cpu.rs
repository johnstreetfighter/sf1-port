//! Ticket #194 (`SC2`, spec #192): the developer-only switch for the
//! stronger Fair CPU, the situations it judges and its first game plan.
//!
//! At the headless session's seam, driven by inputs: Training against the
//! CPU, Ryu against the Fair CPU's Ryu, both in the SF6 era style, at Very
//! Difficult. Player 1's pad is scripted here; what is checked is what the
//! frames show and the stick and buttons the machine took on Player 2's
//! side. Local ROMs only; each test skips without them.

use std::collections::HashMap;
use std::path::Path;

use sf_engine::config::MatchConfig;
use sf_engine::era::{listed_families, listed_revisions, loadouts, EraFamily, Loadout, Revision};
use sf_engine::fair_cpu::{perceive, Judged, Level, Percept, Situation};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::fight::FightDips;
use sf_engine::front::menu::MenuItem;
use sf_engine::front::{FrameInputs, GameSession, PlayerInputs, Transition};
use sf_engine::roster::TournamentFighter;
use sf_engine::run::PowerOnMachine;
use sf_video::callouts::Side;
use sf_video::romset::load_assets;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const FIGHT_LOOP: u32 = 0x3eafa;
const R: u8 = 1;
const L: u8 = 2;
const D: u8 = 4;
const U: u8 = 8;
const LP: u8 = 1;
const MP: u8 = 2;
const HP: u8 = 4;
const MK: u8 = 16;
const HK: u8 = 32;

/// Player 1's pad: direction bits and button bits.
fn p1(dirs: u8, buttons: u8) -> FrameInputs {
    FrameInputs {
        p1: PlayerInputs {
            directions: [R, L, D, U].map(|bit| dirs & bit != 0),
            buttons: std::array::from_fn(|i| buttons & (1 << i) != 0),
        },
        ..FrameInputs::default()
    }
}

fn tap(s: &mut GameSession, input: FrameInputs) -> Option<Transition> {
    let t = s.step(input).transition;
    s.step(FrameInputs::default());
    t
}

fn choose(s: &mut GameSession, item: MenuItem) {
    for _ in 0..8 {
        if s.menu_item() == Some(item) {
            tap(s, p1(0, LP));
            return;
        }
        tap(s, p1(D, 0));
    }
    panic!("{item:?} missing from {}", s.screen());
}

/// Lock the fighter under the cursor in the SF6 era style, on the Style card.
fn lock_sf6(s: &mut GameSession, fighter: TournamentFighter) {
    assert_eq!(tap(s, p1(0, LP)), Some(Transition::StyleOpened), "the Style card opens");
    let order = [
        EraFamily::Sf1Plus,
        EraFamily::Sf2,
        EraFamily::SfZ,
        EraFamily::Sf3,
        EraFamily::Sf4,
        EraFamily::Sf5,
        EraFamily::Sf6,
        EraFamily::Sf1987,
    ];
    let available = listed_families(fighter, true);
    let steps = order.into_iter().filter(|f| available.contains(f)).position(|f| f == EraFamily::Sf6).expect("SF6 is listed");
    for _ in 0..steps {
        tap(s, p1(R, 0));
    }
    assert_eq!(listed_revisions(fighter, EraFamily::Sf6, true), [Revision::Sf6]);
    assert_eq!(loadouts(Revision::Sf6)[0], Loadout::Standard);
    tap(s, p1(0, LP));
}

/// Training against the CPU: Ryu against the Fair CPU's Ryu, both SF6, at
/// `difficulty`, with the stronger Fair CPU's switch as given, from the Main
/// Menu to the first frame of the fight loop.
fn fight(stronger: bool, difficulty: u8) -> Option<GameSession> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    let assets = load_assets(ROMS, MANIFEST).expect("local ROM set assembles");
    let mut config = MatchConfig::definitive();
    config.roster.what_if_styles = true;
    let mut s = GameSession::new(assets, config).with_stronger_cpu(stronger);
    s.set_fight_dips(FightDips { difficulty, round_time: 100 });
    for _ in 0..200 {
        s.step(FrameInputs::default());
    }
    let mut start = FrameInputs::default();
    start.menu.start = true;
    tap(&mut s, start);
    choose(&mut s, MenuItem::SinglePlayer);
    choose(&mut s, MenuItem::Training);
    choose(&mut s, MenuItem::TrainingCpu);
    lock_sf6(&mut s, TournamentFighter::Ryu);
    // The opponent's cursor starts on Ken, one to the right of Ryu.
    tap(&mut s, p1(L, 0));
    lock_sf6(&mut s, TournamentFighter::Ryu);
    let mut f = 0;
    while s.fight_machine().map(flow_position) != Some(FIGHT_LOOP) {
        match s.screen() {
            "stage select" | "country select" => {
                tap(&mut s, p1(0, LP));
            }
            _ => {
                s.step(FrameInputs::default());
            }
        }
        f += 1;
        assert!(f < 3_000, "the fight loop (at {})", s.screen());
    }
    let fight = s.fight().unwrap();
    assert_eq!([fight.player_fighter(0), fight.player_fighter(1)], [TournamentFighter::Ryu; 2]);
    let machine = s.fight_machine().unwrap();
    assert_eq!(machine.era_fighters().map(|e| e.style.revision), [Revision::Sf6; 2]);
    assert_eq!(machine.match_config().stronger_cpu, stronger);
    assert_eq!(Level::from_difficulty(machine.match_config().difficulty).reaction(), Level::from_difficulty(difficulty).reaction());
    Some(s)
}

/// The fight's machine, lifted out of the session at the fight loop: the
/// tests step it bare, with Player 1's pad set here and Player 2's by the
/// Fair CPU inside it.
fn machine(stronger: bool) -> Option<PowerOnMachine> {
    Some(fight(stronger, 4)?.fight_machine().unwrap().clone())
}

/// One frame with Player 1's stick and buttons as given.
fn step(m: &mut PowerOnMachine, dirs: u8, buttons: u8) {
    m.set_p1_direction(dirs & R != 0, dirs & L != 0, dirs & D != 0, dirs & U != 0);
    m.set_p1_buttons(std::array::from_fn(|i| buttons & (1 << i) != 0));
    m.step();
}

/// The frame as Player 2's Fair CPU is shown it: `me` is the CPU's fighter.
fn seen(m: &PowerOnMachine) -> Percept {
    perceive(m.state.frame, true, m.work_ram(), &m.state.objectram, Side::P2)
}

/// From the CPU fighter's centre to the near edge of Player 1's hurtbox.
fn gap(v: &Percept) -> i16 {
    if v.foe.x >= v.me.x { v.foe.hurt.0 - v.me.x } else { v.me.x - v.foe.hurt.1 }
}

/// Player 1's stick toward the CPU's fighter, and away from him.
fn toward(v: &Percept) -> u8 {
    if v.me.x >= v.foe.x { R } else { L }
}

fn away(v: &Percept) -> u8 {
    R + L - toward(v)
}

/// The Fair CPU's pad on the last frame: direction bits and button bits.
fn cpu_pad(m: &PowerOnMachine) -> (u8, u8) {
    let (dirs, buttons) = m.fair_cpu_pad().expect("the Fair CPU plays Player 2");
    let bits = |held: &[bool]| held.iter().enumerate().fold(0u8, |acc, (i, &h)| acc | (u8::from(h) << i));
    (bits(&dirs), bits(&buttons))
}

/// Knocked down or rising, as a frame shows it: a hit reaction drawn off
/// the ground, or the rise from a throw.
fn knocked_down(move_id: u8, state: u8) -> bool {
    move_id == 0x28 || (move_id >= 0x30 && state & 1 != 0)
}

#[test]
fn with_the_switch_off_the_cpu_judges_no_situation_and_follows_no_game_plan() {
    let Some(mut m) = machine(false) else { return };
    for i in 0..1_500u32 {
        let v = seen(&m);
        step(&mut m, if i < 600 { toward(&v) } else { 0 }, 0);
        assert_eq!(m.fair_cpu_situation(), Some(Judged::default()), "frame {i}");
        assert!(!m.fair_cpu_following_game_plan(), "frame {i}");
    }
}

/// Player 1's pad for the situations fight: a few hundred frames each of
/// walking in, standing, guarding low, sweeping, whiffing heavy punches,
/// jumping in, throwing projectiles and backing into the corner.
fn sparring_pad(i: u32, v: &Percept) -> (u8, u8) {
    let t = i % 300;
    match (i / 300) % 8 {
        0 => (toward(v), 0),
        1 => (0, 0),
        2 => (D | away(v), 0),
        3 => if t % 40 < 3 { (D, HK) } else { (toward(v), 0) },
        4 => if t % 45 < 3 { (0, HP) } else { (0, 0) },
        5 => match t % 70 {
            0..=2 => (U | toward(v), 0),
            24..=26 => (0, HK),
            _ => (0, 0),
        },
        6 => match t % 70 {
            0..=2 => (D, 0),
            3..=5 => (D | toward(v), 0),
            6..=9 => (toward(v), HP),
            _ => (0, 0),
        },
        _ => (away(v), 0),
    }
}

#[test]
fn the_nine_situations_are_judged_from_the_delayed_view_only() {
    let Some(mut m) = machine(true) else { return };
    let delay = Level::VeryHard.reaction();
    let mut shown: HashMap<u32, Percept> = HashMap::new();
    let mut met: HashMap<Situation, u32> = HashMap::new();
    let (mut cornered, mut open) = (0u32, 0u32);
    for i in 0..9_600u32 {
        let v = seen(&m);
        let (dirs, buttons) = sparring_pad(i, &v);
        step(&mut m, dirs, buttons);
        let frame = m.state.frame;
        shown.insert(frame, seen(&m));
        let judged = m.fair_cpu_situation().unwrap();
        // The pad for frame `frame` answered the frame `delay` earlier.
        let Some(view) = shown.get(&(frame - delay)).copied().filter(|v| v.fighting) else { continue };
        let (me, foe) = (view.me, view.foe);
        let why = format!("frame {frame}: {judged:?} from {view:?}");
        match judged.situation {
            Situation::KnockedDown => assert!(knocked_down(me.move_id, me.state), "{why}"),
            Situation::OpponentKnockedDown => assert!(knocked_down(foe.move_id, foe.state), "{why}"),
            Situation::ItHit => assert!(foe.reacting(), "{why}"),
            Situation::Defending => assert!(foe.attacking() || foe.jumping() || view.shot || me.reacting(), "{why}"),
            Situation::OpponentWhiffed => assert!(foe.attacking() && !foe.active, "{why}"),
            Situation::ItWasBlocked => {
                let guarded = (0..=14).filter_map(|back| shown.get(&(frame - delay - back))).any(|v| v.foe.guarding() && v.me.attacking());
                assert!(guarded, "{why}");
            }
            Situation::NeutralFar | Situation::NeutralClose => {
                assert!(!foe.attacking() && !foe.jumping() && !foe.reacting() && !me.reacting(), "{why}");
                if judged.situation == Situation::NeutralFar {
                    assert!(gap(&view) > 56, "{why}");
                } else {
                    assert!(gap(&view) <= 128, "{why}");
                }
            }
        }
        let near_wall = |x: i16| !(165..=355).contains(&x);
        if judged.corner {
            assert!(near_wall(me.x) || near_wall(foe.x), "{why}");
            cornered += 1;
        } else {
            open += 1;
        }
        *met.entry(judged.situation).or_default() += 1;
    }
    for situation in [
        Situation::NeutralFar,
        Situation::NeutralClose,
        Situation::ItHit,
        Situation::ItWasBlocked,
        Situation::OpponentKnockedDown,
        Situation::OpponentWhiffed,
        Situation::Defending,
        Situation::KnockedDown,
    ] {
        assert!(met.get(&situation).is_some_and(|&frames| frames >= 5), "{situation:?} was judged: {met:?}");
    }
    assert!(cornered >= 5 && open >= 5, "the corner was judged both ways: {cornered} and {open}");
}

/// How Player 1 plays while the CPU pokes.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Partner {
    /// Stands in reach and never guards.
    Still,
    /// Holds a crouching guard.
    Guarding,
    /// Hovers at the edge of reach and backs off when the CPU comes in.
    BackingOff,
}

/// What a poke met, as the engine has it, and what the CPU did next while
/// it went on with the same game plan.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Met {
    Hit,
    Guard,
    Whiff,
}

#[derive(Clone, Copy, Debug)]
struct Poke {
    met: Met,
    /// Frames the game plan went on after the poke's input.
    followed: u32,
    threw_projectile: bool,
    /// A punch button pressed while the game plan went on: the projectile's.
    punched: bool,
    /// A heavy button with the stick held sideways: the throw's input.
    heavy_with_direction: bool,
    /// Frames of the stick held straight away from Player 1, nothing else.
    gave_ground: u32,
    /// Frames of a guard held: away, or down and away.
    guarded: u32,
}

/// Every crouching medium kick the CPU pressed inside a game plan over
/// `frames`, against `partner`.
fn pokes(m: &mut PowerOnMachine, partner: Partner, frames: u32) -> Vec<Poke> {
    let mut out: Vec<Poke> = Vec::new();
    // The poke being followed: its record, frames since its input, and
    // whether the game plan is still going.
    let mut live: Option<(Poke, u32, bool)> = None;
    let mut pressed = false;
    for _ in 0..frames {
        let v = seen(m);
        let following = m.fair_cpu_following_game_plan();
        let pad = match partner {
            Partner::Still => (0, 0),
            Partner::Guarding => (D | away(&v), 0),
            Partner::BackingOff if following || gap(&v) < 58 => (away(&v), 0),
            Partner::BackingOff if gap(&v) > 70 => (toward(&v), 0),
            Partner::BackingOff => (0, 0),
        };
        step(m, pad.0, pad.1);
        let v = seen(m);
        let (dirs, buttons) = cpu_pad(m);
        let following = m.fair_cpu_following_game_plan();
        let press = following && dirs & D != 0 && buttons & MK != 0;
        if press && !pressed {
            if let Some((poke, _, _)) = live.take() {
                out.push(poke);
            }
            live = Some((
                Poke { met: Met::Whiff, followed: 0, threw_projectile: false, punched: false, heavy_with_direction: false, gave_ground: 0, guarded: 0 },
                0,
                true,
            ));
        } else if let Some((poke, since, going)) = live.as_mut() {
            *since += 1;
            let combo = &m.toggle_state().combo;
            if *since <= 40 && poke.met == Met::Whiff {
                if combo.hitstun[0] != 0 || v.foe.reacting() {
                    poke.met = Met::Hit;
                } else if combo.blockstun[0] != 0 {
                    poke.met = Met::Guard;
                }
            }
            *going &= following;
            if *going {
                let back = if v.me.x >= v.foe.x { R } else { L };
                poke.followed = *since;
                poke.punched |= buttons & (LP | MP | HP) != 0;
                poke.heavy_with_direction |= dirs & (R | L) != 0 && dirs & D == 0 && buttons & (HP | HK) != 0;
                poke.gave_ground += u32::from(dirs == back && buttons == 0);
                poke.guarded += u32::from(dirs & back != 0 && dirs & !(back | D) == 0 && buttons == 0);
            }
            // A projectile leaves the hand some frames after its input,
            // which can be the plan's last.
            if *since <= poke.followed + 25 {
                poke.threw_projectile |= v.my_shot;
            } else if !*going {
                out.push(live.take().unwrap().0);
            }
        }
        pressed = press;
    }
    out.extend(live.map(|(poke, _, _)| poke));
    out
}

#[test]
fn one_game_plan_takes_a_different_next_step_after_a_hit_a_guard_and_a_whiff() {
    let Some(start) = machine(true) else { return };
    let first = |partner: Partner, met: Met| {
        let mut m = start.clone();
        let all = pokes(&mut m, partner, 6_000);
        all.iter().copied().find(|p| p.met == met).unwrap_or_else(|| panic!("no poke {met:?} against {partner:?}: {all:?}"))
    };
    // A hit it sees: a projectile, to keep him pinned.
    let hit = first(Partner::Still, Met::Hit);
    assert!(hit.punched && hit.threw_projectile, "{hit:?}");
    // A guard: it walks in and throws.
    let guard = first(Partner::Guarding, Met::Guard);
    assert!(guard.heavy_with_direction && !guard.threw_projectile, "{guard:?}");
    // A whiff: it gives ground and guards.
    let whiff = first(Partner::BackingOff, Met::Whiff);
    assert!(whiff.gave_ground >= 8 && whiff.guarded > whiff.gave_ground, "{whiff:?}");
    assert!(!whiff.threw_projectile && !whiff.heavy_with_direction, "{whiff:?}");
    // Each next step came several actions into the same plan, after the
    // CPU could have seen what the poke met.
    for poke in [hit, guard, whiff] {
        assert!(poke.followed > Level::VeryHard.reaction() + 6, "{poke:?}");
    }
}

#[test]
fn a_threat_the_cpu_answers_interrupts_its_game_plan() {
    let Some(start) = machine(true) else { return };
    let delay = Level::VeryHard.reaction();
    // Sixty frames from `m`, Player 1 pressing a standing heavy punch at
    // once or doing nothing: the frame his attack first shows, the frame
    // the game plan stops being followed, and the frames the CPU then held
    // its stick away from him with no button.
    let play = |m: &PowerOnMachine, attack: bool| {
        let mut m = m.clone();
        let from = m.state.frame;
        let (mut shown, mut ended, mut guard_frames) = (None, None, 0u32);
        for _ in 0..60 {
            let pressing = attack && m.state.frame < from + 3;
            step(&mut m, 0, if pressing { HP } else { 0 });
            let v = seen(&m);
            if shown.is_none() && v.foe.attacking() {
                shown = Some(m.state.frame);
            }
            if ended.is_none() && !m.fair_cpu_following_game_plan() {
                ended = Some(m.state.frame);
            }
            let (dirs, buttons) = cpu_pad(&m);
            let back = if v.me.x >= v.foe.x { R } else { L };
            if ended.is_some() && !v.me.reacting() {
                guard_frames += u32::from(dirs & back != 0 && buttons == 0);
            }
        }
        (shown, ended.unwrap_or(u32::MAX), guard_frames)
    };
    let mut interrupted = 0;
    // The attack comes a different number of frames into the plan each try.
    for wait in 0..16u32 {
        let mut m = start.clone();
        let mut began = false;
        for _ in 0..3_000 {
            let v = seen(&m);
            step(&mut m, if gap(&v) > 66 { toward(&v) } else { 0 }, 0);
            if m.fair_cpu_following_game_plan() {
                began = true;
                break;
            }
        }
        assert!(began, "a game plan began");
        for _ in 0..wait {
            step(&mut m, 0, 0);
        }
        if !m.fair_cpu_following_game_plan() {
            continue;
        }
        let (_, left_alone, _) = play(&m, false);
        let (shown, ended, guard_frames) = play(&m, true);
        let Some(shown) = shown else { continue };
        // Never sooner than the reaction delay after the attack showed.
        assert!(ended >= shown + delay || ended == left_alone, "the plan ended at {ended}, the attack showed at {shown}");
        // Cut short as the attack is first seen, where the same plan left
        // alone went on, and a guard held after it: the threat answered.
        if ended <= shown + delay + 3 && left_alone > ended + 3 && guard_frames >= 5 {
            interrupted += 1;
        }
    }
    assert!(interrupted >= 2, "an attack seen during a game plan cut it short and was guarded in {interrupted} of 16 tries");
}

#[test]
fn a_fight_restored_from_a_snapshot_mid_plan_continues_identically() {
    let Some(mut m) = machine(true) else { return };
    // Play until the CPU is in the middle of a game plan.
    let mut found = false;
    for _ in 0..3_000 {
        let v = seen(&m);
        step(&mut m, if gap(&v) > 66 { toward(&v) } else { 0 }, 0);
        if m.fair_cpu_following_game_plan() {
            found = true;
            break;
        }
    }
    assert!(found, "a game plan began");
    let snap = m.save_snapshot();
    let inputs: Vec<(u8, u8)> = (0..1_200u32)
        .map(|f| {
            let r = f.wrapping_mul(2_654_435_761) >> 7;
            let dirs = if f % 50 < 30 { (r & 1) as u8 * R } else { (r & 2) as u8 / 2 * L } | if f % 90 > 70 { D } else { 0 };
            (dirs, if f % 37 == 5 { 1 << (f % 6) } else { 0 })
        })
        .collect();
    let run = |m: &mut PowerOnMachine| {
        let mut record = Vec::new();
        for &(dirs, buttons) in &inputs {
            step(m, dirs, buttons);
            record.push((cpu_pad(m), m.fair_cpu_situation(), m.fair_cpu_following_game_plan()));
        }
        record
    };
    let straight = run(&mut m);
    let end = m.save_snapshot();
    // Rollback's own path: a machine that has played something else
    // meanwhile takes the snapshot and the same inputs again.
    let mut other = m.clone();
    for _ in 0..300 {
        step(&mut other, R, 0);
    }
    other.restore_snapshot(&snap);
    let replayed = run(&mut other);
    assert_eq!(straight, replayed);
    assert!(end == other.save_snapshot(), "the two machines end in the same state");
    assert!(straight[0].2, "the snapshot was taken, and restored, in the middle of a game plan");
    let kinds: std::collections::HashSet<_> = straight.iter().map(|(_, judged, _)| judged.unwrap().situation).collect();
    assert!(kinds.len() >= 3, "several situations were judged along the way: {kinds:?}");
}
