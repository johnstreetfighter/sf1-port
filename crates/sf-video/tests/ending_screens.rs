use sf_video::background_bank::{BackgroundBank, Cell, MapLayer, MAP_ROWS};
use sf_video::busts::FIGHTERS;
use sf_video::ending_screens::ending_key;
use sf_video::render::VISAREA_X;
use sf_video::rom_stages::rom_stage;
use sf_video::romset::load_assets;
use std::path::Path;

#[test]
fn all_ten_composites_keep_the_rom_bust_stage_border_and_blank_text_area() {
    let roms = "../../tools/reference/local/roms/sf";
    if !Path::new(roms).is_dir() {
        eprintln!("skipping: local ROMs not present");
        return;
    }
    let rom = load_assets(roms, "../../tools/reference/rom-manifest.json").unwrap();
    let mut bank = BackgroundBank::new();
    assert_eq!(bank.add_ending_screens(&rom), Ok(10));
    assert_eq!(bank.pages(), 1);

    let word = |at: usize| u16::from_be_bytes(rom.maincpu[at..at + 2].try_into().unwrap());
    let vs_table = u32::from_be_bytes(rom.maincpu[0x3fad6..0x3fada].try_into().unwrap()) as usize;
    let visible_column = |scroll| (usize::from(scroll) + VISAREA_X) / 16;
    let read = |first, col, row| {
        Cell::read(
            &rom.tilerom,
            MapLayer::Bg.base(),
            (first + col) * MAP_ROWS + row,
        )
    };

    for (opponent, name) in FIGHTERS[2..].iter().enumerate() {
        let view = bank.view(&ending_key(name)).unwrap();
        let first = visible_column(view.scroll);
        let actual = |col, row| bank.cell(view.page, MapLayer::Bg, first + col, row);
        let vs = visible_column(word(vs_table + opponent * 2));
        let blank = read(vs, 0, 0);
        for col in 4..=9 {
            for row in 3..=8 {
                assert_eq!(actual(col, row), read(vs, col, row), "{name} bust");
            }
        }
        for col in 5..=9 {
            for row in [10, 11] {
                assert_eq!(actual(col, row), read(vs, col, row), "{name} nameplate");
            }
        }
        for col in 10..=22 {
            for row in 0..MAP_ROWS {
                assert_eq!(actual(col, row), blank, "{name} text area");
            }
        }
        let stage = rom_stage(&rom.maincpu, opponent).unwrap().bg;
        assert_eq!(view.palette, stage.palette);
        for row in [1, 2] {
            let cell = read(visible_column(stage.scroll), 0, row);
            assert_eq!(actual(0, row), cell, "{name} top stage border");
            assert_eq!(
                actual(0, MAP_ROWS - 1 - row),
                Cell {
                    attr: cell.attr ^ 2,
                    ..cell
                },
                "{name} bottom stage border"
            );
        }
    }
    assert!(bank.view(&ending_key("Ryu")).is_none());
    assert!(bank.view(&ending_key("Ken")).is_none());
    assert!(bank.add_ending_screens(&rom).is_err());
    assert_eq!(bank.pages(), 1);
}
