//! The ROM box audit (spec #246, `SP3` #249): every move the 1987 ROM gives
//! a tournament fighter, with its boxes measured against what its frames
//! draw.
//!
//! **Read only.** The audit reads the program ROM (`maincpu`) and the sprite
//! region (`gfx3`) as data. It runs no machine, writes no RAM and changes no
//! box or behaviour; `tests/rom_box_audit.rs` turns its measurements into
//! `docs/reference/rom-box-audit.md` and the pictures beside it.
//!
//! # What is measured
//!
//! A frame record (0x20 bytes, `fighter::load_frame`) carries a pose (a
//! grid of 16 or 32 pixel sprites) and the frame's boxes. Every box is an
//! offset from the **grid centre**: `place_frame` (`0x43b6a`) always puts
//! the grid's centre on the fighter's centre as the engine holds it, so a box
//! centred at `(cx + dx, cy - dy)` with half sizes `(hw, hh)` sits at the
//! same place on the drawn pose whatever the facing (a left-facing frame
//! negates the `dx` bytes and mirrors the grid). The audit therefore works
//! on the pose facing right, in grid pixels with the origin at the grid's
//! top-left. That centre is not fixed to the fighter, though: each frame's
//! x-offset (`+0x26`) and its per-tick step (`+0x15`) move it (a lunge, a
//! jump, a turn), so the fighter's own position is the stance centre, with
//! the stance's x-offset 0, and distances from it need both.
//!
//! - **Attack box** (`+0x22..+0x25`, live while flags bit 7 is set): its
//!   *overreach* in each of four directions is how far the box's edge lies
//!   past the farthest drawn pixel inside the box's own band (the rows it
//!   spans for right and left, the columns for up and down). A box that
//!   contains no drawn pixel at all is *floating*.
//! - **Body**: the loose hurtbox (`+0x1e..+0x21`, the outer box a hit must
//!   touch) against the drawn pixels: the share of drawn pixels outside it
//!   (*uncovered*) and how far the farthest sticks out of it. The tight box
//!   (`+0x1a..+0x1d`, the critical box) is carried along for the pictures.
//! - **Startup** is counted on the research's convention (the first active
//!   frame counts: a move that hits on its fifth tick has startup 5):
//!   `1 +` the ticks of every frame before the first one with flags bit 7.
//!   A frame lasts `(repeat + 1) * (duration + 1)` ticks (`animate`,
//!   `0x42be2`); the move's header pose, when it has one, is its first
//!   frame. The count starts when the move starts, not when the button was
//!   pressed (the 1987 game starts a move one frame after the button is
//!   released, `punch.md`).
//! - **Reach** is `x_offset + dx + hw` of the attack frame: pixels from
//!   the fighter's stance centre to the attack box's forward edge. The
//!   frame's x-offset is part of it because `load_frame` writes the offset
//!   into the grid's first sprite and the judge (`0x43e82`) measures from
//!   the grid's centre. **Travel** is how far the frame steps carry the
//!   centre (`+0x15` once on load and once per repeat), reported apart.
//! - **A zero-size box is a point**, not a missing box: the judge has no
//!   test for an empty box, so a loose or tight box of size zero is a single
//!   point at `(dx, -dy)` from the centre, which an attack box can still
//!   touch (a zero tight box makes a hit on that point critical).
//!
//! The thresholds are constants below, chosen on the art's own grain (a
//! sprite tile is 16 pixels) and the research file's numbers; the report
//! states them and what each one means.

use std::collections::{BTreeMap, BTreeSet};

use crate::fighter::pose_grid_entries;
use crate::roster::TournamentFighter;
use sf_video::decode::decode_sprite_tile;
use sf_video::sprite_bank::sprite_slot;

/// The sprite entries a fighter's object region holds (`park_from`).
pub const MAX_ENTRIES: usize = 24;
/// The transparent sprite pen.
pub const TRANSPARENT: u8 = 15;

/// An attack box reaching this far (px) past the drawn pixels in its band is
/// flagged: a whole sprite tile, which is also close to Tukey's outer fence
/// (third quartile plus 1.5 interquartile ranges, 18 px) for the ROM's own
/// attack frames, whose median is 5 px.
pub const OVERREACH_FLAG: i32 = 16;
/// A loose hurtbox reaching this far (px) past the drawn pixels in its band
/// is flagged: a tile and a half. (Third quartile 6 px, 90th percentile 12.)
pub const BODY_OVERREACH_FLAG: i32 = 24;
/// A frame's drawing is flagged when this much more of it (share, as a
/// fraction of the drawn pixels) lies outside the loose hurtbox than in the
/// fighter's own stance. The 1987 hurtbox is a torso box that leaves arms,
/// legs and everything extended outside by design (a stance leaves 16% to
/// 43% outside), so the baseline is the fighter's stance; 0.30 is about the
/// ROM's own 90th percentile.
pub const BODY_EXCESS_FLAG: f64 = 0.30;
/// A tight box sticking this far (px) further out of the loose box than the
/// fighter's own stance's does is flagged: the judge needs an attack to touch
/// the loose box before it looks at the tight one, so an attack cannot score
/// a critical hit through the part of the critical box outside the loose box
/// alone (a box that also touches the loose box still can).
pub const TIGHT_OUTSIDE_FLAG: i32 = 16;
/// An attack move with this many ticks of a zero-size loose hurtbox is
/// flagged. A zero-size box is a single point at the box's offset from the
/// centre (the judge has no empty-box test): the fighter can still be hit
/// there, but only by an attack box that covers that point.
pub const POINT_HURTBOX_FLAG: u32 = 3;
/// The fastest melee normal startup the research lists (first active frame
/// counted): 3 frames (research 3.2 and 3.1: USF4 Adon and Gen, CvS2 Eagle,
/// Alpha 3 Gen's close jab).
pub const FASTEST_RESEARCH_NORMAL: u32 = 3;
/// A melee move that starts faster than the research's fastest (that is, in
/// at most this many ticks) is flagged. Startup 3 is at the research's
/// floor and is shown but not flagged.
pub const FAST_STARTUP: u32 = FASTEST_RESEARCH_NORMAL - 1;
/// A projectile that spawns faster than this (research 5.2: 12 to 16) is
/// flagged as fast.
pub const FAST_PROJECTILE_STARTUP: u32 = 12;
/// A reach of this many pixels or more from the fighter's stance centre
/// (`x_offset + dx + hw`) is flagged long: six sprite tiles, a little under
/// Tukey's outer fence (third quartile plus 1.5 interquartile ranges, which
/// the report prints) over the ROM's attack moves. The research has one
/// pixel figure for reach, Super Turbo's throw range of 48 px from the axis
/// (3.4).
pub const LONG_REACH: i32 = 96;

/// The fighters' tournament order and the ten bosses, for the report.
pub const BOSSES: [TournamentFighter; 10] = [
    TournamentFighter::Retsu,
    TournamentFighter::Geki,
    TournamentFighter::Joe,
    TournamentFighter::Mike,
    TournamentFighter::Lee,
    TournamentFighter::Gen,
    TournamentFighter::Birdie,
    TournamentFighter::Eagle,
    TournamentFighter::Adon,
    TournamentFighter::Sagat,
];

fn rb(rom: &[u8], a: u32) -> u8 {
    rom.get(a as usize).copied().unwrap_or(0xff)
}

fn rw(rom: &[u8], a: u32) -> u16 {
    u16::from_be_bytes([rb(rom, a), rb(rom, a + 1)])
}

fn rl(rom: &[u8], a: u32) -> u32 {
    (rw(rom, a) as u32) << 16 | rw(rom, a + 2) as u32
}

/// One box as the frame record gives it: the vertical centre is `-dy` from
/// the grid centre, the horizontal centre `dx`, and the half sizes are
/// `hh` and `hw`. The judge compares inclusively, so the box covers
/// `centre - half ..= centre + half` in each direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct BoxSpec {
    pub dy: i8,
    pub hh: u8,
    pub dx: i8,
    pub hw: u8,
}

impl BoxSpec {
    fn read(rom: &[u8], a: u32) -> Self {
        BoxSpec {
            dy: rb(rom, a) as i8,
            hh: rb(rom, a + 1),
            dx: rb(rom, a + 2) as i8,
            hw: rb(rom, a + 3),
        }
    }

    /// True when the box has size zero: a single point, which the judge still tests.
    pub fn is_empty(&self) -> bool {
        self.hh == 0 && self.hw == 0
    }

    /// The box in grid pixels for a grid `w` by `h`, as an inclusive
    /// rectangle `(x0, y0, x1, y1)`, origin at the grid's top-left.
    pub fn rect(&self, w: usize, h: usize) -> (i32, i32, i32, i32) {
        let cx = (w / 2) as i32 + self.dx as i32;
        let cy = (h / 2) as i32 - self.dy as i32;
        (cx - self.hw as i32, cy - self.hh as i32, cx + self.hw as i32, cy + self.hh as i32)
    }
}

/// One frame of a move: a stream's 0x20-byte frame record or a move table's
/// 0x30-byte header (which is the move's first frame).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameRecord {
    /// The record's address in the program ROM.
    pub addr: u32,
    /// True for a move table header.
    pub header: bool,
    pub repeat: u8,
    pub duration: u8,
    pub code: u16,
    pub attr: u16,
    /// The per-tick step the frame carries (`animate`'s repeat step).
    pub step_dy: i8,
    pub step_dx: i8,
    /// Rows and columns minus one, as stored.
    pub rows: u8,
    pub cols: u8,
    /// The pushbox halves (`+0x16..+0x19`): top, bottom, left, right.
    pub push: [u8; 4],
    pub tight: BoxSpec,
    pub loose: BoxSpec,
    pub attack: BoxSpec,
    pub flags: u8,
    pub x_offset: i8,
    pub hit_fn: u32,
}

impl FrameRecord {
    /// A stream's frame record at `a` (`load_frame`'s layout), or None when
    /// it cannot be a frame (a grid past the fighter's 24 entries).
    pub fn from_stream(rom: &[u8], a: u32) -> Option<Self> {
        let rec = FrameRecord {
            addr: a,
            header: false,
            repeat: rb(rom, a),
            duration: rb(rom, a + 1),
            code: rw(rom, a + 2),
            attr: rw(rom, a + 4),
            step_dy: rb(rom, a + 6) as i8,
            step_dx: rb(rom, a + 7) as i8,
            rows: rb(rom, a + 8),
            cols: rb(rom, a + 9),
            push: [rb(rom, a + 0xa), rb(rom, a + 0xb), rb(rom, a + 0xc), rb(rom, a + 0xd)],
            tight: BoxSpec::read(rom, a + 0xe),
            loose: BoxSpec::read(rom, a + 0x12),
            attack: BoxSpec::read(rom, a + 0x16),
            flags: rb(rom, a + 0x1a),
            x_offset: rb(rom, a + 0x1b) as i8,
            hit_fn: rl(rom, a + 0x1c),
        };
        rec.sane().then_some(rec)
    }

    /// A move table header at `h` (the entity's first 0x30 bytes).
    pub fn from_header(rom: &[u8], h: u32) -> Option<Self> {
        let rec = FrameRecord {
            addr: h,
            header: true,
            repeat: rb(rom, h + 0xb),
            duration: rb(rom, h + 0xe),
            code: rw(rom, h),
            attr: rw(rom, h + 2),
            step_dy: rb(rom, h + 0x14) as i8,
            step_dx: rb(rom, h + 0x15) as i8,
            rows: rb(rom, h + 0xc),
            cols: rb(rom, h + 0xd),
            push: [rb(rom, h + 0x16), rb(rom, h + 0x17), rb(rom, h + 0x18), rb(rom, h + 0x19)],
            tight: BoxSpec::read(rom, h + 0x1a),
            loose: BoxSpec::read(rom, h + 0x1e),
            attack: BoxSpec::read(rom, h + 0x22),
            flags: rb(rom, h + 0xf),
            x_offset: rb(rom, h + 0x26) as i8,
            hit_fn: rl(rom, h + 0x28),
        };
        rec.sane().then_some(rec)
    }

    fn sane(&self) -> bool {
        (self.rows as usize + 1) * (self.cols as usize + 1) <= MAX_ENTRIES
    }

    /// Big sprites (attr >= 0x400, signed) are 32 pixel cells.
    pub fn big(&self) -> bool {
        (self.attr as i16) >= 0x400
    }

    pub fn width(&self) -> usize {
        (self.cols as usize + 1) * if self.big() { 32 } else { 16 }
    }

    pub fn height(&self) -> usize {
        (self.rows as usize + 1) * if self.big() { 32 } else { 16 }
    }

    /// The ticks the frame is on screen: `animate` shows it once, then
    /// `repeat` more times, each time for `duration` ticks after a tick of
    /// its own.
    pub fn ticks(&self) -> u32 {
        (self.repeat as u32 + 1) * (self.duration as u32 + 1)
    }

    /// The attack box's forward edge in pixels from the fighter's stance
    /// centre: `x_offset + dx + hw` (see the module docs), without the
    /// travel of earlier frames' steps.
    pub fn reach(&self) -> i32 {
        self.x_offset as i32 + self.attack.dx as i32 + self.attack.hw as i32
    }

    /// The attack box is live on this frame (flags bit 7).
    pub fn attack_active(&self) -> bool {
        self.flags & 0x80 != 0
    }

    /// The fighter is invulnerable on this frame (flags bit 3).
    pub fn invulnerable(&self) -> bool {
        self.flags & 0x08 != 0
    }
}

/// A pose drawn as pens, facing right, origin at the grid's top-left.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pose {
    pub w: usize,
    pub h: usize,
    /// Row by row; [`TRANSPARENT`] where nothing is drawn.
    pub pens: Vec<u8>,
}

impl Pose {
    fn blank(w: usize, h: usize) -> Self {
        Pose { w, h, pens: vec![TRANSPARENT; w * h] }
    }

    /// True when a pixel is drawn at `(x, y)`.
    pub fn drawn(&self, x: i32, y: i32) -> bool {
        x >= 0
            && y >= 0
            && (x as usize) < self.w
            && (y as usize) < self.h
            && self.pens[y as usize * self.w + x as usize] != TRANSPARENT
    }

    /// How many pixels are drawn.
    pub fn drawn_count(&self) -> u32 {
        self.pens.iter().filter(|&&p| p != TRANSPARENT).count() as u32
    }

    /// Draw sprite entries (`[code, attr, y, x]`, as `pose_grid_entries`
    /// lists them) as the renderer does: pen 15 transparent, big entries a
    /// 2x2 block of tiles, flips per tile.
    pub fn from_entries(gfx3: &[u8], entries: &[[u16; 4]], w: usize, h: usize) -> Pose {
        let mut pose = Pose::blank(w, h);
        let count = gfx3.len() / 128;
        for &[code, attr, y, x] in entries {
            let (flipx, flipy) = (attr & 0x100 != 0, attr & 0x200 != 0);
            let mut tiles: Vec<(u16, i32, i32)> = Vec::new();
            if attr & 0x400 != 0 {
                let mut quad = [code, code.wrapping_add(1), code.wrapping_add(16), code.wrapping_add(17)];
                if flipx {
                    quad.swap(0, 1);
                    quad.swap(2, 3);
                }
                if flipy {
                    quad.swap(0, 2);
                    quad.swap(1, 3);
                }
                for (i, &c) in quad.iter().enumerate() {
                    tiles.push((c, x as i32 + (i as i32 % 2) * 16, y as i32 + (i as i32 / 2) * 16));
                }
            } else {
                tiles.push((code, x as i32, y as i32));
            }
            for (c, tx, ty) in tiles {
                let tile = decode_sprite_tile(gfx3, sprite_slot(c) % count);
                for py in 0..16usize {
                    for px in 0..16usize {
                        let (sx, sy) = (if flipx { 15 - px } else { px }, if flipy { 15 - py } else { py });
                        let pen = tile[sy][sx];
                        if pen == TRANSPARENT {
                            continue;
                        }
                        let (dx, dy) = (tx + px as i32, ty + py as i32);
                        if dx >= 0 && dy >= 0 && (dx as usize) < w && (dy as usize) < h {
                            pose.pens[dy as usize * w + dx as usize] = pen;
                        }
                    }
                }
            }
        }
        pose
    }

    /// The pose a frame record draws, facing right.
    pub fn of_frame(gfx3: &[u8], rec: &FrameRecord) -> Pose {
        let entries = pose_grid_entries(rec.code, rec.attr, rec.rows, rec.cols, 0, 0, false);
        Pose::from_entries(gfx3, &entries, rec.width(), rec.height())
    }
}

/// Where a stream's primary path ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathEnd {
    /// An end entry (0xfd).
    End,
    /// The CPU follow-up call (0x45630): the 1987 CPU decides the next move.
    FollowUp,
    /// A goto back to a frame already on the path (a cycle).
    Loop,
    /// The step cap (a safety stop; no stream reaches it).
    Cap,
    /// A frame that cannot be one (a grid past 24 entries).
    Broken,
}

/// The effect templates a spawn routine in a stream creates.
fn spawn_templates(rom: &[u8], routine: u32) -> Vec<u32> {
    match routine {
        0x4548c => vec![0x186dc],
        0x454ae => vec![0x1871c],
        0x45ace => vec![0x1873c],
        0x4596e => vec![rl(rom, 0x45a24), rl(rom, 0x45a28), rl(rom, 0x45a2c)],
        _ => Vec::new(),
    }
}

/// One walk of a stream: every frame record reachable from its start (any
/// branch), and the primary path through them, which is the order a move
/// plays in when nothing interrupts it.
#[derive(Clone, Debug)]
pub struct StreamWalk {
    /// The frames, header first when the move has one, then in the order the
    /// walk found them.
    pub frames: Vec<FrameRecord>,
    /// The primary path: indices into `frames`, one per time a frame plays
    /// (a loop's frames repeat).
    pub path: Vec<usize>,
    pub end: PathEnd,
    /// The routines the stream calls, with the path position (ticks into the
    /// move, from tick 1) at which the primary path first reaches each.
    pub calls: Vec<(u32, u32)>,
    /// Routines called that the walker does not know (all but the ones the
    /// stream walk names).
    pub unknown_calls: BTreeSet<u32>,
}

/// Walk the stream at `start`, after `header` (the move table's first
/// frame) when there is one.
pub fn walk_stream(rom: &[u8], header: Option<FrameRecord>, start: u32) -> StreamWalk {
    let mut frames: Vec<FrameRecord> = Vec::new();
    let mut index: BTreeMap<u32, usize> = BTreeMap::new();
    let mut walk = StreamWalk {
        frames: Vec::new(),
        path: Vec::new(),
        end: PathEnd::End,
        calls: Vec::new(),
        unknown_calls: BTreeSet::new(),
    };
    let mut path: Vec<usize> = Vec::new();
    if let Some(h) = header {
        frames.push(h);
        path.push(0);
    }
    let mut tick: u32 = 1 + path.iter().map(|&i| frames[i].ticks()).sum::<u32>();
    let known = |r: u32| {
        matches!(
            r,
            0x45630 | 0x45a34 | 0x4550a | 0x4554a | 0x45526 | 0x45566 | 0x455fc | 0x452e0 | 0x453b6
                | 0x454c6 | 0x454ee | 0x45a7a | 0x45ab2 | 0x45a96 | 0x4548c | 0x454ae | 0x45ace
                | 0x455de | 0x4558a | 0x4594c | 0x4596e
        )
    };

    // The primary path.
    let mut a = start;
    let mut visited: BTreeSet<u32> = BTreeSet::new();
    let (mut loop_count, mut loop_start) = (0u8, 0u32);
    let mut steps = 0;
    walk.end = loop {
        steps += 1;
        if steps > 400 || a as usize >= rom.len() {
            break PathEnd::Cap;
        }
        match rb(rom, a) {
            0xfd => break PathEnd::End,
            0xfe => {
                let target = rl(rom, a + 2);
                if visited.contains(&target) {
                    break PathEnd::Loop;
                }
                a = target;
            }
            0xff => {
                let routine = rl(rom, a + 2);
                walk.calls.push((tick, routine));
                if !known(routine) {
                    walk.unknown_calls.insert(routine);
                }
                match routine {
                    0x45630 => break PathEnd::FollowUp,
                    0x4550a | 0x4554a => {
                        loop_count = rb(rom, a + 1);
                        loop_start = a + 6;
                        a += 6;
                    }
                    0x45526 | 0x45566 => {
                        if loop_count != 0 {
                            loop_count -= 1;
                            a = loop_start;
                        } else {
                            a += 6;
                        }
                    }
                    _ => a += 6,
                }
            }
            _ => {
                let Some(rec) = FrameRecord::from_stream(rom, a) else {
                    break PathEnd::Broken;
                };
                let i = *index.entry(a).or_insert_with(|| {
                    frames.push(rec);
                    frames.len() - 1
                });
                tick += frames[i].ticks();
                path.push(i);
                visited.insert(a);
                a += 0x20;
            }
        }
    };

    // Every other reachable frame (the other side of a choice, the loop
    // body, the follow-up's resume).
    let mut work: Vec<u32> = vec![start];
    let mut seen: BTreeSet<u32> = BTreeSet::new();
    while let Some(mut a) = work.pop() {
        loop {
            if a as usize >= rom.len() || !seen.insert(a) {
                break;
            }
            match rb(rom, a) {
                0xfd => break,
                0xfe => a = rl(rom, a + 2),
                0xff => {
                    let routine = rl(rom, a + 2);
                    match routine {
                        0x45630 => work.push(a + 8),
                        0x45a34 => {
                            work.push(a + 0x20 + 12);
                            work.push(a + 0x40 + 18);
                        }
                        _ => {}
                    }
                    a += 6;
                }
                _ => {
                    let Some(rec) = FrameRecord::from_stream(rom, a) else {
                        break;
                    };
                    if let std::collections::btree_map::Entry::Vacant(e) = index.entry(a) {
                        e.insert(frames.len());
                        frames.push(rec);
                    }
                    a += 0x20;
                }
            }
        }
    }
    walk.frames = frames;
    walk.path = path;
    walk
}

/// What kind of stream a move is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MoveKind {
    /// A move table slot (stance, walk, crouch, jump, block, the normals and
    /// specials of Ryu and Ken).
    Table,
    /// A jump attack Ryu or Ken starts at the apex of a jump.
    JumpAttack,
    /// A stream a boss's 1987 CPU can start from its attack records.
    CpuAttack,
    /// A stream a boss's 1987 CPU starts from its reaction records (guards,
    /// jumps and the like; any with an active frame count as attacks too).
    CpuReaction,
    /// The attacker's recoil when a kick is blocked (Ryu, Ken).
    Recoil,
    /// A hit reaction stream.
    HitReaction,
    /// A knock-out stream.
    KnockOut,
}

impl MoveKind {
    pub fn label(self) -> &'static str {
        match self {
            MoveKind::Table => "table move",
            MoveKind::JumpAttack => "jump attack",
            MoveKind::CpuAttack => "CPU attack",
            MoveKind::CpuReaction => "CPU reaction",
            MoveKind::Recoil => "blocked recoil",
            MoveKind::HitReaction => "hit reaction",
            MoveKind::KnockOut => "knock-out",
        }
    }
}

/// One move to audit: where its frames come from.
#[derive(Clone, Debug)]
pub struct MoveEntry {
    pub fighter: TournamentFighter,
    pub kind: MoveKind,
    pub name: String,
    /// The move table slot, for a table move.
    pub slot: Option<u8>,
    /// The move table header's address, for a table move.
    pub header: Option<u32>,
    /// The stream's start (0 when the move is its header alone).
    pub stream: u32,
    /// Extra facts: the CPU's records that start the stream (its range class
    /// and the move id it sets), the header's state byte.
    pub notes: Vec<String>,
}

fn slot_name(fighter: TournamentFighter, slot: u8) -> String {
    let base = match slot {
        0 => "intro walk-in",
        1 => "stance",
        2 => "walk forward",
        3 => "walk back",
        4 => "standing block",
        5 => "crouch",
        6 => "crouch forward",
        7 => "crouch back",
        8 => "crouching block",
        9 => "jump",
        0xa => "jump forward",
        0xb => "jump back",
        0xc => "win pose",
        _ => "",
    };
    if fighter.is_shoto() {
        let n = match slot {
            0xd => "standing light punch",
            0xe => "light punch (variant input cannot reach)",
            0xf => "standing medium punch",
            0x10 => "standing heavy punch",
            0x11 => "crouching punch (light)",
            0x12 => "crouching punch (medium)",
            0x13 => "crouching punch (heavy)",
            0x14 => "standing light kick",
            0x15 => "standing medium kick",
            0x16 => "standing heavy kick",
            0x17 => "crouching light/medium kick",
            0x18 => "sweep",
            0x19 => "Shoryuken",
            0x1a => "Hadoken",
            0x1b => "Tatsumaki Senpukyaku",
            0x1c => "crouching punch (medium, alt)",
            0x1d => "crouching punch (heavy, alt)",
            0x1e => "crouching punch (alt 2)",
            _ => base,
        };
        if n.is_empty() {
            return format!("move 0x{slot:02x}");
        }
        return n.to_string();
    }
    if fighter == TournamentFighter::Geki && matches!(slot, 4 | 8) {
        return "teleport".to_string();
    }
    if base.is_empty() {
        format!("move 0x{slot:02x}")
    } else {
        base.to_string()
    }
}

/// Every move of `fighter` the audit covers, from the ROM's own tables.
pub fn fighter_moves(rom: &[u8], fighter: TournamentFighter) -> Vec<MoveEntry> {
    let table = fighter.move_table();
    let slots = if fighter.is_shoto() { 0x1f } else { 0x0d };
    let mut out: Vec<MoveEntry> = Vec::new();
    let mut by_stream: BTreeMap<u32, usize> = BTreeMap::new();

    // 1. The move table.
    for slot in 0..slots as u8 {
        let h = table + slot as u32 * 0x30;
        if (0..0x30).all(|i| rb(rom, h + i) == 0) {
            continue;
        }
        let stream = rl(rom, h + 0x2c);
        let animated = rb(rom, h + 9) & 0x80 != 0;
        let stream = if animated && stream != 0 { stream } else { 0 };
        out.push(MoveEntry {
            fighter,
            kind: MoveKind::Table,
            name: slot_name(fighter, slot),
            slot: Some(slot),
            header: Some(h),
            stream,
            notes: vec![format!("state 0x{:02x}", rb(rom, h + 9))],
        });
        if stream != 0 {
            by_stream.entry(stream).or_insert(out.len() - 1);
        }
    }

    let add = |out: &mut Vec<MoveEntry>,
                   by_stream: &mut BTreeMap<u32, usize>,
                   kind: MoveKind,
                   name: String,
                   stream: u32,
                   note: Option<String>| {
        if stream == 0 || stream >= rom.len() as u32 {
            return;
        }
        if let Some(&i) = by_stream.get(&stream) {
            if let Some(n) = note {
                if !out[i].notes.contains(&n) {
                    out[i].notes.push(n);
                }
            }
            return;
        }
        by_stream.insert(stream, out.len());
        out.push(MoveEntry {
            fighter,
            kind,
            name,
            slot: None,
            header: None,
            stream,
            notes: note.into_iter().collect(),
        });
    };

    if fighter.is_shoto() {
        // 2. Jump attacks, from the apex tables.
        let (punch, kick) = if fighter == TournamentFighter::Ryu { (0x45386, 0x4539e) } else { (0x4545c, 0x45474) };
        let names = [
            ["jump punch (neutral)", "jump punch (forward)", "jump punch (back)"],
            ["jump kick (neutral)", "jump kick (forward)", "jump kick (back)"],
        ];
        for (t, tab) in [punch, kick].into_iter().enumerate() {
            for k in 0..3u32 {
                let entry = tab + k * 8;
                let streams = rl(rom, entry + 4);
                for j in 0..4u32 {
                    let s = rw(rom, streams + 2 * j) as u32;
                    let label = if t == 1 && k == 0 && j >= 2 { "jump heavy kick (neutral)" } else { names[t][k as usize] };
                    add(&mut out, &mut by_stream, MoveKind::JumpAttack, label.to_string(), s, None);
                }
            }
        }
        // 3. Blocked-kick recoils (hit functions' `Redirect` streams).
        let recoils: &[u32] = if fighter == TournamentFighter::Ryu {
            &[0xf4a, 0x10b4, 0x1094]
        } else {
            &[0x3028, 0x3192, 0x3172]
        };
        for &s in recoils {
            add(&mut out, &mut by_stream, MoveKind::Recoil, "blocked kick recoil".to_string(), s, None);
        }
        // 4. Hit and knock-out reactions.
        let (alive, ko) = if fighter == TournamentFighter::Ryu { (0x44662, 0x446ca) } else { (0x4473a, 0x4479c) };
        for n in 0..13u32 {
            let s = rw(rom, alive + 4 * n) as i16 as i32 as u32;
            add(&mut out, &mut by_stream, MoveKind::HitReaction, format!("hit reaction 0x{:02x}", 0x30 + n), s, None);
        }
        for n in 0..11u32 {
            let s = rw(rom, ko + 4 * n) as i16 as i32 as u32;
            add(&mut out, &mut by_stream, MoveKind::KnockOut, format!("knock-out 0x{:02x}", 0x40 + n), s, None);
        }
    } else {
        // The opponent index of this boss in the per-country arrays.
        let arr = |n: u32, i: u32| rl(rom, 0x19908 + 0xa0 * n + 4 * i);
        let idx = (0..10u32).find(|&i| arr(0, i) == table).expect("a boss is on Japan's ladder");
        // 5. Attack records: three tables (range classes) x 8 images of 48 records.
        let mut ai: BTreeMap<u32, BTreeSet<String>> = BTreeMap::new();
        for (n, label) in [(10u32, "near"), (11, "middle"), (12, "far")] {
            let p = arr(n, idx);
            for img in 0..8u32 {
                for r in 0..48u32 {
                    let a = p + img * 0x180 + r * 8;
                    let s = rl(rom, a);
                    if s == 0 || s >= rom.len() as u32 {
                        continue;
                    }
                    ai.entry(s).or_default().insert(format!("{label} 0x{:02x}", rb(rom, a + 5)));
                }
            }
        }
        for (s, notes) in &ai {
            for note in notes {
                add(&mut out, &mut by_stream, MoveKind::CpuAttack, "attack".to_string(), *s, Some(note.clone()));
            }
        }
        // 6. Reaction records: 2 tables x 8 images of 16 records.
        let mut react: BTreeMap<u32, BTreeSet<String>> = BTreeMap::new();
        for (n, label) in [(8u32, "vs airborne"), (9, "vs ground")] {
            let p = arr(n, idx);
            for img in 0..8u32 {
                for r in 0..16u32 {
                    let a = p + img * 0x80 + r * 8;
                    let s = rl(rom, a);
                    if s == 0 || s >= rom.len() as u32 {
                        continue;
                    }
                    react.entry(s).or_default().insert(format!("{label} 0x{:02x}", rb(rom, a + 5)));
                }
            }
        }
        for (s, notes) in &react {
            for note in notes {
                add(
                    &mut out,
                    &mut by_stream,
                    MoveKind::CpuReaction,
                    "reaction".to_string(),
                    *s,
                    Some(note.clone()),
                );
            }
        }
        // 7. The bosses' own hit and knock-out tables.
        let (alive, ko) = fighter.reaction_tables().expect("a boss has CPU reaction tables");
        for n in 0..13u32 {
            let s = rw(rom, alive + 4 * n) as u32;
            add(&mut out, &mut by_stream, MoveKind::HitReaction, format!("hit reaction 0x{:02x}", 0x30 + n), s, None);
        }
        for n in 0..11u32 {
            let s = rw(rom, ko + 4 * n) as u32;
            add(&mut out, &mut by_stream, MoveKind::KnockOut, format!("knock-out 0x{:02x}", 0x40 + n), s, None);
        }
    }
    out
}

/// How far an attack box reaches past the drawn pixels, in each direction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttackMetrics {
    /// The box, inclusive grid pixels `(x0, y0, x1, y1)`.
    pub rect: (i32, i32, i32, i32),
    /// `dx + hw`: pixels from the centre to the box's forward edge.
    pub edge: i32,
    /// `hw - dx`: pixels from the centre to the box's back edge.
    pub back: i32,
    /// Overreach right (forward), left, up and down: the box's edge minus the
    /// farthest drawn pixel in the box's band; None when the band has no drawn
    /// pixel.
    pub overreach: [Option<i32>; 4],
    /// Drawn pixels inside the box.
    pub inside: u32,
    /// The box's area in pixels.
    pub area: u32,
}

pub const DIRS: [&str; 4] = ["forward", "back", "up", "down"];

impl AttackMetrics {
    /// The largest overreach and its direction.
    pub fn worst(&self) -> Option<(i32, usize)> {
        self.overreach.iter().enumerate().filter_map(|(d, o)| o.map(|v| (v, d))).max()
    }

    /// No drawn pixel inside the box at all.
    pub fn floating(&self) -> bool {
        self.inside == 0
    }
}

/// The measure of one attack box on one pose.
pub fn measure_attack(pose: &Pose, spec: &BoxSpec) -> AttackMetrics {
    let (x0, y0, x1, y1) = spec.rect(pose.w, pose.h);
    let mut inside = 0u32;
    let (mut min_x, mut max_x) = (i32::MAX, i32::MIN);
    let (mut min_y, mut max_y) = (i32::MAX, i32::MIN);
    // Rows the box spans: the extent of drawn pixels in them.
    for y in y0.max(0)..=y1.min(pose.h as i32 - 1) {
        for x in 0..pose.w as i32 {
            if pose.drawn(x, y) {
                min_x = min_x.min(x);
                max_x = max_x.max(x);
            }
        }
    }
    // Columns the box spans.
    for x in x0.max(0)..=x1.min(pose.w as i32 - 1) {
        for y in 0..pose.h as i32 {
            if pose.drawn(x, y) {
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
    }
    for y in y0.max(0)..=y1.min(pose.h as i32 - 1) {
        for x in x0.max(0)..=x1.min(pose.w as i32 - 1) {
            if pose.drawn(x, y) {
                inside += 1;
            }
        }
    }
    // Each measured only when something is drawn there: the subtraction
    // against the sentinel would overflow (a debug build panics on it).
    AttackMetrics {
        rect: (x0, y0, x1, y1),
        edge: spec.dx as i32 + spec.hw as i32,
        back: spec.hw as i32 - spec.dx as i32,
        overreach: [
            (max_x != i32::MIN).then(|| x1 - max_x),
            (min_x != i32::MAX).then(|| min_x - x0),
            (min_y != i32::MAX).then(|| min_y - y0),
            (max_y != i32::MIN).then(|| y1 - max_y),
        ],
        inside,
        area: ((x1 - x0 + 1) * (y1 - y0 + 1)) as u32,
    }
}

/// How well the loose hurtbox covers the drawn body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BodyMetrics {
    pub rect: (i32, i32, i32, i32),
    pub tight: (i32, i32, i32, i32),
    pub drawn: u32,
    /// Drawn pixels outside the loose box.
    pub uncovered: u32,
    /// How far drawn pixels stick out of the loose box: right, left, up, down
    /// (0 when none do).
    pub extent: [i32; 4],
    /// The bounding box of the uncovered pixels, when there are any.
    pub uncovered_box: Option<(i32, i32, i32, i32)>,
    /// How far the loose box lies past the drawn pixels of its own band
    /// (right, left, up, down), as for an attack box.
    pub overreach: [Option<i32>; 4],
}

impl BodyMetrics {
    pub fn uncovered_share(&self) -> f64 {
        if self.drawn == 0 {
            0.0
        } else {
            self.uncovered as f64 / self.drawn as f64
        }
    }
}

pub fn measure_body(pose: &Pose, loose: &BoxSpec, tight: &BoxSpec) -> BodyMetrics {
    let (x0, y0, x1, y1) = loose.rect(pose.w, pose.h);
    let mut drawn = 0u32;
    let mut uncovered = 0u32;
    let mut extent = [0i32; 4];
    let mut ub: Option<(i32, i32, i32, i32)> = None;
    for y in 0..pose.h as i32 {
        for x in 0..pose.w as i32 {
            if !pose.drawn(x, y) {
                continue;
            }
            drawn += 1;
            if x < x0 || x > x1 || y < y0 || y > y1 {
                uncovered += 1;
                extent[0] = extent[0].max(x - x1);
                extent[1] = extent[1].max(x0 - x);
                extent[2] = extent[2].max(y0 - y);
                extent[3] = extent[3].max(y - y1);
                ub = Some(match ub {
                    None => (x, y, x, y),
                    Some((a, b, c, d)) => (a.min(x), b.min(y), c.max(x), d.max(y)),
                });
            }
        }
    }
    let m = measure_attack(pose, loose);
    BodyMetrics {
        rect: (x0, y0, x1, y1),
        tight: tight.rect(pose.w, pose.h),
        drawn,
        uncovered,
        extent,
        uncovered_box: ub,
        overreach: m.overreach,
    }
}

/// A flagged finding about a move.
#[derive(Clone, Debug, PartialEq)]
pub struct Finding {
    pub kind: FindingKind,
    /// Index into the move's frames of the frame to picture.
    pub frame: usize,
    /// The measure that crossed its threshold: pixels, ticks or percent
    /// points, by kind.
    pub value: i32,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FindingKind {
    /// An active attack box with no drawn pixel inside it.
    AttackFloating,
    /// An attack box reaching [`OVERREACH_FLAG`] past the drawn pixels.
    AttackOverreach,
    /// A loose hurtbox reaching [`BODY_OVERREACH_FLAG`] past the drawn pixels.
    BodyOverreach,
    /// A drawing that outgrows its hurtbox by [`BODY_EXCESS_FLAG`] more than
    /// the fighter's own stance does.
    BodyUncovered,
    /// A tight (critical) box sticking [`TIGHT_OUTSIDE_FLAG`] out of the
    /// loose box that every hit must touch first.
    TightOutside,
    /// An attack move with [`POINT_HURTBOX_FLAG`] ticks of a zero-size
    /// (point) loose hurtbox.
    PointHurtbox,
    /// A move whose frames carry an attack box and a hit function but none
    /// of which ever sets the active bit (flags bit 7): it can never hit.
    Dormant,
    /// A start faster than anything in the research.
    FastStartup,
    /// A reach of [`LONG_REACH`] or more.
    LongReach,
}

impl FindingKind {
    pub const ALL: [FindingKind; 9] = [
        FindingKind::Dormant,
        FindingKind::AttackFloating,
        FindingKind::AttackOverreach,
        FindingKind::BodyOverreach,
        FindingKind::BodyUncovered,
        FindingKind::TightOutside,
        FindingKind::PointHurtbox,
        FindingKind::FastStartup,
        FindingKind::LongReach,
    ];

    pub fn label(self) -> &'static str {
        match self {
            FindingKind::AttackFloating => "attack box over nothing",
            FindingKind::AttackOverreach => "attack box past the drawing",
            FindingKind::BodyOverreach => "hurtbox past the drawing",
            FindingKind::BodyUncovered => "drawing outgrows the hurtbox",
            FindingKind::TightOutside => "critical box outside the hurtbox",
            FindingKind::PointHurtbox => "point hurtbox mid-move",
            FindingKind::Dormant => "attack box that never goes live",
            FindingKind::FastStartup => "faster than the research",
            FindingKind::LongReach => "long reach",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            FindingKind::AttackFloating => "floating",
            FindingKind::AttackOverreach => "overreach",
            FindingKind::BodyOverreach => "bodypast",
            FindingKind::BodyUncovered => "uncovered",
            FindingKind::TightOutside => "tight",
            FindingKind::PointHurtbox => "point",
            FindingKind::Dormant => "dormant",
            FindingKind::FastStartup => "fast",
            FindingKind::LongReach => "reach",
        }
    }
}

/// One audited frame.
#[derive(Clone, Debug)]
pub struct FrameAudit {
    pub rec: FrameRecord,
    pub drawn: u32,
    pub attack: Option<AttackMetrics>,
    pub body: BodyMetrics,
    /// How far the tight box sticks out of the loose box, in pixels (0 when
    /// it lies inside, or has no size).
    pub tight_out: i32,
    /// The loose hurtbox has size zero: a point at its offset from the
    /// centre, not an absent box.
    pub point_hurtbox: bool,
}

/// A projectile a move spawns: its sprite and box per effect frame.
#[derive(Clone, Debug)]
pub struct ProjectileAudit {
    pub template: u32,
    /// Pixels per tick of the first effect frame (signed, facing right).
    pub speed: i32,
    /// Per effect frame: (box half height, half width, attack metrics).
    pub frames: Vec<(u8, u8, AttackMetrics)>,
    pub big: bool,
}

/// One audited move.
#[derive(Clone, Debug)]
pub struct MoveAudit {
    pub entry: MoveEntry,
    pub walk: StreamWalk,
    pub frames: Vec<FrameAudit>,
    /// The tick (1 = the tick the move starts) of the first frame with an
    /// active attack box on the primary path.
    pub startup: Option<u32>,
    /// The same by strength level 1, 2, 3, for a stream that holds its first
    /// frames longer for the stronger buttons (the call at `0x455fc`:
    /// table `0x45620`, extra ticks 0, 6, 9).
    pub startup_by_strength: Option<[u32; 3]>,
    /// Ticks the active frames last on the primary path.
    pub active_ticks: u32,
    /// Ticks the primary path lasts.
    pub total_ticks: u32,
    /// The tick a projectile is spawned at, when the move throws one.
    pub spawn: Option<u32>,
    pub projectiles: Vec<ProjectileAudit>,
    /// The largest [`FrameRecord::reach`] over active frames: pixels from the
    /// stance centre to the attack box's forward edge, not counting steps.
    pub reach: Option<i32>,
    /// How far the frame steps carry the fighter's centre over the whole
    /// primary path (px, forward positive): lunges, jumps, turns. `None`
    /// when the move does not step.
    pub travel: Option<i32>,
    /// The largest attack overreach over active frames: pixels, direction
    /// index into [`DIRS`], frame.
    pub worst_overreach: Option<(i32, usize, usize)>,
    /// The largest share of drawn pixels outside the loose box, minus the
    /// fighter's stance's share, over the frames that have a hurtbox; with
    /// its frame.
    pub uncovered_excess: Option<(f64, usize)>,
    /// The largest loose-box overreach (pixels) over the frames that have a
    /// hurtbox, with its frame.
    pub loose_overreach: Option<(i32, usize)>,
    /// The largest tight-box overhang (pixels) with its frame.
    pub tight_out: Option<(i32, usize)>,
    /// Ticks on the primary path with a zero-size loose hurtbox, and the first
    /// such frame.
    pub point_hurtbox_ticks: u32,
    pub first_point_hurtbox: Option<usize>,
    /// Ticks on the primary path with the invulnerable flag (bit 3).
    pub invulnerable_ticks: u32,
    pub findings: Vec<Finding>,
}

impl MoveAudit {
    pub fn has_attack(&self) -> bool {
        self.startup.is_some() || self.spawn.is_some()
    }

    /// The primary path as `(frame index, first tick)` pairs: tick 1 is the
    /// tick the move starts.
    pub fn timeline(&self) -> Vec<(usize, u32)> {
        let mut tick = 1;
        self.walk
            .path
            .iter()
            .map(|&i| {
                let at = tick;
                tick += self.walk.frames[i].ticks();
                (i, at)
            })
            .collect()
    }

    /// The frame on screen at `tick` along the primary path, if the path
    /// still runs then.
    pub fn frame_at(&self, tick: u32) -> Option<usize> {
        self.timeline()
            .into_iter()
            .rev()
            .find(|&(i, at)| at <= tick && tick < at + self.walk.frames[i].ticks())
            .map(|(i, _)| i)
    }

    /// How far the frame steps have carried the fighter's centre by the end
    /// of `tick` on the primary path (px, forward positive). A frame steps
    /// (`+0x15`) once when it loads (not a header: the move starts there) and
    /// once more each time its repeat count runs it again, every
    /// `duration + 1` ticks.
    pub fn travel_at(&self, tick: u32) -> i32 {
        let mut sum = 0i32;
        for (i, at) in self.timeline() {
            let rec = &self.walk.frames[i];
            if at > tick {
                break;
            }
            let offset = (tick - at).min(rec.ticks() - 1);
            let steps = offset / (rec.duration as u32 + 1) + u32::from(!rec.header);
            sum += steps as i32 * rec.step_dx as i32;
        }
        sum
    }

    /// The first frame on the primary path whose attack box is live.
    pub fn first_active(&self) -> Option<usize> {
        self.walk.path.iter().copied().find(|&i| self.walk.frames[i].attack_active())
    }
}

/// The audit of one fighter.
#[derive(Clone, Debug)]
pub struct FighterAudit {
    pub fighter: TournamentFighter,
    /// The share of the stance's drawn pixels outside its loose hurtbox: the
    /// fighter's own baseline for [`BODY_EXCESS_FLAG`].
    pub stance_share: f64,
    /// How far the stance's critical box sticks out of its loose hurtbox:
    /// the fighter's own baseline for [`TIGHT_OUTSIDE_FLAG`].
    pub stance_tight_out: i32,
    pub moves: Vec<MoveAudit>,
}

fn effect_frames(rom: &[u8], template: u32) -> Vec<(u8, u8, i32, u16)> {
    let mut out = Vec::new();
    let mut a = rl(rom, template + 0x10);
    let mut seen = BTreeSet::new();
    while seen.insert(a) && out.len() < 16 {
        match rb(rom, a) {
            0xff => a += 6,
            0xfe => a = rl(rom, a + 2),
            0xfd => break,
            _ => {
                out.push((rb(rom, a + 4), rb(rom, a + 5), rb(rom, a + 3) as i8 as i32, rw(rom, a + 6)));
                a += 8;
            }
        }
    }
    out
}

fn audit_projectile(rom: &[u8], gfx3: &[u8], template: u32) -> ProjectileAudit {
    let attr = rw(rom, template + 2);
    let big = (attr as i16) >= 0x400;
    let side = if big { 32 } else { 16 };
    let frames = effect_frames(rom, template);
    let speed = frames.first().map(|f| f.2).unwrap_or(0);
    let mut out = Vec::new();
    for (hh, hw, _, code) in frames {
        let entries = [[code, attr & !0x100, 0u16, 0u16]];
        let pose = Pose::from_entries(gfx3, &entries, side, side);
        let spec = BoxSpec { dy: 0, hh, dx: 0, hw };
        out.push((hh, hw, measure_attack(&pose, &spec)));
    }
    ProjectileAudit { template, speed, frames: out, big }
}

/// Audit one move against its fighter's stance baselines (`stance_share`,
/// `stance_tight_out`).
pub fn audit_move(rom: &[u8], gfx3: &[u8], entry: &MoveEntry, stance_share: f64, stance_tight_out: i32) -> MoveAudit {
    let header = entry.header.and_then(|h| FrameRecord::from_header(rom, h));
    let walk = if entry.stream != 0 {
        walk_stream(rom, header.clone(), entry.stream)
    } else {
        StreamWalk {
            frames: header.clone().into_iter().collect(),
            path: if header.is_some() { vec![0] } else { Vec::new() },
            end: PathEnd::End,
            calls: Vec::new(),
            unknown_calls: BTreeSet::new(),
        }
    };
    let mut frames = Vec::new();
    for rec in &walk.frames {
        let pose = Pose::of_frame(gfx3, rec);
        let attack = rec.attack_active().then(|| measure_attack(&pose, &rec.attack));
        let body = measure_body(&pose, &rec.loose, &rec.tight);
        let (l, t) = (body.rect, body.tight);
        let tight_out = if rec.tight.is_empty() || rec.loose.is_empty() {
            0
        } else {
            (l.0 - t.0).max(t.2 - l.2).max(l.1 - t.1).max(t.3 - l.3).max(0)
        };
        frames.push(FrameAudit {
            rec: rec.clone(),
            drawn: pose.drawn_count(),
            attack,
            body,
            tight_out,
            point_hurtbox: rec.loose.is_empty(),
        });
    }
    // Timeline on the primary path.
    let (mut tick, mut startup, mut active_ticks) = (1u32, None, 0u32);
    let (mut point_hurtbox_ticks, mut first_point_hurtbox, mut invulnerable_ticks) = (0u32, None, 0u32);
    for &i in &walk.path {
        let rec = &walk.frames[i];
        if rec.attack_active() {
            startup.get_or_insert(tick);
            active_ticks += rec.ticks();
        }
        if frames[i].point_hurtbox {
            point_hurtbox_ticks += rec.ticks();
            first_point_hurtbox.get_or_insert(i);
        }
        if rec.invulnerable() {
            invulnerable_ticks += rec.ticks();
        }
        tick += rec.ticks();
    }
    let total_ticks = tick - 1;
    // A first-frame hold by strength before the first active frame.
    let startup_by_strength = startup.and_then(|s| {
        walk.calls
            .iter()
            .any(|&(t, r)| r == 0x455fc && t <= s)
            .then(|| [0u32, 1, 2].map(|k| s + rb(rom, 0x45620 + k) as u32))
    });
    // Projectiles.
    let mut spawn = None;
    let mut projectiles = Vec::new();
    for &(t, routine) in &walk.calls {
        let templates = spawn_templates(rom, routine);
        if !templates.is_empty() {
            spawn.get_or_insert(t);
            for template in templates {
                if !projectiles.iter().any(|p: &ProjectileAudit| p.template == template) {
                    projectiles.push(audit_projectile(rom, gfx3, template));
                }
            }
        }
    }
    let reach = frames.iter().filter(|f| f.attack.is_some()).map(|f| f.rec.reach()).max();
    let mut worst_overreach: Option<(i32, usize, usize)> = None;
    let mut uncovered_excess: Option<(f64, usize)> = None;
    let mut loose_overreach: Option<(i32, usize)> = None;
    let mut tight_out: Option<(i32, usize)> = None;
    for (i, f) in frames.iter().enumerate() {
        if let Some((v, d)) = f.attack.as_ref().and_then(|a| if a.floating() { None } else { a.worst() }) {
            if worst_overreach.is_none_or(|(w, _, _)| v > w) {
                worst_overreach = Some((v, d, i));
            }
        }
        if f.point_hurtbox {
            continue;
        }
        let ex = f.body.uncovered_share() - stance_share;
        if uncovered_excess.is_none_or(|(w, _)| ex > w) {
            uncovered_excess = Some((ex, i));
        }
        if let Some(v) = f.body.overreach.iter().flatten().max().copied() {
            if loose_overreach.is_none_or(|(w, _)| v > w) {
                loose_overreach = Some((v, i));
            }
        }
        if f.tight_out > 0 && tight_out.is_none_or(|(w, _)| f.tight_out > w) {
            tight_out = Some((f.tight_out, i));
        }
    }
    let mut audit = MoveAudit {
        entry: entry.clone(),
        walk,
        frames,
        startup,
        startup_by_strength,
        active_ticks,
        total_ticks,
        spawn,
        projectiles,
        reach,
        travel: None,
        worst_overreach,
        uncovered_excess,
        loose_overreach,
        tight_out,
        point_hurtbox_ticks,
        first_point_hurtbox,
        invulnerable_ticks,
        findings: Vec::new(),
    };
    let net = audit.travel_at(audit.total_ticks);
    audit.travel = (net != 0).then_some(net);
    audit.findings = findings(&audit, stance_tight_out);
    audit
}

/// Apply the thresholds to an audited move.
fn findings(m: &MoveAudit, stance_tight_out: i32) -> Vec<Finding> {
    let mut out = Vec::new();
    // Hit reactions and knock-outs are outside the player's kit: summarised
    // in the report, not flagged.
    if matches!(m.entry.kind, MoveKind::HitReaction | MoveKind::KnockOut) {
        return out;
    }
    // Attack boxes: the frame with the biggest empty box.
    let floating = m
        .frames
        .iter()
        .enumerate()
        .filter(|(_, f)| f.attack.as_ref().is_some_and(|a| a.floating()))
        .max_by_key(|(_, f)| f.attack.as_ref().map(|a| a.area).unwrap_or(0));
    if let Some((i, f)) = floating {
        let a = f.attack.as_ref().unwrap();
        out.push(Finding {
            kind: FindingKind::AttackFloating,
            frame: i,
            value: a.area as i32,
            detail: format!("an active box of {} px holds no drawn pixel", a.area),
        });
    }
    if let Some((v, d, i)) = m.worst_overreach {
        if v >= OVERREACH_FLAG {
            out.push(Finding {
                kind: FindingKind::AttackOverreach,
                frame: i,
                value: v,
                detail: format!("{v} px past the drawing, {}", DIRS[d]),
            });
        }
    }
    if let Some((v, i)) = m.loose_overreach {
        if v >= BODY_OVERREACH_FLAG {
            out.push(Finding {
                kind: FindingKind::BodyOverreach,
                frame: i,
                value: v,
                detail: {
                    let rec = &m.frames[i].rec;
                    if rec.rows == 0 && rec.cols == 0 {
                        format!(
                            "the hurtbox lies {v} px past the drawing (the pose is a single {} px sprite, {} drawn px, held {} ticks)",
                            rec.width(),
                            m.frames[i].drawn,
                            rec.ticks()
                        )
                    } else {
                        format!("the hurtbox lies {v} px past the drawing")
                    }
                },
            });
        }
    }
    if let Some((ex, i)) = m.uncovered_excess {
        if ex >= BODY_EXCESS_FLAG {
            let share = m.frames[i].body.uncovered_share();
            out.push(Finding {
                kind: FindingKind::BodyUncovered,
                frame: i,
                value: (ex * 100.0).round() as i32,
                detail: format!(
                    "{:.0}% of the drawing outside the hurtbox, {:.0} points more than the stance",
                    share * 100.0,
                    ex * 100.0
                ),
            });
        }
    }
    if let Some((v, i)) = m.tight_out {
        if v - stance_tight_out >= TIGHT_OUTSIDE_FLAG {
            out.push(Finding {
                kind: FindingKind::TightOutside,
                frame: i,
                value: v,
                detail: if stance_tight_out > 0 {
                    format!("the critical box sticks {v} px out of the hurtbox (his stance's: {stance_tight_out})")
                } else {
                    format!("the critical box sticks {v} px out of the hurtbox")
                },
            });
        }
    }
    // An attack that can never go live.
    let attack_slot = match m.entry.kind {
        MoveKind::CpuAttack | MoveKind::JumpAttack => true,
        MoveKind::Table => m.entry.slot.is_some_and(|s| s >= 0x0d),
        _ => false,
    };
    if attack_slot && !m.frames.iter().any(|f| f.rec.attack_active()) {
        if let Some((i, f)) = m.frames.iter().enumerate().find(|(_, f)| !f.rec.attack.is_empty() && f.rec.hit_fn != 0) {
            out.push(Finding {
                kind: FindingKind::Dormant,
                frame: i,
                value: m.frames.iter().filter(|f| !f.rec.attack.is_empty() && f.rec.hit_fn != 0).count() as i32,
                detail: format!(
                    "{} frame(s) carry an attack box and the hit function {:#07x}, but the active bit is never set",
                    m.frames.iter().filter(|f| !f.rec.attack.is_empty() && f.rec.hit_fn != 0).count(),
                    f.rec.hit_fn
                ),
            });
        }
    }
    if m.startup.is_some() && m.point_hurtbox_ticks >= POINT_HURTBOX_FLAG {
        out.push(Finding {
            kind: FindingKind::PointHurtbox,
            frame: m.first_point_hurtbox.unwrap_or(0),
            value: m.point_hurtbox_ticks as i32,
            detail: format!(
                "{} of {} ticks with a zero-size hurtbox (a point at the centre offset: hit only by a box that covers it)",
                m.point_hurtbox_ticks, m.total_ticks
            ),
        });
    }
    // Startup and reach.
    if let Some(s) = m.startup {
        if s <= FAST_STARTUP {
            out.push(Finding {
                kind: FindingKind::FastStartup,
                frame: m.first_active().unwrap_or(0),
                value: s as i32,
                detail: format!("startup {s}; the research's fastest normal is {FASTEST_RESEARCH_NORMAL}"),
            });
        }
    } else if let Some(s) = m.spawn {
        if s < FAST_PROJECTILE_STARTUP {
            out.push(Finding {
                kind: FindingKind::FastStartup,
                frame: 0,
                value: s as i32,
                detail: format!("projectile spawns on tick {s}; the research's range is 12 to 16"),
            });
        }
    }
    if let Some(r) = m.reach {
        if r >= LONG_REACH {
            let frame = m
                .frames
                .iter()
                .enumerate()
                .filter(|(_, f)| f.attack.is_some())
                .map(|(i, f)| (f.rec.reach(), i))
                .max()
                .map(|(_, i)| i)
                .unwrap_or(0);
            out.push(Finding {
                kind: FindingKind::LongReach,
                frame,
                value: r,
                detail: format!("reach {r} px from the stance centre"),
            });
        }
    }
    out
}

/// The research's startup tiers (section 5.2, first active frame counted),
/// for a melee move's startup.
pub fn startup_tier(s: u32) -> &'static str {
    match s {
        0..=3 => "at or below the research's floor (3)",
        4..=5 => "light (4 to 5)",
        6..=8 => "medium (6 to 8)",
        9..=12 => "heavy (9 to 12)",
        _ => "slower than a modern heavy (12+)",
    }
}

/// The share of the stance's drawn pixels outside its loose box, and how
/// far its critical box sticks out of it.
fn stance_baseline(rom: &[u8], gfx3: &[u8], fighter: TournamentFighter) -> (f64, i32) {
    let Some(rec) = FrameRecord::from_header(rom, fighter.move_table() + 0x30) else {
        return (0.0, 0);
    };
    let pose = Pose::of_frame(gfx3, &rec);
    let b = measure_body(&pose, &rec.loose, &rec.tight);
    let (l, t) = (b.rect, b.tight);
    let out = if rec.tight.is_empty() || rec.loose.is_empty() {
        0
    } else {
        (l.0 - t.0).max(t.2 - l.2).max(l.1 - t.1).max(t.3 - l.3).max(0)
    };
    (b.uncovered_share(), out)
}

/// Audit every move of one fighter.
pub fn audit_fighter(rom: &[u8], gfx3: &[u8], fighter: TournamentFighter) -> FighterAudit {
    let (stance_share, stance_tight_out) = stance_baseline(rom, gfx3, fighter);
    let moves = fighter_moves(rom, fighter)
        .iter()
        .map(|e| audit_move(rom, gfx3, e, stance_share, stance_tight_out))
        .collect();
    FighterAudit { fighter, stance_share, stance_tight_out, moves }
}

/// The first, second and third quartiles of `values` (by the nearest rank
/// below), for the report's population figures.
pub fn quartiles(values: &[f64]) -> Option<[f64; 3]> {
    if values.is_empty() {
        return None;
    }
    let mut v = values.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let at = |p: f64| v[((v.len() - 1) as f64 * p) as usize];
    Some([at(0.25), at(0.5), at(0.75)])
}

/// A picture of a frame's boxes over its pose: an indexed image, ready for a
/// PNG encoder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picture {
    pub width: usize,
    pub height: usize,
    /// Row by row palette indices.
    pub pixels: Vec<u8>,
}

/// The picture palette: background, 15 greys for the pens, the same greys
/// tinted red (inside the attack box), the tinted background, then the
/// outlines.
pub const PICTURE_PALETTE: [[u8; 3]; 38] = {
    let mut p = [[0u8; 3]; 38];
    p[0] = [0x1c, 0x1e, 0x26];
    let mut i = 0;
    while i < 15 {
        let g = 70 + (i as u8) * 12;
        p[1 + i] = [g, g, g];
        let r = if g as u16 + 70 > 255 { 255 } else { g + 70 };
        p[16 + i] = [r, g / 2, g / 2];
        i += 1;
    }
    p[31] = [0x58, 0x1c, 0x22];
    p[32] = [0x30, 0xd0, 0xe8]; // loose hurtbox: cyan
    p[33] = [0xf8, 0xd8, 0x30]; // tight hurtbox: yellow
    p[34] = [0xff, 0x40, 0x40]; // attack box: red
    p[35] = [0x60, 0xf0, 0x60]; // the grid centre: green
    p[36] = [0x48, 0x4c, 0x5c]; // the grid's edge
    p[37] = [0xff, 0x98, 0x20]; // an attack box that is never live: orange
    p
};

/// Draw `rec`'s boxes over `pose`, each grid pixel `scale` picture pixels
/// wide. The picture covers the grid and every box, plus a margin.
pub fn draw_picture(pose: &Pose, rec: &FrameRecord, scale: usize) -> Picture {
    let (w, h) = (pose.w as i32, pose.h as i32);
    let loose = rec.loose.rect(pose.w, pose.h);
    let tight = rec.tight.rect(pose.w, pose.h);
    let live = rec.attack_active();
    // A box on a frame whose active bit is clear is drawn, outline only, in
    // orange: it can never hit.
    let attack = (live || (!rec.attack.is_empty() && rec.hit_fn != 0)).then(|| rec.attack.rect(pose.w, pose.h));
    let mut bounds = (0i32, 0i32, w - 1, h - 1);
    for r in [Some(loose), Some(tight), attack].into_iter().flatten() {
        bounds = (bounds.0.min(r.0), bounds.1.min(r.1), bounds.2.max(r.2), bounds.3.max(r.3));
    }
    let margin = 4;
    let (ox, oy) = (margin - bounds.0, margin - bounds.1);
    let gw = (bounds.2 - bounds.0 + 1 + 2 * margin) as usize;
    let gh = (bounds.3 - bounds.1 + 1 + 2 * margin) as usize;
    let (pw, ph) = (gw * scale, gh * scale);
    let mut pixels = vec![0u8; pw * ph];
    for py in 0..ph {
        for px in 0..pw {
            let (gx, gy) = ((px / scale) as i32 - ox, (py / scale) as i32 - oy);
            let pen = if pose.drawn(gx, gy) { Some(pose.pens[gy as usize * pose.w + gx as usize]) } else { None };
            let hot = live && attack.is_some_and(|a| gx >= a.0 && gx <= a.2 && gy >= a.1 && gy <= a.3);
            pixels[py * pw + px] = match (pen, hot) {
                (Some(p), false) => 1 + p.min(14),
                (Some(p), true) => 16 + p.min(14),
                (None, true) => 31,
                (None, false) => 0,
            };
        }
    }
    let t = (scale / 3).max(1);
    let mut outline = |r: (i32, i32, i32, i32), colour: u8| {
        let (x0, y0) = (((r.0 + ox) as usize) * scale, ((r.1 + oy) as usize) * scale);
        let (x1, y1) = (((r.2 + ox + 1) as usize) * scale, ((r.3 + oy + 1) as usize) * scale);
        for py in y0..y1.min(ph) {
            for px in x0..x1.min(pw) {
                if px < x0 + t || px + t >= x1 || py < y0 + t || py + t >= y1 {
                    pixels[py * pw + px] = colour;
                }
            }
        }
    };
    outline((0, 0, w - 1, h - 1), 36);
    outline(loose, 32);
    outline(tight, 33);
    if let Some(a) = attack {
        outline(a, if live { 34 } else { 37 });
    }
    // The grid centre.
    let (cx, cy) = (((w / 2 + ox) as usize) * scale, ((h / 2 + oy) as usize) * scale);
    for d in 0..scale * 2 {
        for k in 0..t {
            let (a, b) = (cx + d - scale, cy + k);
            if a < pw && b < ph {
                pixels[b * pw + a] = 35;
            }
            let (a, b) = (cx + k, cy + d - scale);
            if a < pw && b < ph {
                pixels[b * pw + a] = 35;
            }
        }
    }
    Picture { width: pw, height: ph, pixels }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pose_with(w: usize, h: usize, drawn: &[(usize, usize)]) -> Pose {
        let mut p = Pose::blank(w, h);
        for &(x, y) in drawn {
            p.pens[y * w + x] = 1;
        }
        p
    }

    #[test]
    fn a_box_is_centred_on_the_grid_with_y_up() {
        let b = BoxSpec { dy: 8, hh: 4, dx: -4, hw: 2 };
        // 32x48 grid: centre (16, 24); the box centre is (12, 16).
        assert_eq!(b.rect(32, 48), (10, 12, 14, 20));
    }

    #[test]
    fn overreach_measures_past_the_farthest_drawn_pixel_in_the_band() {
        // A fist at x 10..=12 on row 5; the box spans rows 4..=6 and
        // columns 8..=20, so it reaches 8 pixels past the fist.
        let p = pose_with(32, 16, &[(10, 5), (11, 5), (12, 5), (2, 9)]);
        let spec = BoxSpec { dy: 11, hh: 1, dx: 4, hw: 6 };
        // centre (16, 8) + (4, -11)... y0 = 8 - 11 - 1 = -4: use an exact box.
        let _ = spec;
        let spec = BoxSpec { dy: 3, hh: 1, dx: 4, hw: 6 };
        let m = measure_attack(&p, &spec);
        assert_eq!(m.rect, (14, 4, 26, 6));
        // Drawn pixels in rows 4..=6: only the fist, ending at x=12; the box
        // starts at 14, so forward overreach is 26 - 12.
        assert_eq!(m.overreach[0], Some(14));
        assert!(m.floating(), "the fist lies left of the box");
        // A box that does hold the fist.
        let spec = BoxSpec { dy: 3, hh: 1, dx: -2, hw: 6 };
        let m = measure_attack(&p, &spec);
        assert_eq!(m.rect, (8, 4, 20, 6));
        assert_eq!(m.inside, 3);
        assert_eq!(m.overreach[0], Some(8));
        assert_eq!(m.overreach[1], Some(2));
        assert_eq!(m.edge, 4);
    }

    #[test]
    fn the_body_measure_counts_drawn_pixels_outside_the_loose_box() {
        let p = pose_with(16, 16, &[(8, 8), (9, 8), (15, 8)]);
        let loose = BoxSpec { dy: 0, hh: 4, dx: 0, hw: 2 };
        let tight = BoxSpec::default();
        let b = measure_body(&p, &loose, &tight);
        assert_eq!(b.drawn, 3);
        assert_eq!(b.uncovered, 1);
        assert_eq!(b.extent[0], 15 - 10);
        assert_eq!(b.uncovered_box, Some((15, 8, 15, 8)));
    }

    #[test]
    fn a_frame_lasts_repeat_plus_one_times_duration_plus_one() {
        let mut rom = vec![0u8; 0x40];
        rom[0] = 1; // repeat
        rom[1] = 2; // duration
        rom[8] = 0;
        rom[9] = 0;
        let rec = FrameRecord::from_stream(&rom, 0).unwrap();
        assert_eq!(rec.ticks(), 6);
        assert_eq!((rec.width(), rec.height()), (16, 16));
    }

    #[test]
    fn a_stream_walk_counts_startup_from_the_first_tick() {
        // Two frames (1+1 and 2+1 ticks) then an active one, then the end.
        let mut rom = vec![0u8; 0x100];
        let frame = |rom: &mut Vec<u8>, at: usize, dur: u8, flags: u8, atk: bool| {
            rom[at + 1] = dur;
            rom[at + 0x1a] = flags;
            if atk {
                rom[at + 0x16..at + 0x1a].copy_from_slice(&[0, 4, 8, 4]);
            }
        };
        frame(&mut rom, 0x10, 1, 0, false);
        frame(&mut rom, 0x30, 2, 0, false);
        frame(&mut rom, 0x50, 3, 0x80, true);
        rom[0x70] = 0xfd;
        let w = walk_stream(&rom, None, 0x10);
        assert_eq!(w.end, PathEnd::End);
        assert_eq!(w.path.len(), 3);
        let mut tick = 1;
        let mut startup = None;
        for &i in &w.path {
            if w.frames[i].attack_active() {
                startup.get_or_insert(tick);
            }
            tick += w.frames[i].ticks();
        }
        assert_eq!(startup, Some(1 + 2 + 3));
    }

    #[test]
    fn a_picture_draws_the_boxes_over_the_pose() {
        let mut p = Pose::blank(16, 16);
        p.pens[8 * 16 + 8] = 3;
        let mut rec = FrameRecord::from_stream(&[0u8; 0x40], 0).unwrap();
        rec.loose = BoxSpec { dy: 0, hh: 2, dx: 0, hw: 2 };
        rec.tight = BoxSpec { dy: 0, hh: 1, dx: 0, hw: 1 };
        rec.attack = BoxSpec { dy: 0, hh: 3, dx: 4, hw: 3 };
        rec.flags = 0x80;
        let pic = draw_picture(&p, &rec, 3);
        assert_eq!(pic.pixels.len(), pic.width * pic.height);
        // The drawn pixel's colour is a grey of its pen, tinted red inside a live attack box.
        let has = |c: u8| pic.pixels.contains(&c);
        assert!(has(32) && has(33) && has(34) && has(35) && has(36), "loose, tight, attack, centre and edge outlines");
        // The same frame with its active bit clear draws the box in orange, untinted.
        rec.flags = 0;
        rec.hit_fn = 0x47234;
        let dim = draw_picture(&p, &rec, 3);
        assert!(dim.pixels.contains(&37) && !dim.pixels.contains(&34));
        assert!(dim.pixels.iter().all(|&c| !(16..=31).contains(&c)), "no red tint without a live box");
    }

    #[test]
    fn quartiles_and_tiers() {
        assert_eq!(quartiles(&[1.0, 2.0, 3.0, 4.0, 5.0]), Some([2.0, 3.0, 4.0]));
        assert_eq!(quartiles(&[]), None);
        assert_eq!(startup_tier(3), "at or below the research's floor (3)");
        assert_eq!(startup_tier(4), "light (4 to 5)");
        assert_eq!(startup_tier(9), "heavy (9 to 12)");
        assert_eq!(startup_tier(30), "slower than a modern heavy (12+)");
    }

    #[test]
    fn a_stream_walk_follows_the_spin_loop_and_stops_at_the_follow_up() {
        // frame A, loop start (3 repeats), frame B, loop end, follow-up call.
        let mut rom = vec![0u8; 0x200];
        let call = |rom: &mut Vec<u8>, at: usize, count: u8, routine: u32| {
            rom[at] = 0xff;
            rom[at + 1] = count;
            rom[at + 2..at + 6].copy_from_slice(&routine.to_be_bytes());
        };
        // Offsets: A at 0x10, loop start at 0x30, B at 0x36, loop end at 0x56, follow-up at 0x5c.
        call(&mut rom, 0x30, 3, 0x4550a);
        call(&mut rom, 0x56, 0, 0x45526);
        call(&mut rom, 0x5c, 0, 0x45630);
        let w = walk_stream(&rom, None, 0x10);
        assert_eq!(w.end, PathEnd::FollowUp);
        // A once and B four times (one pass and three repeats).
        assert_eq!(w.path.len(), 1 + 4);
        assert!(w.unknown_calls.is_empty());
    }
}
