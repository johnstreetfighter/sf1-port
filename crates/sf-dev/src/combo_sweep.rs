//! Combo discovery in native Local fights. The original Ryu route remains
//! runnable and playable alongside coverage sweeps for selected fighters.

mod coverage;

use std::{fs, path::Path};

use minifb::Key;
use serde::{Deserialize, Serialize};
use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::era::{EraFamily, Revision};
use sf_engine::fighter::{P1_STRUCT, P2_STRUCT, WORK_BASE};
use sf_engine::front::cabinet::{flow_position, Players};
use sf_engine::front::fight::FightKind;
use sf_engine::front::{FrameInputs, FrameOutput, GameSession, MenuItem, PlayerInputs, Transition};
use sf_engine::roster::{StartingCountry, TournamentFighter};
use sf_engine::version;
use sf_host::window::{frame_buffer, open_window, present};
use sf_video::callouts::Side;
use sf_video::render::Frame;
use sf_video::romset::{crc32, load_assets_from_manifest, set_name, Assets};

const FORMAT_VERSION: u32 = 1;
const FIGHT_LOOP: u32 = 0x3eafa;
const STAGE_AND_FIGHT: usize = (0xffc003 - WORK_BASE) as usize;
const CLOSE_GAP: i32 = 48;
const STANDING: u8 = 0x01;
const CROUCHING_LIGHT_KICK: u8 = 0x17;
const HADOKEN: u8 = 0x1a;
// Include LP before the motion completes so the sweep covers the recovery
// buffer and the punch release edge under Definitive Overhaul.
const FIRST_LP_FRAME: usize = 1;
const ROUTE_FRAMES: usize = 70;
const PRESET: RulesetPreset = RulesetPreset::DefinitiveOverhaul;
const STAGE: &str = "Japan, Retsu's stage";
const STYLE: &str = "SF1+";

const RIGHT: u16 = 1 << 0;
const LEFT: u16 = 1 << 1;
const DOWN: u16 = 1 << 2;
const UP: u16 = 1 << 3;
const LP: u16 = 1 << 4;
const LK: u16 = 1 << 7;

/// Logical pad bits in FrameInputs order (R, L, D, U, LP, MP, HP, LK, MK,
/// HK). This route also uses the front end's Start key to enter the menu.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Input {
    p1: u16,
    p2: u16,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    start: bool,
}

impl Input {
    fn frame(self) -> FrameInputs {
        fn player(bits: u16) -> PlayerInputs {
            PlayerInputs {
                directions: std::array::from_fn(|i| bits & (1 << i) != 0),
                buttons: std::array::from_fn(|i| bits & (1 << (i + 4)) != 0),
            }
        }
        let mut frame = FrameInputs {
            p1: player(self.p1),
            p2: player(self.p2),
            ..FrameInputs::default()
        };
        frame.menu.start = self.start;
        frame
    }
}

fn p1(bits: u16) -> Input {
    Input {
        p1: bits,
        ..Input::default()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Build {
    version: String,
    revision: String,
    rom_set: String,
    region_crc32: [String; 6],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Setup {
    p1: String,
    p2: String,
    defender_stance: String,
    stage: String,
    preset: String,
    style: String,
    separation_px: i32,
    centers_x: [i32; 2],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Stance {
    Standing,
    Crouching,
}

impl Stance {
    fn name(self) -> &'static str {
        match self {
            Self::Standing => "standing",
            Self::Crouching => "crouching",
        }
    }

    fn pad(self) -> u16 {
        match self {
            Self::Standing => 0,
            Self::Crouching => DOWN,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Hit {
    frame: usize,
    move_id: u8,
    projectile: bool,
    damage: u16,
    chip: bool,
    into_hitstun: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct Replay {
    format_version: u32,
    build: Build,
    setup: Setup,
    /// Frame zero of the route, counted from power-on.
    route_start: usize,
    /// Frames below are relative to `route_start`.
    hadoken_input_frame: usize,
    hadoken_start_frame: usize,
    cancelled: bool,
    expected_hits: [Hit; 2],
    /// Inclusive LP windows, relative to the opener's first hit.
    success_windows: Vec<[i32; 2]>,
    inputs: Vec<Input>,
}

type Result<T> = std::result::Result<T, String>;

pub(super) fn run(command: &str, args: &[String]) -> i32 {
    if args == ["--help"] || args.is_empty() {
        println!("sf-dev combo-sweep <replay.json>   Measure the Ryu route and save a replay");
        println!("sf-dev combo-sweep <fighter> <output-folder> <signature>...");
        println!("sf-dev combo-sweep <fighter> <output-folder> --command <command.json>");
        println!("sf-dev combo-replay <replay.json>  Watch the recorded inputs in a window");
        println!("sf-dev combo-verify <replay.json>  Verify the recorded inputs headlessly");
        return if args.is_empty() { 2 } else { 0 };
    }
    let result = match (command, args) {
        ("combo-sweep", [path]) => sweep(Path::new(path)),
        ("combo-sweep", [fighter, folder, signatures @ ..]) if !signatures.is_empty() => {
            coverage::sweep(fighter, Path::new(folder), signatures)
        }
        ("combo-replay" | "combo-verify", [path]) => {
            replay(Path::new(path), command == "combo-replay")
        }
        _ => {
            eprintln!("{command}: invalid arguments. See {command} --help");
            return 2;
        }
    };
    if let Err(error) = result {
        eprintln!("{command}: {error}");
        return 1;
    }
    0
}

fn load() -> Result<(Build, GameSession)> {
    let (build, assets) = load_roms()?;
    Ok((
        build,
        GameSession::new(assets, MatchConfig::from_preset(PRESET)),
    ))
}

fn load_roms() -> Result<(Build, Assets)> {
    let manifest = fs::read_to_string(super::resolve_local(super::MANIFEST))
        .map_err(|error| format!("read ROM manifest: {error}"))?;
    let assets = load_assets_from_manifest(super::resolve_local(super::ROMS), &manifest)
        .map_err(|error| error.to_string())?;
    let build = build_identity(&manifest, &assets)?;
    Ok((build, assets))
}

fn build_identity(manifest: &str, assets: &Assets) -> Result<Build> {
    let (rom_set, _) = set_name(manifest).map_err(|error| error.to_string())?;
    Ok(Build {
        version: version::VERSION.into(),
        revision: version::GIT_SHA.into(),
        rom_set,
        // In manifest region order for the six assembled assets the fight uses.
        region_crc32: [
            &assets.maincpu,
            &assets.gfx1,
            &assets.gfx2,
            &assets.gfx3,
            &assets.gfx4,
            &assets.tilerom,
        ]
        .map(|region| format!("{:08x}", crc32(region))),
    })
}

fn measure_setup(session: &GameSession) -> Result<Setup> {
    measure_local(session, TournamentFighter::Ryu, Stance::Standing)
}

fn measure_local(
    session: &GameSession,
    fighter: TournamentFighter,
    stance: Stance,
) -> Result<Setup> {
    let fight = session.fight().ok_or("expected a Local fight")?;
    let setup = fight.setup();
    let machine = fight.machine();
    let p1 = fight.player_fighter(0);
    let p2 = fight.player_fighter(1);
    if setup.kind != FightKind::Local
        || setup.players != Players::Two
        || setup.config.preset != PRESET
        || !setup.config.toggles.true_combos
        || setup.config.p2_fair_cpu
        || [p1, p2] != [fighter, TournamentFighter::Ken]
        || ![0, 1].into_iter().all(|side| {
            fight.fighter_style(side).is_some_and(|style| {
                style.family == EraFamily::Sf1Plus && style.revision == Revision::Sf1Plus
            })
        })
        || fight.country() != Some(StartingCountry::Japan as u8)
        || setup.config.roster.stage.is_some()
        || machine.work_ram()[STAGE_AND_FIGHT..STAGE_AND_FIGHT + 2] != [0, 0]
        || flow_position(machine) != FIGHT_LOOP
    {
        return Err(format!(
            "expected the native Local {} vs. Ken fight in Japan, got {setup:?}",
            fighter.name()
        ));
    }
    let telemetry = machine.diagnostic_telemetry();
    let centers_x = [telemetry.p1.center_x, telemetry.p2.center_x];
    let separation_px = centers_x[1] - centers_x[0];
    let ken = fight.fight_read().fighter(Side::P2);
    let ken_move = match stance {
        Stance::Standing => STANDING,
        Stance::Crouching => 0x05,
    };
    if !(1..=CLOSE_GAP).contains(&separation_px)
        || telemetry.p1.facing_left
        || !telemetry.p2.facing_left
        || !ken.present
        || ken.move_id != ken_move
        || ken.locked()
    {
        return Err(format!(
            "expected close {} Ken, got gap {separation_px} px and move {:#04x}",
            stance.name(),
            ken.move_id
        ));
    }
    Ok(Setup {
        p1: p1.name().into(),
        p2: p2.name().into(),
        defender_stance: stance.name().into(),
        stage: STAGE.into(),
        preset: PRESET.display_name().into(),
        style: STYLE.into(),
        separation_px,
        centers_x,
    })
}

struct SetupDrive<'a> {
    session: &'a mut GameSession,
    inputs: Vec<Input>,
}

impl SetupDrive<'_> {
    fn step(&mut self, input: Input) -> FrameOutput {
        self.inputs.push(input);
        self.session.step(input.frame())
    }

    fn idle(&mut self, frames: usize) {
        for _ in 0..frames {
            self.step(Input::default());
        }
    }

    fn tap(&mut self, input: Input) -> Option<Transition> {
        let transition = self.step(input).transition;
        self.idle(1);
        transition
    }

    fn confirm_fighter(&mut self, input: Input, expected: Transition) -> Result<()> {
        // The Style card uses the same confirm input to lock its default.
        for _ in 0..2 {
            if self.tap(input) == Some(expected) {
                return Ok(());
            }
        }
        Err(format!(
            "expected {expected:?}, reached {}",
            self.session.screen()
        ))
    }

    fn until(
        &mut self,
        ready: impl Fn(&GameSession) -> bool,
        limit: usize,
        label: &str,
    ) -> Result<()> {
        for _ in 0..limit {
            if ready(self.session) {
                return Ok(());
            }
            self.idle(1);
        }
        Err(format!(
            "expected {label}, reached {}",
            self.session.screen()
        ))
    }

    fn close_in(&mut self, fighter: TournamentFighter, defender: Stance) -> Result<()> {
        let still = Input {
            p2: defender.pad(),
            ..Input::default()
        };
        for _ in 0..300 {
            let telemetry = self
                .session
                .fight_machine()
                .ok_or("fight ended")?
                .diagnostic_telemetry();
            if telemetry.p2.center_x - telemetry.p1.center_x <= CLOSE_GAP {
                for _ in 0..4 {
                    self.step(still);
                }
                return Ok(());
            }
            self.step(Input { p1: RIGHT, ..still });
        }
        Err(format!(
            "could not walk {} into close range",
            fighter.name()
        ))
    }
}

fn drive_setup(session: &mut GameSession) -> Result<Vec<Input>> {
    drive_local(session, TournamentFighter::Ryu)
}

fn drive_local(session: &mut GameSession, fighter: TournamentFighter) -> Result<Vec<Input>> {
    let mut drive = SetupDrive {
        session,
        inputs: Vec::new(),
    };
    drive.idle(200);
    let started = drive.tap(Input {
        start: true,
        ..Input::default()
    });
    drive.tap(p1(DOWN));
    let multiplayer = drive.tap(p1(LP));
    let local = drive.tap(p1(LP));
    if started != Some(Transition::OpenMainMenu)
        || multiplayer != Some(Transition::Choose(MenuItem::MultiPlayer))
        || local != Some(Transition::Choose(MenuItem::Local))
    {
        return Err("could not enter Multi Player > Local from the Main Menu".into());
    }
    let index = TournamentFighter::ALL
        .iter()
        .position(|&f| f == fighter)
        .ok_or_else(|| format!("{} is outside the tournament roster", fighter.name()))?;
    if index >= 6 {
        drive.tap(p1(DOWN));
    }
    for _ in 0..index % 6 {
        drive.tap(p1(RIGHT));
    }
    drive.confirm_fighter(p1(LP), Transition::Picked(fighter))?;
    drive.confirm_fighter(
        Input {
            p2: LP,
            ..Input::default()
        },
        Transition::Fighters(fighter, TournamentFighter::Ken),
    )?;
    drive.until(|s| s.screen() == "country select", 40, "country select")?;
    drive.tap(p1(LP));
    drive.until(
        |s| s.fight_machine().map(flow_position) == Some(FIGHT_LOOP),
        3000,
        "round control",
    )?;
    drive.idle(240);
    drive.close_in(fighter, Stance::Standing)?;
    Ok(drive.inputs)
}

/// Each attempt uses identical directions. Only the one-frame LP pulse moves.
fn route_inputs(press: Option<usize>) -> Vec<Input> {
    (0..ROUTE_FRAMES)
        .map(|frame| {
            let direction = match frame {
                0..=2 => DOWN,
                3..=4 => DOWN | RIGHT,
                _ => RIGHT,
            };
            let button = if frame == 0 {
                LK
            } else if press == Some(frame) {
                LP
            } else {
                0
            };
            p1(direction | button)
        })
        .collect()
}

#[derive(Debug, Default)]
struct Attempt {
    hits: Vec<Hit>,
    hadoken_started: Option<usize>,
    cancelled: bool,
    last_hitstun: Option<usize>,
}

impl Attempt {
    fn combo(&self) -> Option<[Hit; 2]> {
        let [opener, follow_up] = self.hits.as_slice() else {
            return None;
        };
        let started = self.hadoken_started?;
        (opener.move_id == CROUCHING_LIGHT_KICK
            && !opener.projectile
            && !opener.chip
            && follow_up.move_id == HADOKEN
            && follow_up.projectile
            && !follow_up.chip
            && opener.frame < follow_up.frame
            && started <= follow_up.frame
            && follow_up.into_hitstun)
            .then_some([*opener, *follow_up])
    }

    fn observe(&mut self, session: &GameSession, frame: usize) -> Result<()> {
        let fight = session.fight().ok_or("fight ended during attempt")?;
        let read = fight.fight_read();
        let ryu = read.fighter(Side::P1);
        let machine = fight.machine();
        if ryu.move_id == HADOKEN && self.hadoken_started.is_none() {
            self.hadoken_started = Some(frame);
            self.cancelled = machine.toggle_state().cancel.count[0] > 0;
        }
        if machine.toggle_state().combo.hitstun[1] > 0 {
            self.last_hitstun = Some(frame);
        }
        if let Some(hit) = read.landed_by(Side::P1) {
            let telemetry = machine.diagnostic_telemetry();
            let projectile = telemetry.p2.projectile_impacted_this_frame
                && telemetry.projectiles.iter().any(|effect| {
                    effect.owner_struct == P1_STRUCT && effect.target_struct == P2_STRUCT
                });
            // The only projectile available in this route is the single Hadoken.
            // Its hit is attributed even when Ryu has recovered before impact.
            let move_id = if projectile && self.hadoken_started.is_some() {
                HADOKEN
            } else {
                ryu.move_id
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
        Ok(())
    }
}

fn play_attempt(
    session: &mut GameSession,
    inputs: &[Input],
    mut present_frame: impl FnMut(&Frame) -> bool,
) -> Result<Option<Attempt>> {
    let mut attempt = Attempt::default();
    for (frame, &input) in inputs.iter().enumerate() {
        let out = session.step(input.frame());
        attempt.observe(session, frame)?;
        if !present_frame(&out.frame) {
            return Ok(None);
        }
    }
    Ok(Some(attempt))
}

fn probe(session: &GameSession, press: Option<usize>) -> Result<Attempt> {
    // All measurements start from the same settled, real fight state.
    play_attempt(&mut session.clone(), &route_inputs(press), |_| true)?
        .ok_or_else(|| "attempt interrupted".into())
}

fn sweep(path: &Path) -> Result<()> {
    let (build, mut session) = load()?;
    let mut inputs = drive_setup(&mut session)?;
    let setup = measure_setup(&session)?;
    let baseline = probe(&session, None)?;
    let [normal] = baseline.hits.as_slice() else {
        return Err(format!(
            "expected one opener hit in baseline, got {:?}",
            baseline.hits
        ));
    };
    if normal.move_id != CROUCHING_LIGHT_KICK || normal.chip || normal.projectile {
        return Err(format!(
            "expected a clean crouching light kick, got {normal:?}"
        ));
    }
    let last_press = baseline
        .last_hitstun
        .ok_or("the opener did not cause hitstun")?;
    if !(FIRST_LP_FRAME..ROUTE_FRAMES - 1).contains(&last_press) {
        return Err(format!(
            "unexpected end of opener hitstun at frame {last_press}"
        ));
    }

    println!(
        "Local Ryu ({STYLE}) vs. standing Ken ({STYLE}), {STAGE}, {}",
        PRESET.display_name()
    );
    println!(
        "Starting separation {} px (fighter centers x={} and x={}).",
        setup.separation_px, setup.centers_x[0], setup.centers_x[1]
    );
    println!("Ryu's crouching light kick -> light Hadoken. Ken stays neutral.");
    println!(
        "Fixed inputs from route frame 0: Down+LK at 0, Down at 1-2, Down+Forward at 3-4, Forward at 5-69."
    );
    println!(
        "Only the one-frame LP pulse moves. Baseline first hit is at route frame {}.",
        normal.frame
    );
    println!(
        "Testing {} LP presses on route frames {FIRST_LP_FRAME}-{last_press}.",
        last_press - FIRST_LP_FRAME + 1,
    );

    let mut successes = Vec::new();
    for press in FIRST_LP_FRAME..=last_press {
        let attempt = probe(&session, Some(press))?;
        if let Some(hits) = attempt.combo() {
            successes.push((
                press as i32 - hits[0].frame as i32,
                press,
                hits,
                attempt.hadoken_started.unwrap(),
                attempt.cancelled,
            ));
        } else {
            let offset = press as i32 - normal.frame as i32;
            let projectile = attempt.hits.iter().find(|hit| hit.projectile);
            let outcome = match (attempt.hadoken_started, projectile) {
                (None, _) => "Hadoken did not start",
                (Some(_), None) => "Hadoken started without a projectile hit",
                (Some(_), Some(hit)) if !hit.into_hitstun => "projectile landed outside hitstun",
                _ => "the expected two clean hits did not occur",
            };
            println!("LP {offset:+} (baseline hit offset): {outcome}");
        }
    }
    if successes.is_empty() {
        return Err("no combo found in the tested inputs".into());
    }
    successes.sort_unstable_by_key(|&(offset, press, ..)| (offset, press));
    successes.dedup_by_key(|success| success.0);
    let ranges: Vec<_> = successes
        .chunk_by(|left, right| left.0 + 1 == right.0)
        .collect();
    println!("Successful LP windows relative to crouching light kick contact:");
    let success_windows: Vec<_> = ranges
        .iter()
        .map(|range| [range[0].0, range[range.len() - 1].0])
        .collect();
    for [first, last] in &success_windows {
        println!("  {first:+} to {last:+} ({} frames)", last - first + 1,);
    }
    let widest = ranges
        .into_iter()
        .max_by_key(|r| (r.len(), std::cmp::Reverse(r[0].0)))
        .unwrap();
    let (offset, press, hits, start, cancelled) = widest[(widest.len() - 1) / 2];
    let transition = if cancelled {
        "cancel"
    } else {
        "after recovery"
    };
    println!(
        "Widest window: {:+} to {:+} ({} frames).",
        widest[0].0,
        widest[widest.len() - 1].0,
        widest.len()
    );
    println!(
        "Replay uses LP {offset:+}. Hadoken starts at route frame {start} ({transition}) and hits at {}.",
        hits[1].frame
    );
    let route_start = inputs.len();
    inputs.extend(route_inputs(Some(press)));
    let replay = Replay {
        format_version: FORMAT_VERSION,
        build,
        setup,
        route_start,
        hadoken_input_frame: press,
        hadoken_start_frame: start,
        cancelled,
        expected_hits: hits,
        success_windows,
        inputs,
    };
    let json = serde_json::to_vec(&replay).map_err(|error| error.to_string())?;
    fs::write(path, json).map_err(|error| format!("write {}: {error}", path.display()))?;
    println!("Saved {}", path.display());
    Ok(())
}

fn replay(path: &Path, watch: bool) -> Result<()> {
    let json = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    #[derive(Deserialize)]
    struct Format {
        format_version: u32,
    }
    let format: Format =
        serde_json::from_slice(&json).map_err(|error| format!("read replay: {error}"))?;
    if format.format_version == coverage::FORMAT_VERSION {
        return coverage::replay(&json, watch);
    }
    let replay: Replay =
        serde_json::from_slice(&json).map_err(|error| format!("read replay: {error}"))?;
    let (build, mut session) = load()?;
    if replay.format_version != FORMAT_VERSION {
        return Err(format!(
            "unsupported replay format {}",
            replay.format_version
        ));
    }
    if replay.build != build {
        return Err(format!(
            "build or ROM assets differ from the replay: expected {:?}, got {build:?}",
            replay.build
        ));
    }
    let (setup_inputs, attempt_inputs) = replay
        .inputs
        .split_at_checked(replay.route_start)
        .ok_or("normal input frame is outside the input stream")?;
    let press = replay.hadoken_input_frame;
    if !(FIRST_LP_FRAME..ROUTE_FRAMES).contains(&press) {
        return Err("Hadoken input frame is outside the command sequence".into());
    }
    if attempt_inputs != route_inputs(Some(press)) {
        return Err("recorded attempt inputs differ from the stated command sequence".into());
    }
    let mut show = presenter(watch, "Ryu");
    for input in setup_inputs {
        if !show(&session.step(input.frame()).frame) {
            return Ok(());
        }
    }
    let actual_setup = measure_setup(&session)?;
    if actual_setup != replay.setup {
        return Err(format!(
            "setup mismatch: expected {:?}, got {actual_setup:?}",
            replay.setup
        ));
    }
    let Some(attempt) = play_attempt(&mut session, attempt_inputs, show)? else {
        return Ok(());
    };
    let start = attempt.hadoken_started;
    if attempt.combo() != Some(replay.expected_hits)
        || start != Some(replay.hadoken_start_frame)
        || attempt.cancelled != replay.cancelled
    {
        return Err(format!(
            "hit mismatch: expected {:?}, Hadoken start {} (cancel {}); got {:?}, Hadoken start {start:?} (cancel {})",
            replay.expected_hits, replay.hadoken_start_frame, replay.cancelled, attempt.hits, attempt.cancelled,
        ));
    }
    println!(
        "Verified crouching light kick at route frame {} and light Hadoken at route frame {} in hitstun.",
        replay.expected_hits[0].frame, replay.expected_hits[1].frame
    );
    Ok(())
}

fn presenter(watch: bool, fighter: &str) -> impl FnMut(&Frame) -> bool {
    let mut window = watch.then(|| open_window(&format!("sf-dev combo replay - {fighter} vs Ken")));
    let mut buffer = watch.then(frame_buffer);
    move |frame: &Frame| {
        if let (Some(window), Some(buffer)) = (&mut window, &mut buffer) {
            if !window.is_open() || window.is_key_down(Key::Escape) {
                return false;
            }
            present(window, buffer, frame);
        }
        true
    }
}
