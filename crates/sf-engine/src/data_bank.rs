//! The extended data bank (ADR-0014; spec #246, ticket #248, `SP2`): move
//! tables in Ryu's format for the bosses, which the translated 1987 player
//! code reads through an overhaul-only gate, as the extended sprite bank
//! (ADR-0007) holds their poses' tiles.
//!
//! **Where it lives.** The 1987 program region ends at `0x05ffff`, and the
//! 68000's address space above it holds nothing the game reads until the
//! video RAM at `0x800000`: the bus has always answered `0xff` there. The
//! bank takes [`DATA_BANK_BASE`]`..=`[`DATA_BANK_END`] in that gap. A machine
//! is given a bank only for an overhaul fight (the session's gate, as the
//! extended background bank's); without one the region still reads `0xff`,
//! so Faithful 1987 reads exactly what it always did. Each fighter's data
//! sits in a fixed [`FIGHTER_SPAN`] slot by his place in
//! [`TournamentFighter::ALL`], his move table first, so a table address names
//! its fighter ([`fighter_of_table`]).
//!
//! **What a table holds.** 64 big-endian pointers indexed by move ID. Each
//! populated entry names a `0x30`-byte move header in the fighter's slot,
//! holding its first frame and a pointer to the rest of its stream. The ROM
//! tables still store their headers inline. Streams are 1987 streams: `0x20`-byte
//! frame records (pose tiles, the step, the four boxes, flags, x-offset,
//! hit function) and `0xff` / `0xfe` / `0xfd` control entries. A stream may
//! run on into the ROM (a boss's own 1987 attack, kept with its ROM boxes).
//! A zero pointer is an empty slot. For a special, its header's first-frame
//! byte (`+8`) also indicates whether the input handler can start it.
//!
//! **After the table**: the long naming its jump-apex record
//! ([`APEX_POINTER`]), then the **special slots** ([`SPECIALS`], ticket
//! #251): a [`SPECIAL_SLOT_SIZE`]-byte record for each of the three special
//! moves `0x19`, `0x1a` and `0x1b` ([`SpecialSlot`]), so everything the
//! engine decides about a special comes from the boss's table and not from
//! which slot it sits in (Ryu's and Ken's ROM tables keep the 1987 answers
//! for their three, [`SpecialSlot::ryu`]):
//! - `+0` **command**: the 1987 command tracker that fires it, by its player
//!   structure offset (`0x16` down, down-back, back; `0x19` forward, down,
//!   down-forward; `0x1c` down, down-forward, forward), with bit 7 when a
//!   kick fires it rather than a punch. The input handler fires the slot
//!   when its tracker is done and its button is pressed holding the
//!   command's last direction (or, under lenient commands, any direction it
//!   allows). The byte can grow: the low seven bits are a command *kind*, and
//!   only the three tracker offsets are kinds yet, so a new kind (Gen's
//!   mashed Hyakurenko, a press counter) takes another value, its recogniser
//!   in the input handler and its case in [`validate_specials`], which
//!   rejects any value it does not know.
//! - `+1` **flags**: [`SPECIAL_MELEE`] (it lands by its own hit function, so
//!   a hit through a guard is clean; without it, it is a projectile or
//!   chips, as the Hadoken and Tatsumaki do: the fight reader's chip rule)
//!   and [`SPECIAL_LOW_IN_AIR`] (guarded crouching even while he is in the
//!   air, as the Shoryuken and Tatsumaki are under standard high and low
//!   blocking; otherwise an airborne special is an overhead).
//! - `+2..+3` spare, 0.
//! - `+4` **score entry** (long): the damage entry whose score byte the
//!   special scores with when it lands with a CPU's hit function (0: none;
//!   a projectile scores by its own hit record).
//!
//! A present slot's record is checked when the bank is built
//! ([`validate_specials`]): a known command, known flags, and no two
//! slots on the same tracker and button.
//!
//! **Format extensions** (each overhaul-only, because only a bank stream can
//! reach them):
//! - **Bank routines.** A stream's `0xff` entry names a routine by address.
//!   An address inside the bank names a *routine record* there instead of
//!   68000 code: its first byte says what it does. [`ROUTINE_PROJECTILE`]
//!   (`kind, sound, 0, 0, template.l`) is the Hadoken's spawn (`0x4548c`)
//!   with the template a parameter, since the 1987 routine hard-codes Ryu's.
//!   [`ROUTINE_JUMP_APEX`] (`kind, 0, landing centre y.w, punch entries,
//!   kick entries`) is the jump's apex (`0x452e0`) with its tables in the
//!   record, since the 1987 routine hard-codes Ryu's tables, and their
//!   stream tables hold 16-bit stream addresses (`movea.w`), which cannot
//!   reach the bank: here they are longs. The landing centre is the
//!   anytime jump attacks' target (the 1987 code assumes Ryu's `0xb0`).
//!   [`ROUTINE_PROJECTILE_BY_STRENGTH`] (`kind, sound, 0, 0, template.l` for
//!   each strength level, light to heavy) is the projectile spawn with the
//!   button's strength choosing the template (Geki's Shuriken: his 1987
//!   throw `0x4596e` chooses one by distance, and its desperation volley
//!   reads the CPU's health). [`ROUTINE_TELEPORT`] (`kind, 0, gap.w`) moves
//!   the fighter, unseen in his smoke, to `gap` pixels in front of the
//!   opponent on his own side (Geki's Teleport Strike; his 1987 teleport
//!   `0x45a34` flies a random distance). [`ROUTINE_EFFECT_ROM_HIT`] (`kind,
//!   0, 0, 0, ROM handler.l, score entry.l`), named as a bank copy of a ROM
//!   projectile's +0x14 handler, runs the ROM handler and, when it lands the
//!   hit, scores the thrower with the entry's score byte (the 1987 CPU's
//!   projectiles never score; Geki's stars score as the Hadoken).
//! - The special slots after the table (above).
//! - Nothing else: headers, records, boxes and hit functions keep their 1987
//!   layout, and a boss's attacks may use the 1987 hit functions.
//!
//! **Poses and boxes.** A catalogue pose's anchor (the grid's top-left from
//! the fighter's centre and the ground line, facing right) becomes the
//! record's x-offset (`anchor x + half the grid width`) and, between poses,
//! the step that puts the new grid's top where the anchors say (the engine
//! keeps the grid's centre from frame to frame). Boxes the bank writes for a
//! new pose are rectangles in the drawing's own pixels ([`PixelBoxes`]),
//! turned into the 1987 bytes about the grid's centre; a ROM pose kept as it
//! is keeps its ROM boxes.

use sf_video::art::Catalogue;

use crate::era::EraFamily;
use crate::roster::TournamentFighter;

/// The first address of the bank.
pub const DATA_BANK_BASE: u32 = 0x10_0000;
/// The last address of the bank.
pub const DATA_BANK_END: u32 = 0x1f_ffff;
/// Each fighter's slot in the bank.
pub const FIGHTER_SPAN: u32 = 0x1_0000;
/// Move slots in an extended bank table (`0x00..=0x3f`).
pub const PLAYER_MOVES: u8 = 0x40;
/// Ryu's Shakunetsu Hadoken or Sagat's Tiger Uppercut in the SF2 kit.
pub const SF2_SPECIAL_SLOT: u8 = 0x1f;
/// The fighter's Super Combo in the SF2 kit.
pub const SF2_SUPER_SLOT: u8 = 0x29;
/// The Alpha shotos' second Super Combo (Metsu Shoryuken / Shinryuken).
pub const ALPHA_SUPER_SECOND_SLOT: u8 = 0x2a;
/// The Alpha shotos' third Super Combo (Shinkuu Tatsumaki / Shippu Jinraikyaku).
pub const ALPHA_SUPER_THIRD_SLOT: u8 = 0x2b;
/// The native Alpha guard-cancel attack. Reaction ids start at `0x30`.
pub const ALPHA_COUNTER_SLOT: u8 = 0x2e;
const MOVE_HEADER_SIZE: u32 = 0x30;
/// Where, from a bank table, the long naming its jump-apex record sits
/// (just after the table), for the anytime jump attacks.
pub const APEX_POINTER: u32 = PLAYER_MOVES as u32 * 4;
/// Where, from a bank table, the special slots' records sit, one for each
/// of `0x19`, `0x1a` and `0x1b` (see the module docs and [`SpecialSlot`]).
pub const SPECIALS: u32 = APEX_POINTER + 4;
/// The size of a special slot's record.
pub const SPECIAL_SLOT_SIZE: u32 = 8;
/// The special slots, the move ids a command tracker starts.
pub const SPECIAL_SLOTS: [u8; 3] = [0x19, 0x1a, 0x1b];
const MOVE_HEADERS: u32 = SPECIALS + SPECIAL_SLOTS.len() as u32 * SPECIAL_SLOT_SIZE;
const ERA_HEADERS: u32 = MOVE_HEADERS + PLAYER_MOVES as u32 * MOVE_HEADER_SIZE;
const ERA_HEADER_TABLES: u32 = 6;

fn era_header_offset(family: EraFamily, slot: u8) -> Option<u32> {
    let family_index = match family {
        EraFamily::Sf3 => 0,
        EraFamily::Sf4 => 1,
        EraFamily::Sf5 => 2,
        EraFamily::Sf6 => 3,
        EraFamily::SfZ => 4,
        EraFamily::Sf2 => 5,
        _ => return None,
    };
    (slot < PLAYER_MOVES).then_some(ERA_HEADERS + (family_index * u32::from(PLAYER_MOVES) + u32::from(slot)) * 4)
}

/// A special slot's flag: it lands by its own hit function (melee).
pub const SPECIAL_MELEE: u8 = 1;
/// A special slot's flag: guarded crouching even in the air.
pub const SPECIAL_LOW_IN_AIR: u8 = 2;
/// The command tracker offsets a special's command byte may name.
pub const TRACKERS: [u32; 3] = [0x16, 0x19, 0x1c];
/// The ground line: the bottom of every standing and crouching pose (a
/// standing large pose's header y is `0x88`, 96 pixels above it).
pub const GROUND: i32 = 0xe8;

/// A routine record that spawns a projectile (see the module docs).
pub const ROUTINE_PROJECTILE: u8 = 1;
/// A routine record for the jump's apex (see the module docs).
pub const ROUTINE_JUMP_APEX: u8 = 2;
/// A bank effect's hit on its target (`kind, flags, 0, 0, hit entry.l,
/// chip entry.l, impact template.l`): the Hadoken's hit (`0x433ba`) with
/// its damage and impact as parameters, named as an effect's +0x14 handler.
pub const ROUTINE_EFFECT_HIT: u8 = 3;
/// A bank effect meeting its target's attack box (`kind, 0, 0, 0, impact
/// template.l`, its +0x18 handler): the Hadoken's rule (`0x43490`), but it
/// breaks up into its own impact and leaves its thrower's hold alone.
pub const ROUTINE_EFFECT_VS_ATTACK: u8 = 4;
/// A bank effect meeting another projectile (`kind, 0, 0, 0, impact
/// template.l`, its +0x1c handler): as `0x43476`, both end, this one
/// breaking up into its own impact.
pub const ROUTINE_EFFECT_VS_PROJECTILE: u8 = 5;
/// A routine record that spawns a projectile chosen by the button's
/// strength (see the module docs).
pub const ROUTINE_PROJECTILE_BY_STRENGTH: u8 = 6;
/// A routine record that moves the fighter in front of the opponent (see
/// the module docs).
pub const ROUTINE_TELEPORT: u8 = 7;
/// A bank copy of a ROM projectile's hit handler that scores (see the
/// module docs).
pub const ROUTINE_EFFECT_ROM_HIT: u8 = 8;
/// A routine record in a victim's `"thrown"` stream just before his `lying`
/// pose loads: plays the knockdown thud, awards throw score, deals
/// [`crate::combo::throw::DAMAGE`], and marks a knock-out when the victim's
/// health reaches zero.
pub const ROUTINE_THROW_SLAM: u8 = 9;
/// A routine record in a victim's `"thrown"` stream after his `lying` hold:
/// ends the stream on a knock-out so the victim stays on the mat, or switches
/// the move id to [`crate::combo::throw::GET_UP_MOVE`] for his `"get-up"`
/// poses.
pub const ROUTINE_THROW_GET_UP: u8 = 10;
/// A routine record in a victim's `"thrown"` stream just after his `grabbed`
/// hold (once the 7-frame tech window has closed): turns participants for a
/// crossing throw while preserving their origins; the following airborne
/// steps carry the victim onto the other side.
pub const ROUTINE_THROW_ARC: u8 = 11;
/// A pose-origin conversion immediately before the next frame loads:
/// kind, sound, signed vertical offset. It never presents the old pose shifted.
pub const ROUTINE_POSE_ORIGIN: u8 = 12;
/// A command grab attempt: +4 range in pixels, +5 throw damage.
pub const ROUTINE_ALPHA_CATCH: u8 = 13;
/// Replay the retreat stream at +4 until the fighter reaches the rear edge.
pub const ROUTINE_ALPHA_WALL_REBOUND: u8 = 14;
/// Continue Gekirou on a fresh kick press, or land via the stream at +4.
pub const ROUTINE_ALPHA_REPEAT_K: u8 = 15;
/// Replay the windup at +4 for at most +1 frames while punch is held.
pub const ROUTINE_ALPHA_HOLD_P: u8 = 16;
/// Sagat's Tiger Nexus follow-up, selected after hit or guard contact.
pub const ROUTINE_SF6_TIGER_NEXUS: u8 = 17;

mod alpha_england;
mod alpha_gen;
mod alpha_thailand;
mod sf3;
mod sf4;
mod sf5;
mod sf6;
mod what_if;
mod what_if_others;


/// Addresses of a fighter's `"throw"` and `"thrown"` (`0x30`-byte) move
/// headers in the extended data bank (SP19, `#263`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThrowEntry {
    pub throw_header: u32,
    pub thrown_header: u32,
    /// Post-grab records whose artwork already turns away from the original
    /// opponent (bit 0 is the first stream frame). Their drawing and anchors
    /// mirror opposite the logical heading. The grab/fallback are ordinary.
    pub back_turned_poses: u64,
}

/// What a special slot is (see the module docs): the command that fires it,
/// whether it is melee, whether it is guarded crouching in the air, and the
/// damage entry it scores with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpecialSlot {
    /// The command's kind: a tracker's player structure offset (`0x16`,
    /// `0x19`, `0x1c`).
    pub tracker: u32,
    /// A kick fires it, not a punch.
    pub kick: bool,
    pub melee: bool,
    pub low_in_air: bool,
    /// The score entry, 0 for none.
    pub score: u32,
}

impl SpecialSlot {
    /// Ryu's and Ken's slot `slot`, as 1987 has it: the Shoryuken (the
    /// Shoryuken's tracker and a punch, melee, guarded crouching in the
    /// air), the Hadoken (the Hadoken's tracker and a punch, a projectile)
    /// and the Tatsumaki (its tracker and a kick, chipping a guard, guarded
    /// crouching in the air). Their hits score by their own hit functions.
    pub const fn ryu(slot: u8) -> SpecialSlot {
        match slot {
            0x19 => SpecialSlot { tracker: 0x19, kick: false, melee: true, low_in_air: true, score: 0 },
            0x1a => SpecialSlot { tracker: 0x1c, kick: false, melee: false, low_in_air: false, score: 0 },
            _ => SpecialSlot { tracker: 0x16, kick: true, melee: false, low_in_air: true, score: 0 },
        }
    }

    /// The record's bytes.
    pub fn bytes(&self) -> [u8; SPECIAL_SLOT_SIZE as usize] {
        let mut b = [0u8; SPECIAL_SLOT_SIZE as usize];
        b[0] = self.tracker as u8 | if self.kick { 0x80 } else { 0 };
        b[1] = if self.melee { SPECIAL_MELEE } else { 0 } | if self.low_in_air { SPECIAL_LOW_IN_AIR } else { 0 };
        b[4..8].copy_from_slice(&self.score.to_be_bytes());
        b
    }
}

/// Special slot `slot` (`0x19`, `0x1a` or `0x1b`) of the move table at
/// `table`, read through `rb` (the bus, or a read-only view of it): a bank
/// table's record, or for a ROM table the 1987 slot ([`SpecialSlot::ryu`]).
pub fn special_slot(rb: impl Fn(u32) -> u8, table: u32, slot: u8) -> SpecialSlot {
    if !in_bank(table) {
        return SpecialSlot::ryu(slot);
    }
    let a = table.wrapping_add(SPECIALS + slot.wrapping_sub(0x19) as u32 * SPECIAL_SLOT_SIZE);
    let (command, flags) = (rb(a), rb(a + 1));
    SpecialSlot {
        tracker: (command & 0x7f) as u32,
        kick: command & 0x80 != 0,
        melee: flags & SPECIAL_MELEE != 0,
        low_in_air: flags & SPECIAL_LOW_IN_AIR != 0,
        score: u32::from_be_bytes([rb(a + 4), rb(a + 5), rb(a + 6), rb(a + 7)]),
    }
}

/// The command of special slot `slot` of the table at `table`: its tracker
/// and whether a kick fires it ([`special_slot`]).
pub fn special_command(rb: impl Fn(u32) -> u8, table: u32, slot: u8) -> (u32, bool) {
    let s = special_slot(rb, table, slot);
    (s.tracker, s.kick)
}

/// Check each present special's command kind, flags and tracker/button pair.
fn validate_specials(region: &Region) -> Result<(), String> {
    let rb = |addr: u32| region.bytes.get((addr - region.base) as usize).copied().unwrap_or(0);
    let rl = |addr| u32::from_be_bytes(std::array::from_fn(|i| rb(addr + i as u32)));
    let mut taken: Vec<(u8, (u8, bool))> = Vec::new();
    for slot in SPECIAL_SLOTS {
        let Some(header) = move_header(rl, region.base, slot) else {
            continue;
        };
        if rb(header + 8) == 0 {
            continue;
        }
        let a = region.base + SPECIALS + (slot - 0x19) as u32 * SPECIAL_SLOT_SIZE;
        let (command, flags) = (rb(a), rb(a + 1));
        if !TRACKERS.contains(&((command & 0x7f) as u32)) {
            return Err(format!("special {slot:#04x} has no command it knows (command byte {command:#04x})"));
        }
        if flags & !(SPECIAL_MELEE | SPECIAL_LOW_IN_AIR) != 0 {
            return Err(format!("special {slot:#04x} has flags it does not know ({flags:#04x})"));
        }
        let key = (command & 0x7f, command & 0x80 != 0);
        if let Some((other, _)) = taken.iter().find(|(_, k)| *k == key) {
            return Err(format!(
                "specials {other:#04x} and {slot:#04x} are on the same command (tracker {:#04x}, {})",
                key.0,
                if key.1 { "kick" } else { "punch" }
            ));
        }
        taken.push((slot, key));
    }
    Ok(())
}

/// Whether `addr` falls in the bank.
pub const fn in_bank(addr: u32) -> bool {
    let a = addr & 0xff_ffff;
    a >= DATA_BANK_BASE && a <= DATA_BANK_END
}

/// Find a move's 1987-format header. ROM tables store headers inline.
/// The bank stores pointers to headers within the fighter's own slot.
pub fn move_header(read_long: impl Fn(u32) -> u32, table: u32, id: u8) -> Option<u32> {
    if !in_bank(table) {
        return Some(table.wrapping_add(u32::from(id) * MOVE_HEADER_SIZE));
    }
    if id >= PLAYER_MOVES {
        return None;
    }
    let table = table & 0xff_ffff;
    let header = read_long(table + u32::from(id) * 4);
    let offset = header.checked_sub(table)?;
    (MOVE_HEADERS..=FIGHTER_SPAN - MOVE_HEADER_SIZE).contains(&offset).then_some(header)
}

/// The fighter whose slot holds `table`, for a bank address.
pub fn fighter_of_table(table: u32) -> Option<TournamentFighter> {
    if !in_bank(table) {
        return None;
    }
    let index = ((table & 0xff_ffff) - DATA_BANK_BASE) / FIGHTER_SPAN;
    TournamentFighter::ALL.get(index as usize).copied()
}

/// Where `fighter`'s slot starts: his move table.
pub fn table_address(fighter: TournamentFighter) -> u32 {
    let index = TournamentFighter::ALL.iter().position(|&f| f == fighter).unwrap_or(0) as u32;
    DATA_BANK_BASE + index * FIGHTER_SPAN
}

/// The bank: bytes from [`DATA_BANK_BASE`], and which fighters have a table.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DataBank {
    bytes: Vec<u8>,
    tables: Vec<TournamentFighter>,
    throws: [Option<ThrowEntry>; 12],
    /// Lines for the game's log: what was built, and what was not and why.
    notes: Vec<String>,
}

impl DataBank {
    /// Immutable game data and table bindings used by online compatibility.
    /// Build notes are local diagnostics and do not identify the bank.
    pub(crate) fn encode_compatibility(&self, out: &mut crate::canonical::CanonicalHasher) {
        use crate::canonical::Canonical;
        let Self { bytes, tables, throws, notes: _ } = self;
        out.tag("data-bank-compatibility/v1");
        bytes.encode(out);
        tables.encode(out);
        for entry in throws {
            match entry {
                None => 0u8.encode(out),
                Some(ThrowEntry { throw_header, thrown_header, back_turned_poses }) => {
                    1u8.encode(out);
                    throw_header.encode(out);
                    thrown_header.encode(out);
                    back_turned_poses.encode(out);
                }
            }
        }
    }

    /// The byte at `addr`; `0xff` outside what the bank holds, as the bus
    /// reads unmapped space.
    pub fn rb(&self, addr: u32) -> u8 {
        let a = addr & 0xff_ffff;
        if !in_bank(a) {
            return 0xff;
        }
        self.bytes.get((a - DATA_BANK_BASE) as usize).copied().unwrap_or(0xff)
    }

    /// A bank of these bytes from [`DATA_BANK_BASE`], with no tables (for
    /// the engine's unit tests of bank records).
    #[cfg(test)]
    pub(crate) fn from_bytes(bytes: Vec<u8>) -> DataBank {
        DataBank { bytes, ..DataBank::default() }
    }

    /// `fighter`'s move table, when the bank holds one.
    pub fn table(&self, fighter: TournamentFighter) -> Option<u32> {
        self.tables.contains(&fighter).then(|| table_address(fighter))
    }

    /// Find an authored bank header, including era moves for Ryu and Ken.
    pub fn move_header(&self, fighter: TournamentFighter, id: u8) -> Option<u32> {
        move_header(
            |addr| u32::from_be_bytes(std::array::from_fn(|i| self.rb(addr + i as u32))),
            table_address(fighter),
            id,
        )
    }

    /// Resolve a family-specific header without replacing the SF1+ or Alpha slot.
    pub fn era_move_header(&self, fighter: TournamentFighter, family: EraFamily, slot: u8) -> Option<u32> {
        let table = table_address(fighter);
        let pointer = table + era_header_offset(family, slot)?;
        let header = u32::from_be_bytes(std::array::from_fn(|byte| self.rb(pointer + byte as u32)));
        let offset = header.checked_sub(table)?;
        (ERA_HEADERS..=FIGHTER_SPAN - MOVE_HEADER_SIZE).contains(&offset).then_some(header)
    }

    pub(crate) fn alpha_move_header(&self, fighter: TournamentFighter, slot: u8) -> Option<u32> {
        self.era_move_header(fighter, EraFamily::SfZ, slot)
            .or_else(|| self.move_header(fighter, slot))
    }

    /// `fighter`'s `"throw"` and `"thrown"` headers in the bank (SP19, `#263`).
    pub fn throw_entry(&self, fighter: TournamentFighter) -> Option<ThrowEntry> {
        let idx = TournamentFighter::ALL.iter().position(|&f| f == fighter)?;
        self.throws[idx]
    }

    /// The fighters the bank holds tables for.
    pub fn fighters(&self) -> &[TournamentFighter] {
        &self.tables
    }

    pub fn is_empty(&self) -> bool {
        self.tables.is_empty()
    }

    /// What was built and why anything was not, for the game's log.
    pub fn notes(&self) -> &[String] {
        &self.notes
    }

    /// Build the bank from the program ROM (`maincpu`, whose 1987 streams
    /// and boxes it reuses) and the fighters' pose catalogues (their poses'
    /// places in the extended sprite bank). A fighter gets a table only when
    /// his catalogue is loaded with every pose his moves need; the others
    /// are listed in [`Self::notes`].
    pub fn build(rom: &[u8], catalogues: &[Catalogue]) -> DataBank {
        let mut bank = DataBank::default();
        let put_slot = |bank: &mut DataBank, fighter: TournamentFighter, bytes: &[u8]| {
            let at = (table_address(fighter) - DATA_BANK_BASE) as usize;
            if bank.bytes.len() < at {
                bank.bytes.resize(at, 0xff);
            }
            bank.bytes.truncate(at);
            bank.bytes.extend_from_slice(bytes);
        };
        for (idx, fighter) in TournamentFighter::ALL.into_iter().enumerate() {
            let slug = fighter.name().to_ascii_lowercase();
            let catalogue = catalogues.iter().find(|c| c.fighter == slug);
            let Some(builder) = fighter_builder(fighter) else {
                let mut region = new_slot(fighter);
                if rom.len() >= 0x40000 {
                    build_shoto_sf2_moves(rom, &mut region, fighter);
                    write_alpha_counter(rom, &mut region, fighter);
                    write_alpha_custom(rom, &mut region, fighter);
                    build_shoto_alpha_supers(rom, &mut region, fighter);
                    write_alpha_shoto_overhead(rom, &mut region, fighter);
                    sf3::build(rom, &mut region, fighter);
                    sf4::build(rom, &mut region, fighter);
                    sf5::build(rom, &mut region, fighter);
                    sf6::build(rom, &mut region, fighter);
                }
                let entry = match catalogue {
                    Some(cat) => build_throws(rom, &mut region, fighter, cat).ok(),
                    None if rom.len() >= 0x40000 => Some(build_shoto_rom_throws(rom, &mut region, fighter)),
                    None => None,
                };
                if rom.len() >= 0x40000 || entry.is_some() {
                    match fits(region) {
                        Ok(region) => {
                            put_slot(&mut bank, fighter, &region.bytes);
                            bank.throws[idx] = entry;
                        }
                        Err(why) => bank.notes.push(format!("{} has no extended moves: {why}", fighter.name())),
                    }
                }
                continue;
            };
            let Some(catalogue) = catalogue else {
                bank.notes.push(format!("{} has no 1987 player table: his poses are not loaded", fighter.name()));
                continue;
            };
            let built = builder(rom, catalogue)
                .and_then(|region| validate_specials(&region).map(|()| region))
                .and_then(|mut region| {
                    let entry = build_throws(rom, &mut region, fighter, catalogue)?;
                    write_alpha_counter(rom, &mut region, fighter);
                    write_alpha_custom(rom, &mut region, fighter);
                    match fighter {
                        TournamentFighter::Adon => alpha_thailand::write_adon(rom, &mut region),
                        TournamentFighter::Birdie => alpha_england::build_birdie(rom, &mut region),
                        TournamentFighter::Eagle => alpha_england::build_eagle(rom, &mut region, catalogue)?,
                        TournamentFighter::Gen => alpha_gen::build(rom, &mut region),
                        TournamentFighter::Sagat => alpha_thailand::write_sagat(rom, &mut region),
                        _ => {}
                    }
                    sf4::build(rom, &mut region, fighter);
                    sf5::build(rom, &mut region, fighter);
                    sf6::build(rom, &mut region, fighter);
                    what_if::build(rom, &mut region, fighter);
                    what_if_others::build(rom, &mut region, fighter, catalogue)?;
                    what_if::build_systems(rom, &mut region, fighter);
                    fits(region).map(|region| (region, entry))
                });
            match built {
                Ok((region, entry)) => {
                    put_slot(&mut bank, fighter, &region.bytes);
                    bank.notes.push(format!(
                        "{} plays a 1987 player table ({} bytes of the extended data bank)",
                        fighter.name(),
                        region.bytes.len()
                    ));
                    bank.tables.push(fighter);
                    bank.throws[idx] = Some(entry);
                }
                Err(why) => bank.notes.push(format!("{} has no 1987 player table: {why}", fighter.name())),
            }
        }
        bank
    }
}

/// The builder of each fighter whose table the bank has so far.
type Builder = fn(&[u8], &Catalogue) -> Result<Region, String>;

fn fighter_builder(fighter: TournamentFighter) -> Option<Builder> {
    match fighter {
        TournamentFighter::Retsu => Some(retsu::build),
        TournamentFighter::Geki => Some(geki::build),
        TournamentFighter::Joe => Some(joe::build),
        TournamentFighter::Mike => Some(mike::build),
        TournamentFighter::Lee => Some(lee::build),
        TournamentFighter::Gen => Some(gen::build),
        TournamentFighter::Birdie => Some(birdie::build),
        TournamentFighter::Eagle => Some(eagle::build),
        TournamentFighter::Adon => Some(adon::build),
        TournamentFighter::Sagat => Some(sagat::build),
        _ => None,
    }
}

// ---- reading the ROM ---------------------------------------------------------

fn rom_b(rom: &[u8], a: u32) -> u8 {
    rom.get(a as usize).copied().unwrap_or(0xff)
}

fn rom_w(rom: &[u8], a: u32) -> u16 {
    u16::from_be_bytes([rom_b(rom, a), rom_b(rom, a + 1)])
}

fn rom_l(rom: &[u8], a: u32) -> u32 {
    (rom_w(rom, a) as u32) << 16 | rom_w(rom, a + 2) as u32
}

/// A 1987 frame record (`0x20` bytes) as the ROM holds it.
fn rom_record(rom: &[u8], a: u32) -> [u8; 0x20] {
    std::array::from_fn(|i| rom_b(rom, a + i as u32))
}

// ---- writing the bank ----------------------------------------------------------

/// One fighter's slot as it is written: bytes from `base`.
#[derive(Clone, Debug)]
pub(crate) struct Region {
    base: u32,
    bytes: Vec<u8>,
}

impl Region {
    fn new(base: u32) -> Self {
        Region { base, bytes: Vec::new() }
    }

    /// The address the next byte goes to.
    fn here(&self) -> u32 {
        self.base + self.bytes.len() as u32
    }

    fn put(&mut self, bytes: &[u8]) -> u32 {
        let at = self.here();
        self.bytes.extend_from_slice(bytes);
        at
    }

    fn put_l(&mut self, v: u32) -> u32 {
        self.put(&v.to_be_bytes())
    }

    fn patch(&mut self, at: u32, bytes: &[u8]) {
        let i = (at - self.base) as usize;
        self.bytes[i..i + bytes.len()].copy_from_slice(bytes);
    }

    /// A stream's `0xff` entry calling `routine` with `arg`.
    fn call(&mut self, routine: u32, arg: u8) -> u32 {
        let at = self.put(&[0xff, arg]);
        self.put_l(routine);
        at
    }

    /// A stream's end (`0xfd`).
    fn end(&mut self) -> u32 {
        self.put(&[0xfd])
    }
}

/// A pose's tile grid and anchor, as the frame records place it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PoseGeom {
    pub code: u16,
    /// The palette, with `0x400` for 32-pixel entries.
    pub attr: u16,
    pub rows: u8,
    pub cols: u8,
    /// The grid's top-left from the fighter's centre and the ground line,
    /// facing right.
    pub anchor: [i16; 2],
}

impl PoseGeom {
    fn cell(&self) -> i32 {
        if self.attr & 0x400 != 0 {
            32
        } else {
            16
        }
    }

    pub fn width(&self) -> i32 {
        (self.cols as i32 + 1) * self.cell()
    }

    pub fn height(&self) -> i32 {
        (self.rows as i32 + 1) * self.cell()
    }

    /// Half the grid's width and height (`0x43be8` / `0x43bda`).
    pub fn half(&self) -> (i32, i32) {
        (self.width() / 2, self.height() / 2)
    }

    /// The record's x-offset: the grid's left edge from the fighter's centre
    /// is the anchor's x, and the engine draws it at the centre less half
    /// the grid plus the x-offset.
    pub fn x_offset(&self) -> i8 {
        (self.anchor[0] as i32 + self.half().0) as i8
    }

    /// The grid's top on screen when the fighter stands on the ground.
    pub fn top_on_ground(&self) -> i32 {
        GROUND + self.anchor[1] as i32
    }
}

/// Boxes as rectangles in a pose's own pixels (x right, y down from the
/// grid's top-left, facing right; each `[x0, y0, x1, y1)`), turned into the
/// 1987 bytes about the grid's centre by [`PixelBoxes::bytes`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelBoxes {
    /// The body the fighters push each other apart by (`+0x16..+0x19`): its
    /// extents above, below, in front of and behind the grid's centre, so
    /// it must reach the centre.
    pub push: [i32; 4],
    /// The head (`+0x1a..+0x1d`): a hit that reaches it is the deeper hit.
    pub head: [i32; 4],
    /// The body that can be hit (`+0x1e..+0x21`).
    pub body: [i32; 4],
    /// The attack (`+0x22..+0x25`), on its active frames.
    pub attack: Option<[i32; 4]>,
}

impl PixelBoxes {
    /// The record's sixteen box bytes for `pose`'s grid.
    pub fn bytes(&self, pose: &PoseGeom) -> [u8; 16] {
        let (hx, hy) = pose.half();
        // [y offset up from the centre, half height, x offset, half width].
        let centred = |[x0, y0, x1, y1]: [i32; 4]| -> [u8; 4] {
            let (cx, cy) = ((x0 + x1) / 2, (y0 + y1) / 2);
            [(hy - cy) as u8, ((y1 - y0) / 2) as u8, (cx - hx) as u8, ((x1 - x0) / 2) as u8]
        };
        let [x0, y0, x1, y1] = self.push;
        // [above the centre, below it, in front, behind].
        let push = [(hy - y0) as u8, (y1 - hy) as u8, (x1 - hx) as u8, (hx - x0) as u8];
        let head = centred(self.head);
        let body = centred(self.body);
        let attack = self.attack.map(centred).unwrap_or([0; 4]);
        let mut out = [0u8; 16];
        out[0..4].copy_from_slice(&push);
        out[4..8].copy_from_slice(&head);
        out[8..12].copy_from_slice(&body);
        out[12..16].copy_from_slice(&attack);
        out
    }
}

/// One frame record to write.
#[derive(Clone, Copy, Debug)]
struct Rec {
    hold: u8,
    dur: u8,
    pose: PoseGeom,
    dy: i8,
    dx: i8,
    boxes: [u8; 16],
    flags: u8,
    x_offset: i8,
    hit: u32,
}

impl Rec {
    fn bytes(&self) -> [u8; 0x20] {
        let mut r = [0u8; 0x20];
        r[0] = self.hold;
        r[1] = self.dur;
        r[2..4].copy_from_slice(&self.pose.code.to_be_bytes());
        r[4..6].copy_from_slice(&self.pose.attr.to_be_bytes());
        r[6] = self.dy as u8;
        r[7] = self.dx as u8;
        r[8] = self.pose.rows;
        r[9] = self.pose.cols;
        r[0xa..0x1a].copy_from_slice(&self.boxes);
        r[0x1a] = self.flags;
        r[0x1b] = self.x_offset as u8;
        r[0x1c..0x20].copy_from_slice(&self.hit.to_be_bytes());
        r
    }
}

/// A move header (`0x30` bytes) from its first frame: the frame record's
/// fields where the header keeps them, the grid's top on screen (`+4`, the
/// header places the grid absolutely), its x (`+6`: the x-offset and the
/// frame's step, as a stream's load would place it), the first-frame flag,
/// the state and the move id, its three sounds and its stream.
fn header(rec: &[u8; 0x20], top: i32, state: u8, id: u8, sounds: [u8; 3], stream: u32) -> [u8; 0x30] {
    let mut h = [0u8; 0x30];
    h[0..2].copy_from_slice(&rec[2..4]);
    h[2..4].copy_from_slice(&rec[4..6]);
    h[4..6].copy_from_slice(&(top as u16).to_be_bytes());
    let x = rec[0x1b] as i8 as i16 + rec[7] as i8 as i16;
    h[6..8].copy_from_slice(&x.to_be_bytes());
    h[8] = 1;
    h[9] = state;
    h[0xa] = id;
    h[0xb] = rec[0];
    h[0xc] = rec[8];
    h[0xd] = rec[9];
    h[0xe] = rec[1];
    h[0xf] = rec[0x1a];
    h[0x10..0x13].copy_from_slice(&sounds);
    h[0x14] = rec[6];
    h[0x15] = rec[7];
    h[0x16..0x26].copy_from_slice(&rec[0xa..0x1a]);
    h[0x26] = rec[0x1b];
    h[0x28..0x2c].copy_from_slice(&rec[0x1c..0x20]);
    h[0x2c..0x30].copy_from_slice(&stream.to_be_bytes());
    h
}

/// Writes a stream while keeping track of where the grid's top is, so each
/// pose lands where its anchor says (the engine keeps the grid's centre
/// between frames: a new grid's top is the old centre less half the new
/// height, plus the step).
struct Stream<'r> {
    region: &'r mut Region,
    /// The grid's top on screen, and the current pose.
    top: i32,
    pose: PoseGeom,
}

fn pose_origin_record(region: &mut Region, dy: i32, sound: u8) -> u32 {
    let dy = i16::try_from(dy).expect("a pose-origin conversion fits a word");
    let [hi, lo] = dy.to_be_bytes();
    region.put(&[ROUTINE_POSE_ORIGIN, sound, hi, lo])
}

/// Keep the same height above the standing line when changing sprite grids.
fn ground_origin_record(region: &mut Region, from: PoseGeom, to: PoseGeom) -> (u32, i32) {
    let dy = (to.top_on_ground() + to.half().1) - (from.top_on_ground() + from.half().1);
    (pose_origin_record(region, dy, 0), dy)
}

impl<'r> Stream<'r> {
    fn new(region: &'r mut Region, pose: PoseGeom, top: i32) -> Self {
        Stream { region, top, pose }
    }

    /// The step that places `pose` with its top at `top` after the current
    /// pose (before any movement).
    fn offset_to(&self, pose: &PoseGeom, top: i32) -> i32 {
        top - (self.top + self.pose.half().1 - pose.half().1)
    }

    /// `pose` held still for `frames` frames, its top at `top` (one record:
    /// the duration holds it, nothing repeats).
    fn hold(&mut self, pose: PoseGeom, top: i32, frames: u8, boxes: [u8; 16], flags: u8, hit: u32) -> u32 {
        let dy = self.offset_to(&pose, top);
        let rec = Rec { hold: 0, dur: frames - 1, pose, dy: dy as i8, dx: 0, boxes, flags, x_offset: pose.x_offset(), hit };
        self.top = top;
        self.pose = pose;
        self.region.put(&rec.bytes())
    }

    /// `pose` moving `(dy, dx)` a frame for `frames` frames, its top at
    /// `top` before the first frame's movement. One record repeating the
    /// step (the 1987 hold count) when the engine's kept centre already
    /// places the pose there; otherwise a record whose load places it (and
    /// moves once), then one repeating the step for the rest. An attack's
    /// active frames must be one record (a hit clears the active bit until
    /// the next record loads), so [`Self::strike`] asks for no placement
    /// step. Returns the first record's address.
    fn travel(&mut self, pose: PoseGeom, top: i32, frames: u8, (dy, dx): (i32, i32), boxes: [u8; 16], flags: u8, hit: u32) -> u32 {
        let offset = self.offset_to(&pose, top);
        let rec = Rec {
            hold: frames - 1,
            dur: 0,
            pose,
            dy: dy as i8,
            dx: dx as i8,
            boxes,
            flags,
            x_offset: pose.x_offset(),
            hit,
        };
        let at = if offset == 0 {
            self.region.put(&rec.bytes())
        } else {
            let at = self.region.put(&Rec { hold: 0, dy: (offset + dy) as i8, ..rec }.bytes());
            if frames > 1 {
                self.region.put(&Rec { hold: frames - 2, ..rec }.bytes());
            }
            at
        };
        self.top = top + dy * frames as i32;
        self.pose = pose;
        at
    }

    /// An attack's active frames in the air: as [`Self::travel`], placed
    /// where the kept centre puts it (one record).
    fn strike(&mut self, pose: PoseGeom, frames: u8, step: (i32, i32), boxes: [u8; 16], flags: u8, hit: u32) -> u32 {
        let top = self.top + self.pose.half().1 - pose.half().1;
        self.travel(pose, top, frames, step, boxes, flags, hit)
    }

    fn call(&mut self, routine: u32, arg: u8) -> u32 {
        self.region.call(routine, arg)
    }

    /// Convert the kept centre for the next pose without spending a visible
    /// frame moving the preceding drawing. The named bank routine carries
    /// the same offset and optional strike sound.
    fn change_origin(&mut self, routine: u32, dy: i32) {
        self.call(routine, 0);
        self.top += dy;
    }

    fn end(&mut self) -> u32 {
        self.region.end()
    }
}

/// The routines of the 1987 program the bank's streams call.
mod routine {
    /// The posture bits at a jump's launch (`0x45a7a`).
    pub const RISING: u32 = 0x45a7a;
    /// The posture bits past a jump's apex (`0x45ab2`).
    pub const FALLING: u32 = 0x45ab2;
    /// The posture bits when a jump returns to the ground (`0x45a96`).
    pub const TOUCHDOWN: u32 = 0x45a96;
    /// The landing sound (header `+0x12`).
    pub const LANDING: u32 = 0x454ee;
    /// The sound in the entry's second byte.
    pub const SOUND: u32 = 0x455de;
    /// The CPU's follow-up: the end of every boss stream in the ROM.
    pub const CPU_FOLLOW_UP: u32 = 0x45630;
}

/// The 1987 hit functions the bank's new moves use: Ryu's (his P1 copies),
/// which take the damage by the button's strength.
mod hit {
    pub const CROUCHING_PUNCH: u32 = 0x45d26;
    pub const CROUCHING_KICK: u32 = 0x46250;
    pub const JUMP_PUNCH: u32 = 0x45e10;
    pub const JUMP_KICK: u32 = 0x4638e;
    pub const LIGHT_PUNCH: u32 = 0x45af0;
    pub const LIGHT_KICK: u32 = 0x4602a;
    pub const SWEEP: u32 = 0x46300;
}

/// How [`copy_rom_stream`] changes the stream it copies.
#[derive(Clone, Copy, Default)]
struct CopyHow<'a> {
    /// A call entry (`(before, routine)`) inserted just before the ROM entry
    /// at `before`: the jump's apex.
    apex: Option<(u32, u32)>,
    /// The ROM entry the copy ends before (a bank stream's end there).
    stop: Option<u32>,
    /// ROM routines whose calls call a bank routine record instead.
    replace: &'a [(u32, u32)],
}

/// Copy a ROM stream from `start` into the region until its end, changed as
/// `how` says; the CPU follow-up and the end become the bank stream's end.
/// When copying a jump (`how.apex` is set) whose ROM stream omits the
/// landing call (`0x454ee`), insert it after the landing crouch that follows
/// `0x45ab2` so anytime jump attacks know where the jump touches down.
/// Returns the copy's address.
fn copy_rom_stream(region: &mut Region, rom: &[u8], start: u32, how: CopyHow) -> u32 {
    let at = region.here();
    let has_landing = {
        let mut scan = start;
        let mut found = false;
        for _ in 0..64 {
            match rom_b(rom, scan) {
                0xff => {
                    let r = rom_l(rom, scan + 2);
                    if r == routine::LANDING {
                        found = true;
                        break;
                    }
                    if r == routine::CPU_FOLLOW_UP {
                        break;
                    }
                    scan += 6;
                }
                0xfd => break,
                0xfe => scan = rom_l(rom, scan + 2),
                _ => scan += 0x20,
            }
        }
        found
    };
    let mut need_landing_after_next_frame = false;
    let mut a = start;
    for _ in 0..64 {
        if how.stop == Some(a) {
            region.end();
            return at;
        }
        if let Some((before, routine)) = how.apex {
            if a == before {
                region.call(routine, 0xff);
            }
        }
        match rom_b(rom, a) {
            0xff => {
                let r = rom_l(rom, a + 2);
                if r == routine::CPU_FOLLOW_UP {
                    region.end();
                    return at;
                }
                let r = how.replace.iter().find(|(rom_routine, _)| *rom_routine == r).map_or(r, |&(_, bank)| bank);
                region.call(r, rom_b(rom, a + 1));
                if how.apex.is_some() && !has_landing && r == routine::FALLING {
                    need_landing_after_next_frame = true;
                }
                a += 6;
            }
            0xfd => {
                region.end();
                return at;
            }
            0xfe => a = rom_l(rom, a + 2),
            _ => {
                region.put(&rom_record(rom, a));
                if need_landing_after_next_frame {
                    region.call(routine::LANDING, 0);
                    need_landing_after_next_frame = false;
                }
                a += 0x20;
            }
        }
    }
    region.end();
    at
}

// ---- what every fighter's table is built from ---------------------------------

/// A ROM frame's pose, with the anchor its x-offset gives (its height on the
/// ground taken from the grid: a standing pose's bottom on the ground line).
fn rom_pose(rom: &[u8], frame: u32) -> PoseGeom {
    let r = rom_record(rom, frame);
    let mut p = PoseGeom {
        code: u16::from_be_bytes([r[2], r[3]]),
        attr: u16::from_be_bytes([r[4], r[5]]),
        rows: r[8],
        cols: r[9],
        anchor: [0, 0],
    };
    let (hx, _) = p.half();
    p.anchor = [(r[0x1b] as i8 as i32 - hx) as i16, -(p.height() as i16)];
    p
}

/// A ROM frame's sixteen box bytes.
fn rom_boxes(rom: &[u8], frame: u32) -> [u8; 16] {
    let r = rom_record(rom, frame);
    std::array::from_fn(|i| r[0xa + i])
}

/// A ROM move header's pose and boxes: the header places its grid
/// absolutely (`+4` its top, `+0x26` its x-offset).
fn header_pose(rom: &[u8], h: u32) -> (PoseGeom, [u8; 16]) {
    let mut p = PoseGeom {
        code: rom_w(rom, h),
        attr: rom_w(rom, h + 2),
        rows: rom_b(rom, h + 0xc),
        cols: rom_b(rom, h + 0xd),
        anchor: [0, 0],
    };
    let (hx, _) = p.half();
    p.anchor = [(rom_b(rom, h + 0x26) as i8 as i32 - hx) as i16, (rom_w(rom, h + 4) as i32 - GROUND) as i16];
    (p, std::array::from_fn(|i| rom_b(rom, h + 0x16 + i as u32)))
}

/// A catalogue pose's geometry, in `palette`.
fn catalogue_pose(catalogue: &Catalogue, name: &str, palette: u16) -> Result<PoseGeom, String> {
    let p = catalogue.pose(name).ok_or_else(|| format!("his catalogue has no {name}"))?;
    Ok(PoseGeom {
        code: p.grid.code,
        attr: palette | if p.grid.large { 0x400 } else { 0 },
        rows: p.grid.rows,
        cols: p.grid.cols,
        anchor: p.anchor,
    })
}

/// The box set named `name` in a fighter's list.
fn boxes_named(list: &[(&str, PixelBoxes)], name: &str) -> PixelBoxes {
    list.iter().find(|(n, _)| *n == name).map(|(_, b)| *b).expect("a box set for every new pose")
}

/// A record holding `pose` still for `frames` frames (the duration holds
/// it), placed where the engine's kept centre puts it.
fn still(pose: PoseGeom, frames: u8, boxes: [u8; 16]) -> Rec {
    Rec { hold: 0, dur: frames - 1, pose, dy: 0, dx: 0, boxes, flags: 0, x_offset: pose.x_offset(), hit: 0 }
}

const fn bank_header_offset(id: u8) -> u32 {
    MOVE_HEADERS + id as u32 * MOVE_HEADER_SIZE
}

/// Write a move header and bind its slot in the table.
fn put_header(region: &mut Region, base: u32, id: u8, h: &[u8]) {
    assert!(id < PLAYER_MOVES);
    let address = base + bank_header_offset(id);
    region.patch(base + u32::from(id) * 4, &address.to_be_bytes());
    region.patch(address, h);
}

fn put_era_header(region: &mut Region, family: EraFamily, slot: u8, header: &[u8; 0x30]) {
    let offset = era_header_offset(family, slot).expect("extended era move slot");
    let address = region.put(header);
    region.patch(region.base + offset, &address.to_be_bytes());
}

/// Share a stream with a base or Alpha move while retaining the new move ID.
fn alias_era_header(region: &mut Region, family: EraFamily, slot: u8, existing_slot: u8) {
    let pointer = usize::from(existing_slot) * 4;
    if region.bytes[pointer..pointer + 4] == [0; 4] {
        return;
    }
    let start = bank_header_offset(existing_slot) as usize;
    let mut header: [u8; 0x30] = region.bytes[start..start + 0x30].try_into().unwrap();
    header[0x0a] = slot;
    put_era_header(region, family, slot, &header);
}

/// Copy move `id`'s ROM header from `rom_table` into the table at `base`.
fn keep_rom_header(region: &mut Region, rom: &[u8], rom_table: u32, base: u32, id: u8) {
    let h: Vec<u8> = (0..0x30).map(|i| rom_b(rom, rom_table + id as u32 * 0x30 + i)).collect();
    put_header(region, base, id, &h);
}

/// Write a fighter's `"throw"` stream + header and `"thrown"` / `"get-up"`
/// stream + header into `region` (SP19, `#263`).
fn write_throw_pair(
    region: &mut Region,
    throw_poses: &[(PoseGeom, u8)],
    thrown_poses: [(PoseGeom, u8); 4],
    get_up_poses: &[(PoseGeom, u8)],
    back_turned_poses: u64,
) -> ThrowEntry {
    // Native hit-state clamping uses the pushbox's below-centre extent.
    // Each pose's catalogue anchor places the floor at a different distance
    // from that grid centre; a standing 48px extent lifts a lying pose 32px.
    let ground_boxes = |pose: PoseGeom, mut boxes: [u8; 16]| {
        let below = GROUND - (pose.top_on_ground() + pose.half().1);
        boxes[1] = u8::try_from(below).expect("a throw pose's ground extent fits a byte");
        boxes
    };
    let throw_boxes = |pose| ground_boxes(pose, [0x30, 0x30, 0x0c, 0x0c, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let getup_boxes = |pose| ground_boxes(pose, [0x30, 0x30, 0x10, 0x10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let (p0, d0) = throw_poses[0];
    let first_throw = Rec {
        hold: 0,
        dur: d0 - 1,
        pose: p0,
        dy: 0,
        dx: 0,
        boxes: throw_boxes(p0),
        flags: 0x08,
        x_offset: p0.x_offset(),
        hit: 0,
    }
    .bytes();
    let throw_stream = region.here();
    let mut s = Stream::new(region, p0, p0.top_on_ground());
    for &(p, dur) in &throw_poses[1..] {
        s.hold(p, p.top_on_ground(), dur, throw_boxes(p), 0x08, 0);
    }
    s.end();
    let throw_header = region.put(&header(
        &first_throw,
        p0.top_on_ground(),
        0xc2,
        crate::combo::throw::THROW_MOVE,
        [1, 0, 0],
        throw_stream,
    ));

    let arc = region.put(&[ROUTINE_THROW_ARC, 0]);
    let slam = region.put(&[ROUTINE_THROW_SLAM, 0]);
    let get_up_start = region.put(&[ROUTINE_THROW_GET_UP, 0]);
    let [(grabbed, g_dur), (thrown_1, t1_dur), (thrown_2, t2_dur), (lying, _)] = thrown_poses;
    let first_thrown = Rec {
        hold: 0,
        dur: g_dur - 1,
        pose: grabbed,
        dy: 0,
        dx: 0,
        boxes: throw_boxes(grabbed),
        flags: 0x08,
        x_offset: grabbed.x_offset(),
        hit: 0,
    }
    .bytes();
    let thrown_stream = region.here();
    let mut s = Stream::new(region, grabbed, grabbed.top_on_ground());
    s.call(arc, 0);
    s.travel(thrown_1, thrown_1.top_on_ground(), t1_dur, (0, -2), throw_boxes(thrown_1), 0x08, 0);
    s.travel(thrown_2, thrown_2.top_on_ground(), t2_dur, (0, -2), throw_boxes(thrown_2), 0x08, 0);
    s.call(slam, 0);
    s.hold(lying, lying.top_on_ground(), 18, throw_boxes(lying), 0x08, 0);
    s.call(get_up_start, 0);
    for &(p, dur) in get_up_poses {
        s.hold(p, p.top_on_ground(), dur, getup_boxes(p), 0x08, 0);
    }
    s.end();
    let thrown_header = region.put(&header(
        &first_thrown,
        grabbed.top_on_ground(),
        0x8a,
        crate::combo::throw::THROWN_MOVE,
        [0, 0, 0],
        thrown_stream,
    ));
    ThrowEntry {
        throw_header,
        thrown_header,
        back_turned_poses,
    }
}

/// Build `fighter`'s `"throw"` and `"thrown"` / `"get-up"` streams and headers
/// from his approved `catalogue` (SP19, `#263`).
fn build_throws(
    rom: &[u8],
    region: &mut Region,
    fighter: TournamentFighter,
    catalogue: &Catalogue,
) -> Result<ThrowEntry, String> {
    let palette = rom_w(rom, fighter.move_table() + 0x30 + 2) & 0x3ff;
    let find_move = |name: &str| {
        catalogue
            .moves
            .iter()
            .find(|m| m.name == name)
            .ok_or_else(|| format!("his catalogue has no {name} move"))
    };
    let throw_mv = find_move("throw")?;
    let thrown_mv = find_move("thrown")?;
    let get_up_mv = find_move("get-up")?;
    if throw_mv.poses.is_empty() || thrown_mv.poses.len() < 4 || get_up_mv.poses.len() < 3 {
        return Err("his throw moves have too few poses".to_owned());
    }
    let mut throw_poses = Vec::with_capacity(throw_mv.poses.len());
    let mut back_turned_poses = 0u64;
    for (index, (name, &dur)) in throw_mv.poses.iter().zip(&throw_mv.timing).enumerate() {
        throw_poses.push((catalogue_pose(catalogue, name, palette)?, dur as u8));
        // These specific approved pictures already contain the turn away
        // from the original opponent. A shared ROM pose keeps its ordinary
        // orientation when used outside this catalogue throw stream.
        let back_turned = match fighter {
            TournamentFighter::Ryu | TournamentFighter::Ken => {
                matches!(name.as_str(), "throw_turn" | "throw_heave")
            }
            TournamentFighter::Lee => {
                matches!(name.as_str(), "throw_turn" | "throw_heave" | "throw_recover")
            }
            _ => false,
        };
        if back_turned {
            let bit = index.checked_sub(1).expect("the grab has no authored back turn");
            back_turned_poses |= 1u64.checked_shl(bit as u32)
                .expect("an authored back turn fits the throw stream mask");
        }
    }
    let thrown_poses = [
        (catalogue_pose(catalogue, &thrown_mv.poses[0], palette)?, thrown_mv.timing[0] as u8),
        (catalogue_pose(catalogue, &thrown_mv.poses[1], palette)?, thrown_mv.timing[1] as u8),
        (catalogue_pose(catalogue, &thrown_mv.poses[2], palette)?, thrown_mv.timing[2] as u8),
        (catalogue_pose(catalogue, &thrown_mv.poses[3], palette)?, thrown_mv.timing[3] as u8),
    ];
    let mut get_up_poses = Vec::new();
    for (name, &dur) in get_up_mv
        .poses
        .iter()
        .zip(&get_up_mv.timing)
        .skip(1)
        .take(get_up_mv.poses.len().saturating_sub(2))
    {
        get_up_poses.push((catalogue_pose(catalogue, name, palette)?, dur as u8));
    }
    Ok(write_throw_pair(region, &throw_poses, thrown_poses, &get_up_poses, back_turned_poses))
}

/// ROM-only fallback throw and thrown streams for Ryu and Ken when no art-pack
/// catalogue was given (`GameSession::new` without `with_player_tables`).
fn build_shoto_rom_throws(rom: &[u8], region: &mut Region, fighter: TournamentFighter) -> ThrowEntry {
    let palette = rom_w(rom, fighter.move_table() + 0x30 + 2) & 0x3ff;
    let base: u16 = if fighter == TournamentFighter::Ken { 0x0800 } else { 0x0000 };
    let pose = |code: u16, large: bool, rows: u8, cols: u8, anchor: [i16; 2]| PoseGeom {
        code: base + code,
        attr: palette | if large { 0x400 } else { 0 },
        rows,
        cols,
        anchor,
    };
    let stance = pose(0x0063, true, 2, 1, [-32, -96]);
    let crouch = pose(0x0120, false, 3, 2, [-24, -64]);
    let rising = pose(0x0480, true, 1, 1, [-32, -64]);
    let lying = pose(0x0484, true, 0, 3, [-64, -32]);
    let grabbed = pose(0x00c0, true, 2, 1, [-32, -96]);
    let grab = pose(0x042a, true, 2, 2, [-32, -96]);
    let thrown_1 = pose(0x0480, true, 1, 1, [-32, -96]);
    let thrown_2 = pose(0x0480, true, 1, 1, [-32, -112]);
    write_throw_pair(
        region,
        &[(grab, 8), (grab, 10), (stance, 16)],
        [(grabbed, 8), (thrown_1, 8), (thrown_2, 8), (lying, 30)],
        &[(rising, 12), (crouch, 10)],
        0,
    )
}

/// What a fighter's jump attacks are made from.
struct Air<'a> {
    rom: &'a [u8],
    /// The top of his jump (his catalogue's `jump_top`): every air pose's
    /// anchor is placed from it.
    jump_top: PoseGeom,
    top_boxes: [u8; 16],
    fall: PoseGeom,
    fall_boxes: [u8; 16],
    landing: PoseGeom,
    landing_boxes: [u8; 16],
    stance: PoseGeom,
    stance_boxes: [u8; 16],
}

impl Air<'_> {
    /// One jump attack from the top of a jump (`apex`, the top of his jump
    /// pose there): the top of the jump, the strike and back to the top
    /// (catalogue: 6, 14, 8 frames), moving `step_dx` a frame and down
    /// `plan` a frame in each part, then [`Self::land`]. A same-tick origin
    /// conversion puts the strike where its anchor says without displacing
    /// the preceding visible pose or splitting the active record.
    #[allow(clippy::too_many_arguments)]
    fn attack(&self, region: &mut Region, apex: i32, strike: PoseGeom, strike_boxes: [u8; 16], hit_fn: u32, step_dx: i32, plan: [i32; 3], sound: u8) -> Result<u32, String> {
        let top_boxes = self.top_boxes;
        let rise = (strike.anchor[1] - self.jump_top.anchor[1]) as i32;
        let nudge = rise - self.jump_top.half().1 + strike.half().1;
        let origin = pose_origin_record(region, nudge, sound);
        let mut s = Stream::new(region, self.jump_top, apex);
        let at = s.travel(self.jump_top, apex, 5, (plan[0], step_dx), top_boxes, 0x20, 0);
        let top = s.top;
        s.travel(self.jump_top, top, 1, (plan[0], step_dx), top_boxes, 0, 0);
        s.change_origin(origin, nudge);
        s.strike(strike, 14, (plan[1], step_dx), strike_boxes, 0x80, hit_fn);
        let top = s.top - rise;
        s.travel(self.jump_top, top, 8, (plan[2], step_dx), top_boxes, 0, 0);
        self.land(&mut s, step_dx, 4)?;
        Ok(at)
    }

    /// A ROM strike from the top of a jump, its ROM pose, boxes, flags and
    /// hit as they are: `lead` frames of the top of the jump (moving
    /// `step.1` a frame), then `prep` (a ROM frame held for its frames,
    /// moving its step) if any, the strike's sound, the ROM frame `strike`
    /// held for `frames` frames moving `step`, each placed on the kept
    /// centre as the ROM places them, then [`Self::land`] at `fall_dy` a
    /// frame.
    #[allow(clippy::too_many_arguments)]
    fn rom_strike(
        &self,
        region: &mut Region,
        apex: i32,
        lead: u8,
        prep: Option<(u32, u8, (i32, i32))>,
        sound: u8,
        strike: u32,
        frames: u8,
        step: (i32, i32),
        fall_dy: i32,
    ) -> Result<u32, String> {
        let mut s = Stream::new(region, self.jump_top, apex);
        let mut at = None;
        if lead > 0 {
            at = Some(s.travel(self.jump_top, apex, lead, (0, step.1), self.top_boxes, 0x20, 0));
        }
        if let Some((frame, n, prep_step)) = prep {
            let r = rom_record(self.rom, frame);
            let a = s.strike(rom_pose(self.rom, frame), n, prep_step, rom_boxes(self.rom, frame), r[0x1a], rom_l(self.rom, frame + 0x1c));
            at.get_or_insert(a);
        }
        let a = s.call(routine::SOUND, sound);
        at.get_or_insert(a);
        let r = rom_record(self.rom, strike);
        s.strike(rom_pose(self.rom, strike), frames, step, rom_boxes(self.rom, strike), r[0x1a], rom_l(self.rom, strike + 0x1c));
        self.land(&mut s, step.1, fall_dy)?;
        Ok(at.expect("a first entry"))
    }

    /// Fall at `fall_dy` a frame in his falling pose (placed on the kept
    /// centre as the ROM does) until it stands at the ground line's height,
    /// then the 1987 landing: the half-crouch, the landing call and sound,
    /// the stance, the end.
    fn land(&self, s: &mut Stream, step_dx: i32, fall_dy: i32) -> Result<(), String> {
        let fall_boxes = self.fall_boxes;
        let ground_top = GROUND - self.fall.height();
        let top = s.top + s.pose.half().1 - self.fall.half().1;
        let left = ground_top - top;
        if left < 0 {
            return Err(format!("a jump attack ends {} pixels below the ground", -left));
        }
        let (n, rem) = (left / fall_dy, left % fall_dy);
        if n > 0 {
            s.travel(self.fall, top, n as u8, (fall_dy, step_dx), fall_boxes, 0, 0);
        }
        if rem > 0 || n == 0 {
            let top = if n > 0 { s.top } else { top };
            s.travel(self.fall, top, 1, (rem, step_dx), fall_boxes, 0, 0);
        }
        s.call(routine::FALLING, 0);
        let landing_boxes = self.landing_boxes;
        s.hold(self.landing, self.landing.top_on_ground(), 1, landing_boxes, 0, 0);
        s.call(routine::LANDING, 0);
        s.hold(self.landing, self.landing.top_on_ground(), 3, landing_boxes, 0, 0);
        s.hold(self.stance, self.stance.top_on_ground(), 1, self.stance_boxes, 0, 0);
        s.end();
        Ok(())
    }
}

/// The top of the jump pose at the ROM stream's entry `apex`, from the
/// header's grid at `header_top`, and that pose's half height: each
/// record's load keeps the centre and steps, each repeat steps again.
fn apex_top(rom: &[u8], stream: u32, apex: u32, header_top: i32, header_pose: PoseGeom) -> (i32, i32) {
    let (mut top, mut hy) = (header_top, header_pose.half().1);
    let mut a = stream;
    while a != apex {
        match rom_b(rom, a) {
            0xff => a += 6,
            0xfe => a = rom_l(rom, a + 2),
            0xfd => break,
            _ => {
                let p = rom_pose(rom, a);
                let dy = rom_b(rom, a + 6) as i8 as i32;
                let repeats = rom_b(rom, a) as i32;
                top = top + hy - p.half().1 + dy * (1 + repeats);
                hy = p.half().1;
                a += 0x20;
            }
        }
    }
    (top, hy)
}

/// For each of a ROM table's jumps (`(move, stream, apex entry)`), where
/// the top of `jump_top` is at the jump's apex: on the centre the jump pose
/// there has, which the engine keeps when an attack's first frame loads.
fn jump_top_at_apexes(rom: &[u8], rom_table: u32, jumps: &[(u8, u32, u32)], jump_top: PoseGeom) -> Vec<i32> {
    jumps
        .iter()
        .map(|&(id, stream, apex)| {
            let h = rom_table + id as u32 * 0x30;
            let p = PoseGeom {
                code: rom_w(rom, h),
                attr: rom_w(rom, h + 2),
                rows: rom_b(rom, h + 0xc),
                cols: rom_b(rom, h + 0xd),
                anchor: [0, 0],
            };
            let (top, half) = apex_top(rom, stream, apex, rom_w(rom, h + 4) as i32, p);
            top + half - jump_top.half().1
        })
        .collect()
}

/// Write the apex record (punch and kick entries for the neutral, forward and
/// back jumps: a strength table and a stream table of longs each) with the
/// landing centre `landing_centre`, named at `base + APEX_POINTER`, and return
/// its address.
fn put_apex_record(
    region: &mut Region,
    base: u32,
    landing_centre: i32,
    streams: [[(u8, u32); 3]; 6],
) -> u32 {
    let mut entries = Vec::new();
    for set in streams {
        let strtab = region.put(&[0x10, set[0].0, 0xd0, set[1].0, 0xf0, set[2].0, 0xff, set[2].0]);
        let at = region.put_l(set[0].1);
        region.put_l(set[1].1);
        region.put_l(set[2].1);
        entries.push((strtab, at));
    }
    let apex_routine = region.put(&[ROUTINE_JUMP_APEX, 0]);
    region.patch(base + APEX_POINTER, &apex_routine.to_be_bytes());
    region.put(&(landing_centre as u16).to_be_bytes());
    for (strtab, streams) in entries {
        region.put_l(strtab);
        region.put_l(streams);
    }
    apex_routine
}

/// The apex record (punch and kick entries for the neutral, forward and back
/// jumps: a strength table and a stream table of longs each) with the
/// landing centre `landing_centre`, named at `base + APEX_POINTER`; then
/// the table's jumps copied from the ROM with the apex call at their top.
/// `streams` are the jump attacks: `[neutral, forward, back]` punch then
/// kick, each `(light, medium, heavy)` move id and stream.
fn apex_and_jumps(
    region: &mut Region,
    rom: &[u8],
    rom_table: u32,
    base: u32,
    jumps: &[(u8, u32, u32); 3],
    landing_centre: i32,
    streams: [[(u8, u32); 3]; 6],
) {
    let apex_routine = put_apex_record(region, base, landing_centre, streams);
    let jump_snd: [u8; 3] = std::array::from_fn(|i| rom_b(rom, rom_table + jumps[0].0 as u32 * 0x30 + 0x10 + i as u32));
    for &(id, stream, apex) in jumps {
        let copy = copy_rom_stream(region, rom, stream, CopyHow { apex: Some((apex, apex_routine)), ..CopyHow::default() });
        keep_rom_header(region, rom, rom_table, base, id);
        let h = bank_header_offset(id) as usize;
        // CPU headers can carry the walking/standing bit and diagonal-walk
        // bit (0x82 / 0x92). Player input must hold the takeoff busy as 0x80,
        // as Ryu's jump header does; otherwise held Up restarts it each frame.
        // The stream's 0x45a7a call subsequently marks it airborne (0x81).
        region.bytes[h + 9] &= !0x12;
        if region.bytes[h + 0x10..h + 0x13] == [0, 0, 0] {
            region.bytes[h + 0x10..h + 0x13].copy_from_slice(&jump_snd);
        }
        region.patch(base + bank_header_offset(id) + 0x2c, &copy.to_be_bytes());
    }
}

/// Extend Ryu and Ken using ROM poses and the native effects. Their
/// ordinary moves keep using the ROM table bound by the 1987 engine.
fn build_shoto_sf2_moves(rom: &[u8], region: &mut Region, fighter: TournamentFighter) {
    let base = region.base;
    let rom_table = fighter.move_table();
    for id in 0..SF2_SPECIAL_SLOT {
        keep_rom_header(region, rom, rom_table, base, id);
    }
    for slot in SPECIAL_SLOTS {
        set_special(region, base, slot, SpecialSlot::ryu(slot));
    }
    match fighter {
        TournamentFighter::Ryu => {
            // The wind-up, thrust and settle in Ryu's Hadoken (0x1362).
            // Keep the original spawn routine so the engine attributes
            // projectile durability, tint and hits to the active move.
            for (id, windup, chamber) in [(SF2_SPECIAL_SLOT, 4, 3), (SF2_SUPER_SLOT, 2, 1)] {
                let source = rom_table + 0x1a * MOVE_HEADER_SIZE;
                let mut h: [u8; MOVE_HEADER_SIZE as usize] =
                    std::array::from_fn(|i| rom_b(rom, source + i as u32));
                let mut gather = rom_record(rom, 0x1362);
                gather[1] = chamber - 1;
                let stream = region.put(&gather);
                region.put(&rom_record(rom, 0x1382));
                region.call(0x4548c, 0);
                let mut thrust = rom_record(rom, 0x13a8);
                thrust[1] = 23;
                region.put(&thrust);
                region.put(&rom_record(rom, 0x13c8));
                region.end();
                h[0xa] = id;
                h[0xe] = windup - 1;
                h[0x2c..0x30].copy_from_slice(&stream.to_be_bytes());
                put_header(region, base, id, &h);
            }
        }
        TournamentFighter::Ken => write_shoryureppa(rom, rom_table, region),
        _ => unreachable!("only Ryu and Ken have ROM player tables"),
    }
}

fn build_shoto_alpha_supers(rom: &[u8], region: &mut Region, fighter: TournamentFighter) {
    const CLOSE_REACTION: u32 = 0x45b6e;
    const PUNCH_REACTION: u32 = 0x45c88;
    const RISING_REACTION: u32 = 0x45f52;
    if rom.len() < (RISING_REACTION + 8) as usize {
        return;
    }

    let strike_entry = |region: &mut Region, source, damage| {
        let mut entry: [u8; 8] = std::array::from_fn(|i| rom_b(rom, source + i as u32));
        entry[1] = damage;
        region.put(&entry)
    };
    let hit = strike_entry(region, PUNCH_REACTION, 4);
    let finish = strike_entry(region, RISING_REACTION, 5);
    let rising_hit = if fighter == TournamentFighter::Ken {
        strike_entry(region, CLOSE_REACTION, 4)
    } else {
        hit
    };
    write_alpha_rising_super(rom, region, fighter, rising_hit, finish);
    match fighter {
        TournamentFighter::Ryu => write_shinkuu_tatsumaki(rom, region, hit, finish),
        TournamentFighter::Ken => write_shippu_jinraikyaku(rom, region, hit, finish),
        _ => unreachable!("only Ryu and Ken have Alpha shoto Supers"),
    }
}

fn write_alpha_rising_super(
    rom: &[u8],
    region: &mut Region,
    fighter: TournamentFighter,
    hit: u32,
    finish: u32,
) {
    // Metsu advances into the rising fist. Shinryuken rises in place.
    let (launch_frame, lift_frame, fall_frame, forward) = match fighter {
        TournamentFighter::Ryu => (0x1200, 0x1220, 0x12c0, 2),
        TournamentFighter::Ken => (0x32de, 0x32fe, 0x339e, 0),
        _ => unreachable!("only Ryu and Ken have Alpha shoto Supers"),
    };
    let source = fighter.move_table() + 0x19 * MOVE_HEADER_SIZE;
    let (crouch, crouch_boxes) = header_pose(rom, source);
    let launch = rom_pose(rom, launch_frame);
    let lift = rom_pose(rom, lift_frame);
    let fall = rom_pose(rom, fall_frame);
    let mut launch_boxes = rom_boxes(rom, launch_frame);
    let mut lift_boxes = rom_boxes(rom, lift_frame);
    // Cover a grounded opponent directly under the uppercut as the fighter rises.
    lift_boxes[12..16].copy_from_slice(&[0, 28, 20, 20]);
    if fighter == TournamentFighter::Ken {
        // The vertical Shinryuken's spin reaches past the first hit's recoil.
        launch_boxes[12..16].copy_from_slice(&[12, 20, 26, 26]);
        lift_boxes[12..16].copy_from_slice(&[0xf8, 32, 28, 28]);
    }
    let (launch_origin, launch_dy) = ground_origin_record(region, crouch, launch);
    let (lift_origin, lift_dy) = ground_origin_record(region, launch, lift);
    let stream = region.here();
    let mut s = Stream::new(region, crouch, crouch.top_on_ground());
    s.change_origin(launch_origin, launch_dy);
    s.call(routine::RISING, 0);
    s.strike(launch, 3, (-4, forward), launch_boxes, 0x88, hit);
    s.strike(launch, 3, (-4, forward), launch_boxes, 0x88, hit);
    s.change_origin(lift_origin, lift_dy);
    s.strike(lift, 4, (-4, forward), lift_boxes, 0x88, hit);
    s.strike(lift, 4, (-3, forward), lift_boxes, 0x88, hit);
    s.strike(lift, 4, (-2, forward), lift_boxes, 0x88, finish);
    s.call(routine::FALLING, 0);
    let fall_boxes = rom_boxes(rom, fall_frame);
    s.travel(fall, fall.top_on_ground() - 60, 5, (2, 0), fall_boxes, 0, 0);
    s.travel(fall, fall.top_on_ground() - 50, 5, (4, 0), fall_boxes, 0, 0);
    s.travel(fall, fall.top_on_ground() - 30, 6, (5, 0), fall_boxes, 0, 0);
    s.call(routine::TOUCHDOWN, 0);
    s.hold(crouch, crouch.top_on_ground(), 3, crouch_boxes, 0, 0);
    s.call(routine::LANDING, 0);
    s.hold(crouch, crouch.top_on_ground(), 12, crouch_boxes, 0, 0);
    s.end();

    let mut first = still(crouch, 4, crouch_boxes);
    first.flags = 0x08;
    let sounds = std::array::from_fn(|i| rom_b(rom, source + 0x10 + i as u32));
    let id = ALPHA_SUPER_SECOND_SLOT;
    let h = header(&first.bytes(), crouch.top_on_ground(), 0xc4, id, sounds, stream);
    put_header(region, region.base, id, &h);
}

fn write_shinkuu_tatsumaki(rom: &[u8], region: &mut Region, hit: u32, finish: u32) {
    let source = TournamentFighter::Ryu.move_table() + 0x1b * MOVE_HEADER_SIZE;
    let (crouch, crouch_boxes) = header_pose(rom, source);
    let chamber = rom_pose(rom, 0x13ea);
    let spin = rom_pose(rom, 0x14b0);
    let turn = rom_pose(rom, 0x14d6);
    let fall = rom_pose(rom, 0x14f6);
    let mut spin_boxes = rom_boxes(rom, 0x14b0);
    // Give the spinning leg a useful close hitbox throughout the low hover.
    spin_boxes[12..16].copy_from_slice(&[0xf2, 18, 16, 36]);
    let stream = region.here();
    let mut s = Stream::new(region, crouch, crouch.top_on_ground());
    s.call(routine::RISING, 0);
    s.travel(chamber, chamber.top_on_ground(), 4, (-4, 1), rom_boxes(rom, 0x13ea), 0, 0);
    for spin_index in 0..5 {
        let entry = if spin_index == 4 { finish } else { hit };
        s.strike(spin, 3, (0, 1), spin_boxes, 0x80, entry);
        if spin_index < 4 {
            s.hold(turn, turn.top_on_ground() - 16, 3, rom_boxes(rom, 0x14d6), 0, 0);
        }
    }
    s.call(routine::FALLING, 0);
    s.travel(fall, fall.top_on_ground() - 16, 4, (4, 0), rom_boxes(rom, 0x14f6), 0, 0);
    s.call(routine::TOUCHDOWN, 0);
    s.hold(crouch, crouch.top_on_ground(), 3, crouch_boxes, 0, 0);
    s.call(routine::LANDING, 0);
    s.hold(crouch, crouch.top_on_ground(), 12, crouch_boxes, 0, 0);
    s.end();

    let first = still(crouch, 4, crouch_boxes);
    let id = ALPHA_SUPER_THIRD_SLOT;
    let h = header(&first.bytes(), crouch.top_on_ground(), 0xc4, id, [0x8b, 0, 0x19], stream);
    put_header(region, region.base, id, &h);
}

fn write_shippu_jinraikyaku(rom: &[u8], region: &mut Region, hit: u32, finish: u32) {
    let windup = rom_pose(rom, 0x2f40);
    let windup_boxes = rom_boxes(rom, 0x2f40);
    let chamber = rom_pose(rom, 0x2f60);
    let spin = rom_pose(rom, 0x358e);
    let stream = region.here();
    let mut s = Stream::new(region, windup, windup.top_on_ground());
    for frame in [0x30ca, 0x2f80, 0x30ca, 0x2f80] {
        let pose = rom_pose(rom, frame);
        let mut boxes = rom_boxes(rom, frame);
        boxes[12..16].copy_from_slice(&[20, 22, 15, 32]);
        s.strike(pose, 3, (0, 2), boxes, 0x80, hit);
        s.travel(chamber, chamber.top_on_ground(), 3, (0, 2), rom_boxes(rom, 0x2f60), 0, 0);
    }
    s.call(routine::RISING, 0);
    let mut finisher_boxes = rom_boxes(rom, 0x358e);
    finisher_boxes[12..16].copy_from_slice(&[0, 28, 24, 34]);
    s.strike(spin, 4, (-4, 2), finisher_boxes, 0x80, finish);
    s.call(routine::FALLING, 0);
    s.travel(chamber, chamber.top_on_ground() - 16, 4, (4, 1), rom_boxes(rom, 0x2f60), 0, 0);
    s.call(routine::TOUCHDOWN, 0);
    s.hold(windup, windup.top_on_ground(), 3, windup_boxes, 0, 0);
    s.call(routine::LANDING, 0);
    s.hold(windup, windup.top_on_ground(), 12, windup_boxes, 0, 0);
    s.end();

    let first = still(windup, 4, windup_boxes);
    let id = ALPHA_SUPER_THIRD_SLOT;
    let h = header(&first.bytes(), windup.top_on_ground(), 0xc2, id, [0x82, 0x05, 0x19], stream);
    put_header(region, region.base, id, &h);
}

/// Reuse each Alpha fighter's own 1987 ground attack poses for his counter.
/// Its active frame points at a banked hit entry in the same format as the
/// ROM's, so the native collision handler can use the existing reactions.
fn write_alpha_shoto_overhead(rom: &[u8], region: &mut Region, fighter: TournamentFighter) {
    let [windup, strike, retract] = match fighter {
        TournamentFighter::Ryu => [0x0af0, 0x0b30, 0x0b50],
        TournamentFighter::Ken => [0x2f40, 0x2f80, 0x2f60],
        _ => return,
    };
    let [ready, active, recovery] = [windup, strike, retract].map(|at| rom_pose(rom, at));
    let mut boxes = rom_boxes(rom, strike);
    boxes[12..16].copy_from_slice(&[2, 24, 21, 30]);
    let entry = region.put(&std::array::from_fn::<_, 8, _>(|i| rom_b(rom, 0x45c88 + i as u32)));
    let (origin, shift) = ground_origin_record(region, ready, active);
    let stream = region.here();
    let mut s = Stream::new(region, ready, ready.top_on_ground());
    s.travel(ready, ready.top_on_ground(), 4, (0, 2), rom_boxes(rom, windup), 0, 0);
    s.change_origin(origin, shift);
    s.call(routine::SOUND, 0xa2);
    s.hold(active, active.top_on_ground(), 5, boxes, 0x80, entry);
    s.hold(recovery, recovery.top_on_ground(), 12, rom_boxes(rom, retract), 0, 0);
    s.end();
    let first = still(ready, 11, rom_boxes(rom, windup));
    let slot = 0x1c;
    let h = header(&first.bytes(), ready.top_on_ground(), 0xc2, slot, [0; 3], stream);
    put_header(region, region.base, slot, &h);
}

fn write_alpha_custom(rom: &[u8], region: &mut Region, fighter: TournamentFighter) {
    let (pose, boxes) = header_pose(rom, fighter.move_table() + MOVE_HEADER_SIZE);
    let first = still(pose, 6, boxes);
    let stream = region.end();
    let slot = ALPHA_COUNTER_SLOT;
    let move_header = header(&first.bytes(), pose.top_on_ground(), 0xc2, slot, [0; 3], stream);
    put_era_header(region, EraFamily::SfZ, slot, &move_header);
}

fn write_alpha_counter(rom: &[u8], region: &mut Region, fighter: TournamentFighter) {
    // The 1987 push check uses the drawn center. A compact windup keeps it
    // on the fighter's side of a close opponent when the guard pose changes.
    let (windup_frame, strike_frame, retract_frame) = match fighter {
        TournamentFighter::Ryu => (0x0af0, 0x0b30, 0x0b50), // straight punch
        TournamentFighter::Ken => (0x2bce, 0x2c0e, 0x2c2e), // straight punch
        TournamentFighter::Gen => (0x103b0, 0x103d6, 0x103f6), // palm strike
        TournamentFighter::Birdie => (0x1151c, 0x11562, 0x115a2), // hammer punch
        TournamentFighter::Eagle => (0x12408, 0x1242e, 0x124ae), // stick swing
        TournamentFighter::Adon => (0x1336c, 0x13392, 0x133b2), // body punch
        TournamentFighter::Sagat => (0x14030, 0x14056, 0x14096), // knee strike
        _ => match what_if::alpha_counter_frames(fighter) {
            Some(frames) => frames,
            None => return,
        },
    };
    let windup = rom_pose(rom, windup_frame);
    let strike = rom_pose(rom, strike_frame);
    let retract = rom_pose(rom, retract_frame);
    let mut strike_boxes = rom_boxes(rom, strike_frame);
    const SHORYUKEN_HIT_ENTRY: u32 = 0x45f52;
    if strike_boxes[15] == 0 || rom.len() < (SHORYUKEN_HIT_ENTRY + 8) as usize {
        return;
    }

    // Keep the strike's original tip and widen the box back toward the body
    // and down toward a crouched attacker. The fighter does not slide.
    let origin_x = i32::from(strike.x_offset());
    let origin_height = strike.half().1;
    let tip = origin_x + i32::from(strike_boxes[14] as i8) + i32::from(strike_boxes[15]);
    let top = origin_height + i32::from(strike_boxes[12] as i8) + i32::from(strike_boxes[13]);
    let near = 8;
    let bottom = 24;
    let half_width = (tip - near + 1) / 2;
    let half_height = (top - bottom + 1) / 2;
    strike_boxes[12..16].copy_from_slice(&[
        (bottom + half_height - origin_height) as i8 as u8,
        half_height as u8,
        (near + half_width - origin_x) as i8 as u8,
        half_width as u8,
    ]);

    let mut entry: [u8; 8] = std::array::from_fn(|i| rom_b(rom, SHORYUKEN_HIT_ENTRY + i as u32));
    entry[1] = 5; // Five of the 48 original health points.
    entry[3] = entry[2]; // Crouching targets also get the standing blowback.
    let hit = region.put(&entry);

    const INVULNERABLE: u8 = 0x08;
    const ACTIVE: u8 = 0x80;
    let mut first = still(windup, 3, rom_boxes(rom, windup_frame));
    first.flags = INVULNERABLE;
    let stream = region.here();
    let mut s = Stream::new(region, windup, windup.top_on_ground());
    s.hold(strike, strike.top_on_ground(), 4, strike_boxes, ACTIVE | INVULNERABLE, hit);
    s.hold(retract, retract.top_on_ground(), 16, rom_boxes(rom, retract_frame), 0, 0);
    s.end();

    let h = header(&first.bytes(), windup.top_on_ground(), 0xc2, ALPHA_COUNTER_SLOT, [0; 3], stream);
    put_header(region, region.base, ALPHA_COUNTER_SLOT, &h);
}

fn write_shoryureppa(rom: &[u8], rom_table: u32, region: &mut Region) {
    let source = rom_table + 0x19 * MOVE_HEADER_SIZE;
    let (crouch, crouch_boxes) = header_pose(rom, source);
    let sounds = std::array::from_fn(|i| rom_b(rom, source + 0x10 + i as u32));
    let strike = 0x32de;
    let extended = 0x331e;
    let retract = 0x339e;
    let launch = rom_pose(rom, strike);
    let (launch_origin, launch_dy) = ground_origin_record(region, crouch, launch);
    let (extend_origin, extend_dy) = ground_origin_record(region, launch, rom_pose(rom, extended));
    let base = region.base;
    let stream = region.here();
    let mut s = Stream::new(region, crouch, crouch.top_on_ground());

    let advance = |s: &mut Stream<'_>, frame, duration, step, hit| {
        let flags = if hit == 0 { 0 } else { rom_b(rom, frame + 0x1a) };
        s.strike(rom_pose(rom, frame), duration, step, rom_boxes(rom, frame), flags, hit);
    };

    // A low jab Shoryuken leads into a much higher fierce Shoryuken in ST.
    // Use Ken's close punch reaction on the opener to keep the target in reach.
    let opening_hit = 0x45c16;
    s.change_origin(launch_origin, launch_dy);
    s.call(routine::RISING, 0);
    advance(&mut s, strike, 2, (-7, 6), opening_hit);
    s.change_origin(extend_origin, extend_dy);
    advance(&mut s, extended, 6, (-4, 3), opening_hit);
    s.call(routine::FALLING, 0);
    advance(&mut s, retract, 4, (2, 2), 0);
    advance(&mut s, retract, 6, (5, 2), 0);
    s.call(routine::TOUCHDOWN, 0);
    s.hold(crouch, crouch.top_on_ground(), 3, crouch_boxes, 0, 0);

    let finishing_hit = 0x45f04;
    s.change_origin(launch_origin, launch_dy);
    s.call(routine::RISING, 0);
    advance(&mut s, strike, 2, (-7, 6), finishing_hit);
    s.change_origin(extend_origin, extend_dy);
    advance(&mut s, 0x32fe, 2, (-7, 5), finishing_hit);
    advance(&mut s, extended, 9, (-5, 3), finishing_hit);
    advance(&mut s, extended, 6, (-2, 1), 0);
    s.call(routine::FALLING, 0);
    advance(&mut s, extended, 4, (1, 0), 0);
    advance(&mut s, retract, 9, (2, 0), 0);
    advance(&mut s, retract, 11, (3, 0), 0);
    advance(&mut s, retract, 6, (5, 0), 0);
    s.call(routine::TOUCHDOWN, 0);
    s.hold(crouch, crouch.top_on_ground(), 4, crouch_boxes, 0, 0);
    s.call(routine::LANDING, 0);
    s.hold(crouch, crouch.top_on_ground(), 11, crouch_boxes, 0, 0);
    s.end();

    let first = still(crouch, 4, crouch_boxes);
    let h = header(&first.bytes(), crouch.top_on_ground(), 0xc4, SF2_SUPER_SLOT, sounds, stream);
    put_header(region, base, SF2_SUPER_SLOT, &h);
}

/// Write one or more short rising arcs using two 1987 attack poses per
/// arc. Two active records let the native hit code register each contact.
#[allow(clippy::too_many_arguments)]
fn write_rising_special(
    rom: &[u8],
    region: &mut Region,
    id: u8,
    startup: u8,
    (crouch, crouch_boxes): (PoseGeom, [u8; 16]),
    (fall, fall_boxes): (PoseGeom, [u8; 16]),
    arcs: &[[(u32, i32); 2]],
    sounds: [u8; 3],
) {
    let base = region.base;
    let stream = region.here();
    let mut s = Stream::new(region, crouch, crouch.top_on_ground());
    for (index, arc) in arcs.iter().enumerate() {
        s.call(routine::RISING, 0);
        for ((frame, forward), (duration, dy)) in arc.iter().copied().zip([(3, -5), (4, -4)]) {
            s.strike(
                rom_pose(rom, frame),
                duration,
                (dy, forward),
                rom_boxes(rom, frame),
                rom_b(rom, frame + 0x1a),
                rom_l(rom, frame + 0x1c),
            );
        }
        s.call(routine::FALLING, 0);
        let fall_top = s.top + s.pose.half().1 - fall.half().1;
        s.travel(fall, fall_top, 7, (4, 1), fall_boxes, 0, 0);
        s.call(routine::TOUCHDOWN, 0);
        s.hold(crouch, crouch.top_on_ground(), 3, crouch_boxes, 0, 0);
        s.call(routine::LANDING, 0);
        // A native stream call loads a frame next, even between the arcs.
        let landing = if index + 1 == arcs.len() { 8 } else { 1 };
        s.hold(crouch, crouch.top_on_ground(), landing, crouch_boxes, 0, 0);
    }
    s.end();
    let first = still(crouch, startup, crouch_boxes);
    let h = header(&first.bytes(), crouch.top_on_ground(), 0xc4, id, sounds, stream);
    put_header(region, base, id, &h);
}

/// A fighter's slot with room for his table, the apex pointer and the
/// special slots' records (patched by [`set_special`]).
fn new_slot(fighter: TournamentFighter) -> Region {
    let mut region = Region::new(table_address(fighter));
    region.put(&[0; (ERA_HEADERS + ERA_HEADER_TABLES * PLAYER_MOVES as u32 * 4) as usize]);
    region
}

/// Write special slot `slot`'s record into the table at `base`.
fn set_special(region: &mut Region, base: u32, slot: u8, special: SpecialSlot) {
    region.patch(base + SPECIALS + (slot - 0x19) as u32 * SPECIAL_SLOT_SIZE, &special.bytes());
}

/// The slot's size checked against [`FIGHTER_SPAN`].
fn fits(region: Region) -> Result<Region, String> {
    if region.bytes.len() > FIGHTER_SPAN as usize {
        return Err(format!("his data takes {} bytes, more than his slot", region.bytes.len()));
    }
    Ok(region)
}

/// Retsu (ticket #248): his ROM table's stance, walks, block, crouch and
/// jumps; his four standing 1987 attacks, his sweep and his flying kick as
/// they are; crouching punches and kick and his neutral jump's attacks from
/// his catalogue; the Kiai Shot on the Hadoken's command.
mod retsu {
    use super::*;

    /// His ROM move table (`0x16e6c`, thirteen moves).
    const ROM_TABLE: u32 = 0x16e6c;

    /// His 1987 attacks (the streams his CPU records start).
    const JAB: u32 = 0xc9b6;
    const LUNGE: u32 = 0xca46;
    const MID_KICK: u32 = 0xcb16;
    const HIGH_KICK: u32 = 0xcbe6;
    const SWEEP: u32 = 0xcf1a;

    /// ROM frames whose poses and boxes his new streams reuse.
    const STANCE_FRAME: u32 = 0xca1c; // 0x1000, his stance
    const CROUCH_FRAME: u32 = 0xc414; // 0x124d, his crouch
    const BLOCK_FRAME: u32 = 0xc890; // 0x1065, his standing block
    const CROUCH_BLOCK_FRAME: u32 = 0xc926; // 0x120d, his crouching block
    const WIND_UP_FRAME: u32 = 0xc9b6; // 0x1120, the jab's wind-up
    const STRIKE_FRAME: u32 = 0xc9dc; // 0x1320, the jab's open palm
    const JUMP_TOP_FRAME: u32 = 0xc4ce; // 0x1288, the top of his jump
    const FALL_FRAME: u32 = 0xc50e; // 0x1244, rising and falling
    const LANDING_FRAME: u32 = 0xc534; // 0x1129, his half-crouch
    const FLYING_KICK_FRAME: u32 = 0xccd6 + 0x6c; // 0xcd42: 0x1248, his flying kick

    /// His jumps (ROM streams) and where each reaches its top: the apex
    /// call goes before that entry.
    const JUMPS: [(u8, u32, u32); 3] = [(0x09, 0xc4a8, 0xc4ee), (0x0a, 0xc5c8, 0xc60e), (0x0b, 0xc708, 0xc74e)];

    /// Boxes for his new poses, as rectangles in each drawing (see
    /// `tests/retsu_player.rs`, which checks them against the pens).
    pub const BOXES: [(&str, PixelBoxes); 7] = [
        ("crouching_punch_1", PixelBoxes { push: [4, 0, 36, 64], head: [13, 0, 28, 12], body: [0, 0, 40, 64], attack: None }),
        (
            "crouching_punch_2",
            PixelBoxes { push: [4, 0, 36, 64], head: [13, 0, 28, 12], body: [0, 0, 40, 64], attack: Some([44, 13, 62, 25]) },
        ),
        (
            "crouching_kick",
            PixelBoxes { push: [10, 0, 50, 63], head: [22, 0, 36, 12], body: [2, 0, 52, 63], attack: Some([66, 52, 86, 62]) },
        ),
        ("jump_punch", PixelBoxes { push: [12, 0, 50, 64], head: [19, 0, 33, 12], body: [4, 0, 46, 72], attack: Some([58, 3, 71, 18]) }),
        (
            "jump_punch_down",
            PixelBoxes { push: [14, 0, 52, 64], head: [36, 0, 51, 12], body: [6, 0, 52, 72], attack: Some([62, 37, 77, 48]) },
        ),
        ("jump_kick", PixelBoxes { push: [12, 0, 50, 64], head: [19, 0, 33, 12], body: [4, 0, 48, 72], attack: Some([68, 27, 84, 43]) }),
        (
            "jump_kick_high",
            PixelBoxes { push: [12, 0, 50, 64], head: [21, 0, 36, 12], body: [2, 0, 52, 71], attack: Some([74, 4, 93, 16]) },
        ),
    ];

    fn boxes(name: &str) -> PixelBoxes {
        boxes_named(&BOXES, name)
    }

    pub fn build(rom: &[u8], catalogue: &Catalogue) -> Result<Region, String> {
        let base = table_address(TournamentFighter::Retsu);
        let mut region = new_slot(TournamentFighter::Retsu);
        let palette = rom_w(rom, ROM_TABLE + 0x30 + 2) & 0xff;
        let geom = |name: &str| catalogue_pose(catalogue, name, palette);

        // ROM headers kept as they are: the walk-in, stance, walks and win.
        for id in [0x00u8, 0x01, 0x02, 0x03, 0x0c] {
            keep_rom_header(&mut region, rom, ROM_TABLE, base, id);
        }

        // The held postures, as Ryu's are: a still header (no busy bit, no
        // stream), the pose and boxes of his ROM block, crouch and crouching
        // block. His ROM ones are timed CPU moves that stand him back up.
        let block_sound = rom_b(rom, ROM_TABLE + 4 * 0x30 + 0x10);
        let still_header = |region: &mut Region, id: u8, frame: u32, state: u8, sound: u8| {
            let r = rom_record(rom, frame);
            let p = rom_pose(rom, frame);
            let h = header(&r, p.top_on_ground(), state, id, [sound, 0, 0], 0);
            put_header(region, base, id, &h);
        };
        still_header(&mut region, 0x04, BLOCK_FRAME, 0x22, block_sound);
        for id in [0x05, 0x06, 0x07] {
            still_header(&mut region, id, CROUCH_FRAME, 0x14, 0);
        }
        still_header(&mut region, 0x08, CROUCH_BLOCK_FRAME, 0x24, block_sound);

        // His 1987 attacks: the header is the stream's first frame, the rest
        // of the stream stays in the ROM (it ends at the CPU follow-up,
        // which ends a player's move).
        let stance = rom_pose(rom, STANCE_FRAME);
        let rom_attack = |region: &mut Region, id: u8, stream: u32, state: u8| {
            let r = rom_record(rom, stream);
            let p = rom_pose(rom, stream);
            // Where the CPU's stream would put it from his stance.
            let top = stance.top_on_ground() + stance.half().1 - p.half().1 + r[6] as i8 as i32;
            let h = header(&r, top, state, id, [0, 0, 0], stream + 0x20);
            put_header(region, base, id, &h);
        };
        for (id, stream) in [(0x0d, JAB), (0x0e, JAB), (0x0f, LUNGE), (0x10, LUNGE), (0x14, MID_KICK), (0x15, MID_KICK), (0x16, HIGH_KICK)] {
            rom_attack(&mut region, id, stream, 0xc2);
        }
        rom_attack(&mut region, 0x18, SWEEP, 0xc2);

        // Crouching punch, every strength (catalogue: crouch 8, wind-up 4,
        // strike 12, wind-up 4, crouch 20).
        let crouch = rom_pose(rom, CROUCH_FRAME);
        let crouch_boxes = rom_boxes(rom, CROUCH_FRAME);
        let cp1 = geom("crouching_punch_1")?;
        let cp2 = geom("crouching_punch_2")?;
        let crouching_punch = {
            let mut s = Stream::new(&mut region, crouch, crouch.top_on_ground());
            let at = s.hold(cp1, cp1.top_on_ground(), 4, boxes("crouching_punch_1").bytes(&cp1), 0x20, 0);
            s.call(routine::SOUND, 0xa1);
            s.hold(cp2, cp2.top_on_ground(), 12, boxes("crouching_punch_2").bytes(&cp2), 0x80, hit::CROUCHING_PUNCH);
            s.hold(cp1, cp1.top_on_ground(), 4, boxes("crouching_punch_1").bytes(&cp1), 0, 0);
            s.hold(crouch, crouch.top_on_ground(), 20, crouch_boxes, 0, 0);
            s.end();
            at
        };
        let crouch_rec = still(crouch, 8, crouch_boxes);
        for id in [0x11u8, 0x12, 0x13, 0x1c, 0x1d, 0x1e] {
            let h = header(&crouch_rec.bytes(), crouch.top_on_ground(), 0xc4, id, [0, 0, 0], crouching_punch);
            put_header(&mut region, base, id, &h);
        }

        // Crouching light and medium kick (catalogue: crouch 6, kick 14,
        // crouch 14).
        let ck = geom("crouching_kick")?;
        let crouching_kick = {
            let mut s = Stream::new(&mut region, crouch, crouch.top_on_ground());
            let at = s.call(routine::SOUND, 0xa2);
            s.hold(ck, ck.top_on_ground(), 14, boxes("crouching_kick").bytes(&ck), 0xa0, hit::CROUCHING_KICK);
            s.hold(crouch, crouch.top_on_ground(), 14, crouch_boxes, 0, 0);
            s.end();
            at
        };
        let h = header(&still(crouch, 6, crouch_boxes).bytes(), crouch.top_on_ground(), 0xc4, 0x17, [0, 0, 0], crouching_kick);
        put_header(&mut region, base, 0x17, &h);

        // The Kiai Shot: a short-range blast from his palm that fades out in
        // front of him (the maintainer, 2026-11-08), not Ryu's and Ken's full-screen
        // Hadoken. Its numbers and why are at `kiai`.
        let wind_up = rom_pose(rom, WIND_UP_FRAME);
        let palm = rom_pose(rom, STRIKE_FRAME);
        let mut palm_boxes = rom_boxes(rom, STRIKE_FRAME);
        palm_boxes[12..16].fill(0); // the blast hits, not the palm
        let stance_boxes = rom_boxes(rom, STANCE_FRAME);
        let mut blast_frames = Vec::new();
        for name in kiai::FRAMES {
            blast_frames.push(geom(name)?);
        }
        let blast = kiai::blast(&mut region, rom, blast_frames.try_into().expect("six blast frames"))?;
        let kiai = {
            let mut s = Stream::new(&mut region, stance, stance.top_on_ground());
            let at = s.hold(wind_up, wind_up.top_on_ground(), kiai::WIND_UP, rom_boxes(rom, WIND_UP_FRAME), 0x20, 0);
            s.hold(palm, palm.top_on_ground(), 1, palm_boxes, 0, 0);
            s.call(blast, 0);
            s.hold(palm, palm.top_on_ground(), kiai::PALM_HELD, palm_boxes, 0, 0);
            s.hold(wind_up, wind_up.top_on_ground(), kiai::BACK, rom_boxes(rom, WIND_UP_FRAME), 0, 0);
            s.hold(stance, stance.top_on_ground(), kiai::SETTLE, stance_boxes, 0, 0);
            s.end();
            at
        };
        let h = header(&still(stance, kiai::STANCE, stance_boxes).bytes(), stance.top_on_ground(), 0xc2, 0x1a, [0, 0, 0], kiai);
        put_header(&mut region, base, 0x1a, &h);
        // On the Hadoken's command, a projectile that scores by its own hit
        // record: the Hadoken's slot as 1987 has it.
        set_special(&mut region, base, 0x1a, SpecialSlot::ryu(0x1a));

        // The jumps' attacks, from the top of each jump.
        let air = Air {
            rom,
            jump_top: geom("jump_top")?,
            top_boxes: rom_boxes(rom, JUMP_TOP_FRAME),
            fall: rom_pose(rom, FALL_FRAME),
            fall_boxes: rom_boxes(rom, FALL_FRAME),
            landing: rom_pose(rom, LANDING_FRAME),
            landing_boxes: rom_boxes(rom, LANDING_FRAME),
            stance,
            stance_boxes,
        };
        let jp = geom("jump_punch")?;
        let jpd = geom("jump_punch_down")?;
        let jk = geom("jump_kick")?;
        let jkh = geom("jump_kick_high")?;
        let apex = jump_top_at_apexes(rom, ROM_TABLE, &JUMPS, air.jump_top);
        let (neutral, diagonal, back) = (apex[0], apex[1], apex[2]);
        // Neutral: a hang at the top, then down 2 and 4 a frame; diagonal:
        // down 3 and 4, 4 pixels a frame forward or back, as his diagonal
        // jumps fall.
        let s20 = air.attack(&mut region, neutral, jp, boxes("jump_punch").bytes(&jp), hit::JUMP_PUNCH, 0, [0, 2, 4], 0xa1)?;
        let s21 = air.attack(&mut region, diagonal, jpd, boxes("jump_punch_down").bytes(&jpd), hit::JUMP_PUNCH, 4, [0, 3, 4], 0xa1)?;
        let s22 = air.attack(&mut region, back, jpd, boxes("jump_punch_down").bytes(&jpd), hit::JUMP_PUNCH, -4, [0, 3, 4], 0xa1)?;
        let s23 = air.attack(&mut region, neutral, jk, boxes("jump_kick").bytes(&jk), hit::JUMP_KICK, 0, [0, 2, 4], 0xa2)?;
        let s26 = air.attack(&mut region, neutral, jkh, boxes("jump_kick_high").bytes(&jkh), hit::JUMP_KICK, 0, [0, 2, 4], 0xa2)?;
        // His flying kick from a diagonal jump: the ROM pose, boxes and hit
        // (stream 0xccd6) struck from the top of the jump for its 8 frames
        // after 6 at the top, stepping down 3 as the ROM does, then his ROM
        // fall (7 a frame).
        let s24 = air.rom_strike(&mut region, diagonal, 6, None, 0xa6, FLYING_KICK_FRAME, 8, (3, 4), 7)?;
        let s25 = air.rom_strike(&mut region, back, 6, None, 0xa6, FLYING_KICK_FRAME, 8, (3, -4), 7)?;

        // The apex record and the jumps. The landing centre: his
        // half-crouch on the ground at the landing call.
        let landing_centre = air.landing.top_on_ground() + air.landing.half().1;
        apex_and_jumps(
            &mut region,
            rom,
            ROM_TABLE,
            base,
            &JUMPS,
            landing_centre,
            [
                [(0x20, s20), (0x20, s20), (0x20, s20)],
                [(0x21, s21), (0x21, s21), (0x21, s21)],
                [(0x22, s22), (0x22, s22), (0x22, s22)],
                [(0x23, s23), (0x23, s23), (0x26, s26)],
                [(0x24, s24), (0x24, s24), (0x24, s24)],
                [(0x25, s25), (0x25, s25), (0x25, s25)],
            ],
        );
        fits(region)
    }

    /// The Kiai Shot's numbers (the maintainer, 2026-11-08: a short-range blast
    /// that fades out in front of him, tuned from the research and his
    /// character; `docs/research/sf1plus-reference-numbers.md`).
    ///
    /// - **Startup 12** (stance 3, wind-up 8; the palm's first frame spawns
    ///   the blast, which is out and can hit from the move's 13th frame, for
    ///   16 frames). Slower than his jab (3) and about Ryu's Hadoken's
    ///   (spawned on frame 8, travelling), as a special should be: a
    ///   close-range answer, not a zoning tool.
    /// - **Reach about 110 pixels** from his centre: the blast's centre goes
    ///   from 68 to 100 ahead of him while its box shrinks from 12 to 6 each
    ///   way as it fades, against his high kick's ~96 (`0xcbe6`'s box). Past
    ///   that it is gone, so he has to walk in where Ryu and Ken throw from
    ///   range.
    /// - **Damage 14 of 48** on hit (29%), 4 of chip on a guard: above his
    ///   1987 normals' 8 to 12 (`0x466a8..`) and under the Hadoken's 20; no
    ///   slow motion (a poke, not a projectile knock-out). It scores as the
    ///   Hadoken (`0x50`).
    /// - **Recovery 29** after the blast (palm 19, wind-up 6, stance 4):
    ///   shorter than the Hadoken's 56 because it has no range to hide
    ///   behind, still long enough that a whiffed or guarded blast at close
    ///   range can be punished. Nothing cuts it short, so its frames are
    ///   always its data: the blast meets a fighter, an attack box and
    ///   another projectile by the Hadoken's 1987 rules (`0x433ba`,
    ///   `0x43490`, `0x43476`: an attack box breaks it only outside 2P
    ///   versus on a stage whose `0x434d0` byte is 1), but each breaks it up
    ///   where it was with its own fading frames, and none touches Retsu's
    ///   hold, where the Hadoken's hit and attack-box handlers zero its
    ///   thrower's +0x33 and +0xe.
    pub mod kiai {
        use super::*;

        /// Frames, in order: the stance, the wind-up, the palm held after
        /// the blast leaves it, the wind-up back, the stance.
        pub const STANCE: u8 = 3;
        pub const WIND_UP: u8 = 8;
        pub const PALM_HELD: u8 = 19;
        pub const BACK: u8 = 6;
        pub const SETTLE: u8 = 4;
        pub const DAMAGE: u8 = 14;
        pub const CHIP: u8 = 4;

        /// The Hadoken's projectile template (`0x186dc`), whose layout and
        /// collision flags the blast's template starts from, and its
        /// explosion (`0x186fc`), whose sound (06) the blast's break-up
        /// plays.
        const HADOKEN: u32 = 0x186dc;
        const EXPLOSION: u32 = 0x186fc;
        /// The Hadoken's hit and 2P chip entries, whose score and reactions
        /// the blast's keep.
        const HADOKEN_HIT: u32 = 0x4345e;
        const HADOKEN_CHIP: u32 = 0x43466;
        /// The blast's six approved frames (the maintainer, 2026-11-08), in his
        /// recipe as `kiai_blast_1..6`: two at full strength, four fading.
        pub const FRAMES: [&str; 6] = ["kiai_blast_1", "kiai_blast_2", "kiai_blast_3", "kiai_blast_4", "kiai_blast_5", "kiai_blast_6"];

        /// The blast: its damage entries, its hit record, its effect streams
        /// and templates, and the routine record that spawns it, drawn with
        /// `frames` (his catalogue's [`FRAMES`]). Each frame is one effect
        /// sprite: its pens fit one 32-pixel cell, which the bank holds as a
        /// 2x2 grid of 16-pixel tiles, the same codes a 32-pixel entry
        /// draws. Returns the spawn routine's address.
        pub(in super::super) fn blast(region: &mut Region, rom: &[u8], frames: [PoseGeom; 6]) -> Result<u32, String> {
            if let Some(f) = frames.iter().find(|f| f.attr & 0x400 != 0 || f.rows != 1 || f.cols != 1) {
                return Err(format!("a blast frame ({:#06x}) is not one 32-pixel cell", f.code));
            }
            let palette = (frames[0].attr & 0xff) as u8;
            let code = |i: usize| frames[i].code;
            let entry = |from: u32, damage: u8| -> [u8; 8] {
                let mut e: [u8; 8] = std::array::from_fn(|i| rom_b(rom, from + i as u32));
                e[1] = damage;
                e
            };
            let hit_entry = region.put(&entry(HADOKEN_HIT, DAMAGE));
            let chip_entry = region.put(&entry(HADOKEN_CHIP, CHIP));
            // A template (0x20 bytes, from the Hadoken's): a 32-pixel entry
            // in his palette, its offsets (+4 y, +6 x), its first frame's
            // hold and step, its collision flags and first-frame sound, its
            // stream, and its handlers against a fighter, an attack box and
            // a projectile (+0x14, +0x18, +0x1c; 0 for none).
            #[allow(clippy::too_many_arguments)]
            let template = |first: u16, at: (i16, i16), hold: u8, dx: u8, flags: u8, sound: u8, stream: u32, handlers: [u32; 3]| -> [u8; 0x20] {
                let mut t: [u8; 0x20] = std::array::from_fn(|i| rom_b(rom, HADOKEN + i as u32));
                t[0..2].copy_from_slice(&first.to_be_bytes());
                t[2..4].copy_from_slice(&(0x0400 | palette as u16).to_be_bytes());
                t[4..6].copy_from_slice(&at.0.to_be_bytes());
                t[6..8].copy_from_slice(&at.1.to_be_bytes());
                t[9] = hold;
                t[0xb] = dx;
                t[0xe] = flags;
                t[0xf] = sound;
                t[0x10..0x14].copy_from_slice(&stream.to_be_bytes());
                for (i, h) in handlers.iter().enumerate() {
                    t[0x14 + 4 * i..0x18 + 4 * i].copy_from_slice(&h.to_be_bytes());
                }
                t
            };
            // An effect stream, 8 bytes a frame (hold, palette, dy, dx, box
            // half height and width, tile), then gone (0xfd).
            let stream = |region: &mut Region, steps: &[(u8, u8, u8, usize)]| -> u32 {
                let at = region.here();
                for &(hold, dx, half, frame) in steps {
                    region.put(&[hold, palette, 0, dx, half, half]);
                    region.put(&code(frame).to_be_bytes());
                }
                region.end();
                at
            };
            // When something stops it the blast breaks up where it was: its
            // last three fading frames, standing still, hitting nothing,
            // with the explosion's sound. A transform (0x42eb4) centres the
            // new template on the old one's centre plus its offsets, so these
            // are 0: the break-up plays where the blast was.
            let impact_stream = stream(region, &[(2, 0, 0, 4), (3, 0, 0, 5)]);
            let impact = region.put(&template(code(3), (0, 0), 2, 0, 0, rom_b(rom, EXPLOSION + 0xf), impact_stream, [0; 3]));
            // Its handlers' records: against a fighter (kind, flags (bit 0:
            // cut the thrower's hold short; off), 0, 0, hit entry, chip
            // entry, impact template), an attack box and a projectile (kind,
            // 0, 0, 0, impact template).
            let hit = region.put(&[ROUTINE_EFFECT_HIT, 0, 0, 0]);
            region.put_l(hit_entry);
            region.put_l(chip_entry);
            region.put_l(impact);
            let vs_attack = region.put(&[ROUTINE_EFFECT_VS_ATTACK, 0, 0, 0]);
            region.put_l(impact);
            let vs_projectile = region.put(&[ROUTINE_EFFECT_VS_PROJECTILE, 0, 0, 0]);
            region.put_l(impact);
            // The blast (the recipe's "kiai blast": 2 frames each and the
            // last 3): out at full strength, alternating its two full frames
            // at 4 pixels a frame, then breaking up and fading as it slows
            // and its box shrinks from 12 to 6.
            let flight = stream(region, &[(2, 4, 12, 1), (2, 4, 12, 0), (2, 2, 10, 2), (2, 1, 8, 3), (2, 1, 7, 4), (3, 0, 6, 5)]);
            // Out at his palm: the art sits at its cell's top-left, so the
            // cell goes 4 pixels further ahead and 5 lower than the palm
            // point (centre 52 ahead of his grid's centre, 27 up) to put the
            // drawing there. The Hadoken's collision flags and launch sound.
            let flags = rom_b(rom, HADOKEN + 0xe);
            let sound = rom_b(rom, HADOKEN + 0xf);
            let template = region.put(&template(code(0), (-27, 52), 2, 4, flags, sound, flight, [hit, vs_attack, vs_projectile]));
            let spawn = region.put(&[ROUTINE_PROJECTILE, 7, 0, 0]);
            region.put_l(template);
            Ok(spawn)
        }
    }
}

/// Geki (ticket #251): his ROM table's walk-in, stance, walks, crouch, jumps
/// and win (his vanishing); a real standing block in move 4 (his 1987 move
/// 4 is his teleport) and his squat as his crouching block; his claw thrust,
/// front kick and crouching claw as they are; a jab, a light kick, a
/// crouching kick, a sweep and his jumps' attacks from his catalogue, his
/// diving claw from the top of a diagonal jump; the Shuriken on the
/// Hadoken's command and the Teleport Strike on the Tatsumaki's command
/// with a punch.
mod geki {
    use super::*;

    /// His ROM move table (`0x16bfc`, thirteen moves).
    const ROM_TABLE: u32 = 0x16bfc;

    /// His 1987 attacks (the streams his CPU records start).
    const CLAW: u32 = 0xb968; // the claw thrust
    const FRONT_KICK: u32 = 0xbb54;
    const SHURIKEN: u32 = 0xbf98; // the hop and throw
    /// His shuriken throw, which [`ROUTINE_PROJECTILE_BY_STRENGTH`] stands
    /// in for, and its three stars, steepest first (each `0x0e6e`, 4 pixels
    /// a frame ahead and 3, 2, or 1 then 2 down).
    const SHURIKEN_THROW: u32 = 0x4596e;
    const STARS: [u32; 3] = [0x1849c, 0x184bc, 0x184dc];
    /// The throw's sound.
    const STAR_SOUND: u8 = 0x20;
    /// A star's hit on its target (the stars' +0x14 handler): 4 damage and
    /// its reaction (`0x4338e`), no score, no spark.
    const STAR_HIT: u32 = 0x43348;
    /// The entries his specials score with: the Hadoken's for a star, the
    /// Tatsumaki's for the Teleport Strike's claw.
    const HADOKEN_ENTRY: u32 = 0x4345e;
    const TATSUMAKI_ENTRY: u32 = 0x46012;

    /// ROM frames whose poses and boxes his streams reuse.
    const CROUCH_FRAME: u32 = 0xb0b4; // 0x0fc7, his crouch
    const SQUAT_FRAME: u32 = 0xb16e; // 0x0fc4, the squat his jumps leave from: his crouching block
    const CHAMBER_FRAME: u32 = 0xbb7a; // 0x0f08, his front kick's chamber
    /// His crouching claw (stream `0xba38`) from its crouch on: the ROM
    /// stream rises from his half-crouch into it and stands him up after.
    const CROUCHING_CLAW_CROUCH: u32 = 0xba5e;
    const CROUCHING_CLAW_STAND_UP: u32 = 0xbb04;
    const CLAW_WIND_FRAME: u32 = 0xb988; // 0x0ea0, the claw thrust's wind-up
    const CLAW_FRAME: u32 = 0xb9ae; // 0x0ea6, the claw thrust
    const CLAW_BACK_FRAME: u32 = 0xb9ce; // 0x0ea0, drawn back
    /// His teleport's smoke (stream `0xb6a8`): the cloud at its fullest
    /// (`0x11e6`), thinning (`0x1326`, `0x1329`) and its last wisps (`0x132d`).
    const SMOKE_FRAMES: [u32; 4] = [0xb8be, 0xb6ae, 0xb6ce, 0xb6ee];
    /// The smoke's sound (his ROM teleport's, header `+0x12`).
    const SMOKE_SOUND: u8 = 0x21;
    const JUMP_TOP_FRAME: u32 = 0xb18e; // 0x0fc4, the top of his straight jump
    const FALL_FRAME: u32 = 0xb1ee; // 0x0f2c, his straight jump's fall
    const LANDING_FRAME: u32 = 0xb214; // 0x0f04, his half-crouch
    /// His diving claw (stream `0xbc24`): the claw drawn back, and the dive.
    const DIVE_PREP_FRAME: u32 = 0xbca4; // 0x0eec
    const DIVE_FRAME: u32 = 0xbcca; // 0x0fca

    /// His jumps (ROM streams) and where each reaches its top: the apex
    /// call goes before that entry.
    const JUMPS: [(u8, u32, u32); 3] = [(0x09, 0xb148, 0xb1ae), (0x0a, 0xb2a8, 0xb2ee), (0x0b, 0xb3e8, 0xb42e)];

    /// The bosses' 1987 block sound (every boss's ROM move 4 but his).
    const BLOCK_SOUND: u8 = 0xa3;

    /// Boxes for his new poses, as rectangles in each drawing (see
    /// `tests/geki_player.rs`, which checks them against the pens). A kick
    /// held straight out cannot be one rectangle of body: the light kick's
    /// and the diagonal jump kick's body boxes hold his body, his standing
    /// or tucked leg and his thigh, and the shin and foot are the attack.
    pub const BOXES: [(&str, PixelBoxes); 9] = [
        ("standing_block", PixelBoxes { push: [10, 4, 52, 92], head: [31, 1, 45, 12], body: [6, 4, 54, 92], attack: None }),
        ("jab_2", PixelBoxes { push: [8, 4, 49, 92], head: [31, 1, 45, 12], body: [6, 4, 52, 92], attack: Some([64, 12, 85, 23]) }),
        ("light_kick", PixelBoxes { push: [19, 0, 65, 92], head: [20, 0, 33, 12], body: [19, 0, 51, 91], attack: Some([66, 13, 97, 32]) }),
        ("crouching_kick", PixelBoxes { push: [4, 2, 60, 64], head: [22, 0, 36, 12], body: [1, 2, 64, 64], attack: Some([66, 54, 87, 64]) }),
        ("sweep", PixelBoxes { push: [2, 4, 65, 56], head: [29, 0, 44, 12], body: [2, 4, 69, 56], attack: Some([76, 46, 96, 57]) }),
        ("jump_punch", PixelBoxes { push: [6, 0, 46, 64], head: [22, 0, 36, 12], body: [0, 0, 46, 64], attack: Some([54, 11, 72, 21]) }),
        ("jump_kick", PixelBoxes { push: [16, 4, 65, 68], head: [30, 0, 45, 12], body: [4, 2, 56, 68], attack: Some([80, 7, 97, 23]) }),
        ("jump_kick_high", PixelBoxes { push: [6, 2, 59, 76], head: [30, 2, 45, 14], body: [4, 4, 60, 79], attack: Some([72, 3, 86, 20]) }),
        ("jump_kick_down", PixelBoxes { push: [2, 4, 60, 80], head: [31, 5, 46, 17], body: [10, 10, 58, 95], attack: Some([72, 82, 90, 99]) }),
    ];

    fn boxes(name: &str) -> PixelBoxes {
        boxes_named(&BOXES, name)
    }

    pub fn build(rom: &[u8], catalogue: &Catalogue) -> Result<Region, String> {
        let base = table_address(TournamentFighter::Geki);
        let mut region = new_slot(TournamentFighter::Geki);
        // His specials: the Shuriken in the Hadoken's slot on its command
        // (the 1987 one), a projectile whose stars score for themselves; the
        // Teleport Strike in the Tatsumaki's slot on the Tatsumaki's command
        // with a punch (quarter-circle back and punch, as the art pipeline
        // named it), melee on the ground, scoring as the Tatsumaki; no
        // Shoryuken.
        set_special(&mut region, base, 0x1a, SpecialSlot { tracker: 0x1c, kick: false, melee: false, low_in_air: false, score: 0 });
        set_special(&mut region, base, 0x1b, SpecialSlot { tracker: 0x16, kick: false, melee: true, low_in_air: false, score: TATSUMAKI_ENTRY });
        let palette = rom_w(rom, ROM_TABLE + 0x30 + 2) & 0xff;
        let geom = |name: &str| catalogue_pose(catalogue, name, palette);

        // ROM headers kept as they are: the walk-in, stance, walks and win.
        for id in [0x00u8, 0x01, 0x02, 0x03, 0x0c] {
            keep_rom_header(&mut region, rom, ROM_TABLE, base, id);
        }
        let (stance, stance_boxes) = header_pose(rom, ROM_TABLE + 0x30);

        // The held postures, as Ryu's are: still headers (no busy bit, no
        // stream). His standing block is his approved new pose; his crouch
        // and crouching block are ROM poses with their ROM boxes.
        let block = geom("standing_block")?;
        let h = header(&still(block, 1, boxes("standing_block").bytes(&block)).bytes(), block.top_on_ground(), 0x22, 0x04, [BLOCK_SOUND, 0, 0], 0);
        put_header(&mut region, base, 0x04, &h);
        let crouch = rom_pose(rom, CROUCH_FRAME);
        let crouch_boxes = rom_boxes(rom, CROUCH_FRAME);
        for id in [0x05, 0x06, 0x07] {
            let h = header(&still(crouch, 1, crouch_boxes).bytes(), crouch.top_on_ground(), 0x14, id, [0, 0, 0], 0);
            put_header(&mut region, base, id, &h);
        }
        let squat = rom_pose(rom, SQUAT_FRAME);
        let h = header(&still(squat, 1, rom_boxes(rom, SQUAT_FRAME)).bytes(), squat.top_on_ground(), 0x24, 0x08, [BLOCK_SOUND, 0, 0], 0);
        put_header(&mut region, base, 0x08, &h);

        // His 1987 attacks: the header is the stream's first frame, placed
        // where the CPU's stream would put it from his stance; the rest of
        // the stream is `rest` (the ROM's, ending at the CPU follow-up,
        // which ends a player's move, or a bank copy).
        let rom_attack = |region: &mut Region, id: u8, stream: u32, state: u8, rest: u32| {
            let r = rom_record(rom, stream);
            let p = rom_pose(rom, stream);
            let top = stance.top_on_ground() + stance.half().1 - p.half().1 + r[6] as i8 as i32;
            let h = header(&r, top, state, id, [0, 0, 0], rest);
            put_header(region, base, id, &h);
        };
        for (id, stream) in [(0x0f, CLAW), (0x10, CLAW), (0x15, FRONT_KICK), (0x16, FRONT_KICK)] {
            rom_attack(&mut region, id, stream, 0xc2, stream + 0x20);
        }

        // His crouching claw, every strength: from his crouch, as the ROM
        // stream strikes from it, and back into it (the ROM's rise from his
        // half-crouch and its stand-up are a CPU's way in and out of a
        // crouch a player is already holding).
        let claw_rest = copy_rom_stream(
            &mut region,
            rom,
            CROUCHING_CLAW_CROUCH + 0x20,
            CopyHow { stop: Some(CROUCHING_CLAW_STAND_UP), ..CopyHow::default() },
        );
        let mut claw_crouch = rom_record(rom, CROUCHING_CLAW_CROUCH);
        claw_crouch[6] = 0; // the ROM's step down from his half-crouch
        for id in [0x11u8, 0x12, 0x13, 0x1c, 0x1d, 0x1e] {
            let h = header(&claw_crouch, rom_pose(rom, CROUCHING_CLAW_CROUCH).top_on_ground(), 0xc4, id, [0, 0, 0], claw_rest);
            put_header(&mut region, base, id, &h);
        }

        // The jab, light punch (catalogue: stance 4, jab 10, stance 10).
        let jab = geom("jab_2")?;
        let jab_stream = {
            let mut s = Stream::new(&mut region, stance, stance.top_on_ground());
            let at = s.call(routine::SOUND, 0xa1);
            s.hold(jab, jab.top_on_ground(), 10, boxes("jab_2").bytes(&jab), 0xa0, hit::LIGHT_PUNCH);
            s.hold(stance, stance.top_on_ground(), 10, stance_boxes, 0, 0);
            s.end();
            at
        };
        for id in [0x0du8, 0x0e] {
            let h = header(&still(stance, 4, stance_boxes).bytes(), stance.top_on_ground(), 0xc2, id, [0, 0, 0], jab_stream);
            put_header(&mut region, base, id, &h);
        }

        // The light kick (catalogue: stance 4, his front kick's chamber 5,
        // the kick 10, the chamber 5, stance 10).
        let chamber = rom_pose(rom, CHAMBER_FRAME);
        let chamber_boxes = rom_boxes(rom, CHAMBER_FRAME);
        let lk = geom("light_kick")?;
        let light_kick = {
            let mut s = Stream::new(&mut region, stance, stance.top_on_ground());
            let at = s.hold(chamber, chamber.top_on_ground(), 5, chamber_boxes, 0x20, 0);
            s.call(routine::SOUND, 0xa2);
            s.hold(lk, lk.top_on_ground(), 10, boxes("light_kick").bytes(&lk), 0x80, hit::LIGHT_KICK);
            s.hold(chamber, chamber.top_on_ground(), 5, chamber_boxes, 0, 0);
            s.hold(stance, stance.top_on_ground(), 10, stance_boxes, 0, 0);
            s.end();
            at
        };
        let h = header(&still(stance, 4, stance_boxes).bytes(), stance.top_on_ground(), 0xc2, 0x14, [0, 0, 0], light_kick);
        put_header(&mut region, base, 0x14, &h);

        // The crouching light and medium kick (catalogue: crouch 6, kick
        // 14, crouch 14) and the sweep (crouch 6, sweep 16, crouch 16).
        for (id, name, frames, hit_fn) in [(0x17u8, "crouching_kick", 14u8, hit::CROUCHING_KICK), (0x18, "sweep", 16, hit::SWEEP)] {
            let p = geom(name)?;
            let stream = {
                let mut s = Stream::new(&mut region, crouch, crouch.top_on_ground());
                let at = s.call(routine::SOUND, 0xa2);
                s.hold(p, p.top_on_ground(), frames, boxes(name).bytes(&p), 0xa0, hit_fn);
                s.hold(crouch, crouch.top_on_ground(), frames, crouch_boxes, 0, 0);
                s.end();
                at
            };
            let h = header(&still(crouch, 6, crouch_boxes).bytes(), crouch.top_on_ground(), 0xc4, id, [0, 0, 0], stream);
            put_header(&mut region, base, id, &h);
        }

        // The Shuriken: his ROM hop and throw (stream 0xbf98, his CPU's
        // shuriken attack), its timing, poses and boxes as they are, the
        // throw's star chosen by the button's strength. Each star is his ROM
        // star's template as it is but for its hit handler, which runs the
        // ROM's (0x43348) and scores the hit as the Hadoken's.
        let star_hit = region.put(&[ROUTINE_EFFECT_ROM_HIT, 0, 0, 0]);
        region.put_l(STAR_HIT);
        region.put_l(HADOKEN_ENTRY);
        let stars: Vec<u32> = STARS
            .iter()
            .map(|&rom_star| {
                let mut t: [u8; 0x20] = std::array::from_fn(|i| rom_b(rom, rom_star + i as u32));
                t[0x14..0x18].copy_from_slice(&star_hit.to_be_bytes());
                region.put(&t)
            })
            .collect();
        let throw = region.put(&[ROUTINE_PROJECTILE_BY_STRENGTH, STAR_SOUND, 0, 0]);
        for star in stars {
            region.put_l(star);
        }
        let hop = copy_rom_stream(&mut region, rom, SHURIKEN + 0x20, CopyHow { replace: &[(SHURIKEN_THROW, throw)], ..CopyHow::default() });
        rom_attack(&mut region, 0x1a, SHURIKEN, 0xc1, hop);

        // The Teleport Strike (numbers and why at `teleport`): he vanishes
        // in his smoke, reappears in it in front of the opponent and strikes
        // with his claw thrust, all his ROM frames as they are.
        let warp = region.put(&[ROUTINE_TELEPORT, 0]);
        region.put(&teleport::GAP.to_be_bytes());
        let strike = region.call(routine::SOUND, SMOKE_SOUND);
        for frame in SMOKE_FRAMES {
            region.put(&rom_record(rom, frame));
        }
        region.call(warp, 0);
        let mut reappear = rom_record(rom, SMOKE_FRAMES[0]);
        reappear[1] = teleport::REAPPEAR - 1;
        region.put(&reappear);
        region.put(&rom_record(rom, CLAW_WIND_FRAME));
        region.call(routine::SOUND, 0xa2);
        region.put(&rom_record(rom, CLAW_FRAME));
        region.put(&rom_record(rom, CLAW_BACK_FRAME));
        region.put(&still(stance, teleport::SETTLE, stance_boxes).bytes());
        region.end();
        let h = header(&still(stance, teleport::STANCE, stance_boxes).bytes(), stance.top_on_ground(), 0xc2, 0x1b, [0, 0, 0], strike);
        put_header(&mut region, base, 0x1b, &h);

        // The jumps' attacks, from the top of each jump.
        let air = Air {
            rom,
            jump_top: geom("jump_top")?,
            top_boxes: rom_boxes(rom, JUMP_TOP_FRAME),
            fall: rom_pose(rom, FALL_FRAME),
            fall_boxes: rom_boxes(rom, FALL_FRAME),
            landing: rom_pose(rom, LANDING_FRAME),
            landing_boxes: rom_boxes(rom, LANDING_FRAME),
            stance,
            stance_boxes,
        };
        let jp = geom("jump_punch")?;
        let jk = geom("jump_kick")?;
        let jkh = geom("jump_kick_high")?;
        let jkd = geom("jump_kick_down")?;
        let apex = jump_top_at_apexes(rom, ROM_TABLE, &JUMPS, air.jump_top);
        let (neutral, forward, back) = (apex[0], apex[1], apex[2]);
        // Straight up: a hang at the top, then down 2 and 4 a frame; the
        // diagonal jumps: down 3 and 4, 4 pixels a frame forward or back,
        // as his diagonal jumps fall (as Retsu's, whose arcs his match).
        let s20 = air.attack(&mut region, neutral, jp, boxes("jump_punch").bytes(&jp), hit::JUMP_PUNCH, 0, [0, 2, 4], 0xa1)?;
        let s23 = air.attack(&mut region, neutral, jk, boxes("jump_kick").bytes(&jk), hit::JUMP_KICK, 0, [0, 2, 4], 0xa2)?;
        let s26 = air.attack(&mut region, neutral, jkh, boxes("jump_kick_high").bytes(&jkh), hit::JUMP_KICK, 0, [0, 2, 4], 0xa2)?;
        let s24 = air.attack(&mut region, forward, jkd, boxes("jump_kick_down").bytes(&jkd), hit::JUMP_KICK, 4, [0, 3, 4], 0xa2)?;
        let s25 = air.attack(&mut region, back, jkd, boxes("jump_kick_down").bytes(&jkd), hit::JUMP_KICK, -4, [0, 3, 4], 0xa2)?;
        // His diving claw from a diagonal jump's top: his ROM claw drawn
        // back (6 frames) and the dive (8), their poses, boxes, steps and hit
        // as the ROM's (stream 0xbc24), then his fall (7 a frame).
        let s21 = air.rom_strike(&mut region, forward, 0, Some((DIVE_PREP_FRAME, 6, (3, 1))), 0xa8, DIVE_FRAME, 8, (3, 3), 7)?;
        let s22 = air.rom_strike(&mut region, back, 0, Some((DIVE_PREP_FRAME, 6, (3, -1))), 0xa8, DIVE_FRAME, 8, (3, -3), 7)?;

        // The apex record and the jumps. The landing centre: his
        // half-crouch on the ground at the landing call.
        let landing_centre = air.landing.top_on_ground() + air.landing.half().1;
        apex_and_jumps(
            &mut region,
            rom,
            ROM_TABLE,
            base,
            &JUMPS,
            landing_centre,
            [
                [(0x20, s20), (0x20, s20), (0x20, s20)],
                [(0x21, s21), (0x21, s21), (0x21, s21)],
                [(0x22, s22), (0x22, s22), (0x22, s22)],
                [(0x23, s23), (0x23, s23), (0x26, s26)],
                [(0x24, s24), (0x24, s24), (0x24, s24)],
                [(0x25, s25), (0x25, s25), (0x25, s25)],
            ],
        );
        fits(region)
    }

    /// The Teleport Strike's numbers (quarter-circle back and punch; his
    /// catalogue's timing, tuned against
    /// `docs/research/sf1plus-reference-numbers.md`).
    ///
    /// - **Startup 30**: his stance 4, the smoke 16 (the cloud, thinning,
    ///   the last wisps, 4 each), then he is moved, then the cloud he
    ///   reappears in 6 and the claw's wind-up 4; the claw is out on the
    ///   31st frame. Long enough to see coming and walk or block, as a
    ///   teleport that homes on the opponent should be.
    /// - **Unhittable in his smoke**: the ROM smoke frames' flag (`0x08`),
    ///   from the 5th frame to the 26th, as his 1987 teleport is. He passes
    ///   through a projectile, but not through the strike's wind-up.
    /// - **Where he reappears**: [`GAP`] pixels in front of the opponent's
    ///   centre on his own side, the middle of his claw thrust's reach (its
    ///   box `0xb9ae` is 68 to 92 ahead of him, and a body box ~16 each way
    ///   is hit from 52 to 108), so a standing opponent who stays put is hit.
    ///   At the moment of the warp he is never put behind the opponent (he
    ///   reappears on the side he faces from), so the teleport itself does
    ///   not cross up; the opponent can still move or jump over him in the
    ///   10 frames before the claw.
    /// - **The strike** is his ROM claw thrust as it is: active 13, damage 10
    ///   deep and 6 otherwise (`0x464f0`), blocked high or low. It scores as
    ///   the Tatsumaki.
    /// - **Recovery 14** (the claw drawn back 4, stance 10): a guarded
    ///   strike can be punished.
    pub mod teleport {
        pub const STANCE: u8 = 4;
        pub const REAPPEAR: u8 = 6;
        pub const SETTLE: u8 = 10;
        /// How far in front of the opponent's centre he reappears.
        pub const GAP: u16 = 72;
    }
}

/// Joe (ticket #252): his ROM table's walk-in, stance, walks, jumps and
/// win; his ROM standing block as a held posture, and his new crouch (his
/// ROM table has none: its move 5 points inside a frame record, the
/// untranslated `0x44506`) as his crouch and crouching block; his jab,
/// straight, low kick, low kick into a straight and high kick as they are,
/// his ducking body punch as his crouching punch; a crouching kick, a sweep
/// and his jumps' punches and heavy kick from his catalogue, his flying kick
/// struck from the top of each jump; the Rolling Sobat on the Tatsumaki's
/// command and the Dynamite Straight on the Shoryuken's.
mod joe {
    use super::*;

    /// His ROM move table (`0x1734c`, thirteen moves).
    const ROM_TABLE: u32 = 0x1734c;

    /// His 1987 attacks (the streams his CPU records start).
    const JAB: u32 = 0xe3ba;
    const STRAIGHT: u32 = 0xe44a; // the step-in straight
    const LOW_KICK: u32 = 0xe7fc;
    /// His low kick turning into a long straight: two strikes, the kick a
    /// low (it is guarded crouching only, `0x46c9e`), the straight not.
    const LOW_KICK_STRAIGHT: u32 = 0xe706;
    const HIGH_KICK: u32 = 0xe8cc; // the spinning back kick
    /// His ducking body punch (stream `0xe60a`) from its duck on: the ROM
    /// stream ducks from his block's half-crouch into it and stands him up
    /// after.
    const BODY_PUNCH_DUCK: u32 = 0xe62a;
    const BODY_PUNCH_STAND_UP: u32 = 0xe6bc;

    /// ROM frames whose poses and boxes his streams reuse.
    const BLOCK_FRAME: u32 = 0xe370; // 0x176c, his standing block
    const JUMP_TOP_FRAME: u32 = 0xdfce; // 0x1a88, the top of his jump
    const FALL_FRAME: u32 = 0xdfee; // 0x1884, his jump's take-off and fall
    const LANDING_FRAME: u32 = 0xe014; // 0x1708, his half-crouch
    /// His flying kick (streams `0xeade` straight up, `0xe9bc` forward): the
    /// leg drawn up (`0x188c`) and the strike (`0x1944`).
    const FLYING_KICK_FRAME: u32 = 0xeb6a;
    const FORWARD_FLYING_KICK_PREP: u32 = 0xea22;
    const FORWARD_FLYING_KICK_FRAME: u32 = 0xea48;
    /// The ROM frames the specials show as they are: his high kick's first
    /// and last poses (`0x17c8`, `0x19a8`), his body punch's wind-up
    /// (`0x17c4`), his jump's take-off (`0x1884`) and its landing (`0x1708`).
    const HIGH_KICK_STEP_FRAME: u32 = 0xe8cc;
    const HIGH_KICK_RECOVER_FRAME: u32 = 0xe972;
    const DIP_FRAME: u32 = 0xe650;
    const RISE_FRAME: u32 = 0xdf8e;

    /// His jumps (ROM streams) and where each reaches its top: the apex
    /// call goes before that entry.
    const JUMPS: [(u8, u32, u32); 3] = [(0x09, 0xdf88, 0xdfce), (0x0a, 0xe0a8, 0xe0ee), (0x0b, 0xe1e8, 0xe22e)];

    /// The entries his specials score with: the Tatsumaki's for the Rolling
    /// Sobat, the Shoryuken's for the Dynamite Straight (their 1987 slots').
    const TATSUMAKI_ENTRY: u32 = 0x46012;
    const SHORYUKEN_ENTRY: u32 = 0x45f52;

    /// Boxes for his new poses, as rectangles in each drawing (see
    /// `tests/joe_player.rs`, which checks them against the pens). His
    /// Rolling Sobat's kick, held straight out, cannot be one rectangle of
    /// body: its body box holds his body and standing leg, the attack his
    /// shin and foot, and his thigh between is in neither.
    pub const BOXES: [(&str, PixelBoxes); 11] = [
        ("crouch", PixelBoxes { push: [8, 6, 50, 68], head: [28, 0, 43, 12], body: [2, 4, 54, 68], attack: None }),
        ("crouching_kick", PixelBoxes { push: [10, 2, 56, 66], head: [28, 0, 44, 12], body: [2, 4, 56, 66], attack: Some([66, 50, 87, 64]) }),
        ("sweep", PixelBoxes { push: [12, 4, 66, 68], head: [29, 0, 44, 12], body: [10, 2, 74, 66], attack: Some([86, 54, 115, 66]) }),
        ("jump_punch", PixelBoxes { push: [8, 6, 42, 70], head: [20, 0, 35, 12], body: [2, 0, 44, 68], attack: Some([52, 9, 64, 19]) }),
        (
            "jump_punch_down",
            PixelBoxes { push: [8, 6, 42, 70], head: [20, 0, 35, 12], body: [2, 0, 44, 68], attack: Some([40, 44, 51, 54]) },
        ),
        ("jump_kick_high", PixelBoxes { push: [4, 4, 48, 62], head: [1, 0, 17, 12], body: [0, 0, 46, 64], attack: Some([56, 2, 82, 22]) }),
        ("sobat_turn", PixelBoxes { push: [10, 14, 38, 94], head: [19, 0, 35, 14], body: [8, 14, 42, 94], attack: None }),
        ("sobat_kick", PixelBoxes { push: [16, 2, 49, 88], head: [1, 0, 17, 12], body: [8, 0, 40, 92], attack: Some([52, 2, 82, 26]) }),
        ("sobat_follow", PixelBoxes { push: [12, 8, 42, 94], head: [11, 2, 26, 14], body: [8, 2, 44, 94], attack: None }),
        ("sobat_land", PixelBoxes { push: [20, 14, 52, 94], head: [24, 2, 40, 14], body: [18, 12, 54, 94], attack: None }),
        (
            "dynamite_straight",
            PixelBoxes { push: [4, 14, 36, 96], head: [16, 2, 31, 14], body: [2, 2, 38, 96], attack: Some([44, 0, 59, 14]) },
        ),
    ];

    fn boxes(name: &str) -> PixelBoxes {
        boxes_named(&BOXES, name)
    }

    pub fn build(rom: &[u8], catalogue: &Catalogue) -> Result<Region, String> {
        let base = table_address(TournamentFighter::Joe);
        let mut region = new_slot(TournamentFighter::Joe);
        // His specials: the Rolling Sobat in the Tatsumaki's slot on its
        // command (quarter-circle back and kick, as the art pipeline named
        // it), the Dynamite Straight in the Shoryuken's on its command
        // (forward, down, down-forward and punch); both melee on the ground,
        // scoring as the specials whose slots they take. No Hadoken.
        set_special(&mut region, base, 0x1b, SpecialSlot { tracker: 0x16, kick: true, melee: true, low_in_air: false, score: TATSUMAKI_ENTRY });
        set_special(&mut region, base, 0x19, SpecialSlot { tracker: 0x19, kick: false, melee: true, low_in_air: false, score: SHORYUKEN_ENTRY });
        let palette = rom_w(rom, ROM_TABLE + 0x30 + 2) & 0xff;
        let geom = |name: &str| catalogue_pose(catalogue, name, palette);

        // ROM headers kept as they are: the walk-in, stance, walks and win.
        for id in [0x00u8, 0x01, 0x02, 0x03, 0x0c] {
            keep_rom_header(&mut region, rom, ROM_TABLE, base, id);
        }
        let (stance, stance_boxes) = header_pose(rom, ROM_TABLE + 0x30);

        // The held postures, as Ryu's are: still headers (no busy bit, no
        // stream). His standing block is his ROM block's pose with its ROM
        // boxes (his ROM move 4's header draws one cell of it, a 1987 slip
        // the CPU's block shows for its first 4 frames; its stream's record
        // draws the whole pose). His crouch and crouching block are his
        // approved new crouch, which has its fists up before his face.
        let block_sound = rom_b(rom, ROM_TABLE + 4 * 0x30 + 0x10);
        let block = rom_pose(rom, BLOCK_FRAME);
        let h = header(&still(block, 1, rom_boxes(rom, BLOCK_FRAME)).bytes(), block.top_on_ground(), 0x22, 0x04, [block_sound, 0, 0], 0);
        put_header(&mut region, base, 0x04, &h);
        let crouch = geom("crouch")?;
        let crouch_boxes = boxes("crouch").bytes(&crouch);
        for id in [0x05, 0x06, 0x07] {
            let h = header(&still(crouch, 1, crouch_boxes).bytes(), crouch.top_on_ground(), 0x14, id, [0, 0, 0], 0);
            put_header(&mut region, base, id, &h);
        }
        let h = header(&still(crouch, 1, crouch_boxes).bytes(), crouch.top_on_ground(), 0x24, 0x08, [block_sound, 0, 0], 0);
        put_header(&mut region, base, 0x08, &h);

        // His 1987 attacks: the header is the stream's first frame, placed
        // where the CPU's stream would put it from his stance; the rest of
        // the stream stays in the ROM (it ends at the CPU follow-up, which
        // ends a player's move).
        let rom_attack = |region: &mut Region, id: u8, stream: u32| {
            let r = rom_record(rom, stream);
            let p = rom_pose(rom, stream);
            let top = stance.top_on_ground() + stance.half().1 - p.half().1 + r[6] as i8 as i32;
            let h = header(&r, top, 0xc2, id, [0, 0, 0], stream + 0x20);
            put_header(region, base, id, &h);
        };
        for (id, stream) in
            [(0x0du8, JAB), (0x0e, JAB), (0x0f, STRAIGHT), (0x10, STRAIGHT), (0x14, LOW_KICK), (0x15, LOW_KICK_STRAIGHT), (0x16, HIGH_KICK)]
        {
            rom_attack(&mut region, id, stream);
        }

        // His ducking body punch, every crouching punch: from its duck, as
        // the ROM stream strikes from it, and back into his crouch (the
        // ROM's duck from his block's half-crouch and its stand-up are a
        // CPU's way in and out of a crouch a player is already holding).
        let punch_rest = copy_rom_stream(
            &mut region,
            rom,
            BODY_PUNCH_DUCK + 0x20,
            CopyHow { stop: Some(BODY_PUNCH_STAND_UP), ..CopyHow::default() },
        );
        let duck = rom_record(rom, BODY_PUNCH_DUCK);
        for id in [0x11u8, 0x12, 0x13, 0x1c, 0x1d, 0x1e] {
            let h = header(&duck, rom_pose(rom, BODY_PUNCH_DUCK).top_on_ground(), 0xc4, id, [0, 0, 0], punch_rest);
            put_header(&mut region, base, id, &h);
        }

        // The crouching light and medium kick (catalogue: crouch 6, kick
        // 14, crouch 14) and the sweep (crouch 6, sweep 16, crouch 16).
        for (id, name, frames, hit_fn) in [(0x17u8, "crouching_kick", 14u8, hit::CROUCHING_KICK), (0x18, "sweep", 16, hit::SWEEP)] {
            let p = geom(name)?;
            let stream = {
                let mut s = Stream::new(&mut region, crouch, crouch.top_on_ground());
                let at = s.call(routine::SOUND, 0xa2);
                s.hold(p, p.top_on_ground(), frames, boxes(name).bytes(&p), 0xa0, hit_fn);
                s.hold(crouch, crouch.top_on_ground(), frames, crouch_boxes, 0, 0);
                s.end();
                at
            };
            let h = header(&still(crouch, 6, crouch_boxes).bytes(), crouch.top_on_ground(), 0xc4, id, [0, 0, 0], stream);
            put_header(&mut region, base, id, &h);
        }

        // The Rolling Sobat (numbers and why at `sobat`): his ROM high kick's
        // turn played as a hop, the heel whipped round at the top of it.
        let step = rom_pose(rom, HIGH_KICK_STEP_FRAME);
        let turn = geom("sobat_turn")?;
        let kick = geom("sobat_kick")?;
        let follow = geom("sobat_follow")?;
        let land = geom("sobat_land")?;
        let recover = rom_pose(rom, HIGH_KICK_RECOVER_FRAME);
        let sobat_stream = {
            let mut s = Stream::new(&mut region, stance, stance.top_on_ground());
            let at = s.hold(step, step.top_on_ground(), sobat::STEP, rom_boxes(rom, HIGH_KICK_STEP_FRAME), 0x20, 0);
            s.hold(turn, turn.top_on_ground(), sobat::TURN, boxes("sobat_turn").bytes(&turn), 0, 0);
            s.call(routine::SOUND, 0xa2);
            s.hold(kick, kick.top_on_ground(), sobat::KICK, boxes("sobat_kick").bytes(&kick), 0x80, sobat::HIT);
            s.hold(follow, follow.top_on_ground(), sobat::FOLLOW, boxes("sobat_follow").bytes(&follow), 0, 0);
            s.hold(land, land.top_on_ground(), sobat::LAND, boxes("sobat_land").bytes(&land), 0, 0);
            s.hold(recover, recover.top_on_ground(), sobat::RECOVER, rom_boxes(rom, HIGH_KICK_RECOVER_FRAME), 0, 0);
            s.hold(stance, stance.top_on_ground(), sobat::SETTLE, stance_boxes, 0, 0);
            s.end();
            at
        };
        let h = header(&still(stance, sobat::STANCE, stance_boxes).bytes(), stance.top_on_ground(), 0xc2, 0x1b, [0, 0, 0], sobat_stream);
        put_header(&mut region, base, 0x1b, &h);

        // The Dynamite Straight (numbers and why at `dynamite`): he dips and
        // springs up into an upward straight, his ROM poses either side of
        // the new strike.
        let dip = rom_pose(rom, DIP_FRAME);
        let rise = geom("dynamite_rise")?;
        let straight = geom("dynamite_straight")?;
        let landing = rom_pose(rom, LANDING_FRAME);
        let dynamite_stream = {
            let mut s = Stream::new(&mut region, stance, stance.top_on_ground());
            let at = s.hold(dip, dip.top_on_ground(), dynamite::DIP, rom_boxes(rom, DIP_FRAME), 0x20, 0);
            s.hold(rise, rise.top_on_ground(), dynamite::RISE, rom_boxes(rom, RISE_FRAME), 0, 0);
            s.call(routine::SOUND, 0xa2);
            s.hold(straight, straight.top_on_ground(), dynamite::STRAIGHT, boxes("dynamite_straight").bytes(&straight), 0x80, dynamite::HIT);
            s.hold(landing, landing.top_on_ground(), dynamite::LAND, rom_boxes(rom, LANDING_FRAME), 0, 0);
            s.hold(stance, stance.top_on_ground(), dynamite::SETTLE, stance_boxes, 0, 0);
            s.end();
            at
        };
        let h = header(&still(stance, dynamite::STANCE, stance_boxes).bytes(), stance.top_on_ground(), 0xc2, 0x19, [0, 0, 0], dynamite_stream);
        put_header(&mut region, base, 0x19, &h);

        // The jumps' attacks, from the top of each jump.
        let air = Air {
            rom,
            jump_top: geom("jump_top")?,
            top_boxes: rom_boxes(rom, JUMP_TOP_FRAME),
            fall: rom_pose(rom, FALL_FRAME),
            fall_boxes: rom_boxes(rom, FALL_FRAME),
            landing,
            landing_boxes: rom_boxes(rom, LANDING_FRAME),
            stance,
            stance_boxes,
        };
        let jp = geom("jump_punch")?;
        let jpd = geom("jump_punch_down")?;
        let jk = geom("jump_kick")?;
        let jkh = geom("jump_kick_high")?;
        let apex = jump_top_at_apexes(rom, ROM_TABLE, &JUMPS, air.jump_top);
        let (neutral, forward, back) = (apex[0], apex[1], apex[2]);
        // Straight up his jump is low (48 pixels to its top, 16 frames back
        // down): a hang at the top, then down 2 a frame, his ROM fall's
        // first rate, so the attack lands. The diagonal jumps: down 3 and 4,
        // 5 pixels a frame forward or back, as his diagonal jumps move.
        let s20 = air.attack(&mut region, neutral, jp, boxes("jump_punch").bytes(&jp), hit::JUMP_PUNCH, 0, [0, 2, 2], 0xa1)?;
        let s21 = air.attack(&mut region, forward, jpd, boxes("jump_punch_down").bytes(&jpd), hit::JUMP_PUNCH, 5, [0, 3, 4], 0xa1)?;
        let s22 = air.attack(&mut region, back, jpd, boxes("jump_punch_down").bytes(&jpd), hit::JUMP_PUNCH, -5, [0, 3, 4], 0xa1)?;
        // Light and medium kick straight up: his flying kick's strike (the
        // catalogue's `jump_kick`, its ROM tiles), with its ROM boxes and
        // hit, between two tops of his jump.
        let flying_hit = rom_l(rom, FLYING_KICK_FRAME + 0x1c);
        let s23 = air.attack(&mut region, neutral, jk, rom_boxes(rom, FLYING_KICK_FRAME), flying_hit, 0, [0, 2, 2], 0xaa)?;
        let s26 = air.attack(&mut region, neutral, jkh, boxes("jump_kick_high").bytes(&jkh), hit::JUMP_KICK, 0, [0, 2, 2], 0xa2)?;
        // His flying kick from a diagonal jump's top: the leg drawn up (6
        // frames) and the strike (10), their poses, boxes, steps and hit as
        // the ROM's forward one (stream 0xe9bc) strikes on its way down, then
        // his fall (9 a frame, his jumps' last).
        let s24 = air.rom_strike(&mut region, forward, 0, Some((FORWARD_FLYING_KICK_PREP, 6, (4, 5))), 0xaa, FORWARD_FLYING_KICK_FRAME, 10, (5, 3), 9)?;
        let s25 = air.rom_strike(&mut region, back, 0, Some((FORWARD_FLYING_KICK_PREP, 6, (4, -5))), 0xaa, FORWARD_FLYING_KICK_FRAME, 10, (5, -3), 9)?;

        // The apex record and the jumps. The landing centre: his
        // half-crouch on the ground at the landing call.
        let landing_centre = air.landing.top_on_ground() + air.landing.half().1;
        apex_and_jumps(
            &mut region,
            rom,
            ROM_TABLE,
            base,
            &JUMPS,
            landing_centre,
            [
                [(0x20, s20), (0x20, s20), (0x20, s20)],
                [(0x21, s21), (0x21, s21), (0x21, s21)],
                [(0x22, s22), (0x22, s22), (0x22, s22)],
                [(0x23, s23), (0x23, s23), (0x26, s26)],
                [(0x24, s24), (0x24, s24), (0x24, s24)],
                [(0x25, s25), (0x25, s25), (0x25, s25)],
            ],
        );
        fits(region)
    }

    /// The Rolling Sobat's numbers (quarter-circle back and kick; his
    /// catalogue's timing, tuned against
    /// `docs/research/sf1plus-reference-numbers.md`).
    ///
    /// - **Startup 12**: his stance 4, his high kick's first pose 4, the
    ///   turn 4 (12 pixels off the ground); the heel is out on the 13th
    ///   frame, at the top of the hop (20 pixels up). His high kick is out on
    ///   its 8th, so the special costs him a little time for what it adds.
    /// - **Active 10**, his ROM high kick's 10.
    /// - **A hop in place.** His body and its boxes are drawn off the ground
    ///   (4 to 20 pixels up) from the turn to the landing (22 frames), where
    ///   his high kick keeps a foot planted. It does not travel: the kick's
    ///   grid is drawn far ahead of his body (its centre
    ///   35 pixels in front of his), and the 1987 push between fighters goes
    ///   by grid centres, so a kick carried into the opponent puts him on
    ///   the far side, as his ROM high kick would (its grid's centre 32
    ///   ahead) if it moved.
    /// - **Damage 14 deep, 12 otherwise**: his flying kick's hit (`0x46de8`),
    ///   his heaviest 1987 hit, above his high kick's 10 and 8 and under the
    ///   Tatsumaki's 20; Joe hits lightly (`docs/research`, section 2). It
    ///   is guarded high or low, and scores as the Tatsumaki.
    /// - **Recovery 24** (the follow-through 4, the landing 4, his high
    ///   kick's last pose 6, stance 10): a guarded Sobat can be punished up
    ///   close.
    pub mod sobat {
        pub const STANCE: u8 = 4;
        pub const STEP: u8 = 4;
        pub const TURN: u8 = 4;
        pub const KICK: u8 = 10;
        pub const FOLLOW: u8 = 4;
        pub const LAND: u8 = 4;
        pub const RECOVER: u8 = 6;
        pub const SETTLE: u8 = 10;
        /// His forward flying kick's hit.
        pub const HIT: u32 = 0x46de8;
    }

    /// The Dynamite Straight's numbers (forward, down, down-forward and
    /// punch; his catalogue's timing).
    ///
    /// - **Startup 13**: his stance 4, the dip 6 (his body punch's
    ///   wind-up), the rise 3 (his jump's take-off, 8 pixels up); the fist
    ///   is out on the 14th frame. Not a reversal: no frame of it is
    ///   unhittable (Ryu's 1987 Shoryuken is), so it is an anti-air thrown
    ///   early, as a rising punch with a long active time should be.
    /// - **Active 14**, the fist up and forward above his head, so it meets
    ///   a jump-in that the straight's reach of his standing punches misses.
    /// - **Damage 14 deep, 12 otherwise**: his straight-up flying kick's hit
    ///   (`0x46d7a`), as the Sobat's. Guarded high or low; it scores as the
    ///   Shoryuken.
    /// - **Recovery 16** (the landing 6, stance 10).
    pub mod dynamite {
        pub const STANCE: u8 = 4;
        pub const DIP: u8 = 6;
        pub const RISE: u8 = 3;
        pub const STRAIGHT: u8 = 14;
        pub const LAND: u8 = 6;
        pub const SETTLE: u8 = 10;
        /// His straight-up flying kick's hit.
        pub const HIT: u32 = 0x46d7a;
    }
}

/// Mike (ticket #253): his ROM table's walk-in, stance, walks, jumps and
/// win; his ROM standing block and crouch as held postures, and his ROM
/// crouch (both fists up before his face) as his crouching block; his jab
/// on standing LP and LK, his straight on MP and MK, his lunging hook on HP
/// and HK, and his ducking body punch on every crouching punch and kick as
/// they are; his jumps' five punch attacks from his catalogue; the Bison
/// Rush on the Hadoken's command and the Iron Upper on the Shoryuken's.
mod mike {
    use super::*;

    /// His ROM move table (`0x170dc`, thirteen moves).
    const ROM_TABLE: u32 = 0x170dc;

    /// His 1987 attacks (the streams his CPU records start). His jab starts
    /// with a sound call (`0xd8c6`: `0xa1`), then its first frame at
    /// `0xd8cc`.
    const JAB_SOUND: u8 = 0xa1;
    const JAB_FIRST: u32 = 0xd8cc;
    const STRAIGHT: u32 = 0xd956;
    const HOOK: u32 = 0xdb1c;
    /// His ducking punch (`0xda26`) from its crouch on: the ROM stream
    /// steps down from his stance into it and stands him up after.
    const DUCK_PUNCH_CROUCH: u32 = 0xda46;
    const DUCK_PUNCH_STAND_UP: u32 = 0xdaec;

    /// ROM frames whose poses and boxes his streams reuse.
    const BLOCK_FRAME: u32 = 0xd87c; // 0x1648, his standing block
    const CROUCH_FRAME: u32 = 0xd660; // 0x156a, his crouch (both fists up before his face)
    const JUMP_TOP_FRAME: u32 = 0xd71a; // 0x1608, the top of his jump
    const FALL_FRAME: u32 = 0xd77a; // 0x1406, his jump's take-off and fall
    const LANDING_FRAME: u32 = 0xd7c6; // 0x1403, his landing pose
    /// The ROM frames his specials show as they are: his ducking punch's
    /// wind-up (`0x15aa`), his hook's coil (`0x1409`), his straight (`0x14e0`)
    /// and its wind-up (`0x147b`), and his overhand wind-up (`0x14e7`).
    const DUCK_WINDUP_FRAME: u32 = 0xda66;
    const HOOK_WINDUP_FRAME: u32 = 0xdb3c;
    const STRAIGHT_WINDUP_FRAME: u32 = 0xd976;
    const STRAIGHT_FRAME: u32 = 0xd99c;
    const OVERHAND_WINDUP_FRAME: u32 = 0xdb82;

    /// His jumps (ROM streams) and where each reaches its top: the apex
    /// call goes before that entry.
    const JUMPS: [(u8, u32, u32); 3] = [(0x09, 0xd6d4, 0xd73a), (0x0a, 0xd2d2, 0xd338), (0x0b, 0xd506, 0xd56c)];

    /// The entries his specials score with: the Hadoken's for the Bison
    /// Rush, the Shoryuken's for the Iron Upper (their 1987 slots').
    const HADOKEN_ENTRY: u32 = 0x4345e;
    const SHORYUKEN_ENTRY: u32 = 0x45f52;

    /// Boxes for his new poses, as rectangles in each drawing (see
    /// `tests/mike_player.rs`, which checks them against the pens).
    pub const BOXES: [(&str, PixelBoxes); 6] = [
        (
            "jump_punch",
            PixelBoxes { push: [10, 10, 54, 70], head: [27, 1, 41, 13], body: [4, 6, 56, 74], attack: Some([63, 10, 83, 20]) },
        ),
        (
            "jump_punch_down",
            PixelBoxes { push: [8, 6, 52, 66], head: [37, 1, 51, 13], body: [4, 4, 58, 68], attack: Some([61, 38, 79, 52]) },
        ),
        (
            "jump_jab_down",
            PixelBoxes { push: [12, 10, 56, 70], head: [36, 1, 50, 13], body: [6, 8, 60, 72], attack: Some([64, 42, 80, 54]) },
        ),
        (
            "jump_hook",
            PixelBoxes { push: [10, 10, 54, 68], head: [26, 1, 42, 13], body: [4, 6, 58, 72], attack: Some([61, 12, 81, 26]) },
        ),
        (
            "jump_hammer",
            PixelBoxes { push: [4, 10, 38, 64], head: [19, 1, 33, 11], body: [2, 6, 38, 66], attack: Some([38, 36, 53, 50]) },
        ),
        (
            "iron_peak",
            PixelBoxes { push: [6, 30, 36, 110], head: [23, 23, 37, 35], body: [4, 34, 42, 112], attack: Some([34, 0, 51, 44]) },
        ),
    ];

    fn boxes(name: &str) -> PixelBoxes {
        boxes_named(&BOXES, name)
    }

    pub fn build(rom: &[u8], catalogue: &Catalogue) -> Result<Region, String> {
        let base = table_address(TournamentFighter::Mike);
        let mut region = new_slot(TournamentFighter::Mike);
        // His specials: the Bison Rush in the Hadoken's slot on its command
        // (quarter-circle forward and punch, as the art pipeline named it),
        // the Iron Upper in the Shoryuken's on its command (forward, down,
        // down-forward and punch); both melee on the ground, scoring as the
        // specials whose slots they take. No Tatsumaki.
        set_special(&mut region, base, 0x1a, SpecialSlot { tracker: 0x1c, kick: false, melee: true, low_in_air: false, score: HADOKEN_ENTRY });
        set_special(&mut region, base, 0x19, SpecialSlot { tracker: 0x19, kick: false, melee: true, low_in_air: false, score: SHORYUKEN_ENTRY });
        let palette = rom_w(rom, ROM_TABLE + 0x30 + 2) & 0xff;
        let geom = |name: &str| catalogue_pose(catalogue, name, palette);

        // ROM headers kept as they are: the walk-in, stance, walks and win.
        for id in [0x00u8, 0x01, 0x02, 0x03, 0x0c] {
            keep_rom_header(&mut region, rom, ROM_TABLE, base, id);
        }
        let (stance, stance_boxes) = header_pose(rom, ROM_TABLE + 0x30);

        // The held postures, as Ryu's are: still headers (no busy bit, no
        // stream). His standing block is his ROM block's pose with its ROM
        // boxes (his ROM header's push box is shorter than its stream
        // record's). His crouch and crouching block are his ROM crouch, both
        // fists up before his face.
        let block_sound = rom_b(rom, ROM_TABLE + 4 * 0x30 + 0x10);
        let block = rom_pose(rom, BLOCK_FRAME);
        let h = header(&still(block, 1, rom_boxes(rom, BLOCK_FRAME)).bytes(), block.top_on_ground(), 0x22, 0x04, [block_sound, 0, 0], 0);
        put_header(&mut region, base, 0x04, &h);
        let crouch = rom_pose(rom, CROUCH_FRAME);
        let crouch_boxes = rom_boxes(rom, CROUCH_FRAME);
        for id in [0x05, 0x06, 0x07] {
            let h = header(&still(crouch, 1, crouch_boxes).bytes(), crouch.top_on_ground(), 0x14, id, [0, 0, 0], 0);
            put_header(&mut region, base, id, &h);
        }
        let h = header(&still(crouch, 1, crouch_boxes).bytes(), crouch.top_on_ground(), 0x24, 0x08, [block_sound, 0, 0], 0);
        put_header(&mut region, base, 0x08, &h);

        // His 1987 standing attacks: a boxer, every kick button throws a
        // punch too (`art/recipes/mike.json`). His jab (LP, LK) starts with a
        // sound call before its first frame; his straight (MP, MK) and
        // lunging hook (HP, HK) start on their first frame.
        let rom_attack = |region: &mut Region, id: u8, first: u32, sound: u8| {
            let r = rom_record(rom, first);
            let p = rom_pose(rom, first);
            let top = stance.top_on_ground() + stance.half().1 - p.half().1 + r[6] as i8 as i32;
            let h = header(&r, top, 0xc2, id, [sound, 0, 0], first + 0x20);
            put_header(region, base, id, &h);
        };
        for id in [0x0du8, 0x0e, 0x14] {
            rom_attack(&mut region, id, JAB_FIRST, JAB_SOUND);
        }
        for (id, stream) in [(0x0fu8, STRAIGHT), (0x15, STRAIGHT), (0x10, HOOK), (0x16, HOOK)] {
            rom_attack(&mut region, id, stream, 0);
        }

        // His ducking punch (`0xda26`), every crouching punch and crouching
        // kick (its ROM hit `0x4699c` is guarded crouching only, a low): from
        // its crouch, as the ROM stream strikes from it, and back into his
        // crouch (the ROM's step down from his stance and its stand-up are a
        // CPU's way in and out of a crouch a player is already holding).
        let duck_rest = copy_rom_stream(
            &mut region,
            rom,
            DUCK_PUNCH_CROUCH + 0x20,
            CopyHow { stop: Some(DUCK_PUNCH_STAND_UP), ..CopyHow::default() },
        );
        let duck_crouch = rom_record(rom, DUCK_PUNCH_CROUCH);
        for id in [0x11u8, 0x12, 0x13, 0x1c, 0x1d, 0x1e, 0x17, 0x18] {
            let h = header(&duck_crouch, rom_pose(rom, DUCK_PUNCH_CROUCH).top_on_ground(), 0xc4, id, [0, 0, 0], duck_rest);
            put_header(&mut region, base, id, &h);
        }

        // The Bison Rush (numbers and why at `rush`): he ducks, coils his
        // hook and drives across the stage with his straight, all his ROM
        // poses and boxes as they are.
        let duck = rom_pose(rom, DUCK_WINDUP_FRAME);
        let coil = rom_pose(rom, HOOK_WINDUP_FRAME);
        let straight = rom_pose(rom, STRAIGHT_FRAME);
        let windup = rom_pose(rom, STRAIGHT_WINDUP_FRAME);
        let rush_stream = {
            let mut s = Stream::new(&mut region, stance, stance.top_on_ground());
            let at = s.hold(duck, duck.top_on_ground(), rush::DUCK, rom_boxes(rom, DUCK_WINDUP_FRAME), 0x20, 0);
            s.travel(coil, coil.top_on_ground(), rush::COIL - 1, (0, rush::COIL_DX), rom_boxes(rom, HOOK_WINDUP_FRAME), 0, 0);
            let nudge = straight.top_on_ground() - (s.top + coil.half().1 - straight.half().1);
            let top = s.top;
            s.travel(coil, top, 1, (nudge, rush::COIL_DX), rom_boxes(rom, HOOK_WINDUP_FRAME), 0, 0);
            s.call(routine::SOUND, 0xac);
            s.strike(straight, rush::CHARGE, (0, rush::SPEED), rom_boxes(rom, STRAIGHT_FRAME), 0x80, rush::HIT);
            s.hold(windup, windup.top_on_ground(), rush::RECOVER, rom_boxes(rom, STRAIGHT_WINDUP_FRAME), 0, 0);
            s.hold(stance, stance.top_on_ground(), rush::SETTLE, stance_boxes, 0, 0);
            s.end();
            at
        };
        let h = header(&still(stance, rush::STANCE, stance_boxes).bytes(), stance.top_on_ground(), 0xc2, 0x1a, [0, 0, 0], rush_stream);
        put_header(&mut region, base, 0x1a, &h);

        // The Iron Upper (numbers and why at `iron`): he crouches, coils and
        // rises off his feet into a skyward uppercut (`iron_peak`), his ROM
        // poses either side of the new strike.
        let overhand = rom_pose(rom, OVERHAND_WINDUP_FRAME);
        let peak = geom("iron_peak")?;
        let iron_stream = {
            let mut s = Stream::new(&mut region, crouch, crouch.top_on_ground());
            let at = s.hold(duck, duck.top_on_ground(), iron::DUCK, rom_boxes(rom, DUCK_WINDUP_FRAME), 0x20, 0);
            s.hold(overhand, overhand.top_on_ground(), iron::WIND_UP, rom_boxes(rom, OVERHAND_WINDUP_FRAME), 0, 0);
            s.call(routine::SOUND, 0xac);
            s.hold(peak, peak.top_on_ground(), iron::PEAK, boxes("iron_peak").bytes(&peak), 0x80, iron::HIT);
            s.hold(crouch, crouch.top_on_ground(), iron::LAND, crouch_boxes, 0, 0);
            s.hold(stance, stance.top_on_ground(), iron::SETTLE, stance_boxes, 0, 0);
            s.end();
            at
        };
        let h = header(&still(crouch, iron::CROUCH, crouch_boxes).bytes(), crouch.top_on_ground(), 0xc2, 0x19, [0, 0, 0], iron_stream);
        put_header(&mut region, base, 0x19, &h);

        // The jumps' attacks, from the top of each jump: all five are
        // punches from his catalogue, the kick buttons throwing his jumping
        // low jab (LK), jumping hook (MK) and jumping double-fist hammer (HK).
        let air = Air {
            rom,
            jump_top: geom("jump_top")?,
            top_boxes: rom_boxes(rom, JUMP_TOP_FRAME),
            fall: rom_pose(rom, FALL_FRAME),
            fall_boxes: rom_boxes(rom, FALL_FRAME),
            landing: rom_pose(rom, LANDING_FRAME),
            landing_boxes: rom_boxes(rom, LANDING_FRAME),
            stance,
            stance_boxes,
        };
        let jp = geom("jump_punch")?;
        let jpd = geom("jump_punch_down")?;
        let jjd = geom("jump_jab_down")?;
        let jh = geom("jump_hook")?;
        let jhm = geom("jump_hammer")?;
        let apex = jump_top_at_apexes(rom, ROM_TABLE, &JUMPS, air.jump_top);
        let (neutral, forward, back) = (apex[0], apex[1], apex[2]);
        // All three jumps share his vertical arc (88 pixels from his stance
        // centre to the top): a hang at the top, then down 2 and 4 a frame,
        // and 2 pixels a frame forward or 4 back on his diagonal jumps, as
        // his ROM jumps move.
        let s20 = air.attack(&mut region, neutral, jp, boxes("jump_punch").bytes(&jp), hit::JUMP_PUNCH, 0, [0, 2, 4], 0xa1)?;
        let s21 = air.attack(&mut region, forward, jpd, boxes("jump_punch_down").bytes(&jpd), hit::JUMP_PUNCH, 2, [0, 2, 4], 0xa1)?;
        let s22 = air.attack(&mut region, back, jpd, boxes("jump_punch_down").bytes(&jpd), hit::JUMP_PUNCH, -4, [0, 2, 4], 0xa1)?;
        let s23_l = air.attack(&mut region, neutral, jjd, boxes("jump_jab_down").bytes(&jjd), hit::JUMP_KICK, 0, [0, 2, 4], 0xa1)?;
        let s23_m = air.attack(&mut region, neutral, jh, boxes("jump_hook").bytes(&jh), hit::JUMP_KICK, 0, [0, 2, 4], 0xa2)?;
        let s26 = air.attack(&mut region, neutral, jhm, boxes("jump_hammer").bytes(&jhm), hit::JUMP_KICK, 0, [0, 2, 4], 0xac)?;
        let s24_l = air.attack(&mut region, forward, jjd, boxes("jump_jab_down").bytes(&jjd), hit::JUMP_KICK, 2, [0, 2, 4], 0xa1)?;
        let s24_m = air.attack(&mut region, forward, jh, boxes("jump_hook").bytes(&jh), hit::JUMP_KICK, 2, [0, 2, 4], 0xa2)?;
        let s24_h = air.attack(&mut region, forward, jhm, boxes("jump_hammer").bytes(&jhm), hit::JUMP_KICK, 2, [0, 2, 4], 0xac)?;
        let s25_l = air.attack(&mut region, back, jjd, boxes("jump_jab_down").bytes(&jjd), hit::JUMP_KICK, -4, [0, 2, 4], 0xa1)?;
        let s25_m = air.attack(&mut region, back, jh, boxes("jump_hook").bytes(&jh), hit::JUMP_KICK, -4, [0, 2, 4], 0xa2)?;
        let s25_h = air.attack(&mut region, back, jhm, boxes("jump_hammer").bytes(&jhm), hit::JUMP_KICK, -4, [0, 2, 4], 0xac)?;

        let landing_centre = air.landing.top_on_ground() + air.landing.half().1;
        apex_and_jumps(
            &mut region,
            rom,
            ROM_TABLE,
            base,
            &JUMPS,
            landing_centre,
            [
                [(0x20, s20), (0x20, s20), (0x20, s20)],
                [(0x21, s21), (0x21, s21), (0x21, s21)],
                [(0x22, s22), (0x22, s22), (0x22, s22)],
                [(0x23, s23_l), (0x23, s23_m), (0x26, s26)],
                [(0x24, s24_l), (0x24, s24_m), (0x24, s24_h)],
                [(0x25, s25_l), (0x25, s25_m), (0x25, s25_h)],
            ],
        );
        fits(region)
    }

    /// The Bison Rush's numbers (quarter-circle forward and punch; his
    /// catalogue's timing, tuned against
    /// `docs/research/sf1plus-reference-numbers.md` — Balrog's Dash Straight).
    ///
    /// - **Startup 18**: his stance 4, his ducking wind-up 8, his hook's
    ///   coil 6 (stepping 3 pixels a frame forward, 18 in all); the charging
    ///   straight is out on the 19th frame. Slower than his standing
    ///   straight's 10 so the rush across the screen can be seen and guarded.
    /// - **Active 16** at 4 pixels a frame forward (64 pixels during the
    ///   charge, 82 over the move): his ROM straight's pose and boxes carried
    ///   across the stage in one record so a hit clears the active bit for
    ///   the rest of the rush.
    /// - **Damage 16 deep, 10 otherwise**: his lunging hook's hit (`0x46a0a`),
    ///   his heaviest 1987 hit. Guarded high or low; it scores as the
    ///   Hadoken.
    /// - **Recovery 16** (his straight's wind-up 6, stance 10): a guarded
    ///   rush leaves him right in front of the defender to be punished.
    pub mod rush {
        pub const STANCE: u8 = 4;
        pub const DUCK: u8 = 8;
        pub const COIL: u8 = 6;
        pub const COIL_DX: i32 = 3;
        pub const CHARGE: u8 = 16;
        pub const SPEED: i32 = 4;
        pub const RECOVER: u8 = 6;
        pub const SETTLE: u8 = 10;
        /// His lunging hook's hit.
        pub const HIT: u32 = 0x46a0a;
    }

    /// The Iron Upper's numbers (forward, down, down-forward and punch; his
    /// catalogue's timing).
    ///
    /// - **Startup 14**: his crouch 4, his ducking wind-up 6, his overhand
    ///   wind-up 4; the uppercut (`iron_peak`, 9 pixels off the ground) is
    ///   out on the 15th frame. Not unhittable: an anti-air thrown early to
    ///   meet a jump-in with his raised fist.
    /// - **Active 14**, his fist held high above his head.
    /// - **Damage 16 deep, 10 otherwise**: his lunging hook's hit (`0x46a0a`),
    ///   as the Bison Rush's. Guarded high or low; it scores as the
    ///   Shoryuken.
    /// - **Recovery 18** (his crouch 8, stance 10).
    pub mod iron {
        pub const CROUCH: u8 = 4;
        pub const DUCK: u8 = 6;
        pub const WIND_UP: u8 = 4;
        pub const PEAK: u8 = 14;
        pub const LAND: u8 = 8;
        pub const SETTLE: u8 = 10;
        /// His lunging hook's hit.
        pub const HIT: u32 = 0x46a0a;
    }
}

/// Lee (ticket #254): his ROM table's walk-in, stance, walks, jumps and
/// win; his ROM standing block (`0xf236`) with his stance's hurtboxes as a
/// held posture, and his catalogue's crouch and crouching block (`0x05..=0x08`
/// are empty in his ROM table); his five 1987 standing attacks and his ROM
/// sweep (`0xfc8e`) as they are; his crouching punch and crouching light and
/// medium kick from his catalogue; his straight-up HK jump kick (`0xfaec`) and
/// diagonal flying kicks (`0xf788`, `0xf92a`) from their ROM frames and his
/// other jump attacks from his catalogue; the Zesshouohken on the Hadoken's
/// command and the Double Kick on the Tatsumaki's.
mod lee {
    use super::*;

    /// His ROM move table (`0x175bc`, twenty moves; `0x05..=0x08` are empty).
    const ROM_TABLE: u32 = 0x175bc;

    /// His 1987 standing attacks and sweep (the streams his CPU records
    /// start). His palm strike starts with a sound call (`0xf366`: `0xa1`),
    /// then its first frame at `0xf36c`.
    const PALM_SOUND: u8 = 0xa1;
    const PALM_FIRST: u32 = 0xf36c;
    const STRAIGHT: u32 = 0xf3d6;
    const LUNGE_PUNCH: u32 = 0xf4a6;
    const SIDE_KICK: u32 = 0xf5c8;
    const HIGH_KICK: u32 = 0xf678;
    const SWEEP: u32 = 0xfc8e;

    /// ROM frames whose poses and boxes his streams reuse.
    const BLOCK_FRAME: u32 = 0xf236; // 0x1f40, his standing block
    const JUMP_TOP_FRAME: u32 = 0xeeb6; // 0x1e4c, the top of his jump
    const FALL_FRAME: u32 = 0xef16; // 0x1e40, his jump's take-off and fall
    const LANDING_FRAME: u32 = 0xef3c; // 0x1e44, his landing pose
    /// His 1987 jumping kicks: the straight-up kick (`0xfaec`), the forward
    /// flying kick (`0xf788`) and the back flying kick (`0xf92a`).
    const NEUTRAL_KICK_PREP: u32 = 0xfbb2; // 0x1e48
    const NEUTRAL_KICK_FRAME: u32 = 0xfbd8; // 0x1ea8
    const FLYING_KICK_PREP: u32 = 0xf854; // 0x1f0c
    const FORWARD_FLYING_KICK_FRAME: u32 = 0xf874; // 0x1ec0
    const BACK_FLYING_KICK_FRAME: u32 = 0xfa16; // 0x1ec0
    /// The ROM frames his Zesshouohken and Double Kick show as they are.
    const ZESSH_STEP_FRAME: u32 = 0xf55e; // 0x1ccc, his lunge punch's recovery step
    const ZESSH_READY_FRAME: u32 = 0xf57e; // 0x1c60, his lunge punch's settle
    const STEP_FRAME: u32 = 0xf5c8; // 0x1ccc, his side kick's first step
    const SIDE_CHAMBER_FRAME: u32 = 0xf5e8; // 0x1d28, his side kick's chamber
    const SIDE_KICK_FRAME: u32 = 0xf60e; // 0x1d80, his side kick
    const SIDE_CHAMBER_RECOVER_FRAME: u32 = 0xf62e; // 0x1d28, his side kick's retraction
    const HIGH_STEP_FRAME: u32 = 0xf698; // 0x1ccc, his high kick's 16px step forward
    const HIGH_CHAMBER_FRAME: u32 = 0xf6be; // 0x1d2c, his high kick's chamber
    const HIGH_KICK_FRAME: u32 = 0xf6de; // 0x1d86, his high kick
    const HIGH_FOLLOW_FRAME: u32 = 0xf6fe; // 0x1de6, his high kick's follow-through
    const HIGH_LAND_FRAME: u32 = 0xf71e; // 0x1d8c, his high kick's landing
    const HIGH_SETTLE_FRAME: u32 = 0xf73e; // 0x1c60, his high kick's settle

    /// His jumps (ROM streams) and where each reaches its top: the apex
    /// call goes before that entry.
    const JUMPS: [(u8, u32, u32); 3] = [(0x09, 0xee30, 0xeeb6), (0x0a, 0xefb0, 0xf016), (0x0b, 0xf0f0, 0xf156)];

    /// The entries his specials score with: the Hadoken's for the
    /// Zesshouohken, the Tatsumaki's for the Double Kick (their 1987 slots').
    const HADOKEN_ENTRY: u32 = 0x4345e;
    const TATSUMAKI_ENTRY: u32 = 0x46012;

    /// Boxes for his new poses, as rectangles in each drawing (see
    /// `tests/lee_player.rs`, which checks them against the pens).
    pub const BOXES: [(&str, PixelBoxes); 9] = [
        ("crouch", PixelBoxes { push: [6, 14, 44, 64], head: [11, 2, 25, 14], body: [2, 6, 46, 64], attack: None }),
        ("crouching_block", PixelBoxes { push: [6, 14, 44, 64], head: [11, 2, 25, 14], body: [1, 6, 45, 66], attack: None }),
        (
            "crouching_punch",
            PixelBoxes { push: [6, 12, 42, 64], head: [11, 2, 25, 14], body: [1, 6, 45, 64], attack: Some([33, 10, 69, 16]) },
        ),
        (
            "crouching_kick",
            PixelBoxes { push: [6, 14, 42, 64], head: [11, 2, 25, 14], body: [1, 6, 48, 64], attack: Some([48, 48, 71, 62]) },
        ),
        (
            "jump_punch",
            PixelBoxes { push: [2, 14, 48, 54], head: [17, 2, 32, 13], body: [1, 13, 45, 68], attack: Some([45, 15, 65, 24]) },
        ),
        (
            "jump_punch_down",
            PixelBoxes { push: [2, 18, 48, 54], head: [17, 2, 32, 13], body: [1, 13, 46, 68], attack: Some([46, 30, 66, 45]) },
        ),
        (
            "jump_kick",
            PixelBoxes { push: [2, 18, 48, 58], head: [17, 2, 32, 13], body: [1, 13, 49, 60], attack: Some([49, 44, 87, 59]) },
        ),
        ("zessh_gather", PixelBoxes { push: [6, 25, 52, 95], head: [20, 8, 35, 20], body: [5, 24, 54, 96], attack: None }),
        (
            "zessh_strike",
            PixelBoxes { push: [28, 16, 74, 72], head: [47, 2, 62, 14], body: [12, 12, 76, 80], attack: Some([76, 2, 107, 36]) },
        ),
    ];

    fn boxes(name: &str) -> PixelBoxes {
        boxes_named(&BOXES, name)
    }

    pub fn build(rom: &[u8], catalogue: &Catalogue) -> Result<Region, String> {
        let base = table_address(TournamentFighter::Lee);
        let mut region = new_slot(TournamentFighter::Lee);
        // His specials: the Zesshouohken in the Hadoken's slot on its command
        // (quarter-circle forward and punch, as the art pipeline named it),
        // the Double Kick in the Tatsumaki's on its command (quarter-circle
        // back and kick); both melee on the ground, scoring as the specials
        // whose slots they take. No Shoryuken.
        set_special(&mut region, base, 0x1a, SpecialSlot { tracker: 0x1c, kick: false, melee: true, low_in_air: false, score: HADOKEN_ENTRY });
        set_special(&mut region, base, 0x1b, SpecialSlot { tracker: 0x16, kick: true, melee: true, low_in_air: false, score: TATSUMAKI_ENTRY });
        let palette = rom_w(rom, ROM_TABLE + 0x30 + 2) & 0xff;
        let geom = |name: &str| catalogue_pose(catalogue, name, palette);

        // ROM headers kept as they are: the walk-in, stance, walks and win.
        for id in [0x00u8, 0x01, 0x02, 0x03, 0x0c] {
            keep_rom_header(&mut region, rom, ROM_TABLE, base, id);
        }
        let (stance, stance_boxes) = header_pose(rom, ROM_TABLE + 0x30);

        // The held postures, as Ryu's are: still headers (no busy bit, no
        // stream). His standing block is his ROM block's pose (`0xf236`) with
        // his stance's hurtboxes (the ROM block zeroes his head box, #96).
        // His ROM table has no crouch (`0x05..=0x08` are empty), so his
        // crouches and crouching block come from his catalogue.
        let block_sound = rom_b(rom, ROM_TABLE + 4 * 0x30 + 0x10);
        let block = rom_pose(rom, BLOCK_FRAME);
        let mut block_boxes = rom_boxes(rom, BLOCK_FRAME);
        block_boxes[4..12].copy_from_slice(&stance_boxes[4..12]);
        let h = header(&still(block, 1, block_boxes).bytes(), block.top_on_ground(), 0x22, 0x04, [block_sound, 0, 0], 0);
        put_header(&mut region, base, 0x04, &h);
        let crouch = geom("crouch")?;
        let crouch_boxes = boxes("crouch").bytes(&crouch);
        for id in [0x05u8, 0x06, 0x07] {
            let h = header(&still(crouch, 1, crouch_boxes).bytes(), crouch.top_on_ground(), 0x14, id, [0, 0, 0], 0);
            put_header(&mut region, base, id, &h);
        }
        let crouching_block = geom("crouching_block")?;
        let h = header(
            &still(crouching_block, 1, boxes("crouching_block").bytes(&crouching_block)).bytes(),
            crouching_block.top_on_ground(),
            0x24,
            0x08,
            [block_sound, 0, 0],
            0,
        );
        put_header(&mut region, base, 0x08, &h);

        // His five 1987 standing attacks and his ROM sweep (`0xfc8e`), each
        // streaming its ROM records as they are (`art/recipes/lee.json`):
        // palm strike on LP (`0x0d`, `0x0e`), straight on MP (`0x0f`), lunge
        // punch on HP (`0x10`), side kick on LK and MK (`0x14`, `0x15`),
        // stepping high kick on HK (`0x16`), and sweep on crouching HK (`0x18`).
        let rom_attack = |region: &mut Region, id: u8, first: u32, sound: u8| {
            let r = rom_record(rom, first);
            let p = rom_pose(rom, first);
            let top = stance.top_on_ground() + stance.half().1 - p.half().1 + r[6] as i8 as i32;
            let h = header(&r, top, 0xc2, id, [sound, 0, 0], first + 0x20);
            put_header(region, base, id, &h);
        };
        for id in [0x0du8, 0x0e] {
            rom_attack(&mut region, id, PALM_FIRST, PALM_SOUND);
        }
        for (id, stream) in [(0x0fu8, STRAIGHT), (0x10, LUNGE_PUNCH), (0x14, SIDE_KICK), (0x15, SIDE_KICK), (0x16, HIGH_KICK), (0x18, SWEEP)] {
            rom_attack(&mut region, id, stream, 0);
        }

        // The crouching punch, every strength (catalogue: crouch 6, punch 12,
        // crouch 10).
        let cp = geom("crouching_punch")?;
        let cp_stream = {
            let mut s = Stream::new(&mut region, crouch, crouch.top_on_ground());
            let at = s.call(routine::SOUND, 0xa1);
            s.hold(cp, cp.top_on_ground(), 12, boxes("crouching_punch").bytes(&cp), 0xa0, hit::CROUCHING_PUNCH);
            s.hold(crouch, crouch.top_on_ground(), 10, crouch_boxes, 0, 0);
            s.end();
            at
        };
        for id in [0x11u8, 0x12, 0x13, 0x1c, 0x1d, 0x1e] {
            let h = header(&still(crouch, 6, crouch_boxes).bytes(), crouch.top_on_ground(), 0xc4, id, [0, 0, 0], cp_stream);
            put_header(&mut region, base, id, &h);
        }

        // The crouching light and medium kick (catalogue: crouch 6, kick 14,
        // crouch 14).
        let ck = geom("crouching_kick")?;
        let ck_stream = {
            let mut s = Stream::new(&mut region, crouch, crouch.top_on_ground());
            let at = s.call(routine::SOUND, 0xa2);
            s.hold(ck, ck.top_on_ground(), 14, boxes("crouching_kick").bytes(&ck), 0xa0, hit::CROUCHING_KICK);
            s.hold(crouch, crouch.top_on_ground(), 14, crouch_boxes, 0, 0);
            s.end();
            at
        };
        let h = header(&still(crouch, 6, crouch_boxes).bytes(), crouch.top_on_ground(), 0xc4, 0x17, [0, 0, 0], ck_stream);
        put_header(&mut region, base, 0x17, &h);

        // The Zesshouohken (numbers and why at `zesshou`): he gathers his
        // hands at his waist and lunges across the stage with a twin-palm
        // thrust, settling through his lunge punch's recovery poses.
        let gather = geom("zessh_gather")?;
        let strike = geom("zessh_strike")?;
        let step = rom_pose(rom, ZESSH_STEP_FRAME);
        let ready = rom_pose(rom, ZESSH_READY_FRAME);
        let gather_boxes = boxes("zessh_gather").bytes(&gather);
        let nudge = strike.top_on_ground() - (gather.top_on_ground() + gather.half().1 - strike.half().1);
        let origin = pose_origin_record(&mut region, nudge, 0xa1);
        let zesshou_stream = {
            let mut s = Stream::new(&mut region, stance, stance.top_on_ground());
            let at = s.hold(gather, gather.top_on_ground(), zesshou::GATHER - 1, gather_boxes, 0x20, 0);
            let top = s.top;
            s.hold(gather, top, 1, gather_boxes, 0, 0);
            s.change_origin(origin, nudge);
            s.strike(strike, zesshou::STRIKE, (0, zesshou::SPEED), boxes("zessh_strike").bytes(&strike), 0x80, zesshou::HIT);
            s.hold(step, step.top_on_ground(), zesshou::STEP, rom_boxes(rom, ZESSH_STEP_FRAME), 0, 0);
            s.hold(ready, ready.top_on_ground(), zesshou::READY, rom_boxes(rom, ZESSH_READY_FRAME), 0, 0);
            s.hold(stance, stance.top_on_ground(), zesshou::SETTLE, stance_boxes, 0, 0);
            s.end();
            at
        };
        let h = header(&still(stance, zesshou::STANCE, stance_boxes).bytes(), stance.top_on_ground(), 0xc2, 0x1a, [0, 0, 0], zesshou_stream);
        put_header(&mut region, base, 0x1a, &h);

        // The Double Kick (numbers and why at `double_kick`): his side kick
        // and his stepping high kick in one combination (`0xf5c8` into
        // `0xf698..=0xf75e`), his ROM poses, step and boxes as they are at
        // his catalogue's timing.
        let put_rom = |region: &mut Region, frame: u32, frames: u8| -> u32 {
            let mut r = rom_record(rom, frame);
            r[0] = 0;
            r[1] = frames - 1;
            region.put(&r)
        };
        let dk_stream = put_rom(&mut region, SIDE_CHAMBER_FRAME, double_kick::SIDE_CHAMBER_1);
        region.call(routine::SOUND, 0xa2);
        put_rom(&mut region, SIDE_KICK_FRAME, double_kick::SIDE_KICK);
        put_rom(&mut region, SIDE_CHAMBER_RECOVER_FRAME, double_kick::SIDE_CHAMBER_2);
        put_rom(&mut region, HIGH_STEP_FRAME, double_kick::STEP_2);
        put_rom(&mut region, HIGH_CHAMBER_FRAME, double_kick::HIGH_CHAMBER);
        region.call(routine::SOUND, 0xa2);
        put_rom(&mut region, HIGH_KICK_FRAME, double_kick::HIGH_KICK);
        put_rom(&mut region, HIGH_FOLLOW_FRAME, double_kick::HIGH_FOLLOW);
        put_rom(&mut region, HIGH_LAND_FRAME, double_kick::HIGH_LAND);
        put_rom(&mut region, HIGH_SETTLE_FRAME, double_kick::HIGH_SETTLE);
        region.put(&still(stance, double_kick::SETTLE, stance_boxes).bytes());
        region.end();
        let mut step_first = rom_record(rom, STEP_FRAME);
        step_first[0] = 0;
        step_first[1] = double_kick::STEP_1 - 1;
        let h = header(&step_first, rom_pose(rom, STEP_FRAME).top_on_ground(), 0xc2, 0x1b, [0, 0, 0], dk_stream);
        put_header(&mut region, base, 0x1b, &h);

        // The jumps' attacks, from the top of each jump.
        let air = Air {
            rom,
            jump_top: geom("jump_top")?,
            top_boxes: rom_boxes(rom, JUMP_TOP_FRAME),
            fall: rom_pose(rom, FALL_FRAME),
            fall_boxes: rom_boxes(rom, FALL_FRAME),
            landing: rom_pose(rom, LANDING_FRAME),
            landing_boxes: rom_boxes(rom, LANDING_FRAME),
            stance,
            stance_boxes,
        };
        let jp = geom("jump_punch")?;
        let jpd = geom("jump_punch_down")?;
        let jk = geom("jump_kick")?;
        // The neutral CPU header keeps the standing centre for its taller
        // landing/takeoff grid. Player takeoff needs that pose's grounded
        // centre instead; translate this whole arc, including its authored
        // jump-attack apex, by the same origin conversion.
        let neutral_header = ROM_TABLE + 9 * 0x30;
        let (neutral_pose, _) = header_pose(rom, neutral_header);
        let neutral_origin = air.landing.top_on_ground() + air.landing.half().1
            - (neutral_pose.top_on_ground() + neutral_pose.half().1);
        let mut apex = jump_top_at_apexes(rom, ROM_TABLE, &JUMPS, air.jump_top);
        apex[0] += neutral_origin;
        let (neutral, forward, back) = (apex[0], apex[1], apex[2]);
        // His straight-up jump (`0xee30`) rises 44 pixels from his landing
        // centre to its top; his diagonal jumps (`0xefb0`, `0xf0f0`) skip the
        // 8-pixel drop before the top (`0xee76`) and rise 68 pixels, moving 5
        // pixels a frame across.
        let s20 = air.attack(&mut region, neutral, jp, boxes("jump_punch").bytes(&jp), hit::JUMP_PUNCH, 0, [0, 2, 2], 0xa1)?;
        let s21 = air.attack(&mut region, forward, jpd, boxes("jump_punch_down").bytes(&jpd), hit::JUMP_PUNCH, 5, [0, 2, 3], 0xa1)?;
        let s22 = air.attack(&mut region, back, jpd, boxes("jump_punch_down").bytes(&jpd), hit::JUMP_PUNCH, -5, [0, 2, 3], 0xa1)?;
        let s23 = air.attack(&mut region, neutral, jk, boxes("jump_kick").bytes(&jk), hit::JUMP_KICK, 0, [0, 2, 2], 0xa2)?;
        // His three 1987 jump kicks (`0xfaec` on straight-up HK, `0xf788` and
        // `0xf92a` on the diagonal jumps), from the top of the jump: each
        // chambers and strikes for 8 frames as his ROM stream does, then falls
        // the remaining 24 pixels in 3 frames at his jump's 8 pixels a frame.
        let s26 = air.rom_strike(
            &mut region,
            neutral,
            0,
            Some((NEUTRAL_KICK_PREP, 3, (-4, 0))),
            0xa2,
            NEUTRAL_KICK_FRAME,
            8,
            (4, 0),
            8,
        )?;
        let s24 = air.rom_strike(
            &mut region,
            forward,
            3,
            Some((FLYING_KICK_PREP, 1, (4, 5))),
            0xa2,
            FORWARD_FLYING_KICK_FRAME,
            8,
            (5, 5),
            8,
        )?;
        let s25 = air.rom_strike(
            &mut region,
            back,
            3,
            Some((FLYING_KICK_PREP, 1, (4, -5))),
            0xa2,
            BACK_FLYING_KICK_FRAME,
            8,
            (5, -5),
            8,
        )?;

        let landing_centre = air.landing.top_on_ground() + air.landing.half().1;
        apex_and_jumps(
            &mut region,
            rom,
            ROM_TABLE,
            base,
            &JUMPS,
            landing_centre,
            [
                [(0x20, s20), (0x20, s20), (0x20, s20)],
                [(0x21, s21), (0x21, s21), (0x21, s21)],
                [(0x22, s22), (0x22, s22), (0x22, s22)],
                [(0x23, s23), (0x23, s23), (0x26, s26)],
                [(0x24, s24), (0x24, s24), (0x24, s24)],
                [(0x25, s25), (0x25, s25), (0x25, s25)],
            ],
        );
        let neutral_top = (neutral_pose.top_on_ground() + neutral_origin) as u16;
        region.patch(base + bank_header_offset(9) + 4, &neutral_top.to_be_bytes());
        fits(region)
    }

    /// The Zesshouohken's numbers (quarter-circle forward and punch; his
    /// catalogue's timing, tuned against
    /// `docs/research/sf1plus-reference-numbers.md` — Yun's Zesshoubuho).
    ///
    /// - **Startup 12**: his stance 4, the gather (`zessh_gather`) 8; the
    ///   twin-palm thrust (`zessh_strike`) is out on the 13th frame, faster
    ///   than his ROM lunge punch's 19-frame wind-up (`0xf4a6`).
    /// - **Active 16** at 4 pixels a frame forward (64 pixels across the
    ///   stage), in one record so a hit clears the active bit for the rest of
    ///   the lunge.
    /// - **Damage 14 deep, 8 otherwise**: his ROM lunge punch's hit
    ///   (`0x46f32`). Guarded high or low; it scores as the Hadoken.
    /// - **Recovery 18** (his lunge punch's recovery step 4 and settle 4,
    ///   stance 10): a guarded lunge leaves him in front of the defender to
    ///   be punished.
    pub mod zesshou {
        pub const STANCE: u8 = 4;
        pub const GATHER: u8 = 8;
        pub const STRIKE: u8 = 16;
        pub const SPEED: i32 = 4;
        pub const STEP: u8 = 4;
        pub const READY: u8 = 4;
        pub const SETTLE: u8 = 10;
        /// His lunge punch's hit.
        pub const HIT: u32 = 0x46f32;
    }

    /// The Double Kick's numbers (quarter-circle back and kick; his
    /// catalogue's timing).
    ///
    /// - **First kick**: his side kick's step 3 (`0xf5c8`) and chamber 3
    ///   (`0xf5e8`, startup 6, faster than his standing side kick's 8), then
    ///   his side kick (`0xf60e`, hit `0x46fa0`, 12 deep / 8 shallow) active
    ///   for 10 frames.
    /// - **Second kick**: his side kick's retraction 3 (`0xf62e`), his high
    ///   kick's 16-pixel step forward 3 (`0xf698`) and chamber 3 (`0xf6be`),
    ///   then his high kick (`0xf6de`, hit `0x4700e`, 14 deep / 10 shallow)
    ///   active for 10 frames from the 26th frame. Under `true_combos` the
    ///   one-hit-per-move rule stops after whichever kick connects first, so
    ///   the stepping high kick catches an opponent just outside the side
    ///   kick's reach; without `true_combos` each record carries its own
    ///   active bit.
    /// - **Recovery 19** (follow-through 3, landing 3, settle 3, stance 10).
    ///   Guarded high or low; it scores as the Tatsumaki.
    pub mod double_kick {
        pub const STEP_1: u8 = 3;
        pub const SIDE_CHAMBER_1: u8 = 3;
        pub const SIDE_KICK: u8 = 10;
        pub const SIDE_CHAMBER_2: u8 = 3;
        pub const STEP_2: u8 = 3;
        pub const STEP_DX: i32 = 16;
        pub const HIGH_CHAMBER: u8 = 3;
        pub const HIGH_KICK: u8 = 10;
        pub const HIGH_FOLLOW: u8 = 3;
        pub const HIGH_LAND: u8 = 3;
        pub const HIGH_SETTLE: u8 = 3;
        pub const SETTLE: u8 = 10;
        /// His side kick's hit.
        pub const SIDE_HIT: u32 = 0x46fa0;
        /// His high kick's hit.
        pub const HIGH_HIT: u32 = 0x4700e;
    }
}

/// Gen (ticket #255, `SP8`): his ROM table's intro, stance, walks, jumps and
/// win; his standing block from his ROM block's pose as a still posture; his
/// crouch and crouching block from his catalogue (his ROM table has no
/// crouch: `0x05..=0x08` are empty); his standing jab (`0x103b0`, with its
/// dormant strike record `0x103d6` made active), straight (`0x10440`), front
/// kick (`0x104f0`), rolling low kick (`0x10a26`, on crouching HK) and three
/// jump kicks (`0x10884`, `0x10580`, `0x10702`); his standing low kick,
/// crouching punches and kick, and jump punches from his catalogue; the
/// Hyakurenko (`0x1a`) on the Hadoken's command and the Gekiro (`0x19`) on
/// the Shoryuken's command.
mod gen {
    use super::*;

    /// His ROM move table (`0x1782c`, twenty moves; `0x05..=0x08` are empty).
    const ROM_TABLE: u32 = 0x1782c;
    const HADOKEN_ENTRY: u32 = 0x4345e;
    const SHORYUKEN_ENTRY: u32 = 0x45f52;

    /// His 1987 attacks (the streams his CPU records start).
    const JAB_FIRST: u32 = 0x103b0; // 0x20c8, his jab's 2-frame wind-up
    const JAB_STRIKE: u32 = 0x103d6; // 0x2120, his jab's 4-frame strike (flags 0 in ROM, #90)
    const JAB_RETRACT: u32 = 0x103f6; // 0x20c8, his jab's 2-frame retraction
    const JAB_END: u32 = 0x10416; // 0x2000, his jab's 1-frame stance return
    const STRAIGHT: u32 = 0x10440; // standing MP and HP
    const FRONT_KICK: u32 = 0x104f0; // standing MK and HK (first frame 0x212c is his kick chamber)
    const SWEEP: u32 = 0x10a26; // crouching HK (rolling low kick)

    /// ROM frames whose poses and boxes his streams reuse.
    const JUMP_TOP_FRAME: u32 = 0x10000; // 0x22a6, the top of his jump
    const FALL_FRAME: u32 = 0x10060; // 0x224c, rising and falling
    const LANDING_FRAME: u32 = 0x10086; // 0x218c, his half-crouch
    const NEUTRAL_KICK_LOW_FRAME: u32 = 0x10930; // 0x2186, his neutral jump kick's first strike (5f, dy 4)
    const NEUTRAL_KICK_RISE_FRAME: u32 = 0x10950; // 0x2186, 1f dy -8 before the high kick
    const NEUTRAL_KICK_HIGH_FRAME: u32 = 0x10970; // 0x21e0, his neutral HK jump kick's high strike (5f, dy 0) and Gekiro's kick
    const NEUTRAL_KICK_FALL_FRAME: u32 = 0x10990; // 0x21e0, 3f dy 8
    const FLYING_KICK_PREP: u32 = 0x10626; // 0x2368, his diagonal jump kick's 1-frame chamber
    const FORWARD_FLYING_KICK_FRAME: u32 = 0x1064c; // 0x2309, his forward jump kick (hit 0x4745a)
    const BACK_FLYING_KICK_FRAME: u32 = 0x107ce; // 0x2309, his back jump kick (hit 0x474c8)
    const HYAKU_CROSS_FRAME: u32 = 0x10b4e; // 0x21ec, Hyakurenko arms crossed (zero head box in ROM)
    const HYAKU_LOW_FRAME: u32 = 0x10b6e; // 0x22aa, Hyakurenko low gather (+8 dx)
    const HYAKU_SPREAD_FRAME: u32 = 0x10b8e; // 0x21e6, Hyakurenko arms spread
    const HYAKU_RAISE_FRAME: u32 = 0x10bae; // 0x2240, Hyakurenko hands raised
    const HYAKU_SPIN_FRAME: u32 = 0x10bd4; // 0x2246, Hyakurenko first flurry (hit 0x47310)
    const HYAKU_PALM_FRAME: u32 = 0x10bf4; // 0x22a0, Hyakurenko palm thrust (hit 0x47310)
    const HYAKU_SPIN_LONG_FRAME: u32 = 0x10c14; // 0x2246, Hyakurenko sustained flurry (hit 0x47310)
    const HYAKU_RAISE_REC_FRAME: u32 = 0x10c34; // 0x2240, Hyakurenko recovery raise
    const HYAKU_SPREAD_REC_FRAME: u32 = 0x10c54; // 0x21e6, Hyakurenko recovery spread
    const HYAKU_LOW_REC_FRAME: u32 = 0x10c74; // 0x22aa, Hyakurenko recovery step back (-8 dx)
    const HYAKU_CROSS_REC_FRAME: u32 = 0x10c94; // 0x21ec, Hyakurenko recovery cross (zero head box in ROM)

    /// His jumps (ROM streams) and where each reaches its top (`0x22a6`).
    const JUMPS: [(u8, u32, u32); 3] = [(0x09, 0xff9a, 0x10000), (0x0a, 0x100fa, 0x10140), (0x0b, 0x1021a, 0x10260)];

    /// Boxes for his ten new poses, as rectangles in each drawing (see
    /// `tests/gen_player.rs`, which checks them against the pens).
    pub const BOXES: [(&str, PixelBoxes); 10] = [
        ("crouch", PixelBoxes { push: [6, 14, 44, 64], head: [17, 2, 31, 14], body: [1, 14, 49, 64], attack: None }),
        ("crouching_block", PixelBoxes { push: [6, 14, 44, 64], head: [17, 2, 31, 14], body: [1, 14, 50, 64], attack: None }),
        (
            "crouching_punch",
            PixelBoxes { push: [6, 14, 44, 64], head: [17, 2, 31, 14], body: [1, 14, 50, 64], attack: Some([50, 21, 73, 28]) },
        ),
        (
            "crouching_kick",
            PixelBoxes { push: [6, 14, 50, 64], head: [17, 2, 31, 14], body: [1, 14, 52, 64], attack: Some([44, 48, 78, 57]) },
        ),
        (
            "low_kick",
            PixelBoxes { push: [6, 18, 50, 95], head: [17, 2, 31, 14], body: [3, 16, 46, 95], attack: Some([42, 52, 79, 60]) },
        ),
        (
            "jump_punch",
            PixelBoxes { push: [16, 12, 54, 62], head: [32, 2, 46, 13], body: [4, 6, 58, 62], attack: Some([56, 14, 85, 21]) },
        ),
        (
            "jump_punch_down",
            PixelBoxes { push: [16, 12, 54, 62], head: [32, 2, 46, 13], body: [4, 6, 58, 62], attack: Some([56, 37, 76, 46]) },
        ),
        ("gekiro_over", PixelBoxes { push: [14, 28, 70, 74], head: [25, 14, 39, 26], body: [12, 20, 74, 76], attack: None }),
        ("gekiro_flip", PixelBoxes { push: [10, 14, 52, 64], head: [19, 63, 31, 71], body: [8, 12, 54, 66], attack: None }),
        ("gekiro_round", PixelBoxes { push: [14, 14, 62, 52], head: [58, 33, 72, 44], body: [12, 10, 66, 56], attack: None }),
    ];

    fn boxes(name: &str) -> PixelBoxes {
        boxes_named(&BOXES, name)
    }

    pub fn build(rom: &[u8], catalogue: &Catalogue) -> Result<Region, String> {
        let base = table_address(TournamentFighter::Gen);
        let mut region = new_slot(TournamentFighter::Gen);
        // His specials: the Gekiro in the Shoryuken's slot on its command
        // (forward, down, down-forward and punch), the Hyakurenko in the
        // Hadoken's on its command (quarter-circle forward and punch); both
        // melee on the ground, scoring as the specials whose slots they take.
        // No Tatsumaki.
        set_special(&mut region, base, 0x19, SpecialSlot { tracker: 0x19, kick: false, melee: true, low_in_air: false, score: SHORYUKEN_ENTRY });
        set_special(&mut region, base, 0x1a, SpecialSlot { tracker: 0x1c, kick: false, melee: true, low_in_air: false, score: HADOKEN_ENTRY });
        let palette = rom_w(rom, ROM_TABLE + 0x30 + 2) & 0xff;
        let geom = |name: &str| catalogue_pose(catalogue, name, palette);

        // ROM headers kept as they are: the walk-in, stance, walks and win.
        for id in [0x00u8, 0x01, 0x02, 0x03, 0x0c] {
            keep_rom_header(&mut region, rom, ROM_TABLE, base, id);
        }
        let (stance, stance_boxes) = header_pose(rom, ROM_TABLE + 0x30);

        // The held postures, as Ryu's are: still headers (no busy bit, no
        // stream). His standing block is his ROM block header's pose and
        // boxes (`0x200c`); his ROM table has no crouch (`0x05..=0x08` are
        // empty), so his crouches and crouching block come from his catalogue.
        let block_sound = rom_b(rom, ROM_TABLE + 4 * 0x30 + 0x10);
        let (block, block_boxes) = header_pose(rom, ROM_TABLE + 4 * 0x30);
        let h = header(&still(block, 1, block_boxes).bytes(), block.top_on_ground(), 0x22, 0x04, [block_sound, 0, 0], 0);
        put_header(&mut region, base, 0x04, &h);
        let crouch = geom("crouch")?;
        let crouch_boxes = boxes("crouch").bytes(&crouch);
        for id in [0x05u8, 0x06, 0x07] {
            let h = header(&still(crouch, 1, crouch_boxes).bytes(), crouch.top_on_ground(), 0x14, id, [0, 0, 0], 0);
            put_header(&mut region, base, id, &h);
        }
        let crouching_block = geom("crouching_block")?;
        let h = header(
            &still(crouching_block, 1, boxes("crouching_block").bytes(&crouching_block)).bytes(),
            crouching_block.top_on_ground(),
            0x24,
            0x08,
            [block_sound, 0, 0],
            0,
        );
        put_header(&mut region, base, 0x08, &h);

        // His standing jab (`0x103b0`, on LP `0x0d` and `0x0e`): in the 1987
        // ROM its strike record (`0x103d6`, pose `0x2120`, hit `0x47234`)
        // carries its attack box and hit function with `flags = 0x00`, so the
        // box never goes live (`docs/reference/rom-box-audit.md`, #90). Copy
        // the stream with the wind-up bit (`0x20`) on `0x103b0` and the active
        // bit (`0x80`) on `0x103d6`.
        let jab_stream = region.call(routine::SOUND, 0xa1);
        let mut jab_strike = rom_record(rom, JAB_STRIKE);
        jab_strike[0x1a] = 0x80;
        region.put(&jab_strike);
        region.put(&rom_record(rom, JAB_RETRACT));
        region.put(&rom_record(rom, JAB_END));
        region.end();
        let mut jab_first = rom_record(rom, JAB_FIRST);
        jab_first[0x1a] = 0x20;
        for id in [0x0du8, 0x0e] {
            let h = header(&jab_first, rom_pose(rom, JAB_FIRST).top_on_ground(), 0xc2, id, [0, 0, 0], jab_stream);
            put_header(&mut region, base, id, &h);
        }

        // His 1987 straight (`0x10440`, on MP `0x0f` and HP `0x10`), front
        // kick (`0x104f0`, on MK `0x15` and HK `0x16`), and rolling low kick
        // (`0x10a26`, on crouching HK `0x18`), streamed from the ROM as they
        // are (`art/recipes/gen.json`).
        let rom_attack = |region: &mut Region, id: u8, first: u32| {
            let r = rom_record(rom, first);
            let p = rom_pose(rom, first);
            let top = stance.top_on_ground() + stance.half().1 - p.half().1 + r[6] as i8 as i32;
            let h = header(&r, top, 0xc2, id, [0, 0, 0], first + 0x20);
            put_header(region, base, id, &h);
        };
        for (id, stream) in [(0x0fu8, STRAIGHT), (0x10, STRAIGHT), (0x15, FRONT_KICK), (0x16, FRONT_KICK), (0x18, SWEEP)] {
            rom_attack(&mut region, id, stream);
        }

        // His standing light kick (`0x14`, catalogue: stance 6, kick_chamber
        // 6, low_kick 14, kick_chamber 6).
        let chamber = rom_pose(rom, FRONT_KICK);
        let chamber_boxes = rom_boxes(rom, FRONT_KICK);
        let lk = geom("low_kick")?;
        let lk_stream = {
            let mut s = Stream::new(&mut region, stance, stance.top_on_ground());
            let at = s.hold(chamber, chamber.top_on_ground(), 6, chamber_boxes, 0x20, 0);
            s.call(routine::SOUND, 0xa2);
            s.hold(lk, lk.top_on_ground(), 14, boxes("low_kick").bytes(&lk), 0x80, hit::LIGHT_KICK);
            s.hold(chamber, chamber.top_on_ground(), 6, chamber_boxes, 0, 0);
            s.end();
            at
        };
        let h = header(&still(stance, 6, stance_boxes).bytes(), stance.top_on_ground(), 0xc2, 0x14, [0, 0, 0], lk_stream);
        put_header(&mut region, base, 0x14, &h);

        // The crouching punch, every strength (catalogue: crouch 6, punch 12,
        // crouch 10).
        let cp = geom("crouching_punch")?;
        let cp_stream = {
            let mut s = Stream::new(&mut region, crouch, crouch.top_on_ground());
            let at = s.call(routine::SOUND, 0xa1);
            s.hold(cp, cp.top_on_ground(), 12, boxes("crouching_punch").bytes(&cp), 0xa0, hit::CROUCHING_PUNCH);
            s.hold(crouch, crouch.top_on_ground(), 10, crouch_boxes, 0, 0);
            s.end();
            at
        };
        for id in [0x11u8, 0x12, 0x13, 0x1c, 0x1d, 0x1e] {
            let h = header(&still(crouch, 6, crouch_boxes).bytes(), crouch.top_on_ground(), 0xc4, id, [0, 0, 0], cp_stream);
            put_header(&mut region, base, id, &h);
        }

        // The crouching light and medium kick (catalogue: crouch 6, kick 14,
        // crouch 14).
        let ck = geom("crouching_kick")?;
        let ck_stream = {
            let mut s = Stream::new(&mut region, crouch, crouch.top_on_ground());
            let at = s.call(routine::SOUND, 0xa2);
            s.hold(ck, ck.top_on_ground(), 14, boxes("crouching_kick").bytes(&ck), 0xa0, hit::CROUCHING_KICK);
            s.hold(crouch, crouch.top_on_ground(), 14, crouch_boxes, 0, 0);
            s.end();
            at
        };
        let h = header(&still(crouch, 6, crouch_boxes).bytes(), crouch.top_on_ground(), 0xc4, 0x17, [0, 0, 0], ck_stream);
        put_header(&mut region, base, 0x17, &h);

        // The Hyakurenko (numbers and why at `hyakurenko`): his 1987 rapid
        // hand-stab flurry (`0x10b4e..=0x10c94`) at his catalogue's timing,
        // with his stance's head box on `0x10b4e` and `0x10c94` (which zero
        // their head box in the ROM).
        let put_hyaku = |region: &mut Region, frame: u32, frames: u8| -> u32 {
            let mut r = rom_record(rom, frame);
            r[0] = 0;
            r[1] = frames - 1;
            if r[0xa + 4..0xa + 8] == [0, 0, 0, 0] {
                r[0xa + 4..0xa + 8].copy_from_slice(&stance_boxes[4..8]);
            }
            region.put(&r)
        };
        let hyaku_stream = put_hyaku(&mut region, HYAKU_LOW_FRAME, hyakurenko::LOW_1);
        put_hyaku(&mut region, HYAKU_SPREAD_FRAME, hyakurenko::SPREAD_1);
        put_hyaku(&mut region, HYAKU_RAISE_FRAME, hyakurenko::RAISE_1);
        region.call(routine::SOUND, 0xa2);
        put_hyaku(&mut region, HYAKU_SPIN_FRAME, hyakurenko::SPIN_1);
        put_hyaku(&mut region, HYAKU_PALM_FRAME, hyakurenko::PALM);
        put_hyaku(&mut region, HYAKU_SPIN_LONG_FRAME, hyakurenko::SPIN_2);
        put_hyaku(&mut region, HYAKU_RAISE_REC_FRAME, hyakurenko::RAISE_2);
        put_hyaku(&mut region, HYAKU_SPREAD_REC_FRAME, hyakurenko::SPREAD_2);
        put_hyaku(&mut region, HYAKU_LOW_REC_FRAME, hyakurenko::LOW_2);
        put_hyaku(&mut region, HYAKU_CROSS_REC_FRAME, hyakurenko::CROSS_2);
        region.put(&still(stance, hyakurenko::SETTLE, stance_boxes).bytes());
        region.end();
        let mut hyaku_first = rom_record(rom, HYAKU_CROSS_FRAME);
        hyaku_first[0] = 0;
        hyaku_first[1] = hyakurenko::CROSS_1 - 1;
        hyaku_first[0xa + 4..0xa + 8].copy_from_slice(&stance_boxes[4..8]);
        let h = header(&hyaku_first, rom_pose(rom, HYAKU_CROSS_FRAME).top_on_ground(), 0xc2, 0x1a, [0, 0, 0], hyaku_stream);
        put_header(&mut region, base, 0x1a, &h);

        // The Gekiro (numbers and why at `gekiro`): from a crouch he springs
        // into an upward flying kick (`gekiro_kick`, his ROM `0x21e0`),
        // somersaults backward (`gekiro_over`, `gekiro_flip`, `gekiro_round`)
        // and drops (`gekiro_fall`) back to his crouch and stance.
        let gk = geom("gekiro_kick")?;
        let go = geom("gekiro_over")?;
        let gf = geom("gekiro_flip")?;
        let gr = geom("gekiro_round")?;
        let gfall = geom("gekiro_fall")?;
        let gekiro_stream = {
            let mut s = Stream::new(&mut region, crouch, crouch.top_on_ground());
            let at = s.call(routine::SOUND, 0xb4);
            s.strike(gk, gekiro::KICK, (gekiro::RISE_DY, 0), rom_boxes(rom, NEUTRAL_KICK_HIGH_FRAME), 0x80, gekiro::HIT);
            s.hold(go, go.top_on_ground(), gekiro::OVER, boxes("gekiro_over").bytes(&go), 0, 0);
            s.hold(gf, gf.top_on_ground(), gekiro::FLIP, boxes("gekiro_flip").bytes(&gf), 0, 0);
            s.hold(gr, gr.top_on_ground(), gekiro::ROUND, boxes("gekiro_round").bytes(&gr), 0, 0);
            s.hold(gfall, gfall.top_on_ground(), gekiro::FALL, rom_boxes(rom, FALL_FRAME), 0, 0);
            s.hold(crouch, crouch.top_on_ground(), gekiro::CROUCH_LAND, crouch_boxes, 0, 0);
            s.hold(stance, stance.top_on_ground(), gekiro::SETTLE, stance_boxes, 0, 0);
            s.end();
            at
        };
        let mut gekiro_first = still(crouch, gekiro::CROUCH, crouch_boxes);
        gekiro_first.flags = 0x20;
        let h = header(&gekiro_first.bytes(), crouch.top_on_ground(), 0xc2, 0x19, [0, 0, 0], gekiro_stream);
        put_header(&mut region, base, 0x19, &h);

        // The jumps' attacks, from the top of each jump.
        let air = Air {
            rom,
            jump_top: geom("jump_top")?,
            top_boxes: rom_boxes(rom, JUMP_TOP_FRAME),
            fall: rom_pose(rom, FALL_FRAME),
            fall_boxes: rom_boxes(rom, FALL_FRAME),
            landing: rom_pose(rom, LANDING_FRAME),
            landing_boxes: rom_boxes(rom, LANDING_FRAME),
            stance,
            stance_boxes,
        };
        let jp = geom("jump_punch")?;
        let jpd = geom("jump_punch_down")?;
        let jk = geom("jump_kick")?;
        let apex = jump_top_at_apexes(rom, ROM_TABLE, &JUMPS, air.jump_top);
        let (neutral, forward, back) = (apex[0], apex[1], apex[2]);
        // His straight-up jump (`0xff9a`) rises 60 pixels from his landing
        // centre to its top; his diagonal jumps (`0x100fa`, `0x1021a`) skip
        // the 8-pixel dip (`0xffc0`) and rise 68 pixels, moving 5 pixels a
        // frame across.
        let s20 = air.attack(&mut region, neutral, jp, boxes("jump_punch").bytes(&jp), hit::JUMP_PUNCH, 0, [0, 2, 2], 0xa1)?;
        let s21 = air.attack(&mut region, forward, jpd, boxes("jump_punch_down").bytes(&jpd), hit::JUMP_PUNCH, 5, [0, 2, 3], 0xa1)?;
        let s22 = air.attack(&mut region, back, jpd, boxes("jump_punch_down").bytes(&jpd), hit::JUMP_PUNCH, -5, [0, 2, 3], 0xa1)?;
        // His neutral light and medium jump kick (`0x23`): his first aerial
        // kick pose (`0x2186`, the catalogue's `jump_kick`) with its ROM boxes
        // and hit (`0x473ec`) at `[6, 14, 8]`.
        let neutral_kick_hit = rom_l(rom, NEUTRAL_KICK_LOW_FRAME + 0x1c);
        let s23 = air.attack(&mut region, neutral, jk, rom_boxes(rom, NEUTRAL_KICK_LOW_FRAME), neutral_kick_hit, 0, [0, 2, 2], 0xb4)?;
        // His neutral heavy jump kick (`0x26`, ROM stream `0x10884`): from the
        // top of the jump (`0x1090a`), 1 frame of wind-up, then his two-stage
        // kick (`0x10930..=0x10990`, 14 active frames descending 36 pixels)
        // and 3 frames of his falling pose (`0x109b0`, 24 pixels at 8 a frame)
        // into the standard player landing.
        let s26 = {
            let mut s = Stream::new(&mut region, air.jump_top, neutral);
            let at = s.hold(air.jump_top, neutral, 1, air.top_boxes, 0x20, 0);
            s.call(routine::SOUND, 0xb4);
            s.strike(rom_pose(rom, NEUTRAL_KICK_LOW_FRAME), 5, (4, 0), rom_boxes(rom, NEUTRAL_KICK_LOW_FRAME), 0x80, neutral_kick_hit);
            s.strike(rom_pose(rom, NEUTRAL_KICK_RISE_FRAME), 1, (-8, 0), rom_boxes(rom, NEUTRAL_KICK_RISE_FRAME), 0x80, neutral_kick_hit);
            s.strike(rom_pose(rom, NEUTRAL_KICK_HIGH_FRAME), 5, (0, 0), rom_boxes(rom, NEUTRAL_KICK_HIGH_FRAME), 0x80, neutral_kick_hit);
            s.strike(rom_pose(rom, NEUTRAL_KICK_FALL_FRAME), 3, (8, 0), rom_boxes(rom, NEUTRAL_KICK_FALL_FRAME), 0x80, neutral_kick_hit);
            air.land(&mut s, 0, 8)?;
            at
        };
        // His diagonal jump kicks (`0x10580` forward, `0x10702` back): each
        // chambers for 1 frame and strikes for 8 frames (`0x2309`), then falls
        // the remaining 24 pixels in 3 frames at 8 pixels a frame.
        let s24 = air.rom_strike(
            &mut region,
            forward,
            3,
            Some((FLYING_KICK_PREP, 1, (4, 5))),
            0xb4,
            FORWARD_FLYING_KICK_FRAME,
            8,
            (5, 5),
            8,
        )?;
        let s25 = air.rom_strike(
            &mut region,
            back,
            3,
            Some((FLYING_KICK_PREP, 1, (4, -5))),
            0xb4,
            BACK_FLYING_KICK_FRAME,
            8,
            (5, -5),
            8,
        )?;

        let landing_centre = air.landing.top_on_ground() + air.landing.half().1;
        apex_and_jumps(
            &mut region,
            rom,
            ROM_TABLE,
            base,
            &JUMPS,
            landing_centre,
            [
                [(0x20, s20), (0x20, s20), (0x20, s20)],
                [(0x21, s21), (0x21, s21), (0x21, s21)],
                [(0x22, s22), (0x22, s22), (0x22, s22)],
                [(0x23, s23), (0x23, s23), (0x26, s26)],
                [(0x24, s24), (0x24, s24), (0x24, s24)],
                [(0x25, s25), (0x25, s25), (0x25, s25)],
            ],
        );
        fits(region)
    }

    /// The Hyakurenko's numbers (quarter-circle forward and punch; his
    /// catalogue's timing, from his 1987 rapid hand-stab stream `0x10b4e`).
    ///
    /// - **Startup 15**: arms crossed 3 (`0x10b4e`), low gather 3 (`0x10b6e`,
    ///   stepping 8 pixels in), arms spread 6 (`0x10b8e`), hands raised 3
    ///   (`0x10bae`); the flurry is out on the 16th frame, 4 frames faster
    ///   than his 1987 CPU stream's 20-frame startup.
    /// - **Active 22**: first flurry 3 (`0x10bd4`), palm thrust 3 (`0x10bf4`),
    ///   sustained flurry 16 (`0x10c14`), all with his 1987 flurry hit
    ///   (`0x47310`, 14 deep / 12 shallow, guarded crouching as its 1987 low
    ///   stab boxes sit at `dy = -16`). Under `true_combos` the one-hit-per-move
    ///   rule stops after whichever stab connects first; without `true_combos`
    ///   each record carries its own active bit.
    /// - **Recovery 22** (raise 3, spread 3, step back 3 returning the 8
    ///   pixels, cross 3, stance 10). Scores as the Hadoken.
    pub mod hyakurenko {
        pub const CROSS_1: u8 = 3;
        pub const LOW_1: u8 = 3;
        pub const SPREAD_1: u8 = 6;
        pub const RAISE_1: u8 = 3;
        pub const SPIN_1: u8 = 3;
        pub const PALM: u8 = 3;
        pub const SPIN_2: u8 = 16;
        pub const RAISE_2: u8 = 3;
        pub const SPREAD_2: u8 = 3;
        pub const LOW_2: u8 = 3;
        pub const CROSS_2: u8 = 3;
        pub const SETTLE: u8 = 10;
        /// His 1987 flurry's hit.
        pub const HIT: u32 = 0x47310;
    }

    /// The Gekiro's numbers (forward, down, down-forward and punch; his
    /// catalogue's timing, tuned against
    /// `docs/research/sf1plus-reference-numbers.md`).
    ///
    /// - **Startup 4**: his crouch (`crouch`, 4 frames); the upward flying
    ///   kick (`gekiro_kick`, his ROM `0x21e0` 40 pixels off the ground) is
    ///   out on the 5th frame, matching Ryu's Shoryuken startup.
    /// - **Active 8** on `gekiro_kick` (hit `0x473ec`, 12 deep / 10 shallow),
    ///   guarded high or low, scoring as the Shoryuken.
    /// - **Recovery 45**: the backward somersault (`gekiro_over` 5 at 74
    ///   pixels up, `gekiro_flip` 5 at 91 pixels up, `gekiro_round` 5 at 92
    ///   pixels up), the drop (`gekiro_fall` 8 at 24 pixels up), his crouch
    ///   landing 12, and his stance 10.
    pub mod gekiro {
        pub const CROUCH: u8 = 4;
        pub const KICK: u8 = 8;
        pub const RISE_DY: i32 = -7;
        pub const OVER: u8 = 5;
        pub const FLIP: u8 = 5;
        pub const ROUND: u8 = 5;
        pub const FALL: u8 = 8;
        pub const CROUCH_LAND: u8 = 12;
        pub const SETTLE: u8 = 10;
        /// His neutral flying kick's hit.
        pub const HIT: u32 = 0x473ec;
    }
}

/// Birdie (ticket #256, `SP9`): his ROM table's intro, stance, walks, jumps
/// and win; his standing block from his ROM block's pose (`0x2588`, with his
/// stance's hurtboxes) as a still posture; his crouch and crouching block from
/// his catalogue (his ROM table has no crouch: `0x05..=0x08` are empty); his
/// 1987 jab (`0x1141c`, given a 4-frame stance wind-up so its active fist is
/// not out on frame 1), hook (`0x1146c`), double-fist hammer (`0x1151c`),
/// low kick (`0x115cc`), diving jump punches (`0x1165c`, `0x119a0`) and Bull
/// Head (`0x11b42`, on the Hadoken's command); his standing high kick,
/// crouching punches, crouching kick, sweep, straight-up jump punch, straight-up
/// jump kicks (`jump_kick`, `jump_kick_high`) and diagonal jump kick
/// (`jump_kick_down`) from his catalogue; and the Bull Horn (`0x1b`) on the
/// Tatsumaki's command.
mod birdie {
    use super::*;

    /// His ROM move table (`0x17a9c`, thirteen moves; `0x05..=0x08` are empty).
    const ROM_TABLE: u32 = 0x17a9c;
    const HADOKEN_ENTRY: u32 = 0x4345e;
    const TATSUMAKI_ENTRY: u32 = 0x46012;

    /// His 1987 attacks (the streams his CPU records start).
    const JAB: u32 = 0x1141c; // standing LP (starts with sound call 0xa1, then 0x11422)
    const HOOK: u32 = 0x1146c; // standing MP
    const HAMMER: u32 = 0x1151c; // standing HP
    const LOW_KICK: u32 = 0x115cc; // standing LK and MK
    const BULL_HEAD: u32 = 0x11b42; // 0x1a, his 1987 flying headbutt

    /// ROM frames whose poses and boxes his streams reuse.
    const BLOCK_FRAME: u32 = 0x1138c; // 0x2588, his standing block (zero hurtboxes in ROM, #106)
    const JUMP_TOP_FRAME: u32 = 0x10fcc; // 0x240c, the top of his jump
    const FALL_FRAME: u32 = 0x1104c; // 0x2408, rising and falling
    const LANDING_FRAME: u32 = 0x11072; // 0x2404, his landing pose
    const FORWARD_DIVE_PUNCH_FRAME: u32 = 0x11768; // 0x2720, his diving punch (stream 0x1165c, hit 0x477ca)
    const BACK_DIVE_PUNCH_FRAME: u32 = 0x11aac; // 0x2720, his diving punch (stream 0x119a0, hit 0x4775c)
    const KICK_CHAMBER_FRAME: u32 = 0x115cc; // 0x2726, his low kick's chamber
    const KICK_RECOVER_FRAME: u32 = 0x11612; // 0x2780, his low kick's retraction
    const HIGH_KICK_HIT: u32 = 0x47612; // his hook's hit (12 deep / 8 shallow, guarded high or low)
    const HEADBUTT_WINDUP_FRAME: u32 = 0x11b42; // 0x2820, his headbutt's wind-up
    const HEADBUTT_LUNGE_FRAME: u32 = 0x11b8e; // 0x28a0, his headbutt's lunge step (zero head box in ROM)
    const HEADBUTT_RECOVER_FRAME: u32 = 0x11c14; // 0x282a, his headbutt's recovery

    /// His jumps (ROM streams) and where each reaches its top (`0x240c`).
    const JUMPS: [(u8, u32, u32); 3] = [(0x09, 0x10f46, 0x10fcc), (0x0a, 0x110e6, 0x1114c), (0x0b, 0x11246, 0x112ac)];

    /// Boxes for his eleven new poses, as rectangles in each drawing (see
    /// `tests/birdie_player.rs`, which checks them against the pens).
    pub const BOXES: [(&str, PixelBoxes); 11] = [
        ("crouch", PixelBoxes { push: [8, 16, 62, 88], head: [28, 2, 44, 15], body: [6, 16, 65, 88], attack: None }),
        ("crouching_block", PixelBoxes { push: [4, 16, 54, 88], head: [20, 2, 36, 15], body: [2, 16, 56, 88], attack: None }),
        (
            "crouching_punch",
            PixelBoxes { push: [8, 16, 62, 88], head: [29, 2, 45, 15], body: [6, 16, 65, 88], attack: Some([62, 27, 95, 39]) },
        ),
        (
            "crouching_kick",
            PixelBoxes { push: [10, 16, 64, 72], head: [28, 2, 44, 15], body: [6, 16, 68, 88], attack: Some([60, 57, 105, 68]) },
        ),
        (
            "sweep",
            PixelBoxes { push: [10, 16, 64, 88], head: [28, 2, 44, 15], body: [6, 16, 70, 88], attack: Some([60, 71, 112, 83]) },
        ),
        (
            "high_kick",
            PixelBoxes { push: [8, 18, 54, 96], head: [12, 2, 28, 15], body: [6, 12, 58, 102], attack: Some([50, 33, 94, 54]) },
        ),
        (
            "jump_punch",
            PixelBoxes { push: [8, 16, 60, 86], head: [29, 2, 45, 15], body: [6, 16, 65, 90], attack: Some([60, 27, 95, 39]) },
        ),
        (
            "jump_kick",
            PixelBoxes { push: [6, 18, 64, 72], head: [20, 2, 36, 15], body: [4, 16, 60, 86], attack: Some([52, 57, 101, 68]) },
        ),
        (
            "jump_kick_high",
            PixelBoxes { push: [20, 18, 76, 82], head: [32, 2, 48, 15], body: [18, 10, 76, 88], attack: Some([68, 30, 118, 54]) },
        ),
        (
            "jump_kick_down",
            PixelBoxes { push: [6, 18, 52, 84], head: [20, 2, 36, 15], body: [4, 14, 60, 90], attack: Some([56, 80, 86, 92]) },
        ),
        (
            "bull_horn",
            PixelBoxes { push: [8, 16, 58, 96], head: [44, 4, 62, 18], body: [6, 14, 64, 96], attack: Some([58, 30, 91, 46]) },
        ),
    ];

    fn boxes(name: &str) -> PixelBoxes {
        boxes_named(&BOXES, name)
    }

    pub fn build(rom: &[u8], catalogue: &Catalogue) -> Result<Region, String> {
        let base = table_address(TournamentFighter::Birdie);
        let mut region = new_slot(TournamentFighter::Birdie);
        // His specials: the Bull Head in the Hadoken's slot on its command
        // (quarter-circle forward and punch, as it sits at `0x1a` in his 1987
        // ROM table; `low_in_air: true` because its 1987 stream sets the
        // airborne bit on `0x11bd4`), and the Bull Horn in the Tatsumaki's on
        // its command (quarter-circle back and kick); both melee, scoring as
        // the specials whose slots they take. No Shoryuken.
        set_special(&mut region, base, 0x1a, SpecialSlot { tracker: 0x1c, kick: false, melee: true, low_in_air: true, score: HADOKEN_ENTRY });
        set_special(&mut region, base, 0x1b, SpecialSlot { tracker: 0x16, kick: true, melee: true, low_in_air: false, score: TATSUMAKI_ENTRY });
        let palette = rom_w(rom, ROM_TABLE + 0x30 + 2) & 0xff;
        let geom = |name: &str| catalogue_pose(catalogue, name, palette);

        // ROM headers kept as they are: the walk-in, stance, walks and win.
        for id in [0x00u8, 0x01, 0x02, 0x03, 0x0c] {
            keep_rom_header(&mut region, rom, ROM_TABLE, base, id);
        }
        let (stance, stance_boxes) = header_pose(rom, ROM_TABLE + 0x30);

        // The held postures, as Ryu's are: still headers (no busy bit, no
        // stream). His standing block is his ROM block's pose (`0x1138c`) with
        // his stance's hurtboxes (the ROM block zeroes its head and body
        // boxes, #106). His ROM table has no crouch (`0x05..=0x08` are empty),
        // so his crouches and crouching block come from his catalogue.
        let block_sound = rom_b(rom, ROM_TABLE + 4 * 0x30 + 0x10);
        let block = rom_pose(rom, BLOCK_FRAME);
        let mut block_boxes = rom_boxes(rom, BLOCK_FRAME);
        block_boxes[4..12].copy_from_slice(&stance_boxes[4..12]);
        let h = header(&still(block, 1, block_boxes).bytes(), block.top_on_ground(), 0x22, 0x04, [block_sound, 0, 0], 0);
        put_header(&mut region, base, 0x04, &h);
        let crouch = geom("crouch")?;
        let crouch_boxes = boxes("crouch").bytes(&crouch);
        for id in [0x05u8, 0x06, 0x07] {
            let h = header(&still(crouch, 1, crouch_boxes).bytes(), crouch.top_on_ground(), 0x14, id, [0, 0, 0], 0);
            put_header(&mut region, base, id, &h);
        }
        let crouching_block = geom("crouching_block")?;
        let h = header(
            &still(crouching_block, 1, boxes("crouching_block").bytes(&crouching_block)).bytes(),
            crouching_block.top_on_ground(),
            0x24,
            0x08,
            [block_sound, 0, 0],
            0,
        );
        put_header(&mut region, base, 0x08, &h);

        // His standing jab (`0x1141c`, on LP `0x0d` and `0x0e`): in the 1987
        // ROM `0x1141c` calls the punch sound (`0xa1`) and jumps straight into
        // its active punch frame (`0x11422`, 7 frames, hit `0x475a4`), with no
        // wind-up record of its own. Give the header a 4-frame stance wind-up
        // (`flags = 0x20`) pointing at `0x1141c`, so his jab has a 4-frame
        // startup, 7 active frames and 7 frames of recovery (`+6` on hit:
        // `14` hitstun against `8` total recovery, linking into standing LP).
        let mut jab_first = still(stance, 4, stance_boxes);
        jab_first.flags = 0x20;
        for id in [0x0du8, 0x0e] {
            let h = header(&jab_first.bytes(), stance.top_on_ground(), 0xc2, id, [0, 0, 0], JAB);
            put_header(&mut region, base, id, &h);
        }

        // His 1987 hook (`0x1146c`, on MP `0x0f`), double-fist hammer
        // (`0x1151c`, on HP `0x10`), low kick (`0x115cc`, on LK `0x14` and MK
        // `0x15`) and Bull Head (`0x11b42`, on `0x1a`), streamed from the ROM
        // as they are (`art/recipes/birdie.json`). Mark the first record with
        // the wind-up bit (`0x20`) so the fair CPU sees the move coming.
        let rom_attack = |region: &mut Region, id: u8, first: u32, sound: [u8; 3]| {
            let mut r = rom_record(rom, first);
            r[0x1a] |= 0x20;
            let p = rom_pose(rom, first);
            let top = stance.top_on_ground() + stance.half().1 - p.half().1 + r[6] as i8 as i32;
            let h = header(&r, top, 0xc2, id, sound, first + 0x20);
            put_header(region, base, id, &h);
        };
        for (id, stream) in [(0x0fu8, HOOK), (0x10, HAMMER), (0x14, LOW_KICK), (0x15, LOW_KICK), (0x1a, BULL_HEAD)] {
            rom_attack(&mut region, id, stream, [0, 0, 0]);
        }

        // His standing heavy kick (`0x16`, catalogue: kick_chamber 6, high_kick
        // 14, kick_recover 6, stance 10).
        let chamber = rom_pose(rom, KICK_CHAMBER_FRAME);
        let chamber_boxes = rom_boxes(rom, KICK_CHAMBER_FRAME);
        let recover = rom_pose(rom, KICK_RECOVER_FRAME);
        let recover_boxes = rom_boxes(rom, KICK_RECOVER_FRAME);
        let hk = geom("high_kick")?;
        let hk_stream = {
            let mut s = Stream::new(&mut region, chamber, chamber.top_on_ground());
            let at = s.call(routine::SOUND, 0xa2);
            s.hold(hk, hk.top_on_ground(), 14, boxes("high_kick").bytes(&hk), 0x80, HIGH_KICK_HIT);
            s.hold(recover, recover.top_on_ground(), 6, recover_boxes, 0, 0);
            s.hold(stance, stance.top_on_ground(), 10, stance_boxes, 0, 0);
            s.end();
            at
        };
        let mut hk_first = still(chamber, 6, chamber_boxes);
        hk_first.flags = 0x20;
        let h = header(&hk_first.bytes(), chamber.top_on_ground(), 0xc2, 0x16, [0, 0, 0], hk_stream);
        put_header(&mut region, base, 0x16, &h);

        // The crouching punch, every strength (catalogue: crouch 6, punch 12,
        // crouch 10).
        let cp = geom("crouching_punch")?;
        let cp_stream = {
            let mut s = Stream::new(&mut region, crouch, crouch.top_on_ground());
            let at = s.call(routine::SOUND, 0xa1);
            s.hold(cp, cp.top_on_ground(), 12, boxes("crouching_punch").bytes(&cp), 0xa0, hit::CROUCHING_PUNCH);
            s.hold(crouch, crouch.top_on_ground(), 10, crouch_boxes, 0, 0);
            s.end();
            at
        };
        for id in [0x11u8, 0x12, 0x13, 0x1c, 0x1d, 0x1e] {
            let h = header(&still(crouch, 6, crouch_boxes).bytes(), crouch.top_on_ground(), 0xc4, id, [0, 0, 0], cp_stream);
            put_header(&mut region, base, id, &h);
        }

        // The crouching light and medium kick (catalogue: crouch 6, kick 14,
        // crouch 14).
        let ck = geom("crouching_kick")?;
        let ck_stream = {
            let mut s = Stream::new(&mut region, crouch, crouch.top_on_ground());
            let at = s.call(routine::SOUND, 0xa2);
            s.hold(ck, ck.top_on_ground(), 14, boxes("crouching_kick").bytes(&ck), 0xa0, hit::CROUCHING_KICK);
            s.hold(crouch, crouch.top_on_ground(), 14, crouch_boxes, 0, 0);
            s.end();
            at
        };
        let h = header(&still(crouch, 6, crouch_boxes).bytes(), crouch.top_on_ground(), 0xc4, 0x17, [0, 0, 0], ck_stream);
        put_header(&mut region, base, 0x17, &h);

        // The crouching heavy kick / sweep (`0x18`, catalogue: crouch 8, sweep
        // 18, crouch 16).
        let sw = geom("sweep")?;
        let sw_stream = {
            let mut s = Stream::new(&mut region, crouch, crouch.top_on_ground());
            let at = s.call(routine::SOUND, 0xa2);
            s.hold(sw, sw.top_on_ground(), 18, boxes("sweep").bytes(&sw), 0xa0, hit::SWEEP);
            s.hold(crouch, crouch.top_on_ground(), 16, crouch_boxes, 0, 0);
            s.end();
            at
        };
        let h = header(&still(crouch, 8, crouch_boxes).bytes(), crouch.top_on_ground(), 0xc4, 0x18, [0, 0, 0], sw_stream);
        put_header(&mut region, base, 0x18, &h);

        // The Bull Horn (numbers and why at `bull_horn`): he coils through his
        // headbutt wind-up and lunge step, then charges across the stage with a
        // rising horn uppercut (`bull_horn`), settling through his headbutt's
        // recovery pose.
        let windup = rom_pose(rom, HEADBUTT_WINDUP_FRAME);
        let windup_boxes = rom_boxes(rom, HEADBUTT_WINDUP_FRAME);
        let lunge = rom_pose(rom, HEADBUTT_LUNGE_FRAME);
        let mut lunge_boxes = rom_boxes(rom, HEADBUTT_LUNGE_FRAME);
        lunge_boxes[4..8].copy_from_slice(&windup_boxes[4..8]);
        let hb_recover = rom_pose(rom, HEADBUTT_RECOVER_FRAME);
        let hb_recover_boxes = rom_boxes(rom, HEADBUTT_RECOVER_FRAME);
        let bh = geom("bull_horn")?;
        let nudge = bh.top_on_ground() - (lunge.top_on_ground() + lunge.half().1 - bh.half().1);
        let origin = pose_origin_record(&mut region, nudge, 0xa2);
        let bh_stream = {
            let mut s = Stream::new(&mut region, stance, stance.top_on_ground());
            let at = s.hold(windup, windup.top_on_ground(), bull_horn::WINDUP, windup_boxes, 0x20, 0);
            s.travel(lunge, lunge.top_on_ground(), bull_horn::LUNGE - 1, (0, bull_horn::SPEED), lunge_boxes, 0x20, 0);
            let top = s.top;
            s.travel(lunge, top, 1, (0, bull_horn::SPEED), lunge_boxes, 0x20, 0);
            s.change_origin(origin, nudge);
            s.strike(bh, bull_horn::HORN, (0, bull_horn::SPEED), boxes("bull_horn").bytes(&bh), 0x80, bull_horn::HIT);
            s.hold(hb_recover, hb_recover.top_on_ground(), bull_horn::RECOVER, hb_recover_boxes, 0, 0);
            s.hold(stance, stance.top_on_ground(), bull_horn::SETTLE, stance_boxes, 0, 0);
            s.end();
            at
        };
        let mut bh_first = still(stance, bull_horn::STANCE, stance_boxes);
        bh_first.flags = 0x20;
        let h = header(&bh_first.bytes(), stance.top_on_ground(), 0xc2, 0x1b, [0, 0, 0], bh_stream);
        put_header(&mut region, base, 0x1b, &h);

        // The jumps' attacks, from the top of each jump.
        let air = Air {
            rom,
            jump_top: geom("jump_top")?,
            top_boxes: rom_boxes(rom, JUMP_TOP_FRAME),
            fall: rom_pose(rom, FALL_FRAME),
            fall_boxes: rom_boxes(rom, FALL_FRAME),
            landing: rom_pose(rom, LANDING_FRAME),
            landing_boxes: rom_boxes(rom, LANDING_FRAME),
            stance,
            stance_boxes,
        };
        let jp = geom("jump_punch")?;
        let jk = geom("jump_kick")?;
        let jkh = geom("jump_kick_high")?;
        let jkd = geom("jump_kick_down")?;
        let apex = jump_top_at_apexes(rom, ROM_TABLE, &JUMPS, air.jump_top);
        let (neutral, forward, back) = (apex[0], apex[1], apex[2]);
        // His straight-up jump (`0x10f46`) rises 80 pixels from his landing
        // centre to its top (`neutral = 40`); `[2, 4, 1]` descends 76 pixels
        // across wind-up, strike and tuck so his straight-up punch (`0x20`),
        // low jump kick (`0x23`) and high jump kick (`0x26`) reach a standing
        // opponent on the way down.
        let s20 = air.attack(&mut region, neutral, jp, boxes("jump_punch").bytes(&jp), hit::JUMP_PUNCH, 0, [2, 4, 1], 0xa1)?;
        let s23 = air.attack(&mut region, neutral, jk, boxes("jump_kick").bytes(&jk), hit::JUMP_KICK, 0, [2, 4, 1], 0xa2)?;
        let s26 = air.attack(&mut region, neutral, jkh, boxes("jump_kick_high").bytes(&jkh), hit::JUMP_KICK, 0, [2, 4, 1], 0xa2)?;
        // His 1987 diagonal diving punches (`0x1165c` forward, `0x119a0` back):
        // from the top of his diagonal jump (`forward = -5`, `back = -4`),
        // 6 frames of wind-up (`0x240c`) and 8 active frames of his diving
        // punch (`0x2720`, hits `0x477ca` and `0x4775c`) descending at 8 pixels
        // a frame and moving 4 pixels across, then 1 frame of his falling pose
        // (`0x2408`) into the standard player landing.
        let s21 = air.rom_strike(
            &mut region,
            forward,
            0,
            Some((JUMP_TOP_FRAME, 6, (8, 4))),
            0xae,
            FORWARD_DIVE_PUNCH_FRAME,
            8,
            (8, 4),
            13,
        )?;
        let s22 = air.rom_strike(
            &mut region,
            back,
            0,
            Some((JUMP_TOP_FRAME, 6, (8, -4))),
            0xae,
            BACK_DIVE_PUNCH_FRAME,
            8,
            (8, -4),
            12,
        )?;
        // His diagonal jump kicks (`0x24` forward, `0x25` back): `[2, 5, 4]`
        // descends 114 pixels from the top of his diagonal jump (`125` pixels
        // above his landing centre) while moving 4 pixels a frame across.
        let s24 = air.attack(&mut region, forward, jkd, boxes("jump_kick_down").bytes(&jkd), hit::JUMP_KICK, 4, [2, 5, 4], 0xa2)?;
        let s25 = air.attack(&mut region, back, jkd, boxes("jump_kick_down").bytes(&jkd), hit::JUMP_KICK, -4, [2, 5, 4], 0xa2)?;

        let landing_centre = air.landing.top_on_ground() + air.landing.half().1;
        apex_and_jumps(
            &mut region,
            rom,
            ROM_TABLE,
            base,
            &JUMPS,
            landing_centre,
            [
                [(0x20, s20), (0x20, s20), (0x20, s20)],
                [(0x21, s21), (0x21, s21), (0x21, s21)],
                [(0x22, s22), (0x22, s22), (0x22, s22)],
                [(0x23, s23), (0x23, s23), (0x26, s26)],
                [(0x24, s24), (0x24, s24), (0x24, s24)],
                [(0x25, s25), (0x25, s25), (0x25, s25)],
            ],
        );
        fits(region)
    }

    /// The Bull Horn's numbers (quarter-circle back and kick; his catalogue's
    /// timing, tuned against `docs/research/sf1plus-reference-numbers.md` —
    /// Alpha Birdie's Bull Horn).
    ///
    /// - **Startup 18**: his stance 4, his headbutt wind-up 6 (`0x11b42`), and
    ///   his headbutt lunge step 8 (`0x11b8e`, advancing 4 pixels a frame, 32
    ///   in all); the rising horn uppercut (`bull_horn`) is out on the 19th
    ///   frame.
    /// - **Active 16** at 4 pixels a frame forward (64 pixels during the
    ///   charge, 96 over the move), in one record so a hit clears the active
    ///   bit for the rest of the charge.
    /// - **Damage 16 deep, 12 otherwise**: his 1987 Bull Head's hit
    ///   (`0x478a6`). Guarded high or low; it scores as the Tatsumaki.
    /// - **Recovery 16** (his headbutt recovery 6, stance 10): a guarded
    ///   Bull Horn leaves him right in front of the defender to be punished.
    pub mod bull_horn {
        pub const STANCE: u8 = 4;
        pub const WINDUP: u8 = 6;
        pub const LUNGE: u8 = 8;
        pub const HORN: u8 = 16;
        pub const SPEED: i32 = 4;
        pub const RECOVER: u8 = 6;
        pub const SETTLE: u8 = 10;
        /// His 1987 Bull Head's hit.
        pub const HIT: u32 = 0x478a6;
    }
}

/// Eagle (ticket #257): his ROM table's walk-in, stance, walks, jumps and win;
/// his ROM standing block as a held posture (with its attack box cleared) and
/// his new crouch and crouching block; his 1987 stick jab (`0x1239e`) on his
/// standing light punch and standing light kick, his 1987 low stick thrust
/// (`0x12628`) on his standing medium punch and his crouching punches, and his
/// 1987 spinning sticks (`0x12408`) on his standing heavy punch; his rising
/// stick strike, two-stick smash, crouching stick sweep, crouching stick
/// uppercut and jumping stick attacks from his catalogue, alongside his 1987
/// leaping stick strikes (`0x126f8`, `0x1285a`, `0x129bc`) on his heavy jump
/// kicks; his Manchester Black (re-timed from his 1987 spinning-sticks barrage
/// `0x124d8`) on the Hadoken's command (`0x1a`, quarter-circle forward and
/// punch) and his St. Andrew's Green (re-timed from his 1987 leaping stick
/// strike `0x129bc`) on the Tatsumaki's command (`0x1b`, quarter-circle back
/// and kick).
mod eagle {
    use super::*;

    /// His ROM move table (`0x17d0c`, thirteen moves).
    const ROM_TABLE: u32 = 0x17d0c;

    /// His 1987 ground attacks: the stick jab, the spinning sticks and the low
    /// stick thrust.
    const JAB: u32 = 0x1239e;
    const SPIN_STICKS: u32 = 0x12408;
    const LOW_THRUST: u32 = 0x12628;
    /// His 1987 spinning-sticks barrage (`0x124d8`) from its sound call on
    /// (`0x124f8..0x12628`), and the offset of its recovery record (`0x125fe`)
    /// inside that slice.
    const BARRAGE_SOUND_AND_REST: u32 = 0x124f8;
    const BARRAGE_END: u32 = 0x12628;
    const BARRAGE_SETTLE_OFFSET: usize = (0x125fe - BARRAGE_SOUND_AND_REST) as usize;

    /// ROM frames whose poses and boxes his streams reuse.
    const THRUST_WINDUP_FRAME: u32 = 0x12648; // 0x298c, his stick thrust wind-up
    const SMASH_WINDUP_FRAME: u32 = 0x12408; // 0x2988, both sticks raised behind him
    const JUMP_TOP_FRAME: u32 = 0x11f1a; // 0x2a40, the top of his jump
    const FALL_FRAME: u32 = 0x11f7a; // 0x29ec, his jump's rise and fall
    const LANDING_FRAME: u32 = 0x11fa0; // 0x29e8, his jump squat and landing
    const FORWARD_LEAP_SMASH_FRAME: u32 = 0x127c4; // 0x2a44, forward leaping stick strike (hit 0x47a5e)
    const BACK_LEAP_SMASH_FRAME: u32 = 0x12926; // 0x2a44, back leaping stick strike (hit 0x47b3a)
    const NEUTRAL_LEAP_SMASH_FRAME: u32 = 0x12a88; // 0x2a44, neutral leaping stick strike (hit 0x47acc)

    /// His jumps (ROM streams) and where each reaches its top: the apex call
    /// goes before that entry.
    const JUMPS: [(u8, u32, u32); 3] = [
        (0x09, 0x11eb4, 0x11f1a),
        (0x0a, 0x12014, 0x1207a),
        (0x0b, 0x12174, 0x121da),
    ];

    /// The entries his specials score with: the Hadoken's for Manchester
    /// Black, the Tatsumaki's for St. Andrew's Green.
    const HADOKEN_ENTRY: u32 = 0x45ff6;
    const TATSUMAKI_ENTRY: u32 = 0x46012;

    /// The hit function of his 1987 spinning sticks (`0x12408`): guarded
    /// standing (`0x22`), damage 12 deep and 10 otherwise.
    const SPIN_STICKS_HIT: u32 = 0x47914;

    /// Boxes in each new pose's pixel space (`art/recipes/eagle.json`),
    /// measured against the non-transparent pixels of his sprite sheets.
    pub const BOXES: &[(&str, PixelBoxes)] = &[
        (
            "crouch",
            PixelBoxes {
                push: [8, 16, 48, 70],
                head: [15, 2, 30, 14],
                body: [2, 14, 53, 71],
                attack: None,
            },
        ),
        (
            "crouching_block",
            PixelBoxes {
                push: [8, 16, 48, 70],
                head: [15, 2, 30, 14],
                body: [1, 14, 54, 70],
                attack: None,
            },
        ),
        (
            "crouching_sweep",
            PixelBoxes {
                push: [8, 16, 52, 70],
                head: [20, 2, 35, 14],
                body: [2, 14, 52, 72],
                attack: Some([20, 46, 83, 54]),
            },
        ),
        (
            "crouching_uppercut",
            PixelBoxes {
                push: [8, 52, 48, 108],
                head: [20, 38, 35, 50],
                body: [2, 50, 51, 109],
                attack: Some([46, 18, 58, 52]),
            },
        ),
        (
            "rising_strike",
            PixelBoxes {
                push: [10, 40, 52, 110],
                head: [27, 25, 42, 38],
                body: [4, 38, 56, 113],
                attack: Some([58, 17, 73, 33]),
            },
        ),
        (
            "smash",
            PixelBoxes {
                push: [6, 16, 50, 80],
                head: [24, 2, 39, 14],
                body: [2, 14, 48, 81],
                attack: Some([14, 37, 83, 49]),
            },
        ),
        (
            "jump_high",
            PixelBoxes {
                push: [8, 40, 50, 94],
                head: [20, 25, 35, 38],
                body: [3, 38, 50, 96],
                attack: Some([51, 17, 66, 33]),
            },
        ),
        (
            "jump_jab",
            PixelBoxes {
                push: [8, 16, 64, 70],
                head: [20, 2, 35, 14],
                body: [3, 14, 51, 72],
                attack: Some([30, 10, 87, 22]),
            },
        ),
        (
            "jump_poke",
            PixelBoxes {
                push: [8, 16, 52, 70],
                head: [20, 2, 35, 14],
                body: [2, 14, 51, 72],
                attack: Some([18, 46, 83, 54]),
            },
        ),
        (
            "jump_swing",
            PixelBoxes {
                push: [44, 14, 84, 64],
                head: [45, 1, 60, 12],
                body: [36, 10, 89, 68],
                attack: Some([78, 11, 127, 21]),
            },
        ),
    ];

    pub(super) fn build(rom: &[u8], c: &Catalogue) -> Result<Region, String> {
        let base = table_address(TournamentFighter::Eagle);
        let mut region = new_slot(TournamentFighter::Eagle);
        set_special(
            &mut region,
            base,
            0x1a,
            SpecialSlot {
                tracker: 0x1c,
                kick: false,
                melee: true,
                low_in_air: false,
                score: HADOKEN_ENTRY,
            },
        );
        set_special(
            &mut region,
            base,
            0x1b,
            SpecialSlot {
                tracker: 0x16,
                kick: true,
                melee: true,
                low_in_air: false,
                score: TATSUMAKI_ENTRY,
            },
        );

        let palette = rom_w(rom, ROM_TABLE + 0x30 + 2) & 0xff;
        let geom = |name: &str| catalogue_pose(c, name, palette);
        let boxes = |name: &str| boxes_named(BOXES, name);

        let (stance, stance_boxes) = header_pose(rom, ROM_TABLE + 0x30);
        let (block, mut block_boxes) = header_pose(rom, ROM_TABLE + 4 * 0x30);
        block_boxes[12..16].fill(0);
        let crouch = geom("crouch")?;
        let crouch_boxes = boxes("crouch").bytes(&crouch);
        let crouch_block = geom("crouching_block")?;
        let crouch_block_boxes = boxes("crouching_block").bytes(&crouch_block);

        // Walk-in, stance, walks and win: his ROM headers as they are.
        for id in [0x00, 0x01, 0x02, 0x03, 0x0c] {
            keep_rom_header(&mut region, rom, ROM_TABLE, base, id);
        }

        // Standing block, crouch and crouching block: held postures (`stream = 0`),
        // with the ROM's guard sound on the blocks.
        let block_sound = rom_b(rom, ROM_TABLE + 4 * 0x30 + 0x10);
        let h = header(&still(block, 1, block_boxes).bytes(), block.top_on_ground(), 0x22, 0x04, [block_sound, 0, 0], 0);
        put_header(&mut region, base, 0x04, &h);
        for id in [0x05, 0x06, 0x07] {
            let h = header(&still(crouch, 1, crouch_boxes).bytes(), crouch.top_on_ground(), 0x14, id, [0, 0, 0], 0);
            put_header(&mut region, base, id, &h);
        }
        let h = header(
            &still(crouch_block, 1, crouch_block_boxes).bytes(),
            crouch_block.top_on_ground(),
            0x24,
            0x08,
            [block_sound, 0, 0],
            0,
        );
        put_header(&mut region, base, 0x08, &h);

        // His 1987 ground attacks streamed from the ROM: his stick jab
        // (`0x1239e`, on LP `0x0d`, `0x0e` and LK `0x14`), his low stick thrust
        // (`0x12628`, on MP `0x0f` and crouching punches `0x11..=0x13`,
        // `0x1c..=0x1e`), and his spinning sticks (`0x12408`, on HP `0x10`).
        let rom_attack = |region: &mut Region, id: u8, state: u8, first: u32, sound: [u8; 3]| {
            let mut r = rom_record(rom, first);
            r[0x1a] |= 0x20;
            let p = rom_pose(rom, first);
            let top = stance.top_on_ground() + stance.half().1 - p.half().1 + r[6] as i8 as i32;
            let h = header(&r, top, state, id, sound, first + 0x20);
            put_header(region, base, id, &h);
        };
        for (id, snd) in [(0x0d, 0xa1), (0x0e, 0xa1), (0x14, 0xa2)] {
            rom_attack(&mut region, id, 0xc2, JAB, [snd, 0, 0]);
        }
        rom_attack(&mut region, 0x0f, 0xc2, LOW_THRUST, [0, 0, 0]);
        for id in [0x11, 0x12, 0x13, 0x1c, 0x1d, 0x1e] {
            rom_attack(&mut region, id, 0xc4, LOW_THRUST, [0, 0, 0]);
        }
        rom_attack(&mut region, 0x10, 0xc2, SPIN_STICKS, [0, 0, 0]);

        // Standing medium kick (`0x15`): rising stick strike (`6, 14, 10`),
        // winding up in his thrust wind-up (`0x298c`) and striking with his
        // 1987 spinning sticks' hit (`0x47914`).
        let thrust_windup = rom_pose(rom, THRUST_WINDUP_FRAME);
        let thrust_windup_boxes = rom_boxes(rom, THRUST_WINDUP_FRAME);
        let rs = geom("rising_strike")?;
        let s15 = {
            let mut s = Stream::new(&mut region, thrust_windup, thrust_windup.top_on_ground());
            let at = s.call(routine::SOUND, 0xa2);
            s.hold(rs, rs.top_on_ground(), 14, boxes("rising_strike").bytes(&rs), 0x80, SPIN_STICKS_HIT);
            s.hold(stance, stance.top_on_ground(), 10, stance_boxes, 0, 0);
            s.end();
            at
        };
        let mut first = still(thrust_windup, 6, thrust_windup_boxes);
        first.flags = 0x20;
        let h = header(&first.bytes(), thrust_windup.top_on_ground(), 0xc2, 0x15, [0, 0, 0], s15);
        put_header(&mut region, base, 0x15, &h);

        // Standing heavy kick (`0x16`): two-stick smash (`8, 16, 12`), winding
        // up with both sticks raised (`0x2988`) and striking with his 1987
        // spinning sticks' hit (`0x47914`).
        let smash_windup = rom_pose(rom, SMASH_WINDUP_FRAME);
        let smash_windup_boxes = rom_boxes(rom, SMASH_WINDUP_FRAME);
        let sm = geom("smash")?;
        let s16 = {
            let mut s = Stream::new(&mut region, smash_windup, smash_windup.top_on_ground());
            let at = s.call(routine::SOUND, 0xa2);
            s.hold(sm, sm.top_on_ground(), 16, boxes("smash").bytes(&sm), 0x80, SPIN_STICKS_HIT);
            s.hold(stance, stance.top_on_ground(), 12, stance_boxes, 0, 0);
            s.end();
            at
        };
        let mut first = still(smash_windup, 8, smash_windup_boxes);
        first.flags = 0x20;
        let h = header(&first.bytes(), smash_windup.top_on_ground(), 0xc2, 0x16, [0, 0, 0], s16);
        put_header(&mut region, base, 0x16, &h);

        // Crouching light and medium kick (`0x17`): crouching stick sweep
        // (`6, 14, 12`, low).
        let cs = geom("crouching_sweep")?;
        let s17 = {
            let mut s = Stream::new(&mut region, crouch, crouch.top_on_ground());
            let at = s.call(routine::SOUND, 0xa2);
            s.hold(cs, cs.top_on_ground(), 14, boxes("crouching_sweep").bytes(&cs), 0xa0, hit::CROUCHING_KICK);
            s.hold(crouch, crouch.top_on_ground(), 12, crouch_boxes, 0, 0);
            s.end();
            at
        };
        let mut first = still(crouch, 6, crouch_boxes);
        first.flags = 0x20;
        let h = header(&first.bytes(), crouch.top_on_ground(), 0xc4, 0x17, [0, 0, 0], s17);
        put_header(&mut region, base, 0x17, &h);

        // Crouching heavy kick (`0x18`): crouching stick uppercut (`6, 16, 14`,
        // low).
        let cu = geom("crouching_uppercut")?;
        let s18 = {
            let mut s = Stream::new(&mut region, crouch, crouch.top_on_ground());
            let at = s.call(routine::SOUND, 0xa2);
            s.hold(cu, cu.top_on_ground(), 16, boxes("crouching_uppercut").bytes(&cu), 0xa0, hit::SWEEP);
            s.hold(crouch, crouch.top_on_ground(), 14, crouch_boxes, 0, 0);
            s.end();
            at
        };
        let mut first = still(crouch, 6, crouch_boxes);
        first.flags = 0x20;
        let h = header(&first.bytes(), crouch.top_on_ground(), 0xc4, 0x18, [0, 0, 0], s18);
        put_header(&mut region, base, 0x18, &h);

        // Special `0x19` is unused (`new_slot` leaves its header zeroed);
        // Manchester Black is on `0x1a` (Hadoken's command, QCF+P) and St.
        // Andrew's Green is on `0x1b` (Tatsumaki's command, QCB+K).

        // Manchester Black (`0x1a`): his 1987 spinning-sticks barrage
        // (`0x124d8`) re-timed to `[6, 4, 4, 4, 4, 4, 4, 4, 4, 10]` (see
        // [`manchester_black`]).
        let mut mb_bytes = rom[BARRAGE_SOUND_AND_REST as usize..BARRAGE_END as usize].to_vec();
        mb_bytes[BARRAGE_SETTLE_OFFSET + 1] = manchester_black::SETTLE - 1;
        mb_bytes[BARRAGE_SETTLE_OFFSET] = 0;
        let mb_stream = region.put(&mb_bytes);
        let mut mb_first = still(smash_windup, manchester_black::WINDUP, smash_windup_boxes);
        mb_first.flags = 0x20;
        let h = header(&mb_first.bytes(), smash_windup.top_on_ground(), 0xc2, 0x1a, [0, 0, 0], mb_stream);
        put_header(&mut region, base, 0x1a, &h);

        // St. Andrew's Green (`0x1b`): his 1987 leaping stick strike
        // (`0x129bc`) re-timed to `[4, 6, 8, 14, 6, 4, 10]` and advancing
        // forward across the leap (see [`st_andrews_green`]).
        let rise = rom_pose(rom, FALL_FRAME);
        let rise_boxes = rom_boxes(rom, FALL_FRAME);
        let jump_top_rom = rom_pose(rom, JUMP_TOP_FRAME);
        let top_boxes = rom_boxes(rom, JUMP_TOP_FRAME);
        let landing = rom_pose(rom, LANDING_FRAME);
        let landing_boxes = rom_boxes(rom, LANDING_FRAME);
        let leap_smash = rom_pose(rom, NEUTRAL_LEAP_SMASH_FRAME);
        let leap_smash_boxes = rom_boxes(rom, NEUTRAL_LEAP_SMASH_FRAME);
        let land_sound = rom_b(rom, ROM_TABLE + 9 * 0x30 + 0x12);
        let sag_stream = {
            let mut s = Stream::new(&mut region, landing, landing.top_on_ground());
            let at = s.call(0x45a7a, 0);
            let rise_top = s.top + s.pose.half().1 - rise.half().1;
            s.travel(
                rise,
                rise_top,
                st_andrews_green::RISE,
                (st_andrews_green::RISE_DY, st_andrews_green::SPEED),
                rise_boxes,
                0x20,
                0,
            );
            let apex_top = s.top + s.pose.half().1 - jump_top_rom.half().1;
            s.travel(
                jump_top_rom,
                apex_top,
                st_andrews_green::APEX,
                (st_andrews_green::APEX_DY, st_andrews_green::SPEED),
                top_boxes,
                0x20,
                0,
            );
            s.call(routine::SOUND, 0xb0);
            s.strike(
                leap_smash,
                st_andrews_green::SMASH,
                (st_andrews_green::SMASH_DY, st_andrews_green::SPEED),
                leap_smash_boxes,
                0x80,
                st_andrews_green::HIT,
            );
            let fall_top = s.top + s.pose.half().1 - rise.half().1;
            s.travel(
                rise,
                fall_top,
                st_andrews_green::FALL,
                (st_andrews_green::FALL_DY, st_andrews_green::SPEED),
                rise_boxes,
                0,
                0,
            );
            s.call(routine::FALLING, 0);
            s.hold(landing, landing.top_on_ground(), st_andrews_green::LAND, landing_boxes, 0, 0);
            s.call(routine::LANDING, 0);
            s.hold(stance, stance.top_on_ground(), st_andrews_green::SETTLE, stance_boxes, 0, 0);
            s.end();
            at
        };
        let mut sag_first = still(landing, st_andrews_green::SQUAT, landing_boxes);
        sag_first.flags = 0x20;
        let h = header(
            &sag_first.bytes(),
            landing.top_on_ground(),
            0xc2,
            0x1b,
            [0, 0, land_sound],
            sag_stream,
        );
        put_header(&mut region, base, 0x1b, &h);

        // The jumps' attacks, from the top of each jump.
        let air = Air {
            rom,
            jump_top: geom("jump_top")?,
            top_boxes,
            fall: rise,
            fall_boxes: rise_boxes,
            landing,
            landing_boxes,
            stance,
            stance_boxes,
        };
        let jh = geom("jump_high")?;
        let jj = geom("jump_jab")?;
        let jpk = geom("jump_poke")?;
        let jsw = geom("jump_swing")?;
        let apex = jump_top_at_apexes(rom, ROM_TABLE, &JUMPS, air.jump_top);
        let (neutral, forward, back) = (apex[0], apex[1], apex[2]);
        // His straight-up and back jumps rise 60 pixels above his landing
        // centre (`neutral = 76`, `back = 76`) and his forward jump rises 65
        // pixels (`forward = 71`); `[1, 2, 2]` descends 50 pixels across the
        // 6, 14 and 8 frames so his jumping stick attacks reach a standing
        // opponent on the way down.
        let s20 = air.attack(&mut region, neutral, jh, boxes("jump_high").bytes(&jh), hit::JUMP_PUNCH, 0, [1, 2, 2], 0xa1)?;
        let s21 = air.attack(&mut region, forward, jj, boxes("jump_jab").bytes(&jj), hit::JUMP_PUNCH, 4, [1, 2, 2], 0xa1)?;
        let s22 = air.attack(&mut region, back, jj, boxes("jump_jab").bytes(&jj), hit::JUMP_PUNCH, -3, [1, 2, 2], 0xa1)?;
        let s23 = air.attack(&mut region, neutral, jpk, boxes("jump_poke").bytes(&jpk), hit::JUMP_KICK, 0, [1, 2, 2], 0xa2)?;
        let s24 = air.attack(&mut region, forward, jsw, boxes("jump_swing").bytes(&jsw), hit::JUMP_KICK, 4, [1, 2, 2], 0xa2)?;
        let s25 = air.attack(&mut region, back, jsw, boxes("jump_swing").bytes(&jsw), hit::JUMP_KICK, -3, [1, 2, 2], 0xa2)?;
        // His 1987 leaping stick strikes (`0x129bc` straight up, `0x126f8`
        // forward, `0x1285a` back) on his heavy jump kicks: 8 frames of wind-up
        // (`0x2a40`) and 8 active frames (`0x2a44`, hits `0x47acc`, `0x47a5e`,
        // `0x47b3a`) into his falling pose (`0x29ec`) and landing.
        let s26 = air.rom_strike(
            &mut region,
            neutral,
            6,
            Some((JUMP_TOP_FRAME, 8, (1, 0))),
            0xb0,
            NEUTRAL_LEAP_SMASH_FRAME,
            8,
            (3, 0),
            4,
        )?;
        let s24_rom = air.rom_strike(
            &mut region,
            forward,
            0,
            Some((JUMP_TOP_FRAME, 5, (1, 3))),
            0xb0,
            FORWARD_LEAP_SMASH_FRAME,
            5,
            (4, 3),
            8,
        )?;
        let s25_rom = air.rom_strike(
            &mut region,
            back,
            6,
            Some((JUMP_TOP_FRAME, 8, (1, -3))),
            0xb0,
            BACK_LEAP_SMASH_FRAME,
            8,
            (3, -3),
            4,
        )?;

        let landing_centre = air.landing.top_on_ground() + air.landing.half().1;
        apex_and_jumps(
            &mut region,
            rom,
            ROM_TABLE,
            base,
            &JUMPS,
            landing_centre,
            [
                [(0x20, s20), (0x20, s20), (0x20, s20)],
                [(0x21, s21), (0x21, s21), (0x21, s21)],
                [(0x22, s22), (0x22, s22), (0x22, s22)],
                [(0x23, s23), (0x23, s23), (0x26, s26)],
                [(0x24, s24), (0x24, s24), (0x24, s24_rom)],
                [(0x25, s25), (0x25, s25), (0x25, s25_rom)],
            ],
        );
        fits(region)
    }

    /// The Manchester Black's numbers (`0x1a`, quarter-circle forward and
    /// punch; his catalogue's timing, tuned against
    /// `docs/research/sf1plus-reference-numbers.md` — CvS2 Eagle's Manchester
    /// Black).
    ///
    /// - **Startup 6**: both sticks raised behind him (`0x2988`, `0x124d8`);
    ///   the first spin strike is out on the 7th frame.
    /// - **Active 32**: eight 4-frame spin records (`0x124fe..0x125fe`),
    ///   stepping 28 pixels into the first spin and 4 pixels on each of the
    ///   remaining seven (56 pixels across the barrage), with his 1987
    ///   barrage's hit (`0x47982`, damage 14 deep and 12 otherwise, guarded
    ///   standing or crouching). It scores as the Hadoken.
    /// - **Recovery 10**: his stance (`0x2920`, `0x125fe`).
    pub mod manchester_black {
        pub const WINDUP: u8 = 6;
        pub const SPIN_STEP: u8 = 4;
        pub const SPINS: u8 = 8;
        pub const SETTLE: u8 = 10;
        /// His 1987 spinning-sticks barrage's hit.
        pub const HIT: u32 = 0x47982;
    }

    /// The St. Andrew's Green's numbers (`0x1b`, quarter-circle back and kick;
    /// his catalogue's timing, tuned against
    /// `docs/research/sf1plus-reference-numbers.md` — CvS2 Eagle's St.
    /// Andrew's Green).
    ///
    /// - **Startup 18**: his jump squat 4 (`0x29e8`), rising 6 (`0x29ec` at
    ///   `dy = -5`) and jump top 8 (`0x2a40` at `dy = -3`), advancing 2 pixels
    ///   a frame across the leap; the two-stick overhead smash (`0x2a44`) is
    ///   out on the 19th frame.
    /// - **Active 14** descending 2 pixels a frame and advancing 2 pixels a
    ///   frame forward, with his 1987 leaping stick strike's hit (`0x47acc`,
    ///   damage 16 deep and 12 otherwise, guarded standing as an airborne
    ///   overhead). It scores as the Tatsumaki.
    /// - **Recovery 20**: 6 falling frames (`0x29ec` at `dy = 4`), 4 landing
    ///   squat frames (`0x29e8`) and 10 stance frames (`0x2920`).
    pub mod st_andrews_green {
        pub const SQUAT: u8 = 4;
        pub const RISE: u8 = 6;
        pub const RISE_DY: i32 = -5;
        pub const APEX: u8 = 8;
        pub const APEX_DY: i32 = -3;
        pub const SMASH: u8 = 14;
        pub const SMASH_DY: i32 = 2;
        pub const FALL: u8 = 6;
        pub const FALL_DY: i32 = 4;
        pub const SPEED: i32 = 2;
        pub const LAND: u8 = 4;
        pub const SETTLE: u8 = 10;
        /// His 1987 leaping stick strike's hit.
        pub const HIT: u32 = 0x47acc;
    }
}

/// Adon (ticket #258, `SP11`): his ROM table's intro, stance, walks, jumps and
/// win; his standing block from his ROM block's pose (`0x312c`) as a still
/// posture with his stance's hurtboxes (the ROM block header zeroes its
/// hurtboxes, #11); his crouch and crouching block from his catalogue (his ROM
/// table has no crouch: `0x05..=0x08` are empty); his standing double jab
/// (`0x132dc`, preceded by a 4-frame stance wind-up so it is not active on its
/// first frame, #177), body punch (`0x1336c`), low kick (`0x133fc`),
/// knee-and-roundhouse combination (`0x13850`) and diagonal flying kick
/// (`0x1348c`, dive and somersault kick); his crouching punches, crouching
/// kick, sweep, jump punches and neutral jump kicks from his catalogue; the
/// Rising Jaguar (`0x19`) on the Shoryuken's command and the Jaguar Kick
/// (`0x1b`) on the Tatsumaki's command.
mod adon {
    use super::*;

    /// His ROM move table (`0x17f7c`, twenty moves; `0x05..=0x08` are empty).
    const ROM_TABLE: u32 = 0x17f7c;
    const SHORYUKEN_ENTRY: u32 = 0x45f52;
    const TATSUMAKI_ENTRY: u32 = 0x46012;

    /// His 1987 attacks (the streams his CPU records start).
    const JAB: u32 = 0x132dc; // standing LP (`0x0d`, `0x0e`): double jab (hit `0x47c16`)
    const BODY_PUNCH: u32 = 0x1336c; // standing MP (`0x0f`) and HP (`0x10`, hit `0x47c84`)
    const LOW_KICK: u32 = 0x133fc; // standing LK (`0x14`) and MK (`0x15`, hit `0x47e3c`)
    const TWO_KICKS: u32 = 0x13850; // standing HK (`0x16`): knee and roundhouse (hit `0x47cf2`)

    /// ROM frames whose poses and boxes his streams reuse.
    const TAKEOFF_FRAME: u32 = 0x12dec; // 0x3240, his take-off and landing squat
    const FALL_FRAME: u32 = 0x12e0c; // 0x3244, rising and falling
    const JUMP_TOP_FRAME: u32 = 0x12e4c; // 0x3248, the top of his jump
    const JK_DIVE_FRAME: u32 = 0x13578; // 0x330a, his flying kick's downward punch dive (hit `0x47d60`)
    const JK_FLIP_FRAME: u32 = 0x13598; // 0x32a4, his flying kick's mid-air somersault tuck
    const JK_KICK_FRAME: u32 = 0x135b8; // 0x32a8, his flying kick's horizontal double-foot thrust (hit `0x47dce`)

    /// His jumps (ROM streams) and where each reaches its top (`0x3248`).
    const JUMPS: [(u8, u32, u32); 3] = [(0x09, 0x12de6, 0x12e4c), (0x0a, 0x12f66, 0x12fcc), (0x0b, 0x130e6, 0x1314c)];

    /// Boxes for his ten new poses, as rectangles in each drawing (see
    /// `tests/adon_player.rs`, which checks them against the pens).
    pub const BOXES: [(&str, PixelBoxes); 10] = [
        ("crouch", PixelBoxes { push: [8, 16, 46, 68], head: [19, 4, 32, 16], body: [6, 16, 48, 70], attack: None }),
        ("crouching_block", PixelBoxes { push: [4, 16, 41, 64], head: [15, 3, 28, 15], body: [4, 14, 44, 66], attack: None }),
        (
            "crouching_punch",
            PixelBoxes { push: [8, 14, 48, 60], head: [20, 4, 33, 16], body: [4, 15, 47, 70], attack: Some([44, 10, 79, 22]) },
        ),
        (
            "crouching_kick",
            PixelBoxes { push: [8, 16, 48, 60], head: [19, 4, 32, 16], body: [6, 16, 48, 70], attack: Some([46, 52, 73, 62]) },
        ),
        (
            "sweep",
            PixelBoxes { push: [12, 14, 52, 56], head: [30, 4, 43, 15], body: [6, 13, 66, 61], attack: Some([46, 48, 86, 58]) },
        ),
        (
            "jump_punch",
            PixelBoxes { push: [8, 15, 48, 62], head: [22, 5, 35, 17], body: [4, 14, 52, 66], attack: Some([48, 13, 81, 21]) },
        ),
        (
            "jump_punch_down",
            PixelBoxes { push: [6, 16, 44, 62], head: [20, 4, 33, 16], body: [2, 16, 48, 66], attack: Some([48, 30, 64, 45]) },
        ),
        (
            "jump_kick",
            PixelBoxes { push: [3, 14, 48, 55], head: [17, 3, 30, 15], body: [2, 13, 44, 62], attack: Some([40, 50, 65, 60]) },
        ),
        (
            "jump_kick_high",
            PixelBoxes { push: [12, 12, 52, 56], head: [16, 4, 29, 15], body: [5, 12, 57, 64], attack: Some([62, 7, 92, 24]) },
        ),
        (
            "rj_knee",
            PixelBoxes { push: [6, 16, 28, 60], head: [15, 3, 28, 15], body: [4, 6, 38, 86], attack: Some([18, 15, 45, 62]) },
        ),
    ];

    fn boxes(name: &str) -> PixelBoxes {
        boxes_named(&BOXES, name)
    }

    pub fn build(rom: &[u8], catalogue: &Catalogue) -> Result<Region, String> {
        let base = table_address(TournamentFighter::Adon);
        let mut region = new_slot(TournamentFighter::Adon);
        // His specials: the Rising Jaguar in the Shoryuken's slot on its
        // command (forward, down, down-forward and punch), and the Jaguar Kick
        // in the Tatsumaki's on its command (quarter-circle back and kick);
        // both melee, scoring as the specials whose slots they take. No
        // Hadoken.
        set_special(&mut region, base, 0x19, SpecialSlot { tracker: 0x19, kick: false, melee: true, low_in_air: false, score: SHORYUKEN_ENTRY });
        set_special(&mut region, base, 0x1b, SpecialSlot { tracker: 0x16, kick: true, melee: true, low_in_air: false, score: TATSUMAKI_ENTRY });
        let palette = rom_w(rom, ROM_TABLE + 0x30 + 2) & 0xff;
        let geom = |name: &str| catalogue_pose(catalogue, name, palette);

        // ROM headers kept as they are: the walk-in, stance, walks and win.
        for id in [0x00u8, 0x01, 0x02, 0x03, 0x0c] {
            keep_rom_header(&mut region, rom, ROM_TABLE, base, id);
        }
        let (stance, stance_boxes) = header_pose(rom, ROM_TABLE + 0x30);

        // The held postures, as Ryu's are: still headers (no busy bit, no
        // stream). His standing block is his ROM block header's pose (`0x312c`)
        // with his stance's hurtboxes (the ROM block header zeroes its
        // hurtboxes, #11). His ROM table has no crouch (`0x05..=0x08` are
        // empty), so his crouches and crouching block come from his catalogue.
        let block_sound = rom_b(rom, ROM_TABLE + 4 * 0x30 + 0x10);
        let (block, mut block_boxes) = header_pose(rom, ROM_TABLE + 4 * 0x30);
        block_boxes[4..12].copy_from_slice(&stance_boxes[4..12]);
        let h = header(&still(block, 1, block_boxes).bytes(), block.top_on_ground(), 0x22, 0x04, [block_sound, 0, 0], 0);
        put_header(&mut region, base, 0x04, &h);
        let crouch = geom("crouch")?;
        let crouch_boxes = boxes("crouch").bytes(&crouch);
        for id in [0x05u8, 0x06, 0x07] {
            let h = header(&still(crouch, 1, crouch_boxes).bytes(), crouch.top_on_ground(), 0x14, id, [0, 0, 0], 0);
            put_header(&mut region, base, id, &h);
        }
        let crouching_block = geom("crouching_block")?;
        let h = header(
            &still(crouching_block, 1, boxes("crouching_block").bytes(&crouching_block)).bytes(),
            crouching_block.top_on_ground(),
            0x24,
            0x08,
            [block_sound, 0, 0],
            0,
        );
        put_header(&mut region, base, 0x08, &h);

        // His standing double jab (`0x132dc`, on LP `0x0d` and `0x0e`): in the
        // 1987 ROM `0x132dc` calls the punch sound (`0xa1`) and jumps straight
        // into its first active punch frame (`0x132e2`, 9 frames, hit
        // `0x47c16`), with no wind-up record of its own (#177). Give the
        // header a 4-frame stance wind-up (`flags = 0x20`) pointing at
        // `0x132dc`, so his jab has a 4-frame startup before the first punch.
        let mut jab_first = still(stance, 4, stance_boxes);
        jab_first.flags = 0x20;
        for id in [0x0du8, 0x0e] {
            let h = header(&jab_first.bytes(), stance.top_on_ground(), 0xc2, id, [0, 0, 0], JAB);
            put_header(&mut region, base, id, &h);
        }

        // His 1987 body punch (`0x1336c`, on MP `0x0f` and HP `0x10`), low kick
        // (`0x133fc`, on LK `0x14` and MK `0x15`) and knee-and-roundhouse
        // combination (`0x13850`, on HK `0x16`), streamed from the ROM as they
        // are (`art/recipes/adon.json`).
        let rom_attack = |region: &mut Region, id: u8, first: u32, sound: [u8; 3]| {
            let mut r = rom_record(rom, first);
            r[0x1a] |= 0x20;
            let p = rom_pose(rom, first);
            let top = stance.top_on_ground() + stance.half().1 - p.half().1 + r[6] as i8 as i32;
            let h = header(&r, top, 0xc2, id, sound, first + 0x20);
            put_header(region, base, id, &h);
        };
        for (id, stream) in [(0x0fu8, BODY_PUNCH), (0x10, BODY_PUNCH), (0x14, LOW_KICK), (0x15, LOW_KICK), (0x16, TWO_KICKS)] {
            rom_attack(&mut region, id, stream, [0, 0, 0]);
        }

        // The crouching punch, every strength (catalogue: crouch 6, punch 12,
        // crouch 10).
        let cp = geom("crouching_punch")?;
        let cp_stream = {
            let mut s = Stream::new(&mut region, crouch, crouch.top_on_ground());
            let at = s.call(routine::SOUND, 0xa1);
            s.hold(cp, cp.top_on_ground(), 12, boxes("crouching_punch").bytes(&cp), 0xa0, hit::CROUCHING_PUNCH);
            s.hold(crouch, crouch.top_on_ground(), 10, crouch_boxes, 0, 0);
            s.end();
            at
        };
        for id in [0x11u8, 0x12, 0x13, 0x1c, 0x1d, 0x1e] {
            let mut first = still(crouch, 6, crouch_boxes);
            first.flags = 0x20;
            let h = header(&first.bytes(), crouch.top_on_ground(), 0xc4, id, [0, 0, 0], cp_stream);
            put_header(&mut region, base, id, &h);
        }

        // The crouching light and medium kick (catalogue: crouch 6, kick 14,
        // crouch 14).
        let ck = geom("crouching_kick")?;
        let ck_stream = {
            let mut s = Stream::new(&mut region, crouch, crouch.top_on_ground());
            let at = s.call(routine::SOUND, 0xa2);
            s.hold(ck, ck.top_on_ground(), 14, boxes("crouching_kick").bytes(&ck), 0xa0, hit::CROUCHING_KICK);
            s.hold(crouch, crouch.top_on_ground(), 14, crouch_boxes, 0, 0);
            s.end();
            at
        };
        let mut ck_first = still(crouch, 6, crouch_boxes);
        ck_first.flags = 0x20;
        let h = header(&ck_first.bytes(), crouch.top_on_ground(), 0xc4, 0x17, [0, 0, 0], ck_stream);
        put_header(&mut region, base, 0x17, &h);

        // The crouching heavy kick / sweep (`0x18`, catalogue: crouch 8, sweep
        // 18, crouch 16).
        let sw = geom("sweep")?;
        let sw_stream = {
            let mut s = Stream::new(&mut region, crouch, crouch.top_on_ground());
            let at = s.call(routine::SOUND, 0xa2);
            s.hold(sw, sw.top_on_ground(), 18, boxes("sweep").bytes(&sw), 0xa0, hit::SWEEP);
            s.hold(crouch, crouch.top_on_ground(), 16, crouch_boxes, 0, 0);
            s.end();
            at
        };
        let mut sw_first = still(crouch, 8, crouch_boxes);
        sw_first.flags = 0x20;
        let h = header(&sw_first.bytes(), crouch.top_on_ground(), 0xc4, 0x18, [0, 0, 0], sw_stream);
        put_header(&mut region, base, 0x18, &h);

        // The Rising Jaguar (`0x19`, dragon punch and punch; see
        // [`rising_jaguar`]): he springs up from his take-off squat through
        // his rising pose into his high knee (`rj_knee`), then drops back
        // through his rising pose and lands.
        let takeoff = rom_pose(rom, TAKEOFF_FRAME);
        let takeoff_boxes = rom_boxes(rom, TAKEOFF_FRAME);
        let rise = rom_pose(rom, FALL_FRAME);
        let rise_boxes = rom_boxes(rom, FALL_FRAME);
        let mut rj_rise_boxes = rise_boxes;
        rj_rise_boxes[2] = 4;
        let jump_top_rom = rom_pose(rom, JUMP_TOP_FRAME);
        let top_boxes = rom_boxes(rom, JUMP_TOP_FRAME);
        let rj_knee = geom("rj_knee")?;
        let rj_knee_boxes = boxes("rj_knee").bytes(&rj_knee);
        let land_sound = rom_b(rom, ROM_TABLE + 9 * 0x30 + 0x12);
        let rj_stream = {
            let mut s = Stream::new(&mut region, takeoff, takeoff.top_on_ground());
            let at = s.call(0x45a7a, 0);
            let rise_top = s.top + s.pose.half().1 - rise.half().1;
            s.travel(
                rise,
                rise_top,
                rising_jaguar::RISE,
                (rising_jaguar::RISE_DY, rising_jaguar::SPEED),
                rj_rise_boxes,
                0x20,
                0,
            );
            s.call(routine::SOUND, 0xb6);
            let knee_top = s.top + s.pose.half().1 - rj_knee.half().1;
            s.travel(
                rj_knee,
                knee_top,
                rising_jaguar::KNEE_UP,
                (rising_jaguar::KNEE_UP_DY, rising_jaguar::SPEED),
                rj_knee_boxes,
                0xa0,
                rising_jaguar::HIT,
            );
            let knee_apex_top = s.top;
            s.travel(
                rj_knee,
                knee_apex_top,
                rising_jaguar::KNEE_DOWN,
                (rising_jaguar::KNEE_DOWN_DY, 1),
                rj_knee_boxes,
                0,
                0,
            );
            let fall_top = s.top + s.pose.half().1 - rise.half().1;
            s.travel(rise, fall_top, rising_jaguar::FALL, (rising_jaguar::FALL_DY, 0), rise_boxes, 0, 0);
            s.call(routine::FALLING, 0);
            s.hold(takeoff, takeoff.top_on_ground(), rising_jaguar::LAND, takeoff_boxes, 0, 0);
            s.call(routine::LANDING, 0);
            s.hold(stance, stance.top_on_ground(), rising_jaguar::SETTLE, stance_boxes, 0, 0);
            s.end();
            at
        };
        let mut rj_first = still(takeoff, rising_jaguar::SQUAT, takeoff_boxes);
        rj_first.flags = 0x20;
        let h = header(&rj_first.bytes(), takeoff.top_on_ground(), 0xc2, 0x19, [0, 0, land_sound], rj_stream);
        put_header(&mut region, base, 0x19, &h);

        // The Jaguar Kick (`0x1b`, quarter-circle back and kick; see
        // [`jaguar_kick`]): his 1987 flying kick (`0x1348c`) re-timed to
        // `[2, 3, 3, 4, 4, 5, 3, 3, 10]`, rising into his downward punch dive
        // (`0x330a`), mid-air somersault (`0x32a4`) and flying double-foot
        // kick (`0x32a8`), then landing.
        let jk_dive = rom_pose(rom, JK_DIVE_FRAME);
        let jk_dive_boxes = rom_boxes(rom, JK_DIVE_FRAME);
        let jk_flip = rom_pose(rom, JK_FLIP_FRAME);
        let jk_flip_boxes = rom_boxes(rom, JK_FLIP_FRAME);
        let jk_kick = rom_pose(rom, JK_KICK_FRAME);
        let jk_kick_boxes = rom_boxes(rom, JK_KICK_FRAME);
        let jk_stream = {
            let mut s = Stream::new(&mut region, takeoff, takeoff.top_on_ground());
            let at = s.call(0x45a7a, 0);
            let rise_top = s.top + s.pose.half().1 - rise.half().1;
            s.travel(
                rise,
                rise_top,
                jaguar_kick::RISE,
                (jaguar_kick::RISE_DY, jaguar_kick::SPEED),
                rise_boxes,
                0x20,
                0,
            );
            let apex_top = s.top + s.pose.half().1 - jump_top_rom.half().1;
            s.travel(
                jump_top_rom,
                apex_top,
                jaguar_kick::TOP,
                (jaguar_kick::TOP_DY, jaguar_kick::SPEED),
                top_boxes,
                0x20,
                0,
            );
            s.call(routine::SOUND, 0xb6);
            s.strike(
                jk_dive,
                jaguar_kick::DIVE,
                (jaguar_kick::DIVE_DY, jaguar_kick::SPEED),
                jk_dive_boxes,
                0xa0,
                jaguar_kick::DIVE_HIT,
            );
            let flip_top = s.top + s.pose.half().1 - jk_flip.half().1;
            s.travel(
                jk_flip,
                flip_top,
                jaguar_kick::FLIP,
                (jaguar_kick::FLIP_DY, jaguar_kick::SPEED),
                jk_flip_boxes,
                0x20,
                0,
            );
            s.strike(
                jk_kick,
                jaguar_kick::KICK,
                (jaguar_kick::KICK_DY, jaguar_kick::SPEED),
                jk_kick_boxes,
                0x80,
                jaguar_kick::KICK_HIT,
            );
            let fall_top = s.top + s.pose.half().1 - rise.half().1;
            s.travel(rise, fall_top, jaguar_kick::FALL, (jaguar_kick::FALL_DY, 2), rise_boxes, 0, 0);
            s.call(routine::FALLING, 0);
            s.hold(takeoff, takeoff.top_on_ground(), jaguar_kick::LAND, takeoff_boxes, 0, 0);
            s.call(routine::LANDING, 0);
            s.hold(stance, stance.top_on_ground(), jaguar_kick::SETTLE, stance_boxes, 0, 0);
            s.end();
            at
        };
        let mut jk_first = still(takeoff, jaguar_kick::SQUAT, takeoff_boxes);
        jk_first.flags = 0x20;
        let h = header(&jk_first.bytes(), takeoff.top_on_ground(), 0xc2, 0x1b, [0, 0, land_sound], jk_stream);
        put_header(&mut region, base, 0x1b, &h);

        // The jumps' attacks, from the top of each jump.
        let air = Air {
            rom,
            jump_top: geom("jump_top")?,
            top_boxes,
            fall: rise,
            fall_boxes: rise_boxes,
            landing: takeoff,
            landing_boxes: takeoff_boxes,
            stance,
            stance_boxes,
        };
        let jp = geom("jump_punch")?;
        let jpd = geom("jump_punch_down")?;
        let jk = geom("jump_kick")?;
        let jkh = geom("jump_kick_high")?;
        let apex = jump_top_at_apexes(rom, ROM_TABLE, &JUMPS, air.jump_top);
        let (neutral, forward, back) = (apex[0], apex[1], apex[2]);
        // His straight-up jump (`0x12de6`) rises 59 pixels above his landing
        // centre (`neutral = 77`), his forward jump (`0x12f66`) 60 pixels
        // (`forward = 76`) and his back jump (`0x130e6`) 55 pixels (`back =
        // 81`); `[1, 2, 2]` descends 50 pixels across wind-up, strike and tuck
        // so his jump punches and neutral jump kicks reach a standing opponent
        // on the way down.
        let s20 = air.attack(&mut region, neutral, jp, boxes("jump_punch").bytes(&jp), hit::JUMP_PUNCH, 0, [1, 2, 2], 0xa1)?;
        let s21 = air.attack(&mut region, forward, jpd, boxes("jump_punch_down").bytes(&jpd), hit::JUMP_PUNCH, 4, [1, 2, 2], 0xa1)?;
        let s22 = air.attack(&mut region, back, jpd, boxes("jump_punch_down").bytes(&jpd), hit::JUMP_PUNCH, -4, [1, 2, 2], 0xa1)?;
        let s23 = air.attack(&mut region, neutral, jk, boxes("jump_kick").bytes(&jk), hit::JUMP_KICK, 0, [1, 2, 2], 0xa2)?;
        let s26 = air.attack(&mut region, neutral, jkh, boxes("jump_kick_high").bytes(&jkh), hit::JUMP_KICK, 0, [1, 2, 2], 0xa2)?;
        // His 1987 flying kick (`0x1348c`, dive `0x330a` -> somersault `0x32a4`
        // -> flying double-foot kick `0x32a8`) on his forward (`0x24`) and back
        // (`0x25`) jump kicks.
        let flying_kick = |region: &mut Region, apex_y: i32, step_dx: i32| -> Result<u32, String> {
            let mut s = Stream::new(region, air.jump_top, apex_y);
            let at = s.travel(air.jump_top, apex_y, 5, (1, step_dx), air.top_boxes, 0x20, 0);
            s.call(routine::SOUND, 0xa2);
            s.strike(jk_dive, 5, (1, step_dx), jk_dive_boxes, 0xa0, jaguar_kick::DIVE_HIT);
            let flip_top = s.top + s.pose.half().1 - jk_flip.half().1;
            s.travel(jk_flip, flip_top, 4, (2, step_dx), jk_flip_boxes, 0x20, 0);
            s.strike(jk_kick, 6, (2, step_dx), jk_kick_boxes, 0x80, jaguar_kick::KICK_HIT);
            air.land(&mut s, step_dx, 8)?;
            Ok(at)
        };
        let s24 = flying_kick(&mut region, forward, 4)?;
        let s25 = flying_kick(&mut region, back, -4)?;

        let landing_centre = air.landing.top_on_ground() + air.landing.half().1;
        apex_and_jumps(
            &mut region,
            rom,
            ROM_TABLE,
            base,
            &JUMPS,
            landing_centre,
            [
                [(0x20, s20), (0x20, s20), (0x20, s20)],
                [(0x21, s21), (0x21, s21), (0x21, s21)],
                [(0x22, s22), (0x22, s22), (0x22, s22)],
                [(0x23, s23), (0x23, s23), (0x26, s26)],
                [(0x24, s24), (0x24, s24), (0x24, s24)],
                [(0x25, s25), (0x25, s25), (0x25, s25)],
            ],
        );
        fits(region)
    }

    /// The Rising Jaguar's numbers (`0x19`, dragon punch and punch; his
    /// catalogue's timing `[4, 4, 16, 6, 6, 10]`, tuned against
    /// `docs/research/sf1plus-reference-numbers.md` — Alpha Adon's Rising
    /// Jaguar).
    ///
    /// - **Startup 8**: his take-off squat 4 (`0x3240`) and rising pose 4
    ///   (`0x3244` at `dy = -3`, advancing 2 pixels a frame); his rising knee
    ///   (`rj_knee`) is out on the 9th frame.
    /// - **Active 8**: the rising half of `rj_knee` (`dy = -3`, advancing 2
    ///   pixels a frame), with his 1987 knee-and-roundhouse's hit (`0x47cf2`,
    ///   damage 14 deep and 8 otherwise, guarded standing as an airborne
    ///   overhead). It scores as the Shoryuken.
    /// - **Recovery 30**: 8 descending frames in `rj_knee` (`dy = 2`), 6
    ///   falling frames in `rise` (`0x3244` at `dy = 3`), 6 landing squat
    ///   frames (`0x3240`) and 10 stance frames (`0x3060`).
    pub mod rising_jaguar {
        pub const SQUAT: u8 = 4;
        pub const RISE: u8 = 4;
        pub const RISE_DY: i32 = -3;
        pub const KNEE_UP: u8 = 8;
        pub const KNEE_UP_DY: i32 = -3;
        pub const KNEE_DOWN: u8 = 8;
        pub const KNEE_DOWN_DY: i32 = 2;
        pub const FALL: u8 = 6;
        pub const FALL_DY: i32 = 3;
        pub const SPEED: i32 = 3;
        pub const LAND: u8 = 6;
        pub const SETTLE: u8 = 10;
        /// His 1987 knee-and-roundhouse's hit.
        pub const HIT: u32 = 0x47cf2;
    }

    /// The Jaguar Kick's numbers (`0x1b`, quarter-circle back and kick; his
    /// catalogue's timing `[2, 3, 3, 4, 4, 5, 3, 3, 10]`, tuned against
    /// `docs/research/sf1plus-reference-numbers.md` — Alpha Adon's Jaguar
    /// Kick).
    ///
    /// - **Startup 8**: his take-off squat 2 (`0x3240`), rising 3 (`0x3244` at
    ///   `dy = -8`) and jump top 3 (`0x3248` at `dy = -4`), advancing 4 pixels
    ///   a frame; the downward punch dive (`0x330a`) is out on the 9th frame.
    /// - **Active 4 + 5** across his 1987 flying kick's two strikes: 4 dive
    ///   frames (`0x330a`, hit `0x47d60`, damage 14 deep / 8 shallow), 4
    ///   mid-air somersault frames (`0x32a4`), and 5 flying double-foot kick
    ///   frames (`0x32a8`, hit `0x47dce`, damage 16 deep / 8 shallow), both
    ///   guarded standing as airborne strikes. Under `true_combos` the
    ///   one-hit-per-move rule stops after whichever strike connects first. It
    ///   scores as the Tatsumaki.
    /// - **Recovery 16**: 3 falling frames (`0x3244`), 3 landing squat frames
    ///   (`0x3240`) and 10 stance frames (`0x3060`).
    pub mod jaguar_kick {
        pub const SQUAT: u8 = 2;
        pub const RISE: u8 = 3;
        pub const RISE_DY: i32 = -8;
        pub const TOP: u8 = 3;
        pub const TOP_DY: i32 = -4;
        pub const DIVE: u8 = 4;
        pub const DIVE_DY: i32 = 0;
        pub const FLIP: u8 = 4;
        pub const FLIP_DY: i32 = 2;
        pub const KICK: u8 = 5;
        pub const KICK_DY: i32 = 3;
        pub const FALL: u8 = 3;
        pub const FALL_DY: i32 = 4;
        pub const SPEED: i32 = 4;
        pub const LAND: u8 = 3;
        pub const SETTLE: u8 = 10;
        /// His 1987 flying kick's dive hit (`0x47d60`).
        pub const DIVE_HIT: u32 = 0x47d60;
        /// His 1987 flying kick's double-foot kick hit (`0x47dce`).
        pub const KICK_HIT: u32 = 0x47dce;
    }
}

pub use adon::jaguar_kick as adon_jaguar_kick;
pub use adon::rising_jaguar as adon_rising_jaguar;
pub use adon::BOXES as ADON_BOXES;
pub use birdie::bull_horn as birdie_bull_horn;
pub use birdie::BOXES as BIRDIE_BOXES;
pub use eagle::manchester_black as eagle_manchester_black;
pub use eagle::st_andrews_green as eagle_st_andrews_green;
pub use eagle::BOXES as EAGLE_BOXES;
pub use geki::BOXES as GEKI_BOXES;
pub use geki::teleport as geki_teleport;
pub use gen::gekiro as gen_gekiro;
pub use gen::hyakurenko as gen_hyakurenko;
pub use gen::BOXES as GEN_BOXES;
pub use joe::dynamite as joe_dynamite;
pub use joe::sobat as joe_sobat;
pub use joe::BOXES as JOE_BOXES;
pub use lee::double_kick as lee_double_kick;
pub use lee::zesshou as lee_zesshou;
pub use lee::BOXES as LEE_BOXES;
pub use mike::iron as mike_iron;
pub use mike::rush as mike_rush;
pub use mike::BOXES as MIKE_BOXES;
pub use retsu::BOXES as RETSU_BOXES;
pub use sagat::high_tiger_shot as sagat_high_tiger_shot;
pub use sagat::low_tiger_shot as sagat_low_tiger_shot;
pub use sagat::tiger_knee as sagat_tiger_knee;
pub use sagat::BOXES as SAGAT_BOXES;

/// Sagat (ticket #259, `SP12`): his ROM table's intro, stance, walks and win;
/// his standing block from his ROM block's pose (`0x2d8c`) as a still posture;
/// his crouch and crouching block from his catalogue's `crouch` (his ROM table
/// has no crouch or jumps: `0x05..=0x0b` are empty); his standing jab
/// (`0x13ec0`, preceded by a 4-frame stance wind-up so it is not active on its
/// first frame), step punch (`0x13f10`), low kick (`0x13fa0`) and high kick
/// (`0x14030`); his jumps built from his ROM rising (`0x2d80`), lift (`0x2c8c`)
/// and knee (`0x2d0a`) poses; his crouching punch, crouching kick, three-frame
/// sweep and five jump attacks from his catalogue; the High Tiger Shot (`0x1a`)
/// on the Hadoken's command, the Low Tiger Shot (`0x1b`) on quarter-circle
/// forward and kick, and the Tiger Knee (`0x19`) on dragon punch and kick.
mod sagat {
    use super::*;

    /// His ROM move table (`0x181ec`, thirteen moves; `0x05..=0x0b` are empty).
    const ROM_TABLE: u32 = 0x181ec;
    const SHORYUKEN_ENTRY: u32 = 0x45f52;
    const TIGER_SHOT_ENTRY: u32 = 0x436a8;
    const TIGER_SHOT_HIT: u32 = 0x43652;
    const TIGER_SHOT_SPAWN: u32 = 0x45ace;
    const TIGER_SHOT_TEMPLATE: u32 = 0x1873c;

    /// His 1987 attacks (the streams his CPU records start).
    const JAB: u32 = 0x13ec0; // standing LP (`0x0d`, `0x0e`, hit `0x47eaa`)
    const LOW_STEP_PUNCH: u32 = 0x13f10; // standing MP (`0x0f`) and HP (`0x10`, hit `0x47f18`)
    const LOW_KICK: u32 = 0x13fa0; // standing LK (`0x14`, hit `0x47f86`)
    const HIGH_KICK: u32 = 0x14030; // standing MK (`0x15`) and HK (`0x16`, hit `0x47ff4`)
    const HIGH_TIGER_SHOT: u32 = 0x144b4; // High Tiger Shot (`0x1a`)
    /// Offset in the copied `0x144b4 + 0x20` stream of `0x14500`'s duration byte
    /// (`+0x00` call `0x455de`, `+0x06` `0x144da`, `+0x26` call `0x45ace`,
    /// `+0x2c` `0x14500`, duration at `+0x2d`).
    const HIGH_SHOT_HOLD_OFFSET: usize = 0x2d;

    /// ROM frames whose poses and boxes his jumps and Tiger Knee reuse.
    const RISING_FRAME: u32 = 0x14242; // 0x2d80, his take-off and landing squat
    const JUMP_LIFT_FRAME: u32 = 0x14268; // 0x2c8c, his rising and falling pose
    const JUMP_KNEE_FRAME: u32 = 0x1412c; // 0x2d0a, his forward-leaping knee (hit `0x480d0`)

    /// Boxes for his fourteen new poses, as rectangles in each drawing (see
    /// `tests/sagat_player.rs`, which checks them against the pens).
    pub const BOXES: [(&str, PixelBoxes); 14] = [
        ("crouch", PixelBoxes { push: [8, 4, 52, 85], head: [34, 1, 52, 18], body: [2, 4, 62, 85], attack: None }),
        (
            "crouching_punch",
            PixelBoxes { push: [20, 12, 66, 91], head: [37, 1, 52, 17], body: [16, 12, 74, 92], attack: Some([60, 15, 103, 33]) },
        ),
        (
            "crouching_kick_chamber",
            PixelBoxes { push: [8, 4, 54, 81], head: [30, 1, 47, 17], body: [2, 4, 63, 81], attack: None },
        ),
        (
            "crouching_kick_2",
            PixelBoxes { push: [8, 8, 64, 64], head: [30, 1, 47, 17], body: [2, 8, 60, 88], attack: Some([58, 48, 99, 62]) },
        ),
        (
            "sweep_1",
            PixelBoxes { push: [18, 12, 64, 84], head: [47, 1, 62, 17], body: [15, 12, 80, 95], attack: Some([76, 72, 114, 88]) },
        ),
        ("sweep_2", PixelBoxes { push: [16, 12, 60, 84], head: [45, 1, 60, 17], body: [14, 12, 78, 95], attack: None }),
        ("sweep_3", PixelBoxes { push: [8, 12, 56, 84], head: [42, 1, 56, 17], body: [2, 6, 66, 93], attack: None }),
        (
            "jump_punch",
            PixelBoxes { push: [8, 14, 60, 88], head: [17, 1, 32, 17], body: [6, 14, 66, 94], attack: Some([50, 13, 88, 25]) },
        ),
        (
            "jump_punch_down",
            PixelBoxes { push: [6, 8, 52, 96], head: [42, 1, 58, 18], body: [4, 8, 60, 102], attack: Some([60, 38, 86, 54]) },
        ),
        (
            "jump_kick",
            PixelBoxes { push: [8, 14, 64, 68], head: [17, 1, 32, 17], body: [0, 0, 59, 80], attack: Some([54, 40, 117, 64]) },
        ),
        (
            "jump_kick_high",
            PixelBoxes { push: [14, 14, 50, 74], head: [17, 1, 30, 17], body: [2, 12, 54, 72], attack: Some([54, 12, 90, 24]) },
        ),
        (
            "jump_kick_down",
            PixelBoxes { push: [14, 16, 56, 76], head: [17, 1, 31, 17], body: [3, 16, 64, 84], attack: Some([64, 88, 88, 104]) },
        ),
        (
            "low_tiger_shot_1",
            PixelBoxes { push: [8, 8, 58, 100], head: [45, 1, 62, 18], body: [2, 8, 66, 100], attack: None },
        ),
        (
            "low_tiger_shot_2",
            PixelBoxes { push: [24, 16, 80, 88], head: [61, 1, 76, 17], body: [18, 16, 86, 92], attack: None },
        ),
    ];

    fn boxes(name: &str) -> PixelBoxes {
        boxes_named(&BOXES, name)
    }

    pub fn build(rom: &[u8], catalogue: &Catalogue) -> Result<Region, String> {
        let base = table_address(TournamentFighter::Sagat);
        let mut region = new_slot(TournamentFighter::Sagat);
        // His three specials: the Tiger Knee in the Shoryuken's slot (`0x19`)
        // on dragon punch and kick (melee, guarded high or low in the air,
        // scoring as the Shoryuken); the High Tiger Shot in the Hadoken's slot
        // (`0x1a`) on quarter-circle forward and punch; the Low Tiger Shot in
        // the Tatsumaki's slot (`0x1b`) on quarter-circle forward and kick
        // (both projectiles whose bank templates score for themselves).
        set_special(&mut region, base, 0x19, SpecialSlot { tracker: 0x19, kick: true, melee: true, low_in_air: true, score: SHORYUKEN_ENTRY });
        set_special(&mut region, base, 0x1a, SpecialSlot { tracker: 0x1c, kick: false, melee: false, low_in_air: false, score: 0 });
        set_special(&mut region, base, 0x1b, SpecialSlot { tracker: 0x1c, kick: true, melee: false, low_in_air: false, score: 0 });
        let palette = rom_w(rom, ROM_TABLE + 0x30 + 2) & 0xff;
        let geom = |name: &str| catalogue_pose(catalogue, name, palette);

        // ROM headers kept as they are: the walk-in, stance, walks and win.
        for id in [0x00u8, 0x01, 0x02, 0x03, 0x0c] {
            keep_rom_header(&mut region, rom, ROM_TABLE, base, id);
        }
        let (stance, stance_boxes) = header_pose(rom, ROM_TABLE + 0x30);

        // The held postures, as Ryu's are: still headers (no busy bit, no
        // stream). His standing block is his ROM block header's pose (`0x2d8c`)
        // and boxes; his crouches (`0x05..=0x07`) and crouching block (`0x08`)
        // use his catalogue's `crouch` (`art/recipes/sagat.json`).
        let block_sound = rom_b(rom, ROM_TABLE + 4 * 0x30 + 0x10);
        let (block, block_boxes) = header_pose(rom, ROM_TABLE + 4 * 0x30);
        let h = header(&still(block, 1, block_boxes).bytes(), block.top_on_ground(), 0x22, 0x04, [block_sound, 0, 0], 0);
        put_header(&mut region, base, 0x04, &h);
        let crouch = geom("crouch")?;
        let crouch_boxes = boxes("crouch").bytes(&crouch);
        for id in [0x05u8, 0x06, 0x07] {
            let h = header(&still(crouch, 1, crouch_boxes).bytes(), crouch.top_on_ground(), 0x14, id, [0, 0, 0], 0);
            put_header(&mut region, base, id, &h);
        }
        let h = header(&still(crouch, 1, crouch_boxes).bytes(), crouch.top_on_ground(), 0x24, 0x08, [block_sound, 0, 0], 0);
        put_header(&mut region, base, 0x08, &h);

        // His standing jab (`0x13ec0`, on LP `0x0d` and `0x0e`): in the 1987
        // ROM `0x13ec0` calls the punch sound (`0xa1`) and enters its punch
        // record (`0x13ec6`, 10 frames, hit `0x47eaa`, whose active bit `+0x1a`
        // was left `0x00` in the 1987 ROM) with no wind-up record of its own.
        // Give the header a 4-frame stance wind-up (`flags = 0x20`) pointing at
        // a bank copy of `0x13ec0` with `0x13ec6`'s active bit (`0x80`) set.
        let mut jab_region = Region::new(region.here());
        copy_rom_stream(&mut jab_region, rom, JAB, CopyHow::default());
        jab_region.bytes[6 + 0x1a] |= 0x80;
        let jab_stream = region.put(&jab_region.bytes);
        let mut jab_first = still(stance, 4, stance_boxes);
        jab_first.flags = 0x20;
        for id in [0x0du8, 0x0e] {
            let h = header(&jab_first.bytes(), stance.top_on_ground(), 0xc2, id, [0, 0, 0], jab_stream);
            put_header(&mut region, base, id, &h);
        }

        // His 1987 step punch (`0x13f10`, on MP `0x0f` and HP `0x10`, with its
        // wind-up's front push extent clamped to his stance's `32` so its
        // 3-frame step does not push a point-blank opponent out of reach of
        // `0x13f36`), low kick (`0x13fa0`, on LK `0x14`) and high kick
        // (`0x14030`, on MK `0x15` and HK `0x16`), streamed from the ROM as
        // they are (`art/recipes/sagat.json`).
        let rom_attack = |region: &mut Region, id: u8, first: u32, rest: u32| {
            let mut r = rom_record(rom, first);
            r[0x1a] |= 0x20;
            r[0xc] = r[0xc].min(stance_boxes[2]);
            let p = rom_pose(rom, first);
            let top = stance.top_on_ground() + stance.half().1 - p.half().1 + r[6] as i8 as i32;
            let h = header(&r, top, 0xc2, id, [0, 0, 0], rest);
            put_header(region, base, id, &h);
        };
        for (id, stream) in [(0x0fu8, LOW_STEP_PUNCH), (0x10, LOW_STEP_PUNCH), (0x14, LOW_KICK), (0x15, HIGH_KICK), (0x16, HIGH_KICK)] {
            rom_attack(&mut region, id, stream, stream + 0x20);
        }

        // The crouching punch, every strength (catalogue: crouch 6, punch 12,
        // crouch 10).
        let cp = geom("crouching_punch")?;
        let cp_stream = {
            let mut s = Stream::new(&mut region, crouch, crouch.top_on_ground());
            let at = s.call(routine::SOUND, 0xa1);
            s.hold(cp, cp.top_on_ground(), 12, boxes("crouching_punch").bytes(&cp), 0xa0, hit::CROUCHING_PUNCH);
            s.hold(crouch, crouch.top_on_ground(), 10, crouch_boxes, 0, 0);
            s.end();
            at
        };
        for id in [0x11u8, 0x12, 0x13, 0x1c, 0x1d, 0x1e] {
            let mut first = still(crouch, 6, crouch_boxes);
            first.flags = 0x20;
            let h = header(&first.bytes(), crouch.top_on_ground(), 0xc4, id, [0, 0, 0], cp_stream);
            put_header(&mut region, base, id, &h);
        }

        // The crouching light and medium kick (`0x17`, catalogue: crouch 4,
        // chamber 5, kick 12, chamber 4, crouch 10).
        let ckc = geom("crouching_kick_chamber")?;
        let ckc_boxes = boxes("crouching_kick_chamber").bytes(&ckc);
        let ck2 = geom("crouching_kick_2")?;
        let ck_stream = {
            let mut s = Stream::new(&mut region, crouch, crouch.top_on_ground());
            let at = s.hold(ckc, ckc.top_on_ground(), 5, ckc_boxes, 0x20, 0);
            s.call(routine::SOUND, 0xa2);
            s.hold(ck2, ck2.top_on_ground(), 12, boxes("crouching_kick_2").bytes(&ck2), 0xa0, hit::CROUCHING_KICK);
            s.hold(ckc, ckc.top_on_ground(), 4, ckc_boxes, 0, 0);
            s.hold(crouch, crouch.top_on_ground(), 10, crouch_boxes, 0, 0);
            s.end();
            at
        };
        let mut ck_first = still(crouch, 4, crouch_boxes);
        ck_first.flags = 0x20;
        let h = header(&ck_first.bytes(), crouch.top_on_ground(), 0xc4, 0x17, [0, 0, 0], ck_stream);
        put_header(&mut region, base, 0x17, &h);

        // The crouching heavy kick / sweep (`0x18`, catalogue: crouch 6,
        // sweep_1 16, sweep_2 8, sweep_3 8, crouch 12).
        let sw1 = geom("sweep_1")?;
        let sw2 = geom("sweep_2")?;
        let sw3 = geom("sweep_3")?;
        let sw_stream = {
            let mut s = Stream::new(&mut region, crouch, crouch.top_on_ground());
            let at = s.call(routine::SOUND, 0xa2);
            s.hold(sw1, sw1.top_on_ground(), 16, boxes("sweep_1").bytes(&sw1), 0xa0, hit::SWEEP);
            s.hold(sw2, sw2.top_on_ground(), 8, boxes("sweep_2").bytes(&sw2), 0, 0);
            s.hold(sw3, sw3.top_on_ground(), 8, boxes("sweep_3").bytes(&sw3), 0, 0);
            s.hold(crouch, crouch.top_on_ground(), 12, crouch_boxes, 0, 0);
            s.end();
            at
        };
        let mut sw_first = still(crouch, 6, crouch_boxes);
        sw_first.flags = 0x20;
        let h = header(&sw_first.bytes(), crouch.top_on_ground(), 0xc4, 0x18, [0, 0, 0], sw_stream);
        put_header(&mut region, base, 0x18, &h);

        // Both Tiger Shots' projectiles wrap the ROM's Tiger Shot hit handler
        // (`0x43652`, which scales to 10 hit / 1 chip under `balanced_roster`)
        // in `ROUTINE_EFFECT_ROM_HIT` so a player's Tiger Shot scores with
        // `0x436a8` (1,000 points).
        let tiger_shot_hit = region.put(&[ROUTINE_EFFECT_ROM_HIT, 0, 0, 0]);
        region.put_l(TIGER_SHOT_HIT);
        region.put_l(TIGER_SHOT_ENTRY);

        // High Tiger Shot (`0x1a`, quarter-circle forward and punch; see
        // [`high_tiger_shot`]): his 1987 Tiger Shot stream (`0x144b4`), its
        // post-release hold shortened from the CPU's 51 frames (`0x14500`) to
        // [`high_tiger_shot::HOLD`].
        let mut high_template: [u8; 0x20] = std::array::from_fn(|i| rom_b(rom, TIGER_SHOT_TEMPLATE + i as u32));
        high_template[0x14..0x18].copy_from_slice(&tiger_shot_hit.to_be_bytes());
        let high_template_addr = region.put(&high_template);
        let high_spawn = region.put(&[ROUTINE_PROJECTILE, 7, 0, 0]);
        region.put_l(high_template_addr);
        let mut high_stream_region = Region::new(region.here());
        copy_rom_stream(
            &mut high_stream_region,
            rom,
            HIGH_TIGER_SHOT + 0x20,
            CopyHow { replace: &[(TIGER_SHOT_SPAWN, high_spawn)], ..CopyHow::default() },
        );
        high_stream_region.bytes[HIGH_SHOT_HOLD_OFFSET] = high_tiger_shot::HOLD - 1;
        let high_shot_stream = region.put(&high_stream_region.bytes);
        rom_attack(&mut region, 0x1a, HIGH_TIGER_SHOT, high_shot_stream);

        // Low Tiger Shot (`0x1b`, quarter-circle forward and kick; see
        // [`low_tiger_shot`]): from his crouch (`crouch`, `low_tiger_shot_1`,
        // `low_tiger_shot_2`, `crouch`) he fires a low Tiger Shot template
        // across the ground.
        let mut low_template = high_template;
        let base_y = i16::from_be_bytes([low_template[4], low_template[5]]);
        let low_y = base_y + low_tiger_shot::DROP_Y;
        low_template[4..6].copy_from_slice(&low_y.to_be_bytes());
        let low_template_addr = region.put(&low_template);
        let low_spawn = region.put(&[ROUTINE_PROJECTILE, 7, 0, 0]);
        region.put_l(low_template_addr);
        let lts1 = geom("low_tiger_shot_1")?;
        let lts1_boxes = boxes("low_tiger_shot_1").bytes(&lts1);
        let lts2 = geom("low_tiger_shot_2")?;
        let lts2_boxes = boxes("low_tiger_shot_2").bytes(&lts2);
        let low_shot_stream = {
            let mut s = Stream::new(&mut region, crouch, crouch.top_on_ground());
            let at = s.hold(lts1, lts1.top_on_ground(), low_tiger_shot::CHAMBER, lts1_boxes, 0x20, 0);
            s.call(routine::SOUND, 0xb8);
            s.hold(lts2, lts2.top_on_ground(), 1, lts2_boxes, 0, 0);
            s.call(low_spawn, 0);
            s.hold(lts2, lts2.top_on_ground(), low_tiger_shot::HOLD - 1, lts2_boxes, 0, 0);
            s.hold(crouch, crouch.top_on_ground(), low_tiger_shot::SETTLE, crouch_boxes, 0, 0);
            s.end();
            at
        };
        let mut low_first = still(crouch, low_tiger_shot::CROUCH, crouch_boxes);
        low_first.flags = 0x20;
        let h = header(&low_first.bytes(), crouch.top_on_ground(), 0xc4, 0x1b, [0, 0, 0], low_shot_stream);
        put_header(&mut region, base, 0x1b, &h);

        // Tiger Uppercut uses Sagat's own lunging punch (0x2d00) on a
        // rising arc. Tiger Genocide leads with his forward-leaping knee.
        let punch = LOW_STEP_PUNCH + 0x20 + 6;
        let ready = (rom_pose(rom, RISING_FRAME), rom_boxes(rom, RISING_FRAME));
        let fall = (rom_pose(rom, JUMP_LIFT_FRAME), rom_boxes(rom, JUMP_LIFT_FRAME));
        let uppercut = [(punch, 4), (punch, 8)];
        write_rising_special(rom, &mut region, SF2_SPECIAL_SLOT, 3, ready, fall, &[uppercut], [0xa2, 0, 0]);
        write_rising_special(
            rom,
            &mut region,
            SF2_SUPER_SLOT,
            4,
            ready,
            fall,
            &[[(JUMP_KNEE_FRAME, 4), (JUMP_KNEE_FRAME, 8)], uppercut],
            [0xa2, 0, 0],
        );

        // The Tiger Knee (`0x19`, dragon punch and kick; see [`tiger_knee`]):
        // his 1987 forward-leaping knee and kick (`0x140e0` / `0x14242`)
        // re-timed to ST / USF4 Tiger Knee startup and recovery so it connects
        // at close and mid range and cancels cleanly from his normals.
        let rising = rom_pose(rom, RISING_FRAME);
        let rising_boxes = rom_boxes(rom, RISING_FRAME);
        let lift = rom_pose(rom, JUMP_LIFT_FRAME);
        let lift_boxes = rom_boxes(rom, JUMP_LIFT_FRAME);
        let mut tk_lift_boxes = lift_boxes;
        tk_lift_boxes[2] = 12;
        let knee_rom = rom_pose(rom, JUMP_KNEE_FRAME);
        let tk_knee_boxes = rom_boxes(rom, JUMP_KNEE_FRAME);
        let mut tk_knee_idle_boxes = tk_knee_boxes;
        tk_knee_idle_boxes[12..16].fill(0);
        let kick = geom("tiger_knee_high_kick")?;
        let tk_stream = {
            let mut s = Stream::new(&mut region, rising, rising.top_on_ground());
            let at = s.call(0x45a7a, 0);
            let lift_top = s.top;
            s.travel(
                lift,
                lift_top,
                tiger_knee::LIFT,
                (tiger_knee::LIFT_DY, tiger_knee::SPEED),
                tk_lift_boxes,
                0x20,
                0,
            );
            s.call(routine::SOUND, 0xa2);
            let knee_top = s.top;
            s.travel(
                knee_rom,
                knee_top,
                tiger_knee::KNEE,
                (tiger_knee::KNEE_DY, tiger_knee::SPEED),
                tk_knee_boxes,
                0xa0,
                tiger_knee::HIT,
            );
            let kick_top = s.top;
            s.travel(
                kick,
                kick_top,
                tiger_knee::KICK,
                (tiger_knee::KICK_DY, tiger_knee::SPEED),
                tk_knee_boxes,
                0x80,
                tiger_knee::HIT,
            );
            s.call(routine::FALLING, 0);
            let fall_knee_top = s.top;
            s.travel(
                knee_rom,
                fall_knee_top,
                tiger_knee::FALL_KNEE,
                (tiger_knee::FALL_DY, tiger_knee::FALL_SPEED),
                tk_knee_idle_boxes,
                0,
                0,
            );
            let fall_lift_top = s.top;
            s.travel(
                lift,
                fall_lift_top,
                tiger_knee::FALL_LIFT,
                (tiger_knee::FALL_DY, 2),
                lift_boxes,
                0,
                0,
            );
            s.call(routine::LANDING, 0);
            s.hold(rising, rising.top_on_ground(), tiger_knee::LAND, rising_boxes, 0, 0);
            s.hold(stance, stance.top_on_ground(), tiger_knee::SETTLE, stance_boxes, 0, 0);
            s.end();
            at
        };
        let mut tk_first = still(rising, tiger_knee::SQUAT, rising_boxes);
        tk_first.flags = 0x20;
        let h = header(&tk_first.bytes(), rising.top_on_ground(), 0xc2, 0x19, [0, 0x05, 0x19], tk_stream);
        put_header(&mut region, base, 0x19, &h);

        // His jumps (`0x09` neutral, `0x0a` forward, `0x0b` back) and jump
        // attacks (`0x20..=0x26`): his ROM table has no jump headers (`0x09..=0x0b`
        // are empty), so his jump streams and headers are built from his ROM
        // rising (`0x2d80`), lift (`0x2c8c`) and knee (`0x2d0a`, with its
        // attack box cleared) poses, rising 80 pixels from `top = 104` to
        // `apex = 24`.
        let jump_top = geom("jump_knee")?;
        let mut top_boxes = rom_boxes(rom, JUMP_KNEE_FRAME);
        top_boxes[12..16].fill(0);
        let air = Air {
            rom,
            jump_top,
            top_boxes,
            fall: lift,
            fall_boxes: lift_boxes,
            landing: rising,
            landing_boxes: rising_boxes,
            stance,
            stance_boxes,
        };
        let jp = geom("jump_punch")?;
        let jpd = geom("jump_punch_down")?;
        let jk = geom("jump_kick")?;
        let jkh = geom("jump_kick_high")?;
        let jkd = geom("jump_kick_down")?;
        let apex = 24;
        let s20 = air.attack(&mut region, apex, jp, boxes("jump_punch").bytes(&jp), hit::JUMP_PUNCH, 0, [3, 4, 0], 0xa1)?;
        let s21 = air.attack(&mut region, apex, jpd, boxes("jump_punch_down").bytes(&jpd), hit::JUMP_PUNCH, 4, [3, 4, 0], 0xa1)?;
        let s22 = air.attack(&mut region, apex, jpd, boxes("jump_punch_down").bytes(&jpd), hit::JUMP_PUNCH, -4, [3, 4, 0], 0xa1)?;
        let s23 = air.attack(&mut region, apex, jk, boxes("jump_kick").bytes(&jk), hit::JUMP_KICK, 0, [3, 4, 0], 0xa2)?;
        let s26 = air.attack(&mut region, apex, jkh, boxes("jump_kick_high").bytes(&jkh), hit::JUMP_KICK, 0, [3, 4, 0], 0xa2)?;
        let s24 = air.attack(&mut region, apex, jkd, boxes("jump_kick_down").bytes(&jkd), hit::JUMP_KICK, 4, [3, 4, 0], 0xa2)?;
        let s25 = air.attack(&mut region, apex, jkd, boxes("jump_kick_down").bytes(&jkd), hit::JUMP_KICK, -4, [3, 4, 0], 0xa2)?;

        let landing_centre = air.landing.top_on_ground() + air.landing.half().1;
        let apex_routine = put_apex_record(
            &mut region,
            base,
            landing_centre,
            [
                [(0x20, s20), (0x20, s20), (0x20, s20)],
                [(0x21, s21), (0x21, s21), (0x21, s21)],
                [(0x22, s22), (0x22, s22), (0x22, s22)],
                [(0x23, s23), (0x23, s23), (0x26, s26)],
                [(0x24, s24), (0x24, s24), (0x24, s24)],
                [(0x25, s25), (0x25, s25), (0x25, s25)],
            ],
        );
        for (id, step_dx) in [(0x09u8, 0), (0x0a, 4), (0x0b, -4)] {
            let jump_stream = {
                let mut s = Stream::new(&mut region, rising, rising.top_on_ground());
                let at = s.call(0x45a7a, 0);
                s.hold(rising, rising.top_on_ground(), 3, rising_boxes, 0, 0);
                s.travel(lift, rising.top_on_ground(), 5, (-8, step_dx), lift_boxes, 0, 0);
                let knee_top = s.top;
                s.travel(jump_top, knee_top, 5, (-8, step_dx), top_boxes, 0, 0);
                s.call(apex_routine, 0xff);
                let apex_top = s.top;
                s.travel(jump_top, apex_top, 4, (0, step_dx), top_boxes, 0, 0);
                s.travel(jump_top, apex_top, 5, (8, step_dx), top_boxes, 0, 0);
                s.call(routine::FALLING, 0);
                let fall_top = s.top;
                s.travel(lift, fall_top, 5, (8, step_dx), lift_boxes, 0, 0);
                s.hold(rising, rising.top_on_ground(), 1, rising_boxes, 0, 0);
                s.call(routine::LANDING, 0);
                s.hold(rising, rising.top_on_ground(), 3, rising_boxes, 0, 0);
                s.hold(stance, stance.top_on_ground(), 1, stance_boxes, 0, 0);
                s.end();
                at
            };
            let h = header(
                &still(rising, 1, rising_boxes).bytes(),
                rising.top_on_ground(),
                0x80,
                id,
                [0x82, 0x05, 0x19],
                jump_stream,
            );
            put_header(&mut region, base, id, &h);
        }
        fits(region)
    }

    /// The High Tiger Shot's numbers (`0x1a`, quarter-circle forward and
    /// punch; his 1987 stream `0x144b4` tuned against
    /// `docs/research/sf1plus-reference-numbers.md` — ST / USF4 Sagat's High
    /// Tiger Shot).
    ///
    /// - **Startup 8**: his wind-up (`0x2f00`, 7 frames) and release frame
    ///   (`0x2f06`, 1 frame) before the projectile spawns on the 9th frame.
    /// - **Projectile (`0x1873c`)**: travels horizontally at 6 pixels a frame,
    ///   scoring 1,000 points (`0x436a8`) and dealing 10 hit / 1 chip damage
    ///   under `balanced_roster` (30 / 15 in `Faithful 1987`).
    /// - **Recovery 28**: 20 frames holding `0x2f06` after the spawn (down from
    ///   the CPU stream's 51-frame wait), 7 frames of `0x2f00` and 1 frame of
    ///   stance (`0x2c00`).
    pub mod high_tiger_shot {
        pub const WINDUP: u8 = 7;
        pub const RELEASE: u8 = 1;
        pub const HOLD: u8 = 20;
        pub const RETRACT: u8 = 7;
        pub const SETTLE: u8 = 1;
    }

    /// The Low Tiger Shot's numbers (`0x1b`, quarter-circle forward and kick;
    /// his catalogue's poses `crouch`, `low_tiger_shot_1`, `low_tiger_shot_2`,
    /// `crouch`, tuned against `docs/research/sf1plus-reference-numbers.md` —
    /// ST / USF4 / CvS2 Sagat's Ground Tiger Shot).
    ///
    /// - **Startup 11**: his crouch 4 (`crouch`), low chamber 6
    ///   (`low_tiger_shot_1`) and low release 1 (`low_tiger_shot_2`) before the
    ///   projectile spawns on the 12th frame (3 frames slower than High Tiger
    ///   Shot).
    /// - **Projectile**: his Tiger Shot template (`0x1873c`) spawned from
    ///   `low_tiger_shot_2`'s crouching grid (`top = 147`, 43 pixels lower
    ///   than his standing grid's `104`) plus [`DROP_Y`] (8 pixels), flying 51
    ///   pixels lower than the High Tiger Shot so it hits crouching opponents
    ///   cleanly.
    /// - **Recovery 29**: 15 remaining frames holding `low_tiger_shot_2` (`1 +
    ///   15 = 16` total) and 14 frames in `crouch`.
    pub mod low_tiger_shot {
        pub const CROUCH: u8 = 4;
        pub const CHAMBER: u8 = 6;
        pub const HOLD: u8 = 16;
        pub const SETTLE: u8 = 14;
        pub const DROP_Y: i16 = 8;
    }

    /// The Tiger Knee's numbers (`0x19`, dragon punch and kick; his 1987
    /// forward-leaping knee and kick `0x140e0` / `0x14242` tuned against
    /// `docs/research/sf1plus-reference-numbers.md` — ST / USF4 Sagat's Tiger
    /// Knee).
    ///
    /// - **Startup 6**: his take-off squat 3 (`0x2d80`) and rising lift 3
    ///   (`0x2c8c` at `dy = -4, dx = 4`); his rising knee (`0x2d0a`) is out on
    ///   the 7th frame.
    /// - **Active 6 + 6**: 6 rising knee frames (`0x2d0a` at `dy = -3, dx = 4`)
    ///   and 6 high kick frames (built from `0x2d84`, `dy = -1, dx = 4`), both with his
    ///   1987 leap's hit (`0x480d0`, damage 13 deep and 10 otherwise, guarded
    ///   standing or crouching via `low_in_air: true`, scoring as the
    ///   Shoryuken). Under `true_combos` the one-hit-per-move rule stops after
    ///   whichever strike connects first.
    /// - **Recovery 22**: 6 descending knee frames (`0x2d0a` at `dy = 3`), 6
    ///   descending lift frames (`0x2c8c` at `dy = 3`), 4 landing squat frames
    ///   (`0x2d80`) and 6 stance frames (`0x2c00`).
    pub mod tiger_knee {
        pub const SQUAT: u8 = 3;
        pub const LIFT: u8 = 3;
        pub const LIFT_DY: i32 = -4;
        pub const KNEE: u8 = 6;
        pub const KNEE_DY: i32 = -3;
        pub const KICK: u8 = 6;
        pub const KICK_DY: i32 = -1;
        pub const SPEED: i32 = 4;
        pub const FALL_KNEE: u8 = 6;
        pub const FALL_LIFT: u8 = 6;
        pub const FALL_DY: i32 = 3;
        pub const FALL_SPEED: i32 = 3;
        pub const LAND: u8 = 4;
        pub const SETTLE: u8 = 6;
        /// His 1987 forward-leaping knee's hit (`0x480d0`).
        pub const HIT: u32 = 0x480d0;
    }
}

/// Build a sparse table for the native special command tests.
#[cfg(test)]
pub(crate) fn test_special_table(slots: &[(u8, SpecialSlot)]) -> Vec<u8> {
    let mut region = new_slot(TournamentFighter::Ryu);
    let base = region.base;
    let mut header = [0; MOVE_HEADER_SIZE as usize];
    header[8] = 1;
    for &(id, special) in slots {
        put_header(&mut region, base, id, &header);
        set_special(&mut region, base, id, special);
    }
    region.bytes
}

#[cfg(test)]
mod tests {
    #[test]
    fn online_bank_identity_includes_authored_throw_pose_direction() {
        let mut bank = super::DataBank::default();
        bank.throws[0] = Some(super::ThrowEntry {
            throw_header: super::DATA_BANK_BASE,
            thrown_header: super::DATA_BANK_BASE + 0x30,
            back_turned_poses: 0,
        });
        let identity = |bank: &super::DataBank| {
            let mut hash = crate::canonical::CanonicalHasher::new();
            bank.encode_compatibility(&mut hash);
            hash.finish()
        };
        let ordinary = identity(&bank);
        bank.throws[0].as_mut().unwrap().back_turned_poses = 1;
        assert_ne!(identity(&bank), ordinary, "authored pose direction changes bank compatibility");
    }

    use super::*;

    #[test]
    fn canonical_alpha_fighters_have_grounded_invulnerable_counter_streams() {
        use sf_video::art::{load_art_from, ArtSources};
        use sf_video::art_pack::ArtPack;
        use sf_video::romset::load_assets;
        use std::path::Path;

        let roms = Path::new("../../tools/reference/local/roms/sf");
        let pack_path = Path::new("../../assets/sf1-port-art");
        if !roms.is_dir() || !pack_path.is_dir() {
            eprintln!("skipping: local ROM or art pack is missing");
            return;
        }
        let assets = load_assets(roms, "../../tools/reference/rom-manifest.json").unwrap();
        let pack = ArtPack::open(pack_path).unwrap();
        let recipes = [
            include_str!("../../../art/recipes/adon.json"),
            include_str!("../../../art/recipes/birdie.json"),
            include_str!("../../../art/recipes/eagle.json"),
            include_str!("../../../art/recipes/gen.json"),
            include_str!("../../../art/recipes/sagat.json"),
        ];
        let reviews = [
            include_str!("../../../art/review/adon.json"),
            include_str!("../../../art/review/birdie.json"),
            include_str!("../../../art/review/eagle.json"),
            include_str!("../../../art/review/gen.json"),
            include_str!("../../../art/review/sagat.json"),
        ];
        let sources = ArtSources { recipes: &recipes, reviews: Some(&reviews), pack: Ok(&pack) };
        let art = load_art_from(&assets, &sources).unwrap();
        let bank = DataBank::build(&assets.maincpu, &art.catalogues);
        let long = |at| u32::from_be_bytes(std::array::from_fn(|i| bank.rb(at + i as u32)));
        for fighter in TournamentFighter::ALL {
            let header = bank.move_header(fighter, ALPHA_COUNTER_SLOT);
            if crate::era::what_if_sfz(fighter) {
                assert_eq!(header, None, "{fighter:?}");
                continue;
            }
            let header = header.unwrap_or_else(|| panic!("missing {fighter:?} counter"));
            assert_eq!(bank.rb(header + 0xa), ALPHA_COUNTER_SLOT);
            assert_eq!(bank.rb(header + 0xe), 2, "{fighter:?} startup");
            assert_eq!(bank.rb(header + 0xf), 0x08, "{fighter:?} startup invulnerability");
            let active = long(header + 0x2c);
            let recovery = active + 0x20;
            assert_eq!(bank.rb(active + 1), 3, "{fighter:?} active duration");
            assert_eq!(bank.rb(active + 0x1a), 0x88, "{fighter:?} active invulnerability");
            let hit = long(active + 0x1c);
            assert_eq!(bank.rb(hit + 1), 5, "{fighter:?} damage");
            assert_eq!(bank.rb(hit + 2), 0x35, "{fighter:?} standing blowback");
            assert_eq!(bank.rb(hit + 3), 0x35, "{fighter:?} crouching blowback");
            assert_eq!(bank.rb(recovery + 0x1a), 0, "{fighter:?} vulnerable recovery");

            let (windup, _) = header_pose(&bank.bytes, header - DATA_BANK_BASE);
            let strike = rom_pose(&bank.bytes, active - DATA_BANK_BASE);
            let retract = rom_pose(&bank.bytes, recovery - DATA_BANK_BASE);
            assert_eq!(windup.top_on_ground() + windup.height(), GROUND, "{fighter:?} startup");
            assert_eq!(windup.half().1 - strike.half().1, i32::from(bank.rb(active + 6) as i8));
            assert_eq!(strike.half().1 - retract.half().1, i32::from(bank.rb(recovery + 6) as i8));
        }
    }

    #[test]
    fn alpha_shoto_super_streams_have_five_hits_and_land_on_the_floor() {
        use sf_video::romset::load_assets;
        use std::path::Path;

        let roms = Path::new("../../tools/reference/local/roms/sf");
        if !roms.is_dir() {
            eprintln!("skipping: local ROM set is missing");
            return;
        }
        let assets = load_assets(roms, "../../tools/reference/rom-manifest.json").unwrap();
        let bank = DataBank::build(&assets.maincpu, &[]);
        let long = |at| u32::from_be_bytes(std::array::from_fn(|i| bank.rb(at + i as u32)));
        for fighter in [TournamentFighter::Ryu, TournamentFighter::Ken] {
            for slot in [ALPHA_SUPER_SECOND_SLOT, ALPHA_SUPER_THIRD_SLOT] {
                let h = bank.move_header(fighter, slot).unwrap();
                assert_eq!(bank.rb(h + 0xa), slot);
                let (mut pose, _) = header_pose(&bank.bytes, h - DATA_BANK_BASE);
                let mut center = pose.top_on_ground() + pose.half().1;
                let mut frame = long(h + 0x2c);
                let mut hit_count = 0;
                for _ in 0..64 {
                    match bank.rb(frame) {
                        0xfd => break,
                        0xff => {
                            let call = long(frame + 2);
                            if in_bank(call) {
                                assert_eq!(bank.rb(call), ROUTINE_POSE_ORIGIN);
                                let dy = i16::from_be_bytes([bank.rb(call + 2), bank.rb(call + 3)]);
                                center += i32::from(dy);
                            }
                            frame += 6;
                        }
                        hold => {
                            pose = rom_pose(&bank.bytes, frame - DATA_BANK_BASE);
                            let dy = i32::from(bank.rb(frame + 6) as i8);
                            for _ in 0..=hold {
                                center += dy;
                                assert!(
                                    center + pose.half().1 <= GROUND,
                                    "{fighter:?} {slot:#04x} frame {frame:#x} places bottom at {} (dy {dy}, hold {hold})",
                                    center + pose.half().1,
                                );
                            }
                            if bank.rb(frame + 0x1a) & 0x80 != 0 {
                                let hit = long(frame + 0x1c);
                                assert!(in_bank(hit));
                                assert_ne!(bank.rb(frame + 0x19), 0);
                                hit_count += 1;
                                assert_eq!(bank.rb(hit + 1), if hit_count == 5 { 5 } else { 4 });
                            }
                            frame += 0x20;
                        }
                    }
                }
                assert_eq!(bank.rb(frame), 0xfd, "{fighter:?} {slot:#04x} ends");
                assert_eq!(hit_count, 5);
                assert_eq!(center + pose.half().1, GROUND, "{fighter:?} {slot:#04x} landing");
            }
        }
    }

    /// A table with the three special slots present and their records.
    fn table(slots: [[u8; SPECIAL_SLOT_SIZE as usize]; 3]) -> Region {
        let mut region = new_slot(TournamentFighter::Ryu);
        let base = region.base;
        let mut header = [0; MOVE_HEADER_SIZE as usize];
        header[8] = 1;
        for (id, record) in SPECIAL_SLOTS.into_iter().zip(slots) {
            put_header(&mut region, base, id, &header);
            region.patch(base + SPECIALS + (id - 0x19) as u32 * SPECIAL_SLOT_SIZE, &record);
        }
        region
    }

    #[test]
    fn move_pointers_reach_slot_3f_without_overlapping_apex_or_specials() {
        let mut region = new_slot(TournamentFighter::Sagat);
        let base = region.base;
        let mut header = [0; MOVE_HEADER_SIZE as usize];
        header[8] = 1;
        header[0xa] = 0x3f;
        put_header(&mut region, base, 0x3f, &header);
        let apex = region.put(&[ROUTINE_JUMP_APEX]);
        region.patch(base + APEX_POINTER, &apex.to_be_bytes());
        let rb = |addr: u32| region.bytes[(addr - base) as usize];
        let rl = |addr| u32::from_be_bytes(std::array::from_fn(|i| rb(addr + i as u32)));

        assert_eq!(APEX_POINTER, 0x100);
        assert_eq!(SPECIALS, 0x104);
        let address = move_header(rl, base, 0x3f).expect("last slot's header");
        assert_eq!(address, rl(base + 0xfc));
        assert_eq!(rb(address + 0xa), 0x3f);
        assert_eq!(rl(base + APEX_POINTER), apex);
        assert_eq!(move_header(rl, base, 0x3e), None, "empty slot");
        assert_eq!(move_header(rl, base, 0x40), None, "no slot past the table");
        assert_eq!(move_header(|_| u32::MAX, base, 0x29), None, "unmapped bank bytes");
        assert_eq!(move_header(|_| base + FIGHTER_SPAN, base, 0x29), None, "cross-fighter pointer");
        assert_eq!(move_header(|_| panic!("ROM is inline"), 0x1605c, 0x1a), Some(0x1653c));
    }

    #[test]
    fn a_tables_special_slots_are_checked_when_built() {
        let ryu = SPECIAL_SLOTS.map(|s| SpecialSlot::ryu(s).bytes());
        assert_eq!(validate_specials(&table(ryu)), Ok(()));
        // A command kind it does not know (0: none; 0x40: a mash counter,
        // not one yet).
        for command in [0u8, 0x40] {
            let mut slots = ryu;
            slots[1][0] = command;
            let why = validate_specials(&table(slots)).unwrap_err();
            assert!(why.contains("special 0x1a has no command"), "{why}");
        }
        // Two slots on the same tracker and button.
        let mut slots = ryu;
        slots[2] = SpecialSlot { tracker: 0x1c, kick: false, ..SpecialSlot::ryu(0x1b) }.bytes();
        let why = validate_specials(&table(slots)).unwrap_err();
        assert!(why.contains("specials 0x1a and 0x1b are on the same command"), "{why}");
        // ... but the same tracker with the other button is another command.
        slots[2] = SpecialSlot { tracker: 0x1c, kick: true, ..SpecialSlot::ryu(0x1b) }.bytes();
        assert_eq!(validate_specials(&table(slots)), Ok(()));
        // Unknown flags.
        let mut slots = ryu;
        slots[0][1] = 0x80;
        assert!(validate_specials(&table(slots)).unwrap_err().contains("flags"));
        // An absent slot's record is not checked.
        let mut region = table([[0; 8]; 3]);
        for slot in SPECIAL_SLOTS {
            region.bytes[bank_header_offset(slot) as usize + 8] = 0;
        }
        assert_eq!(validate_specials(&region), Ok(()));
    }

    #[test]
    fn a_slot_record_reads_back_and_a_rom_table_has_the_1987_three() {
        let geki = SpecialSlot { tracker: 0x16, kick: false, melee: true, low_in_air: false, score: 0x46012 };
        let region = table([SpecialSlot::ryu(0x19).bytes(), SpecialSlot::ryu(0x1a).bytes(), geki.bytes()]);
        let rb = |a: u32| region.bytes.get((a - region.base) as usize).copied().unwrap_or(0);
        assert_eq!(special_slot(rb, DATA_BANK_BASE, 0x1b), geki);
        assert_eq!(special_command(rb, DATA_BANK_BASE, 0x1b), (0x16, false));
        for slot in SPECIAL_SLOTS {
            assert_eq!(special_slot(|_| 0xff, 0x1605c, slot), SpecialSlot::ryu(slot), "a ROM table reads no record");
        }
    }
}
