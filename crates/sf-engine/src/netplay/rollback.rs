//! Real-fight rollback state machine (NP2), without sockets or wall clocks.
//!
//! `next_frame()` has not run yet. Ring entries hold pre-step snapshots;
//! audio delivery sets remain outside them across repeated corrections.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::{decode_inputs, encode_inputs};
use crate::front::fight::{Fight, FightEvent, FightFrame, FightLatch, FightSnapshot};
use crate::front::session::PlayerInputs;
use sf_video::announcer::AnnouncerLine;
use sf_video::romset::Assets;

pub const MAX_ROLLBACK_FRAMES: u8 = 8;
const RING: usize = MAX_ROLLBACK_FRAMES as usize + 1;
const INPUT_AHEAD: u32 = 16;
const DISCONNECT_TICKS: u16 = 300;
const IDLE: u8 = 0xff;
const GAMEPLAY: u16 = 0x03ff;
const FORFEIT: u16 = 0x8000;
const MAX_PENDING_AUDIO: usize = RING * 256;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RollbackEvent {
    WaitingForPeer,
    PeerDisconnected,
    PeerForfeit { frame: u32 },
    Desync { frame: u32, local: u64, remote: u64 },
}

#[derive(Clone)]
struct Slot {
    frame: u32,
    before: FightSnapshot,
    remote_used: u16,
    checksum: u64,
    event: Option<FightEvent>,
    heard_latches: [bool; 256],
    heard_announcements: Vec<AnnouncerLine>,
    heard_impacts: Vec<(crate::impact::ImpactKind, crate::impact::ImpactFighter)>,
}

/// Rollback bookkeeping borrowing the session's existing Fight on each call.
#[derive(Clone)]
pub struct RollbackEngine {
    local_side: usize,
    delay: u8,
    max_rollback: u8,
    next: u32,
    confirmed: i64,
    local: Vec<u16>,
    remote: Vec<Option<u16>>,
    // The snapshots are large. Keep their ring off the session stack.
    slots: Box<[Option<Slot>]>,
    confirmed_history: Vec<(u16, u16)>,
    // A peer's confirmed checksum can arrive after its pre-step snapshot
    // was reused. This grows only with locally confirmed play, like inputs.
    confirmed_hashes: Vec<u64>,
    confirmed_checksums: Option<(u32, u64)>,
    confirmed_fight_events: Vec<(u32, FightEvent)>,
    pending_checksums: BTreeMap<u32, u64>,
    reported_desyncs: BTreeSet<u32>,
    pending_latches: VecDeque<FightLatch>,
    // A corrected input can move the same round's theme to another frame.
    heard_themes: BTreeSet<(u32, crate::theme::ThemeId)>,
    pending_announcements: Vec<AnnouncerLine>,
    pending_impacts: BTreeMap<u32, Vec<crate::impact::ImpactCue>>,
    pending_fight_events: VecDeque<FightEvent>,
    events: VecDeque<RollbackEvent>,
    remote_advantage: Option<i8>,
    paced_last_tick: bool,
    waiting: bool,
    wait_ticks: u16,
    disconnected: bool,
    match_over: bool,
    rollback_frames: u8,
    corrected_picture: bool,
}

impl RollbackEngine {
    pub fn new(local_side: usize, delay: u8, max_rollback: u8) -> Self {
        assert!(local_side < 2, "a netplay side is P1 or P2");
        let delay = delay.min(4);
        Self {
            local_side,
            delay,
            max_rollback: max_rollback.clamp(1, MAX_ROLLBACK_FRAMES),
            next: 0,
            confirmed: -1,
            local: vec![0; usize::from(delay)],
            remote: vec![Some(0); usize::from(delay)],
            slots: std::iter::repeat_with(|| None).take(RING).collect(),
            confirmed_history: Vec::new(),
            confirmed_hashes: Vec::new(),
            confirmed_checksums: None,
            confirmed_fight_events: Vec::new(),
            pending_checksums: BTreeMap::new(),
            reported_desyncs: BTreeSet::new(),
            pending_latches: VecDeque::new(),
            heard_themes: BTreeSet::new(),
            pending_announcements: Vec::new(),
            pending_impacts: BTreeMap::new(),
            pending_fight_events: VecDeque::new(),
            events: VecDeque::new(),
            remote_advantage: None,
            paced_last_tick: false,
            waiting: false,
            wait_ticks: 0,
            disconnected: false,
            match_over: false,
            rollback_frames: 0,
            corrected_picture: false,
        }
    }

    pub fn next_frame(&self) -> u32 {
        self.next
    }
    pub fn confirmed_frame(&self) -> i64 {
        self.confirmed
    }
    pub fn local_side(&self) -> usize {
        self.local_side
    }
    pub fn input_delay(&self) -> u8 {
        self.delay
    }
    pub fn max_rollback(&self) -> u8 {
        self.max_rollback
    }
    pub fn last_rollback_frames(&self) -> u8 {
        self.rollback_frames
    }
    /// A correction invalidated the last presented picture, even if the
    /// next tick waits for remote input and does not advance the fight.
    pub(crate) fn take_corrected_picture(&mut self) -> bool {
        std::mem::take(&mut self.corrected_picture)
    }
    pub fn is_waiting(&self) -> bool {
        self.waiting
    }
    pub fn is_disconnected(&self) -> bool {
        self.disconnected
    }
    pub fn confirmed_match_over(&self) -> bool {
        self.match_over
    }
    pub fn latest_confirmed_checksum(&self) -> Option<(u32, u64)> {
        self.confirmed_checksums
    }
    /// The authoritative checksum after the given confirmed input frame.
    pub fn confirmed_checksum(&self, frame: u32) -> Option<u64> {
        self.confirmed_hashes.get(frame as usize).copied()
    }
    pub fn confirmed_inputs(&self) -> &[(u16, u16)] {
        &self.confirmed_history
    }
    pub fn confirmed_events(&self) -> &[(u32, FightEvent)] {
        &self.confirmed_fight_events
    }
    pub fn local_input(&self, frame: u32) -> Option<u16> {
        self.local.get(frame as usize).copied()
    }
    pub fn latest_local_frame(&self) -> Option<u32> {
        self.local
            .len()
            .checked_sub(1)
            .and_then(|f| u32::try_from(f).ok())
    }
    pub fn frame_advantage(&self) -> i8 {
        let received = self
            .remote
            .iter()
            .rposition(Option::is_some)
            .map_or(-1, |f| f as i64);
        (i64::from(self.next) - 1 - received).clamp(-127, 127) as i8
    }
    pub fn set_remote_advantage(&mut self, advantage: i8) {
        self.remote_advantage = Some(advantage);
    }
    pub fn drain_events(&mut self) -> Vec<RollbackEvent> {
        self.events.drain(..).collect()
    }

    /// Clone confirmed state only when requested, not on the advance path.
    pub fn confirmed_snapshot(&self, fight: &Fight) -> Option<FightSnapshot> {
        let next = u32::try_from(self.confirmed).ok()?.checked_add(1)?;
        if next == self.next {
            Some(fight.save_netplay_snapshot(next))
        } else {
            self.slot(next).map(|slot| slot.before.clone())
        }
    }

    /// A stalled display tick does not overwrite an input already sent.
    pub fn step(
        &mut self,
        fight: &mut Fight,
        assets: &Assets,
        local: PlayerInputs,
    ) -> Option<FightFrame> {
        if self.disconnected {
            return None;
        }
        let scheduled = self.next.checked_add(u32::from(self.delay))? as usize;
        if self.local.len() <= scheduled {
            self.local.resize(scheduled + 1, 0);
            self.local[scheduled] = encode_inputs(local);
        }
        if i64::from(self.next) - self.confirmed - 1 >= i64::from(self.max_rollback) {
            if !self.waiting {
                self.push_event(RollbackEvent::WaitingForPeer);
            }
            self.waiting = true;
            self.wait_ticks = self.wait_ticks.saturating_add(1);
            if self.wait_ticks >= DISCONNECT_TICKS {
                self.disconnected = true;
                self.push_event(RollbackEvent::PeerDisconnected);
            }
            return None;
        }
        self.waiting = false;
        self.wait_ticks = 0;
        let ahead = self
            .remote_advantage
            .is_some_and(|remote| i16::from(self.frame_advantage()) - i16::from(remote) >= 2);
        if ahead && !self.paced_last_tick {
            self.paced_last_tick = true;
            return None;
        }
        self.paced_last_tick = false;
        let mut output = self.simulate(self.next, fight, assets);
        self.next = self.next.checked_add(1)?;
        self.advance_confirmed();
        if let Some(latch) = self.pending_latches.pop_front() {
            output.put_latch(latch);
        }
        output.announcements.append(&mut self.pending_announcements);
        for (frame, mut cues) in std::mem::take(&mut self.pending_impacts) {
            let slot = self.slots[frame as usize % RING].as_mut()
                .filter(|slot| slot.frame == frame)
                .expect("pending contacts remain inside the rollback ring");
            slot.heard_impacts.extend(cues.iter().map(|cue| (cue.kind, cue.fighter)));
            output.impacts.append(&mut cues);
        }
        output.event = self.pending_fight_events.pop_front();
        Some(output)
    }

    /// Reject malformed or far-future windows before growing any storage.
    pub fn apply_remote_inputs(
        &mut self,
        fight: &mut Fight,
        assets: &Assets,
        start_frame: u32,
        bits: &[u16],
    ) {
        if self.disconnected || bits.is_empty() || bits.len() > RING {
            return;
        }
        let Some(end) = start_frame.checked_add(bits.len() as u32) else {
            return;
        };
        if end > self.next.saturating_add(INPUT_AHEAD)
            || bits.iter().any(|b| b & !(GAMEPLAY | FORFEIT) != 0)
        {
            return;
        }
        // Session authenticates the active match before this boundary. A
        // quit is transport control even in bootstrap or an old input slot.
        if let Some(offset) = bits.iter().position(|b| b & FORFEIT != 0) {
            self.disconnected = true;
            self.push_event(RollbackEvent::PeerForfeit {
                frame: start_frame + offset as u32,
            });
            return;
        }
        if i64::from(end) <= self.confirmed + 1 {
            return;
        }
        let mut dirty = None;
        for (offset, &value) in bits.iter().enumerate() {
            let frame = start_frame + offset as u32;
            if frame < u32::from(self.delay) && value != 0 {
                return;
            }
            if i64::from(frame) <= self.confirmed {
                if self.remote.get(frame as usize).copied().flatten() != Some(value) {
                    return;
                }
            } else if frame < self.next {
                let Some(slot) = self.slot(frame) else { return };
                if slot.remote_used != value {
                    dirty = Some(dirty.map_or(frame, |f: u32| f.min(frame)));
                }
            }
        }
        if self.remote.len() < end as usize {
            self.remote.resize(end as usize, None);
        }
        for (offset, &value) in bits.iter().enumerate() {
            self.remote[start_frame as usize + offset] = Some(value);
        }
        if let Some(frame) = dirty {
            let snapshot = self
                .slot(frame)
                .expect("validated rollback frame")
                .before
                .clone();
            fight.restore_netplay_snapshot(&snapshot);
            self.rollback_frames = (self.next - frame) as u8;
            self.corrected_picture = true;
            for replay in frame..self.next {
                self.simulate(replay, fight, assets);
            }
        }
        self.advance_confirmed();
    }

    pub fn receive_checksum(&mut self, frame: u32, remote: u64) {
        if self.disconnected || frame > self.next.saturating_add(INPUT_AHEAD) {
            return;
        }
        if i64::from(frame) <= self.confirmed {
            if let Some(local) = self.confirmed_checksum(frame) {
                self.compare_checksum(frame, local, remote);
            }
        } else {
            self.pending_checksums.insert(frame, remote);
        }
    }

    fn slot(&self, frame: u32) -> Option<&Slot> {
        self.slots[frame as usize % RING]
            .as_ref()
            .filter(|s| s.frame == frame)
    }
    fn remote_at(&self, frame: u32) -> u16 {
        self.remote
            .get(frame as usize)
            .copied()
            .flatten()
            .unwrap_or_else(|| {
                frame.checked_sub(1).map_or(0, |previous| {
                    self.remote
                        .get(previous as usize)
                        .copied()
                        .flatten()
                        .or_else(|| self.slot(previous).map(|s| s.remote_used))
                        .unwrap_or(0)
                })
            })
    }

    fn simulate(&mut self, frame: u32, fight: &mut Fight, assets: &Assets) -> FightFrame {
        let remote = self.remote_at(frame);
        let (p1, p2) = self.player_order(self.local[frame as usize], remote);
        let index = frame as usize % RING;
        let previous = self.slots[index].take().filter(|s| s.frame == frame);
        let (heard_latches, heard_announcements, heard_impacts) = previous
            .map_or(([false; 256], Vec::new(), Vec::new()), |s| {
                (s.heard_latches, s.heard_announcements, s.heard_impacts)
            });
        let before = fight.save_netplay_snapshot(frame);
        let mut output = fight.step(assets, decode_inputs(p1), decode_inputs(p2), false);
        let mut slot = Slot {
            frame,
            before,
            remote_used: remote,
            checksum: fight.state_checksum(),
            event: output.event.take(),
            heard_latches,
            heard_announcements,
            heard_impacts,
        };
        let new_latch = match output.theme.zip(output.theme_event) {
            Some((theme, event)) => self.heard_themes.insert((event, theme.id)),
            None if output.sound_latch != IDLE => {
                !std::mem::replace(&mut slot.heard_latches[usize::from(output.sound_latch)], true)
            }
            None => false,
        };
        let latch = output.take_latch();
        if new_latch {
            if self.pending_latches.len() < MAX_PENDING_AUDIO {
                self.pending_latches.push_back(latch);
            } else if !self.disconnected {
                self.disconnected = true;
                self.push_event(RollbackEvent::PeerDisconnected);
            }
        }
        for (index, &line) in output.announcements.iter().enumerate() {
            let occurrence = output.announcements[..=index]
                .iter()
                .filter(|&&seen| seen == line)
                .count();
            let heard = slot
                .heard_announcements
                .iter()
                .filter(|&&seen| seen == line)
                .count();
            if occurrence > heard {
                slot.heard_announcements.push(line);
                self.pending_announcements.push(line);
            }
        }
        output.announcements.clear();
        // A native frame emits at most one contact per affected fighter.
        // Replace undelivered predictions and remember only contacts that
        // have already reached presentation, even if their spark moves.
        output.impacts.retain(|cue| !slot.heard_impacts.contains(&(cue.kind, cue.fighter)));
        if output.impacts.is_empty() {
            self.pending_impacts.remove(&frame);
        } else {
            self.pending_impacts.insert(frame, std::mem::take(&mut output.impacts));
        }
        self.slots[index] = Some(slot);
        output
    }

    fn player_order(&self, local: u16, remote: u16) -> (u16, u16) {
        if self.local_side == 0 {
            (local, remote)
        } else {
            (remote, local)
        }
    }

    fn advance_confirmed(&mut self) {
        while self.confirmed + 1 < i64::from(self.next) {
            let frame = (self.confirmed + 1) as u32;
            let Some(remote) = self.remote.get(frame as usize).copied().flatten() else {
                break;
            };
            let (checksum, event) = {
                let slot = self
                    .slot(frame)
                    .expect("unconfirmed frames remain inside the ring");
                (slot.checksum, slot.event.clone())
            };
            self.confirmed_history
                .push(self.player_order(self.local[frame as usize], remote));
            self.confirmed_hashes.push(checksum);
            self.confirmed = i64::from(frame);
            self.confirmed_checksums = Some((frame, checksum));
            if let Some(event) = event {
                self.match_over |= event == FightEvent::MatchOver;
                self.confirmed_fight_events.push((frame, event.clone()));
                self.pending_fight_events.push_back(event);
            }
            if let Some(remote) = self.pending_checksums.remove(&frame) {
                self.compare_checksum(frame, checksum, remote);
            }
        }
        let oldest = self.next.saturating_sub(RING as u32);
        self.pending_checksums.retain(|frame, _| *frame >= oldest);
    }
    fn compare_checksum(&mut self, frame: u32, local: u64, remote: u64) {
        if local != remote && self.reported_desyncs.insert(frame) {
            self.push_event(RollbackEvent::Desync {
                frame,
                local,
                remote,
            });
        }
    }
    fn push_event(&mut self, event: RollbackEvent) {
        if self.events.len() == 32 {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MatchConfig;
    use crate::front::cabinet::Players;
    use crate::front::fight::{FightDips, FightKind, FightSetup, MatchTheme};
    use crate::theme::ThemeId;
    use sf_video::romset::load_assets;

    #[test]
    fn clean_heavy_impact_audio_is_deduplicated_on_resimulation() {
        let (assets, fight) = crate::impact::tests::close_local_fight();
        let idle = PlayerInputs::default();
        let heavy = PlayerInputs { buttons: [false, false, true, false, false, false], ..idle };
        let mut direct = fight.clone();
        direct.step(&assets, heavy, idle, false);
        let (before, expected) = (0..30).find_map(|_| {
            let before = direct.save_netplay_snapshot(0);
            let out = direct.step(&assets, idle, idle, false);
            out.impacts.first().copied().map(|cue| (before, cue))
        }).expect("a direct standing heavy punch connects");

        let mut predicted = fight;
        predicted.restore_netplay_snapshot(&before);
        direct.restore_netplay_snapshot(&before);
        let mut rollback = RollbackEngine::new(0, 0, 8);
        let out = rollback.step(&mut predicted, &assets, idle).unwrap();
        assert_eq!(out.impacts, [expected]);
        assert_eq!(predicted.impact_visuals().collect::<Vec<_>>(), [(expected, 0)]);

        // P2's late input forces the already-audible strike to be replayed.
        let corrected = PlayerInputs { buttons: [true, false, false, false, false, false], ..idle };
        let authoritative = direct.step(&assets, idle, corrected, false);
        assert_eq!(authoritative.impacts.len(), 1);
        assert_eq!(authoritative.impacts[0].frame, expected.frame);
        rollback.apply_remote_inputs(&mut predicted, &assets, 0, &[encode_inputs(corrected)]);
        assert_eq!(rollback.last_rollback_frames(), 1);
        assert_eq!(predicted.state_checksum(), direct.state_checksum());
        assert_eq!(predicted.impact_visuals().collect::<Vec<_>>(), [(authoritative.impacts[0], 0)]);
        let out = rollback.step(&mut predicted, &assets, idle).unwrap();
        assert!(out.impacts.is_empty(), "the contact's audio played already");
    }

    #[test]
    fn a_second_rollback_cancels_an_undelivered_heavy_impact() {
        let (assets, mut fight) = crate::impact::tests::close_local_fight();
        let idle = PlayerInputs::default();
        let heavy = PlayerInputs { buttons: [false, false, true, false, false, false], ..idle };
        let guard = PlayerInputs { directions: [true, false, false, false], ..idle };
        fight.step(&assets, heavy, idle, false);
        let mut before = Vec::new();
        let expected = (0..30).find_map(|_| {
            before.push(fight.save_netplay_snapshot(0));
            fight.step(&assets, idle, idle, false).impacts.into_iter().next()
        }).expect("a direct standing heavy punch connects");
        let contact = before.len() - 1;
        let lead = contact.min(4);
        assert!(lead >= 2, "room for a late guard before contact");
        fight.restore_netplay_snapshot(&before[contact - lead]);

        let mut rollback = RollbackEngine::new(0, 0, 8);
        // Leave frame zero unconfirmed so both inputs can correct frame one.
        rollback.step(&mut fight, &assets, idle).unwrap();
        rollback.apply_remote_inputs(&mut fight, &assets, 1, &[encode_inputs(guard)]);
        for _ in 0..lead {
            assert!(rollback.step(&mut fight, &assets, idle).unwrap().impacts.is_empty());
        }
        rollback.apply_remote_inputs(&mut fight, &assets, 1, &[encode_inputs(idle)]);
        assert_eq!(fight.impact_visuals().collect::<Vec<_>>(), [(expected, 0)]);
        rollback.apply_remote_inputs(&mut fight, &assets, 1, &[encode_inputs(guard)]);
        assert!(fight.impact_visuals().next().is_none());
        assert!(rollback.take_corrected_picture());
        assert!(!rollback.take_corrected_picture());

        let delivered = rollback.step(&mut fight, &assets, idle).unwrap();
        assert!(delivered.impacts.is_empty(), "the brief prediction was never presented");
    }

    #[test]
    fn a_theme_discovered_during_resimulation_keeps_its_original_latch_time() {
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
        fight.select_theme(Some(MatchTheme::single(ThemeId::Gouken)));
        let neutral = PlayerInputs::default();
        let (before, expected) = (0..1000).find_map(|_| {
            let before = fight.save_netplay_snapshot(0);
            let out = fight.step(&assets, neutral, neutral, false);
            out.theme.map(|cue| (before, (out.theme_frame, out.latch_cycles, cue)))
        }).expect("the direct fight emitted its theme");

        // Simulate a prediction that lacked the native queue entry. A remote
        // correction then starts from the snapshot containing the real latch.
        let mut missed = before.clone();
        missed.machine.toggle_state.queued_themes.fill(None);
        missed.machine.work[0x0662..0x0682].fill(0xff);
        fight.restore_netplay_snapshot(&missed);
        let mut rollback = RollbackEngine::new(0, 0, 8);
        let prediction = rollback.step(&mut fight, &assets, neutral).unwrap();
        assert!(prediction.theme.is_none());
        rollback.slots[0].as_mut().unwrap().before = before;
        let corrected = PlayerInputs { directions: [true, false, false, false], ..neutral };
        rollback.apply_remote_inputs(&mut fight, &assets, 0, &[encode_inputs(corrected)]);
        assert_eq!(rollback.last_rollback_frames(), 1);
        let delivered = rollback.step(&mut fight, &assets, neutral).unwrap();
        assert_eq!((delivered.theme_frame, delivered.latch_cycles, delivered.theme.unwrap()), expected);
        assert!(fight.machine().state.frame > delivered.theme_frame.unwrap());
    }
}
