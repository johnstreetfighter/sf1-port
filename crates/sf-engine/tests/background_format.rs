//! Ticket #194 (`ST4`): the 1987 background format, mapped far enough to
//! place a map past the ROM tilemap's 2048 columns
//! (`docs/reference/background-format.md`).
//!
//! The 68000 cannot read `tilerom`; only the video hardware fetches it, for
//! the cells of each enabled scroll layer that its scroll register puts on
//! screen. So "the game reads the map" means "a layer is enabled with its
//! scroll register here", and the sweep records, for every frame of every
//! committed scenario from power-on, both layers' registers and the map
//! columns they show. It only reads the machine after each frame. Local
//! ROMs only; each test skips without them.

use std::path::{Path, PathBuf};

use sf_engine::run::{observe_power_on_scenario, PowerOnMachine};
use sf_video::render::{HEIGHT, VISAREA_X, VISAREA_Y, WIDTH};
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const SCENARIOS: &str = "../../tools/reference/scenarios";

/// The ROM tilemap: 2048 columns of 16 rows of 16x16 tiles per layer.
const COLS: usize = 2048;
const ROWS: usize = 16;
const TILE: usize = 16;
/// Pixels across the ROM map: one more and the scroll wraps to column 0.
const MAP_PX: u32 = (COLS * TILE) as u32;
/// Each layer's colour/code-low ROM; its attribute/code-high ROM is
/// `0x10000` above.
const BG_MAP: usize = 0x00000;
const FG_MAP: usize = 0x20000;
const HIGH: usize = 0x10000;

fn assets() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(load_assets(ROMS, MANIFEST).expect("local ROM set assembles"))
}

fn rw(rom: &[u8], a: usize) -> u16 {
    u16::from_be_bytes([rom[a], rom[a + 1]])
}

fn rl(rom: &[u8], a: usize) -> usize {
    u32::from_be_bytes([rom[a], rom[a + 1], rom[a + 2], rom[a + 3]]) as usize
}

/// One map cell: (colour byte, attribute byte, tile code).
fn cell(tilerom: &[u8], layer: usize, col: usize, row: usize) -> (u8, u8, usize) {
    let b = layer + 2 * (col * ROWS + row);
    (tilerom[b], tilerom[b + HIGH], ((tilerom[b + HIGH + 1] as usize) << 8) | tilerom[b + 1] as usize)
}

/// An unprogrammed cell: all four bytes 0xff.
fn erased(tilerom: &[u8], layer: usize, col: usize, row: usize) -> bool {
    cell(tilerom, layer, col, row) == (0xff, 0xff, 0xffff)
}

/// The logical map pixels a layer shows with scroll `s`, without wrapping:
/// `s + 64 ..= s + 447` in both orientations (render.rs; flip screen
/// mirrors the window in place, so it shows the same columns).
fn window(s: u16) -> (u32, u32) {
    let first = s as u32 + VISAREA_X as u32;
    (first, first + WIDTH as u32 - 1)
}

/// The map rows on screen: 1..=14, whatever the scroll or flip.
fn visible_rows() -> std::ops::RangeInclusive<usize> {
    VISAREA_Y / TILE..=(VISAREA_Y + HEIGHT - 1) / TILE
}

#[derive(Default)]
struct Layer {
    /// Frames the layer was enabled.
    shown: u64,
    /// Lowest and highest register value while enabled, and the highest at
    /// any time.
    lo: u16,
    hi: u16,
    hi_any: u16,
    /// Enabled frames whose window passes column 2047 (it would wrap to 0).
    past: u64,
    /// Enabled frames with flip screen on.
    flipped: u64,
    /// Columns shown at least once.
    cols: Vec<bool>,
}

impl Layer {
    fn new() -> Self {
        Layer { lo: u16::MAX, cols: vec![false; COLS], ..Layer::default() }
    }

    fn see(&mut self, scroll: u16, enabled: bool, flip: bool) {
        self.hi_any = self.hi_any.max(scroll);
        if !enabled {
            return;
        }
        self.shown += 1;
        self.flipped += u64::from(flip);
        self.lo = self.lo.min(scroll);
        self.hi = self.hi.max(scroll);
        let (first, last) = window(scroll);
        if last >= MAP_PX {
            self.past += 1;
        }
        for px in (first..=last).step_by(TILE).chain([last]) {
            self.cols[(px % MAP_PX) as usize / TILE] = true;
        }
    }

    /// The shown columns as inclusive runs.
    fn runs(&self) -> Vec<(usize, usize)> {
        let mut runs: Vec<(usize, usize)> = Vec::new();
        for c in (0..COLS).filter(|&c| self.cols[c]) {
            match runs.last_mut() {
                Some(r) if r.1 + 1 == c => r.1 = c,
                _ => runs.push((c, c)),
            }
        }
        runs
    }
}

fn duration(path: &Path) -> u32 {
    let text = std::fs::read_to_string(path).unwrap();
    let after = text.split("\"duration_frames\":").nth(1).expect("duration_frames");
    let digits: String = after.trim_start().chars().take_while(char::is_ascii_digit).collect();
    digits.parse().unwrap()
}

/// The palette request's source and palette-RAM offset (`0xff8634`,
/// `0xff8638`; `0x3b362` loads 0x200 bytes from one to the other).
fn palette_request(m: &PowerOnMachine) -> (usize, u16) {
    let w = m.work_ram();
    (rl(w, 0x634), rw(w, 0x638))
}

/// The service mode's SCROLL TEST (item 6, `0x48da4..=0x490f6`): the
/// operator steps each register by 0x20 or 0x40 with no bound.
fn in_scroll_test(m: &PowerOnMachine) -> bool {
    m.task_positions().iter().any(|&(_, at)| (0x48da4..=0x490f6).contains(&at))
}

/// The complete one- and two-player games, the service mode, every boss on
/// the ladder, the bonus stages, the ending and both flip-screen runs are
/// all among them.
#[test]
fn no_committed_scenario_scrolls_or_reads_past_the_rom_map() {
    let Some(a) = assets() else { return };
    let mut paths: Vec<PathBuf> = std::fs::read_dir(SCENARIOS).unwrap().map(|e| e.unwrap().path()).collect();
    paths.sort();
    // The game, and the operator's scroll test, apart.
    let [mut bg, mut fg, mut test_bg, mut test_fg] = std::array::from_fn(|_| Layer::new());
    let mut palettes = std::collections::BTreeSet::new();
    let (mut frames, mut test_frames) = (0u64, 0u64);
    for p in &paths {
        let (last, unrecovered) = observe_power_on_scenario(a.maincpu.clone(), p, duration(p) - 1, |m| {
            let s = &m.state;
            let (bg, fg) = if in_scroll_test(m) {
                test_frames += 1;
                (&mut test_bg, &mut test_fg)
            } else {
                (&mut bg, &mut fg)
            };
            bg.see(s.bg_scroll, s.gfxctrl & 0x20 != 0, s.flip_screen);
            fg.see(s.fg_scroll, s.gfxctrl & 0x40 != 0, s.flip_screen);
            palettes.insert(palette_request(m));
        })
        .unwrap();
        assert!(unrecovered.is_empty(), "{}: untranslated {unrecovered:05x?}", p.display());
        frames += u64::from(last) + 1;
    }
    println!("{} scenarios, {frames} frames, {test_frames} of them in the scroll test", paths.len());
    assert!(paths.len() >= 137);
    for (name, layer, base, tiles) in [("bg", &bg, BG_MAP, a.gfx1.len() / 128), ("fg", &fg, FG_MAP, a.gfx2.len() / 128)] {
        println!(
            "{name}: shown {} frames ({} flipped), scroll {:#06x}..={:#06x} while shown, highest ever {:#06x}",
            layer.shown, layer.flipped, layer.lo, layer.hi, layer.hi_any
        );
        println!("  {} columns shown, in runs {:?}", layer.cols.iter().filter(|&&c| c).count(), layer.runs());
        assert_eq!(layer.past, 0, "{name}: a window passes column 2047");
        assert!(layer.hi_any < 0x8000, "{name}: the register's bit 15 was set");
        // Every cell the game shows is a programmed ROM cell: a tile the
        // region holds, a colour of the layer's 16, flips only.
        let (mut code_hi, mut colour_hi, mut attr_bits) = (0usize, 0u8, 0u8);
        for c in (0..COLS).filter(|&c| layer.cols[c]) {
            for r in visible_rows() {
                assert!(!erased(&a.tilerom, base, c, r), "{name}: column {c} row {r} is unprogrammed");
                let (colour, attr, code) = cell(&a.tilerom, base, c, r);
                code_hi = code_hi.max(code);
                colour_hi = colour_hi.max(colour);
                attr_bits |= attr;
            }
        }
        println!("  cells shown: highest code {code_hi:#06x} of {tiles:#06x}, highest colour {colour_hi}, attribute bits {attr_bits:#04x}");
        assert!(code_hi < tiles && colour_hi < 16 && attr_bits & !3 == 0, "{name}");
    }
    // Flip screen was exercised on both layers.
    assert!(bg.flipped > 0 && fg.flipped > 0);
    // The scroll test is the one place the 1987 code scrolls past column
    // 2047: it wraps, under MAME's 32,768-pixel map, onto the unprogrammed
    // tail and column 0.
    for (name, layer) in [("bg", &test_bg), ("fg", &test_fg)] {
        println!(
            "scroll test {name}: shown {} frames, scroll {:#06x}..={:#06x}, {} frames past column 2047, columns {:?}",
            layer.shown, layer.lo, layer.hi, layer.past, layer.runs()
        );
        assert!(layer.past > 0 && layer.hi >= 0x8000, "{name}");
    }
    let shown: Vec<String> = palettes.iter().map(|(src, off)| format!("{src:#07x}@{off:#05x}")).collect();
    println!("palette requests: {shown:?}");
    // Every palette the game requests is one of the 0x200-byte tables in
    // 0x1a268..0x1c068 (the pointer starts zeroed, before the first).
    assert!(palettes.iter().all(|&(src, _)| src == 0 || ((0x1a268..0x1c068).contains(&src) && (src - 0x1a268) % 0x200 == 0)));
}

/// What `background-format.md` states about the ROM map itself and the
/// tables that place the fight stages on it.
#[test]
fn rom_map_layout() {
    let Some(a) = assets() else { return };
    let t = &a.tilerom;
    assert_eq!(t.len(), 0x40000);
    assert_eq!((a.gfx1.len() / 128, a.gfx2.len() / 128), (0x1000, 0x2000), "background tiles in gfx1 and gfx2");
    assert_eq!(visible_rows(), 1..=14);
    for (name, base, tiles, tail) in [("bg", BG_MAP, 0x1000, 1956), ("fg", FG_MAP, 0x2000, 1108)] {
        // The map's tail is unprogrammed, every cell of every column from
        // `tail` on; no cell before it is.
        for c in 0..COLS {
            let blank = (0..ROWS).filter(|&r| erased(t, base, c, r)).count();
            assert_eq!(blank, if c >= tail { ROWS } else { 0 }, "{name} column {c}");
        }
        let cells = || (0..tail).flat_map(move |c| (0..ROWS).map(move |r| cell(t, base, c, r)));
        let code_hi = cells().map(|(_, _, code)| code).max().unwrap();
        let colour_hi = cells().map(|(colour, _, _)| colour).max().unwrap();
        let attr_bits = cells().fold(0u8, |b, (_, attr, _)| b | attr);
        println!("{name}: programmed columns 0..{tail}, highest code {code_hi:#06x}, highest colour {colour_hi}, attribute bits {attr_bits:#04x}");
        assert!(code_hi < tiles && colour_hi < 16 && attr_bits & !3 == 0, "{name}");
    }

    let m = &a.maincpu;
    let words = |table: usize, n: usize| -> Vec<u16> { (0..n).map(|i| rw(m, table + 2 * i)).collect() };
    let longs = |table: usize, n: usize| -> Vec<usize> { (0..n).map(|i| rl(m, table + 4 * i)).collect() };
    let cols = |first: u16, last: u16| (window(first).0 as usize / TILE, window(last).1 as usize / TILE);
    // Japan's tables (`0xffc006 = 0`), by opponent index stage * 2 + fight:
    // Retsu, Geki, Joe, Mike, Lee, Gen, Birdie, Eagle, Adon, Sagat.
    let fg = words(rl(m, 0x437ee), 10);
    let bg = words(rl(m, 0x437fe), 10);
    assert_eq!(fg, [0x1c80, 0x1f80, 0x2400, 0x2700, 0x2b80, 0x2e80, 0x3300, 0x3600, 0x3a80, 0x3d80]);
    // Each opponent's background block is 0x980 pixels (152 columns) from
    // 0x1500: his mode-2 VS screen, his mode-4 VS screen, his stage, the
    // human's win card and the CPU's.
    let tables = [(0x3fa66, 0x000), (0x3fad6, 0x180), (0x437fe, 0x300), (0x4018a, 0x680), (0x40250, 0x800)];
    for (table, at) in tables {
        let expect: Vec<u16> = (0..10).map(|i| 0x1500 + 0x980 * i + at).collect();
        assert_eq!(words(rl(m, table), 10), expect, "{table:#x}");
    }
    // By stage (country), Japan, USA, China, England, Thailand: the
    // background palette at offset 0 and the foreground's at 0x200.
    assert_eq!(longs(rl(m, 0x43778), 5), [0x1b068, 0x1b268, 0x1b468, 0x1b668, 0x1b868]);
    assert_eq!(longs(rl(m, 0x436e8), 5), [0x1a468, 0x1a668, 0x1a868, 0x1aa68, 0x1ac68]);
    // The other starting countries' tables hold the same words in another
    // order.
    let sorted = |mut v: Vec<u16>| {
        v.sort();
        v
    };
    for c in 1..4 {
        for (table, japan) in [(0x437ee, &fg), (0x437fe, &bg)] {
            assert_eq!(sorted(words(rl(m, table + 4 * c), 10)), sorted(japan.clone()), "{table:#x} country {c}");
        }
    }
    // The foreground: per country its intro (`0x3f8fa`, 24 columns) and its
    // two fights' stages (48 columns each: the camera's 0..=0x180), back to
    // back, 0x780 pixels per country from 0x1b00.
    let intro: Vec<u16> = (0..5).map(|k| rw(m, rl(m, 0x3f8fa) + 4 * k)).collect();
    for k in 0..5 {
        assert_eq!(intro[k], 0x1b00 + 0x780 * k as u16);
        let (i0, i1) = cols(intro[k], intro[k]);
        let (s0, s1) = cols(fg[2 * k], fg[2 * k + 1] + 0x180);
        assert_eq!((i1 - i0 + 1, s0, s1 - s0 + 1), (24, i1 + 1, 96), "country {k}");
    }
    // A fight's background shows its word plus the drift, 0..=0x200: 56
    // columns. The drift jumps from 0x200 back to 0, and every stage's first
    // 24 columns equal the 24 that start 32 columns on, so the jump never
    // shows.
    for (i, &s) in bg.iter().enumerate() {
        let (c0, c1) = cols(s, s + 0x200);
        assert_eq!(c1 - c0 + 1, 56);
        for c in c0..c0 + 24 {
            for r in visible_rows() {
                assert_eq!(cell(t, BG_MAP, c, r), cell(t, BG_MAP, c + 32, r), "stage {i} column {c} row {r}");
            }
        }
    }
    // Each map's last programmed column is the last one a ROM table can
    // show: the Player 2 ending's background (`0x44d5e`) and the
    // four-country flag screen's foreground (`0x4380`, set at `0x3e1e4`).
    assert_eq!(words(0x44d5c, 2), [0x7700, 0x7880]);
    assert_eq!(cols(0x7880, 0x7880).1, 1955);
    assert_eq!(cols(0x4380, 0x4380).1, 1107);
}
/// The renderer reads exactly the columns [`window`] names and the rows
/// [`visible_rows`] names, flipped or not: recolouring every other cell of
/// the map leaves the frame alone, and recolouring the window's first or
/// last column, or its first or last row, changes it.
#[test]
fn the_window_is_what_the_renderer_reads() {
    use sf_video::render::render_frame;
    use sf_video::scene::SceneState;
    let Some(a) = assets() else { return };
    // Every palette entry distinct.
    let paletteram: Vec<u8> = (0..0x400u16).flat_map(|i| (i.wrapping_mul(0x9e5) & 0x0fff).to_be_bytes()).collect();
    let recoloured = |keep: &dyn Fn(usize, usize) -> bool, base: usize| {
        let mut b = Assets { tilerom: a.tilerom.clone(), maincpu: Vec::new(), gfx1: a.gfx1.clone(), gfx2: a.gfx2.clone(), gfx3: a.gfx3.clone(), gfx4: a.gfx4.clone() };
        for c in 0..COLS {
            for r in 0..ROWS {
                if !keep(c, r) {
                    let at = base + 2 * (c * ROWS + r);
                    b.tilerom[at] = b.tilerom[at].wrapping_add(1) & 0x0f;
                }
            }
        }
        b
    };
    for s in [0x0000u16, 0x1843, 0x7700] {
        for flip in [false, true] {
            for (active, base, scroll) in [(0x20u8, BG_MAP, (s, 0)), (0x40, FG_MAP, (0, s))] {
                let scene = SceneState {
                    videoram: vec![0; 0x1000],
                    objectram: vec![0; 0x2000],
                    paletteram: paletteram.clone(),
                    bg_scroll: scroll.0,
                    fg_scroll: scroll.1,
                    active,
                    flip_screen: flip,
                    bg_page: 0,
                    fg_page: 0,
                };
                let (first, last) = window(s);
                let (c0, c1) = (first as usize / TILE, last as usize / TILE);
                let inside = |c: usize, r: usize| (c0..=c1).contains(&c) && visible_rows().contains(&r);
                let frame = render_frame(&a, &scene).argb;
                assert_eq!(render_frame(&recoloured(&inside, base), &scene).argb, frame, "{s:#06x} flip {flip} layer {active:#04x}");
                if active == 0x20 {
                    for edge in [c0, c1] {
                        let not_edge = |c: usize, _: usize| c != edge;
                        assert_ne!(render_frame(&recoloured(&not_edge, base), &scene).argb, frame, "{s:#06x} flip {flip} column {edge}");
                    }
                    for edge in [*visible_rows().start(), *visible_rows().end()] {
                        let not_edge = |_: usize, r: usize| r != edge;
                        assert_ne!(render_frame(&recoloured(&not_edge, base), &scene).argb, frame, "{s:#06x} flip {flip} row {edge}");
                    }
                }
            }
        }
    }
}
