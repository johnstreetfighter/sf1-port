//! The fight reader (spec #216, `EH5`): what the real 1987 fight shows each
//! frame, read from its work and object RAM and never written.
//!
//! Every overhaul fight runs the real 1987 fight, so its RAM already holds
//! what the era HUD needs: each side's health and hit reaction. A
//! [`FightReader`] is handed the RAM once a frame, after the machine has
//! stepped, and keeps only its own copy of the last frame's read; it takes
//! the RAM as shared slices, so it cannot change it. Its [`FightRead`] is
//! what the call-outs are judged from: the [`ComboCounter`] here, and the
//! round's FIRST ATTACK, REVERSAL and PERFECT ([`RoundMoments`], `EH6`).
//!
//! **What is read.** Each side's player structure (P1 `0xff86b0`; P2
//! `0xff86da`, or the CPU's `0xffc664` in its place) by the game mode at
//! `0xffc002`, as the diagnostic overlay reads them:
//!
//! - health: +7 cells × 4 + the +6 remainder (`0x3c034`);
//! - its own fighter entity, the low word of +0xe (`0x400` or `0xa00`).
//!
//! And from that entity in object RAM: +8 (0 for no fighter, 2 for a live
//! reaction, 8 knocked out), +9 the state byte (bit 7 busy, bit 5 a guard
//! move) and +0xa the move. The defender's reaction (`0x4451a`) starts a
//! hit-reaction move `0x30..=0x3f`, or `0x40..` with no health left.
//!
//! **A landed hit** is a frame on which a side's health went down and it is
//! in a hit reaction (alive or knocked out) after it. The attacker is the
//! other side. A hit that does no damage is not seen.
//!
//! **Training.** The health floor holds a fighter at one point, so a hit
//! there takes no health and one that reaches it takes less than its
//! damage. In a Training fight the reader is also handed the machine's
//! Training state, read-only, and takes the damage from its hit record
//! ([`TrainingState::damage_taken`], each hit's full damage before the
//! floor): a landed hit is a frame on which the record's total for a side
//! rose and it is in a hit reaction after it. The reader keeps the last
//! frame's totals as it keeps the last frame's health, so each hit is read
//! once, on the frame the machine stepped it; a restored snapshot brings
//! its record back with it, and [`FightReader::reset`] forgets the totals
//! with the rest. So in Training the hits and their damage are those of a
//! fight with unlimited health.
//!
//! **Chip.** A blocked normal does no damage (its hit function tests the
//! guard move). Only the Tatsumaki's hit function (`0x45f5a`, move `0x1b`)
//! and the projectile's (`0x433ba`) chip a guard, and both judge it by the
//! defender's +9 bit `0x20`; the chipped defender takes reaction `0x30`, so
//! chip is a landed hit too. A landed hit is chip when the defender's +9
//! had bit `0x20` the frame before and the attacker is not in a melee
//! attack (`0x0d..=0x2f` but the Hadoken `0x1a` and Tatsumaki `0x1b`): a
//! melee hit through a guard (a jump attack on the crouching guard, a
//! Shoryuken, Retsu's `0x18`) is a clean hit. The special ids are Ryu's and
//! Ken's; a boss on his 1987 player table (ADR-0014) says which of his
//! specials are melee in his table's special slots, which the reader reads
//! from the extended data bank ([`FightReader::read_banked`]). A CPU's
//! projectile that lands while its thrower is still in a melee-numbered
//! move reads as clean.
//!
//! **No true combos in 1987.** From the frame after a hit until its reaction
//! ends, the defender's entity +0xf bit 3 (invulnerable) is set, and the
//! collision judge (`0x43e82`) and the projectile collision skip such a
//! target. So a real SF1 fight under the classic rules never lands a hit
//! into hitstun and the combo count stays silent. **True combos** (SP17,
//! [`crate::combo`]) make a ground reaction hittable and cut or hold it to
//! the attacking move's hitstun, so the reaction the reader sees (a
//! reaction move while busy, [`FighterRead::hitstun`]) is the move's
//! hitstun, and a hit that lands in it is a combo hit. Blockstun comes from
//! the machine's combo state. Tests also drive the counter with fed reads
//! (`Fight::feed_reads`).

use sf_video::callouts::{CalloutEvent, CalloutKind, ComboCount, Side};

use crate::combo::ComboState;
use crate::fighter::{CPU_STRUCT, GAME_MODE, P1_STRUCT, P2_STRUCT, WORK_BASE};
use crate::training::TrainingState;

/// The fighter entities' slots in object RAM.
pub const FIGHTER_SLOTS: [u16; 3] = [0x400, 0xa00, crate::fighter::DRAMATIC_ALLY2];

/// One side's fighter as read this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FighterRead {
    /// A fighter is in the side's entity slot (entity +8 not 0).
    pub present: bool,
    /// Health points: 4 a cell (`0x3c034`); 0 is knocked out.
    pub health: u16,
    /// The move (entity +0xa) and the state byte (entity +9).
    pub move_id: u8,
    pub state: u8,
    /// True blockstun: a guard that locks the fighter until it ends. In
    /// 1987 there is none: a normal block (moves 4 and 8, +9 `0x22` /
    /// `0x24`) passes the busy gate (`0x403d4`), so the guard can attack on
    /// any frame of it, and chip's reaction `0x30` is a hit reaction, not
    /// blockstun. Under true combos (SP17) a guarded melee hit locks a
    /// player's guard for the move's blockstun, and the reader reads it from
    /// the machine's combo state ([`FightReader::read_with_combos`]).
    pub blockstun: bool,
    /// Getting up from a knockdown. The 1987 reader always leaves it false:
    /// every knockdown is a knock-out (no hit stream gets up from lying,
    /// `docs/reference/art-inventory.md`). Project 4's era styles set it.
    pub getting_up: bool,
    /// The fighter's attack box is live this frame: entity +0xf bit 7, set
    /// by an attack's active frame records and cleared by its hit (the
    /// collision judge `0x43e82` tests it). Startup, active and recovery
    /// frames are read from it.
    pub attack_active: bool,
}

impl FighterRead {
    /// In a live hit reaction while it is busy: hitstun, the window in which
    /// the next hit combos.
    pub fn hitstun(&self) -> bool {
        self.present
            && (0x30..=0x3f).contains(&self.move_id)
            && self.move_id != crate::combo::throw::THROWN_MOVE
            && self.state & 0x80 != 0
    }

    /// Knocked out: the reaction with no health left (`0x40..`).
    pub fn knocked_out(&self) -> bool {
        self.present && self.move_id >= 0x40
    }

    /// In a hit reaction, live or knocked out.
    pub fn reacting(&self) -> bool {
        self.present && self.move_id >= 0x30
    }

    /// In an attack move (`0x0d..=0x2f`: the normals, specials, jump
    /// attacks and throws).
    pub fn attacking(&self) -> bool {
        self.present
            && (0x0d..=0x2f).contains(&self.move_id)
            && self.move_id != crate::combo::throw::GET_UP_MOVE
    }

    /// Locked out of acting by blockstun or a get-up (not a hit reaction).
    pub fn locked(&self) -> bool {
        self.present && (self.blockstun || self.getting_up)
    }

    /// Guarding: the standing or crouching guard move (4, 8) or the state's
    /// guard bit, as the frame meter's blockstun.
    pub fn guarding(&self) -> bool {
        self.present && (self.move_id == 4 || self.move_id == 8 || self.state & 0x20 != 0)
    }
}

/// A hit that landed this frame, by the side that landed it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LandedHit {
    /// The health it took.
    pub damage: u16,
    /// The defender was in hitstun the frame before: the hit combos.
    pub into_hitstun: bool,
    /// Chip damage on a guard, not a clean hit (see the module docs).
    pub chip: bool,
}

/// A melee attack: one whose own hit function lands it, through a guard or
/// not (the module docs' chip rule). A special is melee as the slot its
/// fighter's table gives it says (`slot`: [`crate::data_bank::special_slot`],
/// on Ryu's and Ken's ROM tables the Shoryuken alone).
fn melee_attack(move_id: u8, slot: impl Fn(u8) -> crate::data_bank::SpecialSlot) -> bool {
    if crate::data_bank::SPECIAL_SLOTS.contains(&move_id) {
        return slot(move_id).melee;
    }
    (0x0d..=0x2f).contains(&move_id)
}

/// One frame of the fight as the reader saw it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FightRead {
    /// P1's side, then P2's.
    pub fighters: [FighterRead; 2],
    /// The hit each side landed this frame, by the attacker.
    pub landed: [Option<LandedHit>; 2],
    /// Each side's attack was guarded this frame, by the attacker. The
    /// machine supplies it to `Fight` because a RAM read alone cannot
    /// distinguish a blocked normal from a whiff.
    pub blocked: [bool; 2],
}

impl FightRead {
    pub fn fighter(&self, side: Side) -> &FighterRead {
        &self.fighters[side.index()]
    }

    /// The hit `side` landed this frame.
    pub fn landed_by(&self, side: Side) -> Option<LandedHit> {
        self.landed[side.index()]
    }

    /// `side`'s attack connected this frame: it hit, or it was blocked.
    pub fn connected(&self, side: Side) -> bool {
        self.landed[side.index()].is_some() || self.blocked[side.index()]
    }
}

/// Reads the real fight's RAM once a frame; see the module docs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FightReader {
    /// The last frame's fighters, and the hit record's totals in Training.
    last: Option<([FighterRead; 2], Option<[u16; 2]>)>,
}

/// `side`'s player structure in game mode `mode`: the CPU's stands in for
/// P2 in one-player play (mode 2) and for P1 in mode 4.
pub(crate) fn player_struct(side: Side, mode: u8) -> u32 {
    match (side, mode) {
        (Side::P1, 4) | (Side::P2, 2) => CPU_STRUCT,
        (Side::P1, _) => P1_STRUCT,
        (Side::P2, _) => P2_STRUCT,
    }
}

impl FightReader {
    pub fn new() -> Self {
        Self::default()
    }

    /// Forget the last frame (the machine jumped: a restarted round).
    pub fn reset(&mut self) {
        self.last = None;
    }

    /// Read this frame from `work` (`0xff8000..`) and `obj` (`0xffe000..`)
    /// RAM, the frame after the last read; `training` is the machine's
    /// Training state in a Training fight, and `None` in any other.
    pub fn read(&mut self, work: &[u8], obj: &[u8], training: Option<&TrainingState>) -> FightRead {
        self.read_banked(work, obj, None, training)
    }

    /// [`Self::read`] for a machine holding the extended data bank (an
    /// overhaul fight's, ADR-0014): a boss's special slots are read from his
    /// table there.
    pub fn read_banked(&mut self, work: &[u8], obj: &[u8], bank: Option<&crate::data_bank::DataBank>, training: Option<&TrainingState>) -> FightRead {
        self.read_with_combos(work, obj, bank, training, None)
    }

    /// [`Self::read_banked`], with the machine's true-combo state, given only
    /// with the switch on (`None` with it off): a side in blockstun reads as
    /// [`FighterRead::blockstun`], and a landed hit is into hitstun when the
    /// engine counted it as its combo's second hit or later
    /// ([`ComboState::hits`]). Without it a hit is into hitstun when the
    /// defender read in a hit reaction the frame before, which can also
    /// catch a hit landing as a 1987 reaction's stream ends.
    pub fn read_with_combos(
        &mut self,
        work: &[u8],
        obj: &[u8],
        bank: Option<&crate::data_bank::DataBank>,
        training: Option<&TrainingState>,
        combo: Option<&ComboState>,
    ) -> FightRead {
        let wb = |a: u32| work.get((a - WORK_BASE) as usize).copied().unwrap_or(0);
        let ob = |e: u16, off: u32| obj.get((u32::from(e) + off) as usize).copied().unwrap_or(0);
        let mode = wb(GAME_MODE);
        let fighters = Side::BOTH.map(|side| {
            let ps = player_struct(side, mode);
            let health = 4 * u16::from(wb(ps + 7)) + u16::from(wb(ps + 6));
            let entity = u16::from_be_bytes([wb(ps + 0x10), wb(ps + 0x11)]);
            if !FIGHTER_SLOTS[..2].contains(&entity) || ob(entity, 8) == 0 {
                return FighterRead { health, ..FighterRead::default() };
            }
            FighterRead {
                present: true,
                health,
                move_id: ob(entity, 0xa),
                state: ob(entity, 9),
                attack_active: ob(entity, 0xf) & 0x80 != 0,
                blockstun: combo.is_some_and(|c| c.blockstun[crate::combo::slot(ps, mode)] != 0),
                getting_up: ob(entity, 0xa) == crate::combo::throw::GET_UP_MOVE && ob(entity, 9) & 0x80 != 0,
            }
        });
        let taken = training.map(|t| t.damage_taken);
        let mut landed = [None; 2];
        if let Some((last, last_taken)) = self.last {
            for side in Side::BOTH {
                let d = 1 - side.index();
                let (was, now) = (last[d], fighters[d]);
                // In Training the hit record's rise (the module docs),
                // elsewhere the health's fall.
                let damage = match (taken, last_taken) {
                    (Some(taken), Some(last_taken)) => taken[d].wrapping_sub(last_taken[d]),
                    _ => was.health.saturating_sub(now.health),
                };
                if damage > 0 && now.reacting() {
                    let attacker = player_struct(side, mode);
                    let table = u32::from_be_bytes([wb(attacker + 0x12), wb(attacker + 0x13), wb(attacker + 0x14), wb(attacker + 0x15)]);
                    let slot = |mv| crate::data_bank::special_slot(|a| bank.map_or(0, |b| b.rb(a)), table, mv);
                    let chip = was.state & 0x20 != 0 && !melee_attack(fighters[side.index()].move_id, slot);
                    // Under true combos the engine's own count: the hit was
                    // its combo's second or later (it landed while the
                    // defender's combo-able reaction ran). Otherwise the
                    // defender read in a hit reaction the frame before.
                    let into_hitstun = match combo {
                        Some(c) => c.hits[crate::combo::slot(player_struct(Side::BOTH[d], mode), mode)] >= 2,
                        None => was.hitstun(),
                    };
                    landed[side.index()] = Some(LandedHit { damage, into_hitstun, chip });
                }
            }
        }
        self.last = Some((fighters, taken));
        FightRead { fighters, landed, blocked: [false; 2] }
    }
}

/// Frames a finished combo's count stays up.
pub const COMBO_LINGER_FRAMES: u16 = 60;

/// One attacker's combo.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ComboRun {
    /// The hits and damage of the combo running, while the defender is
    /// still in hitstun from the last one.
    hits: u16,
    damage: u16,
    running: bool,
    /// The last finished combo of two or more, and the frames it has left
    /// (none while it is held).
    last: Option<(u16, u16)>,
    linger: u16,
}

/// The combo count on each side, from the fight reader: every hit that
/// lands while the opponent is still in hitstun from the one before. A
/// count shows from the second hit and for [`COMBO_LINGER_FRAMES`] after
/// the defender leaves hitstun; held, the last count stays up until the
/// next combo reaches its second hit.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ComboCounter {
    runs: [ComboRun; 2],
}

impl ComboCounter {
    pub fn new() -> Self {
        Self::default()
    }

    /// One frame of `read`; `hold` keeps each side's last count up. Returns
    /// the hits of each side's combo that finished this frame, if any.
    pub fn step(&mut self, read: &FightRead, hold: bool) -> [Option<u16>; 2] {
        let mut finished = [None; 2];
        for side in Side::BOTH {
            let run = &mut self.runs[side.index()];
            let defender = read.fighters[1 - side.index()];
            if let Some(hit) = read.landed_by(side) {
                if run.running && hit.into_hitstun {
                    run.hits += 1;
                    run.damage += hit.damage;
                } else {
                    (run.hits, run.damage, run.running) = (1, hit.damage, true);
                }
                if run.hits >= 2 {
                    run.last = None;
                }
            } else if run.running && !defender.hitstun() {
                run.running = false;
                if run.hits >= 2 {
                    run.last = Some((run.hits, run.damage));
                    run.linger = COMBO_LINGER_FRAMES;
                    finished[side.index()] = Some(run.hits);
                }
            } else if run.last.is_some() && !hold {
                run.linger = run.linger.saturating_sub(1);
                if run.linger == 0 {
                    run.last = None;
                }
            }
        }
        finished
    }

    /// Stop the combos running (the machine jumped); finished counts stay.
    pub fn cut(&mut self) {
        for run in &mut self.runs {
            run.running = false;
        }
    }

    /// The count `side` shows, with its damage when `damage` asks for it.
    pub fn shown(&self, side: Side, damage: bool) -> Option<ComboCount> {
        let run = &self.runs[side.index()];
        let (hits, dealt) = if run.running && run.hits >= 2 { (run.hits, run.damage) } else { run.last? };
        Some(ComboCount { hits, damage: damage.then_some(dealt) })
    }
}

/// The round's FIRST ATTACK, REVERSAL and PERFECT, from the fight reader
/// (`EH6`).
///
/// - **FIRST ATTACK**: the round's first clean landed hit (not chip), on
///   the attacker's side, once a round.
/// - **REVERSAL** is literal (user, 2026-11-04): an attack move that starts
///   on the first frame after true blockstun or a get-up ends
///   ([`FighterRead::locked`]), called when it then connects (hits, or is
///   blocked) while that attack is still out. A hit reaction, chip's
///   included, is not such a lock, so a real 1987 fight never earns one; it
///   shows once a style reads blockstun or get-ups.
/// - **PERFECT**: on the frame a side is knocked out (`0x40..`), on the
///   other side's, if its health never went down this round. A double KO
///   has no winner and a time-over no knockout, so neither has one.
///
/// A round starts when the game-flow task reaches the round start
/// (`0x3ea3e`, after the round initialisation refills both sides' health),
/// or the machine is put back there (Restart Round); [`Fight`] calls
/// [`RoundMoments::new_round`] then.
///
/// [`Fight`]: crate::front::fight::Fight
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RoundMoments {
    first_attack: bool,
    knockout: bool,
    damaged: [bool; 2],
    last: Option<[FighterRead; 2]>,
    /// Each side's attack out of a lock, by its move, until it connects or ends.
    reversal: [Option<u8>; 2],
}

impl RoundMoments {
    pub fn new() -> Self {
        Self::default()
    }

    /// A new round (or the current one restarted): everything is earned again.
    pub fn new_round(&mut self) {
        *self = Self::default();
    }

    /// One frame of `read`: the moments it earned, in side order.
    pub fn step(&mut self, read: &FightRead) -> Vec<CalloutEvent> {
        let mut events = Vec::new();
        for side in Side::BOTH {
            let i = side.index();
            let now = read.fighters[i];
            if let Some(last) = self.last {
                self.damaged[i] |= now.health < last[i].health;
                if last[i].locked() && !now.locked() && now.attacking() {
                    self.reversal[i] = Some(now.move_id);
                }
            }
            if let Some(move_id) = self.reversal[i] {
                if !now.attacking() || now.move_id != move_id {
                    self.reversal[i] = None;
                } else if read.connected(side) {
                    self.reversal[i] = None;
                    events.push(CalloutEvent::new(side, CalloutKind::Reversal));
                }
            }
        }
        self.last = Some(read.fighters);
        for side in Side::BOTH {
            if read.landed_by(side).is_some_and(|hit| !hit.chip) && !self.first_attack {
                self.first_attack = true;
                events.push(CalloutEvent::new(side, CalloutKind::FirstAttack));
            }
        }
        let out = read.fighters.map(|f| f.knocked_out());
        if !self.knockout && out.contains(&true) {
            self.knockout = true;
            if let [false, true] | [true, false] = out {
                let winner = if out[0] { Side::P2 } else { Side::P1 };
                if !self.damaged[winner.index()] {
                    events.push(CalloutEvent::new(winner, CalloutKind::Perfect));
                }
            }
        }
        events
    }
}
