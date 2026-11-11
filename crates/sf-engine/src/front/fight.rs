//! A fight under the front end (ADR-0004): a faithful machine powered on
//! with the play mode's DIP words and configuration, carried to the real
//! country-select screen by the cabinet driver behind a black frame, then
//! shown and played on the real engine until the match ends or an
//! untranslated branch stops it.
//!
//! The DIP words are decoded once, at boot (`0x3c132..0x3c184`), so a
//! fight's difficulty and round time take effect only on a machine that
//! powers on with them; the attract's machine keeps the factory words and
//! is never driven.

use std::collections::VecDeque;
use std::sync::Arc;

use sf_video::announcer::AnnouncerLine;
use sf_video::background_bank::BackgroundBank;
use sf_video::callouts::{draw, draw_strip, draw_strip_inputs, regions, CalloutBoard, CalloutEvent, CalloutFrame, CalloutKind, Side};
use sf_video::compositor::{present, EraHud, OverhaulScene, PaletteTint, PresentMode, RomFont};
use sf_video::era_hud::GaugeArt;
use sf_video::render::{render_frame_edited, render_frame_staged_with_tints, Frame, SpriteTint, TextEdits, TextSpan};
use sf_video::scene::SceneState;
use sf_video::romset::Assets;

use super::cabinet::{
    at_country_select, chosen_country, drive_to_country_select, flow_position, pick_country,
    DriveError, Players, STAGE_START,
};
use super::coach::{Coach, Outcome};
use super::session::PlayerInputs;
use super::show::Show;
use super::sound;
use crate::config::{MatchConfig, RulesetPreset};
use crate::canonical::{canonical_enum, canonical_struct, Canonical, CanonicalHasher};
use crate::data_bank::DataBank;
use crate::era::StylePick;
use crate::fight_reader::{ComboCounter, FightRead, FightReader, FighterRead, RoundMoments};
use crate::fight_reader::FIGHTER_SLOTS;
use crate::fighter::{CPU_STRUCT, GAME_MODE, P1_STRUCT, P2_STRUCT};
use crate::moves::{InputNotation, MachineRead, MoveEntry};
use crate::roster::{PlayMode, TournamentFighter};
use crate::overlay::{gauge_lift, render_diagnostic_overlay_parts};
use crate::run::{PowerOnMachine, PowerOnSnapshot};
use crate::tasks::Io;

/// The game flow's round start (`0x3ea3e`): Training's restart point.
const ROUND_START: u32 = 0x3ea3e;
/// The fight loop (`0x3eafa`): the flow position while a round is fought.
const FIGHT_LOOP: u32 = 0x3eafa;
/// The continue countdown's loop (`0x3f092..0x3f162`).
const CONTINUE_LOOP: u32 = 0x3f162;
/// The credit count's low byte (`0xff865d`; `0xff865a..0xff865d`).
const CREDITS_LOW: u32 = 0xff865d;
/// 0 in the attract and its demo, `0xff` in a game (`0xff8003`).
const MODE_DEMO: u32 = 0xff8003;
/// The match winner byte: 1 Player 1, 2 Player 2, 4 the CPU, 0 none.
const MATCH_WINNER: u32 = 0xffc009;

/// Whether P1's fighter faces right, from its entity's facing byte (+0x37:
/// bit 2 set is left, as the engine tests it at `0x4444a`; 2 is right,
/// `docs/reference/adon.md`). The entity is the fighter slot P1's struct
/// points at; with none, facing right.
fn p1_facing_right(machine: &PowerOnMachine) -> bool {
    let ram = machine.work_ram();
    let at = (P1_STRUCT + 0x10 - 0xff8000) as usize;
    let entity = u16::from_be_bytes([ram[at], ram[at + 1]]);
    if !FIGHTER_SLOTS.contains(&entity) {
        return true;
    }
    machine.state.objectram.get(usize::from(entity) + 0x37).is_none_or(|&facing| facing & 4 == 0)
}

/// The text layer's width in cells.
const TEXT_COLS: usize = 64;
/// The ROM's TIME on the fight screen (`0x800e1c`, the label from
/// `0x14f1c`): row 28, the label's four letters and a gap, then three
/// digits, in columns 9 to 16; its first cell is the letter T.
const TIME_ROW: usize = 28;
const TIME_COLS: (usize, usize) = (9, 16);
const TIME_FIRST_CODE: u16 = 0x1d;
/// The ROM's score row (`round_hud`): P1, HI and P2 with their digits, row
/// 3, columns 10 to 53.
const SCORE_ROW: usize = 3;
const SCORE_COLS: (usize, usize) = (10, 53);
/// The ROM's health-bar labels: each label's text row, the word's cells
/// from column 22 (the bar starts at column 29) and the player structure
/// whose fighter the bar is. `PLAYER1` sits over P1's bar on row 5;
/// `PLAYER2`, or `ENEMY` for the CPU, over the other on row 7.
const LABEL_COL: usize = 22;
const LABEL_CELLS: usize = 7;
const LABELS: [(usize, &str, u32); 3] = [(5, "PLAYER1", 0xff86b0), (7, "PLAYER2", 0xff86da), (7, "ENEMY", 0xffc664)];
/// A player structure's move-table pointer (`+0x12`), which names its fighter.
const MOVE_TABLE: u32 = 0x12;
/// The round result's line (event 7, `docs/reference/round-result.md`):
/// text row 10, typed from column 21 for a player (`RYU WON THE 1ST ROUND`,
/// or `KEN WON …`, attribute 0x9000) or column 20 for the CPU (`ENEMY WON …`,
/// 0x5000), centred on column 31; the name's first column, the name, the
/// line's cells, the attribute and the winner's player structure.
const RESULT_ROW: usize = 10;
const RESULTS: [(usize, &str, usize, u16, u32); 3] = [
    (21, "RYU", 21, 0x9000, 0xff86b0),
    (21, "KEN", 21, 0x9000, 0xff86da),
    (20, "ENEMY", 23, 0x5000, 0xffc664),
];
/// The column wipe's tile (event 6, `docs/reference/round-transition.md`):
/// a wiped cell is the wipe's black, never hidden or moved.
const WIPE_TILE: u16 = 0x4a;
/// Where the overhaul layout draws TIME: eight tiles centred on the health
/// block (the PLAYER labels and bars, x 112 to 280), the top a little below
/// the bars' bottom edge (y 48).
const TIME_X: i32 = 196 - 32;
const TIME_Y: i32 = 52;
/// The text cells the drawn TIME covers (x 160 to 232, y 48 to 64): rows
/// 8 and 9, columns 28 to 36. The bonus tally after a won round writes
/// `ROUND BONUS` on row 9 (`docs/reference/match-win.md`); TIME gives way
/// to any ROM text there.
const UNDER_TIME_ROWS: (usize, usize) = (8, 9);
const UNDER_TIME_COLS: (usize, usize) = (28, 36);
/// The text layer's blank tile.
const BLANK_TILE: u16 = 0x3c;
/// The round-win markers (`round_markers`, `0x3bed2`): each side's two
/// 2 x 2 slots, two text rows of four cells from the video offset at its
/// player structure's `+0xc`, drawn empty by `round_hud` (tiles `0x102`,
/// `0x103`, `0x112`, `0x113`) and filled for each win (`0x100`, `0x101`,
/// `0x110`, `0x111`).
const MARKER_OFFSET: u32 = 0xc;
const MARKER_ROWS: usize = 2;
const MARKER_COLS: usize = 4;

/// Frames of black shown for the hidden drive (a cabinet reset's blink).
pub const DRIVE_BLACK_FRAMES: u32 = 15;
/// Hidden-drive frame budgets: power-on to the country select, and the
/// flag to the stage start.
const DRIVE_BUDGET: u32 = 4_000;
const PICK_BUDGET: u32 = 600;

/// The Gameplay tab's `DIFFICULTY` (1 Easy, 2 Normal, 3 Difficult, 4 Very
/// Difficult) and `ROUND TIME` (100, 150, 200, 250), which reach a fight as
/// its DSW2 bits (`docs/reference/dip-switches.md`); every other switch
/// stays at its factory setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FightDips {
    pub difficulty: u8,
    pub round_time: u16,
}

impl Default for FightDips {
    /// The factory settings: Normal, 100.
    fn default() -> Self {
        FightDips {
            difficulty: 2,
            round_time: 100,
        }
    }
}

impl FightDips {
    /// The factory DSW1 (`Io::idle`): demo sounds and attract music on,
    /// normal speed, no flip, freeze or service mode.
    pub const DSW1: u16 = 0xdfff;

    /// DSW2 with these settings in bits 5-6 (difficulty) and 3-4 (round
    /// time), the factory continuation, buy-in and countries elsewhere.
    pub fn dsw2(self) -> u16 {
        let difficulty = match self.difficulty {
            1 => 0x40,
            3 => 0x20,
            4 => 0x00,
            _ => 0x60,
        };
        let round_time = match self.round_time {
            150 => 0x10,
            200 => 0x08,
            250 => 0x00,
            _ => 0x18,
        };
        (0xffff & !0x0078) | difficulty | round_time
    }
}

/// What kind of fight: which machine it keeps, who drives the P2 side, and
/// where it stops.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FightKind {
    /// Two players; stops at the match end.
    Local,
    /// One player on the 1987 ladder; runs until the ROM's game over
    /// returns to its attract.
    Arcade,
    /// One player against a CPU opponent: under Faithful 1987 Capcom's CPU
    /// with the opponent the flag brings, under an overhaul preset the fair
    /// CPU on Player 2's pad (SP21); stops at the match end.
    TrainingCpu,
    /// A two-player fight whose P2 side never moves; stops at the match end.
    TrainingDummy,
    /// Two allies share the stage against the same CPU boss.
    Dramatic { co_op: bool },
}

impl FightKind {
    /// The kinds that stop at the match end with the after-match box.
    fn stops_at_match_end(self) -> bool {
        self != FightKind::Arcade
    }
}

/// The CPU's move table at the first fight from each flag of the
/// four-flag screen (0..=3), the same on all four difficulties
/// (`front_end_training.rs` checks them). Training against a CPU offers
/// these opponents, and the driver picks their flag.
pub const FIRST_OPPONENT_TABLES: [u32; 4] = [0x16e6c, 0x175bc, 0x1734c, 0x17a9c];

/// The flag whose first CPU opponent is `fighter`, if any.
pub fn first_opponent_flag(fighter: TournamentFighter) -> Option<u8> {
    FIRST_OPPONENT_TABLES
        .iter()
        .position(|&t| t == fighter.move_table())
        .map(|flag| flag as u8)
}

/// The diagnostic overlay's modes, as the pause box's `OVERLAYS` row
/// cycles them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OverlayMode {
    #[default]
    Off,
    Hitboxes,
    FrameMeter,
    Both,
}

impl OverlayMode {
    /// What the mode draws: the hitbox viewer, and the telemetry boxes with
    /// the frame meter.
    pub fn parts(self) -> (bool, bool) {
        match self {
            OverlayMode::Off => (false, false),
            OverlayMode::Hitboxes => (true, false),
            OverlayMode::FrameMeter => (false, true),
            OverlayMode::Both => (true, true),
        }
    }

    pub fn next(self) -> Self {
        match self {
            OverlayMode::Off => OverlayMode::Hitboxes,
            OverlayMode::Hitboxes => OverlayMode::FrameMeter,
            OverlayMode::FrameMeter => OverlayMode::Both,
            OverlayMode::Both => OverlayMode::Off,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            OverlayMode::Off => "OFF",
            OverlayMode::Hitboxes => "HITBOXES",
            OverlayMode::FrameMeter => "FRAME METER",
            OverlayMode::Both => "BOTH",
        }
    }
}

/// Everything a fight is started from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FightSetup {
    pub kind: FightKind,
    pub players: Players,
    /// The active preset's configuration with the chosen fighters stamped.
    pub config: MatchConfig,
    pub dsw1: u16,
    pub dsw2: u16,
}

canonical_enum!(FightKind, "fight-kind/v2"; Local => 0, Arcade => 1, TrainingCpu => 2, TrainingDummy => 3, Dramatic { co_op } => 4);
canonical_enum!(Players, "fight-players/v1"; One => 0, Two => 1);
canonical_struct!(FightSetup, "fight-setup/v1", { kind, players, config, dsw1, dsw2 });
canonical_struct!(PlayerInputs, "player-inputs/v1", { directions, buttons });
canonical_struct!(FighterRead, "show-fighter-read/v1", {
    present, health, move_id, state, blockstun, getting_up, attack_active,
});

/// The text layer's code for a character of the ROM font: the digits,
/// then the letters; anything else is the blank.
fn text_code(b: u8) -> u16 {
    match b {
        b'0'..=b'9' => u16::from(b - b'0'),
        b'A'..=b'Z' => 10 + u16::from(b - b'A'),
        _ => BLANK_TILE,
    }
}

/// What a fight frame ended in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FightEvent {
    /// The match ended (Local and Training): the flow waits on its wipe,
    /// before the original carries the winner on or brings the next
    /// opponent.
    MatchOver,
    /// The machine reached original branches the translation lacks.
    Untranslated { frame: u32, addresses: Vec<u32> },
    /// Arcade: the ROM's game-over flow returned to its attract
    /// (`0xff8003` back to 0).
    ReturnedToAttract,
}

impl From<DriveError> for FightEvent {
    fn from(e: DriveError) -> Self {
        match e {
            DriveError::Untranslated { frame, addresses } => FightEvent::Untranslated { frame, addresses },
            DriveError::OutOfBudget { frame, position } => FightEvent::Untranslated {
                frame,
                addresses: vec![position],
            },
        }
    }
}

/// One presented fight frame.
pub struct FightFrame {
    pub frame: Frame,
    pub sound_latch: u8,
    pub latch_cycles: u32,
    /// The announcer's lines asked for on this frame, in order (ADR 0011):
    /// none under the Faithful 1987 preset.
    pub announcements: Vec<AnnouncerLine>,
    pub event: Option<FightEvent>,
}

/// A running fight.
#[derive(Clone)]
pub struct Fight {
    machine: PowerOnMachine,
    setup: FightSetup,
    /// Locked Style card picks, immutable for this fight. The machine owns
    /// the corresponding authoritative round state.
    selected_styles: [StylePick; 2],
    /// The flag, once the country select has been left.
    country: Option<u8>,
    black: u32,
    sounds: VecDeque<u8>,
    over: bool,
    /// Arcade's free continue: frames into the coin and Start pulses fed
    /// after the player's Start on the continue countdown.
    continuing: Option<u32>,
    overlay: OverlayMode,
    /// How the era gauges are drawn, from the Settings value (the pause
    /// box's `GAUGE ART` changes it live). A fight carries no era state
    /// yet, so nothing is drawn in it until project 4 moves the eras onto
    /// the real fighters.
    gauge_art: GaugeArt,
    /// The machine at the start of the current round (`0x3ea3e`).
    round_start: Option<PowerOnSnapshot>,
    last_flow: u32,
    /// The era HUD's call-outs (spec #216), drawn over the overhaul layout.
    callouts: CalloutBoard,
    /// Call-out events for the next stepped frame.
    pending_callouts: Vec<CalloutEvent>,
    /// Finished combos' lines for the next stepped frame.
    pending_combos: Vec<AnnouncerLine>,
    /// The fight reader, its last frame, and the combo counts it feeds.
    reader: FightReader,
    read: FightRead,
    /// Reads that stand in for the reader's on the next stepped frames.
    fed_reads: VecDeque<FightRead>,
    combo: ComboCounter,
    /// The round's FIRST ATTACK, REVERSAL and PERFECT, from the same reads.
    moments: RoundMoments,
    /// Training's `HOLD COMBO`: the last count stays up until the next.
    hold_combo: bool,
    /// The preview gauges supplied through the renderer's test hook.
    era_hud: EraHud,
    /// Training's Show of a move (ticket #230): its input stands in for the
    /// player's pad until the move ends.
    show: Option<Show>,
    /// Training's Try of a move (ticket #231): the coach watches the
    /// player's fighter attempt it, from a show's end or the move's row,
    /// until the pause box opens or another move is picked.
    coach: Option<Coach>,
    /// P1's saved stock while Training supplies meter for Show and Try.
    practice_super: Option<u8>,
    /// How the coach's fixes write a direction (the Settings value `MOVE
    /// INPUTS`, ticket #232, ML6): a player's own choice, set by the session
    /// when a Show or Try starts.
    notation: InputNotation,
}

/// Complete mutable fight state at a pre-step netplay frame (NP1).
///
/// ROM, data and graphics banks stay on the receiving fight. Restore into
/// a fight with the same immutable banks; NP3 checks their compatibility.
/// Local overlay, gauge-art and input-notation choices are not rolled back.
/// Output/read queues are saved even though they are not gameplay checksum
/// inputs: resimulation must reconstruct the same per-frame events.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FightSnapshot {
    pub frame: u32,
    pub machine: PowerOnSnapshot,
    pub setup: FightSetup,
    pub country: Option<u8>,
    pub black: u32,
    pub sounds: VecDeque<u8>,
    pub over: bool,
    /// Arcade-only continue pulses; always None in an online Local fight.
    pub continuing: Option<u32>,
    pub round_start: Option<PowerOnSnapshot>,
    pub last_flow: u32,
    pub callouts: CalloutBoard,
    pub pending_callouts: Vec<CalloutEvent>,
    pub pending_combos: Vec<AnnouncerLine>,
    pub reader: FightReader,
    pub read: FightRead,
    pub fed_reads: VecDeque<FightRead>,
    pub combo: ComboCounter,
    pub moments: RoundMoments,
    pub hold_combo: bool,
    pub era_hud: EraHud,
    /// Training-only input driver and observer; both None in Local.
    pub show: Option<Show>,
    pub coach: Option<Coach>,
    pub practice_super: Option<u8>,
}

impl Fight {
    /// Canonical gameplay identity, independent of local presentation/delivery.
    ///
    /// The native machine, setup and wrapper flow determine future simulation.
    /// HUD readers/counts/call-outs, Coach advice, gauge drawings, pending sound
    /// delivery and diagnostic history are snapshotted for output replay but
    /// do not enter this hash. An active Training Show does drive future pad
    /// inputs, so its state and current/queued fighter observations are hashed.
    /// The external netplay frame is owned by NP2.
    pub fn state_checksum(&self) -> u64 {
        // Keep the checksum audit exhaustive as wrapper state evolves, too.
        let Self {
            machine, setup, country, black, over, continuing, round_start,
            last_flow, show, read, fed_reads,
            sounds: _, overlay: _, gauge_art: _, callouts: _, pending_callouts: _,
            pending_combos: _, reader: _, combo: _, moments: _, hold_combo: _,
            era_hud: _, coach: _, notation: _, selected_styles: _, practice_super,
        } = self;
        let mut out = CanonicalHasher::new();
        out.tag("sf1-port/fight-gameplay/v1");
        machine.encode_gameplay(&mut out);
        setup.encode(&mut out);
        country.encode(&mut out);
        black.encode(&mut out);
        over.encode(&mut out);
        continuing.encode(&mut out);
        last_flow.encode(&mut out);
        practice_super.encode(&mut out);
        match round_start {
            None => 0u8.encode(&mut out),
            Some(start) => {
                1u8.encode(&mut out);
                start.encode_gameplay(&mut out);
            }
        }
        match show {
            None => 0u8.encode(&mut out),
            Some(show) => {
                1u8.encode(&mut out);
                show.encode_gameplay(&mut out);
                read.fighters.encode(&mut out);
                fed_reads.len().encode(&mut out);
                for read in fed_reads {
                    read.fighters.encode(&mut out);
                }
            }
        }
        out.finish()
    }

    /// Save before stepping netplay frame `net_frame`, including frame 0.
    pub fn save_netplay_snapshot(&self, net_frame: u32) -> FightSnapshot {
        // Exhaustive destructuring makes new wrapper fields require an audit.
        let Self {
            machine, setup, country, black, sounds, over, continuing,
            overlay: _, gauge_art: _, selected_styles: _, round_start, last_flow, callouts,
            pending_callouts, pending_combos, reader, read, fed_reads,
            combo, moments, hold_combo, era_hud, show, coach, practice_super, notation: _,
        } = self;
        FightSnapshot {
            frame: net_frame,
            machine: machine.save_snapshot(),
            setup: *setup,
            country: *country,
            black: *black,
            sounds: sounds.clone(),
            over: *over,
            continuing: *continuing,
            round_start: round_start.clone(),
            last_flow: *last_flow,
            callouts: callouts.clone(),
            pending_callouts: pending_callouts.clone(),
            pending_combos: pending_combos.clone(),
            reader: reader.clone(),
            read: *read,
            fed_reads: fed_reads.clone(),
            combo: combo.clone(),
            moments: moments.clone(),
            hold_combo: *hold_combo,
            era_hud: *era_hud,
            show: show.clone(),
            coach: coach.clone(),
            practice_super: *practice_super,
        }
    }

    /// Restore the machine and wrapper while keeping current local choices.
    ///
    /// The caller owns the netplay frame counter; use `snap.frame` to rewind
    /// it together with the fight. That counter is not the cabinet's frame.
    pub fn restore_netplay_snapshot(&mut self, snap: &FightSnapshot) {
        let FightSnapshot {
            frame: _, machine, setup, country, black, sounds, over, continuing,
            round_start, last_flow, callouts, pending_callouts, pending_combos,
            reader, read, fed_reads, combo, moments, hold_combo, era_hud,
            show, coach, practice_super,
        } = snap;
        self.machine.restore_snapshot(machine);
        self.setup = *setup;
        self.country = *country;
        self.black = *black;
        self.sounds.clone_from(sounds);
        self.over = *over;
        self.continuing = *continuing;
        self.round_start.clone_from(round_start);
        self.last_flow = *last_flow;
        self.callouts.clone_from(callouts);
        self.pending_callouts.clone_from(pending_callouts);
        self.pending_combos.clone_from(pending_combos);
        self.reader.clone_from(reader);
        self.read = *read;
        self.fed_reads.clone_from(fed_reads);
        self.combo.clone_from(combo);
        self.moments.clone_from(moments);
        self.hold_combo = *hold_combo;
        self.era_hud = *era_hud;
        self.show.clone_from(show);
        self.coach.clone_from(coach);
        self.practice_super = *practice_super;
    }

    /// Whether the wrapper stopped after a match end or untranslated path.
    pub fn is_over(&self) -> bool {
        self.over
    }

    /// The current local notation choice, retained across netplay restores.
    pub fn move_inputs(&self) -> InputNotation {
        self.notation
    }

    /// Power on a machine for `setup` and drive it, hidden, to the country
    /// select; with `flag` that flag is picked too and the fight is handed
    /// over at the stage start (a rematch, or Training's CPU opponent). The
    /// machine holds `backgrounds`, the extended background bank, when given
    /// (an overhaul fight's; ADR-0009).
    pub fn start(
        program_rom: &[u8],
        setup: FightSetup,
        flag: Option<u8>,
        backgrounds: Option<Arc<BackgroundBank>>,
    ) -> Result<Fight, FightEvent> {
        Self::start_banked(program_rom, setup, flag, backgrounds, None)
    }

    /// [`Self::start`] with the extended data bank too (ADR-0014): the
    /// bosses' 1987 player tables, given only to an overhaul fight.
    pub fn start_banked(
        program_rom: &[u8],
        setup: FightSetup,
        flag: Option<u8>,
        backgrounds: Option<Arc<BackgroundBank>>,
        data: Option<Arc<DataBank>>,
    ) -> Result<Fight, FightEvent> {
        let mut machine = PowerOnMachine::with_io(
            program_rom.to_vec(),
            Io {
                dsw1: setup.dsw1,
                dsw2: setup.dsw2,
                ..Io::idle()
            },
        )
        .with_match_config(setup.config);
        if let Some(bank) = backgrounds {
            machine = machine.with_background_bank(bank);
        }
        if let Some(bank) = data {
            machine = machine.with_data_bank(bank);
        }
        let drive = drive_to_country_select(&mut machine, setup.players, DRIVE_BUDGET)?;
        let mut sounds: VecDeque<u8> = [sound::STOP_ALL, sound::STOP_SAMPLES].into();
        sounds.extend(drive.latches);
        if let Some(country) = flag {
            let pick = pick_country(&mut machine, program_rom, country, PICK_BUDGET)?;
            sounds.extend(pick.latches);
        }
        let last_flow = flow_position(&machine);
        Ok(Fight {
            country: flag,
            machine,
            setup,
            selected_styles: [StylePick::for_preset(setup.config.preset); 2],
            black: DRIVE_BLACK_FRAMES,
            sounds,
            over: false,
            continuing: None,
            overlay: OverlayMode::Off,
            gauge_art: GaugeArt::default(),
            round_start: None,
            last_flow,
            callouts: CalloutBoard::new(),
            pending_callouts: Vec::new(),
            pending_combos: Vec::new(),
            reader: FightReader::new(),
            read: FightRead::default(),
            fed_reads: VecDeque::new(),
            combo: ComboCounter::new(),
            moments: RoundMoments::new(),
            hold_combo: false,
            era_hud: EraHud::NONE,
            show: None,
            coach: None,
            practice_super: None,
            notation: InputNotation::default(),
        })
    }

    /// Whether this is a Training fight.
    pub fn training(&self) -> bool {
        matches!(self.setup.kind, FightKind::TrainingCpu | FightKind::TrainingDummy)
    }

    /// Take the stage tune command a stand-in fight fell back to when Ryu's
    /// extended theme was missing (SP15, `#267`).
    pub fn take_missing_ryu_theme(&mut self) -> Option<u16> {
        self.machine.take_missing_ryu_theme()
    }

    /// The fight reader's read of the last stepped frame.
    pub fn fight_read(&self) -> &FightRead {
        &self.read
    }

    /// Reads that stand in for the fight reader's on the next stepped
    /// frames, one a frame, so a test can show any sequence of reads: a
    /// fight under the classic rules has no true combos (its fighters are
    /// invulnerable for their whole hit reaction), and under true combos
    /// (SP17) a long string is hard to set up by inputs. The machine never
    /// sees them; the reader still reads every frame.
    pub fn feed_reads(&mut self, reads: &[FightRead]) {
        self.fed_reads.extend(reads.iter().copied());
    }

    pub fn hold_combo(&self) -> bool {
        self.hold_combo
    }

    /// Training's `HOLD COMBO`; outside Training a count always clears.
    pub fn set_hold_combo(&mut self, hold: bool) {
        self.hold_combo = hold;
    }

    /// Training's `FILL METERS`: the fight's configuration takes it, and
    /// the live machine with it, as [`Self::set_preset`] pushes a preset.
    /// Outside Training it stays off.
    pub fn set_fill_meters(&mut self, on: bool) {
        self.setup.config.fill_meters = on && self.training();
        self.machine.set_match_config(self.setup.config);
    }

    pub fn setup(&self) -> &FightSetup {
        &self.setup
    }

    pub(super) fn set_selected_styles(&mut self, styles: [Option<StylePick>; 2]) {
        let fixed = StylePick::for_preset(self.setup.config.preset);
        self.selected_styles = styles.map(|pick| pick.unwrap_or(fixed));
        self.machine.set_era_styles(self.selected_styles);
    }

    /// The selected style for a side. Arcade's second side follows the current
    /// ladder opponent and is absent before a country is picked.
    pub fn fighter_style(&self, side: usize) -> Option<StylePick> {
        if side == 1 && self.setup.kind == FightKind::Arcade {
            let opponent = self.machine.arcade_opponent()?;
            return Some(self.selected_styles[0].arcade_opponent(
                opponent,
                self.setup.config.roster.what_if_styles,
                self.machine.arcade_ladder_step(),
            ));
        }
        self.selected_styles.get(side).copied()
    }

    /// Fighters assigned to the two local or network pads.
    pub fn player_fighter(&self, side: usize) -> TournamentFighter {
        let roster = self.setup.config.roster;
        if side == 0 {
            roster.p1
        } else {
            self.setup.config.dramatic.map_or(roster.p2, |rules| rules.ally2)
        }
    }

    pub(super) fn seed_dramatic_cpu(&mut self, seed: u64) {
        self.machine.seed_dramatic_cpu(seed);
    }

    pub fn machine(&self) -> &PowerOnMachine {
        &self.machine
    }

    /// The flag picked, once known.
    pub fn country(&self) -> Option<u8> {
        self.country
    }

    /// Who won the match that just ended, from the original's winner byte
    /// (`0xffc009`: 1 Player 1, 2 Player 2, 4 the CPU, 0 no winner):
    /// `Some(0)` for Player 1, `Some(1)` for Player 2 or the CPU, `None`
    /// for a draw. Meaningful once the fight reported `MatchOver`. Counting
    /// the CPU (4) as Player 2 is right only where the human is Player 1,
    /// as in Local; Arcade never reaches the after-match box, and Training
    /// fights never end (ADR-0013), so the other case does not arise today.
    pub fn match_winner(&self) -> Option<usize> {
        match self.machine.work_ram()[(MATCH_WINNER - 0xff8000) as usize] {
            1 => Some(0),
            0 => None,
            _ => Some(1),
        }
    }

    /// The screen's name: the hidden drive's black, the real country
    /// select, the continue countdown, or the fight.
    pub fn name(&self) -> &'static str {
        if self.black > 0 {
            "drive"
        } else if at_country_select(&self.machine) {
            "country select"
        } else if self.in_continue() {
            "continue"
        } else {
            "fight"
        }
    }

    /// Whether the hidden drive's black is still showing.
    pub fn driving(&self) -> bool {
        self.black > 0
    }

    pub fn overlay(&self) -> OverlayMode {
        self.overlay
    }

    pub fn set_overlay(&mut self, mode: OverlayMode) {
        self.overlay = mode;
    }

    pub fn gauge_art(&self) -> GaugeArt {
        self.gauge_art
    }

    pub fn set_gauge_art(&mut self, art: GaugeArt) {
        self.gauge_art = art;
    }

    /// The machine at the current round's start, if one has started.
    pub fn round_start(&self) -> Option<&PowerOnSnapshot> {
        self.round_start.as_ref()
    }

    /// Put the machine back to the current round's start, bit for bit.
    pub fn restart_round(&mut self) -> bool {
        let Some(snap) = &self.round_start else {
            return false;
        };
        self.machine.restore_snapshot(snap);
        self.last_flow = flow_position(&self.machine);
        self.reader.reset();
        self.combo.cut();
        self.moments.new_round();
        // The abandoned attempt's words go with it.
        self.callouts = CalloutBoard::new();
        self.pending_callouts.clear();
        self.show = None;
        self.coach = None;
        self.practice_super = None;
        true
    }

    /// Training's `RESET POSITIONS`: the round's start, as [`Self::restart_round`]
    /// puts it back, with the fight's present configuration in the machine.
    /// The snapshot holds the configuration the round began under, so a
    /// preset changed since would otherwise be undone in the machine while
    /// `setup` still names it.
    pub fn reset_positions(&mut self) -> bool {
        if !self.restart_round() {
            return false;
        }
        self.machine.set_match_config(self.setup.config);
        true
    }

    /// Switch the live preset (Training), keeping the play mode, the chosen
    /// fighters, which stay bound, and `FILL METERS`, which is the player's
    /// and no preset's.
    pub fn set_preset(&mut self, preset: RulesetPreset) {
        let mut config = MatchConfig::from_preset(preset);
        config.roster.mode = self.setup.config.roster.mode;
        config.roster.p1 = self.setup.config.roster.p1;
        config.roster.p2 = self.setup.config.roster.p2;
        config.fill_meters = self.setup.config.fill_meters;
        // Who plays P2's pad, and the match settings, are the fight's.
        config.p2_fair_cpu = self.setup.config.p2_fair_cpu;
        config.difficulty = self.setup.config.difficulty;
        config.round_time = self.setup.config.round_time;
        config.roster.full_cast = true;
        // The picked stage stays under the overhaul presets; Faithful 1987
        // closes its gates from the next stage set-up on.
        if preset != RulesetPreset::Faithful1987 {
            config.roster.stage = self.setup.config.roster.stage;
        }
        self.setup.config = config;
        self.machine.set_match_config(config);
    }

    /// Whether the ROM's continue countdown is running (`0x3f162`).
    pub fn in_continue(&self) -> bool {
        flow_position(&self.machine) == CONTINUE_LOOP
    }

    /// Sets preview gauges for screenshots and renderer tests.
    pub fn set_era_hud(&mut self, hud: EraHud) {
        self.era_hud = hud;
    }

    /// Era gauges driven by the current native fighter state.
    pub fn era_hud(&self) -> EraHud {
        if self.era_hud != EraHud::NONE {
            return self.era_hud;
        }
        let [p1, p2] = self.machine.era_fighters();
        EraHud { p1: p1.gauge(), p2: p2.gauge() }
    }

    /// Call-out events that arrive with the next stepped frame. They change
    /// only the presented frame: the machine and its inputs never see them,
    /// and the Faithful 1987 preset never shows them.
    pub fn push_callouts(&mut self, events: &[CalloutEvent]) {
        self.pending_callouts.extend_from_slice(events);
    }

    /// A combo of `hits` has ended, on either side (a CPU side's as a
    /// human's): from five hits the announcer calls it with the next
    /// stepped frame ("Five hit combo!"). He says nothing while it runs.
    pub fn finish_combo(&mut self, hits: u16) {
        self.pending_combos.extend(AnnouncerLine::for_combo(hits));
    }

    /// Have the player's fighter perform `entry` (Training's Show, ticket
    /// #230): once the fighter is free and standing the entry's input is fed
    /// to P1 in place of the pad's, until the move ends. Outside Training it
    /// does nothing. A show already running is replaced.
    pub fn show(&mut self, entry: MoveEntry) {
        if self.training() {
            self.begin_super_practice(&entry);
            self.show = Some(Show::new(entry));
            self.coach = None;
        }
    }

    /// Have the coach watch the player try `entry` (Training's Try, ticket
    /// #231): nothing is fed, the pad is the player's. It runs until
    /// [`Self::stop_try`] or another move is picked, and a Show of the same
    /// move that finishes starts it too. Outside Training it does nothing.
    pub fn try_move(&mut self, entry: MoveEntry) {
        if self.training() {
            self.begin_super_practice(&entry);
            self.coach = Some(Coach::new(entry));
            self.show = None;
        }
    }

    /// How the coach writes the direction in a fix: the player's `MOVE
    /// INPUTS` choice.
    pub fn set_move_inputs(&mut self, notation: InputNotation) {
        self.notation = notation;
    }

    /// Stop the Try (the pause box opened).
    pub fn stop_try(&mut self) {
        if self.coach.take().is_some() {
            self.end_super_practice();
        }
    }

    fn begin_super_practice(&mut self, entry: &MoveEntry) {
        self.end_super_practice();
        if entry.kind == crate::moves::MoveKind::Super && !self.setup.config.fill_meters {
            self.practice_super = Some(self.machine.era_fighters()[0].super_meter);
            self.machine.training_super_meter(None);
        }
    }

    fn end_super_practice(&mut self) {
        if let Some(saved) = self.practice_super.take() {
            self.machine.training_super_meter(Some(saved));
        }
    }

    /// The coach, while a Try runs.
    pub fn coach(&self) -> Option<&Coach> {
        self.coach.as_ref()
    }

    /// The show in progress, if any.
    pub fn showing(&self) -> Option<&Show> {
        self.show.as_ref()
    }

    /// The call-outs showing.
    pub fn callouts(&self) -> &CalloutBoard {
        &self.callouts
    }

    /// The machine, for the attract once an Arcade run has returned to it.
    pub fn into_machine(self) -> PowerOnMachine {
        self.machine
    }

    /// The cabinet switches for Arcade's free continue: the player's
    /// `start` on the countdown feeds a coin, then, once it is credited,
    /// `start1`; the ROM's own continue limit still applies. Nothing
    /// otherwise: no coin is left in the machine when the countdown runs
    /// out.
    fn continue_switches(&mut self, start: bool) -> (bool, bool) {
        if self.continuing.is_none() && start && self.in_continue() {
            self.continuing = Some(0);
        }
        let Some(t) = self.continuing else {
            return (false, false);
        };
        let credited = self.machine.work_ram()[(CREDITS_LOW - 0xff8000) as usize] != 0;
        let (coin, start1) = (t < 12, t >= 12 && credited && t < 40);
        self.continuing = (t < 60).then_some(t + 1);
        (coin, start1)
    }

    /// The overhaul fight layout, as edits to the presented text layer:
    /// the ROM's TIME tiles hidden and drawn again centred under the health
    /// bars (not while the ROM has text there, such as the bonus tally's
    /// ROUND BONUS), the P1 / HI / P2 scores hidden outside Arcade, and the
    /// fighters named where the ROM writes PLAYER1, PLAYER2 or ENEMY over the
    /// bars and RYU, KEN or ENEMY in the round result; cells the round's
    /// column wipe has reached are left to the wipe. Training has
    /// no rounds (ADR-0013, ticket #240): its TIME is hidden and not drawn
    /// again, and both sides' round-win markers are hidden too; under the
    /// Faithful 1987 preset only those three are, the rest of the 1987
    /// screen kept. `None` for Faithful 1987 outside Training, off the
    /// fight screen and for a flipped screen. The machine's RAM and text
    /// layer are only read.
    fn layout(&self, scene: &SceneState) -> Option<TextEdits> {
        let faithful = self.setup.config.preset == RulesetPreset::Faithful1987;
        if (faithful && !self.training()) || scene.flip_screen || self.name() != "fight" {
            return None;
        }
        let cell = |col: usize, row: usize| {
            let i = row * TEXT_COLS + col;
            u16::from_be_bytes([scene.videoram[2 * i], scene.videoram[2 * i + 1]])
        };
        let wiped = |col: usize, row: usize| cell(col, row) & 0x3ff == WIPE_TILE;
        let mut edits = TextEdits::default();
        // Hides `row`'s cells in `cols` except those the wipe has reached,
        // so the wipe's black covers the whole screen as it crosses.
        let mut hide = |row: usize, cols: (usize, usize)| {
            for col in (cols.0..=cols.1).filter(|&col| !wiped(col, row)) {
                match edits.hidden.last_mut() {
                    Some(span) if span.row == row && span.col_last + 1 == col => span.col_last = col,
                    _ => edits.hidden.push(TextSpan { row, col_first: col, col_last: col }),
                }
            }
        };
        // Only where the ROM drew TIME: Arcade's other screens (the bonus
        // stages, the ending, name entry) keep whatever they put there.
        let time = cell(TIME_COLS.0, TIME_ROW) & 0x3ff == TIME_FIRST_CODE;
        if time {
            hide(TIME_ROW, TIME_COLS);
        }
        if self.setup.kind != FightKind::Arcade {
            hide(SCORE_ROW, SCORE_COLS);
        }
        let ram = self.machine.work_ram();
        let rw = |at: u32| {
            let at = (at - 0xff8000) as usize;
            u16::from_be_bytes([ram[at], ram[at + 1]])
        };
        if self.training() {
            // Both sides' marker cells, from the two player structures
            // `round_hud` lays out for the game mode; only where a slot or
            // a marker is drawn.
            let sides = match ram[(GAME_MODE - 0xff8000) as usize] {
                2 => [P1_STRUCT, CPU_STRUCT],
                4 => [P2_STRUCT, CPU_STRUCT],
                _ => [P1_STRUCT, P2_STRUCT],
            };
            for player in sides {
                let offset = usize::from(rw(player + MARKER_OFFSET));
                let (row, first) = (offset / 0x80, offset % 0x80 / 2);
                for row in (row..row + MARKER_ROWS).filter(|&row| row < 32) {
                    for col in (first..first + MARKER_COLS).filter(|&col| col < TEXT_COLS) {
                        if matches!(cell(col, row) & 0x3ff, 0x100..=0x103 | 0x110..=0x113) {
                            hide(row, (col, col));
                        }
                    }
                }
            }
        }
        // Faithful 1987 Training keeps the 1987 labels and lettering.
        if faithful {
            return Some(edits);
        }
        // Each bar's fighter by name, in the label's place and colours: a
        // name of up to seven letters from where the label starts, a longer
        // one ending where it ends. A label the ROM did not draw, or whose
        // fighter is not one of the twelve, stays as it is.
        let mut names = Vec::new();
        for (row, label, player) in LABELS {
            let shown = label.bytes().enumerate().filter(|&(i, _)| !wiped(LABEL_COL + i, row)).count();
            let matches = label
                .bytes()
                .enumerate()
                .all(|(i, b)| wiped(LABEL_COL + i, row) || cell(LABEL_COL + i, row) & 0x3ff == text_code(b));
            if shown == 0 || !matches {
                continue;
            }
            let at = (player + MOVE_TABLE - 0xff8000) as usize;
            let table = u32::from_be_bytes([ram[at], ram[at + 1], ram[at + 2], ram[at + 3]]);
            let Some(fighter) = TournamentFighter::from_move_table(table) else {
                continue;
            };
            hide(row, (LABEL_COL, LABEL_COL + label.len() - 1));
            let colour = (LABEL_COL..LABEL_COL + label.len()).find(|&col| !wiped(col, row)).map_or(0, |col| cell(col, row) & !0x3ff);
            names.push((row, fighter.name().to_ascii_uppercase(), colour));
        }
        // The round result names its winner: the ROM's name swapped for the
        // fighter's, the rest of the line as typed so far, centred again; a
        // name still being typed shows as far through as the ROM's is.
        let mut result = Vec::new();
        for (first, word, cells, attr, player) in RESULTS {
            let ours = |col: usize| cell(col, RESULT_ROW) & !0x3ff == attr && cell(col, RESULT_ROW) & 0x3ff != BLANK_TILE;
            let Some(typed) = (first..first + cells).rev().find(|&col| ours(col)).map(|col| col + 1 - first) else {
                continue;
            };
            let name_typed = typed.min(word.len());
            if !word.bytes().take(name_typed).enumerate().all(|(i, b)| ours(first + i) && cell(first + i, RESULT_ROW) & 0x3ff == text_code(b)) {
                continue;
            }
            let at = (player + MOVE_TABLE - 0xff8000) as usize;
            let table = u32::from_be_bytes([ram[at], ram[at + 1], ram[at + 2], ram[at + 3]]);
            let Some(fighter) = TournamentFighter::from_move_table(table) else {
                continue;
            };
            let name = fighter.name().to_ascii_uppercase();
            if name == word {
                break;
            }
            hide(RESULT_ROW, (first, first + typed - 1));
            let start = (63 - (name.len() + cells - word.len())) / 2;
            let shown = (name_typed * name.len()).div_ceil(word.len());
            for (i, b) in name.bytes().take(shown).enumerate() {
                result.push((start + i, attr | text_code(b)));
            }
            for col in first + word.len()..first + typed {
                if cell(col, RESULT_ROW) & 0x3ff != BLANK_TILE {
                    result.push((start + name.len() + col - first - word.len(), cell(col, RESULT_ROW)));
                }
            }
            break;
        }
        let under_time_clear = (UNDER_TIME_ROWS.0..=UNDER_TIME_ROWS.1)
            .all(|row| (UNDER_TIME_COLS.0..=UNDER_TIME_COLS.1).all(|col| matches!(cell(col, row) & 0x3ff, BLANK_TILE | WIPE_TILE)));
        if time && !self.training() && under_time_clear {
            // The same tiles, as the ROM draws them, under the health bars;
            // a wiped tile leaves its place there to the screen beneath.
            for (i, col) in (TIME_COLS.0..=TIME_COLS.1).enumerate().filter(|&(_, col)| !wiped(col, TIME_ROW)) {
                edits.added.push((TIME_X + 8 * i as i32, TIME_Y, cell(col, TIME_ROW)));
            }
        }
        for (col, code) in result {
            edits.added.push((8 * col as i32 - 64, 8 * RESULT_ROW as i32 - 16, code));
        }
        for (row, name, colour) in names {
            let first = if name.len() <= LABEL_CELLS { LABEL_COL } else { LABEL_COL + LABEL_CELLS - name.len() };
            for (i, b) in name.bytes().enumerate().filter(|&(i, _)| !wiped(first + i, row)) {
                let (x, y) = (8 * (first + i) as i32 - 64, 8 * row as i32 - 16);
                edits.added.push((x, y, colour | text_code(b)));
            }
        }
        Some(edits)
    }

    /// The machine's frame through the faithful hardware path (the overhaul
    /// layout applied to the presented frame), the call-outs over it through
    /// the overhaul present path, and the overlay's chosen parts. The hitbox
    /// viewer draws under the era HUD, so its fills never cover a word or a
    /// gauge; the frame meter and telemetry, which never meet it, draw last.
    pub fn render(&self, assets: &Assets) -> Frame {
        let mut frame = self.render_scene(assets);
        self.draw_ally_lifebar(&mut frame);
        // Training's Show strip, under the health block in every preset.
        if let Some(show) = &self.show {
            let font = RomFont::new(&assets.gfx4, &assets.maincpu);
            draw_strip(&mut frame.argb, &show.strip(), Some(&font));
        }
        // And the Try's, with the call-out for a move that came out; the
        // overhaul path draws the call-outs, so under Faithful 1987 (which
        // never does) the Try's own words are drawn here.
        if let Some(coach) = self.coach.as_ref().filter(|_| self.last_flow == FIGHT_LOOP) {
            let font = RomFont::new(&assets.gfx4, &assets.maincpu);
            draw_strip(&mut frame.argb, &coach.strip(), Some(&font));
            // After a miss, the player's last inputs under it.
            if let Some(inputs) = coach.shown_inputs() {
                draw_strip_inputs(&mut frame.argb, inputs, Some(&font));
            }
            if self.setup.config.preset == RulesetPreset::Faithful1987 {
                let board = self.callouts.frame();
                let mut words = CalloutFrame { drop: board.drop, ..CalloutFrame::NONE };
                for (slot, word) in board.side(Side::P1).words.iter().enumerate() {
                    words.sides[Side::P1.index()].words[slot] = word.filter(|kind| matches!(kind, CalloutKind::Move { .. } | CalloutKind::Fix { .. }));
                }
                draw(&mut frame.argb, &words, Some(&font));
            }
        }
        frame
    }

    fn draw_ally_lifebar(&self, frame: &mut Frame) {
        use crate::dramatic::{LifebarMode, ALLY_SUB_HP};
        use sf_video::render::{HEIGHT, VISAREA_X, VISAREA_Y, WIDTH};

        if self.last_flow != FIGHT_LOOP
            || !self.setup.config.dramatic.is_some_and(|rules| rules.lifebar == LifebarMode::Independent)
        {
            return;
        }
        // The cabinet's Ally 1 bar begins at text-RAM offset 0x2bc.
        let (left, top) = (30 * 8 - VISAREA_X, 5 * 8 - VISAREA_Y + 10);
        let health = self.machine.toggle_state().dramatic.health[2].min(ALLY_SUB_HP);
        let fill = (usize::from(health) * 48).div_ceil(usize::from(ALLY_SUB_HP));
        let flip = self.machine.state.flip_screen;
        for row in 0..4 {
            for col in 0..50 {
                let (x, y) = if flip {
                    (WIDTH - 1 - left - col, HEIGHT - 1 - top - row)
                } else {
                    (left + col, top + row)
                };
                frame.argb[y * WIDTH + x] = match (row, col) {
                    (0 | 3, _) | (_, 0 | 49) => 0xffd5c49a,
                    (_, col) if col <= fill => 0xffe6b948,
                    _ => 0xff392626,
                };
            }
        }
    }

    fn era_sprite_tints(&self, scene: &SceneState) -> Vec<SpriteTint> {
        const OBJECT_STRIDE: usize = 0x40;
        let mut tints = Vec::new();
        let freeze = self.machine.era_super_freeze();
        if freeze != 0 {
            let telemetry = self.machine.diagnostic_telemetry();
            for fighter in [telemetry.p1, telemetry.p2] {
                if !fighter.active || fighter.move_id != crate::data_bank::SF2_SUPER_SLOT {
                    continue;
                }
                let start = usize::from(fighter.entity_offset);
                let Some(grid) = scene.objectram.get(start + 0xc..start + 0xe) else { continue };
                let cells = ((usize::from(grid[0]) + 1) * (usize::from(grid[1]) + 1)).min(24);
                tints.push(SpriteTint {
                    entries: start..start + cells * OBJECT_STRIDE,
                    tint: PaletteTint { color: 0xffffe9a6, strength: if freeze & 4 == 0 { 144 } else { 76 } },
                });
            }
        }
        for (index, effect) in self.machine.toggle_state().era.projectiles().iter().enumerate() {
            if effect.crimson && effect.hits_remaining > 0 {
                let start = 0x100 + index * OBJECT_STRIDE;
                tints.push(SpriteTint {
                    entries: start..start + OBJECT_STRIDE,
                    tint: PaletteTint { color: 0xffff351b, strength: 144 },
                });
            }
        }
        tints
    }

    fn render_scene(&self, assets: &Assets) -> Frame {
        let scene = self.machine.state.scene();
        let layout = self.layout(&scene);
        let callouts = self.callouts.frame();
        let era_hud = self.era_hud();
        let tints = self.era_sprite_tints(&scene);
        let (boxes, meter) = self.overlay.parts();
        let telemetry = || self.machine.diagnostic_telemetry();
        let base = match &layout {
            // Faithful 1987 Training's edits only hide cells: the hardware
            // path, as every Faithful 1987 frame.
            Some(edits)
                if self.setup.config.preset != RulesetPreset::Faithful1987
                    && (!callouts.is_empty() || era_hud != EraHud::NONE || !tints.is_empty()) =>
            {
                let (mut hardware, stage) = render_frame_staged_with_tints(assets, &scene, edits, &tints);
                if boxes {
                    hardware = render_diagnostic_overlay_parts(&hardware, &telemetry(), true, false);
                }
                let font = RomFont::new(&assets.gfx4, &assets.maincpu);
                let hud = present(
                    PresentMode::Overhaul,
                    &hardware,
                    &OverhaulScene {
                        callouts,
                        hud: era_hud,
                        gauge_art: self.gauge_art,
                        hud_lift: gauge_lift(meter),
                        stage: Some(&stage),
                        font: Some(&font),
                        ..OverhaulScene::EMPTY
                    },
                );
                if !meter {
                    return hud;
                }
                return render_diagnostic_overlay_parts(&hud, &telemetry(), false, true);
            }
            _ => render_frame_edited(assets, &scene, &layout.unwrap_or_default()),
        };
        if !boxes && !meter {
            return base;
        }
        render_diagnostic_overlay_parts(&base, &telemetry(), boxes, meter)
    }

    /// One frame: black (and the drive's sound commands) while the hidden
    /// drive's blink lasts, then the machine stepped with both players'
    /// inputs (a Training dummy's P2 side held still) and no cabinet
    /// switches but Arcade's free continue. Stops stepping once the match
    /// is over (Local, Training) or the run has returned to the attract
    /// (Arcade).
    pub fn step(&mut self, assets: &Assets, p1: PlayerInputs, p2: PlayerInputs, start: bool) -> FightFrame {
        if self.black > 0 {
            self.black -= 1;
            return FightFrame {
                frame: Frame {
                    argb: vec![0xff00_0000; sf_video::render::WIDTH * sf_video::render::HEIGHT],
                },
                sound_latch: self.sounds.pop_front().unwrap_or(sound::IDLE),
                latch_cycles: 0,
                announcements: Vec::new(),
                event: None,
            };
        }
        let mut event = None;
        if !self.over {
            // A Training dummy's pad is held still, and the fair CPU plays
            // P2's in the machine (SP21), so the pad's never reaches it.
            let p2 = if self.setup.kind == FightKind::TrainingDummy || self.setup.config.p2_fair_cpu {
                PlayerInputs::default()
            } else {
                p2
            };
            // A show feeds P1 the move's input in place of the pad's, the
            // way a pad's would reach the machine; its end gives the pad back.
            let in_fight = self.last_flow == FIGHT_LOOP;
            let p1 = match self.show.as_mut().map(|show| {
                let facing = p1_facing_right(&self.machine);
                show.input(&self.read.fighters[0], &self.read.fighters[1], facing, p1, in_fight)
            }) {
                Some(Some(fed)) => fed,
                Some(None) => {
                    // A show that ran its course goes on as a Try of the same move.
                    let finished = self.show.take().filter(Show::completed);
                    if finished.is_none() {
                        self.end_super_practice();
                    }
                    self.coach = finished.map(|show| Coach::new(*show.entry()));
                    p1
                }
                None => p1,
            };
            let (coin, start1) = if self.setup.kind == FightKind::Arcade {
                self.continue_switches(start)
            } else {
                (false, false)
            };
            // The coach is told what the player gave, and which way P1 faces.
            if let Some(coach) = &mut self.coach {
                coach.input(p1, p1_facing_right(&self.machine));
            }
            if self.practice_super.is_some() && self.read.fighters[0].move_id != crate::data_bank::SF2_SUPER_SLOT {
                self.machine.training_super_meter(None);
            }
            let m = &mut self.machine;
            let [r, l, d, u] = p1.directions;
            m.set_p1_direction(r, l, d, u);
            m.set_p1_buttons(p1.buttons);
            let [r, l, d, u] = p2.directions;
            m.set_p2_direction(r, l, d, u);
            m.set_p2_buttons(p2.buttons);
            m.set_cabinet_inputs(coin, false, start1, false, false);
            m.step();
            for (side, fighter) in Side::BOTH.into_iter().zip(m.take_era_super_events()) {
                if let Some(fighter) = fighter {
                    let name = crate::era::super_label(fighter);
                    self.pending_callouts.push(CalloutEvent::new(side, CalloutKind::Super { name }));
                }
            }
            let flow = flow_position(m);
            if self.country.is_none() && flow == STAGE_START {
                self.country = Some(chosen_country(m));
            }
            if flow == ROUND_START && self.last_flow != ROUND_START {
                self.round_start = Some(m.save_snapshot());
                self.moments.new_round();
            }
            self.last_flow = flow;
            let addresses = m.unrecovered();
            if !addresses.is_empty() {
                self.over = true;
                event = Some(FightEvent::Untranslated {
                    frame: m.state.frame,
                    addresses,
                });
            } else if self.setup.kind.stops_at_match_end() && m.match_over() {
                self.over = true;
                event = Some(FightEvent::MatchOver);
            } else if m.work_ram()[(MODE_DEMO - 0xff8000) as usize] == 0 {
                self.over = true;
                event = Some(FightEvent::ReturnedToAttract);
            }
            // The fight reader reads the stepped frame; the counts it feeds
            // show in every overhaul fight, with their damage in Training.
            // In Training it reads the gate's hit record too, so hits the
            // health floor cut short count in full.
            let m = &self.machine;
            let training = (m.match_config().roster.mode == PlayMode::Training).then(|| m.training());
            let combo = m.match_config().toggles.true_combos.then(|| m.toggle_state().combo);
            let read = self.reader.read_with_combos(m.work_ram(), &m.state.objectram, m.data_bank(), training, combo.as_ref());
            self.read = self.fed_reads.pop_front().unwrap_or(read);
            // The coach reads the player's fighter and the move's recognizer
            // on the stepped frame, and a move that came out is called.
            if let Some(coach) = &mut self.coach {
                let machine = MachineRead { rom: self.machine.program_rom(), work: self.machine.work_ram(), obj: &self.machine.state.objectram, bank: self.machine.data_bank() };
                let toggles = &self.setup.config.toggles;
                let view = coach.entry().live_recognizer(machine, P1_STRUCT, toggles, &self.machine.era_fighters()[0]);
                // The recovery input buffer can bring the move out a few
                // frames after the tracker is gone: a miss is held that long.
                let settle = if toggles.recovery_input_buffer { u32::from(toggles.recovery_buffer_window) + 1 } else { 0 };
                if let Some(Outcome::Success) = coach.observe(&view, &self.read, flow == FIGHT_LOOP, settle) {
                    let word = coach.entry().call_out;
                    self.pending_callouts.push(CalloutEvent::new(Side::P1, CalloutKind::Move { word }));
                }
                // After a miss, the fix, worded in the chosen notation.
                if let Some(fix) = coach.take_fix() {
                    let text = fix.text(self.notation);
                    self.pending_callouts.push(CalloutEvent::new(Side::P1, CalloutKind::fix(&text)));
                }
            }
            // A finished combo of five hits and up is the announcer's too.
            for hits in self.combo.step(&self.read, self.hold_combo && self.training()).into_iter().flatten() {
                self.pending_combos.extend(AnnouncerLine::for_combo(hits));
            }
            // FIRST ATTACK, REVERSAL and PERFECT arrive with this frame, as pushed
            // call-outs do, so the announcer calls them too; a CPU side
            // earns them as a human one. The 1987 preset shows none.
            // PERFECT shows on the knockout frame: the ROM's round-end text
            // (the result at y 64..72, the bonus block at y 56..120) stays
            // within x 104..256, clear of PERFECT's box at either screen
            // edge (x 8..64 or 320..376), so it never waits for the text.
            let moments = self.moments.step(&self.read);
            if self.setup.config.preset != RulesetPreset::Faithful1987 {
                self.pending_callouts.extend(moments);
            }
        }
        let overhaul = self.setup.config.preset != RulesetPreset::Faithful1987;
        for side in Side::BOTH {
            let shown = self.combo.shown(side, self.training()).filter(|_| overhaul);
            self.callouts.set_combo(side, shown);
        }
        let events = std::mem::take(&mut self.pending_callouts);
        // A Training Try's strip has a second line, so its call-out columns sit
        // lower; every other fight keeps the era HUD's layout.
        self.callouts.set_drop(if self.coach.is_some() { regions::TRY_DROP } else { 0 });
        self.callouts.step(&events);
        // The announcer says the word on screen (never a move's name), then
        // any finished combo's count; the 1987 preset has no announcer.
        let mut announcements: Vec<AnnouncerLine> =
            events.iter().filter_map(|e| AnnouncerLine::for_callout(e.kind)).collect();
        announcements.append(&mut self.pending_combos);
        if self.setup.config.preset == RulesetPreset::Faithful1987 {
            announcements.clear();
        }
        let sound_latch = match self.sounds.pop_front() {
            Some(cmd) => cmd,
            None => self.machine.state.sound_latch,
        };
        FightFrame {
            frame: self.render(assets),
            sound_latch,
            latch_cycles: self.machine.state.latch_cycles,
            announcements,
            event,
        }
    }
}
