//! Ticket FE5 (#155): the portrait select's busts are the VS screens' busts.
//!
//! `sf_video::busts::decode_busts` reads the twelve busts from the ROM
//! regions alone. This test checks them against what the native machine
//! draws: it replays two committed scenarios from power-on (the native
//! replays are verified against MAME by `full_1p.rs` and the versus start
//! tests) and, on every frame a VS screen is up, renders the frame and
//! compares each bust with the frame's pixels at the bust's placement.
//!
//! - `sf-full-1p-01`: a one-player game from the Japan flag through Sagat.
//!   Its ten VS screens (event 4, `0x3f9ec`) show Ryu on the left and the
//!   ten opponents on the right, in the order Retsu, Geki, Joe, Mike, Lee,
//!   Gen, Birdie, Eagle, Adon, Sagat.
//! - `sf-start-2p-01`: the two-player start; its VS screen is RYU VS KEN.
//!
//! A VS screen is "up" when the background layer is on (gfxctrl bit 5), the
//! background scroll register holds the bust's VS screen and palette RAM's
//! background entries (offset 0, 0x200 bytes) equal the bust's ROM palette
//! table. Ryu's bust is compared on all eleven VS screens.
//!
//! Where the local MAME captures exist, the busts are also compared with the
//! capture frames `tools/player_select.py` crops, and the crops it wrote to
//! `target/select-busts/*.raw` are checked to be sub-rectangles of the busts.
//! Everything skips with a note when the local ROMs are absent.

use std::path::Path;

use sf_engine::roster::TournamentFighter;
use sf_engine::run::{observe_power_on_scenario, PowerOnMachine};
use sf_video::busts::{decode_busts_placed, Bust, BustPlacement, FIGHTERS};
use sf_video::render::{render_frame, HEIGHT, WIDTH};
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const FULL_1P: &str = "../../tools/reference/scenarios/sf_full_1p_01.json";
const START_2P: &str = "../../tools/reference/scenarios/sf_start_2p_01.json";
const CAPTURES: &str = "../../tools/reference/local/captures";
const CROPS: &str = "../../target/select-busts";

/// Through Sagat's VS screen (his round 1 starts at `37153`).
const FULL_1P_LAST: u32 = 37_153;
/// The scenario's last frame (`duration_frames = 2460`).
const START_2P_LAST: u32 = 2_459;

/// Palette RAM bytes the VS palettes fill: the background's 256 entries.
const VS_PALETTE_BYTES: usize = 0x200;

/// The MAME capture frames `tools/player_select.py` crops each bust from.
const CAPTURE_FRAMES: [(&str, u32); 12] = [
    ("prefight-01-run-a", 2100),
    ("start-2p-01-run-a", 2004),
    ("prefight-01-run-a", 2100),
    ("win-01-run-a", 5457),
    ("win-02-run-a", 10440),
    ("win-03-run-a", 13320),
    ("win-04-run-a", 19003),
    ("win-05-run-a", 21868),
    ("win-06-run-a", 25991),
    ("win-07-run-a", 28995),
    ("win-08-run-a", 33797),
    ("win-09-run-a", 36748),
];

fn assets() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROMs not present");
        return None;
    }
    Some(load_assets(ROMS, MANIFEST).unwrap())
}

/// The decoded busts with their placements, all twelve present.
fn placed(assets: &Assets) -> Vec<(Bust, BustPlacement)> {
    decode_busts_placed(assets)
        .into_iter()
        .enumerate()
        .map(|(i, b)| b.unwrap_or_else(|| panic!("{} has no bust", FIGHTERS[i])))
        .collect()
}

/// Pixels of `bust` that differ from `frame` (384x224 ARGB) at its placement.
fn mismatches(frame: &[u32], bust: &Bust, p: &BustPlacement) -> usize {
    assert!(p.x + bust.width <= WIDTH && p.y + bust.height <= HEIGHT);
    (0..bust.height)
        .map(|y| {
            let row = &frame[(p.y + y) * WIDTH + p.x..][..bust.width];
            let want = &bust.argb[y * bust.width..][..bust.width];
            row.iter().zip(want).filter(|(a, b)| a != b).count()
        })
        .sum()
}

/// Per fighter: VS-screen frames compared, the first and last of them, the
/// differing pixels summed over them, and (negative control) the differing
/// pixels with the bust compared one pixel to the right of its placement.
#[derive(Clone, Copy, Default)]
struct Seen {
    frames: u32,
    first: Option<u32>,
    last: u32,
    differing: usize,
    shifted: usize,
}

impl Seen {
    fn add(&mut self, frame: &[u32], number: u32, bust: &Bust, p: &BustPlacement) {
        self.frames += 1;
        self.first.get_or_insert(number);
        self.last = number;
        self.differing += mismatches(frame, bust, p);
        let right = BustPlacement { x: p.x + 1, ..*p };
        self.shifted += mismatches(frame, bust, &right);
    }
}

/// On a frame where fighter `i`'s VS screen is up (`i` >= 1: the right-hand
/// bust; Ryu is on the left of them all), render it and compare both busts.
fn observe(assets: &Assets, busts: &[(Bust, BustPlacement)], m: &PowerOnMachine, seen: &mut [Seen; 12]) {
    let s = &m.state;
    if s.gfxctrl & 0x20 == 0 {
        return;
    }
    for i in 1..12 {
        let (bust, p) = &busts[i];
        let palette = &assets.maincpu[p.palette..p.palette + VS_PALETTE_BYTES];
        if s.bg_scroll != p.bg_scroll || s.paletteram[..VS_PALETTE_BYTES] != *palette {
            continue;
        }
        let frame = render_frame(assets, &s.scene());
        seen[i].add(&frame.argb, s.frame, bust, p);
        let (ryu, rp) = &busts[0];
        seen[0].add(&frame.argb, s.frame, ryu, rp);
    }
}

#[test]
fn the_decoded_busts_are_what_the_native_vs_screens_draw() {
    let Some(assets) = assets() else { return };
    // The select order is the roster's.
    for (f, name) in TournamentFighter::ALL.iter().zip(FIGHTERS) {
        assert_eq!(f.name(), name);
    }
    let busts = placed(&assets);
    let mut seen = [Seen::default(); 12];

    for (scenario, last) in [(FULL_1P, FULL_1P_LAST), (START_2P, START_2P_LAST)] {
        let (stopped, untranslated) =
            observe_power_on_scenario(assets.maincpu.clone(), Path::new(scenario), last, |m| {
                observe(&assets, &busts, m, &mut seen)
            })
            .unwrap();
        assert!(
            untranslated.is_empty(),
            "{scenario}: untranslated branches {untranslated:05x?} at frame {stopped}"
        );
        assert_eq!(stopped, last);
    }

    for (i, s) in seen.iter().enumerate() {
        let (bust, p) = &busts[i];
        // Ryu's frames span both scenarios; the others' one VS screen each.
        let when = if i == 0 {
            "on all eleven VS screens".to_string()
        } else {
            format!("({}..={})", s.first.unwrap_or(0), s.last)
        };
        eprintln!(
            "{:7} {}x{} at ({}, {}), decoded from bg 0x{:04x} palette 0x{:05x}: {} frames {when}, {} differing pixels ({} one pixel to the right)",
            FIGHTERS[i],
            bust.width,
            bust.height,
            p.x,
            p.y,
            p.bg_scroll,
            p.palette,
            s.frames,
            s.differing,
            s.shifted
        );
    }
    for (i, s) in seen.iter().enumerate() {
        // A VS screen holds 120 + 30 frames (0x3fb28 / 0x3fb2e one-player,
        // 0x3fa46 / 0x3fa4c versus).
        assert!(s.frames >= 120, "{}'s VS screen was up for {} frames", FIGHTERS[i], s.frames);
        assert_eq!(s.differing, 0, "{}'s bust differs from the native VS screen", FIGHTERS[i]);
        assert!(s.shifted > 0, "{}: the comparison must see a one-pixel shift", FIGHTERS[i]);
    }
    // Ryu is on all eleven screens.
    assert_eq!(seen[0].frames, seen[1..].iter().map(|s| s.frames).sum::<u32>());
}

/// A capture frame as ARGB (the `.rgba` files are B, G, R, A bytes).
fn capture_frame(path: &Path) -> Vec<u32> {
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(bytes.len(), WIDTH * HEIGHT * 4);
    bytes
        .chunks_exact(4)
        .map(|p| 0xff00_0000 | (p[2] as u32) << 16 | (p[1] as u32) << 8 | p[0] as u32)
        .collect()
}

#[test]
fn the_decoded_busts_match_the_mame_vs_captures() {
    let Some(assets) = assets() else { return };
    let busts = placed(&assets);
    let mut compared = 0;
    for (i, (dir, frame)) in CAPTURE_FRAMES.iter().enumerate() {
        let path = Path::new(CAPTURES).join(dir).join(format!("pixels_{frame:06}.rgba"));
        if !path.is_file() {
            eprintln!("{}: capture {} not present", FIGHTERS[i], path.display());
            continue;
        }
        let (bust, p) = &busts[i];
        let capture = capture_frame(&path);
        let differing = mismatches(&capture, bust, p);
        let shifted = mismatches(&capture, bust, &BustPlacement { x: p.x + 1, ..*p });
        eprintln!(
            "{:7} vs {dir} frame {frame}: {differing} differing pixels ({shifted} one pixel to the right)",
            FIGHTERS[i]
        );
        assert_eq!(differing, 0, "{}'s bust differs from the MAME capture", FIGHTERS[i]);
        assert!(shifted > 0, "{}: the comparison must see a one-pixel shift", FIGHTERS[i]);
        compared += 1;
    }
    if compared == 0 {
        eprintln!("skipping: local captures not present");
    }
}

/// A `tools/player_select.py` crop: width and height as little-endian u16,
/// then R, G, B, A bytes.
fn crop(path: &Path) -> (usize, usize, Vec<u32>) {
    let bytes = std::fs::read(path).unwrap();
    let w = u16::from_le_bytes([bytes[0], bytes[1]]) as usize;
    let h = u16::from_le_bytes([bytes[2], bytes[3]]) as usize;
    let argb = bytes[4..]
        .chunks_exact(4)
        .map(|p| 0xff00_0000 | (p[0] as u32) << 16 | (p[1] as u32) << 8 | p[2] as u32)
        .collect::<Vec<_>>();
    assert_eq!(argb.len(), w * h);
    (w, h, argb)
}

/// The capture crops cut the largest 4-connected blob of non-backdrop
/// pixels with a one-pixel border; the decoder keeps every pixel the bust
/// tiles draw. So each crop is the same picture, equal to a sub-rectangle of
/// the decoded bust: the whole bust, or the bust less the columns of a
/// detached pixel or of the border beside one.
#[test]
fn the_capture_crops_are_sub_rectangles_of_the_decoded_busts() {
    let Some(assets) = assets() else { return };
    let busts = placed(&assets);
    let mut compared = 0;
    for (i, (bust, _)) in busts.iter().enumerate() {
        let path = Path::new(CROPS).join(format!("{}.raw", FIGHTERS[i].to_lowercase()));
        if !path.is_file() {
            continue;
        }
        let (w, h, argb) = crop(&path);
        assert!(w <= bust.width && h <= bust.height, "{}: crop larger than the bust", FIGHTERS[i]);
        let at = (0..=bust.height - h)
            .flat_map(|dy| (0..=bust.width - w).map(move |dx| (dx, dy)))
            .find(|&(dx, dy)| {
                (0..h).all(|y| argb[y * w..][..w] == bust.argb[(y + dy) * bust.width + dx..][..w])
            });
        let Some((dx, dy)) = at else {
            panic!("{}: the capture crop is not part of the decoded bust", FIGHTERS[i]);
        };
        eprintln!(
            "{:7} crop {w}x{h} = bust {}x{} at ({dx}, {dy})",
            FIGHTERS[i], bust.width, bust.height
        );
        compared += 1;
    }
    if compared == 0 {
        eprintln!("skipping: target/select-busts not present (run tools/player_select.py)");
    }
}
