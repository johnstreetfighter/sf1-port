//! Scenario comparison runner library.
//!
//! The high-level scenario boundary: a scenario (reference identity, machine
//! configuration, frame-indexed logical inputs, bounded duration, expected
//! observations) plus observation captures produced by scenario adapters.
//! This crate compares two captures and reports the first observable
//! mismatch. It knows nothing about translated game functions.

pub mod capture;
pub mod compare;
pub mod report;
pub mod roms;
pub mod scenario;
