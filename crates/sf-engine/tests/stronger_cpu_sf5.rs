//! SC19 (#211): SF5 stronger Fair CPU plans and native tools through real pads.
//! Local ROMs and art supply the same player tables as the measuring run.

use std::path::Path;
use std::sync::{Arc, OnceLock};

use sf_engine::data_bank::DataBank;
use sf_engine::era::{loadouts, revisions, EraFamily, GaugeMode, Loadout, Revision};
use sf_engine::fair_cpu::{perceive, EraAct, Level, Percept, Tool};
use sf_engine::measure::{sparring_machine, MeasureSpec};
use sf_engine::roster::{PlayMode, TournamentFighter};
use sf_engine::route_book::book;
use sf_engine::run::PowerOnMachine;
use sf_video::art::{load_art_from, ArtSources};
use sf_video::art_pack::ArtPack;
use sf_video::callouts::Side;
use sf_video::era_hud::{GaugeRow, GaugeState};
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
const STOCK: u8 = 48;
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
        let art = load_art_from(
            &assets,
            &ArtSources {
                recipes: &RECIPES,
                reviews: Some(&REVIEWS),
                pack: Ok(&pack),
            },
        )
        .expect("the art loads");
        Some((
            assets.maincpu.clone(),
            Arc::new(DataBank::build(&assets.maincpu, &art.catalogues)),
        ))
    })
    .as_ref()
}

fn deep<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap()
}

#[derive(Clone, Copy, Debug)]
struct Style {
    fighter: TournamentFighter,
    revision: Revision,
    loadout: Loadout,
}

impl Style {
    fn ce(fighter: TournamentFighter, loadout: Loadout) -> Self {
        Self {
            fighter,
            revision: Revision::Sf5Ce,
            loadout,
        }
    }
}

fn styles() -> Vec<Style> {
    TournamentFighter::ALL
        .into_iter()
        .flat_map(|fighter| {
            let native = revisions(fighter, EraFamily::Sf5);
            let supported = if native.is_empty() {
                vec![Revision::Sf5Ce]
            } else {
                native
            };
            supported.into_iter().flat_map(move |revision| {
                loadouts(revision).into_iter().map(move |loadout| Style {
                    fighter,
                    revision,
                    loadout,
                })
            })
        })
        .collect()
}

fn fight(style: Style, stronger: bool, level: u8, seed: u64) -> Option<PowerOnMachine> {
    let (rom, bank) = data()?;
    let spec = MeasureSpec {
        fighters: [TournamentFighter::Ryu, style.fighter],
        styles: [
            (Revision::Sf5Ce, Loadout::Skill1Trigger1),
            (style.revision, style.loadout),
        ],
        difficulty: [level; 2],
        seed,
        stronger: [false, stronger],
    };
    Some(sparring_machine(rom, Some(bank.clone()), &spec).expect("the native fight starts"))
}

fn step(m: &mut PowerOnMachine, (dirs, buttons): (u8, u8)) {
    m.set_p1_direction(dirs & R != 0, dirs & L != 0, dirs & D != 0, dirs & U != 0);
    m.set_p1_buttons(std::array::from_fn(|i| buttons & (1 << i) != 0));
    m.step();
}

fn seen(m: &PowerOnMachine) -> Percept {
    perceive(
        m.state.frame,
        true,
        m.work_ram(),
        &m.state.objectram,
        Side::P2,
    )
}

fn toward(v: &Percept) -> u8 {
    if v.me.x >= v.foe.x {
        R
    } else {
        L
    }
}
fn away(v: &Percept) -> u8 {
    R + L - toward(v)
}
fn gap(v: &Percept) -> i16 {
    if v.me.x < v.foe.x {
        v.foe.hurt.0 - v.me.x
    } else {
        v.me.x - v.foe.hurt.1
    }
}
fn standing_pad(_: u32, _: &Percept) -> (u8, u8) {
    (0, 0)
}

fn sparring_pad(i: u32, v: &Percept) -> (u8, u8) {
    let t = i % 300;
    match (i / 300) % 8 {
        0 => (toward(v), 0),
        1 => (0, 0),
        2 => (D | away(v), 0),
        3 => {
            if t % 40 < 3 {
                (D, HK)
            } else {
                (toward(v), 0)
            }
        }
        4 => {
            if t % 45 < 3 {
                (0, HP)
            } else {
                (0, 0)
            }
        }
        5 => match t % 70 {
            0..=2 => (U | toward(v), 0),
            24..=26 => (0, HK),
            _ => (0, 0),
        },
        6 => {
            if t % 24 < 3 {
                (toward(v), HP)
            } else {
                (toward(v), 0)
            }
        }
        _ => (away(v), 0),
    }
}

fn guarding_pad(_: u32, v: &Percept) -> (u8, u8) {
    if (v.me.x - v.foe.x).abs() > 38 {
        (toward(v), 0)
    } else {
        (D | away(v), 0)
    }
}

fn pressing_pad(i: u32, v: &Percept) -> (u8, u8) {
    if (v.me.x - v.foe.x).abs() > 55 {
        (toward(v), 0)
    } else if i % 24 < 3 {
        (0, HP)
    } else {
        (0, 0)
    }
}

fn route_partner(i: u32, v: &Percept) -> (u8, u8) {
    let gap = if v.me.x < v.foe.x {
        v.foe.hurt.0 - v.me.x
    } else {
        v.me.x - v.foe.hurt.1
    };
    if gap > 60 {
        (toward(v), 0)
    } else if gap < 52 {
        (away(v), 0)
    } else if i % 80 < 3 {
        (0, HP)
    } else {
        (0, 0)
    }
}

/// Real quarter-circle-forward Hadokens offer a visible projectile well
/// before contact, so a delayed parry/counter or shift has a fair window.
fn projectile_pad(i: u32, v: &Percept) -> (u8, u8) {
    let forward = toward(v);
    match i % 85 {
        0 => (D, 0),
        1 => (D | forward, 0),
        2 => (forward, HP),
        _ => (0, 0),
    }
}

/// Let the native intro finish under human-neutral controls. Restore the
/// untouched seeded CPU only after arranging the real standing entities.
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
    for _ in 0..3_000 {
        step(m, (0, 0));
        let v = seen(m);
        if m.task_positions().contains(&(1, 0x3eafa))
            && v.me.present
            && v.foe.present
            && v.me.move_id == 1
            && v.foe.move_id == 1
        {
            ready = true;
            break;
        }
        assert!(
            m.unrecovered().is_empty(),
            "the native intro starts without an engine stop"
        );
    }
    assert!(ready, "the real standing fight is reached");
    m.set_match_config(original);
    let v = seen(m);
    let (foe_x, own_x) = positions(&v);
    let mut snapshot = m.save_snapshot();
    snapshot.fair_cpu = Some(fresh);
    m.restore_snapshot(&snapshot);
    place_centers(m, foe_x, own_x);
    let v = seen(m);
    assert_eq!((v.foe.x, v.me.x), (foe_x, own_x));
    assert_eq!((v.foe.move_id, v.me.move_id), (1, 1));
    assert_eq!(m.fair_cpu().unwrap().last_view().frame, 0);
}

fn centered_scene(m: &mut PowerOnMachine) {
    standing_scene(m, |v| (220, 220 + v.foe.hurt.1 - v.foe.x + 50));
}

fn charge_scene(m: &mut PowerOnMachine) {
    standing_scene(m, |_| (156, 328));
    let v = seen(m);
    assert!(v.me.x - v.foe.hurt.1 >= 150 && 400 - v.me.x >= 70);
}

fn projectile_scene(m: &mut PowerOnMachine) {
    standing_scene(m, |_| (150, 350));
}

fn place_centers(m: &mut PowerOnMachine, foe_x: i16, own_x: i16) {
    let v = seen(m);
    let t = m.diagnostic_telemetry();
    let mut snapshot = m.save_snapshot();
    for (entity, current, target) in [
        (t.p1.entity_offset, v.foe.x, foe_x),
        (t.p2.entity_offset, v.me.x, own_x),
    ] {
        let at = usize::from(entity) + 6;
        let x = u16::from_be_bytes([
            snapshot.state.objectram[at],
            snapshot.state.objectram[at + 1],
        ]);
        snapshot.state.objectram[at..at + 2].copy_from_slice(
            &x.wrapping_add(target.wrapping_sub(current) as u16)
                .to_be_bytes(),
        );
    }
    m.restore_snapshot(&snapshot);
}

fn far_skill_scene(m: &mut PowerOnMachine) {
    standing_scene(m, |_| (160, 355));
    assert!(
        gap(&seen(m)) >= 140,
        "the charge skill has a real safe-space opportunity"
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OwnStep {
    Tool(Tool),
    Era(EraAct),
}

fn own_step(m: &PowerOnMachine, fighter: TournamentFighter) -> Option<OwnStep> {
    let cpu = m.fair_cpu().expect("Player 2 is the Fair CPU");
    cpu.game_plan_tool(fighter, EraFamily::Sf5)
        .map(OwnStep::Tool)
        .or_else(|| cpu.game_plan_era(fighter, EraFamily::Sf5).map(OwnStep::Era))
}

#[derive(Clone, Copy, Debug, Default)]
struct V {
    points: u16,
    draining: bool,
}

fn v_gauge(m: &PowerOnMachine) -> V {
    match m.era_fighters()[1].gauge().and_then(|g| g.second) {
        Some(GaugeRow::Stock {
            full, part, state, ..
        }) => V {
            points: u16::from(full) * u16::from(STOCK)
                + part.num * u16::from(STOCK) / part.den.max(1),
            draining: state == GaugeState::Draining,
        },
        _ => V::default(),
    }
}

fn cpu_buttons(m: &PowerOnMachine) -> u8 {
    m.fair_cpu_pad()
        .unwrap()
        .1
        .iter()
        .enumerate()
        .fold(0, |bits, (i, &held)| bits | (u8::from(held) << i))
}

#[derive(Clone, Copy)]
struct Trial {
    stronger: bool,
    level: u8,
    meter: Option<u8>,
    fill_v: bool,
    gauge: GaugeMode,
    pad: fn(u32, &Percept) -> (u8, u8),
    setup: Option<fn(&mut PowerOnMachine)>,
    /// Re-space only after the CPU itself activates and both fighters recover.
    trigger_scene: Option<fn(&Percept) -> (i16, i16)>,
}

impl Default for Trial {
    fn default() -> Self {
        Self {
            stronger: true,
            level: 4,
            meter: None,
            fill_v: false,
            gauge: GaugeMode::EraDefault,
            pad: sparring_pad,
            setup: None,
            trigger_scene: None,
        }
    }
}

#[derive(Default, Debug)]
struct Seen {
    frames: u32,
    plan_frames: u32,
    routes: u32,
    foreign_routes: u32,
    starts: Vec<(u8, OwnStep)>,
    catches: Vec<OwnStep>,
    to_crane: u32,
    to_mantis: u32,
    skill_answers: u32,
    counter_strikes: u32,
    trigger_answers: u32,
    trigger_counters: u32,
    triggers: u32,
    trigger_steps: u32,
    followups: Vec<(u8, u16)>,
    trigger_catches: u32,
    ex: Vec<u8>,
    arts: Vec<u8>,
    reversals: u32,
    shifts: u32,
    shift_evades: u32,
    forbidden_defences: u32,
}

fn watch(style: Style, seed: u64, frames: u32, trial: Trial) -> Option<Seen> {
    data()?;
    Some(deep(move || {
        let mut result = Seen::default();
        let routes = book(style.fighter, style.revision, style.loadout)
            .expect("this supported style has its own route book");
        for seed in seed..seed + 80 {
            let mut m = fight(style, trial.stronger, trial.level, seed).unwrap();
            let mut config = *m.match_config();
            config.gauge_mode = trial.gauge;
            if trial.fill_v {
                config.roster.mode = PlayMode::Training;
                config.fill_meters = true;
            }
            m.set_match_config(config);
            if let Some(setup) = trial.setup {
                setup(&mut m);
            }
            let mut entering = false;
            let mut pending_grab = None;
            let mut staged_trigger = false;
            let mut partner_frame = 0;
            for _ in 0..36_000u32 {
                let mut before = seen(&m);
                if let Some(positions) = trial.trigger_scene.filter(|_| {
                    !staged_trigger && v_gauge(&m).draining && before.me.free() && before.foe.free()
                }) {
                    let (foe_x, own_x) = positions(&before);
                    place_centers(&mut m, foe_x, own_x);
                    before = seen(&m);
                    partner_frame = 0;
                    staged_trigger = true;
                }
                if let Some(points) = trial.meter.filter(|_| before.fighting) {
                    m.set_era_super_meter(1, points);
                }
                let era_before = m.era_fighters()[1];
                let v_before = v_gauge(&m);
                let step_before = own_step(&m, style.fighter);
                let answering_before = m.fair_cpu().unwrap().answering();
                let blockstun = m.toggle_state().combo.blockstun[1] != 0;
                step(&mut m, (trial.pad)(partner_frame, &before));
                partner_frame += 1;
                let now = seen(&m);
                if m.match_over() {
                    break;
                }
                if !now.fighting {
                    entering = false;
                    pending_grab = None;
                    continue;
                }
                result.frames += 1;
                result.plan_frames += u32::from(m.fair_cpu_following_game_plan());
                let cpu = m.fair_cpu().unwrap();
                let route = cpu.route_index().filter(|_| cpu.entering_route());
                if let Some(index) = route.filter(|_| !entering) {
                    result.routes += 1;
                    result.foreign_routes += u32::from(index >= routes.routes.len());
                }
                entering = route.is_some();
                let era = m.era_fighters()[1];
                let v = v_gauge(&m);
                let on = own_step(&m, style.fighter).or(step_before);
                let answer = answering_before || cpu.answering();
                let began = now.me.move_id != before.me.move_id;
                let spent = era_before.super_meter.saturating_sub(era.super_meter);
                let v_spent = v_before.points.saturating_sub(v.points);
                let grab = OwnStep::Tool(Tool::CommandThrow);
                let grab_id = match style.fighter {
                    TournamentFighter::Retsu => Some(0x21),
                    TournamentFighter::Geki => Some(0x20),
                    _ => None,
                };
                if began {
                    if let Some(on) = on {
                        result.starts.push((now.me.move_id, on));
                    }
                    if on == Some(grab) && Some(now.me.move_id) == grab_id {
                        pending_grab = Some(now.me.move_id);
                    }
                    if matches!(now.me.move_id, 0x2b | 0x2c)
                        && cpu_buttons(&m) & 0x12 == 0x12
                        && answer
                    {
                        result.skill_answers += 1;
                    }
                    // An accepted Isshin may collide in its entry frame:
                    // the native counter then returns 0x26 immediately and
                    // no observed frame has the intermediate 0x24. Its real
                    // reactive HeavyPair, ended Trigger and preserved health
                    // identify that same-frame counter without a fake slot.
                    let trigger_counter = style.fighter == TournamentFighter::Ryu
                        && now.me.move_id == 0x26
                        && v_before.draining
                        && !v.draining
                        && era.sub_hp == era_before.sub_hp
                        && answer
                        && (before.me.move_id == 0x24 || cpu_buttons(&m) & 0x24 == 0x24);
                    if now.me.move_id == 0x26
                        && (matches!(before.me.move_id, 0x24 | 0x2b | 0x2c) || trigger_counter)
                        && era.sub_hp == era_before.sub_hp
                    {
                        result.counter_strikes += 1;
                    }
                    result.trigger_counters += u32::from(trigger_counter);
                    result.trigger_answers += u32::from(trigger_counter);
                    if style.fighter == TournamentFighter::Ryu
                        && now.me.move_id == 0x24
                        && v_before.draining
                        && answer
                    {
                        result.trigger_answers += 1;
                    }
                    if !v_before.draining && v.draining {
                        result.triggers += 1;
                        result.trigger_steps +=
                            u32::from(on == Some(OwnStep::Era(EraAct::VTrigger)));
                    }
                    // The active timer makes Ken's second 0x24 distinguishable
                    // from its activation. A per-frame HUD loss of >2 points
                    // cannot be ordinary timer drain (at most one point).
                    if v_before.draining && v_spent > 2 && matches!(now.me.move_id, 0x24..=0x26) {
                        result.followups.push((now.me.move_id, v_spent));
                    }
                    if style.fighter == TournamentFighter::Ryu {
                        if matches!(now.me.move_id, 0x20..=0x22) && spent != 0 {
                            result.ex.push(spent);
                        }
                        if now.me.move_id == 0x29 && spent != 0 {
                            result.arts.push(spent);
                        }
                    }
                    if matches!(now.me.move_id, 0x2d | 0x2e) {
                        result.forbidden_defences += u32::from(v_before.draining);
                    }
                    if now.me.move_id == 0x2d && blockstun && v_spent == u16::from(STOCK) && answer
                    {
                        result.reversals += 1;
                    }
                    if now.me.move_id == 0x2e && v_spent == u16::from(STOCK) && answer {
                        result.shifts += 1;
                    }
                }
                let caught = !era_before.command_throw_pending()
                    && era.command_throw_pending()
                    && now.foe.move_id == THROWN;
                if caught {
                    if on == Some(grab) || pending_grab == Some(before.me.move_id) {
                        result.catches.push(grab);
                    }
                    if style.fighter == TournamentFighter::Birdie && before.me.move_id == 0x25 {
                        result.trigger_catches += 1;
                    }
                }
                if Some(now.me.move_id) != grab_id {
                    pending_grab = None;
                }
                if before.me.move_id == 0x2e
                    && now.me.move_id == 0x2e
                    && v.points >= v_before.points + u16::from(STOCK / 2)
                    && era.sub_hp == era_before.sub_hp
                {
                    result.shift_evades += 1;
                }
                if before.fighting && era.in_crane() != era_before.in_crane() {
                    if era.in_crane() {
                        result.to_crane += 1;
                    } else {
                        result.to_mantis += 1;
                    }
                }
                if result.frames >= frames {
                    return result;
                }
            }
        }
        panic!("{style:?}: insufficient live play for the bounded test: {result:?}");
    }))
}

#[test]
fn every_sf5_pair_uses_game_plans_and_its_own_route_book() {
    let styles = styles();
    assert_eq!(styles.len(), 59);
    for style in styles {
        let meter = (style.fighter == TournamentFighter::Mike).then_some(3 * STOCK);
        let (mut frames, mut plans, mut routes) = (0u32, 0u32, 0u32);
        for seed in 1..=12 {
            for centered in [true, false] {
                let trial = Trial {
                    meter,
                    pad: if centered {
                        standing_pad
                    } else {
                        route_partner
                    },
                    setup: centered.then_some(centered_scene),
                    ..Trial::default()
                };
                let Some(s) = watch(style, seed, 500, trial) else {
                    return;
                };
                assert_eq!(
                    s.foreign_routes, 0,
                    "{style:?} seed{seed}: the route belongs to the selected pair"
                );
                frames += s.frames;
                plans += s.plan_frames;
                routes += s.routes;
            }
        }
        eprintln!("{style:?}: {plans}/{frames} plan frames, {routes} own routes");
        assert!(
            plans * 10 >= frames && routes >= 1,
            "{style:?}: {plans}/{frames} plan frames, {routes} own routes"
        );
    }
}

#[test]
fn with_the_switch_off_no_sf5_pair_takes_a_stronger_game_plan() {
    for style in styles() {
        let Some(s) = watch(
            style,
            1,
            1_000,
            Trial {
                stronger: false,
                ..Trial::default()
            },
        ) else {
            return;
        };
        assert_eq!(s.plan_frames, 0, "{style:?}");
        assert!(s.starts.is_empty(), "{style:?}: no stronger-only step");
    }
}

#[test]
fn sf5_fighters_use_their_real_throws_teleport_charge_and_stance() {
    use TournamentFighter::{Geki, Gen, Lee, Retsu};
    let grab = OwnStep::Tool(Tool::CommandThrow);
    let charge = OwnStep::Tool(Tool::Charge);
    for (fighter, expected) in [
        (Retsu, vec![(0x21, grab)]),
        (
            Geki,
            vec![(0x20, grab), (0x1b, OwnStep::Tool(Tool::Teleport))],
        ),
        (Lee, vec![(0x20, charge)]),
        (
            Gen,
            vec![(0x1f, OwnStep::Era(EraAct::Stance)), (0x20, charge)],
        ),
    ] {
        let trial = Trial {
            pad: if fighter == Retsu {
                guarding_pad
            } else if fighter == Lee {
                standing_pad
            } else {
                sparring_pad
            },
            setup: if fighter == Lee {
                Some(charge_scene)
            } else if fighter == Retsu {
                Some(centered_scene)
            } else {
                None
            },
            ..Trial::default()
        };
        let Some(s) = watch(
            Style::ce(fighter, Loadout::Skill1Trigger1),
            if fighter == Lee { 4 } else { 1 },
            30_000,
            trial,
        ) else {
            return;
        };
        for (id, on) in expected {
            let starts = s
                .starts
                .iter()
                .filter(|&&(move_id, step)| move_id == id && step == on)
                .count();
            let catches = s.catches.iter().filter(|&&step| step == on).count();
            eprintln!("{fighter:?} {on:?}: {starts} starts, {catches} catches");
            assert!(
                starts >= 1 || on == grab && catches >= 1,
                "{fighter:?}: real native move{id:x} from its own step"
            );
            if on == grab {
                assert!(
                    catches >= 1,
                    "{fighter:?}: native reserved command damage and THROWN victim"
                );
            }
        }
        if fighter == Gen {
            assert!(
                s.to_crane >= 1 && s.to_mantis == 0,
                "Gen stays in Crane after changing: {s:?}"
            );
        }
    }
}

#[test]
fn sf5_v_skills_are_used_in_their_plans_and_ryu_answers_a_delayed_threat() {
    for fighter in TournamentFighter::ALL {
        let style = Style::ce(fighter, Loadout::Skill1Trigger1);
        let trial = if fighter == TournamentFighter::Ryu {
            Trial {
                pad: projectile_pad,
                setup: Some(projectile_scene),
                ..Trial::default()
            }
        } else if matches!(
            fighter,
            TournamentFighter::Birdie | TournamentFighter::Sagat
        ) {
            // Their skills charge at range. A close guarding partner never
            // offers that plan's140-pixel gap.
            Trial {
                pad: standing_pad,
                setup: Some(far_skill_scene),
                ..Trial::default()
            }
        } else {
            Trial {
                pad: guarding_pad,
                ..Trial::default()
            }
        };
        let Some(s) = watch(style, 1, 30_000, trial) else {
            return;
        };
        if fighter == TournamentFighter::Ryu {
            assert!(
                s.skill_answers >= 1,
                "Ryu uses his parry after a delayed threat: {s:?}"
            );
        } else {
            assert!(
                s.starts.contains(&(0x2b, OwnStep::Era(EraAct::VSkill))),
                "{fighter:?}: own V-Skill plan starts the selected native skill: {s:?}"
            );
        }
    }
}

#[test]
fn sf5_triggers_activate_and_unlock_their_actual_offensive_followups() {
    use TournamentFighter::{Birdie, Ken, Sagat};
    for fighter in [Ken, Birdie, Sagat] {
        let Some(s) = watch(
            Style::ce(fighter, Loadout::Skill1Trigger2),
            1,
            40_000,
            Trial {
                meter: Some(0),
                fill_v: true,
                pad: if fighter == Sagat {
                    route_partner
                } else {
                    standing_pad
                },
                setup: Some(centered_scene),
                ..Trial::default()
            },
        ) else {
            return;
        };
        eprintln!(
            "{fighter:?}: {} triggers, {} from steps, followups{:?}, catches{}",
            s.triggers, s.trigger_steps, s.followups, s.trigger_catches
        );
        assert!(
            s.triggers >= 1 && s.trigger_steps >= 1,
            "{fighter:?}: its V-Trigger plan actually activates the native timer"
        );
        assert!(
            s.followups
                .iter()
                .any(|&(id, spend)| id == 0x24 && spend > 2),
            "{fighter:?}: active-trigger attack spends timer rather than replaying activation"
        );
        if fighter == Ken {
            assert!(
                s.followups
                    .iter()
                    .any(|&(id, spend)| id == 0x24 && spend >= 63),
                "Ken's follow-up consumes about two thirds of the3000-frame timer"
            );
        }
        if matches!(fighter, Birdie | Sagat) {
            assert!(
                s.followups.iter().any(|&(id, _)| id == 0x25),
                "{fighter:?}: forward HeavyPair follow-up is reachable"
            );
        }
        if fighter == Birdie {
            assert!(
                s.trigger_catches >= 1,
                "Bull Capture must reserve native command damage and catch its victim"
            );
        }
        assert_eq!(
            s.forbidden_defences, 0,
            "an active trigger excludes V-Reversal and V-Shift"
        );
    }
}

#[test]
fn ryu_second_skill_and_isshin_counter_real_attacks_after_the_delay() {
    let style = Style::ce(TournamentFighter::Ryu, Loadout::Skill2Trigger2);
    let trial = Trial {
        meter: Some(0),
        pad: projectile_pad,
        setup: Some(projectile_scene),
        ..Trial::default()
    };
    let Some(skill) = watch(style, 1, 40_000, trial) else {
        return;
    };
    assert!(skill.skill_answers >= 1 && skill.counter_strikes >= 1, "Thrust Strike answers a delayed projectile and its real collision returns a strike: {skill:?}");
    // Preserve the CPU that actually activated the Trigger. Once both
    // fighters recover, arrange a fresh visible projectile opportunity
    // without changing its observations, plan, timer or native actions.
    let Some(trigger) = watch(
        style,
        1,
        40_000,
        Trial {
            fill_v: true,
            trigger_scene: Some(|_| (150, 375)),
            ..trial
        },
    ) else {
        return;
    };
    assert!(trigger.triggers >= 1 && trigger.trigger_answers >= 1 && trigger.trigger_counters >= 1, "an active Kakko Fubatsu supplies a reactive Isshin; a real collision returns its strike and ends the trigger: {trigger:?}");
}

#[test]
fn sf5_critical_art_and_ex_pay_their_actual_native_costs() {
    let style = Style::ce(TournamentFighter::Ryu, Loadout::Skill1Trigger1);
    let Some(ex) = watch(
        style,
        1,
        30_000,
        Trial {
            meter: Some(STOCK),
            pad: standing_pad,
            setup: Some(centered_scene),
            ..Trial::default()
        },
    ) else {
        return;
    };
    assert!(
        ex.ex.contains(&STOCK),
        "a real EX spends one48-point stock: {ex:?}"
    );
    assert!(ex.arts.is_empty(), "one stock cannot buy Critical Art");
    let Some(art) = watch(
        style,
        1,
        30_000,
        Trial {
            meter: Some(3 * STOCK),
            pad: standing_pad,
            setup: Some(centered_scene),
            ..Trial::default()
        },
    ) else {
        return;
    };
    assert!(
        art.arts.contains(&(3 * STOCK)),
        "the real Critical Art spends144points: {art:?}"
    );
    for trial in [
        Trial {
            level: 1,
            meter: Some(3 * STOCK),
            fill_v: true,
            ..Trial::default()
        },
        Trial {
            gauge: GaugeMode::NoGauges,
            meter: Some(3 * STOCK),
            ..Trial::default()
        },
    ] {
        let Some(s) = watch(style, 1, 8_000, trial) else {
            return;
        };
        assert!(
            s.ex.is_empty() && s.arts.is_empty() && s.triggers == 0,
            "Easy and NoGauges leave spending plans out: {s:?}"
        );
    }
}

#[test]
fn sf5_defence_uses_real_blockstun_and_ce_shift_evades_actual_contact() {
    let style = Style::ce(TournamentFighter::Birdie, Loadout::Skill1Trigger1);
    let Some(guard) = watch(
        style,
        1,
        40_000,
        Trial {
            fill_v: true,
            meter: Some(0),
            pad: pressing_pad,
            ..Trial::default()
        },
    ) else {
        return;
    };
    assert!(
        guard.reversals >= 1,
        "V-Reversal leaves real blockstun and spends48V: {guard:?}"
    );
    let Some(shift) = watch(
        style,
        1,
        40_000,
        Trial {
            fill_v: true,
            meter: Some(0),
            pad: projectile_pad,
            setup: Some(projectile_scene),
            ..Trial::default()
        },
    ) else {
        return;
    };
    assert!(
        shift.shifts >= 1 && shift.shift_evades >= 1,
        "CE Shift pays48 and an actual collision refunds24 while health is preserved: {shift:?}"
    );
    let earlier = Style {
        revision: Revision::Sf5Ae,
        ..style
    };
    let Some(ae) = watch(
        earlier,
        1,
        8_000,
        Trial {
            fill_v: true,
            pad: projectile_pad,
            setup: Some(projectile_scene),
            ..Trial::default()
        },
    ) else {
        return;
    };
    assert_eq!(ae.shifts, 0, "AE has no V-Shift");
}

#[test]
fn every_sf5_fighters_real_pad_waits_for_its_reaction_delay() {
    let delay = Level::VeryHard.reaction();
    for fighter in TournamentFighter::ALL {
        if data().is_none() {
            return;
        }
        let style = Style::ce(fighter, Loadout::Skill2Trigger2);
        let (parted, changed) = deep(move || {
            let (mut parted, mut changed, mut frames, mut next) = (0u32, 0u32, 0u32, 300u32);
            for seed in 3..83 {
                let mut m = fight(style, true, 4, seed).unwrap();
                for i in 0..36_000u32 {
                    let v = seen(&m);
                    if frames >= next
                        && v.fighting
                        && v.me.free()
                        && v.foe.free()
                        && (v.me.x - v.foe.x).abs() > 150
                    {
                        next = frames + 400;
                        parted += 1;
                        let (mut jumped, mut stood) = (m.clone(), m.clone());
                        let mut differed = false;
                        for k in 0..delay + 30 {
                            let forward = toward(&seen(&jumped));
                            step(&mut jumped, if k < 3 { (U | forward, 0) } else { (0, 0) });
                            step(&mut stood, (0, 0));
                            let same = jumped.fair_cpu_pad() == stood.fair_cpu_pad();
                            if k < delay {
                                assert!(same, "{fighter:?}: pad changed after{k}frames before the{delay}-frame delay");
                            } else if !same {
                                differed = true;
                            }
                        }
                        changed += u32::from(differed);
                    }
                    frames += u32::from(v.fighting);
                    step(&mut m, sparring_pad(i, &v));
                    if m.match_over() || frames >= 12_000 {
                        break;
                    }
                }
                if frames >= 12_000 {
                    return (parted, changed);
                }
            }
            panic!("{fighter:?}: could not reach the bounded live-frame budget");
        });
        assert!(
            parted >= 3 && changed >= 1,
            "{fighter:?}: {parted} cloned fights, {changed} actual responses after the delay"
        );
    }
}
