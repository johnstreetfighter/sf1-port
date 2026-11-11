//! Ticket #222 (EH6): FIRST ATTACK, REVERSAL and PERFECT in real fights, at
//! the front end's fight-screen seam.
//!
//! Every test plays real rounds with scripted inputs, as a player would:
//! one side attacks a fighter who stands still, so the round is won at full
//! health. FIRST ATTACK goes to the round's first clean hit, once a round
//! and again after Restart Round; chip on a guard is not one. PERFECT shows
//! on the winner's side on the knockout of a round won without taking
//! damage, clear of the ROM's round-end text. The announcer calls both, the
//! machine's RAM is a bare machine's given the same inputs, and the
//! Faithful 1987 preset shows and says nothing.
//!
//! REVERSAL is literal (user, 2026-11-04): only out of true blockstun or a
//! get-up, which the 1987 fight has neither of (its normal blocks have no
//! blockstun and every knockdown is a KO). A real fight with chip locks
//! and hit reactions earns none; fed reads (`GameSession::feed_fight_reads`)
//! stand in for a style that reads those states. Local ROMs only; each test
//! skips without them.

#[path = "support/select.rs"]
mod select;

use std::path::Path;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::fight_reader::{FightRead, FighterRead, LandedHit};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::pause::PauseBox;
use sf_engine::front::{FrameInputs, FrameOutput, GameSession, MenuItem, Transition};
use sf_engine::run::PowerOnMachine;
use sf_video::announcer::AnnouncerLine;
use sf_video::callouts::{regions, CalloutKind, Part, Side};
use sf_video::decode::decode_char_tile;
use sf_video::render::{VISAREA_X, VISAREA_Y};
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const FIGHT_LOOP: u32 = 0x3eafa;
const RIGHT: usize = 0;
const LEFT: usize = 1;
const DOWN: usize = 2;
const HP: usize = 2;
const MK: usize = 4;

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

fn tap(s: &mut GameSession, input: FrameInputs) -> FrameOutput {
    let out = s.step(input);
    s.step(idle());
    out
}

fn confirm() -> FrameInputs {
    with(|i| i.menu.confirm = true)
}

fn start() -> FrameInputs {
    with(|i| i.menu.start = true)
}

fn to_fight_loop(s: &mut GameSession) {
    let mut f = 0;
    while s.fight_machine().map(flow_position) != Some(FIGHT_LOOP) {
        s.step(idle());
        f += 1;
        assert!(f < 3_000, "the fight loop (at {})", s.screen());
    }
}

/// A Local fight (P1 Ryu, P2 Ken, Japan) under `preset`, at the first fight loop.
fn local_fight(a: Assets, preset: RulesetPreset) -> GameSession {
    let mut s = GameSession::new(a, MatchConfig::from_preset(preset));
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, start());
    tap(&mut s, with(|i| i.p1.directions[DOWN] = true));
    tap(&mut s, confirm());
    tap(&mut s, confirm());
    select::confirm_fighter(&mut s, with(|i| i.p1.buttons[0] = true), tap);
    select::confirm_fighter(&mut s, with(|i| i.p2.buttons[0] = true), tap);
    while s.screen() != "country select" {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    to_fight_loop(&mut s);
    s
}

/// Arcade as Ryu from Japan, against the CPU, at the first fight loop.
fn arcade(a: Assets) -> GameSession {
    let mut s = GameSession::new(a, MatchConfig::default());
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, start());
    assert_eq!(tap(&mut s, confirm()).screen, "single player");
    assert_eq!(tap(&mut s, confirm()).transition, Some(Transition::Choose(MenuItem::Arcade)));
    select::confirm_fighter(&mut s, confirm(), tap); // Ryu
    while s.screen() != "country select" {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.p1.buttons[0] = true)); // Japan
    to_fight_loop(&mut s);
    s
}

/// `attacker` walks in and throws a heavy punch every 50 frames; the other
/// side stands still. Ryu starts on the left, Ken on the right.
fn pressing(attacker: Side, t: u32) -> FrameInputs {
    with(|i| {
        let c = t % 50;
        let (pad, forward) = match attacker {
            Side::P1 => (&mut i.p1, RIGHT),
            Side::P2 => (&mut i.p2, LEFT),
        };
        pad.directions[forward] = c < 20;
        pad.buttons[HP] = (30..32).contains(&c);
    })
}

/// Step the bare machine with the inputs the fight gives it.
fn step_bare(bare: &mut PowerOnMachine, input: FrameInputs) {
    let [r, l, d, u] = input.p1.directions;
    bare.set_p1_direction(r, l, d, u);
    bare.set_p1_buttons(input.p1.buttons);
    let [r, l, d, u] = input.p2.directions;
    bare.set_p2_direction(r, l, d, u);
    bare.set_p2_buttons(input.p2.buttons);
    bare.set_cabinet_inputs(false, false, false, false, false);
    bare.step();
}

/// What one stepped frame showed and said.
struct Seen {
    t: u32,
    /// The side that landed a hit, and whether it was chip.
    landed: Option<(Side, bool)>,
    /// A side was knocked out this frame for the first time this round.
    knocked_out: Option<Side>,
    /// The call-out lines the announcer was asked for.
    called: Vec<AnnouncerLine>,
    /// The words each side's column shows after the frame.
    words: [Vec<CalloutKind>; 2],
}

/// Whether a word showing on the board crosses a drawn ROM text cell: the
/// text layer's cells under each word's box, any pen not transparent.
fn word_over_rom_text(s: &GameSession, a: &Assets, kind: CalloutKind) -> bool {
    let vr = &s.fight_machine().unwrap().state.videoram;
    s.fight().unwrap().callouts().frame().elements().iter().any(|e| {
        let Part::Word(n) = e.part else { return false };
        if s.fight().unwrap().callouts().frame().side(e.side).words[n] != Some(kind) {
            return false;
        }
        (0..32usize).any(|row| {
            (0..64usize).any(|col| {
                let (x, y) = ((col * 8) as i32 - VISAREA_X as i32, (row * 8) as i32 - VISAREA_Y as i32);
                if x < 0 || y < 0 {
                    return false;
                }
                let cell = regions::Rect::new(x as usize, y as usize, 8, 8);
                if !cell.intersects(&e.rect) {
                    return false;
                }
                let i = row * 64 + col;
                let code = u16::from_be_bytes([vr[2 * i], vr[2 * i + 1]]) & 0x3ff;
                decode_char_tile(&a.gfx4, code as usize).iter().flatten().any(|&pen| pen != 3)
            })
        })
    })
}

/// Step `s` with `script` for at most `frames` frames or until the fight
/// screen is left, beside a bare machine given the same inputs (the work
/// RAM must match after every frame), and with every shown PERFECT checked
/// against the ROM's text.
fn play(s: &mut GameSession, a: &Assets, frames: u32, mut script: impl FnMut(u32, &GameSession) -> FrameInputs) -> Vec<Seen> {
    let mut bare = s.fight_machine().unwrap().clone();
    let mut seen = Vec::new();
    let mut out_before = [false; 2];
    for t in 0..frames {
        let input = script(t, s);
        let out = s.step(input);
        step_bare(&mut bare, input);
        assert!(s.fight_machine().unwrap().work_image() == bare.work_image(), "frame {t}: the call-outs changed RAM");
        let fight = s.fight().unwrap();
        let read = fight.fight_read();
        let landed = Side::BOTH.into_iter().find_map(|side| read.landed_by(side).map(|h| (side, h.chip)));
        let out_now = read.fighters.map(|f| f.knocked_out());
        let knocked_out = Side::BOTH.into_iter().find(|side| out_now[side.index()] && !out_before[side.index()]);
        out_before = out_now;
        let board = fight.callouts().frame();
        let words = Side::BOTH.map(|side| board.side(side).words.iter().flatten().copied().collect());
        assert!(!word_over_rom_text(s, a, CalloutKind::Perfect), "frame {t}: PERFECT crosses the ROM's text");
        seen.push(Seen { t, landed, knocked_out, called: out.announcements, words });
        if s.screen() != "fight" {
            break;
        }
    }
    seen
}

/// The frames on which `line` was called.
fn called(seen: &[Seen], line: AnnouncerLine) -> Vec<u32> {
    seen.iter().filter(|f| f.called.contains(&line)).map(|f| f.t).collect()
}

/// The frame of each round's first clean hit, by a round break: a knockout
/// ends a round.
fn first_clean_hits(seen: &[Seen]) -> Vec<(u32, Side)> {
    let mut out = Vec::new();
    let mut open = true;
    for f in seen {
        if let Some((side, false)) = f.landed {
            if open {
                out.push((f.t, side));
                open = false;
            }
        }
        if f.knocked_out.is_some() {
            open = true;
        }
    }
    out
}

/// The frames a word newly showed on `side`.
fn arrivals(seen: &[Seen], side: Side, kind: CalloutKind) -> Vec<u32> {
    let count = |f: &Seen| f.words[side.index()].iter().filter(|&&k| k == kind).count();
    seen.windows(2).filter(|w| count(&w[1]) > count(&w[0])).map(|w| w[1].t).collect()
}

#[test]
fn a_local_match_won_at_full_health_earns_first_attack_and_perfect_each_round() {
    let Some(a) = assets() else { return };
    let font = load_assets(ROMS, MANIFEST).unwrap();
    let mut s = local_fight(a, RulesetPreset::DefinitiveOverhaul);
    let seen = play(&mut s, &font, 4_000, |t, _| pressing(Side::P1, t));
    let hits = first_clean_hits(&seen);
    let kos: Vec<(u32, Side)> = seen.iter().filter_map(|f| f.knocked_out.map(|side| (f.t, side))).collect();
    assert_eq!(kos.len(), 2, "P1 wins two rounds: {kos:?}");
    assert!(kos.iter().all(|&(_, side)| side == Side::P2));
    let landed = seen.iter().filter(|f| f.landed.is_some()).count();
    assert!(landed >= 10, "each round takes several hits ({landed})");
    assert_eq!(hits.len(), 2, "a first clean hit each round");
    // FIRST ATTACK on each round's first clean hit only, on the attacker's side.
    let first: Vec<u32> = hits.iter().map(|h| h.0).collect();
    assert_eq!(arrivals(&seen, Side::P1, CalloutKind::FirstAttack), first);
    assert_eq!(called(&seen, AnnouncerLine::FirstAttack), first, "the announcer calls it");
    // PERFECT on each knockout, on the winner's side.
    let ko: Vec<u32> = kos.iter().map(|k| k.0).collect();
    assert_eq!(arrivals(&seen, Side::P1, CalloutKind::Perfect), ko);
    assert_eq!(called(&seen, AnnouncerLine::Perfect), ko, "the announcer calls it");
    // Nothing on the side that never landed a hit, and nothing else called.
    assert!(seen.iter().all(|f| f.words[1].is_empty()));
    assert!(seen.iter().flat_map(|f| &f.called).all(|l| matches!(l, AnnouncerLine::FirstAttack | AnnouncerLine::Perfect)));
}

#[test]
fn p2_earns_them_on_its_own_side() {
    let Some(a) = assets() else { return };
    let font = load_assets(ROMS, MANIFEST).unwrap();
    let mut s = local_fight(a, RulesetPreset::DefinitiveOverhaul);
    let seen = play(&mut s, &font, 1_000, |t, _| pressing(Side::P2, t));
    let ko = seen.iter().find(|f| f.knocked_out.is_some()).expect("P2 wins the round");
    assert_eq!(ko.knocked_out, Some(Side::P1));
    let hits = first_clean_hits(&seen);
    assert_eq!(hits[0].1, Side::P2);
    assert_eq!(arrivals(&seen, Side::P2, CalloutKind::FirstAttack)[0], hits[0].0);
    assert_eq!(arrivals(&seen, Side::P2, CalloutKind::Perfect)[0], ko.t);
    assert!(seen.iter().all(|f| f.words[0].is_empty()), "nothing on P1's side");
    assert!(seen.len() as u32 > ko.t + sf_video::callouts::WORD_FRAMES as u32, "PERFECT's time on screen is checked");
}

#[test]
fn chip_on_a_guard_is_not_the_first_attack() {
    let Some(a) = assets() else { return };
    let font = load_assets(ROMS, MANIFEST).unwrap();
    let mut s = local_fight(a, RulesetPreset::DefinitiveOverhaul);
    // Ken holds back (his back is right) from the start, so he backs to the
    // edge and guards; Ryu follows and throws a Tatsumaki (down, down-back,
    // back with medium kick) into the guard. Then Ken stands still and Ryu
    // punches him.
    const K: u32 = 150;
    let seen = play(&mut s, &font, 600, |t, _| {
        with(|i| {
            i.p2.directions[RIGHT] = t < K + 60;
            if t < K {
                i.p1.directions[RIGHT] = true;
            } else if t < K + 3 {
                i.p1.directions[DOWN] = true;
            } else if t < K + 6 {
                i.p1.directions[DOWN] = true;
                i.p1.directions[LEFT] = true;
            } else if t < K + 28 {
                i.p1.directions[LEFT] = true;
                i.p1.buttons[MK] = (K + 9..K + 11).contains(&t);
            } else if t > K + 120 {
                let c = t % 50;
                i.p1.directions[RIGHT] = c < 20;
                i.p1.buttons[HP] = (30..32).contains(&c);
            }
        })
    });
    let chip: Vec<u32> = seen.iter().filter(|f| f.landed == Some((Side::P1, true))).map(|f| f.t).collect();
    assert!(!chip.is_empty(), "the Tatsumaki chips Ken's guard");
    let clean = seen.iter().find(|f| f.landed == Some((Side::P1, false))).expect("a clean hit follows");
    assert!(clean.t > chip[0]);
    assert_eq!(arrivals(&seen, Side::P1, CalloutKind::FirstAttack), vec![clean.t], "the first clean hit, not the chip");
    assert_eq!(called(&seen, AnnouncerLine::FirstAttack), vec![clean.t]);
}

#[test]
fn a_round_won_after_taking_damage_is_not_perfect() {
    let Some(a) = assets() else { return };
    let font = load_assets(ROMS, MANIFEST).unwrap();
    let mut s = local_fight(a, RulesetPreset::DefinitiveOverhaul);
    // Ken lands one hit on a Ryu standing still; then Ken stands still and
    // Ryu wins the round.
    let mut hurt = None;
    let seen = play(&mut s, &font, 2_000, |t, s| {
        let health = s.fight().unwrap().fight_read().fighter(Side::P1).health;
        if health < 48 {
            hurt.get_or_insert(t);
        }
        match hurt {
            None => pressing(Side::P2, t),
            Some(h) => pressing(Side::P1, t - h + 40),
        }
    });
    let ko = seen.iter().find(|f| f.knocked_out.is_some()).expect("the round ends");
    assert_eq!(ko.knocked_out, Some(Side::P2), "Ryu wins");
    // The round, and the round end while a PERFECT would show; not round 2.
    let end = ko.t + 2 * sf_video::callouts::WORD_FRAMES as u32;
    let seen: Vec<Seen> = seen.into_iter().filter(|f| f.t <= end).collect();
    assert_eq!(seen.last().unwrap().t, end);
    let hits = first_clean_hits(&seen);
    assert_eq!(hits[0].1, Side::P2, "Ken's hit is the first");
    assert_eq!(arrivals(&seen, Side::P2, CalloutKind::FirstAttack), vec![hits[0].0]);
    assert!(arrivals(&seen, Side::P1, CalloutKind::FirstAttack).is_empty(), "Ryu's hits came later");
    assert!(arrivals(&seen, Side::P1, CalloutKind::Perfect).is_empty(), "Ryu took damage");
    assert!(called(&seen, AnnouncerLine::Perfect).is_empty());
}

#[test]
fn restart_round_earns_first_attack_again() {
    let Some(a) = assets() else { return };
    let font = load_assets(ROMS, MANIFEST).unwrap();
    let mut s = local_fight(a, RulesetPreset::DefinitiveOverhaul);
    // Several hits in: only the first earns it.
    let hitting = |s: &mut GameSession| play(s, &font, 300, |t, _| pressing(Side::P1, t));
    let seen = hitting(&mut s);
    let first = first_clean_hits(&seen);
    assert_eq!(arrivals(&seen, Side::P1, CalloutKind::FirstAttack), vec![first[0].0]);
    assert!(seen.iter().filter(|f| f.landed.is_some()).count() >= 2, "more hits land");
    // The pause box's RESTART ROUND puts the machine back to the round's start.
    let row = PauseBox::open(false).items().iter().position(|&i| i == MenuItem::RestartRound).unwrap();
    assert_eq!(s.step(start()).transition, Some(Transition::Paused));
    s.step(idle());
    for _ in 0..row {
        tap(&mut s, with(|i| i.p1.directions[DOWN] = true));
    }
    assert_eq!(s.step(confirm()).transition, Some(Transition::Choose(MenuItem::RestartRound)));
    s.step(idle());
    let seen = hitting(&mut s);
    let again = first_clean_hits(&seen);
    assert_eq!(called(&seen, AnnouncerLine::FirstAttack), vec![again[0].0], "earned again after the restart");
}

#[test]
fn a_cpu_side_earns_first_attack_in_arcade() {
    let Some(a) = assets() else { return };
    let font = load_assets(ROMS, MANIFEST).unwrap();
    let mut s = arcade(a);
    // Ryu stands still; Retsu comes in and hits him.
    let seen = play(&mut s, &font, 600, |_, _| idle());
    let hits = first_clean_hits(&seen);
    assert_eq!(hits.first().map(|h| h.1), Some(Side::P2), "the CPU lands the first hit");
    assert_eq!(arrivals(&seen, Side::P2, CalloutKind::FirstAttack), vec![hits[0].0]);
    assert_eq!(called(&seen, AnnouncerLine::FirstAttack), vec![hits[0].0]);
}

#[test]
fn the_faithful_1987_preset_shows_and_calls_none_of_them() {
    let Some(a) = assets() else { return };
    let font = load_assets(ROMS, MANIFEST).unwrap();
    let mut s = local_fight(a, RulesetPreset::Faithful1987);
    let seen = play(&mut s, &font, 1_000, |t, _| pressing(Side::P1, t));
    assert!(seen.iter().any(|f| f.landed.is_some()), "hits land");
    assert!(seen.iter().any(|f| f.knocked_out.is_some()), "the round is won at full health");
    for f in &seen {
        assert!(f.called.is_empty() && f.words.iter().all(|w| w.is_empty()), "frame {}: none under the 1987 preset", f.t);
    }
}

// ---- REVERSAL ---------------------------------------------------------------

const STANDING: FighterRead =
    FighterRead { present: true, health: 48, move_id: 0x01, state: 0x12, blockstun: false, getting_up: false, attack_active: false };
/// A heavy punch out.
const PUNCH: FighterRead = FighterRead { move_id: 0x10, state: 0xc2, ..STANDING };
const BLOCKSTUN: FighterRead = FighterRead { move_id: 0x04, state: 0xa2, blockstun: true, ..STANDING };
const GETTING_UP: FighterRead = FighterRead { move_id: 0x01, state: 0x80, getting_up: true, ..STANDING };

/// Reads of P1 (`me`, one a frame) against a P2 standing still; `hit_at` and
/// `block_at` are the frames P1's attack hits or is blocked.
fn reads(me: &[FighterRead], hit_at: Option<usize>, block_at: Option<usize>) -> Vec<FightRead> {
    me.iter()
        .enumerate()
        .map(|(t, &f)| {
            let mut read = FightRead { fighters: [f, STANDING], ..FightRead::default() };
            if hit_at == Some(t) {
                read.landed[0] = Some(LandedHit { damage: 4, into_hitstun: false, chip: false });
            }
            read.blocked[0] = block_at == Some(t);
            read
        })
        .collect()
}

fn frames(parts: &[(FighterRead, usize)]) -> Vec<FighterRead> {
    parts.iter().flat_map(|&(f, n)| std::iter::repeat_n(f, n)).collect()
}

/// Feed `script` to a fresh Local fight under `preset`: the frames REVERSAL
/// arrived on P1's side and was called on, and whether P2 ever showed one.
fn fed(preset: RulesetPreset, script: &[FightRead]) -> (Vec<usize>, Vec<usize>, bool) {
    let mut s = local_fight(assets().unwrap(), preset);
    s.feed_fight_reads(script);
    let (mut shown, mut said, mut other) = (Vec::new(), Vec::new(), false);
    let mut before = 0;
    for t in 0..script.len() + 2 {
        let out = s.step(idle());
        let board = s.fight().unwrap().callouts().frame();
        let now = board.side(Side::P1).words.iter().filter(|w| **w == Some(CalloutKind::Reversal)).count();
        if now > before {
            shown.push(t);
        }
        before = now;
        if out.announcements.contains(&AnnouncerLine::Reversal) {
            said.push(t);
        }
        other |= board.side(Side::P2).words.contains(&Some(CalloutKind::Reversal));
    }
    (shown, said, other)
}

#[test]
fn reversal_out_of_blockstun_and_out_of_a_get_up_is_called_when_it_connects() {
    if assets().is_none() {
        return;
    }
    // Out of blockstun: the punch starts the frame blockstun ends (5), hits at 9.
    let script = reads(&frames(&[(STANDING, 2), (BLOCKSTUN, 3), (PUNCH, 8), (STANDING, 4)]), Some(9), None);
    assert_eq!(fed(RulesetPreset::DefinitiveOverhaul, &script), (vec![9], vec![9], false));
    // Out of a get-up, and blocked at 7.
    let script = reads(&frames(&[(GETTING_UP, 4), (PUNCH, 8), (STANDING, 4)]), None, Some(7));
    assert_eq!(fed(RulesetPreset::DefinitiveOverhaul, &script), (vec![7], vec![7], false));
}

#[test]
fn a_reversal_one_frame_late_or_that_never_connects_is_not_called() {
    if assets().is_none() {
        return;
    }
    let none = (vec![], vec![], false);
    // One free frame between the lock and the punch.
    let script = reads(&frames(&[(BLOCKSTUN, 3), (STANDING, 1), (PUNCH, 8), (STANDING, 4)]), Some(8), None);
    assert_eq!(fed(RulesetPreset::DefinitiveOverhaul, &script), none, "a frame late out of blockstun");
    let script = reads(&frames(&[(GETTING_UP, 3), (STANDING, 1), (PUNCH, 8)]), None, Some(6));
    assert_eq!(fed(RulesetPreset::DefinitiveOverhaul, &script), none, "a frame late out of a get-up");
    // The punch out of blockstun whiffs; a later punch from neutral hits.
    let script =
        reads(&frames(&[(BLOCKSTUN, 3), (PUNCH, 8), (STANDING, 4), (PUNCH, 8), (STANDING, 2)]), Some(18), None);
    assert_eq!(fed(RulesetPreset::DefinitiveOverhaul, &script), none, "it never connected");
    // A hit reaction is neither blockstun nor a get-up.
    let reeling = FighterRead { move_id: 0x33, state: 0x89, ..STANDING };
    let script = reads(&frames(&[(reeling, 3), (PUNCH, 8), (STANDING, 2)]), Some(5), None);
    assert_eq!(fed(RulesetPreset::DefinitiveOverhaul, &script), none, "out of a hit reaction");
}

#[test]
fn the_faithful_1987_preset_shows_no_reversal() {
    if assets().is_none() {
        return;
    }
    let script = reads(&frames(&[(BLOCKSTUN, 3), (PUNCH, 8), (STANDING, 4)]), Some(6), None);
    assert_eq!(fed(RulesetPreset::Faithful1987, &script), (vec![], vec![], false));
}

#[test]
fn a_real_fight_earns_no_reversal_out_of_chip_locks_or_hit_reactions() {
    let Some(a) = assets() else { return };
    let font = load_assets(ROMS, MANIFEST).unwrap();
    let mut s = local_fight(a, RulesetPreset::DefinitiveOverhaul);
    // Ryu throws a Tatsumaki into Ken's guard, then punches him; Ken mashes
    // medium punch every other frame, so each time his chip lock or hit
    // reaction ends he attacks at once.
    const K: u32 = 150;
    let mut out_of_lock = 0;
    let mut was_reacting = false;
    let seen = play(&mut s, &font, 900, |t, s| {
        let read = s.fight().unwrap().fight_read();
        assert!(read.fighters.iter().all(|f| !f.blockstun && !f.getting_up), "frame {t}: the 1987 reader reads no lock");
        let ken = *read.fighter(Side::P2);
        if was_reacting && !ken.reacting() && ken.attacking() {
            out_of_lock += 1;
        }
        was_reacting = ken.reacting();
        with(|i| {
            i.p2.directions[RIGHT] = t < K + 60;
            // From inside the chip lock (the chip lands at about K + 41).
            i.p2.buttons[1] = t > K + 45 && t % 2 == 0;
            if t < K {
                i.p1.directions[RIGHT] = true;
            } else if t < K + 3 {
                i.p1.directions[DOWN] = true;
            } else if t < K + 6 {
                i.p1.directions[DOWN] = true;
                i.p1.directions[LEFT] = true;
            } else if t < K + 28 {
                i.p1.directions[LEFT] = true;
                i.p1.buttons[MK] = (K + 9..K + 11).contains(&t);
            } else if t > K + 60 {
                let c = t % 50;
                i.p1.directions[RIGHT] = c < 20;
                i.p1.buttons[HP] = (30..32).contains(&c);
            }
        })
    });
    assert!(seen.iter().any(|f| f.landed == Some((Side::P1, true))), "a chip lock");
    assert!(seen.iter().filter(|f| f.landed.is_some()).count() >= 3, "hit reactions");
    assert!(out_of_lock >= 1, "Ken attacks straight out of a lock ({out_of_lock})");
    for f in &seen {
        assert!(f.words.iter().flatten().all(|&k| k != CalloutKind::Reversal), "frame {}: no REVERSAL", f.t);
        assert!(!f.called.contains(&AnnouncerLine::Reversal), "frame {}: none called", f.t);
    }
}
