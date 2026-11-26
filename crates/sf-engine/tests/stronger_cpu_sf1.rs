//! SC21 (#213): SF1/SF1+ plans, native Arcade ownership, and effective rules.
//!
//! The Arcade cast fixture below changes only which ladder opponent is selected,
//! before the native round setup runs. It is synthetic ladder wiring, not an
//! authored campaign node. Ryu/Ken have no authored Capcom CPU AI tables; their
//! stronger-pad coverage is therefore separate from the ten native-AI activity
//! comparisons. The World Tour fixture also selects an otherwise unauthored
//! Arcade1987 opponent style explicitly.

use std::path::Path;
use std::sync::{Arc, OnceLock};

use sf_engine::capture::screen_hash;
use sf_engine::config::{MatchConfig, RulesetPreset, WorldTourRules};
use sf_engine::data_bank::DataBank;
use sf_engine::era::{Loadout, Revision, StylePick};
use sf_engine::fair_cpu::{perceive_fighters, Level, Percept, ResetKind, FIGHT_LOOP};
use sf_engine::fighter::{CPU_STRUCT, GAME_MODE, P1_STRUCT, P2_STRUCT};
use sf_engine::front::cabinet::{flow_position, Players, STAGE_START};
use sf_engine::front::fight::{Fight, FightDips, FightKind, FightSetup};
use sf_engine::front::PlayerInputs;
use sf_engine::measure::{sparring_machine, MeasureSpec};
use sf_engine::replay::{MatchReplay, ReplayPlayback, TakeoverOpponent, TakeoverPlayer};
use sf_engine::roster::{PlayMode, StagePick, TournamentFighter};
use sf_engine::route_book::book;
use sf_engine::run::PowerOnMachine;
use sf_engine::world_tour::native::MissionKind;
use sf_video::art::{load_art_from, ArtSources};
use sf_video::art_pack::ArtPack;
use sf_video::romset::{load_assets, Assets};

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
const BASE: u32 = 0xff8000;
const REVISIONS: [Revision; 2] = [Revision::Arcade1987, Revision::Sf1Plus];

struct Resources {
    assets: Assets,
    bank: Arc<DataBank>,
}

fn data() -> Option<&'static Resources> {
    static DATA: OnceLock<Option<Resources>> = OnceLock::new();
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
        let bank = Arc::new(DataBank::build(&assets.maincpu, &art.catalogues));
        Some(Resources { assets, bank })
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

fn word(m: &PowerOnMachine, address: u32) -> u16 {
    let at = (address - BASE) as usize;
    u16::from_be_bytes(m.work_ram()[at..at + 2].try_into().unwrap())
}

fn long(m: &PowerOnMachine, address: u32) -> u32 {
    let at = (address - BASE) as usize;
    u32::from_be_bytes(m.work_ram()[at..at + 4].try_into().unwrap())
}

/// The same live-round predicate used by the production measuring observer:
/// task1 is in the native fight loop, both bodies are present and alive.
fn seen(m: &PowerOnMachine) -> Percept {
    let mode = m.work_ram()[(GAME_MODE - BASE) as usize];
    let (own, foe) = match mode {
        2 => (CPU_STRUCT, P1_STRUCT),
        4 => (CPU_STRUCT, P2_STRUCT),
        _ => (P2_STRUCT, P1_STRUCT),
    };
    perceive_fighters(
        m.state.frame,
        flow_position(m) == FIGHT_LOOP,
        m.work_ram(),
        &m.state.objectram,
        own,
        foe,
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
fn idle(_: u32, _: &Percept) -> (u8, u8) {
    (0, 0)
}

/// A genuine human pad: approach, stand, low guard, sweep, whiff a heavy,
/// jump in, attack up close, and retreat. It reads visible body positions.
fn sparring(i: u32, v: &Percept) -> (u8, u8) {
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

fn route_partner(i: u32, v: &Percept) -> (u8, u8) {
    let gap = (v.me.x - v.foe.x).abs();
    match i % 100 {
        0..=2 => (D, LP),
        3..=60 => (D, 0),
        _ if gap > 68 => (toward(v), 0),
        _ if gap < 56 => (away(v), 0),
        _ => (0, 0),
    }
}

/// Only this function steps an engine: a stop is checked before a test can
/// count evidence, finish a proof, reach its bound, or leave a completed match.
fn step(m: &mut PowerOnMachine, (dirs, buttons): (u8, u8)) {
    let directions = [dirs & R != 0, dirs & L != 0, dirs & D != 0, dirs & U != 0];
    let buttons = std::array::from_fn(|i| buttons & (1 << i) != 0);
    if m.work_ram()[(GAME_MODE - BASE) as usize] == 4 {
        m.set_p2_direction(directions[0], directions[1], directions[2], directions[3]);
        m.set_p2_buttons(buttons);
    } else {
        m.set_p1_direction(directions[0], directions[1], directions[2], directions[3]);
        m.set_p1_buttons(buttons);
    }
    m.step();
    assert!(
        m.unrecovered().is_empty(),
        "native frame{} mode{}: engine stops {:x?}",
        m.state.frame,
        m.work_ram()[(GAME_MODE - BASE) as usize],
        m.unrecovered()
    );
}

fn local(
    fighter: TournamentFighter,
    revision: Revision,
    stronger: bool,
    level: u8,
    seed: u64,
) -> PowerOnMachine {
    let d = data().unwrap();
    let spec = MeasureSpec {
        fighters: [TournamentFighter::Ryu, fighter],
        styles: [(revision, Loadout::Standard); 2],
        difficulty: [level; 2],
        seed,
        stronger: [false, stronger],
    };
    sparring_machine(&d.assets.maincpu, Some(d.bank.clone()), &spec)
        .expect("the native Local fight starts")
}

fn start(config: MatchConfig, kind: FightKind, players: Players, rom: &[u8]) -> Fight {
    let d = data().unwrap();
    let fight = Fight::start_banked(
        rom,
        FightSetup {
            kind,
            players,
            config,
            dsw1: FightDips::DSW1,
            dsw2: FightDips {
                difficulty: config.difficulty,
                round_time: 100,
            }
            .dsw2(),
        },
        Some(0),
        None,
        Some(d.bank.clone()),
    )
    .expect("the native fight starts");
    assert!(
        fight.machine().unrecovered().is_empty(),
        "native startup stops {:x?}",
        fight.machine().unrecovered()
    );
    fight
}

/// Begin before the native round setup has bound any CPU player table.
/// An authentic opponent uses its real index in this country's ten-entry
/// native move-pointer array, preserving its corresponding AI tables.
/// Ryu/Ken replace one pointer in a fixture-owned ROM copy; no installed
/// asset is changed, no CPU move or command is started by this fixture.
fn arcade(fighter: TournamentFighter, stronger: bool, level: u8, mode: u8) -> PowerOnMachine {
    arcade_fight(fighter, Revision::Arcade1987, stronger, level, mode)
        .machine()
        .clone()
}

fn arcade_fight(
    fighter: TournamentFighter,
    revision: Revision,
    stronger: bool,
    level: u8,
    mode: u8,
) -> Fight {
    assert!(matches!(mode, 2 | 4));
    assert!(REVISIONS.contains(&revision));
    let d = data().unwrap();
    let mut config = MatchConfig::definitive();
    config.roster.mode = PlayMode::Arcade;
    config.roster.p1 = if fighter == TournamentFighter::Ryu {
        TournamentFighter::Ken
    } else {
        TournamentFighter::Ryu
    };
    config.roster.p2 = fighter;
    config.p2_fair_cpu = true;
    config.stronger_cpu = stronger;
    config.difficulty = level;
    let discover = start(config, FightKind::Arcade, Players::One, &d.assets.maincpu);
    assert_eq!(
        flow_position(discover.machine()),
        STAGE_START,
        "the fixture precedes native round setup"
    );
    let base = long(discover.machine(), 0xffc01c) as usize;
    let native_index = (0..10usize).find(|&index| {
        u32::from_be_bytes(
            d.assets.maincpu[base + index * 4..base + index * 4 + 4]
                .try_into()
                .unwrap(),
        ) == fighter.move_table()
    });
    assert_eq!(
        native_index.is_none(),
        fighter.is_shoto(),
        "the native ladder has ten authored CPU fighters"
    );
    let index = native_index.unwrap_or(0);
    let at = base + index * 4;
    let mut rom = d.assets.maincpu.clone();
    let original = u32::from_be_bytes(rom[at..at + 4].try_into().unwrap());
    rom[at..at + 4].copy_from_slice(&fighter.move_table().to_be_bytes());
    assert!(rom
        .iter()
        .zip(&d.assets.maincpu)
        .enumerate()
        .all(|(i, (a, b))| a == b || (at..at + 4).contains(&i)));
    eprintln!("SC21 Arcade fixture {fighter:?} {revision:?} mode{mode}: native pointer {at:#x} {original:#x}->{:#x}, ladder index{index}, nativeAIIndex{}", fighter.move_table(), native_index.is_some());
    let mut fight = start(config, FightKind::Arcade, Players::One, &rom);
    fight.set_selected_styles([Some(StylePick::for_revision(revision, Loadout::Standard)); 2]);
    let mut machine = fight.machine().clone();
    let mut snapshot = machine.save_snapshot();
    snapshot.work[0x4003] = (index / 2) as u8;
    snapshot.work[0x4004] = (index % 2) as u8;
    if mode == 4 {
        // P2 is the sole human side in mode4. Move the native start's human
        // record before placement and use its own cabinet input-copy word.
        let a = (P1_STRUCT - BASE) as usize;
        let b = (P2_STRUCT - BASE) as usize;
        for i in 0..(b - a) {
            snapshot.work.swap(a + i, b + i);
        }
        snapshot.work[b + 2..b + 6].copy_from_slice(&0xff868eu32.to_be_bytes());
    }
    snapshot.work[(GAME_MODE - BASE) as usize] = mode;
    machine.restore_snapshot(&snapshot);
    assert_eq!(long(&machine, 0xffc01c), base as u32);
    let mut wrapper = fight.save_netplay_snapshot(0);
    wrapper.machine = machine.save_snapshot();
    fight.restore_netplay_snapshot(&wrapper);
    fight
}

/// Arrange only starting positions after genuine standing entities appear.
/// The controller/history is restored from its fresh seeded state; no hit,
/// accepted move, hitstun, resource, or intent is supplied by the fixture.
fn centered(m: &mut PowerOnMachine) {
    let fresh = m.fair_cpu().cloned().expect("fresh Local CPU");
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
        if v.fighting && v.me.move_id == 1 && v.foe.move_id == 1 {
            ready = true;
            break;
        }
    }
    assert!(ready, "genuine native standing entities are reached");
    m.set_match_config(original);
    let v = seen(m);
    let t = m.diagnostic_telemetry();
    let mut snapshot = m.save_snapshot();
    snapshot.fair_cpu = Some(fresh);
    for (entity, current, target) in [
        (t.p1.entity_offset, v.foe.x, 220),
        (
            t.p2.entity_offset,
            v.me.x,
            220 + v.foe.hurt.1 - v.foe.x + 50,
        ),
    ] {
        let at = usize::from(entity) + 6;
        let x = u16::from_be_bytes(snapshot.state.objectram[at..at + 2].try_into().unwrap());
        snapshot.state.objectram[at..at + 2].copy_from_slice(
            &x.wrapping_add(target.wrapping_sub(current) as u16)
                .to_be_bytes(),
        );
    }
    m.restore_snapshot(&snapshot);
}

#[derive(Clone, Copy, Debug)]
struct Limits {
    combos: bool,
    cancels: bool,
    throws: bool,
}

#[derive(Debug, Default)]
struct Observed {
    frames: u32,
    matches_completed: u32,
    plans: u32,
    pad_frames: u32,
    attacks: u32,
    plan_attacks: u32,
    pad_started_attacks: u32,
    bookless_plan_attacks: u32,
    specials: u32,
    idle_frames: u32,
    human_attacks: u32,
    human_moves: u32,
    own_routes: u32,
    connected: u32,
    cancels: u32,
    max_combo: u8,
    cpu_table_frames: u32,
}

impl Observed {
    fn add(&mut self, other: Self) {
        self.frames += other.frames;
        self.matches_completed += other.matches_completed;
        self.plans += other.plans;
        self.pad_frames += other.pad_frames;
        self.attacks += other.attacks;
        self.plan_attacks += other.plan_attacks;
        self.pad_started_attacks += other.pad_started_attacks;
        self.bookless_plan_attacks += other.bookless_plan_attacks;
        self.specials += other.specials;
        self.idle_frames += other.idle_frames;
        self.human_attacks += other.human_attacks;
        self.human_moves += other.human_moves;
        self.own_routes += other.own_routes;
        self.connected += other.connected;
        self.cancels += other.cancels;
        self.max_combo = self.max_combo.max(other.max_combo);
        self.cpu_table_frames += other.cpu_table_frames;
    }
}

/// A fixed native trial. Its only early exit is a naturally completed match;
/// it never retries a seed, omits a stop, or swaps a problematic address out.
fn watch(
    mut m: PowerOnMachine,
    fighter: TournamentFighter,
    revision: Revision,
    frames: u32,
    limits: Limits,
    human: fn(u32, &Percept) -> (u8, u8),
) -> Observed {
    let mut out = Observed::default();
    let mut entering = false;
    let mut last_button = None;
    let own_book = book(fighter, revision, Loadout::Standard);
    for i in 0..24_000 {
        let before = seen(&m);
        let plan_before = m.fair_cpu_following_game_plan();
        let route_before = m.fair_cpu_entering_route();
        let mode = m.work_ram()[0x4002];
        let own_slot = usize::from(mode != 4);
        let foe_slot = 1 - own_slot;
        let cancel_before = m.toggle_state().cancel.count[own_slot];
        let human_input = human(i, &before);
        step(&mut m, human_input);
        let now = seen(&m);
        if now.fighting {
            out.frames += 1;
            let in_plan = m.fair_cpu_following_game_plan();
            out.plans += u32::from(in_plan);
            if let Some(cpu) = m.fair_cpu() {
                let pad = cpu.pad();
                assert_ne!(
                    pad & 3,
                    3,
                    "{fighter:?} {revision:?}: opposite horizontal pad bits"
                );
                assert_ne!(
                    pad & 12,
                    12,
                    "{fighter:?} {revision:?}: opposite vertical pad bits"
                );
                assert_eq!(
                    pad & !0x3ff,
                    0,
                    "{fighter:?} {revision:?}: only native direction/button bits"
                );
                out.pad_frames += u32::from(pad != 0);
                if pad >> 4 != 0 {
                    last_button = Some(i);
                }
                assert!(
                    limits.throws || cpu.route_reset() != Some(ResetKind::Throw),
                    "throws-off forbids attempted throw resets"
                );
                if cpu.entering_route() {
                    let route = own_book
                        .and_then(|b| cpu.route_index().and_then(|index| b.routes.get(index)))
                        .expect("every entered route belongs to this fighter/revision");
                    assert!(
                        !route.true_combos || limits.combos,
                        "{fighter:?}: route needs disabled true combos"
                    );
                    assert!(
                        !route.cancels || limits.cancels,
                        "{fighter:?}: route needs disabled cancels"
                    );
                    assert!(
                        route.hits
                            <= Level::from_difficulty(m.match_config().difficulty).route_hits(),
                        "difficulty route length cap"
                    );
                    if !entering {
                        out.own_routes += 1;
                    }
                }
                entering = cpu.entering_route();
            } else {
                entering = false;
            }
            // These definitions intentionally match measure_match exactly.
            if before.fighting {
                let attack = now.me.attacking()
                    && (!before.me.attacking() || now.me.move_id != before.me.move_id);
                out.attacks += u32::from(attack);
                out.idle_frames +=
                    u32::from(now.me.free() && before.me.free() && now.me.x == before.me.x);
                out.human_attacks += u32::from(
                    now.foe.attacking()
                        && (!before.foe.attacking() || now.foe.move_id != before.foe.move_id),
                );
                out.human_moves += u32::from(now.foe.x != before.foe.x);
                out.plan_attacks += u32::from(attack && (in_plan || plan_before));
                out.pad_started_attacks += u32::from(
                    attack
                        && (in_plan || plan_before)
                        && last_button.is_some_and(|at| i - at <= 30),
                );
                out.bookless_plan_attacks +=
                    u32::from(attack && (in_plan || plan_before) && !route_before && !entering);
                out.specials += u32::from(attack && (0x19..=0x1b).contains(&now.me.move_id));
                out.connected +=
                    u32::from(now.foe.health < before.foe.health && !now.foe.guarding());
            }
            out.max_combo = out.max_combo.max(m.toggle_state().combo.hits[foe_slot]);
            out.cancels +=
                u32::from(m.toggle_state().cancel.count[own_slot].saturating_sub(cancel_before));
            if !limits.combos {
                assert!(
                    m.toggle_state().combo.hits[foe_slot] <= 1,
                    "{fighter:?}: no native combo beyond effective rules"
                );
            }
            if !limits.cancels {
                assert_eq!(
                    m.toggle_state().cancel.count[own_slot],
                    0,
                    "{fighter:?}: no native cancel beyond effective rules"
                );
            }
            if !limits.throws {
                assert_ne!(
                    now.foe.move_id, 0x3e,
                    "{fighter:?}: no thrown victim when throws are off"
                );
            }
            if matches!(mode, 2 | 4) && m.fair_cpu().is_some() {
                assert_eq!(
                    m.era_fighters()[own_slot].style.revision,
                    revision,
                    "mode{mode}: the CPU actually plays the selected baseline revision"
                );
                assert_eq!(
                    m.era_fighters()[own_slot].fighter,
                    Some(fighter),
                    "mode{mode}: the era slot belongs to the selected native CPU fighter"
                );
                assert_eq!(
                    TournamentFighter::from_move_table(long(&m, CPU_STRUCT + 0x12)),
                    Some(fighter),
                    "native CPU_STRUCT table has the intended fighter"
                );
                if !fighter.is_shoto() {
                    assert_eq!(
                        long(&m, CPU_STRUCT + 0x12),
                        data().unwrap().bank.table(fighter).unwrap(),
                        "native setup converted the CPU to its real player table"
                    );
                }
                let human_struct = if mode == 2 { P1_STRUCT } else { P2_STRUCT };
                let cpu_input_copy = if mode == 2 { 0xff868e } else { 0xff868f };
                let human_input_copy = if mode == 2 { 0xff868f } else { 0xff868e };
                assert_eq!(
                    long(&m, CPU_STRUCT + 2),
                    cpu_input_copy,
                    "mode{mode}: CPU reads its own cabinet byte"
                );
                assert_eq!(
                    long(&m, human_struct + 2),
                    human_input_copy,
                    "mode{mode}: human reads the other cabinet byte"
                );
                assert_eq!(
                    !m.work_ram()[(cpu_input_copy - BASE) as usize] & 15,
                    m.fair_cpu().unwrap().pad() as u8 & 15,
                    "native CPU direction-copy byte contains its actual emitted pad"
                );
                assert_eq!(
                    !m.work_ram()[(human_input_copy - BASE) as usize] & 15,
                    human_input.0 & 15,
                    "native human direction-copy byte contains its independent script"
                );
                assert_eq!(
                    TournamentFighter::from_move_table(long(&m, human_struct + 0x12)),
                    Some(m.match_config().roster.p1),
                    "mode{mode}: the human occupies its own native struct"
                );
                assert_ne!(
                    word(&m, human_struct + 0x10),
                    word(&m, CPU_STRUCT + 0x10),
                    "human/CPU native entities are independent"
                );
                out.cpu_table_frames += 1;
            }
        } else {
            entering = false;
        }
        let completed = m.match_over();
        if out.frames >= frames || completed {
            out.matches_completed += u32::from(completed);
            return out;
        }
    }
    panic!("bounded native trial did not finish its live-frame budget: {fighter:?} {revision:?}: {out:?}");
}

fn modern() -> Limits {
    Limits {
        combos: true,
        cancels: true,
        throws: true,
    }
}
fn original() -> Limits {
    Limits {
        combos: false,
        cancels: false,
        throws: false,
    }
}

#[test]
fn all_twenty_four_pairs_start_native_attacks_from_plans_at_every_level() {
    if data().is_none() {
        return;
    }
    deep(|| {
        for fighter in TournamentFighter::ALL {
            for revision in REVISIONS {
                for level in 1..=4 {
                    let mut total = Observed::default();
                    // A fixed sample schedule, including ordinary interactive
                    // fights and a standing opportunity; no failure retries.
                    for seed in 1..=4 {
                        let mut m = local(fighter, revision, true, level, seed);
                        if seed == 1 {
                            centered(&mut m);
                        }
                        total.add(watch(
                            m,
                            fighter,
                            revision,
                            2_400,
                            if revision == Revision::Arcade1987 {
                                original()
                            } else {
                                modern()
                            },
                            if seed == 1 { idle } else { route_partner },
                        ));
                    }
                    eprintln!("SC21 Local {fighter:?} {revision:?} level{level}: {total:?}");
                    assert!(total.plans > 0 && total.pad_frames > 0 && total.pad_started_attacks > 0, "{fighter:?} {revision:?} level{level}: plans produce consumed native attacks: {total:?}");
                    if revision == Revision::Sf1Plus && fighter != TournamentFighter::Geki {
                        assert!(total.own_routes > 0, "{fighter:?} level{level}: its own SF1+ route is actually entered: {total:?}");
                    } else {
                        assert_eq!(
                            total.own_routes, 0,
                            "1987 and bookless Geki take no invented route"
                        );
                    }
                }
            }
        }
    });
}

#[test]
fn bookless_geki_sf1plus_cancels_real_native_normals_only_when_allowed() {
    if data().is_none() {
        return;
    }
    deep(|| {
        assert!(book(
            TournamentFighter::Geki,
            Revision::Sf1Plus,
            Loadout::Standard
        )
        .is_none());
        for cancels in [false, true] {
            let mut total = Observed::default();
            // The same fixed native scenes on both sides of the rule gate.
            // The real Geki bank and his own seeded Fair CPU are retained.
            for seed in 1..=12 {
                let mut machine = local(TournamentFighter::Geki, Revision::Sf1Plus, true, 4, seed);
                let mut config = *machine.match_config();
                config.preset = RulesetPreset::Custom;
                config.toggles.balanced_roster = true;
                config.toggles.true_combos = true;
                config.toggles.cancels = cancels;
                machine.set_match_config(config);
                total.add(watch(
                    machine,
                    TournamentFighter::Geki,
                    Revision::Sf1Plus,
                    3_000,
                    Limits {
                        cancels,
                        ..modern()
                    },
                    sparring,
                ));
            }
            eprintln!("SC21 real bookless Geki SF1+ cancels{cancels}: {total:?}");
            assert!(total.bookless_plan_attacks > 0 && total.specials > 0 && total.connected > 0, "bookless fallback starts and lands real actions; no unavailable two-hit route is invented: {total:?}");
            assert_eq!(total.own_routes, 0);
            if cancels {
                assert!(total.cancels > 0, "Geki's own real native normal is cancelled into a special when that independent rule allows it: {total:?}");
            } else {
                assert_eq!(
                    total.cancels, 0,
                    "the same bookless Geki cannot cancel when the rule is off: {total:?}"
                );
            }
        }
    });
}

#[test]
fn switch_off_leaves_all_twenty_four_pairs_without_stronger_plans_or_routes() {
    if data().is_none() {
        return;
    }
    deep(|| {
        for fighter in TournamentFighter::ALL {
            for revision in REVISIONS {
                let out = watch(
                    local(fighter, revision, false, 4, 1),
                    fighter,
                    revision,
                    1_200,
                    if revision == Revision::Arcade1987 {
                        original()
                    } else {
                        modern()
                    },
                    sparring,
                );
                assert_eq!(
                    (out.plans, out.own_routes),
                    (0, 0),
                    "{fighter:?} {revision:?}: {out:?}"
                );
            }
        }
    });
}

#[test]
fn native_arcade_modes_two_and_four_consume_stronger_pads_for_every_fighter_and_level() {
    if data().is_none() {
        return;
    }
    deep(|| {
        for fighter in TournamentFighter::ALL {
            for mode in [2, 4] {
                for level in 1..=4 {
                    let out = watch(
                        arcade(fighter, true, level, mode),
                        fighter,
                        Revision::Arcade1987,
                        2_400,
                        original(),
                        sparring,
                    );
                    eprintln!("SC21 Arcade {fighter:?} mode{mode} level{level}: {out:?}");
                    assert!(out.frames >= 600 && out.cpu_table_frames > 0 && out.pad_frames > 0 && out.pad_started_attacks > 0, "native CPU_STRUCT consumes its pad into real attacks: {fighter:?} mode{mode} level{level}: {out:?}");
                    assert!(
                        out.human_attacks > 0 && out.human_moves > 0,
                        "mode{mode} independently consumes the scripted human pad: {out:?}"
                    );
                    assert_eq!(out.own_routes, 0);
                }
            }
        }
    });
}

#[test]
fn native_sf1plus_arcade_modes_two_and_four_consume_every_fighters_stronger_pad() {
    if data().is_none() {
        return;
    }
    deep(|| {
        for fighter in TournamentFighter::ALL {
            for mode in [2, 4] {
                let fight = arcade_fight(fighter, Revision::Sf1Plus, true, 4, mode);
                let out = watch(
                    fight.machine().clone(),
                    fighter,
                    Revision::Sf1Plus,
                    2_400,
                    modern(),
                    sparring,
                );
                eprintln!("SC21 SF1+ Arcade {fighter:?} mode{mode} level4: {out:?}");
                assert!(out.frames >= 600 && out.cpu_table_frames > 0 && out.pad_frames > 0 && out.pad_started_attacks > 0,
                    "SF1+ native CPU_STRUCT consumes its pad into real attacks: {fighter:?} mode{mode}: {out:?}");
                assert!(
                    out.human_attacks > 0 && out.human_moves > 0,
                    "SF1+ mode{mode} independently consumes the scripted human pad: {out:?}"
                );
            }
        }
    });
}

#[test]
fn faithful_stronger_flag_preserves_raw_ram_objects_audio_render_and_whole_snapshot() {
    let Some(d) = data() else { return };
    deep(move || {
        for level in 1..=4 {
            let mut config = MatchConfig::from_preset(RulesetPreset::Faithful1987);
            config.roster.mode = PlayMode::Arcade;
            config.p2_fair_cpu = true;
            config.difficulty = level;
            let mut off = start(config, FightKind::Arcade, Players::One, &d.assets.maincpu)
                .machine()
                .clone();
            let mut on = off.clone();
            config.stronger_cpu = true;
            on.set_match_config(config);
            for i in 0..3_600 {
                let pad = sparring(i, &seen(&off));
                step(&mut off, pad);
                step(&mut on, pad);
                assert!(
                    off.fair_cpu_pad().is_none() && on.fair_cpu_pad().is_none(),
                    "Faithful never installs a Fair CPU"
                );
                assert_eq!(
                    on.work_ram(),
                    off.work_ram(),
                    "level{level} frame{i}: raw work RAM"
                );
                assert_eq!(
                    on.state, off.state,
                    "level{level} frame{i}: raw objects/video/audio registers"
                );
                assert_eq!(
                    on.latch_writes(),
                    off.latch_writes(),
                    "level{level} frame{i}: exact sound commands/timing"
                );
                if i % 60 == 0 {
                    assert_eq!(
                        screen_hash(&on.render_faithful_with_overlay(&d.assets)),
                        screen_hash(&off.render_faithful_with_overlay(&d.assets)),
                        "level{level} frame{i}: rendered frame hash"
                    );
                    let mut normalized = on.clone();
                    let mut snapshot = normalized.save_snapshot();
                    snapshot.match_config.stronger_cpu = false;
                    normalized.restore_snapshot(&snapshot);
                    assert_eq!(
                        normalized.gameplay_hash(),
                        off.gameplay_hash(),
                        "the config flag is the sole canonical difference"
                    );
                    assert_eq!(
                        normalized.save_snapshot(),
                        off.save_snapshot(),
                        "the entire snapshot agrees after normalizing only that flag"
                    );
                }
            }
        }
    });
}

#[test]
fn world_tour_1987_style_fixture_uses_real_plans_without_modern_offense() {
    if data().is_none() {
        return;
    }
    deep(|| {
        for fighter in TournamentFighter::ALL {
            let d = data().unwrap();
            let mut config = MatchConfig::definitive();
            config.roster.mode = PlayMode::Versus;
            config.roster.p2 = fighter;
            config.p2_fair_cpu = true;
            config.stronger_cpu = true;
            config.difficulty = 4;
            config.world_tour = Some(WorldTourRules {
                mission: MissionKind::RivalBout,
                bonuses: None,
                starting_hp: None,
            });
            let mut fight = start(
                config,
                FightKind::WorldTour,
                Players::Two,
                &d.assets.maincpu,
            );
            // No authored node presently selects this style. Exercise the
            // actual WorldTour/Players::Two path with that style selected.
            fight.set_selected_styles(
                [Some(StylePick::for_revision(
                    Revision::Arcade1987,
                    Loadout::Standard,
                )); 2],
            );
            assert_eq!(fight.setup().kind, FightKind::WorldTour);
            assert_eq!(fight.setup().players, Players::Two);
            let out = watch(
                fight.machine().clone(),
                fighter,
                Revision::Arcade1987,
                2_400,
                original(),
                sparring,
            );
            assert!(
                out.plans > 0 && out.plan_attacks > 0,
                "World Tour fixture {fighter:?}: {out:?}"
            );
            assert_eq!((out.own_routes, out.cancels), (0, 0));
        }
    });
}

#[test]
fn classic_pure_custom_and_throws_off_restrict_native_offense_and_route_resets() {
    if data().is_none() {
        return;
    }
    deep(|| {
        for fighter in TournamentFighter::ALL {
            for revision in REVISIONS {
                for preset in [
                    RulesetPreset::ClassicCompetitive,
                    RulesetPreset::PureFootsies1987Plus,
                ] {
                    let mut m = local(fighter, revision, true, 4, 1);
                    let mut config = MatchConfig::from_preset(preset);
                    config.roster = m.match_config().roster;
                    config.p2_fair_cpu = true;
                    config.stronger_cpu = true;
                    config.difficulty = 4;
                    m.set_match_config(config);
                    let out = watch(m, fighter, revision, 1_800, original(), sparring);
                    assert_eq!(
                        (out.own_routes, out.cancels),
                        (0, 0),
                        "{fighter:?} {revision:?} {preset:?}: {out:?}"
                    );
                    assert!(
                        out.plan_attacks > 0,
                        "legal stronger offense remains available"
                    );
                }
                for combos in [false, true] {
                    for cancels in [false, true] {
                        let mut m = local(fighter, revision, true, 4, 3);
                        let mut config = *m.match_config();
                        config.preset = RulesetPreset::Custom;
                        config.toggles.balanced_roster = true;
                        config.toggles.true_combos = combos;
                        config.toggles.cancels = cancels;
                        config.toggles.throws = false;
                        m.set_match_config(config);
                        let limits = Limits {
                            combos: combos && revision != Revision::Arcade1987,
                            cancels: cancels && revision != Revision::Arcade1987,
                            throws: false,
                        };
                        let out = watch(m, fighter, revision, 2_400, limits, route_partner);
                        assert!(out.plans > 0 && out.plan_attacks > 0, "{fighter:?} {revision:?} combos{combos} cancels{cancels}: legal native plan attacks: {out:?}");
                    }
                }
            }
        }
    });
}

#[test]
fn all_twenty_four_pairs_snapshot_and_resume_identically() {
    if data().is_none() {
        return;
    }
    deep(|| {
        for fighter in TournamentFighter::ALL {
            for revision in REVISIONS {
                let mut m = local(fighter, revision, true, 4, 7);
                for i in 0..1_200 {
                    let v = seen(&m);
                    step(&mut m, sparring(i, &v));
                }
                let snapshot = m.save_snapshot();
                let mut first = m.clone();
                let mut expected = Vec::new();
                let mut pads = Vec::new();
                for i in 0..360 {
                    let pad = sparring(1_200 + i, &seen(&first));
                    pads.push(pad);
                    step(&mut first, pad);
                    expected.push((
                        first.gameplay_hash(),
                        first.fair_cpu_pad(),
                        first.latch_writes().to_vec(),
                    ));
                }
                m.restore_snapshot(&snapshot);
                for (i, pad) in pads.into_iter().enumerate() {
                    step(&mut m, pad);
                    assert_eq!(
                        (
                            m.gameplay_hash(),
                            m.fair_cpu_pad(),
                            m.latch_writes().to_vec()
                        ),
                        expected[i],
                        "{fighter:?} {revision:?} restored frame{i}"
                    );
                }
                assert_eq!(
                    m.save_snapshot(),
                    first.save_snapshot(),
                    "{fighter:?} {revision:?}: complete restored machine"
                );
            }
        }
    });
}

#[test]
fn all_twenty_four_pairs_wait_the_minimum_reaction_prefix_at_every_level() {
    if data().is_none() {
        return;
    }
    deep(|| {
        for fighter in TournamentFighter::ALL {
            for revision in REVISIONS {
                for level in 1..=4 {
                    let delay = Level::from_difficulty(level).reaction();
                    let (mut qualified, mut later_changes) = (0, 0);
                    // Fixed independent samples, never selected by their pads.
                    for seed in 3..=10 {
                        let mut m = local(fighter, revision, true, level, seed);
                        let mut live = 0u32;
                        let mut next = 300u32;
                        for i in 0..12_000 {
                            let v = seen(&m);
                            if live >= next
                                && v.fighting
                                && v.me.free()
                                && v.foe.free()
                                && !v.my_shot
                                && !v.shot
                                && (v.me.x - v.foe.x).abs() > 150
                            {
                                next = live + 400;
                                let (mut jumped, mut stood) = (m.clone(), m.clone());
                                let resources = |m: &PowerOnMachine| {
                                    let own = &m.era_fighters()[1];
                                    (
                                        own.sub_hp,
                                        own.super_meter,
                                        m.toggle_state().cancel.window[1],
                                        m.toggle_state().cancel.move_id[1],
                                        m.toggle_state().combo.blockstun[1],
                                        m.toggle_state().combo.hitstun[1],
                                    )
                                };
                                let (mut clean, mut changed) = (true, false);
                                for k in 0..delay + 90 {
                                    if k < delay
                                        && (seen(&jumped).me != seen(&stood).me
                                            || resources(&jumped) != resources(&stood))
                                    {
                                        clean = false;
                                        break;
                                    }
                                    let forward = toward(&seen(&jumped));
                                    step(
                                        &mut jumped,
                                        if k < 3 { (U | forward, 0) } else { (0, 0) },
                                    );
                                    step(&mut stood, (0, 0));
                                    let same = jumped.fair_cpu_pad() == stood.fair_cpu_pad();
                                    if k < delay {
                                        // Compare the pad first. Diverging own-state feedback can
                                        // disqualify equal-pad samples only, never hide an early leak.
                                        assert!(same, "{fighter:?} {revision:?} level{level}: pad changed after{k}frames before{delay}");
                                        if seen(&jumped).me != seen(&stood).me
                                            || resources(&jumped) != resources(&stood)
                                        {
                                            clean = false;
                                            break;
                                        }
                                    } else if !same {
                                        changed = true;
                                    }
                                }
                                if clean {
                                    qualified += 1;
                                    later_changes += u32::from(changed);
                                }
                            }
                            live += u32::from(v.fighting);
                            step(&mut m, sparring(i, &v));
                            if m.match_over() || live >= 3_000 {
                                break;
                            }
                        }
                    }
                    assert!(qualified >= 3 && later_changes > 0, "{fighter:?} {revision:?} level{level}: {qualified} qualified native forks, {later_changes} delayed responses");
                }
            }
        }
    });
}

#[test]
fn arcade_capcom_and_stronger_activity_diagnostic() {
    if data().is_none() {
        return;
    }
    deep(|| {
        eprintln!("SC21_ARCADE_ACTIVITY\tfighter\tmode\tlevel\tstronger\tlive_frames\tattacks\tidle_frames\tplan_attacks\thuman_attacks\thuman_moves\tmatches_completed");
        // Ten authored Capcom opponents only. The native shoto fixtures above
        // prove stronger control, and deliberately make no Capcom-AI claim.
        // Each observation lasts up to 6,000 live frames or the first actual
        // native match end. A legitimately shorter completed match is retained;
        // live duration and completion count accompany the raw activity counts.
        for fighter in TournamentFighter::ALL.into_iter().filter(|f| !f.is_shoto()) {
            for mode in [2, 4] {
                for stronger in [false, true] {
                    let out = watch(
                        arcade(fighter, stronger, 4, mode),
                        fighter,
                        Revision::Arcade1987,
                        6_000,
                        original(),
                        sparring,
                    );
                    assert!(out.frames > 0 && (out.frames >= 600 || out.matches_completed > 0) && out.attacks > 0 && out.human_attacks > 0 && out.human_moves > 0, "bounded native activity is observed: {fighter:?} mode{mode} stronger{stronger}: {out:?}");
                    if stronger {
                        assert!(out.cpu_table_frames > 0 && out.plan_attacks > 0);
                    } else {
                        assert_eq!(
                            (
                                out.plans,
                                out.pad_frames,
                                out.own_routes,
                                out.cpu_table_frames
                            ),
                            (0, 0, 0, 0),
                            "the old Arcade run is Capcom, not a forced old Fair CPU"
                        );
                    }
                    eprintln!("SC21_ARCADE_ACTIVITY\t{fighter:?}\t{mode}\t4\t{stronger}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", out.frames, out.attacks, out.idle_frames, out.plan_attacks, out.human_attacks, out.human_moves, out.matches_completed);
                }
            }
        }
    });
}

/// The real Fight wrapper and ReplayPlayback path, starting at a native
/// mode4 standing fight. Its single recorded frame intentionally exhausts
/// before frame2 so both takeover seats exercise their fallback opponent.
fn mode_four_playback(revision: Revision) -> ReplayPlayback {
    let d = data().unwrap();
    let mut fight = arcade_fight(TournamentFighter::Retsu, revision, true, 4, 4);
    let mut ready = false;
    for _ in 0..3_000 {
        let frame = fight.step(
            &d.assets,
            PlayerInputs::default(),
            PlayerInputs::default(),
            false,
        );
        assert!(
            fight.machine().unrecovered().is_empty(),
            "mode4 replay startup engine stops {:x?}",
            fight.machine().unrecovered()
        );
        assert!(
            frame.event.is_none(),
            "mode4 replay startup event {:?}",
            frame.event
        );
        let v = seen(fight.machine());
        if v.fighting && v.me.free() && v.foe.free() {
            ready = true;
            break;
        }
    }
    assert!(
        ready,
        "the mode4 native replay fixture reaches a standing fight"
    );
    assert_eq!(
        fight.machine().era_fighters()[0].style.revision,
        revision,
        "the native mode4 replay CPU actually plays the requested revision"
    );
    assert_eq!(
        TournamentFighter::from_move_table(long(fight.machine(), P2_STRUCT + 0x12)),
        Some(TournamentFighter::Ryu)
    );
    assert_eq!(
        TournamentFighter::from_move_table(long(fight.machine(), CPU_STRUCT + 0x12)),
        Some(TournamentFighter::Retsu)
    );
    let style = StylePick::for_revision(revision, Loadout::Standard);
    let replay = MatchReplay {
        id: 213,
        title: format!("SC21 synthetic mode4 {revision:?} ladder fixture"),
        kind: FightKind::Arcade,
        online: false,
        config: fight.setup().config,
        dips: FightDips {
            difficulty: 4,
            round_time: 100,
        },
        stage: StagePick::Bank(0),
        flag: Some(0),
        seed: 213,
        // ReplayPlayback::new takes an already constructed Fight and does
        // no session data-compatibility lookup. No fake bank is installed.
        data_hash: 0,
        p1_fighter: TournamentFighter::Retsu.into(),
        p2_fighter: TournamentFighter::Ryu.into(),
        p1_style: style,
        p2_style: style,
        theme: None,
        frames: vec![(0, 0)],
        commands: Vec::new(),
    };
    ReplayPlayback::new(replay, fight).with_stronger_cpu(true)
}

fn replay_mode_four_branch(
    playback: &mut ReplayPlayback,
    side: TakeoverPlayer,
) -> Vec<(u64, u8, u8, u16)> {
    let d = data().unwrap();
    assert!(playback.takeover(side));
    let mut reference = playback.fight().clone();
    let reference_frame = reference.step(
        &d.assets,
        PlayerInputs::default(),
        PlayerInputs::default(),
        false,
    );
    assert!(
        reference.machine().unrecovered().is_empty(),
        "recorded mode4 reference engine stops {:x?}",
        reference.machine().unrecovered()
    );
    assert!(reference_frame.event.is_none());
    let recorded_cpu_byte = !reference.machine().work_ram()[(0xff868f - BASE) as usize] & 15;
    let mut observations = Vec::new();
    for i in 0..120 {
        let dirs = if i % 40 < 20 { R } else { L };
        let live = PlayerInputs {
            directions: std::array::from_fn(|bit| dirs & (1 << bit) != 0),
            buttons: std::array::from_fn(|bit| bit == 2 && i % 37 == 0),
        };
        let frame = playback
            .step(&d.assets, live)
            .expect("the mode4 takeover keeps stepping");
        let m = playback.fight().machine();
        assert!(
            m.unrecovered().is_empty(),
            "mode4 takeover {side:?} frame{i}: engine stops {:x?}",
            m.unrecovered()
        );
        assert!(
            frame.event.is_none(),
            "mode4 takeover {side:?} frame{i}: {:?}",
            frame.event
        );
        let cpu_byte = !m.work_ram()[(0xff868f - BASE) as usize] & 15;
        let human_byte = !m.work_ram()[(0xff868e - BASE) as usize] & 15;
        if side == TakeoverPlayer::P1 {
            assert_eq!(
                cpu_byte, dirs,
                "the live P1 takeover is not overwritten by the internal mode4 CPU"
            );
        } else {
            assert_eq!(
                human_byte, dirs,
                "the live human P2 takeover keeps its own physical port"
            );
        }
        let opponent = playback.takeover_opponent();
        if i == 0 {
            assert_eq!(
                opponent,
                Some(TakeoverOpponent::Recorded),
                "one frame still follows the recording"
            );
            if side == TakeoverPlayer::P2 {
                assert_eq!(cpu_byte, recorded_cpu_byte, "the automated opponent's first frame is copied from the genuine recorded reference");
            } else {
                assert_eq!(
                    human_byte, 0,
                    "the human opponent's first frame follows its recorded neutral pad"
                );
            }
        } else {
            assert_eq!(
                opponent,
                Some(TakeoverOpponent::FairCpu),
                "the exhausted recording hands the other seat to a fresh Fair CPU"
            );
            let pad = playback
                .takeover_cpu()
                .expect("the new takeover opponent")
                .pad();
            let other_byte = if side == TakeoverPlayer::P1 {
                human_byte
            } else {
                cpu_byte
            };
            assert_eq!(
                other_byte,
                pad as u8 & 15,
                "the other physical port contains the external takeover CPU's actual pad"
            );
        }
        observations.push((
            playback.fight().state_checksum(),
            cpu_byte,
            human_byte,
            playback.takeover_cpu().unwrap().pad(),
        ));
    }
    observations
}

#[test]
fn mode_four_replay_takeovers_preserve_both_ports_and_rewind_deterministically() {
    if data().is_none() {
        return;
    }
    deep(|| {
        for revision in REVISIONS {
            for side in [TakeoverPlayer::P1, TakeoverPlayer::P2] {
                let mut playback = mode_four_playback(revision);
                let before = playback.fight().state_checksum();
                let first = replay_mode_four_branch(&mut playback, side);
                assert!(playback.rewind());
                assert_eq!(
                    playback.fight().state_checksum(),
                    before,
                    "rewind restores both port owners and their native state"
                );
                assert_eq!(
                replay_mode_four_branch(&mut playback, side),
                first,
                "{revision:?} {side:?}: takeover pads and whole-fight canonical hashes repeat after rewind"
            );
            }
        }
    });
}
