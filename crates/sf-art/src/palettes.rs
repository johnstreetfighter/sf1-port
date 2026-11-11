//! The mirror palettes' review (spec #246, SP16): what the page shows of
//! each fighter's mirror palette, and the verdicts the maintainer gives it.
//!
//! - **Verdicts** are kept per fighter in `art/review/<fighter>-palette.json`,
//!   approve or reject with a reason. A verdict holds for the colours it was
//!   given on (the palette's CRC-32, [`MirrorPalette::digest`]): change the
//!   palette and the fighter is pending again, as a retried pose is. The
//!   rejections and their reasons stay in the file's history.
//! - **Poses** are a few of the fighter's own, drawn from his pens in his
//!   normal colours and in the mirror's, side by side.
//! - **Fight frames** are real: a Local mirror fight is driven through the
//!   engine with the gate on, so P2 is drawn as the game draws him.
//!
//! Only the body has a mirror palette; effects keep their colours.

use std::path::Path;

use serde::{Deserialize, Serialize};
use sf_engine::config::MatchConfig;
use sf_engine::front::cabinet::{drive_to_country_select, flow_position, pick_country, Players};
use sf_engine::roster::{PlayMode, TournamentFighter};
use sf_engine::run::PowerOnMachine;
use sf_video::art::{Catalogue, Pens, Source};
use sf_video::mirror_palette::{argb, MirrorPalette};
use sf_video::render::{render_frame, Frame, HEIGHT, WIDTH};
use sf_video::romset::Assets;

use crate::pack::Rgba;

/// The ROM's 16 sprite palettes, as the boot's palette upload copies them.
const ROM_SPRITE_PALETTES: usize = 0x1ba68;
/// The fight loop's position in slot 1.
const FIGHT_LOOP: u32 = 0x3eafa;

/// What a verdict says.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Verdict {
    Pending,
    Approved,
    Rejected,
}

/// One decision, kept in the history.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decision {
    /// The palette it was given on ([`MirrorPalette::digest`]).
    pub palette: String,
    pub verdict: Verdict,
    pub reason: String,
}

/// A fighter's verdict on his mirror palette.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaletteReview {
    pub fighter: String,
    /// The palette the current verdict was given on.
    pub palette: String,
    pub verdict: Verdict,
    /// Why, for the current verdict (empty for an approval with no note).
    pub reason: String,
    pub rejections: u32,
    pub history: Vec<Decision>,
}

impl PaletteReview {
    /// A fresh review: pending, nothing decided.
    pub fn pending(palette: &MirrorPalette) -> PaletteReview {
        PaletteReview {
            fighter: palette.fighter.clone(),
            palette: palette.digest(),
            verdict: Verdict::Pending,
            reason: String::new(),
            rejections: 0,
            history: Vec::new(),
        }
    }

    /// What was saved for this fighter, made current for `palette`: a verdict
    /// given on other colours is pending again (its reason and the history
    /// stay for the maintainer to read).
    pub fn current(old: Option<PaletteReview>, palette: &MirrorPalette) -> PaletteReview {
        match old {
            Some(r) if r.fighter == palette.fighter => {
                if r.palette == palette.digest() {
                    r
                } else {
                    PaletteReview { palette: palette.digest(), verdict: Verdict::Pending, ..r }
                }
            }
            _ => PaletteReview::pending(palette),
        }
    }

    /// Record a verdict on the palette now committed. A rejection must say
    /// why.
    pub fn decide(&mut self, palette: &MirrorPalette, approve: bool, reason: &str) -> Result<(), String> {
        let reason = reason.trim();
        if !approve && reason.is_empty() {
            return Err("a rejection needs a reason".to_owned());
        }
        self.palette = palette.digest();
        self.verdict = if approve { Verdict::Approved } else { Verdict::Rejected };
        self.reason = reason.to_owned();
        if !approve {
            self.rejections += 1;
        }
        self.history.push(Decision { palette: self.palette.clone(), verdict: self.verdict, reason: self.reason.clone() });
        Ok(())
    }

    /// The fighter is approved when the verdict is one and was given on the
    /// colours now committed.
    pub fn approved_for(&self, palette: &MirrorPalette) -> bool {
        self.verdict == Verdict::Approved && self.palette == palette.digest()
    }

    pub fn load(path: &Path) -> Option<PaletteReview> {
        serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())? + "\n";
        std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
    }
}

/// `art/review/<fighter>-palette.json` under the repository's `art` folder.
pub fn review_path(art: &Path, fighter: &str) -> std::path::PathBuf {
    art.join("review").join(format!("{fighter}-palette.json"))
}

// ---- colours -----------------------------------------------------------------

/// The ROM's sprite palette `palette` as 16 xRGB_444 words.
pub fn rom_palette(rom: &Assets, palette: u8) -> [u16; 16] {
    let at = ROM_SPRITE_PALETTES + palette as usize * 32;
    std::array::from_fn(|pen| u16::from_be_bytes([rom.maincpu[at + 2 * pen], rom.maincpu[at + 2 * pen + 1]]) & 0xfff)
}

/// The 16 ARGB colours of a mirror palette: its pens, then the replaced
/// palette's transparent pen.
pub fn mirror_colours(rom: &Assets, mirror: &MirrorPalette) -> [u32; 16] {
    let mut out = [0u32; 16];
    for (pen, word) in mirror.pens.iter().enumerate() {
        out[pen] = argb(*word);
    }
    out[15] = argb(rom_palette(rom, mirror.from)[15]);
    out
}

/// The 16 ARGB colours of ROM sprite palette `palette`.
pub fn palette_colours(rom: &Assets, palette: u8) -> [u32; 16] {
    rom_palette(rom, palette).map(argb)
}

/// The colours a mirror palette may be picked from: the fighter's own
/// palette.
pub fn own_colours(rom: &Assets, fighter: TournamentFighter) -> Vec<u16> {
    let mut out: Vec<u16> = rom_palette(rom, fighter.sprite_palette())[..15].to_vec();
    out.sort_unstable();
    out.dedup();
    out
}

// ---- pictures ----------------------------------------------------------------

/// A picture of `pens` in `colours`, nearest-neighbour `scale` times larger.
fn drawn(pens: &Pens, colours: &[u32; 16], scale: usize) -> Rgba {
    let small = Rgba::from_pens(pens, colours);
    let (w, h) = (small.width * scale, small.height * scale);
    let mut bytes = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let from = ((y / scale) * small.width + x / scale) * 4;
            bytes[(y * w + x) * 4..(y * w + x) * 4 + 4].copy_from_slice(&small.bytes[from..from + 4]);
        }
    }
    Rgba { width: w, height: h, bytes }
}

/// The page's backdrop for a picture: a mid grey that shows black outlines
/// and white clothes alike.
const BACKDROP: [u8; 3] = [0x5c, 0x5c, 0x6a];

/// `pictures` side by side on the backdrop, each bottom-aligned in a cell of
/// the widest and tallest, with a margin.
fn compose(pictures: &[Rgba]) -> Rgba {
    let margin = 8;
    let cell_w = pictures.iter().map(|p| p.width).max().unwrap_or(0);
    let cell_h = pictures.iter().map(|p| p.height).max().unwrap_or(0);
    let (w, h) = (margin + pictures.len() * (cell_w + margin), 2 * margin + cell_h);
    let mut bytes = Vec::with_capacity(w * h * 4);
    for _ in 0..w * h {
        bytes.extend([BACKDROP[0], BACKDROP[1], BACKDROP[2], 255]);
    }
    for (i, p) in pictures.iter().enumerate() {
        let (ox, oy) = (margin + i * (cell_w + margin) + (cell_w - p.width) / 2, margin + cell_h - p.height);
        for y in 0..p.height {
            for x in 0..p.width {
                let a = p.bytes[(y * p.width + x) * 4 + 3];
                if a == 255 {
                    let to = ((oy + y) * w + ox + x) * 4;
                    bytes[to..to + 3].copy_from_slice(&p.bytes[(y * p.width + x) * 4..(y * p.width + x) * 4 + 3]);
                }
            }
        }
    }
    Rgba { width: w, height: h, bytes }
}

/// One pose or sprite in his normal colours and in the mirror's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pair {
    pub label: String,
    /// A PNG: normal on the left, mirror on the right.
    pub png: Vec<u8>,
}

fn pair(label: &str, pens: &Pens, normal: &[u32; 16], mirror: &[u32; 16]) -> Pair {
    let scale = if pens.height > 100 { 2 } else { 3 };
    Pair { label: label.to_owned(), png: compose(&[drawn(pens, normal, scale), drawn(pens, mirror, scale)]).png() }
}

/// The few poses of a fighter the page shows: the stance, the crouch, and
/// others spread over his catalogue, up to [`POSES`].
pub const POSES: usize = 4;

pub fn chosen_poses(catalogue: &Catalogue) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for want in ["stance", "crouch"] {
        if catalogue.pose(want).is_some() {
            names.push(want.to_owned());
        }
    }
    let rest: Vec<&str> = catalogue.poses.iter().map(|p| p.name.as_str()).filter(|n| !names.iter().any(|x| x == n)).collect();
    for k in 0..(POSES - names.len().min(POSES)) {
        if rest.is_empty() {
            break;
        }
        let name = rest[(k * rest.len() / (POSES - 2).max(1) + rest.len() / 4).min(rest.len() - 1)];
        if !names.iter().any(|n| n == name) {
            names.push(name.to_owned());
        }
    }
    names
}

/// The poses side by side. `banked` is the ROM's graphics with the art
/// bank on top, so built poses draw too; a pose whose art is a generated
/// file the pack lacks is left out.
pub fn pose_pairs(banked: &Assets, catalogue: &Catalogue, normal: &[u32; 16], mirror: &[u32; 16]) -> Vec<Pair> {
    let mut out = Vec::new();
    for name in chosen_poses(catalogue) {
        let Some(pose) = catalogue.pose(&name) else { continue };
        if pose.source == Source::Pack && banked.sprite_tiles() <= sf_video::sprite_bank::ROM_SPRITE_TILES {
            continue;
        }
        out.push(pair(&name, &Pens::from_grid(&banked.gfx3, &pose.grid), normal, mirror));
    }
    out
}

// ---- real fight frames -------------------------------------------------------

/// A Local mirror fight of `fighter` against himself, past the walk-in, with
/// the gate on (the default preset): the machine, at the fight loop.
pub fn mirror_fight(rom: &Assets, fighter: TournamentFighter) -> Result<PowerOnMachine, String> {
    let mut config = MatchConfig::default();
    config.roster.mode = PlayMode::Versus;
    config.roster.full_cast = true;
    config.roster.p1 = fighter;
    config.roster.p2 = fighter;
    let mut m = PowerOnMachine::new(rom.maincpu.clone()).with_match_config(config);
    drive_to_country_select(&mut m, Players::Two, 4_000).map_err(|e| format!("the drive: {e:?}"))?;
    pick_country(&mut m, &rom.maincpu, 0, 2_000).map_err(|e| format!("the flag: {e:?}"))?;
    let mut f = 0;
    while flow_position(&m) != FIGHT_LOOP {
        m.step();
        f += 1;
        if f > 3_000 {
            return Err("the fight loop never came".to_owned());
        }
    }
    Ok(m)
}

/// The inputs for the page's frames: `(directions, buttons)` for each side
/// on fight frame `f` (counted from 60 frames into the round).
type Inputs = ([bool; 4], [bool; 6]);

fn walk_in(_f: u32) -> (Inputs, Inputs) {
    (([true, false, false, false], [false; 6]), ([false, true, false, false], [false; 6]))
}

/// A punch from both sides, held three frames.
fn punch(f: u32) -> (Inputs, Inputs) {
    let mut b = [false; 6];
    b[0] = f < 3;
    (([false; 4], b), ([false; 4], b))
}

fn feed(m: &mut PowerOnMachine, (p1, p2): (Inputs, Inputs)) {
    m.set_p1_direction(p1.0[0], p1.0[1], p1.0[2], p1.0[3]);
    m.set_p1_buttons(p1.1);
    m.set_p2_direction(p2.0[0], p2.0[1], p2.0[2], p2.0[3]);
    m.set_p2_buttons(p2.1);
    m.step();
}

/// Frames of a Local mirror fight, rendered with the ROM's graphics: both
/// standing and both in a punch.
pub fn mirror_frames(rom: &Assets, fighter: TournamentFighter) -> Result<Vec<(String, Frame)>, String> {
    let mut m = mirror_fight(rom, fighter)?;
    for _ in 0..60 {
        m.step();
    }
    let mut out = vec![("standing".to_owned(), render_frame(rom, &m.state.scene()))];
    for f in 0..70 {
        feed(&mut m, walk_in(f));
    }
    for _ in 0..12 {
        feed(&mut m, (([false; 4], [false; 6]), ([false; 4], [false; 6])));
    }
    for f in 0..8 {
        feed(&mut m, punch(f));
    }
    out.push(("punching".to_owned(), render_frame(rom, &m.state.scene())));
    if !m.unrecovered().is_empty() {
        return Err(format!("untranslated {:05x?}", m.unrecovered()));
    }
    Ok(out)
}

/// A frame as a PNG.
pub fn frame_png(frame: &Frame) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(WIDTH * HEIGHT * 4);
    for &p in &frame.argb {
        bytes.extend([(p >> 16) as u8, (p >> 8) as u8, p as u8, 255]);
    }
    Rgba { width: WIDTH, height: HEIGHT, bytes }.png()
}

// ---- readability ---------------------------------------------------------------

/// How far apart two ARGB colours are: the distance between their RGB
/// triples (0 to about 441).
pub fn colour_distance(a: u32, b: u32) -> f64 {
    let c = |x: u32, shift: u32| ((x >> shift) & 0xff) as f64;
    ((c(a, 16) - c(b, 16)).powi(2) + (c(a, 8) - c(b, 8)).powi(2) + (c(a, 0) - c(b, 0)).powi(2)).sqrt()
}

/// How different the mirror reads from the normal palette on `pens` (a
/// pose): the share of its opaque pixels drawn in a colour at least
/// `apart` away from the one the normal palette draws there.
pub fn changed_share(pens: &Pens, normal: &[u32; 16], mirror: &[u32; 16], apart: f64) -> f64 {
    let opaque: Vec<u8> = pens.pens.iter().copied().filter(|&p| p != sf_video::art::TRANSPARENT).collect();
    if opaque.is_empty() {
        return 0.0;
    }
    let changed = opaque.iter().filter(|&&p| colour_distance(normal[p as usize], mirror[p as usize]) >= apart).count();
    changed as f64 / opaque.len() as f64
}
