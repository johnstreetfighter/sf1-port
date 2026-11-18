//! The round's column wipe covers the Era Gauges, as it covers the life bars.
//!
//! Between rounds the ROM sweeps a black tile across the text layer one
//! column at a time, holds the black for the ROUND card, and sweeps it away
//! from a stage that is still switched off.
//! Everything the overhaul presenter adds must stay under that black.

#[path = "support/alpha.rs"]
mod alpha;

use alpha::{local_fight, local_select_with_p1, MANIFEST, ROMS};
use sf_engine::era::{GaugeMode, Loadout, Revision};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::fight::Fight;
use sf_engine::front::PlayerInputs;
use sf_engine::tasks::TIME_DIGITS;
use sf_video::render::{Frame, HEIGHT, VISAREA_X, VISAREA_Y, WIDTH};
use sf_video::romset::load_assets;

const IDLE: PlayerInputs = PlayerInputs { directions: [false; 4], buttons: [false; 6] };
const FIGHT_LOOP: u32 = 0x3eafa;
const ROUND_COUNTER: usize = 0xffc00a - 0xff8000;
const TEXT_COLS: usize = 64;
const WIPE_TILE: u16 = 0x4a;
/// `gfxctrl`'s background and foreground layer enables.
const STAGE_LAYERS: u8 = 0x60;
/// The text cells on screen.
const COLS: std::ops::Range<usize> = VISAREA_X / 8..(VISAREA_X + WIDTH) / 8;
const ROWS: std::ops::Range<usize> = VISAREA_Y / 8..(VISAREA_Y + HEIGHT) / 8;
/// The text rows the gauge corners sit on.
const GAUGE_ROWS: std::ops::RangeInclusive<usize> = 27..=28;

fn word(fight: &Fight, col: usize, row: usize) -> u16 {
    let videoram = &fight.machine().state.videoram;
    let at = 2 * (row * TEXT_COLS + col);
    u16::from_be_bytes([videoram[at], videoram[at + 1]])
}

fn wiped(fight: &Fight, col: usize, row: usize) -> bool {
    word(fight, col, row) & 0x3ff == WIPE_TILE
}

fn cell_pixels(frame: &Frame, col: usize, row: usize) -> Vec<u32> {
    let (x, y) = (col * 8 - VISAREA_X, row * 8 - VISAREA_Y);
    (0..8).flat_map(|dy| (0..8).map(move |dx| (x + dx, y + dy)))
        .map(|(x, y)| frame.argb[y * WIDTH + x])
        .collect()
}

/// The wiped cells of `frame` that show anything but the wipe's black: each
/// is compared with the top cell of its column, where nothing is ever added,
/// when that cell holds the same tile in the same colour.
fn uncovered(fight: &Fight, frame: &Frame) -> Vec<(usize, usize)> {
    let top = ROWS.start;
    let mut cells = Vec::new();
    for col in COLS {
        for row in ROWS.filter(|&row| row != top) {
            if wiped(fight, col, row) && word(fight, col, row) == word(fight, col, top)
                && cell_pixels(frame, col, row) != cell_pixels(frame, col, top)
            {
                cells.push((col, row));
            }
        }
    }
    cells
}

#[test]
fn the_round_wipe_covers_both_era_gauges() {
    let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::Sf6, Loadout::Standard) else {
        return;
    };
    let session = local_fight(&select, Revision::Sfz3, Loadout::VIsm);
    let assets = load_assets(ROMS, MANIFEST).expect("local ROMs assemble");
    let mut fight = session.fight().expect("local fight").clone();
    let hud = fight.era_hud();
    assert!(hud.p1.is_some() && hud.p2.is_some(), "both sides bring an Era Gauge");

    // Run the clock out, then let the native round transition play.
    let mut snapshot = fight.save_netplay_snapshot(0);
    let round = snapshot.machine.work[ROUND_COUNTER];
    let time = (TIME_DIGITS - 0xff8000) as usize;
    snapshot.machine.work[time..time + 3].fill(0);
    fight.restore_netplay_snapshot(&snapshot);

    let (mut sweeping, mut black, mut unstaged) = (0, 0, 0);
    for step in 0..1500 {
        let frame = fight.step(&assets, IDLE, IDLE, false).frame;
        let gauge_cells = || GAUGE_ROWS.flat_map(|row| COLS.map(move |col| (col, row)));
        let covered = gauge_cells().filter(|&(col, row)| wiped(&fight, col, row)).count();
        if covered == gauge_cells().count() {
            black += 1;
        } else if covered > 0 {
            sweeping += 1;
        }
        assert_eq!(
            uncovered(&fight, &frame), [],
            "step {step}: something is drawn over the round wipe (text column, row)",
        );
        // The wipe leaves again before the stage is back: the card's black
        // is the two scroll layers switched off, with no gauge over it.
        if fight.machine().state.gfxctrl & STAGE_LAYERS == 0 {
            let top = ROWS.start;
            let over_black: Vec<_> = gauge_cells()
                .filter(|&(col, row)| !wiped(&fight, col, row) && word(&fight, col, row) == word(&fight, col, top))
                .inspect(|_| unstaged += 1)
                .filter(|&(col, row)| cell_pixels(&frame, col, row) != cell_pixels(&frame, col, top))
                .collect();
            assert_eq!(over_black, [], "step {step}: something is drawn over the round card's black");
        }
        if fight.machine().work_ram()[ROUND_COUNTER] != round && flow_position(fight.machine()) == FIGHT_LOOP {
            break;
        }
    }
    assert_ne!(fight.machine().work_ram()[ROUND_COUNTER], round, "the next round begins");
    assert!(sweeping > 0, "the wipe crossed the gauge rows a column at a time");
    assert!(black > 0, "the wipe held the gauge rows black for the round card");
    assert!(unstaged > 0, "the wipe left the gauge rows before the stage came back");
    let hud = fight.era_hud();
    assert!(hud.p1.is_some() && hud.p2.is_some(), "both gauges are back for the next round");
}
