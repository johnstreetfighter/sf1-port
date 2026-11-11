//! Ticket #239 (`TR1`; spec #238, design §5.1, ADR-0013): the health floor.
//!
//! In Training, under every preset, Faithful 1987 included, no fighter is
//! knocked out: every hit that drains a player's health stops at one point
//! left, and a hit that would have finished the fighter lands as an
//! ordinary hit with the alive reaction. Headless walks through the game
//! session into Training against the dummy and against the CPU, hitting
//! for more than full health and reading the fight through the session's
//! read-only views. A clean fireball still slows the game under Faithful
//! 1987, as in 1987; under the overhaul presets, whose fireball slows only
//! for the knock-out, the floor leaves no slow motion. Training also
//! refills health a second after the last hit (ticket #243), so the walks
//! keep their hits inside that second; the CPU, which steps out of reach,
//! takes more than its full health across its refills. Local ROMs only;
//! each test skips without them.

#[path = "support/select.rs"]
mod select;

use std::path::Path;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::fight_reader::FighterRead;
use sf_engine::fighter::{P1_STRUCT, P2_STRUCT};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::show::ready;
use sf_engine::front::{FrameInputs, GameSession, Transition};
use sf_engine::moves::{facing_right, Strength, REGISTRY};
use sf_engine::roster::{PlayMode, TournamentFighter};
use sf_engine::run::PowerOnMachine;
use sf_video::callouts::Side;
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

const FIGHT_LOOP: u32 = 0x3eafa;
/// The KO slow motion flag (0xffc00b), set by scene event 9.
const SLOW_MOTION: u32 = 0xffc00b;
/// The scene mode (0xffc008); bit 0x20 is a fireball's slow-motion request.
const SCENE_MODE: u32 = 0xffc008;
/// Full health: 11 cells of 4 and a remainder of 4 (`round_hud`).
const FULL: u16 = 48;
/// The health floor: one point left.
const FLOOR: u16 = 1;

/// The four presets a player can pick for a fight.
const PRESETS: [RulesetPreset; 4] = [
    RulesetPreset::DefinitiveOverhaul,
    RulesetPreset::ClassicCompetitive,
    RulesetPreset::PureFootsies1987Plus,
    RulesetPreset::Faithful1987,
];

fn assets() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(load_assets(ROMS, MANIFEST).expect("local ROM set assembles"))
}

fn idle() -> FrameInputs {
    FrameInputs::default()
}

fn with(f: impl Fn(&mut FrameInputs)) -> FrameInputs {
    let mut i = FrameInputs::default();
    f(&mut i);
    i
}

fn tap(s: &mut GameSession, input: FrameInputs) {
    s.step(input);
    s.step(idle());
}

const DOWN: usize = 2;

fn down() -> FrameInputs {
    with(|i| i.p1.directions[DOWN] = true)
}

fn confirm() -> FrameInputs {
    with(|i| i.menu.confirm = true)
}

fn wb(m: &PowerOnMachine, a: u32) -> u8 {
    m.work_ram()[(a - 0xff8000) as usize]
}

/// Power-on under `preset` (the preset the settings carry), the Main Menu,
/// Single Player, Training, CPU or DUMMY; P1 Ryu against the opponent under
/// the cursor; the first fight loop.
fn training(a: Assets, preset: RulesetPreset, dummy: bool) -> GameSession {
    let mut s = GameSession::new(a, MatchConfig::from_preset(preset));
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.menu.start = true));
    tap(&mut s, confirm()); // Single Player
    tap(&mut s, down());
    tap(&mut s, confirm()); // Training
    if dummy {
        tap(&mut s, down());
    }
    tap(&mut s, confirm()); // CPU or DUMMY
    select::confirm_fighter(&mut s, confirm(), tap); // P1 Ryu
    let out = select::confirm_fighter(&mut s, confirm(), GameSession::step);
    assert!(matches!(out.transition, Some(Transition::Fighters(TournamentFighter::Ryu, _))), "{:?}", out.transition);
    if dummy {
        while s.screen() != "country select" {
            s.step(idle());
        }
        tap(&mut s, with(|i| i.p1.buttons[0] = true));
    }
    let mut f = 0;
    while s.fight_machine().map(flow_position) != Some(FIGHT_LOOP) {
        s.step(idle());
        f += 1;
        assert!(f < 3_000, "the fight loop (at {})", s.screen());
    }
    let cfg = s.fight_machine().unwrap().match_config();
    assert_eq!((cfg.roster.mode, cfg.preset), (PlayMode::Training, preset));
    s
}

/// P1 presses HP every 24 frames and walks in between: once in range, a
/// hit every 48 frames, inside the second after which Training refills
/// health (ticket #243), so the hits take the dummy's full health and go
/// on at the floor.
fn attack(f: u32) -> FrameInputs {
    with(|i| {
        i.p1.directions[0] = f % 24 >= 6;
        i.p1.buttons[2] = f % 24 < 3;
    })
}

/// A hit reaction began on this frame: a new one, or one started over on
/// the last frame of the one before (busy again, +9 bit 7).
fn reaction_began(was: &FighterRead, now: &FighterRead) -> bool {
    now.reacting() && (!was.reacting() || (was.state & 0x80 == 0 && now.state & 0x80 != 0))
}

/// What a walk saw of one side.
#[derive(Debug, Default, Clone, Copy)]
struct Seen {
    lowest: u16,
    /// Hit reactions that began while the side was already at the floor:
    /// each one, without the floor, would have knocked it out.
    floor_hits: u32,
    knocked_out: bool,
    /// All the health the side lost, across Training's refills.
    taken: u32,
}

/// Watches every stepped frame of a Training fight: the flow never leaves
/// the fight loop, the match never ends, and no KO's slow motion runs
/// unless `slow_ok`.
#[derive(Clone)]
struct Watch {
    seen: [Seen; 2],
    last: Option<[FighterRead; 2]>,
    slowed: bool,
}

impl Watch {
    fn new() -> Watch {
        Watch { seen: [Seen { lowest: FULL, ..Seen::default() }; 2], last: None, slowed: false }
    }

    fn step(&mut self, s: &mut GameSession, input: FrameInputs) {
        s.step(input);
        assert_eq!(s.screen(), "fight", "the fight goes on");
        let m = s.fight_machine().unwrap();
        assert!(!m.match_over(), "no match end");
        assert_eq!(flow_position(m), FIGHT_LOOP, "the flow stays in the fight loop: no KO path, no round end");
        self.slowed |= wb(m, SLOW_MOTION) != 0 || wb(m, SCENE_MODE) & 0x20 != 0;
        let read = *s.fight().unwrap().fight_read();
        for side in Side::BOTH {
            let now = *read.fighter(side);
            let seen = &mut self.seen[side.index()];
            seen.lowest = seen.lowest.min(now.health);
            seen.knocked_out |= now.knocked_out();
            if let Some(was) = self.last.map(|l| l[side.index()]) {
                if reaction_began(&was, &now) && was.health == FLOOR {
                    seen.floor_hits += 1;
                }
                seen.taken += u32::from(was.health.saturating_sub(now.health));
            }
        }
        self.last = Some(read.fighters);
    }

    /// `side` took more than its full health in hits and was never knocked
    /// out: it reached the floor, never went under it, and was hit again
    /// there.
    fn assert_floored(&self, side: Side, what: &str) {
        let seen = self.seen[side.index()];
        assert!(!seen.knocked_out, "{what}: never knocked out ({seen:?})");
        assert_eq!(seen.lowest, FLOOR, "{what}: down to the floor and never below it ({seen:?})");
        assert!(seen.floor_hits >= 2, "{what}: hit again at the floor, past its full health ({seen:?})");
    }
}

#[test]
fn the_dummy_hit_for_more_than_its_full_health_is_never_knocked_out_under_every_preset() {
    for preset in PRESETS {
        let Some(a) = assets() else { return };
        let mut s = training(a, preset, true);
        let mut w = Watch::new();
        let mut f = 0u32;
        while w.seen[1].floor_hits < 3 {
            w.step(&mut s, attack(f));
            f += 1;
            assert!(f < 1_500, "{preset:?}: the dummy hit past its full health ({:?})", w.seen[1]);
        }
        // Ten seconds longer, still attacking: nothing ends.
        for _ in 0..600 {
            w.step(&mut s, attack(f));
            f += 1;
        }
        w.assert_floored(Side::P2, &format!("{preset:?} dummy"));
        assert_eq!(w.seen[0].lowest, FULL, "{preset:?}: the dummy never hit back");
        assert!(!w.slowed, "{preset:?}: no KO slow motion");
        let m = s.fight_machine().unwrap();
        assert!(m.unrecovered().is_empty(), "{preset:?}: {:?}", m.unrecovered());
        assert_eq!(4 * u16::from(wb(m, P2_STRUCT + 7)) + u16::from(wb(m, P2_STRUCT + 6)), FLOOR);
    }
}

#[test]
fn against_the_cpu_neither_fighter_is_knocked_out_under_every_preset() {
    for preset in PRESETS {
        let Some(a) = assets() else { return };
        let mut s = training(a, preset, false);
        // Standing still, P1 takes the CPU's hits past its full health.
        // Capcom's CPU (Faithful 1987) strings them closely enough to beat
        // the refill and hit P1 at the floor; the fair CPU (SP21, the
        // overhaul presets) waits on what it sees between attacks, so its
        // hits go past P1's full health across refills instead (the floor
        // itself is the dummy test's, under every preset).
        let faithful = preset == RulesetPreset::Faithful1987;
        let mut w = Watch::new();
        let mut f = 0u32;
        while if faithful { w.seen[0].floor_hits < 2 } else { w.seen[0].taken <= 4 * u32::from(FULL) } {
            w.step(&mut s, idle());
            f += 1;
            assert!(f < 4_000, "{preset:?}: P1 hit past its full health ({:?})", w.seen[0]);
        }
        if faithful {
            w.assert_floored(Side::P1, &format!("{preset:?} P1"));
        }
        let p1 = w.seen[0];
        assert!(!p1.knocked_out && p1.lowest >= FLOOR, "{preset:?}: never knocked out, never under the floor ({p1:?})");
        assert!(!w.slowed, "{preset:?}: no KO slow motion");
        let m = s.fight_machine().unwrap();
        assert!(m.unrecovered().is_empty(), "{preset:?}: {:?}", m.unrecovered());
        if faithful {
            assert_eq!(4 * u16::from(wb(m, P1_STRUCT + 7)) + u16::from(wb(m, P1_STRUCT + 6)), FLOOR);
        }
    }
}

/// P1 closes in on the CPU and, within reach, presses HP and HK by turns
/// every ten frames: the hardest pressure on the CPU these walks found.
fn press_the_cpu(s: &GameSession, f: u32) -> FrameInputs {
    let m = s.fight_machine().unwrap();
    let o = &m.state.objectram;
    let x = |e: usize| u16::from_be_bytes([o[e + 6], o[e + 7]]);
    let toward = if facing_right(m.work_ram(), o, P1_STRUCT) { 0 } else { 1 };
    with(|i| {
        if x(0x400).abs_diff(x(0xa00)) > 70 {
            i.p1.directions[toward] = true;
        } else if f % 10 < 2 {
            i.p1.buttons[if (f / 10) % 2 == 0 { 2 } else { 5 }] = true;
        }
    })
}

#[test]
fn the_cpu_hit_for_more_than_its_full_health_is_never_knocked_out_under_every_preset() {
    for preset in PRESETS {
        let Some(a) = assets() else { return };
        let mut s = training(a, preset, false);
        let mut w = Watch::new();
        for f in 0..4_500 {
            let input = press_the_cpu(&s, f);
            w.step(&mut s, input);
        }
        // The CPU steps out of reach and refills a second later (ticket
        // #243), so no string takes all its health at once; across its
        // refills it takes more than four times its full health from
        // Capcom's CPU (Faithful 1987). Under the overhaul presets the CPU
        // is the fair CPU (SP21), which guards an attack still coming rather
        // than answer it and steps away as it sees fit, so it takes less
        // (94 and 120 of its 48 under Classic Competitive and Definitive
        // Overhaul at SP21, where true combos, SP17, also let a move hit
        // once): past its full health, and the floor itself is the dummy
        // test's there.
        let cpu = w.seen[1];
        let times = if preset == RulesetPreset::Faithful1987 { 4 } else { 1 };
        assert!(cpu.taken > times * u32::from(FULL), "{preset:?}: the CPU hit past its full health ({cpu:?})");
        assert!(!cpu.knocked_out && !w.seen[0].knocked_out, "{preset:?}: never knocked out ({:?})", w.seen);
        assert!(cpu.lowest >= FLOOR && w.seen[0].lowest >= FLOOR, "{preset:?}: never under the floor ({:?})", w.seen);
        assert!(!w.slowed, "{preset:?}: no KO slow motion");
        let m = s.fight_machine().unwrap();
        assert!(m.unrecovered().is_empty(), "{preset:?}: {:?}", m.unrecovered());
    }
}

/// Once P1 stands free, wait `delay` frames, feed P1's heavy Hadoken, and
/// step until `side`'s next hit reaction begins; returns `side` the frame
/// before and the reaction's first frame, or `None` if nothing hit `side`
/// within 150 frames.
fn throw_hadoken(s: &mut GameSession, w: &mut Watch, side: Side, delay: u32) -> Option<(FighterRead, FighterRead)> {
    let mut f = 0u32;
    while !ready(s.fight().unwrap().fight_read().fighter(Side::P1)) {
        w.step(s, idle());
        f += 1;
        assert!(f < 600, "P1 free to throw");
    }
    for _ in 0..delay {
        w.step(s, idle());
    }
    let m = s.fight_machine().unwrap();
    let right = facing_right(m.work_ram(), &m.state.objectram, P1_STRUCT);
    let hadoken = REGISTRY.iter().find(|e| e.name == "HADOKEN").unwrap();
    let mut feed = hadoken.input.frames(right, Strength::Heavy).into_iter();
    for _ in 0..150 {
        let was = *s.fight().unwrap().fight_read().fighter(side);
        let p1 = feed.next().unwrap_or_default();
        w.step(s, with(|i| i.p1 = p1));
        let now = *s.fight().unwrap().fight_read().fighter(side);
        if reaction_began(&was, &now) {
            return Some((was, now));
        }
    }
    None
}

/// Feed P1's heavy Hadoken once P1 and `side` both stand free, then step
/// until `side`'s next hit reaction begins; returns the reaction's first
/// frame.
fn hadoken_until_hit(s: &mut GameSession, w: &mut Watch, side: Side, what: &str) -> FighterRead {
    let mut f = 0u32;
    while s.fight().unwrap().fight_read().fighter(side).reacting() {
        w.step(s, idle());
        f += 1;
        assert!(f < 600, "{what}: {side:?} free");
    }
    throw_hadoken(s, w, side, 0).unwrap_or_else(|| panic!("{what}: the Hadoken hits")).1
}

/// The dummy just brought to the floor: a Hadoken that lands while it is
/// still there. Thrown too soon, the fireball passes the dummy while its
/// reaction makes it invulnerable; too late, Training has refilled it
/// (ticket #243). Tries each wait after P1 stands free, from a copy of the
/// session, and goes on from the first that lands at the floor.
fn hadoken_at_the_floor(s: &GameSession, w: &Watch, what: &str) -> (GameSession, Watch, FighterRead) {
    (0..60)
        .find_map(|delay| {
            let (mut s, mut w) = (s.clone(), w.clone());
            let (was, hit) = throw_hadoken(&mut s, &mut w, Side::P2, delay)?;
            (was.health == FLOOR).then_some((s, w, hit))
        })
        .unwrap_or_else(|| panic!("{what}: a Hadoken that lands at the floor"))
}

#[test]
fn a_fireball_at_the_floor_lands_alive_and_slows_only_as_1987_does() {
    for preset in PRESETS {
        let Some(a) = assets() else { return };
        let mut s = training(a, preset, true);
        let what = format!("{preset:?}");
        let mut w = Watch::new();
        let mut f = 0u32;
        while w.seen[1].lowest > FLOOR {
            w.step(&mut s, attack(f));
            f += 1;
            assert!(f < 1_500, "{what}: the dummy down to the floor");
        }
        // The normals that took it there never slow the game.
        assert!(!w.slowed, "{what}: no slow motion from normals");
        let (mut s, mut w, hit) = hadoken_at_the_floor(&s, &w, &what);
        assert!(hit.hitstun() && !hit.knocked_out(), "{what}: the lethal fireball takes the alive reaction ({hit:?})");
        for _ in 0..120 {
            w.step(&mut s, idle());
        }
        let seen = w.seen[1];
        assert!(!seen.knocked_out && seen.lowest == FLOOR && seen.floor_hits >= 1, "{what}: the fireball hit at the floor ({seen:?})");
        // Faithful 1987 slows every clean fireball hit; the overhauls only
        // the knock-out, which the floor never gives.
        assert_eq!(w.slowed, preset == RulesetPreset::Faithful1987, "{what}: slow motion");
    }
}

#[test]
fn under_faithful_1987_training_a_clean_non_lethal_fireball_still_slows_the_game() {
    let Some(a) = assets() else { return };
    let mut s = training(a, RulesetPreset::Faithful1987, true);
    let mut w = Watch::new();
    let hit = hadoken_until_hit(&mut s, &mut w, Side::P2, "Faithful 1987");
    assert!(hit.hitstun());
    let health = s.fight().unwrap().fight_read().fighter(Side::P2).health;
    assert!(health > FLOOR && health < FULL, "a non-lethal hit ({health})");
    let mut slowed = false;
    for _ in 0..30 {
        w.step(&mut s, idle());
        slowed |= wb(s.fight_machine().unwrap(), SLOW_MOTION) == 0xff;
    }
    assert!(slowed, "the 1987 slow motion (0xffc00b)");
}
