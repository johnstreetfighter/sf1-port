//! World Super Turbo timings for Ken's crouching light and medium kick.

use super::{MOVE_HEADER_SIZE, Region, put_era_header, rom_b, rom_record};
use crate::era::{EraFamily, Loadout};
use crate::roster::TournamentFighter::Ken;

const CROUCHING_KICK: u8 = 0x17;

pub(crate) fn crouching_kick_slot(loadout: Loadout, strength: u8) -> u8 {
    match (loadout, strength) {
        (_, 2) => 0x0b,
        (Loadout::Old, _) => 0x0a,
        _ => 0x09,
    }
}

pub(super) fn build(rom: &[u8], region: &mut Region) {
    let source = Ken.move_table() + u32::from(CROUCHING_KICK) * MOVE_HEADER_SIZE;
    let header: [u8; MOVE_HEADER_SIZE as usize] =
        std::array::from_fn(|offset| rom_b(rom, source + offset as u32));

    for (loadout, strength, startup, active, end) in [
        (Loadout::Standard, 1, 4, 4, 13),
        (Loadout::Old, 1, 3, 4, 12),
        (Loadout::Standard, 2, 4, 6, 19),
    ] {
        let mut header = header;
        let mut strike = rom_record(rom, 0x31f4);
        let mut recovery = rom_record(rom, 0x3214);
        header[0xe] = startup - 1;
        strike[1] = active - 1;
        // Keep the native stream's end dispatch in the measured interval.
        recovery[1] = end - startup - active - 2;
        let stream = region.put(&strike);
        region.put(&recovery);
        region.end();
        header[0x2c..0x30].copy_from_slice(&stream.to_be_bytes());
        put_era_header(region, EraFamily::Sf2, crouching_kick_slot(loadout, strength), &header);
    }
}
