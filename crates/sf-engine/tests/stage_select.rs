//! Ticket #193 (`ST1`): a chosen 1987 stage reaches the fight.
//!
//! - Through the headless game session, each of the ten 1987 stages
//!   (Retsu to Sagat, Thailand's two included) reaches a real Local fight
//!   and a real Training fight (against a dummy and against the CPU) with
//!   that stage's scroll positions, both layers' palettes and its stage
//!   tune command, all read here from the ROM's tables, and a rematch
//!   keeps the stage.
//! - In Local and Training under an overhaul preset the stage screen leads
//!   straight to the VS screen and the fight: the country intro is skipped
//!   (about 90 frames of it in Arcade). Arcade still shows the intro and is
//!   frame for frame what it is without the bank.
//! - Under Faithful 1987 Local shows the flag screen and no stage screen,
//!   and its fights are frame for frame the plain Faithful fights.
//! - With no stage chosen every gate is closed: an overhaul fight with the
//!   whole bank loaded is frame for frame the fight without it.
//!
//! Ticket #196 (`ST2`): the stage screen's pick rules.
//!
//! - Player 1 picks the first match; after a match the loser picks; after
//!   a draw whoever picked last picks again; the rule starts over at the
//!   menu. Only the picker's inputs move the stage cursor.
//! - A rematch opens the stage screen with the cursor on the stage just
//!   played, so one press keeps it; a reselect goes through the fighter
//!   select and then the stage screen under the same rule.
//! - The cursor starts on the home stage of Player 2's fighter (Retsu's
//!   for Ryu, Joe's for Ken), and in Training against the CPU on the CPU
//!   opponent's home stage; any stage can be picked.
//! - A `RANDOM` tile picks among the listed stages as the fight starts.
//!
//! Local ROMs only; each test skips without them.

#[path = "support/select.rs"]
mod select;

use std::path::Path;
use std::sync::Arc;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::front::cabinet::{flow_position, Players};
use sf_engine::front::fight::{first_opponent_flag, Fight, FightKind, FightSetup};
use sf_engine::front::stage_select::{home_owner, StageScreen};
use sf_engine::front::{FightDips, FrameInputs, GameSession, PlayerInputs, Transition};
use sf_engine::roster::{MenuInput, PlayMode, StagePick, TournamentFighter};
use sf_video::background_bank::{shown_stages, BackgroundBank, LayerView, StageListing, StageView, RELEASED_WAVES};
use sf_video::compositor::{ink, RomFont};
use sf_video::romset::{load_assets, Assets};
use sf_video::render::Frame;
use sf_video::stage::pack_records;
use sf_video::stage_preview::{StagePreview, StagePreviews};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const TEST_STAGE: &str = include_str!("../../../art/stages/test-stage.json");
/// The fight loop (`0x3eafa`).
const FIGHT_LOOP: u32 = 0x3eafa;

/// The ROM's tables, one longword per starting country (0 Japan), each
/// pointing to ten words per opponent (scroll words, tunes) or five
/// longwords per stage (palettes): this file's own reading of them.
const FG_SCROLLS: usize = 0x437ee;
const BG_SCROLLS: usize = 0x437fe;
const MUSIC: usize = 0x42358;
const FG_PALETTES: usize = 0x436e8;
const BG_PALETTES: usize = 0x43778;

const RIGHT: usize = 0;
const LEFT: usize = 1;
const DOWN: usize = 2;
const OWNERS: [&str; 10] = ["Retsu", "Geki", "Joe", "Mike", "Lee", "Gen", "Birdie", "Eagle", "Adon", "Sagat"];

fn assets() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(load_assets(ROMS, MANIFEST).expect("local ROM set assembles"))
}

fn rom() -> Assets {
    load_assets(ROMS, MANIFEST).expect("local ROM set assembles")
}

/// The bank as `sf-game` loads it: the test stage, the ten 1987 stages and
/// the VS screens.
fn bank(a: &Assets) -> BackgroundBank {
    let mut bank = pack_records(a, &[TEST_STAGE]).expect("the test stage packs");
    bank.add_rom_stages(a).expect("the 1987 stages read");
    bank.add_vs_screens(a).expect("the VS screens build");
    bank
}

fn word(m: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([m[at], m[at + 1]])
}

fn long(m: &[u8], at: usize) -> usize {
    (word(m, at) as usize) << 16 | word(m, at + 2) as usize
}

/// What the ROM's Japan tables give opponent `k`.
struct Expected {
    fg_scroll: u16,
    bg_scroll: u16,
    tune: u16,
    fg_palette: Vec<u8>,
    bg_palette: Vec<u8>,
}

fn expected(m: &[u8], k: usize) -> Expected {
    let table = |tables: usize| long(m, tables);
    let palette = |tables: usize| {
        let at = long(m, table(tables) + 4 * (k / 2));
        m[at..at + 0x200].to_vec()
    };
    Expected {
        fg_scroll: word(m, table(FG_SCROLLS) + 2 * k),
        bg_scroll: word(m, table(BG_SCROLLS) + 2 * k),
        tune: word(m, table(MUSIC) + 2 * k),
        fg_palette: palette(FG_PALETTES),
        bg_palette: palette(BG_PALETTES),
    }
}

fn idle() -> FrameInputs {
    FrameInputs::default()
}

fn with(f: impl Fn(&mut FrameInputs)) -> FrameInputs {
    let mut i = FrameInputs::default();
    f(&mut i);
    i
}

fn tap(s: &mut GameSession, input: FrameInputs) -> &'static str {
    let out = s.step(input);
    s.step(idle());
    out.screen
}

fn confirm() -> FrameInputs {
    with(|i| i.menu.confirm = true)
}

fn down() -> FrameInputs {
    with(|i| i.p1.directions[DOWN] = true)
}

/// A session at power-on plus 200 attract frames, with the bank when
/// `banked`.
fn session(preset: RulesetPreset, banked: bool) -> GameSession {
    session_with(MatchConfig::from_preset(preset), banked)
}

/// [`session`] over `config`.
fn session_with(config: MatchConfig, banked: bool) -> GameSession {
    let mut s = GameSession::new(rom(), config);
    if banked {
        let b = bank(s.assets());
        s = s.with_background_bank(b);
    }
    for _ in 0..200 {
        s.step(idle());
    }
    s
}

/// Which fight a test picks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Local,
    Dummy,
    Cpu,
    Arcade,
}

/// The Main Menu to the select of `mode`.
fn open_select(s: &mut GameSession, mode: Mode) {
    tap(s, with(|i| i.menu.start = true));
    match mode {
        Mode::Local => {
            tap(s, down());
            tap(s, confirm());
            assert_eq!(tap(s, confirm()), "select");
        }
        Mode::Arcade => {
            tap(s, confirm());
            assert_eq!(tap(s, confirm()), "select");
        }
        Mode::Dummy | Mode::Cpu => {
            tap(s, confirm()); // Single Player
            tap(s, down());
            assert_eq!(tap(s, confirm()), "training");
            if mode == Mode::Dummy {
                tap(s, down());
            }
            assert_eq!(tap(s, confirm()), "select");
        }
    }
}

/// The locked fighters of `mode`: Ryu, and Ken (Local, the dummy) or the
/// first live opponent (the CPU), where the cursors start. Returns the
/// last press's transition.
fn lock_fighters(s: &mut GameSession, mode: Mode) -> Option<Transition> {
    match mode {
        Mode::Local => {
            select::confirm_fighter(s, confirm(), tap);
            select::confirm_fighter(s, with(|i| i.p2.buttons[0] = true), GameSession::step).transition
        }
        Mode::Dummy | Mode::Cpu => {
            select::confirm_fighter(s, confirm(), tap);
            select::confirm_fighter(s, confirm(), GameSession::step).transition
        }
        Mode::Arcade => select::confirm_fighter(s, confirm(), GameSession::step).transition,
    }
}

/// The row of the stage `p2` is sent to (the ten 1987 stages in order).
fn home_row(p2: TournamentFighter) -> usize {
    OWNERS.iter().position(|o| *o == home_owner(p2)).expect("a 1987 stage")
}

/// The Main Menu to the stage screen of `mode`. Returns the row the
/// cursor starts on.
fn to_stage_screen(s: &mut GameSession, mode: Mode) -> usize {
    open_select(s, mode);
    let transition = lock_fighters(s, mode);
    let Some(Transition::Fighters(_, p2)) = transition else { panic!("{mode:?}: {transition:?}") };
    s.step(idle());
    assert_eq!(s.screen(), "stage select", "{mode:?}");
    home_row(p2)
}

/// What a run to the fight loop saw.
struct Run {
    /// Every sound latch on the way.
    latches: Vec<u8>,
    /// Frames the machine stepped before the VS screen (the background
    /// alone) first showed.
    frames_to_vs: u32,
    /// The frames shown, as pixels.
    frames: Vec<Vec<u32>>,
    /// Whether the country intro's text showed (a country's name is drawn
    /// at video offset `0x9ba`, 5 to 7 cells) before the VS screen.
    intro_text: bool,
}

/// The five country voices the intro queues (`0x3f992`, one per country).
const COUNTRY_VOICES: [u8; 5] = [0x8f, 0x90, 0x92, 0x91, 0x93];
/// The intro's text block: its video offset and the most cells it writes.
const INTRO_TEXT: std::ops::Range<usize> = 0x9ba..0x9ba + 14;
/// A cleared text cell (`clear_text_layer`).
const BLANK_CELL: [u8; 2] = [0x10, 0x3c];

/// Step until the fight loop, recording the run.
fn to_fight_loop(s: &mut GameSession) -> Run {
    let mut run = Run { latches: Vec::new(), frames_to_vs: 0, frames: Vec::new(), intro_text: false };
    let (mut last, mut vs) = (None, false);
    for f in 0..4_000 {
        let out = s.step(idle());
        run.latches.push(out.sound_latch);
        run.frames.push(out.frame.argb);
        let m = s.fight_machine().expect("a fight");
        let stepped = last != Some(m.state.frame);
        last = Some(m.state.frame);
        if stepped && !vs {
            let scene = m.state.scene();
            if scene.active & 0x60 == 0x20 {
                vs = true;
            } else {
                run.frames_to_vs += 1;
                run.intro_text |= scene.videoram[INTRO_TEXT].chunks(2).any(|cell| cell != BLANK_CELL);
            }
        }
        if flow_position(m) == FIGHT_LOOP {
            assert!(vs, "the VS screen showed before the fight");
            return run;
        }
        assert!(f < 3_999, "the fight loop (at {})", s.screen());
    }
    unreachable!()
}

/// The fight loop's first frame shows stage `k`: its scroll positions on
/// the ROM's page, both layers' palettes, and (in the run's sounds) its
/// tune and no other stage's.
fn assert_stage(s: &GameSession, run: &Run, rom: &Assets, k: usize, what: &str) {
    let want = expected(&rom.maincpu, k);
    let m = s.fight_machine().unwrap();
    let scene = m.state.scene();
    assert_eq!(m.state.fg_scroll, want.fg_scroll + 0xc0, "{what}: the foreground word and the camera's 0xc0");
    assert!(
        (want.bg_scroll..=want.bg_scroll + 0x200).contains(&m.state.bg_scroll),
        "{what}: the background word and the drift: {:#06x}",
        m.state.bg_scroll
    );
    assert_eq!((scene.fg_page, scene.bg_page), (0, 0), "{what}: the ROM's page");
    assert!(scene.paletteram[..0x200] == want.bg_palette[..], "{what}: the background palette");
    assert!(scene.paletteram[0x200..0x400] == want.fg_palette[..], "{what}: the foreground palette");
    assert!(run.latches.contains(&(want.tune as u8)), "{what}: tune {:#x} played", want.tune);
    for j in (0..10).filter(|&j| j != k) {
        let tune = expected(&rom.maincpu, j).tune;
        assert!(!run.latches.contains(&(tune as u8)), "{what}: {}'s tune {tune:#x} did not play", OWNERS[j]);
    }
    assert!(m.match_config().roster.stage.is_some(), "{what}: the roster carries the stage");
}

/// Down presses from row `from` to stage `k`, and confirm: the stage
/// screen's pick of stage `k`.
fn pick(s: &mut GameSession, from: usize, k: usize) {
    for _ in 0..(k + 11 - from) % 11 {
        tap(s, down());
    }
    s.step(confirm());
    s.step(idle());
}

fn each_stage_reaches_the_fight(mode: Mode, mode_play: PlayMode) {
    let Some(check) = assets() else { return };
    for (k, owner) in OWNERS.iter().enumerate() {
        let mut s = session(RulesetPreset::DefinitiveOverhaul, true);
        let from = to_stage_screen(&mut s, mode);
        pick(&mut s, from, k);
        let what = format!("{mode:?} on {owner}'s stage");
        let run = to_fight_loop(&mut s);
        let roster = s.fight_machine().unwrap().match_config().roster;
        assert_eq!(roster.mode, mode_play, "{what}");
        let listed = s.background_bank().unwrap().listed_stages()[k].index;
        assert_eq!(roster.stage, Some(StagePick::Bank(listed)), "{what}");
        assert_stage(&s, &run, &check, k, &what);
        assert!(s.fight_machine().unwrap().unrecovered().is_empty(), "{what}");
    }
}

#[test]
fn each_1987_stage_reaches_a_local_fight_with_its_scrolls_palettes_and_tune() {
    each_stage_reaches_the_fight(Mode::Local, PlayMode::Versus);
}

#[test]
fn each_1987_stage_reaches_a_training_fight_against_a_dummy() {
    each_stage_reaches_the_fight(Mode::Dummy, PlayMode::Training);
}

#[test]
fn each_1987_stage_reaches_a_training_fight_against_the_cpu() {
    each_stage_reaches_the_fight(Mode::Cpu, PlayMode::Training);
}

/// The camera and the drift move the layers during the fight, never off
/// the stage.
#[test]
fn a_chosen_stage_keeps_its_backdrop_through_the_fight() {
    let Some(check) = assets() else { return };
    let want = expected(&check.maincpu, 9);
    let mut s = session(RulesetPreset::DefinitiveOverhaul, true);
    let from = to_stage_screen(&mut s, Mode::Local);
    pick(&mut s, from, 9);
    to_fight_loop(&mut s);
    for f in 0..900u32 {
        s.step(with(|i| i.p1.directions[(f / 150 % 2) as usize] = true));
        let m = s.fight_machine().unwrap();
        assert_eq!(flow_position(m), FIGHT_LOOP, "frame {f}");
        assert!((want.fg_scroll..=want.fg_scroll + 0xc0 + 0x180).contains(&m.state.fg_scroll), "frame {f}: the camera's range");
        assert!((want.bg_scroll..=want.bg_scroll + 0x200).contains(&m.state.bg_scroll), "frame {f}: the drift's range");
        assert_eq!((m.state.fg_page, m.state.bg_page), (0, 0));
    }
}

#[test]
fn the_stage_screen_lists_the_ten_stages_and_back_returns_to_the_select() {
    let Some(a) = assets() else { return };
    let b = bank(&a);
    let (p1, p2) = (TournamentFighter::Ryu, TournamentFighter::Ken);
    let open = |p2| StageScreen::open(b.listed_stages(), FightKind::Local, p1, p2, 0, None).unwrap();
    let mut screen = open(p2);
    assert_eq!(screen.fight(), (FightKind::Local, p1, p2));
    assert_eq!(screen.cursor().unwrap().owner, "Joe", "Ken's opponent starts on Joe's stage");
    assert_eq!(screen.apply(MenuInput::Up), Some(Transition::CursorMoved));
    assert_eq!(screen.apply(MenuInput::Up), Some(Transition::CursorMoved));
    assert_eq!(screen.cursor().unwrap().owner, "Retsu");
    assert_eq!(screen.apply(MenuInput::Up), Some(Transition::CursorMoved));
    assert!(screen.on_random(), "the cursor wraps to the RANDOM tile below Sagat's stage");
    assert_eq!(screen.apply(MenuInput::Confirm), Some(Transition::RandomStage));
    assert_eq!(screen.apply(MenuInput::Up), Some(Transition::CursorMoved));
    assert_eq!(screen.cursor().unwrap().owner, "Sagat");
    assert_eq!(screen.apply(MenuInput::Down), Some(Transition::CursorMoved));
    assert_eq!(screen.apply(MenuInput::Down), Some(Transition::CursorMoved));
    assert_eq!(screen.apply(MenuInput::Down), Some(Transition::CursorMoved));
    let Some(Transition::Stage(StagePick::Bank(index))) = screen.apply(MenuInput::Confirm) else { panic!("a pick") };
    assert_eq!(b.stage_index("geki"), Some(index));
    assert_eq!(screen.apply(MenuInput::Back), Some(Transition::Back));
    assert!(StageScreen::open(&[], FightKind::Local, p1, p2, 0, None).is_none());
    // The cursor starts on the home stage of every fighter P2 can be.
    for f in TournamentFighter::ALL {
        let owner = match f {
            TournamentFighter::Ryu => "Retsu",
            TournamentFighter::Ken => "Joe",
            other => other.name(),
        };
        assert_eq!(open(f).cursor().unwrap().owner, owner, "{f:?}");
    }
    // A fighter with no listed stage (Joe's is left out): the first stage.
    let without_joe: Vec<StageListing> = b.listed_stages().iter().filter(|l| l.owner != "Joe").cloned().collect();
    let screen = StageScreen::open(&without_joe, FightKind::Local, p1, p2, 0, None).unwrap();
    assert_eq!(screen.cursor().unwrap().owner, "Retsu");
    // A rematch's cursor starts on the stage just played.
    let sagat = StagePick::Bank(b.listed_stages()[9].index);
    let rematch = StageScreen::open(b.listed_stages(), FightKind::Local, p1, p2, 1, Some(sagat)).unwrap();
    assert_eq!((rematch.cursor().unwrap().owner.as_str(), rematch.picker()), ("Sagat", 1));
    // Only the picker's inputs count in Local.
    let mut screen = open(p2);
    assert_eq!(screen.apply_side(1, MenuInput::Down), None);
    assert_eq!(screen.apply_side(1, MenuInput::Confirm), None);
    assert_eq!(screen.cursor().unwrap().owner, "Joe");
    assert_eq!(screen.apply_side(0, MenuInput::Down), Some(Transition::CursorMoved));

    let mut s = session(RulesetPreset::DefinitiveOverhaul, true);
    to_stage_screen(&mut s, Mode::Local);
    let first = s.step(idle()).frame.argb;
    assert!(first.iter().any(|&p| p != 0xff00_0000), "a drawn screen");
    tap(&mut s, down());
    assert_ne!(s.step(idle()).frame.argb, first, "the cursor moved");
    // Back is the select again, both fighters still locked.
    assert_eq!(tap(&mut s, with(|i| i.menu.back = true)), "select");
    assert!(s.fight_machine().is_none());
}

/// Steps until the real country select shows, then takes Japan.
fn take_the_flag(s: &mut GameSession) {
    let mut f = 0;
    while s.screen() != "country select" {
        s.step(idle());
        f += 1;
        assert!(f < 600, "the country select (at {})", s.screen());
    }
    tap(s, with(|i| i.p1.buttons[0] = true));
}

#[test]
fn no_country_intro_in_local_or_training_while_arcade_still_shows_it() {
    let Some(_) = assets() else { return };
    // The intro's two holds are 60 and 30 frames; the VS screen follows it.
    for mode in [Mode::Local, Mode::Dummy, Mode::Cpu] {
        // With no bank there is no stage screen: the country intro runs.
        let mut plain = session(RulesetPreset::DefinitiveOverhaul, false);
        open_select(&mut plain, mode);
        lock_fighters(&mut plain, mode);
        // Training against the CPU picks its flag hidden; the others show
        // the real country select.
        if mode != Mode::Cpu {
            take_the_flag(&mut plain);
        }
        let intro = to_fight_loop(&mut plain);
        let shown = intro.frames_to_vs;
        assert!(intro.intro_text, "{mode:?}: the intro's text shows when no stage is chosen");
        assert!(intro.latches.iter().any(|l| COUNTRY_VOICES.contains(l)), "{mode:?}: the country voice plays");
        assert!(shown >= 90, "{mode:?}: the intro runs when no stage is chosen: {shown} frames to the VS screen");

        let mut s = session(RulesetPreset::DefinitiveOverhaul, true);
        let from = to_stage_screen(&mut s, mode);
        pick(&mut s, from, 3);
        let run = to_fight_loop(&mut s);
        let skipped = run.frames_to_vs;
        assert!(!run.intro_text, "{mode:?}: no intro text with a stage chosen");
        assert!(!run.latches.iter().any(|l| COUNTRY_VOICES.contains(l)), "{mode:?}: no country voice with a stage chosen");
        assert!(skipped < 30, "{mode:?}: no intro with a stage chosen: {skipped} frames to the VS screen");
        println!("{mode:?}: {shown} frames to the VS screen with the intro, {skipped} without");
    }
    // Arcade shows the intro, and is frame for frame the game without the
    // bank.
    let mut banked = session(RulesetPreset::DefinitiveOverhaul, true);
    let mut plain = session(RulesetPreset::DefinitiveOverhaul, false);
    let mut runs = Vec::new();
    for s in [&mut banked, &mut plain] {
        open_select(s, Mode::Arcade);
        lock_fighters(s, Mode::Arcade);
        take_the_flag(s);
        runs.push(to_fight_loop(s));
    }
    assert!(runs[0].frames_to_vs >= 90, "Arcade shows the intro: {} frames to the VS screen", runs[0].frames_to_vs);
    assert_eq!(runs[0].frames.len(), runs[1].frames.len());
    for (f, (a, b)) in runs[0].frames.iter().zip(&runs[1].frames).enumerate() {
        assert!(a == b, "Arcade frame {f} differs with the bank loaded");
    }
    assert_eq!(runs[0].latches, runs[1].latches);
    assert_eq!(banked.fight_machine().unwrap().match_config().roster.stage, None);
}

#[test]
fn faithful_1987_local_shows_the_flag_screen_and_no_stage_screen() {
    let Some(_) = assets() else { return };
    let mut banked = session(RulesetPreset::Faithful1987, true);
    let mut plain = session(RulesetPreset::Faithful1987, false);
    let mut runs = Vec::new();
    for s in [&mut banked, &mut plain] {
        open_select(s, Mode::Local);
        // Faithful 1987 binds Ryu to P1 and Ken to P2.
        lock_fighters(s, Mode::Local);
        let mut seen = Vec::new();
        let mut f = 0;
        while s.screen() != "country select" {
            seen.push(s.screen());
            s.step(idle());
            f += 1;
            assert!(f < 600, "the country select (at {})", s.screen());
        }
        assert!(!seen.contains(&"stage select"));
        tap(s, with(|i| i.p1.buttons[0] = true));
        let run = to_fight_loop(s);
        assert!(run.frames_to_vs >= 90, "the intro runs: {}", run.frames_to_vs);
        assert!(s.fight_machine().unwrap().background_bank().is_none());
        assert_eq!(s.fight_machine().unwrap().match_config().roster.stage, None);
        runs.push(run);
    }
    assert_eq!(runs[0].frames.len(), runs[1].frames.len());
    for (f, (a, b)) in runs[0].frames.iter().zip(&runs[1].frames).enumerate() {
        assert!(a == b, "Faithful frame {f} differs with the bank loaded");
    }
    assert_eq!(runs[0].latches, runs[1].latches);
}

/// An overhaul Local fight with the whole bank loaded and no stage chosen
/// is, frame for frame and sound for sound, the fight without the bank.
#[test]
fn with_no_stage_chosen_every_gate_is_closed() {
    let Some(a) = assets() else { return };
    let b = Arc::new(bank(&a));
    let banked_assets = a.with_background_bank(&b);
    let mut config = MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul);
    config.roster.mode = PlayMode::Versus;
    assert_eq!(config.roster.stage, None);
    let setup = FightSetup {
        kind: FightKind::Local,
        players: Players::Two,
        config,
        dsw1: FightDips::DSW1,
        dsw2: FightDips::default().dsw2(),
    };
    let mut banked = Fight::start(&a.maincpu, setup, Some(0), Some(b)).expect("the hidden drive");
    let mut plain = Fight::start(&a.maincpu, setup, Some(0), None).expect("the hidden drive");
    let mut at_loop = None;
    for f in 0..2_500u32 {
        let attack = PlayerInputs { buttons: std::array::from_fn(|i| i == (f / 20 % 6) as usize && f % 20 < 3), ..PlayerInputs::default() };
        let x = banked.step(&banked_assets, attack, PlayerInputs::default(), false);
        let y = plain.step(&a, attack, PlayerInputs::default(), false);
        assert!(x.frame.argb == y.frame.argb, "frame {f}");
        assert_eq!((x.sound_latch, x.latch_cycles), (y.sound_latch, y.latch_cycles), "frame {f}");
        if at_loop.is_none() && flow_position(banked.machine()) == FIGHT_LOOP {
            at_loop = Some(f);
        }
    }
    assert!(at_loop.is_some_and(|f| f > 200), "the intro ran before the fight loop: {at_loop:?}");
    assert_eq!(banked.machine().state.fg_page, 0);
}

/// A listing whose names are far longer than a tile or the caption holds
/// still draws, at every cursor position.
#[test]
fn a_long_stage_name_still_draws_with_its_cursor_arrow() {
    let Some(a) = assets() else { return };
    let font = RomFont::new(&a.gfx4, &a.maincpu);
    let long = "A VERY LONG OWNER NAME THAT RUNS PAST THE SCREEN'S SIXTY-FOUR CELLS ".repeat(2);
    let listing = |i: u8| StageListing { index: i, owner: long.clone(), place: long.clone(), wave: 0 };
    let stages = [listing(0), listing(1), listing(2)];
    let (p1, p2) = (TournamentFighter::Ryu, TournamentFighter::Ken);
    let mut screen = StageScreen::open(&stages, FightKind::Local, p1, p2, 0, None).unwrap();
    assert_eq!((screen.rows(), screen.tile_rect(0).0), (1, 18), "three stages and RANDOM fit one row");
    let mut frames = Vec::new();
    for _ in 0..4 {
        frames.push(screen.render(&font).argb);
        screen.apply(MenuInput::Down);
    }
    assert!(frames.windows(2).all(|w| w[0] != w[1]), "the cursor shows on each tile and on RANDOM");
}

/// A hidden flag that is not Japan's: Joe's first CPU fight comes from the
/// USA flag. Under the overhaul presets the session now picks Japan's flag
/// for Training against the fair CPU too (SP21), so the fight is driven
/// directly: a Training fight on the USA flag with Sagat's stage picked.
/// The picked stage still decides everything the fight shows and plays.
#[test]
fn a_training_fight_on_a_non_japan_flag_holds_the_picked_stage() {
    let Some(a) = assets() else { return };
    let b = bank(&a);
    let sagat = b.listed_stages()[9].index;
    let mut config = MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul);
    config.roster.mode = PlayMode::Training;
    config.roster.p2 = TournamentFighter::Ken;
    config.roster.stage = Some(StagePick::Bank(sagat));
    let flag = first_opponent_flag(TournamentFighter::Joe);
    assert!(flag.is_some_and(|f| f != 0), "Joe's first fight is a non-Japan flag: {flag:?}");
    let setup = FightSetup {
        kind: FightKind::TrainingDummy,
        players: Players::Two,
        config,
        dsw1: FightDips::DSW1,
        dsw2: FightDips::default().dsw2(),
    };
    let banked_assets = a.with_background_bank(&b);
    let mut fight = Fight::start(&a.maincpu, setup, flag, Some(Arc::new(b))).expect("the hidden drive");
    assert_eq!(fight.country(), flag, "the hidden flag");
    let want = expected(&a.maincpu, 9);
    let mut latches = Vec::new();
    for f in 0..3_000 {
        latches.push(fight.step(&banked_assets, PlayerInputs::default(), PlayerInputs::default(), false).sound_latch);
        if flow_position(fight.machine()) == FIGHT_LOOP {
            break;
        }
        assert!(f < 2_999, "the fight loop");
    }
    let m = fight.machine();
    let scene = m.state.scene();
    assert_eq!(m.state.fg_scroll, want.fg_scroll + 0xc0, "the foreground word and the camera's 0xc0");
    assert!((want.bg_scroll..=want.bg_scroll + 0x200).contains(&m.state.bg_scroll));
    assert!(scene.paletteram[..0x200] == want.bg_palette[..] && scene.paletteram[0x200..0x400] == want.fg_palette[..]);
    assert!(latches.contains(&(want.tune as u8)), "Sagat's tune");
    for j in (0..10).filter(|&j| j != 9) {
        assert!(!latches.contains(&(expected(&a.maincpu, j).tune as u8)), "{}'s tune did not play", OWNERS[j]);
    }
}

/// The front end feeds a fight the factory DSW1 (no flip), so a flipped
/// Local or Training fight cannot be reached through the session; this
/// drives the fight directly with Flip Screen on and a 1987 stage picked.
#[test]
fn a_flipped_fight_on_a_1987_stage_keeps_its_scrolls_palettes_and_tune() {
    let Some(a) = assets() else { return };
    let b = bank(&a);
    let sagat = b.listed_stages()[9].index;
    let banked_assets = a.with_background_bank(&b);
    let mut config = MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul);
    config.roster.mode = PlayMode::Versus;
    config.roster.stage = Some(StagePick::Bank(sagat));
    let setup = FightSetup {
        kind: FightKind::Local,
        players: Players::Two,
        config,
        dsw1: FightDips::DSW1 & !0x0100, // Flip Screen on
        dsw2: FightDips::default().dsw2(),
    };
    let mut fight = Fight::start(&a.maincpu, setup, Some(0), Some(Arc::new(b))).expect("the hidden drive");
    let want = expected(&a.maincpu, 9);
    let mut latches = Vec::new();
    for f in 0..3_000 {
        latches.push(fight.step(&banked_assets, PlayerInputs::default(), PlayerInputs::default(), false).sound_latch);
        if flow_position(fight.machine()) == FIGHT_LOOP {
            break;
        }
        assert!(f < 2_999, "the fight loop");
    }
    let scene = fight.machine().state.scene();
    assert!(scene.flip_screen);
    assert_eq!(fight.machine().state.fg_scroll, want.fg_scroll + 0xc0);
    assert!((want.bg_scroll..=want.bg_scroll + 0x200).contains(&fight.machine().state.bg_scroll));
    assert!(scene.paletteram[..0x200] == want.bg_palette[..] && scene.paletteram[0x200..0x400] == want.fg_palette[..]);
    assert!(latches.contains(&(want.tune as u8)), "Sagat's tune");
}

// ---- Ticket #196 (ST2): the stage screen's pick rules ----

/// Player 2's inputs: the stick down, and the first attack button, which
/// on the keyboard is confirm.
fn p2_down() -> FrameInputs {
    with(|i| i.p2.directions[DOWN] = true)
}

fn p2_confirm() -> FrameInputs {
    with(|i| i.p2.buttons[0] = true)
}

/// A session over a configuration with draws left as in 1987 (a drawn
/// match ends with no winner; the overhaul's decider round is off).
fn draws_session() -> GameSession {
    let config = MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul).with_real_second_timer_and_draws(false);
    assert_ne!(config.preset, RulesetPreset::Faithful1987);
    session_with(config, true)
}

/// Inputs that win a match for `side`: walk at the other fighter, then
/// attack with each button in turn.
fn attacks(side: usize, f: u32) -> FrameInputs {
    with(|i| {
        let (pad, toward) = if side == 0 { (&mut i.p1, RIGHT) } else { (&mut i.p2, LEFT) };
        let phase = f % 120;
        pad.directions[toward] = phase < 40;
        if phase >= 40 && phase % 10 < 3 {
            pad.buttons[((f / 10) % 6) as usize] = true;
        }
    })
}

/// Play the fight on until the after-match box: `side` attacks and wins,
/// or with `None` nobody does and the match times out as a draw.
fn until_after_match(s: &mut GameSession, winner: Option<usize>) {
    for f in 0..80_000 {
        let input = winner.map_or_else(idle, |side| attacks(side, f));
        if s.step(input).screen == "after match" {
            assert_eq!(s.fight().unwrap().match_winner(), winner, "who won");
            return;
        }
    }
    panic!("the match end (at {})", s.screen());
}

/// The stage the running fight was started on.
fn fight_stage(s: &GameSession) -> Option<StagePick> {
    s.fight_machine()?.match_config().roster.stage
}

/// The listed stage in row `k`, as the roster holds it.
fn listed(s: &GameSession, k: usize) -> StagePick {
    StagePick::Bank(s.background_bank().unwrap().listed_stages()[k].index)
}

/// The after-match box's item: down `n` rows, then confirm.
fn choose(s: &mut GameSession, n: usize) -> &'static str {
    for _ in 0..n {
        tap(s, down());
    }
    tap(s, confirm())
}

/// Player 1 picks Gen's stage for the first match of a Local session and
/// the match is played to its end by `winner`. Returns the session at the
/// after-match box.
fn first_match(mut s: GameSession, winner: Option<usize>) -> GameSession {
    let from = to_stage_screen(&mut s, Mode::Local);
    pick(&mut s, from, 5); // Gen
    to_fight_loop(&mut s);
    until_after_match(&mut s, winner);
    s
}

#[test]
fn player_1_picks_the_first_match_and_only_the_pickers_inputs_move_the_cursor() {
    let Some(check) = assets() else { return };
    let mut s = session(RulesetPreset::DefinitiveOverhaul, true);
    assert_eq!(to_stage_screen(&mut s, Mode::Local), 2, "Ken's opponent starts on Joe's stage");
    // Player 2 moves, confirms and backs out: none of it counts.
    assert_eq!(tap(&mut s, p2_down()), "stage select");
    assert_eq!(tap(&mut s, p2_confirm()), "stage select");
    assert_eq!(tap(&mut s, with(|i| i.p2.buttons[3] = true)), "stage select");
    assert_eq!(tap(&mut s, with(|i| i.cabinet[3] = true)), "stage select");
    assert!(s.fight_machine().is_none());
    // Player 1 picks, and the cursor never left Joe's stage.
    assert_eq!(tap(&mut s, confirm()), "drive");
    assert_eq!(fight_stage(&s), Some(listed(&s, 2)));
    let run = to_fight_loop(&mut s);
    assert_stage(&s, &run, &check, 2, "Player 1's first pick");
}

#[test]
fn the_loser_picks_and_a_rematch_keeps_the_stage_with_one_press() {
    let Some(check) = assets() else { return };
    let mut s = first_match(draws_session(), Some(0));
    // Player 2 lost, so Player 2 is at the controls, on Gen's stage.
    assert_eq!(choose(&mut s, 0), "stage select", "REMATCH opens the stage screen");
    // Player 1 is not: its moves, confirm and Start do nothing.
    assert_eq!(tap(&mut s, down()), "stage select");
    assert_eq!(tap(&mut s, with(|i| i.p1.directions[RIGHT] = true)), "stage select");
    assert_eq!(tap(&mut s, confirm()), "stage select");
    assert_eq!(tap(&mut s, with(|i| i.menu.start = true)), "stage select");
    assert_eq!(tap(&mut s, with(|i| i.p1.buttons[0] = true)), "stage select");
    assert!(s.fight_machine().is_none());
    // One press from Player 2 keeps the stage.
    assert_eq!(tap(&mut s, p2_confirm()), "drive");
    let run = to_fight_loop(&mut s);
    assert_stage(&s, &run, &check, 5, "the rematch on Gen's stage");
}

#[test]
fn the_loser_can_change_the_stage_on_a_rematch() {
    let Some(check) = assets() else { return };
    let mut s = first_match(draws_session(), Some(0));
    assert_eq!(choose(&mut s, 0), "stage select");
    tap(&mut s, p2_down());
    assert_eq!(tap(&mut s, p2_confirm()), "drive");
    let run = to_fight_loop(&mut s);
    assert_stage(&s, &run, &check, 6, "Player 2's pick, one below Gen");
}

#[test]
fn when_player_2_wins_player_1_picks_the_next_stage() {
    let Some(check) = assets() else { return };
    let mut s = first_match(draws_session(), Some(1));
    assert_eq!(choose(&mut s, 0), "stage select");
    // Player 2 won: its inputs do nothing.
    assert_eq!(tap(&mut s, p2_down()), "stage select");
    assert_eq!(tap(&mut s, p2_confirm()), "stage select");
    assert_eq!(tap(&mut s, down()), "stage select");
    assert_eq!(tap(&mut s, confirm()), "drive");
    let run = to_fight_loop(&mut s);
    assert_stage(&s, &run, &check, 6, "Player 1's pick, one below Gen");
}

#[test]
fn after_a_draw_whoever_picked_last_picks_again() {
    let Some(check) = assets() else { return };
    // Player 1 picks the first match and wins; Player 2 picks the second,
    // which is a draw; Player 2 picks the third.
    let mut s = first_match(draws_session(), Some(0));
    assert_eq!(choose(&mut s, 0), "stage select");
    assert_eq!(tap(&mut s, p2_confirm()), "drive");
    to_fight_loop(&mut s);
    until_after_match(&mut s, None);
    assert_eq!(choose(&mut s, 0), "stage select");
    assert_eq!(tap(&mut s, confirm()), "stage select", "Player 1 does not pick after the draw");
    assert_eq!(tap(&mut s, down()), "stage select");
    assert_eq!(tap(&mut s, p2_confirm()), "drive");
    let run = to_fight_loop(&mut s);
    assert_stage(&s, &run, &check, 5, "Player 2's pick again, on Gen's stage");

    // And a first match that is a draw leaves Player 1 picking.
    let mut s = first_match(draws_session(), None);
    assert_eq!(choose(&mut s, 0), "stage select");
    assert_eq!(tap(&mut s, p2_confirm()), "stage select");
    assert_eq!(tap(&mut s, confirm()), "drive");
}

#[test]
fn the_rule_starts_over_when_players_leave_for_the_menu() {
    let Some(check) = assets() else { return };
    // Player 1 won, so Player 2 would pick next; MENU, then Local again.
    let mut s = first_match(draws_session(), Some(0));
    assert_eq!(choose(&mut s, 2), "main menu", "MENU");
    assert_eq!(tap(&mut s, confirm()), "multi player");
    assert_eq!(tap(&mut s, confirm()), "select", "Local");
    let t = lock_fighters(&mut s, Mode::Local);
    assert!(matches!(t, Some(Transition::Fighters(..))), "{t:?}");
    s.step(idle());
    assert_eq!(s.screen(), "stage select");
    assert_eq!(tap(&mut s, p2_confirm()), "stage select", "Player 2 does not pick the first match of a new session");
    assert_eq!(tap(&mut s, confirm()), "drive");
    let run = to_fight_loop(&mut s);
    assert_stage(&s, &run, &check, 2, "Player 1's pick, on Joe's stage, not the old session's Gen");

    // Quitting from the pause box mid-fight resets it too: Player 1 won the
    // first match, Player 2 picks the rematch, and QUIT TO MENU follows.
    let mut s = first_match(draws_session(), Some(0));
    assert_eq!(choose(&mut s, 0), "stage select");
    assert_eq!(tap(&mut s, p2_confirm()), "drive");
    to_fight_loop(&mut s);
    assert_eq!(tap(&mut s, with(|i| i.menu.start = true)), "pause");
    assert_eq!(choose(&mut s, 5), "main menu", "QUIT TO MENU");
    assert_eq!(tap(&mut s, confirm()), "multi player");
    assert_eq!(tap(&mut s, confirm()), "select", "Local");
    lock_fighters(&mut s, Mode::Local);
    s.step(idle());
    assert_eq!(s.screen(), "stage select");
    assert_eq!(tap(&mut s, p2_confirm()), "stage select", "Player 1 picks again");
    assert_eq!(tap(&mut s, confirm()), "drive");
    assert_eq!(fight_stage(&s), Some(listed(&s, 2)), "a fresh cursor on Joe's stage, not Gen's");

    // Leaving through RESELECT and Back out of the select resets it too.
    let mut s = first_match(draws_session(), Some(0));
    assert_eq!(choose(&mut s, 1), "select", "RESELECT");
    assert_eq!(tap(&mut s, with(|i| i.menu.back = true)), "multi player");
    assert_eq!(tap(&mut s, confirm()), "select", "Local");
    lock_fighters(&mut s, Mode::Local);
    s.step(idle());
    assert_eq!(s.screen(), "stage select");
    assert_eq!(tap(&mut s, p2_confirm()), "stage select");
    assert_eq!(tap(&mut s, confirm()), "drive");
}

#[test]
fn reselect_goes_through_the_select_then_the_stage_screen_under_the_same_rule() {
    let Some(check) = assets() else { return };
    let mut s = first_match(draws_session(), Some(0));
    assert_eq!(choose(&mut s, 1), "select", "RESELECT");
    let transition = lock_fighters(&mut s, Mode::Local);
    assert!(matches!(transition, Some(Transition::Fighters(..))), "{transition:?}");
    s.step(idle());
    assert_eq!(s.screen(), "stage select", "the stage screen follows the fighter select");
    // The loser (Player 2) is at the controls; the cursor is on the new
    // opponent's home stage, not on the stage just played.
    assert_eq!(tap(&mut s, confirm()), "stage select");
    assert_eq!(tap(&mut s, down()), "stage select");
    assert_eq!(tap(&mut s, p2_confirm()), "drive");
    let run = to_fight_loop(&mut s);
    assert_stage(&s, &run, &check, 2, "Joe's stage, Ken's home");
}

#[test]
fn the_cursor_starts_on_the_home_stage_of_player_2s_fighter() {
    if assets().is_none() {
        return;
    }
    // Ken (Local, and the Training dummy): Joe's stage.
    for mode in [Mode::Local, Mode::Dummy] {
        let mut s = session(RulesetPreset::DefinitiveOverhaul, true);
        assert_eq!(to_stage_screen(&mut s, mode), 2, "{mode:?}");
        assert_eq!(tap(&mut s, confirm()), "drive");
        assert_eq!(fight_stage(&s), Some(listed(&s, 2)), "{mode:?}: Ken's opponent starts on Joe's stage");
    }
    // Ryu: Retsu's stage. Player 2 takes Ryu's tile and Player 1 Ken's.
    let mut s = session(RulesetPreset::DefinitiveOverhaul, true);
    open_select(&mut s, Mode::Local);
    tap(&mut s, with(|i| i.p2.directions[LEFT] = true));
    select::confirm_fighter(&mut s, p2_confirm(), GameSession::step);
    s.step(idle());
    tap(&mut s, with(|i| i.p1.directions[RIGHT] = true));
    let t = select::confirm_fighter(&mut s, confirm(), GameSession::step).transition;
    assert_eq!(t, Some(Transition::Fighters(TournamentFighter::Ken, TournamentFighter::Ryu)));
    s.step(idle());
    assert_eq!(s.screen(), "stage select");
    assert_eq!(tap(&mut s, confirm()), "drive");
    assert_eq!(fight_stage(&s), Some(listed(&s, 0)), "Ryu's opponent starts on Retsu's stage");
}

#[test]
fn training_against_the_cpu_starts_on_the_opponents_home_stage_and_any_stage_picks() {
    if assets().is_none() {
        return;
    }
    // Each live opponent (the fair CPU plays Ryu and Ken here, SP21): the
    // cursor starts on the opponent's home stage, Joe's for Ken and
    // Retsu's for Ryu, as in Local.
    for (moves, cpu) in [(0, TournamentFighter::Ken), (1, TournamentFighter::Ryu)] {
        let mut s = session(RulesetPreset::DefinitiveOverhaul, true);
        open_select(&mut s, Mode::Cpu);
        select::confirm_fighter(&mut s, confirm(), tap); // Ryu
        for _ in 0..moves {
            tap(&mut s, with(|i| i.p1.directions[LEFT] = true));
        }
        let Some(Transition::Fighters(_, p2)) = select::confirm_fighter(&mut s, confirm(), GameSession::step).transition else { panic!("the CPU select, {cpu:?}") };
        assert_eq!(p2, cpu);
        s.step(idle());
        assert_eq!(s.screen(), "stage select");
        let want = home_row(p2);
        assert_eq!(home_owner(p2), OWNERS[want]);
        assert_eq!(tap(&mut s, confirm()), "drive");
        assert_eq!(fight_stage(&s), Some(listed(&s, want)), "{p2:?}'s home stage");
    }
    // Any stage can be picked against the CPU, whoever the opponent is.
    let mut s = session(RulesetPreset::DefinitiveOverhaul, true);
    let from = to_stage_screen(&mut s, Mode::Cpu);
    pick(&mut s, from, 9);
    assert_eq!(fight_stage(&s), Some(listed(&s, 9)), "Sagat's stage against a CPU who is not Sagat");
}

#[test]
fn the_random_tile_picks_a_listed_stage_as_the_fight_starts() {
    let Some(check) = assets() else { return };
    let mut at_screen = draws_session();
    let from = to_stage_screen(&mut at_screen, Mode::Local);
    let mut picks = std::collections::BTreeSet::new();
    let mut last = None;
    for wait in 0..12 {
        let mut s = at_screen.clone();
        for _ in 0..wait * 7 {
            s.step(idle());
        }
        // From the start, down to the tile under Sagat's stage.
        for _ in 0..(10 + 11 - from) % 11 {
            tap(&mut s, down());
        }
        assert!(s.fight_machine().is_none(), "no stage is chosen on the way");
        s.step(confirm());
        s.step(idle());
        let stage = fight_stage(&s).expect("the fight has started");
        let k = (0..10).find(|&k| stage == listed(&s, k)).expect("one of the listed stages");
        picks.insert(k);
        last = Some((s, k));
    }
    assert!(picks.len() > 1, "RANDOM does not always give the same stage: {picks:?}");
    // The stage it gave is the fight's, and the rematch's cursor starts there.
    let (mut s, k) = last.unwrap();
    let run = to_fight_loop(&mut s);
    assert_stage(&s, &run, &check, k, "the RANDOM pick");
    until_after_match(&mut s, Some(0));
    assert_eq!(choose(&mut s, 0), "stage select");
    assert_eq!(tap(&mut s, p2_confirm()), "drive");
    assert_eq!(fight_stage(&s), Some(listed(&s, k)), "one press keeps the stage RANDOM gave");
}

#[test]
fn when_player_1_loses_the_rematch_opens_on_the_stage_just_played_for_player_1() {
    // Training has no match end (spec #238), so the rematch rule is
    // Local's: Player 1 picks Sagat's stage and loses to Player 2.
    let Some(check) = assets() else { return };
    let mut s = draws_session();
    let from = to_stage_screen(&mut s, Mode::Local);
    pick(&mut s, from, 9); // Sagat
    to_fight_loop(&mut s);
    until_after_match(&mut s, Some(1));
    assert_eq!(choose(&mut s, 0), "stage select", "REMATCH opens the stage screen");
    // Player 1 lost, so Player 1 is at the controls, and one press keeps
    // Sagat's stage.
    assert_eq!(tap(&mut s, confirm()), "drive");
    let run = to_fight_loop(&mut s);
    assert_stage(&s, &run, &check, 9, "the rematch on Sagat's stage");
}

#[test]
fn the_random_tile_reaches_every_listed_stage() {
    let Some(a) = assets() else { return };
    let b = bank(&a);
    let stages = b.listed_stages();
    let mut seen = std::collections::BTreeSet::new();
    for frame in 0..1_000 {
        let StagePick::Bank(index) = sf_engine::front::stage_select::random_pick(stages, frame);
        seen.insert(index);
    }
    let all: std::collections::BTreeSet<u8> = stages.iter().map(|l| l.index).collect();
    assert_eq!(seen, all, "every listed stage appears, and no other");
}

// Ticket #197 (`ST3`): the stage screen's look, layout A of the approved
// mockup.

const BLACK: u32 = 0xff00_0000;
const FRAME_INNER: std::ops::Range<usize> = 16..368;
/// The pixel rows of the picker line, between the header and the large preview.
const PICKER_LINE: std::ops::Range<usize> = 25..33;

type Box4 = (usize, usize, usize, usize);

fn listing(index: u8, owner: &str, place: &str, wave: u8) -> StageListing {
    StageListing { index, owner: owner.to_owned(), place: place.to_owned(), wave }
}

/// The backdrops of the bank's listed stages, as the session draws them.
fn previews(a: &Assets, b: &BackgroundBank) -> Arc<StagePreviews> {
    Arc::new(b.listed_previews(&a.with_background_bank(b)))
}

/// A simulated full list of 38 stages: the ten 1987 stages, then three
/// waves of simulated challengers' stages, each wearing one of the ten
/// backdrops.
fn simulated_38(ten: &StagePreviews) -> (Vec<StageListing>, Arc<StagePreviews>) {
    let listed: Vec<StageListing> = (0..38u8)
        .map(|i| {
            let wave = if i < 10 { 0 } else { 1 + (i - 10) / 10 };
            listing(i, &format!("Fighter{i}"), "Somewhere", wave)
        })
        .collect();
    let backdrops: Vec<&StagePreview> = ten.values().collect();
    let all = listed.iter().map(|l| (l.index, backdrops[l.index as usize % 10].clone())).collect();
    (listed, Arc::new(all))
}

fn pixel(frame: &Frame, x: usize, y: usize) -> u32 {
    frame.argb[y * 384 + x]
}

fn window(frame: &Frame, (x, y, w, h): Box4) -> Vec<u32> {
    (y..y + h).flat_map(|row| (x..x + w).map(move |col| pixel(frame, col, row))).collect()
}

fn has_pixel(frame: &Frame, rows: std::ops::Range<usize>, color: u32) -> bool {
    rows.into_iter().any(|y| FRAME_INNER.clone().any(|x| pixel(frame, x, y) == color))
}

/// Every tile's outline, with the cursor's second outline, sits inside the
/// frame, above its foot row and clear of every other tile; the large
/// preview and its caption sit above the grid.
fn assert_layout_holds(screen: &StageScreen, what: &str) {
    let tiles = screen.tiles();
    for i in 0..tiles {
        let (x, y, w, h) = screen.tile_rect(i);
        assert!(x > FRAME_INNER.start && x + w < FRAME_INNER.end, "{what}: tile {i} is inside the frame's sides");
        assert!(y + h < 200, "{what}: tile {i} is above the foot row");
        for j in (i + 1)..tiles {
            let (ox, oy, ow, oh) = screen.tile_rect(j);
            let apart = ox > x + w + 1 || x > ox + ow + 1 || oy > y + h + 1 || y > oy + oh + 1;
            assert!(apart, "{what}: tiles {i} and {j} leave room for the cursor's outline");
        }
    }
    let (bx, by, bw, bh) = screen.big_rect();
    assert!(bx >= FRAME_INNER.start && bx + bw <= FRAME_INNER.end, "{what}: the large preview is inside the frame");
    assert!(by > PICKER_LINE.end && by + bh + 3 + 8 < screen.grid_top(), "{what}: the preview and caption sit above the grid");
}

#[test]
fn ten_stages_and_random_fill_two_rows_with_a_large_preview() {
    let Some(a) = assets() else { return };
    let b = bank(&a);
    let font = RomFont::new(&a.gfx4, &a.maincpu);
    let (p1, p2) = (TournamentFighter::Ryu, TournamentFighter::Ken);
    let shown = previews(&a, &b);
    assert_eq!(shown.len(), 10, "every listed stage has a backdrop");
    let screen = StageScreen::open(b.listed_stages(), FightKind::Local, p1, p2, 0, None).unwrap().with_previews(shown);
    assert_eq!((screen.tiles(), screen.rows()), (11, 2), "ten stages and RANDOM, two rows");
    // Eight to a row, left to right: the 9th stage starts the second row,
    // and RANDOM is the third tile there.
    let (first, ninth, random) = (screen.tile_rect(0), screen.tile_rect(8), screen.tile_rect(10));
    assert_eq!((ninth.0, ninth.1 - first.1), (first.0, 24), "the second row starts under the first tile");
    assert_eq!((random.0 - first.0, random.1), (88, ninth.1), "RANDOM follows the tenth stage");
    assert_layout_holds(&screen, "ten stages");
    let (_, _, _, big_h) = screen.big_rect();
    assert!(big_h >= 90, "two rows leave a large preview: {big_h} pixels tall");

    let frame = screen.render(&font);
    // Nothing on the frame's foot row: no preset line (the maintainer, 2026-11-07).
    let foot_clear = (200..208).all(|y| FRAME_INNER.clone().all(|x| pixel(&frame, x, y) == 0xff00_0000));
    assert!(foot_clear, "the stage screen shows nothing at its foot");
    // Each stage's tile is cut from its own backdrop, so no two match.
    let images: Vec<Vec<u32>> = (0..10)
        .map(|i| {
            let (x, y, _, _) = screen.tile_rect(i);
            window(&frame, (x + 1, y + 1, 38, 14))
        })
        .collect();
    for i in 0..10 {
        assert!(images[i].iter().any(|&p| p != images[i][0]), "tile {i} shows a picture, not a swatch");
        for j in (i + 1)..10 {
            assert_ne!(images[i], images[j], "tiles {i} and {j} show their own stages");
        }
    }
    // RANDOM is a striped tile.
    let (x, y, _, _) = screen.tile_rect(10);
    let stripes = window(&frame, (x + 1, y + 1, 38, 14));
    assert!(stripes.contains(&0xff30_3040) && stripes.contains(&0xff50_5068), "RANDOM shows its stripes");
    // The cursor's tile (Joe's, for Ken's opponent) has the picker's frame
    // and the others the gold.
    let joe = screen.tile_rect(2);
    let (yellow, gold) = (font.pen(ink::YELLOW, 0), font.pen(ink::GOLD_A, 2));
    assert_eq!((pixel(&frame, joe.0, joe.1), pixel(&frame, joe.0 - 1, joe.1 - 1)), (yellow, yellow));
    let other = screen.tile_rect(3);
    assert_eq!((pixel(&frame, other.0, other.1), pixel(&frame, other.0 - 1, other.1 - 1)), (gold, BLACK));
    // The large preview is a picture.
    let (bx, by, bw, bh) = screen.big_rect();
    let big = window(&frame, (bx, by, bw, bh));
    assert!(big.iter().filter(|&&p| p != big[0]).count() > bw * bh / 4);
}

#[test]
fn the_large_preview_follows_the_cursor_and_random_shows_a_question_mark() {
    let Some(a) = assets() else { return };
    let b = bank(&a);
    let font = RomFont::new(&a.gfx4, &a.maincpu);
    let (p1, p2) = (TournamentFighter::Ryu, TournamentFighter::Ken);
    let mut screen =
        StageScreen::open(b.listed_stages(), FightKind::Local, p1, p2, 0, None).unwrap().with_previews(previews(&a, &b));
    let big_of = |screen: &StageScreen| window(&screen.render(&font), screen.big_rect());
    let joe = big_of(&screen);
    screen.apply(MenuInput::Down);
    assert_ne!(big_of(&screen), joe, "Mike's stage is not Joe's");
    screen.apply(MenuInput::Up);
    assert_eq!(big_of(&screen), joe);
    for _ in 0..8 {
        screen.apply(MenuInput::Down);
    }
    assert!(screen.on_random());
    let random = big_of(&screen);
    assert!(random.contains(&0xff30_3040) && random.contains(&0xff50_5068), "the stripes");
    assert!(random.contains(&font.pen(ink::YELLOW, 0)), "a ? in the picker's colour");
}

#[test]
fn the_picker_line_shows_in_the_pickers_colour_in_local_only() {
    let Some(a) = assets() else { return };
    let b = bank(&a);
    let font = RomFont::new(&a.gfx4, &a.maincpu);
    let (p1, p2) = (TournamentFighter::Ryu, TournamentFighter::Ken);
    let shown = previews(&a, &b);
    let render = |kind, picker| {
        let screen = StageScreen::open(b.listed_stages(), kind, p1, p2, picker, None).unwrap().with_previews(shown.clone());
        let frame = screen.render(&font);
        (screen, frame)
    };
    let (yellow, blue) = (font.pen(ink::YELLOW, 2), font.pen(ink::P2_BLUE, 2));
    let (_, p1_picks) = render(FightKind::Local, 0);
    assert!(has_pixel(&p1_picks, PICKER_LINE, yellow) && !has_pixel(&p1_picks, PICKER_LINE, blue), "P1 PICKS is yellow");
    let (screen, p2_picks) = render(FightKind::Local, 1);
    assert!(has_pixel(&p2_picks, PICKER_LINE, blue) && !has_pixel(&p2_picks, PICKER_LINE, yellow), "P2 PICKS is blue");
    // P2's cursor tile is framed in the compositor's blue, the select's.
    let at = screen.tile_rect(2);
    assert_eq!(pixel(&p2_picks, at.0, at.1), font.pen(ink::P2_BLUE, 2));
    // Training has one player at the controls: no side is named, and the
    // cursor is Player 1's.
    for kind in [FightKind::TrainingCpu, FightKind::TrainingDummy] {
        let (screen, frame) = render(kind, 0);
        assert!(PICKER_LINE.clone().all(|y| FRAME_INNER.clone().all(|x| pixel(&frame, x, y) == BLACK)), "{kind:?} has no picker line");
        let at = screen.tile_rect(2);
        assert_eq!(pixel(&frame, at.0, at.1), font.pen(ink::YELLOW, 0));
    }
}

#[test]
fn the_caption_names_the_owner_and_place_under_the_large_preview() {
    let Some(a) = assets() else { return };
    let b = bank(&a);
    let font = RomFont::new(&a.gfx4, &a.maincpu);
    let (p1, p2) = (TournamentFighter::Ryu, TournamentFighter::Sagat);
    let screen = StageScreen::open(b.listed_stages(), FightKind::Local, p1, p2, 0, None).unwrap().with_previews(previews(&a, &b));
    assert_eq!(screen.cursor().unwrap().owner, "Sagat");
    // The caption is "SAGAT \u{b7} THAILAND" in the 8x8 font, centred, 3 pixels
    // under the preview: draw the same text on a blank frame to compare.
    let frame = screen.render(&font);
    let (_, by, _, bh) = screen.big_rect();
    let y = by + bh + 3;
    let mut expected = vec![BLACK; 384 * 224];
    let text = "SAGAT \u{b7} THAILAND";
    font.text_px(&mut expected, (384 - 8 * text.chars().count()) / 2, y, text, ink::YELLOW);
    let region = |argb: &[u32]| -> Vec<u32> { (y..y + 8).flat_map(|row| argb[row * 384 + 16..row * 384 + 368].to_vec()).collect() };
    assert_eq!(region(&frame.argb), region(&expected));
    assert!(region(&expected).iter().any(|&p| p != BLACK), "the caption has ink");
}

/// The wide frame, as every stage screen starts: the edge the grid must
/// not touch.
fn gold_frame_only(font: &RomFont<'_>) -> Frame {
    let mut argb = vec![BLACK; 384 * 224];
    font.gold_frame(&mut argb, 1, 2, 46, 25, 0..0);
    Frame { argb }
}

#[test]
fn thirty_eight_stages_and_random_fill_five_rows_on_one_screen() {
    let Some(a) = assets() else { return };
    let b = bank(&a);
    let font = RomFont::new(&a.gfx4, &a.maincpu);
    let (listed, shown) = simulated_38(&previews(&a, &b));
    let (p1, p2) = (TournamentFighter::Ryu, TournamentFighter::Ken);
    let mut screen = StageScreen::open(&listed, FightKind::Local, p1, p2, 1, None).unwrap().with_previews(shown);
    assert_eq!((screen.tiles(), screen.rows()), (39, 5), "38 stages and RANDOM, five rows");
    assert_layout_holds(&screen, "38 stages");
    // Eight to a row: RANDOM is the 7th tile of the fifth row.
    let (first, random) = (screen.tile_rect(0), screen.tile_rect(38));
    assert_eq!((random.0 - first.0, random.1 - first.1), (6 * 44, 4 * 24));
    let (_, _, big_w, big_h) = screen.big_rect();
    assert!(big_h >= 26 && big_w >= 44, "a large preview is left: {big_w}x{big_h}");
    // The cursor walks every tile, each framed in the picker's blue, and no
    // two screens match.
    let blue = font.pen(ink::P2_BLUE, 2);
    let bare = gold_frame_only(&font);
    let mut frames = Vec::new();
    for i in 0..screen.tiles() {
        let frame = screen.render(&font);
        let (x, y, _, _) = screen.tile_rect(i);
        assert_eq!((pixel(&frame, x, y), pixel(&frame, x - 1, y - 1)), (blue, blue), "the cursor frames tile {i}");
        // The grid never touches the gold frame's sides.
        assert_eq!(window(&frame, (8, 24, 8, 168)), window(&bare, (8, 24, 8, 168)), "the left edge is the frame's");
        assert_eq!(window(&frame, (368, 24, 8, 168)), window(&bare, (368, 24, 8, 168)), "the right edge is the frame's");
        frames.push(frame.argb);
        screen.apply(MenuInput::Down);
    }
    assert_eq!(screen.cursor().map(|s| s.index), Some(0), "the cursor wrapped to the first stage");
    frames.sort();
    frames.dedup();
    assert_eq!(frames.len(), 39, "each cursor position draws its own screen");
    // Sizes either side: seven stages and RANDOM fill one row, eight need two.
    let rows = |n: u8| {
        let few: Vec<StageListing> = (0..n).map(|i| listing(i, "A", "B", 0)).collect();
        StageScreen::open(&few, FightKind::Local, p1, p2, 0, None).unwrap().rows()
    };
    assert_eq!((rows(7), rows(8), rows(15), rows(16), rows(31), rows(32)), (1, 2, 2, 3, 4, 5));
}

#[test]
fn the_bonus_filter_shows_released_waves_only_with_bonus_challengers_on() {
    let ten: Vec<StageListing> = OWNERS.iter().enumerate().map(|(i, o)| listing(i as u8, o, "Somewhere", 0)).collect();
    let mut all = ten.clone();
    // Simulated records of waves 1 to 3, interleaved in the list.
    for (k, wave) in [1u8, 2, 3, 1, 2, 3].into_iter().enumerate() {
        all.push(listing(10 + k as u8, &format!("Challenger{k}"), "Elsewhere", wave));
    }
    let owners = |v: &[StageListing]| v.iter().map(|l| l.owner.clone()).collect::<Vec<_>>();
    let shown = |bonus, released| shown_stages(&all, bonus, released);
    // Off: only the 1987 stages, whatever has shipped.
    for released in 0..=3 {
        assert_eq!(shown(false, released), ten, "bonus challengers off, {released} waves out");
    }
    // On with nothing shipped (today): the same ten. Never an unreleased stage.
    assert_eq!(shown(true, 0), ten);
    assert_eq!(RELEASED_WAVES, 0, "no wave has shipped yet");
    assert_eq!(shown_stages(&all, true, RELEASED_WAVES), ten);
    // On with waves out: each released wave's stages join, in list order.
    assert_eq!(shown(true, 1).len(), 12);
    assert_eq!(owners(&shown(true, 1))[10..], ["Challenger0".to_owned(), "Challenger3".to_owned()]);
    assert_eq!(shown(true, 2).len(), 14);
    assert_eq!(shown(true, 3), all);
    assert!(shown(true, 2).iter().all(|l| l.wave <= 2), "wave 3 is not out");
    // A listing made for a wave carries it, and the 1987 listings are wave 0.
    let mut bank = BackgroundBank::new();
    let layer = LayerView { page: 0, scroll: 0, palette: 0 };
    let view = StageView { fg: layer, bg: layer, theme: 0 };
    let (one, two) = (bank.add_stage("one", view), bank.add_stage("two", view));
    bank.list_stage(one, "One", "Japan");
    bank.list_wave_stage(two, "Two", "Brazil", 1);
    let waves: Vec<u8> = bank.listed_stages().iter().map(|l| l.wave).collect();
    assert_eq!(waves, [0, 1]);
    assert_eq!(shown_stages(bank.listed_stages(), true, 0).len(), 1);
    assert_eq!(shown_stages(bank.listed_stages(), true, 1).len(), 2);
    assert_eq!(shown_stages(bank.listed_stages(), false, 1).len(), 1);
}
