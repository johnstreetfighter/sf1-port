//! Ticket #262 (`SP18`): cancels, the second of the modern rules.
//!
//! At the headless session's seam (inputs in; the presented frames and the
//! fight reader out), in Local fights: with the cancels switch on
//! (Definitive Overhaul), a normal the move data marks cancelable
//! (`sf_engine::combo::cancel::window`) that connects, hit or guarded, opens
//! a window in which a special's command cancels it into the special: the
//! special starts while the normal would still be running. A whiff opens no
//! window; a press after the window closes, or on a normal not marked
//! cancelable, cancels nothing; with the switch off nothing cancels and the
//! cancel state stays untouched. Ryu, Ken, Retsu and Geki follow the same
//! rule. A cancel that lands in the normal's hitstun is a combo and the era
//! HUD counts it, and a cancel replays the same from a restored snapshot.
//! Faithful 1987's checks are the rest of the suite's (its MAME checks and
//! frame goldens). Local ROMs only (Retsu and Geki: and the local art
//! pack); each test skips without them.

#[path = "support/select.rs"]
mod select;

use std::path::Path;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::fight_reader::{FighterRead, LandedHit};
use sf_engine::fighter::{P1_STRUCT, P2_STRUCT};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::{FrameInputs, FrameOutput, GameSession, Transition};
use sf_engine::roster::TournamentFighter;
use sf_video::art::{load_art_from, ArtSources};
use sf_video::art_pack::ArtPack;
use sf_video::callouts::{regions, ComboCount, Side};
use sf_video::render::WIDTH;
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const PACK: &str = "../../tools/reference/local/roms/sf1-port-art";
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
const FIGHT_LOOP: u32 = 0x3eafa;
const RIGHT: usize = 0;
const LEFT: usize = 1;
const DOWN: usize = 2;
const LP: usize = 0;
const MP: usize = 1;
const HP: usize = 2;
const LK: usize = 3;
const MK: usize = 4;
const HK: usize = 5;
const MODERN: RulesetPreset = RulesetPreset::DefinitiveOverhaul;
const CLASSIC: RulesetPreset = RulesetPreset::ClassicCompetitive;
/// The specials' move ids: the Shoryuken's slot, the Hadoken's (Retsu's
/// Kiai Shot, Geki's Shuriken).
const SHORYUKEN: u8 = 0x19;
const HADOKEN: u8 = 0x1a;
/// Ken's guards, facing left: standing (back) and crouching (down-back).
const STAND_GUARD: &[usize] = &[RIGHT];
const CROUCH_GUARD: &[usize] = &[DOWN, RIGHT];

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

fn until(s: &mut GameSession, name: &str, limit: u32) {
    for _ in 0..limit {
        if s.step(idle()).screen == name {
            return;
        }
    }
    panic!("never reached {name} in {limit} frames (at {})", s.screen());
}

/// Through the round's intro to where the fighters take control.
fn to_control(s: &mut GameSession) {
    let mut f = 0;
    while s.fight_machine().map(flow_position) != Some(FIGHT_LOOP) {
        s.step(idle());
        f += 1;
        assert!(f < 3_000, "the fight loop (at {})", s.screen());
    }
    for _ in 0..240 {
        s.step(idle());
    }
}

/// Local: P1 `p1` against P2 `p2`, Japan, at control.
fn local(mut s: GameSession, p1: TournamentFighter, p2: TournamentFighter) -> GameSession {
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.menu.start = true));
    tap(&mut s, with(|i| i.p1.directions[DOWN] = true));
    tap(&mut s, with(|i| i.menu.confirm = true)); // Multi Player
    tap(&mut s, with(|i| i.menu.confirm = true)); // Local
    let at = TournamentFighter::ALL
        .iter()
        .position(|&f| f == p1)
        .unwrap();
    if at >= 6 {
        tap(&mut s, with(|i| i.p1.directions[DOWN] = true));
    }
    for _ in 0..(at % 6) {
        tap(&mut s, with(|i| i.p1.directions[RIGHT] = true));
    }
    assert_eq!(
        select::confirm_fighter(&mut s, with(|i| i.menu.confirm = true), tap).transition,
        Some(Transition::Picked(p1))
    );
    // P2's cursor starts on Ken (row 0, col 1).
    let at2 = TournamentFighter::ALL
        .iter()
        .position(|&f| f == p2)
        .unwrap();
    if at2 >= 6 {
        tap(&mut s, with(|i| i.p2.directions[DOWN] = true));
    }
    let col2 = at2 % 6;
    if col2 == 0 {
        tap(&mut s, with(|i| i.p2.directions[LEFT] = true));
    }
    for _ in 1..col2 {
        tap(&mut s, with(|i| i.p2.directions[RIGHT] = true));
    }
    let out = select::confirm_fighter(&mut s, with(|i| i.p2.buttons[0] = true), GameSession::step);
    assert_eq!(out.transition, Some(Transition::Fighters(p1, p2)));
    s.step(idle());
    until(&mut s, "country select", 40);
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    until(&mut s, "fight", 40);
    to_control(&mut s);
    s
}

fn shotos(
    config: MatchConfig,
    p1: TournamentFighter,
    p2: TournamentFighter,
) -> Option<GameSession> {
    let a = assets()?;
    Some(local(GameSession::new(a, config), p1, p2))
}

fn ryu_ken(config: MatchConfig) -> Option<GameSession> {
    shotos(config, TournamentFighter::Ryu, TournamentFighter::Ken)
}

/// `p1` (Retsu or Geki, on his bank table) against Ken, Definitive
/// Overhaul.
fn boss(p1: TournamentFighter) -> Option<GameSession> {
    let a = assets()?;
    if !Path::new(PACK).is_dir() {
        eprintln!("skipping: the local art pack is not present");
        return None;
    }
    let pack = ArtPack::open(Path::new(PACK)).expect("the art pack opens");
    let art = load_art_from(
        &a,
        &ArtSources {
            recipes: &RECIPES,
            reviews: Some(&REVIEWS),
            pack: Ok(&pack),
        },
    )
    .expect("the art loads");
    let s = GameSession::new(a, MatchConfig::from_preset(MODERN))
        .with_sprite_bank(&art.bank)
        .with_player_tables(&art.catalogues);
    let s = local(s, p1, TournamentFighter::Ken);
    assert!(
        s.data_bank().unwrap().table(p1).is_some(),
        "{p1:?} plays his bank table"
    );
    Some(s)
}

/// A side's fighter entity.
fn entity(s: &GameSession, ps: u32) -> usize {
    let w = s.fight_machine().unwrap().work_ram();
    let a = (ps + 0x10 - 0xff8000) as usize;
    u16::from_be_bytes([w[a], w[a + 1]]) as usize
}

/// An entity's horizontal centre (`0x43be8`).
fn centre_x(s: &GameSession, e: usize) -> i32 {
    let o = &s.fight_machine().unwrap().state.objectram;
    let x = u16::from_be_bytes([o[e + 6], o[e + 7]]) as i32;
    let big = i16::from_be_bytes([o[e + 2], o[e + 3]]) >= 0x400;
    let cell = if big { 32 } else { 16 };
    x + ((o[e + 0xd] as i32 + 1) * cell) / 2
}

fn gap(s: &GameSession) -> i32 {
    centre_x(s, entity(s, P2_STRUCT)) - centre_x(s, entity(s, P1_STRUCT))
}

/// Walk P1 right until the fighters' centres are at most `want` apart.
fn close_in(s: &mut GameSession, want: i32) {
    for _ in 0..300 {
        if gap(s) <= want {
            for _ in 0..4 {
                s.step(idle());
            }
            return;
        }
        s.step(with(|i| i.p1.directions[RIGHT] = true));
    }
    panic!("never closed in (gap {})", gap(s));
}

/// What one frame showed.
#[derive(Debug, Clone, Copy)]
struct Frame {
    p1: FighterRead,
    landed: Option<LandedHit>,
    combo: Option<ComboCount>,
    /// P1's cancel window (frames left) after the frame.
    window: u8,
    /// A hash of the presented frame's pixels in P1's combo slot.
    slot: u64,
}

fn step(s: &mut GameSession, input: FrameInputs) -> Frame {
    let out = s.step(input);
    let r = regions::combo_slot(Side::P1);
    let mut slot = 0xcbf2_9ce4_8422_2325u64;
    for (i, px) in out.frame.argb.iter().enumerate() {
        if r.contains(i % WIDTH, i / WIDTH) {
            slot = (slot ^ u64::from(*px)).wrapping_mul(0x100_0000_01b3);
        }
    }
    let f = s.fight().unwrap();
    let read = f.fight_read();
    Frame {
        p1: read.fighters[0],
        landed: read.landed_by(Side::P1),
        combo: f.callouts().frame().side(Side::P1).combo,
        window: s.fight_machine().unwrap().toggle_state().cancel.window[0],
        slot,
    }
}

/// A normal, a special's motion made during it, and the special's button.
#[derive(Clone, Copy)]
struct Try<'a> {
    /// The normal: the directions held and the button pressed on frame 0.
    normal: (&'a [usize], usize),
    /// The directions held on frames 1, 2, ...
    motion: &'a [&'a [usize]],
    /// The special's last direction, held from the motion's end through
    /// the press, and its button.
    last: &'a [usize],
    button: usize,
}

/// Crouching (from down) into the Hadoken's command: down, down-forward,
/// forward, and a punch.
const QCF: &[&[usize]] = &[&[DOWN], &[DOWN], &[DOWN, RIGHT], &[DOWN, RIGHT]];

fn qcf_from(normal: (&'static [usize], usize)) -> Try<'static> {
    Try {
        normal,
        motion: QCF,
        last: &[RIGHT],
        button: LP,
    }
}

/// The Shoryuken's command from a crouch (lenient: forward, down,
/// down-forward), and a punch.
const DP: Try<'static> = Try {
    normal: (&[DOWN], LK),
    motion: &[&[RIGHT], &[DOWN], &[DOWN]],
    last: &[DOWN, RIGHT],
    button: LP,
};

/// Run `t` from `s` with the special's button pressed on frame `press`
/// (`None`: never), P2 holding `p2`, for `frames` frames. After the press P1
/// lets go.
fn run(s: &GameSession, t: Try, press: Option<usize>, p2: &[usize], frames: usize) -> Vec<Frame> {
    run_on(&mut s.clone(), t, press, p2, frames)
}

/// [`run`] on `s` itself.
fn run_on(
    s: &mut GameSession,
    t: Try,
    press: Option<usize>,
    p2: &[usize],
    frames: usize,
) -> Vec<Frame> {
    (0..frames)
        .map(|n| {
            let (held, button): (&[usize], Option<usize>) = if n == 0 {
                (t.normal.0, Some(t.normal.1))
            } else if n <= t.motion.len() {
                (t.motion[n - 1], None)
            } else if press.is_none_or(|p| n <= p) {
                (t.last, (press == Some(n)).then_some(t.button))
            } else {
                (&[], None)
            };
            let input = with(|i| {
                held.iter().for_each(|&d| i.p1.directions[d] = true);
                p2.iter().for_each(|&d| i.p2.directions[d] = true);
                if let Some(b) = button {
                    i.p1.buttons[b] = true;
                }
            });
            step(s, input)
        })
        .collect()
}

/// What a try came to.
#[derive(Debug)]
struct Outcome {
    /// The frame the normal connected on (its window opened).
    contact: Option<usize>,
    /// The first frame P1 was in the special.
    special: Option<usize>,
    /// The first frame the normal, run on its own, was over.
    end: usize,
}

impl Outcome {
    fn cancelled(&self) -> bool {
        matches!((self.contact, self.special), (Some(c), Some(sp)) if c < sp && sp < self.end)
    }
}

/// The frame `t`'s normal connects on, run without the special's button.
fn contact(s: &GameSession, t: Try, p2: &[usize]) -> usize {
    run(s, t, None, p2, 40)
        .iter()
        .position(|f| f.window > 0)
        .expect("the normal connects")
}

fn outcome(
    s: &GameSession,
    t: Try,
    press: usize,
    p2: &[usize],
    special: u8,
) -> (Outcome, Vec<Frame>) {
    let frames = run(s, t, Some(press), p2, 70);
    let alone = run(s, t, None, p2, 70);
    let id = alone[0].p1.move_id;
    let end = alone
        .iter()
        .position(|f| f.p1.move_id != id)
        .expect("the normal ends");
    let contact = frames.iter().position(|f| f.window > 0);
    let sp = frames.iter().position(|f| f.p1.move_id == special);
    (
        Outcome {
            contact,
            special: sp,
            end,
        },
        frames,
    )
}

// ---- the tests ---------------------------------------------------------------

#[test]
fn a_cancelable_normal_cancels_into_a_special_on_hit_and_on_block() {
    let Some(mut s) = ryu_ken(MatchConfig::from_preset(MODERN)) else {
        return;
    };
    close_in(&mut s, 0x30);
    let crouching_short = qcf_from((&[DOWN], LK));
    for (guard, what) in [(&[][..], "on hit"), (CROUCH_GUARD, "on block")] {
        // The crouching short connects on frame 6; the Hadoken's punch two
        // frames later cancels it on the spot.
        let (o, frames) = outcome(&s, crouching_short, 8, guard, HADOKEN);
        assert_eq!(frames[0].p1.move_id, 0x17, "the crouching short");
        assert!(
            o.cancelled(),
            "{what}: the Hadoken cancels the crouching short: {o:?}"
        );
        assert_eq!(
            o.special,
            Some(8),
            "{what}: on the frame its button is pressed"
        );
        let c = o.contact.unwrap();
        assert_eq!(
            frames[c].window,
            sf_engine::combo::cancel::WINDOW,
            "{what}: the window opens on the contact"
        );
        assert_eq!(frames[c].landed.is_some(), guard.is_empty(), "{what}");
        // The Shoryuken's command cancels it too.
        let (o, _) = outcome(&s, DP, 8, guard, SHORYUKEN);
        assert!(
            o.cancelled(),
            "{what}: the Shoryuken cancels the crouching short: {o:?}"
        );
    }
    // The special's button pressed a frame or two before the contact waits
    // in the recovery buffer and cancels on the window's first frame.
    let (o, _) = outcome(&s, crouching_short, 5, &[], HADOKEN);
    assert!(o.cancelled(), "a buffered press cancels: {o:?}");
    assert_eq!(
        o.special,
        Some(o.contact.unwrap() + 1),
        "on the window's first frame"
    );
    // Other cancelable normals: the standing medium punch, the crouching
    // jab and the crouching medium kick, two frames after their contact.
    for (normal, name) in [
        ((&[][..], MP), "medium punch"),
        ((&[DOWN][..], LP), "crouching jab"),
        ((&[DOWN][..], MK), "crouching medium kick"),
    ] {
        let t = qcf_from(normal);
        let c = contact(&s, t, &[]);
        let (o, _) = outcome(&s, t, c.max(4) + 2, &[], HADOKEN);
        assert!(o.cancelled(), "the {name} cancels: {o:?}");
    }
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());
}

#[test]
fn a_whiffed_normal_opens_no_window_and_does_not_cancel() {
    let Some(s) = ryu_ken(MatchConfig::from_preset(MODERN)) else {
        return;
    };
    // At the round's starting distance the crouching short meets nothing.
    let (o, frames) = outcome(&s, qcf_from((&[DOWN], LK)), 8, &[], HADOKEN);
    assert!(
        frames.iter().all(|f| f.window == 0 && f.landed.is_none()),
        "no contact, no window"
    );
    assert!(
        o.special.is_none_or(|sp| sp >= o.end),
        "nothing starts until the normal ends: {o:?}"
    );
}

#[test]
fn outside_the_window_nothing_cancels() {
    let Some(mut s) = boss(TournamentFighter::Geki) else {
        return;
    };
    close_in(&mut s, 0x28);
    // Geki's crouching jab recovers long after its contact: its window
    // closes 12 frames after it, with the jab still running.
    let crouching_jab = qcf_from((&[DOWN], LP));
    let (inside, frames) = outcome(&s, crouching_jab, 12, &[], HADOKEN);
    let c = inside.contact.expect("the crouching jab hits Ken");
    assert!(
        inside.cancelled(),
        "inside the window the Shuriken cancels it: {inside:?}"
    );
    let alone = run(&s, crouching_jab, None, &[], 70);
    let closed = c + usize::from(sf_engine::combo::cancel::WINDOW);
    assert_eq!(
        alone[closed].window,
        0,
        "the window is over {} frames after the contact",
        sf_engine::combo::cancel::WINDOW
    );
    assert_eq!(alone[closed - 1].window, 1);
    assert!(
        closed + 2 < inside.end,
        "the jab runs on after its window: {inside:?}"
    );
    assert_eq!(frames[c].p1.move_id, 0x11);
    // Pressed after the window, with the command done: no cancel; the jab
    // runs its course and the press, too old for the recovery buffer by
    // then, is gone.
    let (late, _) = outcome(&s, crouching_jab, closed + 1, &[], HADOKEN);
    assert!(
        !late.cancelled() && late.special.is_none_or(|sp| sp >= late.end),
        "after the window: {late:?}"
    );
    // Pressed long before the contact (older than the buffer's 4 frames
    // when the window opens): no cancel either.
    let (early, _) = outcome(&s, crouching_jab, 5, &[], HADOKEN);
    assert!(
        early.special.is_none_or(|sp| sp >= early.end),
        "before the window: {early:?}"
    );
}

#[test]
fn a_normal_not_marked_cancelable_does_not_cancel() {
    let Some(mut s) = ryu_ken(MatchConfig::from_preset(MODERN)) else {
        return;
    };
    close_in(&mut s, 0x30);
    // The standing heavy punch and medium kick (on a standing Ken) and the
    // sweep (on a crouching one) all connect and open no window.
    for (normal, p2, name) in [
        ((&[][..], HP), &[][..], "heavy punch"),
        ((&[][..], MK), &[][..], "medium kick"),
        ((&[DOWN][..], HK), &[DOWN][..], "sweep"),
    ] {
        for press in [6, 9, 12] {
            let (o, frames) = outcome(&s, qcf_from(normal), press, p2, HADOKEN);
            assert!(frames.iter().any(|f| f.landed.is_some()), "the {name} hits");
            assert!(
                frames.iter().all(|f| f.window == 0),
                "the {name} opens no window"
            );
            assert!(
                o.special.is_none_or(|sp| sp >= o.end),
                "the {name} is not cancelled (press {press}): {o:?}"
            );
        }
    }
    // A normal's button in the window chains nothing: a second crouching
    // short pressed in the first one's window waits for its end.
    let chain = Try {
        normal: (&[DOWN], LK),
        motion: &[&[DOWN]],
        last: &[DOWN],
        button: LK,
    };
    let (o, frames) = outcome(&s, chain, 8, &[], 0xff);
    let first_end = frames
        .iter()
        .position(|f| f.p1.move_id != 0x17)
        .unwrap_or(frames.len());
    assert!(
        o.contact.is_some() && first_end >= o.end,
        "no chain into a normal: {o:?}"
    );
}

#[test]
fn with_the_switch_off_no_cancel_happens() {
    for config in [
        MatchConfig::from_preset(MODERN).with_cancels(false),
        MatchConfig::from_preset(CLASSIC),
    ] {
        let Some(mut s) = ryu_ken(config) else { return };
        assert!(!s.fight_machine().unwrap().match_config().toggles.cancels);
        close_in(&mut s, 0x30);
        for press in [5, 7, 9] {
            let (o, frames) = outcome(&s, qcf_from((&[DOWN], LK)), press, &[], HADOKEN);
            assert!(
                frames.iter().any(|f| f.landed.is_some()),
                "the crouching short hits"
            );
            assert!(
                o.special.is_none_or(|sp| sp >= o.end),
                "{:?}: not cancelled (press {press}): {o:?}",
                config.preset
            );
        }
        let mut s2 = s.clone();
        run_on(&mut s2, qcf_from((&[DOWN], LK)), Some(8), &[], 80);
        assert_eq!(
            s2.fight_machine().unwrap().toggle_state().cancel,
            Default::default(),
            "the cancel state stays untouched"
        );
    }
}

#[test]
fn ken_cancels_crouching_short_into_hadoken() {
    // Ken on P1 against Ryu: his crouching short cancels into his Hadoken.
    let Some(mut s) = shotos(
        MatchConfig::from_preset(MODERN),
        TournamentFighter::Ken,
        TournamentFighter::Ryu,
    ) else {
        return;
    };
    close_in(&mut s, 0x30);
    for guard in [&[][..], CROUCH_GUARD] {
        let (o, _) = outcome(&s, qcf_from((&[DOWN], LK)), 8, guard, HADOKEN);
        assert!(o.cancelled(), "Ken (guard {guard:?}): {o:?}");
    }
    let (o, _) = outcome(&s, qcf_from((&[], HP)), 8, &[], HADOKEN);
    assert!(
        o.special.is_none_or(|sp| sp >= o.end),
        "Ken's heavy punch is not cancelable: {o:?}"
    );
}

#[test]
fn retsu_cancels_jab_into_kiai_shot() {
    // Retsu: his jab cancels into the Kiai Shot, on hit and on Ken's
    // standing guard (a crouching Ken ducks it).
    let Some(mut s) = boss(TournamentFighter::Retsu) else {
        return;
    };
    close_in(&mut s, 0x38);
    for guard in [&[][..], STAND_GUARD] {
        let (o, frames) = outcome(&s, qcf_from((&[], LP)), 8, guard, HADOKEN);
        assert_eq!(frames[0].p1.move_id, 0x0d);
        assert!(
            o.cancelled(),
            "Retsu's jab into the Kiai Shot (guard {guard:?}): {o:?}"
        );
    }
    let (o, frames) = outcome(&s, qcf_from((&[], HP)), 8, &[], HADOKEN);
    assert!(
        frames.iter().any(|f| f.landed.is_some()),
        "his heavy punch hits"
    );
    assert!(
        o.special.is_none_or(|sp| sp >= o.end),
        "Retsu's heavy punch is not cancelable: {o:?}"
    );
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());
}

#[test]
fn geki_cancels_crouching_jab_into_shuriken() {
    // Geki: his crouching jab cancels into the Shuriken on hit and on
    // Ken's crouching guard (the window closing is `outside_the_window_*`).
    let Some(mut s) = boss(TournamentFighter::Geki) else {
        return;
    };
    close_in(&mut s, 0x28);
    for guard in [&[][..], CROUCH_GUARD] {
        let (o, frames) = outcome(&s, qcf_from((&[DOWN], LP)), 12, guard, HADOKEN);
        assert_eq!(frames[0].p1.move_id, 0x11);
        assert!(
            o.cancelled(),
            "Geki's crouching jab into the Shuriken (guard {guard:?}): {o:?}"
        );
    }
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());
}

#[test]
fn joe_cancels_crouching_jab_into_dynamite_straight() {
    // Joe: his crouching jab (`0x11`, body punch) cancels into the Dynamite
    // Straight (`0x19`) on hit and on Ken's crouching guard.
    let Some(mut s) = boss(TournamentFighter::Joe) else {
        return;
    };
    close_in(&mut s, 0x28);
    let joe_dp = Try {
        normal: (&[DOWN], LP),
        motion: &[&[RIGHT], &[DOWN], &[DOWN]],
        last: &[DOWN, RIGHT],
        button: LP,
    };
    for guard in [&[][..], CROUCH_GUARD] {
        let (o, frames) = outcome(&s, joe_dp, 12, guard, SHORYUKEN);
        assert_eq!(frames[0].p1.move_id, 0x11);
        assert!(
            o.cancelled(),
            "Joe's crouching jab into the Dynamite Straight (guard {guard:?}): {o:?}"
        );
    }
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());
}

#[test]
fn mike_cancels_crouching_jab_into_bison_rush() {
    // Mike: his crouching jab (`0x11`, ducking punch) cancels into the
    // Bison Rush (`0x1a`) on hit and on Ken's crouching guard.
    let Some(mut s) = boss(TournamentFighter::Mike) else {
        return;
    };
    close_in(&mut s, 0x30);
    for guard in [&[][..], CROUCH_GUARD] {
        let (o, frames) = outcome(&s, qcf_from((&[DOWN], LP)), 10, guard, HADOKEN);
        assert_eq!(frames[0].p1.move_id, 0x11);
        assert!(
            o.cancelled(),
            "Mike's crouching jab into the Bison Rush (guard {guard:?}): {o:?}"
        );
    }
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());
}

#[test]
fn lee_cancels_crouching_jab_into_zesshouohken() {
    // Lee: his crouching jab (`0x11`) cancels into the Zesshouohken (`0x1a`)
    // on hit and on Ken's crouching guard.
    let Some(mut s) = boss(TournamentFighter::Lee) else {
        return;
    };
    close_in(&mut s, 0x20);
    for guard in [&[][..], CROUCH_GUARD] {
        let (o, frames) = outcome(&s, qcf_from((&[DOWN], LP)), 12, guard, HADOKEN);
        assert_eq!(frames[0].p1.move_id, 0x11);
        assert!(
            o.cancelled(),
            "Lee's crouching jab into the Zesshouohken (guard {guard:?}): {o:?}"
        );
    }
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());
}

#[test]
fn gen_cancels_crouching_jab_into_hyakurenko() {
    // Gen: his crouching jab (`0x11`) cancels into the Hyakurenko (`0x1a`)
    // on hit and on Ken's crouching guard.
    let Some(mut s) = boss(TournamentFighter::Gen) else {
        return;
    };
    close_in(&mut s, 0x20);
    for guard in [&[][..], CROUCH_GUARD] {
        let (o, frames) = outcome(&s, qcf_from((&[DOWN], LP)), 12, guard, HADOKEN);
        assert_eq!(frames[0].p1.move_id, 0x11);
        assert!(
            o.cancelled(),
            "Gen's crouching jab into the Hyakurenko (guard {guard:?}): {o:?}"
        );
    }
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());
}

#[test]
fn birdie_cancels_crouching_jab_into_bull_head() {
    // Birdie: his crouching jab (`0x11`) cancels into the Bull Head (`0x1a`)
    // on hit and on Ken's crouching guard.
    let Some(mut s) = boss(TournamentFighter::Birdie) else {
        return;
    };
    close_in(&mut s, 0x38);
    for guard in [&[][..], CROUCH_GUARD] {
        let (o, frames) = outcome(&s, qcf_from((&[DOWN], LP)), 12, guard, HADOKEN);
        assert_eq!(frames[0].p1.move_id, 0x11);
        assert!(
            o.cancelled(),
            "Birdie's crouching jab into the Bull Head (guard {guard:?}): {o:?}"
        );
    }
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());
}

#[test]
fn eagle_cancels_crouching_jab_into_manchester_black() {
    // Eagle: his crouching jab (`0x11`, his 1987 low stick thrust) cancels
    // into the Manchester Black (`0x1a`) on hit and on Ken's crouching guard.
    let Some(mut s) = boss(TournamentFighter::Eagle) else {
        return;
    };
    close_in(&mut s, 0x38);
    for guard in [&[][..], CROUCH_GUARD] {
        let (o, frames) = outcome(&s, qcf_from((&[DOWN], LP)), 8, guard, HADOKEN);
        assert_eq!(frames[0].p1.move_id, 0x11);
        assert!(
            o.cancelled(),
            "Eagle's crouching jab into the Manchester Black (guard {guard:?}): {o:?}"
        );
    }
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());
}

#[test]
fn adon_cancels_crouching_jab_into_rising_jaguar() {
    // Adon: his crouching jab (`0x11`) cancels into the Rising Jaguar (`0x19`)
    // on hit and on Ken's crouching guard.
    let Some(mut s) = boss(TournamentFighter::Adon) else {
        return;
    };
    close_in(&mut s, 0x28);
    let adon_dp = Try {
        normal: (&[DOWN], LP),
        motion: &[&[RIGHT], &[DOWN], &[DOWN]],
        last: &[DOWN, RIGHT],
        button: LP,
    };
    for guard in [&[][..], CROUCH_GUARD] {
        let (o, frames) = outcome(&s, adon_dp, 12, guard, SHORYUKEN);
        assert_eq!(frames[0].p1.move_id, 0x11);
        assert!(
            o.cancelled(),
            "Adon's crouching jab into the Rising Jaguar (guard {guard:?}): {o:?}"
        );
    }
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());
}

#[test]
fn sagat_cancels_crouching_jab_into_tiger_shots_and_knee() {
    // Sagat: his crouching jab (`0x11`) cancels into the High Tiger Shot
    // (`0x1a`), the Low Tiger Shot (`0x1b`) and the Tiger Knee (`0x19`) on
    // hit and on Ken's crouching guard.
    let Some(mut s) = boss(TournamentFighter::Sagat) else {
        return;
    };
    close_in(&mut s, 0x28);
    for _ in 0..30 {
        s.step(idle());
    }
    let sagat_low_shot = Try {
        normal: (&[DOWN], LP),
        motion: QCF,
        last: &[RIGHT],
        button: LK,
    };
    let sagat_tk = Try {
        normal: (&[DOWN], LP),
        motion: &[&[RIGHT], &[DOWN], &[DOWN]],
        last: &[DOWN, RIGHT],
        button: LK,
    };
    for guard in [&[][..], CROUCH_GUARD] {
        for (what, try_spec, id) in [
            ("High Tiger Shot", qcf_from((&[DOWN], LP)), HADOKEN),
            ("Low Tiger Shot", sagat_low_shot, 0x1b),
            ("Tiger Knee", sagat_tk, SHORYUKEN),
        ] {
            let (o, frames) = outcome(&s, try_spec, 12, guard, id);
            assert_eq!(frames[0].p1.move_id, 0x11);
            assert!(
                o.cancelled(),
                "Sagat's crouching jab into the {what} (guard {guard:?}): {o:?}"
            );
        }
    }
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());
}

#[test]
fn a_cancel_combo_shows_on_the_era_hud_counter() {
    let Some(mut s) = ryu_ken(MatchConfig::from_preset(MODERN)) else {
        return;
    };
    close_in(&mut s, 0x30);
    // Crouching short into the Hadoken: the fireball lands in the short's
    // hitstun, the combo's second hit, and the HUD draws the count.
    let (o, frames) = outcome(&s, qcf_from((&[DOWN], LK)), 8, &[], HADOKEN);
    assert!(o.cancelled(), "{o:?}");
    let landed: Vec<(usize, LandedHit)> = frames
        .iter()
        .enumerate()
        .filter_map(|(t, f)| f.landed.map(|h| (t, h)))
        .collect();
    assert_eq!(landed.len(), 2, "the short and the fireball: {landed:?}");
    let (second, hit) = landed[1];
    assert!(
        hit.into_hitstun,
        "the fireball lands in the short's hitstun"
    );
    assert_eq!(frames[second].p1.move_id, HADOKEN);
    assert_eq!(frames[second - 1].combo, None);
    assert_eq!(
        frames[second].combo,
        Some(ComboCount {
            hits: 2,
            damage: None
        })
    );
    assert_ne!(
        frames[second].slot,
        frames[second - 1].slot,
        "the count is drawn"
    );
    // Into the Shoryuken, the same on a crouching Ken from the closest
    // spacing (on a standing one the short's pushback leaves him out of the
    // uppercut's reach).
    close_in(&mut s, 0x24);
    let (o, frames) = outcome(&s, DP, 8, &[DOWN], SHORYUKEN);
    assert!(o.cancelled(), "{o:?}");
    let second = frames
        .iter()
        .enumerate()
        .filter(|(_, f)| f.landed.is_some())
        .nth(1)
        .map(|(t, _)| t)
        .expect("the Shoryuken hits");
    assert_eq!(frames[second].p1.move_id, SHORYUKEN);
    assert_eq!(
        frames[second].combo,
        Some(ComboCount {
            hits: 2,
            damage: None
        })
    );
}

#[test]
fn a_cancel_replays_the_same_from_a_restored_snapshot() {
    let Some(mut s) = ryu_ken(MatchConfig::from_preset(MODERN)) else {
        return;
    };
    close_in(&mut s, 0x30);
    let before = s.fight_machine().unwrap().clone();
    let t = qcf_from((&[DOWN], LK));
    // P1's inputs, as `run` gives them, frame by frame.
    let inputs: Vec<([bool; 4], [bool; 6])> = (0..60)
        .map(|n| {
            let (held, button): (&[usize], Option<usize>) = match n {
                0 => (t.normal.0, Some(t.normal.1)),
                n if n <= t.motion.len() => (t.motion[n - 1], None),
                n if n <= 8 => (t.last, (n == 8).then_some(t.button)),
                _ => (&[], None),
            };
            let mut d = [false; 4];
            held.iter().for_each(|&x| d[x] = true);
            let mut b = [false; 6];
            if let Some(x) = button {
                b[x] = true;
            }
            (d, b)
        })
        .collect();
    let mut machine = before.clone();
    let feed = |m: &mut sf_engine::run::PowerOnMachine, (d, b): ([bool; 4], [bool; 6])| {
        m.set_p1_direction(d[0], d[1], d[2], d[3]);
        m.set_p1_buttons(b);
        m.set_p2_direction(false, false, false, false);
        m.set_p2_buttons([false; 6]);
        m.set_cabinet_inputs(false, false, false, false, false);
        m.step();
    };
    // Into the window, before the press.
    let mut n = 0;
    while machine.toggle_state().cancel.window[0] == 0 {
        feed(&mut machine, inputs[n]);
        n += 1;
    }
    assert!(n < 8, "the window opens before the press");
    let snapshot = machine.save_snapshot();
    assert_ne!(
        snapshot.toggle_state.cancel.window[..2],
        [0, 0],
        "the window is in the snapshot"
    );
    let mut straight = machine;
    let mut restored = before;
    restored.restore_snapshot(&snapshot);
    for &input in &inputs[n..] {
        feed(&mut straight, input);
        feed(&mut restored, input);
        assert!(straight.save_snapshot() == restored.save_snapshot());
    }
    assert_eq!(
        straight.toggle_state().cancel.count[0],
        1,
        "the cancel happened"
    );
}

#[test]
fn a_buffered_normal_press_followed_by_a_motion_does_not_cancel_into_a_special() {
    let Some(mut s) = ryu_ken(MatchConfig::from_preset(MODERN)) else {
        return;
    };
    close_in(&mut s, 0x30);
    // Regression for review finding 1: during a crouching short (connects on
    // frame 6), P1 presses and releases LK on down-back (frames 3..=4, before
    // the Tatsumaki's D, DB, B motion is complete) and only reaches back on
    // frame 5 without pressing or releasing LK there. The recovery buffer's
    // own slot collects the later completion, but a cancel must not fire a
    // special that was not ready when the button was pressed.
    let mut s_incomplete = s.clone();
    let frames: Vec<Frame> = (0..40)
        .map(|n| {
            let (held, button): (&[usize], Option<usize>) = match n {
                0 => (&[DOWN], Some(LK)),
                1 | 2 => (&[DOWN], None),
                3 => (&[DOWN, LEFT], Some(LK)),
                4 => (&[DOWN, LEFT], None),
                5..=10 => (&[LEFT], None),
                _ => (&[], None),
            };
            step(
                &mut s_incomplete,
                with(|i| {
                    held.iter().for_each(|&d| i.p1.directions[d] = true);
                    if let Some(b) = button {
                        i.p1.buttons[b] = true;
                    }
                }),
            )
        })
        .collect();
    let c = frames
        .iter()
        .position(|f| f.window > 0)
        .expect("the crouching short connects");
    let end = frames
        .iter()
        .position(|f| f.p1.move_id != 0x17)
        .expect("the crouching short ends");
    assert!(
        frames[..end].iter().all(|f| f.p1.move_id != 0x1b),
        "a press buffered before the command was done does not cancel into the Tatsumaki"
    );
    assert_eq!(
        s_incomplete
            .fight_machine()
            .unwrap()
            .toggle_state()
            .cancel
            .count[0],
        0
    );
    assert!(c < end);

    // When the Tatsumaki's command IS already done at the buffered press
    // (back reached on frame 4 when LK is pressed), the buffered press does
    // cancel the crouching short on the window's first frame.
    let mut s_ready = s;
    let ready_frames: Vec<Frame> = (0..40)
        .map(|n| {
            let (held, button): (&[usize], Option<usize>) = match n {
                0 => (&[DOWN], Some(LK)),
                1 | 2 => (&[DOWN], None),
                3 => (&[DOWN, LEFT], None),
                4 | 5 => (&[LEFT], (n == 4).then_some(LK)),
                _ => (&[], None),
            };
            step(
                &mut s_ready,
                with(|i| {
                    held.iter().for_each(|&d| i.p1.directions[d] = true);
                    if let Some(b) = button {
                        i.p1.buttons[b] = true;
                    }
                }),
            )
        })
        .collect();
    let c_ready = ready_frames
        .iter()
        .position(|f| f.window > 0)
        .expect("the crouching short connects");
    assert_eq!(
        ready_frames[c_ready + 1].p1.move_id,
        0x1b,
        "a special ready at the buffered press cancels on the window's first frame"
    );
    assert_eq!(
        s_ready.fight_machine().unwrap().toggle_state().cancel.count[0],
        1
    );
}

#[test]
fn strict_commands_require_the_commands_last_direction_at_the_press() {
    let strict = MatchConfig::from_preset(MODERN).with_lenient_command_trackers(false);
    let Some(mut s) = ryu_ken(strict) else { return };
    assert!(
        !s.fight_machine()
            .unwrap()
            .match_config()
            .toggles
            .lenient_command_trackers
    );
    close_in(&mut s, 0x30);
    // Holding forward (the Hadoken's last direction) at the press cancels
    // under strict commands.
    let forward_at_press = Try {
        normal: (&[DOWN], LK),
        motion: &[
            &[DOWN],
            &[DOWN, RIGHT],
            &[RIGHT],
            &[RIGHT],
            &[RIGHT],
            &[RIGHT],
            &[RIGHT],
        ],
        last: &[RIGHT],
        button: LP,
    };
    let (o, _) = outcome(&s, forward_at_press, 8, &[], HADOKEN);
    assert!(
        o.cancelled(),
        "strict commands: holding forward at the press cancels into the Hadoken: {o:?}"
    );

    // Letting go to neutral before the press leaves the tracker done under
    // lenient commands, but resets it under strict commands: no cancel.
    let neutral_at_press = Try {
        normal: (&[DOWN], LK),
        motion: &[
            &[DOWN],
            &[DOWN, RIGHT],
            &[RIGHT],
            &[RIGHT],
            &[RIGHT],
            &[RIGHT],
            &[],
        ],
        last: &[],
        button: LP,
    };
    let (o_strict, _) = outcome(&s, neutral_at_press, 8, &[], HADOKEN);
    assert!(
        !o_strict.cancelled() && o_strict.special.is_none_or(|sp| sp >= o_strict.end),
        "strict commands: neutral at the press does not cancel: {o_strict:?}"
    );
    let Some(mut s_lenient) = ryu_ken(MatchConfig::from_preset(MODERN)) else {
        return;
    };
    close_in(&mut s_lenient, 0x30);
    let (o_lenient, _) = outcome(&s_lenient, neutral_at_press, 8, &[], HADOKEN);
    assert!(
        o_lenient.cancelled(),
        "lenient commands: the same neutral press cancels: {o_lenient:?}"
    );
}

#[test]
fn being_hit_closes_the_cancel_window() {
    // With cancels on and true-combo blockstun off, Ken guards Ryu's
    // crouching short (`0x17`, opening Ryu's 12-frame cancel window) and can
    // counterattack immediately while Ryu is still recovering, closing Ryu's
    // open cancel window on the frame Ken's hit lands.
    let Some(mut s) = ryu_ken(MatchConfig::from_preset(MODERN).with_true_combos(false)) else {
        return;
    };
    close_in(&mut s, 0x30);
    let mut frames = Vec::new();
    let mut opened = None;
    for n in 0..25 {
        let input = with(|i| {
            if n == 0 {
                i.p1.directions[DOWN] = true;
                i.p1.buttons[LK] = true;
            }
            if opened.is_none() {
                CROUCH_GUARD.iter().for_each(|&d| i.p2.directions[d] = true);
            } else if opened == Some(n - 1) {
                i.p2.directions[DOWN] = true;
                i.p2.buttons[LK] = true;
            }
        });
        let f = step(&mut s, input);
        if opened.is_none() && f.window > 0 {
            opened = Some(n);
        }
        frames.push(f);
    }
    let c = opened.expect("Ryu's crouching short connects on guard and opens the cancel window");
    let hit = (c + 1..frames.len())
        .find(|&t| frames[t].p1.hitstun())
        .expect("Ken's crouching short hits Ryu while Ryu's cancel window is open");
    assert_eq!(frames[c].window, sf_engine::combo::cancel::WINDOW);
    assert!(frames[c..hit].iter().all(|f| f.window > 0));
    assert_eq!(
        frames[hit].window, 0,
        "being hit closes the cancel window at once"
    );
}
