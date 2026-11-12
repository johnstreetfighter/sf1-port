//! Real-fight rollback state machine (NP2), without sockets or wall clocks.
//!
//! `next_frame()` has not run yet. Ring entries hold pre-step snapshots;
//! audio delivery sets remain outside them across repeated corrections.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::{decode_inputs, encode_inputs};
use crate::front::fight::{Fight, FightEvent, FightFrame, FightSnapshot};
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
    pending_latches: VecDeque<(u8, u32)>,
    pending_announcements: Vec<AnnouncerLine>,
    pending_fight_events: VecDeque<FightEvent>,
    events: VecDeque<RollbackEvent>,
    remote_advantage: Option<i8>,
    paced_last_tick: bool,
    waiting: bool,
    wait_ticks: u16,
    disconnected: bool,
    match_over: bool,
    rollback_frames: u8,
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
            pending_announcements: Vec::new(),
            pending_fight_events: VecDeque::new(),
            events: VecDeque::new(),
            remote_advantage: None,
            paced_last_tick: false,
            waiting: false,
            wait_ticks: 0,
            disconnected: false,
            match_over: false,
            rollback_frames: 0,
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
        if let Some((latch, cycles)) = self.pending_latches.pop_front() {
            output.sound_latch = latch;
            output.latch_cycles = cycles;
        }
        output.announcements.append(&mut self.pending_announcements);
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
        let (heard_latches, heard_announcements) = previous
            .map_or(([false; 256], Vec::new()), |s| {
                (s.heard_latches, s.heard_announcements)
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
        };
        if output.sound_latch != IDLE && !slot.heard_latches[usize::from(output.sound_latch)] {
            slot.heard_latches[usize::from(output.sound_latch)] = true;
            if self.pending_latches.len() < MAX_PENDING_AUDIO {
                self.pending_latches
                    .push_back((output.sound_latch, output.latch_cycles));
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
        output.sound_latch = IDLE;
        output.latch_cycles = 0;
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
