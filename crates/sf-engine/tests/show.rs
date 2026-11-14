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

#[path = "support/select.rs"]
mod select;
#[allow(dead_code)]
#[path = "support/alpha.rs"]
mod alpha;

use std::path::Path;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::data_bank::{
    ALPHA_COUNTER_SLOT, ALPHA_SUPER_SECOND_SLOT, ALPHA_SUPER_THIRD_SLOT, SF2_SUPER_SLOT,
};
use sf_engine::era::{EraFamily, Loadout, Revision};
use sf_engine::era_fight::ST_RYU_SHINKU_FREEZE;
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::moves::Choice;
use sf_engine::front::pause::PauseBox;
use sf_engine::front::settings::Settings;
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
const LEFT: usize = 1;
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
    training_with_style(preset, fighter, dummy, None)
}

fn training_with_style(
    preset: RulesetPreset,
    fighter: TournamentFighter,
    dummy: bool,
    style: Option<(EraFamily, Loadout)>,
) -> GameSession {
    training_with_settings(preset, fighter, dummy, style, Settings::default())
}

fn training_with_settings(
    preset: RulesetPreset,
    fighter: TournamentFighter,
    dummy: bool,
    style: Option<(EraFamily, Loadout)>,
    settings: Settings,
) -> GameSession {
    let a = assets().expect("local ROMs");
    let mut s = GameSession::new(a, MatchConfig::from_preset(preset)).with_settings(settings);
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
    if let Some((family, loadout)) = style {
        assert_eq!(tap(&mut s, confirm()).transition, Some(Transition::StyleOpened));
        let family_steps = match family {
            EraFamily::Sf1Plus => 0,
            EraFamily::Sf2 => 1,
            EraFamily::SfZ => 2,
            EraFamily::Sf3 => 3,
            EraFamily::Sf4 => 4,
            EraFamily::Sf5 => 5,
            EraFamily::Sf6 => 6,
            EraFamily::Sf1987 => 7,
        };
        for _ in 0..family_steps {
            tap(&mut s, stick(RIGHT));
        }
        if matches!(loadout, Loadout::Old | Loadout::XIsm) {
            tap(&mut s, stick(UP)); // LOAD.
            tap(&mut s, stick(RIGHT));
            if loadout == Loadout::XIsm {
                tap(&mut s, stick(RIGHT));
            }
        }
        tap(&mut s, confirm());
    } else {
        select::confirm_fighter(&mut s, confirm(), tap); // P1
    }
    select::confirm_fighter(&mut s, confirm(), GameSession::step); // the opponent under the cursor
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

#[test]
fn refilled_training_supers_can_be_repeated_in_the_same_round() {
    use sf_engine::moves::{ButtonClass, Dir, MoveInput};
    use sf_video::announcer::AnnouncerLine;

    if assets().is_none() {
        return;
    }
    const DOUBLE_QCF: &[Dir] = &[Dir::Down, Dir::DownForward, Dir::Forward, Dir::Down, Dir::DownForward, Dir::Forward];
    const QCF_DP: &[Dir] = &[Dir::Down, Dir::DownForward, Dir::Forward, Dir::Down, Dir::DownForward];
    const DOUBLE_QCB: &[Dir] = &[Dir::Down, Dir::DownBack, Dir::Back, Dir::Down, Dir::DownBack, Dir::Back];
    for (fighter, family, loadout, motion, button, full_meter) in [
        (TournamentFighter::Ryu, EraFamily::SfZ, Loadout::AIsm, DOUBLE_QCF, ButtonClass::Punch, 144),
        (TournamentFighter::Ken, EraFamily::SfZ, Loadout::AIsm, QCF_DP, ButtonClass::Punch, 144),
        (TournamentFighter::Ryu, EraFamily::SfZ, Loadout::XIsm, DOUBLE_QCF, ButtonClass::Punch, 48),
        (TournamentFighter::Ken, EraFamily::Sf2, Loadout::Standard, DOUBLE_QCF, ButtonClass::Punch, 48),
        (TournamentFighter::Ryu, EraFamily::SfZ, Loadout::AIsm, DOUBLE_QCB, ButtonClass::Kick, 144),
        (TournamentFighter::Ryu, EraFamily::SfZ, Loadout::AIsm, QCF_DP, ButtonClass::Kick, 144),
    ] {
        let initial = training_with_settings(
            RulesetPreset::DefinitiveOverhaul,
            fighter,
            true,
            Some((family, loadout)),
            Settings { fill_meters: true, ..Settings::default() },
        );
        let facings: &[bool] = if fighter == TournamentFighter::Ryu { &[false, true] } else { &[false] };
        for &faces_left in facings {
            let mut session = initial.clone();
            if faces_left {
                cross_training_dummy(&mut session);
            }
            assert_eq!(p1_faces_left(&session), faces_left);
            for attempt in 0..2 {
                assert_eq!(session.fight_machine().unwrap().era_fighters()[0].super_meter, full_meter);
                let mut supers = 0;
                let input = MoveInput { motion, button };
                for pad in fast_motion(input, !p1_faces_left(&session)) {
                    let out = session.step(with(|frame| frame.p1 = pad));
                    supers += usize::from(out.announcements.contains(&AnnouncerLine::Super));
                }
                assert_eq!(supers, 1, "{fighter:?} {family:?} {loadout:?}, left={faces_left}, attempt {}", attempt + 1);
                for _ in 0..180 {
                    if session.fight_machine().unwrap().era_fighters()[0].super_meter == full_meter {
                        break;
                    }
                    session.step(idle());
                }
            }
        }
    }
}

#[test]
fn shown_super_can_be_repeated_as_soon_as_the_meter_refills() {
    use sf_video::announcer::AnnouncerLine;

    if assets().is_none() {
        return;
    }
    let initial = training_with_settings(
        RulesetPreset::DefinitiveOverhaul,
        TournamentFighter::Ryu,
        true,
        Some((EraFamily::SfZ, Loadout::AIsm)),
        Settings { fill_meters: true, ..Settings::default() },
    );
    for slot in [SF2_SUPER_SLOT, ALPHA_SUPER_SECOND_SLOT, ALPHA_SUPER_THIRD_SLOT] {
        let mut session = initial.clone();
        open_list_at(&mut session, 0);
        for _ in 0..8 {
            if session.move_list().unwrap().selected().unwrap().produces == slot {
                break;
            }
            tap(&mut session, stick(DOWN));
        }
        let entry = *session.move_list().unwrap().selected().unwrap();
        assert_eq!(entry.produces, slot);
        choose_show(&mut session);
        let full_meter = 144;
        let meter = |session: &GameSession| session.fight_machine().unwrap().era_fighters()[0].super_meter;
        let mut first_super = false;
        for _ in 0..250 {
            let frame = session.step(idle());
            first_super |= frame.announcements.contains(&AnnouncerLine::Super);
            if first_super && meter(&session) == full_meter {
                break;
            }
        }
        assert!(first_super);
        assert_eq!(meter(&session), full_meter);
        let mut repeated = false;
        for pad in fast_motion(entry.input, !p1_faces_left(&session)) {
            let frame = session.step(with(|frame| frame.p1 = pad));
            repeated |= frame.announcements.contains(&AnnouncerLine::Super);
        }
        assert!(repeated, "the first visible refill frame accepts the next {} motion", entry.name);
        assert_eq!(meter(&session), 0);
        assert_eq!(session.coach().expect("Show hands back control in Try").successes(), 1);
    }
}

fn fast_motion(input: sf_engine::moves::MoveInput, facing_right: bool) -> Vec<sf_engine::front::PlayerInputs> {
    use sf_engine::front::PlayerInputs;
    let mut steps: Vec<PlayerInputs> = Vec::new();
    for pad in input.frames(facing_right, Strength::Heavy) {
        match steps.last_mut() {
            Some(step) if step.directions == pad.directions => {
                for (pressed, held) in step.buttons.iter_mut().zip(pad.buttons) {
                    *pressed |= held;
                }
            }
            _ => steps.push(pad),
        }
    }
    steps
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

fn open_list_on(s: &mut GameSession, name: &str) {
    open_list_at(s, 0);
    for _ in 0..64 {
        if s.move_list().unwrap().selected().is_some_and(|entry| entry.name == name) {
            return;
        }
        tap(s, stick(DOWN));
    }
    panic!("missing move {name}");
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

#[test]
fn era_show_and_try_resolve_overlapping_commands_and_rapid_punches() {
    use TournamentFighter::{Adon, Gen, Retsu, Ryu};

    for (fighter, revision, loadout, name, slot) in [
        (Ryu, Revision::Sfz3, Loadout::AIsm, "HADOKEN", 0x1a),
        (Retsu, Revision::St, Loadout::Standard, "IZUNA SHOULDER THROW", 0x21),
        (Retsu, Revision::St, Loadout::Standard, "TOBI GERI", 0x1b),
        (Retsu, Revision::St, Loadout::Standard, "HUNDRED MONK SLAPS", 0x20),
        (Gen, Revision::Usf4, Loadout::Ultra1, "HYAKURENKO", 0x1a),
        (Gen, Revision::Sf6, Loadout::Standard, "HYAKU RENKOU", 0x1a),
        (Adon, Revision::Usf4, Loadout::Ultra1, "JAGUAR TOOTH", 0x20),
        (Adon, Revision::Usf4, Loadout::Ultra1, "EX JAGUAR TOOTH", 0x20),
        (Adon, Revision::Usf4, Loadout::Ultra1, "JAGUAR KICK", 0x1b),
    ] {
        let Some(mut session) = alpha::training_fight_with_fighter(fighter, revision, loadout) else {
            return;
        };
        alpha::assert_training_move_demo(&mut session, name, slot, false);
    }
}

#[test]
fn alpha_command_normal_still_starts_without_a_special_motion() {
    use sf_engine::moves::{ButtonClass, Dir, MoveInput};

    let Some(mut session) = alpha::training_fight_with_fighter(
        TournamentFighter::Ryu, Revision::Sfz3, Loadout::AIsm,
    ) else {
        return;
    };
    let command = MoveInput { motion: &[Dir::Forward], button: ButtonClass::MediumPunch };
    assert!(command.frames(true, Strength::Medium).into_iter().any(|p1| {
        session.step(FrameInputs { p1, ..FrameInputs::default() });
        p1_move(&session) == 0x1c
    }), "forward and medium punch still start Collarbone Breaker");
}

#[test]
fn era_show_lists_the_special_that_owns_a_reused_command() {
    let Some(mut session) = alpha::training_fight_with_fighter(
        TournamentFighter::Eagle, Revision::Sf5Ce, Loadout::Skill1Trigger1,
    ) else {
        return;
    };
    open_list_on(&mut session, "LIVERPOOL WHITE");
    assert!(session.move_list().unwrap().text().iter().all(|line| !line.contains("ST. ANDREW'S GREEN")));
    choose_show(&mut session);
    assert!((0..120).any(|_| {
        session.step(idle());
        p1_move(&session) == 0x22
    }));
}

#[test]
fn try_distinguishes_gens_commands_that_share_a_move_slot() {
    use sf_engine::moves::{ButtonClass, MoveInput};

    let Some(mut session) = alpha::training_fight_with_fighter(
        TournamentFighter::Gen, Revision::Sf6, Loadout::Standard,
    ) else {
        return;
    };
    open_list_on(&mut session, "SWITCH TO CRANE");
    tap(&mut session, confirm());
    tap(&mut session, stick(RIGHT));
    assert_eq!(session.move_list().unwrap().choosing(), Some(Choice::Try));
    tap(&mut session, confirm());

    for button in [ButtonClass::TripleKick, ButtonClass::TriplePunch] {
        let command = MoveInput { motion: &[], button };
        let mut switched = false;
        for p1 in command.frames(true, Strength::Medium) {
            session.step(with(|frame| frame.p1 = p1));
            switched |= p1_move(&session) == 0x1f;
        }
        assert!(switched, "both stance changes enter the same native move slot");
        let coach = session.coach().unwrap();
        assert_eq!((coach.attempts(), coach.successes()), (1, 1));
    }
}

#[test]
fn st_new_shows_and_coaches_the_native_super_and_old_hides_it() {
    if assets().is_none() {
        return;
    }
    let preset = RulesetPreset::DefinitiveOverhaul;
    let mut session = training_with_style(preset, TournamentFighter::Ryu, true, Some((EraFamily::Sf2, Loadout::Standard)));
    assert_eq!(session.fight_machine().unwrap().era_fighters()[0].super_meter, 0);
    assert!(session.fight().unwrap().era_hud().p1.is_some());
    open_list_on(&mut session, "SHINKU HADOKEN");
    choose_show(&mut session);
    let mut announced = false;
    let mut freeze = 0;
    let mut moves = std::collections::BTreeSet::new();
    for _ in 0..400 {
        let output = session.step(idle());
        announced |= output.announcements.contains(&sf_video::announcer::AnnouncerLine::Super);
        freeze = freeze.max(session.fight_machine().unwrap().era_super_freeze());
        moves.insert(p1_move(&session));
        if session.coach().is_some() {
            break;
        }
    }
    assert!(announced && moves.contains(&SF2_SUPER_SLOT) && freeze == ST_RYU_SHINKU_FREEZE - 1,
        "Show ran and announced the native Super: moves={moves:x?}, freeze={freeze}, announced={announced}");
    let entry = *session.coach().expect("completed Show enters Try").entry();
    assert_eq!(entry.produces, SF2_SUPER_SLOT);
    session.step(idle());
    assert!(session.fight_machine().unwrap().era_fighters()[0].super_meter > 0);
    open_list_on(&mut session, "SHINKU HADOKEN");
    assert_eq!(session.fight_machine().unwrap().era_fighters()[0].super_meter, 0,
        "pausing the Try restores the meter from before the Show");
    tap(&mut session, confirm());
    tap(&mut session, stick(RIGHT));
    assert_eq!(session.move_list().unwrap().choosing(), Some(Choice::Try));
    tap(&mut session, confirm());
    assert!(session.fight_machine().unwrap().era_fighters()[0].super_meter > 0);
    for input in entry.input.frames(true, Strength::Medium) {
        session.step(with(|frame| frame.p1 = input));
    }
    assert_eq!(session.coach().unwrap().successes(), 1, "Try saw the native double motion succeed");

    let mut old = training_with_style(preset, TournamentFighter::Ryu, true, Some((EraFamily::Sf2, Loadout::Old)));
    assert!(old.fight().unwrap().era_hud().p1.is_none());
    open_list_at(&mut old, 0);
    assert!(!old.move_list().unwrap().text().iter().any(|line| line.contains("SHINKU HADOKEN")));
}

#[test]
fn alpha_counter_show_and_try_supply_a_real_attack_to_guard() {
    if assets().is_none() {
        return;
    }
    for dummy in [true, false] {
        alpha_counter_training(dummy);
    }
}

fn alpha_counter_training(dummy: bool) {
    use sf_video::announcer::AnnouncerLine;

    let mut session = training_with_style(
        RulesetPreset::DefinitiveOverhaul,
        TournamentFighter::Ryu,
        dummy,
        Some((EraFamily::SfZ, Loadout::AIsm)),
    );
    assert_eq!(session.fight_machine().unwrap().era_fighters()[0].super_meter, 0);
    open_list_at(&mut session, 0);
    for _ in 0..8 {
        if session.move_list().unwrap().selected().unwrap().produces == ALPHA_COUNTER_SLOT {
            break;
        }
        tap(&mut session, stick(DOWN));
    }
    assert_eq!(session.move_list().unwrap().selected().unwrap().produces, ALPHA_COUNTER_SLOT);
    choose_show(&mut session);

    let mut saw_guard = false;
    let mut counter_calls = 0;
    for _ in 0..400 {
        let output = session.step(idle());
        saw_guard |= session.fight().unwrap().fight_read().fighters[0].blockstun;
        counter_calls += output.announcements.iter().filter(|&&line| line == AnnouncerLine::AlphaCounter).count();
        if session.coach().is_some() {
            break;
        }
    }
    assert!(saw_guard, "Show blocked the opponent's attack");
    assert_eq!(counter_calls, 1, "Show announced one real Alpha Counter");
    let entry = *session.coach().expect("Alpha Counter Show enters Try").entry();
    assert_eq!(entry.produces, ALPHA_COUNTER_SLOT);

    for _ in 0..240 {
        if session.fight().unwrap().fight_read().fighters[0].blockstun {
            break;
        }
        let away = if p1_faces_left(&session) { RIGHT } else { LEFT };
        session.step(stick(away));
    }
    assert!(session.fight().unwrap().fight_read().fighters[0].blockstun);
    for input in entry.input.frames(!p1_faces_left(&session), Strength::Medium) {
        session.step(with(|frame| frame.p1 = input));
    }
    assert_eq!(session.coach().unwrap().successes(), 1, "Try saw the guard cancel");
    open_list_at(&mut session, 0);
    assert_eq!(session.fight_machine().unwrap().era_fighters()[0].super_meter, 0);
}

#[test]
fn alpha_super_show_uses_the_selected_isms_meter_and_enters_try() {
    if assets().is_none() {
        return;
    }
    for (loadout, remaining) in [(Loadout::AIsm, 48), (Loadout::XIsm, 0)] {
        check_alpha_super_training(
            TournamentFighter::Ryu,
            loadout,
            SF2_SUPER_SLOT,
            "SHINKUU HADOKEN",
            remaining,
        );
    }
}

#[test]
fn alpha_alternate_supers_can_be_shown_and_practiced() {
    if assets().is_none() {
        return;
    }
    for (fighter, slot, name, remaining) in [
        (TournamentFighter::Ryu, ALPHA_SUPER_SECOND_SLOT, "METSU SHORYUKEN", 0),
        (TournamentFighter::Ryu, ALPHA_SUPER_THIRD_SLOT, "SHINKUU TATSUMAKI", 48),
        (TournamentFighter::Ken, ALPHA_SUPER_SECOND_SLOT, "SHINRYUKEN", 48),
        (TournamentFighter::Ken, ALPHA_SUPER_THIRD_SLOT, "SHIPPU JINRAIKYAKU", 0),
    ] {
        check_alpha_super_training(fighter, Loadout::AIsm, slot, name, remaining);
    }
}

fn check_alpha_super_training(
    fighter: TournamentFighter,
    loadout: Loadout,
    slot: u8,
    name: &str,
    remaining: u8,
) {
    use sf_video::announcer::AnnouncerLine;

    let mut session = training_with_style(
        RulesetPreset::DefinitiveOverhaul,
        fighter,
        true,
        Some((EraFamily::SfZ, loadout)),
    );
    open_list_at(&mut session, 0);
    for _ in 0..8 {
        if session.move_list().unwrap().selected().unwrap().produces == slot {
            break;
        }
        tap(&mut session, stick(DOWN));
    }
    assert_eq!(session.move_list().unwrap().selected().unwrap().name, name);
    choose_show(&mut session);
    let mut announced = 0;
    for _ in 0..400 {
        let output = session.step(idle());
        if output.announcements.contains(&AnnouncerLine::Super) {
            announced += 1;
            assert_eq!(session.fight_machine().unwrap().era_fighters()[0].super_meter, remaining, "{name}");
        }
        if session.coach().is_some() {
            break;
        }
    }
    assert_eq!(announced, 1, "Show announced {name} once in {loadout:?}");
    let entry = *session.coach().expect("Show enters Try").entry();
    assert_eq!(entry.name, name);
    for input in entry.input.frames(!p1_faces_left(&session), Strength::Medium) {
        session.step(with(|frame| frame.p1 = input));
    }
    assert_eq!(session.coach().unwrap().successes(), 1, "Try observed {name}");
    open_list_at(&mut session, 0);
    assert_eq!(session.fight_machine().unwrap().era_fighters()[0].super_meter, 0);
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
    select::confirm_fighter(&mut s, confirm(), tap);
    select::confirm_fighter(&mut s, with(|i| i.p2.buttons[LP] = true), tap);
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
        // Start when the Show can feed its first input immediately.
        for f in 0..2_000 {
            let [p1, _] = base.fight().unwrap().fight_read().fighters;
            if ready(&p1) {
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
        // Mirror the pad motion as the player turns during a CPU fight.
        let frames = [false, true].map(|facing| entry.input.frames(facing, Strength::Medium));
        let at = shown.fight_machine().unwrap().state.frame;
        assert_eq!(padded.fight_machine().unwrap().state.frame, at);
        for n in 0..frames[0].len() + 60 {
            let machine = padded.fight_machine().unwrap();
            let facing = sf_engine::moves::facing_right(
                machine.work_ram(), &machine.state.objectram, sf_engine::fighter::P1_STRUCT,
            );
            let input = frames[usize::from(facing)].get(n).copied().unwrap_or_default();
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
    cross_training_dummy(&mut s);
    show_move(&mut s, 0);
    let mut came_out = false;
    while s.fight().unwrap().showing().is_some() {
        s.step(idle());
        came_out |= p1_move(&s) == REGISTRY[0].produces;
    }
    assert!(came_out, "the Hadoken comes out facing left");
}

fn cross_training_dummy(s: &mut GameSession) {
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
    while p1_move(s) != 1 {
        s.step(idle());
    }
    assert!(p1_faces_left(s));
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

#[test]
fn sf5_show_uses_the_actual_v_meter_and_scripted_defense() {
    if assets().is_none() {
        return;
    }
    let mut session = training_with_style(
        RulesetPreset::DefinitiveOverhaul,
        TournamentFighter::Ryu,
        true,
        Some((EraFamily::Sf5, Loadout::Skill1Trigger1)),
    );
    use sf_video::era_hud::{Fill, GaugeRow};
    let initial = GaugeRow::stock(2, 0, Fill::of(0, 48));
    let gauge = |session: &GameSession| session.fight_machine().unwrap().era_fighters()[0]
        .gauge().unwrap().second.unwrap();
    assert_eq!(gauge(&session), initial);
    let mut fill = session.fight().unwrap().clone();
    fill.set_fill_meters(true);
    fill.step(session.assets(), Default::default(), Default::default(), false);
    let filled = &fill.machine().era_fighters()[0];
    assert_eq!(filled.super_meter, 144);
    assert_eq!(filled.gauge().unwrap().second, Some(GaugeRow::stock(2, 2, Fill::EMPTY)));
    for (name, slot) in [("DENJIN RENKI", 0x2a), ("HASHOGEKI", 0x2d), ("FUMIKOMI JODAN KAIRIGERI", 0x2f)] {
        open_list_at(&mut session, 0);
        for _ in 0..35 {
            if session.move_list().unwrap().selected().unwrap().name == name {
                break;
            }
            tap(&mut session, stick(DOWN));
        }
        assert_eq!(session.move_list().unwrap().selected().unwrap().name, name);
        choose_show(&mut session);
        let mut moves = std::collections::BTreeSet::new();
        for _ in 0..700 {
            session.step(idle());
            moves.insert(p1_move(&session));
            if session.coach().is_some() { break; }
        }
        assert!(moves.contains(&slot), "{name} Show reached {moves:x?}");
        assert_eq!(session.coach().expect("Show transitions to Try").entry().name, name);
        assert_eq!(session.step(start()).transition, Some(Transition::Paused));
        session.step(idle());
        assert_eq!(gauge(&session), initial, "{name} restored the reserve after Try");
        tap(&mut session, start());
    }
}
