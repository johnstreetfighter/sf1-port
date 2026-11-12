//! Native renderer for the original game's video hardware.
//!
//! Ticket #4 scope: reconstruct the fixed fight's initial frame from
//! decoded original graphics data plus documented scene state, matching
//! the reference capture byte-exactly before display scaling.

pub mod announcer;
pub mod art;
pub mod art_pack;
pub mod art_review;
pub mod asset_pack;
pub mod background_bank;
pub mod busts;
pub mod callouts;
pub mod compositor;
pub mod decode;
pub mod ending_screens;
pub mod mirror_palette;
pub mod era_hud;
pub mod palette;
pub mod render;
pub mod rom_stages;
pub mod romset;
pub mod scene;
pub mod sprite_bank;
pub mod stage;
pub mod stage_preview;
pub mod vs_screens;
