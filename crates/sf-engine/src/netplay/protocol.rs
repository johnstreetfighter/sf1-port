//! Versioned, socket-free netplay wire protocol and synchronized match rules.

use super::{Link, RoomKind};
use crate::config::{CoreToggles, MatchConfig, RulesetPreset, TourRules};
use crate::dramatic::{DramaticRules, LifebarMode};
use crate::era::{loadouts, GaugeMode, Loadout, Revision, StylePick};
use crate::front::fight::FightDips;
use crate::roster::{BanFormat, PlayMode, RosterSettings, StagePick, StartingCountry, TournamentFighter};
use crate::session_io::ControlScheme;
use crate::sf1plus::Sf1PlusRules;

pub const PROTOCOL_VERSION: u8 = 1;
pub const MAX_PACKET_BYTES: usize = 512;
pub const PLAYER_TAG_CHARS: &str = " ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_";

impl RoomKind {
    pub const fn is_combatant(self) -> bool {
        matches!(self, Self::Versus | Self::Dramatic)
    }
}

pub fn tag_from_str(text: &str) -> Option<[u8; 12]> {
    if text.is_empty() || text.len() > 12 || text.trim().is_empty()
        || !text.bytes().all(|b| PLAYER_TAG_CHARS.as_bytes().contains(&b))
    {
        return None;
    }
    let mut tag = [0; 12];
    tag[..text.len()].copy_from_slice(text.as_bytes());
    Some(tag)
}

pub fn tag_to_string(tag: [u8; 12]) -> String {
    let end = tag.iter().position(|b| *b == 0).unwrap_or(12);
    String::from_utf8_lossy(&tag[..end]).into_owned()
}

fn valid_tag(tag: [u8; 12]) -> bool {
    tag_from_str(&tag_to_string(tag)) == Some(tag)
}

/// Authoritative match configuration. Physical bindings, video,
/// audio and observer preferences have no fields here. Scheme selectors are
/// currently inert in translated fights and are normalized to Classic on wire;
/// TRD1 must synchronize/normalize their effects before activating them. The
/// constructor's explicit fighters/DIPs/resolved stage are authoritative over
/// their configuration copies; decoding rejects contradictory copies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyncedMatchRules {
    pub config: MatchConfig,
    pub fighters: [TournamentFighter; 2],
    pub styles: [Option<StylePick>; 2],
    pub dips: FightDips,
    pub stage: StagePick,
    pub seed: u64,
    pub bank_hash: u64,
    pub art_pack_hash: u64,
    pub protocol_hash: u32,
}

impl SyncedMatchRules {
    pub fn new(
        mut config: MatchConfig, fighters: [TournamentFighter; 2], dips: FightDips,
        stage: StagePick, seed: u64, bank_hash: u64, art_pack_hash: u64,
    ) -> Self {
        config.p1_scheme = ControlScheme::Classic;
        config.p2_scheme = ControlScheme::Classic;
        [config.roster.p1, config.roster.p2] = fighters;
        config.roster.stage = Some(stage);
        config.difficulty = dips.difficulty;
        config.round_time = dips.round_time;
        Self { config, fighters, styles: [None; 2], dips, stage, seed, bank_hash, art_pack_hash,
            protocol_hash: crate::version::protocol_version_hash() }
    }

    pub fn config_hash(&self) -> u64 {
        self.to_bytes().iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3)
        })
    }

    pub fn room_kind(&self) -> RoomKind {
        if self.config.dramatic.is_some() { RoomKind::Dramatic } else { RoomKind::Versus }
    }

    /// Fighters driven by the two peers. The base Dramatic roster holds Ally 1 and the boss.
    pub fn player_fighters(&self) -> [TournamentFighter; 2] {
        self.config.dramatic.map_or(self.fighters, |rules| [self.fighters[0], rules.ally2])
    }

    pub fn supports_online(&self) -> bool {
        self.config.world_tour.is_none() && self.config.dramatic.is_none_or(|rules| {
            rules.co_op
                && self.config.preset != RulesetPreset::Faithful1987
                && self.config.roster.mode == PlayMode::Versus
                && !self.config.p2_fair_cpu
                && rules.styles_available(&self.config)
        })
    }

    /// Explicit, little-endian schema. Inert scheme choices are never written.
    pub fn to_bytes(&self) -> Vec<u8> {
        let Self { config, fighters, styles, dips, stage, seed, bank_hash, art_pack_hash, protocol_hash } = self;
        let mut out = vec![if config.dramatic.is_some() { 2 } else if *styles != [None; 2] { 3 } else { 1 }];
        write_config(config, &mut out);
        for fighter in fighters { write_fighter(*fighter, &mut out); }
        dips.difficulty.write(&mut out);
        dips.round_time.write(&mut out);
        let StagePick::Bank(index) = stage;
        index.write(&mut out);
        seed.write(&mut out);
        bank_hash.write(&mut out);
        art_pack_hash.write(&mut out);
        protocol_hash.write(&mut out);
        if let Some(rules) = config.dramatic {
            write_dramatic_rules(rules, &mut out);
        }
        if out[0] == 3 {
            for style in styles { write_style(*style, &mut out); }
        }
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let mut read = Reader::new(bytes);
        let value = Self::read(&mut read)?;
        read.done().then_some(value)
    }

    fn read(read: &mut Reader<'_>) -> Option<Self> {
        let schema = read.u8()?;
        if !matches!(schema, 1..=3) { return None; }
        let mut config = read_config(read)?;
        let fighters = [read_fighter(read)?, read_fighter(read)?];
        let dips = FightDips { difficulty: read.u8()?, round_time: read.u16()? };
        if !(1..=4).contains(&dips.difficulty) || ![100, 150, 200, 250].contains(&dips.round_time) {
            return None;
        }
        let stage = StagePick::Bank(read.u8()?);
        if [config.roster.p1, config.roster.p2] != fighters || config.roster.stage != Some(stage)
            || config.difficulty != dips.difficulty || config.round_time != dips.round_time
        { return None; }
        let seed = read.u64()?;
        let bank_hash = read.u64()?;
        let art_pack_hash = read.u64()?;
        let protocol_hash = read.u32()?;
        if schema == 2 {
            config.dramatic = Some(read_dramatic_rules(read)?);
        }
        let styles = if schema == 3 {
            let picks = [read_style(read)?, read_style(read)?];
            if picks == [None; 2] || !config.era_select_enabled()
                || !fighters.into_iter().zip(picks).all(|(fighter, pick)| {
                    pick.is_none_or(|pick| pick.available_for(fighter, config.roster.what_if_styles))
                })
            {
                return None;
            }
            picks
        } else {
            [None; 2]
        };
        Some(Self { config, fighters, styles, dips, stage, seed, bank_hash, art_pack_hash, protocol_hash })
    }
}

trait WireValue { fn write(&self, out: &mut Vec<u8>); }
macro_rules! wire_scalar {
    ($($ty:ty),*) => { $(impl WireValue for $ty {
        fn write(&self, out: &mut Vec<u8>) { out.extend(self.to_le_bytes()); }
    })* };
}
wire_scalar!(u8, u16, u32, u64, i8);
impl WireValue for bool { fn write(&self, out: &mut Vec<u8>) { out.push(u8::from(*self)); } }

struct Reader<'a> { bytes: &'a [u8], at: usize }
impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self { Self { bytes, at: 0 } }
    fn take<const N: usize>(&mut self) -> Option<[u8; N]> {
        let end = self.at.checked_add(N)?;
        let value = self.bytes.get(self.at..end)?.try_into().ok()?;
        self.at = end;
        Some(value)
    }
    fn u8(&mut self) -> Option<u8> { Some(self.take::<1>()?[0]) }
    fn boolean(&mut self) -> Option<bool> { match self.u8()? { 0 => Some(false), 1 => Some(true), _ => None } }
    fn u16(&mut self) -> Option<u16> { Some(u16::from_le_bytes(self.take()?)) }
    fn u32(&mut self) -> Option<u32> { Some(u32::from_le_bytes(self.take()?)) }
    fn u64(&mut self) -> Option<u64> { Some(u64::from_le_bytes(self.take()?)) }
    fn done(&self) -> bool { self.at == self.bytes.len() }
}

// Explicit schema order, and exhaustive destructuring for future field audits.
macro_rules! record_codec {
    ($write:ident, $read:ident, $ty:ident, { $($field:ident: $method:ident),* $(,)? }) => {
        fn $write(value: &$ty, out: &mut Vec<u8>) {
            let $ty { $($field),* } = value;
            $($field.write(out);)*
        }
        fn $read(read: &mut Reader<'_>) -> Option<$ty> { Some($ty { $($field: read.$method()?),* }) }
    };
}
macro_rules! enum_codec {
    ($write:ident, $read:ident, $ty:ident, { $($variant:ident => $id:literal),* $(,)? }) => {
        fn $write(value: $ty, out: &mut Vec<u8>) { out.push(match value { $($ty::$variant => $id),* }); }
        fn $read(read: &mut Reader<'_>) -> Option<$ty> { match read.u8()? { $($id => Some($ty::$variant),)* _ => None } }
    };
}
enum_codec!(write_preset, read_preset, RulesetPreset, {
    DefinitiveOverhaul => 0, Faithful1987 => 1, ClassicCompetitive => 2, PureFootsies1987Plus => 3, Custom => 4,
});
enum_codec!(write_fighter, read_fighter, TournamentFighter, {
    Ryu => 0, Ken => 1, Retsu => 2, Geki => 3, Joe => 4, Mike => 5,
    Lee => 6, Gen => 7, Birdie => 8, Eagle => 9, Adon => 10, Sagat => 11,
});
enum_codec!(write_ban, read_ban, BanFormat, { StandardTournament => 0, LowTierNoShoto => 1, Unrestricted => 2 });
enum_codec!(write_mode, read_mode, PlayMode, { Arcade => 0, Versus => 1, Training => 2, Cpu => 3 });
enum_codec!(write_country, read_country, StartingCountry, { Japan => 0, China => 1, Usa => 2, England => 3 });
enum_codec!(write_gauge, read_gauge, GaugeMode, { EraDefault => 0, NoGauges => 1, Unified => 2 });
enum_codec!(write_lifebar, read_lifebar, LifebarMode, { Shared => 0, Independent => 1 });
enum_codec!(write_revision, read_revision, Revision, {
    Arcade1987 => 0, Sf1Plus => 1, Ww => 2, Ce => 3, Hf => 4, Super => 5,
    St => 6, Sfz1 => 7, Sfz2 => 8, Sfz2A => 9, Sfz3 => 10, Sf3Ng => 11,
    Sf3TwoI => 12, Sf3ThreeS => 13, Sf4 => 14, Ssf4 => 15, Ae2012 => 16,
    Usf4 => 17, Sf5 => 18, Sf5Ae => 19, Sf5Ce => 20, Sf6 => 21,
});
enum_codec!(write_loadout, read_loadout, Loadout, {
    Standard => 0, Old => 1, AIsm => 2, XIsm => 3, VIsm => 4,
    Art1 => 5, Art2 => 6, Art3 => 7, Ultra1 => 8, Ultra2 => 9, UltraDouble => 10,
    Skill1Trigger1 => 11, Skill1Trigger2 => 12, Skill2Trigger1 => 13, Skill2Trigger2 => 14,
});

fn write_style(style: Option<StylePick>, out: &mut Vec<u8>) {
    style.is_some().write(out);
    if let Some(style) = style {
        write_revision(style.revision, out);
        write_loadout(style.loadout, out);
    }
}

fn read_style(read: &mut Reader<'_>) -> Option<Option<StylePick>> {
    if !read.boolean()? { return Some(None); }
    let revision = read_revision(read)?;
    let loadout = read_loadout(read)?;
    loadouts(revision).contains(&loadout).then(|| Some(StylePick::for_revision(revision, loadout)))
}

fn write_dramatic_rules(rules: DramaticRules, out: &mut Vec<u8>) {
    let DramaticRules { co_op, ally2, stage, lifebar, friendly_fire, styles, rivalry } = rules;
    co_op.write(out);
    write_fighter(ally2, out);
    write_fighter(stage, out);
    write_lifebar(lifebar, out);
    friendly_fire.write(out);
    for style in styles {
        write_revision(style, out);
    }
    rivalry.map_or(0, |index| index.saturating_add(1)).write(out);
}

fn read_dramatic_rules(read: &mut Reader<'_>) -> Option<DramaticRules> {
    Some(DramaticRules {
        co_op: read.boolean()?,
        ally2: read_fighter(read)?,
        stage: read_fighter(read)?,
        lifebar: read_lifebar(read)?,
        friendly_fire: read.boolean()?,
        styles: [read_revision(read)?, read_revision(read)?, read_revision(read)?],
        rivalry: match read.u8()? {
            0 => None,
            tag @ 1..=6 => Some(tag - 1),
            _ => return None,
        },
    })
}

record_codec!(write_toggles, read_toggles, CoreToggles, {
    positive_edge_attacks: boolean, negative_edge_specials: boolean, recovery_input_buffer: boolean,
    recovery_buffer_window: u8, lenient_command_trackers: boolean, command_tracker_window: u8,
    command_release_grace: u8, interruptible_walk_cycle: boolean, anytime_jump_attacks: boolean,
    fast_prejump_crouch: boolean, standard_high_low_block: boolean, balanced_roster: boolean,
    true_combos: boolean, cancels: boolean, throws: boolean, chip_death: boolean,
    disable_fireball_slowdown: boolean, real_second_timer_and_draws: boolean,
    fair_cpu_and_sagat_scaling: boolean, cpu_reaction_delay: u8,
});
record_codec!(write_sf1plus, read_sf1plus, Sf1PlusRules, {
    classic_1987_special_damage: boolean, disable_freeze_input_lockout: boolean,
    untechable_sf2_throws: boolean, allow_corner_throw_loops: boolean,
});
record_codec!(write_tour, read_tour, TourRules, {
    rpg: boolean, projectile_only: boolean, no_blocking: boolean, survival_waves: u8,
    time_attack_frames: u16, dramatic: boolean, cross_era: boolean,
});

fn write_roster(value: &RosterSettings, out: &mut Vec<u8>) {
    let RosterSettings { full_cast, four_country, what_if_styles, bonus_challengers, secret_intrusions,
        ban, mode, p1, p2, country, stage } = value;
    for flag in [full_cast, four_country, what_if_styles, bonus_challengers, secret_intrusions] { flag.write(out); }
    write_ban(*ban, out); write_mode(*mode, out); write_fighter(*p1, out); write_fighter(*p2, out); write_country(*country, out);
    stage.is_some().write(out);
    match stage { Some(StagePick::Bank(index)) => index.write(out), None => 0u8.write(out) }
}

fn read_roster(read: &mut Reader<'_>) -> Option<RosterSettings> {
    let mut value = RosterSettings {
        full_cast: read.boolean()?, four_country: read.boolean()?, what_if_styles: read.boolean()?,
        bonus_challengers: read.boolean()?, secret_intrusions: read.boolean()?, ban: read_ban(read)?,
        mode: read_mode(read)?, p1: read_fighter(read)?, p2: read_fighter(read)?, country: read_country(read)?, stage: None,
    };
    let has_stage = read.boolean()?;
    let stage = read.u8()?;
    if has_stage { value.stage = Some(StagePick::Bank(stage)); } else if stage != 0 { return None; }
    Some(value)
}

fn write_config(value: &MatchConfig, out: &mut Vec<u8>) {
    let MatchConfig { preset, toggles, roster, fill_meters, p2_fair_cpu, p1_scheme: _, p2_scheme: _,
        modern_control_damage_penalty, sf1plus, gauge_mode, disable_comeback_scaling, disable_raw_drive_rush,
        tour, difficulty, round_time, dramatic: _, world_tour: _ } = value;
    write_preset(*preset, out); write_toggles(toggles, out); write_roster(roster, out);
    fill_meters.write(out); p2_fair_cpu.write(out); modern_control_damage_penalty.write(out);
    write_sf1plus(sf1plus, out); write_gauge(*gauge_mode, out);
    disable_comeback_scaling.write(out); disable_raw_drive_rush.write(out);
    write_tour(tour, out); difficulty.write(out); round_time.write(out);
}

fn read_config(read: &mut Reader<'_>) -> Option<MatchConfig> {
    let value = MatchConfig {
        preset: read_preset(read)?, toggles: read_toggles(read)?, roster: read_roster(read)?,
        fill_meters: read.boolean()?, p2_fair_cpu: read.boolean()?,
        p1_scheme: ControlScheme::Classic, p2_scheme: ControlScheme::Classic,
        modern_control_damage_penalty: read.boolean()?, sf1plus: read_sf1plus(read)?, gauge_mode: read_gauge(read)?,
        disable_comeback_scaling: read.boolean()?, disable_raw_drive_rush: read.boolean()?,
        tour: read_tour(read)?, difficulty: read.u8()?, round_time: read.u16()?, dramatic: None, world_tour: None,
    };
    let t = value.toggles;
    (t.recovery_buffer_window <= 30 && (1..=60).contains(&t.command_tracker_window)
        && t.command_release_grace <= 30 && t.cpu_reaction_delay <= 60
        && (1..=4).contains(&value.difficulty) && [100, 150, 200, 250].contains(&value.round_time)).then_some(value)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NetPacket {
    Ping { seq: u32, sent_ms: u32, link: Link },
    Pong { seq: u32, echo_ms: u32, link: Link },
    Hello { request_id: u64, tag: [u8; 12], kind: RoomKind, link: Link,
        protocol_hash: u32, config_hash: u64, bank_hash: u64, art_pack_hash: u64 },
    Welcome { request_id: u64, generation: u32, frame_base: u32, phase: LobbyPhase, picker: u8, accepted: bool, reason: RejectReason,
        config: SyncedMatchRules, seed: u64, assigned_delay: u8, max_rollback: u8, host_tag: [u8; 12] },
    LobbyPick { generation: u32, seq: u32, side: u8, fighter: TournamentFighter, stage: StagePick,
        locked: bool, phase: LobbyPhase, random: bool, style: Option<StylePick> },
    /// `current_frame` is the first unsimulated frame (`next_frame`), including
    /// stalled ticks, when an input at `current_frame + delay` can be sampled.
    Input { current_frame: u32, start_frame: u32, count: u8, bits: [u16; 9],
        ack_frame: u32, frame_advantage: i8, checksum_frame: u32, checksum: u64 },
    SpectatorChunk { generation: u32, start_frame: u32, count: u8, pairs: [(u16, u16); 16] },
    SpectatorAck { generation: u32, next_expected_frame: u32 },
    AfterMatchVote { generation: u32, seq: u32, choice: AfterMatchChoice },
    /// Host's current setup draft, published to the second ally and spectators.
    DramaticSetup { generation: u32, seq: u32, rules: SyncedMatchRules },
    /// The second ally's locked choice for the referenced host draft.
    DramaticAllyPick {
        generation: u32, setup_seq: u32, fighter: TournamentFighter, revision: Revision,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RejectReason { None, WiredOnlyRequired, PackHashMismatch, RoomFull, VersionMismatch, UnsupportedMode }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AfterMatchChoice { Rematch, Reselect, Leave }
/// A delayed character lock must never be mistaken for a stage lock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LobbyPhase { Fighters, Stage }

enum_codec!(write_link, read_link, Link, { Ethernet => 0, Wifi => 1 });
enum_codec!(write_kind, read_kind, RoomKind, { Versus => 0, Spectate => 1, Dramatic => 2 });
enum_codec!(write_reason, read_reason, RejectReason, {
    None => 0, WiredOnlyRequired => 1, PackHashMismatch => 2, RoomFull => 3, VersionMismatch => 4, UnsupportedMode => 5,
});
enum_codec!(write_choice, read_choice, AfterMatchChoice, { Rematch => 0, Reselect => 1, Leave => 2 });
enum_codec!(write_phase, read_phase, LobbyPhase, { Fighters => 0, Stage => 1 });

impl NetPacket {
    /// All scalar fields are little-endian. Input slots/pairs beyond `count`
    /// are zero padding. The parser validates even locally constructed packets;
    /// callers can use it to check values before sending.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = b"SF1N\x01".to_vec();
        match self {
            Self::Ping { seq, sent_ms, link } => {
                bytes.push(1); seq.write(&mut bytes); sent_ms.write(&mut bytes); write_link(*link, &mut bytes);
            }
            Self::Pong { seq, echo_ms, link } => {
                bytes.push(2); seq.write(&mut bytes); echo_ms.write(&mut bytes); write_link(*link, &mut bytes);
            }
            Self::Hello { request_id, tag, kind, link, protocol_hash, config_hash, bank_hash, art_pack_hash } => {
                bytes.push(3); request_id.write(&mut bytes); bytes.extend(tag);
                write_kind(*kind, &mut bytes); write_link(*link, &mut bytes);
                protocol_hash.write(&mut bytes); config_hash.write(&mut bytes); bank_hash.write(&mut bytes); art_pack_hash.write(&mut bytes);
            }
            Self::Welcome { request_id, generation, frame_base, phase, picker, accepted, reason, config, seed, assigned_delay, max_rollback, host_tag } => {
                bytes.push(4); request_id.write(&mut bytes); generation.write(&mut bytes);
                accepted.write(&mut bytes); write_reason(*reason, &mut bytes); bytes.extend(config.to_bytes());
                seed.write(&mut bytes); assigned_delay.write(&mut bytes); max_rollback.write(&mut bytes); bytes.extend(host_tag);
                write_phase(*phase, &mut bytes); picker.write(&mut bytes);
                frame_base.write(&mut bytes);
            }
            Self::LobbyPick { generation, seq, side, fighter, stage: StagePick::Bank(stage), locked, phase, random, style } => {
                bytes.push(5); generation.write(&mut bytes); seq.write(&mut bytes); side.write(&mut bytes);
                write_fighter(*fighter, &mut bytes); stage.write(&mut bytes); locked.write(&mut bytes);
                write_phase(*phase, &mut bytes); random.write(&mut bytes);
                if style.is_some() { write_style(*style, &mut bytes); }
            }
            Self::Input { current_frame, start_frame, count, bits, ack_frame, frame_advantage, checksum_frame, checksum } => {
                bytes.push(6); current_frame.write(&mut bytes); start_frame.write(&mut bytes); count.write(&mut bytes);
                for bits in bits { bits.write(&mut bytes); }
                ack_frame.write(&mut bytes); frame_advantage.write(&mut bytes); checksum_frame.write(&mut bytes); checksum.write(&mut bytes);
            }
            Self::SpectatorChunk { generation, start_frame, count, pairs } => {
                bytes.push(7); generation.write(&mut bytes); start_frame.write(&mut bytes); count.write(&mut bytes);
                for (p1, p2) in pairs { p1.write(&mut bytes); p2.write(&mut bytes); }
            }
            Self::SpectatorAck { generation, next_expected_frame } => {
                bytes.push(8); generation.write(&mut bytes); next_expected_frame.write(&mut bytes);
            }
            Self::AfterMatchVote { generation, seq, choice } => {
                bytes.push(9); generation.write(&mut bytes); seq.write(&mut bytes); write_choice(*choice, &mut bytes);
            }
            Self::DramaticSetup { generation, seq, rules } => {
                bytes.push(10);
                generation.write(&mut bytes);
                seq.write(&mut bytes);
                bytes.extend(rules.to_bytes());
            }
            Self::DramaticAllyPick { generation, setup_seq, fighter, revision } => {
                bytes.push(11);
                generation.write(&mut bytes);
                setup_seq.write(&mut bytes);
                write_fighter(*fighter, &mut bytes);
                write_revision(*revision, &mut bytes);
            }
        }
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > MAX_PACKET_BYTES || bytes.get(..5)? != b"SF1N\x01" { return None; }
        let mut read = Reader::new(bytes.get(6..)?);
        let packet = match *bytes.get(5)? {
            1 => Self::Ping { seq: read.u32()?, sent_ms: read.u32()?, link: read_link(&mut read)? },
            2 => Self::Pong { seq: read.u32()?, echo_ms: read.u32()?, link: read_link(&mut read)? },
            3 => {
                let request_id = read.u64()?;
                let tag = read.take()?;
                if !valid_tag(tag) { return None; }
                Self::Hello { request_id, tag, kind: read_kind(&mut read)?, link: read_link(&mut read)?,
                    protocol_hash: read.u32()?, config_hash: read.u64()?, bank_hash: read.u64()?, art_pack_hash: read.u64()? }
            }
            4 => {
                let request_id = read.u64()?;
                let generation = read.u32()?;
                let accepted = read.boolean()?;
                let reason = read_reason(&mut read)?;
                let config = SyncedMatchRules::read(&mut read)?;
                let seed = read.u64()?;
                let assigned_delay = read.u8()?;
                let max_rollback = read.u8()?;
                let host_tag = read.take()?;
                let phase = read_phase(&mut read)?;
                let picker = read.u8()?;
                let frame_base = read.u32()?;
                if accepted != (reason == RejectReason::None) || seed != config.seed
                    || assigned_delay > 4 || !(1..=8).contains(&max_rollback) || !valid_tag(host_tag)
                    || frame_base == u32::MAX || picker > 1
                { return None; }
                Self::Welcome { request_id, generation, frame_base, phase, picker, accepted, reason, config, seed, assigned_delay, max_rollback, host_tag }
            }
            5 => {
                let generation = read.u32()?; let seq = read.u32()?; let side = read.u8()?;
                if side > 1 { return None; }
                let fighter = read_fighter(&mut read)?;
                let stage = StagePick::Bank(read.u8()?);
                let locked = read.boolean()?;
                let phase = read_phase(&mut read)?;
                let random = read.boolean()?;
                if phase == LobbyPhase::Fighters && random { return None; }
                let style = if read.done() { None } else { Some(read_style(&mut read)??) };
                if style.is_some() && (phase != LobbyPhase::Fighters || !locked) { return None; }
                Self::LobbyPick { generation, seq, side, fighter, stage, locked, phase, random, style }
            }
            6 => {
                let current_frame = read.u32()?; let start_frame = read.u32()?; let count = read.u8()?;
                if !(1..=9).contains(&count) || current_frame == u32::MAX { return None; }
                let mut bits = [0; 9]; for bits in &mut bits { *bits = read.u16()?; }
                let ack_frame = read.u32()?; let frame_advantage = read.u8()? as i8;
                let checksum_frame = read.u32()?; let checksum = read.u64()?;
                // Delayed local inputs legitimately reach four frames past the
                // sender's simulation tip. MAX is reserved for absent ACK/hash.
                let last = start_frame.checked_add(u32::from(count) - 1)?;
                let future_tip = current_frame.saturating_add(4).min(u32::MAX - 1);
                if last > future_tip
                    || !bits[..usize::from(count)].iter().all(|b| *b <= 0x03ff || *b == 0x8000)
                    || bits[usize::from(count)..].iter().any(|b| *b != 0)
                    || (ack_frame != u32::MAX && ack_frame > future_tip)
                    || (checksum_frame == u32::MAX && checksum != 0)
                    || (checksum_frame != u32::MAX && checksum_frame > current_frame)
                { return None; }
                Self::Input { current_frame, start_frame, count, bits, ack_frame, frame_advantage, checksum_frame, checksum }
            }
            7 => {
                let generation = read.u32()?; let start_frame = read.u32()?; let count = read.u8()?;
                if !(1..=16).contains(&count) { return None; }
                start_frame.checked_add(u32::from(count))?;
                let mut pairs = [(0, 0); 16];
                for pair in &mut pairs { *pair = (read.u16()?, read.u16()?); }
                if pairs[..usize::from(count)].iter().any(|(p1, p2)| *p1 > 0x03ff || *p2 > 0x03ff)
                    || pairs[usize::from(count)..].iter().any(|pair| *pair != (0, 0))
                { return None; }
                Self::SpectatorChunk { generation, start_frame, count, pairs }
            }
            8 => Self::SpectatorAck { generation: read.u32()?, next_expected_frame: read.u32()? },
            9 => Self::AfterMatchVote { generation: read.u32()?, seq: read.u32()?, choice: read_choice(&mut read)? },
            10 => {
                let generation = read.u32()?;
                let seq = read.u32()?;
                let rules = SyncedMatchRules::read(&mut read)?;
                rules.config.dramatic?;
                Self::DramaticSetup { generation, seq, rules }
            }
            11 => Self::DramaticAllyPick {
                generation: read.u32()?,
                setup_seq: read.u32()?,
                fighter: read_fighter(&mut read)?,
                revision: read_revision(&mut read)?,
            },
            _ => return None,
        };
        read.done().then_some(packet)
    }
}

/// Pure host policy: joining peers adopt the host's rules. Their preferred
/// config hash is advisory; engine/asset compatibility is mandatory. `room_full`
/// is supplied by the session (one challenger, or four spectators).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandshakePolicy {
    pub rules: SyncedMatchRules,
    pub wired_only: bool,
    pub host_tag: [u8; 12],
    pub max_rollback: u8,
    pub generation: u32,
    pub frame_base: u32,
    pub phase: LobbyPhase,
    pub picker: u8,
    pub input_delay: Option<u8>,
}

impl HandshakePolicy {
    pub fn welcome(&self, hello: &NetPacket, rtt_ms: u32, room_full: bool) -> Option<NetPacket> {
        let NetPacket::Hello { request_id, tag, kind, link, protocol_hash, bank_hash, art_pack_hash, .. } = hello else { return None; };
        if !valid_tag(*tag) || !valid_tag(self.host_tag) { return None; }
        let reason = if *protocol_hash != self.rules.protocol_hash {
            RejectReason::VersionMismatch
        } else if *bank_hash != self.rules.bank_hash || *art_pack_hash != self.rules.art_pack_hash {
            RejectReason::PackHashMismatch
        } else if !self.rules.supports_online() || (kind.is_combatant() && *kind != self.rules.room_kind()) {
            RejectReason::UnsupportedMode
        } else if self.wired_only && *link == Link::Wifi {
            RejectReason::WiredOnlyRequired
        } else if room_full {
            RejectReason::RoomFull
        } else {
            RejectReason::None
        };
        let mut config = self.rules;
        if reason == RejectReason::VersionMismatch {
            // A peer with the original rules decoder can still read this rejection.
            config.config.dramatic = None;
            config.styles = [None; 2];
        }
        Some(NetPacket::Welcome {
            request_id: *request_id, generation: self.generation, frame_base: self.frame_base,
            phase: self.phase, picker: self.picker,
            accepted: reason == RejectReason::None, reason,
            config, seed: self.rules.seed,
            assigned_delay: self.input_delay.map(|frames| frames.min(4)).unwrap_or_else(|| (rtt_ms / 34).min(4) as u8),
            max_rollback: self.max_rollback.clamp(1, 8), host_tag: self.host_tag,
        })
    }
}

/// Integer RTT and EMA jitter, using caller-supplied monotonic milliseconds.
/// Wrapping u32 clocks are supported. Future/stale echoes over the five-second
/// timeout are ignored; matching Ping sequence numbers is the session's job.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RttEstimator { rtt: Option<u32>, jitter: u32 }

impl RttEstimator {
    pub fn observe(&mut self, now_ms: u32, echo_ms: u32) -> bool {
        let sample = now_ms.wrapping_sub(echo_ms);
        if sample > 5000 { return false; }
        if let Some(previous) = self.rtt {
            self.jitter = (7 * self.jitter + sample.abs_diff(previous)) / 8;
        }
        self.rtt = Some(sample);
        true
    }
    pub fn rtt_ms(&self) -> u32 { self.rtt.unwrap_or(0) }
    pub fn jitter_ms(&self) -> u32 { self.jitter }
    pub fn auto_delay(&self) -> u8 { (self.rtt_ms() / 34).min(4) as u8 }
}
