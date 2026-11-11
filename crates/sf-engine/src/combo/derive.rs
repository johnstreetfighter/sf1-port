//! The hurtboxes of the hit reactions a combo can land on, measured from
//! what each reaction frame draws (ticket #261, `SP17`).
//!
//! In 1987 every reaction frame has a zero-size hurtbox and the invulnerable
//! flag (`docs/reference/rom-box-audit.md`), so true combos need real boxes
//! for them, and the design's rule binds every box this project writes:
//! the boxes must match what is drawn. This module derives them from the
//! program ROM and the sprite ROM, read only; `tests/reaction_boxes.rs`
//! writes the result into [`super::reaction_boxes`], which the engine reads
//! (the engine has no sprite ROM), and checks the committed table is
//! current and that every box passes the ROM box audit's own thresholds.
//!
//! **Which frames.** Every frame of every *ground* hit-reaction stream (the
//! hit table entries `0x30..=0x3c` whose state `& 7` is not 1: standing
//! `0x8a`, crouching `0x8c`) of all twelve fighters' 1987 hit tables: Ryu's
//! (`0x44662`), Ken's (`0x4473a`) and each boss's CPU table, which a boss
//! keeps as a player (`Bus::bound_boss_tables`). Airborne reactions and
//! knock-outs stay invulnerable, so they need none.
//!
//! **How a box is made.** The reference is the fighter's own 1987 posture
//! the reaction starts from: his stance (move 1) for a standing reaction,
//! his crouch (move 5, when his table has a crouch pose with a hurtbox) for
//! a crouching one.
//!
//! - A reaction frame that draws the reference's own pose (Ryu's and Ken's
//!   reactions end on their stance) takes the reference's 1987 boxes as they
//!   are.
//! - Any other frame keeps the reference's box sizes (the 1987 torso box,
//!   which leaves arms and legs outside by design) and places them on the
//!   drawing: the loose box centred on the median column of the drawn
//!   pixels (where the body's mass is) and on the drawn rows, no taller
//!   than the drawing; the tight (head) box at the reference's distance
//!   below the drawing's top, centred on the median column of the rows it
//!   spans. A reference with a zero-size tight box (Adon's, Sagat's) keeps
//!   it.
//!
//! The pushboxes stay the ROM's.

use std::collections::BTreeMap;

use crate::box_audit::{measure_body, BoxSpec, FrameRecord, Pose, walk_stream};
use crate::roster::TournamentFighter;

/// A fighter's hit table (alive), and whether its stream words are signed
/// (Ryu's and Ken's, `movea.w`) or not (the bosses', `0x447c8`).
pub fn hit_table(fighter: TournamentFighter) -> (u32, bool) {
    match fighter {
        TournamentFighter::Ryu => (0x44662, true),
        TournamentFighter::Ken => (0x4473a, true),
        f => (f.reaction_tables().expect("a boss has a hit table").0, false),
    }
}

fn rb(rom: &[u8], a: u32) -> u8 {
    rom.get(a as usize).copied().unwrap_or(0xff)
}

fn rw(rom: &[u8], a: u32) -> u16 {
    u16::from_be_bytes([rb(rom, a), rb(rom, a + 1)])
}

/// Whether a hit-table entry's state byte is a ground reaction (standing or
/// crouching), which the combo gate makes hittable.
pub fn ground_state(state: u8) -> bool {
    state & 7 != 1
}

/// How a reaction frame's boxes were made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// The frame draws the reference's pose: its 1987 boxes.
    Reference,
    /// Placed on the drawing.
    Drawn,
}

/// One reaction frame's boxes, with what the audit's measures say of them.
#[derive(Clone, Debug)]
pub struct ReactionBoxes {
    pub fighter: TournamentFighter,
    /// The frame record's address in the program ROM.
    pub frame: u32,
    /// The hit-table moves whose streams play it.
    pub moves: Vec<u8>,
    /// True for a crouching reaction.
    pub crouching: bool,
    pub tight: BoxSpec,
    pub loose: BoxSpec,
    pub source: Source,
    /// The share of drawn pixels outside the loose box, less the
    /// reference's own share (the audit's "drawing outgrows the hurtbox").
    pub excess: f64,
    /// How far the loose box lies past the drawing (the audit's "hurtbox
    /// past the drawing"), at worst.
    pub overreach: i32,
    /// How far the tight box sticks out of the loose box, less the
    /// reference's own (the audit's "critical box outside the hurtbox").
    pub tight_out: i32,
}

impl ReactionBoxes {
    /// The eight entity bytes `+0x1a..=+0x21` (tight, then loose: dy, hh,
    /// dx, hw), facing right.
    pub fn bytes(&self) -> [u8; 8] {
        let (t, l) = (self.tight, self.loose);
        [t.dy as u8, t.hh, t.dx as u8, t.hw, l.dy as u8, l.hh, l.dx as u8, l.hw]
    }
}

/// How far the tight box sticks out of the loose box (as the audit's
/// `stance_baseline`).
fn tight_overhang(pose: &Pose, loose: &BoxSpec, tight: &BoxSpec) -> i32 {
    if tight.is_empty() || loose.is_empty() {
        return 0;
    }
    let (l, t) = (loose.rect(pose.w, pose.h), tight.rect(pose.w, pose.h));
    (l.0 - t.0).max(t.2 - l.2).max(l.1 - t.1).max(t.3 - l.3).max(0)
}

/// The drawn pixels of a pose, as (x, y).
fn drawn(pose: &Pose) -> Vec<(i32, i32)> {
    let mut out = Vec::new();
    for y in 0..pose.h as i32 {
        for x in 0..pose.w as i32 {
            if pose.drawn(x, y) {
                out.push((x, y));
            }
        }
    }
    out
}

fn median(mut v: Vec<i32>) -> i32 {
    v.sort_unstable();
    v[v.len() / 2]
}

/// A reference posture: its record, pose, the drawing's top, its uncovered
/// share and its tight overhang.
struct Reference {
    rec: FrameRecord,
    top: i32,
    share: f64,
    tight_out: i32,
}

impl Reference {
    fn new(gfx3: &[u8], rec: FrameRecord) -> Self {
        let pose = Pose::of_frame(gfx3, &rec);
        let top = drawn(&pose).iter().map(|p| p.1).min().unwrap_or(0);
        let share = measure_body(&pose, &rec.loose, &rec.tight).uncovered_share();
        let tight_out = tight_overhang(&pose, &rec.loose, &rec.tight);
        Reference { rec, top, share, tight_out }
    }

    fn same_pose(&self, rec: &FrameRecord) -> bool {
        self.rec.code == rec.code && self.rec.rows == rec.rows && self.rec.cols == rec.cols && self.rec.big() == rec.big()
    }
}

/// Place the reference's boxes on `rec`'s drawing (the module docs).
fn place(pose: &Pose, rec: &FrameRecord, r: &Reference) -> (BoxSpec, BoxSpec) {
    let px = drawn(pose);
    let (w, h) = (pose.w as i32, pose.h as i32);
    let y0 = px.iter().map(|p| p.1).min().unwrap_or(0);
    let y1 = px.iter().map(|p| p.1).max().unwrap_or(h - 1);
    let mx = median(px.iter().map(|p| p.0).collect());
    let hh = i32::from(r.rec.loose.hh).min((y1 - y0) / 2);
    let cy = (y0 + y1) / 2;
    let loose = BoxSpec {
        dy: (h / 2 - cy) as i8,
        hh: hh as u8,
        dx: (mx - w / 2) as i8,
        hw: r.rec.loose.hw,
    };
    let tight = if r.rec.tight.is_empty() {
        r.rec.tight
    } else {
        let (rw_, rh_) = (r.rec.width() as i32, r.rec.height() as i32);
        let (_, ref_top, _, _) = r.rec.tight.rect(rw_ as usize, rh_ as usize);
        let gap = (ref_top - r.top).max(0);
        let th = i32::from(r.rec.tight.hh);
        let ty = (y0 + gap + th).min(cy);
        let band: Vec<i32> = px.iter().filter(|p| (p.1 - ty).abs() <= th).map(|p| p.0).collect();
        let tx = if band.is_empty() { mx } else { median(band) };
        BoxSpec { dy: (h / 2 - ty) as i8, hh: r.rec.tight.hh, dx: (tx - w / 2) as i8, hw: r.rec.tight.hw }
    };
    let _ = rec;
    (tight, loose)
}

/// Every ground reaction frame of `fighter` with its boxes.
pub fn fighter_reaction_boxes(rom: &[u8], gfx3: &[u8], fighter: TournamentFighter) -> Vec<ReactionBoxes> {
    let table = fighter.move_table();
    let stance = Reference::new(gfx3, FrameRecord::from_header(rom, table + 0x30).expect("a stance header"));
    let crouch = FrameRecord::from_header(rom, table + 5 * 0x30)
        .filter(|c| !c.loose.is_empty() && c.code != 0)
        .map(|c| Reference::new(gfx3, c));
    let (alive, signed) = hit_table(fighter);
    let mut frames: BTreeMap<u32, (FrameRecord, Vec<u8>, bool)> = BTreeMap::new();
    for n in 0..13u32 {
        let e = alive + 4 * n;
        let state = rb(rom, e + 2);
        if !ground_state(state) {
            continue;
        }
        let w = rw(rom, e);
        let stream = if signed { w as i16 as i32 as u32 } else { u32::from(w) };
        let walk = walk_stream(rom, None, stream);
        for rec in walk.frames {
            let entry = frames.entry(rec.addr).or_insert_with(|| (rec.clone(), Vec::new(), state & 7 == 4));
            entry.1.push(0x30 + n as u8);
        }
    }
    frames
        .into_values()
        .map(|(rec, moves, crouching)| {
            let r = if crouching { crouch.as_ref().unwrap_or(&stance) } else { &stance };
            let pose = Pose::of_frame(gfx3, &rec);
            let (tight, loose, source) = if r.same_pose(&rec) {
                (r.rec.tight, r.rec.loose, Source::Reference)
            } else {
                let (t, l) = place(&pose, &rec, r);
                (t, l, Source::Drawn)
            };
            let m = measure_body(&pose, &loose, &tight);
            let overreach = m.overreach.iter().flatten().copied().max().unwrap_or(0);
            ReactionBoxes {
                fighter,
                frame: rec.addr,
                moves,
                crouching,
                tight,
                loose,
                source,
                excess: m.uncovered_share() - r.share,
                overreach,
                tight_out: tight_overhang(&pose, &loose, &tight) - r.tight_out,
            }
        })
        .collect()
}

/// Every ground reaction frame of the twelve fighters, in address order.
pub fn all_reaction_boxes(rom: &[u8], gfx3: &[u8]) -> Vec<ReactionBoxes> {
    let mut out: Vec<ReactionBoxes> =
        TournamentFighter::ALL.iter().flat_map(|&f| fighter_reaction_boxes(rom, gfx3, f)).collect();
    out.sort_by_key(|b| b.frame);
    out
}

/// The source of [`super::reaction_boxes`] for `boxes`.
pub fn render_table(boxes: &[ReactionBoxes]) -> String {
    let mut s = String::new();
    s.push_str("//! Generated by `tests/reaction_boxes.rs` from the program and sprite ROMs\n");
    s.push_str("//! (`super::derive`); do not edit. To rewrite it:\n");
    s.push_str("//! `UPDATE_REACTION_BOXES=1 cargo test --release -p sf-engine --test reaction_boxes`.\n");
    s.push_str("//!\n");
    s.push_str("//! Each row: a ground hit-reaction frame record's ROM address and the\n");
    s.push_str("//! entity bytes `+0x1a..=+0x21` the combo gate writes when it loads (tight\n");
    s.push_str("//! box, then loose box: dy, hh, dx, hw; facing right), with its fighter,\n");
    s.push_str("//! the reactions that play it, and how the boxes were made.\n\n");
    s.push_str("/// Ground hit-reaction frames and their hurtboxes, by frame address.\n");
    s.push_str("pub const REACTION_BOXES: &[(u32, [u8; 8])] = &[\n");
    for b in boxes {
        let bytes: Vec<String> = b.bytes().iter().map(|v| format!("0x{v:02x}")).collect();
        let moves: Vec<String> = b.moves.iter().map(|m| format!("0x{m:02x}")).collect();
        let how = match b.source {
            Source::Reference => if b.crouching { "the crouch's own" } else { "the stance's own" },
            Source::Drawn => "placed on the drawing",
        };
        s.push_str(&format!(
            "    (0x{:05x}, [{}]), // {} {}: {}\n",
            b.frame,
            bytes.join(", "),
            b.fighter.name(),
            moves.join(" "),
            how
        ));
    }
    s.push_str("];\n");
    s
}
