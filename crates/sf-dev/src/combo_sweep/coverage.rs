//! Coverage sweeps with built-in and authored SF1+ inputs.
//!
//! The menu/setup and every saved replay use GameSession. Search attempts
//! branch from its settled native machine and drive the same logical pads
//! without rendering every discarded attempt.

use super::{
    crc32, drive_local, flow_position, load_roms, measure_local, presenter, Build, GameSession,
    Hit, Input, MatchConfig, Result, Setup, SetupDrive, Side, Stance, TournamentFighter, DOWN,
    FIGHT_LOOP, LEFT, P1_STRUCT, P2_STRUCT, PRESET, RIGHT, UP, WORK_BASE,
};
use serde::{Deserialize, Serialize};
use sf_engine::data_bank::{in_bank, special_slot, FIGHTER_SPAN, SPECIAL_SLOTS};
use sf_engine::fight_reader::{FightRead, FightReader};
use sf_engine::moves::{move_list, MoveEntry, MoveKind, Strength};
use sf_engine::run::PowerOnMachine;
use sf_host::art::{RECIPES, REVIEWS};
use sf_video::art::{load_art_from, ArtSources};
use sf_video::art_pack::{ArtPack, MANIFEST as ART_MANIFEST, PACK_FOLDER};
use sf_video::render::Frame;
use std::{fs, path::Path};

pub(super) const FORMAT_VERSION: u32 = 2;
const FRAMES_AFTER_OPENER: usize = 90;
const BUTTON_NAMES: [&str; 6] = ["LP", "MP", "HP", "LK", "MK", "HK"];
const STRENGTHS: [Strength; 3] = [Strength::Light, Strength::Medium, Strength::Heavy];
const JUMPS: [u16; 3] = [UP, UP | RIGHT, UP | LEFT];
const P1_TABLE: usize = (P1_STRUCT + 0x12 - WORK_BASE) as usize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct AssetId {
    rom: Build,
    player_table: TableId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct TableId {
    source: String,
    address: String,
    bank_crc32: Option<String>,
    art_manifest_crc32: Option<String>,
}

struct Fixture {
    assets: AssetId,
    address: u32,
    session: GameSession,
}

fn load(fighter: TournamentFighter) -> Result<Fixture> {
    let (rom, assets) = load_roms()?;
    let shoto = matches!(fighter, TournamentFighter::Ryu | TournamentFighter::Ken);
    let mut session = GameSession::new(assets, MatchConfig::from_preset(PRESET));
    let (address, bank_crc32, art_manifest_crc32) = if shoto {
        (fighter.move_table(), None, None)
    } else {
        let rom_folder = crate::resolve_local(crate::ROMS);
        let local = rom_folder.with_file_name(PACK_FOLDER);
        let folder = if local.is_dir() {
            local
        } else {
            crate::resolve_local("../../assets").join(PACK_FOLDER)
        };
        let pack = ArtPack::open(&folder).map_err(|err| {
            format!(
                "{} needs its SF1+ art and player table. The current {PACK_FOLDER} is expected at {}. {err}",
                fighter.name(), folder.display(),
            )
        })?;
        let art = load_art_from(
            session.assets(),
            &ArtSources {
                recipes: &RECIPES,
                reviews: Some(&REVIEWS),
                pack: Ok(&pack),
            },
        )
        .map_err(|err| format!("load {PACK_FOLDER}: {err}"))?;
        session = session
            .with_sprite_bank(&art.bank)
            .with_player_tables(&art.catalogues);
        let bank = session.data_bank().ok_or("player data bank did not load")?;
        let address = bank.table(fighter).ok_or_else(|| {
            let reason = bank.notes().iter().filter(|note| {
                note.to_ascii_lowercase().contains(&fighter.name().to_ascii_lowercase())
            }).cloned().collect::<Vec<_>>().join(" ");
            format!(
                "{} has no playable SF1+ table. Install the latest {PACK_FOLDER} with all approved poses. {reason}",
                fighter.name(),
            )
        })?;
        let bytes: Vec<_> = (address..address + FIGHTER_SPAN)
            .map(|a| bank.rb(a))
            .collect();
        let art_manifest = fs::read(folder.join(ART_MANIFEST))
            .map_err(|err| format!("read {PACK_FOLDER} manifest: {err}"))?;
        (
            address,
            Some(format!("{:08x}", crc32(&bytes))),
            Some(format!("{:08x}", crc32(&art_manifest))),
        )
    };
    let player_table = TableId {
        source: if shoto {
            "ROM".into()
        } else {
            "data bank".into()
        },
        address: format!("{address:#08x}"),
        bank_crc32,
        art_manifest_crc32,
    };
    Ok(Fixture {
        assets: AssetId { rom, player_table },
        address,
        session,
    })
}

fn check_setup(
    session: &GameSession,
    fighter: TournamentFighter,
    stance: Stance,
    address: u32,
) -> Result<Setup> {
    let setup = measure_local(session, fighter, stance)?;
    let machine = session
        .fight_machine()
        .ok_or("expected native fight machine")?;
    let actual = long(machine.work_ram(), P1_TABLE);
    if actual != address {
        return Err(format!(
            "{} used player table {actual:#08x}, expected {address:#08x}",
            fighter.name()
        ));
    }
    Ok(setup)
}

fn long(bytes: &[u8], at: usize) -> u32 {
    u32::from_be_bytes(bytes[at..at + 4].try_into().expect("native longword"))
}

#[derive(Clone, Copy)]
struct Normal {
    posture: &'static str,
    direction: u16,
    button: usize,
    moves: &'static [u8],
}

impl Normal {
    fn name(self) -> String {
        format!("{} {}", self.posture, BUTTON_NAMES[self.button])
    }

    fn bit(self) -> u16 {
        1 << (4 + self.button)
    }

    fn prefix(self, press: usize) -> Vec<u16> {
        let mut inputs = vec![0; press + 1];
        if self.direction & UP != 0 {
            inputs[..2].fill(self.direction);
        } else {
            inputs[press] = self.direction;
        }
        inputs[press] |= self.bit();
        inputs
    }
}

fn normals() -> Vec<Normal> {
    // The translated posture dispatch has distinct diagonal crouch punches
    // and shares its kick slots across crouching directions.
    let postures: &[(&str, u16, &[&[u8]])] = &[
        (
            "standing",
            0,
            &[&[0x0d, 0x0e], &[0x0f], &[0x10], &[0x14], &[0x15], &[0x16]],
        ),
        (
            "crouching",
            DOWN,
            &[&[0x11], &[0x1c], &[0x1c], &[0x17], &[0x17], &[0x18]],
        ),
        (
            "jumping",
            UP,
            &[&[0x20], &[0x20], &[0x20], &[0x23], &[0x23], &[0x26]],
        ),
        (
            "forward jump",
            UP | RIGHT,
            &[&[0x21], &[0x21], &[0x21], &[0x24], &[0x24], &[0x24]],
        ),
        (
            "back jump",
            UP | LEFT,
            &[&[0x22], &[0x22], &[0x22], &[0x25], &[0x25], &[0x25]],
        ),
        ("down-forward", DOWN | RIGHT, &[&[0x12], &[0x1d], &[0x1d]]),
        ("down-back", DOWN | LEFT, &[&[0x13], &[0x1e], &[0x1e]]),
    ];
    postures
        .iter()
        .flat_map(|&(posture, direction, moves)| {
            moves
                .iter()
                .enumerate()
                .map(move |(button, &moves)| Normal {
                    posture,
                    direction,
                    button,
                    moves,
                })
        })
        .collect()
}

#[derive(Clone)]
struct Command {
    frames: Vec<u16>,
    button: u16,
    hold: usize,
}

impl Command {
    fn from_move(entry: &MoveEntry, strength: Strength) -> Result<(Self, Vec<u16>)> {
        let frames: Vec<_> = entry
            .input
            .frames(true, strength)
            .into_iter()
            .map(|pad| {
                pad.directions
                    .into_iter()
                    .chain(pad.buttons)
                    .enumerate()
                    .filter(|(_, held)| *held)
                    .fold(0, |bits, (index, _)| bits | (1 << index))
            })
            .collect();
        let button = frames
            .iter()
            .map(|bits| bits & 0x3f0)
            .find(|&bits| bits != 0)
            .ok_or("built-in command has no activation button")?;
        let hold = frames.iter().filter(|&&bits| bits & button != 0).count();
        let command = Self {
            frames: frames.iter().map(|bits| bits & 0xf).collect(),
            button,
            hold,
        };
        Ok((command, frames))
    }

    fn fixed_frames(&self) -> impl Iterator<Item = u16> + '_ {
        let final_direction = self.frames.last().copied().unwrap_or(0) & 0xf;
        self.frames
            .iter()
            .copied()
            .chain(std::iter::repeat(final_direction))
            .take(FRAMES_AFTER_OPENER)
    }

    fn recipe(&self, opener: &[u16], stance: Stance) -> Vec<Input> {
        opener
            .iter()
            .copied()
            .chain(self.fixed_frames())
            .map(|p1| Input {
                p1,
                p2: stance.pad(),
                start: false,
            })
            .collect()
    }

    fn with_press(&self, fixed: &[Input], at: usize) -> Vec<Input> {
        let mut inputs = fixed.to_vec();
        for input in inputs.iter_mut().skip(at).take(self.hold) {
            input.p1 |= self.button;
        }
        inputs
    }
}

struct NativeProbe {
    machine: PowerOnMachine,
    reader: FightReader,
}

impl NativeProbe {
    fn new(machine: &PowerOnMachine) -> Self {
        let mut probe = Self {
            machine: machine.clone(),
            reader: FightReader::new(),
        };
        probe.read();
        probe
    }

    fn read(&mut self) -> FightRead {
        let m = &self.machine;
        self.reader.read_with_combos(
            m.work_ram(),
            &m.state.objectram,
            m.data_bank(),
            None,
            Some(&m.toggle_state().combo),
        )
    }

    fn step(&mut self, input: Input) -> FightRead {
        let frame = input.frame();
        let [r, l, d, u] = frame.p1.directions;
        self.machine.set_p1_direction(r, l, d, u);
        self.machine.set_p1_buttons(frame.p1.buttons);
        let [r, l, d, u] = frame.p2.directions;
        self.machine.set_p2_direction(r, l, d, u);
        self.machine.set_p2_buttons(frame.p2.buttons);
        self.machine
            .set_cabinet_inputs(false, false, false, false, false);
        self.machine.step();
        self.read()
    }
}

#[derive(Default)]
struct Trace {
    previous_move: u8,
    opener: Option<u8>,
    special_start: Option<usize>,
    special_strength: Option<Strength>,
    other_attack_start: Option<usize>,
    cancelled: bool,
    stun_end: Option<usize>,
    hits: Vec<Hit>,
}

impl Trace {
    fn observe(
        &mut self,
        frame: usize,
        read: &FightRead,
        m: &PowerOnMachine,
        opener: &[u8],
        special: u8,
    ) {
        let current_move = read.fighter(Side::P1).move_id;
        let is_opener = opener.contains(&current_move);
        if current_move != special
            && current_move != self.previous_move
            && (SPECIAL_SLOTS.contains(&current_move) || (is_opener && self.opener.is_some()))
        {
            self.other_attack_start.get_or_insert(frame);
        }
        if is_opener {
            self.opener.get_or_insert(current_move);
        }
        if current_move == special && self.special_start.is_none() {
            self.special_start = Some(frame);
            let latched = m.diagnostic_telemetry().p1.punch_latch.buffered_strength;
            let level = sf_engine::combo::level(latched);
            self.special_strength = Some(STRENGTHS[usize::from(level - 1)]);
            self.cancelled = m.toggle_state().cancel.count[0] > 0;
        }
        if let Some(hit) = read.landed_by(Side::P1) {
            let projectile = self.special_start.is_some() && projectile_impact(m, special);
            let move_id = if projectile {
                // Confirmation rules out other specials before this impact.
                special
            } else if read.fighter(Side::P1).attacking() {
                current_move
            } else {
                // Some move streams end on their last impact frame.
                self.previous_move
            };
            self.hits.push(Hit {
                frame,
                move_id,
                projectile,
                damage: hit.damage,
                chip: hit.chip,
                into_hitstun: hit.into_hitstun,
            });
        }
        // Once the first opener's stun ends, later fixed attacks cannot
        // create more eligible activation frames for that opener.
        if m.toggle_state().combo.hitstun[1] > 0
            && self.other_attack_start.is_none()
            && self.hits.first().is_some_and(|hit| {
                opener.contains(&hit.move_id)
                    && !hit.chip
                    && !hit.projectile
                    && frame == self.stun_end.map_or(hit.frame, |end| end + 1)
            })
        {
            self.stun_end = Some(frame);
        }
        self.previous_move = current_move;
    }

    fn confirmed(&self, opener: u8, special: u8) -> Option<Confirmed> {
        let special_start = self.special_start?;
        let first_special = self.hits.iter().position(|hit| hit.move_id == special)?;
        let (opener_hits, follow_up) = self.hits.split_at(first_special);
        let hit = follow_up[0];
        if opener_hits.is_empty()
            || !opener_hits
                .iter()
                .all(|hit| hit.move_id == opener && !hit.projectile && !hit.chip)
            || opener_hits.iter().skip(1).any(|hit| !hit.into_hitstun)
            || hit.chip
            || !hit.into_hitstun
            || hit.frame < special_start
            || self
                .other_attack_start
                .is_some_and(|frame| frame <= hit.frame)
        {
            return None;
        }
        Some(Confirmed {
            opener_hits: opener_hits.to_vec(),
            follow_up_hit: hit,
            special_start,
            transition: if self.cancelled {
                TransitionKind::Cancel
            } else {
                TransitionKind::Link
            },
        })
    }
}

fn projectile_impact(machine: &PowerOnMachine, special: u8) -> bool {
    let telemetry = machine.diagnostic_telemetry();
    if telemetry.p2.projectile_impacted_this_frame
        && telemetry
            .projectiles
            .iter()
            .any(|effect| effect.owner_struct == P1_STRUCT && effect.target_struct == P2_STRUCT)
    {
        return true;
    }

    // Bank projectiles can transform into their own impact streams, such as
    // Retsu's fading Kiai Shot. The native collision preserves owner/target,
    // installs the non-colliding stream and sets its first-frame state to 1.
    // Read it on the clean damage frame, even if the shot spawned that frame
    // or its thrower has since recovered. Ken does not throw or attack.
    let table = long(machine.work_ram(), P1_TABLE);
    let Some(bank) = machine.data_bank().filter(|_| in_bank(table)) else {
        return false;
    };
    if special_slot(|address| bank.rb(address), table, special).melee {
        return false;
    }
    machine.state.objectram[0x100..0x400]
        .chunks_exact(0x40)
        .any(|slot| {
            slot[8] == 1
                && slot[0x0e] & 0xe0 == 0
                && in_bank(long(slot, 0x10))
                && long(slot, 0x20) == P1_STRUCT
                && long(slot, 0x24) == P2_STRUCT
        })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TransitionKind {
    Cancel,
    Link,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
struct Confirmed {
    opener_hits: Vec<Hit>,
    follow_up_hit: Hit,
    special_start: usize,
    transition: TransitionKind,
}

fn probe(start: &PowerOnMachine, inputs: &[Input], opener: &[u8], special: u8) -> Trace {
    let mut probe = NativeProbe::new(start);
    let mut trace = Trace::default();
    for (frame, &input) in inputs.iter().enumerate() {
        let read = probe.step(input);
        trace.observe(frame, &read, &probe.machine, opener, special);
        if flow_position(&probe.machine) != FIGHT_LOOP {
            break;
        }
    }
    trace
}

fn descent_frames(start: &PowerOnMachine, stance: Stance, direction: u16) -> Vec<usize> {
    let mut probe = NativeProbe::new(start);
    let mut previous_y = start.diagnostic_telemetry().p1.center_y;
    let mut descending = Vec::new();
    for frame in 0..80 {
        let fighter = probe.machine.diagnostic_telemetry().p1;
        if (9..=0xb).contains(&fighter.move_id) {
            if fighter.center_y > previous_y {
                descending.push(frame);
            }
        } else if !descending.is_empty() {
            break;
        }
        previous_y = fighter.center_y;
        probe.step(Input {
            p1: if frame < 2 { direction } else { 0 },
            p2: stance.pad(),
            start: false,
        });
    }
    descending
}

fn opener_prefix(
    start: &PowerOnMachine,
    normal: Normal,
    stance: Stance,
    descent: &[usize],
) -> Option<Vec<u16>> {
    if normal.direction & UP == 0 {
        return Some(normal.prefix(0));
    }
    let mut executable = None;
    for &press in descent.iter().rev() {
        let prefix = normal.prefix(press);
        let input = prefix
            .iter()
            .copied()
            .chain(std::iter::repeat_n(0, 26))
            .map(|p1| Input {
                p1,
                p2: stance.pad(),
                start: false,
            })
            .collect::<Vec<_>>();
        let trace = probe(start, &input, normal.moves, 0xff);
        if trace
            .hits
            .iter()
            .any(|hit| !hit.chip && normal.moves.contains(&hit.move_id))
        {
            return Some(prefix);
        }
        if trace.opener.is_some() && executable.is_none() {
            executable = Some(prefix);
        }
    }
    executable
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthoredInputs {
    fixed: Vec<u16>,
    activation: Activation,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Activation {
    button: String,
    frame: usize,
    hold: usize,
}

impl AuthoredInputs {
    fn prepare(&self) -> Result<(Command, Vec<u16>)> {
        let activation = &self.activation;
        let index = BUTTON_NAMES
            .iter()
            .position(|name| name.eq_ignore_ascii_case(&activation.button))
            .ok_or("activation.button must be LP, MP, HP, LK, MK or HK")?;
        let button = 1 << (4 + index);
        let release = activation
            .frame
            .checked_add(activation.hold)
            .filter(|&frame| frame < FRAMES_AFTER_OPENER && activation.hold > 0)
            .ok_or("activation.frame and activation.hold must leave a release frame within the 90-frame command")?;
        if self.fixed.len() > FRAMES_AFTER_OPENER {
            return Err(format!(
                "inputs.fixed supports at most {FRAMES_AFTER_OPENER} frames"
            ));
        }
        for (frame, &bits) in self.fixed.iter().enumerate() {
            if bits > 0x3ff {
                return Err(format!("inputs.fixed[{frame}] must use pad bits 0-9"));
            }
            if bits & button != 0 {
                return Err(format!(
                    "inputs.fixed[{frame}] holds {}. Put the movable button only in activation.button",
                    activation.button
                ));
            }
        }

        let command = Command {
            frames: self.fixed.clone(),
            button,
            hold: activation.hold,
        };
        let mut standalone: Vec<_> = command.fixed_frames().collect();
        for bits in &mut standalone[activation.frame..release] {
            *bits |= button;
        }
        Ok((command, standalone))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthoredRequest {
    #[serde(rename = "move")]
    name: String,
    strength: String,
    inputs: AuthoredInputs,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Signature {
    name: String,
    input: String,
    move_id: u8,
    strength: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    authored: Option<AuthoredInputs>,
}

impl Signature {
    fn matches_strength(&self, observed: Option<Strength>) -> bool {
        self.authored.is_none()
            || observed.is_some_and(|strength| strength_name(strength) == self.strength)
    }
}

struct SweepCommand {
    signature: Signature,
    command: Command,
    standalone: Vec<u16>,
}

impl SweepCommand {
    fn new(
        entry: &MoveEntry,
        strength: Strength,
        authored: Option<AuthoredInputs>,
    ) -> Result<Self> {
        let (input, (command, standalone)) = match &authored {
            Some(inputs) => ("authored".into(), inputs.prepare()?),
            None => (entry.input.words(), Command::from_move(entry, strength)?),
        };
        Ok(Self {
            signature: Signature {
                name: entry.name.into(),
                input,
                move_id: entry.produces,
                strength: strength_name(strength).into(),
                authored,
            },
            command,
            standalone,
        })
    }
}

#[derive(Debug, Serialize)]
struct InputRun {
    frames: usize,
    p1: u16,
    p2: u16,
}

#[derive(Serialize)]
struct Recipe {
    fixed_inputs: Vec<InputRun>,
    movable_button: &'static str,
    held_frames: usize,
}

impl Recipe {
    fn new(fixed: &[Input], command: &Command) -> Self {
        let fixed_inputs = fixed
            .chunk_by(|a, b| a == b)
            .map(|run| InputRun {
                frames: run.len(),
                p1: run[0].p1,
                p2: run[0].p2,
            })
            .collect();
        let button = command.button.trailing_zeros() as usize - 4;
        Self {
            fixed_inputs,
            movable_button: BUTTON_NAMES[button],
            held_frames: command.hold,
        }
    }
}

#[derive(Serialize)]
struct TimingWindow {
    first: i32,
    last: i32,
    frames: usize,
}

#[derive(Serialize)]
struct Success {
    offset: i32,
    press_frame: usize,
    follow_up_hit_frame: usize,
    transition: TransitionKind,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum Outcome {
    Unsupported {
        reason: &'static str,
    },
    NoSuccess {
        attempts: usize,
        reason: &'static str,
    },
    Confirmed {
        attempts: usize,
        windows: Vec<TimingWindow>,
        widest_window_frames: usize,
        successes: Vec<Success>,
        replay: String,
    },
}

#[derive(Serialize)]
struct Pair {
    opener: String,
    opener_move_id: Option<u8>,
    signature: Signature,
    recipe: Option<Recipe>,
    #[serde(flatten)]
    outcome: Outcome,
}

#[derive(Serialize)]
struct StanceCoverage {
    setup: Setup,
    pairs: Vec<Pair>,
}

#[derive(Serialize)]
struct Coverage {
    assets: AssetId,
    unsupported_signatures: Vec<String>,
    stances: Vec<StanceCoverage>,
}

#[derive(Serialize, Deserialize)]
struct Replay {
    format_version: u32,
    assets: AssetId,
    setup: Setup,
    stance: Stance,
    opener: String,
    opener_move_id: u8,
    signature: Signature,
    expected: Confirmed,
    route_start: usize,
    opener_frame: usize,
    press_frame: usize,
    inputs: Vec<Input>,
}

fn slug(value: &str) -> String {
    value
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .join("-")
}

fn write_json(path: &Path, value: &impl Serialize, pretty: bool) -> Result<()> {
    let bytes = if pretty {
        serde_json::to_vec_pretty(value)
    } else {
        serde_json::to_vec(value)
    }
    .map_err(|err| err.to_string())?;
    fs::write(path, bytes).map_err(|err| format!("write {}: {err}", path.display()))
}

fn strength_name(strength: Strength) -> &'static str {
    match strength {
        Strength::Light => "light",
        Strength::Medium => "medium",
        Strength::Heavy => "heavy",
    }
}

pub(super) fn sweep(fighter: &str, folder: &Path, requested: &[String]) -> Result<()> {
    let fighter = TournamentFighter::from_name(fighter)?;
    let entries: Vec<_> = move_list(fighter, PRESET)
        .into_iter()
        .filter(|entry| matches!(entry.kind, MoveKind::Special | MoveKind::Super))
        .collect();
    let (selected, unsupported) = match requested {
        [flag, path] if flag == "--command" => {
            (vec![read_authored(Path::new(path), &entries)?], Vec::new())
        }
        _ if requested.iter().any(|arg| arg == "--command") => {
            return Err(
                "use --command <command.json> by itself or list built-in signature moves".into(),
            );
        }
        _ => select_built_in(fighter, &entries, requested)?,
    };

    let mut fixture = load(fighter)?;
    let setup_inputs = drive_local(&mut fixture.session, fighter)?;
    fs::create_dir_all(folder).map_err(|err| format!("create {}: {err}", folder.display()))?;
    println!(
        "Sweeping {} on {} table {}.",
        fighter.name(),
        fixture.assets.player_table.source,
        fixture.assets.player_table.address
    );

    let mut stances = Vec::new();
    for stance in [Stance::Standing, Stance::Crouching] {
        let mut session = fixture.session.clone();
        let mut setup_inputs = setup_inputs.clone();
        if stance == Stance::Crouching {
            let down = Input {
                p2: DOWN,
                ..Input::default()
            };
            let mut drive = SetupDrive {
                session: &mut session,
                inputs: setup_inputs,
            };
            for _ in 0..8 {
                drive.step(down);
            }
            drive.close_in(fighter, stance)?;
            setup_inputs = drive.inputs;
        }
        let setup = check_setup(&session, fighter, stance, fixture.address)?;
        println!("{} Ken at {} px.", stance.name(), setup.separation_px);
        let context = StanceSweep {
            folder,
            session: &session,
            inputs: &setup_inputs,
            assets: &fixture.assets,
            stance,
            setup: &setup,
        };
        let pairs = context.sweep(&selected)?;
        let wins = pairs
            .iter()
            .filter(|pair| matches!(pair.outcome, Outcome::Confirmed { .. }))
            .count();
        let unsupported = pairs
            .iter()
            .filter(|pair| matches!(pair.outcome, Outcome::Unsupported { .. }))
            .count();
        println!(
            "  {wins} verified routes, {} pairs with no window, {unsupported} unsupported.",
            pairs.len() - wins - unsupported
        );
        stances.push(StanceCoverage { setup, pairs });
    }
    let report = Coverage {
        assets: fixture.assets,
        unsupported_signatures: unsupported,
        stances,
    };
    let report_path = folder.join("coverage.json");
    write_json(&report_path, &report, true)?;
    println!("Saved {}", report_path.display());
    Ok(())
}

fn read_authored(path: &Path, entries: &[&MoveEntry]) -> Result<SweepCommand> {
    let read = || -> Result<_> {
        let json = fs::read(path).map_err(|err| err.to_string())?;
        let request: AuthoredRequest =
            serde_json::from_slice(&json).map_err(|err| err.to_string())?;
        let entry = entries
            .iter()
            .copied()
            .find(|entry| slug(entry.name) == slug(&request.name))
            .ok_or_else(|| {
                format!(
                    "unknown signature move {:?}. Available moves: {}",
                    request.name,
                    entries
                        .iter()
                        .map(|entry| entry.name)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;
        let strength = STRENGTHS
            .into_iter()
            .find(|&strength| strength_name(strength).eq_ignore_ascii_case(&request.strength))
            .ok_or("strength must be light, medium or heavy")?;
        SweepCommand::new(entry, strength, Some(request.inputs))
    };
    read().map_err(|err| format!("authored command {}: {err}", path.display()))
}

fn select_built_in(
    fighter: TournamentFighter,
    entries: &[&MoveEntry],
    requested: &[String],
) -> Result<(Vec<SweepCommand>, Vec<String>)> {
    let mut selected = Vec::new();
    let mut unsupported = Vec::new();
    for name in requested {
        match entries
            .iter()
            .copied()
            .find(|entry| slug(entry.name) == slug(name))
        {
            Some(entry) if !selected.contains(&entry) => selected.push(entry),
            Some(_) => {}
            None => unsupported.push(name.clone()),
        }
    }
    if !unsupported.is_empty() {
        let available = entries
            .iter()
            .map(|entry| entry.name)
            .collect::<Vec<_>>()
            .join(", ");
        println!(
            "Unsupported signatures: {}. Available for {}: {available}.",
            unsupported.join(", "),
            fighter.name()
        );
    }
    if selected.is_empty() {
        return Err("select at least one supported built-in signature move".into());
    }
    let commands = selected
        .into_iter()
        .flat_map(|entry| STRENGTHS.map(|strength| SweepCommand::new(entry, strength, None)))
        .collect::<Result<_>>()?;
    Ok((commands, unsupported))
}

struct StanceSweep<'a> {
    folder: &'a Path,
    session: &'a GameSession,
    inputs: &'a [Input],
    assets: &'a AssetId,
    stance: Stance,
    setup: &'a Setup,
}

impl StanceSweep<'_> {
    fn sweep(&self, commands: &[SweepCommand]) -> Result<Vec<Pair>> {
        let start = self
            .session
            .fight_machine()
            .ok_or("expected native fight machine")?;
        let descents = JUMPS.map(|direction| descent_frames(start, self.stance, direction));
        let openers: Vec<_> = normals()
            .into_iter()
            .map(|normal| {
                let descent = JUMPS
                    .iter()
                    .position(|&direction| direction == normal.direction)
                    .map_or(&[][..], |index| descents[index].as_slice());
                (normal, opener_prefix(start, normal, self.stance, descent))
            })
            .collect();
        let mut pairs = Vec::new();
        for SweepCommand {
            signature,
            command,
            standalone,
        } in commands
        {
            let standalone: Vec<_> = standalone
                .iter()
                .map(|&p1| Input {
                    p1,
                    p2: self.stance.pad(),
                    start: false,
                })
                .collect();
            let observed = probe(start, &standalone, &[], signature.move_id);
            let supported = observed.special_start.is_some();
            let unsupported = if signature.authored.is_some() {
                let fixed = command.recipe(&[], self.stance);
                if probe(start, &fixed, &[], signature.move_id)
                    .special_start
                    .is_some()
                {
                    Some("the fixed inputs start the intended move without the movable button")
                } else if !supported {
                    Some("the authored command did not start the intended move. Check inputs.fixed and activation.frame")
                } else if !signature.matches_strength(observed.special_strength) {
                    Some("the authored command started a different strength. Check strength and the activation button")
                } else {
                    None
                }
            } else {
                (!supported)
                    .then_some("the standalone built-in command did not start this strength")
            };
            if let Some(reason) = unsupported.filter(|_| signature.authored.is_some()) {
                println!("  {} {}: {reason}.", signature.strength, signature.name);
            }
            for (normal, prefix) in &openers {
                let pair = if let (None, Some(prefix)) = (unsupported, prefix) {
                    self.pair(start, *normal, prefix, command, signature.clone())?
                } else {
                    Pair {
                        opener: normal.name(),
                        opener_move_id: None,
                        signature: signature.clone(),
                        recipe: prefix.as_ref().map(|prefix| {
                            Recipe::new(&command.recipe(prefix, self.stance), command)
                        }),
                        outcome: Outcome::Unsupported {
                            reason: unsupported
                                .unwrap_or("jump normal did not start during the measured descent"),
                        },
                    }
                };
                pairs.push(pair);
            }
        }
        Ok(pairs)
    }

    fn pair(
        &self,
        start: &PowerOnMachine,
        normal: Normal,
        prefix: &[u16],
        command: &Command,
        signature: Signature,
    ) -> Result<Pair> {
        let fixed = command.recipe(prefix, self.stance);
        let baseline = probe(start, &fixed, normal.moves, signature.move_id);
        let mut pair = Pair {
            opener: normal.name(),
            opener_move_id: baseline.opener,
            signature,
            recipe: Some(Recipe::new(&fixed, command)),
            outcome: Outcome::Unsupported {
                reason: "the intended normal did not start",
            },
        };
        let Some(opener) = baseline.opener else {
            return Ok(pair);
        };
        if pair.signature.authored.is_some() && baseline.special_start.is_some() {
            pair.outcome = Outcome::Unsupported {
                reason:
                    "the opener and fixed inputs start the intended move without the movable button",
            };
            return Ok(pair);
        }
        let first_press = prefix.len();
        let last_full_press = fixed.len().saturating_sub(command.hold + 1);
        let Some(last_press) = baseline
            .stun_end
            .map(|end| end.min(last_full_press))
            .filter(|&end| {
                first_press <= end
                    && baseline
                        .hits
                        .iter()
                        .any(|hit| hit.move_id == opener && !hit.chip && !hit.projectile)
            })
        else {
            pair.outcome = Outcome::NoSuccess {
                attempts: 0,
                reason: "the opener had no clean hitstun window in this setup",
            };
            return Ok(pair);
        };

        let mut found = Vec::new();
        for press in first_press..=last_press {
            let input = command.with_press(&fixed, press);
            let trace = probe(start, &input, &[opener], pair.signature.move_id);
            if let Some(confirmed) =
                trace
                    .confirmed(opener, pair.signature.move_id)
                    .filter(|result| {
                        result.special_start >= press
                            && pair.signature.matches_strength(trace.special_strength)
                    })
            {
                let offset = press as i32 - confirmed.opener_hits[0].frame as i32;
                found.push((offset, press, confirmed));
            }
        }
        let attempts = last_press - first_press + 1;
        if found.is_empty() {
            pair.outcome = Outcome::NoSuccess {
                attempts,
                reason: "no signature hit landed during the opener's hitstun",
            };
            return Ok(pair);
        }
        found.sort_unstable_by_key(|&(offset, press, _)| (offset, press));
        let offsets: Vec<_> = found.iter().map(|&(offset, _, _)| offset).collect();
        let mut unique = offsets;
        unique.dedup();
        let windows: Vec<_> = unique
            .chunk_by(|a, b| *b == *a + 1)
            .map(|window| TimingWindow {
                first: window[0],
                last: window[window.len() - 1],
                frames: window.len(),
            })
            .collect();
        let widest = windows
            .iter()
            .max_by_key(|window| (window.frames, std::cmp::Reverse(window.first)))
            .unwrap();
        let middle = widest.first + (widest.frames as i32 - 1) / 2;
        let sample = found
            .iter()
            .position(|&(offset, _, _)| offset == middle)
            .unwrap();
        let successes = found
            .iter()
            .map(|(offset, press, result)| Success {
                offset: *offset,
                press_frame: *press,
                follow_up_hit_frame: result.follow_up_hit.frame,
                transition: result.transition,
            })
            .collect();
        let (_, press, expected) = found.swap_remove(sample);
        let filename = format!(
            "{}-{}-{}-{}.json",
            self.stance.name(),
            slug(&pair.opener),
            slug(&pair.signature.name),
            pair.signature.strength
        );
        let route = command.with_press(&fixed, press);
        let replay = Replay {
            format_version: FORMAT_VERSION,
            assets: self.assets.clone(),
            setup: self.setup.clone(),
            stance: self.stance,
            opener: pair.opener.clone(),
            opener_move_id: opener,
            signature: pair.signature.clone(),
            expected,
            route_start: self.inputs.len(),
            opener_frame: prefix.len() - 1,
            press_frame: press,
            inputs: self.inputs.iter().chain(&route).copied().collect(),
        };
        let mut check = self.session.clone();
        verify_route(&mut check, &route, &replay, |_| true)?;
        write_json(&self.folder.join(&filename), &replay, false)?;
        pair.outcome = Outcome::Confirmed {
            attempts,
            widest_window_frames: widest.frames,
            windows,
            successes,
            replay: filename,
        };
        Ok(pair)
    }
}

fn verify_route(
    session: &mut GameSession,
    inputs: &[Input],
    replay: &Replay,
    mut show: impl FnMut(&Frame) -> bool,
) -> Result<bool> {
    let mut trace = Trace::default();
    for (frame, &input) in inputs.iter().enumerate() {
        let out = session.step(input.frame());
        let fight = session.fight().ok_or("fight ended during the route")?;
        trace.observe(
            frame,
            fight.fight_read(),
            fight.machine(),
            &[replay.opener_move_id],
            replay.signature.move_id,
        );
        if !show(&out.frame) {
            return Ok(false);
        }
    }
    let actual = trace
        .confirmed(replay.opener_move_id, replay.signature.move_id)
        .filter(|result| {
            result.special_start >= replay.press_frame
                && replay.signature.matches_strength(trace.special_strength)
        });
    if actual.as_ref() != Some(&replay.expected) {
        return Err(format!(
            "hit mismatch for {} into {} {}: expected {:?}, got {actual:?} (hits {:?})",
            replay.opener,
            replay.signature.strength,
            replay.signature.name,
            replay.expected,
            trace.hits
        ));
    }
    Ok(true)
}

pub(super) fn replay(json: &[u8], watch: bool) -> Result<()> {
    let replay: Replay =
        serde_json::from_slice(json).map_err(|err| format!("read coverage replay: {err}"))?;
    let fighter = TournamentFighter::from_name(&replay.setup.p1)?;
    let mut fixture = load(fighter)?;
    if replay.assets != fixture.assets {
        return Err(format!(
            "build or fight assets differ from the replay: expected {:?}, got {:?}",
            replay.assets, fixture.assets
        ));
    }
    let (setup_inputs, route) = replay
        .inputs
        .split_at_checked(replay.route_start)
        .ok_or("route start is outside the recorded input stream")?;
    verify_recipe(fighter, &replay, route)?;
    let mut show = presenter(watch, fighter.name());
    for &input in setup_inputs {
        if !show(&fixture.session.step(input.frame()).frame) {
            return Ok(());
        }
    }
    let actual = check_setup(&fixture.session, fighter, replay.stance, fixture.address)?;
    if actual != replay.setup {
        return Err(format!(
            "setup mismatch: expected {:?}, got {actual:?}",
            replay.setup
        ));
    }
    if verify_route(&mut fixture.session, route, &replay, show)? {
        println!(
            "Verified {} at route frame {} into {} {} at frame {} ({:?}) against {} Ken.",
            replay.opener,
            replay.expected.opener_hits[0].frame,
            replay.signature.strength,
            replay.signature.name,
            replay.expected.follow_up_hit.frame,
            replay.expected.transition,
            replay.stance.name()
        );
    }
    Ok(())
}

fn verify_recipe(fighter: TournamentFighter, replay: &Replay, route: &[Input]) -> Result<()> {
    let normal = normals()
        .into_iter()
        .find(|normal| {
            normal.name() == replay.opener && normal.moves.contains(&replay.opener_move_id)
        })
        .ok_or("replay has an unknown opener")?;
    let signature = move_list(fighter, PRESET)
        .into_iter()
        .find(|entry| {
            entry.name == replay.signature.name
                && entry.produces == replay.signature.move_id
                && matches!(entry.kind, MoveKind::Special | MoveKind::Super)
        })
        .ok_or("replay has an unknown signature")?;
    let strength = STRENGTHS
        .into_iter()
        .find(|&strength| strength_name(strength) == replay.signature.strength)
        .ok_or("replay has an unknown signature strength")?;
    let is_jump = normal.direction & UP != 0;
    if replay.opener_frame >= replay.press_frame
        || replay.press_frame >= route.len()
        || (is_jump && replay.opener_frame < 2)
        || (!is_jump && replay.opener_frame != 0)
    {
        return Err("recorded route has invalid activation timing".into());
    }
    let tested = SweepCommand::new(signature, strength, replay.signature.authored.clone())?;
    if tested.signature.input != replay.signature.input {
        return Err("replay has an unknown signature input".into());
    }
    let command = tested.command;
    let fixed = command.recipe(&normal.prefix(replay.opener_frame), replay.stance);
    if replay.press_frame.saturating_add(command.hold) >= fixed.len()
        || route != command.with_press(&fixed, replay.press_frame)
    {
        return Err("recorded route inputs differ from the stated command sequence".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multi_hit_opener_needs_a_distinct_signature_hit_in_stun() {
        let opener = 0x17;
        let signature = 0x1a;
        let first = Hit {
            frame: 6,
            move_id: opener,
            projectile: false,
            damage: 4,
            chip: false,
            into_hitstun: false,
        };
        let second = Hit {
            frame: 9,
            into_hitstun: true,
            ..first
        };
        let projectile = Hit {
            frame: 19,
            move_id: signature,
            projectile: true,
            ..second
        };
        let mut trace = Trace {
            special_start: Some(10),
            cancelled: true,
            hits: vec![first, second],
            ..Trace::default()
        };
        assert!(trace.confirmed(opener, signature).is_none());
        trace.hits.push(Hit {
            into_hitstun: false,
            ..projectile
        });
        assert!(trace.confirmed(opener, signature).is_none());
        trace.hits[2] = projectile;
        let combo = trace
            .confirmed(opener, signature)
            .expect("projectile hit in opener stun");
        assert_eq!(combo.opener_hits, [first, second]);
        assert_eq!(combo.follow_up_hit, projectile);
        assert_eq!(combo.transition, TransitionKind::Cancel);
        trace.hits[1].into_hitstun = false;
        assert!(trace.confirmed(opener, signature).is_none());
    }
}
