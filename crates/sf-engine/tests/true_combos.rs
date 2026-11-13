//! Ticket #261 (`SP17`): true combos, the first of the modern rules.
//!
//! At the headless session's seam (inputs in; the presented frames, the
//! fight reader and the log out), in Local fights: with the true-combos
//! switch on (Definitive Overhaul) a fighter in a ground hit reaction can be
//! hit, so a light normal that leaves enough hitstun links into a second
//! hit and the era HUD's combo count shows it; with it off (Classic
//! Competitive) the defender is invulnerable for his whole reaction, as in
//! 1987. Hitstun and blockstun follow each move's data
//! (`sf_engine::combo::move_stun`), read through the fight reader; Ryu, Ken
//! and Retsu follow the same rule. Everything lives in the machine's
//! snapshot, so a combo replays the same from a restored snapshot. Faithful
//! 1987's checks are the rest of the suite's (its MAME checks and frame
//! goldens). Local ROMs only (Retsu: and the local art pack); each test
//! skips without them.

#[path = "support/select.rs"]
mod select;

use std::path::Path;

use sf_engine::combo::move_stun;
use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::fight_reader::FighterRead;
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
const PACK: &str = "../../assets/sf1-port-art";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const RECIPES: [&str; 2] = [
    include_str!("../../../art/recipes/ryu.json"),
    include_str!("../../../art/recipes/retsu.json"),
];
const REVIEWS: [&str; 2] = [
    include_str!("../../../art/review/ryu.json"),
    include_str!("../../../art/review/retsu.json"),
];
const GEKI_RECIPES: [&str; 2] = [
    include_str!("../../../art/recipes/ryu.json"),
    include_str!("../../../art/recipes/geki.json"),
];
const GEKI_REVIEWS: [&str; 2] = [
    include_str!("../../../art/review/ryu.json"),
    include_str!("../../../art/review/geki.json"),
];
const JOE_RECIPES: [&str; 2] = [
    include_str!("../../../art/recipes/ryu.json"),
    include_str!("../../../art/recipes/joe.json"),
];
const JOE_REVIEWS: [&str; 2] = [
    include_str!("../../../art/review/ryu.json"),
    include_str!("../../../art/review/joe.json"),
];
const MIKE_RECIPES: [&str; 2] = [
    include_str!("../../../art/recipes/ryu.json"),
    include_str!("../../../art/recipes/mike.json"),
];
const MIKE_REVIEWS: [&str; 2] = [
    include_str!("../../../art/review/ryu.json"),
    include_str!("../../../art/review/mike.json"),
];
const LEE_RECIPES: [&str; 2] = [
    include_str!("../../../art/recipes/ryu.json"),
    include_str!("../../../art/recipes/lee.json"),
];
const LEE_REVIEWS: [&str; 2] = [
    include_str!("../../../art/review/ryu.json"),
    include_str!("../../../art/review/lee.json"),
];
const GEN_RECIPES: [&str; 2] = [
    include_str!("../../../art/recipes/ryu.json"),
    include_str!("../../../art/recipes/gen.json"),
];
const GEN_REVIEWS: [&str; 2] = [
    include_str!("../../../art/review/ryu.json"),
    include_str!("../../../art/review/gen.json"),
];
const BIRDIE_RECIPES: [&str; 2] = [
    include_str!("../../../art/recipes/ryu.json"),
    include_str!("../../../art/recipes/birdie.json"),
];
const BIRDIE_REVIEWS: [&str; 2] = [
    include_str!("../../../art/review/ryu.json"),
    include_str!("../../../art/review/birdie.json"),
];
const EAGLE_RECIPES: [&str; 2] = [
    include_str!("../../../art/recipes/ryu.json"),
    include_str!("../../../art/recipes/eagle.json"),
];
const EAGLE_REVIEWS: [&str; 2] = [
    include_str!("../../../art/review/ryu.json"),
    include_str!("../../../art/review/eagle.json"),
];
const ADON_RECIPES: [&str; 2] = [
    include_str!("../../../art/recipes/ryu.json"),
    include_str!("../../../art/recipes/adon.json"),
];
const ADON_REVIEWS: [&str; 2] = [
    include_str!("../../../art/review/ryu.json"),
    include_str!("../../../art/review/adon.json"),
];
const SAGAT_RECIPES: [&str; 2] = [
    include_str!("../../../art/recipes/ryu.json"),
    include_str!("../../../art/recipes/sagat.json"),
];
const SAGAT_REVIEWS: [&str; 2] = [
    include_str!("../../../art/review/ryu.json"),
    include_str!("../../../art/review/sagat.json"),
];
const FIGHT_LOOP: u32 = 0x3eafa;
const RIGHT: usize = 0;
const LEFT: usize = 1;
const DOWN: usize = 2;
const UP: usize = 3;
const LP: usize = 0;
const MP: usize = 1;
const HP: usize = 2;
const LK: usize = 3;
const MK: usize = 4;
const HK: usize = 5;
const MODERN: RulesetPreset = RulesetPreset::DefinitiveOverhaul;
const CLASSIC: RulesetPreset = RulesetPreset::ClassicCompetitive;

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

fn ryu_ken(preset: RulesetPreset) -> Option<GameSession> {
    let a = assets()?;
    Some(local(
        GameSession::new(a, MatchConfig::from_preset(preset)),
        TournamentFighter::Ryu,
        TournamentFighter::Ken,
    ))
}

fn fighter(s: &GameSession, side: Side) -> FighterRead {
    s.fight().unwrap().fight_read().fighters[side.index()]
}

/// A side's fighter entity.
fn entity(s: &GameSession, ps: u32) -> usize {
    let w = s.fight_machine().unwrap().work_ram();
    let a = (ps + 0x10 - 0xff8000) as usize;
    u16::from_be_bytes([w[a], w[a + 1]]) as usize
}

fn obj(s: &GameSession, e: usize, off: usize) -> u8 {
    s.fight_machine().unwrap().state.objectram[e + off]
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

/// One frame of P1 pressing `button` (and holding `dirs`), P2 holding `p2_dirs`.
fn press(button: Option<usize>, dirs: &[usize], p2_dirs: &[usize]) -> FrameInputs {
    with(|i| {
        dirs.iter().for_each(|&d| i.p1.directions[d] = true);
        p2_dirs.iter().for_each(|&d| i.p2.directions[d] = true);
        if let Some(b) = button {
            i.p1.buttons[b] = true;
        }
    })
}

/// What one frame showed.
#[derive(Debug, Clone, Copy)]
struct Frame {
    p1: FighterRead,
    p2: FighterRead,
    landed: Option<sf_engine::fight_reader::LandedHit>,
    heavy: Option<sf_engine::impact::ImpactCue>,
    combo: Option<ComboCount>,
    /// P2's entity +0xf: bit 3 is the invulnerable flag.
    p2_flags: u8,
    /// A hash of the presented frame's pixels in P1's combo slot.
    slot: u64,
}

fn step(s: &mut GameSession, input: FrameInputs) -> Frame {
    let out = s.step(input);
    let p2e = entity(s, P2_STRUCT);
    let r = regions::combo_slot(Side::P1);
    let mut slot = 0xcbf2_9ce4_8422_2325u64;
    for (i, px) in out.frame.argb.iter().enumerate() {
        if r.contains(i % WIDTH, i / WIDTH) {
            slot = (slot ^ u64::from(*px)).wrapping_mul(0x100_0000_01b3);
        }
    }
    Frame {
        p1: fighter(s, Side::P1),
        p2: fighter(s, Side::P2),
        landed: s.fight().unwrap().fight_read().landed_by(Side::P1),
        heavy: out.impacts.into_iter().find(|cue| cue.kind == sf_engine::impact::ImpactKind::HeavyStrike),
        combo: s.fight().unwrap().callouts().frame().side(Side::P1).combo,
        p2_flags: obj(s, p2e, 0xf),
        slot,
    }
}

/// P1 presses `button` (with `dirs`) on frame 0 and then every `every`
/// frames for `frames` frames; P2 holds `p2_dirs`.
fn string(
    s: &mut GameSession,
    button: usize,
    dirs: &[usize],
    p2_dirs: &[usize],
    every: usize,
    frames: usize,
) -> Vec<Frame> {
    (0..frames)
        .map(|t| {
            let b = (t % every == 0).then_some(button);
            step(s, press(b, dirs, p2_dirs))
        })
        .collect()
}

fn hits(frames: &[Frame]) -> Vec<(usize, bool)> {
    frames
        .iter()
        .enumerate()
        .filter_map(|(t, f)| f.landed.map(|h| (t, h.into_hitstun)))
        .collect()
}

/// The frames P2 reads as in hitstun and in blockstun from the frame a hit
/// or a guard lands.
fn stun_frames(frames: &[Frame]) -> (usize, usize) {
    let hit = frames.iter().position(|f| f.landed.is_some());
    let hitstun = hit.map_or(0, |h| {
        frames[h..].iter().take_while(|f| f.p2.hitstun()).count()
    });
    let guard = frames.iter().position(|f| f.p2.blockstun);
    let blockstun = guard.map_or(0, |g| {
        frames[g..].iter().take_while(|f| f.p2.blockstun).count()
    });
    (hitstun, blockstun)
}

// ---- the tests ---------------------------------------------------------------

#[test]
fn clean_heavy_impact_covers_standing_crouching_and_jumping_normals() {
    use sf_engine::impact::ImpactFighter;

    let Some(mut close) = ryu_ken(MODERN) else { return };
    let mut far = close.clone();
    close_in(&mut close, 0x30);
    for (button, dirs, expected_move) in [
        (HP, &[][..], 0x10),
        (HK, &[][..], 0x16),
        (HP, &[DOWN][..], 0x1c),
        (HK, &[DOWN][..], 0x18),
        (HP, &[UP][..], 0x20),
        (HK, &[UP][..], 0x26),
    ] {
        let mut s = close.clone();
        let mut contacts = Vec::new();
        for frame in 0..75 {
            let input = if dirs == [UP] {
                let jump = if frame == 0 { dirs } else { &[] };
                press((frame == 22).then_some(button), jump, &[])
            } else {
                press((frame == 0).then_some(button), dirs, &[])
            };
            let out = step(&mut s, input);
            if out.landed.is_some() {
                let cue = out.heavy.expect("a heavy normal produces a cue on the contact frame");
                assert_eq!(cue.fighter, ImpactFighter::P2);
                assert_eq!(cue.frame, s.fight_machine().unwrap().state.frame);
                contacts.push(out.p1.move_id);
            } else {
                assert!(out.heavy.is_none());
            }
        }
        if expected_move == 0x1c {
            assert!(matches!(contacts.as_slice(), [0x1c..=0x1e]), "{contacts:?}");
        } else {
            assert_eq!(contacts, [expected_move], "input {button} {dirs:?}");
        }
    }

    for button in [LP, MP] {
        let frames = string(&mut close.clone(), button, &[], &[], 60, 60);
        assert_eq!(hits(&frames).len(), 1);
        assert!(frames.iter().all(|frame| frame.heavy.is_none()));
    }
    let guarded = string(&mut close, HP, &[], &[RIGHT], 60, 60);
    assert!(stun_frames(&guarded).1 > 0);
    assert!(guarded.iter().all(|frame| frame.heavy.is_none() && frame.landed.is_none()));
    let whiff = string(&mut far, HP, &[], &[], 60, 60);
    assert!(whiff.iter().any(|frame| frame.p1.attack_active));
    assert!(whiff.iter().all(|frame| frame.heavy.is_none() && frame.landed.is_none()));
}

#[test]
fn a_light_punch_links_into_a_second_and_the_era_hud_counts_two() {
    let Some(mut s) = ryu_ken(MODERN) else { return };
    close_in(&mut s, 0x30);
    let frames = string(&mut s, LP, &[], &[], 2, 30);
    let landed = hits(&frames);
    assert_eq!(landed.len(), 2, "two hits: {landed:?}");
    assert!(
        !landed[0].1 && landed[1].1,
        "the second lands in the first's hitstun: {landed:?}"
    );
    let (first, second) = (landed[0].0, landed[1].0);
    // Ken's reaction is hittable: no invulnerable flag on its frames.
    assert!(frames[first + 1..second]
        .iter()
        .all(|f| f.p2.hitstun() && f.p2_flags & 0x08 == 0));
    // The HUD counts the combo from its second hit, drawn in P1's slot.
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
    // The third punch is pushed out of reach: the reactions' pushback ends it.
    assert_eq!(
        frames[second..]
            .iter()
            .filter(|f| f.landed.is_some())
            .count(),
        1
    );
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());
}

#[test]
fn with_the_switch_off_the_defender_cannot_be_hit_during_his_reaction() {
    let Some(mut s) = ryu_ken(CLASSIC) else {
        return;
    };
    assert!(
        !s.fight_machine()
            .unwrap()
            .match_config()
            .toggles
            .true_combos
    );
    close_in(&mut s, 0x30);
    let frames = string(&mut s, LP, &[], &[], 2, 30);
    let landed = hits(&frames);
    assert_eq!(
        landed.len(),
        1,
        "one hit; the rest of the string meets the 1987 reaction: {landed:?}"
    );
    let first = landed[0].0;
    // Ken is invulnerable from his reaction's first frame to its end: the
    // 1987 stream's 26 frames (9 + 16 + 1), after the hold it cuts short.
    let stunned = frames[first..]
        .iter()
        .take_while(|f| f.p2.hitstun())
        .count();
    assert!(
        stunned >= 26,
        "the 1987 reaction runs its stream: {stunned}"
    );
    assert!(frames[first + 3..first + stunned]
        .iter()
        .all(|f| f.p2_flags & 0x08 != 0));
    assert!(frames.iter().all(|f| f.combo.is_none()));
    assert_eq!(
        s.fight_machine().unwrap().toggle_state().combo,
        Default::default(),
        "the combo state stays untouched"
    );
}

#[test]
fn hitstun_and_blockstun_follow_each_moves_data_through_the_fight_reader() {
    let Some(mut s) = ryu_ken(MODERN) else { return };
    close_in(&mut s, 0x30);
    let ryu = Some(TournamentFighter::Ryu);
    // (button, P1's directions, P2's guard, the move, its strength level)
    let cases: [(usize, &[usize], &[usize], u8, u8); 3] = [
        (LP, &[], &[RIGHT], 0x0d, 1),
        (MP, &[], &[RIGHT], 0x0f, 2),
        (LK, &[DOWN], &[DOWN, RIGHT], 0x17, 1),
    ];
    for (button, dirs, guard, id, level) in cases {
        let want = move_stun(ryu, id, level);
        // On hit.
        let mut hit = s.clone();
        let frames = string(&mut hit, button, dirs, &[], 60, 60);
        assert_eq!(
            frames.iter().filter(|f| f.landed.is_some()).count(),
            1,
            "move {id:#04x} hits"
        );
        assert_eq!(
            stun_frames(&frames).0,
            usize::from(want.hit),
            "move {id:#04x}: hitstun {want:?}"
        );
        // Guarded: P2 holds back (down-back for a low attack) and is locked
        // in his guard for the move's blockstun.
        let mut guarded = s.clone();
        let frames = string(&mut guarded, button, dirs, guard, 60, 60);
        assert!(
            frames.iter().all(|f| f.landed.is_none()),
            "move {id:#04x} is guarded"
        );
        assert_eq!(
            stun_frames(&frames).1,
            usize::from(want.block),
            "move {id:#04x}: blockstun {want:?}"
        );
        let first = frames.iter().position(|f| f.p2.blockstun).unwrap();
        assert!(matches!(frames[first].p2.move_id, 4 | 8), "in his guard");
    }
}

#[test]
fn a_guard_in_blockstun_holds_and_acts_on_the_frame_it_ends() {
    let Some(mut s) = ryu_ken(MODERN) else { return };
    close_in(&mut s, 0x30);
    // Ken guards a medium punch, then lets go of back and presses LP every
    // other frame: nothing comes out until his blockstun ends, and his jab
    // starts on the frame it does.
    let block = move_stun(Some(TournamentFighter::Ryu), 0x0f, 2).block as usize;
    let mut frames = Vec::new();
    let mut guarded_at = None;
    for t in 0..60 {
        let input = with(|i| {
            if t == 0 {
                i.p1.buttons[MP] = true;
            }
            match guarded_at {
                None => i.p2.directions[RIGHT] = true,
                Some(_) => i.p2.buttons[LP] = t % 2 == 0,
            }
        });
        let f = step(&mut s, input);
        if guarded_at.is_none() && f.p2.blockstun {
            guarded_at = Some(t);
        }
        frames.push(f);
    }
    let g = guarded_at.expect("the medium punch is guarded");
    assert!(
        frames[g..g + block]
            .iter()
            .all(|f| f.p2.blockstun && matches!(f.p2.move_id, 4 | 8)),
        "held in his guard"
    );
    let jab = frames
        .iter()
        .position(|f| f.p2.move_id == 0x0d)
        .expect("Ken's jab comes out");
    assert_eq!(
        jab,
        g + block,
        "his jab starts on the first frame out of blockstun"
    );
}

#[test]
fn a_hit_fighter_acts_on_the_frame_his_hitstun_ends() {
    let Some(mut s) = ryu_ken(MODERN) else { return };
    close_in(&mut s, 0x30);
    // Ryu's jab hits; Ken presses LP every other frame from then on, and his
    // jab starts on the first frame out of hitstun. Ryu acts again 4 frames
    // after his hit (the 2 ticks his jab runs after it, and 2), so the jab
    // is +5 on hit, as its row is tuned.
    let stun = usize::from(move_stun(Some(TournamentFighter::Ryu), 0x0d, 1).hit);
    let mut frames = Vec::new();
    let mut hit_at = None;
    for t in 0..40 {
        let input = with(|i| {
            i.p1.buttons[LP] = t == 0;
            i.p2.buttons[LP] = hit_at.is_some() && t % 2 == 0;
        });
        let f = step(&mut s, input);
        if hit_at.is_none() && f.landed.is_some() {
            hit_at = Some(t);
        }
        frames.push(f);
    }
    let h = hit_at.expect("the jab hits");
    let ken = frames
        .iter()
        .position(|f| f.p2.move_id == 0x0d)
        .expect("Ken's jab comes out");
    assert_eq!(ken, h + stun, "Ken acts on the frame his hitstun ends");
    let ryu_free = frames[h..]
        .iter()
        .position(|f| f.p1.state & 0x80 == 0)
        .map(|n| h + n)
        .expect("Ryu's jab ends");
    assert_eq!(ryu_free + 1 - h, 4, "Ryu can act 4 frames after his hit");
    assert_eq!(ken as i64 - (ryu_free + 1) as i64, 5, "+5 on hit");
}

#[test]
fn ken_and_retsu_follow_the_same_rule_as_ryu() {
    // Ken on P1 against Ryu: his jab's hitstun is his row's, and it links.
    let Some(a) = assets() else { return };
    let mut s = local(
        GameSession::new(a, MatchConfig::from_preset(MODERN)),
        TournamentFighter::Ken,
        TournamentFighter::Ryu,
    );
    close_in(&mut s, 0x30);
    let want = move_stun(Some(TournamentFighter::Ken), 0x0d, 1);
    let mut single = s.clone();
    assert_eq!(
        stun_frames(&string(&mut single, LP, &[], &[], 60, 40)).0,
        usize::from(want.hit)
    );
    let frames = string(&mut s, LP, &[], &[], 2, 30);
    let landed = hits(&frames);
    assert!(
        landed.len() >= 2 && landed[1].1,
        "Ken's jab links: {landed:?}"
    );
    assert_eq!(
        frames[landed[1].0].combo,
        Some(ComboCount {
            hits: 2,
            damage: None
        })
    );

    // Retsu, on his 1987 player table.
    if !Path::new(PACK).is_dir() {
        eprintln!("skipping Retsu: the local art pack is not present");
        return;
    }
    let a = load_assets(ROMS, MANIFEST).unwrap();
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
    let session = || {
        GameSession::new(
            load_assets(ROMS, MANIFEST).unwrap(),
            MatchConfig::from_preset(MODERN),
        )
        .with_sprite_bank(&art.bank)
        .with_player_tables(&art.catalogues)
    };
    // His jab's hitstun is his row's.
    let mut s = local(session(), TournamentFighter::Retsu, TournamentFighter::Ken);
    assert!(
        s.data_bank()
            .unwrap()
            .table(TournamentFighter::Retsu)
            .is_some(),
        "Retsu plays his bank table"
    );
    let mut single = s.clone();
    close_in(&mut single, 0x38);
    let want = move_stun(Some(TournamentFighter::Retsu), 0x0d, 1);
    let frames = string(&mut single, LP, &[], &[], 80, 60);
    assert_eq!(
        frames.iter().filter(|f| f.landed.is_some()).count(),
        1,
        "his jab hits"
    );
    assert_eq!(
        stun_frames(&frames).0,
        usize::from(want.hit),
        "his jab's hitstun is his row's"
    );
    // A jab that lands close enough links into a second jab (his partial
    // jab staggers Ken with the 1987 reaction 0x32, whose pushback pulls him
    // out of reach from further out): after a jump in, jabs every other frame.
    close_in(&mut s, 0x60);
    let frames: Vec<Frame> = (0..90)
        .map(|t| {
            let input = with(|i| {
                i.p1.directions[RIGHT] = t < 2;
                i.p1.directions[UP] = t < 2;
                i.p1.buttons[LP] = t > 2 && t % 2 == 0;
            });
            step(&mut s, input)
        })
        .collect();
    let landed = hits(&frames);
    let link = landed
        .iter()
        .position(|h| h.1)
        .unwrap_or_else(|| panic!("Retsu's jab links: {landed:?}"));
    let (first, second) = (landed[link - 1].0, landed[link].0);
    assert_eq!(
        (frames[first].p1.move_id, frames[second].p1.move_id),
        (0x0d, 0x0d),
        "jab into jab"
    );
    assert_eq!(
        frames[second].combo,
        Some(ComboCount {
            hits: 2,
            damage: None
        })
    );
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());

    // Retsu as the defender: his ground reactions take boxes and can be hit.
    let mut s = local(session(), TournamentFighter::Ryu, TournamentFighter::Retsu);
    close_in(&mut s, 0x30);
    let frames = string(&mut s, LP, &[], &[], 2, 30);
    let landed = hits(&frames);
    assert!(
        landed.len() >= 2 && landed[1].1,
        "Ryu's jab links on Retsu: {landed:?}"
    );
    assert!(
        frames[landed[0].0 + 1..landed[1].0]
            .iter()
            .all(|f| f.p2_flags & 0x08 == 0),
        "Retsu's reaction is hittable"
    );
    assert_eq!(
        frames[landed[1].0].combo,
        Some(ComboCount {
            hits: 2,
            damage: None
        })
    );
}

#[test]
fn a_knock_back_stays_invulnerable_no_juggles() {
    let Some(mut s) = ryu_ken(MODERN) else { return };
    close_in(&mut s, 0x30);
    // A standing heavy punch knocks Ken back through the air (reaction
    // 0x33, state 0x89): the 1987 reaction, invulnerable throughout.
    let frames = string(&mut s, HP, &[], &[], 80, 70);
    let h = frames
        .iter()
        .position(|f| f.landed.is_some())
        .expect("the heavy punch hits");
    let airborne: Vec<&Frame> = frames[h..].iter().take_while(|f| f.p2.hitstun()).collect();
    assert!(
        airborne.iter().all(|f| f.p2.state & 7 == 1),
        "an airborne reaction"
    );
    assert!(
        airborne[3..].iter().all(|f| f.p2_flags & 0x08 != 0),
        "invulnerable"
    );
    assert!(
        airborne.len() > 30,
        "the 1987 knock-back's length, not a stun: {}",
        airborne.len()
    );
}

#[test]
fn a_combo_replays_the_same_from_a_restored_snapshot() {
    let Some(mut s) = ryu_ken(MODERN) else { return };
    close_in(&mut s, 0x30);
    let before = s.fight_machine().unwrap().clone();
    // Into the first punch's hitstun.
    let inputs: Vec<FrameInputs> = (0..30)
        .map(|t| press((t % 2 == 0).then_some(LP), &[], &[]))
        .collect();
    let mut t = 0;
    while !fighter(&s, Side::P2).hitstun() {
        s.step(inputs[t]);
        t += 1;
    }
    let mid = s.fight_machine().unwrap().clone();
    let snapshot = mid.save_snapshot();
    assert_ne!(
        snapshot.toggle_state.combo.hitstun[..2],
        [0, 0],
        "the hitstun is in the snapshot"
    );
    // The rest of the combo, straight through and from the snapshot
    // restored over the machine as it was before the combo.
    let mut straight = mid;
    let mut restored = before;
    restored.restore_snapshot(&snapshot);
    for &input in &inputs[t..] {
        for m in [&mut straight, &mut restored] {
            let [r, l, d, u] = input.p1.directions;
            m.set_p1_direction(r, l, d, u);
            m.set_p1_buttons(input.p1.buttons);
            m.set_p2_direction(false, false, false, false);
            m.set_p2_buttons([false; 6]);
            m.set_cabinet_inputs(false, false, false, false, false);
            m.step();
        }
        assert!(straight.save_snapshot() == restored.save_snapshot());
    }
    assert_eq!(
        straight.toggle_state().combo.hits,
        restored.toggle_state().combo.hits
    );
}

#[test]
fn the_combo_state_follows_the_fighter_when_the_object_regions_swap() {
    // Ryu's sweep trades the two fighters' object regions mid-move (its
    // frame flags 0x10 / 0x20, `swap_regions`): Ken's hitstun, his hittable
    // reaction and its end, the combo's count and Ryu's own move must stay
    // with each fighter, wherever his region sits.
    let Some(mut s) = ryu_ken(MODERN) else { return };
    close_in(&mut s, 0x30);
    let want = move_stun(Some(TournamentFighter::Ryu), 0x18, 1);
    let mut frames = Vec::new();
    let mut ken_at = Vec::new();
    let mut ryu_at = Vec::new();
    for t in 0..70 {
        let input = with(|i| {
            i.p1.directions[DOWN] = t < 2;
            i.p1.buttons[5] = t == 0;
            i.p2.directions[DOWN] = true;
        });
        frames.push(step(&mut s, input));
        ken_at.push(entity(&s, P2_STRUCT));
        ryu_at.push(entity(&s, P1_STRUCT));
    }
    assert!(
        ryu_at.contains(&0xa00) && ken_at.contains(&0x400),
        "the regions swapped during the sweep"
    );
    let h = frames
        .iter()
        .position(|f| f.landed.is_some())
        .expect("the sweep hits the crouching Ken");
    assert_eq!(frames[h].p1.move_id, 0x18);
    assert_eq!(
        frames[h].p2.move_id & 0xf8,
        0x38,
        "a crouching (ground) reaction: {:#04x}",
        frames[h].p2.move_id
    );
    let stunned = frames[h..].iter().take_while(|f| f.p2.hitstun()).count();
    assert_eq!(
        stunned,
        usize::from(want.hit),
        "Ken's hitstun is the sweep's"
    );
    assert!(
        frames[h + 3..h + stunned]
            .iter()
            .all(|f| f.p2_flags & 0x08 == 0),
        "Ken's reaction stays hittable across the swap"
    );
    // Nothing is left on Ryu's side: he acts freely after his sweep.
    let c = s.fight_machine().unwrap().toggle_state().combo;
    assert_eq!((c.hitstun, c.blockstun), ([0; 3], [0; 3]));
    assert!(
        frames.last().unwrap().p1.state & 0x80 == 0,
        "Ryu's sweep ended"
    );
}

#[test]
fn geki_follows_the_same_rule_on_his_bank_table() {
    if assets().is_none() || !Path::new(PACK).is_dir() {
        eprintln!("skipping: the local ROM set or art pack is not present");
        return;
    }
    let a = load_assets(ROMS, MANIFEST).unwrap();
    let pack = ArtPack::open(Path::new(PACK)).expect("the art pack opens");
    let art = load_art_from(
        &a,
        &ArtSources {
            recipes: &GEKI_RECIPES,
            reviews: Some(&GEKI_REVIEWS),
            pack: Ok(&pack),
        },
    )
    .expect("the art loads");
    let session = || {
        GameSession::new(
            load_assets(ROMS, MANIFEST).unwrap(),
            MatchConfig::from_preset(MODERN),
        )
        .with_sprite_bank(&art.bank)
        .with_player_tables(&art.catalogues)
    };
    // His front kick's hitstun is his row's (+3 on hit). His claw thrust
    // knocks a standing Ken back (the 1987 reaction 0x33, airborne), so it
    // is not the move to test; and with Balanced roster off his normals fall
    // short of Ken's guard pose at his closest spacing (tested at the end of
    // this file), so his blockstun is tested below with him guarding instead.
    let mut s = local(session(), TournamentFighter::Geki, TournamentFighter::Ken);
    assert!(
        s.data_bank()
            .unwrap()
            .table(TournamentFighter::Geki)
            .is_some(),
        "Geki plays his bank table"
    );
    close_in(&mut s, 0x28);
    let want = move_stun(Some(TournamentFighter::Geki), 0x15, 2);
    let mut hit = s.clone();
    let frames = string(&mut hit, MK, &[], &[], 80, 60);
    assert_eq!(
        frames.iter().filter(|f| f.landed.is_some()).count(),
        1,
        "his front kick hits"
    );
    assert_eq!(
        stun_frames(&frames).0,
        usize::from(want.hit),
        "its hitstun is his row's"
    );
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());

    // Geki as the defender: his bank guard takes the blockstun of Ryu's
    // medium punch, and his ground reactions take boxes and can be hit.
    let mut s = local(session(), TournamentFighter::Ryu, TournamentFighter::Geki);
    close_in(&mut s, 0x30);
    let mut guarded = s.clone();
    let frames = string(&mut guarded, MP, &[], &[RIGHT], 80, 60);
    assert!(frames.iter().all(|f| f.landed.is_none()), "Geki guards");
    let block = usize::from(move_stun(Some(TournamentFighter::Ryu), 0x0f, 2).block);
    assert_eq!(
        stun_frames(&frames).1,
        block,
        "his guard takes the medium punch's blockstun"
    );
    let g = frames.iter().position(|f| f.p2.blockstun).unwrap();
    assert!(
        frames[g..g + block].iter().all(|f| f.p2.move_id == 4),
        "held in his guard"
    );
    let frames = string(&mut s, LP, &[], &[], 2, 30);
    let landed = hits(&frames);
    assert!(
        landed.len() >= 2 && landed[1].1,
        "Ryu's jab links on Geki: {landed:?}"
    );
    assert!(
        frames[landed[0].0 + 1..landed[1].0]
            .iter()
            .all(|f| f.p2_flags & 0x08 == 0),
        "Geki's reaction is hittable"
    );
    assert_eq!(
        frames[landed[1].0].combo,
        Some(ComboCount {
            hits: 2,
            damage: None
        })
    );
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());
}

#[test]
fn joe_follows_the_same_rule_on_his_bank_table() {
    if assets().is_none() || !Path::new(PACK).is_dir() {
        eprintln!("skipping: the local ROM set or art pack is not present");
        return;
    }
    let a = load_assets(ROMS, MANIFEST).unwrap();
    let pack = ArtPack::open(Path::new(PACK)).expect("the art pack opens");
    let art = load_art_from(
        &a,
        &ArtSources {
            recipes: &JOE_RECIPES,
            reviews: Some(&JOE_REVIEWS),
            pack: Ok(&pack),
        },
    )
    .expect("the art loads");
    let session = || {
        GameSession::new(
            load_assets(ROMS, MANIFEST).unwrap(),
            MatchConfig::from_preset(MODERN),
        )
        .with_sprite_bank(&art.bank)
        .with_player_tables(&art.catalogues)
    };
    // His jab's hitstun is his row's (+5 on hit), and it links into a
    // second jab, the era HUD counting two.
    let mut s = local(session(), TournamentFighter::Joe, TournamentFighter::Ken);
    assert!(
        s.data_bank()
            .unwrap()
            .table(TournamentFighter::Joe)
            .is_some(),
        "Joe plays his bank table"
    );
    close_in(&mut s, 0x30);
    let want = move_stun(Some(TournamentFighter::Joe), 0x0d, 1);
    let mut single = s.clone();
    let frames = string(&mut single, LP, &[], &[], 80, 60);
    assert_eq!(
        frames.iter().filter(|f| f.landed.is_some()).count(),
        1,
        "his jab hits"
    );
    assert_eq!(
        stun_frames(&frames).0,
        usize::from(want.hit),
        "its hitstun is his row's"
    );
    // Guarded, it locks Ken's guard for its blockstun.
    let mut guarded = s.clone();
    let frames = string(&mut guarded, LP, &[], &[RIGHT], 80, 60);
    assert!(frames.iter().all(|f| f.landed.is_none()), "Ken guards");
    assert_eq!(
        stun_frames(&frames).1,
        usize::from(want.block),
        "its blockstun is his row's"
    );
    // From point-blank: further out his jab's partial hit staggers Ken with
    // the 1987 reaction 0x30, whose pushback takes him out of the second
    // jab's reach (as Retsu's).
    close_in(&mut s, 0x20);
    let frames = string(&mut s, LP, &[], &[], 2, 30);
    let landed = hits(&frames);
    assert!(
        landed.len() >= 2 && landed[1].1,
        "Joe's jab links: {landed:?}"
    );
    assert_eq!(
        (
            frames[landed[0].0].p1.move_id,
            frames[landed[1].0].p1.move_id
        ),
        (0x0d, 0x0d),
        "jab into jab"
    );
    assert_eq!(
        frames[landed[1].0].combo,
        Some(ComboCount {
            hits: 2,
            damage: None
        })
    );
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());

    // Joe as the defender: his guards take the blockstun of Ryu's light
    // kick standing and his crouching short crouching, and his ground
    // reactions take boxes and can be hit. (With Balanced roster off, Ryu's
    // standing punches and medium kick pass over his standing guard: his ROM
    // block is a duck whose hurtbox is the lower two thirds of its grid. Both
    // sides of that are tested at the end of this file.)
    let mut s = local(session(), TournamentFighter::Ryu, TournamentFighter::Joe);
    close_in(&mut s, 0x30);
    for (button, dirs, guard, id, posture) in [
        (LK, &[][..], &[RIGHT][..], 0x14, 4),
        (LK, &[DOWN][..], &[DOWN, RIGHT][..], 0x17, 8),
    ] {
        let mut guarded = s.clone();
        let frames = string(&mut guarded, button, dirs, guard, 80, 60);
        assert!(
            frames.iter().all(|f| f.landed.is_none()),
            "Joe guards move {id:#04x}"
        );
        let block = usize::from(move_stun(Some(TournamentFighter::Ryu), id, 1).block);
        assert_eq!(
            stun_frames(&frames).1,
            block,
            "his guard takes move {id:#04x}'s blockstun"
        );
        let g = frames.iter().position(|f| f.p2.blockstun).unwrap();
        assert!(
            frames[g..g + block].iter().all(|f| f.p2.move_id == posture),
            "held in his guard"
        );
    }
    let frames = string(&mut s, LP, &[], &[], 2, 30);
    let landed = hits(&frames);
    assert!(
        landed.len() >= 2 && landed[1].1,
        "Ryu's jab links on Joe: {landed:?}"
    );
    assert!(
        frames[landed[0].0 + 1..landed[1].0]
            .iter()
            .all(|f| f.p2_flags & 0x08 == 0),
        "Joe's reaction is hittable"
    );
    assert_eq!(
        frames[landed[1].0].combo,
        Some(ComboCount {
            hits: 2,
            damage: None
        })
    );
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());
}

#[test]
fn mike_follows_the_same_rule_on_his_bank_table() {
    if assets().is_none() || !Path::new(PACK).is_dir() {
        eprintln!("skipping: the local ROM set or art pack is not present");
        return;
    }
    let a = load_assets(ROMS, MANIFEST).unwrap();
    let pack = ArtPack::open(Path::new(PACK)).expect("the art pack opens");
    let art = load_art_from(
        &a,
        &ArtSources {
            recipes: &MIKE_RECIPES,
            reviews: Some(&MIKE_REVIEWS),
            pack: Ok(&pack),
        },
    )
    .expect("the art loads");
    let session = || {
        GameSession::new(
            load_assets(ROMS, MANIFEST).unwrap(),
            MatchConfig::from_preset(MODERN),
        )
        .with_sprite_bank(&art.bank)
        .with_player_tables(&art.catalogues)
    };
    // His ducking punch's hitstun is his row's (+5 on hit), and it links
    // into a second ducking punch, the era HUD counting two (against a
    // standing Ken his 1987 heavyweight punches all trigger the airborne
    // knock-backs 0x33..=0x35; against a crouching Ken his ducking punch
    // triggers the ground crouching reaction 0x3b).
    let mut s = local(session(), TournamentFighter::Mike, TournamentFighter::Ken);
    assert!(
        s.data_bank()
            .unwrap()
            .table(TournamentFighter::Mike)
            .is_some(),
        "Mike plays his bank table"
    );
    close_in(&mut s, 0x28);
    let want = move_stun(Some(TournamentFighter::Mike), 0x11, 1);
    let mut single = s.clone();
    let frames = string(&mut single, LP, &[DOWN], &[DOWN], 80, 60);
    assert_eq!(
        frames.iter().filter(|f| f.landed.is_some()).count(),
        1,
        "his ducking punch hits"
    );
    assert_eq!(
        stun_frames(&frames).0,
        usize::from(want.hit),
        "its hitstun is his row's"
    );
    let mut guarded = s.clone();
    let frames = string(&mut guarded, LP, &[DOWN], &[DOWN, RIGHT], 80, 60);
    assert!(frames.iter().all(|f| f.landed.is_none()), "Ken guards low");
    assert_eq!(
        stun_frames(&frames).1,
        usize::from(want.block),
        "its blockstun is his row's"
    );
    let frames = string(&mut s, LP, &[DOWN], &[DOWN], 2, 35);
    let landed = hits(&frames);
    assert!(
        landed.len() >= 2 && landed[1].1,
        "Mike's ducking punch links: {landed:?}"
    );
    assert_eq!(
        (
            frames[landed[0].0].p1.move_id,
            frames[landed[1].0].p1.move_id
        ),
        (0x11, 0x11),
        "ducking punch into ducking punch"
    );
    assert_eq!(
        frames[landed[1].0].combo,
        Some(ComboCount {
            hits: 2,
            damage: None
        })
    );
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());

    // Mike as the defender: his guards take the blockstun of Ryu's medium
    // punch standing and his crouching short crouching, and his ground
    // reactions take boxes and can be hit.
    let mut s = local(session(), TournamentFighter::Ryu, TournamentFighter::Mike);
    close_in(&mut s, 0x30);
    for (button, dirs, guard, id, level, posture) in [
        (MP, &[][..], &[RIGHT][..], 0x0f, 2, 4),
        (LK, &[DOWN][..], &[DOWN, RIGHT][..], 0x17, 1, 8),
    ] {
        let mut guarded = s.clone();
        let frames = string(&mut guarded, button, dirs, guard, 80, 60);
        assert!(
            frames.iter().all(|f| f.landed.is_none()),
            "Mike guards move {id:#04x}"
        );
        let block = usize::from(move_stun(Some(TournamentFighter::Ryu), id, level).block);
        assert_eq!(
            stun_frames(&frames).1,
            block,
            "his guard takes move {id:#04x}'s blockstun"
        );
        let g = frames.iter().position(|f| f.p2.blockstun).unwrap();
        assert!(
            frames[g..g + block].iter().all(|f| f.p2.move_id == posture),
            "held in his guard"
        );
    }
    let frames = string(&mut s, LP, &[], &[], 2, 30);
    let landed = hits(&frames);
    assert!(
        landed.len() >= 2 && landed[1].1,
        "Ryu's jab links on Mike: {landed:?}"
    );
    assert!(
        frames[landed[0].0 + 1..landed[1].0]
            .iter()
            .all(|f| f.p2_flags & 0x08 == 0),
        "Mike's reaction is hittable"
    );
    assert_eq!(
        frames[landed[1].0].combo,
        Some(ComboCount {
            hits: 2,
            damage: None
        })
    );
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());
}

#[test]
fn lee_follows_the_same_rule_on_his_bank_table() {
    if !Path::new(ROMS).is_dir() || !Path::new(PACK).is_dir() {
        eprintln!("skipping: local ROMs or art pack not present");
        return;
    }
    let a = load_assets(ROMS, MANIFEST).unwrap();
    let pack = ArtPack::open(Path::new(PACK)).expect("the art pack opens");
    let art = load_art_from(
        &a,
        &ArtSources {
            recipes: &LEE_RECIPES,
            reviews: Some(&LEE_REVIEWS),
            pack: Ok(&pack),
        },
    )
    .expect("the art loads");
    let session = || {
        GameSession::new(
            load_assets(ROMS, MANIFEST).unwrap(),
            MatchConfig::from_preset(MODERN),
        )
        .with_sprite_bank(&art.bank)
        .with_player_tables(&art.catalogues)
    };
    // His palm strike's hitstun is his row's (+5 on hit), and it links into
    // his side kick, the era HUD counting two.
    let mut s = local(session(), TournamentFighter::Lee, TournamentFighter::Ken);
    assert!(
        s.data_bank()
            .unwrap()
            .table(TournamentFighter::Lee)
            .is_some(),
        "Lee plays his bank table"
    );
    close_in(&mut s, 0x20);
    let want = move_stun(Some(TournamentFighter::Lee), 0x0d, 1);
    let mut single = s.clone();
    let frames = string(&mut single, LP, &[], &[], 80, 60);
    assert_eq!(
        frames.iter().filter(|f| f.landed.is_some()).count(),
        1,
        "his palm strike hits"
    );
    assert_eq!(
        stun_frames(&frames).0,
        usize::from(want.hit),
        "its hitstun is his row's"
    );
    let mut guarded = s.clone();
    let frames = string(&mut guarded, LP, &[], &[RIGHT], 80, 60);
    assert!(frames.iter().all(|f| f.landed.is_none()), "Ken guards");
    assert_eq!(
        stun_frames(&frames).1,
        usize::from(want.block),
        "its blockstun is his row's"
    );
    // Back Ken into the right corner so 0x32's pushback is stopped by the
    // stage wall and keeps him in the second palm strike's reach.
    for _ in 0..250 {
        s.step(with(|i| {
            i.p1.directions[RIGHT] = true;
            i.p2.directions[RIGHT] = true;
        }));
    }
    close_in(&mut s, 0x20);
    let frames = string(&mut s, LP, &[], &[], 2, 30);
    let landed = hits(&frames);
    assert!(
        landed.len() >= 2 && landed[1].1,
        "Lee's palm strike links: {landed:?}"
    );
    assert_eq!(
        (
            frames[landed[0].0].p1.move_id,
            frames[landed[1].0].p1.move_id
        ),
        (0x0d, 0x0d),
        "palm strike into palm strike"
    );
    assert_eq!(
        frames[landed[1].0].combo,
        Some(ComboCount {
            hits: 2,
            damage: None
        })
    );
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());

    // Lee as the defender: his guards take the blockstun of Ryu's medium
    // punch standing and his crouching short crouching, and his ground
    // reactions take boxes and can be hit.
    let mut s = local(session(), TournamentFighter::Ryu, TournamentFighter::Lee);
    close_in(&mut s, 0x30);
    for (button, dirs, guard, id, level, posture) in [
        (MP, &[][..], &[RIGHT][..], 0x0f, 2, 4),
        (LK, &[DOWN][..], &[DOWN, RIGHT][..], 0x17, 1, 8),
    ] {
        let mut guarded = s.clone();
        let frames = string(&mut guarded, button, dirs, guard, 80, 60);
        assert!(
            frames.iter().all(|f| f.landed.is_none()),
            "Lee guards move {id:#04x}"
        );
        let block = usize::from(move_stun(Some(TournamentFighter::Ryu), id, level).block);
        assert_eq!(
            stun_frames(&frames).1,
            block,
            "his guard takes move {id:#04x}'s blockstun"
        );
        let g = frames.iter().position(|f| f.p2.blockstun).unwrap();
        assert!(
            frames[g..g + block].iter().all(|f| f.p2.move_id == posture),
            "held in his guard"
        );
    }
    let frames = string(&mut s, LP, &[], &[], 2, 30);
    let landed = hits(&frames);
    assert!(
        landed.len() >= 2 && landed[1].1,
        "Ryu's jab links on Lee: {landed:?}"
    );
    assert!(
        frames[landed[0].0 + 1..landed[1].0]
            .iter()
            .all(|f| f.p2_flags & 0x08 == 0),
        "Lee's reaction is hittable"
    );
    assert_eq!(
        frames[landed[1].0].combo,
        Some(ComboCount {
            hits: 2,
            damage: None
        })
    );
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());
}

#[test]
fn gen_follows_the_same_rule_on_his_bank_table() {
    if !Path::new(ROMS).is_dir() || !Path::new(PACK).is_dir() {
        eprintln!("skipping: local ROMs or art pack not present");
        return;
    }
    let a = load_assets(ROMS, MANIFEST).unwrap();
    let pack = ArtPack::open(Path::new(PACK)).expect("the art pack opens");
    let art = load_art_from(
        &a,
        &ArtSources {
            recipes: &GEN_RECIPES,
            reviews: Some(&GEN_REVIEWS),
            pack: Ok(&pack),
        },
    )
    .expect("the art loads");
    let session = || {
        GameSession::new(
            load_assets(ROMS, MANIFEST).unwrap(),
            MatchConfig::from_preset(MODERN),
        )
        .with_sprite_bank(&art.bank)
        .with_player_tables(&art.catalogues)
    };
    // His jab's hitstun is his row's (+5 on hit), and it links into a
    // second jab, the era HUD counting two.
    let mut s = local(session(), TournamentFighter::Gen, TournamentFighter::Ken);
    assert!(
        s.data_bank()
            .unwrap()
            .table(TournamentFighter::Gen)
            .is_some(),
        "Gen plays his bank table"
    );
    close_in(&mut s, 0x20);
    let want = move_stun(Some(TournamentFighter::Gen), 0x0d, 1);
    let mut single = s.clone();
    let frames = string(&mut single, LP, &[], &[], 80, 60);
    assert_eq!(
        frames.iter().filter(|f| f.landed.is_some()).count(),
        1,
        "his jab hits"
    );
    assert_eq!(
        stun_frames(&frames).0,
        usize::from(want.hit),
        "its hitstun is his row's"
    );
    let mut guarded = s.clone();
    let frames = string(&mut guarded, LP, &[], &[RIGHT], 80, 60);
    assert!(frames.iter().all(|f| f.landed.is_none()), "Ken guards");
    assert_eq!(
        stun_frames(&frames).1,
        usize::from(want.block),
        "its blockstun is his row's"
    );
    // Back Ken into the right corner so 0x32's pushback is stopped by the
    // stage wall and keeps him in the second jab's reach.
    for _ in 0..250 {
        s.step(with(|i| {
            i.p1.directions[RIGHT] = true;
            i.p2.directions[RIGHT] = true;
        }));
    }
    close_in(&mut s, 0x20);
    let frames = string(&mut s, LP, &[], &[], 2, 30);
    let landed = hits(&frames);
    assert!(
        landed.len() >= 2 && landed[1].1,
        "Gen's jab links: {landed:?}"
    );
    assert_eq!(
        (
            frames[landed[0].0].p1.move_id,
            frames[landed[1].0].p1.move_id
        ),
        (0x0d, 0x0d),
        "jab into jab"
    );
    assert_eq!(
        frames[landed[1].0].combo,
        Some(ComboCount {
            hits: 2,
            damage: None
        })
    );
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());

    // Gen as the defender: his guards take the blockstun of Ryu's medium
    // punch standing and his crouching short crouching, and his ground
    // reactions take boxes and can be hit.
    let mut s = local(session(), TournamentFighter::Ryu, TournamentFighter::Gen);
    close_in(&mut s, 0x30);
    for (button, dirs, guard, id, level, posture) in [
        (MP, &[][..], &[RIGHT][..], 0x0f, 2, 4),
        (LK, &[DOWN][..], &[DOWN, RIGHT][..], 0x17, 1, 8),
    ] {
        let mut guarded = s.clone();
        let frames = string(&mut guarded, button, dirs, guard, 80, 60);
        assert!(
            frames.iter().all(|f| f.landed.is_none()),
            "Gen guards move {id:#04x}"
        );
        let block = usize::from(move_stun(Some(TournamentFighter::Ryu), id, level).block);
        assert_eq!(
            stun_frames(&frames).1,
            block,
            "his guard takes move {id:#04x}'s blockstun"
        );
        let g = frames.iter().position(|f| f.p2.blockstun).unwrap();
        assert!(
            frames[g..g + block].iter().all(|f| f.p2.move_id == posture),
            "held in his guard"
        );
    }
    let frames = string(&mut s, LP, &[], &[], 2, 30);
    let landed = hits(&frames);
    assert!(
        landed.len() >= 2 && landed[1].1,
        "Ryu's jab links on Gen: {landed:?}"
    );
    assert!(
        frames[landed[0].0 + 1..landed[1].0]
            .iter()
            .all(|f| f.p2_flags & 0x08 == 0),
        "Gen's reaction is hittable"
    );
    assert_eq!(
        frames[landed[1].0].combo,
        Some(ComboCount {
            hits: 2,
            damage: None
        })
    );
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());
}

#[test]
fn birdie_follows_the_same_rule_on_his_bank_table() {
    if !Path::new(ROMS).is_dir() || !Path::new(PACK).is_dir() {
        eprintln!("skipping: local ROMs or art pack not present");
        return;
    }
    let a = load_assets(ROMS, MANIFEST).unwrap();
    let pack = ArtPack::open(Path::new(PACK)).expect("the art pack opens");
    let art = load_art_from(
        &a,
        &ArtSources {
            recipes: &BIRDIE_RECIPES,
            reviews: Some(&BIRDIE_REVIEWS),
            pack: Ok(&pack),
        },
    )
    .expect("the art loads");
    let session = || {
        GameSession::new(
            load_assets(ROMS, MANIFEST).unwrap(),
            MatchConfig::from_preset(MODERN),
        )
        .with_sprite_bank(&art.bank)
        .with_player_tables(&art.catalogues)
    };
    // His jab's hitstun is his row's (+6 on hit), and it links into a
    // second jab, the era HUD counting two.
    let mut s = local(session(), TournamentFighter::Birdie, TournamentFighter::Ken);
    assert!(
        s.data_bank()
            .unwrap()
            .table(TournamentFighter::Birdie)
            .is_some(),
        "Birdie plays his bank table"
    );
    close_in(&mut s, 0x20);
    let want = move_stun(Some(TournamentFighter::Birdie), 0x0d, 1);
    let mut single = s.clone();
    let frames = string(&mut single, LP, &[], &[], 80, 60);
    assert_eq!(
        frames.iter().filter(|f| f.landed.is_some()).count(),
        1,
        "his jab hits"
    );
    assert_eq!(
        stun_frames(&frames).0,
        usize::from(want.hit),
        "its hitstun is his row's"
    );
    let mut guarded = s.clone();
    let frames = string(&mut guarded, LP, &[], &[RIGHT], 80, 60);
    assert!(frames.iter().all(|f| f.landed.is_none()), "Ken guards");
    assert_eq!(
        stun_frames(&frames).1,
        usize::from(want.block),
        "its blockstun is his row's"
    );
    // Back Ken into the right corner so 0x32's pushback is stopped by the
    // stage wall and keeps him in the second jab's reach.
    for _ in 0..250 {
        s.step(with(|i| {
            i.p1.directions[RIGHT] = true;
            i.p2.directions[RIGHT] = true;
        }));
    }
    close_in(&mut s, 0x20);
    let frames = string(&mut s, LP, &[], &[], 2, 30);
    let landed = hits(&frames);
    assert!(
        landed.len() >= 2 && landed[1].1,
        "Birdie's jab links: {landed:?}"
    );
    assert_eq!(
        (
            frames[landed[0].0].p1.move_id,
            frames[landed[1].0].p1.move_id
        ),
        (0x0d, 0x0d),
        "jab into jab"
    );
    assert_eq!(
        frames[landed[1].0].combo,
        Some(ComboCount {
            hits: 2,
            damage: None
        })
    );
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());

    // Birdie as the defender: his guards take the blockstun of Ryu's medium
    // punch standing and his crouching short crouching, and his ground
    // reactions take boxes and can be hit.
    let mut s = local(session(), TournamentFighter::Ryu, TournamentFighter::Birdie);
    close_in(&mut s, 0x30);
    for (button, dirs, guard, id, level, posture) in [
        (MP, &[][..], &[RIGHT][..], 0x0f, 2, 4),
        (LK, &[DOWN][..], &[DOWN, RIGHT][..], 0x17, 1, 8),
    ] {
        let mut guarded = s.clone();
        let frames = string(&mut guarded, button, dirs, guard, 80, 60);
        assert!(
            frames.iter().all(|f| f.landed.is_none()),
            "Birdie guards move {id:#04x}"
        );
        let block = usize::from(move_stun(Some(TournamentFighter::Ryu), id, level).block);
        assert_eq!(
            stun_frames(&frames).1,
            block,
            "his guard takes move {id:#04x}'s blockstun"
        );
        let g = frames.iter().position(|f| f.p2.blockstun).unwrap();
        assert!(
            frames[g..g + block].iter().all(|f| f.p2.move_id == posture),
            "held in his guard"
        );
    }
    let frames = string(&mut s, LP, &[], &[], 2, 30);
    let landed = hits(&frames);
    assert!(
        landed.len() >= 2 && landed[1].1,
        "Ryu's jab links on Birdie: {landed:?}"
    );
    assert!(
        frames[landed[0].0 + 1..landed[1].0]
            .iter()
            .all(|f| f.p2_flags & 0x08 == 0),
        "Birdie's reaction is hittable"
    );
    assert_eq!(
        frames[landed[1].0].combo,
        Some(ComboCount {
            hits: 2,
            damage: None
        })
    );
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());
}

#[test]
fn eagle_follows_the_same_rule_on_his_bank_table() {
    if !Path::new(ROMS).is_dir() || !Path::new(PACK).is_dir() {
        eprintln!("skipping: local ROMs or art pack not present");
        return;
    }
    let a = load_assets(ROMS, MANIFEST).unwrap();
    let pack = ArtPack::open(Path::new(PACK)).expect("the art pack opens");
    let art = load_art_from(
        &a,
        &ArtSources {
            recipes: &EAGLE_RECIPES,
            reviews: Some(&EAGLE_REVIEWS),
            pack: Ok(&pack),
        },
    )
    .expect("the art loads");
    let session = || {
        GameSession::new(
            load_assets(ROMS, MANIFEST).unwrap(),
            MatchConfig::from_preset(MODERN),
        )
        .with_sprite_bank(&art.bank)
        .with_player_tables(&art.catalogues)
    };
    // His 1987 stick jab (0x1239e) knocks a standing Ken back (0x33..=0x35,
    // airborne) and swings above a crouch; his 1987 low stick thrust (0x11,
    // 0x12628) hits a crouching Ken with the ground reaction 0x3b, and his
    // crouching stick sweep (0x17) hits a standing Ken on the ground, each
    // taking its row's hitstun and blockstun.
    let mut s = local(session(), TournamentFighter::Eagle, TournamentFighter::Ken);
    assert!(
        s.data_bank()
            .unwrap()
            .table(TournamentFighter::Eagle)
            .is_some(),
        "Eagle plays his bank table"
    );
    close_in(&mut s, 0x28);
    for (button, dirs, hurt, guard, id, level) in [
        (LP, &[DOWN][..], &[DOWN][..], &[DOWN, RIGHT][..], 0x11, 1),
        (LK, &[DOWN][..], &[][..], &[DOWN, RIGHT][..], 0x17, 1),
    ] {
        let want = move_stun(Some(TournamentFighter::Eagle), id, level);
        let mut single = s.clone();
        let frames = string(&mut single, button, dirs, hurt, 80, 60);
        assert_eq!(
            frames.iter().filter(|f| f.landed.is_some()).count(),
            1,
            "move {id:#04x} hits"
        );
        assert_eq!(
            stun_frames(&frames).0,
            usize::from(want.hit),
            "move {id:#04x}'s hitstun is his row's"
        );
        let mut guarded = s.clone();
        let frames = string(&mut guarded, button, dirs, guard, 80, 60);
        assert!(
            frames.iter().all(|f| f.landed.is_none()),
            "Ken guards move {id:#04x}"
        );
        assert_eq!(
            stun_frames(&frames).1,
            usize::from(want.block),
            "move {id:#04x}'s blockstun is his row's"
        );
    }
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());

    // Eagle as the defender: his guards take the blockstun of Ryu's medium
    // punch standing and his crouching short crouching, and his ground
    // reactions take boxes and can be hit.
    let mut s = local(session(), TournamentFighter::Ryu, TournamentFighter::Eagle);
    close_in(&mut s, 0x30);
    for (button, dirs, guard, id, level, posture) in [
        (MP, &[][..], &[RIGHT][..], 0x0f, 2, 4),
        (LK, &[DOWN][..], &[DOWN, RIGHT][..], 0x17, 1, 8),
    ] {
        let mut guarded = s.clone();
        let frames = string(&mut guarded, button, dirs, guard, 80, 60);
        assert!(
            frames.iter().all(|f| f.landed.is_none()),
            "Eagle guards move {id:#04x}"
        );
        let block = usize::from(move_stun(Some(TournamentFighter::Ryu), id, level).block);
        assert_eq!(
            stun_frames(&frames).1,
            block,
            "his guard takes move {id:#04x}'s blockstun"
        );
        let g = frames.iter().position(|f| f.p2.blockstun).unwrap();
        assert!(
            frames[g..g + block].iter().all(|f| f.p2.move_id == posture),
            "held in his guard"
        );
    }
    let frames = string(&mut s, LP, &[], &[], 2, 30);
    let landed = hits(&frames);
    assert!(
        landed.len() >= 2 && landed[1].1,
        "Ryu's jab links on Eagle: {landed:?}"
    );
    assert!(
        frames[landed[0].0 + 1..landed[1].0]
            .iter()
            .all(|f| f.p2_flags & 0x08 == 0),
        "Eagle's reaction is hittable"
    );
    assert_eq!(
        frames[landed[1].0].combo,
        Some(ComboCount {
            hits: 2,
            damage: None
        })
    );
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());
}

#[test]
fn adon_follows_the_same_rule_on_his_bank_table() {
    if !Path::new(ROMS).is_dir() || !Path::new(PACK).is_dir() {
        eprintln!("skipping: local ROMs or art pack not present");
        return;
    }
    let a = load_assets(ROMS, MANIFEST).unwrap();
    let pack = ArtPack::open(Path::new(PACK)).expect("the art pack opens");
    let art = load_art_from(
        &a,
        &ArtSources {
            recipes: &ADON_RECIPES,
            reviews: Some(&ADON_REVIEWS),
            pack: Ok(&pack),
        },
    )
    .expect("the art loads");
    let session = || {
        GameSession::new(
            load_assets(ROMS, MANIFEST).unwrap(),
            MatchConfig::from_preset(MODERN),
        )
        .with_sprite_bank(&art.bank)
        .with_player_tables(&art.catalogues)
    };
    // His crouching jab (0x11) and crouching kick (0x17) hit a standing Ken
    // on the ground and take his row's hitstun and blockstun.
    let mut s = local(session(), TournamentFighter::Adon, TournamentFighter::Ken);
    assert!(
        s.data_bank()
            .unwrap()
            .table(TournamentFighter::Adon)
            .is_some(),
        "Adon plays his bank table"
    );
    close_in(&mut s, 0x28);
    for (button, dirs, hurt, guard, id, level) in [
        (LP, &[DOWN][..], &[][..], &[DOWN, RIGHT][..], 0x11, 1),
        (LK, &[DOWN][..], &[][..], &[DOWN, RIGHT][..], 0x17, 1),
    ] {
        let want = move_stun(Some(TournamentFighter::Adon), id, level);
        let mut single = s.clone();
        let frames = string(&mut single, button, dirs, hurt, 80, 60);
        assert_eq!(
            frames.iter().filter(|f| f.landed.is_some()).count(),
            1,
            "move {id:#04x} hits"
        );
        assert_eq!(
            stun_frames(&frames).0,
            usize::from(want.hit),
            "move {id:#04x}'s hitstun is his row's"
        );
        let mut guarded = s.clone();
        let frames = string(&mut guarded, button, dirs, guard, 80, 60);
        assert!(
            frames.iter().all(|f| f.landed.is_none()),
            "Ken guards move {id:#04x}"
        );
        assert_eq!(
            stun_frames(&frames).1,
            usize::from(want.block),
            "move {id:#04x}'s blockstun is his row's"
        );
    }
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());

    // Adon as the defender: his guards take the blockstun of Ryu's medium
    // punch standing and his crouching short crouching, and his ground
    // reactions take boxes and can be hit.
    let mut s = local(session(), TournamentFighter::Ryu, TournamentFighter::Adon);
    close_in(&mut s, 0x30);
    for (button, dirs, guard, id, level, posture) in [
        (MP, &[][..], &[RIGHT][..], 0x0f, 2, 4),
        (LK, &[DOWN][..], &[DOWN, RIGHT][..], 0x17, 1, 8),
    ] {
        let mut guarded = s.clone();
        let frames = string(&mut guarded, button, dirs, guard, 80, 60);
        assert!(
            frames.iter().all(|f| f.landed.is_none()),
            "Adon guards move {id:#04x}"
        );
        let block = usize::from(move_stun(Some(TournamentFighter::Ryu), id, level).block);
        assert_eq!(
            stun_frames(&frames).1,
            block,
            "his guard takes move {id:#04x}'s blockstun"
        );
        let g = frames.iter().position(|f| f.p2.blockstun).unwrap();
        assert!(
            frames[g..g + block].iter().all(|f| f.p2.move_id == posture),
            "held in his guard"
        );
    }
    let frames = string(&mut s, LP, &[], &[], 2, 30);
    let landed = hits(&frames);
    assert!(
        landed.len() >= 2 && landed[1].1,
        "Ryu's jab links on Adon: {landed:?}"
    );
    assert!(
        frames[landed[0].0 + 1..landed[1].0]
            .iter()
            .all(|f| f.p2_flags & 0x08 == 0),
        "Adon's reaction is hittable"
    );
    assert_eq!(
        frames[landed[1].0].combo,
        Some(ComboCount {
            hits: 2,
            damage: None
        })
    );
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());
}

#[test]
fn sagat_follows_the_same_rule_on_his_bank_table() {
    if !Path::new(ROMS).is_dir() || !Path::new(PACK).is_dir() {
        eprintln!("skipping: local ROMs or art pack not present");
        return;
    }
    let a = load_assets(ROMS, MANIFEST).unwrap();
    let pack = ArtPack::open(Path::new(PACK)).expect("the art pack opens");
    let art = load_art_from(
        &a,
        &ArtSources {
            recipes: &SAGAT_RECIPES,
            reviews: Some(&SAGAT_REVIEWS),
            pack: Ok(&pack),
        },
    )
    .expect("the art loads");
    let session = || {
        GameSession::new(
            load_assets(ROMS, MANIFEST).unwrap(),
            MatchConfig::from_preset(MODERN),
        )
        .with_sprite_bank(&art.bank)
        .with_player_tables(&art.catalogues)
    };
    // His crouching jab (0x11) and crouching kick (0x17) hit a standing Ken
    // on the ground and take his row's hitstun and blockstun.
    let mut s = local(session(), TournamentFighter::Sagat, TournamentFighter::Ken);
    assert!(
        s.data_bank()
            .unwrap()
            .table(TournamentFighter::Sagat)
            .is_some(),
        "Sagat plays his bank table"
    );
    close_in(&mut s, 0x28);
    let heavy = string(&mut s.clone(), HP, &[], &[], 80, 60);
    assert_eq!(hits(&heavy).len(), 1);
    assert!(heavy.iter().all(|frame| frame.heavy.is_some() == frame.landed.is_some()));
    for (button, dirs, hurt, guard, id, level) in [
        (LP, &[DOWN][..], &[][..], &[DOWN, RIGHT][..], 0x11, 1),
        (LK, &[DOWN][..], &[][..], &[DOWN, RIGHT][..], 0x17, 1),
    ] {
        let want = move_stun(Some(TournamentFighter::Sagat), id, level);
        let mut single = s.clone();
        let frames = string(&mut single, button, dirs, hurt, 80, 60);
        assert_eq!(
            frames.iter().filter(|f| f.landed.is_some()).count(),
            1,
            "move {id:#04x} hits"
        );
        assert_eq!(
            stun_frames(&frames).0,
            usize::from(want.hit),
            "move {id:#04x}'s hitstun is his row's"
        );
        let mut guarded = s.clone();
        let frames = string(&mut guarded, button, dirs, guard, 80, 60);
        assert!(
            frames.iter().all(|f| f.landed.is_none()),
            "Ken guards move {id:#04x}"
        );
        assert_eq!(
            stun_frames(&frames).1,
            usize::from(want.block),
            "move {id:#04x}'s blockstun is his row's"
        );
    }
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());

    // Sagat as the defender: his guards take the blockstun of Ryu's medium
    // punch standing and his crouching short crouching, and his ground
    // reactions take boxes and can be hit.
    let mut s = local(session(), TournamentFighter::Ryu, TournamentFighter::Sagat);
    close_in(&mut s, 0x30);
    for (button, dirs, guard, id, level, posture) in [
        (MP, &[][..], &[RIGHT][..], 0x0f, 2, 4),
        (LK, &[DOWN][..], &[DOWN, RIGHT][..], 0x17, 1, 8),
    ] {
        let mut guarded = s.clone();
        let frames = string(&mut guarded, button, dirs, guard, 80, 60);
        assert!(
            frames.iter().all(|f| f.landed.is_none()),
            "Sagat guards move {id:#04x}"
        );
        let block = usize::from(move_stun(Some(TournamentFighter::Ryu), id, level).block);
        assert_eq!(
            stun_frames(&frames).1,
            block,
            "his guard takes move {id:#04x}'s blockstun"
        );
        let g = frames.iter().position(|f| f.p2.blockstun).unwrap();
        assert!(
            frames[g..g + block].iter().all(|f| f.p2.move_id == posture),
            "held in his guard"
        );
    }
    let frames = string(&mut s, LP, &[], &[], 2, 30);
    let landed = hits(&frames);
    assert!(
        landed.len() >= 2 && landed[1].1,
        "Ryu's jab links on Sagat: {landed:?}"
    );
    assert!(
        frames[landed[0].0 + 1..landed[1].0]
            .iter()
            .all(|f| f.p2_flags & 0x08 == 0),
        "Sagat's reaction is hittable"
    );
    assert_eq!(
        frames[landed[1].0].combo,
        Some(ComboCount {
            hits: 2,
            damage: None
        })
    );
    assert!(s.fight_machine().unwrap().unrecovered().is_empty());
}

/// A session for `p1` against `p2` (both on their bank tables when they
/// have one) under `config`.
fn bank_session(config: MatchConfig, p1: TournamentFighter, p2: TournamentFighter) -> Option<GameSession> {
    if assets().is_none() || !Path::new(PACK).is_dir() {
        eprintln!("skipping: the local ROM set or art pack is not present");
        return None;
    }
    let a = load_assets(ROMS, MANIFEST).unwrap();
    let pack = ArtPack::open(Path::new(PACK)).expect("the art pack opens");
    let (recipes, reviews) = match (p1, p2) {
        (TournamentFighter::Geki, _) | (_, TournamentFighter::Geki) => (&GEKI_RECIPES, &GEKI_REVIEWS),
        _ => (&JOE_RECIPES, &JOE_REVIEWS),
    };
    let art = load_art_from(
        &a,
        &ArtSources {
            recipes,
            reviews: Some(reviews),
            pack: Ok(&pack),
        },
    )
    .expect("the art loads");
    let s = GameSession::new(load_assets(ROMS, MANIFEST).unwrap(), config)
        .with_sprite_bank(&art.bank)
        .with_player_tables(&art.catalogues);
    Some(local(s, p1, p2))
}

#[test]
fn balanced_roster_gives_joe_a_standing_guard_and_classic_keeps_the_roms_duck() {
    // the maintainer (#268): Joe's ROM block pose's hurtbox is the lower two thirds
    // of his grid, so Ryu's standing punches and medium kick passed over his
    // guard. With Balanced roster on it takes his standing hurtbox and every
    // one is guarded for its row's blockstun; off, the ROM's box again.
    for balanced in [true, false] {
        let config = MatchConfig::from_preset(MODERN).with_balanced_roster(balanced);
        let Some(mut s) = bank_session(config, TournamentFighter::Ryu, TournamentFighter::Joe) else { return };
        close_in(&mut s, 0x30);
        for (button, id) in [(LP, 0x0d), (MP, 0x0f), (HP, 0x10), (LK, 0x14), (MK, 0x15)] {
            let mut g = s.clone();
            let frames = string(&mut g, button, &[], &[RIGHT], 80, 60);
            assert!(frames.iter().all(|f| f.landed.is_none()), "Joe guards {id:#04x}");
            let want = usize::from(move_stun(Some(TournamentFighter::Ryu), id, 1).block);
            let got = stun_frames(&frames).1;
            if balanced {
                assert_eq!(got, want, "Balanced: his guard takes {id:#04x}'s blockstun");
            } else if id != 0x14 {
                assert_eq!(got, 0, "Classic: {id:#04x} passes over the ROM's guard");
            }
        }
    }
}

#[test]
fn balanced_roster_lets_gekis_normals_reach_a_guard_from_his_closest_spacing() {
    // the maintainer (#268, seen in SP17): Geki's forward walk draws him 24 px ahead
    // of where he stands and its body kept that drawing apart from Ken, so
    // walking in he stopped short and his light normals and front kick
    // whiffed a guarding Ken. Balanced roster pulls the walk's body back by
    // the offset; every normal then meets the guard.
    for balanced in [true, false] {
        let config = MatchConfig::from_preset(MODERN).with_balanced_roster(balanced);
        let Some(mut s) = bank_session(config, TournamentFighter::Geki, TournamentFighter::Ken) else { return };
        for _ in 0..150 {
            s.step(with(|i| i.p1.directions[RIGHT] = true));
        }
        for _ in 0..4 {
            s.step(idle());
        }
        let mut reached = 0;
        let cases = [(LP, &[][..]), (MP, &[][..]), (HP, &[][..]), (LK, &[][..]), (MK, &[][..]), (LP, &[DOWN][..]), (LK, &[DOWN][..])];
        for (button, dirs) in cases {
            let mut g = s.clone();
            let guard: &[usize] = if dirs.is_empty() { &[RIGHT] } else { &[DOWN, RIGHT] };
            let frames = string(&mut g, button, dirs, guard, 80, 60);
            assert!(frames.iter().all(|f| f.landed.is_none()), "Ken guards");
            if stun_frames(&frames).1 > 0 {
                reached += 1;
            }
        }
        if balanced {
            assert_eq!(reached, cases.len(), "Balanced: every normal meets the guard");
        } else {
            assert!(reached < cases.len(), "Classic: the ROM's walk leaves some short ({reached})");
        }
    }
}
