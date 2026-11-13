//! Assemble the private matchup's VS screen from the native Ryu–Sagat view.

use sf_engine::config::EvilRyuPlaytest;
use sf_video::background_bank::{BackgroundBank, LayerView, MapLayer, MAP_ROWS};
use sf_video::render::VISAREA_X;
use sf_video::romset::Assets;
use sf_video::vs_screens::{vs_key, SCREEN_COLUMNS};

pub(crate) fn install(rom: &Assets, bank: &mut BackgroundBank, profile: EvilRyuPlaytest) -> Result<(), String> {
    let stock = bank.view(&vs_key("Ryu", "Sagat")).ok_or("the bank has no Ryu vs Sagat view")?;
    let page = bank.add_page();
    let first = (usize::from(stock.scroll) + VISAREA_X) / 16;
    for col in first..first + SCREEN_COLUMNS {
        for row in 0..MAP_ROWS {
            let cell = bank.cell(stock.page, MapLayer::Bg, col, row);
            bank.set_cell(page, MapLayer::Bg, col, row, cell);
        }
    }

    let view = crate::evil_ryu_art::apply_vs_bust(rom, bank, LayerView { page, ..stock }, profile);
    if profile.is_kage() {
        crate::evil_ryu_vs_name::apply_kage_vs_name(rom, bank, view);
    } else {
        crate::evil_ryu_vs_name::apply_vs_name(rom, bank, view);
    }
    bank.set_view(&vs_key(profile.display_name(), "Sagat"), view);
    Ok(())
}
