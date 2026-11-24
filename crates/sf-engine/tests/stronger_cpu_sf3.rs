//! SC17 (#209): the stronger Fair CPU playing the SF3 era style in real
//! fights. Local ROMs and the art pack supply the tournament cast's player
//! tables, as in the measuring run; tests skip when those assets are absent.

use std::path::Path;
use std::sync::{Arc, OnceLock};

use sf_engine::data_bank::DataBank;
use sf_engine::era::{EraFamily, Loadout, Revision};
use sf_engine::fair_cpu::{perceive, EraAct, Level, Percept, Tool};
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
const HP: u8 = 4;
const HK: u8 = 32;
const PARRY: u8 = 0x2e;
const OVERHEAD: u8 = 0x2f;
const THROWN: u8 = 0x3e;
const ARTS: [Loadout; 3] = [Loadout::Art1, Loadout::Art2, Loadout::Art3];

fn data() -> Option<&'static (Vec<u8>, Arc<DataBank>)> {
    static DATA: OnceLock<Option<(Vec<u8>, Arc<DataBank>)>> = OnceLock::new();
    DATA.get_or_init(|| {
        if !Path::new(ROMS).is_dir() || !Path::new(PACK).is_dir() {
            eprintln!("skipping: local ROM set or art pack not present");
            return None;
        }
        let assets = load_assets(ROMS, MANIFEST).expect("local ROM set assembles");
        let pack = ArtPack::open(Path::new(PACK)).expect("the art pack opens");
        let art = load_art_from(&assets, &ArtSources { recipes: &RECIPES, reviews: Some(&REVIEWS), pack: Ok(&pack) }).expect("the art loads");
        Some((assets.maincpu.clone(), Arc::new(DataBank::build(&assets.maincpu, &art.catalogues))))
    }).as_ref()
}

fn deep<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new().stack_size(256 << 20).spawn(f).unwrap().join().unwrap()
}

#[derive(Clone, Copy, Debug)]
struct Style {
    fighter: TournamentFighter,
    revision: Revision,
    art: Loadout,
}

impl Style {
    fn third_strike(fighter: TournamentFighter, art: Loadout) -> Self {
        Self { fighter, revision: Revision::Sf3ThreeS, art }
    }
}

fn fight(style: Style, stronger: bool, level: u8, seed: u64) -> Option<PowerOnMachine> {
    let (rom, bank) = data()?;
    let spec = MeasureSpec {
        fighters: [TournamentFighter::Ryu, style.fighter],
        styles: [(Revision::Sf3ThreeS, Loadout::Art1), (style.revision, style.art)],
        difficulty: [level; 2],
        seed,
        stronger: [false, stronger],
    };
    Some(sparring_machine(rom, Some(bank.clone()), &spec).expect("the match starts"))
}

fn step(m: &mut PowerOnMachine, (dirs, buttons): (u8, u8)) {
    m.set_p1_direction(dirs & R != 0, dirs & L != 0, dirs & D != 0, dirs & U != 0);
    m.set_p1_buttons(std::array::from_fn(|i| buttons & (1 << i) != 0));
    m.step();
}

fn seen(m: &PowerOnMachine) -> Percept {
    perceive(m.state.frame, true, m.work_ram(), &m.state.objectram, Side::P2)
}

fn toward(v: &Percept) -> u8 {
    if v.me.x >= v.foe.x { R } else { L }
}

fn away(v: &Percept) -> u8 {
    R + L - toward(v)
}

/// Walking, guarding, whiffing, jumping and pressing attacks present real
/// opportunities for the CPU's different situations and route openers.
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
        6 => if t % 24 < 3 { (toward(v), HP) } else { (toward(v), 0) },
        _ => (away(v), 0),
    }
}

fn guarding_pad(_: u32, v: &Percept) -> (u8, u8) {
    if (v.me.x - v.foe.x).abs() > 38 { (toward(v), 0) } else { (D | away(v), 0) }
}

/// Ground attacks with enough start-up to offer a delayed parry, mixed
/// with low attacks, rather than a test that enters the CPU's parry pad.
fn pressing_pad(i: u32, v: &Percept) -> (u8, u8) {
    if (v.me.x - v.foe.x).abs() > 55 {
        (toward(v), 0)
    } else {
        match i % 100 {
            0..=2 => (0, HP),
            50..=52 => (D, HK),
            _ => (0, 0),
        }
    }
}

/// Leave an opening for the selected Super Art route. Ryu Art2 has one
/// such ground route, whose opener needs a closer gap than his EX routes;
/// repeated pokes can interrupt the walk in before that opener is tried.
fn standing_pad(_: u32, _: &Percept) -> (u8, u8) {
    (0, 0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OwnStep {
    Tool(Tool),
    Era(EraAct),
}

fn own_step(m: &PowerOnMachine, fighter: TournamentFighter) -> Option<OwnStep> {
    let cpu = m.fair_cpu().expect("Player 2 is the Fair CPU");
    cpu.game_plan_tool(fighter, EraFamily::Sf3).map(OwnStep::Tool)
        .or_else(|| cpu.game_plan_era(fighter, EraFamily::Sf3).map(OwnStep::Era))
}

#[derive(Default, Debug)]
struct Seen {
    frames: u32,
    plan_frames: u32,
    routes: u32,
    foreign_routes: u32,
    /// Move starts belonging to a game plan's tool or era step.
    began_on_step: Vec<(u8, OwnStep)>,
    caught_on_step: Vec<OwnStep>,
    to_crane: u32,
    to_mantis: u32,
    parries: u32,
    overhead_hits: u32,
    /// Real EX starts from Enhanced steps, with meter immediately before
    /// and after its engine command is accepted.
    enhanced: Vec<(u8, u8, u8)>,
    enhanced_step_frames: u32,
    /// A native EX slot together with the actual 40-point spend. The ROM
    /// also uses some of these move IDs for non-EX states.
    native_ex_spends: u32,
    super_routes: u32,
    supers_in_routes: u32,
    wrong_art: u32,
}

/// Run complete matches until the fighting-frame budget is spent. `meter`
/// is an explicit test supply, applied before each frame as Training's
/// Fill meters would; recording the meter on both sides of the engine
/// step still proves that the command pays its actual cost.
fn watch(style: Style, stronger: bool, frames: u32, meter: Option<u8>, pad: fn(u32, &Percept) -> (u8, u8)) -> Option<Seen> {
    data()?;
    Some(deep(move || {
        let mut s = Seen::default();
        let routes = book(style.fighter, style.revision, style.art);
        let route_count = routes.map_or(0, |b| b.routes.len());
        let art_slot = match style.art { Loadout::Art1 => 0x29, Loadout::Art2 => 0x2a, Loadout::Art3 => 0x2b, _ => unreachable!() };
        let mut seed = 1;
        while s.frames < frames {
            let mut m = fight(style, stronger, 4, seed).unwrap();
            seed += 1;
            let mut entering = false;
            let mut in_super_route = false;
            let mut last_step = None;
            let mut pending_grab: Option<(OwnStep, u32)> = None;
            let mut overhead_from_step = false;
            for i in 0..36_000u32 {
                let before = seen(&m);
                if let Some(points) = meter.filter(|_| before.fighting) {
                    m.set_era_super_meter(1, points);
                }
                let before_era = m.era_fighters()[1];
                let before_foe_hp = m.era_fighters()[0].sub_hp;
                let step_before = own_step(&m, style.fighter);
                let cpu_before = m.fair_cpu().unwrap();
                let route_step_before = cpu_before.route_index().zip(cpu_before.route_step());
                let opponent_pad = pad(i, &before);
                step(&mut m, opponent_pad);
                let now = seen(&m);
                if m.match_over() { break; }
                if !now.fighting {
                    entering = false;
                    in_super_route = false;
                    last_step = None;
                    pending_grab = None;
                    overhead_from_step = false;
                    continue;
                }
                s.frames += 1;
                s.plan_frames += u32::from(m.fair_cpu_following_game_plan());
                let cpu = m.fair_cpu().unwrap();
                let route = cpu.route_index().filter(|_| cpu.entering_route());
                let route_step_after = cpu.route_index().zip(cpu.route_step());
                if route.is_some() && !entering {
                    s.routes += 1;
                    s.foreign_routes += u32::from(route.is_none_or(|index| index >= route_count));
                }
                entering = route.is_some();
                // A double-quarter-circle step, rather than any meter
                // spend: EX routes also spend Super meter in this style.
                let super_route = route.and_then(|index| routes.and_then(|b| b.routes.get(index)))
                    .is_some_and(|r| r.steps.iter().any(|step| step.name.starts_with("236236+")));
                s.super_routes += u32::from(super_route && !in_super_route);
                in_super_route = super_route;
                // Both observations must identify the same active route
                // and its Super Art command step. A later reactive super
                // after a cancelled route cannot claim this attribution.
                let entering_super_art = route_step_before == route_step_after
                    && route_step_before.and_then(|(index, step)| routes
                        .and_then(|b| b.routes.get(index))
                        .and_then(|r| r.steps.get(step)))
                        .is_some_and(|step| step.name.starts_with("236236+"));
                let on = own_step(&m, style.fighter).or(step_before).or(last_step);
                s.enhanced_step_frames += u32::from(on == Some(OwnStep::Era(EraAct::Enhanced)));
                let era = m.era_fighters()[1];
                let began = now.me.move_id != before.me.move_id;
                if began {
                    if let Some(on) = on {
                        s.began_on_step.push((now.me.move_id, on));
                        if on == OwnStep::Tool(Tool::CommandThrow) && now.me.attacking() {
                            pending_grab = Some((on, i));
                        }
                    }
                    overhead_from_step = now.me.move_id == OVERHEAD && on == Some(OwnStep::Era(EraAct::Overhead));
                    s.parries += u32::from(now.me.move_id == PARRY && era.sub_hp == before_era.sub_hp);
                    let native_ex = matches!(now.me.move_id, 0x21 | 0x24..=0x26 | 0x2c) && style.fighter.is_shoto();
                    s.native_ex_spends += u32::from(native_ex && before_era.super_meter.saturating_sub(era.super_meter) >= 40);
                    if native_ex && on == Some(OwnStep::Era(EraAct::Enhanced)) && before_era.super_meter > era.super_meter {
                        s.enhanced.push((now.me.move_id, before_era.super_meter, era.super_meter));
                    }
                    if (0x29..=0x2b).contains(&now.me.move_id) {
                        s.wrong_art += u32::from(now.me.move_id != art_slot);
                        s.supers_in_routes += u32::from(now.me.move_id == art_slot && entering_super_art && before_era.super_meter > era.super_meter);
                    }
                }
                if let Some((on, from)) = pending_grab {
                    if now.foe.move_id == THROWN {
                        s.caught_on_step.push(on);
                        pending_grab = None;
                    } else if i > from + 90 {
                        pending_grab = None;
                    }
                }
                // The overhead bypasses low guard without first putting
                // him in the guarding animation. Check the actual held
                // down-back pad and his pre-impact crouch instead: damage
                // while this partner is still walking in does not count.
                let crouch_guard = opponent_pad == (D | away(&before), 0) && before.foe.crouching();
                if overhead_from_step && now.me.move_id == OVERHEAD && crouch_guard && m.era_fighters()[0].sub_hp < before_foe_hp && now.foe.reacting() && !now.foe.guarding() {
                    s.overhead_hits += 1;
                }
                if before.fighting && era.in_crane() != before_era.in_crane() {
                    if era.in_crane() { s.to_crane += 1 } else { s.to_mantis += 1 }
                }
                last_step = own_step(&m, style.fighter);
                if s.frames >= frames { break; }
            }
        }
        s
    }))
}

#[test]
fn every_sf3_fighter_and_art_follows_game_plans_and_enters_its_own_routes() {
    for fighter in TournamentFighter::ALL {
        for art in ARTS {
            let style = Style::third_strike(fighter, art);
            let Some(s) = watch(style, true, 12_000, None, sparring_pad) else { return };
            eprintln!("{style:?}: {} of {} frames in plans, {} routes", s.plan_frames, s.frames, s.routes);
            assert!(s.plan_frames * 10 >= s.frames, "{style:?} follows game plans: {s:?}");
            assert!(s.routes >= 1, "{style:?} enters its route book: {s:?}");
            assert_eq!(s.foreign_routes, 0, "{style:?}: every route belongs to this fighter and selected art");
        }
    }
}

#[test]
fn the_earlier_sf3_revisions_use_the_same_planning_and_their_own_route_books() {
    for revision in [Revision::Sf3Ng, Revision::Sf3TwoI] {
        for fighter in [TournamentFighter::Ryu, TournamentFighter::Ken] {
            for art in ARTS {
                let style = Style { fighter, revision, art };
                let Some(s) = watch(style, true, 12_000, None, sparring_pad) else { return };
                assert!(s.plan_frames * 10 >= s.frames && s.routes >= 1, "{style:?}: {s:?}");
                assert_eq!(s.foreign_routes, 0, "{style:?}");
            }
        }
    }
}

#[test]
fn with_the_switch_off_no_sf3_fighter_follows_a_game_plan() {
    for fighter in TournamentFighter::ALL {
        let style = Style::third_strike(fighter, Loadout::Art2);
        let Some(s) = watch(style, false, 4_000, None, sparring_pad) else { return };
        assert_eq!(s.plan_frames, 0, "{style:?}");
        assert!(s.began_on_step.is_empty(), "{style:?}: no stronger-only tool or era step");
    }
}

#[test]
fn the_sf3_fighters_use_and_land_their_own_tools_from_game_plans() {
    use TournamentFighter::{Birdie, Gen, Geki, Lee, Retsu};
    let grab = OwnStep::Tool(Tool::CommandThrow);
    let charge = OwnStep::Tool(Tool::Charge);
    let cases: [(TournamentFighter, &[(u8, OwnStep)]); 5] = [
        (Retsu, &[(0x21, grab)]),
        (Geki, &[(0x20, grab), (0x1b, OwnStep::Tool(Tool::Teleport))]),
        (Birdie, &[(0x1b, grab), (0x19, charge)]),
        (Lee, &[(0x20, charge)]),
        (Gen, &[(0x1f, OwnStep::Era(EraAct::Stance)), (0x20, charge)]),
    ];
    for (fighter, tools) in cases {
        let style = Style::third_strike(fighter, Loadout::Art1);
        let Some(s) = watch(style, true, 30_000, None, sparring_pad) else { return };
        for &(id, on) in tools {
            let started = s.began_on_step.iter().filter(|&&(move_id, step)| move_id == id && step == on).count();
            eprintln!("{fighter:?}: move {id:x} from {on:?} started {started} times");
            assert!(s.began_on_step.contains(&(id, on)), "{fighter:?}: move {id:x} comes from its {on:?} step");
            if on == grab {
                eprintln!("{fighter:?}: {} command throws caught", s.caught_on_step.len());
                assert!(s.caught_on_step.contains(&grab), "{fighter:?}: its command throw actually catches him");
            }
        }
        if fighter == Gen {
            assert!(s.to_crane >= 1 && s.to_mantis == 0, "Gen stays in the Crane after changing to it: {s:?}");
        }
    }
}

#[test]
fn the_sf3_overhead_hits_crouch_guard_and_an_art2_ex_spends_only_forty_points() {
    let style = Style::third_strike(TournamentFighter::Ryu, Loadout::Art2);
    let Some(s) = watch(style, true, 30_000, Some(40), guarding_pad) else { return };
    eprintln!("overhead hits {}, EX starts {}", s.overhead_hits, s.enhanced.len());
    assert!(s.began_on_step.contains(&(OVERHEAD, OwnStep::Era(EraAct::Overhead))), "the overhead comes from its plan step");
    assert!(s.overhead_hits >= 1, "the overhead hits a crouching guard");
    let Some(ex) = watch(style, true, 30_000, Some(40), sparring_pad) else { return };
    eprintln!("EX starts with a varied partner {}", ex.enhanced.len());
    assert!(!ex.enhanced.is_empty(), "an EX starts from its plan step with only 40 points, below Art2's 128-point Super Art stock");
    assert!(ex.enhanced.iter().all(|&(_, before, after)| before == 40 && after == 0), "native EX spends exactly 40: {:?}", ex.enhanced);
}

#[test]
fn new_generation_does_not_attempt_the_later_revisions_ex_moves() {
    for fighter in [TournamentFighter::Ryu, TournamentFighter::Ken] {
        let style = Style { fighter, revision: Revision::Sf3Ng, art: Loadout::Art2 };
        let Some(s) = watch(style, true, 8_000, Some(40), sparring_pad) else { return };
        assert_eq!(s.native_ex_spends, 0, "{style:?}: New Generation has no EX commands");
        assert_eq!(s.enhanced_step_frames, 0, "{style:?}: no unavailable Enhanced step is attempted, even if it produces no move");
    }
}

#[test]
fn selected_super_arts_are_spent_inside_real_routes() {
    for (fighter, art) in [(TournamentFighter::Ryu, Loadout::Art2), (TournamentFighter::Ken, Loadout::Art1), (TournamentFighter::Ken, Loadout::Art3)] {
        let style = Style::third_strike(fighter, art);
        let b = book(fighter, style.revision, art).expect("a checked-in SF3 route book");
        assert!(b.routes.iter().any(|r| r.steps.iter().any(|s| s.name.starts_with("236236+"))), "{style:?} has a proven Super Art route");
        let Some(s) = watch(style, true, 30_000, Some(u8::MAX), standing_pad) else { return };
        eprintln!("{style:?}: {} Super Art routes, {} Super Art starts in them", s.super_routes, s.supers_in_routes);
        assert!(s.super_routes >= 1 && s.supers_in_routes >= 1, "{style:?} actually starts its selected Super Art while entering a route: {s:?}");
        assert_eq!(s.wrong_art, 0, "{style:?}: never uses an unselected Super Art");
    }
}

#[test]
fn the_sf3_cpu_actually_parries_attacks_in_fights() {
    let style = Style::third_strike(TournamentFighter::Ryu, Loadout::Art2);
    let Some(s) = watch(style, true, 30_000, None, pressing_pad) else { return };
    eprintln!("successful SF3 parries {}", s.parries);
    assert!(s.parries >= 1, "a successful parry is drawn and cancels damage, rather than merely a forward tap");
}

#[test]
fn every_sf3_fighters_real_pad_waits_for_its_reaction_delay() {
    let delay = Level::VeryHard.reaction();
    for fighter in TournamentFighter::ALL {
        let style = Style::third_strike(fighter, Loadout::Art2);
        if data().is_none() { return; }
        let (parted, changed) = deep(move || {
            let (mut parted, mut changed) = (0u32, 0u32);
            let (mut frames, mut seed, mut next) = (0u32, 3u64, 300u32);
            while frames < 12_000 {
                let mut m = fight(style, true, 4, seed).unwrap();
                seed += 1;
                for i in 0..36_000u32 {
                    let v = seen(&m);
                    if frames >= next && v.fighting && v.me.free() && v.foe.free() && (v.me.x - v.foe.x).abs() > 150 {
                        next = frames + 400;
                        parted += 1;
                        let (mut jumped, mut stood) = (m.clone(), m.clone());
                        let mut differed = false;
                        for k in 0..delay + 30 {
                            let towards = toward(&seen(&jumped));
                            step(&mut jumped, if k < 3 { (U | towards, 0) } else { (0, 0) });
                            step(&mut stood, (0, 0));
                            let same = jumped.fair_cpu_pad() == stood.fair_cpu_pad();
                            if k < delay {
                                assert!(same, "{fighter:?}: pad changed after {k} frames, inside its {delay}-frame reaction delay");
                            } else if !same {
                                differed = true;
                            }
                        }
                        changed += u32::from(differed);
                    }
                    frames += u32::from(v.fighting);
                    step(&mut m, sparring_pad(i, &v));
                    if m.match_over() || frames >= 12_000 { break; }
                }
            }
            (parted, changed)
        });
        eprintln!("{fighter:?}: {parted} cloned fights, {changed} responding after the delay");
        assert!(parted >= 3 && changed >= 1, "{fighter:?}: delay comparison exercised a real response");
    }
}
