//! The generation ledger (spec #164, AP5): every PixelLab call, kept in a
//! committed file per fighter, `art/generations/<fighter>.json`. A call
//! records its move, references, prompt, canvas, seed and cost, and which
//! of its candidates became which pose, with the packed file's CRC-32 and
//! any touch-ups by hand. A retry keeps the earlier kept candidates in
//! the record, with a note on their fate. A pose made from another
//! fighter's kept one (Ken's from Ryu's) costs nothing and is recorded as
//! derived, with its source and touch-ups. The pixels stay in the art pack.
//!
//! The plan is 2,000 generations a month, resetting on the 27th. The
//! project first kept to 90% of each month's (design, the builder's call
//! 4); the maintainer lifted that to the whole allowance on 2026-11-07, so the
//! last bosses' sets fit before the reset.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;
use sf_video::art::Grid;

/// The plan's generations a month.
pub const ALLOWANCE: u32 = 2_000;
/// What the project may spend of it: all of it (the maintainer, 2026-11-07; 90%
/// before).
pub const CAP: u32 = ALLOWANCE;
/// The day of the month the allowance resets.
pub const RESET_DAY: u32 = 27;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ledger {
    pub fighter: String,
    pub calls: Vec<Call>,
    /// Poses made from kept ones, without a call.
    #[serde(default)]
    pub derived: Vec<Derived>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Call {
    /// When it was made, UTC (`2026-11-03T00:33:30Z`).
    pub at: String,
    /// The move it was for.
    #[serde(rename = "move")]
    pub move_: String,
    pub tool: String,
    pub job: String,
    /// Generations spent.
    pub cost: u32,
    pub canvas: [u32; 2],
    pub seed: u32,
    /// The colours it was held to.
    pub palette: Vec<String>,
    pub references: Vec<Reference>,
    pub prompt: String,
    /// Candidates it returned.
    pub candidates: u32,
    /// Candidates that became poses.
    #[serde(default)]
    pub kept: Vec<Kept>,
    #[serde(default)]
    pub note: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    /// One of the fighter's (or a guide fighter's) ROM poses.
    #[serde(default)]
    pub rom: Option<Grid>,
    #[serde(default)]
    pub fighter: Option<String>,
    /// Or an earlier candidate, `<job>#<index>`.
    #[serde(default)]
    pub candidate: Option<String>,
    pub usage: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Kept {
    /// Its index among the call's candidates, from 0.
    pub candidate: u32,
    pub pose: String,
    /// The packed file's CRC-32, which the recipe names.
    pub crc32: String,
    /// What was touched up by hand before packing.
    #[serde(default)]
    pub touch_ups: Vec<String>,
    /// What became of it, when a retry replaced it.
    #[serde(default)]
    pub note: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Derived {
    pub pose: String,
    /// The packed file's CRC-32, which the recipe names.
    pub crc32: String,
    /// The kept pose it was made from.
    pub from: KeptPose,
    pub touch_ups: Vec<String>,
    #[serde(default)]
    pub note: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeptPose {
    pub fighter: String,
    pub pose: String,
    pub crc32: String,
}

impl Ledger {
    pub fn load(path: &Path) -> Result<Ledger, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Generations spent on each move.
    pub fn by_move(&self) -> BTreeMap<String, u32> {
        let mut out = BTreeMap::new();
        for c in &self.calls {
            *out.entry(c.move_.clone()).or_default() += c.cost;
        }
        out
    }
}

/// The month of the plan a call falls in, named by the day it began
/// (`2026-10-27`).
pub fn cycle(at: &str) -> Result<String, String> {
    let bad = || format!("{at}: not a UTC time like 2026-11-03T00:33:30Z");
    let num = |r: std::ops::Range<usize>| at.get(r).and_then(|s| s.parse::<u32>().ok()).ok_or_else(bad);
    let (mut year, mut month, day) = (num(0..4)?, num(5..7)?, num(8..10)?);
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Err(bad());
    }
    if day < RESET_DAY {
        if month == 1 {
            year -= 1;
            month = 12;
        } else {
            month -= 1;
        }
    }
    Ok(format!("{year:04}-{month:02}-{RESET_DAY}"))
}

/// Generations spent in each month of the plan, over every ledger.
pub fn by_cycle(ledgers: &[Ledger]) -> Result<BTreeMap<String, u32>, String> {
    let mut out = BTreeMap::new();
    for c in ledgers.iter().flat_map(|l| &l.calls) {
        *out.entry(cycle(&c.at)?).or_default() += c.cost;
    }
    Ok(out)
}

/// Whether a call in `ledgers` kept `fighter`'s `pose` as the file `crc32`.
pub fn kept(ledgers: &[Ledger], fighter: &str, pose: &str, crc32: &str) -> bool {
    ledgers
        .iter()
        .filter(|l| l.fighter == fighter)
        .flat_map(|l| &l.calls)
        .flat_map(|c| &c.kept)
        .any(|k| k.pose == pose && k.crc32 == crc32)
}

/// Every call is fully recorded, every derived pose comes from a kept one,
/// and no month passes the cap.
pub fn check(ledgers: &[Ledger]) -> Result<(), String> {
    for l in ledgers {
        for c in &l.calls {
            let at = format!("{} {} ({})", l.fighter, c.move_, c.job);
            if c.cost == 0 || c.prompt.is_empty() || c.palette.is_empty() || c.references.is_empty() {
                return Err(format!("{at}: its cost, prompt, palette and references must all be recorded"));
            }
            if let Some(r) = c.references.iter().find(|r| r.rom.is_some() == r.candidate.is_some()) {
                return Err(format!("{at}: a reference ({}) must be a ROM pose or a candidate", r.usage));
            }
            if let Some(k) = c.kept.iter().find(|k| k.candidate >= c.candidates) {
                return Err(format!("{at}: candidate {} kept, but it returned {}", k.candidate, c.candidates));
            }
        }
        for d in &l.derived {
            let s = &d.from;
            if !kept(ledgers, &s.fighter, &s.pose, &s.crc32) {
                return Err(format!("{} {}: made from {} {} {}, which no call kept", l.fighter, d.pose, s.fighter, s.pose, s.crc32));
            }
            if d.touch_ups.is_empty() {
                return Err(format!("{} {}: what was changed from {} {} must be recorded", l.fighter, d.pose, s.fighter, s.pose));
            }
        }
    }
    for (month, spent) in by_cycle(ledgers)? {
        if spent > CAP {
            return Err(format!("the month from {month} spent {spent} generations, past the cap of {CAP}"));
        }
    }
    Ok(())
}
