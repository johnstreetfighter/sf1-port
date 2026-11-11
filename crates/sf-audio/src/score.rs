//! The score converter (ADR-0009, ticket #195 `ST6`): a readable score in,
//! the music driver's channel streams out, and a clear message for anything
//! the driver cannot play. The format and the driver's limits it enforces
//! are in docs/reference/extended-tunes.md.
//!
//! A score is words separated by white space; `;` starts a comment.
//!
//! ```text
//! title    Test theme
//! command  0x40            ; 0x40..=0x7f, the extended table's commands
//! priority 8               ; 1..=15, optional (8, like the 1987 themes)
//! unit     10              ; sequencer ticks (3 ms) a 1/32 note lasts, halved
//!
//! channel 0                ; the YM2151 channel, 0..=7
//!   patch 23               ; one of the ROM's patches
//!   gate 200               ; 0..=255, optional (200)
//!   pan both               ; left, right or both, optional (both)
//!   C4:8 E4 G4:4. r:8      ; notes; a length stays until the next one
//!   repeat 2 { A3:4+8 }    ; a block played twice; blocks nest three deep
//!   forever { C4:2 }       ; the rest of the channel, repeated for ever
//! ```
//!
//! A channel ends with `end` (the tune stops) or with a `forever` block.
//!
//! The pitch a legacy note byte carries is relative: the driver adds the channel's
//! transpose and the patch's own, so the converter reads the patch's
//! transpose from the ROM and sets the channel's whenever a note would fall
//! outside the 31 semitones a byte reaches.
//!
//! Explicit `voice` channels instead use raw YM2151 KC/KF, elapsed-clock
//! spans, independent gates and per-voice tone controls. They compile to
//! extension-only events in the same table; legacy channels keep their
//! original bytes and timing. See docs/reference/extended-tunes.md.

use std::collections::BTreeSet;
use std::fmt;

use crate::tunes::{ExtendedTunes, Stream, Tune, TuneError, CHANNELS, EXTENDED_FIRST, EXTENDED_LAST, ROM_BYTES};

/// Where the ROM's patches start, their size, and how many the ROM holds.
const PATCH_BASE: usize = 0x6b00;
const PATCH_BYTES: usize = 0x29;
const PATCHES: usize = 128;
/// The key table runs from index 1 (C#0) to 96 (C8); index 0 is no note.
const LOWEST: i32 = 1;
const HIGHEST: i32 = 96;
/// The most notes a tie can join: its count is a nibble.
const MOST_TIED: usize = 15;
/// Repeat counters the driver keeps per channel (opcodes 0xc, 0xd, 0xe).
const MOST_NESTED: usize = 3;
/// What a channel does without a `gate` or `pan` statement.
const DEFAULT_GATE: u8 = 200;
const DEFAULT_PRIORITY: u8 = 8;

/// A score the driver cannot play, and where.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScoreError {
    /// The line of the score, 1-based; 0 when no line is to blame.
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ScoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            f.write_str(&self.message)
        } else {
            write!(f, "line {}: {}", self.line, self.message)
        }
    }
}

impl std::error::Error for ScoreError {}

impl From<TuneError> for ScoreError {
    fn from(e: TuneError) -> Self {
        ScoreError {
            line: 0,
            message: e.to_string(),
        }
    }
}

fn fail<T>(line: usize, message: impl Into<String>) -> Result<T, ScoreError> {
    Err(ScoreError {
        line,
        message: message.into(),
    })
}

/// A converted score: the tune, and the command it asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Converted {
    pub title: String,
    pub command: u8,
    pub tune: Tune,
}

impl Converted {
    /// Put the tune in `tunes` under its command.
    pub fn add_to(&self, tunes: &mut ExtendedTunes) -> Result<(), TuneError> {
        tunes.add(self.command, &self.tune)
    }
}

/// One word of the score and its line.
struct Word<'a> {
    text: &'a str,
    line: usize,
}

fn words(text: &str) -> Vec<Word<'_>> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let code = line.split(';').next().unwrap_or("");
        let mut start = None;
        for (at, c) in code.char_indices() {
            let brace = c == '{' || c == '}';
            if c.is_whitespace() || brace {
                if let Some(s) = start.take() {
                    out.push(Word { text: &code[s..at], line: i + 1 });
                }
                if brace {
                    out.push(Word { text: &code[at..at + 1], line: i + 1 });
                }
            } else if start.is_none() {
                start = Some(at);
            }
        }
        if let Some(s) = start {
            out.push(Word { text: &code[s..], line: i + 1 });
        }
    }
    out
}

fn number(w: &Word, what: &str, lo: u32, hi: u32) -> Result<u32, ScoreError> {
    let t = w.text;
    let parsed = match t.strip_prefix("0x") {
        Some(hex) => u32::from_str_radix(hex, 16),
        None => t.parse(),
    };
    match parsed {
        Ok(n) if (lo..=hi).contains(&n) => Ok(n),
        Ok(n) => fail(w.line, format!("{what} {n} is outside {lo}..={hi}")),
        Err(_) => fail(w.line, format!("{what} must be a number, not `{t}`")),
    }
}

/// The word after `w`, which `w` needs (`what`).
fn take<'a, 'b>(all: &'a [Word<'b>], at: &mut usize, w: &Word, what: &str) -> Result<&'a Word<'b>, ScoreError> {
    let a = all.get(*at);
    *at += 1;
    match a {
        Some(a) => Ok(a),
        None => fail(w.line, format!("`{}` needs {what}", w.text)),
    }
}

/// The ROM's patch `n` must be one it holds and one that is not blank; its
/// transpose (the first byte, signed) is returned.
fn patch_transpose(rom: &[u8], n: u32, line: usize) -> Result<i32, ScoreError> {
    if n as usize >= PATCHES {
        return fail(line, format!("patch {n} is past the ROM's {PATCHES} patches"));
    }
    let at = PATCH_BASE + PATCH_BYTES * n as usize;
    let patch = &rom[at..at + PATCH_BYTES];
    if patch.iter().all(|&b| b == 0) {
        return fail(line, format!("patch {n} is blank in the ROM, so it would be silent"));
    }
    Ok(patch[0] as i8 as i32)
}

/// A note's length: one of the driver's two length tables' entries.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Part {
    /// 7 for a whole note down to 2 for a 1/32.
    index: u8,
    dotted: bool,
}

fn parts(w: &Word, spec: &str) -> Result<Vec<Part>, ScoreError> {
    let mut out = Vec::new();
    for p in spec.split('+') {
        let (digits, dotted) = match p.strip_suffix('.') {
            Some(d) => (d, true),
            None => (p, false),
        };
        let index = match digits {
            "1" => 7,
            "2" => 6,
            "4" => 5,
            "8" => 4,
            "16" => 3,
            "32" => 2,
            _ => {
                return fail(
                    w.line,
                    format!("length `{p}` in `{}`: the driver has 1, 2, 4, 8, 16 and 32, each with an optional dot", w.text),
                )
            }
        };
        out.push(Part { index, dotted });
    }
    if out.len() > MOST_TIED {
        return fail(w.line, format!("`{}` ties {} lengths; the driver ties at most {MOST_TIED}", w.text, out.len()));
    }
    Ok(out)
}

/// A note name's key-table index (C#0 = 1 ... C8 = 96), or `None` for a
/// rest; `Err` when `text` is neither.
fn pitch(w: &Word, name: &str) -> Result<Option<i32>, ScoreError> {
    let mut chars = name.chars();
    let letter = chars.next().unwrap_or(' ');
    if letter == 'r' || letter == 'R' {
        return match chars.next() {
            None => Ok(None),
            Some(_) => fail(w.line, format!("`{}` is not a rest; a rest is a bare `r`", w.text)),
        };
    }
    let semitone = match letter.to_ascii_uppercase() {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return fail(w.line, format!("`{}` is not a statement, a note or a rest", w.text)),
    };
    let rest: String = chars.collect();
    let (shift, octave) = match rest.strip_prefix('#') {
        Some(o) => (1, o),
        None => match rest.strip_prefix('b') {
            Some(o) => (-1, o),
            None => (0, rest.as_str()),
        },
    };
    let octave: i32 = match octave.parse() {
        Ok(o) if (0..=8).contains(&o) => o,
        _ => return fail(w.line, format!("`{}` has no octave 0 to 8 after its note name", w.text)),
    };
    let index = 12 * octave + semitone + shift;
    if !(LOWEST..=HIGHEST).contains(&index) {
        return fail(w.line, format!("`{name}` is outside the driver's key table, C#0 to C8"));
    }
    Ok(Some(index))
}

/// An open `repeat` or `forever` block.
struct Block {
    forever: bool,
    /// The repeat count byte: one less than the plays.
    count: u8,
    start: usize,
    line: usize,
    /// The channel's patch, gate and pan on entry, put back at the end so
    /// the next pass starts as the first did.
    entry: (Option<(u8, i32)>, Option<u8>, Option<u8>, Option<[u8; 4]>),
    notes: bool,
}

/// One channel's stream as it is written.
struct Channel {
    number: usize,
    bytes: Vec<u8>,
    jumps: Vec<usize>,
    /// The transpose byte in force, once known.
    transpose: Option<u8>,
    patch: Option<(u8, i32)>,
    voice: Option<[u8; 4]>,
    gate: Option<u8>,
    pan: Option<u8>,
    length: Option<Vec<Part>>,
    blocks: Vec<Block>,
    ended: bool,
    /// The driver's tie counter may still be 1: a tie leaves it there for
    /// the next plain event to use up.
    stale: bool,
}

impl Channel {
    fn new(number: usize, unit: u8) -> Self {
        Channel {
            number,
            bytes: vec![0x00, unit],
            jumps: Vec::new(),
            transpose: None,
            patch: None,
            voice: None,
            gate: None,
            pan: None,
            length: None,
            blocks: Vec::new(),
            ended: false,
            stale: false,
        }
    }

    fn set_patch(&mut self, n: u8, transpose: i32) {
        self.bytes.extend([0x07, n]);
        self.patch = Some((n, transpose));
    }

    fn set_voice(&mut self, voice: [u8; 4], transpose: i32) {
        self.bytes.extend([0x0a, 0]);
        self.bytes.extend(voice);
        self.patch = Some((voice[0], transpose));
        self.voice = Some(voice);
    }

    fn raw_ready(&mut self, line: usize) -> Result<(), ScoreError> {
        if self.voice.is_none() {
            return fail(line, "`raw`, `pitch`, `wait` and `off` need a `voice` first");
        }
        self.ready(line)
    }

    fn timed(&mut self) {
        if let Some(block) = self.blocks.last_mut() {
            block.notes = true;
        }
    }

    fn set_gate(&mut self, g: u8) {
        self.bytes.extend([0x02, g]);
        self.gate = Some(g);
    }

    fn set_pan(&mut self, p: u8) {
        self.bytes.extend([0x09, p]);
        self.pan = Some(p);
    }

    /// Before a note or a block: the patch must be chosen, and the gate and
    /// pan take their defaults, so a block's entry state is complete.
    fn ready(&mut self, line: usize) -> Result<(), ScoreError> {
        if self.patch.is_none() {
            return fail(line, format!("channel {} needs a `patch` before its first note or block", self.number));
        }
        if self.gate.is_none() && self.voice.is_none() {
            self.set_gate(DEFAULT_GATE);
        }
        if self.pan.is_none() {
            self.set_pan(3);
        }
        Ok(())
    }

    /// A note or rest event: the length table picks the opcode 6 form for a
    /// dotted length. Opcode 6 skips the tie counter's count-down (0x08ba
    /// jumps past 0x04e2), so a dotted event after a tie would be taken for
    /// a tied one and lose its key-on or key-off. When the counter may still
    /// be set, `0x20` first clears it for the event after it.
    fn event(&mut self, part: Part, pitch: u8) {
        let byte = (part.index << 5) | pitch;
        if part.dotted {
            if self.stale {
                self.bytes.push(0x20);
                self.stale = false;
            }
            self.bytes.push(0x06);
        } else {
            self.stale = false;
        }
        self.bytes.push(byte);
    }

    fn note(&mut self, w: &Word, name: &str, spec: Option<&str>) -> Result<(), ScoreError> {
        if self.voice.is_some() {
            return fail(w.line, "a `voice` channel uses `raw` and `wait`, not legacy note lengths");
        }
        let target = pitch(w, name)?;
        if let Some(spec) = spec {
            self.length = Some(parts(w, spec)?);
        }
        let Some(length) = self.length.clone() else {
            return fail(w.line, format!("`{}` has no length, and none came before it", w.text));
        };
        self.ready(w.line)?;
        if let Some(b) = self.blocks.last_mut() {
            b.notes = true;
        }
        match target {
            None => {
                // Rests are not tied: each length is a rest of its own.
                for part in length {
                    self.event(part, 0);
                }
            }
            Some(t) => {
                let (_, shift) = self.patch.expect("ready() chose a patch");
                let reach = |transpose: u8| {
                    let p = (t - shift - transpose as i32).rem_euclid(256);
                    (1..=31).contains(&p).then_some(p as u8)
                };
                let p = match self.transpose.and_then(reach) {
                    Some(p) => p,
                    None => {
                        // Centre the byte's 31 semitones on this note.
                        let transpose = (t - shift - 16).rem_euclid(256) as u8;
                        self.bytes.extend([0x03, transpose]);
                        self.transpose = Some(transpose);
                        reach(transpose).expect("a centred transpose reaches its note")
                    }
                };
                if length.len() > 1 {
                    // Inside a tie a dotted length is two plain ones: opcode
                    // 6 would not count the tie down.
                    let mut plain = Vec::new();
                    for part in length {
                        if !part.dotted {
                            plain.push(part);
                        } else if part.index == 2 {
                            return fail(w.line, format!("`{}` ties a dotted 1/32, which would need a 1/64 the driver does not have", w.text));
                        } else {
                            plain.push(Part { index: part.index, dotted: false });
                            plain.push(Part { index: part.index - 1, dotted: false });
                        }
                    }
                    if plain.len() > MOST_TIED {
                        return fail(w.line, format!("`{}` ties {} lengths once its dots are written out; the driver ties at most {MOST_TIED}", w.text, plain.len()));
                    }
                    self.bytes.push(0x20 | plain.len() as u8);
                    for part in plain {
                        self.event(part, p);
                    }
                    self.stale = true;
                } else {
                    self.event(length[0], p);
                }
            }
        }
        Ok(())
    }

    fn open(&mut self, line: usize, forever: bool, count: u8) -> Result<(), ScoreError> {
        self.ready(line)?;
        if !forever && self.blocks.iter().filter(|b| !b.forever).count() >= MOST_NESTED {
            return fail(line, format!("repeats nest at most {MOST_NESTED} deep: the driver keeps three counters per channel"));
        }
        self.blocks.push(Block {
            forever,
            count,
            start: self.bytes.len(),
            line,
            entry: (self.patch, self.gate, self.pan, self.voice),
            notes: false,
        });
        // The pass that comes back here has not seen what the last set, and
        // may arrive with a tie counter still set.
        self.transpose = None;
        self.stale = true;
        Ok(())
    }

    fn close(&mut self, line: usize, rom: &[u8]) -> Result<(), ScoreError> {
        let Some(block) = self.blocks.pop() else {
            return fail(line, "`}` closes no block");
        };
        if !block.notes {
            return fail(
                block.line,
                "this block has no notes: a pass through it would take no time and the driver would never return from it",
            );
        }
        // Put the entry state back so the jump back starts as the first pass.
        let (patch, gate, pan, voice) = block.entry;
        if let Some(voice) = voice {
            if self.voice != Some(voice) {
                self.set_voice(voice, patch_transpose(rom, voice[0] as u32, line)?);
            }
        } else if self.patch != patch {
            let (n, _) = patch.expect("a block is opened with a patch");
            self.set_patch(n, patch_transpose(rom, n as u32, line)?);
            self.transpose = None;
        }
        if self.gate != gate {
            self.set_gate(gate.expect("a block is opened with a gate"));
        }
        if self.pan != pan {
            self.set_pan(pan.expect("a block is opened with a pan"));
        }
        let depth = self.blocks.iter().filter(|b| !b.forever).count();
        let opcode = if block.forever { 0x0c } else { 0x0c + depth as u8 };
        self.bytes.extend([opcode, block.count]);
        self.jumps.push(self.bytes.len());
        self.bytes.extend((block.start as u16).to_le_bytes());
        if let Some(parent) = self.blocks.last_mut() {
            parent.notes = true;
        }
        if block.forever {
            self.ended = true;
        }
        Ok(())
    }
}

/// Convert the score `text` for the program `rom` (sf-02.7k, whose patches
/// the score names).
pub fn convert(text: &str, rom: &[u8]) -> Result<Converted, ScoreError> {
    if rom.len() < ROM_BYTES {
        return fail(0, format!("the ROM image is {} bytes; the patches need all {ROM_BYTES}", rom.len()));
    }
    let all = words(text);
    let mut at = 0;
    let mut title = None::<String>;
    let (mut command, mut priority, mut unit) = (None, None, None);
    let mut channels: [Option<Channel>; CHANNELS] = Default::default();
    let mut current: Option<usize> = None;
    let mut seen = BTreeSet::new();
    while at < all.len() {
        let w = &all[at];
        at += 1;
        match w.text {
            "title" | "command" | "priority" | "unit" if current.is_some() => {
                return fail(w.line, format!("`{}` belongs before the first channel", w.text));
            }
            "title" => {
                let line = w.line;
                let mut name = Vec::new();
                while let Some(n) = all.get(at).filter(|n| n.line == line) {
                    name.push(n.text);
                    at += 1;
                }
                if title.replace(name.join(" ")).is_some() {
                    return fail(line, "`title` is given twice");
                }
            }
            "command" => {
                let a = take(&all, &mut at, w, "a command")?;
                let c = number(a, "command", EXTENDED_FIRST as u32, EXTENDED_LAST as u32)?;
                if command.replace(c as u8).is_some() {
                    return fail(w.line, "`command` is given twice");
                }
            }
            "priority" => {
                let a = take(&all, &mut at, w, "a priority")?;
                let p = number(a, "priority", 1, 15)?;
                if priority.replace(p as u8).is_some() {
                    return fail(w.line, "`priority` is given twice");
                }
            }
            "unit" => {
                let a = take(&all, &mut at, w, "a number of ticks")?;
                let u = number(a, "unit", 1, 255)?;
                if unit.replace(u as u8).is_some() {
                    return fail(w.line, "`unit` is given twice");
                }
            }
            "channel" => {
                if let Some(done) = current.take() {
                    finish(channels[done].as_ref().expect("a channel is open"))?;
                }
                let a = take(&all, &mut at, w, "a channel number")?;
                let n = number(a, "channel", 0, CHANNELS as u32 - 1)? as usize;
                if !seen.insert(n) {
                    return fail(w.line, format!("channel {n} is given twice"));
                }
                let Some(unit) = unit else {
                    return fail(w.line, "`unit` must come before the first channel");
                };
                channels[n] = Some(Channel::new(n, unit));
                current = Some(n);
            }
            _ => {
                let Some(n) = current else {
                    return fail(w.line, format!("`{}` is not a header (title, command, priority, unit) or a `channel`", w.text));
                };
                let ch = channels[n].as_mut().expect("a channel is open");
                if ch.ended {
                    return fail(w.line, format!("`{}` follows the end of channel {n}; nothing plays past `end` or a `forever` block", w.text));
                }
                match w.text {
                    "voice" => {
                        if ch.length.is_some() || (ch.voice.is_none() && !ch.blocks.is_empty()) {
                            return fail(w.line, "choose `voice` before any legacy notes or blocks; the two timing forms cannot mix");
                        }
                        let mut voice = [0u8; 4];
                        for (i, name) in ["patch", "carrier TL steps", "modulator TL steps"].into_iter().enumerate() {
                            let a = take(&all, &mut at, w, name)?;
                            voice[i] = number(a, name, 0, 127)? as u8;
                        }
                        let a = take(&all, &mut at, w, "feedback 0..7 or native")?;
                        voice[3] = if a.text == "native" { 255 } else { number(a, "feedback", 0, 7)? as u8 };
                        let transpose = patch_transpose(rom, voice[0] as u32, w.line)?;
                        ch.set_voice(voice, transpose);
                    }
                    "raw" => {
                        ch.raw_ready(w.line)?;
                        let mut v = [0u16; 4];
                        for (i, (name, lo, hi)) in [("KC", 0, 127), ("KF", 0, 255), ("duration", 1, 65535), ("gate", 0, 65535)].into_iter().enumerate() {
                            let a = take(&all, &mut at, w, name)?;
                            v[i] = number(a, name, lo, hi)? as u16;
                        }
                        ch.bytes.extend([0x0a, 1, v[0] as u8, v[1] as u8]);
                        ch.bytes.extend(v[2].to_le_bytes());
                        ch.bytes.extend(v[3].to_le_bytes());
                        ch.timed();
                    }
                    "pitch" => {
                        ch.raw_ready(w.line)?;
                        let kc = take(&all, &mut at, w, "KC")?;
                        let kc = number(kc, "KC", 0, 127)? as u8;
                        let kf = take(&all, &mut at, w, "KF")?;
                        let kf = number(kf, "KF", 0, 255)? as u8;
                        ch.bytes.extend([0x0a, 2, kc, kf]);
                    }
                    "wait" => {
                        ch.raw_ready(w.line)?;
                        let a = take(&all, &mut at, w, "a duration")?;
                        let duration = number(a, "duration", 1, 65535)? as u16;
                        ch.bytes.extend([0x0a, 3]);
                        ch.bytes.extend(duration.to_le_bytes());
                        ch.timed();
                    }
                    "off" => {
                        ch.raw_ready(w.line)?;
                        ch.bytes.extend([0x0a, 4]);
                    }
                    "patch" => {
                        if ch.voice.is_some() {
                            return fail(w.line, "a `voice` channel changes its patch with `voice`, not `patch`");
                        }
                        let a = take(&all, &mut at, w, "a patch number")?;
                        let p = number(a, "patch", 0, 255)?;
                        let transpose = patch_transpose(rom, p, a.line)?;
                        ch.set_patch(p as u8, transpose);
                        ch.transpose = None;
                    }
                    "gate" => {
                        if ch.voice.is_some() {
                            return fail(w.line, "a `voice` channel gives its gate explicitly in `raw`");
                        }
                        let a = take(&all, &mut at, w, "a number")?;
                        let g = number(a, "gate", 0, 255)?;
                        ch.set_gate(g as u8);
                    }
                    "pan" => {
                        let a = take(&all, &mut at, w, "left, right or both")?;
                        let p = match a.text {
                            "left" => 1,
                            "right" => 2,
                            "both" => 3,
                            other => return fail(a.line, format!("pan `{other}` is not left, right or both")),
                        };
                        ch.set_pan(p);
                    }
                    "repeat" => {
                        let a = take(&all, &mut at, w, "a count")?;
                        let times = number(a, "repeat count", 2, 256)?;
                        let brace = take(&all, &mut at, w, "`{`")?;
                        if brace.text != "{" {
                            return fail(brace.line, format!("`repeat {times}` must be followed by `{{`, not `{}`", brace.text));
                        }
                        ch.open(w.line, false, (times - 1) as u8)?;
                    }
                    "forever" => {
                        if !ch.blocks.is_empty() {
                            return fail(w.line, "`forever` must be at the channel's top level, not inside a block");
                        }
                        let brace = take(&all, &mut at, w, "`{`")?;
                        if brace.text != "{" {
                            return fail(brace.line, format!("`forever` must be followed by `{{`, not `{}`", brace.text));
                        }
                        ch.open(w.line, true, 0)?;
                    }
                    "}" => ch.close(w.line, rom)?,
                    "end" => {
                        if !ch.blocks.is_empty() {
                            return fail(w.line, "`end` must be at the channel's top level, not inside a block");
                        }
                        ch.bytes.push(0x0f);
                        ch.ended = true;
                    }
                    "{" => return fail(w.line, "`{` opens nothing; write `repeat N {` or `forever {`"),
                    note => {
                        let (name, spec) = match note.split_once(':') {
                            Some((name, spec)) => (name, Some(spec)),
                            None => (note, None),
                        };
                        ch.note(w, name, spec)?;
                    }
                }
            }
        }
    }
    if let Some(done) = current {
        finish(channels[done].as_ref().expect("a channel is open"))?;
    }
    let Some(command) = command else {
        return fail(0, "the score has no `command`");
    };
    if seen.is_empty() {
        return fail(0, "the score has no channel");
    }
    let tune = Tune {
        header: priority.unwrap_or(DEFAULT_PRIORITY),
        channels: channels.map(|c| {
            c.map(|c| Stream {
                bytes: c.bytes,
                jumps: c.jumps,
            })
        }),
    };
    Ok(Converted {
        title: title.unwrap_or_default(),
        command,
        tune,
    })
}

/// A channel is whole when its blocks are closed and it ends.
fn finish(ch: &Channel) -> Result<(), ScoreError> {
    if let Some(b) = ch.blocks.last() {
        return fail(b.line, "this block is never closed with `}`");
    }
    if !ch.ended {
        return fail(
            0,
            format!("channel {} has no `end` and no `forever` block, so the driver would read on past its last byte", ch.number),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A ROM image whose patch 5 has transpose -12, patch 6 is 0, and
    /// whose others are blank.
    fn rom() -> Vec<u8> {
        let mut rom = vec![0; ROM_BYTES];
        for (n, transpose) in [(5usize, 0xf4u8), (6, 0x00)] {
            let at = PATCH_BASE + PATCH_BYTES * n;
            rom[at] = transpose;
            rom[at + 6] = 0x3d;
        }
        rom
    }

    fn bytes(score: &str) -> Vec<u8> {
        let c = convert(score, &rom()).unwrap();
        c.tune.channels[0].clone().unwrap().bytes
    }

    fn error(score: &str) -> String {
        convert(score, &rom()).unwrap_err().to_string()
    }

    const HEAD: &str = "command 0x40\nunit 10\nchannel 0\n";

    #[test]
    fn explicit_voice_spans_preserve_raw_pitch_and_independent_gate() {
        let b = bytes(&format!("{HEAD} voice 6 13 12 native pan both raw 0x79 236 1000 2000 pitch 0x78 16 wait 500 off end"));
        assert_eq!(b, [0,10, 0x0a,0,6,13,12,255, 9,3,
            0x0a,1,0x79,236,0xe8,3,0xd0,7,
            0x0a,2,0x78,16, 0x0a,3,0xf4,1, 0x0a,4, 15]);
    }

    #[test]
    fn explicit_spans_reject_invalid_controls_and_mixed_timing() {
        for body in ["voice 6 128 0 native", "voice 6 0 0 8", "voice 6 0 0 native raw 128 0 1 1",
            "voice 6 0 0 native raw 0x40 256 1 1", "voice 6 0 0 native raw 0x40 0 0 1",
            "voice 6 0 0 native wait 0", "raw 0x40 0 1 0", "voice 6 0 0 native C4:4",
            "voice 6 0 0 native patch 5", "voice 6 0 0 native gate 200",
            "patch 6 C4:4 voice 6 0 0 native"] {
            assert!(convert(&format!("{HEAD} {body} end"), &rom()).is_err(), "{body}");
        }
    }

    #[test]
    fn explicit_loop_restores_voice_without_adding_note_or_gate_events() {
        let b = bytes(&format!("{HEAD} voice 6 4 12 5 forever {{ raw 0x4e 236 1000 0 voice 5 24 0 native wait 200 }}"));
        assert_eq!(&b[b.len()-10..], [0x0a,0,6,4,12,5,0x0c,0,10,0]);
    }

    #[test]
    fn a_channel_opens_with_its_unit_gate_patch_and_pan_and_ends() {
        // Patch 6 (transpose 0): C4 is key index 48; the byte centres it on 16.
        let b = bytes(&format!("{HEAD} patch 6\n C4:4 end"));
        assert_eq!(b, [0x00, 10, 0x07, 6, 0x02, 200, 0x09, 3, 0x03, 32, 0xb0, 0x0f]);
    }

    #[test]
    fn the_patchs_transpose_and_each_notes_reach_set_the_channel_transpose() {
        // Patch 5 transposes by -12, so C4 (48) needs 48 + 12 - 16 = 44; D4
        // (50) is 18 above it, still in reach; C6 (72) is not and re-centres.
        let b = bytes(&format!("{HEAD} patch 5 gate 100 pan left\n C4:8 D4 C6 end"));
        assert_eq!(
            b,
            [0, 10, 7, 5, 2, 100, 9, 1, 3, 44, 0x80 | 16, 0x80 | 18, 3, 68, 0x80 | 16, 0x0f]
        );
    }

    #[test]
    fn lengths_dots_ties_and_rests_make_the_drivers_events() {
        let b = bytes(&format!("{HEAD} patch 6\n C4:4. D4:8+16 r:2 end"));
        // After the prologue (to the transpose at index 8..10): a dotted
        // quarter is opcode 6 and a table-B byte; a tie of two is 0x22 and
        // two events; a half rest is 0xc0.
        assert_eq!(&b[10..], [0x06, 0xb0, 0x22, 0x92, 0x72, 0xc0, 0x0f]);
    }

    #[test]
    fn a_dotted_length_in_a_tie_is_two_plain_ones_and_a_dotted_event_after_a_tie_clears_the_counter() {
        // Patch 6, C4 at transpose 32 (p 16), D4 p 18. 4. is 4+8 in a tie.
        let b = bytes(&format!("{HEAD} patch 6
 C4:4.+8 D4:4. D4:4. end"));
        // Tie of three: quarter, eighth, eighth; then the first dotted event
        // after it has 0x20 first, the second does not.
        assert_eq!(&b[10..], [0x23, 0xb0, 0x90, 0x90, 0x20, 0x06, 0xb2, 0x06, 0xb2, 0x0f]);
        // A plain event uses the counter up, so a dot after it needs nothing.
        let b = bytes(&format!("{HEAD} patch 6
 C4:4+8 D4:4 D4:4. end"));
        assert_eq!(&b[10..], [0x22, 0xb0, 0x90, 0xb2, 0x06, 0xb2, 0x0f]);
        // A block's first event can follow a tie by the jump back.
        let b = bytes(&format!("{HEAD} patch 6
 repeat 2 {{ C4:4. D4:4+8 }} end"));
        assert_eq!(&b[8..], [0x03, 32, 0x20, 0x06, 0xb0, 0x22, 0xb2, 0x92, 0x0c, 1, 8, 0, 0x0f]);
    }

    #[test]
    fn a_dotted_32nd_cannot_be_tied() {
        let e = error(&format!("{HEAD} patch 6
 C4:32.+8 end"));
        assert!(e.contains("ties a dotted 1/32"), "{e}");
        let e = error(&format!("{HEAD} patch 6
 C4:4.+4.+4.+4.+4.+4.+4.+4. end"));
        assert!(e.contains("ties 16 lengths once its dots are written out"), "{e}");
    }

    #[test]
    fn repeats_and_a_forever_block_jump_back_to_their_start() {
        let b = bytes(&format!("{HEAD} patch 6\n repeat 3 {{ C4:8 }} forever {{ D4:8 }}"));
        // [00 10 07 06 02 c8 09 03] = 8 bytes, then the repeat's body.
        let start = 8;
        assert_eq!(&b[start..start + 4], &[0x03, 32, 0x80 | 16, 0x0c]);
        assert_eq!(b[start + 4], 2, "three plays: the jump back is taken twice");
        assert_eq!(u16::from_le_bytes([b[start + 5], b[start + 6]]) as usize, start);
        let forever = start + 7;
        assert_eq!(b[forever + 3], 0x0c, "the same opcode, its counter unused");
        assert_eq!(b[forever + 4], 0, "a zero count always jumps");
        let c = convert(&format!("{HEAD} patch 6\n repeat 3 {{ C4:8 }} forever {{ D4:8 }}"), &rom()).unwrap();
        let stream = c.tune.channels[0].clone().unwrap();
        assert_eq!(stream.jumps, vec![start + 5, forever + 5]);
    }

    #[test]
    fn nested_repeats_use_the_next_counter_and_a_block_restores_its_entry_state() {
        let b = bytes(&format!(
            "{HEAD} patch 6\n repeat 2 {{ C4:8 repeat 2 {{ D4 patch 5 E4 }} }} end"
        ));
        // The inner block ends with patch 6 put back; each repeat ends with
        // its own opcode.
        let inner_end = b.windows(2).position(|w| w == [0x0d, 1]).expect("the inner repeat uses counter 0xd");
        assert_eq!(&b[inner_end - 2..inner_end], &[0x07, 6]);
        assert!(b.windows(2).any(|w| w == [0x0c, 1]), "the outer repeat uses 0xc");
    }

    #[test]
    fn rejects_what_the_driver_cannot_play_with_the_line() {
        for (score, wants) in [
            ("command 0x3f\nunit 10\nchannel 0\n", "command 63 is outside 64..=127"),
            ("command 0x40\nunit 0\nchannel 0\n", "unit 0 is outside 1..=255"),
            ("command 0x40\nunit 10\npriority 0\n", "priority 0 is outside 1..=15"),
            ("command 0x40\nunit 10\nchannel 8\n", "channel 8 is outside 0..=7"),
            ("command 0x40\nunit 10\nchannel 0\n patch 7 end", "patch 7 is blank in the ROM"),
            ("command 0x40\nunit 10\nchannel 0\n patch 200 end", "patch 200 is past the ROM's 128 patches"),
            ("command 0x40\nunit 10\nchannel 0\n C4:4 end", "channel 0 needs a `patch`"),
            ("command 0x40\nunit 10\nchannel 0\n patch 6 C4 end", "has no length"),
            ("command 0x40\nunit 10\nchannel 0\n patch 6 C0:4 end", "outside the driver's key table"),
            ("command 0x40\nunit 10\nchannel 0\n patch 6 C9:4 end", "no octave 0 to 8"),
            ("command 0x40\nunit 10\nchannel 0\n patch 6 H4:4 end", "not a statement, a note or a rest"),
            ("command 0x40\nunit 10\nchannel 0\n patch 6 C4:64 end", "the driver has 1, 2, 4, 8, 16 and 32"),
            ("command 0x40\nunit 10\nchannel 0\n patch 6 C4:3 end", "the driver has 1, 2, 4, 8, 16 and 32"),
            ("command 0x40\nunit 10\nchannel 0\n patch 6 C4:4t end", "the driver has 1, 2, 4, 8, 16 and 32"),
            ("command 0x40\nunit 10\nchannel 0\n patch 6 pan middle end", "pan `middle`"),
            ("command 0x40\nunit 10\nchannel 0\n patch 6 repeat 1 { C4:4 } end", "repeat count 1 is outside 2..=256"),
            ("command 0x40\nunit 10\nchannel 0\n patch 6 repeat 2 { } end", "no notes"),
            ("command 0x40\nunit 10\nchannel 0\n patch 6 repeat 2 { C4:4 end", "`end` must be at the channel's top level"),
            ("command 0x40\nunit 10\nchannel 0\n patch 6 repeat 2 { C4:4 ", "never closed"),
            ("command 0x40\nunit 10\nchannel 0\n patch 6 C4:4 }", "closes no block"),
            ("command 0x40\nunit 10\nchannel 0\n patch 6 C4:4", "no `end` and no `forever` block"),
            ("command 0x40\nunit 10\nchannel 0\n patch 6 forever { C4:4 } D4", "follows the end of channel 0"),
            ("command 0x40\nunit 10\nchannel 0\n patch 6 repeat 2 { forever { C4:4 } } end", "top level"),
            ("command 0x40\nunit 10\nchannel 0\n patch 6 end\nchannel 0\n", "channel 0 is given twice"),
            ("unit 10\nchannel 0\n patch 6 C4:4 end", "no `command`"),
            ("command 0x40\nchannel 0\n", "`unit` must come before the first channel"),
            ("command 0x40\nunit 10\n", "no channel"),
            ("command 0x40\nunit 10\nchannel 0\n patch 6 C4:4 end\nunit 5\n", "belongs before the first channel"),
        ] {
            let e = error(score);
            assert!(e.contains(wants), "`{score}` gave `{e}`, wanted `{wants}`");
        }
    }

    #[test]
    fn the_message_names_the_line() {
        let e = convert("command 0x40\nunit 10\nchannel 0\n patch 6\n C4:4\n D4:99 end", &rom()).unwrap_err();
        assert_eq!(e.line, 6);
        assert!(e.to_string().starts_with("line 6: "));
    }

    #[test]
    fn nests_at_most_three_repeats_and_ties_at_most_fifteen_lengths() {
        let e = error(&format!("{HEAD} patch 6\n repeat 2 {{ repeat 2 {{ repeat 2 {{ repeat 2 {{ C4:4 }} }} }} }} end"));
        assert!(e.contains("nest at most 3 deep"), "{e}");
        let tie = vec!["32"; 16].join("+");
        let e = error(&format!("{HEAD} patch 6\n C4:{tie} end"));
        assert!(e.contains("ties 16 lengths"), "{e}");
        let tie = vec!["32"; 15].join("+");
        assert_eq!(bytes(&format!("{HEAD} patch 6\n C4:{tie} end"))[10], 0x2f);
    }

    #[test]
    fn a_short_rom_is_refused() {
        let e = convert("command 0x40", &[0; 100]).unwrap_err();
        assert!(e.to_string().contains("100 bytes"));
    }
}
