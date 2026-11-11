//! The ten 1987 fight stages as stage records in the extended background
//! bank, read from the ROM's own tables when the game loads (ADR-0009,
//! ticket #193 `ST1`; design section 3).
//!
//! The 1987 game picks a stage by the starting country (`0xffc006`) and the
//! opponent index (stage `0xffc003` x 2 + fight `0xffc004`), from four
//! tables per kind, and each country's table is the Japan table with its
//! opponents rotated to the front. So an opponent has one answer whatever
//! the start, and Japan's tables, the order [`FIGHTERS`] names the bosses
//! in, give all ten:
//!
//! - the foreground and background scroll words (`0x437ee` and `0x437fe`),
//!   the same words `stage_reset` reads; the stage sits on the ROM's page 0;
//! - the stage music's tune command (`0x42358`);
//! - the foreground palette (`0x436e8`) and the background palette
//!   (`0x43778`) of the opponent's country, which the 1987 game picks by
//!   country and stage only, so a country's two opponents share them.
//!
//! The design asks for the tune a start in the opponent's own country
//! gives, and for Adon and Sagat, whose Thailand is never a starting
//! country, the Japan start's. The rotation above makes every start agree
//! (`tests/rom_stages.rs` checks all four), so Japan's tables are those.
//! Palettes stay the ROM's own: a stage's palette pointers are ROM addresses,
//! which the vblank's palette load reads as it does for any 1987 request.

use crate::background_bank::{BackgroundBank, LayerView, StageView};
use crate::busts::FIGHTERS;
use crate::romset::Assets;

/// `0x437ee` / `0x437fe`: one longword per starting country, each pointing
/// to ten words of scroll, one per opponent.
const FG_SCROLLS: usize = 0x437ee;
const BG_SCROLLS: usize = 0x437fe;
/// `0x42358`: one longword per country, each pointing to ten tune commands.
const STAGE_MUSIC: usize = 0x42358;
/// `0x436e8` / `0x43778`: one longword per country, each pointing to five
/// longwords, one per stage, of palette addresses.
const FG_PALETTES: usize = 0x436e8;
const BG_PALETTES: usize = 0x43778;
/// The Japan flag: its tables name the bosses in [`FIGHTERS`] order.
const JAPAN: usize = 0;
/// The first boss in [`FIGHTERS`]: Ryu and Ken have no stage.
const FIRST_OPPONENT: usize = 2;
/// Where each of the five stages is set, in ladder order.
const PLACES: [&str; 5] = ["Japan", "USA", "China", "England", "Thailand"];

fn word(m: &[u8], addr: usize) -> Result<u16, String> {
    match (m.get(addr), m.get(addr + 1)) {
        (Some(&a), Some(&b)) => Ok(u16::from_be_bytes([a, b])),
        _ => Err(format!("{addr:#x} is past the program ROM")),
    }
}

fn long(m: &[u8], addr: usize) -> Result<usize, String> {
    Ok(((word(m, addr)? as usize) << 16) | word(m, addr + 2)? as usize)
}

/// The record name of an opponent's stage: the owner's name in lower case.
pub fn rom_stage_name(owner: &str) -> String {
    owner.to_ascii_lowercase()
}

/// The opponent (0..=9, Retsu to Sagat) the 1987 game sends at ladder step
/// `step` (the stage counter `0xffc003` x 2 + the fight counter `0xffc004`)
/// to a player who started in `country` (`0xffc006`): the opponent whose
/// foreground scroll word Japan's table gives is the one the country's
/// table has at that step (each country lists the same opponents in its own
/// rotation, and a foreground word names one opponent). An error for a
/// country or step the tables do not have.
pub fn ladder_opponent(m: &[u8], country: usize, step: usize) -> Result<usize, String> {
    if country >= 4 || step >= FIGHTERS.len() - FIRST_OPPONENT {
        return Err(format!("no ladder step {step} for country {country}"));
    }
    let table = |c: usize, i: usize| -> Result<u16, String> { word(m, long(m, FG_SCROLLS + 4 * c)? + 2 * i) };
    let here = table(country, step)?;
    let mut found = None;
    for k in 0..FIGHTERS.len() - FIRST_OPPONENT {
        if table(JAPAN, k)? == here {
            if found.is_some() {
                return Err(format!("two opponents share the foreground word {here:#06x}"));
            }
            found = Some(k);
        }
    }
    found.ok_or_else(|| format!("no opponent has the foreground word {here:#06x}"))
}

/// The record name ([`rom_stage_name`]) of the stage [`ladder_opponent`]
/// fights on.
pub fn ladder_stage_name(m: &[u8], country: usize, step: usize) -> Result<String, String> {
    Ok(rom_stage_name(FIGHTERS[FIRST_OPPONENT + ladder_opponent(m, country, step)?]))
}

/// Opponent `k` (0..=9, the 1987 opponent index) of Japan's tables.
pub fn rom_stage(m: &[u8], k: usize) -> Result<StageView, String> {
    let scroll = |tables: usize| -> Result<u16, String> { word(m, long(m, tables + 4 * JAPAN)? + 2 * k) };
    let palette = |tables: usize| -> Result<u32, String> {
        let address = long(m, long(m, tables + 4 * JAPAN)? + 4 * (k / 2))?;
        // A palette is 0x200 bytes in the ROM; one that is not is a bad table.
        m.get(address..address + 0x200).ok_or_else(|| format!("the palette {address:#x} is past the program ROM"))?;
        Ok(address as u32)
    };
    Ok(StageView {
        fg: LayerView { page: 0, scroll: scroll(FG_SCROLLS)?, palette: palette(FG_PALETTES)? },
        bg: LayerView { page: 0, scroll: scroll(BG_SCROLLS)?, palette: palette(BG_PALETTES)? },
        theme: word(m, long(m, STAGE_MUSIC + 4 * JAPAN)? + 2 * k)?,
    })
}

impl BackgroundBank {
    /// Add the ten 1987 fight stages from `rom`'s tables, each named for its
    /// owner and listed on the stage select in the ladder's order (Retsu to
    /// Sagat). Returns how many were added; on an error the bank is as it
    /// was, which includes a stage of the bank already named for an owner
    /// (a record that clashes, or the stages added twice).
    pub fn add_rom_stages(&mut self, rom: &Assets) -> Result<usize, String> {
        let stages = (0..FIGHTERS.len() - FIRST_OPPONENT)
            .map(|k| rom_stage(&rom.maincpu, k))
            .collect::<Result<Vec<_>, _>>()?;
        for owner in &FIGHTERS[FIRST_OPPONENT..] {
            let name = rom_stage_name(owner);
            if self.stage_index(&name).is_some() {
                return Err(format!("the bank already holds a stage named {name}"));
            }
        }
        for (k, view) in stages.into_iter().enumerate() {
            let owner = FIGHTERS[FIRST_OPPONENT + k];
            let index = self.add_stage(&rom_stage_name(owner), view);
            self.list_stage(index, owner, PLACES[k / 2]);
        }
        Ok(FIGHTERS.len() - FIRST_OPPONENT)
    }
}
