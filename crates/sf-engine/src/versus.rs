//! Versus compatibility API. Both players and all fight modes use
//! [`crate::fight::FightMachine`] (ticket #66).

/// First frame of the checkpoint-2450 versus fixture.
pub const INTERVAL_FIRST: u32 = 2450;
pub const INTERVAL_LAST: u32 = 3350;
pub const CROUCH_INTERVAL_LAST: u32 = 3399;
pub const JUMP_INTERVAL_LAST: u32 = 2999;
pub const PUNCH_INTERVAL_LAST: u32 = 3399;

/// The versus name retained for existing callers; this is not a separate
/// implementation.
pub type VersusMachine = crate::fight::FightMachine;
