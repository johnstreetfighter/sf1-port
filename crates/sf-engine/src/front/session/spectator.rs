//! Confirmed spectators share the lobby, without becoming a fighter route.

use super::*;
use std::collections::{BTreeMap, VecDeque};
use crate::front::fight::{FightFrame, FightLatch, MatchTheme};
use crate::netplay::decode_inputs;
use crate::netplay::protocol::{LobbyPhase, NetPacket};

#[derive(Clone, Default)]
pub(super) struct State {
    generation: Option<u32>,
    picks: [[Option<NetPacket>; 2]; 2],
    applied: [[Option<u32>; 2]; 2],
    dirty: bool,
    last_sent: u32,
    confirmed: Vec<(u16, u16)>,
    theme: Option<MatchTheme>,
    played: u32,
    ack_by_peer: BTreeMap<u8, u32>,
    latches: VecDeque<FightLatch>,
}

impl State {
    pub(super) fn reset(&mut self, generation: u32) {
        *self = Self { generation: Some(generation), ..Self::default() };
    }

    fn ensure_generation(&mut self, generation: u32) {
        if self.generation != Some(generation) { self.reset(generation); }
    }

    fn remember(&mut self, packet: &NetPacket) {
        let NetPacket::LobbyPick { generation, phase, side, seq, .. } = packet else { return };
        if Some(*generation) != self.generation || *side > 1 { return; }
        let slot = &mut self.picks[phase_index(*phase)][usize::from(*side)];
        if matches!(slot, Some(NetPacket::LobbyPick { seq: old, .. }) if *old >= *seq) { return; }
        *slot = Some(packet.clone());
        self.dirty = true;
    }

    fn receive_chunk(&mut self, start: u32, count: u8, pairs: &[(u16, u16); 16], theme: Option<MatchTheme>) {
        let start = start as usize;
        let count = usize::from(count);
        if !(1..=16).contains(&count) || start > self.confirmed.len() { return; }
        if self.confirmed.is_empty() {
            self.theme = theme;
        } else if self.theme != theme {
            return;
        }
        let overlap = (self.confirmed.len() - start).min(count);
        if self.confirmed[start..start + overlap] != pairs[..overlap] { return; }
        self.confirmed.extend_from_slice(&pairs[overlap..count]);
    }

    pub(super) fn played_inputs(&self) -> &[(u16, u16)] {
        &self.confirmed[..self.played as usize]
    }

    /// Playback consumes confirmed pairs only. The trailing fifteen remain a
    /// jitter buffer; a late join can replay at most eight pairs per display.
    /// An impact ends the batch on its contact picture so the sound and spark
    /// start together even while playback is catching up.
    pub(super) fn step(&mut self, fight: &mut Fight, assets: &Assets) -> FightFrame {
        let count = self.confirmed.len().saturating_sub(self.played as usize)
            .saturating_sub(15).min(8);
        if self.played == 0 && count > 0 {
            fight.select_theme(self.theme);
        }
        let mut last = None;
        let mut announcements = Vec::new();
        let mut impacts = Vec::new();
        let mut clear_impacts = false;
        let mut event = None;
        for _ in 0..count {
            let (p1, p2) = self.confirmed[self.played as usize];
            let mut frame = fight.step(assets, decode_inputs(p1), decode_inputs(p2), false);
            self.played += 1;
            let audible = frame.sound_latch != sound::IDLE;
            let latch = frame.take_latch();
            if audible && self.latches.len() < 256 {
                self.latches.push_back(latch);
            }
            announcements.append(&mut frame.announcements);
            clear_impacts |= frame.clear_impacts;
            impacts.append(&mut frame.impacts);
            if event.is_none() { event = frame.event.take(); }
            last = Some(frame);
            if !impacts.is_empty() {
                break;
            }
        }
        let mut frame = last.unwrap_or_else(|| FightFrame {
            frame: if fight.driving() {
                Frame { argb: vec![0xff00_0000; sf_video::render::WIDTH * sf_video::render::HEIGHT] }
            } else { fight.render(assets) },
            sound_latch: sound::IDLE, latch_cycles: 0, theme: None,
            theme_frame: None, theme_event: None,
            announcements: Vec::new(), impacts: Vec::new(), clear_impacts: false, event: None,
        });
        if let Some(latch) = self.latches.pop_front() {
            frame.put_latch(latch);
        }
        frame.announcements = announcements;
        frame.impacts = impacts;
        frame.clear_impacts = clear_impacts;
        frame.event = event;
        frame
    }
}

impl GameSession {
    /// The observer's first unsimulated confirmed-input frame, independent of
    /// the native machine counter. None outside an admitted spectator session.
    pub fn spectator_frame(&self) -> Option<u32> {
        let setup = self.net_setup.as_ref()?;
        let connection = self.net_connection.as_ref()?;
        (setup.local_side.is_none() && connection.kind() == RoomKind::Spectate)
            .then_some(self.net_spectators.played)
    }

    /// Returns true when the spectator branch has consumed this packet.
    pub(super) fn net_spectator_packet(&mut self, from: u8, packet: &NetPacket) -> bool {
        let (Some(connection), Some(setup)) = (self.net_connection.as_ref(), self.net_setup.as_ref()) else {
            return false;
        };
        self.net_spectators.ensure_generation(connection.generation());
        if connection.is_host() {
            if let NetPacket::SpectatorAck { generation, next_expected_frame } = packet {
                if *generation == connection.generation() && connection.spectator_peers().contains(&from) {
                    let available = self.play.as_ref().and_then(|play| play.netplay.as_ref())
                        .map_or(0, |rollback| rollback.confirmed_inputs().len());
                    if (*next_expected_frame as usize) <= available {
                        let ack = self.net_spectators.ack_by_peer.entry(from).or_insert(0);
                        *ack = (*ack).max(*next_expected_frame);
                    }
                }
                return true;
            }
            if connection.player_peer() != Some(from) { return false; }
            // Connection authenticates the challenger's role and generation.
            // The observer applies the same Select/Stage eligibility checks.
            self.net_spectators.remember(packet);
            return false;
        }
        if connection.player_peer() != Some(from) { return false; }
        if setup.local_side.is_some() || connection.kind() != RoomKind::Spectate { return false; }
        match packet {
            NetPacket::LobbyPick { .. } => {
                self.net_spectators.remember(packet);
                self.net_spectator_picks();
                true
            }
            NetPacket::SpectatorChunk { generation, start_frame, count, pairs, theme }
                if *generation == connection.generation() =>
            {
                self.net_spectators.receive_chunk(*start_frame, *count, pairs, *theme);
                true
            }
            _ => false,
        }
    }

    /// Retry complete lobby state independently for each observer. A late
    /// observer needs both fighters, including the remote challenger's pick.
    pub(super) fn net_spectator_tick(&mut self) {
        let Some(connection) = self.net_connection.as_ref() else { return };
        if self.net_setup.is_none() { return; }
        self.net_spectators.ensure_generation(connection.generation());
        if !connection.is_host() {
            if connection.kind() == RoomKind::Spectate {
                self.net_spectator_picks();
                if let Ok(next_expected_frame) = u32::try_from(self.net_spectators.confirmed.len()) {
                    let connection = self.net_connection.as_mut().expect("observer connection");
                    connection.send_to_player(NetPacket::SpectatorAck { generation: connection.generation(), next_expected_frame });
                    self.net_commands.extend(connection.drain_commands());
                }
            }
            return;
        }
        for command in &self.net_commands {
            if let NetTransportCommand::SendPacket { bytes, .. } = command {
                if let Some(packet @ NetPacket::LobbyPick { side: 0, .. }) = NetPacket::from_bytes(bytes) {
                    self.net_spectators.remember(&packet);
                }
            }
        }
        let connection = self.net_connection.as_mut().expect("connected room");
        let spectators = connection.spectator_peers();
        if !spectators.is_empty() && (self.net_spectators.dirty
            || self.frames.wrapping_sub(self.net_spectators.last_sent) >= 30)
        {
            for &peer in &spectators {
                for packet in self.net_spectators.picks.iter().flatten().flatten() {
                    connection.send_to(peer, packet.clone());
                }
            }
            self.net_spectators.dirty = false;
            self.net_spectators.last_sent = self.frames;
        }
        self.net_spectators.ack_by_peer.retain(|peer, _| spectators.contains(peer));
        if let Some((fight, rollback)) = self.play.as_ref()
            .and_then(|play| Some((&play.fight, play.netplay.as_ref()?)))
        {
            let theme = fight.selected_theme();
            let history = rollback.confirmed_inputs();
            for peer in spectators {
                let start = self.net_spectators.ack_by_peer.get(&peer).copied().unwrap_or(0) as usize;
                let count = history.len().saturating_sub(start).min(16);
                if count == 0 { continue; }
                let mut pairs = [(0, 0); 16];
                pairs[..count].copy_from_slice(&history[start..start + count]);
                connection.send_to(peer, NetPacket::SpectatorChunk { generation: connection.generation(),
                    start_frame: start as u32, count: count as u8, pairs, theme });
            }
        }
        self.net_commands.extend(connection.drain_commands());
    }

    fn net_spectator_picks(&mut self) {
        let packets: Vec<_> = self.net_spectators.picks.iter().flatten().flatten().cloned().collect();
        for packet in packets {
            let NetPacket::LobbyPick { phase, seq, side, .. } = packet else { continue };
            let index = phase_index(phase);
            if self.net_spectators.applied[index][usize::from(side)].is_some_and(|old| old >= seq) { continue; }
            if let Some(applied) = self.net_apply_lobby_pick(&packet) {
                self.net_spectators.applied[index][usize::from(side)] = Some(seq);
                self.net_lobby_transition(applied);
            }
        }
    }
}

fn phase_index(phase: LobbyPhase) -> usize {
    match phase { LobbyPhase::Fighters => 0, LobbyPhase::Stage => 1 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::front::fight::{FightDips, FightSetup};
    use crate::theme::ThemeId;
    use sf_video::romset::load_assets;

    #[test]
    fn spectator_catch_up_preserves_clean_heavy_impact_contact() {
        let (assets, mut direct) = crate::impact::tests::close_local_fight();
        let mut observer = direct.clone();
        let idle = PlayerInputs::default();
        let heavy = PlayerInputs { buttons: [false, false, true, false, false, false], ..idle };
        let pads = std::iter::once(heavy).chain(std::iter::repeat_n(idle, 95));
        let mut confirmed = Vec::new();
        let mut contact = None;
        for p1 in pads {
            let out = direct.step(&assets, p1, idle, false);
            confirmed.push((crate::netplay::encode_inputs(p1), 0));
            if !out.impacts.is_empty() {
                assert!(contact.is_none(), "only one clean standing heavy");
                contact = Some((out.frame, out.impacts));
            }
        }
        let (expected_frame, expected_cues) = contact.expect("the heavy connects");
        let mut spectator = State { confirmed, ..State::default() };
        let mut actual = Vec::new();
        for _ in 0..15 {
            let out = spectator.step(&mut observer, &assets);
            if let Some(cue) = out.impacts.first() {
                assert_eq!(observer.machine().state.frame, cue.frame);
                assert_eq!(out.frame.argb, expected_frame.argb);
            }
            actual.extend(out.impacts);
        }
        assert_eq!(actual, expected_cues, "the spectator presents the impact once on its contact frame");
    }

    #[test]
    fn late_spectator_delivers_the_original_latch_time_after_catching_up() {
        let roms = "../../tools/reference/local/roms/sf";
        if !std::path::Path::new(roms).is_dir() {
            eprintln!("skipping: local ROM set not present");
            return;
        }
        let assets = load_assets(roms, "../../tools/reference/rom-manifest.json").unwrap();
        let setup = FightSetup {
            kind: FightKind::Local,
            players: Players::Two,
            config: MatchConfig::definitive(),
            dsw1: FightDips::DSW1,
            dsw2: FightDips::default().dsw2(),
        };
        let mut fight = Fight::start_banked(&assets.maincpu, setup, Some(0), None, None).unwrap();
        let selection = MatchTheme { id: ThemeId::Gouken, phase: 1 };
        fight.select_theme(Some(selection));
        let mut observer = fight.clone();
        let input = PlayerInputs::default();
        let expected = (0..1000).find_map(|_| {
            let out = fight.step(&assets, input, input, false);
            out.theme.map(|cue| (out.theme_frame, out.latch_cycles, cue))
        }).expect("the direct fight emitted its theme");
        let mut spectator = State { confirmed: vec![(0, 0); 1200], theme: Some(selection), ..State::default() };
        let actual = (0..160).find_map(|_| {
            let out = spectator.step(&mut observer, &assets);
            out.theme.map(|cue| (out.theme_frame, out.latch_cycles, cue))
        }).expect("spectator catch-up delivered the original theme");
        assert_eq!(actual, expected);
        assert!(observer.machine().state.frame > actual.0.unwrap());
    }
}
