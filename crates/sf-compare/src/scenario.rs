//! The scenario boundary: reference identity, machine configuration,
//! frame-indexed logical inputs, bounded duration, and observation
//! settings. This is the same JSON contract the Python reference tooling
//! consumes; no adapter internals live here.

use std::collections::HashMap;
use std::fmt;
use std::path::Path;

use serde::Deserialize;

#[derive(Debug)]
pub struct ScenarioError(pub String);

impl fmt::Display for ScenarioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid scenario: {}", self.0)
    }
}

impl std::error::Error for ScenarioError {}

pub const DSW1_DEFAULT: u16 = 0xdfff;
pub const DSW2_DEFAULT: u16 = 0xffff;

#[derive(Debug)]
pub struct Scenario {
    pub scenario_id: String,
    pub format_version: u32,
    pub reference_rom_set: String,
    pub reference_mame_version: String,
    pub machine_set: String,
    pub controls: HashMap<String, Control>,
    pub duration_frames: u64,
    pub inputs: Vec<InputEvent>,
    pub screen_downsample_stride: u32,
    pub ram_crc_every_frames: u32,
    pub checkpoints: Vec<u64>,
    pub dsw1: u16,
    pub dsw2: u16,
    pub dip_changes: Vec<DipChange>,
    /// The file this scenario was loaded from, when known; used in reports.
    pub source_path: Option<std::path::PathBuf>,
}

#[derive(Debug, Deserialize)]
pub struct Control {
    pub port: String,
    pub bit: u32,
}

#[derive(Debug, Deserialize)]
pub struct InputEvent {
    pub frame: u64,
    pub press: String,
    pub hold_frames: u64,
    #[serde(default)]
    pub comment: String,
}

#[derive(Debug)]
pub struct DipChange {
    pub frame: u64,
    pub dsw1: u16,
    pub dsw2: u16,
}

#[derive(Deserialize)]
struct DipChangeJson {
    frame: Option<u64>,
    dipswitches: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct ScenarioJson {
    scenario_id: Option<String>,
    format_version: Option<u32>,
    reference: Option<ReferenceJson>,
    machine: Option<MachineJson>,
    controls: Option<HashMap<String, Control>>,
    duration_frames: Option<u64>,
    inputs: Option<Vec<InputEvent>>,
    observations: Option<ObservationsJson>,
    #[serde(default)]
    dip_changes: Vec<DipChangeJson>,
}

#[derive(Deserialize)]
struct ReferenceJson {
    rom_set: Option<String>,
    mame_version: Option<String>,
}

#[derive(Deserialize)]
struct MachineJson {
    set: Option<String>,
    dipswitches: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct ObservationsJson {
    screen_downsample_stride: Option<u32>,
    ram_crc_every_frames: Option<u32>,
    #[serde(default)]
    checkpoints: Vec<u64>,
}

fn require<T>(value: Option<T>, field: &str) -> Result<T, ScenarioError> {
    value.ok_or_else(|| ScenarioError(format!("missing required field {field}")))
}

pub fn load_scenario(path: impl AsRef<Path>) -> Result<Scenario, ScenarioError> {
    let text = std::fs::read_to_string(path.as_ref())
        .map_err(|e| ScenarioError(format!("cannot read {}: {e}", path.as_ref().display())))?;
    let mut scenario = parse_scenario(&text)?;
    scenario.source_path = Some(path.as_ref().to_path_buf());
    Ok(scenario)
}

pub fn parse_scenario(text: &str) -> Result<Scenario, ScenarioError> {
    let data: ScenarioJson =
        serde_json::from_str(text).map_err(|e| ScenarioError(format!("invalid JSON: {e}")))?;

    let scenario_id = require(data.scenario_id, "scenario.scenario_id")?;
    let format_version = require(data.format_version, "scenario.format_version")?;
    if format_version != 1 {
        return Err(ScenarioError(format!(
            "unsupported format_version {format_version} (this runner understands 1)"
        )));
    }
    let reference = require(data.reference, "scenario.reference")?;
    let machine = require(data.machine, "scenario.machine")?;
    let observations = require(data.observations, "scenario.observations")?;
    let controls = require(data.controls, "scenario.controls")?;
    let inputs = require(data.inputs, "scenario.inputs")?;

    for event in &inputs {
        if !controls.contains_key(&event.press) {
            return Err(ScenarioError(format!(
                "input event at frame {} presses unknown logical input '{}'",
                event.frame, event.press
            )));
        }
        if event.hold_frames < 1 {
            return Err(ScenarioError(format!(
                "input event for '{}' at frame {} has hold_frames < 1",
                event.press, event.frame
            )));
        }
    }
    let mut by_input: HashMap<&str, Vec<&InputEvent>> = HashMap::new();
    for event in &inputs {
        by_input
            .entry(event.press.as_str())
            .or_default()
            .push(event);
    }
    for (name, mut events) in by_input {
        events.sort_by_key(|e| e.frame);
        for pair in events.windows(2) {
            let (prev, cur) = (pair[0], pair[1]);
            if cur.frame < prev.frame + prev.hold_frames {
                return Err(ScenarioError(format!(
                    "overlapping press of '{name}' at frames {} and {}",
                    prev.frame, cur.frame
                )));
            }
        }
    }

    let (initial_dsw1, initial_dsw2) = parse_dipswitches(machine.dipswitches.as_ref())?;
    let (mut dsw1, mut dsw2) = (initial_dsw1, initial_dsw2);
    let mut dip_changes: Vec<DipChange> = Vec::new();
    for change in data.dip_changes {
        let frame = require(change.frame, "dip_changes[].frame")?;
        if let Some(previous) = dip_changes.last() {
            if frame <= previous.frame {
                return Err(ScenarioError(
                    "dip_changes frames must be strictly increasing".into(),
                ));
            }
        }
        let raw = require(change.dipswitches, "dip_changes[].dipswitches")?;
        (dsw1, dsw2) = parse_dipswitches_from(Some(&raw), dsw1, dsw2)?;
        dip_changes.push(DipChange { frame, dsw1, dsw2 });
    }

    Ok(Scenario {
        scenario_id,
        format_version,
        reference_rom_set: require(reference.rom_set, "scenario.reference.rom_set")?,
        reference_mame_version: require(reference.mame_version, "scenario.reference.mame_version")?,
        machine_set: require(machine.set, "scenario.machine.set")?,
        controls,
        duration_frames: require(data.duration_frames, "scenario.duration_frames")?,
        inputs,
        screen_downsample_stride: require(
            observations.screen_downsample_stride,
            "scenario.observations.screen_downsample_stride",
        )?,
        ram_crc_every_frames: require(
            observations.ram_crc_every_frames,
            "scenario.observations.ram_crc_every_frames",
        )?,
        checkpoints: observations.checkpoints,
        dsw1: initial_dsw1,
        dsw2: initial_dsw2,
        dip_changes,
        source_path: None,
    })
}

fn parse_port_word(val: &serde_json::Value, port_name: &str) -> Result<u16, ScenarioError> {
    let word: i64 = match val {
        serde_json::Value::Number(n) => n.as_i64().ok_or_else(|| {
            ScenarioError(format!("DIP port {port_name} word {val} is not an integer"))
        })?,
        serde_json::Value::String(s) => {
            let s = s.trim();
            if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
                i64::from_str_radix(hex, 16)
            } else {
                s.parse::<i64>()
            }
            .map_err(|_| {
                ScenarioError(format!(
                    "DIP port {port_name} word '{s}' is not a valid integer"
                ))
            })?
        }
        _ => {
            return Err(ScenarioError(format!(
                "DIP port {port_name} word must be an integer or hex string, got {val}"
            )))
        }
    };
    u16::try_from(word).map_err(|_| {
        ScenarioError(format!(
            "DIP port {port_name} word {val} is out of 16-bit range 0x0000..0xffff"
        ))
    })
}

fn coin_setting(s: &str, shift: u32) -> Option<u16> {
    let raw: u16 = match s {
        "4 Coins/1 Credit" | "4C_1C" | "4C1C" | "4C/1C" => 0x0,
        "3 Coins/1 Credit" | "3C_1C" | "3C1C" | "3C/1C" => 0x1,
        "2 Coins/1 Credit" | "2C_1C" | "2C1C" | "2C/1C" => 0x2,
        "1 Coin/6 Credits" | "1C_6C" | "1C6C" | "1C/6C" => 0x3,
        "1 Coin/4 Credits" | "1C_4C" | "1C4C" | "1C/4C" => 0x4,
        "1 Coin/3 Credits" | "1C_3C" | "1C3C" | "1C/3C" => 0x5,
        "1 Coin/2 Credits" | "1C_2C" | "1C2C" | "1C/2C" => 0x6,
        "1 Coin/1 Credit" | "1C_1C" | "1C1C" | "1C/1C" => 0x7,
        _ => return None,
    };
    Some(raw << shift)
}

fn stage_max_setting(s: &str, shift: u32) -> Option<u16> {
    let raw: u16 = match s {
        "5th Stage Maximum" | "5th Stage" | "5" => 0x7,
        "4th Stage Maximum" | "4th Stage" | "4" => 0x6,
        "3rd Stage Maximum" | "3rd Stage" | "3" => 0x5,
        "2nd Stage Maximum" | "2nd Stage" | "2" => 0x4,
        "1st Stage Maximum" | "1st Stage" | "1" => 0x3,
        "None" => {
            if shift == 0 {
                0x2
            } else {
                0x1
            }
        }
        _ => return None,
    };
    Some(raw << shift)
}

/// Resolve a scenario's `machine.dipswitches` field into `(dsw1, dsw2)`.
pub fn parse_dipswitches(raw: Option<&serde_json::Value>) -> Result<(u16, u16), ScenarioError> {
    parse_dipswitches_from(raw, DSW1_DEFAULT, DSW2_DEFAULT)
}

fn parse_dipswitches_from(
    raw: Option<&serde_json::Value>,
    dsw1: u16,
    dsw2: u16,
) -> Result<(u16, u16), ScenarioError> {
    let Some(raw) = raw else {
        return Ok((dsw1, dsw2));
    };
    match raw {
        serde_json::Value::Null => return Ok((dsw1, dsw2)),
        serde_json::Value::String(s) => {
            return match s.as_str() {
                "defaults"
                | "factory defaults"
                | "factory defaults; observed readback DSW1=0xdfff, DSW2=0xffff" => {
                    Ok((dsw1, dsw2))
                }
                _ => Err(ScenarioError(format!("unknown dipswitches preset '{s}'"))),
            };
        }
        serde_json::Value::Object(_) => {}
        other => {
            return Err(ScenarioError(format!(
                "machine.dipswitches must be a mapping or default string, got {other}"
            )))
        }
    }
    let map = raw.as_object().unwrap();
    if map.is_empty() {
        return Err(ScenarioError(
            "machine.dipswitches mapping cannot be empty".into(),
        ));
    }

    let mut dsw1 = dsw1;
    let mut dsw2 = dsw2;
    let mut explicit_dsw1: Option<u16> = None;
    let mut explicit_dsw2: Option<u16> = None;

    for (key, value) in map {
        if key == "dsw1" || key == "DSW1" {
            let w = parse_port_word(value, "DSW1")?;
            explicit_dsw1 = Some(w);
            dsw1 = w;
        } else if key == "dsw2" || key == "DSW2" {
            let w = parse_port_word(value, "DSW2")?;
            explicit_dsw2 = Some(w);
            dsw2 = w;
        }
    }

    for (key, value) in map {
        if matches!(key.as_str(), "dsw1" | "DSW1" | "dsw2" | "DSW2") {
            continue;
        }
        let val_str = match value {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            _ => {
                return Err(ScenarioError(format!(
                    "invalid value {value} for DIP switch '{key}'"
                )))
            }
        };
        let (is_dsw1, mask, bits) = match key.as_str() {
            "Coin A" | "coin_a" => (
                true,
                0x0007u16,
                coin_setting(&val_str, 0).ok_or_else(|| {
                    ScenarioError(format!("invalid value '{val_str}' for DIP switch '{key}'"))
                })?,
            ),
            "Coin B" | "coin_b" => (
                true,
                0x0038u16,
                coin_setting(&val_str, 3).ok_or_else(|| {
                    ScenarioError(format!("invalid value '{val_str}' for DIP switch '{key}'"))
                })?,
            ),
            "Flip Screen" | "flip_screen" | "Flip" | "flip" => (
                true,
                0x0100u16,
                match val_str.as_str() {
                    "Off" => 0x0100,
                    "On" => 0x0000,
                    _ => {
                        return Err(ScenarioError(format!(
                            "invalid value '{val_str}' for DIP switch '{key}'"
                        )))
                    }
                },
            ),
            "Attract Music" | "attract_music" => (
                true,
                0x0200u16,
                match val_str.as_str() {
                    "Off" => 0x0000,
                    "On" => 0x0200,
                    _ => {
                        return Err(ScenarioError(format!(
                            "invalid value '{val_str}' for DIP switch '{key}'"
                        )))
                    }
                },
            ),
            "Speed" | "speed" => (
                true,
                0x1000u16,
                match val_str.as_str() {
                    "Slow" => 0x0000,
                    "Normal" => 0x1000,
                    _ => {
                        return Err(ScenarioError(format!(
                            "invalid value '{val_str}' for DIP switch '{key}'"
                        )))
                    }
                },
            ),
            "Demo Sounds" | "demo_sounds" => (
                true,
                0x2000u16,
                match val_str.as_str() {
                    "Off" => 0x2000,
                    "On" => 0x0000,
                    _ => {
                        return Err(ScenarioError(format!(
                            "invalid value '{val_str}' for DIP switch '{key}'"
                        )))
                    }
                },
            ),
            "Freeze" | "freeze" => (
                true,
                0x4000u16,
                match val_str.as_str() {
                    "Off" => 0x4000,
                    "On" => 0x0000,
                    _ => {
                        return Err(ScenarioError(format!(
                            "invalid value '{val_str}' for DIP switch '{key}'"
                        )))
                    }
                },
            ),
            "Service Mode" | "service_mode" | "Service" | "service" | "Self-Test" | "self_test" => {
                (
                    true,
                    0x8000u16,
                    match val_str.as_str() {
                        "Off" => 0x8000,
                        "On" => 0x0000,
                        _ => {
                            return Err(ScenarioError(format!(
                                "invalid value '{val_str}' for DIP switch '{key}'"
                            )))
                        }
                    },
                )
            }
            "Game Continuation" | "game_continuation" => (
                false,
                0x0007u16,
                stage_max_setting(&val_str, 0).ok_or_else(|| {
                    ScenarioError(format!("invalid value '{val_str}' for DIP switch '{key}'"))
                })?,
            ),
            "Round Time Count" | "round_time_count" | "Round Time" | "round_time" => (
                false,
                0x0018u16,
                match val_str.as_str() {
                    "100" => 0x0018,
                    "150" => 0x0010,
                    "200" => 0x0008,
                    "250" => 0x0000,
                    _ => {
                        return Err(ScenarioError(format!(
                            "invalid value '{val_str}' for DIP switch '{key}'"
                        )))
                    }
                },
            ),
            "Difficulty" | "difficulty" => (
                false,
                0x0060u16,
                match val_str.as_str() {
                    "Normal" => 0x0060,
                    "Easy" => 0x0040,
                    "Difficult" => 0x0020,
                    "Very Difficult" | "Very_Difficult" => 0x0000,
                    _ => {
                        return Err(ScenarioError(format!(
                            "invalid value '{val_str}' for DIP switch '{key}'"
                        )))
                    }
                },
            ),
            "Buy-In Feature" | "buy_in_feature" | "Buy-In" | "buy_in" => (
                false,
                0x0380u16,
                stage_max_setting(&val_str, 7).ok_or_else(|| {
                    ScenarioError(format!("invalid value '{val_str}' for DIP switch '{key}'"))
                })?,
            ),
            "Number of Countries Selected"
            | "number_of_countries_selected"
            | "Countries"
            | "countries" => (
                false,
                0x0400u16,
                match val_str.as_str() {
                    "2" => 0x0400,
                    "4" => 0x0000,
                    _ => {
                        return Err(ScenarioError(format!(
                            "invalid value '{val_str}' for DIP switch '{key}'"
                        )))
                    }
                },
            ),
            _ => return Err(ScenarioError(format!("unknown DIP switch '{key}'"))),
        };
        if is_dsw1 {
            if let Some(w) = explicit_dsw1 {
                if (w & mask) != bits {
                    return Err(ScenarioError(format!(
                        "DIP switch '{key}' conflicts with explicit DSW1 port word"
                    )));
                }
            }
            dsw1 = (dsw1 & !mask) | bits;
        } else {
            if let Some(w) = explicit_dsw2 {
                if (w & mask) != bits {
                    return Err(ScenarioError(format!(
                        "DIP switch '{key}' conflicts with explicit DSW2 port word"
                    )));
                }
            }
            dsw2 = (dsw2 & !mask) | bits;
        }
    }

    if matches!(dsw2 & 0x0007, 0x0000 | 0x0001) {
        return Err(ScenarioError(format!(
            "DSW2 word 0x{dsw2:04x} uses unlisted Game Continuation bits 0x{:04x}",
            dsw2 & 0x0007
        )));
    }
    if matches!(dsw2 & 0x0380, 0x0000 | 0x0100) {
        return Err(ScenarioError(format!(
            "DSW2 word 0x{dsw2:04x} uses unlisted Buy-In Feature bits 0x{:04x}",
            dsw2 & 0x0380
        )));
    }

    Ok((dsw1, dsw2))
}
