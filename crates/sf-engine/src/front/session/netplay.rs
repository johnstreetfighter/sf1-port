//! Online session flow. Transport membership stays in Connection; gameplay
//! advances only through the same pure screens and Fight used locally.

use std::collections::BTreeSet;

use super::*;
use crate::front::settings::NetplayHudMode;
use crate::netplay::protocol::LobbyPhase;
use crate::netplay::{AfterMatchChoice, NetPacket, RollbackEvent};

/// Namespaces remain retired when a room closes or a former client becomes host.
#[derive(Clone, Default)]
pub(super) struct EpochCursor {
    generation: u32,
    wire_tip: u32,
}

impl EpochCursor {
    fn observe(&mut self, generation: u32, wire_tip: u32) {
        self.generation = self.generation.max(generation);
        self.wire_tip = self.wire_tip.max(wire_tip);
    }
}

#[derive(Clone, Debug)]
pub(super) struct Flow {
    generation: u32,
    frame_base: u32,
    seq: u32,
    remote_seq: [Option<u32>; 2],
    local_picks: [Option<NetPacket>; 2],
    last_lobby_sent: u32,
    pending_inputs: Vec<NetPacket>,
    peer_ack: Option<u32>,
    remote_ack: Option<u32>,
    received: BTreeSet<u32>,
    votes: [Option<AfterMatchChoice>; 2],
    local_vote: Option<NetPacket>,
    remote_vote_seq: Option<u32>,
    last_vote_sent: u32,
    decision_sent: bool,
    dramatic_offer: Option<(u32, SyncedMatchRules)>,
}

impl NetplaySetup {
    pub(super) fn new(
        local_side: Option<usize>,
        delay: u8,
        max_rollback: u8,
        generation: u32,
        frame_base: u32,
    ) -> Self {
        Self {
            local_side,
            delay,
            max_rollback,
            flow: Flow {
                generation,
                frame_base,
                seq: 0,
                remote_seq: [None; 2],
                local_picks: [None, None],
                last_lobby_sent: 0,
                pending_inputs: Vec::new(),
                peer_ack: None,
                remote_ack: None,
                received: BTreeSet::new(),
                votes: [None; 2],
                local_vote: None,
                remote_vote_seq: None,
                last_vote_sent: 0,
                decision_sent: false,
                dramatic_offer: None,
            },
        }
    }
}

impl GameSession {
    pub(super) fn net_quit(&mut self) {
        let combatant = self
            .net_setup
            .as_ref()
            .is_some_and(|setup| setup.local_side.is_some());
        if let (Some(setup), Some(rollback), Some(connection)) = (
            self.net_setup.as_ref(),
            self.play.as_ref().and_then(|play| play.netplay.as_ref()),
            self.net_connection.as_mut(),
        ) {
            if let Some(frame) = setup
                .flow
                .frame_base
                .checked_add(rollback.next_frame())
                .filter(|frame| *frame != u32::MAX)
            {
                let mut bits = [0; 9];
                bits[0] = 0x8000;
                connection.send_to_player(NetPacket::Input {
                    current_frame: frame,
                    start_frame: frame,
                    count: 1,
                    bits,
                    ack_frame: u32::MAX,
                    frame_advantage: 0,
                    checksum_frame: u32::MAX,
                    checksum: 0,
                });
                self.net_epoch_cursor.observe(setup.flow.generation, frame);
            }
        }
        self.net_return_online(if combatant {
            "PEER LEFT ROOM"
        } else {
            "LEFT ROOM"
        });
    }

    pub(super) fn net_rollback_events(&mut self) {
        let events = self
            .play
            .as_mut()
            .and_then(|play| play.netplay.as_mut())
            .map(|rollback| rollback.drain_events())
            .unwrap_or_default();
        for event in events {
            match event {
                RollbackEvent::WaitingForPeer => {}
                RollbackEvent::PeerDisconnected | RollbackEvent::PeerForfeit { .. } => {
                    self.net_return_online("PEER LEFT ROOM");
                    return;
                }
                RollbackEvent::Desync {
                    frame,
                    local,
                    remote,
                } => {
                    self.log.push(format!("netplay: desync at frame {frame}, local {local:016x}, remote {remote:016x}"));
                    if let Some(play) = self.play.as_ref() {
                        let next = play
                            .netplay
                            .as_ref()
                            .map_or(frame, |rollback| rollback.next_frame());
                        self.log.push(format!(
                            "netplay: desync snapshot {:?}",
                            play.fight.save_netplay_snapshot(next)
                        ));
                    }
                    self.net_return_online(&format!("DESYNC DETECTED AT FRAME {frame}"));
                    return;
                }
            }
        }
    }

    pub(super) fn net_fight_started(&mut self) {
        let (Some(connection), Some(play)) = (self.net_connection.as_mut(), self.play.as_ref())
        else {
            return;
        };
        if self.net_setup.is_none() {
            return;
        }
        let previous = connection.rules();
        let stage = self.chosen.unwrap_or(previous.stage);
        let config = play.fight.setup().config;
        let mut rules = SyncedMatchRules::new(
            config,
            [config.roster.p1, config.roster.p2],
            previous.dips,
            stage,
            previous.seed,
            previous.bank_hash,
            previous.art_pack_hash,
        );
        if config.dramatic.is_none() && config.era_select_enabled() {
            rules.styles = [play.fight.fighter_style(0), play.fight.fighter_style(1)];
        }
        connection.set_rules(rules);
        connection.set_lobby_phase(LobbyPhase::Stage, self.picker as u8);
        connection.set_in_match(true);
    }

    pub(super) fn net_host(
        &mut self,
        rules: SyncedMatchRules,
        room: RoomAnnouncement,
        port: u16,
    ) -> bool {
        let reservation = self
            .net_epoch_cursor
            .generation
            .checked_add(1)
            .zip(self.net_epoch_cursor.wire_tip.checked_add(64))
            .filter(|(_, base)| *base != u32::MAX);
        let Some((generation, frame_base)) = reservation else {
            if let Some(Screen::Online(screen)) = self.stack.last_mut() {
                screen.set_notice("ROOM COULD NOT BE OPENED");
            }
            return false;
        };
        let mut connection = Connection::host(
            rules,
            room,
            port,
            self.settings.netplay_input_delay.frames(),
        );
        if !connection.set_epoch(generation, frame_base) {
            return false;
        }
        self.net_epoch_cursor.observe(generation, frame_base);
        self.net_connection = Some(connection);
        true
    }

    pub(super) fn net_after_match_input(&mut self, events: &[MenuInput]) -> Option<Transition> {
        let Some(side) = self.net_setup.as_ref()?.local_side else {
            for &event in events {
                if event == MenuInput::Back {
                    self.net_return_online("LEFT ROOM");
                    return Some(Transition::Back);
                }
                match self.play.as_mut()?.after.as_mut()?.apply(event) {
                    Some(Transition::Choose(MenuItem::Menu)) => {
                        self.net_return_online("LEFT ROOM");
                        return Some(Transition::Choose(MenuItem::Menu));
                    }
                    Some(Transition::CursorMoved) => return Some(Transition::CursorMoved),
                    _ => {}
                }
            }
            return None;
        };
        for &event in events {
            let applied = self.play.as_mut()?.after.as_mut()?.apply(event);
            let Some(applied) = applied else { continue };
            if let Transition::Choose(item) = applied {
                let choice = match item {
                    MenuItem::Rematch => AfterMatchChoice::Rematch,
                    MenuItem::Reselect => AfterMatchChoice::Reselect,
                    MenuItem::Menu => AfterMatchChoice::Leave,
                    _ => continue,
                };
                let flow = &mut self.net_setup.as_mut()?.flow;
                let seq = flow.seq.checked_add(1)?;
                flow.seq = seq;
                flow.votes[side] = Some(choice);
                let packet = NetPacket::AfterMatchVote {
                    generation: flow.generation,
                    seq,
                    choice,
                };
                flow.local_vote = Some(packet.clone());
                flow.last_vote_sent = self.frames;
                if let Some(connection) = self.net_connection.as_mut() {
                    connection.send_to_player(packet);
                }
                self.net_update_votes();
                if choice == AfterMatchChoice::Leave {
                    self.net_return_online("PEER LEFT ROOM");
                } else {
                    self.net_vote_decision();
                }
                self.sounds.push_back(sound::CONFIRM);
            } else if applied == Transition::CursorMoved {
                self.sounds.push_back(sound::CURSOR);
            }
            return Some(applied);
        }
        None
    }

    fn net_update_votes(&mut self) {
        if let (Some(setup), Some(after)) = (
            self.net_setup.as_ref(),
            self.play.as_mut().and_then(|play| play.after.as_mut()),
        ) {
            after.set_peer_votes(setup.flow.votes);
        }
    }

    fn net_vote_decision(&mut self) {
        let (Some(connection), Some(setup)) =
            (self.net_connection.as_ref(), self.net_setup.as_ref())
        else {
            return;
        };
        if !connection.is_host() || setup.flow.decision_sent {
            return;
        }
        if setup.flow.votes.contains(&Some(AfterMatchChoice::Leave)) {
            self.net_return_online("PEER LEFT ROOM");
            return;
        }
        let [Some(p1), Some(p2)] = setup.flow.votes else {
            return;
        };
        let phase = if p1 == AfterMatchChoice::Rematch && p2 == AfterMatchChoice::Rematch {
            LobbyPhase::Stage
        } else {
            LobbyPhase::Fighters
        };
        self.net_advance_epoch(phase);
    }

    fn net_advance_epoch(&mut self, phase: LobbyPhase) {
        let Some(connection) = self.net_connection.as_ref() else { return };
        let mut rules = *connection.rules();
        rules.seed = rules.seed.wrapping_add(1);
        if phase == LobbyPhase::Fighters {
            rules.styles = [None; 2];
        }
        self.net_commit_epoch(rules, phase);
    }

    fn net_commit_epoch(&mut self, rules: SyncedMatchRules, phase: LobbyPhase) {
        let (Some(connection), Some(setup)) =
            (self.net_connection.as_ref(), self.net_setup.as_ref())
        else {
            return;
        };
        if !connection.is_host() || setup.flow.decision_sent {
            return;
        }
        let latest = self
            .play
            .as_ref()
            .and_then(|play| play.netplay.as_ref())
            .map_or(0, |rollback| {
                rollback
                    .latest_local_frame()
                    .unwrap_or(rollback.next_frame())
            });
        if let Some(tip) = setup.flow.frame_base.checked_add(latest) {
            self.net_epoch_cursor.observe(setup.flow.generation, tip);
        }
        let Some(frame_base) = self
            .net_epoch_cursor
            .wire_tip
            .checked_add(64)
            .filter(|base| *base != u32::MAX)
        else {
            return;
        };
        if let Some(generation) = self
            .net_connection
            .as_mut()
            .expect("connected room")
            .advance_epoch(rules, frame_base, phase, self.picker as u8)
        {
            self.net_epoch_cursor.observe(generation, frame_base);
            self.net_setup
                .as_mut()
                .expect("online setup")
                .flow
                .decision_sent = true;
            // Commit the host's Ready before a later tick can associate old
            // Fight history with the newly allocated generation.
            self.connection_output();
        }
    }

    fn net_stage_back(&mut self) {
        if self
            .net_connection
            .as_ref()
            .is_some_and(Connection::is_host)
        {
            self.net_advance_epoch(LobbyPhase::Fighters);
            return;
        }
        let Some(setup) = self.net_setup.as_ref() else {
            return;
        };
        let Some(side) = setup.local_side else { return };
        let Some(Screen::Stage(screen)) = self.stack.last() else {
            return;
        };
        let (_, p1, p2) = screen.fight();
        let stage = screen
            .cursor()
            .map(|stage| StagePick::Bank(stage.index))
            .unwrap_or(StagePick::Bank(screen.stages()[0].index));
        let flow = &mut self.net_setup.as_mut().expect("online setup").flow;
        let Some(seq) = flow.seq.checked_add(1) else {
            return;
        };
        flow.seq = seq;
        let packet = NetPacket::LobbyPick {
            generation: flow.generation,
            seq,
            side: side as u8,
            fighter: if side == 0 { p1 } else { p2 },
            stage,
            locked: false,
            phase: LobbyPhase::Fighters,
            random: false,
            style: None,
        };
        flow.local_picks[0] = Some(packet.clone());
        flow.last_lobby_sent = self.frames;
        flow.decision_sent = true;
        if let Some(connection) = self.net_connection.as_mut() {
            connection.send_to_player(packet);
        }
    }

    pub(super) fn net_return_online(&mut self, notice: &str) {
        if let Some(config) = self.play.as_ref().map(|play| play.fight.setup().config) {
            self.stop_extended_theme(&config);
        }
        if let Some(connection) = self.net_connection.as_mut() {
            self.net_commands.extend(connection.drain_commands());
        }
        self.net_commands.push(NetTransportCommand::Close);
        self.net_connection = None;
        self.net_setup = None;
        self.net_spectators = spectator::State::default();
        self.play = None;
        self.stack
            .retain(|screen| matches!(screen, Screen::Menu(_) | Screen::Online(_)));
        if let Some(Screen::Online(screen)) = self.stack.last_mut() {
            screen.set_notice(notice);
        }
        if let Some(config) = self.net_saved_config.take() {
            self.config = config;
        }
        self.notice = Some(notice.into());
    }

    pub(super) fn net_hud(&self, frame: &mut Frame) {
        let Some(rollback) = self.play.as_ref().and_then(|play| play.netplay.as_ref()) else {
            return;
        };
        if self.settings.netplay_hud == NetplayHudMode::Off {
            return;
        }
        let Some(connection) = self.net_connection.as_ref() else {
            return;
        };
        let info = connection.player_info();
        let measured = info.filter(|info| info.rtt_ms != 0);
        let ping = measured.map_or_else(|| "--MS".into(), |info| format!("{}MS", info.rtt_ms));
        let jitter = measured.map_or_else(|| "--MS".into(), |info| format!("{}MS", info.jitter_ms));
        let link = if connection.local_link() == Link::Ethernet {
            "[ETH]"
        } else {
            "[WIFI]"
        };
        let rollback_frames = rollback.last_rollback_frames();
        let delay = rollback.input_delay();
        let text = if rollback.is_waiting() {
            "WAITING FOR PEER...".into()
        } else {
            match self.settings.netplay_hud {
            NetplayHudMode::Compact => format!("PING {ping}  RBF {rollback_frames}F  DLY {delay}F  {link}"),
            NetplayHudMode::Full => format!("PING {ping}  JIT {jitter}  RBF {rollback_frames}/{}F  DLY {delay}F  ADV {:+}F  {link}",
                rollback.max_rollback(), rollback.frame_advantage()),
            NetplayHudMode::Off => return,
        }
        };
        sf_video::compositor::fallback_text(&mut frame.argb, 2, 2, &text, 0xffff_d438, 1);
    }

    pub(super) fn net_ready(
        &mut self,
        rules: SyncedMatchRules,
        local_side: Option<usize>,
        delay: u8,
        max_rollback: u8,
        generation: u32,
        frame_base: u32,
        phase: LobbyPhase,
        picker: u8,
    ) {
        self.net_epoch_cursor.observe(generation, frame_base);
        self.net_saved_config.get_or_insert(self.config);
        let schemes = (self.config.p1_scheme, self.config.p2_scheme);
        self.config = rules.config;
        (self.config.p1_scheme, self.config.p2_scheme) = schemes;
        self.chosen = Some(rules.stage);
        self.picker = usize::from(picker);
        self.play = None;
        self.lost_pad = None;
        self.net_spectators.reset(generation);
        self.net_setup = Some(NetplaySetup::new(
            local_side,
            delay,
            max_rollback,
            generation,
            frame_base,
        ));
        self.stack
            .retain(|screen| matches!(screen, Screen::Menu(_) | Screen::Online(_)));
        self.notice = None;
        self.log.push(format!(
            "netplay: joined generation {generation}, wire frame base {frame_base}, delay {delay}F"
        ));
        if let Some(dramatic) = rules.config.dramatic {
            if phase == LobbyPhase::Stage {
                if !self.start_dramatic(self.config) {
                    let notice = self.notice.take().unwrap_or_else(|| "CO-OP FIGHT COULD NOT START".into());
                    self.net_return_online(&notice);
                }
            } else if local_side == Some(0) {
                self.stack.push(Screen::DramaticSetup(DramaticSetupScreen::from_rules(dramatic, &rules.config)));
            } else {
                self.net_wait_for_dramatic_host();
            }
            return;
        }
        let mut select = self.select_for(FightKind::Local);
        if phase == LobbyPhase::Stage {
            for side in 0..2 {
                if select.set_remote_pick(side, rules.fighters[side], true, rules.styles[side]).is_none() {
                    self.net_return_online("MATCH STYLE IS UNAVAILABLE");
                    return;
                }
            }
        }
        self.stack.push(Screen::Select(select));
        if phase == LobbyPhase::Stage {
            self.net_open_stage(rules.fighters[0], rules.fighters[1]);
        } else {
            self.net_capture_pick(false);
        }
    }

    pub(super) fn net_dramatic_editing(&self) -> bool {
        self.net_connection.as_ref().is_some_and(|connection| {
            connection.is_host() && connection.rules().room_kind() == RoomKind::Dramatic
        }) && self.net_setup.as_ref().is_some_and(|setup| !setup.flow.decision_sent)
            && matches!(self.stack.last(), Some(Screen::DramaticSetup(_) | Screen::DramaticPick(_) | Screen::Stage(_)))
    }

    pub(super) fn net_dramatic_waiting(&self) -> bool {
        self.net_connection.as_ref().is_some_and(|connection| {
            connection.rules().room_kind() == RoomKind::Dramatic
        }) && self.net_setup.as_ref().is_some_and(|setup| {
            setup.flow.dramatic_offer.is_some() || matches!(self.stack.last(), Some(Screen::Online(_)))
        }) && self.play.is_none()
    }

    fn net_wait_for_dramatic_host(&mut self) {
        if let Some(Screen::Online(screen)) = self.stack.last_mut() {
            screen.set_notice("WAITING FOR HOST TO SET UP CO-OP");
        }
    }

    pub(super) fn net_dramatic_offer(&mut self, config: MatchConfig) -> bool {
        let (Some(connection), Some(dramatic), Some(setup)) = (
            self.net_connection.as_ref(), config.dramatic, self.net_setup.as_mut(),
        ) else { return false };
        let previous = connection.rules();
        let rules = SyncedMatchRules::new(
            config, [config.roster.p1, config.roster.p2], previous.dips,
            config.roster.stage.unwrap_or(previous.stage), previous.seed,
            previous.bank_hash, previous.art_pack_hash,
        );
        let flow = &mut setup.flow;
        let Some(seq) = flow.seq.checked_add(1) else { return false };
        flow.seq = seq;
        flow.dramatic_offer = Some((seq, rules));
        let packet = NetPacket::DramaticSetup { generation: flow.generation, seq, rules };
        self.net_send_dramatic_lobby(packet);
        self.stack.truncate(self.stack.iter().rposition(|screen| matches!(screen, Screen::DramaticSetup(_)))
            .map_or(self.stack.len(), |index| index + 1));
        self.notice = Some(if dramatic.rivalry.is_some() {
            "WAITING FOR ALLY 2 TO READY"
        } else {
            "WAITING FOR ALLY 2 TO PICK"
        }.into());
        true
    }

    fn net_send_dramatic_lobby(&mut self, packet: NetPacket) {
        let flow = &mut self.net_setup.as_mut().expect("online setup").flow;
        flow.local_picks[0] = Some(packet.clone());
        flow.last_lobby_sent = self.frames;
        if let Some(connection) = self.net_connection.as_mut() {
            connection.send_to_player(packet);
        }
    }

    fn net_dramatic_input(&mut self, event: MenuInput) -> Option<Transition> {
        if event == MenuInput::Back && !matches!(self.stack.last(), Some(Screen::DramaticGuest(_))) {
            if self.net_connection.as_ref().is_some_and(Connection::is_host) {
                self.net_advance_epoch(LobbyPhase::Fighters);
            } else {
                self.net_return_online("LEFT ROOM");
            }
            return Some(Transition::Back);
        }
        let setup = self.net_setup.as_mut()?;
        let (setup_seq, _) = setup.flow.dramatic_offer?;
        let Some(Screen::DramaticGuest(screen)) = self.stack.last_mut() else { return None };
        let transition = screen.apply(event)?;
        if transition == Transition::Back {
            self.net_return_online("LEFT ROOM");
            return Some(transition);
        }
        if screen.locked() {
            let packet = NetPacket::DramaticAllyPick {
                generation: setup.flow.generation, setup_seq,
                fighter: screen.fighter(), revision: screen.revision(),
            };
            self.net_send_dramatic_lobby(packet);
            self.sounds.push_back(sound::CONFIRM);
        } else if matches!(transition, Transition::CursorMoved | Transition::StyleOpened | Transition::Unpicked) {
            self.sounds.push_back(sound::CURSOR);
        }
        Some(transition)
    }

    pub(super) fn net_open_stage(&mut self, p1: TournamentFighter, p2: TournamentFighter) {
        match self.stage_screen(FightKind::Local, p1, p2, self.chosen) {
            Some(screen) => self.stack.push(Screen::Stage(screen)),
            None => self.start_fight(FightKind::Local, p1, p2, Some(HIDDEN_FLAG)),
        }
        self.net_capture_pick(false);
    }

    fn net_capture_pick(&mut self, stage_locked: bool) {
        let Some(setup) = self.net_setup.as_ref() else {
            return;
        };
        let Some(side) = setup.local_side else { return };
        let stage = self
            .net_connection
            .as_ref()
            .map(|connection| connection.rules().stage)
            .unwrap_or(StagePick::Bank(0));
        let (fighter, stage, locked, phase, random, style) = match self.stack.last() {
            Some(Screen::Select(screen)) => (
                screen.fighter(side),
                stage,
                screen.locked(side).is_some(),
                LobbyPhase::Fighters,
                false,
                screen.locked_style(side),
            ),
            Some(Screen::Stage(screen)) => {
                let (_, p1, p2) = screen.fight();
                let selected = screen
                    .cursor()
                    .map(|stage| StagePick::Bank(stage.index))
                    .unwrap_or(StagePick::Bank(screen.stages()[0].index));
                (
                    if side == 0 { p1 } else { p2 },
                    selected,
                    stage_locked,
                    LobbyPhase::Stage,
                    screen.on_random(),
                    None,
                )
            }
            _ => return,
        };
        let flow = &mut self.net_setup.as_mut().expect("online setup").flow;
        let Some(seq) = flow.seq.checked_add(1) else {
            return;
        };
        flow.seq = seq;
        let packet = NetPacket::LobbyPick {
            generation: flow.generation,
            seq,
            side: side as u8,
            fighter,
            stage,
            locked,
            phase,
            random,
            style,
        };
        flow.local_picks[phase_index(phase)] = Some(packet.clone());
        flow.last_lobby_sent = self.frames;
        if let Some(connection) = self.net_connection.as_mut() {
            connection.send_to_player(packet);
        }
    }

    pub(super) fn net_screen_input(
        &mut self,
        prev: &FrameInputs,
        now: &FrameInputs,
    ) -> Option<Transition> {
        let setup = self.net_setup.as_ref()?;
        if setup.flow.decision_sent {
            return None;
        }
        let before = local_inputs(*prev);
        let input = local_inputs(*now);
        if self.net_dramatic_waiting() {
            return menu_events(&before, &input).into_iter().find_map(|event| self.net_dramatic_input(event));
        }
        let side = setup.local_side?;
        let mut transition = None;
        for event in menu_events(&before, &input) {
            let applied = match self.stack.last_mut() {
                Some(Screen::Select(screen)) => screen.apply_side(side, event),
                Some(Screen::Stage(screen)) => screen.apply_side(side, event),
                _ => None,
            };
            let Some(applied) = applied else { continue };
            if let Some(Screen::Select(screen)) = self.stack.last() {
                self.preferred_eras[side] = screen.preferred_eras()[side];
            }
            if applied == Transition::Back && matches!(self.stack.last(), Some(Screen::Stage(_))) {
                self.net_stage_back();
                return Some(Transition::Back);
            }
            self.net_capture_pick(matches!(
                applied,
                Transition::Stage(_) | Transition::RandomStage
            ));
            match applied {
                Transition::Back => {
                    self.net_return_online("LEFT ROOM");
                }
                _ => self.net_lobby_transition(applied),
            }
            match applied {
                Transition::CursorMoved => self.sounds.push_back(sound::CURSOR),
                Transition::StyleOpened
                | Transition::Picked(_)
                | Transition::Fighters(_, _)
                | Transition::Stage(_)
                | Transition::RandomStage => self.sounds.push_back(sound::CONFIRM),
                _ => {}
            }
            transition = Some(applied);
        }
        transition
    }

    pub(super) fn net_start_stage(&mut self, pick: Option<StagePick>) {
        let Some(Screen::Stage(screen)) = self.stack.pop() else {
            return;
        };
        let (kind, p1, p2) = screen.fight();
        let seed = self
            .net_connection
            .as_ref()
            .map_or(0, |connection| connection.rules().seed);
        self.chosen = Some(
            pick.unwrap_or_else(|| random_pick(screen.stages(), (seed ^ (seed >> 32)) as u32)),
        );
        self.start_fight(kind, p1, p2, Some(HIDDEN_FLAG));
    }

    pub(super) fn net_packet(&mut self, from: u8, packet: NetPacket) {
        let Some(connection) = self.net_connection.as_ref() else {
            return;
        };
        if connection.player_peer() != Some(from) {
            return;
        }
        let hosting = connection.is_host();
        let Some(setup) = self.net_setup.as_ref() else {
            return;
        };
        let Some(local_side) = setup.local_side else {
            return;
        };
        match packet {
            NetPacket::DramaticSetup { generation, seq, rules }
                if !hosting && generation == setup.flow.generation =>
            {
                if setup.flow.dramatic_offer.is_some_and(|(old, _)| seq <= old) {
                    return;
                }
                let guest = DramaticGuest::open(&rules.config, self.player_tables());
                self.stack.retain(|screen| !matches!(screen, Screen::DramaticGuest(_)));
                self.stack.push(Screen::DramaticGuest(Box::new(guest)));
                let flow = &mut self.net_setup.as_mut().expect("online setup").flow;
                flow.dramatic_offer = Some((seq, rules));
                flow.local_picks[0] = None;
            }
            NetPacket::DramaticAllyPick { generation, setup_seq, fighter, revision }
                if hosting && generation == setup.flow.generation =>
            {
                let Some((seq, mut rules)) = setup.flow.dramatic_offer else { return };
                let Some(dramatic) = rules.config.dramatic else { return };
                if seq != setup_seq
                    || !DramaticRules::style_available(&rules.config, fighter, revision)
                    || (dramatic.rivalry.is_some() && (dramatic.ally2 != fighter || dramatic.styles[2] != revision))
                    || !SelectScreen::fighter_available(&rules.config, self.player_tables(), 1, fighter)
                {
                    return;
                }
                let dramatic = rules.config.dramatic.as_mut().expect("Dramatic rules");
                dramatic.ally2 = fighter;
                dramatic.styles[2] = revision;
                self.net_commit_epoch(rules, LobbyPhase::Stage);
            }
            packet @ NetPacket::LobbyPick {
                generation,
                seq,
                side,
                locked,
                phase,
                ..
            } if generation == setup.flow.generation && usize::from(side) == 1 - local_side => {
                let stage_back = hosting
                    && phase == LobbyPhase::Fighters
                    && !locked
                    && matches!(self.stack.last(), Some(Screen::Stage(screen)) if screen.picker() == usize::from(side));
                let flow = &mut self.net_setup.as_mut().expect("online setup").flow;
                let index = phase_index(phase);
                if flow.remote_seq[index].is_some_and(|old| seq <= old) {
                    return;
                }
                if stage_back && flow.remote_seq[1].is_none_or(|old| seq > old) {
                    flow.remote_seq[index] = Some(seq);
                    self.net_advance_epoch(LobbyPhase::Fighters);
                    return;
                }
                let applied = self.net_apply_lobby_pick(&packet);
                // A packet for a later screen remains retryable until that screen exists.
                if let Some(applied) = applied {
                    self.net_setup
                        .as_mut()
                        .expect("online setup")
                        .flow
                        .remote_seq[index] = Some(seq);
                    self.net_lobby_transition(applied);
                }
            }
            packet @ NetPacket::Input { .. } => {
                if let Some(flow) = self.net_setup.as_mut().map(|setup| &mut setup.flow) {
                    if flow.pending_inputs.len() < 64 {
                        flow.pending_inputs.push(packet);
                    }
                }
                self.net_apply_inputs();
            }
            NetPacket::AfterMatchVote {
                generation,
                seq,
                choice,
            } if generation == setup.flow.generation
                && self.play.as_ref().is_some_and(|play| play.after.is_some()) =>
            {
                let flow = &mut self.net_setup.as_mut().expect("online setup").flow;
                if flow.remote_vote_seq.is_some_and(|old| seq <= old) {
                    return;
                }
                flow.remote_vote_seq = Some(seq);
                flow.votes[1 - local_side] = Some(choice);
                self.net_update_votes();
                if choice == AfterMatchChoice::Leave {
                    self.net_return_online("PEER LEFT ROOM");
                } else {
                    self.net_vote_decision();
                }
            }
            _ => {}
        }
    }

    /// Apply a replicated pick using the same eligibility and phase rules for
    /// combatants and spectators. Callers retain authorization and sequencing.
    pub(super) fn net_apply_lobby_pick(&mut self, packet: &NetPacket) -> Option<Transition> {
        let NetPacket::LobbyPick {
            phase,
            side,
            fighter,
            stage,
            locked,
            random,
            style,
            ..
        } = packet else {
            return None;
        };
        match (phase, self.stack.last_mut()) {
            (LobbyPhase::Fighters, Some(Screen::Select(screen))) => {
                screen.set_remote_pick(usize::from(*side), *fighter, *locked, *style)
            }
            (LobbyPhase::Stage, Some(Screen::Stage(screen)))
                if screen.picker() == usize::from(*side) =>
            {
                screen.set_remote_pick(*stage, *random, *locked)
            }
            _ => None,
        }
    }

    pub(super) fn net_lobby_transition(&mut self, transition: Transition) {
        match transition {
            Transition::Fighters(p1, p2) => self.net_open_stage(p1, p2),
            Transition::Stage(pick) => self.net_start_stage(Some(pick)),
            Transition::RandomStage => self.net_start_stage(None),
            _ => {}
        }
    }

    pub(super) fn net_apply_inputs(&mut self) {
        let (Some(setup), Some(play)) = (self.net_setup.as_mut(), self.play.as_mut()) else {
            return;
        };
        let Some(rollback) = play.netplay.as_mut() else {
            return;
        };
        let assets = fight_assets(
            &self.assets,
            &self.bank_assets,
            play.fight.setup().config.preset,
        );
        let flow = &mut setup.flow;
        for packet in std::mem::take(&mut flow.pending_inputs) {
            let NetPacket::Input {
                current_frame,
                start_frame,
                count,
                bits,
                ack_frame,
                frame_advantage,
                checksum_frame,
                checksum,
            } = packet
            else {
                continue;
            };
            let (Some(current), Some(start)) = (
                current_frame.checked_sub(flow.frame_base),
                start_frame.checked_sub(flow.frame_base),
            ) else {
                continue;
            };
            let Some(end) = start.checked_add(u32::from(count)) else {
                continue;
            };
            if current > rollback.next_frame().saturating_add(12)
                || end > rollback.next_frame().saturating_add(13)
            {
                continue;
            }
            if let Some(tip) = end
                .checked_sub(1)
                .and_then(|frame| flow.frame_base.checked_add(frame))
            {
                self.net_epoch_cursor
                    .observe(flow.generation, tip.max(current_frame));
            }
            if let (Some(ack), Some(latest)) = (
                ack_frame.checked_sub(flow.frame_base),
                rollback.latest_local_frame(),
            ) {
                if ack_frame != u32::MAX && ack <= latest {
                    flow.peer_ack = Some(flow.peer_ack.map_or(ack, |old| old.max(ack)));
                }
            }
            rollback.set_remote_advantage(frame_advantage);
            rollback.apply_remote_inputs(
                &mut play.fight,
                assets,
                start,
                &bits[..usize::from(count)],
            );
            for frame in start..end {
                if flow.remote_ack.is_none_or(|ack| frame > ack) {
                    flow.received.insert(frame);
                }
            }
            while flow
                .received
                .remove(&flow.remote_ack.map_or(0, |frame| frame + 1))
            {
                flow.remote_ack = Some(flow.remote_ack.map_or(0, |frame| frame + 1));
            }
            if checksum_frame != u32::MAX {
                if let Some(frame) = checksum_frame.checked_sub(flow.frame_base) {
                    rollback.receive_checksum(frame, checksum);
                }
            }
        }
    }

    pub(super) fn net_send(&mut self) {
        let Some(setup) = self.net_setup.as_mut() else {
            return;
        };
        let Some(connection) = self.net_connection.as_mut() else {
            return;
        };
        if self.frames.saturating_sub(setup.flow.last_lobby_sent) >= 30 {
            for packet in setup.flow.local_picks.iter().flatten() {
                connection.send_to_player(packet.clone());
            }
            setup.flow.last_lobby_sent = self.frames;
        }
        if self.frames.saturating_sub(setup.flow.last_vote_sent) >= 30 {
            if let Some(packet) = &setup.flow.local_vote {
                connection.send_to_player(packet.clone());
            }
            setup.flow.last_vote_sent = self.frames;
        }
        if let Some(rollback) = self.play.as_ref().and_then(|play| play.netplay.as_ref()) {
            if let Some(latest) = rollback.latest_local_frame() {
                if let Some(tip) = setup
                    .flow
                    .frame_base
                    .checked_add(latest.max(rollback.next_frame()))
                {
                    self.net_epoch_cursor.observe(setup.flow.generation, tip);
                }
                let start = setup
                    .flow
                    .peer_ack
                    .map_or(0, |frame| frame.saturating_add(1))
                    .min(latest);
                let count = (latest - start + 1).min(9) as u8;
                let mut bits = [0; 9];
                for (offset, value) in bits[..usize::from(count)].iter_mut().enumerate() {
                    *value = rollback
                        .local_input(start + offset as u32)
                        .expect("sampled local input");
                }
                let base = setup.flow.frame_base;
                let wire = |frame: u32| base.checked_add(frame).filter(|frame| *frame != u32::MAX);
                if let (Some(current_frame), Some(start_frame)) =
                    (wire(rollback.next_frame()), wire(start))
                {
                    let (checksum_frame, checksum) = rollback
                        .latest_confirmed_checksum()
                        .and_then(|(frame, checksum)| Some((wire(frame)?, checksum)))
                        .unwrap_or((u32::MAX, 0));
                    connection.send_to_player(NetPacket::Input {
                        current_frame,
                        start_frame,
                        count,
                        bits,
                        ack_frame: setup.flow.remote_ack.and_then(wire).unwrap_or(u32::MAX),
                        frame_advantage: rollback.frame_advantage(),
                        checksum_frame,
                        checksum,
                    });
                }
            }
        }
        self.net_commands.extend(connection.drain_commands());
    }
}

fn phase_index(phase: LobbyPhase) -> usize {
    match phase {
        LobbyPhase::Fighters => 0,
        LobbyPhase::Stage => 1,
    }
}

pub(super) fn local_inputs(mut input: FrameInputs) -> FrameInputs {
    input.p2 = PlayerInputs::default();
    input.pads[1] = None;
    input.pad_lost[1] = false;
    input.cabinet[3] = false;
    input
}
