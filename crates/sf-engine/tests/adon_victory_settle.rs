//! #274: the real SC18 Adon replay must finish after Jaguar Tooth and time over.
//!
//! The USF4 Ultra 2 / Normal / seed 48 match was captured with old Player 1
//! against stronger Player 2. Adon's native Tooth stream retained its busy bit
//! after the clock expired, leaving Victory waiting forever. Replay the inputs
//! from the original seeded CPUs through the native fight, rather than copying
//! the stuck entity or replacing its animation with a fabricated state.

use std::path::Path;
use std::sync::{Arc, OnceLock};

use sf_engine::data_bank::DataBank;
use sf_engine::era::{Loadout, Revision};
use sf_engine::fair_cpu::{perceive, Body};
use sf_engine::measure::{measure_match, sparring_machine, MeasureSpec};
use sf_engine::roster::TournamentFighter;
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

fn deep(f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(f)
        .expect("the native replay thread starts")
        .join()
        .unwrap();
}

#[test]
fn jaguar_tooth_does_not_leave_adon_waiting_after_time_over() {
    deep(|| {
        let Some((rom, bank)) = data() else { return };
        let spec = MeasureSpec {
            fighters: [TournamentFighter::Adon; 2],
            styles: [(Revision::Usf4, Loadout::Ultra2); 2],
            difficulty: [2; 2],
            seed: 48,
            stronger: [false, true],
        };
        // measure_match has a fixed 36,000-frame budget and rejects any
        // untranslated native branch. Require a completed match so a fix
        // that lets Tooth recover before Victory also satisfies the test.
        let report = measure_match(rom, Some(bank.clone()), &spec)
            .expect("the native match runs without untranslated branches");
        assert!(
            report.ended,
            "Adon's replay must complete: {}",
            report.row()
        );
    });
}

const R: u8 = 1;
const L: u8 = 2;
const D: u8 = 4;
const FIGHT_LOOP: u32 = 0x3eafa;

fn seen(m: &PowerOnMachine) -> [Body; 2] {
    let view = perceive(
        m.state.frame,
        m.task_positions().contains(&(1, FIGHT_LOOP)),
        m.work_ram(),
        &m.state.objectram,
        Side::P1,
    );
    [view.me, view.foe]
}

fn step(m: &mut PowerOnMachine, actor: usize, direction: u8, kick: bool) {
    let dirs = |side| {
        let d = if side == actor { direction } else { 0 };
        [d & R != 0, d & L != 0, d & D != 0, false]
    };
    let [r, l, d, u] = dirs(0);
    m.set_p1_direction(r, l, d, u);
    m.set_p1_buttons([false, false, false, actor == 0 && kick, false, false]);
    let [r, l, d, u] = dirs(1);
    m.set_p2_direction(r, l, d, u);
    m.set_p2_buttons([false, false, false, actor == 1 && kick, false, false]);
    m.step();
    assert!(
        m.unrecovered().is_empty(),
        "the native Tooth frame has no untranslated branch"
    );
}

/// Shift the actual entity's sprite grid, retaining its native pose, facing,
/// command trackers and frame stream. Only the cross-up spacing is arranged.
fn place(m: &mut PowerOnMachine, side: usize, target: i16) {
    let current = seen(m)[side].x;
    let t = m.diagnostic_telemetry();
    let entity = if side == 0 {
        t.p1.entity_offset
    } else {
        t.p2.entity_offset
    };
    let mut snapshot = m.save_snapshot();
    let at = usize::from(entity) + 6;
    let x = u16::from_be_bytes([
        snapshot.state.objectram[at],
        snapshot.state.objectram[at + 1],
    ]);
    let shifted = x.wrapping_add(target.wrapping_sub(current) as u16);
    snapshot.state.objectram[at..at + 2].copy_from_slice(&shifted.to_be_bytes());
    m.restore_snapshot(&snapshot);
    assert_eq!(seen(m)[side].x, target);
}

#[test]
fn crossed_up_jaguar_tooth_reaches_the_rear_wall_rebounds_hits_and_lands() {
    deep(|| {
        let Some((rom, bank)) = data() else { return };
        for (revision, loadout, tooth) in [
            (Revision::Sfz3, Loadout::AIsm, 0x2c),
            (Revision::Usf4, Loadout::Ultra2, 0x20),
        ] {
            for side in 0..2 {
                let label = format!("{revision:?}/{loadout:?} P{}", side + 1);
                let spec = MeasureSpec {
                    fighters: [TournamentFighter::Adon; 2],
                    styles: [(revision, loadout); 2],
                    difficulty: [2; 2],
                    seed: 48,
                    stronger: [false; 2],
                };
                let mut m = sparring_machine(rom, Some(bank.clone()), &spec)
                    .expect("the native match starts");
                let mut manual = *m.match_config();
                manual.p2_fair_cpu = false;
                m.set_match_config(manual);
                let mut ready = false;
                for _ in 0..3_000 {
                    step(&mut m, side, 0, false);
                    let fighters = seen(&m);
                    if m.task_positions().contains(&(1, FIGHT_LOOP))
                        && fighters.iter().all(|f| f.present && f.move_id == 1)
                    {
                        ready = true;
                        break;
                    }
                }
                assert!(ready, "{label}: the real standing scene opens");
                place(&mut m, 0, 220);
                place(&mut m, 1, 320);
                step(&mut m, side, 0, false);
                let standing = seen(&m)[side];
                assert_eq!(
                    standing.facing_left,
                    side == 1,
                    "{label}: native facing is mirrored"
                );
                let (forward, back, sign) = if standing.facing_left {
                    (L, R, -1)
                } else {
                    (R, L, 1)
                };
                for (i, direction) in [forward, D | forward, D, D | back, back]
                    .into_iter()
                    .enumerate()
                {
                    step(&mut m, side, direction, i == 4);
                }
                let started = seen(&m)[side];
                assert_eq!(
                    started.move_id, tooth,
                    "{label}: the half-circle-back + LK starts Tooth"
                );
                assert_eq!(
                    started.facing_left, standing.facing_left,
                    "{label}: the real move locks its facing"
                );
                // Put the opponent behind the already-started move, just as a
                // cross-up does. Do not alter either entity's native action.
                place(&mut m, 1 - side, started.x - sign * 56);
                let foe_health = m.era_fighters()[1 - side].sub_hp;
                let mut wall = false;
                let mut rebound = false;
                let mut recovered = false;
                let mut backmost = started.x;
                for _ in 0..150 {
                    step(&mut m, side, 0, false);
                    let actor = seen(&m)[side];
                    if sign * actor.x < sign * backmost {
                        backmost = actor.x;
                    }
                    let t = m.diagnostic_telemetry();
                    let entity = if side == 0 {
                        t.p1.entity_offset
                    } else {
                        t.p2.entity_offset
                    };
                    let rear = i16::from(m.state.objectram[usize::from(entity) + 0x19]);
                    let at_wall = actor.move_id == tooth
                        && if sign == 1 {
                            actor.x - rear <= 0x60
                        } else {
                            actor.x + rear >= 0x1a0
                        };
                    if at_wall && !wall {
                        // After proving the retreat crosses the opponent and
                        // reaches the rear wall, offer a standing target farther
                        // along the returning kick. Both orientations now offer
                        // the same hundred-pixel return spacing, independently of
                        // the original cross-up's distance from the rear wall.
                        place(&mut m, 1 - side, actor.x + sign * 100);
                    }
                    wall |= at_wall;
                    rebound |= wall && sign * (actor.x - backmost) > 30;
                    if actor.move_id != tooth {
                        recovered = actor.state & 0x80 == 0 && actor.y == standing.y;
                        break;
                    }
                }
                assert!(
                    wall && rebound && recovered,
                    "{label}: wall={wall}, rebound={rebound}, recovered={recovered}, actor={:?}",
                    seen(&m)[side]
                );
                assert!(
                    m.era_fighters()[1 - side].sub_hp < foe_health,
                    "{label}: the returning kick still hits the crossed-up opponent"
                );
            }
        }
    });
}
