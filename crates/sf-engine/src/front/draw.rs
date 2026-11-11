//! The framed screen every front-end menu shares: black, the gold double
//! frame of `COUNTRY SELECT` with its header in the 2×2 font across the top
//! edge (or, with no header, unbroken), and a dim hint line at the foot.

use sf_video::compositor::{ink, RomFont, TEXT_COLS};
use sf_video::render::{Frame, HEIGHT, WIDTH};

/// The frame's cells, where `COUNTRY SELECT` puts it (`0x15d42`: text row
/// 4, column 13 is screen cell 5, 2).
pub(crate) const FRAME_COL: usize = 5;
pub(crate) const FRAME_ROW: usize = 2;
pub(crate) const FRAME_COLS: usize = 38;
pub(crate) const FRAME_ROWS: usize = 25;
/// The header's top row: it straddles the frame's top edge, as `COUNTRY
/// SELECT`'s does (`0x15d1e` at text row 3).
pub(crate) const HEADER_ROW: usize = 1;
/// The foot line's row, inside the frame's bottom edge.
pub(crate) const FOOT_ROW: usize = FRAME_ROW + FRAME_ROWS - 2;

pub(crate) const BLACK: u32 = 0xff00_0000;

/// A black frame with the gold frame, `header` centred in the top edge's
/// gap (an empty header leaves the edge unbroken) and `foot`, a screen's
/// hint or nothing, centred at the foot.
pub(crate) fn framed(font: &RomFont<'_>, header: &str, foot: &str) -> Frame {
    let mut argb = vec![BLACK; WIDTH * HEIGHT];
    let width = 2 * header.chars().count();
    let start = FRAME_COLS.saturating_sub(width) / 2;
    font.gold_frame(
        &mut argb,
        FRAME_COL,
        FRAME_ROW,
        FRAME_COLS,
        FRAME_ROWS,
        start..start + width,
    );
    font.header(&mut argb, FRAME_COL + start, HEADER_ROW, header, ink::BLUE);
    font.text_centred(&mut argb, FOOT_ROW, foot, ink::DIM);
    Frame { argb }
}

/// [`framed`] at the screen's full width (columns 1..=46), for a screen
/// whose rows need it (Settings' six tab names).
pub(crate) fn framed_wide(font: &RomFont<'_>, header: &str, foot: &str) -> Frame {
    let mut argb = vec![BLACK; WIDTH * HEIGHT];
    let (col, cols) = (1, TEXT_COLS - 2);
    let width = 2 * header.chars().count();
    let start = cols.saturating_sub(width) / 2;
    font.gold_frame(&mut argb, col, FRAME_ROW, cols, FRAME_ROWS, start..start + width);
    font.header(&mut argb, col + start, HEADER_ROW, header, ink::BLUE);
    font.text_centred(&mut argb, FOOT_ROW, foot, ink::DIM);
    Frame { argb }
}

/// The first cell of `text` centred on the screen.
pub(crate) fn centred_col(text: &str) -> usize {
    TEXT_COLS.saturating_sub(text.chars().count()) / 2
}

/// `text` in at most `cells` cells: as it is when it fits, else cut and
/// ended with `..` (just cut when `cells` has no room for the dots).
pub(crate) fn fit(text: &str, cells: usize) -> String {
    if text.chars().count() <= cells {
        return text.to_owned();
    }
    if cells < 3 {
        return text.chars().take(cells).collect();
    }
    let mut short: String = text.chars().take(cells - 2).collect();
    short.push_str("..");
    short
}

/// `frame` at half brightness: a fight under a box.
pub(crate) fn dimmed(frame: &Frame) -> Frame {
    Frame {
        argb: frame
            .argb
            .iter()
            .map(|&p| 0xff00_0000 | ((p >> 1) & 0x007f_7f7f))
            .collect(),
    }
}

/// A black box of `cols` × `rows` cells at (`col`, `row`) inside the gold
/// frame (no header gap).
pub(crate) fn boxed(frame: &mut Frame, font: &RomFont<'_>, col: usize, row: usize, cols: usize, rows: usize) {
    for y in row * 8..(row + rows) * 8 {
        for x in col * 8..(col + cols) * 8 {
            frame.argb[y * WIDTH + x] = BLACK;
        }
    }
    font.gold_frame(&mut frame.argb, col, row, cols, rows, 0..0);
}
