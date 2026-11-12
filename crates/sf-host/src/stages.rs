//! The extended background bank at launch (ADR-0009, tickets #198 and
//! #215): the committed stage records, embedded so the game stays one file,
//! rebuilt from the player's ROM and packed, then the VS and ending screens
//! built from the ROM on the pages after them. Only overhaul fights read
//! the bank.
//! - The test stage, which no screen lists, is picked with
//!   `sf-game --stage test-stage` (a development aid) and in tests. The ten
//!   1987 fight stages come after it, read from the ROM's tables, and the
//!   stage select lists them.
//! - A record that fails to pack is logged and the bank holds no stages,
//!   every stage gate closed; 1987 stages that fail to read are logged and
//!   left out, and no stage select shows; VS screens that fail to build are
//!   logged and left out, the VS gate closed. With none of them the game
//!   runs without the bank.

use sf_video::background_bank::BackgroundBank;
use sf_video::romset::Assets;
use sf_video::stage::pack_records;

use crate::log::Log;

/// Every committed stage record.
pub const RECORDS: [&str; 1] = [include_str!("../../../art/stages/test-stage.json")];

/// The bank built from `rom` and the embedded records, logging what loaded.
pub fn load(rom: &Assets, log: &mut Log) -> Option<BackgroundBank> {
    let mut bank = match pack_records(rom, &RECORDS) {
        Ok(bank) => {
            let names: Vec<&str> = bank.stage_names().collect();
            log.line("stages", &format!("the background bank: {} on {} page(s)", names.join(", "), bank.pages()));
            bank
        }
        Err(e) => {
            log.line("stages", &format!("no stages in the background bank: {e}"));
            BackgroundBank::new()
        }
    };
    match bank.add_rom_stages(rom) {
        Ok(n) => log.line("stages", &format!("{n} 1987 stages on the stage select")),
        Err(e) => log.line("stages", &format!("no 1987 stages in the background bank: {e}")),
    }
    let before = bank.pages();
    match bank.add_vs_screens(rom) {
        Ok(n) => log.line("stages", &format!("{n} VS screens on {} page(s)", bank.pages() - before)),
        Err(e) => log.line("stages", &format!("no VS screens in the background bank: {e}")),
    }
    match bank.add_ending_screens(rom) {
        Ok(n) => log.line("stages", &format!("{n} composite ending views")),
        Err(e) => log.line("stages", &format!("no ending views in the background bank: {e}")),
    }
    (!bank.is_empty()).then_some(bank)
}
