//! Ticket #244 (`TR4`; spec #238, design §5.1, ADR-0013): hits count at the
//! health floor.
//!
//! The fight reader sees a landed hit when a side's health goes down. In
//! Training the health floor holds a fighter at one point, so a hit there
//! takes nothing and a hit that reaches it is cut short; the engine's
//! Training gate records each hit's full damage, and the reader reads that
//! record in Training. So the reads, and the combo count, HOLD COMBO's
//! damage and FIRST ATTACK fed from them, match a fight with unlimited
//! health.
//!
//! The fight with unlimited health is a shadow machine: a copy of the
//! session's, stepped with the same inputs, whose fighters this test puts
//! back to full health before every frame (the test's own copy; the
//! session's machine is never written). Read by the plain health rule, its
//! hits are what the session's reader must read. Headless walks go through
//! the game session into Training against the dummy; local ROMs only, and
//! each test skips without them.

use std::path::Path;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::fight_reader::{FightRead, FightReader, LandedHit};
use sf_engine::fighter::{CPU_STRUCT, P1_STRUCT, P2_STRUCT};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::settings::Settings;
use sf_engine::front::{FrameInputs, GameSession, MenuItem, Transition};
use sf_engine::roster::{PlayMode, TournamentFighter};
use sf_engine::run::{PowerOnMachine, PowerOnSnapshot};
use sf_engine::training::TrainingState;
use sf_video::announcer::AnnouncerLine;
use sf_video::callouts::{ComboCount, Side};
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

const FIGHT_LOOP: u32 = 0x3eafa;
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

/// Step until the fight's flow reaches the fight loop.
fn to_fight_loop(s: &mut GameSession) {
    let mut f = 0;
    while s.fight_machine().map(flow_position) != Some(FIGHT_LOOP) {
        s.step(idle());
        f += 1;
        assert!(f < 3_000, "the fight loop (at {})", s.screen());
    }
}

/// Power-on under `preset` with `settings`, the Main Menu, Single Player,
/// Training, DUMMY; P1 Ryu against the dummy; the first fight loop.
fn training_dummy(a: Assets, preset: RulesetPreset, settings: Settings) -> GameSession {
    let mut s = GameSession::new(a, MatchConfig::from_preset(preset)).with_settings(settings);
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.menu.start = true));
    tap(&mut s, confirm()); // Single Player
    tap(&mut s, down());
    tap(&mut s, confirm()); // Training
    tap(&mut s, down());
    tap(&mut s, confirm()); // DUMMY
    tap(&mut s, confirm()); // P1 Ryu
    let out = s.step(confirm());
    assert!(matches!(out.transition, Some(Transition::Fighters(TournamentFighter::Ryu, _))), "{:?}", out.transition);
    while s.screen() != "country select" {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    to_fight_loop(&mut s);
    let cfg = s.fight_machine().unwrap().match_config();
    assert_eq!((cfg.roster.mode, cfg.preset), (PlayMode::Training, preset));
    s
}

/// Power-on under `preset`, the Main Menu, Multi Player, Local; P1 Ryu, P2
/// Ken; Japan; the first fight loop.
fn local(a: Assets, preset: RulesetPreset) -> GameSession {
    let mut s = GameSession::new(a, MatchConfig::from_preset(preset));
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.menu.start = true));
    tap(&mut s, down());
    tap(&mut s, confirm());
    tap(&mut s, confirm());
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    tap(&mut s, with(|i| i.p2.buttons[0] = true));
    while s.screen() != "country select" {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    to_fight_loop(&mut s);
    assert_eq!(s.fight_machine().unwrap().match_config().roster.mode, PlayMode::Versus);
    s
}

/// P1 presses HP every 24 frames and walks in between: once in range, a
/// hit every 48 frames, inside the second after which Training refills
/// health (ticket #243), so the hits take the dummy down to the floor and
/// go on there (the health-floor tests' attack).
fn attack(f: u32) -> FrameInputs {
    with(|i| {
        i.p1.directions[0] = f % 24 >= 6;
        i.p1.buttons[2] = f % 24 < 3;
    })
}

/// P1 walks in to close range and there presses HP every 22 frames: hits
/// that often land on the last frame of the dummy's reaction and start it
/// over, which at the floor is all the 1987 fight shows of them (the state's
/// busy bit set again; ticket #243 found it).
fn close_hp(f: u32, s: &GameSession) -> FrameInputs {
    let o = &s.fight_machine().unwrap().state.objectram;
    let x = |e: usize| u16::from_be_bytes([o[e + 6], o[e + 7]]);
    let close = x(0x400).abs_diff(x(0xa00)) <= 60;
    with(|i| {
        i.p1.directions[0] = !close;
        i.p1.buttons[2] = close && f % 22 == 0;
    })
}

/// A side's player structure (the CPU's stands in for P2 in game mode 2).
fn player_struct(m: &PowerOnMachine, side: Side) -> u32 {
    match (side, wb(m, 0xffc002)) {
        (Side::P1, _) => P1_STRUCT,
        (Side::P2, 2) => CPU_STRUCT,
        (Side::P2, _) => P2_STRUCT,
    }
}

fn health(m: &PowerOnMachine, side: Side) -> u16 {
    let ps = player_struct(m, side);
    4 * u16::from(wb(m, ps + 7)) + u16::from(wb(m, ps + 6))
}

/// The fight with unlimited health (see the module docs): the session's
/// machine copied, each fighter back at full health before every frame,
/// read by the plain health rule.
struct Shadow {
    m: PowerOnMachine,
    reader: FightReader,
}

impl Shadow {
    fn new(s: &GameSession) -> Shadow {
        let m = s.fight_machine().unwrap().clone();
        let mut reader = FightReader::new();
        reader.read(m.work_ram(), &m.state.objectram, None);
        Shadow { m, reader }
    }

    /// One frame with P1's `input` and the dummy's idle pad, as the fight
    /// gives them; returns the shadow's read.
    fn step(&mut self, input: FrameInputs) -> FightRead {
        let low: Vec<Side> = Side::BOTH.into_iter().filter(|&side| health(&self.m, side) < FULL).collect();
        if !low.is_empty() {
            let mut snap: PowerOnSnapshot = self.m.save_snapshot();
            for side in low {
                let ps = (player_struct(&self.m, side) - 0xff8000) as usize;
                snap.work[ps + 6] = 4;
                snap.work[ps + 7] = 0xb;
            }
            self.m.restore_snapshot(&snap);
            // The reader's last frame is the topped-up one.
            self.reader.read(self.m.work_ram(), &self.m.state.objectram, None);
        }
        let m = &mut self.m;
        let [r, l, d, u] = input.p1.directions;
        m.set_p1_direction(r, l, d, u);
        m.set_p1_buttons(input.p1.buttons);
        m.set_p2_direction(false, false, false, false);
        m.set_p2_buttons([false; 6]);
        m.set_cabinet_inputs(false, false, false, false, false);
        m.step();
        self.reader.read(m.work_ram(), &m.state.objectram, None)
    }
}

/// A hit P1 landed on the dummy, as the session read it.
#[derive(Debug, Clone, Copy)]
struct Hit {
    hit: LandedHit,
    /// The dummy's health before and after it.
    was: u16,
    now: u16,
    /// The session's read on the hit's frame.
    read: FightRead,
    /// It landed on the last frame of the dummy's reaction and started it
    /// over.
    restarted: bool,
}

impl Hit {
    fn at_floor(&self) -> bool {
        self.was == FLOOR
    }

    /// The floor took off less than the hit's damage.
    fn cut_short(&self) -> bool {
        self.hit.damage > self.was - self.now
    }
}

/// Walk `s` with P1's `script` beside its shadow, every frame checked: the
/// session reads the hits the shadow reads, its machine moves as the
/// shadow's does, the Training state is the same, and every hit reaction
/// that begins, or starts over, is read as a hit. Stops once `until` holds
/// for P1's hits on the dummy, then walks `after` frames more; returns them.
fn walk(
    s: &mut GameSession,
    script: impl Fn(u32, &GameSession) -> FrameInputs,
    until: impl Fn(&[Hit]) -> bool,
    after: u32,
    what: &str,
) -> Vec<Hit> {
    let mut shadow = Shadow::new(s);
    let mut hits = Vec::new();
    let mut f = 0u32;
    let mut left = None;
    while left != Some(0) {
        let input = script(f, s);
        let was = health(s.fight_machine().unwrap(), Side::P2);
        let before = *s.fight().unwrap().fight_read().fighter(Side::P2);
        s.step(input);
        let theirs = shadow.step(input);
        let m = s.fight_machine().unwrap();
        let read = *s.fight().unwrap().fight_read();
        assert_eq!(s.screen(), "fight", "{what}: the fight goes on");
        assert!(m.state.objectram == shadow.m.state.objectram, "{what} frame {f}: the shadow moves as the fight does");
        assert_eq!(m.training(), shadow.m.training(), "{what} frame {f}: the same Training state");
        assert_eq!(read.landed, theirs.landed, "{what} frame {f}: the hits a fight with unlimited health reads");
        assert_eq!(read.landed_by(Side::P2), None, "{what} frame {f}: the dummy never hits");
        let now = read.fighter(Side::P2);
        let restarted = before.reacting() && before.state & 0x80 == 0 && now.state & 0x80 != 0;
        if now.reacting() && (!before.reacting() || restarted) {
            assert!(read.landed_by(Side::P1).is_some(), "{what} frame {f}: a reaction began at {was} and is read as a hit");
        }
        if let Some(hit) = read.landed_by(Side::P1) {
            hits.push(Hit { hit, was, now: health(m, Side::P2), read, restarted });
        }
        if left.is_none() && until(&hits) {
            left = Some(after);
        }
        left = left.map(|n| n.saturating_sub(1));
        f += 1;
        assert!(f < 2_000, "{what}: the walk's hits ({hits:?})");
    }
    hits
}

/// Walk `s` with [`attack`] until `n` hits have landed at the floor, then
/// `after` frames more.
fn walk_to_the_floor(s: &mut GameSession, n: usize, after: u32, what: &str) -> Vec<Hit> {
    walk(s, |f, _| attack(f), |hits| hits.iter().filter(|h| h.at_floor()).count() >= n, after, what)
}

#[test]
fn hits_at_the_floor_are_read_as_in_a_fight_with_unlimited_health_under_every_preset() {
    for preset in PRESETS {
        let Some(a) = assets() else { return };
        let mut s = training_dummy(a, preset, Settings::default());
        let what = format!("{preset:?}");
        let hits = walk_to_the_floor(&mut s, 3, 120, &what);
        // Every hit is read, at the floor too, each with damage; the floor
        // never let one under it.
        assert!(hits.iter().all(|h| h.hit.damage > 0 && h.now >= FLOOR), "{what}: {hits:?}");
        let floor: Vec<&Hit> = hits.iter().filter(|h| h.at_floor()).collect();
        assert!(floor.len() >= 3, "{what}: hits at the floor counted ({hits:?})");
        assert!(floor.iter().all(|h| h.now == FLOOR && h.cut_short()), "{what}: at the floor, nothing taken ({floor:?})");
        // The hit that reached the floor took less than its damage, and
        // reads its full damage.
        let reached = hits.iter().find(|h| h.was > FLOOR && h.now == FLOOR).expect("a hit reached the floor");
        assert!(reached.hit.damage >= reached.was - FLOOR, "{what}: {reached:?}");
        // Ryu's HP is the same hit at the floor as above it.
        let above = hits.iter().filter(|h| !h.cut_short()).map(|h| h.hit.damage).max().unwrap();
        assert!(floor.iter().all(|h| h.hit.damage <= above), "{what}: no more than the same hits above the floor");
        let m = s.fight_machine().unwrap();
        assert!(m.unrecovered().is_empty(), "{what}: {:?}", m.unrecovered());
    }
}

#[test]
fn a_hit_on_the_last_frame_of_a_reaction_at_the_floor_is_read_under_every_preset() {
    for preset in PRESETS {
        let Some(a) = assets() else { return };
        let mut s = training_dummy(a, preset, Settings::default());
        let what = format!("{preset:?}");
        let blind = |hits: &[Hit]| hits.iter().filter(|h| h.at_floor() && h.restarted).count();
        let hits = walk(&mut s, close_hp, |hits| blind(hits) >= 2, 0, &what);
        // It took no health and changed no move: only the record shows it.
        let h = hits.iter().find(|h| h.at_floor() && h.restarted).unwrap();
        assert_eq!((h.now, h.hit.damage > 0), (FLOOR, true), "{what}: {h:?}");
        assert!(!h.hit.into_hitstun, "{what}: the reaction had ended, so it combos with nothing");
    }
}

#[test]
fn a_combo_of_hits_at_the_floor_shows_its_full_damage_under_hold_combo() {
    let Some(a) = assets() else { return };
    let settings = Settings { hold_combo: true, ..Settings::default() };
    let mut s = training_dummy(a, RulesetPreset::DefinitiveOverhaul, settings);
    assert!(s.fight().unwrap().hold_combo());
    let hits = walk_to_the_floor(&mut s, 3, 0, "DefinitiveOverhaul");
    // The last hits: the one that reached the floor and those at it.
    let first = hits.iter().position(|h| h.was > FLOOR && h.now == FLOOR).unwrap();
    let string = &hits[first..];
    assert!(string.len() >= 4 && string[1..].iter().all(Hit::at_floor), "{string:?}");
    // A 1987 fight has no true combos (see combo_counter.rs): the machine's
    // own reads of these hits, each made to land into the hitstun of the
    // one before, stand in for the reader's, as the combo tests feed them.
    let mut reads = Vec::new();
    for (i, h) in string.iter().enumerate() {
        assert!(h.read.fighter(Side::P2).hitstun(), "the dummy in hitstun on the hit's frame: {h:?}");
        let mut read = h.read;
        read.landed[0] = Some(LandedHit { into_hitstun: i > 0, ..h.hit });
        reads.push(read);
        read.landed = [None; 2];
        reads.extend([read; 3]);
    }
    s.feed_fight_reads(&reads);
    for _ in 0..reads.len() + 5 {
        s.step(idle());
    }
    let full: u16 = string.iter().map(|h| h.hit.damage).sum();
    let taken: u16 = string.iter().map(|h| h.was - h.now).sum();
    assert!(full > taken, "the floor cut the string short ({full} > {taken})");
    let shown = s.fight().unwrap().callouts().frame().side(Side::P1).combo;
    assert_eq!(shown, Some(ComboCount { hits: string.len() as u16, damage: Some(full) }), "every hit, at its full damage");
}

/// Open the pause box and confirm RESET POSITIONS (its fifth row).
fn reset_positions(s: &mut GameSession) {
    tap(s, with(|i| i.menu.start = true));
    for _ in 0..4 {
        tap(s, down());
    }
    let out = s.step(confirm());
    assert_eq!(out.transition, Some(Transition::Choose(MenuItem::ResetPositions)));
    assert_eq!(out.screen, "fight");
}

/// Walk `s` with [`attack`] for `frames`; returns the frames P1 landed a
/// clean hit and those on which FIRST ATTACK was called.
fn first_attacks(s: &mut GameSession, frames: u32) -> (Vec<u32>, Vec<u32>, usize) {
    let (mut hits, mut called, mut floor) = (Vec::new(), Vec::new(), 0);
    for f in 0..frames {
        let was = health(s.fight_machine().unwrap(), Side::P2);
        let out = s.step(attack(f));
        if out.announcements.contains(&AnnouncerLine::FirstAttack) {
            called.push(f);
        }
        if s.fight().unwrap().fight_read().landed_by(Side::P1).is_some_and(|h| !h.chip) {
            hits.push(f);
            floor += usize::from(was == FLOOR);
        }
    }
    (hits, called, floor)
}

#[test]
fn first_attack_calls_once_a_training_session_and_again_after_reset_positions() {
    let Some(a) = assets() else { return };
    let mut s = training_dummy(a, RulesetPreset::DefinitiveOverhaul, Settings::default());
    // The session: the dummy hit to the floor and on; one call, the first hit.
    let (hits, called, floor) = first_attacks(&mut s, 1_200);
    assert!(floor >= 3, "hits at the floor ({hits:?})");
    assert_eq!(called, vec![hits[0]], "once, on the session's first hit");
    // RESET POSITIONS: earned again, once.
    reset_positions(&mut s);
    let (hits, called, _) = first_attacks(&mut s, 600);
    assert!(hits.len() >= 3, "{hits:?}");
    assert_eq!(called, vec![hits[0]], "again after RESET POSITIONS, on its first hit");
}

#[test]
fn the_record_goes_with_the_snapshot_and_a_replay_reads_the_same_hits() {
    let Some(a) = assets() else { return };
    let mut s = training_dummy(a, RulesetPreset::DefinitiveOverhaul, Settings::default());
    let round_start = s.fight().unwrap().round_start().expect("a round has started").clone();
    let mut straight = s.clone();
    walk_to_the_floor(&mut s, 2, 0, "first walk");
    let m = s.fight_machine().unwrap();
    assert_ne!(*m.training(), round_start.training, "hits recorded");
    // Rollback: from a snapshot at the floor, the same frames record the same.
    let mut copy = m.clone();
    let at_floor = copy.save_snapshot();
    let record = |m: &mut PowerOnMachine| -> Vec<TrainingState> {
        (0..300u32)
            .map(|f| {
                let i = attack(f);
                let [r, l, d, u] = i.p1.directions;
                m.set_p1_direction(r, l, d, u);
                m.set_p1_buttons(i.p1.buttons);
                m.step();
                *m.training()
            })
            .collect()
    };
    let once = record(&mut copy);
    assert!(once.windows(2).any(|w| w[0] != w[1]), "hits recorded in the frames replayed");
    copy.restore_snapshot(&at_floor);
    assert_eq!(record(&mut copy), once, "restored, the same record frame for frame");
    // RESET POSITIONS puts the record back to the round's start, and the
    // fight from there reads each hit once, as the fight played straight.
    reset_positions(&mut s);
    assert_eq!(s.fight_machine().unwrap().save_snapshot(), round_start, "bit for bit, the record included");
    to_fight_loop(&mut s);
    assert_eq!(s.fight_machine().unwrap().save_snapshot(), straight.fight_machine().unwrap().save_snapshot());
    let mut floor = 0;
    for f in 0..1_200u32 {
        let was = health(s.fight_machine().unwrap(), Side::P2);
        s.step(attack(f));
        straight.step(attack(f));
        let (replay, played) = (s.fight().unwrap().fight_read(), straight.fight().unwrap().fight_read());
        assert_eq!(replay, played, "frame {f}: the replay reads what the straight fight read");
        floor += usize::from(was == FLOOR && replay.landed_by(Side::P1).is_some());
    }
    assert!(floor >= 3, "the replay hit at the floor ({floor})");
}

#[test]
fn outside_training_a_hit_is_read_by_the_health_it_took_and_nothing_is_recorded() {
    for preset in [RulesetPreset::DefinitiveOverhaul, RulesetPreset::Faithful1987] {
        let Some(a) = assets() else { return };
        let mut s = local(a, preset);
        let what = format!("{preset:?}");
        let mut hits = 0;
        for f in 0..3_000u32 {
            let was = s.fight().unwrap().fight_read().fighters;
            s.step(attack(f));
            if s.screen() != "fight" {
                break;
            }
            let read = *s.fight().unwrap().fight_read();
            for side in Side::BOTH {
                let d = 1 - side.index();
                let lost = was[d].health.saturating_sub(read.fighters[d].health);
                let expect = (lost > 0 && read.fighters[d].reacting()).then_some(lost);
                assert_eq!(read.landed_by(side).map(|h| h.damage), expect, "{what} frame {f}: by the health taken");
                hits += usize::from(expect.is_some());
            }
            assert_eq!(*s.fight_machine().unwrap().training(), TrainingState::default(), "{what}: no record in Local");
            if read.fighters.iter().any(|f| f.knocked_out()) {
                break;
            }
        }
        assert!(hits >= 5, "{what}: hits read ({hits})");
    }
}

