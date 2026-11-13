use sf_engine::config::MatchConfig;
use sf_engine::era::{Costume, Loadout, Revision, StylePick};
use sf_engine::front::fight::{FightDips, FightKind};
use sf_engine::front::{FrameInputs, FrameOutput, GameSession, MenuItem, Transition};
use sf_engine::replay::{MatchReplay, ReplayCommand, ReplayMode, TakeoverPlayer};
use sf_engine::roster::{StagePick, TournamentFighter};
use sf_engine::session_io::ControlScheme;
use sf_video::romset::{load_assets, Assets};
use std::path::Path;

#[path = "support/select.rs"]
mod select;

#[test]
fn replay_files_preserve_style_costumes_schemes_and_packed_start() {
    for kind in [
        FightKind::Local,
        FightKind::Arcade,
        FightKind::TrainingCpu,
        FightKind::TrainingDummy,
        FightKind::Dramatic { co_op: true },
        FightKind::WorldTour,
    ] {
        for fighter in TournamentFighter::ALL {
            let replay = MatchReplay {
                id: 7,
                title: "Sparring".into(),
                kind,
                online: kind == FightKind::Local,
                config: MatchConfig {
                    p1_scheme: ControlScheme::Pneumatic,
                    p2_scheme: ControlScheme::Modern,
                    ..MatchConfig::default()
                },
                dips: FightDips {
                    difficulty: 3,
                    round_time: 150,
                },
                stage: StagePick::Bank(13),
                flag: Some(2),
                seed: 23,
                data_hash: 42,
                p1_fighter: fighter.into(),
                p2_fighter: TournamentFighter::Ken.into(),
                p1_style: StylePick::sf2(Revision::St, Loadout::Old, Costume::Sf2, 5),
                p2_style: StylePick::default(),
                frames: vec![(0x0402, 0x0200), (0x0081, 0)],
                commands: vec![(1, ReplayCommand::HoldCombo(true))],
            };
            let mut replay = replay;
            if kind == FightKind::TrainingDummy && fighter == TournamentFighter::Ryu {
                replay.frames.resize(15_001, (0, 0));
            }
            let bytes = replay.to_bytes();
            assert_eq!(&bytes[..5], b"SF1R\x01");
            assert_eq!(MatchReplay::from_bytes(&bytes), Ok(replay));
            assert!(MatchReplay::from_bytes(&bytes[..bytes.len() - 1]).is_err());
            let mut extra = bytes;
            extra.push(0);
            assert!(MatchReplay::from_bytes(&extra).is_err());
        }
    }
}

fn assets() -> Option<Assets> {
    let roms = "../../tools/reference/local/roms/sf";
    if !Path::new(roms).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(
        load_assets(roms, "../../tools/reference/rom-manifest.json")
            .expect("local ROM set assembles"),
    )
}

fn input(edit: impl FnOnce(&mut FrameInputs)) -> FrameInputs {
    let mut input = FrameInputs::default();
    edit(&mut input);
    input
}

fn stick(direction: usize) -> FrameInputs {
    input(|i| i.p1.directions[direction] = true)
}

fn confirm() -> FrameInputs {
    input(|i| i.menu.confirm = true)
}

fn tap(session: &mut GameSession, controls: FrameInputs) -> FrameOutput {
    let result = session.step(controls);
    session.step(FrameInputs::default());
    result
}

fn local_session(assets: Assets) -> GameSession {
    let mut session = GameSession::new(assets, MatchConfig::default());
    for _ in 0..200 {
        session.step(FrameInputs::default());
    }
    assert_eq!(
        tap(&mut session, input(|i| i.menu.start = true)).transition,
        Some(Transition::OpenMainMenu)
    );
    assert_eq!(
        tap(&mut session, stick(2)).transition,
        Some(Transition::CursorMoved)
    );
    assert_eq!(
        tap(&mut session, confirm()).transition,
        Some(Transition::Choose(MenuItem::MultiPlayer))
    );
    assert_eq!(
        tap(&mut session, confirm()).transition,
        Some(Transition::Choose(MenuItem::Local))
    );
    select::confirm_fighter(&mut session, input(|i| i.p1.buttons[0] = true), tap);
    select::confirm_fighter(
        &mut session,
        input(|i| i.p2.buttons[0] = true),
        GameSession::step,
    );
    while session.fight().expect("native local fight").driving() {
        session.step(FrameInputs::default());
    }
    assert_eq!(session.screen(), "country select");
    session
}

#[test]
fn a_completed_native_match_replays_its_original_checksums_and_restores_takeover() {
    let Some(assets) = assets() else { return };
    let mut session = local_session(assets);
    let mut trace = vec![session.fight().unwrap().state_checksum()];
    let mut work_at_120 = Vec::new();
    let mut completed = false;
    for frame in 0..6000 {
        let mut controls = FrameInputs::default();
        if frame == 0 {
            controls.p1.buttons[0] = true; // Pick Japan.
        } else if frame >= 60 {
            let phase = frame % 120;
            controls.p1.directions[0] = phase < 45;
            controls.p1.buttons[2] = phase >= 45 && phase % 18 < 3;
            controls.p1.buttons[5] = phase >= 45 && (phase + 9) % 18 < 3;
        }
        let out = session.step(controls);
        let fight = session.fight().expect("native match remains available");
        if trace.len() == 120 {
            work_at_120 = fight.machine().work_ram().to_vec();
        }
        trace.push(fight.state_checksum());
        if out.transition == Some(Transition::MatchOver) {
            completed = true;
            break;
        }
    }
    assert!(
        completed,
        "the scripted local match must finish before the replay cap"
    );
    let saved = session
        .take_saved_replay()
        .expect("match completion autosaves");
    assert!(
        session.take_saved_replay().is_none(),
        "match saves only once"
    );
    assert_eq!(saved.frames.len() + 1, trace.len());
    let from_disk = MatchReplay::from_bytes(&saved.to_bytes()).expect("saved replay round trips");
    assert_eq!(from_disk, saved);

    // Quit the result, then select the recording through the replay browser.
    tap(&mut session, stick(3));
    assert_eq!(tap(&mut session, confirm()).screen, "main menu");
    assert_eq!(
        tap(&mut session, stick(2)).transition,
        Some(Transition::CursorMoved)
    );
    assert_eq!(session.menu_item(), Some(MenuItem::Replays));
    assert_eq!(tap(&mut session, confirm()).screen, "replays");
    let opened = session.step(confirm());
    assert_eq!(opened.transition, Some(Transition::WatchReplay(0)));
    let replay = session.replay_playback().expect("selected replay opened");
    assert_eq!(replay.frame_index(), 0);
    assert_eq!(replay.fight().state_checksum(), trace[0]);

    // Check the first 300 linear frames against the original session.
    for (at, &checksum) in trace.iter().enumerate().take(301).skip(1) {
        session.step(FrameInputs::default());
        let replay = session.replay_playback().unwrap();
        assert_eq!(replay.frame_index(), at as u32);
        assert_eq!(
            replay.fight().state_checksum(),
            checksum,
            "recorded frame {at}"
        );
    }

    // The menu's stick and tab input reach the same 60/300-frame seek paths.
    session.step(stick(1));
    assert_eq!(session.replay_playback().unwrap().frame_index(), 240);
    session.step(input(|i| i.menu.tab_left = true));
    assert_eq!(session.replay_playback().unwrap().frame_index(), 0);
    session.step(input(|i| i.menu.tab_right = true));
    assert_eq!(session.replay_playback().unwrap().frame_index(), 300);

    // A separate playback proves the public seek API against the same trace.
    let mut playback = session.replay_playback().unwrap().clone();
    let assets = session.assets();
    for (delta, at) in [
        (-300, 0usize),
        (60, 60),
        (300, 360),
        (-120, 240),
        (-120, 120),
    ] {
        playback.scrub(assets, delta);
        assert_eq!(playback.frame_index(), at as u32);
        assert_eq!(
            playback.fight().state_checksum(),
            trace[at],
            "scrub to {at}"
        );
    }
    assert!(playback.takeover(TakeoverPlayer::P2));
    assert!(matches!(
        playback.mode(),
        ReplayMode::Takeover {
            window_frames: 600,
            ..
        }
    ));
    for _ in 0..180 {
        let pad = input(|i| {
            i.p1.directions[1] = true;
            i.p1.buttons[2] = true;
        })
        .p1;
        assert!(playback.step(assets, pad).is_some());
    }
    assert_eq!(playback.frame_index(), 300);
    assert_ne!(
        playback.fight().state_checksum(),
        trace[300],
        "P2 takeover changes the fight"
    );
    assert!(playback.rewind());
    assert_eq!(playback.frame_index(), 120);
    assert_eq!(playback.fight().state_checksum(), trace[120]);
    assert_eq!(playback.fight().machine().work_ram(), work_at_120);
    playback.resume();
    for (at, &checksum) in trace.iter().enumerate().take(301).skip(121) {
        playback
            .step(assets, Default::default())
            .expect("bookmark resumes the original replay");
        assert_eq!(
            playback.fight().state_checksum(),
            checksum,
            "resumed frame {at}"
        );
    }
}

#[test]
fn replay_applies_a_pause_menu_restart_before_the_next_recorded_pad() {
    let Some(assets) = assets() else { return };
    let mut session = local_session(assets);
    let mut trace = vec![session.fight().unwrap().state_checksum()];
    let mut restart_at = None;
    for frame in 0..550 {
        if frame >= 180 && restart_at.is_none() && session.fight().unwrap().round_start().is_some()
        {
            assert_eq!(
                tap(&mut session, input(|i| i.menu.start = true)).transition,
                Some(Transition::Paused)
            );
            for _ in 0..4 {
                tap(&mut session, stick(2));
            }
            assert_eq!(
                session.step(confirm()).transition,
                Some(Transition::Choose(MenuItem::RestartRound))
            );
            assert_eq!(session.screen(), "fight");
            restart_at = Some(frame);
        }
        let controls = if frame == 0 {
            input(|i| i.p1.buttons[0] = true)
        } else if frame > 60 && frame < 120 {
            stick(0)
        } else {
            FrameInputs::default()
        };
        session.step(controls);
        trace.push(session.fight().unwrap().state_checksum());
    }
    assert_eq!(
        tap(&mut session, input(|i| i.menu.start = true)).transition,
        Some(Transition::Paused)
    );
    for _ in 0..5 {
        tap(&mut session, stick(2));
    }
    assert_eq!(
        tap(&mut session, confirm()).transition,
        Some(Transition::Choose(MenuItem::QuitToMenu))
    );
    let replay = session
        .take_saved_replay()
        .expect("quitting a fight past three seconds autosaves");
    assert_eq!(replay.frames.len(), 550);
    assert_eq!(
        replay.commands,
        [(
            restart_at.expect("round start was saved"),
            ReplayCommand::Restart
        )]
    );
    let replay = MatchReplay::from_bytes(&replay.to_bytes()).unwrap();
    session.set_replays(vec![replay]);
    tap(&mut session, stick(2));
    assert_eq!(session.menu_item(), Some(MenuItem::Replays));
    tap(&mut session, confirm());
    assert_eq!(
        session.step(confirm()).transition,
        Some(Transition::WatchReplay(0))
    );
    for (frame, &checksum) in trace.iter().enumerate() {
        let replay = session.replay_playback().unwrap();
        assert_eq!(replay.frame_index(), frame as u32);
        assert_eq!(replay.fight().state_checksum(), checksum, "frame {frame}");
        session.step(FrameInputs::default());
    }
}
