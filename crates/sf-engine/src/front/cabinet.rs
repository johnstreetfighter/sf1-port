//! The cabinet driver (ADR-0004): carries a running faithful machine from
//! the attract to the real country-select screen with coin and Start pulses
//! only, stepped hidden, and picks a remembered flag the same way. It reads
//! work RAM to time its pulses; it never writes it.

use crate::run::PowerOnMachine;

/// Who starts the game: one player (a coin, `start1`) or two (`coin1`,
/// `coin2`, `start2`, the verified `sf-start-2p-01` sequence).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Players {
    One,
    Two,
}

/// Slot 1's positions in the country select: the two-country loop
/// (`0x3e110`, `0x3e13a`) and the four-country loop (`0x3e27c`, `0x3e32c`).
pub const COUNTRY_SELECT: [u32; 4] = [0x3e110, 0x3e13a, 0x3e27c, 0x3e32c];
/// Slot 1's position at the stage start after the flag (`0x3e59c`).
pub const STAGE_START: u32 = 0x3e59c;

/// The credit count (`0xff865a..0xff865d`).
const CREDITS: u32 = 0xff865a;
/// The chosen country (`0xffc006`).
pub const COUNTRY: u32 = 0xffc006;
/// The four-country screen's country table (`0x3e2ea`, indexed by row × 2
/// + column).
const COUNTRY_TABLE4: usize = 0x3e2ea;

/// Frames a coin or Start is held, and released, per pulse (the scenarios
/// hold them twelve frames).
const PULSE_ON: u32 = 12;
const PULSE_OFF: u32 = 18;

/// Where a drive stopped short.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DriveError {
    /// The machine reached original branches the translation lacks.
    Untranslated { frame: u32, addresses: Vec<u32> },
    /// The frame budget ran out (slot 1 was at `position`).
    OutOfBudget { frame: u32, position: u32 },
}

/// What a drive stepped through: its frames and the sound commands the
/// machine latched once the game had started (idle `0xff` left out).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Drive {
    pub frames: u32,
    pub latches: Vec<u8>,
}

fn rb(m: &PowerOnMachine, addr: u32) -> u8 {
    m.work_ram()[(addr - 0xff8000) as usize]
}

fn rl(m: &PowerOnMachine, addr: u32) -> u32 {
    let a = (addr - 0xff8000) as usize;
    u32::from_be_bytes(m.work_ram()[a..a + 4].try_into().unwrap())
}

/// Slot 1's (the game flow's) position.
pub fn flow_position(m: &PowerOnMachine) -> u32 {
    m.task_positions()
        .iter()
        .find(|&&(s, _)| s == 1)
        .map_or(0, |&(_, a)| a)
}

/// Whether the country select's input loop is running.
pub fn at_country_select(m: &PowerOnMachine) -> bool {
    COUNTRY_SELECT.contains(&flow_position(m))
}

/// The country the select has chosen (`0xffc006`).
pub fn chosen_country(m: &PowerOnMachine) -> u8 {
    rb(m, COUNTRY)
}

/// Step once; with `record`, keep the frame's sound command (the commands
/// of the start screen and the country select, which the front end plays
/// once the hidden frames are over).
fn step_checked(m: &mut PowerOnMachine, drive: &mut Drive, record: bool) -> Result<(), DriveError> {
    m.step();
    drive.frames += 1;
    if record {
        drive
            .latches
            .extend(m.latch_writes().iter().map(|w| w.value).filter(|&v| v != 0xff));
    }
    let addresses = m.unrecovered();
    if addresses.is_empty() {
        Ok(())
    } else {
        Err(DriveError::Untranslated {
            frame: m.state.frame,
            addresses,
        })
    }
}

/// One pulse of the drive.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pulse {
    Coin1,
    Coin2,
    Start,
    Wait,
}

/// Feed exactly the coins a start needs (`coin1`, and `coin2` for two
/// players), wait until they are credited (a coin during the warning
/// screen is only counted once the attract joins at 794), then pulse Start
/// until the country select's input loop runs. Both players' sticks and
/// buttons are released throughout. Works from any attract phase; `budget`
/// caps the frames.
pub fn drive_to_country_select(
    m: &mut PowerOnMachine,
    players: Players,
    budget: u32,
) -> Result<Drive, DriveError> {
    let coins: &[Pulse] = match players {
        Players::One => &[Pulse::Coin1],
        Players::Two => &[Pulse::Coin1, Pulse::Coin2],
    };
    let mut drive = Drive::default();
    let mut fed = 0;
    let mut pulse = Pulse::Wait;
    let mut t = PULSE_ON + PULSE_OFF;
    m.set_p1_direction(false, false, false, false);
    m.set_p1_buttons([false; 6]);
    m.set_p2_direction(false, false, false, false);
    m.set_p2_buttons([false; 6]);
    while drive.frames < budget {
        if at_country_select(m) {
            m.set_cabinet_inputs(false, false, false, false, false);
            return Ok(drive);
        }
        if t == PULSE_ON + PULSE_OFF {
            t = 0;
            pulse = if fed < coins.len() {
                fed += 1;
                coins[fed - 1]
            } else if rl(m, CREDITS) >= coins.len() as u32 {
                Pulse::Start
            } else {
                Pulse::Wait
            };
        }
        let held = t < PULSE_ON;
        t += 1;
        let start = pulse == Pulse::Start && held;
        m.set_cabinet_inputs(
            pulse == Pulse::Coin1 && held,
            pulse == Pulse::Coin2 && held,
            start && players == Players::One,
            start && players == Players::Two,
            false,
        );
        step_checked(m, &mut drive, pulse == Pulse::Start)?;
    }
    Err(DriveError::OutOfBudget {
        frame: m.state.frame,
        position: flow_position(m),
    })
}

/// From the country select's input loop, pick `country` with P1's stick and
/// light punch, as a player would, and step to the stage start
/// (`0x3e59c`). On the four-country screen the stick picks a row (up,
/// down) and a column (left, right) of table `0x3e2ea`; on the two-country
/// screen left is Japan (0) and right the USA (2).
pub fn pick_country(
    m: &mut PowerOnMachine,
    program_rom: &[u8],
    country: u8,
    budget: u32,
) -> Result<Drive, DriveError> {
    let four = matches!(flow_position(m), 0x3e27c | 0x3e32c);
    // (right, left, down, up) presses, each held two frames then released.
    let mut presses: Vec<[bool; 4]> = Vec::new();
    if four {
        let idx = (0..4)
            .find(|&i| program_rom.get(COUNTRY_TABLE4 + i) == Some(&country))
            .unwrap_or(0);
        presses.push(if idx >= 2 { [false, false, true, false] } else { [false, false, false, true] });
        presses.push(if idx % 2 == 1 { [true, false, false, false] } else { [false, true, false, false] });
    } else {
        presses.push(if country == 2 { [true, false, false, false] } else { [false, true, false, false] });
    }
    let mut drive = Drive::default();
    m.set_cabinet_inputs(false, false, false, false, false);
    let mut script: Vec<([bool; 4], bool)> = Vec::new();
    for p in presses {
        script.extend([(p, false), (p, false), ([false; 4], false), ([false; 4], false)]);
    }
    script.extend([([false; 4], true), ([false; 4], true), ([false; 4], true)]);
    script.extend([([false; 4], false), ([false; 4], false)]);
    for (dirs, lp) in script {
        let [r, l, d, u] = dirs;
        m.set_p1_direction(r, l, d, u);
        m.set_p1_buttons([lp, false, false, false, false, false]);
        step_checked(m, &mut drive, true)?;
        if flow_position(m) == STAGE_START {
            m.set_p1_direction(false, false, false, false);
            m.set_p1_buttons([false; 6]);
            return Ok(drive);
        }
    }
    m.set_p1_direction(false, false, false, false);
    m.set_p1_buttons([false; 6]);
    while drive.frames < budget {
        if flow_position(m) == STAGE_START {
            return Ok(drive);
        }
        step_checked(m, &mut drive, true)?;
    }
    Err(DriveError::OutOfBudget {
        frame: m.state.frame,
        position: flow_position(m),
    })
}
