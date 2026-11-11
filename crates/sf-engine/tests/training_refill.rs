//! Ticket #243 (`TR2`; spec #238, design §5.1, ADR-0013): the refill.
//!
//! In Training, under every preset, a fighter's health comes back to full
//! once a second (60 fight frames) has passed since they were last hit or
//! blocked an attack, they are out of their hit reaction and their guard,
//! and they are below full health. Any hit or block starts the wait over,
//! a blocked attack that does no damage too. Headless walks through the
//! game session into Training against the dummy and against the CPU,
//! reading each frame through the session's read-only views: health and
//! reactions from the fight reader, blocks from the block sound (command
//! 8, which only a blocked attack queues) on a guarding fighter, the bar
//! from video RAM. Local ROMs only; each test skips without them.

#[path = "support/select.rs"]
mod select;

use std::path::Path;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::fight_reader::FighterRead;
use sf_engine::fighter::P1_STRUCT;
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::{FrameInputs, GameSession, MenuItem, Transition};
use sf_engine::moves::facing_right;
use sf_engine::roster::{PlayMode, TournamentFighter};
use sf_engine::run::{PowerOnMachine, PowerOnSnapshot};
use sf_engine::training::{TrainingState, TRAINING_REFILL_FRAMES};
use sf_video::callouts::Side;
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

const FIGHT_LOOP: u32 = 0x3eafa;
/// The KO slow motion flag (0xffc00b): while it is up the fight loop runs
/// one pass in six frames, so the walks here keep it down.
const SLOW_MOTION: u32 = 0xffc00b;
/// The sound queue's write index (0xff8660) and its 16-word ring (0xff8662).
const SOUND_WRITE: u32 = 0xff8660;
const SOUND_QUEUE: u32 = 0xff8662;
/// The block sound: every blocked branch of the hit functions queues it.
const BLOCK_SOUND: u16 = 8;
/// Full health: 11 cells of 4 and a remainder of 4 (`round_hud`).
const FULL: u16 = 48;
const WAIT: u32 = TRAINING_REFILL_FRAMES as u32;

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

const RIGHT: usize = 0;
const LEFT: usize = 1;
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

fn ww(m: &PowerOnMachine, a: u32) -> u16 {
    u16::from_be_bytes([wb(m, a), wb(m, a + 1)])
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

/// Power-on under `preset`, the Main Menu, Single Player, Training, CPU or
/// DUMMY; P1 Ryu against the opponent under the cursor; the first fight
/// loop.
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
    select::confirm_fighter(&mut s, with(|i| i.p1.buttons[0] = true), tap);
    select::confirm_fighter(&mut s, with(|i| i.p2.buttons[0] = true), tap);
    while s.screen() != "country select" {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    to_fight_loop(&mut s);
    assert_eq!(s.fight_machine().unwrap().match_config().roster.mode, PlayMode::Versus);
    s
}

/// P1 walks in for a quarter second, then presses HP and HK by turns, five
/// times a second (the health-floor tests' attack).
fn attack(f: u32) -> FrameInputs {
    let phase = f % 60;
    with(|i| {
        i.p1.directions[RIGHT] = phase < 15;
        if phase >= 15 && phase % 12 < 3 {
            i.p1.buttons[if (f / 12) % 2 == 0 { 2 } else { 5 }] = true;
        }
    })
}

/// P1 presses HP every 24 frames and walks in between: under every preset,
/// once P1 is in range, a hit every 48 frames.
fn pressure(f: u32) -> FrameInputs {
    with(|i| {
        i.p1.directions[RIGHT] = f % 24 >= 6;
        i.p1.buttons[2] = f % 24 < 3;
    })
}

/// The sound commands the machine queued since the write index was `from`.
fn queued_since(m: &PowerOnMachine, from: u16) -> Vec<u16> {
    let to = ww(m, SOUND_WRITE);
    let mut at = from;
    let mut cmds = Vec::new();
    while at != to {
        cmds.push(ww(m, SOUND_QUEUE + u32::from(at)));
        at = (at + 2) % 0x20;
    }
    cmds
}

/// A side's player structure in this fight (the CPU's stands in for P2
/// against the CPU, game mode 2).
fn player_struct(m: &PowerOnMachine, side: Side) -> u32 {
    match (side, wb(m, 0xffc002)) {
        (Side::P1, _) => P1_STRUCT,
        (Side::P2, 2) => sf_engine::fighter::CPU_STRUCT,
        (Side::P2, _) => sf_engine::fighter::P2_STRUCT,
    }
}

/// A side's twelve health-bar cells in video RAM, from its +0xa offset.
fn bar(m: &PowerOnMachine, video: &[u8], side: Side) -> Vec<u8> {
    let at = usize::from(ww(m, player_struct(m, side) + 0xa));
    video[at..at + 24].to_vec()
}

/// A refill one side got: the frame, and its last hit before it.
#[derive(Debug, Clone, Copy)]
struct Refill {
    frame: u32,
    hit: Option<u32>,
}

/// What a walk saw of one side, every frame checked against the rule: the
/// health goes back to full exactly when 60 fight frames have passed since
/// the side's last hit or block, it is out of its hit reaction and its
/// guard, and it was below full; never otherwise.
#[derive(Debug, Default, Clone)]
struct Seen {
    hits: Vec<u32>,
    blocks: Vec<u32>,
    refills: Vec<Refill>,
    /// Frames free of any reaction or guard, below full, that the wait held
    /// off: the side would have refilled there without the wait.
    held: Vec<u32>,
}

impl Seen {
    fn last_contact(&self) -> Option<u32> {
        self.hits.last().copied().max(self.blocks.last().copied())
    }
}

/// Watches every stepped frame of a fight; see [`Seen`].
struct Watch {
    frame: u32,
    sides: [Seen; 2],
    last: [FighterRead; 2],
    /// The round start's bar cells, to check each refill's bar against.
    full_bars: [Vec<u8>; 2],
}

impl Watch {
    fn new(s: &GameSession) -> Watch {
        let fight = s.fight().unwrap();
        let m = fight.machine();
        let start = fight.round_start().expect("a round has started");
        let full_bars = Side::BOTH.map(|side| bar(m, &start.state.videoram, side));
        Watch { frame: 0, sides: Default::default(), last: fight.fight_read().fighters, full_bars }
    }

    fn step(&mut self, s: &mut GameSession, input: FrameInputs) {
        let from = ww(s.fight_machine().unwrap(), SOUND_WRITE);
        s.step(input);
        self.frame += 1;
        let f = self.frame;
        assert_eq!(s.screen(), "fight", "the fight goes on");
        let m = s.fight_machine().unwrap();
        assert_eq!(flow_position(m), FIGHT_LOOP, "frame {f}: the fight loop");
        assert_eq!(wb(m, SLOW_MOTION), 0, "frame {f}: no slow motion, so a frame is a fight frame");
        let blocked = queued_since(m, from).contains(&BLOCK_SOUND);
        let read = s.fight().unwrap().fight_read().fighters;
        let guarding: Vec<Side> = Side::BOTH.into_iter().filter(|side| read[side.index()].guarding()).collect();
        assert!(!(blocked && guarding.len() == 2), "frame {f}: whose block?");
        for side in Side::BOTH {
            let (was, now) = (self.last[side.index()], read[side.index()]);
            let seen = &mut self.sides[side.index()];
            // A hit starts a reaction, takes health, or starts the reaction
            // over: busy again (+9 bit 7) after its last frame, which is
            // all a hit at the floor on that frame shows.
            let restarted = was.state & 0x80 == 0 && now.state & 0x80 != 0;
            let hit = now.reacting() && (!was.reacting() || now.health < was.health || restarted);
            if hit {
                seen.hits.push(f);
            } else if blocked && now.guarding() {
                assert_eq!(now.health, was.health, "frame {f}: {side:?} blocked for no damage");
                seen.blocks.push(f);
            }
            let free = !now.reacting() && !now.guarding() && was.health < FULL && !hit;
            let waited = seen.last_contact().is_none_or(|c| f >= c + WAIT);
            let refilled = now.health > was.health;
            assert_eq!(
                refilled,
                free && waited,
                "frame {f}: {side:?} {was:?} -> {now:?}, last hit {:?}, last block {:?}",
                seen.hits.last(),
                seen.blocks.last()
            );
            if refilled {
                assert_eq!(now.health, FULL, "frame {f}: {side:?} back to full");
                assert_eq!(bar(m, &m.state.videoram, side), self.full_bars[side.index()], "frame {f}: {side:?}'s bar drawn full");
                seen.refills.push(Refill { frame: f, hit: seen.hits.last().copied() });
            } else if free {
                seen.held.push(f);
            }
        }
        self.last = read;
    }

    fn side(&self, side: Side) -> &Seen {
        &self.sides[side.index()]
    }
}

#[test]
fn the_dummy_refills_a_second_after_its_last_hit_and_not_before_under_every_preset() {
    for preset in PRESETS {
        let Some(a) = assets() else { return };
        let mut s = training(a, preset, true);
        let what = format!("{preset:?}");
        let mut w = Watch::new(&s);
        // P1 attacks until the dummy is hit once, then stands still.
        let mut f = 0;
        while w.side(Side::P2).hits.is_empty() {
            w.step(&mut s, attack(f));
            f += 1;
            assert!(f < 600, "{what}: the dummy hit");
        }
        let hit = w.frame;
        let hurt = s.fight().unwrap().fight_read().fighter(Side::P2).health;
        assert!(hurt < FULL, "{what}: a hit that hurt ({hurt})");
        while w.side(Side::P2).refills.is_empty() {
            w.step(&mut s, idle());
            assert!(w.frame < hit + 300, "{what}: the dummy refills");
        }
        let refill = w.side(Side::P2).refills[0];
        // The reaction ended inside the second, and the second decided it.
        assert_eq!(refill.hit, w.side(Side::P2).hits.last().copied());
        assert_eq!(refill.frame, refill.hit.unwrap() + WAIT, "{what}: a second after the hit, not before");
        assert!(!w.side(Side::P2).held.is_empty(), "{what}: out of its reaction, it waited for the second");
        assert_eq!(s.fight().unwrap().fight_read().fighter(Side::P2).health, FULL, "{what}");
        // It stays full, and P1 was never touched.
        for _ in 0..120 {
            w.step(&mut s, idle());
        }
        assert_eq!(w.side(Side::P2).refills.len(), 1, "{what}");
        assert!(w.side(Side::P1).hits.is_empty() && w.side(Side::P1).refills.is_empty(), "{what}");
        assert!(s.fight_machine().unwrap().unrecovered().is_empty(), "{what}");
    }
}

#[test]
fn a_string_of_hits_holds_the_dummys_refill_off_until_a_second_after_the_last_under_every_preset() {
    for preset in PRESETS {
        let Some(a) = assets() else { return };
        let mut s = training(a, preset, true);
        let what = format!("{preset:?}");
        let mut w = Watch::new(&s);
        for f in 0..600 {
            w.step(&mut s, pressure(f));
        }
        // The string: the last hits, each within a second of the one before.
        let dummy = w.side(Side::P2).clone();
        let mut first = dummy.hits.len() - 1;
        while first > 0 && dummy.hits[first] - dummy.hits[first - 1] < WAIT {
            first -= 1;
        }
        let string = &dummy.hits[first..];
        assert!(string.len() >= 4, "{what}: a string of hits ({:?})", dummy.hits);
        let (start, last) = (string[0], *string.last().unwrap());
        // Between each two hits the dummy stood free of its reaction, below
        // full, and the wait held its refill off; none came in the string.
        for pair in string.windows(2) {
            let held = dummy.held.iter().filter(|&&f| f > pair[0] && f < pair[1]).count();
            assert!(held > 0, "{what}: free and held off between the hits at {pair:?}");
        }
        assert!(dummy.refills.iter().all(|r| r.frame < start), "{what}: no refill in the string ({:?})", dummy.refills);
        // P1 stops: the refill comes a second after the last hit.
        while w.side(Side::P2).refills.last().is_none_or(|r| r.frame < last) {
            w.step(&mut s, idle());
            assert!(w.frame < last + 300, "{what}: the dummy refills after the string");
        }
        let refill = *w.side(Side::P2).refills.last().unwrap();
        assert_eq!(refill.hit, Some(w.side(Side::P2).hits.last().copied().unwrap()));
        assert_eq!(refill.frame, refill.hit.unwrap() + WAIT, "{what}: a second after the last hit");
    }
}

/// P1 against the CPU: P1 stands still, and as each of its hit reactions
/// ends holds back (the guard) for `guard` frames.
struct GuardAfterHits {
    guard: u32,
    until: u32,
    reacting: bool,
    /// Guard crouching (down-back) rather than standing.
    low: bool,
}

impl GuardAfterHits {
    fn new(guard: u32) -> Self {
        GuardAfterHits { guard, until: 0, reacting: false, low: false }
    }

    /// The guard against the fair CPU (SP21), the overhaul presets' Training
    /// CPU: it does not attack into a hit reaction it sees, a reaction delay
    /// late, so its next attack after a hit comes later than Capcom's, and
    /// it presses on afterwards. P1 holds the crouching guard (down-back,
    /// which under standard blocking stops all but jump attacks) for
    /// [`FAIR_CPU_GUARD`] frames, so a block comes inside the wait and no
    /// hit spoils the second after it.
    fn against_the_fair_cpu() -> Self {
        GuardAfterHits { low: true, ..GuardAfterHits::new(FAIR_CPU_GUARD) }
    }

    fn input(&mut self, w: &Watch, s: &GameSession) -> FrameInputs {
        let reacting = w.last[0].reacting();
        if self.reacting && !reacting {
            self.until = w.frame + self.guard;
        }
        self.reacting = reacting;
        let m = s.fight_machine().unwrap();
        let right = facing_right(m.work_ram(), &m.state.objectram, P1_STRUCT);
        let back = w.frame < self.until;
        with(|i| {
            i.p1.directions[if right { LEFT } else { RIGHT }] = back;
            i.p1.directions[DOWN] = back && self.low;
        })
    }
}

/// How long P1 guards after a hit against the fair CPU.
const FAIR_CPU_GUARD: u32 = 150;

/// A block, with no later contact, holds a free, wounded fighter's refill
/// past the old hit's deadline and inside the block's new 60-frame wait.
fn put_off_by_a_block(side: &Seen) -> Option<(u32, u32, u32)> {
    let hit = *side.hits.last()?;
    let block = *side.blocks.last()?;
    if block <= hit {
        return None;
    }
    side.held.iter().copied().find(|&f| f > block && f >= hit + WAIT && f < block + WAIT)
        .map(|held| (hit, block, held))
}

#[test]
fn a_blocked_attack_that_does_no_damage_starts_the_wait_over_under_every_preset() {
    for preset in PRESETS {
        let Some(a) = assets() else { return };
        let mut s = training(a, preset, false);
        let what = format!("{preset:?}");
        let mut w = Watch::new(&s);
        // A guard after each hit that, against this CPU, blocks an attack
        // inside the second after a hit and then lets go.
        let mut p1 = if preset == RulesetPreset::Faithful1987 { GuardAfterHits::new(30) } else { GuardAfterHits::against_the_fair_cpu() };
        p1.low = false;
        while put_off_by_a_block(w.side(Side::P1)).is_none() {
            let input = p1.input(&w, &s);
            w.step(&mut s, input);
            assert!(w.frame < 3_000, "{what}: a refill put off by a block ({:?})", w.side(Side::P1));
        }
        let me = w.side(Side::P1);
        let (hit, block, held) = put_off_by_a_block(me).unwrap();
        // The blocked attack did no damage (the watch checks it). A second
        // after the hit P1 stands free and below full, with no newer hit or
        // block, yet cannot refill inside the block's fresh 60-frame wait.
        assert!(hit < block && held >= hit + WAIT && held < block + WAIT, "{what}: hit {hit}, block {block}, held {held}");
        assert_eq!(me.last_contact(), Some(block), "{what}: only the zero-damage block restarted the wait");
        let now = w.last[Side::P1.index()];
        assert!(now.health < FULL && !now.reacting() && !now.guarding(), "{what}: free and still waiting: {now:?}");
    }
}

#[test]
fn under_every_preset_the_player_refills_against_the_cpu_and_blocks_hold_it_off() {
    for preset in PRESETS {
        let Some(a) = assets() else { return };
        let mut s = training(a, preset, false);
        let what = format!("{preset:?}");
        let mut w = Watch::new(&s);
        let mut p1 = if preset == RulesetPreset::Faithful1987 { GuardAfterHits::new(20) } else { GuardAfterHits::against_the_fair_cpu() };
        for _ in 0..3_000 {
            let input = p1.input(&w, &s);
            w.step(&mut s, input);
        }
        let me = w.side(Side::P1);
        assert!(me.hits.len() >= 5, "{what}: the CPU hits P1 ({:?})", me.hits);
        assert!(me.blocks.len() >= 3, "{what}: P1 blocks the CPU ({:?})", me.blocks);
        assert!(me.refills.len() >= 2, "{what}: P1 refills ({:?})", me.refills);
        assert!(s.fight_machine().unwrap().unrecovered().is_empty(), "{what}");
    }
}

/// P1 hits the dummy once, then stands still until the dummy is out of
/// its reaction and a few frames more: mid-wait, below full, no refill yet.
fn hit_once_into_the_wait(s: &mut GameSession, w: &mut Watch) {
    let mut f = 0;
    while w.side(Side::P2).hits.is_empty() {
        w.step(s, attack(f));
        f += 1;
        assert!(f < 600, "the dummy hit");
    }
    while w.last[1].reacting() {
        w.step(s, idle());
    }
    for _ in 0..5 {
        w.step(s, idle());
    }
    let dummy = w.side(Side::P2);
    assert!(w.frame < dummy.hits.last().unwrap() + WAIT && dummy.refills.is_empty(), "mid-wait: {dummy:?}");
    assert!(w.last[1].health < FULL);
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

#[test]
fn reset_positions_restores_the_refill_wait_and_a_replay_matches_a_fight_played_straight_through() {
    let Some(a) = assets() else { return };
    let mut s = training(a, RulesetPreset::DefinitiveOverhaul, true);
    let round_start: PowerOnSnapshot = s.fight().unwrap().round_start().expect("a round has started").clone();
    // The one played straight through: from the fight loop's first frame.
    let mut straight = s.clone();
    // Mid-wait: the dummy hit and below full, its wait half run.
    let mut w = Watch::new(&s);
    hit_once_into_the_wait(&mut s, &mut w);
    let mid = s.fight_machine().unwrap().save_snapshot();
    assert_ne!(mid.training, round_start.training, "the wait is running");
    reset_positions(&mut s);
    assert_eq!(s.fight_machine().unwrap().save_snapshot(), round_start, "bit for bit, the Training state included");
    // From the round's start to the fight loop, then the same fight as the
    // straight one: hits, the wait, the refill and on, frame for frame.
    to_fight_loop(&mut s);
    assert_eq!(s.fight_machine().unwrap().save_snapshot(), straight.fight_machine().unwrap().save_snapshot());
    let mut w_replay = Watch::new(&s);
    let mut w_straight = Watch::new(&straight);
    for f in 0..900u32 {
        let input = if f < 300 { attack(f) } else { idle() };
        w_replay.step(&mut s, input);
        w_straight.step(&mut straight, input);
        assert_eq!(
            s.fight_machine().unwrap().save_snapshot(),
            straight.fight_machine().unwrap().save_snapshot(),
            "frame {f}: the replay matches the straight fight"
        );
    }
    assert!(!w_straight.side(Side::P2).refills.is_empty(), "the dummy refilled in the fight compared");
}

/// Step a machine `frames` fight frames with idle pads; returns the dummy's
/// health on each.
fn health_trace(m: &mut PowerOnMachine, frames: u32) -> Vec<u16> {
    m.set_p1_direction(false, false, false, false);
    m.set_p1_buttons([false; 6]);
    (0..frames)
        .map(|_| {
            m.step();
            let ps = player_struct(m, Side::P2);
            4 * u16::from(wb(m, ps + 7)) + u16::from(wb(m, ps + 6))
        })
        .collect()
}

#[test]
fn a_snapshot_taken_mid_wait_replays_the_refill_on_the_same_frame() {
    let Some(a) = assets() else { return };
    let mut s = training(a, RulesetPreset::Faithful1987, true);
    let mut w = Watch::new(&s);
    hit_once_into_the_wait(&mut s, &mut w);
    let hurt = s.fight().unwrap().fight_read().fighter(Side::P2).health;
    // Rollback: save mid-wait, play on past the refill, restore, play again.
    let mut m = s.fight_machine().unwrap().clone();
    let mid = m.save_snapshot();
    let straight = health_trace(&mut m, WAIT);
    let refill = straight.iter().position(|&h| h == FULL).expect("the refill comes");
    assert!(straight[..refill].iter().all(|&h| h == hurt), "{straight:?}");
    let late = m.save_snapshot();
    assert_ne!(late.training, mid.training);
    m.restore_snapshot(&mid);
    assert_eq!(health_trace(&mut m, WAIT), straight, "restored, the refill comes on the same frame");
    // Without the wait in the snapshot, it would come at once: the wait is
    // what the snapshot carries.
    let mut lost = mid.clone();
    lost.training = late.training;
    m.restore_snapshot(&lost);
    assert_eq!(health_trace(&mut m, 1), [FULL], "a later wait refills at once");
}

#[test]
fn local_has_no_refill() {
    for preset in [RulesetPreset::DefinitiveOverhaul, RulesetPreset::Faithful1987] {
        let Some(a) = assets() else { return };
        let mut s = local(a, preset);
        let what = format!("{preset:?}");
        let mut hurt = None;
        for f in 0..600u32 {
            s.step(attack(f));
            let health = s.fight().unwrap().fight_read().fighter(Side::P2).health;
            if health < FULL {
                hurt = Some(health);
                break;
            }
        }
        // P1 lets go; P2's health never comes back.
        let mut low = hurt.unwrap_or_else(|| panic!("{what}: P2 hit"));
        for _ in 0..600 {
            s.step(idle());
            let m = s.fight_machine().unwrap();
            let health = s.fight().unwrap().fight_read().fighter(Side::P2).health;
            assert!(health <= low, "{what}: no refill in Local ({low} -> {health})");
            low = health;
            assert_eq!(m.save_snapshot().training, TrainingState::default(), "{what}: no Training state in Local");
        }
    }
}
