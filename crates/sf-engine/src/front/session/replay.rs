use super::super::draw::fit;
use super::*;
use crate::netplay::encode_inputs;
use crate::replay::{TakeoverPlayer, MAX_REPLAY_COMMANDS, MAX_REPLAY_FRAMES, REPLAY_LIMIT};
use crate::utility::RosterFighter;
use sf_video::compositor::ink;

#[derive(Clone)]
pub(super) struct Watching {
    playback: ReplayPlayback,
    pub(super) pause: Option<PauseBox>,
}

pub(super) fn record_frame(
    recording: &mut Option<MatchReplay>,
    p1: PlayerInputs,
    p2: PlayerInputs,
    start: bool,
) {
    if let Some(replay) = recording.as_mut() {
        if replay.frames.len() <= MAX_REPLAY_FRAMES {
            replay.frames.push((
                encode_inputs(p1) | (u16::from(start) << 10),
                encode_inputs(p2),
            ));
        }
    }
}

impl Play {
    pub(super) fn record_command(&mut self, command: ReplayCommand) {
        if let Some(replay) = &mut self.recording {
            if replay.frames.len() < MAX_REPLAY_FRAMES
                && replay.commands.len() <= MAX_REPLAY_COMMANDS
            {
                replay.commands.push((replay.frames.len() as u32, command));
            }
        }
    }

    pub(super) fn stop_recorded_try(&mut self) {
        if self.fight.try_active() {
            self.record_command(ReplayCommand::StopTry);
            self.fight.stop_try();
        }
    }
}

impl GameSession {
    pub fn with_replays(mut self, replays: Vec<MatchReplay>) -> Self {
        self.set_replays(replays);
        self
    }

    pub fn set_replays(&mut self, mut replays: Vec<MatchReplay>) {
        replays.sort_unstable_by_key(|r| std::cmp::Reverse(r.id));
        replays.dedup_by_key(|r| r.id);
        self.replays = replays
            .into_iter()
            .take(REPLAY_LIMIT)
            .map(Arc::new)
            .collect();
    }

    /// End an active recording when the game host closes the window.
    pub fn finish_recording(&mut self) {
        self.finish_replay_recording(false);
    }

    pub fn take_saved_replay(&mut self) -> Option<MatchReplay> {
        self.saved_replays
            .pop_front()
            .map(|r| Arc::unwrap_or_clone(r))
    }

    pub fn replay_playback(&self) -> Option<&ReplayPlayback> {
        self.playback.as_ref().map(|watching| &watching.playback)
    }

    pub(super) fn seed_replay_fight(fight: &mut Fight, seed: u64) {
        fight.seed_utility_cpu(seed);
        if fight.setup().config.dramatic.is_some() {
            fight.seed_dramatic_cpu(seed);
        }
    }

    pub(super) fn begin_replay_recording(
        &self,
        fight: &Fight,
        flag: Option<u8>,
        seed: u64,
    ) -> MatchReplay {
        let setup = fight.setup();
        let config = setup.config;
        let id = self.replays.front().map_or(1, |r| r.id.saturating_add(1));
        let [p1_style, p2_style] = fight.replay_styles();
        let p1_fighter = fight.player_fighter(0);
        let p2_fighter = fight.player_fighter(1);
        MatchReplay {
            id,
            title: match config.dramatic {
                Some(rules) => format!(
                    "{} & {} VS {}",
                    p1_fighter.name(),
                    rules.ally2.name(),
                    config.roster.p2.name()
                ),
                None if setup.kind == FightKind::Arcade => {
                    let name = config
                        .evil_ryu_playtest
                        .map_or(p1_fighter.name(), |form| form.display_name());
                    format!("{name} ARCADE RUN")
                }
                None => format!("{} VS {}", p1_fighter.name(), p2_fighter.name()),
            }
            .to_ascii_uppercase(),
            kind: setup.kind,
            online: self.net_setup.is_some(),
            config,
            dips: FightDips::from_dsw2(setup.dsw2),
            stage: config.roster.stage.unwrap_or(StagePick::Bank(0)),
            flag,
            seed,
            data_hash: fight_data_hash(
                &self.assets.maincpu,
                fight.machine().data_bank(),
                fight.machine().background_bank(),
            ),
            p1_fighter: if config.evil_ryu_playtest.is_some() {
                RosterFighter::EvilRyu
            } else {
                p1_fighter.into()
            },
            p2_fighter: p2_fighter.into(),
            p1_style,
            p2_style,
            theme: fight.selected_theme(),
            frames: Vec::new(),
            commands: if fight.hold_combo() {
                vec![(0, ReplayCommand::HoldCombo(true))]
            } else {
                Vec::new()
            },
        }
    }

    pub(super) fn finish_replay_recording(&mut self, completed: bool) {
        let Some(play) = self.play.as_mut() else {
            return;
        };
        let Some(mut replay) = play.recording.take() else {
            return;
        };
        replay.theme = play.fight.selected_theme();
        let confirmed_inputs = play
            .netplay
            .as_ref()
            .map(|rollback| {
                let inputs = rollback.confirmed_inputs();
                // The host can confirm idle frames while delivering the terminal
                // event and while the spectator drains its jitter buffer.
                let end = rollback.confirmed_events().iter()
                    .find(|(_, event)| *event == FightEvent::MatchOver)
                    .map_or(inputs.len(), |(frame, _)| *frame as usize + 1);
                &inputs[..end]
            })
            .or_else(|| {
                self.net_setup
                    .as_ref()
                    .filter(|setup| setup.local_side.is_none())
                    .map(|_| self.net_spectators.played_inputs())
            });
        if let Some(inputs) = confirmed_inputs {
            let count = inputs
                .len()
                .min(MAX_REPLAY_FRAMES + 1 + super::super::fight::DRIVE_BLACK_FRAMES as usize);
            replay.frames.extend_from_slice(
                &inputs[(super::super::fight::DRIVE_BLACK_FRAMES as usize).min(count)..count],
            );
        }
        if replay.frames.is_empty() || (!completed && replay.frames.len() < 180) {
            return;
        }
        replay
            .commands
            .retain(|&(frame, _)| (frame as usize) < replay.frames.len());
        if replay.frames.len() > MAX_REPLAY_FRAMES || replay.commands.len() > MAX_REPLAY_COMMANDS {
            self.log
                .push("replays: fight exceeded the recording limits".into());
            self.notice = Some("FIGHT TOO LONG TO SAVE REPLAY".into());
            return;
        }
        let replay = Arc::new(replay);
        self.replays.push_front(replay.clone());
        self.replays.truncate(REPLAY_LIMIT);
        self.saved_replays.push_back(replay);
        while self.saved_replays.len() > REPLAY_LIMIT {
            self.saved_replays.pop_front();
        }
    }

    pub(super) fn watch_replay(&mut self, index: usize) {
        let Some(replay) = self.replays.get(index).map(|r| r.as_ref().clone()) else {
            return;
        };
        let setup = replay.setup();
        let overhaul = setup.config.preset != RulesetPreset::Faithful1987;
        if let Some(StagePick::Bank(index)) = setup.config.roster.stage {
            if !self
                .backgrounds
                .as_ref()
                .is_some_and(|b| b.stage(index).is_some())
            {
                self.notice = Some("REPLAY STAGE IS UNAVAILABLE".into());
                return;
            }
        }
        let data = overhaul.then(|| {
            self.data_bank
                .clone()
                .unwrap_or_else(|| Arc::new(DataBank::build(&self.assets.maincpu, &[])))
        });
        let backgrounds = self.backgrounds.clone().filter(|_| overhaul);
        if replay.data_hash
            != fight_data_hash(
                &self.assets.maincpu,
                data.as_deref(),
                backgrounds.as_deref(),
            )
        {
            self.notice = Some("REPLAY NEEDS ITS ORIGINAL GAME DATA".into());
            return;
        }
        let Ok(mut fight) =
            Fight::start_banked(&self.assets.maincpu, setup, replay.flag, backgrounds, data)
        else {
            self.notice = Some("CANNOT START REPLAY".into());
            return;
        };
        fight.select_theme(replay.theme);
        fight.set_satsui_art(self.satsui_art.clone());
        fight.set_selected_styles([Some(replay.p1_style), Some(replay.p2_style)]);
        fight.set_gauge_art(self.settings.gauge_art);
        fight.set_screen_shake(self.settings.screen_shake);
        Self::seed_replay_fight(&mut fight, replay.seed);
        let assets = fight_assets(&self.assets, &self.bank_assets, setup.config.preset);
        while fight.driving() {
            fight.step(
                assets,
                PlayerInputs::default(),
                PlayerInputs::default(),
                false,
            );
        }
        self.notice = None;
        self.sounds = [sound::STOP_ALL].into();
        self.playback = Some(Watching {
            playback: ReplayPlayback::new(replay, fight),
            pause: None,
        });
    }

    pub(super) fn step_replay(
        &mut self,
        prev: &FrameInputs,
        input: &FrameInputs,
        events: &[MenuInput],
    ) -> FrameOutput {
        let mut watching = self.playback.take().expect("watching a replay");
        let preset = watching.playback.fight().setup().config.preset;
        let assets = fight_assets(&self.assets, &self.bank_assets, preset);
        let mut ff = None;
        let mut sound_latch = sound::IDLE;
        let mut exit = false;
        let mut used_input = false;
        for &event in events {
            if let Some(pause) = watching.pause.as_mut() {
                match pause.apply(event) {
                    Some(Transition::Choose(MenuItem::Resume)) => {
                        watching.pause = None;
                        watching.playback.resume();
                    }
                    Some(Transition::Choose(
                        item @ (MenuItem::TakeoverP1 | MenuItem::TakeoverP2),
                    )) => {
                        let side = if item == MenuItem::TakeoverP1 {
                            TakeoverPlayer::P1
                        } else {
                            TakeoverPlayer::P2
                        };
                        if watching.playback.takeover(side) {
                            watching.pause = None;
                        }
                    }
                    Some(Transition::Choose(MenuItem::RewindBookmark)) => {
                        watching.playback.rewind();
                        watching.pause = None;
                    }
                    Some(Transition::Choose(MenuItem::Overlays)) => {
                        let fight = watching.playback.fight_mut();
                        fight.set_overlay(fight.overlay().next());
                        pause.note_value_change(MenuItem::Overlays, event, self.frames);
                    }
                    Some(Transition::Choose(MenuItem::Look)) => {
                        let n = self.looks.len();
                        if n > 0 {
                            let before = super::super::pause::short_look_name(self.look());
                            let at = self
                                .looks
                                .iter()
                                .position(|l| l.id == self.look())
                                .unwrap_or(0);
                            self.settings.crt_look = self.looks
                                [(at + if event == MenuInput::Left { n - 1 } else { 1 }) % n]
                                .id
                                .clone();
                            self.failed_look = None;
                            self.settings_changed = true;
                            if before != super::super::pause::short_look_name(self.look()) {
                                pause.note_value_change(MenuItem::Look, event, self.frames);
                            }
                        }
                    }
                    Some(Transition::Choose(MenuItem::QuitToReplays)) => {
                        exit = true;
                    }
                    Some(Transition::CursorMoved) => {
                        sound_latch = sound::CURSOR;
                    }
                    _ => {}
                }
                used_input = true;
                break;
            }
            if watching.playback.in_branch() && event == MenuInput::TabLeft {
                watching.playback.rewind();
                used_input = true;
                break;
            }
            if watching.playback.in_branch()
                && event == MenuInput::Back
                && !edge(prev.menu.back, input.menu.back)
            {
                continue;
            }
            let delta = match event {
                MenuInput::Left => -60,
                MenuInput::Right => 60,
                MenuInput::TabLeft => -300,
                MenuInput::TabRight => 300,
                MenuInput::Start | MenuInput::Back => {
                    watching.playback.pause();
                    let takeover = [TakeoverPlayer::P1, TakeoverPlayer::P2]
                        .map(|side| watching.playback.can_take_over(side));
                    watching.pause = Some(PauseBox::open_replay(
                        watching.playback.has_bookmark(),
                        takeover,
                    ));
                    used_input = true;
                    break;
                }
                _ => continue,
            };
            if !watching.playback.in_branch() {
                watching.playback.scrub(assets, delta);
                used_input = true;
                break;
            }
        }
        if !used_input && watching.pause.is_none() {
            ff = watching.playback.step(assets, input.p1);
        }
        let font = RomFont::new(&self.assets.gfx4, &self.assets.maincpu);
        let fight = watching.playback.fight();
        let (mut frame, latch, latch_cycles, announcements, theme, theme_frame, impacts, clear_impacts) = match ff {
            Some(ff) => (ff.frame, ff.sound_latch, ff.latch_cycles, ff.announcements, ff.theme, ff.theme_frame, ff.impacts, ff.clear_impacts),
            None => (fight.render(assets), sound_latch, 0, Vec::new(), None, None, Vec::new(), used_input),
        };
        if let ReplayMode::Takeover {
            player,
            window_frames,
            elapsed_frames,
            ..
        } = watching.playback.mode()
        {
            let tenths = window_frames.saturating_sub(elapsed_frames).div_ceil(6);
            let side = if player == TakeoverPlayer::P1 { 1 } else { 2 };
            let banner = format!(
                "TAKEOVER P{side}  {}.{}S  TAB L REWIND",
                tenths / 10,
                tenths % 10
            );
            font.text_centred(&mut frame.argb, 8, &fit(&banner, 42), ink::YELLOW);
        }
        if watching.pause.is_none() && watching.playback.mode() == ReplayMode::Paused {
            let help = if watching.playback.in_branch() {
                "TAKEOVER ENDED   TAB L REWIND"
            } else {
                "L/R 1S   TAB L/R 5S   START PAUSE"
            };
            font.text_centred(&mut frame.argb, 8, help, ink::YELLOW);
        }
        if let Some(pause) = &watching.pause {
            frame = pause.render_with_training_at(
                &font,
                &frame,
                fight.overlay().label(),
                &super::super::pause::short_look_name(self.look()),
                fight.gauge_art(),
                preset,
                false,
                false,
                None,
                None,
                self.frames,
            );
        }
        if exit {
            self.sounds.push_back(sound::STOP_SAMPLES);
            if let Some(Screen::Replays(screen)) = self.stack.last() {
                frame = screen.render_with_title(&font, self.replays.iter(), None, 2);
                self.title_started_at = Some(self.frames);
            }
        } else {
            self.playback = Some(watching);
        }
        FrameOutput {
            frame,
            sound_frame: self.frames,
            sound_latch: if exit { sound::STOP_ALL } else { latch },
            latch_cycles,
            theme: theme.filter(|_| !exit),
            theme_frame: theme_frame.filter(|_| !exit),
            impacts: if exit { Vec::new() } else { impacts },
            clear_impacts,
            announcements,
            overhaul_fight: !exit && preset != RulesetPreset::Faithful1987,
            tune_table: preset != RulesetPreset::Faithful1987,
            track: self.track_cue(if exit { sound::STOP_ALL } else { latch }, theme.filter(|_| !exit)),
            screen: self.screen(),
            transition: None,
            quit: false,
            settings_changed: std::mem::take(&mut self.settings_changed),
            bindings_changed: std::mem::take(&mut self.bindings_changed),
            video: self.video(),
            panel: None,
        }
    }
}
