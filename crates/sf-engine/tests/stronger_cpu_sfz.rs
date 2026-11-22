//! Ticket #208 (`SC16`, spec #192): the stronger Fair CPU for every fighter
//! in the SFZ era style.
//!
//! Each fight is the measuring run's match (`sf_engine::measure`), with
//! the Fair CPU under test on Player 2's pad and Player 1's pad scripted
//! here. What is checked is what the frames show of Player 2's fighter, its
//! era state and its pad, and what the Fair CPU says it is doing. Local ROMs
//! and the art pack only (the cast's player tables come from it); each test
//! skips without them.

use std::path::Path;
use std::sync::{Arc, OnceLock};

use sf_engine::data_bank::DataBank;
use sf_engine::era::{Loadout, Revision};
use sf_engine::fair_cpu::{perceive, OwnMoveStep, Percept};
use sf_engine::measure::{pad_match_machine, MeasureSpec};
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
/// The move an Alpha Counter (and Custom Combo's start) plays.
const ALPHA_COUNTER: u8 = 0x2e;
/// Super meter points in one stock, as the gauge counts them.
const STOCK_AT: u8 = 48;

/// The program ROM and the data bank with the cast's player tables, built
/// once for the whole file.
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
        let bank = Arc::new(DataBank::build(&assets.maincpu, &art.catalogues));
        Some((assets.maincpu.clone(), bank))
    })
    .as_ref()
}

/// Run `f` on a thread with room for a whole match's depth of calls.
fn deep<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new().stack_size(256 << 20).spawn(f).unwrap().join().unwrap()
}

/// A match with Player 2's Fair CPU playing `fighter` in the SFZ3 style
/// `loadout` at `level`, the stronger game as `stronger` says, against
/// Player 1's Ryu in the SFZ3 A-ism, whose pad the test sets.
fn fight(fighter: TournamentFighter, loadout: Loadout, stronger: bool, level: u8, seed: u64) -> Option<PowerOnMachine> {
    let (rom, bank) = data()?;
    let spec = MeasureSpec {
        fighters: [TournamentFighter::Ryu, fighter],
        styles: [(Revision::Sfz3, Loadout::AIsm), (Revision::Sfz3, loadout)],
        difficulty: [level; 2],
        seed,
        stronger: [false, stronger],
    };
    Some(pad_match_machine(rom, Some(bank.clone()), &spec).expect("the match starts"))
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

/// Player 1's pad: a few hundred frames each of walking in, standing,
/// guarding low, sweeping, whiffing heavy punches, jumping in, attacking
/// from up close and backing off; and a heavy punch at the CPU's fighter
/// whenever it is in the air in front of him.
fn sparring_pad(i: u32, v: &Percept) -> (u8, u8) {
    if v.me.jumping() && (v.me.x - v.foe.x).abs() < 90 && i % 8 == 0 {
        return (0, HP);
    }
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

/// What a run of fights showed of Player 2's Fair CPU.
#[derive(Default, Debug)]
struct Seen {
    frames: u32,
    plan_frames: u32,
    /// Routes entered, and those outside its route book (none should be).
    routes: u32,
    foreign_routes: u32,
    /// Moves its fighter began, by move id.
    began: Vec<u32>,
    /// Of those, the ones begun on a game plan step for its own kind of
    /// move or an era tool, by the step's kind: on that step that frame or
    /// the one before (the move shows a frame or two after its input).
    began_on_step: Vec<(u8, OwnMoveStep)>,
    /// Alpha Counters begun out of its guard.
    alpha_counters: u32,
    /// Frames in the air holding back while Player 1's attack box was live,
    /// with the CPU's fighter not hit.
    air_guards: u32,
    custom_combos: u32,
    /// Of those, the ones started from the game plan step that starts it.
    custom_combos_on_step: u32,
    /// Hits on Player 1 while Custom Combo ran.
    custom_hits: u32,
    /// Supers begun outside a route: (level spent, the most the meter held).
    supers: Vec<(u8, u8)>,
}

/// Fights from seed 1 on, each to the match's end, until `frames` frames of
/// fighting have been stepped. With `stocks`, each round starts with one,
/// two or three stocks of Super meter in turn on the CPU's side (the SFZ
/// meter builds slowly against this partner: in 20,000 frames it never
/// held much more than one).
fn watch(fighter: TournamentFighter, loadout: Loadout, stronger: bool, level: u8, frames: u32, stocks: bool) -> Option<Seen> {
    data()?;
    Some(deep(move || {
        let mut s = Seen { began: vec![0; 256], ..Seen::default() };
        let routes = book(fighter, Revision::Sfz3, loadout).map_or(0, |b| b.routes.len());
        let mut seed = 1;
        while s.frames < frames {
            let mut m = fight(fighter, loadout, stronger, level, seed).unwrap();
            seed += 1;
            let mut before = seen(&m);
            let mut meter = m.era_fighters()[1].super_meter;
            let mut entering = false;
            let mut custom = false;
            let mut rounds = 0u8;
            let mut step_before = None;
            for i in 0..36_000u32 {
                let v = seen(&m);
                let (dirs, buttons) = sparring_pad(i, &v);
                step(&mut m, dirs, buttons);
                let now = seen(&m);
                if m.match_over() {
                    break;
                }
                if !now.fighting {
                    before = now;
                    meter = m.era_fighters()[1].super_meter;
                    continue;
                }
                if stocks && !before.fighting {
                    m.set_era_super_meter(1, (rounds % 3 + 1) * STOCK_AT);
                    meter = m.era_fighters()[1].super_meter;
                    rounds += 1;
                }
                s.frames += 1;
                let in_plan = m.fair_cpu_following_game_plan();
                s.plan_frames += u32::from(in_plan);
                let now_entering = m.fair_cpu_entering_route();
                if now_entering && !entering {
                    s.routes += 1;
                    if m.fair_cpu_route().is_none_or(|index| index >= routes) {
                        s.foreign_routes += 1;
                    }
                }
                let me = now.me;
                let own_step = m.fair_cpu_own_move_step();
                if me.move_id != before.me.move_id {
                    s.began[usize::from(me.move_id)] += 1;
                    if let Some(kind) = own_step.or(step_before) {
                        s.began_on_step.push((me.move_id, kind));
                    }
                }
                let era = m.era_fighters()[1];
                let now_custom = era.custom_combo_active();
                if me.move_id == ALPHA_COUNTER && before.me.guarding() && !now_custom {
                    s.alpha_counters += 1;
                }
                if now_custom && !custom {
                    s.custom_combos += 1;
                    if own_step.or(step_before) == Some(OwnMoveStep::CustomCombo) {
                        s.custom_combos_on_step += 1;
                    }
                }
                if now_custom && now.foe.health < before.foe.health {
                    s.custom_hits += 1;
                }
                let (pad_dirs, _) = cpu_pad(&m);
                if me.jumping() && !me.reacting() && now.foe.active && pad_dirs & away(&Percept { me: now.foe, foe: me, ..now }) != 0 {
                    s.air_guards += 1;
                }
                // A super begun outside a route: the stocks it took (an
                // Alpha Counter takes one too).
                let spent = meter.saturating_sub(era.super_meter);
                if spent >= STOCK_AT && !now_entering && !entering && !now_custom && me.move_id != ALPHA_COUNTER {
                    s.supers.push(((spent / STOCK_AT).min(3), (meter / STOCK_AT).min(3)));
                }
                entering = now_entering;
                custom = now_custom;
                step_before = own_step;
                meter = era.super_meter;
                before = now;
            }
        }
        s
    }))
}

#[test]
fn every_sfz_fighter_follows_game_plans_and_enters_routes_from_its_book() {
    for fighter in TournamentFighter::ALL {
        let Some(s) = watch(fighter, Loadout::AIsm, true, 4, 12_000, false) else { return };
        eprintln!("{fighter:?}: {} of {} frames in game plans, {} routes", s.plan_frames, s.frames, s.routes);
        assert!(s.plan_frames * 10 >= s.frames, "{fighter:?} follows game plans: {} of {} frames", s.plan_frames, s.frames);
        assert!(s.routes >= 1, "{fighter:?} enters routes from its book");
        assert_eq!(s.foreign_routes, 0, "{fighter:?}: every route is its book's");
    }
}

#[test]
fn with_the_switch_off_an_sfz_fighter_follows_no_game_plan() {
    for fighter in [TournamentFighter::Birdie, TournamentFighter::Gen, TournamentFighter::Ryu] {
        let Some(s) = watch(fighter, Loadout::VIsm, false, 4, 6_000, false) else { return };
        assert_eq!(s.plan_frames, 0, "{fighter:?}");
    }
}

#[test]
fn fighter_specific_moves_are_used_by_the_fighters_who_have_them() {
    use OwnMoveStep::{Charge, CommandThrow, Stance, Teleport};
    use TournamentFighter::{Birdie, Gen, Geki, Lee};
    // (fighter, move id, the plan step, what it is): command throws, a
    // teleport, charge moves and a change of stance, each begun from the
    // game plan step for its kind (Birdie's Murderer Chain is also the old
    // play's mix-up; only the step's own count here).
    let moves = [
        (Birdie, 0x2cu8, CommandThrow, "Murderer Chain, a command throw"),
        (Birdie, 0x1a, Charge, "Bull Head, a charge move"),
        // Not Retsu's Izuna Shoulder Throw: his grab plan is scored where he
        // hardly ever takes it, since taking it lost him matches to the old
        // CPU at Very Difficult (listed in SC16's report for a decision).
        (Geki, 0x20, CommandThrow, "Execution Drop, a command throw"),
        (Geki, 0x1b, Teleport, "Utsusemi, a teleport"),
        (Lee, 0x20, Charge, "Firewater, a charge move"),
        (Gen, 0x1f, Stance, "a change of stance"),
    ];
    let mut by_fighter: Vec<(TournamentFighter, Seen)> = Vec::new();
    for (fighter, id, kind, what) in moves {
        if !by_fighter.iter().any(|(f, _)| *f == fighter) {
            let Some(s) = watch(fighter, Loadout::AIsm, true, 4, 20_000, false) else { return };
            by_fighter.push((fighter, s));
        }
        let s = &by_fighter.iter().find(|(f, _)| *f == fighter).unwrap().1;
        let on_step = s.began_on_step.iter().filter(|&&(began, on)| began == id && on == kind).count();
        eprintln!("{fighter:?} {what}: begun {} times, {on_step} from its plan step", s.began[usize::from(id)]);
        let from_step: Vec<String> = s.began_on_step.iter().filter(|&&(_, on)| on == kind).map(|(id, _)| format!("{id:x}")).collect();
        assert!(on_step >= 1, "{fighter:?} uses {what} from its game plan step; that step began {from_step:?}");
    }
}

#[test]
fn the_sfz_tools_are_used_alpha_counters_air_guards_and_supers_at_the_level_the_meter_allows() {
    let Some(s) = watch(TournamentFighter::Ryu, Loadout::AIsm, true, 4, 30_000, true) else { return };
    eprintln!("Alpha Counters {}, air guards {} frames, supers {:?}", s.alpha_counters, s.air_guards, s.supers);
    assert!(s.alpha_counters >= 1, "Alpha Counters out of a guard");
    assert!(s.air_guards >= 1, "guards in the air");
    assert!(s.supers.iter().any(|&(spent, _)| spent >= 2), "a super above level 1: {:?}", s.supers);
    for (spent, held) in &s.supers {
        assert_eq!(spent, held, "a super at the level its meter allows: {:?}", s.supers);
    }
}

#[test]
fn in_the_v_ism_custom_combo_is_started_up_close_and_its_chain_hits() {
    let Some(s) = watch(TournamentFighter::Ryu, Loadout::VIsm, true, 4, 30_000, true) else { return };
    eprintln!("Custom Combos {} ({} from its plan step), hits in them {}", s.custom_combos, s.custom_combos_on_step, s.custom_hits);
    assert!(s.custom_combos_on_step >= 1, "Custom Combo started from its game plan step");
    assert!(s.custom_hits >= 1, "its chains land hits");
}
