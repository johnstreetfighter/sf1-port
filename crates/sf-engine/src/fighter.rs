//! Translated 68000 gameplay routines (tickets #9, #12, #15, #16).
//!
//! Each function below is a traceable translation of one original routine
//! of the US set 1 program (`sf`, sfd-19/22/20/23/21/24), named after what it
//! does and annotated with its original address. They operate on the
//! original memory layout through [`Bus`]: work RAM 0xff8000-0xffdfff,
//! object RAM 0xffe000-0xffffff (fighter and effect entities share it with
//! the sprite list: entity `+0..+7` is the sprite entry, `+8..+0x3f` the
//! object state), the text layer at 0x800000, and the program ROM, which is
//! read only as data (move tables, frame records, damage tables). No
//! original code is executed.
//!
//! Register conventions are kept where they carry meaning: `me` / `other`
//! are the entity offsets from A4 = 0xffe000 that the original keeps in the
//! low and high words of D1 (the two fighter regions 0x400 and 0xa00,
//! effects 0x100 + 0x40k). Which fighter is in which region is not fixed:
//! the frame-flag swap (0x3b852) trades them, so fighters are always found
//! through their player structure's entity words (+0xe). 68000 byte/word
//! arithmetic wraps; signed and unsigned
//! comparisons follow the branch used by the original (`bgt`/`blt`/`bge`
//! signed, `bhi`/`bcs`/`bcc`/`bls` unsigned).
//!
//! Branches outside the bounded slice are not guessed: reaching one records
//! its original address in [`Bus::unrecovered`] and the routine returns,
//! so a run can prove it never left the translated paths (the acceptance
//! tests assert the set stays empty).
//!
//! Recovery evidence: docs/reference/punch.md, cpu-decision.md,
//! posture-dispatch.md, slot-swap.md.

use std::collections::BTreeSet;

use crate::combo::HitSource;
use crate::config::EvilRyuPlaytest;
use crate::data_bank::{
    KAGE_ASHURA_AIR_BACK, KAGE_ASHURA_AIR_FORWARD, KAGE_ASHURA_BACK, KAGE_ASHURA_FORWARD,
    KAGE_MISOGI, KAGE_SEKIEIKEN, KAGE_SHAKUNETSU,
};
use crate::machine::MachineState;
use crate::roster::PlayMode;

/// A4: object RAM base (entity offsets are relative to it).
pub const OBJ_BASE: u32 = 0xffe000;
/// A5: work RAM base.
pub const WORK_BASE: u32 = 0xff8000;
/// Player structures (P1 0xff86b0, P2 0xff86da): +0/+1 latched punch/kick
/// strength, +2 input-byte pointer, +6/+7 health remainder/cells, +0xa
/// health-bar text offset, +0xe entity offsets (other << 16 | self), +0x12
/// move table, +0x20 score text offset, +0x22..+0x29 score digits.
pub const P1_STRUCT: u32 = 0xff86b0;
pub const P2_STRUCT: u32 = 0xff86da;
/// The CPU opponent's player structure (A5+0x4664), used in one-player
/// play (game mode 2) in place of P2's; +0x16..+0x1b hold the AI's range
/// roll (see [`Bus::cpu_decide`]).
pub const CPU_STRUCT: u32 = 0xffc664;
pub const DRAMATIC_ALLY2: u16 = 0x1000;
pub(crate) const DRAMATIC_PLAYERS: [u32; 3] = [P1_STRUCT, CPU_STRUCT, P2_STRUCT];

// Addressable backing for the translated latch and tracker routines. The
// boss cannot borrow P2's work RAM while both fighters are active.
const BOSS_LATCHES: u32 = 0x600000;
const BOSS_TRACKERS: u32 = BOSS_LATCHES + 16;
/// The fighters' entity regions at the fixtures' starts (P1 in 0x400, P2 or
/// the CPU in 0xa00); after a slot swap they are exchanged.
pub const RYU: u16 = 0x400;
pub const KEN: u16 = 0xa00;

/// Work RAM globals (A5-relative in the original).
pub(crate) const MODE_DEMO: u32 = 0xff8003; // ($3,A5): 0 in attract/demo play
const SOUND_READ: u32 = 0xff865e; // sound-queue read index
const SOUND_WRITE: u32 = 0xff8660; // sound-queue write index
const SOUND_QUEUE: u32 = 0xff8662; // 16-word ring of sound commands
const IN1_COPY: u32 = 0xff868e; // vblank copy of IN1 (P2 byte, P1 byte)
pub(crate) const RNG_STATE: u32 = 0xff8e4a; // 8-byte random state
pub const GAME_MODE: u32 = 0xffc002; // ($4002,A5): 1 = 2P versus, 2 = 1P vs CPU
/// Training's health floor: the points a hit always leaves (ADR-0013).
pub const TRAINING_HEALTH_FLOOR: u16 = 1;
/// Private Ashura action IDs in slots unused by the SFII and Alpha Ryu banks.
pub const EVIL_RYU_ASHURA_FORWARD: u8 = 0x2d;
pub const EVIL_RYU_ASHURA_BACK: u8 = 0x2f;
/// The SF4 bank uses 0x2f for Focus release. Evil Ryu has no move in 0x29.
pub const EVIL_RYU_SF4_ASHURA_BACK: u8 = 0x29;
/// The private Raging Demon rush and whiff recovery. A grab uses the normal throw ID.
pub const EVIL_RYU_RAGING_DEMON: u8 = 0x2c;
/// The same rush in Kage's second loadout, where 0x2c is Ryusokyaku.
pub const KAGE_RAGING_DEMON: u8 = 0x2b;
/// Private SF6 Demon uses the Denjin slot so Drive Rush keeps 0x2c.
pub const SF6_RAGING_DEMON: u8 = 0x20;
const DEMON_RUSH_FRAMES: u8 = 12;
const DEMON_WHIFF_FRAMES: u8 = 16;
const DEMON_BLACKOUT_FRAMES: u8 = 60;
const DEMON_SPARK_INTERVAL: u8 = 10;
const DEMON_VICTORY_FRAMES: u8 = 90;
const DEMON_REVEAL_GAP: u16 = 40;
const MODERN_DEMON_GRAB_RANGE: u16 = 56;
const RYU_VICTORY_POSE: u32 = 0x009a8;
const DEMON_DAMAGE: u8 = 20;
const DEMON_SPARK: u32 = 0x18984;
/// An otherwise unused byte in each native effect slot.
pub(crate) const DEMON_SPARK_MARKER: usize = 0x2e;
const GUARD_SPECIAL_PRESSURE: u8 = 8;
pub const SCENE_MODE: u32 = 0xffc008; // ($4008,A5): 0x80 during the fight
const DRIFT_COUNT: u32 = 0xffc010; // ($4010,A5): stage-drift frame counter
const DRIFT_POS: u32 = 0xffc00e; // ($400e,A5): stage-drift position
pub(crate) const CAMERA: u32 = 0xffc00c; // ($400c,A5): camera x (0..=0x180)
const HISCORE: u32 = 0xffc012; // ($4012,A5): top-score digits
const FG_SCROLL_COPY: u32 = 0xffc680; // ($4680,A5): fg_scroll register copy
const BG_SCROLL_COPY: u32 = 0xffc682; // ($4682,A5): bg_scroll register copy
/// The CPU opponent's AI tables used by 0x40bd4 (A5+0x4058..0x40b3), part of
/// the per-opponent block 0x44880 copies to A5+0x4058..0x4663 before the
/// round: range classes, preferred ranges, distance thresholds, and the
/// near / middle / far move tables indexed by a random nibble.
const AI_RANGE_CLASS: u32 = 0xffc058;
const AI_RANGE: u32 = 0xffc068;
const AI_NEAR_LIMIT: u32 = 0xffc06e;
const AI_MID_LIMIT: u32 = 0xffc070;
const AI_NEAR_MOVES: u32 = 0xffc074;
const AI_MID_MOVES: u32 = 0xffc084;
const AI_FAR_MOVES: u32 = 0xffc094;
const AI_STUNNED_MOVES: u32 = 0xffc0a4;

/// Hit-function entry points dispatched through entity +0x28.
/// A recovered hit function: its shape and the data it is
/// parameterized with (spark template, damage table, blocked-kick recoil
/// stream).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HitFn {
    /// Damage entry by depth; blocked by the standing (`low` false) or the
    /// crouching block move.
    Basic {
        template: u32,
        table: u32,
        low: bool,
    },
    /// As `Basic` (high), and a block sends the attacker to `blocked`.
    Redirect {
        template: u32,
        table: u32,
        blocked: u32,
    },
    /// Damage entry by the attacker's strength (+0x34: 16 bytes per level)
    /// and depth; blocked low (crouching attacks) or high (jump attacks).
    Strength { spark: Spark, table: u32, low: bool },
    /// The Shoryuken's hit (0x45efa): unblockable, one fixed damage entry.
    Shoryuken { template: u32 },
    /// The Tatsumaki's hit (0x45f5a): no hit on a defender in hit stun,
    /// chip damage on a blocking one.
    Tatsumaki { template: u32 },
    /// The CPU opponent's hits (0x46492.. for Geki, 0x4664a.. for Retsu):
    /// no score; blocked when the defender's state +9 equals `block_state`
    /// (0 disables the test); the damage entry by depth (1 or not).
    Cpu {
        template: u32,
        depth1: u32,
        other: u32,
        block_state: u8,
    },
}

/// How a strength-indexed hit function picks its spark template.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Spark {
    Fixed(u32),
    /// The first for strength level +0x34 <= 3, the second above.
    ByLevel(u32, u32),
    /// The first for move 0x20 (the neutral jump punch), the second else.
    ByMove(u32, u32),
}

/// How a stream-called routine continues the animation step.
enum Resume {
    /// Load the frame after the call entry (0x42cae).
    Load,
    /// Load the grounded pose after a fall or throw and cue its contact.
    LoadMatSlam,
    /// Load the victim's lying frame and reveal the Demon victory pose.
    RevealDemon,
    /// Retry the stream call on the next fighter tick.
    Hold,
    /// Load the frame at this stream address instead (0x42cae with A1 set).
    LoadAt(u32),
    /// Go on with the control-byte loop after the entry (0x42c20).
    Control,
    /// Go on with the control-byte loop at this stream address (0x42c20
    /// with A1 set).
    ControlAt(u32),
    /// Start the animation step over (0x42be2).
    Restart,
}

/// The frame hit functions (+0x28) of the standing attacks. Each original
/// routine is a near-copy of one shape with its own constants; P1's and
/// P2's copies differ only in the spark template (and the recoil stream of
/// the other fighter's move data).
fn hit_function(f: u32) -> Option<HitFn> {
    let basic = |template, table| {
        Some(HitFn::Basic {
            template,
            table,
            low: false,
        })
    };
    let low = |template, table| {
        Some(HitFn::Basic {
            template,
            table,
            low: true,
        })
    };
    let redirect = |template, table, blocked| {
        Some(HitFn::Redirect {
            template,
            table,
            blocked,
        })
    };
    let low_strength = |spark, table| {
        Some(HitFn::Strength {
            spark,
            table,
            low: true,
        })
    };
    let air_strength = |spark, table| {
        Some(HitFn::Strength {
            spark,
            table,
            low: false,
        })
    };
    let cpu = |template, depth1, other, block_state| {
        Some(HitFn::Cpu {
            template,
            depth1,
            other,
            block_state,
        })
    };
    match f {
        0x45af0 => basic(0x184fc, 0x45b6e), // light punch
        0x45afa => basic(0x1875c, 0x45b6e),
        0x45c0c => basic(0x1853c, 0x45c88), // medium punch
        0x45c16 => basic(0x1879c, 0x45c88),
        0x45c98 => basic(0x1853c, 0x45d16), // heavy punch
        0x45ca2 => basic(0x1879c, 0x45d16),
        0x4602a => basic(0x185bc, 0x460a8), // light kick
        0x46034 => basic(0x1881c, 0x460a8),
        0x460b8 => redirect(0x185dc, 0x46154, 0xf4a), // medium kick
        0x460d8 => redirect(0x1883c, 0x46154, 0x3028),
        0x46164 => redirect(0x1861c, 0x46240, 0x10b4), // heavy kick, frame 1
        0x46184 => redirect(0x185fc, 0x46240, 0x1094), // heavy kick, frame 2
        0x461a4 => redirect(0x1887c, 0x46240, 0x3192),
        0x461c4 => redirect(0x1885c, 0x46240, 0x3172),
        0x45d26 => low_strength(Spark::ByLevel(0x1855c, 0x1857c), 0x45de0), // crouching punches
        0x45d44 => low_strength(Spark::ByLevel(0x187bc, 0x187dc), 0x45de0),
        0x46250 => low_strength(Spark::Fixed(0x1863c), 0x462e0), // crouching light/medium kick
        0x4625a => low_strength(Spark::Fixed(0x1889c), 0x462e0),
        0x45e10 => air_strength(Spark::ByMove(0x1859c, 0x185bc), 0x45eca), // jump punches
        0x45e2e => air_strength(Spark::ByMove(0x187fc, 0x1881c), 0x45eca),
        0x4638e => air_strength(Spark::Fixed(0x1867c), 0x46462), // jump kicks
        0x46398 => air_strength(Spark::Fixed(0x188dc), 0x46462),
        0x463ae => air_strength(Spark::Fixed(0x1869c), 0x46462),
        0x463b8 => air_strength(Spark::Fixed(0x188fc), 0x46462),
        0x463ce => air_strength(Spark::Fixed(0x186bc), 0x46462),
        0x463d8 => air_strength(Spark::Fixed(0x1891c), 0x46462),
        0x45efa => Some(HitFn::Shoryuken { template: 0x1853c }),
        0x45f04 => Some(HitFn::Shoryuken { template: 0x1879c }),
        0x45f5a => Some(HitFn::Tatsumaki { template: 0x186bc }),
        0x45f64 => Some(HitFn::Tatsumaki { template: 0x1891c }),
        // Geki's attacks. The second blocks against state 0x24.
        0x46492 => cpu(0x18984, 0x464f8, 0x464f0, 0x22),
        0x46500 => cpu(0x18984, 0x46566, 0x4655e, 0x24),
        0x4656e => cpu(0x18984, 0x465d4, 0x465cc, 0x22),
        0x465dc => cpu(0x18b04, 0x46642, 0x4663a, 0x22),
        // Retsu's attacks: seven copies, 0x6e bytes apart; the last has no
        // block test (its stream 0xcf1a, move 0x18).
        0x4664a => cpu(0x18984, 0x466b0, 0x466a8, 0x22),
        0x466b8 => cpu(0x18964, 0x4671e, 0x46716, 0x22),
        0x46726 => cpu(0x18a24, 0x4678c, 0x46784, 0x22),
        0x46794 => cpu(0x18a44, 0x467fa, 0x467f2, 0x22),
        0x46802 => cpu(0x18ac4, 0x46868, 0x46860, 0x22),
        0x46870 => cpu(0x18b04, 0x468d6, 0x468ce, 0x22),
        0x468de => cpu(0x18aa4, 0x46926, 0x4691e, 0),
        // Mike's attacks, selected by the AI records for move table 0x170dc.
        // 0x4699c blocks against crouching state 0x24.
        0x4692e => cpu(0x18964, 0x46994, 0x4698c, 0x22),
        0x4699c => cpu(0x18984, 0x46a02, 0x469fa, 0x24),
        0x46a0a => cpu(0x18984, 0x46a70, 0x46a68, 0x22),
        0x46a78 => cpu(0x18984, 0x46ade, 0x46ad6, 0x22),
        // Joe's attacks, referenced by the streams in move table 0x1734c.
        // 0x46bc2 and 0x46c9e block against crouching state 0x24.
        0x46ae6 => cpu(0x18984, 0x46b4c, 0x46b44, 0x22),
        0x46b54 => cpu(0x18984, 0x46bba, 0x46bb2, 0x22),
        0x46bc2 => cpu(0x18984, 0x46c28, 0x46c20, 0x24),
        0x46c30 => cpu(0x18984, 0x46c96, 0x46c8e, 0x22),
        0x46c9e => cpu(0x18a24, 0x46d04, 0x46cfc, 0x24),
        0x46d0c => cpu(0x18a44, 0x46d72, 0x46d6a, 0x22),
        0x46d7a => cpu(0x18ac4, 0x46de0, 0x46dd8, 0x22),
        0x46de8 => cpu(0x18b04, 0x46e4e, 0x46e46, 0x22),
        // Lee's attacks, selected by the AI records for move table 0x175bc.
        // 0x46e56 uses spark 0x18944; 0x471c6 blocks against crouching state 0x24.
        0x46e56 => cpu(0x18944, 0x46ebc, 0x46eb4, 0x22),
        0x46ec4 => cpu(0x18984, 0x46f2a, 0x46f22, 0x22),
        0x46f32 => cpu(0x18984, 0x46f98, 0x46f90, 0x22),
        0x46fa0 => cpu(0x18a24, 0x47006, 0x46ffe, 0x22),
        0x4700e => cpu(0x18a44, 0x47074, 0x4706c, 0x22),
        0x4707c => cpu(0x18ac4, 0x470e2, 0x470da, 0x22),
        0x470ea => cpu(0x18b04, 0x47150, 0x47148, 0x22),
        0x47158 => cpu(0x18b04, 0x471be, 0x471b6, 0x22),
        0x471c6 => cpu(0x18aa4, 0x4722c, 0x47224, 0x24),
        // Gen's attacks, selected by the AI records for move table 0x1782c.
        // 0x47310 uses spark 0x189a4; 0x47310 and 0x47536 block against crouching state 0x24.
        0x47234 => cpu(0x18984, 0x4729a, 0x47292, 0x22),
        0x472a2 => cpu(0x18984, 0x47308, 0x47300, 0x22),
        0x47310 => cpu(0x189a4, 0x47376, 0x4736e, 0x24),
        0x4737e => cpu(0x18984, 0x473e4, 0x473dc, 0x22),
        0x473ec => cpu(0x18ac4, 0x47452, 0x4744a, 0x22),
        0x4745a => cpu(0x18b04, 0x474c0, 0x474b8, 0x22),
        0x474c8 => cpu(0x18b04, 0x4752e, 0x47526, 0x22),
        0x47536 => cpu(0x18aa4, 0x4759c, 0x47594, 0x24),
        // Birdie's attacks, selected by the AI records for move table 0x17a9c
        // (0x47838 is referenced by the unreferenced Birdie jump-kick stream at 0x117fe..0x1199f).
        // 0x47680 and 0x476ee block against crouching state 0x24.
        0x475a4 => cpu(0x18964, 0x4760a, 0x47602, 0x22),
        0x47612 => cpu(0x18984, 0x47678, 0x47670, 0x22),
        0x47680 => cpu(0x18984, 0x476e6, 0x476de, 0x24),
        0x476ee => cpu(0x18964, 0x47754, 0x4774c, 0x24),
        0x4775c => cpu(0x18984, 0x477c2, 0x477ba, 0x22),
        0x477ca => cpu(0x18a04, 0x47830, 0x47828, 0x22),
        0x47838 => cpu(0x18a04, 0x4789e, 0x47896, 0x22),
        0x478a6 => cpu(0x18ac4, 0x4790c, 0x47904, 0x22),
        // Eagle's attacks, selected by the AI records for move table 0x17d0c.
        // 0x479f0 and 0x47ba8 block against crouching state 0x24; 0x47ba8 uses spark 0x189c4.
        0x47914 => cpu(0x18984, 0x4797a, 0x47972, 0x22),
        0x47982 => cpu(0x18984, 0x479e8, 0x479e0, 0x22),
        0x479f0 => cpu(0x18964, 0x47a56, 0x47a4e, 0x24),
        0x47a5e => cpu(0x18984, 0x47ac4, 0x47abc, 0x22),
        0x47acc => cpu(0x18ac4, 0x47b32, 0x47b2a, 0x22),
        0x47b3a => cpu(0x18a04, 0x47ba0, 0x47b98, 0x22),
        0x47ba8 => cpu(0x189c4, 0x47c0e, 0x47c06, 0x24),
        // Adon's attacks, selected by the AI records for move table 0x17f7c.
        // 0x47c84 and 0x47e3c block against crouching state 0x24; 0x47e3c uses spark 0x189c4.
        0x47c16 => cpu(0x18984, 0x47c7c, 0x47c74, 0x22),
        0x47c84 => cpu(0x18964, 0x47cea, 0x47ce2, 0x24),
        0x47cf2 => cpu(0x18984, 0x47d58, 0x47d50, 0x22),
        0x47d60 => cpu(0x18964, 0x47dc6, 0x47dbe, 0x22),
        0x47dce => cpu(0x18ac4, 0x47e34, 0x47e2c, 0x22),
        0x47e3c => cpu(0x189c4, 0x47ea2, 0x47e9a, 0x24),
        // Sagat's attacks, selected by the AI records for move table 0x181ec.
        // 0x47f86 blocks against crouching state 0x24.
        0x47eaa => cpu(0x18984, 0x47f10, 0x47f08, 0x22),
        0x47f18 => cpu(0x18984, 0x47f7e, 0x47f76, 0x22),
        0x47f86 => cpu(0x18964, 0x47fec, 0x47fe4, 0x24),
        0x47ff4 => cpu(0x18984, 0x4805a, 0x48052, 0x22),
        0x48062 => cpu(0x18ac4, 0x480c8, 0x480c0, 0x22),
        0x480d0 => cpu(0x18a04, 0x48136, 0x4812e, 0x22),
        0x46300 => low(0x1865c, 0x4637e), // sweep
        0x4630a => low(0x188bc, 0x4637e),
        _ => None,
    }
}

fn sext8(b: u8) -> u16 {
    b as i8 as i16 as u16
}

/// Program data and work RAM for the translated routines, carried by a
/// scene machine next to its machine-visible state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Program {
    /// 0x000000-0x05ffff, read as data.
    pub rom: Vec<u8>,
    /// 0xff8000-0xffdfff, from the fixture's work dump.
    pub work: Vec<u8>,
    /// Original addresses of untranslated branches reached so far.
    pub unrecovered: BTreeSet<u32>,
    /// Target addresses of effect `0xff` stream calls (`0x42dd2`) reached so far.
    pub effect_stream_calls: BTreeSet<u32>,
    /// Session-level ruleset preset and Tier 1 core overhaul toggles (`#94`, `O3`).
    pub match_config: crate::config::MatchConfig,
    /// Per-session runtime state for Tier 1 core overhaul toggles (`#94`, `O3`).
    pub toggle_state: crate::config::CoreToggleState,
    /// What Training's rules keep between frames (ADR-0013, `#243`).
    pub training: crate::training::TrainingState,
    /// The extended background bank (ADR-0009), for an overhaul fight's
    /// machine only; the stage gates and the palette load read it.
    pub backgrounds: Option<std::sync::Arc<sf_video::background_bank::BackgroundBank>>,
    /// The extended data bank (ADR-0014), for an overhaul fight's machine
    /// only: the bosses' 1987 player tables, read at `0x100000..`.
    pub data_bank: Option<std::sync::Arc<crate::data_bank::DataBank>>,
}

impl Program {
    /// Power-on state: the program region and zeroed work RAM
    /// (0xff8000-0xffdfff).
    pub fn power_on(rom: Vec<u8>) -> Self {
        Program {
            rom,
            work: vec![0; 0x6000],
            unrecovered: BTreeSet::new(),
            effect_stream_calls: BTreeSet::new(),
            match_config: crate::config::MatchConfig::faithful(),
            toggle_state: crate::config::CoreToggleState::default(),
            training: crate::training::TrainingState::default(),
            backgrounds: None,
            data_bank: None,
        }
    }

    /// From the program region and a reference work dump (0xff8000-based,
    /// 0x8000 bytes; the object-RAM tail stays with the machine state).
    pub fn from_fixture(rom: Vec<u8>, work_dump: &[u8]) -> Option<Self> {
        Some(Program {
            rom,
            work: work_dump.get(0..0x6000)?.to_vec(),
            unrecovered: BTreeSet::new(),
            effect_stream_calls: BTreeSet::new(),
            match_config: crate::config::MatchConfig::faithful(),
            toggle_state: crate::config::CoreToggleState::default(),
            training: crate::training::TrainingState::default(),
            backgrounds: None,
            data_bank: None,
        })
    }

    /// The original-memory view over this program data and `state`, with
    /// `dsw1` at the pinned baseline's default (`0xdfff`).
    pub fn bus<'a>(&'a mut self, state: &'a mut MachineState) -> Bus<'a> {
        self.bus_with_dsw1(state, crate::tasks::Io::idle().dsw1)
    }

    /// The original-memory view over this program data and `state`, carrying
    /// the live `dsw1` port word (`0xc00008`).
    pub fn bus_with_dsw1<'a>(&'a mut self, state: &'a mut MachineState, dsw1: u16) -> Bus<'a> {
        Bus {
            rom: &self.rom,
            work: &mut self.work,
            obj: &mut state.objectram,
            video: &mut state.videoram,
            fg_scroll: &mut state.fg_scroll,
            bg_scroll: &mut state.bg_scroll,
            gfxctrl: &mut state.gfxctrl,
            flip_screen: &mut state.flip_screen,
            coin_ctrl: &mut state.coin_ctrl,
            palette: &mut state.paletteram,
            dsw1,
            unrecovered: &mut self.unrecovered,
            effect_stream_calls: &mut self.effect_stream_calls,
            toggles: self.match_config.toggles,
            preset: self.match_config.preset,
            roster: self.match_config.roster,
            dramatic: self.match_config.dramatic,
            world_tour: self.match_config.world_tour,
            evil_ryu_playtest: self.match_config.evil_ryu_playtest,
            era_rules: self.match_config.into(),
            toggle_state: &mut self.toggle_state,
            training: &mut self.training,
            backgrounds: self.backgrounds.as_deref(),
            data_bank: self.data_bank.as_deref(),
            bg_page: &mut state.bg_page,
            fg_page: &mut state.fg_page,
        }
    }
}

/// Original memory as the translated routines see it.
pub struct Bus<'a> {
    /// 0x000000-0x05ffff (read-only data).
    pub rom: &'a [u8],
    /// 0xff8000-0xffdfff.
    pub work: &'a mut [u8],
    /// 0xffe000-0xffffff.
    pub obj: &'a mut [u8],
    /// 0x800000-0x800fff.
    pub video: &'a mut [u8],
    /// The fg_scroll hardware register (0xc00014, write-only).
    pub fg_scroll: &'a mut u16,
    /// The bg_scroll hardware register (0xc00018, write-only).
    pub bg_scroll: &'a mut u16,
    /// The low byte of the write-only register 0xc0001a (layer enables;
    /// its work-RAM copy is 0xff8682).
    pub gfxctrl: &'a mut u8,
    /// `gfxctrl` bit 2 (`sf_state::gfxctrl_w` -> `flip_screen_set(data & 0x04)`).
    pub flip_screen: &'a mut bool,
    /// The write-only coin-control register 0xc00010 (low byte goes to
    /// MAME's `coin_w` at 0xc00011; its work-RAM copy is 0xff8684).
    pub coin_ctrl: &'a mut u16,
    /// 0xb00000-0xb007ff (the palette, written by the vblank's palette
    /// load).
    pub palette: &'a mut [u8],
    /// Live DSW1 port word (0xc00008), read by the sound-queue filters at
    /// 0x3b7cc / 0x3b7f2.
    pub dsw1: u16,
    /// Original addresses of untranslated branches reached.
    pub unrecovered: &'a mut BTreeSet<u32>,
    /// Target addresses of effect `0xff` stream calls (`0x42dd2`) reached.
    pub effect_stream_calls: &'a mut BTreeSet<u32>,
    /// Active Tier 1 core overhaul toggles (`#94`, `O3`).
    pub toggles: crate::config::CoreToggles,
    /// The session's ruleset preset. Gates that must never act under
    /// Faithful 1987 whatever else is switched on (the mirror palettes)
    /// read it.
    pub preset: crate::config::RulesetPreset,
    /// 12-fighter roster binding (`#98`, `O7`). Ignored while `full_cast` is false.
    pub roster: crate::roster::RosterSettings,
    pub dramatic: Option<crate::dramatic::DramaticRules>,
    pub(crate) world_tour: Option<crate::config::WorldTourRules>,
    pub(crate) evil_ryu_playtest: Option<EvilRyuPlaytest>,
    pub(crate) era_rules: crate::era_fight::EraRules,
    /// Per-session runtime state for Tier 1 core overhaul toggles (`#94`, `O3`).
    pub toggle_state: &'a mut crate::config::CoreToggleState,
    /// What Training's rules keep between frames (ADR-0013, `#243`).
    pub training: &'a mut crate::training::TrainingState,
    /// The extended background bank, when this machine was given one
    /// (an overhaul fight's; ADR-0009).
    pub backgrounds: Option<&'a sf_video::background_bank::BackgroundBank>,
    /// The extended data bank, when this machine was given one (an overhaul
    /// fight's; ADR-0014). Without it its addresses read `0xff`, as they
    /// always have.
    pub data_bank: Option<&'a crate::data_bank::DataBank>,
    /// The map pages the renderer reads for the bg and fg layers
    /// (`MachineState::bg_page` / `fg_page`).
    pub bg_page: &'a mut u8,
    pub fg_page: &'a mut u8,
}

impl Bus<'_> {
    pub(crate) fn dramatic_rules(&self) -> Option<crate::dramatic::DramaticRules> {
        self.dramatic.filter(|_| {
            self.preset != crate::config::RulesetPreset::Faithful1987
                && self.rb(GAME_MODE) == 2 && self.rb(MODE_DEMO) != 0
        })
    }

    fn dramatic_alive(&self, ps: u32) -> bool {
        DRAMATIC_PLAYERS
            .iter()
            .position(|&player| player == ps)
            .is_some_and(|slot| self.toggle_state.dramatic.health[slot] != 0)
    }

    fn dramatic_hit_targets(&self, owner: u32, friendly_fire: bool) -> [u32; 2] {
        match owner {
            CPU_STRUCT => [P1_STRUCT, P2_STRUCT],
            P1_STRUCT => [CPU_STRUCT, if friendly_fire { P2_STRUCT } else { 0 }],
            P2_STRUCT => [CPU_STRUCT, if friendly_fire { P1_STRUCT } else { 0 }],
            _ => [0; 2],
        }
    }

    fn bind_dramatic_target(&mut self, owner: u32, defender: u32) {
        let entity = self.rw(owner + 0x10);
        self.ww(owner + 0xe, self.rw(defender + 0x10));
        self.set_el(entity, 0x3c, defender);
    }

    fn bind_dramatic_boss_target(&mut self) {
        let boss = self.rw(CPU_STRUCT + 0x10);
        let partner = self.toggle_state.throw.partners[1];
        if partner != 0
            && matches!(self.eb(boss, 0xa), crate::combo::throw::THROW_MOVE | crate::combo::throw::THROWN_MOVE)
        {
            self.bind_dramatic_target(CPU_STRUCT, partner);
            return;
        }
        let boss_x = self.center_x(boss) as i16;
        let target = [P1_STRUCT, P2_STRUCT]
            .into_iter()
            .filter(|&ps| self.dramatic_alive(ps))
            .min_by_key(|&ps| boss_x.abs_diff(self.center_x(self.rw(ps + 0x10)) as i16))
            .unwrap_or(P1_STRUCT);
        self.bind_dramatic_target(CPU_STRUCT, target);
    }

    // ---- memory -------------------------------------------------------

    pub fn rb(&self, a: u32) -> u8 {
        let a = a & 0xff_ffff;
        match a {
            0..=0x05_ffff => self.rom.get(a as usize).copied().unwrap_or(0xff),
            // The extended data bank (ADR-0014): `0xff` without one.
            crate::data_bank::DATA_BANK_BASE..=crate::data_bank::DATA_BANK_END => {
                self.data_bank.map_or(0xff, |b| b.rb(a))
            }
            0x80_0000..=0x80_0fff => self.video[(a - 0x80_0000) as usize],
            BOSS_LATCHES..=0x60000f if self.dramatic_rules().is_some() => {
                self.toggle_state.dramatic.boss_latches[(a - BOSS_LATCHES) as usize]
            }
            BOSS_TRACKERS..=0x600018 if self.dramatic_rules().is_some() => {
                self.toggle_state.dramatic.boss_trackers[(a - BOSS_TRACKERS) as usize]
            }
            0xff_8000..=0xff_dfff => self.work[(a - WORK_BASE) as usize],
            0xff_e000..=0xff_ffff => self.obj[(a - OBJ_BASE) as usize],
            _ => 0xff,
        }
    }

    pub fn wb(&mut self, a: u32, v: u8) {
        let a = a & 0xff_ffff;
        match a {
            0x80_0000..=0x80_0fff => self.video[(a - 0x80_0000) as usize] = v,
            BOSS_LATCHES..=0x60000f if self.dramatic_rules().is_some() => {
                self.toggle_state.dramatic.boss_latches[(a - BOSS_LATCHES) as usize] = v;
            }
            BOSS_TRACKERS..=0x600018 if self.dramatic_rules().is_some() => {
                self.toggle_state.dramatic.boss_trackers[(a - BOSS_TRACKERS) as usize] = v;
            }
            0xff_8000..=0xff_dfff => self.work[(a - WORK_BASE) as usize] = v,
            0xff_e000..=0xff_ffff => self.obj[(a - OBJ_BASE) as usize] = v,
            _ => {} // ROM and unmapped space ignore writes
        }
    }

    pub fn rw(&self, a: u32) -> u16 {
        u16::from_be_bytes([self.rb(a), self.rb(a + 1)])
    }

    pub fn ww(&mut self, a: u32, v: u16) {
        let [h, l] = v.to_be_bytes();
        self.wb(a, h);
        self.wb(a + 1, l);
    }

    pub fn rl(&self, a: u32) -> u32 {
        (self.rw(a) as u32) << 16 | self.rw(a + 2) as u32
    }

    pub fn wl(&mut self, a: u32, v: u32) {
        self.ww(a, (v >> 16) as u16);
        self.ww(a + 2, v as u16);
    }

    pub fn eb(&self, e: u16, off: u32) -> u8 {
        self.rb(OBJ_BASE + e as u32 + off)
    }
    pub fn set_eb(&mut self, e: u16, off: u32, v: u8) {
        self.wb(OBJ_BASE + e as u32 + off, v)
    }
    pub fn ew(&self, e: u16, off: u32) -> u16 {
        self.rw(OBJ_BASE + e as u32 + off)
    }
    pub fn set_ew(&mut self, e: u16, off: u32, v: u16) {
        self.ww(OBJ_BASE + e as u32 + off, v)
    }
    pub fn el(&self, e: u16, off: u32) -> u32 {
        self.rl(OBJ_BASE + e as u32 + off)
    }
    pub fn set_el(&mut self, e: u16, off: u32, v: u32) {
        self.wl(OBJ_BASE + e as u32 + off, v)
    }

    fn note_unrecovered(&mut self, addr: u32) {
        self.unrecovered.insert(addr);
    }

    /// Record an untranslated original branch reached (for the task
    /// translations outside this module).
    pub(crate) fn note(&mut self, addr: u32) {
        self.note_unrecovered(addr);
    }

    /// Self/other entity offsets of a player structure (+0xe: other in the
    /// high word, self in the low word).
    pub fn player_entities(&self, ps: u32) -> (u16, u16) {
        let d1 = self.rl(ps + 0xe);
        (d1 as u16, (d1 >> 16) as u16)
    }

    // ---- geometry -----------------------------------------------------

    /// Big sprites (attr >= 0x400, signed word compare) are 32px cells.
    fn is_big(&self, e: u16) -> bool {
        (self.ew(e, 2) as i16) >= 0x400
    }

    /// Half the grid extent for `n + 1` cells (0x43be8 / 0x43b6a).
    fn half_extent(n: u8, big: bool) -> u16 {
        let cell = if big { 0x20 } else { 0x10 };
        ((n as u16).wrapping_mul(cell).wrapping_add(cell)) >> 1
    }

    /// 0x43be8: horizontal centre = sx + half the grid width.
    pub fn center_x(&self, e: u16) -> u16 {
        self.ew(e, 6)
            .wrapping_add(Self::half_extent(self.eb(e, 0xd), self.is_big(e)))
    }

    /// 0x43bda: vertical centre = sy + half the grid height.
    pub fn center_y(&self, e: u16) -> u16 {
        self.ew(e, 4)
            .wrapping_add(Self::half_extent(self.eb(e, 0xc), self.is_big(e)))
    }

    /// Visit every grid entry of entity `e` ((rows) x (cols) entries at stride 0x40
    /// from `e`, wrapping on a garbage grid), as the original's nested dbra loops do.
    fn grid_entries(&self, e: u16) -> Vec<u16> {
        let cells = (self.eb(e, 0xc) as u16 + 1).wrapping_mul(self.eb(e, 0xd) as u16 + 1);
        let entry = |k: u16| e.wrapping_add(k.wrapping_mul(0x40));
        (0..cells).map(entry).collect()
    }

    /// Add `d` to the x (+6) of every grid entry (0x43d64, 0x43e6a, 0x43b56).
    fn shift_grid_x(&mut self, e: u16, d: u16) {
        for s in self.grid_entries(e) {
            let x = self.ew(s, 6).wrapping_add(d);
            self.set_ew(s, 6, x);
        }
    }

    fn teleport_to(&mut self, fighter: u16, target_x: u16) {
        self.shift_grid_x(fighter, target_x.wrapping_sub(self.center_x(fighter)));
    }

    /// Move a fighter's whole grid horizontally (the per-entry loops of the
    /// separation, clamp, and camera routines).
    pub fn shift_entity_x(&mut self, e: u16, d: u16) {
        self.shift_grid_x(e, d);
    }

    /// Add `d` to the y (+4) of every grid entry (0x43c82).
    fn shift_grid_y(&mut self, e: u16, d: u16) {
        for s in self.grid_entries(e) {
            let y = self.ew(s, 4).wrapping_add(d);
            self.set_ew(s, 4, y);
        }
    }

    /// 0x43b6a: position a freshly loaded frame around the entity's centre
    /// (+6/+4 hold the centre on entry). Returns the first entry's (x, y):
    /// x = sx + x-offset (+0x26), y = sy + dy (+0x14); +6/+4 keep sx/sy.
    fn place_frame(&mut self, e: u16) -> (u16, u16) {
        let big = self.is_big(e);
        let hx = Self::half_extent(self.eb(e, 0xd), big);
        let hy = Self::half_extent(self.eb(e, 0xc), big);
        let dx = self.world_tour_horizontal_step(self.el(e, 0x38), self.eb(e, 0xa), sext8(self.eb(e, 0x15)) as i16);
        let sx = self.ew(e, 6).wrapping_sub(hx).wrapping_add(dx as u16);
        self.set_ew(e, 6, sx);
        let x = sext8(self.eb(e, 0x26)).wrapping_add(sx);
        let sy = self.ew(e, 4).wrapping_sub(hy);
        self.set_ew(e, 4, sy);
        let y = sext8(self.eb(e, 0x14)).wrapping_add(sy);
        (x, y)
    }

    /// 0x44506 / 0x442ca: zero the position of every remaining entry of the
    /// fighter's 24-entry region (codes/attr keep their last values).
    fn park_from(&mut self, e: u16, mut d0: u16) {
        while d0 != 0x600 {
            if d0 > 0x600 {
                self.note_unrecovered(0x44506);
                return;
            }
            self.set_el(e.wrapping_add(d0), 4, 0);
            d0 += 0x40;
        }
    }

    /// 0x443c6..0x44446: write the frame grid unflipped. Big: code +2 per
    /// column, +0x20 per row, 32px steps; small: +1 / +0x10, 16px steps.
    fn write_grid(&mut self, e: u16, code: u16, attr: u16, x: u16, y: u16) {
        let (rows, cols) = (self.eb(e, 0xc), self.eb(e, 0xd));
        self.write_entries(e, &pose_grid_entries(code, attr, rows, cols, x, y, false));
    }

    /// 0x4445e..0x44502: write the frame grid x-flipped (facing left): attr
    /// gains 0x100 and each row's codes run in reverse column order.
    fn write_grid_flipped(&mut self, e: u16, code: u16, attr: u16, x: u16, y: u16) {
        let (rows, cols) = (self.eb(e, 0xc), self.eb(e, 0xd));
        self.write_entries(e, &pose_grid_entries(code, attr, rows, cols, x, y, true));
    }

    /// Write a grid's entries from entity `e` at stride 0x40, then park the
    /// rest of the 24-entry region.
    fn write_entries(&mut self, e: u16, entries: &[[u16; 4]]) {
        let mut d0: u16 = 0;
        for &[c, attr, ry, cx] in entries {
            let s = e.wrapping_add(d0);
            self.set_ew(s, 0, c);
            self.set_ew(s, 2, attr);
            self.set_ew(s, 4, ry);
            self.set_ew(s, 6, cx);
            d0 = d0.wrapping_add(0x40);
        }
        self.park_from(e, d0);
    }

    /// 0x43fa8: turn to face left — negate the facing-relative frame bytes
    /// and mirror the grid's codes in place (attr gains 0x100).
    pub fn face_left(&mut self, e: u16) {
        for off in [0x15, 0x26, 0x1c, 0x20, 0x24] {
            let v = self.eb(e, off).wrapping_neg();
            self.set_eb(e, off, v);
        }
        self.mirror_codes(e, true);
    }

    /// 0x4405a: turn to face right — the inverse of [`Self::face_left`].
    pub fn face_right(&mut self, e: u16) {
        for off in [0x15, 0x26, 0x1c, 0x20, 0x24] {
            let v = self.eb(e, off).wrapping_neg();
            self.set_eb(e, off, v);
        }
        self.mirror_codes(e, false);
    }

    /// The shared code-mirroring loops of 0x43fa8 / 0x4405a.
    fn mirror_codes(&mut self, e: u16, to_left: bool) {
        let rows = self.eb(e, 0xc) as u16;
        let cols = self.eb(e, 0xd) as u16;
        let big = self.is_big(e);
        let (span, rev) = if big { (cols << 1, 4u16) } else { (cols, 2u16) };
        let mut d0 = e;
        for _ in 0..=rows {
            let mut d4: u16 = 0;
            for _ in 0..=cols {
                let code = self.ew(d0, 0);
                let attr = self.ew(d0, 2);
                if to_left {
                    self.set_ew(d0, 0, code.wrapping_add(span).wrapping_sub(d4));
                    self.set_ew(d0, 2, attr | 0x100);
                } else {
                    self.set_ew(d0, 0, code.wrapping_add(d4).wrapping_sub(span));
                    self.set_ew(d0, 2, attr & !0x100);
                }
                d4 = d4.wrapping_add(rev);
                d0 = d0.wrapping_add(0x40);
            }
        }
    }

    // ---- moves and animation -------------------------------------------

    fn is_native_super(&self, ps: u32, move_id: u8) -> bool {
        (move_id == crate::era_fight::SF2_SUPER && self.sf2_state(ps).is_some())
            || self.era_state(ps).is_some_and(|era| {
                era.alpha_super_level(move_id) != 0
                    || (move_id == crate::data_bank::ALPHA_COUNTER_SLOT && era.custom_combo_active())
            })
    }

    /// 0x441b0: start move `mv` from the player's move table (+0x12): copy
    /// the 0x30-byte move header over entity +0..+0x2f, keep the centre
    /// (minus the old x-offset), and write the first pose grid. Native
    /// Supers may show that grid before the runner processes its first frame.
    pub fn start_move(&mut self, e: u16, ps: u32, mv: u8) {
        let d3 = self.center_x(e).wrapping_sub(sext8(self.eb(e, 0x26)));
        self.start_move_at(e, ps, mv, d3);
    }

    /// 0x441c4: the body of 0x441b0 with the target centre given (D3): the
    /// placement entries 0x44114.. enter here with 0 (keep the header's
    /// position) or an x such as 0x1c0.
    pub fn start_move_at(&mut self, e: u16, ps: u32, mv: u8, mut d3: u16) {
        self.world_tour_guard_reaction(ps, false);
        self.world_tour_shuriken_recovery(ps, false);
        if self.evil_ryu_profile(ps).is_some() {
            self.toggle_state.raging_demon.victory_pose = false;
        }
        self.era_clear_alpha_move(ps, mv);
        let table = self.rl(ps + 0x12);
        let family_move = self.era_state(ps)
            .filter(|state| {
                state.alpha_move_active(mv) && (state.uses_family_bank()
                    || (mv == crate::data_bank::ALPHA_COUNTER_SLOT && state.custom_combo_active()))
            });
        let src = if let Some(state) = family_move {
            self.family_move_header(state, e, mv)
        } else if mv == crate::data_bank::ALPHA_COUNTER_SLOT {
            self.alpha_counter_header(ps)
        } else if mv == 1 && self.era_state(ps).is_some_and(|era| {
            (era.is_alpha() || era.is_sf4() || era.what_if_active()) && era.gen_stance() == crate::alpha::Stance::Crane
        }) {
            self.data_bank.and_then(|bank| bank.move_header(crate::roster::TournamentFighter::Gen, 0x28))
        } else if let Some(fighter) = self.alpha_state(ps).filter(|era| era.alpha_move_active(mv))
            .and_then(|era| era.fighter)
        {
            self.data_bank.and_then(|bank| bank.alpha_move_header(fighter, mv))
        } else {
            let era_slot = matches!(mv, crate::era_fight::SF2_EXTRA_SPECIAL | crate::era_fight::SF2_SUPER);
            let fighter = self.sf2_state(ps).filter(|_| era_slot)
                .and_then(|era| era.fighter);
            match fighter {
                Some(fighter) => self.data_bank.and_then(|bank| bank.move_header(fighter, mv)),
                None => crate::data_bank::move_header(|addr| self.rl(addr), table, mv),
            }
        };
        let Some(src) = src else {
            return;
        };
        self.control_move_started(ps, mv);
        for i in 0..0x30u32 {
            let b = self.rb(src + i);
            self.set_eb(e, i, b);
        }
        self.toggle_state.dp_connected[self.player_slot_index(ps).unwrap_or(1)] = false;
        if let Some(slot) = self.player_slot_index(ps) {
            self.toggle_state.human_attack_frames[slot] = 0;
        }
        if self.era_true_combos(ps) || self.era_state(ps).is_some_and(|era| era.alpha_move_active(mv)) {
            self.combo_new_move(e, mv);
        }
        if self.toggles.cancels {
            // Cancels (SP18): a new move closes the cancel window.
            self.cancel_close(e);
        }
        self.clear_air_attack_trajectory(ps);
        if self.toggles.interruptible_walk_cycle
            && mv == 3
            && self.is_player_driven(ps)
            && matches!(table, 0x1605c | 0x1662c)
        {
            // (Not a boss's bank table: Retsu's 1987 walk back moves only
            // by its header's step, so dropping it would stop him walking.)
            // Suppress the 16-pixel first-frame backward shift in Ryu/Ken's Walk Back
            // header (+6 = 0xfff0, +0x26 = 0) so starting a normal attack while
            // holding Back does not step 16px out of range on frame 1 (#95, O4).
            self.set_ew(e, 6, 0);
        }
        if self.toggles.standard_high_low_block && matches!(mv, 4 | 8) {
            self.apply_standard_block_hurtbox(e, table);
        }
        if self.toggles.balanced_roster && table == 0x1662c {
            if mv == 0x15 {
                // Ken Standing MK initial frame duration 12 -> 13 to match Ryu (#96, O5).
                self.set_eb(e, 0xe, 13);
            } else if mv == 0x19 {
                // Ken Shoryuken initial startup invulnerability bit 0x08 to match Ryu (#96, O5).
                self.set_eb(e, 0xf, self.eb(e, 0xf) | 0x08);
            }
        }
        if self.toggles.balanced_roster {
            self.apply_balanced_move_start(e, table, mv);
        }
        self.era_move_started(e, ps, mv);
        if self.toggles.anytime_jump_attacks
            && self.toggles.fast_prejump_crouch
            && self.is_player_driven(ps)
            && Self::is_player_table(table)
            && matches!(mv, 9..=0x0b)
        {
            // Shorten pre-jump crouch startup from 9 frames (0..=8) to 4 frames (0..=3) (#95, O4).
            self.set_eb(e, 0xe, 3);
        }
        if d3 != 0 {
            d3 = d3.wrapping_sub(Self::half_extent(self.eb(e, 0xd), self.is_big(e)));
            if (self.eb(e, 0x37) as i8) >= 4 {
                let x = self.ew(e, 6).wrapping_neg();
                self.set_ew(e, 6, x);
            }
            // 0x4420c-0x44228 also negate the new x-offset into a scratch
            // register that nothing reads.
            let dx = self.world_tour_horizontal_step(ps, mv, self.ew(e, 6) as i16);
            let x = d3.wrapping_add(dx as u16);
            self.set_ew(e, 6, x);
        }
        // 0x4422e: the unflipped grid from the header's code/attr/position.
        let (sy, sx, code, attr) = (self.ew(e, 4), self.ew(e, 6), self.ew(e, 0), self.ew(e, 2));
        self.write_grid(e, code, attr, sx, sy);
        if self.eb(e, 0x37) & 4 != 0 && self.is_native_super(ps, mv) {
            // Super freeze presents this pose before the object runner.
            self.face_left(e);
        }
    }

    /// 0x44174 (P1, `ps = 0xff86b0`) / 0x44192 (P2, `ps = 0xff86da`, mode 4
    /// unexercised; ticket #124, F52): face right (`+0x37 = 2`) and start the
    /// stance move (`mv = 1`) at `x = 0x100` (`0x441c4`).
    pub fn place_bonus_player(&mut self, ps: u32) {
        let me = self.rw(ps + 0x10);
        self.set_eb(me, 0x37, 2);
        self.start_move_at(me, ps, 1, 0x100);
    }

    /// A bank player's crouch can have a different centre from his 1987
    /// CPU crouch. Enter the original grounded reaction at its own ground
    /// origin once, rather than carrying that crouch-centre difference
    /// through every reaction and its restored standing pose.
    fn bank_crouching_reaction_center(&self, e: u16, frame: u32) -> Option<u16> {
        let ps = self.el(e, 0x38);
        let mv = self.eb(e, 0xa);
        if !self.on_bank_player_table(ps)
            || self.eb(e, 9) & 7 != 4
            || !(0x30..=0x3f).contains(&mv)
        {
            return None;
        }
        let (alive_table, _) = self.bound_boss_tables(ps)?;
        let entry = alive_table + u32::from(mv - 0x30) * 4;
        if frame != u32::from(self.rw(entry)) {
            return None;
        }
        let half_height = Self::half_extent(self.rb(frame + 8), self.rw(frame + 4) >= 0x400);
        Some((crate::data_bank::GROUND as u16)
            .wrapping_sub(half_height)
            .wrapping_sub(sext8(self.rb(frame + 6))))
    }

    /// Some catalogue throw pictures already turn away from the original
    /// opponent. Their drawing and anchors mirror opposite the logical
    /// heading; the grab, ordinary throws and ROM fallback do not.
    fn bank_throw_pose_faces_back(&self, e: u16, frame: u32) -> bool {
        if !self.toggles.throws || self.eb(e, 0xa) != crate::combo::throw::THROW_MOVE {
            return false;
        }
        let Some(entry) = self.fighter_of_entity(e)
            .and_then(|fighter| self.data_bank?.throw_entry(fighter))
        else {
            return false;
        };
        let stream = self.rl(entry.throw_header + 0x2c);
        let Some(offset) = frame.checked_sub(stream) else { return false; };
        offset % 0x20 == 0 && entry.back_turned_poses.checked_shr(offset / 0x20)
            .is_some_and(|mask| mask & 1 != 0)
    }

    /// 0x442e2: load the stream frame at `a1` (0x20 bytes: hold, duration,
    /// code, attr, dy, dx, rows-1, cols-1, boxes +0x16..+0x25, flags, x-offset,
    /// hit function) and redraw the grid around the kept centre.
    fn load_frame(&mut self, e: u16, a1: u32) {
        let mut cx = self.center_x(e);
        let x26 = self.eb(e, 0x26);
        if x26 != 0 {
            cx = cx.wrapping_sub(sext8(x26));
        }
        self.set_ew(e, 6, cx);
        let cy = self.bank_crouching_reaction_center(e, a1).unwrap_or_else(|| self.center_y(e));
        self.set_ew(e, 4, cy);
        self.set_ew(e, 2, self.rw(a1 + 4));
        self.set_eb(e, 0xb, self.rb(a1));
        self.set_eb(e, 0xe, self.rb(a1 + 1));
        self.set_eb(e, 0x33, self.rb(a1 + 1));
        self.set_eb(e, 0x14, self.rb(a1 + 6));
        self.set_eb(e, 0x15, self.rb(a1 + 7));
        self.set_eb(e, 0xc, self.rb(a1 + 8));
        self.set_eb(e, 0xd, self.rb(a1 + 9));
        for i in 0..0x10u32 {
            self.set_eb(e, 0x16 + i, self.rb(a1 + 0xa + i));
        }
        self.set_eb(e, 0xf, self.rb(a1 + 0x1a));
        self.set_eb(e, 0x26, self.rb(a1 + 0x1b));
        if self.toggles.throws
            && self.eb(e, 0xa) == crate::combo::throw::THROWN_MOVE
            && self.toggle_state.throw.crosses[self.combo_slot(e)]
        {
            // The victim's generic airborne poses are shared by all throws.
            // A crossing throw carries him through the shoulder to the new
            // side; its direction is mirrored with the pose below.
            self.set_eb(e, 0x15, (-crate::combo::throw::CROSS_ARC_SPEED) as u8);
        }
        self.set_el(e, 0x28, self.rl(a1 + 0x1c));
        if self.toggles.standard_high_low_block {
            self.apply_standard_reaction_hurtbox(e, a1);
        }
        if self.toggles.balanced_roster {
            self.apply_shoto_frame_symmetry(e, a1);
            self.apply_balanced_frame(e, a1);
        }
        if a1 == 0x01340 && self.evil_ryu_profile(self.el(e, 0x38)) == Some(EvilRyuPlaytest::UltraSf2)
        {
            // The final grounded Shoryuken pose is stationary and has no repeats.
            // Keep Evil Ryu committed to that recovery pose for four more frames.
            let hold = self.eb(e, 0xe) + 4;
            self.set_eb(e, 0xe, hold);
            self.set_eb(e, 0x33, hold);
        }
        let move_id = self.eb(e, 0xa);
        let alpha3_counter = move_id == crate::data_bank::ALPHA_COUNTER_SLOT
            && self.evil_ryu_alpha3(self.el(e, 0x38));
        if self.toggles.balanced_roster && (matches!(move_id, 0x19 | 0x1b) || alpha3_counter) {
            let slot = self.player_slot_index(self.el(e, 0x38)).unwrap_or(1);
            if self.toggle_state.dp_connected[slot] {
                self.set_eb(e, 0xf, self.eb(e, 0xf) & 0x7f);
            }
        }
        let alpha_move = self.era_state(self.el(e, 0x38))
            .is_some_and(|era| era.alpha_move_active(self.eb(e, 0xa)));
        if self.era_true_combos(self.el(e, 0x38))
            || alpha_move || self.combo_reacting(e) || self.guard_crush_reaction(e)
        {
            self.combo_load_frame(e, a1);
        }
        self.era_move_recovery(e, a1);
        let code = self.rw(a1 + 2);
        self.set_el(e, 0x2c, a1 + 0x20);
        let mirror = (self.eb(e, 0x37) & 4 != 0) ^ self.bank_throw_pose_faces_back(e, a1);
        if mirror {
            // 0x4444a: facing-relative bytes flip sign before placement.
            for off in [0x15, 0x26, 0x24, 0x1c, 0x20] {
                let v = self.eb(e, off).wrapping_neg();
                self.set_eb(e, off, v);
            }
            let (x, y) = self.place_frame(e);
            let attr = self.ew(e, 2);
            self.write_grid_flipped(e, code, attr, x, y);
        } else {
            let (x, y) = self.place_frame(e);
            let attr = self.ew(e, 2);
            self.write_grid(e, code, attr, x, y);
        }
    }

    /// 0x42ad0: one frame of a fighter's object runner. `ps` is the
    /// fighter's player structure (A3 = entity +0x38 in the original).
    pub fn run_fighter(&mut self, me: u16, other: u16) {
        if let Some(slot) = self.player_slot_index(self.el(me, 0x38)) {
            if self.eb(me, 9) & 0x40 != 0 {
                self.toggle_state.human_attack_frames[slot] =
                    self.toggle_state.human_attack_frames[slot].saturating_add(1);
            } else {
                self.toggle_state.human_attack_frames[slot] = 0;
            }
        }
        let ps = self.el(me, 0x38);
        if self.evil_ryu_profile(ps).is_some() {
            let has_demon = self.evil_ryu_has_demon(ps);
            if has_demon && self.toggle_state.raging_demon.victory_pose {
                self.run_demon_victory(me, other);
                return;
            }
            let mv = self.eb(me, 0xa);
            if self.legacy_evil_ryu(ps) && self.is_ashura_move(ps, mv) {
                self.run_ashura(me, other);
                return;
            }
            if has_demon && mv == self.demon_rush_move(ps) {
                self.run_raging_demon(me, other);
                return;
            }
            if has_demon && mv == crate::combo::throw::THROW_MOVE && self.toggle_state.raging_demon.active
                && self.eb(me, 0x30) == 4
            {
                self.toggle_state.raging_demon.blackout_frames = Some(DEMON_BLACKOUT_FRAMES);
                self.spawn_demon_spark(other);
            }
        }
        if self.eb(me, 8) == 1 {
            // First frame of a move started by the input handler.
            self.set_eb(me, 0x33, self.eb(me, 0xe));
            self.set_eb(me, 8, 2);
            let super_grid_mirrored = self.ew(me, 2) & 0x100 != 0
                && self.is_native_super(self.el(me, 0x38), self.eb(me, 0xa));
            if self.eb(me, 0x37) & 4 != 0 && !super_grid_mirrored {
                self.face_left(me);
            }
            let snd = self.eb(me, 0x10);
            if snd != 0 {
                self.queue_sound(snd as u16);
            }
            self.push_apart(me, other);
            self.clamp_y(me);
            self.clamp_x(me);
            let (me, other) = self.effect_flag_spawn(me, other);
            self.judge_targets(me, other);
            return;
        }
        if self.eb(me, 9) == 0x81 {
            self.buffer_air_strength(me);
        }
        self.animate(me, other);
    }

    fn is_ashura_move(&self, ps: u32, mv: u8) -> bool {
        mv == EVIL_RYU_ASHURA_FORWARD || mv == self.ashura_back_move(ps)
    }

    fn ashura_back_move(&self, ps: u32) -> u8 {
        if self.evil_ryu_sf4(ps) { EVIL_RYU_SF4_ASHURA_BACK } else { EVIL_RYU_ASHURA_BACK }
    }

    fn hold_evil_ryu_stance(&mut self, me: u16, mv: u8) {
        self.start_move(me, self.el(me, 0x38), 1);
        if self.eb(me, 0x37) & 4 != 0 {
            self.face_left(me);
        }
        self.set_eb(me, 8, 2);
        self.set_eb(me, 9, 0x82);
        self.set_eb(me, 0xa, mv);
    }

    fn load_evil_ryu_glide_pose(&mut self, me: u16) {
        let x = self.center_x(me);
        self.load_frame(me, 0x00e20); // Ryu's raised-knee kick chamber.
        self.teleport_to(me, x);
        self.set_eb(me, 0x26, 0);
    }

    fn run_ashura(&mut self, me: u16, other: u16) {
        let ps = self.el(me, 0x38);
        let remaining = self.eb(me, 0x33);
        let mv = self.eb(me, 0xa);
        let variable_range = self.evil_ryu_alpha3(ps) || self.evil_ryu_sf4(ps);
        let long_glide = variable_range && self.eb(me, 0x34) != 0;
        let travel_frames = if long_glide { 10 } else { 6 };
        let glide_start = 14 + travel_frames;
        match remaining {
            tick if tick == glide_start => {
                self.load_evil_ryu_glide_pose(me);
                self.set_eb(me, 0xf, 0x08);
            }
            14 => {
                if variable_range {
                    self.keep_facing(me, other);
                    self.set_eb(me, 0x34, 0);
                }
                self.hold_evil_ryu_stance(me, mv);
            }
            0 => {
                self.set_eb(me, 9, 0x12);
                self.set_eb(me, 0xa, 1);
                return;
            }
            _ => {}
        }
        if (15..=glide_start).contains(&remaining) {
            let tick = i16::from(glide_start - remaining);
            let forward = mv == EVIL_RYU_ASHURA_FORWARD;
            let range = match (forward, long_glide) {
                (true, true) => 148,
                (false, true) => 112,
                (true, false) => 88,
                (false, false) => 64,
            };
            let frames = i16::from(travel_frames);
            let step = (tick + 1) * range / frames - tick * range / frames;
            let x = self.center_x(me) as i16;
            let facing = if self.eb(me, 0x37) & 4 == 0 { 1 } else { -1 };
            let distance = if !forward {
                -step
            } else if variable_range {
                step
            } else {
                let gap = (self.center_x(other) as i16 - x) * facing;
                (gap - 48).clamp(0, step)
            };
            self.teleport_to(me, (x + facing * distance) as u16);
        }
        self.set_eb(me, 0x33, remaining - 1);
        self.push_apart(me, other);
        self.clamp_x(me);
    }

    fn run_raging_demon(&mut self, me: u16, other: u16) {
        let ps = self.el(me, 0x38);
        let remaining = self.eb(me, 0x33);
        if remaining == 0 {
            self.set_eb(me, 9, 0x12);
            self.set_eb(me, 0xa, 1);
            return;
        }
        if remaining > DEMON_WHIFF_FRAMES {
            // The glide and modern Sagat's standing pushboxes separate to 52px.
            let range = if matches!(self.demon_rush_move(ps), KAGE_RAGING_DEMON | SF6_RAGING_DEMON) {
                MODERN_DEMON_GRAB_RANGE
            } else {
                crate::combo::throw::RANGE
            };
            let facing = if self.eb(me, 0x37) & 4 == 0 { 1 } else { -1 };
            let gap = (self.center_x(other) as i16 - self.center_x(me) as i16) * facing;
            let advance = (gap - range as i16).clamp(0, 6);
            self.shift_grid_x(me, (advance * facing) as u16);
            self.push_apart(me, other);
            self.clamp_x(me);
            // Even at point blank, show the approach before attempting a grab.
            let travelled = DEMON_RUSH_FRAMES + DEMON_WHIFF_FRAMES - remaining;
            if travelled >= 2 && gap >= 0 && self.can_throw_at_range(me, other, range) {
                if let Some((thrower, victim)) = self.throw_entries(me, other) {
                    self.start_throw(ps, me, other, thrower.throw_header, victim.thrown_header, false);
                    let victim_slot = self.combo_slot(other);
                    self.toggle_state.throw.crosses[victim_slot] = false;
                    if self.demon_rush_move(ps) == SF6_RAGING_DEMON {
                        self.toggle_state.throw.tech_window[victim_slot] = 0;
                    }
                    self.set_eb(me, 0x30, 0);
                    self.toggle_state.raging_demon.active = true;
                    return;
                }
            }
        } else if remaining == DEMON_WHIFF_FRAMES {
            self.hold_evil_ryu_stance(me, self.demon_rush_move(ps));
        }
        self.set_eb(me, 0x33, remaining - 1);
    }

    fn reveal_raging_demon(&mut self, thrower: u16, victim: u16) {
        // The native victory header supplies the ground origin for the tall pose.
        self.start_move(thrower, self.el(thrower, 0x38), 0x0c);
        self.load_frame(thrower, RYU_VICTORY_POSE);
        self.set_eb(thrower, 8, 2);
        self.set_eb(thrower, 9, 0x82);
        self.set_eb(thrower, 0xf, 0x08);
        // Land the victim beside Ryu on the same side as the grab.
        let offset = if self.eb(thrower, 0x37) & 4 == 0 {
            DEMON_REVEAL_GAP
        } else {
            DEMON_REVEAL_GAP.wrapping_neg()
        };
        self.teleport_to(victim, self.center_x(thrower).wrapping_add(offset));
        self.clamp_x(victim);
        self.teleport_to(thrower, self.center_x(victim).wrapping_sub(offset));
        self.clamp_x(thrower);
        self.set_eb(victim, 0x33, DEMON_VICTORY_FRAMES - 1);
        self.toggle_state.raging_demon.victory_pose = true;
        self.clear_raging_demon_blackout();
    }

    fn run_demon_victory(&mut self, me: u16, victim: u16) {
        let victim_down = self.eb(victim, 8) == 8;
        if self.eb(victim, 9) & 0x80 != 0
            && (victim_down || self.eb(victim, 0xa) == crate::combo::throw::THROWN_MOVE)
        {
            return;
        }
        self.toggle_state.raging_demon.victory_pose = false;
        if victim_down {
            self.end_move(me);
        } else {
            self.hold_evil_ryu_stance(me, 1);
            self.set_eb(me, 9, 0x12);
        }
    }

    /// 0x42b34..0x42bdc: in state 0x81 (a jump), a latched punch (player
    /// +0) or, failing that, kick (+1) strength above 0x10 moves into the
    /// entity (+0x34 / +0x35) and marks the state (+9 |= 0x40), to be
    /// re-latched when the stream ends (0x42c60). The original tests moves
    /// 0x0b, 0x0a, and the rest on separate but identical paths.
    fn buffer_air_strength(&mut self, me: u16) {
        let ps = self.el(me, 0x38);
        let p = self.rb(ps);
        if p as u16 > 0x10 {
            self.wb(ps, 0);
            self.set_eb(me, 0x34, p);
        } else {
            let k = self.rb(ps + 1);
            if (k as u16) <= 0x10 {
                return;
            }
            self.wb(ps + 1, 0);
            self.set_eb(me, 0x35, k);
        }
        self.set_eb(me, 9, self.eb(me, 9) | 0x40);
    }

    /// 0x42be2: advance the animation stream of a busy entity (+9 bit 7).
    fn animate(&mut self, me: u16, other: u16) {
        // The trial advances two native recovery frames per tick after Geki throws.
        let accelerated = self.world_tour_shuriken_recovering(self.el(me, 0x38), self.eb(me, 0xa));
        self.animate_frame(me, other);
        if accelerated && self.world_tour_shuriken_recovering(self.el(me, 0x38), self.eb(me, 0xa)) {
            self.animate_frame(me, other);
        }
    }

    fn animate_frame(&mut self, me: u16, other: u16) {
        if self.eb(me, 9) & 0x80 == 0 {
            return;
        }
        if self.toggles.anytime_jump_attacks && self.try_anytime_jump_attack(me, other) {
            return;
        }
        self.set_eb(me, 0x30, self.eb(me, 0x30).wrapping_add(1));
        let hold = self.eb(me, 0x33);
        if hold != 0 {
            self.set_eb(me, 0x33, hold - 1);
            self.step_air_attack_trajectory(me);
            self.judge_targets(me, other);
            return;
        }
        self.set_eb(me, 0x33, self.eb(me, 0xe));
        if self.eb(me, 0xb) != 0 {
            // 0x42cd6: repeat the frame with its (dy, dx) step.
            self.set_eb(me, 0xb, self.eb(me, 0xb) - 1);
            let dy = sext8(self.eb(me, 0x14));
            let dx = self.world_tour_horizontal_step(
                self.el(me, 0x38), self.eb(me, 0xa), sext8(self.eb(me, 0x15)) as i16,
            ) as u16;
            for s in self.grid_entries(me) {
                let y = self.ew(s, 4).wrapping_add(dy);
                let x = self.ew(s, 6).wrapping_add(dx);
                self.set_ew(s, 4, y);
                self.set_ew(s, 6, x);
            }
            self.step_air_attack_trajectory(me);
            self.push_apart(me, other);
            self.clamp_y(me);
            self.clamp_x(me);
            self.judge_targets(me, other);
            return;
        }
        let mut a1 = self.el(me, 0x2c);
        let mut mat_slam = false;
        loop {
            match self.rb(a1) {
                // True combos (SP17): a combo-able reaction whose stream
                // runs out before its hitstun holds its last frame.
                0xfd | 0xff if self.combo_holds(me, a1) => return,
                0xff => {
                    // 0x42c2a: the stream calls a routine (6-byte entry:
                    // 0xff, 0, address), which continues the frame in one
                    // of three ways.
                    let routine = self.rl(a1 + 2);
                    if self.eb(me, 8) == 2 && crate::data_bank::in_bank(routine)
                        && self.rb(routine) == crate::data_bank::ROUTINE_THROW_GET_UP
                    {
                        let delay = self.era_sf4_wakeup_extra(self.el(me, 0x38));
                        if delay != 0 {
                            // Keep the lying pose and repeat the get-up call after the hold.
                            self.set_eb(me, 0x33, delay - 1);
                            return;
                        }
                    }
                    match self.stream_call(me, other, routine, a1) {
                        // jmp 0x42cae: load the frame after the entry,
                        // without re-checking for control bytes.
                        Some(Resume::Load) => {
                            a1 += 6;
                            break;
                        }
                        Some(Resume::LoadMatSlam) => {
                            a1 += 6;
                            mat_slam = true;
                            break;
                        }
                        Some(Resume::RevealDemon) => {
                            self.load_frame(me, a1 + 6);
                            self.clamp_y(me);
                            self.record_mat_impact(me);
                            self.reveal_raging_demon(other, me);
                            return;
                        }
                        Some(Resume::Hold) => {
                            self.set_eb(me, 0x33, 0);
                            return;
                        }
                        Some(Resume::LoadAt(stream)) => {
                            a1 = stream;
                            break;
                        }
                        // jmp 0x42c20: continue the control-byte loop.
                        Some(Resume::Control) => a1 += 6,
                        Some(Resume::ControlAt(stream)) => a1 = stream,
                        // jmp 0x42be2: the stream pointer moved past the
                        // entry; the animation step starts over this frame.
                        Some(Resume::Restart) => {
                            self.set_el(me, 0x2c, a1 + 6);
                            self.animate_frame(me, other);
                            return;
                        }
                        None => {
                            self.note_unrecovered(routine);
                            return;
                        }
                    }
                }
                0xfe => a1 = self.rl(a1 + 2),
                0xfd => {
                    self.end_move(me);
                    return;
                }
                _ => break,
            }
        }
        self.load_frame(me, a1);
        self.step_air_attack_trajectory(me);
        let (me, other) = self.effect_flag_spawn(me, other);
        self.push_apart(me, other);
        self.clamp_y(me);
        self.clamp_x(me);
        if mat_slam {
            self.record_mat_impact(me);
        }
        self.judge_targets(me, other);
    }

    /// Stream-called routines that adjust state before the stream goes on.
    /// Returns how the animation step continues, or None for routines not
    /// translated.
    fn stream_call(&mut self, me: u16, other: u16, routine: u32, a1: u32) -> Option<Resume> {
        let st = self.eb(me, 9);
        match routine {
            // 0x45a7a / 0x45ab2 / 0x45a96: the posture bits of +9 (0x45a7a
            // rising: 1; 0x45ab2 past the apex: 2; 0x45a96: 4), read by the
            // hit reaction (0x4451a).
            0x45a7a => self.set_eb(me, 9, (st & 0xf9) | 1),
            0x45ab2 => self.set_eb(me, 9, (st & 0xfa) | 2),
            0x45a96 => self.set_eb(me, 9, (st & 0xfc) | 4),
            // 0x454ee: the move's landing sound (header byte +0x12).
            0x454ee => {
                self.clear_air_attack_trajectory(self.el(me, 0x38));
                let snd = self.eb(me, 0x12);
                if snd != 0 {
                    self.queue_sound(snd as u16);
                }
                // Native KO streams follow this call with the grounded prone
                // pose. Live hit reactions and jumps recover on their feet.
                if self.eb(me, 8) == 8 && matches!(self.eb(me, 0xa), 0x40..=0x4a) {
                    return Some(Resume::LoadMatSlam);
                }
            }
            // 0x454c6: the whiff sound — while the attack frame is still
            // active (+0xf bit 7; a hit clears it), queue header byte +0x11.
            0x454c6 => {
                let snd = self.eb(me, 0x11);
                if self.eb(me, 0xf) & 0x80 != 0 && snd != 0 {
                    self.queue_sound(snd as u16);
                }
            }
            // 0x452e0 (Ryu) / 0x453b6 (Ken): the jump's apex. Without a
            // buffered attack (+9 != 0xc1) the entry is skipped; with one
            // the jump turns into its jump attack.
            0x452e0 | 0x453b6 if st != 0xc1 => {}
            0x452e0 => return Some(self.jump_attack(me, 0x45386, 0x4539e)),
            // 0x4548c: the Hadoken's projectile — sound 7, then the effect
            // template 0x186dc spawned at the attack box (0x42e1e).
            // 0x45ace..0x45aee: Sagat's Tiger Shot projectile — sound 7, then
            // effect template 0x1873c spawned at the attack box (0x42e1e).
            0x4548c | 0x45ace => {
                let owner = self.el(me, 0x38);
                let move_id = self.eb(me, 0xa);
                if routine == 0x4548c && self.era_sf4_hold_metsu(owner, move_id) {
                    return Some(Resume::Hold);
                }
                self.queue_sound(7);
                let template = if routine == 0x4548c { 0x186dc } else { 0x1873c };
                self.spawn_effect(me, template);
                if routine == 0x4548c {
                    self.era_sf4_finish_metsu(owner, move_id);
                }
            }
            // 0x454ae: the same spawn with template 0x1871c, without the
            // sound.
            0x454ae => self.spawn_effect(me, 0x1871c),
            // 0x455de: queue the sound command in the entry's second byte.
            0x455de => {
                let snd = self.rb(a1 + 1);
                self.queue_sound(snd as u16);
            }
            // 0x45630: the CPU opponent's follow-up decision. A human playing
            // that fighter's raw table ends the action here instead, so the
            // AI does not take the next attack (`#98`, `O7`).
            0x45630 => {
                if self.human_controlled_boss(me) {
                    self.end_move(me);
                    return Some(Resume::Restart);
                }
                return Some(self.cpu_follow_up(me, other, a1));
            }
            // 0x4550a / 0x4554a (Ryu / Ken): the Tatsumaki's spin loop start
            // — the entry's second byte is the repeat count and the frame
            // after the entry the loop's start, kept in globals (Ryu
            // 0xff8704 / 0xff8706, Ken 0xff870a / 0xff870c).
            0x4550a | 0x4554a => {
                let (count, start) = if routine == 0x4550a {
                    (0xff8704, 0xff8706)
                } else {
                    (0xff870a, 0xff870c)
                };
                self.wb(count, self.rb(a1 + 1));
                self.wl(start, a1 + 6);
            }
            // 0x45526 / 0x45566: the loop's end — while the count is not
            // zero, count down and go back to the loop's start.
            0x45526 | 0x45566 => {
                let (count, start) = if routine == 0x45526 {
                    (0xff8704, 0xff8706)
                } else {
                    (0xff870a, 0xff870c)
                };
                let n = self.rb(count);
                if n != 0 {
                    self.wb(count, n - 1);
                    return Some(Resume::LoadAt(self.rl(start)));
                }
            }
            // 0x4558a..0x455d8: Adon's mid-combo turn helper (streams 0x13850
            // and 0x1398c). Compares the attacker's centre X (D2, 0x4558a)
            // against the opponent's (D7, 0x45590): when D2 < D7 (0x4559a bcs
            // 0x455bc) and +0x37 != 2 (0x455c2), sets +0x37 = 2 and calls
            // 0x4405a face_right; when D2 == D7 (0x4559e beq 0x455d2), leaves
            // facing unchanged; when D2 > D7 (0x455a2) and +0x37 != 4
            // (0x455a8), sets +0x37 = 4 and calls 0x43fa8 face_left; then
            // advances A1 by 6 and loads the next frame at 0x42cae (0x455d2).
            0x4558a => {
                let my_cx = self.center_x(me);
                let ot_cx = self.center_x(other);
                if my_cx < ot_cx {
                    if self.eb(me, 0x37) != 2 {
                        self.set_eb(me, 0x37, 2);
                        self.face_right(me);
                    }
                } else if my_cx > ot_cx && self.eb(me, 0x37) != 4 {
                    self.set_eb(me, 0x37, 4);
                    self.face_left(me);
                }
            }
            0x453b6 => return Some(self.jump_attack(me, 0x4545c, 0x45474)),
            // 0x4594c: Geki's disappearing-smoke effect and its sound.
            0x4594c => {
                self.spawn_effect(me, 0x1847c);
                self.queue_sound(0x20);
            }
            // 0x4596e: Geki's shuriken throw. With TIME > 5 and health below
            // 24, throw all three templates. Every other path joins 0x459c4:
            // centre distance selects one template, then the sound is queued.
            0x4596e => {
                let health = 4 * self.rb(CPU_STRUCT + 7) as u16 + self.rb(CPU_STRUCT + 6) as u16;
                let triple = self.compare_bytes(0xff8e18, 0x45a18, 2)
                    == std::cmp::Ordering::Greater
                    && health < 0x18;
                if triple {
                    for template in [0x1849c, 0x184bc, 0x184dc] {
                        self.spawn_effect(me, template);
                    }
                } else {
                    let distance = Self::borrow_abs(self.center_x(other), self.center_x(me));
                    let mut index = 0;
                    if distance < 0x58 {
                        index += 1;
                    }
                    if index != 0 && distance < 0x84 {
                        index += 1;
                    }
                    self.spawn_effect(me, self.rl(0x45a24 + index * 4));
                }
                self.queue_sound(0x20);
            }
            // 0x45a34: Geki's teleport chooses one of the three frames in
            // the entry's group by the random nibble at 0xff8e4e. The table
            // at 0x45a6a supplies 0, 1 or 2; each group is 0x20 bytes. D2
            // starts at 6 and advances once per choice (6, 12, 18).
            0x45a34 => {
                let roll = (self.rb(0xff8e4e) & 0xf) as u32;
                let choice = self.rb(0x45a6a + roll) as u32;
                let group = (choice + 1) * 6;
                return Some(Resume::LoadAt(a1 + choice * 0x20 + group));
            }
            // 0x455fc: the crouching medium and heavy punches' extra hold on
            // their first frame, by strength (table 0x45620: 0, 6, 9). With
            // a hold the stream moves past the entry and the animation step
            // restarts (holding this frame); without one the control loop
            // goes on.
            0x455fc => {
                let i = (sext8(self.eb(me, 0x34)) as i16).wrapping_sub(1);
                let hold = self.rb((0x45620i32 + i as i32) as u32);
                if hold == 0 {
                    return Some(Resume::Control);
                }
                self.set_eb(me, 0x33, hold);
                return Some(Resume::Restart);
            }
            // A routine record in the extended data bank (ADR-0014): only a
            // bank stream names one.
            r if crate::data_bank::in_bank(r) => return self.bank_routine(me, other, r, st),
            _ => return None,
        }
        Some(Resume::Load)
    }

    /// A routine record of the extended data bank ([`crate::data_bank`]):
    /// the 1987 routine it stands for, with the parameters the 1987 routine
    /// hard-codes taken from the record. `None` for a kind it does not know.
    fn bank_routine(&mut self, me: u16, other: u16, r: u32, st: u8) -> Option<Resume> {
        match self.rb(r) {
            crate::data_bank::ROUTINE_POSE_ORIGIN => {
                let sound = self.rb(r + 1);
                if sound != 0 {
                    self.queue_sound(sound as u16);
                }
                // Convert pose origins in the same tick as the next frame
                // loads, including when a posture call comes before the frame.
                self.shift_grid_y(me, self.rw(r + 2));
                Some(Resume::Control)
            }
            // As 0x4548c: the sound, then the projectile from the record's
            // template, spawned at the attack box (0x42e1e).
            crate::data_bank::ROUTINE_PROJECTILE => {
                let snd = self.rb(r + 1);
                if snd != 0 {
                    self.queue_sound(snd as u16);
                }
                self.spawn_effect(me, self.rl(r + 4));
                Some(Resume::Load)
            }
            // The same with the template the button's strength level
            // (+0x34: 1 light .. 3 heavy) picks from the record's three.
            crate::data_bank::ROUTINE_PROJECTILE_BY_STRENGTH => {
                let snd = self.rb(r + 1);
                if snd != 0 {
                    self.queue_sound(snd as u16);
                }
                let level = (self.eb(me, 0x34) as u32).clamp(1, 3) - 1;
                self.spawn_effect(me, self.rl(r + 4 + level * 4));
                self.world_tour_shuriken_recovery(self.el(me, 0x38), true);
                Some(Resume::Load)
            }
            crate::data_bank::ROUTINE_RYUSOKYAKU_VARIANT => {
                let strength = u32::from(self.eb(me, 0x34).clamp(1, 3));
                let slot = self.eb(me, 0xa);
                let variant = match self.era_state(self.el(me, 0x38)) {
                    Some(state) if state.sf4_ex_axe_kick(slot) || state.sf6_evil_ryu_od_axe_kick(slot) => 3,
                    Some(state)
                        if strength == 3
                            && state.style.revision == crate::era::Revision::Usf4
                            && state.is_evil_ryu_sf4() => 4,
                    _ => strength - 1,
                };
                Some(Resume::LoadAt(self.rl(r + 4 + variant * 4)))
            }
            crate::data_bank::ROUTINE_KAGE_SENHA => {
                let owner = self.el(me, 0x38);
                Some(match self.sf5_kage_senha_branch(owner) {
                    Some(branch) => Resume::ControlAt(self.rl(r + 4 + u32::from(branch) * 4)),
                    None => Resume::Hold,
                })
            }
            crate::data_bank::ROUTINE_KAGE_MISOGI_ALIGN => {
                let apex = u16::from(self.rb(r + 1));
                let approach_limit = self.rw(r + 2) as i16;
                let facing = if self.eb(me, 0x37) & 4 == 0 { 1 } else { -1 };
                let gap = (self.center_x(other) as i16 - self.center_x(me) as i16) * facing;
                let approach = (gap - 42).clamp(0, approach_limit) * facing;
                self.shift_grid_x(me, approach as u16);
                let vertical = self.height_above_mat(me).wrapping_sub(apex);
                self.shift_grid_y(me, vertical);
                Some(Resume::Control)
            }
            // The fighter, unseen in his smoke, moves to the record's gap
            // in front of the opponent's centre on his own side (by his
            // facing); the separation and clamps of the frame then apply as
            // to any step. Air Utsusemi sets its falling posture before that
            // frame, so continue reading control entries after the warp.
            crate::data_bank::ROUTINE_TELEPORT => {
                let gap = self.rw(r + 2);
                let target = if (self.eb(me, 0x37) as i8) < 4 {
                    self.center_x(other).wrapping_sub(gap)
                } else {
                    self.center_x(other).wrapping_add(gap)
                };
                self.shift_grid_x(me, target.wrapping_sub(self.center_x(me)));
                Some(Resume::Control)
            }
            // As 0x452e0: the jump's apex, skipped without a buffered
            // attack.
            crate::data_bank::ROUTINE_JUMP_APEX if st != 0xc1 => Some(Resume::Load),
            crate::data_bank::ROUTINE_JUMP_APEX => Some(self.jump_attack_bank(me, r)),
            // Throws (SP19): `me` is the victim running his `thrown` stream,
            // `other` the thrower.
            crate::data_bank::ROUTINE_ALPHA_HOLD_P => {
                if self.era_hold_punch(self.el(me, 0x38), self.rb(r + 1)) {
                    Some(Resume::LoadAt(self.rl(r + 4)))
                } else {
                    Some(Resume::Control)
                }
            }
            crate::data_bank::ROUTINE_ALPHA_REPEAT_K => {
                let ps = self.el(me, 0x38);
                if self.era_take_repeated_kick(ps) {
                    Some(Resume::Control)
                } else {
                    Some(Resume::ControlAt(self.rl(r + 4)))
                }
            }
            crate::data_bank::ROUTINE_SF6_TIGER_NEXUS => {
                use crate::era_fight::what_if_others::{GREEDY_TIGER, MIGHTY_TIGER, NEXUS, NOVA_TIGER, OD_NEXUS};

                let ps = self.el(me, 0x38);
                let (held, _) = self.era_buttons(ps);
                let kicks = (held >> 3) & 7;
                let connected = self.era_state(ps).is_some_and(|state| {
                    (state.what_if_sagat_slot(NEXUS) || state.what_if_sagat_slot(OD_NEXUS))
                        && state.move_hits != 0
                });
                if connected && kicks != 0 && self.relative_directions(me, self.pad_directions(ps)) == 1 {
                    let strength = kicks.ilog2() as u8;
                    let slot = [MIGHTY_TIGER, GREEDY_TIGER, NOVA_TIGER][usize::from(strength)];
                    self.era_stage_banked_move(ps, slot, 0, None);
                    self.set_eb(me, 0xa, slot);
                    let attacker = self.combo_slot(me);
                    self.toggle_state.combo.connected[attacker] = false;
                    Some(Resume::LoadAt(self.rl(r + 4 + u32::from(strength) * 4)))
                } else {
                    Some(Resume::Control)
                }
            }
            crate::data_bank::ROUTINE_ALPHA_CATCH => {
                if self.try_alpha_catch(me, other, self.rb(r + 4).into(), self.rb(r + 5)) {
                    Some(Resume::Restart)
                } else {
                    Some(Resume::Load)
                }
            }
            crate::data_bank::ROUTINE_ALPHA_WALL_REBOUND => {
                let cx = self.center_x(me);
                let rear = u16::from(self.eb(me, 0x19));
                let intro = self.rb(SCENE_MODE) == 0xc0;
                let at_wall = if self.eb(me, 0x37) & 4 == 0 {
                    cx.saturating_sub(rear) <= if intro { 0xb0 } else { 0x60 }
                } else {
                    cx + rear >= if intro { 0x16e } else { 0x1a0 }
                };
                if at_wall || self.dramatic_rules().is_some() {
                    Some(Resume::Control)
                } else {
                    Some(Resume::LoadAt(self.rl(r + 4)))
                }
            }
            crate::data_bank::ROUTINE_THROW_ARC => {
                let s = self.combo_slot(me);
                if self.toggle_state.throw.crosses[s] {
                    self.throw_turn_for_crossing(other, me);
                }
                Some(Resume::Load)
            }
            crate::data_bank::ROUTINE_AIR_THROW_LAND => {
                let height = self.height_above_mat(me);
                if height == 0 {
                    self.set_eb(me, 9, (self.eb(me, 9) & !3) | 2);
                    Some(Resume::Control)
                } else {
                    self.shift_grid_y(me, height.min(6));
                    Some(Resume::Hold)
                }
            }
            crate::data_bank::ROUTINE_THROW_SLAM => {
                if self.evil_ryu_profile(self.el(other, 0x38)).is_some()
                    && matches!(self.toggle_state.raging_demon.blackout_frames, Some(1..))
                {
                    return Some(Resume::Hold);
                }
                Some(if self.throw_slam(me, other) {
                    Resume::RevealDemon
                } else {
                    Resume::LoadMatSlam
                })
            }
            crate::data_bank::ROUTINE_THROW_GET_UP => {
                if self.eb(me, 8) == 8 {
                    self.end_move(me);
                    return Some(Resume::Restart);
                }
                self.set_eb(me, 0xa, crate::combo::throw::GET_UP_MOVE);
                Some(Resume::Load)
            }
            _ => None,
        }
    }

    /// [`Self::jump_attack`] with a bank apex record's tables: the punch
    /// entries at +4 and the kick entries at +0x1c, 8 bytes each by jump
    /// (strength table, stream table), whose streams are longs.
    fn jump_attack_bank(&mut self, me: u16, rec: u32) -> Resume {
        let d5 = (self.eb(me, 0xa) as u32).wrapping_sub(9).wrapping_mul(8);
        let ps = self.el(me, 0x38);
        let stream = |b: &Self, entry: u32| {
            let level = (b.eb(me, 0x34) as u32).wrapping_sub(1);
            b.rl(b.rl(entry + 4).wrapping_add(level.wrapping_mul(4)))
        };
        let p = self.eb(me, 0x34);
        if p != 0 {
            let entry = rec.wrapping_add(4).wrapping_add(d5);
            if let Some(mv) = self.pick_preloaded(p, self.rl(entry), me, ps) {
                self.set_eb(me, 0xa, mv);
                return Resume::LoadAt(stream(self, entry));
            }
        }
        let k = self.eb(me, 0x35);
        if k != 0 {
            let entry = rec.wrapping_add(0x1c).wrapping_add(d5);
            if let Some(mv) = self.pick_preloaded(k, self.rl(entry), me, ps) {
                self.set_eb(me, 0xa, mv);
                self.set_eb(me, 0x35, 0);
                return Resume::LoadAt(stream(self, entry));
            }
        }
        Resume::Load
    }

    /// A move table the 1987 player handler runs: Ryu's, Ken's, or a boss's
    /// in the extended data bank.
    fn is_player_table(table: u32) -> bool {
        matches!(table, 0x1605c | 0x1662c) || crate::data_bank::in_bank(table)
    }

    /// Whether the move table bound to `ps` has move `mv`. A ROM table has
    /// every move the handler starts; a bank table may leave a special out
    /// (its header's first-frame byte is 0), and the handler then does not
    /// start it (an overhaul-only gate: no 1987 table is in the bank).
    fn move_in_table(&self, ps: u32, mv: u8) -> bool {
        let table = self.rl(ps + 0x12);
        !crate::data_bank::in_bank(table)
            || crate::data_bank::move_header(|addr| self.rl(addr), table, mv)
                .is_some_and(|header| self.rb(header + 8) != 0)
    }

    /// Special slot `slot` of the table bound to `ps`: a boss's bank record,
    /// or the 1987 slot on a ROM table ([`crate::data_bank::special_slot`]).
    fn special_slot(&self, ps: u32, slot: u8) -> crate::data_bank::SpecialSlot {
        crate::data_bank::special_slot(|a| self.rb(a), self.rl(ps + 0x12), slot)
    }

    /// The special the command tracker at player `+off` (`0x16`, `0x19` or
    /// `0x1c`) fires with a punch (`kick` false) or a kick, for the fighter
    /// on the table bound to `ps`, if he has one: on a ROM table the 1987
    /// three (the Hadoken's tracker and a punch the Hadoken, the
    /// Shoryuken's and a punch the Shoryuken, the Tatsumaki's and a kick the
    /// Tatsumaki); on a bank table whichever of his special slots his table
    /// puts on that tracker and button (validated to be at most one when the
    /// bank is built, [`crate::data_bank::validate_specials`]).
    fn tracker_special(&self, ps: u32, off: u32, kick: bool) -> Option<u8> {
        crate::data_bank::SPECIAL_SLOTS.into_iter().find(|&slot| {
            let s = self.special_slot(ps, slot);
            (s.tracker, s.kick) == (off, kick) && self.move_in_table(ps, slot)
                && !self.alpha_state(ps).is_some_and(|era| era.replaces_special(slot))
        })
    }

    /// The tracker at `ps + off` has its command done and the fighter has a
    /// special it fires with the button (punch, or `kick`): that special.
    fn command_done(&self, ps: u32, off: u32, kick: bool) -> Option<u8> {
        if self.world_tour_pure_footsies() || self.rb(self.tracker_base(ps) + off + 2) == 0 {
            return None;
        }
        self.tracker_special(ps, off, kick)
    }

    /// Whether the fighter on `ps` is a player on a boss's 1987 player
    /// table in the extended data bank (an overhaul fight only).
    fn on_bank_table(&self, ps: u32) -> bool {
        matches!(ps, P1_STRUCT | P2_STRUCT) && crate::data_bank::in_bank(self.rl(ps + 0x12))
    }

    /// A bank fighter driven by player input, including the overhaul's
    /// Arcade opponent. Scoring still uses the human-only predicate above.
    fn on_bank_player_table(&self, ps: u32) -> bool {
        self.is_player_driven(ps) && crate::data_bank::in_bank(self.rl(ps + 0x12))
    }

    /// The damage entry whose score byte a boss's own 1987 attack scores
    /// with when a player lands it on his bank table (the maintainer, 2026-11-08:
    /// as Ryu's attacks score): Ryu's entry for the same move, by the hit's
    /// depth (`0x45b6e` light punch .. `0x46240` heavy kick, `0x4637e` the
    /// sweep), or for a crouching punch, a jump punch or a jump kick his
    /// entry for it by strength (`0x45de0`, `0x45eca`, `0x46462`), or for a
    /// special the score entry the boss's table gives its slot
    /// ([`crate::data_bank::SpecialSlot`]). `None` off a
    /// bank table (the 1987 CPU's hits, and every fighter in Faithful 1987,
    /// never score) and for a move Ryu has no entry for.
    fn player_score_entry(&self, me: u16, depth: u8) -> Option<u32> {
        let ps = self.el(me, 0x38);
        if !self.on_bank_table(ps) {
            return None;
        }
        let odd = (depth as u32 & 1) * 8;
        let by_level = |table: u32| table + ((self.eb(me, 0x34) as u32).clamp(1, 3) - 1) * 16 + odd;
        let table = match self.eb(me, 0xa) {
            0x0d | 0x0e => 0x45b6e,
            0x0f => 0x45c88,
            0x10 => 0x45d16,
            0x14 => 0x460a8,
            0x15 => 0x46154,
            0x16 => 0x46240,
            0x18 => 0x4637e,
            0x11..=0x13 | 0x1c..=0x1e => return Some(by_level(0x45de0)),
            0x17 => return Some(by_level(0x462e0)),
            0x20..=0x22 => return Some(by_level(0x45eca)),
            0x23..=0x26 => return Some(by_level(0x46462)),
            // A special scores with the entry his table gives the slot.
            0x19..=0x1b => {
                let score = self.special_slot(ps, self.eb(me, 0xa)).score;
                return (score != 0).then_some(score);
            }
            _ => return None,
        };
        Some(table + odd)
    }

    /// A bank effect's hit on its target (a [`crate::data_bank`] routine
    /// record of kind `ROUTINE_EFFECT_HIT` named as the effect's +0x14
    /// handler): as the Hadoken's 0x433ba, but its damage entries, its
    /// impact effect and whether it cuts the thrower's hold short come from
    /// the record, and it never asks for the slow motion.
    fn bank_effect_hit(&mut self, s: u16, rec: u32) {
        let attack_x = self.effect_centre(s).0;
        let era = self.era_projectile(s);
        if self.era_projectile_hit(s) {
            self.transform_effect(s, self.rl(rec + 0xc));
        }
        let owner_ps = self.el(s, 0x20);
        let (me, other) = self.player_entities(owner_ps);
        let air_block = !era.unblockable && self.alpha_air_blocks(other, attack_x);
        let is_blocked = !era.unblockable && (air_block || self.eb(other, 9) & 0x20 != 0);
        let entry = self.rl(rec + if is_blocked { 8 } else { 4 });
        if self.rb(rec + 1) & 1 != 0 {
            self.set_eb(me, 0x33, 0);
            self.set_eb(me, 0xe, 0);
        }
        self.add_score(me, self.rb(entry));
        self.show_score(owner_ps);
        self.check_hiscore(me);
        let dmg = if is_blocked && self.era_state(self.el(other, 0x38)).is_some_and(|state| state.sf6_burnout()) {
            self.effective_entry_damage(me, self.rl(rec + 4), false).div_ceil(4)
        } else {
            self.effective_entry_damage(me, entry, is_blocked)
        };
        self.damage_with_source(me, dmg, is_blocked, Some(era));
        self.react_projectile(me, other, entry, is_blocked, air_block);
    }

    /// 0x45630: the CPU opponent's follow-up decision, called at the end of
    /// each of its move streams (`me` the CPU's fighter, `other` the
    /// player's). It continues the stream's control loop at:
    ///
    /// - past the entry + 2 (A1 + 8) while the CPU structure's countdown
    ///   +0x19 runs (it counts down);
    /// - past the entry (A1 + 6) when the player is down (+8 == 8) or TIME
    ///   is zero (the word at 0xff8e18 with 0xff8e1a or'ed into its low
    ///   byte);
    /// - the stream of a reaction record when the player is attacking (+9
    ///   bit 6), the react roll (0xffc0d4 by random byte 3) is 0x20, and the
    ///   player's attack can reach: its reach (word table 0x146aa by the
    ///   player's move id - 0xd, or - 0x12 from 0x20) plus the CPU's +0x21
    ///   is at least the centres' distance (0x4571a);
    /// - otherwise, with the distance inside the CPU's range band (+0x16
    ///   .. +0x17), the stream of an attack record (0x457fa) for the range
    ///   class (+0x18) and the player's posture (+9 & 7, halved);
    /// - otherwise the range-shift roll (0xffc0c4 by random byte 6) of 0x40
    ///   tries the next two range classes (0xffc068 pairs) for one whose
    ///   band holds the distance; failing that, or without the roll, the
    ///   held range roll is requested (+0x1b = 0x80) and the stream goes on
    ///   past the entry. The reach branch's band test compares only the
    ///   distance's low byte (cmp.b), the others the word.
    ///
    /// A record (8 bytes: stream, +9, +0xa, two bytes) becomes the CPU's
    /// new stream and state; the CPU turns to face the player first
    /// (+0x37 = 2 or 4).
    fn cpu_follow_up(&mut self, me: u16, other: u16, a1: u32) -> Resume {
        let cs = CPU_STRUCT;
        if self.rb(cs + 0x19) != 0 {
            self.wb(cs + 0x19, self.rb(cs + 0x19) - 1);
            return Resume::ControlAt(a1 + 8);
        }
        if self.eb(other, 8) == 8 {
            return Resume::ControlAt(a1 + 6);
        }
        let t = self.rw(0xff8e18);
        if ((t & 0xff00) | ((t as u8) | self.rb(0xff8e1a)) as u16) == 0 {
            return Resume::ControlAt(a1 + 6);
        }
        let distance = |b: &Self| {
            let d = b.center_x(other).wrapping_sub(b.center_x(me));
            if b.center_x(other) < b.center_x(me) {
                d.wrapping_neg()
            } else {
                d
            }
        };
        let in_band = |b: &Self, d: u16| d >= b.rb(cs + 0x16) as u16 && d <= b.rb(cs + 0x17) as u16;
        let can_read_attack = if self.toggles.fair_cpu_and_sagat_scaling {
            let slot = self.player_slot_index(self.el(other, 0x38)).unwrap_or(0);
            self.toggle_state.human_attack_frames[slot] >= self.toggles.cpu_reaction_delay
        } else {
            true
        };
        if self.eb(other, 9) & 0x40 != 0 && can_read_attack {
            let roll = (self.rb(RNG_STATE + 3) & 0xf) as u32;
            if self.rb(0xffc0d4 + roll) == 0x20 {
                // 0x4568a: the player's attack's reach.
                let mv = self.eb(other, 0xa);
                let row = if mv >= 0x20 {
                    mv.wrapping_sub(0x12)
                } else {
                    mv.wrapping_sub(0xd)
                };
                let reach = self.rw(0x146aa + ((row as u16) << 1) as u32);
                let d2 = distance(self);
                let d0 = (self.eb(me, 0x21) as u16).wrapping_add(reach);
                if d0 >= d2 {
                    return self.cpu_react(me, other);
                }
                // 0x456da (byte compares).
                let lo = d2 as u8;
                if lo >= self.rb(cs + 0x16) && lo <= self.rb(cs + 0x17) {
                    return Resume::ControlAt(a1 + 6);
                }
                self.wb(cs + 0x1b, 0x80);
                return Resume::ControlAt(a1 + 6);
            }
        }
        // 0x457c8
        let d2 = distance(self);
        if in_band(self, d2) {
            return self.cpu_attack(me, other);
        }
        // 0x458a2: the range-shift roll.
        let roll = (self.rb(RNG_STATE + 6) & 0xf) as u32;
        if self.rb(0xffc0c4 + roll) != 0x40 {
            // 0x456da with this frame's distance (byte compares).
            let lo = d2 as u8;
            if lo >= self.rb(cs + 0x16) && lo <= self.rb(cs + 0x17) {
                return Resume::ControlAt(a1 + 6);
            }
            self.wb(cs + 0x1b, 0x80);
            return Resume::ControlAt(a1 + 6);
        }
        let (saved_band, saved_class) = (self.rw(cs + 0x16), self.rb(cs + 0x18));
        for _ in 0..2 {
            let mut class = (sext8(self.rb(cs + 0x18)) as i16).wrapping_add(1) as u16;
            if class > 2 {
                class = 0;
            }
            self.wb(cs + 0x18, class as u8);
            let pair = 0xffc068 + ((class << 1) as u32);
            self.ww(cs + 0x16, self.rw(pair));
            if d2 >= self.rb(pair) as u16 && d2 <= self.rb(pair + 1) as u16 {
                return self.cpu_attack(me, other);
            }
        }
        self.wb(cs + 0x18, saved_class);
        self.ww(cs + 0x16, saved_band);
        self.wb(cs + 0x1b, 0x80);
        Resume::ControlAt(a1 + 6)
    }

    /// The CPU turns to face the player (0x45734.. / 0x45834..): +0x37 = 4
    /// when its centre is at or right of the player's, else 2.
    fn cpu_face(&mut self, me: u16, other: u16) {
        let want = if self.center_x(me) >= self.center_x(other) {
            4
        } else {
            2
        };
        if self.eb(me, 0x37) != want {
            self.set_eb(me, 0x37, want);
        }
    }

    /// 0x4571a: a reaction record (vs airborne attacks 0xffc0e4 when the
    /// player's +9 & 3 is set, else vs ground attacks 0xffc164) by random
    /// byte 4. A jump record (+9 0x81) also sets +0x11 / +0x12; any other
    /// sets +0x12 and the countdown +0x19 from the record's last two bytes.
    fn cpu_react(&mut self, me: u16, other: u16) -> Resume {
        let table = if self.eb(other, 9) & 3 != 0 {
            0xffc0e4
        } else {
            0xffc164
        };
        self.cpu_face(me, other);
        if self.toggles.true_combos {
            let s = self.combo_slot(me);
            self.toggle_state.combo.connected[s] = false;
        }
        let rec = table + (((self.rb(RNG_STATE + 4) & 0xf) as u32) << 3);
        let stream = self.rl(rec);
        self.set_eb(me, 9, self.rb(rec + 4));
        self.set_eb(me, 0xa, self.rb(rec + 5));
        if self.eb(me, 9) == 0x81 {
            self.set_eb(me, 0x11, self.rb(rec + 6));
            self.set_eb(me, 0x12, self.rb(rec + 7));
        } else {
            self.set_eb(me, 0x12, self.rb(rec + 7));
            self.wb(CPU_STRUCT + 0x19, self.rb(rec + 6));
        }
        self.wb(CPU_STRUCT + 0x1b, 0);
        Resume::ControlAt(stream)
    }

    /// 0x457fa: an attack record for the range class (+0x18: 0 → 0xffc1e4,
    /// 1 → 0xffc364, 2 → 0xffc4e4) and the player's posture row ((+9 & 7)
    /// / 2, 0x80 bytes each), by random byte 5.
    fn cpu_attack(&mut self, me: u16, other: u16) -> Resume {
        let table = match self.rb(CPU_STRUCT + 0x18) {
            2 => 0xffc4e4,
            1 => 0xffc364,
            _ => 0xffc1e4,
        };
        let row = (((self.eb(other, 9) & 7) as u32) >> 1) << 7;
        self.cpu_face(me, other);
        if self.toggles.true_combos {
            let s = self.combo_slot(me);
            self.toggle_state.combo.connected[s] = false;
        }
        let rec = table + row + (((self.rb(RNG_STATE + 5) & 0xf) as u32) << 3);
        let stream = self.rl(rec);
        self.set_eb(me, 9, self.rb(rec + 4));
        self.set_eb(me, 0xa, self.rb(rec + 5));
        self.set_eb(me, 0x11, self.rb(rec + 6));
        self.set_eb(me, 0x12, self.rb(rec + 7));
        self.wb(CPU_STRUCT + 0x1b, 0);
        Resume::ControlAt(stream)
    }

    /// 0x452f4..0x45380 (Ryu; 0x453ca.. Ken): the apex of a jump with a
    /// buffered attack. The jump (move 9 / 0xa / 0xb) selects an 8-byte
    /// entry of the punch / kick tables (`punch`, `kick`: a (threshold, move)
    /// strength table and a stream table). A buffered punch strength (+0x34)
    /// is mapped first, then a kick (+0x35) through the strength mapper's
    /// preloaded entry (0x40b8e), which clears the byte at A0 — in the
    /// object runner the player structure's punch latch, even for a kick.
    /// The mapped move id replaces +0xa and the stream continues at the
    /// stream table's entry for the strength level; with nothing mapped the
    /// jump goes on (the entry is skipped).
    fn jump_attack(&mut self, me: u16, punch: u32, kick: u32) -> Resume {
        let d5 = (self.eb(me, 0xa) as u16).wrapping_sub(9).wrapping_shl(3) as i16 as i32;
        let ps = self.el(me, 0x38);
        let entry = |t: u32| (t as i32 + d5) as u32;
        let p = self.eb(me, 0x34);
        if p != 0 {
            if let Some(mv) = self.pick_preloaded(p, self.rl(entry(punch)), me, ps) {
                self.set_eb(me, 0xa, mv);
                return Resume::LoadAt(self.attack_stream(me, entry(punch)));
            }
        }
        let k = self.eb(me, 0x35);
        if k != 0 {
            if let Some(mv) = self.pick_preloaded(k, self.rl(entry(kick)), me, ps) {
                self.set_eb(me, 0xa, mv);
                self.set_eb(me, 0x35, 0);
                return Resume::LoadAt(self.attack_stream(me, entry(kick)));
            }
        }
        Resume::Load
    }

    /// The stream of a mapped jump attack: the word at the entry's stream
    /// table (+4) for strength level +0x34 (movea.w: sign-extended).
    fn attack_stream(&self, me: u16, entry: u32) -> u32 {
        let d0 = (self.eb(me, 0x34) as u16).wrapping_sub(1).wrapping_shl(1);
        let streams = self.rl(entry + 4);
        self.rw(streams.wrapping_add(d0 as u32)) as i16 as i32 as u32
    }

    /// 0x42c4a: stream end — re-latch buffered attack strengths for a
    /// chainable move, then clear the busy bits (+9 &= 0x3f).
    fn end_move(&mut self, me: u16) {
        let ps = self.el(me, 0x38);
        if matches!(self.eb(me, 0xa), 0x28 | 0x35..=0x3f) {
            let delay = self.era_sf4_wakeup_extra(ps);
            if delay != 0 {
                self.set_eb(me, 0x33, delay - 1);
                return;
            }
        }
        if self.eb(me, 0xa) & 0x0b != 0 && self.eb(me, 9) == 0xc1 {
            let p = self.eb(me, 0x34);
            self.set_eb(me, 0x34, 0);
            if p >= 0x10 && self.rb(ps) == 0 {
                self.wb(ps, p);
            }
            let k = self.eb(me, 0x35);
            self.set_eb(me, 0x35, 0);
            if k >= 0x10 && self.rb(ps + 1) == 0 {
                self.wb(ps + 1, k);
            }
        }
        self.clear_air_attack_trajectory(ps);
        // When CPU Adon (0x17f7c) finishes move 4 (after reaction frame 0x1326c),
        // clear +0x1a so cpu_decide (0x40bd4) does not treat the restored tight-box
        // dy byte as a 24-frame post-move idle wait counter once +9 bit 7 clears.
        if self.toggles.standard_high_low_block
            && ps == CPU_STRUCT
            && self.rl(ps + 0x12) == 0x17f7c
            && self.eb(me, 0xa) == 4
        {
            self.set_eb(me, 0x1a, 0);
        }
        self.set_eb(me, 9, self.eb(me, 9) & 0x3f);
        if self.toggles.true_combos {
            // Whatever ended, a combo-able reaction is over, and the next
            // attack has not connected (whatever starts it: start_move, the
            // 1987 CPU's records, the boss-only controls).
            let s = self.combo_slot(me);
            self.toggle_state.combo.hitstun[s] = 0;
            self.toggle_state.combo.fresh[s] = false;
            self.toggle_state.combo.connected[s] = false;
        }
        if self.toggles.cancels {
            // Cancels (SP18): the move is over, and so is its window.
            self.cancel_close(me);
        }
        if self.toggles.throws && self.eb(me, 0xa) == crate::combo::throw::GET_UP_MOVE {
            let slot = self.combo_slot(me);
            self.toggle_state.throw.wakeup_invuln[slot] = crate::combo::throw::WAKEUP_INVULN + 1;
            self.separate_corner_throw(me);
        }
    }

    // ---- separation, clamps, camera requests ----------------------------

    /// 0x43d7c: when the fighters' vertical extents (+0x16 below, +0x17
    /// above the centre) overlap, push `me` horizontally out of the other's
    /// body (+0x18 / +0x19 half-widths, by facing).
    fn push_apart(&mut self, me: u16, other: u16) {
        if self.dramatic_rules().is_some() {
            let owner = self.el(me, 0x38);
            let target = self.el(other, 0x38);
            if !self.dramatic_alive(owner) || !self.dramatic_alive(target)
                || (owner == CPU_STRUCT) == (target == CPU_STRUCT)
            {
                return;
            }
        }
        if self.toggles.throws
            && [me, other].into_iter().any(|e| {
                self.eb(e, 0xa) == crate::combo::throw::THROWN_MOVE
                    && self.toggle_state.throw.crosses[self.combo_slot(e)]
            })
        {
            // A caught victim travels over the thrower's body. Ordinary
            // standing separation would push each airborne step back out
            // of that path before the victim can cross the shoulder.
            return;
        }
        if [me, other].into_iter().any(|entity| {
            let ps = self.el(entity, 0x38);
            (self.evil_ryu_alpha3(ps) || self.evil_ryu_sf4(ps))
                && self.is_ashura_move(ps, self.eb(entity, 0xa)) && self.eb(entity, 0xf) & 0x08 != 0
        }) {
            return;
        }
        if [me, other].into_iter().any(|entity| {
            let slot = self.eb(entity, 0xa);
            let flags = self.eb(entity, 0xf);
            self.era_state(self.el(entity, 0x38)).is_some_and(|state| {
                (state.is_kage() && flags & 8 != 0 && matches!(slot,
                    KAGE_ASHURA_FORWARD | KAGE_ASHURA_BACK | KAGE_ASHURA_AIR_FORWARD | KAGE_ASHURA_AIR_BACK
                )) || (slot == crate::era_fight::sf6::ASHURA_FORWARD
                    && flags & crate::data_bank::SF6_ASHURA_GLIDE != 0
                    && state.sf6_evil_ryu_ashura(slot))
            })
        }) {
            return;
        }
        let my_cy = self.center_y(me);
        let ot_cy = self.center_y(other);
        if ot_cy >= my_cy {
            let reach = (self.eb(other, 0x17) as u16).wrapping_add(self.eb(me, 0x16) as u16);
            if ot_cy.wrapping_sub(my_cy) >= reach {
                return;
            }
        } else {
            let reach = (self.eb(other, 0x16) as u16).wrapping_add(self.eb(me, 0x17) as u16);
            if my_cy.wrapping_sub(ot_cy) >= reach {
                return;
            }
        }
        let my_cx = self.center_x(me);
        let ot_cx = self.center_x(other);
        let d0 = my_cx.wrapping_sub(ot_cx);
        let facing_left = (self.eb(me, 0x37) as i8) >= 4;
        // Which half-width byte faces the other fighter depends on the side
        // and the facing (0x43dde / 0x43e1c).
        let (near_left, near_right) = if facing_left {
            (0x19, 0x18)
        } else {
            (0x18, 0x19)
        };
        let push = if (d0 as i16) > 0 {
            let w =
                (self.eb(me, near_right) as u16).wrapping_add(self.eb(other, near_right) as u16);
            let r = d0.wrapping_sub(w);
            if (r as i16) >= 0 {
                return;
            }
            r.wrapping_neg()
        } else {
            let w = (self.eb(me, near_left) as u16).wrapping_add(self.eb(other, near_left) as u16);
            let r = d0.wrapping_neg().wrapping_sub(w);
            if (r as i16) >= 0 {
                return;
            }
            r
        };
        self.shift_grid_x(me, push);
    }

    /// 0x43c18: vertical screen clamp, only for the hit-stun states 0x89 /
    /// 0x8a: keep the extents within y 0x10..=0xe8; a clamp also cancels the
    /// frame's steps (+0xb, +0x14, +0x15).
    fn clamp_y(&mut self, me: u16) {
        let st = self.eb(me, 9);
        if st != 0x89 && st != 0x8a {
            return;
        }
        let cy = self.center_y(me);
        let top = cy.wrapping_sub(self.eb(me, 0x16) as u16);
        let d0 = if top < 0x10 {
            top.wrapping_sub(0x10)
        } else {
            let bottom = (self.eb(me, 0x17) as u16).wrapping_add(cy);
            if bottom <= 0xe8 {
                return;
            }
            bottom - 0xe8
        };
        self.set_eb(me, 0xb, 0);
        self.set_eb(me, 0x14, 0);
        self.set_eb(me, 0x15, 0);
        self.shift_grid_y(me, d0.wrapping_neg());
    }

    /// 0x43c9a: horizontal screen clamp. A fighter whose near/far extent
    /// leaves the screen band is pulled back, and its frame step (+0x15) is
    /// left in +0x36 as a camera-scroll request (0x43d48).
    fn clamp_x(&mut self, me: u16) {
        if self.dramatic_rules().is_some() {
            return;
        }
        let rebound = self.sf3_wall_motion(me);
        if rebound != 0 {
            self.shift_grid_x(me, i16::from(rebound) as u16);
            self.set_eb(me, 0x15, 0);
        }
        let cx = self.center_x(me);
        let c0 = self.rb(SCENE_MODE) == 0xc0;
        let facing_left = (self.eb(me, 0x37) as i8) >= 4;
        let (front, back, right_lim, left_lim) = if facing_left {
            (
                0x19,
                0x18,
                if c0 { 0x16e } else { 0x1a0 },
                if c0 { 0xb0 } else { 0x80 },
            )
        } else {
            (
                0x18,
                0x19,
                if c0 { 0x150 } else { 0x180 },
                if c0 { 0xb0 } else { 0x60 },
            )
        };
        let right = (self.eb(me, front) as u16).wrapping_add(cx);
        let right_wall = right > right_lim;
        let d6 = if right_wall {
            right - right_lim
        } else {
            let left = cx.wrapping_sub(self.eb(me, back) as u16);
            if left >= left_lim {
                return;
            }
            left.wrapping_sub(left_lim)
        };
        if self.sf3_wall_contact(me, right_wall) {
            use crate::impact::{ImpactDirection, ImpactKind};
            let (x, direction) = if right_wall {
                (right_lim, ImpactDirection::Left)
            } else {
                (left_lim, ImpactDirection::Right)
            };
            self.record_surface_impact(me, ImpactKind::WallSlam, x, self.center_y(me), direction);
            self.set_eb(me, 0x15, 0);
        }
        self.set_eb(me, 0x36, self.eb(me, 0x15));
        self.shift_grid_x(me, d6.wrapping_neg());
    }

    /// 0x3b828: the frame-flag slot swap. A frame with flag 0x10 (+0xf)
    /// moves a fighter in object slot 0x400 into slot 0xa00, a frame with
    /// flag 0x20 (or 0x30) moves one in 0xa00 into 0x400; the two fighters'
    /// regions trade places (0x3b852). The original returns with D1 swapped,
    /// so the rest of the caller's frame runs on the fighter's new slot:
    /// the returned pair is (me, other) after the swap.
    fn effect_flag_spawn(&mut self, me: u16, other: u16) -> (u16, u16) {
        if self.dramatic_rules().is_some() {
            return (me, other);
        }
        let f = self.eb(me, 0xf) & 0x30;
        if f == 0 {
            return (me, other);
        }
        let from = if f == 0x10 { 0x400 } else { 0xa00 };
        if me != from {
            return (me, other);
        }
        self.swap_regions(me, other);
        (other, me)
    }

    /// 0x3b852..0x3b8ba: trade the two fighters' object regions through the
    /// 0x600-byte buffer at 0xff8712 (positions cleared first, 0x3b8ea),
    /// clearing the swap flags, and swap both player structures' entity
    /// words (+0xe: other / self). Each copy moves the larger of the
    /// source's grid and the first copy's grid (count at 0xff8710).
    /// Finally the positions of objects 0x80 and 0xc0 are cleared.
    fn swap_regions(&mut self, me: u16, other: u16) {
        const BUF: u32 = 0xff8712;
        const COUNT: u32 = 0xff8710;
        for d0 in (0..0x600).step_by(0x40) {
            self.wl(BUF + d0 + 4, 0);
        }
        self.set_eb(me, 0xf, self.eb(me, 0xf) & 0xcf);
        self.ww(COUNT, 0);
        let n = self.copy_grid(OBJ_BASE + me as u32, BUF, me);
        self.ww(COUNT, n);
        self.copy_grid(OBJ_BASE + other as u32, OBJ_BASE + me as u32, other);
        let ps = self.el(me, 0x38);
        self.wl(ps + 0xe, self.rl(ps + 0xe).rotate_left(16));
        self.copy_grid(BUF, OBJ_BASE + other as u32, other);
        let ps = self.el(other, 0x38);
        self.wl(ps + 0xe, self.rl(ps + 0xe).rotate_left(16));
        self.wl(OBJ_BASE + 0x84, 0);
        self.wl(OBJ_BASE + 0xc4, 0);
    }

    /// 0x3b8bc: copy longs from `src` to `dst`: the grid of the entity at
    /// `sizer` ((rows) x (cols) entries of 0x10 longs), or the count at
    /// 0xff8710 if larger. Returns the grid's own count minus one (D2).
    fn copy_grid(&mut self, src: u32, dst: u32, sizer: u16) -> u16 {
        let rows = self.eb(sizer, 0xc).wrapping_add(1) as u16;
        let cols = self.eb(sizer, 0xd).wrapping_add(1) as u16;
        let d2 = rows.wrapping_mul(cols).wrapping_mul(0x10).wrapping_sub(1);
        let d3 = d2.max(self.rw(0xff8710));
        for i in 0..=d3 as u32 {
            let v = self.rl(src + 4 * i);
            self.wl(dst + 4 * i, v);
        }
        d2
    }

    // ---- collision and hit handling ---------------------------------------

    fn judge_targets(&mut self, me: u16, other: u16) {
        if self.eb(me, 0xf) & 0x80 != 0 {
            self.era_mark_attack_active(self.el(me, 0x38));
        }
        if let Some(rules) = self.dramatic_rules() {
            let owner = self.el(me, 0x38);
            if self.eb(me, 0xf) & 0x80 == 0 || !self.dramatic_alive(owner) {
                return;
            }
            let primary = self.el(me, 0x3c);
            let mut connected = false;
            for defender in self.dramatic_hit_targets(owner, rules.friendly_fire) {
                if !self.dramatic_alive(defender) {
                    continue;
                }
                self.bind_dramatic_target(owner, defender);
                self.set_eb(me, 0xf, self.eb(me, 0xf) | 0x80);
                self.judge(me, self.rw(defender + 0x10));
                connected |= self.eb(me, 0xf) & 0x80 == 0;
            }
            if connected {
                self.set_eb(me, 0xf, self.eb(me, 0xf) & 0x7f);
            }
            self.bind_dramatic_target(owner, primary);
        } else {
            self.judge(me, other);
        }
    }

    /// 0x43e82: when the attacker's frame is active (+0xf bit 7) and the
    /// defender is not invulnerable (+0xf bit 3), test the attack box
    /// (+0x24/+0x25 x, +0x22/+0x23 y) against the defender's loose box
    /// (+0x20/+0x21, +0x1e/+0x1f) and tight box (+0x1c/+0x1d, +0x1a/+0x1b);
    /// a hit calls the frame's hit function (+0x28) with depth 1 (inside the
    /// tight box) or 2 (loose only).
    fn judge(&mut self, me: u16, other: u16) {
        use crate::era_fight::what_if_others::{GREEDY_TIGER, MIGHTY_TIGER, NEXUS, OD_NEXUS};

        let owner = self.el(me, 0x38);
        let move_id = self.eb(me, 0xa);
        let alpha = self.era_state(owner).filter(|era| era.alpha_move_active(move_id));
        let alpha_move = alpha.is_some();
        if alpha.is_some_and(|era| era.is_alpha()
            && era.fighter == Some(crate::roster::TournamentFighter::Gen)
            && move_id == 0x2b)
            && !self.is_jump_airborne(other) && !self.airborne_reaction(other)
            && !matches!(self.eb(other, 0xa), 0x20..=0x26)
        {
            return;
        }
        if move_id == GREEDY_TIGER && alpha.is_some_and(|era| era.what_if_sagat_slot(move_id))
            && self.eb(other, 9) & 4 != 0
        {
            return;
        }
        let era = self.sf2_state(owner).or(alpha);
        let hit_limit = era.map_or(1, |era| {
            if matches!(move_id, NEXUS | OD_NEXUS) && era.what_if_sagat_slot(move_id) {
                2
            } else {
                era.move_hit_limit(move_id)
            }
        });
        // Misogi can catch an airborne opponent from a confirmed Shoryuken.
        // Its dive and landing still count as one hit.
        let juggle_limit = if move_id == KAGE_MISOGI && alpha_move && alpha.is_some_and(|state| state.is_kage()) {
            4
        } else {
            hit_limit
        };
        let juggle = self.era_can_juggle(other, juggle_limit);
        let alpha_rehit = alpha.is_some_and(|era| era.move_hits != 0 && era.move_hits < hit_limit)
            && self.eb(other, 8) == 2
            && (0x30..=0x3f).contains(&self.eb(other, 0xa))
            && self.eb(other, 0xa) != crate::combo::throw::THROWN_MOVE
            && self.eb(other, 9) & 1 == 0;
        let custom = self.alpha_state(owner).is_some_and(|era| era.custom_combo_active());
        let can_rehit = juggle || alpha_rehit || (custom && self.era_can_juggle(other, 8));
        if self.eb(me, 0xf) & 0x80 == 0
            || ((self.eb(other, 0xf) & 0x08 != 0 || (era.is_some() && self.airborne_reaction(other)))
                && !can_rehit)
        {
            return;
        }
        let (ax, ay) = self.contact_position(me);
        let aw = self.eb(me, 0x25) as u16;
        let ah = self.eb(me, 0x23) as u16;
        let dcx = self.center_x(other);
        let dcy = self.center_y(other);
        let apart = |p: u16, q: u16, reach: u16| {
            let d = p.wrapping_sub(q) as i16;
            (d.unsigned_abs() as i16) > reach as i16
        };
        let body = self.era_juggle_body(other, can_rehit);
        let dx_loose = body.map_or_else(|| dcx.wrapping_add(sext8(self.eb(other, 0x20))), |_| dcx);
        let dy_loose = body.map_or_else(|| dcy.wrapping_sub(sext8(self.eb(other, 0x1e))), |_| dcy);
        let (loose_w, loose_h) = body.unwrap_or_else(|| (u16::from(self.eb(other, 0x21)), u16::from(self.eb(other, 0x1f))));
        if apart(dx_loose, ax, aw.wrapping_add(loose_w)) {
            return;
        }
        if apart(dy_loose, ay, ah.wrapping_add(loose_h)) {
            return;
        }
        let dx_tight = body.map_or_else(|| dcx.wrapping_add(sext8(self.eb(other, 0x1c))), |_| dcx);
        let dy_tight = body.map_or_else(|| dcy.wrapping_sub(sext8(self.eb(other, 0x1a))), |_| dcy);
        let (tight_w, tight_h) = body.map_or_else(
            || (u16::from(self.eb(other, 0x1d)), u16::from(self.eb(other, 0x1b))),
            |(w, h)| (w / 2, h / 2),
        );
        let depth = if apart(dx_tight, ax, aw.wrapping_add(tight_w))
            || apart(dy_tight, ay, ah.wrapping_add(tight_h))
        {
            2
        } else {
            1
        };
        let air_move = alpha.is_some_and(|era| era.alpha_move_airborne(move_id) || era.sf4_airborne_move(move_id) || era.what_if_airborne(move_id));
        let overhead = alpha.is_some_and(|era| era.alpha_move_overhead(move_id) || era.sf3_overhead(move_id)
            || era.sf4_ex_axe_kick(move_id) || era.sf6_jinrai_overhead(move_id)
            || era.sf6_evil_ryu_od_axe_kick(move_id) || era.what_if_overhead(move_id)
            || move_id == MIGHTY_TIGER && era.what_if_sagat_slot(move_id));
        let f = self.el(me, 0x28);
        let hit_function = hit_function(f);
        let low = matches!(hit_function, Some(
            HitFn::Basic { low: true, .. } | HitFn::Strength { low: true, .. }
            | HitFn::Cpu { block_state: 0x24, .. }
        ));
        let defender = self.el(other, 0x38);
        let parried = self.what_if_eagle_riposte(other, self.center_x(me))
            || self.sf3_parry(other, self.center_x(me), low, false, self.era_airborne(other));
        if parried || self.sf5_defend(defender, other, false) || self.sf6_defend(defender) {
            self.set_eb(me, 0xf, self.eb(me, 0xf) & 0x7f);
            let attacker_slot = self.combo_slot(me);
            if hit_limit <= 1 {
                self.toggle_state.combo.connected[attacker_slot] = true;
                if parried {
                    self.set_eb(me, 0x33, self.eb(me, 0x33).max(8));
                }
            }
            if parried {
                let defender_slot = self.combo_slot(other);
                self.toggle_state.combo.blockstun[defender_slot] = 0;
            }
            return;
        }
        match hit_function {
            Some(_) if self.eb(me, 9) & 3 == 1 && self.alpha_air_blocks(other, self.center_x(me)) => {
                self.block_contact(other);
                self.combo_blocked(me, other);
                self.set_eb(me, 0xf, self.eb(me, 0xf) & 0x7f);
            }
            Some(HitFn::Basic {
                template,
                table,
                low,
            }) => self.hit_basic(me, other, depth, template, table, if low { 8 } else { 4 }),
            Some(HitFn::Shoryuken { template }) => {
                if self.toggles.balanced_roster && hit_limit == 1 {
                    let slot = self.player_slot_index(self.el(me, 0x38)).unwrap_or(1);
                    self.toggle_state.dp_connected[slot] = true;
                }
                if self.toggles.balanced_roster
                    && (self.eb(other, 9) & 0x20 != 0 || matches!(self.eb(other, 0xa), 4 | 8))
                {
                    self.queue_sound(8);
                    self.training_contact(other);
                    self.combo_blocked(me, other);
                } else {
                    self.land_hit_entry(me, other, template, 0x45f52);
                }
                self.set_eb(me, 0xf, self.eb(me, 0xf) & 0x7f);
            }
            Some(HitFn::Tatsumaki { template }) => self.hit_tatsumaki(me, other, template),
            Some(HitFn::Cpu {
                template,
                depth1,
                other: entry_other,
                block_state,
            }) => {
                if self.cpu_attack_is_blocked(me, other, block_state) {
                    self.queue_sound(8);
                    self.training_contact(other);
                    self.combo_blocked(me, other);
                } else {
                    let entry = if depth == 1 { depth1 } else { entry_other };
                    let raw = self.rb(entry + 1);
                    let dmg = if self.toggles.balanced_roster {
                        match crate::balanced_roster::cpu_damage(f) {
                            Some((crit, rest)) => if depth == 1 { crit } else { rest },
                            None => raw.min(crate::balanced_roster::CPU_DAMAGE_CAP),
                        }
                    } else {
                        raw
                    };
                    // A player's boss on his bank table scores as Ryu's
                    // matching attack would (the CPU's hits never score).
                    if let Some(scored) = self.player_score_entry(me, depth) {
                        self.add_score(me, self.rb(scored));
                        let ps = self.el(me, 0x38);
                        self.show_score(ps);
                    }
                    self.damage(me, dmg);
                    self.react(me, other, entry);
                    if self.player_score_entry(me, depth).is_some() {
                        self.check_hiscore(me);
                    }
                    self.spawn_effect(me, template);
                }
                self.set_eb(me, 0xf, self.eb(me, 0xf) & 0x7f);
            }
            Some(HitFn::Strength { spark, table, low }) => {
                let template = match spark {
                    Spark::Fixed(t) => t,
                    Spark::ByLevel(t, above) => {
                        if self.eb(me, 0x34) > 3 {
                            above
                        } else {
                            t
                        }
                    }
                    Spark::ByMove(t, other_move) => {
                        if self.eb(me, 0xa) == 0x20 {
                            t
                        } else {
                            other_move
                        }
                    }
                };
                self.hit_strength(me, other, depth, template, table, if low { 8 } else { 4 })
            }
            Some(HitFn::Redirect {
                template,
                table,
                blocked,
            }) => self.hit_redirect(me, other, depth, template, table, blocked),
            None if (move_id == crate::data_bank::ALPHA_COUNTER_SLOT || alpha_move)
                && crate::data_bank::in_bank(f) =>
            {
                let airborne = self.eb(me, 9) & 1 != 0
                    || air_move
                    || (alpha_move && self.eb(me, 9) == 0xc2 && self.eb(me, 0x14) != 0);
                let air_block = airborne && self.alpha_air_blocks(other, self.center_x(me));
                let guard = if overhead {
                    self.eb(other, 9) == 0x22 || self.eb(other, 0xa) == 4
                } else {
                    self.eb(other, 9) & 0x20 != 0 || matches!(self.eb(other, 0xa), 4 | 8)
                };
                if (air_block || guard) && !self.era_sf4_focus_unblockable(owner, move_id) {
                    if self.era_state(defender).is_some_and(|state| state.sf6_burnout()) {
                        let chip = self.rb(f + 1).div_ceil(4);
                        self.damage_with_chip(me, chip, true);
                    }
                    self.block_contact(other);
                    self.combo_blocked(me, other);
                } else {
                    self.land_hit_entry(me, other, 0x1853c, f);
                }
                self.set_eb(me, 0xf, self.eb(me, 0xf) & 0x7f);
            }
            None => self.note_unrecovered(f),
        }
        // True combos (SP17): every hit function clears the attacker's
        // active bit, hit or guarded; the move has connected, so its later
        // active frames no longer hit (`combo_load_frame`).
        if (self.era_true_combos(owner) || alpha_move) && self.eb(me, 0xf) & 0x80 == 0 {
            let s = self.combo_slot(me);
            let move_hits = self.era_side(owner).filter(|_| hit_limit > 1).map(|side| {
                let count = &mut self.toggle_state.era.fighters[side].move_hits;
                *count = count.saturating_add(1);
                *count
            });
            self.toggle_state.combo.connected[s] = move_hits.is_none_or(|hits| hits >= hit_limit);
        }
        // Cancels (SP18): a cancelable normal that connected, hit or
        // guarded, opens its cancel window.
        if self.era_offense_flag(owner, self.toggles.cancels) && self.eb(me, 0xf) & 0x80 == 0 {
            self.cancel_open(me);
        }
    }

    /// The basic hit shape (0x45af0, 0x45c0c, 0x45c98, 0x4602a and their
    /// copies). A defender blocking high (+0xa == 4, the standing block
    /// move) only gets the block sound; otherwise the damage entry at
    /// `table` + (depth & 1) * 8 supplies the score index, damage, and
    /// reaction moves, and a spark spawns from `template`. The attacker's
    /// active bit clears either way (one hit per active window).
    fn hit_basic(&mut self, me: u16, other: u16, depth: u8, template: u32, table: u32, block: u8) {
        let is_blocked = if self.toggles.standard_high_low_block && block == 4 {
            matches!(self.eb(other, 0xa), 4 | 8)
        } else {
            self.eb(other, 0xa) == block
        };
        if is_blocked {
            self.queue_sound(8);
            self.training_contact(other);
            self.combo_blocked(me, other);
        } else {
            self.land_hit(me, other, depth, template, table);
        }
        self.set_eb(me, 0xf, self.eb(me, 0xf) & 0x7f);
    }

    /// The strength-indexed shape (0x45d26 crouching punches, 0x46250
    /// crouching kick, 0x45e10 jump punches, 0x4638e.. jump kicks, and their
    /// copies): blocked by the `block` move (8 crouching for the low
    /// attacks, 4 standing for the jump attacks); the damage entry is
    /// `table` + (strength - 1) * 16, + 8 for an odd depth.
    fn hit_strength(
        &mut self,
        me: u16,
        other: u16,
        depth: u8,
        template: u32,
        table: u32,
        block: u8,
    ) {
        if self.eb(other, 0xa) == block {
            self.queue_sound(8);
            self.training_contact(other);
            self.combo_blocked(me, other);
        } else {
            let level = (sext8(self.eb(me, 0x34)) as i16)
                .wrapping_sub(1)
                .wrapping_shl(4);
            let mut entry = (table as i32 + level as i32) as u32;
            if depth & 1 != 0 {
                entry += 8;
            }
            self.land_hit_entry(me, other, template, entry);
        }
        self.set_eb(me, 0xf, self.eb(me, 0xf) & 0x7f);
    }

    /// 0x45f5a / 0x45f64: the Tatsumaki's hit. The defender's state +9 &
    /// 0x28 decides: 0x08 (in hit stun) takes no hit; 0x20 (the standing or
    /// crouching block moves) takes chip damage — entry 0x4601a in 2P
    /// versus (game mode 1), otherwise the entry at 0x46012 + the stage's
    /// byte of the table at 0x45fa8 (8, 8, 16, 16, 0, 0xff by 0xffc003; 0
    /// gives only the block sound); anything else takes the full hit, entry
    /// 0x46012.
    fn hit_tatsumaki(&mut self, me: u16, other: u16, template: u32) {
        match self.eb(other, 9) & 0x28 {
            0x08 => {}
            0x20 => {
                if self.toggles.balanced_roster {
                    let slot = self.player_slot_index(self.el(me, 0x38)).unwrap_or(1);
                    self.toggle_state.dp_connected[slot] = true;
                    self.land_hit_entry_with_chip(
                        me,
                        other,
                        template,
                        0x4601a,
                        true,
                        HitSource::Chip,
                    );
                } else if self.rb(GAME_MODE) == 1 {
                    self.land_hit_entry_with_chip(
                        me,
                        other,
                        template,
                        0x4601a,
                        false,
                        HitSource::Chip,
                    );
                } else {
                    // ext.w of the stage byte, then its low byte replaced by
                    // the table's byte (move.b keeps the high byte).
                    let stage = sext8(self.rb(0xffc003));
                    let d0 = (stage & 0xff00)
                        | self.rb((0x45fa8i32 + stage as i16 as i32) as u32) as u16;
                    if d0 & 0xff == 0 {
                        self.queue_sound(8);
                        self.training_contact(other);
                        self.guard_power_contact(me, other, GUARD_SPECIAL_PRESSURE);
                    } else {
                        self.land_hit_entry_with_chip(
                            me,
                            other,
                            template,
                            (0x46012i32 + d0 as i16 as i32) as u32,
                            false,
                            HitSource::Chip,
                        );
                    }
                }
            }
            _ => {
                if self.toggles.balanced_roster {
                    let slot = self.player_slot_index(self.el(me, 0x38)).unwrap_or(1);
                    self.toggle_state.dp_connected[slot] = true;
                }
                self.land_hit_entry(me, other, template, 0x46012);
            }
        }
        self.set_eb(me, 0xf, self.eb(me, 0xf) & 0x7f);
    }

    /// The blocked-kick shape (0x460b8 medium kick, 0x46164 / 0x46184 heavy
    /// kick, and their copies): as [`Self::hit_basic`], but a high block
    /// also sends the attacker's stream (+0x2c) to `blocked`, its recoil.
    fn hit_redirect(
        &mut self,
        me: u16,
        other: u16,
        depth: u8,
        template: u32,
        table: u32,
        blocked: u32,
    ) {
        let is_blocked = if self.toggles.standard_high_low_block {
            matches!(self.eb(other, 0xa), 4 | 8)
        } else {
            self.eb(other, 0xa) == 4
        };
        if is_blocked {
            self.set_el(me, 0x2c, blocked);
            self.queue_sound(8);
            self.training_contact(other);
            self.combo_blocked(me, other);
        } else {
            self.land_hit(me, other, depth, template, table);
        }
        self.set_eb(me, 0xf, self.eb(me, 0xf) & 0x7f);
    }

    /// The landed-hit sequence every hit shape shares: score (0x3b9f0) and
    /// its redraw (0x3be6a), damage (0x3c034), the defender's reaction
    /// (0x4451a), the top-score check (0x3ba84), and the spark (0x42e1e).
    fn land_hit(&mut self, me: u16, other: u16, depth: u8, template: u32, table: u32) {
        self.land_hit_entry(me, other, template, table + (depth as u32 & 1) * 8);
    }

    /// [`Self::land_hit`] with the damage entry already chosen.
    fn land_hit_entry(&mut self, me: u16, other: u16, template: u32, entry: u32) {
        self.land_hit_entry_with_chip(me, other, template, entry, false, HitSource::Melee);
    }

    /// `is_chip` is Balanced roster's chip rule for the damage; `src` what
    /// the true-combo gate makes of the reaction (a Tatsumaki through a
    /// guard is chip to it, whichever damage entry the 1987 code takes).
    fn land_hit_entry_with_chip(
        &mut self,
        me: u16,
        other: u16,
        template: u32,
        entry: u32,
        is_chip: bool,
        src: HitSource,
    ) {
        self.add_score(me, self.rb(entry));
        let ps = self.el(me, 0x38);
        self.show_score(ps);
        let defender = self.el(other, 0x38);
        let metsu = self.evil_ryu_profile(ps) == Some(EvilRyuPlaytest::UltraSf2)
            && entry == 0x45f52 && self.el(me, 0x28) == 0x45efa;
        let burnout = self.era_state(defender).is_some_and(|state| state.sf6_burnout());
        let dmg = if metsu {
            14
        } else if burnout && src == HitSource::Chip {
            self.effective_entry_damage(me, 0x46012, false).div_ceil(4)
        } else {
            self.effective_entry_damage(me, entry, is_chip)
        };
        // A Tatsumaki's chip outside Balanced roster passes as chip too: only
        // Balanced roster's chip rule (whose branches never reach here
        // with it off) and the combo scaling read the flag.
        let chip = is_chip || src == HitSource::Chip;
        let punish = !chip && self.sf6_impact_punish(ps, defender, self.eb(me, 0xa));
        let crush = self.sf5_strike(me, other, self.eb(me, 0xa), chip);
        self.damage_with_chip(me, dmg, chip);
        self.react_as(me, other, if punish { 0x45b6e } else if crush { 0x45f52 } else { entry }, src);
        if crush {
            let slot = self.combo_slot(other);
            let stun = &mut self.toggle_state.combo.hitstun[slot];
            *stun = stun.saturating_add(10);
        }
        self.check_hiscore(me);
        self.spawn_effect(me, if metsu { 0x186fc } else { template });
    }

    /// 0x3b9f0: add the 7-digit decimal score at 0x146d6 + idx to the
    /// attacker's score (player +0x22..+0x28), outside demo play only.
    fn add_score(&mut self, me: u16, idx: u8) {
        let ps = self.score_player(self.el(me, 0x38));
        if self.rb(MODE_DEMO) == 0 || ps == CPU_STRUCT {
            return;
        }
        self.decimal_add(ps + 0x29, 0x146d6 + 7 + idx as u32, 7);
    }

    /// 0x3ba0e (`digits == 7`) / 0x3ba12: add the `digits`-digit decimal number
    /// ending just before `src` onto the one ending just before `dst`, one
    /// digit per byte, propagating the decimal carry (X flag) leftwards.
    pub(crate) fn decimal_add(&mut self, dst: u32, src: u32, digits: u32) {
        let (mut a0, mut a1, mut carry) = (dst, src, 0u8);
        for _ in 0..digits {
            a0 -= 1;
            a1 -= 1;
            let mut d = self.rb(a0).wrapping_add(self.rb(a1)).wrapping_add(carry);
            if (d as i8) >= 10 {
                d = d.wrapping_sub(10);
                carry = 1;
            } else {
                carry = 0;
            }
            self.wb(a0, d);
        }
    }

    /// Write `n + 1` digit bytes from `src` as text cells `0x8000 | digit`
    /// starting at video offset `cell` (0x3be0c).
    pub(crate) fn show_digits(&mut self, src: u32, cell: u32, n: u32) {
        for i in 0..=n {
            let v = 0x8000 | self.rb(src + i) as u16;
            self.ww(0x80_0000 + cell + 2 * i, v);
        }
    }

    /// 0x3be6a: redraw a player's score (8 cells at player +0x20).
    pub(crate) fn show_score(&mut self, ps: u32) {
        let ps = self.score_player(ps);
        if ps == CPU_STRUCT {
            return;
        }
        let cell = self.rw(ps + 0x20) as u32;
        self.show_digits(ps + 0x22, cell, 7);
    }

    fn score_player(&self, ps: u32) -> u32 {
        if ps == P2_STRUCT && self.dramatic_rules().is_some() {
            P1_STRUCT
        } else {
            ps
        }
    }

    /// 0x3c034: drain the defender's health (player +6 remainder, +7 cells;
    /// 4 points per cell), redraw the active cell from the tile table at
    /// 0x3c0a6 (indexed by the remainder) and fill the lost cells.
    fn damage(&mut self, me: u16, dmg: u8) {
        self.damage_with_chip(me, dmg, false);
    }

    /// [`Self::damage`] for every hit: normals, specials, projectiles, chip
    /// and the CPU's and bosses' hits all drain health here. In Training,
    /// under every preset, the drain stops at the health floor, one point
    /// left (ADR-0013): the hit lands, the reaction ([`Self::react`]) finds
    /// the fighter alive, and nothing that follows only a knock-out runs.
    /// Training's hit record takes the full damage first
    /// ([`Self::training_hit`]).
    fn damage_with_chip(&mut self, me: u16, dmg: u8, is_chip: bool) {
        self.damage_with_source(me, dmg, is_chip, None);
    }

    fn damage_with_source(
        &mut self,
        me: u16,
        mut dmg: u8,
        is_chip: bool,
        projectile: Option<crate::era_fight::EraProjectileState>,
    ) {
        let owner = self.el(me, 0x38);
        let modern_scaled = projectile.map_or_else(|| self.modern_scaled_move(owner), |source| source.modern_scaled);
        let projectile = projectile.filter(|source| source.hits_remaining > 0);
        let ps = self.el(me, 0x3c);
        let suppress_chip = is_chip && self.era_state(ps).is_some_and(|state| state.is_sf6() && state.gauge().is_some() && !state.sf6_burnout());
        let demon_slam = self.evil_ryu_profile(owner).is_some()
            && self.toggle_state.raging_demon.active && projectile.is_none()
            && self.eb(me, 0xa) == crate::combo::throw::THROW_MOVE;
        let alpha = self.era_state(owner);
        let move_id = projectile.map_or_else(|| self.eb(me, 0xa), |source| source.move_id);
        if !demon_slam && self.era_true_combos(owner) && !is_chip {
            // Airborne reactions use their own clock instead of ground hitstun.
            let air_juggle = projectile.is_none() && move_id == KAGE_MISOGI
                && alpha.is_some_and(|state| state.is_kage() && state.alpha_move_active(move_id))
                && self.airborne_reaction(self.rw(ps + 0x10));
            dmg = self.combo_scale(ps, dmg, air_juggle);
        }
        let sf2 = self.sf2_state(owner).copied();
        let alpha = self.era_state(owner);
        let alpha_level = projectile.map_or_else(
            || alpha.map_or(0, |era| era.alpha_super_level(move_id)),
            |source| source.alpha_super_level,
        );
        let mut damage = u16::from(dmg) * crate::era_fight::SUB_HP_PER_POINT;
        if let Some(era) = sf2.filter(|_| !demon_slam) {
            let delta = crate::era_fight::RevisionDelta::of(era.style);
            if !is_chip {
                if move_id == crate::era_fight::SF2_SUPER {
                    damage = if projectile.is_some() { 56 } else { 95 };
                } else if projectile.is_some() && move_id == crate::era_fight::SF2_EXTRA_SPECIAL {
                    damage = 110;
                }
            }
            if let Some(delta) = delta {
                damage = (u32::from(damage) * u32::from(delta.damage_percent) / 100) as u16;
            }
        }
        if !is_chip && projectile.is_some_and(|source| source.family_hits != 0) {
            damage = if alpha_level != 0 { 60 } else { 70 };
        } else if alpha_level != 0 && !is_chip {
            use crate::roster::TournamentFighter::{Ken, Ryu, Sagat};
            match (alpha.and_then(|era| era.fighter), move_id, projectile.is_some()) {
                (Some(Ryu), crate::era_fight::SF2_SUPER, true)
                | (Some(Sagat), crate::data_bank::ALPHA_SUPER_SECOND_SLOT, true) => damage = 60,
                (Some(Ken), crate::era_fight::SF2_SUPER, false) => damage = 76,
                _ => {}
            }
            let percent = 90 + 10 * u32::from(alpha_level);
            damage = (u32::from(damage) * percent / 100) as u16;
        }
        if !is_chip && projectile.is_none() && move_id == crate::era_fight::SF2_EXTRA_SPECIAL
            && alpha.is_some_and(|era| era.charged_tiger_blow)
        {
            damage += damage / 2;
        }
        let custom = alpha.is_some_and(|era| era.custom_combo_active() || era.gen_genei_jin());
        if custom {
            damage /= 2;
        }
        let x_attack = alpha.is_some_and(|era| era.is_alpha_x_ism());
        let x_defense = self.alpha_state(ps).is_some_and(|era| era.is_alpha_x_ism());
        if x_attack || x_defense {
            let outgoing = if x_attack { 112 } else { 100 };
            let incoming = if x_defense { 115 } else { 100 };
            damage = ((u32::from(damage) * outgoing * incoming + 5000) / 10000) as u16;
        }
        if !is_chip && (custom || sf2.is_some() || alpha_level != 0
            || alpha.is_some_and(|era| era.alpha_move_active(move_id) && era.move_hit_limit(move_id) > 1))
        {
            let defender = self.rw(ps + 0x10);
            if self.eb(defender, 9) & 3 == 1 {
                if let Some(side) = self.era_side(ps) {
                    let juggle = &mut self.toggle_state.era.fighters[side].juggle_count;
                    *juggle = juggle.saturating_add(1);
                }
            }
        }
        let Some(adjusted) = self.era_sf4_damage(owner, ps, move_id, damage, is_chip, projectile) else { return };
        let armor_break = alpha_level != 0 || (sf2.is_some() && move_id == crate::era_fight::SF2_SUPER);
        let Some(adjusted) = self.sf6_damage(owner, ps, move_id, adjusted, is_chip, armor_break) else { return };
        damage = self.sf5_damage(owner, ps, move_id, if suppress_chip { 0 } else { adjusted }, is_chip);
        if modern_scaled {
            damage = crate::control::shortcut_damage(damage);
            // The native fallback still drains the cabinet's integral HP
            // instead of the overhaul's sub-HP value.
            dmg = crate::control::shortcut_damage(u16::from(dmg)) as u8;
        }
        if !is_chip {
            self.era_alpha_damage_effects(owner, ps, move_id, alpha_level);
        }
        let record = damage.div_ceil(crate::era_fight::SUB_HP_PER_POINT).min(u16::from(u8::MAX)) as u8;
        self.training_hit(ps, record);
        if self.dramatic_rules().is_some() {
            let damage = self.dramatic_scale_damage(owner, damage);
            self.damage_dramatic(ps, damage, is_chip);
            return;
        }
        let prevent_chip_ko = self.toggles.balanced_roster && is_chip && !self.toggles.chip_death;
        if self.era_damage(
            ps,
            damage,
            prevent_chip_ko || self.roster.mode == PlayMode::Training,
        ) {
            return;
        }
        if self.toggles.balanced_roster && is_chip && !self.toggles.chip_death {
            let cur_hp = (self.rb(ps + 7) as u16) * 4 + (self.rb(ps + 6) as u16);
            if cur_hp >= 1 {
                dmg = (cur_hp - 1).min(dmg as u16) as u8;
            }
            if dmg == 0 {
                return;
            }
        }
        if self.roster.mode == PlayMode::Training {
            let cur_hp = (self.rb(ps + 7) as u16) * 4 + (self.rb(ps + 6) as u16);
            dmg = cur_hp.saturating_sub(TRAINING_HEALTH_FLOOR).min(dmg as u16) as u8;
        }
        self.drain_native_health(ps, dmg);
    }

    fn era_can_juggle(&self, defender: u16, limit: u8) -> bool {
        limit > 1
            && self.airborne_reaction(defender)
            && self.era_side(self.el(defender, 0x38))
                .is_some_and(|side| self.toggle_state.era.fighters[side].juggle_count < limit)
    }

    fn airborne_reaction(&self, entity: u16) -> bool {
        self.eb(entity, 8) == 2
            && self.eb(entity, 9) & 3 == 1
            && (0x30..=0x3f).contains(&self.eb(entity, 0xa))
    }

    /// Airborne 1987 reactions omit their boxes because only the newer
    /// rules can hit during them. Bound the fallback by the current pose.
    fn era_juggle_body(&self, entity: u16, juggle: bool) -> Option<(u16, u16)> {
        if !juggle || [0x1b, 0x1d, 0x1f, 0x21].into_iter().any(|offset| self.eb(entity, offset) != 0) {
            return None;
        }
        let big = self.is_big(entity);
        Some((
            Self::half_extent(self.eb(entity, 0xd), big).min(18),
            Self::half_extent(self.eb(entity, 0xc), big).min(26),
        ))
    }

    pub(crate) fn drain_native_health(&mut self, ps: u32, dmg: u8) {
        loop {
            if dmg <= self.rb(ps + 6) {
                self.wb(ps + 6, self.rb(ps + 6) - dmg);
                break;
            }
            if self.rb(ps + 7) == 0 {
                self.wb(ps + 6, 0);
                break;
            }
            self.wb(ps + 7, self.rb(ps + 7) - 1);
            self.wb(ps + 6, self.rb(ps + 6).wrapping_add(4));
        }
        self.draw_health(ps);
    }

    pub(crate) fn dramatic_scale_damage(&self, owner: u32, damage: u16) -> u16 {
        if owner == CPU_STRUCT && self.dramatic_rules().is_some() {
            damage.saturating_add(damage / 4)
        } else {
            damage
        }
    }

    fn damage_dramatic(&mut self, defender: u32, damage: u16, chip: bool) {
        let Some(slot) = self.player_slot_index(defender) else { return };
        let hp = self.toggle_state.dramatic.health[slot];
        if hp == 0 {
            return;
        }
        let floor = u16::from(chip && self.toggles.balanced_roster && !self.toggles.chip_death);
        self.dramatic_set_health(defender, hp.saturating_sub(damage).max(floor));
    }

    pub(crate) fn dramatic_set_health(&mut self, defender: u32, hp: u16) {
        use crate::dramatic::{ALLY_SUB_HP, BOSS_SUB_HP, LifebarMode};

        let Some(rules) = self.dramatic_rules() else { return };
        let Some(slot) = self.player_slot_index(defender) else { return };
        let health = &mut self.toggle_state.dramatic.health;
        health[slot] = hp;
        if slot != 1 && rules.lifebar == LifebarMode::Shared {
            health[0] = hp;
            health[2] = hp;
        }
        for (state, hp) in self.toggle_state.era.fighters.iter_mut().zip(*health) {
            state.sub_hp = hp;
        }

        let [ally1, boss, ally2] = *health;
        for (ps, hp, max) in [
            (P1_STRUCT, ally1, ALLY_SUB_HP),
            (CPU_STRUCT, boss, BOSS_SUB_HP),
            (P2_STRUCT, ally2, ALLY_SUB_HP),
        ] {
            let points = (u32::from(hp) * 48).div_ceil(u32::from(max));
            let cells = points.saturating_sub(1) / 4;
            let remainder = points - cells * 4;
            self.wb(ps + 6, remainder as u8);
            self.wb(ps + 7, cells as u8);
            if ps != P2_STRUCT {
                self.draw_health(ps);
            }
        }
        if ally1 == 0 && ally2 != 0 {
            self.wb(P1_STRUCT + 6, 1);
        }
    }

    fn defender_alive(&self, ps: u32) -> bool {
        if self.dramatic_rules().is_some() {
            self.dramatic_alive(ps)
        } else {
            self.rb(ps + 7) | self.rb(ps + 6) != 0
        }
    }

    pub(crate) fn draw_health(&mut self, ps: u32) {
        let tile = self.rw(0x3c0a6 + 2 * self.rb(ps + 6) as u32);
        let cells = self.rb(ps + 7) as u16;
        let mut a1 = 0x80_0000 + (cells * 2).wrapping_add(self.rw(ps + 0xa)) as u32;
        self.ww(a1, tile);
        a1 += 2;
        let fill = self.rw(0x3c0a6);
        if cells != 0xb {
            let n = (cells.wrapping_sub(0xb)).wrapping_neg();
            for _ in 0..n {
                self.ww(a1, fill);
                a1 += 2;
            }
        }
    }

    /// 0x4451a: the defender's reaction. Turn to face the attacker, pick the
    /// reaction move from the damage entry by the defender's state (+9 & 7:
    /// 4, 1, or anything else — standing Ken has 2), start its stream from the
    /// per-player reaction table (alive) or knock-down table (no health
    /// left), queue the table's sound, and cut the current hold short.
    /// Every landed hit, chip included, comes here, so in Training it also
    /// starts the defender's refill wait over ([`Self::training_contact`]).
    fn react(&mut self, me: u16, other: u16, entry: u32) {
        self.react_as(me, other, entry, HitSource::Melee);
    }

    /// [`Self::react`] for a hit landed by `src`, which the true-combo gate
    /// reads at the end ([`Self::combo_react`]).
    fn react_as(&mut self, me: u16, other: u16, entry: u32, src: HitSource) {
        let target = self.el(other, 0x38);
        if self.era_sf4_take_absorbed(target) || self.sf5_take_absorbed(target) || self.sf6_take_absorbed(target) {
            return;
        }
        if src == HitSource::Chip && self.guard_power_contact(me, other, GUARD_SPECIAL_PRESSURE) {
            return;
        }
        if src == HitSource::Melee {
            self.record_heavy_strike(me, other);
            self.era_normal_contact(me, false);
        }
        self.react_one(me, other, entry, src);
        if src == HitSource::Melee {
            let owner = self.el(me, 0x38);
            let move_id = self.eb(me, 0xa);
            self.sf3_mark_wall_launch(owner, target, move_id);
            let grounded_alive = !self.era_airborne(other) && self.defender_alive(target);
            self.era_crumple_hit(owner, target, move_id, grounded_alive);
        }
        self.dramatic_shared_knockdown(me, other, entry, src);
    }

    fn react_projectile(&mut self, me: u16, other: u16, entry: u32, blocked: bool, air_block: bool) {
        let defender = self.el(other, 0x38);
        if self.era_sf4_take_absorbed(defender) || self.sf5_take_absorbed(defender) || self.sf6_take_absorbed(defender) {
            return;
        }
        if blocked {
            self.record_guard_contact(other);
        }
        if blocked && self.era_state(defender).is_some_and(|state| state.is_sf6()) && self.defender_alive(defender) {
            let burnout = self.era_state(defender).is_some_and(|state| state.sf6_burnout());
            self.block_contact(other);
            self.alpha_guard_contact(other, if burnout { 22 } else { 18 });
            return;
        }
        let can_counter = matches!(self.eb(other, 0xa), 4 | 8) && (self.can_alpha_counter(defender)
            || self.era_state(defender).is_some_and(|state| state.is_sf3()));
        if blocked && (air_block || can_counter) && self.defender_alive(defender) {
            if !self.guard_power_contact(me, other, GUARD_SPECIAL_PRESSURE) {
                self.block_contact(other);
                if can_counter {
                    // Use a guard window for Alpha without changing the chip damage.
                    self.alpha_guard_contact(other, 18);
                }
            }
            return;
        }
        let src = if blocked { HitSource::Chip } else { HitSource::Projectile };
        self.react_as(me, other, entry, src);
    }

    fn dramatic_shared_knockdown(&mut self, me: u16, other: u16, entry: u32, src: HitSource) {
        use crate::dramatic::LifebarMode;

        if !self.dramatic_rules().is_some_and(|rules| rules.lifebar == LifebarMode::Shared) {
            return;
        }
        let partner = match self.el(other, 0x38) {
            P1_STRUCT => P2_STRUCT,
            P2_STRUCT => P1_STRUCT,
            _ => return,
        };
        let entity = self.rw(partner + 0x10);
        if !self.dramatic_alive(partner) && self.eb(entity, 8) != 8 {
            self.react_one(me, entity, entry, src);
        }
    }

    fn react_one(&mut self, me: u16, other: u16, entry: u32, src: HitSource) {
        self.training_contact(other);
        let acx = self.center_x(me);
        let dcx = self.center_x(other);
        if acx > dcx {
            if self.eb(other, 0x37) != 2 {
                self.set_eb(other, 0x37, 2);
                self.face_right(other);
            }
        } else if acx != dcx && self.eb(other, 0x37) != 4 {
            self.set_eb(other, 0x37, 4);
            self.face_left(other);
        }
        self.ww(0xff8d12, self.ew(other, 0));
        self.wb(0xff8d14, self.eb(other, 0xa));
        self.wb(0xff8d15, self.eb(other, 9));
        let (lo, hi) = match self.eb(other, 9) & 7 {
            4 => (self.rb(entry + 3), self.rb(entry + 6)),
            1 => (self.rb(entry + 4), self.rb(entry + 7)),
            _ => (self.rb(entry + 2), self.rb(entry + 5)),
        };
        let ps = self.el(other, 0x38);
        self.clear_air_attack_trajectory(ps);
        self.world_tour_guard_reaction(ps, src == HitSource::Chip);
        let alive = self.defender_alive(ps);
        let te = match ps {
            ps if self.is_player_driven(ps) => {
                // 0x4461e / 0x446f6: the human players' tables (movea.w:
                // the stream word sign-extended): Ryu's on P1, Ken's on
                // P2, or the bound shoto's own under the full cast. A boss
                // bound here takes his CPU tables, whose stream word is
                // zero-extended (0x447c8).
                let (alive_table, ko_table, signed) = match self.bound_boss_tables(ps) {
                    Some((alive, ko)) => (alive, ko, false),
                    None if self.kens_side(ps) => (0x4473a, 0x4479c, true),
                    None => (0x44662u32, 0x446cau32, true),
                };
                let stream = |w: u16| {
                    if signed {
                        w as i16 as i32 as u32
                    } else {
                        w as u32
                    }
                };
                if alive {
                    self.set_eb(other, 0xa, lo);
                    let idx = ((lo as u16).wrapping_sub(0x30) << 2) as i16;
                    let te = alive_table.wrapping_add(idx as i32 as u32);
                    self.set_el(other, 0x2c, stream(self.rw(te)));
                    self.set_eb(other, 9, self.rb(te + 2));
                    if self.rb(te + 2) & 1 != 0 {
                        self.set_eb(other, 0x12, 0x19);
                    }
                    self.set_eb(other, 8, 2);
                    te
                } else {
                    self.set_eb(other, 8, 8);
                    self.set_eb(other, 0xa, hi);
                    let idx = ((hi as u16).wrapping_sub(0x40) << 2) as i16;
                    let te = ko_table.wrapping_add(idx as i32 as u32);
                    self.set_el(other, 0x2c, stream(self.rw(te)));
                    self.set_eb(other, 9, self.rb(te + 2));
                    self.set_eb(other, 0x12, 9);
                    te
                }
            }
            _ => {
                // 0x447c8: the CPU opponent. Its countdown (+0x19) is
                // cleared, and its tables come from the per-opponent
                // pointer tables at 0xffc050 (alive) / 0xffc054 (knocked
                // out), by the opponent index (stage * 2 + fight, the fight
                // added as a byte) * 4; the stream word is zero-extended.
                self.wb(ps + 0x19, 0);
                let mut d2 = (self.rb(0xffc003) as u16) << 1;
                d2 = (d2 & 0xff00) | (d2 as u8).wrapping_add(self.rb(0xffc004)) as u16;
                d2 <<= 2;
                if alive {
                    self.set_eb(other, 0xa, lo);
                    let idx = ((lo as u16).wrapping_sub(0x30) << 2) as i16;
                    let table = self.rl(self.rl(0xffc050) + d2 as u32);
                    let te = table.wrapping_add(idx as i32 as u32);
                    self.set_el(other, 0x2c, self.rw(te) as u32);
                    self.set_eb(other, 9, self.rb(te + 2));
                    if self.rb(te + 2) & 1 != 0 {
                        self.set_eb(other, 0x12, 0x19);
                    }
                    self.set_eb(other, 8, 2);
                    te
                } else {
                    self.set_eb(other, 8, 8);
                    self.set_eb(other, 0xa, hi);
                    let idx = ((hi as u16).wrapping_sub(0x40) << 2) as i16;
                    let table = self.rl(self.rl(0xffc054) + d2 as u32);
                    let te = table.wrapping_add(idx as i32 as u32);
                    self.set_el(other, 0x2c, self.rw(te) as u32);
                    self.set_eb(other, 9, self.rb(te + 2));
                    self.set_eb(other, 0x12, 9);
                    te
                }
            }
        };
        // 0x445d6
        self.queue_sound(self.rb(te + 3) as u16);
        self.set_eb(other, 0xe, 0);
        if self.eb(other, 0x33) != 0 {
            self.set_eb(other, 0xb, 0);
            if self.eb(other, 0x33) >= 2 {
                self.set_eb(other, 0x33, 2);
            }
        } else if self.eb(other, 0xb) >= 2 {
            self.set_eb(other, 0xb, 2);
        }
        if self.era_true_combos(self.el(me, 0x38)) {
            self.combo_react(me, other, te, src);
        } else {
            let slot = self.combo_slot(other);
            self.toggle_state.combo.blockstun[slot] = 0;
        }
        if self.toggles.cancels {
            // Cancels (SP18): a hit fighter's own window closes (his
            // reaction is started here, not through `start_move`).
            self.cancel_close(other);
        }
    }

    /// 0x3b934..0x3b96a: clear 4 words (8 bytes) starting at `a0` (`D1 = 3`
    /// into `0x3b964: clr.w (A0)+; dbra D1, $3b964`).
    pub(crate) fn clear_words_4(&mut self, a0: u32) {
        for i in 0..4u32 {
            self.ww(a0 + 2 * i, 0);
        }
    }

    /// 0x3b93c..0x3b96a: clear 8 words (16 bytes) starting at `a0` (`D1 = 7`
    /// into `0x3b964: clr.w (A0)+; dbra D1, $3b964`). Translated in F75 (#147)
    /// and exercised in F76 (#148).
    pub(crate) fn clear_words_8(&mut self, a0: u32) {
        for i in 0..8u32 {
            self.ww(a0 + 2 * i, 0);
        }
    }

    /// 0x3b96c..0x3b9ba: clear 3 bytes starting at `a0` (`D0 = 2` into
    /// `0x3b9b4: clr.b (A0)+; dbra D0, $3b9b4`).
    pub(crate) fn clear_bytes_3(&mut self, a0: u32) {
        for i in 0..3u32 {
            self.wb(a0 + i, 0);
        }
    }

    /// 0x3baf6: compare 3 bytes at `a0` and `a1` (`D0 = 3` into `0x3bb0e`).
    /// Translated in F5 (#45; exercised in F52 / F61).
    pub(crate) fn compare_bytes_3(&self, a0: u32, a1: u32) -> std::cmp::Ordering {
        self.compare_bytes(a0, a1, 3)
    }

    /// 0x3bafe: compare 7 bytes at `a0` and `a1` (`D0 = 7` into `0x3bb0e`).
    pub(crate) fn compare_bytes_7(&self, a0: u32, a1: u32) -> std::cmp::Ordering {
        self.compare_bytes(a0, a1, 7)
    }

    /// 0x3bb06: compare 15 bytes at `a0` and `a1` (`D0 = 0xf` into `0x3bb0e`).
    /// No direct reference found in the ROM; translated in F5 (#45) and
    /// labelled unexercised.
    #[allow(dead_code)]
    pub(crate) fn compare_bytes_15(&self, a0: u32, a1: u32) -> std::cmp::Ordering {
        self.compare_bytes(a0, a1, 15)
    }

    /// 0x3bb0e..0x3bb32: compare `count` bytes (`D0` decremented by `dbra`
    /// before each `cmpm.b (A0)+, (A1)+`), returning the ordering of the
    /// `a1` slice relative to `a0`:
    /// - `Ordering::Equal` -> `0x3bb12: rts` (`+0` return slot)
    /// - `Ordering::Less` (`a1 < a0`) -> `0x3bb2a` (`+4` return slot)
    /// - `Ordering::Greater` (`a1 > a0`) -> `0x3bb20` (`+8` return slot)
    pub(crate) fn compare_bytes(&self, a0: u32, a1: u32, count: u16) -> std::cmp::Ordering {
        for i in 0..count as u32 {
            let (b0, b1) = (self.rb(a0 + i), self.rb(a1 + i));
            if b1 > b0 {
                return std::cmp::Ordering::Greater;
            }
            if b1 < b0 {
                return std::cmp::Ordering::Less;
            }
        }
        std::cmp::Ordering::Equal
    }

    /// 0x3ba84: a score above the top score (0xffc012) replaces it and
    /// redraws it (0x3be42), outside demo play only.
    pub(crate) fn check_hiscore(&mut self, me: u16) {
        let ps = self.score_player(self.el(me, 0x38));
        if self.rb(MODE_DEMO) == 0 || ps == CPU_STRUCT {
            return;
        }
        let score = ps + 0x22;
        if self.compare_bytes_7(HISCORE, score) == std::cmp::Ordering::Greater {
            for j in 0..8 {
                let b = self.rb(score + j);
                self.wb(HISCORE + j, b);
            }
            self.show_digits(HISCORE, 0x1bc, 7);
        }
    }

    // ---- effect objects --------------------------------------------------

    const SHAKUNETSU_PALETTE: u8 = 14;

    fn has_evil_ryu_fire_palette(&self, effect: u16) -> bool {
        let owner = self.el(effect, 0x20);
        self.eb(effect, 3) & 0x0f == Self::SHAKUNETSU_PALETTE
            && self.evil_ryu_profile(owner).is_some_and(|profile| profile != EvilRyuPlaytest::Alpha2)
    }

    fn shakunetsu_attr(attr: u8) -> u8 {
        (attr & 0xf0) | Self::SHAKUNETSU_PALETTE
    }

    /// 0x42e1e: spawn an effect object from a 0x20-byte template into the
    /// first free slot (0x4304c: 12 slots at 0x100 + 0x40k), at the attack
    /// box's contact point.
    pub fn spawn_effect(&mut self, me: u16, template: u32) {
        self.spawn_contact_effect(me, template, false);
    }

    fn spawn_demon_spark(&mut self, victim: u16) {
        self.spawn_contact_effect(victim, DEMON_SPARK, true);
    }

    fn clear_raging_demon_blackout(&mut self) {
        self.toggle_state.raging_demon.blackout_frames = None;
        for slot in (0..12u16).map(|k| 0x100 + 0x40 * k) {
            if self.eb(slot, DEMON_SPARK_MARKER as u32) == 1 {
                self.set_eb(slot, 8, 0);
                self.set_el(slot, 4, 0);
                self.set_eb(slot, DEMON_SPARK_MARKER as u32, 0);
            }
        }
    }

    /// Centre of the strike box, also the native hit spark's anchor.
    fn contact_position(&self, e: u16) -> (u16, u16) {
        (
            self.center_x(e).wrapping_add(sext8(self.eb(e, 0x24))),
            self.center_y(e).wrapping_sub(sext8(self.eb(e, 0x22))),
        )
    }

    /// Called after collision, guarding and armor have resolved. The cue
    /// describes the contact even when Training clamps the defender's life.
    fn record_heavy_strike(&mut self, me: u16, other: u16) {
        use crate::combo::{Class, class_of, level};
        use crate::impact::{ImpactContact, ImpactDirection, ImpactKind};
        use std::cmp::Ordering;

        if self.preset == crate::config::RulesetPreset::Faithful1987 || self.rb(MODE_DEMO) == 0 {
            return;
        }
        let owner = self.el(me, 0x38);
        let move_id = self.eb(me, 0xa);
        // Family specials can borrow native jumping-normal move numbers.
        if !Self::is_player_table(self.rl(owner + 0x12))
            || self.era_state(owner).is_some_and(|state| state.alpha_move_active(move_id))
            || !matches!(class_of(move_id, level(self.eb(me, 0x34))), Class::Heavy | Class::Sweep | Class::Jump(3))
        {
            return;
        }
        let slot = self.combo_slot(other);
        let (x, y) = self.contact_position(me);
        let direction = match self.center_x(me).cmp(&self.center_x(other)) {
            Ordering::Greater => ImpactDirection::Left,
            Ordering::Less => ImpactDirection::Right,
            Ordering::Equal if self.eb(me, 0x37) & 4 != 0 => ImpactDirection::Left,
            Ordering::Equal => ImpactDirection::Right,
        };
        let contact = ImpactContact {
            kind: ImpactKind::HeavyStrike,
            x: x.wrapping_add(self.rw(CAMERA)),
            y,
            direction,
        };
        self.toggle_state.impact_contacts[slot].get_or_insert(contact);
    }

    fn record_surface_impact(
        &mut self,
        victim: u16,
        kind: crate::impact::ImpactKind,
        x: u16,
        y: u16,
        direction: crate::impact::ImpactDirection,
    ) {
        use crate::impact::{ImpactContact, ImpactKind};

        if self.preset == crate::config::RulesetPreset::Faithful1987 || self.rb(MODE_DEMO) == 0 {
            return;
        }
        let slot = self.combo_slot(victim);
        if self.toggle_state.impact_contacts[slot].is_none_or(|contact| contact.kind == ImpactKind::HeavyStrike) {
            self.toggle_state.impact_contacts[slot] = Some(ImpactContact {
                kind,
                x: x.wrapping_add(self.rw(CAMERA)),
                y,
                direction,
            });
        }
    }

    fn spawn_contact_effect(&mut self, me: u16, template: u32, demon_spark: bool) {
        let Some(s) = (0..12u16)
            .map(|k| 0x100 + 0x40 * k)
            .find(|&s| self.eb(s, 8) == 0)
        else {
            return;
        };
        for i in 0..0x20u32 {
            let b = self.rb(template + i);
            self.set_eb(s, i, b);
        }
        let owner = self.el(me, 0x38);
        if self.evil_ryu_playtest.is_some() || self.toggle_state.satsui_awakening.is_some() {
            self.set_eb(s, DEMON_SPARK_MARKER as u32, u8::from(demon_spark));
            if template == 0x186dc {
                let red = match self.evil_ryu_profile(owner) {
                    Some(EvilRyuPlaytest::UltraSf2) => self.eb(me, 0x34) == 3,
                    Some(profile) if profile.is_kage() => self.eb(me, 0xa) == KAGE_SHAKUNETSU,
                    Some(EvilRyuPlaytest::Sf6) => self.era_state(owner)
                        .is_some_and(|state| state.sf6_evil_ryu_shakunetsu(self.eb(me, 0xa))),
                    Some(playtest) if playtest.is_alpha3() || playtest.is_sf4() => {
                        let move_id = self.eb(me, 0xa);
                        move_id == crate::data_bank::SF2_SPECIAL_SLOT
                            || self.era_state(owner).is_some_and(|state| state.sf4_metsu_fully_charged(move_id))
                    }
                    _ => false,
                };
                if red {
                    self.set_eb(s, 3, Self::shakunetsu_attr(self.eb(s, 3)));
                }
            }
        }
        self.set_el(s, 0x20, owner);
        self.set_el(s, 0x24, self.el(me, 0x3c));
        if self.eb(s, 0xe) & 0x20 != 0 {
            self.era_spawn_projectile(s, self.el(me, 0x38), self.eb(me, 0xa));
        } else {
            self.era_clear_projectile(s);
        }
        if self.ew(me, 2) & 0x100 != 0 {
            self.set_ew(s, 6, self.ew(s, 6).wrapping_neg());
            self.set_eb(s, 0xb, self.eb(s, 0xb).wrapping_neg());
            self.set_ew(s, 2, self.ew(s, 2) | 0x100);
        }
        let (x, y) = self.contact_position(me);
        let half = if self.ew(s, 2) & 0x400 != 0 { 0x10 } else { 8 };
        self.set_ew(s, 4, self.ew(s, 4).wrapping_add(y).wrapping_sub(half));
        self.set_ew(s, 6, self.ew(s, 6).wrapping_add(x).wrapping_sub(half));
    }

    /// 0x42f1e: spawn an effect from an absolute-position 0x20-byte template.
    /// The first free of the 12 slots is used; x/y are biased up and left by
    /// 16, x-flip negates dx, and owner pointers are copied only for templates
    /// whose flags request them (none of the game-over templates do).
    pub(crate) fn spawn_effect_absolute(&mut self, template: u32, owner: u16) {
        self.spawn_effect_with_bias(template, owner, 0x10);
    }

    /// 0x42f72: spawn a small-sprite effect from an absolute-position 0x20-byte
    /// template (`0x1845c` in name entry, ticket #82, F42): identical to
    /// `0x42f1e` (`spawn_effect_absolute`) except x/y (`+6` / `+4`) are biased
    /// up and left by 8 instead of 16.
    pub(crate) fn spawn_effect_small(&mut self, template: u32, owner: u16) {
        self.spawn_effect_with_bias(template, owner, 8);
    }

    fn spawn_effect_with_bias(&mut self, template: u32, owner: u16, bias: u16) {
        let Some(s) = (0..12u16)
            .map(|k| 0x100 + 0x40 * k)
            .find(|&s| self.eb(s, 8) == 0)
        else {
            return;
        };
        self.era_clear_projectile(s);
        for i in 0..0x20u32 {
            let b = self.rb(template + i);
            self.set_eb(s, i, b);
        }
        if self.evil_ryu_playtest.is_some() || self.toggle_state.satsui_awakening.is_some() {
            self.set_eb(s, DEMON_SPARK_MARKER as u32, 0);
        }
        self.set_ew(s, 4, self.ew(s, 4).wrapping_sub(bias));
        self.set_ew(s, 6, self.ew(s, 6).wrapping_sub(bias));
        if self.ew(s, 2) & 0x100 != 0 {
            self.set_eb(s, 0xb, self.eb(s, 0xb).wrapping_neg());
        }
        if self.eb(s, 0xe) & 0xc0 != 0 {
            self.set_el(s, 0x20, self.el(owner, 0x38));
            self.set_el(s, 0x24, self.el(owner, 0x3c));
        }
    }

    /// 0x4329c: count all 12 effect slots whose state byte is nonzero.
    pub(crate) fn live_effect_count(&self) -> u16 {
        (0..12u16)
            .map(|k| 0x100 + 0x40 * k)
            .filter(|&s| self.eb(s, 8) != 0)
            .count() as u16
    }

    /// 0x42d22: run every live effect object.
    pub fn run_effects(&mut self) {
        for k in 0..12u16 {
            let s = 0x100 + 0x40 * k;
            if self.eb(s, 8) != 0 {
                self.era_projectile_frame(s);
                self.run_effect(s);
            }
            if self.eb(s, 8) == 0 {
                self.era_clear_projectile(s);
            }
        }
    }

    /// 0x42d46: one effect object's frame: first-frame sound, hold
    /// countdown with (dy, dx) drift, or the next 8-byte stream frame
    /// (0x42fc6); 0xfd ends it (slot freed, position zeroed).
    fn run_effect(&mut self, s: u16) {
        if self.eb(s, 8) == 1 {
            self.set_eb(s, 8, 2);
            let snd = self.eb(s, 0xf);
            if snd != 0 {
                self.queue_sound(snd as u16);
            }
            self.effect_collisions(s);
            return;
        }
        if self.eb(s, 9) != 0 {
            self.set_eb(s, 9, self.eb(s, 9) - 1);
            self.set_ew(s, 4, self.ew(s, 4).wrapping_add(sext8(self.eb(s, 0xa))));
            self.set_ew(s, 6, self.ew(s, 6).wrapping_add(sext8(self.eb(s, 0xb))));
            self.effect_offscreen(s);
            self.effect_collisions(s);
            return;
        }
        let mut a0 = self.el(s, 0x10);
        while self.rb(a0) == 0xff {
            // 0x42dd2: dispatch through the stream entry's routine pointer,
            // returning to 0x42dd8 (or looping back to 0x42dc4) with A0
            // updated. Report an untranslated target itself, not the
            // trampoline.
            let target = self.rl(a0 + 2);
            self.effect_stream_calls.insert(target);
            let Some(next_a0) = self.effect_stream_call(s, a0, target) else {
                self.note_unrecovered(target);
                return;
            };
            a0 = next_a0;
        }
        match self.rb(a0) {
            0xfe => a0 = self.rl(a0 + 2),
            0xfd => {
                self.set_eb(s, 8, 0);
                self.set_el(s, 4, 0);
                return;
            }
            _ => {}
        }
        // 0x42fc6
        self.set_eb(s, 9, self.rb(a0));
        self.set_eb(s, 0xa, self.rb(a0 + 2));
        self.set_eb(s, 0xb, self.rb(a0 + 3));
        self.set_eb(s, 0xc, self.rb(a0 + 4));
        self.set_eb(s, 0xd, self.rb(a0 + 5));
        let attr = self.rb(a0 + 1);
        // Keep the private heavy fireball's palette when the ROM advances its frame.
        let attr = if self.has_evil_ryu_fire_palette(s) { Self::shakunetsu_attr(attr) } else { attr };
        self.set_eb(s, 3, attr);
        self.set_ew(s, 0, self.rw(a0 + 6));
        self.set_el(s, 0x10, a0 + 8);
        if self.ew(s, 2) & 0x100 != 0 {
            self.set_eb(s, 0xb, self.eb(s, 0xb).wrapping_neg());
        }
        let dy = self.eb(s, 0xa);
        if dy != 0 {
            self.set_ew(s, 4, self.ew(s, 4).wrapping_add(sext8(dy)));
        }
        let dx = self.eb(s, 0xb);
        if dx != 0 {
            self.set_ew(s, 6, self.ew(s, 6).wrapping_add(sext8(dx)));
        }
        self.set_eb(s, 9, self.eb(s, 9).wrapping_sub(1));
        self.effect_offscreen(s);
        self.effect_collisions(s);
    }

    /// 0x42dd2: dispatch an effect's `0xff` stream call (`target = rl(a0 + 2)`),
    /// returning the updated `A0` stream pointer passed into `0x42dd8` or
    /// `0x42dc4` (`None` for an untranslated routine).
    fn effect_stream_call(&mut self, s: u16, a0: u32, target: u32) -> Option<u32> {
        match target {
            0x427e2 => Some(self.cursor_move_and_button(s, a0)),
            0x42920 => Some(self.cursor_commit_letter(a0)),
            0x432c4 => Some(self.plane_loop_start(s, a0)),
            0x432e0 => Some(self.plane_loop_next(s)),
            0x43306 => Some(self.plane_flip(s)),
            0x4331a => Some(self.plane_sound(s, a0)),
            _ => None,
        }
    }

    /// 0x432c4..0x432df: start a travel-map plane stream loop (ticket #83, F43).
    ///
    /// Loads the loop count byte `rb(a0 + 1)` into `($28, A4, D2.w)`, advances
    /// `A0` by 6 bytes past this `0xff` entry, stores `A0` as the loop target
    /// at `($2a, A4, D2.w)`, and jumps to `0x42dd8`.
    fn plane_loop_start(&mut self, s: u16, a0: u32) -> u32 {
        self.set_eb(s, 0x28, self.rb(a0 + 1));
        let next = a0.wrapping_add(6);
        self.set_el(s, 0x2a, next);
        next
    }

    /// 0x432e0..0x43305: step a travel-map plane stream loop (ticket #83, F43).
    ///
    /// While `($28, A4, D2.w) != 0`, decrements the count, reloads `A0` from
    /// `($2a, A4, D2.w)`, and jumps to `0x42dfe` (`0x42fc6`); when zero
    /// (`0x432f8`), adds 6 to `($10, A4, D2.w)` and jumps back to `0x42dc4`.
    fn plane_loop_next(&mut self, s: u16) -> u32 {
        let d0 = self.eb(s, 0x28);
        if d0 != 0 {
            self.set_eb(s, 0x28, d0.wrapping_sub(1));
            return self.el(s, 0x2a);
        }
        let next = self.el(s, 0x10).wrapping_add(6);
        self.set_el(s, 0x10, next);
        next
    }

    /// 0x43306..0x43319: set the y-flip attribute bit (`ori.b #2, ($2, A4,
    /// D2.w)`) on the travel-map plane sprite, add 6 to `($10, A4, D2.w)`, and
    /// jump back to `0x42dc4` (ticket #83, F43).
    fn plane_flip(&mut self, s: u16) -> u32 {
        self.set_eb(s, 2, self.eb(s, 2) | 2);
        let next = self.el(s, 0x10).wrapping_add(6);
        self.set_el(s, 0x10, next);
        next
    }

    /// 0x4331a..0x43339: queue sound byte `rb(a0 + 1)` (`0x3b7c8`), add 6 to
    /// `($10, A4, D2.w)`, and jump back to `0x42dc4` (ticket #83, F43).
    fn plane_sound(&mut self, s: u16, a0: u32) -> u32 {
        let d0 = self.rb(a0 + 1) as u16;
        self.queue_sound(d0);
        let next = self.el(s, 0x10).wrapping_add(6);
        self.set_el(s, 0x10, next);
        next
    }

    /// 0x427e2..0x4291f: the name-entry cursor's movement and button-press
    /// stream routine (called via `0x42dd2` from `0x18ef4` in stream `0x18ee4`,
    /// ticket #82, F42).
    ///
    /// Reads the active player's direction byte from `*(0xff8e62)` (`A2`),
    /// steps `(row, col)` at `0xff8e52..0xff8e54` and the cursor's `(y, x)` at
    /// `s + 4` / `s + 6` through the 15-entry table at `0x4280a` (wrapping row
    /// `0..=2` by `+-0x20` and col `0..=9` by `+-0x90`), then checks the
    /// player's latched punch strength at `*(0xff8e5e)` (`A1`):
    /// - `<= 2`: clears `(A1)` and loops the stream to `0x18ee4`;
    /// - `> 2`: clears `(A1)`, queues sound `0x1b`, saves the grid cell's video
    ///   offset at `0xff8e54` and tile word at `0xff8e56`, blanks the cell with
    ///   `0x003c`, and advances the stream to `a0 + 6` (`0x18efa`).
    fn cursor_move_and_button(&mut self, s: u16, a0: u32) -> u32 {
        let a2 = self.rl(0xff8e62);
        let a1 = self.rl(0xff8e5e);
        let d0 = (!self.rb(a2) as u16) & 0x000f;
        if d0 != 0 {
            let entry = 0x4280a + ((d0 - 1) as u32) * 4;
            let drow = self.rb(entry + 2);
            let row = self.rb(0xff8e52).wrapping_add(drow);
            self.wb(0xff8e52, row);
            if (row as i8) < 0 {
                self.wb(0xff8e52, 2);
                self.set_ew(s, 4, self.ew(s, 4).wrapping_add(0x0020));
            } else if row > 2 {
                self.wb(0xff8e52, 0);
                self.set_ew(s, 4, self.ew(s, 4).wrapping_sub(0x0020));
            } else {
                let dy = sext8(self.rb(entry));
                self.set_ew(s, 4, self.ew(s, 4).wrapping_add(dy));
            }
            let dcol = self.rb(entry + 3);
            let col = self.rb(0xff8e53).wrapping_add(dcol);
            self.wb(0xff8e53, col);
            if (col as i8) < 0 {
                self.wb(0xff8e53, 9);
                self.set_ew(s, 6, self.ew(s, 6).wrapping_add(0x0090));
            } else if col > 9 {
                self.wb(0xff8e53, 0);
                self.set_ew(s, 6, self.ew(s, 6).wrapping_sub(0x0090));
            } else {
                let dx = sext8(self.rb(entry + 1));
                self.set_ew(s, 6, self.ew(s, 6).wrapping_add(dx));
            }
        }
        if self.rb(a1) <= 2 {
            self.wb(a1, 0);
            return 0x18ee4;
        }
        self.wb(a1, 0);
        self.queue_sound(0x1b);
        let row = self.rb(0xff8e52);
        let mut d1 = if row == 0 {
            0u16
        } else {
            0x0100u16.wrapping_shl((sext8(row).wrapping_sub(1)) as u32)
        };
        d1 = d1.wrapping_add(0x062e);
        d1 = d1.wrapping_add(sext8(self.rb(0xff8e53)).wrapping_shl(2));
        self.ww(0xff8e54, d1);
        let cell_addr = 0x800000 + d1 as u32;
        self.ww(0xff8e56, self.rw(cell_addr));
        self.ww(cell_addr, 0x003c);
        a0 + 6
    }

    /// 0x42920..0x429f0: the name-entry cursor's letter-commit stream routine
    /// (called via `0x42dd2` from `0x18f1a` in stream `0x18ee4`, ticket #82,
    /// F42).
    ///
    /// Restores the saved grid tile word (`0xff8e56`) at video offset
    /// `0xff8e54`, looks up the selected character in `0x14638` (`row * 10 +
    /// col`), and dispatches on its code:
    /// - `0x32` (`BACK`): if `0xff8e5a != 0xff8e6a`, decrements the buffer
    ///   pointer and resets its byte to `0x31`, decrements the display offset
    ///   `0xff8e58` by 2 and writes `0x1031` to video RAM, then loops to
    ///   `0x18ee4`;
    /// - `0x33` (`END`): advances the stream to `a0 + 6` (`0x18f20`, `0xfd`);
    /// - any other byte: stores it at `*(0xff8e5a)++`, writes `0x1000 | code`
    ///   at video offset `0xff8e58` and adds 2; if `0xff8e58` reaches `0x044a`
    ///   (third initial entered), advances to `a0 + 6` (`0x18f20`, `0xfd`),
    ///   otherwise loops to `0x18ee4`.
    fn cursor_commit_letter(&mut self, a0: u32) -> u32 {
        let saved_off = self.rw(0xff8e54);
        let saved_word = self.rw(0xff8e56);
        self.ww(0x800000 + saved_off as u32, saved_word);
        let row = self.rb(0xff8e52);
        let mut d1 = if row == 0 {
            0u16
        } else {
            0x000au16.wrapping_shl((sext8(row).wrapping_sub(1)) as u32)
        };
        d1 = d1.wrapping_add(sext8(self.rb(0xff8e53)));
        let code = self.rb(0x14638 + d1 as u32);
        match code {
            0x32 => {
                let buf = self.rl(0xff8e5a);
                if buf != 0xff8e6a {
                    let prev = buf - 1;
                    self.wb(prev, 0x31);
                    self.wl(0xff8e5a, prev);
                    let disp = self.rw(0xff8e58).wrapping_sub(2);
                    self.ww(0x800000 + disp as u32, 0x1031);
                    self.ww(0xff8e58, disp);
                }
                0x18ee4
            }
            0x33 => a0 + 6,
            _ => {
                let buf = self.rl(0xff8e5a);
                self.wb(buf, code);
                self.wl(0xff8e5a, buf + 1);
                let disp = self.rw(0xff8e58);
                self.ww(0x800000 + disp as u32, 0x1000 | (code as u16));
                let next_disp = disp.wrapping_add(2);
                self.ww(0xff8e58, next_disp);
                if next_disp == 0x044a {
                    a0 + 6
                } else {
                    0x18ee4
                }
            }
        }
    }

    /// The effect runner's collision tests (0x42d6e.. / 0x42dac..): against
    /// the target fighter (0x430b6, +0xe bit 0x80), and while the effect is
    /// live (+8 == 2) against the target's attack box (0x43144, bit 0x40)
    /// and another projectile (0x431c6, bit 0x20).
    fn effect_collisions(&mut self, s: u16) {
        if let Some(rules) = self.dramatic_rules() {
            self.dramatic_effect_collisions(s, rules.friendly_fire);
            return;
        }
        self.effect_vs_fighter(s);
        if self.eb(s, 8) == 2 {
            self.effect_vs_attack(s);
            self.effect_vs_projectile(s);
        }
    }

    fn dramatic_effect_collisions(&mut self, s: u16, friendly_fire: bool) {
        let owner = self.el(s, 0x20);
        if self.eb(s, 0xe) & 0xe0 == 0 || !DRAMATIC_PLAYERS.contains(&owner) {
            return;
        }
        let targets = self.dramatic_hit_targets(owner, friendly_fire);
        let [first, second] = targets.map(|ps| {
            self.dramatic_alive(ps) && self.effect_overlaps_fighter(s, self.rw(ps + 0x10))
        });
        let previous = self.el(s, 0x24);
        let primary = self.el(self.rw(owner + 0x10), 0x3c);
        let both = first && second;
        // One impact can contact both allies before the projectile changes
        // into its impact animation. Each hit uses the incoming effect frame.
        let incoming: [u32; 8] = if both {
            std::array::from_fn(|i| self.el(s, i as u32 * 4))
        } else {
            [0; 8]
        };
        for (index, (defender, hit)) in targets.into_iter().zip([first, second]).enumerate() {
            if !hit || !self.dramatic_alive(defender) {
                continue;
            }
            if both && index == 1 {
                for (i, value) in incoming.into_iter().enumerate() {
                    self.set_el(s, i as u32 * 4, value);
                }
            }
            self.bind_dramatic_target(owner, defender);
            self.set_el(s, 0x24, defender);
            self.effect_handler(s, self.el(s, 0x14), None);
        }
        for defender in targets {
            if self.eb(s, 8) == 2 && self.dramatic_alive(defender) {
                self.set_el(s, 0x24, defender);
                self.effect_vs_attack(s);
            }
        }
        if self.eb(s, 8) == 2 {
            self.effect_vs_projectile(s);
        }
        if first || second {
            self.bind_dramatic_target(owner, primary);
        }
        self.set_el(s, 0x24, previous);
    }

    /// 0x43234: an effect's centre: its position + 0x10 (big sprite, attr
    /// bit 0x400) or + 8, as (x, y).
    fn effect_centre(&self, s: u16) -> (u16, u16) {
        let half = if self.ew(s, 2) & 0x400 != 0 { 0x10 } else { 8 };
        (
            self.ew(s, 6).wrapping_add(half),
            self.ew(s, 4).wrapping_add(half),
        )
    }

    /// The distance test of the effect collisions: `a - b`, negated if the
    /// subtraction borrowed (sub.w; bcc; neg.w).
    fn borrow_abs(a: u16, b: u16) -> u16 {
        let d = a.wrapping_sub(b);
        if a < b {
            d.wrapping_neg()
        } else {
            d
        }
    }

    /// The target fighter of an effect: its player structure (+0x24) and
    /// that structure's own entity.
    fn effect_target(&self, s: u16) -> u16 {
        let ps = self.el(s, 0x24);
        self.rw(ps + 0x10)
    }

    /// 0x430b6: the effect's box (half-sizes +0xc / +0xd about its centre)
    /// against the target's loose box (+0x1e / +0x1f vertical, +0x20 /
    /// +0x21 horizontal); an invulnerable target (+0xf bit 3) is skipped.
    /// A hit runs the effect's handler at +0x14.
    fn effect_vs_fighter(&mut self, s: u16) {
        let defender = self.effect_target(s);
        if !self.effect_overlaps_fighter(s, defender) {
            return;
        }
        let parried = self.what_if_eagle_riposte(defender, self.effect_centre(s).0)
            || self.sf3_parry(defender, self.effect_centre(s).0, false, true, self.era_airborne(defender));
        if parried || self.sf5_defend(self.el(defender, 0x38), defender, false)
            || self.sf6_defend(self.el(defender, 0x38))
        {
            if parried {
                let slot = self.combo_slot(defender);
                self.toggle_state.combo.blockstun[slot] = 0;
            }
            if self.era_projectile_hit(s) {
                self.transform_effect(s, 0x186fc);
            }
            return;
        }
        self.effect_handler(s, self.el(s, 0x14), None);
    }

    fn effect_overlaps_fighter(&self, s: u16, t: u16) -> bool {
        if self.eb(s, 0xe) & 0x80 == 0 || !self.era_projectile_can_hit(s) {
            return false;
        }
        let era = self.era_projectile(s);
        let limit = era.hit_limit();
        let juggle = self.era_can_juggle(t, limit);
        if (self.eb(t, 0xf) & 8 != 0 || (era.hits_remaining != 0 && self.airborne_reaction(t))) && !juggle {
            return false;
        }
        let (cx, cy) = self.effect_centre(s);
        let (tx, ty, w, h) = if let Some((w, h)) = self.era_juggle_body(t, juggle) {
            (self.center_x(t), self.center_y(t), w, h)
        } else {
            (
                self.center_x(t).wrapping_add(sext8(self.eb(t, 0x20))),
                self.center_y(t).wrapping_sub(sext8(self.eb(t, 0x1e))),
                u16::from(self.eb(t, 0x21)),
                u16::from(self.eb(t, 0x1f)),
            )
        };
        w + u16::from(self.eb(s, 0xd)) > Self::borrow_abs(tx, cx)
            && h + u16::from(self.eb(s, 0xc)) > Self::borrow_abs(ty, cy)
    }

    /// 0x43144: the effect's box against the target's attack box (+0x22 /
    /// +0x23 vertical, +0x24 / +0x25 horizontal); a hit runs the handler at
    /// +0x18.
    fn effect_vs_attack(&mut self, s: u16) {
        if self.eb(s, 0xe) & 0x40 == 0 {
            return;
        }
        let (cx, cy) = self.effect_centre(s);
        let t = self.effect_target(s);
        let ty = self
            .center_y(t)
            .wrapping_add(sext8(self.eb(t, 0x22)).wrapping_neg());
        if (self.eb(t, 0x23) as u16) + (self.eb(s, 0xc) as u16) <= Self::borrow_abs(ty, cy) {
            return;
        }
        let tx = self.center_x(t).wrapping_add(sext8(self.eb(t, 0x24)));
        if (self.eb(t, 0x25) as u16) + (self.eb(s, 0xd) as u16) <= Self::borrow_abs(tx, cx) {
            return;
        }
        let handler = self.el(s, 0x18);
        self.effect_handler(s, handler, None);
    }

    /// 0x431c6: the first other live projectile (0x43072: the next 11
    /// slots after this one, with +0xe bit 0x20; the step wraps 0x400 to
    /// 0x100, the first slot unwrapped as in the original)
    /// whose box overlaps this one's (the half-sizes summed as bytes); a
    /// hit runs the handler at +0x1c.
    fn effect_vs_projectile(&mut self, s: u16) {
        if self.eb(s, 0xe) & 0x20 == 0 || !self.era_projectile_can_clash(s) {
            return;
        }
        let mut d2 = s.wrapping_add(0x40);
        let mut other = None;
        for _ in 0..11 {
            if self.eb(d2, 8) != 0 && self.eb(d2, 0xe) & 0x20 != 0
                && self.era_projectile_can_clash(d2)
                && ((self.era_projectile(s).hits_remaining == 0 && self.era_projectile(d2).hits_remaining == 0)
                    || self.el(s, 0x20) != self.el(d2, 0x20))
                && self.dramatic_rules().is_none_or(|rules| {
                    self.dramatic_hit_targets(self.el(s, 0x20), rules.friendly_fire)
                        .contains(&self.el(d2, 0x20))
                })
            {
                other = Some(d2);
                break;
            }
            d2 = d2.wrapping_add(0x40);
            if d2 == 0x400 {
                d2 = 0x100;
            }
        }
        let Some(o) = other else {
            return;
        };
        let (x2, y2) = self.effect_centre(s);
        let (x3, y3) = self.effect_centre(o);
        if Self::borrow_abs(x3, x2) >= self.eb(s, 0xd).wrapping_add(self.eb(o, 0xd)) as u16 {
            return;
        }
        if Self::borrow_abs(y3, y2) >= self.eb(s, 0xc).wrapping_add(self.eb(o, 0xc)) as u16 {
            return;
        }
        let handler = self.el(s, 0x1c);
        self.effect_handler(s, handler, Some(o));
    }

    /// The collision handlers of the projectile templates: the Hadoken's
    /// (0x186dc: +0x14 0x433ba, +0x18 0x43490, +0x1c 0x43476), Geki's
    /// shurikens (0x43348 / 0x43396) and 0x1871c's.
    fn effect_handler(&mut self, s: u16, handler: u32, other: Option<u16>) {
        match (handler, other) {
            (0x43348, None) => {
                // 0x43348: a target with +8 bit 3 set rejects the hit.
                // Otherwise the shuriken becomes its hit effect and applies
                // its inline damage/reaction entry, with no score or spark.
                let (me, other) = self.player_entities(self.el(s, 0x20));
                if self.eb(me, 8) & 8 != 0 {
                    return;
                }
                let air_block = self.alpha_air_blocks(other, self.effect_centre(s).0);
                self.transform_effect(s, 0x18944);
                if air_block
                    || (self.toggles.standard_high_low_block
                        && (self.eb(other, 9) & 0x20 != 0 || matches!(self.eb(other, 0xa), 4 | 8)))
                {
                    if !self.guard_power_contact(me, other, GUARD_SPECIAL_PRESSURE) {
                        self.block_contact(other);
                    }
                    return;
                }
                self.damage(me, self.rb(0x4338f));
                self.react_as(me, other, 0x4338e, HitSource::Projectile);
            }
            (0x43396, _) => {
                // 0x43396: deflect this shuriken (or disappearing smoke) when struck
                // by the opponent's attack box (0x43144 -> jmp 0x431c0). +0xa = 4
                // makes the effect runner animate it.
                self.set_eb(s, 9, 0x63);
                self.set_eb(s, 0xb, self.eb(s, 0xb).wrapping_neg());
                self.set_eb(s, 0xa, 4);
                self.set_eb(s, 0xe, 0);
                self.queue_sound(0x22);
            }
            (0x433ba, None) => self.fireball_hits_fighter(s),
            // A bank effect's hit record (ADR-0014): only a bank template
            // names one.
            (h, None)
                if crate::data_bank::in_bank(h)
                    && self.rb(h) == crate::data_bank::ROUTINE_EFFECT_HIT =>
            {
                self.bank_effect_hit(s, h)
            }
            // A bank copy of a ROM projectile's hit (ADR-0014): the ROM
            // handler, then, when it landed the hit (the target was put into
            // a reaction: its move and stream start one, `react`), the
            // thrower scores with the record's entry.
            (h, None)
                if crate::data_bank::in_bank(h)
                    && self.rb(h) == crate::data_bank::ROUTINE_EFFECT_ROM_HIT =>
            {
                let target = self.effect_target(s);
                let before = (self.eb(target, 0xa), self.el(target, 0x2c));
                self.effect_handler(s, self.rl(h + 4), None);
                let now = (self.eb(target, 0xa), self.el(target, 0x2c));
                if now != before && now.0 >= 0x30 {
                    let owner_ps = self.el(s, 0x20);
                    let thrower = self.rw(owner_ps + 0x10);
                    self.add_score(thrower, self.rb(self.rl(h + 8)));
                    self.show_score(owner_ps);
                    self.check_hiscore(thrower);
                }
            }
            // A bank effect against an attack box: as 0x43490 (nothing in
            // 2P versus; otherwise the stage table at 0x434d0 decides), but
            // into its own impact, and its thrower's hold is left alone.
            (h, None)
                if crate::data_bank::in_bank(h)
                    && self.rb(h) == crate::data_bank::ROUTINE_EFFECT_VS_ATTACK =>
            {
                if self.rb(GAME_MODE) != 1 {
                    let stage = sext8(self.rb(0xffc003)) as i16 as i32;
                    if self.rb((0x434d0 + stage) as u32) == 1 {
                        self.transform_effect(s, self.rl(h + 4));
                    }
                }
            }
            // A bank effect against another projectile: as 0x43476, both
            // end, this one into its own impact.
            (h, Some(o))
                if crate::data_bank::in_bank(h)
                    && self.rb(h) == crate::data_bank::ROUTINE_EFFECT_VS_PROJECTILE =>
            {
                self.resolve_projectile_clash(s, o, self.rl(h + 4));
            }
            (0x43476, Some(o)) => {
                // Both projectiles end: this one explodes, the other is freed.
                self.resolve_projectile_clash(s, o, 0x186fc);
            }
            (0x43490, None) => {
                // Against an attack box: nothing in 2P versus; otherwise the
                // stage table (0x434d0: 0, 0, 1, 1, 1, 1) decides whether it
                // explodes and cuts the owner's hold short.
                if self.rb(GAME_MODE) != 1 {
                    let stage = sext8(self.rb(0xffc003)) as i16 as i32;
                    if self.rb((0x434d0 + stage) as u32) == 1 {
                        self.transform_effect(s, 0x186fc);
                        let owner = self.rw(self.el(s, 0x20) + 0x10);
                        self.set_eb(owner, 0x33, 0);
                        self.set_eb(owner, 0xe, 0);
                    }
                }
            }
            // 0x434d6 (template 0x1871c's only handler): the effect ends.
            (0x434d6, None) => {
                self.set_eb(s, 8, 0);
                self.set_el(s, 4, 0);
            }
            // 0x434e4 (0x18e24), 0x4352c (0x18e44), 0x4357a (0x18e64),
            // 0x435c2 (0x18e84), 0x4360a (0x18ea4): stage-1 and stage-3 bonus
            // concrete block stack break handlers (tickets #124 / F52 and
            // #133 / F61). Spawn the break debris effect `0x18ec4` at the
            // attack box's contact point (`0x42e1e`), switch the stack's
            // stream (`+0x10`) to its broken stream (`0x1943a`, `0x19456`,
            // `0x19472`, `0x1948e`, `0x194aa`), clear its hold (`+9`) and
            // collision flags (`+0x0e`), and add the inline 7-digit bonus
            // (`0x4352b` = 1,000, `0x43579` = 3,000, `0x435c1` = 1,000,
            // `0x43609` = 3,000, `0x43651` = 1,000) onto `0xff8f18..0xff8f1f`
            // (`0x3ba0e`). Only `0x4352c` has the second `jsr $3ba0e.l`
            // (`0x43566`) that also adds its 3,000 points directly onto the
            // player's score (`ps + 0x22..+0x29`).
            (0x434e4 | 0x4352c | 0x4357a | 0x435c2 | 0x4360a, None) => {
                let (broken_stream, score_end, add_player) = match handler {
                    0x434e4 => (0x1943au32, 0x4352bu32, false),
                    0x4352c => (0x19456, 0x43579, true),
                    0x4357a => (0x19472, 0x435c1, false),
                    0x435c2 => (0x1948e, 0x43609, false),
                    _ => (0x194aa, 0x43651, false),
                };
                self.spawn_effect(self.effect_target(s), 0x18ec4);
                self.set_el(s, 0x10, broken_stream);
                self.set_eb(s, 9, 0);
                self.set_eb(s, 0xe, 0);
                self.decimal_add(0xff8f1f, score_end, 7);
                if add_player {
                    let ps = self.el(s, 0x24);
                    self.decimal_add(ps + 0x29, score_end, 7);
                }
            }
            // 0x43652..0x436b6: Sagat's Tiger Shot projectile (0x1873c) hits
            // the target fighter (ticket #135, F63). Transforms the projectile
            // into the explosion 0x186fc (0x42eb4); a blocking target (+9 bit
            // 0x20, 0x4366e..0x4367a) takes chip-damage entry 0x436b0 (15
            // damage), whereas an unblocked hit (0x4367e..0x43688) requests
            // slow motion (SCENE_MODE |= 0x20, 0xff8e36 = 9) and takes entry
            // 0x436a8 (30 damage), applying damage (0x3c034) and reaction
            // (0x4451a).
            (0x43652, None) => {
                let attack_x = self.effect_centre(s).0;
                let era = self.era_projectile(s);
                if self.era_projectile_hit(s) {
                    self.transform_effect(s, 0x186fc);
                }
                let (me, other) = self.player_entities(self.el(s, 0x20));
                let air_block = !era.unblockable && self.alpha_air_blocks(other, attack_x);
                let is_blocked = !era.unblockable && (air_block || self.eb(other, 9) & 0x20 != 0);
                let entry = if is_blocked { 0x436b0 } else { 0x436a8 };
                let dmg = if is_blocked && self.era_state(self.el(other, 0x38)).is_some_and(|state| state.sf6_burnout()) {
                    self.effective_entry_damage(me, 0x436a8, false).div_ceil(4)
                } else {
                    self.effective_entry_damage(me, entry, is_blocked)
                };
                self.damage_with_source(me, dmg, is_blocked, Some(era));
                if !is_blocked {
                    let def_ps = self.el(me, 0x3c);
                    let def_ko = !self.defender_alive(def_ps);
                    if !self.toggles.disable_fireball_slowdown || def_ko {
                        self.wb(SCENE_MODE, self.rb(SCENE_MODE) | 0x20);
                        self.wb(0xff8e36, 9);
                    }
                }
                self.react_projectile(me, other, entry, is_blocked, air_block);
            }
            (h, _) => self.note_unrecovered(h),
        }
    }

    fn resolve_projectile_clash(&mut self, first: u16, second: u16, impact: u32) {
        let first_ends = self.era_projectile_clash(first);
        let second_ends = self.era_projectile_clash(second);
        if first_ends {
            self.transform_effect(first, impact);
        }
        if second_ends {
            self.era_clear_projectile(second);
            self.set_ew(second, 8, 0);
            self.set_el(second, 4, 0);
        }
    }

    /// 0x433ba: the Hadoken hits the target: it becomes the explosion
    /// (0x186fc) and cuts the thrower's hold short (+0x33, +0xe). A blocking
    /// target (+9 bit 0x20) takes chip damage (entry 0x43466 in 2P versus,
    /// else 0x4345e + the stage's byte of 0x4340a: 8, 8, 16, 16, 0, 0xff; 0
    /// gives nothing); otherwise the hit requests the slow motion (scene
    /// event 9) and takes entry 0x4345e. Score, its redraw, the top-score
    /// check, damage and the reaction follow as for any landed hit.
    fn fireball_hits_fighter(&mut self, s: u16) {
        let attack_x = self.effect_centre(s).0;
        let era = self.era_projectile(s);
        if self.era_projectile_hit(s) {
            self.transform_effect(s, 0x186fc);
        }
        let owner_ps = self.el(s, 0x20);
        let (me, other) = self.player_entities(owner_ps);
        let air_block = !era.unblockable && self.alpha_air_blocks(other, attack_x);
        let is_blocked = !era.unblockable && (air_block || self.eb(other, 9) & 0x20 != 0);
        let entry = if is_blocked {
            if self.toggles.balanced_roster || self.rb(GAME_MODE) == 1 {
                0x43466
            } else {
                let stage = sext8(self.rb(0xffc003));
                let d0 =
                    (stage & 0xff00) | self.rb((0x4340ai32 + stage as i16 as i32) as u32) as u16;
                if d0 & 0xff == 0 {
                    // Blocked for nothing, but blocked: Training's wait
                    // starts over all the same.
                    self.record_guard_contact(other);
                    self.training_contact(other);
                    return;
                }
                (0x4345ei32 + d0 as i16 as i32) as u32
            }
        } else {
            0x4345e
        };
        self.set_eb(me, 0x33, 0);
        self.set_eb(me, 0xe, 0);
        self.add_score(me, self.rb(entry));
        self.show_score(owner_ps);
        self.check_hiscore(me);
        let dmg = if is_blocked && self.era_state(self.el(other, 0x38)).is_some_and(|state| state.sf6_burnout()) {
            self.effective_entry_damage(me, 0x4345e, false).div_ceil(4)
        } else {
            self.effective_entry_damage(me, entry, is_blocked)
        };
        self.damage_with_source(me, dmg, is_blocked, Some(era));
        if !is_blocked {
            let def_ps = self.el(me, 0x3c);
            let def_ko = !self.defender_alive(def_ps);
            if !self.toggles.disable_fireball_slowdown || def_ko {
                self.wb(SCENE_MODE, self.rb(SCENE_MODE) | 0x20);
                self.wb(0xff8e36, 9);
            }
        }
        self.react_projectile(me, other, entry, is_blocked, air_block);
    }

    /// 0x42eb4: turn effect `s` into `template` in place: the template's
    /// 0x20 bytes copied over it, its flip kept (attr bit 0x100: x offset
    /// and dx negated), and its position offsets moved to the old effect's
    /// centre (less 0x10 for a big sprite, 8 otherwise).
    fn transform_effect(&mut self, s: u16, template: u32) {
        let red_impact = template == 0x186fc && self.has_evil_ryu_fire_palette(s);
        self.era_clear_projectile(s);
        let (cx, cy) = self.effect_centre(s);
        let attr = self.ew(s, 2);
        for i in 0..0x20u32 {
            let b = self.rb(template + i);
            self.set_eb(s, i, b);
        }
        if red_impact {
            self.set_eb(s, 3, Self::shakunetsu_attr(self.eb(s, 3)));
        }
        if attr & 0x100 != 0 {
            self.set_ew(s, 2, self.ew(s, 2) | 0x100);
            self.set_ew(s, 6, self.ew(s, 6).wrapping_neg());
            self.set_eb(s, 0xb, self.eb(s, 0xb).wrapping_neg());
        }
        let half = if self.ew(s, 2) & 0x400 != 0 { 0x10 } else { 8 };
        self.set_ew(s, 4, self.ew(s, 4).wrapping_add(cy.wrapping_sub(half)));
        self.set_ew(s, 6, self.ew(s, 6).wrapping_add(cx.wrapping_sub(half)));
    }

    /// 0x43268: free an effect whose centre (0x43234) leaves x 0x60..=0x1a0
    /// or y 0x10..=0xe8.
    fn effect_offscreen(&mut self, s: u16) {
        let half = if self.ew(s, 2) & 0x400 != 0 { 0x10 } else { 8 };
        let x = self.ew(s, 6).wrapping_add(half);
        let y = self.ew(s, 4).wrapping_add(half);
        if !(0x10..=0xe8).contains(&y) || !(0x60..=0x1a0).contains(&x) {
            self.set_eb(s, 8, 0);
            self.set_el(s, 4, 0);
            self.set_eb(s, 0xe, 0);
        }
    }

    // ---- camera ------------------------------------------------------------

    /// 0x4392c: resolve the fighters' camera-scroll requests (+0x36). A
    /// single request scrolls the camera and moves the other fighter back
    /// by it; opposite requests cancel; same-direction requests scroll by
    /// one and leave the difference to the other.
    pub fn resolve_camera(&mut self) {
        if self.dramatic_rules().is_some() {
            self.resolve_dramatic_camera();
            return;
        }
        let (a0, a1) = match self.rb(GAME_MODE) {
            2 => (P1_STRUCT, CPU_STRUCT),
            4 => (P2_STRUCT, CPU_STRUCT),
            _ => (P1_STRUCT, P2_STRUCT),
        };
        let (d1, _) = self.player_entities(a0);
        let (d2, _) = self.player_entities(a1);
        let r1 = self.eb(d1, 0x36);
        let r2 = self.eb(d2, 0x36);
        let (target, d3) = if r1 == 0 {
            if r2 == 0 {
                self.set_eb(d1, 0x36, 0);
                self.set_eb(d2, 0x36, 0);
                return;
            }
            self.set_eb(d1, 0x36, r2);
            self.set_eb(d2, 0x36, 0);
            (d1, sext8(r2))
        } else if r2 == 0 {
            self.set_eb(d2, 0x36, r1);
            self.set_eb(d1, 0x36, 0);
            (d2, sext8(r1))
        } else if ((r1 as i8) < 0) != ((r2 as i8) < 0) {
            self.set_eb(d1, 0x36, 0);
            self.set_eb(d2, 0x36, 0);
            return;
        } else if (r1 as i8) >= 0 {
            if r1 == r2 {
                self.set_eb(d2, 0x36, 0);
                self.set_eb(d1, 0x36, 0);
                (d1, r1 as u16)
            } else if r2 >= r1 {
                self.set_eb(d1, 0x36, r2 - r1);
                self.set_eb(d2, 0x36, 0);
                (d1, sext8(r2))
            } else {
                self.set_eb(d2, 0x36, r1 - r2);
                self.set_eb(d1, 0x36, 0);
                (d2, sext8(r1))
            }
        } else {
            let (n1, n2) = (r1.wrapping_neg(), r2.wrapping_neg());
            if n1 == n2 {
                self.set_eb(d2, 0x36, 0);
                self.set_eb(d1, 0x36, 0);
                (d1, sext8(r1))
            } else if n2 >= n1 {
                self.set_eb(d1, 0x36, n2.wrapping_sub(n1).wrapping_neg());
                self.set_eb(d2, 0x36, 0);
                (d1, sext8(r2))
            } else {
                self.set_eb(d2, 0x36, n1.wrapping_sub(n2).wrapping_neg());
                self.set_eb(d1, 0x36, 0);
                (d2, sext8(r1))
            }
        };
        // 0x43a58: the camera stays within 0..=0x180 (the lower bound check
        // at 0x43a6e subtracts 0 and never fires; a negative result wraps
        // above 0x180 instead).
        if self.rw(CAMERA).wrapping_add(d3) > 0x180 {
            self.set_eb(target, 0x36, 0);
            return;
        }
        let d3 = self.camera_room(target, d3);
        if d3 == 0 {
            return;
        }
        self.ww(CAMERA, self.rw(CAMERA).wrapping_add(d3));
        // 0x3b674: fg_scroll copy and register.
        let fg = self.rw(FG_SCROLL_COPY).wrapping_add(d3);
        self.ww(FG_SCROLL_COPY, fg);
        *self.fg_scroll = fg;
        // 0x43b34: move the target back by its remaining request.
        let r = self.eb(target, 0x36);
        if r != 0 {
            self.set_eb(target, 0x36, 0);
            self.shift_grid_x(target, sext8(r).wrapping_neg());
        }
        // 0x4302c: effects move with the stage.
        let back = d3.wrapping_neg();
        for k in 0..12u16 {
            let s = 0x100 + 0x40 * k;
            if self.eb(s, 8) != 0 {
                self.set_ew(s, 6, self.ew(s, 6).wrapping_add(back));
            }
        }
    }

    fn resolve_dramatic_camera(&mut self) {
        let origin = sf_video::render::VISAREA_X as i32;
        let camera = i32::from(self.rw(CAMERA));
        let fighters = DRAMATIC_PLAYERS.map(|ps| self.rw(ps + 0x10));
        let world = fighters.map(|entity| camera + i32::from(self.center_x(entity) as i16) - origin);
        let alive = self.toggle_state.dramatic.health.map(|hp| hp != 0);
        let mut living = world.into_iter().zip(alive).filter_map(|(x, alive)| alive.then_some(x));
        let Some(first) = living.next() else {
            return;
        };
        let (min, max) = living.fold((first, first), |(min, max), x| (min.min(x), max.max(x)));
        let next = ((min + max - crate::dramatic::VIEW_W) / 2).clamp(0, 0x180);
        let delta = (next - camera) as u16;
        self.ww(CAMERA, next as u16);
        let fg = self.rw(FG_SCROLL_COPY).wrapping_add(delta);
        self.ww(FG_SCROLL_COPY, fg);
        *self.fg_scroll = fg;
        for ((entity, x), alive) in fighters.into_iter().zip(world).zip(alive) {
            let desired = if alive {
                x.clamp(next + 16, next + crate::dramatic::VIEW_W - 16)
            } else {
                x
            };
            self.set_eb(entity, 0x36, 0);
            self.shift_grid_x(entity, ((desired - x) as u16).wrapping_sub(delta));
        }
        for k in 0..12u16 {
            let s = 0x100 + 0x40 * k;
            if self.eb(s, 8) != 0 {
                self.set_ew(s, 6, self.ew(s, 6).wrapping_sub(delta));
            }
        }
    }

    /// 0x43aa8: shrink a scroll so the fighter moved back by it stays inside
    /// its own screen band; the excess also adjusts its +0x36 request.
    fn camera_room(&mut self, e: u16, mut d3: u16) -> u16 {
        let mut d7 = self.center_x(e);
        let d0 = sext8(self.eb(e, 0x36)).wrapping_neg();
        let facing_left = self.eb(e, 0x37) >= 4;
        if (d0 as i16) < 0 {
            // 0x43af8
            d7 = d7.wrapping_add(d3);
            let (half, lim) = if facing_left {
                (0x18, 0x80)
            } else {
                (0x19, 0x60)
            };
            let d2 = d7.wrapping_sub(self.eb(e, half) as u16);
            if d2 >= lim {
                return d3;
            }
            let d2 = d2.wrapping_sub(lim);
            d3 = d3.wrapping_add(d2);
            self.set_eb(e, 0x36, self.eb(e, 0x36).wrapping_add(d2 as u8));
        } else {
            // 0x43ac0
            d7 = d7.wrapping_add(d0);
            let (half, lim) = if facing_left {
                (0x19, 0x1a0)
            } else {
                (0x18, 0x180)
            };
            let d2 = (self.eb(e, half) as u16).wrapping_add(d7);
            if d2 <= lim {
                return d3;
            }
            let d2 = d2 - lim;
            d3 = d3.wrapping_add(d2);
            self.set_eb(e, 0x36, self.eb(e, 0x36).wrapping_add(d2 as u8));
        }
        d3
    }

    // ---- input, sound, random ----------------------------------------------

    /// 0x3b7c8: queue a sound command (16-word ring at 0xff8662). With
    /// attract music off (0x3b7cc, DSW1 bit 9 == 0), commands 0x28..=0x3e
    /// are dropped; in demo play with demo sounds off (0x3b7f2, DSW1 bit 13
    /// != 0), everything is.
    pub fn queue_sound(&mut self, cmd: u16) {
        if self.sound_allowed(cmd) {
            self.queue_sound_unfiltered(cmd);
        }
    }

    fn sound_allowed(&self, cmd: u16) -> bool {
        (self.dsw1 & 0x200 != 0 || !((cmd as i16) >= 0x28 && cmd <= 0x3e))
            && (self.rb(MODE_DEMO) != 0 || self.dsw1 & 0x2000 == 0)
    }

    pub(crate) fn queue_stage_music(&mut self, fallback: u16, cue: Option<crate::theme::ThemeCue>, legacy: u8) {
        if self.sound_allowed(fallback) {
            let idx = usize::from(self.rw(SOUND_WRITE) / 2);
            self.queue_sound_unfiltered(fallback);
            if let Some(slot) = self.toggle_state.queued_themes.get_mut(idx) {
                let event = self.toggle_state.next_theme_event;
                self.toggle_state.next_theme_event = event.wrapping_add(1);
                *slot = Some(crate::tasks::StageMusic { cue, legacy, event });
            }
        }
    }

    /// 0x3b802: queue a sound command into the 16-word ring at 0xff8662
    /// without the attract-music or demo-sounds DIP filters (the tail
    /// 0x3b806 shared with 0x3b7c8).
    pub fn queue_sound_unfiltered(&mut self, cmd: u16) {
        let idx = self.rw(SOUND_WRITE);
        if let Some(slot) = self.toggle_state.queued_themes.get_mut(usize::from(idx / 2)) {
            *slot = None;
        }
        self.ww(SOUND_QUEUE + idx as u32, cmd);
        let next = if idx + 2 == 0x20 { 0 } else { idx + 2 };
        self.ww(SOUND_WRITE, next);
    }

    /// 0x3b12c (vblank): pop one queued command into the sound latch (an
    /// empty slot writes 0xffff, i.e. latch 0xff).
    pub fn pop_sound(&mut self) -> u8 {
        let idx = self.rw(SOUND_READ);
        let cmd = self.rw(SOUND_QUEUE + idx as u32);
        let music = self.toggle_state.queued_themes
            .get_mut(usize::from(idx / 2)).and_then(Option::take);
        self.toggle_state.latched_theme = if cmd != 0xffff { music } else { None };
        if cmd != 0xffff {
            self.ww(SOUND_QUEUE + idx as u32, 0xffff);
            let next = if idx + 2 == 0x20 { 0 } else { idx + 2 };
            self.ww(SOUND_READ, next);
        }
        cmd as u8
    }

    /// 0x3b05a..0x3b0c8 (vblank): run the attack-button latches the scene
    /// and game mode call for — scene mode 2: P1's, 1: P2's; 0x80 / 0xc0 by
    /// game mode (1 both players, 2 P1, 4 P2); otherwise none. Ports are
    /// active low: IN0 high byte bit 1 / 2 = P1 / P2 button 3, IN0 low byte
    /// bit 2 = P1 button 6, IN0 high byte bit 0 = P2 button 6; IN1 bytes:
    /// bits 4/5 buttons 1/2, bits 6/7 buttons 4/5.
    ///
    /// Returns the 68000 cycles of the dispatch and the latch routines
    /// (ticket #35; `tasks::vblank`): each chain step is cmpi.b #,(d16,A5)
    /// 16 and a branch, taken 10 / not taken 12; a latch routine is bsr 18,
    /// its head (0x3b3ae: lea 8, lea 8, move.b 12, move.b 12, not.w 4,
    /// andi.w 8, beq taken 10 without the heavy button; with it, not taken
    /// 12 + move.w #3,D1 8, + bra 10 in the P1 routines) and its body
    /// ([`latch_cycles`]); 0x3b0b4 / 0x3b0c0 end with bra $3b0cc 10, the
    /// P2-only pair falls through.
    pub fn vblank_buttons(&mut self, in0: u16, in1: u16) -> u32 {
        let (in0_hi, in0_lo) = ((in0 >> 8) as u8, in0 as u8);
        let (p2, p1) = ((in1 >> 8) as u8, in1 as u8);
        let group = |raw: u8, mask: u8, shift: u8, heavy: bool| {
            ((!raw & mask) >> shift) | if heavy { 3 } else { 0 }
        };
        let alpha_buttons = |raw: u8, hp: bool, hk: bool| {
            ((!raw & 0x30) >> 4) | (u8::from(hp) << 2)
                | ((!raw & 0xc0) >> 3) | (u8::from(hk) << 5)
        };
        let head = |heavy: bool, pressed_cost: u32| 52 + if heavy { pressed_cost } else { 10 };
        // 0x3b05a: the scene chain to the latch calls at 0x3b0b8 (P1), 0x3b0c4
        // (P2), 0x3b082 (the game-mode chain: 26 to 0x3b0a4 both players, 54
        // to 0x3b0b8, 82 to 0x3b0c4, 94 with bra $3b0cc past them) or 0x3b0cc.
        let scene = self.rb(SCENE_MODE);
        let dramatic = self.dramatic_rules().is_some() && matches!(scene, 0x80 | 0xc0);
        let (do_p1, do_p2, mut cycles) = match scene {
            2 => (true, false, 26),
            1 => (false, true, 54),
            scene @ (0x80 | 0xc0) => {
                let chain = if scene == 0xc0 { 82 } else { 112 };
                match self.rb(GAME_MODE) {
                    1 => (true, true, chain + 26),
                    2 => (true, dramatic, chain + 54),
                    4 => (false, true, chain + 82),
                    _ => (false, false, chain + 94),
                }
            }
            _ => (false, false, 110),
        };
        if do_p1 {
            let (heavy1, heavy2) = (!in0_hi & 2 != 0, !in0_lo & 4 != 0);
            if self.evil_ryu_has_demon(P1_STRUCT) && self.is_active_ryu(P1_STRUCT) {
                self.track_raging_demon(P1_STRUCT, !p1, heavy1, heavy2);
            }
            self.era_read_buttons(P1_STRUCT, alpha_buttons(p1, heavy1, heavy2));
            let path = self.button_latch(0xff8690, P1_STRUCT, group(p1, 0x30, 4, heavy1), 0x3b450);
            cycles += 18 + head(heavy1, 30) + latch_cycles(path, 14);
            let path =
                self.button_latch(0xff8698, P1_STRUCT + 1, group(p1, 0xc0, 6, heavy2), 0x3b506);
            cycles += 18 + head(heavy2, 30) + latch_cycles(path, 18) + 10;
        }
        if do_p2 {
            let (heavy1, heavy2) = (!in0_hi & 4 != 0, !in0_hi & 1 != 0);
            if self.evil_ryu_has_demon(P2_STRUCT) && self.is_active_ryu(P2_STRUCT) {
                self.track_raging_demon(P2_STRUCT, !p2, heavy1, heavy2);
            }
            self.era_read_buttons(P2_STRUCT, alpha_buttons(p2, heavy1, heavy2));
            let path = self.button_latch(0xff86a0, P2_STRUCT, group(p2, 0x30, 4, heavy1), 0x3b450);
            cycles += 18 + head(heavy1, 20) + latch_cycles(path, 14);
            let path =
                self.button_latch(0xff86a8, P2_STRUCT + 1, group(p2, 0xc0, 6, heavy2), 0x3b506);
            cycles += 18 + head(heavy2, 20) + latch_cycles(path, 18);
        } else if matches!(self.rb(SCENE_MODE), 0x80 | 0xc0)
            && self.rb(GAME_MODE) == 2
            && self.arcade_player_opponent().is_some()
        {
            let (heavy1, heavy2) = (!in0_hi & 4 != 0, !in0_hi & 1 != 0);
            if self.evil_ryu_has_demon(CPU_STRUCT) && self.is_active_ryu(CPU_STRUCT) {
                self.track_raging_demon(CPU_STRUCT, !p2, heavy1, heavy2);
            }
            self.era_read_buttons(CPU_STRUCT, alpha_buttons(p2, heavy1, heavy2));
            self.button_latch(0xff86a0, CPU_STRUCT, group(p2, 0x30, 4, heavy1), 0x3b450);
            self.button_latch(0xff86a8, CPU_STRUCT + 1, group(p2, 0xc0, 6, heavy2), 0x3b506);
        }
        if dramatic {
            let buttons = self.toggle_state.dramatic.boss_pad.buttons;
            for (offset, table) in [(0usize, 0x3b450), (3, 0x3b506)] {
                let pressed = if buttons[offset + 2] {
                    3
                } else {
                    u8::from(buttons[offset]) | (u8::from(buttons[offset + 1]) << 1)
                };
                let group = u32::from(offset != 0);
                self.button_latch(BOSS_LATCHES + group * 8, CPU_STRUCT + group, pressed, table);
            }
        }
        cycles
    }

    const DEMON_SEQUENCE: [u8; 5] = [0x10, 0x10, 0x01, 0x40, 0x08];

    fn track_raging_demon(&mut self, ps: u32, input: u8, heavy_punch: bool, heavy_kick: bool) {
        let me = self.rw(ps + 0x10);
        let profile = self.evil_ryu_profile(ps);
        let forward = self.relative_directions(me, input & 0x0f) == 1;
        // Keep raw press edges so the second LP may arrive during the first jab.
        let held = input & 0xf0 | u8::from(heavy_punch) << 3 | u8::from(heavy_kick) << 1 | u8::from(forward);
        let demon = &mut self.toggle_state.raging_demon;
        let pressed = held & !demon.held;
        demon.held = held;
        if demon.progress != 0 {
            demon.age += 1;
            if demon.age >= 60 {
                demon.progress = 0;
                demon.age = 0;
            }
        }
        if pressed == 0 || usize::from(demon.progress) == Self::DEMON_SEQUENCE.len() {
            return;
        }
        // Kage also accepts forward and LK as a single chord.
        if matches!(profile, Some(EvilRyuPlaytest::KageII | EvilRyuPlaytest::Sf6))
            && demon.progress == 2 && pressed == 0x41
        {
            demon.progress += 2;
        } else if pressed == Self::DEMON_SEQUENCE[usize::from(demon.progress)] {
            demon.progress += 1;
        } else {
            demon.progress = u8::from(pressed == Self::DEMON_SEQUENCE[0]);
            demon.age = 0;
        }
    }

    /// 0x3b10a (vblank): copy IN1 (active low; P2 high byte, P1 low byte).
    pub fn latch_in1(&mut self, in1: u16) {
        self.ww(IN1_COPY, in1);
    }
}

/// The branch a button-latch routine's body (0x3b3ee; 0x3b4a4 for the
/// second group) took, for the vblank handler's cycle accounting (ticket
/// #35).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatchPath {
    /// No group state and nothing pressed.
    Idle,
    /// No group state and a button pressed: the state opens.
    FirstPress,
    /// Held: the count steps unless saturated; a stronger button is kept.
    Held { saturated: bool, stronger: bool },
    /// Released: the strength is written unless the count saturated.
    Release { saturated: bool },
}

/// The body's cycles: not.w 4, andi.w 8, lsr.w `lsr` (#4: 14; #6: 18), or
/// 4, cmpi.b #0,($0,A0) 16, beq (no state: taken 10); then the branch —
/// idle: 0x3b454 or.b 4, beq.s taken 10, rts 16; first press: beq.s not
/// taken 8, move.b #$ff,($0,A0) 16, move.b D0,($4,A0) 12, bra.s 10, rts 16;
/// held (0x3b402): or.b 4, beq not taken 12, cmpi.b #-1,($3,A0) 16 and beq
/// taken 10 or not taken 12 + addq.b 16, cmp.b ($4,A0),D0 12 and bls taken
/// 10 or not taken 12 + move.b 12 + bra 10, rts 16; release (0x3b426):
/// or.b 4, beq taken 10, cmpi.b 16 and beq taken 10 or not taken 12 +
/// move.b ($4,A0),D0 12 + ext.w 4 + subq 4 + move.b (d8,PC,D0.w),D0 14 +
/// move.b D0,(A1) 12, then clr.w 4 and three move.b 12, rts 16.
pub fn latch_cycles(path: LatchPath, lsr: u32) -> u32 {
    let base = 32 + lsr;
    base + match path {
        LatchPath::Idle => 10 + 4 + 10 + 16,
        LatchPath::FirstPress => 10 + 4 + 8 + 16 + 12 + 10 + 16,
        LatchPath::Held {
            saturated,
            stronger,
        } => {
            12 + 4 + 12 + if saturated { 26 } else { 44 } + 12 + if stronger { 34 } else { 10 } + 16
        }
        LatchPath::Release { saturated } => 12 + 4 + 10 + if saturated { 26 } else { 74 } + 40 + 16,
    }
}

/// A bank jump stays busy through its CPU falling phase (0x82), which
/// the ROM player gate would otherwise treat as an interruptible walk.
pub(crate) const fn busy_banked_jump(state: u8, move_id: u8, table: u32) -> bool {
    state & 0x80 != 0 && matches!(move_id, 9..=0x0b) && crate::data_bank::in_bank(table)
}

impl Bus<'_> {
    /// 0x3b3ae family (vblank): the six-button cabinet's emulation of the
    /// pressure-sensing pads. While any of a group's three buttons is held,
    /// count frames (+3, saturating at 0xff) and keep the strongest (+4: 1
    /// light, 2 medium, 3 heavy); on release, unless the count saturated,
    /// write the pad-equivalent strength from `table` (0x40 / 0xe0 / 0xfe)
    /// to the player's latch byte `out`. Returns the branch taken, for the
    /// cycle accounting.
    pub fn button_latch(&mut self, state: u32, out: u32, pressed: u8, table: u32) -> LatchPath {
        let pneumatic = self.player_slot_index(out & !1).is_some_and(|side| {
            self.toggle_state.schemes[side].mode == crate::session_io::ControlScheme::Pneumatic
        });
        if pneumatic {
            // The pressure translator has already resolved the pad gesture
            // to one cabinet-strength impulse. Do not delay it again or
            // generate a second attack on the following release.
            if pressed != 0 {
                self.wb(out, self.rb(table + u32::from(pressed - 1)));
                self.set_neg_edge_latch(out, false);
                return LatchPath::FirstPress;
            }
            return LatchPath::Idle;
        }
        if self.era_offense_flag(out & !1, self.toggles.positive_edge_attacks) {
            if self.rb(state) == 0 {
                if pressed != 0 {
                    self.wb(state, 0xff);
                    self.wb(state + 3, 0);
                    self.wb(state + 4, pressed);
                    let v = self.rb(table + pressed.wrapping_sub(1) as u32);
                    self.wb(out, v);
                    self.set_neg_edge_latch(out, false);
                    return LatchPath::FirstPress;
                }
                return LatchPath::Idle;
            }
            if pressed != 0 {
                let count = self.rb(state + 3);
                if count < 0xfe {
                    self.wb(state + 3, count + 1);
                }
                let stronger = pressed > self.rb(state + 4);
                if stronger {
                    self.wb(state + 4, pressed);
                    let v = self.rb(table + pressed.wrapping_sub(1) as u32);
                    self.wb(out, v);
                    self.set_neg_edge_latch(out, false);
                }
                return LatchPath::Held {
                    saturated: false,
                    stronger,
                };
            }
            if self.toggles.negative_edge_specials
                && matches!(self.rb(SCENE_MODE), 0x80 | 0xc0)
                && self.rb(out) == 0
            {
                let strongest = self.rb(state + 4);
                if strongest != 0 {
                    let v = self.rb(table + strongest.wrapping_sub(1) as u32);
                    self.wb(out, v);
                    self.set_neg_edge_latch(out, true);
                }
            }
            self.wb(state + 3, 0);
            self.wb(state, 0);
            self.wb(state + 4, 0);
            return LatchPath::Release { saturated: false };
        }
        if self.rb(state) == 0 {
            if pressed != 0 {
                self.wb(state, 0xff);
                self.wb(state + 4, pressed);
                return LatchPath::FirstPress;
            }
            return LatchPath::Idle;
        }
        if pressed != 0 {
            let saturated = self.rb(state + 3) == 0xff;
            if !saturated {
                self.wb(state + 3, self.rb(state + 3) + 1);
            }
            let stronger = pressed > self.rb(state + 4);
            if stronger {
                self.wb(state + 4, pressed);
            }
            return LatchPath::Held {
                saturated,
                stronger,
            };
        }
        let saturated = self.rb(state + 3) == 0xff;
        if !saturated {
            let strongest = self.rb(state + 4);
            let v = self.rb(table + strongest.wrapping_sub(1) as u32);
            self.wb(out, v);
        }
        self.wb(state + 3, 0);
        self.wb(state, 0);
        self.wb(state + 4, 0);
        LatchPath::Release { saturated }
    }

    /// 0x429f2 / 0x42a00: the two player structures in this frame's object
    /// run order, from the game mode's pair table (2P versus 0x42a3e, 1P vs
    /// CPU 0x42a88, 2P-side vs CPU 0x42ac0) at the random-parity offset.
    pub fn dispatch_order(&self) -> [u32; 2] {
        let idx = self.rb(RNG_STATE + 2) & 0xf;
        let d0 = (self.rb(0x42a4e + idx as u32) as u32) << 3;
        let table = match self.rb(GAME_MODE) {
            2 => 0x42a88,
            4 => 0x42ac0,
            _ => 0x42a3e,
        };
        [self.rl(table + d0), self.rl(table + d0 + 4)]
    }

    /// 0x42a1c..0x42a36: run one player's objects (its fighter's runner).
    pub fn run_player(&mut self, ps: u32) {
        let (me, other) = self.player_entities(ps);
        self.run_fighter(me, other);
    }

    pub(crate) fn dispatch_fighters(&mut self) {
        // Count once before either fighter runs so the slam is released on
        // the same tick whichever fighter the dispatcher visits first.
        if let Some(remaining) = self.toggle_state.raging_demon.blackout_frames {
            let remaining = remaining.saturating_sub(1);
            self.toggle_state.raging_demon.blackout_frames = Some(remaining);
            if remaining != 0 && remaining % DEMON_SPARK_INTERVAL == 0 {
                let owner = self.toggle_state.satsui_awakening.map_or(P1_STRUCT, |form| form.side.fighter_struct());
                let (_, victim) = self.player_entities(owner);
                self.spawn_demon_spark(victim);
            }
        }
        if self.dramatic_rules().is_some() {
            self.bind_dramatic_boss_target();
            for ps in DRAMATIC_PLAYERS {
                self.run_player(ps);
            }
            return;
        }
        for ps in self.dispatch_order() {
            self.run_player(ps);
        }
    }

    pub(crate) fn position_dramatic_ally(&mut self) {
        let (p1, _) = self.player_entities(P1_STRUCT);
        let left = sf_video::render::VISAREA_X as i16 + 16;
        let x = (self.center_x(p1) as i16).saturating_sub(36).max(left) as u16;
        self.shift_entity_x(DRAMATIC_ALLY2, x.wrapping_sub(self.center_x(DRAMATIC_ALLY2)));
    }

    /// 0x438ea: the stage layer's slow drift — every fifth frame the drift
    /// position advances by one (wrapping 0x200 back to 0) and bg_scroll
    /// follows it (0x3b682).
    pub fn drift_stage(&mut self) {
        let n = self.rb(DRIFT_COUNT);
        if n != 4 {
            self.wb(DRIFT_COUNT, n.wrapping_add(1));
            return;
        }
        self.wb(DRIFT_COUNT, 0);
        let d0: u16 = if self.rw(DRIFT_POS) == 0x200 {
            0x200u16.wrapping_neg()
        } else {
            1
        };
        self.ww(DRIFT_POS, self.rw(DRIFT_POS).wrapping_add(d0));
        let bg = self.rw(BG_SCROLL_COPY).wrapping_add(d0);
        self.ww(BG_SCROLL_COPY, bg);
        *self.bg_scroll = bg;
    }

    /// 0x40bd4: the CPU opponent's decision for this frame. A busy fighter
    /// (+9 bit 7) or one still waiting (+0x1a) does nothing; otherwise it
    /// first turns to face the opponent (a turn takes the whole frame).
    /// A stunned fighter (+9 bit 3) picks from the stunned table with random
    /// byte 2. Otherwise, unless the range roll is held (+0x1b == 0x80), a
    /// new preferred range is rolled from random byte 0 (class table 0x4058,
    /// range words 0x4068; +0x18 class, +0x16 range); the distance from that
    /// range selects the near / middle / far move table (limits 0x406e /
    /// 0x4070), random byte 1 picks the move, and being closer than the
    /// preferred range selects the next move id. The move starts at 0x441b0.
    pub fn cpu_decide(&mut self) {
        let a1 = CPU_STRUCT;
        let (me, other) = self.player_entities(a1);
        if self.eb(me, 9) & 0x80 != 0 || self.era_guard_crushed(a1) {
            return;
        }
        let wait = self.eb(me, 0x1a);
        if wait != 0 {
            self.set_eb(me, 0x1a, wait - 1);
            return;
        }
        let my_cx = self.center_x(me);
        let ot_cx = self.center_x(other);
        if ot_cx > my_cx {
            if self.eb(me, 0x37) != 2 {
                self.set_eb(me, 0x37, 2);
                self.face_right(me);
                self.set_eb(me, 0x1a, 0);
                return;
            }
        } else if ot_cx != my_cx && self.eb(me, 0x37) != 4 {
            self.set_eb(me, 0x37, 4);
            self.face_left(me);
            self.set_eb(me, 0x1a, 0);
            return;
        }
        if self.eb(me, 8) == 8 {
            return;
        }
        if self.eb(me, 9) & 8 != 0 {
            let i = ((self.rb(RNG_STATE + 2) & 0xf) as u32) << 1;
            let mv = self.rb(AI_STUNNED_MOVES + i);
            self.wb(a1 + 0x19, self.rb(AI_STUNNED_MOVES + i + 1));
            self.start_move(me, a1, mv);
            return;
        }
        if self.rb(a1 + 0x1b) != 0x80 {
            let class = self.rb(AI_RANGE_CLASS + (self.rb(RNG_STATE) & 0xf) as u32);
            self.wb(a1 + 0x18, class);
            self.ww(a1 + 0x16, self.rw(AI_RANGE + 2 * class as u32));
        }
        self.wb(a1 + 0x1b, 0);
        let dist = ot_cx.abs_diff(my_cx);
        let range = self.rw(a1 + 0x16) & 0xff;
        let (mut d0, mut closer) = (dist.wrapping_sub(range), 0u8);
        if dist < range {
            d0 = d0.wrapping_neg();
            closer = 1;
        }
        let table = if d0 <= self.rw(AI_NEAR_LIMIT) {
            AI_NEAR_MOVES
        } else if d0 <= self.rw(AI_MID_LIMIT) {
            AI_MID_MOVES
        } else {
            AI_FAR_MOVES
        };
        let pick = self.rb(RNG_STATE + 1) & 0xf;
        let mv = self.rb(table + pick as u32).wrapping_add(closer);
        self.start_move(me, a1, mv);
    }

    /// 0x44880: the copies that set up the CPU opponent's AI tables, as
    /// (source in ROM, destination in work RAM, words). The opponent index
    /// is (stage 0xffc003 * 2 + fight 0xffc004) * 4 into the per-opponent
    /// pointer arrays A5+0x4020..0x404c (filled for the starting country
    /// by 0x3e38a); rows are chosen by the difficulty 0xffc007 (D1) and the
    /// variant bit 1 of 0xffc006 (D2):
    ///
    /// | Pointer | Destination | Words | Row offset |
    /// | --- | --- | --- | --- |
    /// | 0x4020 | 0x4068 preferred ranges | 3 | - |
    /// | 0x4024 | 0x4058 range classes | 8 | D1 * 16 + D2 * 64 |
    /// | 0x4028 | 0x406e distance limits | 3 | - |
    /// | 0x402c | 0x4074 near / middle / far moves | 24 | 0, 0x30, 0x60, 0x90 by D1, + 0xc0 for D2 unless D1 is 0 |
    /// | 0x4030 | 0x40a4 stunned moves | 16 | D1 * 32 + D2 * 128 |
    /// | 0x4034 / 0x4038 | 0x40c4 / 0x40d4 range-shift and react rolls | 8 each | D1 * 16 + D2 * 64 |
    /// | 0x403c / 0x4040 | 0x40e4 / 0x4164 reactions to airborne / ground attacks | 64 each | D1 * 128 + D2 * 512 |
    /// | 0x4044 / 0x4048 / 0x404c | 0x41e4 / 0x4364 / 0x44e4 attack records | 192 each | D1 * 0x180 + (D2 ? 0x600 : 0) |
    ///
    /// At difficulty 0 the move tables' variant offset is skipped (the
    /// branch at 0x448f0 bypasses the D2 test); the translation keeps it.
    pub fn ai_table_copies(&self) -> Vec<(u32, u32, u32)> {
        let d0 = (((self.rb(0xffc003) as u16) << 1) & 0xff00)
            | ((((self.rb(0xffc003) as u16) << 1) as u8).wrapping_add(self.rb(0xffc004))) as u16;
        let idx = (d0 << 2) as u32;
        let d1 = self.rb(0xffc007) as u32;
        let d2 = ((self.rb(0xffc006) >> 1) & 1) as u32;
        let src = |ptr: u32| self.rl(self.rl(ptr).wrapping_add(idx));
        let moves_row = {
            let (mut n, mut off) = (d1 as u8, 0u32);
            if n.checked_sub(1).is_some() {
                n -= 1;
                off = 0x30;
                for _ in 0..2 {
                    if n.checked_sub(1).is_none() {
                        break;
                    }
                    n -= 1;
                    off += 0x30;
                }
                if d2 == 1 {
                    off += 0xc0;
                }
            }
            off
        };
        let r16 = d1 * 16 + d2 * 64;
        let r128 = d1 * 128 + d2 * 512;
        let r180 = d1 * 0x180 + if d2 != 0 { 0x600 } else { 0 };
        vec![
            (src(0xffc020), 0xffc068, 3),
            (src(0xffc024) + r16, 0xffc058, 8),
            (src(0xffc028), 0xffc06e, 3),
            (src(0xffc02c) + moves_row, 0xffc074, 24),
            (src(0xffc030) + d1 * 32 + d2 * 128, 0xffc0a4, 16),
            (src(0xffc034) + r16, 0xffc0c4, 8),
            (src(0xffc038) + r16, 0xffc0d4, 8),
            (src(0xffc03c) + r128, 0xffc0e4, 64),
            (src(0xffc040) + r128, 0xffc164, 64),
            (src(0xffc044) + r180, 0xffc1e4, 192),
            (src(0xffc048) + r180, 0xffc364, 192),
            (src(0xffc04c) + r180, 0xffc4e4, 192),
        ]
    }

    /// 0x44880 (with 0x44860): set up the CPU opponent for the current
    /// stage and fight: its AI tables (`ai_table_copies`, copied a word at
    /// a time by 0x44a38) and its move table (CPU structure +0x12, from the
    /// pointer array at A5+0x401c).
    pub fn setup_cpu_opponent(&mut self) {
        let d0 = ((self.rb(0xffc003) as u32) << 1).wrapping_add(self.rb(0xffc004) as u32) << 2;
        let player_opponent = self.dramatic_rules().map(|_| self.roster.p2)
            .or_else(|| self.arcade_player_opponent_at_round_start());
        let moves = player_opponent
            .map_or_else(|| self.rl(self.rl(0xffc01c).wrapping_add(d0)), |opp| self.player_table(opp));
        self.wl(CPU_STRUCT + 0x12, moves);
        if player_opponent.is_some() {
            self.wl(CPU_STRUCT + 2, 0xff868e);
            self.ww(CPU_STRUCT, 0);
            self.clear_all_trackers(CPU_STRUCT);
        }
        for (src, dst, words) in self.ai_table_copies() {
            for i in 0..words {
                let v = self.rw(src + 2 * i);
                self.ww(dst + 2 * i, v);
            }
        }
        if self.toggles.fair_cpu_and_sagat_scaling && self.rb(0xffc003) == 4 {
            self.apply_thailand_ai_scaling();
        }
    }

    /// 0x4813e: one pass of the random-state task: fold the inverted input
    /// byte of the player at +2 of P2's structure (P1's in mode 2) into the
    /// 8-byte state with the fixed increments 3,3,5,7,9,11,13,15.
    pub fn step_random(&mut self) {
        let ptr = if self.rb(GAME_MODE) == 2 {
            self.rl(P1_STRUCT + 2)
        } else {
            self.rl(P2_STRUCT + 2)
        };
        let mut d0 = !self.rb(ptr);
        for (i, add) in [3u8, 3, 5, 7, 9, 11, 13, 15].into_iter().enumerate() {
            d0 = d0
                .wrapping_add(self.rb(RNG_STATE + i as u32))
                .wrapping_add(add);
            self.wb(RNG_STATE + i as u32, d0);
        }
    }

    // ---- the player input handler (0x402f8) ---------------------------------

    /// True when `me` is a human-controlled CPU fighter under the full cast (`#98`, `O7`).
    fn human_controlled_boss(&self, me: u16) -> bool {
        if !self.roster.full_cast {
            return false;
        }
        let ps = self.el(me, 0x38);
        self.boss_on_struct(ps)
    }

    /// The opponent the Arcade ladder brings for the current stage and
    /// fight (`0xffc003`, `0xffc004` through `0xffc01c`): under the full
    /// cast outside demo play, Ryu stands in when the ladder slot is the
    /// human player's own fighter (SP14, `#266`).
    pub(crate) fn arcade_opponent(&self) -> Option<crate::roster::TournamentFighter> {
        let base = self.rl(0xffc01c);
        if base == 0 {
            return None;
        }
        let d0 = (((self.rb(0xffc003) as u32) << 1).wrapping_add(self.rb(0xffc004) as u32)) << 2;
        let slot = crate::roster::TournamentFighter::from_move_table(self.rl(base.wrapping_add(d0)))?;
        if self.roster.full_cast && self.rb(MODE_DEMO) != 0 && slot == self.roster.p1 {
            Some(crate::roster::TournamentFighter::Ryu)
        } else {
            Some(slot)
        }
    }

    /// The Arcade CPU opponent when he plays on a 1987 player table
    /// (`CPU_STRUCT` driven by `input_handler`): Ryu (standing in), or a
    /// boss whose 1987 player table is in the machine's data bank.
    pub(crate) fn arcade_player_opponent(&self) -> Option<crate::roster::TournamentFighter> {
        let opp = self.banked_cpu_opponent()?;
        if self.toggle_state.scripted_cpu_pad {
            return Some(opp);
        }
        let native = self.era_state(CPU_STRUCT)
            .map_or_else(|| self.arcade_cpu_family(opp), |state| state.style.family);
        (native != crate::era::EraFamily::Sf1987).then_some(opp)
    }

    fn banked_cpu_opponent(&self) -> Option<crate::roster::TournamentFighter> {
        if self.rb(MODE_DEMO) == 0 || !matches!(self.rb(GAME_MODE), 2 | 4) {
            return None;
        }
        let opp = self.arcade_opponent()?;
        (opp.is_shoto() || self.data_bank.and_then(|b| b.table(opp)).is_some()).then_some(opp)
    }

    fn arcade_player_opponent_at_round_start(&self) -> Option<crate::roster::TournamentFighter> {
        let opp = self.banked_cpu_opponent()?;
        (self.roster.full_cast && self.era_rules.enabled
            && self.arcade_cpu_family(opp) != crate::era::EraFamily::Sf1987).then_some(opp)
    }

    fn arcade_cpu_family(&self, fighter: crate::roster::TournamentFighter) -> crate::era::EraFamily {
        if !self.roster.full_cast || !self.era_rules.enabled {
            return crate::era::EraFamily::Sf1987;
        }
        let picks = self.toggle_state.era.selections;
        if self.roster.mode == crate::roster::PlayMode::Arcade {
            let step = usize::from(self.rb(0xffc003)) * 2 + usize::from(self.rb(0xffc004));
            picks[0].arcade_opponent(fighter, self.roster.what_if_styles, step).family
        } else {
            picks[1].family
        }
    }

    /// Whether `ps` is driven by the 1987 player handler (`P1_STRUCT`,
    /// `P2_STRUCT`, or `CPU_STRUCT` when the Arcade opponent plays on a
    /// 1987 player table).
    fn is_player_driven(&self, ps: u32) -> bool {
        matches!(ps, P1_STRUCT | P2_STRUCT)
            || (ps == CPU_STRUCT && (self.dramatic_rules().is_some() || self.arcade_player_opponent().is_some()))
    }

    /// Where command trackers (`+0x16`, `+0x19`, `+0x1c`) live for `ps`:
    /// `CPU_STRUCT` (`0xffc664..0xffc680`) is only `0x1c` bytes long, so a
    /// player-driven Arcade CPU opponent keeps its command trackers in
    /// `P2_STRUCT` (unused in mode 2).
    fn tracker_base(&self, ps: u32) -> u32 {
        match ps {
            CPU_STRUCT if self.dramatic_rules().is_some() => BOSS_TRACKERS - 0x16,
            CPU_STRUCT => P2_STRUCT,
            _ => ps,
        }
    }

    /// True when the player structure is bound to one of the ten CPU fighters.
    pub fn boss_on_struct(&self, ps: u32) -> bool {
        if let Some(rules) = self.dramatic_rules() {
            return match ps {
                P1_STRUCT => !self.roster.p1.is_shoto(),
                CPU_STRUCT => !self.roster.p2.is_shoto(),
                P2_STRUCT => !rules.ally2.is_shoto(),
                _ => false,
            };
        }
        if !self.roster.full_cast {
            return false;
        }
        let fighter = if ps == P1_STRUCT {
            self.roster.p1
        } else if ps == P2_STRUCT {
            self.roster.p2
        } else if ps == CPU_STRUCT {
            let Some(opp) = self.arcade_player_opponent() else {
                return false;
            };
            opp
        } else {
            return false;
        };
        !fighter.is_shoto()
    }

    /// Whether the human side `ps` takes Ken's per-player tables (hit,
    /// knock-out, jump attack) rather than Ryu's. The 1987 game keys them
    /// by side, Ken's on P2. Roster-bound shotos in full-cast and Dramatic
    /// fights use their own tables on every slot.
    fn kens_side(&self, ps: u32) -> bool {
        let ken = crate::roster::TournamentFighter::Ken.move_table();
        let ryu = crate::roster::TournamentFighter::Ryu.move_table();
        let roster_bound = self.roster.full_cast || self.dramatic_rules().is_some();
        match self.rl(ps + 0x12) {
            t if roster_bound && t == ken => true,
            t if roster_bound && t == ryu => false,
            _ => ps == P2_STRUCT,
        }
    }

    /// The hit and knock-out tables of a boss the full cast binds to the
    /// human side `ps`, by the move table bound there.
    fn bound_boss_tables(&self, ps: u32) -> Option<(u32, u32)> {
        if !self.roster.full_cast {
            return None;
        }
        let fighter = crate::roster::TournamentFighter::from_move_table(self.rl(ps + 0x12))?;
        fighter.reaction_tables()
    }

    /// Bind each human side's move table to its selected fighter under the
    /// full cast, as [`drive_fight_inputs`](Self::drive_fight_inputs) does
    /// every frame: the round's placement (`0x3e714`) starts move 0, the
    /// walk-in, from these tables before the first frame of input.
    pub fn bind_roster_tables(&mut self) {
        for (ps, fighter) in self.roster_sides() {
            let table = self.player_table(fighter);
            self.wl(ps + 0x12, table);
        }
    }

    /// The move table a human side plays `fighter` on: his 1987 player
    /// table in the extended data bank when this machine has one for him
    /// (ADR-0014), else his ROM table.
    fn player_table(&self, fighter: crate::roster::TournamentFighter) -> u32 {
        self.data_bank
            .and_then(|b| b.table(fighter))
            .unwrap_or(fighter.move_table())
    }

    /// The human sides the full cast binds, with their fighters (none
    /// without it), plus `CPU_STRUCT` when the Arcade opponent plays on a
    /// 1987 player table (SP14, `#266`).
    fn roster_sides(&self) -> Vec<(u32, crate::roster::TournamentFighter)> {
        if let Some(rules) = self.dramatic_rules() {
            return vec![(P1_STRUCT, self.roster.p1), (CPU_STRUCT, self.roster.p2), (P2_STRUCT, rules.ally2)];
        }
        if !self.roster.full_cast {
            return Vec::new();
        }
        match self.rb(GAME_MODE) {
            2 => {
                let mut sides = vec![(P1_STRUCT, self.roster.p1)];
                if let Some(opp) = self.arcade_player_opponent_at_round_start() {
                    sides.push((CPU_STRUCT, opp));
                }
                sides
            }
            4 => {
                let mut sides = vec![(P2_STRUCT, self.roster.p1)];
                if let Some(opp) = self.arcade_player_opponent_at_round_start() {
                    sides.push((CPU_STRUCT, opp));
                }
                sides
            }
            _ => vec![(P1_STRUCT, self.roster.p1), (P2_STRUCT, self.roster.p2)],
        }
    }

    /// Where the round's placement (`0x3e714`) starts the walk-in of a
    /// fighter the full cast binds to a side the 1987 game gives another:
    /// the target `x` and facing for [`start_move_at`](Self::start_move_at),
    /// or `None` to keep the 1987 placement.
    /// - Ryu's and Ken's move 0 start from an absolute x in the header,
    ///   Ryu's on the left (P1), Ken's on the right (P2). On the other side
    ///   a shoto starts from the mirror of his own, facing in.
    /// - A boss's move 0 is placed as the 1987 game places the CPU on that
    ///   side: `0x40` facing right (mode 4), `0x1c0` facing left (mode 2).
    pub fn roster_walk_in(&self, ps: u32) -> Option<(u16, u8)> {
        use crate::roster::TournamentFighter as T;
        let (_, fighter) = self
            .roster_sides()
            .into_iter()
            .find(|&(side, _)| side == ps)?;
        let left = ps == P1_STRUCT;
        if fighter == [T::Ken, T::Ryu][usize::from(left)] {
            return None;
        }
        let facing = if left { 2 } else { 4 };
        if !fighter.is_shoto() {
            return Some((if left { 0x40 } else { 0x1c0 }, facing));
        }
        // The two headers' x mirror each other about the stage's centre.
        let table = fighter.move_table();
        let sx = self.rw(table + 6);
        let mirror_sum = self
            .rw(T::Ryu.move_table() + 6)
            .wrapping_add(self.rw(T::Ken.move_table() + 6));
        let edge = mirror_sum.wrapping_sub(sx);
        let half = Self::half_extent(self.rb(table + 0xd), (self.rw(table + 2) as i16) >= 0x400);
        // start_move_at's left edge: the header's x (negated facing left)
        // plus the target less half the grid.
        let x = if left {
            edge.wrapping_sub(sx)
        } else {
            edge.wrapping_add(sx)
        };
        Some((x.wrapping_add(half), facing))
    }

    /// One frame of fight input. Faithful play keeps the original game-mode
    /// dispatch. Full-cast play binds each human side to its selected fighter.
    pub fn drive_fight_inputs(&mut self) {
        if self.dramatic_rules().is_some() {
            self.bind_dramatic_boss_target();
        }
        if self.toggles.true_combos || self.era_rules.enabled {
            self.combo_frame();
        } else {
            for frames in &mut self.toggle_state.combo.blockstun {
                *frames = frames.saturating_sub(1);
            }
        }
        if self.toggles.throws || self.era_rules.enabled {
            self.throw_frame();
        }
        if let Some(rules) = self.dramatic_rules() {
            for (slot, fighter) in [self.roster.p1, self.roster.p2, rules.ally2].into_iter().enumerate() {
                if self.toggle_state.dramatic.health[slot] != 0 {
                    self.drive_roster_side(DRAMATIC_PLAYERS[slot], fighter);
                }
            }
            return;
        }
        if !self.roster.full_cast {
            match self.rb(GAME_MODE) {
                2 => {
                    self.input_handler(P1_STRUCT);
                    if let Some(opp) = self.arcade_player_opponent() {
                        self.wl(CPU_STRUCT + 2, IN1_COPY);
                        self.drive_roster_side(CPU_STRUCT, opp);
                    } else {
                        self.cpu_decide();
                    }
                }
                4 => {
                    self.input_handler(P2_STRUCT);
                    if let Some(opp) = self.arcade_player_opponent() {
                        self.wl(CPU_STRUCT + 2, IN1_COPY);
                        self.drive_roster_side(CPU_STRUCT, opp);
                    } else {
                        self.cpu_decide();
                    }
                }
                _ => {
                    self.input_handler(P1_STRUCT);
                    self.input_handler(P2_STRUCT);
                }
            }
            return;
        }
        match self.rb(GAME_MODE) {
            2 => {
                self.drive_roster_side(P1_STRUCT, self.roster.p1);
                if let Some(opp) = self.arcade_player_opponent() {
                    self.wl(CPU_STRUCT + 2, 0xff868e);
                    self.drive_roster_side(CPU_STRUCT, opp);
                } else {
                    self.cpu_decide();
                }
            }
            4 => {
                // Mode 4 is the original single-human side (Ken's struct).
                // The selected arcade fighter is bound there without a versus join.
                self.drive_roster_side(P2_STRUCT, self.roster.p1);
                if let Some(opp) = self.arcade_player_opponent() {
                    self.wl(CPU_STRUCT + 2, 0xff868e);
                    self.drive_roster_side(CPU_STRUCT, opp);
                } else {
                    self.cpu_decide();
                }
            }
            _ => {
                self.drive_roster_side(P1_STRUCT, self.roster.p1);
                self.drive_roster_side(P2_STRUCT, self.roster.p2);
            }
        }
    }

    fn drive_roster_side(&mut self, ps: u32, fighter: crate::roster::TournamentFighter) {
        let table = self.player_table(fighter);
        if self.rl(ps + 0x12) != table {
            self.wl(ps + 0x12, table);
        }
        self.input_handler(ps);
    }

    pub(crate) fn pad_directions(&self, ps: u32) -> u8 {
        if ps == CPU_STRUCT && self.dramatic_rules().is_some() {
            self.toggle_state.dramatic.boss_pad.directions.into_iter().enumerate()
                .fold(0, |bits, (bit, held)| bits | (u8::from(held) << bit))
        } else {
            !self.rb(self.rl(ps + 2)) & 0xf
        }
    }

    /// 0x402f8: one frame of a human player's input handler. In attract-demo
    /// play (`MODE_DEMO == 0`, `0x4030c..0x40334`), copies the 3-byte record
    /// at `0x3cebf + [0xff8f2a]` into the player's input byte (`(A1)` =
    /// `[ps + 2]`) and P1's punch/kick latches (`0xff86b0` / `0xff86b1`) and
    /// advances `0xff8f2a` by 3 (wrapping to 0 when `>= [0x3cebc]` = `0x0fff`).
    /// Then reads the player's input byte (active low, bits 0-3 = right, left,
    /// down, up), runs the special-move command trackers, gates a busy fighter,
    /// keeps it facing the opponent, and dispatches on the held direction
    /// through the table at 0x404cc. The tail (0x4050c) drops latched strengths
    /// while an attack is in progress.
    pub fn input_handler(&mut self, ps: u32) {
        let (me, other) = self.player_entities(ps);
        let a1 = self.rl(ps + 2);
        if self.rb(MODE_DEMO) == 0 {
            let mut d0 = self.rw(0xff8f2a);
            let rec = (0x3cebfi32 + d0 as i16 as i32) as u32;
            self.wb(a1, self.rb(rec));
            self.wb(P1_STRUCT, self.rb(rec + 1));
            self.wb(P1_STRUCT + 1, self.rb(rec + 2));
            d0 = d0.wrapping_add(3);
            if (d0 as i16) >= (self.rw(0x3cebc) as i16) {
                d0 = 0;
            }
            self.ww(0xff8f2a, d0);
        }
        let d0 = self.pad_directions(ps);
        if self.eb(me, 8) == 8 {
            self.clear_recovery_buffer(ps);
            self.input_tail(me, ps);
            return;
        }
        if self.era_guard_crushed(ps) || self.era_dizzy(ps) || self.era_holding_crumple(ps) || self.sf6_wall_stun(ps) {
            if self.era_dizzy(ps) && matches!(self.eb(me, 0xa), 2..=4 | 8) {
                self.start_move(me, ps, 1);
            }
            self.ww(ps, 0);
            self.clear_neg_edge_latches_for_player(ps);
            self.clear_recovery_buffer(ps);
            self.input_tail(me, ps);
            return;
        }
        let legacy_evil_ryu = self.legacy_evil_ryu(ps) && self.is_active_ryu(ps);
        let has_demon = self.evil_ryu_has_demon(ps);
        let mv = self.eb(me, 0xa);
        if self.evil_ryu_profile(ps).is_some()
            && ((self.legacy_evil_ryu(ps) && self.is_ashura_move(ps, mv))
                || (has_demon && (mv == self.demon_rush_move(ps)
                || self.toggle_state.raging_demon.blackout_frames.is_some()
                || self.toggle_state.raging_demon.victory_pose)))
        {
            self.toggle_state.ashura_motion = [[0; 2]; 2];
            self.clear_recovery_buffer(ps);
            self.ww(ps, 0);
            self.clear_neg_edge_latches_for_player(ps);
            self.input_tail(me, ps);
            return;
        }
        let grounded = !self.era_airborne(me) && (self.eb(me, 9) & 3 != 1 || matches!(mv, 2 | 3));
        let ashura_ready = legacy_evil_ryu && grounded && self.eb(me, 8) == 2
            && !self.is_in_recovery_or_stun(me) && self.toggle_state.combo.blockstun[self.combo_slot(me)] == 0;
        if ashura_ready {
            self.track_ashura(me, d0);
        } else if self.evil_ryu_profile(ps).is_some() {
            self.toggle_state.ashura_motion = [[0; 2]; 2];
        }
        if has_demon && self.is_active_ryu(ps) && self.try_raging_demon(ps, me, other, grounded) {
            self.input_tail(me, ps);
            return;
        }
        let era_cancels = self.era_offense_flag(ps, self.toggles.cancels);
        let era_throws = self.era_offense_flag(ps, self.toggles.throws);
        let mode = self.rb(SCENE_MODE);
        let scripted_turn = mode == 0x40 || mode == 0xc0;
        let pure_footsies = self.world_tour_pure_footsies();
        if !scripted_turn {
            self.track_commands(ps, me, d0);
            let relative = self.relative_directions(me, d0);
            self.era_track_motion(ps, relative);
            if self.try_family_input(ps, me, relative) || (!pure_footsies && self.try_alpha_counter(ps, me, relative)) {
                self.input_tail(me, ps);
                return;
            }
        }
        if self.era_offense_flag(ps, self.toggles.positive_edge_attacks) {
            self.filter_neg_edge_latches(ps, me, d0);
        }
        // Cancels (SP18): in a cancelable normal's window, a special's
        // command cancels the normal into the special.
        let cancel_slot = self.combo_slot(me);
        if !scripted_turn && era_cancels && self.toggle_state.cancel.window[cancel_slot] != 0
            && self.toggle_state.cancel.move_id[cancel_slot] == self.eb(me, 0xa)
            && self.eb(me, 9) & 0x80 != 0
            && (self.try_modern_assist_normal(ps, me, true) || self.try_modern_shortcut(ps, me, true))
        {
            let count = &mut self.toggle_state.cancel.count[cancel_slot];
            *count = count.wrapping_add(1);
            self.input_tail(me, ps);
            return;
        }
        let era_cancel = !scripted_turn
            && era_cancels
            && self.toggle_state.cancel.window[cancel_slot] != 0
            && self.toggle_state.cancel.move_id[cancel_slot] == self.eb(me, 0xa)
            && self.eb(me, 9) & 0x80 != 0
            && self.try_era_special(ps, me);
        if era_cancel {
            let count = &mut self.toggle_state.cancel.count[cancel_slot];
            *count = count.wrapping_add(1);
        }
        if era_cancel || (era_cancels && self.cancel_gate(ps, me, other, d0)) {
            self.clear_neg_edge_latches_for_player(ps);
            self.input_tail(me, ps);
            return;
        }
        // Throws (SP19): in the 7-frame tech window of a throw, pressing
        // forward or back with heavy punch or heavy kick techs the throw.
        if era_throws && self.try_throw_tech(ps, me, other, d0) {
            self.clear_neg_edge_latches_for_player(ps);
            self.input_tail(me, ps);
            return;
        }
        // True combos (SP17): blockstun holds the guard.
        if self.combo_blockstun(ps, me, d0) {
            self.record_recovery_buffer_frame(ps, me, d0);
            self.input_tail(me, ps);
            return;
        }
        // 0x403d4: the busy gate.
        let state = self.eb(me, 9);
        let d2 = state & 0xc9;
        let mv = self.eb(me, 0xa);
        let custom = self.era_state(ps).is_some_and(|era| era.custom_combo_active() || era.gen_genei_jin());
        let cancel_custom = custom && state & 0x80 != 0 && state & 3 != 1
            && matches!(mv, 0xd..=0x1b | 0x1f | 0x2c..=0x2f) && mv != crate::data_bank::ALPHA_COUNTER_SLOT
            && self.era_buttons(ps).1 != 0
            && (self.latch_fires(ps, false) || self.latch_fires(ps + 1, true));
        if !scripted_turn && !pure_footsies && cancel_custom {
            if !self.try_era_special(ps, me) {
                self.dispatch_direction(ps, me, other, d0, d0);
            }
            self.clear_neg_edge_latches_for_player(ps);
            self.input_tail(me, ps);
            return;
        }
        let jumping = matches!(mv, 9..=0xb);
        let airborne = jumping && self.is_jump_airborne(me) && self.frames_until_landing(me) >= 4;
        let normal = matches!(mv, 0xd..=0x18) && state & 0x80 != 0;
        use crate::era_fight::SpecialContext::{Airborne, MashCancel};
        if !scripted_turn && !pure_footsies
            && ((airborne && era_throws && self.try_air_shoulder_throw(ps, me, other, d0))
                || (airborne && self.era_sf4_command(ps, me, Airborne))
                || (airborne && self.try_what_if_move(ps, me, Airborne))
                || (jumping && (airborne || state & 3 == 1) && self.try_alpha_move(ps, me, airborne, false))
                || (normal && (self.try_alpha_move(ps, me, false, true)
                    || self.try_what_if_move(ps, me, MashCancel)
                    || self.era_sf4_command(ps, me, MashCancel))))
        {
            self.clear_neg_edge_latches_for_player(ps);
            self.input_tail(me, ps);
            return;
        }
        if self.is_player_driven(ps) && busy_banked_jump(state, mv, self.rl(ps + 0x12)) {
            self.record_recovery_buffer_frame(ps, me, d0);
            self.input_tail(me, ps);
            return;
        }
        let interruptible_walk = (self.toggles.interruptible_walk_cycle
            || (self.toggles.standard_high_low_block && self.is_guard_interrupt(me, other, d0)))
            && (self.eb(me, 9) & 0x80) != 0
            && matches!(mv, 2 | 3);
        if d2 == 0 || mv == 0 || mv == 0xc || d2 == 8 || d2 == 9 || d2 == 1 {
            if scripted_turn {
                // 0x4046c: in these scene modes the held direction turns
                // the fighter instead of the opponent's position.
                if d0 == 1 && self.eb(me, 0x37) != 2 {
                    self.face_right(me);
                    self.set_eb(me, 0x37, 2);
                    self.input_tail(me, ps);
                    return;
                }
                if d0 == 2 && self.eb(me, 0x37) != 4 {
                    self.face_left(me);
                    self.set_eb(me, 0x37, 4);
                    self.input_tail(me, ps);
                    return;
                }
            } else {
                self.keep_facing(me, other);
            }
        } else if d2 != 0x80 && !interruptible_walk {
            self.record_recovery_buffer_frame(ps, me, d0);
            self.input_tail(me, ps);
            return;
        }
        // 0x404b4
        if self.eb(me, 9) == 0x80 && !interruptible_walk {
            self.record_recovery_buffer_frame(ps, me, d0);
            self.input_tail(me, ps);
            return;
        }
        if self.era_offense_flag(ps, self.toggles.positive_edge_attacks) {
            self.filter_neg_edge_latches(ps, me, d0);
        }
        if ashura_ready && self.try_ashura(ps, me, d0) {
            self.input_tail(me, ps);
            return;
        }
        let dispatch_d0 = self.apply_recovery_buffer(ps, me, d0);
        if !scripted_turn && (self.try_modern_assist_normal(ps, me, false)
            || self.try_modern_shortcut(ps, me, false))
        {
            self.input_tail(me, ps);
            return;
        }
        if !scripted_turn && ((!pure_footsies && self.try_custom_combo(ps, me)) || self.try_era_special(ps, me)) {
            self.clear_neg_edge_latches_for_player(ps);
            self.input_tail(me, ps);
            return;
        }
        if era_throws && self.try_throw(ps, me, other, dispatch_d0) {
            self.clear_neg_edge_latches_for_player(ps);
            self.input_tail(me, ps);
            return;
        }
        self.dispatch_direction(ps, me, other, dispatch_d0, d0);
        self.clear_neg_edge_latches_for_player(ps);
        self.input_tail(me, ps);
    }

    fn control_move_started(&mut self, ps: u32, slot: u8) {
        if let Some(index) = self.player_slot_index(ps) {
            let state = &mut self.toggle_state.schemes[index];
            state.scaled_move = state.executing == Some(slot) && state.shortcut_scaled;
        }
    }

    pub(crate) fn modern_scaled_move(&self, ps: u32) -> bool {
        self.player_slot_index(ps).is_some_and(|side| self.toggle_state.schemes[side].scaled_move)
    }

    fn try_modern_assist_normal(&mut self, ps: u32, entity: u16, canceling: bool) -> bool {
        let Some(side) = self.player_slot_index(ps) else { return false };
        let state = self.toggle_state.schemes[side];
        let Some((strength, kick, crouch)) = state.assist_normal else { return false };
        if (canceling && !state.assist_can_chain_normal()) || self.era_airborne(entity)
            || self.toggle_state.combo.blockstun[self.combo_slot(entity)] != 0
        {
            return false;
        }
        // Use the same strength and move tables as neutral/down dispatch.
        let table = (if crouch { 0x40884 } else { 0x405b6 }) + if kick { 8 } else { 0 };
        let latch = ps + u32::from(kick);
        let original = self.rb(latch);
        self.wb(latch, [0x40, 0xe0, 0xfe][usize::from(strength - 1)]);
        let Some(slot) = self.pick_by_strength(latch, table, entity) else {
            self.wb(latch, original);
            return false;
        };
        self.clear_tracker(self.tracker_base(ps) + 0x1c);
        self.start_move(entity, ps, slot);
        self.clear_neg_edge_latches_for_player(ps);
        self.clear_recovery_buffer(ps);
        self.toggle_state.schemes[side].accept_assist();
        true
    }

    fn try_modern_shortcut(&mut self, ps: u32, entity: u16, canceling: bool) -> bool {
        use crate::control::{shortcut_buttons, Shortcut};
        use crate::moves::{Dir, MoveKind};
        use crate::roster::TournamentFighter::Sagat;

        let Some(side) = self.player_slot_index(ps) else { return false };
        let scheme = self.toggle_state.schemes[side];
        let Some(shortcut) = scheme.shortcut else { return false };
        if self.world_tour_pure_footsies() || self.era_airborne(entity)
            || self.toggle_state.combo.blockstun[self.combo_slot(entity)] != 0
        {
            return false;
        }
        let strength = scheme.shortcut_strength;
        let era = self.era_state(ps).copied().unwrap_or_default();
        let Some(fighter) = era.fighter.or_else(|| self.fighter_of_entity(entity)) else { return false };
        let specials = crate::moves::live_move_list(fighter, self.preset, self.on_bank_player_table(ps), &era);

        let candidates = specials.iter().filter(|entry| {
            let ground = !entry.input.motion.contains(&Dir::Jump) && shortcut_buttons(entry.input.button, strength) != 0;
            ground && match shortcut {
                Shortcut::Finisher => entry.kind == MoveKind::Super,
                _ => entry.kind == MoveKind::Special && !matches!(entry.input.button,
                    crate::moves::ButtonClass::PairPunch | crate::moves::ButtonClass::PairKick),
            }
        });
        if shortcut == Shortcut::Finisher {
            let mut finishers: Vec<_> = candidates.copied().collect();
            finishers.sort_by_key(|entry| std::cmp::Reverse(entry.produces));
            return finishers.into_iter().any(|entry| {
                self.start_modern_shortcut(ps, entity, side, entry, strength, canceling)
            });
        }
        let entry = match shortcut {
            Shortcut::Finisher => unreachable!("finishers try their available meter levels in order"),
            Shortcut::Projectile => candidates.filter(|entry| entry.produces == 0x1a)
                .max_by_key(|entry| !entry.input.button.kick())
                .or_else(|| specials.iter().find(|entry| entry.kind == MoveKind::Special
                    && !entry.input.motion.contains(&Dir::Jump) && entry.input.motion == [Dir::Down, Dir::DownForward, Dir::Forward]
                    && !entry.input.button.kick() && entry.input.button == crate::moves::ButtonClass::Punch)),
            Shortcut::AntiAir => candidates.filter(|entry| entry.produces == 0x19 ||
                entry.input.motion == [Dir::Forward, Dir::Down, Dir::DownForward])
                .max_by_key(|entry| (!entry.input.button.kick(), entry.produces)),
            Shortcut::Advance => candidates.filter(|entry| entry.produces == 0x1b ||
                entry.input.motion == [Dir::Down, Dir::DownBack, Dir::Back])
                .min_by_key(|entry| (entry.produces != 0x1b, entry.produces)),
            Shortcut::Extra => candidates.filter(|entry| matches!(entry.produces, 0x1f | 0x20)
                && (fighter != Sagat || entry.input.motion != [Dir::Forward, Dir::Down, Dir::DownForward]))
                .min_by_key(|entry| entry.produces),
        }.copied();
        entry.is_some_and(|entry| self.start_modern_shortcut(ps, entity, side, entry, strength, canceling))
    }

    fn start_modern_shortcut(
        &mut self, ps: u32, entity: u16, side: usize, entry: crate::moves::MoveEntry,
        strength: u8, canceling: bool,
    ) -> bool {
        use crate::moves::Recognizer;

        self.toggle_state.schemes[side].executing = Some(entry.produces);
        let started = if matches!(entry.recognizer, Recognizer::Tracker(_)) && entry.produces <= 0x1b {
            if self.move_in_table(ps, entry.produces) {
                self.start_walk_attack(entity, ps, entry.produces);
                self.set_eb(entity, 0x34, strength);
                true
            } else {
                false
            }
        } else {
            self.try_modern_era_shortcut(ps, entity, entry, strength, canceling)
        };
        self.toggle_state.schemes[side].executing = None;
        if started {
            self.toggle_state.schemes[side].shortcut = None;
            self.toggle_state.schemes[side].accept_assist();
            self.clear_neg_edge_latches_for_player(ps);
            self.clear_recovery_buffer(ps);
        }
        started
    }

    pub(crate) fn dispatch_modern_era_shortcut(
        &mut self, ps: u32, entity: u16, entry: crate::moves::MoveEntry,
        strength: u8, canceling: bool,
    ) -> bool {
        use crate::era::EraFamily;
        use crate::era_fight::SpecialContext;
        use crate::moves::Recognizer;

        let context = SpecialContext::Ground { canceling };
        if entry.recognizer == Recognizer::FamilyMove {
            if self.era_state(ps).is_some_and(|state| state.what_if_active()) {
                return self.try_what_if_move(ps, entity, context);
            }
            return match self.chosen_era(ps) {
                Some(EraFamily::Sf3) => self.try_sf3_move(ps, entity, false),
                Some(EraFamily::Sf4) => self.era_sf4_command(ps, entity, context),
                Some(EraFamily::Sf5) => self.sf5_modern_attack(ps, entity, canceling),
                Some(EraFamily::Sf6) => self.sf6_modern_attack(ps, entity, canceling),
                _ => false,
            };
        }
        let latches = [self.rb(ps), self.rb(ps + 1)];
        let marks = [self.is_neg_edge_latch(ps), self.is_neg_edge_latch(ps + 1)];
        let kick = entry.input.button.kick();
        self.ww(ps, 0);
        self.clear_neg_edge_latches_for_player(ps);
        self.wb(ps + u32::from(kick), [0x40, 0xe0, 0xfe][usize::from(strength - 1)]);
        let tracker = self.tracker_base(ps) + 0x1b;
        let original = self.rb(tracker);
        if matches!(entry.recognizer, Recognizer::Tracker(_)) {
            self.wb(tracker, 0xff);
        }
        let started = self.try_era_special(ps, entity);
        if !started {
            self.wb(tracker, original);
            for (i, (latch, mark)) in latches.into_iter().zip(marks).enumerate() {
                let at = ps + i as u32;
                self.wb(at, latch);
                self.set_neg_edge_latch(at, mark);
            }
        }
        started
    }

    fn try_family_input(&mut self, ps: u32, entity: u16, relative: u8) -> bool {
        if self.world_tour_pure_footsies()
            || !self.era_state(ps).is_some_and(|state| state.uses_family_bank())
        {
            return false;
        }
        let posture = self.eb(entity, 9);
        let move_id = self.eb(entity, 0xa);
        let slot = self.combo_slot(entity);
        let blockstun = self.toggle_state.combo.blockstun[slot] != 0;
        let airborne = self.era_airborne(entity);
        let actionable = !blockstun && (posture & 0x80 == 0
            || (self.toggles.interruptible_walk_cycle && matches!(move_id, 2 | 3)));
        let cancel = !airborne && self.toggle_state.cancel.window[slot] != 0
            && self.toggle_state.cancel.move_id[slot] == move_id && posture & 0x80 != 0;
        let in_jump = self.is_jump_airborne(entity);
        self.what_if_track(ps, relative);
        self.sf3_track_input(ps, relative, airborne, actionable || in_jump, blockstun);
        if in_jump && self.frames_until_landing(entity) >= 4 && self.try_sf3_move(ps, entity, true) {
            self.clear_neg_edge_latches_for_player(ps);
            return true;
        }
        let special_active = posture & 0xc0 == 0xc0 && (matches!(move_id, 0x19..=0x1b)
            || self.era_state(ps).is_some_and(|state| state.sf4_special_active(move_id) || state.what_if_special_active(move_id)));
        let hard_knockdown = !airborne && posture & 0x80 != 0 && matches!(move_id, 0x28 | 0x35..=0x3f);
        if self.era_sf4_input(ps, entity, relative, actionable && !airborne, special_active, hard_knockdown)
            || self.sf5_input(ps, entity, relative, actionable && !airborne, cancel, blockstun, in_jump)
            || self.sf6_input(ps, entity, relative, actionable && !airborne, cancel, blockstun)
        {
            if blockstun && (self.era_state(ps).is_some_and(|state| state.is_sf6())
                && self.eb(entity, 0xa) == crate::era_fight::sf6::REVERSAL
                || self.era_state(ps).is_some_and(|state| state.style.family == crate::era::EraFamily::Sf5)
                    && self.eb(entity, 0xa) == crate::era_fight::sf5::REVERSAL)
            {
                self.toggle_state.combo.blockstun[slot] = 0;
            }
            self.clear_neg_edge_latches_for_player(ps);
            return true;
        }
        false
    }

    fn era_airborne(&self, entity: u16) -> bool {
        self.is_jump_airborne(entity) || self.airborne_reaction(entity)
            || (self.eb(entity, 9) & 3 == 1
                && (matches!(self.eb(entity, 0xa), 0x20..=0x26) || self.kage_air_strike(entity)))
    }

    fn kage_air_strike(&self, entity: u16) -> bool {
        let move_id = self.eb(entity, 0xa);
        matches!(move_id, KAGE_SEKIEIKEN | KAGE_MISOGI)
            && self.era_state(self.el(entity, 0x38))
                .is_some_and(|state| state.is_kage() && state.alpha_move_active(move_id))
    }

    fn era_special_command(&self, ps: u32, kick: bool) -> Option<u8> {
        use crate::era_fight::{SF2_EXTRA_SPECIAL, SF2_SUPER};
        use crate::roster::TournamentFighter::{Ryu, Sagat};

        if self.world_tour_pure_footsies() {
            return None;
        }
        let era = self.sf2_state(ps)?;
        let fighter = era.fighter?;
        // This profile uses the approved heavy red fireball and Demon command.
        if self.evil_ryu_profile(ps) == Some(EvilRyuPlaytest::UltraSf2) {
            return None;
        }
        let super_button = kick == (fighter == Sagat);
        let uppercut = fighter == Sagat && !kick && self.rb(self.tracker_base(ps) + 0x1b) != 0;
        let red_fireball = fighter == Ryu && !kick && era.extra_ready();
        let knee = fighter == Sagat && kick && era.extra_ready();
        let move_id = if era.super_ready() && super_button {
            SF2_SUPER
        } else if uppercut || red_fireball {
            SF2_EXTRA_SPECIAL
        } else if knee {
            0x19
        } else {
            return None;
        };
        if move_id == 0x19 {
            crate::data_bank::move_header(|addr| self.rl(addr), self.rl(ps + 0x12), move_id)?;
        } else {
            self.data_bank?.move_header(fighter, move_id)?;
        }
        Some(move_id)
    }

    fn try_era_special(&mut self, ps: u32, entity: u16) -> bool {
        use crate::era_fight::{SpecialContext, SF2_SUPER};

        if self.world_tour_pure_footsies() {
            return self.try_alpha_move(ps, entity, false, false) || self.try_sf3_move(ps, entity, false);
        }
        let canceling = self.eb(entity, 9) & 0xc0 == 0xc0;
        let context = SpecialContext::Ground { canceling };
        if self.try_what_if_move(ps, entity, context) {
            return true;
        }
        if self.alpha_state(ps).is_some() {
            return self.try_alpha_move(ps, entity, false, false);
        }
        if self.try_sf3_move(ps, entity, false) || self.era_sf4_command(ps, entity, context) {
            return true;
        }
        let punch = self.latch_fires(ps, false).then(|| self.era_special_command(ps, false)).flatten();
        let kick = self.latch_fires(ps + 1, true).then(|| self.era_special_command(ps, true)).flatten();
        let (move_id, latch, strength_table) = if let Some(mv) = kick.filter(|mv| *mv == SF2_SUPER || punch.is_none()) {
            (mv, ps + 1, 0x405be)
        } else if let Some(mv) = punch {
            (mv, ps, 0x405b6)
        } else {
            return false;
        };
        if self.pick_by_strength(latch, strength_table, entity).is_none() {
            return false;
        }
        self.ww(ps, 0);
        self.clear_all_trackers(ps);
        self.clear_recovery_buffer(ps);
        self.start_move(entity, ps, move_id);
        if move_id == SF2_SUPER {
            self.era_spend_super(ps);
        } else {
            self.era_use_extra_motion(ps);
        }
        true
    }

    fn family_move_header(&self, state: &crate::era_fight::EraFighterState, entity: u16, slot: u8) -> Option<u32> {
        let bank = self.data_bank?;
        if state.is_evil_ryu_sf6() {
            if let Some(header) = bank.sf6_evil_ryu_move_header(slot) {
                return Some(header);
            }
        }
        if state.is_kage() {
            use crate::era::Loadout;
            let enabled = match slot {
                KAGE_SEKIEIKEN => matches!(state.style.loadout, Loadout::Skill2Trigger1 | Loadout::Skill2Trigger2),
                KAGE_MISOGI => matches!(state.style.loadout, Loadout::Skill1Trigger2 | Loadout::Skill2Trigger2),
                _ => true,
            };
            if enabled {
                if let Some(header) = bank.kage_move_header(slot, self.eb(entity, 9) & 3 == 1) {
                    return Some(header);
                }
            }
        }
        bank.era_move_header(state.fighter?, state.style.family, slot)
    }

    pub(crate) fn era_start_banked_move(
        &mut self, ps: u32, entity: u16, slot: u8, level: u8, call_out: Option<&'static str>,
    ) -> bool {
        let Some(state) = self.era_state(ps).filter(|state| state.uses_family_bank()) else { return false };
        if self.family_move_header(state, entity, slot).is_none() {
            return false;
        }
        let air_ashura = state.is_kage() && matches!(slot, KAGE_ASHURA_AIR_FORWARD | KAGE_ASHURA_AIR_BACK);
        let air_misogi = state.is_kage() && slot == KAGE_MISOGI && self.eb(entity, 9) & 3 == 1;
        let height = (air_ashura || air_misogi).then(|| self.height_above_mat(entity));
        self.ww(ps, 0);
        self.clear_all_trackers(ps);
        self.clear_recovery_buffer(ps);
        self.era_stage_banked_move(ps, slot, level, call_out);
        self.start_move(entity, ps, slot);
        if let Some(height) = height {
            let step = self.height_above_mat(entity).wrapping_sub(height);
            self.shift_grid_y(entity, step);
        }
        self.set_eb(entity, 0x34, level.clamp(1, 3));
        true
    }

    fn try_custom_combo(&mut self, ps: u32, entity: u16) -> bool {
        let (held, pressed) = self.era_buttons(ps);
        let activates = if self.evil_ryu_profile(ps) == Some(EvilRyuPlaytest::Alpha2) {
            held.count_ones() == 3 && held & 0b000111 != 0 && held & 0b111000 != 0 && pressed != 0
        } else {
            [0x09, 0x12, 0x24].into_iter().any(|pair| held & pair == pair && pressed & pair != 0)
        };
        let slot = crate::data_bank::ALPHA_COUNTER_SLOT;
        if !activates || self.alpha_state(ps).and_then(|state| state.fighter)
            .and_then(|fighter| self.data_bank?.era_move_header(fighter, crate::era::EraFamily::SfZ, slot)).is_none()
            || !self.era_begin_custom_combo(ps)
        {
            return false;
        }
        self.ww(ps, 0);
        self.clear_all_trackers(ps);
        self.clear_recovery_buffer(ps);
        self.start_move(entity, ps, slot);
        true
    }

    fn try_alpha_move(&mut self, ps: u32, entity: u16, airborne: bool, normal: bool) -> bool {
        let strength = [false, true].map(|kick| {
            let latch = ps + u32::from(kick);
            if self.latch_fires(latch, kick) && !self.is_neg_edge_latch(latch) {
                crate::combo::level(self.rb(latch))
            } else {
                0
            }
        });
        let Some((entry, level, strength)) = self.era_alpha_command(ps, strength, airborne, normal) else {
            return false;
        };
        if entry.kind == crate::moves::MoveKind::Normal {
            // A completed special takes priority over a forward command normal.
            let forward = if self.eb(entity, 0x37) & 4 == 0 { 1 } else { 2 };
            if self.special_ready_for_dir(ps, entity, forward, entry.input.button.kick()) {
                return false;
            }
            if entry.slot == crate::data_bank::EVIL_RYU_ALPHA2_SENPUKYAKU
                || (self.evil_ryu_alpha3(ps))
            {
                let (_, other) = self.player_entities(ps);
                if self.try_throw(ps, entity, other, forward) {
                    return true;
                }
            }
        }
        if self.alpha_state(ps).and_then(|era| era.fighter)
                .and_then(|fighter| self.data_bank?.alpha_move_header(fighter, entry.slot)).is_none()
        {
            return false;
        }
        let center_y = self.center_y(entity);
        self.ww(ps, 0);
        self.clear_all_trackers(ps);
        self.clear_recovery_buffer(ps);
        self.era_begin_alpha_move(ps, entry, level);
        self.start_move(entity, ps, entry.slot);
        self.set_eb(entity, 0x34, strength);
        if airborne {
            self.era_adjust_air_start(ps, entity, center_y);
        }
        true
    }

    pub(crate) fn era_adjust_air_start(&mut self, ps: u32, entity: u16, center_y: u16) {
        let authored_y = self.center_y(entity);
        let offset = center_y.wrapping_sub(authored_y);
        self.shift_grid_y(entity, offset);
        let (frames, _) = self.stream_air_frames_and_dy(self.el(entity, 0x2c));
        if let Some(slot) = self.player_slot_index(ps) {
            self.toggle_state.air_adj_frames_1[slot] = frames.saturating_sub(1);
            self.toggle_state.air_adj_rem_1[slot] = offset.wrapping_neg() as i16;
        }
    }

    fn relative_directions(&self, entity: u16, directions: u8) -> u8 {
        if self.eb(entity, 0x37) & 4 == 0 {
            directions
        } else {
            (directions & !3) | ((directions & 1) << 1) | ((directions & 2) >> 1)
        }
    }

    fn what_if_eagle_riposte(&mut self, entity: u16, source_x: u16) -> bool {
        use crate::era_fight::what_if_others::{EAGLE_COUNTER, EAGLE_RIPOSTE};
        use crate::roster::TournamentFighter::Eagle;

        let ps = self.el(entity, 0x38);
        let center = self.center_x(entity);
        let from_front = source_x == center || (source_x > center) == (self.eb(entity, 0x37) & 4 == 0);
        // The guard window is the header pose, before the first recovery frame.
        let in_counter_pose = self.era_state(ps).filter(|state| state.what_if_eagle_counter())
            .and_then(|state| self.data_bank?.era_move_header(Eagle, state.style.family, EAGLE_COUNTER))
            .is_some_and(|header| self.el(entity, 0x2c) == self.rl(header + 0x2c));
        if !from_front || self.eb(entity, 0xa) != EAGLE_COUNTER || !in_counter_pose
            || !self.era_start_banked_move(ps, entity, EAGLE_RIPOSTE, 0, None)
        {
            return false;
        }
        self.era_emit_callout(ps, sf_video::callouts::CalloutKind::Move { word: "PARRY" });
        true
    }

    fn can_alpha_counter(&self, ps: u32) -> bool {
        self.alpha_state(ps).is_some_and(|era| era.alpha_counter_enabled())
    }

    fn alpha_counter_header(&self, ps: u32) -> Option<u32> {
        let fighter = self.alpha_state(ps)?.fighter?;
        let bank = self.data_bank?;
        if self.evil_ryu_alpha3(ps) {
            bank.evil_ryu_alpha3_counter(self.evil_ryu_profile(ps) == Some(EvilRyuPlaytest::Alpha3V))
        } else {
            bank.move_header(fighter, crate::data_bank::ALPHA_COUNTER_SLOT)
        }
    }

    fn try_alpha_counter(&mut self, ps: u32, entity: u16, relative: u8) -> bool {
        let slot = self.combo_slot(entity);
        if self.toggle_state.combo.blockstun[slot] == 0 || !matches!(self.eb(entity, 0xa), 4 | 8) {
            self.era_reset_alpha_counter(ps);
            return false;
        }
        let alpha3 = self.evil_ryu_alpha3(ps);
        let command = if alpha3 {
            relative == crate::moves::Dir::Forward.bits()
        } else {
            self.era_track_alpha_counter(ps, relative);
            [(ps, false), (ps + 1, true)].into_iter().any(|(latch, kick)| {
                self.latch_fires(latch, kick) && !self.is_neg_edge_latch(latch)
            })
        };
        if !command || !self.era_alpha_counter_ready(ps) || self.alpha_counter_header(ps).is_none()
            || !self.era_spend_alpha_counter(ps)
        {
            return false;
        }
        self.ww(ps, 0);
        self.clear_neg_edge_latches_for_player(ps);
        self.clear_all_trackers(ps);
        self.clear_recovery_buffer(ps);
        self.toggle_state.combo.blockstun[slot] = 0;
        self.start_move(entity, ps, crate::data_bank::ALPHA_COUNTER_SLOT);
        if alpha3 {
            self.set_eb(entity, 0x34, 3);
        }
        true
    }

    fn era_move_started(&mut self, entity: u16, ps: u32, move_id: u8) {
        use crate::era_fight::SF2_EXTRA_SPECIAL;
        use crate::roster::TournamentFighter::{Ken, Ryu, Sagat};

        self.era_sf4_started(ps, move_id);
        self.sf6_started(ps, move_id);
        self.sf5_on_move_started(ps, move_id);
        if (self.evil_ryu_profile(ps) == Some(EvilRyuPlaytest::Alpha2) || self.evil_ryu_alpha3(ps))
            && self.alpha_state(ps).is_some()
        {
            // Build stock on specials in the private Alpha fights, including whiffs.
            let gain = match move_id {
                0x19 => 7,
                0x1a | SF2_EXTRA_SPECIAL => 2,
                0x1b | crate::era_fight::sf3::AIR_TATSU => 6,
                _ => 0,
            };
            if gain != 0 {
                self.era_meter_gain(ps, gain);
            }
        }
        let Some(era) = self.sf2_state(ps) else { return };
        let Some(delta) = crate::era_fight::RevisionDelta::of(era.style) else { return };
        // Super Turbo awards special meter once when the move starts, even on a whiff.
        let meter = match (era.fighter, move_id) {
            (Some(Ryu), 0x19) | (Some(Sagat), SF2_EXTRA_SPECIAL) => 7,
            (Some(Ken), 0x19) => 8,
            (Some(Ryu | Ken), 0x1a) | (Some(Ryu), SF2_EXTRA_SPECIAL) => 2,
            (Some(Ryu | Ken), 0x1b) | (Some(Sagat), 0x19) => 6,
            (Some(Ryu | Ken), crate::era_fight::sf3::AIR_TATSU) if era.alpha_move_active(move_id) => 6,
            (Some(Sagat), 0x1a | 0x1b) => 3,
            _ => 0,
        };
        if let Some(side) = self.era_side(ps) {
            self.toggle_state.era.fighters[side].move_hits = 0;
        }
        if matches!(move_id, 0x0d..=SF2_EXTRA_SPECIAL | 0x20..=0x26) {
            self.set_eb(entity, 0xe, self.eb(entity, 0xe).saturating_add_signed(delta.startup));
        }
        if meter != 0 {
            self.era_meter_gain(ps, meter);
        }
    }

    fn era_normal_contact(&mut self, entity: u16, guarded: bool) {
        use crate::combo::{Class, class_of, level};

        let owner = self.el(entity, 0x38);
        let move_id = self.eb(entity, 0xa);
        if move_id == crate::era_fight::sf3::AIR_TATSU
            && self.era_state(owner).is_some_and(|state| state.alpha_move_active(move_id))
        {
            return;
        }
        if self.era_state(owner).is_some_and(|state| state.is_sf3() || state.is_sf4())
            && matches!(move_id, 0x0d..=0x18 | 0x1c..=0x1e | 0x20..=0x26)
        {
            self.era_meter_gain(owner, if guarded { 2 } else { 4 });
            return;
        }
        if (self.sf2_state(owner).is_none() && self.alpha_state(owner).is_none())
            || !Self::is_player_table(self.rl(owner + 0x12))
            || !matches!(move_id, 0x0d..=0x18 | 0x1c..=0x1e | 0x20..=0x26)
        {
            return;
        }
        // Arcade ST's contact awards by button strength (hit, guard).
        let (hit, guard) = match class_of(move_id, level(self.eb(entity, 0x34))) {
            Class::Light | Class::Jump(1) => (2, 1),
            Class::Medium | Class::Jump(2) => (4, 3),
            Class::Heavy | Class::Sweep | Class::Jump(_) => (5, 4),
            Class::Special => return,
        };
        self.era_meter_gain(owner, if guarded { guard } else { hit });
    }

    fn era_move_recovery(&mut self, entity: u16, frame: u32) {
        let owner = self.el(entity, 0x38);
        let Some(delta) = self.sf2_state(owner).and_then(|era| crate::era_fight::RevisionDelta::of(era.style)) else {
            return;
        };
        let move_id = self.eb(entity, 0xa);
        if !matches!(move_id, 0x0d..=crate::era_fight::SF2_EXTRA_SPECIAL | 0x20..=0x26)
            || self.eb(entity, 0xf) & 0x80 != 0
        {
            return;
        }
        let next = frame + 0x20;
        if self.rb(next) == 0xfd || (self.rb(next) == 0xff && self.rl(next + 2) == 0x45630) {
            for byte in [0xe, 0x33] {
                self.set_eb(entity, byte, self.eb(entity, byte).saturating_add_signed(delta.recovery));
            }
        }
    }

    pub(crate) fn evil_ryu_profile(&self, ps: u32) -> Option<EvilRyuPlaytest> {
        if let Some(awakening) = self.toggle_state.satsui_awakening {
            use crate::era::EraFamily::{Sf1Plus, Sf2};

            if awakening.side.fighter_struct() == ps
                && (awakening.profile != EvilRyuPlaytest::UltraSf2
                    || matches!(self.chosen_era(ps), Some(Sf1Plus | Sf2)))
            {
                return Some(awakening.profile);
            }
        }
        self.evil_ryu_playtest.filter(|_| ps == P1_STRUCT)
    }

    fn is_active_ryu(&self, ps: u32) -> bool {
        self.rb(SCENE_MODE) == 0x80
            && self.rb(MODE_DEMO) != 0
            && self.rl(ps + 0x12) == crate::roster::TournamentFighter::Ryu.move_table()
    }

    fn legacy_evil_ryu(&self, ps: u32) -> bool {
        self.evil_ryu_profile(ps).is_some_and(|profile| !profile.is_kage() && profile != EvilRyuPlaytest::Sf6)
    }

    fn evil_ryu_alpha3(&self, ps: u32) -> bool {
        self.evil_ryu_profile(ps).is_some_and(EvilRyuPlaytest::is_alpha3)
    }

    fn evil_ryu_sf4(&self, ps: u32) -> bool {
        self.evil_ryu_profile(ps).is_some_and(EvilRyuPlaytest::is_sf4)
    }

    fn evil_ryu_has_demon(&self, ps: u32) -> bool {
        match self.evil_ryu_profile(ps) {
            Some(EvilRyuPlaytest::UltraSf2) => self.sf2_state(ps).is_some_and(|state| {
                state.style.revision == crate::era::Revision::St && state.style.loadout == crate::era::Loadout::Standard
            }),
            Some(EvilRyuPlaytest::Alpha2 | EvilRyuPlaytest::Alpha3A | EvilRyuPlaytest::Alpha3X
                | EvilRyuPlaytest::KageII | EvilRyuPlaytest::Sf6) => true,
            Some(profile) => profile.is_sf4(),
            None => false,
        }
    }

    fn demon_rush_move(&self, ps: u32) -> u8 {
        match self.evil_ryu_profile(ps) {
            Some(EvilRyuPlaytest::KageII) => KAGE_RAGING_DEMON,
            Some(EvilRyuPlaytest::Sf6) => SF6_RAGING_DEMON,
            _ => EVIL_RYU_RAGING_DEMON,
        }
    }

    /// Arcade Start shares a button with pause. Consume it as a feint only
    /// while the native Hadoken motion can finish on the held direction.
    pub(crate) fn try_alpha2_feint(&mut self, direction: u8) -> bool {
        use crate::data_bank::EVIL_RYU_ALPHA2_FAKE_HADOKEN;
        use crate::roster::TournamentFighter::Ryu;

        let ps = if self.rb(GAME_MODE) == 4 { P2_STRUCT } else { P1_STRUCT };
        if self.evil_ryu_profile(ps) != Some(EvilRyuPlaytest::Alpha2)
            || !self.is_active_ryu(ps)
            || self.toggle_state.era.freeze != 0
            || self.era_dizzy(ps)
        {
            return false;
        }
        let (me, _) = self.player_entities(ps);
        if self.eb(me, 8) != 2 || self.era_airborne(me) || self.is_in_recovery_or_stun(me)
            || self.toggle_state.combo.blockstun[self.combo_slot(me)] != 0
            || self.toggle_state.raging_demon.victory_pose
            || self.relative_directions(me, direction) != 1
        {
            return false;
        }
        let tracker = self.tracker_base(ps) + 0x1c;
        let limit = if self.toggles.lenient_command_trackers {
            self.toggles.command_tracker_window
        } else {
            0x12
        };
        if !matches!(self.rb(tracker), 2 | 3) || self.rb(tracker + 1) >= limit
            || self.data_bank.and_then(|bank| bank.alpha_move_header(Ryu, EVIL_RYU_ALPHA2_FAKE_HADOKEN)).is_none()
        {
            return false;
        }
        self.clear_all_trackers(ps);
        self.clear_recovery_buffer(ps);
        self.clear_neg_edge_latches_for_player(ps);
        self.ww(ps, 0);
        self.era_begin_alpha2_feint(ps);
        self.start_move(me, ps, EVIL_RYU_ALPHA2_FAKE_HADOKEN);
        true
    }

    const ASHURA_MOTIONS: [[u8; 3]; 2] = [
        [1, 4, 5], // Forward, down, down-forward.
        [2, 4, 6], // Back, down, down-back.
    ];

    fn track_ashura(&mut self, me: u16, direction: u8) {
        let direction = self.relative_directions(me, direction);
        let window = self.toggles.command_tracker_window;
        for (pattern, motion) in Self::ASHURA_MOTIONS.into_iter().zip(&mut self.toggle_state.ashura_motion) {
            if motion[1] >= window || (motion[0] == 3 && direction != pattern[2]) {
                *motion = [0, 0];
            }
            if motion[0] != 0 {
                motion[1] += 1;
            }
            if pattern.get(usize::from(motion[0])) == Some(&direction) {
                motion[0] += 1;
            }
        }
    }

    fn ready_ashura(&self, ps: u32, me: u16, direction: u8) -> Option<u8> {
        let direction = self.relative_directions(me, direction);
        [EVIL_RYU_ASHURA_FORWARD, self.ashura_back_move(ps)].into_iter()
            .zip(Self::ASHURA_MOTIONS).zip(self.toggle_state.ashura_motion)
            .find_map(|((mv, pattern), [stage, _])| (stage == 3 && direction == pattern[2]).then_some(mv))
    }

    fn try_ashura(&mut self, ps: u32, me: u16, direction: u8) -> bool {
        let Some(mv) = self.ready_ashura(ps, me, direction) else { return false };
        let variable_range = self.evil_ryu_alpha3(ps) || self.evil_ryu_sf4(ps);
        let mut long_glide = false;
        let fires = if self.evil_ryu_profile(ps) == Some(EvilRyuPlaytest::Alpha2) || variable_range {
            let (held, pressed) = self.era_buttons(ps);
            let punch = held & 0b000111 == 0b000111 && pressed & 0b000111 != 0;
            let kick = held & 0b111000 == 0b111000 && pressed & 0b111000 != 0;
            long_glide = punch && variable_range;
            punch || kick
        } else {
            self.latch_fires(ps + 1, true) && !self.latch_fires(ps, false)
        };
        if !fires {
            return false;
        }
        self.clear_all_trackers(ps);
        self.toggle_state.ashura_motion = [[0; 2]; 2];
        self.clear_recovery_buffer(ps);
        self.clear_neg_edge_latches_for_player(ps);
        self.ww(ps, 0);
        self.hold_evil_ryu_stance(me, mv);
        // Ryu's planted Hadoken gather winds up the raised-knee glide.
        self.load_frame(me, 0x01362);
        if variable_range {
            self.set_eb(me, 0x34, u8::from(long_glide));
        }
        self.set_eb(me, 0x33, if long_glide { 27 } else { 23 });
        true
    }

    fn try_raging_demon(&mut self, ps: u32, me: u16, other: u16, grounded: bool) -> bool {
        let demon = &mut self.toggle_state.raging_demon;
        if usize::from(demon.progress) != Self::DEMON_SEQUENCE.len() {
            return false;
        }
        demon.progress = 0;
        demon.age = 0;
        let state = self.eb(me, 9);
        let light_attack = state & 0x40 != 0 && matches!(self.eb(me, 0xa), 0x0d | 0x14);
        if !grounded || self.eb(me, 8) != 2 || state & 8 != 0
            || (self.is_in_recovery_or_stun(me) && !light_attack)
            || self.toggle_state.combo.blockstun[self.combo_slot(me)] != 0
            || !self.toggles.throws || self.throw_entries(me, other).is_none()
            || !self.try_spend_demon_super(ps)
        {
            return false;
        }
        self.toggle_state.ashura_motion = [[0; 2]; 2];
        self.clear_all_trackers(ps);
        self.clear_recovery_buffer(ps);
        self.clear_neg_edge_latches_for_player(ps);
        self.ww(ps, 0);
        self.hold_evil_ryu_stance(me, self.demon_rush_move(ps));
        self.load_evil_ryu_glide_pose(me);
        self.set_eb(me, 0xf, 0);
        self.set_eb(me, 0x33, DEMON_RUSH_FRAMES + DEMON_WHIFF_FRAMES);
        true
    }

    /// 0x404cc: the posture dispatch on direction `dir` (the held one, or
    /// the recovery buffer's); `held` is the direction actually held, which
    /// the diagonal jumps read.
    fn dispatch_direction(&mut self, ps: u32, me: u16, other: u16, dir: u8, held: u8) {
        match dir {
            0 => self.input_neutral_dir(ps, me),
            1 => self.input_right(ps, me, other),
            2 => self.input_left(ps, me, other),
            4 => self.input_down(ps, me),
            5 => self.input_down_right(ps, me, other),
            6 => self.input_down_left(ps, me, other),
            8 => {
                // 0x40a86: neutral jump; the attack strengths are dropped.
                self.set_ew(me, 0x34, 0);
                self.start_move(me, ps, 9);
            }
            9 | 10 => {
                // 0x40a98 / 0x40abe: up-right is a forward jump when facing
                // right, a back jump when facing left; up-left the reverse.
                let facing_left = self.eb(me, 0x37) & 4 != 0;
                let forward = (held == 9) != facing_left;
                self.set_ew(me, 0x34, 0);
                self.start_move(me, ps, if forward { 0xa } else { 0xb });
            }
            _ => {} // 3, 7, 11-15: the table sends them to the tail.
        }
    }

    /// 0x40360..0x403b2: the three special-move command trackers (3 bytes
    /// each: stage, timer, done) at player +0x16 (Tatsumaki: down, down-back,
    /// back), +0x19 (Shoryuken: forward, down, down-forward) and +0x1c
    /// (Hadoken: down, down-forward, forward), fed the facing-relative
    /// direction from the 11-byte tables at 0x403be / 0x403c9. Directions
    /// 11-15 read past those tables into the following bytes, as the
    /// original does.
    fn track_commands(&mut self, ps: u32, me: u16, d0: u8) {
        let map = if (self.eb(me, 0x37) as i8) >= 4 {
            0x403c9
        } else {
            0x403be
        };
        let dir = self.rb(map + d0 as u32);
        for (tracker, pattern) in [(0x16, 0x4038e), (0x19, 0x403a2), (0x1c, 0x403ba)] {
            self.track_command(self.tracker_base(ps) + tracker, pattern, dir);
        }
    }

    /// 0x40ae4: advance one command tracker. Stage 0 waits for the first
    /// direction; stages 1-2 advance on the next ones (stage 3 sets done =
    /// 0xff) while counting frames; stage 3 holds while the last direction
    /// is held. 0x12 frames from the first direction, or leaving the last
    /// direction in stage 3, resets the tracker.
    fn track_command(&mut self, t: u32, pattern: u32, dir: u8) {
        if self.toggles.lenient_command_trackers {
            self.track_command_lenient(t, pattern, dir);
            return;
        }
        match self.rb(t) {
            1 => {
                if dir == self.rb(pattern + 1) {
                    self.wb(t, 2);
                }
                self.count_tracker(t);
            }
            2 => {
                if dir == self.rb(pattern + 2) {
                    self.wb(t, 3);
                    self.wb(t + 2, 0xff);
                }
                self.count_tracker(t);
            }
            3 => {
                if self.rb(t + 1) == 0x12 || dir != self.rb(pattern + 2) {
                    self.clear_tracker(t);
                } else {
                    self.wb(t + 1, self.rb(t + 1).wrapping_add(1));
                }
            }
            s => {
                if dir == self.rb(pattern) {
                    self.wb(t, s.wrapping_add(1));
                    self.wb(t + 1, 0);
                }
            }
        }
    }

    /// 0x40b20 / 0x40b50: count a tracker's frame, resetting it at 0x12 (or
    /// `command_tracker_window` when `lenient_command_trackers` is enabled).
    fn count_tracker(&mut self, t: u32) {
        let limit = if self.toggles.lenient_command_trackers {
            self.toggles.command_tracker_window
        } else {
            0x12
        };
        if self.rb(t + 1) >= limit {
            self.clear_tracker(t);
        } else {
            self.wb(t + 1, self.rb(t + 1).wrapping_add(1));
        }
    }

    /// Clear a command tracker's three bytes (stage, timer, done) and any
    /// lenient release-grace age.
    fn clear_tracker(&mut self, t: u32) {
        self.wb(t + 2, 0);
        self.wb(t + 1, 0);
        self.wb(t, 0);
        if let Some((p_idx, t_idx)) = self.tracker_slot(t) {
            self.toggle_state.tracker_release_age[p_idx][t_idx] = 0;
        }
    }

    /// The tracker's command was completed and an attack strength is
    /// latched (+0x34 >= 1, done != 0): the special-move checks of the
    /// attack branches.
    ///
    /// Returns the special the tracker at `ps + off` fires with the button
    /// (punch, or `kick`; [`Self::tracker_special`]): on a ROM table only
    /// the 1987 three, so the 1987 branches fire what they always did, and
    /// the branches the 1987 handler has no special in (a punch held back,
    /// a kick held forward or down-forward) fire one only for a boss whose
    /// table puts a special there.
    fn special_ready(&self, me: u16, ps: u32, off: u32, kick: bool) -> Option<u8> {
        if self.eb(me, 0x34) < 1 {
            return None;
        }
        self.command_done(ps, off, kick)
    }

    /// Start `mv` unless it is already the current move of a busy fighter
    /// (+9 bit 7): the walk branches' keep-walking test.
    fn continue_or_start(&mut self, me: u16, ps: u32, mv: u8) {
        if !(self.eb(me, 9) & 0x80 != 0 && self.eb(me, 0xa) == mv) {
            self.start_move(me, ps, mv);
        }
    }

    /// Start `mv` unless it is already the current move (the crouch and
    /// block branches test the move id only).
    fn start_unless_current(&mut self, me: u16, ps: u32, mv: u8) {
        if self.eb(me, 0xa) != mv {
            self.start_move(me, ps, mv);
        }
    }

    /// The opponent is attacking high (+9 0xc2) or in a jump attack (0xc1):
    /// holding back blocks (0x40626 / 0x40714). Under `standard_high_low_block`,
    /// holding back enters standing guard (move 4) against any opponent attack
    /// or incoming projectile (#96, O5).
    fn opponent_attacks_high(&self, other: u16) -> bool {
        if self.toggles.standard_high_low_block {
            matches!(self.eb(other, 9), 0xc2 | 0xc1 | 0xc4) || self.has_incoming_projectile(other)
        } else {
            matches!(self.eb(other, 9), 0xc2 | 0xc1)
        }
    }

    /// The opponent is attacking low (+9 0xc4) or in a jump attack (0xc1):
    /// holding down-back blocks crouching (0x408da / 0x409b4). Under
    /// `standard_high_low_block`, holding down-back enters crouching guard
    /// (move 8) against grounded attacks (0xc4, 0xc2), airborne specials
    /// guarded crouching (the Shoryuken 0x19 and Tatsumaki 0x1b; a boss's
    /// by his table's slot flag, [`crate::data_bank::SpecialSlot`]), and
    /// incoming projectiles, while excluding normal jumping overheads (0xc1)
    /// so down-back does not falsely enter move 8 against jump attacks (#96,
    /// O5).
    fn opponent_attacks_low(&self, other: u16) -> bool {
        if self.toggles.standard_high_low_block {
            let st = self.eb(other, 9);
            let mv = self.eb(other, 0xa);
            let low_in_air = || {
                self.kage_air_strike(other)
                    || (crate::data_bank::SPECIAL_SLOTS.contains(&mv)
                        && self.special_slot(self.el(other, 0x38), mv).low_in_air)
            };
            matches!(st, 0xc4 | 0xc2)
                || (st == 0xc1 && low_in_air())
                || self.has_incoming_projectile(other)
        } else {
            matches!(self.eb(other, 9), 0xc4 | 0xc1)
        }
    }

    /// 0x4052c: no direction held. A latched strength starts the standing
    /// attack it maps to (0x405b6 punch / 0x405be kick); otherwise a fighter
    /// whose current move is not the stance (move 1) restarts it
    /// (0x40548..0x405ac), and a stance with a pending turn turns.
    fn input_neutral_dir(&mut self, ps: u32, me: u16) {
        if self.toggles.lenient_command_trackers {
            if let Some(mut mv) = self.pick_by_strength(ps, 0x405b6, me) {
                if let Some(sp) = self.lenient_punch_special(me, ps) {
                    mv = sp;
                    self.clear_all_trackers(ps);
                    self.wb(ps + 1, 0);
                } else {
                    self.clear_tracker(self.tracker_base(ps) + 0x1c);
                }
                self.start_move(me, ps, mv);
                return;
            }
            if let Some(mut mv) = self.pick_by_strength(ps + 1, 0x405be, me) {
                if let Some(sp) = self.lenient_kick_special(me, ps) {
                    mv = sp;
                    self.clear_all_trackers(ps);
                    self.wb(ps, 0);
                } else {
                    self.clear_tracker(self.tracker_base(ps) + 0x1c);
                }
                self.start_move(me, ps, mv);
                return;
            }
        }
        let attack = self
            .pick_by_strength(ps, 0x405b6, me)
            .or_else(|| self.pick_by_strength(ps + 1, 0x405be, me));
        if let Some(mv) = attack {
            // 0x4059c: reset the Hadoken tracker.
            self.clear_tracker(self.tracker_base(ps) + 0x1c);
            self.start_move(me, ps, mv);
        } else if !(self.eb(me, 9) & 0x80 != 0 && self.eb(me, 0xa) & 3 != 0) {
            if self.eb(me, 0xa) == 1 {
                match self.eb(me, 0x37) {
                    5 => self.face_left(me),
                    3 => self.face_right(me),
                    _ => {}
                }
            } else {
                self.start_move(me, ps, 1);
            }
        }
    }

    /// 0x406ac / 0x407d0: an attack from a walk direction. Reset the
    /// Hadoken and Tatsumaki trackers and both latches, then start `mv`.
    fn start_walk_attack(&mut self, me: u16, ps: u32, mv: u8) {
        let b = self.tracker_base(ps);
        self.clear_tracker(b + 0x1c);
        self.clear_tracker(b + 0x16);
        if self.toggles.lenient_command_trackers {
            self.clear_tracker(b + 0x19);
        }
        self.wb(ps, 0);
        self.wb(ps + 1, 0);
        self.start_move(me, ps, mv);
    }

    /// 0x405c6: right held. Facing right it is forward: punch (0x406de; a
    /// completed Hadoken command fires it, 0x40670), kick (0x406e6), else
    /// walk forward (move 2). Facing left it is back: punch, kick (a
    /// completed Tatsumaki command fires it, 0x40690), else block (move 4)
    /// against an attacking opponent or walk back (move 3).
    fn input_right(&mut self, ps: u32, me: u16, other: u16) {
        if (self.eb(me, 0x37) as i8) < 4 {
            if let Some(mut mv) = self.pick_by_strength(ps, 0x406de, me) {
                if let Some(sp) = self.lenient_punch_special(me, ps) {
                    mv = sp;
                } else if let Some(sp) = self.special_ready(me, ps, 0x1c, false) {
                    mv = sp;
                }
                self.start_walk_attack(me, ps, mv);
            } else if let Some(mut mv) = self.pick_by_strength(ps + 1, 0x406e6, me) {
                if let Some(sp) = self.lenient_kick_special(me, ps) {
                    mv = sp;
                } else if let Some(sp) = self.special_ready(me, ps, 0x1c, true) {
                    // A boss's special on the Hadoken's command and a kick.
                    mv = sp;
                }
                self.start_walk_attack(me, ps, mv);
            } else {
                self.continue_or_start(me, ps, 2);
            }
        } else if let Some(mut mv) = self.pick_by_strength(ps, 0x406de, me) {
            if let Some(sp) = self.lenient_punch_special(me, ps) {
                mv = sp;
            } else if let Some(sp) = self.special_ready(me, ps, 0x16, false) {
                // A boss's special on the Tatsumaki's command and a punch.
                mv = sp;
            }
            self.start_walk_attack(me, ps, mv);
        } else if let Some(mut mv) = self.pick_by_strength(ps + 1, 0x406e6, me) {
            if let Some(sp) = self.lenient_kick_special(me, ps) {
                mv = sp;
            } else if let Some(sp) = self.special_ready(me, ps, 0x16, true) {
                mv = sp;
            }
            self.start_walk_attack(me, ps, mv);
        } else if self.incoming_attack(me, other, false) {
            // 0x4063c: block (started without the tracker reset).
            self.start_unless_current(me, ps, 4);
        } else {
            self.continue_or_start(me, ps, 3);
        }
    }

    /// 0x406ee: left held, the mirror of [`Self::input_right`] with two
    /// differences kept from the original: the block (0x4072a) resets the
    /// trackers and latches first, and the forward punch's Hadoken check
    /// falls through into the Tatsumaki check (0x40798 -> 0x407b4).
    fn input_left(&mut self, ps: u32, me: u16, other: u16) {
        if (self.eb(me, 0x37) as i8) < 4 {
            // Back.
            if let Some(mut mv) = self.pick_by_strength(ps, 0x40802, me) {
                if let Some(sp) = self.lenient_punch_special(me, ps) {
                    mv = sp;
                } else if let Some(sp) = self.special_ready(me, ps, 0x16, false) {
                    // A boss's special on the Tatsumaki's command and a punch.
                    mv = sp;
                }
                self.start_walk_attack(me, ps, mv);
            } else if let Some(mut mv) = self.pick_by_strength(ps + 1, 0x4080a, me) {
                if let Some(sp) = self.lenient_kick_special(me, ps) {
                    mv = sp;
                } else if let Some(sp) = self.special_ready(me, ps, 0x16, true) {
                    mv = sp;
                }
                self.start_walk_attack(me, ps, mv);
            } else if self.incoming_attack(me, other, false) {
                if self.toggles.standard_high_low_block || self.toggles.balanced_roster {
                    self.start_unless_current(me, ps, 4);
                } else if self.eb(me, 0xa) != 4 {
                    self.start_walk_attack(me, ps, 4);
                }
            } else {
                self.continue_or_start(me, ps, 3);
            }
        } else if let Some(mut mv) = self.pick_by_strength(ps, 0x40802, me) {
            // Forward punch: 0x40798.
            if let Some(sp) = self.lenient_punch_special(me, ps) {
                mv = sp;
            } else if let Some(sp) = self.special_ready(me, ps, 0x1c, false) {
                mv = sp;
                if !self.toggles.balanced_roster {
                    // 0x407b4: the Tatsumaki's done command (with its kick)
                    // takes the punch over.
                    if let Some(sp) = self.special_ready(me, ps, 0x16, true) {
                        mv = sp;
                    }
                }
            }
            self.start_walk_attack(me, ps, mv);
        } else if let Some(mut mv) = self.pick_by_strength(ps + 1, 0x4080a, me) {
            if let Some(sp) = self.lenient_kick_special(me, ps) {
                mv = sp;
            } else if let Some(sp) = self.special_ready(me, ps, 0x1c, true) {
                // A boss's special on the Hadoken's command and a kick.
                mv = sp;
            }
            self.start_walk_attack(me, ps, mv);
        } else {
            self.continue_or_start(me, ps, 2);
        }
    }

    /// 0x40812: down held. A crouching punch (0x40884) or kick (0x4088c)
    /// with the Hadoken tracker reset (0x4086a), else the crouch (move 5);
    /// a fighter already crouching applies a pending turn instead.
    fn input_down(&mut self, ps: u32, me: u16) {
        if self.toggles.lenient_command_trackers {
            if let Some(mut mv) = self.pick_by_strength(ps, 0x40884, me) {
                if let Some(sp) = self.lenient_punch_special(me, ps) {
                    mv = sp;
                    self.clear_all_trackers(ps);
                    self.wb(ps + 1, 0);
                } else {
                    self.clear_tracker(self.tracker_base(ps) + 0x1c);
                }
                self.start_move(me, ps, mv);
                return;
            }
            if let Some(mut mv) = self.pick_by_strength(ps + 1, 0x4088c, me) {
                if let Some(sp) = self.lenient_kick_special(me, ps) {
                    mv = sp;
                    self.clear_all_trackers(ps);
                    self.wb(ps, 0);
                } else {
                    self.clear_tracker(self.tracker_base(ps) + 0x1c);
                }
                self.start_move(me, ps, mv);
                return;
            }
        }
        let attack = self
            .pick_by_strength(ps, 0x40884, me)
            .or_else(|| self.pick_by_strength(ps + 1, 0x4088c, me));
        if let Some(mv) = attack {
            self.clear_tracker(self.tracker_base(ps) + 0x1c);
            self.start_move(me, ps, mv);
        } else if self.eb(me, 0xa) == 5 {
            match self.eb(me, 0x37) {
                5 => self.face_left(me),
                3 => self.face_right(me),
                _ => {}
            }
        } else {
            self.start_move(me, ps, 5);
        }
    }

    /// 0x40940 / 0x40a38: a crouching kick from a diagonal. Reset the
    /// Hadoken tracker and the punch latch, then start `mv`.
    fn start_low_kick(&mut self, me: u16, ps: u32, mut mv: u8) {
        if let Some(sp) = self.lenient_kick_special(me, ps) {
            mv = sp;
            self.clear_all_trackers(ps);
        } else {
            self.clear_tracker(self.tracker_base(ps) + 0x1c);
        }
        self.wb(ps, 0);
        self.start_move(me, ps, mv);
    }

    /// 0x40958 / 0x40a50: a crouching punch from a diagonal. Reset the
    /// Hadoken tracker and the kick latch, then start `mv`.
    fn start_low_punch(&mut self, me: u16, ps: u32, mut mv: u8) {
        if let Some(sp) = self.lenient_punch_special(me, ps) {
            mv = sp;
            self.clear_all_trackers(ps);
        } else {
            self.clear_tracker(self.tracker_base(ps) + 0x1c);
        }
        self.wb(ps + 1, 0);
        self.start_move(me, ps, mv);
    }

    /// 0x40918 / 0x40a10: a down-forward punch; a completed Shoryuken
    /// command fires it and clears that tracker's timer and done bytes.
    fn down_forward_punch(&mut self, me: u16, ps: u32, mut mv: u8) {
        if let Some(sp) = self.special_ready(me, ps, 0x19, false) {
            let b = self.tracker_base(ps);
            mv = sp;
            self.wb(b + 0x1a, 0);
            self.wb(b + 0x1b, 0);
            if self.toggles.lenient_command_trackers {
                self.clear_tracker(b + 0x19);
            }
        }
        self.start_low_punch(me, ps, mv);
    }

    /// A down-forward kick: as [`Self::down_forward_punch`] for a boss whose
    /// table puts a special on the Shoryuken's command with a kick (no ROM
    /// table does, so the 1987 kick is untouched).
    fn down_forward_kick(&mut self, me: u16, ps: u32, mut mv: u8) {
        if let Some(sp) = self.special_ready(me, ps, 0x19, true) {
            let b = self.tracker_base(ps);
            mv = sp;
            self.wb(b + 0x1a, 0);
            self.wb(b + 0x1b, 0);
            if self.toggles.lenient_command_trackers {
                self.clear_tracker(b + 0x19);
            }
        }
        self.start_low_kick(me, ps, mv);
    }

    /// The down-back posture without a button: crouch-block (move 8)
    /// against a low or jumping attack, else crouch back (move 7).
    fn down_back(&mut self, me: u16, ps: u32, other: u16) {
        if self.incoming_attack(me, other, true) {
            self.start_unless_current(me, ps, 8);
        } else {
            self.start_unless_current(me, ps, 7);
        }
    }

    /// 0x40894: down-right held. The kick table (0x40986) is tried first
    /// whatever the facing; then, facing right (down-forward), a punch
    /// (0x40976, Shoryuken check) or the forward crouch (move 6); facing
    /// left (down-back), a punch (0x4097e) or block / back crouch.
    fn input_down_right(&mut self, ps: u32, me: u16, other: u16) {
        if let Some(mv) = self.pick_by_strength(ps + 1, 0x40986, me) {
            if (self.eb(me, 0x37) as i8) < 4 {
                self.down_forward_kick(me, ps, mv);
            } else {
                self.start_low_kick(me, ps, mv);
            }
        } else if (self.eb(me, 0x37) as i8) < 4 {
            if let Some(mv) = self.pick_by_strength(ps, 0x40976, me) {
                self.down_forward_punch(me, ps, mv);
            } else {
                self.start_unless_current(me, ps, 6);
            }
        } else if let Some(mv) = self.pick_by_strength(ps, 0x4097e, me) {
            self.start_low_punch(me, ps, mv);
        } else {
            self.down_back(me, ps, other);
        }
    }

    /// 0x4098e: down-left held, the mirror of [`Self::input_down_right`]
    /// (kick table 0x40a7e first).
    fn input_down_left(&mut self, ps: u32, me: u16, other: u16) {
        if let Some(mv) = self.pick_by_strength(ps + 1, 0x40a7e, me) {
            if (self.eb(me, 0x37) as i8) < 4 {
                self.start_low_kick(me, ps, mv);
            } else {
                self.down_forward_kick(me, ps, mv);
            }
        } else if (self.eb(me, 0x37) as i8) < 4 {
            if let Some(mv) = self.pick_by_strength(ps, 0x4097e, me) {
                self.start_low_punch(me, ps, mv);
            } else {
                self.down_back(me, ps, other);
            }
        } else if let Some(mv) = self.pick_by_strength(ps, 0x40976, me) {
            self.down_forward_punch(me, ps, mv);
        } else {
            self.start_unless_current(me, ps, 6);
        }
    }

    /// 0x40418: keep +0x37 pointing at the opponent (3 / 5 mark a pending
    /// turn right / left, applied by the stance branch).
    fn keep_facing(&mut self, me: u16, other: u16) {
        let ot = self.center_x(other);
        let my = self.center_x(me);
        if (ot as i16) >= (my as i16) {
            let v = self.eb(me, 0x37) & 2;
            self.set_eb(me, 0x37, if v == 0 { 3 } else { v });
        } else {
            let v = self.eb(me, 0x37) & 4;
            self.set_eb(me, 0x37, if v == 0 { 5 } else { v });
        }
    }

    /// 0x4050c: clear the turn bit; drop latched strengths during an attack.
    fn input_tail(&mut self, me: u16, ps: u32) {
        self.era_finish_input(ps);
        self.set_eb(me, 0x37, self.eb(me, 0x37) & 0xfe);
        if self.eb(me, 9) & 0xc0 == 0xc0
            || (self.toggles.positive_edge_attacks && self.is_in_recovery_or_stun(me))
        {
            self.ww(ps, 0);
            self.clear_neg_edge_latches_for_player(ps);
        }
    }

    /// 0x40b8e: the strength mapper with the strength preloaded (D2); on a
    /// match it clears the byte at `clear` (the caller's A0).
    fn pick_preloaded(&mut self, strength: u8, table: u32, me: u16, clear: u32) -> Option<u8> {
        let d2 = (strength & 0xfe) as u16;
        if d2 <= self.rb(table) as u16 {
            return None;
        }
        let mut d3: u32 = 2;
        while d2 >= self.rb(table + d3) as u16 {
            d3 += 2;
        }
        d3 -= 2;
        let mv = self.rb(table + d3 + 1);
        self.set_eb(me, 0x34, (d3 / 2 + 1) as u8);
        self.wb(clear, 0);
        Some(mv)
    }

    /// 0x40b72 / 0x40b80: map a latched strength (byte at `latch`) through a
    /// (threshold, move) table: above the first threshold, the move of the
    /// last threshold not above it; +0x34 gets the strength level (1..).
    /// Consumes the latch on success.
    pub(crate) fn pick_by_strength(&mut self, latch: u32, table: u32, me: u16) -> Option<u8> {
        let d2 = (self.rb(latch) & 0xfe) as u16;
        if d2 <= self.rb(table) as u16 {
            return None;
        }
        let mut d3: u32 = 2;
        while d2 >= self.rb(table + d3) as u16 {
            d3 += 2;
        }
        d3 -= 2;
        let mv = self.rb(table + d3 + 1);
        self.set_eb(me, 0x34, (d3 / 2 + 1) as u8);
        self.wb(latch, 0);
        Some(mv)
    }

    // ---- Tier 1 input overhaul helpers (#94, O3) --------------------------

    fn latch_slot_index(&self, out: u32) -> Option<usize> {
        match out {
            P1_STRUCT => Some(0),
            x if x == P1_STRUCT + 1 => Some(1),
            CPU_STRUCT if self.dramatic_rules().is_some() => Some(4),
            x if x == CPU_STRUCT + 1 && self.dramatic_rules().is_some() => Some(5),
            P2_STRUCT | CPU_STRUCT => Some(2),
            x if x == P2_STRUCT + 1 || x == CPU_STRUCT + 1 => Some(3),
            _ => None,
        }
    }

    fn player_slot_index(&self, ps: u32) -> Option<usize> {
        match ps {
            P1_STRUCT => Some(0),
            P2_STRUCT if self.dramatic_rules().is_some() => Some(2),
            P2_STRUCT | CPU_STRUCT => Some(1),
            _ => None,
        }
    }

    fn set_neg_edge_latch(&mut self, out: u32, neg_edge: bool) {
        if let Some(idx) = self.latch_slot_index(out) {
            self.toggle_state.neg_edge_latch[idx] = neg_edge;
        }
    }

    fn is_neg_edge_latch(&self, out: u32) -> bool {
        self.latch_slot_index(out)
            .map(|idx| self.toggle_state.neg_edge_latch[idx])
            .unwrap_or(false)
    }

    fn clear_neg_edge_latches_for_player(&mut self, ps: u32) {
        self.set_neg_edge_latch(ps, false);
        self.set_neg_edge_latch(ps + 1, false);
    }

    fn punch_special_ready_for_dir(&self, ps: u32, me: u16, d0: u8) -> bool {
        self.special_ready_for_dir(ps, me, d0, false)
    }

    fn kick_special_ready_for_dir(&self, ps: u32, me: u16, d0: u8) -> bool {
        self.special_ready_for_dir(ps, me, d0, true)
            || (self.evil_ryu_profile(ps) == Some(EvilRyuPlaytest::UltraSf2)
                && self.is_active_ryu(ps) && self.ready_ashura(ps, me, d0).is_some())
    }

    /// A special fires from the button (punch, or `kick`) held at `d0`:
    /// under lenient commands any done command the button fires, at any
    /// direction but up; otherwise a done command whose last direction is
    /// held (forward for the Hadoken's, down-forward for the Shoryuken's,
    /// back for the Tatsumaki's). On a ROM table only the 1987 three fire
    /// ([`Self::tracker_special`]).
    fn special_ready_for_dir(&self, ps: u32, me: u16, d0: u8, kick: bool) -> bool {
        let done = |off| self.command_done(ps, off, kick).is_some();
        if self.toggles.lenient_command_trackers {
            return matches!(d0, 0..=6) && (done(0x1c) || done(0x19) || done(0x16));
        }
        let facing_right = (self.eb(me, 0x37) as i8) < 4;
        let is_forward = (facing_right && d0 == 1) || (!facing_right && d0 == 2);
        let is_down_forward = (facing_right && d0 == 5) || (!facing_right && d0 == 6);
        let is_back = (facing_right && d0 == 2) || (!facing_right && d0 == 1);
        (is_forward && done(0x1c)) || (is_down_forward && done(0x19)) || (is_back && done(0x16))
    }

    fn filter_neg_edge_latches(&mut self, ps: u32, me: u16, d0: u8) {
        if self.is_neg_edge_latch(ps)
            && !self.punch_special_ready_for_dir(ps, me, d0)
            && self.era_special_command(ps, false).is_none()
        {
            self.wb(ps, 0);
            self.set_neg_edge_latch(ps, false);
        }
        if self.is_neg_edge_latch(ps + 1)
            && !self.kick_special_ready_for_dir(ps, me, d0)
            && self.era_special_command(ps, true).is_none()
        {
            self.wb(ps + 1, 0);
            self.set_neg_edge_latch(ps + 1, false);
        }
    }

    fn is_in_recovery_or_stun(&self, me: u16) -> bool {
        let st = self.eb(me, 9);
        if (st & 0x80) == 0 {
            return false;
        }
        let mv = self.eb(me, 0xa);
        !matches!(mv, 0..=3 | 9..=0x0c)
    }

    pub(crate) fn clear_frozen_attack_inputs(&mut self) {
        let mode = self.rb(GAME_MODE);
        let players = match mode {
            _ if self.dramatic_rules().is_some() => [Some(P1_STRUCT), Some(CPU_STRUCT), Some(P2_STRUCT)],
            2 => [Some(P1_STRUCT), Some(CPU_STRUCT), None],
            4 => [Some(P2_STRUCT), Some(CPU_STRUCT), None],
            _ => [Some(P1_STRUCT), Some(P2_STRUCT), None],
        };
        for ps in players.into_iter().flatten() {
            self.ww(ps, 0);
            self.clear_neg_edge_latches_for_player(ps);
            self.era_finish_input(ps);
        }
    }

    fn clear_recovery_buffer(&mut self, ps: u32) {
        if let Some(p_idx) = self.player_slot_index(ps) {
            self.toggle_state.recovery_buffer[p_idx] = None;
        }
    }

    fn record_recovery_buffer_frame(&mut self, ps: u32, me: u16, d0: u8) {
        let Some(p_idx) = self.player_slot_index(ps) else {
            return;
        };
        if !self.toggles.recovery_input_buffer || self.toggles.recovery_buffer_window == 0 {
            self.toggle_state.recovery_buffer[p_idx] = None;
            return;
        }
        if !self.is_in_recovery_or_stun(me) {
            return;
        }
        let b = self.tracker_base(ps);
        let window = self.toggles.recovery_buffer_window;
        if let Some(mut slot) = self.toggle_state.recovery_buffer[p_idx] {
            slot.age = slot.age.saturating_add(1);
            if slot.age > window {
                self.toggle_state.recovery_buffer[p_idx] = None;
            } else {
                let hadoken_now = self.rb(b + 0x1c + 2) != 0;
                let shoryuken_now = self.rb(b + 0x19 + 2) != 0;
                let tatsumaki_now = self.rb(b + 0x16 + 2) != 0;
                slot.hadoken_done |= hadoken_now;
                slot.shoryuken_done |= shoryuken_now;
                slot.tatsumaki_done |= tatsumaki_now;
                if (slot.punch > 0x10 && self.punch_special_ready_for_dir(ps, me, d0))
                    || (slot.kick > 0x10 && self.kick_special_ready_for_dir(ps, me, d0))
                {
                    slot.d0 = d0;
                }
                self.toggle_state.recovery_buffer[p_idx] = Some(slot);
            }
        }
        let punch = self.rb(ps);
        let kick = self.rb(ps + 1);
        if punch > 0x10 || kick > 0x10 {
            // Cancels (SP18): whether this press fired a special as it was
            // made (the slot's own fields keep collecting completions).
            let special_press = (self.toggles.cancels
                && ((punch > 0x10 && self.punch_special_ready_for_dir(ps, me, d0))
                    || (kick > 0x10 && self.kick_special_ready_for_dir(ps, me, d0))))
            .then(|| crate::config::SpecialPress {
                d0,
                done: [0x16u32, 0x19, 0x1c].map(|t| self.rb(b + t + 2) != 0),
            });
            self.toggle_state.recovery_buffer[p_idx] = Some(crate::config::RecoveryBufferSlot {
                punch,
                kick,
                d0,
                hadoken_done: self.rb(b + 0x1c + 2) != 0,
                shoryuken_done: self.rb(b + 0x19 + 2) != 0,
                tatsumaki_done: self.rb(b + 0x16 + 2) != 0,
                age: 1,
                special_press,
            });
            self.ww(ps, 0);
            self.clear_neg_edge_latches_for_player(ps);
        }
    }

    fn apply_recovery_buffer(&mut self, ps: u32, me: u16, d0: u8) -> u8 {
        let Some(p_idx) = self.player_slot_index(ps) else {
            return d0;
        };
        let Some(slot) = self.toggle_state.recovery_buffer[p_idx].take() else {
            return d0;
        };
        if !self.toggles.recovery_input_buffer || slot.age > self.toggles.recovery_buffer_window {
            return d0;
        }
        if self.rb(ps) > 0x10 || self.rb(ps + 1) > 0x10 {
            return d0;
        }
        if slot.punch > 0x10 {
            self.wb(ps, slot.punch);
        }
        if slot.kick > 0x10 {
            self.wb(ps + 1, slot.kick);
        }
        let b = self.tracker_base(ps);
        if slot.hadoken_done {
            self.wb(b + 0x1c + 2, 0xff);
        }
        if slot.shoryuken_done {
            self.wb(b + 0x19 + 2, 0xff);
        }
        if slot.tatsumaki_done {
            self.wb(b + 0x16 + 2, 0xff);
        }
        if slot.punch > 0x10 {
            if self.punch_special_ready_for_dir(ps, me, d0) {
                return d0;
            }
            if self.punch_special_ready_for_dir(ps, me, slot.d0) {
                return slot.d0;
            }
        }
        if slot.kick > 0x10 {
            if self.kick_special_ready_for_dir(ps, me, d0) {
                return d0;
            }
            if self.kick_special_ready_for_dir(ps, me, slot.d0) {
                return slot.d0;
            }
        }
        if matches!(d0, 1..=6) {
            d0
        } else if matches!(slot.d0, 0..=6) {
            slot.d0
        } else {
            0
        }
    }

    // ---- Tier 1 command tracker, walk, and jump overhaul helpers (#95, O4) --

    fn tracker_slot(&self, t: u32) -> Option<(usize, usize)> {
        let count = if self.dramatic_rules().is_some() { 3 } else { 2 };
        let bases = [P1_STRUCT, self.tracker_base(CPU_STRUCT), P2_STRUCT];
        for (p_idx, base) in bases.into_iter().take(count).enumerate() {
            if t == base + 0x16 {
                return Some((p_idx, 0));
            }
            if t == base + 0x19 {
                return Some((p_idx, 1));
            }
            if t == base + 0x1c {
                return Some((p_idx, 2));
            }
        }
        None
    }

    fn clear_all_trackers(&mut self, ps: u32) {
        let b = self.tracker_base(ps);
        self.clear_tracker(b + 0x16);
        self.clear_tracker(b + 0x19);
        self.clear_tracker(b + 0x1c);
    }

    fn tracker_matches_stage(&self, t: u32, pattern: u32, stage: u8, dir: u8) -> bool {
        let is_shoryuken = pattern == 0x403a2;
        match stage {
            0 => {
                if is_shoryuken {
                    // Accept Forward (2) or Down-Forward (6) to start Shoryuken
                    // (enabling crouching DP shortcuts DF, D, DF and DF, DB, DF).
                    dir == 2 || dir == 6
                } else {
                    dir == self.rb(pattern)
                }
            }
            1 => {
                if is_shoryuken {
                    // Accept Down (5) or Down-Back (7) as the middle Shoryuken
                    // direction (enabling F, DB, DF and DF, DB, DF shortcuts).
                    dir == 5 || dir == 7
                } else {
                    dir == self.rb(pattern + 1)
                }
            }
            2 => {
                if is_shoryuken {
                    // Accept Down-Forward (6), or Forward (2) when the Hadoken
                    // tracker (+0x1c) on the same player is not concurrently at
                    // stage >= 2 (enabling DF, D, F while keeping D, DF, F Hadoken).
                    if dir == 6 {
                        true
                    } else if dir == 2 {
                        let hadoken_stage = self.tracker_slot(t)
                            .map(|(_, tracker)| self.rb(t + 6 - 3 * tracker as u32))
                            .unwrap_or(0);
                        hadoken_stage < 2
                    } else {
                        false
                    }
                } else {
                    dir == self.rb(pattern + 2)
                }
            }
            _ => false,
        }
    }

    fn track_command_lenient(&mut self, t: u32, pattern: u32, dir: u8) {
        let limit = self.toggles.command_tracker_window;
        match self.rb(t) {
            1 => {
                if self.tracker_matches_stage(t, pattern, 1, dir) {
                    self.wb(t, 2);
                }
                self.count_tracker(t);
            }
            2 => {
                if self.tracker_matches_stage(t, pattern, 2, dir) {
                    self.wb(t, 3);
                    self.wb(t + 2, 0xff);
                    if let Some((p_idx, t_idx)) = self.tracker_slot(t) {
                        self.toggle_state.tracker_release_age[p_idx][t_idx] = 0;
                    }
                }
                self.count_tracker(t);
            }
            3 => {
                if self.rb(t + 1) >= limit {
                    self.clear_tracker(t);
                } else if self.tracker_matches_stage(t, pattern, 2, dir) {
                    if let Some((p_idx, t_idx)) = self.tracker_slot(t) {
                        self.toggle_state.tracker_release_age[p_idx][t_idx] = 0;
                    }
                    self.wb(t + 1, self.rb(t + 1).wrapping_add(1));
                } else if let Some((p_idx, t_idx)) = self.tracker_slot(t) {
                    let age = self.toggle_state.tracker_release_age[p_idx][t_idx].saturating_add(1);
                    if age > self.toggles.command_release_grace {
                        self.clear_tracker(t);
                    } else {
                        self.toggle_state.tracker_release_age[p_idx][t_idx] = age;
                        self.wb(t + 1, self.rb(t + 1).wrapping_add(1));
                    }
                } else {
                    self.clear_tracker(t);
                }
            }
            s => {
                if self.tracker_matches_stage(t, pattern, 0, dir) {
                    self.wb(t, s.wrapping_add(1));
                    self.wb(t + 1, 0);
                }
            }
        }
    }

    fn lenient_punch_special(&self, me: u16, ps: u32) -> Option<u8> {
        self.lenient_special(me, ps, false)
    }

    fn lenient_kick_special(&self, me: u16, ps: u32) -> Option<u8> {
        self.lenient_special(me, ps, true)
    }

    /// Under lenient commands, the special a button (punch, or `kick`)
    /// fires from any direction: the one whose command is done, or of two
    /// or more the one let go of last (the youngest release age); a tie goes
    /// to the Shoryuken's command when down-forward is held, else the
    /// Hadoken's, then the Tatsumaki's. On a ROM table a punch has only the
    /// Shoryuken and the Hadoken and a kick only the Tatsumaki
    /// ([`Self::tracker_special`]), so this is the 1987 pair's rule.
    fn lenient_special(&self, me: u16, ps: u32, kick: bool) -> Option<u8> {
        if !self.toggles.lenient_command_trackers || self.eb(me, 0x34) < 1 {
            return None;
        }
        // (tracker index of `tracker_release_age`, offset, special).
        let ready: Vec<(usize, u32, u8)> = [(0usize, 0x16u32), (1, 0x19), (2, 0x1c)]
            .into_iter()
            .filter_map(|(i, off)| self.command_done(ps, off, kick).map(|sp| (i, off, sp)))
            .collect();
        if ready.len() <= 1 {
            return ready.first().map(|r| r.2);
        }
        let p_idx = self.player_slot_index(ps).unwrap_or(0);
        let age = |i: usize| self.toggle_state.tracker_release_age[p_idx][i];
        let youngest = ready.iter().map(|r| age(r.0)).min().unwrap_or(0);
        let tied: Vec<&(usize, u32, u8)> = ready.iter().filter(|r| age(r.0) == youngest).collect();
        if tied.len() == 1 {
            return Some(tied[0].2);
        }
        let d0 = self.pad_directions(ps);
        let facing_right = (self.eb(me, 0x37) as i8) < 4;
        let is_down_forward = (facing_right && d0 == 5) || (!facing_right && d0 == 6);
        let pick = |off: u32| tied.iter().find(|r| r.1 == off).map(|r| r.2);
        if is_down_forward {
            if let Some(sp) = pick(0x19) {
                return Some(sp);
            }
        }
        pick(0x1c).or_else(|| pick(0x16)).or_else(|| pick(0x19))
    }

    fn is_jump_airborne(&self, me: u16) -> bool {
        let mv = self.eb(me, 0xa);
        if !matches!(mv, 9..=0x0b) || self.eb(me, 8) == 1 {
            return false;
        }
        let ps = self.el(me, 0x38);
        let table = self.rl(ps + 0x12);
        let Some(header) = crate::data_bank::move_header(|addr| self.rl(addr), table, mv) else {
            return false;
        };
        let init_stream = self.rl(header + 0x2c);
        self.el(me, 0x2c) != init_stream
    }

    fn frames_until_landing(&self, me: u16) -> u16 {
        let mv = self.eb(me, 0xa);
        if !matches!(mv, 9..=0x0b | 0x20..=0x26) || self.eb(me, 8) == 1 {
            return 0;
        }
        let cur_dur = self.eb(me, 0xe) as u16 + 1;
        let mut total =
            self.eb(me, 0x33) as u16 + (self.eb(me, 0xb) as u16).saturating_mul(cur_dur);
        let mut a1 = self.el(me, 0x2c);
        let mut found_landing = false;
        for _ in 0..64 {
            match self.rb(a1) {
                0xff => {
                    let routine = self.rl(a1 + 2);
                    if routine == 0x454ee {
                        found_landing = true;
                        break;
                    }
                    a1 += 6;
                }
                0xfd | 0xfe => break,
                _ => {
                    let hold = self.rb(a1) as u16;
                    let dur = self.rb(a1 + 1) as u16;
                    total = total.saturating_add((hold + 1).saturating_mul(dur + 1));
                    a1 += 0x20;
                }
            }
        }
        if found_landing {
            total
        } else {
            0
        }
    }

    fn clear_air_attack_trajectory(&mut self, ps: u32) {
        if let Some(slot) = self.player_slot_index(ps) {
            self.toggle_state.air_adj_frames_1[slot] = 0;
            self.toggle_state.air_adj_rem_1[slot] = 0;
            self.toggle_state.air_adj_frames_2[slot] = 0;
            self.toggle_state.air_adj_rem_2[slot] = 0;
        }
    }

    fn stream_air_frames_and_dy(&self, mut a1: u32) -> (u8, i16) {
        let mut total_frames = 0u16;
        let mut stream_dy = 0i16;
        for _ in 0..64 {
            match self.rb(a1) {
                0xff => {
                    let routine = self.rl(a1 + 2);
                    if routine == 0x454ee {
                        break;
                    }
                    if crate::data_bank::in_bank(routine)
                        && self.rb(routine) == crate::data_bank::ROUTINE_POSE_ORIGIN
                    {
                        stream_dy = stream_dy.saturating_add(self.rw(routine + 2) as i16);
                    }
                    a1 += 6;
                }
                0xfd | 0xfe => break,
                _ => {
                    let hold = self.rb(a1) as u16;
                    let mut dur = self.rb(a1 + 1) as u16;
                    if self.toggles.balanced_roster && a1 == 0x03b30 {
                        dur = 3;
                    }
                    let fdy = sext8(self.rb(a1 + 6)) as i16;
                    total_frames = total_frames.saturating_add((hold + 1).saturating_mul(dur + 1));
                    stream_dy = stream_dy.saturating_add((hold as i16 + 1).saturating_mul(fdy));
                    a1 += 0x20;
                }
            }
        }
        (total_frames.min(u8::MAX as u16) as u8, stream_dy)
    }

    fn step_air_attack_trajectory(&mut self, me: u16) {
        let slot = self.eb(me, 0xa);
        let air_special = self.era_state(self.el(me, 0x38)).is_some_and(|era| era.alpha_move_airborne(slot) || era.sf4_airborne_move(slot) || era.what_if_airborne(slot));
        if !air_special && (!self.toggles.anytime_jump_attacks || !matches!(slot, 0x20..=0x26)) {
            return;
        }
        let Some(slot) = self.player_slot_index(self.el(me, 0x38)) else {
            return;
        };
        let step_adj = if self.toggle_state.air_adj_frames_1[slot] > 0 {
            let f = self.toggle_state.air_adj_frames_1[slot] as i16;
            let rem = self.toggle_state.air_adj_rem_1[slot];
            let step = rem / f;
            self.toggle_state.air_adj_rem_1[slot] = rem - step;
            self.toggle_state.air_adj_frames_1[slot] -= 1;
            step
        } else if self.toggle_state.air_adj_frames_2[slot] > 0 {
            let f = self.toggle_state.air_adj_frames_2[slot] as i16;
            let rem = self.toggle_state.air_adj_rem_2[slot];
            let step = rem / f;
            self.toggle_state.air_adj_rem_2[slot] = rem - step;
            self.toggle_state.air_adj_frames_2[slot] -= 1;
            step
        } else {
            0
        };
        if step_adj != 0 {
            self.shift_grid_y(me, step_adj as u16);
        }
        if air_special {
            let rows = i32::from(self.eb(me, 0xc)) + 1;
            let cell = if self.is_big(me) { 32 } else { 16 };
            let bottom = i32::from(self.ew(me, 4) as i16) + rows * cell;
            if bottom > crate::data_bank::GROUND {
                self.shift_grid_y(me, (crate::data_bank::GROUND - bottom) as u16);
            }
        }
    }

    fn try_anytime_jump_attack(&mut self, me: u16, other: u16) -> bool {
        let ps = self.el(me, 0x38);
        let table = self.rl(ps + 0x12);
        if !self.is_player_driven(ps)
            || !Self::is_player_table(table)
            || self.eb(me, 9) != 0xc1
            || !self.is_jump_airborne(me)
        {
            return false;
        }
        let rem = self.frames_until_landing(me);
        if rem < 4 {
            // Less than 4 frames before landing (or already in landing frames):
            // drop the late air attack so the jump lands normally (#95, O4).
            self.set_eb(me, 9, 0x81);
            self.set_ew(me, 0x34, 0);
            return false;
        }
        let cy_start = self.center_y(me) as i16;
        let rising = (self.eb(me, 0x14) as i8) < 0;
        // The landing's centre: Ryu's and Ken's 0xb0, a bank table's from
        // its apex record (ADR-0014).
        let (attack, landing_cy) = if crate::data_bank::in_bank(table) {
            let rec = self.rl(table + crate::data_bank::APEX_POINTER);
            (self.jump_attack_bank(me, rec), self.rw(rec + 2) as i16)
        } else if self.kens_side(ps) {
            (self.jump_attack(me, 0x4545c, 0x45474), 0xb0)
        } else {
            (self.jump_attack(me, 0x45386, 0x4539e), 0xb0)
        };
        // Bank attacks use their authored arc and actual rising state.
        // Keep the original shoto early-rise rule on the ROM tables.
        let was_ascending = rising && (crate::data_bank::in_bank(table) || cy_start > 0x90);
        if let Resume::LoadAt(stream) = attack {
            if let Some(slot) = self.player_slot_index(ps) {
                let (total_frames, stream_dy) = self.stream_air_frames_and_dy(stream);
                // Bank air attacks already reach the floor on the last fall
                // frame. Keep the landing pose's offset in stream_dy, but
                // finish applying the trajectory correction before that pose.
                let airborne_frames = if crate::data_bank::in_bank(stream) {
                    total_frames.saturating_sub(1)
                } else {
                    total_frames
                };
                if airborne_frames > 0 {
                    let net_adj = landing_cy - (cy_start + stream_dy);
                    let n1 = airborne_frames / 2;
                    let n2 = airborne_frames - n1;
                    let adj1 = if was_ascending {
                        net_adj - 24
                    } else {
                        net_adj / 2
                    };
                    let adj2 = net_adj - adj1;
                    self.toggle_state.air_adj_frames_1[slot] = n1;
                    self.toggle_state.air_adj_rem_1[slot] = adj1;
                    self.toggle_state.air_adj_frames_2[slot] = n2;
                    self.toggle_state.air_adj_rem_2[slot] = adj2;
                }
            }
            self.set_eb(me, 0x30, self.eb(me, 0x30).wrapping_add(1));
            self.load_frame(me, stream);
            self.step_air_attack_trajectory(me);
            let (me, other) = self.effect_flag_spawn(me, other);
            self.push_apart(me, other);
            self.clamp_y(me);
            self.clamp_x(me);
            self.judge_targets(me, other);
            return true;
        }
        false
    }

    // ---- Tier 1 blocking & shoto symmetry helpers (#96, O5) ---------------

    fn has_incoming_projectile(&self, attacker: u16) -> bool {
        let attacker_ps = self.el(attacker, 0x38);
        (0..12u16).any(|k| {
            let s = 0x100 + 0x40 * k;
            self.eb(s, 8) != 0 && (self.eb(s, 0xe) & 0x80) != 0 && self.el(s, 0x20) == attacker_ps
        })
    }

    fn is_guard_interrupt(&self, me: u16, other: u16, d0: u8) -> bool {
        let facing_right = (self.eb(me, 0x37) as i8) < 4;
        let (back_dir, down_back_dir) = if facing_right { (2, 6) } else { (1, 5) };
        (d0 == back_dir && self.incoming_attack(me, other, false))
            || (d0 == down_back_dir && self.incoming_attack(me, other, true))
    }

    fn alpha_air_blocks(&self, defender: u16, attack_x: u16) -> bool {
        use crate::era::{GaugeMode, Loadout, Revision};

        let ps = self.el(defender, 0x38);
        if !self.world_tour_can_block(ps) { return false; }
        let Some(side) = self.era_side(ps) else { return false };
        let fighter = &self.toggle_state.era.fighters[side];
        let air_guard = self.world_tour_perk(ps, crate::world_tour::native::Perk::AirGuard);
        let can_air_block = matches!(
            (fighter.style.revision, fighter.style.loadout),
            (Revision::Sfz1 | Revision::Sfz2 | Revision::Sfz2A, Loadout::Standard)
                | (Revision::Sfz3, Loadout::AIsm | Loadout::VIsm)
        );
        if (!air_guard && (!can_air_block || !fighter.is_alpha()
            || self.era_gauge_mode(side) == GaugeMode::NoGauges)) || fighter.custom_combo_active()
            || self.era_guard_crushed(ps)
            || self.eb(defender, 9) & 0x40 != 0
            || !self.is_jump_airborne(defender)
            || self.frames_until_landing(defender) == 0
        {
            return false;
        }

        let defender_x = self.center_x(defender);
        let source_on_right = attack_x > defender_x
            || (attack_x == defender_x && self.eb(defender, 0x37) & 4 == 0);
        let away = if source_on_right { 2 } else { 1 };
        self.pad_directions(ps) & 3 == away
    }

    fn block_contact(&mut self, defender: u16) {
        self.queue_sound(8);
        self.training_contact(defender);
    }

    fn incoming_attack(&self, me: u16, other: u16, low: bool) -> bool {
        if !self.world_tour_can_block(self.el(me, 0x38)) {
            return false;
        }
        if self.alpha_state(self.el(me, 0x38)).is_some_and(|era| era.custom_combo_active()) {
            return false;
        }
        let attacks = |e| {
            if low { self.opponent_attacks_low(e) } else { self.opponent_attacks_high(e) }
        };
        let Some(rules) = self.dramatic_rules() else {
            return attacks(other);
        };
        self.dramatic_hit_targets(self.el(me, 0x38), rules.friendly_fire).into_iter()
            .any(|ps| ps != 0 && attacks(self.rw(ps + 0x10)))
    }

    fn cpu_attack_is_blocked(&self, me: u16, other: u16, block_state: u8) -> bool {
        if !self.toggles.standard_high_low_block {
            if self.toggles.fair_cpu_and_sagat_scaling && block_state == 0 {
                // Retsu's 0x468de kick lacked a block check in 1987; make it blockable
                // by standing or crouching guard under fair_cpu_and_sagat_scaling (#97, O6).
                return matches!(self.eb(other, 9), 0x22 | 0x24)
                    || matches!(self.eb(other, 0xa), 4 | 8);
            }
            return block_state != 0 && self.eb(other, 9) == block_state;
        }
        let mv = self.eb(me, 0xa);
        let low_in_air = crate::data_bank::SPECIAL_SLOTS.contains(&mv)
            && self.special_slot(self.el(me, 0x38), mv).low_in_air;
        if block_state == 0x24 || block_state == 0 {
            // CPU low attack (including Retsu's 0x468de low kick which lacked a block test in 1987):
            // blocked only by crouching guard (move 8 / state 0x24).
            self.eb(other, 9) == 0x24 || self.eb(other, 0xa) == 8
        } else if (self.eb(me, 9) & 3) == 1 && !low_in_air {
            // CPU airborne/jumping attack: overhead, blocked only by standing guard (move 4 / state 0x22).
            self.eb(other, 9) == 0x22 || self.eb(other, 0xa) == 4
        } else {
            // CPU grounded high/mid attack, or an airborne special guarded crouching:
            // blocked by either standing or crouching guard.
            matches!(self.eb(other, 9), 0x22 | 0x24) || matches!(self.eb(other, 0xa), 4 | 8)
        }
    }

    // ---- true combos (SP17, #261; `crate::combo`) ------------------------
    //
    // Every gate below runs only with `CoreToggles::true_combos` on; its
    // state is `toggle_state.combo`, in every machine snapshot.

    /// The combo state's slot of fighter entity `e`, by its player
    /// structure (+0x38; [`crate::combo::slot`]).
    fn combo_slot(&self, e: u16) -> usize {
        crate::combo::slot_in_fight(self.el(e, 0x38), self.rb(GAME_MODE), self.dramatic_rules().is_some())
    }

    /// Whether `e` is in a combo-able hit reaction: one started by
    /// [`Self::combo_react`] whose stun is still running.
    fn combo_reacting(&self, e: u16) -> bool {
        self.toggle_state.combo.hitstun[self.combo_slot(e)] != 0
    }

    /// The fighter whose move table the entity's player structure holds
    /// (Ryu, Ken, a boss on his bank or ROM table), for his stun rows.
    fn fighter_of_entity(&self, e: u16) -> Option<crate::roster::TournamentFighter> {
        let ps = self.el(e, 0x38);
        crate::roster::TournamentFighter::from_move_table(self.rl(ps + 0x12))
    }

    /// The stun of the move `me` is in.
    fn move_stun_of(&self, me: u16) -> crate::combo::Stun {
        let level = crate::combo::level(self.eb(me, 0x34));
        let fighter = self.fighter_of_entity(me);
        let mv = self.eb(me, 0xa);
        if self.toggles.balanced_roster {
            if let Some(s) = crate::balanced_roster::second_hit_stun(fighter, mv, self.el(me, 0x2c)) {
                return s;
            }
        }
        let mut stun = crate::combo::move_stun(fighter, mv, level);
        if let Some(delta) = self.sf2_state(self.el(me, 0x38))
            .and_then(|era| crate::era_fight::RevisionDelta::of(era.style))
        {
            if mv == 0x0d && delta.hitstun == 2 {
                stun.hit = 20; // World Warrior's close jab.
            } else if mv == crate::era_fight::SF2_SUPER {
                stun.hit = 22;
                stun.block = 14;
            } else {
                stun.hit = stun.hit.saturating_add_signed(delta.hitstun);
            }
        }
        stun
    }

    /// Once a fight frame, before the input handlers (`drive_fight_inputs`):
    /// count each fighter's blockstun and hitstun down, and end a combo-able
    /// reaction whose hitstun ran out. A human side's reaction ends here, as
    /// its stream's end would (`0xfd`, or for a boss on a player side the CPU
    /// follow-up, which ends a player's move), so its input handler acts
    /// this frame; the 1987 CPU's stream is sent to its end entry, whose
    /// follow-up decision its runner takes this frame as at the stream's
    /// own end.
    pub(crate) fn combo_frame(&mut self) {
        let count = if self.dramatic_rules().is_some() { 3 } else { 2 };
        for e in crate::fight_reader::FIGHTER_SLOTS.into_iter().take(count) {
            let s = self.combo_slot(e);
            let c = &mut self.toggle_state.combo;
            c.blockstun[s] = c.blockstun[s].saturating_sub(1);
            if c.hitstun[s] == 0 {
                continue;
            }
            c.hitstun[s] -= 1;
            if c.hitstun[s] != 0 {
                continue;
            }
            c.fresh[s] = false;
            if self.eb(e, 9) & 0x80 == 0 || !(0x30..=0x3f).contains(&self.eb(e, 0xa)) {
                continue;
            }
            let end = self.stream_end(self.el(e, 0x2c));
            if self.is_player_driven(self.el(e, 0x38)) {
                self.end_move(e);
            } else {
                self.set_el(e, 0x2c, end);
                self.set_eb(e, 0x33, 0);
                self.set_eb(e, 0xb, 0);
            }
        }
    }

    /// The control entry that ends the stream at `a`: its `0xfd`, or the
    /// CPU follow-up call (`0x45630`) a boss's reaction stream ends with.
    /// Frame records are skipped and gotos followed; other calls are
    /// stepped over (no ground reaction stream has one).
    fn stream_end(&self, mut a: u32) -> u32 {
        for _ in 0..64 {
            match self.rb(a) {
                0xfd => return a,
                0xfe => a = self.rl(a + 2),
                0xff if self.rl(a + 2) == 0x45630 => return a,
                0xff => a += 6,
                _ => a += 0x20,
            }
        }
        a
    }

    /// Ticks the stream at `a` shows frames for, up to its end.
    fn stream_ticks(&self, mut a: u32) -> u16 {
        let mut ticks: u16 = 0;
        for _ in 0..64 {
            match self.rb(a) {
                0xfd => break,
                0xfe => a = self.rl(a + 2),
                0xff if self.rl(a + 2) == 0x45630 => break,
                0xff => a += 6,
                hold => {
                    let dur = self.rb(a + 1);
                    ticks = ticks.saturating_add((u16::from(hold) + 1) * (u16::from(dur) + 1));
                    a += 0x20;
                }
            }
        }
        ticks
    }

    /// The animation step met the end entry at `a1` of `me`'s stream: a
    /// combo-able reaction or guard crush holds its last frame through the stun.
    fn combo_holds(&self, me: u16, a1: u32) -> bool {
        self.rb(SCENE_MODE) & 0x80 != 0
            && (self.combo_reacting(me) || self.guard_crush_reaction(me)
                || self.era_holding_crumple(self.el(me, 0x38)) || self.sf6_wall_stun(self.el(me, 0x38)))
            && (self.rb(a1) == 0xfd || self.rl(a1 + 2) == 0x45630)
    }

    /// `load_frame` has copied the frame record at `a1` into `e`.
    /// - In a combo-able reaction, a ground reaction frame becomes hittable:
    ///   the invulnerable flag is cleared and the hurtboxes measured for it
    ///   ([`crate::combo::frame_boxes`]) are written (facing right; the load
    ///   flips them after). The reaction's first frame is held longer when
    ///   the stream is shorter than the hitstun, so the hit pose lasts it.
    /// - A move that has connected keeps its later frames' attack boxes
    ///   dormant: one hit per move.
    fn combo_load_frame(&mut self, e: u16, a1: u32) {
        let s = self.combo_slot(e);
        let guard_crush = self.guard_crush_reaction(e);
        if (self.combo_reacting(e) || guard_crush) && (0x30..=0x3f).contains(&self.eb(e, 0xa)) {
            let boxes = crate::combo::frame_boxes(a1);
            if let Some(boxes) = boxes {
                for (i, b) in boxes.into_iter().enumerate() {
                    self.set_eb(e, 0x1a + i as u32, b);
                }
            }
            if guard_crush || boxes.is_some() {
                self.set_eb(e, 0xf, self.eb(e, 0xf) & !0x08);
            }
            if self.toggle_state.combo.fresh[s] {
                self.toggle_state.combo.fresh[s] = false;
                let this = (u16::from(self.rb(a1)) + 1) * (u16::from(self.rb(a1 + 1)) + 1);
                let rest = this.saturating_add(self.stream_ticks(a1 + 0x20));
                let left = u16::from(self.toggle_state.combo.hitstun[s]);
                if left > rest {
                    let hold = u16::from(self.eb(e, 0x33)) + (left - rest);
                    self.set_eb(e, 0x33, hold.min(0xff) as u8);
                }
            }
        }
        if self.toggle_state.combo.connected[s] {
            self.set_eb(e, 0xf, self.eb(e, 0xf) & 0x7f);
        }
    }

    /// `start_move` on `e`: a new move has not connected, and whatever
    /// reaction or blockstun it was in is over (a guard switch keeps its
    /// blockstun).
    fn combo_new_move(&mut self, e: u16, mv: u8) {
        let s = self.combo_slot(e);
        let c = &mut self.toggle_state.combo;
        c.connected[s] = false;
        c.hitstun[s] = 0;
        c.fresh[s] = false;
        if !matches!(mv, 4 | 8) {
            c.blockstun[s] = 0;
        }
    }

    /// The end of `react_as`: the hit `src` landed by `me` on `other`, whose
    /// reaction is the hit table entry `te`. A ground reaction while alive,
    /// from anything but chip, is combo-able for the hit's stun; anything
    /// else is the 1987 reaction. A hit ends blockstun.
    fn combo_react(&mut self, me: u16, other: u16, te: u32, src: HitSource) {
        let stun = match src {
            HitSource::Melee => {
                let owner = self.el(me, 0x38);
                self.move_stun_of(me).hit.saturating_add(self.sf6_take_rush_bonus(owner))
            }
            HitSource::Projectile => {
                let delta = self.sf2_state(self.el(me, 0x38))
                    .and_then(|era| crate::era_fight::RevisionDelta::of(era.style))
                    .map_or(0, |delta| delta.hitstun);
                crate::combo::PROJECTILE.hit.saturating_add_signed(delta)
            }
            HitSource::Chip => 0,
        };
        let alive = self.eb(other, 8) == 2;
        let ground = crate::combo::derive::ground_state(self.rb(te + 2));
        let s = self.combo_slot(other);
        let c = &mut self.toggle_state.combo;
        let combo = alive && ground && stun != 0;
        c.hitstun[s] = if combo { stun } else { 0 };
        c.fresh[s] = combo;
        c.blockstun[s] = 0;
    }

    /// A guarded melee hit from `me`: a player-driven guard (`other` on the
    /// P1 or P2 structure; the 1987 CPU's guards are its own streams) takes
    /// the move's blockstun.
    fn combo_blocked(&mut self, me: u16, other: u16) {
        self.record_guard_contact(other);
        self.era_normal_contact(me, true);
        self.era_sf4_blocked(self.el(me, 0x38), self.eb(me, 0xa));
        self.sf5_block(me, other);
        let strength = crate::combo::level(self.eb(me, 0x34));
        use crate::combo::Class;
        let pressure = match self.eb(me, 0xa) {
            crate::data_bank::ALPHA_COUNTER_SLOT
            | crate::data_bank::SF2_SPECIAL_SLOT
            | crate::data_bank::SF2_SUPER_SLOT
            | crate::data_bank::ALPHA_SUPER_SECOND_SLOT
            | crate::data_bank::ALPHA_SUPER_THIRD_SLOT => GUARD_SPECIAL_PRESSURE,
            mv => match crate::combo::class_of(mv, strength) {
                Class::Light | Class::Jump(1) => 4,
                Class::Medium | Class::Jump(2) => 7,
                Class::Heavy | Class::Sweep | Class::Jump(_) => 10,
                Class::Special => GUARD_SPECIAL_PRESSURE,
            },
        };
        if self.guard_power_contact(me, other, pressure) {
            return;
        }
        let defender = self.el(other, 0x38);
        if !self.is_player_driven(defender) {
            return;
        }
        let owner = self.el(me, 0x38);
        self.sf6_drain_drive(defender, pressure.saturating_add(12));
        let burnout = self.era_state(defender).is_some_and(|state| state.sf6_burnout());
        let center = self.center_x(other);
        if self.sf6_impact_block(owner, defender, !(0x86..=0x17a).contains(&center)) {
            use crate::impact::{ImpactDirection, ImpactKind};
            let (x, direction) = if center > 0x17a {
                (0x17a, ImpactDirection::Left)
            } else {
                (0x86, ImpactDirection::Right)
            };
            self.record_surface_impact(other, ImpactKind::WallSlam, x, self.center_y(other), direction);
            self.react_one(me, other, 0x45b6e, HitSource::Chip);
            return;
        }
        let block = self.move_stun_of(me).block.saturating_add(self.sf6_take_rush_bonus(owner))
            .saturating_add(if burnout { 4 } else { 0 });
        if block > 0 && matches!(self.eb(other, 0xa), 4 | 8) && self.can_alpha_counter(defender) {
            self.alpha_guard_contact(other, block);
        } else if self.era_true_combos(owner)
            || self.era_state(defender).is_some_and(|state| state.uses_family_bank())
        {
            let slot = self.combo_slot(other);
            self.toggle_state.combo.blockstun[slot] = block;
        }
    }

    fn record_guard_contact(&mut self, defender: u16) {
        let ps = self.el(defender, 0x38);
        let mode = self.rb(GAME_MODE);
        for side in sf_video::callouts::Side::BOTH {
            if ps == crate::fight_reader::player_struct(side, mode) {
                self.toggle_state.era.guard_contacts[1 - side.index()] = true;
                break;
            }
        }
    }

    fn guard_power_contact(&mut self, attacker: u16, defender: u16, pressure: u8) -> bool {
        let ps = self.el(defender, 0x38);
        if !self.era_guard_contact(ps, pressure) {
            return false;
        }
        let slot = self.combo_slot(defender);
        self.toggle_state.combo.blockstun[slot] = 0;
        self.clear_all_trackers(ps);
        self.clear_recovery_buffer(ps);
        self.ww(ps, 0);
        self.clear_neg_edge_latches_for_player(ps);
        // A grounded guard breaks into the existing stagger without taking
        // the blocked attack's damage. A jumping defender finishes the jump,
        // vulnerable and without access to a second air guard.
        if !self.is_jump_airborne(defender) {
            self.react_one(attacker, defender, 0x45b6e, HitSource::Chip);
        }
        self.set_eb(defender, 0xf, self.eb(defender, 0xf) & !0x88);
        true
    }

    fn guard_crush_reaction(&self, entity: u16) -> bool {
        let move_id = self.eb(entity, 0xa);
        self.eb(entity, 8) == 2
            && (0x30..=0x3f).contains(&move_id)
            && move_id != crate::combo::throw::THROWN_MOVE
            && self.eb(entity, 9) & 0x81 == 0x80
            && self.era_guard_crushed(self.el(entity, 0x38))
    }

    fn alpha_guard_contact(&mut self, entity: u16, frames: u8) {
        let ps = self.el(entity, 0x38);
        let slot = self.combo_slot(entity);
        if self.toggle_state.combo.blockstun[slot] == 0 {
            let relative = self.relative_directions(entity, self.pad_directions(ps));
            self.era_reset_alpha_counter(ps);
            self.era_track_alpha_counter(ps, relative);
        }
        self.toggle_state.combo.blockstun[slot] = frames;
        // Alpha meter uses the native contact rates, with a medium guard award.
        self.era_meter_gain(ps, 3);
    }

    /// The input handler of `ps` (entity `me`, direction `d0`) in blockstun:
    /// the guard holds, and may only switch between high (back held, move 4)
    /// and low (down-back, move 8). Returns whether the fighter is locked.
    fn combo_blockstun(&mut self, ps: u32, me: u16, d0: u8) -> bool {
        let s = self.combo_slot(me);
        if self.toggle_state.combo.blockstun[s] == 0 {
            return false;
        }
        if matches!(self.eb(me, 0xa), 4 | 8) {
            let facing_left = self.eb(me, 0x37) & 4 != 0;
            let (back, down_back) = if facing_left { (1, 5) } else { (2, 6) };
            let want = if d0 == down_back {
                8
            } else if d0 == back {
                4
            } else {
                self.eb(me, 0xa)
            };
            if want != self.eb(me, 0xa) {
                self.start_move(me, ps, want);
            }
        }
        true
    }

    /// Combo damage scaling: a hit on the fighter of player structure `ps`
    /// while his combo-able reaction runs is the combo's next hit
    /// ([`crate::combo::scaled`]); any other starts a new count.
    fn combo_scale(&mut self, ps: u32, dmg: u8, air_juggle: bool) -> u8 {
        if !matches!(ps, P1_STRUCT | P2_STRUCT | CPU_STRUCT) {
            return dmg;
        }
        let s = crate::combo::slot_in_fight(ps, self.rb(GAME_MODE), self.dramatic_rules().is_some());
        let c = &self.toggle_state.combo;
        let n = if c.hitstun[s] != 0 || air_juggle {
            c.hits[s].saturating_add(1)
        } else {
            1
        };
        self.toggle_state.combo.hits[s] = n;
        crate::combo::scaled(dmg, n)
    }

    // ---- cancels (SP18, #262; `crate::combo::cancel`) --------------------
    //
    // Every gate below runs only with `CoreToggles::cancels` on; its state
    // is `toggle_state.cancel`, in every machine snapshot.

    /// Close `e`'s cancel window.
    fn cancel_close(&mut self, e: u16) {
        let s = self.combo_slot(e);
        self.toggle_state.cancel.window[s] = 0;
    }

    /// `me`'s move connected (hit or guarded; the judge's hit function
    /// cleared its active bit): a normal the move data marks cancelable
    /// ([`crate::combo::cancel::window`]), on a player's table, opens its
    /// window. The 1987 CPU's attacks (on its own structure) and a boss on
    /// the boss-only controls (on his CPU table) never cancel.
    fn cancel_open(&mut self, me: u16) {
        let ps = self.el(me, 0x38);
        if !self.is_player_driven(ps) || !Self::is_player_table(self.rl(ps + 0x12)) {
            return;
        }
        let mv = self.eb(me, 0xa);
        let level = crate::combo::level(self.eb(me, 0x34));
        let window = crate::combo::cancel::window(self.fighter_of_entity(me), mv, level);
        if window == 0 {
            return;
        }
        let s = self.combo_slot(me);
        self.toggle_state.cancel.window[s] = window;
        self.toggle_state.cancel.move_id[s] = mv;
    }

    /// The input handler of `ps` (entity `me`, direction `d0`), before its
    /// busy gate: with the fighter's window open, count it down and cancel
    /// the normal if a special's command fires. A press this frame decides;
    /// with none, a special pressed in the frames before the contact, held
    /// in the recovery input buffer, is taken. Returns whether the normal
    /// was cancelled.
    fn cancel_gate(&mut self, ps: u32, me: u16, other: u16, d0: u8) -> bool {
        let s = self.combo_slot(me);
        let left = self.toggle_state.cancel.window[s];
        if left == 0 {
            return false;
        }
        if self.eb(me, 9) & 0x80 == 0 || self.eb(me, 0xa) != self.toggle_state.cancel.move_id[s] {
            self.toggle_state.cancel.window[s] = 0;
            return false;
        }
        self.toggle_state.cancel.window[s] = left - 1;
        if self.latch_fires(ps, false) || self.latch_fires(ps + 1, true) {
            self.cancel_into_special(ps, me, other, d0)
        } else {
            self.cancel_from_buffer(ps, me, other)
        }
    }

    /// A latched strength (punch at `latch`, or `kick`) that the dispatch's
    /// strength tables map to a move: above their first threshold. Every
    /// table of the dispatch starts with the same one (`0x10`,
    /// `docs/reference/posture-dispatch.md`), read here from the neutral
    /// punch table (`0x405b6`) and kick table (`0x405be`).
    fn latch_fires(&self, latch: u32, kick: bool) -> bool {
        let table = if kick { 0x405be } else { 0x405b6 };
        (self.rb(latch) & 0xfe) > self.rb(table)
    }

    /// Cancel into the special that a latched button fires at direction
    /// `dir`, if one does ([`Self::special_ready_for_dir`]: the 1987 rule,
    /// strict or lenient). The other button's latch is dropped, so the 1987
    /// dispatch, run on `dir`, can only start that special, with the same
    /// strength and tracker clears as the input on a free frame. A press
    /// that fires no special does nothing (no chains into normals).
    fn cancel_into_special(&mut self, ps: u32, me: u16, other: u16, dir: u8) -> bool {
        if !matches!(dir, 0 | 1 | 2 | 4 | 5 | 6) {
            return false;
        }
        let punch = self.latch_fires(ps, false) && self.special_ready_for_dir(ps, me, dir, false);
        let kick = !punch
            && self.latch_fires(ps + 1, true)
            && self.special_ready_for_dir(ps, me, dir, true);
        if !punch && !kick {
            return false;
        }
        self.wb(if punch { ps + 1 } else { ps }, 0);
        self.clear_recovery_buffer(ps);
        self.dispatch_direction(ps, me, other, dir, dir);
        debug_assert!(
            crate::data_bank::SPECIAL_SLOTS.contains(&self.eb(me, 0xa)),
            "a cancel starts a special, not {:#04x}",
            self.eb(me, 0xa)
        );
        let s = self.combo_slot(me);
        self.toggle_state.cancel.count[s] = self.toggle_state.cancel.count[s].wrapping_add(1);
        true
    }

    /// No press this frame: a special pressed before the window opened
    /// waits in the recovery input buffer (the busy gate moved it there).
    /// Only a press that fired a special as it was made counts
    /// ([`crate::config::RecoveryBufferSlot::special_press`]): its latches,
    /// its direction and the trackers' done flags as they were then are put
    /// back and it cancels with them. A command completed after the press
    /// (say a normal's button mashed on down-back, then back) does not turn
    /// it into a special. If it fires none, the latches, the done bytes and
    /// the buffer are put back as they were, for the normal's end.
    fn cancel_from_buffer(&mut self, ps: u32, me: u16, other: u16) -> bool {
        let Some(p) = self.player_slot_index(ps) else {
            return false;
        };
        let Some(slot) = self.toggle_state.recovery_buffer[p] else {
            return false;
        };
        let Some(press) = slot.special_press else {
            return false;
        };
        if !self.toggles.recovery_input_buffer || slot.age > self.toggles.recovery_buffer_window {
            return false;
        }
        let b = self.tracker_base(ps);
        let latches = self.rw(ps);
        let done = [0x16u32, 0x19, 0x1c].map(|t| self.rb(b + t + 2));
        self.wb(ps, if slot.punch > 0x10 { slot.punch } else { 0 });
        self.wb(ps + 1, if slot.kick > 0x10 { slot.kick } else { 0 });
        for (t, d) in [0x16u32, 0x19, 0x1c].into_iter().zip(press.done) {
            self.wb(b + t + 2, if d { 0xff } else { 0 });
        }
        if self.cancel_into_special(ps, me, other, press.d0) {
            return true;
        }
        self.ww(ps, latches);
        for (t, d) in [0x16u32, 0x19, 0x1c].into_iter().zip(done) {
            self.wb(b + t + 2, d);
        }
        self.toggle_state.recovery_buffer[p] = Some(slot);
        false
    }

    // ---- throws and throw techs (SP19, #263; `crate::combo::throw`) -------
    //
    // Every gate below runs only with `CoreToggles::throws` on; its state is
    // `toggle_state.throw`, in every machine snapshot.

    /// Once a fight frame, before the input handlers (`drive_fight_inputs`):
    /// count each fighter's throw tech window and post-wakeup throw
    /// invulnerability down.
    pub(crate) fn throw_frame(&mut self) {
        let t = &mut self.toggle_state.throw;
        for s in 0..t.tech_window.len() {
            t.tech_window[s] = t.tech_window[s].saturating_sub(1);
            t.wakeup_invuln[s] = t.wakeup_invuln[s].saturating_sub(1);
        }
    }

    /// Start the `0x30`-byte move header at `src` (in the extended data bank)
    /// on entity `e` (player structure `ps`), keeping its centre at `d3`.
    fn start_move_header(&mut self, e: u16, ps: u32, src: u32, mut d3: u16) {
        self.control_move_started(ps, self.rb(src + 0xa));
        for i in 0..0x30u32 {
            let b = self.rb(src + i);
            self.set_eb(e, i, b);
        }
        let mv = self.eb(e, 0xa);
        self.era_clear_alpha_move(ps, mv);
        self.era_sf4_started(ps, mv);
        self.sf5_on_move_started(ps, mv);
        self.sf6_started(ps, mv);
        self.toggle_state.dp_connected[self.player_slot_index(ps).unwrap_or(1)] = false;
        if let Some(slot) = self.player_slot_index(ps) {
            self.toggle_state.human_attack_frames[slot] = 0;
        }
        if self.toggles.true_combos {
            self.combo_new_move(e, mv);
        }
        if self.toggles.cancels {
            self.cancel_close(e);
        }
        self.clear_air_attack_trajectory(ps);
        if d3 != 0 {
            d3 = d3.wrapping_sub(Self::half_extent(self.eb(e, 0xd), self.is_big(e)));
            if (self.eb(e, 0x37) as i8) >= 4 {
                let x = self.ew(e, 6).wrapping_neg();
                self.set_ew(e, 6, x);
            }
            let x = self.ew(e, 6).wrapping_add(d3);
            self.set_ew(e, 6, x);
        }
        let (sy, sx, code, attr) = (self.ew(e, 4), self.ew(e, 6), self.ew(e, 0), self.ew(e, 2));
        self.write_grid(e, code, attr, sx, sy);
    }

    /// Whether `me` (player structure `ps`, direction `dir`) starts a throw on
    /// `other` this frame: `dir` is forward or back (`1` or `2`), heavy punch
    /// or heavy kick is latched (`0xfe`), no special command fires for that
    /// direction and button, and `other` is throwable and within
    /// [`crate::combo::throw::RANGE`].
    pub(crate) fn try_throw(&mut self, ps: u32, me: u16, other: u16, dir: u8) -> bool {
        if self.evil_ryu_sf4(ps) {
            return self.try_evil_sf4_throw(ps, me, other, dir);
        }
        let macro_throw = self.player_slot_index(ps)
            .is_some_and(|side| self.toggle_state.schemes[side].throw_macro);
        if (!matches!(dir, 1 | 2) && !(dir == 0 && macro_throw))
            || self.chosen_era(ps) == Some(crate::era::EraFamily::Sf1987)
        {
            return false;
        }
        let alpha3 = self.evil_ryu_alpha3(ps);
        let (hp, hk) = if macro_throw {
            (true, false)
        } else if alpha3 {
            self.alpha3_throw_buttons(ps)
        } else {
            (self.rb(ps) == 0xfe, self.rb(ps + 1) == 0xfe)
        };
        let kick_throw = !hp && (alpha3 && hk
            || (self.evil_ryu_profile(ps) == Some(EvilRyuPlaytest::Alpha2)
                && self.era_buttons(ps).1 & 0b110000 != 0));
        if !hp && !hk && !kick_throw {
            return false;
        }
        if hp && self.punch_special_ready_for_dir(ps, me, dir) {
            return false;
        }
        if (hk || kick_throw) && self.kick_special_ready_for_dir(ps, me, dir) {
            return false;
        }
        if !self.can_throw(me, other) {
            return false;
        }
        if kick_throw {
            let Some((throw_header, thrown_header)) = self.data_bank.and_then(|bank| bank.evil_ryu_kick_throw()) else {
                return false;
            };
            self.start_throw(ps, me, other, throw_header, thrown_header, true);
            return true;
        }
        let Some((me_entry, other_entry)) = self.throw_entries(me, other) else {
            return false;
        };
        let facing_right = (self.eb(me, 0x37) as i8) < 4;
        let back_throw = (facing_right && dir == 2) || (!facing_right && dir == 1);
        self.start_throw(ps, me, other, me_entry.throw_header, other_entry.thrown_header, back_throw);
        true
    }

    fn light_throw_chord(&self, ps: u32) -> bool {
        let (held, pressed) = self.era_buttons(ps);
        let light_pair = 0b001001;
        held == light_pair && pressed & light_pair != 0
    }

    fn try_evil_sf4_throw(&mut self, ps: u32, me: u16, other: u16, dir: u8) -> bool {
        if !matches!(dir, 0..=2) || !self.light_throw_chord(ps) || !self.can_throw(me, other) {
            return false;
        }
        let back = self.relative_directions(me, dir) == 2;
        let pair = if back {
            self.data_bank.and_then(|bank| bank.evil_ryu_kick_throw())
        } else {
            self.throw_entries(me, other).map(|(attacker, defender)| (attacker.throw_header, defender.thrown_header))
        };
        let Some((throw_header, thrown_header)) = pair else { return false; };
        self.start_throw(ps, me, other, throw_header, thrown_header, back);
        true
    }

    fn alpha3_throw_buttons(&self, ps: u32) -> (bool, bool) {
        let (held, pressed) = self.era_buttons(ps);
        let [punch, kick] = [0b000111, 0b111000].map(|mask| {
            (held & mask).count_ones() == 2 && pressed & mask != 0
        });
        (punch, kick)
    }

    fn height_above_mat(&self, fighter: u16) -> u16 {
        let bottom = self.center_y(fighter).saturating_add(u16::from(self.eb(fighter, 0x17)));
        (crate::data_bank::GROUND as u16).saturating_sub(bottom)
    }

    fn try_air_shoulder_throw(&mut self, ps: u32, me: u16, other: u16, dir: u8) -> bool {
        use crate::combo::throw::RANGE;

        let horizontal = dir & 3;
        if !self.evil_ryu_alpha3(ps)
            || !matches!(horizontal, 1 | 2) || self.alpha3_throw_buttons(ps) != (true, false)
        {
            return false;
        }

        let height = self.height_above_mat(me);
        // Some jump kicks switch to the falling posture before touchdown.
        let target_airborne = self.era_airborne(other) || self.frames_until_landing(other) > 0;
        if height < 8 || self.height_above_mat(other) < 8 || !target_airborne
            || !self.throw_target_is_vulnerable(other)
            || self.center_x(me).abs_diff(self.center_x(other)) > RANGE
            || self.center_y(me).abs_diff(self.center_y(other)) > 40
        {
            return false;
        }

        let Some((throw_header, thrown_header)) = self.data_bank.and_then(|bank| bank.evil_ryu_air_throw()) else {
            return false;
        };
        let toward = if self.center_x(other) >= self.center_x(me) { 1 } else { 2 };
        self.start_throw(ps, me, other, throw_header, thrown_header, horizontal != toward);
        for fighter in [me, other] {
            self.shift_grid_y(fighter, height.wrapping_neg());
        }
        true
    }

    fn throw_entries(&self, me: u16, other: u16) -> Option<(crate::data_bank::ThrowEntry, crate::data_bank::ThrowEntry)> {
        let bank = self.data_bank?;
        Some((
            bank.throw_entry(self.fighter_of_entity(me)?)?,
            bank.throw_entry(self.fighter_of_entity(other)?)?,
        ))
    }

    /// Whether `me` can throw `other` right now: `me` is grounded, and `other`
    /// is alive, grounded, within [`crate::combo::throw::RANGE`], and not in
    /// hitstun, blockstun, a throw, a get-up, post-wakeup throw
    /// invulnerability, or an invulnerable frame.
    pub(crate) fn can_throw(&self, me: u16, other: u16) -> bool {
        self.can_throw_at_range(me, other, crate::combo::throw::RANGE)
    }

    fn can_throw_at_range(&self, me: u16, other: u16, range: u16) -> bool {
        if self.sf5_throw_invulnerable(self.el(other, 0x38)) {
            return false;
        }
        if self.dramatic_rules().is_some() {
            let owner = self.el(me, 0x38);
            let target = self.el(other, 0x38);
            if !self.dramatic_alive(target) || (owner == CPU_STRUCT) == (target == CPU_STRUCT) {
                return false;
            }
        }
        let ps = self.el(me, 0x38);
        let can_throw_from_walk = (self.evil_ryu_alpha3(ps) || self.evil_ryu_sf4(ps))
            && matches!(self.eb(me, 0xa), 2 | 3);
        if (self.eb(me, 9) & 3 != 2 && !can_throw_from_walk)
            || self.eb(other, 9) & 3 == 1 || !self.throw_target_is_vulnerable(other)
        {
            return false;
        }
        self.center_x(me).abs_diff(self.center_x(other)) <= range
    }

    fn throw_target_is_vulnerable(&self, other: u16) -> bool {
        if self.eb(other, 8) != 2 || self.eb(other, 0xf) & 0x08 != 0 {
            return false;
        }
        let other_mv = self.eb(other, 0xa);
        if (0x30..=0x3f).contains(&other_mv)
            || other_mv == crate::combo::throw::THROW_MOVE
            || other_mv == crate::combo::throw::GET_UP_MOVE
        {
            return false;
        }
        let other_slot = self.combo_slot(other);
        self.toggle_state.combo.hitstun[other_slot] == 0
            && self.toggle_state.combo.blockstun[other_slot] == 0
            && self.toggle_state.throw.wakeup_invuln[other_slot] == 0
    }

    fn try_alpha_catch(&mut self, me: u16, other: u16, range: u16, damage: u8) -> bool {
        if !self.can_throw_at_range(me, other, range) {
            return false;
        }
        let ps = self.el(me, 0x38);
        let Some(side) = self.era_side(ps) else { return false };
        let entries = self.data_bank.and_then(|bank| {
            let attacker = bank.throw_entry(self.fighter_of_entity(me)?)?;
            let defender = bank.throw_entry(self.fighter_of_entity(other)?)?;
            Some((attacker, defender))
        });
        let Some((attacker, defender)) = entries else { return false };
        self.start_throw(ps, me, other, attacker.throw_header, defender.thrown_header, false);
        let victim_slot = self.combo_slot(other);
        self.toggle_state.throw.tech_window[victim_slot] = 0;
        self.toggle_state.throw.crosses[victim_slot] = false;
        self.toggle_state.era.fighters[side].command_throw_damage = damage;
        true
    }

    /// Start `me`'s throw (`throw_header`) and `other`'s being-thrown reaction
    /// (`thrown_header`), opening `other`'s 7-frame tech window.
    pub(crate) fn start_throw(
        &mut self,
        ps: u32,
        me: u16,
        other: u16,
        throw_header: u32,
        thrown_header: u32,
        back_throw: bool,
    ) {
        let other_ps = self.el(other, 0x38);
        if let Some(side) = self.era_side(ps) {
            self.toggle_state.era.fighters[side].command_throw_damage = 0;
        }
        self.clear_all_trackers(ps);
        self.ww(ps, 0);
        self.clear_recovery_buffer(ps);
        if self.is_player_driven(other_ps) {
            self.clear_all_trackers(other_ps);
            self.clear_recovery_buffer(other_ps);
        }
        let gap = crate::combo::throw::GRAB_GAP;
        let (left, right) = self.throw_bounds();
        let to_right = self.center_x(other) >= self.center_x(me);
        let (my_cx, ot_cx) = if to_right {
            let my = self.center_x(me).clamp(left, right - gap);
            (my, (my + gap).min(right))
        } else {
            let my = self.center_x(me).clamp(left + gap, right);
            (my, my.saturating_sub(gap).max(left))
        };
        self.set_eb(me, 0x37, if to_right { 2 } else { 4 });
        self.set_eb(other, 0x37, if to_right { 4 } else { 2 });
        self.start_move_header(me, ps, throw_header, my_cx);
        self.start_move_header(other, other_ps, thrown_header, ot_cx);
        let other_slot = self.combo_slot(other);
        if self.dramatic_rules().is_some() {
            let slot = self.combo_slot(me);
            self.toggle_state.throw.partners[slot] = other_ps;
            self.toggle_state.throw.partners[other_slot] = ps;
            self.bind_dramatic_target(other_ps, ps);
        }
        let untechable = self.chosen_era(other_ps) == Some(crate::era::EraFamily::Sf1987)
            || (self.sf2_state(ps).is_some() && self.era_rules.untechable_sf2_throws);
        self.toggle_state.throw.tech_window[other_slot] =
            if untechable { 0 } else { crate::combo::throw::TECH_WINDOW + 1 };
        self.toggle_state.throw.crosses[other_slot] = self.fighter_of_entity(me)
            .map_or(back_throw, |fighter| crate::combo::throw::crosses_original_side(fighter, back_throw));
    }

    fn separate_corner_throw(&mut self, victim: u16) {
        use crate::combo::throw::{RANGE, TECH_PUSH};
        use crate::era::EraFamily::{Sf1Plus, Sf2};

        let thrower_ps = self.el(victim, 0x3c);
        if self.era_rules.allow_corner_throw_loops
            || !matches!(self.chosen_era(thrower_ps), Some(Sf1Plus | Sf2))
        {
            return;
        }
        let thrower = self.rw(thrower_ps + 0x10);
        let victim_x = self.center_x(victim);
        let thrower_x = self.center_x(thrower);
        if victim_x.abs_diff(thrower_x) > RANGE {
            return;
        }
        if victim_x <= 0x86 && thrower_x >= victim_x {
            self.shift_grid_x(thrower, TECH_PUSH);
        } else if victim_x >= 0x17a && thrower_x <= victim_x {
            self.shift_grid_x(thrower, TECH_PUSH.wrapping_neg());
        }
    }

    fn throw_bounds(&self) -> (u16, u16) {
        if self.dramatic_rules().is_some() {
            let origin = sf_video::render::VISAREA_X as u16;
            (origin + 16, origin + crate::dramatic::VIEW_W as u16 - 16)
        } else {
            (0x70, 0x290)
        }
    }

    /// In `me`'s 7-frame tech window while being thrown, pressing forward or
    /// back (`d0` 1 or 2) with heavy punch or heavy kick latched (`0xfe`) techs
    /// the throw: both fighters return to stance (`move 1`), push apart by
    /// [`crate::combo::throw::TECH_PUSH`] pixels, and take no damage.
    fn try_throw_tech(&mut self, ps: u32, me: u16, other: u16, d0: u8) -> bool {
        let s = self.combo_slot(me);
        if self.toggle_state.throw.tech_window[s] == 0
            || self.eb(me, 0xa) != crate::combo::throw::THROWN_MOVE
            || self.eb(me, 9) & 0x80 == 0
        {
            return false;
        }
        let heavy = self.rb(ps) == 0xfe || self.rb(ps + 1) == 0xfe;
        let era_light = self.chosen_era(ps).is_some_and(|family| family != crate::era::EraFamily::Sf1987);
        let sf2 = self.sf2_state(ps).is_some() || self.sf2_state(self.el(other, 0x38)).is_some();
        let light = |at, table| {
            let latch = self.rb(at) & 0xfe;
            latch > self.rb(table) && latch < self.rb(table + 2)
        };
        let light_chord = (era_light || sf2) && light(ps, 0x405b6) && light(ps + 1, 0x405be);
        let tech = if self.toggle_state.schemes[s].throw_macro {
            true
        } else if self.evil_ryu_sf4(ps) {
            self.light_throw_chord(ps)
        } else if self.evil_ryu_alpha3(ps) {
            let (punch, kick) = self.alpha3_throw_buttons(ps);
            matches!(d0, 1 | 2) && (punch || kick)
        } else {
            light_chord || (matches!(d0, 1 | 2) && heavy)
        };
        if !tech {
            return false;
        }
        self.tech_throw(ps, me, other);
        true
    }

    /// Execute a throw tech between victim `me` (`ps`) and thrower `other`.
    pub(crate) fn tech_throw(&mut self, ps: u32, me: u16, other: u16) {
        if self.dramatic_rules().is_some() {
            for entity in [me, other] {
                let slot = self.combo_slot(entity);
                self.toggle_state.throw.tech_window[slot] = 0;
                self.toggle_state.throw.crosses[slot] = false;
                self.toggle_state.throw.partners[slot] = 0;
            }
        } else {
            self.toggle_state.throw.tech_window.fill(0);
            self.toggle_state.throw.crosses.fill(false);
        }
        self.ww(ps, 0);
        self.clear_recovery_buffer(ps);
        let other_ps = self.el(other, 0x38);
        if self.evil_ryu_profile(other_ps).is_some() {
            self.toggle_state.raging_demon.active = false;
            self.clear_raging_demon_blackout();
        }
        if self.is_player_driven(other_ps) {
            self.ww(other_ps, 0);
            self.clear_recovery_buffer(other_ps);
        }
        self.start_move(me, ps, 1);
        self.end_move(me);
        self.start_move(other, other_ps, 1);
        self.end_move(other);
        let half = crate::combo::throw::TECH_PUSH / 2;
        let (left, right) = self.throw_bounds();
        let (my_cx, ot_cx) = (self.center_x(me), self.center_x(other));
        let (new_me, new_ot) = if my_cx <= ot_cx {
            (my_cx.saturating_sub(half).max(left), (ot_cx + half).min(right))
        } else {
            ((my_cx + half).min(right), ot_cx.saturating_sub(half).max(left))
        };
        self.shift_grid_x(me, new_me.wrapping_sub(my_cx));
        self.shift_grid_x(other, new_ot.wrapping_sub(ot_cx));
        self.queue_sound(6);
    }

    /// At the beginning of a crossing arc, turn the thrower and victim
    /// without relocating either fighter. The airborne frame steps carry
    /// the victim continuously through to the thrower's other side.
    fn throw_turn_for_crossing(&mut self, thrower: u16, victim: u16) {
        let origins = [thrower, victim].map(|e| {
            self.center_x(e).wrapping_sub(sext8(self.eb(e, 0x26)))
        });
        let thrower_was_right = (self.eb(thrower, 0x37) as i8) < 4;
        // Depending on player order, the thrower may already have loaded
        // his turned pose this tick, or may still show the grab. Mirror the
        // currently displayed pose according to its own authored direction.
        let frame = self.el(thrower, 0x2c).wrapping_sub(0x20);
        let back_turned = self.bank_throw_pose_faces_back(thrower, frame);
        if thrower_was_right {
            self.set_eb(thrower, 0x37, 4);
            if back_turned { self.face_right(thrower); } else { self.face_left(thrower); }
            self.set_eb(victim, 0x37, 2);
            self.face_right(victim);
        } else {
            self.set_eb(thrower, 0x37, 2);
            if back_turned { self.face_left(thrower); } else { self.face_right(thrower); }
            self.set_eb(victim, 0x37, 4);
            self.face_left(victim);
        }
        for (e, origin) in [thrower, victim].into_iter().zip(origins) {
            // A facing change reverses the pose's x-offset as well as its
            // tiles. Keep the fighter's world origin fixed under that new
            // anchor instead of moving him by twice the old x-offset.
            let now = self.center_x(e).wrapping_sub(sext8(self.eb(e, 0x26)));
            self.shift_grid_x(e, origin.wrapping_sub(now));
        }
    }

    /// [`crate::data_bank::ROUTINE_THROW_SLAM`]: `victim` hits the mat from
    /// `thrower`'s throw.
    fn throw_slam(&mut self, victim: u16, thrower: u16) -> bool {
        let victim_slot = self.combo_slot(victim);
        self.toggle_state.throw.crosses[victim_slot] = false;
        self.training_contact(victim);
        self.queue_sound(9);
        let thrower_ps = self.el(thrower, 0x38);
        if matches!(thrower_ps, P1_STRUCT | P2_STRUCT) {
            self.add_score(thrower, self.rb(0x4637e));
            self.show_score(thrower_ps);
            self.check_hiscore(thrower);
        }
        let victim_ps = self.el(victim, 0x38);
        let primary = self.el(thrower, 0x3c);
        let dramatic = self.dramatic_rules().is_some();
        if dramatic {
            self.bind_dramatic_target(thrower_ps, victim_ps);
        }
        let command_damage = self.era_side(thrower_ps).map_or(0, |side| {
            std::mem::take(&mut self.toggle_state.era.fighters[side].command_throw_damage)
        });
        let demon = self.evil_ryu_profile(thrower_ps).is_some() && self.toggle_state.raging_demon.active;
        let damage = if demon {
            if self.evil_ryu_sf4(thrower_ps) { 18 } else { DEMON_DAMAGE }
        } else if command_damage == 0 {
            crate::combo::throw::DAMAGE
        } else {
            command_damage
        };
        self.damage_with_chip(thrower, damage, false);
        if demon {
            self.toggle_state.raging_demon.active = false;
        }
        if !self.defender_alive(victim_ps) {
            self.set_eb(victim, 8, 8);
            self.set_eb(victim, 0xa, 0x40);
            self.dramatic_shared_knockdown(thrower, victim, 0x45f52, HitSource::Melee);
        }
        if dramatic {
            self.bind_dramatic_target(thrower_ps, primary);
        }
        demon
    }

    fn record_mat_impact(&mut self, victim: u16) {
        use crate::impact::{ImpactDirection, ImpactKind};

        self.record_surface_impact(
            victim,
            ImpactKind::MatSlam,
            self.center_x(victim),
            crate::data_bank::GROUND as u16,
            ImpactDirection::Down,
        );
    }

    fn apply_standard_block_hurtbox(&mut self, e: u16, table: u32) {
        let boxes: Option<[(u32, u8); 8]> = match table {
            0x175bc => Some([
                // Lee: restore standing tight [32, 8, -8, 8] & loose [-4, 44, -16, 16] (#96, O5).
                (0x1a, 32),
                (0x1b, 8),
                (0x1c, 0xf8),
                (0x1d, 8),
                (0x1e, 0xfc),
                (0x1f, 44),
                (0x20, 0xf0),
                (0x21, 16),
            ]),
            0x17a9c => Some([
                // Birdie: restore standing tight [24, 8, 0, 8] & loose [-4, 60, -8, 16] (#96, O5).
                (0x1a, 24),
                (0x1b, 8),
                (0x1c, 0),
                (0x1d, 8),
                (0x1e, 0xfc),
                (0x1f, 60),
                (0x20, 0xf8),
                (0x21, 16),
            ]),
            0x17f7c => Some([
                // Adon: restore standing tight [24, 8, 0, 8] & loose [0, 48, -8, 16] instead
                // of 15-frame hurtbox disappearance ("Omniblock") (#96, O5).
                (0x1a, 24),
                (0x1b, 8),
                (0x1c, 0),
                (0x1d, 8),
                (0x1e, 0),
                (0x1f, 48),
                (0x20, 0xf8),
                (0x21, 16),
            ]),
            _ => None,
        };
        if let Some(entries) = boxes {
            for (off, v) in entries {
                self.set_eb(e, off, v);
            }
        }
    }

    fn apply_standard_reaction_hurtbox(&mut self, e: u16, a1: u32) {
        let boxes: Option<[(u32, u8); 8]> = match a1 {
            0x0f236 => Some([
                // Lee 16-frame block/reaction frame: restore full standing hurtboxes (#96, O5).
                (0x1a, 32),
                (0x1b, 8),
                (0x1c, 0xf8),
                (0x1d, 8),
                (0x1e, 0xfc),
                (0x1f, 44),
                (0x20, 0xf0),
                (0x21, 16),
            ]),
            0x1138c => Some([
                // Birdie 16-frame block/reaction frame: restore full standing hurtboxes (#96, O5).
                (0x1a, 24),
                (0x1b, 8),
                (0x1c, 0),
                (0x1d, 8),
                (0x1e, 0xfc),
                (0x1f, 60),
                (0x20, 0xf8),
                (0x21, 16),
            ]),
            0x1324c | 0x1326c => Some([
                // Adon block/reaction frames: restore full standing hurtboxes (#96, O5).
                (0x1a, 24),
                (0x1b, 8),
                (0x1c, 0),
                (0x1d, 8),
                (0x1e, 0),
                (0x1f, 48),
                (0x20, 0xf8),
                (0x21, 16),
            ]),
            _ => None,
        };
        if let Some(entries) = boxes {
            for (off, v) in entries {
                self.set_eb(e, off, v);
            }
        }
    }

    fn apply_shoto_frame_symmetry(&mut self, e: u16, a1: u32) {
        // 1. Synchronize Ken's stream frame durations (+0xe and +0x33) with Ryu's (#96, O5).
        let dur_override = match a1 {
            0x02f40 => Some(2u8), // Ken Standing MK step 0 (8 -> 2, matching Ryu 0x00e62)
            0x034a6 => Some(4),   // Ken Hadoken recovery step 4 (16 -> 4, matching Ryu 0x013c8)
            0x03620 => Some(9),   // Ken Tatsumaki landing step 14 (3 -> 9, matching Ryu 0x01542)
            0x03640 => Some(0),   // Ken Tatsumaki landing step 15 (2 -> 0, matching Ryu 0x01562)
            0x03a8e => Some(9), // Ken Neutral Jump HK landing step 7 (3 -> 9, matching Ryu 0x019b0)
            0x03aae => Some(0), // Ken Neutral Jump HK landing step 8 (2 -> 0, matching Ryu 0x019d0)
            0x03b30 => Some(3), // Ken Forward Jump Kick step 3 (2 -> 3, matching Ryu 0x01a52)
            _ => None,
        };
        if let Some(dur) = dur_override {
            self.set_eb(e, 0xe, dur);
            self.set_eb(e, 0x33, dur);
        }

        // 2. Vulnerable Shoryuken descent & landing recovery (#96, O5):
        //    Clear invulnerability bit 0x08 on Ryu's (0x012c0..=0x01340) and Ken's
        //    (0x0339e..=0x0341e) descending and landing recovery frames so whiffed
        //    or blocked uppercuts can be punished.
        if matches!(a1, 0x012c0..=0x01340 | 0x0339e..=0x0341e) {
            self.set_eb(e, 0xf, self.eb(e, 0xf) & !0x08);
        }

        // 3. Single-hit Shoryuken (#96, O5):
        //    Once Shoryuken (mv 0x19) has connected (hit or blocked) during its
        //    current execution, clear active hitbox bit 0x80 on subsequent rising frames.
        if self.eb(e, 0xa) == 0x19 {
            let slot = self.player_slot_index(self.el(e, 0x38)).unwrap_or(1);
            if self.toggle_state.dp_connected[slot] {
                self.set_eb(e, 0xf, self.eb(e, 0xf) & 0x7f);
            }
        }
    }

    // ---- Tier 1 damage, chip, and CPU AI scaling helpers (#97, O6) --------

    fn effective_entry_damage(&self, attacker: u16, entry: u32, is_chip: bool) -> u8 {
        let classic_special = !is_chip
            && matches!(entry, 0x4345e | 0x45f52 | 0x46012 | 0x436a8)
            && self.chosen_era(self.el(attacker, 0x38)).is_some_and(|era| {
                self.era_rules.classic_1987_special_damage || era == crate::era::EraFamily::Sf1987
            });
        if !self.toggles.balanced_roster || classic_special {
            return self.rb(entry + 1);
        }
        if is_chip || matches!(entry, 0x43466 | 0x4346e | 0x4601a | 0x46022 | 0x436b0) {
            // Normalize blocked special chip damage across all stages (including Thailand)
            // to 1/48 HP (~2.1% life) (#97, O6).
            return 1;
        }
        match entry {
            0x4345e => 9,  // Unblocked Hadoken: 9/48 HP (18.75% life, down from 20/48 = 41.7%)
            0x45f52 => 10, // Unblocked Shoryuken: 10/48 HP (20.8% life, down from 20/48 per frame)
            0x46012 => 9,  // Unblocked Tatsumaki: 9/48 HP (18.75% life, down from 20/48 per hit)
            0x436a8 => 10, // Unblocked Sagat Tiger Shot: 10/48 HP (20.8% life, down from 30/48 = 62.5%)
            // Ryu/Ken's light normals into the research's light band (#268, SP20).
            _ => crate::balanced_roster::shoto_entry_damage(entry).unwrap_or_else(|| self.rb(entry + 1)),
        }
    }

    /// Balanced roster's frame patches (#268, SP20): the boxes, duration and
    /// flags [`crate::balanced_roster::FRAME_PATCHES`] gives the frame just
    /// loaded from `a1`, before the facing flip.
    fn apply_balanced_frame(&mut self, e: u16, a1: u32) {
        let Some(p) = crate::balanced_roster::frame_patch(a1) else {
            return;
        };
        for (off, b) in [(0x1a, p.tight), (0x1e, p.loose), (0x22, p.attack)] {
            if let Some(b) = b {
                for (i, v) in b.into_iter().enumerate() {
                    self.set_eb(e, off + i as u32, v as u8);
                }
            }
        }
        if let Some(w) = p.body_front {
            self.set_eb(e, 0x18, w);
        }
        if let Some(d) = p.duration {
            self.set_eb(e, 0xe, d);
            self.set_eb(e, 0x33, d);
        }
        if p.flags_or != 0 {
            self.set_eb(e, 0xf, self.eb(e, 0xf) | p.flags_or);
        }
    }

    /// Balanced roster's move-header patches (#268, SP20): Joe's guard
    /// header (move 4 on his ROM or bank table) takes his standing hurtbox,
    /// as its frame `0x0e370` does, and Geki's forward-walk header (move 2)
    /// loses its 64 px critical box.
    fn apply_balanced_move_start(&mut self, e: u16, table: u32, mv: u8) {
        use crate::balanced_roster::{JOE_STANDING_LOOSE, JOE_STANDING_TIGHT};
        use crate::data_bank::table_address;
        use crate::roster::TournamentFighter;
        let joe = table == 0x1734c || table == table_address(TournamentFighter::Joe);
        let geki = table == 0x16bfc || table == table_address(TournamentFighter::Geki);
        let boxes: &[(u32, [i8; 4])] = match mv {
            4 if joe => &[(0x1a, JOE_STANDING_TIGHT), (0x1e, JOE_STANDING_LOOSE)],
            2 if geki => &[(0x1a, [16, 8, 8, 8])],
            _ => &[],
        };
        for &(off, b) in boxes {
            for (i, v) in b.into_iter().enumerate() {
                self.set_eb(e, off + i as u32, v as u8);
            }
        }
    }

    fn apply_thailand_ai_scaling(&mut self) {
        let d1 = (self.rb(0xffc007) as usize).min(3);
        let fight = self.rb(0xffc004);
        // 1. Scale range-shift roll (0xffc0c4..0xffc0d4, 16 bytes: 0x40 active, 0x20 inactive):
        let active_shift = if fight == 0 {
            [4usize, 8, 12, 16][d1]
        } else {
            [3usize, 6, 10, 14][d1]
        };
        for i in 0..16usize {
            let v = if i < active_shift { 0x40 } else { 0x20 };
            self.wb(0xffc0c4 + i as u32, v);
        }
        // 2. Scale input-reaction roll (0xffc0d4..0xffc0e4, 16 bytes: 0x20 react, 0x40 ignore):
        let active_react = [2usize, 5, 8, 12][d1];
        for i in 0..16usize {
            let v = if i < active_react { 0x20 } else { 0x40 };
            self.wb(0xffc0d4 + i as u32, v);
        }
        // 3. Scale stunned / post-hit idle move wait countdowns (0xffc0a4..0xffc0c4, odd bytes):
        let extra_stun_wait = [6u8, 3, 1, 0][d1];
        for i in 0..16u32 {
            let addr = 0xffc0a4 + 2 * i + 1;
            self.wb(addr, self.rb(addr).saturating_add(extra_stun_wait));
        }
        // 4. Scale airborne and ground reaction wait countdowns (0xffc0e4..0xffc1e4, 32 records):
        let extra_react_wait = [8u8, 4, 2, 0][d1];
        for i in 0..32u32 {
            let rec = 0xffc0e4 + 8 * i;
            if self.rb(rec + 4) != 0x81 {
                self.wb(rec + 6, self.rb(rec + 6).saturating_add(extra_react_wait));
            }
        }
        // 5. Scale special-move frequency in attack tables (0xffc1e4..0xffc664, 144 records):
        //    Easy replaces 3/4 of special attacks with normals; Normal replaces 2/4;
        //    Hard replaces 1/4; Very Hard keeps 100% of ROM special attacks.
        for idx in 0..144u32 {
            let rec = 0xffc1e4 + 8 * idx;
            let mv = self.rb(rec + 5);
            let slot = (idx & 3) as usize;
            let replace_with_normal = match d1 {
                0 => slot < 3,
                1 => slot < 2,
                2 => slot < 1,
                _ => false,
            };
            if !replace_with_normal {
                continue;
            }
            if fight != 0 && matches!(mv, 0x19 | 0x23 | 0x24) {
                // Sagat: replace Tiger Shot (0x24), Tiger Knee (0x23), or combo (0x19)
                // with normal Standing LP (0x13ec0, mv 0x0d) or Standing HK (0x14030, mv 0x15).
                let (stream, norm_mv) = if idx & 1 == 0 {
                    (0x13ec0u32, 0x0du8)
                } else {
                    (0x14030u32, 0x15u8)
                };
                self.wl(rec, stream);
                self.wb(rec + 4, 0xc2);
                self.wb(rec + 5, norm_mv);
                self.wb(rec + 6, 0);
                self.wb(rec + 7, 0);
            } else if fight == 0 && matches!(mv, 0x20 | 0x21) {
                // Adon: replace Jaguar Kick (0x21) or leaping attack (0x20)
                // with normal Standing Punch (0x132dc, mv 0x0d) or Kick (0x1336c, mv 0x0f).
                let (stream, norm_mv) = if idx & 1 == 0 {
                    (0x132dcu32, 0x0du8)
                } else {
                    (0x1336cu32, 0x0fu8)
                };
                self.wl(rec, stream);
                self.wb(rec + 4, 0xc2);
                self.wb(rec + 5, norm_mv);
                self.wb(rec + 6, 0);
                self.wb(rec + 7, 0);
            }
        }
    }
}

/// The object entries a pose grid writes, as `[code, attr, y, x]` in the
/// order the original writes them (0x443c6 / 0x4445e): `rows + 1` rows of
/// `cols + 1` entries from `(x, y)`. Big entries (attr bit 0x400) step 32
/// pixels with codes +2 per column and +0x20 per row; small ones step 16
/// with +1 and +0x10. Flipped (facing left), attr gains 0x100 and each
/// row's codes run in reverse column order. The engine's frame loader and
/// the art pipeline's drawing checks both use it.
pub fn pose_grid_entries(
    code: u16,
    attr: u16,
    rows: u8,
    cols: u8,
    x: u16,
    y: u16,
    flipped: bool,
) -> Vec<[u16; 4]> {
    let (rows, cols) = (rows as u16, cols as u16);
    let attr = if flipped { attr | 0x100 } else { attr };
    let big = (attr as i16) >= 0x400;
    let (col_code, row_code, step, span, rev) = if big {
        (2u16, 0x20u16, 0x20u16, cols << 1, 4u16)
    } else {
        (1, 0x10, 0x10, cols, 2)
    };
    let mut out = Vec::with_capacity(((rows + 1) * (cols + 1)) as usize);
    let (mut rc, mut ry) = (code, y);
    for _ in 0..=rows {
        let (mut c, mut cx, mut d7) = (rc, x, 0u16);
        for _ in 0..=cols {
            let drawn = if flipped {
                c.wrapping_add(span).wrapping_sub(d7)
            } else {
                c
            };
            out.push([drawn, attr, ry, cx]);
            c = c.wrapping_add(col_code);
            cx = cx.wrapping_add(step);
            d7 = d7.wrapping_add(rev);
        }
        rc = rc.wrapping_add(row_code);
        ry = ry.wrapping_add(step);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Ram {
        rom: Vec<u8>,
        work: Vec<u8>,
        obj: Vec<u8>,
        video: Vec<u8>,
        fg: u16,
        bg: u16,
        gfx: u8,
        flip: bool,
        coin_ctrl: u16,
        pal: Vec<u8>,
        unrec: BTreeSet<u32>,
        calls: BTreeSet<u32>,
        toggles: crate::config::CoreToggles,
        toggle_state: crate::config::CoreToggleState,
        training: crate::training::TrainingState,
        bg_page: u8,
        fg_page: u8,
        bank: Option<crate::data_bank::DataBank>,
    }

    impl Ram {
        fn new() -> Self {
            Ram {
                rom: vec![0; 0x60000],
                work: vec![0; 0x6000],
                obj: vec![0; 0x2000],
                video: vec![0; 0x1000],
                fg: 0,
                bg: 0,
                gfx: 0,
                flip: false,
                coin_ctrl: 0x7070,
                pal: vec![0; 0x800],
                unrec: BTreeSet::new(),
                calls: BTreeSet::new(),
                toggles: crate::config::CoreToggles::faithful(),
                toggle_state: crate::config::CoreToggleState::default(),
                training: crate::training::TrainingState::default(),
                bg_page: 0,
                fg_page: 0,
                bank: None,
            }
        }
        fn bus(&mut self) -> Bus<'_> {
            Bus {
                rom: &self.rom,
                work: &mut self.work,
                obj: &mut self.obj,
                video: &mut self.video,
                fg_scroll: &mut self.fg,
                bg_scroll: &mut self.bg,
                gfxctrl: &mut self.gfx,
                flip_screen: &mut self.flip,
                coin_ctrl: &mut self.coin_ctrl,
                palette: &mut self.pal,
                dsw1: crate::tasks::Io::idle().dsw1,
                unrecovered: &mut self.unrec,
                effect_stream_calls: &mut self.calls,
                toggles: self.toggles,
                preset: crate::config::RulesetPreset::Faithful1987,
                roster: crate::roster::RosterSettings::faithful(),
                dramatic: None,
                world_tour: None,
                evil_ryu_playtest: None,
                era_rules: crate::config::MatchConfig::faithful().into(),
                toggle_state: &mut self.toggle_state,
                training: &mut self.training,
                backgrounds: None,
                data_bank: self.bank.as_ref(),
                bg_page: &mut self.bg_page,
                fg_page: &mut self.fg_page,
            }
        }
    }

    #[test]
    fn dramatic_shotos_use_their_own_tables_with_full_cast_disabled() {
        use crate::dramatic::{DramaticRules, LifebarMode};
        use crate::roster::TournamentFighter::{Ken, Retsu, Ryu};

        let mut ram = Ram::new();
        let mut bus = ram.bus();
        bus.preset = crate::config::RulesetPreset::DefinitiveOverhaul;
        bus.roster.full_cast = false;
        bus.wb(GAME_MODE, 2);
        bus.wb(MODE_DEMO, 1);
        bus.wl(CPU_STRUCT + 0x12, Ken.move_table());
        bus.wl(P2_STRUCT + 0x12, Ryu.move_table());
        assert!(!bus.kens_side(CPU_STRUCT));
        assert!(bus.kens_side(P2_STRUCT));

        bus.dramatic = Some(DramaticRules {
            co_op: true,
            ally2: Ryu,
            stage: Retsu,
            lifebar: LifebarMode::Shared,
            friendly_fire: false,
            styles: [crate::era::Revision::Sf1Plus; 3],
            rivalry: None,
        });
        assert!(bus.kens_side(CPU_STRUCT));
        assert!(!bus.kens_side(P2_STRUCT));
    }

    /// A Kiai Shot blast in effect slot 0x100, owned by P1 (entity 0x400,
    /// holding his palm: +0x33 and +0xe 9) against P2 (entity 0xa00), its
    /// handlers bank records against an attack box (+0x18) and a projectile
    /// (+0x1c) whose impact template (code 0x3a00, sound 6) breaks it up.
    /// Returns the Ram, with P2's attack box over the blast's centre.
    fn blast_setup(game_mode: u8, stage: u8) -> Ram {
        use crate::data_bank::{
            DATA_BANK_BASE as B, ROUTINE_EFFECT_VS_ATTACK, ROUTINE_EFFECT_VS_PROJECTILE,
        };
        let mut bytes = vec![0u8; 0x80];
        bytes[0] = ROUTINE_EFFECT_VS_ATTACK;
        bytes[4..8].copy_from_slice(&(B + 0x20).to_be_bytes());
        bytes[8] = ROUTINE_EFFECT_VS_PROJECTILE;
        bytes[0xc..0x10].copy_from_slice(&(B + 0x20).to_be_bytes());
        // The impact: code 0x3a00, a 32-pixel entry in palette 3, first
        // frame (+8 = 1), hold 2, no collisions, sound 6, a stream that ends.
        let t = 0x20;
        bytes[t..t + 2].copy_from_slice(&0x3a00u16.to_be_bytes());
        bytes[t + 2..t + 4].copy_from_slice(&0x0403u16.to_be_bytes());
        bytes[t + 8] = 1;
        bytes[t + 9] = 2;
        bytes[t + 0xf] = 6;
        bytes[t + 0x10..t + 0x14].copy_from_slice(&(B + 0x60).to_be_bytes());
        bytes[0x60] = 0xfd;
        let mut ram = Ram::new();
        ram.bank = Some(crate::data_bank::DataBank::from_bytes(bytes));
        ram.rom[0x434d0..0x434d6].copy_from_slice(&[0, 0, 1, 1, 1, 1]);
        let mut b = ram.bus();
        b.wb(GAME_MODE, game_mode);
        b.wb(0xffc003, stage);
        b.ww(P1_STRUCT + 0x10, 0x400);
        b.ww(P2_STRUCT + 0x10, 0xa00);
        let (me, other) = (0x400u16, 0xa00u16);
        b.set_eb(me, 0x33, 9);
        b.set_eb(me, 0xe, 9);
        // The blast: centre (216, 116), a 32-pixel entry, box 12.
        let s = 0x100u16;
        b.set_eb(s, 8, 2);
        b.set_ew(s, 0, 0x3800);
        b.set_ew(s, 2, 0x0403);
        b.set_ew(s, 4, 100);
        b.set_ew(s, 6, 200);
        b.set_eb(s, 0xc, 12);
        b.set_eb(s, 0xd, 12);
        b.set_eb(s, 0xe, 0xe0);
        b.set_el(s, 0x18, B);
        b.set_el(s, 0x1c, B + 8);
        b.set_el(s, 0x20, P1_STRUCT);
        b.set_el(s, 0x24, P2_STRUCT);
        // P2: a small one-cell grid centred on (216, 116), its attack box
        // there too.
        b.set_ew(other, 2, 0);
        b.set_ew(other, 4, 108);
        b.set_ew(other, 6, 208);
        b.set_eb(other, 0x23, 8);
        b.set_eb(other, 0x25, 8);
        drop(b);
        ram
    }

    #[test]
    fn a_blast_meeting_an_attack_box_breaks_up_in_place_and_leaves_its_throwers_hold() {
        // 1P on a stage whose 0x434d0 byte is 1: it breaks up, where it was,
        // into its own impact with the explosion's sound; Retsu's palm hold
        // is untouched (the Hadoken's 0x43490 would zero it).
        let mut ram = blast_setup(2, 2);
        let mut b = ram.bus();
        b.effect_vs_attack(0x100);
        assert_eq!(b.ew(0x100, 0), 0x3a00, "its own impact");
        assert_eq!(
            (b.ew(0x100, 6), b.ew(0x100, 4)),
            (200, 100),
            "where it was (its offsets are 0)"
        );
        assert_eq!(b.eb(0x100, 0xf), 6, "the explosion's sound");
        assert_eq!(
            (b.eb(0x400, 0x33), b.eb(0x400, 0xe)),
            (9, 9),
            "his hold is his"
        );
        // In 2P versus, and on a stage whose byte is 0, nothing happens, as
        // in 1987.
        for (mode, stage) in [(1, 2), (2, 0)] {
            let mut ram = blast_setup(mode, stage);
            let mut b = ram.bus();
            b.effect_vs_attack(0x100);
            assert_eq!(
                b.ew(0x100, 0),
                0x3800,
                "mode {mode} stage {stage}: the blast goes on"
            );
        }
    }

    #[test]
    fn a_blast_meeting_a_projectile_breaks_up_and_ends_the_other() {
        let mut ram = blast_setup(1, 0);
        let mut b = ram.bus();
        let o = 0x140u16;
        b.set_eb(o, 8, 2);
        b.set_ew(o, 2, 0x0404);
        b.set_ew(o, 4, 100);
        b.set_ew(o, 6, 200);
        b.set_eb(o, 0xc, 12);
        b.set_eb(o, 0xd, 12);
        b.set_eb(o, 0xe, 0x20);
        b.effect_vs_projectile(0x100);
        assert_eq!(
            b.ew(0x100, 0),
            0x3a00,
            "its own impact, not the Hadoken's explosion"
        );
        assert_eq!((b.ew(0x100, 6), b.ew(0x100, 4)), (200, 100), "where it was");
        assert_eq!(b.eb(o, 8), 0, "the other projectile ends");
        assert_eq!(
            (b.eb(0x400, 0x33), b.eb(0x400, 0xe)),
            (9, 9),
            "his hold is his"
        );
    }

    #[test]
    fn the_last_effect_slot_keeps_the_original_probe_outside_era_state() {
        let mut ram = Ram::new();
        let mut bus = ram.bus();
        bus.set_eb(0x3c0, 8, 2);
        bus.set_eb(0x3c0, 0xe, 0x20);
        // The translated scan begins one slot after the projectile, so the
        // final slot can examine the first fighter's grid header.
        bus.set_eb(0x400, 8, 2);
        bus.set_eb(0x400, 0xe, 0x20);
        bus.effect_vs_projectile(0x3c0);
        assert_eq!(bus.eb(0x400, 8), 2);
    }

    /// A bank table's bytes with the special slots `slots` present and
    /// their records.
    fn bank_table(slots: &[(u8, crate::data_bank::SpecialSlot)]) -> Vec<u8> {
        crate::data_bank::test_special_table(slots)
    }

    /// A bank table at the bank's base with all three special slots, which
    /// put 0x19 on the Shoryuken's command with a kick, 0x1a on the
    /// Hadoken's with a kick and 0x1b on the Tatsumaki's with a punch; P1
    /// bound to it, his entity 0x400 facing right.
    fn special_commands_setup() -> Ram {
        use crate::data_bank::{SpecialSlot, DATA_BANK_BASE};
        let slot = |tracker, kick| SpecialSlot {
            tracker,
            kick,
            melee: true,
            low_in_air: false,
            score: 0,
        };
        let bytes = bank_table(&[
            (0x19, slot(0x19, true)),
            (0x1a, slot(0x1c, true)),
            (0x1b, slot(0x16, false)),
        ]);
        let mut ram = Ram::new();
        ram.bank = Some(crate::data_bank::DataBank::from_bytes(bytes));
        let mut b = ram.bus();
        b.wl(P1_STRUCT + 0x12, DATA_BANK_BASE);
        b.ww(P1_STRUCT + 0x10, 0x400);
        b.set_eb(0x400, 0x37, 2);
        ram
    }

    #[test]
    fn a_bank_tables_special_commands_put_each_slot_on_any_tracker_and_button() {
        let mut ram = special_commands_setup();
        let mut b = ram.bus();
        let all = |b: &Bus| -> Vec<Option<u8>> {
            [
                (0x16, false),
                (0x16, true),
                (0x19, false),
                (0x19, true),
                (0x1c, false),
                (0x1c, true),
            ]
            .into_iter()
            .map(|(off, kick)| b.tracker_special(P1_STRUCT, off, kick))
            .collect()
        };
        assert_eq!(
            all(&b),
            [Some(0x1b), None, None, Some(0x19), None, Some(0x1a)]
        );
        // A slot his table leaves out (its first-frame byte 0) is no special.
        b.data_bank = None;
        let teleport = crate::data_bank::SpecialSlot {
            tracker: 0x16,
            kick: false,
            melee: true,
            low_in_air: false,
            score: 0,
        };
        let bank = crate::data_bank::DataBank::from_bytes(bank_table(&[(0x1b, teleport)]));
        b.data_bank = Some(&bank);
        assert_eq!(all(&b), [Some(0x1b), None, None, None, None, None]);
        // Ryu's ROM table: the 1987 three, each on its own command.
        b.wl(P1_STRUCT + 0x12, 0x1605c);
        assert_eq!(
            all(&b),
            [None, Some(0x1b), Some(0x19), None, Some(0x1a), None]
        );
    }

    #[test]
    fn a_special_fires_from_its_commands_last_direction_and_button() {
        let mut ram = special_commands_setup();
        let mut b = ram.bus();
        // The Tatsumaki's tracker done: on this table a punch fires 0x1b
        // held back (left, facing right), a kick does not.
        b.wb(P1_STRUCT + 0x16 + 2, 0xff);
        assert!(b.punch_special_ready_for_dir(P1_STRUCT, 0x400, 2));
        assert!(
            !b.punch_special_ready_for_dir(P1_STRUCT, 0x400, 1),
            "not held forward"
        );
        assert!(!b.kick_special_ready_for_dir(P1_STRUCT, 0x400, 2));
        b.wb(P1_STRUCT + 0x16 + 2, 0);
        // The Hadoken's: a kick held forward; the Shoryuken's: a kick held
        // down-forward.
        b.wb(P1_STRUCT + 0x1c + 2, 0xff);
        assert!(b.kick_special_ready_for_dir(P1_STRUCT, 0x400, 1));
        assert!(!b.punch_special_ready_for_dir(P1_STRUCT, 0x400, 1));
        b.wb(P1_STRUCT + 0x1c + 2, 0);
        b.wb(P1_STRUCT + 0x19 + 2, 0xff);
        assert!(b.kick_special_ready_for_dir(P1_STRUCT, 0x400, 5));
        assert!(!b.punch_special_ready_for_dir(P1_STRUCT, 0x400, 5));
        // Facing left, down-forward is down-left.
        b.set_eb(0x400, 0x37, 4);
        assert!(b.kick_special_ready_for_dir(P1_STRUCT, 0x400, 6));
        assert!(!b.kick_special_ready_for_dir(P1_STRUCT, 0x400, 5));
        // Under lenient commands any direction but up, with its button,
        // and the strength latched: the special that tracker fires.
        b.toggles.lenient_command_trackers = true;
        assert!(b.kick_special_ready_for_dir(P1_STRUCT, 0x400, 0));
        b.set_eb(0x400, 0x34, 2);
        assert_eq!(b.lenient_kick_special(0x400, P1_STRUCT), Some(0x19));
        assert_eq!(b.lenient_punch_special(0x400, P1_STRUCT), None);
    }

    #[test]
    fn an_airborne_specials_crouch_guard_and_its_score_come_from_its_slot_not_its_id() {
        use crate::data_bank::{SpecialSlot, DATA_BANK_BASE};
        // P2 (entity 0xa00) in the air (+9 0xc1) in a special, P1 guarding
        // under standard high and low blocking.
        let knee = SpecialSlot {
            tracker: 0x19,
            kick: true,
            melee: true,
            low_in_air: false,
            score: 0x46012,
        };
        let shot = SpecialSlot {
            tracker: 0x1c,
            kick: false,
            melee: false,
            low_in_air: true,
            score: 0,
        };
        let mut ram = Ram::new();
        ram.toggles.standard_high_low_block = true;
        ram.bank = Some(crate::data_bank::DataBank::from_bytes(bank_table(&[
            (0x19, knee),
            (0x1b, shot),
        ])));
        let mut b = ram.bus();
        b.ww(P2_STRUCT + 0x10, 0xa00);
        b.set_el(0xa00, 0x38, P2_STRUCT);
        b.set_eb(0xa00, 9, 0xc1);
        let low = |b: &mut Bus, table: u32, mv: u8| {
            b.wl(P2_STRUCT + 0x12, table);
            b.set_eb(0xa00, 0xa, mv);
            b.opponent_attacks_low(0xa00)
        };
        // A boss's: as his table's slot says, in whichever slot it sits.
        assert!(
            !low(&mut b, DATA_BANK_BASE, 0x19),
            "his knee in 0x19 is an overhead in the air"
        );
        assert!(
            low(&mut b, DATA_BANK_BASE, 0x1b),
            "his low shot in 0x1b is guarded crouching"
        );
        // Ryu's ROM table: the Shoryuken and the Tatsumaki, as 1987 (#96).
        assert!(
            low(&mut b, 0x1605c, 0x19) && low(&mut b, 0x1605c, 0x1b) && !low(&mut b, 0x1605c, 0x1a)
        );
        // The slot's score entry, on a bank table only.
        b.wl(P2_STRUCT + 0x12, DATA_BANK_BASE);
        b.set_eb(0xa00, 0xa, 0x19);
        assert_eq!(b.player_score_entry(0xa00, 1), Some(0x46012));
        b.set_eb(0xa00, 0xa, 0x1b);
        assert_eq!(
            b.player_score_entry(0xa00, 1),
            None,
            "a projectile scores by its own hit"
        );
        b.wl(P2_STRUCT + 0x12, 0x1605c);
        b.set_eb(0xa00, 0xa, 0x19);
        assert_eq!(
            b.player_score_entry(0xa00, 1),
            None,
            "Ryu's specials score by their own hit functions"
        );
    }

    #[test]
    fn button_latch_writes_the_strength_on_release_only() {
        let mut ram = Ram::new();
        ram.rom[0x3b450..0x3b453].copy_from_slice(&[0x40, 0xe0, 0xfe]);
        let mut bus = ram.bus();
        let (state, out) = (0xff8690, 0xff86b0);
        bus.button_latch(state, out, 1, 0x3b450); // press (light)
        assert_eq!(bus.rb(out), 0);
        for _ in 0..11 {
            bus.button_latch(state, out, 1, 0x3b450); // held
        }
        assert_eq!(bus.rb(out), 0, "nothing while held");
        bus.button_latch(state, out, 0, 0x3b450); // release
        assert_eq!(bus.rb(out), 0x40, "light strength on the release frame");
        assert_eq!(bus.rb(state), 0, "group state cleared");
    }

    #[test]
    fn button_latch_keeps_the_strongest_and_drops_saturated_holds() {
        let mut ram = Ram::new();
        ram.rom[0x3b450..0x3b453].copy_from_slice(&[0x40, 0xe0, 0xfe]);
        let mut bus = ram.bus();
        let (state, out) = (0xff8690, 0xff86b0);
        bus.button_latch(state, out, 1, 0x3b450);
        bus.button_latch(state, out, 3, 0x3b450);
        bus.button_latch(state, out, 1, 0x3b450);
        bus.button_latch(state, out, 0, 0x3b450);
        assert_eq!(bus.rb(out), 0xfe, "heavy was the strongest");
        bus.wb(out, 0);
        bus.button_latch(state, out, 1, 0x3b450);
        for _ in 0..300 {
            bus.button_latch(state, out, 1, 0x3b450);
        }
        bus.button_latch(state, out, 0, 0x3b450);
        assert_eq!(bus.rb(out), 0, "a hold past the 0xff count never fires");
    }

    #[test]
    fn strength_table_picks_light_medium_heavy_moves() {
        let mut ram = Ram::new();
        ram.rom[0x405b6..0x405be]
            .copy_from_slice(&[0x10, 0x0d, 0xd0, 0x0f, 0xf0, 0x10, 0xff, 0x10]);
        let mut bus = ram.bus();
        for (strength, mv, level) in [(0x40u8, 0x0d, 1), (0xe0, 0x0f, 2), (0xfe, 0x10, 3)] {
            bus.wb(0xff86b0, strength);
            assert_eq!(bus.pick_by_strength(0xff86b0, 0x405b6, RYU), Some(mv));
            assert_eq!(bus.eb(RYU, 0x34), level);
            assert_eq!(bus.rb(0xff86b0), 0, "latch consumed");
        }
        bus.wb(0xff86b0, 0);
        assert_eq!(bus.pick_by_strength(0xff86b0, 0x405b6, RYU), None);
    }

    #[test]
    fn sound_queue_pops_one_command_per_vblank() {
        let mut ram = Ram::new();
        for i in 0..16 {
            ram.work[0x662 + 2 * i] = 0xff;
            ram.work[0x663 + 2 * i] = 0xff;
        }
        ram.work[0x3] = 0xff;
        let mut bus = ram.bus();
        assert_eq!(bus.pop_sound(), 0xff, "empty queue idles the latch");
        bus.queue_sound(0x84);
        bus.queue_sound(0x01);
        assert_eq!(bus.pop_sound(), 0x84);
        assert_eq!(bus.pop_sound(), 0x01);
        assert_eq!(bus.pop_sound(), 0xff);
    }

    #[test]
    fn random_step_folds_the_idle_p2_byte_with_fixed_increments() {
        // Fixture state at 2450 and its first update (observed at 2452).
        let mut ram = Ram::new();
        ram.work[0xe4a..0xe52].copy_from_slice(&[0xc1, 0xb7, 0x77, 0xd4, 0xb3, 0x73, 0x29, 0xbd]);
        ram.work[0x6dc..0x6e0].copy_from_slice(&[0x00, 0xff, 0x86, 0x8e]);
        ram.work[0x68e] = 0xff; // P2 idle (active low)
        let mut bus = ram.bus();
        bus.step_random();
        let mut want = [0u8; 8];
        let mut d0 = 0u8;
        for (i, (s, a)) in [0xc1u8, 0xb7, 0x77, 0xd4, 0xb3, 0x73, 0x29, 0xbd]
            .iter()
            .zip([3u8, 3, 5, 7, 9, 11, 13, 15])
            .enumerate()
        {
            d0 = d0.wrapping_add(*s).wrapping_add(a);
            want[i] = d0;
        }
        let got: Vec<u8> = (0..8).map(|i| bus.rb(0xff8e4a + i)).collect();
        assert_eq!(got, want);
        assert_eq!(
            bus.rb(0xff8e4c),
            0xfa,
            "byte 2 matches the reference write at 2452"
        );
    }
}
