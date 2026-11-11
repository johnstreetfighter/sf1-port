//! Ticket #230 (`ML4`): Show in Training.
//!
//! At the headless session's seam, driven by inputs: confirm on a move's
//! row in Training offers SHOW and TRY (ML5 takes the Try); Show closes the panel and the
//! player's own fighter performs the move, its input fed as P1's player
//! input; from neutral it comes out at once, from a jump or a recovery it
//! waits with `WAIT` on the strip and then does; the strip lights step by
//! step and control returns when the move ends; the dummy (and the CPU
//! opponent) is untouched by Show, which is bit for bit what a pad giving
//! the same inputs would do. Local ROMs only; each test skips without them.

use std::path::Path;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::moves::Choice;
use sf_engine::front::pause::PauseBox;
use sf_engine::front::show::ready;
use sf_engine::front::{FrameInputs, FrameOutput, GameSession, MenuItem, Transition};
use sf_engine::moves::{Strength, REGISTRY};
use sf_engine::roster::TournamentFighter;
use sf_video::callouts::{regions, Strip, LETTER_PENS, STRIP_UNLIT_PENS};
use sf_video::compositor::small_glyph;
use sf_video::decode::decode_char_tile;
use sf_video::render::WIDTH;
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

const FIGHT_LOOP: u32 = 0x3eafa;
const RIGHT: usize = 0;
const DOWN: usize = 2;
const UP: usize = 3;
const LP: usize = 0;

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

fn stick(dir: usize) -> FrameInputs {
    with(|i| i.p1.directions[dir] = true)
}

fn confirm() -> FrameInputs {
    with(|i| i.menu.confirm = true)
}

fn back() -> FrameInputs {
    with(|i| i.menu.back = true)
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

/// Training against the dummy (whose P2 side never moves) or the CPU, under
/// `preset`, with `fighter` as P1, stopped at the first fight loop and then
/// run through the round's intro to where the fighters take control.
fn training(preset: RulesetPreset, fighter: TournamentFighter, dummy: bool) -> GameSession {
    let a = assets().expect("local ROMs");
    let mut s = GameSession::new(a, MatchConfig::from_preset(preset));
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, start());
    tap(&mut s, confirm()); // Single Player
    tap(&mut s, stick(DOWN));
    tap(&mut s, confirm()); // Training
    if dummy {
        tap(&mut s, stick(DOWN));
    }
    tap(&mut s, confirm()); // CPU or DUMMY
    let at = TournamentFighter::ALL.iter().position(|&f| f == fighter).unwrap();
    for _ in 0..at % 6 {
        tap(&mut s, stick(RIGHT));
    }
    tap(&mut s, confirm()); // P1
    s.step(confirm()); // the opponent under the cursor
    if dummy {
        while s.screen() != "country select" {
            s.step(idle());
        }
        tap(&mut s, with(|i| i.p1.buttons[LP] = true));
    }
    to_fight_loop(&mut s);
    for _ in 0..240 {
        s.step(idle());
    }
    s
}

fn p1_move(s: &GameSession) -> u8 {
    s.fight().unwrap().fight_read().fighters[0].move_id
}

/// Open the pause box, choose MOVES and put the cursor on the registry
/// entry at `row` of the list; returns the panel's frame.
fn open_list_at(s: &mut GameSession, row: usize) -> FrameOutput {
    assert_eq!(s.step(start()).transition, Some(Transition::Paused));
    s.step(idle());
    let at = PauseBox::open(true).items().iter().position(|&i| i == MenuItem::Moves).expect("a MOVES row");
    for _ in 0..at {
        tap(s, stick(DOWN));
    }
    tap(s, confirm());
    assert!(s.move_list().is_some());
    for _ in 0..row {
        tap(s, stick(DOWN));
    }
    s.step(idle())
}

/// Confirm the row and then SHOW; the fight is resumed by it.
fn choose_show(s: &mut GameSession) -> FrameOutput {
    s.step(confirm());
    s.step(idle());
    assert_eq!(s.move_list().unwrap().choosing(), Some(Choice::Show));
    let out = s.step(confirm());
    assert_eq!(out.transition, Some(Transition::Choose(MenuItem::Resume)));
    out
}

fn show_move(s: &mut GameSession, row: usize) {
    open_list_at(s, row);
    choose_show(s);
    assert!(s.move_list().is_none(), "Show closes the panel");
}

/// Step the session until `done`, at most `limit` frames, collecting the
/// strip each frame.
fn run(s: &mut GameSession, limit: usize, mut done: impl FnMut(&GameSession) -> bool) -> Vec<Option<Strip>> {
    let mut strips = Vec::new();
    for _ in 0..limit {
        s.step(idle());
        strips.push(s.fight().unwrap().showing().map(|show| show.strip()));
        if done(s) {
            return strips;
        }
    }
    panic!("not done in {limit} frames");
}

#[test]
fn confirm_on_a_move_offers_show_and_try_in_training_only() {
    if assets().is_none() {
        return;
    }
    let mut s = training(RulesetPreset::DefinitiveOverhaul, TournamentFighter::Ryu, true);
    open_list_at(&mut s, 0);
    s.step(confirm());
    assert_eq!(s.move_list().unwrap().choosing(), Some(Choice::Show));
    // Left and right pick between SHOW and TRY (ML5's tests take a Try).
    for (dir, picked) in [(1usize, Choice::Try), (0, Choice::Show)] {
        s.step(idle());
        s.step(with(|i| i.p1.directions[dir] = true));
        assert_eq!(s.move_list().unwrap().choosing(), Some(picked));
    }
    // Back drops the choice first, then the list.
    s.step(idle());
    s.step(back());
    assert!(s.move_list().is_some() && s.move_list().unwrap().choosing().is_none());
    s.step(idle());
    assert_eq!(s.step(back()).transition, Some(Transition::Back));
    assert!(s.move_list().is_none());

    // Outside Training the row offers nothing.
    let a = assets().unwrap();
    let mut s = GameSession::new(a, MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul));
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, start());
    tap(&mut s, stick(DOWN));
    tap(&mut s, confirm());
    tap(&mut s, confirm());
    tap(&mut s, confirm());
    tap(&mut s, with(|i| i.p2.buttons[LP] = true));
    while s.screen() != "country select" {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.p1.buttons[LP] = true));
    to_fight_loop(&mut s);
    for _ in 0..240 {
        s.step(idle());
    }
    assert_eq!(s.step(start()).transition, Some(Transition::Paused));
    s.step(idle());
    let at = PauseBox::open(false).items().iter().position(|&i| i == MenuItem::Moves).unwrap();
    for _ in 0..at {
        tap(&mut s, stick(DOWN));
    }
    tap(&mut s, confirm());
    assert_eq!(s.step(confirm()).transition, None);
    assert!(s.move_list().unwrap().choosing().is_none());
}

#[test]
fn show_from_neutral_brings_the_move_out_and_the_strip_lights_step_by_step() {
    if assets().is_none() {
        return;
    }
    // Ryu's entries (the registry's first rows are his list's).
    for (row, entry) in REGISTRY.iter().enumerate().filter(|(_, e)| e.available.fighters.contains(&TournamentFighter::Ryu)) {
        let mut s = training(RulesetPreset::DefinitiveOverhaul, TournamentFighter::Ryu, true);
        assert_eq!(p1_move(&s), 1, "the stance");
        show_move(&mut s, row);
        let strips = run(&mut s, 400, |s| s.fight().unwrap().showing().is_none());
        // Never WAIT from neutral; the strip lit one step at a time, in order.
        let lits: Vec<usize> = strips
            .iter()
            .flatten()
            .map(|strip| match strip {
                Strip::Input { lit, steps } => {
                    assert_eq!(*steps, entry.input.strip_steps());
                    *lit
                }
                Strip::Wait => panic!("{}: WAIT from neutral", entry.name),
            })
            .collect();
        let steps = entry.input.strip_steps().len();
        assert_eq!(lits[0], 1, "{}: the first step lights as it is fed", entry.name);
        assert_eq!(*lits.last().unwrap(), steps, "{}", entry.name);
        assert!(lits.windows(2).all(|w| w[1] == w[0] || w[1] == w[0] + 1), "{}: one step at a time: {lits:?}", entry.name);
        for n in 1..=steps {
            assert!(lits.contains(&n), "{}: step {n} lit", entry.name);
        }
        // The move came out from the player's side (P1), and control came back
        // when it ended: the show is gone and the pad moves the fighter.
        let came_out = strips.len();
        assert!(came_out > 0);
        let mut again = training(RulesetPreset::DefinitiveOverhaul, TournamentFighter::Ryu, true);
        show_move(&mut again, row);
        let mut seen = false;
        while again.fight().unwrap().showing().is_some() {
            again.step(idle());
            seen |= p1_move(&again) == entry.produces;
        }
        assert!(seen, "{} came out", entry.name);
        assert_eq!(p1_move(&again), 1, "{}: the move has ended and the fighter stands", entry.name);
        for _ in 0..4 {
            again.step(stick(UP));
        }
        assert!(matches!(p1_move(&again), 9 | 0xa | 0xb), "{}: the pad is back: {:#x}", entry.name, p1_move(&again));
    }
}

#[test]
fn show_from_a_jump_or_a_recovery_waits_then_does() {
    if assets().is_none() {
        return;
    }
    // A jump: pause in the air; a recovery: pause during a punch.
    for scenario in ["jump", "recovery"] {
        let mut s = training(RulesetPreset::DefinitiveOverhaul, TournamentFighter::Ryu, true);
        match scenario {
            "jump" => {
                for _ in 0..4 {
                    s.step(stick(UP));
                }
                for _ in 0..6 {
                    s.step(idle());
                }
                assert!(matches!(p1_move(&s), 9 | 0xa | 0xb), "in the air: {:#x}", p1_move(&s));
            }
            _ => {
                s.step(with(|i| i.p1.buttons[LP] = true));
                s.step(idle());
                assert_ne!(p1_move(&s), 1, "punching");
            }
        }
        show_move(&mut s, 0);
        let hadoken = &REGISTRY[0];
        let mut waited = 0;
        let mut fed = false;
        let mut came_out = false;
        for _ in 0..600 {
            let before = s.fight().unwrap().fight_read().fighters[0];
            s.step(idle());
            let Some(show) = s.fight().unwrap().showing() else { break };
            match show.strip() {
                Strip::Wait => {
                    assert!(!fed, "{scenario}: WAIT after the input began");
                    assert!(!ready(&before), "{scenario}: waited with the fighter free and standing");
                    waited += 1;
                }
                Strip::Input { .. } => {
                    if !fed {
                        // Fed from the stance, the gate free.
                        assert!(ready(&before), "{scenario}: began while {:#x}/{:#x}", before.move_id, before.state);
                        fed = true;
                    }
                }
            }
            came_out |= p1_move(&s) == hadoken.produces;
        }
        assert!(waited > 0, "{scenario}: waited with WAIT on the strip");
        assert!(fed && came_out, "{scenario}: then did it");
        assert!(s.fight().unwrap().showing().is_none(), "{scenario}: control returned");
    }
}

#[test]
fn show_is_the_same_as_a_pad_giving_the_same_inputs_and_leaves_the_opponent_alone() {
    if assets().is_none() {
        return;
    }
    for dummy in [true, false] {
        let mut base = training(RulesetPreset::DefinitiveOverhaul, TournamentFighter::Ryu, dummy);
        // Against the CPU (the fair CPU under this preset, SP21) the fight
        // moves on its own: start from a frame where the show feeds at once,
        // P1 free, standing and facing right, as the pad's frames are, with
        // the CPU on the ground and far enough off not to cross over P1 (the
        // show would then feed the mirrored motion, as it should).
        for f in 0..2_000 {
            let m = base.fight_machine().unwrap();
            let facing_right = sf_engine::moves::facing_right(m.work_ram(), &m.state.objectram, sf_engine::fighter::P1_STRUCT);
            let o = &m.state.objectram;
            let x = |e: usize| u16::from_be_bytes([o[e + 6], o[e + 7]]);
            let [p1, p2] = base.fight().unwrap().fight_read().fighters;
            let apart = x(0x400).abs_diff(x(0xa00)) > 120 && !matches!(p2.move_id, 9..=0xb) && p2.state & 1 == 0;
            if ready(&p1) && facing_right && (dummy || apart) {
                break;
            }
            assert!(f < 1_999, "a moment to show from");
            base.step(idle());
        }
        let entry = &REGISTRY[1]; // the Shoryuken
        // A: the show. B: the same frames on the pad, from the machine frame
        // the show's first frame is stepped on.
        let mut shown = base.clone();
        show_move(&mut shown, 1);
        let mut padded = base.clone();
        // The pad: the entry's frames, then neutral.
        let frames = entry.input.frames(true, Strength::Medium);
        let at = shown.fight_machine().unwrap().state.frame;
        assert_eq!(padded.fight_machine().unwrap().state.frame, at);
        for n in 0..frames.len() + 60 {
            let input = frames.get(n).copied().unwrap_or_default();
            shown.step(idle());
            padded.step(with(|i| i.p1 = input));
            let (a, b) = (shown.fight_machine().unwrap(), padded.fight_machine().unwrap());
            assert_eq!(a.state.frame, b.state.frame);
            assert_eq!(a.work_ram(), b.work_ram(), "dummy={dummy}: frame {n}");
            assert_eq!(a.state.objectram, b.state.objectram, "dummy={dummy}: frame {n}");
        }
        assert_eq!(p1_move(&shown), p1_move(&padded));
    }
}

#[test]
fn the_dummy_does_what_training_has_it_doing_with_or_without_a_show() {
    if assets().is_none() {
        return;
    }
    // The dummy: P2's stick and buttons reach nothing, show or no show. The
    // same fight with P2's pad held and with it idle is the same machine,
    // and the show's P2 side is what a pad-driven P1 gives it.
    let base = training(RulesetPreset::DefinitiveOverhaul, TournamentFighter::Ryu, true);
    let mut shown = base.clone();
    let mut shown_held = base.clone();
    let mut padded_held = base.clone();
    show_move(&mut shown, 0);
    show_move(&mut shown_held, 0);
    let frames = REGISTRY[0].input.frames(true, Strength::Medium);
    let held = |i: &mut FrameInputs| {
        i.p2.directions[RIGHT] = true;
        i.p2.buttons[LP] = true;
    };
    for n in 0..frames.len() + 200 {
        let input = frames.get(n).copied().unwrap_or_default();
        shown.step(idle());
        shown_held.step(with(held));
        padded_held.step(with(|i| {
            held(i);
            i.p1 = input;
        }));
        let (a, b, c) = (shown.fight_machine().unwrap(), shown_held.fight_machine().unwrap(), padded_held.fight_machine().unwrap());
        assert_eq!(a.work_ram(), b.work_ram(), "frame {n}: P2's pad reached the dummy");
        assert_eq!(b.work_ram(), c.work_ram(), "frame {n}: Show is not what a pad gives");
    }
}

#[test]
fn the_strip_sits_under_the_health_block_clear_of_every_other_region() {
    for (name, region) in regions::all() {
        if name != regions::Region::Strip {
            assert!(!regions::STRIP.intersects(&region), "meets {name:?}");
        }
    }
    assert!(regions::STRIP.y >= regions::HEALTH_BLOCK.y + regions::HEALTH_BLOCK.h);
    assert_eq!(regions::STRIP.x * 2 + regions::STRIP.w, WIDTH, "centred");
}

/// Whether `text` is drawn in the strip from its `x` in the ROM font in
/// `pens` (glyph pixels only).
fn strip_text(frame: &[u32], a: &Assets, x: usize, text: &str, pens: [u32; 3]) -> bool {
    text.chars().enumerate().all(|(i, ch)| {
        let Some(code) = small_glyph(ch) else { return true };
        let tile = decode_char_tile(&a.gfx4, code as usize);
        (0..8).all(|ty| {
            (0..8).all(|tx| {
                tile[ty][tx] == 3 || frame[(regions::STRIP.y + ty) * WIDTH + x + 8 * i + tx] == pens[tile[ty][tx] as usize]
            })
        })
    })
}

#[test]
fn the_strip_is_drawn_in_every_preset_with_wait_and_then_the_input_lighting() {
    let Some(a) = assets() else { return };
    let centred = |len: usize| regions::STRIP.x + 8 * (regions::STRIP_LETTERS - len) / 2;
    for preset in RulesetPreset::ALL {
        let mut s = training(preset, TournamentFighter::Ryu, true);
        for _ in 0..4 {
            s.step(stick(UP));
        }
        for _ in 0..4 {
            s.step(idle());
        }
        let plain = s.step(idle()).frame;
        show_move(&mut s, 0);
        // In the air: WAIT, in gold, centred in the strip's place.
        let waiting = s.step(idle()).frame;
        assert_eq!(s.fight().unwrap().showing().unwrap().strip(), Strip::Wait, "{preset:?}");
        assert!(strip_text(&waiting.argb, &a, centred(4), "WAIT", LETTER_PENS), "{preset:?}: WAIT is lettered");
        assert_ne!(plain.argb, waiting.argb, "{preset:?}");
        // Landed and fed: `D DF F P` with the fed steps gold and the rest grey.
        let mut lit_seen = Vec::new();
        for _ in 0..400 {
            let frame = s.step(idle()).frame;
            let Some(show) = s.fight().unwrap().showing() else { break };
            if let Strip::Input { lit, .. } = show.strip() {
                let x = centred(8);
                let steps = ["D", "DF", "F", "P"];
                let mut at = x;
                for (n, step) in steps.iter().enumerate() {
                    let pens = if n < lit { LETTER_PENS } else { STRIP_UNLIT_PENS };
                    assert!(strip_text(&frame.argb, &a, at, step, pens), "{preset:?}: step {n} with {lit} lit");
                    at += 8 * (step.len() + 1);
                }
                lit_seen.push(lit);
            }
        }
        assert_eq!(lit_seen.first(), Some(&1), "{preset:?}");
        assert_eq!(lit_seen.last(), Some(&4), "{preset:?}");
    }
}

fn p1_faces_left(s: &GameSession) -> bool {
    s.fight_machine().unwrap().state.objectram[0x400 + 0x37] & 4 != 0
}

#[test]
fn show_feeds_the_mirror_to_a_fighter_who_has_turned_to_face_left() {
    if assets().is_none() {
        return;
    }
    let mut s = training(RulesetPreset::DefinitiveOverhaul, TournamentFighter::Ryu, true);
    // Walk up to the dummy, then jump forward over it: Ryu turns to face it.
    for _ in 0..400 {
        s.step(stick(RIGHT));
    }
    let up_right = with(|i| {
        i.p1.directions[RIGHT] = true;
        i.p1.directions[UP] = true;
    });
    let mut turned = false;
    for _ in 0..4 {
        for _ in 0..70 {
            s.step(up_right);
            turned |= p1_faces_left(&s);
        }
        for _ in 0..40 {
            s.step(idle());
            turned |= p1_faces_left(&s);
        }
        if turned {
            break;
        }
    }
    assert!(turned, "the fighter turned to face left");
    while p1_move(&s) != 1 {
        s.step(idle());
    }
    assert!(p1_faces_left(&s));
    show_move(&mut s, 0);
    let mut came_out = false;
    while s.fight().unwrap().showing().is_some() {
        s.step(idle());
        came_out |= p1_move(&s) == REGISTRY[0].produces;
    }
    assert!(came_out, "the Hadoken comes out facing left");
}

#[test]
fn a_press_while_show_waits_cancels_it_and_the_pad_blocks_again() {
    if assets().is_none() {
        return;
    }
    let mut s = training(RulesetPreset::DefinitiveOverhaul, TournamentFighter::Ryu, true);
    for _ in 0..4 {
        s.step(stick(UP));
    }
    for _ in 0..4 {
        s.step(idle());
    }
    show_move(&mut s, 0);
    s.step(idle());
    assert_eq!(s.fight().unwrap().showing().map(|show| show.strip()), Some(Strip::Wait));
    // A new press of back (left: Ryu faces right) while still in the air.
    s.step(stick(1));
    assert!(s.fight().unwrap().showing().is_none(), "the show is cancelled");
    // The pad has the fighter: back held on landing is a walk back or a block.
    for _ in 0..120 {
        s.step(stick(1));
    }
    assert!(matches!(p1_move(&s), 3 | 4 | 7 | 8), "the pad drives the fighter: {:#x}", p1_move(&s));
}
