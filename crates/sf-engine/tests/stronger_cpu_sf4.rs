//! SC18 (#210): the stronger Fair CPU playing the SF4 era style in real
//! fights. Local ROMs and the art pack supply the tournament cast's player
//! tables, as in the measuring run; tests skip when those assets are absent.

use std::path::Path;
use std::sync::{Arc, OnceLock};

use sf_engine::data_bank::DataBank;
use sf_engine::era::{loadouts, revisions, EraFamily, Loadout, Revision};
use sf_engine::fair_cpu::{perceive, EraAct, Level, Percept, Tool};
use sf_engine::measure::{sparring_machine, MeasureSpec};
use sf_engine::roster::TournamentFighter;
use sf_engine::route_book::book;
use sf_engine::run::PowerOnMachine;
use sf_video::art::{load_art_from, ArtSources};
use sf_video::art_pack::ArtPack;
use sf_video::callouts::Side;
use sf_video::era_hud::GaugeRow;
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
const FOCUS_CHARGE: u8 = 0x2e;
const FOCUS: u8 = 0x2f;
const DASH: u8 = 0x1e;
const BACKDASH: u8 = 0x1d;
const STOCK_AT: u8 = 48;
const THROWN: u8 = 0x3e;
const ULTRAS: [Loadout; 3] = [Loadout::Ultra1, Loadout::Ultra2, Loadout::UltraDouble];

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
    loadout: Loadout,
}

impl Style {
    fn ultra(fighter: TournamentFighter, loadout: Loadout) -> Self {
        Self { fighter, revision: Revision::Usf4, loadout }
    }
}

fn fight(style: Style, stronger: bool, level: u8, seed: u64) -> Option<PowerOnMachine> {
    let (rom, bank) = data()?;
    let spec = MeasureSpec {
        fighters: [TournamentFighter::Ryu, style.fighter],
        styles: [(Revision::Usf4, Loadout::Ultra1), (style.revision, style.loadout)],
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

/// Offer a standing, settled opponent at the route plan's proven spacing.
/// Some fighters have only one meterless route and it needs this stance;
/// the moving/crouching sparring partner need not offer that opening.
fn route_partner(_: u32, v: &Percept) -> (u8, u8) {
    let gap = if v.me.x < v.foe.x { v.foe.hurt.0 - v.me.x } else { v.me.x - v.foe.hurt.1 };
    if gap > 60 { (toward(v), 0) }
    else if gap < 52 { (away(v), 0) }
    else { (0, 0) }
}

/// Block the normal that offers the ItWasBlocked Focus plan, then let go
/// while the CPU charges and releases it so its level-two hit can land.
fn focus_partner(i: u32, v: &Percept) -> (u8, u8) {
    if matches!(v.me.move_id, FOCUS_CHARGE | FOCUS) { (0, 0) } else { guarding_pad(i, v) }
}

/// Offer the guarded-normal Focus plan, then attack into its armor while
/// it is charging rather than on an unrelated fixed attack schedule.
fn focus_armor_partner(i: u32, v: &Percept) -> (u8, u8) {
    match v.me.move_id {
        FOCUS_CHARGE => (0, HP),
        FOCUS => (0, 0),
        _ => guarding_pad(i, v),
    }
}

/// Leave an opening for Super and Ultra routes. Repeated pokes
/// can interrupt the walk in before a route's closer opener is tried.
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
    cpu.game_plan_tool(fighter, EraFamily::Sf4).map(OwnStep::Tool)
        .or_else(|| cpu.game_plan_era(fighter, EraFamily::Sf4).map(OwnStep::Era))
}

fn revenge(m: &PowerOnMachine, side: usize) -> u16 {
    match m.era_fighters()[side].gauge().and_then(|gauge| gauge.second) {
        Some(GaugeRow::Bar { fill, .. }) => fill.num,
        _ => 0,
    }
}

fn cpu_buttons(m: &PowerOnMachine) -> u8 {
    m.fair_cpu_pad().unwrap().1.iter().enumerate()
        .fold(0, |bits, (index, &held)| bits | (u8::from(held) << index))
}

#[derive(Clone, Copy, Default, Debug)]
struct Fadc {
    from: u8,
    spent: u8,
    answer: bool,
    dash: bool,
    dash_after: Option<u32>,
    moved: bool,
    continued: bool,
    safe: bool,
}

#[derive(Default, Debug)]
struct Seen {
    frames: u32,
    plan_frames: u32,
    routes: u32,
    foreign_routes: u32,
    began_on_step: Vec<(u8, OwnStep)>,
    caught_on_step: Vec<OwnStep>,
    to_crane: u32,
    to_mantis: u32,
    focus_on_step: u32,
    /// Health lost to a real attack while the same charge survives it.
    armored_hits: u32,
    charged_focus_hits: u32,
    /// The victim stays in the crumpling hit reaction for at least twelve
    /// frames after a level-two-or-higher Focus hit, with no intervening hit.
    crumple_holds: u32,
    fadcs: Vec<Fadc>,
    supers_on_step: u32,
    supers_in_routes: u32,
    reactive_supers: u32,
    ultras_on_step: Vec<u8>,
    ultras_in_routes: Vec<u8>,
    reactive_ultras: Vec<u8>,
    wrong_ultras: u32,
}

/// `super_points` supplies the native Super gauge before each frame.
/// `fill_revenge` enables Training's existing meter fill: the ready
/// fighter receives Revenge, and an accepted Ultra still visibly spends
/// it while its own move is busy. Both before/after gauges are recorded.
fn watch(
    style: Style,
    stronger: bool,
    frames: u32,
    super_points: Option<u8>,
    fill_revenge: bool,
    pad: fn(u32, &Percept) -> (u8, u8),
) -> Option<Seen> {
    data()?;
    Some(deep(move || {
        let mut s = Seen::default();
        let b = book(style.fighter, style.revision, style.loadout);
        let mut seed = 1u64;
        while s.frames < frames {
            let mut m = fight(style, stronger, 4, seed).unwrap();
            seed += 1;
            if fill_revenge {
                let mut config = *m.match_config();
                config.roster.mode = sf_engine::roster::PlayMode::Training;
                config.fill_meters = true;
                m.set_match_config(config);
            }
            let mut entering = false;
            let mut previous_step = None;
            let mut pending_grab: Option<(OwnStep, u32)> = None;
            let mut held_focus = 0u32;
            let mut focus_from_plan = false;
            let mut released_focus: Option<(u32, bool)> = None;
            let mut crumple_from: Option<u32> = None;
            let mut pending_fadc: Option<(Fadc, u32, i16, u16)> = None;
            let mut last_hit: Option<u32> = None;
            for i in 0..36_000u32 {
                let before = seen(&m);
                if let Some(points) = super_points.filter(|_| before.fighting) {
                    m.set_era_super_meter(1, points);
                }
                let before_era = m.era_fighters()[1];
                let before_foe_hp = m.era_fighters()[0].sub_hp;
                let before_revenge = revenge(&m, 1);
                let step_before = own_step(&m, style.fighter);
                let cpu_before = m.fair_cpu().unwrap();
                let route_before = cpu_before.route_index().zip(cpu_before.route_step());
                let answering_before = cpu_before.answering();
                let opponent_pad = pad(i, &before);
                step(&mut m, opponent_pad);
                let now = seen(&m);
                if m.match_over() { break; }
                if !now.fighting {
                    entering = false;
                    previous_step = None;
                    pending_grab = None;
                    held_focus = 0;
                    focus_from_plan = false;
                    released_focus = None;
                    crumple_from = None;
                    pending_fadc = None;
                    last_hit = None;
                    continue;
                }
                s.frames += 1;
                s.plan_frames += u32::from(m.fair_cpu_following_game_plan());
                let cpu = m.fair_cpu().unwrap();
                let route = cpu.route_index().filter(|_| cpu.entering_route());
                let route_after = cpu.route_index().zip(cpu.route_step());
                if route.is_some() && !entering {
                    s.routes += 1;
                    s.foreign_routes += u32::from(route.is_none_or(|index| b.is_none_or(|book| index >= book.routes.len())));
                }
                entering = route.is_some();
                let active_command = (route_before == route_after).then_some(route_before).flatten()
                    .and_then(|(index, step)| b.and_then(|book| book.routes.get(index)).and_then(|route| route.steps.get(step)));
                let super_command = active_command.is_some_and(|step| {
                    (step.name.starts_with("236236+") || step.name.starts_with("214214+"))
                        && !step.name.ends_with("PPP") && !step.name.ends_with("KKK")
                });
                let ultra_command = active_command.is_some_and(|step| {
                    (step.name.starts_with("236236+") || step.name.starts_with("214214+"))
                        && (step.name.ends_with("PPP") || step.name.ends_with("KKK"))
                });
                let on = own_step(&m, style.fighter).or(step_before).or(previous_step);
                let confirmed_answer = (answering_before || cpu.answering())
                    && last_hit.is_some_and(|from| i >= from + Level::VeryHard.reaction() && i <= from + 120);
                let era = m.era_fighters()[1];
                let spent = before_era.super_meter.saturating_sub(era.super_meter);
                let foe_hurt = m.era_fighters()[0].sub_hp < before_foe_hp && now.foe.reacting() && !now.foe.guarding();
                let began = now.me.move_id != before.me.move_id;
                if began {
                    if let Some(on) = on {
                        s.began_on_step.push((now.me.move_id, on));
                        if on == OwnStep::Tool(Tool::CommandThrow) && now.me.attacking() {
                            pending_grab = Some((on, i));
                        }
                    }
                    if now.me.move_id == FOCUS_CHARGE && cpu_buttons(&m) & 0x12 == 0x12 {
                        focus_from_plan = on == Some(OwnStep::Era(EraAct::Focus));
                        s.focus_on_step += u32::from(focus_from_plan);
                        // Ryu's real native specials are 0x19..=0x1b.
                        // Requiring their state and the paid Focus start
                        // distinguishes FADC from a free neutral Focus.
                        if style.fighter == TournamentFighter::Ryu && matches!(before.me.move_id, 0x19..=0x1b) && spent == 2 * STOCK_AT {
                            if let Some((fadc, _, _, _)) = pending_fadc.take() { s.fadcs.push(fadc); }
                            pending_fadc = Some((Fadc { from: before.me.move_id, spent, answer: answering_before || cpu.answering(), ..Fadc::default() }, i, now.me.x, era.sub_hp));
                        }
                    }
                    // These observations are used only by the native Ryu
                    // resource tests; Gen's stance changes these slots.
                    if style.fighter == TournamentFighter::Ryu {
                        if now.me.move_id == 0x29 && spent == 4 * STOCK_AT {
                            s.supers_on_step += u32::from(on == Some(OwnStep::Era(EraAct::Super)));
                            s.supers_in_routes += u32::from(super_command);
                            s.reactive_supers += u32::from(confirmed_answer);
                        }
                        if matches!(now.me.move_id, 0x2a | 0x2b) && before_revenge >= 320 && revenge(&m, 1) < before_revenge && era.super_meter == before_era.super_meter {
                            let selected = match style.loadout {
                                Loadout::Ultra1 => now.me.move_id == 0x2a,
                                Loadout::Ultra2 => now.me.move_id == 0x2b,
                                Loadout::UltraDouble => true,
                                _ => false,
                            };
                            s.wrong_ultras += u32::from(!selected);
                            if on == Some(OwnStep::Era(EraAct::Ultra)) { s.ultras_on_step.push(now.me.move_id); }
                            if ultra_command { s.ultras_in_routes.push(now.me.move_id); }
                            if confirmed_answer { s.reactive_ultras.push(now.me.move_id); }
                        }
                    }
                }
                if before.me.move_id == FOCUS_CHARGE && now.me.move_id == FOCUS_CHARGE && era.sub_hp < before_era.sub_hp && !now.me.reacting() && now.foe.attacking() {
                    s.armored_hits += 1;
                }
                if now.me.move_id == FOCUS_CHARGE && cpu_buttons(&m) & 0x12 == 0x12 {
                    held_focus += 1;
                } else if before.me.move_id == FOCUS_CHARGE && now.me.move_id == FOCUS {
                    released_focus = Some((i, held_focus >= 20 && focus_from_plan));
                    held_focus = 0;
                } else if now.me.move_id != FOCUS {
                    held_focus = 0;
                    focus_from_plan = false;
                }
                if released_focus.is_some_and(|(from, charged)| charged && i <= from + 35) && now.me.move_id == FOCUS && foe_hurt {
                    s.charged_focus_hits += 1;
                    crumple_from = Some(i);
                    released_focus = None;
                }
                if let Some(from) = crumple_from {
                    if !now.foe.reacting() || now.foe.guarding() || i > from && foe_hurt {
                        crumple_from = None;
                    } else if i >= from + 12 {
                        s.crumple_holds += 1;
                        crumple_from = None;
                    }
                }
                if let Some((on, from)) = pending_grab {
                    if now.foe.move_id == THROWN {
                        s.caught_on_step.push(on);
                        pending_grab = None;
                    } else if i > from + 90 { pending_grab = None; }
                }
                if let Some((fadc, from, x, hp)) = pending_fadc.as_mut() {
                    // The supplied FADC input has seven frames. Credit
                    // only the direct charge→dash transition inside it,
                    // while it is still a reactive answer. A fresh dash
                    // much later in this fight cannot stand in for it.
                    if !fadc.dash && i <= *from + 7 && before.me.move_id == FOCUS_CHARGE && matches!(now.me.move_id, DASH | BACKDASH) && fadc.answer && (answering_before || cpu.answering()) {
                        fadc.dash = true;
                        fadc.dash_after = Some(i - *from);
                    }
                    fadc.moved |= fadc.dash && (now.me.x - *x).abs() > 2;
                    fadc.continued |= fadc.moved && now.me.attacking() && !matches!(now.me.move_id, FOCUS | FOCUS_CHARGE | DASH | BACKDASH);
                    fadc.safe |= fadc.moved && now.me.free() && era.sub_hp >= *hp;
                    if !fadc.dash && i > *from + 7 || i > *from + 90 {
                        s.fadcs.push(*fadc);
                        pending_fadc = None;
                    }
                }
                if before.fighting && era.in_crane() != before_era.in_crane() {
                    if era.in_crane() { s.to_crane += 1 } else { s.to_mantis += 1 }
                }
                if foe_hurt { last_hit = Some(i); }
                previous_step = own_step(&m, style.fighter);
                if s.frames >= frames { break; }
            }
            if let Some((fadc, _, _, _)) = pending_fadc { s.fadcs.push(fadc); }
        }
        s
    }))
}

#[test]
fn every_usf4_fighter_and_loadout_uses_game_plans_and_its_own_routes() {
    for fighter in TournamentFighter::ALL {
        for loadout in ULTRAS {
            let style = Style::ultra(fighter, loadout);
            let Some(s) = watch(style, true, 12_000, None, false, route_partner) else { return };
            eprintln!("{style:?}: {} of {} plan frames, {} routes", s.plan_frames, s.frames, s.routes);
            assert!(s.plan_frames * 10 >= s.frames && s.routes >= 1, "{style:?}: plans and routes are used: {s:?}");
            assert_eq!(s.foreign_routes, 0, "{style:?}: its routes belong to this fighter and style");
        }
    }
}

#[test]
fn native_sf4_fighters_use_every_supported_revision_and_loadout() {
    for fighter in [TournamentFighter::Ryu, TournamentFighter::Ken, TournamentFighter::Gen, TournamentFighter::Adon, TournamentFighter::Sagat] {
        for revision in revisions(fighter, EraFamily::Sf4).into_iter().filter(|&r| r != Revision::Usf4) {
            for loadout in loadouts(revision) {
                let style = Style { fighter, revision, loadout };
                let Some(s) = watch(style, true, 12_000, None, false, sparring_pad) else { return };
                assert!(s.plan_frames * 10 >= s.frames && s.routes >= 1, "{style:?}: {s:?}");
                assert_eq!(s.foreign_routes, 0, "{style:?}");
            }
        }
    }
}

#[test]
fn with_the_switch_off_no_sf4_fighter_takes_a_stronger_game_plan() {
    for fighter in TournamentFighter::ALL {
        let style = Style::ultra(fighter, Loadout::Ultra2);
        let Some(s) = watch(style, false, 4_000, None, false, sparring_pad) else { return };
        assert_eq!(s.plan_frames, 0, "{style:?}");
        assert!(s.began_on_step.is_empty(), "{style:?}: no stronger-only tool step");
    }
}

#[test]
fn sf4_fighters_use_real_command_throws_teleports_charges_and_stances() {
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
        let Some(s) = watch(Style::ultra(fighter, Loadout::Ultra1), true, 30_000, None, false, sparring_pad) else { return };
        for &(id, on) in tools {
            let starts = s.began_on_step.iter().filter(|&&(move_id, step)| move_id == id && step == on).count();
            eprintln!("{fighter:?} {on:?}: {starts} move starts");
            assert!(starts >= 1, "{fighter:?}: move {id:x} starts from its own {on:?} step");
            if on == grab {
                assert!(s.caught_on_step.contains(&grab), "{fighter:?}: the command throw actually catches him");
            }
        }
        if fighter == Gen { assert!(s.to_crane >= 1 && s.to_mantis == 0, "Gen keeps the Crane stance after changing to it"); }
    }
}

#[test]
fn sf4_focus_plans_land_a_charged_crumple_and_armor_absorbs_a_real_attack() {
    let style = Style::ultra(TournamentFighter::Ryu, Loadout::Ultra1);
    let Some(charged) = watch(style, true, 30_000, None, false, focus_partner) else { return };
    eprintln!("Focus plan starts {}, charged hits {}, crumple holds {}", charged.focus_on_step, charged.charged_focus_hits, charged.crumple_holds);
    assert!(charged.focus_on_step >= 1 && charged.charged_focus_hits >= 1 && charged.crumple_holds >= 1, "the plan enters, charges and lands Focus, holding the victim in its crumpling reaction: {charged:?}");
    let Some(armor) = watch(style, true, 30_000, None, false, focus_armor_partner) else { return };
    eprintln!("attacks absorbed while Focus remains charging {}", armor.armored_hits);
    assert!(armor.armored_hits >= 1, "a real attack takes recoverable health while the CPU stays in its Focus charge");
}

#[test]
fn sf4_reactive_fadc_pays_two_stocks_and_dashes_to_continue_or_recover_safely() {
    let style = Style::ultra(TournamentFighter::Ryu, Loadout::Ultra1);
    let Some(s) = watch(style, true, 40_000, Some(2 * STOCK_AT), false, sparring_pad) else { return };
    eprintln!("paid FADCs {:?}", s.fadcs);
    assert!(s.fadcs.iter().any(|fadc| matches!(fadc.from, 0x19..=0x1b) && fadc.spent == 2 * STOCK_AT && fadc.answer && fadc.dash_after.is_some_and(|frames| frames <= 7) && fadc.dash && fadc.moved && (fadc.continued || fadc.safe)), "a paid reactive special→Focus cancel directly dashes inside its input sequence and resumes an attack or reaches safe recovery: {:?}", s.fadcs);
}

#[test]
fn sf4_super_is_deliberately_started_from_its_plan_or_actual_route_command() {
    let style = Style::ultra(TournamentFighter::Ryu, Loadout::Ultra1);
    // A charged Focus crumple offers enough seen hitstun for a deliberate
    // Super. Ryu's checked-in book has Ultra routes but no Super recipe.
    let Some(s) = watch(style, true, 30_000, Some(4 * STOCK_AT), false, focus_partner) else { return };
    eprintln!("192-point Supers: {} from Super steps, {} inside their route command, {} on a delayed hit confirm", s.supers_on_step, s.supers_in_routes, s.reactive_supers);
    assert!(s.supers_on_step + s.supers_in_routes + s.reactive_supers >= 1, "a real native Super spends all four stocks from an attributed plan, route, or delayed hit confirm");
}

#[test]
fn sf4_ultras_spend_revenge_and_obey_the_selected_loadout() {
    for loadout in ULTRAS {
        let style = Style::ultra(TournamentFighter::Ryu, loadout);
        let Some(s) = watch(style, true, 30_000, Some(0), true, standing_pad) else { return };
        eprintln!("{loadout:?}: Ultras from steps {:?}, routes {:?}, delayed hit confirms {:?}", s.ultras_on_step, s.ultras_in_routes, s.reactive_ultras);
        assert!(!s.ultras_on_step.is_empty() || !s.ultras_in_routes.is_empty() || !s.reactive_ultras.is_empty(), "the selected Ultra actually spends Revenge without spending native Super: {s:?}");
        assert_eq!(s.wrong_ultras, 0, "the fighter never starts an unselected Ultra");
    }
}

#[test]
fn every_sf4_fighters_real_pad_waits_for_its_reaction_delay() {
    let delay = Level::VeryHard.reaction();
    for fighter in TournamentFighter::ALL {
        let style = Style::ultra(fighter, Loadout::Ultra2);
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
                            if k < delay { assert!(same, "{fighter:?}: changed after {k} frames, before its {delay}-frame reaction delay"); }
                            else if !same { differed = true; }
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
        eprintln!("{fighter:?}: {parted} cloned fights, {changed} responses after the delay");
        assert!(parted >= 3 && changed >= 1, "{fighter:?}: the delay comparison exercises a real response");
    }
}
