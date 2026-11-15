use sf_engine::config::MatchConfig;
use sf_engine::era::{Costume, Loadout, Revision, StylePick};
use sf_engine::front::fight::{FightDips, FightKind, MatchTheme};
use sf_engine::front::{FrameInputs, FrameOutput, GameSession, MenuItem, Transition};
use sf_engine::replay::{MatchReplay, ReplayCommand, ReplayMode, TakeoverPlayer};
use sf_engine::roster::{StagePick, TournamentFighter};
use sf_engine::theme::{ThemeCue, ThemeId};
use sf_video::stage::pack_records;
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
                theme: (kind == FightKind::Local).then_some(MatchTheme {
                    id: sf_engine::theme::ThemeId::Akuma,
                    phase: 1,
                }),
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
    let stage = include_str!("../../../art/stages/test-stage.json")
        .replace(r#""name": "test-stage""#, r#""name": "gouken""#);
    let bank = pack_records(&assets, &[&stage]).expect("shared stage fixture packs");
    let index = bank.stage_index("gouken").unwrap();
    let mut session = GameSession::new(assets, MatchConfig::default()).with_background_bank(bank);
    session.set_stage(Some(StagePick::Bank(index)));
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
    let mut themes = Vec::new();
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
        if let Some(theme) = out.theme {
            themes.push((frame, out.theme_frame.unwrap(), out.latch_cycles, theme));
        }
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
    let expected = Some(MatchTheme { id: ThemeId::Gouken, phase: 0 });
    assert_eq!(saved.theme, expected);
    assert_eq!(themes.first().map(|(_, _, _, theme)| *theme),
        Some(ThemeCue { id: ThemeId::Gouken, fallback: 0x29 }));
    assert!(themes.iter().all(|(_, _, _, theme)| theme.id == ThemeId::Gouken));
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
    assert_eq!(replay.fight().selected_theme(), expected);
    assert_eq!(replay.fight().state_checksum(), trace[0]);

    // Check the first 300 linear frames against the original session.
    for (at, &checksum) in trace.iter().enumerate().take(301).skip(1) {
        let out = session.step(FrameInputs::default());
        let theme = themes.iter().find(|(frame, _, _, _)| *frame == at - 1);
        assert_eq!(out.theme.map(|cue| (out.theme_frame.unwrap(), out.latch_cycles, cue)),
            theme.map(|&(_, frame, cycles, cue)| (frame, cycles, cue)), "recorded theme at {at}");
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
        assert_eq!(playback.fight().selected_theme(), expected);
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
    assert_eq!(playback.fight().selected_theme(), expected);
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
fn correcting_the_inputs_on_a_theme_frame_does_not_play_the_theme_twice() {
    use sf_engine::front::PlayerInputs;
    use sf_engine::netplay::{encode_inputs, rollback::RollbackEngine};

    let Some(assets) = assets() else { return };
    let session = local_session(assets);
    let assets = session.fight_assets().unwrap();
    let mut direct = session.fight().unwrap().clone();
    let mut predicted = direct.clone();
    let mut rollback = RollbackEngine::new(0, 0, 8);
    let mut direct_themes = Vec::new();
    let mut predicted_themes = Vec::new();
    for frame in 0..1500 {
        let mut p1 = PlayerInputs::default();
        p1.buttons[0] = frame == 0;
        let mut p2 = PlayerInputs::default();
        // Every input differs from the preceding confirmed input. This
        // corrects the frame that queued the theme and the frame it latched.
        p2.directions[0] = frame % 2 == 0;
        let actual = direct.step(assets, p1, p2, false);
        let guessed = rollback.step(&mut predicted, assets, p1).unwrap();
        rollback.apply_remote_inputs(&mut predicted, assets, frame, &[encode_inputs(p2)]);
        assert!(rollback.last_rollback_frames() > 0);
        assert_eq!(predicted.state_checksum(), direct.state_checksum());
        if let Some(theme) = actual.theme {
            let original_frame = direct.machine().state.frame;
            assert_eq!(actual.theme_frame, Some(original_frame));
            direct_themes.push((frame, original_frame, actual.latch_cycles, theme));
        }
        if let Some(theme) = guessed.theme {
            predicted_themes.push((frame, guessed.theme_frame.unwrap(), guessed.latch_cycles, theme));
        }
        if direct_themes.first().is_some_and(|&(first, _, _, _)| frame >= first + 10) {
            break;
        }
    }
    assert_eq!(direct_themes.len(), 1);
    assert_eq!(predicted_themes, direct_themes);
}

#[test]
fn replay_applies_a_pause_menu_restart_before_the_next_recorded_pad() {
    let Some(assets) = assets() else { return };
    let mut session = local_session(assets);
    let mut trace = vec![session.fight().unwrap().state_checksum()];
    let mut restart_at = None;
    let theme = session.fight().unwrap().selected_theme().unwrap();
    assert_eq!(theme.id, ThemeId::Gouken);
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
            assert_eq!(session.fight().unwrap().selected_theme(), Some(theme));
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
    assert_eq!(replay.theme, Some(theme));
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
