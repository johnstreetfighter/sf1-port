use sf_video::background_bank::{BackgroundBank, Cell, LayerView, MapLayer};
use sf_video::decode::decode_sprite_tile;
use sf_video::render::VISAREA_X;
use sf_video::romset::Assets;
use sf_video::vs_screens::vs_key;

const NAME_ROW: usize = 10;
const NAME_COLUMN: usize = 6;
const NAME_HEIGHT: usize = 32;
type NamePixels = [[u8; 64]; NAME_HEIGHT];

fn first_column(view: LayerView) -> usize {
    (usize::from(view.scroll) + VISAREA_X) / 16
}

fn read_plate(rom: &Assets, bank: &BackgroundBank, view: LayerView, column: usize) -> NamePixels {
    let mut pixels = [[0; 64]; NAME_HEIGHT];
    for row in 0..2 {
        for col in 0..4 {
            let cell = bank.cell(view.page, MapLayer::Bg, first_column(view) + column + col, NAME_ROW + row);
            let tile = decode_sprite_tile(&rom.gfx1, usize::from(cell.code));
            for y in 0..16 {
                for x in 0..16 {
                    let sy = if cell.attr & 2 == 0 { y } else { 15 - y };
                    let sx = if cell.attr & 1 == 0 { x } else { 15 - x };
                    pixels[row * 16 + y][col * 16 + x] = tile[sy][sx];
                }
            }
        }
    }
    pixels
}

/// Reuse the original VS lettering: E and L from Eagle, I from Birdie, V
/// from the central V.S., and RYU from Ryu's own plate. A fifth name tile
/// fits the longer word at the native height while retaining the gray rails.
pub(crate) fn apply_vs_name(rom: &Assets, bank: &mut BackgroundBank, view: LayerView) {
    let ryu = read_plate(rom, bank, view, NAME_COLUMN);
    let eagle = read_plate(rom, bank, bank.view(&vs_key("Eagle", "Sagat")).expect("Eagle's native VS view"), NAME_COLUMN);
    let birdie = read_plate(rom, bank, bank.view(&vs_key("Birdie", "Sagat")).expect("Birdie's native VS view"), NAME_COLUMN);
    let versus = read_plate(rom, bank, view, 10);
    // The empty gutter before V.S. has both gray bands and the two rails.
    // Ryu's last column crosses a letter's dark shadow in the upper band.
    let mut plate = std::array::from_fn::<_, NAME_HEIGHT, _>(|y| [versus[y][0]; 80]);
    let mut left = 0;
    for (source, source_x, source_width, width, gap, gold) in [
        (&eagle, 0, 13, 11, 1, false),
        (&versus, 16, 16, 12, 1, true),
        (&birdie, 13, 5, 5, 1, false),
        (&eagle, 40, 12, 11, 3, false),
        (&ryu, 0, 23, 11, 1, false),
        (&ryu, 23, 22, 11, 1, false),
        (&ryu, 45, 18, 11, 0, false),
    ] {
        for y in 8..24 {
            for x in 0..width {
                let pen = source[y][source_x + (2 * x + 1) * source_width / (2 * width)];
                let red = match (gold, pen) {
                    (true, 1) => 8,
                    (true, 2) => 9,
                    (true, 3..=7) => 10,
                    (_, pen) => pen,
                };
                if (8..=10).contains(&red) {
                    plate[y][left + x] = red;
                }
            }
        }
        left += width + gap;
    }

    let first = first_column(view) + NAME_COLUMN;
    for row in 0..2 {
        let bracket = bank.cell(view.page, MapLayer::Bg, first - 1, NAME_ROW + row);
        bank.set_cell(view.page, MapLayer::Bg, first - 2, NAME_ROW + row, bracket);
    }
    write_plate(bank, view, first - 1, &plate);
}

/// Compose KAGE using K from Ken and the matching red letters from Eagle.
pub(crate) fn apply_kage_vs_name(rom: &Assets, bank: &mut BackgroundBank, view: LayerView) {
    let ken_view = bank.view(&vs_key("Ken", "Sagat")).expect("Ken's native VS view");
    let eagle_view = bank.view(&vs_key("Eagle", "Sagat")).expect("Eagle's native VS view");
    let ken = read_plate(rom, bank, ken_view, NAME_COLUMN);
    let eagle = read_plate(rom, bank, eagle_view, NAME_COLUMN);
    let versus = read_plate(rom, bank, view, 10);
    let mut plate = std::array::from_fn::<_, NAME_HEIGHT, _>(|y| [versus[y][0]; 64]);

    for (source, source_x, source_width, left) in [
        (&ken, 0, 23, 4),
        (&eagle, 14, 12, 18),
        (&eagle, 27, 13, 32),
        (&eagle, 0, 13, 46),
    ] {
        for y in 8..24 {
            for x in 0..13 {
                let pen = source[y][source_x + (2 * x + 1) * source_width / 26];
                if (8..=10).contains(&pen) {
                    plate[y][left + x] = pen;
                }
            }
        }
    }
    write_plate(bank, view, first_column(view) + NAME_COLUMN, &plate);
}

fn write_plate<const WIDTH: usize>(
    bank: &mut BackgroundBank,
    view: LayerView,
    start: usize,
    plate: &[[u8; WIDTH]; NAME_HEIGHT],
) {
    let colour = bank.cell(view.page, MapLayer::Bg, first_column(view) + NAME_COLUMN, NAME_ROW).colour;
    for row in 0..2 {
        for col in 0..WIDTH / 16 {
            let tile = std::array::from_fn(|y| std::array::from_fn(|x| plate[16 * row + y][16 * col + x]));
            let code = bank.add_tile(MapLayer::Bg, &tile);
            bank.set_cell(view.page, MapLayer::Bg, start + col, NAME_ROW + row, Cell { code, colour, attr: 0 });
        }
    }
}
