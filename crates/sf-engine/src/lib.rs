//! sf-engine: native machine-state translation for bounded SF1 scenes.
//!
//! This crate holds the Rust state update logic that replaces the original
//! 68000 program for one bounded, documented interval at a time:
//!
//! - `versus` (tickets #6-#9, #15): the 2P-versus fixture from the
//!   checkpoint-2450 state (Ryu and Ken standing): walks with the
//!   opponent-proximity stop, camera follow and stage-left edge
//!   (sf-move-01), crouch/stand (sf-crouch-01), the neutral jump
//!   (sf-jump-01), diagonal jumps (sf-jump-02), and the standing light punch
//!   against a controlled Ken (sf-punch-01), all through the translated
//!   input handler and move streams.
//! - `cpu` (tickets #12, #25-#29): one-player play, 1P Ryu vs CPU Retsu,
//!   from a checkpoint: the round-1 intro (sf-idle-scene-01), the first
//!   decision (sf-cpu-01), round 2 to the KO, the round results, and the
//!   transition into round 2.
//! - `tasks` (tickets #14, #24, #28, #29): the task scheduler and the
//!   tasks it runs: game flow, round timer, screen events, the challenger
//!   watch, the random state.
//! - `fighter` (tickets #9, #12): traceable translations of original routines —
//!   button latches, the input handler with its command trackers and
//!   posture dispatch (ticket #15), move start and
//!   animation streams, separation and screen clamps, the collision judge,
//!   damage/score/reaction, effect objects, the camera requests, the sound
//!   queue, the random state, the CPU decision, and the stage drift — on an
//!   original-address memory view.
//!
//! Everything here is state-driven and nothing captured is replayed: the
//! scenes read move headers, frame streams, text blocks and tables from the
//! local program ROM and interpret them with translated code. No original
//! CPU code runs.
//!
//! See docs/reference/idle-scene.md, movement.md, crouch.md, jump.md,
//! posture-dispatch.md, punch.md, and cpu-decision.md for the interval evidence, the disclosed fixture controls, and
//! the explicitly uncovered behavior.

pub mod alpha;
pub mod balanced_roster;
pub mod box_audit;
pub mod box_audit_report;
pub mod arcade;
pub mod dramatic;
pub mod era;
pub mod era_fight;
pub mod era_preview;
pub mod capture;
mod canonical;
pub mod combo;
pub mod config;
pub mod cpu;
pub mod data_bank;
pub mod fair_cpu;
pub mod fight;
pub mod fight_reader;
pub mod front;
pub mod jukebox;
pub mod fighter;
pub mod machine;
pub mod mirror;
pub mod moves;
pub mod mugen_extract;
pub mod overlay;
pub mod replay;
pub mod roster;
pub mod run;
pub mod session_io;
mod control;
pub mod utility;
pub mod world_tour;
pub mod netplay;
pub mod bonus;
pub mod bonus_arcade;
pub mod sf1plus;
pub mod tasks;
pub mod theme;
pub mod training;
pub mod versus;
pub mod version;

pub use machine::MachineState;
