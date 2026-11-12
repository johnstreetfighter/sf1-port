//! ROM-format text blocks for Arcade. The first twelve slots are the move
//! tables, so this data begins in the next slot of the same bus window.

use super::{DataBank, DATA_BANK_BASE, FIGHTER_SPAN};
use crate::arcade::{ending_epilogue, win_quote_lines};
use crate::roster::TournamentFighter;
use sf_video::compositor::{header_glyph, small_glyph};

const BASE: u32 = DATA_BANK_BASE + FIGHTER_SPAN * TournamentFighter::ALL.len() as u32;
const EPILOGUES: u32 = BASE + 0x20;
const EPILOGUE_SPAN: u32 = 0x400;
const QUOTES: u32 = EPILOGUES + 10 * EPILOGUE_SPAN;
const QUOTE_SPAN: u32 = 0x200;

const ROM_EPILOGUE: usize = 0x44d60;
const ROM_WIN_QUOTE: usize = 0x158e4;
const ROM_FINAL_QUOTE: usize = 0x159b2;
// 0x44d60 begins at text column 17. The right of the composite portrait
// begins at text column 28 (visible column 10 in the 16-pixel bg map).
const EPILOGUE_LEFT_PADDING: usize = 11;

fn loaded(bank: Option<&DataBank>, addr: u32) -> Option<u32> {
    bank.filter(|bank| bank.rb(addr) != 0xff).map(|_| addr)
}

pub(crate) fn stand_in_card(bank: Option<&DataBank>) -> Option<u32> {
    loaded(bank, BASE)
}

pub(crate) fn epilogue(bank: Option<&DataBank>, fighter: TournamentFighter) -> Option<u32> {
    let index = (fighter as u32).checked_sub(2)?;
    loaded(bank, EPILOGUES + index * EPILOGUE_SPAN)
}

pub(crate) fn quote(
    bank: Option<&DataBank>,
    fighter: TournamentFighter,
    finale: bool,
) -> Option<u32> {
    let index = fighter as u32 * 2 + u32::from(finale);
    loaded(bank, QUOTES + index * QUOTE_SPAN)
}

pub(super) fn append(rom: &[u8], bank: &mut Vec<u8>) {
    let Some(epilogue_header) = header(rom, ROM_EPILOGUE, 13, 33) else {
        return;
    };
    let Some(quote_header) = header(rom, ROM_WIN_QUOTE, 5, 36) else {
        return;
    };
    let Some(final_header) = header(rom, ROM_FINAL_QUOTE, 5, 28) else {
        return;
    };

    // The same origin and attribute as the three-letter JOE montage card.
    let ryu = start_block(bank, BASE, &[0x09, 0xb8, 1, 3, 0x90, 0]);
    for ch in "RYU".chars() {
        ryu.extend_from_slice(&header_glyph(ch).expect("ROM header glyph").to_be_bytes());
    }
    for fighter in TournamentFighter::ALL {
        let Some(text) = ending_epilogue(fighter) else {
            continue;
        };
        let index = fighter as u32 - 2;
        let block = start_block(bank, EPILOGUES + index * EPILOGUE_SPAN, epilogue_header);
        let width = usize::from(epilogue_header[3]);
        let lines = wrap(text, width - EPILOGUE_LEFT_PADDING);
        assert!(
            lines.len() <= usize::from(epilogue_header[2]),
            "{fighter:?} epilogue"
        );
        for row in 0..usize::from(epilogue_header[2]) {
            let line = lines.get(row).map_or("", String::as_str);
            write_line(block, line, width, EPILOGUE_LEFT_PADDING);
        }
    }
    for fighter in TournamentFighter::ALL {
        for (finale, header) in [(false, quote_header), (true, final_header)] {
            let index = fighter as u32 * 2 + u32::from(finale);
            let block = start_block(bank, QUOTES + index * QUOTE_SPAN, header);
            let width = usize::from(header[3]);
            let [first, second] = win_quote_lines(fighter, finale);
            for line in [first, "", second, "", ""] {
                assert!(line.len() <= width, "{fighter:?} win quote");
                write_line(block, line, width, (width - line.len()) / 2);
            }
        }
    }
}

fn header(rom: &[u8], addr: usize, rows: u8, cols: u8) -> Option<&[u8]> {
    rom.get(addr..addr + 6)
        .filter(|header| header[2] == rows && header[3] == cols)
}

fn start_block<'a>(bank: &'a mut Vec<u8>, addr: u32, header: &[u8]) -> &'a mut Vec<u8> {
    let offset = (addr - DATA_BANK_BASE) as usize;
    assert!(
        bank.len() <= offset,
        "Arcade text block at {addr:#x} overlaps the bank"
    );
    bank.resize(offset, 0xff);
    bank.extend_from_slice(header);
    bank
}

fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = vec![String::new()];
    for word in text.split_ascii_whitespace() {
        let line = lines.last_mut().expect("first line");
        if !line.is_empty() && line.len() + 1 + word.len() > width {
            lines.push(word.to_owned());
        } else {
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
    }
    lines
}

fn write_line(block: &mut Vec<u8>, line: &str, width: usize, left: usize) {
    let mut blanks = left;
    for ch in line.chars() {
        if ch == ' ' {
            blanks += 1;
        } else {
            write_fill(block, blanks);
            blanks = 0;
            let glyph = small_glyph(ch).expect("ROM text glyph");
            block.extend_from_slice(&glyph.to_be_bytes());
        }
    }
    write_fill(block, blanks + width - left - line.len());
}

fn write_fill(block: &mut Vec<u8>, count: usize) {
    if count != 0 {
        block.extend_from_slice(&0xffffu16.to_be_bytes());
        block.extend_from_slice(&(count as u16).to_be_bytes());
    }
}
