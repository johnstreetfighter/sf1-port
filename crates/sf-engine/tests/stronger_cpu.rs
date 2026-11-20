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
use sf_engine::fair_cpu::{perceive, Attention, Body, Focus, Judged, Level, Percept, Situation};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::fight::FightDips;
use sf_engine::front::menu::MenuItem;
use sf_engine::front::{FrameInputs, GameSession, PlayerInputs, Transition};
use sf_engine::roster::TournamentFighter;
use sf_engine::route_book::{book, Route, RouteBook, Stance};
use sf_engine::run::PowerOnMachine;
use sf_video::callouts::Side;
use sf_video::art::{load_art_from, ArtSources};
use sf_video::art_pack::ArtPack;
use sf_video::romset::load_assets;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const PACK: &str = "../../assets/sf1-port-art";
const RECIPES: [&str; 11] = [
    include_str!("../../../art/recipes/ryu.json"),
    include_str!("../../../art/recipes/retsu.json"),
    include_str!("../../../art/recipes/geki.json"),
    include_str!("../../../art/recipes/joe.json"),
    include_str!("../../../art/recipes/mike.json"),
    include_str!("../../../art/recipes/lee.json"),
    include_str!("../../../art/recipes/gen.json"),
    include_str!("../../../art/recipes/birdie.json"),
    include_str!("../../../art/recipes/eagle.json"),
    include_str!("../../../art/recipes/adon.json"),
    include_str!("../../../art/recipes/sagat.json"),
];
const REVIEWS: [&str; 11] = [
    include_str!("../../../art/review/ryu.json"),
    include_str!("../../../art/review/retsu.json"),
    include_str!("../../../art/review/geki.json"),
    include_str!("../../../art/review/joe.json"),
    include_str!("../../../art/review/mike.json"),
    include_str!("../../../art/review/lee.json"),
    include_str!("../../../art/review/gen.json"),
    include_str!("../../../art/review/birdie.json"),
    include_str!("../../../art/review/eagle.json"),
    include_str!("../../../art/review/adon.json"),
    include_str!("../../../art/review/sagat.json"),
];
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
/// Ryu's Shoryuken, as its move id shows it.
const SHORYUKEN: u8 = 0x19;
/// The SF6 era style's Drive moves and Super Arts, as their move ids show
/// them (SC8, `#200`).
const DRIVE_RUSH: u8 = 0x2c;
const DRIVE_PARRY: u8 = 0x2d;
const DRIVE_REVERSAL: u8 = 0x2e;
const DRIVE_IMPACT: u8 = 0x2f;
const SUPER_ARTS: std::ops::RangeInclusive<u8> = 0x29..=0x2b;
/// The reach of Ryu's SF6 standing medium punch, from his centre, a
/// little over: what a Drive Rush into it must close.
const RUSHED_NORMAL_REACH: i16 = 50;
/// Ryu's SF6 Super meter, full (three bars of 48).
const FULL_SUPER_METER: u8 = 3 * 48;
/// Health left that Ryu's longest SF6 punish takes (640 twentieths).
const LONGEST_PUNISH: i16 = 32;
/// How near, centre to centre, the CPU's guard techs a throw.
const TECH_REACH: i16 = 46;

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
    fight_as(TournamentFighter::Ryu, stronger, difficulty)
}

/// The same with Player 1 playing `p1_fighter` in SF6.
fn fight_as(p1_fighter: TournamentFighter, stronger: bool, difficulty: u8) -> Option<GameSession> {
    if !Path::new(ROMS).is_dir() || !Path::new(PACK).is_dir() {
        eprintln!("skipping: local ROM set or art pack not present");
        return None;
    }
    let assets = load_assets(ROMS, MANIFEST).expect("local ROM set assembles");
    // The art pack gives the fighters' throw entries: without it no throw connects.
    let pack = ArtPack::open(Path::new(PACK)).expect("the art pack opens");
    let art = load_art_from(&assets, &ArtSources { recipes: &RECIPES, reviews: Some(&REVIEWS), pack: Ok(&pack) }).expect("the art loads");
    let mut config = MatchConfig::definitive();
    config.roster.what_if_styles = true;
    let mut s = GameSession::new(assets, config)
        .with_sprite_bank(&art.bank)
        .with_player_tables(&art.catalogues)
        .with_stronger_cpu(stronger);
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
    // The cast in its listed order on a grid six wide; the cursor does not
    // wrap at its edges.
    let at = TournamentFighter::ALL.iter().position(|&f| f == p1_fighter).unwrap();
    for _ in 0..at / 6 {
        tap(&mut s, p1(D, 0));
    }
    for _ in 0..at % 6 {
        tap(&mut s, p1(R, 0));
    }
    lock_sf6(&mut s, p1_fighter);
    // The opponent's cursor starts beside Player 1's pick: up and left to
    // the grid's corner, Ryu.
    for input in [U, U, L, L, L, L, L, L] {
        tap(&mut s, p1(input, 0));
    }
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
    assert_eq!([fight.player_fighter(0), fight.player_fighter(1)], [p1_fighter, TournamentFighter::Ryu]);
    let machine = s.fight_machine().unwrap();
    assert_eq!(machine.era_fighters().map(|e| e.style.revision), [Revision::Sf6; 2]);
    assert_eq!(machine.match_config().stronger_cpu, stronger);
    assert_eq!(Level::from_difficulty(machine.match_config().difficulty).reaction(), Level::from_difficulty(difficulty).reaction());
    Some(s)
}

/// The local ROM set and art pack the fights need are here. A test that
/// finds them missing skips; one that finds them measures, and an empty
/// count then fails it.
fn roms_present() -> bool {
    Path::new(ROMS).is_dir() && Path::new(PACK).is_dir()
}

/// The fight's machine, lifted out of the session at the fight loop: the
/// tests step it bare, with Player 1's pad set here and Player 2's by the
/// Fair CPU inside it.
fn machine(stronger: bool) -> Option<PowerOnMachine> {
    machine_at(stronger, 4)
}

fn machine_at(stronger: bool, difficulty: u8) -> Option<PowerOnMachine> {
    Some(fight(stronger, difficulty)?.fight_machine().unwrap().clone())
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

/// The frame the CPU decided from on the step that ended on `frame`, as it
/// saw it: the frame its focused delay shows, with Player 1's pose, height,
/// attack box and projectile from the frame its view of his actions came
/// from (SC7, `#199`: a new action that is not its focus shows only at the
/// slower delay), moved with him to where he is.
fn as_seen(shown: &HashMap<u32, Percept>, frame: u32, attention: Attention) -> Option<Percept> {
    let mut v = *shown.get(&(frame - Level::VeryHard.reaction()))?;
    if attention.action_frame != v.frame {
        let was = shown.get(&attention.action_frame)?;
        let dx = v.foe.x - was.foe.x;
        v.foe = Body {
            x: v.foe.x,
            health: v.foe.health,
            present: v.foe.present,
            front: was.foe.front + dx,
            hurt: (was.foe.hurt.0 + dx, was.foe.hurt.1 + dx),
            ..was.foe
        };
        (v.shot, v.shot_x, v.shot_half) = (was.shot, was.shot_x, was.shot_half);
    }
    Some(v)
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
    let (fast, slow) = (Level::VeryHard.reaction(), Level::VeryHard.unfocused_reaction());
    let mut met: HashMap<Situation, u32> = HashMap::new();
    let (mut cornered, mut open) = (0u32, 0u32);
    // Over a few fights, so that every situation turning up does not rest
    // on one run of the CPU's random choices.
    let fights = starts(true);
    if fights.is_empty() {
        return;
    }
    for mut m in fights {
        let mut shown: HashMap<u32, Percept> = HashMap::new();
        let mut cpu_saw: HashMap<u32, Percept> = HashMap::new();
        for i in 0..9_600u32 {
            let v = seen(&m);
            let (dirs, buttons) = sparring_pad(i, &v);
            step(&mut m, dirs, buttons);
            let frame = m.state.frame;
            shown.insert(frame, seen(&m));
            let judged = m.fair_cpu_situation().unwrap();
            // The pad for frame `frame` answered the frame its focused delay
            // earlier, with Player 1's new actions as old as its delays allow.
            let attention = m.fair_cpu_attention().unwrap();
            let Some(view) = as_seen(&shown, frame, attention).filter(|v| v.fighting) else { continue };
            assert!((frame - slow..=frame - fast).contains(&attention.action_frame), "frame {frame}: {attention:?}");
            cpu_saw.insert(frame, view);
            let (me, foe) = (view.me, view.foe);
            let why = format!("frame {frame}: {judged:?} from {view:?}");
            match judged.situation {
                Situation::KnockedDown => assert!(knocked_down(me.move_id, me.state), "{why}"),
                Situation::OpponentKnockedDown => assert!(knocked_down(foe.move_id, foe.state), "{why}"),
                Situation::ItHit => assert!(foe.reacting(), "{why}"),
                Situation::Defending => assert!(foe.attacking() || foe.jumping() || view.shot || me.reacting(), "{why}"),
                Situation::OpponentWhiffed => assert!(foe.attacking() && !foe.active, "{why}"),
                Situation::ItWasBlocked => {
                    let guarded = (0..=14).filter_map(|back| cpu_saw.get(&(frame - back))).any(|v| v.foe.guarding() && v.me.attacking());
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
        assert!(met.get(&situation).is_some_and(|&frames| frames >= 1), "{situation:?} was judged: {met:?}");
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
    /// A punch button pressed while the game plan went on, and a punch
    /// pressed with the stick forward within a few frames of the stick held
    /// down: a projectile's input, whether or not the motion came out.
    punched: bool,
    projectile_input: bool,
    /// Frames since the stick was last held down with no button.
    since_down: u32,
    /// The same crouching medium kick pressed again later in the plan.
    again: bool,
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
    // The game plan the poke was pressed in (plans can follow back to back).
    let mut plan_began: Option<u32> = None;
    let mut rising = 0;
    for _ in 0..frames {
        let v = seen(m);
        let following = m.fair_cpu_following_game_plan();
        let pad = match partner {
            Partner::Still => rising_guard(&mut rising, &v).unwrap_or((0, 0)),
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
        if press && !pressed && live.as_ref().is_some_and(|(_, _, going)| *going) {
            // Another press inside the same plan.
            live.as_mut().unwrap().0.again = true;
        } else if press && !pressed {
            if let Some((poke, _, _)) = live.take() {
                out.push(poke);
            }
            plan_began = m.fair_cpu_game_plan_began();
            live = Some((
                Poke { met: Met::Whiff, followed: 0, threw_projectile: false, punched: false, projectile_input: false, since_down: 99, again: false, heavy_with_direction: false, gave_ground: 0, guarded: 0 },
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
            *going &= following && !m.fair_cpu_entering_route() && m.fair_cpu_game_plan_began() == plan_began;
            if *going {
                let back = if v.me.x >= v.foe.x { R } else { L };
                poke.followed = *since;
                poke.punched |= buttons & (LP | MP | HP) != 0;
                let forward = if v.me.x >= v.foe.x { L } else { R };
                poke.projectile_input |= buttons & (LP | MP | HP) != 0 && dirs & forward != 0 && poke.since_down <= 8;
                poke.since_down = if dirs & D != 0 && buttons == 0 { 0 } else { poke.since_down + 1 };
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
    let fights = starts(true);
    if fights.is_empty() {
        return;
    }
    // Every poke that met `met` against `partner`, over a few fights, and
    // one that shows the step taken. Not every poke does: execution can
    // slip, and the CPU goes by what it sees, so a whiff into an opponent
    // walking back, drawn in his guard pose, is a guard to it.
    let find = |partner: Partner, met: Met, shows: &dyn Fn(&Poke) -> bool| {
        let all: Vec<Poke> = fights.iter().flat_map(|m| pokes(&mut m.clone(), partner, 9_000)).collect();
        let met_it: Vec<Poke> = all.iter().copied().filter(|p| p.met == met).collect();
        assert!(!met_it.is_empty(), "no poke {met:?} against {partner:?}: {all:?}");
        let shown = met_it.iter().copied().find(|p| shows(p)).unwrap_or_else(|| panic!("{met:?}: {met_it:?}"));
        shown
    };
    // A hit it sees (the kick knocks down): it follows him in and presses
    // the kick again as he rises.
    let hit = find(Partner::Still, Met::Hit, &|p| p.again && !p.heavy_with_direction && p.gave_ground == 0);
    // A guard: it walks in and throws.
    let guard = find(Partner::Guarding, Met::Guard, &|p| p.heavy_with_direction && !p.projectile_input);
    // A whiff: it gives ground and guards.
    let whiff = find(Partner::BackingOff, Met::Whiff, &|p| {
        p.gave_ground >= 8 && p.guarded > p.gave_ground && !p.projectile_input && !p.heavy_with_direction
    });
    // Each next step came several actions into the same plan, after the
    // CPU could have seen what the poke met.
    for poke in [hit, guard, whiff] {
        assert!(poke.followed > Level::VeryHard.reaction() + 6, "{poke:?}");
    }
}

#[test]
fn a_threat_the_cpu_answers_interrupts_its_game_plan() {
    if !roms_present() {
        return;
    }
    let (fast, slow) = (Level::VeryHard.reaction(), Level::VeryHard.unfocused_reaction());
    // Sixty frames from `m`, Player 1 pressing a standing heavy punch at
    // once or doing nothing: the frame his attack first shows and the delay
    // that applies to it (the focused one if the CPU was watching for a
    // button as the attack came into its focused delay, else the slower
    // one), the earliest it may answer at all (sooner where his own step
    // into the attack shows at the focused delay), the frame the game plan
    // stops being followed, and the frames the CPU then guarded or parried.
    let play = |m: &PowerOnMachine, attack: bool| {
        let mut m = m.clone();
        let from = m.state.frame;
        let (mut shown, mut ended, mut guard_frames) = (None, None, 0u32);
        let mut delay = slow;
        let mut from_down = false;
        // Where he stands, and the frame he is first seen to step anywhere
        // else (not pushed: not in a hit reaction or a guard).
        let x0 = seen(&m).foe.x;
        let mut moved = None;
        for _ in 0..60 {
            let pressing = attack && m.state.frame < from + 3;
            let before = seen(&m);
            step(&mut m, 0, if pressing { HP } else { 0 });
            let v = seen(&m);
            if shown.is_none() && v.foe.attacking() {
                shown = Some(m.state.frame);
                from_down = down(&before.foe);
            }
            // Watched: a button, or his wake-up when the attack began
            // straight from a knockdown (SC6 sets that focus over him).
            let focus = m.fair_cpu_attention().unwrap().focus;
            let watching = focus == Focus::Attack || (focus == Focus::Reversal && from_down);
            // Watched as it came into its focused delay, or watched for at
            // any step after that before its slower delay: it can show then,
            // never sooner than the focused delay.
            if shown.is_some_and(|shown| (shown + fast..shown + slow).contains(&(m.state.frame + 1))) && watching {
                delay = delay.min((m.state.frame + 1).saturating_sub(shown.unwrap()).max(fast));
            }
            if ended.is_none() && !m.fair_cpu_following_game_plan() {
                ended = Some(m.state.frame);
            }
            let pushed = v.foe.reacting() || v.foe.guarding() || m.toggle_state().combo.blockstun[0] != 0;
            if moved.is_none() && shown.is_some() && v.foe.x != x0 && !pushed {
                moved = Some(m.state.frame);
            }
            let (dirs, buttons) = cpu_pad(&m);
            let back = if v.me.x >= v.foe.x { R } else { L };
            // A guard (the stick held away, no button) or the era style's
            // Drive Parry (medium punch and kick together): an answer.
            if ended.is_some() && !v.me.reacting() {
                guard_frames += u32::from((dirs & back != 0 && buttons == 0) || buttons == MP | MK);
            }
        }
        // Where he stands shows at the focused delay (SC7's two-frame rule),
        // so an attack that steps him forward can be answered that soon
        // after the step shows (SC8: his standing heavy punch steps on its
        // first frame).
        let earliest = match (shown, moved) {
            (Some(shown), Some(moved)) => delay.min((moved + fast).saturating_sub(shown).max(fast)),
            _ => delay,
        };
        (shown.map(|shown| (shown, delay, earliest)), ended.unwrap_or(u32::MAX), guard_frames)
    };
    let mut interrupted = 0;
    let mut tries = 0;
    // Moments of a few fights where the CPU follows a game plan, Player 1
    // is on his feet and free to attack, and the plan left alone would go
    // on past the slower delay: sixteen of them, at least 8 frames apart.
    // (Against this CPU he is seldom free at such a moment.)
    for start in starts(true) {
        let mut m = start;
        let mut last_try = 0u32;
        let mut rising = 0;
        for _ in 0..30_000 {
            if tries == 16 {
                break;
            }
            let v = seen(&m);
            // In reach of his standing heavy punch (its box about 52 pixels
            // from his centre, after a 16-pixel step): an attack it must answer.
            let (dirs, buttons) = rising_guard(&mut rising, &v).unwrap_or((if gap(&v) > 60 { toward(&v) } else { 0 }, 0));
            step(&mut m, dirs, buttons);
            let v = seen(&m);
            let foe_down = m.fair_cpu_situation().is_some_and(|j| j.situation == Situation::OpponentKnockedDown);
            if !m.fair_cpu_following_game_plan() || foe_down || down(&v.foe) || !v.foe.free() || m.state.frame < last_try + 8 {
                continue;
            }
            let (_, left_alone, _) = play(&m, false);
            if left_alone <= m.state.frame + slow + 8 {
                continue;
            }
            last_try = m.state.frame;
            let (shown, ended, guard_frames) = play(&m, true);
            let Some((shown, delay, earliest)) = shown else { continue };
            tries += 1;
            // Never sooner than it could see the attack, or his step into it.
            assert!(ended >= shown + earliest || ended == left_alone, "the plan ended at {ended}, the attack showed at {shown}, earliest {earliest}");
            // Cut short as the attack is first seen, where the same plan left
            // alone went on, and a guard or a parry after it (the parry's input
            // is four frames): the threat answered.
            if ended <= shown + delay + 3 && left_alone > ended + 3 && guard_frames >= 3 {
                interrupted += 1;
            }
        }
    }
    assert!(tries >= 8, "moments tried: {tries}");
    assert!(interrupted >= 2, "an attack seen during a game plan cut it short and was guarded or parried in {interrupted} of {tries} tries");
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

/// One route the CPU entered, as the frames showed it.
#[derive(Clone, Debug, Default)]
struct RouteSeen {
    /// The CPU's pad each frame of it, its walk in included.
    pads: Vec<(u8, u8)>,
    /// The longest combo on Player 1 while it ran and just after.
    combo: u8,
    /// The CPU's Super meter when it began, and the least it fell to.
    meter_at_start: u8,
    meter_low: u8,
    /// Frames from the start of the route to its first hit.
    first_hit: Option<u32>,
    /// Player 1 had just whiffed when it began.
    after_whiff: bool,
    /// The CPU's fighter was on Player 1's left when it began.
    facing_right: bool,
    /// The CPU has begun an attack of its own since the route ended.
    closed: bool,
}

impl RouteSeen {
    /// Its pads as a route book writes them, for an attacker facing right.
    fn as_written(&self, facing_right: bool) -> Vec<(u8, u8)> {
        self.pads.iter().map(|&(d, b)| if facing_right { (d, b) } else { ((d & !3) | ((d & 1) << 1) | ((d & 2) >> 1), b) }).collect()
    }

    /// The book's route these pads enter, in full or cut short at one of
    /// its steps: the route and the steps entered.
    fn route<'a>(&self, book: &'a RouteBook) -> Option<(&'a Route, usize)> {
        let all = self.as_written(self.facing_right);
        // A run starts with its walk in: any number of frames forward alone.
        (0..=all.len())
            .take_while(|&lead| all[..lead].iter().all(|&p| p == (R, 0)))
            .find_map(|lead| Self::matching(&all[lead..], book))
    }

    fn matching<'a>(pads: &[(u8, u8)], book: &'a RouteBook) -> Option<(&'a Route, usize)> {
        // The pads after the last button are the route's tail of waiting.
        let end = pads.iter().rposition(|&(_, b)| b != 0)? + 1;
        for route in &book.routes {
            let script = route.script(None);
            for steps in (1..=route.steps.len()).rev() {
                let cut = route.step_start(steps);
                let last = script[..cut].iter().rposition(|&(_, b)| b != 0).map_or(0, |at| at + 1);
                if last == end && script[..end] == pads[..end] {
                    return Some((route, steps));
                }
            }
        }
        None
    }
}

/// Every route the CPU entered over `frames`, with Player 1's pad given by
/// `pad` each frame from what the frame shows.
fn routes_seen(m: &mut PowerOnMachine, frames: u32, mut pad: impl FnMut(u32, &Percept) -> (u8, u8)) -> Vec<RouteSeen> {
    let mut out: Vec<RouteSeen> = Vec::new();
    let mut live: Option<(RouteSeen, u32)> = None;
    let mut whiffed_at: Option<u32> = None;
    let mut p1_active = false;
    let mut p1_contact = false;
    let mut was_in_route = false;
    for i in 0..frames {
        let v = seen(m);
        let (dirs, buttons) = pad(i, &v);
        let cpu_health = v.me.health;
        let was_me = v.me;
        step(m, dirs, buttons);
        let v = seen(m);
        // Player 1's attack box went away without touching the CPU: a whiff.
        if v.foe.active {
            // A whiff as a person sees it: the box never reached the CPU.
            p1_active = true;
            let toward_cpu: i16 = if v.me.x >= v.foe.x { 1 } else { -1 };
            let near = if toward_cpu > 0 { v.me.hurt.0 } else { v.me.hurt.1 };
            p1_contact |= v.me.reacting() || v.me.health < cpu_health || (v.foe.front - near) * toward_cpu >= 0;
        } else if p1_active {
            if !p1_contact {
                whiffed_at = Some(i);
            }
            p1_active = false;
            p1_contact = false;
        }
        let combo = m.toggle_state().combo.hits[0];
        let in_route = m.fair_cpu_entering_route();
        let meter = m.era_fighters()[1].super_meter;
        // Another route begun before the last one's watch is over is a run
        // of its own.
        if in_route && !was_in_route && live.as_ref().is_some_and(|(run, _)| !run.pads.is_empty()) {
            out.push(live.take().unwrap().0);
        }
        was_in_route = in_route;
        if in_route && live.is_none() {
            let after_whiff = whiffed_at.is_some_and(|at| i - at <= 30);
            let facing_right = v.me.x < v.foe.x;
            live = Some((RouteSeen { meter_at_start: meter, meter_low: meter, after_whiff, facing_right, ..RouteSeen::default() }, i));
        }
        if let Some((run, began)) = live.as_mut() {
            let (dirs, buttons) = cpu_pad(m);
            if in_route {
                run.pads.push((dirs, buttons));
            } else if v.me.attacking() && !was_me.attacking() {
                // An attack of its own begun from a free fighter after the
                // route (not the route's own cancel): what it hits is not
                // the route's.
                run.closed = true;
            }
            if v.foe.reacting() && !run.closed {
                run.combo = run.combo.max(combo);
                run.first_hit.get_or_insert(i - *began);
            }
            if in_route {
                run.meter_low = run.meter_low.min(meter);
            }
            if !in_route && i - *began > 60 + run.pads.len() as u32 {
                out.push(live.take().unwrap().0);
            }
        }
    }
    out.extend(live.map(|(run, _)| run));
    out
}

/// Frames Player 1 keeps a low guard after rising from a knockdown.
const RISING_GUARD: u32 = 24;

/// Player 1 guarding low while he is knocked down and for a little after
/// he rises, as a person does: without it the CPU's attack timed to his
/// rise keeps a dummy that only stands down for most of a fight (SC8,
/// `#200`). `left` counts the guard's frames down.
fn rising_guard(left: &mut u32, v: &Percept) -> Option<(u8, u8)> {
    if down(&v.foe) {
        *left = RISING_GUARD;
    } else if *left == 0 {
        return None;
    } else {
        *left -= 1;
    }
    Some((D | away(v), 0))
}

/// Player 1 standing still in reach of the CPU's routes, guarding low as
/// he rises.
fn standing_in_reach() -> impl FnMut(u32, &Percept) -> (u8, u8) {
    let mut left = 0;
    move |_, v| rising_guard(&mut left, v).unwrap_or(if gap(v) > 70 { (toward(v), 0) } else { (0, 0) })
}

#[test]
fn a_hit_from_a_route_is_followed_by_the_routes_further_hits_and_the_routes_vary() {
    // Over a few fights, so that the variety does not rest on one run of
    // the CPU's random choices.
    let runs: Vec<RouteSeen> = starts(true).iter().flat_map(|m| routes_seen(&mut m.clone(), 6_000, standing_in_reach())).collect();
    if !roms_present() {
        return;
    }
    let combos: Vec<&RouteSeen> = runs.iter().filter(|r| r.combo >= 2).collect();
    assert!(combos.len() >= 4, "routes that combo: {} of {}: {runs:?}", combos.len(), runs.len());
    // More than one press in it: the further hits come from its later steps,
    // pressed before the first hit could be seen. (A route cut short by a
    // slip can still combo with what the CPU does next, which is not the
    // route's: only routes entered in full are checked.)
    let book = book(TournamentFighter::Ryu, Revision::Sf6, Loadout::Standard).unwrap();
    for run in combos.iter().filter(|r| r.route(book).is_some_and(|(route, steps)| steps == route.steps.len())) {
        let presses = run.pads.windows(2).filter(|w| w[0].1 == 0 && w[1].1 != 0).count() + usize::from(run.pads[0].1 != 0);
        assert!(presses >= 2 || run.combo >= 3, "{run:?}");
    }
    // Which of the book's routes each was, entered in full.
    let distinct: std::collections::HashSet<String> = combos
        .iter()
        .filter_map(|r| r.route(book).filter(|(route, steps)| *steps == route.steps.len()).map(|(route, _)| route.name()))
        .collect();
    assert!(distinct.len() >= 2, "more than one route over repeated openings: {distinct:?}");
}

#[test]
fn a_route_the_book_records_failing_on_the_opponent_is_never_taken_against_him() {
    // Ryu's SF6 book records his medium punch into a Hadoken failing on Lee
    // (SC13, `#205`: `route_check`). Against Lee standing in reach the CPU
    // still takes routes, never those. Summed over a few fights.
    let book = book(TournamentFighter::Ryu, Revision::Sf6, Loadout::Standard).unwrap();
    let lee = "Lee Sf6/Standard";
    let failing: Vec<String> = book.routes.iter().filter(|r| r.fails_against(lee, Stance::Standing)).map(Route::name).collect();
    assert!(!failing.is_empty(), "the book records routes failing on Lee");
    let Some(m) = fight_as(TournamentFighter::Lee, true, 4).map(|s| s.fight_machine().unwrap().clone()) else { return };
    let mut entered: Vec<String> = Vec::new();
    for idle in [0u32, 97, 211] {
        let mut m = m.clone();
        for _ in 0..idle {
            step(&mut m, 0, 0);
        }
        let runs = routes_seen(&mut m, 6_000, standing_in_reach());
        entered.extend(runs.iter().filter_map(|run| run.route(book)).map(|(route, _)| route.name()));
    }
    assert!(entered.len() >= 3, "routes taken against Lee: {entered:?}");
    assert!(entered.iter().all(|name| !failing.contains(name)), "taken: {entered:?}; recorded failing on Lee: {failing:?}");
}

#[test]
fn a_whiff_in_reach_is_punished_with_a_route_not_one_heavy_normal() {
    // Player 1 whiffs a Shoryuken, a move with a long recovery, whenever
    // the CPU is just out of its reach, and walks in when it is farther.
    // Summed over a few fights.
    let mut runs: Vec<RouteSeen> = Vec::new();
    for mut m in starts(true) {
        let mut started: Option<u32> = None;
        let pad = |i: u32, v: &Percept| {
            let f = toward(v);
            if let Some(at) = started {
                let motion = [(f, 0), (f, 0), (D, 0), (D, 0), (D | f, HP), (D | f, HP), (0, 0)];
                if let Some(&pad) = motion.get((i - at) as usize) {
                    return pad;
                }
                if i - at < 90 {
                    return (0, 0);
                }
                started = None;
            }
            let g = gap(v);
            if (60..=90).contains(&g) && v.foe.free() {
                started = Some(i);
                (f, 0)
            } else if g > 90 {
                (f, 0)
            } else {
                (0, 0)
            }
        };
        runs.extend(routes_seen(&mut m, 12_000, pad));
    }
    if !roms_present() {
        return;
    }
    let punishes: Vec<&RouteSeen> = runs.iter().filter(|r| r.after_whiff).collect();
    assert!(!punishes.is_empty(), "no route followed a whiff: {runs:?}");
    let landed: Vec<_> = punishes.iter().filter(|r| r.combo >= 2).collect();
    assert!(landed.len() >= 2, "punishes of two hits or more: {} of {}: {punishes:?}", landed.len(), punishes.len());
    // Every punish is one of the book's routes, entered in full or dropped
    // partway by a slip of execution: never a lone heavy normal. A route cut
    // by a hit while it still walks in pressed nothing to tell it by.
    let book = book(TournamentFighter::Ryu, Revision::Sf6, Loadout::Standard).unwrap();
    for run in punishes.iter().filter(|run| run.pads.iter().any(|&(_, buttons)| buttons != 0)) {
        assert!(run.route(book).is_some(), "a punish is a route: {run:?}");
    }
}

#[test]
fn a_route_is_taken_only_with_the_meter_it_spends() {
    // Summed over a few fights.
    let runs: Vec<RouteSeen> = starts(true).iter().flat_map(|m| routes_seen(&mut m.clone(), 9_000, standing_in_reach())).collect();
    if !roms_present() {
        return;
    }
    assert!(runs.len() >= 4, "{runs:?}");
    // A Super Art's least cost is one bar. A route begun with less never
    // spends any; one begun with a bar or more may.
    for run in runs.iter().filter(|r| r.meter_at_start < 48) {
        assert!(run.meter_low >= run.meter_at_start, "{run:?}");
    }
}

#[test]
fn at_easy_a_route_is_sometimes_dropped_partway() {
    // Summed over a few fights.
    let runs: Vec<RouteSeen> = starts_at(true, 1).iter().flat_map(|m| routes_seen(&mut m.clone(), 20_000, standing_in_reach())).collect();
    if !roms_present() {
        return;
    }
    let book = book(TournamentFighter::Ryu, Revision::Sf6, Loadout::Standard).unwrap();
    // A run that matches a route of the book only cut short at one of its
    // steps, and is no route of the book itself, is a dropped route.
    let whole: Vec<Vec<(u8, u8)>> = book.routes.iter().map(|r| r.script(None)).collect();
    let mut entered = 0;
    let mut dropped = 0;
    for run in &runs {
        let Some((route, steps)) = run.route(book) else { continue };
        entered += 1;
        let all = run.as_written(run.facing_right);
        let lead = all.iter().take_while(|&&p| p == (R, 0)).count();
        let pads = &all[lead..];
        let own = whole.iter().any(|script| script.iter().rposition(|&(_, b)| b != 0) == pads.iter().rposition(|&(_, b)| b != 0) && script.starts_with(&pads[..=pads.iter().rposition(|&(_, b)| b != 0).unwrap()]));
        if steps < route.steps.len() && !own {
            dropped += 1;
        }
    }
    assert!(entered >= 10, "routes entered at Easy: {entered} of {} runs", runs.len());
    assert!(dropped >= 1, "no dropped route among {entered} entered at Easy");
}

/// A new button on the CPU's pad this frame that was not on it the last.
fn cpu_pressed(m: &PowerOnMachine, last: &mut u8) -> bool {
    let (_, buttons) = cpu_pad(m);
    let new = buttons & !*last != 0;
    *last = buttons;
    new
}

/// The fight from a few starting points: as it begins, and after Player 1
/// has stood still a while, so a test's counts do not rest on one run of
/// the CPU's random choices (any change to its play reshuffles them).
fn starts(stronger: bool) -> Vec<PowerOnMachine> {
    starts_at(stronger, 4)
}

/// The same, at `difficulty`.
fn starts_at(stronger: bool, difficulty: u8) -> Vec<PowerOnMachine> {
    starts_from(stronger, difficulty, &[0, 97, 211])
}

/// The fight from the starting points `idle` frames of Player 1 standing
/// still in.
fn starts_from(stronger: bool, difficulty: u8, idle: &[u32]) -> Vec<PowerOnMachine> {
    let Some(m) = machine_at(stronger, difficulty) else { return Vec::new() };
    idle
        .iter()
        .map(|&idle| {
            let mut m = m.clone();
            for _ in 0..idle {
                step(&mut m, 0, 0);
            }
            m
        })
        .collect()
}

/// The CPU's fighter started an attack between `before` and `v` (not a
/// button alone: its guard taps the throw's buttons to tech one).
fn attack_started(before: &Percept, v: &Percept) -> bool {
    v.me.attacking() && (!before.me.attacking() || before.me.move_id != v.me.move_id)
}

#[test]
fn after_a_guarded_attack_the_cpu_keeps_attacking_and_its_gaps_catch_a_pressed_button() {
    // Player 1 guards low, and as soon as his guard is no longer locked he
    // presses a standing light punch, the way a player tries to take his turn.
    let (mut guards, mut pressured, mut punches, mut caught) = (0u32, 0u32, 0u32, 0u32);
    for mut m in starts(true) {
        let mut was_stunned = false;
        let mut since_guard: Option<u32> = None;
        let mut press_at: Option<u32> = None;
        let mut punch_out = false;
        for i in 0..12_000u32 {
            let before = seen(&m);
            let stunned = m.toggle_state().combo.blockstun[0] != 0;
            if was_stunned && !stunned {
                guards += 1;
                press_at = Some(i);
                since_guard = Some(i);
            }
            was_stunned = stunned;
            let pressing = press_at.is_some_and(|at| i - at < 3);
            let pad = if pressing { (0, LP) } else if gap(&before) > 70 { (toward(&before), 0) } else { (D | away(&before), 0) };
            let following = m.fair_cpu_following_game_plan();
            step(&mut m, pad.0, pad.1);
            let v = seen(&m);
            // Within a second of the guard ending, a game plan starts another
            // attack.
            if since_guard.is_some_and(|at| i - at < 60) && following && attack_started(&before, &v) {
                pressured += 1;
                since_guard = None;
            }
            // His punch started, and then he was hit: caught in the gap.
            if press_at.is_some_and(|at| i - at < 20) && v.foe.attacking() && !punch_out {
                punch_out = true;
                punches += 1;
            }
            // Hit by one of its attacks (not thrown).
            let struck = v.me.attacking() && v.me.move_id != sf_engine::combo::throw::THROW_MOVE;
            if v.foe.reacting() && struck && punch_out && press_at.is_some_and(|at| i - at < 20) {
                caught += 1;
                press_at = None;
            }
            if press_at.is_none_or(|at| i - at >= 20) {
                punch_out = false;
            }
        }
    }
    if guards == 0 {
        return;
    }
    eprintln!("guards {guards}, attacks soon after {pressured}, punches {punches}, caught {caught}");
    assert!(guards >= 30, "guards: {guards}");
    // Many guards are followed at once by a game plan's next attack: it
    // keeps its turn.
    assert!(pressured * 4 > guards, "game plan attacks started soon after a guard: {pressured} of {guards}");
    assert!(caught >= 3, "buttons pressed in the gap and hit: {caught} of {punches}");
}

#[test]
fn against_an_opponent_who_keeps_guarding_it_walks_in_and_throws() {
    let (mut thrown, mut walked_in) = (0u32, 0u32);
    let mut each = Vec::new();
    for mut m in starts(true) {
        let mut throws_here = 0;
        let mut was_throwing = false;
        // The CPU's pad was forward alone on some frame since its last throw.
        let mut walking = false;
        for _ in 0..9_000u32 {
            let v = seen(&m);
            let pad = if gap(&v) > 90 { (toward(&v), 0) } else { (D | away(&v), 0) };
            let guarding = v.foe.guarding() || v.foe.crouching();
            step(&mut m, pad.0, pad.1);
            let v = seen(&m);
            let (dirs, buttons) = cpu_pad(&m);
            // Player 1's away is the CPU's toward him.
            if buttons == 0 && dirs == away(&v) {
                walking = true;
            }
            // The CPU's fighter starts the throw move on him while he guards
            // low (a throw the engine let connect: its input alone is not one).
            let throwing = v.me.move_id == sf_engine::combo::throw::THROW_MOVE;
            if throwing && !was_throwing && guarding {
                throws_here += 1;
                walked_in += u32::from(walking);
            }
            if throwing {
                walking = false;
            }
            was_throwing = throwing;
        }
        thrown += throws_here;
        each.push(throws_here);
    }
    if each.is_empty() {
        return;
    }
    eprintln!("throws {thrown} ({each:?}), after a walk in {walked_in}");
    assert!(each.iter().all(|&n| n >= 1), "throws against a guarding opponent in each fight: {each:?}");
    assert!(walked_in * 2 >= thrown, "throws after a walk in: {walked_in} of {thrown}");
}

/// Knocked down, as a frame shows it.
fn down(b: &sf_engine::fair_cpu::Body) -> bool {
    knocked_down(b.move_id, b.state)
}

#[test]
fn against_a_rising_opponent_it_times_an_attack_to_his_rise_and_sometimes_guards_and_punishes_a_reversal() {
    // Player 1 rises and, every other time, reverses with a Shoryuken.
    let (mut rises, mut met, mut baited, mut punished) = (0u32, 0u32, 0u32, 0u32);
    for mut m in starts(true) {
        let mut rises_here = 0u32;
        let mut was_down = false;
        let mut rose_at: Option<u32> = None;
        let mut reversal: Option<u32> = None;
        let mut timed = false;
        let mut counted_meet = false;
        // His Shoryuken came out, met the CPU's guard, and did not hit it;
        // and the CPU's health as it met the guard, while it is out.
        let mut reversal_guarded = false;
        let mut reversal_health: Option<u16> = None;
        for i in 0..20_000u32 {
            let v = seen(&m);
            let f = toward(&v);
            let mut pad = if gap(&v) > 70 { (f, 0) } else { (0, 0) };
            if let Some(at) = reversal {
                let motion = [(f, 0), (f, 0), (D, 0), (D, 0), (D | f, HP), (D | f, HP), (0, 0)];
                match motion.get((i - at) as usize) {
                    Some(&p) => pad = p,
                    None => reversal = None,
                }
            }
            step(&mut m, pad.0, pad.1);
            let v = seen(&m);
            let is_down = down(&v.foe);
            if was_down && !is_down {
                rises_here += 1;
                rose_at = Some(i);
                // A game plan's attack under way as he rises was pressed
                // while he was down: the CPU sees his rise a reaction time
                // late, so it timed it.
                timed = v.me.attacking() && m.fair_cpu_following_game_plan();
                counted_meet = false;
                reversal_guarded = false;
                reversal_health = None;
                if rises_here % 2 == 0 {
                    reversal = Some(i);
                }
            }
            was_down = is_down;
            if let Some(at) = rose_at {
                let since = i - at;
                // That attack met him within a few frames of his rise: he was
                // hit or guarded it. (Its attack box is gone by the frame
                // after it hits, so the box itself is not looked for.)
                let stunned = v.foe.reacting() || m.toggle_state().combo.blockstun[0] != 0;
                if timed && since <= 12 && stunned && !counted_meet {
                    met += 1;
                    counted_meet = true;
                }
                // Guarding (or holding the Drive Parry, SC8) as he rose and
                // his reversal came; counted once its box is gone with the
                // CPU unhurt.
                let guarding = v.me.guarding() || v.me.move_id == DRIVE_PARRY;
                if rises_here % 2 == 0 && v.foe.active && v.foe.move_id == SHORYUKEN && guarding && !reversal_guarded && reversal_health.is_none() {
                    reversal_health = Some(v.me.health);
                }
                if reversal_health.is_some() && v.me.reacting() {
                    reversal_health = None;
                }
                if let Some(health) = reversal_health.filter(|_| !v.foe.active && v.foe.move_id == SHORYUKEN) {
                    reversal_health = None;
                    if v.me.health == health && !v.me.reacting() {
                        reversal_guarded = true;
                        baited += 1;
                    }
                }
                if reversal_guarded && v.me.reacting() {
                    reversal_guarded = false;
                }
                if reversal_guarded && !v.foe.active && m.fair_cpu_entering_route() {
                    punished += 1;
                    rose_at = None;
                }
                if since >= 120 {
                    rose_at = None;
                }
            }
        }
        rises += rises_here;
    }
    if rises == 0 {
        return;
    }
    eprintln!("rises {rises}, met {met}, guarded reversals {baited}, punished {punished}");
    assert!(rises >= 30, "knockdowns: {rises}");
    // A good share of rises meet an attack timed to them.
    assert!(met * 10 >= rises, "attacks out as he rose: {met} of {rises}");
    assert!(baited >= 2, "guards held over a reversal: {baited}");
    assert!(punished >= 2, "reversals punished with a route: {punished}");
}

#[test]
fn when_its_pressure_has_run_out_a_game_plan_backs_off() {
    // Player 1 guards low in the open, walking out of a corner. The frame
    // trap's second attack guarded too means its pressure has run out: the
    // plan gives ground and guards. Counted: back-offs inside a game plan,
    // the stick held away from him alone, begun within a second of a guard.
    let (mut guards, mut backed_off) = (0u32, 0u32);
    for mut m in starts(true) {
        let mut was_stunned = false;
        let mut since_guard: Option<u32> = None;
        for i in 0..9_000u32 {
            let v = seen(&m);
            let in_corner = m.fair_cpu_situation().is_some_and(|j| j.foe_cornered);
            let pad = if in_corner { (toward(&v), 0) } else { (D | away(&v), 0) };
            step(&mut m, pad.0, pad.1);
            let v = seen(&m);
            let stunned = m.toggle_state().combo.blockstun[0] != 0;
            if was_stunned && !stunned {
                guards += 1;
                since_guard = Some(i);
            }
            was_stunned = stunned;
            // Player 1's toward is the CPU's away from him.
            let (dirs, buttons) = cpu_pad(&m);
            if since_guard.is_some_and(|at| i - at < 60) && m.fair_cpu_following_game_plan() && dirs == toward(&v) && buttons == 0 && !v.me.reacting() {
                backed_off += 1;
                since_guard = None;
            }
        }
    }
    if guards == 0 {
        return;
    }
    eprintln!("guards {guards}, game plan back-offs soon after {backed_off}");
    assert!(backed_off >= 5, "game plans that backed off after a guard: {backed_off} of {guards}");
}

#[test]
fn with_the_opponent_cornered_it_presses_more_and_gives_less_ground() {
    // Player 1 crouches and guards in both cases. In the corner case he
    // backs into a wall until the CPU judges him cornered, and back again
    // whenever he is put out of it (a throw turns him round); in the open
    // case he walks out whenever it judges him cornered. Only the frames
    // where the case holds are counted.
    // Ground given: pixels the CPU moved away from him while free to move
    // (not in hitstun or blockstun, whose pushback it does not choose).
    // Pressing: of the frames he was on his feet, the share the CPU spent
    // passive (standing, walking back, crouching or guarding) rather than
    // attacking or walking in. (Attacks per frame do not tell: in the
    // corner its guarded attacks push it back off him, not him away.)
    let run = |start: &PowerOnMachine, cornered: bool| {
        let mut m = start.clone();
        let (mut retreats, mut passive, mut standing, mut counted) = (0u32, 0u32, 0u32, 0u32);
        for i in 0..8_000u32 {
            let v = seen(&m);
            let in_corner = m.fair_cpu_situation().is_some_and(|j| j.foe_cornered);
            let pad = match (cornered, in_corner) {
                (true, false) => (away(&v), 0),
                (false, true) => (toward(&v), 0),
                _ => (D | away(&v), 0),
            };
            let before = v;
            step(&mut m, pad.0, pad.1);
            let v = seen(&m);
            if i < 600 || in_corner != cornered {
                continue;
            }
            counted += 1;
            let free = !v.me.reacting() && m.toggle_state().combo.blockstun[1] == 0;
            let away_px = (v.me.x - before.me.x) * if v.me.x >= v.foe.x { 1 } else { -1 };
            if free && before.me.free() {
                retreats += away_px.max(0) as u32;
            }
            if !down(&v.foe) && v.foe.move_id < 0x30 {
                standing += 1;
                passive += u32::from(matches!(v.me.move_id, 1 | 3..=8));
            }
        }
        [retreats, passive, standing, counted]
    };
    let sum = |cornered: bool| {
        starts(true).iter().map(|m| run(m, cornered)).fold([0u32; 4], |acc, r| std::array::from_fn(|k| acc[k] + r[k]))
    };
    let [corner_back, corner_passive, corner_standing, corner_frames] = sum(true);
    let [open_back, open_passive, open_standing, open_frames] = sum(false);
    if corner_frames + open_frames == 0 {
        return;
    }
    let per_thousand = |n: u32, of: u32| n * 1000 / of.max(1);
    let (corner_ground, open_ground) = (per_thousand(corner_back, corner_frames), per_thousand(open_back, open_frames));
    let (corner_rate, open_rate) = (per_thousand(corner_passive, corner_standing), per_thousand(open_passive, open_standing));
    eprintln!("frames {corner_frames}/{open_frames}, ground per thousand {corner_ground}/{open_ground}, passive per thousand {corner_rate}/{open_rate} (cornered/open)");
    assert!(corner_frames >= 6_000 && open_frames >= 6_000, "frames counted: {corner_frames} cornered, {open_frames} in the open");
    assert!(corner_ground < open_ground, "ground given per thousand frames: {corner_ground} cornered, {open_ground} in the open");
    assert!(corner_rate < open_rate, "passive frames per thousand he was on his feet: {corner_rate} cornered, {open_rate} in the open");
    // In the open it does give ground: a pressure sequence that has run out
    // ends by backing off.
    assert!(open_back > 0, "it backed off in the open");
}

#[test]
fn inside_a_game_plan_it_does_not_start_an_attack_into_one_it_has_seen_start() {
    let mut presses = 0;
    // Summed over a few fights, so the count does not rest on one run of
    // the CPU's random choices.
    for mut m in starts(true) {
        let mut shown: HashMap<u32, Percept> = HashMap::new();
        let mut cpu_saw: HashMap<u32, Percept> = HashMap::new();
        let mut last = 0u8;
        let mut was_route = false;
        for i in 0..12_000u32 {
            let v = seen(&m);
            let r = i.wrapping_mul(2_654_435_761) >> 9;
            let pad = if i % 23 == 0 { (0, 1 << (r % 6)) } else if gap(&v) > 70 { (toward(&v), 0) } else { (0, 0) };
            let following = m.fair_cpu_following_game_plan();
            step(&mut m, pad.0, pad.1);
            let frame = m.state.frame;
            shown.insert(frame, seen(&m));
            let route_began = m.fair_cpu_entering_route() && !was_route;
            was_route = m.fair_cpu_entering_route();
            // A press, or a route begun, inside a game plan. Not the Drive
            // Parry's buttons held in a guard, the stick back: that is no
            // attack (SC8). (With the stick forward they are a Drive Rush.)
            let now = seen(&m);
            let (dirs, buttons) = cpu_pad(&m);
            let parry = buttons == MP | MK && dirs & toward(&now) != 0 && dirs & away(&now) == 0;
            let pressed = (cpu_pressed(&m, &mut last) && !was_route && !parry) || route_began;
            // The view this press was decided on, and the one before it, as
            // the CPU saw them.
            let Some(view) = as_seen(&shown, frame, m.fair_cpu_attention().unwrap()) else { continue };
            cpu_saw.insert(frame, view);
            let Some(before) = cpu_saw.get(&(frame - 1)).copied() else { continue };
            if pressed && following {
                presses += 1;
                let started = view.foe.attacking() && (!before.foe.attacking() || before.foe.move_id != view.foe.move_id);
                assert!(!started, "frame {frame}: a plan's attack pressed as it saw his attack start: {view:?}");
            }
        }
    }
    if !roms_present() {
        return;
    }
    assert!(presses >= 10, "attacks pressed inside game plans: {presses}");
}

// SC6 (#198): purposeful movement in neutral.

/// From Player 1's centre to the near edge of the CPU fighter's hurtbox:
/// what Player 1's attacks must reach.
fn reach_gap(v: &Percept) -> i16 {
    if v.me.x >= v.foe.x { v.me.hurt.0 - v.foe.x } else { v.foe.x - v.me.hurt.1 }
}

/// What the CPU's fighter did in neutral over `frames` against a Player 1
/// who throws out a Shoryuken when he sees the CPU come into his reach, 10
/// frames after it does (a quick person's reaction). (His normals recover
/// within about 5 to 19 frames of their box going, inside the CPU's
/// 12-frame reaction at Very Difficult: a whiffed normal cannot be punished
/// on sight. A whiffed Shoryuken can.)
#[derive(Clone, Copy, Debug, Default)]
struct Hover {
    /// Turns from walking in to walking out with Player 1's reach edge
    /// near (within 24 pixels of the farthest his attacks have reached).
    edge_turns: u32,
    /// Player 1's Shoryukens whose box never reached the CPU.
    whiffs: u32,
    /// His swings begun within 20 frames of an edge turn, and those that
    /// touched the CPU (hit or guarded): it had not stepped back out of
    /// his reach.
    swung_after_turn: u32,
    touched_after_turn: u32,
    /// Of those, the ones he began within 20 frames of an edge turn: he
    /// swung at the CPU's step in, and it had stepped back out.
    drawn: u32,
    /// Routes begun within 30 frames of a drawn whiff's box going, and
    /// those that hit him.
    punished: u32,
    landed: u32,
    /// Frames played.
    frames: u32,
}

fn hover(start: &PowerOnMachine, frames: u32) -> Hover {
    let mut m = start.clone();
    let mut out = Hover { frames, ..Hover::default() };
    // Player 1's longest attack box seen, from his centre, and the
    // distance he swings from (a little inside it).
    let mut p1_reach: i16 = 50;
    let mut poked_at: Option<u32> = None;
    let mut poke_after_turn = false;
    let (mut p1_active, mut p1_contact) = (false, false);
    // The CPU's last sideways movement (+1 toward him, -1 away) and the
    // frame of its last edge turn.
    let mut moving = 0i16;
    let mut turned_at: Option<u32> = None;
    let mut drawn_at: Option<u32> = None;
    let mut punish_at: Option<u32> = None;
    let mut was_route = false;
    let mut gaps = [i16::MAX; 10];
    let mut rising = 0;
    let mut touched: Option<u32> = None;
    for i in 0..frames {
        let v = seen(&m);
        // He swings at the CPU as he sees it come into his reach.
        let seen_gap = gaps[i as usize % 10];
        gaps[i as usize % 10] = reach_gap(&v);
        let near = seen_gap <= p1_reach;
        let guard = if poked_at.is_some_and(|at| i - at < 7) { None } else { rising_guard(&mut rising, &v) };
        let pad = match poked_at {
            _ if guard.is_some() => guard.unwrap(),
            Some(at) if i - at < 7 => {
                let f = toward(&v);
                [(f, 0), (f, 0), (D, 0), (D, 0), (D | f, HP), (D | f, HP), (0, 0)][(i - at) as usize]
            }
            Some(at) if i - at < 90 => (0, 0),
            _ if near && v.foe.free() && v.me.free() => {
                poked_at = Some(i);
                poke_after_turn = turned_at.is_some_and(|t| i - t <= 20);
                out.swung_after_turn += u32::from(poke_after_turn);
                (toward(&v), 0)
            }
            // Out of a corner, and in from beyond a projectile's range: the
            // fight stays in the open, around the edge of his reach.
            _ if v.foe.x < 114 + 50 || v.foe.x > 400 - 50 || gap(&v) > 90 => (toward(&v), 0),
            _ => (0, 0),
        };
        let cpu_health = v.me.health;
        step(&mut m, pad.0, pad.1);
        let after = seen(&m);
        // His Shoryuken can hit the CPU without its box ever being seen
        // live (the CPU is shown already in its hit reaction): a reaction
        // begun, or health lost, while his swing runs touched it too.
        let hit_now = (after.me.reacting() && !v.me.reacting()) || after.me.health < cpu_health;
        if hit_now && poke_after_turn && touched != poked_at && poked_at.is_some_and(|at| i - at < 40) {
            touched = poked_at;
            out.touched_after_turn += 1;
        }
        // The CPU's own walking (not pushback from a hit or a guard).
        let to_p1: i16 = if after.foe.x >= after.me.x { 1 } else { -1 };
        let dx = (after.me.x - v.me.x) * to_p1;
        if after.me.free() && v.me.free() && !after.me.guarding() && dx != 0 {
            let dir = dx.signum();
            if moving > 0 && dir < 0 && (reach_gap(&after) - p1_reach).abs() <= 24 {
                out.edge_turns += 1;
                turned_at = Some(i);
            }
            moving = dir;
        }
        if after.foe.active {
            p1_active = true;
            p1_reach = p1_reach.max((after.foe.front - after.foe.x) * -to_p1);
            let toward_cpu = -to_p1;
            let edge = if toward_cpu > 0 { after.me.hurt.0 } else { after.me.hurt.1 };
            p1_contact |= after.me.reacting() || after.me.health < cpu_health || (after.foe.front - edge) * toward_cpu >= 0;
            if p1_contact && poke_after_turn && touched != poked_at {
                touched = poked_at;
                out.touched_after_turn += 1;
            }
        } else if p1_active {
            p1_contact |= after.me.reacting() || after.me.health < cpu_health;
            if !p1_contact {
                out.whiffs += 1;
                if poke_after_turn {
                    out.drawn += 1;
                    drawn_at = Some(i);
                }
            }
            p1_active = false;
            p1_contact = false;
        }
        let in_route = m.fair_cpu_entering_route();
        if in_route && !was_route && drawn_at.is_some_and(|at| i - at <= 30) {
            out.punished += 1;
            punish_at = Some(i);
            drawn_at = None;
        }
        was_route = in_route;
        if after.foe.reacting() && punish_at.is_some_and(|at| i - at <= 60) {
            out.landed += 1;
            punish_at = None;
        }
    }
    out
}

/// The starts the edge test sums: twenty, so that its rates hold still
/// (over five, drawn whiffs ran from 7 to 10 as unrelated changes
/// reshuffled the CPU's random choices).
const HOVER_STARTS: [u32; 20] = [0, 97, 211, 331, 449, 563, 677, 791, 907, 1013, 1129, 1237, 1361, 1471, 1583, 1693, 1801, 1913, 2029, 2141];

fn hover_sum(stronger: bool) -> Hover {
    starts_from(stronger, 4, &HOVER_STARTS).iter().map(|m| hover(m, 12_000)).fold(Hover::default(), |a, h| Hover {
        edge_turns: a.edge_turns + h.edge_turns,
        whiffs: a.whiffs + h.whiffs,
        swung_after_turn: a.swung_after_turn + h.swung_after_turn,
        touched_after_turn: a.touched_after_turn + h.touched_after_turn,
        drawn: a.drawn + h.drawn,
        punished: a.punished + h.punished,
        landed: a.landed + h.landed,
        frames: a.frames + h.frames,
    })
}

#[test]
fn in_neutral_it_steps_in_and_out_at_the_edge_of_the_opponents_reach_and_punishes_the_whiff_it_draws() {
    let new = hover_sum(true);
    let old = hover_sum(false);
    if !roms_present() {
        return;
    }
    eprintln!("stronger {new:?}\nold {old:?}");
    // Rates per 10,000 frames of what was played.
    let per = |n: u32| f64::from(n) * 10_000.0 / f64::from(new.frames.max(1));
    eprintln!("per 10,000 frames: edge turns {:.1}, whiffs {:.1}, drawn {:.1}, punished {:.1}, landed {:.1}", per(new.edge_turns), per(new.whiffs), per(new.drawn), per(new.punished), per(new.landed));
    // Floors for twenty starts of 12,000 frames (240,000 frames).
    assert!(new.edge_turns >= 480, "turns from in to out at the edge of his reach: {new:?}");
    assert!(2 * new.edge_turns >= 3 * old.edge_turns, "the old Fair CPU turns there by chance: {new:?} against {old:?}");
    // Its last walk in, to poke, stops at the poke's reach, so his swing at
    // an edge turn meets the CPU less often: of those swings, 30 of 66
    // touch it (SC13, `#205`; the same count gives 128 of 185 with the walk
    // reckoned whole, and 160 of 254 before SC8). Touches include his
    // Shoryukens that hit it without their box ever being seen live.
    assert!(new.swung_after_turn >= 40, "swings at an edge turn: {new:?}");
    assert!(2 * new.touched_after_turn < new.swung_after_turn, "swings at an edge turn that touched the CPU: {new:?}");
    // Drawn whiffs: about 1.5 per 10,000 frames (35 here), against 3.0
    // before SC8 (71 over the same twenty starts). The drop is put down to
    // SC8's removal of the Utility AI's idle-frame commands, which moved the
    // CPU in and out between plan steps (no fault was found in the plans;
    // the commands cannot be switched back on to measure it directly). The
    // maintainer accepted the lower rate on 2026-11-20; the floor is 1.2
    // per 10,000.
    assert!(per(new.drawn) >= 1.2 && new.drawn > old.drawn, "whiffs drawn by stepping out: {new:?} against {old:?}");
    assert!(new.punished >= 24 && new.landed >= 24, "drawn whiffs punished with a route, and landed: {new:?}");
}

#[test]
fn in_far_neutral_it_follows_game_plans_that_take_space_and_throw_projectiles_from_afar() {
    // Player 1 keeps his distance away from the walls: he jumps back while
    // the CPU is near, walks out of a corner when he is in one, and
    // otherwise stands still, guarding nothing. Only far neutral away from
    // the corner is counted (a cornered opponent has a plan of its own).
    let run = |start: &PowerOnMachine| {
        let mut m = start.clone();
        let (mut far, mut far_in_plan, mut far_shots, mut far_walk_in) = (0u32, 0u32, 0u32, 0u32);
        // The game plan being followed, and whether it began in far neutral;
        // frames since the CPU last held the stick down, and its buttons.
        let (mut plan, mut plan_far) = (None, false);
        let (mut since_down, mut last_buttons) = (99u32, 0u8);
        for i in 0..9_000u32 {
            let v = seen(&m);
            let walled = v.foe.x < 114 + 50 || v.foe.x > 400 - 50;
            let pad = if walled {
                (toward(&v), 0)
            } else if gap(&v) < 130 && v.foe.free() {
                (U | away(&v), 0)
            } else {
                (0, 0)
            };
            let judged_far = m.fair_cpu_situation().is_some_and(|j| j.situation == Situation::NeutralFar && !j.foe_cornered);
            let following = m.fair_cpu_following_game_plan();
            step(&mut m, pad.0, pad.1);
            let after = seen(&m);
            let began = m.fair_cpu_game_plan_began();
            if began != plan {
                plan = began;
                plan_far = began.is_some() && m.fair_cpu_situation().is_some_and(|j| j.situation == Situation::NeutralFar);
            }
            // A projectile's input (a punch with the stick forward soon after
            // it was held down) inside a game plan begun in far neutral.
            let (dirs, buttons) = cpu_pad(&m);
            let forward = if after.me.x >= after.foe.x { L } else { R };
            let punch = buttons & (LP | MP | HP) & !last_buttons != 0;
            if i >= 300 && punch && dirs & forward != 0 && since_down <= 8 && began.is_some() && plan_far {
                far_shots += 1;
            }
            // Walking in, inside a game plan begun in far neutral.
            let to_p1: i16 = if after.foe.x >= after.me.x { 1 } else { -1 };
            if i >= 300 && began.is_some() && plan_far && after.me.free() && (after.me.x - v.me.x) * to_p1 > 0 {
                far_walk_in += 1;
            }
            since_down = if dirs & D != 0 && buttons == 0 { 0 } else { since_down + 1 };
            last_buttons = buttons;
            if i < 300 || !judged_far {
                continue;
            }
            far += 1;
            far_in_plan += u32::from(following);
        }
        [far, far_in_plan, far_shots, far_walk_in]
    };
    let [far, in_plan, shots, walk_in] =
        starts(true).iter().map(run).fold([0u32; 4], |a, r| std::array::from_fn(|k| a[k] + r[k]));
    if !roms_present() {
        return;
    }
    eprintln!("far frames {far}, in a game plan {in_plan}, projectile inputs from a plan begun far {shots}, frames walking in inside a plan begun far {walk_in}");
    // Far neutral is mostly spent in a game plan, walking in is part of it,
    // and projectiles are thrown from plans begun far off. (Far neutral is
    // short against this CPU, which closes in fast; the plans' contents are
    // pinned by the unit test `far_neutral_plans_walk_in_and_throw_projectiles`.)
    assert!(in_plan * 2 >= far, "far neutral followed by a game plan: {in_plan} of {far} frames");
    assert!(walk_in >= 30, "frames walking in to take space inside a game plan begun far: {walk_in}");
    assert!(shots >= 1, "projectiles thrown from afar inside a game plan: {shots}");
}

#[test]
fn it_does_not_walk_into_a_projectile_or_an_attack_it_has_seen() {
    let run = |start: &PowerOnMachine| {
        let mut m = start.clone();
        // Each frame shown, and over the views the CPU decided from (SC7:
        // an attack it was not watching for shows only at its slower
        // delay), whether its attack box has been seen to come and go.
        let mut shown_at: HashMap<u32, Percept> = HashMap::new();
        let (mut box_seen, mut last_move) = (false, 0u8);
        let (mut walks, mut walks_into, mut route_pressed) = (0u32, Vec::new(), false);
        let mut recent_down = 0u8;
        let mut shots = 0u32;
        let mut had_shot = false;
        for i in 0..14_000u32 {
            let v = seen(&m);
            let f = toward(&v);
            // Player 1: projectiles from afar, buttons up close, walking in
            // between, on a cycle.
            let t = i % 160;
            let pad = if gap(&v) > 110 {
                match t {
                    0..=2 => (D, 0),
                    3..=5 => (D | f, 0),
                    6..=9 => (f, LP),
                    10..=60 => (0, 0),
                    _ => (f, 0),
                }
            } else if t % 37 < 3 {
                (0, [HP, HK, MK][(i / 37 % 3) as usize])
            } else if t < 80 {
                (0, 0)
            } else {
                (f, 0)
            };
            let was_route = m.fair_cpu_entering_route();
            step(&mut m, pad.0, pad.1);
            let frame = m.state.frame;
            let shown = seen(&m);
            shown_at.insert(frame, shown);
            // The view the CPU decided this frame's pad from, as it saw it.
            let Some(attention) = m.fair_cpu_attention() else { continue };
            let Some(view) = as_seen(&shown_at, frame, attention) else { continue };
            if view.foe.attacking() {
                if view.foe.move_id != last_move {
                    box_seen = false;
                }
                box_seen |= view.foe.active;
                last_move = view.foe.move_id;
            } else {
                box_seen = false;
                last_move = 0;
            }
            // Something coming at it in that view: a projectile in front of
            // it, or an attack whose box has not yet come and gone (start-up
            // or active, not recovery).
            let recovering = box_seen && !view.foe.active;
            let to_p1: i16 = if view.foe.x >= view.me.x { 1 } else { -1 };
            let shot_coming = view.shot && (view.shot_x - view.me.x) * to_p1 > -16;
            let seen_coming = shot_coming || (view.foe.attacking() && !recovering);
            // Toward him as that view showed it (a jump over it turns the
            // fighters round before it can see that).
            let forward = if view.foe.x >= view.me.x { R } else { L };
            if shown.shot && !had_shot {
                shots += 1;
            }
            had_shot = shown.shot;
            // A route's walk in is its pads before its first button.
            let in_route = m.fair_cpu_entering_route();
            let (dirs, buttons) = cpu_pad(&m);
            if !in_route || !was_route {
                route_pressed = false;
            }
            route_pressed |= in_route && buttons != 0;
            // Held in blockstun the stick forward is no walk: it begins a
            // Drive Reversal (SC8). Nor is it just after the stick was down:
            // it is a motion's last direction (a quarter circle's, a Super
            // Art's), which the CPU's answer to what it saw can be (SC13).
            let held = m.toggle_state().combo.blockstun[1] != 0;
            let motion = recent_down != 0;
            recent_down = if dirs & D != 0 { 3 } else { recent_down.saturating_sub(1) };
            let walking_in = dirs == forward && buttons == 0 && !route_pressed && !held && !motion;
            if i < 300 || !walking_in || !shown.me.free() || down(&shown.foe) {
                continue;
            }
            walks += 1;
            if seen_coming {
                walks_into.push(frame);
            }
        }
        (walks, walks_into, shots)
    };
    let mut total = (0u32, Vec::new(), 0u32);
    for m in starts(true) {
        let (w, into, s) = run(&m);
        total.0 += w;
        total.1.extend(into);
        total.2 += s;
    }
    let (walks, into, shots) = total;
    if !roms_present() {
        return;
    }
    eprintln!("frames walking in {walks}, projectiles thrown at it {shots}, walking in with something coming {}", into.len());
    assert!(shots >= 15, "Player 1 threw projectiles: {shots}");
    assert!(walks >= 500, "frames walking in: {walks}");
    assert!(into.is_empty(), "frames it walked in on a view showing an attack or projectile coming: {into:?}");
}

/// Player 1's pad for the focus fight: in turns of 80 frames he backs off
/// and jumps in, presses a button where he stands, or walks in and stops,
/// in an order that does not repeat, so the CPU sees each while watching
/// for one thing or another.
fn focus_pad(i: u32, v: &Percept) -> (u8, u8) {
    let turn = i / 80;
    let t = i % 80;
    let r = turn.wrapping_mul(2_654_435_761) >> 11;
    match r % 3 {
        0 if t < 40 => (away(v), 0),
        0 if (50..53).contains(&t) => (U | toward(v), 0),
        1 if (20..23).contains(&t) => (if r & 8 != 0 { D } else { 0 }, [LP, MP, HP, MK, HK][(r as usize >> 4) % 5]),
        2 if t < 40 && gap(v) > 40 => (toward(v), 0),
        _ => (0, 0),
    }
}

/// Player 1's attack drawn on frame `f` reached the CPU's fighter (or hit
/// him) on an earlier frame of it: not a whiff. As the CPU judges it, over
/// the 39 frames before it that the CPU holds.
fn met_before(shown: &HashMap<u32, Percept>, f: u32) -> bool {
    let Some(now) = shown.get(&f) else { return false };
    for g in (f.saturating_sub(39)..f).rev() {
        let Some(v) = shown.get(&g) else { return false };
        if !v.foe.attacking() || v.foe.move_id != now.foe.move_id {
            return false;
        }
        if v.me.reacting() {
            return true;
        }
        if v.foe.active {
            let toward_me = if v.me.x >= v.foe.x { 1 } else { -1 };
            let near = if toward_me > 0 { v.me.hurt.0 } else { v.me.hurt.1 };
            if (v.foe.front - near) * toward_me >= 0 {
                return true;
            }
        }
    }
    false
}

#[test]
fn its_focus_is_answered_at_the_focused_delay_and_anything_else_at_the_slower_one() {
    let (fast, slow) = (Level::VeryHard.reaction(), Level::VeryHard.unfocused_reaction());
    // Player 1's jumps, buttons and whiffs, by whether the CPU was watching
    // for them as they came into its focused delay: how many, and how many
    // changed its pad on the step it first saw them.
    let mut seen_at: HashMap<(Focus, bool), (u32, u32)> = HashMap::new();
    for mut m in starts(true) {
        let mut shown: HashMap<u32, Percept> = HashMap::new();
        let mut attention: HashMap<u32, Attention> = HashMap::new();
        let mut pads = HashMap::new();
        for i in 0..16_000u32 {
            let v = seen(&m);
            let (dirs, buttons) = focus_pad(i, &v);
            step(&mut m, dirs, buttons);
            let frame = m.state.frame;
            shown.insert(frame, seen(&m));
            attention.insert(frame, m.fair_cpu_attention().unwrap());
            pads.insert(frame, cpu_pad(&m));
        }
        let first = *shown.keys().min().unwrap();
        let last = *shown.keys().max().unwrap();
        for f in first + 40..last - 40 {
            let (Some(v), Some(b)) = (shown.get(&f), shown.get(&(f - 1))) else { continue };
            if !v.fighting || !b.fighting {
                continue;
            }
            let (foe, was) = (v.foe, b.foe);
            let event = if matches!(foe.move_id, 9..=0xb) && !was.jumping() {
                Focus::Jump
            } else if foe.attacking() && !was.jumping() && !was.reacting() && (!was.attacking() || was.move_id != foe.move_id) {
                Focus::Attack
            } else if foe.attacking() && was.active && !foe.active && !v.me.reacting() && !met_before(&shown, f) {
                Focus::Whiff
            } else {
                continue;
            };
            // What the CPU watched for on the step before the frame came into
            // its focused delay, and the step on which its view of Player 1's
            // actions first reached the frame.
            let held = attention[&(f + fast - 1)];
            if held.action_frame == 0 {
                continue;
            }
            let Some(at) = (f..=f + slow).find(|s| attention[s].action_frame >= f) else {
                panic!("frame {f}: {event:?} not seen within {slow} frames");
            };
            let delay = at - f;
            let watched = held.focus == event;
            let why = format!("frame {f}: {event:?} first seen on {at}, watching for {:?}", held.focus);
            if watched {
                // Its focus: as soon as its focused delay shows it.
                assert_eq!(delay, fast, "{why}");
            } else {
                // Anything else: at the slower delay, unless its focus showed
                // after it or it came to watch for it meanwhile, but never
                // sooner than the focused delay.
                assert!((fast..=slow).contains(&delay), "{why}");
                if delay != slow {
                    continue;
                }
            }
            let entry = seen_at.entry((event, watched)).or_default();
            entry.0 += 1;
            entry.1 += u32::from(pads[&at] != pads[&(at - 1)]);
        }
    }
    let sum = |watched: bool| {
        seen_at.iter().filter(|((_, w), _)| *w == watched).fold((0, 0), |(n, a), (_, &(dn, da))| (n + dn, a + da))
    };
    let ((watched, watched_answered), (unwatched, unwatched_answered)) = (sum(true), sum(false));
    eprintln!("{seen_at:?}");
    assert!(watched >= 10, "events seen while watching for them: {seen_at:?}");
    assert!(unwatched >= 10, "events seen while watching for something else: {seen_at:?}");
    assert!(watched_answered >= 3, "watched events answered as first seen, at the focused delay: {seen_at:?}");
    assert!(unwatched_answered >= 3, "other events answered as first seen, at the slower delay: {seen_at:?}");
    // More than one kind of event was watched for.
    assert!(seen_at.keys().filter(|(_, w)| *w).count() >= 2, "{seen_at:?}");
}

// SC8 (#200): the Fair CPU owns every era command in CPU fights.

/// The CPU's fighter began move `id` between `before` and `v`.
fn cpu_began(before: &Percept, v: &Percept, id: u8) -> bool {
    v.me.move_id == id && before.me.move_id != id
}

/// Player 1 began an attack between `before` and `v`.
fn p1_attack_began(before: &Percept, v: &Percept) -> bool {
    v.foe.attacking() && (!before.foe.attacking() || before.foe.move_id != v.foe.move_id)
}

/// Player 1 walks into reach and presses a normal every 41 frames.
fn poking(i: u32, v: &Percept) -> (u8, u8) {
    if gap(v) > 60 {
        (toward(v), 0)
    } else if i % 41 < 3 {
        (0, [LP, MP, HP, MK, HK][(i / 41 % 5) as usize])
    } else {
        (0, 0)
    }
}

/// Player 1 walks in close and presses crouching jabs, guarding low
/// between them: pressure for the CPU to guard.
fn pressing(i: u32, v: &Percept) -> (u8, u8) {
    if gap(v) > 30 {
        (toward(v), 0)
    } else if i % 12 < 2 {
        (D, LP)
    } else {
        (D | away(v), 0)
    }
}

/// Player 1 whiffs a Shoryuken whenever the CPU stands just out of its
/// reach, and walks in when it is farther (as in the whiff punish test).
fn whiffing() -> impl FnMut(u32, &Percept) -> (u8, u8) {
    let mut started: Option<u32> = None;
    move |i, v| {
        let f = toward(v);
        if let Some(at) = started {
            let motion = [(f, 0), (f, 0), (D, 0), (D, 0), (D | f, HP), (D | f, HP), (0, 0)];
            if let Some(&pad) = motion.get((i - at) as usize) {
                return pad;
            }
            if i - at < 90 {
                return (0, 0);
            }
            started = None;
        }
        let g = gap(v);
        if (60..=90).contains(&g) && v.foe.free() {
            started = Some(i);
            (f, 0)
        } else if g > 90 {
            (f, 0)
        } else {
            (0, 0)
        }
    }
}

#[test]
fn with_the_switch_on_no_command_in_a_cpu_fight_comes_from_the_utility_ai() {
    // Frames a command of the Utility AI's was under way on Player 2's pad,
    // over a few fights.
    let utility_frames = |stronger: bool| -> u32 {
        starts(stronger)
            .into_iter()
            .map(|mut m| {
                let mut frames = 0;
                for i in 0..6_000u32 {
                    let v = seen(&m);
                    let (dirs, buttons) = sparring_pad(i, &v);
                    step(&mut m, dirs, buttons);
                    frames += u32::from(m.toggle_state().utility[1].macro_remaining() > 0);
                }
                frames
            })
            .sum()
    };
    let (on, off) = (utility_frames(true), utility_frames(false));
    if !roms_present() {
        return;
    }
    eprintln!("frames with a Utility AI command under way: switch on {on}, off {off}");
    // The old play shares the pad with the Utility AI; the stronger play
    // enters every command itself.
    assert!(off > 0, "with the switch off the Utility AI adds commands");
    assert_eq!(on, 0, "with the switch on the Utility AI adds none");
}

#[test]
fn ryu_sf6_closes_in_from_afar_with_a_drive_rush() {
    // Short trials from a few starting points: Player 1 backs off for a
    // while (out of a corner first), then stands far off. A Drive Rush begun
    // from beyond the reach of the standing medium punch it rushes into,
    // followed within 45 frames by an attack of its own begun in reach of
    // him, closed in.
    let (mut trials, mut rushes, mut closed) = (0u32, 0u32, 0u32);
    for start in starts_from(true, 4, &[0, 97, 211, 331, 449, 563, 677, 791]) {
        for back in [40u32, 60, 80, 100, 120, 140] {
            let mut m = start.clone();
            let mut before = seen(&m);
            let mut rushed_at: Option<u32> = None;
            trials += 1;
            for i in 0..480u32 {
                let walled = before.foe.x < 114 + 50 || before.foe.x > 400 - 50;
                let pad = if i < back { if walled { toward(&before) } else { away(&before) } } else { 0 };
                step(&mut m, pad, 0);
                let v = seen(&m);
                if cpu_began(&before, &v, DRIVE_RUSH) && gap(&before) > RUSHED_NORMAL_REACH {
                    rushes += 1;
                    rushed_at = Some(i);
                } else if let Some(at) = rushed_at {
                    if attack_started(&before, &v) && v.me.normal() && gap(&v) <= 60 {
                        closed += 1;
                        rushed_at = None;
                    } else if i - at > 45 {
                        rushed_at = None;
                    }
                }
                before = v;
            }
        }
    }
    if !roms_present() {
        return;
    }
    eprintln!("trials {trials}, Drive Rushes from afar {rushes}, followed by an attack in reach {closed}");
    assert!(closed >= 3, "Drive Rushes that closed in to an attack: {closed} of {rushes} in {trials} trials");
}

#[test]
fn ryu_sf6_meets_an_attack_it_has_seen_with_a_drive_impact_or_a_parry() {
    // Every Drive Impact or Drive Parry begun within a second of Player 1
    // beginning an attack, and how long after it began. One begun sooner
    // than the CPU can see an attack that he began from 20 frames or more
    // of standing free (nothing before it to answer) would answer what it
    // has not seen.
    let (mut impacts, mut parries) = (0u32, 0u32);
    let mut early = Vec::new();
    for mut m in starts(true) {
        let mut before = seen(&m);
        let mut p1_began: Option<(u32, bool)> = None;
        let mut free_for = 0u32;
        for i in 0..12_000u32 {
            let (dirs, buttons) = poking(i, &before);
            step(&mut m, dirs, buttons);
            let v = seen(&m);
            if p1_attack_began(&before, &v) {
                p1_began = Some((i, free_for >= 20));
            }
            free_for = if v.foe.free() && !v.foe.guarding() { free_for + 1 } else { 0 };
            for (id, count) in [(DRIVE_IMPACT, &mut impacts), (DRIVE_PARRY, &mut parries)] {
                if let Some((at, isolated)) = p1_began.filter(|&(at, _)| cpu_began(&before, &v, id) && i - at <= 60) {
                    *count += 1;
                    // His attack can only be answered once the CPU has seen it.
                    if isolated && i - at < Level::VeryHard.reaction() {
                        early.push((id, i - at));
                    }
                }
            }
            before = v;
        }
    }
    if !roms_present() {
        return;
    }
    eprintln!("answered with a Drive Impact {impacts}, with a Drive Parry {parries}");
    assert!(impacts + parries >= 5, "Drive Impacts {impacts} and Parries {parries} against attacks it saw");
    assert!(early.is_empty(), "answered sooner than it could see the attack: {early:?}");
}

#[test]
fn ryu_sf6_drive_reverses_out_of_its_guard() {
    // A Drive Reversal begun while the CPU's fighter was held in blockstun.
    let mut reversals = 0u32;
    for mut m in starts(true) {
        let mut before = seen(&m);
        for i in 0..12_000u32 {
            let guarding = m.toggle_state().combo.blockstun[1] != 0;
            let (dirs, buttons) = pressing(i, &before);
            step(&mut m, dirs, buttons);
            let v = seen(&m);
            reversals += u32::from(guarding && cpu_began(&before, &v, DRIVE_REVERSAL));
            before = v;
        }
    }
    if !roms_present() {
        return;
    }
    eprintln!("Drive Reversals out of a guard {reversals}");
    assert!(reversals >= 3, "Drive Reversals out of a guard: {reversals}");
}

#[test]
fn ryu_sf6_spends_a_super_inside_a_route() {
    // Training's FILL METERS keeps its Super meter full, so the round state
    // lets it spend; Player 1 pokes in reach, and the routes it is punished
    // and hit with include ones that go into a Super Art. Five starting
    // points: such a route is one of several the CPU picks from.
    let book = book(TournamentFighter::Ryu, Revision::Sf6, Loadout::Standard).unwrap();
    let mut supers = 0u32;
    let mut routes = 0u32;
    for mut m in starts_from(true, 4, &[0, 97, 211, 331, 449]) {
        let mut config = *m.match_config();
        config.fill_meters = true;
        m.set_match_config(config);
        let mut before = seen(&m);
        let (mut was_route, mut since_route) = (false, u32::MAX);
        // The book's route and step it entered last: was that step a
        // Super Art's motion (a double quarter circle)?
        let mut at_super_step = false;
        for i in 0..12_000u32 {
            let (dirs, buttons) = poking(i, &before);
            step(&mut m, dirs, buttons);
            let v = seen(&m);
            let in_route = m.fair_cpu_entering_route();
            routes += u32::from(in_route && !was_route);
            since_route = if in_route { 0 } else { since_route.saturating_add(1) };
            was_route = in_route;
            if let (Some(index), Some(at)) = (m.fair_cpu_route(), m.fair_cpu_route_step()) {
                let route = &book.routes[index];
                at_super_step = route.super_spent > 0 && ["236236", "214214"].iter().any(|motion| route.steps[at].name.starts_with(motion));
            } else if in_route {
                at_super_step = false;
            }
            // A Super Art begun while the route enters its super's step, or
            // just after (its motion ends the route's pads): not the era
            // kit's own super on a hit it sees, which FILL METERS lets it
            // spend too.
            let super_began = SUPER_ARTS.contains(&v.me.move_id) && !SUPER_ARTS.contains(&before.me.move_id);
            supers += u32::from(super_began && since_route <= 20 && at_super_step);
            before = v;
        }
    }
    if !roms_present() {
        return;
    }
    eprintln!("routes {routes}, Super Arts inside them {supers}");
    assert!(supers >= 2, "Super Arts spent inside a route: {supers} in {routes} routes");
}

#[test]
fn without_a_full_meter_it_keeps_its_super_while_it_leads() {
    // Over a few fights against openings to punish and to hit: every Super
    // Art the CPU chooses comes while it is behind on health, its Super
    // meter is full, or Player 1 is low enough for a route to take the
    // round (its longest punish takes 32 health points). A Super Art that
    // comes out of a route its book records as spending none is not one it
    // chose (two Hadoken motions close together read as the super's motion
    // in play): those are counted apart.
    let book = book(TournamentFighter::Ryu, Revision::Sf6, Loadout::Standard).unwrap();
    let (mut chosen, mut unbooked) = (0u32, 0u32);
    let mut kept = Vec::new();
    for start in starts(true) {
        for whiffs in [false, true] {
            let mut m = start.clone();
            let mut pad = whiffing();
            let mut before = seen(&m);
            // The health bars and its meter over the last second, and the
            // last route it entered and when.
            let mut history: Vec<(i16, i16, u8)> = Vec::new();
            let mut last_route: Option<(usize, u32)> = None;
            for i in 0..12_000u32 {
                let (dirs, buttons) = if whiffs { pad(i, &before) } else { poking(i, &before) };
                step(&mut m, dirs, buttons);
                let v = seen(&m);
                history.push((v.me.health as i16, v.foe.health as i16, m.era_fighters()[1].super_meter));
                if let Some(index) = m.fair_cpu_route() {
                    last_route = Some((index, i));
                }
                if SUPER_ARTS.contains(&v.me.move_id) && !SUPER_ARTS.contains(&before.me.move_id) {
                    let route = last_route.filter(|&(_, at)| i - at <= 30).map(|(index, _)| &book.routes[index]);
                    if route.is_some_and(|route| route.super_spent == 0) {
                        unbooked += 1;
                    } else {
                        chosen += 1;
                        // Some moment of the last second (the route or the
                        // hit it answered was chosen in it) allowed the spend.
                        let window = &history[history.len().saturating_sub(60)..];
                        let allowed = window.iter().any(|&(me, foe, meter)| me < foe || meter >= FULL_SUPER_METER || foe <= LONGEST_PUNISH);
                        if !allowed {
                            kept.push((i, window[0]));
                        }
                    }
                }
                before = v;
            }
        }
    }
    if !roms_present() {
        return;
    }
    eprintln!("Super Arts chosen {chosen}, out of routes that spend none {unbooked}");
    assert!(kept.is_empty(), "Super Arts chosen while leading with meter to gain and the round not to win: {kept:?}");
}

#[test]
fn a_round_that_could_end_the_match_is_known_from_the_rounds_won() {
    // A Versus match of the Fair CPU against a Player 1 who stands still:
    // in the first round neither fighter has a round won; once one has,
    // the round could end the match (the super meter rule spends there).
    use sf_engine::data_bank::DataBank;
    use sf_engine::front::cabinet::Players;
    use sf_engine::front::fight::{Fight, FightKind, FightSetup};
    use sf_engine::roster::PlayMode;
    if !roms_present() {
        return;
    }
    let assets = load_assets(ROMS, MANIFEST).expect("local ROM set assembles");
    let mut config = MatchConfig::definitive();
    config.roster.mode = PlayMode::Versus;
    config.roster.p1 = TournamentFighter::Ryu;
    config.roster.p2 = TournamentFighter::Ryu;
    config.p2_fair_cpu = true;
    config.stronger_cpu = true;
    config.difficulty = 4;
    let setup = FightSetup {
        kind: FightKind::Local,
        players: Players::Two,
        config,
        dsw1: FightDips::DSW1,
        dsw2: FightDips { difficulty: 4, round_time: 100 }.dsw2(),
    };
    let fight = Fight::start_banked(&assets.maincpu, setup, Some(0), None, Some(std::sync::Arc::new(DataBank::build(&assets.maincpu, &[]))))
        .expect("a Versus fight starts");
    let mut m = fight.machine().clone();
    let (mut round, mut was_fighting) = (0u32, false);
    let mut seen_in = [Vec::new(), Vec::new()];
    for _ in 0..30_000 {
        step(&mut m, 0, 0);
        let fighting = seen(&m).fighting;
        if fighting && !was_fighting {
            round += 1;
        }
        was_fighting = fighting;
        if fighting && (1..=2).contains(&round) {
            seen_in[round as usize - 1].push(m.round_decides_match());
        }
        if round > 2 {
            break;
        }
    }
    assert!(!seen_in[0].is_empty() && !seen_in[1].is_empty(), "two rounds fought: {round}");
    // (The round's win is counted at its knockout, a little before the
    // round stops being fought: only its opening is checked.)
    assert!(seen_in[0][..60].iter().all(|&d| !d), "the first round cannot end the match as it begins");
    assert!(seen_in[1].iter().all(|&d| d), "after a round won, the next could");
}

#[test]
fn it_techs_a_throw_only_from_what_it_has_seen() {
    // Player 1 walks in, stands a moment and throws. The stronger play
    // techs from a guard it holds against an opponent it has seen close: a
    // one-frame pulse of back and heavy punch (its own throw holds them two
    // frames). Every such pulse comes on a step whose view, as the CPU saw
    // it, showed him within a throw's reach: never from the engine's own
    // tech window, which the Utility AI read.
    let mut techs = 0u32;
    let mut unseen = Vec::new();
    // Pulses with the CPU on Player 1's left and on his right, and any
    // entered with the stick forward.
    let mut sides = [0u32; 2];
    let mut wrong_way = Vec::new();
    for mut m in starts(true) {
        let mut shown: HashMap<u32, Percept> = HashMap::new();
        let mut last_buttons = 0u8;
        // A press of back and heavy punch on the last step, and how far
        // apart the fighters were in the view it was decided from.
        let mut pressed: Option<(u32, i16, bool, bool)> = None;
        for i in 0..9_000u32 {
            let v = seen(&m);
            let f = toward(&v);
            let t = i % 60;
            let pad = if t < 40 && (v.foe.x - v.me.x).abs() > 40 {
                (f, 0)
            } else if (44..46).contains(&t) {
                (f, HP)
            } else {
                (0, 0)
            };
            step(&mut m, pad.0, pad.1);
            let frame = m.state.frame;
            let now = seen(&m);
            shown.insert(frame, now);
            let (dirs, buttons) = cpu_pad(&m);
            if let Some((at, dist, back, right)) = pressed.take() {
                // Held a second frame: its own throw, not a tech.
                if buttons & HP == 0 {
                    if !back {
                        wrong_way.push(at);
                    } else {
                        techs += 1;
                        sides[usize::from(right)] += 1;
                        if dist > TECH_REACH {
                            unseen.push((at, dist));
                        }
                    }
                }
            }
            // Away from him as the CPU's view shows the two (it turns as
            // that view does), or toward him: a one-frame pulse that way
            // would be a tech entered the wrong way round.
            if buttons == HP && (dirs == R || dirs == L) && last_buttons & HP == 0 {
                let view = as_seen(&shown, frame, m.fair_cpu_attention().unwrap());
                pressed = view.map(|view| {
                    let back = if view.foe.x >= view.me.x { L } else { R };
                    (frame, (view.foe.x - view.me.x).abs(), dirs == back, view.me.x > view.foe.x)
                });
            }
            last_buttons = buttons;
        }
    }
    if !roms_present() {
        return;
    }
    eprintln!("throw tech pulses {techs} (CPU on the left, right: {sides:?})");
    assert!(techs >= 10, "throw tech pulses: {techs}");
    assert!(sides.iter().all(|&n| n >= 3), "tech pulses on each side: {sides:?}");
    assert!(wrong_way.is_empty(), "tech pulses with the stick forward: {wrong_way:?}");
    assert!(unseen.is_empty(), "tech pulses with him out of a throw's reach as it saw him: {unseen:?}");
}
