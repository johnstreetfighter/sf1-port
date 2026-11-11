//! A fighter bound to a side by the roster (full cast) draws his own
//! streams there, not the ROM's fighter for that side (Ryu on P1, Ken on
//! P2):
//! - the round's walk-in is move 0 of his own move table, from the first
//!   round on, in Local and Arcade;
//! - Ken struck on P1 (and Ryu on P2) reacts and falls with his own hit
//!   and knock-out streams (the 1987 tables `0x4473a` / `0x4479c` and
//!   `0x44662` / `0x446ca` are each one fighter's, not one side's);
//! - a boss struck on either side reacts with his own, the tables the 1987
//!   game gives him as the CPU, never a shoto's.
//!
//! Local ROMs only; each test skips without them.

use std::ops::Range;
use std::path::Path;

use sf_engine::config::MatchConfig;
use sf_engine::fighter::{P1_STRUCT, P2_STRUCT};
use sf_engine::front::cabinet::{drive_to_country_select, flow_position, pick_country, Players, STAGE_START};
use sf_engine::roster::{PlayMode, TournamentFighter as T};
use sf_engine::run::PowerOnMachine;
use sf_video::art::{load_art_from, ArtSources};
use sf_video::art_pack::ArtPack;
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const PACK: &str = "../../tools/reference/local/roms/sf1-port-art";
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

fn data_bank(a: &Assets) -> std::sync::Arc<sf_engine::data_bank::DataBank> {
    static BANK: std::sync::OnceLock<std::sync::Arc<sf_engine::data_bank::DataBank>> =
        std::sync::OnceLock::new();
    BANK.get_or_init(|| {
        let pack = ArtPack::open(Path::new(PACK)).expect("local art pack");
        let art = load_art_from(
            a,
            &ArtSources {
                recipes: &RECIPES,
                reviews: Some(&REVIEWS),
                pack: Ok(&pack),
            },
        )
        .expect("art loads");
        std::sync::Arc::new(sf_engine::data_bank::DataBank::build(&a.maincpu, &art.catalogues))
    })
    .clone()
}

/// The intro loop (`0x3ea30`) and the fight loop (`0x3eafa`).
const INTRO: u32 = 0x3ea30;
const FIGHT_LOOP: u32 = 0x3eafa;
/// Every frame of Ryu's 13 hit and 11 knock-out streams (`0x01b82..`), and
/// Ken's (`0x03c60..`): `docs/reference/art-inventory.md`.
const RYU_REACTIONS: Range<u32> = 0x01b82..0x024de;
const KEN_REACTIONS: Range<u32> = 0x03c60..0x045bc;

fn assets() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(load_assets(ROMS, MANIFEST).expect("local ROM set assembles"))
}

fn rl(m: &PowerOnMachine, addr: u32) -> u32 {
    let a = (addr - 0xff8000) as usize;
    u32::from_be_bytes(m.work_ram()[a..a + 4].try_into().unwrap())
}

/// The stream a side's fighter is running (its entity's `+0x2c`).
fn stream(m: &PowerOnMachine, ps: u32) -> u32 {
    let e = (rl(m, ps + 0xe) & 0xffff) as usize;
    u32::from_be_bytes(m.state.objectram[e + 0x2c..e + 0x30].try_into().unwrap())
}

/// Move 0's stream in `fighter`'s move table (its 0x30-byte header's
/// `+0x2c`).
fn walk_in(rom: &[u8], fighter: T) -> u32 {
    let at = fighter.move_table() as usize + 0x2c;
    u32::from_be_bytes(rom[at..at + 4].try_into().unwrap())
}

/// A machine at the stage start of `p1` v `p2` from Japan.
fn at_stage_start(a: &Assets, mode: PlayMode, p1: T, p2: T) -> PowerOnMachine {
    let mut cfg = MatchConfig::default();
    cfg.roster.mode = mode;
    cfg.roster.p1 = p1;
    cfg.roster.p2 = p2;
    assert!(cfg.roster.full_cast, "the default preset binds the full cast");
    let mut m = PowerOnMachine::new(a.maincpu.clone())
        .with_match_config(cfg)
        .with_data_bank(data_bank(a));
    let players = if mode == PlayMode::Versus { Players::Two } else { Players::One };
    drive_to_country_select(&mut m, players, 4_000).expect("country select");
    pick_country(&mut m, &a.maincpu, 0, 600).expect("stage start");
    assert_eq!(flow_position(&m), STAGE_START);
    m
}

/// One round's walk-in: each side's first intro stream, and where each
/// side stands (x, facing) when the fight loop begins. P1's side, then its
/// opponent's (P2, or Arcade's CPU).
struct WalkIn {
    streams: [u32; 2],
    ends: [(u16, u8); 2],
}

fn walk_in_run(a: &Assets, mode: PlayMode, p1: T, p2: T) -> WalkIn {
    let mut m = at_stage_start(a, mode, p1, p2);
    let sides = |m: &PowerOnMachine| {
        let d1 = rl(m, P1_STRUCT + 0xe);
        [(d1 & 0xffff) as usize, (d1 >> 16) as usize]
    };
    let mut f = 0;
    while flow_position(&m) != INTRO {
        m.step();
        f += 1;
        assert!(f < 1_000, "{mode:?} {p1:?} v {p2:?}: no intro");
    }
    let o = &m.state.objectram;
    let streams = sides(&m).map(|e| u32::from_be_bytes(o[e + 0x2c..e + 0x30].try_into().unwrap()));
    while flow_position(&m) != FIGHT_LOOP {
        m.step();
        f += 1;
        assert!(f < 1_500, "{mode:?} {p1:?} v {p2:?}: no fight loop");
    }
    assert!(m.unrecovered().is_empty(), "{mode:?} {p1:?} v {p2:?}: {:05x?}", m.unrecovered());
    let o = &m.state.objectram;
    let ends = sides(&m).map(|e| (u16::from_be_bytes([o[e + 6], o[e + 7]]), o[e + 0x37]));
    WalkIn { streams, ends }
}

#[test]
fn the_round_one_walk_in_is_the_bound_fighters_own_move_0_on_his_own_side() {
    let Some(a) = assets() else { return };
    let rom = &a.maincpu;
    assert_ne!(walk_in(rom, T::Ryu), walk_in(rom, T::Ken));
    // The 1987 sides: Ryu walks in from the left, Ken from the right.
    let faithful = walk_in_run(&a, PlayMode::Versus, T::Ryu, T::Ken);
    assert_eq!(faithful.streams, [walk_in(rom, T::Ryu), walk_in(rom, T::Ken)]);
    // Swapped, each walks in as himself along the other's path, ending
    // where the 1987 fighter of that side ends.
    let swapped = walk_in_run(&a, PlayMode::Versus, T::Ken, T::Ryu);
    assert_eq!(swapped.streams, [walk_in(rom, T::Ken), walk_in(rom, T::Ryu)], "Ken v Ryu walk-ins");
    assert_eq!(swapped.ends, faithful.ends, "Ken v Ryu end where Ryu v Ken do");
    let arcade = walk_in_run(&a, PlayMode::Arcade, T::Ken, T::Ryu);
    assert_eq!(arcade.streams[0], walk_in(rom, T::Ken), "Arcade: Ken walks in as himself");
    assert_eq!(arcade.ends[0], faithful.ends[0], "Arcade: Ken ends where Ryu does");
    // Every fighter on either side: his own move 0, P1 on the left facing
    // right, the opponent on the right facing left.
    for (i, &p1) in T::ALL.iter().enumerate() {
        let p2 = T::ALL[(i + 1) % T::ALL.len()];
        let run = walk_in_run(&a, PlayMode::Versus, p1, p2);
        assert_eq!(run.streams, [walk_in(rom, p1), walk_in(rom, p2)], "{p1:?} v {p2:?} walk-ins");
        let [(x1, face1), (x2, face2)] = run.ends;
        assert!(x1 < x2 && face1 == 2 && face2 == 4, "{p1:?} v {p2:?} end at {:x?}", run.ends);
    }
}

#[test]
fn ken_on_p1_and_ryu_on_p2_are_struck_in_their_own_hit_streams() {
    let Some(a) = assets() else { return };
    let mut m = at_stage_start(&a, PlayMode::Versus, T::Ken, T::Ryu);
    let mut hits = [0u32; 2];
    // Both walk in and press a button in turn until a round is lost.
    for f in 0..12_000u32 {
        let flow = flow_position(&m);
        if f > 2_000 && flow != FIGHT_LOOP && hits.iter().all(|&h| h > 0) {
            break;
        }
        press(&mut m, f);
        m.step();
        assert!(m.unrecovered().is_empty(), "frame {}: {:05x?}", m.state.frame, m.unrecovered());
        let (ken, ryu) = (stream(&m, P1_STRUCT), stream(&m, P2_STRUCT));
        assert!(!RYU_REACTIONS.contains(&ken), "frame {}: Ken on P1 runs Ryu's stream {ken:05x}", m.state.frame);
        assert!(!KEN_REACTIONS.contains(&ryu), "frame {}: Ryu on P2 runs Ken's stream {ryu:05x}", m.state.frame);
        hits[0] += KEN_REACTIONS.contains(&ken) as u32;
        hits[1] += RYU_REACTIONS.contains(&ryu) as u32;
    }
    assert!(hits[0] > 0 && hits[1] > 0, "both were struck: {hits:?} frames in their reactions");
}

/// One frame of the scripted round: P1 presses in, P2 mostly stands his
/// ground, and each presses a button in turn.
fn press(m: &mut PowerOnMachine, f: u32) {
    let (walk1, walk2) = (f % 40 < 35, f % 40 < 10);
    let mut b1 = [false; 6];
    let mut b2 = [false; 6];
    if f % 20 < 3 {
        b1[((f / 20) % 6) as usize] = true;
    }
    if (f + 10) % 20 < 3 {
        b2[((f / 20 + 3) % 6) as usize] = true;
    }
    m.set_p1_direction(walk1, false, false, false);
    m.set_p1_buttons(b1);
    m.set_p2_direction(false, walk2, false, false);
    m.set_p2_buttons(b2);
}

/// A boss's hit and knock-out stream starts, read from the 1987 game's
/// per-country opponent arrays (`0x3e38a`): array 0 the move tables, 13 and
/// 14 the hit and knock-out tables the CPU reads through `0xffc050` /
/// `0xffc054` (13 and 11 four-byte entries, the stream word first).
fn boss_reactions(rom: &[u8], boss: T) -> Vec<u32> {
    let rl = |x: u32| u32::from_be_bytes(rom[x as usize..x as usize + 4].try_into().unwrap());
    let rw = |x: u32| u16::from_be_bytes(rom[x as usize..x as usize + 2].try_into().unwrap()) as u32;
    let handler = rl(0x3e38a);
    let array = |n: u32| rl(handler + 8 * n + 2);
    let i = (0..12).find(|&i| rl(array(0) + 4 * i) == boss.move_table()).expect("on Japan's ladder");
    let (alive, ko) = (rl(array(13) + 4 * i), rl(array(14) + 4 * i));
    assert_eq!(boss.reaction_tables(), Some((alive, ko)), "{boss:?}'s tables in the roster");
    (0..13).map(|k| rw(alive + 4 * k)).chain((0..11).map(|k| rw(ko + 4 * k))).collect()
}

#[test]
fn a_boss_on_either_side_is_struck_in_his_own_hit_streams() {
    let Some(a) = assets() else { return };
    let shoto = |s: u32| RYU_REACTIONS.contains(&s) || KEN_REACTIONS.contains(&s);
    for boss in T::ALL.into_iter().filter(|f| !f.is_shoto()) {
        let own = boss_reactions(&a.maincpu, boss);
        for (p1, p2, side) in [(boss, T::Ryu, P1_STRUCT), (T::Ken, boss, P2_STRUCT)] {
            let mut m = at_stage_start(&a, PlayMode::Versus, p1, p2);
            let mut struck = false;
            for f in 0..12_000u32 {
                press(&mut m, f);
                if !struck {
                    if side == P1_STRUCT {
                        m.set_p1_buttons([false; 6]);
                    } else {
                        m.set_p2_buttons([false; 6]);
                    }
                }
                m.step();
                assert!(m.unrecovered().is_empty(), "{p1:?} v {p2:?} frame {}: {:05x?}", m.state.frame, m.unrecovered());
                let s = stream(&m, side);
                assert!(!shoto(s), "{p1:?} v {p2:?} frame {}: {boss:?} runs a shoto's stream {s:05x}", m.state.frame);
                struck |= own.contains(&s) || own.contains(&s.wrapping_sub(0x20));
                if struck && flow_position(&m) != FIGHT_LOOP {
                    break;
                }
            }
            assert!(struck, "{p1:?} v {p2:?}: {boss:?} was never struck in his own streams");
        }
    }
}
