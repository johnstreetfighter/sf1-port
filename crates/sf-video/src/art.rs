//! The art pipeline's loading seam (spec #164, ADR-0007): recipes in, the
//! extended sprite bank and each fighter's pose catalogue out.
//!
//! A **recipe** file lists one fighter's poses and the moves they make.
//! - A pose is either one of his ROM poses used as it is, which needs no
//!   bank tiles, or **built**. A built pose is pieced from parts of his ROM
//!   poses (turned, mirrored, cropped, placed) and cut into a rectangle of
//!   tiles in the bank.
//! - Or it is **generated**: its pens come from the art pack, named in the
//!   recipe by the CRC-32 of the approved file. Parts of his ROM poses may
//!   be drawn over it (his real head over a drifted face).
//! - Recipes hold no pixels: everything else is rebuilt from the player's
//!   ROM at launch, so the same ROM always gives the same tiles.
//! - Each pose carries an **anchor**: the top-left corner of its grid
//!   relative to the fighter's centre and the ground line, facing right.
//!   The SF1+ project turns it into frame-record offsets.
//!
//! A fighter keeps his ROM art, with the reason, when his reviewed set is
//! not all approved, or not finished (his recipe says what is still to
//! come), or a generated pose of his is missing from the pack or is not the
//! approved one (decision 9).

use serde::Deserialize;

use crate::art_pack::ArtPack;
use crate::art_review::{Review, Verdict};
use crate::decode::decode_sprite_tile;
use crate::romset::Assets;
use crate::sprite_bank::{sprite_slot, SpriteBank, ROM_SPRITE_TILES};

/// The sprite entries a fighter's object region holds.
pub const MAX_ENTRIES: usize = 24;
/// The transparent sprite pen.
pub const TRANSPARENT: u8 = 15;

/// A pose's tile grid as the engine's frame records give it: a base tile
/// code, `rows + 1` by `cols + 1` entries, 16-pixel or (large) 32-pixel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct Grid {
    #[serde(deserialize_with = "hex_u16")]
    pub code: u16,
    pub large: bool,
    pub rows: u8,
    pub cols: u8,
}

impl Grid {
    fn cell(&self) -> usize {
        if self.large {
            32
        } else {
            16
        }
    }

    pub fn width(&self) -> usize {
        (self.cols as usize + 1) * self.cell()
    }

    pub fn height(&self) -> usize {
        (self.rows as usize + 1) * self.cell()
    }

    pub fn entries(&self) -> usize {
        (self.rows as usize + 1) * (self.cols as usize + 1)
    }

    /// The tile code of every 16×16 tile the grid draws, with its pixel
    /// offset in the pose, facing right.
    pub fn tiles(&self) -> Vec<(u16, usize, usize)> {
        let mut out = Vec::new();
        let (col_code, row_code) = if self.large { (2u16, 0x20u16) } else { (1, 0x10) };
        for r in 0..=self.rows as u16 {
            for c in 0..=self.cols as u16 {
                let base = self.code.wrapping_add(r * row_code).wrapping_add(c * col_code);
                let (x, y) = (c as usize * self.cell(), r as usize * self.cell());
                if self.large {
                    out.extend([
                        (base, x, y),
                        (base.wrapping_add(1), x + 16, y),
                        (base.wrapping_add(16), x, y + 16),
                        (base.wrapping_add(17), x + 16, y + 16),
                    ]);
                } else {
                    out.push((base, x, y));
                }
            }
        }
        out
    }
}

/// A pose drawn as pens, row by row (`TRANSPARENT` where nothing is).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pens {
    pub width: usize,
    pub height: usize,
    pub pens: Vec<u8>,
}

impl Pens {
    fn blank(width: usize, height: usize) -> Self {
        Pens { width, height, pens: vec![TRANSPARENT; width * height] }
    }

    pub fn at(&self, x: usize, y: usize) -> u8 {
        self.pens[y * self.width + x]
    }

    /// The same pens mirrored left to right (the pose facing left).
    pub fn mirrored(&self) -> Pens {
        let mut out = Pens::blank(self.width, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                out.pens[y * self.width + x] = self.at(self.width - 1 - x, y);
            }
        }
        out
    }

    /// Draw a ROM grid from the sprite region, facing right.
    pub fn from_grid(gfx3: &[u8], grid: &Grid) -> Pens {
        let mut out = Pens::blank(grid.width(), grid.height());
        let count = gfx3.len() / 128;
        for (code, ox, oy) in grid.tiles() {
            let tile = decode_sprite_tile(gfx3, sprite_slot(code) % count);
            for (y, row) in tile.iter().enumerate() {
                for (x, &pen) in row.iter().enumerate() {
                    out.pens[(oy + y) * out.width + ox + x] = pen;
                }
            }
        }
        out
    }

    fn crop(&self, [x0, y0, w, h]: [usize; 4]) -> Result<Pens, String> {
        if x0 + w > self.width || y0 + h > self.height {
            return Err(format!("crop {x0},{y0} {w}x{h} falls outside a {}x{} pose", self.width, self.height));
        }
        let mut out = Pens::blank(w, h);
        for y in 0..h {
            for x in 0..w {
                out.pens[y * w + x] = self.at(x0 + x, y0 + y);
            }
        }
        Ok(out)
    }

    /// Turned clockwise by `quarter` quarter turns.
    fn turned(&self, quarter: u8) -> Pens {
        let mut p = self.clone();
        for _ in 0..quarter % 4 {
            let mut out = Pens::blank(p.height, p.width);
            for y in 0..p.height {
                for x in 0..p.width {
                    // (x, y) moves to (height - 1 - y, x).
                    out.pens[x * out.width + (p.height - 1 - y)] = p.at(x, y);
                }
            }
            p = out;
        }
        p
    }
}

fn hex_u16<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u16, D::Error> {
    let s = String::deserialize(d)?;
    u16::from_str_radix(s.trim_start_matches("0x"), 16).map_err(serde::de::Error::custom)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecipeFile {
    fighter: String,
    /// What his set still lacks, when it comes in parts (Sagat's ground set
    /// before his air set and specials). Until it is gone he keeps his ROM
    /// art in the game, however much is approved.
    #[serde(default)]
    still_to_come: Option<String>,
    poses: Vec<PoseRecipe>,
    moves: Vec<MoveRecipe>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PoseRecipe {
    name: String,
    /// A ROM pose used as it is.
    rom: Option<Grid>,
    /// Or a generated pose from the art pack.
    pack: Option<PackRef>,
    /// Or the parts of a built pose, drawn in order onto a canvas (over
    /// the generated pose, when there is one).
    parts: Option<Vec<Part>>,
    anchor: [i16; 2],
    #[serde(default)]
    note: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PackRef {
    /// The CRC-32 of the approved file, as eight hex digits.
    crc32: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Part {
    rom: Grid,
    #[serde(default)]
    crop: Option<[usize; 4]>,
    /// Clockwise quarter turns.
    #[serde(default)]
    turn: u8,
    #[serde(default)]
    mirror: bool,
    #[serde(default)]
    at: [usize; 2],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MoveRecipe {
    name: String,
    poses: Vec<String>,
    /// Game frames each pose shows for (a suggestion for review and SF1+).
    #[serde(default)]
    timing: Vec<u16>,
    #[serde(default)]
    note: String,
}

/// Where a catalogue pose's tiles come from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    /// One of the fighter's ROM poses, used as it is.
    Rom,
    /// Built from parts of his ROM poses into the bank.
    Recipe,
    /// Generated: from the art pack into the bank.
    Pack,
}

/// One pose in a fighter's catalogue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pose {
    pub name: String,
    pub source: Source,
    /// Its place: the ROM's own grid, or its rectangle in the bank.
    pub grid: Grid,
    /// The grid's top-left relative to the fighter's centre and the ground
    /// line, facing right.
    pub anchor: [i16; 2],
    /// A generated pose's art pack file, by CRC-32 (eight hex digits).
    pub pack: Option<String>,
    pub note: String,
}

/// One move: its poses in a suggested order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Move {
    pub name: String,
    pub poses: Vec<String>,
    /// Game frames each pose shows for, one per pose (a suggestion).
    pub timing: Vec<u16>,
    pub note: String,
}

/// A fighter's pose catalogue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Catalogue {
    pub fighter: String,
    pub poses: Vec<Pose>,
    pub moves: Vec<Move>,
}

impl Catalogue {
    pub fn pose(&self, name: &str) -> Option<&Pose> {
        self.poses.iter().find(|p| p.name == name)
    }

    /// The poses of move `name`, in order.
    pub fn move_poses(&self, name: &str) -> Option<Vec<&Pose>> {
        let m = self.moves.iter().find(|m| m.name == name)?;
        m.poses.iter().map(|p| self.pose(p)).collect()
    }
}

/// What the seam hands the game: the bank and the catalogues.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Art {
    pub bank: SpriteBank,
    pub catalogues: Vec<Catalogue>,
    /// The pens of every pose in the bank, by fighter and pose name (for
    /// drawing checks and the review page).
    pub built: Vec<(String, String, Pens)>,
    /// Fighters keeping their ROM art, and why.
    pub kept: Vec<(String, String)>,
}

/// What the art is made from.
pub struct ArtSources<'a> {
    /// Each fighter's recipe file.
    pub recipes: &'a [&'a str],
    /// Each fighter's review file. `None` loads every fighter whatever his
    /// review says (the review page itself).
    pub reviews: Option<&'a [&'a str]>,
    /// The art pack, or why there is none.
    pub pack: Result<&'a ArtPack, &'a str>,
}

impl Art {
    pub fn catalogue(&self, fighter: &str) -> Option<&Catalogue> {
        self.catalogues.iter().find(|c| c.fighter == fighter)
    }

    /// A line for the game's log.
    pub fn describe(&self) -> String {
        if self.catalogues.is_empty() {
            return "no fighter's new poses are ready".to_owned();
        }
        let mut parts = Vec::new();
        for c in &self.catalogues {
            let count = |s: Source| c.poses.iter().filter(|p| p.source == s).count();
            let mut d = format!(
                "{} ({}; {}, {} rebuilt",
                c.fighter,
                plural(c.moves.len(), "move"),
                plural(count(Source::Rom), "ROM pose"),
                count(Source::Recipe)
            );
            if count(Source::Pack) > 0 {
                d += &format!(", {} generated", count(Source::Pack));
            }
            parts.push(d + ")");
        }
        format!("{}; the bank spans {} tile slots", parts.join(", "), self.bank.tiles())
    }
}

/// `n thing` or `n things`.
pub fn plural(n: usize, thing: &str) -> String {
    format!("{n} {thing}{}", if n == 1 { "" } else { "s" })
}

/// The recipe without its generated poses and the moves that use them: a
/// fighter's art from his ROM alone. The art pack's pixels never enter the
/// public repository, so checks that run without it build from this.
pub fn rom_only(recipe: &str) -> Result<String, String> {
    let mut v: serde_json::Value = serde_json::from_str(recipe).map_err(|e| format!("a recipe file: {e}"))?;
    let poses = v["poses"].as_array_mut().ok_or("a recipe file: no poses")?;
    let generated: Vec<String> =
        poses.iter().filter(|p| p.get("pack").is_some()).filter_map(|p| p["name"].as_str().map(str::to_owned)).collect();
    poses.retain(|p| p.get("pack").is_none());
    if let Some(moves) = v["moves"].as_array_mut() {
        let uses_generated =
            |m: &serde_json::Value| m["poses"].as_array().is_some_and(|ps| ps.iter().any(|n| generated.iter().any(|g| n == g)));
        moves.retain(|m| !uses_generated(m));
    }
    serde_json::to_string(&v).map_err(|e| e.to_string())
}

/// Build the art from the ROM and the recipe files alone: every fighter,
/// whatever his review says, and no art pack.
pub fn load_art(rom: &Assets, recipes: &[&str]) -> Result<Art, String> {
    load_art_from(rom, &ArtSources { recipes, reviews: None, pack: Err("none was given") })
}

/// Build the art from the ROM and its sources. An error (a broken recipe
/// or review file) is the reason to fall back to the ROM's own graphics
/// for everyone. A fighter whose new poses are not ready keeps his ROM art
/// and is listed in [`Art::kept`].
pub fn load_art_from(rom: &Assets, sources: &ArtSources) -> Result<Art, String> {
    if rom.sprite_tiles() != ROM_SPRITE_TILES {
        return Err(format!("the sprite region holds {} tiles, not the ROM's {ROM_SPRITE_TILES}", rom.sprite_tiles()));
    }
    let reviews = match sources.reviews {
        Some(texts) => Some(
            texts
                .iter()
                .map(|t| serde_json::from_str::<Review>(t).map_err(|e| format!("a review file: {e}")))
                .collect::<Result<Vec<_>, _>>()?,
        ),
        None => None,
    };
    let mut art = Art { bank: SpriteBank::new(), catalogues: Vec::new(), built: Vec::new(), kept: Vec::new() };
    // Built poses are laid out one after another in whole 16-tile rows of
    // the sheet, from the bank's first row.
    let mut next_row = 0usize;
    for text in sources.recipes {
        let file: RecipeFile = serde_json::from_str(text).map_err(|e| format!("a recipe file: {e}"))?;
        let fighter = file.fighter;
        // First what each pose is; its pens wait until he is known to load.
        let mut poses = Vec::new();
        for p in &file.poses {
            let source = match (&p.rom, &p.pack, &p.parts) {
                (Some(grid), None, None) => {
                    check_rom_grid(&fighter, &p.name, grid)?;
                    if grid.entries() > MAX_ENTRIES {
                        return Err(format!("{fighter} {}: {} sprite entries, more than {MAX_ENTRIES}", p.name, grid.entries()));
                    }
                    Source::Rom
                }
                (None, None, Some(_)) => Source::Recipe,
                (None, Some(_), _) => Source::Pack,
                _ => return Err(format!("{fighter} {}: give a ROM pose, a pack pose or parts", p.name)),
            };
            let grid = p.rom.unwrap_or(UNPLACED);
            let pack = p.pack.as_ref().map(|r| r.crc32.to_ascii_lowercase());
            poses.push(Pose { name: p.name.clone(), source, grid, anchor: p.anchor, pack, note: p.note.clone() });
        }
        let mut moves = Vec::new();
        for m in file.moves {
            if let Some(missing) = m.poses.iter().find(|n| !poses.iter().any(|p: &Pose| &p.name == *n)) {
                return Err(format!("{fighter} {}: no pose named {missing}", m.name));
            }
            if !m.timing.is_empty() && m.timing.len() != m.poses.len() {
                return Err(format!("{fighter} {}: {} timings for {} poses", m.name, m.timing.len(), m.poses.len()));
            }
            let timing = if m.timing.is_empty() { vec![10; m.poses.len()] } else { m.timing };
            moves.push(Move { name: m.name, poses: m.poses, timing, note: m.note });
        }
        let mut catalogue = Catalogue { fighter, poses, moves };
        if let Some(reviews) = &reviews {
            if let Err(why) = check_review(reviews, &catalogue) {
                art.kept.push((catalogue.fighter, why));
                continue;
            }
            if let Some(rest) = &file.still_to_come {
                art.kept.push((catalogue.fighter, format!("his set is not finished, with {rest} still to come")));
                continue;
            }
        }
        // Then the pens of every pose that goes in the bank.
        let mut drawn = Vec::new();
        let mut missing = None;
        for p in &file.poses {
            if p.rom.is_some() {
                drawn.push(None);
                continue;
            }
            let base = match &p.pack {
                Some(r) => match pack_pens(sources.pack, &catalogue.fighter, &p.name, &r.crc32) {
                    Ok(pens) => Some(pens),
                    Err(why) => {
                        missing = Some(why);
                        break;
                    }
                },
                None => None,
            };
            let parts = p.parts.as_deref().unwrap_or(&[]);
            drawn.push(Some(build(&rom.gfx3, &catalogue.fighter, &p.name, base, parts)?));
        }
        if let Some(why) = missing {
            art.kept.push((catalogue.fighter, why));
            continue;
        }
        for (pose, pens) in catalogue.poses.iter_mut().zip(drawn) {
            let Some(pens) = pens else { continue };
            let at = |e: String| format!("{} {}: {e}", catalogue.fighter, pose.name);
            pose.grid = place(&mut art.bank, &pens, &mut next_row).map_err(at)?;
            if pose.grid.entries() > MAX_ENTRIES {
                return Err(at(format!("{} sprite entries, more than {MAX_ENTRIES}", pose.grid.entries())));
            }
            art.built.push((catalogue.fighter.clone(), pose.name.clone(), pens));
        }
        art.catalogues.push(catalogue);
    }
    Ok(art)
}

/// A pose's grid until it is placed in the bank.
const UNPLACED: Grid = Grid { code: 0, large: false, rows: 0, cols: 0 };

/// Every pose in his moves must be approved in his review file.
fn check_review(reviews: &[Review], catalogue: &Catalogue) -> Result<(), String> {
    let Some(old) = reviews.iter().find(|r| r.fighter == catalogue.fighter) else {
        return Err("his new poses have not been reviewed".to_owned());
    };
    let review = Review::for_catalogue(catalogue, Some(old));
    if review.approved() {
        return Ok(());
    }
    let waiting = review.poses.values().filter(|p| p.verdict != Verdict::Approved).count();
    Err(format!("{waiting} of his {} new poses are not approved yet", review.poses.len()))
}

/// A generated pose's pens, if the pack holds the approved file.
fn pack_pens(pack: Result<&ArtPack, &str>, fighter: &str, name: &str, crc: &str) -> Result<Pens, String> {
    let pack = pack.map_err(|why| format!("his generated pose {name} needs the art pack, and {why}"))?;
    let found = pack
        .pose(fighter, name)
        .ok_or_else(|| format!("the art pack has no {name} for him (the pack is out of date)"))?;
    if format!("{:08x}", found.crc32) != crc.to_ascii_lowercase() {
        return Err(format!(
            "the art pack's {name} is not the approved one (CRC-32 {:08x}; the recipe wants {crc})",
            found.crc32
        ));
    }
    Ok(found.pens.clone())
}

fn check_rom_grid(fighter: &str, name: &str, grid: &Grid) -> Result<(), String> {
    if grid.tiles().iter().any(|&(code, _, _)| sprite_slot(code) >= ROM_SPRITE_TILES) {
        return Err(format!("{fighter} {name}: its grid reaches past the ROM's tiles"));
    }
    Ok(())
}

/// Draw the parts, over the base when there is one, onto a canvas just big
/// enough for them.
fn build(gfx3: &[u8], fighter: &str, name: &str, base: Option<Pens>, parts: &[Part]) -> Result<Pens, String> {
    let mut drawn: Vec<(Pens, [usize; 2])> = base.into_iter().map(|b| (b, [0, 0])).collect();
    for part in parts {
        check_rom_grid(fighter, name, &part.rom)?;
        let mut p = Pens::from_grid(gfx3, &part.rom);
        if let Some(c) = part.crop {
            p = p.crop(c).map_err(|e| format!("{fighter} {name}: {e}"))?;
        }
        p = p.turned(part.turn);
        if part.mirror {
            p = p.mirrored();
        }
        drawn.push((p, part.at));
    }
    let width = drawn.iter().map(|(p, at)| at[0] + p.width).max().unwrap_or(0);
    let height = drawn.iter().map(|(p, at)| at[1] + p.height).max().unwrap_or(0);
    if width == 0 || height == 0 {
        return Err(format!("{fighter} {name}: no parts"));
    }
    let mut canvas = Pens::blank(width, height);
    for (p, [ax, ay]) in drawn {
        for y in 0..p.height {
            for x in 0..p.width {
                let pen = p.at(x, y);
                if pen != TRANSPARENT {
                    canvas.pens[(ay + y) * width + ax + x] = pen;
                }
            }
        }
    }
    Ok(canvas)
}

/// Cut `pens` into a grid in the bank: 16-pixel entries, or 32-pixel ones
/// when that many would pass the entry limit. The canvas is padded right
/// and down to whole cells.
fn place(bank: &mut SpriteBank, pens: &Pens, next_row: &mut usize) -> Result<Grid, String> {
    let fits = |cell: usize| pens.width.div_ceil(cell) * pens.height.div_ceil(cell);
    let large = fits(16) > MAX_ENTRIES;
    let cell = if large { 32 } else { 16 };
    let (cols, rows) = (pens.width.div_ceil(cell), pens.height.div_ceil(cell));
    let tiles_wide = cols * cell / 16;
    if tiles_wide > 16 {
        return Err(format!("{} pixels wide, more than the sheet's 16 tiles", pens.width));
    }
    let code = (ROM_SPRITE_TILES + *next_row * 16) as u16;
    let grid = Grid { code, large, rows: (rows - 1) as u8, cols: (cols - 1) as u8 };
    for (tile_code, ox, oy) in grid.tiles() {
        let tile: [[u8; 16]; 16] = std::array::from_fn(|y| {
            std::array::from_fn(|x| {
                let (px, py) = (ox + x, oy + y);
                if px < pens.width && py < pens.height {
                    pens.at(px, py)
                } else {
                    TRANSPARENT
                }
            })
        });
        bank.put(tile_code, &tile);
    }
    *next_row += rows * cell / 16;
    Ok(grid)
}
