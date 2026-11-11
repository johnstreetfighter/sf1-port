//! Ticket #263 (`SP19`): throws and throw techs, the third of the modern rules.
//!
//! At the headless session's seam (inputs in; the presented frames and the
//! fight reader out), in Local fights:
//! - With the throws switch on (`Definitive Overhaul`, `Pure Footsies`),
//!   pressing forward or back and heavy punch or heavy kick in throw range
//!   (`<= 48` pixels centre-to-centre) starts the fighter's throw from his
//!   catalogue (`THROW_MOVE`) and the victim's being-thrown set
//!   (`THROWN_MOVE`); outside throw range the normal heavy attack (`0x10` /
//!   `0x16`) comes out.
//! - A throw that connects without a tech plays the thrower's throw and the
//!   victim's being-thrown arc, slams the victim to the mat for `DAMAGE` (`6`
//!   HP), runs his catalogue's `"get-up"` (`GET_UP_MOVE`), and grants
//!   `WAKEUP_INVULN` (`13`) frames of post-wakeup throw invulnerability (or
//!   knocks him out when his health reaches zero). Authored cross-over throws
//!   carry the victim over the thrower for either input; front shoves/slams
//!   reverse only on back. Crossing follows the arc instead of teleporting.
//! - Pressing throw during the 7-frame tech window (`TECH_WINDOW`) techs the
//!   throw: both fighters return to standing (`move 1`), push apart, and take
//!   no damage. Pressing throw after the window closes does not tech.
//! - An opponent in hitstun, blockstun, airborne, in `"get-up"`, or in his
//!   post-wakeup throw invulnerability cannot be thrown.
//! - With the throws switch off (`Classic Competitive`, `Faithful 1987`), no
//!   input throws and `ThrowState` stays at its default.

use std::path::Path;

use sf_engine::combo::throw::{
    ThrowState, DAMAGE, GET_UP_MOVE, RANGE, TECH_WINDOW, THROWN_MOVE, THROW_MOVE, WAKEUP_INVULN,
};
use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::fight_reader::FightRead;
use sf_engine::fighter::{P1_STRUCT, P2_STRUCT};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::{FrameInputs, FrameOutput, GameSession, Transition};
use sf_engine::roster::TournamentFighter;
use sf_video::art::{load_art_from, ArtSources};
use sf_video::art_pack::ArtPack;
use sf_video::callouts::Side;
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const PACK: &str = "../../tools/reference/local/roms/sf1-port-art";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const RECIPES: [&str; 12] = [
    include_str!("../../../art/recipes/ryu.json"),
    include_str!("../../../art/recipes/ken.json"),
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
const REVIEWS: [&str; 12] = [
    include_str!("../../../art/review/ryu.json"),
    include_str!("../../../art/review/ken.json"),
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
const UP: usize = 3;
const LP: usize = 0;
const HP: usize = 2;
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
        tap(&mut s, with(|i| i.menu.confirm = true)).transition,
        Some(Transition::Picked(p1))
    );
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
    let out = s.step(with(|i| i.p2.buttons[0] = true));
    assert_eq!(out.transition, Some(Transition::Fighters(p1, p2)));
    s.step(idle());
    until(&mut s, "country select", 40);
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    until(&mut s, "fight", 40);
    to_control(&mut s);
    s
}

fn match_with_art(
    config: MatchConfig,
    p1: TournamentFighter,
    p2: TournamentFighter,
) -> Option<GameSession> {
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
    let s = GameSession::new(a, config)
        .with_sprite_bank(&art.bank)
        .with_player_tables(&art.catalogues);
    Some(local(s, p1, p2))
}

fn read(s: &GameSession) -> FightRead {
    *s.fight().unwrap().fight_read()
}

fn entity(s: &GameSession, ps: u32) -> usize {
    let w = s.fight_machine().unwrap().work_ram();
    let a = (ps + 0x10 - 0xff8000) as usize;
    u16::from_be_bytes([w[a], w[a + 1]]) as usize
}

fn center_x(s: &GameSession, ps: u32) -> u16 {
    let e = entity(s, ps);
    let obj = &s.fight_machine().unwrap().state.objectram;
    let sx = u16::from_be_bytes([obj[e + 6], obj[e + 7]]);
    let cols = u16::from(obj[e + 0xd]);
    let attr = u16::from_be_bytes([obj[e + 2], obj[e + 3]]);
    let cell = if attr & 0x400 != 0 { 32 } else { 16 };
    sx.wrapping_add((cols + 1) * cell / 2)
}

/// The fighter's world origin, excluding a pose's authored horizontal
/// offset. A large arm/weapon grid or a mirrored anchor can change its
/// drawn centre without moving the fighter through the stage.
fn logical_x(s: &GameSession, ps: u32) -> i32 {
    let e = entity(s, ps);
    let offset = s.fight_machine().unwrap().state.objectram[e + 0x26] as i8;
    i32::from(center_x(s, ps)) - i32::from(offset)
}

/// The rendered grid's source pose and horizontal mirror flag. Mirroring
/// also reverses its cell codes, so undo that ordering to identify a pose.
fn displayed_pose(s: &GameSession, ps: u32) -> (u16, bool) {
    let e = entity(s, ps);
    let obj = &s.fight_machine().unwrap().state.objectram;
    let attr = u16::from_be_bytes([obj[e + 2], obj[e + 3]]);
    let code = u16::from_be_bytes([obj[e], obj[e + 1]]);
    let mirrored = attr & 0x100 != 0;
    let span = u16::from(obj[e + 0xd]) * if attr & 0x400 != 0 { 2 } else { 1 };
    (
        if mirrored {
            code.wrapping_sub(span)
        } else {
            code
        },
        mirrored,
    )
}

fn facing_left(s: &GameSession, ps: u32) -> bool {
    s.fight_machine().unwrap().state.objectram[entity(s, ps) + 0x37] & 4 != 0
}

/// Contact can separate the overlapping bodies when the caught victim lands.
/// Bound that correction by the current combined body widths, independently
/// of the stricter continuity check while the victim is carried through air.
fn contact_span(s: &GameSession, first: u32, second: u32) -> u32 {
    let (a, b) = (entity(s, first), entity(s, second));
    let obj = &s.fight_machine().unwrap().state.objectram;
    u32::from(obj[a + 0x18].max(obj[a + 0x19])) + u32::from(obj[b + 0x18].max(obj[b + 0x19]))
}

fn distance(s: &GameSession) -> u16 {
    center_x(s, P1_STRUCT).abs_diff(center_x(s, P2_STRUCT))
}

fn p1_toward_p2(s: &GameSession) -> usize {
    if center_x(s, P1_STRUCT) < center_x(s, P2_STRUCT) {
        RIGHT
    } else {
        LEFT
    }
}

/// Select a real approach frame at which the requested throw can start.
/// A walking pose can change the drawn centre before input dispatch, so a
/// pre-step distance alone does not establish an eligible throw fixture.
fn enter_throw(close: &GameSession, side: Side, direction: usize) -> Option<GameSession> {
    let mut approach = close.clone();
    for _ in 0..=16 {
        let mut candidate = approach.clone();
        candidate.step(with(|i| {
            let pad = if side == Side::P1 {
                &mut i.p1
            } else {
                &mut i.p2
            };
            pad.directions[direction] = true;
            pad.buttons[HP] = true;
        }));
        if read(&candidate).fighter(side).move_id == THROW_MOVE {
            return Some(candidate);
        }
        let (me, other) = if side == Side::P1 {
            (P1_STRUCT, P2_STRUCT)
        } else {
            (P2_STRUCT, P1_STRUCT)
        };
        let inward = if center_x(&approach, me) < center_x(&approach, other) {
            RIGHT
        } else {
            LEFT
        };
        approach.step(with(|i| {
            let pad = if side == Side::P1 {
                &mut i.p1
            } else {
                &mut i.p2
            };
            pad.directions[inward] = true;
        }));
    }
    None
}

/// Walk P2 toward P1 until they are within throw range, including after a
/// crossing throw has exchanged their sides, then let both return to idle.
fn walk_to_throw_range(s: &mut GameSession) {
    for _ in 0..200 {
        if distance(s) <= 40 {
            break;
        }
        let inward = if center_x(s, P2_STRUCT) > center_x(s, P1_STRUCT) {
            LEFT
        } else {
            RIGHT
        };
        s.step(with(|i| i.p2.directions[inward] = true));
    }
    for _ in 0..16 {
        s.step(idle());
    }
    assert!(
        distance(s) <= RANGE,
        "fighters walked into throw range (distance {}, positions=({}, {}), fighters={:?}/{:?}, moves={:?})",
        distance(s), center_x(s, P1_STRUCT), center_x(s, P2_STRUCT),
        s.fight_machine().unwrap().match_config().roster.p1,
        s.fight_machine().unwrap().match_config().roster.p2,
        read(s).fighters.map(|f| (f.move_id, f.state))
    );
}

/// Walk toward a wall together, waiting for the slower fighter instead of
/// letting the pair drift out of throw range. Short stick releases exercise
/// ordinary walking while allowing first-step-only boss walks to keep up.
fn walk_pair_to_edge(s: &mut GameSession, direction: usize, attacker: Side) {
    let mut last_scroll = s.fight_machine().unwrap().state.bg_scroll;
    let mut still_scroll = 0;
    for frame in 0..720 {
        let scroll = s.fight_machine().unwrap().state.bg_scroll;
        still_scroll = if scroll == last_scroll {
            still_scroll + 1
        } else {
            0
        };
        last_scroll = scroll;
        if still_scroll >= 24 && distance(s) <= RANGE && edge_distance(s, direction) <= 8 {
            break;
        }
        let leading_p1 = if direction == LEFT {
            center_x(s, P1_STRUCT) < center_x(s, P2_STRUCT)
        } else {
            center_x(s, P1_STRUCT) > center_x(s, P2_STRUCT)
        };
        let together = distance(s) <= RANGE;
        s.step(with(|i| {
            if frame % 4 != 3 {
                i.p1.directions[direction] = !leading_p1 || together;
                i.p2.directions[direction] = leading_p1 || together;
            }
        }));
    }
    for _ in 0..16 {
        s.step(idle());
    }
    // Some idle poses change the drawn centre after the walk. Approach by
    // real pad input until the throw can be entered, retaining the wall.
    for _ in 0..32 {
        if distance(s) <= RANGE {
            break;
        }
        let (me, other) = if attacker == Side::P1 {
            (P1_STRUCT, P2_STRUCT)
        } else {
            (P2_STRUCT, P1_STRUCT)
        };
        let inward = if center_x(s, me) < center_x(s, other) {
            RIGHT
        } else {
            LEFT
        };
        s.step(with(|i| {
            let pad = if attacker == Side::P1 {
                &mut i.p1
            } else {
                &mut i.p2
            };
            pad.directions[inward] = true;
        }));
    }
    assert!(
        distance(s) <= RANGE,
        "wall fixture remains within throw range: {}, fighters={:?}/{:?}, positions=({}, {})",
        distance(s),
        s.fight_machine().unwrap().match_config().roster.p1,
        s.fight_machine().unwrap().match_config().roster.p2,
        center_x(s, P1_STRUCT),
        center_x(s, P2_STRUCT)
    );
    let remaining = edge_distance(s, direction);
    assert!(remaining <= 24, "wall fixture reaches the edge: remaining={remaining}, fighters={:?}/{:?}, wall={direction}, positions=({}, {})", s.fight_machine().unwrap().match_config().roster.p1, s.fight_machine().unwrap().match_config().roster.p2, center_x(s, P1_STRUCT), center_x(s, P2_STRUCT));
}

fn edge_distance(s: &GameSession, direction: usize) -> u16 {
    let leading_ps = if direction == LEFT {
        P1_STRUCT
    } else {
        P2_STRUCT
    };
    let e = entity(s, leading_ps);
    let o = &s.fight_machine().unwrap().state.objectram;
    let left = o[e + 0x37] & 4 != 0;
    let cx = center_x(s, leading_ps);
    if direction == LEFT {
        let bound = if left { 0x80 } else { 0x60 };
        let half = u16::from(o[e + if left { 0x18 } else { 0x19 }]);
        cx.saturating_sub(bound + half)
    } else {
        let bound: u16 = if left { 0x1a0 } else { 0x180 };
        let half = u16::from(o[e + if left { 0x19 } else { 0x18 }]);
        bound.saturating_sub(cx + half)
    }
}

#[test]
fn forward_and_back_throws_start_in_range_and_heavy_normals_outside_range() {
    let Some(mut s) = match_with_art(
        MatchConfig::from_preset(MODERN),
        TournamentFighter::Ryu,
        TournamentFighter::Ken,
    ) else {
        return;
    };
    // At round-start distance (outside RANGE = 48), Forward + HP and Back + HK
    // start the standing heavy punch (0x10) and standing heavy kick (0x16).
    assert!(distance(&s) > RANGE);
    let far = s.clone();
    s.step(with(|i| {
        i.p1.directions[RIGHT] = true;
        i.p1.buttons[HP] = true;
    }));
    let r = read(&s);
    assert_eq!(
        r.fighter(Side::P1).move_id,
        0x10,
        "Forward + HP outside range is standing HP"
    );
    assert_eq!(r.fighter(Side::P2).move_id, 1);

    let mut s = far.clone();
    s.step(with(|i| {
        i.p1.directions[LEFT] = true;
        i.p1.buttons[HK] = true;
    }));
    let r = read(&s);
    assert_eq!(
        r.fighter(Side::P1).move_id,
        0x16,
        "Back + HK outside range is standing HK"
    );

    // Walk into throw range: Forward + HP, Forward + HK, Back + HP, and
    // Back + HK all start THROW_MOVE on P1 and THROWN_MOVE on P2.
    let mut close = far;
    walk_to_throw_range(&mut close);

    for (dir, btn, label) in [
        (RIGHT, HP, "Forward + HP"),
        (RIGHT, HK, "Forward + HK"),
        (LEFT, HP, "Back + HP"),
        (LEFT, HK, "Back + HK"),
    ] {
        let mut s = close.clone();
        s.step(with(|i| {
            i.p1.directions[dir] = true;
            i.p1.buttons[btn] = true;
        }));
        let r = read(&s);
        assert_eq!(
            r.fighter(Side::P1).move_id,
            THROW_MOVE,
            "{label} in throw range starts P1's throw"
        );
        assert!(r.fighter(Side::P1).attacking());
        assert_eq!(
            r.fighter(Side::P2).move_id,
            THROWN_MOVE,
            "{label} in throw range puts P2 into his being-thrown reaction"
        );
        assert!(r.fighter(Side::P2).reacting());
        assert!(
            !r.fighter(Side::P2).hitstun(),
            "a throw is not combo-able hitstun"
        );
    }
}

#[test]
fn shoulder_throws_carry_the_victim_across_the_thrower_without_a_side_switch_teleport() {
    let mut failures = Vec::new();
    for thrower in [TournamentFighter::Ryu, TournamentFighter::Ken] {
        let opponent = if thrower == TournamentFighter::Ryu {
            TournamentFighter::Ken
        } else {
            TournamentFighter::Ryu
        };
        for side in Side::BOTH {
            let pair = if side == Side::P1 {
                (thrower, opponent)
            } else {
                (opponent, thrower)
            };
            let Some(mut close) = match_with_art(MatchConfig::from_preset(MODERN), pair.0, pair.1)
            else {
                return;
            };
            walk_to_throw_range(&mut close);
            let (attacker_ps, victim_ps, victim_side) = if side == Side::P1 {
                (P1_STRUCT, P2_STRUCT, Side::P2)
            } else {
                (P2_STRUCT, P1_STRUCT, Side::P1)
            };
            let initial_side =
                (logical_x(&close, victim_ps) - logical_x(&close, attacker_ps)).signum();
            for back in [false, true] {
                let mut s = close.clone();
                let forward = if initial_side > 0 { RIGHT } else { LEFT };
                let dir = if back {
                    if forward == RIGHT {
                        LEFT
                    } else {
                        RIGHT
                    }
                } else {
                    forward
                };
                s.step(with(|i| {
                    let pad = if side == Side::P1 {
                        &mut i.p1
                    } else {
                        &mut i.p2
                    };
                    pad.directions[dir] = true;
                    pad.buttons[HP] = true;
                }));
                assert_eq!(
                    read(&s).fighter(side).move_id,
                    THROW_MOVE,
                    "{thrower:?}/{side:?}/{back}: starts throw"
                );
                assert_eq!(read(&s).fighter(victim_side).move_id, THROWN_MOVE);
                let mut previous_x = logical_x(&s, victim_ps);
                let mut largest_step = 0;
                let mut crossed_before_slam = false;
                let mut slammed = false;
                let mut path = Vec::new();
                for frame in 0..100 {
                    s.step(idle());
                    let (ax, vx) = (logical_x(&s, attacker_ps), logical_x(&s, victim_ps));
                    largest_step = largest_step.max(previous_x.abs_diff(vx));
                    previous_x = vx;
                    let impact = read(&s).landed_by(side).is_some();
                    if !slammed && !impact && (vx - ax).signum() == -initial_side {
                        crossed_before_slam = true;
                    }
                    slammed |= impact;
                    if frame < 35 {
                        path.push((frame, ax, vx));
                    }
                }
                let final_side = (logical_x(&s, victim_ps) - logical_x(&s, attacker_ps)).signum();
                if !crossed_before_slam || final_side != -initial_side || largest_step > 16 {
                    failures.push(format!("{thrower:?}/{side:?}/back={back}: crossed_before_slam={crossed_before_slam}, final_side={final_side}, expected={}, largest_victim_step={largest_step}, path={path:?}", -initial_side));
                }
                assert_eq!(read(&s).fighter(victim_side).health, 48 - u16::from(DAMAGE));
            }
        }
    }
    assert!(failures.is_empty(), "shoulder throws must carry the victim continuously over the thrower's shoulder to the opposite original side:\n{}", failures.join("\n"));
}

fn back_turned_throwers_face_the_landed_victim(throwers: &[TournamentFighter]) {
    let mut failures = Vec::new();
    for &thrower in throwers {
        let opponent = if thrower == TournamentFighter::Ryu {
            TournamentFighter::Ken
        } else {
            TournamentFighter::Ryu
        };
        for side in Side::BOTH {
            let pair = if side == Side::P1 {
                (thrower, opponent)
            } else {
                (opponent, thrower)
            };
            let Some(mut close) = match_with_art(MatchConfig::from_preset(MODERN), pair.0, pair.1)
            else {
                return;
            };
            walk_to_throw_range(&mut close);
            let (attacker_ps, victim_ps, victim_side) = if side == Side::P1 {
                (P1_STRUCT, P2_STRUCT, Side::P2)
            } else {
                (P2_STRUCT, P1_STRUCT, Side::P1)
            };
            let initial_left = facing_left(&close, attacker_ps);
            let original_side =
                (logical_x(&close, victim_ps) - logical_x(&close, attacker_ps)).signum();
            for back in [false, true] {
                let forward = if original_side > 0 { RIGHT } else { LEFT };
                let direction = if back {
                    if forward == RIGHT {
                        LEFT
                    } else {
                        RIGHT
                    }
                } else {
                    forward
                };
                let mut s =
                    enter_throw(&close, side, direction).expect("real shoulder throw starts");
                let grab_code = displayed_pose(&s, attacker_ps).0;
                let mut saw_turn = false;
                let mut saw_slam = false;
                let mut saw_get_up = false;
                let mut turn_poses = Vec::new();
                let mut incorrect_pose_frames = Vec::new();
                let mut trace = Vec::new();
                let mut previous = None;
                for frame in 0..=100 {
                    let r = read(&s);
                    let (code, mirrored) = displayed_pose(&s, attacker_ps);
                    let heading_left = facing_left(&s, attacker_ps);
                    let impact = r.landed_by(side).is_some();
                    let phase = if impact {
                        "slam"
                    } else if r.fighter(victim_side).getting_up {
                        "get-up"
                    } else if r.fighter(side).move_id == THROW_MOVE && code == grab_code {
                        "grab"
                    } else if r.fighter(side).move_id == THROW_MOVE {
                        "turn/heave"
                    } else {
                        "recovery"
                    };
                    let pose = (phase, code, mirrored, heading_left);
                    if previous != Some(pose) {
                        trace.push((frame, phase, format!("{code:#06x}"), mirrored, heading_left));
                        previous = Some(pose);
                    }
                    let authored_turn = r.fighter(side).move_id == THROW_MOVE && code != grab_code;
                    saw_turn |= authored_turn;
                    if authored_turn && !turn_poses.contains(&code) {
                        turn_poses.push(code);
                    }
                    // These approved post-grab pictures already face
                    // away from the original opponent. Preserve the grab's
                    // mirror here; flipping again visually cancels the turn.
                    // Their normal stance later uses the reversed heading.
                    let visibly_left = mirrored ^ authored_turn;
                    if saw_turn && visibly_left == initial_left {
                        incorrect_pose_frames.push(frame);
                    }
                    saw_slam |= impact;
                    saw_get_up |= r.fighter(victim_side).getting_up;
                    s.step(idle());
                }
                let victim_is_left = logical_x(&s, victim_ps) < logical_x(&s, attacker_ps);
                let (stance_code, stance_mirrored) = displayed_pose(&s, attacker_ps);
                if !saw_turn
                    || !incorrect_pose_frames.is_empty()
                    || facing_left(&s, attacker_ps) != victim_is_left
                    || stance_mirrored != victim_is_left
                    || victim_is_left == initial_left
                {
                    failures.push(format!("{thrower:?}/{side:?}/back={back}: initial_left={initial_left}, wrong_turn_frames={incorrect_pose_frames:?}, end=(pose={stance_code:#06x}, mirror={stance_mirrored}, heading_left={}, victim_left={victim_is_left}), trace(frame,phase,pose,mirror,heading_left)={trace:?}", facing_left(&s, attacker_ps)));
                }
                assert!(saw_slam && saw_get_up, "throw slams and recovers");
                if thrower == TournamentFighter::Lee {
                    assert_eq!(
                        turn_poses,
                        [0x1dec, 0x1f04, 0x1f00],
                        "all three back-turned ROM poses were reached"
                    );
                } else {
                    assert_eq!(
                        turn_poses.len(),
                        2,
                        "both authored shoulder-throw poses were reached"
                    );
                }
                assert_eq!(read(&s).fighter(victim_side).health, 48 - u16::from(DAMAGE));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "the authored turn must visibly face the new side, independently of the heading bit:\n{}",
        failures.join("\n")
    );
}

#[test]
fn shoulder_throwers_visibly_turn_after_the_grab_and_face_the_landed_victim() {
    back_turned_throwers_face_the_landed_victim(&[TournamentFighter::Ryu, TournamentFighter::Ken]);
}

#[test]
fn lee_thrower_visibly_turns_each_authored_pose_and_recovers_toward_the_victim() {
    back_turned_throwers_face_the_landed_victim(&[TournamentFighter::Lee]);
}

#[test]
fn lee_sweep_keeps_the_reused_throw_pose_normally_mirrored_outside_a_throw() {
    for side in Side::BOTH {
        let pair = if side == Side::P1 {
            (TournamentFighter::Lee, TournamentFighter::Ryu)
        } else {
            (TournamentFighter::Ryu, TournamentFighter::Lee)
        };
        let Some(mut s) = match_with_art(MatchConfig::from_preset(MODERN), pair.0, pair.1) else {
            return;
        };
        let ps = if side == Side::P1 {
            P1_STRUCT
        } else {
            P2_STRUCT
        };
        let initial_left = facing_left(&s, ps);
        for _ in 0..8 {
            s.step(with(|i| {
                let pad = if side == Side::P1 {
                    &mut i.p1
                } else {
                    &mut i.p2
                };
                pad.directions[DOWN] = true;
            }));
        }
        s.step(with(|i| {
            let pad = if side == Side::P1 {
                &mut i.p1
            } else {
                &mut i.p2
            };
            pad.directions[DOWN] = true;
            pad.buttons[HK] = true;
        }));
        let mut saw_reused_pose = false;
        for _ in 0..80 {
            let (code, mirrored) = displayed_pose(&s, ps);
            if code == 0x1f00 {
                saw_reused_pose = true;
                assert_ne!(read(&s).fighter(side).move_id, THROW_MOVE);
                assert_eq!(
                    mirrored, initial_left,
                    "the ordinary sweep still uses its original facing"
                );
            }
            s.step(idle());
        }
        assert!(
            saw_reused_pose,
            "the real sweep reaches its ROM recovery pose"
        );
    }
}

#[test]
fn throw_deals_damage_plays_get_up_follows_shoulder_motion_and_can_ko() {
    let Some(start) = match_with_art(
        MatchConfig::from_preset(MODERN),
        TournamentFighter::Ryu,
        TournamentFighter::Ken,
    ) else {
        return;
    };
    let mut close = start.clone();
    walk_to_throw_range(&mut close);

    // 1. Forward shoulder throw: P2 crosses over P1 to the left, takes
    //    DAMAGE (6 HP) on the slam frame, rises through GET_UP_MOVE, and
    //    returns to stance.
    let mut s = close.clone();
    s.step(with(|i| {
        i.p1.directions[RIGHT] = true;
        i.p1.buttons[HP] = true;
    }));
    // Snapshot determinism mid-throw.
    let mid = s.clone();
    let mid_snap = s.fight_machine().unwrap().save_snapshot();
    let mut slammed = None;
    let mut saw_get_up = false;
    for f in 1..=90 {
        s.step(idle());
        let r = read(&s);
        if let Some(hit) = r.landed_by(Side::P1) {
            slammed = Some((f, hit));
        }
        if r.fighter(Side::P2).getting_up {
            saw_get_up = true;
            assert_eq!(r.fighter(Side::P2).move_id, GET_UP_MOVE);
            assert!(r.fighter(Side::P2).locked());
            assert!(!r.fighter(Side::P2).attacking());
        }
    }
    let (slam_frame, hit) = slammed.expect("throw slam lands damage");
    assert_eq!(hit.damage, u16::from(DAMAGE));
    assert!(!hit.into_hitstun);
    assert!(!hit.chip);
    assert!(saw_get_up, "victim plays his get-up after the slam");
    let end_read = read(&s);
    assert_eq!(end_read.fighter(Side::P2).health, 48 - u16::from(DAMAGE));
    assert!(
        center_x(&s, P2_STRUCT) < center_x(&s, P1_STRUCT),
        "the forward shoulder throw carries P2 across P1 to the left"
    );

    // Replay from mid and verify identical snapshot.
    let end_snap = s.fight_machine().unwrap().save_snapshot();
    let mut replay = mid;
    assert_eq!(replay.fight_machine().unwrap().save_snapshot(), mid_snap);
    for _ in 1..=90 {
        replay.step(idle());
    }
    assert_eq!(replay.fight_machine().unwrap().save_snapshot(), end_snap);

    // 2. Back throw (`LEFT + HP`): once the tech window closes, P1 turns left
    //    and throws P2 onto his left side (switching sides).
    let mut s = close.clone();
    assert!(center_x(&s, P1_STRUCT) < center_x(&s, P2_STRUCT));
    s.step(with(|i| {
        i.p1.directions[LEFT] = true;
        i.p1.buttons[HP] = true;
    }));
    for _ in 0..90 {
        s.step(idle());
    }
    assert!(
        center_x(&s, P2_STRUCT) < center_x(&s, P1_STRUCT),
        "back throw switches sides so P2 lands to P1's left (P1={}, P2={})",
        center_x(&s, P1_STRUCT),
        center_x(&s, P2_STRUCT)
    );
    assert_eq!(
        read(&s).fighter(Side::P2).health,
        48 - u16::from(DAMAGE),
        "slam at frame {slam_frame} dealt {DAMAGE} damage on back throw too"
    );

    // 3. Eight consecutive throws empty P2's 48-point life bar (`8 * 6 == 48`)
    //    and knock him out (`knocked_out() == true`) without playing `get-up`.
    let mut s = close;
    let mut saw_ko = false;
    for round_throw in 0..8 {
        walk_to_throw_range(&mut s);
        s.step(with(|i| {
            i.p1.directions[RIGHT] = true;
            i.p1.buttons[HP] = true;
        }));
        for _ in 0..75 {
            s.step(idle());
            let r = read(&s);
            if round_throw == 7 {
                assert!(
                    !r.fighter(Side::P2).getting_up,
                    "a KO'd victim stays on the mat"
                );
                if r.fighter(Side::P2).knocked_out() {
                    saw_ko = true;
                    break;
                }
            }
        }
    }
    assert!(saw_ko, "a lethal throw marks the victim knocked out");
    assert_eq!(read(&s).fighter(Side::P2).health, 0);
}

fn throw_destination_matrix(throwers: &[TournamentFighter], edges: &[Option<usize>]) {
    let mut failures = Vec::new();
    for &thrower in throwers {
        let opponent = if thrower == TournamentFighter::Ryu {
            TournamentFighter::Ken
        } else {
            TournamentFighter::Ryu
        };
        for side in Side::BOTH {
            let pair = if side == Side::P1 {
                (thrower, opponent)
            } else {
                (opponent, thrower)
            };
            let Some(start) = match_with_art(MatchConfig::from_preset(MODERN), pair.0, pair.1)
            else {
                return;
            };
            let (attacker_ps, victim_ps, victim_side) = if side == Side::P1 {
                (P1_STRUCT, P2_STRUCT, Side::P2)
            } else {
                (P2_STRUCT, P1_STRUCT, Side::P1)
            };
            for &edge in edges {
                let mut close = start.clone();
                if let Some(direction) = edge {
                    walk_pair_to_edge(&mut close, direction, side);
                } else {
                    walk_to_throw_range(&mut close);
                }
                let original_side =
                    (logical_x(&close, victim_ps) - logical_x(&close, attacker_ps)).signum();
                assert_ne!(
                    original_side, 0,
                    "fixture: distinct sides before {thrower:?}'s throw"
                );
                for back in [false, true] {
                    let forward = if original_side > 0 { RIGHT } else { LEFT };
                    let dir = if back {
                        if forward == RIGHT {
                            LEFT
                        } else {
                            RIGHT
                        }
                    } else {
                        forward
                    };
                    let mut s = enter_throw(&close, side, dir).unwrap_or_else(|| panic!("no real throw entry in the bounded approach window: {thrower:?}/{side:?}/{edge:?}/{back}, raw=({}, {}), logical=({}, {})", center_x(&close, attacker_ps), center_x(&close, victim_ps), logical_x(&close, attacker_ps), logical_x(&close, victim_ps)));
                    let grab_code = displayed_pose(&s, attacker_ps).0;
                    assert_eq!(
                        read(&s).fighter(side).move_id,
                        THROW_MOVE,
                        "fixture {thrower:?}/{side:?}/{edge:?}/{back}: throw starts"
                    );
                    let authored_cross = matches!(
                        thrower,
                        TournamentFighter::Ryu
                            | TournamentFighter::Ken
                            | TournamentFighter::Geki
                            | TournamentFighter::Joe
                            | TournamentFighter::Lee
                            | TournamentFighter::Eagle
                    );
                    let expected_side = if authored_cross || back {
                        -original_side
                    } else {
                        original_side
                    };
                    let mut crossed_before_slam = false;
                    let mut slammed = false;
                    let mut previous_x = logical_x(&s, victim_ps);
                    let mut largest_air_step = 0;
                    let mut impact_step = 0;
                    let mut impact_limit = 0;
                    let mut facing_errors = Vec::new();
                    for _ in 0..110 {
                        let contact_limit = contact_span(&s, attacker_ps, victim_ps);
                        s.step(idle());
                        let vx = logical_x(&s, victim_ps);
                        let step = previous_x.abs_diff(vx);
                        previous_x = vx;
                        let impact = read(&s).landed_by(side).is_some();
                        let (pose, mirrored) = displayed_pose(&s, attacker_ps);
                        if read(&s).fighter(side).move_id == THROW_MOVE && pose != grab_code {
                            let intrinsically_back_turned = matches!(
                                thrower,
                                TournamentFighter::Ryu
                                    | TournamentFighter::Ken
                                    | TournamentFighter::Lee
                            );
                            if mirrored ^ intrinsically_back_turned != (expected_side < 0)
                                && facing_errors.len() < 3
                            {
                                facing_errors.push((pose, mirrored, facing_left(&s, attacker_ps)));
                            }
                        }
                        if impact {
                            impact_step = step;
                            impact_limit =
                                contact_limit.max(contact_span(&s, attacker_ps, victim_ps));
                        } else if !slammed {
                            largest_air_step = largest_air_step.max(step);
                        }
                        if !slammed
                            && !impact
                            && (vx - logical_x(&s, attacker_ps)).signum() == -original_side
                        {
                            crossed_before_slam = true;
                        }
                        slammed |= impact;
                    }
                    let final_side =
                        (logical_x(&s, victim_ps) - logical_x(&s, attacker_ps)).signum();
                    let final_visible_left = displayed_pose(&s, attacker_ps).1;
                    if final_side != expected_side
                        || !facing_errors.is_empty()
                        || facing_left(&s, attacker_ps) != (final_side < 0)
                        || final_visible_left != (final_side < 0)
                        || impact_step > impact_limit
                        || ((authored_cross || back)
                            && (!crossed_before_slam || largest_air_step > 16))
                    {
                        failures.push(format!("{thrower:?}/{side:?}/edge={edge:?}/back={back}: original={original_side}, final={final_side}, expected={expected_side}, crossed_before_slam={crossed_before_slam}, largest_air_step={largest_air_step}, impact_step={impact_step}/{impact_limit}, facing_errors(pose,mirror,heading_left)={facing_errors:?}, final_facing=(mirror={final_visible_left},heading_left={}), raw_positions=({}, {}), logical_positions=({}, {})", facing_left(&s, attacker_ps), center_x(&s, attacker_ps), center_x(&s, victim_ps), logical_x(&s, attacker_ps), logical_x(&s, victim_ps)));
                    }
                    assert_eq!(
                        read(&s).fighter(victim_side).health,
                        48 - u16::from(DAMAGE),
                        "{thrower:?}/{side:?}/{edge:?}/{back}: one slam"
                    );
                    assert!(
                        !s.fight_machine()
                            .unwrap()
                            .toggle_state()
                            .throw
                            .crosses
                            .into_iter()
                            .any(|active| active),
                        "crossing state ends with the slam"
                    );
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "authored destinations and smooth paths survive centre/corner play:\n{}",
        failures.join("\n")
    );
}

#[test]
fn every_authored_throw_keeps_its_destination_and_continuous_crossing_in_centre_play() {
    throw_destination_matrix(&TournamentFighter::ALL, &[None]);
}

#[test]
fn lee_over_the_back_throw_carries_the_victim_across_in_either_facing() {
    throw_destination_matrix(&[TournamentFighter::Lee], &[None]);
}

#[test]
fn shoulder_shove_and_slam_destinations_survive_validated_corners() {
    throw_destination_matrix(
        &[
            TournamentFighter::Ryu,
            TournamentFighter::Ken,
            TournamentFighter::Mike,
            TournamentFighter::Birdie,
            TournamentFighter::Sagat,
        ],
        &[Some(LEFT), Some(RIGHT)],
    );
}

#[test]
fn every_fighter_on_the_roster_runs_his_catalogue_throw_and_get_up() {
    for fighter in TournamentFighter::ALL {
        let Some(mut s) = match_with_art(
            MatchConfig::from_preset(MODERN),
            fighter,
            TournamentFighter::Ryu,
        ) else {
            return;
        };
        walk_to_throw_range(&mut s);
        s.step(with(|i| {
            i.p1.directions[RIGHT] = true;
            i.p1.buttons[HP] = true;
        }));
        assert_eq!(
            read(&s).fighter(Side::P1).move_id,
            THROW_MOVE,
            "{fighter:?} starts his catalogue throw"
        );
        for _ in 0..85 {
            s.step(idle());
            assert_eq!(
                s.screen(),
                "fight",
                "{fighter:?}'s throw has no untranslated branch"
            );
        }
        assert_eq!(
            read(&s).fighter(Side::P2).health,
            48 - u16::from(DAMAGE),
            "{fighter:?}'s throw dealt {DAMAGE} damage"
        );
    }
}

#[test]
fn throw_tech_succeeds_inside_the_seven_frame_window_and_fails_after_it() {
    let Some(start) = match_with_art(
        MatchConfig::from_preset(MODERN),
        TournamentFighter::Ryu,
        TournamentFighter::Ken,
    ) else {
        return;
    };
    let mut close = start.clone();
    walk_to_throw_range(&mut close);

    // Simultaneous throw on frame 0, tech on frame 1, and tech on frame 7
    // (the last frame of TECH_WINDOW) all tech the throw with 0 damage and
    // push the fighters apart.
    for wait in [0usize, 1, usize::from(TECH_WINDOW)] {
        let mut s = close.clone();
        let before_dist = distance(&s);
        if wait == 0 {
            s.step(with(|i| {
                i.p1.directions[RIGHT] = true;
                i.p1.buttons[HP] = true;
                i.p2.directions[LEFT] = true;
                i.p2.buttons[HP] = true;
            }));
        } else {
            s.step(with(|i| {
                i.p1.directions[RIGHT] = true;
                i.p1.buttons[HP] = true;
            }));
            for _ in 1..wait {
                s.step(idle());
            }
            s.step(with(|i| {
                i.p2.directions[RIGHT] = true; // back + HK also techs
                i.p2.buttons[HK] = true;
            }));
        }
        let r = read(&s);
        assert_eq!(
            r.fighter(Side::P1).move_id,
            1,
            "tech at frame {wait} returns P1 to stance"
        );
        assert_eq!(
            r.fighter(Side::P2).move_id,
            1,
            "tech at frame {wait} returns P2 to stance"
        );
        assert_eq!(
            s.fight_machine().unwrap().toggle_state().throw.crosses,
            [false; 3],
            "tech at frame {wait} releases the crossing trajectory"
        );
        assert!(
            distance(&s) > before_dist,
            "tech at frame {wait} pushes the fighters apart ({} -> {})",
            before_dist,
            distance(&s)
        );
        for _ in 0..40 {
            s.step(idle());
        }
        let r = read(&s);
        assert_eq!(r.fighter(Side::P1).health, 48);
        assert_eq!(
            r.fighter(Side::P2).health,
            48,
            "tech at frame {wait} deals no damage"
        );
    }

    // Pressing throw 1 frame after TECH_WINDOW (frame 8) is too late: the
    // throw is not teched and deals full damage.
    let mut s = close;
    s.step(with(|i| {
        i.p1.directions[RIGHT] = true;
        i.p1.buttons[HP] = true;
    }));
    for _ in 0..usize::from(TECH_WINDOW) {
        s.step(idle());
    }
    s.step(with(|i| {
        i.p2.directions[LEFT] = true;
        i.p2.buttons[HP] = true;
    }));
    assert_eq!(
        read(&s).fighter(Side::P2).move_id,
        THROWN_MOVE,
        "pressing throw on frame {} (after the {}-frame window) does not tech",
        TECH_WINDOW + 1,
        TECH_WINDOW
    );
    for _ in 0..70 {
        s.step(idle());
    }
    assert_eq!(read(&s).fighter(Side::P2).health, 48 - u16::from(DAMAGE));
}

#[test]
fn an_active_crossing_throw_snapshot_restores_its_arc_damage_and_wakeup() {
    let Some(mut s) = match_with_art(
        MatchConfig::from_preset(MODERN),
        TournamentFighter::Ryu,
        TournamentFighter::Ken,
    ) else {
        return;
    };
    walk_to_throw_range(&mut s);
    s.step(with(|i| {
        i.p1.directions[RIGHT] = true;
        i.p1.buttons[HP] = true;
    }));
    for _ in 0..12 {
        s.step(idle());
    }
    let mut machine = s.fight_machine().unwrap().clone();
    assert!(
        machine.toggle_state().throw.crosses[1],
        "fixture: snapshot taken during the airborne crossing"
    );
    let saved = machine.save_snapshot();
    let mut expected = Vec::new();
    let mut saw_wakeup_invulnerability = false;
    for _ in 0..90 {
        machine.step();
        saw_wakeup_invulnerability |= machine
            .toggle_state()
            .throw
            .wakeup_invuln
            .into_iter()
            .any(|frames| frames > 0);
        expected.push(machine.save_snapshot());
    }
    assert!(
        saw_wakeup_invulnerability,
        "the original branch runs through get-up and wakeup protection"
    );
    assert_eq!(
        machine.toggle_state().throw.crosses,
        [false; 3],
        "the slam finishes the crossing trajectory"
    );
    machine.restore_snapshot(&saved);
    assert_eq!(
        machine.save_snapshot(),
        saved,
        "the active crossing state restores exactly"
    );
    for (frame, snapshot) in expected.into_iter().enumerate() {
        machine.step();
        assert_eq!(
            machine.save_snapshot(),
            snapshot,
            "crossing throw replay diverged at frame {frame}"
        );
    }
}

#[test]
fn opponent_in_hitstun_blockstun_airborne_or_wakeup_cannot_be_thrown() {
    let Some(start) = match_with_art(
        MatchConfig::from_preset(MODERN),
        TournamentFighter::Ryu,
        TournamentFighter::Ken,
    ) else {
        return;
    };
    let mut close = start.clone();
    walk_to_throw_range(&mut close);

    // 1. Hitstun: hit P2 with crouching MK (0x17, 18f hitstun on a 25f stream
    //    hitting early) or standing MP (0x0f), wait for P1's attack to end
    //    while P2's hitstun is still active, then press Forward + HP: P1 gets
    //    standing HP (0x10), not a throw.
    let mut s = close.clone();
    s.step(with(|i| i.p1.buttons[LP] = true));
    let mut tried_in_hitstun = false;
    for _ in 0..25 {
        s.step(idle());
        let r = read(&s);
        if !r.fighter(Side::P1).attacking() && r.fighter(Side::P2).hitstun() {
            s.step(with(|i| {
                i.p1.directions[RIGHT] = true;
                i.p1.buttons[HP] = true;
            }));
            let after = read(&s);
            assert_eq!(
                after.fighter(Side::P1).move_id,
                0x10,
                "opponent in hitstun cannot be thrown"
            );
            assert_ne!(after.fighter(Side::P2).move_id, THROWN_MOVE);
            tried_in_hitstun = true;
            break;
        }
    }
    assert!(
        tried_in_hitstun,
        "tested throw attempt during opponent hitstun"
    );

    // 2. A late Lee jump HK is guarded by the other Lee. Keep the defender
    //    still until the attack starts, then approach as the attacker lands:
    //    he becomes actionable in throw range while the defender remains in
    //    blockstun. Forward + HP starts standing HP rather than a throw.
    let Some(mut lee_close) = match_with_art(
        MatchConfig::from_preset(MODERN),
        TournamentFighter::Lee,
        TournamentFighter::Lee,
    ) else {
        return;
    };
    walk_to_throw_range(&mut lee_close);
    let mut tried_in_blockstun = false;
    let mut s_lee = lee_close;
    s_lee.step(with(|i| i.p1.directions[UP] = true));
    for _ in 0..20 {
        s_lee.step(idle());
    }
    s_lee.step(with(|i| {
        i.p1.buttons[HK] = true;
        i.p2.directions[RIGHT] = true;
    }));
    for _ in 0..60 {
        s_lee.step(with(|i| {
            i.p1.directions[RIGHT] = true;
            i.p2.directions[RIGHT] = true;
        }));
        let r = read(&s_lee);
        if matches!(r.fighter(Side::P1).state, 0x02 | 0x12 | 0x92)
            && r.fighter(Side::P2).blockstun
            && distance(&s_lee) <= RANGE
        {
            s_lee.step(with(|i| {
                i.p1.directions[RIGHT] = true;
                i.p1.buttons[HP] = true;
                i.p2.directions[RIGHT] = true;
            }));
            let after = read(&s_lee);
            assert_eq!(
                after.fighter(Side::P1).move_id,
                0x10,
                "opponent in blockstun cannot be thrown"
            );
            assert_ne!(after.fighter(Side::P2).move_id, THROWN_MOVE);
            tried_in_blockstun = true;
            break;
        }
    }
    assert!(
        tried_in_blockstun,
        "tested throw attempt during opponent blockstun"
    );

    // 3. Airborne: P2 jumps straight up (`UP`); while P2 is airborne (`+9 & 3 == 1`),
    //    P1's Forward + HP starts standing HP (`0x10`), not a throw.
    let mut s = close.clone();
    s.step(with(|i| i.p2.directions[UP] = true));
    for _ in 0..8 {
        s.step(idle());
    }
    assert_eq!(read(&s).fighter(Side::P2).state & 3, 1);
    s.step(with(|i| {
        i.p1.directions[RIGHT] = true;
        i.p1.buttons[HP] = true;
    }));
    assert_eq!(
        read(&s).fighter(Side::P1).move_id,
        0x10,
        "airborne opponent cannot be thrown"
    );

    // 4. Wakeup (`getting_up`) and post-wakeup throw invulnerability
    //    (`WAKEUP_INVULN` = 13 frames): throw P2, walk up next to him while he
    //    rises, verify he cannot be thrown during `getting_up` or during the
    //    first 13 frames after `getting_up` finishes, and CAN be thrown once
    //    those 13 frames expire.
    let mut s = close;
    s.step(with(|i| {
        i.p1.directions[RIGHT] = true;
        i.p1.buttons[HP] = true;
    }));
    while !read(&s).fighter(Side::P2).getting_up {
        let forward = p1_toward_p2(&s);
        s.step(with(|i| i.p1.directions[forward] = true));
    }
    while distance(&s) > RANGE && read(&s).fighter(Side::P2).getting_up {
        let forward = p1_toward_p2(&s);
        s.step(with(|i| i.p1.directions[forward] = true));
    }
    assert!(read(&s).fighter(Side::P2).getting_up);
    assert!(distance(&s) <= RANGE);
    let during_getup = s.clone();
    let forward = p1_toward_p2(&s);
    s.step(with(|i| {
        i.p1.directions[forward] = true;
        i.p1.buttons[HP] = true;
    }));
    assert_eq!(
        read(&s).fighter(Side::P1).move_id,
        0x10,
        "opponent in get-up cannot be thrown"
    );

    // Advance to the exact frame `getting_up` ends (`wakeup_invuln` starts).
    let mut s = during_getup;
    while read(&s).fighter(Side::P2).getting_up {
        if distance(&s) > 40 {
            let forward = p1_toward_p2(&s);
            s.step(with(|i| i.p1.directions[forward] = true));
        } else {
            s.step(idle());
        }
    }
    // On the first wakeup frame (`wakeup_invuln == 13` after `throw_frame`),
    // P2 is throw-invulnerable.
    let wakeup_start = s.clone();
    let forward = p1_toward_p2(&s);
    s.step(with(|i| {
        i.p1.directions[forward] = true;
        i.p1.buttons[HP] = true;
    }));
    assert_eq!(
        read(&s).fighter(Side::P1).move_id,
        0x10,
        "opponent in post-wakeup throw invulnerability cannot be thrown"
    );

    // After WAKEUP_INVULN (13) frames, P2 can be thrown again.
    let mut s = wakeup_start;
    for _ in 0..usize::from(WAKEUP_INVULN) {
        s.step(idle());
    }
    let forward = p1_toward_p2(&s);
    s.step(with(|i| {
        i.p1.directions[forward] = true;
        i.p1.buttons[HP] = true;
    }));
    assert_eq!(
        read(&s).fighter(Side::P1).move_id,
        THROW_MOVE,
        "once the {WAKEUP_INVULN}-frame wakeup invulnerability expires, P2 can be thrown again"
    );
}

#[test]
fn with_throws_off_nothing_throws_and_state_stays_default() {
    let Some(mut close) = match_with_art(
        MatchConfig::from_preset(CLASSIC),
        TournamentFighter::Ryu,
        TournamentFighter::Ken,
    ) else {
        return;
    };
    assert!(!close.fight_machine().unwrap().match_config().toggles.throws);
    walk_to_throw_range(&mut close);
    for (dir, btn, want) in [
        (RIGHT, HP, 0x10u8),
        (LEFT, HP, 0x10),
        (RIGHT, HK, 0x16),
        (LEFT, HK, 0x16),
    ] {
        let mut s = close.clone();
        s.step(with(|i| {
            i.p1.directions[dir] = true;
            i.p1.buttons[btn] = true;
        }));
        let r = read(&s);
        assert_eq!(r.fighter(Side::P1).move_id, want);
        assert_ne!(r.fighter(Side::P2).move_id, THROWN_MOVE);
        assert_eq!(
            s.fight_machine().unwrap().toggle_state().throw,
            ThrowState::default()
        );
    }
}
