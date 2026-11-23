//! Ticket #207 (`SC15`, spec #192): the stronger Fair CPU across the cast
//! in the SF2 era style.
//!
//! Each fight is a Versus mirror in the style, Player 2 played by the
//! stronger Fair CPU at Very Difficult and Player 1's pad scripted here
//! ([`sf_engine::measure::sparring_machine`]). What is checked is what the
//! frames show and what the CPU's state says it is doing. Local ROMs and
//! the art pack only (the cast beyond Ryu and Ken needs its player
//! tables); each test skips without them.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::sync::{Arc, OnceLock};

use sf_engine::data_bank::DataBank;
use sf_engine::era::{EraFamily, Loadout, Revision};
use sf_engine::fair_cpu::{perceive, Level, Percept, Tool};
use sf_engine::measure::{sparring_machine, MeasureSpec};
use sf_engine::roster::TournamentFighter;
use sf_engine::route_book::book;
use sf_engine::run::PowerOnMachine;
use sf_video::art::{load_art_from, ArtSources};
use sf_video::art_pack::ArtPack;
use sf_video::callouts::Side;
use sf_video::romset::load_assets;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const PACK: &str = "../../assets/sf1-port-art";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
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
const R: u8 = 1;
const L: u8 = 2;
const D: u8 = 4;
const U: u8 = 8;
const LP: u8 = 1;
const HP: u8 = 4;
const HK: u8 = 32;

/// The SF2 era style's revisions every tournament fighter plays: Hyper
/// Fighting and Super Turbo (the canon three have the earlier ones too).
const STYLES: [Revision; 2] = [Revision::Hf, Revision::St];

/// The fighters with game plans in the style. Gen and Eagle have none yet:
/// the measuring run found no set of the style's plans with which they
/// beat the old Fair CPU, and a follow-up builds their own game.
const PLANNED: [TournamentFighter; 10] = [
    TournamentFighter::Ryu,
    TournamentFighter::Ken,
    TournamentFighter::Retsu,
    TournamentFighter::Geki,
    TournamentFighter::Joe,
    TournamentFighter::Mike,
    TournamentFighter::Lee,
    TournamentFighter::Birdie,
    TournamentFighter::Adon,
    TournamentFighter::Sagat,
];

/// The ROM and the data bank with the cast's player tables, loaded once.
fn assets() -> Option<&'static (Arc<Vec<u8>>, Arc<DataBank>)> {
    static ASSETS: OnceLock<Option<(Arc<Vec<u8>>, Arc<DataBank>)>> = OnceLock::new();
    ASSETS
        .get_or_init(|| {
            if !Path::new(ROMS).is_dir() || !Path::new(PACK).is_dir() {
                eprintln!("skipping: local ROM set or art pack not present");
                return None;
            }
            let assets = load_assets(ROMS, MANIFEST).expect("local ROM set assembles");
            let pack = ArtPack::open(Path::new(PACK)).expect("the art pack opens");
            let art = load_art_from(&assets, &ArtSources { recipes: &RECIPES, reviews: Some(&REVIEWS), pack: Ok(&pack) }).expect("the art loads");
            let data = Arc::new(DataBank::build(&assets.maincpu, &art.catalogues));
            Some((Arc::new(assets.maincpu), data))
        })
        .as_ref()
}

/// A whole match is deep in the translated program's calls.
fn deep<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new().stack_size(256 << 20).spawn(f).unwrap().join().unwrap()
}

/// A mirror in `revision`: Player 1's pad is the test's, Player 2 the
/// Fair CPU, stronger or not, at Very Difficult.
fn sparring(fighter: TournamentFighter, revision: Revision, stronger: bool, seed: u64) -> Option<PowerOnMachine> {
    let (rom, data) = assets()?;
    let spec = MeasureSpec {
        fighters: [fighter; 2],
        styles: [(revision, Loadout::Standard); 2],
        difficulty: [4; 2],
        seed,
        stronger: [false, stronger],
    };
    Some(sparring_machine(rom, Some(data.clone()), &spec).expect("the fight starts"))
}

/// Matches one after another, each from a new seed, until `frames` have
/// been played: `each` steps the machine once a call. A match ends quickly
/// against a scripted Player 1, and the tests want many situations.
fn matches(fighter: TournamentFighter, revision: Revision, stronger: bool, seed: u64, frames: u32, each: impl FnMut(&mut PowerOnMachine, u32)) -> Option<()> {
    matches_with(fighter, revision, stronger, seed, frames, |_| {}, each)
}

/// The same, each machine set up by `setup` before its first frame.
fn matches_with(
    fighter: TournamentFighter,
    revision: Revision,
    stronger: bool,
    seed: u64,
    frames: u32,
    setup: impl Fn(&mut PowerOnMachine),
    mut each: impl FnMut(&mut PowerOnMachine, u32),
) -> Option<()> {
    let mut m = sparring(fighter, revision, stronger, seed)?;
    setup(&mut m);
    let mut next = seed;
    for i in 0..frames {
        each(&mut m, i);
        if m.match_over() {
            next += 1_000;
            m = sparring(fighter, revision, stronger, next)?;
            setup(&mut m);
        }
    }
    Some(())
}

fn step(m: &mut PowerOnMachine, (dirs, buttons): (u8, u8)) {
    m.set_p1_direction(dirs & R != 0, dirs & L != 0, dirs & D != 0, dirs & U != 0);
    m.set_p1_buttons(std::array::from_fn(|i| buttons & (1 << i) != 0));
    m.step();
}

/// The frame as Player 2's Fair CPU is shown it: `me` is the CPU's fighter.
fn seen(m: &PowerOnMachine) -> Percept {
    perceive(m.state.frame, true, m.work_ram(), &m.state.objectram, Side::P2)
}

fn toward(v: &Percept) -> u8 {
    if v.me.x >= v.foe.x { R } else { L }
}

fn away(v: &Percept) -> u8 {
    R + L - toward(v)
}

/// Player 1's pad for a sparring fight: a few hundred frames each of
/// walking in, standing, guarding low, sweeping, whiffing heavy punches,
/// jumping in, a projectile motion and backing off.
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

/// The sparring pad and the close guard in turns of 2,400 frames.
fn mixed_pad(i: u32, v: &Percept) -> (u8, u8) {
    if (i / 2_400) % 2 == 0 { sparring_pad(i, v) } else { guarding_pad(i, v) }
}

/// What one sparring fight showed of the CPU's play.
#[derive(Default)]
struct Seen {
    /// The situations the game plans it took are for.
    plan_situations: BTreeSet<String>,
    plans: u32,
    /// Routes entered.
    routes: u32,
    /// The tools whose step it was on, and the frames each was.
    tools: HashMap<String, u32>,
}

fn spar(fighter: TournamentFighter, revision: Revision, frames: u32, seed: u64, pad: fn(u32, &Percept) -> (u8, u8)) -> Option<Seen> {
    let mut out = Seen::default();
    let mut was_plan = None;
    let mut was_route = false;
    matches(fighter, revision, true, seed, frames, |m, i| {
        let v = seen(m);
        step(m, pad(i, &v));
        let cpu = m.fair_cpu().expect("Player 2 is the Fair CPU");
        let plan = cpu.game_plan().map(|(plan, _)| plan);
        if plan.is_some() && plan != was_plan {
            out.plans += 1;
            let situation = cpu.game_plan_situation(fighter, EraFamily::Sf2).expect("a plan of its table");
            out.plan_situations.insert(format!("{situation:?}"));
        }
        was_plan = plan;
        let route = cpu.entering_route();
        if route && !was_route {
            out.routes += 1;
        }
        was_route = route;
        if let Some(tool) = cpu.game_plan_tool(fighter, EraFamily::Sf2) {
            *out.tools.entry(format!("{tool:?}")).or_default() += 1;
        }
    })?;
    Some(out)
}

#[test]
fn every_fighter_in_the_sf2_style_takes_game_plans_in_each_situation_and_routes_from_its_book() {
    // The four situations Ryu's SF6 game has plans for: close and far
    // neutral, after a guarded attack and over a knocked-down opponent.
    // (A hit and a whiff are followed up with routes, the rest answered.)
    let want = ["NeutralClose", "NeutralFar", "ItWasBlocked", "OpponentKnockedDown"];
    for revision in STYLES {
        for fighter in PLANNED {
            // Two fights, pooled.
            let Some(seen) = deep(move || {
                let mut seen = spar(fighter, revision, 40_000, 1, mixed_pad)?;
                let more = spar(fighter, revision, 40_000, 2, mixed_pad)?;
                seen.plans += more.plans;
                seen.plan_situations.extend(more.plan_situations);
                seen.routes += more.routes;
                Some(seen)
            }) else {
                return;
            };
            eprintln!("{fighter:?} {revision:?}: {} plans in {:?}, {} routes", seen.plans, seen.plan_situations, seen.routes);
            for situation in want {
                assert!(seen.plan_situations.contains(situation), "{fighter:?} {revision:?} took no plan in {situation}: {:?}", seen.plan_situations);
            }
            // A book whose every route spends the super (Mike's Super Turbo
            // one) is entered with a full bar: the super test shows it.
            let meterless = book(fighter, revision, Loadout::Standard).is_some_and(|b| b.routes.iter().any(|r| r.super_spent == 0));
            if meterless {
                assert!(seen.routes >= 1, "{fighter:?} {revision:?} entered {} routes from its book", seen.routes);
            }
        }
    }
}

#[test]
fn gen_and_eagle_take_no_game_plan_in_the_sf2_style_yet() {
    // Listed on #207 as not passing: with the switch on they play the
    // stronger play's single choices, as before this ticket.
    for fighter in [TournamentFighter::Gen, TournamentFighter::Eagle] {
        let Some(seen) = deep(move || spar(fighter, Revision::St, 4_800, 1, sparring_pad)) else { return };
        assert_eq!(seen.plans, 0, "{fighter:?} took a game plan");
        assert!(seen.tools.is_empty(), "{fighter:?} used a tool step");
    }
}

#[test]
fn with_the_switch_off_no_sf2_fighter_judges_a_situation_follows_a_plan_or_uses_a_tool() {
    // What the switch-off CPU presses is checked against the build before
    // this ticket by the measuring run's old-against-old rows (#207's
    // notes); here, that none of the new machinery runs.
    for fighter in TournamentFighter::ALL {
        let Some(()) = deep(move || {
            let mut m = sparring(fighter, Revision::St, false, 1)?;
            for i in 0..2_400u32 {
                let v = seen(&m);
                step(&mut m, sparring_pad(i, &v));
                let cpu = m.fair_cpu().unwrap();
                assert_eq!(cpu.situation(), Default::default(), "{fighter:?} frame {i}");
                assert_eq!(cpu.game_plan(), None, "{fighter:?} frame {i}");
                assert!(!cpu.entering_route() || cpu.route_reset().is_none(), "{fighter:?} frame {i}");
            }
            Some(())
        }) else {
            return;
        };
    }
}

/// Player 1 walks in to close range and guards low there, now and then
/// standing: a guarding opponent up close is what a command throw is for.
fn guarding_pad(i: u32, v: &Percept) -> (u8, u8) {
    if (v.me.x - v.foe.x).abs() > 60 {
        (toward(v), 0)
    } else if i % 200 < 40 {
        (0, 0)
    } else {
        (D | away(v), 0)
    }
}

#[test]
fn fighters_use_their_own_command_throws_teleports_and_charge_moves() {
    // Retsu, Geki and Birdie throw with their command throws, Geki
    // teleports, and Birdie and Lee charge: each tool's step comes, and the
    // fighter is seen in a special move on it. (Gen's charge moves and
    // stance wait for his own game, with Eagle's.)
    // Each tool with the move id its fighter is drawn in for it, as the
    // what-if kits number them (`era_fight::what_if`).
    let cases: [(TournamentFighter, &'static [(Tool, u8)]); 4] = [
        (TournamentFighter::Retsu, &[(Tool::CommandThrow, 0x21)]),
        (TournamentFighter::Geki, &[(Tool::CommandThrow, 0x20), (Tool::Teleport, 0x1b)]),
        (TournamentFighter::Birdie, &[(Tool::CommandThrow, 0x1b), (Tool::Charge, 0x19)]),
        (TournamentFighter::Lee, &[(Tool::Charge, 0x20)]),
    ];
    for (fighter, tools) in cases {
        let Some(used) = deep(move || tool_moves(fighter, Revision::St, tools)) else { return };
        eprintln!("{fighter:?}: {used:?}");
        for (tool, _) in tools {
            let ToolUse { steps, came, grabbed } = used.get(tool).copied().unwrap_or_default();
            assert!(steps >= 3, "{fighter:?} took its {tool:?} {steps} times: {used:?}");
            // A step can end before its move: he steps out of a command
            // throw's range, or attacks into a charge's walk back.
            assert!(came * 2 >= steps, "{fighter:?} took its {tool:?} {steps} times and it came out {came}: {used:?}");
            if *tool == Tool::CommandThrow {
                // And it grabs him: he is drawn being thrown.
                assert!(grabbed * 3 >= came && grabbed >= 2, "{fighter:?}'s command throw came out {came} times and grabbed {grabbed}: {used:?}");
            }
        }
    }
}

/// What came of the steps that use one tool.
#[derive(Clone, Copy, Debug, Default)]
struct ToolUse {
    /// Steps taken.
    steps: u32,
    /// Of those, the ones its fighter was seen in the tool's own move
    /// (`moves`) within 60 frames.
    came: u32,
    /// Of those, the ones Player 1 was seen being thrown in (a command
    /// throw that grabbed).
    grabbed: u32,
}

/// A fighter being thrown, as he is drawn (`combo::throw::THROWN_MOVE`).
const THROWN_MOVE: u8 = 0x3e;

fn tool_moves(fighter: TournamentFighter, revision: Revision, moves: &'static [(Tool, u8)]) -> Option<HashMap<Tool, ToolUse>> {
    let mut used: HashMap<Tool, ToolUse> = HashMap::new();
    for (seed, pad) in [(1, guarding_pad as fn(u32, &Percept) -> (u8, u8)), (2, sparring_pad)] {
        // The tool of the step taken, the frame, and whether its move came
        // out and he was thrown since.
        let mut on: Option<(Tool, u32, bool, bool)> = None;
        let close = |used: &mut HashMap<Tool, ToolUse>, on: Option<(Tool, u32, bool, bool)>| {
            if let Some((tool, _, came, grabbed)) = on {
                let entry = used.entry(tool).or_default();
                entry.came += u32::from(came);
                entry.grabbed += u32::from(came && grabbed);
            }
        };
        matches(fighter, revision, true, seed, 30_000, |m, i| {
            let v = seen(m);
            step(m, pad(i, &v));
            let tool = m.fair_cpu().unwrap().game_plan_tool(fighter, EraFamily::Sf2);
            if let Some(tool) = tool.filter(|t| on.is_none_or(|(was, at, _, _)| *t != was || i > at + 60)) {
                close(&mut used, on.take());
                used.entry(tool).or_default().steps += 1;
                on = Some((tool, i, false, false));
            }
            let now = seen(m);
            if let Some((tool, at, came, grabbed)) = on.as_mut() {
                let id = moves.iter().find(|(t, _)| t == tool).map(|(_, id)| *id);
                if i <= *at + 60 {
                    *came |= Some(now.me.move_id) == id;
                    *grabbed |= *came && now.foe.move_id == THROWN_MOVE;
                }
            }
        })?;
        close(&mut used, on);
    }
    Some(used)
}

#[test]
fn the_sf2_style_cast_answers_nothing_sooner_than_its_reaction_delay() {
    // Two copies of one fight part on a frame where the fighters stand far
    // apart: in one Player 1 jumps in, in the other he stands. However the
    // CPU's game plans and tools play, its pad is the same in both until
    // its focused delay has passed; and over the partings, the jump does
    // change it after that.
    let delay = Level::VeryHard.reaction();
    for fighter in TournamentFighter::ALL {
        let Some((parted, changed)) = deep(move || {
            let (mut parted, mut changed) = (0u32, 0u32);
            let mut next = 300u32;
            matches(fighter, Revision::St, true, 3, 12_000, |m, i| {
                let v = seen(m);
                let far = (v.me.x - v.foe.x).abs() > 150 && v.me.free() && v.foe.free() && v.fighting;
                if i >= next && far {
                    next = i + 400;
                    parted += 1;
                    let (mut jumped, mut stood) = (m.clone(), m.clone());
                    let mut differed = false;
                    for k in 0..delay + 30 {
                        let towards = toward(&seen(&jumped));
                        step(&mut jumped, if k < 3 { (U | towards, 0) } else { (0, 0) });
                        step(&mut stood, (0, 0));
                        let same = jumped.fair_cpu_pad() == stood.fair_cpu_pad();
                        if k < delay {
                            assert!(same, "{fighter:?}: its pad changed {k} frames after the jump, inside its {delay}-frame delay");
                        } else if !same {
                            differed = true;
                        }
                    }
                    changed += u32::from(differed);
                }
                step(m, sparring_pad(i, &v));
            })?;
            Some((parted, changed))
        }) else {
            return;
        };
        eprintln!("{fighter:?}: {parted} partings, {changed} changed after the delay");
        assert!(parted >= 3, "{fighter:?}: the fight parted only {parted} times");
        assert!(changed >= 1, "{fighter:?}: the jump never changed its pad, so the test checked nothing");
    }
}

/// Player 1 pokes from just inside reach: openings to hit and to punish.
fn poking(i: u32, v: &Percept) -> (u8, u8) {
    match i % 90 {
        0..=2 => (D, LP),
        30..=33 => (0, HP),
        60..=79 => (toward(v), 0),
        _ => (0, 0),
    }
}

/// The super's move as a fighter is drawn in it (`0x29`, and the what-if
/// kits' finishers up to `0x2b`).
const SUPERS: std::ops::RangeInclusive<u8> = 0x29..=0x2b;

#[test]
fn in_super_turbo_a_super_is_spent_inside_a_route() {
    // With its Super meter kept full it takes routes its book records as
    // spending the super, and the super's move is seen to begin while it
    // enters one, for most fighters whose Super Turbo book has such routes.
    let mut checked = 0;
    for fighter in PLANNED {
        let Some(st) = book(fighter, Revision::St, Loadout::Standard) else { continue };
        if !st.routes.iter().any(|r| r.super_spent > 0) {
            continue;
        }
        let Some((routes, supers)) = deep(move || {
            let book = book(fighter, Revision::St, Loadout::Standard).unwrap();
            let (mut routes, mut supers) = (0u32, 0u32);
            let (mut in_super_route, mut since) = (false, u32::MAX);
            let mut was = 0u8;
            // Training's FILL METERS keeps its Super meter full, so the
            // round state lets it spend.
            let fill = |m: &mut PowerOnMachine| {
                let mut config = *m.match_config();
                config.roster.mode = sf_engine::roster::PlayMode::Training;
                config.fill_meters = true;
                m.set_match_config(config);
            };
            matches_with(fighter, Revision::St, true, 5, 20_000, fill, |m, i| {
                let v = seen(m);
                step(m, poking(i, &v));
                let cpu = m.fair_cpu().unwrap();
                let index = cpu.route_index().filter(|_| cpu.entering_route());
                let super_route = index.is_some_and(|index| book.routes[index].super_spent > 0);
                routes += u32::from(super_route && !in_super_route);
                in_super_route = super_route;
                since = if super_route { 0 } else { since.saturating_add(1) };
                let me = seen(m).me.move_id;
                // Begun while the route runs, or as its last pads end it.
                supers += u32::from(SUPERS.contains(&me) && !SUPERS.contains(&was) && since <= 20);
                was = me;
            })?;
            Some((routes, supers))
        }) else {
            return;
        };
        eprintln!("{fighter:?}: {routes} routes into the super, {supers} supers in them");
        if supers > 0 {
            checked += 1;
        }
    }
    // Two thirds of the fighters whose books have such routes: a route
    // into the super is one of several it picks from, so a fighter can go
    // a fight without one.
    let with_super_routes = PLANNED
        .iter()
        .filter(|&&f| book(f, Revision::St, Loadout::Standard).is_some_and(|b| b.routes.iter().any(|r| r.super_spent > 0)))
        .count();
    assert!(checked * 3 >= with_super_routes * 2, "only {checked} of {with_super_routes} fighters spent a super inside a route");
}

/// Player 1 stands close and presses a crouching heavy punch over and over:
/// an attack the CPU guards, recovering in its reach each time.
fn pressing_pad(i: u32, v: &Percept) -> (u8, u8) {
    if (v.me.x - v.foe.x).abs() > 70 {
        (toward(v), 0)
    } else if i % 40 < 3 {
        (D, HP)
    } else {
        (D, 0)
    }
}

/// Counts, over frames of a fight against [`pressing_pad`], the times the
/// CPU's fighter was seen guarding one of Player 1's attacks, and the times
/// it then started one of its own before his next began.
#[derive(Default)]
struct Answers {
    guarded: u32,
    answered: u32,
    pending: bool,
}

impl Answers {
    fn see(&mut self, before: &Percept, v: &Percept) {
        if v.foe.attacking() && (!before.foe.attacking() || before.foe.move_id != v.foe.move_id) {
            self.pending = false;
        }
        // Guarded: pushed back in its guard pose by his attack (the pose
        // alone is drawn as soon as an attack is near).
        let pushed = (v.me.x - v.foe.x).abs() > (before.me.x - before.foe.x).abs();
        if v.foe.attacking() && v.me.guarding() && before.me.guarding() && pushed && !self.pending {
            self.pending = true;
            self.guarded += 1;
        }
        if self.pending && v.me.attacking() && (!before.me.attacking() || before.me.move_id != v.me.move_id) {
            self.answered += 1;
            self.pending = false;
        }
    }
}

fn answers(revision: Revision) -> Option<Answers> {
    let mut out = Answers::default();
    matches(TournamentFighter::Ryu, revision, true, 9, 12_000, |m, i| {
        let before = seen(m);
        step(m, pressing_pad(i, &before));
        out.see(&before, &seen(m));
    })?;
    Some(out)
}

#[test]
fn in_the_sf2_style_a_guarded_attack_recovering_in_reach_is_answered() {
    // SC15 (`#207`): his crouching heavy punch shows no attack box the CPU
    // can read, so it is known as guarded by the CPU's own guard. Out of
    // its blockstun, the CPU takes its turn with a punish route or a
    // crouching light punch, on at least half of them.
    let Some(sf2) = deep(|| answers(Revision::Hf)) else { return };
    eprintln!("SF2: {} of {} guarded attacks answered", sf2.answered, sf2.guarded);
    assert!(sf2.guarded >= 20, "he was guarded only {} times", sf2.guarded);
    assert!(sf2.answered * 2 >= sf2.guarded, "it answered {} of {} guarded attacks", sf2.answered, sf2.guarded);
}

#[test]
fn in_the_sf2_style_it_shimmies_rather_than_standing_over_a_knocked_down_opponent() {
    // SC15 (`#207`): waiting in reach to time an attack to his rise, the
    // CPU's fighter steps in and out; its stick is left still on few of
    // those frames.
    for fighter in [TournamentFighter::Ryu, TournamentFighter::Lee, TournamentFighter::Mike] {
        let Some((waiting, still)) = deep(move || {
            let (mut waiting, mut still) = (0u32, 0u32);
            matches(fighter, Revision::Hf, true, 4, 60_000, |m, i| {
                let before = seen(m);
                // Player 1 stands and now and then whiffs: he is knocked
                // down often, and does not press out of it.
                step(m, if i % 90 < 3 { (0, HP) } else { (0, 0) });
                let v = seen(m);
                let cpu = m.fair_cpu().unwrap();
                let meaty_wait = cpu.game_plan_situation(fighter, EraFamily::Sf2) == Some(sf_engine::fair_cpu::Situation::OpponentKnockedDown)
                    && cpu.game_plan().is_some_and(|(_, step)| step == 0)
                    && v.me.free()
                    && before.me.free()
                    && v.foe.move_id >= 0x30;
                if meaty_wait {
                    waiting += 1;
                    // Its stick held nowhere: standing. (A walk can leave
                    // its place unchanged on a frame, slow or against his
                    // body.)
                    let (stick, _) = m.fair_cpu_pad().unwrap();
                    still += u32::from(!stick[0] && !stick[1]);
                }
            })?;
            Some((waiting, still))
        }) else {
            return;
        };
        eprintln!("{fighter:?}: {still} of {waiting} frames waiting over him stood still");
        assert!(waiting >= 200, "{fighter:?} waited over him only {waiting} frames");
        assert!(still * 3 <= waiting, "{fighter:?} stood still on {still} of {waiting} frames");
    }
}

/// Player 1 keeps just outside reach and swings a crouching heavy punch
/// at the air: the CPU's guard pose shows as each comes near, but nothing
/// meets it.
fn swinging_short_pad(i: u32, v: &Percept) -> (u8, u8) {
    let dist = (v.me.x - v.foe.x).abs();
    if dist < 74 {
        (away(v), 0)
    } else if dist > 92 {
        (toward(v), 0)
    } else if i % 24 < 3 {
        (D, HP)
    } else {
        (D, 0)
    }
}

#[test]
fn in_the_sf2_style_attacks_that_never_meet_it_are_not_answered_as_guarded() {
    // SC15 (`#207`, review): its guard pose shows as soon as an attack is
    // near, so the answer to a guarded attack waits for one that put it in
    // blockstun. Against swings that never meet it, it starts no attack of
    // its own, while guarding and defending, in their first frames. (Under
    // a real string it can: it decides from the frame its delay shows, the
    // last attack recovering, and he may press again first. The trigger
    // itself is unit-tested in `fair_cpu::game`.)
    for fighter in [TournamentFighter::Ryu, TournamentFighter::Retsu, TournamentFighter::Adon] {
        let Some((swings, guarded, into)) = deep(move || {
            let (mut swings, mut guarded, mut into) = (0u32, 0u32, 0u32);
            // Frames since his current attack began, and whether it or the
            // one before it pushed the CPU back or hit it.
            let (mut since, mut met, mut met_before) = (u32::MAX, false, false);
            matches(fighter, Revision::Hf, true, 6, 30_000, |m, i| {
                let before = seen(m);
                step(m, swinging_short_pad(i, &before));
                let v = seen(m);
                if v.foe.attacking() && (!before.foe.attacking() || before.foe.move_id != v.foe.move_id) {
                    swings += 1;
                    since = 0;
                    met_before = met;
                    met = false;
                } else {
                    since = since.saturating_add(1);
                }
                let pushed = v.me.guarding() && v.foe.attacking() && (v.me.x - v.foe.x).abs() > (before.me.x - before.foe.x).abs();
                met |= pushed || v.me.reacting();
                guarded += u32::from(v.foe.attacking() && v.me.guarding() && !met && !met_before);
                let cpu_began = v.me.attacking() && (!before.me.attacking() || before.me.move_id != v.me.move_id);
                // Begun while it judged itself defending: where the answer
                // to a guarded attack comes from (a punish of his last whiff
                // is chosen as he recovers, before the next swing shows).
                let defending = m.fair_cpu().unwrap().situation().situation == sf_engine::fair_cpu::Situation::Defending;
                into += u32::from(cpu_began && since < 6 && before.me.guarding() && defending && !met && !met_before);
            })?;
            Some((swings, guarded, into))
        }) else {
            return;
        };
        eprintln!("{fighter:?}: {swings} swings, {guarded} frames in its guard pose against ones that never met it, {into} attacks into their start");
        assert!(swings >= 30 && guarded >= 30, "{fighter:?}: {swings} swings, {guarded} guard-pose frames");
        assert_eq!(into, 0, "{fighter:?} attacked into the start of {into} swings that had not met it");
    }
}
