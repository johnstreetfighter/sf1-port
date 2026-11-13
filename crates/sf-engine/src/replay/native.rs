//! Native match input files. The host owns file I/O.

use super::{ReplayMode, TakeoverPlayer, DEFAULT_TAKEOVER_WINDOW_FRAMES};
use crate::config::{MatchConfig, RulesetPreset};
use crate::era::StylePick;
use crate::front::cabinet::Players;
use crate::front::fight::{Fight, FightDips, FightFrame, FightKind, FightSetup, FightSnapshot};
use crate::front::PlayerInputs;
use crate::netplay::decode_inputs;
use crate::netplay::protocol::{read_replay_header, write_replay_header};
use crate::roster::StagePick;
use crate::utility::{Drill, RosterFighter, UtilityState};
use sf_video::romset::Assets;
use std::collections::BTreeMap;

pub const REPLAY_LIMIT: usize = 20;
pub const MAX_REPLAY_FRAMES: usize = 60 * 60 * 60 * 2;
pub(crate) const MAX_REPLAY_COMMANDS: usize = 12_000;
pub const MAX_REPLAY_BYTES: usize = MAX_REPLAY_FRAMES * 4 + MAX_REPLAY_COMMANDS * 6 + 1024;
const KEYFRAME: u32 = 60;
const RECENT_KEYFRAMES: u32 = 180 * KEYFRAME;
const ARCHIVE_KEYFRAME: u32 = 30 * KEYFRAME;
const MAX_TITLE: usize = 120;
const MAGIC: &[u8] = b"SF1R\x01";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchReplay {
    pub id: u64,
    pub title: String,
    pub kind: FightKind,
    pub online: bool,
    pub config: MatchConfig,
    pub dips: FightDips,
    pub stage: StagePick,
    pub flag: Option<u8>,
    pub seed: u64,
    pub data_hash: u64,
    pub p1_fighter: RosterFighter,
    pub p2_fighter: RosterFighter,
    pub p1_style: StylePick,
    pub p2_style: StylePick,
    pub frames: Vec<(u16, u16)>,
    /// Paused-menu edits applied immediately before the indexed frame.
    pub commands: Vec<(u32, ReplayCommand)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplayCommand {
    Restart,
    ResetPositions,
    Preset(RulesetPreset),
    FillMeters(bool),
    HoldCombo(bool),
    Drill(Option<Drill>),
    Trial(Option<u8>),
    Show(u8),
    Try(u8),
    StopTry,
    Alpha2Feint,
}

impl ReplayCommand {
    fn bytes(self) -> [u8; 2] {
        match self {
            Self::Restart => [0, 0],
            Self::ResetPositions => [1, 0],
            Self::Preset(preset) => [
                2,
                RulesetPreset::ALL
                    .iter()
                    .position(|&p| p == preset)
                    .unwrap_or(0) as u8,
            ],
            Self::FillMeters(on) => [3, u8::from(on)],
            Self::HoldCombo(on) => [4, u8::from(on)],
            Self::Drill(drill) => [
                5,
                drill.map_or(0, |d| {
                    Drill::ALL.iter().position(|&x| x == d).unwrap() as u8 + 1
                }),
            ],
            Self::Trial(trial) => [6, trial.unwrap_or(0)],
            Self::Show(entry) => [7, entry],
            Self::Try(entry) => [8, entry],
            Self::StopTry => [9, 0],
            Self::Alpha2Feint => [10, 0],
        }
    }

    fn read(kind: u8, value: u8) -> Option<Self> {
        Some(match kind {
            0 if value == 0 => Self::Restart,
            1 if value == 0 => Self::ResetPositions,
            2 => Self::Preset(*RulesetPreset::ALL.get(usize::from(value))?),
            3 if value <= 1 => Self::FillMeters(value != 0),
            4 if value <= 1 => Self::HoldCombo(value != 0),
            5 if value <= Drill::ALL.len() as u8 => {
                Self::Drill(value.checked_sub(1).map(|i| Drill::ALL[i as usize]))
            }
            6 if value <= 5 => Self::Trial((value != 0).then_some(value)),
            7 => Self::Show(value),
            8 => Self::Try(value),
            9 if value == 0 => Self::StopTry,
            10 if value == 0 => Self::Alpha2Feint,
            _ => return None,
        })
    }

    fn apply(self, fight: &mut Fight, input: PlayerInputs) {
        match self {
            Self::Restart => {
                fight.restart_round();
            }
            Self::ResetPositions => {
                fight.reset_positions();
            }
            Self::Preset(preset) => fight.set_preset(preset),
            Self::FillMeters(on) => fight.set_fill_meters(on),
            Self::HoldCombo(on) => fight.set_hold_combo(on),
            Self::Drill(drill) => fight.set_drill(drill),
            Self::Trial(trial) => fight.set_combo_trial(trial),
            Self::Show(entry) | Self::Try(entry) => {
                if let Some(entry) = crate::front::session::side_moves(fight, 0, Default::default())
                    .replay_entry(entry)
                {
                    if matches!(self, Self::Show(_)) {
                        fight.show(entry);
                    } else {
                        fight.try_move(entry);
                    }
                }
            }
            Self::StopTry => fight.stop_try(),
            Self::Alpha2Feint => {
                fight.try_alpha2_feint(input);
            }
        }
    }
}

impl MatchReplay {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut header = Vec::new();
        write_replay_header(
            &self.config,
            self.kind,
            [self.p1_fighter, self.p2_fighter],
            [self.p1_style, self.p2_style],
            &mut header,
        );
        let mut bytes = MAGIC.to_vec();
        bytes.extend_from_slice(&crate::version::replay_compatibility_hash().to_le_bytes());
        bytes.extend_from_slice(&self.id.to_le_bytes());
        bytes.extend_from_slice(&self.seed.to_le_bytes());
        let StagePick::Bank(stage) = self.stage;
        bytes.extend_from_slice(&[stage, self.flag.unwrap_or(0xff), self.dips.difficulty]);
        bytes.push(u8::from(self.online));
        bytes.extend_from_slice(&self.dips.round_time.to_le_bytes());
        bytes.extend_from_slice(&(self.title.len() as u16).to_le_bytes());
        bytes.extend_from_slice(&(header.len() as u16).to_le_bytes());
        bytes.extend_from_slice(&(self.frames.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&(self.commands.len() as u16).to_le_bytes());
        bytes.extend_from_slice(&self.data_hash.to_le_bytes());
        bytes.extend_from_slice(self.title.as_bytes());
        bytes.extend_from_slice(&header);
        for &(a, b) in &self.frames {
            bytes.extend_from_slice(&a.to_le_bytes());
            bytes.extend_from_slice(&b.to_le_bytes());
        }
        for &(index, command) in &self.commands {
            bytes.extend_from_slice(&index.to_le_bytes());
            bytes.extend_from_slice(&command.bytes());
        }
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let mut tail = bytes
            .strip_prefix(MAGIC)
            .ok_or("unrecognized replay format")?;
        let (header, rest) = tail.split_at_checked(44).ok_or("truncated replay header")?;
        tail = rest;
        let n16 = |at| u16::from_le_bytes([header[at], header[at + 1]]);
        let n32 = |at| u32::from_le_bytes(header[at..at + 4].try_into().unwrap());
        let n64 = |at| u64::from_le_bytes(header[at..at + 8].try_into().unwrap());
        if n32(0) != crate::version::replay_compatibility_hash() {
            return Err("incompatible replay engine".into());
        }
        let title_len = usize::from(n16(26));
        let header_len = usize::from(n16(28));
        let count = n32(30) as usize;
        let commands_count = usize::from(n16(34));
        if title_len > MAX_TITLE
            || header_len > 512
            || count > MAX_REPLAY_FRAMES
            || commands_count > MAX_REPLAY_COMMANDS
        {
            return Err("replay exceeds format limits".into());
        }
        if tail.len() != title_len + header_len + count * 4 + commands_count * 6 {
            return Err("replay length does not match its header".into());
        }
        let flag = match header[21] {
            0xff => None,
            value @ 0..=3 => Some(value),
            _ => return Err("invalid replay country".into()),
        };
        let online = match header[23] {
            0 => false,
            1 => true,
            _ => return Err("invalid replay mode".into()),
        };
        let dips = FightDips {
            difficulty: header[22],
            round_time: n16(24),
        };
        if !(1..=4).contains(&dips.difficulty) || ![100, 150, 200, 250].contains(&dips.round_time) {
            return Err("invalid replay cabinet settings".into());
        }
        let (title, tail) = tail.split_at(title_len);
        let title = std::str::from_utf8(title)
            .map_err(|_| "invalid replay title")?
            .to_owned();
        let (rules, tail) = tail.split_at(header_len);
        let (config, kind, [p1_fighter, p2_fighter], [p1_style, p2_style]) =
            read_replay_header(rules).ok_or("invalid replay rules")?;
        let (pads, actions) = tail.split_at(count * 4);
        let frames = pads
            .chunks_exact(4)
            .map(|b| {
                let p1 = u16::from_le_bytes([b[0], b[1]]);
                let p2 = u16::from_le_bytes([b[2], b[3]]);
                if p1 & !0x07ff != 0 || p2 & !0x03ff != 0 {
                    Err("invalid replay inputs")
                } else {
                    Ok((p1, p2))
                }
            })
            .collect::<Result<_, _>>()?;
        let mut commands = Vec::with_capacity(commands_count);
        for item in actions.chunks_exact(6) {
            let index = u32::from_le_bytes(item[..4].try_into().unwrap());
            if index as usize >= count || commands.last().is_some_and(|&(last, _)| last > index) {
                return Err("invalid replay command frame".into());
            }
            commands.push((
                index,
                ReplayCommand::read(item[4], item[5]).ok_or("invalid replay command")?,
            ));
        }
        Ok(Self {
            id: n64(4),
            title,
            kind,
            online,
            config,
            dips,
            stage: StagePick::Bank(header[20]),
            flag,
            seed: n64(12),
            data_hash: n64(36),
            p1_fighter,
            p2_fighter,
            p1_style,
            p2_style,
            frames,
            commands,
        })
    }

    pub fn setup(&self) -> FightSetup {
        let players = match self.kind {
            FightKind::Arcade | FightKind::Dramatic { .. } => Players::One,
            FightKind::TrainingCpu if !self.config.p2_fair_cpu => Players::One,
            _ => Players::Two,
        };
        FightSetup {
            kind: self.kind,
            players,
            config: self.config,
            dsw1: FightDips::DSW1,
            dsw2: self.dips.dsw2(),
        }
    }
}

#[derive(Clone)]
pub struct ReplayPlayback {
    replay: MatchReplay,
    fight: Fight,
    frame: u32,
    mode: ReplayMode,
    keyframes: BTreeMap<u32, FightSnapshot>,
    bookmark: Option<(u32, FightSnapshot)>,
    reference: Option<Fight>,
    branch_side: Option<TakeoverPlayer>,
    fallback: bool,
    utility: UtilityState,
}

impl ReplayPlayback {
    /// The supplied native fight uses the replay's original data banks and
    /// starts immediately after the hidden-drive frames.
    pub fn new(replay: MatchReplay, fight: Fight) -> Self {
        let keyframes = BTreeMap::from([(0, fight.save_netplay_snapshot(0))]);
        Self {
            replay,
            fight,
            frame: 0,
            mode: ReplayMode::Playback,
            keyframes,
            bookmark: None,
            reference: None,
            branch_side: None,
            fallback: false,
            utility: UtilityState::default(),
        }
    }

    pub fn fight(&self) -> &Fight {
        &self.fight
    }
    pub fn fight_mut(&mut self) -> &mut Fight {
        &mut self.fight
    }
    pub fn frame_index(&self) -> u32 {
        self.frame
    }
    pub fn len(&self) -> u32 {
        self.replay.frames.len() as u32
    }
    pub fn is_empty(&self) -> bool {
        self.replay.frames.is_empty()
    }
    pub fn mode(&self) -> ReplayMode {
        self.mode
    }
    pub fn has_bookmark(&self) -> bool {
        self.bookmark.is_some()
    }
    pub fn in_branch(&self) -> bool {
        self.branch_side.is_some()
    }

    pub fn pause(&mut self) {
        // Keep the remaining takeover time as well as the input owner.
        if !self.in_branch() {
            self.mode = ReplayMode::Paused;
        }
    }

    pub fn resume(&mut self) {
        if !self.in_branch() {
            self.mode = ReplayMode::Playback;
        }
    }

    pub fn scrub(&mut self, assets: &Assets, delta: i32) {
        if self.in_branch() {
            return;
        }
        let target = self.frame.saturating_add_signed(delta).min(self.len());
        if let Some((&index, snap)) = self.keyframes.range(..=target).next_back() {
            self.fight.restore_netplay_snapshot(snap);
            self.frame = index;
        }
        while self.frame < target {
            self.step_recorded(assets);
        }
    }

    pub fn can_take_over(&self, side: TakeoverPlayer) -> bool {
        let side = usize::from(side == TakeoverPlayer::P2);
        self.frame < self.len()
            && !self.in_branch()
            && self.fight.machine().utility_takeover_side_available(side)
    }

    pub fn takeover(&mut self, side: TakeoverPlayer) -> bool {
        if !self.can_take_over(side) {
            return false;
        }
        self.bookmark = Some((self.frame, self.fight.save_netplay_snapshot(self.frame)));
        self.reference = Some(self.fight.clone());
        self.fight.release_training_pads();
        self.branch_side = Some(side);
        self.fallback = false;
        self.utility = UtilityState::seeded(self.replay.seed ^ u64::from(self.frame));
        self.mode = ReplayMode::Takeover {
            player: side,
            start_frame: self.frame,
            window_frames: DEFAULT_TAKEOVER_WINDOW_FRAMES,
            elapsed_frames: 0,
        };
        true
    }

    pub fn rewind(&mut self) -> bool {
        let Some((index, saved)) = &self.bookmark else {
            return false;
        };
        self.fight.restore_netplay_snapshot(saved);
        self.frame = *index;
        self.reference = None;
        self.branch_side = None;
        self.fallback = false;
        self.utility = UtilityState::default();
        self.mode = ReplayMode::Paused;
        true
    }

    fn recorded(&self) -> ([PlayerInputs; 2], bool) {
        let (p1, p2) = self
            .replay
            .frames
            .get(self.frame as usize)
            .copied()
            .unwrap_or_default();
        ([decode_inputs(p1), decode_inputs(p2)], p1 & (1 << 10) != 0)
    }

    fn step_recorded(&mut self, assets: &Assets) -> Option<FightFrame> {
        if self.frame >= self.len() {
            return None;
        }
        let ([p1, p2], start) = self.recorded();
        for command in commands_at(&self.replay, self.frame) {
            command.apply(&mut self.fight, p1);
        }
        let result = self.fight.step(assets, p1, p2, start);
        self.frame += 1;
        if self.frame.is_multiple_of(KEYFRAME) {
            self.keyframes
                .entry(self.frame)
                .or_insert_with(|| self.fight.save_netplay_snapshot(self.frame));
            // Keep recent second-by-second scrubs fast and bound memory for
            // long Training sessions. Older regions keep periodic anchors.
            let oldest_recent = self.frame.saturating_sub(RECENT_KEYFRAMES);
            self.keyframes.retain(|&frame, _| {
                frame >= oldest_recent && frame <= self.frame + RECENT_KEYFRAMES
                    || frame.is_multiple_of(ARCHIVE_KEYFRAME)
            });
        }
        Some(result)
    }

    pub fn step(&mut self, assets: &Assets, live: PlayerInputs) -> Option<FightFrame> {
        match self.mode {
            ReplayMode::Paused => None,
            ReplayMode::Playback => {
                let result = self.step_recorded(assets);
                if self.frame >= self.len() {
                    self.pause();
                }
                result
            }
            ReplayMode::Takeover {
                player,
                start_frame,
                window_frames,
                elapsed_frames,
            } => {
                let controlled = usize::from(player == TakeoverPlayer::P2);
                let other = 1 - controlled;
                let (mut pads, start) = self.recorded();
                let mut bypass = [false; 2];
                let mut override_p2 = controlled == 1;
                if let Some(reference) = &mut self.reference {
                    for command in commands_at(&self.replay, self.frame) {
                        command.apply(reference, pads[0]);
                    }
                    let live_state = self.fight.replay_fighter_read(other);
                    let recorded_state = reference.replay_fighter_read(other);
                    let new_block = self.replay.config.dramatic.is_none()
                        && self.fight.fight_read().blocked[controlled]
                        && !reference.fight_read().blocked[controlled];
                    self.fallback |= self.frame >= self.replay.frames.len() as u32
                        || new_block
                        || (live_state != recorded_state
                            && (live_state.blockstun
                                || live_state.hitstun()
                                || live_state.health < recorded_state.health
                                || live_state.getting_up));
                    reference.step(assets, pads[0], pads[1], start);
                    if other == 1
                        && !self.fallback
                        && reference.replay_automated_p2()
                        && self.fight.machine().utility_takeover_side_available(1)
                    {
                        pads[1] = reference.machine().applied_p2_pad();
                        override_p2 = true;
                        bypass[1] = true;
                    }
                }
                pads[controlled] = live;
                if self.fallback && self.fight.machine().utility_takeover_side_available(other) {
                    pads[other] = self
                        .fight
                        .machine()
                        .utility_takeover_pad(other, &mut self.utility);
                    bypass[other] = true;
                    override_p2 |= other == 1;
                }
                let result =
                    self.fight
                        .step_with_pad_override(assets, pads, start, bypass, override_p2);
                self.frame += 1;
                let elapsed_frames = elapsed_frames + 1;
                self.mode = if elapsed_frames >= window_frames || result.event.is_some() {
                    ReplayMode::Paused
                } else {
                    ReplayMode::Takeover {
                        player,
                        start_frame,
                        window_frames,
                        elapsed_frames,
                    }
                };
                Some(result)
            }
        }
    }
}

fn commands_at(replay: &MatchReplay, frame: u32) -> impl Iterator<Item = ReplayCommand> + '_ {
    let first = replay.commands.partition_point(|&(index, _)| index < frame);
    replay.commands[first..]
        .iter()
        .take_while(move |&&(index, _)| index == frame)
        .map(|&(_, command)| command)
}
