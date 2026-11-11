//! The extended tune table (ADR-0009, ticket #195 `ST6`): themes the 1987
//! game does not have, played by the translated FM driver.
//!
//! The ROM's table (0x1011, two bytes a command) names 0x40 commands, and
//! the game sends none past 0x3f. This table takes commands
//! [`EXTENDED_FIRST`]`..=`[`EXTENDED_LAST`]: it sits at [`EXTENSION_BASE`],
//! past the ROM's 0x8000 bytes, 64 little-endian words (0 = no tune), then
//! the tunes it names. A tune is laid out as the ROM's are: the header byte
//! (its priority; bit 6 is the saved tune's silent advance and bit 7 marks
//! an effect, neither used here), eight little-endian stream pointers
//! (0 = the channel is off), then the channels' streams. The driver reads
//! it all through the same code as a ROM tune, so the saved-tune slot, the
//! stops and the win jingle treat it alike.
//!
//! [`ExtendedTunes::program`] puts the table behind a ROM image; a program
//! without it, which is all Faithful 1987 ever runs, has no extended
//! commands (docs/reference/extended-tunes.md).

use std::fmt;

pub use crate::music::{EXTENDED_FIRST, EXTENDED_LAST, EXTENSION_BASE, EXTENSION_END};
use crate::music_pack::MusicPack;
use crate::score::{self, ScoreError};

/// The ROM's length: the extension begins right after it.
pub const ROM_BYTES: usize = EXTENSION_BASE as usize;
/// Commands in the extended table.
const COMMANDS: usize = (EXTENDED_LAST - EXTENDED_FIRST) as usize + 1;
/// Bytes of the table itself.
const TABLE_BYTES: usize = 2 * COMMANDS;
/// Bytes the extension can hold, table included.
pub const CAPACITY: usize = (EXTENSION_END - EXTENSION_BASE) as usize;
/// Channels of a tune.
pub const CHANNELS: usize = 8;

/// One channel's stream, with where its jump targets sit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stream {
    /// The stream's bytes, its jump targets as offsets into itself.
    pub bytes: Vec<u8>,
    /// Where each little-endian jump target starts in `bytes`; placing the
    /// stream adds its address to every one.
    pub jumps: Vec<usize>,
}

/// A tune as the driver plays it: a header and up to eight streams.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tune {
    /// The header byte: the priority in its low nibble.
    pub header: u8,
    /// The channels' streams; `None` leaves a channel off.
    pub channels: [Option<Stream>; CHANNELS],
}

/// Why a tune could not go into the table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TuneError {
    /// The command is not one the table takes.
    Command(u8),
    /// The command already names a tune.
    Taken(u8),
    /// The table has no room for the tune.
    Full { needed: usize, free: usize },
    /// The program image is longer than the ROM, so it already has one.
    NotARom(usize),
}

impl fmt::Display for TuneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TuneError::Command(c) => write!(
                f,
                "command {c:#04x} is not in the extended table's {EXTENDED_FIRST:#04x}..={EXTENDED_LAST:#04x}"
            ),
            TuneError::Taken(c) => write!(f, "command {c:#04x} already names a tune"),
            TuneError::Full { needed, free } => {
                write!(f, "the tune needs {needed} bytes and the table has {free} free")
            }
            TuneError::NotARom(len) => {
                write!(f, "the program image is {len} bytes; the ROM is {ROM_BYTES}")
            }
        }
    }
}

impl std::error::Error for TuneError {}

/// The extended tune table: the extension's bytes from [`EXTENSION_BASE`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtendedTunes {
    image: Vec<u8>,
}

impl Default for ExtendedTunes {
    fn default() -> Self {
        Self::new()
    }
}

impl ExtendedTunes {
    /// An empty table: every extended command names nothing.
    pub fn new() -> Self {
        ExtendedTunes {
            image: vec![0; TABLE_BYTES],
        }
    }

    /// The table and its tunes, as they sit from [`EXTENSION_BASE`].
    pub fn image(&self) -> &[u8] {
        &self.image
    }

    /// The commands that name a tune, lowest first.
    pub fn commands(&self) -> Vec<u8> {
        (0..COMMANDS)
            .filter(|&i| self.image[2 * i] | self.image[2 * i + 1] != 0)
            .map(|i| EXTENDED_FIRST + i as u8)
            .collect()
    }

    /// Where `command`'s tune starts in the driver's address space.
    pub fn address(&self, command: u8) -> Option<u16> {
        let i = command.checked_sub(EXTENDED_FIRST)? as usize;
        if i >= COMMANDS {
            return None;
        }
        let a = u16::from_le_bytes([self.image[2 * i], self.image[2 * i + 1]]);
        (a != 0).then_some(a)
    }

    /// Put `tune` in the table under `command`.
    pub fn add(&mut self, command: u8, tune: &Tune) -> Result<(), TuneError> {
        if !(EXTENDED_FIRST..=EXTENDED_LAST).contains(&command) {
            return Err(TuneError::Command(command));
        }
        if self.address(command).is_some() {
            return Err(TuneError::Taken(command));
        }
        let head = 1 + 2 * CHANNELS;
        let needed = head
            + tune
                .channels
                .iter()
                .flatten()
                .map(|s| s.bytes.len())
                .sum::<usize>();
        let free = CAPACITY - self.image.len();
        if needed > free {
            return Err(TuneError::Full { needed, free });
        }
        let at = EXTENSION_BASE as usize + self.image.len();
        let mut bytes = vec![tune.header];
        let mut streams = Vec::new();
        let mut next = at + head;
        for channel in &tune.channels {
            match channel {
                Some(stream) => {
                    bytes.extend((next as u16).to_le_bytes());
                    let mut placed = stream.bytes.clone();
                    for &j in &stream.jumps {
                        let target = u16::from_le_bytes([placed[j], placed[j + 1]]) + next as u16;
                        placed[j..j + 2].copy_from_slice(&target.to_le_bytes());
                    }
                    next += placed.len();
                    streams.push(placed);
                }
                None => bytes.extend([0, 0]),
            }
        }
        bytes.extend(streams.concat());
        let i = (command - EXTENDED_FIRST) as usize;
        self.image[2 * i..2 * i + 2].copy_from_slice(&(at as u16).to_le_bytes());
        self.image.extend(bytes);
        Ok(())
    }

    /// The program image the music CPU runs when it plays these themes: the
    /// ROM (`rom`, the 0x8000 bytes of sf-02.7k) and the table behind it.
    pub fn program(&self, rom: &[u8]) -> Result<Vec<u8>, TuneError> {
        if rom.len() > ROM_BYTES {
            return Err(TuneError::NotARom(rom.len()));
        }
        let mut program = rom.to_vec();
        program.resize(ROM_BYTES, 0xff);
        program.extend_from_slice(&self.image);
        Ok(program)
    }
}

/// The scores committed with the game: the test theme (`art/themes`), which
/// has no Capcom melody.
pub const COMMITTED: [&str; 1] = [include_str!("../../../art/themes/test-theme.score")];

/// The table the committed scores make, converted against the ROM.
pub fn committed(rom: &[u8]) -> Result<ExtendedTunes, ScoreError> {
    let mut tunes = ExtendedTunes::new();
    for text in COMMITTED {
        score::convert(text, rom)?.add_to(&mut tunes)?;
    }
    Ok(tunes)
}

/// Add the pack's scores to `tunes`, converted against the ROM. A score that
/// does not convert, or asks for a command that is taken, is left out; each
/// such score is one message naming its file, and the rest are added.
pub fn add_pack(tunes: &mut ExtendedTunes, rom: &[u8], pack: &MusicPack) -> Vec<String> {
    let mut left_out = Vec::new();
    for s in &pack.scores {
        let added = score::convert(&s.text, rom)
            .map_err(|e| e.to_string())
            .and_then(|c| c.add_to(tunes).map_err(|e| e.to_string()));
        if let Err(why) = added {
            left_out.push(format!("{}: {why}", s.file));
        }
    }
    left_out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tune(header: u8, bytes: &[u8], jumps: &[usize]) -> Tune {
        let mut channels: [Option<Stream>; CHANNELS] = Default::default();
        channels[0] = Some(Stream {
            bytes: bytes.to_vec(),
            jumps: jumps.to_vec(),
        });
        Tune { header, channels }
    }

    #[test]
    fn a_tune_is_laid_out_like_the_roms_and_its_jumps_are_placed() {
        let mut t = ExtendedTunes::new();
        // A stream whose jump word, at 2, points back at its own start.
        t.add(0x41, &tune(8, &[0xc, 0, 0, 0, 0xf], &[2])).unwrap();
        let at = t.address(0x41).unwrap();
        assert_eq!(at, EXTENSION_BASE + TABLE_BYTES as u16);
        assert_eq!(t.address(0x40), None);
        assert_eq!(t.commands(), vec![0x41]);
        let off = (at - EXTENSION_BASE) as usize;
        assert_eq!(t.image()[off], 8, "header");
        let stream = u16::from_le_bytes([t.image()[off + 1], t.image()[off + 2]]);
        assert_eq!(stream, at + 17, "after the header and eight pointers");
        assert_eq!(&t.image()[off + 3..off + 17], &[0; 14], "seven channels off");
        let body = (stream - EXTENSION_BASE) as usize;
        assert_eq!(&t.image()[body..body + 2], &[0xc, 0]);
        assert_eq!(u16::from_le_bytes([t.image()[body + 2], t.image()[body + 3]]), stream);
    }

    #[test]
    fn the_table_refuses_what_it_cannot_hold() {
        let mut t = ExtendedTunes::new();
        let small = tune(8, &[0xf], &[]);
        assert_eq!(t.add(0x3f, &small), Err(TuneError::Command(0x3f)));
        assert_eq!(t.add(0x80, &small), Err(TuneError::Command(0x80)));
        t.add(0x40, &small).unwrap();
        assert_eq!(t.add(0x40, &small), Err(TuneError::Taken(0x40)));
        let big = tune(8, &vec![0xf; CAPACITY], &[]);
        assert!(matches!(t.add(0x41, &big), Err(TuneError::Full { .. })));
    }

    #[test]
    fn the_program_is_the_rom_with_the_table_behind_it() {
        let mut t = ExtendedTunes::new();
        t.add(0x40, &tune(8, &[0xf], &[])).unwrap();
        let rom = vec![0x11; ROM_BYTES];
        let p = t.program(&rom).unwrap();
        assert_eq!(&p[..ROM_BYTES], &rom[..]);
        assert_eq!(&p[ROM_BYTES..], t.image());
        assert_eq!(t.program(&p), Err(TuneError::NotARom(p.len())));
    }
}
