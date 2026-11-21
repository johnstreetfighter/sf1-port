//! Ticket #214 (`SC22`, spec #192): with the developer-only switch for the
//! stronger Fair CPU on, a replay takeover's opponent is the Fair CPU.
//!
//! At the headless session's seam: a replay recorded in a local session is
//! opened through the Replays menu of a session started with the switch on
//! or off, and taken over through its playback. Player 1 and Player 2 are
//! both Ryu in the SF6 era style. Local ROMs only; each test skips without
//! them.

use std::path::Path;
use std::sync::OnceLock;

use sf_engine::config::MatchConfig;
use sf_engine::era::{EraFamily, Loadout, Revision, StylePick};
use sf_engine::fair_cpu::{perceive, Judged, Level, FIGHT_LOOP};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::fight::FightKind;
use sf_engine::front::{FrameInputs, GameSession, MenuItem, PlayerInputs, Transition};
use sf_engine::replay::{MatchReplay, ReplayCommand, ReplayPlayback, TakeoverOpponent, TakeoverPlayer};
use sf_engine::roster::{PlayMode, StagePick, TournamentFighter};
use sf_engine::utility::Drill;
use sf_video::callouts::Side;
use sf_video::romset::{load_assets, Assets};
use sf_video::background_bank::BackgroundBank;
use sf_video::stage::pack_records;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
/// Frames the takeover runs for in these tests, inside its 600-frame window.
const BRANCH: u32 = 400;

fn assets() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(load_assets(ROMS, MANIFEST).expect("local ROM set assembles"))
}

fn input(edit: impl FnOnce(&mut FrameInputs)) -> FrameInputs {
    let mut input = FrameInputs::default();
    edit(&mut input);
    input
}

fn confirm() -> FrameInputs {
    input(|i| i.menu.confirm = true)
}

fn tap(session: &mut GameSession, controls: FrameInputs) -> Option<Transition> {
    let result = session.step(controls).transition;
    session.step(FrameInputs::default());
    result
}

/// The shared test stage, packed as the session's background bank: the
/// replay's data must match the session that opens it.
fn background_bank(assets: &Assets) -> BackgroundBank {
    let stage = include_str!("../../../art/stages/test-stage.json")
        .replace(r#""name": "test-stage""#, r#""name": "gouken""#);
    pack_records(assets, &[&stage]).expect("shared stage fixture packs")
}

/// A session at its title, with the test stage.
fn base_session(assets: Assets, stronger: bool) -> GameSession {
    let bank = background_bank(&assets);
    let index = bank.stage_index("gouken").unwrap();
    let mut session = GameSession::new(assets, MatchConfig::default())
        .with_background_bank(bank)
        .with_stronger_cpu(stronger);
    session.set_stage(Some(StagePick::Bank(index)));
    for _ in 0..200 {
        session.step(FrameInputs::default());
    }
    session
}

/// A replay a local session wrote, for its header: its data hash matches
/// a session built by [`base_session`]. Its pads and rules are replaced by
/// each test.
fn template() -> Option<MatchReplay> {
    static TEMPLATE: OnceLock<Option<MatchReplay>> = OnceLock::new();
    TEMPLATE
        .get_or_init(|| {
            let mut session = base_session(assets()?, false);
            assert_eq!(tap(&mut session, input(|i| i.menu.start = true)), Some(Transition::OpenMainMenu));
            while session.menu_item() != Some(MenuItem::MultiPlayer) {
                tap(&mut session, input(|i| i.p1.directions[2] = true));
            }
            assert_eq!(tap(&mut session, confirm()), Some(Transition::Choose(MenuItem::MultiPlayer)));
            assert_eq!(tap(&mut session, confirm()), Some(Transition::Choose(MenuItem::Local)));
            for _ in 0..3_000 {
                if session.fight().is_some_and(|f| !f.driving()) {
                    break;
                }
                // Each side confirms its pick, and Player 1 the country.
                session.step(input(|i| {
                    i.p1.buttons[0] = true;
                    i.p2.buttons[0] = true;
                }));
                session.step(FrameInputs::default());
            }
            for _ in 0..240 {
                session.step(FrameInputs::default());
            }
            session.finish_recording();
            Some(session.take_saved_replay().expect("a local fight of 240 frames is saved"))
        })
        .clone()
}

fn sf6() -> StylePick {
    StylePick { family: EraFamily::Sf6, revision: Revision::Sf6, loadout: Loadout::Standard, ..StylePick::default() }
}

/// A recording of `frames` pads: Ryu against Ryu, both SF6, at the
/// match `difficulty`, of `kind`.
fn recording(kind: FightKind, difficulty: u8, frames: Vec<(u16, u16)>, commands: Vec<(u32, ReplayCommand)>) -> Option<MatchReplay> {
    let mut replay = template()?;
    replay.kind = kind;
    let stage = replay.config.roster.stage;
    replay.config = MatchConfig::definitive();
    replay.config.roster.p1 = sf_engine::roster::TournamentFighter::Ryu;
    replay.config.roster.p2 = sf_engine::roster::TournamentFighter::Ryu;
    replay.config.roster.stage = stage;
    replay.config.difficulty = difficulty;
    if matches!(kind, FightKind::TrainingCpu | FightKind::TrainingDummy) {
        replay.config.roster.mode = PlayMode::Training;
        replay.config.p2_fair_cpu = kind == FightKind::TrainingCpu;
    }
    if let FightKind::Dramatic { co_op } = kind {
        // Ryu and his ally Ryu against the boss Ken.
        replay.config.roster.mode = PlayMode::Versus;
        replay.config.roster.p2 = TournamentFighter::Ken;
        replay.config.dramatic = Some(sf_engine::dramatic::DramaticRules {
            co_op,
            ally2: TournamentFighter::Ryu,
            stage: TournamentFighter::Ken,
            lifebar: sf_engine::dramatic::LifebarMode::Independent,
            friendly_fire: false,
            styles: [Revision::Sf6; 3],
            rivalry: None,
        });
    }
    replay.p1_fighter = sf_engine::roster::TournamentFighter::Ryu.into();
    replay.p2_fighter = sf_engine::roster::TournamentFighter::Ryu.into();
    replay.p1_style = sf6();
    replay.p2_style = sf6();
    replay.theme = None;
    replay.frames = frames;
    replay.commands = commands;
    Some(replay)
}

/// The recording opened through the Replays menu of a session started with
/// the switch as given: its playback, at the replay's first frame.
fn open(replay: MatchReplay, stronger: bool) -> (GameSession, ReplayPlayback) {
    let assets = assets().expect("checked by the template");
    let mut session = base_session(assets, stronger).with_replays(vec![replay]);
    assert_eq!(tap(&mut session, input(|i| i.menu.start = true)), Some(Transition::OpenMainMenu));
    while session.menu_item() != Some(MenuItem::Replays) {
        tap(&mut session, input(|i| i.p1.directions[2] = true));
    }
    assert!(tap(&mut session, confirm()).is_some());
    assert_eq!(session.screen(), "replays");
    assert_eq!(session.step(confirm()).transition, Some(Transition::WatchReplay(0)));
    let playback = session.replay_playback().expect("the replay opened").clone();
    assert_eq!(playback.frame_index(), 0);
    (session, playback)
}

/// Player 1 walks toward Player 2 for two seconds, then both stand.
fn pads(len: usize) -> Vec<(u16, u16)> {
    // Bit 0 of the packed pad is right (`encode_inputs`).
    (0..len).map(|i| (u16::from(i < 120), 0)).collect()
}

fn in_fight(playback: &ReplayPlayback) -> bool {
    flow_position(playback.fight().machine()) == FIGHT_LOOP
}

/// The replay frame a second into the first round: where these tests take
/// over.
fn takeover_frame(kind: FightKind) -> Option<u32> {
    let (session, mut playback) = open(recording(kind, 4, pads(3_000), Vec::new())?, false);
    let assets = session.assets();
    while !in_fight(&playback) {
        assert!(playback.step(assets, PlayerInputs::default()).is_some(), "the round starts inside the recording");
    }
    Some(playback.frame_index() + 60)
}

fn step_to(playback: &mut ReplayPlayback, assets: &Assets, frame: u32) {
    while playback.frame_index() < frame {
        playback.step(assets, PlayerInputs::default()).expect("the recording reaches the takeover");
    }
    assert!(in_fight(playback), "the takeover is in the round");
}

/// The frame as the side `side` (0 Player 1) sees it.
fn seen(playback: &ReplayPlayback, side: usize) -> sf_engine::fair_cpu::Percept {
    let m = playback.fight().machine();
    perceive(m.state.frame, true, m.work_ram(), &m.state.objectram, [Side::P1, Side::P2][side])
}

/// What one takeover did, frame by frame.
struct Branch {
    /// The machine's frame when the player took over.
    from: u32,
    checksums: Vec<u64>,
    opponents: Vec<Option<TakeoverOpponent>>,
    /// The opponent Fair CPU's pad, and the frame its last view of the
    /// opponent's actions came from (0: it has had no view).
    pads: Vec<Option<(u16, u32)>>,
    judged: Vec<Option<Judged>>,
    /// Where the opponent's fighter stood, and whether it attacked.
    opponent_x: Vec<i16>,
    opponent_attacked: bool,
}

/// Take over `side` at `at` and play `live` for [`BRANCH`] frames.
fn branch(playback: &mut ReplayPlayback, assets: &Assets, side: TakeoverPlayer, live: impl Fn(u32) -> PlayerInputs) -> Branch {
    let from = playback.fight().machine().state.frame;
    assert!(playback.takeover(side));
    let other = usize::from(side == TakeoverPlayer::P1);
    let mut b = Branch {
        from,
        checksums: Vec::new(),
        opponents: Vec::new(),
        pads: Vec::new(),
        judged: Vec::new(),
        opponent_x: Vec::new(),
        opponent_attacked: false,
    };
    for i in 0..BRANCH {
        assert!(playback.step(assets, live(i)).is_some(), "takeover frame {i}");
        b.checksums.push(playback.fight().state_checksum());
        b.opponents.push(playback.takeover_opponent());
        b.pads.push(playback.takeover_cpu().map(|cpu| (cpu.pad(), cpu.attention().action_frame)));
        b.judged.push(playback.takeover_cpu().map(|cpu| cpu.situation()));
        let v = seen(playback, other);
        b.opponent_x.push(v.me.x);
        b.opponent_attacked |= v.me.attacking();
    }
    b
}

/// A recording that ends one frame after the takeover point, so the
/// takeover's opponent is handed to a CPU on the takeover's second frame
/// (the recording has no pads beyond it).
fn short_recording(difficulty: u8) -> Option<(MatchReplay, u32)> {
    let at = takeover_frame(FightKind::Local)?;
    Some((recording(FightKind::Local, difficulty, pads(at as usize + 1), Vec::new())?, at))
}

#[test]
fn taking_over_either_side_gives_a_fair_cpu_opponent_on_the_other() {
    let Some((replay, at)) = short_recording(4) else { return };
    for side in [TakeoverPlayer::P1, TakeoverPlayer::P2] {
        let (session, mut on) = open(replay.clone(), true);
        let assets = session.assets();
        step_to(&mut on, assets, at);
        let off = on.clone();
        let fair = branch(&mut on, assets, side, |_| PlayerInputs::default());
        // The recorded pad plays the takeover's first frame; from the
        // second the Fair CPU plays the other side.
        assert_eq!(fair.opponents[0], Some(TakeoverOpponent::Recorded), "{side:?}");
        assert!(fair.opponents[1..].iter().all(|&o| o == Some(TakeoverOpponent::FairCpu)), "{side:?}: {:?}", fair.opponents);
        // It plays the stronger game: it judges situations, presses, and
        // its fighter moves and attacks.
        assert!(fair.judged.iter().flatten().any(|&j| j != Judged::default()), "{side:?}: it judges a situation");
        assert!(fair.pads.iter().flatten().any(|&(pad, _)| pad != 0), "{side:?}: it presses");
        let start = fair.opponent_x[0];
        assert!(fair.opponent_x.iter().any(|&x| x != start), "{side:?}: its fighter moves");
        assert!(fair.opponent_attacked, "{side:?}: its fighter attacks");

        // The same takeover with the session's switch off, from the same
        // frame, plays the Utility AI there, and the fights differ.
        let (session_off, from_off) = open(replay.clone(), false);
        drop(session_off);
        let mut utility = from_off;
        step_to(&mut utility, assets, at);
        assert_eq!(utility.fight().state_checksum(), off.fight().state_checksum(), "the switch leaves playback alone");
        let old = branch(&mut utility, assets, side, |_| PlayerInputs::default());
        assert_eq!(old.opponents[0], Some(TakeoverOpponent::Recorded));
        assert!(old.opponents[1..].iter().all(|&o| o == Some(TakeoverOpponent::UtilityAi)), "{side:?}: {:?}", old.opponents);
        assert!(old.pads.iter().all(Option::is_none), "{side:?}: no Fair CPU with the switch off");
        assert_ne!(old.checksums, fair.checksums, "{side:?}: the opponents play differently");
        // Rewinding drops the Fair CPU with the branch.
        assert!(on.rewind());
        assert_eq!(on.takeover_cpu().map(|_| ()), None);
        assert_eq!(on.takeover_opponent(), None);
    }
}

#[test]
fn the_opponent_plays_at_the_match_level_and_sees_nothing_from_before_the_takeover() {
    for difficulty in [1, 4] {
        let Some((replay, at)) = short_recording(difficulty) else { return };
        let delay = Level::from_difficulty(difficulty).reaction();
        for side in [TakeoverPlayer::P1, TakeoverPlayer::P2] {
            let (session, mut playback) = open(replay.clone(), true);
            let assets = session.assets();
            step_to(&mut playback, assets, at);
            let b = branch(&mut playback, assets, side, |_| PlayerInputs::default());
            // Its view of the fight begins with the takeover's first frame
            // and shows a frame `delay` old: it decides from a view first
            // on the takeover's frame `delay + 1` (index `delay`), and gives
            // an empty pad before.
            let first_view = b.pads.iter().position(|p| p.is_some_and(|(_, seen)| seen != 0));
            assert_eq!(first_view, Some(delay as usize), "difficulty {difficulty}, {side:?}: {:?}", &b.pads[..30]);
            assert!(b.pads[..delay as usize].iter().all(|&p| p == Some((0, 0))), "difficulty {difficulty}, {side:?}");
            // Every view it decides from is of a frame after the takeover.
            for (i, &(_, seen)) in b.pads.iter().flatten().enumerate() {
                assert!(seen == 0 || seen > b.from, "difficulty {difficulty}, {side:?}: frame {i} decided from {seen}, took over at {}", b.from);
            }
        }
    }
}

#[test]
fn the_opponent_watches_from_the_takeover_while_the_recording_still_plays_it() {
    // A full recording: Player 2 takes over and walks in to attack where
    // the recording stood, so the takeover runs on the recorded Player 1
    // until the hits make the fight diverge, and the Fair CPU, watching
    // all along, plays him after.
    let Some(at) = takeover_frame(FightKind::Local) else { return };
    let Some(replay) = recording(FightKind::Local, 4, pads(3_000), Vec::new()) else { return };
    let (session, mut playback) = open(replay, true);
    let assets = session.assets();
    step_to(&mut playback, assets, at);
    let b = branch(&mut playback, assets, TakeoverPlayer::P2, |i| {
        let mut pad = PlayerInputs::default();
        // Player 2 is on the right: hold left to walk in, then attack.
        pad.directions[1] = i < 90;
        pad.buttons[i as usize % 6] = i >= 40 && i % 12 < 2;
        pad
    });
    let recorded = b.opponents.iter().take_while(|&&o| o == Some(TakeoverOpponent::Recorded)).count();
    assert!(b.opponents[..recorded].iter().all(|&o| o == Some(TakeoverOpponent::Recorded)));
    // The Fair CPU has watched every frame since the takeover, and decides
    // from those alone.
    assert!(b.pads.iter().all(Option::is_some));
    for &(_, seen) in b.pads.iter().flatten() {
        assert!(seen == 0 || seen > b.from, "decided from {seen}, took over at {}", b.from);
    }
    assert!(recorded > 1 && recorded < BRANCH as usize, "the recording plays him, then the fight diverges: {recorded}");
    {
        assert!(b.opponents[recorded..].iter().all(|&o| o == Some(TakeoverOpponent::FairCpu)), "{:?}", b.opponents);
    }
    // While the recording plays the opponent, the Fair CPU gives no pad.
    assert!(b.pads[..recorded].iter().flatten().all(|&(pad, _)| pad == 0));
    // It takes him over with a view already: on the first frame it plays
    // him it decides from a frame it watched while the recording played,
    // its reaction delay old, not from nothing.
    let delay = Level::VeryHard.reaction() as usize;
    assert!(recorded > delay, "the recording plays him past the reaction delay: {recorded}");
    let (_, decided_from) = b.pads[recorded].unwrap();
    assert!(decided_from > b.from, "decided from {decided_from}, took over at {}", b.from);
    assert!(
        decided_from <= b.from + (recorded - delay) as u32 + 1,
        "its first view is of a frame the recording played: {decided_from}, took over at {}, recorded for {recorded}",
        b.from,
    );
}

#[test]
fn taking_over_a_one_player_arcade_replay_gives_a_fair_cpu_computer_opponent() {
    // The computer opponent's struct takes the Fair CPU's pad, as it took
    // the Utility AI's before.
    // The first rung's opponent is Joe. A computer opponent takes a pad
    // only with a command table, and Joe has none in the ROM's data, so
    // Player 1 plays Joe himself: with the full cast Ryu stands in for him.
    let joe = |mut replay: MatchReplay| {
        replay.config.roster.full_cast = true;
        replay.config.roster.p1 = TournamentFighter::Joe;
        replay.p1_fighter = TournamentFighter::Joe.into();
        replay.p1_style = StylePick::default();
        replay
    };
    let Some(probe) = recording(FightKind::Arcade, 4, pads(3_000), Vec::new()) else { return };
    let (session, mut playback) = open(joe(probe), true);
    while !in_fight(&playback) {
        assert!(playback.step(session.assets(), PlayerInputs::default()).is_some());
    }
    let at = playback.frame_index() + 60;
    drop((session, playback));
    let replay = joe(recording(FightKind::Arcade, 4, pads(at as usize + 1), Vec::new()).unwrap());
    let (session, mut playback) = open(replay, true);
    let assets = session.assets();
    step_to(&mut playback, assets, at);
    assert!(playback.can_take_over(TakeoverPlayer::P1));
    assert_eq!(playback.fight().machine().arcade_opponent(), Some(TournamentFighter::Ryu));
    let b = branch(&mut playback, assets, TakeoverPlayer::P1, |_| PlayerInputs::default());
    assert_eq!(b.opponents[0], Some(TakeoverOpponent::Recorded));
    assert!(b.opponents[1..].iter().all(|&o| o == Some(TakeoverOpponent::FairCpu)), "{:?}", b.opponents);
    assert!(b.judged.iter().flatten().any(|&j| j != Judged::default()), "it judges a situation");
    assert!(b.pads.iter().flatten().any(|&(pad, _)| pad != 0), "it presses");
    let start = b.opponent_x[0];
    assert!(b.opponent_x.iter().any(|&x| x != start), "the computer's fighter moves");
    assert!(b.opponent_attacked, "the computer's fighter attacks");
}

#[test]
fn the_same_takeover_point_and_inputs_give_the_same_fight() {
    let Some((replay, at)) = short_recording(4) else { return };
    let (session, mut playback) = open(replay.clone(), true);
    let assets = session.assets();
    step_to(&mut playback, assets, at);
    let live = |i: u32| {
        let mut pad = PlayerInputs::default();
        pad.directions[0] = i % 90 < 40;
        pad.buttons[1] = i % 45 == 20;
        pad
    };
    let base = playback.clone();
    let mut twin = playback.clone();
    let first = branch(&mut playback, assets, TakeoverPlayer::P1, live);
    let second = branch(&mut twin, assets, TakeoverPlayer::P1, live);
    assert_eq!(first.checksums, second.checksums, "two takeovers from one point");
    assert!(first.pads.iter().flatten().any(|&(pad, _)| pad != 0), "the Fair CPU played");

    // Rewound to the bookmark and taken over again, it plays the same.
    assert!(playback.rewind());
    let again = branch(&mut playback, assets, TakeoverPlayer::P1, live);
    assert_eq!(first.checksums, again.checksums, "after a rewind");

    // Player 2 taken over from the same point: the same again, with the
    // Fair CPU playing Player 1.
    let live_p2 = |i: u32| {
        let mut pad = PlayerInputs::default();
        pad.directions[1] = i % 90 < 40;
        pad.buttons[4] = i % 45 == 20;
        pad
    };
    let mut p2 = base.clone();
    let mut p2_twin = base.clone();
    let p2_first = branch(&mut p2, assets, TakeoverPlayer::P2, live_p2);
    let p2_second = branch(&mut p2_twin, assets, TakeoverPlayer::P2, live_p2);
    assert_eq!(p2_first.checksums, p2_second.checksums, "two Player 2 takeovers from one point");
    assert!(p2_first.pads.iter().flatten().any(|&(pad, _)| pad != 0), "the Fair CPU played Player 1");
    assert_ne!(p2_first.checksums, first.checksums, "Player 2's takeover is its own fight");
    assert!(p2.rewind());
    let p2_again = branch(&mut p2, assets, TakeoverPlayer::P2, live_p2);
    assert_eq!(p2_first.checksums, p2_again.checksums, "Player 2's takeover after a rewind");

    // A session that opens the replay afresh plays the same from there.
    let (fresh_session, mut fresh) = open(replay, true);
    drop(fresh_session);
    step_to(&mut fresh, assets, at);
    let fresh = branch(&mut fresh, assets, TakeoverPlayer::P1, live);
    assert_eq!(first.checksums, fresh.checksums, "in a new session");

    // A copy taken mid-branch goes on the same as the original.
    let (_, mut half) = open(recording(FightKind::Local, 4, pads(at as usize + 1), Vec::new()).unwrap(), true);
    step_to(&mut half, assets, at);
    assert!(half.takeover(TakeoverPlayer::P1));
    for i in 0..150 {
        half.step(assets, live(i));
    }
    let mut copy = half.clone();
    for i in 150..BRANCH {
        half.step(assets, live(i));
        copy.step(assets, live(i));
        assert_eq!(half.fight().state_checksum(), copy.fight().state_checksum(), "copy at {i}");
        assert_eq!(half.fight().state_checksum(), first.checksums[i as usize], "mid-branch copy at {i}");
    }
}

#[test]
fn scripted_drills_and_combo_trials_still_play_the_opponent_after_a_takeover() {
    for (kind, command) in [
        (FightKind::TrainingCpu, ReplayCommand::Drill(Some(Drill::ThrowTech))),
        (FightKind::TrainingCpu, ReplayCommand::Drill(Some(Drill::AntiAir))),
        (FightKind::TrainingDummy, ReplayCommand::Trial(Some(1))),
    ] {
        let Some(at) = takeover_frame(kind) else { return };
        // Player 1 stands throughout; the drill or trial is on from the
        // start.
        let replay = recording(kind, 4, vec![(0, 0); 3_000], vec![(0, command)]).unwrap();
        let mut runs = Vec::new();
        for stronger in [false, true] {
            let (session, mut playback) = open(replay.clone(), stronger);
            let assets = session.assets();
            step_to(&mut playback, assets, at);
            // Player 1 is taken over and plays the recorded (empty) pad, so
            // nothing diverges: the opponent stays the recording's, the
            // drill's script or the trial's dummy.
            let b = branch(&mut playback, assets, TakeoverPlayer::P1, |_| PlayerInputs::default());
            assert!(b.opponents.iter().all(|&o| o == Some(TakeoverOpponent::Recorded)), "{kind:?} {command:?} switch {stronger}");
            runs.push(b.checksums);
        }
        assert_eq!(runs[0], runs[1], "{kind:?} {command:?}: the switch changes nothing");
    }
}

/// The replay's playback built as `watch_replay` builds it, without a
/// session: under an overhaul preset with the ROM's data bank, under
/// Faithful 1987 with none.
fn direct_playback(assets: &Assets, replay: MatchReplay, stronger: bool) -> ReplayPlayback {
    use sf_engine::config::RulesetPreset;
    use sf_engine::data_bank::DataBank;
    use sf_engine::front::fight::Fight;
    let data = (replay.config.preset != RulesetPreset::Faithful1987)
        .then(|| std::sync::Arc::new(DataBank::build(&assets.maincpu, &[])));
    let mut fight = Fight::start_banked(&assets.maincpu, replay.setup(), replay.flag, None, data).expect("the replay's fight starts");
    while fight.driving() {
        fight.step(assets, PlayerInputs::default(), PlayerInputs::default(), false);
    }
    ReplayPlayback::new(replay, fight).with_stronger_cpu(stronger)
}

#[test]
fn a_faithful_1987_replay_keeps_its_takeover_opponent_with_the_switch_on() {
    // Faithful 1987 has no Fair CPU: its takeover's opponent stays the
    // Utility AI it was (the maintainer's call). The same replay under
    // the Definitive Overhaul, built the same way, gets the Fair CPU.
    let Some(assets) = assets() else { return };
    let Some(template) = template() else { return };
    for (config, expected) in [
        (MatchConfig::faithful(), TakeoverOpponent::UtilityAi),
        (MatchConfig::definitive(), TakeoverOpponent::FairCpu),
    ] {
        let mut replay = template.clone();
        replay.kind = FightKind::Local;
        replay.config = config;
        replay.config.difficulty = 4;
        replay.theme = None;
        replay.commands = Vec::new();
        replay.frames = pads(3_000);
        let mut probe = direct_playback(&assets, replay.clone(), true);
        while !in_fight(&probe) {
            assert!(probe.step(&assets, PlayerInputs::default()).is_some(), "{:?}: the round starts", config.preset);
        }
        let at = probe.frame_index() + 60;
        replay.frames = pads(at as usize + 1);
        for side in [TakeoverPlayer::P1, TakeoverPlayer::P2] {
            let mut playback = direct_playback(&assets, replay.clone(), true);
            step_to(&mut playback, &assets, at);
            assert_eq!(playback.fight().machine().match_config().preset, config.preset);
            let b = branch(&mut playback, &assets, side, |_| PlayerInputs::default());
            assert_eq!(b.opponents[0], Some(TakeoverOpponent::Recorded), "{:?} {side:?}", config.preset);
            assert!(b.opponents[1..].iter().all(|&o| o == Some(expected)), "{:?} {side:?}: {:?}", config.preset, b.opponents);
            assert_eq!(b.pads.iter().all(Option::is_some), expected == TakeoverOpponent::FairCpu, "{:?} {side:?}", config.preset);
            assert_eq!(b.pads.iter().any(Option::is_some), expected == TakeoverOpponent::FairCpu, "{:?} {side:?}", config.preset);
        }
    }
}

#[test]
fn a_dramatic_battle_replay_keeps_the_utility_ai_as_its_takeover_opponent() {
    // Dramatic Battle's CPUs keep the old play until SC23 (`#215`): with
    // the switch on, the ally or Player 1 left to the machine is the
    // Utility AI's, and it plays.
    let kind = FightKind::Dramatic { co_op: true };
    let Some(at) = takeover_frame(kind) else { return };
    assert!(at > 120, "both sides' recorded pads are empty from the takeover on");
    let replay = recording(kind, 4, pads(at as usize + 1), Vec::new()).unwrap();
    // The control: the same takeover in the full recording, where the
    // player plays the recorded (empty) pad, nothing diverges, and the
    // other side replays its recorded empty pad.
    let full = recording(kind, 4, pads(3_000), Vec::new()).unwrap();
    for side in [TakeoverPlayer::P1, TakeoverPlayer::P2] {
        let (session, mut playback) = open(replay.clone(), true);
        let assets = session.assets();
        step_to(&mut playback, assets, at);
        assert!(playback.fight().machine().match_config().dramatic.is_some());
        let b = branch(&mut playback, assets, side, |_| PlayerInputs::default());
        assert_eq!(b.opponents[0], Some(TakeoverOpponent::Recorded), "{side:?}");
        assert!(b.opponents[1..].iter().all(|&o| o == Some(TakeoverOpponent::UtilityAi)), "{side:?}: {:?}", b.opponents);
        assert!(b.pads.iter().all(Option::is_none), "{side:?}: no Fair CPU in Dramatic Battle");
        let (control_session, mut control) = open(full.clone(), true);
        drop(control_session);
        step_to(&mut control, assets, at);
        let idle = branch(&mut control, assets, side, |_| PlayerInputs::default());
        assert!(idle.opponents.iter().all(|&o| o == Some(TakeoverOpponent::Recorded)), "{side:?}: the control stays on the recording");
        // The two differ only in the other side's pad: the Utility AI's
        // against an empty one. It plays, so the fights part.
        assert_eq!(b.checksums[0], idle.checksums[0], "{side:?}: the takeover's first frame is the recording's in both");
        assert_ne!(b.checksums, idle.checksums, "{side:?}: the Utility AI plays the other side");
    }
}
