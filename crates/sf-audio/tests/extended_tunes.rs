//! Acceptance tests for ticket #195 (`ST6`): the extended tune table plays
//! the committed test theme through the translated FM driver, and the win
//! jingle, the round stops and the saved-tune slot treat it as they treat a
//! 1987 tune (ADR-0009, docs/reference/extended-tunes.md). The sound-CPU
//! test seam is the music CPU from power-on on a latch timeline
//! (`check::run_music_from_power_on`), as the music and sound-test suites
//! use it.
//!
//! Consumes the local ROM set (sf-02.7k); skips on a clean checkout.

use std::path::Path;

use sf_audio::check::{music_ram_at_frame_ends, run_music_from_power_on, MusicRun, OpmWrite};
use sf_audio::opm::OPM_RATE;
use sf_audio::tunes::{self, ExtendedTunes};
use sf_audio::LatchTimeline;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
/// The key-code table the driver turns a note into a key code with.
const KEY_TABLE: usize = 0x0a3a;
/// The 1987 stage theme the test theme is compared with: priority 8 too.
const STAGE_THEME: u8 = 0x29;
const THEME: u8 = 0x40;
const WIN_JINGLE: u8 = 0x37;
const STOP: u8 = 0x3f;
/// A tune of priority 15 that ends by itself (the ending's staff roll).
const HIGHER: u8 = 0x36;
/// One sequencer tick, timer A's period: 168 x 64 chip clocks.
const TICK: f64 = 168.0 * 64.0 / 3_579_545.0;

fn rom() -> Option<Vec<u8>> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(sf_video::romset::load_region(ROMS, MANIFEST, "audiocpu").expect("audiocpu region"))
}

fn table(rom: &[u8]) -> ExtendedTunes {
    tunes::committed(rom).expect("the committed scores convert against the ROM")
}

/// The latch the engine writes: a command on a frame, 0xff on the next.
fn latch(script: &[(u32, u8)]) -> LatchTimeline {
    let mut events = Vec::new();
    for &(frame, command) in script {
        events.push((frame as f64 / 60.0, command));
        events.push(((frame + 1) as f64 / 60.0, 0xff));
    }
    LatchTimeline::new(events)
}

fn play(program: &[u8], script: &[(u32, u8)], seconds: f64) -> MusicRun {
    let run = run_music_from_power_on(
        program.to_vec(),
        latch(script),
        (seconds * OPM_RATE as f64) as u64,
    );
    assert!(run.unrecovered.is_empty(), "translated paths only: {:04x?}", run.unrecovered);
    run
}

/// A key-on of `channel` with the note's key-code table index (C#0 = 1).
#[derive(Debug, Clone, Copy, PartialEq)]
struct KeyOn {
    channel: u8,
    note: u8,
    /// Seconds from the start of the run.
    at: f64,
}

/// Every key-on of the run, with the key code written before it.
fn key_ons(rom: &[u8], writes: &[OpmWrite]) -> Vec<KeyOn> {
    let mut code = [0u8; 8];
    let mut out = Vec::new();
    for w in writes {
        match w.reg {
            0x28..=0x2f => code[(w.reg - 0x28) as usize] = w.data,
            0x08 if w.data & 0x78 != 0 => {
                let channel = w.data & 7;
                let note = (1..=96u8).find(|&i| rom[KEY_TABLE + i as usize] == code[channel as usize]);
                out.push(KeyOn {
                    channel,
                    note: note.expect("a key code from the driver's table"),
                    at: w.sample as f64 / OPM_RATE as f64,
                });
            }
            _ => {}
        }
    }
    out
}

/// The notes of `channel`, in order, within `[from, to)` seconds.
fn notes(ons: &[KeyOn], channel: u8, from: f64, to: f64) -> Vec<u8> {
    ons.iter()
        .filter(|k| k.channel == channel && k.at >= from && k.at < to)
        .map(|k| k.note)
        .collect()
}

fn peak(run: &MusicRun, from: f64, to: f64) -> f32 {
    let (a, b) = ((from * OPM_RATE as f64) as usize, (to * OPM_RATE as f64) as usize);
    run.left[a..b.min(run.left.len())].iter().fold(0.0, |m, s| m.max(s.abs()))
}

/// Scientific pitch to the key table's index: C4 = 48.
const fn idx(octave: u8, semitone: u8) -> u8 {
    12 * octave + semitone
}
const C: u8 = 0;
const D: u8 = 2;
const E: u8 = 4;
const F: u8 = 5;
const G: u8 = 7;
const A: u8 = 9;
const B: u8 = 11;

/// The tune channel's first bars, from test-theme.score.
fn lead_first_two_bars() -> Vec<u8> {
    vec![
        idx(4, A), idx(5, C), idx(5, E), idx(5, D), idx(5, C),
        idx(4, B), idx(4, G), idx(4, A),
    ]
}

/// The music CPU's RAM at the end of each of `frames`, or of the first frame
/// after it that ends with the CPU idle (a handler's mid-state is not a
/// state to compare).
fn rams(program: &[u8], script: &[(u32, u8)], frames: &[u32]) -> Vec<Vec<u8>> {
    const TRIES: u32 = 12;
    let asked: Vec<u32> = frames.iter().flat_map(|&f| f..f + TRIES).collect();
    let (rams, unrecovered) = music_ram_at_frame_ends(program.to_vec(), latch(script), &asked);
    assert!(unrecovered.is_empty(), "translated paths only: {unrecovered:04x?}");
    rams.chunks(TRIES as usize)
        .map(|group| {
            let (_, _, ram) = group.iter().find(|(_, idle, _)| *idle).expect("an idle frame end");
            ram.clone()
        })
        .collect()
}

/// The current tune's header (0xc011) and the saved one's (0xc012).
fn headers(ram: &[u8]) -> (u8, u8) {
    (ram[0x11], ram[0x12])
}

fn all_headers(rams: &[Vec<u8>]) -> Vec<(u8, u8)> {
    rams.iter().map(|r| headers(r)).collect()
}

/// Channel `ch`'s stream pointer in the current tune (0xc100) or the saved
/// one (0xc300).
fn stream(ram: &[u8], saved: bool, ch: usize) -> u16 {
    let at = if saved { 0x300 } else { 0x100 } + 0x30 * ch;
    u16::from_be_bytes([ram[at], ram[at + 1]])
}

/// A frame a third of the way into second `second`; `rams` takes the first
/// idle frame end from it on.
fn at(second: u32) -> u32 {
    60 * second + 20
}

fn is_extended(pointer: u16) -> bool {
    (tunes::EXTENSION_BASE..tunes::EXTENSION_END).contains(&pointer)
}

#[test]
fn the_test_theme_plays_from_the_extended_table() {
    let Some(rom) = rom() else { return };
    let tunes = table(&rom);
    assert_eq!(tunes.commands(), vec![THEME]);
    let program = tunes.program(&rom).unwrap();
    assert_eq!(&program[..0x8000], &rom[..], "the ROM's own bytes are untouched");

    // Twenty seconds: one pass of the eight bars (15.4 s) and the next one's start.
    let run = play(&program, &[(30, THEME)], 20.0);
    let ons = key_ons(&rom, &run.writes);
    let start = ons.first().expect("the theme sounds").at;
    assert!(start > 0.5 && start < 0.6, "the first note comes with the command ({start})");

    // The notes of each channel, as the score gives them.
    assert_eq!(&notes(&ons, 0, 0.0, 20.0)[..8], &lead_first_two_bars()[..]);
    let arpeggio: Vec<u8> = [(3, A), (4, E), (3, A), (4, E), (3, G), (4, D), (3, G), (4, D), (3, F), (4, C)]
        .iter()
        .map(|&(o, s)| idx(o, s))
        .collect();
    assert_eq!(&notes(&ons, 1, 0.0, 20.0)[..10], &arpeggio[..]);
    let bass: Vec<u8> = [(2, A), (2, E), (2, G), (2, D), (2, F), (2, C)]
        .iter()
        .map(|&(o, s)| idx(o, s))
        .collect();
    assert_eq!(&notes(&ons, 2, 0.0, 20.0)[..6], &bass[..]);
    assert!(ons.iter().all(|k| k.channel < 3), "only the score's three channels sound");

    // The lengths: a quarter note is 16 units of 10 ticks, so the second
    // note comes a quarter after the first, the fourth two quarters, the
    // second bar four quarters.
    let lead: Vec<f64> = ons.iter().filter(|k| k.channel == 0).map(|k| k.at - start).collect();
    let quarter = 160.0 * TICK;
    for (i, quarters) in [(1, 1.0), (3, 2.0), (4, 3.0), (5, 4.0), (8, 8.0)] {
        let want = quarters * quarter;
        // The driver loses a tick when two come due inside one handler, so
        // the clock runs a little slow: within 2%.
        assert!((lead[i] - want).abs() < 0.02 * want + 0.02, "note {i} at {} s, wanted {want}", lead[i]);
    }

    // It loops: the second pass begins as the first did, after 32 quarters.
    let second = ons.iter().filter(|k| k.channel == 0).nth(32).expect("a second pass").at - start;
    assert!((second - 32.0 * quarter).abs() < 0.02 * 32.0 * quarter, "the loop at {second} s");
    assert_eq!(notes(&ons, 0, start + second - 0.05, 20.0)[..8], lead_first_two_bars()[..]);

    // It is heard.
    assert!(peak(&run, 1.0, 10.0) > 0.05, "the theme sounds ({})", peak(&run, 1.0, 10.0));
    // And a channel the theme leaves off stays off.
    assert!(run.ram[0x100 + 3 * 0x30..0x100 + 3 * 0x30 + 2] == [0, 0]);
}

#[test]
fn an_extended_command_that_names_no_tune_does_nothing() {
    let Some(rom) = rom() else { return };
    let program = table(&rom).program(&rom).unwrap();
    let script = [(30, 0x41), (90, 0x7f)];
    let run = play(&program, &script, 3.0);
    assert!(key_ons(&rom, &run.writes).is_empty());
    let ram = &rams(&program, &script, &[at(2)])[0];
    assert_eq!(headers(ram), (0, 0));
    assert!((0..8).all(|ch| stream(ram, false, ch) == 0));

    // Over a playing tune it changes nothing either: the stage theme goes on
    // as if the command had not been sent.
    let alone = play(&program, &[(30, STAGE_THEME)], 8.0);
    let sent = play(&program, &[(30, STAGE_THEME), (300, 0x41), (360, 0x7f)], 8.0);
    assert!(alone.writes.len() > 1000);
    // The latch holds each command for a frame, and the driver's polls of
    // it count as repeats; nothing else about its paths differs.
    let paths = |r: &MusicRun| sf_audio::music::MusicObservations { repeated_cmd_ignores: 0, ..r.observations };
    assert_eq!(paths(&sent), paths(&alone));
    // The stray command's intake costs the driver a few T-states, which can
    // move a later write by a sample: the same notes on the same channels in
    // the same order, within a millisecond.
    let (a, b) = (key_ons(&rom, &alone.writes), key_ons(&rom, &sent.writes));
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(&b) {
        assert_eq!((x.channel, x.note), (y.channel, y.note));
        assert!((x.at - y.at).abs() < 0.001, "{x:?} against {y:?}");
    }
    assert_eq!(sent.left.len(), alone.left.len());
    let ram = &rams(&program, &[(30, STAGE_THEME), (300, 0x41)], &[at(6)])[0];
    assert_eq!(headers(ram), (8, 0));
}

#[test]
fn the_table_changes_nothing_for_the_1987_commands() {
    let Some(rom) = rom() else { return };
    let program = table(&rom).program(&rom).unwrap();
    // A stage theme, the win jingle over it, the stop, another tune, a
    // sound effect, the stop of effects: the 1987 paths.
    let script = [(30, 0x29), (300, 0x37), (480, STOP), (540, 0x34), (800, 0x0b), (900, 0x27), (960, 0x3d)];
    let plain = play(&rom, &script, 17.0);
    let extended = play(&program, &script, 17.0);
    assert!(plain.writes.len() > 1000);
    assert_eq!(extended.writes, plain.writes, "every write, on the same clock");
    assert_eq!(extended.left, plain.left, "every output sample");
    assert_eq!(extended.ram, plain.ram);
    assert_eq!(extended.observations, plain.observations);
}

/// The win jingle replaces a theme at equal priority, as it does a 1987
/// stage theme, and ends alone when it is over.
#[test]
fn the_win_jingle_cuts_the_theme_as_it_cuts_a_stage_theme() {
    let Some(rom) = rom() else { return };
    let program = table(&rom).program(&rom).unwrap();
    let seconds: Vec<u32> = (1..=14).map(at).collect();
    let run = |tune: u8| {
        let script = [(30, tune), (300, WIN_JINGLE)];
        (rams(&program, &script, &seconds), play(&program, &script, 14.0))
    };
    let (theme_ram, theme) = run(THEME);
    let (stage_ram, stage) = run(STAGE_THEME);
    assert_eq!(all_headers(&theme_ram), all_headers(&stage_ram), "the same header pair every second");
    assert_eq!(all_headers(&theme_ram)[0], (8, 0), "the theme plays");
    assert_eq!(all_headers(&theme_ram)[5], (8, 0), "the jingle plays in its place");
    assert_eq!(all_headers(&theme_ram).last(), Some(&(0, 0)), "the jingle ended and nothing was handed back");
    // After the cut not one channel is on the theme any more.
    assert!(theme_ram[5..9]
        .iter()
        .all(|r| (0..8).all(|ch| !is_extended(stream(r, false, ch)) && stream(r, true, ch) == 0)));
    assert_eq!(theme.observations, stage.observations, "the same driver paths");
    // The theme's notes stop at the cut, and the jingle's go on.
    let cut = 300.0 / 60.0;
    let ons = key_ons(&rom, &theme.writes);
    assert!(ons.iter().any(|k| k.at < cut), "the theme sounded");
    assert!(ons.iter().any(|k| k.at > cut + 1.0), "the jingle sounds");
}

/// The round stops (0x35, 0x3d, 0x3f) stop a theme, and a saved one with
/// it, as they do a 1987 tune.
#[test]
fn the_round_stops_cut_the_theme_as_they_cut_a_stage_theme() {
    let Some(rom) = rom() else { return };
    let program = table(&rom).program(&rom).unwrap();
    for stop in [0x35, 0x3d, STOP] {
        let script = |tune: u8| [(30, tune), (300, HIGHER), (600, stop)];
        let frames = [at(4), at(8), at(11), at(13)];
        let theme_ram = rams(&program, &script(THEME), &frames);
        let stage_ram = rams(&program, &script(STAGE_THEME), &frames);
        assert_eq!(all_headers(&theme_ram), all_headers(&stage_ram), "stop {stop:#04x}");
        assert_eq!(all_headers(&theme_ram), vec![(8, 0), (0x0f, 8), (0, 0), (0, 0)], "stop {stop:#04x}");
        // The saved theme was in the extension; after the stop nothing is.
        assert!((0..3).all(|ch| is_extended(stream(&theme_ram[1], true, ch))));
        for ch in 0..8 {
            assert_eq!((stream(&theme_ram[2], false, ch), stream(&theme_ram[2], true, ch)), (0, 0));
        }
        assert!(theme_ram[2][0x300..0x480].iter().all(|&b| b == 0), "the saved slot is cleared");
        // Silent after it, and every channel keyed off, as for the stage theme.
        let theme = play(&program, &script(THEME), 13.0);
        let stage = play(&program, &script(STAGE_THEME), 13.0);
        assert_eq!(theme.observations, stage.observations, "stop {stop:#04x}");
        assert!(peak(&theme, 11.0, 13.0) < 0.001, "silence after the stop ({})", peak(&theme, 11.0, 13.0));
        let off = |run: &MusicRun| {
            let mut off = [false; 8];
            for w in run.writes.iter().filter(|w| w.sample as f64 / OPM_RATE as f64 > 10.0) {
                assert!(w.reg != 0x08 || w.data & 0x78 == 0, "no key-on after the stop");
                if w.reg == 0x08 {
                    off[(w.data & 7) as usize] = true;
                }
            }
            off
        };
        assert_eq!(off(&theme), [true; 8], "stop {stop:#04x}");
        assert_eq!(off(&theme), off(&stage));
    }
}

/// A tune of higher priority saves the theme and gives it back when it
/// ends, the theme's note sounding again from its attack and the melody on
/// from where it was cut.
#[test]
fn a_higher_priority_tune_saves_the_theme_and_hands_it_back() {
    let Some(rom) = rom() else { return };
    let program = table(&rom).program(&rom).unwrap();
    let script = |tune: u8| [(30, tune), (300, HIGHER)];
    let seconds = [at(4), at(8), at(31), at(35)];
    let theme_ram = rams(&program, &script(THEME), &seconds);
    let stage_ram = rams(&program, &script(STAGE_THEME), &seconds);
    assert_eq!(all_headers(&theme_ram), all_headers(&stage_ram));
    assert_eq!(all_headers(&theme_ram), vec![(8, 0), (0x0f, 8), (0x0f, 8), (8, 0)]);
    // While the higher tune plays the theme sits in the saved slot, on its
    // three channels; when it is over the theme is back in the playing one.
    for ch in 0..8 {
        assert_eq!(is_extended(stream(&theme_ram[1], true, ch)), ch < 3, "saved channel {ch}");
        assert!(!is_extended(stream(&theme_ram[1], false, ch)));
        assert_eq!(is_extended(stream(&theme_ram[3], false, ch)), ch < 3, "restored channel {ch}");
        assert_eq!(stream(&theme_ram[3], true, ch), 0, "nothing left saved");
    }

    let theme = play(&program, &script(THEME), 36.0);
    let stage = play(&program, &script(STAGE_THEME), 36.0);
    assert_eq!(theme.observations, stage.observations, "the same driver paths");
    assert_eq!(theme.observations.tune_higher_priority_saves, 2, "the theme, then the higher tune");
    assert_eq!(theme.observations.saved_pops, 1);
    assert_eq!(theme.observations.saved_discards, 1);

    // The hand-back: the frame the theme's header is the playing one again
    // is where the pop re-keys the held note of each channel.
    let cut = 300.0 / 60.0;
    let ons = key_ons(&rom, &theme.writes);
    let alone = key_ons(&rom, &play(&program, &script(THEME)[..1], 20.0).writes);
    let scan: Vec<u32> = (at(30)..at(36)).collect();
    let (scanned, _) = music_ram_at_frame_ends(program.clone(), latch(&script(THEME)), &scan);
    let (back, _, _) = scanned.iter().find(|(_, _, r)| headers(r) == (8, 0)).expect("the theme comes back");
    let pop = *back as f64 / 60.0 - 0.01;
    for ch in 0..3u8 {
        // The loop's notes in order, from the theme alone.
        let first_pass = notes(&ons, ch, 0.0, cut);
        let whole = notes(&alone, ch, 0.0, 20.0);
        let held = first_pass.len() - 1;
        let resumed: Vec<u8> = ons
            .iter()
            .filter(|k| k.channel == ch && k.at >= pop)
            .map(|k| k.note)
            .collect();
        assert!(resumed.len() >= 3, "channel {ch} plays on ({} notes)", resumed.len());
        assert_eq!(resumed[0], whole[held], "channel {ch} sounds its held note again from the attack");
        assert_eq!(resumed[1], whole[held + 1], "channel {ch} goes on with the next note");
        assert_eq!(resumed[2], whole[held + 2]);
    }
}

/// The commands the 1987 sweep (`sf-engine`'s `extended_tunes` suite) checks
/// are the table's.
#[test]
fn the_table_takes_the_commands_the_1987_game_never_sends() {
    assert_eq!((tunes::EXTENDED_FIRST, tunes::EXTENDED_LAST), (0x40, 0x7f));
    assert_eq!((tunes::EXTENSION_BASE, tunes::EXTENSION_END), (0x8000, 0xc000));
}

/// The converter against the real ROM's patches: a score the driver cannot
/// play is refused with a message that names the fault and the line.
#[test]
fn the_converter_refuses_what_the_driver_cannot_play() {
    let Some(rom) = rom() else { return };
    let head = "command 0x41\nunit 10\nchannel 0\n";
    for (body, wants) in [
        ("patch 100\n C4:4 end", "line 4: patch 100 is blank in the ROM"),
        ("patch 128\n C4:4 end", "line 4: patch 128 is past the ROM's 128 patches"),
        ("patch 24\n C0:4 end", "line 5: `C0` is outside the driver's key table, C#0 to C8"),
        ("patch 24\n C4:64 end", "line 5: length `64`"),
        ("patch 24\n C4:4", "channel 0 has no `end` and no `forever` block"),
        ("patch 24\n forever { }", "line 5: this block has no notes"),
    ] {
        let e = sf_audio::score::convert(&format!("{head}{body}"), &rom).unwrap_err().to_string();
        assert!(e.contains(wants), "`{body}` gave `{e}`, wanted `{wants}`");
    }
    // And the committed score, and one more tune beside it, are accepted.
    let mut t = table(&rom);
    let more = sf_audio::score::convert(&format!("{head}patch 24\n C4:4 D4 E4 end"), &rom).unwrap();
    more.add_to(&mut t).unwrap();
    assert_eq!(t.commands(), vec![0x40, 0x41]);
    // A command twice is refused by the table, not silently replaced.
    let again = sf_audio::score::convert(&format!("{head}patch 24\n C4:4 end"), &rom).unwrap();
    assert!(again.add_to(&mut t).unwrap_err().to_string().contains("already names a tune"));
}

/// Every channel stream of the committed theme is one the driver can play:
/// it ends or loops, reads only opcodes the 1987 tunes use, and its jump
/// targets are inside it.
#[test]
fn the_committed_theme_uses_only_the_1987_tunes_opcodes() {
    let Some(rom) = rom() else { return };
    let tunes = table(&rom);
    let image = tunes.image();
    let at = |a: u16| (a - tunes::EXTENSION_BASE) as usize;
    let tune = at(tunes.address(THEME).unwrap());
    assert_eq!(image[tune], 8, "priority 8 like the 1987 themes");
    for ch in 0..8 {
        let p = u16::from_le_bytes([image[tune + 1 + 2 * ch], image[tune + 2 + 2 * ch]]);
        assert_eq!(p != 0, ch < 3, "channel {ch}");
        if p == 0 {
            continue;
        }
        let mut pc = at(p);
        // The stream ends where the next one starts, or where the tune does.
        let end = (0..8)
            .map(|c| u16::from_le_bytes([image[tune + 1 + 2 * c], image[tune + 2 + 2 * c]]))
            .filter(|&q| q > p)
            .min()
            .map_or(image.len(), at);
        // Walk to the stream's last event: the unconditional jump or the end.
        loop {
            let op = image[pc];
            match op >> 4 {
                0 => match op {
                    0..=3 | 6 | 7 | 9 => pc += 2,
                    0xc..=0xe => {
                        let target = at(u16::from_le_bytes([image[pc + 2], image[pc + 3]]));
                        assert!((at(p)..end).contains(&target), "channel {ch}: a jump stays in its own stream");
                        if image[pc + 1] == 0 {
                            break;
                        }
                        pc += 4;
                    }
                    0xf => break,
                    other => panic!("channel {ch}: opcode {other:#x} is not one the 1987 tunes use here"),
                },
                _ => pc += 1,
            }
        }
    }
}

/// What one channel does to its key: on with the note's table index, or off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Key {
    On(u8),
    Off,
}

fn key_events(rom: &[u8], writes: &[OpmWrite], channel: u8) -> Vec<Key> {
    let mut code = 0u8;
    let mut out = Vec::new();
    for w in writes {
        if w.reg == 0x28 + channel {
            code = w.data;
        } else if w.reg == 0x08 && w.data & 7 == channel {
            if w.data & 0x78 != 0 {
                let note = (1..=96u8).find(|&i| rom[KEY_TABLE + i as usize] == code);
                out.push(Key::On(note.expect("a key code from the driver's table")));
            } else {
                out.push(Key::Off);
            }
        }
    }
    out
}

/// Convert `body` as channel 0 of a one-channel tune on patch 24, play it
/// from the extended table and return channel 0's key events after the
/// boot's own key-offs.
fn played(rom: &[u8], body: &str) -> Vec<Key> {
    let text = format!("command 0x40\nunit 10\nchannel 0\n patch 24\n gate 150\n {body}");
    let mut t = ExtendedTunes::new();
    sf_audio::score::convert(&text, rom).unwrap().add_to(&mut t).unwrap();
    let run = play(&t.program(rom).unwrap(), &[(30, THEME)], 6.0);
    let mut events = key_events(rom, &run.writes, 0);
    let first = events.iter().position(|k| matches!(k, Key::On(_))).expect("the tune sounds");
    events.drain(..first);
    events
}

fn ons(events: &[Key]) -> Vec<u8> {
    events.iter().filter_map(|k| if let Key::On(n) = k { Some(*n) } else { None }).collect()
}

/// Opcode 6 (a dotted event) skips the tie counter's count-down in the
/// driver (0x08ba jumps past 0x04e2), so the converter writes a dotted
/// length inside a tie as two plain ones and clears the counter before a
/// dotted event that could follow a tie. Each case is the driver's own
/// key-ons: a note that was lost shows as a missing one.
#[test]
fn a_dotted_event_beside_a_tie_keeps_its_key_on_and_key_off() {
    let Some(rom) = rom() else { return };
    const C4: u8 = 48;
    const D4: u8 = 50;
    const E4: u8 = 52;
    const F4: u8 = 53;
    // A dotted note after a tie.
    assert_eq!(ons(&played(&rom, "C4:4+8 D4:4. E4:4 F4:4 end")), vec![C4, D4, E4, F4]);
    // A dotted length inside a tie: the tie is one key-on, the next note its own.
    assert_eq!(ons(&played(&rom, "C4:4+8. D4:4 end")), vec![C4, D4]);
    assert_eq!(ons(&played(&rom, "C4:4.+8. D4:4 end")), vec![C4, D4]);
    // A dotted note opening a block after a tie ended, on both passes.
    assert_eq!(ons(&played(&rom, "C4:4+8 repeat 2 { D4:4. } E4:4 end")), vec![C4, D4, D4, E4]);
    // A tie ending a block, then the dotted note that opens it again.
    assert_eq!(ons(&played(&rom, "repeat 3 { D4:4. C4:4+8 } E4:4 end")), vec![D4, C4, D4, C4, D4, C4, E4]);
    // A dotted rest after a tie still keys the tied note off, before the next.
    let rest = played(&rom, "C4:4+8 r:4. D4:4 end");
    let (c, d) = (rest.iter().position(|k| *k == Key::On(C4)).unwrap(), rest.iter().position(|k| *k == Key::On(D4)).unwrap());
    assert!(rest[c..d].contains(&Key::Off), "the rest keys C4 off: {rest:?}");
    let rest = played(&rom, "C4:4+8 repeat 2 { r:4. D4:4 } end");
    let first_d = rest.iter().position(|k| *k == Key::On(D4)).unwrap();
    assert!(rest[..first_d].contains(&Key::Off), "the block's dotted rest keys C4 off: {rest:?}");
    assert_eq!(ons(&rest), vec![C4, D4, D4]);
}

/// The control: reading the table one word off finds nothing for the
/// command and the next command's tune for the one after it, so the tests
/// above do depend on the lookup reading the word for its own command.
#[test]
fn reading_the_table_one_word_off_would_change_what_plays() {
    let Some(rom) = rom() else { return };
    let program = table(&rom).program(&rom).unwrap();
    let mut off = program.clone();
    let base = 0x8000;
    off.copy_within(base..base + 126, base + 2);
    off[base..base + 2].fill(0);
    let theme = key_ons(&rom, &play(&program, &[(30, THEME)], 6.0).writes);
    assert!(!theme.is_empty());
    assert!(key_ons(&rom, &play(&off, &[(30, THEME)], 6.0).writes).is_empty(), "0x40 names nothing in the shifted table");
    let next = key_ons(&rom, &play(&off, &[(30, THEME + 1)], 6.0).writes);
    assert_eq!(notes(&next, 0, 0.0, 6.0), notes(&theme, 0, 0.0, 6.0), "0x41 finds the theme's word");
}

// ---- the table handed to a running music CPU (ticket #199, `ST7`) -------------

mod handed_over {
    use std::sync::Arc;

    use sf_audio::mix::GameSound;
    use sf_audio::{LatchTimeline, FRAME_HZ, IRQ_LATENCY_MID};

    use super::*;

    const FRAMES: u32 = 240;
    /// The frame the theme is asked for on.
    const ASK: u32 = 30;

    fn regions() -> Option<(Vec<u8>, Vec<u8>)> {
        let rom = rom()?;
        Some((sf_video::romset::load_region(ROMS, MANIFEST, "audio2").expect("audio2 region"), rom))
    }

    fn value(frame: u32, script: &[(u32, u8)]) -> u8 {
        script.iter().find(|&&(f, _)| f == frame).map_or(0xff, |&(_, c)| c)
    }

    /// The mix over `FRAMES` frames, every latch write known at power-on,
    /// on a program that already carries the table or not.
    fn known(audio2: &[u8], program: &[u8], script: &[(u32, u8)]) -> Vec<[f32; 2]> {
        let mut timeline = LatchTimeline::default();
        for frame in 0..FRAMES {
            timeline.push_frame(frame, value(frame, script), 0, IRQ_LATENCY_MID);
        }
        let mut sound = GameSound::power_on(audio2.to_vec(), program.to_vec(), timeline, f64::INFINITY);
        mix(&mut sound)
    }

    /// The mix as the launcher drives it: frames pushed one at a time, the
    /// table given or taken away on the frames `give` and `take` name.
    fn streamed(audio2: &[u8], rom: &[u8], table: &Arc<Vec<u8>>, script: &[(u32, u8)], give: u32, take: Option<u32>) -> Vec<[f32; 2]> {
        let mut sound = GameSound::power_on(audio2.to_vec(), rom.to_vec(), LatchTimeline::default(), 0.0);
        for frame in 0..FRAMES {
            if frame == give {
                sound.set_tune_table(Some(table.clone()));
            }
            if Some(frame) == take {
                sound.set_tune_table(None);
            }
            sound.push_frame(frame, value(frame, script), 0, IRQ_LATENCY_MID);
        }
        mix(&mut sound)
    }

    fn mix(sound: &mut GameSound) -> Vec<[f32; 2]> {
        let end = f64::from(FRAMES) / f64::from(FRAME_HZ) - 0.05;
        assert!(sound.wait_ready(end), "the mix is ready");
        let mut out = Vec::new();
        let mut t = 0.0;
        while t < end {
            out.push(sound.sample_at(t));
            t += 1.0 / 8_000.0;
        }
        assert!(sound.unrecovered().is_empty());
        out
    }

    fn loudest(out: &[[f32; 2]], from: f64) -> f32 {
        out[(from * 8_000.0) as usize..].iter().fold(0.0, |m, s| m.max(s[0].abs()).max(s[1].abs()))
    }

    #[test]
    fn the_table_given_to_a_running_music_cpu_plays_the_theme_as_if_it_had_had_it_from_power_on() {
        let Some((audio2, rom)) = regions() else { return };
        let table = table(&rom);
        let program = table.program(&rom).unwrap();
        let image = Arc::new(table.image().to_vec());
        let script = [(ASK, THEME)];
        let from_power_on = known(&audio2, &program, &script);
        assert!(loudest(&from_power_on, f64::from(ASK) / 60.0 + 0.2) > 0.01, "the theme sounds");
        // Given on the frame before the command, and long before it.
        for give in [ASK - 1, 0] {
            assert_eq!(streamed(&audio2, &rom, &image, &script, give, None), from_power_on, "given on frame {give}");
        }
    }

    #[test]
    fn a_music_cpu_without_the_table_is_the_roms_for_every_1987_command() {
        let Some((audio2, rom)) = regions() else { return };
        let table = table(&rom);
        let program = table.program(&rom).unwrap();
        let image = Arc::new(table.image().to_vec());
        // The stage theme, then the win jingle and a stop: with the table
        // never given, given from the start, or given and taken away.
        let script = [(ASK, STAGE_THEME), (120, WIN_JINGLE), (200, STOP)];
        let alone = known(&audio2, &rom, &script);
        assert_eq!(known(&audio2, &program, &script), alone);
        assert_eq!(streamed(&audio2, &rom, &image, &script, 0, None), alone);
        assert_eq!(streamed(&audio2, &rom, &image, &script, 0, Some(60)), alone);
    }

    #[test]
    fn taking_the_table_away_does_not_cut_a_theme_that_is_playing() {
        let Some((audio2, rom)) = regions() else { return };
        let table = table(&rom);
        let program = table.program(&rom).unwrap();
        let image = Arc::new(table.image().to_vec());
        let script = [(ASK, THEME)];
        let always = known(&audio2, &program, &script);
        // Taken away while the theme loops: it plays on, every sample.
        assert_eq!(streamed(&audio2, &rom, &image, &script, 0, Some(100)), always);
    }
}

/// Ticket #267 (`SP15`): when the local music pack (`sf1-port-music`) is
/// present beside the ROM set, Ryu's theme (`0x41`) converts cleanly
/// against the sound ROM and plays on all three FM channels through the
/// translated driver.
#[test]
fn local_music_pack_ryu_theme_converts_and_plays_from_the_extended_table() {
    use sf_audio::music_pack::{find_pack, pack_folders};
    use sf_video::background_bank::RYU_THEME;

    let Some(rom) = rom() else { return };
    let Ok(pack) = find_pack(&pack_folders(&[Path::new(ROMS)])) else {
        eprintln!("skipping: local music pack not present");
        return;
    };
    assert!(pack.unreadable.is_empty(), "unreadable pack scores: {:?}", pack.unreadable);
    let mut t = table(&rom);
    let errs = tunes::add_pack(&mut t, &rom, &pack);
    assert!(errs.is_empty(), "pack conversion errors: {errs:?}");
    assert!(t.commands().contains(&RYU_THEME), "pack provides Ryu's theme ({RYU_THEME:#04x}): {:?}", t.commands());

    let program = t.program(&rom).unwrap();
    let run = play(&program, &[(30, RYU_THEME)], 25.0);
    let ons = key_ons(&rom, &run.writes);
    for ch in 0..3u8 {
        assert!(!notes(&ons, ch, 0.0, 25.0).is_empty(), "channel {ch} sounds in Ryu's theme");
    }
    assert!(peak(&run, 1.0, 20.0) > 0.05, "Ryu's theme is heard ({})", peak(&run, 1.0, 20.0));
}
