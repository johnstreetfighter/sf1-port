//! One-player compatibility API. The implementation is shared with versus
//! play by [`crate::fight::FightMachine`] (ticket #66).

/// First frame of the original CPU-decision interval.
pub const INTERVAL_FIRST: u32 = 2389;
/// Last frame: Retsu's landing stance, before his second decision.
pub const INTERVAL_LAST: u32 = 2433;

/// The one-player name retained for existing callers; this is not a separate
/// implementation.
pub type CpuMachine = crate::fight::FightMachine;
