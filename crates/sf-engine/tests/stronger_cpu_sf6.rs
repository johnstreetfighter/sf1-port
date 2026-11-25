//! SC20 (#212): the rest of SF6's cast plays its own stronger game through real pads.

use std::collections::VecDeque;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use sf_engine::config::WorldTourRules;
use sf_engine::data_bank::DataBank;
use sf_engine::era::{EraFamily, GaugeMode, Loadout, Revision};
use sf_engine::fair_cpu::{perceive, EraAct, Level, Percept, Situation, Tool};
use sf_engine::measure::{sparring_machine, MeasureSpec};
use sf_engine::roster::TournamentFighter;
use sf_engine::route_book::book;
use sf_engine::run::PowerOnMachine;
use sf_engine::world_tour::native::{MissionKind, RuleModifier};
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
const STOCK: u8 = 48;
const RUSH: u8 = 0x2c;
const PARRY: u8 = 0x2d;
const REVERSAL: u8 = 0x2e;
const IMPACT: u8 = 0x2f;
const THROWN: u8 = 0x3e;

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

fn newcomers() -> impl Iterator<Item = TournamentFighter> {
    TournamentFighter::ALL.into_iter().filter(|fighter| *fighter != TournamentFighter::Ryu)
}

fn fight(fighter: TournamentFighter, stronger: bool, level: u8, seed: u64) -> Option<PowerOnMachine> {
    let (rom, bank) = data()?;
    let spec = MeasureSpec {
        fighters: [TournamentFighter::Ryu, fighter],
        styles: [(Revision::Sf6, Loadout::Standard); 2],
        difficulty: [level; 2], seed, stronger: [false, stronger],
    };
    Some(sparring_machine(rom, Some(bank.clone()), &spec).expect("the native fight starts"))
}

fn step(m: &mut PowerOnMachine, (dirs, buttons): (u8, u8)) {
    m.set_p1_direction(dirs & R != 0, dirs & L != 0, dirs & D != 0, dirs & U != 0);
    m.set_p1_buttons(std::array::from_fn(|i| buttons & (1 << i) != 0));
    m.step();
}

fn seen(m: &PowerOnMachine) -> Percept {
    perceive(m.state.frame, true, m.work_ram(), &m.state.objectram, Side::P2)
}

fn toward(v: &Percept) -> u8 { if v.me.x >= v.foe.x { R } else { L } }
fn away(v: &Percept) -> u8 { R + L - toward(v) }
fn gap(v: &Percept) -> i16 { if v.me.x < v.foe.x { v.foe.hurt.0 - v.me.x } else { v.me.x - v.foe.hurt.1 } }
fn idle(_: u32, _: &Percept) -> (u8, u8) { (0, 0) }

fn sparring(i: u32, v: &Percept) -> (u8, u8) {
    let t = i % 300;
    match (i / 300) % 8 {
        0 => (toward(v), 0),
        1 => (0, 0),
        2 => (D | away(v), 0),
        3 => if t % 40 < 3 { (D, HK) } else { (toward(v), 0) },
        4 => if t % 45 < 3 { (0, HP) } else { (0, 0) },
        5 => match t % 70 { 0..=2 => (U | toward(v), 0), 24..=26 => (0, HK), _ => (0, 0) },
        6 => if t % 24 < 3 { (toward(v), HP) } else { (toward(v), 0) },
        _ => (away(v), 0),
    }
}

fn guarding(_: u32, v: &Percept) -> (u8, u8) {
    if (v.me.x - v.foe.x).abs() > 38 { (toward(v), 0) } else { (D | away(v), 0) }
}

fn pressing(i: u32, v: &Percept) -> (u8, u8) {
    if gap(v) > 30 { (toward(v), 0) } else if i % 12 < 2 { (D, LP) } else { (D | away(v), 0) }
}

fn poking(i: u32, v: &Percept) -> (u8, u8) {
    if gap(v) > 60 { (toward(v), 0) } else if i % 41 < 3 { (0, [LP, 2, HP, 16, HK][(i / 41 % 5) as usize]) } else { (0, 0) }
}

fn route_partner(i: u32, v: &Percept) -> (u8, u8) {
    match i % 100 {
        0..=2 => (D, LP),
        3..=60 => (D, 0),
        _ if gap(v) > 44 => (toward(v), 0),
        _ if gap(v) < 38 => (away(v), 0),
        _ => (0, 0),
    }
}

/// The native Punish books start after guarding a real sweep. Use the
/// human's attack gap, rather than the CPU-to-human distance, to offer one.
fn sweep_partner(i: u32, v: &Percept) -> (u8, u8) {
    let reach = if v.me.x >= v.foe.x { v.me.hurt.0 - v.foe.x } else { v.foe.x - v.me.hurt.1 };
    match i % 90 {
        0..=2 if reach <= 40 => (D, HK),
        3..=40 => (D | away(v), 0),
        _ if reach > 28 => (toward(v), 0),
        _ => (D | away(v), 0),
    }
}

fn projectile(i: u32, v: &Percept) -> (u8, u8) {
    let forward = toward(v);
    match i % 100 { 0 => (D, 0), 1 => (D | forward, 0), 2 => (forward, HP), _ => (0, 0) }
}

fn place_centers(m: &mut PowerOnMachine, foe_x: i16, own_x: i16) {
    let v = seen(m);
    let t = m.diagnostic_telemetry();
    let mut snapshot = m.save_snapshot();
    for (entity, current, target) in [(t.p1.entity_offset, v.foe.x, foe_x), (t.p2.entity_offset, v.me.x, own_x)] {
        let at = usize::from(entity) + 6;
        let x = u16::from_be_bytes([snapshot.state.objectram[at], snapshot.state.objectram[at + 1]]);
        snapshot.state.objectram[at..at + 2].copy_from_slice(&x.wrapping_add(target.wrapping_sub(current) as u16).to_be_bytes());
    }
    m.restore_snapshot(&snapshot);
}

/// Arrange genuine standing entities after the native intro; no accepted move,
/// hit, command tracker, or CPU intent is injected into the fixture.
fn standing_scene(m: &mut PowerOnMachine, positions: fn(&Percept) -> (i16, i16)) {
    let fresh = m.fair_cpu().cloned().expect("the freshly seeded CPU");
    assert_eq!(fresh.last_view().frame, 0);
    let original = *m.match_config();
    let mut manual = original;
    manual.p2_fair_cpu = false;
    m.set_match_config(manual);
    m.set_p2_direction(false, false, false, false);
    m.set_p2_buttons([false; 6]);
    let mut ready = false;
    for i in 0..3_000 {
        step(m, (0, 0));
        assert!(m.unrecovered().is_empty(), "native intro frame{i}: engine stops {:x?}", m.unrecovered());
        let v = seen(m);
        if m.task_positions().contains(&(1, 0x3eafa)) && v.me.present && v.foe.present && v.me.move_id == 1 && v.foe.move_id == 1 {
            ready = true;
            break;
        }
    }
    assert!(ready, "the native standing fight is reached");
    m.set_match_config(original);
    let (foe_x, own_x) = positions(&seen(m));
    let mut snapshot = m.save_snapshot();
    snapshot.fair_cpu = Some(fresh);
    m.restore_snapshot(&snapshot);
    place_centers(m, foe_x, own_x);
    assert_eq!((seen(m).foe.x, seen(m).me.x), (foe_x, own_x));
    assert_eq!(m.fair_cpu().unwrap().last_view().frame, 0);
}

fn centered(m: &mut PowerOnMachine) { standing_scene(m, |v| (220, 220 + v.foe.hurt.1 - v.foe.x + 50)); }
fn far(m: &mut PowerOnMachine) { standing_scene(m, |_| (150, 350)); }
fn rush_room(m: &mut PowerOnMachine) { standing_scene(m, |v| (200, 200 + v.foe.hurt.1 - v.foe.x + 116)); }
fn projectile_room(m: &mut PowerOnMachine) { standing_scene(m, |_| (150, 310)); }
fn charge_room(m: &mut PowerOnMachine) { standing_scene(m, |_| (156, 328)); }

/// A real sweep puts the CPU behind before its untouched controller
/// begins. Low-stock supers are then allowed by the normal spending policy.
fn behind_for_super(m: &mut PowerOnMachine) {
    standing_scene(m, |_| (220, 255));
    let fresh = m.fair_cpu().cloned().unwrap();
    let original = *m.match_config();
    let mut manual = original;
    manual.p2_fair_cpu = false;
    m.set_match_config(manual);
    let mut hurt = false;
    for i in 0..1_600 {
        let v = seen(m);
        let pad = if i % 70 < 2 { (D, HK) } else if (v.me.x - v.foe.x).abs() > 35 { (toward(&v), 0) } else { (0, 0) };
        step(m, pad);
        assert!(m.unrecovered().is_empty(), "super precondition attack frame{i}: engine stops {:x?}", m.unrecovered());
        let own = &m.era_fighters()[1];
        if own.sub_hp > 0 && own.sub_hp * 2 <= own.max_sub_hp { hurt = true; break; }
    }
    assert!(hurt, "real attacks establish the behind-in-health super opportunity");
    let mut recovered = false;
    for i in 0..300 {
        step(m, (0, 0));
        assert!(m.unrecovered().is_empty(), "super precondition recovery frame{i}: engine stops {:x?}", m.unrecovered());
        let v = seen(m);
        if v.me.move_id == 1 && v.foe.move_id == 1 { recovered = true; break; }
    }
    assert!(recovered, "both native fighters recover before the CPU begins");
    m.set_match_config(original);
    let mut snapshot = m.save_snapshot();
    snapshot.fair_cpu = Some(fresh);
    m.restore_snapshot(&snapshot);
    let v = seen(m);
    place_centers(m, 220, 220 + v.foe.hurt.1 - v.foe.x + 50);
    assert!(seen(m).me.health < seen(m).foe.health, "the visible health bars allow spending");
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OwnStep { Tool(Tool), Era(EraAct) }

fn own_step(m: &PowerOnMachine, fighter: TournamentFighter) -> Option<OwnStep> {
    let cpu = m.fair_cpu().unwrap();
    cpu.game_plan_tool(fighter, EraFamily::Sf6).map(OwnStep::Tool)
        .or_else(|| cpu.game_plan_era(fighter, EraFamily::Sf6).map(OwnStep::Era))
}

#[derive(Clone, Copy)]
struct Trial {
    stronger: bool,
    level: u8,
    super_meter: Option<u8>,
    gauge: GaugeMode,
    no_raw_rush: bool,
    footsies: bool,
    pad: fn(u32, &Percept) -> (u8, u8),
    setup: Option<fn(&mut PowerOnMachine)>,
}

impl Default for Trial {
    fn default() -> Self {
        Self { stronger: true, level: 4, super_meter: None, gauge: GaugeMode::EraDefault,
            no_raw_rush: false, footsies: false, pad: sparring, setup: None }
    }
}

#[derive(Debug)]
struct Start { id: u8, own: Option<OwnStep>, super_spent: u8, drive_spent: u16, raw_rush: bool }

#[derive(Default)]
struct Observed {
    frames: u32,
    plan_frames: u32,
    routes: u32,
    foreign_routes: u32,
    long_easy_routes: u32,
    paid_easy_routes: u32,
    starts: Vec<Start>,
    command_catches: u32,
    teleport_steps: u32,
    impact_steps: u32,
    super_steps: u32,
    situations: [u32; 3],
    crane: u32,
    mantis: u32,
    rush_closes: u32,
    impact_contacts: u32,
    art_contacts: Vec<u8>,
    parry_contacts: u32,
    reactive_impacts: u32,
    reactive_parries: u32,
    early_answers: u32,
    reversals: Vec<u16>,
    illegal_mission_pads: u32,
    illegal_mission_routes: u32,
}

impl std::fmt::Debug for Observed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut starts = std::collections::BTreeMap::<String, usize>::new();
        for s in &self.starts {
            *starts.entry(format!("{:02x} {:?} S{} D{} raw{}", s.id, s.own, s.super_spent, s.drive_spent, s.raw_rush)).or_default() += 1;
        }
        f.debug_struct("Observed").field("frames", &self.frames).field("plan_frames", &self.plan_frames)
            .field("routes", &self.routes).field("foreign_routes", &self.foreign_routes)
            .field("long_easy_routes", &self.long_easy_routes).field("paid_easy_routes", &self.paid_easy_routes)
            .field("starts", &starts).field("command_catches", &self.command_catches).field("teleport_steps", &self.teleport_steps)
            .field("impact_steps", &self.impact_steps).field("super_steps", &self.super_steps).field("hit_block_whiff_frames", &self.situations)
            .field("crane", &self.crane).field("mantis", &self.mantis).field("rush_closes", &self.rush_closes)
            .field("impact_contacts", &self.impact_contacts).field("art_contacts", &self.art_contacts)
            .field("parry_contacts", &self.parry_contacts).field("reactive_impacts", &self.reactive_impacts)
            .field("reactive_parries", &self.reactive_parries).field("early_answers", &self.early_answers)
            .field("reversals", &self.reversals).field("illegal_mission_pads", &self.illegal_mission_pads)
            .field("illegal_mission_routes", &self.illegal_mission_routes).finish()
    }
}

fn watch(fighter: TournamentFighter, seed: u64, frames: u32, trial: Trial) -> Option<Observed> {
    watch_until(fighter, seed, frames, trial, |_| false)
}

/// Stop once the requested native fact has been proved. Every engine step
/// before that proof is checked; the frame bound still fails missing evidence.
fn watch_until(fighter: TournamentFighter, seed: u64, frames: u32, trial: Trial, done: impl Fn(&Observed) -> bool + Send + 'static) -> Option<Observed> {
    data()?;
    Some(deep(move || {
        let mut out = Observed::default();
        let routes = book(fighter, Revision::Sf6, Loadout::Standard).expect("the fighter's own SF6 book");
        for seed in seed..seed + 80 {
            let mut m = fight(fighter, trial.stronger, trial.level, seed).unwrap();
            let mut cfg = *m.match_config();
            cfg.gauge_mode = trial.gauge;
            cfg.disable_raw_drive_rush = trial.no_raw_rush;
            cfg.world_tour = trial.footsies.then_some(WorldTourRules {
                mission: MissionKind::RuleModifierTrial(RuleModifier::PureFootsies), bonuses: None, starting_hp: None,
            });
            m.set_match_config(cfg);
            if let Some(setup) = trial.setup { setup(&mut m); }
            let mut entering = false;
            let mut pending_grab = false;
            let mut rushed = None;
            let mut impact = None;
            let mut latest_attack = None;
            let mut free_for = 0u32;
            let mut recent = VecDeque::new();
            for i in 0..36_000u32 {
                let before = seen(&m);
                if let Some(meter) = trial.super_meter.filter(|_| before.fighting) { m.set_era_super_meter(1, meter); }
                let era_before = m.era_fighters()[1];
                let drive_before = era_before.sf6_drive_points().unwrap_or(0);
                let on_before = own_step(&m, fighter);
                let answer_before = m.fair_cpu().unwrap().answering();
                let blocked = m.toggle_state().combo.blockstun[1] != 0;
                let foe_block_before = m.toggle_state().combo.blockstun[0];
                let plan_began_before = m.fair_cpu().unwrap().game_plan_began();
                let human_pad = (trial.pad)(i, &before);
                step(&mut m, human_pad);
                assert!(m.unrecovered().is_empty(), "{fighter:?} level{} seed{seed} frame{i} meter{:?} gauge{:?} human{human_pad:?} own{on_before:?} move{:02x} pad{:04x} route{:?} game{:?} paidArts{:?}: native engine stops {:x?}",
                    trial.level, trial.super_meter, trial.gauge, before.me.move_id, m.fair_cpu().unwrap().pad(), m.fair_cpu().unwrap().route_index(), m.fair_cpu().unwrap().game_plan(),
                    out.starts.iter().filter(|s| (0x29..=0x2b).contains(&s.id) && s.super_spent != 0).collect::<Vec<_>>(), m.unrecovered());
                let now = seen(&m);
                if m.match_over() { break; }
                if !now.fighting { entering = false; continue; }
                out.frames += 1;
                out.plan_frames += u32::from(m.fair_cpu_following_game_plan());
                let cpu = m.fair_cpu().unwrap();
                let route = cpu.route_index().filter(|_| cpu.entering_route());
                if let Some(index) = route.filter(|_| !entering) {
                    out.routes += 1;
                    if let Some(route) = routes.routes.get(index) {
                        if trial.level == 1 {
                            out.long_easy_routes += u32::from(route.hits > Level::Easy.route_hits());
                            out.paid_easy_routes += u32::from(route.super_spent + route.drive_spent != 0);
                        }
                        out.illegal_mission_routes += u32::from(trial.footsies && !route.normals_only());
                    } else { out.foreign_routes += 1; }
                }
                entering = route.is_some();
                let on = own_step(&m, fighter).or(on_before);
                out.teleport_steps += u32::from(on == Some(OwnStep::Tool(Tool::Teleport)));
                out.impact_steps += u32::from(on == Some(OwnStep::Era(EraAct::DriveImpact)));
                out.super_steps += u32::from(on == Some(OwnStep::Era(EraAct::Super)));
                match cpu.situation().situation {
                    Situation::ItHit => out.situations[0] += 1,
                    Situation::ItWasBlocked => out.situations[1] += 1,
                    Situation::OpponentWhiffed => out.situations[2] += 1,
                    _ => (),
                }
                let answer = answer_before || cpu.answering();
                let era = m.era_fighters()[1];
                let drive = era.sf6_drive_points().unwrap_or(0);
                let spent_drive = drive_before.saturating_sub(drive);
                let spent_super = era_before.super_meter.saturating_sub(era.super_meter);
                let began = now.me.move_id != before.me.move_id;
                let foe_began = now.foe.attacking() && (!before.foe.attacking() || now.foe.move_id != before.foe.move_id);
                if foe_began { latest_attack = Some((i, free_for >= 20)); }
                free_for = if now.foe.free() && !now.foe.guarding() { free_for + 1 } else { 0 };
                if began {
                    out.starts.push(Start { id: now.me.move_id, own: on, super_spent: spent_super, drive_spent: spent_drive,
                        raw_rush: now.me.move_id == RUSH && before.me.free() && spent_drive <= u16::from(STOCK) });
                    if on == Some(OwnStep::Tool(Tool::CommandThrow)) { pending_grab = true; }
                    if now.me.move_id == RUSH && gap(&before) > 50 {
                        rushed = cpu.game_plan_began().or(plan_began_before).map(|plan| (i, gap(&before), plan));
                    }
                    if now.me.move_id == IMPACT { impact = Some(i); }
                    if answer && matches!(now.me.move_id, IMPACT | PARRY) {
                        if let Some((at, isolated)) = latest_attack.filter(|&(at, _)| i - at <= 60) {
                            out.reactive_impacts += u32::from(now.me.move_id == IMPACT);
                            out.reactive_parries += u32::from(now.me.move_id == PARRY);
                            out.early_answers += u32::from(isolated && i - at < Level::from_difficulty(trial.level).reaction());
                        }
                    }
                    if now.me.move_id == REVERSAL && blocked && answer { out.reversals.push(spent_drive.max(u16::from(spent_super))); }
                }
                if let Some(at) = impact {
                    if i - at <= 60 && before.me.move_id == IMPACT && !before.my_shot && !now.my_shot
                        && (now.foe.health < before.foe.health || m.toggle_state().combo.blockstun[0] > foe_block_before) {
                        out.impact_contacts += 1;
                        impact = None;
                    } else if i - at > 60 { impact = None; }
                }
                // The normal must actually hit, after closing space, inside
                // the very same bounded game plan that entered Drive Rush.
                if before.me.normal() && !before.my_shot && now.foe.health < before.foe.health {
                    if rushed.is_some_and(|(at, from, plan)| i - at <= 300 && gap(&now) < from
                        && (plan_began_before == Some(plan) || cpu.game_plan_began() == Some(plan))) {
                        out.rush_closes += 1;
                        rushed = None;
                    }
                }
                // A parry refunds a stock on actual contact. Ordinary regeneration
                // is at most one point, so this is not just a held pose.
                if before.me.move_id == PARRY && drive >= drive_before + u16::from(STOCK / 2) && era.sub_hp == era_before.sub_hp {
                    out.parry_contacts += 1;
                }
                if !era_before.command_throw_pending() && era.command_throw_pending() && now.foe.move_id == THROWN {
                    out.command_catches += u32::from(pending_grab);
                }
                if now.foe.health < before.foe.health && (0x29..=0x2b).contains(&before.me.move_id) {
                    out.art_contacts.push(before.me.move_id);
                }
                if began && now.me.move_id < 0x19 { pending_grab = false; }
                // A native new round resets stance while move0/HP are being
                // installed. Count only accepted stance changes in live play.
                if era.in_crane() != era_before.in_crane() && before.me.health != 0 && before.foe.health != 0
                    && (before.me.move_id == 0x1f || now.me.move_id == 0x1f) {
                    out.crane += u32::from(era.in_crane());
                    out.mantis += u32::from(!era.in_crane());
                }
                let pad = cpu.pad();
                let buttons = (pad >> 4) as u8;
                let forward = if now.me.facing_left { L } else { R };
                let direction = (pad & 15) as u8;
                if recent.back().copied() != Some(direction) { recent.push_back(direction); }
                while recent.len() > 6 { recent.pop_front(); }
                let motion = recent.iter().copied().collect::<Vec<_>>();
                let qcf = motion.ends_with(&[D, D | forward, forward]);
                let qcb = motion.ends_with(&[D, D | (R + L - forward), R + L - forward]);
                let era_chord = buttons & 0x12 == 0x12 || buttons & 0x24 == 0x24 || buttons & 0x07 == 0x07 || buttons & 0x38 == 0x38;
                out.illegal_mission_pads += u32::from(trial.footsies && (era_chord || buttons != 0 && (qcf || qcb) || on.is_some()));
                if out.frames >= frames || done(&out) { return out; }
            }
        }
        panic!("{fighter:?}: insufficient live play in bounded native trials: {out:?}");
    }))
}

#[test]
fn every_new_sf6_fighter_uses_its_own_routes_and_game_plans_at_all_four_levels() {
    for fighter in newcomers() {
        for level in 1..=4 {
            let (mut frames, mut plans, mut routes) = (0, 0, 0);
            let mut situations = [0; 3];
            for seed in 1..=12 {
                for scene in 0..3 {
                    let Some(s) = watch(fighter, seed, 1_800, Trial {
                        level, pad: [idle, route_partner, sweep_partner][scene], setup: (scene == 0).then_some(centered), ..Trial::default()
                    }) else { return };
                    assert_eq!((s.foreign_routes, s.long_easy_routes, s.paid_easy_routes), (0, 0, 0), "{fighter:?} level{level}: {s:?}");
                    frames += s.frames;
                    plans += s.plan_frames;
                    routes += s.routes;
                    for (total, count) in situations.iter_mut().zip(s.situations) { *total += count; }
                }
            }
            eprintln!("{fighter:?} level{level}: {plans}/{frames} plan frames; {routes} own routes; hit/block/whiff {situations:?}");
            assert!(plans * 10 >= frames && routes > 0, "{fighter:?} level{level}: {plans}/{frames} plan frames; {routes} own routes; hit/block/whiff {situations:?}");
        }
    }
}

#[test]
fn switch_off_leaves_every_sf6_fighter_on_the_old_choices() {
    for fighter in TournamentFighter::ALL {
        let Some(s) = watch(fighter, 1, 1_000, Trial { stronger: false, ..Trial::default() }) else { return };
        assert_eq!((s.plan_frames, s.routes), (0, 0), "{fighter:?}: {s:?}");
        assert!(s.starts.iter().all(|s| s.own.is_none()), "{fighter:?}: no stronger game-plan step");
    }
}

#[test]
fn sf6_fighters_use_native_catches_teleport_charges_and_the_crane() {
    use TournamentFighter::{Birdie, Geki, Gen, Lee};
    for (fighter, expected) in [
        (Geki, vec![(0x20, OwnStep::Tool(Tool::CommandThrow)), (0x1b, OwnStep::Tool(Tool::Teleport))]),
        (Birdie, vec![(0x1b, OwnStep::Tool(Tool::CommandThrow)), (0x19, OwnStep::Tool(Tool::Charge))]),
        (Lee, vec![(0x20, OwnStep::Tool(Tool::Charge))]),
        (Gen, vec![(0x1f, OwnStep::Era(EraAct::Stance)), (0x20, OwnStep::Tool(Tool::Charge))]),
    ] {
        for (slot, own) in expected {
            let charging = own == OwnStep::Tool(Tool::Charge);
            let Some(s) = watch(fighter, 1, 30_000, Trial {
                pad: if fighter == Birdie && !charging { guarding } else if matches!(fighter, Birdie | Lee) { idle } else { sparring },
                setup: if matches!(fighter, Birdie | Lee) { Some(charge_room) } else if fighter == Geki { Some(far) } else { None }, ..Trial::default()
            }) else { return };
            assert!(s.starts.iter().any(|s| s.id == slot && s.own == Some(own)), "{fighter:?}: {own:?} starts native{slot:x}: {s:?}");
            if own == OwnStep::Tool(Tool::CommandThrow) { assert!(s.command_catches > 0, "{fighter:?}: reserved command damage and THROWN victim: {s:?}"); }
            if fighter == Gen { assert!(s.crane > 0 && s.mantis == 0, "Gen changes to Crane and stays there: {s:?}"); }
        }
    }
}

#[test]
fn sf6_drive_rush_closes_in_and_deliberate_impact_pays_for_actual_contact() {
    for fighter in newcomers() {
        let Some(rush) = watch_until(fighter, 1, 10_000, Trial { pad: idle, setup: Some(rush_room), ..Trial::default() }, |s| s.rush_closes > 0) else { return };
        assert!(rush.rush_closes > 0, "{fighter:?}: a real Rush closes into a normal: {rush:?}");
        let Some(impact) = watch(fighter, 1, 10_000, Trial { pad: guarding, setup: Some(centered), ..Trial::default() }) else { return };
        assert!(impact.starts.iter().any(|s| s.id == IMPACT && s.own == Some(OwnStep::Era(EraAct::DriveImpact)) && s.drive_spent == u16::from(STOCK)), "{fighter:?}: deliberate native Impact spends48: {impact:?}");
        assert!(impact.impact_contacts > 0, "{fighter:?}: Impact hits or locks a real guard: {impact:?}");
    }
}

#[test]
fn sf6_defence_waits_for_seen_attacks_and_parries_actual_contact() {
    for fighter in newcomers() {
        let Some(melee) = watch(fighter, 1, 30_000, Trial { pad: poking, ..Trial::default() }) else { return };
        assert!(melee.reactive_impacts + melee.reactive_parries > 0, "{fighter:?}: delayed Drive answer to a real attack: {melee:?}");
        assert_eq!(melee.early_answers, 0, "{fighter:?}: no answer before its reaction delay");
        let Some(shots) = watch(fighter, 1, 12_000, Trial { pad: projectile, setup: Some(projectile_room), ..Trial::default() }) else { return };
        assert!(shots.parry_contacts > 0, "{fighter:?}: native parry refunds Drive and preserves HP on real contact: {shots:?}");
        assert_eq!(shots.early_answers, 0, "{fighter:?}: projectile answers respect the delay");
        let Some(guard) = watch(fighter, 1, 15_000, Trial { pad: pressing, ..Trial::default() }) else { return };
        assert!(guard.reversals.contains(&96), "{fighter:?}: Drive Reversal spends96 out of genuine blockstun: {guard:?}");
    }
}

#[test]
fn sf6_supers_and_overdrive_spend_their_real_native_costs() {
    for fighter in newcomers() {
        for (meter, slot) in [(48, 0x29), (96, 0x2a), (144, 0x2b)] {
            eprintln!("{fighter:?}: native Art{slot:x} with {meter} Super");
            let Some(s) = watch_until(fighter, 1, 24_000, Trial { super_meter: Some(meter), pad: sparring, setup: Some(behind_for_super), ..Trial::default() }, move |s| {
                s.starts.iter().any(|s| s.id == slot && s.super_spent == meter)
                    && (fighter != TournamentFighter::Retsu || slot != 0x2b || s.art_contacts.contains(&slot))
            }) else { return };
            assert!(s.starts.iter().any(|s| s.id == slot && s.super_spent == meter), "{fighter:?}: native Art{slot:x} spends{meter}: {s:?}");
            if fighter == TournamentFighter::Retsu && slot == 0x2b {
                assert!(s.art_contacts.contains(&slot), "Retsu's native SA3 pays144 and lands its implemented strikes: {s:?}");
            }
        }
        let Some(s) = watch(fighter, 1, 10_000, Trial { pad: idle, setup: Some(centered), ..Trial::default() }) else { return };
        assert!(s.starts.iter().any(|s| s.own == Some(OwnStep::Era(EraAct::Enhanced))
            && !matches!(s.id, RUSH | PARRY | REVERSAL | IMPACT) && s.drive_spent == 96),
            "{fighter:?}: the OD plan starts a native special paying96Drive, not Ken's free Quick Dash: {s:?}");
    }
}

#[test]
fn easy_keeps_its_meter_even_when_successive_special_motions_could_form_a_super() {
    for fighter in newcomers() {
        let Some(s) = watch(fighter, 1, 15_000, Trial { level: 1, super_meter: Some(144), pad: idle, setup: Some(centered), ..Trial::default() }) else { return };
        assert!(s.starts.iter().all(|s| s.super_spent == 0 && s.drive_spent == 0), "{fighter:?}: Easy's unthreatened play never accidentally spends paid commands: {s:?}");
        assert_eq!((s.long_easy_routes, s.paid_easy_routes), (0, 0));
    }
}

#[test]
fn sf6_gauge_rush_and_pure_footsies_rules_restrict_what_the_cpu_attempts() {
    for fighter in newcomers() {
        let Some(off) = watch(fighter, 1, 3_000, Trial { gauge: GaugeMode::NoGauges, super_meter: Some(144), ..Trial::default() }) else { return };
        assert!(off.starts.iter().all(|s| !matches!(s.id, 0x29..=0x2f) && s.super_spent == 0 && s.drive_spent == 0), "{fighter:?}: no gauged native command: {off:?}");
        let Some(no_raw) = watch(fighter, 1, 3_000, Trial { no_raw_rush: true, pad: idle, setup: Some(far), ..Trial::default() }) else { return };
        assert!(no_raw.starts.iter().all(|s| !s.raw_rush), "{fighter:?}: raw Rush disabled, cancel Rush remains legal: {no_raw:?}");
        let Some(unified) = watch(fighter, 1, 6_000, Trial { gauge: GaugeMode::Unified, super_meter: Some(96), pad: guarding, setup: Some(centered), ..Trial::default() }) else { return };
        assert!(unified.starts.iter().any(|s| s.own == Some(OwnStep::Era(EraAct::DriveImpact)) && s.id == IMPACT && s.super_spent == STOCK), "{fighter:?}: Unified Impact pays one super stock: {unified:?}");
        let Some(foot) = watch(fighter, 1, 3_000, Trial { footsies: true, super_meter: Some(144), ..Trial::default() }) else { return };
        assert_eq!((foot.illegal_mission_pads, foot.illegal_mission_routes), (0, 0), "{fighter:?}: mission restrictions govern attempted pads and eligible routes: {foot:?}");
        assert!(foot.plan_frames > 0, "{fighter:?}: legal plans remain available");
    }
}

#[test]
fn every_sf6_fighters_real_pad_waits_before_responding_to_new_information() {
    if data().is_none() { return; }
    for fighter in TournamentFighter::ALL {
      for level in 1..=4 {
        let delay = Level::from_difficulty(level).reaction();
        let (parted, changed) = deep(move || {
            let (mut parted, mut changed, mut frames, mut next) = (0, 0, 0, 300);
            for seed in 3..83 {
                let mut m = fight(fighter, true, level, seed).unwrap();
                for i in 0..36_000 {
                    let v = seen(&m);
                    if frames >= next && v.fighting && v.me.free() && v.foe.free() && !v.my_shot && !v.shot && (v.me.x - v.foe.x).abs() > 150 {
                        next = frames + 400;
                        let (mut jumped, mut stood) = (m.clone(), m.clone());
                        let mut differed = false;
                        let mut qualified = true;
                        for k in 0..delay + 90 {
                            let own_before = (seen(&jumped).me, seen(&stood).me);
                            let resources = |m: &PowerOnMachine| {
                                let own = &m.era_fighters()[1];
                                (own.sub_hp, own.super_meter, own.sf6_drive_points(), m.toggle_state().cancel.window[1], m.toggle_state().cancel.move_id[1], m.toggle_state().combo.blockstun[1], m.toggle_state().combo.hitstun[1])
                            };
                            let resources_before = (resources(&jumped), resources(&stood));
                            if k < delay && (own_before.0 != own_before.1 || resources_before.0 != resources_before.1) {
                                qualified = false;
                                break;
                            }
                            let forward = toward(&seen(&jumped));
                            step(&mut jumped, if k < 3 { (U | forward, 0) } else { (0, 0) });
                            step(&mut stood, (0, 0));
                            assert!(jumped.unrecovered().is_empty(), "{fighter:?} level{level} seed{seed} fork{i} step{k}: jumped engine stops {:x?}", jumped.unrecovered());
                            assert!(stood.unrecovered().is_empty(), "{fighter:?} level{level} seed{seed} fork{i} step{k}: stood engine stops {:x?}", stood.unrecovered());
                            let same = jumped.fair_cpu_pad() == stood.fair_cpu_pad();
                            if k < delay {
                                assert!(same, "{fighter:?} level{level}: pad changed after{k}frames, before{delay}");
                                // Equal pads can still produce different physical feedback when the
                                // opponent jumps into us. Keep that feedback out of the delayed-view
                                // comparison; an early differing pad above always fails first.
                                if seen(&jumped).me != seen(&stood).me || resources(&jumped) != resources(&stood) {
                                    qualified = false;
                                    break;
                                }
                            }
                            else if !same { differed = true; }
                        }
                        if qualified {
                            parted += 1;
                            changed += u32::from(differed);
                        }
                    }
                    frames += u32::from(v.fighting);
                    step(&mut m, sparring(i, &v));
                    assert!(m.unrecovered().is_empty(), "{fighter:?} level{level} seed{seed} frame{i}: delay setup engine stops {:x?}", m.unrecovered());
                    if m.match_over() || frames >= 12_000 { break; }
                }
                if frames >= 12_000 { return (parted, changed); }
            }
            panic!("{fighter:?} level{level}: bounded delay trial could not reach live-frame budget");
        });
        assert!(parted >= 3 && changed >= 1, "{fighter:?} level{level}: {parted} real cloned fights, {changed} responses after the delay");
      }
    }
}
